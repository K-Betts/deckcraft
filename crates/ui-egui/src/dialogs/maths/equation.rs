//! The Equation dialog: type in linear form (or use the palette), see the equation drawn as you
//! type, and Insert (or Update the selected one).

use egui::{Align2, Sense, Stroke, Ui, vec2};
use serde_json::{Value, json};

use super::buttons_if;
use crate::SlideApp;
use crate::dialogs::Dialog;
use crate::theme::{self, Tokens};

/// One button on the Equation palette: a picture, its name, and the text it types.
struct Piece {
    glyph: &'static str,
    name: &'static str,
    /// What it types; `□` is a box to fill in.
    typed: &'static str,
    /// Raises or lowers what is already there instead of starting something new.
    script: bool,
}

const fn piece(glyph: &'static str, name: &'static str, typed: &'static str) -> Piece {
    Piece { glyph, name, typed, script: false }
}

const fn script(glyph: &'static str, name: &'static str, typed: &'static str) -> Piece {
    Piece { glyph, name, typed, script: true }
}

const STRUCTURES: [Piece; 11] = [
    piece("a⁄b", "Fraction", "(□)/(□)"),
    script("x²", "Squared", "^2"),
    script("xⁿ", "Power", "^(□)"),
    script("xₙ", "Subscript", "_(□)"),
    piece("√", "Square root", "sqrt(□)"),
    piece("ⁿ√", "Root", "sqrt[□](□)"),
    piece("( )", "Brackets", "(□)"),
    piece("|x|", "Absolute value", "|□|"),
    piece("∑", "Sum", "∑_(□)^(□) □"),
    piece("∫", "Integral", "∫_(□)^(□) □"),
    piece("sin", "Sine", "sin(□)"),
];

const SYMBOL_GROUPS: [(&str, &str); 4] =
    [("Operators", "± × ÷ · ≠ ≈ ≤ ≥ < >"), ("Maths", "° ∞ ∠ ⊥ ∥ ∴ ∵ % ′ ″"), ("Sets", "∈ ∉ ⊂ ∪ ∩ ∅ ∀ ∃"), ("Arrows", "→ ← ⇒ ⇔ ↔")];

const GREEK: &str = "α β γ δ ε θ λ μ π ρ σ τ φ ω Δ Σ Θ Φ Ω";

const EXAMPLES: [(&str, &str); 3] =
    [("Pythagoras", "a^2+b^2=c^2"), ("Quadratic formula", "x=(-b+-sqrt(b^2-4ac))/(2a)"), ("Area of a circle", "A=pi r^2")];

fn char_to_byte(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map_or(s.len(), |(b, _)| b)
}

/// Put `typed` where the cursor is (over the selection, if any), then select the first empty box in
/// it, or leave the cursor after it.
fn type_into(ctx: &egui::Context, id: egui::Id, text: &mut String, p: &Piece) {
    use egui::text::{CCursor, CCursorRange};
    let len = text.chars().count();
    let (a, b) = TextEditRange::of(ctx, id, len);
    let selected: String = text.chars().skip(a).take(b - a).collect();
    let before = text.chars().take(a).last();
    let mut typed = p.typed.to_string();
    if p.script {
        // Raising what is already there needs no base; on its own it gets a box for one.
        let has_base = !selected.is_empty() || before.is_some_and(|c| c.is_alphanumeric() || matches!(c, ')' | ']' | '}' | '|'));
        if !has_base {
            typed.insert(0, deckcraft_engine::equation::PLACEHOLDER);
        }
        typed.insert_str(0, &selected);
    } else if !selected.is_empty() {
        typed = typed.replacen(deckcraft_engine::equation::PLACEHOLDER, &selected, 1);
    }
    let (ba, bb) = (char_to_byte(text, a), char_to_byte(text, b));
    text.replace_range(ba..bb, &typed);
    let end = a + typed.chars().count();
    let range = typed.chars().position(|c| c == deckcraft_engine::equation::PLACEHOLDER).map_or((end, end), |k| (a + k, a + k + 1));
    let mut st = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    st.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(range.0), CCursor::new(range.1))));
    st.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

/// Where the cursor and selection are in the equation field.
struct TextEditRange;

impl TextEditRange {
    fn of(ctx: &egui::Context, id: egui::Id, len: usize) -> (usize, usize) {
        let r = egui::TextEdit::load_state(ctx, id).and_then(|s| s.cursor.char_range());
        match r {
            Some(r) => {
                let (x, y) = (usize::from(r.primary.index).min(len), usize::from(r.secondary.index).min(len));
                (x.min(y), x.max(y))
            }
            None => (len, len),
        }
    }
}

/// Tab moves to the next empty box, as it does in PowerPoint's equation editor.
fn next_box(ctx: &egui::Context, id: egui::Id, text: &str) {
    use egui::text::{CCursor, CCursorRange};
    let boxes: Vec<usize> = text.chars().enumerate().filter(|(_, c)| *c == deckcraft_engine::equation::PLACEHOLDER).map(|(k, _)| k).collect();
    let (_, hi) = TextEditRange::of(ctx, id, text.chars().count());
    if let Some(&k) = boxes.iter().find(|&&k| k >= hi).or(boxes.first()) {
        let mut st = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        st.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(k), CCursor::new(k + 1))));
        st.store(ctx, id);
    }
}

pub(super) fn equation_dialog(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> bool {
    use deckcraft_engine::equation as eq;
    let t = Tokens::get(ui.ctx());
    ui.set_min_width(560.0);
    let id = egui::Id::unique("equation-text");
    let first = !d.fields.contains_key("started");
    d.fields.insert("started".into(), "1".into());
    if first
        && let Ok(v) = app.session.execute("equation.source", &json!({}))
        && let Some(lin) = v.get("linear").and_then(Value::as_str)
    {
        // An equation is selected: open on it, and Update changes it in place.
        d.fields.insert("eq".into(), lin.to_string());
        d.fields.insert("editing".into(), "1".into());
    }
    let editing = d.fields.contains_key("editing");
    let mut text = d.get("eq", "");

    // Tab jumps to the next empty box. It has to be taken before the field sees it.
    let text_has_boxes = text.contains(eq::PLACEHOLDER);
    if text_has_boxes && ui.memory(|m| m.has_focus(id)) && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
        next_box(ui.ctx(), id, &text);
    }

    // What the equation will look like.
    let parsed = eq::parse(&text);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), 112.0), Sense::hover());
    ui.painter().rect(area, 8.0, t.field_bg, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    if text.trim().is_empty() {
        ui.painter().text(area.center(), Align2::CENTER_CENTER, "Your equation appears here", theme::font(15.0), t.text_faint);
    } else {
        crate::maths::equation_view::paint(&ui.painter().with_clip_rect(area.shrink(2.0)), area, &parsed, t.text, t.text_faint);
    }

    // Typing.
    ui.add_space(8.0);
    let field = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id(id)
            .lock_focus(text_has_boxes)
            .hint_text("Type here, e.g. 3/4 or x^2 or sqrt(x), or use the buttons below")
            .desired_width(f32::INFINITY)
            .font(theme::font(17.0)),
    );
    if first {
        field.request_focus();
    }

    // The buttons.
    ui.add_space(8.0);
    let tab = d.get("tab", "Structures");
    ui.horizontal(|ui| {
        for name in ["Structures", "Symbols", "Greek"] {
            if ui.selectable_label(tab == name, egui::RichText::new(name).font(theme::bold(13.0))).clicked() {
                d.fields.insert("tab".into(), name.into());
            }
        }
    });
    ui.separator();
    let tab = d.get("tab", "Structures");
    let ctx = ui.ctx().clone();
    match tab.as_str() {
        "Structures" => {
            ui.horizontal_wrapped(|ui| {
                for p in &STRUCTURES {
                    ui.vertical(|ui| {
                        ui.set_width(62.0);
                        let b = egui::Button::new(egui::RichText::new(p.glyph).size(20.0)).min_size(vec2(60.0, 38.0));
                        if ui.add(b).on_hover_text(p.name).clicked() {
                            type_into(&ctx, id, &mut text, p);
                        }
                        ui.label(egui::RichText::new(p.name).size(10.5).color(t.text_dim));
                    });
                }
            });
        }
        _ => {
            let groups: Vec<(&str, &str)> = if tab == "Greek" { vec![("Greek letters", GREEK)] } else { SYMBOL_GROUPS.to_vec() };
            for (name, chars) in groups {
                ui.label(egui::RichText::new(name).size(11.5).color(t.text_dim));
                ui.horizontal_wrapped(|ui| {
                    for c in chars.split(' ') {
                        let b = egui::Button::new(egui::RichText::new(c).size(17.0)).min_size(vec2(34.0, 32.0));
                        if ui.add(b).clicked() {
                            type_into(&ctx, id, &mut text, &Piece { glyph: "", name: "", typed: c, script: false });
                        }
                    }
                });
            }
        }
    }

    // A nudge for whoever has not typed anything, or has boxes left.
    let blank = text.trim().is_empty();
    let unfilled = eq::has_empty(&eq::parse(&text));
    ui.add_space(6.0);
    if blank {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Try one:").size(12.0).color(t.text_dim));
            for (name, typed) in EXAMPLES {
                if ui.small_button(name).clicked() {
                    text = typed.to_string();
                    ctx.memory_mut(|m| m.request_focus(id));
                }
            }
        });
    } else if unfilled {
        ui.label(egui::RichText::new("Click each empty box and type in it (Tab moves to the next one).").size(12.0).color(t.text_dim));
    } else {
        ui.label(
            egui::RichText::new(if editing { "Change it, then click Update." } else { "Looks right? Click Insert to put it on the slide." })
                .size(12.0)
                .color(t.text_dim),
        );
    }
    d.fields.insert("eq".into(), text.clone());

    let (ok, cancel) = buttons_if(ui, if editing { "Update" } else { "Insert" }, !blank && !unfilled);
    if ok {
        let _ = app.run("insert.equation", json!({"linear": text, "replace": editing}));
    }
    ok || cancel
}
