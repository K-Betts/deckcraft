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
    // The question text follows its number badge on the slide. A fixed question is found in the
    // bank; one made from a template has new numbers, so it is checked against the templates.
    let squash = |t: &str| t.split_whitespace().collect::<String>();
    let check = |n: usize, topic: &str, level: u8| {
        let st = h.state().session.active().unwrap();
        let slide = st.doc.slides.get(st.selection.slide).unwrap();
        let shape = slide.shapes.iter().find(|s| s.name == format!("Question {n} text")).unwrap();
        let t = &shape.text.as_ref().unwrap().paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join(" ");
        match bank.questions.iter().find(|q| squash(&deckcraft_engine::latex::plain(&q.text)) == squash(t)) {
            Some(q) => assert_eq!((q.topic.as_str(), q.difficulty), (topic, level), "question {n}: {t}"),
            None => assert!(bank.templates.iter().any(|x| x.topic == topic && x.difficulty == level), "question {n} is from no template: {t}"),
        }
    };
    check(2, "Algebra", 3);
    check(4, "Number", 1);
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
    h.state_mut().ui.removed_questions = starter
        .questions
        .iter()
        .map(|q| (&q.topic, &q.id))
        .chain(starter.templates.iter().map(|t| (&t.topic, &t.id)))
        .filter(|(topic, _)| *topic == "Ratio")
        .map(|(_, id)| id.clone())
        .collect();
    h.state_mut().ui.my_questions = vec![Question {
        id: "my-001".into(),
        topic: "Ratio".into(),
        subtopic: String::new(),
        difficulty: 2,
        keywords: vec![],
        qtype: String::new(),
        source: String::new(),
        year: String::new(),
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
    // Every built-in question is a template; the list shows each once.
    let first = deckcraft_engine::questions::starter().unwrap().templates[0].id.clone();
    h.query_all_by_label("Remove").next().unwrap().click();
    h.run_steps(3);
    assert_eq!(h.state().ui.removed_questions, vec![first]);
    h.get_by_label("Restore them").click();
    h.run_steps(3);
    assert!(h.state().ui.removed_questions.is_empty());
}

const TWO_STEP: &str = "Y7 Autumn B3 Expressions and equations · S7 Solve 2-step equations";

#[test]
fn a_teacher_searches_the_skills_chooses_one_and_the_question_is_about_it() {
    let mut h = harness();
    h.state_mut().ui.get_ready_slots = vec![(String::new(), 3), (String::new(), 0), (String::new(), 0), (String::new(), 0)];
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    // Question 1: open the picker, search, and see the skills that match.
    h.query_all_by_label_contains("Any skill").next().unwrap().click();
    h.run_steps(3);
    assert!(h.query_by_label_contains("Choose the skill for question 1").is_some());
    h.state_mut().dialog.as_mut().unwrap().fields.insert("q".into(), "2-step equations".into());
    h.run_steps(3);
    assert!(h.query_by_label("Solve 2-step equations").is_some(), "the small step is listed by name");
    assert!(h.query_by_label_contains("Year 7").is_some(), "with its year group and block");
    h.query_all_by_label("Choose").next().unwrap().click();
    h.run_steps(3);
    // Back on the Get Ready dialog with the skill chosen for question 1 only.
    assert_eq!(h.state().ui.get_ready_skill[0], TWO_STEP);
    assert!(h.state().ui.get_ready_skill[1].is_empty());
    assert!(h.query_by_label_contains("Solve 2-step equations").is_some());
    h.get_by_label("Insert").click();
    h.run_steps(3);
    // The stretch question for that step is a fraction-in-an-equation: it starts with "Solve".
    let st = h.state().session.active().unwrap();
    let slide = st.doc.slides.get(st.selection.slide).unwrap();
    let q1 = slide.shapes.iter().find(|s| s.name == "Question 1 text").unwrap();
    let text: String = q1.text.as_ref().unwrap().paragraphs.iter().map(|p| p.text()).collect();
    assert!(text.starts_with("Solve"), "{text}");
}

#[test]
fn the_picker_narrows_by_year_and_can_go_back_to_any_skill() {
    let mut h = harness();
    h.state_mut().ui.get_ready_skill = vec![TWO_STEP.into(), String::new(), String::new(), String::new()];
    h.state_mut().run("app.dialog", json!({"id": "chooseSkill", "slot": 1})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label_contains("Choose the skill for question 2").is_some());
    h.state_mut().dialog.as_mut().unwrap().fields.insert("year".into(), "General".into());
    h.run_steps(3);
    assert!(h.query_by_label("Solve 2-step equations").is_none(), "year 7 steps are hidden under General");
    h.get_by_label("Any skill (no choice)").click();
    h.run_steps(3);
    assert!(h.state().ui.get_ready_skill[1].is_empty());
    assert_eq!(h.state().ui.get_ready_skill[0], TWO_STEP, "the other questions keep their skill");
    assert_eq!(h.state().dialog.as_ref().map(|d| d.id.as_str()), Some("getReady"), "back on the Get Ready dialog");
}

#[test]
fn importing_a_question_file_adds_templates_and_skips_repeats() {
    let file = r#"{"version":1,"templates":[{"id":"my-t-001","topic":"Number","subtopic":"Halving","difficulty":1,
        "vars":[["n",{"int":[2,30]}]],"calc":[["d","n * 2"]],"text":"Halve [[d]].","answer":"[[n]]"}],
        "questions":[{"id":"alg-001","topic":"Algebra","difficulty":1,"text":"Already here","answer":"1"}]}"#;
    let services = Services {
        pick_open: Some(Box::new(|_: &str| Some("questions.json".to_string()))),
        read: Some(Box::new(move |_: &str| Ok(file.as_bytes().to_vec()))),
        ..Default::default()
    };
    let mut app = SlideApp::new(Session::with_new(), services);
    app.ui.tab = "Insert".into();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    h.state_mut().run("app.dialog", json!({"id": "questionBank"})).unwrap();
    h.run_steps(3);
    h.get_by_label_contains("Import questions").click();
    h.run_steps(3);
    assert_eq!(h.state().ui.my_templates.len(), 1);
    assert_eq!(h.state().ui.my_questions.len(), 1);
    assert!(h.state().ui.my_templates[0].id.starts_with("my-t-"), "it has an id of its own, so it cannot clash with a built-in one");
    assert!(h.state().ui.my_questions[0].id.starts_with("my-"));
    // The same file again adds nothing new.
    h.get_by_label_contains("Import questions").click();
    h.run_steps(3);
    assert_eq!(h.state().ui.my_templates.len(), 1);
    assert_eq!(h.state().ui.my_questions.len(), 1);
    assert!(h.query_by_label_contains("already in your bank").is_some());
}

fn paste_into_the_dialog(text: &str) -> egui_kittest::Harness<'static, SlideApp> {
    let mut h = harness();
    h.state_mut().run("app.dialog", json!({"id": "questionBank"})).unwrap();
    h.run_steps(3);
    h.get_by_label_contains("Add questions written by an AI").click();
    h.run_steps(3);
    h.state_mut().dialog.as_mut().unwrap().fields.insert("paste".into(), text.to_string());
    h.run_steps(2);
    h
}

#[test]
fn pasting_an_ais_answer_shows_examples_and_adds_it_once_checked() {
    let good = "Sure!\n```json\n{\"version\":1,\"templates\":[{\"id\":\"num-t-001\",\"topic\":\"Number\",\"subtopic\":\"Doubling\",\"difficulty\":1,\"keywords\":[\"double\"],\"vars\":[[\"a\",{\"int\":[2,40]}]],\"check\":[\"a * 2 == a + a\"],\"text\":\"Double [[a]].\",\"answer\":\"[[a * 2]]\"}]}\n```";
    let mut h = paste_into_the_dialog(good);
    assert!(h.query_by_label_contains("Press Check first").is_some());
    h.get_by_label("Add these to my bank").click();
    h.run_steps(2);
    assert!(h.state().ui.my_templates.is_empty(), "nothing is added before it has been checked");
    h.get_by_label("Check").click();
    h.run_steps(3);
    assert!(h.query_by_label_contains("Found 1 recipe").is_some(), "the check says what it found");
    assert!(h.query_all_by_label_contains("Double").count() >= 3, "and shows three example questions");
    h.get_by_label("Add these to my bank").click();
    h.run_steps(3);
    assert_eq!(h.state().ui.my_templates.len(), 1);
    assert!(h.query_by_label_contains("Added 1").is_some());
}

#[test]
fn a_mistake_in_pasted_text_is_explained_and_nothing_is_added() {
    // The slip a local AI made: a } where the ] belongs.
    let broken = "{\"version\":1,\"templates\":[\n{\"id\":\"t\",\"vars\":[[\"v\",{\"pick\":[\"x\"]}}],\"text\":\"a\",\"answer\":\"b\"}]}";
    let mut h = paste_into_the_dialog(broken);
    h.get_by_label("Check").click();
    h.run_steps(3);
    assert!(h.query_by_label_contains("not valid JSON").is_some());
    assert!(h.query_by_label_contains("Common causes").is_some());
    assert!(h.state().ui.my_templates.is_empty());
}

/// A small curriculum in our own words, with this school year's dates far in the past and
/// future so “this week” does not depend on when the test runs.
const CURRICULUM: &str = r#"{"version": 1, "name": "Test school", "courses": [
  {"year": "Y7", "route": "Core", "units": [
    {"title": "Angles", "start": "2000-01-03", "weeks": 3, "objectives": [
      {"text": "Angles on a straight line"}, {"text": "Angles in a triangle"}]},
    {"title": "Equations", "start": "2000-02-07", "weeks": 4, "objectives": [
      {"text": "Solve one-step equations"}, {"text": "Solve two-step equations"},
      {"text": "Solve equations with the unknown on both sides"}],
      "prior": [{"text": "Substitute into an expression"}]},
    {"title": "Ratio", "start": "2999-01-01", "weeks": 2, "objectives": [{"text": "Share an amount in a given ratio"}]}]},
  {"year": "Y7", "route": "Support", "units": [
    {"title": "Place value", "start": "2000-01-03", "weeks": 3, "objectives": [{"text": "Round to the nearest 10"}]}]}]}"#;

fn taught_harness() -> egui_kittest::Harness<'static, SlideApp> {
    let mut h = harness();
    h.state_mut().ui.curriculum = CURRICULUM.into();
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    h
}

fn field(h: &egui_kittest::Harness<'static, SlideApp>, k: &str) -> String {
    h.state().dialog.as_ref().unwrap().fields.get(k).cloned().unwrap_or_default()
}

#[test]
fn with_no_curriculum_the_dialog_offers_to_load_one_and_still_works_by_hand() {
    let mut h = harness();
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label("Question 1").is_some(), "no curriculum: choosing by hand, as before");
    h.get_by_label("From what I just taught").click();
    h.run_steps(3);
    assert!(h.query_by_label("Load your department's curriculum").is_some());
    assert_eq!(h.state().ui.get_ready_mode, "taught", "the choice is remembered");
    // A broken paste is explained and nothing is kept.
    h.query_by_label("Or paste a curriculum file's text").unwrap().click();
    h.run_steps(3);
    h.state_mut().dialog.as_mut().unwrap().fields.insert("paste".into(), "{\"version\": 1}".into());
    h.run_steps(2);
    h.get_by_label("Use this curriculum").click();
    h.run_steps(3);
    assert!(field(&h, "load").starts_with("Not loaded"), "{}", field(&h, "load"));
    assert!(h.state().ui.curriculum.is_empty());
    h.state_mut().dialog.as_mut().unwrap().fields.insert("paste".into(), CURRICULUM.into());
    h.run_steps(2);
    h.get_by_label("Use this curriculum").click();
    h.run_steps(3);
    assert_eq!(h.state().ui.curriculum, CURRICULUM);
    assert!(h.query_by_label("Which lesson have you just taught?").is_some());
}

#[test]
fn a_teacher_says_what_they_taught_and_gets_four_planned_questions() {
    let mut h = taught_harness();
    assert!(h.query_by_label("Which lesson have you just taught?").is_some(), "a loaded curriculum opens on this mode");
    assert_eq!(field(&h, "unit"), "1", "the unit under way (Equations) is chosen; Ratio has not started");
    // Equations ended long ago, so the guess is its last lesson.
    assert_eq!(field(&h, "objective"), "2");
    h.get_by_label("Solve two-step equations").click();
    h.run_steps(2);
    h.get_by_label("Make the questions").click();
    h.run_steps(3);
    assert_eq!(field(&h, "obj0"), "Solve two-step equations");
    assert_eq!(field(&h, "h0"), "Last lesson");
    assert!(field(&h, "from1").ends_with("Equations"), "last week: earlier in the unit");
    assert!(field(&h, "from2").ends_with("Angles"), "last topic: the unit before");
    for i in 0..4 {
        assert!(!field(&h, &format!("text{i}")).is_empty(), "card {i} has a question");
    }
    assert!(h.query_all_by_label_contains("Practises:").count() == 4, "each card says what it practises");
    // Edit card 1, rename card 3's heading, and insert.
    let d = h.state_mut().dialog.as_mut().unwrap();
    d.fields.insert("text0".into(), "My own two-step question: solve $2x + 3 = 11$".into());
    d.fields.insert("answer0".into(), "$x = 4$".into());
    d.fields.insert("h2".into(), "Angles".into());
    h.run_steps(2);
    let before = slides(&h);
    h.get_by_label("Insert").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none(), "the dialog closes");
    assert_eq!(slides(&h), before + 5);
    let texts = slide_texts(&h).join(" | ");
    assert!(texts.contains("My own two-step question"), "{texts}");
    assert!(texts.contains("Angles") && texts.contains("Last week") && !texts.contains("Last topic"), "{texts}");
    assert_eq!(h.state().ui.get_ready_year, "Y7");
    assert_eq!(h.state().ui.get_ready_route, "Core");
}

#[test]
fn another_question_and_a_new_skill_change_one_card_only() {
    let mut h = taught_harness();
    h.get_by_label("Make the questions").click();
    h.run_steps(3);
    let (q1, q2) = (field(&h, "q1"), field(&h, "q2"));
    // Another question for card 2 changes card 2 (a recipe can give the same text only by chance,
    // so the stored question, with its id and numbers, is compared).
    let mut changed = false;
    for _ in 0..5 {
        h.query_all_by_label("Another question").nth(1).unwrap().click();
        h.run_steps(2);
        changed |= field(&h, "q1") != q1;
    }
    assert!(changed, "card 2 got a new question");
    assert_eq!(field(&h, "q2"), q2, "card 3 is untouched");
    // Change card 2's skill in the picker; the other cards survive the trip.
    let (q0, q2) = (field(&h, "q0"), field(&h, "q2"));
    h.query_all_by_label("Change skill…").nth(1).unwrap().click();
    h.run_steps(3);
    assert!(h.query_by_label_contains("Choose the skill for question 2").is_some());
    h.state_mut().dialog.as_mut().unwrap().fields.insert("q".into(), "2-step equations".into());
    h.run_steps(3);
    h.query_all_by_label("Choose").next().unwrap().click();
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().map(|d| d.id.as_str()), Some("getReady"));
    assert_eq!((field(&h, "q0"), field(&h, "q2")), (q0, q2), "cards 1 and 3 kept their questions");
    assert!(field(&h, "skills1").contains(TWO_STEP), "{}", field(&h, "skills1"));
    let q1: deckcraft_engine::questions::Question = serde_json::from_str(&field(&h, "q1")).unwrap();
    assert_eq!(q1.skill_key(), TWO_STEP);
    assert!(h.query_by_label_contains("Practises: Solve 2-step equations").is_some());
}

#[test]
fn support_classes_start_at_the_support_level_and_a_new_choice_asks_for_new_questions() {
    let mut h = taught_harness();
    // The route row comes before the level row, which has a Support button too.
    h.query_all_by_label("Support").next().unwrap().click();
    h.run_steps(3);
    assert_eq!(field(&h, "route"), "Support");
    assert_eq!(field(&h, "level"), "1", "a Support class starts at the Support level");
    h.get_by_label("Make the questions").click();
    h.run_steps(3);
    assert_eq!(field(&h, "lvl0"), "1", "Support classes get Support-level questions");
    // The choices fold into one line once the questions are made; Change… opens them again.
    assert!(h.query_by_label("Which lesson have you just taught?").is_none());
    assert!(h.query_by_label_contains("Year 7 Support · Place value · just taught: Round to the nearest 10").is_some());
    h.get_by_label("Change…").click();
    h.run_steps(3);
    assert!(h.query_by_label("Which lesson have you just taught?").is_some());
    // Back to Core (as the route button does: “Core” is also a level, so the button is ambiguous here).
    h.state_mut().dialog.as_mut().unwrap().fields.insert("route".into(), "Core".into());
    h.state_mut().dialog.as_mut().unwrap().fields.remove("level");
    h.run_steps(3);
    assert!(h.query_by_label("Your choices changed: make the questions again.").is_some());
}

/// Pictures of the dialog to look at (not a check): run with
/// `GET_READY_SHOTS=dir [GET_READY_CURRICULUM=file] cargo test -p deckcraft-ui-egui -- --ignored look_at`.
#[test]
#[ignore]
fn look_at_the_dialog_planned_from_a_curriculum() {
    let dir = std::env::var("GET_READY_SHOTS").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
    let curriculum = std::env::var("GET_READY_CURRICULUM").ok().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_else(|| CURRICULUM.into());
    let mut h = harness();
    h.state_mut().ui.curriculum = curriculum;
    h.set_size(egui::vec2(1400.0, 1000.0));
    h.state_mut().run("app.dialog", json!({"id": "getReady"})).unwrap();
    h.run_steps(4);
    h.render().unwrap().save(format!("{dir}/get-ready-1-choose.png")).unwrap();
    // A year group with a year before it, at its second lesson.
    if let Ok(year) = std::env::var("GET_READY_YEAR") {
        h.state_mut().dialog.as_mut().unwrap().fields.insert("year".into(), year);
        h.run_steps(2);
    }
    h.state_mut().dialog.as_mut().unwrap().fields.insert("objective".into(), "1".into());
    h.run_steps(2);
    h.get_by_label("Make the questions").click();
    h.run_steps(4);
    h.render().unwrap().save(format!("{dir}/get-ready-2-cards.png")).unwrap();
    h.get_by_label("Insert").click();
    h.run_steps(4);
    h.render().unwrap().save(format!("{dir}/get-ready-3-slide.png")).unwrap();
}
