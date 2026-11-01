//! Office Math in a DrawingML paragraph: the plain text that stands in for an equation, and the
//! formatting it keeps.

use super::*;

/// The plain text that stands in for an equation. Each equation of a display group (`m:oMathPara`
/// holding several `m:oMath`) is its own line, and relations and binary operators get the space
/// PowerPoint puts round them, which the bare `m:t` strings leave out.
pub(super) fn math_plain_text(math: &El) -> String {
    fn rec(e: &El, out: &mut String, depth: usize) {
        if depth > 64 {
            return;
        }
        if e.is("d") && e.prefix() == "m" {
            // Brackets round what they hold: `(` and `)` unless the equation names others.
            let chr = |name: &str, default: &str| {
                e.child("dPr").and_then(|p| p.child(name)).and_then(|c| c.attr("val")).map(str::to_string).unwrap_or_else(|| default.to_string())
            };
            out.push_str(&chr("begChr", "("));
            for (i, c) in e.elements().filter(|c| c.is("e")).enumerate() {
                if i > 0 {
                    out.push_str(&chr("sepChr", "|"));
                }
                rec(c, out, depth + 1);
            }
            out.push_str(&chr("endChr", ")"));
            return;
        }
        if e.is("t") {
            out.push_str(&e.text());
            return;
        }
        let mut lines = 0;
        for c in e.elements() {
            if e.is("oMathPara") && c.is("oMath") {
                if lines > 0 {
                    out.push('\u{b}');
                }
                lines += 1;
            }
            rec(c, out, depth + 1);
        }
    }
    let mut raw = String::new();
    rec(math, &mut raw, 0);
    space_operators(&raw)
}

fn space_operators(s: &str) -> String {
    const RELATIONS: &str = "=<>≤≥≠≈≡";
    const BINARY: &str = "+−-±×÷·";
    let operand = |c: char| c.is_alphanumeric() || matches!(c, ')' | ']' | '}' | '.' | '%');
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let before = i.checked_sub(1).and_then(|j| chars.get(j)).copied();
        let after = chars.get(i + 1).copied();
        let spaced = RELATIONS.contains(c) || (BINARY.contains(c) && before.is_some_and(operand) && after.is_some_and(|a| a != ' '));
        if spaced {
            if before.is_some_and(|b| b != ' ' && b != '\u{b}') {
                out.push(' ');
            }
            out.push(c);
            if after.is_some_and(|a| a != ' ' && a != '\u{b}') {
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Formatting for the plain-text stand-in of an equation: the DrawingML run properties
/// (`a:rPr`, not the math `m:rPr`) of the equation's runs. The first one that sets a size wins, as
/// the first run is often a bare variable; otherwise the first one. Italic is dropped: the math
/// italic letters are already italic characters.
pub(super) fn math_text_props(imp: &mut Imp, part: &Part, math: &El) -> RunProps {
    let mut found = vec![];
    math.find_all("rPr", &mut found);
    found.retain(|r| r.prefix() != "m");
    let Some(r) = found.iter().find(|r| r.has_attr("sz")).or(found.first()) else {
        return RunProps::default();
    };
    let mut props = rpr(imp, part, r);
    props.italic = None;
    props
}
