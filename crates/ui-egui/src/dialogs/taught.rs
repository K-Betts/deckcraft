//! Get Ready from the curriculum: the teacher says what the class has just been taught (year
//! group, route, unit, lesson), the app plans the four questions (last lesson, last week, last
//! topic, last year) and shows them to be edited, swapped or given another skill before Insert.
//!
//! The dialog keeps its state in `Dialog::fields` as text: the choices (`year`, `route`, `unit`,
//! `objective`, `level`) and, once questions are made, for each card `i` its heading `h{i}`, the
//! objective it comes from `obj{i}` and `from{i}`, the skills that fit `skills{i}` (JSON list),
//! its level `lvl{i}`, the question as made `q{i}` (JSON) and the teacher's text `text{i}` and
//! answer `answer{i}`.

use std::cell::RefCell;
use std::rc::Rc;

use deckcraft_engine::curriculum::{self, Curriculum, Matcher};
use deckcraft_engine::questions::{Bank, LEVEL_NAMES, Question};
use egui::{Ui, vec2};
use serde_json::{Value, json};

use super::{Dialog, GET_READY_QUESTIONS, check, fresh_seed, question_bank, shows_as};
use crate::SlideApp;
use crate::theme::{self, Tokens};

/// The curriculum and the question bank, read once and kept while neither changes (the dialog is
/// drawn every frame; reading 3,000 recipes each time would be slow).
struct Prepared {
    key: u64,
    curriculum: Result<Curriculum, String>,
    bank: Result<Bank, String>,
    matcher: Matcher,
}

thread_local! {
    static PREPARED: RefCell<Option<Rc<Prepared>>> = const { RefCell::new(None) };
}

fn prepared(app: &SlideApp) -> Rc<Prepared> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    app.ui.curriculum.hash(&mut h);
    (app.ui.my_questions.len(), app.ui.my_templates.len(), &app.ui.removed_questions).hash(&mut h);
    for q in &app.ui.my_questions {
        q.id.hash(&mut h);
    }
    let key = h.finish();
    if let Some(p) = PREPARED.with(|c| c.borrow().clone()).filter(|p| p.key == key) {
        return p;
    }
    let curriculum = if app.ui.curriculum.trim().is_empty() { Err(String::new()) } else { curriculum::parse(&app.ui.curriculum) };
    let bank = question_bank(app);
    let skills = bank.as_ref().map(|b| b.instantiate(0).skills()).unwrap_or_default();
    let p = Rc::new(Prepared { key, curriculum, bank, matcher: Matcher::new(skills) });
    PREPARED.with(|c| *c.borrow_mut() = Some(p.clone()));
    p
}

/// Today as `YYYY-MM-DD` (the clock is read here, in the UI; the engine stays deterministic).
fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    curriculum::date_from_days((secs / 86_400) as i64)
}

/// “w/c 4 Jan” for `2027-01-04`.
fn week_of(date: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (m, d) = (date.get(5..7).and_then(|m| m.parse::<usize>().ok()), date.get(8..10).and_then(|d| d.parse::<u32>().ok()));
    match (m, d) {
        (Some(m @ 1..=12), Some(d)) => format!("w/c {d} {}", MONTHS[m - 1]),
        _ => String::new(),
    }
}

pub(super) fn dialog(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> bool {
    let t = Tokens::get(ui.ctx());
    let p = prepared(app);
    if let Err(e) = &p.bank {
        ui.label(format!("The question bank could not be read: {e}"));
        return ui.button("Close").clicked();
    }
    let cur = match &p.curriculum {
        Ok(c) => c,
        Err(e) => return no_curriculum(app, ui, d, e),
    };

    // What was just taught: year group, route, unit and lesson. Once the questions are made the
    // choices fold into one line, so the cards fit the window.
    let open = !d.fields.contains_key("made") || d.get("editing", "") == "1";
    let years = cur.years();
    let year =
        d.get("year", if years.contains(&app.ui.get_ready_year) { &app.ui.get_ready_year } else { years.first().map(String::as_str).unwrap_or("") });
    if open {
        ui.horizontal_wrapped(|ui| {
            ui.add_sized(vec2(90.0, 24.0), egui::Label::new(egui::RichText::new("Year group").strong()));
            for y in &years {
                let label = format!("Year {}", y.trim_start_matches(['Y', 'y']));
                if ui.selectable_label(&year == y, label).clicked() {
                    d.fields.insert("year".into(), y.clone());
                    for k in ["route", "unit", "objective", "level"] {
                        d.fields.remove(k);
                    }
                }
            }
        });
    }
    let year = d.get("year", &year);
    let routes = cur.routes(&year);
    let route = d.get(
        "route",
        if routes.contains(&app.ui.get_ready_route) { &app.ui.get_ready_route } else { routes.first().map(String::as_str).unwrap_or("") },
    );
    if open && routes.len() > 1 {
        ui.horizontal(|ui| {
            ui.add_sized(vec2(90.0, 24.0), egui::Label::new(egui::RichText::new("Route").strong()));
            for r in &routes {
                if ui.selectable_label(&route == r, r).clicked() {
                    d.fields.insert("route".into(), r.clone());
                    d.fields.remove("level");
                }
            }
        });
    }
    let route = d.get("route", &route);
    app.ui.get_ready_year = year.clone();
    app.ui.get_ready_route = route.clone();
    let Some(course) = cur.course(&year, &route) else {
        ui.label("The curriculum has nothing for this year group.");
        return ui.button("Close").clicked();
    };
    let date = today();
    let this_week = course.unit_on(&date).unwrap_or(0);
    let mut unit: usize = d.get("unit", &this_week.to_string()).parse().unwrap_or(this_week).min(course.units.len().saturating_sub(1));
    let unit_name = |i: usize| {
        let Some(u) = course.units.get(i) else { return String::new() };
        let when = week_of(&u.start);
        let now = if i == this_week { "  (this week)" } else { "" };
        if when.is_empty() { format!("{}{now}", u.title) } else { format!("{when} · {}{now}", u.title) }
    };
    if open {
        ui.horizontal(|ui| {
            ui.add_sized(vec2(90.0, 24.0), egui::Label::new(egui::RichText::new("Unit").strong()));
            egui::ComboBox::from_id_salt("gr-unit").selected_text(unit_name(unit)).width(460.0).height(420.0).show_ui(ui, |ui| {
                for i in 0..course.units.len() {
                    let has = !course.units[i].objectives.is_empty();
                    if ui.add_enabled(has, egui::Button::selectable(unit == i, unit_name(i))).clicked() {
                        unit = i;
                    }
                }
            });
        });
    }
    if d.get("unit", "") != unit.to_string() {
        d.fields.insert("unit".into(), unit.to_string());
        d.fields.remove("objective");
    }
    let Some(u) = course.units.get(unit) else {
        ui.label("This course has no units.");
        return ui.button("Close").clicked();
    };
    let guess = course.objective_on(unit, &date);
    let mut objective: usize = d.get("objective", &guess.to_string()).parse().unwrap_or(0).min(u.objectives.len().saturating_sub(1));
    if open {
        ui.label(egui::RichText::new("Which lesson have you just taught?").strong());
        egui::ScrollArea::vertical().max_height(150.0).id_salt("gr-objectives").show(ui, |ui| {
            for (i, o) in u.objectives.iter().enumerate() {
                ui.radio_value(&mut objective, i, o.text.as_str());
            }
        });
    }
    d.fields.insert("objective".into(), objective.to_string());
    let default_level = if route.eq_ignore_ascii_case("support") { "1" } else { "2" };
    let level: u8 = d.get("level", default_level).parse().unwrap_or(2);
    if open {
        ui.horizontal(|ui| {
            ui.add_sized(vec2(90.0, 24.0), egui::Label::new(egui::RichText::new("Level").strong()));
            for (i, name) in LEVEL_NAMES.iter().enumerate() {
                let v = i as u8 + 1;
                if ui.selectable_label(level == v, *name).clicked() {
                    d.fields.insert("level".into(), v.to_string());
                }
            }
        });
    }
    let level: u8 = d.get("level", default_level).parse().unwrap_or(2);

    // The choices the questions were made from: a new choice means new questions.
    let choice = format!("{year}|{route}|{unit}|{objective}|{level}");
    let made = d.fields.get("made").is_some_and(|m| m == &choice);
    let bank = p.bank.as_ref().ok();
    let mut make = false;
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if !open {
            let taught = u.objectives.get(objective).map(|o| o.text.as_str()).unwrap_or_default();
            let summary = format!("Year {} {route} · {} · just taught: {taught}", year.trim_start_matches(['Y', 'y']), u.title);
            if ui.button("Change…").on_hover_text("Change the year group, unit, lesson or level").clicked() {
                d.fields.insert("editing".into(), "1".into());
            }
            ui.add(egui::Label::new(egui::RichText::new(summary).strong()).truncate());
            return;
        }
        let label = if made { "New set of questions" } else { "Make the questions" };
        let button = egui::Button::new(egui::RichText::new(label).color(t.accent_text)).fill(t.accent).min_size(vec2(170.0, 30.0));
        make = ui.add(button).on_hover_text("Plan four questions from what the class has just been taught").clicked();
        if !made && d.fields.contains_key("made") {
            ui.label(egui::RichText::new("Your choices changed: make the questions again.").size(11.5).color(t.text_dim));
        }
    });
    let again = d.fields.remove("again").is_some() && made;
    if (make || again)
        && let Some(bank) = bank
    {
        make_questions(d, &p.matcher, bank, cur, course, unit, objective, level);
        d.fields.insert("made".into(), choice.clone());
        d.fields.remove("editing");
    }
    // A skill chosen in the skill picker for one card: a new question for it.
    for i in 0..GET_READY_QUESTIONS {
        if let (Some(key), Some(bank)) = (d.fields.remove(&format!("pick{i}")), bank) {
            d.fields.insert(format!("skills{i}"), json!([key]).to_string());
            another(d, bank, i);
        }
    }

    let mut done = false;
    if d.fields.contains_key("made") {
        ui.separator();
        if let Some(open_picker) = preview(ui, d, &p.matcher, bank) {
            let keep: serde_json::Map<String, Value> = d.fields.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
            *d = Dialog::new("chooseSkill");
            d.params = json!({"slot": open_picker, "return": "getReadyTaught", "keep": keep});
            return false;
        }
        ui.add_space(4.0);
        let expand = check(ui, d, "Add a full-screen slide for each question (the Expand buttons)", "expand", true);
        let headings = check(ui, d, "Show the headings on the cards", "headings", true);
        ui.label(
            egui::RichText::new("Answers appear on click in Slide Show and are in the speaker notes. You can still edit the cards on the slide.")
                .size(11.0)
                .color(t.text_dim),
        );
        ui.add_space(6.0);
        let on_starter = app.session.active().is_some_and(|st| st.doc.slides.get(st.selection.slide).is_some_and(|s| s.name == "Get Ready"));
        let ready = (0..GET_READY_QUESTIONS).all(|i| !d.get(&format!("text{i}"), "").trim().is_empty());
        let (mut go, mut replace) = (false, false);
        ui.horizontal(|ui| {
            let button = egui::Button::new(egui::RichText::new("Insert").color(t.accent_text)).fill(t.accent).min_size(vec2(96.0, 32.0));
            go = ui.add_enabled(ready, button).clicked();
            if on_starter {
                replace = ui.add_enabled(ready, egui::Button::new("Replace this slide").min_size(vec2(140.0, 32.0))).clicked();
            }
            if ui.add(egui::Button::new("New set").min_size(vec2(90.0, 32.0))).on_hover_text("Plan four new questions from the same lesson").clicked()
            {
                d.fields.insert("again".into(), "1".into());
            }
            if ui.add(egui::Button::new("Cancel").min_size(vec2(80.0, 32.0))).clicked() {
                done = true;
            }
        });
        if !ready {
            ui.label(egui::RichText::new("Every card needs a question: type one, or choose a skill for it.").size(11.5).color(t.text_dim));
        }
        if go || replace {
            match insert(app, d, expand, headings, replace) {
                Ok(()) => return true,
                Err(e) => {
                    d.fields.insert("error".into(), e);
                }
            }
        }
    } else {
        ui.add_space(4.0);
        if ui.add(egui::Button::new("Cancel").min_size(vec2(80.0, 30.0))).clicked() {
            done = true;
        }
    }
    if let Some(e) = d.fields.get("error").filter(|e| !e.is_empty()) {
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), e);
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let name = if cur.name.is_empty() { "your curriculum".to_string() } else { cur.name.clone() };
        ui.label(egui::RichText::new(format!("Planned from {name}.")).size(11.0).color(t.text_dim));
        if app.services.pick_open.is_some() && ui.small_button("Load another curriculum…").clicked() {
            let msg = load_curriculum(app);
            d.fields.insert("error".into(), msg);
            d.fields.remove("made");
        }
    });
    done
}

/// No curriculum yet (or a broken one): say what it is for and how to load one.
fn no_curriculum(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog, problem: &str) -> bool {
    let t = Tokens::get(ui.ctx());
    ui.label(egui::RichText::new("Load your department's curriculum").font(theme::bold(14.0)));
    ui.label(
        "A curriculum file lists what each year group is taught, unit by unit and week by week. With it, \
         you say what you have just taught and the starter's four questions are planned for you: \
         last lesson, last week, last topic and last year.",
    );
    if !problem.is_empty() {
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), format!("The curriculum you loaded cannot be used: {problem}"));
    }
    ui.add_space(6.0);
    let mut done = false;
    ui.horizontal(|ui| {
        if app.services.pick_open.is_some() {
            let button =
                egui::Button::new(egui::RichText::new("Load curriculum file…").color(t.accent_text)).fill(t.accent).min_size(vec2(170.0, 30.0));
            if ui.add(button).clicked() {
                let msg = load_curriculum(app);
                d.fields.insert("load".into(), msg);
            }
        }
        if ui.add(egui::Button::new("Choose each question instead").min_size(vec2(200.0, 30.0))).clicked() {
            d.fields.insert("mode".into(), "choose".into());
            app.ui.get_ready_mode = "choose".into();
        }
        if ui.add(egui::Button::new("Cancel").min_size(vec2(80.0, 30.0))).clicked() {
            done = true;
        }
    });
    if let Some(m) = d.fields.get("load").filter(|m| !m.is_empty()) {
        ui.label(egui::RichText::new(m).size(12.0).color(egui::Color32::from_rgb(200, 60, 60)));
    }
    egui::CollapsingHeader::new("Or paste a curriculum file's text").id_salt("gr-paste").show(ui, |ui| {
        let mut text = d.get("paste", "");
        egui::ScrollArea::vertical().max_height(120.0).id_salt("gr-paste-box").show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut text).desired_rows(5).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
        });
        d.fields.insert("paste".into(), text.clone());
        if ui.add_enabled(!text.trim().is_empty(), egui::Button::new("Use this curriculum")).clicked() {
            match curriculum::parse(&text) {
                Ok(_) => {
                    app.ui.curriculum = text;
                    d.fields.remove("paste");
                }
                Err(e) => {
                    d.fields.insert("load".into(), format!("Not loaded: {e}"));
                }
            }
        }
    });
    ui.label(
        egui::RichText::new("The file is kept with the app's settings on this computer. Its format is in docs/curriculum-files.md.")
            .size(11.0)
            .color(t.text_dim),
    );
    done
}

/// Asks for a curriculum file and keeps it if it can be used. Returns a problem to show, or
/// nothing when it was loaded.
fn load_curriculum(app: &mut SlideApp) -> String {
    let Some(path) = app.services.pick_open.as_mut().and_then(|pick| pick("curriculum")) else { return String::new() };
    let Some(bytes) = app.services.read.as_mut().map(|read| read(&path)) else { return String::new() };
    let text = match bytes.map_err(|e| e.to_string()).and_then(|b| String::from_utf8(b).map_err(|_| "that file is not text".to_string())) {
        Ok(t) => t,
        Err(e) => return format!("Could not read the file: {e}"),
    };
    match curriculum::parse(&text) {
        Ok(_) => {
            app.ui.curriculum = text;
            String::new()
        }
        Err(e) => format!("Not loaded: {e}"),
    }
}

/// Plans the four cards and makes a question for each.
#[allow(clippy::too_many_arguments)]
fn make_questions(d: &mut Dialog, m: &Matcher, bank: &Bank, cur: &Curriculum, course: &curriculum::Course, unit: usize, objective: usize, level: u8) {
    let seed = fresh_seed();
    let slots = curriculum::plan(cur, course, unit, objective, m, seed);
    let made = bank.instantiate(seed);
    let questions = curriculum::questions_for(&made, &slots, level, seed);
    for (i, (slot, q)) in slots.iter().zip(questions).enumerate().take(GET_READY_QUESTIONS) {
        d.fields.insert(format!("h{i}"), slot.heading.clone());
        d.fields.insert(format!("obj{i}"), slot.objective.clone());
        d.fields.insert(format!("from{i}"), slot.from.clone());
        d.fields.insert(format!("skills{i}"), json!(slot.skills).to_string());
        d.fields.insert(format!("lvl{i}"), level.to_string());
        set_question(d, i, q.as_ref());
    }
    d.fields.remove("error");
}

fn set_question(d: &mut Dialog, i: usize, q: Option<&Question>) {
    match q {
        Some(q) => {
            d.fields.insert(format!("q{i}"), serde_json::to_string(q).unwrap_or_default());
            d.fields.insert(format!("text{i}"), q.text.clone());
            d.fields.insert(format!("answer{i}"), q.answer.clone());
        }
        None => {
            for k in ["q", "text", "answer"] {
                d.fields.insert(format!("{k}{i}"), String::new());
            }
        }
    }
}

fn question(d: &Dialog, i: usize) -> Option<Question> {
    d.fields.get(&format!("q{i}")).and_then(|j| serde_json::from_str(j).ok())
}

/// A different question for card `i`, on the same skills and level, with new numbers.
fn another(d: &mut Dialog, bank: &Bank, i: usize) {
    let skills: Vec<String> = d.fields.get(&format!("skills{i}")).and_then(|j| serde_json::from_str(j).ok()).unwrap_or_default();
    let level: u8 = d.get(&format!("lvl{i}"), "2").parse().unwrap_or(2);
    let seed = fresh_seed();
    let made = bank.instantiate(seed);
    // Not the same question again, nor one another card already has; a skill with one recipe
    // gives the same recipe with new numbers.
    let mut exclude: Vec<String> = (0..GET_READY_QUESTIONS).filter_map(|k| question(d, k).map(|q| q.id)).collect();
    let mut q = curriculum::pick_for(&made, &skills, level, seed, &exclude);
    if q.is_none() {
        exclude.retain(|id| question(d, i).is_none_or(|q| &q.id != id));
        q = curriculum::pick_for(&made, &skills, level, seed, &exclude);
    }
    set_question(d, i, q.as_ref());
}

/// The four cards, each editable. Returns the card whose skill the teacher wants to change.
fn preview(ui: &mut Ui, d: &mut Dialog, m: &Matcher, bank: Option<&Bank>) -> Option<usize> {
    let t = Tokens::get(ui.ctx());
    let mut open_picker = None;
    // Leave room below for the options and buttons.
    let room = (ui.ctx().content_rect().height() - 420.0).clamp(220.0, 640.0);
    egui::ScrollArea::vertical().max_height(room).id_salt("gr-preview").show(ui, |ui| {
        for i in 0..GET_READY_QUESTIONS {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 6)).corner_radius(6.0).stroke(egui::Stroke::new(1.0, t.border)).show(
                ui,
                |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{}", i + 1)).font(theme::bold(15.0)));
                        let mut heading = d.get(&format!("h{i}"), "");
                        ui.add(egui::TextEdit::singleline(&mut heading).desired_width(120.0).font(theme::bold(13.0)));
                        d.fields.insert(format!("h{i}"), heading);
                        let from = d.get(&format!("from{i}"), "");
                        let obj = d.get(&format!("obj{i}"), "");
                        ui.add(egui::Label::new(egui::RichText::new(format!("{from}: {obj}")).size(11.5).color(t.text_dim)).truncate())
                            .on_hover_text(&obj);
                    });
                    let skills: Vec<String> = d.fields.get(&format!("skills{i}")).and_then(|j| serde_json::from_str(j).ok()).unwrap_or_default();
                    let q = question(d, i);
                    let practising = q
                        .as_ref()
                        .map(|q| q.skill_key())
                        .or_else(|| skills.first().cloned())
                        .and_then(|k| m.skills.iter().find(|s| s.key == k))
                        .map(|s| if s.year.is_empty() { s.name.clone() } else { format!("{} ({})", s.name, s.year) });
                    ui.horizontal(|ui| {
                        // The buttons on the right keep their room; a long skill name is cut short.
                        let width = (ui.available_width() - 350.0).max(120.0);
                        let words = match &practising {
                            Some(name) => egui::RichText::new(format!("Practises: {name}")).size(12.0),
                            None => egui::RichText::new("Nothing in the question bank fits this: choose a skill or type a question.")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(190, 110, 0)),
                        };
                        let full = words.text().to_string();
                        ui.allocate_ui_with_layout(vec2(width, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.set_width(width);
                            ui.add(egui::Label::new(words).truncate()).on_hover_text(full);
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Change skill…").on_hover_text("Search the skills and choose another for this card").clicked() {
                                open_picker = Some(i);
                            }
                            if ui
                                .add_enabled(!skills.is_empty(), egui::Button::new("Another question").small())
                                .on_hover_text("A different question on the same skill")
                                .clicked()
                                && let Some(bank) = bank
                            {
                                another(d, bank, i);
                            }
                            let level: u8 = d.get(&format!("lvl{i}"), "2").parse().unwrap_or(2);
                            let mut new_level = level;
                            egui::ComboBox::from_id_salt(format!("gr-lvl{i}"))
                                .width(96.0)
                                .selected_text(LEVEL_NAMES[usize::from(level.clamp(1, 4)) - 1])
                                .show_ui(ui, |ui| {
                                    for (k, name) in LEVEL_NAMES.iter().enumerate() {
                                        ui.selectable_value(&mut new_level, k as u8 + 1, *name);
                                    }
                                });
                            if new_level != level {
                                d.fields.insert(format!("lvl{i}"), new_level.to_string());
                                if let Some(bank) = bank {
                                    another(d, bank, i);
                                }
                            }
                        });
                    });
                    let mut text = d.get(&format!("text{i}"), "");
                    ui.horizontal(|ui| {
                        ui.add_sized(vec2(64.0, 20.0), egui::Label::new("Question"));
                        ui.add(egui::TextEdit::multiline(&mut text).desired_rows(2).desired_width(f32::INFINITY));
                    });
                    d.fields.insert(format!("text{i}"), text.clone());
                    let mut answer = d.get(&format!("answer{i}"), "");
                    ui.horizontal(|ui| {
                        ui.add_sized(vec2(64.0, 20.0), egui::Label::new("Answer"));
                        ui.add(egui::TextEdit::singleline(&mut answer).desired_width(f32::INFINITY));
                    });
                    d.fields.insert(format!("answer{i}"), answer);
                    if text.contains('$') {
                        ui.label(egui::RichText::new(format!("Shows as: {}", shows_as(&text))).size(11.5).color(t.text_dim));
                    }
                    if q.as_ref().is_some_and(|q| q.diagram.is_some() && q.text != text) {
                        ui.label(
                            egui::RichText::new("This question has a figure, drawn from its first numbers: check it still matches your edit.")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(190, 110, 0)),
                        );
                    }
                },
            );
            ui.add_space(4.0);
        }
    });
    open_picker
}

/// Makes the slide from the cards as edited.
fn insert(app: &mut SlideApp, d: &mut Dialog, expand: bool, headings: bool, replace: bool) -> Result<(), String> {
    let mut questions = vec![];
    let mut heads = vec![];
    for i in 0..GET_READY_QUESTIONS {
        let text = d.get(&format!("text{i}"), "").trim().to_string();
        let answer = d.get(&format!("answer{i}"), "").trim().to_string();
        let q = match question(d, i) {
            Some(q) => Question { text: text.clone(), answer, ..q },
            // A question the teacher typed on a card nothing in the bank fitted.
            None => Question { id: format!("typed-{}", i + 1), difficulty: 2, text: text.clone(), answer, ..Default::default() },
        };
        if text.is_empty() {
            return Err(format!("Card {} has no question.", i + 1));
        }
        if text.chars().count() > 600 {
            return Err(format!("Card {}'s question is too long (600 characters at most).", i + 1));
        }
        questions.push(serde_json::to_value(&q).map_err(|e| e.to_string())?);
        heads.push(json!(d.get(&format!("h{i}"), "")));
    }
    let mut params = json!({"questions": questions, "expand": expand, "headings": if headings { Value::Array(heads) } else { json!(false) }});
    if replace {
        params["reroll"] = json!(true);
    }
    app.run("slide.getReady", params).map(|_| ())
}
