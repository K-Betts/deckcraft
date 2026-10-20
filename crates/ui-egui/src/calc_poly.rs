//! Polynomial roots, inequalities and simultaneous equations for the calculator's Equation and
//! Inequality apps, plus the numerical Solver.
#![allow(clippy::should_implement_trait, clippy::needless_range_loop)]

use crate::calculator::{CalcError, Value, div, mul, sub};

/// A complex number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct C {
    pub re: f64,
    pub im: f64,
}

impl C {
    pub fn new(re: f64, im: f64) -> C {
        C { re, im }
    }

    pub fn add(self, o: C) -> C {
        C::new(self.re + o.re, self.im + o.im)
    }

    pub fn sub(self, o: C) -> C {
        C::new(self.re - o.re, self.im - o.im)
    }

    pub fn mul(self, o: C) -> C {
        C::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }

    pub fn div(self, o: C) -> C {
        let d = o.re * o.re + o.im * o.im;
        C::new((self.re * o.re + self.im * o.im) / d, (self.im * o.re - self.re * o.im) / d)
    }

    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
}

/// Evaluate with coefficients `c[0] + c[1] x + ...`.
fn horner(c: &[f64], x: C) -> C {
    c.iter().rev().fold(C::new(0.0, 0.0), |acc, k| acc.mul(x).add(C::new(*k, 0.0)))
}

/// All roots of the polynomial with ascending coefficients `c` (the last non-zero is the lead).
/// Real roots come first, in ascending order, then the complex ones.
pub fn roots(c: &[f64]) -> Vec<C> {
    let mut c = c.to_vec();
    while c.last().is_some_and(|k| *k == 0.0) {
        c.pop();
    }
    let n = c.len().saturating_sub(1);
    if n == 0 {
        return vec![];
    }
    let lead = c[n];
    let monic: Vec<f64> = c.iter().map(|k| k / lead).collect();
    // Durand–Kerner.
    let radius = 1.0 + monic[..n].iter().map(|k| k.abs()).fold(0.0, f64::max);
    let mut z: Vec<C> = (0..n)
        .map(|k| {
            let t = 2.0 * std::f64::consts::PI * k as f64 / n as f64 + 0.4;
            C::new(radius * 0.5 * t.cos(), radius * 0.5 * t.sin())
        })
        .collect();
    for _ in 0..500 {
        let mut worst: f64 = 0.0;
        for i in 0..n {
            let mut denom = C::new(1.0, 0.0);
            for j in 0..n {
                if i != j {
                    denom = denom.mul(z[i].sub(z[j]));
                }
            }
            if denom.abs() < 1e-300 {
                continue;
            }
            let delta = horner(&monic, z[i]).div(denom);
            z[i] = z[i].sub(delta);
            worst = worst.max(delta.abs());
        }
        if worst < 1e-15 {
            break;
        }
    }
    // Polish with Newton and snap tiny parts to zero.
    let deriv: Vec<f64> = monic.iter().enumerate().skip(1).map(|(i, k)| k * i as f64).collect();
    for r in &mut z {
        for _ in 0..4 {
            let d = horner(&deriv, *r);
            if d.abs() > 1e-300 {
                *r = r.sub(horner(&monic, *r).div(d));
            }
        }
        let scale = r.abs().max(1.0);
        if r.im.abs() < 1e-9 * scale {
            r.im = 0.0;
        }
        if r.re.abs() < 1e-12 * scale {
            r.re = 0.0;
        }
        // Twelve significant figures: root-finding noise out, real values kept.
        let tidy = |v: f64| format!("{v:.11e}").parse::<f64>().unwrap_or(v);
        (r.re, r.im) = (tidy(r.re), tidy(r.im));
    }
    let (mut real, mut cplx): (Vec<C>, Vec<C>) = z.into_iter().partition(|r| r.im == 0.0);
    real.sort_by(|a, b| a.re.total_cmp(&b.re));
    cplx.sort_by(|a, b| a.re.total_cmp(&b.re).then(b.im.total_cmp(&a.im)));
    real.extend(cplx);
    real
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmp {
    Greater,
    Less,
    GreaterEq,
    LessEq,
}

impl Cmp {
    pub const ALL: [Cmp; 4] = [Cmp::Greater, Cmp::Less, Cmp::GreaterEq, Cmp::LessEq];

    pub fn text(self) -> &'static str {
        match self {
            Cmp::Greater => ">",
            Cmp::Less => "<",
            Cmp::GreaterEq => "\u{2265}",
            Cmp::LessEq => "\u{2264}",
        }
    }

    fn wants_positive(self) -> bool {
        matches!(self, Cmp::Greater | Cmp::GreaterEq)
    }

    fn inclusive(self) -> bool {
        matches!(self, Cmp::GreaterEq | Cmp::LessEq)
    }
}

/// The solution of `poly(x) cmp 0` as intervals over the real line: `(lo, hi, lo_closed, hi_closed)`.
#[derive(Clone, Debug, PartialEq)]
pub enum Solution {
    None,
    All,
    Intervals(Vec<(f64, f64, bool, bool)>),
}

pub fn inequality(c: &[f64], cmp: Cmp) -> Solution {
    let mut rs: Vec<f64> = roots(c).iter().filter(|r| r.im == 0.0).map(|r| r.re).collect();
    rs.dedup_by(|a, b| (*a - *b).abs() < 1e-9 * a.abs().max(1.0));
    let sign_at = |x: f64| horner(c, C::new(x, 0.0)).re;
    // A point in each gap between roots, and beyond both ends.
    let mut probes = vec![rs.first().map_or(0.0, |r| r - 1.0)];
    for w in rs.windows(2) {
        probes.push((w[0] + w[1]) / 2.0);
    }
    if let Some(l) = rs.last() {
        probes.push(l + 1.0);
    }
    let mut out: Vec<(f64, f64, bool, bool)> = vec![];
    for (i, p) in probes.iter().enumerate() {
        let v = sign_at(*p);
        if (v > 0.0) == cmp.wants_positive() && v != 0.0 {
            let lo = if i == 0 { f64::NEG_INFINITY } else { rs[i - 1] };
            let hi = if i == rs.len() { f64::INFINITY } else { rs[i] };
            out.push((lo, hi, cmp.inclusive() && lo.is_finite(), cmp.inclusive() && hi.is_finite()));
        }
    }
    // Roots themselves count for ≥ and ≤ even when no open gap touches them.
    if cmp.inclusive() {
        for r in &rs {
            if !out.iter().any(|(lo, hi, _, _)| lo <= r && r <= hi) {
                out.push((*r, *r, true, true));
            }
        }
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    // Join neighbours that touch and are closed there.
    let mut joined: Vec<(f64, f64, bool, bool)> = vec![];
    for iv in out {
        match joined.last_mut() {
            Some(last) if last.1 == iv.0 && (last.3 || iv.2) => {
                last.1 = iv.1;
                last.3 = iv.3;
            }
            _ => joined.push(iv),
        }
    }
    if joined.is_empty() {
        Solution::None
    } else if joined.len() == 1 && joined[0].0 == f64::NEG_INFINITY && joined[0].1 == f64::INFINITY {
        Solution::All
    } else {
        Solution::Intervals(joined)
    }
}

/// The vertex of `a x² + b x + c` and whether it is a minimum.
pub fn vertex(a: f64, b: f64, c: f64) -> Option<(f64, f64, bool)> {
    (a != 0.0).then(|| {
        let x = -b / (2.0 * a);
        (x, a * x * x + b * x + c, a > 0.0)
    })
}

/// Solve a square system exactly by Gauss–Jordan elimination over the rationals. `rows` hold the
/// coefficients followed by the constant. A zero determinant is a math error.
pub fn solve_exact(rows: &[Vec<Value>]) -> Result<Vec<Value>, CalcError> {
    let n = rows.len();
    let mut m: Vec<Vec<Value>> = rows.to_vec();
    if m.iter().any(|r| r.len() != n + 1) {
        return Err(CalcError::Syntax);
    }
    for col in 0..n {
        let pivot = (col..n).find(|&r| !m[r][col].is_zero()).ok_or(CalcError::Math)?;
        m.swap(col, pivot);
        let p = m[col][col];
        for k in col..=n {
            m[col][k] = div(m[col][k], p)?;
        }
        for r in 0..n {
            if r != col && !m[r][col].is_zero() {
                let f = m[r][col];
                for k in col..=n {
                    let t = mul(f, m[col][k])?;
                    m[r][k] = sub(m[r][k], t)?;
                }
            }
        }
    }
    Ok((0..n).map(|i| m[i][n]).collect())
}

/// Solve `f(x) = 0` near `start` by the secant method, falling back to widening a bracket and
/// bisecting. Returns the root and the value of `f` there.
pub fn solve_numeric(f: &mut dyn FnMut(f64) -> Option<f64>, start: f64) -> Option<(f64, f64)> {
    let (mut x0, mut x1) = (start, start + 0.1 * start.abs().max(1.0));
    let (mut f0, mut f1) = (f(x0)?, f(x1)?);
    for _ in 0..100 {
        if f1 == 0.0 {
            return Some((x1, 0.0));
        }
        if (f1 - f0).abs() < 1e-300 {
            break;
        }
        let x2 = x1 - f1 * (x1 - x0) / (f1 - f0);
        if !x2.is_finite() {
            break;
        }
        let f2 = f(x2)?;
        (x0, f0, x1, f1) = (x1, f1, x2, f2);
        if (x1 - x0).abs() < 1e-13 * x1.abs().max(1.0) && f1.abs() < 1e-9 {
            return Some((x1, f1));
        }
    }
    // Look for a sign change around the start and bisect.
    let mut width = 0.5;
    for _ in 0..60 {
        let (a, b) = (start - width, start + width);
        if let (Some(fa), Some(fb)) = (f(a), f(b))
            && fa.signum() != fb.signum()
        {
            let (mut lo, mut hi, mut flo) = (a, b, fa);
            for _ in 0..200 {
                let mid = (lo + hi) / 2.0;
                let fm = f(mid)?;
                if fm.signum() == flo.signum() {
                    (lo, flo) = (mid, fm);
                } else {
                    hi = mid;
                }
            }
            let x = (lo + hi) / 2.0;
            return Some((x, f(x)?));
        }
        width *= 1.7;
    }
    (f1.abs() < 1e-6).then_some((x1, f1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(c: &[f64]) -> Vec<f64> {
        roots(c).iter().filter(|r| r.im == 0.0).map(|r| (r.re * 1e8).round() / 1e8).collect()
    }

    #[test]
    fn quadratic_cubic_and_quartic_roots() {
        assert_eq!(real(&[6.0, -5.0, 1.0]), vec![2.0, 3.0]);
        assert_eq!(real(&[-6.0, 11.0, -6.0, 1.0]), vec![1.0, 2.0, 3.0]);
        assert_eq!(real(&[24.0, -50.0, 35.0, -10.0, 1.0]), vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn complex_roots_come_after_the_real_ones() {
        // (x − 1)(x² + 1) = x³ − x² + x − 1
        let r = roots(&[-1.0, 1.0, -1.0, 1.0]);
        assert_eq!(r.len(), 3);
        assert!((r[0].re - 1.0).abs() < 1e-9 && r[0].im == 0.0);
        assert!(r[1].re.abs() < 1e-9 && (r[1].im - 1.0).abs() < 1e-9, "positive imaginary part first");
        assert!((r[2].im + 1.0).abs() < 1e-9);
    }

    #[test]
    fn repeated_roots() {
        let r = real(&[1.0, -2.0, 1.0]);
        assert!(r.iter().all(|x| (x - 1.0).abs() < 1e-6), "{r:?}");
    }

    #[test]
    fn the_guides_inequality() {
        // x² + 2x − 3 ≥ 0 is x ≤ −3 or x ≥ 1
        let s = inequality(&[-3.0, 2.0, 1.0], Cmp::GreaterEq);
        assert_eq!(s, Solution::Intervals(vec![(f64::NEG_INFINITY, -3.0, false, true), (1.0, f64::INFINITY, true, false)]));
        // x² + 2x − 3 < 0 is −3 < x < 1
        assert_eq!(inequality(&[-3.0, 2.0, 1.0], Cmp::Less), Solution::Intervals(vec![(-3.0, 1.0, false, false)]));
        // x² < 0 has no solution, x² ≥ 0 is every real number
        assert_eq!(inequality(&[0.0, 0.0, 1.0], Cmp::Less), Solution::None);
        assert_eq!(inequality(&[0.0, 0.0, 1.0], Cmp::GreaterEq), Solution::All);
        // x² ≤ 0 is just x = 0
        assert_eq!(inequality(&[0.0, 0.0, 1.0], Cmp::LessEq), Solution::Intervals(vec![(0.0, 0.0, true, true)]));
        // cubic: (x−1)(x−2)(x−3) > 0 is 1<x<2 or x>3
        let s = inequality(&[-6.0, 11.0, -6.0, 1.0], Cmp::Greater);
        assert_eq!(s, Solution::Intervals(vec![(1.0, 2.0, false, false), (3.0, f64::INFINITY, false, false)]));
    }

    #[test]
    fn the_vertex_of_a_parabola() {
        // x² + 2x − 2 has its minimum at (−1, −3), as in the guide
        assert_eq!(vertex(1.0, 2.0, -2.0), Some((-1.0, -3.0, true)));
        assert_eq!(vertex(-1.0, 0.0, 4.0), Some((0.0, 4.0, false)));
        assert_eq!(vertex(0.0, 1.0, 1.0), None);
    }

    #[test]
    fn three_unknowns_solve_exactly() {
        let v = |n: i128| Value::int(n);
        // x + y + z = 6, 2y + 5z = −4, 2x + 5y − z = 27 → (5, 3, −2)
        let rows = vec![vec![v(1), v(1), v(1), v(6)], vec![v(0), v(2), v(5), v(-4)], vec![v(2), v(5), v(-1), v(27)]];
        assert_eq!(solve_exact(&rows).unwrap(), vec![v(5), v(3), v(-2)]);
        let singular = vec![vec![v(1), v(1), v(2)], vec![v(2), v(2), v(5)]];
        assert_eq!(solve_exact(&singular), Err(CalcError::Math));
    }

    #[test]
    fn the_solver_finds_a_root_from_a_nearby_guess() {
        // x² − B/2 = 0 with B = 4: x = √2 from x = 1
        let (x, resid) = solve_numeric(&mut |x| Some(x * x - 2.0), 1.0).unwrap();
        assert!((x - 2f64.sqrt()).abs() < 1e-10 && resid.abs() < 1e-9);
        // A start where the secant step is flat still finds the root by bracketing.
        let (x, _) = solve_numeric(&mut |x| Some(x * x * x - 8.0), 0.0).unwrap();
        assert!((x - 2.0).abs() < 1e-8);
        // x² + 1 = 0 has none.
        assert!(solve_numeric(&mut |x| Some(x * x + 1.0), 1.0).is_none());
    }
}
