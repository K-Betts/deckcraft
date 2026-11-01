//! Insert ▸ Equation: a real equation (OMML) typed in linear form, and reading back the typed form
//! of the selected one so the Equation dialog can open on it.

use super::*;
use crate::cmd::targets;

pub(super) fn spec() -> CommandSpec {
    cmd!(
        "insert.equation",
        "Equation",
        ["Insert", "Symbols"],
        None,
        "{linear: string typed as x^2/3, sqrt(x), sum_(i=1)^n i…, rect?: [x,y,w,h]} → {id}",
        has_slide,
        equation
    )
}

pub(super) fn source_spec() -> CommandSpec {
    cmd!(
        query "equation.source",
        "Equation Source",
        [],
        None,
        "{} → {id, linear} for the selected equation, else null",
        has_slide,
        equation_source
    )
}

/// A real equation (OMML, so PowerPoint opens it as one) in a new box, centred on its own line.
/// The box shows the equation's plain text where the editor cannot draw it.
fn equation(s: &mut Session, p: &Value) -> Result<Value> {
    use deckcraft_model::text::{Align, Paragraph, Run, RunProps};
    let linear = str_param(p, "linear").unwrap_or("");
    if linear.trim().is_empty() {
        return Err(bad("insert.equation", "type an equation first"));
    }
    let parsed = crate::equation::parse(linear);
    if crate::equation::has_empty(&parsed) {
        return Err(bad("insert.equation", "fill in the empty boxes first"));
    }
    let text = crate::equation::to_plain(&parsed);
    let size = 28.0;
    let omml = crate::equation::to_omml_sized(&parsed, Some(size));
    if bool_or(p, "replace", false) {
        // Change the equation already on the slide, keeping its box, size and font.
        let out = edit_shapes(s, p, "insert.equation", |sh| {
            let run = sh
                .text
                .as_mut()
                .and_then(|t| t.paragraphs.iter_mut().flat_map(|q| q.runs.iter_mut()).find(|r| matches!(r.kind, RunKind::Math { .. })))
                .ok_or_else(|| bad("insert.equation", "that is not an equation"))?;
            run.text = text.clone();
            let own = crate::equation::to_omml_sized(&parsed, Some(run.props.size.unwrap_or(size)));
            run.kind = RunKind::Math { omml: own };
            Ok(())
        })?;
        if let Some(id) = targets(s, p)?.first() {
            crate::cmd::text::fit_text_box(s, *id)?;
        }
        return Ok(out);
    }
    let rect = rect_param(p, "rect").unwrap_or_else(|| default_rect(s, 360.0, 60.0));
    let mut shape = new_text_box(rect, "");
    let props = RunProps { font: Some("Cambria Math".into()), size: Some(size), ..Default::default() };
    let run = Run { text, props, kind: RunKind::Math { omml } };
    if let Some(t) = shape.text.as_mut() {
        let mut para = Paragraph { runs: vec![run], ..Default::default() };
        para.props.align = Some(Align::Center);
        t.paragraphs = vec![para];
    }
    shape.name = "Equation".into();
    let id = add_shape(s, "insert.equation", shape, true)?;
    crate::cmd::text::fit_text_box(s, id)?;
    Ok(json!({"id": id}))
}

/// The typed form of the one selected equation, so the Equation dialog can open on it.
fn equation_source(s: &mut Session, _p: &Value) -> Result<Value> {
    let st = s.doc()?;
    let [id] = st.selection.shapes[..] else { return Ok(Value::Null) };
    let Some(shape) = st.shape(id) else { return Ok(Value::Null) };
    let math = shape.text.as_ref().and_then(|t| {
        let mut runs = t.paragraphs.iter().flat_map(|q| q.runs.iter());
        runs.find_map(|r| match &r.kind {
            RunKind::Math { omml } => Some(omml.clone()),
            _ => None,
        })
    });
    let Some(expr) = math.and_then(|o| crate::equation::from_omml(&o)) else { return Ok(Value::Null) };
    Ok(json!({"id": id.0, "linear": crate::equation::to_linear(&expr)}))
}
