//! The maths edition's part of the desktop host: Google Drive, in-app updates, the folder picker
//! and the maths tools' settings file. `main.rs` reaches it through one call each.

mod drive;
mod update;

use std::path::Path;

use deckcraft_ui_egui::SlideApp;
use deckcraft_ui_egui::maths::app::MathsServices;

/// The maths edition's host services.
pub fn services() -> MathsServices {
    MathsServices {
        pick_folder: Some(Box::new(|_: &str| rfd::FileDialog::new().pick_folder().map(|p| p.to_string_lossy().to_string()))),
        // Constructing the backend reads no keychain and opens no connection; that waits for
        // Preferences ▸ Use Google Drive and the Sign in button.
        updates: super::config_dir()
            .map(|d| Box::new(update::backend(d.join("updates"))) as Box<dyn deckcraft_ui_egui::maths::update_ui::UpdateBackend>),
        drive: super::config_dir()
            .map(|d| Box::new(drive::backend(d.join("drive-cache"))) as Box<dyn deckcraft_ui_egui::maths::drive_ui::DriveBackend>),
    }
}

/// Read the maths tools' settings from the settings folder `dir`.
pub fn load_prefs(app: &mut SlideApp, dir: &Path) {
    app.maths.tools = deckcraft_ui_egui::settings::load_maths_tools(dir);
}

/// Write the maths tools' settings to the settings folder `dir`.
pub fn save_prefs(app: &SlideApp, dir: &Path) {
    deckcraft_ui_egui::settings::save_maths_tools(dir, &app.maths.tools);
}
