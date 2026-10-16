//! Subject packs add and never rearrange; a fresh install is neutral.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use serde_json::json;

use crate::{Services, SlideApp, UiState, packs};

fn app() -> SlideApp {
    SlideApp::new(Session::with_new(), Services::default())
}

#[test]
fn a_fresh_install_has_no_pack_on() {
    let app = app();
    assert!(app.ui.packs.is_empty());
    assert!(!app.pack_on(packs::MATHS));
}

#[test]
fn switching_a_pack_on_and_off_restores_the_ui_state_exactly() {
    let mut app = app();
    let before = serde_json::to_value(&app.ui).unwrap();
    app.run("app.pack", json!({"id": "maths", "on": true})).unwrap();
    assert!(app.pack_on("maths"));
    app.run("app.pack", json!({"id": "maths"})).unwrap();
    assert!(!app.pack_on("maths"));
    assert_eq!(serde_json::to_value(&app.ui).unwrap(), before, "on then off leaves nothing behind");
}

#[test]
fn an_unknown_pack_is_an_error_and_unknown_saved_ids_are_dropped() {
    let mut app = app();
    assert!(app.run("app.pack", json!({"id": "alchemy"})).is_err());
    let ui: UiState = serde_json::from_str(r#"{"packs":["maths","alchemy"]}"#).unwrap();
    let ui = ui.sanitized();
    assert_eq!(ui.packs.into_iter().collect::<Vec<_>>(), ["maths"]);
}

#[test]
fn every_pack_declares_what_it_contributes() {
    for p in packs::PACKS {
        assert!(!p.slots.is_empty() && !p.label.is_empty() && !p.blurb.is_empty(), "{}", p.id);
    }
}

fn home_harness() -> egui_kittest::Harness<'static, SlideApp> {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app(),
    );
    h.state_mut().run("app.home", json!({"on": true})).unwrap();
    h.run_steps(3);
    h
}

#[test]
fn home_offers_the_subject_on_a_fresh_install_and_one_tap_turns_it_on() {
    use egui_kittest::kittest::Queryable;
    let mut h = home_harness();
    assert!(h.query_by_label_contains("Teaching a subject?").is_some());
    h.get_by_label("Turn on Maths").click();
    h.run_steps(3);
    assert!(h.state().pack_on("maths"));
    assert!(h.query_by_label_contains("Teaching a subject?").is_none(), "the card goes once it has been used");
}

#[test]
fn not_now_dismisses_the_card_for_good_without_turning_anything_on() {
    use egui_kittest::kittest::Queryable;
    let mut h = home_harness();
    h.get_by_label("Not now").click();
    h.run_steps(3);
    assert!(h.state().ui.packs.is_empty());
    assert!(h.state().ui.subjects_hint_dismissed);
    assert!(h.query_by_label_contains("Teaching a subject?").is_none());
}

#[test]
fn cancelling_preferences_puts_the_subjects_and_links_switches_back() {
    use egui_kittest::kittest::Queryable;
    let mut h = home_harness();
    h.state_mut().run("app.home", json!({"on": false})).unwrap();
    h.state_mut().run("app.preferences", json!({})).unwrap();
    h.run_steps(3);
    h.get_by_label("Maths tools").click();
    h.run_steps(3);
    assert!(h.state().pack_on("maths"));
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert!(!h.state().pack_on("maths"), "Cancel undoes the switch");
}
