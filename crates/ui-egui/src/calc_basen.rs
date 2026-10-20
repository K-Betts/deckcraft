//! The Base-N app: whole-number arithmetic and logic in binary, octal, decimal and hexadecimal,
//! on 32-bit two's complement numbers.
#![allow(clippy::should_implement_trait, clippy::needless_range_loop)]

use crate::calculator::{CalcError, Func, Tok};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Base {
    Dec,
    Hex,
    Bin,
    Oct,
}

impl Base {
    pub const ALL: [Base; 4] = [Base::Dec, Base::Hex, Base::Bin, Base::Oct];

    pub fn radix(self) -> u32 {
        match self {
            Base::Dec => 10,
            Base::Hex => 16,
            Base::Bin => 2,
            Base::Oct => 8,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Base::Dec => "Dec",
            Base::Hex => "Hex",
            Base::Bin => "Bin",
            Base::Oct => "Oct",
        }
    }

    /// The most digits a 32-bit value takes in this base (the screen limit).
    fn max_digits(self) -> usize {
        match self {
            Base::Dec => 10,
            Base::Hex => 8,
            Base::Bin => 32,
            Base::Oct => 11,
        }
    }
}

struct P<'a> {
    toks: &'a [Tok],
    i: usize,
    base: Base,
}

fn wrap(v: i64) -> Result<i64, CalcError> {
    // Results outside 32 bits are a math error, as on the real thing.
    if (-2_147_483_648..=4_294_967_295).contains(&v) { Ok(v) } else { Err(CalcError::Math) }
}

/// Reinterpret as the signed 32-bit value a two's-complement register would hold.
fn signed(v: i64) -> i64 {
    i64::from(v as u32 as i32)
}

impl P<'_> {
    fn peek(&self) -> Option<Tok> {
        self.toks.get(self.i).copied()
    }

    fn close(&mut self) -> Result<(), CalcError> {
        if self.peek() == Some(Tok::RParen) {
            self.i += 1;
            Ok(())
        } else if self.peek().is_none() {
            Ok(())
        } else {
            Err(CalcError::Syntax)
        }
    }

    // Lowest: or, xor, xnor. Then and. Then + −. Then × ÷.
    fn or(&mut self) -> Result<i64, CalcError> {
        let mut v = self.and()?;
        while let Some(Tok::Logic(k @ 1..=3)) = self.peek() {
            self.i += 1;
            let r = self.and()?;
            let (a, b) = (v as u32, r as u32);
            v = i64::from(match k {
                1 => a | b,
                2 => a ^ b,
                _ => !(a ^ b),
            });
            v = signed(v);
        }
        Ok(v)
    }

    fn and(&mut self) -> Result<i64, CalcError> {
        let mut v = self.sum()?;
        while self.peek() == Some(Tok::Logic(0)) {
            self.i += 1;
            let r = self.sum()?;
            v = signed(i64::from(v as u32 & r as u32));
        }
        Ok(v)
    }

    fn sum(&mut self) -> Result<i64, CalcError> {
        let mut v = self.product()?;
        loop {
            match self.peek() {
                Some(Tok::Plus) => {
                    self.i += 1;
                    v = wrap(v + self.product()?)?;
                }
                Some(Tok::Minus) => {
                    self.i += 1;
                    v = wrap(v - self.product()?)?;
                }
                _ => return Ok(v),
            }
        }
    }

    fn product(&mut self) -> Result<i64, CalcError> {
        let mut v = self.unary()?;
        loop {
            match self.peek() {
                Some(Tok::Times) => {
                    self.i += 1;
                    v = wrap(v.checked_mul(self.unary()?).ok_or(CalcError::Math)?)?;
                }
                Some(Tok::Divide) => {
                    self.i += 1;
                    let d = self.unary()?;
                    if d == 0 {
                        return Err(CalcError::Math);
                    }
                    v /= d;
                }
                _ => return Ok(v),
            }
        }
    }

    fn unary(&mut self) -> Result<i64, CalcError> {
        if matches!(self.peek(), Some(Tok::Neg | Tok::Minus)) {
            self.i += 1;
            return wrap(-self.unary()?);
        }
        match self.peek().ok_or(CalcError::Syntax)? {
            Tok::LParen => {
                self.i += 1;
                let v = self.or()?;
                self.close()?;
                Ok(v)
            }
            Tok::Func(f @ (Func::Not | Func::NegB)) => {
                self.i += 1;
                let v = self.or()?;
                self.close()?;
                Ok(if f == Func::Not { signed(i64::from(!(v as u32))) } else { signed(-v) })
            }
            Tok::Digit(_) | Tok::Var(0..=5) => {
                let mut v: i64 = 0;
                while let Some(t) = self.peek() {
                    let d = match t {
                        Tok::Digit(d) => u32::from(d),
                        Tok::Var(i @ 0..=5) => 10 + u32::from(i),
                        _ => break,
                    };
                    if d >= self.base.radix() {
                        return Err(CalcError::Syntax);
                    }
                    v = v.checked_mul(i64::from(self.base.radix())).and_then(|x| x.checked_add(i64::from(d))).ok_or(CalcError::Math)?;
                    self.i += 1;
                }
                // A hex, binary or octal number is a bit pattern; in decimal it is just a number.
                if self.base == Base::Dec {
                    wrap(v)
                } else if v <= 4_294_967_295 {
                    Ok(signed(v))
                } else {
                    Err(CalcError::Math)
                }
            }
            _ => Err(CalcError::Syntax),
        }
    }
}

/// Evaluate an expression typed in `base`.
pub fn evaluate(toks: &[Tok], base: Base) -> Result<i64, CalcError> {
    let mut p = P { toks, i: 0, base };
    let v = p.or()?;
    if p.i == toks.len() { Ok(v) } else { Err(CalcError::Syntax) }
}

/// The digits of `v` in `base`: two's complement for the bit-pattern bases, a sign for decimal.
pub fn show(v: i64, base: Base) -> String {
    let text = if base == Base::Dec {
        v.to_string()
    } else {
        let u = v as u32;
        match base {
            Base::Hex => format!("{u:X}"),
            Base::Bin => format!("{u:b}"),
            _ => format!("{u:o}"),
        }
    };
    debug_assert!(text.trim_start_matches('-').len() <= base.max_digits() + 1);
    text
}

/// The answer as the tokens of digits (letters are `Var` tokens, A to F).
pub fn tokens(v: i64, base: Base) -> Vec<Tok> {
    let mut out = vec![];
    for c in show(v, base).chars() {
        out.push(match c {
            '-' => Tok::Minus,
            d if d.is_ascii_digit() => Tok::Digit(d as u8 - b'0'),
            l => Tok::Var(l as u8 - b'A'),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use Tok::*;

    #[test]
    fn arithmetic_in_each_base() {
        assert_eq!(evaluate(&[Digit(1), Digit(2), Plus, Digit(3)], Base::Dec), Ok(15));
        // hex: 1F + 1 = 20
        assert_eq!(evaluate(&[Digit(1), Var(5), Plus, Digit(1)], Base::Hex), Ok(0x20));
        // binary: 101 × 11 = 1111
        assert_eq!(evaluate(&[Digit(1), Digit(0), Digit(1), Times, Digit(1), Digit(1)], Base::Bin), Ok(15));
        // octal: 77 + 1 = 100
        assert_eq!(evaluate(&[Digit(7), Digit(7), Plus, Digit(1)], Base::Oct), Ok(64));
    }

    #[test]
    fn a_digit_that_does_not_belong_is_a_syntax_error() {
        assert_eq!(evaluate(&[Digit(2)], Base::Bin), Err(CalcError::Syntax));
        assert_eq!(evaluate(&[Digit(8)], Base::Oct), Err(CalcError::Syntax));
        assert_eq!(evaluate(&[Var(0)], Base::Dec), Err(CalcError::Syntax), "A is not a decimal digit");
    }

    #[test]
    fn logic_works_on_bit_patterns() {
        // 1010 and 0110 = 0010; or = 1110; xor = 1100
        let (a, b) = ([Digit(1), Digit(0), Digit(1), Digit(0)], [Digit(0), Digit(1), Digit(1), Digit(0)]);
        let with = |op: u8| [&a[..], &[Logic(op)], &b[..]].concat();
        assert_eq!(evaluate(&with(0), Base::Bin), Ok(0b0010));
        assert_eq!(evaluate(&with(1), Base::Bin), Ok(0b1110));
        assert_eq!(evaluate(&with(2), Base::Bin), Ok(0b1100));
        assert_eq!(show(evaluate(&with(3), Base::Bin).unwrap(), Base::Bin), format!("{:b}", !0b1100u32), "xnor flips all 32 bits");
    }

    #[test]
    fn not_and_negative_numbers_are_twos_complement() {
        assert_eq!(evaluate(&[Tok::Func(crate::calculator::Func::Not), Digit(0), RParen], Base::Hex), Ok(-1));
        assert_eq!(show(-1, Base::Hex), "FFFFFFFF");
        assert_eq!(show(-1, Base::Dec), "-1");
        assert_eq!(evaluate(&[Tok::Func(crate::calculator::Func::NegB), Digit(5), RParen], Base::Dec), Ok(-5));
        assert_eq!(show(-5, Base::Bin), "11111111111111111111111111111011");
    }

    #[test]
    fn overflow_and_division() {
        assert_eq!(
            evaluate(
                &[Digit(9), Digit(9), Digit(9), Digit(9), Digit(9), Digit(9), Digit(9), Digit(9), Digit(9), Digit(9), Times, Digit(9)],
                Base::Dec
            ),
            Err(CalcError::Math)
        );
        assert_eq!(evaluate(&[Digit(7), Divide, Digit(2)], Base::Dec), Ok(3), "whole numbers only");
        assert_eq!(evaluate(&[Digit(7), Divide, Digit(0)], Base::Dec), Err(CalcError::Math));
    }

    #[test]
    fn the_answer_is_shown_as_digit_tokens() {
        assert_eq!(tokens(255, Base::Hex), vec![Var(5), Var(5)]);
        assert_eq!(tokens(-3, Base::Dec), vec![Minus, Digit(3)]);
    }
}
