//! The question bank behind the Get Ready starter slide.
//!
//! The bank is a JSON file: `{"version": 1, "questions": [Question, …]}`. The format is documented
//! in `docs/design/ux-review.md` (area E). A starter bank ships in the binary
//! (`assets/questions/bank.json`); a teacher's own bank is read from a path on request.
//! Nothing here touches the network or the pupils' data: questions are plain text.
//! LaTeX between `$…$` in `text` and `answer` is rendered as real equations on the slide.

use serde::{Deserialize, Serialize};

/// The bank compiled into the app.
pub const STARTER: &str = include_str!("../../../assets/questions/bank.json");

const MAX_BYTES: usize = 4 << 20;
const MAX_QUESTIONS: usize = 20_000;
const MAX_TEXT: usize = 600;
/// The figure kinds the Get Ready slide can draw.
pub const DRAWABLE: &[&str] = &["triangle-angles", "isosceles-triangle", "parallel-lines"];

/// Whether a question's `diagram` is one the slide can draw.
pub fn is_drawable(d: &serde_json::Value) -> bool {
    d.get("kind").and_then(|k| k.as_str()).is_some_and(|k| DRAWABLE.contains(&k))
}

impl Question {
    /// Usable on a slide: it needs no figure, or its figure can be drawn.
    pub fn usable(&self) -> bool {
        !self.needs_diagram || self.diagram.as_ref().is_some_and(is_drawable)
    }
}

pub const DIFFICULTIES: std::ops::RangeInclusive<u8> = 1..=3;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: String,
    pub topic: String,
    #[serde(default, alias = "subtopics")]
    pub subtopic: String,
    /// 1 = secure, 2 = core, 3 = stretch.
    pub difficulty: u8,
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
    pub questions: Vec<Question>,
}

#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub topic: Option<String>,
    pub difficulty: Option<u8>,
}

pub fn parse(json: &str) -> Result<Bank, String> {
    if json.len() > MAX_BYTES {
        return Err("the question bank is too large".into());
    }
    let bank: Bank = serde_json::from_str(json).map_err(|e| format!("not a question bank: {e}"))?;
    if bank.version != 1 {
        return Err(format!("question bank version {} is not supported (expected 1)", bank.version));
    }
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
            return Err(format!("question {} has difficulty {} (use 1, 2 or 3)", q.id, q.difficulty));
        }
    }
    Ok(bank)
}

pub fn starter() -> Result<Bank, String> {
    parse(STARTER)
}

impl Bank {
    /// Distinct topics in the order they first appear.
    pub fn topics(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for q in &self.questions {
            if !out.contains(&q.topic) {
                out.push(q.topic.clone());
            }
        }
        out
    }

    /// The starter bank without the questions in `removed`, followed by the teacher's own.
    pub fn merged(starter: Bank, removed: &[String], mine: &[Question]) -> Bank {
        let mut questions: Vec<Question> = starter.questions.into_iter().filter(|q| !removed.contains(&q.id)).collect();
        for q in mine {
            if !questions.iter().any(|o| o.id == q.id) {
                questions.push(q.clone());
            }
        }
        Bank { version: 1, note: starter.note, questions }
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
        let mut pool: Vec<&Question> = self
            .questions
            .iter()
            .filter(|q| q.usable())
            .filter(|q| filter.topic.as_ref().is_none_or(|t| q.topic.eq_ignore_ascii_case(t)))
            .filter(|q| filter.difficulty.is_none_or(|d| q.difficulty == d))
            .collect();
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
        let pool: Vec<&Question> = self
            .questions
            .iter()
            .filter(|q| q.usable() && !exclude.contains(&q.id))
            .filter(|q| filter.topic.as_ref().is_none_or(|t| q.topic.eq_ignore_ascii_case(t)))
            .filter(|q| filter.difficulty.is_none_or(|d| q.difficulty == d))
            .collect();
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
        self.questions
            .iter()
            .filter(|q| q.usable())
            .filter(|q| filter.topic.as_ref().is_none_or(|t| q.topic.eq_ignore_ascii_case(t)))
            .filter(|q| filter.difficulty.is_none_or(|d| q.difficulty == d))
            .collect()
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
        let f = Filter { topic: Some("geometry".into()), difficulty: Some(1) };
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
        let f = Filter { topic: Some("Algebra".into()), difficulty: Some(1) };
        let mut used: Vec<String> = vec![];
        while let Some(q) = bank.pick_one(&f, used.len() as u64, &used) {
            assert!(q.topic == "Algebra" && q.difficulty == 1);
            assert!(!used.contains(&q.id));
            used.push(q.id.clone());
        }
        assert!(!used.is_empty(), "there are algebra level-1 questions");
        assert!(bank.pick_one(&Filter { topic: Some("Origami".into()), difficulty: None }, 1, &[]).is_none());
    }

    #[test]
    fn a_loose_slot_never_takes_the_only_question_a_strict_slot_needs() {
        let bank = starter().unwrap();
        let strict = Filter { topic: Some("Ratio".into()), difficulty: Some(3) };
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
    fn merging_hides_removed_starter_questions_and_adds_the_teachers_own() {
        let starter = starter().unwrap();
        let first = starter.questions[0].id.clone();
        let n = starter.questions.len();
        let mine = Question {
            id: "my-001".into(),
            topic: "Number".into(),
            subtopic: String::new(),
            difficulty: 2,
            text: "Work out $\\frac{1}{2} + \\frac{1}{3}$".into(),
            answer: "$\\frac{5}{6}$".into(),
            needs_diagram: false,
            diagram: None,
        };
        let merged = Bank::merged(starter, std::slice::from_ref(&first), std::slice::from_ref(&mine));
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
