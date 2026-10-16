//! Maths teaching aids displayed as transient windows over Teacher Mode.

use std::collections::BTreeSet;

use egui::{Align2, Color32, CornerRadius, Id, Order, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MathsTool {
    Ruler,
    Protractor,
    Compass,
    NumberLine,
    CoordinateGrid,
    FractionWall,
    AlgebraTiles,
    HundredSquare,
    Calculator,
}

impl MathsTool {
    pub const ALL: [Self; 9] = [
        Self::Ruler,
        Self::Protractor,
        Self::Compass,
        Self::NumberLine,
        Self::CoordinateGrid,
        Self::FractionWall,
        Self::AlgebraTiles,
        Self::HundredSquare,
        Self::Calculator,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ruler => "Ruler",
            Self::Protractor => "Protractor",
            Self::Compass => "Compass",
            Self::NumberLine => "Number line",
            Self::CoordinateGrid => "Coordinate grid",
            Self::FractionWall => "Fraction wall",
            Self::AlgebraTiles => "Algebra tiles",
            Self::HundredSquare => "Hundred square",
            Self::Calculator => "Calculator",
        }
    }
}

/// Preferences for the current local teacher account; never part of a presentation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MathsToolsSettings {
    pub enabled: BTreeSet<MathsTool>,
}

impl Default for MathsToolsSettings {
    fn default() -> Self {
        Self { enabled: [MathsTool::Ruler, MathsTool::NumberLine].into_iter().collect() }
    }
}

impl MathsToolsSettings {
    pub fn is_enabled(&self, tool: MathsTool) -> bool {
        self.enabled.contains(&tool)
    }

    pub fn set_enabled(&mut self, tool: MathsTool, enabled: bool) {
        if enabled {
            self.enabled.insert(tool);
        } else {
            self.enabled.remove(&tool);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NumberLineRange {
    start: i32,
    end: i32,
    step: i32,
}

impl Default for NumberLineRange {
    fn default() -> Self {
        Self { start: -5, end: 5, step: 1 }
    }
}

#[derive(Default)]
pub struct MathsToolsPanel {
    pub open: bool,
    open_tools: BTreeSet<MathsTool>,
    number_line: NumberLineRange,
}

impl MathsToolsPanel {
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn has_visible_widgets(&self) -> bool {
        self.open || !self.open_tools.is_empty()
    }

    pub fn ui(&mut self, ctx: &egui::Context, canvas: Rect, settings: &mut MathsToolsSettings) -> Vec<Rect> {
        let mut rects = Vec::new();
        self.open_tools.retain(|tool| settings.is_enabled(*tool));
        if self.open {
            let mut open = self.open;
            let shown = egui::Window::new("Maths tools")
                .id(Id::new("maths_tools_panel"))
                .order(Order::Foreground)
                .default_pos(canvas.min + vec2(24.0, 84.0))
                .default_width(260.0)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Enabled tools");
                    ui.horizontal_wrapped(|ui| {
                        for tool in MathsTool::ALL {
                            if settings.is_enabled(tool) && ui.button(format!("Open {}", tool.label())).clicked() {
                                self.open_tools.insert(tool);
                            }
                        }
                    });
                    ui.separator();
                    ui.label("Choose which tools are available");
                    for tool in MathsTool::ALL {
                        let mut enabled = settings.is_enabled(tool);
                        if ui.checkbox(&mut enabled, tool.label()).changed() {
                            settings.set_enabled(tool, enabled);
                            if !enabled {
                                self.open_tools.remove(&tool);
                            }
                        }
                    }
                });
            self.open = open;
            if let Some(shown) = shown {
                rects.push(shown.response.rect);
            }
        }

        for tool in MathsTool::ALL {
            if !self.open_tools.contains(&tool) {
                continue;
            }
            let mut open = true;
            let shown = egui::Window::new(tool.label())
                .id(Id::new(("maths_tool", tool)))
                .order(Order::Foreground)
                .default_pos(canvas.center() - vec2(180.0, 70.0))
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| match tool {
                    MathsTool::Ruler => draw_ruler(ui),
                    MathsTool::NumberLine => draw_number_line(ui, &mut self.number_line),
                    _ => {
                        ui.label(format!("{} tool coming soon.", tool.label()));
                    }
                });
            if let Some(shown) = shown {
                rects.push(shown.response.rect);
            }
            if !open {
                self.open_tools.remove(&tool);
            }
        }
        rects
    }
}

fn draw_ruler(ui: &mut egui::Ui) {
    ui.label("Centimetres");
    let (rect, _) = ui.allocate_exact_size(vec2(324.0, 68.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(255, 248, 204));
    painter.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, Color32::from_gray(90)), StrokeKind::Inside);

    let left = rect.left() + 12.0;
    let width = rect.width() - 24.0;
    for tenth in 0..=100 {
        let x = left + width * tenth as f32 / 100.0;
        let height = match tenth % 10 {
            0 => 25.0,
            5 => 17.0,
            _ => 10.0,
        };
        painter.line_segment([pos2(x, rect.top()), pos2(x, rect.top() + height)], Stroke::new(1.0, Color32::DARK_GRAY));
        if tenth % 10 == 0 {
            painter.text(pos2(x, rect.top() + 31.0), Align2::CENTER_TOP, (tenth / 10).to_string(), egui::FontId::proportional(11.0), Color32::BLACK);
        }
    }
    painter.text(pos2(rect.right() - 8.0, rect.bottom() - 4.0), Align2::RIGHT_BOTTOM, "cm", egui::FontId::proportional(10.0), Color32::DARK_GRAY);
}

fn draw_number_line(ui: &mut egui::Ui, range: &mut NumberLineRange) {
    ui.horizontal(|ui| {
        ui.label("From");
        ui.add(egui::DragValue::new(&mut range.start).range(-100..=99));
        ui.label("to");
        ui.add(egui::DragValue::new(&mut range.end).range(-99..=100));
        ui.label("step");
        ui.add(egui::DragValue::new(&mut range.step).range(1..=20));
    });
    if range.end <= range.start {
        range.end = range.start.saturating_add(1).min(100);
        range.start = range.start.min(range.end - 1);
    }

    let (rect, _) = ui.allocate_exact_size(vec2(380.0, 84.0), Sense::hover());
    let painter = ui.painter();
    let y = rect.center().y - 8.0;
    let left = rect.left() + 18.0;
    let right = rect.right() - 18.0;
    painter.line_segment([pos2(left, y), pos2(right, y)], Stroke::new(2.0, Color32::from_gray(50)));
    painter.line_segment([pos2(left, y), pos2(left + 7.0, y - 5.0)], Stroke::new(2.0, Color32::from_gray(50)));
    painter.line_segment([pos2(left, y), pos2(left + 7.0, y + 5.0)], Stroke::new(2.0, Color32::from_gray(50)));
    painter.line_segment([pos2(right, y), pos2(right - 7.0, y - 5.0)], Stroke::new(2.0, Color32::from_gray(50)));
    painter.line_segment([pos2(right, y), pos2(right - 7.0, y + 5.0)], Stroke::new(2.0, Color32::from_gray(50)));

    let span = (range.end - range.start).max(1) as f32;
    for value in range.start..=range.end {
        let x = left + (right - left) * (value - range.start) as f32 / span;
        let major = (value - range.start) % range.step == 0;
        let tick = if major { 13.0 } else { 6.0 };
        painter.line_segment([pos2(x, y - tick / 2.0), pos2(x, y + tick / 2.0)], Stroke::new(1.5, Color32::from_gray(50)));
        if major {
            painter.text(pos2(x, y + 12.0), Align2::CENTER_TOP, value.to_string(), egui::FontId::proportional(11.0), Color32::BLACK);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use egui_kittest::kittest::Queryable;

    struct State {
        panel: MathsToolsPanel,
        settings: MathsToolsSettings,
    }

    #[test]
    fn tool_toggles_change_enabled_state() {
        let mut settings = MathsToolsSettings::default();
        assert!(settings.is_enabled(MathsTool::Ruler));
        assert!(settings.is_enabled(MathsTool::NumberLine));
        assert!(!settings.is_enabled(MathsTool::Compass));

        settings.set_enabled(MathsTool::Compass, true);
        assert!(settings.is_enabled(MathsTool::Compass));
        settings.set_enabled(MathsTool::Ruler, false);
        assert!(!settings.is_enabled(MathsTool::Ruler));
    }

    #[test]
    fn toggling_a_tool_changes_its_panel_button() {
        let state = State { panel: MathsToolsPanel { open: true, ..Default::default() }, settings: MathsToolsSettings::default() };
        let mut harness = egui_kittest::Harness::builder().with_size(vec2(900.0, 600.0)).build_ui_state(
            |ui, state: &mut State| {
                state.panel.ui(ui.ctx(), ui.max_rect(), &mut state.settings);
            },
            state,
        );
        harness.run_steps(2);
        assert!(harness.query_by_label("Open Compass").is_none());
        harness.get_by_label("Compass").click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Open Compass").is_some());
        harness.get_by_label("Ruler").click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Open Ruler").is_none());
    }

    #[test]
    fn panel_offers_only_enabled_tools() {
        let settings = MathsToolsSettings { enabled: [MathsTool::Ruler].into_iter().collect() };
        let state = State { panel: MathsToolsPanel { open: true, ..Default::default() }, settings };
        let mut harness = egui_kittest::Harness::builder().with_size(vec2(900.0, 600.0)).build_ui_state(
            |ui, state: &mut State| {
                state.panel.ui(ui.ctx(), ui.max_rect(), &mut state.settings);
            },
            state,
        );
        harness.run_steps(2);
        assert!(harness.query_by_label("Open Ruler").is_some());
        assert!(harness.query_by_label("Open Number line").is_none());
        assert!(harness.query_by_label("Open Compass").is_none());
    }

    #[test]
    fn enabled_ruler_opens_as_a_floating_tool() {
        let state = State {
            panel: MathsToolsPanel { open: true, ..Default::default() },
            settings: MathsToolsSettings { enabled: [MathsTool::Ruler].into_iter().collect() },
        };
        let mut harness = egui_kittest::Harness::builder().with_size(vec2(900.0, 600.0)).build_ui_state(
            |ui, state: &mut State| {
                state.panel.ui(ui.ctx(), ui.max_rect(), &mut state.settings);
            },
            state,
        );
        harness.run_steps(2);
        harness.get_by_label("Open Ruler").click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Centimetres").is_some());
    }
}
