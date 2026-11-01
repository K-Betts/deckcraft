//! Teacher Mode in the slide show: the annotation overlay over the slide, which takes the pointer,
//! the wheel and (for the calculator) the keyboard while it needs them.

use super::*;
use crate::maths::teacher_overlay::Action;

/// Before the show reacts to the pointer: whether Teacher Mode's palette or tools take the
/// pointer, and whether they take the wheel.
pub(super) fn pointer(app: &SlideApp, show: &mut Show, pointer: Option<Pos2>) -> (bool, bool) {
    if let Some(t) = show.teacher.as_mut() {
        t.keep_on_advance = app.ui.maths.keep_annotations_on_advance;
    }
    let blocks = show.teacher.as_ref().is_some_and(|t| t.wants_pointer(pointer));
    let wheel = show.teacher.as_ref().is_some_and(|t| t.owns_wheel());
    (blocks, wheel)
}

/// Draw the overlay over the slide and carry out what its palette asked for; `keep` goes false
/// when that ends the show.
pub(super) fn overlay(app: &mut SlideApp, show: &mut Show, ui: &mut Ui, full: Rect, now: f64, doc: &Presentation, keep: &mut bool) {
    if !show.grid
        && let Some(mut t) = show.teacher.take()
    {
        let asked = t.ui(ui, full, now, &mut app.maths.tools, app.ui.maths.packs.contains(crate::maths::packs::MATHS));
        show.teacher = Some(t);
        for action in asked {
            match action {
                Action::Prev => {
                    show.ended = false;
                    let a = show.state.prev(doc);
                    show.apply(a, doc, now);
                }
                Action::Next if show.ended => *keep = false,
                Action::Next => {
                    let a = show.state.next(doc);
                    if !show.apply(a, doc, now) {
                        *keep = false;
                    }
                }
                Action::Exit => *keep = false,
            }
        }
    }
}

/// The show is ending: put the overlay aside. Next time the slides come first, with the board's
/// drawing kept for later.
pub(super) fn stash_on_exit(app: &mut SlideApp, show: &mut Show) {
    if let Some(mut t) = show.teacher.take() {
        t.set_whiteboard(false);
        app.maths.teacher_stash = Some(t);
    }
}

impl Show {
    #[cfg(test)]
    pub(crate) fn pen_on(&self) -> bool {
        self.pen
    }

    #[cfg(test)]
    pub(crate) fn laser_on(&self) -> bool {
        self.laser
    }
}
