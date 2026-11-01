//! Equations drawn in text: a display equation keeps its own justification, and one inside a
//! sentence is drawn as maths.

use deckcraft_model::text::RunKind;

use super::*;

#[test]
fn display_equation_is_centred_even_in_a_left_aligned_paragraph() {
    const PARA: &str = r#"<m:oMathPara xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:oMath><m:r><m:t>x=3</m:t></m:r></m:oMath></m:oMathPara>"#;
    let (p, mut sh) = setup("x=3", 400.0, 100.0);
    let para = &mut sh.text.as_mut().unwrap().paragraphs[0];
    para.props.align = Some(Align::Left);
    para.props.bullet = Some(Bullet::None);
    para.runs = vec![Run { text: "x=3".into(), props: Default::default(), kind: RunKind::Math { omml: PARA.into() } }];
    let display = lay(&p, &sh, 400.0, 100.0);
    assert!(display.lines[0].caret_x[0] > 100.0, "display equation not centred: {}", display.lines[0].caret_x[0]);

    // Right-justified (m:jc="right") goes to the right edge.
    sh.text.as_mut().unwrap().paragraphs[0].runs[0].kind =
        RunKind::Math { omml: PARA.replace("<m:oMath>", r#"<m:oMathParaPr><m:jc m:val="right"/></m:oMathParaPr><m:oMath>"#) };
    let right = lay(&p, &sh, 400.0, 100.0);
    assert!(right.lines[0].caret_x[0] > display.lines[0].caret_x[0], "right {} centre {}", right.lines[0].caret_x[0], display.lines[0].caret_x[0]);

    // An inline equation (no m:oMathPara) keeps the paragraph's left alignment.
    sh.text.as_mut().unwrap().paragraphs[0].runs[0].kind = RunKind::Math { omml: "<m:oMath><m:r><m:t>x=3</m:t></m:r></m:oMath>".into() };
    let inline = lay(&p, &sh, 400.0, 100.0);
    assert!(inline.lines[0].caret_x[0] < 50.0, "inline equation moved: {}", inline.lines[0].caret_x[0]);
}

#[test]
fn an_equation_inside_a_sentence_is_drawn_as_maths() {
    const FRAC: &str = "<m:oMath><m:f><m:num><m:r><m:t>1</m:t></m:r></m:num><m:den><m:r><m:t>2</m:t></m:r></m:den></m:f></m:oMath>";
    let (p, mut sh) = setup("x", 600.0, 200.0);
    let para = &mut sh.text.as_mut().unwrap().paragraphs[0];
    para.props.bullet = Some(Bullet::None);
    para.runs =
        vec![Run::new("Add "), Run { text: "1/2".into(), props: Default::default(), kind: RunKind::Math { omml: FRAC.into() } }, Run::new(" now")];
    let with = lay(&p, &sh, 600.0, 200.0);
    let plain = {
        let mut q = sh.clone();
        q.text.as_mut().unwrap().paragraphs[0].runs = vec![Run::new("Add 1/2 now")];
        lay(&p, &q, 600.0, 200.0)
    };
    let h = |l: &TextLayout| l.lines[0].bottom - l.lines[0].top;
    assert!(h(&with) > h(&plain), "the stacked fraction makes the line taller: {} vs {}", h(&with), h(&plain));
    assert!(with.decos.len() > plain.decos.len(), "the fraction bar is drawn");
    // Words on either side of it stay on the line, the fraction between them.
    assert!(with.lines[0].caret_x[4] < with.lines[0].caret_x[with.lines[0].end - with.lines[0].start - 4]);
}
