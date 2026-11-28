//! Regression test for the maths edition's fix to the clipboard shortcuts (see
//! `docs/fork-changes.md`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use serde_json::json;

use crate::{OBJECTS_MARKER, Services, SlideApp};

/// Cmd+C, Cmd+X and Cmd+V did nothing most of the time: a widget that is not a text field
/// (a ribbon button reached by Tab or the arrow keys) held egui's keyboard focus, and
/// `keyboard` gave up on every event while any widget had it.
#[test]
fn clipboard_shortcuts_work_while_a_non_text_widget_has_focus() {
    let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SlideApp::new(Session::with_new(), Services::default()),
    );
    harness.run_steps(2);
    harness.state_mut().session.execute("shape.insert", &json!({"preset": "rect", "rect": [10, 10, 50, 50]})).unwrap();
    let count = |h: &egui_kittest::Harness<'_, SlideApp>| h.state().session.doc().unwrap().current_slide().unwrap().shapes.len();
    let copied = |h: &egui_kittest::Harness<'_, SlideApp>| {
        h.output().platform_output.commands.iter().find_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
    };
    let before = count(&harness);
    // A real ribbon button, as Tab or the arrow keys leave it.
    use egui_kittest::kittest::Queryable;
    harness.get_by_role_and_label(egui::accesskit::Role::Button, "Light/Dark appearance").focus();
    harness.run_steps(1);
    assert!(harness.ctx.memory(|m| m.focused()).is_some(), "the ribbon button holds focus");
    assert!(!harness.ctx.text_edit_focused());

    harness.event(egui::Event::Copy);
    harness.run_steps(1);
    assert_eq!(copied(&harness).as_deref(), Some(OBJECTS_MARKER), "Cmd+C copies the selected rectangle");
    harness.event(egui::Event::Paste(OBJECTS_MARKER.to_string()));
    harness.run_steps(2);
    assert_eq!(count(&harness), before + 1, "Cmd+V pastes it");
    harness.event(egui::Event::Cut);
    harness.run_steps(2);
    assert_eq!(count(&harness), before, "Cmd+X removes the selected copy");
}
