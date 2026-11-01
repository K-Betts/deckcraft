//! Personal templates: Save as Template keeps a copy in the teacher's templates folder, and
//! File › New starts an untitled copy of one.

use super::{DocState, EngineError, Result, Session, Value, bad, bool_or, json, open_presentation, save_bytes, str_param};

/// `file.new` with `templateFile`, or a `template` that names one of the teacher's own: the new
/// presentation, or `None` for an ordinary `file.new`.
pub(super) fn new(s: &mut Session, p: &Value, template: Option<&str>, theme_name: Option<&str>) -> Option<Result<Value>> {
    if let Some(path) = str_param(p, "templateFile").filter(|t| !t.is_empty()) {
        if template.is_some() || theme_name.is_some() || bool_or(p, "blank", false) {
            return Some(Err(bad("file.new", "`templateFile` stands alone: don't pass `template`, `theme` or `blank`")));
        }
        return Some(new_from_file(s, path));
    }
    if let Some(name) = template
        && crate::templates::find(name).is_none()
        && let Some(own) = crate::templates::find_personal(name)
    {
        return Some(new_from_file(s, &own.path.to_string_lossy()));
    }
    None
}

/// A new untitled presentation that is a copy of the file at `path`: a personal template.
fn new_from_file(s: &mut Session, path: &str) -> Result<Value> {
    let name = std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path).to_string();
    let bytes = crate::io::read_file(path).map_err(EngineError::Other)?;
    let doc = open_presentation(&name, &bytes)?;
    let stem = std::path::Path::new(path).file_stem().and_then(|n| n.to_str()).unwrap_or(&name).to_string();
    let slides = doc.slides.len();
    let title = s.next_untitled();
    let i = s.add_document(DocState::new(doc, None, title));
    Ok(json!({"document": i, "slides": slides, "template": stem}))
}

pub(super) fn save_template(s: &mut Session, p: &Value) -> Result<Value> {
    let dir =
        crate::templates::personal_dir().ok_or_else(|| bad("file.saveTemplate", "templates are kept by the desktop app, not in the browser"))?;
    save_template_in(s, p, &dir)
}

/// [`save_template`] into the folder `dir`.
fn save_template_in(s: &mut Session, p: &Value, dir: &std::path::Path) -> Result<Value> {
    let name = match str_param(p, "name").filter(|n| !n.trim().is_empty()) {
        Some(n) => n.to_string(),
        None => {
            let title = s.doc()?.title();
            title.trim_end_matches(".deckcraft").trim_end_matches(".pptx").to_string()
        }
    };
    let path = crate::templates::personal_path(dir, &name)?;
    let (dir_s, path_s) = (dir.to_string_lossy().to_string(), path.to_string_lossy().to_string());
    let bytes = save_bytes(&s.doc()?.doc, "deckcraft")?;
    crate::io::create_dir(&dir_s).map_err(EngineError::Other)?;
    crate::io::write_file(&path_s, &bytes).map_err(EngineError::Other)?;
    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("").to_string();
    Ok(json!({"name": stem, "path": path_s}))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use serde_json::json;

    use super::save_template_in;
    use crate::Session;

    #[test]
    fn save_template_keeps_a_copy_in_the_templates_folder_and_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("deckcraft-templates-{}-save", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = Session::with_new();
        s.execute("slide.new", &json!({"layout": "titleOnly", "title": "Unit fractions"})).unwrap();
        let r = save_template_in(&mut s, &json!({"name": "Lesson slides"}), &dir).unwrap();
        assert_eq!(r["name"], "Lesson slides");
        assert!(dir.join("Lesson slides.deckcraft").is_file(), "created the folder and the file");
        assert!(s.active().unwrap().path.is_none(), "the open presentation is not attached to the template file");
        let again = save_template_in(&mut s, &json!({"name": "lesson SLIDES"}), &dir);
        assert!(again.is_err(), "an existing template (any case) is never overwritten");
        let unsafe_name = save_template_in(&mut s, &json!({"name": "../../escape"}), &dir).unwrap();
        assert!(unsafe_name["path"].as_str().unwrap().starts_with(dir.to_str().unwrap()), "the name cannot leave the folder: {unsafe_name}");
        assert!(save_template_in(&mut s, &json!({"name": "  ...  "}), &dir).is_err(), "a name of nothing is refused");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_template_file_opens_as_an_untitled_copy_and_the_file_is_untouched() {
        let dir = std::env::temp_dir().join(format!("deckcraft-templates-{}-open", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = Session::with_new();
        s.execute("slide.new", &json!({"layout": "titleOnly", "title": "From my template"})).unwrap();
        let saved = save_template_in(&mut s, &json!({"name": "Mine"}), &dir).unwrap();
        let path = saved["path"].as_str().unwrap().to_string();
        let before = std::fs::read(&path).unwrap();
        let slides = s.active().unwrap().doc.slides.len();
        let r = s.execute("file.new", &json!({"templateFile": path})).unwrap();
        assert_eq!(r["template"], "Mine");
        let st = s.active().unwrap();
        assert!(st.path.is_none(), "a copy, not the template file itself");
        assert_eq!(st.doc.slides.len(), slides);
        assert!(st.doc.slides.iter().any(|sl| sl.title() == "From my template"));
        s.execute("slide.new", &json!({})).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before, "editing the copy leaves the template alone");
        assert!(s.execute("file.new", &json!({"templateFile": path, "blank": true})).is_err(), "templateFile stands alone");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
