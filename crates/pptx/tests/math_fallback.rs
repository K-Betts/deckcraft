//! The plain text that stands in for an equation (for readers without OMML) keeps the size, font
//! and colour the equation was set in, instead of falling back to the 18 pt body default.
//!
//! The deck here is built in code (AGENTS.md §1.1: no decks produced by PowerPoint are committed).

use std::io::{Cursor, Read};

use deckcraft_geom::Xfrm;
use deckcraft_model::text::{Paragraph, Run, RunKind, RunProps, TextBody};
use deckcraft_model::{Shape, ShapeId, defaults};

const OMML: &str = concat!(
    r#"<m:oMathPara xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" "#,
    r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><m:oMath>"#,
    // A bare variable first, as PowerPoint writes it: no size on this run.
    r#"<m:r><a:rPr lang="en-GB"><a:latin typeface="Cambria Math"/></a:rPr><m:t>x</m:t></m:r>"#,
    r#"<m:r><a:rPr lang="en-GB" sz="4000" b="0" i="1"><a:solidFill><a:srgbClr val="112233"/></a:solidFill>"#,
    r#"<a:latin typeface="Cambria Math"/></a:rPr><m:t>+1=8</m:t></m:r>"#,
    r#"</m:oMath></m:oMathPara>"#
);

fn slide_xml(bytes: &[u8]) -> String {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).expect("zip");
    let mut f = z.by_name("ppt/slides/slide1.xml").expect("slide1");
    let mut s = String::new();
    f.read_to_string(&mut s).expect("utf8");
    s
}

fn deck_with_equation() -> deckcraft_model::Presentation {
    let mut p = defaults::new_presentation(None);
    let id = ShapeId(p.alloc_id().expect("id"));
    let eq = Shape {
        id,
        name: "Equation".into(),
        xfrm: Some(Xfrm::new(10.0, 10.0, 200.0, 40.0)),
        text: Some(TextBody {
            paragraphs: vec![Paragraph {
                runs: vec![Run { text: "x+1=8".into(), props: RunProps::default(), kind: RunKind::Math { omml: OMML.into() } }],
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    if let Some(s) = p.slides.first_mut() {
        std::sync::Arc::make_mut(s).shapes = vec![eq];
    }
    p
}

#[test]
fn equation_text_keeps_its_size_font_and_colour_on_import() {
    let q = deckcraft_pptx::import(&deckcraft_pptx::export(&deck_with_equation()).expect("export")).expect("import");
    let shape = q.slides[0].shapes.iter().find(|s| s.name == "Equation").expect("shape");
    let run = shape.text.as_ref().and_then(|t| t.paragraphs.first()).and_then(|p| p.runs.first()).expect("run");
    assert!(matches!(run.kind, RunKind::Math { .. }));
    assert_eq!(run.props.size, Some(40.0));
    assert_eq!(run.props.font.as_deref(), Some("Cambria Math"));
    // The letters are already math-italic characters; no second italic on top.
    assert_eq!(run.props.italic, None);
    assert!(run.props.fill.is_some());
}

#[test]
fn equation_fallback_text_is_written_with_the_equation_size() {
    let q = deckcraft_pptx::import(&deckcraft_pptx::export(&deck_with_equation()).expect("export")).expect("import");
    let xml = slide_xml(&deckcraft_pptx::export(&q).expect("export again"));
    let (choice, fallback) = xml.split_once("<mc:Fallback>").expect("fallback branch");
    assert!(choice.contains("<a14:m"), "the OMML branch is still written");
    assert!(fallback.contains(r#"sz="4000""#), "fallback run lost its size: {fallback}");
    assert!(fallback.contains("Cambria Math"), "fallback run lost its font: {fallback}");
}
