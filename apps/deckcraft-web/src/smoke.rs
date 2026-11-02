//! `?smoke`: walk through the screens a teacher uses most, a few frames each, and log each step,
//! so a headless browser can check that the web build starts and draws them with no errors
//! (`apps/deckcraft-web/tests/smoke.mjs`, run in CI). Each step logs `smoke: <step> ok` or
//! `smoke: <step> FAILED: <why>`; the walk ends with `smoke: done`.

use deckcraft_ui_egui::SlideApp;
use serde_json::json;

/// Frames each step is drawn for before the next one.
const FRAMES: u32 = 8;
const STEPS: [&str; 9] = ["home", "getReady", "examQuestion", "examLibrary", "template", "teacher", "mathsTools", "teacherOff", "done"];

#[derive(Default)]
pub(crate) struct Smoke {
    step: usize,
    frames: u32,
}

impl Smoke {
    /// Called every frame: start the next step once the current one has been drawn.
    pub(crate) fn tick(&mut self, app: &mut SlideApp, ctx: &egui::Context) {
        if self.step >= STEPS.len() {
            return;
        }
        ctx.request_repaint();
        self.frames += 1;
        if self.frames < FRAMES {
            return;
        }
        self.frames = 0;
        let name = STEPS[self.step];
        match step(app, name) {
            Ok(note) if name == "done" => log::info!("smoke: done{note}"),
            Ok(note) => log::info!("smoke: {name} ok{note}"),
            Err(e) => log::error!("smoke: {name} FAILED: {e}"),
        }
        self.step += 1;
    }
}

fn step(app: &mut SlideApp, name: &str) -> Result<String, String> {
    match name {
        "home" => Ok(String::new()),
        "getReady" | "examQuestion" | "examLibrary" => {
            app.dialog = None;
            app.run("app.pack", json!({"id": "maths", "on": true}))?;
            app.run("app.dialog", json!({"id": name}))?;
            Ok(if name == "examLibrary" {
                format!(" (library {})", if app.ui.maths.exam_library_dir.is_empty() { "none" } else { "published" })
            } else {
                String::new()
            })
        }
        "template" => {
            app.dialog = None;
            let own = deckcraft_engine::templates::personal();
            match own.first() {
                Some(t) => {
                    app.run("file.new", json!({"template": t.name}))?;
                    Ok(format!(" ({} published templates)", own.len()))
                }
                None => Ok(" (no published templates)".into()),
            }
        }
        "teacher" => {
            app.run("show.teacher", json!({"on": true}))?;
            Ok(String::new())
        }
        "mathsTools" => {
            let t = app.show.as_mut().and_then(|s| s.teacher.as_mut()).ok_or("Teacher Mode is not on")?;
            t.open_palette();
            t.maths_tools_panel.open = true;
            Ok(format!(" ({} tools chosen; medium pen {:.1} points wide)", app.maths.tools.enabled.len(), t.pen_width * t.ink_scale()))
        }
        "teacherOff" => {
            app.run("show.teacher", json!({"on": false}))?;
            Ok(String::new())
        }
        _ => Ok(String::new()),
    }
}
