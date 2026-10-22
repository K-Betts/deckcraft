//! A department's curriculum, and the Get Ready starter planned from it.
//!
//! A curriculum file says what each year group is taught, route by route (Core, Support, Higher,
//! Foundation…), unit by unit, with the week each unit starts and its objectives:
//!
//! ```json
//! {"version": 1, "name": "Our maths 2026-27", "courses": [
//!   {"year": "Y7", "route": "Core", "units": [
//!     {"title": "Fractions", "start": "2027-01-04", "weeks": 4,
//!      "objectives": [{"text": "Add and subtract fractions"}, …],
//!      "prior": [{"text": "Converting fractions to decimals"}]}]}]}
//! ```
//!
//! A teacher says what they have just taught (year, route, unit, objective) and [`plan`] picks what
//! the four questions should be about: that objective (Last lesson), an earlier one in the unit
//! (Last week), one from the unit before (Last topic) and one from the unit's prior learning or the
//! year before (Last year). Objectives are matched to the question bank's skills by their words
//! ([`Matcher`]); an objective may name its skills (`skills`) or the words to match (`search`)
//! when the words alone pick the wrong ones. The file is the school's own: none ships in the app.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::questions::{Bank, Filter, Question, Skill};

const MAX_BYTES: usize = 8 << 20;
const MAX_COURSES: usize = 100;
const MAX_UNITS: usize = 200;
const MAX_OBJECTIVES: usize = 200;
const MAX_TEXT: usize = 600;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Curriculum {
    pub version: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    pub courses: Vec<Course>,
}

/// One year group on one route, e.g. Year 7 Core.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Course {
    /// `Y7` to `Y11` (or `Y12`, `Y13`).
    pub year: String,
    /// `Core`, `Support`, `Higher`, `Foundation`… (empty when a year has one route).
    #[serde(default)]
    pub route: String,
    pub units: Vec<Unit>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Unit {
    pub title: String,
    /// Where the unit comes from, e.g. `Unit 4` or `Sparx`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    /// The Monday of its first week, `YYYY-MM-DD` (empty when not known).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub start: String,
    /// How many teaching weeks it takes.
    #[serde(default)]
    pub weeks: u32,
    #[serde(default)]
    pub objectives: Vec<Objective>,
    /// What pupils should already know: practised as the starter's last question.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prior: Vec<Objective>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Objective {
    pub text: String,
    /// Skill keys (`Question::skill_key`) to use instead of matching the words.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    /// Words to match instead of `text`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub search: String,
}

pub fn parse(json: &str) -> Result<Curriculum, String> {
    if json.len() > MAX_BYTES {
        return Err("the curriculum file is too large".into());
    }
    let c: Curriculum = serde_json::from_str(json).map_err(|e| format!("not a curriculum file: {e}"))?;
    if c.version != 1 {
        return Err(format!("curriculum version {} is not supported (expected 1)", c.version));
    }
    if c.courses.is_empty() {
        return Err("the curriculum has no courses".into());
    }
    if c.courses.len() > MAX_COURSES {
        return Err("too many courses in the curriculum".into());
    }
    for course in &c.courses {
        let name = course.name();
        if course.year.trim().is_empty() {
            return Err("a course has no year group".into());
        }
        if course.units.len() > MAX_UNITS {
            return Err(format!("{name} has too many units"));
        }
        for u in &course.units {
            if u.title.trim().is_empty() || u.title.chars().count() > MAX_TEXT {
                return Err(format!("{name}: a unit has no title, or too long a one"));
            }
            if !u.start.is_empty() && !is_date(&u.start) {
                return Err(format!("{name}, {}: start “{}” is not a date like 2027-01-04", u.title, u.start));
            }
            if u.objectives.len() + u.prior.len() > MAX_OBJECTIVES {
                return Err(format!("{name}, {}: too many objectives", u.title));
            }
            for o in u.objectives.iter().chain(&u.prior) {
                if o.text.trim().is_empty() || o.text.chars().count() > MAX_TEXT || o.search.chars().count() > MAX_TEXT {
                    return Err(format!("{name}, {}: an objective is empty or too long", u.title));
                }
            }
        }
    }
    Ok(c)
}

/// `YYYY-MM-DD`, which also sorts as text.
fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-' && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// Today's date as `YYYY-MM-DD` from days since 1970-01-01 (the UI reads the clock; the engine
/// stays deterministic).
pub fn date_from_days(days: i64) -> String {
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 for a `YYYY-MM-DD` date (the inverse of [`date_from_days`]).
pub fn days_from_date(s: &str) -> Option<i64> {
    if !is_date(s) {
        return None;
    }
    let (y, m, d): (i64, i64, i64) = (s[0..4].parse().ok()?, s[5..7].parse().ok()?, s[8..10].parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

impl Course {
    /// A guess at the objective a class has just been taught on `date`: as far through the unit's
    /// objectives as the date is through its weeks (the last once the unit is over).
    pub fn objective_on(&self, unit: usize, date: &str) -> usize {
        let Some(u) = self.units.get(unit) else { return 0 };
        let n = u.objectives.len();
        let (Some(start), Some(today)) = (days_from_date(&u.start), days_from_date(date)) else { return 0 };
        if n == 0 || today < start {
            return 0;
        }
        // Lessons in the week of `date` have not all happened: count to the day before.
        let length = i64::from(u.weeks.max(1)) * 7;
        let done = (today - start).min(length);
        ((done as f64 / length as f64 * n as f64) as usize).min(n - 1)
    }

    /// “Y7 Core”, or just “Y7”.
    pub fn name(&self) -> String {
        if self.route.is_empty() { self.year.clone() } else { format!("{} {}", self.year, self.route) }
    }

    /// The unit being taught on `date` (`YYYY-MM-DD`): the last one with objectives that has
    /// started by then. Before the first unit, the first.
    pub fn unit_on(&self, date: &str) -> Option<usize> {
        let taught = |u: &Unit| !u.objectives.is_empty();
        let started = self.units.iter().rposition(|u| taught(u) && !u.start.is_empty() && u.start.as_str() <= date);
        started.or_else(|| self.units.iter().position(taught))
    }
}

impl Curriculum {
    /// Year groups in the order they first appear.
    pub fn years(&self) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        for c in &self.courses {
            if !out.contains(&c.year) {
                out.push(c.year.clone());
            }
        }
        out
    }

    /// The routes of one year group, in file order.
    pub fn routes(&self, year: &str) -> Vec<String> {
        self.courses.iter().filter(|c| c.year == year).map(|c| c.route.clone()).collect()
    }

    /// The course for `year` on `route` (an unknown or empty route: the year's first course).
    pub fn course(&self, year: &str, route: &str) -> Option<&Course> {
        self.courses.iter().find(|c| c.year == year && c.route.eq_ignore_ascii_case(route)).or_else(|| self.courses.iter().find(|c| c.year == year))
    }

    /// The year before `year` on the closest route (the same name, else the first).
    fn year_before(&self, year: &str, route: &str) -> Option<&Course> {
        let n: u32 = year.trim_start_matches(['Y', 'y']).parse().ok()?;
        let prev = format!("Y{}", n.checked_sub(1)?);
        let same = |r: &str| r.eq_ignore_ascii_case(route);
        // Core and Higher, Support and Foundation, are the nearest routes across Key Stages.
        let near = |r: &str| match route.to_ascii_lowercase().as_str() {
            "higher" => r.eq_ignore_ascii_case("core"),
            "foundation" => r.eq_ignore_ascii_case("support"),
            "core" => r.eq_ignore_ascii_case("higher"),
            "support" => r.eq_ignore_ascii_case("foundation"),
            _ => false,
        };
        let of_year = || self.courses.iter().filter(|c| c.year == prev);
        of_year().find(|c| same(&c.route)).or_else(|| of_year().find(|c| near(&c.route))).or_else(|| of_year().next())
    }
}

// ---------------------------------------------------------------------------------------------
// Matching objectives to skills.

/// Words that say nothing about the maths.
const STOP: &[&str] = &[
    "a",
    "an",
    "the",
    "of",
    "and",
    "or",
    "to",
    "in",
    "on",
    "at",
    "with",
    "without",
    "use",
    "using",
    "used",
    "find",
    "finding",
    "work",
    "working",
    "out",
    "from",
    "for",
    "by",
    "into",
    "including",
    "include",
    "know",
    "knowing",
    "recognise",
    "recognising",
    "recognize",
    "identify",
    "identifying",
    "understand",
    "understanding",
    "be",
    "is",
    "are",
    "as",
    "their",
    "its",
    "it",
    "that",
    "this",
    "these",
    "those",
    "eg",
    "etc",
    "where",
    "when",
    "which",
    "what",
    "how",
    "other",
    "more",
    "than",
    "all",
    "any",
    "each",
    "between",
    "given",
    "involving",
    "problem",
    "problems",
    "solving",
    "core",
    "support",
    "stretch",
    "exam",
    "style",
    "year",
    "foundation",
    "higher",
    "e",
    "s",
    "x",
    "y",
    "n",
    "both",
    "positive",
    "meaning",
    "calculate",
    "calculating",
    "calculation",
    "calculations",
    "write",
    "writing",
    "context",
    "real",
    "life",
    "different",
    "such",
    "make",
    "made",
    "same",
    "one",
    "two",
    "three",
    "form",
    "simple",
    "type",
    "types",
    "notation",
];

/// Short forms in curriculum documents and what they stand for.
const SHORT: &[(&str, &str)] = &[
    ("pythag", "pythagoras"),
    ("trig", "trigonometry"),
    ("sohcahtoa", "trigonometry"),
    ("sin", "sine trigonometry"),
    ("cos", "cosine trigonometry"),
    ("tan", "tangent trigonometry"),
    ("trigonometric", "trigonometry"),
    ("constructing", "form"),
    ("fdp", "fraction decimal percentage"),
    ("quads", "quadrilateral"),
    ("quad", "quadrilateral"),
    ("hcf", "highest common factor"),
    ("lcm", "lowest common multiple"),
    ("ops", "operation"),
    ("calc", "calculator"),
    ("sig", "significant"),
    ("figs", "figure"),
    ("bidmas", "ordops"),
    ("bodmas", "ordops"),
    ("stats", "statistic"),
    ("prob", "probability"),
    ("vol", "volume"),
    ("sa", "surface area"),
    ("nets", "net"),
    ("cuboids", "cuboid"),
    ("squared", "square"),
    ("squaring", "square"),
    ("cubed", "cube"),
    ("cubing", "cube"),
    ("rooting", "root"),
    ("indices", "index power"),
    ("powers", "power index"),
    ("simultaneous", "simultaneous"),
    ("negatives", "negative directed"),
    ("negative", "negative directed"),
    ("directed", "directed negative"),
    ("probabilities", "probability"),
    ("percent", "percentage"),
    ("multiple", "multiplenum"),
    ("multiples", "multiplenum"),
    ("multiplication", "multiply"),
    ("addition", "add"),
    ("adding", "add"),
    ("subtraction", "subtract"),
    ("division", "divide"),
    ("radius", "radius circle vocabulary"),
    ("diameter", "diameter circle vocabulary"),
    ("subject", "subject rearrange"),
    ("rearranging", "rearrange subject"),
    ("rearrange", "rearrange subject"),
    ("vectors", "vector"),
    ("decimals", "decimal"),
];

/// Phrases that mean one thing, made one word before the text is split.
const PHRASES: &[(&str, &str)] = &[
    ("co-ordinate", "coordinate"),
    ("3-d", "threed"),
    ("3d", "threed"),
    ("2-d", "twod"),
    ("2d", "twod"),
    ("number lines", "numberline"),
    ("number line", "numberline"),
    ("order of operations", "ordops"),
    ("set up", "form"),
    ("non-right-angled", "nonrightangled"),
    ("non right-angled", "nonrightangled"),
    ("right-angled", "rightangled"),
    ("right angled", "rightangled"),
    ("position-to-term", "nth term"),
    ("position to term", "nth term"),
    ("term-to-term", "termtoterm"),
    ("term to term", "termtoterm"),
    ("like terms", "liketerms"),
];

/// A word reduced to a stem, so “factorising” meets “factorise” and “equations” “equation”.
fn stem(w: &str) -> String {
    let w = w.replace("ize", "ise");
    let mut s = w.as_str();
    for (end, keep) in [("isation", 2), ("ising", 2), ("ised", 2), ("ises", 2), ("ise", 2), ("ing", 0), ("ies", 0), ("es", 0), ("ed", 0)] {
        if s.len() >= end.len() + 3 && s.ends_with(end) {
            let cut = s.len() - end.len();
            let mut out = s[..cut].to_string();
            out.push_str(&end[..keep]);
            if end == "ies" {
                out.push('y');
            }
            return out.trim_end_matches('e').to_string();
        }
    }
    if s.len() > 3 && s.ends_with('s') && !s.ends_with("ss") && !s.ends_with("us") {
        s = &s[..s.len() - 1];
    }
    if s.len() > 4 { s.trim_end_matches('e').to_string() } else { s.to_string() }
}

/// The stems of the words that carry meaning in `text`.
pub fn words(text: &str) -> Vec<String> {
    let mut lower = crate::latex::plain(text).to_lowercase().replace('%', " percentage ");
    for (phrase, word) in PHRASES {
        lower = lower.replace(phrase, word);
    }
    let mut out: Vec<String> = vec![];
    for w in lower.split(|c: char| !c.is_alphanumeric()) {
        // Numbers say little, except powers of ten (“multiply by 10, 100 and 1000”).
        if w.is_empty() || (w.chars().all(|c| c.is_ascii_digit()) && !matches!(w, "10" | "100" | "1000")) {
            continue;
        }
        let expanded = SHORT.iter().find(|(k, _)| *k == w).map(|(_, v)| *v).unwrap_or(w);
        for e in expanded.split_whitespace() {
            if STOP.contains(&e) {
                continue;
            }
            let st = stem(e);
            if st.len() >= 2 && !out.contains(&st) {
                out.push(st);
            }
        }
    }
    out
}

/// Two stems are the same word, or one starts the other (“fraction”, “fractional”).
fn same_word(a: &str, b: &str) -> bool {
    a == b || (a.len().min(b.len()) >= 5 && (a.starts_with(b) || b.starts_with(a)))
}

struct SkillWords {
    name: Vec<String>,
    block: Vec<String>,
    rest: Vec<String>,
}

/// Matches objectives to the skills of a question bank.
pub struct Matcher {
    pub skills: Vec<Skill>,
    words: Vec<SkillWords>,
    idf: HashMap<String, f64>,
    rarest: f64,
}

impl Matcher {
    /// For the skills of `bank` (`Bank::instantiate(…).skills()`).
    pub fn new(skills: Vec<Skill>) -> Matcher {
        let words: Vec<SkillWords> = skills
            .iter()
            .map(|s| SkillWords {
                name: self::words(&s.name),
                block: self::words(&s.block),
                rest: self::words(&format!("{} {}", s.topic, s.keywords.join(" "))),
            })
            .collect();
        let mut df: HashMap<String, f64> = HashMap::new();
        for w in &words {
            let mut seen: Vec<&String> = vec![];
            for t in w.name.iter().chain(&w.block) {
                if !seen.contains(&t) {
                    seen.push(t);
                    *df.entry(t.clone()).or_default() += 1.0;
                }
            }
        }
        let n = words.len().max(1) as f64;
        let idf = df.into_iter().map(|(k, v)| (k, (n / v).ln() + 1.0)).collect();
        Matcher { skills, words, idf, rarest: n.ln() + 1.0 }
    }

    fn idf(&self, w: &str) -> f64 {
        // A word no skill uses is as rare as can be: an objective about something the bank does
        // not cover (“commutative laws”) should not match on its common words alone.
        self.idf.get(w).copied().unwrap_or(self.rarest)
    }

    /// The skills that best fit `text` for a course in `year` on `route`, best first, with a score
    /// from 0 to 1. Close seconds are kept (at most `max`), weak matches are not.
    pub fn best(&self, text: &str, year: &str, route: &str, max: usize) -> Vec<(usize, f64)> {
        self.best_in(text, "", year, route, max)
    }

    /// [`best`](Self::best) for an objective of a unit called `context`: the unit's words count
    /// for less than the objective's own, and only when some skill uses them (“Equations of
    /// parallel lines” in a unit on linear graphs is about graphs, not angles).
    pub fn best_in(&self, text: &str, context: &str, year: &str, route: &str, max: usize) -> Vec<(usize, f64)> {
        let mut obj: Vec<(String, f64)> = words(text).into_iter().map(|w| (w, 1.0)).collect();
        for w in words(context) {
            if self.idf.contains_key(&w) && !obj.iter().any(|(o, _)| same_word(o, &w)) {
                obj.push((w, CONTEXT));
            }
        }
        let total: f64 = obj.iter().map(|(w, f)| f * self.idf(w)).sum();
        if total <= 0.0 {
            return vec![];
        }
        let course_year: i32 = year.trim_start_matches(['Y', 'y']).parse().unwrap_or(0);
        let mut scored: Vec<(usize, f64)> = vec![];
        for (i, sw) in self.words.iter().enumerate() {
            let has = |list: &[String], w: &str| list.iter().any(|t| same_word(t, w));
            let mut got = 0.0;
            let mut own = false;
            for (w, f) in &obj {
                let weight = if has(&sw.name, w) {
                    1.0
                } else if has(&sw.block, w) {
                    0.45
                } else if has(&sw.rest, w) {
                    0.25
                } else {
                    0.0
                };
                own |= weight > 0.0 && *f == 1.0;
                got += f * weight * self.idf(w);
            }
            // The unit's words alone do not make a match.
            if got <= 0.0 || !own {
                continue;
            }
            let name_total: f64 = sw.name.iter().map(|w| self.idf(w)).sum();
            // How much of the skill's name the objective itself says (the unit's words do not count).
            let name_got: f64 = sw.name.iter().filter(|t| obj.iter().any(|(w, f)| *f == 1.0 && same_word(t, w))).map(|t| self.idf(t)).sum();
            let name_cover = if name_total > 0.0 { name_got / name_total } else { 0.0 };
            let mut score = 0.6 * (got / total) + 0.4 * name_cover;
            let s = &self.skills[i];
            // The course's own year first, then the years either side; general questions last.
            let skill_year: i32 = s.year.trim_start_matches('Y').parse().unwrap_or(0);
            score *= match (skill_year, (skill_year - course_year).abs()) {
                (0, _) => 0.82,
                (_, 0) => 1.0,
                (_, 1) => 0.94,
                (_, 2) => 0.9,
                _ => 0.85,
            };
            // Key Stage 4 skills belong to a tier.
            let tier = s.term.split_whitespace().next().unwrap_or("");
            if matches!(tier, "Foundation" | "Higher") && !route.is_empty() {
                let wants_higher = matches!(route.to_ascii_lowercase().as_str(), "higher" | "core");
                if (tier == "Higher") != wants_higher {
                    score *= 0.93;
                }
            }
            // Extension steps are for stretch, not the first choice.
            if s.name.ends_with("(E)") {
                score *= 0.92;
            }
            scored.push((i, score));
        }
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let best = scored.first().map(|s| s.1).unwrap_or(0.0);
        scored.into_iter().filter(|(_, sc)| *sc >= 0.35 && *sc >= best * 0.85).take(max).collect()
    }

    /// The skill keys for an objective of a unit called `context`: its own `skills` when it names
    /// any that exist, else the best matches for its `search` words or its text.
    pub fn skills_for(&self, o: &Objective, context: &str, year: &str, route: &str) -> Vec<String> {
        self.scored(o, context, year, route).0
    }

    /// [`skills_for`](Self::skills_for) with how well the best one fits (1 for skills the file names).
    pub fn scored(&self, o: &Objective, context: &str, year: &str, route: &str) -> (Vec<String>, f64) {
        let named: Vec<String> = o.skills.iter().filter(|k| self.skills.iter().any(|s| &s.key == *k)).cloned().collect();
        if !named.is_empty() {
            return (named, 1.0);
        }
        let text = if o.search.trim().is_empty() { &o.text } else { &o.search };
        let found = self.best_in(text, context, year, route, 3);
        let score = found.first().map(|f| f.1).unwrap_or(0.0);
        (found.into_iter().map(|(i, _)| self.skills[i].key.clone()).collect(), score)
    }
}

/// How much a word of the unit's title counts, against 1 for a word of the objective.
const CONTEXT: f64 = 0.6;

/// A match at least this good is taken at once; weaker ones only when nothing better turns up.
const GOOD_FIT: f64 = 0.45;

// ---------------------------------------------------------------------------------------------
// Planning a starter.

/// What one question of the starter is about.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Slot {
    /// `Last lesson`, `Last week`, `Last topic` or `Last year`.
    pub heading: String,
    /// The objective, as the curriculum words it.
    pub objective: String,
    /// The unit it comes from, e.g. “Y7 Core · Fractions”.
    pub from: String,
    /// Skill keys for it, best first (empty: nothing in the bank fits).
    pub skills: Vec<String>,
}

/// The card headings. The last becomes “Prior learning” or “Earlier this year” when its question
/// comes from the unit's prior learning or an earlier unit of this year rather than the year before.
pub const HEADINGS: [&str; 4] = ["Last lesson", "Last week", "Last topic", "Last year"];

/// Ends a slot's `from` when it is one of its unit's prior learning objectives.
const PRIOR: &str = " · prior learning";

/// The four slots of a starter for a class that has just been taught objective `objective` of
/// unit `unit` of `course`. `seed` varies the earlier objectives chosen. Each slot takes the first
/// objective on its list that some skill fits and that no other slot has used.
pub fn plan(cur: &Curriculum, course: &Course, unit: usize, objective: usize, m: &Matcher, seed: u64) -> Vec<Slot> {
    let Some(u) = course.units.get(unit) else { return vec![] };
    let objective = objective.min(u.objectives.len().saturating_sub(1));
    let mut rng = seed;
    let tag = |c: &Course, u: &Unit| format!("{} · {}", c.name(), u.title);
    type Cand<'a> = (&'a Objective, String, &'a Course, &'a str);
    fn shuffled<T>(mut v: Vec<T>, rng: &mut u64) -> Vec<T> {
        for i in (1..v.len()).rev() {
            let j = (splitmix(rng) % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
        v
    }
    let earlier = |n: usize| -> Vec<&Unit> { course.units[..unit].iter().rev().filter(|u| !u.objectives.is_empty()).skip(n).collect() };

    // Last lesson: what was just taught.
    let lesson: Vec<Cand> = u.objectives.get(objective).map(|o| (o, tag(course, u), course, u.title.as_str())).into_iter().collect();
    // Last week: the objectives just before it in this unit (the nearest four), then the unit before.
    let mut week: Vec<Cand> = shuffled(
        u.objectives[objective.saturating_sub(4)..objective].iter().map(|o| (o, tag(course, u), course, u.title.as_str())).collect(),
        &mut rng,
    );
    if let Some(prev) = earlier(0).first() {
        week.extend(prev.objectives.iter().rev().map(|o| (o, tag(course, prev), course, prev.title.as_str())));
    }
    // Last topic: the unit before, then the one before that.
    let mut topic: Vec<Cand> = vec![];
    for prev in earlier(0).into_iter().take(2) {
        topic.extend(shuffled(prev.objectives.iter().map(|o| (o, tag(course, prev), course, prev.title.as_str())).collect(), &mut rng));
    }
    // Last year: the unit's prior learning, then the year before, then units from longer ago.
    let mut year: Vec<Cand> = shuffled(u.prior.iter().map(|o| (o, format!("{}{PRIOR}", tag(course, u)), course, "")).collect(), &mut rng);
    if let Some(before) = cur.year_before(&course.year, &course.route) {
        let all: Vec<Cand> =
            before.units.iter().flat_map(|bu| bu.objectives.iter().map(move |o| (o, tag(before, bu), before, bu.title.as_str()))).collect();
        year.extend(shuffled(all, &mut rng));
    }
    let long_ago: Vec<Cand> =
        earlier(2).into_iter().flat_map(|pu| pu.objectives.iter().map(move |o| (o, tag(course, pu), course, pu.title.as_str()))).collect();
    year.extend(shuffled(long_ago, &mut rng));
    // When nothing else fits, any other objective of this unit is better than an empty card.
    let rest: Vec<Cand> =
        u.objectives.iter().enumerate().filter(|(i, _)| *i != objective).map(|(_, o)| (o, tag(course, u), course, u.title.as_str())).collect();

    let mut used: Vec<String> = vec![];
    let mut out = vec![];
    for (heading, mut list) in HEADINGS.iter().zip([lesson, week, topic, year]) {
        let first = heading == &HEADINGS[0];
        if !first {
            list.extend(rest.iter().cloned());
        }
        let mut slot = Slot { heading: heading.to_string(), objective: String::new(), from: String::new(), skills: vec![] };
        // The first objective that fits well; failing that, the one that fits best.
        let mut fallback: Option<(f64, Slot)> = None;
        for (o, from, c, context) in list.iter().take(200) {
            let (keys, score) = m.scored(o, context, &c.year, &c.route);
            let keys: Vec<String> = keys.into_iter().filter(|k| !used.contains(k)).collect();
            // The last card says where its question comes from.
            let heading = if heading == &HEADINGS[3] {
                if c.year != course.year {
                    HEADINGS[3]
                } else if from.ends_with(PRIOR) {
                    "Prior learning"
                } else {
                    "Earlier this year"
                }
            } else {
                heading
            };
            let here = Slot { heading: heading.to_string(), objective: o.text.clone(), from: from.clone(), skills: keys };
            if slot.objective.is_empty() {
                // Keep the first choice's words even when nothing fits, so the teacher sees why.
                slot = Slot { skills: vec![], ..here.clone() };
            }
            if here.skills.is_empty() {
                continue;
            }
            if score >= GOOD_FIT || first {
                fallback = Some((score, here));
                break;
            }
            if fallback.as_ref().is_none_or(|(best, _)| score > *best) {
                fallback = Some((score, here));
            }
        }
        if let Some((_, found)) = fallback {
            slot = found;
        }
        if let Some(k) = slot.skills.first() {
            used.push(k.clone());
        }
        out.push(slot);
    }
    out
}

/// One question for each slot at `level` (1 to 4, 0 for any), different from each other, from
/// `bank` made with `seed` (`Bank::instantiate`). A skill with nothing at that level gives a
/// question at the nearest level it has; a slot no skill fits gives `None`.
pub fn questions_for(bank: &Bank, slots: &[Slot], level: u8, seed: u64) -> Vec<Option<Question>> {
    let mut taken: Vec<String> = vec![];
    let mut out = vec![];
    for (i, slot) in slots.iter().enumerate() {
        let q = pick_for(bank, &slot.skills, level, seed.wrapping_add(i as u64 * 7919), &taken);
        if let Some(q) = &q {
            taken.push(q.id.clone());
        }
        out.push(q);
    }
    out
}

/// One question practising any of `skills` (the first that has one), at `level` or the nearest
/// level that skill has, not in `exclude`.
pub fn pick_for(bank: &Bank, skills: &[String], level: u8, seed: u64, exclude: &[String]) -> Option<Question> {
    let mut levels: Vec<Option<u8>> = vec![];
    if level == 0 {
        levels.push(None);
    } else {
        let mut by_distance: Vec<u8> = crate::questions::DIFFICULTIES.collect();
        by_distance.sort_by_key(|l| (l.abs_diff(level), *l));
        levels.extend(by_distance.into_iter().map(Some));
    }
    for key in skills {
        for d in &levels {
            let f = Filter { skill: Some(key.clone()), difficulty: *d, ..Default::default() };
            if let Some(q) = bank.pick_one(&f, seed, exclude) {
                return Some(q.clone());
            }
        }
    }
    None
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

    /// A small curriculum in our own words (no provider's wording in the repository).
    pub(crate) const SAMPLE: &str = r#"{"version": 1, "name": "Test school", "courses": [
      {"year": "Y7", "route": "Core", "units": [
        {"title": "Place value", "start": "2026-09-01", "weeks": 3, "objectives": [
          {"text": "Round numbers to the nearest 10, 100 or 1000"}, {"text": "Order decimals"}]},
        {"title": "Angles", "start": "2026-09-22", "weeks": 3, "objectives": [
          {"text": "Angles on a straight line"}, {"text": "Angles in a triangle"}]},
        {"title": "Holiday", "start": "2026-10-19", "weeks": 2},
        {"title": "Equations", "start": "2026-11-02", "weeks": 4, "objectives": [
          {"text": "Solve one-step equations"}, {"text": "Solve two-step equations"},
          {"text": "Solve equations with the unknown on both sides"}],
          "prior": [{"text": "Substitute into an expression"}]}]},
      {"year": "Y8", "route": "Core", "units": [
        {"title": "Ratio", "start": "2026-09-01", "weeks": 4, "objectives": [
          {"text": "Share an amount in a given ratio"}, {"text": "Simplify ratios"}]}]}]}"#;

    #[test]
    fn parses_and_finds_this_weeks_unit() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.years(), vec!["Y7", "Y8"]);
        assert_eq!(c.routes("Y7"), vec!["Core"]);
        let y7 = c.course("Y7", "core").unwrap();
        assert_eq!(y7.unit_on("2026-08-01"), Some(0), "before the first unit: the first");
        assert_eq!(y7.unit_on("2026-09-23"), Some(1));
        assert_eq!(y7.unit_on("2026-10-22"), Some(1), "in the holiday: the unit before it");
        assert_eq!(y7.unit_on("2026-11-20"), Some(3));
        // An unknown route falls back to the year's first course.
        assert_eq!(c.course("Y8", "Support").unwrap().name(), "Y8 Core");
    }

    #[test]
    fn rejects_bad_files() {
        assert!(parse("{}").is_err());
        assert!(parse(r#"{"version": 2, "courses": [{"year": "Y7", "units": []}]}"#).is_err());
        assert!(parse(r#"{"version": 1, "courses": []}"#).is_err());
        assert!(
            parse(r#"{"version": 1, "courses": [{"year": "Y7", "units": [{"title": "A", "start": "4 Jan"}]}]}"#).unwrap_err().contains("not a date")
        );
        assert!(parse(r#"{"version": 1, "courses": [{"year": "Y7", "units": [{"title": "A", "objectives": [{"text": " "}]}]}]}"#).is_err());
    }

    #[test]
    fn dates_from_days() {
        assert_eq!(date_from_days(0), "1970-01-01");
        assert_eq!(date_from_days(20_748), "2026-10-22");
        assert_eq!(date_from_days(11_016), "2000-02-29");
        for days in [0, 11_016, 20_748, 21_000, -365] {
            assert_eq!(days_from_date(&date_from_days(days)), Some(days));
        }
        assert_eq!(days_from_date("2026-13-01"), None);
        assert_eq!(days_from_date("4 Jan"), None);
    }

    #[test]
    fn the_objective_just_taught_follows_the_weeks() {
        let c = parse(SAMPLE).unwrap();
        let y7 = c.course("Y7", "Core").unwrap();
        // Equations: 3 objectives over 4 weeks from 2 November.
        assert_eq!(y7.objective_on(3, "2026-11-02"), 0);
        assert_eq!(y7.objective_on(3, "2026-11-16"), 1);
        assert_eq!(y7.objective_on(3, "2026-11-26"), 2);
        assert_eq!(y7.objective_on(3, "2027-03-01"), 2, "after the unit: the last");
        assert_eq!(y7.objective_on(3, "2026-10-01"), 0, "before it: the first");
        assert_eq!(y7.objective_on(2, "2026-10-20"), 0, "a unit with no objectives");
    }

    #[test]
    fn words_meet_their_other_forms() {
        assert_eq!(words("Factorising quadratics"), words("factorise quadratic"));
        assert!(words("Solve equations").contains(&"equation".to_string()));
        assert!(words("Rearranging formulae").iter().any(|w| w == "rearrang"));
        assert_eq!(words("Pythag"), words("Pythagoras"));
        assert!(words("Calculate the area of the circle").iter().all(|w| !["calculate", "the"].contains(&w.as_str())));
    }

    fn matcher() -> (Bank, Matcher) {
        let bank = crate::questions::starter().unwrap().instantiate(0);
        let m = Matcher::new(bank.skills());
        (bank, m)
    }

    #[test]
    fn objectives_find_the_skills_they_name() {
        let (_, m) = matcher();
        let name = |text: &str, year: &str, route: &str| {
            let found = m.best(text, year, route, 1);
            m.skills[found.first().expect(text).0].name.clone()
        };
        let low = |s: String| s.to_lowercase();
        assert!(low(name("Solve equations with the unknown on both sides", "Y8", "Core")).contains("both sides"));
        assert!(low(name("Use Pythagoras' theorem to find missing lengths", "Y9", "Core")).contains("pythagoras"));
        assert!(low(name("Share an amount in a given ratio", "Y8", "Core")).contains("ratio"));
        assert!(low(name("Calculate the area of circles", "Y8", "Core")).contains("circle"));
        assert!(low(name("Write a number as the product of its prime factors", "Y8", "Core")).contains("prime"));
        // Words no skill uses find nothing rather than something random.
        assert!(m.best("Investigate how Egyptians wrote numbers", "Y7", "Core", 3).len() <= 3);
        assert!(m.best("zzzz qqqq", "Y7", "Core", 3).is_empty());
    }

    #[test]
    fn named_skills_win_over_the_words() {
        let (_, m) = matcher();
        let key = m.skills.iter().find(|s| s.name.to_lowercase().contains("ratio")).unwrap().key.clone();
        let o = Objective { text: "Solve two-step equations".into(), skills: vec![key.clone(), "no such skill".into()], search: String::new() };
        assert_eq!(m.skills_for(&o, "", "Y7", "Core"), vec![key]);
        let o = Objective { text: "Something vague".into(), skills: vec![], search: "angles in a triangle".into() };
        assert!(m.skills_for(&o, "", "Y7", "Core").iter().any(|k| k.to_lowercase().contains("triangle")));
    }

    #[test]
    fn plans_last_lesson_week_topic_and_year() {
        let (bank, m) = matcher();
        let c = parse(SAMPLE).unwrap();
        let y7 = c.course("Y7", "Core").unwrap();
        let slots = plan(&c, y7, 3, 2, &m, 1);
        assert_eq!(slots.iter().map(|s| s.heading.as_str()).collect::<Vec<_>>(), ["Last lesson", "Last week", "Last topic", "Prior learning"]);
        assert_eq!(slots[0].objective, "Solve equations with the unknown on both sides");
        assert!(slots[1].from.ends_with("Equations"), "last week is earlier in the unit: {:?}", slots[1]);
        assert!(slots[1].objective.starts_with("Solve") && slots[1].objective != slots[0].objective);
        assert!(slots[2].from.ends_with("Angles"), "last topic is the unit before (the holiday has no objectives): {:?}", slots[2]);
        assert!(slots[3].from.ends_with("prior learning"), "{:?}", slots[3]);
        assert!(slots.iter().all(|s| !s.skills.is_empty()), "{slots:?}");
        let firsts: Vec<&String> = slots.iter().map(|s| &s.skills[0]).collect();
        for (i, k) in firsts.iter().enumerate() {
            assert!(!firsts[..i].contains(k), "two slots practise the same skill: {firsts:?}");
        }
        let qs = questions_for(&bank, &slots, 2, 9);
        assert!(qs.iter().all(Option::is_some));
        let ids: Vec<&str> = qs.iter().flatten().map(|q| q.id.as_str()).collect();
        assert_eq!(ids.len(), 4);
        // The same seed plans the same starter.
        assert_eq!(plan(&c, y7, 3, 2, &m, 1), slots);
    }

    #[test]
    fn the_first_objective_of_a_unit_looks_back_to_the_unit_before() {
        let (_, m) = matcher();
        let c = parse(SAMPLE).unwrap();
        let y7 = c.course("Y7", "Core").unwrap();
        let slots = plan(&c, y7, 3, 0, &m, 5);
        assert!(slots[1].from.ends_with("Angles"), "{:?}", slots[1]);
        // The first unit of the year has nothing before it but still fills four cards.
        let slots = plan(&c, y7, 0, 0, &m, 5);
        assert_eq!(slots.len(), 4);
        // Year 8's last-year question can come from Year 7.
        let y8 = c.course("Y8", "Core").unwrap();
        let slots = plan(&c, y8, 0, 1, &m, 2);
        assert!(slots[3].from.starts_with("Y7"), "{:?}", slots[3]);
        assert_eq!(slots[3].heading, "Last year");
    }

    fn slide_texts(s: &crate::Session) -> Vec<String> {
        let st = s.doc().unwrap();
        let slide = &st.doc.slides[st.selection.slide];
        slide.shapes.iter().filter_map(|x| x.text.as_ref().map(|t| t.paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join(" "))).collect()
    }

    #[test]
    fn the_command_plans_a_starter_from_what_was_taught() {
        let mut s = crate::Session::with_new();
        let out = s
            .execute(
                "slide.getReady",
                &serde_json::json!({"seed": 4, "taught": {"curriculumJson": SAMPLE, "year": "Y7", "route": "Core", "unit": "Equations", "objective": 1}}),
            )
            .unwrap();
        let plan = out["plan"].as_array().unwrap();
        assert_eq!(plan.len(), 4);
        assert_eq!(plan[0]["objective"], "Solve two-step equations");
        assert_eq!(plan[0]["heading"], "Last lesson");
        assert_eq!(out["questions"].as_array().unwrap().len(), 4);
        let texts = slide_texts(&s).join(" | ");
        for h in ["Last lesson", "Last week", "Last topic", "Prior learning"] {
            assert!(texts.contains(h), "{h} heading on the slide: {texts}");
        }
        // A unit with nothing to plan from, and a year the file does not have, say so.
        let err =
            s.execute("slide.getReady", &serde_json::json!({"taught": {"curriculumJson": SAMPLE, "year": "Y7", "unit": "Holiday"}})).unwrap_err();
        assert!(err.to_string().contains("no objectives"), "{err}");
        let err = s.execute("slide.getReady", &serde_json::json!({"taught": {"curriculumJson": SAMPLE, "year": "Y11", "unit": 0}})).unwrap_err();
        assert!(err.to_string().contains("no Y11"), "{err}");
    }

    #[test]
    fn the_command_uses_given_questions_and_headings_as_they_are() {
        let mut s = crate::Session::with_new();
        let qs = serde_json::json!([
            {"text": "What is $3 \\times 4$?", "answer": "12"},
            {"text": "Round 4.67 to 1 decimal place", "answer": "4.7"},
            {"text": "Simplify $2a + 3a$", "answer": "$5a$"},
            {"text": "My own question about angles", "answer": "60°"}
        ]);
        let out = s
            .execute("slide.getReady", &serde_json::json!({"questions": qs, "headings": ["Starter", "", "Warm up", "Old"], "expand": false}))
            .unwrap();
        assert_eq!(out["questions"], serde_json::json!(["given-1", "given-2", "given-3", "given-4"]));
        let texts = slide_texts(&s).join(" | ");
        assert!(texts.contains("Round 4.67 to 1 decimal place") && texts.contains("My own question about angles"), "{texts}");
        assert!(texts.contains("Starter") && texts.contains("Warm up") && !texts.contains("Last week"), "{texts}");
        let notes = {
            let st = s.doc().unwrap();
            st.doc.slides[st.selection.slide].notes.paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join("\n")
        };
        assert!(notes.contains("4.7") && notes.contains("60°"), "answers in the notes: {notes}");
        assert!(s.execute("slide.getReady", &serde_json::json!({"questions": [{"text": " "}]})).is_err());
        assert!(s.execute("slide.getReady", &serde_json::json!({"questions": []})).is_err());
        assert!(s.execute("slide.getReady", &serde_json::json!({"questions": qs, "headings": 3})).is_err());
    }

    #[test]
    fn a_skill_without_the_level_gives_the_nearest_level() {
        let (bank, m) = matcher();
        let key = m.skills.iter().find(|s| s.counts[0] == 0 && s.counts[1] > 0).map(|s| s.key.clone());
        if let Some(key) = key {
            let q = pick_for(&bank, &[key], 1, 3, &[]).unwrap();
            assert_eq!(q.difficulty, 2);
        }
        assert!(pick_for(&bank, &["no such skill".into()], 2, 3, &[]).is_none());
    }
}
