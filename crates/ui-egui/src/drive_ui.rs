//! Google Drive in the interface: a switch in Preferences (off by default), and one section on
//! File ▸ Account for signing in, browsing what DeckCraft has saved, opening, and saving.
//!
//! The network lives in the host (`deckcraft-drive`, injected through `Services::drive`); this
//! file only shows state and runs commands. Nothing leaves the machine except by a button the
//! teacher presses here, and signing out deletes the stored token.

use std::collections::HashMap;

use egui::{RichText, Ui};
use serde_json::{Value, json};

use crate::SlideApp;
use crate::theme::{self, Tokens};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriveState {
    /// No client ID is configured; the text says how to add one.
    NotConfigured(String),
    SignedOut,
    SigningIn,
    SignedIn(Option<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DriveItem {
    pub id: String,
    pub name: String,
    /// Drive's `modifiedTime`, compared as text to spot a change.
    pub modified: String,
    pub is_folder: bool,
}

#[derive(Debug)]
pub enum DriveNote {
    State(DriveState),
    Listing(Result<Vec<DriveItem>, String>),
    Opened { id: String, name: String, modified: String, path: String },
    Saved { id: String, name: String, modified: String },
    Failed(String),
}

/// What the host provides. Every method returns at once; results arrive through `poll`.
pub trait DriveBackend {
    fn state(&self) -> DriveState;
    /// First use: look in the keychain (not at launch).
    fn start(&mut self);
    fn sign_in(&mut self);
    fn cancel_sign_in(&mut self);
    fn sign_out(&mut self);
    fn list(&mut self);
    fn open(&mut self, item: DriveItem);
    /// Save as a new Drive file, or update `(id, modified-when-opened)`.
    fn save(&mut self, name: String, bytes: Vec<u8>, existing: Option<(String, String)>);
    fn poll(&mut self) -> Vec<DriveNote>;
}

/// What the interface remembers about Drive between frames.
#[derive(Default)]
pub struct DriveUi {
    pub files: Option<Result<Vec<DriveItem>, String>>,
    /// Local path of a downloaded deck → `(Drive file id, modifiedTime when it was opened)`.
    pub links: HashMap<String, (String, String)>,
    /// The last thing that happened, for the Account page: `(is_error, text)`.
    pub message: Option<(bool, String)>,
    pub busy: bool,
}

impl SlideApp {
    pub fn drive_state(&self) -> Option<DriveState> {
        self.ui.drive_enabled.then(|| self.services.drive.as_ref().map(|d| d.state())).flatten()
    }

    /// Take what the host finished: called every frame while Drive is on.
    pub fn poll_drive(&mut self) {
        if !self.ui.drive_enabled {
            return;
        }
        let Some(backend) = self.services.drive.as_mut() else { return };
        let notes = backend.poll();
        for note in notes {
            self.drive.busy = false;
            match note {
                DriveNote::State(_) => {}
                DriveNote::Listing(r) => self.drive.files = Some(r),
                DriveNote::Failed(e) => self.drive.message = Some((true, e)),
                DriveNote::Saved { id, name, modified } => {
                    if let Some(path) = self.session.active().and_then(|s| s.path.clone()) {
                        self.drive.links.insert(path, (id, modified));
                    }
                    self.drive.message = Some((false, format!("Saved “{name}” to Google Drive.")));
                    if let Some(b) = self.services.drive.as_mut() {
                        b.list();
                    }
                }
                DriveNote::Opened { id, name, modified, path } => {
                    if self.session.documents().iter().any(|d| d.path.as_deref() == Some(path.as_str())) {
                        self.drive.message = Some((true, format!("“{name}” is already open. Close it first to open the Drive copy again.")));
                        continue;
                    }
                    match self.run("file.open", json!({"path": path})) {
                        Ok(_) => {
                            self.drive.links.insert(path, (id, modified));
                            self.drive.message = Some((false, format!("Opened “{name}” from Google Drive.")));
                            self.close_backstage();
                        }
                        Err(e) => self.drive.message = Some((true, e)),
                    }
                }
            }
        }
    }

    /// Run a Drive command (`drive.signIn`, `drive.signOut`, `drive.refresh`, `drive.open`,
    /// `drive.save`, `drive.cancel`).
    pub(crate) fn run_drive(&mut self, id: &str, p: &Value) -> Result<Value, String> {
        if !self.ui.drive_enabled {
            return Err("Google Drive is switched off (Preferences > Google Drive)".into());
        }
        let Some(backend) = self.services.drive.as_mut() else { return Err("Google Drive is not available in this build".into()) };
        backend.start();
        match id {
            "drive.signIn" => backend.sign_in(),
            "drive.cancel" => backend.cancel_sign_in(),
            "drive.signOut" => {
                backend.sign_out();
                self.drive.files = None;
                self.drive.links.clear();
                self.drive.message = Some((false, "Signed out. The stored sign-in was deleted from this Mac.".into()));
            }
            "drive.refresh" => {
                self.drive.busy = true;
                backend.list();
            }
            "drive.open" => {
                let s = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
                let item = DriveItem { id: s("id"), name: s("name"), modified: s("modified"), is_folder: false };
                if item.id.is_empty() {
                    return Err("`id` is required".into());
                }
                self.drive.busy = true;
                backend.open(item);
            }
            "drive.save" => {
                let Some(st) = self.session.active() else { return Err("no presentation is open".into()) };
                let bytes = deckcraft_engine::cmd::file::save_bytes(&st.doc, "pptx").map_err(|e| e.to_string())?;
                let existing = st.path.as_ref().and_then(|p| self.drive.links.get(p)).cloned();
                let stem = st.title.trim_end_matches(".pptx").trim_end_matches(".deckcraft");
                let name = format!("{}.pptx", if stem.is_empty() { "Presentation" } else { stem });
                self.drive.busy = true;
                backend.save(name, bytes, existing);
            }
            _ => return Err(format!("unknown Drive command {id}")),
        }
        Ok(Value::Null)
    }
}

/// The “Google Drive” block on File ▸ Account.
pub fn section(app: &mut SlideApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(16.0);
    ui.label(RichText::new("Google Drive").font(theme::bold(15.0)).color(t.text));
    let Some(state) = app.drive_state() else {
        ui.label(RichText::new("Not available in this build.").color(t.text_dim));
        return;
    };
    ui.label(
        RichText::new(
            "Optional. DeckCraft can only see files it saved to your Drive. Nothing is uploaded unless you press Save to Google Drive, and signing out deletes the stored sign-in.",
        )
        .size(12.0)
        .color(t.text_dim),
    );
    ui.add_space(6.0);
    match state {
        DriveState::NotConfigured(why) => {
            ui.label(RichText::new(format!("Not set up: {why}")).color(t.text_dim));
        }
        DriveState::SignedOut => {
            if ui.button("Sign in with Google…").clicked() {
                let _ = app.run("drive.signIn", json!({}));
            }
        }
        DriveState::SigningIn => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Waiting for you to finish in your browser…");
                if ui.button("Cancel").clicked() {
                    let _ = app.run("drive.cancel", json!({}));
                }
            });
        }
        DriveState::SignedIn(who) => {
            ui.horizontal(|ui| {
                ui.label(format!("Signed in{}", who.map(|w| format!(" as {w}")).unwrap_or_default()));
                if ui.button("Sign out").clicked() {
                    let _ = app.run("drive.signOut", json!({}));
                }
            });
            ui.horizontal(|ui| {
                let can_save = app.session.active().is_some();
                if ui.add_enabled(can_save && !app.drive.busy, egui::Button::new("Save to Google Drive")).clicked() {
                    let _ = app.run("drive.save", json!({}));
                }
                if ui.add_enabled(!app.drive.busy, egui::Button::new("Refresh list")).clicked() {
                    let _ = app.run("drive.refresh", json!({}));
                }
                if app.drive.busy {
                    ui.spinner();
                }
            });
            if app.drive.files.is_none() && !app.drive.busy {
                let _ = app.run("drive.refresh", json!({}));
            }
            match app.drive.files.clone() {
                Some(Ok(files)) if files.is_empty() => {
                    ui.label(RichText::new("Nothing saved from DeckCraft yet.").color(t.text_dim));
                }
                Some(Ok(files)) => {
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for f in files.iter().filter(|f| !f.is_folder) {
                            ui.horizontal(|ui| {
                                if ui.button("Open").clicked() {
                                    let _ = app.run("drive.open", json!({"id": f.id, "name": f.name, "modified": f.modified}));
                                }
                                ui.label(&f.name);
                            });
                        }
                    });
                }
                Some(Err(e)) => {
                    ui.colored_label(t.accent, e);
                }
                None => {}
            }
        }
    }
    if let Some((is_err, text)) = &app.drive.message {
        ui.add_space(4.0);
        if *is_err {
            ui.colored_label(egui::Color32::from_rgb(200, 60, 60), text);
        } else {
            ui.label(RichText::new(text).color(t.text_dim));
        }
    }
}
