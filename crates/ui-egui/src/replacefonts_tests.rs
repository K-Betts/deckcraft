//! Home › Replace Fonts…: the dialog lists the deck's fonts and replaces one, as one undo step.

use deckcraft_engine::Session;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::{Services, SlideApp};

/// Build the standard Replace Fonts UI harness with a fresh session.
fn harness() -> Harness<'static, SlideApp> {
    let mut s = Session::with_new();
    s.execute("slide.new", &json!({"layout": "blank"})).unwrap();
    s.execute("insert.textBox", &json!({"rect": [40, 40, 400, 80], "text": "Hello fonts"})).unwrap();
    s.execute("text.exit", &json!({})).unwrap();
    s.execute("format.font", &json!({"family": "Comic Neue"})).unwrap();
    s.execute("format.size", &json!({"size": 40})).unwrap();
    let app = SlideApp::new(s, Services::default());
    let mut h = Harness::builder().with_size(egui::vec2(1700.0, 900.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    h
}

fn run_font(h: &Harness<'_, SlideApp>) -> Option<String> {
    let st = h.state().session.active().unwrap();
    st.current_slide()
        .unwrap()
        .shapes
        .iter()
        .find_map(|s| s.text.as_ref())
        .and_then(|b| b.paragraphs.first())
        .and_then(|p| p.runs.first())
        .and_then(|r| r.props.font.clone())
}

/// The ribbon folds the groups that do not fit into its More menu (as in the app, where folding is
/// on); open More if Replace Fonts… was folded away.
fn reach_replace_fonts<'a>(h: &'a mut Harness<'static, SlideApp>) -> egui_kittest::Node<'a> {
    h.state_mut().ribbon_fold = true;
    // The first frames teach the ribbon each group's width; one frame later it folds.
    h.run_steps(6);
    if h.query_by_label("Replace Fonts…").is_none() {
        h.get_by_label("More tools").click();
        h.run_steps(3);
    }
    h.get_by_label("Replace Fonts…")
}

/// The ribbon command opens Replace Fonts; its Replace button updates the document and leaves
/// the dialog open with a confirmation.
#[test]
fn ribbon_button_opens_the_dialog_and_replaces() {
    let mut h = harness();
    let button = reach_replace_fonts(&mut h);
    assert!(button.rect().max.x <= 1700.0 && button.rect().min.x >= 0.0, "Replace Fonts… is on screen: {:?}", button.rect());
    button.click();
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().map(|d| d.id.as_str()), Some("replaceFonts"));
    // Pick the font to replace and type the new one, the way a person would.
    if let Some(d) = h.state_mut().dialog.as_mut() {
        d.fields.insert("from".into(), "Comic Neue".into());
        d.fields.insert("to".into(), "Lato".into());
    }
    h.run_steps(2);
    // The ribbon has a Replace button too (Find and Replace); the dialog's is drawn last.
    h.query_all_by_label("Replace").last().expect("the dialog's Replace button").click();
    h.run_steps(3);
    assert_eq!(run_font(&h).as_deref(), Some("Lato"));
    // The dialog stays open with a confirmation, like PowerPoint's.
    assert!(h.query_by_label_contains("Replaced “Comic Neue”").is_some());
    h.state_mut().session.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(run_font(&h).as_deref(), Some("Comic Neue"));
}

#[test]
#[ignore = "writes a screenshot for review"]
fn screenshot() {
    let _gpu = crate::gpu_lock::gpu_lock();
    let mut h = harness();
    h.render().unwrap().save("/tmp/replacefonts0.png").unwrap();
    h.state_mut().dialog = Some(crate::dialogs::Dialog::new("replaceFonts"));
    h.run_steps(4);
    h.render().unwrap().save("/tmp/replacefonts.png").unwrap();
}
