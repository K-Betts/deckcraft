//! The question bank behind the Get Ready starter slide.
//!
//! The bank is a JSON file: `{"version": 1, "questions": [Question, …]}`. The format is documented
//! in `docs/design/ux-review.md` (area E). A starter bank ships in the binary
//! (`assets/questions/bank.json`); a teacher's own bank is read from a path on request.
//! Nothing here touches the network or the pupils' data: questions are plain text.
//! LaTeX between `$…$` in `text` and `answer` is rendered as real equations on the slide.

use serde::{Deserialize, Serialize};

/// The question files compiled into the app, merged into one bank: the general questions, then one
/// file for each block of the scheme of work (see `docs/question-writing-guide.md`).
pub const STARTER_FILES: &[&str] = &[
    include_str!("../../../assets/questions/bank.json"),
    include_str!("../../../assets/questions/y7-autumn-b1-sequences.json"),
    include_str!("../../../assets/questions/y7-autumn-b2-algebraic-notation.json"),
    include_str!("../../../assets/questions/y7-autumn-b3-expressions-equations.json"),
    include_str!("../../../assets/questions/y7-autumn-b4-place-value.json"),
    include_str!("../../../assets/questions/y7-autumn-b5-four-operations.json"),
    include_str!("../../../assets/questions/y7-autumn-b6-averages-range.json"),
    include_str!("../../../assets/questions/y7-autumn-b7-rounding-estimation.json"),
    include_str!("../../../assets/questions/y7-spring-b2-fractions-decimals-percentages.json"),
    include_str!("../../../assets/questions/y7-spring-b4-fractions-percentages-of-amounts.json"),
    include_str!("../../../assets/questions/y7-summer-b3-add-subtract-fractions.json"),
    include_str!("../../../assets/questions/y7-spring-b1-graphing-data.json"),
    include_str!("../../../assets/questions/y7-spring-b3-directed-number.json"),
    include_str!("../../../assets/questions/y7-spring-b5-perimeter-area.json"),
    include_str!("../../../assets/questions/y7-summer-b1-speed-distance-time.json"),
    include_str!("../../../assets/questions/y7-summer-b2-properties-of-number.json"),
    include_str!("../../../assets/questions/y7-summer-b4-angles-polygons.json"),
    include_str!("../../../assets/questions/y8-autumn-b1-ratio.json"),
    include_str!("../../../assets/questions/y8-autumn-b2-proportion-scale.json"),
    include_str!("../../../assets/questions/y8-autumn-b3-algebraic-manipulation.json"),
    include_str!("../../../assets/questions/y8-autumn-b4-coordinates-graphs.json"),
    include_str!("../../../assets/questions/y8-autumn-b5-multiply-divide-fractions.json"),
    include_str!("../../../assets/questions/y8-autumn-b6-symmetry-reflection.json"),
    include_str!("../../../assets/questions/y8-spring-b1-area-volume-density.json"),
    include_str!("../../../assets/questions/y8-spring-b2-equations-inequalities.json"),
    include_str!("../../../assets/questions/y8-spring-b3-percentages.json"),
    include_str!("../../../assets/questions/y8-spring-b4-indices.json"),
    include_str!("../../../assets/questions/y8-spring-b5-standard-form.json"),
    include_str!("../../../assets/questions/y8-spring-b6-interpret-represent-data.json"),
    include_str!("../../../assets/questions/y8-summer-b1-angles-parallel-polygons.json"),
    include_str!("../../../assets/questions/y8-summer-b2-tables-probability.json"),
    include_str!("../../../assets/questions/y8-summer-b3-circles.json"),
    include_str!("../../../assets/questions/y8-summer-b4-charts-graphs.json"),
    include_str!("../../../assets/questions/y8-summer-b5-sequences.json"),
];

const MAX_BYTES: usize = 4 << 20;
const MAX_QUESTIONS: usize = 20_000;
const MAX_TEXT: usize = 600;
/// The figure kinds the Get Ready slide can draw.
pub const DRAWABLE: &[&str] = &[
    "triangle-angles",
    "isosceles-triangle",
    "parallel-lines",
    "right-triangle",
    "similar-triangles",
    "angles-line",
    "angles-point",
    "circle",
    "regular-polygon",
    "rectangle",
    "l-shape",
    "trapezium",
    "parallelogram",
    "cuboid",
    "bar-chart",
    "number-line",
    "coordinate-grid",
    "venn",
    "tree",
    "function-machine",
    "sequence-arrows",
    "pattern",
    "bar-model",
    "table",
    "fraction-bar",
    "shade-grid",
    "pictogram",
    "scatter",
    "line-graph",
    "polygon-shape",
    "polygon-angles",
    "ruler",
    "factor-tree",
    "pie",
    "counters",
    "crossing-lines",
];

/// Whether a question's `diagram` is one the slide can draw.
pub fn is_drawable(d: &serde_json::Value) -> bool {
    d.get("kind").and_then(|k| k.as_str()).is_some_and(|k| DRAWABLE.contains(&k))
}

impl Question {
    /// The year group (`Y7`…): the question's own, else the one its `source` starts with, else empty.
    pub fn year_group(&self) -> String {
        if !self.year.is_empty() {
            return self.year.clone();
        }
        let first = self.source.split_whitespace().next().unwrap_or("");
        let digits = first.strip_prefix('Y').unwrap_or("");
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) { first.to_string() } else { String::new() }
    }

    /// What identifies the skill (small step) the question practises: its scheme-of-work step when
    /// it has one, else its topic and subtopic.
    pub fn skill_key(&self) -> String {
        if self.source.is_empty() { format!("{} · {}", self.topic, self.subtopic) } else { self.source.clone() }
    }

    /// Usable on a slide: it needs no figure, or its figure can be drawn.
    pub fn usable(&self) -> bool {
        !self.needs_diagram || self.diagram.as_ref().is_some_and(is_drawable)
    }
}

pub const DIFFICULTIES: std::ops::RangeInclusive<u8> = 1..=4;

/// The names of the four levels, from 1: support, core, stretch and exam style.
pub const LEVEL_NAMES: [&str; 4] = ["Support", "Core", "Stretch", "Exam style"];

/// What a question asks of a pupil, as tagged by the writer.
pub const QUESTION_TYPES: [&str; 3] = ["fluency", "reasoning", "problem"];

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: String,
    pub topic: String,
    #[serde(default, alias = "subtopics")]
    pub subtopic: String,
    /// 1 = support, 2 = core, 3 = stretch, 4 = exam style.
    pub difficulty: u8,
    /// Words a teacher might search for, besides the topic and subtopic.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// `fluency`, `reasoning` or `problem`: what the question asks of a pupil.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub qtype: String,
    /// Where it sits in a scheme of work, e.g. “Y7 Autumn B3 S7 Solve 2-step equations”.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    /// The year group it is for: `Y7` to `Y11`. Empty falls back to the year at the start of `source`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub year: String,
    pub text: String,
    /// Shown in the slide's speaker notes, never on the slide.
    /// May contain LaTeX between `$…$` that is rendered as a real equation.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub answer: String,
    /// The question is meaningless without its figure. Such questions are skipped until diagrams
    /// are built (see the review: the figure is parameterised by `diagram`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub needs_diagram: bool,
    /// Free-form figure description, reserved for the diagram follow-up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagram: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bank {
    pub version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    #[serde(default)]
    pub questions: Vec<Question>,
    /// Questions written as recipes: new numbers every time (see `template.rs`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<crate::template::Template>,
}

#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub topic: Option<String>,
    pub difficulty: Option<u8>,
    /// One skill (small step): its `Question::skill_key`.
    pub skill: Option<String>,
    /// One year group, e.g. `Y7`.
    pub year: Option<String>,
    /// Words to look for in the topic, subtopic, keywords and text; every word must be found.
    pub query: Option<String>,
}

impl Filter {
    fn allows(&self, q: &Question) -> bool {
        self.topic.as_ref().is_none_or(|t| q.topic.eq_ignore_ascii_case(t))
            && self.difficulty.is_none_or(|d| q.difficulty == d)
            && self.skill.as_deref().is_none_or(|k| q.skill_key() == k)
            && self.year.as_deref().is_none_or(|y| q.year_group().eq_ignore_ascii_case(y))
            && self.query.as_deref().is_none_or(|w| matches_words(q, w))
    }
}

/// Whether every word of `words` appears in the question's searchable text (case and a plural
/// "s" do not matter).
pub fn matches_words(q: &Question, words: &str) -> bool {
    let year = q.year_group();
    let year_words = if year.is_empty() { String::new() } else { format!("{year} year {}", year.trim_start_matches('Y')) };
    let hay = format!("{} {} {} {} {} {} {}", q.topic, q.subtopic, q.keywords.join(" "), q.qtype, q.source, year_words, crate::latex::plain(&q.text))
        .to_lowercase();
    words.to_lowercase().split_whitespace().all(|w| {
        let stem = w.trim_end_matches('s');
        hay.contains(w) || (stem.len() >= 3 && hay.contains(stem))
    })
}

/// A skill a teacher can choose: one small step (or, for general questions, one subtopic), with
/// how many questions there are for it at each level.
#[derive(Clone, Debug, PartialEq)]
pub struct Skill {
    /// `Question::skill_key`, what a slot stores.
    pub key: String,
    pub name: String,
    pub topic: String,
    /// `Y7`… (empty for general questions).
    pub year: String,
    /// `Autumn`, `Spring`, `Summer` (empty when unknown).
    pub term: String,
    /// The block it belongs to, e.g. “Place value, ordering and rounding”.
    pub block: String,
    pub keywords: Vec<String>,
    /// Questions at support, core, stretch and exam style.
    pub counts: [usize; 4],
}

impl Skill {
    /// Words for a search: every word must be in the skill's name, block, topic, keywords or year.
    pub fn matches(&self, words: &str) -> bool {
        let year = if self.year.is_empty() { String::new() } else { format!("{} year {}", self.year, self.year.trim_start_matches('Y')) };
        let hay = format!("{} {} {} {} {} {}", self.name, self.block, self.topic, self.term, self.keywords.join(" "), year).to_lowercase();
        words.to_lowercase().split_whitespace().all(|w| {
            let stem = w.trim_end_matches('s');
            hay.contains(w) || (stem.len() >= 3 && hay.contains(stem))
        })
    }

    /// How well the search words fit the name: higher is better (used to put the likeliest first).
    pub fn relevance(&self, words: &str) -> usize {
        let name = self.name.to_lowercase();
        words.to_lowercase().split_whitespace().filter(|w| name.contains(w.trim_end_matches('s'))).count()
    }

    pub fn total(&self) -> usize {
        self.counts.iter().sum()
    }
}

pub fn parse(json: &str) -> Result<Bank, String> {
    let bank = parse_raw(json)?;
    validate(&bank)?;
    Ok(bank)
}

fn parse_raw(json: &str) -> Result<Bank, String> {
    if json.len() > MAX_BYTES {
        return Err("the question bank is too large".into());
    }
    let bank: Bank = serde_json::from_str(json).map_err(|e| format!("not a question bank: {e}"))?;
    if bank.version != 1 {
        return Err(format!("question bank version {} is not supported (expected 1)", bank.version));
    }
    Ok(bank)
}

/// Everything that can be wrong with a bank: repeated ids, empty or over-long text, a level
/// outside 1 to 4, and every template (it must make good questions for 25 different seeds).
fn validate(bank: &Bank) -> Result<(), String> {
    if bank.questions.len() > MAX_QUESTIONS {
        return Err("too many questions in the bank".into());
    }
    let mut seen = std::collections::HashSet::new();
    for q in &bank.questions {
        if q.id.is_empty() || !seen.insert(q.id.as_str()) {
            return Err(format!("question id “{}” is empty or repeated", q.id));
        }
        if q.text.trim().is_empty() || q.text.chars().count() > MAX_TEXT {
            return Err(format!("question {} has no text, or too much", q.id));
        }
        if !DIFFICULTIES.contains(&q.difficulty) {
            return Err(format!("question {} has difficulty {} (use 1 support, 2 core, 3 stretch or 4 exam style)", q.id, q.difficulty));
        }
    }
    let mut seen_t = std::collections::HashSet::new();
    for tpl in &bank.templates {
        if !seen_t.insert(tpl.id.as_str()) || seen.contains(tpl.id.as_str()) {
            return Err(format!("template id “{}” is empty or repeated", tpl.id));
        }
        tpl.check()?;
    }
    Ok(())
}

/// Reads pasted text as a bank, for a teacher who copies an AI's answer into the app. It looks
/// past a Markdown fence and chatter around the JSON, accepts a bare list of templates or
/// questions, and when the text is not valid JSON says where and what usually causes it.
pub fn parse_loose(input: &str) -> Result<Bank, String> {
    let text = input.replace(['\u{201C}', '\u{201D}'], "\"").replace(['\u{2018}', '\u{2019}'], "'");
    let open = text.find(['{', '[']).ok_or("There is no question data in that text (it should start with { ).")?;
    let close = text.rfind(['}', ']']).ok_or("The pasted text stops before the end of the question data: the last } or ] is missing.")?;
    if close < open {
        return Err("The pasted text is cut off: the closing } or ] is missing.".into());
    }
    let text = &text[open..=close];
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| {
        let line = text.lines().nth(e.line().saturating_sub(1)).unwrap_or("").trim();
        let near: String = line.chars().take(90).collect();
        format!(
            "The text is not valid JSON: {e}. Look near: “{near}”. Common causes: a missing comma between items, a }} where a ] belongs (or the other way round), a missing quote, or an extra bracket at the end. Ask the AI to fix that line."
        )
    })?;
    let value = match value {
        serde_json::Value::Array(items) => {
            let (templates, questions): (Vec<_>, Vec<_>) = items.into_iter().partition(|i| i.get("vars").is_some());
            serde_json::json!({"version": 1, "templates": templates, "questions": questions})
        }
        mut v => {
            if v.get("version").is_none() {
                v["version"] = serde_json::json!(1);
            }
            v
        }
    };
    let bank: Bank = serde_json::from_value(value).map_err(|e| format!("Some of it is not in the question format: {e}"))?;
    if bank.questions.is_empty() && bank.templates.is_empty() {
        return Err("There are no questions in that text.".into());
    }
    parse(&serde_json::to_string(&bank).map_err(|e| e.to_string())?)
}

/// The built-in bank: every file in `STARTER_FILES` merged and checked once (the first call does
/// the work; later calls copy the result).
pub fn starter() -> Result<Bank, String> {
    static CACHE: std::sync::OnceLock<Result<Bank, String>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut all = Bank { version: 1, note: String::new(), questions: vec![], templates: vec![] };
            for (i, file) in STARTER_FILES.iter().enumerate() {
                let b = parse_raw(file).map_err(|e| format!("built-in question file {}: {e}", i + 1))?;
                if all.note.is_empty() {
                    all.note = b.note;
                }
                all.questions.extend(b.questions);
                all.templates.extend(b.templates);
            }
            validate(&all)?;
            Ok(all)
        })
        .clone()
}

impl Bank {
    /// Distinct topics in the order they first appear.
    pub fn topics(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let topics = self.questions.iter().map(|q| &q.topic).chain(self.templates.iter().map(|t| &t.topic));
        for t in topics {
            if !out.contains(t) {
                out.push(t.clone());
            }
        }
        out
    }

    /// The skills of the questions in this bank (call it on `instantiate(…)`), in the order they
    /// first appear, which for the built-in files is the order of the scheme of work.
    pub fn skills(&self) -> Vec<Skill> {
        let mut out: Vec<Skill> = vec![];
        for q in self.questions.iter().filter(|q| q.usable()) {
            let key = q.skill_key();
            let at = match out.iter().position(|s| s.key == key) {
                Some(i) => i,
                None => {
                    // “Y7 Autumn B4 Place value, ordering and rounding · S3 Compare…”
                    let head = q.source.split(" · ").next().unwrap_or("");
                    let mut words = head.split_whitespace();
                    let (_, term, _) = (words.next(), words.next().unwrap_or(""), words.next());
                    let block = words.collect::<Vec<_>>().join(" ");
                    let (term, block) = if q.source.is_empty() { (String::new(), q.topic.clone()) } else { (term.to_string(), block) };
                    out.push(Skill {
                        key,
                        name: q.subtopic.clone(),
                        topic: q.topic.clone(),
                        year: q.year_group(),
                        term,
                        block,
                        keywords: vec![],
                        counts: [0; 4],
                    });
                    out.len() - 1
                }
            };
            let skill = &mut out[at];
            if let Some(c) = skill.counts.get_mut(usize::from(q.difficulty).saturating_sub(1)) {
                *c += 1;
            }
            for k in &q.keywords {
                if !skill.keywords.contains(k) && skill.keywords.len() < 24 {
                    skill.keywords.push(k.clone());
                }
            }
        }
        out
    }

    /// The bank as a plain list of questions for `seed`: every template made into one question
    /// (new numbers for each seed). A template that cannot be made is left out.
    pub fn instantiate(&self, seed: u64) -> Bank {
        let mut questions = self.questions.clone();
        questions.extend(self.templates.iter().filter_map(|t| t.instantiate(seed).ok()));
        Bank { version: self.version, note: self.note.clone(), questions, templates: vec![] }
    }

    /// The starter bank without the questions in `removed`, followed by the teacher's own.
    pub fn merged(starter: Bank, removed: &[String], mine: &[Question], mine_templates: &[crate::template::Template]) -> Bank {
        let mut templates: Vec<crate::template::Template> = starter.templates.into_iter().filter(|t| !removed.contains(&t.id)).collect();
        for t in mine_templates {
            if !templates.iter().any(|o| o.id == t.id) {
                templates.push(t.clone());
            }
        }
        let mut questions: Vec<Question> = starter.questions.into_iter().filter(|q| !removed.contains(&q.id)).collect();
        for q in mine {
            if !questions.iter().any(|o| o.id == q.id) {
                questions.push(q.clone());
            }
        }
        Bank { version: 1, note: starter.note, questions, templates }
    }

    /// An id for a new question of the teacher's own that no question in the bank has.
    pub fn new_id(&self, mine: &[Question]) -> String {
        (1..).map(|n| format!("my-{n:03}")).find(|id| !self.questions.iter().chain(mine).any(|q| &q.id == id)).unwrap_or_else(|| "my-0".into())
    }

    /// Serialize this bank back to indented JSON (for export and user-bank persistence).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Distinct subtopics for a given topic, in the order they first appear.
    pub fn subtopics_for(&self, topic: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for q in &self.questions {
            if q.topic.eq_ignore_ascii_case(topic) && !out.contains(&q.subtopic) {
                out.push(q.subtopic.clone());
            }
        }
        out
    }

    /// `n` questions that match `filter`, chosen by `seed` (the same seed gives the same four, a
    /// new seed is a reroll). With no topic asked for, the pick spreads across topics first so a
    /// starter revisits several areas rather than one. Questions that need a figure are skipped.
    pub fn pick(&self, filter: &Filter, seed: u64, n: usize) -> Vec<&Question> {
        let mut pool: Vec<&Question> = self.questions.iter().filter(|q| q.usable()).filter(|q| filter.allows(q)).collect();
        let mut rng = seed;
        for i in (1..pool.len()).rev() {
            let j = (splitmix(&mut rng) % (i as u64 + 1)) as usize;
            pool.swap(i, j);
        }
        if filter.topic.is_some() {
            pool.truncate(n);
            return pool;
        }
        let mut out: Vec<&Question> = Vec::new();
        let mut topics: Vec<&str> = Vec::new();
        for q in &pool {
            if out.len() < n && !topics.contains(&q.topic.as_str()) {
                topics.push(q.topic.as_str());
                out.push(q);
            }
        }
        for q in &pool {
            if out.len() < n && !out.iter().any(|o| o.id == q.id) {
                out.push(q);
            }
        }
        out
    }
}

impl Bank {
    /// One question matching `filter` that is not in `exclude`, chosen by `seed`.
    pub fn pick_one(&self, filter: &Filter, seed: u64, exclude: &[String]) -> Option<&Question> {
        let pool: Vec<&Question> = self.questions.iter().filter(|q| q.usable() && !exclude.contains(&q.id)).filter(|q| filter.allows(q)).collect();
        if pool.is_empty() {
            return None;
        }
        let mut rng = seed;
        pool.get((splitmix(&mut rng) % pool.len() as u64) as usize).copied()
    }
}

impl Bank {
    /// Every usable question matching `filter` (figure questions are skipped).
    pub fn candidates(&self, filter: &Filter) -> Vec<&Question> {
        self.questions.iter().filter(|q| q.usable()).filter(|q| filter.allows(q)).collect()
    }

    /// One different question for each filter, in the filters' order, chosen by `seed`, or `None`
    /// when no such set exists. It fills the most tightly constrained slots first and backtracks,
    /// so a slot left on "any" can never take the only question another slot needs.
    pub fn assign(&self, filters: &[Filter], seed: u64) -> Option<Vec<&Question>> {
        let mut pools: Vec<Vec<&Question>> = filters.iter().map(|f| self.candidates(f)).collect();
        let mut rng = seed;
        for pool in &mut pools {
            for i in (1..pool.len()).rev() {
                let j = (splitmix(&mut rng) % (i as u64 + 1)) as usize;
                pool.swap(i, j);
            }
        }
        let mut order: Vec<usize> = (0..filters.len()).collect();
        order.sort_by_key(|&i| (pools[i].len(), i));
        let mut chosen: Vec<Option<&Question>> = vec![None; filters.len()];
        fn go<'a>(k: usize, order: &[usize], pools: &[Vec<&'a Question>], chosen: &mut Vec<Option<&'a Question>>) -> bool {
            let Some(&slot) = order.get(k) else { return true };
            for q in pools.get(slot).map(|p| p.as_slice()).unwrap_or_default() {
                if chosen.iter().flatten().any(|c| c.id == q.id) {
                    continue;
                }
                chosen[slot] = Some(q);
                if go(k + 1, order, pools, chosen) {
                    return true;
                }
                chosen[slot] = None;
            }
            false
        }
        go(0, &order, &pools, &mut chosen).then(|| chosen.into_iter().flatten().collect())
    }
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_starter_bank_is_valid_and_has_ks3_ks4_questions() {
        let bank = starter().unwrap();
        let usable = bank.questions.iter().filter(|q| q.usable()).count();
        // The KS3/KS4 curriculum bank has many more than the 20 placeholder questions.
        assert!(usable >= 18, "expected at least 18 usable questions, got {usable}");
        assert!(bank.topics().len() >= 4);
    }

    #[test]
    fn bank_round_trips_through_to_json() {
        let bank = starter().unwrap();
        let json = bank.to_json();
        let bank2 = parse(&json).unwrap();
        assert_eq!(bank.questions.len(), bank2.questions.len());
        assert_eq!(bank.questions[0].id, bank2.questions[0].id);
    }

    #[test]
    fn the_same_seed_gives_the_same_questions_and_a_new_seed_a_reroll() {
        let bank = starter().unwrap();
        let ids = |seed| bank.pick(&Filter::default(), seed, 4).iter().map(|q| q.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(7), ids(7));
        assert!((1..20).any(|s| ids(s) != ids(7)), "some seed changes the four");
    }

    #[test]
    fn topic_and_difficulty_narrow_the_pick_and_figures_are_skipped() {
        let bank = starter().unwrap();
        let f = Filter { topic: Some("geometry".into()), difficulty: Some(1), query: None, ..Default::default() };
        let got = bank.pick(&f, 3, 4);
        assert!(!got.is_empty());
        assert!(got.iter().all(|q| q.topic == "Geometry" && q.difficulty == 1 && q.usable()));
        assert!(bank.pick(&Filter::default(), 1, 50).iter().all(|q| q.usable()));
    }

    #[test]
    fn with_no_topic_the_four_cover_four_topics() {
        let bank = starter().unwrap();
        for seed in 0..10 {
            let picked = bank.pick(&Filter::default(), seed, 4);
            let mut topics: Vec<_> = picked.iter().map(|q| q.topic.clone()).collect();
            topics.sort();
            topics.dedup();
            assert_eq!(topics.len(), 4, "seed {seed}");
        }
    }

    #[test]
    fn pick_one_honours_its_filter_and_never_repeats_an_excluded_question() {
        let bank = starter().unwrap();
        let f = Filter { topic: Some("Algebra".into()), difficulty: Some(1), query: None, ..Default::default() };
        let mut used: Vec<String> = vec![];
        while let Some(q) = bank.pick_one(&f, used.len() as u64, &used) {
            assert!(q.topic == "Algebra" && q.difficulty == 1);
            assert!(!used.contains(&q.id));
            used.push(q.id.clone());
        }
        assert!(!used.is_empty(), "there are algebra level-1 questions");
        assert!(bank.pick_one(&Filter { topic: Some("Origami".into()), difficulty: None, query: None, ..Default::default() }, 1, &[]).is_none());
    }

    #[test]
    fn a_loose_slot_never_takes_the_only_question_a_strict_slot_needs() {
        let bank = starter().unwrap();
        let strict = Filter { topic: Some("Ratio".into()), difficulty: Some(3), query: None, ..Default::default() };
        assert_eq!(bank.candidates(&strict).len(), 1, "the bank has exactly one ratio stretch question");
        for seed in 0..300 {
            let got = bank.assign(&[Filter::default(), strict.clone(), Filter::default(), Filter::default()], seed).expect("a set exists");
            assert_eq!(got[1].id, "rat-006");
            let mut ids: Vec<_> = got.iter().map(|q| q.id.clone()).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), 4, "all different");
        }
        let impossible = bank.assign(&[strict.clone(), strict], 1);
        assert!(impossible.is_none(), "two questions cannot both be the one ratio stretch question");
    }

    #[test]
    fn every_built_in_template_makes_good_questions_for_many_seeds() {
        let bank = starter().unwrap();
        for t in &bank.templates {
            for seed in 0..300u64 {
                let q = t.instantiate(seed).unwrap_or_else(|e| panic!("{} seed {seed}: {e}", t.id));
                assert!(!q.text.contains("[[") && !q.answer.contains("[["), "{} seed {seed}", t.id);
                assert!(q.text.chars().count() <= 600, "{} seed {seed}: too long", t.id);
            }
        }
    }

    #[test]
    fn skills_group_the_questions_by_small_step_with_a_count_for_each_level() {
        let bank = starter().unwrap().instantiate(0);
        let skills = bank.skills();
        let two_step = skills.iter().find(|s| s.name == "Solve 2-step equations").expect("a skill for the step");
        assert_eq!(two_step.counts, [1, 1, 1, 1], "support, core, stretch and exam style");
        assert_eq!((two_step.year.as_str(), two_step.term.as_str()), ("Y7", "Autumn"));
        assert_eq!(two_step.block, "Expressions and equations");
        // Skills come in the order of the scheme of work: sequences before equations.
        let at = |n: &str| skills.iter().position(|s| s.name == n).unwrap();
        assert!(at("Describe and continue sequences") < at("Solve 2-step equations"));
        // General questions without a step are grouped by topic and subtopic.
        assert!(skills.iter().any(|s| s.year.is_empty() && s.block == "Geometry"));
        // Searching finds skills by name, block, keyword and year.
        let find = |w: &str| skills.iter().filter(|s| s.matches(w)).map(|s| s.name.clone()).collect::<Vec<_>>();
        assert!(find("equations").contains(&"Solve 2-step equations".to_string()));
        assert!(find("year 7 rounding").contains(&"Round to decimal places".to_string()));
        assert!(find("y7 mean").contains(&"Mean".to_string()));
        assert!(find("origami").is_empty());
        // A slot filter by skill and year keeps only that step.
        let f = Filter { skill: Some(two_step.key.clone()), difficulty: Some(3), ..Default::default() };
        assert_eq!(bank.candidates(&f).len(), 1);
        let y = Filter { year: Some("Y7".into()), ..Default::default() };
        assert!(bank.candidates(&y).iter().all(|q| q.year_group() == "Y7"));
    }

    #[test]
    fn every_built_in_template_gives_different_questions_not_the_same_one() {
        // Most make dozens; a few cover a small topic (eighths, quarters, powers of 10) and make
        // fewer, but never fewer than three.
        let bank = starter().unwrap();
        for t in &bank.templates {
            let seen: std::collections::HashSet<(String, String)> =
                (0..300u64).map(|s| t.instantiate(s).map(|q| (q.text, q.answer)).unwrap()).collect();
            assert!(seen.len() >= 3, "{} makes only {} different questions", t.id, seen.len());
        }
    }

    #[test]
    fn every_scheme_of_work_step_has_questions_at_all_four_levels() {
        let bank = starter().unwrap();
        let mut steps: std::collections::BTreeMap<String, std::collections::BTreeSet<u8>> = Default::default();
        for t in &bank.templates {
            if t.source.starts_with("Y7 ") || t.source.starts_with("Y8 ") {
                steps.entry(t.source.clone()).or_default().insert(t.difficulty);
            }
        }
        assert!(steps.len() >= 280, "the Year 7 and 8 steps written so far are all there: {}", steps.len());
        for (step, levels) in &steps {
            assert_eq!(levels.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3, 4], "{step} needs support, core, stretch and exam style");
        }
    }

    #[test]
    fn pasted_text_is_read_past_fences_and_chatter_and_errors_say_where() {
        let good = "Here you go!\n```json\n{\"version\":1,\"templates\":[{\"id\":\"t\",\"topic\":\"Number\",\"difficulty\":1,\"vars\":[[\"a\",{\"int\":[1,5]}]],\"text\":\"Add [[a]] and 1.\",\"answer\":\"[[a + 1]]\"}]}\n```\nHope that helps";
        assert_eq!(parse_loose(good).unwrap().templates.len(), 1);
        let bare = r#"[{"id":"q","topic":"Number","difficulty":1,"text":"1 + 1","answer":"2"}]"#;
        assert_eq!(parse_loose(bare).unwrap().questions.len(), 1);
        // The mistake a local AI made: a } where the ] belongs.
        let broken = "{\"version\":1,\"templates\":[\n{\"id\":\"t\",\"vars\":[[\"v\",{\"pick\":[\"x\"]}}],\"text\":\"a\",\"answer\":\"b\"}]}";
        let e = parse_loose(broken).unwrap_err();
        assert!(e.contains("not valid JSON") && e.contains("line 2") && e.contains("pick"), "{e}");
        assert!(parse_loose("nothing here").is_err());
        assert!(parse_loose("{\"questions\": []}").unwrap_err().contains("no questions"));
    }

    #[test]
    fn merging_hides_removed_starter_questions_and_adds_the_teachers_own() {
        let starter = starter().unwrap();
        let first = starter.questions[0].id.clone();
        let n = starter.questions.len();
        let mine = Question {
            id: "my-001".into(),
            topic: "Number".into(),
            subtopic: String::new(),
            difficulty: 2,
            keywords: vec![],
            qtype: String::new(),
            source: String::new(),
            year: String::new(),
            text: "Work out $\\frac{1}{2} + \\frac{1}{3}$".into(),
            answer: "$\\frac{5}{6}$".into(),
            needs_diagram: false,
            diagram: None,
        };
        let merged = Bank::merged(starter, std::slice::from_ref(&first), std::slice::from_ref(&mine), &[]);
        assert_eq!(merged.questions.len(), n);
        assert!(merged.questions.iter().all(|q| q.id != first));
        assert_eq!(merged.questions.last().unwrap().id, "my-001");
        assert_eq!(merged.new_id(&[]), "my-002");
        assert!(parse(&merged.to_json()).is_ok());
    }

    #[test]
    fn a_bad_bank_is_refused_with_a_reason() {
        assert!(parse("{}").is_err());
        assert!(parse(r#"{"version":2,"questions":[]}"#).unwrap_err().contains("version"));
        let dup = r#"{"version":1,"questions":[
            {"id":"a","topic":"T","difficulty":1,"text":"x"},{"id":"a","topic":"T","difficulty":1,"text":"y"}]}"#;
        assert!(parse(dup).unwrap_err().contains("repeated"));
        let diff = r#"{"version":1,"questions":[{"id":"a","topic":"T","difficulty":9,"text":"x"}]}"#;
        assert!(parse(diff).unwrap_err().contains("difficulty"));
    }
}
