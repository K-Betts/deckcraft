//! The Equation dialog: type or click, see it drawn, insert it.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use serde_json::json;

use crate::{Services, SlideApp};
use deckcraft_engine::Session;
use deckcraft_model::text::RunKind;

fn harness() -> egui_kittest::Harness<'static, SlideApp> {
    let mut s = Session::with_new();
    s.execute("slide.new", &json!({})).unwrap();
    s.execute("slide.go", &json!({"index": 0})).unwrap();
    let mut app = SlideApp::new(s, Services::default());
    app.ui.presenter_view = false;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1000.0, 700.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(2);
    h.state_mut().run("app.dialog", json!({"id": "equation"})).unwrap();
    h.run_steps(4);
    h
}

fn typed(h: &egui_kittest::Harness<'static, SlideApp>) -> String {
    h.state().dialog.as_ref().and_then(|d| d.fields.get("eq").cloned()).unwrap_or_default()
}

fn type_text(h: &mut egui_kittest::Harness<'static, SlideApp>, s: &str) {
    h.event(Event::Text(s.into()));
    h.run_steps(3);
}

fn tab(h: &mut egui_kittest::Harness<'static, SlideApp>) {
    h.event(Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.event(Event::Key { key: Key::Tab, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}

fn equation_shape(h: &egui_kittest::Harness<'static, SlideApp>) -> Option<deckcraft_model::Shape> {
    let d = h.state().session.active()?;
    d.doc.slides[0].shapes.iter().find(|s| s.name == "Equation").cloned()
}

#[test]
fn the_field_is_ready_to_type_in_and_the_picture_follows() {
    let mut h = harness();
    type_text(&mut h, "x^2/3");
    assert_eq!(typed(&h), "x^2/3");
    h.render().unwrap().save("/tmp/equation_typed.png").unwrap();
}

#[test]
fn a_button_types_boxes_and_tab_moves_between_them() {
    let mut h = harness();
    h.get_by_label("a⁄b").click();
    h.run_steps(3);
    assert_eq!(typed(&h), "(□)/(□)");
    type_text(&mut h, "1");
    tab(&mut h);
    type_text(&mut h, "2");
    assert_eq!(typed(&h), "(1)/(2)");
    h.render().unwrap().save("/tmp/equation_boxes.png").unwrap();
}

#[test]
fn a_power_goes_on_what_was_just_typed() {
    let mut h = harness();
    type_text(&mut h, "x");
    h.get_by_label("x²").click();
    h.run_steps(3);
    assert_eq!(typed(&h), "x^2");
    h.get_by_label("xⁿ").click();
    h.run_steps(3);
    assert_eq!(typed(&h), "x^2^(□)");
}

#[test]
fn insert_waits_for_the_boxes_then_puts_a_real_equation_on_the_slide() {
    let mut h = harness();
    h.get_by_label("a⁄b").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_some());
    assert!(h.get_all_by_label("Insert").last().unwrap().accesskit_node().is_disabled(), "boxes left empty: Insert is off");
    assert!(equation_shape(&h).is_none());
    type_text(&mut h, "1");
    tab(&mut h);
    type_text(&mut h, "2");
    assert_eq!(typed(&h), "(1)/(2)");
    h.get_all_by_label("Insert").last().unwrap().click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    let shape = equation_shape(&h).expect("the equation is on the slide");
    let run = &shape.text.as_ref().unwrap().paragraphs[0].runs[0];
    assert!(matches!(&run.kind, RunKind::Math { omml } if omml.contains("<m:f>")));
    assert_eq!(run.text, "1/2");
}

#[test]
fn an_equation_on_the_slide_reopens_in_the_dialog_and_updates_in_place() {
    let mut h = harness();
    type_text(&mut h, "x^2/3");
    h.get_all_by_label("Insert").last().unwrap().click();
    h.run_steps(3);
    let before = equation_shape(&h).expect("inserted");
    // It is selected: the dialog opens on what was typed.
    h.state_mut().run("app.dialog", json!({"id": "equation"})).unwrap();
    h.run_steps(4);
    assert_eq!(typed(&h), "(x^2)/(3)");
    type_text(&mut h, "+1");
    h.get_all_by_label("Update").last().unwrap().click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    let d = h.state().session.active().unwrap();
    let all: Vec<_> = d.doc.slides[0].shapes.iter().filter(|s| s.name == "Equation").collect();
    assert_eq!(all.len(), 1, "changed in place, not added again");
    assert_eq!(all[0].id, before.id);
    let run = &all[0].text.as_ref().unwrap().paragraphs[0].runs[0];
    assert!(matches!(&run.kind, RunKind::Math { omml } if omml.contains("<m:f>") && omml.contains('1')));
}
