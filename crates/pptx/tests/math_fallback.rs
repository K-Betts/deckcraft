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

fn deck_with_equation_in_a_table_cell() -> deckcraft_model::Presentation {
    use deckcraft_model::ShapeKind;
    let mut p = defaults::new_presentation(None);
    let id = ShapeId(p.alloc_id().expect("id"));
    let mut t = deckcraft_model::Table::new(2, 2, 200.0, 30.0);
    if let Some(cell) = t.rows.first_mut().and_then(|r| r.cells.first_mut()) {
        cell.text = TextBody {
            paragraphs: vec![Paragraph {
                runs: vec![Run { text: "x+1=8".into(), props: RunProps::default(), kind: RunKind::Math { omml: OMML.into() } }],
                ..Default::default()
            }],
            ..Default::default()
        };
    }
    let table = Shape { id, name: "Table".into(), xfrm: Some(Xfrm::new(10.0, 10.0, 200.0, 60.0)), kind: ShapeKind::Table(t), ..Default::default() };
    if let Some(s) = p.slides.first_mut() {
        std::sync::Arc::make_mut(s).shapes = vec![table];
    }
    p
}

fn first_cell_run(p: &deckcraft_model::Presentation) -> Run {
    let shape = p.slides[0].shapes.iter().find(|s| s.name == "Table").expect("table");
    let deckcraft_model::ShapeKind::Table(t) = &shape.kind else { panic!("not a table") };
    t.rows[0].cells[0].text.paragraphs[0].runs[0].clone()
}

#[test]
fn equation_in_a_table_cell_survives_a_round_trip() {
    let bytes = deckcraft_pptx::export(&deck_with_equation_in_a_table_cell()).expect("export");
    let q = deckcraft_pptx::import(&bytes).expect("import");
    let run = first_cell_run(&q);
    // Still an equation (not flattened to text), and still with its formatting.
    match &run.kind {
        RunKind::Math { omml } => assert!(omml.contains("+1=8"), "{omml}"),
        other => panic!("equation in a cell was flattened: {other:?}"),
    }
    assert_eq!(run.props.size, Some(40.0));
    // And a second pass changes nothing.
    let again = deckcraft_pptx::import(&deckcraft_pptx::export(&q).expect("export")).expect("import");
    assert_eq!(first_cell_run(&again).kind, run.kind);
}

#[test]
fn table_with_an_equation_has_an_omml_branch_and_a_formatted_text_fallback() {
    let bytes = deckcraft_pptx::export(&deck_with_equation_in_a_table_cell()).expect("export");
    let q = deckcraft_pptx::import(&bytes).expect("import");
    let xml = slide_xml(&deckcraft_pptx::export(&q).expect("export again"));
    let (choice, fallback) = xml.split_once("<mc:Fallback>").expect("fallback branch");
    assert!(choice.contains("<mc:Choice") && choice.contains("<a:tbl>") && choice.contains("<a14:m"), "{choice}");
    assert!(fallback.contains("<a:tbl>") && !fallback.contains("<a14:m"), "{fallback}");
    assert!(fallback.contains(r#"sz="4000""#), "fallback cell text lost its size: {fallback}");
}

#[test]
fn table_without_equations_is_not_wrapped() {
    let mut p = deck_with_equation_in_a_table_cell();
    if let Some(s) = p.slides.first_mut()
        && let Some(shape) = std::sync::Arc::make_mut(s).shapes.first_mut()
        && let deckcraft_model::ShapeKind::Table(t) = &mut shape.kind
        && let Some(cell) = t.rows.first_mut().and_then(|r| r.cells.first_mut())
    {
        cell.text = TextBody::default();
    }
    let xml = slide_xml(&deckcraft_pptx::export(&p).expect("export"));
    assert!(!xml.contains("mc:AlternateContent"), "{xml}");
}

fn deck_with_display_equation(jc: Option<&str>) -> deckcraft_model::Presentation {
    let jc = jc.map(|v| format!(r#"<m:oMathParaPr><m:jc m:val="{v}"/></m:oMathParaPr>"#)).unwrap_or_default();
    let omml = OMML.replacen("<m:oMath>", &format!("{jc}<m:oMath>"), 1);
    let mut p = deck_with_equation();
    if let Some(s) = p.slides.first_mut()
        && let Some(shape) = std::sync::Arc::make_mut(s).shapes.first_mut()
        && let Some(run) = shape.text.as_mut().and_then(|t| t.paragraphs.first_mut()).and_then(|p| p.runs.first_mut())
    {
        run.kind = RunKind::Math { omml };
    }
    p
}

/// The part of the slide after the fallback opens, and the part before.
fn branches(p: &deckcraft_model::Presentation) -> (String, String) {
    let xml = slide_xml(&deckcraft_pptx::export(p).expect("export"));
    let (choice, fallback) = xml.split_once("<mc:Fallback>").expect("fallback branch");
    (choice.to_string(), fallback.to_string())
}

#[test]
fn equation_wrappers_declare_a14_on_the_alternate_content() {
    for p in [deck_with_equation(), deck_with_equation_in_a_table_cell()] {
        let (choice, _) = branches(&p);
        let ac = choice.split("<mc:AlternateContent").nth(1).expect("alternate content");
        let (wrapper, rest) = ac.split_once("<mc:Choice").expect("choice");
        assert!(wrapper.contains("xmlns:a14="), "a14 is not declared on the wrapper: {wrapper}");
        let choice_tag = rest.split('>').next().expect("choice tag");
        assert!(choice_tag.contains(r#"Requires="a14""#) && !choice_tag.contains("xmlns"), "{choice_tag}");
    }
}

#[test]
fn display_equation_fallback_text_is_centred_like_the_equation() {
    for jc in [None, Some("centerGroup"), Some("center")] {
        let (choice, fallback) = branches(&deck_with_display_equation(jc));
        assert!(fallback.contains(r#"algn="ctr""#), "{jc:?}: {fallback}");
        // The OMML branch is left exactly as it was read.
        assert!(!choice.contains(r#"algn="ctr""#), "{jc:?}: {choice}");
    }
}

#[test]
fn display_equation_fallback_is_centred_even_when_the_paragraph_says_left() {
    // PowerPoint writes algn="l" on text boxes holding a centred display equation.
    let mut p = deck_with_display_equation(None);
    if let Some(s) = p.slides.first_mut()
        && let Some(shape) = std::sync::Arc::make_mut(s).shapes.first_mut()
        && let Some(para) = shape.text.as_mut().and_then(|t| t.paragraphs.first_mut())
    {
        para.props.align = Some(deckcraft_model::text::Align::Left);
    }
    let (choice, fallback) = branches(&p);
    assert!(fallback.contains(r#"algn="ctr""#), "{fallback}");
    // The OMML branch keeps the paragraph as it was read.
    assert!(choice.contains(r#"algn="l""#), "{choice}");
}

#[test]
fn display_equation_fallback_follows_the_equation_alignment() {
    let (_, left) = branches(&deck_with_display_equation(Some("left")));
    assert!(left.contains(r#"algn="l""#), "{left}");
    let (_, right) = branches(&deck_with_display_equation(Some("right")));
    assert!(right.contains(r#"algn="r""#), "{right}");
}
