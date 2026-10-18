//! Figures for Get Ready questions: triangles, angles, circles, solids, charts, number lines,
//! graphs, Venn and tree diagrams. Each is drawn from ordinary shapes and text boxes in the area
//! it is given, so a figure saves to PowerPoint like anything else and can be edited by hand.
//!
//! A question names its figure with `"diagram": {"kind": …, …}`; `crate::questions::DRAWABLE`
//! lists the kinds and what each one reads is described beside its drawing function. A label that
//! is `null` is the unknown, drawn as `?` (or as the figure's `unknown` letter where it has one).

use serde_json::{Value, json};

use super::get_ready::{draw_line, draw_shape, rich_text_box};
use crate::{Result, Session};

const INK: &str = "#111827";
const LIGHT: &str = "#D1D5DB";
const BLUE: &str = "#4A7BD0";

type P = [f64; 2];

thread_local! {
    /// How much larger than a quarter-slide card the figure being drawn is (labels and lines scale with it).
    static SCALE: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

fn k() -> f64 {
    SCALE.with(std::cell::Cell::get)
}

/// Draws the figure `d` inside `area` = [x, y, w, h].
pub(crate) fn draw(s: &mut Session, d: &Value, area: [f64; 4], scale: f64) -> Result<()> {
    SCALE.with(|c| c.set(scale.max(0.5)));
    let kind = d.get("kind").and_then(Value::as_str).unwrap_or("");
    let f = Fig { d, area };
    match kind {
        "triangle-angles" => f.triangle_angles(s),
        "isosceles-triangle" => f.isosceles(s),
        "parallel-lines" => f.parallel(s),
        "right-triangle" => f.right_triangle(s),
        "angles-line" => f.angles_fan(s, 180.0),
        "angles-point" => f.angles_fan(s, 360.0),
        "circle" => f.circle(s),
        "rectangle" => f.rectangle(s),
        "l-shape" => f.l_shape(s),
        "trapezium" => f.trapezium(s),
        "parallelogram" => f.parallelogram(s),
        "cuboid" => f.cuboid(s),
        "bar-chart" => f.bar_chart(s),
        "number-line" => f.number_line(s),
        "coordinate-grid" => f.grid(s),
        "venn" => f.venn(s),
        "tree" => f.tree(s),
        "regular-polygon" => f.polygon(s),
        "similar-triangles" => f.similar(s),
        "function-machine" => f.function_machine(s),
        "sequence-arrows" => f.sequence_arrows(s),
        "pattern" => f.pattern(s),
        "bar-model" => f.bar_model(s),
        "table" => f.table(s),
        "fraction-bar" => f.fraction_bar(s),
        "shade-grid" => f.shade_grid(s),
        _ => Ok(()),
    }
}

/// Figures that want to be wider than tall, and so take more of a card.
pub(crate) fn is_wide(d: &Value) -> bool {
    matches!(
        d.get("kind").and_then(Value::as_str),
        Some(
            "bar-chart"
                | "number-line"
                | "tree"
                | "similar-triangles"
                | "venn"
                | "l-shape"
                | "cuboid"
                | "function-machine"
                | "sequence-arrows"
                | "pattern"
                | "bar-model"
                | "table"
        )
    )
}

struct Fig<'a> {
    d: &'a Value,
    area: [f64; 4],
}

/// A number without floating-point noise: `0.30000000000000004` is `0.3`.
fn num_text(v: f64) -> String {
    let r = (v * 1e6).round() / 1e6;
    if (r - r.round()).abs() < 1e-9 { format!("{}", r.round() as i64) } else { format!("{r}") }
}

fn pt(a: P, dx: f64, dy: f64) -> P {
    [a[0] + dx, a[1] + dy]
}

fn mid(a: P, b: P) -> P {
    [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
}

fn ln(s: &mut Session, a: P, b: P) -> Result<()> {
    draw_line(s, a, b, INK, 2.5 * k().sqrt())?;
    Ok(())
}

fn thin(s: &mut Session, a: P, b: P, color: &str) -> Result<u64> {
    draw_line(s, a, b, color, 1.0)
}

fn dashed(s: &mut Session, a: P, b: P) -> Result<()> {
    let id = draw_line(s, a, b, INK, 1.5 * k().sqrt())?;
    s.execute("shape.line", &json!({"ids": [id], "dash": "dash"}))?;
    Ok(())
}

fn poly(s: &mut Session, pts: &[P]) -> Result<()> {
    for i in 0..pts.len() {
        ln(s, pts[i], pts[(i + 1) % pts.len()])?;
    }
    Ok(())
}

/// A small label centred on `at`; `$…$` is drawn as maths.
fn label(s: &mut Session, text: &str, at: P) -> Result<()> {
    label_sized(s, text, at, 15.0)
}

fn label_sized(s: &mut Session, text: &str, at: P, size: f64) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    let size = size * k();
    let chars = crate::latex::plain(text).chars().count() as f64;
    let (w, h) = ((chars * size * 0.63 + 16.0).max(30.0), size * 1.6);
    let id = rich_text_box(s, text, [at[0] - w / 2.0, at[1] - h / 2.0, w, h], size, INK)?;
    center(s, id)
}

fn center(s: &mut Session, id: u64) -> Result<()> {
    s.edit(|doc, sel| {
        let list = crate::shapes_mut(doc, sel).ok_or_else(|| crate::EngineError::Other("no slide".into()))?;
        for para in list.iter_mut().filter(|sh| u64::from(sh.id.0) == id).filter_map(|sh| sh.text.as_mut()).flat_map(|t| t.paragraphs.iter_mut()) {
            para.props.align = Some(deckcraft_model::text::Align::Center);
        }
        Ok(())
    })
}

/// A right-angle mark in the corner `at`, its sides running towards `a` and `b`.
fn right_angle(s: &mut Session, at: P, a: P, b: P) -> Result<()> {
    let unit = |p: P| {
        let (dx, dy) = (p[0] - at[0], p[1] - at[1]);
        let n = (dx * dx + dy * dy).sqrt().max(1e-9);
        [dx / n * 11.0, dy / n * 11.0]
    };
    let (u, v) = (unit(a), unit(b));
    let c = [at[0] + u[0] + v[0], at[1] + u[1] + v[1]];
    draw_line(s, pt(at, u[0], u[1]), c, INK, 1.5)?;
    draw_line(s, pt(at, v[0], v[1]), c, INK, 1.5)?;
    Ok(())
}

impl Fig<'_> {
    fn text(&self, key: &str) -> Option<String> {
        self.d.get(key).and_then(Value::as_str).map(str::to_string)
    }

    /// A label from the spec: a missing key draws nothing, `null` is the unknown.
    fn lab(&self, key: &str) -> Option<String> {
        match self.d.get(key) {
            None => None,
            Some(Value::Null) => Some(self.unknown()),
            Some(v) => v.as_str().map(str::to_string),
        }
    }

    fn unknown(&self) -> String {
        self.text("unknown").map(|u| format!("${u}$")).unwrap_or_else(|| "?".into())
    }

    fn list(&self, key: &str) -> Vec<Option<String>> {
        self.d.get(key).and_then(Value::as_array).map(|v| v.iter().map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
    }

    /// `sizes`: the true sizes in degrees to draw the angles at, as numbers or numeric strings.
    fn sizes(&self) -> Option<Vec<f64>> {
        let v: Vec<f64> = self
            .d
            .get("sizes")?
            .as_array()?
            .iter()
            .filter_map(|x| x.as_f64().or_else(|| x.as_str().and_then(|s| s.trim().trim_end_matches('°').parse().ok())))
            .collect();
        (!v.is_empty() && v.iter().all(|x| *x > 0.0)).then_some(v)
    }

    fn num(&self, key: &str, default: f64) -> f64 {
        self.d.get(key).and_then(Value::as_f64).unwrap_or(default)
    }

    /// The largest `aspect` (w / h) box that fits the area, centred, with `pad` kept clear.
    fn fit(&self, aspect: f64, pad: f64) -> [f64; 4] {
        let [ax, ay, aw, ah] = self.area;
        let (aw, ah) = ((aw - 2.0 * pad).max(10.0), (ah - 2.0 * pad).max(10.0));
        let (w, h) = if aw / ah > aspect { (ah * aspect, ah) } else { (aw, aw / aspect) };
        [ax + pad + (aw - w) / 2.0, ay + pad + (ah - h) / 2.0, w, h]
    }

    // ── triangles ────────────────────────────────────────────────────────────────────────────

    /// `angles`: three labels, `null` the unknown (`unknown`: the letter to write there). With
    /// `sizes` (the three true angles) the triangle is drawn to those angles, so what pupils see
    /// agrees with the numbers.
    fn triangle_angles(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.5, 36.0);
        let (a, b, c) = match self.sizes().filter(|v| v.len() == 3 && (v.iter().sum::<f64>() - 180.0).abs() < 1.0) {
            Some(v) => {
                // Base from A (bottom left) to B; the apex follows from the angles at A and B.
                let (ta, tb) = (v[0].to_radians(), v[1].to_radians());
                let apex_x = (tb.tan() / (ta.tan() + tb.tan())) * 1.0;
                let height = apex_x * ta.tan();
                // Unit base: fit the shape in the area.
                let scale = (w / 1.0).min(h / height.max(0.05));
                let (bw, bh) = (scale, height * scale);
                let (ox, oy) = (x + (w - bw) / 2.0, y + (h - bh) / 2.0 + bh);
                ([ox, oy], [ox + bw, oy], [ox + apex_x * bw, oy - bh])
            }
            None => ([x, y + h], [x + w, y + h], [x + w * 0.66, y]),
        };
        poly(s, &[a, b, c])?;
        let unknown = self.unknown();
        let angles: Vec<String> = self.list("angles").into_iter().map(|v| v.unwrap_or_else(|| unknown.clone())).collect();
        // Each label sits just outside its corner, away from the middle, so they never crowd.
        let centre = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0];
        for (t, v) in angles.iter().zip([a, b, c]) {
            let (dx, dy) = (v[0] - centre[0], v[1] - centre[1]);
            let n = (dx * dx + dy * dy).sqrt().max(1.0);
            let chars = crate::latex::plain(t).chars().count() as f64;
            let out = (14.0 + chars * 4.5) * k();
            label(s, t, [v[0] + dx / n * out, v[1] + dy / n * out * 0.8])?;
        }
        Ok(())
    }

    /// `topAngle`: the angle at A; the base angles are marked equal and one is asked for.
    fn isosceles(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.3, 24.0);
        let (a, b, c) = ([x + w / 2.0, y], [x, y + h], [x + w, y + h]);
        poly(s, &[a, b, c])?;
        for (p, q) in [(a, b), (a, c)] {
            let m = mid(p, q);
            let sign = if q[0] < p[0] { 1.0 } else { -1.0 };
            draw_line(s, pt(m, -5.0 * sign, -5.0), pt(m, 5.0 * sign, 5.0), INK, 2.0)?;
        }
        label(s, "$A$", pt(a, 0.0, -14.0))?;
        label(s, "$B$", pt(b, -12.0, 12.0))?;
        label(s, "$C$", pt(c, 12.0, 12.0))?;
        label(s, &self.text("topAngle").unwrap_or_default(), pt(a, 0.0, 36.0))?;
        label(s, "?", pt(b, 42.0, -12.0))
    }

    /// `base`, `height`, `hyp`: side labels; `angle`: the angle at the right-hand end of the base.
    fn right_triangle(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.35, 26.0);
        let (c, b, a) = ([x, y + h], [x + w, y + h], [x, y]);
        poly(s, &[a, b, c])?;
        right_angle(s, c, a, b)?;
        if let Some(t) = self.lab("height") {
            label(s, &t, pt(mid(a, c), -30.0, 0.0))?;
        }
        if let Some(t) = self.lab("base") {
            label(s, &t, pt(mid(c, b), 0.0, 18.0))?;
        }
        if let Some(t) = self.lab("hyp") {
            label(s, &t, pt(mid(a, b), 30.0, -16.0))?;
        }
        if let Some(t) = self.lab("angle") {
            label(s, &t, pt(b, -46.0, -12.0))?;
        }
        Ok(())
    }

    /// `small` and `large`: three side labels each (base, height, slant), `null` the unknown.
    fn similar(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(2.2, 22.0);
        let (sw, sh) = (w * 0.3, h * 0.5);
        let small = [[x, y + h], [x + sw, y + h], [x + sw * 0.2, y + h - sh]];
        let (lw, lh) = (w * 0.52, h);
        let large = [[x + w - lw, y + h], [x + w, y + h], [x + w - lw + lw * 0.2, y + h - lh]];
        for (tri, key) in [(small, "small"), (large, "large")] {
            poly(s, &tri)?;
            let labels = self.list(key);
            let spots = [pt(mid(tri[0], tri[1]), 0.0, 16.0), pt(mid(tri[1], tri[2]), 26.0, 0.0), pt(mid(tri[0], tri[2]), -26.0, 0.0)];
            for (i, at) in spots.iter().enumerate() {
                match labels.get(i) {
                    Some(Some(t)) => label_sized(s, t, *at, 12.0)?,
                    Some(None) => label_sized(s, &self.unknown(), *at, 12.0)?,
                    None => {}
                }
            }
        }
        Ok(())
    }

    // ── angles ───────────────────────────────────────────────────────────────────────────────

    /// `angles`: labels for the angles on a straight line (`total` 180) or round a point (360).
    /// Angles written as degrees are drawn to size; the unknowns share what is left.
    fn angles_fan(&self, s: &mut Session, total: f64) -> Result<()> {
        let [x, y, w, h] = self.fit(if total > 200.0 { 1.0 } else { 1.8 }, 20.0);
        let r = if total > 200.0 { w.min(h) / 2.0 } else { (w / 2.0).min(h) };
        let c = if total > 200.0 { [x + w / 2.0, y + h / 2.0] } else { [x + w / 2.0, y + h] };
        let unknown = self.unknown();
        let labels = self.list("angles");
        let known: Vec<Option<f64>> = match self.sizes() {
            Some(sizes) => sizes.into_iter().map(Some).collect(),
            None => labels
                .iter()
                .map(|l| l.as_ref().and_then(|t| t.trim().trim_end_matches('°').trim_end_matches("\\circ").trim().parse::<f64>().ok()))
                .collect(),
        };
        let used: f64 = known.iter().flatten().sum();
        let open = known.iter().filter(|k| k.is_none()).count().max(1) as f64;
        let share = ((total - used) / open).max(total / 12.0);
        let sizes: Vec<f64> = known.iter().map(|k| k.unwrap_or(share)).collect();
        let sum: f64 = sizes.iter().sum::<f64>().max(1.0);
        let mut at = if total > 200.0 { 20.0 } else { 0.0 };
        let rad = |deg: f64| deg.to_radians();
        let on = |deg: f64, len: f64| [c[0] + len * rad(deg).cos(), c[1] - len * rad(deg).sin()];
        if total <= 200.0 {
            ln(s, on(0.0, r), on(180.0, r))?;
        }
        for (i, size) in sizes.iter().enumerate() {
            let size = size / sum * total;
            ln(s, c, on(at, r))?;
            let text = labels.get(i).cloned().flatten().unwrap_or_else(|| unknown.clone());
            label(s, &text, on(at + size / 2.0, r * 0.55))?;
            at += size;
        }
        if total > 200.0 {
            // The last ray closes the turn.
        } else {
            ln(s, c, on(at.min(180.0), r))?;
        }
        Ok(())
    }

    /// Two parallel lines cut by a transversal; `knownAngle` and `unknown` are co-interior.
    fn parallel(&self, s: &mut Session) -> Result<()> {
        let [ax, ay, aw, ah] = self.area;
        let (y1, y2) = (ay + ah * 0.3, ay + ah * 0.7);
        for y in [y1, y2] {
            ln(s, [ax + aw * 0.04, y], [ax + aw * 0.96, y])?;
            draw_line(s, [ax + aw * 0.82, y - 5.0], [ax + aw * 0.82 + 8.0, y], INK, 2.0)?;
            draw_line(s, [ax + aw * 0.82, y + 5.0], [ax + aw * 0.82 + 8.0, y], INK, 2.0)?;
        }
        let on_t = |y: f64| {
            let t = ((y - ay) / ah - 0.04) / 0.92;
            ax + aw * (0.28 + 0.44 * t)
        };
        ln(s, [ax + aw * 0.28, ay + ah * 0.04], [ax + aw * 0.72, ay + ah * 0.96])?;
        label(s, &self.text("knownAngle").unwrap_or_default(), [on_t(y1) + 38.0, y1 + 22.0])?;
        label(s, &format!("${}$", self.text("unknown").unwrap_or_else(|| "x".into())), [on_t(y2) + 30.0, y2 - 22.0])
    }

    // ── circles and polygons ─────────────────────────────────────────────────────────────────

    /// `radius` or `diameter`: its label; `sector`: an angle label, drawn with two radii; `centre`.
    fn circle(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.0, 22.0);
        draw_shape(s, "ellipse", [x, y, w, h], None, Some((INK, 2.5)))?;
        let (c, r) = ([x + w / 2.0, y + h / 2.0], w / 2.0);
        draw_shape(s, "ellipse", [c[0] - 2.5, c[1] - 2.5, 5.0, 5.0], Some(INK), None)?;
        if let Some(o) = self.text("centre") {
            label(s, &format!("${o}$"), pt(c, -12.0, 12.0))?;
        }
        let sector = self.lab("sector");
        // Half the sector's angle, from its label (60° when it cannot be read).
        let half =
            sector.as_deref().and_then(|t| t.trim().trim_end_matches('°').trim().parse::<f64>().ok()).unwrap_or(60.0).clamp(20.0, 160.0).to_radians()
                / 2.0;
        if let Some(t) = self.lab("diameter") {
            ln(s, pt(c, -r, 0.0), pt(c, r, 0.0))?;
            label(s, &t, pt(c, 0.0, -14.0))?;
        } else if let Some(t) = self.lab("radius") {
            if sector.is_some() {
                let end = pt(c, r * half.cos(), r * half.sin());
                label(s, &t, pt(mid(c, end), 4.0, 16.0))?;
            } else {
                let end = pt(c, r * 0.866, -r * 0.5);
                ln(s, c, end)?;
                label(s, &t, pt(mid(c, end), -6.0, -16.0))?;
            }
        }
        if let Some(t) = sector {
            ln(s, c, pt(c, r * half.cos(), -r * half.sin()))?;
            ln(s, c, pt(c, r * half.cos(), r * half.sin()))?;
            label_sized(s, &t, pt(c, r * 0.62, 0.0), 13.0)?;
        }
        Ok(())
    }

    /// `sides` (5 to 8): a regular polygon; `angle`: a label inside (interior or exterior).
    fn polygon(&self, s: &mut Session) -> Result<()> {
        let n = self.num("sides", 6.0).clamp(5.0, 8.0) as usize;
        let [x, y, w, h] = self.fit(1.0, 18.0);
        let (c, r) = ([x + w / 2.0, y + h / 2.0], w / 2.0);
        let verts: Vec<P> = (0..n)
            .map(|i| {
                let t = std::f64::consts::TAU * (i as f64) / (n as f64) - std::f64::consts::FRAC_PI_2;
                [c[0] + r * t.cos(), c[1] + r * t.sin()]
            })
            .collect();
        poly(s, &verts)?;
        if let Some(t) = self.lab("angle") {
            if self.text("at").as_deref() == Some("exterior") {
                let (a, b) = (verts[0], verts[1]);
                let d = [b[0] - a[0], b[1] - a[1]];
                let len = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-9);
                let ext = pt(b, d[0] / len * 34.0, d[1] / len * 34.0);
                dashed(s, b, ext)?;
                label(s, &t, pt(ext, 12.0, -14.0))?;
            } else {
                label(s, &t, pt(verts[0], 0.0, 26.0))?;
            }
        }
        if let Some(t) = self.text("centreLabel") {
            label(s, &t, c)?;
        }
        Ok(())
    }

    // ── four-sided shapes and solids ─────────────────────────────────────────────────────────

    /// `width`, `height`: side labels.
    fn rectangle(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.5, 28.0);
        draw_shape(s, "rect", [x, y, w, h], Some("#FFFFFF"), Some((INK, 2.5)))?;
        if let Some(t) = self.lab("width") {
            label(s, &t, [x + w / 2.0, y + h + 16.0])?;
        }
        if let Some(t) = self.lab("height") {
            label(s, &t, [x - 30.0, y + h / 2.0])?;
        }
        if let Some(t) = self.text("centre") {
            label(s, &t, [x + w / 2.0, y + h / 2.0])?;
        }
        Ok(())
    }

    /// `labels`: the six sides of an L, from the top edge clockwise.
    fn l_shape(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.3, 30.0);
        let (cw, ch) = (w * 0.45, h * 0.5);
        let p = [[x, y], [x + cw, y], [x + cw, y + ch], [x + w, y + ch], [x + w, y + h], [x, y + h]];
        poly(s, &p)?;
        let labels = self.list("labels");
        let spots = [
            pt(mid(p[0], p[1]), 0.0, -14.0),
            pt(mid(p[1], p[2]), 26.0, 0.0),
            pt(mid(p[2], p[3]), 0.0, 16.0),
            pt(mid(p[3], p[4]), 26.0, 0.0),
            pt(mid(p[4], p[5]), 0.0, 16.0),
            pt(mid(p[5], p[0]), -28.0, 0.0),
        ];
        for (i, at) in spots.iter().enumerate() {
            match labels.get(i) {
                Some(Some(t)) => label(s, t, *at)?,
                Some(None) => label(s, &self.unknown(), *at)?,
                None => {}
            }
        }
        Ok(())
    }

    /// `top`, `bottom`, `height`: side labels; the height is a dashed line.
    fn trapezium(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.8, 28.0);
        let inset = w * 0.2;
        let p = [[x + inset, y], [x + w - inset * 0.4, y], [x + w, y + h], [x, y + h]];
        poly(s, &p)?;
        let foot = [x + inset, y + h];
        dashed(s, p[0], foot)?;
        right_angle(s, foot, p[0], [x + w, y + h])?;
        if let Some(t) = self.lab("top") {
            label(s, &t, pt(mid(p[0], p[1]), 0.0, -14.0))?;
        }
        if let Some(t) = self.lab("bottom") {
            label(s, &t, pt(mid(p[2], p[3]), 0.0, 16.0))?;
        }
        if let Some(t) = self.lab("height") {
            label(s, &t, pt(mid(p[0], foot), 24.0, 0.0))?;
        }
        Ok(())
    }

    /// `base`, `height`, `side`: labels; the height is a dashed line.
    fn parallelogram(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.8, 28.0);
        let slant = w * 0.22;
        let p = [[x + slant, y], [x + w, y], [x + w - slant, y + h], [x, y + h]];
        poly(s, &p)?;
        let foot = [x + slant, y + h];
        dashed(s, p[0], foot)?;
        right_angle(s, foot, p[0], p[2])?;
        if let Some(t) = self.lab("base") {
            label(s, &t, pt(mid(p[2], p[3]), 0.0, 16.0))?;
        }
        if let Some(t) = self.lab("height") {
            label(s, &t, pt(mid(p[0], foot), 24.0, 0.0))?;
        }
        if let Some(t) = self.lab("side") {
            label(s, &t, pt(mid(p[0], p[3]), -26.0, 0.0))?;
        }
        Ok(())
    }

    /// `length`, `width`, `height`: edge labels on a cuboid, hidden edges dashed.
    fn cuboid(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.5, 32.0);
        let (dx, dy) = (w * 0.28, h * 0.32);
        let fw = w - dx;
        let f = [[x, y + dy], [x + fw, y + dy], [x + fw, y + h], [x, y + h]];
        let b: Vec<P> = f.iter().map(|p| pt(*p, dx, -dy)).collect();
        poly(s, &f)?;
        ln(s, b[0], b[1])?;
        ln(s, b[1], b[2])?;
        ln(s, f[0], b[0])?;
        ln(s, f[1], b[1])?;
        ln(s, f[2], b[2])?;
        dashed(s, b[0], b[3])?;
        dashed(s, b[3], b[2])?;
        dashed(s, f[3], b[3])?;
        if let Some(t) = self.lab("length") {
            label(s, &t, pt(mid(f[2], f[3]), 0.0, 16.0))?;
        }
        if let Some(t) = self.lab("height") {
            label(s, &t, pt(mid(f[0], f[3]), -28.0, 0.0))?;
        }
        if let Some(t) = self.lab("width") {
            label(s, &t, pt(mid(f[0], b[0]), -14.0, -14.0))?;
        }
        Ok(())
    }

    // ── charts, lines and graphs ─────────────────────────────────────────────────────────────

    /// `categories` and `values`; optional `yLabel`.
    fn bar_chart(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.5, 14.0);
        let cats = self.list("categories");
        let vals: Vec<f64> = self.d.get("values").and_then(Value::as_array).map(|v| v.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
        let n = cats.len().min(vals.len());
        if n == 0 {
            return Ok(());
        }
        let top = vals.iter().cloned().fold(1.0, f64::max);
        let step = if top <= 10.0 {
            2.0
        } else if top <= 30.0 {
            5.0
        } else if top <= 60.0 {
            10.0
        } else {
            20.0
        };
        let max = (top / step).ceil() * step;
        let (left, bottom) = (x + 34.0, y + h - 22.0);
        let (pw, ph) = (w - 40.0, h - 30.0);
        let mut v = 0.0;
        while v <= max + 1e-9 {
            let yy = bottom - ph * v / max;
            thin(s, [left, yy], [left + pw, yy], LIGHT)?;
            label_sized(s, &format!("{v}"), [left - 14.0, yy], 11.0)?;
            v += step;
        }
        ln(s, [left, bottom - ph], [left, bottom])?;
        ln(s, [left, bottom], [left + pw, bottom])?;
        let slot = pw / n as f64;
        for (i, val) in vals.iter().take(n).enumerate() {
            let bh = ph * val / max;
            draw_shape(s, "rect", [left + slot * i as f64 + slot * 0.2, bottom - bh, slot * 0.6, bh], Some(BLUE), Some((INK, 1.5)))?;
            if let Some(Some(c)) = cats.get(i) {
                label_sized(s, c, [left + slot * (i as f64 + 0.5), bottom + 12.0], 10.0)?;
            }
        }
        if let Some(t) = self.text("yLabel") {
            label(s, &t, [x + 30.0, y - 4.0])?;
        }
        Ok(())
    }

    /// `min`, `max`, `step`; `marks`: `{at, open, to: "left"|"right"|"none"}` for an inequality;
    /// `dots`: values to mark, each a number or `{at, label}` (a letter above the point);
    /// `labelEvery`: label every nth tick; `hideLabels`: no tick numbers; `blank`: tick values to
    /// show as `?`.
    fn number_line(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(5.0, 8.0);
        let (lo, hi, step) = (self.num("min", -5.0), self.num("max", 5.0), self.num("step", 1.0).max(1e-6));
        let (l, r, yy) = (x + 12.0, x + w - 12.0, y + h / 2.0);
        let at = |v: f64| l + (r - l) * (v - lo) / (hi - lo).max(1e-9);
        ln(s, [l - 8.0, yy], [r + 8.0, yy])?;
        draw_line(s, [r + 8.0, yy], [r - 2.0, yy - 6.0], INK, 2.5)?;
        draw_line(s, [r + 8.0, yy], [r - 2.0, yy + 6.0], INK, 2.5)?;
        draw_line(s, [l - 8.0, yy], [l + 2.0, yy - 6.0], INK, 2.5)?;
        draw_line(s, [l - 8.0, yy], [l + 2.0, yy + 6.0], INK, 2.5)?;
        let every = self.num("labelEvery", 1.0).max(1.0) as usize;
        let hide = self.d.get("hideLabels").and_then(Value::as_bool).unwrap_or(false);
        let blanks: Vec<f64> =
            self.d.get("blank").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
        let n = ((hi - lo) / step + 1e-6).floor() as usize;
        // `tickLabels`: a text for each tick, left to right (fractions: "$\\frac{1}{4}$"; null leaves it blank).
        let custom: Vec<Option<String>> = self.list("tickLabels");
        for i in 0..=n {
            let v = lo + step * i as f64;
            let big = i % every == 0 || !custom.is_empty();
            draw_line(s, [at(v), yy - if big { 7.0 } else { 4.0 }], [at(v), yy + if big { 7.0 } else { 4.0 }], INK, 1.5)?;
            if !custom.is_empty() {
                if let Some(Some(text)) = custom.get(i) {
                    label_sized(s, text, [at(v), yy + 22.0], 12.0)?;
                }
            } else if big && !hide {
                let text = if blanks.iter().any(|b| (b - v).abs() < 1e-9) { "?".to_string() } else { num_text(v) };
                label_sized(s, &text, [at(v), yy + 20.0], 12.0)?;
            }
        }
        for m in self.d.get("marks").and_then(Value::as_array).into_iter().flatten() {
            let v = m.get("at").and_then(Value::as_f64).unwrap_or(0.0);
            let open = m.get("open").and_then(Value::as_bool).unwrap_or(false);
            match m.get("to").and_then(Value::as_str).unwrap_or("none") {
                "right" => {
                    draw_line(s, [at(v), yy], [r + 4.0, yy], BLUE, 5.0)?;
                }
                "left" => {
                    draw_line(s, [l - 4.0, yy], [at(v), yy], BLUE, 5.0)?;
                }
                _ => {}
            }
            draw_shape(s, "ellipse", [at(v) - 6.0, yy - 6.0, 12.0, 12.0], Some(if open { "#FFFFFF" } else { BLUE }), Some((BLUE, 2.5)))?;
        }
        for dot in self.d.get("dots").and_then(Value::as_array).into_iter().flatten() {
            let (v, name) = match dot {
                Value::Number(n) => (n.as_f64().unwrap_or(0.0), None),
                o => (o.get("at").and_then(Value::as_f64).unwrap_or(0.0), o.get("label").and_then(Value::as_str).map(str::to_string)),
            };
            draw_shape(s, "ellipse", [at(v) - 5.0, yy - 5.0, 10.0, 10.0], Some(BLUE), Some((BLUE, 1.5)))?;
            if let Some(n) = name {
                label(s, &n, [at(v), yy - 20.0])?;
            }
        }
        Ok(())
    }

    // ── number work: function machines, sequences, bar models, tables ────────────────────────

    /// `input`, `steps` (operations like `"× 3"`, `null` = hidden), `output`; `null` input or
    /// output is the unknown (`?`). Boxes joined by arrows, left to right.
    fn function_machine(&self, s: &mut Session) -> Result<()> {
        let steps: Vec<Option<String>> = self.list("steps");
        let n = steps.len() + 2;
        let [x, y, w, h] = self.fit((n as f64 * 1.15).max(3.0), 6.0);
        let slot = w / n as f64;
        let bw = (slot * 0.62).min(120.0 * k());
        let bh = (h * 0.6).min(70.0 * k());
        let cy = y + h / 2.0;
        let cell = |i: usize| x + slot * (i as f64 + 0.5);
        let end_text = |key: &str| self.lab_or(key, "?");
        let mut boxes: Vec<(String, &str, &str)> = vec![(end_text("input"), "#FFFFFF", "rect")];
        for st in &steps {
            boxes.push((st.clone().unwrap_or_else(|| "?".into()), "#FDE9B8", "rect"));
        }
        boxes.push((end_text("output"), "#FFFFFF", "rect"));
        for (i, (text, fill, _)) in boxes.iter().enumerate() {
            draw_shape(s, "rect", [cell(i) - bw / 2.0, cy - bh / 2.0, bw, bh], Some(fill), Some((INK, 2.5)))?;
            label_sized(s, text, [cell(i), cy], 17.0)?;
            if i + 1 < boxes.len() {
                let id = draw_line(s, [cell(i) + bw / 2.0 + 3.0, cy], [cell(i + 1) - bw / 2.0 - 3.0, cy], INK, 2.5)?;
                s.execute("shape.line", &json!({"ids": [id], "tail": "triangle"}))?;
            }
        }
        Ok(())
    }

    /// A key's label, or `fallback` when it is `null` (the unknown); empty when the key is absent.
    fn lab_or(&self, key: &str, fallback: &str) -> String {
        match self.d.get(key) {
            None | Some(Value::Null) => fallback.to_string(),
            Some(v) => v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string()),
        }
    }

    /// `terms`: the numbers, `null` a gap (`?`); `rule`: a label for the step between terms, one
    /// text for all or a list (`null` entries show `?`).
    fn sequence_arrows(&self, s: &mut Session) -> Result<()> {
        let terms: Vec<Option<String>> = self
            .d
            .get("terms")
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|v| v.as_str().map(str::to_string).or_else(|| v.as_f64().map(num_text))).collect())
            .unwrap_or_default();
        let n = terms.len().max(2);
        let [x, y, w, h] = self.fit((n as f64 * 1.0).max(3.0), 6.0);
        let slot = w / n as f64;
        let cy = y + h * 0.62;
        let r = (slot * 0.34).min(h * 0.28).min(28.0 * k());
        let rules: Vec<String> = match self.d.get("rule") {
            Some(Value::Array(a)) => a.iter().map(|v| v.as_str().unwrap_or("?").to_string()).collect(),
            Some(Value::String(t)) => vec![t.clone(); n.saturating_sub(1)],
            _ => vec![],
        };
        for i in 0..n {
            let cx = x + slot * (i as f64 + 0.5);
            draw_shape(s, "ellipse", [cx - r, cy - r, 2.0 * r, 2.0 * r], Some("#FFFFFF"), Some((INK, 2.5)))?;
            let text = terms.get(i).cloned().flatten().unwrap_or_else(|| "?".into());
            label_sized(s, &text, [cx, cy], 17.0)?;
            if i + 1 < n {
                let nx = x + slot * (i as f64 + 1.5);
                let (a, b) = ([cx + r * 0.8, cy - r * 0.6], [nx - r * 0.8, cy - r * 0.6]);
                // An arc over the top: up, across, down.
                let top = cy - r - 14.0 * k();
                draw_line(s, a, [a[0] + 4.0, top], INK, 2.0)?;
                draw_line(s, [a[0] + 4.0, top], [b[0] - 4.0, top], INK, 2.0)?;
                let id = draw_line(s, [b[0] - 4.0, top], b, INK, 2.0)?;
                s.execute("shape.line", &json!({"ids": [id], "tail": "triangle"}))?;
                if let Some(rule) = rules.get(i) {
                    label_sized(s, rule, [(a[0] + b[0]) / 2.0, top - 12.0 * k()], 13.0)?;
                }
            }
        }
        Ok(())
    }

    /// `cells`: for each pattern `[rows, columns]` (or a number of squares in a row), drawn as
    /// squares side by side with a caption `Pattern 1`, `Pattern 2`… (`captions`: false to hide).
    fn pattern(&self, s: &mut Session) -> Result<()> {
        let shapes: Vec<(usize, usize)> = self
            .d
            .get("cells")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|c| match c.as_array() {
                        Some(rc) => {
                            (rc.first().and_then(Value::as_u64).unwrap_or(1) as usize, rc.get(1).and_then(Value::as_u64).unwrap_or(1) as usize)
                        }
                        None => (1, c.as_u64().unwrap_or(1) as usize),
                    })
                    .collect()
            })
            .unwrap_or_default();
        if shapes.is_empty() {
            return Ok(());
        }
        let captions = self.d.get("captions").and_then(Value::as_bool).unwrap_or(true);
        let [x, y, w, h] = self.fit(2.2, 6.0);
        let widest: usize = shapes.iter().map(|c| c.1).sum::<usize>().max(1) + shapes.len().saturating_sub(1);
        let tallest = shapes.iter().map(|c| c.0).max().unwrap_or(1);
        let cap_h = if captions { 20.0 * k() } else { 0.0 };
        let unit = (w / widest as f64).min((h - cap_h) / tallest as f64).min(34.0 * k());
        let total_w = unit * widest as f64;
        let mut left = x + (w - total_w) / 2.0;
        let base = y + h - cap_h;
        for (i, (rows, cols)) in shapes.iter().enumerate() {
            for r in 0..*rows {
                for c in 0..*cols {
                    draw_shape(s, "rect", [left + unit * c as f64, base - unit * (r as f64 + 1.0), unit, unit], Some("#BFD4F6"), Some((INK, 1.8)))?;
                }
            }
            if captions {
                label_sized(s, &format!("Pattern {}", i + 1), [left + unit * *cols as f64 / 2.0, base + 11.0 * k()], 11.0)?;
            }
            left += unit * (*cols as f64 + 1.0);
        }
        Ok(())
    }

    /// `bars`: each bar a list of parts `{"label": text|null, "size": number}` drawn to scale as
    /// side-by-side blocks; `total`: a label (or `null` for `?`) for a bracket over the first bar.
    fn bar_model(&self, s: &mut Session) -> Result<()> {
        let bars: Vec<Vec<(Option<String>, f64)>> = self
            .d
            .get("bars")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .map(|row| {
                        row.as_array()
                            .map(|parts| {
                                parts
                                    .iter()
                                    .map(|p| {
                                        (
                                            p.get("label").and_then(|l| l.as_str().map(str::to_string).or_else(|| l.as_f64().map(num_text))),
                                            p.get("size").and_then(Value::as_f64).unwrap_or(1.0).max(0.1),
                                        )
                                    })
                                    .collect()
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        if bars.is_empty() {
            return Ok(());
        }
        let has_total = self.d.get("total").is_some();
        let [x, y, w, h] = self.fit(2.6, 8.0);
        let widest = bars.iter().map(|b| b.iter().map(|p| p.1).sum::<f64>()).fold(1.0, f64::max);
        let top = if has_total { 26.0 * k() } else { 0.0 };
        let row_h = ((h - top) / bars.len() as f64).min(52.0 * k());
        for (ri, bar) in bars.iter().enumerate() {
            let mut left = x;
            let ty = y + top + row_h * ri as f64;
            for (label, size) in bar {
                let bw = w * size / widest;
                draw_shape(s, "rect", [left, ty, bw, row_h * 0.8], Some("#FDE9B8"), Some((INK, 2.2)))?;
                label_sized(s, &label.clone().unwrap_or_else(|| "?".into()), [left + bw / 2.0, ty + row_h * 0.4], 15.0)?;
                left += bw;
            }
            if ri == 0 && has_total {
                let end = x + w * bar.iter().map(|p| p.1).sum::<f64>() / widest;
                let by = ty - 8.0;
                draw_line(s, [x, by], [end, by], INK, 2.0)?;
                draw_line(s, [x, by], [x, by + 6.0], INK, 2.0)?;
                draw_line(s, [end, by], [end, by + 6.0], INK, 2.0)?;
                label_sized(s, &self.lab_or("total", "?"), [(x + end) / 2.0, by - 12.0 * k()], 15.0)?;
            }
        }
        Ok(())
    }

    /// `bars`: a list of `{parts, shaded, label?}` drawn one under another, every bar the same
    /// length so fractions can be compared; or `parts` and `shaded` for a single bar.
    fn fraction_bar(&self, s: &mut Session) -> Result<()> {
        let bars: Vec<(usize, usize, Option<String>)> = match self.d.get("bars").and_then(Value::as_array) {
            Some(list) => list
                .iter()
                .map(|b| {
                    (
                        b.get("parts").and_then(Value::as_u64).unwrap_or(1).clamp(1, 60) as usize,
                        b.get("shaded").and_then(Value::as_u64).unwrap_or(0) as usize,
                        b.get("label").and_then(Value::as_str).map(str::to_string),
                    )
                })
                .collect(),
            None => vec![(
                self.num("parts", 1.0).clamp(1.0, 60.0) as usize,
                self.num("shaded", 0.0).max(0.0) as usize,
                self.d.get("label").and_then(Value::as_str).map(str::to_string),
            )],
        };
        let [x, y, w, h] = self.fit(3.2 * bars.len().max(1) as f64 / 1.2, 8.0);
        let row_h = (h / bars.len().max(1) as f64).min(46.0 * k());
        let left = x + 24.0 * k();
        let bw = w - 28.0 * k();
        for (r, (parts, shaded, label)) in bars.iter().enumerate() {
            let top = y + row_h * r as f64 + (h - row_h * bars.len() as f64) / 2.0;
            let cw = bw / *parts as f64;
            for i in 0..*parts {
                draw_shape(
                    s,
                    "rect",
                    [left + cw * i as f64, top, cw, row_h * 0.78],
                    Some(if i < *shaded { "#F4B183" } else { "#FFFFFF" }),
                    Some((INK, 2.0)),
                )?;
            }
            if let Some(l) = label {
                label_sized(s, l, [x + 10.0 * k(), top + row_h * 0.39], 14.0)?;
            }
        }
        Ok(())
    }

    /// `rows` × `cols` squares (default 10 × 10) with the first `shaded` of them coloured, row by row.
    fn shade_grid(&self, s: &mut Session) -> Result<()> {
        let rows = self.num("rows", 10.0).clamp(1.0, 20.0) as usize;
        let cols = self.num("cols", 10.0).clamp(1.0, 20.0) as usize;
        let shaded = self.num("shaded", 0.0).max(0.0) as usize;
        let [x, y, w, h] = self.fit(cols as f64 / rows as f64, 8.0);
        let (cw, ch) = (w / cols as f64, h / rows as f64);
        for r in 0..rows {
            for c in 0..cols {
                let on = r * cols + c < shaded;
                draw_shape(
                    s,
                    "rect",
                    [x + cw * c as f64, y + ch * r as f64, cw, ch],
                    Some(if on { "#F4B183" } else { "#FFFFFF" }),
                    Some((INK, 1.2)),
                )?;
            }
        }
        draw_shape(s, "rect", [x, y, w, h], None, Some((INK, 2.5)))?;
        Ok(())
    }

    /// `headers` and `rows` (each cell a text or number; `null` is `?`); an optional `title`
    /// above. Also used for place-value charts and frequency tables.
    fn table(&self, s: &mut Session) -> Result<()> {
        let headers: Vec<String> = self.list("headers").into_iter().map(|c| c.unwrap_or_else(|| "?".into())).collect();
        let rows: Vec<Vec<String>> = self
            .d
            .get("rows")
            .and_then(Value::as_array)
            .map(|rs| {
                rs.iter()
                    .map(|r| {
                        r.as_array()
                            .map(|cs| {
                                cs.iter()
                                    .map(|c| c.as_str().map(str::to_string).or_else(|| c.as_f64().map(num_text)).unwrap_or_else(|| "?".into()))
                                    .collect()
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let cols = headers.len().max(rows.iter().map(Vec::len).max().unwrap_or(0)).max(1);
        let nrows = rows.len() + usize::from(!headers.is_empty());
        let [x, y, w, h] = self.fit((cols as f64 * 0.7 + 0.8).max(1.6), 6.0);
        let (cw, rh) = (w / cols as f64, (h / nrows.max(1) as f64).min(44.0 * k()));
        let top = y + (h - rh * nrows as f64) / 2.0;
        let mut row_i = 0usize;
        let mut draw_row = |s: &mut Session, cells: &[String], fill: &str, bold: bool| -> Result<()> {
            for c in 0..cols {
                let ry = top + rh * row_i as f64;
                draw_shape(s, "rect", [x + cw * c as f64, ry, cw, rh], Some(fill), Some((INK, 1.8)))?;
                if let Some(t) = cells.get(c) {
                    let t = if bold { format!("**{t}**") } else { t.clone() };
                    label_sized(s, &t.replace("**", ""), [x + cw * (c as f64 + 0.5), ry + rh / 2.0], 15.0)?;
                }
            }
            row_i += 1;
            Ok(())
        };
        if !headers.is_empty() {
            draw_row(s, &headers, "#D7E4FA", true)?;
        }
        for r in &rows {
            draw_row(s, r, "#FFFFFF", false)?;
        }
        Ok(())
    }

    /// `xmin`, `xmax`, `ymin`, `ymax`; `line`: `{m, c}` for y = mx + c; `points`: `[x, y, name]`.
    fn grid(&self, s: &mut Session) -> Result<()> {
        let (x0, x1, y0, y1) = (self.num("xmin", -5.0), self.num("xmax", 5.0), self.num("ymin", -5.0), self.num("ymax", 5.0));
        let aspect = (x1 - x0) / (y1 - y0).max(1.0);
        let [x, y, w, h] = self.fit(aspect, 22.0);
        let px = |v: f64| x + w * (v - x0) / (x1 - x0);
        let py = |v: f64| y + h - h * (v - y0) / (y1 - y0);
        let (mut gx, mut gy) = (x0.ceil(), y0.ceil());
        while gx <= x1 {
            thin(s, [px(gx), y], [px(gx), y + h], LIGHT)?;
            gx += 1.0;
        }
        while gy <= y1 {
            thin(s, [x, py(gy)], [x + w, py(gy)], LIGHT)?;
            gy += 1.0;
        }
        let (ax, ay) = (px(0.0f64.clamp(x0, x1)), py(0.0f64.clamp(y0, y1)));
        draw_line(s, [x, ay], [x + w, ay], INK, 1.8)?;
        draw_line(s, [ax, y], [ax, y + h], INK, 1.8)?;
        let step = if x1 - x0 > 12.0 { 2.0 } else { 1.0 };
        let mut v = (x0 / step).ceil() * step;
        while v <= x1 {
            if v.abs() > 1e-9 {
                label_sized(s, &format!("{v}"), [px(v), y + h + 11.0], 11.0)?;
            }
            v += step;
        }
        let mut v = (y0 / step).ceil() * step;
        while v <= y1 {
            if v.abs() > 1e-9 {
                label_sized(s, &format!("{v}"), [x - 12.0, py(v)], 11.0)?;
            }
            v += step;
        }
        if let Some(l) = self.d.get("line") {
            let (m, c) = (l.get("m").and_then(Value::as_f64).unwrap_or(1.0), l.get("c").and_then(Value::as_f64).unwrap_or(0.0));
            // Clip y = mx + c to the grid.
            let mut ends: Vec<P> = vec![];
            for xv in [x0, x1] {
                let yv = m * xv + c;
                if (y0..=y1).contains(&yv) {
                    ends.push([xv, yv]);
                }
            }
            if m.abs() > 1e-9 {
                for yv in [y0, y1] {
                    let xv = (yv - c) / m;
                    if (x0..=x1).contains(&xv) && !ends.iter().any(|e| (e[0] - xv).abs() < 1e-6) {
                        ends.push([xv, yv]);
                    }
                }
            }
            if let [a, b, ..] = ends[..] {
                draw_line(s, [px(a[0]), py(a[1])], [px(b[0]), py(b[1])], "#D0453A", 3.0)?;
            }
        }
        for p in self.d.get("points").and_then(Value::as_array).into_iter().flatten() {
            let (Some(xv), Some(yv)) = (p.get(0).and_then(Value::as_f64), p.get(1).and_then(Value::as_f64)) else { continue };
            draw_shape(s, "ellipse", [px(xv) - 4.5, py(yv) - 4.5, 9.0, 9.0], Some("#D0453A"), None)?;
            if let Some(n) = p.get(2).and_then(Value::as_str) {
                label(s, &format!("${n}$"), [px(xv) + 14.0, py(yv) - 12.0])?;
            }
        }
        Ok(())
    }

    // ── sets and probability ─────────────────────────────────────────────────────────────────

    /// `a`, `b`: set names; `onlyA`, `both`, `onlyB`, `outside`: the counts (or `null`).
    fn venn(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.5, 14.0);
        draw_shape(s, "rect", [x, y, w, h], None, Some((INK, 2.0)))?;
        let r = h * 0.36;
        let (c1, c2) = ([x + w * 0.38, y + h * 0.5], [x + w * 0.62, y + h * 0.5]);
        draw_shape(s, "ellipse", [c1[0] - r, c1[1] - r, 2.0 * r, 2.0 * r], None, Some(("#D0453A", 2.5)))?;
        draw_shape(s, "ellipse", [c2[0] - r, c2[1] - r, 2.0 * r, 2.0 * r], None, Some((BLUE, 2.5)))?;
        label(s, &format!("${}$", self.text("a").unwrap_or_else(|| "A".into())), [c1[0] - r * 0.7, c1[1] - r - 12.0])?;
        label(s, &format!("${}$", self.text("b").unwrap_or_else(|| "B".into())), [c2[0] + r * 0.7, c2[1] - r - 12.0])?;
        for (key, at) in [
            ("onlyA", [c1[0] - r * 0.5, c1[1]]),
            ("both", [(c1[0] + c2[0]) / 2.0, c1[1]]),
            ("onlyB", [c2[0] + r * 0.5, c2[1]]),
            ("outside", [x + w - 22.0, y + h - 18.0]),
        ] {
            if let Some(t) = self.lab(key) {
                label(s, &t, at)?;
            }
        }
        Ok(())
    }

    /// `first`: `[[outcome, probability]]`; `second`: for each first outcome, its own list.
    /// A probability that is `null` is the unknown.
    fn tree(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.6, 8.0);
        let first = self.d.get("first").and_then(Value::as_array).cloned().unwrap_or_default();
        let second = self.d.get("second").and_then(Value::as_array).cloned().unwrap_or_default();
        let root = [x + 4.0, y + h / 2.0];
        let c1 = x + w * 0.3;
        let size = 12.0;
        let width = |t: &str| (crate::latex::plain(t).chars().count() as f64 * size * 0.6 + 8.0) * k();
        let pair = |v: &Value| -> (String, String) {
            let name = v.get(0).and_then(Value::as_str).unwrap_or("").to_string();
            let prob = v.get(1).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| self.unknown());
            (name, prob)
        };
        let n1 = first.len().max(1) as f64;
        let longest = second.iter().filter_map(Value::as_array).flatten().map(|kid| width(&pair(kid).0)).fold(30.0, f64::max);
        let c2 = x + w - longest - 4.0;
        for (i, f) in first.iter().enumerate() {
            let fy = y + h * (i as f64 + 0.5) / n1;
            let (name, prob) = pair(f);
            let node = [c1, fy];
            ln(s, root, node)?;
            let side = if fy < root[1] { -1.0 } else { 1.0 };
            label_sized(s, &prob, pt(mid(root, node), -10.0 * k(), 14.0 * k() * side), size)?;
            label_sized(s, &name, [c1 + 4.0 * k() + width(&name) / 2.0, fy], size)?;
            let start = [c1 + width(&name) + 8.0, fy];
            let kids = second.get(i).and_then(Value::as_array).cloned().unwrap_or_default();
            let n2 = kids.len().max(1) as f64;
            let band = h / n1;
            for (j, kid) in kids.iter().enumerate() {
                let ky = y + h * i as f64 / n1 + band * (j as f64 + 0.5) / n2;
                let end = [c2, ky];
                ln(s, start, end)?;
                let (kname, kprob) = pair(kid);
                let up = if ky < fy { -1.0 } else { 1.0 };
                label_sized(s, &kprob, pt(mid(start, end), 0.0, 12.0 * k() * up), size)?;
                label_sized(s, &kname, [c2 + 4.0 * k() + width(&kname) / 2.0, ky], size)?;
            }
        }
        Ok(())
    }
}
