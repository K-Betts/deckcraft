//! Appearance: follow the computer's light or dark setting, or choose one.

use crate::theme::Brightness;

/// `"system"`, `"light"` or `"dark"`: how the appearance is being chosen.
pub fn appearance_mode(follow_system: bool, b: Brightness) -> &'static str {
    match (follow_system, b) {
        (true, _) => "system",
        (false, Brightness::Dark) => "dark",
        (false, Brightness::Light) => "light",
    }
}

/// The Appearance drop-down shared by Preferences and the File page.
pub fn appearance_picker(app: &mut crate::SlideApp, ui: &mut egui::Ui, salt: &str) {
    let now = appearance_mode(app.ui.maths.follow_system, app.ui.brightness);
    let name = |m: &str| match m {
        "system" => "Match this computer",
        "dark" => "Dark",
        _ => "Light",
    };
    let combo = craft_ui::ComboBox::from_id_salt(salt).width(200.0).selected_text(name(now)).show_ui(ui, |ui| {
        for mode in ["system", "light", "dark"] {
            if ui.selectable_label(now == mode, name(mode)).clicked() {
                let _ = app.run("view.appearance", serde_json::json!({ "mode": mode }));
            }
        }
    });
    combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Appearance"));
}
