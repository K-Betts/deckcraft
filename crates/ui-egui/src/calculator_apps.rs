//! The fx-991CW's home menu and the apps this emulator has: Calculate, Table and Equation. The
//! others on the menu (Statistics, Inequality, Distribution) are listed but say so when opened.

use crate::calculator::{
    Angle, CONSTANTS, CONVERSIONS, Calc, CalcError, Config, Func, Io, Key, Memory, NumFmt, Outcome, ResultFormat, Tok, Value, quadratic,
    simultaneous, table_rows, value_tokens,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum App {
    Home,
    Calculate,
    Table,
    Equation,
}

/// What the equation app can solve.
pub const EQUATIONS: [&str; 2] = ["Simultaneous (2 unknowns)", "Quadratic"];

/// One line of an answer: a label and a value.
#[derive(Clone, Debug)]
pub struct Row {
    pub label: String,
    pub value: Vec<Tok>,
    /// Shown instead of a value when this row has none (an error).
    pub note: &'static str,
}

/// A page of fields you fill in one by one and then solve.
#[derive(Clone)]
pub struct Form {
    pub title: &'static str,
    pub labels: Vec<&'static str>,
    pub fields: Vec<Calc>,
    pub focus: usize,
    pub rows: Option<Vec<Row>>,
    pub scroll: usize,
}

impl Form {
    fn new(title: &'static str, labels: &[&'static str], defaults: &[&str], angle: Angle) -> Form {
        let fields = labels
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let mut c = Calc::blank(angle);
                for ch in defaults.get(i).copied().unwrap_or("").chars() {
                    c.press(match ch {
                        '.' => Key::Dot,
                        '-' => Key::Neg,
                        d => Key::Digit(d as u8 - b'0'),
                    });
                }
                c
            })
            .collect();
        Form { title, labels: labels.to_vec(), fields, focus: 0, rows: None, scroll: 0 }
    }

    fn value(&self, i: usize) -> Result<Value, CalcError> {
        self.fields.get(i).ok_or(CalcError::Syntax)?.value(None)
    }

    fn press(&mut self, key: Key, solve: fn(&Form) -> Result<Vec<Row>, CalcError>) {
        let last = self.fields.len().saturating_sub(1);
        match key {
            Key::Up => {
                if self.rows.is_some() {
                    self.scroll = self.scroll.saturating_sub(1);
                } else {
                    self.focus = self.focus.saturating_sub(1);
                }
            }
            Key::Down => {
                if let Some(rows) = &self.rows {
                    self.scroll = (self.scroll + 1).min(rows.len().saturating_sub(1));
                } else {
                    self.focus = (self.focus + 1).min(last);
                }
            }
            Key::Ok | Key::Equals => {
                if self.rows.is_some() {
                    self.rows = None;
                } else if self.focus < last {
                    self.focus += 1;
                } else {
                    self.scroll = 0;
                    self.rows = Some(solve(self).unwrap_or_else(|e| vec![Row { label: e.message().into(), value: vec![], note: "" }]));
                }
            }
            Key::Ac => {
                if self.rows.is_some() {
                    self.rows = None;
                } else if let Some(f) = self.fields.get_mut(self.focus) {
                    *f = Calc::blank(f.angle);
                }
            }
            other => {
                self.rows = None;
                if let Some(f) = self.fields.get_mut(self.focus) {
                    f.press(other);
                }
            }
        }
    }

    fn set_angle(&mut self, d: Angle) {
        for f in &mut self.fields {
            f.angle = d;
        }
    }
}

fn solve_table(f: &Form) -> Result<Vec<Row>, CalcError> {
    let expr = f.fields.first().ok_or(CalcError::Syntax)?;
    if expr.tokens.is_empty() {
        return Err(CalcError::Syntax);
    }
    let rows = table_rows(&expr.tokens, f.value(1)?, f.value(2)?, f.value(3)?, expr.angle, &expr.mem)?;
    Ok(rows
        .into_iter()
        .map(|(x, y)| match y {
            Ok(v) => Row { label: tokens_text(&value_tokens(x, true)), value: value_tokens(v, false), note: "" },
            Err(e) => Row { label: tokens_text(&value_tokens(x, true)), value: vec![], note: e.message() },
        })
        .collect())
}

fn solve_simultaneous(f: &Form) -> Result<Vec<Row>, CalcError> {
    let v = |i| f.value(i);
    let (x, y) = simultaneous(v(0)?, v(1)?, v(2)?, v(3)?, v(4)?, v(5)?)?;
    Ok(vec![Row { label: "x".into(), value: value_tokens(x, false), note: "" }, Row { label: "y".into(), value: value_tokens(y, false), note: "" }])
}

fn solve_quadratic(f: &Form) -> Result<Vec<Row>, CalcError> {
    Ok(quadratic(f.value(0)?, f.value(1)?, f.value(2)?)?
        .into_iter()
        .map(|(n, v)| Row { label: n.into(), value: value_tokens(v, false), note: "" })
        .collect())
}

/// Flat tokens (digits, a point, a sign) as text.
pub fn tokens_text(toks: &[Tok]) -> String {
    toks.iter().map(|t| t.text()).collect()
}

/// The home screen's apps, three to a row and two rows to a page.
pub const HOME_APPS: [(&str, Option<App>); 13] = [
    ("Calculate", Some(App::Calculate)),
    ("Statistics", None),
    ("Table", Some(App::Table)),
    ("Equation", Some(App::Equation)),
    ("Inequality", None),
    ("Distribution", None),
    ("Complex", None),
    ("Matrix", None),
    ("Vector", None),
    ("Spreadsheet", None),
    ("Ratio", None),
    ("Math Box", None),
    ("Base-N", None),
];

/// Which menu a [`Menu`] is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuId {
    Settings,
    CalcSettings,
    SystemSettings,
    InputOutput,
    AngleUnit,
    NumberFormat,
    FixDigits,
    SciDigits,
    FractionResult,
    FormatKey,
    Reset,
    Catalog,
    CatalogCategory(u8),
    ConstCategories,
    ConstCategory(u8),
    ConvCategories,
    ConvCategory(u8),
    ExpKey,
    DecimalMark,
    DigitSeparator,
    Tools,
    Format,
    Function,
    VarAction(u8),
}

/// What choosing a menu line does.
#[derive(Clone, Debug)]
pub enum Act {
    Open(MenuId),
    /// Press these keys on the calculator line, then close the menus.
    Keys(Vec<Key>),
    SetIo(Io),
    SetAngle(Angle),
    SetNum(NumFmt),
    SetMixed(bool),
    SetFormatKeyMenu(bool),
    SetExpPower(bool),
    SetCommaMark(bool),
    SetSeparator(bool),
    SetOptionNumbers(bool),
    SetFormat(ResultFormat),
    Undo,
    ClearHistory,
    Store(u8),
    Recall(u8),
    EditVar(u8),
    DefineFn(u8),
    ResetSettings,
    ResetVariables,
    ResetAll,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub label: String,
    /// `Some(selected)` draws a radio button.
    pub radio: Option<bool>,
    /// A lower level opens from this line.
    pub more: bool,
    pub act: Act,
}

#[derive(Clone, Debug)]
pub struct Menu {
    pub id: MenuId,
    pub items: Vec<Item>,
    pub sel: usize,
}

/// What an input screen does with the value typed.
#[derive(Clone, Copy, Debug)]
pub enum Then {
    SetVar(u8),
    DefineFn(u8),
}

/// A one-line editor opened over the calculator: a variable's new value, the equation of f or g.
#[derive(Clone)]
pub struct Prompt {
    pub label: String,
    pub field: Calc,
    pub then: Then,
}

/// Something laid over the app: a menu, the variable list or an editor.
#[derive(Clone)]
pub enum Overlay {
    Menu(Menu),
    Variables { sel: usize },
    Prompt(Prompt),
}

fn line(label: &str, act: Act) -> Item {
    Item { label: label.into(), radio: None, more: false, act }
}

fn sub(label: &str, id: MenuId) -> Item {
    Item { label: label.into(), radio: None, more: true, act: Act::Open(id) }
}

fn radio(label: &str, on: bool, act: Act) -> Item {
    Item { label: label.into(), radio: Some(on), more: false, act }
}

fn keys(label: &str, k: &[Key]) -> Item {
    line(label, Act::Keys(k.to_vec()))
}

/// The categories in a table, in the order they first appear.
fn category_names<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut out: Vec<&str> = vec![];
    for n in names {
        if !out.contains(&n) {
            out.push(n);
        }
    }
    out
}

/// The whole device: which app is showing and the state of each.
#[derive(Clone)]
pub struct Device {
    pub app: App,
    pub home_sel: usize,
    /// Said on the home screen after choosing an app that is not here.
    pub note: Option<&'static str>,
    pub shift: bool,
    pub on: bool,
    pub calc: Calc,
    pub table: Form,
    /// The kind of equation being chosen (before one is open).
    pub eq_sel: usize,
    pub equation: Option<(usize, Form)>,
    /// Open menus, newest last, or an editor, or the variable list.
    pub stack: Vec<Overlay>,
    pub cfg: Config,
    pub mem: Memory,
    /// The FORMAT key opens the FORMAT menu (and SHIFT+FORMAT toggles), instead of the other way round.
    pub format_key_menu: bool,
    pub option_numbers: bool,
}

impl Default for Device {
    fn default() -> Self {
        Device {
            app: App::Calculate,
            home_sel: 0,
            note: None,
            shift: false,
            on: true,
            calc: Calc::default(),
            table: Form::new("Table", &["f(x) =", "Start", "End", "Step"], &["", "1", "5", "1"], Angle::Degree),
            eq_sel: 0,
            equation: None,
            stack: vec![],
            cfg: Config::default(),
            mem: Memory::default(),
            format_key_menu: false,
            option_numbers: true,
        }
    }
}

const NUM_FORMATS: [(&str, NumFmt); 2] = [("Norm 1", NumFmt::Norm(1)), ("Norm 2", NumFmt::Norm(2))];

impl Device {
    pub fn angle(&self) -> Angle {
        self.calc.angle
    }

    pub fn menu(&self) -> Option<&Menu> {
        match self.stack.last() {
            Some(Overlay::Menu(m)) => Some(m),
            _ => None,
        }
    }

    /// A key press, with SHIFT choosing between the printed key and its second function.
    pub fn tap(&mut self, key: Key, shifted: Option<Key>) {
        if key == Key::Shift {
            if self.on {
                self.shift = !self.shift;
            }
            return;
        }
        let k = if std::mem::take(&mut self.shift) { shifted.unwrap_or(key) } else { key };
        self.press(k);
    }

    fn sync(&mut self) {
        self.calc.cfg = self.cfg;
        self.calc.mem = self.mem.clone();
        let (cfg, mem) = (self.cfg, self.mem.clone());
        let set = |f: &mut Form| {
            for field in &mut f.fields {
                field.cfg = cfg;
                field.mem = mem.clone();
            }
        };
        set(&mut self.table);
        if let Some((_, f)) = &mut self.equation {
            set(f);
        }
    }

    pub fn press(&mut self, key: Key) {
        match key {
            Key::On => {
                self.on = true;
                return;
            }
            Key::Off => {
                self.on = false;
                self.stack.clear();
                self.shift = false;
                return;
            }
            _ if !self.on => return,
            _ => {}
        }
        self.sync();
        if self.app != App::Home {
            self.note = None;
        }
        match key {
            Key::Qr => {
                self.note = Some("QR codes are not in this emulator");
                return;
            }
            Key::ImagI => {
                self.note = Some("i works in the Complex app");
                return;
            }
            _ => {}
        }
        if !self.stack.is_empty() {
            self.overlay_key(key);
            return;
        }
        match key {
            Key::Home => {
                self.app = App::Home;
                self.note = None;
            }
            Key::AngleMode => {
                let d = self.calc.angle.next();
                self.calc.angle = d;
                self.table.set_angle(d);
                if let Some((_, f)) = &mut self.equation {
                    f.set_angle(d);
                }
            }
            Key::Settings if self.app != App::Home => self.open_menu(MenuId::Settings),
            Key::Catalog if self.app != App::Home => self.open_menu(MenuId::Catalog),
            Key::Tools if self.app != App::Home => self.open_menu(MenuId::Tools),
            Key::Function if self.app != App::Home => self.open_menu(MenuId::Function),
            Key::Variable if self.app != App::Home => self.stack.push(Overlay::Variables { sel: 0 }),
            Key::FormatMenu | Key::Sd | Key::SetFormat(_) if self.app == App::Calculate => self.format_key(key),
            Key::Back => {
                if self.app == App::Equation && self.equation.is_some() {
                    self.equation = None;
                }
            }
            _ => match self.app {
                App::Home => self.home_key(key),
                App::Calculate => {
                    self.calc.press(key);
                    // Pol( and Rec( store their answers in x and y.
                    self.mem.vars = self.calc.mem.vars;
                }
                App::Table => self.table.press(key, solve_table),
                App::Equation => self.equation_key(key),
            },
        }
    }

    /// The FORMAT key (`Sd`) and SHIFT+FORMAT (`FormatMenu`): which does what depends on the setting.
    fn format_key(&mut self, key: Key) {
        let menu = match key {
            Key::Sd => self.format_key_menu,
            Key::FormatMenu => !self.format_key_menu,
            other => {
                self.calc.press(other);
                return;
            }
        };
        if !menu {
            self.calc.press(Key::Sd);
        } else if matches!(self.calc.outcome, Outcome::Answer(_)) {
            self.open_menu(MenuId::Format);
        }
    }

    fn open_menu(&mut self, id: MenuId) {
        let items = self.menu_items(id);
        let sel = items.iter().position(|i| i.radio == Some(true)).unwrap_or(0);
        self.stack.push(Overlay::Menu(Menu { id, items, sel }));
    }

    fn menu_items(&self, id: MenuId) -> Vec<Item> {
        let c = &self.cfg;
        match id {
            MenuId::Settings => {
                vec![sub("Calc Settings", MenuId::CalcSettings), sub("System Settings", MenuId::SystemSettings), sub("Reset", MenuId::Reset)]
            }
            MenuId::CalcSettings => vec![
                sub("Input/Output", MenuId::InputOutput),
                sub("Angle Unit", MenuId::AngleUnit),
                sub("Number Format", MenuId::NumberFormat),
                sub("Fraction Result", MenuId::FractionResult),
                sub("Decimal Mark", MenuId::DecimalMark),
                sub("Digit Separator", MenuId::DigitSeparator),
                sub("\u{00D7}10\u{02E3} Key", MenuId::ExpKey),
                sub("FORMAT Key", MenuId::FormatKey),
            ],
            MenuId::ExpKey => vec![
                radio("\u{00D7}10 (Sci Notat)", !c.exp_power, Act::SetExpPower(false)),
                radio("^(10) (Power)", c.exp_power, Act::SetExpPower(true)),
            ],
            MenuId::DecimalMark => vec![radio("Dot", !c.comma_mark, Act::SetCommaMark(false)), radio("Comma", c.comma_mark, Act::SetCommaMark(true))],
            MenuId::DigitSeparator => vec![radio("On", c.separator, Act::SetSeparator(true)), radio("Off", !c.separator, Act::SetSeparator(false))],
            MenuId::SystemSettings => vec![
                radio("Show Option Num: On", self.option_numbers, Act::SetOptionNumbers(true)),
                radio("Show Option Num: Off", !self.option_numbers, Act::SetOptionNumbers(false)),
            ],
            MenuId::InputOutput => [
                ("MathI/MathO", Io::MathMath),
                ("MathI/DecimalO", Io::MathDecimal),
                ("LineI/LineO", Io::LineLine),
                ("LineI/DecimalO", Io::LineDecimal),
            ]
            .iter()
            .map(|(n, io)| radio(n, c.io == *io, Act::SetIo(*io)))
            .collect(),
            MenuId::AngleUnit => [("Degree", Angle::Degree), ("Radian", Angle::Radian), ("Gradian", Angle::Gradian)]
                .iter()
                .map(|(n, a)| radio(n, self.calc.angle == *a, Act::SetAngle(*a)))
                .collect(),
            MenuId::NumberFormat => {
                let mut v = vec![
                    Item { label: "Fix".into(), radio: Some(matches!(c.num, NumFmt::Fix(_))), more: true, act: Act::Open(MenuId::FixDigits) },
                    Item { label: "Sci".into(), radio: Some(matches!(c.num, NumFmt::Sci(_))), more: true, act: Act::Open(MenuId::SciDigits) },
                ];
                v.extend(NUM_FORMATS.iter().map(|(n, f)| radio(n, c.num == *f, Act::SetNum(*f))));
                v
            }
            MenuId::FixDigits => (0..=9u8).map(|n| radio(&format!("Fix {n}"), c.num == NumFmt::Fix(n), Act::SetNum(NumFmt::Fix(n)))).collect(),
            MenuId::SciDigits => (1..=10u8).map(|n| radio(&format!("Sci {n}"), c.num == NumFmt::Sci(n), Act::SetNum(NumFmt::Sci(n)))).collect(),
            MenuId::FractionResult => {
                vec![radio("Mixed Number", c.mixed, Act::SetMixed(true)), radio("Improp Fraction", !c.mixed, Act::SetMixed(false))]
            }
            MenuId::FormatKey => vec![
                radio("Decimal", !self.format_key_menu, Act::SetFormatKeyMenu(false)),
                radio("Format Menu", self.format_key_menu, Act::SetFormatKeyMenu(true)),
            ],
            MenuId::Reset => {
                vec![line("Settings & Data", Act::ResetSettings), line("Variable Memory", Act::ResetVariables), line("Initialize All", Act::ResetAll)]
            }
            MenuId::Catalog => {
                ["Func Analysis", "Probability", "Numeric Calc", "Angle/Coord/Sexa", "Hyperbolic/Trig", "Sci Constants", "Unit Conversions", "Other"]
                    .iter()
                    .enumerate()
                    .map(|(i, n)| match i {
                        5 => sub(n, MenuId::ConstCategories),
                        6 => sub(n, MenuId::ConvCategories),
                        _ => sub(n, MenuId::CatalogCategory(i as u8)),
                    })
                    .collect()
            }
            MenuId::CatalogCategory(0) => vec![
                keys("Derivative(d/dx)", &[Key::Deriv]),
                keys("Integration(\u{222B})", &[Key::Integral]),
                keys("Summation(\u{03A3})", &[Key::Sum]),
                keys("Logarithm(log\u{2090}b)", &[Key::LogBase]),
                keys("Logarithm(log)", &[Key::Func(Func::Log)]),
                keys("Natural Logarithm", &[Key::Func(Func::Ln)]),
            ],
            MenuId::CatalogCategory(1) => vec![
                keys("%", &[Key::Percent]),
                keys("Factorial(!)", &[Key::Fact]),
                keys("Permutation(P)", &[Key::Npr]),
                keys("Combination(C)", &[Key::Ncr]),
                keys("Random Number", &[Key::Ran]),
                keys("Random Integer", &[Key::Func(Func::RanInt)]),
            ],
            MenuId::CatalogCategory(2) => vec![
                keys("GCD", &[Key::Func(Func::Gcd)]),
                keys("LCM", &[Key::Func(Func::Lcm)]),
                keys("Absolute Value", &[Key::Func(Func::Abs)]),
                keys("Round Off", &[Key::Func(Func::Rnd)]),
            ],
            MenuId::CatalogCategory(3) => vec![
                keys("Degrees", &[Key::AngD]),
                keys("Radians", &[Key::AngR]),
                keys("Gradians", &[Key::AngG]),
                keys("Rect to Polar", &[Key::Func(Func::Pol)]),
                keys("Polar to Rect", &[Key::Func(Func::Rec)]),
                keys("Degs Mins Secs", &[Key::Dms]),
            ],
            MenuId::CatalogCategory(4) => [
                Func::Sinh,
                Func::Cosh,
                Func::Tanh,
                Func::ASinh,
                Func::ACosh,
                Func::ATanh,
                Func::Sin,
                Func::Cos,
                Func::Tan,
                Func::ASin,
                Func::ACos,
                Func::ATan,
            ]
            .iter()
            .map(|f| keys(f.text().trim_end_matches('('), &[Key::Func(*f)]))
            .collect(),
            MenuId::CatalogCategory(_) => vec![
                keys("Ans", &[Key::Ans]),
                keys("\u{03C0}", &[Key::Pi]),
                keys("e", &[Key::E]),
                keys("\u{221A}(", &[Key::Sqrt]),
                keys("\u{02E3}\u{221A}(", &[Key::NRoot]),
                keys("\u{207B}\u{00B9}", &[Key::Inverse]),
                keys("\u{00B2}", &[Key::Square]),
                keys("^(", &[Key::Pow]),
                keys("(\u{2212})", &[Key::Neg]),
                keys(",", &[Key::Comma]),
                keys("(", &[Key::LParen]),
                keys(")", &[Key::RParen]),
            ],
            MenuId::ConstCategories => {
                category_names(CONSTANTS.iter().map(|c| c.0)).into_iter().enumerate().map(|(i, n)| sub(n, MenuId::ConstCategory(i as u8))).collect()
            }
            MenuId::ConstCategory(k) => {
                let cat = category_names(CONSTANTS.iter().map(|c| c.0)).get(usize::from(k)).copied().unwrap_or("");
                CONSTANTS.iter().enumerate().filter(|(_, c)| c.0 == cat).map(|(i, c)| keys(c.1, &[Key::Const(i as u8)])).collect()
            }
            MenuId::ConvCategories => {
                category_names(CONVERSIONS.iter().map(|c| c.0)).into_iter().enumerate().map(|(i, n)| sub(n, MenuId::ConvCategory(i as u8))).collect()
            }
            MenuId::ConvCategory(k) => {
                let cat = category_names(CONVERSIONS.iter().map(|c| c.0)).get(usize::from(k)).copied().unwrap_or("");
                CONVERSIONS.iter().enumerate().filter(|(_, c)| c.0 == cat).map(|(i, c)| keys(c.1, &[Key::Conv(i as u8)])).collect()
            }
            MenuId::Tools => vec![line("Undo", Act::Undo), line("Clear History", Act::ClearHistory)],
            MenuId::Format => [
                ("Standard", ResultFormat::Standard),
                ("Decimal", ResultFormat::Decimal),
                ("Prime Factor", ResultFormat::PrimeFactor),
                ("Improper Fraction", ResultFormat::ImproperFraction),
                ("Mixed Number", ResultFormat::MixedNumber),
                ("Standard Form", ResultFormat::StandardForm),
                ("Sexagesimal", ResultFormat::Sexagesimal),
            ]
            .iter()
            .map(|(n, f)| radio(n, self.calc.format == *f, Act::SetFormat(*f)))
            .collect(),
            MenuId::Function => vec![
                line("Define f(x)", Act::DefineFn(0)),
                line("Define g(x)", Act::DefineFn(1)),
                keys("f(x)", &[Key::Func(Func::F)]),
                keys("g(x)", &[Key::Func(Func::G)]),
            ],
            MenuId::VarAction(i) => vec![line("Store", Act::Store(i)), line("Recall", Act::Recall(i)), line("Edit", Act::EditVar(i))],
        }
    }

    fn overlay_key(&mut self, key: Key) {
        match self.stack.last_mut() {
            Some(Overlay::Menu(m)) => {
                let n = m.items.len();
                match key {
                    Key::Up => m.sel = (m.sel + n - 1) % n,
                    Key::Down => m.sel = (m.sel + 1) % n,
                    Key::PageUp => m.sel = m.sel.saturating_sub(4),
                    Key::PageDown => m.sel = (m.sel + 4).min(n - 1),
                    Key::Ok | Key::Equals | Key::Right => {
                        if let Some(it) = m.items.get(m.sel) {
                            let act = it.act.clone();
                            self.act(act);
                        }
                    }
                    Key::Digit(d) => {
                        let i = usize::from(d).wrapping_sub(1);
                        if let Some(it) = m.items.get(i) {
                            m.sel = i;
                            let act = it.act.clone();
                            self.act(act);
                        }
                    }
                    Key::Back | Key::Left | Key::Ac => {
                        self.stack.pop();
                    }
                    Key::Home => {
                        self.stack.clear();
                        self.app = App::Home;
                    }
                    _ => {}
                }
            }
            Some(Overlay::Variables { sel }) => match key {
                Key::Up => *sel = (*sel + 8) % 9,
                Key::Down => *sel = (*sel + 1) % 9,
                Key::PageUp => *sel = sel.saturating_sub(4),
                Key::PageDown => *sel = (*sel + 4).min(8),
                Key::Ok | Key::Equals => {
                    let i = *sel as u8;
                    self.open_menu(MenuId::VarAction(i));
                }
                Key::Back | Key::Variable | Key::Ac => {
                    self.stack.pop();
                }
                Key::Home => {
                    self.stack.clear();
                    self.app = App::Home;
                }
                _ => {}
            },
            Some(Overlay::Prompt(p)) => match key {
                Key::Back | Key::Ac if p.field.tokens.is_empty() || key == Key::Back => {
                    self.stack.pop();
                }
                Key::Ok | Key::Equals => {
                    let Prompt { field, then, .. } = p.clone();
                    match (then, field.value(None)) {
                        (Then::SetVar(i), Ok(v)) => {
                            if let Some(slot) = self.mem.vars.get_mut(usize::from(i)) {
                                *slot = v;
                            }
                            self.stack.pop();
                        }
                        (Then::DefineFn(i), _) if !field.tokens.is_empty() => {
                            if let Some(slot) = self.mem.defs.get_mut(usize::from(i)) {
                                *slot = field.tokens.clone();
                            }
                            self.stack.pop();
                        }
                        _ => {}
                    }
                }
                Key::Up
                | Key::Down
                | Key::Home
                | Key::Settings
                | Key::Catalog
                | Key::Tools
                | Key::Variable
                | Key::Function
                | Key::FormatMenu
                | Key::Sd
                | Key::Off
                | Key::On => {}
                other => p.field.press(other),
            },
            None => {}
        }
        self.sync();
    }

    fn close_all(&mut self) {
        self.stack.clear();
    }

    fn act(&mut self, act: Act) {
        let back = |d: &mut Device| {
            // Back to the list the choice was made in, redrawn with the new radio button.
            let Some(Overlay::Menu(m)) = d.stack.pop() else { return };
            let sel = m.sel;
            d.open_menu(m.id);
            if let Some(Overlay::Menu(n)) = d.stack.last_mut() {
                n.sel = sel.min(n.items.len().saturating_sub(1));
            }
        };
        match act {
            Act::Open(id) => self.open_menu(id),
            Act::Keys(ks) => {
                self.close_all();
                for k in ks {
                    self.calc.press(k);
                }
            }
            Act::SetIo(io) => {
                self.cfg.io = io;
                back(self);
            }
            Act::SetAngle(a) => {
                self.calc.angle = a;
                self.table.set_angle(a);
                if let Some((_, f)) = &mut self.equation {
                    f.set_angle(a);
                }
                back(self);
            }
            Act::SetNum(n) => {
                self.cfg.num = n;
                // The digits list sits one level down from the Number Format list.
                if matches!(self.menu().map(|m| m.id), Some(MenuId::FixDigits | MenuId::SciDigits)) {
                    self.stack.pop();
                }
                back(self);
            }
            Act::SetMixed(m) => {
                self.cfg.mixed = m;
                back(self);
            }
            Act::SetExpPower(v) => {
                self.cfg.exp_power = v;
                back(self);
            }
            Act::SetCommaMark(v) => {
                self.cfg.comma_mark = v;
                back(self);
            }
            Act::SetSeparator(v) => {
                self.cfg.separator = v;
                back(self);
            }
            Act::SetFormatKeyMenu(m) => {
                self.format_key_menu = m;
                back(self);
            }
            Act::SetOptionNumbers(o) => {
                self.option_numbers = o;
                back(self);
            }
            Act::SetFormat(f) => {
                self.close_all();
                self.calc.press(Key::SetFormat(f));
            }
            Act::Undo => {
                self.close_all();
                self.calc.undo();
            }
            Act::ClearHistory => {
                self.close_all();
                self.calc.clear_history();
            }
            Act::Store(i) => {
                if let Some(slot) = self.mem.vars.get_mut(usize::from(i)) {
                    *slot = self.calc.answer();
                }
                // Back to the variable list.
                self.stack.pop();
            }
            Act::Recall(i) => {
                self.close_all();
                self.calc.press(Key::Var(i));
            }
            Act::EditVar(i) => {
                let label = format!("{}=", ["A", "B", "C", "D", "E", "F", "x", "y", "z"].get(usize::from(i)).copied().unwrap_or("?"));
                self.stack.pop();
                self.stack.push(Overlay::Prompt(Prompt { label, field: Calc::blank_like(&self.calc), then: Then::SetVar(i) }));
            }
            Act::DefineFn(i) => {
                let mut field = Calc::blank_like(&self.calc);
                for t in self.mem.defs.get(usize::from(i)).cloned().unwrap_or_default() {
                    field.tokens.push(t);
                }
                field.cursor = field.tokens.len();
                self.stack.pop();
                self.stack.push(Overlay::Prompt(Prompt { label: format!("{}(x)=", if i == 0 { "f" } else { "g" }), field, then: Then::DefineFn(i) }));
            }
            Act::ResetSettings => {
                self.cfg = Config::default();
                self.calc.angle = Angle::Degree;
                self.format_key_menu = false;
                self.close_all();
            }
            Act::ResetVariables => {
                self.mem = Memory { defs: self.mem.defs.clone(), ..Memory::default() };
                self.close_all();
            }
            Act::ResetAll => {
                let angle = Angle::Degree;
                *self = Device { app: App::Home, ..Device::default() };
                self.calc.angle = angle;
            }
        }
        self.sync();
    }

    fn open(&mut self, i: usize) {
        self.home_sel = i;
        match HOME_APPS.get(i).and_then(|m| m.1) {
            Some(app) => {
                self.app = app;
                self.note = None;
                if app == App::Equation {
                    self.equation = None;
                }
            }
            None => self.note = Some("Not in this emulator yet"),
        }
    }

    fn home_key(&mut self, key: Key) {
        let n = HOME_APPS.len();
        match key {
            Key::Left => self.home_sel = (self.home_sel + n - 1) % n,
            Key::Right => self.home_sel = (self.home_sel + 1) % n,
            Key::Up => self.home_sel = if self.home_sel >= 3 { self.home_sel - 3 } else { (self.home_sel + n - (n % 3)).min(n - 1) },
            Key::Down => self.home_sel = if self.home_sel + 3 < n { self.home_sel + 3 } else { self.home_sel % 3 },
            Key::PageUp => self.home_sel = self.home_sel.saturating_sub(6),
            Key::PageDown => self.home_sel = (self.home_sel + 6).min(n - 1),
            Key::Ok | Key::Equals => self.open(self.home_sel),
            Key::Digit(d @ 1..=6) => self.open(usize::from(d) - 1),
            _ => {}
        }
    }

    fn equation_key(&mut self, key: Key) {
        let Some((kind, form)) = &mut self.equation else {
            match key {
                Key::Up | Key::Down => self.eq_sel = (self.eq_sel + 1) % EQUATIONS.len(),
                Key::Ok | Key::Equals => self.start_equation(self.eq_sel),
                Key::Digit(d @ 1..=2) => self.start_equation(usize::from(d) - 1),
                _ => {}
            }
            return;
        };
        let solve = if *kind == 0 { solve_simultaneous } else { solve_quadratic };
        form.press(key, solve);
    }

    fn start_equation(&mut self, kind: usize) {
        self.eq_sel = kind;
        let d = self.calc.angle;
        self.equation = Some((
            kind,
            if kind == 0 {
                Form::new("a1x+b1y=c1, a2x+b2y=c2", &["a1", "b1", "c1", "a2", "b2", "c2"], &["", "", "", "", "", ""], d)
            } else {
                Form::new("ax\u{00B2}+bx+c=0", &["a", "b", "c"], &["", "", ""], d)
            },
        ));
        self.sync();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculator::Func;

    fn type_into(d: &mut Device, text: &str) {
        for ch in text.chars() {
            d.press(match ch {
                '.' => Key::Dot,
                '-' => Key::Neg,
                'x' => Key::X,
                '+' => Key::Plus,
                '*' => Key::Times,
                d => Key::Digit(d as u8 - b'0'),
            });
        }
    }

    fn rows(f: &Form) -> Vec<(String, String)> {
        f.rows.iter().flatten().map(|r| (r.label.clone(), tokens_text(&r.value))).collect()
    }

    #[test]
    fn the_home_menu_opens_the_apps_that_exist_and_says_so_for_those_that_do_not() {
        let mut d = Device::default();
        d.press(Key::Home);
        assert_eq!(d.app, App::Home);
        d.press(Key::Digit(3));
        assert_eq!(d.app, App::Table);
        d.press(Key::Home);
        d.press(Key::Digit(2));
        assert_eq!(d.app, App::Home);
        assert_eq!(d.note, Some("Not in this emulator yet"));
        d.press(Key::Right);
        d.press(Key::Right);
        d.press(Key::Ok);
        assert_eq!(d.app, App::Equation);
        d.press(Key::Home);
        d.press(Key::Down);
        assert_eq!(d.home_sel, 6, "Down moves a row; from the second row it goes to the next page");
    }

    #[test]
    fn calculate_still_works_through_the_device_and_shift_picks_the_second_function() {
        let mut d = Device::default();
        d.tap(Key::Shift, None);
        d.tap(Key::Func(Func::Sin), Some(Key::Func(Func::ASin)));
        assert_eq!(d.calc.tokens, vec![Tok::Func(Func::ASin)]);
        assert!(!d.shift);
    }

    #[test]
    fn a_table_lists_f_of_x_for_each_step() {
        let mut d = Device::default();
        d.press(Key::Home);
        d.press(Key::Digit(3));
        type_into(&mut d, "2x+1");
        for _ in 0..4 {
            d.press(Key::Ok);
        }
        let got = rows(&d.table);
        assert_eq!(got.first(), Some(&("1".to_string(), "3".to_string())));
        assert_eq!(got.len(), 5);
        assert_eq!(got.last(), Some(&("5".to_string(), "11".to_string())));
    }

    #[test]
    fn a_table_with_no_function_is_a_syntax_error() {
        let mut d = Device::default();
        d.press(Key::Home);
        d.press(Key::Digit(3));
        for _ in 0..4 {
            d.press(Key::Ok);
        }
        assert_eq!(d.table.rows.as_ref().map(|r| r[0].label.as_str()), Some("Syntax ERROR"));
    }

    #[test]
    fn simultaneous_equations_solve_exactly() {
        let mut d = Device::default();
        d.press(Key::Home);
        d.press(Key::Digit(4));
        d.press(Key::Ok); // simultaneous
        // 2x + 4y = 3 and 6x - 2y = 1 give x = 5/14, y = 4/7
        for v in ["2", "4", "3", "6", "-2", "1"] {
            type_into(&mut d, v);
            d.press(Key::Ok);
        }
        let f = &d.equation.as_ref().unwrap().1;
        let got = rows(f);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, "x");
        let rat = |n, d| value_tokens(Value::Rat(crate::calculator::Rat::new(n, d).unwrap()), false);
        assert_eq!(f.rows.as_ref().unwrap()[0].value, rat(5, 14));
        assert_eq!(f.rows.as_ref().unwrap()[1].value, rat(4, 7));
    }

    #[test]
    fn a_quadratic_gives_both_roots_in_order_and_one_when_they_meet() {
        let mut d = Device::default();
        d.press(Key::Home);
        d.press(Key::Digit(4));
        d.press(Key::Digit(2));
        for v in ["1", "-5", "6"] {
            type_into(&mut d, v);
            d.press(Key::Ok);
        }
        assert_eq!(rows(&d.equation.as_ref().unwrap().1), vec![("x1".into(), "2".into()), ("x2".into(), "3".into())]);
        // x^2 - 2x + 1 = 0
        d.press(Key::Ok);
        d.press(Key::Home);
        d.press(Key::Digit(4));
        d.press(Key::Digit(2));
        for v in ["1", "-2", "1"] {
            type_into(&mut d, v);
            d.press(Key::Ok);
        }
        assert_eq!(rows(&d.equation.as_ref().unwrap().1), vec![("x".into(), "1".into())]);
    }

    #[test]
    fn parallel_lines_and_a_zero_a_are_math_errors() {
        let mut d = Device::default();
        d.press(Key::Home);
        d.press(Key::Digit(4));
        d.press(Key::Digit(1));
        for v in ["1", "1", "1", "1", "1", "2"] {
            type_into(&mut d, v);
            d.press(Key::Ok);
        }
        assert_eq!(d.equation.as_ref().unwrap().1.rows.as_ref().unwrap()[0].label, "Math ERROR");
    }

    // ---- the real key layout, menus and memory ----

    fn press_all(d: &mut Device, keys: &[Key]) {
        for k in keys {
            d.press(*k);
        }
    }

    fn answer_text(d: &Device) -> String {
        match d.calc.outcome {
            Outcome::Answer(v) => tokens_text(&crate::calculator::result_tokens(v, d.calc.format, &d.calc.cfg)),
            Outcome::Error(e) => e.message().into(),
            Outcome::Nothing => "-".into(),
        }
    }

    fn pick(d: &mut Device, path: &[u8]) {
        for n in path {
            d.press(Key::Digit(*n));
        }
    }

    #[test]
    fn the_settings_key_opens_a_menu_and_choosing_an_angle_unit_changes_it() {
        let mut d = Device::default();
        d.press(Key::Settings);
        assert_eq!(d.menu().map(|m| m.id), Some(MenuId::Settings));
        pick(&mut d, &[1, 2]); // Calc Settings > Angle Unit
        assert_eq!(d.menu().map(|m| m.id), Some(MenuId::AngleUnit));
        d.press(Key::Digit(3)); // Gradian
        assert_eq!(d.angle(), Angle::Gradian);
        assert_eq!(d.menu().map(|m| m.id), Some(MenuId::AngleUnit), "it stays on the list with the new radio button");
        d.press(Key::Back);
        d.press(Key::Back);
        d.press(Key::Back);
        assert!(d.stack.is_empty(), "back closes the levels one at a time");
        press_all(&mut d, &[Key::Func(Func::Sin), Key::Digit(1), Key::Digit(0), Key::Digit(0), Key::Equals]);
        assert_eq!(answer_text(&d), "1", "sin 100 gradians is 1");
    }

    #[test]
    fn number_format_fix_and_sci_change_how_decimals_are_shown() {
        let mut d = Device::default();
        d.press(Key::Settings);
        pick(&mut d, &[1, 3, 1, 3]); // Calc Settings > Number Format > Fix > Fix 2
        assert_eq!(d.cfg.num, NumFmt::Fix(2));
        d.press(Key::Home);
        d.press(Key::Digit(1));
        press_all(&mut d, &[Key::Digit(1), Key::Divide, Key::Digit(6), Key::Equals, Key::Sd]);
        assert_eq!(answer_text(&d), "0.17");
        d.press(Key::Settings);
        pick(&mut d, &[1, 3, 2, 3]); // Sci 3
        assert_eq!(d.cfg.num, NumFmt::Sci(3));
        d.press(Key::Equals);
        d.press(Key::Sd);
        assert_eq!(answer_text(&d), "1.67\u{00D7}10\u{2212}1", "three significant figures, in standard form");
    }

    #[test]
    fn decimal_output_and_linear_output_are_settings() {
        let mut d = Device::default();
        d.cfg.io = Io::MathDecimal;
        press_all(&mut d, &[Key::Digit(1), Key::Divide, Key::Digit(4), Key::Equals]);
        assert_eq!(answer_text(&d), "0.25");
        d.cfg.io = Io::LineLine;
        d.press(Key::Equals);
        d.press(Key::AngleMode);
        press_all(&mut d, &[Key::Ac, Key::Digit(1), Key::Divide, Key::Digit(4), Key::Equals]);
        assert_eq!(answer_text(&d), "1/4", "one line, no stacked fraction");
    }

    #[test]
    fn the_format_key_toggles_and_shift_format_opens_the_menu() {
        let mut d = Device::default();
        press_all(&mut d, &[Key::Digit(3), Key::Divide, Key::Digit(2), Key::Equals]);
        assert_eq!(answer_text(&d), "32", "3/2 as a stacked fraction (flat text)");
        d.tap(Key::Sd, Some(Key::FormatMenu));
        assert_eq!(answer_text(&d), "1.5");
        d.tap(Key::Shift, None);
        d.tap(Key::Sd, Some(Key::FormatMenu));
        assert_eq!(d.menu().map(|m| m.id), Some(MenuId::Format));
        d.press(Key::Digit(5)); // Mixed Number
        assert_eq!(answer_text(&d), "112", "1 and 1/2");
        d.tap(Key::Shift, None);
        d.tap(Key::Sd, Some(Key::FormatMenu));
        d.press(Key::Digit(6)); // Standard Form
        assert_eq!(answer_text(&d), "1.5\u{00D7}100", "a x 10^0 (the exponent is a template)");
    }

    #[test]
    fn prime_factors_and_exe_decimal() {
        let mut d = Device::default();
        press_all(&mut d, &[Key::Digit(6), Key::Digit(0), Key::Equals]);
        d.tap(Key::Shift, None);
        d.tap(Key::Sd, Some(Key::FormatMenu));
        d.press(Key::Digit(3));
        assert_eq!(answer_text(&d), "22\u{00D7}3\u{00D7}5", "2 squared (the exponent is a template), times 3, times 5");
        press_all(&mut d, &[Key::Ac, Key::Digit(1), Key::Divide, Key::Digit(8)]);
        d.press(Key::ExeDecimal);
        assert_eq!(answer_text(&d), "0.125", "SHIFT EXE gives the decimal straight away");
    }

    #[test]
    fn variables_are_stored_from_the_variable_list_and_used_in_sums() {
        let mut d = Device::default();
        press_all(&mut d, &[Key::Digit(3), Key::Plus, Key::Digit(5), Key::Equals]);
        d.press(Key::Variable);
        assert!(matches!(d.stack.last(), Some(Overlay::Variables { .. })));
        d.press(Key::Ok); // A=
        d.press(Key::Digit(1)); // Store
        assert!(matches!(d.stack.last(), Some(Overlay::Variables { .. })), "back on the variable list, as on the real thing");
        assert_eq!(d.mem.vars[0], Value::int(8));
        press_all(&mut d, &[Key::Ac, Key::Ac]);
        assert!(d.stack.is_empty());
        d.tap(Key::Shift, None);
        d.tap(Key::Digit(4), Some(Key::Var(0))); // SHIFT 4 is A
        press_all(&mut d, &[Key::Times, Key::Digit(1), Key::Digit(0), Key::Equals]);
        assert_eq!(answer_text(&d), "80");
    }

    #[test]
    fn a_variable_can_be_edited_and_recalled() {
        let mut d = Device::default();
        d.press(Key::Variable);
        d.press(Key::Down);
        d.press(Key::Ok); // B=
        d.press(Key::Digit(3)); // Edit
        assert!(matches!(d.stack.last(), Some(Overlay::Prompt(_))));
        press_all(&mut d, &[Key::Digit(7), Key::Ok]);
        assert_eq!(d.mem.vars[1], Value::int(7));
        d.press(Key::Ok); // still on B in the list
        d.press(Key::Digit(2)); // Recall
        assert_eq!(d.calc.tokens, vec![Tok::Var(1)]);
    }

    #[test]
    fn define_f_of_x_and_use_it() {
        let mut d = Device::default();
        d.press(Key::Function);
        d.press(Key::Digit(1)); // Define f(x)
        press_all(&mut d, &[Key::X, Key::Square, Key::Plus, Key::Digit(1), Key::Ok]);
        assert_eq!(d.mem.defs[0].len(), 6);
        d.press(Key::Function);
        d.press(Key::Digit(3)); // f(
        press_all(&mut d, &[Key::Digit(5), Key::RParen, Key::Equals]);
        assert_eq!(answer_text(&d), "26", "f(5) = 5 squared + 1");
        // g(x) = f(x) x 2 - x, as in the manual
        d.press(Key::Function);
        d.press(Key::Digit(2));
        press_all(&mut d, &[Key::Func(Func::F), Key::X, Key::RParen, Key::Times, Key::Digit(2), Key::Minus, Key::X, Key::Ok]);
        press_all(&mut d, &[Key::Ac, Key::Func(Func::G), Key::Digit(3), Key::Equals]);
        assert_eq!(answer_text(&d), "17", "g(3) = f(3) x 2 - 3 = 10 x 2 - 3");
    }

    #[test]
    fn the_catalog_inserts_functions_and_tools_undoes() {
        let mut d = Device::default();
        d.press(Key::Digit(2));
        d.press(Key::Catalog);
        pick(&mut d, &[5, 1]); // Hyperbolic/Trig > sinh
        assert_eq!(d.calc.tokens.last(), Some(&Tok::Func(Func::Sinh)));
        assert!(d.stack.is_empty());
        d.press(Key::Tools);
        d.press(Key::Digit(1)); // Undo
        assert_eq!(d.calc.tokens, vec![Tok::Digit(2)]);
        press_all(&mut d, &[Key::Ac, Key::Digit(2), Key::Digit(0), Key::Percent, Key::Times, Key::Digit(3), Key::Digit(0), Key::Equals]);
        assert_eq!(answer_text(&d), "6", "20% of 30");
    }

    #[test]
    fn off_blanks_the_screen_until_on_and_back_steps_out_of_an_equation() {
        let mut d = Device::default();
        d.tap(Key::Shift, None);
        d.tap(Key::Ac, Some(Key::Off));
        assert!(!d.on);
        d.press(Key::Digit(5));
        assert!(d.calc.tokens.is_empty(), "keys do nothing while it is off");
        d.press(Key::On);
        assert!(d.on);
        d.press(Key::Home);
        d.press(Key::Digit(4));
        d.press(Key::Digit(2));
        assert!(d.equation.is_some());
        d.press(Key::Back);
        assert!(d.equation.is_none(), "back returns to the list of equation types");
    }

    #[test]
    fn log_with_a_base_and_the_mixed_number_template_evaluate() {
        let mut d = Device::default();
        press_all(&mut d, &[Key::LogBase, Key::Digit(2), Key::Right, Key::Digit(8), Key::Equals]);
        assert_eq!(answer_text(&d), "3");
        press_all(&mut d, &[Key::Ac, Key::Mixed, Key::Digit(1), Key::Right, Key::Digit(1), Key::Right, Key::Digit(2), Key::Equals]);
        assert_eq!(answer_text(&d), "32", "1 and 1/2 is 3/2");
    }

    #[test]
    fn reset_clears_variables_but_keeps_the_functions() {
        let mut d = Device::default();
        d.mem.vars[0] = Value::int(9);
        d.mem.defs[0] = vec![Tok::X];
        d.press(Key::Settings);
        pick(&mut d, &[3, 2]); // Reset > Variable Memory
        assert_eq!(d.mem.vars[0], Value::int(0));
        assert_eq!(d.mem.defs[0], vec![Tok::X]);
    }

    // ---- examples from the fx-991CW+ user's guide ----

    fn calc(keys: &[Key]) -> String {
        let mut d = Device::default();
        press_all(&mut d, keys);
        d.press(Key::Equals);
        answer_text(&d)
    }

    /// The answer's tokens with templates marked: `{a/b}` fractions, `√[n]` roots, `^[e]` powers.
    fn shape(d: &Device) -> String {
        let Outcome::Answer(v) = d.calc.outcome else { return "-".into() };
        crate::calculator::result_tokens(v, d.calc.format, &d.calc.cfg)
            .iter()
            .map(|t| match t {
                Tok::FracOpen => "{".into(),
                Tok::FracMid => "/".into(),
                Tok::SqrtOpen => "\u{221A}[".into(),
                Tok::SupOpen => "^[".into(),
                Tok::Close => "]".into(),
                other => other.text(),
            })
            .collect()
    }

    fn run_shape(keys: &[Key]) -> String {
        let mut d = Device::default();
        press_all(&mut d, keys);
        d.press(Key::Equals);
        shape(&d)
    }

    use Key::{Digit as D, Divide, Equals, Minus, Plus, Right, Sqrt, Times};

    #[test]
    fn manual_arithmetic_and_powers() {
        assert_eq!(calc(&[D(7), Times, D(8), Minus, D(4), Times, D(5)]), "36");
        // (5²)³ = 15625
        assert_eq!(calc(&[Key::LParen, D(5), Key::Square, Key::RParen, Key::Pow, D(3)]), "15,625");
        // (1 + 1)^(2+2) = 16
        assert_eq!(calc(&[Key::LParen, D(1), Plus, D(1), Key::RParen, Key::Pow, D(2), Plus, D(2)]), "16");
        // 5th root of 32 = 2
        assert_eq!(calc(&[Key::NRoot, D(5), Right, D(3), D(2)]), "2");
        // 10^-1 = 1/10
        assert_eq!(run_shape(&[D(1), D(0), Key::Inverse]), "{1/10]");
    }

    #[test]
    fn manual_surds_stay_exact_within_the_display_ranges() {
        // √2 × 3 = 3√2
        assert_eq!(run_shape(&[Sqrt, D(2), Right, Times, D(3)]), "3\u{221A}[2]");
        // 10√2 + 15 × 3√3 = 45√3 + 10√2
        let keys = [D(1), D(0), Sqrt, D(2), Right, Plus, D(1), D(5), Times, D(3), Sqrt, D(3)];
        assert_eq!(run_shape(&keys), "45\u{221A}[3]+10\u{221A}[2]");
        // 99√999 (= 297√111) is out of range: 3129.089165
        assert_eq!(calc(&[D(9), D(9), Sqrt, D(9), D(9), D(9)]), "3,129.089165");
        // 1/3 + √12 = 2√3 + 1/3
        assert_eq!(run_shape(&[Key::Frac, D(1), Right, D(3), Right, Plus, Sqrt, D(1), D(2)]), "2\u{221A}[3]+{1/3]");
        // 1 ÷ (1 + √2) rationalises to √2 − 1
        assert_eq!(
            run_shape(&[D(1), Divide, Key::LParen, D(1), Plus, Sqrt, D(2), Right, Key::RParen]),
            "\u{2212}1+\u{221A}[2]",
            "a whole number goes first, as d ± a√b"
        );
    }

    #[test]
    fn manual_pi_answers_keep_pi() {
        // π ÷ 6 = 1/6 π
        let mut d = Device::default();
        press_all(&mut d, &[Key::Pi, Divide, D(6), Equals]);
        assert_eq!(shape(&d), "{1/6]\u{03C0}");
        d.press(Key::Sd);
        assert_eq!(answer_text(&d), "0.5235987756");
        assert_eq!(run_shape(&[D(2), Key::Pi]), "2\u{03C0}");
        // sin⁻¹(1) in radians is π/2
        let mut d = Device::default();
        d.calc.angle = Angle::Radian;
        press_all(&mut d, &[Key::Func(Func::ASin), D(1), Equals]);
        assert_eq!(shape(&d), "{1/2]\u{03C0}");
    }

    #[test]
    fn manual_fractions() {
        // 2/3 + 1 1/2 = 13/6
        let keys = [Key::Frac, D(2), Right, D(3), Right, Plus, Key::Mixed, D(1), Right, D(1), Right, D(2)];
        assert_eq!(run_shape(&keys), "{13/6]");
        // 1 1/123456 fits in ten digits as a fraction; 1 1/1234567 does not.
        let mut d = Device::default();
        press_all(&mut d, &[Key::Mixed, D(1), Right, D(1), Right, D(1), D(2), D(3), D(4), D(5), D(6), Equals]);
        assert_eq!(shape(&d), "{123,457/123,456]");
        press_all(&mut d, &[Key::Ac, Key::Mixed, D(1), Right, D(1), Right, D(1), D(2), D(3), D(4), D(5), D(6), D(7), Equals]);
        assert_eq!(answer_text(&d), "1.00000081");
    }

    #[test]
    fn manual_times_ten_binds_as_one_number() {
        // 4 × 10⁷ ÷ 3 × 10⁸ = 2/15 with the Sci Notat key (the default)
        let keys = [D(4), Key::Exp10, D(7), Right, Divide, D(3), Key::Exp10, D(8)];
        assert_eq!(run_shape(&keys), "{2/15]");
        // With the Power setting the same keys work left to right.
        let mut d = Device::default();
        d.cfg.exp_power = true;
        press_all(&mut d, &keys);
        d.press(Equals);
        assert_eq!(answer_text(&d), "1.333333333\u{00D7}1015", "4×10⁷ ÷ 3, then × 10⁸: left to right");
    }

    #[test]
    fn manual_probability_and_numeric_functions() {
        assert_eq!(calc(&[D(1), D(5), D(0), Times, D(2), D(0), Key::Percent]), "30");
        assert_eq!(calc(&[Key::LParen, D(5), Plus, D(3), Key::RParen, Key::Fact]), "40,320");
        assert_eq!(calc(&[D(1), D(0), Key::Npr, D(4)]), "5,040");
        assert_eq!(calc(&[D(1), D(0), Key::Ncr, D(4)]), "210");
        assert_eq!(calc(&[Key::Func(Func::Gcd), D(2), D(8), Key::Comma, D(3), D(5), Key::RParen]), "7");
        assert_eq!(calc(&[Key::Func(Func::Lcm), D(9), Key::Comma, D(1), D(5), Key::RParen]), "45");
        assert_eq!(calc(&[Key::Func(Func::Abs), D(2), Minus, D(7), Key::RParen]), "5");
        for _ in 0..20 {
            let r = calc(&[Key::Func(Func::RanInt), D(1), Key::Comma, D(6), Key::RParen]);
            assert!(["1", "2", "3", "4", "5", "6"].contains(&r.as_str()), "{r}");
        }
        assert_eq!(calc(&[Key::Func(Func::Gcd), D(2), D(8), Key::RParen]), "Syntax ERROR", "GCD needs two arguments");
    }

    #[test]
    fn manual_round_off_follows_the_number_format() {
        let mut d = Device::default();
        d.cfg.num = NumFmt::Fix(3);
        d.cfg.io = Io::MathDecimal;
        press_all(&mut d, &[D(1), D(0), Divide, D(3), Times, D(3), Equals]);
        assert_eq!(answer_text(&d), "10.000");
        press_all(&mut d, &[Key::Ac, Key::Func(Func::Rnd), D(1), D(0), Divide, D(3), Key::RParen, Times, D(3), Equals]);
        assert_eq!(answer_text(&d), "9.999");
    }

    #[test]
    fn manual_angles_sexagesimal_and_coordinates() {
        // π/2 radians = 90° (Angle Unit: Degree)
        assert_eq!(calc(&[Key::LParen, Key::Pi, Divide, D(2), Key::RParen, Key::AngR]), "90");
        // 2°20′30″ + 0°39′30″ = 3°0′0″
        let mut d = Device::default();
        press_all(
            &mut d,
            &[D(2), Key::Dms, D(2), D(0), Key::Dms, D(3), D(0), Key::Dms, Plus, D(0), Key::Dms, D(3), D(9), Key::Dms, D(3), D(0), Key::Dms, Equals],
        );
        assert_eq!(answer_text(&d), "3");
        d.press(Key::SetFormat(ResultFormat::Sexagesimal));
        assert_eq!(answer_text(&d), "3\u{00B0}0\u{2032}0\u{2033}");
        // Pol(√2, √2): r = 2, θ = 45, stored in x and y
        let mut d = Device::default();
        press_all(&mut d, &[Key::Func(Func::Pol), Sqrt, D(2), Right, Key::Comma, Sqrt, D(2), Right, Key::RParen, Equals]);
        assert_eq!(d.calc.pair, Some((Value::int(2), Value::int(45), true)));
        assert_eq!((d.mem.vars[6], d.mem.vars[7]), (Value::int(2), Value::int(45)));
        // Rec(√2, 45) = (1, 1)
        let mut d = Device::default();
        press_all(&mut d, &[Key::Func(Func::Rec), Sqrt, D(2), Right, Key::Comma, D(4), D(5), Key::RParen, Equals]);
        assert_eq!(d.calc.pair.map(|(a, b, _)| (a, b)), Some((Value::int(1), Value::int(1))));
    }

    #[test]
    fn manual_calculus_functions() {
        // d/dx(sin x) at π/2 in radians is 0
        let mut d = Device::default();
        d.calc.angle = Angle::Radian;
        press_all(&mut d, &[Key::Deriv, Key::Func(Func::Sin), Key::X, Key::RParen, Right, Key::Pi, Divide, D(2), Equals]);
        assert_eq!(answer_text(&d), "0");
        // d/dx(x³) at 2 is 12
        assert_eq!(calc(&[Key::Deriv, Key::X, Key::Cube, Right, D(2)]), "12");
        // ∫ ln x dx from 1 to e = 1
        assert_eq!(calc(&[Key::Integral, Key::Func(Func::Ln), Key::X, Key::RParen, Right, D(1), Right, Key::E]), "1");
        // Σ (x + 1) for x = 1 to 5 is 20
        assert_eq!(calc(&[Key::Sum, Key::X, Plus, D(1), Right, D(1), Right, D(5)]), "20");
    }

    #[test]
    fn constants_and_conversions_come_from_the_catalog() {
        let mut d = Device::default();
        d.press(Key::Catalog);
        pick(&mut d, &[6, 1, 3]); // Sci Constants > Universal > c
        d.press(Equals);
        assert_eq!(answer_text(&d), "299,792,458");
        // 5 cm into inches
        let mut d = Device::default();
        d.press(D(5));
        d.press(Key::Catalog);
        pick(&mut d, &[7, 1, 2]); // Unit Conversions > Length > cm→in
        d.press(Key::ExeDecimal);
        assert_eq!(answer_text(&d), "1.968503937");
        // 100 °C in °F
        let mut d = Device::default();
        press_all(&mut d, &[D(1), D(0), D(0)]);
        d.press(Key::Catalog);
        pick(&mut d, &[7, 9, 2]);
        d.press(Equals);
        assert_eq!(answer_text(&d), "212");
    }

    #[test]
    fn ins_makes_the_next_template_take_what_follows() {
        // Type 1 + 7/6, put the cursor before the fraction, INS, √: 1 + √(7/6)
        let mut d = Device::default();
        press_all(&mut d, &[D(1), Plus, Key::Frac, D(7), Right, D(6), Right, Key::Left, Key::Left, Key::Left, Key::Left, Key::Left]);
        assert_eq!(d.calc.cursor, 2);
        d.tap(Key::Shift, None);
        d.tap(Key::Del, Some(Key::Ins));
        d.press(Sqrt);
        assert_eq!(d.calc.tokens[2], Tok::SqrtOpen);
        assert_eq!(d.calc.tokens[3], Tok::FracOpen);
        assert_eq!(d.calc.tokens.last(), Some(&Tok::Close));
        assert_eq!(d.calc.tokens.len(), 9, "1 + √( 7/6 )");
    }

    #[test]
    fn digit_separator_and_decimal_mark_are_settings() {
        let mut d = Device::default();
        press_all(&mut d, &[D(1), D(2), D(3), D(4), D(5), Key::Dot, D(5), Equals]);
        assert_eq!(answer_text(&d), "12,345.5");
        d.cfg.separator = false;
        d.cfg.comma_mark = true;
        d.press(Equals);
        assert_eq!(answer_text(&d), "12345,5");
    }

    #[test]
    fn qr_and_i_say_they_are_not_here() {
        let mut d = Device::default();
        d.tap(Key::Shift, None);
        d.tap(Key::X, Some(Key::Qr));
        assert!(d.note.is_some());
        assert!(d.calc.tokens.is_empty());
        d.press(D(1));
        assert!(d.note.is_none(), "the note goes with the next key");
    }
}
