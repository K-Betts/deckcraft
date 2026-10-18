//! Turns the LaTeX used in the question bank into the linear equation syntax the engine's
//! equation parser reads (`\frac{a}{b}` becomes `(a)/(b)`), and splits a line of text into plain
//! and maths pieces at `$…$`. Only the small part of LaTeX a school question needs is handled;
//! anything else passes through and is drawn as typed.

/// One piece of a line: ordinary text, or maths in linear syntax.
#[derive(Clone, Debug, PartialEq)]
pub enum Segment {
    Text(String),
    Math(String),
}

/// Splits `text` at `$…$`. `\text{…}` inside maths (units, words) comes out as plain text, with a
/// following `^2` or `^3` as a raised digit.
pub fn segments(text: &str) -> Vec<Segment> {
    let mut out: Vec<Segment> = vec![];
    for (i, part) in text.split('$').enumerate() {
        if part.is_empty() {
            continue;
        }
        if i % 2 == 0 {
            push_text(&mut out, part);
        } else {
            math_pieces(part, &mut out);
        }
    }
    out
}

/// The line as plain text: maths without its `$` signs, drawn as the equation's text stand-in.
pub fn plain(text: &str) -> String {
    segments(text)
        .into_iter()
        .map(|seg| match seg {
            Segment::Text(t) => t,
            Segment::Math(m) => crate::equation::to_plain(&crate::equation::parse(&m)),
        })
        .collect()
}

fn push_text(out: &mut Vec<Segment>, t: &str) {
    if let Some(Segment::Text(last)) = out.last_mut() {
        last.push_str(t);
    } else {
        out.push(Segment::Text(t.to_string()));
    }
}

fn math_pieces(src: &str, out: &mut Vec<Segment>) {
    let mut rest = src;
    let mut math = String::new();
    let flush = |math: &mut String, out: &mut Vec<Segment>| {
        if !math.trim().is_empty() {
            out.push(Segment::Math(to_linear(math)));
        }
        math.clear();
    };
    while let Some(at) = rest.find("\\text{") {
        math.push_str(&rest[..at]);
        let after = &rest[at + 6..];
        let Some(end) = after.find('}') else {
            break;
        };
        flush(&mut math, out);
        let mut word = after[..end].to_string();
        rest = &after[end + 1..];
        for (sup, ch) in [("^2", '²'), ("^3", '³')] {
            if let Some(r) = rest.strip_prefix(sup) {
                word.push(ch);
                rest = r;
                break;
            }
        }
        push_text(out, &word);
    }
    math.push_str(rest);
    flush(&mut math, out);
}

/// The group in braces starting at `chars[*i]` (which must be `{`), without the braces.
fn group(chars: &[char], i: &mut usize) -> String {
    let mut depth = 0;
    let mut s = String::new();
    while *i < chars.len() {
        let c = chars[*i];
        *i += 1;
        match c {
            '{' => {
                depth += 1;
                if depth > 1 {
                    s.push(c);
                }
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                s.push(c);
            }
            _ => s.push(c),
        }
    }
    s
}

/// LaTeX to the equation parser's linear syntax.
pub fn to_linear(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                if word.is_empty() {
                    // `\\`, `\,` and friends: a space.
                    i += 1;
                    out.push(' ');
                    continue;
                }
                while i < chars.len() && chars[i] == ' ' && word != "text" {
                    i += 1;
                }
                match word.as_str() {
                    "frac" => {
                        let a = group_at(&chars, &mut i);
                        let b = group_at(&chars, &mut i);
                        out.push_str(&format!("({})/({})", to_linear(&a), to_linear(&b)));
                    }
                    // Bold and vector marks are dropped: the letter stays.
                    "mathbf" | "boldsymbol" => {
                        let a = group_at(&chars, &mut i);
                        out.push_str(&to_linear(&a));
                    }
                    "overrightarrow" | "vec" => {
                        let a = group_at(&chars, &mut i);
                        out.push_str(&to_linear(&a));
                        out.push('\u{20D7}');
                    }
                    "binom" => {
                        let a = group_at(&chars, &mut i);
                        let b = group_at(&chars, &mut i);
                        out.push_str(&format!("vec({},{})", to_linear(&a), to_linear(&b)));
                    }
                    "sqrt" => {
                        let mut deg = String::new();
                        if chars.get(i) == Some(&'[') {
                            i += 1;
                            while i < chars.len() && chars[i] != ']' {
                                deg.push(chars[i]);
                                i += 1;
                            }
                            i += 1;
                        }
                        let a = group_at(&chars, &mut i);
                        if deg.is_empty() {
                            out.push_str(&format!("sqrt({})", to_linear(&a)));
                        } else {
                            out.push_str(&format!("sqrt[{deg}]({})", to_linear(&a)));
                        }
                    }
                    "text" | "mathrm" => out.push_str(&group_at(&chars, &mut i)),
                    "times" => out.push('×'),
                    "div" => out.push('÷'),
                    "cdot" => out.push('·'),
                    "pm" => out.push('±'),
                    "le" | "leq" => out.push('≤'),
                    "ge" | "geq" => out.push('≥'),
                    "neq" | "ne" => out.push('≠'),
                    "approx" => out.push('≈'),
                    "circ" => out.push('°'),
                    "ldots" | "dots" | "cdots" => out.push('…'),
                    "Rightarrow" | "implies" => out.push('⇒'),
                    "left" | "right" => {}
                    "cap" | "cup" | "pi" | "theta" | "alpha" | "beta" | "sigma" | "mu" | "angle" => {
                        out.push('\\');
                        out.push_str(&word);
                        out.push(' ');
                    }
                    _ => out.push_str(&word),
                }
            }
            '^' | '_' => {
                out.push(c);
                i += 1;
                if chars.get(i) == Some(&'{') {
                    let g = group(&chars, &mut i);
                    out.push_str(&format!("({})", to_linear(&g)));
                }
            }
            '{' => {
                let g = group(&chars, &mut i);
                out.push_str(&format!("({})", to_linear(&g)));
            }
            '}' => i += 1,
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// The `{…}` argument at `chars[*i]`; a bare character is its own argument.
fn group_at(chars: &[char], i: &mut usize) -> String {
    while chars.get(*i) == Some(&' ') {
        *i += 1;
    }
    if chars.get(*i) == Some(&'{') {
        group(chars, i)
    } else if let Some(c) = chars.get(*i) {
        *i += 1;
        c.to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn fractions_roots_and_powers_become_linear_syntax() {
        assert_eq!(to_linear(r"\frac{3}{4} + \frac{1}{8}"), "(3)/(4) + (1)/(8)");
        assert_eq!(to_linear(r"\binom{3}{-2}"), "vec(3,-2)");
        assert_eq!(to_linear(r"\sqrt{50}"), "sqrt(50)");
        assert_eq!(to_linear(r"x^{2} + 3"), "x^(2) + 3");
        assert_eq!(to_linear(r"3 \times 4"), "3 ×4");
    }

    #[test]
    fn dollars_split_text_from_maths_and_units_become_text() {
        let s = segments(r"Find $x$ when $80\text{ cm}^3$ is used");
        assert_eq!(
            s,
            vec![
                Segment::Text("Find ".into()),
                Segment::Math("x".into()),
                Segment::Text(" when ".into()),
                Segment::Math("80".into()),
                Segment::Text(" cm³ is used".into()),
            ]
        );
    }

    #[test]
    fn every_bank_equation_parses_without_empty_boxes() {
        let bank = crate::questions::starter().unwrap();
        for q in &bank.questions {
            for text in [&q.text, &q.answer] {
                for seg in segments(text) {
                    if let Segment::Math(m) = seg {
                        let e = crate::equation::parse(&m);
                        assert!(!crate::equation::has_empty(&e), "{}: {m}", q.id);
                    }
                }
            }
        }
    }
}
