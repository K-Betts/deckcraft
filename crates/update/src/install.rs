//! Download the new app, check it, and swap it in.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::UpdateError;
use crate::net::Http;
use crate::release::Available;

/// No Mac app is anywhere near this big; anything larger is refused.
const MAX_ZIP: u64 = 400 << 20;
pub const APP_NAME: &str = "DeckCraft.app";

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The first 64-hex-digit word of a `.sha256` file (`<hash>  <name>`).
pub fn parse_checksum(text: &str) -> Option<String> {
    let w = text.split_whitespace().next()?.to_ascii_lowercase();
    (w.len() == 64 && w.chars().all(|c| c.is_ascii_hexdigit())).then_some(w)
}

/// Fetch the zip and its checksum into `dir`, and keep it only if they agree.
pub fn download(http: &dyn Http, a: &Available, dir: &Path) -> Result<PathBuf, UpdateError> {
    if a.size > MAX_ZIP {
        return Err(UpdateError::BadDownload(format!("it is {} MB, more than the {} MB allowed", a.size >> 20, MAX_ZIP >> 20)));
    }
    let (status, sum_body) = http.get(&a.sha256_url, 4096)?;
    if status != 200 {
        return Err(UpdateError::BadDownload(format!("the checksum file answered {status}")));
    }
    let want =
        parse_checksum(&String::from_utf8_lossy(&sum_body)).ok_or_else(|| UpdateError::BadDownload("the checksum file is not a SHA-256".into()))?;
    let (status, zip) = http.get(&a.zip_url, MAX_ZIP)?;
    if status != 200 {
        return Err(UpdateError::BadDownload(format!("the download answered {status}")));
    }
    let got = sha256_hex(&zip);
    if got != want {
        return Err(UpdateError::BadDownload("its checksum does not match".into()));
    }
    std::fs::create_dir_all(dir).map_err(|e| UpdateError::Install(format!("cannot make the download folder: {e}")))?;
    let path = dir.join(format!("DeckCraft-{}.zip", a.sha));
    std::fs::write(&path, zip).map_err(|e| UpdateError::Install(format!("cannot save the download: {e}")))?;
    Ok(path)
}

/// The `DeckCraft.app` this process is running from, if it is an installed app (not a build run
/// from a terminal with `cargo run`).
pub fn current_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors().find(|p| p.file_name().is_some_and(|n| n == APP_NAME)).map(Path::to_path_buf)
}

/// Replace `app` with the app inside `zip`. Either the new app is in place at the end, or the old
/// one still is: nothing is left half-done.
#[cfg(target_os = "macos")]
pub fn install(zip: &Path, app: &Path, check_signature: bool) -> Result<(), UpdateError> {
    use std::process::Command;
    let fail = |m: String| UpdateError::Install(m);
    if app.file_name().is_none_or(|n| n != APP_NAME) {
        return Err(UpdateError::Unsupported(
            "This copy is not an installed DeckCraft.app, so it cannot replace itself. Use packaging/macos/update-local.sh.".into(),
        ));
    }
    let dir = app.parent().ok_or_else(|| fail("the app has no folder".into()))?;
    // Stage beside the old app so the final moves stay on one volume (and are atomic renames).
    let stage = dir.join(format!(".DeckCraft-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage)
        .map_err(|e| fail(format!("cannot write to {}: {e}. Is DeckCraft allowed to change that folder?", dir.display())))?;
    let result = (|| {
        let out = Command::new("/usr/bin/ditto").arg("-x").arg("-k").arg(zip).arg(&stage).output().map_err(|e| fail(format!("cannot unzip: {e}")))?;
        if !out.status.success() {
            return Err(fail(format!("unzip failed: {}", String::from_utf8_lossy(&out.stderr).trim())));
        }
        let new_app = stage.join(APP_NAME);
        if !new_app.join("Contents/MacOS/DeckCraft").is_file() {
            return Err(fail("the download does not contain DeckCraft.app".into()));
        }
        if check_signature {
            let out = Command::new("/usr/bin/codesign")
                .args(["--verify", "--strict", "--deep"])
                .arg(&new_app)
                .output()
                .map_err(|e| fail(format!("cannot check the signature: {e}")))?;
            if !out.status.success() {
                return Err(fail(format!("the new app's signature is not valid: {}", String::from_utf8_lossy(&out.stderr).trim())));
            }
        }
        // Nothing is added to the bundle: any new file would break its code-signature seal.
        let previous = dir.join("DeckCraft.app.previous");
        let _ = std::fs::remove_dir_all(&previous);
        std::fs::rename(app, &previous).map_err(|e| fail(format!("cannot move the old app aside: {e}")))?;
        if let Err(e) = std::fs::rename(&new_app, app) {
            // Put the old one back.
            let _ = std::fs::rename(&previous, app);
            return Err(fail(format!("cannot put the new app in place: {e}")));
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&stage);
    result
}

#[cfg(not(target_os = "macos"))]
pub fn install(_zip: &Path, _app: &Path, _check_signature: bool) -> Result<(), UpdateError> {
    Err(UpdateError::Unsupported("In-app updates are Mac-only for now.".into()))
}

/// Start the new app a moment after this one quits.
pub fn relaunch(app: &Path) -> Result<(), UpdateError> {
    let cmd = format!("sleep 1; /usr/bin/open -n '{}'", app.display().to_string().replace('\'', "'\\''"));
    std::process::Command::new("/bin/sh").arg("-c").arg(cmd).spawn().map(|_| ()).map_err(|e| UpdateError::Install(format!("cannot restart: {e}")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::net::fake::Fake;

    fn avail(size: u64) -> Available {
        Available {
            label: "build-abc1234".into(),
            sha: "abc1234".into(),
            notes: String::new(),
            zip_url: "https://github.com/x/y/z.zip".into(),
            sha256_url: "https://github.com/x/y/z.zip.sha256".into(),
            size,
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("deckcraft-update-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn sha256_matches_a_known_value_and_checksum_files_parse() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let good = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD  DeckCraft.zip\n";
        assert_eq!(parse_checksum(good).unwrap(), sha256_hex(b"abc"));
        assert_eq!(parse_checksum("not a hash"), None);
        assert_eq!(parse_checksum(""), None);
    }

    #[test]
    fn a_matching_download_is_saved_and_a_mismatch_is_refused() {
        let a = avail(10);
        let bytes = b"zip-bytes".to_vec();
        let ok = Fake::default().with(&a.sha256_url, 200, format!("{}  z.zip", sha256_hex(&bytes))).with(&a.zip_url, 200, bytes.clone());
        let dir = tmp("dl");
        let path = download(&ok, &a, &dir).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let tampered =
            Fake::default().with(&a.sha256_url, 200, format!("{}  z.zip", sha256_hex(&bytes))).with(&a.zip_url, 200, b"zip-bytez".to_vec());
        let dir2 = tmp("dl2");
        assert!(matches!(download(&tampered, &a, &dir2), Err(UpdateError::BadDownload(m)) if m.contains("does not match")));
        assert!(std::fs::read_dir(&dir2).map(|mut d| d.next().is_none()).unwrap_or(true), "a refused download is not left on disk");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    #[test]
    fn oversize_and_failed_downloads_are_refused() {
        assert!(matches!(download(&Fake::default(), &avail(500 << 20), &tmp("big")), Err(UpdateError::BadDownload(m)) if m.contains("MB")));
        let a = avail(10);
        let gone = Fake::default().with(&a.sha256_url, 404, "x");
        assert!(matches!(download(&gone, &a, &tmp("gone")), Err(UpdateError::BadDownload(_))));
        assert!(matches!(download(&Fake::default(), &a, &tmp("off")), Err(UpdateError::Offline(_))));
    }

    #[cfg(target_os = "macos")]
    fn fake_app(root: &Path, marker: &str) -> PathBuf {
        let app = root.join(APP_NAME);
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        std::fs::write(app.join("Contents/MacOS/DeckCraft"), marker).unwrap();
        app
    }

    #[cfg(target_os = "macos")]
    fn zip_of(app: &Path, zip: &Path) {
        let ok = std::process::Command::new("/usr/bin/ditto").args(["-c", "-k", "--keepParent"]).arg(app).arg(zip).status().unwrap().success();
        assert!(ok);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn installing_swaps_the_app_and_keeps_the_old_one() {
        let root = tmp("swap");
        let new_src = tmp("swap-src");
        let zip = root.join("new.zip");
        zip_of(&fake_app(&new_src, "NEW"), &zip);
        let installed = fake_app(&root.join("Applications_"), "OLD").parent().map(Path::to_path_buf).unwrap();
        let app = installed.join(APP_NAME);
        install(&zip, &app, false).unwrap();
        assert_eq!(std::fs::read_to_string(app.join("Contents/MacOS/DeckCraft")).unwrap(), "NEW");
        assert_eq!(std::fs::read_to_string(installed.join("DeckCraft.app.previous/Contents/MacOS/DeckCraft")).unwrap(), "OLD", "a way back");
        assert!(
            std::fs::read_dir(&installed).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().starts_with(".DeckCraft-update")),
            "staging is cleaned up"
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&new_src);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_bad_zip_or_a_missing_app_inside_leaves_the_old_app_alone() {
        let root = tmp("bad");
        let installed = root.join("Applications_");
        let app = fake_app(&installed, "OLD");
        std::fs::write(root.join("junk.zip"), b"not a zip").unwrap();
        assert!(matches!(install(&root.join("junk.zip"), &app, false), Err(UpdateError::Install(_))));
        let wrong = tmp("bad-src");
        std::fs::create_dir_all(wrong.join("Other.app")).unwrap();
        std::fs::write(wrong.join("Other.app/file"), "x").unwrap();
        zip_of(&wrong.join("Other.app"), &root.join("other.zip"));
        assert!(matches!(install(&root.join("other.zip"), &app, false), Err(UpdateError::Install(m)) if m.contains("does not contain")));
        assert_eq!(std::fs::read_to_string(app.join("Contents/MacOS/DeckCraft")).unwrap(), "OLD");
        assert!(!installed.join("DeckCraft.app.previous").exists());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&wrong);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_fake_unsigned_app_fails_the_signature_check_and_is_not_installed() {
        let root = tmp("sig");
        let src = tmp("sig-src");
        let zip = root.join("new.zip");
        zip_of(&fake_app(&src, "NEW"), &zip);
        let app = fake_app(&root.join("Applications_"), "OLD");
        assert!(matches!(install(&zip, &app, true), Err(UpdateError::Install(m)) if m.contains("signature")));
        assert_eq!(std::fs::read_to_string(app.join("Contents/MacOS/DeckCraft")).unwrap(), "OLD");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&src);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn only_a_folder_called_deckcraft_app_can_be_replaced() {
        let root = tmp("name");
        let zip = root.join("x.zip");
        std::fs::write(&zip, b"x").unwrap();
        let other = root.join("Other.app");
        std::fs::create_dir_all(&other).unwrap();
        assert!(matches!(install(&zip, &other, false), Err(UpdateError::Unsupported(_))));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Uses a real, signed DeckCraft.app (set `DECKCRAFT_TEST_APP` to one):
    /// `DECKCRAFT_TEST_APP=/Applications/DeckCraft.app cargo test -p deckcraft-update -- --ignored real_signed_app`.
    /// It installs a copy into a temporary folder and checks the signature passes.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "needs a real DeckCraft.app"]
    fn real_signed_app_installs_over_an_older_copy_and_passes_the_signature_check() {
        let Some(real) = std::env::var_os("DECKCRAFT_TEST_APP").map(PathBuf::from) else { return };
        let root = tmp("real");
        let zip = root.join("real.zip");
        zip_of(&real, &zip);
        let app = fake_app(&root.join("Applications_"), "OLD");
        install(&zip, &app, true).unwrap();
        assert!(app.join("Contents/MacOS/DeckCraft").metadata().unwrap().len() > 1_000_000, "the real executable is in place");
        let ok = std::process::Command::new("/usr/bin/codesign").args(["--verify", "--strict", "--deep"]).arg(&app).status().unwrap().success();
        assert!(ok, "the installed app's code signature is still valid (nothing was added to the bundle)");
        let _ = std::fs::remove_dir_all(&root);
    }
}
