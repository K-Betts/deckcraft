//! Teacher Mode inside the running slide show: entry and exit, what a slide change clears, and
//! that drawing does not also turn the page.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use egui::{Event, Key, Modifiers, PointerButton, Pos2, pos2};
use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::teacher_overlay::{TeacherOverlay, Tool};
use crate::{Services, SlideApp};
use deckcraft_engine::Session;

fn harness() -> egui_kittest::Harness<'static, SlideApp> {
    let mut s = Session::with_new();
    s.execute("slide.new", &json!({})).unwrap();
    s.execute("slide.new", &json!({})).unwrap();
    s.execute("slide.go", &json!({"index": 0})).unwrap();
    let mut app = SlideApp::new(s, Services::default());
    // The presenter view is a second window; the harness would embed it over the slide.
    app.ui.presenter_view = false;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(2);
    h
}

fn teacher(h: &mut egui_kittest::Harness<'static, SlideApp>) {
    h.state_mut().run("show.teacher", json!({"on": true})).unwrap();
    h.run_steps(3);
}

fn slide(h: &egui_kittest::Harness<'static, SlideApp>) -> Option<usize> {
    h.state().show.as_ref().map(|s| s.state.slide)
}

fn overlay<'a>(h: &'a mut egui_kittest::Harness<'static, SlideApp>) -> &'a mut TeacherOverlay {
    h.state_mut().show.as_mut().unwrap().teacher.as_mut().unwrap()
}

fn drag(h: &mut egui_kittest::Harness<'static, SlideApp>, from: Pos2, to: Pos2) {
    let btn = |pressed| Event::PointerButton { pos: from, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    h.event(Event::PointerMoved(from));
    h.run_steps(1);
    h.event(btn(true));
    h.run_steps(1);
    for i in 1..=6 {
        h.event(Event::PointerMoved(from + (to - from) * (i as f32 / 6.0)));
        h.run_steps(1);
    }
    h.event(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

#[test]
fn the_command_starts_a_show_with_the_overlay_and_turns_it_off() {
    let mut h = harness();
    assert!(h.state().show.is_none());
    teacher(&mut h);
    assert!(h.state().show.as_ref().is_some_and(|s| s.teacher.is_some()));
    h.state_mut().run("show.teacher", json!({"on": false})).unwrap();
    h.run_steps(2);
    assert!(h.state().show.as_ref().is_some_and(|s| s.teacher.is_none()), "the plain show carries on");
}

#[test]
fn the_shortcut_enters_teacher_mode_and_f5_stays_the_plain_show() {
    let mut h = harness();
    h.key_press(Key::F5);
    h.run_steps(3);
    assert!(h.state().show.as_ref().is_some_and(|s| s.teacher.is_none()), "F5 is the ordinary slide show");
    h.state_mut().run("show.end", json!({})).unwrap();
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::T);
    h.run_steps(3);
    assert!(h.state().show.as_ref().is_some_and(|s| s.teacher.is_some()));
}

#[test]
fn escape_leaves_teacher_mode() {
    let mut h = harness();
    teacher(&mut h);
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert!(h.state().show.is_none());
}

#[test]
fn the_overlay_pen_and_laser_shortcuts_are_off_in_teacher_mode() {
    let mut h = harness();
    teacher(&mut h);
    h.key_press_modifiers(Modifiers::COMMAND, Key::P);
    h.key_press_modifiers(Modifiers::COMMAND, Key::L);
    h.run_steps(2);
    let show = h.state().show.as_ref().unwrap();
    assert!(show.blank_name().is_none() && show.teacher.is_some());
    assert!(!show.pen_on() && !show.laser_on());
}

#[test]
fn advancing_clears_marks_but_keeps_timers() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).add_timer();
    overlay(&mut h).set_tool(Tool::Pen);
    drag(&mut h, pos2(300.0, 300.0), pos2(500.0, 360.0));
    assert_eq!(overlay(&mut h).strokes.len(), 1, "the drag drew a stroke");
    assert_eq!(slide(&h), Some(0), "drawing did not turn the page");
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    assert_eq!(slide(&h), Some(1));
    let o = overlay(&mut h);
    assert!(o.strokes.is_empty());
    assert_eq!(o.timers.len(), 1);
}

#[test]
fn the_keep_setting_holds_marks_across_slides() {
    let mut h = harness();
    h.state_mut().run("show.teacherKeep", json!({"on": true})).unwrap();
    teacher(&mut h);
    overlay(&mut h).set_tool(Tool::Pen);
    drag(&mut h, pos2(300.0, 300.0), pos2(500.0, 360.0));
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    assert_eq!(slide(&h), Some(1));
    assert_eq!(overlay(&mut h).strokes.len(), 1);
}

#[test]
fn a_click_turns_the_page_with_the_pointer_but_not_with_a_drawing_tool() {
    let mut h = harness();
    teacher(&mut h);
    let at = pos2(600.0, 300.0);
    let click = |h: &mut egui_kittest::Harness<'static, SlideApp>| {
        h.event(Event::PointerMoved(at));
        h.run_steps(1);
        for pressed in [true, false] {
            h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
            h.run_steps(1);
        }
        h.run_steps(2);
    };
    overlay(&mut h).set_tool(Tool::Pen);
    click(&mut h);
    assert_eq!(slide(&h), Some(0), "a pen tap leaves a dot, not a page turn");
    assert_eq!(overlay(&mut h).strokes.len(), 1);
    overlay(&mut h).set_tool(Tool::Pointer);
    click(&mut h);
    assert_eq!(slide(&h), Some(1), "the pointer tool clicks through to the show");
}

#[test]
fn undo_takes_back_the_last_stroke() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).set_tool(Tool::Pen);
    drag(&mut h, pos2(300.0, 300.0), pos2(500.0, 360.0));
    drag(&mut h, pos2(300.0, 500.0), pos2(500.0, 560.0));
    assert_eq!(overlay(&mut h).strokes.len(), 2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(2);
    assert_eq!(overlay(&mut h).strokes.len(), 1);
}

#[test]
fn the_revealer_edge_drags_and_survives_until_the_slide_changes() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).set_tool(Tool::Revealer);
    h.run_steps(2);
    assert_eq!(overlay(&mut h).revealer.map(|r| r.cover), Some(1.0));
    // The shade starts over everything, so its edge is at the top of the screen.
    drag(&mut h, pos2(600.0, 4.0), pos2(600.0, 200.0));
    let cover = overlay(&mut h).revealer.map(|r| r.cover).unwrap();
    assert!((0.7..0.8).contains(&cover), "{cover}");
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    assert!(overlay(&mut h).revealer.is_none());
}

#[test]
fn the_eraser_removes_ink_under_the_pointer() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).set_tool(Tool::Pen);
    drag(&mut h, pos2(300.0, 300.0), pos2(500.0, 300.0));
    overlay(&mut h).set_tool(Tool::Eraser);
    drag(&mut h, pos2(400.0, 290.0), pos2(420.0, 290.0));
    assert!(overlay(&mut h).strokes.is_empty());
}

fn click_at(h: &mut egui_kittest::Harness<'static, SlideApp>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
    h.run_steps(2);
}

fn tab_centre(h: &egui_kittest::Harness<'static, SlideApp>) -> Pos2 {
    h.ctx.memory(|m| m.area_rect(egui::Id::new("teacher_palette_tab"))).expect("the tab is drawn").center()
}

#[test]
fn the_palette_starts_away_and_the_left_tab_brings_it_out_and_puts_it_back() {
    let mut h = harness();
    teacher(&mut h);
    let palette = |h: &egui_kittest::Harness<'static, SlideApp>| {
        h.ctx.memory(|m| m.areas().visible_last_frame(&egui::LayerId::new(egui::Order::Foreground, egui::Id::new("teacher_palette"))))
    };
    assert!(!palette(&h), "out of the way on entry");
    let tab = h.ctx.memory(|m| m.area_rect(egui::Id::new("teacher_palette_tab"))).expect("tab");
    assert!(tab.min.x < 12.0 && tab.center().x < 40.0, "the tab sits on the left edge");
    for _ in 0..(5 * 60) {
        h.step();
    }
    assert!(!palette(&h), "it does not appear by itself either");
    let c = tab_centre(&h);
    click_at(&mut h, c);
    assert!(palette(&h), "the tab brings the palette out");
    for _ in 0..(5 * 60) {
        h.step();
    }
    assert!(palette(&h), "and it stays out while idle");
    let c = tab_centre(&h);
    click_at(&mut h, c);
    assert!(!palette(&h), "the tab puts it away again");
}

#[test]
fn a_new_timer_opens_its_dials_and_start_runs_the_dialled_time() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).add_timer();
    h.run_steps(3);
    assert_eq!(overlay(&mut h).timers[0].setup, Some([0, 5, 0]), "five minutes is the quick setting");
    h.get_by_label("10").click();
    h.run_steps(3);
    h.get_by_label("Start").click();
    h.run_steps(3);
    let t = &overlay(&mut h).timers[0];
    assert!(t.running && t.setup.is_none());
    assert_eq!(t.duration, 600.0);
}

#[test]
fn the_timer_is_a_small_card_not_a_bar_across_the_screen() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).add_timer();
    h.run_steps(3);
    let setting = h.ctx.memory(|m| m.area_rect(egui::Id::new(("teacher_timer", 1u64)))).expect("timer card");
    assert!(setting.width() < 320.0 && setting.height() < 220.0, "setting: {setting:?}");
    h.get_by_label("Start").click();
    h.run_steps(3);
    let running = h.ctx.memory(|m| m.area_rect(egui::Id::new(("teacher_timer", 1u64)))).expect("timer card");
    assert!(running.width() < 320.0 && running.height() < 160.0, "running: {running:?}");
}
