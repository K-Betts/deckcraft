//! The sign-in and file operations as background work, for a UI that must never block.
//!
//! The UI calls a method (which returns at once) and later drains [`DriveService::poll`] for
//! events. Each operation runs on its own short-lived thread.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use crate::DriveError;
use crate::api::{Drive, DriveFile};
use crate::config::{Config, ConfigError};
use crate::http::Http;
use crate::oauth::{self, Pkce};
use crate::signin::Listener;
use crate::store::TokenStore;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriveStatus {
    /// No client ID configured: the message says how to set one up.
    NotConfigured(String),
    SignedOut,
    SigningIn,
    SignedIn(Option<String>),
}

#[derive(Debug)]
pub enum DriveEvent {
    Status(DriveStatus),
    Listing(Result<Vec<DriveFile>, DriveError>),
    /// A file was downloaded to `path` (in the app's cache) ready to open.
    Opened {
        file: DriveFile,
        path: PathBuf,
    },
    /// A save finished; the file's new version is `file`.
    Saved(DriveFile),
    Failed(DriveError),
}

pub type Opener = Arc<dyn Fn(&str) -> Result<(), DriveError> + Send + Sync>;

pub struct DriveService {
    drive: Option<Drive>,
    http: Arc<dyn Http>,
    cfg: Result<Config, ConfigError>,
    opener: Opener,
    cache_dir: PathBuf,
    status: DriveStatus,
    cancel: Arc<AtomicBool>,
    tx: Sender<DriveEvent>,
    rx: Receiver<DriveEvent>,
    started: bool,
    store: Arc<dyn TokenStore>,
}

/// A name safe to use as a file name inside the cache.
pub fn safe_name(name: &str) -> String {
    let cleaned: String = name.chars().map(|c| if c.is_control() || matches!(c, '/' | '\\' | ':') { '_' } else { c }).collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_string();
    if cleaned.is_empty() { "Presentation.pptx".into() } else { cleaned.chars().take(120).collect() }
}

impl DriveService {
    pub fn new(http: Arc<dyn Http>, store: Arc<dyn TokenStore>, cfg: Result<Config, ConfigError>, opener: Opener, cache_dir: PathBuf) -> Self {
        let (tx, rx) = channel();
        let drive = cfg.clone().ok().map(|c| Drive::new(http.clone(), store.clone(), c));
        let status = match &cfg {
            Ok(_) => DriveStatus::SignedOut,
            Err(e) => DriveStatus::NotConfigured(e.to_string()),
        };
        DriveService { drive, http, cfg, opener, cache_dir, status, cancel: Arc::default(), tx, rx, started: false, store }
    }

    pub fn status(&self) -> &DriveStatus {
        &self.status
    }

    /// Look in the keychain once, the first time Drive is actually wanted (not at launch).
    pub fn start(&mut self) {
        if self.started {
            return;
        }
        self.started = true;
        let Some(drive) = self.drive.clone() else { return };
        if drive.signed_in() {
            self.status = DriveStatus::SignedIn(None);
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(DriveEvent::Status(DriveStatus::SignedIn(drive.account())));
            });
        }
    }

    pub fn sign_in(&mut self) {
        self.started = true;
        let (Some(drive), Ok(cfg)) = (self.drive.clone(), self.cfg.clone()) else { return };
        if self.status == DriveStatus::SigningIn {
            return;
        }
        self.status = DriveStatus::SigningIn;
        self.cancel.store(false, Ordering::SeqCst);
        let (tx, http, opener, cancel) = (self.tx.clone(), self.http.clone(), self.opener.clone(), self.cancel.clone());
        std::thread::spawn(move || {
            let result = (|| -> Result<(), DriveError> {
                let pkce = Pkce::new()?;
                let state = oauth::new_state()?;
                let listener = Listener::bind()?;
                let redirect = listener.redirect_uri();
                opener(&oauth::auth_url(&cfg, &redirect, &state, &pkce.challenge))?;
                let code = listener.wait_for_code(&state, Duration::from_secs(300), || cancel.load(Ordering::SeqCst))?;
                let tokens = oauth::exchange_code(http.as_ref(), &cfg, &code, &pkce.verifier, &redirect)?;
                drive.finish_sign_in(&tokens)
            })();
            let _ = tx.send(match result {
                Ok(()) => DriveEvent::Status(DriveStatus::SignedIn(drive.account())),
                Err(e) => DriveEvent::Failed(e),
            });
        });
    }

    pub fn cancel_sign_in(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Delete the tokens now, then tell Google in the background.
    pub fn sign_out(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        let Some(drive) = self.drive.clone() else { return };
        self.started = true;
        match drive.sign_out() {
            Ok(()) => self.status = DriveStatus::SignedOut,
            Err(e) => {
                let _ = self.tx.send(DriveEvent::Failed(e));
            }
        }
    }

    fn spawn(&self, f: impl FnOnce(Drive, Sender<DriveEvent>) + Send + 'static) {
        let Some(drive) = self.drive.clone() else { return };
        let tx = self.tx.clone();
        std::thread::spawn(move || f(drive, tx));
    }

    pub fn list(&mut self, folder: Option<String>) {
        self.spawn(move |d, tx| {
            let _ = tx.send(DriveEvent::Listing(d.list(folder.as_deref())));
        });
    }

    pub fn open(&mut self, file: DriveFile) {
        let dir = self.cache_dir.join(safe_name(&file.id));
        self.spawn(move |d, tx| {
            let r = (|| -> Result<PathBuf, DriveError> {
                let bytes = d.download(&file.id)?;
                std::fs::create_dir_all(&dir).map_err(|e| DriveError::Other(format!("cannot make the cache folder: {e}")))?;
                let path = dir.join(safe_name(&file.name));
                // Write beside, then rename, so a crash leaves no half-written deck.
                let tmp = path.with_extension("part");
                std::fs::write(&tmp, bytes)
                    .and_then(|_| std::fs::rename(&tmp, &path))
                    .map_err(|e| DriveError::Other(format!("cannot save the download: {e}")))?;
                Ok(path)
            })();
            let _ = tx.send(match r {
                Ok(path) => DriveEvent::Opened { file, path },
                Err(e) => DriveEvent::Failed(e),
            });
        });
    }

    /// Save as a new file in the DeckCraft folder, or update `existing` (id and the
    /// `modifiedTime` it was opened at) when saving over an earlier save.
    pub fn save(&mut self, name: String, bytes: Vec<u8>, existing: Option<(String, String)>) {
        self.spawn(move |d, tx| {
            let r = match existing {
                Some((id, opened_at)) => d.update(&id, bytes, &opened_at),
                None => d.app_folder().and_then(|f| d.create(&safe_name(&name), bytes, Some(&f.id))),
            };
            let _ = tx.send(match r {
                Ok(f) => DriveEvent::Saved(f),
                Err(e) => DriveEvent::Failed(e),
            });
        });
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Everything that has finished since the last call. A failure that means "signed out"
    /// also moves the status.
    pub fn poll(&mut self) -> Vec<DriveEvent> {
        let mut out = vec![];
        while let Ok(ev) = self.rx.try_recv() {
            match &ev {
                DriveEvent::Status(s) => self.status = s.clone(),
                DriveEvent::Failed(DriveError::SignedOut) => self.status = DriveStatus::SignedOut,
                DriveEvent::Failed(DriveError::Cancelled(_)) if self.status == DriveStatus::SigningIn => self.status = DriveStatus::SignedOut,
                DriveEvent::Failed(_) if self.status == DriveStatus::SigningIn => self.status = DriveStatus::SignedOut,
                _ => {}
            }
            out.push(ev);
        }
        out
    }

    pub fn has_tokens(&self) -> bool {
        matches!(self.store.load(), Ok(Some(_)))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Instant;

    use super::*;
    use crate::http::fake::Fake;
    use crate::oauth::{Tokens, dec};
    use crate::store::Memory;

    fn wait(svc: &mut DriveService, mut done: impl FnMut(&DriveEvent) -> bool) -> Vec<DriveEvent> {
        let start = Instant::now();
        let mut all = vec![];
        while start.elapsed() < Duration::from_secs(10) {
            let got = svc.poll();
            let finished = got.iter().any(&mut done);
            all.extend(got);
            if finished {
                return all;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        all
    }

    fn cfg() -> Result<Config, ConfigError> {
        Ok(Config { client_id: "cid".into(), client_secret: None })
    }

    /// A "browser" that does what a teacher would: reads the URL, and sends the redirect back.
    fn browser(code: &'static str, state_override: Option<&'static str>) -> Opener {
        Arc::new(move |url: &str| {
            let q = url.split_once('?').map(|x| x.1).unwrap_or("");
            let get = |k: &str| q.split('&').find_map(|p| p.strip_prefix(&format!("{k}="))).map(dec).unwrap_or_default();
            let redirect = get("redirect_uri");
            let state = state_override.map(String::from).unwrap_or_else(|| get("state"));
            let port: u16 = redirect.rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or(0);
            assert!(url.contains("code_challenge="));
            std::thread::spawn(move || {
                if let Ok(mut s) = TcpStream::connect(("127.0.0.1", port)) {
                    let _ = write!(s, "GET /?state={state}&code={code} HTTP/1.1\r\nHost: x\r\n\r\n");
                    let mut sink = String::new();
                    let _ = s.read_to_string(&mut sink);
                }
            });
            Ok(())
        })
    }

    #[test]
    fn without_a_client_id_it_says_how_to_set_one_up_and_never_signs_in() {
        let http = Arc::new(Fake::default());
        let mut svc =
            DriveService::new(http.clone(), Arc::new(Memory::default()), Err(ConfigError::Missing), browser("x", None), std::env::temp_dir());
        assert!(matches!(svc.status(), DriveStatus::NotConfigured(m) if m.contains("client ID")));
        svc.sign_in();
        assert!(matches!(svc.status(), DriveStatus::NotConfigured(_)));
        assert!(http.requests().is_empty(), "no client ID, no network");
    }

    #[test]
    fn a_full_sign_in_stores_tokens_and_reports_the_account() {
        let http = Arc::new(Fake::with(vec![
            Fake::json(200, r#"{"access_token":"A","refresh_token":"R","expires_in":3600}"#),
            Fake::json(200, r#"{"user":{"emailAddress":"teacher@example.org"}}"#),
        ]));
        let store = Arc::new(Memory::default());
        let mut svc = DriveService::new(http.clone(), store.clone(), cfg(), browser("the-code", None), std::env::temp_dir());
        svc.sign_in();
        assert_eq!(*svc.status(), DriveStatus::SigningIn);
        let evs = wait(&mut svc, |e| matches!(e, DriveEvent::Status(DriveStatus::SignedIn(_))));
        assert!(evs.iter().any(|e| matches!(e, DriveEvent::Status(DriveStatus::SignedIn(Some(a))) if a == "teacher@example.org")), "{evs:?}");
        assert_eq!(store.load().unwrap().unwrap().refresh_token, "R");
        let body = String::from_utf8(http.requests()[0].body.clone()).unwrap();
        assert!(body.contains("code=the-code") && body.contains("code_verifier="));
    }

    #[test]
    fn a_redirect_with_the_wrong_state_never_reaches_the_token_endpoint() {
        let http = Arc::new(Fake::default());
        let mut svc = DriveService::new(http.clone(), Arc::new(Memory::default()), cfg(), browser("stolen", Some("forged")), std::env::temp_dir());
        svc.sign_in();
        let evs = wait(&mut svc, |e| matches!(e, DriveEvent::Failed(_)));
        assert!(evs.iter().any(|e| matches!(e, DriveEvent::Failed(DriveError::Cancelled(_)))));
        assert_eq!(*svc.status(), DriveStatus::SignedOut);
        assert!(http.requests().is_empty());
    }

    #[test]
    fn signing_out_deletes_the_stored_token() {
        let http = Arc::new(Fake::default());
        let store = Arc::new(Memory::default());
        store.save(&Tokens { access_token: "A".into(), refresh_token: "R".into(), expires_at: u64::MAX }).unwrap();
        let mut svc = DriveService::new(http, store.clone(), cfg(), browser("x", None), std::env::temp_dir());
        svc.start();
        svc.sign_out();
        assert_eq!(*svc.status(), DriveStatus::SignedOut);
        assert!(store.load().unwrap().is_none(), "the token is gone from the store");
        assert!(!svc.has_tokens());
    }

    #[test]
    fn nothing_touches_the_keychain_until_drive_is_started() {
        struct Tripwire;
        impl TokenStore for Tripwire {
            fn load(&self) -> Result<Option<Tokens>, DriveError> {
                Err(DriveError::Other("the keychain was read".into()))
            }
            fn save(&self, _: &Tokens) -> Result<(), DriveError> {
                Err(DriveError::Other("the keychain was written".into()))
            }
            fn delete(&self) -> Result<(), DriveError> {
                Err(DriveError::Other("the keychain was written".into()))
            }
        }
        let svc = DriveService::new(Arc::new(Fake::default()), Arc::new(Tripwire), cfg(), browser("x", None), std::env::temp_dir());
        assert_eq!(*svc.status(), DriveStatus::SignedOut, "constructing the service reads nothing");
    }

    #[test]
    fn opening_downloads_into_the_cache_with_a_safe_name() {
        let http = Arc::new(Fake::with(vec![Ok(crate::http::Response { status: 200, headers: vec![], body: b"PKdata".to_vec() })]));
        let store = Arc::new(Memory::default());
        store.save(&Tokens { access_token: "A".into(), refresh_token: "R".into(), expires_at: u64::MAX }).unwrap();
        let cache = std::env::temp_dir().join(format!("deckcraft-drive-test-{}", std::process::id()));
        let mut svc = DriveService::new(http, store, cfg(), browser("x", None), cache.clone());
        svc.open(DriveFile { id: "id1".into(), name: "../../etc/evil.pptx".into(), mime_type: String::new(), modified_time: "t".into() });
        let evs = wait(&mut svc, |e| matches!(e, DriveEvent::Opened { .. } | DriveEvent::Failed(_)));
        let path = evs.iter().find_map(|e| if let DriveEvent::Opened { path, .. } = e { Some(path.clone()) } else { None }).expect("opened");
        assert!(path.starts_with(&cache), "{path:?}");
        assert_eq!(std::fs::read(&path).unwrap(), b"PKdata");
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn file_names_cannot_climb_out_of_the_cache() {
        for evil in ["../../x.pptx", "/etc/passwd", "a\\b", "..", "", "\u{0}"] {
            let n = safe_name(evil);
            assert!(!n.contains('/') && !n.contains('\\') && !n.starts_with('.') && !n.is_empty(), "{evil:?} -> {n:?}");
        }
        assert_eq!(safe_name("Lesson 1.pptx"), "Lesson 1.pptx");
    }
}
