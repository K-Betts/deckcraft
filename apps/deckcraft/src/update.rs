//! Connects the interface's update dialog to `deckcraft-update`.
//!
//! Native builds only. Nothing here talks to the network until the teacher turns on *Check for
//! updates* in Preferences or presses *Check now*, and nothing is installed until they press
//! *Update now*.

use std::path::PathBuf;
use std::sync::Arc;

use deckcraft_ui_egui::update_ui::{UpdateBackend, UpdateState};
use deckcraft_update::net::Ureq;
use deckcraft_update::release::REPO;
use deckcraft_update::service::{State, Updater};

pub struct Backend(Updater);

pub fn backend(work_dir: PathBuf) -> Backend {
    let sha = option_env!("DECKCRAFT_BUILD_SHA").filter(|s| s.len() >= 7 && *s != "unknown").map(String::from);
    Backend(Updater::new(Arc::new(Ureq::default()), REPO, sha, work_dir))
}

impl UpdateBackend for Backend {
    fn state(&self) -> UpdateState {
        match self.0.state() {
            State::Idle => UpdateState::Idle,
            State::Checking => UpdateState::Checking,
            State::UpToDate => UpdateState::UpToDate,
            State::Available(a) => UpdateState::Available { label: a.label.clone(), sha: a.sha.clone(), notes: a.notes.clone() },
            State::Installing => UpdateState::Installing,
            State::Installed => UpdateState::Installed,
            State::Failed(m) => UpdateState::Failed(m.clone()),
        }
    }

    fn current(&self) -> Option<String> {
        self.0.current_sha().map(String::from)
    }

    fn check(&mut self) {
        self.0.check();
    }

    fn install(&mut self) {
        self.0.install();
    }

    fn restart(&mut self) -> Result<(), String> {
        self.0.restart().map_err(|e| e.to_string())
    }

    fn poll(&mut self) -> bool {
        self.0.poll()
    }
}
