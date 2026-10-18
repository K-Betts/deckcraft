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
        "pie" => f.pie(s),
        "regular-polygon" => f.polygon(s),
        "similar-triangles" => f.similar(s),
        "function-machine" => f.function_machine(s),
        "sequence-arrows" => f.sequence_arrows(s),
        "pattern" => f.pattern(s),
        "bar-model" => f.bar_model(s),
        "table" => f.table(s),
        "fraction-bar" => f.fraction_bar(s),
        "pictogram" => f.pictogram(s),
        "scatter" => f.plot(s, false),
        "line-graph" => f.plot(s, true),
        "polygon-shape" => f.polygon_shape(s),
        "polygon-angles" => f.polygon_angles(s),
        "ruler" => f.ruler(s),
        "factor-tree" => f.factor_tree(s),
        "counters" => f.counters(s),
        "crossing-lines" => f.crossing_lines(s),
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
                | "pie"
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
    let (w, h) = ((chars * size * 0.66 + 26.0).max(34.0), size * 1.6);
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
        let longest = self.list("angles").iter().flatten().map(|t| crate::latex::plain(t).chars().count()).max().unwrap_or(3) as f64;
        let [x, y, w, h] = self.fit(1.5, 28.0 + longest * 5.0);
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
        label(s, &self.unknown(), pt(b, 42.0, -12.0))
    }

    /// `base`, `height`, `hyp`: side labels; `angle`: the angle at the right-hand end of the base.
    fn right_triangle(&self, s: &mut Session) -> Result<()> {
        // `baseLength` and `heightLength` (numbers) draw the triangle to the right proportions.
        let (bl, hl) = (self.num("baseLength", 0.0), self.num("heightLength", 0.0));
        let aspect = if bl > 0.0 && hl > 0.0 { (bl / hl).clamp(0.45, 2.4) } else { 1.35 };
        let [x, y, w, h] = self.fit(aspect, 26.0);
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
            ax + aw * (0.16 + 0.34 * t)
        };
        ln(s, [ax + aw * 0.16, ay + ah * 0.04], [ax + aw * 0.5, ay + ah * 0.96])?;
        // Longer labels are moved right, clear of the transversal.
        let known = self.text("knownAngle").unwrap_or_default();
        let unknown = self.text("unknown").unwrap_or_else(|| "x".into());
        let reach = |t: &str| 22.0 + crate::latex::plain(t).chars().count() as f64 * 4.5;
        let right = |y: f64, t: &str| (on_t(y) + reach(t)).min(ax + aw - reach(t) * 0.6);
        label(s, &known, [right(y1, &known), y1 + 22.0])?;
        label(s, &format!("${unknown}$"), [right(y2, &unknown), y2 - 22.0])
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

    /// `values`: the sizes of the slices, clockwise from the top (drawn to scale); `labels`: the
    /// text beside each slice.
    fn pie(&self, s: &mut Session) -> Result<()> {
        let values: Vec<f64> =
            self.d.get("values").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
        let total: f64 = values.iter().sum();
        if values.len() < 2 || total <= 0.0 {
            return Ok(());
        }
        let labels = self.list("labels");
        let [x, y, w, h] = self.fit(1.45, 12.0);
        let r = h.min(w * 0.55) / 2.0 * 0.82;
        let c = [x + w / 2.0, y + h / 2.0];
        draw_shape(s, "ellipse", [c[0] - r, c[1] - r, 2.0 * r, 2.0 * r], None, Some((INK, 2.5)))?;
        let at = |t: f64, d: f64| [c[0] + d * t.sin(), c[1] - d * t.cos()];
        let mut start = 0.0;
        for (i, v) in values.iter().enumerate() {
            let t0 = std::f64::consts::TAU * start / total;
            draw_line(s, c, at(t0, r), INK, 2.0)?;
            let mid = std::f64::consts::TAU * (start + v / 2.0) / total;
            if let Some(Some(t)) = labels.get(i) {
                let out = at(mid, r + 8.0 + crate::latex::plain(t).chars().count() as f64 * 4.0);
                label_sized(s, t, out, 12.0)?;
            }
            start += v;
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
        if self.d.get("series").is_some() {
            return self.series_bar_chart(s);
        }
        let [x, y, w, h] = self.fit(1.5, 14.0);
        let cats = self.list("categories");
        let vals: Vec<f64> = self.d.get("values").and_then(Value::as_array).map(|v| v.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
        let n = cats.len().min(vals.len());
        if n == 0 {
            return Ok(());
        }
        // `yMin` starts the axis above zero (to show a misleading graph); `lines` draws a vertical
        // line chart instead of bars.
        let lo = self.num("yMin", 0.0);
        let lines = self.d.get("lines").and_then(Value::as_bool).unwrap_or(false);
        let top = vals.iter().cloned().fold(1.0, f64::max);
        let span = top - lo;
        let step = if span <= 10.0 {
            if span <= 5.0 { 1.0 } else { 2.0 }
        } else if span <= 30.0 {
            5.0
        } else if span <= 60.0 {
            10.0
        } else {
            20.0
        };
        let max = lo + (span / step).ceil() * step;
        let (left, bottom) = (x + 34.0, y + h - 22.0);
        let (pw, ph) = (w - 40.0, h - 30.0);
        let mut v = lo;
        while v <= max + 1e-9 {
            let yy = bottom - ph * (v - lo) / (max - lo);
            thin(s, [left, yy], [left + pw, yy], LIGHT)?;
            label_sized(s, &num_text(v), [left - 14.0, yy], 11.0)?;
            v += step;
        }
        ln(s, [left, bottom - ph], [left, bottom])?;
        ln(s, [left, bottom], [left + pw, bottom])?;
        let slot = pw / n as f64;
        for (i, val) in vals.iter().take(n).enumerate() {
            let bh = ph * (val - lo) / (max - lo);
            if lines {
                draw_line(s, [left + slot * (i as f64 + 0.5), bottom], [left + slot * (i as f64 + 0.5), bottom - bh], BLUE, 6.0)?;
            } else {
                draw_shape(s, "rect", [left + slot * i as f64 + slot * 0.2, bottom - bh, slot * 0.6, bh], Some(BLUE), Some((INK, 1.5)))?;
            }
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
                // `end`: the line runs to this value (a two-sided inequality).
                _ => {
                    if let Some(e) = m.get("end").and_then(Value::as_f64) {
                        draw_line(s, [at(v), yy], [at(e), yy], BLUE, 5.0)?;
                    }
                }
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

    /// `categories`, `series`: `[{"name": "Boys", "values": [..]}, ..]`; `stacked` true gives a
    /// composite bar chart (series on top of each other), false a dual one (side by side).
    fn series_bar_chart(&self, s: &mut Session) -> Result<()> {
        let [x, y, w, h] = self.fit(1.5, 12.0);
        let cats = self.list("categories");
        let series: Vec<(String, Vec<f64>)> = self
            .d
            .get("series")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|v| {
                        (
                            v.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                            v.get("values").and_then(Value::as_array).map(|x| x.iter().filter_map(Value::as_f64).collect()).unwrap_or_default(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let stacked = self.d.get("stacked").and_then(Value::as_bool).unwrap_or(false);
        let n = cats.len();
        if n == 0 || series.is_empty() {
            return Ok(());
        }
        let col = |i: usize| ["#4A7BD0", "#F4B183", "#6FAE7B", "#B58CD9"][i % 4];
        let total = |c: usize| -> f64 {
            if stacked {
                series.iter().map(|sr| sr.1.get(c).copied().unwrap_or(0.0)).sum()
            } else {
                series.iter().map(|sr| sr.1.get(c).copied().unwrap_or(0.0)).fold(0.0, f64::max)
            }
        };
        let top = (0..n).map(total).fold(1.0, f64::max);
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
        let legend = 18.0 * k();
        let (left, bottom) = (x + 34.0, y + h - 22.0);
        let (pw, ph) = (w - 40.0, h - 30.0 - legend);
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
        for c in 0..n {
            let groups = if stacked { 1.0 } else { series.len() as f64 };
            let bw = slot * 0.7 / groups;
            let mut base = 0.0;
            for (i, sr) in series.iter().enumerate() {
                let val = sr.1.get(c).copied().unwrap_or(0.0);
                let bh = ph * val / max;
                let bx = if stacked { left + slot * c as f64 + slot * 0.15 } else { left + slot * c as f64 + slot * 0.15 + bw * i as f64 };
                let by = if stacked { bottom - ph * base / max - bh } else { bottom - bh };
                draw_shape(s, "rect", [bx, by, bw, bh], Some(col(i)), Some((INK, 1.5)))?;
                if stacked {
                    base += val;
                }
            }
            if let Some(Some(cn)) = cats.get(c) {
                label_sized(s, cn, [left + slot * (c as f64 + 0.5), bottom + 12.0], 10.0)?;
            }
        }
        let mut lx = left;
        for (i, (name, _)) in series.iter().enumerate() {
            draw_shape(s, "rect", [lx, y, 12.0 * k(), 12.0 * k()], Some(col(i)), Some((INK, 1.2)))?;
            label_sized(s, name, [lx + 22.0 * k() + name.chars().count() as f64 * 3.0 * k(), y + 6.0 * k()], 11.0)?;
            lx += 40.0 * k() + name.chars().count() as f64 * 6.5 * k();
        }
        Ok(())
    }

    /// `rows`: `[{"label": "Mon", "count": 3.5}]`, `key`: how many each symbol stands for
    /// (default 1); half symbols for halves. `keyLabel` text beside the key symbol.
    fn pictogram(&self, s: &mut Session) -> Result<()> {
        let rows: Vec<(String, f64)> = self
            .d
            .get("rows")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|r| {
                        (r.get("label").and_then(Value::as_str).unwrap_or("").to_string(), r.get("count").and_then(Value::as_f64).unwrap_or(0.0))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if rows.is_empty() {
            return Ok(());
        }
        let key = self.num("key", 1.0).max(0.01);
        let widest = rows.iter().map(|r| (r.1 / key).ceil()).fold(1.0, f64::max);
        let [x, y, w, h] = self.fit(1.7, 6.0);
        let label_w = 44.0 * k();
        let rh = ((h - 24.0 * k()) / rows.len() as f64).min(34.0 * k());
        let unit = ((w - label_w) / widest.max(4.0)).min(rh * 0.9);
        for (i, (label, count)) in rows.iter().enumerate() {
            let ty = y + rh * i as f64;
            label_sized(s, label, [x + label_w / 2.0, ty + rh / 2.0], 12.0)?;
            let symbols = count / key;
            let full = symbols.floor() as usize;
            for j in 0..full {
                draw_shape(s, "ellipse", [x + label_w + unit * j as f64, ty + rh * 0.1, unit * 0.8, rh * 0.8], Some("#F4B183"), Some((INK, 1.5)))?;
            }
            if symbols - symbols.floor() > 0.01 {
                draw_shape(s, "rect", [x + label_w + unit * full as f64, ty + rh * 0.1, unit * 0.4, rh * 0.8], Some("#F4B183"), Some((INK, 1.5)))?;
            }
        }
        let ky = y + rh * rows.len() as f64 + 10.0 * k();
        draw_shape(s, "ellipse", [x + label_w, ky, unit * 0.6, rh * 0.6], Some("#F4B183"), Some((INK, 1.5)))?;
        let kl = self.text("keyLabel").unwrap_or_else(|| format!("= {}", num_text(key)));
        label_sized(s, &kl, [x + label_w + unit * 0.9 + kl.chars().count() as f64 * 3.5, ky + rh * 0.3], 12.0)?;
        Ok(())
    }

    /// Axes `xmin`..`xmax`, `ymin`..`ymax` with `xstep`/`ystep` gridlines, `xlabel`/`ylabel` and
    /// `points`: `[[x, y], ..]`. With `join` the points are joined in order (a line graph, a
    /// distance-time graph); `line`: `[[x1, y1], [x2, y2]]` adds a straight line (line of best fit).
    fn plot(&self, s: &mut Session, join: bool) -> Result<()> {
        let (x0, x1, y0, y1) = (self.num("xmin", 0.0), self.num("xmax", 10.0), self.num("ymin", 0.0), self.num("ymax", 10.0));
        let (xs, ys) = (self.num("xstep", ((x1 - x0) / 5.0).max(1.0)), self.num("ystep", ((y1 - y0) / 5.0).max(1.0)));
        let [x, y, w, h] = self.fit(1.45, 4.0);
        let ytitle = if self.text("ylabel").is_some() { 14.0 * k() } else { 0.0 };
        let (left, right, top, bottom) = (x + 40.0 * k(), x + w - 8.0, y + 12.0 * k() + ytitle, y + h - 32.0 * k());
        let px = |v: f64| left + (right - left) * (v - x0) / (x1 - x0).max(1e-9);
        let py = |v: f64| bottom - (bottom - top) * (v - y0) / (y1 - y0).max(1e-9);
        let mut gx = x0;
        while gx <= x1 + 1e-9 {
            thin(s, [px(gx), top], [px(gx), bottom], LIGHT)?;
            label_sized(s, &num_text(gx), [px(gx), bottom + 11.0 * k()], 11.0)?;
            gx += xs;
        }
        let mut gy = y0;
        while gy <= y1 + 1e-9 {
            thin(s, [left, py(gy)], [right, py(gy)], LIGHT)?;
            label_sized(s, &num_text(gy), [left - 16.0 * k(), py(gy)], 11.0)?;
            gy += ys;
        }
        draw_line(s, [left, top], [left, bottom], INK, 2.0)?;
        draw_line(s, [left, bottom], [right, bottom], INK, 2.0)?;
        if let Some(t) = self.text("xlabel") {
            label_sized(s, &t, [(left + right) / 2.0, y + h - 7.0 * k()], 12.0)?;
        }
        if let Some(t) = self.text("ylabel") {
            label_sized(s, &t, [left + 20.0 * k() + t.chars().count() as f64 * 3.0 * k(), top - ytitle - 2.0 * k() + 8.0], 12.0)?;
        }
        let raw: Vec<&Value> = self.d.get("points").and_then(Value::as_array).map(|a| a.iter().collect()).unwrap_or_default();
        let pts: Vec<P> = raw.iter().filter_map(|p| Some([p.get(0)?.as_f64()?, p.get(1)?.as_f64()?])).collect();
        if let Some(l) = self.d.get("line").and_then(Value::as_array) {
            let g = |i: usize, j: usize| l.get(i).and_then(|p| p.get(j)).and_then(Value::as_f64);
            if let (Some(a), Some(b), Some(c), Some(d)) = (g(0, 0), g(0, 1), g(1, 0), g(1, 1)) {
                draw_line(s, [px(a), py(b)], [px(c), py(d)], "#D0453A", 2.5 * k().sqrt())?;
            }
        }
        if join {
            for pair in pts.windows(2) {
                draw_line(s, [px(pair[0][0]), py(pair[0][1])], [px(pair[1][0]), py(pair[1][1])], BLUE, 2.5 * k().sqrt())?;
            }
        }
        for p in &pts {
            let r = if join { 3.5 } else { 4.5 } * k().sqrt();
            draw_shape(s, "ellipse", [px(p[0]) - r, py(p[1]) - r, 2.0 * r, 2.0 * r], Some(if join { BLUE } else { "#2F4F9F" }), None)?;
        }
        // A third item in a point is its name, written beside it.
        for p in &raw {
            if let (Some(xv), Some(yv), Some(name)) =
                (p.get(0).and_then(Value::as_f64), p.get(1).and_then(Value::as_f64), p.get(2).and_then(Value::as_str))
            {
                label(s, &format!("${name}$"), [px(xv) + 12.0 * k(), py(yv) - 12.0 * k()])?;
            }
        }
        Ok(())
    }

    /// `shape`: square, rectangle, parallelogram, rhombus, trapezium, kite, equilateral,
    /// isosceles, scalene, right-angled; drawn with equal-side and right-angle marks.
    fn polygon_shape(&self, s: &mut Session) -> Result<()> {
        let name = self.text("shape").unwrap_or_default();
        let [x, y, w, h] = self.fit(1.35, 16.0);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let r = |fx: f64, fy: f64| [cx + fx * w / 2.0, cy + fy * h / 2.0];
        let pts: Vec<P> = match name.as_str() {
            "square" => {
                let a = h.min(w) * 0.5;
                vec![[cx - a, cy - a], [cx + a, cy - a], [cx + a, cy + a], [cx - a, cy + a]]
            }
            "rectangle" => vec![r(-0.9, -0.55), r(0.9, -0.55), r(0.9, 0.55), r(-0.9, 0.55)],
            "parallelogram" => vec![r(-0.55, -0.6), r(0.95, -0.6), r(0.55, 0.6), r(-0.95, 0.6)],
            "rhombus" => vec![r(0.0, -0.9), r(0.75, 0.0), r(0.0, 0.9), r(-0.75, 0.0)],
            "trapezium" => vec![r(-0.5, -0.6), r(0.5, -0.6), r(0.95, 0.6), r(-0.95, 0.6)],
            "kite" => vec![r(0.0, -0.95), r(0.6, -0.25), r(0.0, 0.95), r(-0.6, -0.25)],
            "equilateral" => vec![r(0.0, -0.85), r(0.95, 0.75), r(-0.95, 0.75)],
            "isosceles" => vec![r(0.0, -0.9), r(0.7, 0.75), r(-0.7, 0.75)],
            "scalene" => vec![r(-0.3, -0.85), r(0.95, 0.75), r(-0.95, 0.5)],
            "right-angled" => vec![r(-0.8, -0.8), r(-0.8, 0.75), r(0.9, 0.75)],
            _ => return Ok(()),
        };
        poly(s, &pts)?;
        let tick = |s: &mut Session, a: P, b: P| -> Result<()> {
            let m = mid(a, b);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let n = (dx * dx + dy * dy).sqrt().max(1.0);
            let (ux, uy) = (-dy / n * 6.0, dx / n * 6.0);
            draw_line(s, [m[0] - ux, m[1] - uy], [m[0] + ux, m[1] + uy], INK, 2.0)?;
            Ok(())
        };
        let equal: Vec<(usize, usize)> = match name.as_str() {
            "square" | "rhombus" => vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            "equilateral" => vec![(0, 1), (1, 2), (2, 0)],
            "isosceles" => vec![(0, 1), (2, 0)],
            "kite" => vec![(0, 1), (3, 0)],
            _ => vec![],
        };
        for (i, j) in equal {
            tick(s, pts[i], pts[j])?;
        }
        if matches!(name.as_str(), "square" | "rectangle") {
            for i in 0..4 {
                right_angle(s, pts[i], pts[(i + 1) % 4], pts[(i + 3) % 4])?;
            }
        }
        if name == "right-angled" {
            right_angle(s, pts[1], pts[0], pts[2])?;
        }
        Ok(())
    }

    /// `angles`: a label for each interior angle (`null` the unknown), `sizes`: the true sizes in
    /// degrees (3 to 6 of them, adding up to (n - 2) × 180): the polygon is drawn with those angles.
    fn polygon_angles(&self, s: &mut Session) -> Result<()> {
        let sizes = self.sizes().unwrap_or_default();
        let n = sizes.len();
        if !(3..=6).contains(&n) {
            return Ok(());
        }
        let [x, y, w, h] = self.fit(1.35, 34.0);
        // Walk the edges turning by each exterior angle; try some side lengths until it closes.
        let mut best: Option<Vec<P>> = None;
        let lens: Vec<f64> = (0..n).map(|i| 1.0 + 0.1 * (i as f64)).collect();
        'search: for a in 0..9 {
            for b in 0..9 {
                let mut l = lens.clone();
                l[0] = 0.6 + a as f64 * 0.25;
                l[1] = 0.6 + b as f64 * 0.25;
                let (mut ang, mut p) = (0.0f64, [0.0, 0.0]);
                let mut pts = vec![p];
                let mut ok = true;
                // All but the last two sides are chosen; the last two follow from closure.
                for i in 0..n.saturating_sub(2) {
                    p = [p[0] + l[i] * ang.cos(), p[1] + l[i] * ang.sin()];
                    pts.push(p);
                    ang += (180.0 - sizes[(i + 1) % n]).to_radians();
                }
                // Directions of the last two sides: solve p + l1*d1 + l2*d2 = start.
                let d1 = ang;
                let d2 = ang + (180.0 - sizes[(n - 1) % n]).to_radians();
                let det = d1.cos() * d2.sin() - d1.sin() * d2.cos();
                if det.abs() < 1e-6 {
                    continue;
                }
                let (rx, ry) = (-p[0], -p[1]);
                let l1 = (rx * d2.sin() - ry * d2.cos()) / det;
                let l2 = (d1.cos() * ry - d1.sin() * rx) / det;
                if l1 < 0.25 || l2 < 0.25 {
                    ok = false;
                }
                if ok {
                    let q = [p[0] + l1 * d1.cos(), p[1] + l1 * d1.sin()];
                    pts.push(q);
                    best = Some(pts);
                    break 'search;
                }
            }
        }
        let pts = match best {
            Some(p) => p,
            None => return Ok(()),
        };
        let (minx, maxx) = pts.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
        let (miny, maxy) = pts.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
        let scale = (w / (maxx - minx).max(1e-6)).min(h / (maxy - miny).max(1e-6));
        let off = [x + (w - (maxx - minx) * scale) / 2.0, y + (h - (maxy - miny) * scale) / 2.0];
        let pts: Vec<P> = pts.iter().map(|p| [off[0] + (p[0] - minx) * scale, off[1] + (p[1] - miny) * scale]).collect();
        poly(s, &pts)?;
        let unknown = self.unknown();
        let labels: Vec<String> = self.list("angles").into_iter().map(|v| v.unwrap_or_else(|| unknown.clone())).collect();
        let centre = [pts.iter().map(|p| p[0]).sum::<f64>() / n as f64, pts.iter().map(|p| p[1]).sum::<f64>() / n as f64];
        // The first angle sits at vertex 1 (after the first edge), and so on round the shape.
        for (i, t) in labels.iter().enumerate().take(n) {
            let v = pts[(i + 1) % n];
            let (dx, dy) = (v[0] - centre[0], v[1] - centre[1]);
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let chars = crate::latex::plain(t).chars().count() as f64;
            let out = (14.0 + chars * 4.5) * k();
            label(s, t, [v[0] + dx / d * out, v[1] + dy / d * out * 0.8])?;
        }
        Ok(())
    }

    /// `length`: the length of a line in cm (to one decimal), drawn from `from` (default 0) along
    /// a ruler marked in cm and mm; `unit` shows `cm` labels.
    fn ruler(&self, s: &mut Session) -> Result<()> {
        let len = self.num("length", 5.0);
        let from = self.num("from", 0.0);
        let cm = (from + len).ceil().max(len.ceil()) + 1.0;
        let [x, y, w, h] = self.fit(cm.max(4.0) * 0.45 + 1.0, 6.0);
        let (left, right) = (x + 8.0, x + w - 8.0);
        let at = |v: f64| left + (right - left) * v / cm;
        let top = y + h * 0.5;
        // The line being measured.
        draw_line(s, [at(from), y + h * 0.22], [at(from + len), y + h * 0.22], "#D0453A", 4.0 * k().sqrt())?;
        draw_line(s, [at(from), y + h * 0.12], [at(from), y + h * 0.32], "#D0453A", 2.0)?;
        draw_line(s, [at(from + len), y + h * 0.12], [at(from + len), y + h * 0.32], "#D0453A", 2.0)?;
        draw_shape(s, "rect", [left, top, right - left, h * 0.42], Some("#FDF3C8"), Some((INK, 1.8)))?;
        let mm = (cm * 10.0) as usize;
        for i in 0..=mm {
            let v = i as f64 / 10.0;
            let tall = if i % 10 == 0 {
                0.22
            } else if i % 5 == 0 {
                0.15
            } else {
                0.09
            };
            draw_line(s, [at(v), top], [at(v), top + h * tall], INK, 1.2)?;
            if i % 10 == 0 {
                label_sized(s, &format!("{}", i / 10), [at(v), top + h * 0.32], 11.0)?;
            }
        }
        Ok(())
    }

    /// `root`: `{"v": 60, "l": {...}, "r": {...}}`; `v` is a number or text, `null` is `?`; a node
    /// without children is a leaf (drawn circled, as a prime in a factor tree).
    fn factor_tree(&self, s: &mut Session) -> Result<()> {
        fn depth(n: &Value) -> usize {
            1 + ["l", "r"].iter().filter_map(|k| n.get(k)).map(depth).max().unwrap_or(0)
        }
        fn leaves(n: &Value) -> usize {
            let kids: Vec<&Value> = ["l", "r"].iter().filter_map(|k| n.get(k)).collect();
            if kids.is_empty() { 1 } else { kids.iter().map(|c| leaves(c)).sum() }
        }
        let Some(root) = self.d.get("root") else { return Ok(()) };
        let (d, l) = (depth(root), leaves(root));
        let [x, y, w, h] = self.fit((l as f64 / d as f64 * 1.15).max(1.4), 6.0);
        let (cw, rh) = (w / l as f64, h / d as f64);
        let r = (cw * 0.34).min(rh * 0.34).min(20.0 * k());
        #[allow(clippy::too_many_arguments)]
        fn place(s: &mut Session, n: &Value, lvl: usize, first: usize, x: f64, y: f64, cw: f64, rh: f64, r: f64) -> Result<(f64, f64)> {
            let kids: Vec<&Value> = ["l", "r"].iter().filter_map(|k| n.get(k)).collect();
            let span = leaves(n);
            let cx = x + cw * (first as f64 + span as f64 / 2.0);
            let cy = y + rh * (lvl as f64 + 0.5);
            let mut at = first;
            for c in &kids {
                let (kx, ky) = place(s, c, lvl + 1, at, x, y, cw, rh, r)?;
                draw_line(s, [cx, cy + r], [kx, ky - r], INK, 2.0)?;
                at += leaves(c);
            }
            let text = match n.get("v") {
                Some(Value::Null) | None => "?".to_string(),
                Some(Value::String(t)) => t.clone(),
                Some(v) => v.to_string(),
            };
            let leaf = kids.is_empty();
            draw_shape(s, "ellipse", [cx - r, cy - r, 2.0 * r, 2.0 * r], Some(if leaf { "#FDE9B8" } else { "#FFFFFF" }), Some((INK, 2.0)))?;
            label_sized(s, &text, [cx, cy], 14.0)?;
            Ok((cx, cy))
        }
        place(s, root, 0, 0, x, y, cw, rh, r)?;
        Ok(())
    }

    /// Two straight lines crossing: `theta` is the acute angle between them (default 60) and
    /// `labels` the four angles round the crossing, starting with the acute angle on the right
    /// and going anticlockwise (`null` the unknown).
    fn crossing_lines(&self, s: &mut Session) -> Result<()> {
        let theta = self.num("theta", 60.0).clamp(25.0, 155.0);
        let [x, y, w, h] = self.fit(1.5, 26.0);
        let (c, r) = ([x + w / 2.0, y + h / 2.0], (w.min(h * 1.5) / 2.0) * 0.95);
        let a0 = -theta / 2.0;
        let dir = |deg: f64, len: f64| [c[0] + len * deg.to_radians().cos(), c[1] - len * deg.to_radians().sin()];
        for base in [a0, a0 + theta] {
            ln(s, dir(base, r), dir(base + 180.0, r))?;
        }
        let unknown = self.unknown();
        let labels = self.list("labels");
        // Sectors, anticlockwise from the first line: θ, 180 − θ, θ, 180 − θ.
        let sectors = [(a0, theta), (a0 + theta, 180.0 - theta), (a0 + 180.0, theta), (a0 + 180.0 + theta, 180.0 - theta)];
        for (i, (start, size)) in sectors.iter().enumerate() {
            if let Some(l) = labels.get(i) {
                let text = l.clone().unwrap_or_else(|| unknown.clone());
                label(s, &text, dir(start + size / 2.0, r * 0.62))?;
            }
        }
        Ok(())
    }

    /// `pos`, `neg`: counters (yellow plus, red minus) in rows of five, for directed number.
    fn counters(&self, s: &mut Session) -> Result<()> {
        let (pos, neg) = (self.num("pos", 0.0).max(0.0) as usize, self.num("neg", 0.0).max(0.0) as usize);
        let total = (pos + neg).max(1);
        let cols = total.min(10);
        let rows = total.div_ceil(cols);
        let [x, y, w, h] = self.fit(cols as f64 / rows as f64 * 1.0, 6.0);
        let u = (w / cols as f64).min(h / rows as f64).min(34.0 * k());
        let left = x + (w - u * cols as f64) / 2.0;
        for i in 0..total {
            let (c, r) = (i % cols, i / cols);
            let positive = i < pos;
            let colour = if i >= pos + neg {
                "#FFFFFF"
            } else if positive {
                "#F6D860"
            } else {
                "#E8735F"
            };
            draw_shape(
                s,
                "ellipse",
                [left + u * c as f64 + u * 0.08, y + u * r as f64 + u * 0.08, u * 0.84, u * 0.84],
                Some(colour),
                Some((INK, 1.8)),
            )?;
            let ch = if positive { "+" } else { "−" };
            label_sized(s, ch, [left + u * (c as f64 + 0.5), y + u * (r as f64 + 0.5)], 15.0)?;
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
        // Columns are as wide as their longest text, so long headings and "Frequency" fit.
        let chars = |c: usize| -> usize {
            let cell = |t: &String| crate::latex::plain(t).chars().count();
            headers.get(c).map(cell).into_iter().chain(rows.iter().filter_map(|r| r.get(c)).map(cell)).max().unwrap_or(1).max(3)
        };
        let weights: Vec<f64> = (0..cols).map(|c| chars(c) as f64 + 1.5).collect();
        let total: f64 = weights.iter().sum();
        let [x, y, w, h] = self.fit((total / (4.0 * nrows.max(1) as f64)).max(1.6), 6.0);
        let rh = (h / nrows.max(1) as f64).min(44.0 * k());
        let top = y + (h - rh * nrows as f64) / 2.0;
        let mut row_i = 0usize;
        let mut draw_row = |s: &mut Session, cells: &[String], fill: &str, bold: bool| -> Result<()> {
            let mut cx = x;
            for (c, wt) in weights.iter().enumerate() {
                let cw = w * wt / total;
                let ry = top + rh * row_i as f64;
                draw_shape(s, "rect", [cx, ry, cw, rh], Some(fill), Some((INK, 1.8)))?;
                if let Some(t) = cells.get(c) {
                    let size = (15.0_f64).min((cw - 6.0) / (chars(c) as f64 * 0.62) / k()).max(8.0);
                    let _ = bold;
                    label_sized(s, t, [cx + cw / 2.0, ry + rh / 2.0], size)?;
                }
                cx += cw;
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
        let step = if x1 - x0 >= 12.0 { 2.0 } else { 1.0 };
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
        // Straight lines: `line` {m, c}, and `lines`, a list of {m, c}, {x} or {y} with an optional
        // `color`, `label` and `dash`. Each is clipped to the grid.
        let mut all: Vec<&Value> = self.d.get("line").into_iter().collect();
        all.extend(self.d.get("lines").and_then(Value::as_array).into_iter().flatten());
        for (i, l) in all.iter().enumerate() {
            let g = |key: &str| l.get(key).and_then(Value::as_f64);
            let mut ends: Vec<P> = vec![];
            if let Some(xv) = g("x") {
                ends = vec![[xv, y0], [xv, y1]];
            } else if let Some(yv) = g("y") {
                ends = vec![[x0, yv], [x1, yv]];
            } else {
                let (m, c) = (g("m").unwrap_or(1.0), g("c").unwrap_or(0.0));
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
            }
            if let [a, b, ..] = ends[..] {
                let color = l.get("color").and_then(Value::as_str).unwrap_or(["#D0453A", "#2F6FDB", "#2E9E5B"][i % 3]);
                let id = draw_line(s, [px(a[0]), py(a[1])], [px(b[0]), py(b[1])], color, 3.0)?;
                if l.get("dash").and_then(Value::as_bool).unwrap_or(false) {
                    s.execute("shape.line", &json!({"ids": [id], "dash": "dash"}))?;
                }
                if let Some(t) = l.get("label").and_then(Value::as_str) {
                    let e = if b[1] >= a[1] { b } else { a };
                    label(
                        s,
                        t,
                        [
                            (px(e[0]) - 6.0).clamp(x + 24.0, (x + w - 24.0).max(x + 24.0)),
                            (py(e[1]) + 14.0).clamp(y + 10.0, (y + h - 10.0).max(y + 10.0)),
                        ],
                    )?;
                }
            }
        }
        // Shapes: `segments` [[x1, y1, x2, y2]] and `polygons` [{points, color, label}].
        for seg in self.d.get("segments").and_then(Value::as_array).into_iter().flatten() {
            let v: Vec<f64> = seg.as_array().map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
            if let [ax, ay, bx, by] = v[..] {
                draw_line(s, [px(ax), py(ay)], [px(bx), py(by)], "#2F6FDB", 3.0)?;
            }
        }
        for pg in self.d.get("polygons").and_then(Value::as_array).into_iter().flatten() {
            let pts: Vec<P> = pg
                .get("points")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|p| Some([px(p.get(0)?.as_f64()?), py(p.get(1)?.as_f64()?)])).collect())
                .unwrap_or_default();
            if pts.len() < 3 {
                continue;
            }
            let color = pg.get("color").and_then(Value::as_str).unwrap_or("#2F6FDB");
            for i in 0..pts.len() {
                draw_line(s, pts[i], pts[(i + 1) % pts.len()], color, 3.0)?;
            }
            if let Some(t) = pg.get("label").and_then(Value::as_str) {
                let n = pts.len() as f64;
                label(s, t, [pts.iter().map(|p| p[0]).sum::<f64>() / n, pts.iter().map(|p| p[1]).sum::<f64>() / n])?;
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
