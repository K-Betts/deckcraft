//! Get Ready Starter, the skill picker and Manage Questions.

use egui::{Ui, vec2};
use serde_json::{Value, json};

use crate::SlideApp;
use crate::dialogs::{Dialog, buttons, check};
use crate::theme::{self, Tokens};

/// The skill picker: search the small steps (by name, topic, keyword or year), narrow by year
/// group, and choose the one skill a Get Ready question should practise.
pub(super) fn choose_skill_dialog(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> bool {
    use deckcraft_engine::questions::LEVEL_NAMES;
    ui.set_min_width(780.0);
    let slot = d.params.get("slot").and_then(Value::as_u64).unwrap_or(0) as usize;
    let t = Tokens::get(ui.ctx());
    let skills: Vec<_> = question_bank(app).map(|b| b.instantiate(0).skills()).unwrap_or_default();
    let mut years: Vec<String> = vec![];
    for s in &skills {
        let y = if s.year.is_empty() { "General".to_string() } else { s.year.clone() };
        if !years.contains(&y) {
            years.push(y);
        }
    }
    // The Exam Question dialog opens the picker too, and gets its choices back with the skill.
    let for_exam = d.params.get("return").and_then(Value::as_str) == Some("examQuestion");
    let title = if for_exam { "Choose the skill for the exam question".to_string() } else { format!("Choose the skill for question {}", slot + 1) };
    ui.label(egui::RichText::new(title).font(theme::bold(14.0)));
    let mut words = d.get("q", "");
    ui.horizontal(|ui| {
        let edit = egui::TextEdit::singleline(&mut words).hint_text("Search: fractions, solving equations, angles, mean…").desired_width(420.0);
        let resp = ui.add(edit);
        if !d.fields.contains_key("focused") {
            resp.request_focus();
            d.fields.insert("focused".into(), "1".into());
        }
        if ui.button("Clear").clicked() {
            words.clear();
        }
    });
    d.fields.insert("q".into(), words.clone());
    let year = d.get("year", "");
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("Year group:").size(12.0).color(t.text_dim));
        if ui.selectable_label(year.is_empty(), "All").clicked() {
            d.fields.insert("year".into(), String::new());
        }
        for y in &years {
            if ui.selectable_label(&year == y, y).clicked() {
                d.fields.insert("year".into(), y.clone());
            }
        }
    });
    let mut found: Vec<&deckcraft_engine::questions::Skill> = skills
        .iter()
        .filter(|s| match year.as_str() {
            "" => true,
            "General" => s.year.is_empty(),
            y => s.year == y,
        })
        .filter(|s| words.trim().is_empty() || s.matches(&words))
        .collect();
    found.sort_by_key(|s| std::cmp::Reverse(s.relevance(&words)));
    ui.label(egui::RichText::new(format!("{} skill{}", found.len(), if found.len() == 1 { "" } else { "s" })).size(11.0).color(t.text_dim));
    let mut chosen: Option<String> = None;
    egui::ScrollArea::vertical().max_height(360.0).min_scrolled_height(360.0).auto_shrink([false, false]).id_salt("skill-list").show(ui, |ui| {
        for s in found.iter().take(80) {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&s.name).font(theme::bold(13.5)));
                        let place = [
                            if s.year.is_empty() { String::new() } else { format!("Year {}", s.year.trim_start_matches('Y')) },
                            s.term.clone(),
                            s.block.clone(),
                            s.topic.clone(),
                        ];
                        let place: Vec<String> = place.into_iter().filter(|p| !p.is_empty()).collect();
                        ui.label(egui::RichText::new(place.join("  ·  ")).size(11.5).color(t.text_dim));
                        let counts: Vec<String> =
                            LEVEL_NAMES.iter().zip(s.counts).filter(|(_, n)| *n > 0).map(|(name, n)| format!("{name} {n}")).collect();
                        ui.label(egui::RichText::new(counts.join("   ")).size(11.0).color(t.text_dim));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(egui::RichText::new("Choose").color(t.accent_text)).fill(t.accent).min_size(vec2(80.0, 28.0)))
                            .clicked()
                        {
                            chosen = Some(s.key.clone());
                        }
                    });
                });
            });
            ui.separator();
        }
        if found.len() > 80 {
            ui.label(egui::RichText::new("There are more: add a word to the search or pick a year group.").size(11.0).color(t.text_dim));
        }
    });
    let mut back = false;
    ui.horizontal(|ui| {
        if ui.button("Any skill (no choice)").clicked() {
            chosen = Some(String::new());
        }
        if ui.button("Back").clicked() {
            back = true;
        }
    });
    // The Get Ready preview (planned from the curriculum) gets its cards back with the new skill.
    if d.params.get("return").and_then(Value::as_str) == Some("getReadyTaught") && (chosen.is_some() || back) {
        let keep = d.params.get("keep").cloned().unwrap_or_default();
        *d = Dialog::new("getReady");
        if let Some(map) = keep.as_object() {
            for (k, v) in map {
                d.fields.insert(k.clone(), v.as_str().unwrap_or_default().to_string());
            }
        }
        if let Some(key) = chosen.filter(|k| !k.is_empty()) {
            d.fields.insert(format!("pick{slot}"), key);
        }
        return false;
    }
    if for_exam && (chosen.is_some() || back) {
        let keep = d.params.get("keep").cloned().unwrap_or_default();
        *d = Dialog::new("examQuestion");
        if let Some(map) = keep.as_object() {
            for (k, v) in map {
                d.fields.insert(k.clone(), v.as_str().unwrap_or_default().to_string());
            }
        }
        if let Some(key) = chosen {
            d.fields.insert("skill".into(), key);
        }
        return false;
    }
    if let Some(key) = chosen {
        if app.ui.maths.get_ready_skill.len() <= slot {
            app.ui.maths.get_ready_skill.resize(slot + 1, String::new());
        }
        app.ui.maths.get_ready_skill[slot] = key;
        back = true;
    }
    if back {
        *d = Dialog::new("getReady");
    }
    false
}

/// How many questions one Get Ready slide shows.
pub(super) const GET_READY_QUESTIONS: usize = 4;

/// The question bank in use: the built-in questions less any the teacher removed, plus their own.
pub(super) fn question_bank(app: &SlideApp) -> Result<deckcraft_engine::questions::Bank, String> {
    let starter = deckcraft_engine::questions::starter()?;
    Ok(deckcraft_engine::questions::Bank::merged(starter, &app.ui.maths.removed_questions, &app.ui.maths.my_questions, &app.ui.maths.my_templates))
}

/// Manage Questions: look through the bank, add a question of your own, and remove any question
/// (built-in ones are hidden and can be restored; your own are deleted). Changes are kept with the
/// app's other settings and used by the Get Ready starter at once.
pub(super) fn question_bank_dialog(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> bool {
    use deckcraft_engine::questions::{DIFFICULTIES, Question};
    const LEVELS: [&str; 4] = deckcraft_engine::questions::LEVEL_NAMES;
    ui.set_min_width(640.0);
    let Ok(bank) = question_bank(app) else {
        ui.label("The built-in question bank could not be read.");
        let (ok, cancel) = buttons(ui, "Done");
        return ok || cancel;
    };
    let topics = bank.topics();
    let t = Tokens::get(ui.ctx());
    // Each recipe is listed once, with the numbers of one example; it makes new ones every time.
    let listing = bank.instantiate(0);
    ui.label(
        egui::RichText::new(format!(
            "{} questions in your bank ({} of them make new numbers every time)",
            listing.questions.len(),
            bank.templates.len()
        ))
        .font(theme::bold(13.0)),
    );
    ui.horizontal(|ui| {
        let topic = d.get("filter", "");
        craft_ui::ComboBox::from_id_salt("qb-filter")
            .selected_text(if topic.is_empty() { "All topics" } else { topic.as_str() })
            .width(150.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(topic.is_empty(), "All topics").clicked() {
                    d.fields.insert("filter".into(), String::new());
                }
                for tp in &topics {
                    if ui.selectable_label(&topic == tp, tp).clicked() {
                        d.fields.insert("filter".into(), tp.clone());
                    }
                }
            });
        let mut search = d.get("search", "");
        ui.add(egui::TextEdit::singleline(&mut search).hint_text("Search: e.g. both sides, pythagoras").desired_width(260.0));
        d.fields.insert("search".into(), search);
    });
    let topic = d.get("filter", "");
    let search = d.get("search", "");
    let mut remove: Option<String> = None;
    egui::ScrollArea::vertical().max_height(220.0).min_scrolled_height(220.0).auto_shrink([false, false]).id_salt("qb-list").show(ui, |ui| {
        for q in &listing.questions {
            let shown = deckcraft_engine::latex::plain(&q.text);
            if (!topic.is_empty() && !q.topic.eq_ignore_ascii_case(&topic))
                || (!search.trim().is_empty() && !deckcraft_engine::questions::matches_words(q, &search))
            {
                continue;
            }
            let generated = bank.templates.iter().any(|t| t.id == q.id);
            let mine = app.ui.maths.my_questions.iter().any(|m| m.id == q.id) || app.ui.maths.my_templates.iter().any(|m| m.id == q.id);
            ui.horizontal(|ui| {
                let level = LEVELS.get(usize::from(q.difficulty).saturating_sub(1)).copied().unwrap_or("");
                let head = format!(
                    "{} · {}{}{}",
                    q.topic,
                    level,
                    if q.needs_diagram { " · figure" } else { "" },
                    if generated { " · new numbers each time" } else { "" }
                );
                ui.add_sized(vec2(210.0, 20.0), egui::Label::new(egui::RichText::new(head).size(11.0).color(t.text_dim)).truncate());
                let line: String = shown.chars().take(70).collect();
                ui.add_sized(vec2(300.0, 20.0), egui::Label::new(line).truncate());
                if ui.small_button(if mine { "Delete" } else { "Remove" }).clicked() {
                    remove = Some(q.id.clone());
                }
            });
        }
    });
    if let Some(id) = remove {
        if app.ui.maths.my_questions.iter().any(|m| m.id == id) || app.ui.maths.my_templates.iter().any(|m| m.id == id) {
            app.ui.maths.my_questions.retain(|m| m.id != id);
            app.ui.maths.my_templates.retain(|m| m.id != id);
        } else if !app.ui.maths.removed_questions.contains(&id) {
            app.ui.maths.removed_questions.push(id);
        }
    }
    if !app.ui.maths.removed_questions.is_empty() {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} built-in questions removed.", app.ui.maths.removed_questions.len())).size(11.0).color(t.text_dim),
            );
            if ui.small_button("Restore them").clicked() {
                app.ui.maths.removed_questions.clear();
            }
        });
    }
    if let Some(picked) = super::take_text(app, "questions") {
        let msg = import_questions(app, Some(picked));
        d.fields.insert("import".into(), msg);
    }
    if super::can_pick_text(app) {
        ui.horizontal(|ui| {
            if ui.button("Import questions from a file…").on_hover_text("A question file (.json) made for DeckCraft").clicked() {
                let picked = super::pick_text(app, "questions");
                let msg = import_questions(app, picked);
                d.fields.insert("import".into(), msg);
            }
            if let Some(m) = d.fields.get("import").filter(|m| !m.is_empty()) {
                ui.label(egui::RichText::new(m).size(11.0).color(t.text_dim));
            }
        });
    }
    paste_section(app, ui, d);
    ui.separator();
    ui.label(egui::RichText::new("Add a question").font(theme::bold(13.0)));
    ui.horizontal(|ui| {
        ui.add_sized(vec2(70.0, 24.0), egui::Label::new("Topic:"));
        let mut tp = d.get("nq_topic", &topics.first().cloned().unwrap_or_default());
        craft_ui::ComboBox::from_id_salt("qb-topic").selected_text(tp.clone()).width(130.0).show_ui(ui, |ui| {
            for x in &topics {
                if ui.selectable_label(&tp == x, x).clicked() {
                    tp = x.clone();
                }
            }
        });
        ui.label("or new:");
        let mut new_topic = d.get("nq_newtopic", "");
        ui.add(egui::TextEdit::singleline(&mut new_topic).desired_width(110.0));
        d.fields.insert("nq_newtopic".into(), new_topic);
        d.fields.insert("nq_topic".into(), tp);
        let level: u8 = d.get("nq_level", "2").parse().unwrap_or(2);
        for (i, name) in LEVELS.iter().enumerate() {
            let v = i as u8 + 1;
            if ui.selectable_label(level == v, *name).clicked() {
                d.fields.insert("nq_level".into(), v.to_string());
            }
        }
    });
    field_wide(ui, d, "Subtopic:", "nq_sub");
    multiline(ui, d, "Question:", "nq_text", 2);
    multiline(ui, d, "Answer:", "nq_answer", 1);
    ui.label(
        egui::RichText::new("Put maths between dollar signs: $\\frac{3}{4} + \\frac{1}{8}$, $x^2 - 9$, $\\sqrt{50}$, $80\\text{ cm}^3$.")
            .size(11.0)
            .color(t.text_dim),
    );
    let text = d.get("nq_text", "");
    if !text.trim().is_empty() {
        ui.label(egui::RichText::new(format!("Shows as: {}", shows_as(&text))).size(12.0));
    }
    if let Some(e) = d.fields.get("error").cloned().filter(|e| !e.is_empty()) {
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), e);
    }
    let mut done = false;
    ui.horizontal(|ui| {
        let can_add = !text.trim().is_empty() && !d.get("nq_answer", "").trim().is_empty();
        if ui
            .add_enabled(
                can_add,
                egui::Button::new(egui::RichText::new("Add question").color(t.accent_text)).fill(t.accent).min_size(vec2(120.0, 28.0)),
            )
            .clicked()
        {
            let topic = {
                let n = d.get("nq_newtopic", "");
                if n.trim().is_empty() { d.get("nq_topic", "") } else { n.trim().to_string() }
            };
            let q = Question {
                id: bank.new_id(&app.ui.maths.my_questions),
                topic: topic.chars().take(40).collect(),
                subtopic: d.get("nq_sub", "").trim().chars().take(60).collect(),
                keywords: vec![],
                qtype: String::new(),
                source: String::new(),
                year: String::new(),
                difficulty: d.get("nq_level", "2").parse().ok().filter(|l| DIFFICULTIES.contains(l)).unwrap_or(2),
                text: text.trim().to_string(),
                answer: d.get("nq_answer", "").trim().to_string(),
                needs_diagram: false,
                diagram: None,
            };
            if q.topic.trim().is_empty() {
                d.fields.insert("error".into(), "Choose a topic or type a new one.".into());
            } else if q.text.chars().count() > 600 {
                d.fields.insert("error".into(), "The question is too long (600 characters at most).".into());
            } else {
                app.ui.maths.my_questions.push(q);
                for k in ["nq_text", "nq_answer", "nq_sub", "nq_newtopic", "error"] {
                    d.fields.insert(k.into(), String::new());
                }
            }
        }
        if ui.add(egui::Button::new("Done").min_size(vec2(80.0, 28.0))).clicked() {
            done = true;
        }
    });
    done
}

/// Adds what is in a question file the teacher picked (see `pick_text`) to their own bank.
/// Returns what to tell them (nothing when nothing was picked yet): how many were added, or what is wrong with the file.
fn import_questions(app: &mut SlideApp, picked: Option<Result<String, String>>) -> String {
    let Some(picked) = picked else { return String::new() };
    let text = match picked {
        Ok(t) => t,
        Err(e) => return format!("Could not read the file: {e}"),
    };
    match deckcraft_engine::questions::parse_loose(&text) {
        Ok(bank) => add_to_my_bank(app, bank),
        Err(e) => format!("Not added. {e}"),
    }
}

/// Adds `incoming` to the teacher's own bank. Each item gets a fresh id of its own (so an AI that
/// numbered its questions the same as the built-in ones cannot clash), and an item that is already
/// there, the same question or the same recipe, is skipped. Returns what to tell the teacher.
fn add_to_my_bank(app: &mut SlideApp, incoming: deckcraft_engine::questions::Bank) -> String {
    let existing = match question_bank(app) {
        Ok(b) => b,
        Err(e) => return e,
    };
    let used = |app: &SlideApp, id: &str| {
        existing.questions.iter().any(|q| q.id == id)
            || existing.templates.iter().any(|t| t.id == id)
            || app.ui.maths.my_questions.iter().any(|q| q.id == id)
            || app.ui.maths.my_templates.iter().any(|t| t.id == id)
    };
    let fresh = |app: &SlideApp, prefix: &str| (1..).map(|n| format!("{prefix}{n:03}")).find(|id| !used(app, id)).unwrap_or_default();
    let (mut added, mut skipped) = (0, 0);
    for mut q in incoming.questions {
        let same = |o: &deckcraft_engine::questions::Question| o.text == q.text && o.answer == q.answer;
        if existing.questions.iter().any(same) || app.ui.maths.my_questions.iter().any(same) {
            skipped += 1;
            continue;
        }
        q.id = fresh(app, "my-");
        app.ui.maths.my_questions.push(q);
        added += 1;
    }
    for mut t in incoming.templates {
        let same = |o: &deckcraft_engine::template::Template| deckcraft_engine::template::Template { id: t.id.clone(), ..o.clone() } == t;
        if existing.templates.iter().any(same) || app.ui.maths.my_templates.iter().any(same) {
            skipped += 1;
            continue;
        }
        t.id = fresh(app, "my-t-");
        app.ui.maths.my_templates.push(t);
        added += 1;
    }
    match (added, skipped) {
        (0, 0) => "Nothing to add.".into(),
        (n, 0) => format!("Added {n}. They are in your bank now."),
        (0, k) => format!("Nothing new: {k} of these are already in your bank."),
        (n, k) => format!("Added {n}. Skipped {k} that were already in your bank."),
    }
}

/// What a pasted bank will give, in words a teacher can check: for each recipe a few of the
/// questions it makes with their answers, and anything the checks think looks wrong.
fn preview_bank(bank: &deckcraft_engine::questions::Bank) -> String {
    use deckcraft_engine::latex::plain;
    let mut lines = vec![];
    lines.push(format!(
        "Found {} recipe{} (new numbers every time) and {} fixed question{}.",
        bank.templates.len(),
        if bank.templates.len() == 1 { "" } else { "s" },
        bank.questions.len(),
        if bank.questions.len() == 1 { "" } else { "s" }
    ));
    for t in &bank.templates {
        lines.push(format!("▸ {} · {} · level {}", t.subtopic, t.topic, t.difficulty));
        for n in 0..3u64 {
            if let Ok(q) = t.instantiate(n * 7 + 1) {
                lines.push(format!("    {}   →   {}", plain(&q.text), plain(&q.answer)));
            }
        }
        for w in t.lint() {
            lines.push(format!("    ⚠ {w}"));
        }
    }
    for q in bank.questions.iter().take(8) {
        lines.push(format!("▸ {} · {}\n    {}   →   {}", q.subtopic, q.topic, plain(&q.text), plain(&q.answer)));
    }
    if bank.questions.len() > 8 {
        lines.push(format!("… and {} more fixed questions", bank.questions.len() - 8));
    }
    lines.join("\n")
}

/// The “paste an AI's answer” part of Manage Questions: Check shows what the text will make (or
/// what is wrong with it in plain words), and Add puts it in the teacher's bank.
fn paste_section(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) {
    let t = Tokens::get(ui.ctx());
    egui::CollapsingHeader::new(egui::RichText::new("Add questions written by an AI").font(theme::bold(13.0))).id_salt("qb-paste").default_open(false).show(ui, |ui| {
        ui.label(
            egui::RichText::new("Paste the AI's whole answer here (see docs/question-writing-guide.md for what to ask it), press Check, read the examples, then Add.")
                .size(11.0)
                .color(t.text_dim),
        );
        let mut text = d.get("paste", "");
        egui::ScrollArea::vertical().max_height(110.0).id_salt("qb-paste-box").show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut text).desired_rows(5).desired_width(f32::INFINITY).hint_text("Paste here…").font(egui::TextStyle::Monospace));
        });
        if text != d.get("paste", "") {
            // New text: the last check no longer applies.
            d.fields.insert("paste_done".into(), String::new());
        }
        d.fields.insert("paste".into(), text.clone());
        let checked = d.get("paste_checked", "") == text && !text.trim().is_empty();
        let ok = checked && d.get("paste_error", "").is_empty();
        ui.horizontal(|ui| {
            if ui.add_enabled(!text.trim().is_empty(), egui::Button::new("Check")).clicked() {
                match deckcraft_engine::questions::parse_loose(&text) {
                    Ok(bank) => {
                        d.fields.insert("paste_error".into(), String::new());
                        d.fields.insert("paste_preview".into(), preview_bank(&bank));
                    }
                    Err(e) => {
                        d.fields.insert("paste_error".into(), e);
                        d.fields.insert("paste_preview".into(), String::new());
                    }
                }
                d.fields.insert("paste_checked".into(), text.clone());
                d.fields.insert("paste_done".into(), String::new());
            }
            if ui.add_enabled(ok, egui::Button::new(egui::RichText::new("Add these to my bank").color(t.accent_text)).fill(t.accent)).clicked()
                && let Ok(bank) = deckcraft_engine::questions::parse_loose(&text)
            {
                let msg = add_to_my_bank(app, bank);
                d.fields.insert("paste_done".into(), msg);
                d.fields.insert("paste".into(), String::new());
                d.fields.insert("paste_checked".into(), String::new());
            }
            if let Some(m) = d.fields.get("paste_done").filter(|m| !m.is_empty()) {
                ui.label(egui::RichText::new(m).size(12.0).strong());
            } else if !text.trim().is_empty() && !checked {
                ui.label(egui::RichText::new("Press Check first.").size(11.0).color(t.text_dim));
            }
        });
        if checked {
            let err = d.get("paste_error", "");
            if !err.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), err);
            } else {
                egui::ScrollArea::vertical().max_height(200.0).id_salt("qb-paste-preview").show(ui, |ui| {
                    for line in d.get("paste_preview", "").lines() {
                        let rich = if line.trim_start().starts_with('⚠') {
                            egui::RichText::new(line).size(12.0).color(egui::Color32::from_rgb(190, 110, 0))
                        } else if line.starts_with('▸') || line.starts_with("Found") {
                            egui::RichText::new(line).size(12.5).strong()
                        } else {
                            egui::RichText::new(line).size(12.0)
                        };
                        ui.add(egui::Label::new(rich).wrap());
                    }
                });
            }
        }
    });
}

/// A question as plain text for a dialog: `latex::plain`, with its maths-italic letters (𝑥, 𝑦…)
/// made ordinary ones, which the dialog font has (it drew them as boxes).
pub(super) fn shows_as(text: &str) -> String {
    deckcraft_engine::latex::plain(text)
        .chars()
        .map(|c| match c as u32 {
            0x1D434..=0x1D44D => char::from_u32(c as u32 - 0x1D434 + 'A' as u32).unwrap_or(c),
            0x1D44E..=0x1D467 => char::from_u32(c as u32 - 0x1D44E + 'a' as u32).unwrap_or(c),
            0x210E => 'h',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn shows_as_for_tests(text: &str) -> String {
    shows_as(text)
}

fn field_wide(ui: &mut Ui, d: &mut Dialog, label: &str, key: &str) {
    let mut v = d.get(key, "");
    ui.horizontal(|ui| {
        ui.add_sized(vec2(70.0, 24.0), egui::Label::new(label));
        ui.add(egui::TextEdit::singleline(&mut v).desired_width(520.0));
    });
    d.fields.insert(key.into(), v);
}

fn multiline(ui: &mut Ui, d: &mut Dialog, label: &str, key: &str, rows: usize) {
    let mut v = d.get(key, "");
    ui.horizontal(|ui| {
        ui.add_sized(vec2(70.0, 24.0), egui::Label::new(label));
        ui.add(egui::TextEdit::multiline(&mut v).desired_rows(rows).desired_width(520.0));
    });
    d.fields.insert(key.into(), v);
}

/// A seed that differs every time the teacher asks, so Reroll gives different questions. The
/// engine command stays deterministic: the clock is read here, in the UI.
pub(super) fn fresh_seed() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
}

/// Get Ready Starter: each question gets its own topic and level; Insert makes the slide (plain
/// text boxes that save to PowerPoint like any others), Reroll swaps the starter already on
/// this slide for new questions with the same choices. The choices are remembered.
pub(super) fn get_ready_dialog(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> bool {
    const LEVELS: [(u8, &str); 5] = [(0, "Any"), (1, "Support"), (2, "Core"), (3, "Stretch"), (4, "Exam style")];
    ui.set_min_width(760.0);
    // Two ways to fill the four cards: from what the class has just been taught (the curriculum
    // plans them), or each question chosen by hand. Until the teacher picks, a loaded curriculum
    // means the first.
    let default_mode = match app.ui.maths.get_ready_mode.as_str() {
        "" if app.ui.maths.curriculum.is_empty() => "choose",
        "" => "taught",
        m => m,
    }
    .to_string();
    let mode = d.get("mode", &default_mode);
    ui.horizontal(|ui| {
        for (value, label) in [("taught", "From what I just taught"), ("choose", "Choose each question")] {
            if ui.selectable_label(mode == value, egui::RichText::new(label).size(14.0)).clicked() {
                d.fields.insert("mode".into(), value.into());
                app.ui.maths.get_ready_mode = value.into();
            }
        }
    });
    if d.get("mode", &default_mode) == "taught" {
        ui.separator();
        return super::taught::dialog(app, ui, d);
    }
    let bank = question_bank(app).ok();
    let topics = bank.as_ref().map(|b| b.topics()).unwrap_or_default();
    // The bank with each recipe made once, to count what a search finds.
    let listing = bank.map(|b| b.instantiate(0)).unwrap_or_else(|| deckcraft_engine::questions::Bank {
        version: 1,
        note: String::new(),
        questions: vec![],
        templates: vec![],
    });
    let on_starter = app.session.active().is_some_and(|st| st.doc.slides.get(st.selection.slide).is_some_and(|s| s.name == "Get Ready"));
    // Start from what was chosen last time.
    if !d.fields.contains_key("count") {
        for (i, k) in app.ui.maths.get_ready_skill.iter().enumerate() {
            d.fields.insert(format!("skill{i}"), k.clone());
        }
        let saved = app.ui.maths.get_ready_slots.clone();
        d.fields.insert("count".into(), saved.len().clamp(2, 6).to_string());
        for (i, (topic, level)) in saved.iter().enumerate() {
            d.fields.insert(format!("topic{i}"), topic.clone());
            d.fields.insert(format!("level{i}"), level.to_string());
        }
    }
    // One slide always shows four questions, as a 2×2 grid.
    let count = GET_READY_QUESTIONS;
    ui.separator();
    let topic_combo = |ui: &mut Ui, id: String, current: &str| -> Option<String> {
        let mut picked = None;
        craft_ui::ComboBox::from_id_salt(id).selected_text(if current.is_empty() { "Any topic" } else { current }).width(150.0).show_ui(ui, |ui| {
            if ui.selectable_label(current.is_empty(), "Any topic").clicked() {
                picked = Some(String::new());
            }
            for t in &topics {
                if ui.selectable_label(current == t, t).clicked() {
                    picked = Some(t.clone());
                }
            }
        });
        picked
    };
    // One row sets every question, then each can still be changed.
    ui.horizontal(|ui| {
        ui.add_sized(vec2(110.0, 28.0), egui::Label::new(egui::RichText::new("Set all to:").strong()));
        if let Some(t) = topic_combo(ui, "getready-all".into(), "") {
            for i in 0..6 {
                d.fields.insert(format!("topic{i}"), t.clone());
            }
        }
        for (value, label) in LEVELS {
            if ui.button(label).clicked() {
                for i in 0..6 {
                    d.fields.insert(format!("level{i}"), value.to_string());
                }
            }
        }
    });
    ui.add_space(4.0);
    let skills = listing.skills();
    let mut pick_for: Option<usize> = None;
    for i in 0..count {
        ui.horizontal(|ui| {
            ui.add_sized(vec2(90.0, 28.0), egui::Label::new(format!("Question {}", i + 1)));
            let key = d.get(&format!("skill{i}"), "");
            let chosen = skills.iter().find(|s| s.key == key);
            let level: u8 = d.get(&format!("level{i}"), "0").parse().unwrap_or(0);
            // The skill: a button that opens the picker, so a teacher chooses what the question is about.
            let label = match chosen {
                Some(sk) => format!("{}  ·  {}", sk.name, if sk.year.is_empty() { sk.topic.clone() } else { sk.year.clone() }),
                None => "Any skill: choose one…".to_string(),
            };
            let button = egui::Button::new(egui::RichText::new(label).size(13.0)).min_size(vec2(300.0, 28.0)).truncate();
            if ui.add(button).on_hover_text("Search the skills and choose the one this question should practise").clicked() {
                pick_for = Some(i);
            }
            if chosen.is_some() && ui.small_button("✕").on_hover_text("Back to any skill").clicked() {
                d.fields.insert(format!("skill{i}"), String::new());
            }
            if chosen.is_none() {
                let topic = d.get(&format!("topic{i}"), "");
                if let Some(t) = topic_combo(ui, format!("getready-topic{i}"), &topic) {
                    d.fields.insert(format!("topic{i}"), t);
                }
            }
            for (value, label) in LEVELS {
                if ui.selectable_label(level == value, label).clicked() {
                    d.fields.insert(format!("level{i}"), value.to_string());
                }
            }
            // Say when the chosen skill has nothing at the chosen level.
            if let Some(sk) = chosen
                && level > 0
                && sk.counts.get(usize::from(level) - 1).copied().unwrap_or(0) == 0
            {
                ui.label(egui::RichText::new("none at this level").size(11.0).color(egui::Color32::from_rgb(200, 60, 60)));
            }
        });
    }
    if let Some(i) = pick_for {
        // Keep what has been chosen so far, then open the picker for this question.
        app.ui.maths.get_ready_slots =
            (0..count).map(|k| (d.get(&format!("topic{k}"), ""), d.get(&format!("level{k}"), "0").parse().unwrap_or(0))).collect();
        app.ui.maths.get_ready_skill = (0..count).map(|k| d.get(&format!("skill{k}"), "")).collect();
        *d = Dialog::new("chooseSkill");
        d.params = json!({"slot": i});
        return false;
    }
    ui.add_space(4.0);
    let expand = check(ui, d, "Add a full-screen slide for each question (the Expand buttons)", "expand", true);
    let headings = check(ui, d, "Show Last lesson, Last week, Last topic and Last year", "headings", true);
    ui.label(
        egui::RichText::new("Four questions a slide. Answers appear on click in Slide Show and are in the speaker notes.")
            .size(11.0)
            .color(Tokens::get(ui.ctx()).text_dim),
    );
    let slots: Vec<(String, u8)> =
        (0..count).map(|i| (d.get(&format!("topic{i}"), ""), d.get(&format!("level{i}"), "0").parse().unwrap_or(0))).collect();
    let mut go = false;
    let mut reroll = false;
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let t = Tokens::get(ui.ctx());
        go = ui.add(egui::Button::new(egui::RichText::new("Insert").color(t.accent_text)).fill(t.accent).min_size(vec2(96.0, 32.0))).clicked();
        if on_starter {
            reroll = ui.add(egui::Button::new("Reroll this slide").min_size(vec2(140.0, 32.0))).clicked();
        }
        if ui.add(egui::Button::new("Cancel").min_size(vec2(80.0, 32.0))).clicked() {
            go = false;
            reroll = false;
            d.fields.insert("cancel".into(), "1".into());
        }
        if ui.add(egui::Button::new("Manage questions…").min_size(vec2(150.0, 32.0))).clicked() {
            go = false;
            reroll = false;
            d.fields.insert("manage".into(), "1".into());
        }
    });
    if d.fields.contains_key("manage") {
        let _ = app.run("app.dialog", json!({"id": "questionBank"}));
        return true;
    }
    if go || reroll {
        let skill_keys: Vec<String> = (0..count).map(|i| d.get(&format!("skill{i}"), "")).collect();
        let spec: Vec<Value> = slots
            .iter()
            .zip(&skill_keys)
            .map(|((topic, level), skill)| {
                let mut s = json!({});
                if !skill.is_empty() {
                    s["skill"] = json!(skill);
                } else if !topic.is_empty() {
                    s["topic"] = json!(topic);
                }
                if *level > 0 {
                    s["difficulty"] = json!(level);
                }
                s
            })
            .collect();
        let mut params = json!({"seed": fresh_seed(), "slots": spec, "expand": expand, "headings": headings});
        app.ui.maths.get_ready_skill = skill_keys;
        if !app.ui.maths.my_questions.is_empty() || !app.ui.maths.my_templates.is_empty() || !app.ui.maths.removed_questions.is_empty() {
            match question_bank(app) {
                Ok(bank) => params["bankJson"] = json!(bank.to_json()),
                Err(e) => {
                    d.fields.insert("error".into(), e);
                    return false;
                }
            }
        }
        if reroll {
            params["reroll"] = json!(true);
        }
        app.ui.maths.get_ready_slots = slots;
        if let Err(e) = app.run("slide.getReady", params) {
            d.fields.insert("error".into(), e);
            return false;
        }
        return true;
    }
    if let Some(e) = d.fields.get("error").cloned().filter(|e| !e.is_empty()) {
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), e);
    }
    d.fields.contains_key("cancel")
}
