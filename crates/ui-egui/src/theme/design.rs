//! The design system's scales (corner radii, spacing, type sizes) and its promise that text stays
//! readable on every surface, in both modes. Written down in `docs/design/design-system.md`.

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

#[cfg(test)]
mod tests {
    use egui::Color32;

    use super::super::Tokens;

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
}
