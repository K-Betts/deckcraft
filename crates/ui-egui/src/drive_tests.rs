//! Google Drive in the interface: off by default, explicit actions only, honest about failure.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::json;

use crate::drive_ui::{DriveBackend, DriveItem, DriveNote, DriveState};
use crate::{Services, SlideApp};
use deckcraft_engine::Session;

#[derive(Default)]
struct Log {
    calls: Vec<String>,
    state: Option<DriveState>,
    queue: Vec<DriveNote>,
    saved_with: Option<(String, usize, Option<(String, String)>)>,
}

struct Fake(Rc<RefCell<Log>>);

impl DriveBackend for Fake {
    fn state(&self) -> DriveState {
        self.0.borrow().state.clone().unwrap_or(DriveState::SignedOut)
    }
    fn start(&mut self) {
        self.0.borrow_mut().calls.push("start".into());
    }
    fn sign_in(&mut self) {
        self.0.borrow_mut().calls.push("signIn".into());
    }
    fn cancel_sign_in(&mut self) {
        self.0.borrow_mut().calls.push("cancel".into());
    }
    fn sign_out(&mut self) {
        let mut l = self.0.borrow_mut();
        l.calls.push("signOut".into());
        l.state = Some(DriveState::SignedOut);
    }
    fn list(&mut self) {
        self.0.borrow_mut().calls.push("list".into());
    }
    fn open(&mut self, item: DriveItem) {
        self.0.borrow_mut().calls.push(format!("open {}", item.id));
    }
    fn save(&mut self, name: String, bytes: Vec<u8>, existing: Option<(String, String)>) {
        let mut l = self.0.borrow_mut();
        l.calls.push("save".into());
        l.saved_with = Some((name, bytes.len(), existing));
    }
    fn poll(&mut self) -> Vec<DriveNote> {
        std::mem::take(&mut self.0.borrow_mut().queue)
    }
}

fn app() -> (SlideApp, Rc<RefCell<Log>>) {
    let log = Rc::new(RefCell::new(Log::default()));
    let services = Services { drive: Some(Box::new(Fake(log.clone()))), ..Default::default() };
    (SlideApp::new(Session::with_new(), services), log)
}

#[test]
fn drive_is_off_on_a_fresh_install_and_commands_refuse_to_run() {
    let (mut app, log) = app();
    assert!(!app.ui.drive_enabled);
    assert!(app.drive_state().is_none());
    assert!(app.run("drive.signIn", json!({})).is_err());
    assert!(app.run("drive.save", json!({})).is_err());
    app.poll_drive();
    assert!(log.borrow().calls.is_empty(), "nothing is asked of the host while Drive is off");
}

#[test]
fn switching_it_on_does_not_sign_in_or_touch_the_keychain() {
    let (mut app, log) = app();
    app.run("app.drive", json!({"on": true})).unwrap();
    assert!(app.drive_state().is_some());
    assert!(log.borrow().calls.is_empty(), "sign-in waits for the button");
    app.run("app.drive", json!({"on": false})).unwrap();
    assert!(app.drive_state().is_none());
}

#[test]
fn saving_uploads_pptx_bytes_and_remembers_the_drive_file_for_the_next_save() {
    let (mut app, log) = app();
    app.run("app.drive", json!({"on": true})).unwrap();
    app.run("drive.save", json!({})).unwrap();
    {
        let l = log.borrow();
        let (name, len, existing) = l.saved_with.clone().unwrap();
        assert!(name.ends_with(".pptx"));
        assert!(len > 1000, "a real .pptx went out");
        assert!(existing.is_none(), "first save creates");
    }
    app.session.execute("file.saveAs", &json!({"path": std::env::temp_dir().join("drive-ui-test.pptx").to_str().unwrap()})).unwrap();
    log.borrow_mut().queue.push(DriveNote::Saved { id: "F1".into(), name: "x.pptx".into(), modified: "T1".into() });
    app.poll_drive();
    assert!(app.drive.message.as_ref().is_some_and(|(err, m)| !err && m.contains("Saved")));
    app.run("drive.save", json!({})).unwrap();
    assert_eq!(log.borrow().saved_with.clone().unwrap().2, Some(("F1".to_string(), "T1".to_string())), "the second save updates the same file");
    let _ = std::fs::remove_file(std::env::temp_dir().join("drive-ui-test.pptx"));
}

#[test]
fn a_failed_save_is_shown_to_the_teacher_and_changes_nothing_locally() {
    let (mut app, log) = app();
    app.run("app.drive", json!({"on": true})).unwrap();
    app.run("drive.save", json!({})).unwrap();
    log.borrow_mut().queue.push(DriveNote::Failed("Can't reach Google Drive (no network?): dns".into()));
    app.poll_drive();
    assert!(app.drive.message.as_ref().is_some_and(|(err, m)| *err && m.contains("no network")));
    assert!(!app.drive.busy);
    assert!(app.session.active().is_some_and(|s| s.path.is_none()), "the local presentation is untouched");
}

#[test]
fn signing_out_clears_the_list_and_every_remembered_link() {
    let (mut app, log) = app();
    app.run("app.drive", json!({"on": true})).unwrap();
    app.drive.files = Some(Ok(vec![]));
    app.drive.links.insert("/cache/a.pptx".into(), ("F".into(), "T".into()));
    app.run("drive.signOut", json!({})).unwrap();
    assert!(log.borrow().calls.contains(&"signOut".to_string()));
    assert!(app.drive.files.is_none() && app.drive.links.is_empty());
    assert!(app.drive.message.as_ref().is_some_and(|(_, m)| m.contains("deleted")));
}

#[test]
fn a_downloaded_deck_opens_and_is_linked_but_not_opened_twice() {
    let (mut app, log) = app();
    app.run("app.drive", json!({"on": true})).unwrap();
    let mut seed = Session::with_new();
    let bytes = deckcraft_engine::cmd::file::save_bytes(&seed.active().unwrap().doc, "pptx").unwrap();
    let _ = &mut seed;
    let path = std::env::temp_dir().join(format!("drive-ui-open-{}.pptx", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    let note = || DriveNote::Opened { id: "F9".into(), name: "L.pptx".into(), modified: "T9".into(), path: path.to_string_lossy().into_owned() };
    log.borrow_mut().queue.push(note());
    app.poll_drive();
    assert_eq!(app.drive.links.get(path.to_str().unwrap()), Some(&("F9".to_string(), "T9".to_string())));
    let docs = app.session.documents().len();
    log.borrow_mut().queue.push(note());
    app.poll_drive();
    assert_eq!(app.session.documents().len(), docs, "the same Drive file is not opened in two places");
    assert!(app.drive.message.as_ref().is_some_and(|(err, m)| *err && m.contains("already open")));
    let _ = std::fs::remove_file(&path);
}

fn prefs_harness(app: SlideApp) -> egui_kittest::Harness<'static, SlideApp> {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.state_mut().run("app.preferences", json!({})).unwrap();
    h.run_steps(3);
    h
}

#[test]
fn only_builds_with_drive_offer_the_drive_switch_in_preferences() {
    use egui_kittest::kittest::Queryable;
    let with = prefs_harness(app().0);
    assert!(with.query_by_label_contains("Use Google Drive").is_some());
    let without = prefs_harness(SlideApp::new(Session::with_new(), Services::default()));
    assert!(without.query_by_label_contains("Use Google Drive").is_none(), "the web version has no Drive, so it does not offer it");
    assert!(without.query_by_label_contains("Maths tools").is_some(), "the rest of Preferences is still there");
}
