//! The question bank behind the Get Ready starter slide.
//!
//! The bank is a JSON file: `{"version": 1, "questions": [Question, …]}`. The format is documented
//! in `docs/design/ux-review.md` (area E). A starter bank ships in the binary
//! (`assets/questions/bank.json`); a teacher's own bank is read from a path on request.
//! Nothing here touches the network or the pupils' data: questions are plain text.

use serde::Deserialize;

/// The bank compiled into the app.
pub const STARTER: &str = include_str!("../../../assets/questions/bank.json");

const MAX_BYTES: usize = 4 << 20;
const MAX_QUESTIONS: usize = 20_000;
const MAX_TEXT: usize = 600;
pub const DIFFICULTIES: std::ops::RangeInclusive<u8> = 1..=3;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: String,
    pub topic: String,
    #[serde(default)]
    pub subtopic: String,
    /// 1 = secure, 2 = core, 3 = stretch.
    pub difficulty: u8,
    pub text: String,
    /// Shown in the slide's speaker notes, never on the slide.
    #[serde(default)]
    pub answer: String,
    /// The question is meaningless without its figure. Such questions are skipped until diagrams
    /// are built (see the review: the figure is parameterised by `diagram`).
    #[serde(default)]
    pub needs_diagram: bool,
    /// Free-form figure description, reserved for the diagram follow-up.
    #[serde(default)]
    pub diagram: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Bank {
    pub version: u32,
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

    /// `n` questions that match `filter`, chosen by `seed` (the same seed gives the same four, a
    /// new seed is a reroll). With no topic asked for, the pick spreads across topics first so a
    /// starter revisits several areas rather than one. Questions that need a figure are skipped.
    pub fn pick(&self, filter: &Filter, seed: u64, n: usize) -> Vec<&Question> {
        let mut pool: Vec<&Question> = self
            .questions
            .iter()
            .filter(|q| !q.needs_diagram)
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
            .filter(|q| !q.needs_diagram && !exclude.contains(&q.id))
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
    fn the_starter_bank_is_valid_and_has_about_twenty_questions() {
        let bank = starter().unwrap();
        let usable = bank.questions.iter().filter(|q| !q.needs_diagram).count();
        assert!((18..=24).contains(&usable), "{usable}");
        assert!(bank.topics().len() >= 4);
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
        assert!(got.iter().all(|q| q.topic == "Geometry" && q.difficulty == 1 && !q.needs_diagram));
        assert!(bank.pick(&Filter::default(), 1, 50).iter().all(|q| !q.needs_diagram));
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
