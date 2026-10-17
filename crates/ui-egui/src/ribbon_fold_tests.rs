//! The responsive ribbon: whole groups that do not fit fold into a More menu, and nothing is lost.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use egui_kittest::kittest::Queryable;

use crate::ribbon::groups_that_fit;
use crate::{Services, SlideApp};
use deckcraft_engine::Session;

fn harness(width: f32, tab: &str) -> egui_kittest::Harness<'static, SlideApp> {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    app.ribbon_fold = true;
    app.ui.tab = tab.into();
    app.ui.subjects_hint_dismissed = true;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(width, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

#[test]
fn whole_groups_are_kept_left_to_right_and_the_rest_fold() {
    // Fits: nothing folds, and no room is kept for More.
    assert_eq!(groups_that_fit(&[100.0, 100.0, 100.0], 300.0), 3);
    // Does not fit: leave room for the More button (66pt), keep only whole groups.
    assert_eq!(groups_that_fit(&[100.0, 100.0, 100.0], 299.0), 2);
    assert_eq!(groups_that_fit(&[100.0, 100.0, 100.0], 270.0), 2);
    assert_eq!(groups_that_fit(&[100.0, 100.0, 100.0], 160.0), 0);
    // A wide group is never split, and nothing after it sneaks in ahead of it.
    assert_eq!(groups_that_fit(&[50.0, 400.0, 20.0], 300.0), 1);
    assert_eq!(groups_that_fit(&[], 300.0), 0);
}

#[test]
fn a_wide_window_shows_every_group_and_no_more_button() {
    let h = harness(2600.0, "Home");
    assert_eq!(h.state().ribbon_folded, 0);
    assert!(h.query_by_label("More tools").is_none());
}

#[test]
fn a_narrow_window_folds_groups_into_a_more_menu_that_still_has_them() {
    let mut h = harness(900.0, "Home");
    let folded = h.state().ribbon_folded;
    assert!(folded > 0, "at 900pt something must fold");
    assert!(h.query_by_label("Selection Pane").is_none(), "folded groups are not on the ribbon itself");
    h.get_by_label("More tools").click();
    h.run_steps(4);
    assert!(h.query_by_label("Selection Pane").is_some(), "the More menu holds the folded buttons");
    assert!(h.query_by_label("Find").is_some());
}

#[test]
fn a_folded_button_still_works_from_the_menu() {
    let mut h = harness(900.0, "Home");
    h.get_by_label("More tools").click();
    h.run_steps(4);
    h.get_by_label("Replace Fonts…").click();
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().map(|d| d.id.as_str()), Some("replaceFonts"));
}

#[test]
fn busy_tabs_fold_when_narrow_and_all_tabs_fit_when_wide() {
    for tab in ["Home", "Insert", "Draw", "Design", "Transitions", "Animations", "Slide Show", "Record", "Review", "View"] {
        assert_eq!(harness(3000.0, tab).state().ribbon_folded, 0, "{tab} fits at 3000pt");
    }
    for tab in ["Home", "Insert", "Design", "Animations"] {
        assert!(harness(500.0, tab).state().ribbon_folded > 0, "{tab} folds at 500pt");
    }
}

#[test]
fn nothing_is_lost_every_button_is_on_the_ribbon_or_in_the_menu() {
    let wide = |tab: &str, labels: &[&str]| {
        let h = harness(3000.0, tab);
        for l in labels {
            assert!(h.query_by_label(l).is_some(), "{tab}: {l} exists when there is room");
        }
    };
    let narrow = |tab: &str, labels: &[&str]| {
        let mut h = harness(700.0, tab);
        h.get_by_label("More tools").click();
        h.run_steps(4);
        for l in labels {
            let found = h.query_all_by_label(l).count();
            assert_eq!(found, 1, "{tab}: {l} appears exactly once across the ribbon and the menu");
        }
    };
    let home = ["Paste", "Picture", "Shapes", "Text Box", "Find", "Replace", "Selection Pane", "Design Ideas"];
    wide("Home", &home);
    narrow("Home", &home);
    let insert = ["Table", "Equation", "Symbol"];
    wide("Insert", &insert);
    narrow("Insert", &insert);
}
