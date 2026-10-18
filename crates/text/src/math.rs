//! Equations laid out as maths: fractions stacked over a bar, powers raised, roots under a bar,
//! sums with their limits, brackets grown to the height of what they hold.
//!
//! The layout is a box of glyphs and rules in points, measured from the left end of the baseline
//! (y grows downward). Everything is set in one face, normally STIX Two Math (standing in for
//! Cambria Math), whose math-italic letters are the variables.

use std::sync::Arc;

use deckcraft_fonts::{FontDb, FontFace};

use crate::equation::{Expr, math_italic};

/// A glyph placed in the equation.
#[derive(Clone, Debug, PartialEq)]
pub struct MathGlyph {
    pub gid: u32,
    /// Pen position of the glyph's origin on the baseline.
    pub x: f64,
    pub y: f64,
    pub size: f64,
    /// The face has no italic form of this letter: slant it.
    pub slant: bool,
}

/// A bar: a fraction line, a root's overline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MathRule {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MathBox {
    pub w: f64,
    /// Height above the baseline and depth below it.
    pub asc: f64,
    pub desc: f64,
    pub glyphs: Vec<MathGlyph>,
    pub rules: Vec<MathRule>,
}

/// Height of the line a fraction bar sits on, above the baseline (em).
const AXIS: f64 = 0.25;
/// Thickness of bars (em).
const RULE: f64 = 0.05;
/// Height of capitals and digits, and the depth of descenders (em).
const TEXT_ASC: f64 = 0.72;
const TEXT_DESC: f64 = 0.2;
/// Powers and subscripts, and then the next level down.
const SCRIPT: f64 = 0.7;

impl MathBox {
    fn put(&mut self, other: MathBox, dx: f64, dy: f64) {
        self.asc = self.asc.max(other.asc - dy);
        self.desc = self.desc.max(other.desc + dy);
        self.glyphs.extend(other.glyphs.into_iter().map(|g| MathGlyph { x: g.x + dx, y: g.y + dy, ..g }));
        self.rules.extend(other.rules.into_iter().map(|r| MathRule { x0: r.x0 + dx, y0: r.y0 + dy, x1: r.x1 + dx, y1: r.y1 + dy }));
    }
}

/// Whether the equation needs more than a line of text to be drawn right.
pub fn is_complex(e: &Expr) -> bool {
    match e {
        Expr::Row(r) => r.iter().any(is_complex),
        Expr::Frac(..) | Expr::Stack(..) | Expr::Script { .. } | Expr::Sqrt { .. } | Expr::Nary { .. } => true,
        Expr::Delim { body, .. } => is_complex(body),
        Expr::Var(_) | Expr::Text(_) | Expr::Func(_) | Expr::Empty => false,
    }
}

struct Ctx<'a> {
    face: &'a FontFace,
}

/// Lay `e` out at `size` points in `face`.
pub fn layout(e: &Expr, face: &Arc<FontFace>, size: f64) -> MathBox {
    lay(&Ctx { face }, e, size, 0)
}

impl Ctx<'_> {
    fn adv(&self, gid: u32, size: f64) -> f64 {
        self.face.advance(gid) * size / self.face.upem.max(1.0)
    }

    /// One character. `None` when the face does not have it.
    fn glyph(&self, c: char) -> Option<u32> {
        Some(self.face.glyph_for(c)).filter(|g| *g != 0)
    }

    fn text(&self, s: &str, size: f64, italic: bool) -> MathBox {
        let mut b = MathBox { asc: TEXT_ASC * size, desc: TEXT_DESC * size, ..Default::default() };
        for c in s.chars() {
            let (gid, slant) = if italic {
                match self.glyph(math_italic(c)) {
                    Some(g) => (Some(g), false),
                    None => (self.glyph(c), true),
                }
            } else {
                (self.glyph(c), false)
            };
            match gid {
                Some(g) => {
                    b.glyphs.push(MathGlyph { gid: g, x: b.w, y: 0.0, size, slant });
                    b.w += self.adv(g, size);
                }
                None => b.w += size * 0.5,
            }
        }
        b
    }

    /// A glyph drawn at its own size with the middle of its ink on height `centre` above the baseline.
    fn centred(&self, c: char, size: f64, centre: f64) -> Option<MathBox> {
        let gid = self.glyph(c)?;
        let k = size / self.face.upem.max(1.0);
        let (x0, y0, x1, y1) = FontDb::global().glyph_bounds(self.face, gid)?;
        let _ = (x0, x1);
        let origin = -centre - (y0 + y1) / 2.0 * k;
        let top = origin + y0 * k;
        let bottom = origin + y1 * k;
        let w = self.adv(gid, size);
        Some(MathBox { w, asc: -top, desc: bottom, glyphs: vec![MathGlyph { gid, x: 0.0, y: origin, size, slant: false }], rules: vec![] })
    }
}

fn is_relation(c: char) -> bool {
    "=<>≤≥≠≈≡→⇒⇔∈∉⊂".contains(c)
}

fn is_binary(c: char) -> bool {
    "+−±∓×÷·".contains(c)
}

fn single(e: &Expr) -> Option<char> {
    match e {
        Expr::Text(t) if t.chars().count() == 1 => t.chars().next(),
        _ => None,
    }
}

fn row(cx: &Ctx, items: &[Expr], size: f64, depth: usize) -> MathBox {
    let mut out = MathBox::default();
    let mut x = 0.0;
    let mut k = 0;
    while k < items.len() {
        let it = &items[k];
        // A run of digits, or of variables, is one piece: no gaps between its characters.
        let part = match it {
            Expr::Var(_) => {
                let mut s = String::new();
                while let Some(Expr::Var(c)) = items.get(k) {
                    s.push(*c);
                    k += 1;
                }
                Some(cx.text(&s, size, true))
            }
            Expr::Text(t) if t.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                let mut s = String::new();
                while let Some(Expr::Text(t)) =
                    items.get(k).filter(|e| matches!(e, Expr::Text(t) if t.chars().all(|c| c.is_ascii_digit() || c == '.')))
                {
                    s.push_str(t);
                    k += 1;
                }
                Some(cx.text(&s, size, false))
            }
            _ => None,
        };
        if let Some(p) = part {
            let w = p.w;
            out.put(p, x, 0.0);
            x += w;
            continue;
        }
        k += 1;
        let prev = k.checked_sub(2).and_then(|j| items.get(j));
        let after_operator = prev.is_none_or(|p| single(p).is_some_and(|c| is_relation(c) || is_binary(c)) || matches!(p, Expr::Text(t) if t == "("));
        let (before, after) = match single(it) {
            Some(c) if is_relation(c) => (0.28 * size, 0.28 * size),
            Some(c) if is_binary(c) && !after_operator && k < items.len() => (0.22 * size, 0.22 * size),
            Some(',') => (0.0, 0.17 * size),
            _ => (0.0, if matches!(it, Expr::Func(_)) { 0.17 * size } else { 0.0 }),
        };
        let b = lay(cx, it, size, depth);
        let w = b.w;
        out.put(b, x + before, 0.0);
        x += before + w + after;
    }
    out.w = x;
    out
}

fn lay(cx: &Ctx, e: &Expr, size: f64, depth: usize) -> MathBox {
    if depth > 24 {
        return MathBox::default();
    }
    let d = depth + 1;
    match e {
        Expr::Row(items) => row(cx, items, size, d),
        Expr::Var(c) => cx.text(&c.to_string(), size, true),
        Expr::Text(t) => cx.text(t, size, false),
        Expr::Func(f) => cx.text(f, size, false),
        Expr::Empty => MathBox::default(),
        Expr::Frac(n, den) => {
            let (n, den) = (lay(cx, n, size, d), lay(cx, den, size, d));
            let (axis, t, gap) = (AXIS * size, RULE * size, 0.12 * size);
            let pad = 0.1 * size;
            let w = n.w.max(den.w) + 2.0 * pad;
            let (nw, dw) = (n.w, den.w);
            let ny = -(axis + t / 2.0 + gap + n.desc);
            let dy = -axis + t / 2.0 + gap + den.asc;
            let mut out = MathBox::default();
            out.put(n, (w - nw) / 2.0, ny);
            out.put(den, (w - dw) / 2.0, dy);
            out.rules.push(MathRule { x0: pad * 0.5, y0: -axis - t / 2.0, x1: w - pad * 0.5, y1: -axis + t / 2.0 });
            out.w = w;
            out
        }
        Expr::Stack(n, den) => {
            let (n, den) = (lay(cx, n, size, d), lay(cx, den, size, d));
            let gap = 0.18 * size;
            let w = n.w.max(den.w);
            let (nw, dw) = (n.w, den.w);
            let ny = -(AXIS * size + gap / 2.0 + n.desc);
            let dy = -AXIS * size + gap / 2.0 + den.asc;
            let mut inner = MathBox::default();
            inner.put(n, (w - nw) / 2.0, ny);
            inner.put(den, (w - dw) / 2.0, dy);
            inner.w = w;
            wrap_in_brackets(cx, inner, '(', ')', size)
        }
        Expr::Script { base, sub, sup } => {
            let b = lay(cx, base, size, d);
            let small = size * SCRIPT;
            let (up_box, down_box) = (sup.as_ref().map(|s| lay(cx, s, small, d)), sub.as_ref().map(|s| lay(cx, s, small, d)));
            let bw = b.w;
            let mut up = up_box.as_ref().map_or(0.0, |u| (0.42 * size).max(b.asc - 0.6 * u.asc));
            let mut down = down_box.as_ref().map_or(0.0, |_| (0.18 * size).max(b.desc * 0.9));
            if let (Some(u), Some(s)) = (&up_box, &down_box) {
                let gap = down - s.asc + up - u.desc;
                if gap < 0.15 * size {
                    down += 0.15 * size - gap;
                }
            }
            if up_box.is_none() {
                up = 0.0;
            }
            if down_box.is_none() {
                down = 0.0;
            }
            let mut out = MathBox::default();
            out.put(b, 0.0, 0.0);
            let mut sw = 0.0f64;
            if let Some(u) = up_box {
                sw = sw.max(u.w);
                out.put(u, bw, -up);
            }
            if let Some(s) = down_box {
                sw = sw.max(s.w);
                out.put(s, bw, down);
            }
            out.w = bw + sw + 0.05 * size;
            out
        }
        Expr::Sqrt { degree, body } => {
            let b = lay(cx, body, size, d);
            let t = RULE * size;
            let gap = 0.1 * size;
            let top = -(b.asc + gap + t);
            let bottom = b.desc + 0.03 * size;
            let inner = bottom - top;
            let mut out = MathBox::default();
            let bw = b.w;
            let mut glyph_w = 0.45 * size;
            let mut rad = None;
            if let Some(gid) = cx.glyph('√')
                && let Some((x0, y0, x1, y1)) = FontDb::global().glyph_bounds(cx.face, gid)
                && y1 > y0
            {
                // Grown until the sign's ink reaches from the bottom of the body to the bar.
                let up = cx.face.upem.max(1.0);
                let sz = (inner / ((y1 - y0) / up)).clamp(size * 0.8, size * 5.0);
                let k = sz / up;
                glyph_w = (x1 - x0) * k;
                rad = Some((gid, sz, -x0 * k, bottom - y1 * k, top));
            }
            let deg = degree.as_ref().map(|dg| lay(cx, dg, size * 0.5, d));
            let gx = deg.as_ref().map_or(0.0, |dg| (dg.w - 0.45 * glyph_w).max(0.0));
            if let Some(dg) = deg {
                let dx = (gx + 0.45 * glyph_w - dg.w).max(0.0);
                out.put(dg, dx, top + inner * 0.55);
            }
            if let Some((gid, sz, ox, oy, ink_top)) = rad {
                out.put(
                    MathBox {
                        asc: -(ink_top),
                        desc: oy,
                        glyphs: vec![MathGlyph { gid, x: gx + ox, y: oy, size: sz, slant: false }],
                        ..Default::default()
                    },
                    0.0,
                    0.0,
                );
            }
            let bx = gx + glyph_w + 0.04 * size;
            out.rules.push(MathRule { x0: bx - 0.04 * size, y0: top, x1: bx + bw + 0.1 * size, y1: top + t });
            out.put(b, bx, 0.0);
            out.asc = out.asc.max(-top);
            out.desc = out.desc.max(bottom);
            out.w = bx + bw + 0.1 * size;
            out
        }
        Expr::Nary { op, sub, sup, body } => {
            let stacked = matches!(op, '∑' | '∏');
            let big = size * if stacked { 1.4 } else { 1.9 };
            let sign = cx.centred(*op, big, AXIS * size).unwrap_or_else(|| cx.text(&op.to_string(), big, false));
            let small = size * SCRIPT * 0.9;
            let (up, down) = (sup.as_ref().map(|s| lay(cx, s, small, d)), sub.as_ref().map(|s| lay(cx, s, small, d)));
            let (sw, sa, sd) = (sign.w, sign.asc, sign.desc);
            let gap = 0.1 * size;
            let mut out = MathBox::default();
            let mut w = sw;
            if stacked {
                w = w.max(up.as_ref().map_or(0.0, |u| u.w)).max(down.as_ref().map_or(0.0, |l| l.w));
                out.put(sign, (w - sw) / 2.0, 0.0);
                if let Some(u) = up {
                    let uw = u.w;
                    let y = -(sa + gap + u.desc);
                    out.put(u, (w - uw) / 2.0, y);
                }
                if let Some(l) = down {
                    let lw = l.w;
                    let y = sd + gap + l.asc;
                    out.put(l, (w - lw) / 2.0, y);
                }
            } else {
                out.put(sign, 0.0, 0.0);
                let mut lw = 0.0f64;
                if let Some(u) = up {
                    lw = lw.max(sw * 0.6 + u.w);
                    let y = -sa + u.asc;
                    out.put(u, sw * 0.6, y);
                }
                if let Some(l) = down {
                    lw = lw.max(sw * 0.1 + l.w);
                    let y = sd - l.desc;
                    out.put(l, sw * 0.1, y);
                }
                w = sw.max(lw);
            }
            let b = lay(cx, body, size, d);
            let bw = b.w;
            let bx = w + 0.17 * size;
            out.put(b, bx, 0.0);
            out.w = bx + bw;
            out
        }
        Expr::Delim { open, close, body } => {
            let b = lay(cx, body, size, d);
            wrap_in_brackets(cx, b, *open, *close, size)
        }
    }
}

/// `b` between a pair of brackets that grow with its height.
fn wrap_in_brackets(cx: &Ctx, b: MathBox, open: char, close: char, size: f64) -> MathBox {
    let h = b.asc + b.desc;
    // The middle of the body, above the baseline.
    let centre = (b.asc - b.desc) / 2.0;
    let grown = if h > 1.15 * size { (size * h / (size * 0.95)).min(size * 3.5) } else { size };
    let mark = |c: char| cx.centred(c, grown, centre).unwrap_or_else(|| cx.text(&c.to_string(), size, false));
    let (o, c) = (mark(open), mark(close));
    let mut out = MathBox::default();
    let (ow, bw, cw) = (o.w, b.w, c.w);
    out.put(o, 0.0, 0.0);
    out.put(b, ow, 0.0);
    out.put(c, ow + bw, 0.0);
    out.w = ow + bw + cw;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equation::parse;

    fn face() -> Arc<FontFace> {
        FontDb::global().document_face("Cambria Math", "Regular")
    }

    fn lay_out(s: &str) -> MathBox {
        layout(&parse(s), &face(), 28.0)
    }

    #[test]
    fn a_fraction_stacks_the_top_over_the_bottom_with_a_bar_between() {
        let b = lay_out("(1)/(2)");
        assert_eq!(b.rules.len(), 1, "one bar");
        let bar = b.rules[0];
        let ys: Vec<f64> = b.glyphs.iter().map(|g| g.y).collect();
        assert_eq!(ys.len(), 2);
        assert!(ys.iter().any(|y| *y < bar.y0) && ys.iter().any(|y| *y > bar.y1), "one glyph above the bar and one below: {ys:?}");
        // Compare the middles of the two digits: a font whose digits differ in width (the CI
        // machine's) puts their left edges at different places even when they are centred.
        let f = face();
        let mid = |g: &MathGlyph| g.x + f.advance(g.gid) * g.size / f.upem.max(1.0) / 2.0;
        let (n, d) = (mid(&b.glyphs[0]), mid(&b.glyphs[1]));
        assert!((n - d).abs() < 1.0, "the two digits are centred on each other: {n} {d}");
        assert!(b.asc > 20.0 && b.desc > 5.0);
    }

    #[test]
    fn a_column_vector_stacks_two_numbers_in_brackets_with_no_bar() {
        let b = lay_out("vec(3,-2)");
        assert!(b.rules.is_empty(), "no bar");
        let ys: Vec<f64> = b.glyphs.iter().map(|g| g.y).collect();
        assert!(ys.iter().any(|y| *y < -2.0) && ys.iter().any(|y| *y > 2.0), "one number above the other: {ys:?}");
        assert!(crate::equation::to_omml(&parse("vec(3,-2)")).contains("noBar"));
    }

    #[test]
    fn a_power_is_smaller_and_higher_than_its_base() {
        let b = lay_out("x^2");
        assert_eq!(b.glyphs.len(), 2);
        assert!(b.glyphs[1].size < b.glyphs[0].size);
        assert!(b.glyphs[1].y < b.glyphs[0].y - 5.0);
        assert!(b.glyphs[1].x > b.glyphs[0].x);
    }

    #[test]
    fn a_square_root_has_a_bar_over_what_it_holds() {
        let b = lay_out("sqrt(x+1)");
        assert_eq!(b.rules.len(), 1);
        let bar = b.rules[0];
        assert!(b.glyphs.iter().skip(1).all(|g| g.y > bar.y1), "the body sits under the bar");
        assert!(bar.x1 - bar.x0 > 20.0);
    }

    #[test]
    fn brackets_grow_with_a_fraction_between_them() {
        let small = lay_out("(x)");
        let tall = lay_out("((1)/(2))");
        let size = |b: &MathBox| b.glyphs.iter().map(|g| g.size).fold(0.0, f64::max);
        assert!(size(&tall) > size(&small) * 1.3, "{} {}", size(&tall), size(&small));
    }

    #[test]
    fn a_sum_carries_its_limits_above_and_below() {
        let b = lay_out("sum_(i=1)^n i");
        let sign = b.glyphs.iter().map(|g| g.size).fold(0.0, f64::max);
        assert!(sign > 28.0, "the sign is large");
        let ys: Vec<f64> = b.glyphs.iter().filter(|g| g.size < 28.0).map(|g| g.y).collect();
        assert!(ys.iter().any(|y| *y < -10.0) && ys.iter().any(|y| *y > 10.0), "limits above and below: {ys:?}");
    }

    #[test]
    fn plain_equations_are_not_complex_and_structured_ones_are() {
        assert!(!is_complex(&parse("x=1+2")));
        assert!(is_complex(&parse("x^2")));
        assert!(is_complex(&parse("(1)/(2)")));
    }

    #[test]
    fn nothing_typed_breaks_the_layout() {
        for s in ["", "((((", "x^", "sqrt", "sum", "(□)/(□)", "a/", "int_0^1"] {
            let b = lay_out(s);
            assert!(b.w.is_finite() && b.asc.is_finite() && b.desc.is_finite(), "{s}");
        }
    }
}
