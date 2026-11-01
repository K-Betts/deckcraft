//! A school's own curriculum, built into a private web build so Get Ready works without each
//! teacher loading the file.
//!
//! `DECKCRAFT_WEB_CURRICULUM=/path/to/curriculum.json trunk build --release` puts the file in the
//! app (`build.rs`). A curriculum file is usually the school's own and may use a provider's
//! wording, so such a build belongs on a private site (`docs/web-hosting.md`), never on the
//! public one, which is built without it.
//!
//! The built-in curriculum is used when the browser has none, and replaces the browser's copy
//! when a new version is published; a teacher who loads another file keeps it until then.

use deckcraft_ui_egui::SlideApp;

const CURRICULUM: &str = include_str!(concat!(env!("OUT_DIR"), "/curriculum.json"));
/// Which built-in curriculum this browser last took, by fingerprint.
const SEEN_KEY: &str = "deckcraft.builtInCurriculum";

/// Give `app` the built-in curriculum if this build has one and the browser has not taken this
/// version yet (or has no curriculum at all).
pub fn apply(app: &mut SlideApp) {
    if CURRICULUM.trim().is_empty() {
        return;
    }
    let id = fingerprint(CURRICULUM);
    let store = crate::web::storage();
    let seen = store.as_ref().and_then(|s| s.get_item(SEEN_KEY).ok().flatten());
    if app.ui.curriculum.trim().is_empty() || seen.as_deref() != Some(id.as_str()) {
        app.ui.curriculum = CURRICULUM.to_string();
        if let Some(s) = store {
            let _ = s.set_item(SEEN_KEY, &id);
        }
    }
}

/// FNV-1a, enough to tell one published file from the next.
fn fingerprint(text: &str) -> String {
    let h = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
    format!("{h:016x}")
}
