//! A school's own material, built into a private web build so the web app matches the Mac app
//! without each teacher loading anything: the curriculum, own questions, subject packs, Teacher
//! Mode tools and lesson templates, and (served beside the site) the exam question library.
//!
//! `packaging/web/deploy-private.sh` builds with `DECKCRAFT_WEB_SCHOOL` set to the Mac app's
//! settings folder (see `build.rs`). That material is usually the school's own and may use a
//! provider's or an exam board's wording, so such a build belongs on a private site
//! (`docs/web-hosting.md`), never on the public one, which is built without it.
//!
//! Published settings are taken when the browser has none, or when a new version is published;
//! what a teacher changed in the browser is kept until then (`deckcraft_ui_egui::maths::published`).

use deckcraft_engine::web_files;
use deckcraft_ui_egui::SlideApp;
use deckcraft_ui_egui::maths::published;

const SCHOOL: &str = include_str!(concat!(env!("OUT_DIR"), "/school.json"));
/// The lesson templates, `(file name, bytes)`.
const TEMPLATES: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/school_templates.rs"));
/// Which published version this browser last took, by fingerprint.
const SEEN_KEY: &str = "deckcraft.builtInSchool";
/// Where the exam question library is on the site (as laid out by `deploy-private.sh`): the same
/// layout as the library folder on the Mac, `items/exam/*.json` and `derived/exam_crops/*.png`.
pub(crate) const EXAM_DIR: &str = "exam";

/// Give `app` the published material it has not taken yet, and the templates.
pub fn apply(app: &mut SlideApp) {
    for (name, bytes) in TEMPLATES {
        web_files::provide(&format!("{}/{name}", deckcraft_engine::templates::WEB_DIR), bytes.to_vec());
    }
    // A folder chosen on another computer means nothing here; the published library is set when
    // it has arrived (`exam_library`).
    app.ui.maths.exam_library_dir.clear();
    if SCHOOL.trim().is_empty() {
        return;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(SCHOOL) else {
        log::error!("the published school material is damaged; it was not used");
        return;
    };
    let id = published::fingerprint(SCHOOL);
    let store = crate::web::settings::storage();
    let seen = store.as_ref().and_then(|s| s.get_item(SEEN_KEY).ok().flatten());
    published::apply(app, &value, seen.as_deref() != Some(id.as_str()));
    if let Some(s) = store {
        let _ = s.set_item(SEEN_KEY, &id);
    }
}

/// Whether this build was published with the exam question library beside it.
pub fn exam_library() -> bool {
    serde_json::from_str::<serde_json::Value>(SCHOOL).ok().and_then(|v| v.get("examLibrary").and_then(serde_json::Value::as_bool)).unwrap_or(false)
}
