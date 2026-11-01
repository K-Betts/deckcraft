//! The maths edition's own screens: Teacher Mode and its tools, subject packs, Get Ready and the
//! other maths dialogs, Google Drive and in-app updates.
//!
//! Upstream DeckCraft's files reach this code through small hooks (a field, a call, a match arm),
//! listed in `docs/fork-changes.md`; everything else lives here.

pub mod app;
pub mod calc_basen;
pub mod calc_complex;
pub mod calc_matrix;
pub mod calc_poly;
pub mod calc_sheet;
pub mod calc_stats;
pub mod calculator;
pub mod calculator_apps;
pub mod calculator_forms;
pub mod calculator_ui;
pub mod drive_ui;
pub mod equation_view;
pub mod maths_tools;
pub mod packs;
pub mod teacher_overlay;
pub mod update_ui;

#[cfg(test)]
mod appearance_tests;
#[cfg(test)]
mod cleanup_tests;
#[cfg(test)]
mod drive_tests;
#[cfg(test)]
mod equation_tests;
#[cfg(test)]
mod exam_library_tests;
#[cfg(test)]
mod exam_question_tests;
#[cfg(test)]
mod get_ready_tests;
#[cfg(test)]
mod packs_tests;
#[cfg(test)]
mod ribbon_fold_tests;
#[cfg(test)]
mod teacher_tests;
#[cfg(test)]
mod update_tests;
