//! Teacher Mode: ephemeral annotation tools drawn over a running slide show (pen, highlighter,
//! shapes, eraser, revealer and countdown timers). Nothing here is ever written
//! to the presentation; it lives and dies with the [`crate::show::Show`] that owns it.

use egui::{Align2, Color32, CornerRadius, Id, LayerId, Margin, Order, Painter, Pos2, Rect, Response, Sense, Shape, Ui, pos2, vec2};

/// How far from the eraser centre ink is rubbed out.
pub const ERASER_RADIUS: f32 = 20.0;
/// Fingers down at once that count as a flat hand: a palm wipe, whatever tool is selected.
pub const PALM_TOUCHES: usize = 4;
/// How far from the middle of the hand ink is rubbed out.
pub const PALM_RADIUS: f32 = 70.0;
pub const HIGHLIGHT_WIDTH: f32 = 20.0;
/// The quick setting every new timer starts on.
const TIMER_DEFAULT_SECS: f64 = 300.0;
const TIMER_MAX_SECS: f64 = 23.0 * 3600.0 + 59.0 * 60.0 + 59.0;
/// One-tap durations offered while setting a timer, in minutes.
const TIMER_QUICK_MINS: [u32; 4] = [1, 2, 5, 10];
const REVEAL_GRAB: f32 = 24.0;
const PALETTE_MARGIN: f32 = 16.0;
/// The smallest a control may be: a fingertip on a board or a tablet, not a mouse pointer.
pub const TOUCH_TARGET: f32 = 44.0;

/// The presenting surfaces (palette, timer) use the design system's ink, a little see-through, so
/// they sit on any slide; selected controls use the accent, danger and success are the status colours.
const INK: Color32 = Color32::from_rgba_premultiplied(18, 20, 25, 235);
const INK_LIGHT: Color32 = Color32::from_rgba_premultiplied(48, 51, 62, 240);
/// A button face lifted off the ink.
const RAISED: Color32 = Color32::from_rgb(47, 51, 62);
const RAISED_HOVER: Color32 = Color32::from_rgb(64, 69, 84);
const ACCENT: Color32 = Color32::from_rgb(0x2B, 0x59, 0xFF);
const DANGER: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);
const DANGER_HOVER: Color32 = Color32::from_rgb(0xF2, 0x6B, 0x6F);
/// Darker than the mockup's green so white text on it stays above 4.5:1.
const SUCCESS: Color32 = Color32::from_rgb(0x11, 0x87, 0x5A);
const SURFACE_RADIUS: u8 = 28;

pub const PEN_COLOURS: [(&str, Color32); 5] = [
    ("Red", Color32::from_rgb(230, 40, 40)),
    ("Blue", Color32::from_rgb(30, 100, 230)),
    ("Green", Color32::from_rgb(30, 170, 70)),
    ("Black", Color32::BLACK),
    ("White", Color32::WHITE),
];
pub const WIDTHS: [(&str, f32); 3] = [("Thin", 4.0), ("Medium", 8.0), ("Thick", 14.0)];
pub const HIGHLIGHT_COLOURS: [(&str, [u8; 3]); 3] = [("Yellow", [255, 235, 0]), ("Cyan", [0, 220, 255]), ("Pink", [255, 80, 200])];
pub const HIGHLIGHT_ALPHA: u8 = 60;

pub fn highlight_colour([r, g, b]: [u8; 3]) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, HIGHLIGHT_ALPHA)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Pointer,
    Pen,
    Highlighter,
    Shape,
    Eraser,
    Revealer,
}

/// What the eraser takes away.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EraseMode {
    /// Only the ink under the eraser; the rest of the stroke stays.
    Pixel,
    /// Any stroke or shape the eraser touches, whole.
    Stroke,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShapeKind {
    Rect,
    Ellipse,
    Line,
    Arrow,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// What the palette asks the show to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Prev,
    Next,
    Exit,
}

/// A freehand stroke in screen points. `seq` orders marks so undo removes the newest one.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub points: Vec<Pos2>,
    pub colour: Color32,
    pub width: f32,
    pub is_highlight: bool,
    seq: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawnShape {
    pub kind: ShapeKind,
    pub from: Pos2,
    pub to: Pos2,
    pub colour: Color32,
    pub width: f32,
    seq: u64,
}

/// A shade anchored to one screen edge. `cover` is the fraction of the screen it hides (0 to 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Revealer {
    pub from: Edge,
    pub cover: f32,
}

impl Revealer {
    pub fn shade(&self, full: Rect) -> Rect {
        let c = self.cover.clamp(0.0, 1.0);
        match self.from {
            Edge::Top => Rect::from_min_max(full.min, pos2(full.max.x, full.min.y + full.height() * c)),
            Edge::Bottom => Rect::from_min_max(pos2(full.min.x, full.max.y - full.height() * c), full.max),
            Edge::Left => Rect::from_min_max(full.min, pos2(full.min.x + full.width() * c, full.max.y)),
            Edge::Right => Rect::from_min_max(pos2(full.max.x - full.width() * c, full.min.y), full.max),
        }
    }

    /// The draggable edge of the shade, as a segment.
    pub fn edge_line(&self, full: Rect) -> [Pos2; 2] {
        let s = self.shade(full);
        match self.from {
            Edge::Top => [pos2(s.min.x, s.max.y), s.max],
            Edge::Bottom => [s.min, pos2(s.max.x, s.min.y)],
            Edge::Left => [pos2(s.max.x, s.min.y), s.max],
            Edge::Right => [s.min, pos2(s.min.x, s.max.y)],
        }
    }

    /// Move the edge to the pointer.
    pub fn drag_to(&mut self, full: Rect, p: Pos2) {
        let c = match self.from {
            Edge::Top => (p.y - full.min.y) / full.height().max(1.0),
            Edge::Bottom => (full.max.y - p.y) / full.height().max(1.0),
            Edge::Left => (p.x - full.min.x) / full.width().max(1.0),
            Edge::Right => (full.max.x - p.x) / full.width().max(1.0),
        };
        self.cover = c.clamp(0.0, 1.0);
    }

    pub fn near_edge(&self, full: Rect, p: Pos2) -> bool {
        let [a, b] = self.edge_line(full);
        dist_to_segment(p, a, b) <= REVEAL_GRAB
    }
}

/// A countdown. `tick` reports reaching zero once.
#[derive(Clone, Debug, PartialEq)]
pub struct Timer {
    pub id: u64,
    pub duration: f64,
    pub remaining: f64,
    pub running: bool,
    pub expired: bool,
    /// Hours, minutes and seconds on the dials while the timer is being set; `None` once started.
    pub setup: Option<[u32; 3]>,
}

impl Timer {
    pub fn new(id: u64, duration: f64) -> Self {
        let duration = duration.clamp(1.0, TIMER_MAX_SECS);
        Timer { id, duration, remaining: duration, running: false, expired: false, setup: None }
    }

    /// Open the dials on the current duration.
    pub fn begin_setup(&mut self) {
        self.running = false;
        self.setup = Some(split_hms(self.duration));
    }

    /// Take the dialled time and start counting down. Does nothing at 00:00:00.
    pub fn start_from_setup(&mut self) {
        let Some([h, m, s]) = self.setup else { return };
        let secs = f64::from(h * 3600 + m * 60 + s);
        if secs < 1.0 {
            return;
        }
        self.set_duration(secs);
        self.setup = None;
        self.running = true;
    }

    /// Advance by `dt` seconds. `on_zero` runs once, on the tick that reaches zero.
    pub fn tick(&mut self, dt: f64, on_zero: impl FnOnce()) {
        if !self.running || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.remaining -= dt;
        if self.remaining <= 0.0 {
            self.remaining = 0.0;
            self.running = false;
            self.expired = true;
            on_zero();
        }
    }

    pub fn toggle(&mut self) {
        if self.expired {
            self.reset();
        }
        self.running = !self.running;
    }

    pub fn reset(&mut self) {
        self.remaining = self.duration;
        self.running = false;
        self.expired = false;
    }

    pub fn set_duration(&mut self, secs: f64) {
        self.duration = secs.clamp(1.0, TIMER_MAX_SECS);
        self.reset();
    }

    /// `MM:SS`, or `H:MM:SS` from an hour up; rounds up so a fresh 5:00 timer reads 05:00 until a
    /// full second has passed.
    pub fn label(&self) -> String {
        let s = self.remaining.max(0.0).ceil() as u64;
        if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{:02}:{:02}", s / 60, s % 60) }
    }
}

fn split_hms(secs: f64) -> [u32; 3] {
    let s = secs.clamp(0.0, TIMER_MAX_SECS) as u32;
    [s / 3600, s / 60 % 60, s % 60]
}

/// Step one field of the set-up time by `delta`, wrapping within its range (hours 0–23,
/// minutes and seconds 0–59) like the up and down arrows on a board timer.
pub fn nudge(set: &mut [u32; 3], field: usize, delta: i32) {
    let range = if field == 0 { 24 } else { 60 };
    if let Some(v) = set.get_mut(field) {
        *v = (*v as i32 + delta).rem_euclid(range) as u32;
    }
}

enum Draft {
    Stroke(Stroke),
    Shape(DrawnShape),
}

pub struct TeacherOverlay {
    pub tool: Tool,
    pub strokes: Vec<Stroke>,
    pub shapes: Vec<DrawnShape>,
    pub pen_colour: Color32,
    pub pen_width: f32,
    pub highlight_colour: Color32,
    pub shape_kind: ShapeKind,
    pub revealer: Option<Revealer>,
    /// The edge the next revealer is anchored to.
    pub reveal_from: Edge,
    pub erase_mode: EraseMode,
    pub timers: Vec<Timer>,
    /// Keep marks and the revealer when the slide changes (the Preferences setting).
    pub keep_on_advance: bool,
    next_id: u64,
    seq: u64,
    /// Where the middle of the hand was last frame while wiping, so a fast rub leaves no gaps.
    palm_last: Option<Pos2>,
    palm_now: Option<Pos2>,
    draft: Option<Draft>,
    revealer_drag: bool,
    edge: Edge,
    along: f32,
    palette_drag: Option<Pos2>,
    /// Whether the tool palette is out; the tab on the left edge toggles it.
    pub palette_open: bool,
    /// The maths tools chooser and its transient canvas widgets.
    pub maths_tools_panel: crate::maths_tools::MathsToolsPanel,
    /// The Maths subject pack is on: show its tool group (set every frame by `ui`).
    pub maths: bool,
    palette_rect: Option<Rect>,
    last_tick: Option<f64>,
    hit_rects: Vec<Rect>,
}

impl Default for TeacherOverlay {
    fn default() -> Self {
        TeacherOverlay {
            tool: Tool::Pointer,
            strokes: vec![],
            shapes: vec![],
            pen_colour: PEN_COLOURS[0].1,
            pen_width: WIDTHS[1].1,
            highlight_colour: highlight_colour(HIGHLIGHT_COLOURS[0].1),
            shape_kind: ShapeKind::Rect,
            revealer: None,
            reveal_from: Edge::Bottom,
            erase_mode: EraseMode::Pixel,
            timers: vec![],
            keep_on_advance: false,
            next_id: 0,
            seq: 0,
            palm_last: None,
            palm_now: None,
            draft: None,
            revealer_drag: false,
            edge: Edge::Left,
            along: 0.5,
            palette_drag: None,
            palette_open: false,
            maths_tools_panel: crate::maths_tools::MathsToolsPanel::default(),
            maths: false,
            palette_rect: None,
            last_tick: None,
            hit_rects: vec![],
        }
    }
}

impl TeacherOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    /// Remove every stroke and shape (not timers or the revealer).
    pub fn clear(&mut self) {
        self.strokes.clear();
        self.shapes.clear();
        self.draft = None;
    }

    /// Remove the newest stroke or shape.
    pub fn undo(&mut self) {
        let s = self.strokes.iter().map(|s| s.seq).max();
        let h = self.shapes.iter().map(|s| s.seq).max();
        match (s, h) {
            (Some(a), Some(b)) if b > a => {
                self.shapes.pop();
            }
            (Some(_), _) => {
                self.strokes.pop();
            }
            (None, Some(_)) => {
                self.shapes.pop();
            }
            (None, None) => {}
        }
    }

    /// The slide changed: marks go, and so does the revealer, unless the
    /// teacher chose to keep annotations. Timers always carry on.
    pub fn on_slide_change(&mut self) {
        if !self.keep_on_advance {
            self.clear();
            self.revealer = None;
        }
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.draft = None;
        self.revealer_drag = false;
        if tool == Tool::Revealer {
            if self.tool == Tool::Revealer {
                // Choosing the revealer again puts it away.
                self.revealer = None;
                self.tool = Tool::Pointer;
                return;
            }
            self.revealer.get_or_insert(Revealer { from: self.reveal_from, cover: 1.0 });
        }
        self.tool = tool;
    }

    pub fn set_reveal_from(&mut self, edge: Edge) {
        self.reveal_from = edge;
        if let Some(r) = self.revealer.as_mut() {
            r.from = edge;
        }
    }

    pub fn add_timer(&mut self) {
        self.next_id += 1;
        let id = self.next_id;
        let mut t = Timer::new(id, TIMER_DEFAULT_SECS);
        t.begin_setup();
        self.timers.push(t);
    }

    /// Advance the timers to `now`, calling `on_zero` for each that finishes.
    pub fn tick_timers(&mut self, now: f64, mut on_zero: impl FnMut()) {
        let dt = self.last_tick.map_or(0.0, |t| (now - t).max(0.0));
        self.last_tick = Some(now);
        for t in &mut self.timers {
            t.tick(dt, &mut on_zero);
        }
    }

    /// Whether the overlay takes the pointer, so the show must not also act on the click.
    pub fn wants_pointer(&self, at: Option<Pos2>) -> bool {
        self.tool != Tool::Pointer || at.is_some_and(|p| self.hit_rects.iter().any(|r| r.expand(6.0).contains(p)))
    }

    /// With a tool on, the wheel belongs to the overlay rather than turning the page.
    pub fn owns_wheel(&self) -> bool {
        self.tool != Tool::Pointer || self.maths_tools_panel.has_visible_widgets()
    }

    /// Rub out ink within `radius` of `p`. In pixel mode only the ink under the eraser goes and
    /// strokes are split around the gap; a touched shape becomes strokes first. In stroke mode
    /// whatever is touched goes whole.
    pub fn erase_with(&mut self, p: Pos2, radius: f32) {
        if self.erase_mode == EraseMode::Stroke {
            self.strokes.retain(|s| !near_polyline(&s.points, p, radius));
            self.shapes.retain(|s| !s.outlines().iter().any(|o| near_polyline(o, p, radius)));
            return;
        }
        let (touched, kept): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.shapes).into_iter().partition(|s| s.outlines().iter().any(|o| near_polyline(o, p, radius)));
        self.shapes = kept;
        for s in touched {
            for o in s.outlines() {
                self.strokes.push(Stroke { points: o, colour: s.colour, width: s.width, is_highlight: false, seq: s.seq });
            }
        }
        let mut out = Vec::with_capacity(self.strokes.len());
        for s in std::mem::take(&mut self.strokes) {
            if !near_polyline(&s.points, p, radius) {
                out.push(s);
                continue;
            }
            for points in split_outside(&s.points, p, radius) {
                out.push(Stroke { points, ..s.clone() });
            }
        }
        self.strokes = out;
    }

    /// Rub out under the eraser tool.
    pub fn erase_at(&mut self, p: Pos2) {
        self.erase_with(p, ERASER_RADIUS);
    }

    /// A flat hand on the screen: wipe along the path of its centre, whatever tool is on.
    /// `None` means the hand is up.
    pub fn palm_wipe(&mut self, centre: Option<Pos2>) {
        self.palm_now = centre;
        let Some(c) = centre else {
            self.palm_last = None;
            return;
        };
        let from = self.palm_last.unwrap_or(c);
        let steps = ((from.distance(c) / (PALM_RADIUS * 0.5)).ceil() as usize).max(1);
        for i in 1..=steps {
            self.erase_with(from.lerp(c, i as f32 / steps as f32), PALM_RADIUS);
        }
        self.palm_last = Some(c);
    }

    fn commit(&mut self, draft: Draft) {
        match draft {
            Draft::Stroke(s) => self.strokes.push(s),
            Draft::Shape(s) if s.from.distance(s.to) >= 3.0 => self.shapes.push(s),
            Draft::Shape(_) => {}
        }
    }

    fn new_stroke(&mut self, at: Pos2) -> Stroke {
        let seq = self.next_seq();
        let (colour, width, is_highlight) =
            if self.tool == Tool::Highlighter { (self.highlight_colour, HIGHLIGHT_WIDTH, true) } else { (self.pen_colour, self.pen_width, false) };
        Stroke { points: vec![at], colour, width, is_highlight, seq }
    }

    fn new_shape(&mut self, at: Pos2) -> DrawnShape {
        let seq = self.next_seq();
        DrawnShape { kind: self.shape_kind, from: at, to: at, colour: self.pen_colour, width: self.pen_width, seq }
    }

    /// One frame: timers, pointer tools, painting and the palette. Returns what the palette asked for.
    pub fn ui(&mut self, ui: &mut Ui, full: Rect, now: f64, settings: &mut crate::maths_tools::MathsToolsSettings, maths: bool) -> Vec<Action> {
        self.maths = maths;
        let ctx = ui.ctx().clone();
        let mut actions = vec![];
        let mut beep_now = false;
        self.tick_timers(now, || beep_now = true);
        if beep_now {
            beep();
        }
        let (hover, origin) = ctx.input(|i| (i.pointer.hover_pos(), i.pointer.press_origin()));
        let resp = ui.interact(full, Id::new("teacher_surface"), if self.tool == Tool::Pointer { Sense::hover() } else { Sense::click_and_drag() });
        let palm = ctx.multi_touch().filter(|m| m.num_touches >= PALM_TOUCHES).map(|m| m.center_pos);
        self.palm_wipe(palm);
        if palm.is_none() {
            self.pointer_tools(&resp, full, hover, origin);
        }
        if matches!(self.tool, Tool::Pen | Tool::Highlighter | Tool::Shape) {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        } else if self.tool == Tool::Eraser {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }
        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("teacher_overlay")));
        self.paint(&painter, full, hover);
        if let Some(c) = self.palm_now {
            painter.circle_stroke(c, PALM_RADIUS, egui::Stroke::new(2.0, Color32::WHITE));
        }
        self.hit_rects.clear();
        if self.palette_open {
            self.palette(&ctx, full, &mut actions);
        } else {
            self.palette_rect = None;
        }
        self.palette_tab(&ctx, full);
        let mut dead = vec![];
        for (i, t) in self.timers.iter_mut().enumerate() {
            let (remove, rect) = timer_widget(&ctx, t, i, full, now);
            self.hit_rects.push(rect);
            if remove {
                dead.push(t.id);
            }
        }
        self.timers.retain(|t| !dead.contains(&t.id));
        if self.maths {
            self.hit_rects.extend(self.maths_tools_panel.ui(&ctx, full, settings));
        }
        if self.timers.iter().any(|t| t.running || t.expired) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        actions
    }

    /// The small arrow on the left edge that puts the palette away or brings it out.
    fn palette_tab(&mut self, ctx: &egui::Context, full: Rect) {
        // Beside the palette when that is docked on the left, so the two never overlap.
        let x = match (self.palette_open, self.edge, self.palette_rect) {
            (true, Edge::Left, Some(r)) => r.max.x + 2.0,
            _ => full.min.x,
        };
        let out = egui::Area::new(Id::new("teacher_palette_tab"))
            .order(Order::Foreground)
            .fixed_pos(pos2(x, full.center().y))
            .pivot(Align2::LEFT_CENTER)
            .show(ctx, |ui| {
                let (r, resp) = ui.allocate_exact_size(vec2(34.0, 72.0), Sense::click());
                let fill = if resp.hovered() { INK_LIGHT } else { INK };
                ui.painter().rect_filled(r, CornerRadius { nw: 0, sw: 0, ne: 12, se: 12 }, fill);
                let (c, s) = (r.center(), 5.0);
                let dir = if self.palette_open { -1.0 } else { 1.0 };
                let pts = [pos2(c.x - s * dir * 0.6, c.y - s), pos2(c.x + s * dir * 0.6, c.y), pos2(c.x - s * dir * 0.6, c.y + s)];
                ui.painter().add(Shape::line(pts.to_vec(), egui::Stroke::new(2.5, Color32::WHITE)));
                resp.on_hover_text(if self.palette_open { "Hide the tools" } else { "Show the tools" })
            });
        self.hit_rects.push(out.response.rect);
        if out.inner.clicked() {
            self.palette_open = !self.palette_open;
        }
    }

    fn pointer_tools(&mut self, resp: &Response, full: Rect, hover: Option<Pos2>, origin: Option<Pos2>) {
        let dragging = resp.dragged();
        let at = resp.interact_pointer_pos().or(hover);
        match self.tool {
            Tool::Pointer => {}
            Tool::Pen | Tool::Highlighter => {
                if resp.drag_started() {
                    let s = self.new_stroke(origin.or(at).unwrap_or(Pos2::ZERO));
                    self.draft = Some(Draft::Stroke(s));
                }
                if dragging
                    && let (Some(Draft::Stroke(s)), Some(p)) = (self.draft.as_mut(), at)
                    && s.points.last().is_none_or(|l| l.distance(p) >= 1.0)
                {
                    s.points.push(p);
                }
                if resp.drag_stopped()
                    && let Some(d) = self.draft.take()
                {
                    self.commit(d);
                } else if resp.clicked()
                    && let Some(p) = at
                {
                    let mut s = self.new_stroke(p);
                    s.points.push(p);
                    self.strokes.push(s);
                }
            }
            Tool::Shape => {
                if resp.drag_started() {
                    let s = self.new_shape(origin.or(at).unwrap_or(Pos2::ZERO));
                    self.draft = Some(Draft::Shape(s));
                }
                if dragging && let (Some(Draft::Shape(s)), Some(p)) = (self.draft.as_mut(), at) {
                    s.to = p;
                }
                if resp.drag_stopped()
                    && let Some(d) = self.draft.take()
                {
                    self.commit(d);
                }
            }
            Tool::Eraser => {
                if (dragging || resp.is_pointer_button_down_on())
                    && let Some(p) = at
                {
                    self.erase_at(p);
                }
            }
            Tool::Revealer => {
                if let Some(r) = self.revealer.as_mut() {
                    if resp.drag_started() {
                        self.revealer_drag = origin.is_some_and(|p| r.near_edge(full, p));
                    }
                    if dragging
                        && self.revealer_drag
                        && let Some(p) = at
                    {
                        r.drag_to(full, p);
                    }
                    if resp.drag_stopped() {
                        self.revealer_drag = false;
                    }
                }
            }
        }
    }

    fn paint(&self, p: &Painter, full: Rect, hover: Option<Pos2>) {
        for s in &self.strokes {
            paint_stroke(p, s);
        }
        for s in &self.shapes {
            paint_shape(p, s);
        }
        match &self.draft {
            Some(Draft::Stroke(s)) => paint_stroke(p, s),
            Some(Draft::Shape(s)) => paint_shape(p, s),
            None => {}
        }
        if let Some(r) = self.revealer {
            let shade = r.shade(full);
            p.rect_filled(shade, CornerRadius::ZERO, Color32::from_rgba_unmultiplied(0, 0, 0, 200));
            let [a, b] = r.edge_line(full);
            // Keep the line on screen when the shade covers everything.
            let nudge = match r.from {
                Edge::Top => vec2(0.0, -1.0),
                Edge::Bottom => vec2(0.0, 1.0),
                Edge::Left => vec2(-1.0, 0.0),
                Edge::Right => vec2(1.0, 0.0),
            };
            let (a, b) = if full.contains(a) && full.contains(b) { (a, b) } else { (a + nudge, b + nudge) };
            p.line_segment([a, b], egui::Stroke::new(2.0, Color32::WHITE));
        }
        if self.tool == Tool::Eraser
            && let Some(h) = hover
        {
            p.circle_stroke(h, ERASER_RADIUS, egui::Stroke::new(2.0, Color32::WHITE));
            p.circle_stroke(h, ERASER_RADIUS + 1.0, egui::Stroke::new(1.0, Color32::from_black_alpha(160)));
        }
    }

    fn palette(&mut self, ctx: &egui::Context, full: Rect, actions: &mut Vec<Action>) {
        let horizontal = matches!(self.edge, Edge::Top | Edge::Bottom);
        let (pos, pivot) = match self.palette_drag {
            Some(p) => (p, Align2::CENTER_CENTER),
            None => palette_anchor(full, self.edge, self.along),
        };
        let out = egui::Area::new(Id::new("teacher_palette")).order(Order::Foreground).fixed_pos(pos).pivot(pivot).constrain(true).show(ctx, |ui| {
            egui::Frame::new().fill(INK).corner_radius(CornerRadius::same(SURFACE_RADIUS)).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                let has_options = matches!(self.tool, Tool::Pen | Tool::Highlighter | Tool::Shape | Tool::Revealer | Tool::Eraser);
                let mut body = |this: &mut Self, ui: &mut Ui| {
                    if has_options {
                        lay(ui, horizontal, |ui| this.options(ui));
                    }
                    lay(ui, horizontal, |ui| this.main_row(ui, full, horizontal, actions));
                };
                if horizontal {
                    ui.vertical_centered(|ui| body(self, ui));
                } else {
                    ui.horizontal(|ui| body(self, ui));
                }
            });
        });
        let rect = out.response.rect;
        self.hit_rects.push(rect);
        self.palette_rect = Some(rect);
    }

    fn options(&mut self, ui: &mut Ui) {
        let swatch = |ui: &mut Ui, colour: Color32, on: bool, tip: &str| {
            let (r, resp) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::click());
            let shown = Color32::from_rgb(colour.r(), colour.g(), colour.b());
            ui.painter().circle_filled(r.center(), 13.0, shown);
            if on || resp.hovered() {
                ui.painter().circle_stroke(r.center(), 16.0, egui::Stroke::new(2.5, Color32::WHITE));
            }
            resp.on_hover_text(tip)
        };
        match self.tool {
            Tool::Pen | Tool::Shape => {
                for (name, c) in PEN_COLOURS {
                    if swatch(ui, c, self.pen_colour == c, name).clicked() {
                        self.pen_colour = c;
                    }
                }
                for (name, w) in WIDTHS {
                    let (r, resp) = ui.allocate_exact_size(vec2(40.0, 36.0), Sense::click());
                    let col = if self.pen_width == w { Color32::WHITE } else { Color32::from_gray(150) };
                    ui.painter().line_segment(
                        [r.left_center() + vec2(4.0, 0.0), r.right_center() - vec2(4.0, 0.0)],
                        egui::Stroke::new(w.min(12.0) * 0.8, col),
                    );
                    if resp.on_hover_text(name).clicked() {
                        self.pen_width = w;
                    }
                }
                if self.tool == Tool::Shape {
                    for (kind, name) in
                        [(ShapeKind::Rect, "Rectangle"), (ShapeKind::Ellipse, "Circle"), (ShapeKind::Line, "Line"), (ShapeKind::Arrow, "Arrow")]
                    {
                        let on = self.shape_kind == kind;
                        let resp = icon_button(ui, on, name, |p, r, c| paint_shape_kind_icon(p, r, c, kind));
                        if resp.clicked() {
                            self.shape_kind = kind;
                        }
                    }
                }
            }
            Tool::Highlighter => {
                for (name, rgb) in HIGHLIGHT_COLOURS {
                    let c = highlight_colour(rgb);
                    if swatch(ui, c, self.highlight_colour == c, name).clicked() {
                        self.highlight_colour = c;
                    }
                }
            }
            Tool::Revealer => {
                for (edge, name) in [
                    (Edge::Bottom, "Shade from the bottom"),
                    (Edge::Top, "Shade from the top"),
                    (Edge::Left, "Shade from the left"),
                    (Edge::Right, "Shade from the right"),
                ] {
                    let on = self.reveal_from == edge;
                    if icon_button(ui, on, name, |p, r, c| paint_edge_icon(p, r, c, edge)).clicked() {
                        self.set_reveal_from(edge);
                    }
                }
            }
            Tool::Eraser => {
                if icon_button(ui, self.erase_mode == EraseMode::Pixel, "Pixel eraser: rubs out only what it touches", |p, r, c| {
                    p.circle_stroke(r.center(), r.width() * 0.3, egui::Stroke::new(2.0, c));
                })
                .clicked()
                {
                    self.erase_mode = EraseMode::Pixel;
                }
                if icon_button(ui, self.erase_mode == EraseMode::Stroke, "Stroke eraser: removes a whole mark at a touch", |p, r, c| {
                    p.line_segment([r.left_bottom(), r.right_top()], egui::Stroke::new(2.0, c));
                })
                .clicked()
                {
                    self.erase_mode = EraseMode::Stroke;
                }
            }
            _ => {}
        }
    }

    fn main_row(&mut self, ui: &mut Ui, full: Rect, horizontal: bool, actions: &mut Vec<Action>) {
        let grip = ui
            .allocate_response(if horizontal { vec2(24.0, TOUCH_TARGET) } else { vec2(TOUCH_TARGET, 24.0) }, Sense::drag())
            .on_hover_text("Drag to move the toolbar");
        for d in [-7.0, 0.0, 7.0] {
            let off = if horizontal { vec2(0.0, d) } else { vec2(d, 0.0) };
            ui.painter().circle_filled(grip.rect.center() + off, 2.0, Color32::from_gray(150));
        }
        if grip.dragged() {
            self.palette_drag = grip.interact_pointer_pos();
        }
        if grip.drag_stopped()
            && let Some(p) = self.palette_drag.take()
        {
            self.edge = nearest_edge(full, p);
            self.along = along_edge(full, self.edge, p);
        }
        let tool_btn = |this: &mut Self, ui: &mut Ui, tool: Tool, name: &str| {
            if icon_button(ui, this.tool == tool, name, |p, r, c| paint_tool_icon(p, r, c, tool)).clicked() {
                this.set_tool(tool);
            }
        };
        // Four groups, each a different job: look, mark up, rub out, and run the lesson.
        tool_btn(self, ui, Tool::Pointer, "Pointer");
        group_gap(ui, horizontal);
        tool_btn(self, ui, Tool::Pen, "Pen");
        tool_btn(self, ui, Tool::Highlighter, "Highlighter");
        tool_btn(self, ui, Tool::Shape, "Shapes");
        group_gap(ui, horizontal);
        tool_btn(self, ui, Tool::Eraser, "Eraser");
        if icon_button(ui, false, "Clear all marks", paint_clear_icon).clicked() {
            self.clear();
        }
        group_gap(ui, horizontal);
        tool_btn(self, ui, Tool::Revealer, "Revealer (click again to put it away)");
        if icon_button(ui, false, "Add a countdown timer", paint_timer_icon).clicked() {
            self.add_timer();
        }
        if icon_button(ui, self.maths_tools_panel.open, "Maths tools", paint_math_tools_icon).clicked() {
            self.maths_tools_panel.toggle();
        }
        if self.maths && icon_button(ui, self.maths_tools_panel.open, "Maths tools", paint_math_tools_icon).clicked() {
            self.maths_tools_panel.toggle();
        }
        group_gap(ui, horizontal);
        if icon_button(ui, false, "Previous", |p, r, c| paint_arrow_icon(p, r, c, false)).clicked() {
            actions.push(Action::Prev);
        }
        if icon_button(ui, false, "Next", |p, r, c| paint_arrow_icon(p, r, c, true)).clicked() {
            actions.push(Action::Next);
        }
        let (r, resp) = ui.allocate_exact_size(vec2(TOUCH_TARGET, TOUCH_TARGET), Sense::click());
        let fill = if resp.hovered() { DANGER_HOVER } else { DANGER };
        ui.painter().circle_filled(r.center(), 16.0, fill);
        let c = r.center();
        let s = egui::Stroke::new(2.2, Color32::WHITE);
        ui.painter().line_segment([c + vec2(-4.5, -4.5), c + vec2(4.5, 4.5)], s);
        ui.painter().line_segment([c + vec2(-4.5, 4.5), c + vec2(4.5, -4.5)], s);
        if resp.on_hover_text("Exit Teacher Mode (Esc)").clicked() {
            actions.push(Action::Exit);
        }
    }
}

/// A thin rule between groups of tools.
fn group_gap(ui: &mut Ui, horizontal: bool) {
    let (r, _) = ui.allocate_exact_size(if horizontal { vec2(9.0, TOUCH_TARGET) } else { vec2(TOUCH_TARGET, 9.0) }, Sense::hover());
    let s = egui::Stroke::new(1.0, Color32::from_white_alpha(60));
    if horizontal {
        ui.painter().line_segment([r.center_top() + vec2(0.0, 8.0), r.center_bottom() - vec2(0.0, 8.0)], s);
    } else {
        ui.painter().line_segment([r.left_center() + vec2(8.0, 0.0), r.right_center() - vec2(8.0, 0.0)], s);
    }
}

fn lay(ui: &mut Ui, horizontal: bool, add: impl FnOnce(&mut Ui)) {
    if horizontal {
        ui.horizontal(add);
    } else {
        ui.vertical(add);
    }
}

fn icon_button(ui: &mut Ui, selected: bool, tip: &str, paint: impl FnOnce(&Painter, Rect, Color32)) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(TOUCH_TARGET, TOUCH_TARGET), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(r, CornerRadius::same(12), ACCENT);
    } else if resp.hovered() {
        p.rect_filled(r, CornerRadius::same(12), Color32::from_white_alpha(28));
    }
    paint(p, r.shrink(11.0), Color32::WHITE);
    resp.on_hover_text(tip)
}

fn paint_tool_icon(p: &Painter, r: Rect, c: Color32, tool: Tool) {
    let s = egui::Stroke::new(2.0, c);
    let ctr = r.center();
    match tool {
        Tool::Pointer => {
            let pts = vec![
                r.left_top(),
                r.left_top() + vec2(0.0, r.height() * 0.85),
                ctr + vec2(-1.0, 2.0),
                ctr + vec2(3.0, 8.0),
                ctr + vec2(5.0, 7.0),
                ctr + vec2(1.5, 1.0),
                r.left_top() + vec2(r.width() * 0.7, r.height() * 0.58),
            ];
            p.add(Shape::convex_polygon(pts, c, egui::Stroke::NONE));
        }
        Tool::Pen => {
            p.line_segment([r.left_bottom(), r.right_top()], egui::Stroke::new(4.0, c));
            p.line_segment([r.left_bottom(), r.left_bottom() + vec2(3.0, -3.0)], egui::Stroke::new(4.0, Color32::from_rgb(255, 90, 90)));
        }
        Tool::Highlighter => {
            p.line_segment(
                [r.left_bottom() + vec2(0.0, -2.0), r.right_top() + vec2(0.0, 2.0)],
                egui::Stroke::new(8.0, Color32::from_rgba_unmultiplied(255, 230, 0, 200)),
            );
        }
        Tool::Shape => {
            let a = Rect::from_min_size(r.min, r.size() * 0.62);
            p.rect_stroke(a, CornerRadius::ZERO, s, egui::StrokeKind::Middle);
            p.circle_stroke(r.max - r.size() * 0.28, r.width() * 0.28, s);
        }
        Tool::Eraser => {
            let e = Rect::from_center_size(ctr, vec2(r.width(), r.height() * 0.6));
            p.rect_filled(e, CornerRadius::same(3), c);
            p.rect_filled(
                Rect::from_min_max(e.min, pos2(e.min.x + e.width() * 0.35, e.max.y)),
                CornerRadius::same(3),
                Color32::from_rgb(235, 130, 130),
            );
        }
        Tool::Revealer => {
            p.rect_stroke(r, CornerRadius::same(2), s, egui::StrokeKind::Middle);
            p.rect_filled(Rect::from_min_max(pos2(r.min.x, ctr.y), r.max), CornerRadius::same(2), c);
        }
    }
}

fn paint_clear_icon(p: &Painter, r: Rect, c: Color32) {
    let ctr = r.center();
    let s = egui::Stroke::new(2.0, c);
    p.circle_stroke(ctr, r.width() * 0.5, s);
    p.line_segment([ctr + vec2(-4.0, -4.0), ctr + vec2(4.0, 4.0)], s);
}

fn paint_timer_icon(p: &Painter, r: Rect, c: Color32) {
    let ctr = r.center();
    let s = egui::Stroke::new(2.0, c);
    p.circle_stroke(ctr, r.width() * 0.5, s);
    p.line_segment([ctr, ctr + vec2(0.0, -r.width() * 0.32)], s);
    p.line_segment([ctr, ctr + vec2(r.width() * 0.25, 0.0)], s);
}

fn paint_math_tools_icon(p: &Painter, r: Rect, c: Color32) {
    let s = egui::Stroke::new(1.8, c);
    let left = r.left() + 2.0;
    let right = r.right() - 2.0;
    let top = r.top() + 3.0;
    let base = r.bottom() - 3.0;
    p.line_segment([pos2(left, top), pos2(right, top)], s);
    p.line_segment([pos2(left, base), pos2(right, base)], s);
    for i in 0..=10 {
        let x = egui::lerp(left..=right, i as f32 / 10.0);
        let height = if i % 5 == 0 {
            7.0
        } else if i % 2 == 0 {
            4.0
        } else {
            2.0
        };
        p.line_segment([pos2(x, top), pos2(x, top + height)], s);
    }
}

fn paint_arrow_icon(p: &Painter, r: Rect, c: Color32, right: bool) {
    let (x0, x1) = if right { (r.min.x + 2.0, r.max.x - 2.0) } else { (r.max.x - 2.0, r.min.x + 2.0) };
    p.add(Shape::convex_polygon(vec![pos2(x0, r.min.y), pos2(x1, r.center().y), pos2(x0, r.max.y)], c, egui::Stroke::NONE));
}

fn paint_edge_icon(p: &Painter, r: Rect, c: Color32, edge: Edge) {
    p.rect_stroke(r, CornerRadius::same(2), egui::Stroke::new(1.5, c), egui::StrokeKind::Middle);
    let h = Revealer { from: edge, cover: 0.55 }.shade(r);
    p.rect_filled(h, CornerRadius::same(2), c);
}

fn paint_shape_kind_icon(p: &Painter, r: Rect, c: Color32, kind: ShapeKind) {
    let s = egui::Stroke::new(2.0, c);
    match kind {
        ShapeKind::Rect => {
            p.rect_stroke(r, CornerRadius::ZERO, s, egui::StrokeKind::Middle);
        }
        ShapeKind::Ellipse => {
            p.circle_stroke(r.center(), r.width() * 0.5, s);
        }
        ShapeKind::Line => {
            p.line_segment([r.left_bottom(), r.right_top()], s);
        }
        ShapeKind::Arrow => {
            let d = DrawnShape { kind, from: r.left_bottom(), to: r.right_top(), colour: c, width: 2.0, seq: 0 };
            paint_shape(p, &d);
        }
    }
}

fn paint_stroke(p: &Painter, s: &Stroke) {
    match s.points.as_slice() {
        [] => {}
        [only] => {
            p.circle_filled(*only, s.width * 0.5, s.colour);
        }
        [a, b] if a == b => {
            p.circle_filled(*a, s.width * 0.5, s.colour);
        }
        pts => {
            p.add(Shape::line(pts.to_vec(), egui::Stroke::new(s.width, s.colour)));
        }
    }
}

fn paint_shape(p: &Painter, s: &DrawnShape) {
    let stroke = egui::Stroke::new(s.width, s.colour);
    let r = Rect::from_two_pos(s.from, s.to);
    match s.kind {
        ShapeKind::Rect => {
            p.rect_stroke(r, CornerRadius::ZERO, stroke, egui::StrokeKind::Middle);
        }
        ShapeKind::Ellipse => {
            p.add(Shape::ellipse_stroke(r.center(), r.size() * 0.5, stroke));
        }
        ShapeKind::Line | ShapeKind::Arrow => {
            for o in s.outlines() {
                p.add(Shape::line(o, stroke));
            }
        }
    }
}

impl DrawnShape {
    /// The polylines the shape is drawn along (also what the eraser tests against).
    pub fn outlines(&self) -> Vec<Vec<Pos2>> {
        let r = Rect::from_two_pos(self.from, self.to);
        match self.kind {
            ShapeKind::Rect => vec![vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()]],
            ShapeKind::Ellipse => {
                let (c, h) = (r.center(), r.size() * 0.5);
                let n = 48;
                vec![
                    (0..=n)
                        .map(|i| {
                            let t = i as f32 / n as f32 * std::f32::consts::TAU;
                            c + vec2(h.x * t.cos(), h.y * t.sin())
                        })
                        .collect(),
                ]
            }
            ShapeKind::Line => vec![vec![self.from, self.to]],
            ShapeKind::Arrow => {
                let v = self.to - self.from;
                let len = v.length();
                if len < 1.0 {
                    return vec![vec![self.from, self.to]];
                }
                let dir = v / len;
                let head = (self.width * 3.0 + 8.0).min(len * 0.5);
                let wing = |a: f32| self.to - egui::emath::Rot2::from_angle(a) * dir * head;
                vec![vec![self.from, self.to], vec![wing(0.5), self.to, wing(-0.5)]]
            }
        }
    }
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let l2 = ab.length_sq();
    if l2 <= f32::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// The parts of a polyline outside the circle (centre `c`, radius `r`), each a separate run.
/// Long segments are subdivided so a gap is cut even when no sampled point fell inside.
fn split_outside(points: &[Pos2], c: Pos2, r: f32) -> Vec<Vec<Pos2>> {
    let mut dense: Vec<Pos2> = vec![];
    for (i, &a) in points.iter().enumerate() {
        dense.push(a);
        if let Some(&b) = points.get(i + 1) {
            let n = (a.distance(b) / 3.0).ceil() as usize;
            for k in 1..n {
                dense.push(a.lerp(b, k as f32 / n as f32));
            }
        }
    }
    let mut runs: Vec<Vec<Pos2>> = vec![];
    let mut cur: Vec<Pos2> = vec![];
    for q in dense {
        if q.distance(c) > r {
            cur.push(q);
        } else if !cur.is_empty() {
            runs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    runs
}

fn near_polyline(points: &[Pos2], p: Pos2, r: f32) -> bool {
    match points {
        [] => false,
        [only] => only.distance(p) <= r,
        _ => points.windows(2).any(|w| dist_to_segment(p, w[0], w[1]) <= r),
    }
}

fn nearest_edge(full: Rect, p: Pos2) -> Edge {
    let d = [(p.y - full.min.y, Edge::Top), (full.max.y - p.y, Edge::Bottom), (p.x - full.min.x, Edge::Left), (full.max.x - p.x, Edge::Right)];
    d.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)).map_or(Edge::Bottom, |e| e.1)
}

fn along_edge(full: Rect, edge: Edge, p: Pos2) -> f32 {
    let f = match edge {
        Edge::Top | Edge::Bottom => (p.x - full.min.x) / full.width().max(1.0),
        Edge::Left | Edge::Right => (p.y - full.min.y) / full.height().max(1.0),
    };
    f.clamp(0.08, 0.92)
}

fn palette_anchor(full: Rect, edge: Edge, along: f32) -> (Pos2, Align2) {
    let x = full.min.x + full.width() * along;
    let y = full.min.y + full.height() * along;
    match edge {
        Edge::Bottom => (pos2(x, full.max.y - PALETTE_MARGIN), Align2::CENTER_BOTTOM),
        Edge::Top => (pos2(x, full.min.y + PALETTE_MARGIN), Align2::CENTER_TOP),
        Edge::Left => (pos2(full.min.x + PALETTE_MARGIN, y), Align2::LEFT_CENTER),
        Edge::Right => (pos2(full.max.x - PALETTE_MARGIN, y), Align2::RIGHT_CENTER),
    }
}

/// A digit takes this fraction of the type size across, so the numbers never shift as they change.
const DIGIT_CELL: f32 = 0.62;
const COLON_CELL: f32 = 0.34;
/// With this long left the digits turn amber, and red from `TIMER_RED_SECS`.
const TIMER_AMBER_SECS: f64 = 30.0;
const TIMER_RED_SECS: f64 = 10.0;

fn cell_width(c: char, size: f32) -> f32 {
    if c == ':' { size * COLON_CELL } else { size * DIGIT_CELL }
}

fn run_width(text: &str, size: f32) -> f32 {
    text.chars().map(|c| cell_width(c, size)).sum()
}

/// Draw `text` with every character in a fixed-width cell, so `:` and `1` take their place and
/// the line cannot wrap or wobble.
fn paint_cells(p: &Painter, left_top: Pos2, text: &str, size: f32, colour: Color32) {
    let mut x = left_top.x;
    for c in text.chars() {
        let w = cell_width(c, size);
        p.text(pos2(x + w / 2.0, left_top.y + size / 2.0), Align2::CENTER_CENTER, c.to_string(), crate::theme::bold(size), colour);
        x += w;
    }
}

/// The digits' colour: white, then amber for the last half minute, then red for the last ten
/// seconds, so a class sees time running out without reading the numbers.
pub fn timer_ink(remaining: f64, running: bool) -> Color32 {
    if !running {
        Color32::WHITE
    } else if remaining <= TIMER_RED_SECS {
        Color32::from_rgb(255, 99, 99)
    } else if remaining <= TIMER_AMBER_SECS {
        Color32::from_rgb(255, 196, 64)
    } else {
        Color32::WHITE
    }
}

/// How big the running digits are for a screen of this height: readable from the back of a room.
pub fn timer_digit_size(screen_height: f32) -> f32 {
    (screen_height * 0.16).clamp(72.0, 200.0)
}

/// A drawn cross: the ✕ glyph is missing from the bundled fonts and shows as an empty box.
fn close_button(ui: &mut Ui) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(TOUCH_TARGET, TOUCH_TARGET), Sense::click());
    let fill = if resp.hovered() { RAISED_HOVER } else { RAISED };
    ui.painter().rect_filled(r, CornerRadius::same(8), fill);
    let (c, k) = (r.center(), 8.0);
    let s = egui::Stroke::new(2.5, Color32::WHITE);
    ui.painter().line_segment([c + vec2(-k, -k), c + vec2(k, k)], s);
    ui.painter().line_segment([c + vec2(-k, k), c + vec2(k, -k)], s);
    resp
}

fn timer_button(ui: &mut Ui, text: &str, fill: Color32, width: f32) -> Response {
    ui.add(egui::Button::new(egui::RichText::new(text).size(18.0).strong().color(Color32::WHITE)).fill(fill).min_size(vec2(width, TOUCH_TARGET)))
}

/// A floating timer. Returns whether it was dismissed, and where it is.
///
/// Setting: `HH:MM:SS` with an arrow above and below each pair, 1/2/5/10 minute shortcuts and a
/// big Start. Running: nearly only digits, as large as the screen allows; tap them to pause. The
/// buttons recede while it runs and come back when the pointer is over it or it is paused.
fn timer_widget(ctx: &egui::Context, t: &mut Timer, index: usize, full: Rect, now: f64) -> (bool, Rect) {
    let mut remove = false;
    let mut start = false;
    let flash = t.expired && (now * 2.0) as i64 % 2 == 0;
    let fill = if flash { Color32::from_rgb(214, 48, 52) } else { INK };
    let setting = t.setup.is_some();
    let size = if setting { 52.0 } else { timer_digit_size(full.height()) };
    let id = Id::new(("teacher_timer", t.id));
    let last_rect: Option<Rect> = ctx.data(|d| d.get_temp(id.with("rect")));
    let hovered = last_rect.zip(ctx.pointer_hover_pos()).is_some_and(|(r, p)| r.expand(8.0).contains(p));
    let digits_w = run_width("00:00:00", 52.0).max(run_width("00:00", size));
    let out = egui::Area::new(id)
        .order(Order::Foreground)
        .default_pos(pos2(full.center().x - digits_w / 2.0 - 24.0, full.min.y + 24.0 + index as f32 * 90.0))
        .movable(true)
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::new().fill(fill).corner_radius(CornerRadius::same(SURFACE_RADIUS)).inner_margin(Margin::symmetric(26, 16)).show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                if setting {
                    let mut set = t.setup.unwrap_or_default();
                    timer_dials(ui, &mut set, size);
                    ui.horizontal(|ui| {
                        for mins in TIMER_QUICK_MINS {
                            let on = set == [0, mins, 0];
                            let fill = if on { ACCENT } else { RAISED };
                            if timer_button(ui, &format!("{mins}"), fill, TOUCH_TARGET + 6.0).on_hover_text(format!("{mins} minute(s)")).clicked() {
                                set = [0, mins, 0];
                            }
                        }
                        ui.label(egui::RichText::new("min").size(16.0).color(Color32::from_gray(200)));
                    });
                    t.setup = Some(set);
                    ui.horizontal(|ui| {
                        let go = egui::Button::new(egui::RichText::new("Start").size(22.0).strong().color(Color32::WHITE))
                            .fill(SUCCESS)
                            .min_size(vec2(150.0, 52.0));
                        if ui.add_enabled(set != [0, 0, 0], go).clicked() {
                            start = true;
                        }
                        if close_button(ui).on_hover_text("Dismiss").clicked() {
                            remove = true;
                        }
                    });
                } else {
                    let text = t.label();
                    let (r, resp) = ui.allocate_exact_size(vec2(run_width(&text, size), size * 1.15), Sense::click());
                    paint_cells(ui.painter(), r.left_top(), &text, size, timer_ink(t.remaining, t.running));
                    if resp.on_hover_text("Tap to pause or resume").clicked() {
                        t.toggle();
                    }
                    if !t.running || hovered {
                        ui.horizontal(|ui| {
                            let (label, fill) = if t.running { ("Pause", RAISED) } else { ("Resume", SUCCESS) };
                            if timer_button(ui, label, fill, 110.0).clicked() {
                                t.toggle();
                            }
                            if timer_button(ui, "Reset", RAISED, 96.0).on_hover_text("Back to setting the time").clicked() {
                                t.reset();
                                t.begin_setup();
                            }
                            if close_button(ui).on_hover_text("Dismiss").clicked() {
                                remove = true;
                            }
                        });
                    }
                }
            });
        });
    ctx.data_mut(|d| d.insert_temp(id.with("rect"), out.response.rect));
    if start {
        t.start_from_setup();
    }
    (remove, out.response.rect)
}

/// `HH:MM:SS` for setting: each pair has a large up arrow above and down arrow below.
fn timer_dials(ui: &mut Ui, set: &mut [u32; 3], size: f32) {
    let pair_w = size * DIGIT_CELL * 2.0;
    let arrow = |ui: &mut Ui, up: bool| -> bool {
        let (r, resp) = ui.allocate_exact_size(vec2(pair_w, 34.0), Sense::click());
        let col = if resp.hovered() { Color32::WHITE } else { Color32::from_gray(190) };
        let (c, s) = (r.center(), 8.0);
        let d = if up { -1.0 } else { 1.0 };
        ui.painter().add(Shape::line(
            vec![pos2(c.x - s, c.y - s * 0.5 * d), pos2(c.x, c.y + s * 0.5 * d), pos2(c.x + s, c.y - s * 0.5 * d)],
            egui::Stroke::new(3.0, col),
        ));
        resp.clicked()
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for i in 0..3 {
            if i > 0 {
                let (r, _) = ui.allocate_exact_size(vec2(size * COLON_CELL, 34.0 + size * 1.15 + 34.0), Sense::hover());
                let y = r.min.y + 34.0;
                paint_cells(ui.painter(), pos2(r.min.x, y), ":", size, Color32::WHITE);
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if arrow(ui, true) {
                    nudge(set, i, 1);
                }
                let (r, _) = ui.allocate_exact_size(vec2(pair_w, size * 1.15), Sense::hover());
                paint_cells(ui.painter(), r.left_top(), &format!("{:02}", set.get(i).copied().unwrap_or(0)), size, Color32::WHITE);
                if arrow(ui, false) {
                    nudge(set, i, -1);
                }
            });
        }
    });
}

/// A short system sound when a timer ends. macOS only (no audio dependency); elsewhere the
/// timer's red flash is the signal.
fn beep() {
    #[cfg(target_os = "macos")]
    if let Ok(mut child) = std::process::Command::new("afplay").arg("/System/Library/Sounds/Glass.aiff").spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn stroke(o: &mut TeacherOverlay, pts: &[(f32, f32)]) {
        let mut s = o.new_stroke(pos2(pts[0].0, pts[0].1));
        s.points = pts.iter().map(|p| pos2(p.0, p.1)).collect();
        o.strokes.push(s);
    }

    fn shape(o: &mut TeacherOverlay, kind: ShapeKind, a: (f32, f32), b: (f32, f32)) {
        o.shape_kind = kind;
        let mut s = o.new_shape(pos2(a.0, a.1));
        s.to = pos2(b.0, b.1);
        o.shapes.push(s);
    }

    #[test]
    fn clear_removes_all_strokes_and_shapes() {
        let mut o = TeacherOverlay::new();
        stroke(&mut o, &[(0.0, 0.0), (10.0, 10.0)]);
        shape(&mut o, ShapeKind::Rect, (0.0, 0.0), (50.0, 50.0));
        o.add_timer();
        o.clear();
        assert!(o.strokes.is_empty() && o.shapes.is_empty());
        assert_eq!(o.timers.len(), 1, "clear leaves timers alone");
    }

    #[test]
    fn undo_removes_only_the_last_stroke() {
        let mut o = TeacherOverlay::new();
        stroke(&mut o, &[(0.0, 0.0), (1.0, 1.0)]);
        stroke(&mut o, &[(5.0, 5.0), (6.0, 6.0)]);
        stroke(&mut o, &[(9.0, 9.0), (9.5, 9.5)]);
        o.undo();
        assert_eq!(o.strokes.len(), 2);
        assert_eq!(o.strokes[1].points[0], pos2(5.0, 5.0));
        o.undo();
        o.undo();
        o.undo();
        assert!(o.strokes.is_empty(), "undo on nothing is harmless");
    }

    #[test]
    fn undo_takes_the_newest_of_strokes_and_shapes() {
        let mut o = TeacherOverlay::new();
        stroke(&mut o, &[(0.0, 0.0), (1.0, 1.0)]);
        shape(&mut o, ShapeKind::Line, (0.0, 0.0), (30.0, 30.0));
        o.undo();
        assert_eq!((o.strokes.len(), o.shapes.len()), (1, 0));
        shape(&mut o, ShapeKind::Line, (0.0, 0.0), (30.0, 30.0));
        stroke(&mut o, &[(2.0, 2.0), (3.0, 3.0)]);
        o.undo();
        assert_eq!((o.strokes.len(), o.shapes.len()), (1, 1));
    }

    #[test]
    fn timer_counts_down_and_fires_once_at_zero() {
        let mut t = Timer::new(1, 3.0);
        let fired = Cell::new(0);
        t.tick(1.0, || fired.set(fired.get() + 1));
        assert_eq!(t.label(), "00:03", "a stopped timer does not run");
        t.toggle();
        t.tick(1.0, || fired.set(fired.get() + 1));
        assert_eq!(t.label(), "00:02");
        t.tick(2.5, || fired.set(fired.get() + 1));
        assert_eq!((t.remaining, t.expired, t.running, fired.get()), (0.0, true, false, 1));
        t.tick(5.0, || fired.set(fired.get() + 1));
        assert_eq!(fired.get(), 1, "fires once");
        t.toggle();
        assert_eq!((t.remaining, t.expired, t.running), (3.0, false, true), "starting an expired timer restarts it");
    }

    #[test]
    fn overlay_ticks_every_timer_from_the_clock() {
        let mut o = TeacherOverlay::new();
        o.add_timer();
        o.add_timer();
        o.timers[0].set_duration(2.0);
        o.timers[0].toggle();
        let n = Cell::new(0);
        o.tick_timers(10.0, || n.set(n.get() + 1));
        o.tick_timers(12.5, || n.set(n.get() + 1));
        assert_eq!(n.get(), 1);
        assert!(o.timers[0].expired && !o.timers[1].expired);
    }

    #[test]
    fn a_new_timer_opens_its_dials_on_the_five_minute_quick_setting() {
        let mut o = TeacherOverlay::new();
        o.add_timer();
        assert_eq!(o.timers[0].setup, Some([0, 5, 0]));
        assert!(!o.timers[0].running);
    }

    #[test]
    fn starting_from_the_dials_sets_the_time_and_runs() {
        let mut t = Timer::new(1, 300.0);
        t.setup = Some([1, 2, 3]);
        t.start_from_setup();
        assert_eq!((t.duration, t.running, t.setup), (3723.0, true, None));
        assert_eq!(t.label(), "1:02:03");
    }

    #[test]
    fn dials_at_zero_do_not_start() {
        let mut t = Timer::new(1, 300.0);
        t.setup = Some([0, 0, 0]);
        t.start_from_setup();
        assert!(!t.running && t.setup.is_some());
    }

    #[test]
    fn clicking_the_time_reopens_the_dials_on_the_current_duration() {
        let mut t = Timer::new(1, 90.0);
        t.begin_setup();
        assert_eq!(t.setup, Some([0, 1, 30]));
    }

    #[test]
    fn the_arrows_step_a_field_and_wrap_without_carrying() {
        let mut set = [0, 59, 0];
        nudge(&mut set, 1, 1);
        assert_eq!(set, [0, 0, 0], "59 minutes up is 00, hours untouched");
        nudge(&mut set, 2, -1);
        assert_eq!(set, [0, 0, 59]);
        nudge(&mut set, 0, -1);
        assert_eq!(set, [23, 0, 59]);
        nudge(&mut set, 0, 1);
        assert_eq!(set, [0, 0, 59]);
    }

    #[test]
    fn reset_goes_back_to_the_dials_on_the_full_time() {
        let mut t = Timer::new(1, 120.0);
        t.setup = None;
        t.running = true;
        t.remaining = 30.0;
        t.reset();
        t.begin_setup();
        assert_eq!((t.running, t.remaining, t.setup), (false, 120.0, Some([0, 2, 0])));
    }

    #[test]
    fn the_palette_starts_put_away() {
        assert!(!TeacherOverlay::new().palette_open);
    }

    #[test]
    fn eraser_removes_what_it_touches_and_keeps_the_rest() {
        let mut o = TeacherOverlay::new();
        o.erase_mode = EraseMode::Stroke;
        stroke(&mut o, &[(100.0, 100.0), (200.0, 100.0)]);
        stroke(&mut o, &[(100.0, 400.0), (200.0, 400.0)]);
        shape(&mut o, ShapeKind::Rect, (500.0, 500.0), (600.0, 600.0));
        o.erase_at(pos2(150.0, 115.0));
        assert_eq!(o.strokes.len(), 1);
        assert_eq!(o.strokes[0].points[0].y, 400.0);
        o.erase_at(pos2(550.0, 500.0));
        assert!(o.shapes.is_empty());
        o.erase_at(pos2(150.0, 430.0));
        assert!(o.strokes.len() == 1, "more than 20 points away is safe");
    }

    #[test]
    fn the_eraser_defaults_to_pixel_mode() {
        assert_eq!(TeacherOverlay::new().erase_mode, EraseMode::Pixel);
    }

    #[test]
    fn pixel_eraser_cuts_a_gap_and_leaves_the_rest_of_the_stroke() {
        let mut o = TeacherOverlay::new();
        stroke(&mut o, &[(0.0, 100.0), (300.0, 100.0)]);
        o.erase_at(pos2(150.0, 100.0));
        assert_eq!(o.strokes.len(), 2, "split in two, even though no sampled point was under the eraser");
        let (a, b) = (&o.strokes[0].points, &o.strokes[1].points);
        assert!(a.iter().all(|p| p.x < 150.0 - ERASER_RADIUS + 0.5));
        assert!(b.iter().all(|p| p.x > 150.0 + ERASER_RADIUS - 0.5));
        assert!(a.first().is_some_and(|p| p.x == 0.0) && b.last().is_some_and(|p| p.x == 300.0));
    }

    #[test]
    fn pixel_eraser_takes_a_bite_out_of_a_shape() {
        let mut o = TeacherOverlay::new();
        shape(&mut o, ShapeKind::Rect, (100.0, 100.0), (300.0, 300.0));
        o.erase_at(pos2(200.0, 100.0));
        assert!(o.shapes.is_empty());
        assert!(!o.strokes.is_empty());
        assert!(o.strokes.iter().all(|s| s.points.iter().all(|p| p.distance(pos2(200.0, 100.0)) > ERASER_RADIUS)));
        assert!(o.strokes.iter().any(|s| s.points.iter().any(|p| p.y == 300.0)), "the far side is untouched");
    }

    #[test]
    fn a_palm_wipe_rubs_out_ink_along_its_path_with_any_tool() {
        let mut o = TeacherOverlay::new();
        stroke(&mut o, &[(0.0, 100.0), (600.0, 100.0)]);
        stroke(&mut o, &[(0.0, 500.0), (600.0, 500.0)]);
        o.palm_wipe(Some(pos2(100.0, 100.0)));
        o.palm_wipe(Some(pos2(500.0, 100.0)));
        o.palm_wipe(None);
        assert!(o.strokes.iter().all(|s| s.points.iter().all(|p| p.y == 500.0 || p.x < 31.0 || p.x > 569.0)));
        assert!(o.strokes.iter().any(|s| s.points.iter().any(|p| p.y == 500.0)), "ink away from the hand stays");
    }

    #[test]
    fn slide_change_clears_marks_but_keeps_timers() {
        let mut o = TeacherOverlay::new();
        stroke(&mut o, &[(0.0, 0.0), (1.0, 1.0)]);
        o.add_timer();
        o.set_tool(Tool::Revealer);
        o.on_slide_change();
        assert!(o.strokes.is_empty() && o.revealer.is_none());
        assert_eq!(o.timers.len(), 1);
    }

    #[test]
    fn keep_setting_holds_marks_and_revealer_across_slides() {
        let mut o = TeacherOverlay::new();
        o.keep_on_advance = true;
        stroke(&mut o, &[(0.0, 0.0), (1.0, 1.0)]);
        o.set_tool(Tool::Revealer);
        o.on_slide_change();
        assert_eq!(o.strokes.len(), 1);
        assert!(o.revealer.is_some());
    }

    #[test]
    fn revealer_edge_follows_the_pointer_from_each_side() {
        let full = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 500.0));
        let mut r = Revealer { from: Edge::Bottom, cover: 1.0 };
        r.drag_to(full, pos2(10.0, 125.0));
        assert!((r.cover - 0.75).abs() < 1e-6);
        assert_eq!(r.shade(full), Rect::from_min_max(pos2(0.0, 125.0), pos2(1000.0, 500.0)));
        let mut r = Revealer { from: Edge::Left, cover: 0.0 };
        r.drag_to(full, pos2(250.0, 0.0));
        assert!((r.cover - 0.25).abs() < 1e-6);
        r.drag_to(full, pos2(5000.0, 0.0));
        assert_eq!(r.cover, 1.0);
        r.drag_to(full, pos2(-50.0, 0.0));
        assert_eq!(r.cover, 0.0);
    }

    #[test]
    fn revealer_tool_toggles_and_remembers_the_side() {
        let mut o = TeacherOverlay::new();
        o.set_reveal_from(Edge::Top);
        o.set_tool(Tool::Revealer);
        assert_eq!(o.revealer, Some(Revealer { from: Edge::Top, cover: 1.0 }));
        o.revealer.as_mut().unwrap().cover = 0.4;
        o.set_reveal_from(Edge::Left);
        assert_eq!(o.revealer, Some(Revealer { from: Edge::Left, cover: 0.4 }));
        o.set_tool(Tool::Revealer);
        assert!(o.revealer.is_none() && o.tool == Tool::Pointer);
    }

    #[test]
    fn palette_snaps_to_the_nearest_edge() {
        let full = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 600.0));
        assert_eq!(nearest_edge(full, pos2(500.0, 590.0)), Edge::Bottom);
        assert_eq!(nearest_edge(full, pos2(500.0, 10.0)), Edge::Top);
        assert_eq!(nearest_edge(full, pos2(8.0, 300.0)), Edge::Left);
        assert_eq!(nearest_edge(full, pos2(990.0, 300.0)), Edge::Right);
        assert_eq!(along_edge(full, Edge::Left, pos2(0.0, 300.0)), 0.5);
        assert_eq!(along_edge(full, Edge::Bottom, pos2(0.0, 0.0)), 0.08);
    }

    #[test]
    fn arrow_has_a_head_and_a_shaft() {
        let mut o = TeacherOverlay::new();
        shape(&mut o, ShapeKind::Arrow, (0.0, 0.0), (100.0, 0.0));
        let out = o.shapes[0].outlines();
        assert_eq!(out.len(), 2);
        assert_eq!(out[1][1], pos2(100.0, 0.0));
        assert!(out[1][0].x < 100.0 && out[1][2].x < 100.0 && out[1][0].y * out[1][2].y < 0.0);
    }

    #[test]
    fn the_wheel_and_pointer_belong_to_the_overlay_only_when_a_tool_is_on() {
        let mut o = TeacherOverlay::new();
        assert!(!o.wants_pointer(Some(pos2(5.0, 5.0))) && !o.owns_wheel());
        o.set_tool(Tool::Pen);
        assert!(o.wants_pointer(None) && o.owns_wheel());
        assert_eq!(o.strokes.len(), 0);
    }

    #[test]
    fn highlighter_strokes_are_translucent_and_wide() {
        let mut o = TeacherOverlay::new();
        o.set_tool(Tool::Highlighter);
        let s = o.new_stroke(pos2(0.0, 0.0));
        assert!(s.is_highlight && s.width == HIGHLIGHT_WIDTH && s.colour.a() == 60);
        o.set_tool(Tool::Pen);
        let s = o.new_stroke(pos2(0.0, 0.0));
        assert!(!s.is_highlight && s.width == 8.0 && s.colour == PEN_COLOURS[0].1);
    }
}
