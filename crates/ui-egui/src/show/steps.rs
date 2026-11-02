//! Reveal Step by Step in the slide show: a click on a line of working, or Tab and Shift+Tab, shows
//! or hides just that line, without moving the show on.

use super::render::fit;
use super::{ParaState, Presentation, Rect, ShapeId, ShapeState, Show, Timeline, Ui, pos2};

/// A step line shown (`true`) or hidden by hand in a slide show: (shape, line, shown).
pub(super) type StepMark = (ShapeId, usize, bool);

/// The step lines of the slide being shown.
pub(super) struct Steps {
    /// Where the step lines (Reveal Step by Step) of slide `.0` lie, in slide points:
    /// (shape, line, [x0, y0, x1, y1]), one box per laid-out line of text.
    boxes: Option<(usize, Vec<(ShapeId, usize, [f64; 4])>)>,
    /// Step lines of slide `.0` shown or hidden by hand (a click on the line, Tab, Shift+Tab).
    /// Never saved; they go when the slide changes.
    marks: (usize, Vec<StepMark>),
}

impl Steps {
    pub(super) fn new(slide: usize) -> Steps {
        Steps { boxes: None, marks: (slide, vec![]) }
    }

    /// Lines shown or hidden by hand belong to that visit of the slide.
    pub(super) fn slide_changed(&mut self, slide: usize) {
        self.marks = (slide, vec![]);
    }
}

/// Apply the step lines shown or hidden by hand to `shape`'s paragraphs (a line's indented and
/// blank followers go with it).
pub(super) fn mark_steps(tl: &Timeline, shape: ShapeId, st: &mut ShapeState, marks: &[StepMark]) {
    for (p, ps) in st.paras.iter_mut() {
        let line = tl.paragraph_owner(shape, *p);
        if let Some(&(_, _, shown)) = marks.iter().find(|m| m.0 == shape && m.1 == line) {
            *ps = if shown {
                ParaState { visible: true, opacity: 1.0, offset: deckcraft_geom::Vec2::new(0.0, 0.0) }
            } else {
                ParaState { visible: false, ..*ps }
            };
        }
    }
}

/// The step line (Reveal Step by Step) under the pointer on the shown slide: (shape, line). A
/// hidden line can be clicked where it will appear.
pub(super) fn step_line_under_pointer(ctx: &egui::Context, ui: &Ui, show: &mut Show, doc: &Presentation) -> Option<(ShapeId, usize)> {
    let p = ctx.input(|i| i.pointer.interact_pos())?;
    let full = ui.max_rect();
    let avail = if show.reading { Rect::from_min_max(full.min, pos2(full.max.x, full.max.y - 34.0)) } else { full };
    let srect = fit(avail, doc);
    let k = (srect.width() / doc.slide_size.width.max(1.0) as f32) as f64;
    let (x, y) = ((p.x - srect.min.x) as f64 / k, (p.y - srect.min.y) as f64 / k);
    show.step_boxes(doc).iter().rev().find(|(_, _, b)| b[0] - STEP_PAD <= x && x <= b[2] + STEP_PAD && b[1] <= y && y <= b[3]).map(|b| (b.0, b.1))
}

/// How far beside its text a step line still takes a click, in points.
const STEP_PAD: f64 = 12.0;

impl Show {
    /// Where the current slide's step lines lie (laid out once per slide).
    pub(super) fn step_boxes(&mut self, doc: &Presentation) -> Vec<(ShapeId, usize, [f64; 4])> {
        let i = self.state.slide;
        self.timeline(doc);
        if self.steps.boxes.as_ref().is_none_or(|(s, _)| *s != i) {
            let Some((_, tl)) = &self.timeline else { return vec![] };
            let mut boxes = vec![];
            for t in deckcraft_render::place_slide(doc, i).texts {
                let lines = tl.step_lines(t.shape);
                if lines.is_empty() {
                    continue;
                }
                let [a, b, c, d, e, f] = t.transform.as_coeffs();
                for l in t.layout.lines.iter().filter(|l| l.extent.1 > l.extent.0) {
                    let line = tl.paragraph_owner(t.shape, l.para);
                    if !lines.iter().any(|(p, _)| *p == line) {
                        continue;
                    }
                    let corners = [(l.extent.0, l.top), (l.extent.1, l.top), (l.extent.0, l.bottom), (l.extent.1, l.bottom)]
                        .map(|(x, y)| (a * x + c * y + e, b * x + d * y + f));
                    let (xs, ys) = (corners.map(|q| q.0), corners.map(|q| q.1));
                    let lo = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
                    let hi = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
                    boxes.push((t.shape, line, [lo(xs), lo(ys), hi(xs), hi(ys)]));
                }
            }
            self.steps.boxes = Some((i, boxes));
        }
        self.steps.boxes.as_ref().map(|(_, b)| b.clone()).unwrap_or_default()
    }

    /// The step lines shown or hidden by hand on the current slide.
    pub(super) fn current_marks(&self) -> Vec<StepMark> {
        if self.steps.marks.0 == self.state.slide { self.steps.marks.1.clone() } else { vec![] }
    }

    /// The current slide's step lines in the order they are revealed, each with whether it shows
    /// now: as set by hand, else as far as the show has played.
    pub(super) fn step_lines(&mut self, doc: &Presentation) -> Vec<StepMark> {
        self.timeline(doc);
        let marks = self.current_marks();
        let Some((_, tl)) = &self.timeline else { return vec![] };
        let mut v: Vec<(usize, usize, ShapeId, usize)> = vec![];
        for (k, id) in tl.animated_shapes().into_iter().enumerate() {
            v.extend(tl.step_lines(id).into_iter().map(|(line, step)| (step, k, id, line)));
        }
        v.sort_unstable_by_key(|x| (x.0, x.1, x.3));
        v.into_iter()
            .map(|(step, _, id, line)| {
                let shown = marks.iter().find(|m| m.0 == id && m.1 == line).map_or(step < self.state.step, |m| m.2);
                (id, line, shown)
            })
            .collect()
    }

    pub(super) fn set_step(&mut self, shape: ShapeId, line: usize, shown: bool) {
        if self.steps.marks.0 != self.state.slide {
            self.steps.marks = (self.state.slide, vec![]);
        }
        let marks = &mut self.steps.marks.1;
        marks.retain(|m| !(m.0 == shape && m.1 == line));
        marks.push((shape, line, shown));
    }

    /// A click on a step line: show it if hidden, hide it if shown.
    pub(super) fn toggle_step(&mut self, doc: &Presentation, shape: ShapeId, line: usize) {
        let shown = self.step_lines(doc).iter().any(|m| m.0 == shape && m.1 == line && m.2);
        self.set_step(shape, line, !shown);
    }

    /// Tab: show the first hidden step line; Shift+Tab (`forward` false): hide the last shown.
    /// False when there is none.
    pub(super) fn tab_step(&mut self, doc: &Presentation, forward: bool) -> bool {
        let lines = self.step_lines(doc);
        let pick = if forward { lines.iter().find(|m| !m.2) } else { lines.iter().rev().find(|m| m.2) };
        match pick.copied() {
            Some((id, line, _)) => {
                self.set_step(id, line, forward);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::super::state_for;
    use super::*;
    use crate::SlideApp;
    use egui::{Pos2, vec2};
    use serde_json::json;

    /// A session whose first slide has a Reveal Step by Step box of three lines (a blank line
    /// between the second and third) and a second, empty slide.
    fn reveal_session() -> (deckcraft_engine::Session, ShapeId) {
        let mut s = deckcraft_engine::Session::with_new();
        let id = s.execute("shape.insert", &json!({"preset": "rect", "rect": [60.0, 60.0, 400.0, 300.0]})).unwrap()["id"].as_u64().unwrap();
        s.execute("text.set", &json!({"id": id, "text": "2x + 3 = 11\n2x = 8\n\nx = 4"})).unwrap();
        s.execute("animation.steps", &json!({"ids": [id]})).unwrap();
        s.execute("slide.new", &json!({})).unwrap();
        (s, ShapeId(id as u32))
    }

    #[test]
    fn tab_and_clicks_show_and_hide_step_lines_by_hand() {
        let (s, id) = reveal_session();
        let doc = s.active().unwrap().doc.clone();
        let mut show = Show::new(&doc, 0, false, false);
        let shown = |show: &mut Show| show.step_lines(&doc).iter().map(|m| m.2).collect::<Vec<_>>();
        assert_eq!(shown(&mut show), [false, false, false]);
        assert!(show.tab_step(&doc, true));
        assert!(show.tab_step(&doc, true));
        assert_eq!(shown(&mut show), [true, true, false], "Tab shows the next hidden line");
        assert!(show.tab_step(&doc, false));
        assert_eq!(shown(&mut show), [true, false, false], "Shift+Tab hides the last shown line");
        // A click shows or hides one line, in any order.
        show.toggle_step(&doc, id, 3);
        show.toggle_step(&doc, id, 0);
        assert_eq!(shown(&mut show), [false, false, true]);

        // The frame draws what was set by hand: the blank line follows its owner.
        let tl = show.timeline(&doc).clone();
        let mut st = state_for(&tl, id, 0, 0.0, &[], &[]);
        mark_steps(&tl, id, &mut st, &show.current_marks());
        let vis: Vec<(usize, bool)> = st.paras.iter().map(|(p, ps)| (*p, ps.visible)).collect();
        assert_eq!(vis, [(0, false), (1, false), (2, false), (3, true)]);

        // A line played by the show can be hidden again; a new slide starts with nothing set.
        for a in [show.state.next(&doc), show.state.step_done(&doc)] {
            show.apply(a, &doc, 0.0);
        }
        assert_eq!(show.state.step, 1);
        show.set_step(id, 0, false);
        assert!(!shown(&mut show)[0], "hidden again by hand");
        let a = show.state.goto(&doc, 1);
        show.apply(a, &doc, 0.0);
        assert!(show.current_marks().is_empty() && !show.tab_step(&doc, true), "no step lines on the next slide");
        let a = show.state.goto(&doc, 0);
        show.apply(a, &doc, 0.0);
        assert!(show.current_marks().is_empty(), "the marks went with the slide change");
    }

    #[test]
    fn in_the_show_tab_and_a_click_on_a_line_reveal_it() {
        let (s, id) = reveal_session();
        let mut app = SlideApp::new(s, crate::Services::default());
        app.ui.presenter_view = false;
        let mut harness = egui_kittest::Harness::builder().with_size(vec2(1000.0, 700.0)).build_ui_state(
            |ui, app: &mut SlideApp| {
                app.logic(ui.ctx());
                app.ui(ui);
            },
            app,
        );
        harness.run_steps(3);
        harness.state_mut().start_show(0, false);
        harness.run_steps(3);
        harness.key_press(egui::Key::Tab);
        harness.run_steps(2);
        let marks = harness.state().show.as_ref().unwrap().current_marks();
        assert_eq!(marks, [(id, 0, true)], "Tab shows the first line");
        assert_eq!(harness.state().show.as_ref().unwrap().state.step, 0, "without moving the show on");

        // Click on the (hidden) last line, where it will appear.
        let doc = harness.state().session.active().unwrap().doc.clone();
        let show = harness.state_mut().show.as_mut().unwrap();
        let b = show.step_boxes(&doc).into_iter().find(|b| b.0 == id && b.1 == 3).expect("x = 4 has a box").2;
        let srect = fit(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0)), &doc);
        let k = srect.width() / doc.slide_size.width as f32;
        let at = pos2(srect.min.x + ((b[0] + b[2]) / 2.0) as f32 * k, srect.min.y + ((b[1] + b[3]) / 2.0) as f32 * k);
        harness.input_mut().events.push(egui::Event::PointerMoved(at));
        harness.run_steps(1);
        for pressed in [true, false] {
            harness.input_mut().events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            harness.run_steps(1);
        }
        harness.run_steps(2);
        let show = harness.state().show.as_ref().unwrap();
        assert_eq!(show.current_marks(), [(id, 0, true), (id, 3, true)], "the click showed just that line");
        assert_eq!((show.state.slide, show.state.step), (0, 0), "and did not advance the show");
    }
}
