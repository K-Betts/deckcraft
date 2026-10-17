//! Check and install as background work, for a UI that must never block.
//!
//! The UI calls a method (it returns at once) and drains [`Updater::poll`] each frame.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::UpdateError;
use crate::install;
use crate::net::Http;
use crate::release::{self, Available};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Checking,
    UpToDate,
    Available(Available),
    /// Downloading, checking and swapping in the new app.
    Installing,
    /// The new app is in place; restarting uses it.
    Installed,
    Failed(String),
}

enum Event {
    Checked(Result<Option<Available>, UpdateError>),
    Installed,
    Failed(UpdateError),
}

pub struct Updater {
    http: Arc<dyn Http>,
    repo: String,
    /// The running app's commit, if this build knows it.
    current_sha: Option<String>,
    work_dir: PathBuf,
    state: State,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl Updater {
    pub fn new(http: Arc<dyn Http>, repo: &str, current_sha: Option<String>, work_dir: PathBuf) -> Self {
        let (tx, rx) = channel();
        Updater { http, repo: repo.to_string(), current_sha, work_dir, state: State::Idle, tx, rx }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn current_sha(&self) -> Option<&str> {
        self.current_sha.as_deref().map(|s| s.get(..7).unwrap_or(s))
    }

    /// Ask GitHub whether a newer version exists.
    pub fn check(&mut self) {
        if matches!(self.state, State::Checking | State::Installing) {
            return;
        }
        self.state = State::Checking;
        let (http, repo, current, tx) = (self.http.clone(), self.repo.clone(), self.current_sha.clone(), self.tx.clone());
        std::thread::spawn(move || {
            let r = release::check(http.as_ref(), &repo, std::env::consts::OS, std::env::consts::ARCH, current.as_deref());
            let _ = tx.send(Event::Checked(r));
        });
    }

    /// Download, verify and install the available version. Does nothing unless one was found.
    pub fn install(&mut self) {
        let State::Available(avail) = self.state.clone() else { return };
        self.state = State::Installing;
        let (http, dir, tx) = (self.http.clone(), self.work_dir.clone(), self.tx.clone());
        std::thread::spawn(move || {
            let r = (|| {
                let app = install::current_app_bundle().ok_or_else(|| {
                    UpdateError::Unsupported(
                        "This copy is not an installed DeckCraft.app, so it cannot replace itself. Use packaging/macos/update-local.sh.".into(),
                    )
                })?;
                let zip = install::download(http.as_ref(), &avail, &dir)?;
                let done = install::install(&zip, &app, true);
                let _ = std::fs::remove_file(&zip);
                done
            })();
            let _ = tx.send(match r {
                Ok(()) => Event::Installed,
                Err(e) => Event::Failed(e),
            });
        });
    }

    /// Start the installed new version a moment after this one quits.
    pub fn restart(&self) -> Result<(), UpdateError> {
        let app = install::current_app_bundle().ok_or_else(|| UpdateError::Unsupported("not an installed app".into()))?;
        install::relaunch(&app)
    }

    /// Move the state on with whatever finished since the last call. True if the state changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(ev) = self.rx.try_recv() {
            changed = true;
            self.state = match ev {
                Event::Checked(Ok(Some(a))) => State::Available(a),
                Event::Checked(Ok(None)) => State::UpToDate,
                Event::Checked(Err(e)) | Event::Failed(e) => State::Failed(e.to_string()),
                Event::Installed => State::Installed,
            };
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use std::time::{Duration, Instant};

    use super::*;
    use crate::net::fake::Fake;

    #[cfg(target_os = "macos")]
    const URL: &str = "https://api.github.com/repos/K-Betts/deckcraft/releases/latest";

    fn wait(u: &mut Updater, done: impl Fn(&State) -> bool) {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            u.poll();
            if done(u.state()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[cfg(target_os = "macos")]
    fn json(sha: &str) -> String {
        let z = format!("DeckCraft-macos-{}-{sha}.zip", std::env::consts::ARCH);
        format!(
            r#"{{"tag_name":"build-{sha}","assets":[{{"name":"{z}","size":9,"browser_download_url":"https://github.com/a/b/{z}"}},{{"name":"{z}.sha256","size":9,"browser_download_url":"https://github.com/a/b/{z}.sha256"}}]}}"#
        )
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn checking_finds_a_newer_build_and_up_to_date_is_reported() {
        let newer = Arc::new(Fake::default().with(URL, 200, json("abc1234")));
        let mut u = Updater::new(newer, "K-Betts/deckcraft", Some("9ebdf74aaaa".into()), std::env::temp_dir());
        assert_eq!(*u.state(), State::Idle);
        u.check();
        assert_eq!(*u.state(), State::Checking);
        wait(&mut u, |s| matches!(s, State::Available(_)));
        assert!(matches!(u.state(), State::Available(a) if a.sha == "abc1234"));
        let same = Arc::new(Fake::default().with(URL, 200, json("abc1234")));
        let mut v = Updater::new(same, "K-Betts/deckcraft", Some("abc1234zzz".into()), std::env::temp_dir());
        v.check();
        wait(&mut v, |s| *s == State::UpToDate);
        assert_eq!(*v.state(), State::UpToDate);
    }

    #[test]
    fn a_failed_check_is_a_message_not_a_crash() {
        let mut u = Updater::new(Arc::new(Fake::default()), "K-Betts/deckcraft", Some("abc1234".into()), std::env::temp_dir());
        u.check();
        wait(&mut u, |s| matches!(s, State::Failed(_)));
        assert!(matches!(u.state(), State::Failed(m) if m.contains("GitHub") || m.contains("Mac")), "{:?}", u.state());
    }

    #[test]
    fn install_does_nothing_until_an_update_was_found() {
        let http = Arc::new(Fake::default());
        let mut u = Updater::new(http.clone(), "K-Betts/deckcraft", Some("abc1234".into()), std::env::temp_dir());
        u.install();
        assert_eq!(*u.state(), State::Idle);
        assert!(http.urls().is_empty(), "no request without a found update");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn installing_from_a_copy_that_is_not_the_installed_app_is_refused_without_downloading() {
        let http = Arc::new(Fake::default().with(URL, 200, json("abc1234")));
        let mut u = Updater::new(http.clone(), "K-Betts/deckcraft", Some("9ebdf74".into()), std::env::temp_dir());
        u.check();
        wait(&mut u, |s| matches!(s, State::Available(_)));
        u.install();
        wait(&mut u, |s| matches!(s, State::Failed(_)));
        assert!(matches!(u.state(), State::Failed(m) if m.contains("installed DeckCraft.app")), "{:?}", u.state());
        assert_eq!(http.urls().len(), 1, "only the check went out; nothing was downloaded");
    }
}
