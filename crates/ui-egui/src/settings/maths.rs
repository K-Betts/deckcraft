//! The maths tools' settings file (`maths-tools.json`), beside the other local settings.

use std::path::Path;

use super::{read, write_atomic};
pub use crate::maths::maths_tools::MathsToolsSettings;

pub const MATHS_TOOLS_FILE: &str = "maths-tools.json";

/// Load local maths-tool preferences, defaulting when the file is missing or unusable.
pub fn load_maths_tools(dir: &Path) -> MathsToolsSettings {
    read(&dir.join(MATHS_TOOLS_FILE)).unwrap_or_default()
}

/// Save maths-tool preferences outside the presentation, beside the other local settings.
pub fn save_maths_tools(dir: &Path, settings: &MathsToolsSettings) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        log::warn!("{}: {e}; maths tools settings not saved", dir.display());
        return;
    }
    let path = dir.join(MATHS_TOOLS_FILE);
    if let Err(e) = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other).and_then(|b| write_atomic(&path, &b)) {
        log::warn!("{}: {e}; maths tools settings not saved", path.display());
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("deckcraft-settings-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn maths_tools_settings_round_trip_in_the_local_config_file() {
        let d = dir("maths-tools");
        let settings = MathsToolsSettings {
            enabled: [crate::maths::maths_tools::MathsTool::Calculator, crate::maths::maths_tools::MathsTool::Ruler].into_iter().collect(),
        };
        save_maths_tools(&d, &settings);
        assert_eq!(load_maths_tools(&d), settings);
        assert!(d.join(MATHS_TOOLS_FILE).is_file());
        assert!(!d.join("presentation.pptx").exists());
    }
}
