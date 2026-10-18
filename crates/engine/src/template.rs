//! Question templates: a question written once as a recipe, made new each time it is asked.
//!
//! A template names some variables and the numbers they may take (`"a": {"int": [2, 9]}`), works
//! out others from them (`"calc"`), says what must be true of the result (`"where"`) and then
//! fills `[[placeholders]]` in its text, answer and diagram. Asking again with a new seed gives
//! new numbers; the same seed always gives the same question. The format is documented for
//! question writers in `docs/question-writing-guide.md`.
//!
//! Nothing here runs code from the file: expressions are read by the small evaluator below, which
//! knows arithmetic, comparisons and a few maths functions, and nothing else.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::questions::Question;

const MAX_TRIES: usize = 300;
const MAX_EXPR: usize = 300;
const EPS: f64 = 1e-9;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub topic: String,
    #[serde(default)]
    pub subtopic: String,
    /// 1 = support, 2 = core, 3 = stretch, 4 = exam style.
    pub difficulty: u8,
    /// Words a teacher might search for ("both sides", "simultaneous"); topic and subtopic are searched too.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// `fluency`, `reasoning` or `problem`: what the question asks of a pupil.
    #[serde(default)]
    pub qtype: String,
    /// Where it sits in a scheme of work, e.g. “Y7 Autumn B3 S7 Solve 2-step equations”.
    #[serde(default)]
    pub source: String,
    /// The variables, in the order they are chosen. A later one's range may use an earlier one.
    #[serde(default)]
    pub vars: Vec<(String, VarSpec)>,
    /// Values worked out from the variables, in order: `["d", "(a - c) * x + b"]`.
    #[serde(default)]
    pub calc: Vec<(String, String)>,
    /// Every one must be true for the numbers to be used: `"a != c"`, `"d > 0"`.
    #[serde(default, rename = "where")]
    pub conditions: Vec<String>,
    /// Sums that must hold for every question made, to catch a wrong answer formula: put the
    /// answer back into the question, e.g. `"a * x + b == c * x + d"`. A failure is an error.
    #[serde(default, rename = "check")]
    pub checks: Vec<String>,
    pub text: String,
    pub answer: String,
    /// A figure whose strings may hold `[[placeholders]]`; see `cmd/figures.rs` for the kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagram: Option<Value>,
}

/// What a variable can be. Exactly one of the choices is given.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct VarSpec {
    /// A whole number from `[low, high]` inclusive; each end a number or an expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub int: Option<Vec<Value>>,
    /// With `int`: numbers it must not be.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not: Vec<f64>,
    /// One of these numbers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choose: Option<Vec<f64>>,
    /// `[low, high, step]`: 0.5, 1, 1.5 … (for decimals).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<Vec<f64>>,
    /// One of these words or letters, used as `[[name]]` in text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pick: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
enum Val {
    Num(f64),
    Text(String),
}

type Scope = HashMap<String, Val>;

// ── choosing numbers ─────────────────────────────────────────────────────────────────────────

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn bound(v: &Value, scope: &Scope) -> Result<f64, String> {
    match v {
        Value::Number(n) => n.as_f64().ok_or_else(|| "a range end is not a number".to_string()),
        Value::String(s) => eval(s, scope),
        _ => Err("a range end must be a number or an expression".into()),
    }
}

fn draw(name: &str, spec: &VarSpec, scope: &Scope, rng: &mut u64) -> Result<Val, String> {
    let kinds = [spec.int.is_some(), spec.choose.is_some(), spec.step.is_some(), spec.pick.is_some()].iter().filter(|k| **k).count();
    if kinds != 1 {
        return Err(format!("variable {name}: give exactly one of int, choose, step or pick"));
    }
    if let Some(r) = &spec.int {
        let [lo, hi] = r.as_slice() else { return Err(format!("variable {name}: int needs [low, high]")) };
        let (lo, hi) = (bound(lo, scope)?.round() as i64, bound(hi, scope)?.round() as i64);
        if hi < lo {
            return Err(format!("variable {name}: the range {lo} to {hi} is empty"));
        }
        let span = (hi - lo + 1) as u64;
        for _ in 0..50 {
            let v = lo + (splitmix(rng) % span) as i64;
            if !spec.not.iter().any(|n| (n - v as f64).abs() < EPS) {
                return Ok(Val::Num(v as f64));
            }
        }
        return Err(format!("variable {name}: every number in its range is excluded"));
    }
    if let Some(c) = &spec.choose {
        if c.is_empty() {
            return Err(format!("variable {name}: choose is empty"));
        }
        return Ok(Val::Num(c[(splitmix(rng) % c.len() as u64) as usize]));
    }
    if let Some(s) = &spec.step {
        let [lo, hi, st] = s.as_slice() else { return Err(format!("variable {name}: step needs [low, high, step]")) };
        if *st <= 0.0 || hi < lo {
            return Err(format!("variable {name}: step needs low <= high and a positive step"));
        }
        let n = ((hi - lo) / st + EPS).floor() as u64 + 1;
        let v = lo + (splitmix(rng) % n) as f64 * st;
        return Ok(Val::Num((v * 1e9).round() / 1e9));
    }
    let p = spec.pick.as_deref().unwrap_or_default();
    if p.is_empty() {
        return Err(format!("variable {name}: pick is empty"));
    }
    Ok(Val::Text(p[(splitmix(rng) % p.len() as u64) as usize].clone()))
}

// ── the expression evaluator ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Id(String),
    Op(&'static str),
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    if src.len() > MAX_EXPR {
        return Err("an expression is too long".into());
    }
    let c: Vec<char> = src.chars().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() || (ch == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let s = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            let t: String = c[s..i].iter().collect();
            out.push(Tok::Num(t.parse().map_err(|_| format!("`{t}` is not a number"))?));
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            let s = i;
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Tok::Id(c[s..i].iter().collect()));
        } else {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            let op = ["==", "!=", "<=", ">=", "&&", "||"].into_iter().find(|o| *o == two);
            if let Some(o) = op {
                out.push(Tok::Op(o));
                i += 2;
            } else {
                let one = ["+", "-", "*", "/", "%", "^", "(", ")", ",", "<", ">", "!"].into_iter().find(|o| o.starts_with(ch));
                out.push(Tok::Op(one.ok_or_else(|| format!("unexpected `{ch}` in `{src}`"))?));
                i += 1;
            }
        }
    }
    Ok(out)
}

struct Parser<'a> {
    t: Vec<Tok>,
    i: usize,
    scope: &'a Scope,
    depth: usize,
}

fn eval(src: &str, scope: &Scope) -> Result<f64, String> {
    let mut p = Parser { t: lex(src)?, i: 0, scope, depth: 0 };
    let v = p.or()?;
    if p.i != p.t.len() {
        return Err(format!("`{src}` has something left over after the end"));
    }
    if !v.is_finite() {
        return Err(format!("`{src}` is not a finite number"));
    }
    Ok(v)
}

impl Parser<'_> {
    fn eat(&mut self, op: &str) -> bool {
        if matches!(self.t.get(self.i), Some(Tok::Op(o)) if *o == op) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn or(&mut self) -> Result<f64, String> {
        let mut v = self.and()?;
        while self.eat("||") {
            let r = self.and()?;
            v = f64::from(v != 0.0 || r != 0.0);
        }
        Ok(v)
    }

    fn and(&mut self) -> Result<f64, String> {
        let mut v = self.cmp()?;
        while self.eat("&&") {
            let r = self.cmp()?;
            v = f64::from(v != 0.0 && r != 0.0);
        }
        Ok(v)
    }

    fn cmp(&mut self) -> Result<f64, String> {
        let l = self.sum()?;
        for op in ["==", "!=", "<=", ">=", "<", ">"] {
            if self.eat(op) {
                let r = self.sum()?;
                let same = (l - r).abs() < EPS;
                return Ok(f64::from(match op {
                    "==" => same,
                    "!=" => !same,
                    "<=" => l < r || same,
                    ">=" => l > r || same,
                    "<" => l < r && !same,
                    _ => l > r && !same,
                }));
            }
        }
        Ok(l)
    }

    fn sum(&mut self) -> Result<f64, String> {
        let mut v = self.product()?;
        loop {
            if self.eat("+") {
                v += self.product()?;
            } else if self.eat("-") {
                v -= self.product()?;
            } else {
                return Ok(v);
            }
        }
    }

    fn product(&mut self) -> Result<f64, String> {
        let mut v = self.unary()?;
        loop {
            if self.eat("*") {
                v *= self.unary()?;
            } else if self.eat("/") {
                let r = self.unary()?;
                if r.abs() < EPS {
                    return Err("divides by zero".into());
                }
                v /= r;
            } else if self.eat("%") {
                let r = self.unary()?;
                if r.abs() < EPS {
                    return Err("divides by zero".into());
                }
                v = v.rem_euclid(r);
            } else {
                return Ok(v);
            }
        }
    }

    fn unary(&mut self) -> Result<f64, String> {
        if self.eat("-") {
            return Ok(-self.unary()?);
        }
        if self.eat("+") {
            return self.unary();
        }
        if self.eat("!") {
            return Ok(f64::from(self.unary()? == 0.0));
        }
        self.power()
    }

    fn power(&mut self) -> Result<f64, String> {
        let base = self.primary()?;
        if self.eat("^") {
            let e = self.unary()?;
            return Ok(base.powf(e));
        }
        Ok(base)
    }

    fn primary(&mut self) -> Result<f64, String> {
        self.depth += 1;
        if self.depth > 40 {
            return Err("an expression is nested too deeply".into());
        }
        let r = self.primary_inner();
        self.depth -= 1;
        r
    }

    fn primary_inner(&mut self) -> Result<f64, String> {
        match self.t.get(self.i).cloned() {
            Some(Tok::Num(n)) => {
                self.i += 1;
                Ok(n)
            }
            Some(Tok::Op("(")) => {
                self.i += 1;
                let v = self.or()?;
                if !self.eat(")") {
                    return Err("a bracket is not closed".into());
                }
                Ok(v)
            }
            Some(Tok::Id(name)) => {
                self.i += 1;
                if self.eat("(") {
                    let mut args = vec![];
                    if !self.eat(")") {
                        loop {
                            args.push(self.or()?);
                            if self.eat(")") {
                                break;
                            }
                            if !self.eat(",") {
                                return Err(format!("`{name}(` is missing a bracket or comma"));
                            }
                        }
                    }
                    return call(&name, &args);
                }
                match self.scope.get(&name) {
                    Some(Val::Num(n)) => Ok(*n),
                    Some(Val::Text(_)) => Err(format!("`{name}` is a word, not a number")),
                    None => Err(format!("`{name}` is not a variable of this template")),
                }
            }
            _ => Err("an expression ends too soon".into()),
        }
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn call(name: &str, a: &[f64]) -> Result<f64, String> {
    let one = |f: fn(f64) -> f64| match a {
        [x] => Ok(f(*x)),
        _ => Err(format!("{name}() takes one number")),
    };
    match name {
        "abs" => one(f64::abs),
        "sqrt" => match a {
            [x] if *x >= 0.0 => Ok(x.sqrt()),
            _ => Err("sqrt() of a negative number".into()),
        },
        "sin" => one(|x| x.to_radians().sin()),
        "cos" => one(|x| x.to_radians().cos()),
        "tan" => one(|x| x.to_radians().tan()),
        "asin" => one(|x| x.asin().to_degrees()),
        "acos" => one(|x| x.acos().to_degrees()),
        "atan" => one(|x| x.atan().to_degrees()),
        "round" => one(f64::round),
        "log10" => match a {
            [x] if *x > 0.0 => Ok(x.log10()),
            _ => Err("log10() needs a positive number".into()),
        },
        // `sf(x, n)`: x to n significant figures.
        "sf" => match a {
            [x, n] if *n >= 1.0 && *n <= 12.0 => {
                if x.abs() < EPS {
                    return Ok(0.0);
                }
                let digits = x.abs().log10().floor() + 1.0;
                let factor = 10f64.powf(n.round() - digits);
                Ok((x * factor).round() / factor)
            }
            _ => Err("sf() takes a number and 1 to 12 significant figures".into()),
        },
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "isint" => one(|x| f64::from((x - x.round()).abs() < EPS)),
        "min" if !a.is_empty() => Ok(a.iter().cloned().fold(f64::INFINITY, f64::min)),
        "max" if !a.is_empty() => Ok(a.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
        "gcd" => match a {
            [x, y] => Ok(gcd(x.round() as i64, y.round() as i64) as f64),
            _ => Err("gcd() takes two numbers".into()),
        },
        "lcm" => match a {
            [x, y] => {
                let (x, y) = (x.round() as i64, y.round() as i64);
                let g = gcd(x, y).max(1);
                Ok((x / g * y).abs() as f64)
            }
            _ => Err("lcm() takes two numbers".into()),
        },
        _ => Err(format!("`{name}` is not a function a template can use")),
    }
}

// ── writing numbers into text ────────────────────────────────────────────────────────────────

/// Whether `x` is a whole number, allowing for the rounding error of large values
/// (9.8 × 10⁸ comes out as 980000000.0000001).
fn is_int(x: f64) -> bool {
    (x - x.round()).abs() < EPS * x.abs().max(1.0)
}

fn int_text(x: f64) -> String {
    format!("{}", x.round() as i64)
}

/// `n/d` for `x` when it is a ratio with a small denominator.
fn fraction(x: f64) -> Option<(i64, i64)> {
    (1..=1000i64).find_map(|d| {
        let n = x * d as f64;
        is_int(n).then(|| (n.round() as i64, d))
    })
}

/// `x` as a decimal with at most six places and no trailing zeros.
fn trimmed(x: f64) -> String {
    let s = format!("{x:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn plain(x: f64) -> String {
    if is_int(x) {
        return int_text(x);
    }
    let t = trimmed(x);
    if (t.parse::<f64>().unwrap_or(f64::NAN) - x).abs() < 1e-7 * x.abs().max(1.0) { t } else { frac_text(x) }
}

fn frac_text(x: f64) -> String {
    match fraction(x) {
        Some((n, 1)) => format!("{n}"),
        Some((n, d)) => format!("{}\\frac{{{}}}{{{d}}}", if n < 0 { "-" } else { "" }, n.abs()),
        None => trimmed(x),
    }
}

fn coef(x: f64) -> String {
    if (x - 1.0).abs() < EPS {
        String::new()
    } else if (x + 1.0).abs() < EPS {
        "-".into()
    } else {
        plain(x)
    }
}

/// A whole number in English, in the form a teacher writes it: “four thousand and thirty-two”.
fn words(n: u64) -> String {
    const ONES: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 10] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    fn below_1000(n: u64) -> String {
        let (h, r) = (n / 100, n % 100);
        let mut out = String::new();
        if h > 0 {
            out.push_str(&format!("{} hundred", ONES[h as usize]));
        }
        if r > 0 {
            if h > 0 {
                out.push_str(" and ");
            }
            if r < 20 {
                out.push_str(ONES[r as usize]);
            } else {
                out.push_str(TENS[(r / 10) as usize]);
                if r % 10 > 0 {
                    out.push_str(&format!("-{}", ONES[(r % 10) as usize]));
                }
            }
        }
        out
    }
    if n == 0 {
        return "zero".into();
    }
    let mut parts: Vec<String> = vec![];
    for (size, name) in [(1_000_000_000u64, "billion"), (1_000_000, "million"), (1_000, "thousand")] {
        if !(n / size).is_multiple_of(1000) {
            parts.push(format!("{} {name}", below_1000(n / size % 1000)));
        }
    }
    let rest = n % 1000;
    let mut out = parts.join(" ");
    if rest > 0 {
        // “four thousand and thirty-two”, but “four thousand two hundred”.
        if !out.is_empty() && rest < 100 {
            out.push_str(" and ");
        } else if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&below_1000(rest));
    }
    out
}

/// One placeholder's `expr` or `expr|filter[:argument]` as text, or as a number for `|n`.
fn fill(inner: &str, scope: &Scope) -> Result<(String, Option<f64>), String> {
    let (expr, filter) = match inner.split_once('|') {
        Some((e, f)) => (e.trim(), Some(f.trim())),
        None => (inner.trim(), None),
    };
    if let Some(Val::Text(t)) = scope.get(expr) {
        return Ok((t.clone(), None));
    }
    let x = eval(expr, scope)?;
    let (name, arg) = match filter.map(|f| f.split_once(':').map_or((f, ""), |(a, b)| (a.trim(), b.trim()))) {
        Some((n, a)) => (n, a),
        None => ("", ""),
    };
    let text = match name {
        "" | "n" => plain(x),
        "coef" => coef(x),
        "abs" => plain(x.abs()),
        "signed" => match x {
            v if v.abs() < EPS => String::new(),
            v if v > 0.0 => format!(" + {}", plain(v)),
            v => format!(" - {}", plain(-v)),
        },
        "term" => {
            if x.abs() < EPS {
                "0".into()
            } else {
                format!("{}{arg}", coef(x))
            }
        }
        "sterm" => match x {
            v if v.abs() < EPS => String::new(),
            v if v > 0.0 => format!(" + {}{arg}", coef(v)),
            v => format!(" - {}{arg}", coef(-v)),
        },
        "sgn" => {
            if x < 0.0 {
                format!("-{}", plain(-x))
            } else {
                format!("+{}", plain(x))
            }
        }
        // `[[cond|if:Yes/No]]`: the first word when the expression is true (not 0), else the second.
        "if" => {
            let (yes, no) = arg.split_once('/').ok_or_else(|| format!("if needs two choices, like if:Yes/No (in `{inner}`)"))?;
            (if x != 0.0 { yes } else { no }).to_string()
        }
        // Groups of three digits with a thin space, as UK schools write them: 10 000, 82 000 000.
        "group" => {
            let t = plain(x);
            let (whole, rest) = t.split_once('.').map_or((t.as_str(), None), |(w, r)| (w, Some(r)));
            let (sign, digits) = whole.strip_prefix('-').map_or(("", whole), |d| ("-", d));
            let grouped = if digits.len() >= 5 {
                let mut out = String::new();
                for (i, c) in digits.chars().enumerate() {
                    if i > 0 && (digits.len() - i) % 3 == 0 {
                        out.push('\u{2009}');
                    }
                    out.push(c);
                }
                out
            } else {
                digits.to_string()
            };
            match rest {
                Some(r) => format!("{sign}{grouped}.{r}"),
                None => format!("{sign}{grouped}"),
            }
        }
        "words" => {
            if !is_int(x) || !(0.0..1e12).contains(&x) {
                return Err(format!("words needs a whole number from 0 to a trillion (in `{inner}`)"));
            }
            words(x.round() as u64)
        }
        "frac" => frac_text(x),
        "mixed" => match fraction(x) {
            Some((n, d)) if d > 1 && n.abs() > d => format!("{}{}\\frac{{{}}}{{{d}}}", if n < 0 { "-" } else { "" }, n.abs() / d, n.abs() % d),
            _ => frac_text(x),
        },
        "dec" => plain(x),
        "dp" => {
            let places: usize = arg.parse().map_err(|_| format!("dp needs a number of places, like dp:1 (in `{inner}`)"))?;
            if places > 8 {
                return Err("dp: at most 8 places".into());
            }
            format!("{x:.places$}")
        }
        other => {
            return Err(format!(
                "`{other}` is not a filter (use coef, signed, sgn, term:x, sterm:x, frac, mixed, dec, dp:N, abs, words, group, if:A/B, n)"
            ));
        }
    };
    Ok((text, (name == "n").then_some(x)))
}

/// `text` with every `[[…]]` replaced; the number when the whole string is one `|n` placeholder.
fn fill_all(text: &str, scope: &Scope) -> Result<Value, String> {
    let mut out = String::new();
    let mut rest = text;
    let mut only: Option<f64> = None;
    let mut parts = 0;
    while let Some(a) = rest.find("[[") {
        out.push_str(&rest[..a]);
        let after = &rest[a + 2..];
        let b = after.find("]]").ok_or_else(|| format!("`[[` is never closed in `{text}`"))?;
        let (t, n) = fill(&after[..b], scope)?;
        out.push_str(&t);
        only = n;
        parts += 1;
        rest = &after[b + 2..];
    }
    out.push_str(rest);
    if parts == 1
        && text.trim().starts_with("[[")
        && text.trim().ends_with("]]")
        && let Some(n) = only
    {
        // A whole number is an integer in the JSON, so figures that count things can read it.
        return Ok(if is_int(n) { serde_json::json!(n.round() as i64) } else { serde_json::json!(n) });
    }
    if out.contains("]]") {
        return Err(format!("a stray `]]` in `{text}`"));
    }
    Ok(Value::String(out))
}

fn fill_json(v: &Value, scope: &Scope) -> Result<Value, String> {
    Ok(match v {
        Value::String(s) if s.contains("[[") => fill_all(s, scope)?,
        Value::Array(a) => Value::Array(a.iter().map(|x| fill_json(x, scope)).collect::<Result<_, _>>()?),
        Value::Object(o) => {
            let mut m = Map::new();
            for (k, x) in o {
                m.insert(k.clone(), fill_json(x, scope)?);
            }
            Value::Object(m)
        }
        other => other.clone(),
    })
}

// ── making a question ────────────────────────────────────────────────────────────────────────

impl Template {
    /// The question for `seed`: the same seed always gives the same numbers.
    pub fn instantiate(&self, seed: u64) -> Result<Question, String> {
        let mut rng = seed ^ fnv(&self.id);
        let mut last = String::new();
        for _ in 0..MAX_TRIES {
            match self.attempt(&mut rng) {
                Ok(Some(q)) => return Ok(q),
                Ok(None) => last = "the conditions in `where` were never all true".into(),
                Err(e) => return Err(format!("template {}: {e}", self.id)),
            }
        }
        Err(format!("template {}: {MAX_TRIES} tries found no numbers that satisfy it ({last}); loosen the ranges or `where`", self.id))
    }

    /// One try. `None` when its numbers break a `where` rule or a calculation fails in a way a
    /// different choice of numbers could fix (division by zero, the root of a negative).
    fn attempt(&self, rng: &mut u64) -> Result<Option<Question>, String> {
        let mut scope = Scope::new();
        for (name, spec) in &self.vars {
            scope.insert(name.clone(), draw(name, spec, &scope, rng)?);
        }
        for (name, expr) in &self.calc {
            match eval(expr, &scope) {
                Ok(v) => {
                    scope.insert(name.clone(), Val::Num(v));
                }
                Err(e) if e.contains("not a variable") || e.contains("not a function") || e.contains("left over") || e.contains("ends too soon") => {
                    return Err(format!("`{name}`: {e}"));
                }
                Err(_) => return Ok(None),
            }
        }
        for cond in &self.conditions {
            match eval(cond, &scope) {
                Ok(v) if v != 0.0 => {}
                Ok(_) => return Ok(None),
                Err(e) if e.contains("not a variable") || e.contains("not a function") || e.contains("left over") || e.contains("ends too soon") => {
                    return Err(format!("where `{cond}`: {e}"));
                }
                Err(_) => return Ok(None),
            }
        }
        for rule in &self.checks {
            match eval(rule, &scope) {
                Ok(v) if v != 0.0 => {}
                Ok(_) => return Err(format!("its own check `{rule}` fails for {}: the answer or a formula is wrong", show_scope(&scope))),
                Err(e) => return Err(format!("check `{rule}`: {e}")),
            }
        }
        let text = fill_all(&self.text, &scope)?;
        let answer = fill_all(&self.answer, &scope)?;
        let diagram = self.diagram.as_ref().map(|d| fill_json(d, &scope)).transpose()?;
        let (Value::String(text), Value::String(answer)) = (text, answer) else {
            return Err("text and answer must be words, not a bare number".into());
        };
        Ok(Some(Question {
            id: self.id.clone(),
            topic: self.topic.clone(),
            subtopic: self.subtopic.clone(),
            difficulty: self.difficulty,
            keywords: self.keywords.clone(),
            qtype: self.qtype.clone(),
            source: self.source.clone(),
            text,
            answer,
            needs_diagram: diagram.is_some(),
            diagram,
        }))
    }

    /// Everything wrong with the template itself, found by making questions from it.
    pub fn check(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("a template has no id".into());
        }
        if !crate::questions::DIFFICULTIES.contains(&self.difficulty) {
            return Err(format!("template {}: difficulty must be 1 (support), 2 (core), 3 (stretch) or 4 (exam style)", self.id));
        }
        if self.text.trim().is_empty() || self.answer.trim().is_empty() {
            return Err(format!("template {}: needs both text and answer", self.id));
        }
        for seed in 0..25u64 {
            let q = self.instantiate(seed.wrapping_mul(7919))?;
            if q.text.chars().count() > 600 {
                return Err(format!("template {}: the question comes out over 600 characters", self.id));
            }
            if let Some(d) = &q.diagram
                && !crate::questions::is_drawable(d)
            {
                return Err(format!("template {}: the diagram kind is not one of the drawable kinds", self.id));
            }
            if q.text.contains("[[") || q.answer.contains("[[") {
                return Err(format!("template {}: a placeholder was left unfilled", self.id));
            }
        }
        Ok(())
    }
}

/// The numbers of one try, for an error message: `a = 3, b = 5`.
fn show_scope(scope: &Scope) -> String {
    let mut v: Vec<String> = scope
        .iter()
        .map(|(k, v)| match v {
            Val::Num(n) => format!("{k} = {}", plain(*n)),
            Val::Text(t) => format!("{k} = {t}"),
        })
        .collect();
    v.sort();
    v.join(", ")
}

impl Template {
    /// Things that are not wrong enough to refuse the template but that a teacher would not want
    /// on the board: a minus sign right after a plus, money without two decimal places, an
    /// unused variable, a question that never changes. Found by reading questions made from it.
    pub fn lint(&self) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        let samples: Vec<Question> = (0..25u64).filter_map(|s| self.instantiate(s.wrapping_mul(7919)).ok()).collect();
        let any = |f: &dyn Fn(&str) -> bool| samples.iter().any(|q| f(&q.text) || f(&q.answer));
        if any(&|t| ["+ -", "+ −", "- -", "− −", "−−", "+ +"].iter().any(|b| t.contains(b))) {
            out.push("a negative number can appear right after a sign (like “+ −5”): write it with the signed filter, e.g. [[b|signed]]".into());
        }
        let money_one_dp = |t: &str| {
            t.match_indices('£').any(|(i, _)| {
                let rest: String = t[i + '£'.len_utf8()..].chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
                rest.split_once('.').is_some_and(|(_, d)| d.len() == 1)
            })
        };
        let money_whole_mixed = samples.iter().any(|q| money_one_dp(&q.answer));
        if money_whole_mixed {
            out.push("money should have two decimal places (£12.50): use [[amount|dp:2]]".into());
        }
        let long_decimal = |t: &str| {
            let b: Vec<char> = t.chars().collect();
            b.windows(6).any(|w| w[0].is_ascii_digit() && w[1] == '.' && w[2..6].iter().all(char::is_ascii_digit))
        };
        // Small decimals are the point of some topics (negative powers of 10, rounding, place value).
        let on_purpose = ["power", "standard form", "decimal", "place value", "round", "significant", "error interval"]
            .iter()
            .any(|w| self.subtopic.to_lowercase().contains(w));
        if !on_purpose && any(&long_decimal) {
            out.push("a decimal with four or more places can appear: round it with a filter such as [[x|dp:2]]".into());
        }
        let mut used = format!("{} {} ", self.text, self.answer);
        for e in self.calc.iter().map(|(_, e)| e).chain(self.conditions.iter()).chain(self.checks.iter()) {
            used.push_str(e);
            used.push(' ');
        }
        used.push_str(&self.diagram.as_ref().map(Value::to_string).unwrap_or_default());
        for (_, v) in &self.vars {
            for e in v.int.iter().flatten().filter_map(Value::as_str) {
                used.push(' ');
                used.push_str(e);
            }
        }
        let has_word = |w: &str| used.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).any(|t| t == w);
        for name in self.vars.iter().map(|(n, _)| n).chain(self.calc.iter().map(|(n, _)| n)) {
            if !has_word(name) {
                out.push(format!("`{name}` is never used"));
            }
        }
        if !crate::questions::QUESTION_TYPES.contains(&self.qtype.as_str()) {
            out.push("`qtype` must be fluency, reasoning or problem".into());
        }
        if self.checks.is_empty() {
            out.push("no \"check\": add one that puts the answer back into the question, so a wrong formula is caught".into());
        }
        if samples.len() > 3 && samples.iter().all(|q| q.text == samples[0].text && q.answer == samples[0].answer) {
            out.push("every question made from it is the same: give a variable a range".into());
        }
        out
    }
}

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn t(json: &str) -> Template {
        serde_json::from_str(json).unwrap()
    }

    const BOTH_SIDES: &str = r#"{
        "id": "alg-t-001", "topic": "Algebra", "subtopic": "Solving equations: unknown on both sides", "difficulty": 2,
        "keywords": ["solve", "both sides"],
        "vars": [["a", {"int": [3, 9]}], ["c", {"int": [1, 8]}], ["x", {"int": [-9, 12], "not": [0]}],
                 ["b", {"int": [1, 20]}], ["v", {"pick": ["x", "y", "n"]}]],
        "calc": [["d", "(a - c) * x + b"]],
        "where": ["a > c"],
        "text": "Solve $[[a|coef]][[v]] + [[b]] = [[c|coef]][[v]][[d|signed]]$",
        "answer": "$[[v]] = [[x]]$"
    }"#;

    #[test]
    fn the_same_seed_gives_the_same_question_and_a_new_seed_new_numbers() {
        let tpl = t(BOTH_SIDES);
        assert_eq!(tpl.instantiate(5).unwrap(), tpl.instantiate(5).unwrap());
        assert!((0..20).any(|s| tpl.instantiate(s).unwrap().text != tpl.instantiate(5).unwrap().text));
    }

    #[test]
    fn generated_equations_are_true_and_their_answer_is_the_stated_number() {
        let tpl = t(BOTH_SIDES);
        for seed in 0..200 {
            let q = tpl.instantiate(seed).unwrap();
            // Pull the numbers back out of the text and check the answer solves the equation.
            let v = q.answer.trim_matches('$').split(" = ").next().unwrap().to_string();
            let x: f64 = q.answer.trim_matches('$').split(" = ").nth(1).unwrap().parse().unwrap();
            let body = q.text.trim_start_matches("Solve $").trim_end_matches('$');
            let (l, r) = body.split_once(" = ").unwrap();
            let side = |s: &str| -> f64 {
                let s = s.replace(&v, "*X").replace(' ', "");
                let mut total = 0.0;
                let mut cur = String::new();
                let mut sign = 1.0;
                let flush = |cur: &mut String, sign: f64, total: &mut f64| {
                    if cur.is_empty() {
                        return;
                    }
                    let term = if let Some(c) = cur.strip_suffix("*X") {
                        let c = c.trim();
                        (if c.is_empty() { 1.0 } else { c.parse().unwrap() }) * x
                    } else {
                        cur.parse().unwrap()
                    };
                    *total += sign * term;
                    cur.clear();
                };
                for ch in s.chars() {
                    if ch == '+' || ch == '-' {
                        flush(&mut cur, sign, &mut total);
                        sign = if ch == '+' { 1.0 } else { -1.0 };
                    } else {
                        cur.push(ch);
                    }
                }
                flush(&mut cur, sign, &mut total);
                total
            };
            assert!((side(l) - side(r)).abs() < 1e-9, "seed {seed}: {} -> {}", q.text, q.answer);
        }
    }

    #[test]
    fn check_accepts_a_good_template_and_names_the_problem_in_a_bad_one() {
        assert!(t(BOTH_SIDES).check().is_ok());
        let bad = BOTH_SIDES.replace("(a - c) * x + b", "(a - c) * z + b");
        assert!(t(&bad).check().unwrap_err().contains("`z` is not a variable"));
        let impossible = BOTH_SIDES.replace("\"a > c\"", "\"a > 50\"");
        assert!(t(&impossible).check().unwrap_err().contains("no numbers"));
        let unknown_filter = BOTH_SIDES.replace("|coef]][[v]] + ", "|wibble]][[v]] + ");
        assert!(t(&unknown_filter).check().unwrap_err().contains("not a filter"));
        let junk = r#"{"id":"x","topic":"T","difficulty":1,"vars":[["a",{"int":[1,2],"wrong":1}]],"text":"t","answer":"a"}"#;
        assert!(serde_json::from_str::<Template>(junk).is_err(), "unknown variable settings are refused");
    }

    #[test]
    fn filters_write_signs_fractions_and_places() {
        let mut s = Scope::new();
        for (k, v) in [("a", 1.0), ("b", -3.0), ("c", 0.0), ("r", 0.75), ("p", 22.6234)] {
            s.insert(k.into(), Val::Num(v));
        }
        let f = |x: &str| fill_all(x, &s).unwrap().as_str().unwrap().to_string();
        assert_eq!(f("[[a|coef]]x"), "x");
        assert_eq!(f("[[b|coef]]x"), "-3x");
        assert_eq!(f("x[[b|signed]]"), "x - 3");
        assert_eq!(f("x[[c|signed]]"), "x");
        assert_eq!(f("x[[b|sterm:y]]"), "x - 3y");
        assert_eq!(f("[[r|frac]]"), "\\frac{3}{4}");
        assert_eq!(f("[[b|sgn]] and [[a|sgn]]"), "-3 and +1");
        assert_eq!(f("[[b < 0|if:Subtract/Add]]"), "Subtract");
        assert_eq!(f("[[4032|words]]"), "four thousand and thirty-two");
        assert_eq!(f("[[82000000|group]] [[4000|group]] [[12345.5|group]]"), "82\u{2009}000\u{2009}000 4000 12\u{2009}345.5");
        assert_eq!(f("[[56208|words]]"), "fifty-six thousand two hundred and eight");
        assert_eq!(f("[[1500000|words]]"), "one million five hundred thousand");
        assert_eq!(f("[[sf(4.862, 2)]] [[sf(0.003416, 1)]] [[sf(7260, 1)]]"), "4.9 0.003 7000");
        assert_eq!(f("[[p|dp:1]]"), "22.6");
        assert_eq!(f("[[a + b * 2]]"), "-5");
        assert_eq!(f("[[1/3|frac]]"), "\\frac{1}{3}");
    }

    #[test]
    fn a_wrong_answer_formula_is_caught_by_the_templates_own_check() {
        let wrong = BOTH_SIDES.replace(
            "\"calc\": [[\"d\", \"(a - c) * x + b\"]],",
            "\"calc\": [[\"d\", \"(a - c) * x + b + 1\"]], \"check\": [\"a * x + b == c * x + d\"],",
        );
        let err = t(&wrong).check().unwrap_err();
        assert!(err.contains("its own check") && err.contains("wrong"), "{err}");
        let right = BOTH_SIDES.replace("\"where\"", "\"check\": [\"a * x + b == c * x + d\"], \"where\"");
        assert!(t(&right).check().is_ok());
    }

    #[test]
    fn lint_flags_signs_money_and_unused_variables_and_the_built_in_templates_are_clean() {
        let sloppy = t(r#"{"id":"n","topic":"Number","difficulty":1,
            "vars":[["a",{"int":[1,9]}],["b",{"int":[-9,9],"not":[0]}],["unused",{"int":[1,3]}]],
            "calc":[["m","a * 5 / 2"]],
            "text":"Work out $[[a]] + [[b]]$","answer":"£[[m]]"}"#);
        let w = sloppy.lint().join(" | ");
        assert!(w.contains("right after a sign"), "{w}");
        assert!(w.contains("two decimal places"), "{w}");
        assert!(w.contains("`unused` is never used"), "{w}");
        assert!(w.contains("no \"check\""), "{w}");
        for tpl in crate::questions::starter().unwrap().templates {
            assert!(tpl.lint().is_empty(), "{}: {:?}", tpl.id, tpl.lint());
        }
    }

    #[test]
    fn large_numbers_with_rounding_error_are_written_not_looped_over() {
        // 9.8 × 10^8 is 980000000.0000001 in floating point: it used to recurse for ever.
        let s = Scope::new();
        assert_eq!(fill_all("[[9.8 * 10^8|group]]", &s).unwrap(), "980\u{2009}000\u{2009}000");
        assert_eq!(fill_all("[[4.7 * 10^-5]]", &s).unwrap(), "0.000047");
        for m in 12..=98 {
            for k in 3..=9 {
                let t = Template {
                    id: "x".into(),
                    topic: "Number".into(),
                    subtopic: String::new(),
                    difficulty: 1,
                    keywords: vec![],
                    qtype: "fluency".into(),
                    source: String::new(),
                    vars: vec![],
                    calc: vec![],
                    conditions: vec![],
                    checks: vec![],
                    text: format!("[[{m} / 10 * 10^{k}|group]]"),
                    answer: "a".into(),
                    diagram: None,
                };
                assert!(t.instantiate(1).is_ok());
            }
        }
    }

    #[test]
    fn expressions_cannot_do_anything_but_maths() {
        let s = Scope::new();
        for bad in ["system(1)", "1 +", "2 $ 3", "((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))"] {
            assert!(eval(bad, &s).is_err(), "{bad}");
        }
        assert_eq!(eval("gcd(12, 18) + 2^3 % 5", &s).unwrap(), 9.0);
    }

    #[test]
    fn a_diagram_gets_numbers_and_labels() {
        let tpl = t(r#"{"id":"geo-t","topic":"Geometry","difficulty":1,
            "vars":[["a",{"int":[20,80]}]],
            "text":"Find $x$.","answer":"$x = [[180 - a]]°$",
            "diagram":{"kind":"angles-line","angles":["[[a]]°",null],"sizes":["[[a|n]]","[[180 - a|n]]"],"unknown":"x"}}"#);
        let q = tpl.instantiate(3).unwrap();
        let d = q.diagram.unwrap();
        assert!(d["angles"][0].as_str().unwrap().ends_with('°'));
        assert!(d["angles"][1].is_null());
        assert!(d["sizes"][0].is_number(), "{d}");
        assert!(d["sizes"][0].as_u64().is_some(), "whole numbers are integers: {d}");
        assert!(q.needs_diagram);
    }
}
