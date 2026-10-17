//! The top layer carries no links to another product unless the teacher asks for them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use serde_json::json;

use crate::{Services, SlideApp};

#[test]
fn community_links_are_off_on_a_fresh_install_and_toggle_by_command() {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    assert!(!app.ui.show_community_links);
    app.run("app.communityLinks", json!({})).unwrap();
    assert!(app.ui.show_community_links);
    app.run("app.communityLinks", json!({"on": false})).unwrap();
    assert!(!app.ui.show_community_links);
}

#[test]
fn a_ui_json_written_before_the_setting_existed_still_loads_with_links_off() {
    let ui: crate::UiState = serde_json::from_str(r#"{"tab":"Home"}"#).unwrap();
    assert!(!ui.show_community_links);
}
