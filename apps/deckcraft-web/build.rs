//! Builds a school's own curriculum into the web app when `DECKCRAFT_WEB_CURRICULUM` names a
//! curriculum file (see `src/school.rs`). Without it the app is built with none, as published.

fn main() {
    println!("cargo:rerun-if-env-changed=DECKCRAFT_WEB_CURRICULUM");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()).join("curriculum.json");
    let text = match std::env::var("DECKCRAFT_WEB_CURRICULUM") {
        Ok(path) if !path.trim().is_empty() => {
            println!("cargo:rerun-if-changed={path}");
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    println!("cargo:warning=building in the curriculum from {path}: this build is for a private site only");
                    text
                }
                Err(e) => {
                    println!("cargo:warning=DECKCRAFT_WEB_CURRICULUM={path} cannot be read ({e}); building without a curriculum");
                    String::new()
                }
            }
        }
        _ => String::new(),
    };
    if let Err(e) = std::fs::write(&out, text) {
        println!("cargo:warning=cannot write {}: {e}", out.display());
    }
}
