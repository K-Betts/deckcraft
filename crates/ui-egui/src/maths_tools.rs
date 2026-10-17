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

const TILES_PER_ROW: usize = 3;
const TILE_W: f32 = 108.0;
const TILE_H: f32 = 52.0;

pub struct MathsToolsPanel {
    pub open: bool,
    open_tools: BTreeSet<MathsTool>,
    number_line: NumberLineRange,
    protractor_angle: f32,
    compass_radius: f32,
    coordinate_grid: CoordinateGridState,
    fraction_denominators: BTreeSet<usize>,
    algebra_tiles: Vec<AlgebraTile>,
    hundred_square: BTreeSet<u8>,
    calculator: CalculatorState,
}

impl Default for MathsToolsPanel {
    fn default() -> Self {
        Self {
            open: false,
            open_tools: BTreeSet::new(),
            number_line: NumberLineRange::default(),
            protractor_angle: 45.0,
            compass_radius: 50.0,
            coordinate_grid: CoordinateGridState::default(),
            fraction_denominators: [2, 3, 4, 5, 6, 8, 10, 12].into_iter().collect(),
            algebra_tiles: Vec::new(),
            hundred_square: BTreeSet::new(),
            calculator: CalculatorState::default(),
        }
    }
}

#[derive(Default)]
struct CoordinateGridState {
    x: i32,
    y: i32,
    points: Vec<(i32, i32)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AlgebraTile {
    X,
    NegativeX,
    Unit,
    NegativeUnit,
}

impl AlgebraTile {
    fn label(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::NegativeX => "-x",
            Self::Unit => "1",
            Self::NegativeUnit => "-1",
        }
    }

    fn colour(self) -> Color32 {
        match self {
            Self::X => Color32::from_rgb(79, 145, 212),
            Self::NegativeX => Color32::from_rgb(222, 112, 112),
            Self::Unit => Color32::from_rgb(102, 178, 126),
            Self::NegativeUnit => Color32::from_rgb(226, 173, 83),
        }
    }
}

#[derive(Clone)]
struct CalculatorState {
    display: String,
    accumulator: Option<f64>,
    operation: Option<char>,
    replace_input: bool,
}

impl Default for CalculatorState {
    fn default() -> Self {
        Self { display: "0".into(), accumulator: None, operation: None, replace_input: true }
    }
}

impl CalculatorState {
    fn press(&mut self, key: &str) {
        match key {
            "AC" => *self = Self::default(),
            "±" => {
                if let Ok(value) = self.display.parse::<f64>() {
                    self.display = format_number(-value);
                }
            }
            "." if self.replace_input => {
                self.display = "0.".into();
                self.replace_input = false;
            }
            "." if !self.display.contains('.') => self.display.push('.'),
            "." => {}
            "+" | "−" | "×" | "÷" => {
                if let Ok(value) = self.display.parse::<f64>() {
                    self.accumulator = Some(value);
                    self.operation = key.chars().next();
                    self.replace_input = true;
                }
            }
            "=" => {
                let Some(left) = self.accumulator else { return };
                let Ok(right) = self.display.parse::<f64>() else { return };
                let Some(operation) = self.operation.take() else { return };
                let result = match operation {
                    '+' => Some(left + right),
                    '−' => Some(left - right),
                    '×' => Some(left * right),
                    '÷' if right != 0.0 => Some(left / right),
                    '÷' => None,
                    _ => None,
                };
                self.display = result.filter(|n| n.is_finite()).map_or_else(|| "Error".into(), format_number);
                self.accumulator = None;
                self.replace_input = true;
            }
            _ if key.chars().all(|c| c.is_ascii_digit()) => {
                if self.replace_input || self.display == "0" || self.display == "Error" {
                    self.display = key.into();
                    self.replace_input = false;
                } else {
                    self.display.push_str(key);
                }
            }
            _ => {}
        }
    }
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1.0e15 {
        format!("{value:.0}")
    } else {
        format!("{value:.8}").trim_end_matches('0').trim_end_matches('.').to_string()
    }
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
                // Beside the palette (docked on the left by default), clear of the timers.
                .default_pos(canvas.min + vec2(88.0, canvas.height() * 0.25))
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    let chosen: Vec<MathsTool> = MathsTool::ALL.into_iter().filter(|t| settings.is_enabled(*t)).collect();
                    if chosen.is_empty() {
                        ui.label("No tools chosen yet. Open “Choose tools” below.");
                    }
                    // Big tiles, three across: tap to bring a tool onto the slide, tap again to put it away.
                    for row in chosen.chunks(TILES_PER_ROW) {
                        ui.horizontal(|ui| {
                            for tool in row {
                                let on = self.open_tools.contains(tool);
                                let tokens = crate::theme::Tokens::get(ui.ctx());
                                let (fill, ink) = if on { (tokens.accent, tokens.accent_text) } else { (tokens.hover, tokens.text) };
                                let label = egui::RichText::new(tool.label()).size(15.0).strong().color(ink);
                                let tile = egui::Button::new(label)
                                    .min_size(vec2(TILE_W, TILE_H))
                                    .fill(fill)
                                    .corner_radius(crate::theme::radius::CARD)
                                    .wrap();
                                if ui.add(tile).clicked() {
                                    if on {
                                        self.open_tools.remove(tool);
                                    } else {
                                        self.open_tools.insert(*tool);
                                    }
                                }
                            }
                        });
                    }
                    ui.add_space(4.0);
                    egui::CollapsingHeader::new("Choose tools").default_open(false).show(ui, |ui| {
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
                });
            self.open = open;
            if let Some(shown) = shown {
                rects.push(shown.response.rect);
            }
        }

        for (index, tool) in MathsTool::ALL.into_iter().enumerate() {
            if !self.open_tools.contains(&tool) {
                continue;
            }
            let mut open = true;
            let shown = egui::Window::new(tool.label())
                .id(Id::new(("maths_tool", tool)))
                .order(Order::Foreground)
                .default_pos(canvas.center() - vec2(180.0, 70.0) + vec2(index as f32 * 18.0, index as f32 * 20.0))
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| self.draw_tool(tool, ui));
            if let Some(shown) = shown {
                rects.push(shown.response.rect);
            }
            if !open {
                self.open_tools.remove(&tool);
            }
        }
        rects
    }

    fn draw_tool(&mut self, tool: MathsTool, ui: &mut egui::Ui) {
        match tool {
            MathsTool::Ruler => draw_ruler(ui),
            MathsTool::Protractor => draw_protractor(ui, &mut self.protractor_angle),
            MathsTool::Compass => draw_compass(ui, &mut self.compass_radius),
            MathsTool::NumberLine => draw_number_line(ui, &mut self.number_line),
            MathsTool::CoordinateGrid => draw_coordinate_grid(ui, &mut self.coordinate_grid),
            MathsTool::FractionWall => draw_fraction_wall(ui, &mut self.fraction_denominators),
            MathsTool::AlgebraTiles => draw_algebra_tiles(ui, &mut self.algebra_tiles),
            MathsTool::HundredSquare => draw_hundred_square(ui, &mut self.hundred_square),
            MathsTool::Calculator => self.calculator.ui(ui),
        }
    }
}

fn draw_protractor(ui: &mut egui::Ui, degrees: &mut f32) {
    ui.add(egui::Slider::new(degrees, 0.0..=180.0).step_by(1.0).suffix("°"));
    ui.label(format!("Angle: {:.0}°", *degrees));
    let (rect, _) = ui.allocate_exact_size(vec2(260.0, 148.0), Sense::hover());
    let painter = ui.painter();
    let centre = pos2(rect.center().x, rect.bottom() - 8.0);
    let radius = (rect.width() * 0.46).min(rect.height() * 0.9);
    painter.line_segment([pos2(centre.x - radius, centre.y), pos2(centre.x + radius, centre.y)], Stroke::new(1.5, Color32::DARK_GRAY));
    for angle in (0..=180).step_by(10) {
        let radians = (180 - angle) as f32 * std::f32::consts::PI / 180.0;
        let outer = centre + vec2(radians.cos(), -radians.sin()) * radius;
        let length = if angle % 30 == 0 { 10.0 } else { 5.0 };
        let inner = centre + vec2(radians.cos(), -radians.sin()) * (radius - length);
        painter.line_segment([inner, outer], Stroke::new(1.0, Color32::DARK_GRAY));
        if angle % 30 == 0 {
            let label_pos = centre + vec2(radians.cos(), -radians.sin()) * (radius - 23.0);
            painter.text(label_pos, Align2::CENTER_CENTER, angle.to_string(), egui::FontId::proportional(9.0), Color32::DARK_GRAY);
        }
    }
    let radians = (180.0 - *degrees) * std::f32::consts::PI / 180.0;
    let tip = centre + vec2(radians.cos(), -radians.sin()) * (radius - 13.0);
    painter.line_segment([centre, tip], Stroke::new(2.5, Color32::from_rgb(200, 50, 50)));
    painter.circle_filled(centre, 3.0, Color32::from_rgb(200, 50, 50));
}

fn draw_compass(ui: &mut egui::Ui, radius: &mut f32) {
    ui.add(egui::Slider::new(radius, 10.0..=100.0).suffix(" units"));
    ui.label(format!("Radius: {:.0} units", *radius));
    let (rect, _) = ui.allocate_exact_size(vec2(250.0, 210.0), Sense::hover());
    let centre = rect.center();
    let screen_radius = *radius * 0.9;
    ui.painter().circle_stroke(centre, screen_radius, Stroke::new(2.0, Color32::from_rgb(42, 105, 178)));
    ui.painter().line_segment([centre, centre + vec2(screen_radius, 0.0)], Stroke::new(1.5, Color32::from_gray(70)));
    ui.painter().circle_filled(centre, 3.0, Color32::DARK_GRAY);
    ui.painter().circle_filled(centre + vec2(screen_radius, 0.0), 3.0, Color32::from_rgb(190, 50, 50));
}

fn draw_coordinate_grid(ui: &mut egui::Ui, state: &mut CoordinateGridState) {
    ui.horizontal(|ui| {
        ui.label("Point (x, y)");
        ui.add(egui::DragValue::new(&mut state.x).range(-10..=10));
        ui.add(egui::DragValue::new(&mut state.y).range(-10..=10));
        if ui.button("Plot").clicked() {
            state.points.push((state.x, state.y));
        }
        if ui.button("Clear points").clicked() {
            state.points.clear();
        }
    });
    let (rect, _) = ui.allocate_exact_size(vec2(300.0, 280.0), Sense::hover());
    let painter = ui.painter();
    let step = 12.0;
    let centre = rect.center();
    for i in -10..=10 {
        let x = centre.x + i as f32 * step;
        let y = centre.y + i as f32 * step;
        let axis = i == 0;
        let color = if axis { Color32::from_gray(65) } else { Color32::from_gray(205) };
        let width = if axis { 1.5 } else { 0.7 };
        painter.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(width, color));
        painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(width, color));
    }
    for &(x, y) in &state.points {
        painter.circle_filled(centre + vec2(x as f32 * step, -(y as f32) * step), 4.5, Color32::from_rgb(200, 55, 55));
    }
}

fn draw_fraction_wall(ui: &mut egui::Ui, denominators: &mut BTreeSet<usize>) {
    ui.label("Choose equal-part rows. Each coloured block is one unit fraction.");
    ui.horizontal_wrapped(|ui| {
        for denominator in 2..=12 {
            let mut selected = denominators.contains(&denominator);
            if ui.checkbox(&mut selected, denominator.to_string()).changed() {
                if selected {
                    denominators.insert(denominator);
                } else {
                    denominators.remove(&denominator);
                }
            }
        }
    });
    for &denominator in denominators.iter() {
        ui.horizontal(|ui| {
            ui.label(format!("1/{denominator}"));
            let (rect, _) = ui.allocate_exact_size(vec2(300.0, 22.0), Sense::hover());
            let width = rect.width() / denominator as f32;
            for part in 0..denominator {
                let cell = Rect::from_min_max(
                    pos2(rect.left() + part as f32 * width, rect.top()),
                    pos2(rect.left() + (part + 1) as f32 * width, rect.bottom()),
                );
                let color = if part % 2 == 0 { Color32::from_rgb(91, 157, 208) } else { Color32::from_rgb(135, 190, 224) };
                ui.painter().rect_filled(cell, CornerRadius::ZERO, color);
                ui.painter().rect_stroke(cell, CornerRadius::ZERO, Stroke::new(0.7, Color32::WHITE), StrokeKind::Inside);
            }
        });
    }
}

fn draw_algebra_tiles(ui: &mut egui::Ui, tiles: &mut Vec<AlgebraTile>) {
    ui.label("Add tiles, then click a tile to remove it.");
    ui.horizontal_wrapped(|ui| {
        for (tile, label) in
            [(AlgebraTile::X, "+ x"), (AlgebraTile::NegativeX, "− x"), (AlgebraTile::Unit, "+ 1"), (AlgebraTile::NegativeUnit, "− 1")]
        {
            if ui.button(label).clicked() {
                tiles.push(tile);
            }
        }
        if ui.button("Clear").clicked() {
            tiles.clear();
        }
    });
    ui.label(format!(
        "x tiles: {}, unit tiles: {}",
        tiles.iter().filter(|t| matches!(t, AlgebraTile::X | AlgebraTile::NegativeX)).count(),
        tiles.len()
    ));
    let mut remove = None;
    ui.horizontal_wrapped(|ui| {
        for (index, tile) in tiles.iter().enumerate() {
            let color = tile.colour();
            let text = egui::RichText::new(tile.label()).color(Color32::BLACK).strong();
            if ui
                .add(
                    egui::Button::new(text)
                        .fill(color)
                        .min_size(vec2(if matches!(tile, AlgebraTile::X | AlgebraTile::NegativeX) { 48.0 } else { 30.0 }, 32.0)),
                )
                .clicked()
            {
                remove = Some(index);
                break;
            }
        }
    });
    if let Some(index) = remove {
        tiles.remove(index);
    }
}

fn draw_hundred_square(ui: &mut egui::Ui, selected: &mut BTreeSet<u8>) {
    ui.horizontal(|ui| {
        ui.label(format!("Selected: {} / 100", selected.len()));
        if ui.button("Clear").clicked() {
            selected.clear();
        }
    });
    egui::Grid::new("maths_hundred_square").spacing(vec2(2.0, 2.0)).show(ui, |ui| {
        for row in 0..10 {
            for col in 0..10 {
                let value = row * 10 + col + 1;
                let value = value as u8;
                let color = if selected.contains(&value) { Color32::from_rgb(250, 205, 83) } else { Color32::from_rgb(228, 239, 249) };
                if ui.add(egui::Button::new(value.to_string()).fill(color).min_size(vec2(28.0, 25.0))).clicked() && !selected.insert(value) {
                    selected.remove(&value);
                }
            }
            ui.end_row();
        }
    });
}

impl CalculatorState {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new(&self.display).size(28.0).monospace());
        egui::Grid::new("maths_calculator").spacing(vec2(4.0, 4.0)).show(ui, |ui| {
            for (index, key) in ["AC", "±", "÷", "7", "8", "9", "×", "4", "5", "6", "−", "1", "2", "3", "+", "0", ".", "="].into_iter().enumerate()
            {
                if ui.add_sized(vec2(48.0, 36.0), egui::Button::new(key)).clicked() {
                    self.press(key);
                }
                if index % 4 == 3 {
                    ui.end_row();
                }
            }
        });
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
        assert_eq!(harness.query_all_by_label("Compass").count(), 0, "not chosen, so no tile");
        harness.get_by_label("Choose tools").click();
        harness.run_steps(2);
        harness.get_by_label("Compass").click();
        harness.run_steps(2);
        assert!(harness.state().settings.is_enabled(MathsTool::Compass));
        assert_eq!(harness.query_all_by_label("Compass").count(), 2, "the checkbox and its new tile");
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
        assert!(harness.query_by_label("Ruler").is_some());
        assert!(harness.query_by_label("Number line").is_none());
        assert!(harness.query_by_label("Compass").is_none());
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
        harness.get_by_label("Ruler").click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Centimetres").is_some());
    }

    #[test]
    fn every_enabled_tool_opens_without_a_placeholder() {
        let state = State {
            panel: MathsToolsPanel { open: true, ..Default::default() },
            settings: MathsToolsSettings { enabled: MathsTool::ALL.into_iter().collect() },
        };
        let mut harness = egui_kittest::Harness::builder().with_size(vec2(1200.0, 900.0)).build_ui_state(
            |ui, state: &mut State| {
                state.panel.ui(ui.ctx(), ui.max_rect(), &mut state.settings);
            },
            state,
        );
        harness.run_steps(2);
        for tool in MathsTool::ALL {
            harness.get_by_label(tool.label()).click();
            harness.run_steps(2);
        }
        assert!(harness.query_by_label_contains("coming soon").is_none());
        for label in ["Angle: 45°", "Radius: 50 units", "Point (x, y)", "Selected: 0 / 100", "AC"] {
            assert!(harness.query_by_label(label).is_some(), "{label}");
        }
    }

    #[test]
    fn calculator_handles_operations_and_division_by_zero() {
        let mut calculator = CalculatorState::default();
        calculator.press("7");
        calculator.press("×");
        calculator.press("6");
        calculator.press("=");
        assert_eq!(calculator.display, "42");

        calculator.press("AC");
        calculator.press("8");
        calculator.press("÷");
        calculator.press("0");
        calculator.press("=");
        assert_eq!(calculator.display, "Error");
    }
}
