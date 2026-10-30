//! Insert › Question Library: search a skill ("simultaneous equations"), narrow by tier, calculator,
//! marks, strand and year, and insert a past-paper question on a slide.
//!
//! The questions are the teacher's own (see `deckcraft_engine::exam_library`): they stay in the
//! library folder the teacher chose and are never copied into DeckCraft or into a saved deck other
//! than as the one question's picture on the slide it was inserted on.

use std::collections::HashMap;

use deckcraft_engine::exam_library::{self, Item, Query};
use egui::{Align, Layout, Ui, vec2};
use serde_json::json;

use super::Dialog;
use crate::SlideApp;
use crate::theme::{self, Tokens};

/// Most questions listed at once; narrowing the search shows the rest.
const SHOWN: usize = 300;
/// Pictures kept ready; the list is rebuilt from the file again when this is passed.
const MAX_THUMBS: usize = 150;
/// Error text (the same red the other dialogs use).
const DANGER: egui::Color32 = egui::Color32::from_rgb(0xB0, 0x30, 0x30);
const ROW_H: f32 = 124.0;
const THUMB_W: f32 = 300.0;
const THUMB_H: f32 = 108.0;

/// What the dialog remembers while DeckCraft is open: the questions read from the folder, the last
/// search and its results, and the small pictures drawn in the list.
#[derive(Default)]
pub struct LibState {
    /// The folder the questions were read from.
    loaded_dir: String,
    items: Vec<Item>,
    error: String,
    pub(crate) query: Query,
    shown_query: Option<(Query, usize, bool)>,
    /// Names of the question pictures present in the folder.
    pictures: std::collections::HashSet<String>,
    /// List the questions that have no picture yet too (they cannot be inserted).
    include_unpictured: bool,
    results: Vec<usize>,
    total: usize,
    strands: Vec<String>,
    years: Vec<u32>,
    thumbs: HashMap<String, Option<egui::TextureHandle>>,
    status: String,
    first: bool,
}

impl LibState {
    /// Read the folder again when it has changed (or `force`).
    fn ensure_loaded(&mut self, dir: &str, force: bool) {
        if dir.trim().is_empty() {
            *self = LibState { query: std::mem::take(&mut self.query), include_unpictured: self.include_unpictured, ..Default::default() };
            return;
        }
        if !force && self.loaded_dir == dir {
            return;
        }
        let query = std::mem::take(&mut self.query);
        let include_unpictured = self.include_unpictured;
        *self = LibState { query, include_unpictured, loaded_dir: dir.to_string(), first: true, ..Default::default() };
        let crops = format!("{}/derived/exam_crops", dir.trim_end_matches('/'));
        self.pictures = deckcraft_engine::io::list_files(&crops)
            .iter()
            .filter_map(|f| std::path::Path::new(f).file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        match exam_library::load_dir(dir) {
            Ok(items) => {
                self.strands = exam_library::strands(&items);
                self.years = exam_library::years(&items);
                self.items = items;
            }
            Err(e) => self.error = e,
        }
    }

    fn refresh_results(&mut self) {
        let key = (self.query.clone(), self.items.len(), self.include_unpictured);
        if self.shown_query.as_ref() == Some(&key) {
            return;
        }
        let mut all = exam_library::search(&self.items, &self.query, usize::MAX);
        if !self.include_unpictured {
            all.retain(|&i| self.items.get(i).is_some_and(|it| self.pictures.contains(&it.crop_name())));
        }
        self.total = all.len();
        self.results = all.into_iter().take(SHOWN).collect();
        self.shown_query = Some(key);
    }

    /// The small picture of a question, read from the folder the first time it is needed.
    fn thumb(&mut self, ctx: &egui::Context, dir: &str, item: &Item) -> Option<egui::TextureHandle> {
        if let Some(t) = self.thumbs.get(&item.id) {
            return t.clone();
        }
        if self.thumbs.len() >= MAX_THUMBS {
            self.thumbs.clear();
        }
        let tex = deckcraft_engine::io::read_file(&item.crop_path(dir)).ok().and_then(|bytes| deckcraft_render::decode_image(&bytes)).map(|img| {
            let (w, h) = (img.width() as usize, img.height() as usize);
            let small = shrink(img.as_raw(), w, h, (THUMB_W * 2.0) as usize);
            crate::textures::upload(ctx, format!("exam-lib-{}", item.id), small)
        });
        self.thumbs.insert(item.id.clone(), tex.clone());
        tex
    }
}

/// `rgba` (straight alpha, `w` by `h`) averaged down so that it is at most `max_w` wide.
fn shrink(rgba: &[u8], w: usize, h: usize, max_w: usize) -> egui::ColorImage {
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return egui::ColorImage::new([1, 1], vec![egui::Color32::WHITE]);
    }
    let k = w.div_ceil(max_w.max(1)).max(1);
    let (nw, nh) = (w / k, h / k);
    let (nw, nh) = (nw.max(1), nh.max(1));
    let mut px = Vec::with_capacity(nw * nh);
    for y in 0..nh {
        for x in 0..nw {
            let mut acc = [0u32; 3];
            let mut n = 0u32;
            for dy in 0..k {
                for dx in 0..k {
                    let (sx, sy) = (x * k + dx, y * k + dy);
                    if sx < w && sy < h {
                        let i = (sy * w + sx) * 4;
                        // Over white: the question pictures are black on white.
                        let a = u32::from(rgba[i + 3]);
                        for (c, slot) in acc.iter_mut().enumerate() {
                            *slot += (u32::from(rgba[i + c]) * a + 255 * (255 - a)) / 255;
                        }
                        n += 1;
                    }
                }
            }
            let n = n.max(1);
            px.push(egui::Color32::from_rgb((acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8));
        }
    }
    egui::ColorImage::new([nw, nh], px)
}

/// A line of small chips: `Any` plus the choices, one selected.
fn chips<T: PartialEq + Clone>(ui: &mut Ui, label: &str, current: &mut Option<T>, choices: &[(T, &str)]) {
    ui.label(label);
    if ui.selectable_label(current.is_none(), "Any").clicked() {
        *current = None;
    }
    for (v, name) in choices {
        if ui.selectable_label(current.as_ref() == Some(v), *name).clicked() {
            *current = Some(v.clone());
        }
    }
}

pub(super) fn dialog(app: &mut SlideApp, ui: &mut Ui, _d: &mut Dialog) -> bool {
    let t = Tokens::get(ui.ctx());
    ui.set_min_width(940.0);
    ui.set_min_height(580.0);
    let mut state = std::mem::take(&mut app.exam_lib);
    let dir = app.ui.exam_library_dir.clone();
    state.ensure_loaded(&dir, false);

    // The folder.
    let mut reload = false;
    ui.horizontal(|ui| {
        ui.label("Library folder:");
        let shown = if dir.is_empty() { "None chosen".to_string() } else { dir.clone() };
        ui.add(egui::Label::new(egui::RichText::new(shown).color(t.text_dim)).truncate());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if app.services.pick_folder.is_some()
                && ui.button("Choose folder…").clicked()
                && let Some(path) = app.services.pick_folder.as_mut().and_then(|pick| pick("library"))
            {
                app.ui.exam_library_dir = path;
            }
            if !dir.is_empty() && ui.button("Reload").on_hover_text("Read the folder again").clicked() {
                reload = true;
            }
        });
    });
    if reload {
        state.ensure_loaded(&dir, true);
    }
    if dir.is_empty() {
        ui.add_space(10.0);
        ui.label("Choose the folder that holds your questions (it has items/exam inside it and, for the pictures, derived/exam_crops).");
        ui.label(egui::RichText::new("The questions stay in that folder. DeckCraft only reads them.").color(t.text_dim));
        app.exam_lib = state;
        return close_row(ui);
    }
    if !state.error.is_empty() {
        ui.add_space(8.0);
        ui.colored_label(DANGER, &state.error);
        app.exam_lib = state;
        return close_row(ui);
    }
    ui.separator();

    // Search and filters.
    let search = ui.add(
        egui::TextEdit::singleline(&mut state.query.text)
            .hint_text("Search a skill, e.g. simultaneous equations")
            .desired_width(f32::INFINITY)
            .font(theme::font(15.0)),
    );
    if state.first {
        search.request_focus();
        state.first = false;
    }
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        chips(ui, "Tier:", &mut state.query.tier, &[("Foundation".to_string(), "Foundation"), ("Higher".to_string(), "Higher")]);
        ui.add_space(12.0);
        chips(ui, "Paper:", &mut state.query.calculator, &[(true, "Calculator"), (false, "Non-calculator")]);
    });
    ui.horizontal_wrapped(|ui| {
        let mut lo = state.query.min_marks.unwrap_or(0);
        let mut hi = state.query.max_marks.unwrap_or(0);
        ui.label("Marks from");
        let a = ui.add(egui::DragValue::new(&mut lo).range(0..=20).speed(0.2));
        a.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SpinButton, true, "Fewest marks"));
        ui.label("to");
        let b = ui
            .add(egui::DragValue::new(&mut hi).range(0..=20).speed(0.2).custom_formatter(|v, _| if v < 1.0 { "any".into() } else { format!("{v}") }));
        b.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SpinButton, true, "Most marks"));
        state.query.min_marks = (lo > 0).then_some(lo);
        state.query.max_marks = (hi > 0).then_some(hi);
        ui.add_space(12.0);
        let strand_text = state.query.strand.clone().unwrap_or_else(|| "Any strand".into());
        let combo = egui::ComboBox::from_id_salt("exam-lib-strand").selected_text(strand_text).width(210.0).show_ui(ui, |ui| {
            if ui.selectable_label(state.query.strand.is_none(), "Any strand").clicked() {
                state.query.strand = None;
            }
            for s in &state.strands.clone() {
                if ui.selectable_label(state.query.strand.as_deref() == Some(s.as_str()), s).clicked() {
                    state.query.strand = Some(s.clone());
                }
            }
        });
        combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Strand"));
        let year_text = state.query.year.map_or("Any year".to_string(), |y| y.to_string());
        let combo = egui::ComboBox::from_id_salt("exam-lib-year").selected_text(year_text).width(100.0).show_ui(ui, |ui| {
            if ui.selectable_label(state.query.year.is_none(), "Any year").clicked() {
                state.query.year = None;
            }
            for y in &state.years.clone() {
                if ui.selectable_label(state.query.year == Some(*y), y.to_string()).clicked() {
                    state.query.year = Some(*y);
                }
            }
        });
        combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Year sat"));
        ui.checkbox(&mut state.query.past_papers_only, "Past papers only");
        ui.checkbox(&mut state.include_unpictured, "Include questions with no picture yet");
        if ui.small_button("Clear").clicked() {
            state.query = Query::default();
            search.request_focus();
        }
    });
    state.refresh_results();
    let shown = state.results.len();
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(if state.total > shown {
            format!("{} questions (showing the first {shown}; narrow the search to see the rest)", state.total)
        } else {
            format!("{} question{}", state.total, if state.total == 1 { "" } else { "s" })
        })
        .color(t.text_dim),
    );
    ui.separator();

    // The list.
    let mut insert: Option<String> = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(380.0).show_rows(ui, ROW_H, shown, |ui, range| {
        for row in range {
            let Some(item) = state.items.get(state.results[row]).cloned() else { continue };
            ui.horizontal(|ui| {
                ui.set_height(ROW_H - 8.0);
                let (rect, _) = ui.allocate_exact_size(vec2(THUMB_W, THUMB_H), egui::Sense::hover());
                ui.painter().rect_filled(rect, 4.0, egui::Color32::WHITE);
                match state.thumb(ui.ctx(), &dir, &item) {
                    Some(tex) => {
                        let size = tex.size_vec2();
                        let k = (THUMB_W / size.x).min(THUMB_H / size.y).min(1.0);
                        let img = egui::Rect::from_min_size(rect.min, size * k);
                        ui.painter().image(tex.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                    }
                    None => {
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "No picture yet", theme::font(12.0), t.text_faint);
                    }
                }
                ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                ui.vertical(|ui| {
                    let title = if item.topic.is_empty() { item.strand.clone() } else { item.topic.clone() };
                    ui.label(egui::RichText::new(title).font(theme::bold(14.0)));
                    let calc = match item.calculator {
                        Some(true) => "Calculator",
                        Some(false) => "Non-calculator",
                        None => "",
                    };
                    let facts: Vec<String> =
                        [item.tier.clone(), format!("{} mark{}", item.marks, if item.marks == 1 { "" } else { "s" }), calc.to_string()]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect();
                    ui.label(egui::RichText::new(facts.join("  ·  ")).color(t.text_dim));
                    ui.label(egui::RichText::new(item.source_line()).size(11.5).color(t.text_faint));
                    let has_pic = state.thumbs.get(&item.id).is_some_and(Option::is_some);
                    let b = ui.add_enabled(has_pic, egui::Button::new("Insert on a new slide"));
                    b.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, has_pic, format!("Insert {}", item.id)));
                    if b.clicked() {
                        insert = Some(item.id.clone());
                    }
                });
            });
            ui.separator();
        }
    });
    if let Some(id) = insert {
        state.status = match app.run("slide.examLibraryQuestion", json!({"dir": dir, "id": id})) {
            Ok(_) => "Inserted. The answers appear on a click in Slide Show and are in the speaker notes.".to_string(),
            Err(e) => format!("Not inserted: {e}"),
        };
    }
    if !state.status.is_empty() {
        let bad = state.status.starts_with("Not inserted");
        ui.colored_label(if bad { DANGER } else { t.text_dim }, state.status.clone());
    }
    app.exam_lib = state;
    close_row(ui)
}

fn close_row(ui: &mut Ui) -> bool {
    let mut close = false;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        close = ui.button("Close").clicked();
    });
    close
}
