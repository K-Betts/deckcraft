//! The Get Ready starter in the UI: only with Maths on, and Insert and Reroll make real slides.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::{Services, SlideApp};
use deckcraft_engine::Session;

fn harness() -> egui_kittest::Harness<'static, SlideApp> {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    app.ui.tab = "Insert".into();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 800.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    h
}

fn slides(h: &egui_kittest::Harness<'static, SlideApp>) -> usize {
    h.state().session.active().unwrap().doc.slides.len()
}

#[test]
fn the_insert_tab_shows_get_ready_only_with_the_maths_pack_on() {
    let mut h = harness();
    assert!(h.query_by_label_contains("Get Ready").is_none(), "a fresh install is neutral");
    h.state_mut().run("app.pack", json!({"id": "maths", "on": true})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label_contains("Get Ready").is_some());
    h.state_mut().run("app.pack", json!({"id": "maths", "on": false})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label_contains("Get Ready").is_none(), "and gone again when it is switched off");
}

#[test]
fn the_dialog_inserts_a_starter_and_rerolls_it_in_place() {
    let mut h = harness();
    let before = slides(&h);
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label("Reroll this slide").is_none(), "nothing to reroll yet");
    h.get_by_label("Insert").click();
    h.run_steps(3);
    assert_eq!(slides(&h), before + 1);
    assert!(h.state().dialog.is_none(), "the dialog closes");
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    h.get_by_label("Reroll this slide").click();
    h.run_steps(3);
    assert_eq!(slides(&h), before + 1, "reroll replaces, it does not add");
}
