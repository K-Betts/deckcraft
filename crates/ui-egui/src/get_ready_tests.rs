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
    assert_eq!(slides(&h), before + 5, "the starter and a full-screen slide for each question");
    assert!(h.state().dialog.is_none(), "the dialog closes");
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    h.get_by_label("Reroll this slide").click();
    h.run_steps(3);
    assert_eq!(slides(&h), before + 5, "reroll replaces, it does not add");
}

fn slide_texts(h: &egui_kittest::Harness<'static, SlideApp>) -> Vec<String> {
    let st = h.state().session.active().unwrap();
    let slide = st.doc.slides.get(st.selection.slide).unwrap();
    slide.shapes.iter().filter_map(|s| s.text.as_ref().map(|t| t.paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join(" "))).collect()
}

#[test]
fn the_dialog_has_a_topic_and_level_for_every_question_and_uses_them() {
    let mut h = harness();
    h.state_mut().ui.get_ready_slots = vec![(String::new(), 0), ("Algebra".into(), 3), (String::new(), 0), ("Number".into(), 1)];
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    for n in 1..=4 {
        assert!(h.query_by_label(&format!("Question {n}")).is_some(), "a row for question {n}");
    }
    assert!(h.query_by_label("Question 5").is_none());
    h.get_by_label("Insert").click();
    h.run_steps(3);
    let bank = deckcraft_engine::questions::starter().unwrap();
    let texts = slide_texts(&h);
    let find = |n: usize| {
        // The question text follows its number badge on the slide.
        let squash = |t: &str| t.split_whitespace().collect::<String>();
        let is = |q: &deckcraft_engine::questions::Question, t: &str| squash(&deckcraft_engine::latex::plain(&q.text)) == squash(t);
        let at = texts.iter().position(|t| *t == n.to_string()).unwrap();
        let t = texts.iter().skip(at + 1).find(|t| bank.questions.iter().any(|q| is(q, t))).unwrap();
        bank.questions.iter().find(|q| is(q, t)).unwrap().clone()
    };
    assert_eq!((find(2).topic.as_str(), find(2).difficulty), ("Algebra", 3));
    assert_eq!((find(4).topic.as_str(), find(4).difficulty), ("Number", 1));
}

#[test]
fn choosing_a_level_for_one_question_changes_only_that_question_and_is_remembered() {
    let mut h = harness();
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    // The first "Stretch" is the Set-all button; the next four are questions 1 to 4.
    h.query_all_by_label("Stretch").nth(2).unwrap().click();
    h.run_steps(3);
    h.get_by_label("Insert").click();
    h.run_steps(3);
    let slots = h.state().ui.get_ready_slots.clone();
    assert_eq!(slots.iter().map(|s| s.1).collect::<Vec<_>>(), [0, 3, 0, 0], "only question 2 is Stretch");
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    h.get_by_label("Reroll this slide").click();
    h.run_steps(3);
    assert_eq!(h.state().ui.get_ready_slots.iter().map(|s| s.1).collect::<Vec<_>>(), [0, 3, 0, 0], "reopening starts from the last choices");
}

#[test]
fn set_all_works_and_a_slide_always_has_four_questions() {
    let mut h = harness();
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label("Question 5").is_none(), "four questions, no more");
    h.query_all_by_label("Core").next().unwrap().click();
    h.run_steps(3);
    h.get_by_label("Insert").click();
    h.run_steps(3);
    let slots = h.state().ui.get_ready_slots.clone();
    assert_eq!(slots.len(), 4);
    assert!(slots.iter().all(|s| s.1 == 2), "{slots:?}");
    assert_eq!((1..=4).filter(|n| slide_texts(&h).contains(&n.to_string())).count(), 4);
}

#[test]
fn removing_and_adding_questions_changes_what_the_starter_can_pick() {
    use deckcraft_engine::questions::Question;
    let mut h = harness();
    // Take every built-in Ratio question out and add one of our own: Ratio now has just that one.
    let starter = deckcraft_engine::questions::starter().unwrap();
    h.state_mut().ui.removed_questions = starter.questions.iter().filter(|q| q.topic == "Ratio").map(|q| q.id.clone()).collect();
    h.state_mut().ui.my_questions = vec![Question {
        id: "my-001".into(),
        topic: "Ratio".into(),
        subtopic: String::new(),
        difficulty: 2,
        text: "Share 20 in the ratio $2 : 3$".into(),
        answer: "8 and 12".into(),
        needs_diagram: false,
        diagram: None,
    }];
    h.state_mut().ui.get_ready_slots = vec![("Ratio".into(), 0), (String::new(), 0), (String::new(), 0), (String::new(), 0)];
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    h.get_by_label("Insert").click();
    h.run_steps(3);
    assert!(slide_texts(&h).iter().any(|t| t == "Share 20 in the ratio 2:3"), "{:?}", slide_texts(&h));
}

#[test]
fn the_manage_dialog_adds_a_question_and_removes_a_built_in_one() {
    let mut h = harness();
    h.state_mut().run("app.dialog", json!({"id": "questionBank"})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label_contains("questions in your bank").is_some());
    let first = deckcraft_engine::questions::starter().unwrap().questions[0].id.clone();
    h.query_all_by_label("Remove").next().unwrap().click();
    h.run_steps(3);
    assert_eq!(h.state().ui.removed_questions, vec![first]);
    h.get_by_label("Restore them").click();
    h.run_steps(3);
    assert!(h.state().ui.removed_questions.is_empty());
}
