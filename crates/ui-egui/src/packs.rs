//! Subject packs: optional bundles of teaching features a teacher switches on.
//!
//! A fresh install is subject-neutral. A pack only ever *adds* things (a Teacher Mode tool group,
//! New Slide layouts, a ribbon tab, templates); it never moves or hides what was already there, so
//! switching a pack on and off again leaves the app exactly as it was (design principle 4).
//! Adding science, languages, music or PE later means adding one [`Pack`] to [`PACKS`] and
//! gating its contributions on `SlideApp::pack_on`.

use std::collections::BTreeSet;

/// What a pack can put in front of the teacher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// A group of ephemeral tools in the Teacher Mode palette.
    TeacherTools,
    /// Entries in the New Slide gallery.
    NewSlideLayouts,
    /// A ribbon tab of authoring tools.
    RibbonTab,
    /// Lesson deck templates on the start screen.
    Templates,
}

pub struct Pack {
    /// Stable id stored in `ui.json`.
    pub id: &'static str,
    pub label: &'static str,
    /// One line shown beside the switch.
    pub blurb: &'static str,
    pub slots: &'static [Slot],
}

pub const MATHS: &str = "maths";

pub const PACKS: &[Pack] = &[Pack {
    id: MATHS,
    label: "Maths",
    blurb: "Ruler, protractor, number line and other board tools in Teacher Mode; Get Ready starter slides.",
    slots: &[Slot::TeacherTools, Slot::NewSlideLayouts, Slot::Templates],
}];

pub fn find(id: &str) -> Option<&'static Pack> {
    PACKS.iter().find(|p| p.id == id)
}

/// Keep only packs this build knows about (a hand-edited or newer `ui.json` may name others).
pub fn sanitized(on: BTreeSet<String>) -> BTreeSet<String> {
    on.into_iter().filter(|id| find(id).is_some()).collect()
}
