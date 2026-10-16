//! Teacher Mode: ephemeral annotation tools drawn over a running slide show (pen, highlighter,
//! shapes, eraser, laser, revealer, spotlight and countdown timers). Nothing here is ever written
//! to the presentation; it lives and dies with the [`crate::show::Show`] that owns it.

use std::collections::VecDeque;

use egui::{Align2, Color32, CornerRadius, Id, LayerId, Margin, Mesh, Order, Painter, Pos2, Rect, Response, Sense, Shape, Ui, Vec2, pos2, vec2};

/// Strokes within this distance of the eraser centre are removed.
pub const ERASER_RADIUS: f32 = 20.0;
/// Positions kept behind the laser pointer.
pub const TRAIL_LEN: usize = 8;
/// Seconds without pointer activity before the palette hides.
pub const HIDE_AFTER: f64 = 3.0;
pub const HIGHLIGHT_WIDTH: f32 = 20.0;
pub const SPOTLIGHT_DEFAULT: f32 = 200.0;
const SPOTLIGHT_RANGE: (f32, f32) = (60.0, 700.0);
const TIMER_DEFAULT_SECS: f64 = 300.0;
const TIMER_MAX_SECS: f64 = 99.0 * 60.0 + 59.0;
const REVEAL_GRAB: f32 = 24.0;
const PALETTE_MARGIN: f32 = 16.0;

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
    Laser,
    Pen,
    Highlighter,
    Shape,
    Eraser,
    Revealer,
    Spotlight,
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
    editing: Option<String>,
}

impl Timer {
    pub fn new(id: u64, duration: f64) -> Self {
        let duration = duration.clamp(1.0, TIMER_MAX_SECS);
        Timer { id, duration, remaining: duration, running: false, expired: false, editing: None }
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

    /// `MM:SS`, rounding up so a fresh 5:00 timer reads 05:00 until a full second has passed.
    pub fn label(&self) -> String {
        let s = self.remaining.max(0.0).ceil() as u64;
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

/// `m:ss`, `mm:ss` or plain minutes (`5`, `7.5`) as seconds.
pub fn parse_duration(text: &str) -> Option<f64> {
    let t = text.trim();
    let secs = match t.split_once(':') {
        Some((m, s)) => {
            let (m, s) = (m.trim().parse::<u32>().ok()?, s.trim().parse::<u32>().ok()?);
            if s >= 60 {
                return None;
            }
            f64::from(m) * 60.0 + f64::from(s)
        }
        None => t.parse::<f64>().ok().filter(|m| m.is_finite())? * 60.0,
    };
    (1.0..=TIMER_MAX_SECS).contains(&secs).then_some(secs)
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
    pub spotlight_locked_pos: Option<Pos2>,
    pub spotlight_radius: f32,
    pub timers: Vec<Timer>,
    /// Keep marks and the revealer when the slide changes (the Preferences setting).
    pub keep_on_advance: bool,
    next_id: u64,
    seq: u64,
    trail: VecDeque<Pos2>,
    draft: Option<Draft>,
    revealer_drag: bool,
    edge: Edge,
    along: f32,
    palette_drag: Option<Pos2>,
    palette_hot: bool,
    last_activity: Option<f64>,
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
            spotlight_locked_pos: None,
            spotlight_radius: SPOTLIGHT_DEFAULT,
            timers: vec![],
            keep_on_advance: false,
            next_id: 0,
            seq: 0,
            trail: VecDeque::new(),
            draft: None,
            revealer_drag: false,
            edge: Edge::Bottom,
            along: 0.5,
            palette_drag: None,
            palette_hot: false,
            last_activity: None,
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

    /// Remove every stroke and shape (not timers, the revealer or the spotlight).
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

    /// The slide changed: marks go, and so do the revealer and a pinned spotlight, unless the
    /// teacher chose to keep annotations. Timers always carry on.
    pub fn on_slide_change(&mut self) {
        if !self.keep_on_advance {
            self.clear();
            self.revealer = None;
            self.spotlight_locked_pos = None;
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
        self.timers.push(Timer::new(id, TIMER_DEFAULT_SECS));
    }

    /// Advance the timers to `now`, calling `on_zero` for each that finishes.
    pub fn tick_timers(&mut self, now: f64, mut on_zero: impl FnMut()) {
        let dt = self.last_tick.map_or(0.0, |t| (now - t).max(0.0));
        self.last_tick = Some(now);
        for t in &mut self.timers {
            t.tick(dt, &mut on_zero);
        }
    }

    pub fn editing_timer(&self) -> bool {
        self.timers.iter().any(|t| t.editing.is_some())
    }

    /// Whether the overlay takes the pointer, so the show must not also act on the click.
    pub fn wants_pointer(&self, at: Option<Pos2>) -> bool {
        self.tool != Tool::Pointer || at.is_some_and(|p| self.hit_rects.iter().any(|r| r.expand(6.0).contains(p)))
    }

    /// The wheel resizes the spotlight rather than turning the page.
    pub fn owns_wheel(&self) -> bool {
        self.tool != Tool::Pointer
    }

    /// Remove strokes and shapes that pass within [`ERASER_RADIUS`] of `p`.
    pub fn erase_at(&mut self, p: Pos2) {
        self.strokes.retain(|s| !near_polyline(&s.points, p, ERASER_RADIUS));
        self.shapes.retain(|s| !s.outlines().iter().any(|o| near_polyline(o, p, ERASER_RADIUS)));
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
    pub fn ui(&mut self, ui: &mut Ui, full: Rect, now: f64) -> Vec<Action> {
        let ctx = ui.ctx().clone();
        let mut actions = vec![];
        let mut beep_now = false;
        self.tick_timers(now, || beep_now = true);
        if beep_now {
            beep();
        }
        let (hover, delta, down, origin, scroll) =
            ctx.input(|i| (i.pointer.hover_pos(), i.pointer.delta(), i.pointer.any_down(), i.pointer.press_origin(), i.smooth_scroll_delta.y));
        if delta.length() > 0.0 || down || self.last_activity.is_none() {
            self.last_activity = Some(now);
        }
        let resp = ui.interact(full, Id::new("teacher_surface"), if self.tool == Tool::Pointer { Sense::hover() } else { Sense::click_and_drag() });
        self.pointer_tools(&resp, full, hover, origin, scroll);
        if self.tool == Tool::Laser && hover.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        } else if matches!(self.tool, Tool::Pen | Tool::Highlighter | Tool::Shape) {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        } else if matches!(self.tool, Tool::Eraser | Tool::Spotlight) {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }
        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("teacher_overlay")));
        self.paint(&painter, full, hover);
        self.hit_rects.clear();
        let idle = now - self.last_activity.unwrap_or(now);
        if idle < HIDE_AFTER || self.palette_hot || self.palette_drag.is_some() {
            self.palette(&ctx, full, &mut actions);
        } else {
            self.palette_hot = false;
        }
        let mut dead = vec![];
        for (i, t) in self.timers.iter_mut().enumerate() {
            let (remove, rect) = timer_widget(&ctx, t, i, full, now);
            self.hit_rects.push(rect);
            if remove {
                dead.push(t.id);
            }
        }
        self.timers.retain(|t| !dead.contains(&t.id));
        if self.timers.iter().any(|t| t.running || t.expired) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        } else if idle < HIDE_AFTER {
            ctx.request_repaint_after(std::time::Duration::from_millis(((HIDE_AFTER - idle).max(0.05) * 1000.0) as u64));
        }
        actions
    }

    fn pointer_tools(&mut self, resp: &Response, full: Rect, hover: Option<Pos2>, origin: Option<Pos2>, scroll: f32) {
        let dragging = resp.dragged();
        let at = resp.interact_pointer_pos().or(hover);
        match self.tool {
            Tool::Pointer => {}
            Tool::Laser => {
                if let Some(p) = hover
                    && self.trail.back() != Some(&p)
                {
                    self.trail.push_back(p);
                    while self.trail.len() > TRAIL_LEN {
                        self.trail.pop_front();
                    }
                }
            }
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
            Tool::Spotlight => {
                if scroll != 0.0 {
                    self.spotlight_radius = (self.spotlight_radius + scroll * 0.5).clamp(SPOTLIGHT_RANGE.0, SPOTLIGHT_RANGE.1);
                }
                if resp.clicked() {
                    self.spotlight_locked_pos = match self.spotlight_locked_pos {
                        Some(_) => None,
                        None => hover,
                    };
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
        if self.tool == Tool::Spotlight
            && let Some(c) = self.spotlight_locked_pos.or(hover)
        {
            p.add(Shape::mesh(spotlight_mesh(full, c, self.spotlight_radius, Color32::from_rgba_unmultiplied(0, 0, 0, 180))));
            p.circle_stroke(c, self.spotlight_radius, egui::Stroke::new(1.5, Color32::from_white_alpha(120)));
        }
        if self.tool == Tool::Laser {
            let n = self.trail.len();
            for (i, q) in self.trail.iter().enumerate() {
                let f = (i + 1) as f32 / (n.max(1) + 1) as f32;
                p.circle_filled(*q, 4.0 + 4.0 * f, Color32::from_rgba_unmultiplied(255, 30, 30, (90.0 * f) as u8));
            }
            if let Some(h) = hover {
                p.circle_filled(h, 8.0, Color32::from_rgba_unmultiplied(255, 30, 30, 90));
                p.circle_filled(h, 5.0, Color32::from_rgb(255, 30, 30));
            }
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
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(24, 24, 28, 232))
                .corner_radius(CornerRadius::same(26))
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                    let has_options = matches!(self.tool, Tool::Pen | Tool::Highlighter | Tool::Shape | Tool::Revealer);
                    let mut body = |this: &mut Self, ui: &mut Ui| {
                        if has_options {
                            lay(ui, horizontal, |ui| this.options(ui));
                        }
                        lay(ui, horizontal, |ui| this.main_row(ui, full, actions));
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
        self.palette_hot = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| rect.expand(8.0).contains(p));
    }

    fn options(&mut self, ui: &mut Ui) {
        let swatch = |ui: &mut Ui, colour: Color32, on: bool, tip: &str| {
            let (r, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
            let shown = Color32::from_rgb(colour.r(), colour.g(), colour.b());
            ui.painter().circle_filled(r.center(), 9.0, shown);
            if on || resp.hovered() {
                ui.painter().circle_stroke(r.center(), 11.5, egui::Stroke::new(2.0, Color32::WHITE));
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
                    let (r, resp) = ui.allocate_exact_size(vec2(26.0, 24.0), Sense::click());
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
            _ => {}
        }
    }

    fn main_row(&mut self, ui: &mut Ui, full: Rect, actions: &mut Vec<Action>) {
        let grip = ui.allocate_response(vec2(16.0, 34.0), Sense::drag()).on_hover_text("Drag to move the toolbar");
        for dy in [-7.0, 0.0, 7.0] {
            ui.painter().circle_filled(grip.rect.center() + vec2(0.0, dy), 1.8, Color32::from_gray(150));
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
        let tools: [(Tool, &str); 8] = [
            (Tool::Pointer, "Pointer"),
            (Tool::Laser, "Laser pointer"),
            (Tool::Pen, "Pen"),
            (Tool::Highlighter, "Highlighter"),
            (Tool::Shape, "Shapes"),
            (Tool::Eraser, "Eraser"),
            (Tool::Revealer, "Revealer (click again to put it away)"),
            (Tool::Spotlight, "Spotlight (click to pin, scroll to resize)"),
        ];
        for (tool, name) in tools {
            if tool == Tool::Revealer && icon_button(ui, false, "Clear all marks", paint_clear_icon).clicked() {
                self.clear();
            }
            if icon_button(ui, self.tool == tool, name, |p, r, c| paint_tool_icon(p, r, c, tool)).clicked() {
                self.set_tool(tool);
            }
        }
        if icon_button(ui, false, "Add a countdown timer", paint_timer_icon).clicked() {
            self.add_timer();
        }
        if icon_button(ui, false, "Previous", |p, r, c| paint_arrow_icon(p, r, c, false)).clicked() {
            actions.push(Action::Prev);
        }
        if icon_button(ui, false, "Next", |p, r, c| paint_arrow_icon(p, r, c, true)).clicked() {
            actions.push(Action::Next);
        }
        let (r, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
        let fill = if resp.hovered() { Color32::from_rgb(240, 70, 70) } else { Color32::from_rgb(200, 40, 40) };
        ui.painter().circle_filled(r.center(), 13.0, fill);
        let c = r.center();
        let s = egui::Stroke::new(2.2, Color32::WHITE);
        ui.painter().line_segment([c + vec2(-4.5, -4.5), c + vec2(4.5, 4.5)], s);
        ui.painter().line_segment([c + vec2(-4.5, 4.5), c + vec2(4.5, -4.5)], s);
        if resp.on_hover_text("Exit Teacher Mode (Esc)").clicked() {
            actions.push(Action::Exit);
        }
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
    let (r, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(r, CornerRadius::same(10), Color32::from_rgb(52, 112, 214));
    } else if resp.hovered() {
        p.rect_filled(r, CornerRadius::same(10), Color32::from_white_alpha(28));
    }
    paint(p, r.shrink(7.0), Color32::WHITE);
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
        Tool::Laser => {
            p.circle_filled(ctr, 8.0, Color32::from_rgba_unmultiplied(255, 60, 60, 90));
            p.circle_filled(ctr, 4.0, Color32::from_rgb(255, 60, 60));
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
        Tool::Spotlight => {
            p.rect_filled(r, CornerRadius::same(2), Color32::from_gray(90));
            p.circle_filled(ctr, r.width() * 0.3, c);
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

fn near_polyline(points: &[Pos2], p: Pos2, r: f32) -> bool {
    match points {
        [] => false,
        [only] => only.distance(p) <= r,
        _ => points.windows(2).any(|w| dist_to_segment(p, w[0], w[1]) <= r),
    }
}

/// The screen with a circle cut out, as a triangle mesh (egui has no "hole" shape).
pub fn spotlight_mesh(full: Rect, centre: Pos2, r: f32, colour: Color32) -> Mesh {
    let mut m = Mesh::default();
    let sq = Rect::from_center_size(centre, Vec2::splat(r * 2.0));
    let cx = |x: f32| x.clamp(full.min.x, full.max.x);
    let cy = |y: f32| y.clamp(full.min.y, full.max.y);
    let (x0, x1, y0, y1) = (cx(sq.min.x), cx(sq.max.x), cy(sq.min.y), cy(sq.max.y));
    for rect in [
        Rect::from_min_max(full.min, pos2(full.max.x, y0)),
        Rect::from_min_max(pos2(full.min.x, y1), full.max),
        Rect::from_min_max(pos2(full.min.x, y0), pos2(x0, y1)),
        Rect::from_min_max(pos2(x1, y0), pos2(full.max.x, y1)),
    ] {
        if rect.width() > 0.0 && rect.height() > 0.0 {
            m.add_colored_rect(rect, colour);
        }
    }
    // The ring between the circle and its bounding square. The corners of the square sit on
    // sample angles, so the outer edge follows the square exactly.
    const N: u32 = 96;
    let base = m.vertices.len() as u32;
    for k in 0..N {
        let t = std::f32::consts::FRAC_PI_4 + k as f32 * std::f32::consts::TAU / N as f32;
        let d = vec2(t.cos(), t.sin());
        let out = r / d.x.abs().max(d.y.abs());
        m.colored_vertex(centre + d * r, colour);
        m.colored_vertex(centre + d * out, colour);
    }
    for k in 0..N {
        let n = (k + 1) % N;
        let (a, b, c, d) = (base + 2 * k, base + 2 * k + 1, base + 2 * n, base + 2 * n + 1);
        m.add_triangle(a, b, d);
        m.add_triangle(a, d, c);
    }
    m
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

/// A floating timer. Returns whether it was dismissed, and where it is.
fn timer_widget(ctx: &egui::Context, t: &mut Timer, index: usize, full: Rect, now: f64) -> (bool, Rect) {
    let mut remove = false;
    let flash = t.expired && (now * 2.0) as i64 % 2 == 0;
    let fill = if flash { Color32::from_rgb(200, 30, 30) } else { Color32::from_rgba_unmultiplied(24, 24, 28, 235) };
    let out = egui::Area::new(Id::new(("teacher_timer", t.id)))
        .order(Order::Foreground)
        .default_pos(pos2(full.min.x + 24.0, full.min.y + 24.0 + index as f32 * 150.0))
        .movable(true)
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::new().fill(fill).corner_radius(CornerRadius::same(16)).inner_margin(Margin::same(12)).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(text) = t.editing.as_mut() {
                        let edit = ui.add(
                            egui::TextEdit::singleline(text)
                                .desired_width(130.0)
                                .font(egui::FontId::monospace(40.0))
                                .horizontal_align(egui::Align::Center),
                        );
                        edit.request_focus();
                        let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
                        if edit.lost_focus() || esc {
                            let committed = if esc { None } else { parse_duration(text) };
                            t.editing = None;
                            if let Some(secs) = committed {
                                t.set_duration(secs);
                            }
                        }
                    } else {
                        let label = egui::RichText::new(t.label()).monospace().size(44.0).color(Color32::WHITE);
                        if ui.add(egui::Button::new(label).frame(false)).on_hover_text("Click to set the time (m:ss or minutes)").clicked() {
                            t.editing = Some(t.label());
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui.button(if t.running { "Pause" } else { "Start" }).clicked() {
                            t.toggle();
                        }
                        if ui.button("Reset").clicked() {
                            t.reset();
                        }
                        if ui.button("✕").on_hover_text("Dismiss").clicked() {
                            remove = true;
                        }
                    });
                });
            });
        });
    (remove, out.response.rect)
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
    fn durations_parse_as_minutes_or_mm_ss() {
        assert_eq!(parse_duration("5"), Some(300.0));
        assert_eq!(parse_duration("7:30"), Some(450.0));
        assert_eq!(parse_duration(" 0:45 "), Some(45.0));
        assert_eq!(parse_duration("1.5"), Some(90.0));
        for bad in ["", "abc", "1:75", "0", "0:00", "-3", "NaN", "9999"] {
            assert_eq!(parse_duration(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn eraser_removes_what_it_touches_and_keeps_the_rest() {
        let mut o = TeacherOverlay::new();
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
    fn slide_change_clears_marks_but_keeps_timers() {
        let mut o = TeacherOverlay::new();
        stroke(&mut o, &[(0.0, 0.0), (1.0, 1.0)]);
        o.add_timer();
        o.set_tool(Tool::Revealer);
        o.spotlight_locked_pos = Some(pos2(5.0, 5.0));
        o.on_slide_change();
        assert!(o.strokes.is_empty() && o.revealer.is_none() && o.spotlight_locked_pos.is_none());
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
    fn spotlight_mesh_covers_everything_but_the_circle() {
        let full = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0));
        let r = 200.0;
        let m = spotlight_mesh(full, pos2(500.0, 400.0), r, Color32::BLACK);
        let area: f32 = m
            .indices
            .chunks(3)
            .map(|t| {
                let v = |i: u32| m.vertices[i as usize].pos;
                let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
                ((b - a).x * (c - a).y - (b - a).y * (c - a).x).abs() / 2.0
            })
            .sum();
        // A 96-gon is slightly smaller than the circle it approximates.
        let hole = 96.0 / 2.0 * r * r * (std::f32::consts::TAU / 96.0).sin();
        assert!((area - (full.area() - hole)).abs() < 1.0, "{area}");
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
