//! Light and dark: follow the computer by default, until the teacher chooses one.

use deckcraft_engine::Session;
use egui_kittest::Harness;
use serde_json::json;

use crate::theme::Brightness;
use crate::{Services, SlideApp};

fn harness() -> Harness<'static, SlideApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SlideApp::new(Session::with_new(), Services::default()),
    );
    h.run_steps(2);
    h
}

fn set_system(h: &mut Harness<'_, SlideApp>, theme: egui::Theme) {
    h.input_mut().system_theme = Some(theme);
    h.run_steps(2);
}

fn is_dark(h: &Harness<'_, SlideApp>) -> bool {
    h.ctx.global_style().visuals.dark_mode
}

#[test]
fn it_follows_the_computer_by_default() {
    let mut h = harness();
    set_system(&mut h, egui::Theme::Dark);
    assert_eq!(h.state().ui.brightness, Brightness::Dark);
    assert!(is_dark(&h), "the whole style, not just a flag, went dark");
    set_system(&mut h, egui::Theme::Light);
    assert_eq!(h.state().ui.brightness, Brightness::Light);
    assert!(!is_dark(&h));
}

#[test]
fn choosing_light_or_dark_stops_following_and_system_resumes() {
    let mut h = harness();
    h.state_mut().run("view.appearance", json!({"mode": "light"})).unwrap();
    set_system(&mut h, egui::Theme::Dark);
    assert_eq!(h.state().ui.brightness, Brightness::Light, "a chosen appearance wins over the computer");
    h.state_mut().run("view.dark", json!({"on": true})).unwrap();
    set_system(&mut h, egui::Theme::Light);
    assert_eq!(h.state().ui.brightness, Brightness::Dark, "the older Dark Mode command also pins it");
    h.state_mut().run("view.appearance", json!({"mode": "system"})).unwrap();
    h.run_steps(2);
    assert_eq!(h.state().ui.brightness, Brightness::Light, "going back to Match this computer takes the computer's setting");
}

#[test]
fn a_bad_mode_is_an_error_not_a_change() {
    let mut h = harness();
    assert!(h.state_mut().run("view.appearance", json!({"mode": "purple"})).is_err());
    assert!(h.state().ui.follow_system);
}
