//! The maths edition's dialogs (Get Ready, Choose a Skill, Manage Questions, Exam Question, the
//! Question Library, its own Equation dialog, Update) and its additions to Preferences. Upstream's
//! `dialogs.rs` asks here first: [`title`], [`body`] and the Preferences hooks.

mod equation;
pub(crate) mod exam_library;
mod exam_question;
mod get_ready;
mod taught;

use egui::{Color32, Ui, vec2};
use serde_json::{Value, json};

use super::Dialog;
use crate::SlideApp;
use crate::maths::packs;
use crate::theme::{self, Tokens};

#[cfg(test)]
pub(crate) use get_ready::shows_as_for_tests;

/// The window title of a maths edition dialog.
pub(super) fn title(id: &str) -> Option<&'static str> {
    Some(match id {
        "getReady" => "Get Ready Starter",
        "examQuestion" => "Exam Question",
        "examLibrary" => "Question Library",
        "chooseSkill" => "Choose a Skill",
        "questionBank" => "Manage Questions",
        "update" => "Update DeckCraft",
        _ => return None,
    })
}

/// The size a maths edition dialog's window starts at.
pub(super) fn sized<'a>(window: egui::Window<'a>, id: &str) -> egui::Window<'a> {
    if id == "examLibrary" { window.default_size(vec2(980.0, 640.0)).min_width(940.0).min_height(600.0) } else { window }
}

/// Draw dialog `d` when it is one of the maths edition's (its Equation dialog replaces
/// upstream's): `Some(close)`, or `None` for upstream's own dialogs.
pub(super) fn body(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> Option<bool> {
    Some(match d.id.as_str() {
        "equation" => equation::equation_dialog(app, ui, d),
        "getReady" => get_ready::get_ready_dialog(app, ui, d),
        "examQuestion" => exam_question::exam_question_dialog(app, ui, d),
        "examLibrary" => exam_library::dialog(app, ui, d),
        "chooseSkill" => get_ready::choose_skill_dialog(app, ui, d),
        "questionBank" => get_ready::question_bank_dialog(app, ui, d),
        "update" => crate::maths::update_ui::dialog(app, ui),
        _ => return None,
    })
}

/// OK and Cancel, with OK greyed out until there is something to do.
pub(super) fn buttons_if(ui: &mut Ui, ok_label: &str, can_ok: bool) -> (bool, bool) {
    let mut ok = false;
    let mut cancel = false;
    ui.add_space(8.0);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let t = Tokens::get(ui.ctx());
        // The one main action is a filled accent button; the way out is quiet text.
        let go = ui.add_enabled(
            can_ok,
            egui::Button::new(egui::RichText::new(ok_label).color(t.accent_text).font(theme::bold(13.0)))
                .fill(t.accent)
                .corner_radius(theme::radius::CONTROL)
                .min_size(vec2(88.0, 32.0)),
        );
        if can_ok && (go.clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter))) {
            ok = true;
        }
        if ui.add(egui::Button::new(egui::RichText::new("Cancel").color(t.text_dim)).fill(Color32::TRANSPARENT).min_size(vec2(80.0, 32.0))).clicked()
        {
            cancel = true;
        }
    });
    (ok, cancel)
}

/// What the maths edition adds to Preferences, as it was when the dialog opened.
pub(super) fn preferences_snapshot(app: &SlideApp) -> Value {
    json!({
        "appearance": crate::maths::appearance::appearance_mode(app.ui.maths.follow_system, app.ui.brightness),
        "packs": app.ui.maths.packs,
        "communityLinks": app.ui.maths.show_community_links,
        "drive": app.ui.maths.drive_enabled,
        "updates": app.ui.maths.check_for_updates,
    })
}

/// Puts back the appearance and the maths edition's settings Preferences opened with. `before`
/// holds upstream's snapshot, with [`preferences_snapshot`] under `maths`.
pub(super) fn restore_preferences(app: &mut SlideApp, before: &Value) {
    let Some(maths) = before.get("maths") else { return };
    if let Some(mode) = maths.get("appearance").and_then(Value::as_str)
        && mode != crate::maths::appearance::appearance_mode(app.ui.maths.follow_system, app.ui.brightness)
    {
        let _ = app.run("view.appearance", json!({ "mode": mode }));
        // "Match this computer" has no colour of its own: put back the one it had.
        if let Some(dark) = before.get("dark").and_then(Value::as_bool) {
            app.ui.brightness = if dark { theme::Brightness::Dark } else { theme::Brightness::Light };
        }
    }
    if let Some(on) = maths.get("drive").and_then(Value::as_bool)
        && on != app.ui.maths.drive_enabled
    {
        let _ = app.run("app.drive", json!({"on": on}));
    }
    if let Some(on) = maths.get("updates").and_then(Value::as_bool) {
        app.ui.maths.check_for_updates = on;
    }
    if let Some(on) = maths.get("communityLinks").and_then(Value::as_bool) {
        app.ui.maths.show_community_links = on;
    }
    if let Some(packs) = maths.get("packs").and_then(|p| serde_json::from_value(p.clone()).ok()) {
        app.ui.maths.packs = packs;
    }
}

/// Preferences ▸ General: the Appearance drop-down (Match this computer, Light, Dark).
pub(super) fn appearance_row(app: &mut SlideApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label("Appearance");
        crate::maths::appearance::appearance_picker(app, ui, "prefs_appearance");
    });
}

/// Preferences: the Subjects, Updates, Google Drive and Links sections.
pub(super) fn preferences_sections(app: &mut SlideApp, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Subjects").font(theme::bold(13.0)));
    for pack in packs::PACKS {
        let mut on = app.pack_on(pack.id);
        if ui.checkbox(&mut on, format!("{} tools", pack.label)).on_hover_text(pack.blurb).changed() {
            let _ = app.run("app.pack", json!({"id": pack.id, "on": on}));
        }
        ui.label(egui::RichText::new(pack.blurb).size(11.0).color(Tokens::get(ui.ctx()).text_faint));
    }
    if app.services.maths.updates.is_some() {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Updates").font(theme::bold(13.0)));
        let mut auto = app.ui.maths.check_for_updates;
        if ui.checkbox(&mut auto, "Check for a newer version when DeckCraft starts (contacts github.com)").changed() {
            let _ = app.run("app.updates", json!({"on": auto}));
        }
        if ui.button("Check for updates now…").clicked() {
            let _ = app.run("update.check", json!({}));
            app.dialog = Some(Dialog::new("update"));
        }
    }
    // Only builds that have Drive (the desktop app) offer the switch.
    if app.services.maths.drive.is_some() {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Google Drive").font(theme::bold(13.0)));
        let mut drive = app.ui.maths.drive_enabled;
        if ui.checkbox(&mut drive, "Use Google Drive (optional; you sign in on File > Account)").changed() {
            let _ = app.run("app.drive", json!({"on": drive}));
        }
    }
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Links").font(theme::bold(13.0)));
    let mut links = app.ui.maths.show_community_links;
    if ui.checkbox(&mut links, "Show community links (Discord, website, source) in Home and About").changed() {
        let _ = app.run("app.communityLinks", json!({"on": links}));
    }
    ui.add_space(4.0);
}
