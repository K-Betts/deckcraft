//! The face of the scientific calculator: a natural display that draws stacked fractions, root
//! signs and raised powers while you type, and the keypad. The sums are done in `calculator`.

use egui::{Align2, Color32, CornerRadius, Event, FontId, Key as EKey, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};

use crate::calculator::{Calc, Key, Outcome, Tok};
use crate::calculator_apps::{App, Device, EQUATIONS, Form, HOME_APPS, Menu, Overlay, Prompt};

const LCD: Color32 = Color32::from_rgb(0xD9, 0xDF, 0xD6);
const LCD_INK: Color32 = Color32::from_rgb(0x10, 0x13, 0x16);
const LCD_DIM: Color32 = Color32::from_rgb(0x7F, 0x88, 0x7D);
/// The case of an fx-991CW+: near-black plastic with grey round keys.
const BODY: Color32 = Color32::from_rgb(0x1C, 0x1D, 0x20);
const KEY_NUM: Color32 = Color32::from_rgb(0x4D, 0x4F, 0x55);
const KEY_FUNC: Color32 = Color32::from_rgb(0x30, 0x32, 0x37);
const KEY_PAD: Color32 = Color32::from_rgb(0x0B, 0x0B, 0x0D);
const KEY_SHIFT: Color32 = Color32::from_rgb(0xC9, 0xA8, 0x5A);
/// The gold of the SHIFT legends printed above the keys.
const LEGEND: Color32 = Color32::from_rgb(0xC9, 0xA8, 0x5A);
const PRINT: Color32 = Color32::from_rgb(0xE4, 0xE6, 0xEA);

/// The keypad is laid out in the coordinates of a photograph of the real thing, 870 wide by 1830
/// tall from the top left of the case, and scaled to the size asked for.
const CASE_W: f32 = 870.0;
const CASE_H: f32 = 1650.0;
const CASE_X: f32 = 530.0;
const CASE_Y: f32 = 230.0;
/// The width of the calculator at scale 1.
const BASE_W: f32 = 380.0;

/// How wide the calculator is at `scale`.
pub fn width(scale: f32) -> f32 {
    BASE_W * scale
}

fn height(scale: f32) -> f32 {
    CASE_H * BASE_W / CASE_W * scale
}

// ---- Natural display -----------------------------------------------------------------------

enum Node {
    Tok { idx: usize, tok: Tok },
    Frac { open: usize, top: Row, bottom: Row },
    Sqrt { open: usize, body: Row },
    Root { open: usize, index: Row, body: Row },
    Sup { open: usize, body: Row },
    Log { open: usize, base: Row, arg: Row },
    Mix { open: usize, whole: Row, top: Row, bottom: Row },
}

struct Row {
    start: usize,
    end: usize,
    nodes: Vec<Node>,
}

impl Node {
    fn first(&self) -> usize {
        match self {
            Node::Tok { idx, .. } => *idx,
            Node::Frac { open, .. }
            | Node::Sqrt { open, .. }
            | Node::Root { open, .. }
            | Node::Sup { open, .. }
            | Node::Log { open, .. }
            | Node::Mix { open, .. } => *open,
        }
    }
}

/// Group the flat tokens into rows of nodes. A template left open at the end is closed there.
fn build(toks: &[Tok], i: &mut usize, inside: bool) -> Row {
    let start = *i;
    let mut nodes = vec![];
    while let Some(&t) = toks.get(*i) {
        if t == Tok::Close || t.is_mid() {
            if inside {
                break;
            }
            *i += 1;
            continue;
        }
        let at = *i;
        *i += 1;
        let node = match t {
            Tok::FracOpen => {
                let top = build(toks, i, true);
                if toks.get(*i) == Some(&Tok::FracMid) {
                    *i += 1;
                }
                let bottom = build(toks, i, true);
                Node::Frac { open: at, top, bottom }
            }
            Tok::SqrtOpen => Node::Sqrt { open: at, body: build(toks, i, true) },
            Tok::RootOpen => {
                let index = build(toks, i, true);
                if toks.get(*i) == Some(&Tok::RootMid) {
                    *i += 1;
                }
                Node::Root { open: at, index, body: build(toks, i, true) }
            }
            Tok::SupOpen => Node::Sup { open: at, body: build(toks, i, true) },
            Tok::LogOpen => {
                let base = build(toks, i, true);
                if toks.get(*i) == Some(&Tok::LogMid) {
                    *i += 1;
                }
                Node::Log { open: at, base, arg: build(toks, i, true) }
            }
            Tok::MixOpen => {
                let whole = build(toks, i, true);
                if toks.get(*i) == Some(&Tok::FracMid) {
                    *i += 1;
                }
                let top = build(toks, i, true);
                if toks.get(*i) == Some(&Tok::FracMid) {
                    *i += 1;
                }
                Node::Mix { open: at, whole, top, bottom: build(toks, i, true) }
            }
            tok => Node::Tok { idx: at, tok },
        };
        if !matches!(node, Node::Tok { .. }) && toks.get(*i) == Some(&Tok::Close) {
            *i += 1;
        }
        nodes.push(node);
    }
    Row { start, end: *i, nodes }
}

#[derive(Clone, Copy, Default)]
struct Size {
    w: f32,
    above: f32,
    below: f32,
}

struct Set<'a> {
    p: &'a Painter,
    ink: Color32,
    /// Draw nothing, only find where the cursor would be.
    dry: bool,
    cursor: Option<usize>,
    cursor_at: Option<(Pos2, f32)>,
}

const MIN_SIZE: f32 = 6.0;
const AXIS: f32 = 0.3;

fn pad_of(t: Tok, fs: f32) -> f32 {
    match t {
        Tok::Plus | Tok::Minus | Tok::Times | Tok::Divide | Tok::Ncr | Tok::Npr => fs * 0.2,
        _ => 0.0,
    }
}

impl Set<'_> {
    fn text_size(&self, text: &str, fs: f32) -> f32 {
        if text.is_empty() { 0.0 } else { self.p.layout_no_wrap(text.to_string(), FontId::proportional(fs), self.ink).size().x }
    }

    fn slot_size(fs: f32) -> Size {
        Size { w: fs * 0.55, above: fs * 0.75, below: fs * 0.2 }
    }

    fn row_size(&self, row: &Row, fs: f32) -> Size {
        if row.nodes.is_empty() {
            return Self::slot_size(fs);
        }
        row.nodes.iter().map(|n| self.node_size(n, fs)).fold(Size::default(), |a, b| Size {
            w: a.w + b.w,
            above: a.above.max(b.above),
            below: a.below.max(b.below),
        })
    }

    fn node_size(&self, node: &Node, fs: f32) -> Size {
        let line = Size { w: 0.0, above: fs * 0.78, below: fs * 0.25 };
        match node {
            Node::Tok { tok, .. } => Size { w: self.text_size(&tok.text(), fs) + 2.0 * pad_of(*tok, fs), ..line },
            Node::Frac { top, bottom, .. } => {
                let (t, b) = (self.row_size(top, fs), self.row_size(bottom, fs));
                let gap = fs * 0.12;
                Size { w: t.w.max(b.w) + fs * 0.3, above: fs * AXIS + gap + t.above + t.below, below: (gap + b.above + b.below - fs * AXIS).max(0.0) }
            }
            Node::Sqrt { body, .. } => {
                let b = self.row_size(body, fs);
                Size { w: fs * 0.62 + b.w + fs * 0.08, above: b.above + fs * 0.16, below: b.below }
            }
            Node::Root { index, body, .. } => {
                let (i, b) = (self.row_size(index, (fs * 0.6).max(MIN_SIZE)), self.row_size(body, fs));
                let lift = (b.above * 0.5).max(i.below + fs * 0.1);
                Size { w: i.w.max(fs * 0.25) + fs * 0.62 + b.w + fs * 0.08, above: (b.above + fs * 0.16).max(lift + i.above), below: b.below }
            }
            Node::Sup { body, .. } => {
                let f = (fs * 0.66).max(MIN_SIZE);
                let b = self.row_size(body, f);
                let raise = fs * 0.42;
                Size { w: b.w + fs * 0.04, above: raise + b.above, below: (b.below - raise).max(0.0) }
            }
            Node::Log { base, arg, .. } => {
                let f = (fs * 0.62).max(MIN_SIZE);
                let (b, a) = (self.row_size(base, f), self.row_size(arg, fs));
                let word = self.text_size("log", fs);
                Size { w: word + b.w + fs * 0.2 + a.w + fs * 0.2, above: line.above.max(a.above), below: (fs * 0.25 + b.below).max(a.below) }
            }
            Node::Mix { whole, top, bottom, .. } => {
                let (w, t, b) = (self.row_size(whole, fs), self.row_size(top, fs), self.row_size(bottom, fs));
                let gap = fs * 0.12;
                Size {
                    w: w.w + t.w.max(b.w) + fs * 0.4,
                    above: (fs * AXIS + gap + t.above + t.below).max(w.above),
                    below: (gap + b.above + b.below - fs * AXIS).max(0.0).max(w.below),
                }
            }
        }
    }

    fn text(&self, x: f32, base: f32, text: &str, fs: f32) {
        if self.dry || text.is_empty() {
            return;
        }
        let g = self.p.layout_no_wrap(text.to_string(), FontId::proportional(fs), self.ink);
        let top = base - fs * 0.78 - (g.size().y - fs * 1.03) / 2.0;
        self.p.galley(pos2(x, top), g, self.ink);
    }

    fn mark_cursor(&mut self, idx: usize, x: f32, base: f32, fs: f32) {
        if self.cursor == Some(idx) && self.cursor_at.is_none() {
            self.cursor_at = Some((pos2(x, base), fs));
        }
    }

    fn paint_row(&mut self, row: &Row, x0: f32, base: f32, fs: f32) -> f32 {
        let mut x = x0;
        if row.nodes.is_empty() {
            let s = Self::slot_size(fs);
            if !self.dry {
                let r = Rect::from_min_size(pos2(x + fs * 0.06, base - s.above * 0.82), vec2(s.w - fs * 0.12, s.above * 0.82 + s.below));
                self.p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, self.ink.gamma_multiply(0.55)), egui::StrokeKind::Inside);
            }
            self.mark_cursor(row.start, x + fs * 0.03, base, fs);
            return s.w;
        }
        for node in &row.nodes {
            self.mark_cursor(node.first(), x, base, fs);
            x += self.paint_node(node, x, base, fs);
        }
        self.mark_cursor(row.end, x, base, fs);
        x - x0
    }

    fn paint_node(&mut self, node: &Node, x: f32, base: f32, fs: f32) -> f32 {
        let size = self.node_size(node, fs);
        match node {
            Node::Tok { tok, .. } => {
                let pad = pad_of(*tok, fs);
                self.text(x + pad, base, &tok.text(), fs);
            }
            Node::Frac { top, bottom, .. } => {
                let (t, b) = (self.row_size(top, fs), self.row_size(bottom, fs));
                let gap = fs * 0.12;
                let axis = base - fs * AXIS;
                self.paint_row(top, x + (size.w - t.w) / 2.0, axis - gap - t.below, fs);
                self.paint_row(bottom, x + (size.w - b.w) / 2.0, axis + gap + b.above, fs);
                if !self.dry {
                    self.p.line_segment([pos2(x + fs * 0.05, axis), pos2(x + size.w - fs * 0.05, axis)], Stroke::new((fs * 0.06).max(1.2), self.ink));
                }
            }
            Node::Sqrt { body, .. } => {
                let b = self.row_size(body, fs);
                let (top, bottom) = (base - b.above - fs * 0.12, base + b.below);
                self.radical(x, top, bottom, size.w, fs);
                self.paint_row(body, x + fs * 0.62, base, fs);
            }
            Node::Root { index, body, .. } => {
                let small = (fs * 0.6).max(MIN_SIZE);
                let (i, b) = (self.row_size(index, small), self.row_size(body, fs));
                let ix = i.w.max(fs * 0.25);
                let (top, bottom) = (base - b.above - fs * 0.12, base + b.below);
                let lift = (b.above * 0.5).max(i.below + fs * 0.1);
                self.paint_row(index, x + ix - i.w, base - lift, small);
                self.radical(x + ix, top, bottom, size.w - ix, fs);
                self.paint_row(body, x + ix + fs * 0.62, base, fs);
            }
            Node::Sup { body, .. } => {
                let f = (fs * 0.66).max(MIN_SIZE);
                self.paint_row(body, x, base - fs * 0.42, f);
            }
            Node::Log { base: b, arg, .. } => {
                let f = (fs * 0.62).max(MIN_SIZE);
                let word = self.text_size("log", fs);
                self.text(x, base, "log", fs);
                let bw = self.paint_row(b, x + word, base + fs * 0.25, f);
                self.text(x + word + bw + fs * 0.05, base, "(", fs);
                let open = self.text_size("(", fs);
                let aw = self.paint_row(arg, x + word + bw + fs * 0.05 + open, base, fs);
                self.text(x + word + bw + fs * 0.05 + open + aw, base, ")", fs);
            }
            Node::Mix { whole, top, bottom, .. } => {
                let w = self.paint_row(whole, x, base, fs);
                let (t, b) = (self.row_size(top, fs), self.row_size(bottom, fs));
                let width = t.w.max(b.w) + fs * 0.3;
                let gap = fs * 0.12;
                let axis = base - fs * AXIS;
                let fx = x + w + fs * 0.1;
                self.paint_row(top, fx + (width - t.w) / 2.0, axis - gap - t.below, fs);
                self.paint_row(bottom, fx + (width - b.w) / 2.0, axis + gap + b.above, fs);
                if !self.dry {
                    self.p
                        .line_segment([pos2(fx + fs * 0.05, axis), pos2(fx + width - fs * 0.05, axis)], Stroke::new((fs * 0.06).max(1.2), self.ink));
                }
            }
        }
        size.w
    }

    /// The root sign and the bar over what it covers.
    fn radical(&self, x: f32, top: f32, bottom: f32, w: f32, fs: f32) {
        if self.dry {
            return;
        }
        let h = bottom - top;
        let pts = vec![
            pos2(x + fs * 0.02, top + h * 0.62),
            pos2(x + fs * 0.12, top + h * 0.55),
            pos2(x + fs * 0.27, bottom),
            pos2(x + fs * 0.52, top),
            pos2(x + w - fs * 0.04, top),
        ];
        self.p.add(Shape::line(pts, Stroke::new((fs * 0.07).max(1.3), self.ink)));
    }
}

/// Draw `toks` with its baseline at `base`. `align_right` ends it at `x`, otherwise it starts there.
/// Returns where the cursor is (point on the baseline and type size) if `cursor` was given.
#[allow(clippy::too_many_arguments)]
fn typeset(
    p: &Painter,
    toks: &[Tok],
    x: f32,
    base: f32,
    fs: f32,
    ink: Color32,
    align_right: bool,
    cursor: Option<usize>,
    scroll_to_cursor: Option<f32>,
) -> Option<(Pos2, f32)> {
    let mut i = 0;
    let row = build(toks, &mut i, false);
    let mut set = Set { p, ink, dry: true, cursor, cursor_at: None };
    let width = set.row_size(&row, fs).w;
    let start = if align_right {
        x - width
    } else if let Some(room) = scroll_to_cursor {
        set.paint_row(&row, 0.0, base, fs);
        let at = set.cursor_at.map_or(0.0, |(c, _)| c.x);
        x - (at - room).max(0.0)
    } else {
        x
    };
    set.dry = false;
    set.cursor_at = None;
    set.paint_row(&row, start, base, fs);
    set.cursor_at
}

// ---- Keys ----------------------------------------------------------------------------------

/// What is printed on a key or above it.
#[derive(Clone, Copy)]
enum Cap {
    Text(&'static str),
    /// A word with a raised part: `x²`, `sin⁻¹`. An empty raised part is an empty box.
    Sup(&'static str, &'static str),
    /// A template drawn with the same typesetting as the display.
    Toks(&'static [Tok]),
    Icon(Icon),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Icon {
    Left,
    Right,
    Up,
    Down,
    Home,
    Settings,
    Back,
    Variable,
    Catalog,
    Tools,
    Shift,
    Backspace,
    Format,
    PageUp,
    PageDown,
}

#[derive(Clone, Copy, PartialEq)]
enum Style {
    /// Digits and the four rules: the lighter grey.
    Num,
    Func,
    /// The d-pad and OK.
    Pad,
    Shift,
}

#[derive(Clone, Copy)]
enum Shape2 {
    Round(f32),
    Pill(f32, f32),
}

struct KeyDef {
    /// Centre in photograph coordinates.
    at: (f32, f32),
    shape: Shape2,
    cap: Cap,
    key: Key,
    shift: Option<(Cap, Key)>,
    /// Printed above the key, e.g. `HOME`.
    label: Option<&'static str>,
    style: Style,
}

const MINUS_ONE: &str = "\u{2212}1";
const FRAC: &[Tok] = &[Tok::FracOpen, Tok::FracMid, Tok::Close];
const MIXED: &[Tok] = &[Tok::MixOpen, Tok::FracMid, Tok::FracMid, Tok::Close];
const SQRT: &[Tok] = &[Tok::SqrtOpen, Tok::Close];
const NROOT: &[Tok] = &[Tok::RootOpen, Tok::RootMid, Tok::Close];
const LOGB: &[Tok] = &[Tok::LogOpen, Tok::LogMid, Tok::Close];

fn key(at: (f32, f32), shape: Shape2, cap: Cap, key: Key, style: Style) -> KeyDef {
    KeyDef { at, shape, cap, key, shift: None, label: None, style }
}

fn with(mut d: KeyDef, cap: Cap, shifted: Key) -> KeyDef {
    d.shift = Some((cap, shifted));
    d
}

fn labelled(mut d: KeyDef, label: &'static str) -> KeyDef {
    d.label = Some(label);
    d
}

/// Every key, where it is on the real calculator (the fx-991CW+).
fn keys() -> Vec<KeyDef> {
    use Cap::{Icon as I, Sup, Text, Toks};
    use Key::*;
    use Shape2::{Pill, Round};
    use Style::{Func as F, Num, Pad, Shift};
    let cols6 = [645.0, 773.0, 900.0, 1030.0, 1160.0, 1287.0];
    let cols5 = [650.0, 808.0, 965.0, 1123.0, 1282.0];
    let small = Pill(88.0, 72.0);
    let big = Round(54.0);
    let mut v = vec![
        key((645.0, 757.0), Round(33.0), Text("ON"), On, F),
        labelled(key((773.0, 765.0), Round(37.0), I(Icon::Home), Home, F), "HOME"),
        labelled(key((645.0, 878.0), Round(42.0), I(Icon::Settings), Settings, F), "SETTINGS"),
        key((773.0, 878.0), Round(42.0), I(Icon::Back), Back, F),
        key((1030.0, 767.0), Round(40.0), I(Icon::Up), Up, Pad),
        key((900.0, 880.0), Round(40.0), I(Icon::Left), Left, Pad),
        key((1030.0, 880.0), Round(42.0), Text("OK"), Ok, Pad),
        key((1160.0, 880.0), Round(40.0), I(Icon::Right), Right, Pad),
        key((1030.0, 993.0), Round(40.0), I(Icon::Down), Down, Pad),
        key((1287.0, 772.0), Pill(70.0, 76.0), I(Icon::PageUp), PageUp, Pad),
        key((1287.0, 868.0), Pill(70.0, 76.0), I(Icon::PageDown), PageDown, Pad),
        labelled(key((645.0, 1013.0), Round(42.0), I(Icon::Shift), Shift_, Shift), "SHIFT"),
        labelled(key((773.0, 1013.0), Round(40.0), I(Icon::Variable), Variable, F), "VARIABLE"),
        labelled(key((900.0, 1013.0), Round(40.0), Text("f(x)"), Function, F), "FUNCTION"),
        labelled(key((1160.0, 1013.0), Round(40.0), I(Icon::Catalog), Catalog, F), "CATALOG"),
        labelled(key((1287.0, 1013.0), Round(40.0), I(Icon::Tools), Tools, F), "TOOLS"),
    ];
    let row_d = [
        key((cols6[0], 1127.0), small, Text("x"), X, F),
        with(key((cols6[1], 1127.0), small, Toks(FRAC), Frac, F), Toks(MIXED), Mixed),
        with(key((cols6[2], 1127.0), small, Toks(SQRT), Sqrt, F), Toks(NROOT), NRoot),
        with(key((cols6[3], 1127.0), small, Sup("x", ""), Pow, F), Sup("x", MINUS_ONE), Inverse),
        with(key((cols6[4], 1127.0), small, Sup("x", "2"), Square, F), Text("log"), Func(Func_::Log)),
        with(key((cols6[5], 1127.0), small, Toks(LOGB), LogBase, F), Text("ln"), Func(Func_::Ln)),
    ];
    v.extend(row_d);
    let trig = |f, g, name: &'static str, i: usize| with(key((cols6[i], 1240.0), small, Text(name), Func(f), F), Sup(name, MINUS_ONE), Func(g));
    v.extend([
        key((cols6[0], 1240.0), small, Text("Ans"), Ans, F),
        trig(Func_::Sin, Func_::ASin, "sin", 1),
        trig(Func_::Cos, Func_::ACos, "cos", 2),
        trig(Func_::Tan, Func_::ATan, "tan", 3),
        key((cols6[4], 1240.0), small, Text("("), LParen, F),
        key((cols6[5], 1240.0), small, Text(")"), RParen, F),
    ]);
    let digit = |d: u8, label: &'static str, row: usize, col: usize, y: f32, letter: Option<(&'static str, Key)>| {
        let k = key((cols5[col], y), big, Text(label), Digit(d), Num);
        let _ = row;
        match letter {
            Some((l, key)) => with(k, Text(l), key),
            None => k,
        }
    };
    v.extend([
        digit(7, "7", 0, 0, 1365.0, Some(("\u{03C0}", Pi))),
        digit(8, "8", 0, 1, 1365.0, Some(("e", E))),
        key((cols5[2], 1365.0), big, Text("9"), Digit(9), Num),
        key((cols5[3], 1365.0), big, Icon_(Icon::Backspace), Del, Num),
        with(key((cols5[4], 1365.0), big, Text("AC"), Ac, Num), Text("OFF"), Off),
        digit(4, "4", 1, 0, 1498.0, Some(("A", Var(0)))),
        digit(5, "5", 1, 1, 1498.0, Some(("B", Var(1)))),
        digit(6, "6", 1, 2, 1498.0, Some(("C", Var(2)))),
        key((cols5[3], 1498.0), big, Text("\u{00D7}"), Times, Num),
        key((cols5[4], 1498.0), big, Text("\u{00F7}"), Divide, Num),
        digit(1, "1", 2, 0, 1630.0, Some(("D", Var(3)))),
        digit(2, "2", 2, 1, 1630.0, Some(("E", Var(4)))),
        digit(3, "3", 2, 2, 1630.0, Some(("F", Var(5)))),
        key((cols5[3], 1630.0), big, Text("+"), Plus, Num),
        with(key((cols5[4], 1630.0), big, Text("\u{2212}"), Minus, Num), Text("(\u{2212})"), Neg),
        digit(0, "0", 3, 0, 1762.0, Some(("x", Var(6)))),
        with(key((cols5[1], 1762.0), big, Text("."), Dot, Num), Text("y"), Var(7)),
        with(key((cols5[2], 1762.0), big, Sup("\u{00D7}10", ""), Exp10, Num), Text("z"), Var(8)),
        with(key((cols5[3], 1762.0), big, Icon_(Icon::Format), Sd, Num), Text("\u{21BB}"), FormatMenu),
        with(key((cols5[4], 1762.0), big, Text("EXE"), Equals, Num), Text("\u{2248}"), ExeDecimal),
    ]);
    v
}

use crate::calculator::Func as Func_;
use Cap::Icon as Icon_;
use Key::Shift as Shift_;

fn key_name(def: &KeyDef) -> String {
    match def.key {
        Key::Frac => "Fraction".into(),
        Key::Sqrt => "Square root".into(),
        Key::Square => "Square".into(),
        Key::Pow => "Power".into(),
        Key::LogBase => "Logarithm".into(),
        Key::Left => "Cursor left".into(),
        Key::Right => "Cursor right".into(),
        Key::Up => "Cursor up".into(),
        Key::Down => "Cursor down".into(),
        Key::Sd => "FORMAT".into(),
        Key::Neg => "Negative sign".into(),
        Key::Exp10 => "Times ten to the power".into(),
        Key::Ok => "OK".into(),
        Key::Home => "HOME".into(),
        Key::Settings => "SETTINGS".into(),
        Key::Back => "Back".into(),
        Key::Variable => "VARIABLE".into(),
        Key::Function => "FUNCTION".into(),
        Key::Catalog => "CATALOG".into(),
        Key::Tools => "TOOLS".into(),
        Key::Shift => "SHIFT".into(),
        Key::Del => "DEL".into(),
        Key::PageUp => "Page up".into(),
        Key::PageDown => "Page down".into(),
        Key::Equals => "EXE".into(),
        _ => match def.cap {
            Cap::Text(t) => t.to_string(),
            Cap::Sup(base, sup) => format!("{base} {sup}"),
            _ => String::new(),
        },
    }
}

fn paint_icon(p: &Painter, icon: Icon, c: Pos2, r: f32, ink: Color32) {
    let s = Stroke::new((r * 0.13).max(1.4), ink);
    match icon {
        Icon::Left | Icon::Right | Icon::Up | Icon::Down => {
            let d = r * 0.5;
            let (a, b, e) = match icon {
                Icon::Left => (vec2(d * 0.5, -d), vec2(-d * 0.5, 0.0), vec2(d * 0.5, d)),
                Icon::Right => (vec2(-d * 0.5, -d), vec2(d * 0.5, 0.0), vec2(-d * 0.5, d)),
                Icon::Up => (vec2(-d, d * 0.5), vec2(0.0, -d * 0.5), vec2(d, d * 0.5)),
                _ => (vec2(-d, -d * 0.5), vec2(0.0, d * 0.5), vec2(d, -d * 0.5)),
            };
            p.add(Shape::line(vec![c + a, c + b, c + e], s));
        }
        Icon::PageUp | Icon::PageDown => {
            let sign = if icon == Icon::PageUp { -1.0 } else { 1.0 };
            let d = r * 0.42;
            for off in [-0.32, 0.32] {
                let y = -(off * r * sign);
                p.add(Shape::line(vec![c + vec2(-d, y + d * 0.5 * sign), c + vec2(0.0, y - d * 0.5 * sign), c + vec2(d, y + d * 0.5 * sign)], s));
            }
        }
        Icon::Home => {
            let d = r * 0.5;
            p.add(Shape::line(vec![c + vec2(-d, 0.0), c + vec2(0.0, -d), c + vec2(d, 0.0)], s));
            p.add(Shape::line(
                vec![c + vec2(-d * 0.7, -d * 0.1), c + vec2(-d * 0.7, d * 0.8), c + vec2(d * 0.7, d * 0.8), c + vec2(d * 0.7, -d * 0.1)],
                s,
            ));
        }
        Icon::Settings => {
            for (i, y) in [-0.4, 0.0, 0.4].iter().enumerate() {
                let yy = c.y + y * r;
                p.line_segment([pos2(c.x - r * 0.5, yy), pos2(c.x + r * 0.5, yy)], s);
                p.circle_filled(pos2(c.x + [-0.2, 0.25, -0.05][i] * r, yy), r * 0.13, ink);
            }
        }
        Icon::Back => {
            let d = r * 0.45;
            p.add(Shape::line(
                vec![
                    c + vec2(d, -d * 0.6),
                    c + vec2(-d * 0.4, -d * 0.6),
                    c + vec2(-d, -d * 0.2),
                    c + vec2(-d, d * 0.3),
                    c + vec2(-d * 0.4, d * 0.7),
                    c + vec2(d * 0.8, d * 0.7),
                ],
                s,
            ));
            p.add(Shape::line(vec![c + vec2(-d * 0.2, -d * 1.2), c + vec2(-d, -d * 0.6), c + vec2(-d * 0.2, 0.0)], s));
        }
        Icon::Variable => {
            p.text(c, Align2::CENTER_CENTER, "\u{2192}x", FontId::proportional(r * 0.9), ink);
        }
        Icon::Catalog => {
            let d = r * 0.5;
            p.add(Shape::line(
                vec![
                    c + vec2(0.0, -d * 0.6),
                    c + vec2(-d, -d * 0.8),
                    c + vec2(-d, d * 0.6),
                    c + vec2(0.0, d * 0.8),
                    c + vec2(d, d * 0.6),
                    c + vec2(d, -d * 0.8),
                    c + vec2(0.0, -d * 0.6),
                    c + vec2(0.0, d * 0.8),
                ],
                s,
            ));
        }
        Icon::Tools => {
            for dx in [-0.5, 0.0, 0.5] {
                p.circle_stroke(c + vec2(dx * r, 0.0), r * 0.17, s);
            }
        }
        Icon::Shift => {
            let d = r * 0.5;
            p.add(Shape::closed_line(
                vec![
                    c + vec2(0.0, -d),
                    c + vec2(d, 0.0),
                    c + vec2(d * 0.4, 0.0),
                    c + vec2(d * 0.4, d),
                    c + vec2(-d * 0.4, d),
                    c + vec2(-d * 0.4, 0.0),
                    c + vec2(-d, 0.0),
                ],
                s,
            ));
        }
        Icon::Backspace => {
            let d = r * 0.5;
            p.add(Shape::convex_polygon(
                vec![c + vec2(-d, 0.0), c + vec2(-d * 0.4, -d * 0.7), c + vec2(d, -d * 0.7), c + vec2(d, d * 0.7), c + vec2(-d * 0.4, d * 0.7)],
                ink,
                Stroke::NONE,
            ));
            let x = Stroke::new(s.width, KEY_NUM);
            let k = d * 0.3;
            p.line_segment([c + vec2(0.0, -k), c + vec2(k * 2.0, k)], x);
            p.line_segment([c + vec2(0.0, k), c + vec2(k * 2.0, -k)], x);
        }
        Icon::Format => {
            p.text(c, Align2::CENTER_CENTER, "FORMAT", FontId::proportional(r * 0.48), ink);
        }
    }
}

fn paint_cap(p: &Painter, cap: Cap, centre: Pos2, fs: f32, ink: Color32, max_w: f32) {
    match cap {
        Cap::Text(t) => {
            let g = p.layout_no_wrap(t.to_string(), FontId::proportional(fs), ink);
            let fs = if g.size().x > max_w { fs * max_w / g.size().x } else { fs };
            p.text(centre, Align2::CENTER_CENTER, t, FontId::proportional(fs), ink);
        }
        Cap::Sup(base, sup) => {
            let small = fs * 0.66;
            let bw = p.layout_no_wrap(base.to_string(), FontId::proportional(fs), ink).size().x;
            let sw = if sup.is_empty() { small * 0.55 } else { p.layout_no_wrap(sup.to_string(), FontId::proportional(small), ink).size().x };
            let left = centre.x - (bw + sw) / 2.0;
            p.text(pos2(left, centre.y + fs * 0.1), Align2::LEFT_CENTER, base, FontId::proportional(fs), ink);
            if sup.is_empty() {
                let r = Rect::from_min_size(pos2(left + bw + 1.0, centre.y - fs * 0.62), vec2(small * 0.5, small * 0.6));
                p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, ink), egui::StrokeKind::Inside);
            } else {
                p.text(pos2(left + bw, centre.y - fs * 0.38), Align2::LEFT_CENTER, sup, FontId::proportional(small), ink);
            }
        }
        Cap::Toks(toks) => {
            let mut i = 0;
            let row = build(toks, &mut i, false);
            let mut set = Set { p, ink, dry: true, cursor: None, cursor_at: None };
            let s = set.row_size(&row, fs);
            set.dry = false;
            set.paint_row(&row, centre.x - s.w / 2.0, centre.y + (s.above - s.below) / 2.0, fs);
        }
        Cap::Icon(icon) => paint_icon(p, icon, centre, fs * 1.4, ink),
    }
}

fn face(style: Style, lit: bool, hover: bool) -> (Color32, Color32) {
    let lift = |c: Color32| if hover { c.gamma_multiply(1.3) } else { c };
    match style {
        Style::Num => (lift(KEY_NUM), Color32::WHITE),
        Style::Func => (lift(KEY_FUNC), Color32::WHITE),
        Style::Pad => (lift(KEY_PAD), Color32::WHITE),
        Style::Shift => (if lit { Color32::from_rgb(0xF0, 0xCF, 0x7A) } else { lift(KEY_SHIFT) }, Color32::from_rgb(0x2A, 0x24, 0x10)),
    }
}

// ---- Screens -------------------------------------------------------------------------------

fn status_bar(clip: &Painter, inside: Rect, dev: &Device, s: f32) {
    let f = FontId::proportional(9.5 * s);
    let y = inside.min.y + 6.0 * s;
    let mut x = inside.min.x + 2.0 * s;
    if dev.shift {
        clip.text(pos2(x, y), Align2::LEFT_CENTER, "S", f.clone(), LCD_INK);
    }
    x += 12.0 * s;
    let io = match dev.cfg.io {
        crate::calculator::Io::MathMath | crate::calculator::Io::MathDecimal => "Math",
        _ => "Line",
    };
    clip.text(pos2(x, y), Align2::LEFT_CENTER, io, f.clone(), LCD_INK);
    x += 30.0 * s;
    let unit = Rect::from_center_size(pos2(x + 6.0 * s, y), vec2(13.0 * s, 11.0 * s));
    clip.rect_filled(unit, CornerRadius::ZERO, LCD_INK);
    clip.text(unit.center(), Align2::CENTER_CENTER, dev.angle().letter(), f.clone(), LCD);
    x += 20.0 * s;
    match dev.cfg.num {
        crate::calculator::NumFmt::Fix(_) => {
            clip.text(pos2(x, y), Align2::LEFT_CENTER, "FIX", f.clone(), LCD_INK);
        }
        crate::calculator::NumFmt::Sci(_) => {
            clip.text(pos2(x, y), Align2::LEFT_CENTER, "SCI", f.clone(), LCD_INK);
        }
        _ => {}
    }
    if dev.app == App::Calculate && matches!(dev.calc.outcome, Outcome::Answer(_)) {
        let (cx, d) = (inside.max.x - 8.0 * s, 3.5 * s);
        clip.add(Shape::convex_polygon(vec![pos2(cx - d, y - d * 0.4), pos2(cx + d, y - d * 0.4), pos2(cx, y - d * 1.6)], LCD_INK, Stroke::NONE));
        clip.add(Shape::convex_polygon(vec![pos2(cx - d, y + d * 0.4), pos2(cx + d, y + d * 0.4), pos2(cx, y + d * 1.6)], LCD_INK, Stroke::NONE));
    }
}

fn draw_calculate(clip: &Painter, inside: Rect, calc: &Calc, s: f32, blink: bool) {
    let editing = !matches!(calc.outcome, Outcome::Answer(_) | Outcome::Error(_));
    let line_base = inside.min.y + 40.0 * s;
    let cursor = typeset(
        clip,
        &calc.tokens,
        inside.min.x + 2.0 * s,
        line_base,
        22.0 * s,
        LCD_INK,
        false,
        (editing || matches!(calc.outcome, Outcome::Error(_))).then_some(calc.cursor),
        Some(inside.width() - 14.0 * s),
    );
    if let Some((at, fs)) = cursor
        && blink
        && editing
    {
        clip.line_segment([pos2(at.x, at.y - fs * 0.85), pos2(at.x, at.y + fs * 0.28)], Stroke::new(1.6 * s, LCD_INK));
    }
    let answer_base = inside.max.y - 12.0 * s;
    match calc.outcome {
        Outcome::Answer(v) => {
            typeset(
                clip,
                &crate::calculator::result_tokens(v, calc.format, &calc.cfg),
                inside.max.x - 4.0 * s,
                answer_base,
                28.0 * s,
                LCD_INK,
                true,
                None,
                None,
            );
        }
        Outcome::Error(e) => {
            clip.text(pos2(inside.max.x - 4.0 * s, answer_base), Align2::RIGHT_BOTTOM, e.message(), FontId::proportional(19.0 * s), LCD_INK);
        }
        Outcome::Nothing => {}
    }
}

/// A pictogram for a home-screen app, in a box about 40 by 22.
fn pictogram(p: &Painter, i: usize, c: Pos2, s: f32, ink: Color32) {
    let st = Stroke::new(1.3 * s, ink);
    let f = |n: f32| FontId::proportional(n * s);
    match i {
        0 => {
            p.text(c, Align2::CENTER_CENTER, "\u{00D7}\u{00F7}\n+\u{2212}", f(8.5), ink);
        }
        1 => {
            for (k, h) in [10.0, 16.0, 22.0, 14.0, 8.0].iter().enumerate() {
                let x = c.x - 12.0 * s + k as f32 * 6.0 * s;
                p.rect_filled(Rect::from_min_max(pos2(x, c.y + 11.0 * s - h * s * 0.8), pos2(x + 4.0 * s, c.y + 11.0 * s)), CornerRadius::ZERO, ink);
            }
        }
        2 => {
            let r = Rect::from_center_size(c, vec2(26.0, 18.0) * s);
            p.rect_stroke(r, CornerRadius::ZERO, st, egui::StrokeKind::Inside);
            p.line_segment([r.center_top(), r.center_bottom()], st);
            p.line_segment([r.left_center(), r.right_center()], st);
        }
        3 => {
            p.text(c, Align2::CENTER_CENTER, "xy=0", f(11.0), ink);
        }
        4 => {
            p.text(c, Align2::CENTER_CENTER, "xy>0", f(11.0), ink);
        }
        5 => {
            let pts: Vec<Pos2> = (0..=16)
                .map(|k| {
                    let t = (k as f32 / 16.0 - 0.5) * 4.0;
                    pos2(c.x + t * 7.0 * s, c.y + 10.0 * s - (-t * t / 1.6).exp() * 20.0 * s)
                })
                .collect();
            p.add(Shape::line(pts, st));
        }
        6 => {
            p.text(c, Align2::CENTER_CENTER, "a+bi", f(11.0), ink);
        }
        7 => {
            p.text(c, Align2::CENTER_CENTER, "[ ]", f(15.0), ink);
        }
        8 => {
            p.text(c, Align2::CENTER_CENTER, "\u{2192}", f(18.0), ink);
        }
        9 => {
            let r = Rect::from_center_size(c, vec2(28.0, 18.0) * s);
            p.rect_stroke(r, CornerRadius::ZERO, st, egui::StrokeKind::Inside);
            p.line_segment([pos2(r.min.x + r.width() / 3.0, r.min.y), pos2(r.min.x + r.width() / 3.0, r.max.y)], st);
            p.line_segment([pos2(r.min.x + r.width() * 2.0 / 3.0, r.min.y), pos2(r.min.x + r.width() * 2.0 / 3.0, r.max.y)], st);
            p.line_segment([r.left_center(), r.right_center()], st);
        }
        10 => {
            p.text(c, Align2::CENTER_CENTER, "a:b", f(12.0), ink);
        }
        11 => {
            p.rect_stroke(Rect::from_center_size(c, vec2(16.0, 16.0) * s), CornerRadius::same(3), st, egui::StrokeKind::Inside);
        }
        _ => {
            p.text(c, Align2::CENTER_CENTER, "BASE", f(10.0), ink);
        }
    }
}

/// The home screen: six apps a page, three across.
fn draw_home(clip: &Painter, inside: Rect, dev: &Device, s: f32) {
    let body = Rect::from_min_max(inside.min + vec2(0.0, 14.0 * s), inside.max);
    let (cw, ch) = (body.width() / 3.0, body.height() / 2.0);
    let page = dev.home_sel / 6;
    for slot in 0..6 {
        let i = page * 6 + slot;
        let Some((name, app)) = HOME_APPS.get(i) else { break };
        let cell = Rect::from_min_size(body.min + vec2((slot % 3) as f32 * cw, (slot / 3) as f32 * ch), vec2(cw, ch)).shrink(2.0 * s);
        let on = dev.home_sel == i;
        let (bg, ink) = if on {
            (LCD_INK, LCD)
        } else if app.is_some() {
            (Color32::TRANSPARENT, LCD_INK)
        } else {
            (Color32::TRANSPARENT, LCD_DIM)
        };
        if on {
            clip.rect_filled(cell, CornerRadius::same((3.0 * s) as u8), bg);
        }
        pictogram(clip, i, cell.center() - vec2(0.0, 7.0 * s), s, ink);
        clip.text(cell.center_bottom() - vec2(0.0, 6.0 * s), Align2::CENTER_CENTER, name, FontId::proportional(10.5 * s), ink);
        let badge = Rect::from_center_size(cell.right_top() + vec2(-8.0 * s, 8.0 * s), vec2(11.0, 11.0) * s);
        clip.circle_filled(badge.center(), 5.5 * s, ink);
        clip.text(badge.center(), Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(7.5 * s), if on { LCD_INK } else { LCD });
    }
    // The scroll bar down the right: which page.
    let bar = Rect::from_min_max(pos2(inside.max.x - 3.0 * s, body.min.y), pos2(inside.max.x, body.max.y));
    let half = bar.height() / 2.0;
    clip.rect_filled(Rect::from_min_size(bar.min + vec2(0.0, page as f32 * half), vec2(bar.width(), half)), CornerRadius::ZERO, LCD_INK);
    if let Some(note) = dev.note {
        let r = Rect::from_center_size(inside.center(), vec2(inside.width() * 0.8, 24.0 * s));
        clip.rect_filled(r, CornerRadius::same((4.0 * s) as u8), LCD_INK);
        clip.text(r.center(), Align2::CENTER_CENTER, note, FontId::proportional(11.0 * s), LCD);
    }
}

fn draw_menu(clip: &Painter, inside: Rect, m: &Menu, dev: &Device, s: f32) {
    let body = Rect::from_min_max(inside.min + vec2(0.0, 14.0 * s), inside.max);
    let line_h = 22.0 * s;
    let visible = ((body.height() / line_h).floor() as usize).max(1);
    let first = m.sel.saturating_sub(visible - 1).min(m.items.len().saturating_sub(visible));
    let first = if m.sel < first { m.sel } else { first };
    for (n, (i, it)) in m.items.iter().enumerate().skip(first).take(visible).enumerate() {
        let r = Rect::from_min_size(body.min + vec2(0.0, n as f32 * line_h), vec2(body.width() - 6.0 * s, line_h));
        let on = i == m.sel;
        if on {
            clip.rect_filled(r, CornerRadius::ZERO, LCD_INK);
        }
        let ink = if on { LCD } else { LCD_INK };
        let mut x = r.min.x + 3.0 * s;
        if dev.option_numbers {
            clip.text(pos2(x, r.center().y), Align2::LEFT_CENTER, (i + 1).to_string(), FontId::proportional(12.0 * s), ink);
            x += 15.0 * s;
        }
        if let Some(sel) = it.radio {
            let c = pos2(x + 4.0 * s, r.center().y);
            clip.circle_stroke(c, 4.0 * s, Stroke::new(1.2 * s, ink));
            if sel {
                clip.circle_filled(c, 2.2 * s, ink);
            }
            x += 14.0 * s;
        }
        clip.text(pos2(x, r.center().y), Align2::LEFT_CENTER, &it.label, FontId::proportional(13.0 * s), ink);
        if it.more {
            let c = pos2(r.max.x - 7.0 * s, r.center().y);
            clip.add(Shape::convex_polygon(vec![c + vec2(-2.5, -4.0) * s, c + vec2(2.5, 0.0) * s, c + vec2(-2.5, 4.0) * s], ink, Stroke::NONE));
        }
    }
    if m.items.len() > visible {
        let bar = Rect::from_min_max(pos2(inside.max.x - 3.0 * s, body.min.y), pos2(inside.max.x, body.max.y));
        let h = bar.height() * visible as f32 / m.items.len() as f32;
        let y = bar.min.y + bar.height() * first as f32 / m.items.len() as f32;
        clip.rect_filled(Rect::from_min_size(pos2(bar.min.x, y), vec2(bar.width(), h)), CornerRadius::ZERO, LCD_INK);
    }
}

fn draw_variables(clip: &Painter, inside: Rect, sel: usize, dev: &Device, s: f32) {
    let body = Rect::from_min_max(inside.min + vec2(0.0, 14.0 * s), inside.max);
    let line_h = 22.0 * s;
    let visible = ((body.height() / line_h).floor() as usize).max(1);
    let first = sel.saturating_sub(visible - 1);
    for (n, i) in (first..9).take(visible).enumerate() {
        let r = Rect::from_min_size(body.min + vec2(0.0, n as f32 * line_h), vec2(body.width() - 6.0 * s, line_h));
        if i == sel {
            clip.rect_filled(r, CornerRadius::ZERO, LCD_INK);
        }
        let ink = if i == sel { LCD } else { LCD_INK };
        clip.text(
            pos2(r.min.x + 4.0 * s, r.center().y),
            Align2::LEFT_CENTER,
            format!("{}=", ["A", "B", "C", "D", "E", "F", "x", "y", "z"][i]),
            FontId::proportional(13.0 * s),
            ink,
        );
        let toks = crate::calculator::result_tokens(
            dev.mem.vars[i],
            crate::calculator::ResultFormat::Standard,
            &crate::calculator::Config { num: crate::calculator::NumFmt::Norm(1), ..dev.cfg },
        );
        typeset(clip, &toks, r.max.x - 4.0 * s, r.center().y + 5.0 * s, 15.0 * s, ink, true, None, None);
    }
}

fn draw_prompt(clip: &Painter, inside: Rect, p: &Prompt, s: f32, blink: bool) {
    clip.text(pos2(inside.min.x + 4.0 * s, inside.min.y + 28.0 * s), Align2::LEFT_BOTTOM, &p.label, FontId::proportional(14.0 * s), LCD_DIM);
    let cursor = typeset(
        clip,
        &p.field.tokens,
        inside.min.x + 4.0 * s,
        inside.min.y + 56.0 * s,
        22.0 * s,
        LCD_INK,
        false,
        Some(p.field.cursor),
        Some(inside.width() - 20.0 * s),
    );
    if let Some((at, fs)) = cursor
        && blink
    {
        clip.line_segment([pos2(at.x, at.y - fs * 0.85), pos2(at.x, at.y + fs * 0.28)], Stroke::new(1.6 * s, LCD_INK));
    }
    clip.text(
        pos2(inside.min.x + 4.0 * s, inside.max.y - 2.0 * s),
        Align2::LEFT_BOTTOM,
        "OK to save, back to cancel",
        FontId::proportional(9.5 * s),
        LCD_DIM,
    );
}

/// A short list with one line lit.
fn draw_list(clip: &Painter, inside: Rect, items: &[&str], sel: usize, s: f32) {
    for (i, name) in items.iter().enumerate() {
        let r = Rect::from_min_size(inside.min + vec2(0.0, 16.0 * s + i as f32 * 26.0 * s), vec2(inside.width(), 24.0 * s));
        let on = i == sel;
        if on {
            clip.rect_filled(r, CornerRadius::ZERO, LCD_INK);
        }
        clip.text(
            r.left_center() + vec2(5.0 * s, 0.0),
            Align2::LEFT_CENTER,
            format!("{}  {name}", i + 1),
            FontId::proportional(13.5 * s),
            if on { LCD } else { LCD_INK },
        );
    }
}

/// A form (a table's setting, an equation's coefficients) or, once solved, its answer lines.
fn draw_form(clip: &Painter, inside: Rect, f: &Form, s: f32, blink: bool) {
    let line_h = 27.0 * s;
    let visible = 3usize;
    let top = inside.min.y + 16.0 * s;
    if let Some(rows) = &f.rows {
        for (n, row) in rows.iter().skip(f.scroll).take(visible).enumerate() {
            let base = top + 20.0 * s + n as f32 * line_h;
            clip.text(pos2(inside.min.x + 6.0 * s, base), Align2::LEFT_BOTTOM, &row.label, FontId::proportional(15.0 * s), LCD_INK);
            if row.note.is_empty() {
                typeset(clip, &row.value, inside.min.x + 70.0 * s, base - 2.0 * s, 19.0 * s, LCD_INK, false, None, None);
            } else {
                clip.text(pos2(inside.min.x + 70.0 * s, base), Align2::LEFT_BOTTOM, row.note, FontId::proportional(13.0 * s), LCD_INK);
            }
        }
        if rows.len() > visible {
            clip.text(
                pos2(inside.max.x - 4.0 * s, inside.max.y - 2.0 * s),
                Align2::RIGHT_BOTTOM,
                format!("{}/{}", f.scroll + 1, rows.len()),
                FontId::proportional(9.5 * s),
                LCD_DIM,
            );
        }
        return;
    }
    let first = f.focus.saturating_sub(visible - 1).min(f.fields.len().saturating_sub(visible)).min(f.focus);
    for (n, i) in (first..f.fields.len().min(first + visible)).enumerate() {
        let base = top + 20.0 * s + n as f32 * line_h;
        let on = i == f.focus;
        if on {
            clip.rect_filled(
                Rect::from_min_max(pos2(inside.min.x, base - 20.0 * s), pos2(inside.max.x, base + 5.0 * s)),
                CornerRadius::ZERO,
                Color32::from_black_alpha(22),
            );
        }
        clip.text(
            pos2(inside.min.x + 6.0 * s, base),
            Align2::LEFT_BOTTOM,
            f.labels.get(i).copied().unwrap_or(""),
            FontId::proportional(13.0 * s),
            LCD_DIM,
        );
        if let Some(field) = f.fields.get(i) {
            let cursor = typeset(
                clip,
                &field.tokens,
                inside.min.x + 70.0 * s,
                base - 2.0 * s,
                19.0 * s,
                LCD_INK,
                false,
                on.then_some(field.cursor),
                Some(inside.width() - 90.0 * s),
            );
            if let Some((at, fs)) = cursor
                && blink
            {
                clip.line_segment([pos2(at.x, at.y - fs * 0.85), pos2(at.x, at.y + fs * 0.28)], Stroke::new(1.6 * s, LCD_INK));
            }
        }
    }
}

// ---- The calculator ------------------------------------------------------------------------

/// Draw the calculator at `scale` and apply what is pressed. Returns its full rectangle.
pub fn show(ui: &mut Ui, dev: &mut Device, scale: f32, keyboard: bool) -> Rect {
    let s = scale;
    let k = BASE_W / CASE_W * s;
    let (rect, _) = ui.allocate_exact_size(vec2(width(s), height(s)), Sense::hover());
    let p = ui.painter().clone();
    let at = |x: f32, y: f32| rect.min + vec2(x - CASE_X, y - CASE_Y) * k;
    p.rect_filled(rect, CornerRadius::same((46.0 * k) as u8), BODY);
    p.text(at(627.0, 268.0), Align2::LEFT_CENTER, "fx-991CW+", FontId::proportional(40.0 * k), PRINT.gamma_multiply(0.8));

    // The screen, with the screen's own scale (the screen is 694 photograph units across).
    let screen = Rect::from_min_max(at(618.0, 322.0), at(1312.0, 600.0));
    p.rect_filled(screen.expand(10.0 * k), CornerRadius::same((10.0 * k) as u8), Color32::from_rgb(0x12, 0x13, 0x15));
    p.rect_filled(screen, CornerRadius::same((6.0 * k) as u8), if dev.on { LCD } else { Color32::from_rgb(0x9B, 0xA3, 0x9A) });
    let ss = screen.width() / 303.0;
    let inside = screen.shrink(7.0 * ss);
    let clip = p.with_clip_rect(inside);
    let blink = ui.input(|i| (i.time * 1.6) as i64 % 2 == 0);
    if dev.on {
        status_bar(&clip, inside, dev, ss);
        match dev.stack.last() {
            Some(Overlay::Menu(m)) => draw_menu(&clip, inside, m, dev, ss),
            Some(Overlay::Variables { sel }) => draw_variables(&clip, inside, *sel, dev, ss),
            Some(Overlay::Prompt(pr)) => draw_prompt(&clip, inside, pr, ss, blink),
            None => match dev.app {
                App::Calculate => draw_calculate(&clip, inside, &dev.calc, ss, blink),
                App::Home => draw_home(&clip, inside, dev, ss),
                App::Table => draw_form(&clip, inside, &dev.table, ss, blink),
                App::Equation => match &dev.equation {
                    Some((_, f)) => draw_form(&clip, inside, f, ss, blink),
                    None => draw_list(&clip, inside, &EQUATIONS, dev.eq_sel, ss),
                },
            },
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(400));
    }

    // The d-pad's cross and the scroll key's slot sit behind their keys.
    let cross = Color32::from_rgb(0x08, 0x08, 0x0A);
    p.rect_filled(Rect::from_min_max(at(985.0, 740.0), at(1075.0, 1040.0)), CornerRadius::same((30.0 * k) as u8), cross);
    p.rect_filled(Rect::from_min_max(at(860.0, 838.0), at(1200.0, 922.0)), CornerRadius::same((40.0 * k) as u8), cross);
    p.rect_filled(
        Rect::from_min_max(at(1245.0, 722.0), at(1330.0, 918.0)),
        CornerRadius::same((40.0 * k) as u8),
        Color32::from_rgb(0x0F, 0x10, 0x12),
    );

    let mut pressed = None;
    for (n, def) in keys().iter().enumerate() {
        let c = at(def.at.0, def.at.1);
        let (w, h) = match def.shape {
            Shape2::Round(r) => (r * 2.0, r * 2.0),
            Shape2::Pill(w, h) => (w, h),
        };
        let kr = Rect::from_center_size(c, vec2(w, h) * k);
        let resp = ui.interact(kr, ui.id().with(("calc_key", n)), Sense::click());
        let name = key_name(def);
        resp.widget_info(move || egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name.clone()));
        let lit = def.key == Key::Shift && dev.shift;
        let (fill, ink) = face(def.style, lit, resp.hovered());
        let down = resp.is_pointer_button_down_on();
        let fr = if down { kr.translate(vec2(0.0, 1.5 * s)) } else { kr };
        let radius = (w.min(h) * 0.5 * k) as u8;
        p.rect_filled(fr, CornerRadius::same(radius), fill);
        if let Some(label) = def.label {
            p.text(
                c - vec2(0.0, h * 0.5 * k + 7.0 * k * 1.6),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(24.0 * k),
                if label == "SHIFT" { LEGEND } else { PRINT.gamma_multiply(0.85) },
            );
        }
        if let Some((cap, _)) = def.shift {
            let lc = c + vec2(-w * 0.62, -h * 0.72) * k;
            paint_cap(&p, cap, lc, 28.0 * k, LEGEND, 120.0 * k);
            let (tx, ty) = (lc.x + 6.0 * k, lc.y + 18.0 * k);
            p.add(Shape::line(
                vec![pos2(tx, ty), pos2(tx, ty + 14.0 * k), pos2(tx + 12.0 * k, ty + 14.0 * k)],
                Stroke::new(1.0, LEGEND.gamma_multiply(0.8)),
            ));
        }
        let fs = match def.cap {
            Cap::Text(t) if t.chars().count() > 2 => 30.0,
            Cap::Text(_) => 44.0,
            _ => 36.0,
        };
        let fs = if matches!(def.style, Style::Num) { fs * 1.15 } else { fs };
        paint_cap(&p, def.cap, fr.center(), fs * k, ink, w * k * 0.8);
        if resp.clicked() {
            pressed = Some((def.key, def.shift.map(|(_, key)| key)));
        }
    }
    if let Some((key, shifted)) = pressed {
        dev.tap(key, shifted);
    }
    if keyboard {
        type_keys(ui.ctx(), dev);
    }
    rect
}

/// Keys typed at the real keyboard while the pointer is over the calculator.
fn type_keys(ctx: &egui::Context, calc: &mut Device) {
    let events: Vec<Event> = ctx.input(|i| i.events.clone());
    for e in events {
        match e {
            Event::Text(t) => {
                for c in t.chars() {
                    let key = match c {
                        '0'..='9' => Key::Digit(c as u8 - b'0'),
                        '.' => Key::Dot,
                        '+' => Key::Plus,
                        '-' => Key::Minus,
                        '*' => Key::Times,
                        'x' => Key::X,
                        '/' => Key::Divide,
                        '(' => Key::LParen,
                        ')' => Key::RParen,
                        '^' => Key::Pow,
                        '!' => Key::Fact,
                        '%' => Key::Percent,
                        '=' => Key::Equals,
                        _ => continue,
                    };
                    calc.press(key);
                }
            }
            Event::Key { key, pressed: true, modifiers, .. } if modifiers.is_none() => {
                let k = match key {
                    EKey::Enter => Key::Equals,
                    EKey::Backspace | EKey::Delete => Key::Del,
                    EKey::ArrowLeft => Key::Left,
                    EKey::ArrowRight => Key::Right,
                    EKey::ArrowUp => Key::Up,
                    EKey::ArrowDown => Key::Down,
                    _ => continue,
                };
                calc.press(k);
            }
            _ => {}
        }
    }
}
