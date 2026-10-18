//! Maths teaching aids that sit on the board over a running lesson (Teacher Mode), the way they do
//! on an interactive whiteboard: no window frames, nothing to find in a title bar.
//!
//! * **Instruments** (ruler, protractor, compass) are drawn straight onto the board. Drag one to
//!   move it, drag the round knobs at its ends to turn it or resize it, or use two fingers (or the
//!   scroll wheel) on it. The protractor has an arm to read an angle with.
//! * **Cards** (number line, coordinate grid, fraction wall, algebra tiles, hundred square,
//!   calculator) are floating panels you drag by their slim top bar, with `−` / `+` to resize them
//!   and `×` to put them away.
//!
//! Nothing here is part of the presentation.

use std::collections::{BTreeMap, BTreeSet};

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, LayerId, Margin, Order, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use serde::{Deserialize, Serialize};

use crate::calculator_apps::Device;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MathsTool {
    Ruler,
    Protractor,
    Compass,
    NumberLine,
    CoordinateGrid,
    FractionWall,
    AlgebraTiles,
    HundredSquare,
    Calculator,
}

impl MathsTool {
    pub const ALL: [Self; 9] = [
        Self::Ruler,
        Self::Protractor,
        Self::Compass,
        Self::NumberLine,
        Self::CoordinateGrid,
        Self::FractionWall,
        Self::AlgebraTiles,
        Self::HundredSquare,
        Self::Calculator,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ruler => "Ruler",
            Self::Protractor => "Protractor",
            Self::Compass => "Compass",
            Self::NumberLine => "Number line",
            Self::CoordinateGrid => "Coordinate grid",
            Self::FractionWall => "Fraction wall",
            Self::AlgebraTiles => "Algebra tiles",
            Self::HundredSquare => "Hundred square",
            Self::Calculator => "Calculator",
        }
    }

    /// Drawn straight on the board and moved, turned and resized with handles; the rest are cards.
    pub fn is_instrument(self) -> bool {
        matches!(self, Self::Ruler | Self::Protractor | Self::Compass)
    }
}

/// Preferences for the current local teacher account; never part of a presentation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MathsToolsSettings {
    pub enabled: BTreeSet<MathsTool>,
}

impl Default for MathsToolsSettings {
    fn default() -> Self {
        Self { enabled: [MathsTool::Ruler, MathsTool::NumberLine].into_iter().collect() }
    }
}

impl MathsToolsSettings {
    pub fn is_enabled(&self, tool: MathsTool) -> bool {
        self.enabled.contains(&tool)
    }

    pub fn set_enabled(&mut self, tool: MathsTool, enabled: bool) {
        if enabled {
            self.enabled.insert(tool);
        } else {
            self.enabled.remove(&tool);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NumberLineRange {
    start: i32,
    end: i32,
    step: i32,
}

impl Default for NumberLineRange {
    fn default() -> Self {
        Self { start: -5, end: 5, step: 1 }
    }
}

const TILES_PER_ROW: usize = 3;
const TILE_W: f32 = 108.0;
const TILE_H: f32 = 52.0;

/// How long a centimetre is on the board, so the ruler and the compass agree.
pub const PX_PER_CM: f32 = 32.0;
const RULER_CM: f32 = 15.0;
const RULER_CM_MIN: f32 = 5.0;
const RULER_CM_MAX: f32 = 30.0;
const RULER_H: f32 = 68.0;
const RULER_PAD: f32 = 18.0;
const PROTRACTOR_R: f32 = 190.0;
const PROTRACTOR_LIP: f32 = 18.0;
const HANDLE_R: f32 = 15.0;
const HANDLE_HIT: f32 = 22.0;
const SCALE_MIN: f32 = 0.5;
const SCALE_MAX: f32 = 2.5;
const CARD_SCALE_MIN: f32 = 0.6;
const CARD_SCALE_MAX: f32 = 1.8;
/// Turning snaps to every 15° when this close (radians).
const SNAP_RANGE: f32 = 0.045;
const SNAP_STEP: f32 = std::f32::consts::PI / 12.0;

/// Where an open tool is and how it is turned and sized.
#[derive(Clone, Copy, Debug)]
struct Placement {
    centre: Pos2,
    /// Clockwise radians.
    angle: f32,
    scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handle {
    Close,
    Rotate,
    Scale,
    /// The ruler's length.
    Length,
    /// The protractor's reading arm.
    Arm,
    /// The compass's pencil end.
    Radius,
    Body,
}

#[derive(Clone, Copy, Debug)]
enum Grab {
    Move(Vec2),
    Rotate(f32),
    Scale {
        from: f32,
        start: f32,
    },
    Arm,
    Radius,
    Length,
    /// A press that already did its job (closing); ignore the rest of the gesture.
    Spent,
}

#[derive(Clone, Copy, Debug)]
struct Drag {
    tool: MathsTool,
    grab: Grab,
}

/// Local (unscaled, unturned) coordinates to the board.
#[derive(Clone, Copy)]
struct Xf {
    c: Pos2,
    angle: f32,
    s: f32,
}

impl Xf {
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        let (sn, cs) = self.angle.sin_cos();
        self.c + vec2((x * cs - y * sn) * self.s, (x * sn + y * cs) * self.s)
    }

    fn local(&self, p: Pos2) -> Vec2 {
        let d = p - self.c;
        let (sn, cs) = self.angle.sin_cos();
        vec2((d.x * cs + d.y * sn) / self.s, (-d.x * sn + d.y * cs) / self.s)
    }
}

pub struct MathsToolsPanel {
    /// The tool chooser is showing.
    pub open: bool,
    /// Where the chooser pops out, beside the toolbar button that opened it.
    pub anchor: Option<(Pos2, Align2)>,
    /// Open tools, the one touched last on top.
    open_tools: Vec<MathsTool>,
    placed: BTreeMap<MathsTool, Placement>,
    drag: Option<Drag>,
    /// The tool a two-finger gesture is moving.
    touching: Option<MathsTool>,
    number_line: NumberLineRange,
    /// The protractor arm, in degrees from its right-hand zero.
    protractor_arm: f32,
    /// The compass radius.
    compass_cm: f32,
    /// The ruler's length.
    ruler_cm: f32,
    coordinate_grid: CoordinateGridState,
    fraction_denominators: BTreeSet<usize>,
    algebra_tiles: Vec<AlgebraTile>,
    hundred_square: BTreeSet<u8>,
    calculator: Device,
    calculator_rect: Option<Rect>,
    /// Typing goes to the calculator: it is open and the pointer is over it.
    keys_to_calculator: bool,
    chooser_rect: Option<Rect>,
    /// Draw the instruments on this layer (the whiteboard's, so they sit on top of it).
    pub layer: Option<LayerId>,
}

impl Default for MathsToolsPanel {
    fn default() -> Self {
        Self {
            open: false,
            anchor: None,
            open_tools: Vec::new(),
            placed: BTreeMap::new(),
            drag: None,
            touching: None,
            number_line: NumberLineRange::default(),
            protractor_arm: 45.0,
            compass_cm: 3.0,
            ruler_cm: RULER_CM,
            coordinate_grid: CoordinateGridState::default(),
            fraction_denominators: [2, 3, 4, 5, 6, 8, 10, 12].into_iter().collect(),
            algebra_tiles: Vec::new(),
            hundred_square: BTreeSet::new(),
            calculator: Device::default(),
            calculator_rect: None,
            keys_to_calculator: false,
            chooser_rect: None,
            layer: None,
        }
    }
}

#[derive(Default)]
struct CoordinateGridState {
    x: i32,
    y: i32,
    points: Vec<(i32, i32)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AlgebraTile {
    X,
    NegativeX,
    Unit,
    NegativeUnit,
}

impl AlgebraTile {
    fn label(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::NegativeX => "-x",
            Self::Unit => "1",
            Self::NegativeUnit => "-1",
        }
    }

    fn colour(self) -> Color32 {
        match self {
            Self::X => Color32::from_rgb(79, 145, 212),
            Self::NegativeX => Color32::from_rgb(222, 112, 112),
            Self::Unit => Color32::from_rgb(102, 178, 126),
            Self::NegativeUnit => Color32::from_rgb(226, 173, 83),
        }
    }
}

// ---- Instruments: geometry -----------------------------------------------------------------

fn ruler_width(cm: f32) -> f32 {
    cm * PX_PER_CM + 2.0 * RULER_PAD
}

/// Where along the baseline (degrees from the right-hand zero) a point on the protractor is.
fn arm_angle(local: Vec2) -> f32 {
    if local.y >= 0.0 {
        return if local.x >= 0.0 { 0.0 } else { 180.0 };
    }
    (-local.y).atan2(local.x).to_degrees().clamp(0.0, 180.0)
}

fn snapped(angle: f32) -> f32 {
    let nearest = (angle / SNAP_STEP).round() * SNAP_STEP;
    if (angle - nearest).abs() < SNAP_RANGE { nearest } else { angle }
}

fn wrap(angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (angle + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}

impl MathsToolsPanel {
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Anything of ours is showing, so the wheel and clicks belong to the board tools.
    pub fn has_visible_widgets(&self) -> bool {
        self.open || !self.open_tools.is_empty()
    }

    /// Put an instrument at a chosen place, turn and size (for pictures and tests).
    #[cfg(test)]
    pub fn set_pose(&mut self, tool: MathsTool, centre: Pos2, angle: f32, scale: f32) {
        self.open_tool(tool);
        self.placed.insert(tool, Placement { centre, angle, scale });
    }

    /// Where an open tool's centre is, if it is open.
    pub fn centre_of(&self, tool: MathsTool) -> Option<Pos2> {
        self.is_open(tool).then(|| self.placed.get(&tool).map(|p| p.centre)).flatten()
    }

    /// The calculator's state, so a test (or a later feature) can drive it.
    pub fn calculator(&mut self) -> &mut Device {
        &mut self.calculator
    }

    /// Where the chooser is showing, if it is.
    pub fn chooser_rect(&self) -> Option<Rect> {
        self.chooser_rect
    }

    /// Typing goes to the calculator rather than the show.
    pub fn keyboard_captured(&self) -> bool {
        self.keys_to_calculator
    }

    fn is_open(&self, tool: MathsTool) -> bool {
        self.open_tools.contains(&tool)
    }

    /// Bring `tool` onto the board (as the chooser does).
    pub fn open_tool(&mut self, tool: MathsTool) {
        if !self.is_open(tool) {
            self.open_tools.push(tool);
        }
    }

    fn close_tool(&mut self, tool: MathsTool) {
        self.open_tools.retain(|t| *t != tool);
        if self.drag.is_some_and(|d| d.tool == tool) {
            self.drag = None;
        }
    }

    fn raise(&mut self, tool: MathsTool) {
        self.open_tools.retain(|t| *t != tool);
        self.open_tools.push(tool);
    }

    fn placement(&mut self, tool: MathsTool, canvas: Rect) -> Placement {
        let rank = self.open_tools.iter().position(|t| *t == tool).unwrap_or(0) as f32;
        let (w, h) = (canvas.width(), canvas.height());
        *self.placed.entry(tool).or_insert_with(|| {
            let centre = match tool {
                MathsTool::Ruler => canvas.center() + vec2(0.0, -h * 0.12),
                MathsTool::Protractor => canvas.center() + vec2(0.0, h * 0.18),
                MathsTool::Compass => canvas.center() + vec2(-w * 0.18, 0.0),
                _ => canvas.center() + vec2(rank * 56.0, rank * 44.0),
            };
            Placement { centre, angle: 0.0, scale: if tool == MathsTool::Calculator { 0.8 } else { 1.0 } }
        })
    }

    fn xf(&mut self, tool: MathsTool, canvas: Rect) -> Xf {
        let p = self.placement(tool, canvas);
        Xf { c: p.centre, angle: p.angle, s: if tool == MathsTool::Protractor { p.scale } else { 1.0 } }
    }

    /// The handles of an instrument, most important first.
    fn handles(&self, tool: MathsTool, xf: Xf) -> Vec<(Pos2, Handle)> {
        match tool {
            MathsTool::Ruler => {
                let w = ruler_width(self.ruler_cm);
                vec![
                    (xf.pt(w / 2.0 + 28.0, 0.0), Handle::Rotate),
                    (xf.pt(w / 2.0 - 6.0, RULER_H / 2.0 + 26.0), Handle::Length),
                    (xf.pt(-w / 2.0 - 28.0, 0.0), Handle::Close),
                ]
            }
            MathsTool::Protractor => {
                let tip = self.protractor_arm.to_radians();
                let r = PROTRACTOR_R - 8.0;
                vec![
                    (xf.pt(r * tip.cos(), -r * tip.sin()), Handle::Arm),
                    (xf.pt(PROTRACTOR_R + 30.0, 10.0), Handle::Rotate),
                    (xf.pt(-PROTRACTOR_R - 30.0, 10.0), Handle::Scale),
                    (xf.pt(0.0, PROTRACTOR_LIP + 32.0), Handle::Close),
                ]
            }
            MathsTool::Compass => {
                let r = self.compass_cm * PX_PER_CM;
                let a = xf.angle;
                vec![(xf.c + vec2(r * a.cos(), r * a.sin()), Handle::Radius), (xf.c + vec2(-30.0, -30.0), Handle::Close), (xf.c, Handle::Body)]
            }
            _ => vec![],
        }
    }

    fn on_body(&self, tool: MathsTool, xf: Xf, p: Pos2) -> bool {
        let l = xf.local(p);
        match tool {
            MathsTool::Ruler => l.x.abs() <= ruler_width(self.ruler_cm) / 2.0 && l.y.abs() <= RULER_H / 2.0,
            MathsTool::Protractor => (l.y <= 0.0 && l.length() <= PROTRACTOR_R) || (l.y > 0.0 && l.y <= PROTRACTOR_LIP && l.x.abs() <= PROTRACTOR_R),
            // Only the ring is solid: the middle is left free to write in.
            MathsTool::Compass => ((p - xf.c).length() - self.compass_cm * PX_PER_CM).abs() <= 10.0,
            _ => false,
        }
    }

    fn hit(&mut self, p: Pos2, canvas: Rect) -> Option<(MathsTool, Handle)> {
        for tool in self.open_tools.clone().into_iter().rev().filter(|t| t.is_instrument()) {
            let xf = self.xf(tool, canvas);
            for (at, kind) in self.handles(tool, xf) {
                if at.distance(p) <= HANDLE_HIT {
                    return Some((tool, kind));
                }
            }
            if self.on_body(tool, xf, p) {
                return Some((tool, Handle::Body));
            }
        }
        None
    }

    /// Whether `p` is on an instrument, so a click there is not the board's.
    pub fn hits(&self, p: Pos2) -> bool {
        self.open_tools.iter().filter(|t| t.is_instrument()).any(|tool| {
            let Some(pl) = self.placed.get(tool) else { return false };
            let xf = Xf { c: pl.centre, angle: pl.angle, s: if *tool == MathsTool::Protractor { pl.scale } else { 1.0 } };
            self.handles(*tool, xf).iter().any(|(at, _)| at.distance(p) <= HANDLE_HIT) || self.on_body(*tool, xf, p)
        })
    }

    fn begin(&mut self, tool: MathsTool, kind: Handle, p: Pos2, canvas: Rect) {
        self.raise(tool);
        let xf = self.xf(tool, canvas);
        let pl = self.placement(tool, canvas);
        let grab = match kind {
            Handle::Close => {
                self.close_tool(tool);
                Grab::Spent
            }
            Handle::Rotate => Grab::Rotate((p - pl.centre).angle() - pl.angle),
            Handle::Scale => Grab::Scale { from: p.distance(pl.centre).max(1.0), start: pl.scale },
            Handle::Arm => Grab::Arm,
            Handle::Radius => Grab::Radius,
            Handle::Length => Grab::Length,
            Handle::Body => Grab::Move(p - xf.c),
        };
        self.drag = Some(Drag { tool, grab });
    }

    fn update_drag(&mut self, drag: Drag, p: Pos2, canvas: Rect) {
        let xf = self.xf(drag.tool, canvas);
        match drag.grab {
            Grab::Move(off) => {
                let at = p - off;
                let centre = pos2(at.x.clamp(canvas.min.x, canvas.max.x), at.y.clamp(canvas.min.y, canvas.max.y));
                if let Some(pl) = self.placed.get_mut(&drag.tool) {
                    pl.centre = centre;
                }
            }
            Grab::Rotate(offset) => {
                if let Some(pl) = self.placed.get_mut(&drag.tool) {
                    pl.angle = snapped(wrap((p - pl.centre).angle() - offset));
                }
            }
            Grab::Scale { from, start } => {
                if let Some(pl) = self.placed.get_mut(&drag.tool) {
                    pl.scale = (start * p.distance(pl.centre) / from).clamp(SCALE_MIN, SCALE_MAX);
                }
            }
            Grab::Arm => self.protractor_arm = arm_angle(xf.local(p)).round(),
            Grab::Radius => {
                let v = p - xf.c;
                self.compass_cm = ((v.length() / PX_PER_CM) * 2.0).round() / 2.0;
                self.compass_cm = self.compass_cm.clamp(1.0, 15.0);
                if let Some(pl) = self.placed.get_mut(&drag.tool) {
                    pl.angle = snapped(v.angle());
                }
            }
            Grab::Length => {
                let half = xf.local(p).x.max(0.0);
                self.ruler_cm = ((2.0 * (half - RULER_PAD)) / PX_PER_CM).round().clamp(RULER_CM_MIN, RULER_CM_MAX);
            }
            Grab::Spent => {}
        }
    }

    /// Settle presses and drags on the instruments before anything else sees them. Returns whether
    /// the gesture is theirs, in which case the board must not also draw or turn the page.
    pub fn instrument_input(&mut self, ctx: &egui::Context, canvas: Rect) -> bool {
        if !self.open_tools.iter().any(|t| t.is_instrument()) {
            self.drag = None;
            self.touching = None;
            return false;
        }
        if let Some(mt) = ctx.multi_touch().filter(|m| (2..4).contains(&m.num_touches)) {
            let tool = self.touching.or_else(|| self.hit(mt.center_pos, canvas).map(|(t, _)| t));
            self.touching = tool;
            if let Some(tool) = tool {
                self.drag = None;
                let pl = self.placement(tool, canvas);
                if let Some(slot) = self.placed.get_mut(&tool) {
                    slot.centre = pl.centre + mt.translation_delta;
                    slot.angle = wrap(pl.angle + mt.rotation_delta);
                    if tool == MathsTool::Protractor {
                        slot.scale = (pl.scale * mt.zoom_delta).clamp(SCALE_MIN, SCALE_MAX);
                    }
                }
                return true;
            }
        } else {
            self.touching = None;
        }
        let (pressed, down, pos, scroll, command) = ctx.input(|i| {
            (i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.interact_pos(), i.smooth_scroll_delta.y, i.modifiers.command)
        });
        if self.drag.is_none()
            && pressed
            && let Some(p) = pos
            && let Some((tool, kind)) = self.hit(p, canvas)
        {
            self.begin(tool, kind, p, canvas);
        }
        if let Some(drag) = self.drag {
            if down {
                if let Some(p) = pos {
                    self.update_drag(drag, p, canvas);
                }
            } else {
                self.drag = None;
            }
            return true;
        }
        // The wheel turns an instrument under the pointer; with the command key it resizes the protractor.
        if scroll != 0.0
            && let Some(p) = ctx.input(|i| i.pointer.hover_pos())
            && let Some((tool, _)) = self.hit(p, canvas)
            && let Some(pl) = self.placed.get_mut(&tool)
        {
            if command && tool == MathsTool::Protractor {
                pl.scale = (pl.scale * (1.0 + scroll * 0.002)).clamp(SCALE_MIN, SCALE_MAX);
            } else {
                pl.angle = wrap(pl.angle + scroll * 0.004);
            }
        }
        false
    }

    // ---- Instruments: drawing --------------------------------------------------------------

    fn paint_instruments(&mut self, ctx: &egui::Context, canvas: Rect) {
        let hover = ctx.input(|i| i.pointer.hover_pos());
        // Below the ink and the toolbar, above the slide.
        let painter = ctx.layer_painter(self.layer.unwrap_or_else(|| LayerId::new(Order::Middle, Id::new("maths_instruments"))));
        for tool in self.open_tools.clone() {
            if !tool.is_instrument() {
                continue;
            }
            let xf = self.xf(tool, canvas);
            match tool {
                MathsTool::Ruler => paint_ruler(&painter, xf, self.ruler_cm),
                MathsTool::Protractor => self.paint_protractor(&painter, xf),
                MathsTool::Compass => self.paint_compass(&painter, xf),
                _ => {}
            }
            for (at, kind) in self.handles(tool, xf) {
                if kind != Handle::Body {
                    paint_handle(&painter, at, kind, hover.is_some_and(|h| h.distance(at) <= HANDLE_HIT));
                }
            }
        }
    }

    fn paint_protractor(&self, p: &Painter, xf: Xf) {
        let r = PROTRACTOR_R;
        let mut outline: Vec<Pos2> = (0..=60)
            .map(|i| {
                let t = std::f32::consts::PI * i as f32 / 60.0;
                xf.pt(r * t.cos(), -r * t.sin())
            })
            .collect();
        outline.extend([xf.pt(-r, PROTRACTOR_LIP), xf.pt(r, PROTRACTOR_LIP)]);
        let ink = Color32::from_rgb(26, 74, 110);
        p.add(Shape::convex_polygon(outline, Color32::from_rgba_unmultiplied(120, 200, 235, 96), Stroke::new(1.5, ink)));
        let hole: Vec<Pos2> = (0..=40)
            .map(|i| {
                let t = std::f32::consts::PI * i as f32 / 40.0;
                xf.pt(r * 0.3 * t.cos(), -r * 0.3 * t.sin())
            })
            .collect();
        p.add(Shape::line(hole, Stroke::new(1.0, ink.gamma_multiply(0.6))));
        p.line_segment([xf.pt(-r, 0.0), xf.pt(r, 0.0)], Stroke::new(1.5, ink));
        for deg in 0..=180 {
            let t = (deg as f32).to_radians();
            let len = if deg % 10 == 0 {
                13.0
            } else if deg % 5 == 0 {
                9.0
            } else {
                5.0
            };
            p.line_segment([xf.pt(r * t.cos(), -r * t.sin()), xf.pt((r - len) * t.cos(), -(r - len) * t.sin())], Stroke::new(1.0, ink));
            if deg % 10 == 0 {
                let outer = r - 26.0;
                xtext(p, xf, (outer * t.cos(), -outer * t.sin()), &deg.to_string(), 12.0, ink, t);
                let inner = r - 44.0;
                xtext(p, xf, (inner * t.cos(), -inner * t.sin()), &(180 - deg).to_string(), 11.0, ink.gamma_multiply(0.7), t);
            }
        }
        p.circle_filled(xf.pt(0.0, 0.0), 3.5, ink);
        // The arm and its reading.
        let t = self.protractor_arm.to_radians();
        let arm = Color32::from_rgb(214, 40, 48);
        p.line_segment([xf.pt(0.0, 0.0), xf.pt((r - 8.0) * t.cos(), -(r - 8.0) * t.sin())], Stroke::new(2.5, arm));
        let at = xf.pt(r * 0.58 * t.cos(), -r * 0.58 * t.sin());
        let label = format!("{:.0}\u{00B0}", self.protractor_arm);
        let g = p.layout_no_wrap(label, FontId::proportional(18.0 * xf.s.max(0.8)), arm);
        let pill = Rect::from_center_size(at, g.size() + vec2(12.0, 6.0));
        p.rect_filled(pill, CornerRadius::same(10), Color32::from_white_alpha(235));
        p.galley(pill.center() - g.size() / 2.0, g, arm);
    }

    fn paint_compass(&self, p: &Painter, xf: Xf) {
        let r = self.compass_cm * PX_PER_CM;
        let blue = Color32::from_rgb(33, 96, 190);
        let tip = xf.c + vec2(r * xf.angle.cos(), r * xf.angle.sin());
        p.circle_stroke(xf.c, r, Stroke::new(2.5, blue.gamma_multiply(0.85)));
        p.line_segment([xf.c, tip], Stroke::new(2.0, Color32::from_gray(70)));
        p.circle_filled(xf.c, 6.0, Color32::from_gray(50));
        p.circle_stroke(xf.c, 11.0, Stroke::new(2.0, Color32::from_gray(50)));
        let mid = xf.c + (tip - xf.c) * 0.5;
        let label = format!("r = {} cm", format_number(f64::from(self.compass_cm)));
        let g = p.layout_no_wrap(label, FontId::proportional(15.0), Color32::from_gray(30));
        let pill = Rect::from_center_size(mid + vec2(0.0, -16.0), g.size() + vec2(12.0, 6.0));
        p.rect_filled(pill, CornerRadius::same(10), Color32::from_white_alpha(235));
        p.galley(pill.center() - g.size() / 2.0, g, Color32::from_gray(30));
    }

    // ---- Cards and the chooser -------------------------------------------------------------

    pub fn ui(&mut self, ctx: &egui::Context, canvas: Rect, settings: &mut MathsToolsSettings) -> Vec<Rect> {
        let mut rects = Vec::new();
        self.open_tools.retain(|tool| settings.is_enabled(*tool));
        if self.drag.is_some_and(|d| !self.is_open(d.tool)) {
            self.drag = None;
        }
        self.paint_instruments(ctx, canvas);
        self.chooser_rect = if self.open { self.chooser(ctx, canvas, settings) } else { None };
        rects.extend(self.chooser_rect);
        self.keys_to_calculator = false;
        for tool in self.open_tools.clone().into_iter().filter(|t| !t.is_instrument()) {
            if let Some(r) = self.card(ctx, canvas, tool) {
                rects.push(r);
            }
        }
        if !self.is_open(MathsTool::Calculator) {
            self.calculator_rect = None;
        }
        rects
    }

    fn chooser(&mut self, ctx: &egui::Context, canvas: Rect, settings: &mut MathsToolsSettings) -> Option<Rect> {
        let (pos, pivot) = self.anchor.unwrap_or((canvas.min + vec2(88.0, canvas.height() * 0.25), Align2::LEFT_TOP));
        let mut close = false;
        let out =
            egui::Area::new(Id::new("maths_tools_panel")).order(Order::Foreground).fixed_pos(pos).pivot(pivot).constrain(true).show(ctx, |ui| {
                light_frame().show(ui, |ui| {
                    ui.visuals_mut().override_text_color = Some(Color32::from_rgb(0x14, 0x16, 0x1C));
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    let width = TILES_PER_ROW as f32 * TILE_W + (TILES_PER_ROW as f32 - 1.0) * 8.0;
                    let (head, _) = ui.allocate_exact_size(vec2(width, 26.0), Sense::hover());
                    ui.painter().text(
                        head.left_center(),
                        Align2::LEFT_CENTER,
                        "Maths tools",
                        FontId::proportional(15.0),
                        Color32::from_rgb(0x14, 0x16, 0x1C),
                    );
                    if chip(
                        ui,
                        Rect::from_center_size(head.right_center() - vec2(13.0, 0.0), vec2(26.0, 26.0)),
                        ChipIcon::Close,
                        "Close the maths tools",
                        false,
                    )
                    .clicked()
                    {
                        close = true;
                    }
                    let chosen: Vec<MathsTool> = MathsTool::ALL.into_iter().filter(|t| settings.is_enabled(*t)).collect();
                    if chosen.is_empty() {
                        ui.label("No tools chosen yet. Open \u{201C}Choose tools\u{201D} below.");
                    }
                    // Big tiles, three across: tap to bring a tool onto the board, tap again to put it away.
                    for row in chosen.chunks(TILES_PER_ROW) {
                        ui.horizontal(|ui| {
                            for tool in row {
                                let on = self.is_open(*tool);
                                let (fill, ink) = if on {
                                    (Color32::from_rgb(0x2B, 0x59, 0xFF), Color32::WHITE)
                                } else {
                                    (Color32::from_rgb(0xF1, 0xF3, 0xF7), Color32::from_rgb(0x14, 0x16, 0x1C))
                                };
                                let label = egui::RichText::new(tool.label()).size(15.0).strong().color(ink);
                                let tile = egui::Button::new(label)
                                    .min_size(vec2(TILE_W, TILE_H))
                                    .fill(fill)
                                    .corner_radius(crate::theme::radius::CARD)
                                    .wrap();
                                if ui.add(tile).clicked() {
                                    if on {
                                        self.close_tool(*tool);
                                    } else {
                                        self.open_tool(*tool);
                                    }
                                }
                            }
                        });
                    }
                    ui.add_space(2.0);
                    egui::CollapsingHeader::new("Choose tools").default_open(false).show(ui, |ui| {
                        for tool in MathsTool::ALL {
                            let mut enabled = settings.is_enabled(tool);
                            if ui.checkbox(&mut enabled, tool.label()).changed() {
                                settings.set_enabled(tool, enabled);
                                if !enabled {
                                    self.close_tool(tool);
                                }
                            }
                        }
                    });
                });
            });
        if close {
            self.open = false;
        }
        Some(out.response.rect)
    }

    /// One floating card: a slim bar to drag it by, `−` `+` `×` in the corner, and the tool inside.
    fn card(&mut self, ctx: &egui::Context, canvas: Rect, tool: MathsTool) -> Option<Rect> {
        let pl = self.placement(tool, canvas);
        let scale = pl.scale.clamp(CARD_SCALE_MIN, CARD_SCALE_MAX);
        let dark = tool == MathsTool::Calculator;
        let body_w = card_width(tool, scale);
        let (mut close, mut factor) = (false, 1.0f32);
        // The calculator opens at the right, clear of the toolbar; the rest open mid-board, one a little
        // down and across from the last.
        let default = if tool == MathsTool::Calculator {
            pos2((canvas.max.x - body_w - 48.0).max(canvas.min.x), canvas.min.y + 40.0)
        } else {
            pos2(
                (canvas.center().x - body_w / 2.0 + (pl.centre.x - canvas.center().x)).clamp(canvas.min.x, (canvas.max.x - body_w).max(canvas.min.x)),
                (canvas.min.y + canvas.height() * 0.18 + (pl.centre.y - canvas.center().y)).max(canvas.min.y),
            )
        };
        let id = Id::new(("maths_tool", tool));
        let mut keys = false;
        let out = egui::Area::new(id).order(Order::Foreground).default_pos(default).movable(true).constrain(true).show(ctx, |ui| {
            let frame = if dark { dark_frame() } else { light_frame() };
            frame.show(ui, |ui| {
                if !dark {
                    ui.visuals_mut().override_text_color = Some(Color32::from_rgb(0x14, 0x16, 0x1C));
                    ui.style_mut().visuals = egui::Visuals::light();
                }
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                let ink = if dark { Color32::from_gray(210) } else { Color32::from_gray(90) };
                let (head, _) = ui.allocate_exact_size(vec2(body_w, 26.0), Sense::hover());
                grip(ui.painter(), head.left_center() + vec2(10.0, 0.0), ink);
                ui.painter().text(head.left_center() + vec2(26.0, 0.0), Align2::LEFT_CENTER, tool.label(), FontId::proportional(13.0), ink);
                let at = |n: f32| Rect::from_center_size(head.right_center() - vec2(13.0 + 30.0 * n, 0.0), vec2(26.0, 26.0));
                if chip(ui, at(0.0), ChipIcon::Close, &format!("Put the {} away", tool.label().to_lowercase()), dark).clicked() {
                    close = true;
                }
                if chip(ui, at(1.0), ChipIcon::Plus, "Make it bigger", dark).clicked() {
                    factor = 1.15;
                }
                if chip(ui, at(2.0), ChipIcon::Minus, "Make it smaller", dark).clicked() {
                    factor = 1.0 / 1.15;
                }
                match tool {
                    MathsTool::NumberLine => draw_number_line(ui, &mut self.number_line, scale),
                    MathsTool::CoordinateGrid => draw_coordinate_grid(ui, &mut self.coordinate_grid, scale),
                    MathsTool::FractionWall => draw_fraction_wall(ui, &mut self.fraction_denominators, scale),
                    MathsTool::AlgebraTiles => draw_algebra_tiles(ui, &mut self.algebra_tiles, scale),
                    MathsTool::HundredSquare => draw_hundred_square(ui, &mut self.hundred_square, scale),
                    MathsTool::Calculator => {
                        let typing = ui.ctx().pointer_hover_pos().is_some_and(|p| self.calculator_rect.is_some_and(|r| r.contains(p)));
                        let r = crate::calculator_ui::show(ui, &mut self.calculator, scale, typing);
                        self.calculator_rect = Some(r);
                        keys = typing;
                    }
                    _ => {}
                }
            });
        });
        if tool == MathsTool::Calculator {
            self.keys_to_calculator = keys;
        }
        if out.response.clicked() || out.response.drag_started() {
            self.raise(tool);
        }
        if close {
            self.close_tool(tool);
            return None;
        }
        if factor != 1.0
            && let Some(p) = self.placed.get_mut(&tool)
        {
            p.scale = (scale * factor).clamp(CARD_SCALE_MIN, CARD_SCALE_MAX);
        }
        Some(out.response.rect)
    }
}

fn card_width(tool: MathsTool, s: f32) -> f32 {
    match tool {
        MathsTool::NumberLine => 400.0 * s,
        MathsTool::CoordinateGrid => 300.0 * s,
        MathsTool::FractionWall => 372.0 * s,
        MathsTool::AlgebraTiles => 320.0 * s,
        MathsTool::HundredSquare => 10.0 * 34.0 * s,
        MathsTool::Calculator => crate::calculator_ui::width(s),
        _ => 300.0,
    }
}

fn light_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(252, 252, 250, 248))
        .stroke(Stroke::new(1.0, Color32::from_black_alpha(46)))
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::same(10))
        .shadow(egui::Shadow { offset: [0, 6], blur: 20, spread: 0, color: Color32::from_black_alpha(70) })
}

fn dark_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(0x23, 0x26, 0x2D))
        .stroke(Stroke::new(1.0, Color32::from_white_alpha(30)))
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::same(8))
        .shadow(egui::Shadow { offset: [0, 6], blur: 20, spread: 0, color: Color32::from_black_alpha(90) })
}

fn grip(p: &Painter, at: Pos2, ink: Color32) {
    for dx in [-4.0, 0.0, 4.0] {
        for dy in [-3.0, 3.0] {
            p.circle_filled(at + vec2(dx, dy), 1.4, ink);
        }
    }
}

#[derive(Clone, Copy)]
enum ChipIcon {
    Close,
    Plus,
    Minus,
}

/// A small round button in a card's corner.
fn chip(ui: &mut Ui, rect: Rect, icon: ChipIcon, tip: &str, dark: bool) -> egui::Response {
    let resp = ui.interact(rect, ui.id().with(("chip", tip)), Sense::click());
    let (fill, ink) = match (dark, resp.hovered()) {
        (true, true) => (Color32::from_white_alpha(46), Color32::WHITE),
        (true, false) => (Color32::from_white_alpha(18), Color32::from_gray(220)),
        (false, true) => (Color32::from_black_alpha(34), Color32::from_gray(20)),
        (false, false) => (Color32::from_black_alpha(14), Color32::from_gray(60)),
    };
    let p = ui.painter();
    p.circle_filled(rect.center(), rect.width() / 2.0, fill);
    let (c, k, s) = (rect.center(), 5.0, Stroke::new(2.0, ink));
    match icon {
        ChipIcon::Close => {
            p.line_segment([c + vec2(-k, -k), c + vec2(k, k)], s);
            p.line_segment([c + vec2(-k, k), c + vec2(k, -k)], s);
        }
        ChipIcon::Plus => {
            p.line_segment([c + vec2(-k, 0.0), c + vec2(k, 0.0)], s);
            p.line_segment([c + vec2(0.0, -k), c + vec2(0.0, k)], s);
        }
        ChipIcon::Minus => {
            p.line_segment([c + vec2(-k, 0.0), c + vec2(k, 0.0)], s);
        }
    }
    let name = tip.to_string();
    resp.widget_info(move || egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name.clone()));
    resp.on_hover_text(tip)
}

/// A knob on the end of an instrument.
fn paint_handle(p: &Painter, at: Pos2, kind: Handle, hot: bool) {
    let fill = if kind == Handle::Arm || kind == Handle::Radius {
        Color32::from_rgb(214, 40, 48)
    } else {
        Color32::from_rgba_unmultiplied(30, 33, 41, if hot { 245 } else { 215 })
    };
    p.circle_filled(at, HANDLE_R, fill);
    p.circle_stroke(at, HANDLE_R, Stroke::new(if hot { 2.5 } else { 1.5 }, Color32::WHITE));
    let s = Stroke::new(2.0, Color32::WHITE);
    let k = 5.5;
    match kind {
        Handle::Close => {
            p.line_segment([at + vec2(-k, -k), at + vec2(k, k)], s);
            p.line_segment([at + vec2(-k, k), at + vec2(k, -k)], s);
        }
        Handle::Rotate => {
            let arc: Vec<Pos2> = (0..=18)
                .map(|i| {
                    let t = std::f32::consts::PI * (0.25 + 1.4 * i as f32 / 18.0);
                    at + vec2(t.cos(), t.sin()) * 6.5
                })
                .collect();
            let end = arc.last().copied().unwrap_or(at);
            p.add(Shape::line(arc, s));
            p.add(Shape::convex_polygon(vec![end + vec2(-4.0, -1.0), end + vec2(3.5, -3.0), end + vec2(1.0, 4.0)], Color32::WHITE, Stroke::NONE));
        }
        Handle::Scale | Handle::Length => {
            p.line_segment([at + vec2(-5.0, 5.0), at + vec2(5.0, -5.0)], s);
            p.line_segment([at + vec2(1.0, -5.0), at + vec2(5.0, -5.0)], s);
            p.line_segment([at + vec2(5.0, -5.0), at + vec2(5.0, -1.0)], s);
            p.line_segment([at + vec2(-1.0, 5.0), at + vec2(-5.0, 5.0)], s);
            p.line_segment([at + vec2(-5.0, 5.0), at + vec2(-5.0, 1.0)], s);
        }
        Handle::Arm | Handle::Radius | Handle::Body => {}
    }
}

/// Text on an instrument, turned with it.
fn xtext(p: &Painter, xf: Xf, at: (f32, f32), text: &str, size: f32, colour: Color32, t: f32) {
    let g = p.layout_no_wrap(text.to_string(), FontId::proportional(size * xf.s), colour);
    let centre = xf.pt(at.0, at.1);
    // `t` is the direction the numeral's "up" points (radians, anticlockwise from the right): upright
    // for `FRAC_PI_2`, pointing out from the middle for a numeral on a rim.
    let angle = xf.angle + (std::f32::consts::FRAC_PI_2 - t);
    p.add(Shape::Text(egui::epaint::TextShape::new(centre - g.size() / 2.0, g, colour).with_angle_and_anchor(angle, Align2::CENTER_CENTER)));
}

fn paint_ruler(p: &Painter, xf: Xf, cm: f32) {
    let (w, h) = (ruler_width(cm), RULER_H);
    let ink = Color32::from_rgb(70, 56, 20);
    let corners = [xf.pt(-w / 2.0, -h / 2.0), xf.pt(w / 2.0, -h / 2.0), xf.pt(w / 2.0, h / 2.0), xf.pt(-w / 2.0, h / 2.0)];
    p.add(Shape::convex_polygon(
        corners.to_vec(),
        Color32::from_rgba_unmultiplied(255, 241, 190, 238),
        Stroke::new(1.5, Color32::from_rgb(150, 125, 60)),
    ));
    let x0 = -cm * PX_PER_CM / 2.0;
    for mm in 0..=(cm as i32 * 10) {
        let x = x0 + mm as f32 * PX_PER_CM / 10.0;
        let len = if mm % 10 == 0 {
            24.0
        } else if mm % 5 == 0 {
            17.0
        } else {
            10.0
        };
        p.line_segment([xf.pt(x, -h / 2.0), xf.pt(x, -h / 2.0 + len)], Stroke::new(1.2, ink));
        if mm % 10 == 0 {
            xtext(p, xf, (x, -h / 2.0 + 37.0), &(mm / 10).to_string(), 14.0, ink, std::f32::consts::FRAC_PI_2);
        }
    }
    xtext(p, xf, (w / 2.0 - 26.0, h / 2.0 - 12.0), "cm", 13.0, ink.gamma_multiply(0.8), std::f32::consts::FRAC_PI_2);
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1.0e15 {
        format!("{value:.0}")
    } else {
        format!("{value:.8}").trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// ---- Cards: contents ----------------------------------------------------------------------

fn draw_coordinate_grid(ui: &mut Ui, state: &mut CoordinateGridState, s: f32) {
    ui.horizontal(|ui| {
        ui.label("Point (x, y)");
        ui.add(egui::DragValue::new(&mut state.x).range(-10..=10));
        ui.add(egui::DragValue::new(&mut state.y).range(-10..=10));
        if ui.button("Plot").clicked() {
            state.points.push((state.x, state.y));
        }
        if ui.button("Clear points").clicked() {
            state.points.clear();
        }
    });
    let (rect, resp) = ui.allocate_exact_size(vec2(300.0, 290.0) * s, Sense::click());
    let painter = ui.painter();
    let step = 14.0 * s;
    let centre = rect.center();
    painter.rect_filled(rect, CornerRadius::same(6), Color32::WHITE);
    for i in -10..=10 {
        let x = centre.x + i as f32 * step;
        let y = centre.y + i as f32 * step;
        let axis = i == 0;
        let color = if axis { Color32::from_gray(65) } else { Color32::from_gray(205) };
        let width = if axis { 1.8 } else { 0.7 };
        painter.line_segment([pos2(x, centre.y - 10.0 * step), pos2(x, centre.y + 10.0 * step)], Stroke::new(width, color));
        painter.line_segment([pos2(centre.x - 10.0 * step, y), pos2(centre.x + 10.0 * step, y)], Stroke::new(width, color));
        if i != 0 && i % 5 == 0 {
            let f = FontId::proportional(11.0 * s);
            painter.text(pos2(x, centre.y + 3.0), Align2::CENTER_TOP, i.to_string(), f.clone(), Color32::from_gray(60));
            painter.text(pos2(centre.x - 4.0, y), Align2::RIGHT_CENTER, (-i).to_string(), f, Color32::from_gray(60));
        }
    }
    // Tap a crossing to plot it, tap a point to take it away.
    if resp.clicked()
        && let Some(at) = resp.interact_pointer_pos()
    {
        let (gx, gy) = (((at.x - centre.x) / step).round() as i32, (-(at.y - centre.y) / step).round() as i32);
        if gx.abs() <= 10 && gy.abs() <= 10 {
            if let Some(i) = state.points.iter().position(|q| *q == (gx, gy)) {
                state.points.remove(i);
            } else {
                state.points.push((gx, gy));
            }
        }
    }
    for &(x, y) in &state.points {
        painter.circle_filled(centre + vec2(x as f32 * step, -(y as f32) * step), 4.5 * s.max(0.8), Color32::from_rgb(200, 55, 55));
    }
}

fn draw_fraction_wall(ui: &mut Ui, denominators: &mut BTreeSet<usize>, s: f32) {
    ui.label("Choose equal-part rows. Each coloured block is one unit fraction.");
    ui.horizontal_wrapped(|ui| {
        for denominator in 2..=12 {
            let mut selected = denominators.contains(&denominator);
            if ui.checkbox(&mut selected, denominator.to_string()).changed() {
                if selected {
                    denominators.insert(denominator);
                } else {
                    denominators.remove(&denominator);
                }
            }
        }
    });
    for &denominator in denominators.iter() {
        ui.horizontal(|ui| {
            ui.label(format!("1/{denominator}"));
            let (rect, _) = ui.allocate_exact_size(vec2(300.0 * s, 24.0 * s), Sense::hover());
            let width = rect.width() / denominator as f32;
            for part in 0..denominator {
                let cell = Rect::from_min_max(
                    pos2(rect.left() + part as f32 * width, rect.top()),
                    pos2(rect.left() + (part + 1) as f32 * width, rect.bottom()),
                );
                let color = if part % 2 == 0 { Color32::from_rgb(91, 157, 208) } else { Color32::from_rgb(135, 190, 224) };
                ui.painter().rect_filled(cell, CornerRadius::ZERO, color);
                ui.painter().rect_stroke(cell, CornerRadius::ZERO, Stroke::new(0.7, Color32::WHITE), StrokeKind::Inside);
            }
        });
    }
}

fn draw_algebra_tiles(ui: &mut Ui, tiles: &mut Vec<AlgebraTile>, s: f32) {
    ui.label("Add tiles, then click a tile to remove it.");
    ui.horizontal_wrapped(|ui| {
        for (tile, label) in
            [(AlgebraTile::X, "+ x"), (AlgebraTile::NegativeX, "\u{2212} x"), (AlgebraTile::Unit, "+ 1"), (AlgebraTile::NegativeUnit, "\u{2212} 1")]
        {
            if ui.button(label).clicked() {
                tiles.push(tile);
            }
        }
        if ui.button("Clear").clicked() {
            tiles.clear();
        }
    });
    ui.label(format!(
        "x tiles: {}, unit tiles: {}",
        tiles.iter().filter(|t| matches!(t, AlgebraTile::X | AlgebraTile::NegativeX)).count(),
        tiles.len()
    ));
    let mut remove = None;
    ui.horizontal_wrapped(|ui| {
        for (index, tile) in tiles.iter().enumerate() {
            let text = egui::RichText::new(tile.label()).color(Color32::BLACK).strong().size(14.0 * s);
            let long = matches!(tile, AlgebraTile::X | AlgebraTile::NegativeX);
            if ui.add(egui::Button::new(text).fill(tile.colour()).min_size(vec2(if long { 52.0 } else { 34.0 } * s, 36.0 * s))).clicked() {
                remove = Some(index);
                break;
            }
        }
    });
    if let Some(index) = remove {
        tiles.remove(index);
    }
}

fn draw_hundred_square(ui: &mut Ui, selected: &mut BTreeSet<u8>, s: f32) {
    ui.horizontal(|ui| {
        ui.label(format!("Selected: {} / 100", selected.len()));
        if ui.button("Clear").clicked() {
            selected.clear();
        }
    });
    egui::Grid::new("maths_hundred_square").spacing(vec2(2.0, 2.0)).show(ui, |ui| {
        for row in 0..10 {
            for col in 0..10 {
                let value = (row * 10 + col + 1) as u8;
                let color = if selected.contains(&value) { Color32::from_rgb(250, 205, 83) } else { Color32::from_rgb(228, 239, 249) };
                let text = egui::RichText::new(value.to_string()).size(13.0 * s).color(Color32::from_gray(20));
                if ui.add(egui::Button::new(text).fill(color).min_size(vec2(31.0 * s, 28.0 * s))).clicked() && !selected.insert(value) {
                    selected.remove(&value);
                }
            }
            ui.end_row();
        }
    });
}

fn draw_number_line(ui: &mut Ui, range: &mut NumberLineRange, s: f32) {
    ui.horizontal(|ui| {
        ui.label("From");
        ui.add(egui::DragValue::new(&mut range.start).range(-100..=99));
        ui.label("to");
        ui.add(egui::DragValue::new(&mut range.end).range(-99..=100));
        ui.label("step");
        ui.add(egui::DragValue::new(&mut range.step).range(1..=20));
    });
    if range.end <= range.start {
        range.end = range.start.saturating_add(1).min(100);
        range.start = range.start.min(range.end - 1);
    }

    let (rect, _) = ui.allocate_exact_size(vec2(400.0, 90.0) * s, Sense::hover());
    let painter = ui.painter();
    let y = rect.center().y - 8.0 * s;
    let (left, right) = (rect.left() + 18.0 * s, rect.right() - 18.0 * s);
    let line = Stroke::new(2.0, Color32::from_gray(50));
    painter.line_segment([pos2(left, y), pos2(right, y)], line);
    for (x, dir) in [(left, 1.0), (right, -1.0)] {
        painter.line_segment([pos2(x, y), pos2(x + 7.0 * dir * s, y - 5.0 * s)], line);
        painter.line_segment([pos2(x, y), pos2(x + 7.0 * dir * s, y + 5.0 * s)], line);
    }
    let span = (range.end - range.start).max(1) as f32;
    for value in range.start..=range.end {
        let x = left + (right - left) * (value - range.start) as f32 / span;
        let major = (value - range.start) % range.step == 0;
        let tick = if major { 15.0 } else { 7.0 } * s;
        painter.line_segment([pos2(x, y - tick / 2.0), pos2(x, y + tick / 2.0)], Stroke::new(1.5, Color32::from_gray(50)));
        if major {
            painter.text(pos2(x, y + 12.0 * s), Align2::CENTER_TOP, value.to_string(), FontId::proportional(12.0 * s), Color32::BLACK);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use egui_kittest::kittest::Queryable;

    struct State {
        panel: MathsToolsPanel,
        settings: MathsToolsSettings,
    }

    fn harness(panel: MathsToolsPanel, settings: MathsToolsSettings) -> egui_kittest::Harness<'static, State> {
        egui_kittest::Harness::builder().with_size(vec2(1200.0, 900.0)).build_ui_state(
            |ui, state: &mut State| {
                state.panel.ui(ui.ctx(), ui.max_rect(), &mut state.settings);
            },
            State { panel, settings },
        )
    }

    fn area_rect(h: &egui_kittest::Harness<'static, State>, tool: MathsTool) -> Option<Rect> {
        h.ctx.memory(|m| m.area_rect(Id::new(("maths_tool", tool))))
    }

    #[test]
    fn tool_toggles_change_enabled_state() {
        let mut settings = MathsToolsSettings::default();
        assert!(settings.is_enabled(MathsTool::Ruler));
        assert!(settings.is_enabled(MathsTool::NumberLine));
        assert!(!settings.is_enabled(MathsTool::Compass));

        settings.set_enabled(MathsTool::Compass, true);
        assert!(settings.is_enabled(MathsTool::Compass));
        settings.set_enabled(MathsTool::Ruler, false);
        assert!(!settings.is_enabled(MathsTool::Ruler));
    }

    #[test]
    fn toggling_a_tool_changes_its_panel_button() {
        let mut h = harness(MathsToolsPanel { open: true, ..Default::default() }, MathsToolsSettings::default());
        h.run_steps(2);
        assert_eq!(h.query_all_by_label("Compass").count(), 0, "not chosen, so no tile");
        h.get_by_label("Choose tools").click();
        h.run_steps(2);
        h.get_by_label("Compass").click();
        h.run_steps(2);
        assert!(h.state().settings.is_enabled(MathsTool::Compass));
        assert_eq!(h.query_all_by_label("Compass").count(), 2, "the checkbox and its new tile");
    }

    #[test]
    fn panel_offers_only_enabled_tools() {
        let settings = MathsToolsSettings { enabled: [MathsTool::Ruler].into_iter().collect() };
        let mut h = harness(MathsToolsPanel { open: true, ..Default::default() }, settings);
        h.run_steps(2);
        assert!(h.query_by_label("Ruler").is_some());
        assert!(h.query_by_label("Number line").is_none());
        assert!(h.query_by_label("Compass").is_none());
    }

    #[test]
    fn the_chooser_is_a_card_on_the_board_not_a_window() {
        let mut h = harness(MathsToolsPanel { open: true, ..Default::default() }, MathsToolsSettings::default());
        h.run_steps(2);
        assert!(h.query_by_label("Maths tools").is_none(), "there is no title bar; the heading is painted on the card");
        assert!(h.query_by_label("Close the maths tools").is_some());
        h.get_by_label("Close the maths tools").click();
        h.run_steps(2);
        assert!(!h.state().panel.open);
    }

    #[test]
    fn every_enabled_tool_opens_on_the_board() {
        let settings = MathsToolsSettings { enabled: MathsTool::ALL.into_iter().collect() };
        let mut h = harness(MathsToolsPanel { open: true, ..Default::default() }, settings);
        h.run_steps(2);
        for tool in MathsTool::ALL {
            h.get_by_label(tool.label()).click();
            h.run_steps(2);
        }
        for tool in MathsTool::ALL.into_iter().filter(|t| !t.is_instrument()) {
            assert!(area_rect(&h, tool).is_some(), "{} is showing", tool.label());
        }
        for label in ["Point (x, y)", "Selected: 0 / 100", "AC"] {
            assert!(h.query_by_label(label).is_some(), "{label}");
        }
        assert_eq!(h.state().panel.open_tools.len(), MathsTool::ALL.len());
    }

    #[test]
    fn a_card_has_no_window_chrome_and_its_chips_resize_and_close_it() {
        let settings = MathsToolsSettings { enabled: [MathsTool::NumberLine].into_iter().collect() };
        let mut h = harness(MathsToolsPanel { open: true, ..Default::default() }, settings);
        h.run_steps(2);
        h.get_by_label("Number line").click();
        h.run_steps(3);
        let before = area_rect(&h, MathsTool::NumberLine).unwrap();
        h.get_by_label("Make it bigger").click();
        h.run_steps(3);
        let bigger = area_rect(&h, MathsTool::NumberLine).unwrap();
        assert!(bigger.width() > before.width(), "{} then {}", before.width(), bigger.width());
        for _ in 0..2 {
            h.get_by_label("Make it smaller").click();
            h.run_steps(3);
        }
        assert!(area_rect(&h, MathsTool::NumberLine).unwrap().width() < before.width());
        h.get_by_label("Put the number line away").click();
        h.run_steps(3);
        assert!(!h.state().panel.is_open(MathsTool::NumberLine));
    }

    #[test]
    fn an_instrument_is_moved_by_its_body_and_ignores_the_empty_board() {
        let canvas = Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0));
        let mut panel = MathsToolsPanel::default();
        panel.open_tool(MathsTool::Ruler);
        let c = panel.xf(MathsTool::Ruler, canvas).c;
        assert!(panel.hits(c), "the middle of the ruler");
        assert!(!panel.hits(c + vec2(0.0, 200.0)), "clear board below it");
        assert!(!panel.hits(Pos2::ZERO));
        // It is solid across its length and no further.
        assert!(panel.hits(c + vec2(RULER_CM * PX_PER_CM / 2.0, 0.0)));
        assert!(!panel.hits(c + vec2(ruler_width(RULER_CM) / 2.0 + 60.0, 0.0)));
    }

    #[test]
    fn the_rotate_knob_turns_the_ruler_and_the_corners_stop_catching_clicks() {
        let canvas = Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0));
        let mut panel = MathsToolsPanel::default();
        panel.open_tool(MathsTool::Ruler);
        let xf = panel.xf(MathsTool::Ruler, canvas);
        let knob = panel.handles(MathsTool::Ruler, xf).into_iter().find(|(_, k)| *k == Handle::Rotate).unwrap().0;
        panel.begin(MathsTool::Ruler, Handle::Rotate, knob, canvas);
        // Drag the knob to straight below the centre: a quarter turn clockwise.
        let drag = panel.drag.unwrap();
        panel.update_drag(drag, xf.c + vec2(0.0, 300.0), canvas);
        let turned = panel.placement(MathsTool::Ruler, canvas).angle;
        assert!((turned - std::f32::consts::FRAC_PI_2).abs() < 0.01, "{turned}");
        // Turned upright, a point out along the old right-hand side is no longer on the ruler.
        panel.drag = None;
        assert!(!panel.hits(xf.c + vec2(200.0, 0.0)));
        assert!(panel.hits(xf.c + vec2(0.0, 200.0)));
    }

    #[test]
    fn the_ruler_stretches_to_the_length_dragged_to_within_its_limits() {
        let canvas = Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0));
        let mut panel = MathsToolsPanel::default();
        panel.open_tool(MathsTool::Ruler);
        let xf = panel.xf(MathsTool::Ruler, canvas);
        let knob = panel.handles(MathsTool::Ruler, xf).into_iter().find(|(_, k)| *k == Handle::Length).unwrap().0;
        panel.begin(MathsTool::Ruler, Handle::Length, knob, canvas);
        let d = panel.drag.unwrap();
        panel.update_drag(d, xf.c + vec2(10.0 * PX_PER_CM / 2.0 + RULER_PAD + 1.0, 0.0), canvas);
        assert_eq!(panel.ruler_cm, 10.0);
        panel.update_drag(d, xf.c + vec2(5000.0, 0.0), canvas);
        assert_eq!(panel.ruler_cm, RULER_CM_MAX);
        panel.update_drag(d, xf.c, canvas);
        assert_eq!(panel.ruler_cm, RULER_CM_MIN);
        assert!(panel.hits(xf.c + vec2(RULER_CM_MIN * PX_PER_CM / 2.0, 0.0)));
        assert!(!panel.hits(xf.c + vec2(RULER_CM * PX_PER_CM / 2.0 + 60.0, 0.0)), "a short ruler leaves the board clear beyond its end");
    }

    #[test]
    fn turning_snaps_to_the_quarter_turns_and_a_close_miss_does_not() {
        assert!((snapped(0.02)).abs() < 1e-6);
        assert!((snapped(std::f32::consts::FRAC_PI_2 - 0.03) - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!((snapped(0.12) - 0.12).abs() < 1e-6);
    }

    #[test]
    fn the_protractor_arm_reads_the_angle_where_it_is_dragged() {
        assert_eq!(arm_angle(vec2(10.0, 0.0)), 0.0);
        assert!((arm_angle(vec2(0.0, -10.0)) - 90.0).abs() < 1e-3);
        assert!((arm_angle(vec2(-10.0, -10.0)) - 135.0).abs() < 1e-3);
        assert_eq!(arm_angle(vec2(-10.0, 5.0)), 180.0, "below the base line it sticks to the nearer end");
    }

    #[test]
    fn the_compass_radius_is_whole_halves_of_a_centimetre() {
        let canvas = Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0));
        let mut panel = MathsToolsPanel::default();
        panel.open_tool(MathsTool::Compass);
        let c = panel.xf(MathsTool::Compass, canvas).c;
        panel.begin(MathsTool::Compass, Handle::Radius, c + vec2(96.0, 0.0), canvas);
        let d = panel.drag.unwrap();
        panel.update_drag(d, c + vec2(5.0 * PX_PER_CM + 3.0, 0.0), canvas);
        assert_eq!(panel.compass_cm, 5.0);
        panel.update_drag(d, c + vec2(1000.0 * PX_PER_CM, 0.0), canvas);
        assert_eq!(panel.compass_cm, 15.0, "never larger than the biggest radius it offers");
    }

    #[test]
    fn closing_an_instrument_from_its_handle_puts_it_away() {
        let canvas = Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0));
        let mut panel = MathsToolsPanel::default();
        panel.open_tool(MathsTool::Protractor);
        let xf = panel.xf(MathsTool::Protractor, canvas);
        let close = panel.handles(MathsTool::Protractor, xf).into_iter().find(|(_, k)| *k == Handle::Close).unwrap().0;
        let (tool, kind) = panel.hit(close, canvas).unwrap();
        assert_eq!((tool, kind), (MathsTool::Protractor, Handle::Close));
        panel.begin(tool, kind, close, canvas);
        assert!(!panel.is_open(MathsTool::Protractor));
        assert!(!panel.has_visible_widgets());
    }
}
