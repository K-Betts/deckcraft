//! A teacher's own library of past-paper questions, searched from Insert › Exam Question.
//!
//! The questions are never part of DeckCraft: they are other people's exam papers, so they live in a
//! folder on the teacher's computer (the "library folder") that the teacher chooses, and nothing
//! here copies them anywhere. The folder holds the question files (`items/exam/*.json`, one file per
//! paper in the `maths-workspace.items/2` format) and a picture of each question
//! (`derived/exam_crops/<paper>-Q<n>.png`, the paper's own page cropped to that question).
//!
//! This module only reads and searches; `slide.examLibraryQuestion` puts one on a slide.

use serde_json::Value;

/// One part of a question: its label, the marks it carries and its mark-scheme answer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Part {
    pub label: String,
    pub text: String,
    pub answer: String,
    pub marks: Option<u32>,
}

/// A past-paper question.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Item {
    /// `exam-<paper>-Q<n>`.
    pub id: String,
    /// The exam board and specification, e.g. `Edexcel 1MA1`.
    pub board: String,
    /// The paper's record, e.g. `EDEX-GCSE-1MA1-2017-JUN-1F`; also the file's name.
    pub record: String,
    pub question: u32,
    /// `Foundation` or `Higher`.
    pub tier: String,
    pub calculator: Option<bool>,
    pub marks: u32,
    /// `Summer`, `November` or empty.
    pub series: String,
    pub year: Option<u32>,
    /// `1F`, `2H`, …
    pub paper: String,
    pub strand: String,
    pub topic: String,
    pub text: String,
    pub parts: Vec<Part>,
    /// The mark-scheme answer when the question is not split into parts.
    pub answer: String,
    /// `checked`, `text_ok_answer_unverified`, `needs_check` or `needs_person`.
    pub status: String,
    pub past_paper: bool,
}

impl Item {
    /// The picture's file name, without the folder.
    pub fn crop_name(&self) -> String {
        format!("{}-Q{}.png", self.record, self.question)
    }

    /// Where the question's picture is inside the library folder `dir`.
    pub fn crop_path(&self, dir: &str) -> String {
        format!("{}/derived/exam_crops/{}", dir.trim_end_matches('/'), self.crop_name())
    }

    /// "Summer 2019, Paper 1H, Question 7": where it came from, for the slide's footer and the list.
    pub fn source_line(&self) -> String {
        let when = match (self.series.as_str(), self.year) {
            ("", Some(y)) => y.to_string(),
            (s, Some(y)) => format!("{s} {y}"),
            (s, None) => s.to_string(),
        };
        let kind = if self.past_paper { "" } else { " (practice paper)" };
        let sep = if when.is_empty() { "" } else { ", " };
        let board = if self.board.is_empty() { String::new() } else { format!("{} GCSE Mathematics, ", self.board) };
        format!("{board}{when}{sep}Paper {}, Question {}{kind}", self.paper, self.question)
    }

    /// Whether the questions can be used as they stand: not held back for a person to look at.
    pub fn usable(&self) -> bool {
        !matches!(self.status.as_str(), "needs_person" | "needs_check")
    }
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

fn u(v: &Value, k: &str) -> Option<u32> {
    v.get(k).and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok())
}

/// The questions in one question file. Records that are not questions, or lack an id, are skipped.
pub fn parse_items(json: &str) -> Result<Vec<Item>, String> {
    let root: Value = serde_json::from_str(json).map_err(|e| format!("not a question file: {e}"))?;
    let items = root.get("items").and_then(Value::as_array).ok_or("not a question file: no `items`")?;
    let mut out = Vec::with_capacity(items.len());
    for it in items {
        if it.get("kind").and_then(Value::as_str).is_some_and(|k| k != "question") {
            continue;
        }
        let id = s(it, "id");
        if id.is_empty() {
            continue;
        }
        let ex = it.get("exam").cloned().unwrap_or(Value::Null);
        let src = it.get("source").cloned().unwrap_or(Value::Null);
        let parts = it
            .get("parts")
            .and_then(Value::as_array)
            .map(|ps| ps.iter().map(|p| Part { label: s(p, "label"), text: s(p, "text"), answer: s(p, "answer"), marks: u(p, "marks") }).collect())
            .unwrap_or_default();
        let tier = match s(it, "tier").as_str() {
            "Foundation" | "foundation" => "Foundation".to_string(),
            "Higher" | "higher" => "Higher".to_string(),
            other => other.to_string(),
        };
        out.push(Item {
            record: s(&src, "record"),
            question: u(&src, "question").unwrap_or(0),
            id,
            board: [s(&ex, "exam_board"), s(&ex, "qual_code")].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" "),
            tier,
            calculator: match s(&ex, "calculator").as_str() {
                "calculator" => Some(true),
                "non-calculator" => Some(false),
                _ => None,
            },
            marks: u(&ex, "total_marks").unwrap_or(0),
            series: s(&ex, "series"),
            year: u(&ex, "year_sat"),
            paper: s(&ex, "paper"),
            strand: s(&ex, "strand"),
            topic: s(&ex, "topic"),
            text: s(it, "text"),
            parts,
            answer: s(it, "answer"),
            status: s(it, "status"),
            past_paper: s(&ex, "source_type") != "practice_test",
        });
    }
    Ok(out)
}

/// Every question in the library folder `dir` (its `items/exam/*.json`), in a steady order.
pub fn load_dir(dir: &str) -> Result<Vec<Item>, String> {
    let folder = format!("{}/items/exam", dir.trim_end_matches('/'));
    let mut files: Vec<String> = crate::io::list_files(&folder).into_iter().filter(|f| f.ends_with(".json")).collect();
    if files.is_empty() {
        return Err("no question files in that folder (expected items/exam/*.json inside it)".into());
    }
    files.sort();
    let mut all = Vec::new();
    for f in files {
        let bytes = crate::io::read_file(&f)?;
        let text = String::from_utf8(bytes).map_err(|_| format!("{f} is not text"))?;
        all.extend(parse_items(&text)?);
    }
    all.sort_by(|a, b| (&a.record, a.question).cmp(&(&b.record, b.question)));
    Ok(all)
}

/// What to look for. Empty fields do not filter.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Query {
    /// Words; every one must be somewhere in the question, its topic or its strand.
    pub text: String,
    /// `Foundation` or `Higher`.
    pub tier: Option<String>,
    pub calculator: Option<bool>,
    pub min_marks: Option<u32>,
    pub max_marks: Option<u32>,
    /// A strand such as `Algebra`.
    pub strand: Option<String>,
    pub year: Option<u32>,
    /// Only the questions from real past papers, not practice papers.
    pub past_papers_only: bool,
    /// Also list the questions held back for checking.
    pub include_unchecked: bool,
}

fn norm(t: &str) -> String {
    t.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect()
}

/// A word with a plural `s` or `es` taken off, so "equations" finds "equation".
fn stem(w: &str) -> &str {
    let w = w.strip_suffix("es").filter(|r| r.len() > 3).unwrap_or(w);
    w.strip_suffix('s').filter(|r| r.len() > 2).unwrap_or(w)
}

/// Indexes into `items` of the matches, best first (a topic or strand match before one in the
/// words, then the more marks, then paper order), at most `limit`.
pub fn search(items: &[Item], q: &Query, limit: usize) -> Vec<usize> {
    let words: Vec<String> = norm(&q.text).split_whitespace().map(|w| stem(w).to_string()).collect();
    let phrase = words.join(" ");
    let mut hits: Vec<(u32, usize)> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        if !(q.include_unchecked || it.usable())
            || q.tier.as_deref().is_some_and(|t| !it.tier.eq_ignore_ascii_case(t))
            || q.calculator.is_some_and(|c| it.calculator != Some(c))
            || q.min_marks.is_some_and(|m| it.marks < m)
            || q.max_marks.is_some_and(|m| it.marks > m)
            || q.strand.as_deref().is_some_and(|t| !it.strand.eq_ignore_ascii_case(t))
            || q.year.is_some_and(|y| it.year != Some(y))
            || (q.past_papers_only && !it.past_paper)
        {
            continue;
        }
        let mut score = 1;
        if !words.is_empty() {
            let topic = norm(&it.topic);
            let strand = norm(&it.strand);
            let mut body = norm(&it.text);
            for p in &it.parts {
                body.push(' ');
                body.push_str(&norm(&p.text));
            }
            let stemmed = |t: &str| t.split_whitespace().map(stem).collect::<Vec<_>>().join(" ");
            let (topic, strand, body) = (stemmed(&topic), stemmed(&strand), stemmed(&body));
            if !words.iter().all(|w| topic.contains(w.as_str()) || strand.contains(w.as_str()) || body.contains(w.as_str())) {
                continue;
            }
            if topic.contains(&phrase) {
                score += 20;
            }
            score += words.iter().filter(|w| topic.contains(w.as_str())).count() as u32 * 5;
            score += words.iter().filter(|w| strand.contains(w.as_str())).count() as u32 * 2;
            if body.contains(&phrase) {
                score += 3;
            }
        }
        hits.push((score, i));
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(items[b.1].marks.cmp(&items[a.1].marks)).then(a.1.cmp(&b.1)));
    hits.into_iter().take(limit).map(|(_, i)| i).collect()
}

/// The strands present, for the filter, in a steady order.
pub fn strands(items: &[Item]) -> Vec<String> {
    let mut v: Vec<String> = items.iter().map(|i| i.strand.clone()).filter(|s| !s.is_empty() && s != "unassigned").collect();
    v.sort();
    v.dedup();
    v
}

/// The years the papers were sat, newest first.
pub fn years(items: &[Item]) -> Vec<u32> {
    let mut v: Vec<u32> = items.iter().filter_map(|i| i.year).collect();
    v.sort_unstable_by(|a, b| b.cmp(a));
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny library built in code. Nothing here is from a real paper.
    fn file() -> String {
        serde_json::json!({
            "schema": "maths-workspace.items/2",
            "items": [
                item(1, "Foundation", "Algebra", "Simultaneous equations", "Solve the pair of equations.", 3, "calculator", 2019, "Summer", "checked"),
                item(2, "Higher", "Algebra", "Simultaneous equations", "Solve the pair, one of them a circle.", 5, "non-calculator", 2018, "November", "checked"),
                item(3, "Higher", "Number", "Indices and surds", "Simplify the surd.", 2, "non-calculator", 2019, "Summer", "needs_person"),
                {"id": "exam-X-Q9", "kind": "notes"},
            ]
        })
        .to_string()
    }

    #[allow(clippy::too_many_arguments)]
    fn item(n: u32, tier: &str, strand: &str, topic: &str, text: &str, marks: u32, calc: &str, year: u32, series: &str, status: &str) -> Value {
        serde_json::json!({
            "id": format!("exam-TEST-1{}-Q{n}", &tier[..1]), "kind": "question", "tier": tier, "status": status, "text": text,
            "parts": [{"label": "a", "text": "Part text", "answer": "x = 1", "marks": 2}],
            "answer": "", "source": {"record": format!("TEST-1{}", &tier[..1]), "question": n},
            "exam": {"exam_board": "Test board", "qual_code": "T1", "calculator": calc, "total_marks": marks, "series": series, "year_sat": year, "paper": format!("1{}", &tier[..1]),
                     "strand": strand, "topic": topic, "source_type": "past_paper"}
        })
    }

    fn items() -> Vec<Item> {
        parse_items(&file()).unwrap()
    }

    #[test]
    fn a_file_gives_its_questions_and_skips_other_records() {
        let v = items();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].tier, "Foundation");
        assert_eq!(v[0].calculator, Some(true));
        assert_eq!(v[1].calculator, Some(false));
        assert_eq!((v[0].marks, v[0].year), (3, Some(2019)));
        assert_eq!(v[0].parts[0].marks, Some(2));
        assert_eq!(v[0].crop_name(), "TEST-1F-Q1.png");
        assert_eq!(v[0].crop_path("/lib/"), "/lib/derived/exam_crops/TEST-1F-Q1.png");
        assert_eq!(v[0].source_line(), "Test board T1 GCSE Mathematics, Summer 2019, Paper 1F, Question 1");
        assert!(parse_items("[]").is_err() && parse_items("not json").is_err());
    }

    #[test]
    fn searching_a_skill_finds_it_in_the_topic_and_in_plural_words() {
        let v = items();
        let q = Query { text: "simultaneous equation".into(), ..Default::default() };
        assert_eq!(search(&v, &q, 10), vec![1, 0], "the 5-mark one first");
        let q = Query { text: "Simultaneous Equations".into(), ..Default::default() };
        assert_eq!(search(&v, &q, 10).len(), 2);
        let q = Query { text: "circle".into(), ..Default::default() };
        assert_eq!(search(&v, &q, 10), vec![1], "words in the question text count too");
        let q = Query { text: "simultaneous trigonometry".into(), ..Default::default() };
        assert!(search(&v, &q, 10).is_empty(), "every word must be somewhere");
    }

    #[test]
    fn filters_narrow_the_list_and_held_back_questions_are_hidden() {
        let v = items();
        let base = Query { text: "equations".into(), ..Default::default() };
        let by = |f: &dyn Fn(&mut Query)| {
            let mut q = base.clone();
            f(&mut q);
            search(&v, &q, 10)
        };
        assert_eq!(by(&|q| q.tier = Some("Higher".into())), vec![1]);
        assert_eq!(by(&|q| q.calculator = Some(true)), vec![0]);
        assert_eq!(by(&|q| q.min_marks = Some(4)), vec![1]);
        assert_eq!(by(&|q| q.max_marks = Some(3)), vec![0]);
        assert_eq!(by(&|q| q.year = Some(2018)), vec![1]);
        assert_eq!(by(&|q| q.strand = Some("number".into())), Vec::<usize>::new());
        let all = Query::default();
        assert_eq!(search(&v, &all, 10).len(), 2, "the held-back one is not listed");
        assert_eq!(search(&v, &Query { include_unchecked: true, ..all }, 10).len(), 3);
        assert_eq!(search(&v, &Query::default(), 1).len(), 1, "the limit holds");
    }

    #[test]
    fn the_filters_list_what_is_there() {
        let v = items();
        assert_eq!(strands(&v), vec!["Algebra", "Number"]);
        assert_eq!(years(&v), vec![2019, 2018]);
    }

    #[test]
    fn a_missing_folder_is_explained() {
        assert!(load_dir("/definitely/not/here").unwrap_err().contains("items/exam"));
    }

    /// Against a real library folder, for the teacher to run by hand:
    /// `DECKCRAFT_TEST_LIBRARY=~/maths-workspace/library cargo test -p deckcraft-engine real_library -- --ignored --nocapture`.
    /// It prints counts only, never question text.
    #[test]
    #[ignore = "needs the teacher's own library folder"]
    fn real_library_loads_and_searches() {
        let dir = std::env::var("DECKCRAFT_TEST_LIBRARY").expect("set DECKCRAFT_TEST_LIBRARY to the library folder");
        let items = load_dir(&dir).unwrap();
        let q = |text: &str| Query { text: text.into(), ..Default::default() };
        let hits = search(&items, &q("simultaneous equations"), usize::MAX);
        let higher = search(&items, &Query { tier: Some("Higher".into()), ..q("simultaneous equations") }, usize::MAX);
        let with_pictures = items.iter().filter(|i| std::path::Path::new(&i.crop_path(&dir)).exists()).count();
        println!(
            "questions {}, simultaneous equations {}, of those Higher {}, with a picture {}",
            items.len(),
            hits.len(),
            higher.len(),
            with_pictures
        );
        println!("strands {:?}, years {:?}", strands(&items), years(&items));
        assert!(!items.is_empty() && !hits.is_empty() && higher.len() < hits.len());
    }
}
