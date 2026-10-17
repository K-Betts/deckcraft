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
    let o = overlay(&mut h);
    assert!(o.strokes.len() == 2, "the stroke is split around the gap, not removed");
    assert!(o.strokes.iter().all(|s| s.points.iter().all(|p| p.x < 385.0 || p.x > 435.0)), "ink under the eraser is gone");
}

fn touch(h: &mut egui_kittest::Harness<'static, SlideApp>, id: u64, phase: egui::TouchPhase, pos: Pos2) {
    h.event(Event::Touch { device_id: egui::TouchDeviceId(0), id: egui::TouchId(id), phase, pos, force: None });
}

#[test]
fn four_fingers_wipe_ink_even_with_the_pointer_tool() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).set_tool(Tool::Pen);
    drag(&mut h, pos2(300.0, 300.0), pos2(700.0, 300.0));
    overlay(&mut h).set_tool(Tool::Pointer);
    assert!(!overlay(&mut h).strokes.is_empty());
    let hand = |dx: f32| [pos2(480.0 + dx, 300.0), pos2(510.0 + dx, 290.0), pos2(540.0 + dx, 290.0), pos2(570.0 + dx, 300.0)];
    for (i, p) in hand(0.0).into_iter().enumerate() {
        touch(&mut h, i as u64, egui::TouchPhase::Start, p);
    }
    h.run_steps(2);
    for (i, p) in hand(30.0).into_iter().enumerate() {
        touch(&mut h, i as u64, egui::TouchPhase::Move, p);
    }
    h.run_steps(2);
    for (i, p) in hand(30.0).into_iter().enumerate() {
        touch(&mut h, i as u64, egui::TouchPhase::End, p);
    }
    h.run_steps(2);
    assert!(overlay(&mut h).strokes.iter().all(|s| s.points.iter().all(|p| p.x < 460.0 || p.x > 620.0)), "the swept patch is clean");
    assert!(overlay(&mut h).strokes.len() == 2, "ink either side of the hand stays");
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
    assert!(setting.width() < 420.0 && setting.height() < 340.0, "setting: {setting:?}");
    h.get_by_label("Start").click();
    h.run_steps(3);
    let running = h.ctx.memory(|m| m.area_rect(egui::Id::new(("teacher_timer", 1u64)))).expect("timer card");
    assert!(running.width() < 520.0 && running.height() < 260.0, "running: {running:?}");
}

#[test]
fn maths_tools_appear_in_teacher_mode_only_with_the_maths_pack_on() {
    let mut h = harness();
    teacher(&mut h);
    assert!(!overlay(&mut h).maths, "a fresh install is subject-neutral");
    h.state_mut().run("app.pack", json!({"id": "maths", "on": true})).unwrap();
    h.run_steps(2);
    assert!(overlay(&mut h).maths);
    h.state_mut().run("app.pack", json!({"id": "maths", "on": false})).unwrap();
    h.run_steps(2);
    assert!(!overlay(&mut h).maths, "switching it off puts the palette back as it was");
}

#[test]
fn palette_controls_are_at_least_a_fingertip_wide() {
    const { assert!(crate::teacher_overlay::TOUCH_TARGET >= 44.0) };
}

#[test]
fn a_running_timer_is_digits_only_until_it_is_tapped_or_paused() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).add_timer();
    h.run_steps(3);
    h.get_by_label("Start").click();
    h.run_steps(3);
    assert!(overlay(&mut h).timers[0].running);
    h.event(Event::PointerMoved(pos2(1100.0, 700.0)));
    h.run_steps(3);
    assert!(h.query_by_label("Reset").is_none(), "no chrome while it counts");
    assert!(h.query_by_label("Pause").is_none());
    overlay(&mut h).timers[0].toggle();
    h.run_steps(3);
    assert!(h.query_by_label("Resume").is_some(), "paused brings the controls back");
    assert!(h.query_by_label("Reset").is_some());
}

#[test]
fn the_digits_turn_amber_then_red_and_grow_with_the_screen() {
    use crate::teacher_overlay::{timer_digit_size, timer_ink};
    let white = egui::Color32::WHITE;
    assert_eq!(timer_ink(120.0, true), white);
    assert_ne!(timer_ink(30.0, true), white);
    assert_ne!(timer_ink(10.0, true), timer_ink(30.0, true), "red is not amber");
    assert_eq!(timer_ink(5.0, false), white, "a paused timer keeps its calm colour");
    assert!(timer_digit_size(1080.0) > timer_digit_size(540.0));
    assert!(timer_digit_size(300.0) >= 72.0 && timer_digit_size(5000.0) <= 200.0);
}

#[test]
fn timer_digits_stay_on_one_line_for_every_value() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).add_timer();
    h.run_steps(3);
    h.get_by_label("Start").click();
    h.run_steps(3);
    let id = egui::Id::new(("teacher_timer", 1u64));
    let tall = h.ctx.memory(|m| m.area_rect(id)).unwrap().height();
    overlay(&mut h).timers[0].remaining = 48.0;
    h.run_steps(3);
    let later = h.ctx.memory(|m| m.area_rect(id)).unwrap().height();
    assert_eq!(tall, later, "5:00 and 0:48 occupy the same box (the old row wrapped to two lines)");
}

#[test]
#[ignore = "writes screenshots for review"]
fn screenshot_timer() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).add_timer();
    h.run_steps(4);
    h.render().unwrap().save("/Users/kieranbetts/maths-office/deckcraft-ui/docs/design/img/teacher-after-timer.png").unwrap();
}
