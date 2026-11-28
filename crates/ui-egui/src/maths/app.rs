//! The maths edition's part of the app state, its settings and its UI commands. Upstream's
//! `SlideApp`, `UiState` and `Services` each carry one field of these types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{drive_ui, maths_tools, packs, teacher_overlay, update_ui};
use crate::{PickFn, SlideApp, theme};

/// Host services only the maths edition uses (`Services::maths`).
#[derive(Default)]
pub struct MathsServices {
    /// Choose-a-folder dialog for a purpose (`library`) → path.
    pub pick_folder: Option<PickFn>,
    /// Google Drive, when the host was built with it (desktop only). See [`drive_ui`].
    pub drive: Option<Box<dyn drive_ui::DriveBackend>>,
    /// Updating from inside the app, when the host can (installed Mac app). See [`update_ui`].
    pub updates: Option<Box<dyn update_ui::UpdateBackend>>,
}

/// The maths edition's saved interface settings (`UiState::maths`, flattened into `ui.json`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MathsUi {
    /// Light or dark follows the computer's setting (the default). Choosing Light or Dark yourself turns this off.
    pub follow_system: bool,
    /// Teacher Mode keeps its marks and revealer when the slide changes.
    pub keep_annotations_on_advance: bool,
    /// Upstream community links (Discord, app page, source) in Home, Info and About. Off by
    /// default: a school app does not point staff at another product's chat server.
    pub show_community_links: bool,
    /// Subject packs the teacher has switched on (see [`packs`]); empty on a fresh install.
    pub packs: std::collections::BTreeSet<String>,
    /// The Home screen's “teach a subject?” card was dismissed.
    pub subjects_hint_dismissed: bool,
    /// Google Drive features are switched on (Preferences). Off by default: signed out, silent.
    pub drive_enabled: bool,
    /// Ask GitHub for a newer version when DeckCraft starts. Off by default: nothing contacts the
    /// network until the teacher turns this on or presses Check now.
    pub check_for_updates: bool,
    /// The Get Ready starter's last choices, one `(topic, level)` per question; topic "" and level 0 mean any.
    pub get_ready_slots: Vec<(String, u8)>,
    /// Questions the teacher has added to the Get Ready bank (Manage Questions).
    pub my_questions: Vec<deckcraft_engine::questions::Question>,
    /// Question templates the teacher imported from a file (Manage Questions ▸ Import).
    pub my_templates: Vec<deckcraft_engine::template::Template>,
    /// The skill (small step) chosen for each Get Ready question: a `Question::skill_key`, or empty for any.
    pub get_ready_skill: Vec<String>,
    /// Ids of built-in questions the teacher has taken out of the bank.
    pub removed_questions: Vec<String>,
    /// The department's curriculum (a curriculum file's JSON, see `deckcraft_engine::curriculum`),
    /// loaded in the Get Ready dialog; empty until one is loaded.
    pub curriculum: String,
    /// The folder of past-paper questions the Question Library reads (the teacher's own; never
    /// part of DeckCraft). Empty until one is chosen.
    pub exam_library_dir: String,
    /// Get Ready: `taught` (plan from the curriculum) or `choose` (each question by hand); empty
    /// until the teacher picks, which means `taught` once a curriculum is loaded.
    pub get_ready_mode: String,
    /// Get Ready's last year group and route, e.g. `Y7` and `Core`.
    pub get_ready_year: String,
    pub get_ready_route: String,
}

impl Default for MathsUi {
    fn default() -> Self {
        MathsUi {
            follow_system: true,
            keep_annotations_on_advance: false,
            show_community_links: false,
            packs: Default::default(),
            subjects_hint_dismissed: false,
            drive_enabled: false,
            check_for_updates: false,
            get_ready_slots: vec![(String::new(), 0); 4],
            my_questions: vec![],
            my_templates: vec![],
            get_ready_skill: vec![String::new(); 4],
            removed_questions: vec![],
            curriculum: String::new(),
            exam_library_dir: String::new(),
            get_ready_mode: String::new(),
            get_ready_year: String::new(),
            get_ready_route: String::new(),
        }
    }
}

impl MathsUi {
    /// Bring values read from `ui.json` back within bounds (see `UiState::sanitized`).
    pub fn sanitize(&mut self) {
        self.packs = packs::sanitized(std::mem::take(&mut self.packs));
        self.get_ready_slots.truncate(6);
        if self.get_ready_slots.len() < 2 {
            self.get_ready_slots = MathsUi::default().get_ready_slots;
        }
        for (topic, level) in &mut self.get_ready_slots {
            topic.truncate(60);
            *level = (*level).min(4);
        }
        self.my_questions
            .retain(|q| !q.id.is_empty() && !q.text.trim().is_empty() && deckcraft_engine::questions::DIFFICULTIES.contains(&q.difficulty));
        self.my_questions.truncate(5_000);
        self.my_templates.retain(|t| t.check().is_ok());
        self.my_templates.truncate(2_000);
        self.get_ready_skill.resize(4, String::new());
        for w in &mut self.get_ready_skill {
            w.truncate(200);
        }
        self.removed_questions.truncate(5_000);
        if self.curriculum.len() > 8 << 20 {
            self.curriculum.clear();
        }
        if self.exam_library_dir.len() > 1_000 {
            self.exam_library_dir.clear();
        }
        for short in [&mut self.get_ready_mode, &mut self.get_ready_year, &mut self.get_ready_route] {
            if short.len() > 40 {
                short.clear();
            }
        }
    }
}

/// The maths edition's part of the running app (`SlideApp::maths`); none of it is saved here.
pub struct MathsApp {
    /// Maths tools available to the current local teacher account.
    pub tools: maths_tools::MathsToolsSettings,
    pub drive: drive_ui::DriveUi,
    /// How many groups of the current ribbon tab are folded into its More menu (0 when all fit).
    pub ribbon_folded: usize,
    /// Fold ribbon groups that do not fit into a More menu. On in the app; off by default in unit
    /// tests (their stand-in windows and fonts are narrower than a real one, and most tests are
    /// about what a button does, not where it sits). Tests of the folding turn it on.
    pub ribbon_fold: bool,
    /// Each ribbon tab's group widths, learned as the groups are drawn (see `ribbon::fold`).
    pub(crate) ribbon_widths: std::collections::HashMap<String, Vec<f32>>,
    pub(crate) update_started: bool,
    /// An update was installed and the teacher chose Restart: close the window on the next frame.
    pub(crate) pending_quit: bool,
    /// Teacher Mode's writing, kept when the show ends so it is still there next time.
    pub teacher_stash: Option<teacher_overlay::TeacherOverlay>,
    /// Insert › Question Library: the questions read from the teacher's folder and the last search.
    pub(crate) exam_lib: crate::dialogs::maths::exam_library::LibState,
    /// A text file the browser read for a maths dialog: `(purpose, its text or why not)`.
    pub(crate) picked_text: Option<(&'static str, Result<String, String>)>,
}

impl Default for MathsApp {
    fn default() -> Self {
        MathsApp {
            tools: Default::default(),
            drive: Default::default(),
            ribbon_folded: 0,
            ribbon_fold: !cfg!(test),
            ribbon_widths: Default::default(),
            update_started: false,
            pending_quit: false,
            teacher_stash: None,
            exam_lib: Default::default(),
            picked_text: None,
        }
    }
}

/// Narrower than this (in points) is a phone held upright: panels give way to the slide.
pub const COMPACT_WIDTH: f32 = 640.0;

pub fn is_compact(width: f32) -> bool {
    width < COMPACT_WIDTH
}

/// The maths edition's UI commands (listed in `UI_COMMANDS`), run by `SlideApp::run_ui`.
pub(crate) fn owns(id: &str) -> bool {
    matches!(
        id,
        "view.appearance"
            | "show.teacher"
            | "app.pack"
            | "app.updates"
            | "update.check"
            | "update.install"
            | "update.restart"
            | "app.drive"
            | "drive.signIn"
            | "drive.cancel"
            | "drive.signOut"
            | "drive.refresh"
            | "drive.open"
            | "drive.save"
            | "app.dismissSubjectsHint"
            | "app.communityLinks"
            | "show.teacherKeep"
    )
}

impl SlideApp {
    /// Run one of the commands [`owns`] names.
    pub(crate) fn run_maths(&mut self, id: &str, p: &Value) -> Result<Value, String> {
        match id {
            "view.appearance" => {
                match p.get("mode").and_then(Value::as_str) {
                    Some("system") => self.ui.maths.follow_system = true,
                    Some("light") => (self.ui.maths.follow_system, self.ui.brightness) = (false, theme::Brightness::Light),
                    Some("dark") => (self.ui.maths.follow_system, self.ui.brightness) = (false, theme::Brightness::Dark),
                    _ => return Err("mode must be system, light or dark".into()),
                }
                self.restyle = true;
            }
            "show.teacher" => {
                let on = self.flag(p, self.show.as_ref().is_some_and(|s| s.teacher.is_some()));
                self.set_teacher_mode(on);
            }
            "app.pack" => {
                let id = p.get("id").and_then(Value::as_str).unwrap_or_default();
                if packs::find(id).is_none() {
                    return Err(format!("unknown subject pack “{id}”"));
                }
                let on = self.flag(p, self.pack_on(id));
                if on {
                    self.ui.maths.packs.insert(id.to_string());
                } else {
                    self.ui.maths.packs.remove(id);
                }
            }
            "app.updates" => self.ui.maths.check_for_updates = self.flag(p, self.ui.maths.check_for_updates),
            "update.check" | "update.install" | "update.restart" => return self.run_update(id),
            "app.drive" => {
                self.ui.maths.drive_enabled = self.flag(p, self.ui.maths.drive_enabled);
                if !self.ui.maths.drive_enabled {
                    self.maths.drive = Default::default();
                }
            }
            "drive.signIn" | "drive.cancel" | "drive.signOut" | "drive.refresh" | "drive.open" | "drive.save" => return self.run_drive(id, p),
            "app.dismissSubjectsHint" => self.ui.maths.subjects_hint_dismissed = true,
            "app.communityLinks" => self.ui.maths.show_community_links = self.flag(p, self.ui.maths.show_community_links),
            "show.teacherKeep" => self.ui.maths.keep_annotations_on_advance = self.flag(p, self.ui.maths.keep_annotations_on_advance),
            _ => return Err(format!("unknown UI command `{id}`")),
        }
        Ok(Value::Null)
    }

    /// Whether the subject pack `id` is switched on.
    pub fn pack_on(&self, id: &str) -> bool {
        self.ui.maths.packs.contains(id)
    }

    /// Teacher Mode is the slide show with the annotation overlay on top. Turning it on starts a
    /// show at the current slide when none is running; turning it off puts the overlay away.
    pub fn set_teacher_mode(&mut self, on: bool) {
        if on && self.show.is_none() {
            let from = self.session.active().map(|d| d.selection.slide).unwrap_or(0);
            self.start_show(from, false);
        }
        if on {
            // The writing from the last time is still on its slides.
            let mut t = self.maths.teacher_stash.take().unwrap_or_default();
            t.timers.clear();
            if let Some(show) = self.show.as_mut() {
                show.teacher = Some(t);
            }
        } else {
            self.stash_teacher();
        }
    }

    /// Puts the Teacher Mode overlay of a show that is ending (or being turned off) aside.
    pub fn stash_teacher(&mut self) {
        if let Some(t) = self.show.as_mut().and_then(|s| s.teacher.take()) {
            self.maths.teacher_stash = Some(t);
        }
    }

    /// The format a Save dialog suggests: PowerPoint (`pptx`) for a new presentation, and the
    /// presentation's own format when it already has a `.deckcraft` or `.pptx` file.
    pub fn default_save_format(&self) -> &'static str {
        let path = self.session.active().and_then(|d| d.path.clone()).unwrap_or_default().to_lowercase();
        if path.ends_with(".deckcraft") { "deckcraft" } else { "pptx" }
    }

    /// Start of `logic`: an update was installed and the teacher chose Restart.
    pub(crate) fn maths_quit_if_asked(&mut self, ctx: &egui::Context) {
        if self.maths.pending_quit {
            self.maths.pending_quit = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Every `logic` pass: collect what Google Drive and the updater finished.
    pub(crate) fn poll_maths(&mut self) {
        self.poll_drive();
        self.poll_updates();
    }

    /// Light or dark follows the computer's setting while the teacher has not chosen one.
    pub(crate) fn follow_system_theme(&mut self, ctx: &egui::Context) {
        if self.ui.maths.follow_system
            && let Some(system) = ctx.input(|i| i.raw.system_theme)
        {
            let want = if system == egui::Theme::Dark { theme::Brightness::Dark } else { theme::Brightness::Light };
            if want != self.ui.brightness {
                self.ui.brightness = want;
                self.restyle = true;
            }
        }
    }
}
