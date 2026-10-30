//! Insert › Question Library: search a skill, narrow by tier, insert a question. The library is
//! made up in code; none of it is from a real paper.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use egui_kittest::kittest::{NodeT, Queryable};
use serde_json::{Value, json};

use crate::{Services, SlideApp};
use deckcraft_engine::Session;

fn question(record: &str, n: u32, tier: &str, topic: &str, marks: u32, calc: &str) -> Value {
    json!({"id": format!("exam-{record}-Q{n}"), "kind": "question", "tier": tier, "status": "checked", "text": "Made-up question.",
           "parts": [{"label": "a", "text": "p", "answer": "x = 2", "marks": marks}], "answer": "", "source": {"record": record, "question": n},
           "exam": {"exam_board": "Test board", "qual_code": "T1", "calculator": calc, "total_marks": marks, "series": "Summer", "year_sat": 2021,
                    "paper": &record[record.len() - 2..], "strand": "Algebra", "topic": topic, "source_type": "past_paper"}})
}

/// A folder with four made-up questions in two papers and a picture for each.
fn library() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!("deckcraft-exam-ui-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("items/exam")).unwrap();
    std::fs::create_dir_all(dir.join("derived/exam_crops")).unwrap();
    let files = [
        (
            "TEST-1F",
            vec![
                question("TEST-1F", 1, "Foundation", "Simultaneous equations", 3, "calculator"),
                question("TEST-1F", 2, "Foundation", "Indices", 2, "non-calculator"),
            ],
        ),
        (
            "TEST-1H",
            vec![
                question("TEST-1H", 1, "Higher", "Simultaneous equations", 5, "non-calculator"),
                question("TEST-1H", 2, "Higher", "Surds", 4, "calculator"),
            ],
        ),
    ];
    let pic = deckcraft_render::Image { width: 120, height: 48, pixels: vec![235; 120 * 48 * 4] }.to_png();
    for (record, items) in files {
        std::fs::write(dir.join(format!("items/exam/{record}.json")), json!({"schema": "maths-workspace.items/2", "items": items}).to_string())
            .unwrap();
        for n in [1, 2] {
            std::fs::write(dir.join(format!("derived/exam_crops/{record}-Q{n}.png")), &pic).unwrap();
        }
    }
    dir
}

fn harness(dir: Option<&std::path::Path>) -> egui_kittest::Harness<'static, SlideApp> {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    app.ui.exam_library_dir = dir.map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
    app.dialog = Some(crate::dialogs::Dialog::new("examLibrary"));
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.ctx.set_missing_glyph_policy(egui::MissingGlyphPolicy::Tofu);
    h.run_steps(4);
    h
}

fn slides(h: &egui_kittest::Harness<'static, SlideApp>) -> usize {
    h.state().session.active().unwrap().doc.slides.len()
}

#[test]
fn without_a_folder_the_dialog_says_what_to_choose() {
    let h = harness(None);
    assert!(h.query_by_label_contains("Choose the folder").is_some());
}

#[test]
fn searching_a_skill_lists_its_questions_and_the_tier_chip_narrows_them() {
    let dir = library();
    let mut h = harness(Some(&dir));
    assert!(h.query_by_label("4 questions").is_some());
    h.state_mut().exam_lib.query.text = "simultaneous equations".into();
    h.run_steps(3);
    assert!(h.query_by_label("2 questions").is_some());
    h.get_by_label("Higher").click();
    h.run_steps(3);
    assert!(h.query_by_label("1 question").is_some());
    assert!(h.query_by_label("Insert exam-TEST-1H-Q1").is_some());
    assert!(h.query_by_label("Insert exam-TEST-1F-Q1").is_none());
    // Clear brings everything back.
    h.get_by_label("Clear").click();
    h.run_steps(3);
    assert!(h.query_by_label("4 questions").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn insert_puts_the_question_on_a_new_slide() {
    let dir = library();
    let mut h = harness(Some(&dir));
    let before = slides(&h);
    h.get_by_label("Insert exam-TEST-1H-Q1").click();
    h.run_steps(3);
    assert_eq!(slides(&h), before + 1);
    assert!(h.query_by_label_contains("Inserted").is_some());
    assert!(h.state().dialog.is_some(), "the dialog stays open for the next question");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn questions_with_no_picture_are_listed_only_when_asked_for() {
    let dir = library();
    std::fs::remove_file(dir.join("derived/exam_crops/TEST-1F-Q2.png")).unwrap();
    let mut h = harness(Some(&dir));
    assert!(h.query_by_label("3 questions").is_some(), "the one without a picture is left out");
    h.get_by_label("Include questions with no picture yet").click();
    h.run_steps(3);
    assert!(h.query_by_label("4 questions").is_some());
    assert!(h.get_by_label("Insert exam-TEST-1F-Q2").accesskit_node().is_disabled(), "and it cannot be inserted");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_folder_without_questions_is_explained() {
    let dir = std::env::temp_dir().join(format!("deckcraft-exam-ui-empty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let h = harness(Some(&dir));
    assert!(h.query_by_label_contains("items/exam").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "writes a screenshot for review"]
fn screenshot() {
    let _gpu = crate::gpu_lock::gpu_lock();
    let dir = library();
    let mut h = harness(Some(&dir));
    h.state_mut().exam_lib.query.text = "equations".into();
    h.run_steps(4);
    let path = std::env::var("DECKCRAFT_SHOT").unwrap_or_else(|_| "/tmp/exam-library.png".into());
    h.render().unwrap().save(path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
