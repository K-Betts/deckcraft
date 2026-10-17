//! Inserting an equation through the command.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[test]
fn insert_equation_makes_a_real_equation_that_survives_saving() {
    use deckcraft_model::text::RunKind;
    use serde_json::json;
    let mut s = crate::Session::with_new();
    s.execute("slide.new", &json!({})).unwrap();
    assert!(s.execute("insert.equation", &json!({"linear": "  "})).is_err(), "nothing typed");
    assert!(s.execute("insert.equation", &json!({"linear": "(□)/(2)"})).is_err(), "a box is still empty");
    s.execute("insert.equation", &json!({"linear": "x=(-b+-sqrt(b^2-4ac))/(2a)"})).unwrap();
    let doc = deckcraft_pptx::import(&deckcraft_pptx::export(&s.doc().unwrap().doc).unwrap()).unwrap();
    let shape = doc.slides.iter().flat_map(|sl| sl.shapes.iter()).find(|sh| sh.name == "Equation").expect("equation shape");
    let run = &shape.text.as_ref().unwrap().paragraphs[0].runs[0];
    assert!(matches!(&run.kind, RunKind::Math { omml } if omml.contains("<m:f>") && omml.contains("<m:rad>")), "{run:?}");
}

/// The rows of the slide that hold any ink, from the first to the last.
fn ink_rows(s: &mut crate::Session) -> (usize, usize) {
    let st = s.doc().unwrap();
    let img = deckcraft_render::render_slide(&st.doc, st.selection.slide, &deckcraft_render::RenderOpts { scale: 1.0, ..Default::default() });
    let w = img.width as usize;
    let rows: Vec<usize> = (0..img.height as usize).filter(|y| (0..w).any(|x| img.pixels[(y * w + x) * 4] < 128)).collect();
    (*rows.first().unwrap_or(&0), *rows.last().unwrap_or(&0))
}

/// Equations on a slide are drawn as maths: a fraction stands taller than a line of text, its
/// top over its bottom with a bar between.
#[test]
fn a_fraction_on_the_slide_is_drawn_stacked() {
    use serde_json::json;
    let height = |lin: &str| {
        let mut s = crate::Session::with_new();
        s.execute("slide.new", &json!({})).unwrap();
        s.execute("insert.equation", &json!({"linear": lin, "rect": [60, 200, 640, 80]})).unwrap();
        let (a, b) = ink_rows(&mut s);
        b - a
    };
    let (line, fraction, nested) = (height("x=12"), height("(1)/(2)"), height("x=(-b+-sqrt(b^2-4ac))/(2a)"));
    assert!(fraction as f64 > line as f64 * 1.6, "a fraction is two lines tall: {fraction} against {line}");
    assert!(nested > fraction, "the quadratic formula is taller still: {nested} against {fraction}");
}

/// After saving and reopening, the equation is read back from the file and still drawn as maths.
#[test]
fn a_saved_and_reopened_fraction_is_still_drawn_stacked() {
    use serde_json::json;
    let mut s = crate::Session::with_new();
    s.execute("slide.new", &json!({})).unwrap();
    s.execute("insert.equation", &json!({"linear": "(1)/(2)", "rect": [60, 200, 640, 80]})).unwrap();
    let before = ink_rows(&mut s);
    let bytes = deckcraft_pptx::export(&s.doc().unwrap().doc).unwrap();
    let mut t = crate::Session::with_new();
    t.execute("slide.new", &json!({})).unwrap();
    let reopened = deckcraft_pptx::import(&bytes).unwrap();
    let doc = t.doc_mut().unwrap();
    doc.doc = std::sync::Arc::new(reopened);
    doc.selection.slide = s.doc().unwrap().selection.slide;
    let after = ink_rows(&mut t);
    assert!((after.1 - after.0).abs_diff(before.1 - before.0) <= 2, "{before:?} became {after:?}");
}
