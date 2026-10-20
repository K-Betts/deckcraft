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

fn click(h: &mut egui_kittest::Harness<'static, SlideApp>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
    h.run_steps(2);
}

#[test]
fn the_arrow_on_the_left_edge_brings_the_tools_out() {
    let mut h = harness();
    teacher(&mut h);
    assert!(!overlay(&mut h).palette_open);
    click(&mut h, pos2(14.0, 400.0));
    assert!(overlay(&mut h).palette_open, "clicking the arrow brings the tools out");
}

#[test]
fn a_tools_colours_pop_out_when_it_is_chosen_and_tuck_away_after_a_pick() {
    let mut o = TeacherOverlay::new();
    assert!(!o.options_open);
    o.set_tool(Tool::Pen);
    assert!(o.options_open, "choosing the pen shows its colours");
    // Picking a colour tucks them away (the palette code does this on a swatch click).
    o.options_open = false;
    o.set_tool(Tool::Pen);
    assert!(o.options_open, "choosing the pen again brings them back");
    o.set_tool(Tool::Pen);
    assert!(!o.options_open, "and once more puts them away");
    o.set_tool(Tool::Highlighter);
    assert!(o.options_open, "a different tool opens its own");
}

#[test]
fn clicking_a_colour_in_the_running_palette_closes_the_options() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).palette_open = true;
    overlay(&mut h).set_tool(Tool::Pen);
    h.run_steps(3);
    let before = overlay(&mut h).pen_colour;
    let swatch = h.get_all_by_label("Blue").next().expect("a blue swatch");
    swatch.click();
    h.run_steps(3);
    assert!(!overlay(&mut h).options_open, "the colours tuck away after a pick");
    assert_ne!(overlay(&mut h).pen_colour, before);
}

#[test]
fn the_maths_tools_button_is_on_the_toolbar_once() {
    let mut h = harness();
    h.state_mut().run("app.pack", json!({"id": "maths", "on": true})).unwrap();
    teacher(&mut h);
    overlay(&mut h).palette_open = true;
    h.run_steps(3);
    assert_eq!(h.query_all_by_label("Maths tools").count(), 1);
}

#[test]
fn writing_is_still_there_after_leaving_teacher_mode_and_coming_back() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).set_tool(Tool::Pen);
    drag(&mut h, pos2(300.0, 300.0), pos2(500.0, 360.0));
    assert_eq!(overlay(&mut h).strokes.len(), 1);
    h.state_mut().run("show.end", json!({})).unwrap();
    h.run_steps(2);
    assert!(h.state().show.is_none());
    teacher(&mut h);
    assert_eq!(overlay(&mut h).strokes.len(), 1, "the writing came back with the slide");
}

#[test]
fn the_toolbar_tucks_away_when_left_alone_and_a_double_tapped_grip_keeps_it_out() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).auto_hide_secs = 0.5;
    overlay(&mut h).open_palette();
    overlay(&mut h).set_tool(Tool::Pen);
    h.event(Event::PointerMoved(pos2(1100.0, 700.0)));
    h.run_steps(10);
    assert!(overlay(&mut h).palette_open, "not yet: it was only just left alone");
    h.run_steps(60);
    assert!(!overlay(&mut h).palette_open, "left alone for a while, the toolbar tucks away");
    assert!(!overlay(&mut h).options_open, "and takes the pen's colours with it");
    assert_eq!(overlay(&mut h).tool, Tool::Pen, "the pen is still the pen");
    // Brought back and kept: it stays however long it is left.
    overlay(&mut h).open_palette();
    overlay(&mut h).pinned = true;
    h.run_steps(90);
    assert!(overlay(&mut h).palette_open);
}

#[test]
fn using_the_toolbar_holds_off_the_tuck_away() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).auto_hide_secs = 0.5;
    overlay(&mut h).open_palette();
    h.run_steps(2);
    // The pointer rests on the toolbar: that is using it.
    let on_bar = overlay(&mut h).palette_rect_for_test().center();
    h.event(Event::PointerMoved(on_bar));
    h.run_steps(90);
    assert!(overlay(&mut h).palette_open);
}

#[test]
fn the_pens_choices_pop_out_beside_the_tool_and_drawing_puts_them_away() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).open_palette();
    overlay(&mut h).set_tool(Tool::Pen);
    h.run_steps(3);
    assert!(overlay(&mut h).options_open);
    let pop = h.ctx.memory(|m| m.area_rect(egui::Id::new("teacher_options"))).expect("the popover");
    let bar = overlay(&mut h).palette_rect_for_test();
    assert!(pop.min.x >= bar.max.x, "beside the toolbar, not inside it: {pop:?} vs {bar:?}");
    assert!(pop.height() < 80.0, "one row, as in the mock-up");
    // Choosing a width leaves it for the colour; drawing closes it.
    drag(&mut h, pos2(600.0, 400.0), pos2(800.0, 400.0));
    assert!(!overlay(&mut h).options_open);
    h.run_steps(2);
    assert!(h.ctx.memory(|m| m.area_rect(egui::Id::new("teacher_options"))).is_none() || !overlay(&mut h).options_open);
}

#[test]
fn a_timer_can_be_made_bigger_or_smaller_and_the_next_one_keeps_the_size() {
    use crate::teacher_overlay::{TIMER_SCALE_MAX, TIMER_SCALE_MIN};
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).add_timer();
    h.run_steps(3);
    h.get_by_label("Start").click();
    h.run_steps(3);
    overlay(&mut h).timers[0].toggle();
    h.run_steps(3);
    let size = |h: &egui_kittest::Harness<'static, SlideApp>| h.ctx.memory(|m| m.area_rect(egui::Id::new(("teacher_timer", 1u64)))).unwrap().height();
    let before = size(&h);
    h.get_by_label("Smaller digits").click();
    h.run_steps(3);
    h.get_by_label("Smaller digits").click();
    h.run_steps(3);
    assert!(overlay(&mut h).timers[0].scale < 0.8, "{}", overlay(&mut h).timers[0].scale);
    assert!(size(&h) < before, "the card follows the digits (the buttons keep the width)");
    overlay(&mut h).add_timer();
    assert!(overlay(&mut h).timers[1].scale < 0.8, "a new timer starts at the size the teacher last chose");
    // It cannot be lost off either end.
    let t = &mut overlay(&mut h).timers[0];
    for _ in 0..40 {
        t.resize(1.2);
    }
    assert_eq!(t.scale, TIMER_SCALE_MAX);
    for _ in 0..80 {
        t.resize(1.0 / 1.2);
    }
    assert_eq!(t.scale, TIMER_SCALE_MIN);
}

#[test]
fn dragging_a_ruler_moves_it_and_does_not_draw() {
    use crate::maths_tools::MathsTool;
    let mut h = harness();
    h.state_mut().run("app.pack", json!({"id": "maths", "on": true})).unwrap();
    h.state_mut().maths_tools.enabled = MathsTool::ALL.into_iter().collect();
    teacher(&mut h);
    overlay(&mut h).set_tool(Tool::Pen);
    overlay(&mut h).maths_tools_panel.open_tool(MathsTool::Ruler);
    h.run_steps(3);
    let centre = overlay(&mut h).maths_tools_panel.centre_of(MathsTool::Ruler).expect("it is on the board");
    drag(&mut h, centre, centre + egui::vec2(120.0, 90.0));
    let moved = overlay(&mut h).maths_tools_panel.centre_of(MathsTool::Ruler).unwrap();
    assert!(((moved - centre) - egui::vec2(120.0, 90.0)).length() < 8.0, "{centre:?} -> {moved:?}");
    assert!(overlay(&mut h).strokes.is_empty(), "the drag was the ruler's, not the pen's");
    // Clear board still takes ink.
    drag(&mut h, pos2(300.0, 600.0), pos2(500.0, 600.0));
    assert_eq!(overlay(&mut h).strokes.len(), 1);
}

#[test]
fn typing_goes_to_the_calculator_only_while_the_pointer_is_over_it() {
    use crate::maths_tools::MathsTool;
    let mut h = harness();
    h.state_mut().run("app.pack", json!({"id": "maths", "on": true})).unwrap();
    h.state_mut().maths_tools.enabled = MathsTool::ALL.into_iter().collect();
    teacher(&mut h);
    overlay(&mut h).maths_tools_panel.open_tool(MathsTool::Calculator);
    h.run_steps(3);
    let rect = h.ctx.memory(|m| m.area_rect(egui::Id::new(("maths_tool", MathsTool::Calculator)))).expect("the calculator");
    h.event(Event::PointerMoved(rect.center()));
    h.run_steps(3);
    assert!(overlay(&mut h).keyboard_captured());
    h.key_press(Key::Num5);
    h.event(Event::Text("5".into()));
    h.run_steps(3);
    assert_eq!(overlay(&mut h).maths_tools_panel.calculator().calc.tokens.len(), 1, "the 5 went to the calculator");
    assert_eq!(slide(&h), Some(0), "and nothing else acted on the key");
    h.key_press(Key::Enter);
    h.run_steps(3);
    assert_eq!(slide(&h), Some(0), "Enter is the calculator's equals here, not 'next slide'");
    // Off the calculator the show has its keys back.
    h.event(Event::PointerMoved(pos2(300.0, 600.0)));
    h.run_steps(3);
    assert!(!overlay(&mut h).keyboard_captured());
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    assert_eq!(slide(&h), Some(1));
}

#[test]
fn the_whiteboard_is_a_blank_page_with_its_own_marks_and_the_slide_comes_back_as_it_was() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).set_tool(Tool::Pen);
    drag(&mut h, pos2(300.0, 300.0), pos2(500.0, 300.0));
    assert_eq!(overlay(&mut h).strokes.len(), 1);
    overlay(&mut h).set_whiteboard(true);
    h.run_steps(2);
    assert!(overlay(&mut h).strokes.is_empty(), "a fresh page: the slide's ink is put by");
    drag(&mut h, pos2(300.0, 400.0), pos2(500.0, 400.0));
    drag(&mut h, pos2(300.0, 500.0), pos2(500.0, 500.0));
    assert_eq!(overlay(&mut h).strokes.len(), 2);
    overlay(&mut h).set_whiteboard(false);
    assert_eq!(overlay(&mut h).strokes.len(), 1, "back on the slide with its one line");
    overlay(&mut h).set_whiteboard(true);
    assert_eq!(overlay(&mut h).strokes.len(), 2, "and the board kept what was written on it");
}

#[test]
fn the_whiteboard_button_toggles_and_the_arrows_bring_the_slide_back_instead_of_turning_it() {
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).open_palette();
    h.run_steps(3);
    h.get_by_label("Blank whiteboard").click();
    h.run_steps(3);
    assert!(overlay(&mut h).whiteboard.is_some());
    h.get_by_label("Next").click();
    h.run_steps(3);
    assert!(overlay(&mut h).whiteboard.is_none(), "Next returns to the slide");
    assert_eq!(slide(&h), Some(0), "without moving on");
}

#[test]
fn a_click_on_the_whiteboard_does_not_turn_the_page_and_choosing_black_swaps_a_black_pen() {
    use crate::teacher_overlay::Board;
    let mut h = harness();
    teacher(&mut h);
    overlay(&mut h).pen_colour = egui::Color32::BLACK;
    overlay(&mut h).set_whiteboard(true);
    click_at(&mut h, pos2(600.0, 400.0));
    assert_eq!(slide(&h), Some(0));
    overlay(&mut h).set_board_kind(Board::Black);
    assert_eq!(overlay(&mut h).whiteboard, Some(Board::Black));
    assert_eq!(overlay(&mut h).pen_colour, egui::Color32::WHITE);
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    assert_eq!(slide(&h), Some(1), "a key still turns the slide, which also leaves the board");
    assert!(overlay(&mut h).whiteboard.is_none());
}

#[test]
fn picking_a_maths_tool_closes_the_chooser_and_the_bar_is_gone_within_two_seconds() {
    use crate::maths_tools::MathsTool;
    let mut h = harness();
    h.state_mut().run("app.pack", json!({"id": "maths", "on": true})).unwrap();
    h.state_mut().maths_tools.enabled = MathsTool::ALL.into_iter().collect();
    teacher(&mut h);
    overlay(&mut h).open_palette();
    overlay(&mut h).maths_tools_panel.open = true;
    h.run_steps(3);
    h.get_by_label("Calculator").click();
    h.run_steps(3);
    assert!(!overlay(&mut h).maths_tools_panel.open, "the chooser closes as soon as a tool is picked");
    assert!(overlay(&mut h).palette_open, "the bar waits a moment");
    // The pointer resting on the bar does not keep it: two seconds at the most.
    let on_bar = overlay(&mut h).palette_rect_for_test().center();
    h.event(Event::PointerMoved(on_bar));
    h.run_steps(60);
    assert!(overlay(&mut h).palette_open, "a second in, still there");
    h.run_steps(90);
    assert!(!overlay(&mut h).palette_open, "gone after two");
    assert!(overlay(&mut h).maths_tools_panel.has_visible_widgets(), "the calculator stays");
}

/// Writes pictures of the teacher overlay to `$DECKCRAFT_SHOTS` for a human to look at:
/// `DECKCRAFT_SHOTS=/some/dir cargo test -p deckcraft-ui-egui --lib screenshots_for_review -- --ignored`.
#[test]
#[ignore = "writes screenshots for review"]
fn screenshots_for_review() {
    use crate::calculator::Key;
    use crate::maths_tools::MathsTool;
    let dir = std::path::PathBuf::from(std::env::var("DECKCRAFT_SHOTS").unwrap_or_else(|_| "out".into()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = harness();
    h.state_mut().run("app.pack", json!({"id": "maths", "on": true})).unwrap();
    h.state_mut().maths_tools.enabled = MathsTool::ALL.into_iter().collect();
    teacher(&mut h);
    let shot = |h: &mut egui_kittest::Harness<'static, SlideApp>, name: &str| {
        h.run_steps(4);
        h.render().unwrap().save(dir.join(name)).unwrap();
    };
    // The toolbar out with the pen's choices beside it.
    overlay(&mut h).open_palette();
    overlay(&mut h).set_tool(Tool::Pen);
    shot(&mut h, "1-pen-popover.png");
    // Then tucked away, only the pen's tab left.
    overlay(&mut h).collapse_palette();
    shot(&mut h, "2-collapsed.png");
    // Instruments and the calculator on the board, with a timer.
    {
        let o = overlay(&mut h);
        o.open_palette();
        o.set_tool(Tool::Pointer);
        o.maths_tools_panel.open_tool(MathsTool::Ruler);
        o.maths_tools_panel.open_tool(MathsTool::Protractor);
        o.maths_tools_panel.open_tool(MathsTool::Calculator);
        let calc = o.maths_tools_panel.calculator();
        for k in [
            Key::Digit(1),
            Key::Frac,
            Key::Digit(3),
            Key::Right,
            Key::Right,
            Key::Plus,
            Key::Sqrt,
            Key::Digit(1),
            Key::Digit(2),
            Key::Right,
            Key::Equals,
        ] {
            calc.press(k);
        }
        o.add_timer();
    }
    shot(&mut h, "3-board.png");
    {
        let o = overlay(&mut h);
        o.maths_tools_panel.open = true;
        o.timers[0].setup = None;
        o.timers[0].running = true;
        o.timers[0].scale = 0.4;
    }
    shot(&mut h, "4-chooser-small-timer.png");
    // Turned instruments, a compass and the grid and number line cards.
    {
        let o = overlay(&mut h);
        o.maths_tools_panel = Default::default();
        o.collapse_palette();
        let m = &mut o.maths_tools_panel;
        m.set_pose(MathsTool::Ruler, pos2(520.0, 330.0), 0.35, 1.0);
        m.set_pose(MathsTool::Protractor, pos2(360.0, 600.0), -0.2, 0.9);
        m.set_pose(MathsTool::Compass, pos2(820.0, 560.0), -0.6, 1.0);
        m.open_tool(MathsTool::CoordinateGrid);
        m.open_tool(MathsTool::NumberLine);
        o.timers.clear();
    }
    shot(&mut h, "5-instruments-cards.png");
    // The calculator's home menu and a solved table, then the whiteboard.
    {
        let o = overlay(&mut h);
        o.maths_tools_panel = Default::default();
        o.maths_tools_panel.open_tool(MathsTool::Calculator);
        o.maths_tools_panel.calculator().press(Key::Home);
    }
    shot(&mut h, "6-calc-home.png");
    {
        let d = overlay(&mut h).maths_tools_panel.calculator();
        d.press(Key::Digit(3));
        for k in [Key::Digit(2), Key::X, Key::Square, Key::Minus, Key::Digit(1)] {
            d.press(k);
        }
        for _ in 0..4 {
            d.press(Key::Ok);
        }
    }
    shot(&mut h, "7-calc-table.png");
    {
        let o = overlay(&mut h);
        o.maths_tools_panel = Default::default();
        o.maths_tools_panel.set_pose(MathsTool::Ruler, pos2(600.0, 420.0), 0.0, 1.0);
        o.open_palette();
        o.set_board_kind(crate::teacher_overlay::Board::Squared);
        o.set_whiteboard(true);
    }
    shot(&mut h, "8-whiteboard.png");
    // The calculator close up: a surd sum, an integral being typed, the CATALOG and SETTINGS menus.
    let calc_shot = |h: &mut egui_kittest::Harness<'static, SlideApp>, keys: &[Key], name: &str| {
        {
            let o = overlay(h);
            o.set_whiteboard(false);
            o.maths_tools_panel = Default::default();
            o.maths_tools_panel.open_tool(MathsTool::Calculator);
            let d = o.maths_tools_panel.calculator();
            for k in keys {
                d.press(*k);
            }
        }
        h.run_steps(4);
        h.render().unwrap().save(dir.join(name)).unwrap();
    };
    use Key::{Digit as D, Right};
    calc_shot(
        &mut h,
        &[D(1), D(0), Key::Sqrt, D(2), Right, Key::Plus, D(1), D(5), Key::Times, D(3), Key::Sqrt, D(3), Key::Equals],
        "9-calc-surds.png",
    );
    calc_shot(
        &mut h,
        &[Key::Integral, Key::Func(crate::calculator::Func::Ln), Key::X, Key::RParen, Right, D(1), Right, Key::E],
        "10-calc-integral.png",
    );
    calc_shot(&mut h, &[Key::Catalog], "11-calc-catalog.png");
    calc_shot(&mut h, &[Key::Settings, D(1)], "12-calc-settings.png");
    // The other apps.
    let app_shot = |h: &mut egui_kittest::Harness<'static, SlideApp>, n: usize, keys: &[Key], name: &str| {
        {
            let o = overlay(h);
            o.set_whiteboard(false);
            o.maths_tools_panel = Default::default();
            o.maths_tools_panel.open_tool(MathsTool::Calculator);
            let d = o.maths_tools_panel.calculator();
            d.press(Key::Home);
            d.home_sel = n;
            d.press(Key::Ok);
            for k in keys {
                d.press(*k);
            }
        }
        h.run_steps(4);
        h.render().unwrap().save(dir.join(name)).unwrap();
    };
    app_shot(&mut h, 1, &[D(1), D(1), Key::Ok, D(3), Key::Ok, D(5), Key::Ok, Key::Equals, D(1)], "13-stats-results.png");
    app_shot(&mut h, 9, &[D(4), Key::Ok, Key::EqSign, Key::Var(0), D(1), Key::Times, D(2), Key::Ok, D(7), Key::Right], "14-sheet.png");
    app_shot(&mut h, 12, &[Key::Tools, D(2), D(1), Key::Var(5), Key::Plus, D(1), Key::Equals], "15-basen.png");
    app_shot(&mut h, 3, &[D(1), D(1), D(1), D(2), Key::Ok, Key::Ok, Key::Ok], "16-equation-menu.png");
}
