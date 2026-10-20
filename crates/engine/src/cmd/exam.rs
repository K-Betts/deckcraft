//! Exam Question: one GCSE-style question on a slide, set out as Edexcel papers do.
//!
//! `slide.examQuestion` draws the question number in a box, the question (and its parts (a), (b), …),
//! an answer line under each part, and the total marks at the bottom right: "(Total for Question 1
//! is 4 marks)". A one-part question also shows its marks at the end of its answer line, "(4)". The
//! answers sit on the answer lines, hidden until a click in Slide Show, and are in the speaker
//! notes. It is filled from the bank's exam-style questions (new numbers each time) or left blank to
//! type into. Everything is ordinary shapes, so the slide opens the same in PowerPoint.

use serde_json::{Value, json};

use super::get_ready::{
    bank_from, draw_shape, edit_runs, plain_answer, plain_layout_name, rich_text_box, set_rich_shape_text, set_shape_text, slide_is_empty,
};
use super::*;
use crate::{Result, Session};

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "slide.examQuestion",
        "Exam Question",
        ["Insert", "Slides From"],
        None,
        "{skill?: a skill key (see the skill picker), year?: Y7.., topic?, search?: words, seed?: number (a new seed is new numbers), blank?: bool (an empty question to type into), parts?: 0-6 (blank only: parts (a), (b), …; 0 = one part), number?: 1-99 (default 1), reroll?: bool (replace the current Exam Question slide), bank?: path, bankJson?: text, at?: index} → {index, question: id | null, marks: n | null}",
        has_doc,
        exam_question
    )]
}

/// The name that marks a slide made by `slide.examQuestion`, so a reroll can find it again.
pub const EXAM_NAME: &str = "Exam Question";

const INK: &str = "#111827";
const DIM: &str = "#6B7280";
/// The level of the bank's exam-style questions.
const EXAM_LEVEL: u8 = 4;
const MAX_PARTS: usize = 6;

/// A question split for the page: what comes before the parts, the parts, and the total marks.
#[derive(Debug, PartialEq)]
struct Layout {
    stem: String,
    parts: Vec<String>,
    marks: Option<u32>,
}

/// "… (4 marks)" at the end of a question: the text without it, and the number.
fn take_marks(text: &str) -> (String, Option<u32>) {
    let t = text.trim_end();
    if let Some(open) = t.rfind('(')
        && t.ends_with(')')
    {
        let inside = &t[open + 1..t.len() - 1];
        let (n, word) = inside.split_once(' ').unwrap_or((inside, ""));
        if matches!(word, "mark" | "marks")
            && let Ok(n) = n.parse::<u32>()
        {
            return (t[..open].trim_end().to_string(), Some(n));
        }
    }
    (t.to_string(), None)
}

/// "stem (a) one (b) two" as the stem and its parts. Parts must run (a), (b), (c)… in order;
/// anything else is one part.
fn split_parts(text: &str) -> (String, Vec<String>) {
    let mut cuts: Vec<(usize, usize)> = vec![];
    let mut from = 0;
    for letter in ('a'..='z').take(MAX_PARTS) {
        let mark = format!("({letter})");
        match text[from..].find(&mark) {
            Some(k) => {
                cuts.push((from + k, mark.len()));
                from += k + mark.len();
            }
            None => break,
        }
    }
    if cuts.len() < 2 {
        return (text.trim().to_string(), vec![]);
    }
    let stem = text[..cuts[0].0].trim().to_string();
    let parts = cuts
        .iter()
        .enumerate()
        .map(|(i, &(at, len))| {
            let end = cuts.get(i + 1).map_or(text.len(), |c| c.0);
            text[at + len..end].trim().to_string()
        })
        .collect();
    (stem, parts)
}

fn lay_out(text: &str) -> Layout {
    let (text, marks) = take_marks(text);
    let (stem, parts) = split_parts(&text);
    Layout { stem, parts, marks }
}

/// Part letter `i`: a, b, c…
fn letter(i: usize) -> char {
    char::from(b'a' + (i % 26) as u8)
}

fn exam_question(s: &mut Session, p: &Value) -> Result<Value> {
    use crate::questions::Filter;
    let number = usize_param(p, "number").unwrap_or(1);
    if !(1..=99).contains(&number) {
        return Err(bad("slide.examQuestion", "`number` must be 1 to 99"));
    }
    let blank = bool_param(p, "blank").unwrap_or(false);
    let text_param = |k: &str| str_param(p, k).map(str::trim).filter(|t| !t.is_empty()).map(String::from);

    // What goes on the slide: a question from the bank, or empty boxes with prompts.
    let (layout, answers, id) = if blank {
        let n = usize_param(p, "parts").unwrap_or(0);
        if n > MAX_PARTS {
            return Err(bad("slide.examQuestion", format!("`parts` must be 0 to {MAX_PARTS}")));
        }
        let parts = (0..n).map(|i| format!("Type part ({}) here.", letter(i))).collect();
        let answers = if n == 0 { vec!["Answer".to_string()] } else { (0..n).map(|i| format!("Answer ({})", letter(i))).collect() };
        (Layout { stem: "Type the question here.".into(), parts, marks: None }, answers, None)
    } else {
        let seed = p.get("seed").and_then(Value::as_u64).unwrap_or(0);
        let bank = bank_from(p, "slide.examQuestion")?.instantiate(seed);
        let filter = Filter {
            topic: text_param("topic"),
            difficulty: Some(EXAM_LEVEL),
            skill: text_param("skill"),
            year: text_param("year"),
            query: text_param("search"),
        };
        let q = bank
            .pick(&filter, seed, 1)
            .into_iter()
            .next()
            .cloned()
            .ok_or_else(|| bad("slide.examQuestion", "no exam-style question for that skill or topic"))?;
        let layout = lay_out(&q.text);
        // The answer in the same parts as the question, when it has them; else as one.
        let (_, a_parts) = split_parts(&q.answer);
        let answers = if !layout.parts.is_empty() && a_parts.len() == layout.parts.len() { a_parts } else { vec![q.answer.trim().to_string()] };
        (layout, answers, Some(q.id))
    };

    // Where it goes: after the current slide, or in place of the current Exam Question slide. A deck
    // with its own "Exam Question" layout gets that layout, and an empty slide on it is filled.
    let (cur, replace, layout_name) = {
        let st = s.doc()?;
        let cur = st.selection.slide;
        let slide = st.doc.slides.get(cur);
        let master = slide.and_then(|sl| st.doc.masters.iter().position(|m| m.layout(sl.layout).is_some())).unwrap_or(0);
        let own = exam_layout(&st.doc, master).map(|l| (l.name.clone(), l.id));
        let fill_in = matches!((&own, slide), (Some((_, id)), Some(sl)) if sl.layout == *id && slide_is_empty(sl));
        let reroll = bool_param(p, "reroll").unwrap_or(false) && slide.is_some_and(|sl| sl.name == EXAM_NAME);
        (cur, reroll || fill_in, own.map(|o| o.0).unwrap_or_else(|| plain_layout_name(&st.doc, master)))
    };
    let at = if replace { cur } else { usize_param(p, "at").unwrap_or(cur + 1) };
    s.execute("slide.new", &json!({"layout": layout_name, "at": at}))?;
    let index = s.doc()?.selection.slide;
    s.execute("slide.rename", &json!({"name": EXAM_NAME, "index": index}))?;
    let (w, h) = {
        let d = &s.doc()?.doc;
        (d.slide_size.width.max(320.0), d.slide_size.height.max(240.0))
    };
    draw_question(s, &layout, &answers, number, w, h, blank, id.as_deref(), p)?;

    // Speaker notes: the answers and the marks.
    let mut notes = String::from("Answers:\n");
    for (i, a) in answers.iter().enumerate() {
        if answers.len() > 1 {
            notes.push_str(&format!("({}) ", letter(i)));
        }
        notes.push_str(&plain_answer(a));
        notes.push('\n');
    }
    if let Some(m) = layout.marks {
        notes.push_str(&format!("Total: {m} mark{}\n", if m == 1 { "" } else { "s" }));
    }
    if !blank {
        s.execute("slide.notes", &json!({"text": notes}))?;
    }
    if replace {
        s.execute("slide.delete", &json!({"index": index + 1}))?;
        s.execute("slide.go", &json!({"index": index}))?;
    }
    Ok(json!({"index": index, "question": id, "marks": layout.marks}))
}

/// The deck's own "Exam Question" layout, if its master has one.
fn exam_layout(doc: &deckcraft_model::Presentation, master: usize) -> Option<&deckcraft_model::Layout> {
    let norm = |x: &str| x.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_ascii_lowercase();
    doc.masters.get(master).or(doc.masters.first())?.layouts.iter().find(|l| norm(&l.name).starts_with("examquestion"))
}

/// Lines `text` takes at `size` points in a column `width` wide (a rough measure, with room for a
/// stacked fraction or root).
fn lines_at(text: &str, size: f64, width: f64) -> f64 {
    let len = plain_answer(text).chars().count().max(1) as f64;
    let per_line = (width / (size * 0.52)).max(1.0);
    let stacked = text.contains("\\frac") || text.contains("\\sqrt");
    (len / per_line).ceil().max(1.0) + if stacked { 1.5 } else { 0.0 }
}

#[allow(clippy::too_many_arguments)]
fn draw_question(
    s: &mut Session,
    q: &Layout,
    answers: &[String],
    number: usize,
    w: f64,
    h: f64,
    blank: bool,
    id: Option<&str>,
    p: &Value,
) -> Result<()> {
    let margin = 32.0;
    let num_box = 46.0;
    let x0 = margin + num_box + 18.0;
    let footer = 40.0;
    let bottom = h - margin - footer;
    let top = margin;

    // The figure, when the question has one, takes the right of the slide.
    let figure = if blank {
        None
    } else {
        let bank = bank_from(p, "slide.examQuestion")?.instantiate(p.get("seed").and_then(Value::as_u64).unwrap_or(0));
        bank.questions.iter().find(|x| Some(x.id.as_str()) == id).and_then(|x| x.diagram.clone()).filter(crate::questions::is_drawable)
    };
    let share = match &figure {
        Some(d) if super::figures::is_wide(d) => 0.5,
        Some(_) => 0.4,
        None => 0.0,
    };
    let full_w = w - x0 - margin;
    let text_w = full_w * (1.0 - share) - if share > 0.0 { 16.0 } else { 0.0 };

    // Question number: bold, in a box in the left margin, as on the paper.
    let badge = draw_shape(s, "rect", [margin, top, num_box, num_box], Some("#FFFFFF"), Some((INK, 2.0)))?;
    s.execute("shape.rename", &json!({"id": badge, "name": "Question number"}))?;
    set_shape_text(s, badge, &number.to_string(), 24.0, INK, true)?;

    // Rows: the stem, then each part with its answer line (one answer line when there are no parts).
    let one_part = q.parts.is_empty();
    // A stacked fraction or root in an answer needs a taller answer box.
    let stacked = answers.iter().any(|a| a.contains("\\frac") || a.contains("\\sqrt"));
    let answer_h = if stacked { 66.0 } else { 44.0 };
    let gap = 10.0;
    let rows: Vec<&str> = if one_part { vec![] } else { q.parts.iter().map(String::as_str).collect() };
    // The part letters' column fits "(m)" at the largest text size.
    let letter_w = 64.0;
    let part_w = text_w - letter_w;
    // The largest text size at which everything fits above the footer.
    let size = [26.0, 24.0, 22.0, 20.0, 18.0, 16.0, 15.0, 14.0]
        .into_iter()
        .find(|&sz| {
            let line = sz * 1.3;
            let stem = if q.stem.is_empty() { 0.0 } else { lines_at(&q.stem, sz, text_w) * line + gap };
            let parts: f64 = rows.iter().map(|r| lines_at(r, sz, part_w) * line + 4.0 + answer_h + gap).sum();
            let single = if one_part { answer_h + gap } else { 0.0 };
            top + stem + parts + single <= bottom
        })
        .unwrap_or(14.0);
    let line = size * 1.3;
    let mut y = top;
    // A question that is all parts starts with (a) beside the number.
    if !q.stem.is_empty() {
        let stem_h = lines_at(&q.stem, size, text_w) * line;
        let stem = rich_text_box(s, &q.stem, [x0, y, text_w, stem_h], size, INK)?;
        s.execute("shape.rename", &json!({"id": stem, "name": "Question"}))?;
        y += stem_h + gap;
    }

    let mut hidden = vec![];
    let mut answer_line = |s: &mut Session, y: f64, i: usize, marks: Option<&str>| -> Result<()> {
        let marks_w = if marks.is_some() { 56.0 } else { 0.0 };
        // The line ends at the right of the question column and never starts left of it.
        let line_w = (text_w - marks_w).min(460.0);
        let x1 = x0 + text_w - marks_w;
        let lx = x1 - line_w;
        let dots = rich_text_box(s, &"·".repeat(((line_w - 8.0) / 6.0).max(4.0) as usize), [lx, y, line_w, answer_h], 18.0, DIM)?;
        s.execute("shape.rename", &json!({"id": dots, "name": format!("Answer line{}", tag(one_part, i))}))?;
        set_anchor_align(s, dots, "bottom", "right")?;
        if let Some(m) = marks {
            let mk = rich_text_box(s, m, [x1, y, marks_w, answer_h], 18.0, INK)?;
            s.execute("shape.rename", &json!({"id": mk, "name": format!("Marks{}", tag(one_part, i))}))?;
            set_anchor_align(s, mk, "bottom", "right")?;
            edit_runs(s, mk, |r| r.bold = Some(true))?;
        }
        // The answer sits on the line, hidden until a click in Slide Show.
        let a = answers.get(i).map(String::as_str).unwrap_or("");
        let box_ = draw_shape(s, "roundRect", [lx, y + 2.0, line_w, answer_h - 4.0], Some("#E3F4E8"), Some(("#2E7D4F", 1.5)))?;
        s.execute("shape.rename", &json!({"id": box_, "name": format!("Answer{}", tag(one_part, i))}))?;
        set_rich_shape_text(s, box_, a, (size * 0.85).max(14.0), "#14532D")?;
        hidden.push(box_);
        Ok(())
    };

    if one_part {
        let y_line = y.max(bottom - answer_h - gap).min(bottom - answer_h);
        let marks = if blank { Some("( )".to_string()) } else { q.marks.map(|m| format!("({m})")) };
        answer_line(s, y_line, 0, marks.as_deref())?;
    } else {
        for (i, part) in rows.iter().enumerate() {
            let ph = lines_at(part, size, part_w) * line;
            let l = rich_text_box(s, &format!("({})", letter(i)), [x0, y, letter_w, line], size, INK)?;
            s.execute("shape.rename", &json!({"id": l, "name": format!("Part ({})", letter(i))}))?;
            let t = rich_text_box(s, part, [x0 + letter_w, y, part_w, ph], size, INK)?;
            s.execute("shape.rename", &json!({"id": t, "name": format!("Part ({}) text", letter(i))}))?;
            y += ph + 4.0;
            // Per-part marks are not in the bank; a blank question gets a "( )" to fill in.
            answer_line(s, y, i, blank.then_some("( )"))?;
            y += answer_h + gap;
        }
    }

    // The total, bottom right, in bold.
    let total = match (blank, q.marks) {
        (true, _) | (false, None) => format!("(Total for Question {number} is    marks)"),
        (false, Some(m)) => format!("(Total for Question {number} is {m} mark{})", if m == 1 { "" } else { "s" }),
    };
    let t = rich_text_box(s, &total, [w - margin - 420.0, h - margin - footer + 6.0, 420.0, footer - 6.0], 18.0, INK)?;
    s.execute("shape.rename", &json!({"id": t, "name": "Total marks"}))?;
    set_anchor_align(s, t, "middle", "right")?;
    edit_runs(s, t, |r| r.bold = Some(true))?;

    if let Some(d) = figure {
        let fx = x0 + text_w + 16.0;
        super::figures::draw(s, &d, [fx, top, w - margin - fx, bottom - top], 1.6)?;
    }
    // Answers appear one click at a time, in order.
    for id in hidden {
        s.execute("animation.add", &json!({"ids": [id], "effect": "appear", "start": "onClick"}))?;
    }
    Ok(())
}

/// How a part is named on its shapes: " (a)", or nothing for a one-part question.
fn tag(one_part: bool, i: usize) -> String {
    if one_part { String::new() } else { format!(" ({})", letter(i)) }
}

/// Sets a text shape's vertical anchor and its paragraphs' alignment.
fn set_anchor_align(s: &mut Session, id: u64, anchor: &str, align: &str) -> Result<()> {
    use deckcraft_model::text::{Align, Anchor};
    let anchor = match anchor {
        "bottom" => Anchor::Bottom,
        "middle" => Anchor::Middle,
        _ => Anchor::Top,
    };
    let align = if align == "right" { Align::Right } else { Align::Left };
    s.edit(|doc, sel| {
        let list = crate::shapes_mut(doc, sel).ok_or_else(|| bad("slide.examQuestion", "no slide"))?;
        for sh in list.iter_mut().filter(|sh| u64::from(sh.id.0) == id) {
            if let Some(t) = sh.text.as_mut() {
                t.body.anchor = Some(anchor);
                for para in &mut t.paragraphs {
                    para.props.align = Some(align);
                }
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use serde_json::json;

    use super::*;

    fn names_and_texts(s: &Session, i: usize) -> Vec<(String, String)> {
        let slide = &s.doc().unwrap().doc.slides[i];
        slide
            .shapes
            .iter()
            .map(|sh| {
                (sh.name.clone(), sh.text.as_ref().map(|t| t.paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join(" ")).unwrap_or_default())
            })
            .collect()
    }

    fn text_of(v: &[(String, String)], name: &str) -> String {
        v.iter().find(|(n, _)| n == name).map(|(_, t)| t.clone()).unwrap_or_else(|| panic!("no {name}: {v:?}"))
    }

    #[test]
    fn marks_and_parts_are_read_from_the_question() {
        assert_eq!(take_marks("Solve 2x = 8. (2 marks)"), ("Solve 2x = 8.".into(), Some(2)));
        assert_eq!(take_marks("Solve it (1 mark) "), ("Solve it".into(), Some(1)));
        assert_eq!(take_marks("Work out (3 + 4)"), ("Work out (3 + 4)".into(), None));
        let l = lay_out("A shop. (a) Work out the price. (b) Then the sale price. (4 marks)");
        assert_eq!(l, Layout { stem: "A shop.".into(), parts: vec!["Work out the price.".into(), "Then the sale price.".into()], marks: Some(4) });
        // A lone (a), or parts out of order, is one part.
        assert!(lay_out("Look at (a) only. (1 mark)").parts.is_empty());
        assert!(lay_out("(b) first (a) second").parts.is_empty());
    }

    #[test]
    fn a_bank_question_is_set_out_edexcel_style_with_hidden_answers_and_notes() {
        let mut s = Session::with_new();
        // A two-part question from a bank given as JSON, so the test does not depend on the built-in one.
        let bank = json!({"version": 1, "questions": [{
            "id": "t1", "topic": "Number", "subtopic": "Primes", "difficulty": 4, "qtype": "fluency",
            "source": "Y9", "year": "Y9",
            "text": "Here is a number. (a) Write down the next prime after 7. (b) Work out $\\frac{1}{2} + \\frac{1}{4}$. (3 marks)",
            "answer": "(a) 11  (b) $\\frac{3}{4}$"
        }]});
        let r = s.execute("slide.examQuestion", &json!({"bankJson": bank.to_string(), "number": 2})).unwrap();
        assert_eq!((r["question"].as_str(), r["marks"].as_u64()), (Some("t1"), Some(3)));
        let i = r["index"].as_u64().unwrap() as usize;
        let v = names_and_texts(&s, i);
        assert_eq!(s.doc().unwrap().doc.slides[i].name, EXAM_NAME);
        assert_eq!(text_of(&v, "Question number"), "2");
        assert_eq!(text_of(&v, "Question"), "Here is a number.");
        assert_eq!(text_of(&v, "Part (a)"), "(a)");
        assert_eq!(text_of(&v, "Part (a) text"), "Write down the next prime after 7.");
        assert!(text_of(&v, "Part (b) text").starts_with("Work out"));
        assert_eq!(text_of(&v, "Answer (a)"), "11");
        assert_eq!(text_of(&v, "Total marks"), "(Total for Question 2 is 3 marks)");
        assert!(v.iter().all(|(n, _)| !n.starts_with("Marks")), "the bank has no per-part marks, so none are shown");
        // The answers appear on click, (a) then (b).
        let slide = &s.doc().unwrap().doc.slides[i];
        let id_of = |name: &str| slide.shapes.iter().find(|sh| sh.name == name).unwrap().id;
        let order: Vec<_> = slide.animations.iter().map(|a| (a.shape, a.effect.as_str(), a.start)).collect();
        use deckcraft_model::AnimStart::OnClick;
        assert_eq!(order, vec![(id_of("Answer (a)"), "appear", OnClick), (id_of("Answer (b)"), "appear", OnClick)]);
        let notes = slide.notes_text();
        assert!(notes.contains("(a) 11") && notes.contains("Total: 3 marks"), "{notes}");
        // Everything is inside the slide.
        for sh in &slide.shapes {
            let Some(x) = sh.xfrm else { continue };
            assert!(x.x >= 0.0 && x.y >= 0.0 && x.x + x.w <= 960.5 && x.y + x.h <= 540.5, "{} at {x:?}", sh.name);
        }
    }

    #[test]
    fn a_one_part_question_shows_its_marks_on_the_answer_line() {
        let mut s = Session::with_new();
        let bank = json!({"version": 1, "questions": [{
            "id": "t2", "topic": "Algebra", "subtopic": "Equations", "difficulty": 4, "qtype": "fluency",
            "source": "Y8", "year": "Y8", "text": "Solve $3x - 5 = 16$ (2 marks)", "answer": "$x = 7$"
        }]});
        let r = s.execute("slide.examQuestion", &json!({"bankJson": bank.to_string()})).unwrap();
        let v = names_and_texts(&s, r["index"].as_u64().unwrap() as usize);
        assert_eq!(text_of(&v, "Marks"), "(2)");
        assert_eq!(text_of(&v, "Total marks"), "(Total for Question 1 is 2 marks)");
        assert!(text_of(&v, "Answer").ends_with("= 7"), "{v:?}");
    }

    #[test]
    fn a_blank_question_has_prompts_and_empty_marks_and_rerolls_in_place() {
        let mut s = Session::with_new();
        let r = s.execute("slide.examQuestion", &json!({"blank": true, "parts": 3})).unwrap();
        let i = r["index"].as_u64().unwrap() as usize;
        let v = names_and_texts(&s, i);
        assert_eq!(text_of(&v, "Part (c) text"), "Type part (c) here.");
        assert_eq!(text_of(&v, "Marks (b)"), "( )");
        assert_eq!(text_of(&v, "Total marks"), "(Total for Question 1 is    marks)");
        assert!(r["marks"].is_null() && r["question"].is_null());
        let slides = s.doc().unwrap().doc.slides.len();
        s.execute("slide.examQuestion", &json!({"blank": true, "reroll": true})).unwrap();
        assert_eq!(s.doc().unwrap().doc.slides.len(), slides, "a reroll replaces the slide");
        assert!(names_and_texts(&s, i).iter().all(|(n, _)| !n.starts_with("Part")), "now one part");
        assert!(s.execute("slide.examQuestion", &json!({"blank": true, "parts": 9})).is_err());
        assert!(s.execute("slide.examQuestion", &json!({"blank": true, "number": 0})).is_err());
    }

    #[test]
    fn the_built_in_bank_gives_an_exam_question_that_saves_to_powerpoint() {
        let mut s = Session::with_new();
        let r = s.execute("slide.examQuestion", &json!({"seed": 3})).unwrap();
        assert!(r["question"].is_string() && r["marks"].as_u64().is_some(), "{r}");
        let bytes = deckcraft_pptx::export(&s.doc().unwrap().doc).unwrap();
        let back = deckcraft_pptx::import(&bytes).unwrap();
        let i = r["index"].as_u64().unwrap() as usize;
        assert_eq!(back.slides[i].animations.len(), s.doc().unwrap().doc.slides[i].animations.len(), "the hidden answers survive saving");
        assert!(s.execute("slide.examQuestion", &json!({"search": "no such words anywhere zzz"})).is_err());
    }
}
