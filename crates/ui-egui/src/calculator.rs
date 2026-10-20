//! The engine behind the on-screen scientific calculator, modelled on a Casio ClassWiz: an
//! expression is edited in place (fractions, roots and powers are templates you step into), answers
//! stay exact where they can (fractions and surds such as `2√3`), SHIFT reaches the second
//! function of a key, `Ans` and the replay history are kept, and S⇔D flips an answer between its
//! exact and decimal forms.
//!
//! No drawing happens here, so all of it is tested without a window. The keypad and the natural
//! display that draw this are in `calculator_ui`.

/// A function key that opens a bracket: `sin(` and friends. The bracket may be left unclosed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Func {
    Sin,
    Cos,
    Tan,
    ASin,
    ACos,
    ATan,
    Log,
    Ln,
    Abs,
    Sinh,
    Cosh,
    Tanh,
    ASinh,
    ACosh,
    ATanh,
    /// `f(` and `g(`: the functions the teacher has defined.
    F,
    G,
    /// Two arguments, separated by a comma.
    RanInt,
    Gcd,
    Lcm,
    Pol,
    Rec,
    /// Round off to the Number Format setting.
    Rnd,
    /// Complex numbers: argument, conjugate, real and imaginary parts.
    Arg,
    Conj,
    Re,
    Im,
    /// Base-N logic on whole numbers.
    Not,
    NegB,
    /// The Spreadsheet app's range functions: `Sum(A1:A5)`.
    SheetSum,
    SheetMean,
    SheetMin,
    SheetMax,
}

impl Func {
    /// Takes two arguments (`a, b`).
    pub fn two_args(self) -> bool {
        matches!(self, Func::RanInt | Func::Gcd | Func::Lcm | Func::Pol | Func::Rec)
    }

    pub fn text(self) -> &'static str {
        match self {
            Func::Sin => "sin(",
            Func::Cos => "cos(",
            Func::Tan => "tan(",
            Func::ASin => "sin\u{207B}\u{00B9}(",
            Func::ACos => "cos\u{207B}\u{00B9}(",
            Func::ATan => "tan\u{207B}\u{00B9}(",
            Func::Log => "log(",
            Func::Ln => "ln(",
            Func::Abs => "Abs(",
            Func::Sinh => "sinh(",
            Func::Cosh => "cosh(",
            Func::Tanh => "tanh(",
            Func::ASinh => "sinh\u{207B}\u{00B9}(",
            Func::ACosh => "cosh\u{207B}\u{00B9}(",
            Func::ATanh => "tanh\u{207B}\u{00B9}(",
            Func::F => "f(",
            Func::G => "g(",
            Func::RanInt => "RanInt#(",
            Func::Gcd => "GCD(",
            Func::Lcm => "LCM(",
            Func::Pol => "Pol(",
            Func::Rec => "Rec(",
            Func::Rnd => "Rnd(",
            Func::Arg => "Arg(",
            Func::Conj => "Conjg(",
            Func::Re => "ReP(",
            Func::Im => "ImP(",
            Func::Not => "Not(",
            Func::NegB => "Neg(",
            Func::SheetSum => "Sum(",
            Func::SheetMean => "Mean(",
            Func::SheetMin => "Min(",
            Func::SheetMax => "Max(",
        }
    }
}

/// One element of the expression being typed. Templates open with `FracOpen`, `SqrtOpen`,
/// `RootOpen` or `SupOpen`, may have a `FracMid` / `RootMid` between their two slots, and always
/// end with `Close`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tok {
    Digit(u8),
    Dot,
    Plus,
    Minus,
    Times,
    Divide,
    /// The `(−)` key: a sign, not a subtraction.
    Neg,
    LParen,
    RParen,
    Pi,
    E,
    /// The variable of a table or an equation: typed with the `x` key.
    X,
    /// A stored variable: A to F are 0 to 5, y and z are 7 and 8 (x is `X`).
    Var(u8),
    Ans,
    Fact,
    /// `%`: divide by a hundred.
    Percent,
    Ncr,
    Npr,
    Func(Func),
    FracOpen,
    FracMid,
    /// `√▭`
    SqrtOpen,
    /// `▭√▭`: the first slot is the index.
    RootOpen,
    RootMid,
    /// A superscript: the power of whatever comes before it.
    SupOpen,
    /// `log▭▭`: the base, then the number.
    LogOpen,
    LogMid,
    /// A mixed number: the whole part, the numerator, the denominator (two `FracMid` dividers).
    MixOpen,
    /// The bar of a fraction in a linear answer (`LineI/LineO`).
    Slash,
    /// `×10▭` as one number (the ×10ˣ key in its Sci Notat setting): binds tighter than × and ÷.
    Exp10Open,
    /// `d/dx(f)|x=a`, `∫ f dx` from a to b, `Σ f` for x from a to b: slots split by `SlotMid`.
    DerivOpen,
    IntOpen,
    SumOpen,
    SlotMid,
    /// `=` and `,` (SHIFT on the bracket keys).
    EqSign,
    Comma,
    /// The sexagesimal mark: `12°34°56°` is 12° 34′ 56″.
    Dms,
    /// The imaginary unit (the Complex app only).
    I,
    /// Angle marks: the value is in degrees, radians or gradians whatever the setting.
    AngD,
    AngR,
    AngG,
    Ran,
    /// `:` in a cell range, `A1:A5`.
    Colon,
    /// A statistic from the Statistics app, by index into the device's names.
    Stat(u8),
    /// ŷ (estimate y for the x before it) and x̂, x̂₁, x̂₂ (estimate x for a y).
    YHat,
    XHat(u8),
    /// A gap between columns in a matrix or vector answer.
    Space,
    /// Base-N logic between two whole numbers: and, or, xor, xnor.
    Logic(u8),
    /// A repeating block of decimal digits, drawn with a bar over it: `0.1(6)`.
    RecurOpen,
    /// A scientific constant, by index into [`CONSTANTS`].
    Const(u8),
    /// A unit conversion, by index into [`CONVERSIONS`].
    Conv(u8),
    /// Shown in answers only: minutes and seconds marks, the digit separator and a decimal comma.
    MinMark,
    SecMark,
    Sep,
    DecComma,
    Close,
}

/// The built-in scientific constants (CODATA 2022), by category: symbol and value.
pub const CONSTANTS: [(&str, &str, f64); 47] = [
    ("Universal", "h", 6.626_070_15e-34),
    ("Universal", "\u{0127}", 1.054_571_817e-34),
    ("Universal", "c\u{2080}", 299_792_458.0),
    ("Universal", "\u{03B5}\u{2080}", 8.854_187_818_8e-12),
    ("Universal", "\u{03BC}\u{2080}", 1.256_637_061_27e-6),
    ("Universal", "Z\u{2080}", 376.730_313_412),
    ("Universal", "G", 6.674_30e-11),
    ("Universal", "l\u{209A}", 1.616_255e-35),
    ("Universal", "t\u{209A}", 5.391_247e-44),
    ("Electromagnetic", "\u{03BC}N", 5.050_783_739_3e-27),
    ("Electromagnetic", "\u{03BC}B", 9.274_010_065_7e-24),
    ("Electromagnetic", "e", 1.602_176_634e-19),
    ("Electromagnetic", "\u{03A6}\u{2080}", 2.067_833_848e-15),
    ("Electromagnetic", "G\u{2080}", 7.748_091_729e-5),
    ("Electromagnetic", "KJ", 483_597.848_4e9),
    ("Electromagnetic", "RK", 25_812.807_45),
    ("Atomic&Nuclear", "mp", 1.672_621_925_95e-27),
    ("Atomic&Nuclear", "mn", 1.674_927_500_56e-27),
    ("Atomic&Nuclear", "me", 9.109_383_713_9e-31),
    ("Atomic&Nuclear", "m\u{03BC}", 1.883_531_627e-28),
    ("Atomic&Nuclear", "a\u{2080}", 5.291_772_105_44e-11),
    ("Atomic&Nuclear", "\u{03B1}", 7.297_352_564_3e-3),
    ("Atomic&Nuclear", "re", 2.817_940_320_5e-15),
    ("Atomic&Nuclear", "\u{03BB}C", 2.426_310_235_38e-12),
    ("Atomic&Nuclear", "\u{03B3}p", 2.675_221_870_8e8),
    ("Atomic&Nuclear", "\u{03BB}Cp", 1.321_409_853_60e-15),
    ("Atomic&Nuclear", "\u{03BB}Cn", 1.319_590_903_82e-15),
    ("Atomic&Nuclear", "R\u{221E}", 10_973_731.568_157),
    ("Atomic&Nuclear", "\u{03BC}p", 1.410_606_795_45e-26),
    ("Atomic&Nuclear", "\u{03BC}e", -9.284_764_691_7e-24),
    ("Atomic&Nuclear", "\u{03BC}n", -9.662_365_3e-27),
    ("Atomic&Nuclear", "\u{03BC}\u{03BC}", -4.490_448_30e-26),
    ("Atomic&Nuclear", "m\u{03C4}", 3.167_54e-27),
    ("Physico-Chem", "mu", 1.660_539_068_92e-27),
    ("Physico-Chem", "F", 96_485.332_12),
    ("Physico-Chem", "NA", 6.022_140_76e23),
    ("Physico-Chem", "k", 1.380_649e-23),
    ("Physico-Chem", "Vm", 22.413_969_54e-3),
    ("Physico-Chem", "R", 8.314_462_618),
    ("Physico-Chem", "c\u{2081}", 3.741_771_852e-16),
    ("Physico-Chem", "c\u{2082}", 1.438_776_877e-2),
    ("Physico-Chem", "\u{03C3}", 5.670_374_419e-8),
    ("Adopted Values", "gn", 9.806_65),
    ("Adopted Values", "atm", 101_325.0),
    ("Adopted Values", "RK-90", 25_812.807),
    ("Adopted Values", "KJ-90", 483_597.9e9),
    ("Other", "t", 273.15),
];

/// Unit conversions (NIST SP 811), by category: the command and `to = from × factor + offset`.
pub const CONVERSIONS: [(&str, &str, f64, f64); 40] = [
    ("Length", "in\u{2192}cm", 2.54, 0.0),
    ("Length", "cm\u{2192}in", 1.0 / 2.54, 0.0),
    ("Length", "ft\u{2192}m", 0.3048, 0.0),
    ("Length", "m\u{2192}ft", 1.0 / 0.3048, 0.0),
    ("Length", "yd\u{2192}m", 0.9144, 0.0),
    ("Length", "m\u{2192}yd", 1.0 / 0.9144, 0.0),
    ("Length", "mile\u{2192}km", 1.609_344, 0.0),
    ("Length", "km\u{2192}mile", 1.0 / 1.609_344, 0.0),
    ("Length", "n mile\u{2192}m", 1852.0, 0.0),
    ("Length", "m\u{2192}n mile", 1.0 / 1852.0, 0.0),
    ("Length", "pc\u{2192}km", 3.085_677_581_491_367e13, 0.0),
    ("Length", "km\u{2192}pc", 1.0 / 3.085_677_581_491_367e13, 0.0),
    ("Area", "acre\u{2192}m\u{00B2}", 4_046.856_422_4, 0.0),
    ("Area", "m\u{00B2}\u{2192}acre", 1.0 / 4_046.856_422_4, 0.0),
    ("Volume", "gal(US)\u{2192}L", 3.785_411_784, 0.0),
    ("Volume", "L\u{2192}gal(US)", 1.0 / 3.785_411_784, 0.0),
    ("Volume", "gal(UK)\u{2192}L", 4.546_09, 0.0),
    ("Volume", "L\u{2192}gal(UK)", 1.0 / 4.546_09, 0.0),
    ("Mass", "oz\u{2192}g", 28.349_523_125, 0.0),
    ("Mass", "g\u{2192}oz", 1.0 / 28.349_523_125, 0.0),
    ("Mass", "lb\u{2192}kg", 0.453_592_37, 0.0),
    ("Mass", "kg\u{2192}lb", 1.0 / 0.453_592_37, 0.0),
    ("Velocity", "km/h\u{2192}m/s", 1.0 / 3.6, 0.0),
    ("Velocity", "m/s\u{2192}km/h", 3.6, 0.0),
    ("Pressure", "atm\u{2192}Pa", 101_325.0, 0.0),
    ("Pressure", "Pa\u{2192}atm", 1.0 / 101_325.0, 0.0),
    ("Pressure", "mmHg\u{2192}Pa", 133.322_387_415, 0.0),
    ("Pressure", "Pa\u{2192}mmHg", 1.0 / 133.322_387_415, 0.0),
    ("Pressure", "kgf/cm\u{00B2}\u{2192}Pa", 98_066.5, 0.0),
    ("Pressure", "Pa\u{2192}kgf/cm\u{00B2}", 1.0 / 98_066.5, 0.0),
    ("Pressure", "lbf/in\u{00B2}\u{2192}kPa", 6.894_757_293_168, 0.0),
    ("Pressure", "kPa\u{2192}lbf/in\u{00B2}", 1.0 / 6.894_757_293_168, 0.0),
    ("Energy", "kgf\u{00B7}m\u{2192}J", 9.806_65, 0.0),
    ("Energy", "J\u{2192}kgf\u{00B7}m", 1.0 / 9.806_65, 0.0),
    ("Energy", "J\u{2192}cal\u{2081}\u{2085}", 1.0 / 4.1855, 0.0),
    ("Energy", "cal\u{2081}\u{2085}\u{2192}J", 4.1855, 0.0),
    ("Power", "hp\u{2192}kW", 0.745_699_872, 0.0),
    ("Power", "kW\u{2192}hp", 1.0 / 0.745_699_872, 0.0),
    ("Temperature", "\u{00B0}F\u{2192}\u{00B0}C", 5.0 / 9.0, -32.0 * 5.0 / 9.0),
    ("Temperature", "\u{00B0}C\u{2192}\u{00B0}F", 1.8, 32.0),
];

impl Tok {
    pub fn opens(self) -> bool {
        matches!(
            self,
            Tok::FracOpen
                | Tok::SqrtOpen
                | Tok::RootOpen
                | Tok::SupOpen
                | Tok::LogOpen
                | Tok::MixOpen
                | Tok::Exp10Open
                | Tok::DerivOpen
                | Tok::IntOpen
                | Tok::SumOpen
                | Tok::RecurOpen
        )
    }

    pub fn is_mid(self) -> bool {
        matches!(self, Tok::FracMid | Tok::RootMid | Tok::LogMid | Tok::SlotMid)
    }

    /// What the token looks like when it is drawn as plain text.
    pub fn text(self) -> String {
        match self {
            Tok::Digit(d) => d.to_string(),
            Tok::Dot => ".".into(),
            Tok::Plus => "+".into(),
            Tok::Minus | Tok::Neg => "\u{2212}".into(),
            Tok::Times => "\u{00D7}".into(),
            Tok::Divide => "\u{00F7}".into(),
            Tok::LParen => "(".into(),
            Tok::RParen => ")".into(),
            Tok::Pi => "\u{03C0}".into(),
            Tok::E => "e".into(),
            Tok::X => "x".into(),
            Tok::Var(i) => ["A", "B", "C", "D", "E", "F", "x", "y", "z"].get(usize::from(i)).copied().unwrap_or("?").into(),
            Tok::Slash => "/".into(),
            Tok::EqSign => "=".into(),
            Tok::Comma | Tok::DecComma | Tok::Sep => ",".into(),
            Tok::Dms | Tok::AngD => "\u{00B0}".into(),
            Tok::MinMark => "\u{2032}".into(),
            Tok::SecMark => "\u{2033}".into(),
            Tok::I => "i".into(),
            Tok::AngR => "r".into(),
            Tok::AngG => "g".into(),
            Tok::Ran => "Ran#".into(),
            Tok::Colon => ":".into(),
            Tok::YHat => "\u{0177}".into(),
            Tok::XHat(0) => "x^".into(),
            Tok::XHat(k) => format!("x^{k}"),
            Tok::Stat(i) => crate::calculator_forms::STAT_NAMES.get(usize::from(i)).copied().unwrap_or("?").into(),
            Tok::Space => "  ".into(),
            Tok::Logic(k) => ["and", "or", "xor", "xnor"].get(usize::from(k)).copied().unwrap_or("?").into(),
            Tok::Const(i) => CONSTANTS.get(usize::from(i)).map_or("?", |c| c.1).into(),
            Tok::Conv(i) => CONVERSIONS.get(usize::from(i)).map_or("?", |c| c.1).into(),
            Tok::Ans => "Ans".into(),
            Tok::Fact => "!".into(),
            Tok::Percent => "%".into(),
            Tok::Ncr => "C".into(),
            Tok::Npr => "P".into(),
            Tok::Func(f) => f.text().into(),
            Tok::FracOpen
            | Tok::FracMid
            | Tok::SqrtOpen
            | Tok::RootOpen
            | Tok::RootMid
            | Tok::SupOpen
            | Tok::LogOpen
            | Tok::LogMid
            | Tok::MixOpen
            | Tok::Exp10Open
            | Tok::DerivOpen
            | Tok::IntOpen
            | Tok::SumOpen
            | Tok::SlotMid
            | Tok::RecurOpen
            | Tok::Close => String::new(),
        }
    }
}

/// Why an expression has no answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CalcError {
    /// Dividing by zero, a root of a negative number, tan 90°: the sum makes no sense.
    Math,
    /// The expression is not well formed: a gap in a fraction, a stray bracket.
    Syntax,
}

impl CalcError {
    pub fn message(self) -> &'static str {
        match self {
            CalcError::Math => "Math ERROR",
            CalcError::Syntax => "Syntax ERROR",
        }
    }
}

// ---- Settings and memory ------------------------------------------------------------------

/// The unit angles are typed and answered in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Angle {
    Degree,
    Radian,
    Gradian,
}

impl Angle {
    pub fn to_radians(self, x: f64) -> f64 {
        match self {
            Angle::Degree => x.to_radians(),
            Angle::Radian => x,
            Angle::Gradian => x * std::f64::consts::PI / 200.0,
        }
    }

    pub fn from_radians(self, r: f64) -> f64 {
        match self {
            Angle::Degree => r.to_degrees(),
            Angle::Radian => r,
            Angle::Gradian => r * 200.0 / std::f64::consts::PI,
        }
    }

    pub fn next(self) -> Angle {
        match self {
            Angle::Degree => Angle::Radian,
            Angle::Radian => Angle::Gradian,
            Angle::Gradian => Angle::Degree,
        }
    }

    /// The letter on the screen's status line.
    pub fn letter(self) -> &'static str {
        match self {
            Angle::Degree => "D",
            Angle::Radian => "R",
            Angle::Gradian => "G",
        }
    }
}

/// How sums are typed and answered (SETTINGS > Input/Output).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Io {
    MathMath,
    MathDecimal,
    LineLine,
    LineDecimal,
}

impl Io {
    pub fn decimal_output(self) -> bool {
        matches!(self, Io::MathDecimal | Io::LineDecimal)
    }

    pub fn line_output(self) -> bool {
        matches!(self, Io::LineLine | Io::LineDecimal)
    }
}

/// SETTINGS > Number Format.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NumFmt {
    /// Decimal places, 0 to 9.
    Fix(u8),
    /// Significant digits, 1 to 10.
    Sci(u8),
    /// Standard form outside 10⁻² to 10¹⁰ (`Norm(1)`) or 10⁻⁹ to 10¹⁰ (`Norm(2)`).
    Norm(u8),
}

/// The way the answer is shown (the FORMAT menu).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResultFormat {
    Standard,
    Decimal,
    PrimeFactor,
    ImproperFraction,
    MixedNumber,
    StandardForm,
    /// Degrees, minutes and seconds.
    Sexagesimal,
    /// A repeating decimal, with a bar over the repeating digits.
    Recurring,
}

/// The calculator's settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Config {
    pub io: Io,
    pub num: NumFmt,
    /// Fractions in answers as mixed numbers rather than improper.
    pub mixed: bool,
    /// The ×10ˣ key types `×10^(` (Power) rather than the Sci Notat `×10▭` that binds as one number.
    pub exp_power: bool,
    /// Commas every three digits in answers (Digit Separator, on at first).
    pub separator: bool,
    /// A comma for the decimal point in answers.
    pub comma_mark: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { io: Io::MathMath, num: NumFmt::Norm(2), mixed: false, exp_power: false, separator: true, comma_mark: false }
    }
}

/// What the calculator remembers between sums: the nine variables and the functions f and g.
#[derive(Clone, Debug)]
pub struct Memory {
    pub vars: [Value; 9],
    pub defs: [Vec<Tok>; 2],
    /// The Statistics app's values for the Statistics Calc screen, by [`STAT_NAMES`] index.
    pub stats: [f64; 27],
    /// The regression the Statistics Calc screen estimates with.
    pub reg: Option<crate::calc_stats::Reg>,
}

impl Default for Memory {
    fn default() -> Self {
        Memory { vars: [Value::Rat(Rat { n: 0, d: 1 }); 9], defs: [vec![], vec![]], stats: [f64::NAN; 27], reg: None }
    }
}

/// Everything a sum can refer to.
#[derive(Clone, Copy)]
pub struct Env<'a> {
    pub ans: Value,
    pub angle: Angle,
    pub x: Option<Value>,
    pub mem: &'a Memory,
    /// For Rnd(.
    pub num: NumFmt,
    /// The Spreadsheet app's cells: `A1` is column 0, row 1.
    pub cells: Option<&'a dyn Fn(u8, u32) -> Result<Value, CalcError>>,
}

// ---- Exact numbers -------------------------------------------------------------------------

pub(crate) fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// A fraction in lowest terms with a positive denominator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rat {
    pub n: i128,
    pub d: i128,
}

impl Rat {
    pub fn new(n: i128, d: i128) -> Option<Rat> {
        if d == 0 {
            return None;
        }
        let g = gcd(n, d).max(1);
        let (n, d) = (n / g, d / g);
        Some(if d < 0 { Rat { n: n.checked_neg()?, d: d.checked_neg()? } } else { Rat { n, d } })
    }

    pub fn int(n: i128) -> Rat {
        Rat { n, d: 1 }
    }

    pub fn is_int(self) -> bool {
        self.d == 1
    }

    pub fn to_f64(self) -> f64 {
        self.n as f64 / self.d as f64
    }

    pub(crate) fn add(self, o: Rat) -> Option<Rat> {
        Rat::new(self.n.checked_mul(o.d)?.checked_add(o.n.checked_mul(self.d)?)?, self.d.checked_mul(o.d)?)
    }

    pub(crate) fn mul(self, o: Rat) -> Option<Rat> {
        Rat::new(self.n.checked_mul(o.n)?, self.d.checked_mul(o.d)?)
    }

    pub(crate) fn neg(self) -> Option<Rat> {
        Some(Rat { n: self.n.checked_neg()?, d: self.d })
    }

    pub(crate) fn recip(self) -> Option<Rat> {
        Rat::new(self.d, self.n)
    }

    pub(crate) fn powi(self, k: i128) -> Option<Rat> {
        let base = if k < 0 { self.recip()? } else { self };
        let mut out = Rat::int(1);
        for _ in 0..k.unsigned_abs() {
            out = out.mul(base)?;
        }
        Some(out)
    }
}

/// The integer `k`th root of `x`, when it is exact.
fn iroot(x: i128, k: u32) -> Option<i128> {
    if x < 0 || k == 0 {
        return None;
    }
    let guess = (x as f64).powf(1.0 / f64::from(k)).round() as i128;
    (guess.saturating_sub(1)..=guess.saturating_add(1)).find(|&c| c >= 0 && c.checked_pow(k) == Some(x))
}

/// A value the calculator holds: exact where it can be, a float where it cannot.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    Rat(Rat),
    /// A sum of a rational and one or two roots, such as `1/3 + 2√3` or `45√3 + 10√2`.
    Surd(Surds),
    /// A rational multiple of π, such as `π/6`.
    Pi(Rat),
    Float(f64),
}

/// `rat + Σ coef × √rad`: each `rad` a whole number above 1, free of square factors, and different.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Surds {
    pub rat: Rat,
    pub terms: [(Rat, i128); 2],
    pub n: usize,
}

impl Surds {
    pub fn roots(&self) -> &[(Rat, i128)] {
        &self.terms[..self.n.min(2)]
    }
}

impl Value {
    pub fn int(n: i128) -> Value {
        Value::Rat(Rat::int(n))
    }

    pub fn to_f64(self) -> f64 {
        match self {
            Value::Rat(r) => r.to_f64(),
            Value::Surd(s) => s.rat.to_f64() + s.roots().iter().map(|(c, r)| c.to_f64() * (*r as f64).sqrt()).sum::<f64>(),
            Value::Pi(k) => k.to_f64() * std::f64::consts::PI,
            Value::Float(f) => f,
        }
    }

    pub(crate) fn is_zero(self) -> bool {
        matches!(self, Value::Rat(r) if r.n == 0) || matches!(self, Value::Float(f) if f == 0.0)
    }

    /// The value as `(coefficient, radicand)` parts, radicand 1 for the rational part.
    fn parts(self) -> Option<Vec<(Rat, i128)>> {
        match self {
            Value::Rat(r) => Some(vec![(r, 1)]),
            Value::Surd(s) => {
                let mut v = vec![(s.rat, 1)];
                v.extend_from_slice(s.roots());
                Some(v)
            }
            _ => None,
        }
    }
}

fn float(x: f64) -> Result<Value, CalcError> {
    if x.is_finite() { Ok(Value::Float(x)) } else { Err(CalcError::Math) }
}

fn from_rat(r: Option<Rat>, fallback: f64) -> Result<Value, CalcError> {
    r.map_or_else(|| float(fallback), |r| Ok(Value::Rat(r)))
}

/// `k` and `m` with `k² × m = n` and `m` free of square factors.
fn simplify_root(n: i128) -> Option<(i128, i128)> {
    if !(0..=1_000_000_000_000).contains(&n) {
        return None;
    }
    let (mut outside, mut inside, mut f) = (1i128, n, 2i128);
    while f * f <= inside {
        while inside % (f * f) == 0 {
            inside /= f * f;
            outside *= f;
        }
        f += 1;
    }
    Some((outside, inside))
}

/// Gather parts into a value: like roots combined, zeros dropped. More than two roots is not
/// something the screen can show exactly, so that is `None`.
fn from_parts(parts: Vec<(Rat, i128)>) -> Option<Value> {
    let mut rat = Rat::int(0);
    let mut roots: Vec<(Rat, i128)> = vec![];
    for (c, r) in parts {
        let (k, m) = simplify_root(r)?;
        let c = c.mul(Rat::int(k))?;
        if m == 1 {
            rat = rat.add(c)?;
        } else if let Some(slot) = roots.iter_mut().find(|(_, rr)| *rr == m) {
            slot.0 = slot.0.add(c)?;
        } else {
            roots.push((c, m));
        }
    }
    roots.retain(|(c, _)| c.n != 0);
    // The bigger root first, as the calculator writes 45√3 + 10√2.
    roots.sort_by_key(|t| std::cmp::Reverse(t.1));
    match roots.len() {
        0 => Some(Value::Rat(rat)),
        1 | 2 => {
            let mut terms = [(Rat::int(0), 1); 2];
            for (i, t) in roots.iter().enumerate() {
                terms[i] = *t;
            }
            Some(Value::Surd(Surds { rat, terms, n: roots.len() }))
        }
        _ => None,
    }
}

/// `coef × √rad` in simplest form, or an exact rational when the root comes out whole.
pub(crate) fn surd(coef: Rat, rad: i128) -> Value {
    from_parts(vec![(coef, rad)]).unwrap_or(Value::Float(coef.to_f64() * (rad as f64).sqrt()))
}

fn sqrt_rat(r: Rat) -> Result<Value, CalcError> {
    if r.n < 0 {
        return Err(CalcError::Math);
    }
    // √(n/d) = √(n·d) / d, and √(n·d) simplifies as a whole number.
    let Some(prod) = r.n.checked_mul(r.d) else { return float(r.to_f64().sqrt()) };
    if prod > 1_000_000_000_000 {
        return float(r.to_f64().sqrt());
    }
    Ok(Rat::new(1, r.d).map_or_else(|| Value::Float(r.to_f64().sqrt()), |c| surd(c, prod)))
}

pub(crate) fn sqrt(v: Value) -> Result<Value, CalcError> {
    match v {
        Value::Rat(r) => sqrt_rat(r),
        other => {
            let x = other.to_f64();
            if x < 0.0 { Err(CalcError::Math) } else { float(x.sqrt()) }
        }
    }
}

fn scale(v: Value, k: Rat) -> Result<Value, CalcError> {
    match v {
        Value::Pi(p) => Ok(p.mul(k).map_or(Value::Float(v.to_f64() * k.to_f64()), |c| if c.n == 0 { Value::int(0) } else { Value::Pi(c) })),
        _ => mul(v, Value::Rat(k)),
    }
}

pub(crate) fn neg(v: Value) -> Result<Value, CalcError> {
    match v {
        Value::Float(f) => Ok(Value::Float(-f)),
        _ => scale(v, Rat::int(-1)),
    }
}

pub(crate) fn add(a: Value, b: Value) -> Result<Value, CalcError> {
    if a.is_zero() && !matches!(a, Value::Float(_)) {
        return Ok(b);
    }
    if b.is_zero() && !matches!(b, Value::Float(_)) {
        return Ok(a);
    }
    match (a, b) {
        (Value::Pi(x), Value::Pi(y)) => Ok(match x.add(y) {
            Some(c) if c.n == 0 => Value::int(0),
            Some(c) => Value::Pi(c),
            None => Value::Float(a.to_f64() + b.to_f64()),
        }),
        _ => match (a.parts(), b.parts()) {
            (Some(mut x), Some(y)) => {
                x.extend(y);
                from_parts(x).map_or_else(|| float(a.to_f64() + b.to_f64()), Ok)
            }
            _ => float(a.to_f64() + b.to_f64()),
        },
    }
}

pub(crate) fn sub(a: Value, b: Value) -> Result<Value, CalcError> {
    add(a, neg(b)?)
}

pub(crate) fn mul(a: Value, b: Value) -> Result<Value, CalcError> {
    match (a, b) {
        (Value::Rat(x), Value::Rat(y)) => from_rat(x.mul(y), x.to_f64() * y.to_f64()),
        (Value::Pi(p), Value::Rat(k)) | (Value::Rat(k), Value::Pi(p)) => Ok(match p.mul(k) {
            Some(c) if c.n == 0 => Value::int(0),
            Some(c) => Value::Pi(c),
            None => Value::Float(a.to_f64() * b.to_f64()),
        }),
        _ => match (a.parts(), b.parts()) {
            (Some(x), Some(y)) => {
                let mut out = vec![];
                for (c1, r1) in &x {
                    for (c2, r2) in &y {
                        match (c1.mul(*c2), r1.checked_mul(*r2)) {
                            (Some(c), Some(r)) => out.push((c, r)),
                            _ => return float(a.to_f64() * b.to_f64()),
                        }
                    }
                }
                from_parts(out).map_or_else(|| float(a.to_f64() * b.to_f64()), Ok)
            }
            _ => float(a.to_f64() * b.to_f64()),
        },
    }
}

pub(crate) fn div(a: Value, b: Value) -> Result<Value, CalcError> {
    if b.is_zero() {
        return Err(CalcError::Math);
    }
    let fallback = || float(a.to_f64() / b.to_f64());
    match b {
        Value::Rat(k) => match k.recip() {
            Some(r) => scale(a, r),
            None => fallback(),
        },
        Value::Pi(q) => match a {
            Value::Pi(p) => from_rat(q.recip().and_then(|r| p.mul(r)), a.to_f64() / b.to_f64()),
            _ => fallback(),
        },
        // Clear the root from the bottom: multiply top and bottom by the conjugate.
        Value::Surd(s) if s.n == 1 => {
            let (c, r) = s.terms[0];
            let conj = sub(Value::Rat(s.rat), surd(c, r))?;
            let bottom = match (s.rat.mul(s.rat), c.mul(c).and_then(|cc| cc.mul(Rat::int(r)))) {
                (Some(p2), Some(q2r)) => p2.add(q2r.neg().unwrap_or(Rat::int(0))),
                _ => None,
            };
            match bottom {
                Some(d) if d.n != 0 => div(mul(a, conj)?, Value::Rat(d)),
                _ => fallback(),
            }
        }
        _ => fallback(),
    }
}

pub(crate) fn pow(a: Value, b: Value) -> Result<Value, CalcError> {
    if let Value::Rat(e) = b {
        if e.is_int() && e.n.abs() <= 64 {
            if a.is_zero() && e.n <= 0 {
                return Err(CalcError::Math);
            }
            match a {
                Value::Rat(x) => {
                    if let Some(r) = x.powi(e.n) {
                        return Ok(Value::Rat(r));
                    }
                }
                Value::Surd(_) | Value::Pi(_) => {
                    let mut out = Value::int(1);
                    for _ in 0..e.n.unsigned_abs() {
                        out = mul(out, a)?;
                    }
                    return if e.n < 0 { div(Value::int(1), out) } else { Ok(out) };
                }
                Value::Float(_) => {}
            }
        } else if let Value::Rat(x) = a
            && x.n >= 0
            && let Ok(q) = u32::try_from(e.d)
            && let (Some(n), Some(d)) = (iroot(x.n, q), iroot(x.d, q))
            && let Some(root) = Rat::new(n, d)
            && let Some(r) = root.powi(e.n)
        {
            return Ok(Value::Rat(r));
        }
    }
    let (x, y) = (a.to_f64(), b.to_f64());
    if x == 0.0 && y <= 0.0 {
        return Err(CalcError::Math);
    }
    let out = x.powf(y);
    if out.is_nan() { Err(CalcError::Math) } else { float(out) }
}

/// `index`th root of `radicand`. An odd index takes the root of a negative number.
fn root(index: Value, radicand: Value) -> Result<Value, CalcError> {
    if index.is_zero() {
        return Err(CalcError::Math);
    }
    let reciprocal = div(Value::int(1), index)?;
    if let (Value::Rat(i), true) = (index, radicand.to_f64() < 0.0)
        && i.is_int()
        && i.n % 2 != 0
    {
        return neg(pow(neg(radicand)?, reciprocal)?);
    }
    if index == Value::int(2) {
        return sqrt(radicand);
    }
    pow(radicand, reciprocal)
}

/// Round a float that is only off by rounding noise to the whole number or small fraction it is.
pub(crate) fn snap(x: f64, max_den: i128) -> Option<Rat> {
    if !x.is_finite() || x.abs() > 1.0e12 {
        return None;
    }
    (1..=max_den).find_map(|d| {
        let n = (x * d as f64).round();
        ((n / d as f64 - x).abs() < 1.0e-11 * x.abs().max(1.0)).then(|| Rat::new(n as i128, d)).flatten()
    })
}

/// The exact form of a trig answer: `0.5` is `1/2`, `0.8660254…` is `√3/2`.
fn snap_trig(x: f64) -> Value {
    if let Some(r) = snap(x, 100) {
        return Value::Rat(r);
    }
    match snap(x * x, 100) {
        Some(sq) if sq.n > 0 => sqrt_rat(sq).and_then(|v| if x < 0.0 { neg(v) } else { Ok(v) }).unwrap_or(Value::Float(x)),
        _ => Value::Float(x),
    }
}

/// An angle answer: a whole or simple fraction of degrees, or of π in radians.
fn snap_angle(x: f64, unit: Angle) -> Value {
    if unit == Angle::Radian {
        return match snap(x / std::f64::consts::PI, 12) {
            Some(r) if r.n == 0 => Value::int(0),
            Some(r) => Value::Pi(r),
            None => Value::Float(x),
        };
    }
    snap(x, 100).map_or(Value::Float(x), Value::Rat)
}

fn apply(f: Func, v: Value, unit: Angle) -> Result<Value, CalcError> {
    let x = v.to_f64();
    let radians = unit.to_radians(x);
    let angle = |r: f64| unit.from_radians(r);
    match f {
        Func::Sinh => float(x.sinh()),
        Func::Cosh => float(x.cosh()),
        Func::Tanh => float(x.tanh()),
        Func::ASinh => float(x.asinh()),
        Func::ACosh if x < 1.0 => Err(CalcError::Math),
        Func::ACosh => float(x.acosh()),
        Func::ATanh if x.abs() >= 1.0 => Err(CalcError::Math),
        Func::ATanh => float(x.atanh()),
        // Handled by the parser, which knows the definitions and reads the second argument.
        Func::F
        | Func::G
        | Func::RanInt
        | Func::Gcd
        | Func::Lcm
        | Func::Pol
        | Func::Rec
        | Func::Rnd
        | Func::Arg
        | Func::Conj
        | Func::Re
        | Func::Im
        | Func::Not
        | Func::NegB
        | Func::SheetSum
        | Func::SheetMean
        | Func::SheetMin
        | Func::SheetMax => Err(CalcError::Syntax),
        Func::Sin => Ok(snap_trig(radians.sin())),
        Func::Cos => Ok(snap_trig(radians.cos())),
        Func::Tan => {
            if radians.cos().abs() < 1.0e-12 {
                return Err(CalcError::Math);
            }
            Ok(snap_trig(radians.tan()))
        }
        Func::ASin | Func::ACos if x.abs() > 1.0 => Err(CalcError::Math),
        Func::ASin => Ok(snap_angle(angle(x.asin()), unit)),
        Func::ACos => Ok(snap_angle(angle(x.acos()), unit)),
        Func::ATan => Ok(snap_angle(angle(x.atan()), unit)),
        Func::Log | Func::Ln if x <= 0.0 => Err(CalcError::Math),
        Func::Log => Ok(snap(x.log10(), 1).map_or(Value::Float(x.log10()), Value::Rat)),
        Func::Ln => Ok(snap(x.ln(), 1).map_or(Value::Float(x.ln()), Value::Rat)),
        Func::Abs => match v {
            Value::Rat(r) => from_rat(Rat::new(r.n.abs(), r.d), r.to_f64().abs()),
            Value::Float(f) => Ok(Value::Float(f.abs())),
            other if other.to_f64() < 0.0 => neg(other),
            other => Ok(other),
        },
    }
}

fn whole(v: Value) -> Result<i128, CalcError> {
    match v {
        Value::Rat(r) if r.is_int() && r.n >= 0 => Ok(r.n),
        _ => Err(CalcError::Math),
    }
}

fn factorial(v: Value) -> Result<Value, CalcError> {
    let n = whole(v)?;
    if n > 69 {
        return Err(CalcError::Math);
    }
    let mut out = Some(1i128);
    for k in 2..=n {
        out = out.and_then(|o| o.checked_mul(k));
    }
    out.map_or_else(|| float((2..=n).map(|k| k as f64).product()), |o| Ok(Value::int(o)))
}

fn combine(n: Value, r: Value, ordered: bool) -> Result<Value, CalcError> {
    let (n, r) = (whole(n)?, whole(r)?);
    if r > n {
        return Err(CalcError::Math);
    }
    let r = if ordered { r } else { r.min(n - r) };
    let mut out = Some(1i128);
    for k in 0..r {
        out = out.and_then(|o| o.checked_mul(n - k));
        if !ordered {
            out = out.and_then(|o| o.checked_div(k + 1));
        }
    }
    out.map_or_else(|| float((0..r).map(|k| (n - k) as f64).product()), |o| Ok(Value::int(o)))
}

// ---- Reading an expression -----------------------------------------------------------------

struct Parser<'a> {
    toks: &'a [Tok],
    i: usize,
    env: Env<'a>,
    /// How deep f and g are calling each other.
    depth: u8,
    /// Pol( and Rec( give two answers: (first, second, polar).
    pair: Option<(Value, Value, bool)>,
}

fn starts_factor(t: Tok) -> bool {
    matches!(
        t,
        Tok::Digit(_)
            | Tok::Dot
            | Tok::LParen
            | Tok::Pi
            | Tok::E
            | Tok::X
            | Tok::Var(_)
            | Tok::Ans
            | Tok::Func(_)
            | Tok::SqrtOpen
            | Tok::RootOpen
            | Tok::FracOpen
            | Tok::LogOpen
            | Tok::MixOpen
            | Tok::Ran
            | Tok::Const(_)
            | Tok::DerivOpen
            | Tok::IntOpen
            | Tok::SumOpen
    )
}

impl Parser<'_> {
    /// A cell reference: a column letter (`A` to `E`, typed as the variable keys) and a row number.
    fn cell_ref(&mut self) -> Result<(u8, u32), CalcError> {
        let Some(Tok::Var(col)) = self.peek() else { return Err(CalcError::Syntax) };
        self.i += 1;
        let mut row = 0u32;
        let mut any = false;
        while let Some(Tok::Digit(d)) = self.peek() {
            row = row.saturating_mul(10).saturating_add(u32::from(d));
            any = true;
            self.i += 1;
        }
        if any && (1..=45).contains(&row) { Ok((col, row)) } else { Err(CalcError::Syntax) }
    }

    /// `f` with `x` standing for `x`.
    fn at(&self, f: &[Tok], x: f64) -> Result<f64, CalcError> {
        let env = Env { x: Some(Value::Float(x)), ..self.env };
        let mut p = Parser { toks: f, i: 0, env, depth: self.depth + 1, pair: None };
        let v = p.expr()?;
        if p.i == f.len() { Ok(v.to_f64()) } else { Err(CalcError::Syntax) }
    }

    fn slot_value(&self, slot: &[Tok]) -> Result<Value, CalcError> {
        let mut p = Parser { toks: slot, i: 0, env: self.env, depth: self.depth + 1, pair: None };
        let v = p.expr()?;
        if p.i == slot.len() { Ok(v) } else { Err(CalcError::Syntax) }
    }

    /// d/dx, ∫ and Σ: their slots are read straight from the tokens, so the function can be
    /// evaluated again and again.
    fn calculus(&mut self) -> Result<Value, CalcError> {
        let open = self.i;
        let kind = self.peek().ok_or(CalcError::Syntax)?;
        let (mids, close) = template_end(self.toks, open);
        let close = close.min(self.toks.len());
        self.i = (close + 1).min(self.toks.len());
        let mut starts = vec![open + 1];
        starts.extend(mids.iter().map(|m| m + 1));
        let mut ends = mids.clone();
        ends.push(close);
        let toks = self.toks;
        let slot = |k: usize| -> Result<&[Tok], CalcError> {
            match (starts.get(k), ends.get(k)) {
                (Some(&a), Some(&b)) if a <= b => Ok(&toks[a..b]),
                _ => Err(CalcError::Syntax),
            }
        };
        let f = slot(0)?;
        if f.is_empty() {
            return Err(CalcError::Syntax);
        }
        match kind {
            Tok::DerivOpen => {
                let a = self.slot_value(slot(1)?)?.to_f64();
                let h = 1.0e-3 * a.abs().max(1.0);
                let d = (-self.at(f, a + 2.0 * h)? + 8.0 * self.at(f, a + h)? - 8.0 * self.at(f, a - h)? + self.at(f, a - 2.0 * h)?) / (12.0 * h);
                if d.is_finite() { Ok(tidy(d)) } else { Err(CalcError::Math) }
            }
            Tok::IntOpen => {
                let (a, b) = (self.slot_value(slot(1)?)?.to_f64(), self.slot_value(slot(2)?)?.to_f64());
                // Gauss–Legendre, five points on each of 200 pieces.
                const X: [f64; 5] = [0.0, -0.538_469_310_105_683, 0.538_469_310_105_683, -0.906_179_845_938_664, 0.906_179_845_938_664];
                const W: [f64; 5] =
                    [0.568_888_888_888_889, 0.478_628_670_499_366, 0.478_628_670_499_366, 0.236_926_885_056_189, 0.236_926_885_056_189];
                let n = 200;
                let step = (b - a) / n as f64;
                let mut total = 0.0;
                for k in 0..n {
                    let mid = a + (k as f64 + 0.5) * step;
                    for (x, w) in X.iter().zip(W) {
                        total += w * self.at(f, mid + x * step / 2.0)?;
                    }
                }
                let r = total * step / 2.0;
                if r.is_finite() { Ok(tidy(r)) } else { Err(CalcError::Math) }
            }
            _ => {
                let (a, b) = (self.slot_value(slot(1)?)?, self.slot_value(slot(2)?)?);
                let (a, b) = match (a, b) {
                    (Value::Rat(a), Value::Rat(b)) if a.is_int() && b.is_int() && b.n >= a.n && b.n - a.n <= 100_000 => (a.n, b.n),
                    _ => return Err(CalcError::Math),
                };
                let mut total = Value::int(0);
                for k in a..=b {
                    let env = Env { x: Some(Value::int(k)), ..self.env };
                    let mut p = Parser { toks: f, i: 0, env, depth: self.depth + 1, pair: None };
                    total = add(total, p.expr()?)?;
                }
                Ok(total)
            }
        }
    }

    fn two(&mut self, f: Func, a: Value, b: Value) -> Result<Value, CalcError> {
        let int = |v: Value| match v {
            Value::Rat(r) if r.is_int() => Ok(r.n),
            _ => Err(CalcError::Math),
        };
        match f {
            Func::RanInt => {
                let (lo, hi) = (int(a)?, int(b)?);
                if hi < lo {
                    return Err(CalcError::Math);
                }
                let span = u64::try_from(hi - lo + 1).map_err(|_| CalcError::Math)?;
                Ok(Value::int(lo + i128::from(random() % span)))
            }
            Func::Gcd | Func::Lcm => {
                let (x, y) = (int(a)?.abs(), int(b)?.abs());
                let g = gcd(x, y);
                if f == Func::Gcd { Ok(Value::int(g)) } else { Ok(Value::int((x / g.max(1)).checked_mul(y).ok_or(CalcError::Math)?)) }
            }
            Func::Pol => {
                let r = sqrt(add(mul(a, a)?, mul(b, b)?)?)?;
                let theta = snap_angle(self.env.angle.from_radians(b.to_f64().atan2(a.to_f64())), self.env.angle);
                self.pair = Some((r, theta, true));
                Ok(r)
            }
            _ => {
                let t = self.env.angle.to_radians(b.to_f64());
                let (x, y) = (mul(a, snap_trig(t.cos()))?, mul(a, snap_trig(t.sin()))?);
                self.pair = Some((x, y, false));
                Ok(x)
            }
        }
    }

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

    /// The end of a template or bracket. Leaving it off at the very end of the line is fine.
    fn close(&mut self, t: Tok) -> Result<(), CalcError> {
        if self.eat(t) || self.peek().is_none() { Ok(()) } else { Err(CalcError::Syntax) }
    }

    fn expr(&mut self) -> Result<Value, CalcError> {
        let mut v = self.term()?;
        loop {
            match self.peek() {
                Some(Tok::Plus) => {
                    self.i += 1;
                    v = add(v, self.term()?)?;
                }
                Some(Tok::Minus) => {
                    self.i += 1;
                    v = sub(v, self.term()?)?;
                }
                _ => return Ok(v),
            }
        }
    }

    fn term(&mut self) -> Result<Value, CalcError> {
        let mut v = self.combinations()?;
        loop {
            match self.peek() {
                Some(Tok::Times) => {
                    self.i += 1;
                    v = mul(v, self.combinations()?)?;
                }
                Some(Tok::Divide) => {
                    self.i += 1;
                    v = div(v, self.combinations()?)?;
                }
                _ => return Ok(v),
            }
        }
    }

    fn combinations(&mut self) -> Result<Value, CalcError> {
        let mut v = self.implicit()?;
        loop {
            match self.peek() {
                Some(t @ (Tok::Ncr | Tok::Npr)) => {
                    self.i += 1;
                    v = combine(v, self.implicit()?, t == Tok::Npr)?;
                }
                _ => return Ok(v),
            }
        }
    }

    /// `2π`, `3(4+1)`, `2sin(30)`: touching factors multiply, and bind tighter than `÷`.
    fn implicit(&mut self) -> Result<Value, CalcError> {
        let mut v = self.signed()?;
        while self.peek().is_some_and(starts_factor) {
            v = mul(v, self.signed()?)?;
        }
        Ok(v)
    }

    fn signed(&mut self) -> Result<Value, CalcError> {
        if matches!(self.peek(), Some(Tok::Neg | Tok::Minus)) {
            self.i += 1;
            return neg(self.signed()?);
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Value, CalcError> {
        let mut v = self.primary()?;
        loop {
            match self.peek() {
                Some(Tok::SupOpen) => {
                    self.i += 1;
                    let e = self.expr()?;
                    self.close(Tok::Close)?;
                    v = pow(v, e)?;
                }
                Some(Tok::Fact) => {
                    self.i += 1;
                    v = factorial(v)?;
                }
                Some(Tok::Percent) => {
                    self.i += 1;
                    v = div(v, Value::int(100))?;
                }
                Some(Tok::Exp10Open) => {
                    self.i += 1;
                    let e = self.expr()?;
                    self.close(Tok::Close)?;
                    v = mul(v, pow(Value::int(10), e)?)?;
                }
                Some(Tok::Dms) => {
                    self.i += 1;
                    // Minutes and seconds may follow: 12°34°56°.
                    let mut unit = 60;
                    while unit <= 3600 && matches!(self.peek(), Some(Tok::Digit(_) | Tok::Dot)) {
                        let save = self.i;
                        let part = self.number()?;
                        if !self.eat(Tok::Dms) {
                            self.i = save;
                            break;
                        }
                        let step = div(part, Value::int(unit))?;
                        v = if v.to_f64() < 0.0 { sub(v, step)? } else { add(v, step)? };
                        unit *= 60;
                    }
                }
                Some(t @ (Tok::AngD | Tok::AngR | Tok::AngG)) => {
                    self.i += 1;
                    let from = match t {
                        Tok::AngD => Angle::Degree,
                        Tok::AngR => Angle::Radian,
                        _ => Angle::Gradian,
                    };
                    v = convert_angle(v, from, self.env.angle)?;
                }
                Some(Tok::YHat) => {
                    self.i += 1;
                    let reg = self.env.mem.reg.ok_or(CalcError::Math)?;
                    v = tidy(reg.y_hat(v.to_f64()));
                }
                Some(Tok::XHat(k)) => {
                    self.i += 1;
                    let reg = self.env.mem.reg.ok_or(CalcError::Math)?;
                    let xs = reg.x_hat(v.to_f64());
                    v = tidy(*xs.get(usize::from(k.saturating_sub(1))).ok_or(CalcError::Math)?);
                }
                Some(Tok::Conv(c)) => {
                    self.i += 1;
                    let (_, _, factor, offset) = CONVERSIONS.get(usize::from(c)).copied().ok_or(CalcError::Syntax)?;
                    v = tidy(float(v.to_f64() * factor + offset)?.to_f64());
                }

                _ => return Ok(v),
            }
        }
    }

    fn number(&mut self) -> Result<Value, CalcError> {
        let (mut text, mut dots) = (String::new(), 0);
        while let Some(t) = self.peek() {
            match t {
                Tok::Digit(d) => text.push(char::from(b'0' + d)),
                Tok::Dot => {
                    dots += 1;
                    text.push('.');
                }
                _ => break,
            }
            self.i += 1;
        }
        if dots > 1 || text == "." {
            return Err(CalcError::Syntax);
        }
        if self.peek() == Some(Tok::RecurOpen) {
            // `0.12(34)`: the digits in the bracket repeat for ever.
            self.i += 1;
            let mut rep = String::new();
            while let Some(Tok::Digit(d)) = self.peek() {
                rep.push(char::from(b'0' + d));
                self.i += 1;
            }
            self.close(Tok::Close)?;
            if rep.is_empty() {
                return Err(CalcError::Syntax);
            }
            let (whole, frac) = text.split_once('.').unwrap_or((&text, ""));
            let (k, m) = (u32::try_from(frac.len()).map_err(|_| CalcError::Syntax)?, u32::try_from(rep.len()).map_err(|_| CalcError::Syntax)?);
            let pow = |e: u32| 10i128.checked_pow(e).ok_or(CalcError::Math);
            let head: i128 = format!("{whole}{frac}").parse().map_err(|_| CalcError::Syntax)?;
            let tail: i128 = rep.parse().map_err(|_| CalcError::Syntax)?;
            // head/10^k + tail / (10^k (10^m − 1))
            let den = pow(k)?.checked_mul(pow(m)? - 1).ok_or(CalcError::Math)?;
            let num = head.checked_mul(pow(m)? - 1).and_then(|h| h.checked_add(tail)).ok_or(CalcError::Math)?;
            return Rat::new(num, den).map(Value::Rat).ok_or(CalcError::Math);
        }
        let (whole, frac) = text.split_once('.').unwrap_or((&text, ""));
        let digits = format!("{whole}{frac}");
        let exact = u32::try_from(frac.len())
            .ok()
            .and_then(|k| Some((digits.parse::<i128>().ok()?, 10i128.checked_pow(k)?)))
            .and_then(|(n, d)| Rat::new(n, d));
        exact.map_or_else(|| text.parse::<f64>().map_err(|_| CalcError::Syntax).and_then(float), |r| Ok(Value::Rat(r)))
    }

    fn primary(&mut self) -> Result<Value, CalcError> {
        match self.peek().ok_or(CalcError::Syntax)? {
            Tok::Digit(_) | Tok::Dot => self.number(),
            Tok::Pi => {
                self.i += 1;
                Ok(Value::Pi(Rat::int(1)))
            }
            Tok::E => {
                self.i += 1;
                Ok(Value::Float(std::f64::consts::E))
            }
            Tok::Ans => {
                self.i += 1;
                Ok(self.env.ans)
            }
            Tok::X => {
                self.i += 1;
                Ok(self.env.x.unwrap_or(self.env.mem.vars[6]))
            }
            Tok::Var(i) if self.env.cells.is_some() && i < 5 && matches!(self.toks.get(self.i + 1), Some(Tok::Digit(_))) => {
                let (col, row) = self.cell_ref()?;
                (self.env.cells.ok_or(CalcError::Syntax)?)(col, row)
            }
            Tok::Func(f @ (Func::SheetSum | Func::SheetMean | Func::SheetMin | Func::SheetMax)) if self.env.cells.is_some() => {
                self.i += 1;
                let (c1, r1) = self.cell_ref()?;
                let (c2, r2) = if self.eat(Tok::Colon) { self.cell_ref()? } else { (c1, r1) };
                self.close(Tok::RParen)?;
                let cells = self.env.cells.ok_or(CalcError::Syntax)?;
                let mut vals = vec![];
                for col in c1.min(c2)..=c1.max(c2) {
                    for row in r1.min(r2)..=r1.max(r2) {
                        vals.push(cells(col, row)?);
                    }
                }
                match f {
                    Func::SheetSum => vals.into_iter().try_fold(Value::int(0), add),
                    Func::SheetMean => {
                        let n = i128::try_from(vals.len()).map_err(|_| CalcError::Math)?;
                        div(vals.into_iter().try_fold(Value::int(0), add)?, Value::int(n))
                    }
                    Func::SheetMin => vals.into_iter().min_by(|a, b| a.to_f64().total_cmp(&b.to_f64())).ok_or(CalcError::Math),
                    _ => vals.into_iter().max_by(|a, b| a.to_f64().total_cmp(&b.to_f64())).ok_or(CalcError::Math),
                }
            }
            Tok::Var(i) => {
                self.i += 1;
                self.env.mem.vars.get(usize::from(i)).copied().ok_or(CalcError::Syntax)
            }
            Tok::LParen => {
                self.i += 1;
                let v = self.expr()?;
                self.close(Tok::RParen)?;
                Ok(v)
            }
            // `×10▭` with nothing before it is 1 × 10▭; postfix does the rest.
            Tok::Exp10Open => Ok(Value::int(1)),
            Tok::Ran => {
                self.i += 1;
                Ok(Value::Rat(Rat::new(i128::from((random() % 1000) as u16), 1000).unwrap_or(Rat::int(0))))
            }
            Tok::Stat(i) => {
                self.i += 1;
                let x = self.env.mem.stats.get(usize::from(i)).copied().ok_or(CalcError::Syntax)?;
                if x.is_nan() { Err(CalcError::Math) } else { Ok(tidy(x)) }
            }
            Tok::Const(c) => {
                self.i += 1;
                CONSTANTS.get(usize::from(c)).map(|k| Value::Float(k.2)).ok_or(CalcError::Syntax)
            }
            Tok::I => Err(CalcError::Math),
            Tok::Func(f) if f.two_args() => {
                self.i += 1;
                let a = self.expr()?;
                if !self.eat(Tok::Comma) {
                    return Err(CalcError::Syntax);
                }
                let b = self.expr()?;
                self.close(Tok::RParen)?;
                self.two(f, a, b)
            }
            Tok::Func(Func::Rnd) => {
                self.i += 1;
                let v = self.expr()?;
                self.close(Tok::RParen)?;
                round_off(v, self.env.num)
            }
            Tok::DerivOpen | Tok::IntOpen | Tok::SumOpen => self.calculus(),
            Tok::Func(f @ (Func::F | Func::G)) => {
                self.i += 1;
                let v = self.expr()?;
                self.close(Tok::RParen)?;
                let def = &self.env.mem.defs[usize::from(f == Func::G)];
                if def.is_empty() || self.depth >= 6 {
                    return Err(CalcError::Syntax);
                }
                let env = Env { x: Some(v), ..self.env };
                let mut inner = Parser { toks: def, i: 0, env, depth: self.depth + 1, pair: None };
                let out = inner.expr()?;
                if inner.i == def.len() { Ok(out) } else { Err(CalcError::Syntax) }
            }
            Tok::Func(f) => {
                self.i += 1;
                let v = self.expr()?;
                self.close(Tok::RParen)?;
                apply(f, v, self.env.angle)
            }
            Tok::LogOpen => {
                self.i += 1;
                let base = self.expr()?;
                if !self.eat(Tok::LogMid) {
                    return Err(CalcError::Syntax);
                }
                let number = self.expr()?;
                self.close(Tok::Close)?;
                let (b, n) = (base.to_f64(), number.to_f64());
                if b <= 0.0 || b == 1.0 || n <= 0.0 {
                    return Err(CalcError::Math);
                }
                let r = n.ln() / b.ln();
                Ok(snap(r, 1).map_or(Value::Float(r), Value::Rat))
            }
            Tok::MixOpen => {
                self.i += 1;
                let whole = self.expr()?;
                if !self.eat(Tok::FracMid) {
                    return Err(CalcError::Syntax);
                }
                let top = self.expr()?;
                if !self.eat(Tok::FracMid) {
                    return Err(CalcError::Syntax);
                }
                let bottom = self.expr()?;
                self.close(Tok::Close)?;
                let part = div(top, bottom)?;
                if whole.to_f64() < 0.0 { sub(whole, part) } else { add(whole, part) }
            }
            Tok::SqrtOpen => {
                self.i += 1;
                let v = self.expr()?;
                self.close(Tok::Close)?;
                sqrt(v)
            }
            Tok::RootOpen => {
                self.i += 1;
                let index = self.expr()?;
                if !self.eat(Tok::RootMid) {
                    return Err(CalcError::Syntax);
                }
                let radicand = self.expr()?;
                self.close(Tok::Close)?;
                root(index, radicand)
            }
            Tok::FracOpen => {
                self.i += 1;
                let top = self.expr()?;
                if !self.eat(Tok::FracMid) {
                    return Err(CalcError::Syntax);
                }
                let bottom = self.expr()?;
                self.close(Tok::Close)?;
                div(top, bottom)
            }
            _ => Err(CalcError::Syntax),
        }
    }
}

/// The value of `toks`, with `Ans` standing for `ans`.
pub fn evaluate(toks: &[Tok], ans: Value, degrees: bool) -> Result<Value, CalcError> {
    evaluate_with(toks, ans, degrees, None)
}

/// As `evaluate`, with the variable `x` standing for a value (a table row, a root being checked).
pub fn evaluate_with(toks: &[Tok], ans: Value, degrees: bool, x: Option<Value>) -> Result<Value, CalcError> {
    let mem = Memory::default();
    evaluate_env(toks, &Env { ans, angle: if degrees { Angle::Degree } else { Angle::Radian }, x, mem: &mem, num: NumFmt::Norm(2), cells: None })
}

/// Evaluate with everything the sum can refer to.
pub fn evaluate_env(toks: &[Tok], env: &Env) -> Result<Value, CalcError> {
    evaluate_full(toks, env).map(|(v, _)| v)
}

/// As `evaluate_env`, with the second answer of Pol( or Rec( if there is one.
pub fn evaluate_full(toks: &[Tok], env: &Env) -> Result<(Value, Option<(Value, Value, bool)>), CalcError> {
    let mut p = Parser { toks, i: 0, env: *env, depth: 0, pair: None };
    let v = p.expr()?;
    if p.i == toks.len() { Ok((v, p.pair)) } else { Err(CalcError::Syntax) }
}

/// A cheap random number generator (xorshift), seeded from the clock: for Ran# and RanInt#.
fn random() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut x = STATE.load(Ordering::Relaxed);
    if x == 0 {
        x = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64) | 1;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    STATE.store(x, Ordering::Relaxed);
    x
}

/// `v`, given in `from`, in `to`. Exact where it can be: 90° is π/2 radians.
pub fn convert_angle(v: Value, from: Angle, to: Angle) -> Result<Value, CalcError> {
    match (from, to) {
        _ if from == to => Ok(v),
        (Angle::Degree, Angle::Radian) => mul(v, Value::Pi(Rat { n: 1, d: 180 })),
        (Angle::Gradian, Angle::Radian) => mul(v, Value::Pi(Rat { n: 1, d: 200 })),
        (Angle::Radian, _) => {
            let full = if to == Angle::Degree { 180 } else { 200 };
            match v {
                Value::Pi(k) => from_rat(k.mul(Rat::int(full)), v.to_f64() * full as f64 / std::f64::consts::PI),
                _ => float(v.to_f64() * full as f64 / std::f64::consts::PI),
            }
        }
        (Angle::Degree, _) => mul(v, Value::Rat(Rat { n: 10, d: 9 })),
        (Angle::Gradian, _) => mul(v, Value::Rat(Rat { n: 9, d: 10 })),
    }
}

/// Round off as the screen would show it (Rnd): to the Fix places, the Sci figures, or ten figures.
fn round_off(v: Value, num: NumFmt) -> Result<Value, CalcError> {
    let x = v.to_f64();
    let text = match num {
        NumFmt::Fix(n) => format!("{x:.*}", usize::from(n)),
        NumFmt::Sci(n) => format!("{x:.*e}", usize::from(n.max(1)) - 1),
        NumFmt::Norm(_) => format!("{x:.9e}"),
    };
    let r: f64 = text.parse().map_err(|_| CalcError::Math)?;
    Ok(snap(r, 1000).map_or(Value::Float(r), Value::Rat))
}

/// Round noise off a numerical answer (derivative, integral) when it is a simple number.
pub(crate) fn tidy(x: f64) -> Value {
    if x.abs() < 1.0e-10 {
        return Value::int(0);
    }
    (1..=1000)
        .find_map(|d| {
            let n = (x * d as f64).round();
            ((n / d as f64 - x).abs() < 1.0e-9 * x.abs().max(1.0)).then(|| Rat::new(n as i128, d)).flatten()
        })
        .map_or(Value::Float(x), Value::Rat)
}

// ---- Showing an answer ---------------------------------------------------------------------

fn digits(n: u128, out: &mut Vec<Tok>) {
    let text = n.to_string();
    out.extend(text.bytes().map(|b| Tok::Digit(b - b'0')));
}

fn push_text(text: &str, out: &mut Vec<Tok>) {
    out.extend(text.chars().filter_map(|c| match c {
        '.' => Some(Tok::Dot),
        d if d.is_ascii_digit() => Some(Tok::Digit(d as u8 - b'0')),
        _ => None,
    }));
}

/// `mantissa × 10^exp`, the mantissa to `decimals` places (trailing zeros dropped when `trim`).
fn sci_tokens(a: f64, decimals: usize, trim: bool, out: &mut Vec<Tok>) {
    let sci = format!("{a:.decimals$e}");
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let mantissa = if trim && mantissa.contains('.') { mantissa.trim_end_matches('0').trim_end_matches('.') } else { mantissa };
    push_text(mantissa, out);
    out.extend([Tok::Times, Tok::Digit(1), Tok::Digit(0), Tok::SupOpen]);
    if exp < 0 {
        out.push(Tok::Minus);
    }
    digits(u128::from(exp.unsigned_abs()), out);
    out.push(Tok::Close);
}

/// A decimal number as the screen shows it under the chosen Number Format.
fn float_tokens(x: f64, fmt: NumFmt, out: &mut Vec<Tok>) {
    if x == 0.0 {
        out.push(Tok::Digit(0));
        return;
    }
    if x < 0.0 {
        out.push(Tok::Minus);
    }
    let a = x.abs();
    let exp = format!("{a:.9e}").split_once('e').and_then(|(_, e)| e.parse::<i32>().ok()).unwrap_or(0);
    match fmt {
        NumFmt::Sci(n) => sci_tokens(a, usize::from(n.clamp(1, 10)) - 1, false, out),
        NumFmt::Fix(n) if exp < 10 => push_text(&format!("{a:.*}", usize::from(n.min(9))), out),
        NumFmt::Fix(_) => sci_tokens(a, 9, true, out),
        NumFmt::Norm(n) => {
            let low = if n == 1 { -2 } else { -9 };
            if (low..10).contains(&exp) {
                // Ten significant figures, as a calculator's screen holds, without trailing zeros.
                let sci = format!("{a:.9e}");
                let mantissa = sci.split_once('e').map_or("", |(m, _)| m).trim_end_matches('0').trim_end_matches('.');
                let all: String = mantissa.chars().filter(char::is_ascii_digit).collect();
                let point = exp + 1;
                let text = if point <= 0 {
                    format!("0.{}{all}", "0".repeat(usize::try_from(-point).unwrap_or(0)))
                } else {
                    let point = usize::try_from(point).unwrap_or(0);
                    if all.len() <= point {
                        format!("{all}{}", "0".repeat(point - all.len()))
                    } else {
                        format!("{}.{}", &all[..point], &all[point..])
                    }
                };
                push_text(&text, out);
            } else {
                sci_tokens(a, 9, true, out);
            }
        }
    }
}

fn root_tokens(coef_num: i128, rad: i128, out: &mut Vec<Tok>) {
    if coef_num != 1 {
        digits(coef_num.unsigned_abs(), out);
    }
    out.push(Tok::SqrtOpen);
    digits(rad.unsigned_abs(), out);
    out.push(Tok::Close);
}

/// An integer as a product of prime powers: `2²×3`.
fn prime_tokens(n: i128) -> Option<Vec<Tok>> {
    let m = n.unsigned_abs();
    if !(2..=1_000_000_000_000).contains(&m) {
        return None;
    }
    let mut out = vec![];
    if n < 0 {
        out.push(Tok::Minus);
    }
    let (mut rest, mut f, mut first) = (m, 2u128, true);
    while rest > 1 {
        let mut e = 0;
        while rest % f == 0 {
            rest /= f;
            e += 1;
        }
        if e > 0 {
            if !first {
                out.push(Tok::Times);
            }
            first = false;
            digits(f, &mut out);
            if e > 1 {
                out.push(Tok::SupOpen);
                digits(e, &mut out);
                out.push(Tok::Close);
            }
        }
        f += if f == 2 { 1 } else { 2 };
        if f * f > rest && rest > 1 {
            if !first {
                out.push(Tok::Times);
            }
            digits(rest, &mut out);
            break;
        }
    }
    Some(out)
}

/// Fractions on one line: `1/2` rather than a stacked fraction.
fn linearise(toks: Vec<Tok>) -> Vec<Tok> {
    let (mut out, mut stack) = (vec![], vec![]);
    for t in toks {
        match t {
            Tok::FracOpen | Tok::MixOpen => stack.push(true),
            t if t.opens() => {
                stack.push(false);
                out.push(t);
            }
            Tok::FracMid if stack.last() == Some(&true) => out.push(Tok::Slash),
            Tok::Close => {
                if stack.pop() != Some(true) {
                    out.push(Tok::Close);
                }
            }
            t => out.push(t),
        }
    }
    out
}

fn count_digits(n: u128) -> usize {
    n.to_string().len()
}

/// How many digits a fraction takes as a mixed number, counting the bars: more than ten and the
/// calculator shows a decimal instead (whichever way the fraction is shown).
fn fraction_digits(r: Rat) -> usize {
    let (n, d) = (r.n.unsigned_abs(), r.d.unsigned_abs());
    if n > d { count_digits(n / d) + count_digits(n % d) + count_digits(d) + 2 } else { count_digits(n) + count_digits(d) + 1 }
}

/// One part of a √-form answer: `a√b`, `a√b/c`, a whole number or a fraction (no sign).
fn surd_term(c: Rat, rad: i128, out: &mut Vec<Tok>) {
    let body = |out: &mut Vec<Tok>| {
        if rad == 1 {
            digits(c.n.unsigned_abs(), out);
        } else {
            root_tokens(i128::try_from(c.n.unsigned_abs()).unwrap_or(1), rad, out);
        }
    };
    if c.d == 1 {
        body(out);
    } else {
        out.push(Tok::FracOpen);
        body(out);
        out.push(Tok::FracMid);
        digits(c.d.unsigned_abs(), out);
        out.push(Tok::Close);
    }
}

/// A √-form answer, if it is inside the ranges the calculator shows that way (a < 100, b < 1000,
/// c < 100, and at most two parts).
fn surd_tokens(s: &Surds) -> Option<Vec<Tok>> {
    let mut parts: Vec<(Rat, i128)> = s.roots().to_vec();
    if s.rat.n != 0 {
        if parts.len() >= 2 {
            return None;
        }
        // A whole number goes first (d ± a√b); a fraction goes after the root.
        if s.rat.is_int() { parts.insert(0, (s.rat, 1)) } else { parts.push((s.rat, 1)) }
    }
    if parts.iter().any(|(c, r)| c.n.unsigned_abs() >= 100 || c.d >= 100 || *r >= 1000) {
        return None;
    }
    let mut out = vec![];
    for (i, (c, r)) in parts.iter().enumerate() {
        if c.n < 0 {
            out.push(Tok::Minus);
        } else if i > 0 {
            out.push(Tok::Plus);
        }
        surd_term(*c, *r, &mut out);
    }
    Some(out)
}

fn natural_tokens(v: Value, decimal: bool, cfg: &Config) -> Vec<Tok> {
    let mut out = vec![];
    let v = match v {
        Value::Float(f) if !decimal => snap(f, 1000).filter(|r| !r.is_int()).map_or(v, Value::Rat),
        _ => v,
    };
    match v {
        Value::Rat(r) if r.is_int() || decimal => {
            // A whole number keeps all its digits under Norm (and when exact answers are asked for).
            if r.is_int() && r.n.unsigned_abs() < 10_000_000_000 && (!decimal || matches!(cfg.num, NumFmt::Norm(_))) {
                if r.n < 0 {
                    out.push(Tok::Minus);
                }
                digits(r.n.unsigned_abs(), &mut out);
            } else {
                float_tokens(r.to_f64(), cfg.num, &mut out);
            }
        }
        // A fraction needing more than ten digits (counting the bar) is shown as a decimal.
        Value::Rat(r) if fraction_digits(r) > 10 => float_tokens(r.to_f64(), cfg.num, &mut out),
        Value::Rat(r) => {
            if r.n < 0 {
                out.push(Tok::Minus);
            }
            let (whole, top) = (r.n.unsigned_abs() / r.d.unsigned_abs(), r.n.unsigned_abs() % r.d.unsigned_abs());
            if cfg.mixed && whole > 0 {
                digits(whole, &mut out);
            }
            out.push(Tok::FracOpen);
            digits(if cfg.mixed && whole > 0 { top } else { r.n.unsigned_abs() }, &mut out);
            out.push(Tok::FracMid);
            digits(r.d.unsigned_abs(), &mut out);
            out.push(Tok::Close);
        }
        Value::Surd(_) | Value::Pi(_) if decimal => float_tokens(v.to_f64(), cfg.num, &mut out),
        Value::Surd(sd) => match surd_tokens(&sd) {
            Some(t) => out = t,
            // Outside the ranges the screen shows in √ form: a decimal, as the real one does.
            None => float_tokens(v.to_f64(), cfg.num, &mut out),
        },
        Value::Pi(k) if k.n.unsigned_abs() < 1000 && k.d < 1000 => {
            if k.n < 0 {
                out.push(Tok::Minus);
            }
            if k.d != 1 {
                out.push(Tok::FracOpen);
                digits(k.n.unsigned_abs(), &mut out);
                out.push(Tok::FracMid);
                digits(k.d.unsigned_abs(), &mut out);
                out.push(Tok::Close);
            } else if k.n.unsigned_abs() != 1 {
                digits(k.n.unsigned_abs(), &mut out);
            }
            out.push(Tok::Pi);
        }
        Value::Pi(_) => float_tokens(v.to_f64(), cfg.num, &mut out),
        Value::Float(f) => float_tokens(f, cfg.num, &mut out),
    }
    out
}

/// The answer as tokens, so the screen can draw it with the same typesetting as the input.
pub fn result_tokens(v: Value, format: ResultFormat, cfg: &Config) -> Vec<Tok> {
    let format = if format == ResultFormat::Standard && cfg.io.decimal_output() { ResultFormat::Decimal } else { format };
    let out = match format {
        ResultFormat::Standard => natural_tokens(v, false, cfg),
        ResultFormat::Decimal => natural_tokens(v, true, cfg),
        ResultFormat::ImproperFraction => natural_tokens(v, false, &Config { mixed: false, ..*cfg }),
        ResultFormat::MixedNumber => natural_tokens(v, false, &Config { mixed: true, ..*cfg }),
        ResultFormat::StandardForm => {
            let mut o = vec![];
            let x = v.to_f64();
            if x < 0.0 {
                o.push(Tok::Minus);
            }
            if x == 0.0 {
                o.push(Tok::Digit(0))
            } else {
                sci_tokens(x.abs(), 9, true, &mut o)
            }
            o
        }
        ResultFormat::PrimeFactor => match v {
            Value::Rat(r) if r.is_int() => prime_tokens(r.n).unwrap_or_else(|| natural_tokens(v, false, cfg)),
            _ => natural_tokens(v, false, cfg),
        },
        ResultFormat::Sexagesimal => dms_tokens(v.to_f64()),
        ResultFormat::Recurring => match v {
            Value::Rat(r) => recurring_tokens(r).unwrap_or_else(|| natural_tokens(v, true, cfg)),
            _ => natural_tokens(v, true, cfg),
        },
    };
    let out = if cfg.io.line_output() { linearise(out) } else { out };
    decorate(out, cfg)
}

/// The decimal expansion of a fraction with its repeating digits marked, if it fits in ten digits.
fn recurring_tokens(r: Rat) -> Option<Vec<Tok>> {
    if r.is_int() {
        return None;
    }
    let (n, d) = (r.n.unsigned_abs(), r.d.unsigned_abs());
    let mut out = vec![];
    if r.n < 0 {
        out.push(Tok::Minus);
    }
    digits(n / d, &mut out);
    out.push(Tok::Dot);
    let mut rem = n % d;
    let mut seen: std::collections::HashMap<u128, usize> = std::collections::HashMap::new();
    let mut frac: Vec<u8> = vec![];
    while rem != 0 && !seen.contains_key(&rem) && frac.len() < 40 {
        seen.insert(rem, frac.len());
        rem *= 10;
        frac.push((rem / d) as u8);
        rem %= d;
    }
    let start = if rem == 0 { None } else { seen.get(&rem).copied() };
    if frac.len() > 10 || (rem != 0 && start.is_none()) {
        return None;
    }
    for (i, dgt) in frac.iter().enumerate() {
        if Some(i) == start {
            out.push(Tok::RecurOpen);
        }
        out.push(Tok::Digit(*dgt));
    }
    if start.is_some() {
        out.push(Tok::Close);
    }
    Some(out)
}

/// `12°34′56.7″`: whole degrees, whole minutes and seconds to two places.
fn dms_tokens(x: f64) -> Vec<Tok> {
    let mut out = vec![];
    if x < 0.0 {
        out.push(Tok::Minus);
    }
    let total = (x.abs() * 360_000.0).round() / 100.0;
    let deg = (total / 3600.0).floor();
    let min = ((total - deg * 3600.0) / 60.0).floor();
    let sec = total - deg * 3600.0 - min * 60.0;
    digits(deg as u128, &mut out);
    out.push(Tok::Dms);
    digits(min as u128, &mut out);
    out.push(Tok::MinMark);
    push_text(format!("{sec:.2}").trim_end_matches('0').trim_end_matches('.'), &mut out);
    out.push(Tok::SecMark);
    out
}

/// The Digit Separator (a comma every three digits of a whole part) and the Decimal Mark.
fn decorate(toks: Vec<Tok>, cfg: &Config) -> Vec<Tok> {
    let mut out = Vec::with_capacity(toks.len() + 4);
    let mut i = 0;
    let mut depth = 0usize;
    let mut after_point = false;
    while i < toks.len() {
        let t = toks[i];
        match t {
            Tok::SupOpen | Tok::Exp10Open => depth += 1,
            Tok::Close => depth = depth.saturating_sub(1),
            _ => {}
        }
        if let Tok::Digit(_) = t {
            let run = toks[i..].iter().take_while(|t| matches!(t, Tok::Digit(_))).count();
            let group = cfg.separator && depth == 0 && !after_point && run > 3;
            for (k, d) in toks[i..i + run].iter().enumerate() {
                if group && k > 0 && (run - k) % 3 == 0 {
                    out.push(Tok::Sep);
                }
                out.push(*d);
            }
            i += run;
            continue;
        }
        after_point = t == Tok::Dot;
        out.push(if t == Tok::Dot && cfg.comma_mark { Tok::DecComma } else { t });
        i += 1;
    }
    out
}

/// The answer under the default settings, exact or decimal.
pub fn value_tokens(v: Value, decimal: bool) -> Vec<Tok> {
    result_tokens(v, if decimal { ResultFormat::Decimal } else { ResultFormat::Standard }, &Config::default())
}

// ---- Tables and equations ------------------------------------------------------------------

/// `f(x)` for each `x` from `start` to `end` in steps of `step` (at most 45 rows, as on the real thing).
pub fn table_rows(
    f: &[Tok],
    start: Value,
    end: Value,
    step: Value,
    angle: Angle,
    mem: &Memory,
) -> Result<Vec<(Value, Result<Value, CalcError>)>, CalcError> {
    if step.is_zero() {
        return Err(CalcError::Math);
    }
    let count = div(sub(end, start)?, step)?.to_f64().floor();
    if !(0.0..=44.0).contains(&count) {
        return Err(CalcError::Math);
    }
    let mut rows = vec![];
    for k in 0..=(count as i128) {
        let x = add(start, mul(step, Value::int(k))?)?;
        rows.push((x, evaluate_env(f, &Env { ans: Value::int(0), angle, x: Some(x), mem, num: NumFmt::Norm(2), cells: None })));
    }
    Ok(rows)
}

/// The real roots of `ax² + bx + c`, named `x1`, `x2` (one root is named `x`).
pub fn quadratic(a: Value, b: Value, c: Value) -> Result<Vec<(&'static str, Value)>, CalcError> {
    if a.is_zero() {
        return Err(CalcError::Math);
    }
    let disc = sub(mul(b, b)?, mul(Value::int(4), mul(a, c)?)?)?;
    let two_a = mul(Value::int(2), a)?;
    if disc.is_zero() {
        return Ok(vec![("x", div(neg(b)?, two_a)?)]);
    }
    let root = sqrt(disc)?;
    let (lo, hi) = (div(sub(neg(b)?, root)?, two_a)?, div(add(neg(b)?, root)?, two_a)?);
    let (first, second) = if lo.to_f64() <= hi.to_f64() { (lo, hi) } else { (hi, lo) };
    Ok(vec![("x1", first), ("x2", second)])
}

/// `a1 x + b1 y = c1` and `a2 x + b2 y = c2`.
pub fn simultaneous(a1: Value, b1: Value, c1: Value, a2: Value, b2: Value, c2: Value) -> Result<(Value, Value), CalcError> {
    let det = sub(mul(a1, b2)?, mul(a2, b1)?)?;
    let x = div(sub(mul(c1, b2)?, mul(c2, b1)?)?, det)?;
    let y = div(sub(mul(a1, c2)?, mul(a2, c1)?)?, det)?;
    Ok((x, y))
}

// ---- The calculator ------------------------------------------------------------------------

/// What the answer line shows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Outcome {
    Nothing,
    Answer(Value),
    /// A complex answer (the Complex app).
    Complex(crate::calc_poly::C),
    Error(CalcError),
}

/// A key press, after SHIFT has chosen between a key's two jobs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Digit(u8),
    Dot,
    Plus,
    Minus,
    Times,
    Divide,
    Neg,
    LParen,
    RParen,
    Pi,
    E,
    Ans,
    X,
    Exp10,
    Frac,
    Sqrt,
    Cbrt,
    NRoot,
    Square,
    Cube,
    Inverse,
    Pow,
    TenPow,
    EPow,
    Fact,
    Percent,
    Ncr,
    Npr,
    Func(Func),
    /// Exact or decimal.
    Sd,
    Del,
    Ac,
    Left,
    Right,
    Up,
    Down,
    Equals,
    Shift,
    /// Degrees or radians.
    AngleMode,
    /// Back to the home menu (handled by the device).
    Home,
    /// Stored variable `A`..`F`, `y`, `z` (indices as in `Tok::Var`; 6 is x).
    Var(u8),
    Mixed,
    LogBase,
    Undo,
    /// Run the sum and show the answer as a decimal (SHIFT + EXE).
    ExeDecimal,
    SetFormat(ResultFormat),
    EqSign,
    Comma,
    Dms,
    ImagI,
    Qr,
    /// The next template takes what follows the cursor as its argument (SHIFT + DEL).
    Ins,
    AngD,
    AngR,
    AngG,
    Ran,
    Const(u8),
    Conv(u8),
    Deriv,
    Integral,
    Sum,
    /// A statistic by index, ŷ, x̂ (0 is the single x̂, 1 and 2 are the quadratic's pair).
    Stat(u8),
    YHat,
    XHat(u8),
    Colon,
    /// A repeating block of digits, with a bar over it.
    Recur,
    /// Base-N logic: and, or, xor, xnor.
    Logic(u8),
    // The keys that open menus and screens belong to the device, not the sum.
    Back,
    Settings,
    Variable,
    Function,
    Catalog,
    Tools,
    FormatMenu,
    On,
    Off,
    PageUp,
    PageDown,
    /// The OK key: equals in Calculate, choose elsewhere.
    Ok,
}

#[derive(Clone, Debug)]
struct Entry {
    tokens: Vec<Tok>,
    value: Value,
}

const HISTORY: usize = 50;

#[derive(Clone, Debug)]
pub struct Calc {
    pub tokens: Vec<Tok>,
    pub cursor: usize,
    pub shift: bool,
    pub angle: Angle,
    pub outcome: Outcome,
    /// How the answer is being shown (the FORMAT key and menu).
    pub format: ResultFormat,
    pub cfg: Config,
    pub mem: Memory,
    /// INS is on: the next template wraps what follows the cursor.
    pub ins: bool,
    /// The Complex app: sums are over complex numbers and answers are `a+bi`.
    pub complex: bool,
    pub ans_c: crate::calc_poly::C,
    /// The two answers of Pol( or Rec( (r and θ, or x and y), shown on two lines.
    pub pair: Option<(Value, Value, bool)>,
    ans: Value,
    history: Vec<Entry>,
    replay: Option<usize>,
    /// Earlier states of the line, for Undo.
    undo: Vec<(Vec<Tok>, usize)>,
    /// The last thing pressed was `=`: the next key starts a new sum (or carries on from `Ans`).
    fresh: bool,
}

impl Default for Calc {
    fn default() -> Self {
        Calc {
            tokens: vec![],
            cursor: 0,
            shift: false,
            angle: Angle::Degree,
            outcome: Outcome::Nothing,
            format: ResultFormat::Standard,
            cfg: Config::default(),
            mem: Memory::default(),
            pair: None,
            ins: false,
            complex: false,
            ans_c: crate::calc_poly::C::new(0.0, 0.0),
            ans: Value::int(0),
            history: vec![],
            replay: None,
            undo: vec![],
            fresh: false,
        }
    }
}

/// The index just past the template opened at `open`, and the slot dividers between.
fn template_end(toks: &[Tok], open: usize) -> (Vec<usize>, usize) {
    let (mut depth, mut mids) = (0usize, vec![]);
    for (j, t) in toks.iter().enumerate().skip(open + 1) {
        if t.opens() {
            depth += 1;
        } else if *t == Tok::Close {
            if depth == 0 {
                return (mids, j);
            }
            depth -= 1;
        } else if t.is_mid() && depth == 0 {
            mids.push(j);
        }
    }
    (mids, toks.len())
}

impl Calc {
    /// The value of what is typed, without running it as a sum: for the fields of a table or equation.
    pub fn value(&self, x: Option<Value>) -> Result<Value, CalcError> {
        evaluate_env(&self.tokens, &Env { ans: self.ans, angle: self.angle, x, mem: &self.mem, num: self.cfg.num, cells: None })
    }

    /// A blank line that works with the same settings and memory as `like`.
    pub fn blank_like(like: &Calc) -> Calc {
        Calc { angle: like.angle, cfg: like.cfg, mem: like.mem.clone(), ans: like.ans, ..Calc::default() }
    }

    /// A blank line in the given angle unit.
    pub fn blank(angle: Angle) -> Calc {
        Calc { angle, ..Calc::default() }
    }

    /// Put `Ans` into variable `i` (0 to 8, as in `Tok::Var`; 6 is x).
    pub fn store(&mut self, i: usize) {
        if let Some(slot) = self.mem.vars.get_mut(i) {
            *slot = self.ans;
        }
    }

    /// Forget the earlier sums (the replay list).
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.replay = None;
    }

    /// Take back the last edit to the line.
    pub fn undo(&mut self) {
        if let Some((tokens, cursor)) = self.undo.pop() {
            self.tokens = tokens;
            self.cursor = cursor;
            self.outcome = Outcome::Nothing;
            self.fresh = false;
        }
    }

    fn snapshot(&mut self) {
        self.undo.push((self.tokens.clone(), self.cursor));
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
    }

    pub fn answer(&self) -> Value {
        self.ans
    }

    /// Press the key that is printed on the cap, or its SHIFT function when SHIFT is lit.
    pub fn tap(&mut self, key: Key, shifted: Option<Key>) {
        if key == Key::Shift {
            self.shift = !self.shift;
        } else if std::mem::take(&mut self.shift) {
            self.press(shifted.unwrap_or(key));
        } else {
            self.press(key);
        }
    }

    fn insert(&mut self, toks: &[Tok], cursor_in: usize) {
        if self.fresh {
            self.fresh = false;
            let carries_on = matches!(
                toks.first(),
                Some(
                    Tok::Plus
                        | Tok::Minus
                        | Tok::Times
                        | Tok::Divide
                        | Tok::Fact
                        | Tok::Percent
                        | Tok::SupOpen
                        | Tok::Exp10Open
                        | Tok::Ncr
                        | Tok::Npr
                        | Tok::AngD
                        | Tok::AngR
                        | Tok::AngG
                        | Tok::Conv(_)
                )
            );
            self.tokens.clear();
            self.cursor = 0;
            if carries_on {
                self.tokens.push(Tok::Ans);
                self.cursor = 1;
            }
        }
        self.snapshot();
        self.outcome = Outcome::Nothing;
        self.replay = None;
        self.format = ResultFormat::Standard;
        let at = self.cursor.min(self.tokens.len());
        // INS: a template takes what follows the cursor into its first slot.
        if std::mem::take(&mut self.ins)
            && toks.first().is_some_and(|t| t.opens())
            && let Some(end) = self.argument_end(at)
        {
            let arg: Vec<Tok> = self.tokens.drain(at..end).collect();
            let mut piece = vec![toks[0]];
            piece.extend(&arg);
            piece.extend(&toks[1..]);
            self.tokens.splice(at..at, piece);
            self.cursor = at + 1 + arg.len();
            return;
        }
        self.tokens.splice(at..at, toks.iter().copied());
        self.cursor = at + cursor_in;
    }

    /// Where the argument starting at `at` ends: a number, a bracket, a function's bracket or a template.
    fn argument_end(&self, at: usize) -> Option<usize> {
        let t = *self.tokens.get(at)?;
        match t {
            Tok::Digit(_) | Tok::Dot => Some(at + self.tokens[at..].iter().take_while(|t| matches!(t, Tok::Digit(_) | Tok::Dot)).count()),
            t if t.opens() => Some((template_end(&self.tokens, at).1 + 1).min(self.tokens.len())),
            Tok::LParen | Tok::Func(_) => {
                let mut depth = 0i32;
                for (j, u) in self.tokens.iter().enumerate().skip(at) {
                    match u {
                        Tok::LParen | Tok::Func(_) => depth += 1,
                        Tok::RParen => {
                            depth -= 1;
                            if depth == 0 {
                                return Some(j + 1);
                            }
                        }
                        _ => {}
                    }
                }
                Some(self.tokens.len())
            }
            _ => Some(at + 1),
        }
    }

    fn add(&mut self, t: Tok) {
        self.insert(&[t], 1);
    }

    pub fn press(&mut self, key: Key) {
        match key {
            Key::Digit(d) => self.add(Tok::Digit(d)),
            Key::Dot => self.add(Tok::Dot),
            Key::Plus => self.add(Tok::Plus),
            Key::Minus => self.add(Tok::Minus),
            Key::Times => self.add(Tok::Times),
            Key::Divide => self.add(Tok::Divide),
            Key::Neg => self.add(Tok::Neg),
            Key::LParen => self.add(Tok::LParen),
            Key::RParen => self.add(Tok::RParen),
            Key::Pi => self.add(Tok::Pi),
            Key::E => self.add(Tok::E),
            Key::Ans => self.add(Tok::Ans),
            Key::X => self.add(Tok::X),
            Key::Fact => self.add(Tok::Fact),
            Key::Percent => self.add(Tok::Percent),
            Key::Ncr => self.add(Tok::Ncr),
            Key::Npr => self.add(Tok::Npr),
            Key::Func(f) => self.add(Tok::Func(f)),
            Key::Var(i) => self.add(if i == 6 { Tok::X } else { Tok::Var(i) }),
            Key::Mixed => self.insert(&[Tok::MixOpen, Tok::FracMid, Tok::FracMid, Tok::Close], 1),
            Key::LogBase => self.insert(&[Tok::LogOpen, Tok::LogMid, Tok::Close], 1),
            Key::Undo => self.undo(),
            Key::SetFormat(f) => {
                if matches!(self.outcome, Outcome::Answer(_)) {
                    self.format = f;
                }
            }
            Key::ExeDecimal => {
                self.equals();
                if matches!(self.outcome, Outcome::Answer(_)) {
                    self.format = ResultFormat::Decimal;
                }
            }
            Key::Exp10 if self.cfg.exp_power => self.insert(&[Tok::Times, Tok::Digit(1), Tok::Digit(0), Tok::SupOpen, Tok::Close], 4),
            Key::Exp10 => self.insert(&[Tok::Exp10Open, Tok::Close], 1),
            Key::EqSign => self.add(Tok::EqSign),
            Key::Comma => self.add(Tok::Comma),
            Key::Dms => self.add(Tok::Dms),
            Key::AngD => self.add(Tok::AngD),
            Key::AngR => self.add(Tok::AngR),
            Key::AngG => self.add(Tok::AngG),
            Key::Ran => self.add(Tok::Ran),
            Key::Const(c) => self.add(Tok::Const(c)),
            Key::Conv(c) => self.add(Tok::Conv(c)),
            Key::Deriv => self.insert(&[Tok::DerivOpen, Tok::SlotMid, Tok::Close], 1),
            Key::Integral => self.insert(&[Tok::IntOpen, Tok::SlotMid, Tok::SlotMid, Tok::Close], 1),
            Key::Sum => self.insert(&[Tok::SumOpen, Tok::SlotMid, Tok::SlotMid, Tok::Close], 1),
            Key::Stat(i) => self.add(Tok::Stat(i)),
            Key::YHat => self.add(Tok::YHat),
            Key::XHat(k) => self.add(Tok::XHat(k)),
            Key::Colon => self.add(Tok::Colon),
            Key::Recur => self.insert(&[Tok::RecurOpen, Tok::Close], 1),
            Key::Logic(k) => self.add(Tok::Logic(k)),
            Key::Ins => self.ins = !self.ins,
            Key::ImagI => self.add(Tok::I),
            // The device answers this one.
            Key::Qr => {}
            Key::Frac => self.frac(),
            Key::Sqrt => self.insert(&[Tok::SqrtOpen, Tok::Close], 1),
            Key::Cbrt => self.insert(&[Tok::RootOpen, Tok::Digit(3), Tok::RootMid, Tok::Close], 3),
            Key::NRoot => self.insert(&[Tok::RootOpen, Tok::RootMid, Tok::Close], 1),
            Key::Square => self.insert(&[Tok::SupOpen, Tok::Digit(2), Tok::Close], 3),
            Key::Cube => self.insert(&[Tok::SupOpen, Tok::Digit(3), Tok::Close], 3),
            Key::Inverse => self.insert(&[Tok::SupOpen, Tok::Neg, Tok::Digit(1), Tok::Close], 4),
            Key::Pow => self.insert(&[Tok::SupOpen, Tok::Close], 1),
            Key::TenPow => self.insert(&[Tok::Digit(1), Tok::Digit(0), Tok::SupOpen, Tok::Close], 3),
            Key::EPow => self.insert(&[Tok::E, Tok::SupOpen, Tok::Close], 2),
            Key::Sd => {
                if matches!(self.outcome, Outcome::Answer(_)) {
                    self.format = if self.format == ResultFormat::Standard { ResultFormat::Decimal } else { ResultFormat::Standard };
                }
            }
            Key::Del => self.delete(),
            Key::Ac => {
                let history = std::mem::take(&mut self.history);
                *self = Calc { history, ..Calc::blank_like(self) };
            }
            Key::Left => self.step(false),
            Key::Right => self.step(true),
            Key::Up => self.replay_step(true),
            Key::Down => self.replay_step(false),
            Key::Equals | Key::Ok => self.equals(),
            Key::Home
            | Key::Back
            | Key::Settings
            | Key::Variable
            | Key::Function
            | Key::Catalog
            | Key::Tools
            | Key::FormatMenu
            | Key::On
            | Key::Off => {}
            Key::PageUp => self.cursor = 0,
            Key::PageDown => self.cursor = self.tokens.len(),
            Key::Shift => self.shift = !self.shift,
            Key::AngleMode => self.angle = self.angle.next(),
        }
    }

    /// `▭/▭`. A number just typed becomes the top of the fraction.
    fn frac(&mut self) {
        let end = self.cursor.min(self.tokens.len());
        let start = self.tokens[..end].iter().rposition(|t| !matches!(t, Tok::Digit(_) | Tok::Dot)).map_or(0, |p| p + 1);
        if self.fresh || start == end {
            self.insert(&[Tok::FracOpen, Tok::FracMid, Tok::Close], 1);
            return;
        }
        self.outcome = Outcome::Nothing;
        self.replay = None;
        let number: Vec<Tok> = self.tokens[start..end].to_vec();
        let mut piece = vec![Tok::FracOpen];
        piece.extend(&number);
        piece.extend([Tok::FracMid, Tok::Close]);
        self.tokens.splice(start..end, piece);
        self.cursor = start + 1 + number.len() + 1;
    }

    fn step(&mut self, forward: bool) {
        self.fresh = false;
        self.outcome = Outcome::Nothing;
        self.cursor = if forward { (self.cursor + 1).min(self.tokens.len()) } else { self.cursor.saturating_sub(1) };
    }

    fn delete(&mut self) {
        self.snapshot();
        self.fresh = false;
        self.outcome = Outcome::Nothing;
        self.replay = None;
        self.format = ResultFormat::Standard;
        let Some(before) = self.cursor.checked_sub(1).filter(|b| *b < self.tokens.len()) else { return };
        let t = self.tokens[before];
        if t == Tok::Close || t.is_mid() {
            // Step back into the template rather than tear a hole in it.
            self.cursor = before;
        } else if t.opens() {
            let (mids, close) = template_end(&self.tokens, before);
            if close < self.tokens.len() {
                self.tokens.remove(close);
            }
            for m in mids.into_iter().rev() {
                self.tokens.remove(m);
            }
            self.tokens.remove(before);
            self.cursor = before;
        } else {
            self.tokens.remove(before);
            self.cursor = before;
        }
    }

    fn equals(&mut self) {
        if self.tokens.is_empty() {
            return;
        }
        self.pair = None;
        if self.complex {
            let vars: [f64; 9] = std::array::from_fn(|i| self.mem.vars[i].to_f64());
            match crate::calc_complex::evaluate(&self.tokens, self.ans_c, self.angle, vars) {
                Ok(z) => {
                    self.ans_c = z;
                    self.ans = Value::Float(z.re);
                    self.outcome = Outcome::Complex(z);
                    self.fresh = true;
                }
                Err(e) => {
                    self.outcome = Outcome::Error(e);
                    self.fresh = false;
                }
            }
            self.cursor = self.tokens.len();
            return;
        }
        match evaluate_full(&self.tokens, &Env { ans: self.ans, angle: self.angle, x: None, mem: &self.mem, num: self.cfg.num, cells: None }) {
            Ok((v, pair)) => {
                // Pol( and Rec( answer twice, and keep the two answers in x and y.
                if let Some((a, b, _)) = pair {
                    self.mem.vars[6] = a;
                    self.mem.vars[7] = b;
                }
                self.pair = pair;
                self.ans = v;
                self.outcome = Outcome::Answer(v);
                self.fresh = true;
                // Typing a decimal point asks for a decimal answer, as on the real thing.
                self.format = if self.tokens.contains(&Tok::Dot) { ResultFormat::Decimal } else { ResultFormat::Standard };
                self.history.push(Entry { tokens: self.tokens.clone(), value: v });
                if self.history.len() > HISTORY {
                    self.history.remove(0);
                }
                self.replay = None;
            }
            Err(e) => {
                self.outcome = Outcome::Error(e);
                self.fresh = false;
            }
        }
        self.cursor = self.tokens.len();
    }

    /// Bring back an earlier sum to edit or run again: `up` goes further back.
    fn replay_step(&mut self, up: bool) {
        if self.history.is_empty() {
            return;
        }
        let next = match (self.replay, up) {
            (None, true) => Some(self.history.len() - 1),
            (None, false) => return,
            (Some(i), true) => Some(i.saturating_sub(1)),
            (Some(i), false) if i + 1 < self.history.len() => Some(i + 1),
            (Some(_), false) => None,
        };
        self.fresh = false;
        self.format = ResultFormat::Standard;
        self.replay = next;
        match next.and_then(|i| self.history.get(i)) {
            Some(e) => {
                self.tokens = e.tokens.clone();
                self.outcome = Outcome::Answer(e.value);
            }
            None => {
                self.tokens.clear();
                self.outcome = Outcome::Nothing;
            }
        }
        self.cursor = self.tokens.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(keys: &[Key]) -> Calc {
        let mut c = Calc::default();
        for k in keys {
            c.press(*k);
        }
        c
    }

    fn num(s: &str) -> Vec<Key> {
        s.chars().map(|ch| if ch == '.' { Key::Dot } else { Key::Digit(ch as u8 - b'0') }).collect()
    }

    fn eq(mut keys: Vec<Key>) -> Calc {
        keys.push(Key::Equals);
        run(&keys)
    }

    fn shown(c: &Calc) -> String {
        match c.outcome {
            Outcome::Answer(v) => text(&result_tokens(v, c.format, &c.cfg)),
            Outcome::Error(e) => e.message().into(),
            Outcome::Complex(_) => "complex".into(),
            Outcome::Nothing => "-".into(),
        }
    }

    /// A readable form of tokens: `1/2` for a fraction, `2√3`, `x^(..)`.
    fn text(toks: &[Tok]) -> String {
        let mut out = String::new();
        for t in toks {
            out.push_str(&match t {
                Tok::FracOpen => "{".into(),
                Tok::FracMid => "/".into(),
                Tok::SqrtOpen => "\u{221A}[".into(),
                Tok::SupOpen => "^[".into(),
                Tok::Close => "}".into(),
                other => other.text(),
            });
        }
        out
    }

    #[test]
    fn arithmetic_follows_the_order_of_operations() {
        let sum: Vec<Key> = [num("2"), vec![Key::Plus], num("3"), vec![Key::Times], num("4")].concat();
        assert_eq!(shown(&eq(sum)), "14");
        let bracketed: Vec<Key> = [vec![Key::LParen], num("2"), vec![Key::Plus], num("3"), vec![Key::RParen, Key::Times], num("4")].concat();
        assert_eq!(shown(&eq(bracketed)), "20");
    }

    #[test]
    fn division_gives_an_exact_fraction_that_sd_turns_to_a_decimal() {
        let mut c = eq([num("1"), vec![Key::Divide], num("3")].concat());
        assert_eq!(shown(&c), "{1/3}");
        c.press(Key::Sd);
        assert_eq!(shown(&c), "0.3333333333");
        c.press(Key::Sd);
        assert_eq!(shown(&c), "{1/3}");
    }

    #[test]
    fn fractions_add_exactly() {
        // 1/2 + 1/3, typed with the fraction template
        let mut c = Calc::default();
        c.press(Key::Digit(1));
        c.press(Key::Frac);
        c.press(Key::Digit(2));
        c.press(Key::Right);
        c.press(Key::Right);
        c.press(Key::Plus);
        c.press(Key::Digit(1));
        c.press(Key::Frac);
        c.press(Key::Digit(3));
        c.press(Key::Equals);
        assert_eq!(shown(&c), "{5/6}");
    }

    #[test]
    fn a_typed_number_becomes_the_numerator() {
        let mut c = Calc::default();
        c.press(Key::Digit(2));
        c.press(Key::Digit(7));
        c.press(Key::Frac);
        assert_eq!(c.tokens, vec![Tok::FracOpen, Tok::Digit(2), Tok::Digit(7), Tok::FracMid, Tok::Close]);
        assert_eq!(c.cursor, 4, "the cursor waits in the denominator");
    }

    #[test]
    fn roots_stay_exact() {
        let sq = |n: &str| eq([vec![Key::Sqrt], num(n)].concat());
        assert_eq!(shown(&sq("49")), "7");
        assert_eq!(shown(&sq("12")), "2\u{221A}[3}");
        assert_eq!(shown(&sq("2")), "\u{221A}[2}");
        assert_eq!(shown(&sq("4")), "2");
        let cube = eq([vec![Key::Cbrt], num("27")].concat());
        assert_eq!(shown(&cube), "3");
    }

    #[test]
    fn a_negative_root_is_a_math_error() {
        let c = eq(vec![Key::Sqrt, Key::Neg, Key::Digit(4)]);
        assert_eq!(c.outcome, Outcome::Error(CalcError::Math));
    }

    #[test]
    fn powers_and_squares() {
        assert_eq!(shown(&eq([num("5"), vec![Key::Square]].concat())), "25");
        assert_eq!(shown(&eq([num("2"), vec![Key::Pow], num("10")].concat())), "1,024", "the digit separator is on at first");
        assert_eq!(shown(&eq([num("2"), vec![Key::Inverse]].concat())), "{1/2}");
        let neg_square = eq(vec![Key::Neg, Key::Digit(3), Key::Square]);
        assert_eq!(shown(&neg_square), "\u{2212}9", "(\u{2212})3\u{00B2} is \u{2212}(3\u{00B2}), as on a Casio");
    }

    #[test]
    fn trig_in_degrees_is_exact_for_the_special_angles() {
        let f = |f: Func, a: &str| eq([vec![Key::Func(f)], num(a)].concat());
        assert_eq!(shown(&f(Func::Sin, "30")), "{1/2}");
        assert_eq!(shown(&f(Func::Cos, "60")), "{1/2}");
        assert_eq!(shown(&f(Func::Tan, "45")), "1");
        assert_eq!(shown(&f(Func::Sin, "60")), "{\u{221A}[3}/2}");
        assert_eq!(shown(&f(Func::ASin, "0.5")), "30");
        assert_eq!(f(Func::Tan, "90").outcome, Outcome::Error(CalcError::Math));
    }

    #[test]
    fn radians_are_a_switch() {
        let mut c = Calc::default();
        c.press(Key::AngleMode);
        c.press(Key::Func(Func::Sin));
        c.press(Key::Digit(0));
        c.press(Key::Equals);
        assert_eq!(shown(&c), "0");
        assert_ne!(c.angle, Angle::Degree);
    }

    #[test]
    fn division_by_zero_is_a_math_error_and_a_gap_is_a_syntax_error() {
        assert_eq!(eq([num("5"), vec![Key::Divide], num("0")].concat()).outcome, Outcome::Error(CalcError::Math));
        assert_eq!(eq(vec![Key::Digit(5), Key::Times]).outcome, Outcome::Error(CalcError::Syntax));
        let hole = eq(vec![Key::Frac, Key::Digit(1)]);
        assert_eq!(hole.outcome, Outcome::Error(CalcError::Syntax), "the denominator is empty");
    }

    #[test]
    fn implicit_multiplication_binds_tighter_than_division() {
        // 1 ÷ 2π = 1 ÷ (2π), not (1 ÷ 2)π
        let c = eq([num("1"), vec![Key::Divide], num("2"), vec![Key::Pi]].concat());
        assert_eq!(shown(&c), "0.1591549431");
        assert_eq!(shown(&eq([num("2"), vec![Key::LParen], num("3"), vec![Key::RParen]].concat())), "6");
    }

    #[test]
    fn factorials_and_combinations() {
        assert_eq!(shown(&eq([num("5"), vec![Key::Fact]].concat())), "120");
        assert_eq!(shown(&eq([num("5"), vec![Key::Ncr], num("2")].concat())), "10");
        assert_eq!(shown(&eq([num("5"), vec![Key::Npr], num("2")].concat())), "20");
        let negative = eq(vec![Key::LParen, Key::Neg, Key::Digit(1), Key::RParen, Key::Fact]);
        assert_eq!(negative.outcome, Outcome::Error(CalcError::Math));
    }

    #[test]
    fn large_and_small_answers_use_standard_form() {
        let big = eq([num("12345678901"), vec![Key::Times], num("10")].concat());
        assert_eq!(shown(&big), "1.23456789\u{00D7}10^[11}", "more digits than the screen holds go to standard form");
        assert_eq!(shown(&eq(num("0.00001"))), "0.00001", "Norm 2 keeps small decimals plain");
        let tiny = eq(num("0.0000000001"));
        assert_eq!(shown(&tiny), "1\u{00D7}10^[\u{2212}10}", "a very small decimal is in standard form");
    }

    #[test]
    fn ans_carries_on_after_equals() {
        let mut c = eq([num("6"), vec![Key::Times], num("7")].concat());
        c.press(Key::Plus);
        assert_eq!(c.tokens, vec![Tok::Ans, Tok::Plus], "an operator carries on from the answer");
        c.press(Key::Digit(8));
        c.press(Key::Equals);
        assert_eq!(shown(&c), "50");
        c.press(Key::Digit(1));
        assert_eq!(c.tokens, vec![Tok::Digit(1)], "a digit starts a new sum");
    }

    #[test]
    fn shift_takes_the_second_function_once() {
        let mut c = Calc::default();
        c.tap(Key::Shift, None);
        assert!(c.shift);
        c.tap(Key::Func(Func::Sin), Some(Key::Func(Func::ASin)));
        assert!(!c.shift);
        assert_eq!(c.tokens, vec![Tok::Func(Func::ASin)]);
        c.tap(Key::Func(Func::Sin), Some(Key::Func(Func::ASin)));
        assert_eq!(c.tokens.last(), Some(&Tok::Func(Func::Sin)));
    }

    #[test]
    fn delete_steps_over_a_template_boundary_and_takes_a_whole_empty_template_out() {
        let mut c = Calc::default();
        c.press(Key::Sqrt);
        c.press(Key::Digit(9));
        c.press(Key::Right);
        assert_eq!(c.cursor, 3);
        c.press(Key::Del);
        assert_eq!(c.cursor, 2, "the first delete only steps back inside the root");
        c.press(Key::Del);
        c.press(Key::Del);
        assert!(c.tokens.is_empty(), "taking the 9 and then the root sign leaves nothing behind: {:?}", c.tokens);
    }

    #[test]
    fn replay_brings_back_earlier_sums() {
        let mut c = eq([num("2"), vec![Key::Plus], num("2")].concat());
        c.press(Key::Ac);
        c.press(Key::Digit(9));
        c.press(Key::Equals);
        c.press(Key::Up);
        assert_eq!(c.tokens, vec![Tok::Digit(9)]);
        c.press(Key::Up);
        assert_eq!(text(&c.tokens), "2+2");
        assert_eq!(shown(&c), "4");
        c.press(Key::Down);
        c.press(Key::Down);
        assert!(c.tokens.is_empty());
    }

    #[test]
    fn ac_clears_the_line_but_keeps_ans_and_the_angle_mode() {
        let mut c = eq([num("7"), vec![Key::Times], num("6")].concat());
        c.press(Key::AngleMode);
        c.press(Key::Ac);
        assert!(c.tokens.is_empty());
        assert_ne!(c.angle, Angle::Degree);
        assert_eq!(c.answer(), Value::int(42));
    }

    #[test]
    fn decimals_are_exact_while_they_can_be() {
        assert_eq!(shown(&eq([num("0.1"), vec![Key::Plus], num("0.2")].concat())), "0.3");
        assert_eq!(shown(&eq([num("0.5"), vec![Key::Times], num("4")].concat())), "2");
    }

    #[test]
    fn e_and_logs() {
        assert_eq!(shown(&eq([vec![Key::Func(Func::Log)], num("1000")].concat())), "3");
        assert_eq!(shown(&eq(vec![Key::Func(Func::Ln), Key::E])), "1");
        assert_eq!(eq(vec![Key::Func(Func::Log), Key::Digit(0)]).outcome, Outcome::Error(CalcError::Math));
        assert_eq!(shown(&eq([vec![Key::TenPow], num("3")].concat())), "1,000");
    }
}
