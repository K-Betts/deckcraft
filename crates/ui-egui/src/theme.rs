//! Design tokens (light and dark: "Fresh Paper", airy neutrals with one cobalt accent) and fonts
//! for the UI. The system is written down in `docs/design/design-system.md`; this file is its code.

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Brightness {
    #[default]
    Light,
    Dark,
}

/// Corner radii (points). See the design system: small controls, buttons and fields, ribbon
/// buttons and cards, and the big floating surfaces.
pub mod radius {
    pub const SMALL: u8 = 6;
    pub const CONTROL: u8 = 8;
    pub const CARD: u8 = 12;
    pub const SURFACE: u8 = 16;
}

/// The spacing scale (points): 4, 8, 12, 16, 24, 32.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const S: f32 = 8.0;
    pub const M: f32 = 12.0;
    pub const L: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
}

/// Type sizes (points) for the chrome.
pub mod size {
    pub const DISPLAY: f32 = 22.0;
    pub const HEADING: f32 = 16.0;
    pub const BODY: f32 = 13.0;
    pub const LABEL: f32 = 12.0;
    pub const CAPTION: f32 = 11.0;
}

/// Every colour the chrome uses.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub dark: bool,
    /// Window background (title bar, tab row, side panes).
    pub chrome: Color32,
    /// The ribbon card and popovers.
    pub card: Color32,
    pub card_border: Color32,
    /// Canvas around the slide.
    pub canvas: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub border: Color32,
    pub separator: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub selected: Color32,
    /// DeckCraft's accent (tabs underline, selected thumbnail, primary buttons).
    pub accent: Color32,
    pub accent_text: Color32,
    pub contextual: Color32,
    pub selection: Color32,
    pub handle_fill: Color32,
    pub handle_stroke: Color32,
    pub guide: Color32,
    pub caret: Color32,
    pub text_selection: Color32,
    pub field_bg: Color32,
    pub shadow: Color32,
}

impl Tokens {
    pub fn light() -> Self {
        Tokens {
            dark: false,
            chrome: Color32::from_rgb(0xF6, 0xF7, 0xF9),
            card: Color32::WHITE,
            card_border: Color32::from_rgb(0xE6, 0xE8, 0xEE),
            canvas: Color32::from_rgb(0xF6, 0xF7, 0xF9),
            text: Color32::from_rgb(0x14, 0x16, 0x1C),
            text_dim: Color32::from_rgb(0x5B, 0x60, 0x70),
            text_faint: Color32::from_rgb(0x6A, 0x70, 0x82),
            border: Color32::from_rgb(0xD8, 0xDC, 0xE6),
            separator: Color32::from_rgb(0xE6, 0xE8, 0xEE),
            hover: Color32::from_rgb(0xEE, 0xF2, 0xFF),
            pressed: Color32::from_rgb(0xE0, 0xE7, 0xFF),
            selected: Color32::from_rgb(0xE3, 0xEA, 0xFF),
            accent: Color32::from_rgb(0x2B, 0x59, 0xFF),
            accent_text: Color32::WHITE,
            contextual: Color32::from_rgb(0x1F, 0x3F, 0xBF),
            selection: Color32::from_rgb(0x7A, 0x7A, 0x7A),
            handle_fill: Color32::WHITE,
            handle_stroke: Color32::from_rgb(0x6E, 0x6E, 0x6E),
            guide: Color32::from_rgb(0xE8, 0x5D, 0x2A),
            caret: Color32::BLACK,
            text_selection: Color32::from_rgba_unmultiplied(0x5A, 0x8F, 0xE0, 90),
            field_bg: Color32::WHITE,
            shadow: Color32::from_black_alpha(18),
        }
    }
    pub fn dark() -> Self {
        Tokens {
            dark: true,
            chrome: Color32::from_rgb(0x16, 0x18, 0x1D),
            card: Color32::from_rgb(0x1F, 0x22, 0x2A),
            card_border: Color32::from_rgb(0x2E, 0x32, 0x3D),
            canvas: Color32::from_rgb(0x10, 0x12, 0x16),
            text: Color32::from_rgb(0xEC, 0xEE, 0xF3),
            text_dim: Color32::from_rgb(0xB3, 0xB8, 0xC6),
            text_faint: Color32::from_rgb(0x8B, 0x91, 0xA1),
            border: Color32::from_rgb(0x3A, 0x3F, 0x4C),
            separator: Color32::from_rgb(0x2E, 0x32, 0x3D),
            hover: Color32::from_rgb(0x26, 0x2B, 0x3A),
            pressed: Color32::from_rgb(0x2F, 0x36, 0x50),
            selected: Color32::from_rgb(0x27, 0x30, 0x4F),
            accent: Color32::from_rgb(0x6C, 0x8C, 0xFF),
            accent_text: Color32::WHITE,
            contextual: Color32::from_rgb(0x8F, 0xA8, 0xFF),
            selection: Color32::from_rgb(0xAA, 0xAA, 0xAA),
            handle_fill: Color32::WHITE,
            handle_stroke: Color32::from_rgb(0x6E, 0x6E, 0x6E),
            guide: Color32::from_rgb(0xF2, 0x8A, 0x4B),
            caret: Color32::BLACK,
            text_selection: Color32::from_rgba_unmultiplied(0x5A, 0x8F, 0xE0, 90),
            field_bg: Color32::from_rgb(0x1A, 0x1D, 0x24),
            shadow: Color32::from_black_alpha(90),
        }
    }
    pub fn for_brightness(b: Brightness) -> Self {
        match b {
            Brightness::Light => Self::light(),
            Brightness::Dark => Self::dark(),
        }
    }
    pub fn get(ctx: &egui::Context) -> Self {
        if ctx.global_style().visuals.dark_mode { Self::dark() } else { Self::light() }
    }
}

/// Apply tokens to egui's style.
pub fn apply(ctx: &egui::Context, t: &Tokens) {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.chrome;
    v.window_fill = t.card;
    v.extreme_bg_color = t.field_bg;
    v.faint_bg_color = t.hover;
    v.window_stroke = Stroke::new(1.0, t.card_border);
    v.window_corner_radius = CornerRadius::same(radius::SURFACE);
    v.menu_corner_radius = CornerRadius::same(radius::CARD);
    v.selection.bg_fill = t.accent.gamma_multiply(0.85);
    v.selection.stroke = Stroke::new(1.0, t.accent_text);
    v.hyperlink_color = t.accent;
    v.override_text_color = Some(t.text);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = CornerRadius::same(radius::CONTROL);
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.separator);
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_fill = t.field_bg;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.weak_bg_fill = t.hover;
    v.widgets.hovered.bg_fill = t.hover;
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.active.weak_bg_fill = t.pressed;
    v.widgets.active.bg_fill = t.pressed;
    v.widgets.open.weak_bg_fill = t.pressed;
    v.window_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: t.shadow };
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 16, spread: 0, color: t.shadow };
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(space::S, 6.0);
        s.spacing.button_padding = egui::vec2(8.0, 5.0);
        s.spacing.interact_size.y = 26.0;
        s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(size::BODY));
        s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(size::BODY));
        s.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(size::CAPTION));
        s.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(size::HEADING + 2.0));
    });
}

/// UI fonts: Inter (from craft-fonts) when embedded, a Japanese UI face for CJK, then egui's.
pub fn install_fonts(ctx: &egui::Context) {
    install_fonts_with(ctx, &[]);
}

/// [`install_fonts`], with `extra` faces from the font database after every interface font: the
/// installed or embedded faces that draw text the interface faces lack (Hangul, Simplified
/// Chinese, Devanagari…, #101). See [`crate::uifonts`].
pub fn install_fonts_with(ctx: &egui::Context, extra: &[std::sync::Arc<deckcraft_fonts::FontFace>]) {
    ctx.set_fonts(font_definitions(extra));
}

/// The interface font definitions; see [`install_fonts_with`].
pub fn font_definitions(extra: &[std::sync::Arc<deckcraft_fonts::FontFace>]) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let mut names = vec![];
    for (i, f) in deckcraft_fonts::CRAFT_FONTS.iter().enumerate() {
        let wanted = (f.family == "Inter" && (f.style == "Regular" || f.style == "SemiBold" || f.style == "Bold"))
            || (f.family == "BIZ UDPGothic" && f.style == "Regular")
            || (f.family == "Noto Sans Arabic");
        if !wanted {
            continue;
        }
        let name = format!("craft-{i}-{}-{}", f.family, f.style);
        defs.font_data.insert(name.clone(), std::sync::Arc::new(FontData::from_static(f.bytes)));
        names.push((f.family, f.style, name));
    }
    let regular: Vec<String> = names.iter().filter(|(fam, st, _)| !(*fam == "Inter" && *st != "Regular")).map(|(_, _, n)| n.clone()).collect();
    if let Some(list) = defs.families.get_mut(&FontFamily::Proportional) {
        for (k, n) in regular.iter().enumerate() {
            list.insert(k, n.clone());
        }
    }
    // A bold family for headings and selected tabs.
    let mut bold: Vec<String> = names.iter().filter(|(fam, st, _)| *fam == "Inter" && *st == "SemiBold").map(|(_, _, n)| n.clone()).collect();
    bold.extend(defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    defs.families.insert(FontFamily::Name("bold".into()), bold);
    add_fallback_faces(&mut defs, extra);
    // The Japanese interface face (the first one when Inter isn't embedded) hints its en and em
    // dashes, bars thinner than a pixel, down to nothing, so they drew as gaps in interface text;
    // unhinted, they draw.
    for (name, d) in defs.font_data.iter_mut() {
        if name.contains("BIZ UDPGothic") {
            let mut data = (**d).clone();
            data.tweak.hinting = Some(false);
            *d = std::sync::Arc::new(data);
        }
    }
    defs
}

/// Add `faces` after every other font of each interface family (the last fallbacks).
pub fn add_fallback_faces(defs: &mut FontDefinitions, faces: &[std::sync::Arc<deckcraft_fonts::FontFace>]) {
    for f in faces {
        let name = format!("fallback {} {} {}", f.family, f.style, f.id());
        if defs.font_data.contains_key(&name) {
            continue;
        }
        let mut data = FontData::from_owned(f.data().to_vec());
        data.index = f.index();
        defs.font_data.insert(name.clone(), std::sync::Arc::new(data));
        for list in defs.families.values_mut() {
            list.push(name.clone());
        }
    }
}

pub fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("bold".into()))
}

pub fn font(size: f32) -> egui::FontId {
    egui::FontId::proportional(size)
}

pub fn to_color32(c: deckcraft_color::Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

#[cfg(test)]
mod tests {
    use super::{Color32, Tokens};

    /// WCAG relative luminance and contrast ratio.
    fn luminance(c: Color32) -> f64 {
        let f = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// The design system's promise (docs/design/design-system.md): text is readable in both modes.
    #[test]
    fn tokens_keep_the_contrast_the_design_system_promises() {
        for t in [Tokens::light(), Tokens::dark()] {
            let mode = if t.dark { "dark" } else { "light" };
            let ok = |what: &str, fg: Color32, bg: Color32, min: f64| {
                let c = contrast(fg, bg);
                assert!(c >= min, "{mode}: {what} is {c:.2}:1, needs {min}:1");
            };
            for bg in [t.chrome, t.card, t.canvas] {
                ok("text", t.text, bg, 7.0);
                ok("text_dim", t.text_dim, bg, 4.5);
                ok("text_faint", t.text_faint, bg, 4.5);
                ok("accent as text", t.accent, bg, 4.5);
                ok("contextual as text", t.contextual, bg, 4.5);
            }
            ok("text on hover", t.text, t.hover, 7.0);
            ok("text on selected", t.text, t.selected, 7.0);
            ok("text on pressed", t.text, t.pressed, 7.0);
            ok("text on a field", t.text, t.field_bg, 7.0);
        }
        // White on the accent is the Present button and every primary button.
        let light = Tokens::light();
        assert!(contrast(light.accent_text, light.accent) >= 4.5);
    }

    /// Each character of `text` laid out in the interface fonts at interface sizes, and whether it
    /// has ink (a glyph image).
    fn drawn(text: &str) -> Vec<(char, f32, bool)> {
        let ctx = egui::Context::default();
        ctx.set_fonts(super::font_definitions(&[]));
        let mut o = ctx.run_ui(Default::default(), |_| {});
        o.textures_delta.clear();
        let mut out = vec![];
        ctx.fonts_mut(|f| {
            for size in [11.0, 12.0, 13.0, 16.0] {
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Name("bold".into())] {
                    let g = f.layout_no_wrap(text.to_string(), egui::FontId::new(size, family), egui::Color32::WHITE);
                    for gl in g.rows.iter().flat_map(|r| &r.glyphs) {
                        out.push((gl.chr, size, gl.uv_rect.size.x > 0.0 && gl.uv_rect.size.y > 0.0));
                    }
                }
            }
        });
        let mut o = ctx.run_ui(Default::default(), |_| {});
        o.textures_delta.clear();
        out
    }

    #[test]
    fn thin_punctuation_is_drawn_in_interface_text() {
        // En and em dashes (thin bars) were hinted down to nothing in the Japanese interface face,
        // so notes and the outline showed gaps where they were.
        for (c, size, ink) in drawn("\u{2013}\u{2014}-_\u{2026}\u{2022}\u{00b7}\u{2018}\u{2019}\u{201c}\u{201d}=~") {
            assert!(ink, "{c:?} ({:04X}) draws nothing at {size} px", c as u32);
        }
    }
}
