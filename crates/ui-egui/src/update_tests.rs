//! Updating from inside the app: silent until asked, explicit to install, careful with unsaved work.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::update_ui::{UpdateBackend, UpdateState};
use crate::{Services, SlideApp};
use deckcraft_engine::Session;

#[derive(Default)]
struct Log {
    calls: Vec<&'static str>,
    state: Option<UpdateState>,
}

struct Fake(Rc<RefCell<Log>>);

impl UpdateBackend for Fake {
    fn state(&self) -> UpdateState {
        self.0.borrow().state.clone().unwrap_or(UpdateState::Idle)
    }
    fn current(&self) -> Option<String> {
        Some("9ebdf74".into())
    }
    fn check(&mut self) {
        let mut l = self.0.borrow_mut();
        l.calls.push("check");
        l.state = Some(UpdateState::Checking);
    }
    fn install(&mut self) {
        let mut l = self.0.borrow_mut();
        l.calls.push("install");
        l.state = Some(UpdateState::Installing);
    }
    fn restart(&mut self) -> Result<(), String> {
        self.0.borrow_mut().calls.push("restart");
        Ok(())
    }
    fn poll(&mut self) -> bool {
        self.0.borrow_mut().calls.push("poll");
        false
    }
}

fn available() -> UpdateState {
    UpdateState::Available { label: "build-abc1234".into(), sha: "abc1234".into(), notes: "Per-question Get Ready".into() }
}

fn app() -> (SlideApp, Rc<RefCell<Log>>) {
    let log = Rc::new(RefCell::new(Log::default()));
    let services = Services { updates: Some(Box::new(Fake(log.clone()))), ..Default::default() };
    (SlideApp::new(Session::with_new(), services), log)
}

fn count(log: &Rc<RefCell<Log>>, what: &str) -> usize {
    log.borrow().calls.iter().filter(|c| **c == what).count()
}

#[test]
fn nothing_is_checked_on_a_fresh_install() {
    let (mut app, log) = app();
    assert!(!app.ui.check_for_updates);
    for _ in 0..3 {
        app.poll_updates();
    }
    assert_eq!(count(&log, "check"), 0, "no network until the teacher asks");
}

#[test]
fn the_start_up_check_runs_once_and_only_when_switched_on() {
    let (mut app, log) = app();
    app.run("app.updates", json!({"on": true})).unwrap();
    app.poll_updates();
    app.poll_updates();
    assert_eq!(count(&log, "check"), 1);
}

#[test]
fn installing_needs_a_found_update_and_checking_is_a_command() {
    let (mut app, log) = app();
    assert!(app.run("update.install", json!({})).is_err(), "nothing found yet");
    assert_eq!(count(&log, "install"), 0);
    app.run("update.check", json!({})).unwrap();
    assert_eq!(count(&log, "check"), 1);
    log.borrow_mut().state = Some(available());
    app.run("update.install", json!({})).unwrap();
    assert_eq!(count(&log, "install"), 1);
}

#[test]
fn restarting_is_refused_while_there_is_unsaved_work() {
    let (mut app, log) = app();
    log.borrow_mut().state = Some(UpdateState::Installed);
    app.session.execute("slide.new", &json!({})).unwrap();
    assert!(app.session.documents().iter().any(|d| d.is_dirty()));
    let err = app.run("update.restart", json!({})).unwrap_err();
    assert!(err.contains("save your work"), "{err}");
    assert_eq!(count(&log, "restart"), 0);
    assert!(!app.pending_quit);
}

#[test]
fn restarting_with_nothing_unsaved_asks_the_host_then_closes_the_window() {
    let (mut app, log) = app();
    log.borrow_mut().state = Some(UpdateState::Installed);
    assert!(!app.session.documents().iter().any(|d| d.is_dirty()));
    app.run("update.restart", json!({})).unwrap();
    assert_eq!(count(&log, "restart"), 1);
    assert!(app.pending_quit && app.quit_confirmed);
}

#[test]
fn the_status_bar_and_dialog_tell_the_teacher_an_update_is_ready() {
    let (app, log) = app();
    log.borrow_mut().state = Some(available());
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 800.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    assert!(h.query_by_label("Update available").is_some(), "a chip in the status bar");
    h.state_mut().run("app.dialog", json!({"id": "update"})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label("A new version of EduDeck is ready.").is_some());
    assert!(h.query_by_label_contains("Per-question Get Ready").is_some(), "the notes are shown");
    h.get_by_label("Update now").click();
    h.run_steps(3);
    assert_eq!(count(&log, "install"), 1);
}

#[test]
fn no_chip_when_up_to_date_and_none_without_a_host_backend() {
    let (app, log) = app();
    log.borrow_mut().state = Some(UpdateState::UpToDate);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 800.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    assert!(h.query_by_label("Update available").is_none());
    let mut bare = SlideApp::new(Session::with_new(), Services::default());
    assert!(bare.update_state().is_none());
    assert!(bare.run("update.check", json!({})).is_err(), "a build without updates says so");
}

#[test]
fn the_restart_refusal_names_edudeck() {
    let (mut app, log) = app();
    log.borrow_mut().state = Some(UpdateState::Installed);
    app.session.execute("slide.new", &json!({})).unwrap();
    let err = app.run("update.restart", json!({})).unwrap_err();
    assert!(err.contains("restarting closes EduDeck"), "{err}");
}
