//! Builds a school's own material into a private web build (see `src/school.rs`). Without any of
//! these variables the app is built with none, as published.
//!
//! - `DECKCRAFT_WEB_SCHOOL`: the Mac app's settings folder (`~/Library/Application Support/DeckCraft`).
//!   From its `ui.json` the curriculum, own questions and question templates, removed questions,
//!   subject packs and Get Ready choices are taken; from `maths-tools.json` the Teacher Mode tools;
//!   and the lesson templates in `Templates/`. Nothing else (recent files, the author's name,
//!   folder paths) is read.
//! - `DECKCRAFT_WEB_CURRICULUM`: a curriculum file, used in place of the one in `ui.json`.
//! - `DECKCRAFT_WEB_EXAM=1`: the site has the exam question library beside it (`exam/`, put there
//!   by `packaging/web/deploy-private.sh`); the app reads it from there.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The fields of the Mac app's `ui.json` a private build carries.
const UI_FIELDS: [&str; 8] =
    ["curriculum", "myQuestions", "myTemplates", "removedQuestions", "packs", "getReadyMode", "getReadyYear", "getReadyRoute"];
const TEMPLATE_EXTS: [&str; 3] = ["deckcraft", "pptx", "potx"];

fn env(name: &str) -> Option<String> {
    println!("cargo:rerun-if-env-changed={name}");
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default());
    let mut school = serde_json::Map::new();
    let mut templates: Vec<(String, PathBuf)> = vec![];

    if let Some(dir) = env("DECKCRAFT_WEB_SCHOOL") {
        let dir = PathBuf::from(dir);
        println!("cargo:warning=building in the school material from {}: this build is for a private site only", dir.display());
        read_ui(&dir.join("ui.json"), &mut school);
        let tools = dir.join("maths-tools.json");
        println!("cargo:rerun-if-changed={}", tools.display());
        if let Some(v) = std::fs::read_to_string(&tools).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
            school.insert("mathsTools".into(), v);
        }
        let folder = dir.join("Templates");
        println!("cargo:rerun-if-changed={}", folder.display());
        templates = list_templates(&folder);
    }
    if let Some(path) = env("DECKCRAFT_WEB_CURRICULUM") {
        println!("cargo:rerun-if-changed={path}");
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                println!("cargo:warning=building in the curriculum from {path}: this build is for a private site only");
                school.insert("curriculum".into(), text.into());
            }
            Err(e) => println!("cargo:warning=DECKCRAFT_WEB_CURRICULUM={path} cannot be read ({e}); building without it"),
        }
    }
    if env("DECKCRAFT_WEB_EXAM").is_some_and(|v| v != "0") {
        school.insert("examLibrary".into(), true.into());
    }

    let text = if school.is_empty() { String::new() } else { serde_json::Value::Object(school).to_string() };
    write(&out.join("school.json"), text.as_bytes());

    // The templates, as `(name, bytes)` pairs included from where they are.
    let mut list = String::from("&[\n");
    for (name, path) in &templates {
        let _ = writeln!(list, "    ({name:?}, include_bytes!({:?})),", path.display().to_string());
    }
    list.push(']');
    write(&out.join("school_templates.rs"), list.as_bytes());
}

/// The fields of `ui.json` in [`UI_FIELDS`] that are set.
fn read_ui(path: &Path, school: &mut serde_json::Map<String, serde_json::Value>) {
    println!("cargo:rerun-if-changed={}", path.display());
    let ui = match std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).map_err(|e| e.to_string()))
    {
        Ok(v) => v,
        Err(e) => {
            println!("cargo:warning={} cannot be read ({e}); building without its settings", path.display());
            return;
        }
    };
    for key in UI_FIELDS {
        if let Some(v) = ui.get(key).filter(|v| !v.is_null()) {
            school.insert(key.into(), v.clone());
        }
    }
}

/// The presentation files directly in `folder`, by name, skipping hidden and lock files.
fn list_templates(folder: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|x| x.to_str()).is_some_and(|x| TEMPLATE_EXTS.contains(&x.to_ascii_lowercase().as_str())))
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?.to_string();
            (!name.starts_with('.') && !name.starts_with("~$")).then_some((name, p))
        })
        .collect();
    out.sort();
    out
}

fn write(path: &Path, bytes: &[u8]) {
    if let Err(e) = std::fs::write(path, bytes) {
        println!("cargo:warning=cannot write {}: {e}", path.display());
    }
}
