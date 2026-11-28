//! The three settings as JSON text, for hosts with no settings folder (the browser keeps them in
//! local storage). Reading is as defensive as for the files: anything unusable gives `None`.

use super::{MAX_BYTES, Prefs, UiState};
use crate::maths::maths_tools::MathsToolsSettings;

fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Option<T> {
    if text.len() as u64 > MAX_BYTES {
        return None;
    }
    serde_json::from_str(text).ok()
}

pub fn ui(ui: &UiState) -> String {
    serde_json::to_string(ui).unwrap_or_default()
}

pub fn prefs(prefs: &Prefs) -> String {
    serde_json::to_string(prefs).unwrap_or_default()
}

pub fn maths_tools(settings: &MathsToolsSettings) -> String {
    serde_json::to_string(settings).unwrap_or_default()
}

pub fn parse_ui(text: &str) -> Option<UiState> {
    parse::<UiState>(text).map(UiState::sanitized)
}

pub fn parse_prefs(text: &str) -> Option<Prefs> {
    parse::<Prefs>(text).map(Prefs::sanitized)
}

pub fn parse_maths_tools(text: &str) -> Option<MathsToolsSettings> {
    parse(text)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn text_settings_round_trip_and_damaged_text_gives_none() {
        let mut ui = UiState::default();
        ui.maths.packs.insert("maths".into());
        ui.maths.show_community_links = true;
        let back = parse_ui(&super::ui(&ui)).unwrap();
        assert!(back.maths.packs.contains("maths"), "the Maths pack survives a reload");
        let prefs = Prefs { author: "A. Teacher".into(), ..Prefs::default() };
        assert_eq!(parse_prefs(&super::prefs(&prefs)).unwrap().author, "A. Teacher");
        let tools = MathsToolsSettings::default();
        assert_eq!(parse_maths_tools(&super::maths_tools(&tools)).unwrap(), tools);
        for bad in ["", "{", "null", "[1]", "\u{0}", "{\"zoom\": \"big\"}"] {
            assert!(parse_ui(bad).is_none(), "{bad:?}");
        }
        assert!(parse_ui(&"x".repeat(MAX_BYTES as usize + 1)).is_none());
    }
}
