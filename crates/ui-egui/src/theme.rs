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
    /// Text fields. They have no outline, so the fill alone must set them apart from the card.
    pub text_field: Color32,
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
    /// Unchecked checkbox and radio boxes and slider rails, drawn without an outline, so they
    /// must stand apart from the card and the chrome.
    pub control: Color32,
    /// [`Tokens::control`] under the pointer and pressed: a step darker each in light mode.
    pub control_hover: Color32,
    pub control_pressed: Color32,
    pub shadow: Color32,
}

impl Tokens {
    pub fn light() -> Self {
        Tokens {
            dark: false,
            chrome: Color32::from_rgb(0xF6, 0xF7, 0xF9),
            card: Color32::WHITE,
            card_border: Color32::from_rgb(0xE6, 0xE8, 0xEE),
            text_field: Color32::from_rgb(0xED, 0xF0, 0xF7),
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
            control: Color32::from_rgb(0xE0, 0xE7, 0xFF),
            control_hover: Color32::from_rgb(0xD3, 0xDC, 0xFF),
            control_pressed: Color32::from_rgb(0xC4, 0xD0, 0xFF),
            shadow: Color32::from_black_alpha(18),
        }
    }
    pub fn dark() -> Self {
        Tokens {
            dark: true,
            chrome: Color32::from_rgb(0x16, 0x18, 0x1D),
            card: Color32::from_rgb(0x1F, 0x22, 0x2A),
            card_border: Color32::from_rgb(0x2E, 0x32, 0x3D),
            text_field: Color32::from_rgb(0x1A, 0x1D, 0x24),
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
            accent_text: Color32::from_rgb(0x0E, 0x12, 0x24),
            contextual: Color32::from_rgb(0x8F, 0xA8, 0xFF),
            selection: Color32::from_rgb(0xAA, 0xAA, 0xAA),
            handle_fill: Color32::WHITE,
            handle_stroke: Color32::from_rgb(0x6E, 0x6E, 0x6E),
            guide: Color32::from_rgb(0xF2, 0x8A, 0x4B),
            caret: Color32::BLACK,
            text_selection: Color32::from_rgba_unmultiplied(0x5A, 0x8F, 0xE0, 90),
            field_bg: Color32::from_rgb(0x1A, 0x1D, 0x24),
            control: Color32::from_rgb(0x2F, 0x36, 0x50),
            control_hover: Color32::from_rgb(0x3A, 0x42, 0x62),
            control_pressed: Color32::from_rgb(0x45, 0x4E, 0x72),
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

/// `"system"`, `"light"` or `"dark"`: how the appearance is being chosen.
pub fn appearance_mode(follow_system: bool, b: Brightness) -> &'static str {
    match (follow_system, b) {
        (true, _) => "system",
        (false, Brightness::Dark) => "dark",
        (false, Brightness::Light) => "light",
    }
}

/// The Appearance drop-down shared by Preferences and the File page.
pub fn appearance_picker(app: &mut crate::SlideApp, ui: &mut egui::Ui, salt: &str) {
    let now = appearance_mode(app.ui.follow_system, app.ui.brightness);
    let name = |m: &str| match m {
        "system" => "Match this computer",
        "dark" => "Dark",
        _ => "Light",
    };
    let combo = egui::ComboBox::from_id_salt(salt).width(200.0).selected_text(name(now)).show_ui(ui, |ui| {
        for mode in ["system", "light", "dark"] {
            if ui.selectable_label(now == mode, name(mode)).clicked() {
                let _ = app.run("view.appearance", serde_json::json!({ "mode": mode }));
            }
        }
    });
    combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Appearance"));
}

/// Apply tokens to egui's style.
pub fn apply(ctx: &egui::Context, t: &Tokens) {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.chrome;
    v.window_fill = t.card;
    v.extreme_bg_color = t.field_bg;
    v.text_edit_bg_color = Some(t.text_field);
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
    v.widgets.inactive.bg_fill = t.control;
    // Fields, checkboxes and slider rails need an outline at rest, or they vanish on a white card.
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.hovered.weak_bg_fill = t.hover;
    v.widgets.hovered.bg_fill = t.control_hover;
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.active.weak_bg_fill = t.pressed;
    v.widgets.active.bg_fill = t.control_pressed;
    v.widgets.open.weak_bg_fill = t.pressed;
    v.window_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: t.shadow };
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 16, spread: 0, color: t.shadow };
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(space::S, 6.0);
        s.spacing.button_padding = egui::vec2(8.0, 5.0);
        s.spacing.interact_size.y = 26.0;
        // Dialogs and floating panels: room around their contents.
        s.spacing.window_margin = egui::Margin::same(20);
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
            // Exact craft-fonts family names: the Korean face is "NanumGothic" (no space), and
            // the symbol faces are "Noto Sans Symbols" / "Noto Sans Symbols 2". A near-miss here
            // would leave Hangul and the ◉ laser toggle without a glyph. Hebrew backs the
            // Language dialog's native "עברית" label where no system font supplies it.
            || matches!(f.family, "Noto Sans Arabic" | "Noto Sans Hebrew" | "NanumGothic" | "Noto Sans Symbols" | "Noto Sans Symbols 2")
            // This opt-in native face follows the existing families. Keep per-character
            // fallback and Latin metrics; no Chinese payload enters the wasm allowlist.
            || (cfg!(not(target_arch = "wasm32")) && f.family == "LXGW WenKai Lite" && f.style == "Regular");
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
    // Hack is already bundled for Monospace and covers the interface's arrows and triangles.
    // Keep it after every proportional face so ordinary text metrics stay unchanged.
    if defs.font_data.contains_key("Hack") {
        for chain in defs.families.values_mut() {
            if !chain.iter().any(|name| name == "Hack") {
                chain.push("Hack".into());
            }
        }
    }
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
    use super::*;

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
        // Text on the accent is the Present button and every primary button.
        for t in [Tokens::light(), Tokens::dark()] {
            assert!(contrast(t.accent_text, t.accent) >= 4.5, "dark={}", t.dark);
        }
    }

    /// Each character of `text` laid out in the interface fonts at interface sizes, and whether it
    /// has ink (a glyph image).
    fn drawn(text: &str) -> Vec<(char, f32, bool)> {
        let ctx = egui::Context::default();
        // A replacement glyph also gets a nonempty atlas rectangle. Fail before layout when no
        // font can draw a character, so `uv_rect` cannot mistake tofu for real ink.
        ctx.set_missing_glyph_policy(egui::MissingGlyphPolicy::Panic);
        ctx.set_fonts(super::font_definitions(&[]));
        let mut o = ctx.run_ui(Default::default(), |_| {});
        o.textures_delta.clear();
        let mut out = vec![];
        ctx.fonts_mut(|f| {
            for size in [11.0, 12.0, 13.0, 16.0] {
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Name("bold".into())] {
                    let font = egui::FontId::new(size, family);
                    for c in text.chars() {
                        assert!(f.has_glyph(&font, c), "no font supplies {c:?} at {size} px in {font:?}");
                    }
                    let g = f.layout_no_wrap(text.to_string(), font, egui::Color32::WHITE);
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

    /// The UI font library supplies default text fonts and an explicit monochrome emoji face,
    /// independent of kittest defaults.
    #[test]
    fn the_ui_library_has_default_fonts_and_explicit_monochrome_emoji_for_tests() {
        // `egui`'s `default_fonts` is no longer a default feature and kittest's own dev
        // dependencies do not propagate. This library enables it (and the test build enables
        // `monochrome_emoji_fonts`) explicitly.
        let defs = FontDefinitions::default();
        assert!(defs.font_data.contains_key("Hack"), "default_fonts must be enabled by deckcraft-ui-egui itself");
        assert!(defs.font_data.contains_key("Ubuntu-Light"));
        assert!(defs.font_data.contains_key("NotoEmoji-Regular"), "monochrome_emoji_fonts is an explicit test feature");
        assert!(defs.families.contains_key(&FontFamily::Proportional));
        // The production font policy still owns the UI families: craft faces first, then egui's.
        let ours = super::font_definitions(&[]);
        assert!(ours.families.contains_key(&FontFamily::Name("bold".into())));
        assert!(ours.font_data.contains_key("Hack"));
    }

    /// When the Chinese face is present (required builds guarantee it), exactly one LXGW WenKai
    /// Lite Regular is embedded, and Chinese labels have glyph coverage and ink; builds without the
    /// optional face skip this test.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn native_chinese_fallback_draws_initial_document_labels() {
        let faces: Vec<_> = deckcraft_fonts::CRAFT_FONTS.iter().filter(|f| f.family == "LXGW WenKai Lite").collect();
        if faces.is_empty() {
            assert!(option_env!("CRAFT_FONTS_REQUIRED").is_none(), "required native font input must include the reviewed Chinese fallback");
            eprintln!("optional Chinese input absent: bundled Chinese coverage was not qualified");
            return;
        }
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].style, "Regular");
        assert_eq!(faces[0].bytes.len(), 13_949_472);
        let face = skrifa::FontRef::new(faces[0].bytes).unwrap();
        use skrifa::MetadataProvider as _;
        assert!(face.charmap().map('简').is_some(), "the approved fallback supplies the original missing glyph");
        let defs = font_definitions(&[]);
        for family in [FontFamily::Proportional, FontFamily::Name("bold".into())] {
            let chain = &defs.families[&family];
            let chinese = chain.iter().position(|name| name.contains("LXGW WenKai Lite")).unwrap();
            let japanese = chain.iter().position(|name| name.contains("BIZ UDPGothic")).unwrap();
            assert!(japanese < chinese, "retain existing glyph selection and Latin metrics");
            assert_eq!(chain.iter().filter(|name| name.contains("LXGW WenKai Lite")).count(), 1);
        }
        let text = "简体中文演示文稿";
        let ctx = egui::Context::default();
        ctx.set_missing_glyph_policy(egui::MissingGlyphPolicy::Panic);
        ctx.set_fonts(defs);
        let mut output = ctx.run_ui(Default::default(), |_| {});
        output.textures_delta.clear();
        ctx.fonts_mut(|fonts| {
            for family in [FontFamily::Proportional, FontFamily::Name("bold".into())] {
                let font = egui::FontId::new(13.0, family);
                for c in text.chars() {
                    assert!(fonts.has_glyph(&font, c), "the embedded fallback does not supply {c:?} in {font:?}");
                }
            }
        });
        let ink = drawn(text);
        assert_eq!(ink.len(), text.chars().count() * 4 * 2);
        for (c, size, visible) in ink {
            assert!(visible, "Chinese label {c:?} has no ink at {size} px");
        }
    }

    /// When the face is present (required builds guarantee it), the native Hebrew fallback is exactly
    /// one static Regular Noto Sans Hebrew face with glyph coverage and ink for language labels.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn native_hebrew_fallback_is_one_regular_face_and_draws_language_labels() {
        let faces: Vec<_> = deckcraft_fonts::CRAFT_FONTS.iter().filter(|f| f.family == "Noto Sans Hebrew").collect();
        if faces.is_empty() {
            assert!(option_env!("CRAFT_FONTS_REQUIRED").is_none(), "required native font input must include Noto Sans Hebrew Regular");
            eprintln!("optional font input absent: native Hebrew coverage was not qualified");
            return;
        }
        assert_eq!(faces.len(), 1, "one static face, without variable named instances");
        assert_eq!(faces[0].style, "Regular");
        assert_eq!(faces[0].bytes.len(), 66_156, "use the approved full static Regular face");
        use skrifa::raw::{ReadError, TableProvider};
        let face = skrifa::FontRef::new(faces[0].bytes).unwrap();
        assert_eq!(face.os2().unwrap().us_weight_class(), 400, "the embedded bytes must be Regular, not a renamed Thin face");
        assert!(matches!(face.fvar(), Err(ReadError::TableIsMissing(_))), "embed one static face, not variable named instances");
        let defs = super::font_definitions(&[]);
        let names: Vec<_> = defs.font_data.keys().filter(|name| name.contains("Noto Sans Hebrew")).collect();
        assert_eq!(names.len(), 1);
        for family in [FontFamily::Proportional, FontFamily::Name("bold".into())] {
            assert_eq!(defs.families[&family].iter().filter(|name| *name == names[0]).count(), 1);
        }
        let ctx = egui::Context::default();
        ctx.set_fonts(defs);
        let mut output = ctx.run_ui(Default::default(), |_| {});
        output.textures_delta.clear();
        ctx.fonts_mut(|fonts| {
            for family in [FontFamily::Proportional, FontFamily::Name("bold".into())] {
                let font = egui::FontId::new(13.0, family);
                for c in "עבריתשלוםdeck42".chars() {
                    assert!(fonts.has_glyph(&font, c), "missing language-label glyph {c:?}");
                }
            }
        });
        // This checks ink and coverage only, not bidi ordering or GPOS mark positioning.
        for (c, size, ink) in drawn("עבריתשלוםdeck42") {
            assert!(ink, "{c:?} has no ink at {size} px");
        }
    }

    /// Hebrew fallback preserves logical-order editing in mixed-direction text.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn native_hebrew_fallback_keeps_mixed_text_editing_in_logical_order() {
        if !deckcraft_fonts::CRAFT_FONTS.iter().any(|f| f.family == "Noto Sans Hebrew") {
            assert!(option_env!("CRAFT_FONTS_REQUIRED").is_none(), "required native Hebrew face is absent");
            return;
        }
        let ctx = egui::Context::default();
        ctx.set_fonts(super::font_definitions(&[]));
        let id = egui::Id::unique("mixed-hebrew-label-edit");
        let mut text = "deck שלום 42".to_owned();
        let frame = |text: &mut String, events: Vec<egui::Event>| {
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 100.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    ui.memory_mut(|m| m.request_focus(id));
                    ui.add(egui::TextEdit::singleline(text).id(id));
                },
            );
            output.drop_without_applying_deltas();
        };
        frame(&mut text, vec![]);
        // Character offsets remain logical even though the Hebrew glyphs are drawn right-to-left.
        let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(7))));
        egui::TextEdit::store_state(&ctx, id, state);
        frame(&mut text, vec![egui::Event::Text("א".into())]);
        assert_eq!(text, "deck שלאום 42");
        let state = egui::TextEdit::load_state(&ctx, id).unwrap();
        assert_eq!(state.cursor.char_range().unwrap().primary.index.0, 8);
        frame(
            &mut text,
            vec![egui::Event::Key { key: egui::Key::Backspace, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }],
        );
        assert_eq!(text, "deck שלום 42");
        ctx.fonts_mut(|fonts| {
            let galley = fonts.layout_no_wrap(text.clone(), egui::FontId::proportional(13.0), egui::Color32::WHITE);
            let row = &galley.rows[0];
            let x = |c| row.glyphs.iter().find(|g| g.chr == c).unwrap().pos.x;
            assert!(x('ש') > x('ל') && x('ל') > x('ו') && x('ו') > x('ם'), "Hebrew paints in visual RTL order");
            assert!(x('4') < x('2'), "the adjacent number keeps its LTR order");
        });
        frame(&mut text, vec![]); // consume the atlas updates from the explicit layout above
    }

    #[test]
    fn light_checkboxes_and_slider_rails_stand_out_from_their_background() {
        // An unchecked checkbox and a slider rail are filled with the inactive widget fill, so in
        // light mode they vanished into the white dialog (#766). DeckCraft gives them an outline at
        // rest; without one the fill alone must stand apart from the background.
        let ctx = egui::Context::default();
        super::apply(&ctx, &super::Tokens::light());
        let v = ctx.global_style().visuals.clone();
        let fill = v.widgets.inactive.bg_fill;
        let outline = v.widgets.inactive.bg_stroke;
        assert!(outline.width >= 1.0 && outline.color.a() > 0, "controls have an outline at rest");
        for (name, bg) in [("dialog", v.window_fill), ("panel", v.panel_fill)] {
            let diff = [outline.color.r().abs_diff(bg.r()), outline.color.g().abs_diff(bg.g()), outline.color.b().abs_diff(bg.b())];
            assert!(diff.iter().any(|d| *d >= 16), "control outline {:?} is too close to the {name} background {bg:?}", outline.color);
        }
        // Hovering and pressing darken the control a step each, as they do buttons.
        let (hovered, active) = (v.widgets.hovered.bg_fill, v.widgets.active.bg_fill);
        assert!(fill.r() > hovered.r() && hovered.r() > active.r(), "idle {fill:?}, hovered {hovered:?}, active {active:?} don't darken in turn");
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
