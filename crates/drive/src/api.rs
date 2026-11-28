//! Drive v3, the few calls DeckCraft needs: list, open, create, update. With the `drive.file`
//! scope every one of them can only see files this app created or the teacher picked.

use std::sync::Arc;

use serde::Deserialize;

use crate::config::Config;
use crate::http::{Http, Request, Response};
use crate::oauth::{self, Tokens, enc};
use crate::store::TokenStore;
use crate::{DriveError, now_secs};

const API: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3";
pub const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
pub const PPTX_MIME: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
/// The folder DeckCraft keeps its saves in.
pub const APP_FOLDER: &str = "DeckCraft";
const FILE_FIELDS: &str = "id,name,mimeType,modifiedTime";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub mime_type: String,
    /// RFC 3339, exactly as Drive reports it; compared as text to spot a change.
    #[serde(default)]
    pub modified_time: String,
}

impl DriveFile {
    pub fn is_folder(&self) -> bool {
        self.mime_type == FOLDER_MIME
    }
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<DriveFile>,
}

#[derive(Clone)]
pub struct Drive {
    http: Arc<dyn Http>,
    store: Arc<dyn TokenStore>,
    cfg: Config,
}

/// A Drive query string value: quotes and backslashes escaped.
pub fn q_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

fn google_message(res: &Response) -> String {
    #[derive(Deserialize)]
    struct E {
        error: Inner,
    }
    #[derive(Deserialize)]
    struct Inner {
        message: Option<String>,
    }
    serde_json::from_slice::<E>(&res.body).ok().and_then(|e| e.error.message).unwrap_or_else(|| format!("status {}", res.status))
}

impl Drive {
    pub fn new(http: Arc<dyn Http>, store: Arc<dyn TokenStore>, cfg: Config) -> Self {
        Drive { http, store, cfg }
    }

    pub fn signed_in(&self) -> bool {
        matches!(self.store.load(), Ok(Some(_)))
    }

    /// Complete a sign-in: keep what Google sent in the keychain.
    pub fn finish_sign_in(&self, tokens: &Tokens) -> Result<(), DriveError> {
        self.store.save(tokens)
    }

    /// A working access token, refreshed if it has (nearly) expired. A grant that Google no
    /// longer honours signs the teacher out and clears the keychain.
    fn token(&self, force_refresh: bool) -> Result<String, DriveError> {
        let tokens = self.store.load()?.ok_or(DriveError::SignedOut)?;
        if !force_refresh && !tokens.expired(now_secs()) {
            return Ok(tokens.access_token);
        }
        match oauth::refresh(self.http.as_ref(), &self.cfg, &tokens.refresh_token) {
            Ok(fresh) => {
                self.store.save(&fresh)?;
                Ok(fresh.access_token)
            }
            Err(DriveError::SignedOut) => {
                let _ = self.store.delete();
                Err(DriveError::SignedOut)
            }
            Err(e) => Err(e),
        }
    }

    /// Send with the bearer token; on a 401 refresh once and try again.
    fn authed(&self, mut req: Request) -> Result<Response, DriveError> {
        let mut force = false;
        for _ in 0..2 {
            let token = self.token(force)?;
            req.headers.retain(|(k, _)| !k.eq_ignore_ascii_case("authorization"));
            req.headers.push(("Authorization".into(), format!("Bearer {token}")));
            let res = self.http.send(&req)?;
            if res.status == 401 && !force {
                force = true;
                continue;
            }
            if res.ok() {
                return Ok(res);
            }
            return Err(match res.status {
                401 => DriveError::SignedOut,
                404 => DriveError::Refused("that file is not available to DeckCraft (it can only see files it saved or you chose)".into()),
                _ => DriveError::Refused(google_message(&res)),
            });
        }
        Err(DriveError::SignedOut)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, url: String) -> Result<T, DriveError> {
        let res = self.authed(Request { method: "GET", url, ..Default::default() })?;
        serde_json::from_slice(&res.body).map_err(|e| DriveError::Other(format!("unreadable reply from Drive: {e}")))
    }

    /// Who is signed in, if Drive says (a failure is not an error: the name is a nicety).
    pub fn account(&self) -> Option<String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct About {
            user: User,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct User {
            email_address: Option<String>,
            display_name: Option<String>,
        }
        let about: About = self.get_json(format!("{API}/about?fields=user(emailAddress,displayName)")).ok()?;
        about.user.email_address.or(about.user.display_name)
    }

    /// Files (and folders) the app can see, newest first; inside `parent` when given.
    pub fn list(&self, parent: Option<&str>) -> Result<Vec<DriveFile>, DriveError> {
        let mut q = String::from("trashed = false");
        if let Some(p) = parent {
            q.push_str(&format!(" and '{}' in parents", q_escape(p)));
        }
        let url = format!("{API}/files?q={}&orderBy=modifiedTime%20desc&pageSize=100&fields=files({FILE_FIELDS})", enc(&q));
        Ok(self.get_json::<FileList>(url)?.files)
    }

    pub fn metadata(&self, id: &str) -> Result<DriveFile, DriveError> {
        self.get_json(format!("{API}/files/{}?fields={FILE_FIELDS}", enc(id)))
    }

    pub fn download(&self, id: &str) -> Result<Vec<u8>, DriveError> {
        Ok(self.authed(Request { method: "GET", url: format!("{API}/files/{}?alt=media", enc(id)), ..Default::default() })?.body)
    }

    /// The app's own folder, created on first use.
    pub fn app_folder(&self) -> Result<DriveFile, DriveError> {
        let q = format!("trashed = false and mimeType = '{FOLDER_MIME}' and name = '{}'", q_escape(APP_FOLDER));
        let found: FileList = self.get_json(format!("{API}/files?q={}&pageSize=1&fields=files({FILE_FIELDS})", enc(&q)))?;
        if let Some(f) = found.files.into_iter().next() {
            return Ok(f);
        }
        let meta = serde_json::json!({"name": APP_FOLDER, "mimeType": FOLDER_MIME});
        let res = self.authed(Request {
            method: "POST",
            url: format!("{API}/files?fields={FILE_FIELDS}"),
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: meta.to_string().into_bytes(),
        })?;
        serde_json::from_slice(&res.body).map_err(|e| DriveError::Other(format!("unreadable reply from Drive: {e}")))
    }

    /// Upload as a new file. A resumable upload: nothing exists in Drive until the last byte
    /// arrives, so a transfer that dies halfway leaves no half-written file.
    pub fn create(&self, name: &str, bytes: Vec<u8>, parent: Option<&str>) -> Result<DriveFile, DriveError> {
        let mut meta = serde_json::json!({"name": name, "mimeType": PPTX_MIME});
        if let Some(p) = parent {
            meta["parents"] = serde_json::json!([p]);
        }
        let start = self.authed(Request {
            method: "POST",
            url: format!("{UPLOAD}/files?uploadType=resumable&fields={FILE_FIELDS}"),
            headers: vec![("Content-Type".into(), "application/json; charset=UTF-8".into()), ("X-Upload-Content-Type".into(), PPTX_MIME.into())],
            body: meta.to_string().into_bytes(),
        })?;
        let session = start.header("location").ok_or_else(|| DriveError::Other("Drive gave no upload address".into()))?.to_string();
        self.put_bytes(&session, bytes)
    }

    /// Replace a file's contents, but only if it is still the version that was opened.
    pub fn update(&self, id: &str, bytes: Vec<u8>, expected_modified: &str) -> Result<DriveFile, DriveError> {
        let now = self.metadata(id)?;
        if now.modified_time != expected_modified {
            return Err(DriveError::Conflict);
        }
        let start = self.authed(Request {
            method: "PATCH",
            url: format!("{UPLOAD}/files/{}?uploadType=resumable&fields={FILE_FIELDS}", enc(id)),
            headers: vec![("Content-Type".into(), "application/json; charset=UTF-8".into()), ("X-Upload-Content-Type".into(), PPTX_MIME.into())],
            body: b"{}".to_vec(),
        })?;
        let session = start.header("location").ok_or_else(|| DriveError::Other("Drive gave no upload address".into()))?.to_string();
        self.put_bytes(&session, bytes)
    }

    fn put_bytes(&self, session_url: &str, bytes: Vec<u8>) -> Result<DriveFile, DriveError> {
        let res =
            self.authed(Request { method: "PUT", url: session_url.into(), headers: vec![("Content-Type".into(), PPTX_MIME.into())], body: bytes })?;
        serde_json::from_slice(&res.body).map_err(|e| DriveError::Other(format!("Drive saved the file but the reply was unreadable: {e}")))
    }

    /// Delete the keychain entry first (that is what matters), then ask Google to revoke.
    pub fn sign_out(&self) -> Result<(), DriveError> {
        let tokens = self.store.load().ok().flatten();
        self.store.delete()?;
        if let Some(t) = tokens {
            oauth::revoke(self.http.as_ref(), &t.refresh_token);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::http::fake::Fake;
    use crate::store::Memory;

    fn drive(replies: Vec<Result<Response, DriveError>>, tokens: Option<Tokens>) -> (Drive, Arc<Fake>, Arc<Memory>) {
        let http = Arc::new(Fake::with(replies));
        let store = Arc::new(Memory::default());
        if let Some(t) = tokens {
            store.save(&t).unwrap();
        }
        (Drive::new(http.clone(), store.clone(), Config { client_id: "cid".into(), client_secret: None }), http, store)
    }

    fn fresh() -> Tokens {
        Tokens { access_token: "AT".into(), refresh_token: "RT".into(), expires_at: now_secs() + 3000 }
    }

    fn stale() -> Tokens {
        Tokens { access_token: "OLD".into(), refresh_token: "RT".into(), expires_at: now_secs() }
    }

    fn auth_of(r: &Request) -> &str {
        r.headers.iter().find(|(k, _)| k == "Authorization").map(|(_, v)| v.as_str()).unwrap_or("")
    }

    #[test]
    fn not_signed_in_means_signed_out_and_nothing_is_sent() {
        let (d, http, _) = drive(vec![], None);
        assert_eq!(d.list(None).unwrap_err(), DriveError::SignedOut);
        assert!(http.requests().is_empty());
        assert!(!d.signed_in());
    }

    #[test]
    fn listing_sends_the_bearer_token_and_a_trashed_filter() {
        let (d, http, _) = drive(vec![Fake::json(200, r#"{"files":[{"id":"1","name":"A.pptx","mimeType":"x","modifiedTime":"t"}]}"#)], Some(fresh()));
        let files = d.list(Some("fo'lder")).unwrap();
        assert_eq!(files[0].name, "A.pptx");
        let r = &http.requests()[0];
        assert_eq!(auth_of(r), "Bearer AT");
        assert!(r.url.contains("trashed%20%3D%20false"), "{}", r.url);
        assert!(r.url.contains("fo%5C%27lder"), "the quote is escaped: {}", r.url);
    }

    #[test]
    fn an_expired_token_is_refreshed_first_and_the_new_one_is_stored() {
        let (d, http, store) =
            drive(vec![Fake::json(200, r#"{"access_token":"NEW","expires_in":3600}"#), Fake::json(200, r#"{"files":[]}"#)], Some(stale()));
        d.list(None).unwrap();
        let reqs = http.requests();
        assert!(reqs[0].url.starts_with(oauth::TOKEN_ENDPOINT));
        assert_eq!(auth_of(&reqs[1]), "Bearer NEW");
        assert_eq!(store.load().unwrap().unwrap().access_token, "NEW");
        assert_eq!(store.load().unwrap().unwrap().refresh_token, "RT", "the refresh token is kept");
    }

    #[test]
    fn a_401_triggers_one_refresh_and_a_retry() {
        let (d, http, _) = drive(
            vec![Fake::json(401, "{}"), Fake::json(200, r#"{"access_token":"N2","expires_in":3600}"#), Fake::json(200, r#"{"files":[]}"#)],
            Some(fresh()),
        );
        assert!(d.list(None).unwrap().is_empty());
        assert_eq!(http.requests().len(), 3);
    }

    #[test]
    fn a_revoked_grant_signs_out_and_clears_the_keychain() {
        let (d, _, store) = drive(vec![Fake::json(400, r#"{"error":"invalid_grant"}"#)], Some(stale()));
        assert_eq!(d.list(None).unwrap_err(), DriveError::SignedOut);
        assert!(store.load().unwrap().is_none());
    }

    #[test]
    fn no_network_is_offline_and_keeps_the_teacher_signed_in() {
        let (d, _, store) = drive(vec![Err(DriveError::Offline("dns".into()))], Some(fresh()));
        assert!(matches!(d.list(None), Err(DriveError::Offline(_))));
        assert!(store.load().unwrap().is_some(), "a dead network is not a sign-out");
    }

    #[test]
    fn google_errors_carry_googles_message_and_404_explains_the_scope() {
        let (d, _, _) = drive(vec![Fake::json(403, r#"{"error":{"message":"Rate limit"}}"#), Fake::json(404, "{}")], Some(fresh()));
        assert_eq!(d.metadata("x").unwrap_err(), DriveError::Refused("Rate limit".into()));
        assert!(matches!(d.metadata("x").unwrap_err(), DriveError::Refused(m) if m.contains("only see files")));
    }

    #[test]
    fn create_starts_a_resumable_upload_then_puts_the_bytes() {
        let started = Ok(Response { status: 200, headers: vec![("Location".into(), "https://upload.example/session1".into())], body: vec![] });
        let (d, http, _) = drive(vec![started, Fake::json(200, r#"{"id":"9","name":"Lesson.pptx","modifiedTime":"m1"}"#)], Some(fresh()));
        let f = d.create("Lesson.pptx", b"PK-bytes".to_vec(), Some("folder1")).unwrap();
        assert_eq!((f.id.as_str(), f.modified_time.as_str()), ("9", "m1"));
        let reqs = http.requests();
        assert!(reqs[0].url.contains("uploadType=resumable"));
        let meta = String::from_utf8(reqs[0].body.clone()).unwrap();
        assert!(meta.contains("Lesson.pptx") && meta.contains("folder1"));
        assert_eq!((reqs[1].method, reqs[1].url.as_str(), reqs[1].body.as_slice()), ("PUT", "https://upload.example/session1", &b"PK-bytes"[..]));
    }

    #[test]
    fn a_save_that_dies_halfway_reports_it_and_creates_nothing_to_clean_up() {
        let started = Ok(Response { status: 200, headers: vec![("location".into(), "https://upload.example/s".into())], body: vec![] });
        let (d, http, _) = drive(vec![started, Err(DriveError::Offline("connection reset".into()))], Some(fresh()));
        assert!(matches!(d.create("x.pptx", vec![1, 2, 3], None), Err(DriveError::Offline(_))));
        assert_eq!(http.requests().len(), 2, "no half-finished file is created, and nothing is retried silently");
    }

    #[test]
    fn updating_a_file_that_changed_in_drive_is_a_conflict_and_uploads_nothing() {
        let (d, http, _) = drive(vec![Fake::json(200, r#"{"id":"9","name":"n","modifiedTime":"NEWER"}"#)], Some(fresh()));
        assert_eq!(d.update("9", vec![1], "OLDER").unwrap_err(), DriveError::Conflict);
        assert_eq!(http.requests().len(), 1, "only the metadata check went out");
    }

    #[test]
    fn updating_an_unchanged_file_patches_it() {
        let started = Ok(Response { status: 200, headers: vec![("Location".into(), "https://upload.example/u".into())], body: vec![] });
        let (d, http, _) = drive(
            vec![
                Fake::json(200, r#"{"id":"9","name":"n","modifiedTime":"T1"}"#),
                started,
                Fake::json(200, r#"{"id":"9","name":"n","modifiedTime":"T2"}"#),
            ],
            Some(fresh()),
        );
        let f = d.update("9", vec![7], "T1").unwrap();
        assert_eq!(f.modified_time, "T2");
        assert_eq!(http.requests()[1].method, "PATCH");
    }

    #[test]
    fn signing_out_deletes_the_tokens_even_if_revoking_fails() {
        let (d, http, store) = drive(vec![Err(DriveError::Offline("no network".into()))], Some(fresh()));
        d.sign_out().unwrap();
        assert!(store.load().unwrap().is_none());
        assert!(http.requests()[0].url.starts_with(oauth::REVOKE_ENDPOINT));
        assert!(!d.signed_in());
    }

    #[test]
    fn the_app_folder_is_found_or_created_once() {
        let (d, http, _) = drive(
            vec![
                Fake::json(200, r#"{"files":[]}"#),
                Fake::json(200, r#"{"id":"F","name":"DeckCraft","mimeType":"application/vnd.google-apps.folder"}"#),
            ],
            Some(fresh()),
        );
        let f = d.app_folder().unwrap();
        assert!(f.is_folder());
        assert_eq!(http.requests()[1].method, "POST");
    }
}
