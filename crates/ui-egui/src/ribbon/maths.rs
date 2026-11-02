//! The maths edition's ribbon groups. Subject packs add; they never move what is already there
//! (principle 4).

use serde_json::json;

use super::big;
use crate::SlideApp;
use crate::icons::Icon;
use crate::maths::packs;
use crate::widgets::{big_button, group, rows};

/// Insert: Get Ready Starter, Exam Question and Question Library (Maths pack).
pub(super) fn insert_group(app: &mut SlideApp, ui: &mut egui::Ui) {
    if app.pack_on(packs::MATHS) {
        group(ui, |ui| {
            big(app, ui, Icon::NewSlide, "Get Ready\nStarter", "app.dialog", json!({"id": "getReady"}));
            big(app, ui, Icon::NewSlide, "Exam\nQuestion", "app.dialog", json!({"id": "examQuestion"}));
            big(app, ui, Icon::OpenFolder, "Question\nLibrary", "app.dialog", json!({"id": "examLibrary"}));
        });
    }
}

/// Animations: Reveal Steps (Maths pack).
pub(super) fn animations_group(app: &mut SlideApp, ui: &mut egui::Ui) {
    if app.pack_on(packs::MATHS) {
        group(ui, |ui| {
            big(app, ui, Icon::Numbering, "Reveal\nSteps", "animation.steps", json!({}));
        });
    }
}

/// View: Teacher Mode, and whether its marks stay when the slide changes.
pub(super) fn view_group(app: &mut SlideApp, ui: &mut egui::Ui) {
    group(ui, |ui| {
        let has = app.session.active().is_some();
        if big_button(ui, Icon::PlayFromCurrent, "Teacher\nMode", has)
            .on_hover_text("Present with pen, revealer, spotlight and timers (Cmd+Shift+T)")
            .clicked()
        {
            let _ = app.run("show.teacher", json!({"on": true}));
        }
        rows(ui, |ui| {
            let mut keep = app.ui.maths.keep_annotations_on_advance;
            if ui.checkbox(&mut keep, "Keep marks on advance").changed() {
                app.ui.maths.keep_annotations_on_advance = keep;
            }
        });
    });
}
