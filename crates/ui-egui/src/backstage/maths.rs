//! The maths edition's parts of the File view: the Home cards (an update is ready; switch on a
//! subject's tools) and the teacher's own templates.

use egui::{RichText, Ui, vec2};
use serde_json::json;

use super::dim;
use crate::SlideApp;
use crate::theme::{self, Tokens};

/// Home, under the title: the update card, and the card offering a subject's tools until the
/// teacher switches one on or says Not now.
pub(super) fn home_cards(app: &mut SlideApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    crate::maths::update_ui::home_card(app, ui);
    if app.ui.maths.packs.is_empty() && !app.ui.maths.subjects_hint_dismissed {
        ui.add_space(12.0);
        egui::Frame::new().fill(t.selected).corner_radius(theme::radius::SURFACE).inner_margin(egui::Margin::symmetric(18, 14)).show(ui, |ui| {
            // The text wraps and the buttons sit under it, so the card fits a narrow window (iPad
            // upright, a small laptop) instead of pushing its buttons off the edge.
            ui.add(
                egui::Label::new(
                    "Teaching a subject? Switch on its tools (Teacher Mode aids, starter slides). You can change this any time in Preferences.",
                )
                .wrap(),
            );
            ui.horizontal_wrapped(|ui| {
                for pack in crate::maths::packs::PACKS {
                    let main = egui::Button::new(RichText::new(format!("Turn on {}", pack.label)).color(t.accent_text).font(theme::bold(13.0)))
                        .fill(t.accent)
                        .corner_radius(theme::radius::CONTROL)
                        .min_size(vec2(0.0, 30.0));
                    if ui.add(main).clicked() {
                        let _ = app.run("app.pack", json!({"id": pack.id, "on": true}));
                        let _ = app.run("app.dismissSubjectsHint", json!({}));
                    }
                }
                if ui.button("Not now").clicked() {
                    let _ = app.run("app.dismissSubjectsHint", json!({}));
                }
            });
        });
    }
}

/// The teacher's own templates, looked up at most every two seconds: the page is redrawn every
/// frame, and the folder is the teacher's, so a template saved elsewhere shows up soon.
pub(super) fn personal_templates(ui: &Ui) -> Vec<deckcraft_engine::templates::PersonalTemplate> {
    let id = egui::Id::unique("deckcraft-personal-templates");
    let now = ui.input(|i| i.time);
    let cached: Option<(f64, Vec<deckcraft_engine::templates::PersonalTemplate>)> = ui.ctx().data(|d| d.get_temp(id));
    match cached {
        Some((at, list)) if now - at < 2.0 => list,
        _ => {
            let list = deckcraft_engine::templates::personal();
            ui.ctx().data_mut(|d| d.insert_temp(id, (now, list.clone())));
            list
        }
    }
}

/// New, under the templates: where the teacher's own templates are kept.
pub(super) fn templates_folder_hint(ui: &mut Ui, query: &str) {
    if query.trim().is_empty()
        && let Some(dir) = deckcraft_engine::templates::personal_dir()
    {
        ui.add_space(8.0);
        dim(ui, &format!("Your own templates: save a presentation here, or use File ▸ Save as Template.   {}", dir.display()));
    }
}
