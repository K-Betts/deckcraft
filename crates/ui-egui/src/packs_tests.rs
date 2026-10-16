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
