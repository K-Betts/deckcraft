//! A live picture of an equation as it is typed: fractions stacked, powers raised, roots drawn.
//! This is only a preview inside the Equation dialog; the slide shows the equation's plain text.

use std::sync::Arc;

use deckcraft_engine::equation::Expr;
use egui::{Color32, FontId, Galley, Painter, Pos2, Rect, Stroke, StrokeKind, TextFormat, Vec2, pos2, text::LayoutJob, vec2};

/// Drawing instructions, relative to the left end of the baseline.
enum Prim {
    Text(Pos2, Arc<Galley>),
    Line(Vec<Pos2>, f32),
    Box(Rect),
}

/// A laid-out piece of equation.
struct Lay {
    w: f32,
    /// Height above the baseline and depth below it.
    asc: f32,
    desc: f32,
    prims: Vec<Prim>,
}

impl Lay {
    fn blank() -> Lay {
        Lay { w: 0.0, asc: 0.0, desc: 0.0, prims: vec![] }
    }

    fn put(&mut self, other: Lay, dx: f32, dy: f32) {
        for p in other.prims {
            self.prims.push(match p {
                Prim::Text(at, g) => Prim::Text(at + vec2(dx, dy), g),
                Prim::Line(pts, w) => Prim::Line(pts.into_iter().map(|q| q + vec2(dx, dy)).collect(), w),
                Prim::Box(r) => Prim::Box(r.translate(vec2(dx, dy))),
            });
        }
        self.asc = self.asc.max(other.asc - dy);
        self.desc = self.desc.max(other.desc + dy);
    }
}

struct Ink {
    colour: Color32,
    faint: Color32,
}

/// The ascent of a line of text as a share of its height; close enough for a preview.
const ASCENT: f32 = 0.78;

fn text(p: &Painter, ink: &Ink, s: &str, size: f32, italic: bool) -> Lay {
    if s.is_empty() {
        return Lay::blank();
    }
    let mut job = LayoutJob::default();
    job.append(s, 0.0, TextFormat { font_id: FontId::proportional(size), color: ink.colour, italics: italic, ..Default::default() });
    let g = p.layout_job(job);
    let (w, h) = (g.size().x, g.size().y);
    let asc = h * ASCENT;
    Lay { w, asc, desc: h - asc, prims: vec![Prim::Text(pos2(0.0, -asc), g)] }
}

fn row(p: &Painter, ink: &Ink, items: &[Expr], size: f32) -> Lay {
    let mut out = Lay::blank();
    // Neighbouring variables and numbers share one run so letters sit together as typed.
    let mut x = 0.0;
    let mut k = 0;
    while k < items.len() {
        let upright = |e: &Expr| matches!(e, Expr::Text(t) if t.chars().all(|c| c.is_ascii_digit() || c == '.'));
        let lay = if let Expr::Var(_) = items[k] {
            let mut run = String::new();
            while let Some(Expr::Var(c)) = items.get(k) {
                run.push(*c);
                k += 1;
            }
            text(p, ink, &run, size, true)
        } else if upright(&items[k]) {
            let mut run = String::new();
            while let Some(Expr::Text(t)) = items.get(k).filter(|e| upright(e)) {
                run.push_str(t);
                k += 1;
            }
            text(p, ink, &run, size, false)
        } else {
            let e = &items[k];
            k += 1;
            let l = layout(p, ink, e, size);
            // Space round relations and binary operators, as maths is set.
            if let Expr::Text(t) = e
                && t.chars().count() == 1
                && t.chars().all(|c| "=<>≤≥≠≈≡→⇒⇔+−±∓×÷·".contains(c))
            {
                x += size * 0.22;
                let w = l.w;
                out.put(l, x, 0.0);
                x += w + size * 0.22;
                continue;
            }
            l
        };
        let w = lay.w;
        out.put(lay, x, 0.0);
        x += w;
    }
    out.w = x;
    out
}

fn layout(p: &Painter, ink: &Ink, e: &Expr, size: f32) -> Lay {
    match e {
        Expr::Row(items) => row(p, ink, items, size),
        Expr::Var(c) => text(p, ink, &c.to_string(), size, true),
        Expr::Text(t) => text(p, ink, t, size, false),
        Expr::Func(f) => {
            let mut l = text(p, ink, f, size, false);
            l.w += size * 0.12;
            l
        }
        Expr::Empty => {
            let (w, h) = (size * 0.55, size * 0.8);
            Lay { w: w + size * 0.1, asc: h, desc: 0.0, prims: vec![Prim::Box(Rect::from_min_size(pos2(size * 0.05, -h), vec2(w, h)))] }
        }
        Expr::Frac(n, d) => {
            let (n, d) = (layout(p, ink, n, size), layout(p, ink, d, size));
            let (axis, gap) = (size * 0.3, size * 0.12);
            let w = n.w.max(d.w) + size * 0.2;
            let mut out = Lay::blank();
            let (nw, dw) = (n.w, d.w);
            let ny = -(axis + gap + n.desc);
            let dy = -axis + gap + d.asc;
            out.put(n, (w - nw) / 2.0, ny);
            out.put(d, (w - dw) / 2.0, dy);
            out.prims.push(Prim::Line(vec![pos2(size * 0.05, -axis), pos2(w - size * 0.05, -axis)], (size * 0.06).max(1.0)));
            out.w = w;
            out
        }
        Expr::Script { base, sub, sup } => {
            let b = layout(p, ink, base, size);
            let small = size * 0.7;
            let mut out = Lay::blank();
            let bw = b.w;
            let up = (b.asc * 0.62).max(size * 0.45);
            out.put(b, 0.0, 0.0);
            let mut w = 0.0f32;
            if let Some(s) = sup {
                let l = layout(p, ink, s, small);
                w = w.max(l.w);
                out.put(l, bw, -up);
            }
            if let Some(s) = sub {
                let l = layout(p, ink, s, small);
                w = w.max(l.w);
                let down = size * 0.22 + l.asc * 0.3;
                out.put(l, bw, down);
            }
            out.w = bw + w;
            out
        }
        Expr::Sqrt { degree, body } => {
            let b = layout(p, ink, body, size);
            let pad = size * 0.65;
            let top = -(b.asc + size * 0.14);
            let bot = b.desc + size * 0.04;
            let h = bot - top;
            let mut out = Lay::blank();
            let mut dx = 0.0;
            if let Some(d) = degree {
                let l = layout(p, ink, d, size * 0.55);
                dx = (l.w - pad * 0.35).max(0.0);
                out.put(l, 0.0, top + h * 0.38);
            }
            let (x0, bw) = (dx, b.w);
            out.prims.push(Prim::Line(
                vec![
                    pos2(x0, top + h * 0.6),
                    pos2(x0 + pad * 0.25, top + h * 0.55),
                    pos2(x0 + pad * 0.55, bot),
                    pos2(x0 + pad, top),
                    pos2(x0 + pad + bw + size * 0.1, top),
                ],
                (size * 0.06).max(1.0),
            ));
            out.put(b, x0 + pad, 0.0);
            out.asc = out.asc.max(-top);
            out.desc = out.desc.max(bot);
            out.w = x0 + pad + bw + size * 0.1;
            out
        }
        Expr::Nary { op, sub, sup, body } => {
            let big = size * 1.5;
            let o = text(p, ink, &op.to_string(), big, false);
            let small = size * 0.65;
            let stacked = matches!(op, '∑' | '∏');
            let (s, u) = (sub.as_ref().map(|e| layout(p, ink, e, small)), sup.as_ref().map(|e| layout(p, ink, e, small)));
            let mut out = Lay::blank();
            let centre = size * 0.3;
            let (ow, o_asc, o_desc) = (o.w, o.asc, o.desc);
            // Put the sign's middle on the line of the maths.
            let lift = (o_asc - o_desc) / 2.0 - centre;
            let mut w = ow;
            if stacked {
                w = w.max(s.as_ref().map_or(0.0, |l| l.w)).max(u.as_ref().map_or(0.0, |l| l.w));
                out.put(o, (w - ow) / 2.0, lift);
                if let Some(l) = u {
                    let lw = l.w;
                    let dy = -(o_asc - lift) - l.desc - size * 0.05;
                    out.put(l, (w - lw) / 2.0, dy);
                }
                if let Some(l) = s {
                    let lw = l.w;
                    let dy = (o_desc + lift) + l.asc + size * 0.05;
                    out.put(l, (w - lw) / 2.0, dy);
                }
            } else {
                out.put(o, 0.0, lift);
                let mut lw = 0.0f32;
                if let Some(l) = u {
                    lw = lw.max(l.w);
                    out.put(l, ow, -(o_asc - lift) * 0.8);
                }
                if let Some(l) = s {
                    lw = lw.max(l.w);
                    out.put(l, ow * 0.6, o_desc + lift);
                }
                w = ow * 0.6 + lw.max(ow * 0.4);
            }
            let b = layout(p, ink, body, size);
            let bw = b.w;
            out.put(b, w + size * 0.15, 0.0);
            out.w = w + size * 0.15 + bw;
            out
        }
        Expr::Stack(n, d) => {
            // A column vector: two lines with no bar, in brackets.
            let (n, d) = (layout(p, ink, n, size), layout(p, ink, d, size));
            let (axis, gap) = (size * 0.3, size * 0.16);
            let w = n.w.max(d.w);
            let mut b = Lay::blank();
            let (nw, dw) = (n.w, d.w);
            let ny = -(axis + gap + n.desc);
            let dy = -axis + gap + d.asc;
            b.put(n, (w - nw) / 2.0, ny);
            b.put(d, (w - dw) / 2.0, dy);
            b.w = w;
            brackets(p, ink, b, '(', ')', size)
        }
        Expr::Delim { open, close, body } => {
            let b = layout(p, ink, body, size);
            brackets(p, ink, b, *open, *close, size)
        }
    }
}

/// `b` between a pair of brackets as tall as it is.
fn brackets(p: &Painter, ink: &Ink, b: Lay, open: char, close: char, size: f32) -> Lay {
    {
        {
            let h = (b.asc + b.desc).max(size);
            let mark = |c: char| {
                let mut l = text(p, ink, &c.to_string(), (h * 1.02).max(size), false);
                // Centre the bracket on the maths line.
                let mid = (l.asc - l.desc) / 2.0;
                let shift = (b.asc - b.desc) / 2.0 - mid;
                for pr in &mut l.prims {
                    if let Prim::Text(at, _) = pr {
                        at.y += -shift;
                    }
                }
                l.asc = l.asc.max(b.asc);
                l.desc = l.desc.max(b.desc);
                l
            };
            let (o, c) = (mark(open), mark(close));
            let mut out = Lay::blank();
            let (ow, bw) = (o.w, b.w);
            out.put(o, 0.0, 0.0);
            out.put(b, ow, 0.0);
            let cw = c.w;
            out.put(c, ow + bw, 0.0);
            out.w = ow + bw + cw;
            out
        }
    }
}

/// The room the equation needs at this size.
pub fn measure(p: &Painter, e: &Expr, size: f32) -> Vec2 {
    let l = layout(p, &Ink { colour: Color32::BLACK, faint: Color32::GRAY }, e, size);
    vec2(l.w, l.asc + l.desc)
}

/// Draw the equation centred in `area`, shrinking it if it would not fit.
pub fn paint(p: &Painter, area: Rect, e: &Expr, colour: Color32, faint: Color32) {
    let ink = Ink { colour, faint };
    let mut size = 34.0;
    let mut lay = layout(p, &ink, e, size);
    while (lay.w > area.width() - 24.0 || lay.asc + lay.desc > area.height() - 12.0) && size > 12.0 {
        size -= 2.0;
        lay = layout(p, &ink, e, size);
    }
    let origin = pos2(area.center().x - lay.w / 2.0, area.center().y + (lay.asc - lay.desc) / 2.0);
    for pr in lay.prims {
        match pr {
            Prim::Text(at, g) => p.galley(origin + at.to_vec2(), g, colour),
            Prim::Line(pts, w) => {
                p.add(egui::Shape::line(pts.into_iter().map(|q| origin + q.to_vec2()).collect(), Stroke::new(w, colour)));
            }
            Prim::Box(r) => {
                p.rect_stroke(r.translate(origin.to_vec2()), 2.0, Stroke::new(1.2, ink.faint), StrokeKind::Inside);
            }
        }
    }
}
