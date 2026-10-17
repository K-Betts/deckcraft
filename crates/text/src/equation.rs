//! Equations typed the way people type them: `x^2/3`, `sqrt(b^2-4ac)`, `sum_(i=1)^n i`.
//!
//! `parse` turns that text into an [`Expr`]; `to_omml` makes the real PowerPoint equation and
//! `to_plain` the plain Unicode stand-in drawn where the equation cannot be (see `Run::text`).
//! Anything can be typed: unknown characters pass through, brackets left open are closed, and
//! nothing panics.

/// A parsed equation.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Row(Vec<Expr>),
    /// A variable: drawn in italics.
    Var(char),
    /// Digits, operators and symbols: drawn upright.
    Text(String),
    /// A function name such as `sin`: drawn upright.
    Func(String),
    /// A box still to be filled in (`□`).
    Empty,
    Frac(Box<Expr>, Box<Expr>),
    Script {
        base: Box<Expr>,
        sub: Option<Box<Expr>>,
        sup: Option<Box<Expr>>,
    },
    Sqrt {
        degree: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    Nary {
        op: char,
        sub: Option<Box<Expr>>,
        sup: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    Delim {
        open: char,
        close: char,
        body: Box<Expr>,
    },
}

/// The character that marks a box still to be filled in.
pub const PLACEHOLDER: char = '□';

const FUNCS: &[&str] =
    &["arcsin", "arccos", "arctan", "sinh", "cosh", "tanh", "sin", "cos", "tan", "sec", "csc", "cot", "log", "ln", "exp", "lim", "min", "max", "det"];

const NAMES: &[(&str, char)] = &[
    ("alpha", 'α'),
    ("beta", 'β'),
    ("gamma", 'γ'),
    ("delta", 'δ'),
    ("epsilon", 'ε'),
    ("zeta", 'ζ'),
    ("eta", 'η'),
    ("theta", 'θ'),
    ("iota", 'ι'),
    ("kappa", 'κ'),
    ("lambda", 'λ'),
    ("mu", 'μ'),
    ("nu", 'ν'),
    ("xi", 'ξ'),
    ("pi", 'π'),
    ("rho", 'ρ'),
    ("sigma", 'σ'),
    ("tau", 'τ'),
    ("upsilon", 'υ'),
    ("phi", 'φ'),
    ("chi", 'χ'),
    ("psi", 'ψ'),
    ("omega", 'ω'),
    ("Gamma", 'Γ'),
    ("Delta", 'Δ'),
    ("Theta", 'Θ'),
    ("Lambda", 'Λ'),
    ("Xi", 'Ξ'),
    ("Pi", 'Π'),
    ("Sigma", 'Σ'),
    ("Phi", 'Φ'),
    ("Psi", 'Ψ'),
    ("Omega", 'Ω'),
    ("infinity", '∞'),
    ("infty", '∞'),
    ("inf", '∞'),
    ("times", '×'),
    ("div", '÷'),
    ("cdot", '·'),
    ("pm", '±'),
    ("mp", '∓'),
    ("leq", '≤'),
    ("geq", '≥'),
    ("neq", '≠'),
    ("le", '≤'),
    ("ge", '≥'),
    ("ne", '≠'),
    ("approx", '≈'),
    ("degrees", '°'),
    ("therefore", '∴'),
    ("because", '∵'),
    ("angle", '∠'),
    ("perp", '⊥'),
    ("parallel", '∥'),
    ("in", '∈'),
    ("subset", '⊂'),
    ("union", '∪'),
    ("cap", '∩'),
    ("emptyset", '∅'),
    ("forall", '∀'),
    ("exists", '∃'),
    ("to", '→'),
    ("implies", '⇒'),
    ("iff", '⇔'),
];

const MAX_DEPTH: usize = 40;

pub fn parse(s: &str) -> Expr {
    let mut p = Parser { c: s.chars().collect(), i: 0, depth: 0 };
    let mut items = vec![];
    // A stray closing bracket is just a character.
    while p.i < p.c.len() {
        match p.row(None) {
            Expr::Row(mut r) => items.append(&mut r),
            e => items.push(e),
        }
        if let Some(c) = p.peek() {
            p.i += 1;
            items.push(Expr::Text(c.to_string()));
        }
    }
    one(items)
}

/// Whether any box is still waiting to be filled in.
pub fn has_empty(e: &Expr) -> bool {
    let any = |o: &Option<Box<Expr>>| o.as_deref().is_some_and(has_empty);
    match e {
        Expr::Empty => true,
        Expr::Row(r) => r.iter().any(has_empty),
        Expr::Frac(a, b) => has_empty(a) || has_empty(b),
        Expr::Script { base, sub, sup } => has_empty(base) || any(sub) || any(sup),
        Expr::Sqrt { degree, body } => any(degree) || has_empty(body),
        Expr::Nary { sub, sup, body, .. } => any(sub) || any(sup) || has_empty(body),
        Expr::Delim { body, .. } => has_empty(body),
        Expr::Var(_) | Expr::Text(_) | Expr::Func(_) => false,
    }
}

fn one(mut items: Vec<Expr>) -> Expr {
    if items.len() == 1 { items.remove(0) } else { Expr::Row(items) }
}

struct Parser {
    c: Vec<char>,
    i: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.c.get(self.i).copied()
    }

    fn skip_spaces(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        s.chars().enumerate().all(|(k, ch)| self.c.get(self.i + k) == Some(&ch))
    }

    /// Terms up to the closing character (not consumed) or the end.
    fn row(&mut self, close: Option<char>) -> Expr {
        self.depth += 1;
        let mut items: Vec<Expr> = vec![];
        loop {
            self.skip_spaces();
            let Some(ch) = self.peek() else { break };
            if Some(ch) == close || (close.is_none() && matches!(ch, ')' | ']' | '}')) {
                break;
            }
            if ch == '/' {
                self.i += 1;
                self.skip_spaces();
                let den = self.term();
                let mut num = items.pop().unwrap_or(Expr::Empty);
                // `3x/4`: the number in front belongs to the top.
                let coefficient = matches!(&num, Expr::Var(_) | Expr::Script { .. })
                    && matches!(items.last(), Some(Expr::Text(t)) if t.chars().all(|c| c.is_ascii_digit() || c == '.'));
                if coefficient && let Some(n) = items.pop() {
                    num = Expr::Row(vec![n, num]);
                }
                items.push(Expr::Frac(Box::new(unwrap(num)), Box::new(unwrap(den))));
                continue;
            }
            items.push(self.term());
        }
        self.depth -= 1;
        one(items)
    }

    /// An atom with any powers and subscripts after it.
    fn term(&mut self) -> Expr {
        let base = self.atom();
        if matches!(base, Expr::Nary { .. }) {
            return base;
        }
        self.scripts(base)
    }

    fn scripts(&mut self, base: Expr) -> Expr {
        let (mut sub, mut sup) = (None, None);
        loop {
            match self.peek() {
                Some('^') if sup.is_none() => {
                    self.i += 1;
                    sup = Some(Box::new(self.script_arg()));
                }
                Some('_') if sub.is_none() => {
                    self.i += 1;
                    sub = Some(Box::new(self.script_arg()));
                }
                _ => break,
            }
        }
        if sub.is_some() || sup.is_some() { Expr::Script { base: Box::new(base), sub, sup } } else { base }
    }

    /// What follows `^` or `_`: a bracketed group, or just the next number or letter.
    fn script_arg(&mut self) -> Expr {
        self.skip_spaces();
        if self.peek() == Some('(') && self.depth < MAX_DEPTH {
            self.i += 1;
            let r = self.row(Some(')'));
            self.eat(')');
            return r;
        }
        let mut items = vec![];
        if let Some(sign @ ('-' | '+')) = self.peek() {
            self.i += 1;
            items.push(Expr::Text(if sign == '-' { "−".into() } else { "+".into() }));
        }
        match self.peek() {
            Some(c) if c.is_ascii_digit() => items.push(self.number()),
            Some(c) if c.is_alphabetic() => {
                let a = self.word_atom(true);
                items.push(a);
            }
            Some(PLACEHOLDER) => {
                self.i += 1;
                items.push(Expr::Empty);
            }
            Some(c) if !c.is_whitespace() && !matches!(c, ')' | ']' | '}') => {
                self.i += 1;
                items.push(Expr::Text(c.to_string()));
            }
            _ => {}
        }
        if items.is_empty() { Expr::Empty } else { one(items) }
    }

    fn number(&mut self) -> Expr {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || (c == '.' && self.c.get(self.i + 1).is_some_and(char::is_ascii_digit)) {
                s.push(c);
                self.i += 1;
            } else {
                break;
            }
        }
        Expr::Text(s)
    }

    fn atom(&mut self) -> Expr {
        self.skip_spaces();
        let Some(ch) = self.peek() else { return Expr::Empty };
        match ch {
            '(' | '[' | '{' if self.depth < MAX_DEPTH => {
                self.i += 1;
                let close = match ch {
                    '(' => ')',
                    '[' => ']',
                    _ => '}',
                };
                let body = self.row(Some(close));
                // A half-open interval such as [0,1) closes with whichever bracket comes.
                let end = match self.peek() {
                    Some(c @ (')' | ']' | '}')) => {
                        self.i += 1;
                        c
                    }
                    _ => close,
                };
                Expr::Delim { open: ch, close: end, body: Box::new(body) }
            }
            '|' if self.depth < MAX_DEPTH => {
                self.i += 1;
                let body = self.row(Some('|'));
                self.eat('|');
                Expr::Delim { open: '|', close: '|', body: Box::new(body) }
            }
            PLACEHOLDER => {
                self.i += 1;
                Expr::Empty
            }
            '0'..='9' => self.number(),
            '.' => {
                self.i += 1;
                Expr::Text(".".into())
            }
            '√' => {
                self.i += 1;
                self.root(None)
            }
            '∑' | '∏' | '∫' | '∬' | '∭' | '∮' => {
                self.i += 1;
                self.nary(ch)
            }
            '\\' => {
                self.i += 1;
                self.word_atom(false)
            }
            c if c.is_alphabetic() => self.word_atom(false),
            '<' if self.starts_with("<=>") => self.take("<=>", "⇔"),
            '<' if self.starts_with("<=") => self.take("<=", "≤"),
            '>' if self.starts_with(">=") => self.take(">=", "≥"),
            '!' if self.starts_with("!=") => self.take("!=", "≠"),
            '+' if self.starts_with("+-") => self.take("+-", "±"),
            '-' if self.starts_with("->") => self.take("->", "→"),
            '=' if self.starts_with("=>") => self.take("=>", "⇒"),
            '~' if self.starts_with("~=") => self.take("~=", "≈"),
            '-' => self.take("-", "−"),
            '*' => self.take("*", "·"),
            c => {
                self.i += 1;
                Expr::Text(c.to_string())
            }
        }
    }

    fn take(&mut self, src: &str, out: &str) -> Expr {
        self.i += src.chars().count();
        Expr::Text(out.into())
    }

    /// A word: a function or Greek name, `sqrt`, a sum sign, or a single variable.
    fn word_atom(&mut self, single: bool) -> Expr {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.i += 1;
        }
        let word: String = self.c[start..self.i].iter().collect();
        if word.is_empty() {
            // A letter outside a-z (α, é…): one variable.
            let c = self.peek().unwrap_or(' ');
            self.i += 1;
            return Expr::Var(c);
        }
        match word.as_str() {
            "sqrt" => return self.root(None),
            "cbrt" => return self.root(Some(Expr::Text("3".into()))),
            "sum" => return self.nary('∑'),
            "prod" => return self.nary('∏'),
            "int" | "integral" => return self.nary('∫'),
            _ => {}
        }
        if let Some(&(_, c)) = NAMES.iter().find(|(n, _)| *n == word) {
            return if matches!(c, 'α'..='ω' | 'Α'..='Ω') { Expr::Var(c) } else { Expr::Text(c.to_string()) };
        }
        if !single && let Some(f) = FUNCS.iter().filter(|f| word.starts_with(**f)).max_by_key(|f| f.len()) {
            self.i = start + f.len();
            return Expr::Func((*f).to_string());
        }
        // Not a word we know: one variable now, the rest next time.
        self.i = start + 1;
        Expr::Var(word.chars().next().unwrap_or(' '))
    }

    fn root(&mut self, degree: Option<Expr>) -> Expr {
        let mut degree = degree;
        if self.peek() == Some('[') && self.depth < MAX_DEPTH {
            self.i += 1;
            degree = Some(self.row(Some(']')));
            self.eat(']');
        }
        self.skip_spaces();
        let body = if self.peek() == Some('(') && self.depth < MAX_DEPTH {
            self.i += 1;
            let r = self.row(Some(')'));
            self.eat(')');
            r
        } else {
            self.term()
        };
        Expr::Sqrt { degree: degree.map(Box::new), body: Box::new(body) }
    }

    fn nary(&mut self, op: char) -> Expr {
        let (mut sub, mut sup) = (None, None);
        loop {
            match self.peek() {
                Some('^') if sup.is_none() => {
                    self.i += 1;
                    sup = Some(Box::new(self.script_arg()));
                }
                Some('_') if sub.is_none() => {
                    self.i += 1;
                    sub = Some(Box::new(self.script_arg()));
                }
                _ => break,
            }
        }
        self.skip_spaces();
        let body = match self.peek() {
            None | Some(')' | ']' | '}') => Expr::Empty,
            _ => self.term(),
        };
        Expr::Nary { op, sub, sup, body: Box::new(body) }
    }
}

/// `(a+b)/c` is a fraction of `a+b` and `c`, without the brackets.
fn unwrap(e: Expr) -> Expr {
    match e {
        Expr::Delim { open: '(', close: ')', body } => *body,
        e => e,
    }
}

// ---------------------------------------------------------------------------------------------
// The real equation

const NS_M: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";
const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// The equation as PowerPoint stores it (`m:oMathPara`, centred on its own line).
pub fn to_omml(e: &Expr) -> String {
    to_omml_sized(e, None)
}

/// The same, with each run carrying its size in points and the maths font, as PowerPoint writes
/// them: without that the size is lost when the file is saved.
pub fn to_omml_sized(e: &Expr, size: Option<f64>) -> String {
    let mut out = String::new();
    omml(e, &mut out);
    if let Some(pt) = size {
        let rpr = format!(
            "<a:rPr lang=\"en-GB\" sz=\"{}\" i=\"1\"><a:latin typeface=\"Cambria Math\" panose=\"02040503050406030204\" pitchFamily=\"18\" charset=\"0\"/></a:rPr>",
            (pt * 100.0).round() as i64
        );
        out = out.replace("</m:rPr><m:t>", &format!("</m:rPr>{rpr}<m:t>")).replace("<m:r><m:t>", &format!("<m:r>{rpr}<m:t>"));
    }
    format!("<m:oMathPara xmlns:m=\"{NS_M}\" xmlns:a=\"{NS_A}\"><m:oMath>{out}</m:oMath></m:oMathPara>")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn run(out: &mut String, text: &str, upright: bool) {
    out.push_str("<m:r>");
    if upright {
        out.push_str("<m:rPr><m:sty m:val=\"p\"/></m:rPr>");
    }
    out.push_str(&format!("<m:t>{}</m:t></m:r>", esc(text)));
}

fn part(out: &mut String, tag: &str, e: &Expr) {
    let mut inner = String::new();
    omml(e, &mut inner);
    if inner.is_empty() {
        run(&mut inner, "", false);
    }
    out.push_str(&format!("<m:{tag}>{inner}</m:{tag}>"));
}

fn omml(e: &Expr, out: &mut String) {
    match e {
        Expr::Row(items) => {
            let mut pending = String::new();
            for it in items {
                match it {
                    Expr::Var(c) => pending.push(*c),
                    Expr::Text(t) => pending.push_str(t),
                    other => {
                        if !pending.is_empty() {
                            run(out, &std::mem::take(&mut pending), false);
                        }
                        omml(other, out);
                    }
                }
            }
            if !pending.is_empty() {
                run(out, &pending, false);
            }
        }
        Expr::Var(c) => run(out, &c.to_string(), false),
        Expr::Text(t) => run(out, t, false),
        Expr::Func(f) => run(out, f, true),
        Expr::Empty => {}
        Expr::Frac(n, d) => {
            out.push_str("<m:f>");
            part(out, "num", n);
            part(out, "den", d);
            out.push_str("</m:f>");
        }
        Expr::Script { base, sub, sup } => {
            let tag = match (sub.is_some(), sup.is_some()) {
                (true, true) => "sSubSup",
                (true, false) => "sSub",
                _ => "sSup",
            };
            out.push_str(&format!("<m:{tag}>"));
            part(out, "e", base);
            if let Some(s) = sub {
                part(out, "sub", s);
            }
            if let Some(s) = sup {
                part(out, "sup", s);
            }
            out.push_str(&format!("</m:{tag}>"));
        }
        Expr::Sqrt { degree, body } => {
            out.push_str("<m:rad>");
            match degree {
                Some(d) => {
                    out.push_str("<m:radPr/>");
                    part(out, "deg", d);
                }
                None => out.push_str("<m:radPr><m:degHide m:val=\"1\"/></m:radPr><m:deg/>"),
            }
            part(out, "e", body);
            out.push_str("</m:rad>");
        }
        Expr::Nary { op, sub, sup, body } => {
            let loc = if matches!(op, '∑' | '∏') { "undOvr" } else { "subSup" };
            out.push_str(&format!("<m:nary><m:naryPr><m:chr m:val=\"{op}\"/><m:limLoc m:val=\"{loc}\"/>"));
            if sub.is_none() {
                out.push_str("<m:subHide m:val=\"1\"/>");
            }
            if sup.is_none() {
                out.push_str("<m:supHide m:val=\"1\"/>");
            }
            out.push_str("</m:naryPr>");
            match sub {
                Some(s) => part(out, "sub", s),
                None => out.push_str("<m:sub/>"),
            }
            match sup {
                Some(s) => part(out, "sup", s),
                None => out.push_str("<m:sup/>"),
            }
            part(out, "e", body);
            out.push_str("</m:nary>");
        }
        Expr::Delim { open, close, body } => {
            out.push_str(&format!(
                "<m:d><m:dPr><m:begChr m:val=\"{}\"/><m:endChr m:val=\"{}\"/></m:dPr>",
                esc(&open.to_string()),
                esc(&close.to_string())
            ));
            part(out, "e", body);
            out.push_str("</m:d>");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The plain stand-in

/// The equation as plain text: italic variables, `x²`, `√`, `a/b`.
pub fn to_plain(e: &Expr) -> String {
    let mut s = String::new();
    plain(e, &mut s);
    s
}

/// Mathematical italic, as PowerPoint writes variables in its own plain text.
pub fn math_italic(c: char) -> char {
    match c {
        'h' => 'ℎ',
        'a'..='z' => char::from_u32(0x1D44E + (c as u32 - 'a' as u32)).unwrap_or(c),
        'A'..='Z' => char::from_u32(0x1D434 + (c as u32 - 'A' as u32)).unwrap_or(c),
        _ => c,
    }
}

fn plain_ascii(c: char) -> char {
    match c {
        'ℎ' => 'h',
        '\u{1D44E}'..='\u{1D467}' => char::from_u32('a' as u32 + (c as u32 - 0x1D44E)).unwrap_or(c),
        '\u{1D434}'..='\u{1D44D}' => char::from_u32('A' as u32 + (c as u32 - 0x1D434)).unwrap_or(c),
        _ => c,
    }
}

fn sup_char(c: char) -> Option<char> {
    Some(match plain_ascii(c) {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        'n' => 'ⁿ',
        'i' => 'ⁱ',
        '+' => '⁺',
        '−' | '-' => '⁻',
        '=' => '⁼',
        '(' => '⁽',
        ')' => '⁾',
        _ => return None,
    })
}

fn sub_char(c: char) -> Option<char> {
    Some(match plain_ascii(c) {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        '+' => '₊',
        '−' | '-' => '₋',
        '=' => '₌',
        '(' => '₍',
        ')' => '₎',
        'a' => 'ₐ',
        'e' => 'ₑ',
        'o' => 'ₒ',
        'x' => 'ₓ',
        'h' => 'ₕ',
        'k' => 'ₖ',
        'l' => 'ₗ',
        'm' => 'ₘ',
        'n' => 'ₙ',
        'p' => 'ₚ',
        's' => 'ₛ',
        't' => 'ₜ',
        'i' => 'ᵢ',
        'j' => 'ⱼ',
        'r' => 'ᵣ',
        'u' => 'ᵤ',
        'v' => 'ᵥ',
        _ => return None,
    })
}

fn raised(text: &str, up: bool) -> String {
    let f = if up { sup_char } else { sub_char };
    let mapped: Option<String> = text.chars().map(f).collect();
    match mapped {
        Some(m) if !text.is_empty() => m,
        _ => format!("{}({text})", if up { '^' } else { '_' }),
    }
}

fn is_relation(c: char) -> bool {
    "=<>≤≥≠≈≡→⇒⇔∈⊂".contains(c)
}

fn is_binary(c: char) -> bool {
    "+−±∓×÷·".contains(c)
}

/// Brackets only where a bare run of more than one symbol would read wrongly.
fn bracket_if_long(s: String) -> String {
    if s.chars().count() <= 1 || s.chars().all(|c| c.is_alphanumeric() || c == '.') { s } else { format!("({s})") }
}

fn plain(e: &Expr, out: &mut String) {
    match e {
        Expr::Row(items) => {
            for (k, it) in items.iter().enumerate() {
                if let Expr::Text(t) = it
                    && let Some(c) = t.chars().next().filter(|_| t.chars().count() == 1)
                {
                    let before = k.checked_sub(1).is_some_and(|j| !matches!(items[j], Expr::Text(ref p) if p.chars().count() == 1 && (is_relation(p.chars().next().unwrap_or(' ')) || is_binary(p.chars().next().unwrap_or(' ')))));
                    let after = k + 1 < items.len();
                    if is_relation(c) || (is_binary(c) && before && after) {
                        if !out.ends_with(' ') && !out.is_empty() {
                            out.push(' ');
                        }
                        out.push(c);
                        if after {
                            out.push(' ');
                        }
                        continue;
                    }
                }
                plain(it, out);
            }
        }
        Expr::Var(c) => out.push(math_italic(*c)),
        Expr::Text(t) => out.push_str(t),
        Expr::Func(f) => out.push_str(f),
        Expr::Empty => {}
        Expr::Frac(n, d) => {
            out.push_str(&bracket_if_long(to_plain(n)));
            out.push('/');
            out.push_str(&bracket_if_long(to_plain(d)));
        }
        Expr::Script { base, sub, sup } => {
            plain(base, out);
            if let Some(s) = sub {
                out.push_str(&raised(&to_plain(s), false));
            }
            if let Some(s) = sup {
                out.push_str(&raised(&to_plain(s), true));
            }
        }
        Expr::Sqrt { degree, body } => {
            if let Some(d) = degree {
                let d = to_plain(d);
                match d.chars().map(sup_char).collect::<Option<String>>() {
                    Some(m) if !d.is_empty() => out.push_str(&m),
                    _ => out.push_str(&format!("[{d}]")),
                }
            }
            out.push('√');
            let b = to_plain(body);
            if b.chars().count() <= 1 || b.chars().all(|c| c.is_alphanumeric()) && b.chars().count() <= 2 {
                out.push_str(&b);
            } else {
                out.push_str(&format!("({b})"));
            }
        }
        Expr::Nary { op, sub, sup, body } => {
            out.push(*op);
            if let Some(s) = sub {
                out.push_str(&raised(&to_plain(s), false));
            }
            if let Some(s) = sup {
                out.push_str(&raised(&to_plain(s), true));
            }
            out.push(' ');
            plain(body, out);
        }
        Expr::Delim { open, close, body } => {
            out.push(*open);
            plain(body, out);
            out.push(*close);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Reading an equation back

struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    kids: Vec<Node>,
    text: String,
}

impl Node {
    fn val(&self) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == "val").map(|(_, v)| v.as_str())
    }
    fn kid(&self, name: &str) -> Option<&Node> {
        self.kids.iter().find(|k| k.name == name)
    }
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

fn local(n: &str) -> &str {
    n.rsplit(':').next().unwrap_or(n)
}

/// Just enough XML for an equation: elements, attributes and text.
fn read_xml(src: &str) -> Option<Node> {
    let mut stack: Vec<Node> = vec![Node { name: String::new(), attrs: vec![], kids: vec![], text: String::new() }];
    let mut rest = src;
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            stack.last_mut()?.text.push_str(&unescape(rest));
            break;
        };
        if lt > 0 {
            stack.last_mut()?.text.push_str(&unescape(&rest[..lt]));
        }
        rest = &rest[lt..];
        let gt = rest.find('>')?;
        let tag = &rest[1..gt];
        rest = &rest[gt + 1..];
        if tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        if let Some(close) = tag.strip_prefix('/') {
            let done = stack.pop()?;
            if done.name != local(close.trim()) || stack.is_empty() {
                return None;
            }
            stack.last_mut()?.kids.push(done);
            continue;
        }
        let selfclose = tag.ends_with('/');
        let tag = tag.trim_end_matches('/');
        let (name, mut attrs_src) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
        let mut attrs = vec![];
        while let Some((k, after)) = attrs_src.split_once('=') {
            let after = after.trim_start();
            let q = after.chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let end = after[1..].find(q)?;
            attrs.push((local(k.trim()).to_string(), unescape(&after[1..1 + end])));
            attrs_src = &after[end + 2..];
        }
        let node = Node { name: local(name).to_string(), attrs, kids: vec![], text: String::new() };
        if selfclose {
            stack.last_mut()?.kids.push(node);
        } else {
            stack.push(node);
        }
        if stack.len() > 200 {
            return None;
        }
    }
    if stack.len() == 1 { stack.pop()?.kids.into_iter().next() } else { None }
}

/// The equation inside PowerPoint's OMML, as far as it is made of things the editor knows.
/// Anything else is kept as its text so nothing typed is lost.
pub fn from_omml(omml: &str) -> Option<Expr> {
    let root = read_xml(omml)?;
    Some(from_node(&root, 0))
}

fn arg(n: &Node, name: &str, depth: usize) -> Option<Box<Expr>> {
    let k = n.kid(name)?;
    let e = from_node(k, depth + 1);
    if matches!(&e, Expr::Row(r) if r.is_empty()) { None } else { Some(Box::new(e)) }
}

fn must(n: &Node, name: &str, depth: usize) -> Box<Expr> {
    arg(n, name, depth).unwrap_or_else(|| Box::new(Expr::Empty))
}

fn chars_to_expr(t: &str, upright: bool, out: &mut Vec<Expr>) {
    if upright && t.chars().count() > 1 && t.chars().all(|c| c.is_ascii_alphabetic()) {
        out.push(Expr::Func(t.to_string()));
        return;
    }
    let mut digits = String::new();
    let flush = |digits: &mut String, out: &mut Vec<Expr>| {
        if !digits.is_empty() {
            out.push(Expr::Text(std::mem::take(digits)));
        }
    };
    for c in t.chars() {
        if c.is_ascii_digit() || c == '.' {
            digits.push(c);
            continue;
        }
        flush(&mut digits, out);
        let c = plain_ascii(c);
        if c.is_alphabetic() {
            out.push(Expr::Var(c));
        } else if !c.is_whitespace() {
            out.push(Expr::Text(if c == '-' { "−".into() } else { c.to_string() }));
        }
    }
    flush(&mut digits, out);
}

fn from_node(n: &Node, depth: usize) -> Expr {
    if depth > 60 {
        return Expr::Empty;
    }
    let group = |k: &[Node], out: &mut Vec<Expr>| {
        for c in k {
            out.push(from_node(c, depth + 1));
        }
    };
    match n.name.as_str() {
        "r" => {
            let upright = n.kid("rPr").and_then(|p| p.kid("sty")).and_then(Node::val) == Some("p");
            let mut out = vec![];
            for t in n.kids.iter().filter(|k| k.name == "t") {
                chars_to_expr(&t.text, upright, &mut out);
            }
            one(out)
        }
        "f" => Expr::Frac(must(n, "num", depth), must(n, "den", depth)),
        "sSup" => Expr::Script { base: must(n, "e", depth), sub: None, sup: Some(must(n, "sup", depth)) },
        "sSub" => Expr::Script { base: must(n, "e", depth), sub: Some(must(n, "sub", depth)), sup: None },
        "sSubSup" => Expr::Script { base: must(n, "e", depth), sub: Some(must(n, "sub", depth)), sup: Some(must(n, "sup", depth)) },
        "rad" => {
            let hidden = n.kid("radPr").and_then(|p| p.kid("degHide")).and_then(Node::val).is_some_and(|v| v == "1" || v == "on" || v == "true");
            Expr::Sqrt { degree: if hidden { None } else { arg(n, "deg", depth) }, body: must(n, "e", depth) }
        }
        "nary" => {
            let pr = n.kid("naryPr");
            let op = pr.and_then(|p| p.kid("chr")).and_then(Node::val).and_then(|v| v.chars().next()).unwrap_or('∫');
            let hide = |name: &str| pr.and_then(|p| p.kid(name)).and_then(Node::val).is_some_and(|v| v == "1" || v == "on" || v == "true");
            Expr::Nary {
                op,
                sub: if hide("subHide") { None } else { arg(n, "sub", depth) },
                sup: if hide("supHide") { None } else { arg(n, "sup", depth) },
                body: must(n, "e", depth),
            }
        }
        "d" => {
            let pr = n.kid("dPr");
            let chr = |name: &str, default: char| match pr.and_then(|p| p.kid(name)).and_then(Node::val) {
                Some(v) => v.chars().next().unwrap_or(' '),
                None => default,
            };
            let parts: Vec<Expr> = n.kids.iter().filter(|k| k.name == "e").map(|k| from_node(k, depth + 1)).collect();
            let mut body = vec![];
            for (k, e) in parts.into_iter().enumerate() {
                if k > 0 {
                    body.push(Expr::Text(",".into()));
                }
                body.push(e);
            }
            Expr::Delim { open: chr("begChr", '('), close: chr("endChr", ')'), body: Box::new(one(body)) }
        }
        // Property elements say how to draw; they hold nothing to show.
        "rPr" | "ctrlPr" | "radPr" | "naryPr" | "dPr" | "fPr" | "sSupPr" | "sSubPr" | "sSubSupPr" | "accPr" | "barPr" | "funcPr" => Expr::Row(vec![]),
        _ => {
            let mut out = vec![];
            group(&n.kids, &mut out);
            let out: Vec<Expr> = out.into_iter().filter(|e| !matches!(e, Expr::Row(r) if r.is_empty())).collect();
            // Lines of a display group stay on their own lines as best a one-line field can.
            one(out)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Back to typing

/// The equation as the text that makes it again: `parse(&to_linear(e))` draws the same equation.
pub fn to_linear(e: &Expr) -> String {
    let mut s = String::new();
    linear(e, &mut s);
    s
}

fn is_simple(e: &Expr) -> bool {
    match e {
        Expr::Var(_) | Expr::Func(_) | Expr::Empty | Expr::Delim { .. } | Expr::Sqrt { .. } => true,
        Expr::Text(t) => t.chars().all(|c| c.is_ascii_digit() || c == '.') && !t.is_empty(),
        _ => false,
    }
}

fn group_arg(e: &Expr, out: &mut String) {
    out.push('(');
    linear(e, out);
    out.push(')');
}

fn script_arg_text(e: &Expr, out: &mut String) {
    match e {
        Expr::Var(_) | Expr::Empty => linear(e, out),
        Expr::Text(t) if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) => out.push_str(t),
        _ => group_arg(e, out),
    }
}

fn keyword_trouble(run: &str) -> bool {
    NAMES.iter().any(|(n, _)| *n == run)
        || FUNCS.iter().any(|f| run.starts_with(f))
        || matches!(run, "sqrt" | "cbrt" | "sum" | "prod" | "int" | "integral")
        || run.len() > 1 && (run.contains("sqrt") || run.contains("sum"))
}

fn linear(e: &Expr, out: &mut String) {
    match e {
        Expr::Row(items) => {
            let mut k = 0;
            while k < items.len() {
                if let Expr::Var(_) = items[k] {
                    let mut run = String::new();
                    let start = k;
                    while let Some(Expr::Var(c)) = items.get(k) {
                        run.push(*c);
                        k += 1;
                    }
                    let ascii: String = run.chars().filter(char::is_ascii_alphabetic).collect();
                    if ascii.len() == run.chars().count() && keyword_trouble(&ascii) {
                        for (n, c) in run.chars().enumerate() {
                            if n > 0 {
                                out.push(' ');
                            }
                            out.push(c);
                        }
                    } else {
                        out.push_str(&run);
                    }
                    let _ = start;
                } else {
                    let before = out.chars().last();
                    let mut piece = String::new();
                    linear(&items[k], &mut piece);
                    // Keep a function name from running into the letter after it.
                    if matches!(items[k], Expr::Func(_)) && items.get(k + 1).is_some_and(|n| matches!(n, Expr::Var(_))) {
                        piece.push(' ');
                    }
                    if before.is_some_and(|b| b.is_ascii_alphabetic()) && piece.starts_with(|c: char| c.is_ascii_alphabetic()) {
                        out.push(' ');
                    }
                    out.push_str(&piece);
                    k += 1;
                }
            }
        }
        Expr::Var(c) => out.push(*c),
        Expr::Text(t) => out.push_str(t),
        Expr::Func(f) => out.push_str(f),
        Expr::Empty => out.push(PLACEHOLDER),
        Expr::Frac(n, d) => {
            group_arg(n, out);
            out.push('/');
            group_arg(d, out);
        }
        Expr::Script { base, sub, sup } => {
            if is_simple(base) {
                linear(base, out);
            } else {
                group_arg(base, out);
            }
            if let Some(s) = sub {
                out.push('_');
                script_arg_text(s, out);
            }
            if let Some(s) = sup {
                out.push('^');
                script_arg_text(s, out);
            }
        }
        Expr::Sqrt { degree, body } => {
            out.push_str("sqrt");
            if let Some(d) = degree {
                out.push('[');
                linear(d, out);
                out.push(']');
            }
            group_arg(body, out);
        }
        Expr::Nary { op, sub, sup, body } => {
            out.push(*op);
            if let Some(s) = sub {
                out.push('_');
                group_arg(s, out);
            }
            if let Some(s) = sup {
                out.push('^');
                group_arg(s, out);
            }
            out.push(' ');
            if matches!(**body, Expr::Row(_) | Expr::Frac(..)) { group_arg(body, out) } else { linear(body, out) }
        }
        Expr::Delim { open, close, body } => {
            out.push(*open);
            linear(body, out);
            out.push(*close);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_of(s: &str) -> String {
        to_plain(&parse(s))
    }

    #[test]
    fn a_fraction_takes_the_term_before_and_after_the_slash() {
        let Expr::Frac(n, d) = parse("(a+b)/c") else { panic!("not a fraction") };
        assert_eq!(to_plain(&n), "𝑎 + 𝑏");
        assert_eq!(to_plain(&d), "𝑐");
        assert!(matches!(parse("3x/4"), Expr::Frac(n, _) if matches!(*n, Expr::Row(ref r) if r.len() == 2)));
        assert!(matches!(parse("x^2/3"), Expr::Frac(n, _) if matches!(*n, Expr::Script { .. })));
    }

    #[test]
    fn powers_and_subscripts_become_the_matching_script() {
        assert_eq!(plain_of("x^2"), "𝑥²");
        assert_eq!(plain_of("x_1"), "𝑥₁");
        assert_eq!(plain_of("x^(n+1)"), "𝑥^(𝑛 + 1)");
        assert_eq!(plain_of("2^-3"), "2⁻³");
        assert!(matches!(parse("a_1^2"), Expr::Script { sub: Some(_), sup: Some(_), .. }));
    }

    #[test]
    fn the_quadratic_formula_types_as_it_reads() {
        let e = parse("x=(-b+-sqrt(b^2-4ac))/2a");
        let omml = to_omml(&e);
        assert!(omml.contains("<m:f>") && omml.contains("<m:rad>") && omml.contains("<m:sSup>"), "{omml}");
        assert!(omml.contains("±"));
        assert_eq!(plain_of("x=(-b+-sqrt(b^2-4ac))/2a"), "𝑥 = (−𝑏 ± √(𝑏² − 4𝑎𝑐))/2𝑎");
    }

    #[test]
    fn names_and_symbols_are_turned_into_the_characters() {
        assert_eq!(plain_of("pi r^2"), "π𝑟²");
        assert_eq!(plain_of("a<=b"), "𝑎 ≤ 𝑏");
        assert_eq!(plain_of("a!=b"), "𝑎 ≠ 𝑏");
        assert_eq!(plain_of("3*4"), "3 · 4");
        assert_eq!(plain_of("sinx"), "sin𝑥");
        assert_eq!(plain_of("\\theta"), "θ");
    }

    #[test]
    fn sums_and_integrals_keep_their_limits_and_body() {
        let e = parse("sum_(i=1)^n i");
        let Expr::Nary { op: '∑', sub: Some(_), sup: Some(_), .. } = e else { panic!("{e:?}") };
        assert!(to_omml(&e).contains("<m:nary>"));
        assert!(matches!(parse("int_0^1 x"), Expr::Nary { op: '∫', .. }));
    }

    #[test]
    fn boxes_left_empty_are_reported() {
        assert!(has_empty(&parse("(□)/(□)")));
        assert!(has_empty(&parse("x^□")));
        assert!(!has_empty(&parse("(1)/(2)")));
    }

    #[test]
    fn roots_take_a_degree() {
        assert!(matches!(parse("sqrt[3](x)"), Expr::Sqrt { degree: Some(_), .. }));
        assert_eq!(plain_of("sqrt[3](x+1)"), "³√(𝑥 + 1)");
        assert!(to_omml(&parse("sqrt(x)")).contains("degHide"));
    }

    #[test]
    fn nonsense_never_panics() {
        for s in [
            "",
            "((((",
            "))))",
            "^",
            "_",
            "/",
            "//",
            "sqrt",
            "sum",
            "x^",
            "a/",
            "/a",
            "[0,1)",
            "|x",
            "\\",
            "□□□",
            "<",
            "&<>",
            "((((((((((((((((((((((((((((((((((((((((((((((((((((((((x",
        ] {
            let e = parse(s);
            let _ = to_omml(&e);
            let _ = to_plain(&e);
        }
        assert!(to_omml(&parse("a&b<c")).contains("a&amp;b&lt;c"));
    }

    #[test]
    fn the_equation_is_one_display_paragraph_with_the_math_namespace() {
        let omml = to_omml(&parse("x=1"));
        assert!(omml.starts_with("<m:oMathPara xmlns:m=") && omml.ends_with("</m:oMathPara>"));
    }

    #[test]
    fn what_we_make_reads_back_and_types_the_same() {
        for typed in [
            "x=(-b+-sqrt(b^2-4ac))/(2a)",
            "a^2+b^2=c^2",
            "sum_(i=1)^n i^2",
            "int_0^1 x dx",
            "sqrt[3](x+1)",
            "sin(x)/cos(x)",
            "x_1^2",
            "(1)/(2)",
            "3 pi r^2",
            "|x-1|<=4",
        ] {
            let made = parse(typed);
            let back = from_omml(&to_omml(&made)).unwrap_or_else(|| panic!("{typed}"));
            assert_eq!(to_omml(&back), to_omml(&made), "{typed} read back differently");
            let again = parse(&to_linear(&back));
            assert_eq!(to_omml(&again), to_omml(&made), "{typed} typed as {}", to_linear(&back));
        }
    }

    #[test]
    fn an_equation_from_powerpoint_reads_into_the_editor() {
        let omml = r#"<m:oMathPara xmlns:m="x"><m:oMath><m:r><m:rPr><m:sty m:val="p"/></m:rPr><m:t>sin</m:t></m:r><m:d><m:dPr><m:ctrlPr/></m:dPr><m:e><m:r><m:t>θ</m:t></m:r></m:e></m:d><m:r><m:t>=</m:t></m:r><m:f><m:fPr><m:ctrlPr/></m:fPr><m:num><m:r><m:t>1</m:t></m:r></m:num><m:den><m:r><m:t>2</m:t></m:r></m:den></m:f></m:oMath></m:oMathPara>"#;
        let e = from_omml(omml).expect("reads");
        assert_eq!(to_linear(&e), "sin(θ)=(1)/(2)");
        assert!(from_omml("<m:oMath><m:r>").is_none(), "broken XML is refused, not guessed at");
    }
}
