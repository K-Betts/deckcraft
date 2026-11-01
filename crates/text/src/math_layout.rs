//! Equations in text layout: the stand-in text of an equation not drawn as maths, and equations
//! drawn as maths (fractions, powers, roots) over blank space as wide as they are.

use crate::*;

/// The text that stands in for an equation we do not draw as maths: letters are the mathematical
/// italic ones, as the equation editor writes them, unless the equation sets its own style or the
/// text holds a word (`sin`, `max`), which stays upright. One character in, one out.
pub(crate) fn stand_in_text(text: &str, omml: &str) -> String {
    let word = text.chars().collect::<Vec<_>>().windows(3).any(|w| w.iter().all(char::is_ascii_alphabetic));
    if word || omml.contains("<m:sty") {
        return text.to_string();
    }
    text.chars().map(|c| if c.is_ascii_alphabetic() { equation::math_italic(c) } else { c }).collect()
}

/// Whether the paragraph is one equation on its own (a display equation), which is laid out whole.
pub(crate) fn math_of_paragraph_kind(para: &deckcraft_model::text::Paragraph) -> bool {
    let maths = para.runs.iter().filter(|r| matches!(r.kind, RunKind::Math { .. })).count();
    maths == 1 && para.runs.iter().all(|r| matches!(r.kind, RunKind::Math { .. }) || (r.kind == RunKind::Text && r.text.trim().is_empty()))
}

/// The maths box of a paragraph that is one equation worth drawing as maths, and the index of the
/// style (one per run, in order) its colour and size come from.
pub(crate) fn math_of_paragraph(para: &deckcraft_model::text::Paragraph, styles: &[Style]) -> Option<(math::MathBox, usize)> {
    let mut found = None;
    for (i, r) in para.runs.iter().enumerate() {
        match &r.kind {
            RunKind::Math { omml } if found.is_none() => found = Some((i, omml)),
            RunKind::Text if r.text.trim().is_empty() => {}
            _ => return None,
        }
    }
    let (i, omml) = found?;
    let expr = equation::from_omml(omml)?;
    if !math::is_complex(&expr) {
        return None;
    }
    let st = styles.get(i)?;
    Some((math::layout(&expr, &st.face, st.size), i))
}

/// The maths box of an equation among other text, when it is worth drawing as maths.
pub(crate) fn inline_math_box(kind: &RunKind, st: &Style) -> Option<math::MathBox> {
    let RunKind::Math { omml } = kind else { return None };
    let expr = equation::from_omml(omml)?;
    math::is_complex(&expr).then(|| math::layout(&expr, &st.face, st.size))
}

/// Non-breaking spaces as wide as the equation `b` in style `st`, which it is drawn over.
pub(crate) fn math_blank(b: &math::MathBox, st: &Style) -> String {
    let space = st.face.advance(st.face.glyph_for('\u{a0}')) * st.size / st.face.upem.max(1.0);
    let n = if space > 0.1 { (b.w / space).ceil().clamp(1.0, 2000.0) as usize } else { 1 };
    "\u{a0}".repeat(n)
}

/// Put an equation's glyphs and bars on the layout, its baseline at (`left`, `baseline`).
pub(crate) fn emit_math(out: &mut TextLayout, m: &math::MathBox, st: &Style, left: f64, baseline: f64, para: usize) {
    let mut groups: Vec<((u64, bool), GlyphRun)> = vec![];
    for g in &m.glyphs {
        let key = (g.size.to_bits(), g.slant);
        let at = match groups.iter().position(|(k, _)| *k == key) {
            Some(i) => i,
            None => {
                groups.push((
                    key,
                    GlyphRun {
                        face: st.face.clone(),
                        size: g.size,
                        color: st.color,
                        glyphs: vec![],
                        cells: vec![],
                        fake_bold: st.fake_bold,
                        fake_italic: g.slant,
                        outline: None,
                        para,
                        chars: (0, 0),
                        link: false,
                        alpha: st.alpha,
                        gradient: st.gradient.clone(),
                        effects: st.effects,
                        upright: false,
                    },
                ));
                groups.len() - 1
            }
        };
        groups[at].1.glyphs.push((g.gid, left + g.x, baseline + g.y));
    }
    out.runs.extend(groups.into_iter().map(|(_, r)| r));
    for r in &m.rules {
        out.decos.push(Deco {
            rect: Rect::new(left + r.x0, baseline + r.y0, left + r.x1, baseline + r.y1),
            color: st.color,
            behind: false,
            para,
            line: DecoLine::Solid,
        });
    }
}
