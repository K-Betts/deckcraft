//! Reveal Step by Step (`animation.steps`): each line of working appears on its own click.

use super::{
    AnimStart, Animation, Arc, CommandSpec, Result, Session, TextBuild, Value, bad, build, cmd, has_selection, json, slide_extent, str_param, targets,
};

pub(super) fn spec() -> CommandSpec {
    cmd!(
        "animation.steps",
        "Reveal Step by Step",
        ["Animations", "Maths"],
        None,
        "{effect?: appear|fade|wipe (default appear), ids?} → {steps} — each line of the shapes' text appears on its own click; replaces their animations",
        has_selection,
        steps
    )
}

/// The steps a by-paragraph build reveals: first-level paragraphs that hold something (the same
/// rule as the show timeline, so blank lines between steps never cost a click).
fn step_count(t: &deckcraft_model::text::TextBody) -> usize {
    let filled = |p: &deckcraft_model::text::Paragraph| p.runs.iter().any(|r| !r.text.trim().is_empty());
    let top = t.paragraphs.iter().filter(|p| filled(p)).map(|p| p.level).min().unwrap_or(0);
    t.paragraphs.iter().filter(|p| p.level <= top && filled(p)).count()
}

/// Reveal Step by Step: each line of working (first-level paragraph) of the shapes' text appears
/// on its own click; deeper lines come with the line above them. Saved to PowerPoint as a
/// by-paragraph entrance, so it plays the same in PowerPoint's Slide Show.
fn steps(s: &mut Session, p: &Value) -> Result<Value> {
    let effect = str_param(p, "effect").unwrap_or("appear");
    if !matches!(effect, "appear" | "fade" | "wipe") {
        return Err(bad("animation.steps", format!("effect `{effect}` is not one of appear, fade, wipe")));
    }
    let wanted = targets(s, p)?;
    let (ids, total) = {
        let slide = s.doc()?.current_slide().ok_or_else(|| bad("animation.steps", "no slide"))?;
        let mut ids = vec![];
        let mut total = 0;
        for id in wanted {
            if let Some(n) = slide.shape(id).and_then(|sh| sh.text.as_ref()).map(step_count).filter(|n| *n > 0) {
                ids.push(id);
                total += n;
            }
        }
        (ids, total)
    };
    if ids.is_empty() {
        return Err(bad("animation.steps", "select a text box with each step of the working on its own line"));
    }
    let mut a = build(&json!({"effect": effect, "class": "entrance"}), slide_extent(s)?)?;
    a.text_build = TextBuild::ByParagraph;
    a.start = AnimStart::OnClick;
    s.edit(|doc, sel| {
        let sl = Arc::make_mut(doc.slides.get_mut(sel.slide).ok_or_else(|| bad("animation.steps", "no slide"))?);
        // Like animation.set: the shapes' old animations go, and the steps take the place of the
        // first of them in the sequence (or go at the end).
        let mut out = vec![];
        for x in sl.animations.drain(..) {
            if !ids.contains(&x.shape) {
                out.push(x);
            } else if !out.iter().any(|o: &Animation| o.shape == x.shape) {
                out.push(Animation { shape: x.shape, ..a.clone() });
            }
        }
        for id in &ids {
            if !out.iter().any(|o| o.shape == *id) {
                out.push(Animation { shape: *id, ..a.clone() });
            }
        }
        sl.animations = out;
        Ok(json!({"steps": total}))
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::super::AnimClass;
    use super::*;

    fn working_box(s: &mut Session, text: &str) -> u64 {
        let id = s.execute("shape.insert", &json!({"preset": "rect", "rect": [40.0, 40.0, 300.0, 200.0]})).unwrap()["id"].as_u64().unwrap();
        s.execute("text.set", &json!({"id": id, "text": text})).unwrap();
        id
    }

    #[test]
    fn reveal_step_by_step_gives_each_line_its_own_click_and_saves_to_powerpoint() {
        let mut s = Session::with_new();
        // A blank line between steps is not a step of its own.
        let id = working_box(&mut s, "2x + 3 = 11\n2x = 8\n\nx = 4");
        let before = s.doc().unwrap().doc.clone();
        let r = s.execute("animation.steps", &json!({"ids": [id]})).unwrap();
        assert_eq!(r["steps"], 3);
        let anims = &s.doc().unwrap().current_slide().unwrap().animations;
        assert_eq!(anims.len(), 1);
        let a = &anims[0];
        assert_eq!((a.effect.as_str(), a.class, a.start, a.text_build), ("appear", AnimClass::Entrance, AnimStart::OnClick, TextBuild::ByParagraph));
        let tl = deckcraft_anim::ShowState::timeline(&s.doc().unwrap().doc, 0);
        assert_eq!(tl.steps(), 3, "one click per line of working, the blank line skipped");

        // PowerPoint gets a by-paragraph build, and it comes back the same.
        let bytes = deckcraft_pptx::export(&s.doc().unwrap().doc).unwrap();
        let back = deckcraft_pptx::import(&bytes).unwrap();
        let b = &back.slides[0].animations;
        assert_eq!(b.len(), 1);
        assert_eq!((b[0].effect.as_str(), b[0].start, b[0].text_build), ("appear", AnimStart::OnClick, TextBuild::ByParagraph));

        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().doc.slides, before.slides, "one undo takes the steps off");
    }

    #[test]
    fn reveal_step_by_step_replaces_old_animations_and_keeps_others() {
        let mut s = Session::with_new();
        let other = s.execute("shape.insert", &json!({"preset": "ellipse", "rect": [400.0, 40.0, 80.0, 80.0]})).unwrap()["id"].as_u64().unwrap();
        let id = working_box(&mut s, "Step one\nStep two");
        s.execute("animation.add", &json!({"effect": "fly", "ids": [id]})).unwrap();
        s.execute("animation.add", &json!({"effect": "spin", "class": "emphasis", "ids": [id]})).unwrap();
        s.execute("animation.add", &json!({"effect": "fade", "ids": [other]})).unwrap();
        let r = s.execute("animation.steps", &json!({"ids": [id], "effect": "fade"})).unwrap();
        assert_eq!(r["steps"], 2);
        let anims = &s.doc().unwrap().current_slide().unwrap().animations;
        let got: Vec<(u32, &str)> = anims.iter().map(|a| (a.shape.0, a.effect.as_str())).collect();
        assert_eq!(got, vec![(id as u32, "fade"), (other as u32, "fade")], "the steps take the first old animation's place");
        assert_eq!(anims[0].text_build, TextBuild::ByParagraph);
    }

    #[test]
    fn reveal_step_by_step_needs_text_and_a_known_effect() {
        let mut s = Session::with_new();
        let shape = s.execute("shape.insert", &json!({"preset": "rect", "rect": [10.0, 10.0, 50.0, 50.0]})).unwrap()["id"].as_u64().unwrap();
        assert!(s.execute("animation.steps", &json!({"ids": [shape]})).is_err(), "a shape without text has no steps");
        let id = working_box(&mut s, "x = 4");
        assert!(s.execute("animation.steps", &json!({"ids": [id], "effect": "spin"})).is_err());
        assert!(s.doc().unwrap().current_slide().unwrap().animations.is_empty(), "nothing changed");
    }
}
