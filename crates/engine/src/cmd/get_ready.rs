//! The Get Ready starter: four question cards on a slide, each with a full-screen slide of its own.
//!
//! `slide.getReady` picks questions from the bank (`crate::questions`), draws a 2×2 grid of cards
//! (number, heading, Expand button, question, figure, answer that appears on click) and one
//! full-screen slide per question with a Back button. Figures are drawn by `figures.rs`; maths in
//! the text (`$…$`) goes through `crate::latex`.

use serde_json::{Value, json};

use super::*;
use crate::{Result, Session, questions};

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "slide.getReady",
        "Get Ready Starter",
        ["Insert", "Slides From"],
        None,
        "{slots?: [{topic?: text, difficulty?: 1|2|3, search?: words to find in the topic, subtopic, keywords or question}] (one entry per question, 1-8: each question gets its own topic and level), topic?, difficulty?, count?: 1-8 (default 4) (the same topic and level for all, used when `slots` is absent), seed?: number (a new seed is a reroll), reroll?: bool (replace the current Get Ready slide), bank?: path to a question bank, bankJson?: a question bank as JSON text (default: the built-in one), at?: index} → {index, questions: [ids]}",
        has_doc,
        get_ready
    )]
}

/// The name that marks a slide made by `slide.getReady`, so a reroll can find it again.
pub const GET_READY_NAME: &str = "Get Ready";

/// A retrieval starter: four questions in a 2×2 card grid (matching the Dr Frost / MathsWhiteboard
/// format). Each card shows the question with real equations (LaTeX `$…$` rendered as OMML), a
/// numbered coloured header, and an answer box initially hidden via `shape.visible`. The answers
/// also go in the speaker notes. Cards save as ordinary shapes, so the deck opens in PowerPoint.
fn get_ready(s: &mut Session, p: &Value) -> Result<Value> {
    use crate::questions::{self, Filter};
    let bank = match (str_param(p, "bankJson"), str_param(p, "bank")) {
        (Some(json), _) => questions::parse(json),
        (None, Some(path)) => {
            let bytes = crate::io::read_file(path).map_err(EngineError::Other)?;
            let text = String::from_utf8(bytes).map_err(|_| bad("slide.getReady", "the question bank is not text"))?;
            questions::parse(&text)
        }
        (None, None) => questions::starter(),
    }
    .map_err(|e| bad("slide.getReady", e))?;
    let seed = p.get("seed").and_then(Value::as_u64).unwrap_or(0);
    // Templates become questions with this seed's numbers: a new seed is a reroll with new numbers.
    let bank = bank.instantiate(seed);
    let level = |v: Option<&Value>, what: &str| -> Result<Option<u8>> {
        match v {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v
                .as_u64()
                .and_then(|d| u8::try_from(d).ok())
                .filter(|d| questions::DIFFICULTIES.contains(d))
                .map(Some)
                .ok_or_else(|| bad("slide.getReady", format!("{what} must be 1, 2 or 3"))),
        }
    };
    // Always pick exactly 4 questions for the standard 2×2 retrieval grid.
    let picked: Vec<_> = if let Some(slots) = p.get("slots") {
        let slots = slots.as_array().ok_or_else(|| bad("slide.getReady", "`slots` must be a list"))?;
        if !(1..=8).contains(&slots.len()) {
            return Err(bad("slide.getReady", "`slots` must have 1 to 8 entries"));
        }
        let mut filters = vec![];
        for (i, slot) in slots.iter().enumerate() {
            let topic = slot.get("topic").and_then(Value::as_str).filter(|t| !t.is_empty()).map(String::from);
            let difficulty = level(slot.get("difficulty"), &format!("question {}: `difficulty`", i + 1))?;
            let query = slot.get("search").and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()).map(String::from);
            filters.push(Filter { topic, difficulty, query });
        }
        match bank.assign(&filters, seed) {
            Some(qs) => qs.into_iter().cloned().collect(),
            None => {
                let at = |i: usize| {
                    let f = &filters[i];
                    let what = f.topic.as_deref().unwrap_or("any topic");
                    let lvl = f.difficulty.map(|d| format!(" at level {d}")).unwrap_or_default();
                    format!("{what}{lvl}")
                };
                let empty = (0..filters.len()).find(|&i| bank.candidates(&filters[i]).is_empty());
                let culprit = empty.unwrap_or_else(|| (0..filters.len()).min_by_key(|&i| bank.candidates(&filters[i]).len()).unwrap_or(0));
                return Err(bad("slide.getReady", format!("question {}: no more different questions for {}", culprit + 1, at(culprit))));
            }
        }
    } else {
        let difficulty = level(p.get("difficulty"), "`difficulty`")?;
        let count = usize_param(p, "count").unwrap_or(4);
        if !(1..=8).contains(&count) {
            return Err(bad("slide.getReady", "`count` must be 1 to 8"));
        }
        let filter = Filter {
            topic: str_param(p, "topic").filter(|t| !t.is_empty()).map(String::from),
            difficulty,
            query: str_param(p, "search").map(str::trim).filter(|t| !t.is_empty()).map(String::from),
        };
        let picked: Vec<_> = bank.pick(&filter, seed, count).into_iter().cloned().collect();
        if picked.is_empty() {
            return Err(bad("slide.getReady", "no questions match that topic and difficulty"));
        }
        picked
    };

    // Where it goes: after the current slide, or in place of the current Get Ready slide.
    let (cur, replace) = {
        let st = s.doc()?;
        let cur = st.selection.slide;
        let is_ready = st.doc.slides.get(cur).is_some_and(|sl| sl.name == GET_READY_NAME);
        (cur, bool_param(p, "reroll").unwrap_or(false) && is_ready)
    };
    let at = if replace { cur } else { usize_param(p, "at").unwrap_or(cur + 1) };
    s.execute("slide.new", &json!({"layout": "titleOnly", "title": GET_READY_NAME, "at": at}))?;
    let new_index = {
        let st = s.doc()?;
        st.selection.slide
    };
    s.execute("slide.rename", &json!({"name": GET_READY_NAME, "index": new_index}))?;

    // The title is a slim heading so the four cards get the slide.
    let title = {
        let st = s.doc()?;
        st.doc.slides.get(new_index).and_then(|sl| sl.shapes.iter().find(|x| x.ph_type().is_some_and(|k| k.is_title())).map(|x| x.id.0))
    };
    if let Some(id) = title {
        s.execute("shape.setBounds", &json!({"ids": [id], "x": 24.0, "y": 10.0, "w": 700.0, "h": 46.0}))?;
        edit_runs(s, u64::from(id), |r| r.size = Some(30.0))?;
    }

    // A 2×2 grid of cards: Last lesson / week / topic / year, as on the MathsWhiteboard starter.
    // Each card has its number, question (with equations), figure and an answer that appears on
    // click; its Expand button opens it full screen on a slide of its own, with a Back button.
    let expand = bool_param(p, "expand").unwrap_or(true);
    let headings = bool_param(p, "headings").unwrap_or(true);
    const HEADINGS: [&str; 4] = ["Last lesson", "Last week", "Last topic", "Last year"];
    let cols = 2usize;
    let rows = picked.len().div_ceil(cols).max(1);
    let (margin, top, bottom, gap) = (20.0_f64, 64.0_f64, 16.0_f64, 14.0_f64);
    let card_w = (960.0 - 2.0 * margin - gap * (cols as f64 - 1.0)) / cols as f64;
    let card_h = (540.0 - top - bottom - gap * (rows as f64 - 1.0)) / rows as f64;

    let mut ids = vec![];
    let mut answers = String::new();
    let mut buttons: Vec<(u64, u64)> = vec![];
    for (i, q) in picked.iter().enumerate() {
        let (col, row) = ((i % cols) as f64, (i / cols) as f64);
        let rect = [margin + col * (card_w + gap), top + row * (card_h + gap), card_w, card_h];
        let heading = headings.then(|| HEADINGS.get(i).copied()).flatten();
        let card = draw_card(s, q, i, rect, 1.0, heading, expand)?;
        buttons.push((card.badge, card.expand));
        ids.push(q.id.clone());
        answers.push_str(&format!("{}. {}\n", i + 1, plain_answer(&q.answer)));
    }
    s.execute("slide.notes", &json!({"text": format!("Answers:\n{answers}")}))?;

    // One full-screen slide per question, straight after this one.
    if expand {
        for (i, q) in picked.iter().enumerate() {
            s.execute("slide.new", &json!({"layout": "blank", "at": new_index + 1 + i}))?;
            let here = s.doc()?.selection.slide;
            s.execute("slide.rename", &json!({"name": format!("{GET_READY_NAME} Q{}", i + 1), "index": here}))?;
            draw_card(s, q, i, [20.0, 20.0, 920.0, 500.0], 2.0, headings.then(|| HEADINGS.get(i).copied()).flatten(), false)?;
            let back = draw_shape(s, "roundRect", [786.0, 34.0, 130.0, 36.0], Some("#FFFFFF"), Some(("#6B7280", 1.5)))?;
            s.execute("shape.rename", &json!({"id": back, "name": "Back"}))?;
            set_shape_text(s, back, "Back", 16.0, "#374151", true)?;
            link_to_slide(s, &[back], new_index, "Back to the questions")?;
            s.execute("slide.notes", &json!({"text": format!("Answer: {}", plain_answer(&q.answer))}))?;
        }
        s.execute("slide.go", &json!({"index": new_index}))?;
        for (i, (badge, button)) in buttons.iter().enumerate() {
            link_to_slide(s, &[*badge, *button], new_index + 1 + i, "Open this question full screen")?;
        }
    }

    if replace {
        // The old Get Ready slide, and the full-screen slides that followed it, are replaced.
        let old = new_index + 1 + if expand { picked.len() } else { 0 };
        s.execute("slide.delete", &json!({"index": old}))?;
        loop {
            let more = s.doc()?.doc.slides.get(old).is_some_and(|sl| sl.name.starts_with("Get Ready Q"));
            if !more {
                break;
            }
            s.execute("slide.delete", &json!({"index": old}))?;
        }
        s.execute("slide.go", &json!({"index": new_index}))?;
    }
    Ok(json!({"index": new_index, "questions": ids}))
}

/// Makes clicking the shapes (on the current slide) jump to slide `target` in a show.
fn link_to_slide(s: &mut Session, ids: &[u64], target: usize, tip: &str) -> Result<()> {
    use deckcraft_model::text::{Action, Hyperlink};
    let slide = s.doc()?.doc.slides.get(target).map(|sl| sl.id).ok_or_else(|| bad("slide.getReady", "no such slide"))?;
    s.edit(|doc, sel| {
        let list = crate::shapes_mut(doc, sel).ok_or_else(|| bad("slide.getReady", "no slide"))?;
        for sh in list.iter_mut().filter(|sh| ids.contains(&u64::from(sh.id.0))) {
            sh.click = Some(Hyperlink {
                action: Action::Slide { slide },
                tooltip: tip.to_string(),
                highlight_click: false,
                sound: None,
                stop_sound: false,
                zoom_return: None,
            });
        }
        Ok(())
    })
}

/// Changes the properties of every run in one shape's text.
fn edit_runs(s: &mut Session, id: u64, f: impl Fn(&mut deckcraft_model::text::RunProps)) -> Result<()> {
    s.edit(|doc, sel| {
        let list = crate::shapes_mut(doc, sel).ok_or_else(|| bad("slide.getReady", "no slide"))?;
        for sh in list.iter_mut().filter(|sh| u64::from(sh.id.0) == id) {
            if let Some(t) = sh.text.as_mut() {
                for r in t.paragraphs.iter_mut().flat_map(|p| p.runs.iter_mut()) {
                    f(&mut r.props);
                }
            }
        }
        Ok(())
    })
}

struct Card {
    badge: u64,
    expand: u64,
    #[allow(dead_code)]
    answer: u64,
}

/// One question card in `rect`: number badge and heading, an Expand button, the question, its
/// figure, and the answer (which appears on click). `k` scales text and figure: 1 for a quarter
/// of the slide, 2 for a whole slide.
fn draw_card(s: &mut Session, q: &questions::Question, i: usize, rect: [f64; 4], k: f64, heading: Option<&str>, expand: bool) -> Result<Card> {
    let [cx, cy, card_w, card_h] = rect;
    let kc = k.min(1.4);
    let accents = ["#E8735F", "#4FA37A", "#4A7BD0", "#8A62C9"];
    let tints = ["#FFF3F0", "#F0F9F4", "#EFF4FD", "#F6F1FC"];
    let (accent, tint) = (accents[i % accents.len()], tints[i % tints.len()]);
    let (tab, pad) = (34.0 * kc, 10.0 * kc);
    let name = |what: &str| format!("Question {} {what}", i + 1);

    let card = draw_shape(s, "roundRect", [cx, cy, card_w, card_h], Some(tint), Some((accent, 2.0)))?;
    s.execute("shape.rename", &json!({"id": card, "name": name("card")}))?;
    let badge = draw_shape(s, "ellipse", [cx + pad, cy + pad, tab, tab], Some(accent), None)?;
    s.execute("shape.rename", &json!({"id": badge, "name": name("number")}))?;
    set_shape_text(s, badge, &(i + 1).to_string(), 18.0 * kc, "#FFFFFF", true)?;
    if let Some(h) = heading {
        let id = rich_text_box(s, h, [cx + pad + tab + 6.0, cy + pad, card_w * 0.5, tab], 15.0 * kc, accent)?;
        s.execute("shape.rename", &json!({"id": id, "name": name("heading")}))?;
        edit_runs(s, id, |r| r.bold = Some(true))?;
        s.edit(|doc, sel| {
            let list = crate::shapes_mut(doc, sel).ok_or_else(|| bad("slide.getReady", "no slide"))?;
            for sh in list.iter_mut().filter(|sh| u64::from(sh.id.0) == id) {
                if let Some(t) = sh.text.as_mut() {
                    t.body.anchor = Some(deckcraft_model::text::Anchor::Middle);
                }
            }
            Ok(())
        })?;
    }
    // The Expand button sits at the top right; on the full-screen slide it is not needed.
    let button = if expand {
        let b = draw_shape(s, "roundRect", [cx + card_w - pad - 72.0, cy + pad + 4.0, 72.0, 26.0], Some("#FFFFFF"), Some((accent, 1.5)))?;
        s.execute("shape.rename", &json!({"id": b, "name": name("expand")}))?;
        set_shape_text(s, b, "Expand", 12.0, accent, true)?;
        b
    } else {
        badge
    };

    // The question and its answer fill the left of the card (all of it when there is no figure);
    // a figure takes the right-hand side from the top row to the bottom of the card.
    let figure = q.diagram.as_ref().filter(|d| questions::is_drawable(d));
    let wide = figure.is_some_and(super::figures::is_wide);
    let share = match (figure.is_some(), wide) {
        (false, _) => 0.0,
        (true, true) => 0.55,
        (true, false) => 0.45,
    };
    let col_w = card_w * (1.0 - share) - 2.0 * pad;
    let stacked =
        crate::latex::segments(&q.answer).into_iter().any(|g| matches!(g, crate::latex::Segment::Math(m) if m.contains(")/(") || m.contains("sqrt")));
    let answer_text = plain_answer(&q.answer);
    let a_len = answer_text.chars().count().max(1) as f64;
    let a_h = if stacked { 62.0 * k.min(1.6) } else { 40.0 * k.min(1.8) };
    // As large as fits on one line, up to a comfortable size.
    let a_cap = if stacked { 19.0 * k } else { 22.0 * k };
    let a_size = a_cap.min(((col_w - 16.0) / (a_len * 0.58)).max(10.0 * k));
    let a_rect = [cx + pad, cy + card_h - a_h - pad, col_w, a_h];
    let answer = draw_shape(s, "roundRect", a_rect, Some("#E3F4E8"), Some(("#2E7D4F", 1.5)))?;
    s.execute("shape.rename", &json!({"id": answer, "name": name("answer")}))?;
    set_rich_shape_text(s, answer, &q.answer, if stacked { a_size.min(a_cap) } else { a_size }, "#14532D")?;
    s.execute("animation.set", &json!({"ids": [answer], "effect": "appear"}))?;

    let body_top = cy + pad + tab + 6.0 * kc;
    let text_h = (a_rect[1] - body_top - 6.0).max(20.0);
    // The largest size whose lines (with an allowance for a stacked fraction) fit the space.
    let len = plain_answer(&q.text).chars().count() as f64;
    let stacked_q = q.text.contains("\\frac") || q.text.contains("\\sqrt");
    let size = [26.0, 24.0, 22.0, 20.0, 18.0, 16.0, 15.0, 14.0, 13.0, 12.0]
        .into_iter()
        .map(|c| c * k)
        .find(|c| {
            let per_line = (col_w / (c * 0.6)).max(1.0);
            let lines = (len / per_line).ceil().max(1.0) + if stacked_q { 2.0 } else { 0.0 };
            lines * c * 1.32 <= text_h
        })
        .unwrap_or(12.0 * k);
    let q_id = rich_text_box(s, &q.text, [cx + pad, body_top, col_w, text_h], size, "#1F2937")?;
    s.execute("shape.rename", &json!({"id": q_id, "name": name("text")}))?;
    if let Some(d) = figure {
        let area = [cx + card_w * (1.0 - share), body_top - 4.0, card_w * share - pad, cy + card_h - pad - body_top + 4.0];
        super::figures::draw(s, d, area, k)?;
    }
    Ok(Card { badge, expand: button, answer })
}

/// An answer as plain text for the speaker notes (`$…$` removed).
fn plain_answer(text: &str) -> String {
    crate::latex::plain(text)
}

/// A preset shape with an optional fill and outline (`#RRGGBB`, width in pt).
pub(super) fn draw_shape(s: &mut Session, preset: &str, rect: [f64; 4], fill: Option<&str>, line: Option<(&str, f64)>) -> Result<u64> {
    use super::insert::add_shape;
    use deckcraft_geom::Xfrm;
    use deckcraft_model::{Geom, Shape};
    let shape = Shape {
        xfrm: Some(Xfrm { x: rect[0], y: rect[1], w: rect[2].max(1.0), h: rect[3].max(1.0), ..Default::default() }),
        geom: Geom::preset(preset),
        ..Default::default()
    };
    let id = add_shape(s, "slide.getReady", shape, false)?.0;
    let id = u64::from(id);
    match fill {
        Some(c) => s.execute("shape.fill", &json!({"ids": [id], "color": c}))?,
        None => s.execute("shape.fill", &json!({"ids": [id], "none": true}))?,
    };
    match line {
        Some((c, w)) => s.execute("shape.line", &json!({"ids": [id], "color": c, "width": w}))?,
        None => s.execute("shape.line", &json!({"ids": [id], "none": true}))?,
    };
    Ok(id)
}

/// A straight line from `a` to `b`.
pub(super) fn draw_line(s: &mut Session, a: [f64; 2], b: [f64; 2], color: &str, width: f64) -> Result<u64> {
    use super::insert::add_shape;
    use deckcraft_geom::Xfrm;
    use deckcraft_model::{Geom, Shape};
    let shape = Shape {
        xfrm: Some(Xfrm {
            x: a[0].min(b[0]),
            y: a[1].min(b[1]),
            w: (a[0] - b[0]).abs(),
            h: (a[1] - b[1]).abs(),
            flip_h: b[0] < a[0],
            flip_v: b[1] < a[1],
            ..Default::default()
        }),
        geom: Geom::preset("line"),
        ..Default::default()
    };
    let id = u64::from(add_shape(s, "slide.getReady", shape, false)?.0);
    s.execute("shape.line", &json!({"ids": [id], "color": color, "width": width}))?;
    Ok(id)
}

/// Text on a shape, one run, no maths.
fn set_shape_text(s: &mut Session, id: u64, text: &str, size: f64, color: &str, bold: bool) -> Result<()> {
    use deckcraft_model::text::{Align, Anchor, Run, RunProps};
    let props = RunProps { size: Some(size), bold: Some(bold), fill: Some(parse_fill(color)), ..Default::default() };
    let para = deckcraft_model::text::Paragraph { runs: vec![Run::with(text, props)], ..Default::default() };
    edit_shape_paragraphs(s, id, vec![para], Align::Center, Anchor::Middle)
}

/// Text on a shape with `$…$` drawn as equations.
fn set_rich_shape_text(s: &mut Session, id: u64, text: &str, size: f64, color: &str) -> Result<()> {
    use deckcraft_model::text::{Align, Anchor};
    edit_shape_paragraphs(s, id, rich_paragraphs(text, size, color), Align::Center, Anchor::Middle)
}

fn parse_fill(hex: &str) -> deckcraft_model::style::Fill {
    use deckcraft_model::style::{ColorRef, Fill};
    let rgba = deckcraft_color::Rgba::from_hex(hex).unwrap_or_default();
    Fill::solid(ColorRef::rgb(rgba))
}

fn edit_shape_paragraphs(
    s: &mut Session,
    id: u64,
    paras: Vec<deckcraft_model::text::Paragraph>,
    align: deckcraft_model::text::Align,
    anchor: deckcraft_model::text::Anchor,
) -> Result<()> {
    use deckcraft_model::text::TextBody;
    s.edit(|doc, sel| {
        let list = crate::shapes_mut(doc, sel).ok_or_else(|| bad("slide.getReady", "no slide"))?;
        let sh = list.iter_mut().find(|sh| u64::from(sh.id.0) == id).ok_or_else(|| bad("slide.getReady", "shape vanished"))?;
        let mut paras = paras;
        for para in &mut paras {
            para.props.align.get_or_insert(align);
        }
        let mut body = TextBody { paragraphs: paras, ..Default::default() };
        body.body.wrap = Some(true);
        body.body.anchor = Some(anchor);
        sh.text = Some(body);
        Ok(())
    })
}

/// Paragraphs for a line with `$…$` maths. The editor stacks a fraction or root only when the
/// equation has its paragraph to itself, so those go on their own centred line (as on a printed
/// worksheet); simple maths such as `x^2` stays inside the sentence.
fn rich_paragraphs(text: &str, size: f64, color: &str) -> Vec<deckcraft_model::text::Paragraph> {
    use crate::latex::{Segment, segments};
    use deckcraft_model::text::{Align, Paragraph, Run, RunKind, RunProps};
    let fill = parse_fill(color);
    let mut paras: Vec<Paragraph> = vec![];
    let mut line: Vec<Run> = vec![];
    let flush = |line: &mut Vec<Run>, paras: &mut Vec<Paragraph>| {
        if line.iter().any(|r| !r.text.trim().is_empty()) {
            paras.push(Paragraph { runs: std::mem::take(line), ..Default::default() });
        }
        line.clear();
    };
    for seg in segments(text) {
        match seg {
            Segment::Text(t) => line.push(Run::with(t, RunProps { size: Some(size), fill: Some(fill.clone()), ..Default::default() })),
            Segment::Math(m) => {
                let parsed = crate::equation::parse(&m);
                let stacked = {
                    let omml = crate::equation::to_omml(&parsed);
                    omml.contains("<m:f>") || omml.contains("<m:rad>") || omml.contains("<m:nary>")
                };
                let size = if stacked { size * 1.2 } else { size };
                let run = Run {
                    text: crate::equation::to_plain(&parsed),
                    props: RunProps { font: Some("Cambria Math".into()), size: Some(size), fill: Some(fill.clone()), ..Default::default() },
                    kind: RunKind::Math { omml: crate::equation::to_omml_sized(&parsed, Some(size)) },
                };
                if stacked {
                    flush(&mut line, &mut paras);
                    let mut para = Paragraph { runs: vec![run], ..Default::default() };
                    para.props.align = Some(Align::Center);
                    paras.push(para);
                } else {
                    line.push(run);
                }
            }
        }
    }
    flush(&mut line, &mut paras);
    if paras.is_empty() {
        paras.push(Paragraph::default());
    }
    paras
}

/// A text box holding one wrapped paragraph of text and equations.
pub(super) fn rich_text_box(s: &mut Session, text: &str, rect: [f64; 4], size: f64, color: &str) -> Result<u64> {
    use super::insert::{add_shape, new_text_box};
    use deckcraft_geom::Xfrm;
    let xfrm = Xfrm { x: rect[0], y: rect[1], w: rect[2].max(1.0), h: rect[3].max(1.0), ..Default::default() };
    let mut shape = new_text_box(xfrm, "");
    if let Some(t) = shape.text.as_mut() {
        t.paragraphs = rich_paragraphs(text, size, color);
    }
    Ok(u64::from(add_shape(s, "slide.getReady", shape, false)?.0))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::Session;

    fn ready_texts(s: &Session, i: usize) -> Vec<String> {
        let st = s.doc().expect("doc");
        let slide = st.doc.slides.get(i).expect("slide");
        slide.shapes.iter().filter_map(|sh| sh.text.as_ref().map(|t| t.paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join(" "))).collect()
    }

    #[test]
    fn get_ready_makes_a_titled_slide_of_four_question_cards_with_answers_in_the_notes() {
        let mut s = Session::with_new();
        let r = s.execute("slide.getReady", &json!({"seed": 5})).expect("get ready");
        assert_eq!(r["questions"].as_array().expect("ids").len(), 4);
        let i = r["index"].as_u64().expect("index") as usize;
        let texts = ready_texts(&s, i);
        assert!(texts.iter().any(|t| t == "Get Ready"), "{texts:?}");
        for n in 1..=4 {
            assert!(texts.iter().any(|t| *t == n.to_string()), "number badge {n}: {texts:?}");
        }
        let names: Vec<_> = s.doc().expect("doc").doc.slides[i].shapes.iter().map(|sh| sh.name.clone()).collect();
        for n in 1..=4 {
            for part in ["card", "number", "text", "answer"] {
                assert!(names.contains(&format!("Question {n} {part}")), "{n} {part}: {names:?}");
            }
        }
        let st = s.doc().expect("doc");
        let slide = st.doc.slides.get(i).expect("slide");
        assert_eq!(slide.name, super::GET_READY_NAME);
        assert!(slide.notes.paragraphs.iter().map(|p| p.text()).collect::<String>().contains("Answers:"));
    }

    #[test]
    fn get_ready_filters_by_topic_and_difficulty_and_refuses_when_nothing_matches() {
        let mut s = Session::with_new();
        let r = s.execute("slide.getReady", &json!({"topic": "Algebra", "difficulty": 1, "count": 2})).expect("get ready");
        assert!(r["questions"].as_array().expect("ids").iter().all(|q| q.as_str().is_some_and(|q| q.starts_with("alg-"))));
        assert!(s.execute("slide.getReady", &json!({"topic": "Origami"})).is_err());
        assert!(s.execute("slide.getReady", &json!({"difficulty": 7})).is_err());
        assert!(s.execute("slide.getReady", &json!({"count": 0})).is_err());
    }

    #[test]
    fn each_question_can_have_its_own_topic_and_level() {
        let mut s = Session::with_new();
        let slots = json!([
            {"topic": "Number", "difficulty": 1},
            {"topic": "Algebra", "difficulty": 3},
            {"topic": "Geometry"},
            {"difficulty": 2}
        ]);
        let r = s.execute("slide.getReady", &json!({"slots": slots, "seed": 4})).expect("get ready");
        let ids: Vec<String> = r["questions"].as_array().expect("ids").iter().map(|v| v.as_str().expect("id").to_string()).collect();
        assert_eq!(ids.len(), 4);
        assert!(ids[0].starts_with("num-") && ids[1].starts_with("alg-") && ids[2].starts_with("geo-"), "{ids:?}");
        let bank = crate::questions::starter().expect("bank");
        let find = |id: &str| bank.questions.iter().find(|q| q.id == id).expect("question").clone();
        assert_eq!(find(&ids[0]).difficulty, 1);
        assert_eq!(find(&ids[1]).difficulty, 3);
        assert_eq!(find(&ids[3]).difficulty, 2);
        let mut distinct = ids.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 4, "no question twice on one slide");
    }

    #[test]
    fn two_slots_with_the_same_narrow_filter_never_repeat_and_run_out_with_a_clear_error() {
        let mut s = Session::with_new();
        let same = json!({"topic": "Ratio", "difficulty": 3});
        let enough = s.execute("slide.getReady", &json!({"slots": [same, same], "seed": 1}));
        let err = enough.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(err.contains("question 1") || err.contains("question 2"), "{err}");
        let before = s.doc().expect("doc").doc.slides.len();
        let many = json!({"topic": "Algebra", "difficulty": 1});
        let r = s.execute("slide.getReady", &json!({"slots": [many, many]}));
        assert!(r.is_ok(), "two different algebra level-1 questions exist");
        assert_eq!(s.doc().expect("doc").doc.slides.len(), before + 3, "the slide and a full-screen slide for each question");
        let bad = s.execute("slide.getReady", &json!({"slots": [{"difficulty": 9}]})).err().map(|e| e.to_string()).unwrap_or_default();
        assert!(bad.contains("question 1") && bad.contains("difficulty"), "{bad}");
        assert!(s.execute("slide.getReady", &json!({"slots": []})).is_err());
        assert!(s.execute("slide.getReady", &json!({"slots": "Algebra"})).is_err());
    }

    #[test]
    fn a_question_left_on_any_cannot_starve_one_that_needs_a_specific_topic_and_level() {
        // Regression: with slots [any, Algebra level 3, …] the loose first slot could draw the
        // bank's only Algebra stretch question, and the command then failed for the second.
        let slots = json!([{}, {"topic": "Ratio", "difficulty": 3}, {}, {"topic": "Number", "difficulty": 1}]);
        for seed in 0..300u64 {
            let mut s = Session::with_new();
            let r = s.execute("slide.getReady", &json!({"slots": slots, "seed": seed})).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            assert_eq!(r["questions"][1], "rat-006", "seed {seed}");
        }
    }

    #[test]
    fn every_starter_question_reaches_the_slide_text_unchanged() {
        // Many seeds, so every question gets drawn: the text on the slide must be the bank's text.
        let starter = crate::questions::starter().expect("bank");
        let mut seen = std::collections::HashSet::new();
        for seed in 0..400u64 {
            // Templates make their question from the seed, as the slide will.
            let bank = starter.instantiate(seed);
            let mut s = Session::with_new();
            let r = s.execute("slide.getReady", &json!({"seed": seed, "count": 6})).expect("get ready");
            let i = r["index"].as_u64().expect("index") as usize;
            let texts = ready_texts(&s, i);
            for id in r["questions"].as_array().expect("ids") {
                let q = bank.questions.iter().find(|q| Some(q.id.as_str()) == id.as_str()).expect("question");
                // A fraction or root sits on its own line, so compare without the line breaks.
                let squash = |t: &str| t.split_whitespace().collect::<String>();
                let want = squash(&super::plain_answer(&q.text));
                assert!(texts.iter().any(|t| squash(t) == want), "seed {seed}: {want:?} is not on the slide: {texts:?}");
                seen.insert(q.id.clone());
            }
        }
        let usable = starter.questions.iter().filter(|q| q.usable()).count() + starter.templates.len();
        assert_eq!(seen.len(), usable, "every usable question was drawn at least once");
    }

    #[test]
    fn rerolling_replaces_the_get_ready_slide_and_leaves_other_slides_alone() {
        let mut s = Session::with_new();
        s.execute("slide.new", &json!({"layout": "titleOnly", "title": "Lesson"})).expect("slide");
        s.execute("slide.getReady", &json!({"seed": 1})).expect("first");
        let n = s.doc().expect("doc").doc.slides.len();
        let first = ready_texts(&s, s.doc().expect("doc").selection.slide);
        s.execute("slide.getReady", &json!({"seed": 2, "reroll": true})).expect("reroll");
        assert_eq!(s.doc().expect("doc").doc.slides.len(), n, "replaced, not added");
        let again = ready_texts(&s, s.doc().expect("doc").selection.slide);
        assert_ne!(first, again);
        // Not on a Get Ready slide, a reroll inserts instead of overwriting.
        s.execute("slide.go", &json!({"index": 0})).expect("go");
        s.execute("slide.getReady", &json!({"reroll": true})).expect("insert");
        assert_eq!(s.doc().expect("doc").doc.slides.len(), n + 5, "a slide and four full-screen slides");
    }

    #[test]
    fn a_get_ready_slide_survives_a_pptx_round_trip_with_its_equations() {
        use deckcraft_model::text::RunKind;
        let maths = |s: &Session, i: usize| -> Vec<String> {
            let st = s.doc().expect("doc");
            st.doc.slides[i]
                .shapes
                .iter()
                .filter_map(|sh| sh.text.as_ref())
                .flat_map(|t| t.paragraphs.iter().flat_map(|p| p.runs.iter()))
                .filter_map(|r| match &r.kind {
                    RunKind::Math { omml } => Some(omml.clone()),
                    _ => None,
                })
                .collect()
        };
        let mut s = Session::with_new();
        // Question 1 of this seed has a fraction, so there is real maths to keep.
        s.execute("slide.getReady", &json!({"topic": "Number", "difficulty": 1, "count": 4, "seed": 3})).expect("get ready");
        let at = s.doc().expect("doc").selection.slide;
        let before_texts = ready_texts(&s, at);
        let before_maths = maths(&s, at);
        assert!(!before_maths.is_empty());
        let dir = std::env::temp_dir().join(format!("deckcraft-getready-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("starter.pptx");
        s.execute("file.saveAs", &json!({"path": path.to_str().expect("utf8")})).expect("save");
        let mut o = Session::new();
        o.execute("file.open", &json!({"path": path.to_str().expect("utf8")})).expect("open");
        let n = o.doc().expect("doc").doc.slides.len();
        let idx = (0..n).find(|i| ready_texts(&o, *i).contains(&"Get Ready".to_string())).expect("the Get Ready slide reopened");
        let after_texts = ready_texts(&o, idx);
        let after_maths = maths(&o, idx);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(after_texts.len(), before_texts.len());
        assert_eq!(after_maths.len(), before_maths.len(), "every equation is still an equation");
        assert!(after_maths.iter().any(|m| m.contains("<m:f>")), "fractions are still stacked: {after_maths:?}");
        let st = o.doc().expect("doc");
        let badge = st.doc.slides[idx].shapes.iter().find(|x| x.name == "Question 1 number").expect("badge");
        assert!(badge.click.is_some(), "the badge still opens its full-screen slide");
    }

    #[test]
    fn searching_finds_a_template_and_every_reroll_has_new_numbers() {
        let mut s = Session::with_new();
        let mut texts = std::collections::HashSet::new();
        for seed in 0..12u64 {
            let r = s
                .execute("slide.getReady", &json!({"seed": seed, "slots": [{"search": "both sides"}, {"search": "pythagoras"}, {"search": "straight line"}, {"search": "percentage"}]}))
                .expect("get ready");
            let ids: Vec<&str> = r["questions"].as_array().expect("ids").iter().filter_map(Value::as_str).collect();
            assert_eq!(ids[0], "alg-t-001", "the only question about both sides is the template");
            let bank = crate::questions::starter().expect("bank").instantiate(seed);
            for (id, words) in ids.iter().zip(["both sides", "pythagoras", "straight line", "percentage"]) {
                let q = bank.questions.iter().find(|q| q.id == *id).expect("question");
                assert!(crate::questions::matches_words(q, words), "{id} does not match {words}");
            }
            let i = r["index"].as_u64().expect("index") as usize;
            texts.insert(ready_texts(&s, i).join("|"));
        }
        assert!(texts.len() > 6, "different numbers most times: {}", texts.len());
        let none = s.execute("slide.getReady", &json!({"slots": [{"search": "origami"}]}));
        assert!(none.is_err(), "a search with no match says so");
    }

    #[test]
    fn each_card_expands_to_its_own_slide_and_comes_back() {
        use deckcraft_model::text::Action;
        let mut s = Session::with_new();
        let r = s.execute("slide.getReady", &json!({"seed": 2})).expect("get ready");
        let i = r["index"].as_u64().expect("index") as usize;
        let st = s.doc().expect("doc");
        assert_eq!(st.doc.slides.len(), 6, "the title slide, the starter and four full-screen slides");
        let main = &st.doc.slides[i];
        let target = |sh: &deckcraft_model::Shape| match sh.click.as_ref().map(|c| &c.action) {
            Some(Action::Slide { slide }) => st.doc.slides.iter().position(|x| x.id == *slide),
            _ => None,
        };
        for n in 1..=4usize {
            let badge = main.shapes.iter().find(|x| x.name == format!("Question {n} number")).expect("badge");
            let expand = main.shapes.iter().find(|x| x.name == format!("Question {n} expand")).expect("expand button");
            assert_eq!(target(badge), Some(i + n), "the badge opens question {n}");
            assert_eq!(target(expand), Some(i + n));
            let full = &st.doc.slides[i + n];
            assert_eq!(full.name, format!("Get Ready Q{n}"));
            let back = full.shapes.iter().find(|x| x.name == "Back").expect("back button");
            assert_eq!(target(back), Some(i), "Back returns to the four questions");
            assert!(full.shapes.iter().any(|x| x.name == format!("Question {n} answer")));
        }
        // Without full-screen slides there is just the one.
        let mut t = Session::with_new();
        t.execute("slide.getReady", &json!({"seed": 2, "expand": false})).expect("get ready");
        assert_eq!(t.doc().expect("doc").doc.slides.len(), 2);
    }
}
