//! The form-style apps of the calculator: Table, Equation, Inequality, Distribution, Ratio,
//! Statistics, Matrix, Vector and Math Box. Each is a page of fields (typed with the same editor as
//! the Calculate line) that is solved into a list of answer lines.
#![allow(clippy::should_implement_trait, clippy::needless_range_loop)]

use crate::calc_matrix::{self, Mat};
use crate::calc_poly::{self, C, Cmp, Solution};
use crate::calc_stats::{self, RegKind};
use crate::calculator::{
    Angle, Calc, CalcError, Config, Env, Func, Key, NumFmt, ResultFormat, Tok, Value, evaluate_env, result_tokens, table_rows, tidy,
};

/// The apps that are a form.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Kind {
    Table,
    Equation,
    Inequality,
    Distribution,
    Ratio,
    Statistics,
    Matrix,
    Vector,
    MathBox,
}

/// One line of an answer: a label and a value (or a second value, for a table's g(x)).
#[derive(Clone, Debug)]
pub struct Row {
    pub label: String,
    pub value: Vec<Tok>,
    pub value2: Vec<Tok>,
    /// Shown instead of a value when this row has none (an error).
    pub note: &'static str,
}

impl Row {
    pub fn new(label: impl Into<String>, value: Vec<Tok>) -> Row {
        Row { label: label.into(), value, value2: vec![], note: "" }
    }

    pub fn text(label: impl Into<String>) -> Row {
        Row { label: label.into(), value: vec![], value2: vec![], note: "" }
    }
}

/// An editor that grows: data in columns, one row at a time.
#[derive(Clone)]
pub struct Grow {
    pub names: Vec<String>,
    pub max_fields: usize,
}

/// A page of fields you fill in one by one and then solve.
#[derive(Clone)]
pub struct Form {
    pub kind: Kind,
    /// Which of the app's types this is.
    pub variant: usize,
    pub title: String,
    pub labels: Vec<String>,
    pub fields: Vec<Calc>,
    pub focus: usize,
    pub rows: Option<Vec<Row>>,
    pub scroll: usize,
    pub grow: Option<Grow>,
    /// Which results the Statistics app shows (set before solving).
    pub view: usize,
    /// The focused field holds a default: the first thing typed replaces it.
    pub replace: bool,
}

impl Form {
    pub fn new(kind: Kind, variant: usize, title: &str, labels: &[&str], defaults: &[&str], like: &Calc) -> Form {
        let fields = (0..labels.len()).map(|i| Form::field(like, defaults.get(i).copied().unwrap_or(""))).collect();
        Form {
            kind,
            variant,
            title: title.into(),
            labels: labels.iter().map(|s| s.to_string()).collect(),
            fields,
            focus: 0,
            rows: None,
            scroll: 0,
            grow: None,
            view: 0,
            replace: true,
        }
    }

    fn field(like: &Calc, default: &str) -> Calc {
        let mut c = Calc::blank_like(like);
        for ch in default.chars() {
            c.press(match ch {
                '.' => Key::Dot,
                '-' => Key::Neg,
                d => Key::Digit(d as u8 - b'0'),
            });
        }
        c
    }

    /// A column editor with `cols` columns of names and room for `max_fields` cells.
    pub fn editor(kind: Kind, variant: usize, title: &str, names: &[&str], max_fields: usize, like: &Calc) -> Form {
        let mut f = Form::new(kind, variant, title, &[], &[], like);
        f.grow = Some(Grow { names: names.iter().map(|s| s.to_string()).collect(), max_fields });
        f.add_row(like);
        f
    }

    fn add_row(&mut self, like: &Calc) {
        let Some(g) = &self.grow else { return };
        let row = self.fields.len() / g.names.len().max(1) + 1;
        let labels: Vec<String> = g.names.iter().map(|n| format!("{n}{row}")).collect();
        for l in labels {
            self.labels.push(l);
            self.fields.push(Calc::blank_like(like));
        }
    }

    pub fn value(&self, i: usize) -> Result<Value, CalcError> {
        self.fields.get(i).ok_or(CalcError::Syntax)?.value(None)
    }

    pub fn real(&self, i: usize) -> Result<f64, CalcError> {
        Ok(self.value(i)?.to_f64())
    }

    /// A key press. Returns whether the answer was asked for.
    pub fn press(&mut self, key: Key) -> bool {
        let last = self.fields.len().saturating_sub(1);
        let before = self.focus;
        let solve = self.press_key(key, last);
        if self.focus != before {
            self.replace = true;
        }
        solve
    }

    fn press_key(&mut self, key: Key, last: usize) -> bool {
        match key {
            Key::Up => {
                if self.rows.is_some() {
                    self.scroll = self.scroll.saturating_sub(1);
                } else {
                    self.focus = self.focus.saturating_sub(1);
                }
            }
            Key::Down => {
                if let Some(rows) = &self.rows {
                    self.scroll = (self.scroll + 1).min(rows.len().saturating_sub(1));
                } else {
                    self.focus = (self.focus + 1).min(last);
                }
            }
            Key::PageUp => {
                if self.rows.is_some() {
                    self.scroll = self.scroll.saturating_sub(3);
                } else {
                    self.focus = self.focus.saturating_sub(3);
                }
            }
            Key::PageDown => {
                if let Some(rows) = &self.rows {
                    self.scroll = (self.scroll + 3).min(rows.len().saturating_sub(1));
                } else {
                    self.focus = (self.focus + 3).min(last);
                }
            }
            Key::Ok | Key::Equals => {
                if self.rows.is_some() {
                    self.rows = None;
                } else if self.grow.is_some() {
                    // In a data editor OK moves on (adding a row at the end); EXE asks for results.
                    if key == Key::Equals {
                        self.scroll = 0;
                        return true;
                    }
                    if self.focus < last {
                        self.focus += 1;
                    } else if self.fields.len() < self.grow.as_ref().map_or(0, |g| g.max_fields)
                        && let Some(f) = self.fields.last().cloned()
                    {
                        self.add_row(&f);
                        self.focus += 1;
                    }
                } else if self.focus < last {
                    self.focus += 1;
                } else {
                    self.scroll = 0;
                    return true;
                }
            }
            Key::Ac => {
                if self.rows.is_some() {
                    self.rows = None;
                } else if let Some(f) = self.fields.get_mut(self.focus) {
                    *f = Calc::blank_like(f);
                }
            }
            other => {
                self.rows = None;
                let replace = std::mem::take(&mut self.replace) && !matches!(other, Key::Left | Key::Right | Key::Del);
                if let Some(f) = self.fields.get_mut(self.focus) {
                    if replace {
                        *f = Calc::blank_like(f);
                    }
                    f.press(other);
                }
                if matches!(other, Key::Left | Key::Right | Key::Del) {
                    self.replace = false;
                }
            }
        }
        false
    }

    pub fn set_angle(&mut self, d: Angle) {
        for f in &mut self.fields {
            f.angle = d;
        }
    }

    /// The cells of a data editor as rows of values; empty cells are skipped.
    fn data(&self, cols: usize) -> Result<Vec<Vec<f64>>, CalcError> {
        let mut out = vec![];
        for row in self.fields.chunks(cols) {
            if row.iter().all(|f| f.tokens.is_empty()) {
                continue;
            }
            // A partly filled row is an error; a missing frequency means 1.
            let vals: Result<Vec<f64>, CalcError> = row.iter().map(|f| f.value(None).map(|v| v.to_f64())).collect();
            out.push(vals?);
        }
        Ok(out)
    }
}

// ---- Answer lines --------------------------------------------------------------------------

pub fn val(v: Value, cfg: &Config) -> Vec<Tok> {
    result_tokens(v, ResultFormat::Standard, cfg)
}

/// A decimal answer from a float, as the screen would show it.
pub fn fl(x: f64, cfg: &Config) -> Vec<Tok> {
    result_tokens(Value::Float(x), ResultFormat::Decimal, &Config { io: crate::calculator::Io::MathDecimal, ..*cfg })
}

/// `a+bi` as tokens, in decimals.
pub fn complex_tokens(z: C, cfg: &Config) -> Vec<Tok> {
    let (re, im) = (clean(z.re), clean(z.im));
    let mut out = vec![];
    if re != 0.0 || im == 0.0 {
        out.extend(fl(re, cfg));
    }
    if im != 0.0 {
        if im > 0.0 && !out.is_empty() {
            out.push(Tok::Plus);
        }
        if im < 0.0 {
            out.push(Tok::Minus);
        }
        if im.abs() != 1.0 {
            out.extend(fl(im.abs(), cfg));
        }
        out.push(Tok::I);
    }
    out
}

/// Ten significant figures, so noise like 1.0000000000000002 shows as 1.
fn clean(x: f64) -> f64 {
    format!("{x:.9e}").parse().unwrap_or(x)
}

fn text_of(toks: &[Tok]) -> String {
    toks.iter().map(|t| t.text()).collect()
}

// ---- Building forms ------------------------------------------------------------------------

pub const TABLE_TYPES: [&str; 2] = ["f(x)", "f(x),g(x)"];
pub const EQUATION_TYPES: [&str; 3] = ["Simul Equation", "Polynomial", "Solver"];
pub const DISTRIBUTION_TYPES: [&str; 7] = ["Binomial PD", "Binomial CD", "Normal PD", "Normal CD", "Inverse Normal", "Poisson PD", "Poisson CD"];
pub const RATIO_TYPES: [&str; 2] = ["a:b=X:d", "a:b=c:X"];
pub const MATHBOX_TYPES: [&str; 3] = ["Dice Roll", "Coin Toss", "Circle"];
pub const MATRIX_OPS: [&str; 11] = [
    "Edit MatA",
    "Edit MatB",
    "Edit MatC",
    "Edit MatD",
    "MatA+MatB",
    "MatA\u{2212}MatB",
    "MatA\u{00D7}MatB",
    "Det MatA",
    "Trn MatA",
    "MatA\u{207B}\u{00B9}",
    "k\u{00D7}MatA",
];
pub const VECTOR_OPS: [&str; 11] = [
    "Edit VctA",
    "Edit VctB",
    "Edit VctC",
    "Edit VctD",
    "VctA+VctB",
    "VctA\u{2212}VctB",
    "VctA\u{00B7}VctB",
    "VctA\u{00D7}VctB",
    "Abs(VctA)",
    "Angle(A,B)",
    "Unitv(VctA)",
];

/// The form for `kind` and `variant`, using `like` for the angle unit, settings and memory.
pub fn build(kind: Kind, variant: usize, like: &Calc) -> Form {
    let f = |title: &str, labels: &[&str], defaults: &[&str]| Form::new(kind, variant, title, labels, defaults, like);
    match kind {
        Kind::Table => {
            if variant == 0 {
                f("Table", &["f(x)=", "Start", "End", "Step"], &["", "1", "5", "1"])
            } else {
                f("Table", &["f(x)=", "g(x)=", "Start", "End", "Step"], &["", "", "1", "5", "1"])
            }
        }
        Kind::Equation => match variant {
            0..=2 => {
                let n = variant + 2;
                let names: Vec<String> = (1..=n).flat_map(|r| (0..=n).map(move |c| format!("{}{r}", ["a", "b", "c", "d", "e"][c]))).collect();
                let labels: Vec<&str> = names.iter().map(String::as_str).collect();
                f(&format!("{n} unknowns"), &labels, &[])
            }
            3..=5 => {
                let deg = variant - 1;
                f(&format!("Degree {deg}"), &["a", "b", "c", "d", "e"][..=deg], &[])
            }
            _ => f("Solver", &["Eq", "x\u{2248}"], &["", "1"]),
        },
        Kind::Inequality => {
            let deg = variant / 4 + 2;
            f(&format!("Degree {deg}"), &["a", "b", "c", "d", "e"][..=deg], &[])
        }
        Kind::Distribution => match variant {
            0 | 1 => f(DISTRIBUTION_TYPES[variant], &["x", "N", "p"], &["", "", ""]),
            2 => f("Normal PD", &["x", "\u{03C3}", "\u{03BC}"], &["", "1", "0"]),
            3 => f("Normal CD", &["Lower", "Upper", "\u{03C3}", "\u{03BC}"], &["", "", "1", "0"]),
            4 => f("Inverse Normal", &["Area", "\u{03C3}", "\u{03BC}"], &["", "1", "0"]),
            _ => f(DISTRIBUTION_TYPES[variant], &["x", "\u{03BB}"], &["", ""]),
        },
        Kind::Ratio => {
            if variant == 0 {
                f("a:b=X:d", &["a", "b", "d"], &[])
            } else {
                f("a:b=c:X", &["a", "b", "c"], &[])
            }
        }
        Kind::Statistics => {
            // Variant bits: 1 = paired, 2 = frequency column.
            let names: Vec<&str> = match variant {
                0 => vec!["x"],
                1 => vec!["x", "y"],
                2 => vec!["x", "f"],
                _ => vec!["x", "y", "f"],
            };
            let max = match names.len() {
                1 => 160,
                2 => 160,
                _ => 159,
            };
            Form::editor(kind, variant, "Statistics", &names, max, like)
        }
        Kind::Matrix => match variant {
            0..=3 => f("Size", &["rows", "cols"], &["2", "2"]),
            10 => f("k\u{00D7}MatA", &["k"], &[]),
            _ => f("", &[""], &[]),
        },
        Kind::Vector => match variant {
            0..=3 => f("Size", &["dim"], &["3"]),
            _ => f("", &[""], &[]),
        },
        Kind::MathBox => match variant {
            0 => f("Dice Roll", &["Dice", "Rolls"], &["1", "1"]),
            1 => f("Coin Toss", &["Coins", "Tosses"], &["1", "1"]),
            _ => f("Circle", &["Angle"], &["30"]),
        },
    }
}

// ---- Solving -------------------------------------------------------------------------------

/// Matrices and vectors the Matrix and Vector apps hold.
#[derive(Clone, Debug)]
pub struct Store {
    pub mats: [Mat; 4],
    pub vcts: [Vec<Value>; 4],
}

impl Default for Store {
    fn default() -> Self {
        Store { mats: [Mat::new(2, 2), Mat::new(2, 2), Mat::new(2, 2), Mat::new(2, 2)], vcts: [vec![], vec![], vec![], vec![]] }
    }
}

/// The statistical values the Statistics Calc screen can recall, by name.
pub const STAT_NAMES: [&str; 27] = [
    "n",
    "x-bar",
    "\u{03C3}x",
    "sx",
    "\u{03A3}x",
    "\u{03A3}x\u{00B2}",
    "min(x)",
    "Q1",
    "Med",
    "Q3",
    "max(x)",
    "y-bar",
    "\u{03C3}y",
    "sy",
    "\u{03A3}y",
    "\u{03A3}y\u{00B2}",
    "\u{03A3}xy",
    "min(y)",
    "max(y)",
    "a",
    "b",
    "c",
    "r",
    "\u{03A3}x\u{00B3}",
    "\u{03A3}x\u{00B2}y",
    "\u{03A3}x\u{2074}",
    "\u{03C3}\u{00B2}x",
];

/// The data in a Statistics form, as `(x, freq)` or `(x, y, freq)` rows.
fn stat_cols(f: &Form) -> (usize, bool) {
    let paired = f.variant & 1 == 1;
    let freq = f.variant & 2 == 2;
    (usize::from(paired) + 1 + usize::from(freq), paired)
}

pub fn stat_one(f: &Form) -> Result<Vec<(f64, f64)>, CalcError> {
    let (cols, _) = stat_cols(f);
    let freq = f.variant & 2 == 2;
    Ok(f.data(cols)?.into_iter().map(|r| (r[0], if freq { r[1] } else { 1.0 })).collect())
}

pub fn stat_two(f: &Form) -> Result<Vec<(f64, f64, f64)>, CalcError> {
    let (cols, _) = stat_cols(f);
    let freq = f.variant & 2 == 2;
    Ok(f.data(cols)?.into_iter().map(|r| (r[0], r[1], if freq { r[2] } else { 1.0 })).collect())
}

/// Every statistic the form's data has, for the calculator's Statistics Calc screen (NaN if none).
pub fn stat_values(f: &Form, reg: RegKind) -> [f64; 27] {
    let mut out = [f64::NAN; 27];
    let paired = f.variant & 1 == 1;
    if paired {
        let Some(t) = stat_two(f).ok().and_then(|d| calc_stats::two_var(&d)) else { return out };
        let vals = [
            t.n,
            t.mean_x,
            t.sigma_x,
            t.s_x.unwrap_or(f64::NAN),
            t.sum_x,
            t.sum_x2,
            t.min_x,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            t.max_x,
            t.mean_y,
            t.sigma_y,
            t.s_y.unwrap_or(f64::NAN),
            t.sum_y,
            t.sum_y2,
            t.sum_xy,
            t.min_y,
            t.max_y,
        ];
        out[..19].copy_from_slice(&vals);
        out[23] = t.sum_x3;
        out[24] = t.sum_x2y;
        out[25] = t.sum_x4;
        out[26] = t.sigma_x * t.sigma_x;
        if let Some(r) = stat_two(f).ok().and_then(|d| calc_stats::regression(reg, &d)) {
            (out[19], out[20], out[21], out[22]) = (r.a, r.b, r.c, r.r.unwrap_or(f64::NAN));
        }
    } else if let Some(o) = stat_one(f).ok().and_then(|d| calc_stats::one_var(&d)) {
        out[..11].copy_from_slice(&[o.n, o.mean, o.sigma, o.s.unwrap_or(f64::NAN), o.sum_x, o.sum_x2, o.min, o.q1, o.med, o.q3, o.max]);
        out[26] = o.sigma * o.sigma;
    }
    out
}

fn stat_rows(f: &Form, cfg: &Config) -> Result<Vec<Row>, CalcError> {
    let paired = f.variant & 1 == 1;
    let n = |x: f64| fl(x, cfg);
    let opt = |x: Option<f64>| x.map_or_else(Vec::new, n);
    match (paired, f.view) {
        (false, _) => {
            let o = calc_stats::one_var(&stat_one(f)?).ok_or(CalcError::Math)?;
            Ok(vec![
                Row::new("x-bar", n(o.mean)),
                Row::new("\u{03A3}x", n(o.sum_x)),
                Row::new("\u{03A3}x\u{00B2}", n(o.sum_x2)),
                Row::new("\u{03C3}x", n(o.sigma)),
                Row::new("sx", opt(o.s)),
                Row::new("n", n(o.n)),
                Row::new("min(x)", n(o.min)),
                Row::new("Q1", n(o.q1)),
                Row::new("Med", n(o.med)),
                Row::new("Q3", n(o.q3)),
                Row::new("max(x)", n(o.max)),
            ])
        }
        (true, 0) => {
            let t = calc_stats::two_var(&stat_two(f)?).ok_or(CalcError::Math)?;
            Ok(vec![
                Row::new("x-bar", n(t.mean_x)),
                Row::new("\u{03A3}x", n(t.sum_x)),
                Row::new("\u{03A3}x\u{00B2}", n(t.sum_x2)),
                Row::new("\u{03C3}x", n(t.sigma_x)),
                Row::new("sx", opt(t.s_x)),
                Row::new("n", n(t.n)),
                Row::new("y-bar", n(t.mean_y)),
                Row::new("\u{03A3}y", n(t.sum_y)),
                Row::new("\u{03A3}y\u{00B2}", n(t.sum_y2)),
                Row::new("\u{03C3}y", n(t.sigma_y)),
                Row::new("sy", opt(t.s_y)),
                Row::new("\u{03A3}xy", n(t.sum_xy)),
                Row::new("min(x)", n(t.min_x)),
                Row::new("max(x)", n(t.max_x)),
                Row::new("min(y)", n(t.min_y)),
                Row::new("max(y)", n(t.max_y)),
            ])
        }
        (true, v) => {
            let kind = RegKind::ALL.get(v - 1).copied().ok_or(CalcError::Syntax)?;
            let r = calc_stats::regression(kind, &stat_two(f)?).ok_or(CalcError::Math)?;
            let mut rows = vec![Row::new("a", n(r.a)), Row::new("b", n(r.b))];
            if kind == RegKind::Quadratic {
                rows.push(Row::new("c", n(r.c)));
            } else {
                rows.push(Row::new("r", opt(r.r)));
            }
            Ok(rows)
        }
    }
}

fn poly_rows(c: &[Value], cfg: &Config) -> Result<Vec<Row>, CalcError> {
    let coefs: Vec<f64> = c.iter().map(|v| v.to_f64()).collect();
    if coefs.last().is_none_or(|k| *k == 0.0) {
        return Err(CalcError::Math);
    }
    let deg = coefs.len() - 1;
    let mut rows = vec![];
    if deg == 2 {
        // Quadratics are exact when they can be.
        let disc = c[1].to_f64().powi(2) - 4.0 * c[2].to_f64() * c[0].to_f64();
        if disc >= 0.0
            && let Ok(rs) = crate::calculator::quadratic(c[2], c[1], c[0])
        {
            for (n, v) in rs {
                rows.push(Row::new(n, val(v, cfg)));
            }
        }
    }
    if rows.is_empty() {
        let rs = calc_poly::roots(&coefs);
        let name = |i: usize| if rs.len() == 1 { "x".to_string() } else { format!("x{}", i + 1) };
        for (i, r) in rs.iter().enumerate() {
            rows.push(Row::new(name(i), complex_tokens(*r, cfg)));
        }
    }
    if deg == 2
        && let Some((x, y, min)) = calc_poly::vertex(coefs[2], coefs[1], coefs[0])
    {
        let w = if min { "Min" } else { "Max" };
        rows.push(Row::new(format!("{w} x"), fl(x, cfg)));
        rows.push(Row::new(format!("{w} y"), fl(y, cfg)));
    }
    Ok(rows)
}

fn solution_rows(s: &Solution, cfg: &Config) -> Vec<Row> {
    let num = |x: f64| text_of(&fl(x, cfg));
    match s {
        Solution::None => vec![Row::text("No Solution")],
        Solution::All => vec![Row::text("All Real Numbers")],
        Solution::Intervals(v) => v
            .iter()
            .map(|(lo, hi, lc, hc)| {
                let (l, h) = (if *lc { "\u{2264}" } else { "<" }, if *hc { "\u{2264}" } else { "<" });
                let text = match (lo.is_finite(), hi.is_finite()) {
                    (false, true) => format!("x{h}{}", num(*hi)),
                    (true, false) => format!("x{}{}", if *lc { "\u{2265}" } else { ">" }, num(*lo)),
                    (true, true) if lo == hi => format!("x={}", num(*lo)),
                    _ => format!("{}{l}x{h}{}", num(*lo), num(*hi)),
                };
                Row::text(text)
            })
            .collect(),
    }
}

/// Split at the `=` sign.
fn sides(toks: &[Tok]) -> Option<(&[Tok], &[Tok])> {
    let at = toks.iter().position(|t| *t == Tok::EqSign)?;
    Some((&toks[..at], &toks[at + 1..]))
}

/// A cheap random number for dice and coins.
fn random() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static S: AtomicU64 = AtomicU64::new(0x2545_F491_4F6C_DD1D);
    let mut x = S.load(Ordering::Relaxed) ^ std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    S.store(x | 1, Ordering::Relaxed);
    x
}

/// Solve the form into answer lines.
pub fn solve(f: &Form, st: &Store, cfg: &Config) -> Result<Vec<Row>, CalcError> {
    let any = f.fields.first().ok_or(CalcError::Syntax)?;
    let (angle, mem) = (any.angle, &any.mem);
    match f.kind {
        Kind::Table => {
            let two = f.variant == 1;
            let (fx, gx) = (&f.fields[0], if two { Some(&f.fields[1]) } else { None });
            let k = if two { 2 } else { 1 };
            if fx.tokens.is_empty() || gx.is_some_and(|g| g.tokens.is_empty()) {
                return Err(CalcError::Syntax);
            }
            let (a, b, c) = (f.value(k)?, f.value(k + 1)?, f.value(k + 2)?);
            let fr = table_rows(&fx.tokens, a, b, c, angle, mem)?;
            let gr = match gx {
                Some(g) => Some(table_rows(&g.tokens, a, b, c, angle, mem)?),
                None => None,
            };
            Ok(fr
                .into_iter()
                .enumerate()
                .map(|(i, (x, y))| {
                    let mut row = Row::text(text_of(&val(x, &Config { io: crate::calculator::Io::MathDecimal, ..*cfg })));
                    match y {
                        Ok(v) => row.value = val(v, cfg),
                        Err(e) => row.note = e.message(),
                    }
                    if let Some(g) = &gr
                        && let Some((_, Ok(v))) = g.get(i)
                    {
                        row.value2 = val(*v, cfg);
                    }
                    row
                })
                .collect())
        }
        Kind::Equation => match f.variant {
            0..=2 => {
                let n = f.variant + 2;
                let rows: Vec<Vec<Value>> =
                    (0..n).map(|r| (0..=n).map(|c| f.value(r * (n + 1) + c)).collect::<Result<_, _>>()).collect::<Result<_, _>>()?;
                let sol = calc_poly::solve_exact(&rows)?;
                Ok(sol.into_iter().zip(["x", "y", "z", "t"]).map(|(v, n)| Row::new(n, val(v, cfg))).collect())
            }
            3..=5 => {
                let c: Vec<Value> = (0..f.fields.len()).map(|i| f.value(i)).collect::<Result<_, _>>()?;
                // The form lists a (the lead) first; the roots code wants ascending powers.
                let asc: Vec<Value> = c.into_iter().rev().collect();
                poly_rows(&asc, cfg)
            }
            _ => {
                let toks = &f.fields[0].tokens;
                let (l, r) = sides(toks).ok_or(CalcError::Syntax)?;
                let guess = f.real(1)?;
                let eval = |side: &[Tok], x: f64| {
                    evaluate_env(side, &Env { ans: Value::int(0), angle, x: Some(Value::Float(x)), mem, num: NumFmt::Norm(2), cells: None })
                        .map(|v| v.to_f64())
                };
                let mut g = |x: f64| Some(eval(l, x).ok()? - eval(r, x).ok()?);
                let (x, resid) = calc_poly::solve_numeric(&mut g, guess).ok_or(CalcError::Math)?;
                Ok(vec![Row::new("x=", fl(x, cfg)), Row::new("L\u{2212}R=", fl(resid, cfg))])
            }
        },
        Kind::Inequality => {
            let cmp = Cmp::ALL[f.variant % 4];
            let mut c: Vec<f64> = (0..f.fields.len()).map(|i| f.real(i)).collect::<Result<_, _>>()?;
            if c.first().is_none_or(|a| *a == 0.0) {
                return Err(CalcError::Math);
            }
            c.reverse();
            let mut rows = vec![Row::text(format!("{}{}0", poly_title(c.len() - 1), cmp.text()))];
            rows.extend(solution_rows(&calc_poly::inequality(&c, cmp), cfg));
            Ok(rows)
        }
        Kind::Distribution => {
            let v = |i| f.real(i);
            let p = |x: f64| Ok(vec![Row::new("p=", fl(x, cfg))]);
            match f.variant {
                0 => {
                    let (x, n, pr) = (v(0)?, v(1)?, v(2)?);
                    if !(0.0..=1.0).contains(&pr) || n < 1.0 || n.fract() != 0.0 || x.fract() != 0.0 || x < 0.0 {
                        return Err(CalcError::Math);
                    }
                    p(calc_stats::binomial_pd(n, pr, x))
                }
                1 => {
                    let (x, n, pr) = (v(0)?, v(1)?, v(2)?);
                    if !(0.0..=1.0).contains(&pr) || n < 1.0 || n.fract() != 0.0 || x.fract() != 0.0 || x < 0.0 {
                        return Err(CalcError::Math);
                    }
                    p(calc_stats::binomial_cd(n, pr, x))
                }
                2 => {
                    let (x, s, m) = (v(0)?, v(1)?, v(2)?);
                    if s <= 0.0 {
                        return Err(CalcError::Math);
                    }
                    p(calc_stats::normal_pdf(x, m, s))
                }
                3 => {
                    let (lo, hi, s, m) = (v(0)?, v(1)?, v(2)?, v(3)?);
                    if s <= 0.0 || lo > hi {
                        return Err(CalcError::Math);
                    }
                    Ok(vec![
                        Row::new("p=", fl(calc_stats::normal_cdf(hi, m, s) - calc_stats::normal_cdf(lo, m, s), cfg)),
                        Row::new("z Low=", fl((lo - m) / s, cfg)),
                        Row::new("z Up=", fl((hi - m) / s, cfg)),
                    ])
                }
                4 => {
                    let (a, s, m) = (v(0)?, v(1)?, v(2)?);
                    Ok(vec![Row::new("xInv=", fl(calc_stats::inv_normal(a, m, s).ok_or(CalcError::Math)?, cfg))])
                }
                5 => {
                    let (x, l) = (v(0)?, v(1)?);
                    if l <= 0.0 || x < 0.0 || x.fract() != 0.0 {
                        return Err(CalcError::Math);
                    }
                    p(calc_stats::poisson_pd(l, x))
                }
                _ => {
                    let (x, l) = (v(0)?, v(1)?);
                    if l <= 0.0 || x < 0.0 || x.fract() != 0.0 {
                        return Err(CalcError::Math);
                    }
                    p(calc_stats::poisson_cd(l, x))
                }
            }
        }
        Kind::Ratio => {
            let (a, b, c) = (f.value(0)?, f.value(1)?, f.value(2)?);
            if a.is_zero() || b.is_zero() {
                return Err(CalcError::Math);
            }
            // a:b = X:d gives X = a·d/b; a:b = c:X gives X = b·c/a
            let x = if f.variant == 0 {
                crate::calculator::div(crate::calculator::mul(a, c)?, b)?
            } else {
                crate::calculator::div(crate::calculator::mul(b, c)?, a)?
            };
            Ok(vec![Row::new("X=", val(x, cfg))])
        }
        Kind::Statistics => stat_rows(f, cfg),
        Kind::Matrix => {
            let m = |i: usize| &st.mats[i];
            let show = |m: &Mat| -> Vec<Row> {
                (0..m.rows)
                    .map(|r| {
                        let mut toks = vec![];
                        for c in 0..m.cols {
                            if c > 0 {
                                toks.push(Tok::Space);
                            }
                            toks.extend(val(m.at(r, c), cfg));
                        }
                        Row::new(format!("{}", r + 1), toks)
                    })
                    .collect()
            };
            match f.variant {
                4 => Ok(show(&m(0).plus(m(1))?)),
                5 => Ok(show(&m(0).minus(m(1))?)),
                6 => Ok(show(&m(0).times(m(1))?)),
                7 => Ok(vec![Row::new("Det=", val(m(0).det()?, cfg))]),
                8 => Ok(show(&m(0).transpose())),
                9 => Ok(show(&m(0).inverse()?)),
                _ => Ok(show(&m(0).scale(f.value(0)?)?)),
            }
        }
        Kind::Vector => {
            let v = |i: usize| &st.vcts[i];
            let show = |a: &[Value]| -> Vec<Row> {
                let mut toks = vec![];
                for (i, x) in a.iter().enumerate() {
                    if i > 0 {
                        toks.push(Tok::Space);
                    }
                    toks.extend(val(*x, cfg));
                }
                vec![Row::new("", toks)]
            };
            let same = || if v(0).len() == v(1).len() && !v(0).is_empty() { Ok(()) } else { Err(CalcError::Math) };
            match f.variant {
                4 => {
                    same()?;
                    Ok(show(&v(0).iter().zip(v(1)).map(|(a, b)| crate::calculator::add(*a, *b)).collect::<Result<Vec<_>, _>>()?))
                }
                5 => {
                    same()?;
                    Ok(show(&v(0).iter().zip(v(1)).map(|(a, b)| crate::calculator::sub(*a, *b)).collect::<Result<Vec<_>, _>>()?))
                }
                6 => {
                    same()?;
                    Ok(vec![Row::new("A\u{00B7}B=", val(calc_matrix::dot(v(0), v(1))?, cfg))])
                }
                7 => Ok(show(&calc_matrix::cross(v(0), v(1))?)),
                8 => Ok(vec![Row::new("|A|=", val(calc_matrix::norm(v(0))?, cfg))]),
                9 => {
                    same()?;
                    Ok(vec![Row::new("\u{03B8}=", val(calc_matrix::angle_between(v(0), v(1), angle).map(|a| tidy(a.to_f64()))?, cfg))])
                }
                _ => Ok(show(&calc_matrix::unit(v(0))?)),
            }
        }
        Kind::MathBox => match f.variant {
            0 => {
                let (dice, rolls) = (f.real(0)? as usize, f.real(1)? as usize);
                if !(1..=3).contains(&dice) || !(1..=100).contains(&rolls) {
                    return Err(CalcError::Math);
                }
                let mut counts = [0u32; 6];
                let mut rows = vec![];
                for _ in 0..rolls {
                    let faces: Vec<usize> = (0..dice).map(|_| (random() % 6) as usize + 1).collect();
                    for d in &faces {
                        counts[d - 1] += 1;
                    }
                    rows.push(Row::text(faces.iter().map(usize::to_string).collect::<Vec<_>>().join(" ")));
                }
                rows.insert(0, Row::text(format!("1:{} 2:{} 3:{} 4:{} 5:{} 6:{}", counts[0], counts[1], counts[2], counts[3], counts[4], counts[5])));
                Ok(rows)
            }
            1 => {
                let (coins, tosses) = (f.real(0)? as usize, f.real(1)? as usize);
                if !(1..=3).contains(&coins) || !(1..=100).contains(&tosses) {
                    return Err(CalcError::Math);
                }
                let (mut h, mut t) = (0, 0);
                let mut rows = vec![];
                for _ in 0..tosses {
                    let faces: String = (0..coins).map(|_| if random() & 1 == 0 { 'H' } else { 'T' }).collect();
                    h += faces.matches('H').count();
                    t += faces.matches('T').count();
                    rows.push(Row::text(faces));
                }
                rows.insert(0, Row::text(format!("Heads:{h} Tails:{t}")));
                Ok(rows)
            }
            _ => {
                let a = f.value(0)?;
                let _ = a;
                let eval = |func: Func| {
                    let mut toks = vec![Tok::Func(func)];
                    toks.extend(f.fields[0].tokens.iter().copied());
                    toks.push(Tok::RParen);
                    evaluate_env(&toks, &Env { ans: Value::int(0), angle, x: None, mem, num: NumFmt::Norm(2), cells: None })
                };
                let tan = eval(Func::Tan);
                let mut rows = vec![Row::new("sin", val(eval(Func::Sin)?, cfg)), Row::new("cos", val(eval(Func::Cos)?, cfg))];
                match tan {
                    Ok(v) => rows.push(Row::new("tan", val(v, cfg))),
                    Err(e) => {
                        let mut r = Row::text("tan");
                        r.note = e.message();
                        rows.push(r);
                    }
                }
                Ok(rows)
            }
        },
    }
}

fn poly_title(deg: usize) -> &'static str {
    match deg {
        2 => "ax\u{00B2}+bx+c",
        3 => "ax\u{00B3}+bx\u{00B2}+cx+d",
        _ => "ax\u{2074}+bx\u{00B3}+cx\u{00B2}+dx+e",
    }
}

/// Build a matrix from the cells of a filled-in size form (rows × columns) and its data editor.
pub fn matrix_from(f: &Form, rows: usize, cols: usize) -> Result<Mat, CalcError> {
    let mut m = Mat::new(rows, cols);
    for r in 0..rows {
        for c in 0..cols {
            m.set(r, c, f.value(r * cols + c)?);
        }
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press_all(f: &mut Form, keys: &[Key]) {
        for k in keys {
            f.press(*k);
        }
    }

    fn type_num(f: &mut Form, n: &str) {
        for ch in n.chars() {
            f.press(match ch {
                '-' => Key::Neg,
                '.' => Key::Dot,
                d => Key::Digit(d as u8 - b'0'),
            });
        }
        f.press(Key::Ok);
    }

    fn solved(f: &Form) -> Vec<(String, String)> {
        solve(f, &Store::default(), &Config::default()).unwrap().iter().map(|r| (r.label.clone(), text_of(&r.value))).collect()
    }

    #[test]
    fn a_data_editor_grows_a_row_when_ok_passes_the_last_cell() {
        let mut f = Form::editor(Kind::Statistics, 0, "Statistics", &["x"], 160, &Calc::default());
        assert_eq!(f.fields.len(), 1);
        type_num(&mut f, "5");
        assert_eq!(f.fields.len(), 2);
        assert_eq!(f.labels, vec!["x1", "x2"]);
        let mut p = Form::editor(Kind::Statistics, 3, "Statistics", &["x", "y", "f"], 159, &Calc::default());
        assert_eq!(p.labels, vec!["x1", "y1", "f1"]);
        p.press(Key::Ok);
        p.press(Key::Ok);
        p.press(Key::Ok);
        assert_eq!(p.labels.len(), 6);
        assert!(p.press(Key::Equals), "EXE asks for the results");
    }

    #[test]
    fn one_variable_results_from_typed_data() {
        let mut f = Form::editor(Kind::Statistics, 0, "Statistics", &["x"], 160, &Calc::default());
        for n in ["1", "2", "3", "4", "5"] {
            type_num(&mut f, n);
        }
        let rows = solved(&f);
        let get = |l: &str| rows.iter().find(|r| r.0 == l).unwrap().1.clone();
        assert_eq!(get("x-bar"), "3");
        assert_eq!(get("n"), "5");
        assert_eq!(get("\u{03A3}x"), "15");
        assert_eq!(get("Med"), "3");
        assert_eq!(get("Q1"), "1.5");
        assert_eq!(get("max(x)"), "5");
    }

    #[test]
    fn frequencies_count_each_value_that_many_times() {
        let mut f = Form::editor(Kind::Statistics, 2, "Statistics", &["x", "f"], 160, &Calc::default());
        for n in ["1", "3", "5", "1"] {
            type_num(&mut f, n);
        }
        let rows = solved(&f);
        assert_eq!(rows.iter().find(|r| r.0 == "n").unwrap().1, "4");
        assert_eq!(rows.iter().find(|r| r.0 == "x-bar").unwrap().1, "2");
    }

    #[test]
    fn regression_results_name_their_coefficients() {
        let mut f = Form::editor(Kind::Statistics, 1, "Statistics", &["x", "y"], 160, &Calc::default());
        for n in ["1", "3", "2", "5", "3", "7"] {
            type_num(&mut f, n);
        }
        f.view = 1;
        let rows = solved(&f);
        assert_eq!(rows[0], ("a".into(), "1".into()));
        assert_eq!(rows[1], ("b".into(), "2".into()));
        assert_eq!(rows[2], ("r".into(), "1".into()));
        f.view = 2;
        assert_eq!(solved(&f).len(), 3, "quadratic: a, b, c");
    }

    #[test]
    fn missing_or_part_filled_data_is_an_error() {
        let f = Form::editor(Kind::Statistics, 0, "Statistics", &["x"], 160, &Calc::default());
        assert!(solve(&f, &Store::default(), &Config::default()).is_err(), "no data");
        let mut p = Form::editor(Kind::Statistics, 1, "Statistics", &["x", "y"], 160, &Calc::default());
        p.press(Key::Digit(1));
        assert!(solve(&p, &Store::default(), &Config::default()).is_err(), "x with no y");
    }

    #[test]
    fn simultaneous_equations_with_three_unknowns() {
        let mut f = build(Kind::Equation, 1, &Calc::default());
        for n in ["1", "1", "1", "6", "0", "2", "5", "-4", "2", "5", "-1", "27"] {
            type_num(&mut f, n);
        }
        assert_eq!(solved(&f), vec![("x".into(), "5".into()), ("y".into(), "3".into()), ("z".into(), "\u{2212}2".into())]);
    }

    #[test]
    fn polynomial_roots_exact_numeric_and_complex() {
        let mut f = build(Kind::Equation, 3, &Calc::default());
        for n in ["1", "-5", "6"] {
            type_num(&mut f, n);
        }
        let rows = solved(&f);
        assert_eq!(rows[0], ("x1".into(), "2".into()));
        assert_eq!(rows[1], ("x2".into(), "3".into()));
        assert_eq!(rows[2].0, "Min x");
        assert_eq!(rows[2].1, "2.5");
        // x² + 1: two complex roots, i and −i
        let mut f = build(Kind::Equation, 3, &Calc::default());
        for n in ["1", "0", "1"] {
            type_num(&mut f, n);
        }
        let rows = solved(&f);
        assert_eq!((rows[0].1.as_str(), rows[1].1.as_str()), ("i", "\u{2212}i"));
        // x³ − 6x² + 11x − 6
        let mut f = build(Kind::Equation, 4, &Calc::default());
        for n in ["1", "-6", "11", "-6"] {
            type_num(&mut f, n);
        }
        assert_eq!(solved(&f).iter().map(|r| r.1.clone()).collect::<Vec<_>>(), vec!["1", "2", "3"]);
    }

    #[test]
    fn the_solver_finds_a_root_of_a_typed_equation() {
        // x² − 4 = 0 from x ≈ 1 gives 2
        let mut f = build(Kind::Equation, 6, &Calc::default());
        for k in [Key::X, Key::Square, Key::Minus, Key::Digit(4), Key::EqSign, Key::Digit(0)] {
            f.press(k);
        }
        f.press(Key::Ok);
        let rows = solved(&f);
        assert_eq!(rows[0], ("x=".into(), "2".into()));
    }

    #[test]
    fn inequalities_are_described_in_words_of_the_guide() {
        // x² + 2x − 3 ≥ 0
        let mut f = build(Kind::Inequality, 2, &Calc::default());
        for n in ["1", "2", "-3"] {
            type_num(&mut f, n);
        }
        let rows = solved(&f);
        assert_eq!(rows[1].0, "x\u{2264}\u{2212}3");
        assert_eq!(rows[2].0, "x\u{2265}1");
        // x² < 0: no solution; x² ≥ 0: all reals
        let mut f = build(Kind::Inequality, 1, &Calc::default());
        for n in ["1", "0", "0"] {
            type_num(&mut f, n);
        }
        assert_eq!(solved(&f)[1].0, "No Solution");
        let mut f = build(Kind::Inequality, 2, &Calc::default());
        for n in ["1", "0", "0"] {
            type_num(&mut f, n);
        }
        assert_eq!(solved(&f)[1].0, "All Real Numbers");
    }

    #[test]
    fn distributions_from_the_guide() {
        // Binomial CD, x = 5, N = 5, p = 0.5 is 1
        let mut f = build(Kind::Distribution, 1, &Calc::default());
        for n in ["5", "5", "0.5"] {
            type_num(&mut f, n);
        }
        assert_eq!(solved(&f), vec![("p=".into(), "1".into())]);
        // Normal CD from −1.96 to 1.96 is 0.9500042097
        let mut f = build(Kind::Distribution, 3, &Calc::default());
        for n in ["-1.96", "1.96", "1", "0"] {
            type_num(&mut f, n);
        }
        let rows = solved(&f);
        assert_eq!(rows[0].1, "0.9500042097");
        assert_eq!(rows[1].1, "\u{2212}1.96");
        // Inverse normal at 0.975 is 1.959963985
        let mut f = build(Kind::Distribution, 4, &Calc::default());
        for n in ["0.975", "1", "0"] {
            type_num(&mut f, n);
        }
        assert_eq!(solved(&f)[0].1, "1.959963985");
        // A probability above 1 is a math error.
        let mut f = build(Kind::Distribution, 0, &Calc::default());
        for n in ["2", "5", "1.5"] {
            type_num(&mut f, n);
        }
        assert!(solve(&f, &Store::default(), &Config::default()).is_err());
    }

    #[test]
    fn ratio_solves_for_the_missing_term() {
        // 2:3 = X:12 → X = 8
        let mut f = build(Kind::Ratio, 0, &Calc::default());
        for n in ["2", "3", "12"] {
            type_num(&mut f, n);
        }
        assert_eq!(solved(&f), vec![("X=".into(), "8".into())]);
        // 2:3 = 5:X → X = 15/2
        let mut f = build(Kind::Ratio, 1, &Calc::default());
        for n in ["2", "3", "5"] {
            type_num(&mut f, n);
        }
        assert_eq!(solved(&f)[0].0, "X=");
        assert!(f.rows.is_none());
    }

    #[test]
    fn a_table_of_two_functions_has_both_columns() {
        let mut f = build(Kind::Table, 1, &Calc::default());
        press_all(&mut f, &[Key::X, Key::Square, Key::Ok, Key::Digit(2), Key::X, Key::Ok]);
        type_num(&mut f, "1");
        type_num(&mut f, "3");
        type_num(&mut f, "1");
        let rows = solve(&f, &Store::default(), &Config::default()).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!((text_of(&rows[2].value), text_of(&rows[2].value2)), ("9".to_string(), "6".to_string()));
        assert_eq!(rows[2].label, "3");
    }

    #[test]
    fn the_circle_gives_exact_trig_values() {
        let f = build(Kind::MathBox, 2, &Calc::default());
        let rows = solved(&f);
        assert_eq!(rows[0].0, "sin");
        assert_eq!(rows[1].0, "cos");
    }

    #[test]
    fn dice_and_coins_tally_what_was_rolled() {
        let mut f = build(Kind::MathBox, 0, &Calc::default());
        f.fields[0] = Form::field(&Calc::default(), "2");
        f.fields[1] = Form::field(&Calc::default(), "10");
        let rows = solve(&f, &Store::default(), &Config::default()).unwrap();
        assert_eq!(rows.len(), 11, "the tally and ten rolls");
        let total: u32 = rows[0].label.split_whitespace().map(|w| w.split(':').nth(1).unwrap().parse::<u32>().unwrap()).sum();
        assert_eq!(total, 20, "ten rolls of two dice");
        let mut c = build(Kind::MathBox, 1, &Calc::default());
        c.fields[1] = Form::field(&Calc::default(), "7");
        let rows = solve(&c, &Store::default(), &Config::default()).unwrap();
        assert!(rows[0].label.starts_with("Heads:"));
        assert_eq!(rows.len(), 8);
        let mut bad = build(Kind::MathBox, 0, &Calc::default());
        bad.fields[0] = Form::field(&Calc::default(), "9");
        assert!(solve(&bad, &Store::default(), &Config::default()).is_err());
    }

    #[test]
    fn matrix_operations_use_the_stored_matrices() {
        let mut st = Store::default();
        st.mats[0] = Mat::from_rows(&[&[1, 2], &[3, 4]]);
        st.mats[1] = Mat::from_rows(&[&[0, 1], &[1, 0]]);
        let mut f = build(Kind::Matrix, 6, &Calc::default());
        f.variant = 6;
        let rows = solve(&f, &st, &Config::default()).unwrap();
        let text: Vec<String> = rows.iter().map(|r| text_of(&r.value)).collect();
        assert_eq!(text, vec!["2  1".to_string(), "4  3".to_string()], "MatA × MatB, columns apart");
        f.variant = 7;
        assert_eq!(solved_with(&f, &st)[0], ("Det=".to_string(), "\u{2212}2".to_string()));
        f.variant = 9;
        assert_eq!(solve(&f, &st, &Config::default()).unwrap().len(), 2);
    }

    fn solved_with(f: &Form, st: &Store) -> Vec<(String, String)> {
        solve(f, st, &Config::default()).unwrap().iter().map(|r| (r.label.clone(), text_of(&r.value))).collect()
    }

    #[test]
    fn vector_operations() {
        let mut st = Store::default();
        st.vcts[0] = vec![Value::int(1), Value::int(2), Value::int(3)];
        st.vcts[1] = vec![Value::int(4), Value::int(5), Value::int(6)];
        let mut f = build(Kind::Vector, 6, &Calc::default());
        f.variant = 6;
        assert_eq!(solved_with(&f, &st)[0], ("A\u{00B7}B=".to_string(), "32".to_string()));
        f.variant = 9;
        let theta = solve(&f, &st, &Config::default()).unwrap();
        assert!(text_of(&theta[0].value).starts_with("12.9"), "{}", text_of(&theta[0].value));
        f.variant = 4;
        st.vcts[1] = vec![Value::int(1)];
        assert!(solve(&f, &st, &Config::default()).is_err(), "different sizes");
    }
}
