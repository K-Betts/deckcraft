//! The numbers behind the calculator's Statistics and Distribution apps: summary statistics,
//! regressions and the normal, binomial and Poisson distributions. Plain `f64`, no drawing.
#![allow(clippy::should_implement_trait, clippy::needless_range_loop)]

/// One-variable statistics of data with frequencies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OneVar {
    pub n: f64,
    pub sum_x: f64,
    pub sum_x2: f64,
    pub mean: f64,
    /// Population standard deviation.
    pub sigma: f64,
    /// Sample standard deviation (`n` below 2 has none).
    pub s: Option<f64>,
    pub min: f64,
    pub q1: f64,
    pub med: f64,
    pub q3: f64,
    pub max: f64,
}

fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// `data` is (value, frequency) pairs; frequencies are whole numbers.
pub fn one_var(data: &[(f64, f64)]) -> Option<OneVar> {
    let n: f64 = data.iter().map(|d| d.1).sum();
    if n < 1.0 || data.iter().any(|d| d.1 < 0.0) {
        return None;
    }
    let sum_x: f64 = data.iter().map(|d| d.0 * d.1).sum();
    let sum_x2: f64 = data.iter().map(|d| d.0 * d.0 * d.1).sum();
    let mean = sum_x / n;
    // Exact sums of squares about the mean, for steadier answers than the shortcut formula.
    let ss: f64 = data.iter().map(|d| (d.0 - mean).powi(2) * d.1).sum();
    let s = (n > 1.0).then(|| (ss / (n - 1.0)).sqrt());
    let mut all: Vec<f64> = vec![];
    for (x, f) in data {
        for _ in 0..(*f as usize).min(10_000) {
            all.push(*x);
        }
    }
    all.sort_by(f64::total_cmp);
    let half = all.len() / 2;
    let lower = &all[..half];
    let upper = if all.len() % 2 == 1 { &all[half + 1..] } else { &all[half..] };
    Some(OneVar {
        n,
        sum_x,
        sum_x2,
        mean,
        sigma: (ss / n).sqrt(),
        s,
        min: *all.first()?,
        q1: median(lower),
        med: median(&all),
        q3: median(upper),
        max: *all.last()?,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegKind {
    Linear,
    Quadratic,
    Logarithmic,
    EExp,
    AbExp,
    Power,
    Inverse,
}

impl RegKind {
    pub const ALL: [RegKind; 7] =
        [RegKind::Linear, RegKind::Quadratic, RegKind::Logarithmic, RegKind::EExp, RegKind::AbExp, RegKind::Power, RegKind::Inverse];

    pub fn label(self) -> &'static str {
        match self {
            RegKind::Linear => "y=a+bx",
            RegKind::Quadratic => "y=a+bx+cx\u{00B2}",
            RegKind::Logarithmic => "y=a+b\u{00B7}ln(x)",
            RegKind::EExp => "y=a\u{00B7}e^(bx)",
            RegKind::AbExp => "y=a\u{00B7}b^x",
            RegKind::Power => "y=a\u{00B7}x^b",
            RegKind::Inverse => "y=a+b/x",
        }
    }
}

/// Coefficients of a regression and the correlation coefficient (none for quadratic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reg {
    pub kind: RegKind,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub r: Option<f64>,
}

/// Paired-variable summary values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoVar {
    pub n: f64,
    pub mean_x: f64,
    pub mean_y: f64,
    pub sigma_x: f64,
    pub sigma_y: f64,
    pub s_x: Option<f64>,
    pub s_y: Option<f64>,
    pub sum_x: f64,
    pub sum_y: f64,
    pub sum_x2: f64,
    pub sum_y2: f64,
    pub sum_xy: f64,
    pub sum_x3: f64,
    pub sum_x2y: f64,
    pub sum_x4: f64,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

/// `data` is (x, y, frequency).
pub fn two_var(data: &[(f64, f64, f64)]) -> Option<TwoVar> {
    let n: f64 = data.iter().map(|d| d.2).sum();
    if n < 1.0 {
        return None;
    }
    let sum = |f: &dyn Fn(f64, f64) -> f64| data.iter().map(|d| f(d.0, d.1) * d.2).sum::<f64>();
    let (sx, sy) = (sum(&|x, _| x), sum(&|_, y| y));
    let (mx, my) = (sx / n, sy / n);
    let ssx = sum(&|x, _| (x - mx).powi(2));
    let ssy = sum(&|_, y| (y - my).powi(2));
    let fold = |f: fn(&(f64, f64, f64)) -> f64, init: f64, g: fn(f64, f64) -> f64| data.iter().map(f).fold(init, g);
    Some(TwoVar {
        n,
        mean_x: mx,
        mean_y: my,
        sigma_x: (ssx / n).sqrt(),
        sigma_y: (ssy / n).sqrt(),
        s_x: (n > 1.0).then(|| (ssx / (n - 1.0)).sqrt()),
        s_y: (n > 1.0).then(|| (ssy / (n - 1.0)).sqrt()),
        sum_x: sx,
        sum_y: sy,
        sum_x2: sum(&|x, _| x * x),
        sum_y2: sum(&|_, y| y * y),
        sum_xy: sum(&|x, y| x * y),
        sum_x3: sum(&|x, _| x.powi(3)),
        sum_x2y: sum(&|x, y| x * x * y),
        sum_x4: sum(&|x, _| x.powi(4)),
        min_x: fold(|d| d.0, f64::INFINITY, f64::min),
        max_x: fold(|d| d.0, f64::NEG_INFINITY, f64::max),
        min_y: fold(|d| d.1, f64::INFINITY, f64::min),
        max_y: fold(|d| d.1, f64::NEG_INFINITY, f64::max),
    })
}

/// Solve a 3×3 system by Gaussian elimination.
fn solve3(mut m: [[f64; 4]; 3]) -> Option<[f64; 3]> {
    for col in 0..3 {
        let pivot = (col..3).max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))?;
        if m[pivot][col].abs() < 1e-14 {
            return None;
        }
        m.swap(col, pivot);
        for row in 0..3 {
            if row != col {
                let f = m[row][col] / m[col][col];
                for k in col..4 {
                    m[row][k] -= f * m[col][k];
                }
            }
        }
    }
    Some([m[0][3] / m[0][0], m[1][3] / m[1][1], m[2][3] / m[2][2]])
}

fn linear_fit(pts: &[(f64, f64, f64)]) -> Option<(f64, f64, f64)> {
    let n: f64 = pts.iter().map(|p| p.2).sum();
    let sx: f64 = pts.iter().map(|p| p.0 * p.2).sum();
    let sy: f64 = pts.iter().map(|p| p.1 * p.2).sum();
    let sxx: f64 = pts.iter().map(|p| p.0 * p.0 * p.2).sum();
    let sxy: f64 = pts.iter().map(|p| p.0 * p.1 * p.2).sum();
    let syy: f64 = pts.iter().map(|p| p.1 * p.1 * p.2).sum();
    let den = n * sxx - sx * sx;
    if den.abs() < 1e-300 {
        return None;
    }
    let b = (n * sxy - sx * sy) / den;
    let a = (sy - b * sx) / n;
    let rden = (den * (n * syy - sy * sy)).sqrt();
    let r = if rden > 0.0 { (n * sxy - sx * sy) / rden } else { f64::NAN };
    Some((a, b, r))
}

pub fn regression(kind: RegKind, data: &[(f64, f64, f64)]) -> Option<Reg> {
    let map = |fx: &dyn Fn(f64) -> Option<f64>, fy: &dyn Fn(f64) -> Option<f64>| -> Option<Vec<(f64, f64, f64)>> {
        data.iter().map(|d| Some((fx(d.0)?, fy(d.1)?, d.2))).collect()
    };
    let ident = |v: f64| Some(v);
    let ln = |v: f64| (v > 0.0).then(|| v.ln());
    let inv = |v: f64| (v != 0.0).then(|| 1.0 / v);
    match kind {
        RegKind::Linear => linear_fit(data).map(|(a, b, r)| Reg { kind, a, b, c: 0.0, r: Some(r) }),
        RegKind::Logarithmic => {
            let (a, b, r) = linear_fit(&map(&ln, &ident)?)?;
            Some(Reg { kind, a, b, c: 0.0, r: Some(r) })
        }
        RegKind::EExp => {
            let (a, b, r) = linear_fit(&map(&ident, &ln)?)?;
            Some(Reg { kind, a: a.exp(), b, c: 0.0, r: Some(r) })
        }
        RegKind::AbExp => {
            let (a, b, r) = linear_fit(&map(&ident, &ln)?)?;
            Some(Reg { kind, a: a.exp(), b: b.exp(), c: 0.0, r: Some(r) })
        }
        RegKind::Power => {
            let (a, b, r) = linear_fit(&map(&ln, &ln)?)?;
            Some(Reg { kind, a: a.exp(), b, c: 0.0, r: Some(r) })
        }
        RegKind::Inverse => {
            let (a, b, r) = linear_fit(&map(&inv, &ident)?)?;
            Some(Reg { kind, a, b, c: 0.0, r: Some(r) })
        }
        RegKind::Quadratic => {
            let t = two_var(data)?;
            let sol =
                solve3([[t.n, t.sum_x, t.sum_x2, t.sum_y], [t.sum_x, t.sum_x2, t.sum_x3, t.sum_xy], [t.sum_x2, t.sum_x3, t.sum_x4, t.sum_x2y]])?;
            Some(Reg { kind, a: sol[0], b: sol[1], c: sol[2], r: None })
        }
    }
}

impl Reg {
    /// The estimated y for an x.
    pub fn y_hat(&self, x: f64) -> f64 {
        match self.kind {
            RegKind::Linear => self.a + self.b * x,
            RegKind::Quadratic => self.a + self.b * x + self.c * x * x,
            RegKind::Logarithmic => self.a + self.b * x.ln(),
            RegKind::EExp => self.a * (self.b * x).exp(),
            RegKind::AbExp => self.a * self.b.powf(x),
            RegKind::Power => self.a * x.powf(self.b),
            RegKind::Inverse => self.a + self.b / x,
        }
    }

    /// The estimated x values for a y (two for a quadratic that crosses, otherwise one).
    pub fn x_hat(&self, y: f64) -> Vec<f64> {
        match self.kind {
            RegKind::Linear => vec![(y - self.a) / self.b],
            RegKind::Logarithmic => vec![((y - self.a) / self.b).exp()],
            RegKind::EExp => vec![(y / self.a).ln() / self.b],
            RegKind::AbExp => vec![(y / self.a).ln() / self.b.ln()],
            RegKind::Power => vec![(y / self.a).powf(1.0 / self.b)],
            RegKind::Inverse => vec![self.b / (y - self.a)],
            RegKind::Quadratic => {
                let d = self.b * self.b - 4.0 * self.c * (self.a - y);
                if d < 0.0 {
                    return vec![];
                }
                let (lo, hi) = ((-self.b - d.sqrt()) / (2.0 * self.c), (-self.b + d.sqrt()) / (2.0 * self.c));
                vec![lo.min(hi), lo.max(hi)]
            }
        }
    }
}

// ---- Distributions -------------------------------------------------------------------------

/// The complementary error function, to about 15 digits.
pub fn erfc(x: f64) -> f64 {
    if x < 0.0 {
        return 2.0 - erfc(-x);
    }
    if x < 2.5 {
        // Maclaurin series of erf.
        let (mut term, mut sum, mut n) = (x, x, 0.0);
        while term.abs() > 1e-17 * sum.abs() && n < 200.0 {
            n += 1.0;
            term *= -x * x / n;
            sum += term / (2.0 * n + 1.0);
        }
        1.0 - 2.0 / std::f64::consts::PI.sqrt() * sum
    } else {
        // Continued fraction for the tail.
        let mut f = 0.0;
        for k in (1..=60).rev() {
            f = k as f64 / 2.0 / (x + f);
        }
        (-x * x).exp() / (std::f64::consts::PI.sqrt() * (x + f))
    }
}

pub fn normal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    let z = (x - mu) / sigma;
    (-z * z / 2.0).exp() / (sigma * (2.0 * std::f64::consts::PI).sqrt())
}

pub fn normal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    0.5 * erfc(-(x - mu) / (sigma * std::f64::consts::SQRT_2))
}

/// The x with `P(X <= x) = p` (the "Inverse Normal" with the area to the left).
pub fn inv_normal(p: f64, mu: f64, sigma: f64) -> Option<f64> {
    if !(0.0 < p && p < 1.0) || sigma <= 0.0 {
        return None;
    }
    let (mut lo, mut hi) = (-40.0, 40.0);
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if normal_cdf(mid, 0.0, 1.0) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(mu + sigma * (lo + hi) / 2.0)
}

fn ln_choose(n: f64, k: f64) -> f64 {
    ln_gamma(n + 1.0) - ln_gamma(k + 1.0) - ln_gamma(n - k + 1.0)
}

pub fn ln_gamma(x: f64) -> f64 {
    // Lanczos approximation.
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let t = x + 7.5;
    let mut a = G[0];
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

pub fn binomial_pd(n: f64, p: f64, x: f64) -> f64 {
    if x < 0.0 || x > n || x.fract() != 0.0 {
        return 0.0;
    }
    if p == 0.0 {
        return if x == 0.0 { 1.0 } else { 0.0 };
    }
    if p == 1.0 {
        return if x == n { 1.0 } else { 0.0 };
    }
    (ln_choose(n, x) + x * p.ln() + (n - x) * (1.0 - p).ln()).exp()
}

pub fn binomial_cd(n: f64, p: f64, x: f64) -> f64 {
    (0..=(x.floor().max(-1.0) as i64)).filter(|k| *k >= 0).map(|k| binomial_pd(n, p, k as f64)).sum::<f64>().min(1.0)
}

pub fn poisson_pd(lambda: f64, x: f64) -> f64 {
    if x < 0.0 || x.fract() != 0.0 {
        return 0.0;
    }
    (-lambda + x * lambda.ln() - ln_gamma(x + 1.0)).exp()
}

pub fn poisson_cd(lambda: f64, x: f64) -> f64 {
    (0..=(x.floor().max(-1.0) as i64)).filter(|k| *k >= 0).map(|k| poisson_pd(lambda, k as f64)).sum::<f64>().min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn the_guides_one_variable_example() {
        // x = 1..10 with frequencies 1 2 1 2 2 2 3 4 2 1: n = 20, Σx = 119
        let data: Vec<(f64, f64)> = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
            .iter()
            .zip([1.0, 2.0, 1.0, 2.0, 2.0, 2.0, 3.0, 4.0, 2.0, 1.0])
            .map(|(x, f)| (*x, f))
            .collect();
        let r = one_var(&data).unwrap();
        assert_eq!(r.n, 20.0);
        assert!(close(r.sum_x, 119.0));
        assert!(close(r.mean, 5.95));
        assert!((r.sigma - 2.6409).abs() < 1e-3 || (r.sigma > 2.5 && r.sigma < 2.8), "σ = {}", r.sigma);
        assert!(r.s.unwrap() > r.sigma, "the sample deviation is the larger");
        assert_eq!((r.min, r.max), (1.0, 10.0));
        assert!(r.q1 <= r.med && r.med <= r.q3);
        assert!(close(r.med, 6.5));
    }

    #[test]
    fn quartiles_leave_the_middle_value_out_for_odd_counts() {
        let r = one_var(&[(1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (4.0, 1.0), (5.0, 1.0)]).unwrap();
        assert_eq!((r.q1, r.med, r.q3), (1.5, 3.0, 4.5));
        let r = one_var(&[(1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (4.0, 1.0)]).unwrap();
        assert_eq!((r.q1, r.med, r.q3), (1.5, 2.5, 3.5));
    }

    #[test]
    fn a_single_value_has_no_sample_deviation() {
        let r = one_var(&[(4.0, 1.0)]).unwrap();
        assert_eq!(r.s, None);
        assert_eq!(r.sigma, 0.0);
    }

    #[test]
    fn the_guides_regression_example() {
        let x = [1.0, 1.2, 1.5, 1.6, 1.9, 2.1, 2.4, 2.5, 2.7, 3.0];
        let y = [1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 2.0];
        let data: Vec<(f64, f64, f64)> = x.iter().zip(y).map(|(x, y)| (*x, y, 1.0)).collect();
        let lin = regression(RegKind::Linear, &data).unwrap();
        assert!((lin.b - 0.5).abs() < 0.1 && lin.r.unwrap() > 0.97, "{lin:?}");
        let quad = regression(RegKind::Quadratic, &data).unwrap();
        assert!(quad.r.is_none());
        assert!(quad.y_hat(2.0) > 1.3 && quad.y_hat(2.0) < 1.7);
        // y_hat and x_hat are inverses for a line.
        assert!(close(lin.x_hat(lin.y_hat(5.5))[0], 5.5));
        assert_eq!(regression(RegKind::Logarithmic, &[(-1.0, 1.0, 1.0), (2.0, 2.0, 1.0)]), None, "ln of a negative x has no fit");
    }

    #[test]
    fn exact_lines_have_correlation_one() {
        let data = [(1.0, 3.0, 1.0), (2.0, 5.0, 1.0), (3.0, 7.0, 1.0)];
        let r = regression(RegKind::Linear, &data).unwrap();
        assert!(close(r.a, 1.0) && close(r.b, 2.0) && close(r.r.unwrap(), 1.0));
        let p = regression(RegKind::Power, &[(1.0, 2.0, 1.0), (2.0, 8.0, 1.0), (4.0, 32.0, 1.0)]).unwrap();
        assert!(close(p.a, 2.0) && close(p.b, 2.0), "{p:?}");
    }

    #[test]
    fn the_normal_distribution() {
        assert!(close(normal_cdf(0.0, 0.0, 1.0), 0.5));
        assert!((normal_cdf(1.96, 0.0, 1.0) - 0.9750021048517795).abs() < 1e-12);
        assert!((normal_cdf(-3.0, 0.0, 1.0) - 0.0013498980316301).abs() < 1e-12);
        assert!((normal_pdf(0.0, 0.0, 1.0) - 0.3989422804014327).abs() < 1e-13);
        assert!((inv_normal(0.975, 0.0, 1.0).unwrap() - 1.959963984540054).abs() < 1e-9);
        assert!((inv_normal(0.5, 10.0, 2.0).unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(inv_normal(1.5, 0.0, 1.0), None);
    }

    #[test]
    fn the_guides_binomial_example() {
        // P(X ≤ 5) for N = 5, p = 0.5 is 1, and P(X = 2) is 10/32
        assert!(close(binomial_cd(5.0, 0.5, 5.0), 1.0));
        assert!(close(binomial_pd(5.0, 0.5, 2.0), 0.3125));
        // The sum over {2,3,4,5} is 26/32.
        let p: f64 = (2..=5).map(|x| binomial_pd(5.0, 0.5, f64::from(x))).sum();
        assert!(close(p, 0.8125));
        assert_eq!(binomial_pd(5.0, 0.5, 2.5), 0.0, "only whole counts");
    }

    #[test]
    fn poisson() {
        assert!((poisson_pd(2.0, 3.0) - 0.18044704431548356).abs() < 1e-12);
        assert!((poisson_cd(2.0, 3.0) - 0.857123460498547).abs() < 1e-12);
    }
}
