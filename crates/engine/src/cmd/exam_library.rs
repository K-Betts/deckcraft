//! Insert a past-paper question from the teacher's own library folder (see `crate::exam_library`).
//!
//! `slide.examLibraryQuestion` puts the question's picture on a new slide, with its answers in a box
//! that appears on a click in Slide Show, the answers in the speaker notes, and where the question
//! came from in small print at the bottom.

use serde_json::{Value, json};

use super::exam::{EXAM_NAME, exam_layout};
use super::get_ready::{draw_shape, plain_answer, plain_layout_name, rich_text_box, set_rich_shape_text};
use super::*;
use crate::exam_library::{Item, parse_items};
use crate::{Result, Session};

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "slide.examLibraryQuestion",
        "Exam Question from My Library",
        ["Insert", "Slides From"],
        None,
        "{dir: the library folder, id: a question id (exam-<paper>-Q<n>), at?: slide index} — the question's picture, its answers on a click",
        has_doc,
        library_question
    )]
}

const DIM: &str = "#6B7280";

/// Width and height in pixels of a PNG, from its header.
fn png_size(bytes: &[u8]) -> Option<(f64, f64)> {
    if bytes.len() < 24 || !bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return None;
    }
    let be = |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    Some((f64::from(be(16)), f64::from(be(20))))
}

/// The paper and question number in an id: `exam-<paper>-Q<n>`.
fn record_of(id: &str) -> Option<&str> {
    let rest = id.strip_prefix("exam-")?;
    let (record, n) = rest.rsplit_once("-Q")?;
    (!record.is_empty() && !record.contains(['/', '\\', '.']) && n.parse::<u32>().is_ok()).then_some(record)
}

/// The answers as lines: one per part that has one, or the question's own answer.
fn answer_lines(item: &Item) -> Vec<String> {
    let parts: Vec<String> = item
        .parts
        .iter()
        .filter(|p| !p.answer.trim().is_empty())
        .map(|p| format!("({}) {}", p.label.trim_matches(['(', ')']), p.answer.trim()))
        .collect();
    if !parts.is_empty() {
        parts
    } else if !item.answer.trim().is_empty() {
        vec![item.answer.trim().to_string()]
    } else {
        vec![]
    }
}

fn library_question(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "slide.examLibraryQuestion";
    let dir = str_param(p, "dir").ok_or_else(|| bad(cmd, "missing `dir` (the library folder)"))?;
    let id = str_param(p, "id").ok_or_else(|| bad(cmd, "missing `id`"))?;
    let record = record_of(id).ok_or_else(|| bad(cmd, "`id` should look like exam-<paper>-Q<n>"))?;
    let file = format!("{}/items/exam/{record}.json", dir.trim_end_matches('/'));
    let text = String::from_utf8(crate::io::read_file(&file).map_err(EngineError::Other)?).map_err(|_| bad(cmd, "the question file is not text"))?;
    let item = parse_items(&text)
        .map_err(EngineError::Other)?
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| bad(cmd, format!("{id} is not in {record}.json")))?;
    let crop = item.crop_path(dir);
    let bytes = crate::io::read_file(&crop).map_err(|_| {
        bad(cmd, format!("no picture for this question yet ({}); make the pictures with the library's crop_exam_all.py", item.crop_name()))
    })?;
    let (px, py) = png_size(&bytes).ok_or_else(|| bad(cmd, "the question's picture is not a PNG"))?;
    let answers = answer_lines(&item);

    // Where it goes: after the current slide.
    let (cur, layout_name) = {
        let st = s.doc()?;
        let cur = st.selection.slide;
        let slide = st.doc.slides.get(cur);
        let master = slide.and_then(|sl| st.doc.masters.iter().position(|m| m.layout(sl.layout).is_some())).unwrap_or(0);
        (cur, exam_layout(&st.doc, master).map(|l| l.name.clone()).unwrap_or_else(|| plain_layout_name(&st.doc, master)))
    };
    let at = usize_param(p, "at").unwrap_or(cur + 1);
    s.execute("slide.new", &json!({"layout": layout_name, "at": at}))?;
    let index = s.doc()?.selection.slide;
    s.execute("slide.rename", &json!({"name": format!("{EXAM_NAME} (library)"), "index": index}))?;
    let (w, h) = {
        let d = &s.doc()?.doc;
        (d.slide_size.width.max(320.0), d.slide_size.height.max(240.0))
    };

    let margin = 32.0;
    let footer = 22.0;
    let gap = 12.0;
    let answers_h = if answers.is_empty() { 0.0 } else { (answers.len() as f64 * 30.0 + 16.0).min(h * 0.3) };
    let box_w = w - margin * 2.0;
    let box_h = h - margin * 2.0 - footer - if answers.is_empty() { 0.0 } else { answers_h + gap };
    // The picture fills the space, never wider than the slide and never stretched out of shape.
    let k = (box_w / px).min(box_h / py);
    let (pw, ph) = (px * k, py * k);
    let pic = s.execute("insert.picture", &json!({"path": crop, "name": item.crop_name(), "rect": [margin + (box_w - pw) / 2.0, margin, pw, ph]}))?;
    if let Some(pid) = pic.get("id").and_then(Value::as_u64) {
        s.execute("shape.rename", &json!({"id": pid, "name": "Question"}))?;
    }

    let mut hidden = None;
    if !answers.is_empty() {
        let y = margin + ph + gap;
        let b = draw_shape(s, "roundRect", [margin, y, box_w, answers_h], Some("#E3F4E8"), Some(("#2E7D4F", 1.5)))?;
        s.execute("shape.rename", &json!({"id": b, "name": "Answers"}))?;
        set_rich_shape_text(s, b, &answers.join("\n"), 16.0, "#14532D")?;
        hidden = Some(b);
    }
    let src =
        rich_text_box(s, &format!("{}. For school use only.", item.source_line()), [margin, h - margin - footer + 4.0, box_w, footer], 10.0, DIM)?;
    s.execute("shape.rename", &json!({"id": src, "name": "Source"}))?;
    if let Some(b) = hidden {
        s.execute("animation.add", &json!({"ids": [b], "effect": "appear", "start": "onClick"}))?;
    }

    let mut notes = String::from("Answers:\n");
    if answers.is_empty() {
        notes.push_str("(the library has no mark-scheme answer for this question)\n");
    }
    for a in &answers {
        notes.push_str(&plain_answer(a));
        notes.push('\n');
    }
    if item.marks > 0 {
        notes.push_str(&format!("Total: {} mark{}\n", item.marks, if item.marks == 1 { "" } else { "s" }));
    }
    notes.push_str(&item.source_line());
    s.execute("slide.notes", &json!({"text": notes}))?;
    Ok(json!({"index": index, "question": item.id, "marks": item.marks}))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 40x20 pixel PNG, built by hand: header, a one-pixel-row image is not needed for sizing.
    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        b.extend(w.to_be_bytes());
        b.extend(h.to_be_bytes());
        b.extend([8, 2, 0, 0, 0, 0, 0, 0, 0]);
        b
    }

    #[test]
    fn ids_and_sizes_are_read_safely() {
        assert_eq!(record_of("exam-TEST-1F-Q12"), Some("TEST-1F"));
        assert_eq!(record_of("exam-../../x-Q1"), None);
        assert_eq!(record_of("mathspad-7C-Q1"), None);
        assert_eq!(record_of("exam-TEST-1F-Qx"), None);
        assert_eq!(png_size(&png(40, 20)), Some((40.0, 20.0)));
        assert_eq!(png_size(b"nope"), None);
    }

    #[test]
    fn answers_come_by_part_or_as_one() {
        let mut it = Item::default();
        assert!(answer_lines(&it).is_empty());
        it.answer = "12".into();
        assert_eq!(answer_lines(&it), vec!["12"]);
        it.parts = vec![
            crate::exam_library::Part { label: "a".into(), answer: "x = 1".into(), ..Default::default() },
            crate::exam_library::Part { label: "(b)".into(), answer: " ".into(), ..Default::default() },
            crate::exam_library::Part { label: "c".into(), answer: "7".into(), ..Default::default() },
        ];
        assert_eq!(answer_lines(&it), vec!["(a) x = 1", "(c) 7"]);
    }

    /// A library folder built in code: one made-up question with a picture of a plain rectangle.
    fn library(with_picture: bool) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!("deckcraft-exam-lib-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(dir.join("items/exam")).unwrap();
        std::fs::create_dir_all(dir.join("derived/exam_crops")).unwrap();
        let q = |n: u32, parts: Value, answer: &str| {
            json!({"id": format!("exam-TEST-1H-Q{n}"), "kind": "question", "tier": "Higher", "status": "checked", "text": "Made-up question.",
                   "parts": parts, "answer": answer, "source": {"record": "TEST-1H", "question": n},
                   "exam": {"exam_board": "Test board", "qual_code": "T1", "calculator": "calculator", "total_marks": 4, "series": "Summer",
                            "year_sat": 2020, "paper": "1H", "strand": "Algebra", "topic": "Made up", "source_type": "past_paper"}})
        };
        let file = json!({"schema": "maths-workspace.items/2", "items": [
            q(1, json!([{"label": "a", "text": "p", "answer": "x = 2", "marks": 2}, {"label": "b", "text": "q", "answer": "y = 3", "marks": 2}]), ""),
            q(2, json!([]), ""),
        ]});
        std::fs::write(dir.join("items/exam/TEST-1H.json"), file.to_string()).unwrap();
        if with_picture {
            let img = image::RgbImage::from_pixel(300, 120, image::Rgb([240, 240, 240]));
            for n in [1, 2] {
                img.save(dir.join(format!("derived/exam_crops/TEST-1H-Q{n}.png"))).unwrap();
            }
        }
        dir
    }

    #[test]
    fn a_library_question_becomes_a_picture_slide_with_hidden_answers_and_notes() {
        let dir = library(true);
        let d = dir.to_str().unwrap();
        let mut s = Session::with_new();
        let before = s.doc().unwrap().doc.slides.len();
        let r = s.execute("slide.examLibraryQuestion", &json!({"dir": d, "id": "exam-TEST-1H-Q1"})).unwrap();
        assert_eq!(r["marks"], 4);
        let st = s.doc().unwrap();
        assert_eq!(st.doc.slides.len(), before + 1);
        let slide = &st.doc.slides[r["index"].as_u64().unwrap() as usize];
        assert_eq!(slide.name, "Exam Question (library)");
        let names: Vec<&str> = slide.shapes.iter().map(|x| x.name.as_str()).collect();
        for want in ["Question", "Answers", "Source"] {
            assert!(names.contains(&want), "{want} in {names:?}");
        }
        let notes: String = slide.notes.paragraphs.iter().flat_map(|p| p.runs.iter().map(|r| r.text.clone())).collect::<Vec<_>>().join("|");
        assert!(
            notes.contains("(a) x = 2")
                && notes.contains("(b) y = 3")
                && notes.contains("Total: 4 marks")
                && notes.contains("Summer 2020, Paper 1H, Question 1"),
            "{notes}"
        );
        // One undo takes the whole slide away.
        s.execute("edit.undo", &json!({})).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_question_with_no_answers_has_no_answer_box_and_says_so_in_the_notes() {
        let dir = library(true);
        let mut s = Session::with_new();
        let r = s.execute("slide.examLibraryQuestion", &json!({"dir": dir.to_str().unwrap(), "id": "exam-TEST-1H-Q2"})).unwrap();
        let st = s.doc().unwrap();
        let slide = &st.doc.slides[r["index"].as_u64().unwrap() as usize];
        assert!(slide.shapes.iter().all(|x| x.name != "Answers"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_things_are_explained_and_nothing_is_added() {
        let dir = library(false);
        let d = dir.to_str().unwrap();
        let mut s = Session::with_new();
        let before = s.doc().unwrap().doc.slides.len();
        let e = s.execute("slide.examLibraryQuestion", &json!({"dir": d, "id": "exam-TEST-1H-Q1"})).unwrap_err().to_string();
        assert!(e.contains("no picture"), "{e}");
        let e = s.execute("slide.examLibraryQuestion", &json!({"dir": d, "id": "exam-TEST-1H-Q9"})).unwrap_err().to_string();
        assert!(e.contains("not in"), "{e}");
        assert!(s.execute("slide.examLibraryQuestion", &json!({"dir": d, "id": "exam-../x-Q1"})).is_err());
        assert!(s.execute("slide.examLibraryQuestion", &json!({"id": "exam-TEST-1H-Q1"})).is_err());
        assert_eq!(s.doc().unwrap().doc.slides.len(), before);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
