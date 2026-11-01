//! Exam Question: one question set out as on an exam paper.

use egui::{Ui, vec2};
use serde_json::{Value, json};

use super::get_ready::{fresh_seed, question_bank};
use crate::SlideApp;
use crate::dialogs::*;
use crate::theme::Tokens;

/// Exam Question: one question set out as on an exam paper, from the question bank's exam-style
/// questions (by skill or topic) or blank to type into.
pub(super) fn exam_question_dialog(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> bool {
    ui.set_min_width(560.0);
    let bank = question_bank(app).ok();
    let listing = bank.as_ref().map(|b| b.instantiate(0));
    let topics = bank.as_ref().map(|b| b.topics()).unwrap_or_default();
    let on_exam = app.session.active().is_some_and(|st| st.doc.slides.get(st.selection.slide).is_some_and(|s| s.name == "Exam Question"));
    let blank = d.get("mode", "bank") == "blank";
    ui.horizontal(|ui| {
        if ui.selectable_label(!blank, "From the question bank").clicked() {
            d.fields.insert("mode".into(), "bank".into());
        }
        if ui.selectable_label(blank, "Blank, to type into").clicked() {
            d.fields.insert("mode".into(), "blank".into());
        }
    });
    ui.add_space(6.0);
    let mut pick_skill = false;
    if blank {
        ui.horizontal(|ui| {
            ui.label("Parts:");
            let n: usize = d.get("parts", "0").parse().unwrap_or(0);
            for (v, label) in [(0, "One part"), (2, "(a) (b)"), (3, "(a) to (c)"), (4, "(a) to (d)")] {
                if ui.selectable_label(n == v, label).clicked() {
                    d.fields.insert("parts".into(), v.to_string());
                }
            }
        });
    } else {
        let skills = listing.as_ref().map(|b| b.skills()).unwrap_or_default();
        let key = d.get("skill", "");
        let chosen = skills.iter().find(|s| s.key == key);
        ui.horizontal(|ui| {
            ui.add_sized(vec2(60.0, 28.0), egui::Label::new("Skill:"));
            let label = match chosen {
                Some(sk) => format!("{}  ·  {}", sk.name, if sk.year.is_empty() { sk.topic.clone() } else { sk.year.clone() }),
                None => "Any skill: choose one…".to_string(),
            };
            let button = egui::Button::new(egui::RichText::new(label).size(13.0)).min_size(vec2(340.0, 28.0)).truncate();
            if ui.add(button).on_hover_text("Search the skills and choose the one the question should test").clicked() {
                pick_skill = true;
            }
            if chosen.is_some() && ui.small_button("✕").on_hover_text("Back to any skill").clicked() {
                d.fields.insert("skill".into(), String::new());
            }
        });
        if let Some(sk) = chosen
            && sk.counts.get(3).copied().unwrap_or(0) == 0
        {
            ui.label(egui::RichText::new("This skill has no exam-style questions yet.").size(11.0).color(egui::Color32::from_rgb(200, 60, 60)));
        }
        if chosen.is_none() {
            ui.horizontal(|ui| {
                ui.add_sized(vec2(60.0, 28.0), egui::Label::new("Topic:"));
                let current = d.get("topic", "");
                egui::ComboBox::from_id_salt("exam-topic")
                    .selected_text(if current.is_empty() { "Any topic" } else { &current })
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(current.is_empty(), "Any topic").clicked() {
                            d.fields.insert("topic".into(), String::new());
                        }
                        for t in &topics {
                            if ui.selectable_label(current == *t, t).clicked() {
                                d.fields.insert("topic".into(), t.clone());
                            }
                        }
                    });
            });
        }
    }
    if pick_skill {
        let keep: serde_json::Map<String, Value> = d.fields.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        *d = Dialog::new("chooseSkill");
        d.params = json!({"slot": 0, "return": "examQuestion", "keep": keep});
        return false;
    }
    field(ui, d, "Question number:", "number", "1");
    ui.label(
        egui::RichText::new(
            "Set out as on an exam paper. Answers sit on the answer lines, hidden until a click in Slide Show, and are in the speaker notes. \
             Marks are the question's total from the bank; a blank question has ( ) to fill in.",
        )
        .size(11.0)
        .color(Tokens::get(ui.ctx()).text_dim),
    );
    let mut go = false;
    let mut reroll = false;
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let t = Tokens::get(ui.ctx());
        go = ui.add(egui::Button::new(egui::RichText::new("Insert").color(t.accent_text)).fill(t.accent).min_size(vec2(96.0, 32.0))).clicked();
        if on_exam {
            reroll = ui.add(egui::Button::new("Reroll this slide").min_size(vec2(140.0, 32.0))).clicked();
        }
        if ui.add(egui::Button::new("Cancel").min_size(vec2(80.0, 32.0))).clicked() {
            d.fields.insert("cancel".into(), "1".into());
        }
    });
    if go || reroll {
        let number: u64 = d.get("number", "1").trim().parse().unwrap_or(0);
        let mut params = json!({"number": number, "seed": fresh_seed()});
        if blank {
            params["blank"] = json!(true);
            params["parts"] = json!(d.get("parts", "0").parse::<u64>().unwrap_or(0));
        } else {
            let (skill, topic) = (d.get("skill", ""), d.get("topic", ""));
            if !skill.is_empty() {
                params["skill"] = json!(skill);
            } else if !topic.is_empty() {
                params["topic"] = json!(topic);
            }
            if !app.ui.maths.my_questions.is_empty() || !app.ui.maths.my_templates.is_empty() || !app.ui.maths.removed_questions.is_empty() {
                match bank {
                    Some(b) => params["bankJson"] = json!(b.to_json()),
                    None => {
                        d.fields.insert("error".into(), "the question bank could not be read".into());
                        return false;
                    }
                }
            }
        }
        if reroll {
            params["reroll"] = json!(true);
        }
        match app.run("slide.examQuestion", params) {
            Ok(_) => return true,
            Err(e) => {
                d.fields.insert("error".into(), e);
            }
        }
    }
    if let Some(e) = d.fields.get("error").cloned().filter(|e| !e.is_empty()) {
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), e);
    }
    d.fields.contains_key("cancel")
}
