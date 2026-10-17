//! Connects the interface's Google Drive switch to `deckcraft-drive`.
//!
//! Built only for native targets. Nothing here runs until the teacher switches Google Drive on in
//! Preferences and presses Sign in.

use std::path::PathBuf;
use std::sync::Arc;

use deckcraft_drive::api::DriveFile;
use deckcraft_drive::http::Ureq;
use deckcraft_drive::service::{DriveEvent, DriveService, DriveStatus};
use deckcraft_drive::store::Keychain;
use deckcraft_ui_egui::drive_ui::{DriveBackend, DriveItem, DriveNote, DriveState};

pub struct Backend(DriveService);

pub fn backend(cache_dir: PathBuf) -> Backend {
    let opener: deckcraft_drive::service::Opener = Arc::new(|url: &str| deckcraft_drive::signin::open_in_browser(url));
    Backend(DriveService::new(Arc::new(Ureq::default()), Arc::new(Keychain), deckcraft_drive::config::from_process(), opener, cache_dir))
}

fn state_of(s: &DriveStatus) -> DriveState {
    match s {
        DriveStatus::NotConfigured(m) => DriveState::NotConfigured(m.clone()),
        DriveStatus::SignedOut => DriveState::SignedOut,
        DriveStatus::SigningIn => DriveState::SigningIn,
        DriveStatus::SignedIn(who) => DriveState::SignedIn(who.clone()),
    }
}

fn item_of(f: DriveFile) -> DriveItem {
    DriveItem { is_folder: f.is_folder(), id: f.id, name: f.name, modified: f.modified_time }
}

impl DriveBackend for Backend {
    fn state(&self) -> DriveState {
        state_of(self.0.status())
    }

    fn start(&mut self) {
        self.0.start();
    }

    fn sign_in(&mut self) {
        self.0.sign_in();
    }

    fn cancel_sign_in(&mut self) {
        self.0.cancel_sign_in();
    }

    fn sign_out(&mut self) {
        self.0.sign_out();
    }

    fn list(&mut self) {
        self.0.list(None);
    }

    fn open(&mut self, item: DriveItem) {
        self.0.open(DriveFile { id: item.id, name: item.name, mime_type: String::new(), modified_time: item.modified });
    }

    fn save(&mut self, name: String, bytes: Vec<u8>, existing: Option<(String, String)>) {
        self.0.save(name, bytes, existing);
    }

    fn poll(&mut self) -> Vec<DriveNote> {
        self.0
            .poll()
            .into_iter()
            .map(|e| match e {
                DriveEvent::Status(s) => DriveNote::State(state_of(&s)),
                DriveEvent::Listing(r) => DriveNote::Listing(r.map(|v| v.into_iter().map(item_of).collect()).map_err(|e| e.to_string())),
                DriveEvent::Opened { file, path } => {
                    DriveNote::Opened { id: file.id, name: file.name, modified: file.modified_time, path: path.to_string_lossy().into_owned() }
                }
                DriveEvent::Saved(f) => DriveNote::Saved { id: f.id, name: f.name, modified: f.modified_time },
                DriveEvent::Failed(e) => DriveNote::Failed(e.to_string()),
            })
            .collect()
    }
}
