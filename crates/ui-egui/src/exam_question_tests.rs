//! The Exam Question dialog: only with Maths on; Insert, Blank, Reroll and the skill picker.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::{Services, SlideApp};
use deckcraft_engine::Session;

const TWO_STEP: &str = "Y7 Autumn B3 Expressions and equations · S7 Solve 2-step equations";

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

fn current(h: &egui_kittest::Harness<'static, SlideApp>) -> (usize, Vec<(String, String)>) {
    let st = h.state().session.active().unwrap();
    let slide = st.doc.slides.get(st.selection.slide).unwrap();
    let texts = slide
        .shapes
        .iter()
        .map(|s| (s.name.clone(), s.text.as_ref().map(|t| t.paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join(" ")).unwrap_or_default()))
        .collect();
    (st.doc.slides.len(), texts)
}

#[test]
fn the_insert_tab_has_no_exam_buttons_for_now_but_get_ready_stays() {
    let mut h = harness();
    h.state_mut().run("app.pack", json!({"id": "maths", "on": true})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label_contains("Exam").is_none(), "Exam Question is off the ribbon until it is ready");
    assert!(h.query_by_label_contains("Question\nLibrary").is_none() && h.query_by_label_contains("Library").is_none());
    assert!(h.query_by_label_contains("Get Ready").is_some(), "the Get Ready starter stays");
}

#[test]
fn the_dialog_inserts_a_bank_question_then_a_blank_one_and_rerolls_in_place() {
    let mut h = harness();
    let (before, _) = current(&h);
    h.state_mut().run("app.dialog", json!({"id": "examQuestion"})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label("Reroll this slide").is_none(), "nothing to reroll yet");
    h.get_by_label("Insert").click();
    h.run_steps(3);
    let (n, texts) = current(&h);
    assert_eq!(n, before + 1);
    assert!(h.state().dialog.is_none(), "the dialog closes");
    let total = texts.iter().find(|(name, _)| name == "Total marks").map(|t| t.1.clone()).unwrap();
    assert!(total.starts_with("(Total for Question 1 is ") && total.ends_with("marks)"), "{total}");

    // Blank, three parts, question 5: reroll replaces the slide just made.
    h.state_mut().run("app.dialog", json!({"id": "examQuestion"})).unwrap();
    h.run_steps(3);
    h.get_by_label("Blank, to type into").click();
    h.run_steps(2);
    // The parts row has just appeared: give it a frame before clicking it.
    h.run_steps(2);
    h.get_by_label("(a) to (c)").click();
    h.run_steps(2);
    h.state_mut().dialog.as_mut().unwrap().fields.insert("number".into(), "5".into());
    h.run_steps(2);
    h.get_by_label("Reroll this slide").click();
    h.run_steps(3);
    let (n, texts) = current(&h);
    assert_eq!(n, before + 1, "reroll replaces, it does not add");
    assert!(texts.iter().any(|(name, t)| name == "Part (c) text" && t == "Type part (c) here."), "{texts:?}");
    assert!(texts.iter().any(|(name, t)| name == "Question number" && t == "5"));
}

#[test]
fn the_skill_picker_comes_back_to_the_exam_dialog_with_the_skill_and_the_choices() {
    let mut h = harness();
    h.state_mut().run("app.dialog", json!({"id": "examQuestion"})).unwrap();
    h.run_steps(3);
    h.state_mut().dialog.as_mut().unwrap().fields.insert("number".into(), "3".into());
    h.run_steps(2);
    h.query_all_by_label_contains("Any skill").next().unwrap().click();
    h.run_steps(3);
    assert!(h.query_by_label_contains("Choose the skill for the exam question").is_some());
    h.state_mut().dialog.as_mut().unwrap().fields.insert("q".into(), "2-step equations".into());
    h.run_steps(3);
    h.query_all_by_label("Choose").next().unwrap().click();
    h.run_steps(3);
    let d = h.state().dialog.as_ref().unwrap();
    assert_eq!(d.id, "examQuestion", "back on the Exam Question dialog");
    let field = |k: &str| d.fields.get(k).cloned().unwrap_or_default();
    assert_eq!((field("skill"), field("number")), (TWO_STEP.to_string(), "3".to_string()), "with the skill and what was typed");
    assert!(h.state().ui.get_ready_skill.iter().all(String::is_empty), "Get Ready's choices are untouched");
    h.get_by_label("Insert").click();
    h.run_steps(3);
    let (_, texts) = current(&h);
    let q: String = texts.iter().filter(|(n, _)| n == "Question" || n.ends_with("text")).map(|t| t.1.clone()).collect::<Vec<_>>().join(" ");
    assert!(q.contains("Solve") || q.contains('x'), "a 2-step equations question: {q}");
}
