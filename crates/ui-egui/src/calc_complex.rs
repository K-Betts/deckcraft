//! The Complex app: expressions over complex numbers, `a+bi`.
#![allow(clippy::should_implement_trait, clippy::needless_range_loop)]

use crate::calc_poly::C;
use crate::calculator::{Angle, CalcError, Func, Tok};

pub fn from_polar(r: f64, theta: f64) -> C {
    C::new(r * theta.cos(), r * theta.sin())
}

fn polar(z: C) -> (f64, f64) {
    (z.abs(), z.im.atan2(z.re))
}

pub fn exp(z: C) -> C {
    from_polar(z.re.exp(), z.im)
}

pub fn ln(z: C) -> Result<C, CalcError> {
    if z.re == 0.0 && z.im == 0.0 {
        return Err(CalcError::Math);
    }
    let (r, t) = polar(z);
    Ok(C::new(r.ln(), t))
}

pub fn pow(a: C, b: C) -> Result<C, CalcError> {
    if a.re == 0.0 && a.im == 0.0 {
        return if b.re > 0.0 { Ok(C::new(0.0, 0.0)) } else { Err(CalcError::Math) };
    }
    // Whole powers by repeated multiplication keep integers exact.
    if b.im == 0.0 && b.re.fract() == 0.0 && b.re.abs() <= 64.0 {
        let mut out = C::new(1.0, 0.0);
        for _ in 0..(b.re.abs() as u32) {
            out = out.mul(a);
        }
        return Ok(if b.re < 0.0 { C::new(1.0, 0.0).div(out) } else { out });
    }
    Ok(exp(b.mul(ln(a)?)))
}

pub fn sqrt(z: C) -> C {
    let (r, t) = polar(z);
    from_polar(r.sqrt(), t / 2.0)
}

fn sin(z: C) -> C {
    C::new(z.re.sin() * z.im.cosh(), z.re.cos() * z.im.sinh())
}

fn cos(z: C) -> C {
    C::new(z.re.cos() * z.im.cosh(), -z.re.sin() * z.im.sinh())
}

struct P<'a> {
    toks: &'a [Tok],
    i: usize,
    angle: Angle,
    ans: C,
    vars: [f64; 9],
}

impl P<'_> {
    fn peek(&self) -> Option<Tok> {
        self.toks.get(self.i).copied()
    }

    fn eat(&mut self, t: Tok) -> bool {
        let hit = self.peek() == Some(t);
        if hit {
            self.i += 1;
        }
        hit
    }

    fn close(&mut self, t: Tok) -> Result<(), CalcError> {
        if self.eat(t) || self.peek().is_none() { Ok(()) } else { Err(CalcError::Syntax) }
    }

    fn expr(&mut self) -> Result<C, CalcError> {
        let mut v = self.term()?;
        loop {
            match self.peek() {
                Some(Tok::Plus) => {
                    self.i += 1;
                    v = v.add(self.term()?);
                }
                Some(Tok::Minus) => {
                    self.i += 1;
                    v = v.sub(self.term()?);
                }
                _ => return Ok(v),
            }
        }
    }

    fn term(&mut self) -> Result<C, CalcError> {
        let mut v = self.implicit()?;
        loop {
            match self.peek() {
                Some(Tok::Times) => {
                    self.i += 1;
                    v = v.mul(self.implicit()?);
                }
                Some(Tok::Divide) => {
                    self.i += 1;
                    let d = self.implicit()?;
                    if d.re == 0.0 && d.im == 0.0 {
                        return Err(CalcError::Math);
                    }
                    v = v.div(d);
                }
                _ => return Ok(v),
            }
        }
    }

    fn implicit(&mut self) -> Result<C, CalcError> {
        let mut v = self.signed()?;
        while self.peek().is_some_and(|t| {
            matches!(
                t,
                Tok::Digit(_)
                    | Tok::Dot
                    | Tok::LParen
                    | Tok::Pi
                    | Tok::E
                    | Tok::I
                    | Tok::Ans
                    | Tok::Var(_)
                    | Tok::X
                    | Tok::Func(_)
                    | Tok::SqrtOpen
                    | Tok::FracOpen
            )
        }) {
            v = v.mul(self.signed()?);
        }
        Ok(v)
    }

    fn signed(&mut self) -> Result<C, CalcError> {
        if matches!(self.peek(), Some(Tok::Neg | Tok::Minus)) {
            self.i += 1;
            let v = self.signed()?;
            return Ok(C::new(0.0 - v.re, 0.0 - v.im));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<C, CalcError> {
        let mut v = self.primary()?;
        loop {
            match self.peek() {
                Some(Tok::SupOpen) => {
                    self.i += 1;
                    let e = self.expr()?;
                    self.close(Tok::Close)?;
                    v = pow(v, e)?;
                }
                Some(Tok::Percent) => {
                    self.i += 1;
                    v = v.div(C::new(100.0, 0.0));
                }
                _ => return Ok(v),
            }
        }
    }

    fn number(&mut self) -> Result<C, CalcError> {
        let mut text = String::new();
        while let Some(t) = self.peek() {
            match t {
                Tok::Digit(d) => text.push(char::from(b'0' + d)),
                Tok::Dot => text.push('.'),
                _ => break,
            }
            self.i += 1;
        }
        text.parse::<f64>().map(|v| C::new(v, 0.0)).map_err(|_| CalcError::Syntax)
    }

    fn primary(&mut self) -> Result<C, CalcError> {
        match self.peek().ok_or(CalcError::Syntax)? {
            Tok::Digit(_) | Tok::Dot => self.number(),
            Tok::I => {
                self.i += 1;
                Ok(C::new(0.0, 1.0))
            }
            Tok::Pi => {
                self.i += 1;
                Ok(C::new(std::f64::consts::PI, 0.0))
            }
            Tok::E => {
                self.i += 1;
                Ok(C::new(std::f64::consts::E, 0.0))
            }
            Tok::Ans => {
                self.i += 1;
                Ok(self.ans)
            }
            Tok::X => {
                self.i += 1;
                Ok(C::new(self.vars[6], 0.0))
            }
            Tok::Var(i) => {
                self.i += 1;
                self.vars.get(usize::from(i)).map(|v| C::new(*v, 0.0)).ok_or(CalcError::Syntax)
            }
            Tok::LParen => {
                self.i += 1;
                let v = self.expr()?;
                self.close(Tok::RParen)?;
                Ok(v)
            }
            Tok::SqrtOpen => {
                self.i += 1;
                let v = self.expr()?;
                self.close(Tok::Close)?;
                Ok(sqrt(v))
            }
            Tok::FracOpen => {
                self.i += 1;
                let top = self.expr()?;
                if !self.eat(Tok::FracMid) {
                    return Err(CalcError::Syntax);
                }
                let bottom = self.expr()?;
                self.close(Tok::Close)?;
                if bottom.re == 0.0 && bottom.im == 0.0 {
                    return Err(CalcError::Math);
                }
                Ok(top.div(bottom))
            }
            Tok::Func(f) => {
                self.i += 1;
                let v = self.expr()?;
                self.close(Tok::RParen)?;
                let real = |x: f64| C::new(x, 0.0);
                match f {
                    Func::Abs => Ok(real(v.abs())),
                    Func::Arg => Ok(real(self.angle.from_radians(v.im.atan2(v.re)))),
                    Func::Conj => Ok(C::new(v.re, -v.im)),
                    Func::Re => Ok(real(v.re)),
                    Func::Im => Ok(real(v.im)),
                    Func::Ln => ln(v),
                    Func::Log => Ok(ln(v)?.div(real(10f64.ln()))),
                    Func::Sin => Ok(sin(C::new(self.angle.to_radians(v.re), v.im))),
                    Func::Cos => Ok(cos(C::new(self.angle.to_radians(v.re), v.im))),
                    Func::Tan => {
                        let z = C::new(self.angle.to_radians(v.re), v.im);
                        let c = cos(z);
                        if c.abs() < 1e-12 { Err(CalcError::Math) } else { Ok(sin(z).div(c)) }
                    }
                    _ => Err(CalcError::Syntax),
                }
            }
            _ => Err(CalcError::Syntax),
        }
    }
}

/// Evaluate a complex expression.
pub fn evaluate(toks: &[Tok], ans: C, angle: Angle, vars: [f64; 9]) -> Result<C, CalcError> {
    let mut p = P { toks, i: 0, angle, ans, vars };
    let v = p.expr()?;
    if p.i != toks.len() {
        return Err(CalcError::Syntax);
    }
    if v.re.is_finite() && v.im.is_finite() { Ok(v) } else { Err(CalcError::Math) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculator::Tok::*;

    fn ev(t: &[Tok]) -> C {
        evaluate(t, C::new(0.0, 0.0), Angle::Degree, [0.0; 9]).unwrap()
    }

    fn close(z: C, re: f64, im: f64) -> bool {
        (z.re - re).abs() < 1e-9 && (z.im - im).abs() < 1e-9
    }

    #[test]
    fn arithmetic_with_i() {
        // (1 + 2i)(3 − i) = 5 + 5i
        assert!(close(ev(&[LParen, Digit(1), Plus, Digit(2), I, RParen, LParen, Digit(3), Minus, I, RParen]), 5.0, 5.0));
        // i² = −1
        assert!(close(ev(&[I, SupOpen, Digit(2), Close]), -1.0, 0.0));
        // 1 ÷ i = −i
        assert!(close(ev(&[Digit(1), Divide, I]), 0.0, -1.0));
    }

    #[test]
    fn roots_of_negative_numbers() {
        // √(−4) = 2i
        assert!(close(ev(&[SqrtOpen, Neg, Digit(4), Close]), 0.0, 2.0));
    }

    #[test]
    fn abs_arg_and_conjugate() {
        assert!(close(ev(&[Tok::Func(crate::calculator::Func::Abs), Digit(3), Plus, Digit(4), I, RParen]), 5.0, 0.0));
        assert!(close(ev(&[Tok::Func(crate::calculator::Func::Arg), Digit(1), Plus, I, RParen]), 45.0, 0.0));
        assert!(close(ev(&[Tok::Func(crate::calculator::Func::Conj), Digit(1), Plus, Digit(2), I, RParen]), 1.0, -2.0));
        assert!(close(ev(&[Tok::Func(crate::calculator::Func::Re), Digit(1), Plus, Digit(2), I, RParen]), 1.0, 0.0));
        assert!(close(ev(&[Tok::Func(crate::calculator::Func::Im), Digit(1), Plus, Digit(2), I, RParen]), 2.0, 0.0));
    }

    #[test]
    fn euler_and_fractional_powers() {
        // e^(iπ) = −1
        assert!(close(ev(&[E, SupOpen, I, Pi, Close]), -1.0, 0.0));
        // i^i = e^(−π/2)
        assert!(close(ev(&[I, SupOpen, I, Close]), (-std::f64::consts::FRAC_PI_2).exp(), 0.0));
    }

    #[test]
    fn division_by_zero_and_syntax() {
        assert_eq!(evaluate(&[Digit(1), Divide, Digit(0)], C::new(0.0, 0.0), Angle::Degree, [0.0; 9]), Err(CalcError::Math));
        assert_eq!(evaluate(&[Plus], C::new(0.0, 0.0), Angle::Degree, [0.0; 9]), Err(CalcError::Syntax));
    }
}
