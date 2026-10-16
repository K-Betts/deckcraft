//! The plain text that stands in for an equation: one line per equation of a display group, and
//! spaces round relations and binary operators, as PowerPoint draws them.
//!
//! The deck here is built in code (AGENTS.md §1.1: no decks produced by PowerPoint are committed).

use deckcraft_geom::Xfrm;
use deckcraft_model::text::{Paragraph, Run, RunKind, RunProps, TextBody};
use deckcraft_model::{Shape, ShapeId, defaults};

const NS: &str = r#"xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math""#;

fn omath(parts: &[&str]) -> String {
    let runs: String = parts.iter().map(|t| format!("<m:r><m:t>{t}</m:t></m:r>")).collect();
    format!("<m:oMath>{runs}</m:oMath>")
}

fn imported_text(omml: String) -> String {
    let mut p = defaults::new_presentation(None);
    let id = ShapeId(p.alloc_id().expect("id"));
    let eq = Shape {
        id,
        name: "Equation".into(),
        xfrm: Some(Xfrm::new(10.0, 10.0, 200.0, 40.0)),
        text: Some(TextBody {
            paragraphs: vec![Paragraph {
                runs: vec![Run { text: String::new(), props: RunProps::default(), kind: RunKind::Math { omml } }],
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    if let Some(s) = p.slides.first_mut() {
        std::sync::Arc::make_mut(s).shapes = vec![eq];
    }
    let q = deckcraft_pptx::import(&deckcraft_pptx::export(&p).expect("export")).expect("import");
    let shape = q.slides[0].shapes.iter().find(|s| s.name == "Equation").expect("shape");
    shape.text.as_ref().and_then(|t| t.paragraphs.first()).and_then(|p| p.runs.first()).map(|r| r.text.clone()).expect("run")
}

#[test]
fn equations_of_a_display_group_are_on_separate_lines() {
    let omml = format!("<m:oMathPara {NS}>{}{}</m:oMathPara>", omath(&["y", "=2", "x", "+1"]), omath(&["y", "=3−2", "x"]));
    assert_eq!(imported_text(omml), "y = 2x + 1\u{b}y = 3 − 2x");
}

#[test]
fn relations_and_binary_operators_are_spaced() {
    let omml = format!("<m:oMath {NS}><m:r><m:t>x=0.5</m:t></m:r></m:oMath>");
    assert_eq!(imported_text(omml), "x = 0.5");
}

#[test]
fn a_leading_minus_stays_attached() {
    let omml = format!("<m:oMath {NS}><m:r><m:t>x=−3</m:t></m:r></m:oMath>");
    assert_eq!(imported_text(omml), "x = −3");
}

#[test]
fn a_single_equation_has_no_line_break() {
    let omml = format!("<m:oMathPara {NS}>{}</m:oMathPara>", omath(&["y=2"]));
    assert_eq!(imported_text(omml), "y = 2");
}
