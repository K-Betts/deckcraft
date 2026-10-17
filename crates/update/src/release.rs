//! Reading GitHub's "latest release" and deciding whether it is newer than the running app.

use serde::Deserialize;

use crate::UpdateError;
use crate::net::Http;

/// This fork's releases. (Never the upstream `storytold` repositories.)
pub const REPO: &str = "K-Betts/deckcraft";

/// A release asset name: `DeckCraft-macos-aarch64-<sha7>.zip`.
pub fn asset_prefix(os: &str, arch: &str) -> Option<String> {
    match (os, arch) {
        ("macos", "aarch64") | ("macos", "x86_64") => Some(format!("DeckCraft-macos-{arch}-")),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Available {
    /// The release's name, e.g. `build-abc1234`.
    pub label: String,
    /// The first seven characters of the commit it was built from.
    pub sha: String,
    pub notes: String,
    pub zip_url: String,
    pub sha256_url: String,
    pub size: u64,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    #[serde(default)]
    size: u64,
    browser_download_url: String,
}

/// Parse the release JSON into the Mac app for this machine, if it has one.
pub fn parse(json: &[u8], os: &str, arch: &str) -> Result<Available, UpdateError> {
    let prefix = asset_prefix(os, arch).ok_or_else(|| UpdateError::Unsupported("In-app updates are Mac-only for now.".into()))?;
    let rel: Release = serde_json::from_slice(json).map_err(|e| UpdateError::NoRelease(format!("unreadable release: {e}")))?;
    let zip = rel
        .assets
        .iter()
        .find(|a| a.name.starts_with(&prefix) && a.name.ends_with(".zip"))
        .ok_or_else(|| UpdateError::NoRelease(format!("release {} has no {prefix}… download for this Mac", rel.tag_name)))?;
    let sha = zip.name[prefix.len()..zip.name.len() - 4].to_string();
    if sha.len() < 7 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(UpdateError::NoRelease(format!("{} has no commit id in its name", zip.name)));
    }
    let sum = rel
        .assets
        .iter()
        .find(|a| a.name == format!("{}.sha256", zip.name))
        .ok_or_else(|| UpdateError::NoRelease(format!("{} has no checksum file, so it will not be installed", zip.name)))?;
    // Only GitHub's own download hosts: never follow a link a release body points at.
    for u in [&zip.browser_download_url, &sum.browser_download_url] {
        if !u.starts_with("https://github.com/") {
            return Err(UpdateError::NoRelease("a download address is not on github.com".into()));
        }
    }
    Ok(Available {
        label: rel.tag_name,
        sha: sha[..7].to_string(),
        notes: rel.body.unwrap_or_default().chars().take(2000).collect(),
        zip_url: zip.browser_download_url.clone(),
        sha256_url: sum.browser_download_url.clone(),
        size: zip.size,
    })
}

/// Is `release_sha` a different build from the running one? `current` is the running app's commit
/// (any length of at least seven characters).
pub fn differs(current: &str, release_sha: &str) -> bool {
    let n = release_sha.len().min(current.len()).min(7);
    n < 7 || !current[..n].eq_ignore_ascii_case(&release_sha[..n])
}

/// Ask GitHub for the latest release and say whether it is not the running version.
pub fn check(http: &dyn Http, repo: &str, os: &str, arch: &str, current_sha: Option<&str>) -> Result<Option<Available>, UpdateError> {
    let current = current_sha.filter(|s| s.len() >= 7).ok_or_else(|| {
        UpdateError::Unsupported("This copy does not know which version it is (it is a development build), so it cannot compare.".into())
    })?;
    asset_prefix(os, arch).ok_or_else(|| UpdateError::Unsupported("In-app updates are Mac-only for now.".into()))?;
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let (status, body) = http.get(&url, 2 << 20)?;
    match status {
        200 => {}
        404 => return Err(UpdateError::NoRelease("no release has been published yet".into())),
        s => return Err(UpdateError::NoRelease(format!("GitHub answered {s}"))),
    }
    let avail = parse(&body, os, arch)?;
    Ok(differs(current, &avail.sha).then_some(avail))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::net::fake::Fake;

    pub fn release_json(sha: &str) -> String {
        let z = format!("DeckCraft-macos-aarch64-{sha}.zip");
        format!(
            r#"{{"tag_name":"build-{sha}","body":"Notes here","assets":[
              {{"name":"{z}","size":1234,"browser_download_url":"https://github.com/K-Betts/deckcraft/releases/download/build-{sha}/{z}"}},
              {{"name":"{z}.sha256","size":90,"browser_download_url":"https://github.com/K-Betts/deckcraft/releases/download/build-{sha}/{z}.sha256"}}]}}"#
        )
    }

    const URL: &str = "https://api.github.com/repos/K-Betts/deckcraft/releases/latest";

    #[test]
    fn a_newer_release_is_offered_with_its_notes_and_downloads() {
        let http = Fake::default().with(URL, 200, release_json("abc1234"));
        let a = check(&http, REPO, "macos", "aarch64", Some("9ebdf74aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")).unwrap().unwrap();
        assert_eq!((a.sha.as_str(), a.label.as_str(), a.notes.as_str(), a.size), ("abc1234", "build-abc1234", "Notes here", 1234));
        assert!(a.zip_url.ends_with("abc1234.zip") && a.sha256_url.ends_with("abc1234.zip.sha256"));
        assert_eq!(http.urls(), [URL], "one request, to GitHub's API");
    }

    #[test]
    fn the_same_commit_means_up_to_date() {
        let http = Fake::default().with(URL, 200, release_json("abc1234"));
        assert_eq!(check(&http, REPO, "macos", "aarch64", Some("abc1234ffff")).unwrap(), None);
        assert_eq!(check(&http, REPO, "macos", "aarch64", Some("ABC1234")).unwrap(), None, "case does not matter");
    }

    #[test]
    fn no_release_no_network_and_other_systems_are_explained() {
        let none = Fake::default().with(URL, 404, "{}");
        assert!(matches!(check(&none, REPO, "macos", "aarch64", Some("abc1234")), Err(UpdateError::NoRelease(m)) if m.contains("no release")));
        assert!(matches!(check(&Fake::default(), REPO, "macos", "aarch64", Some("abc1234")), Err(UpdateError::Offline(_))));
        assert!(matches!(check(&none, REPO, "windows", "x86_64", Some("abc1234")), Err(UpdateError::Unsupported(_))));
        assert!(matches!(check(&none, REPO, "macos", "aarch64", None), Err(UpdateError::Unsupported(m)) if m.contains("development")));
        assert!(matches!(check(&none, REPO, "macos", "aarch64", Some("abc")), Err(UpdateError::Unsupported(_))));
    }

    #[test]
    fn a_release_without_a_checksum_or_for_another_mac_is_refused() {
        let no_sum = release_json("abc1234").replace(".zip.sha256", ".zip.sig");
        assert!(matches!(parse(no_sum.as_bytes(), "macos", "aarch64"), Err(UpdateError::NoRelease(m)) if m.contains("checksum")));
        assert!(matches!(parse(release_json("abc1234").as_bytes(), "macos", "x86_64"), Err(UpdateError::NoRelease(_))));
        assert!(parse(b"not json", "macos", "aarch64").is_err());
    }

    #[test]
    fn download_links_must_be_on_github() {
        let evil = release_json("abc1234").replace("https://github.com/", "https://evil.example/");
        assert!(matches!(parse(evil.as_bytes(), "macos", "aarch64"), Err(UpdateError::NoRelease(m)) if m.contains("github.com")));
    }

    #[test]
    fn differs_needs_seven_characters_on_both_sides() {
        assert!(differs("abc1234", "abc1235"));
        assert!(!differs("abc1234567", "abc1234"));
        assert!(differs("abc", "abc1234"), "too short to say they match");
    }
}
