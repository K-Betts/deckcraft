//! The teacher's own templates: presentation files in a per-user templates folder, listed by
//! File › New and written by Save as Template.

use super::{Result, bad};

/// A teacher's own template: a presentation file kept in the personal templates folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonalTemplate {
    /// The file's name without its extension: what File › New shows.
    pub name: String,
    pub path: std::path::PathBuf,
}

/// The per-user folder for personal templates (`DeckCraft/Templates` next to the recovery folder).
/// Not on the web build, which has no folders.
pub fn personal_dir() -> Option<std::path::PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Application Support/DeckCraft/Templates"));
    }
    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("DeckCraft").join("Templates"));
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| home.map(|h| h.join(".local/share")))
        .map(|d| d.join("deckcraft/templates"))
}

/// The templates in `dir`: `.deckcraft`, `.pptx` and `.potx` files, by name (not case-sensitive).
/// A missing or unreadable folder has none.
pub fn personal_in(dir: &std::path::Path) -> Vec<PersonalTemplate> {
    let Some(dir) = dir.to_str() else { return vec![] };
    let mut out: Vec<PersonalTemplate> = crate::io::list_files(dir)
        .into_iter()
        .map(std::path::PathBuf::from)
        .filter(|p| p.extension().and_then(|x| x.to_str()).is_some_and(|x| ["deckcraft", "pptx", "potx"].contains(&x.to_ascii_lowercase().as_str())))
        .filter(|p| !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.') || n.starts_with("~$")))
        .filter_map(|p| Some(PersonalTemplate { name: p.file_stem()?.to_str()?.to_string(), path: p }))
        .collect();
    out.sort_by_key(|t| t.name.to_lowercase());
    out
}

/// The teacher's own templates, from [`personal_dir`].
pub fn personal() -> Vec<PersonalTemplate> {
    personal_dir().map(|d| personal_in(&d)).unwrap_or_default()
}

/// The personal template called `name` (not case-sensitive).
pub fn find_personal(name: &str) -> Option<PersonalTemplate> {
    let name = name.trim();
    personal().into_iter().find(|t| t.name.eq_ignore_ascii_case(name))
}

/// Where a new personal template called `name` goes in `dir`: `<dir>/<name>.deckcraft`. The name
/// is made safe as a file name, and an existing template is never overwritten.
pub fn personal_path(dir: &std::path::Path, name: &str) -> Result<std::path::PathBuf> {
    let clean: String =
        name.chars().map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { ' ' } else { c }).collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean = clean.trim_start_matches('.').trim().to_string();
    if clean.is_empty() {
        return Err(bad("file.saveTemplate", "give the template a name"));
    }
    if personal_in(dir).iter().any(|t| t.name.eq_ignore_ascii_case(&clean)) {
        return Err(bad("file.saveTemplate", format!("there is already a template called `{clean}`; choose another name")));
    }
    Ok(dir.join(format!("{clean}.deckcraft")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn personal_templates_are_the_presentation_files_in_the_folder_by_name() {
        let dir = std::env::temp_dir().join(format!("deckcraft-templates-{}-list", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        for f in ["b lesson.deckcraft", "A Lesson.pptx", "exam.potx", "notes.txt", ".hidden.deckcraft", "~$temp.pptx"] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let names: Vec<String> = personal_in(&dir).into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["A Lesson", "b lesson", "exam"], "sorted by name, presentations only, no hidden or lock files");
        assert!(personal_in(&dir.join("missing")).is_empty(), "a missing folder has none");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
