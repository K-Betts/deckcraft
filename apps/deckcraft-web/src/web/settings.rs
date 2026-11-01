//! The settings (interface, preferences, maths tools) in the browser's local storage. Decks are
//! never stored here: they live in files the teacher opens and saves.

use deckcraft_ui_egui::SlideApp;

const SETTINGS_KEYS: [&str; 3] = ["deckcraft.ui", "deckcraft.prefs", "deckcraft.mathsTools"];
/// The most often (seconds) the settings are written to local storage.
const SAVE_EVERY: f64 = 1.0;

pub(crate) fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// What was last written to local storage, so it is written only when something changed.
pub(super) struct Settings {
    /// The settings text as last written.
    saved: [String; 3],
    last_save: f64,
}

impl Settings {
    /// Read the saved settings into `app` (a missing, blocked or damaged store leaves the defaults:
    /// private windows and locked-down school browsers can refuse local storage).
    pub(super) fn load(app: &mut SlideApp) -> Settings {
        use deckcraft_ui_egui::settings::text;
        let Some(s) = storage() else { return Settings { saved: Default::default(), last_save: 0.0 } };
        let get = |i: usize| s.get_item(SETTINGS_KEYS[i]).ok().flatten().unwrap_or_default();
        let saved = [get(0), get(1), get(2)];
        if let Some(ui) = text::parse_ui(&saved[0]) {
            app.ui = ui;
        }
        if let Some(prefs) = text::parse_prefs(&saved[1]) {
            app.session.prefs = prefs;
        }
        if let Some(m) = text::parse_maths_tools(&saved[2]) {
            app.maths.tools = m;
        }
        Settings { saved, last_save: 0.0 }
    }

    /// Keep the three settings in local storage up to date. They are compared every frame (three
    /// small JSON strings), and written when they differ, at most once a `SAVE_EVERY` seconds. A
    /// change held back by that limit asks for one more frame when the wait is over, because the
    /// app only runs a frame when something happens. A refused write is ignored; nothing depends on it.
    pub(super) fn save(&mut self, app: &SlideApp, ctx: &egui::Context) {
        use deckcraft_ui_egui::settings::text;
        let Some(s) = storage() else { return };
        let now = ctx.input(|i| i.time);
        let now_text = [text::ui(&app.ui), text::prefs(&app.session.prefs), text::maths_tools(&app.maths.tools)];
        if now_text == self.saved {
            return;
        }
        if now - self.last_save < SAVE_EVERY {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(SAVE_EVERY - (now - self.last_save) + 0.05));
            return;
        }
        self.last_save = now;
        for i in 0..3 {
            if now_text[i] != self.saved[i] && s.set_item(SETTINGS_KEYS[i], &now_text[i]).is_ok() {
                self.saved[i] = now_text[i].clone();
            }
        }
    }
}
