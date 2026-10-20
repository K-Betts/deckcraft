//! The Spreadsheet app: a grid of cells, columns A to E and rows 1 to 45. A cell holds a number
//! or a formula that can use other cells (`A1+B2`) and `Sum(A1:A5)`, `Mean(`, `Min(`, `Max(`.
#![allow(clippy::should_implement_trait, clippy::needless_range_loop)]

use std::collections::BTreeMap;

use crate::calculator::{Calc, CalcError, Env, Key, NumFmt, Tok, Value, evaluate_env};

pub const COLS: u8 = 5;
pub const ROWS: u32 = 45;

#[derive(Clone)]
pub struct Sheet {
    pub cells: BTreeMap<(u8, u32), Vec<Tok>>,
    pub cursor: (u8, u32),
    /// The cell being typed into, if any.
    pub edit: Option<Calc>,
}

impl Default for Sheet {
    fn default() -> Self {
        Sheet { cells: BTreeMap::new(), cursor: (0, 1), edit: None }
    }
}

fn cell_value(sheet: &Sheet, col: u8, row: u32, like: &Calc, depth: u32) -> Result<Value, CalcError> {
    if col >= COLS || !(1..=ROWS).contains(&row) {
        return Err(CalcError::Syntax);
    }
    // A cell that refers back to itself, however indirectly, is a math error.
    if depth > 60 {
        return Err(CalcError::Math);
    }
    let Some(toks) = sheet.cells.get(&(col, row)).filter(|t| !t.is_empty()) else { return Ok(Value::int(0)) };
    let body: &[Tok] = if toks.first() == Some(&Tok::EqSign) { &toks[1..] } else { toks };
    let lookup = |c: u8, r: u32| cell_value(sheet, c, r, like, depth + 1);
    let env = Env { ans: Value::int(0), angle: like.angle, x: None, mem: &like.mem, num: NumFmt::Norm(2), cells: Some(&lookup) };
    evaluate_env(body, &env)
}

impl Sheet {
    /// The value of a cell (0 when empty).
    pub fn value(&self, col: u8, row: u32, like: &Calc) -> Result<Value, CalcError> {
        cell_value(self, col, row, like, 0)
    }

    /// What the cell shows: its value, or nothing when it is empty.
    pub fn is_empty(&self, col: u8, row: u32) -> bool {
        self.cells.get(&(col, row)).is_none_or(Vec::is_empty)
    }

    /// The cell cursor, kept inside the grid.
    fn step(&mut self, dc: i32, dr: i32) {
        let c = (i32::from(self.cursor.0) + dc).clamp(0, i32::from(COLS) - 1) as u8;
        let r = (self.cursor.1 as i32 + dr).clamp(1, ROWS as i32) as u32;
        self.cursor = (c, r);
    }

    pub fn commit(&mut self) {
        if let Some(e) = self.edit.take() {
            if e.tokens.is_empty() {
                self.cells.remove(&self.cursor);
            } else {
                self.cells.insert(self.cursor, e.tokens);
            }
        }
    }

    /// A key press, `like` supplying the settings. Returns whether it was used.
    pub fn press(&mut self, key: Key, like: &Calc) {
        if let Some(e) = &mut self.edit {
            match key {
                Key::Ok | Key::Equals => {
                    self.commit();
                    self.step(0, 1);
                }
                Key::Back => self.edit = None,
                Key::Ac => {
                    if e.tokens.is_empty() {
                        self.edit = None;
                    } else {
                        *e = Calc::blank_like(like);
                    }
                }
                Key::Up | Key::Down | Key::PageUp | Key::PageDown => {
                    self.commit();
                    self.press(key, like);
                }
                other => e.press(other),
            }
            return;
        }
        match key {
            Key::Left => self.step(-1, 0),
            Key::Right => self.step(1, 0),
            Key::Up => self.step(0, -1),
            Key::Down => self.step(0, 1),
            Key::PageUp => self.step(0, -4),
            Key::PageDown => self.step(0, 4),
            Key::Del | Key::Ac => {
                self.cells.remove(&self.cursor);
            }
            Key::Ok | Key::Equals => {
                // Edit what is there.
                let mut e = Calc::blank_like(like);
                if let Some(t) = self.cells.get(&self.cursor) {
                    e.tokens = t.clone();
                    e.cursor = e.tokens.len();
                }
                self.edit = Some(e);
            }
            Key::Home
            | Key::Back
            | Key::Settings
            | Key::Variable
            | Key::Function
            | Key::Catalog
            | Key::Tools
            | Key::FormatMenu
            | Key::On
            | Key::Off
            | Key::Sd
            | Key::Shift => {}
            other => {
                // Typing starts a new entry in the cell.
                let mut e = Calc::blank_like(like);
                e.press(other);
                self.edit = Some(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_into(s: &mut Sheet, keys: &[Key]) {
        let like = Calc::default();
        for k in keys {
            s.press(*k, &like);
        }
        s.press(Key::Ok, &like);
    }

    fn at(s: &mut Sheet, col: u8, row: u32) {
        s.cursor = (col, row);
    }

    fn v(s: &Sheet, col: u8, row: u32) -> Result<Value, CalcError> {
        s.value(col, row, &Calc::default())
    }

    #[test]
    fn typing_fills_a_cell_and_ok_moves_down() {
        let mut s = Sheet::default();
        type_into(&mut s, &[Key::Digit(4), Key::Digit(2)]);
        assert_eq!(v(&s, 0, 1), Ok(Value::int(42)));
        assert_eq!(s.cursor, (0, 2), "OK moves to the cell below");
        assert_eq!(v(&s, 0, 2), Ok(Value::int(0)), "empty cells are zero");
    }

    #[test]
    fn formulas_use_other_cells() {
        let mut s = Sheet::default();
        type_into(&mut s, &[Key::Digit(3)]);
        type_into(&mut s, &[Key::Digit(4)]);
        at(&mut s, 1, 1);
        // B1 = A1 × A2 (column letters are the variable keys A to E)
        type_into(&mut s, &[Key::EqSign, Key::Var(0), Key::Digit(1), Key::Times, Key::Var(0), Key::Digit(2)]);
        assert_eq!(v(&s, 1, 1), Ok(Value::int(12)));
        // Changing A1 changes B1.
        at(&mut s, 0, 1);
        type_into(&mut s, &[Key::Digit(5)]);
        assert_eq!(v(&s, 1, 1), Ok(Value::int(20)));
    }

    #[test]
    fn ranges_sum_mean_min_and_max() {
        let mut s = Sheet::default();
        for n in [2, 4, 9] {
            type_into(&mut s, &[Key::Digit(n)]);
        }
        at(&mut s, 1, 1);
        let range = |f| [Key::Func(f), Key::Var(0), Key::Digit(1), Key::Colon, Key::Var(0), Key::Digit(3), Key::RParen];
        type_into(&mut s, &range(crate::calculator::Func::SheetSum));
        type_into(&mut s, &range(crate::calculator::Func::SheetMean));
        type_into(&mut s, &range(crate::calculator::Func::SheetMin));
        type_into(&mut s, &range(crate::calculator::Func::SheetMax));
        assert_eq!(v(&s, 1, 1), Ok(Value::int(15)));
        assert_eq!(v(&s, 1, 2), Ok(Value::int(5)));
        assert_eq!(v(&s, 1, 3), Ok(Value::int(2)));
        assert_eq!(v(&s, 1, 4), Ok(Value::int(9)));
    }

    #[test]
    fn a_cell_that_refers_to_itself_is_a_math_error() {
        let mut s = Sheet::default();
        type_into(&mut s, &[Key::EqSign, Key::Var(0), Key::Digit(1), Key::Plus, Key::Digit(1)]);
        assert_eq!(v(&s, 0, 1), Err(CalcError::Math));
        at(&mut s, 0, 2);
        type_into(&mut s, &[Key::EqSign, Key::Var(1), Key::Digit(2)]);
        at(&mut s, 1, 2);
        type_into(&mut s, &[Key::EqSign, Key::Var(0), Key::Digit(2)]);
        assert_eq!(v(&s, 0, 2), Err(CalcError::Math), "A2 and B2 point at each other");
    }

    #[test]
    fn arrows_move_inside_the_grid_and_del_clears() {
        let mut s = Sheet::default();
        let like = Calc::default();
        for _ in 0..9 {
            s.press(Key::Right, &like);
        }
        assert_eq!(s.cursor.0, COLS - 1);
        s.press(Key::Up, &like);
        assert_eq!(s.cursor.1, 1, "no row above the first");
        for _ in 0..100 {
            s.press(Key::Down, &like);
        }
        assert_eq!(s.cursor.1, ROWS);
        type_into(&mut s, &[Key::Digit(7)]);
        at(&mut s, COLS - 1, ROWS);
        s.press(Key::Del, &like);
        assert!(s.is_empty(COLS - 1, ROWS));
    }

    #[test]
    fn back_cancels_an_edit_and_a_bad_reference_is_a_syntax_error() {
        let mut s = Sheet::default();
        let like = Calc::default();
        s.press(Key::Digit(9), &like);
        s.press(Key::Back, &like);
        assert!(s.edit.is_none() && s.is_empty(0, 1));
        type_into(&mut s, &[Key::EqSign, Key::Var(0), Key::Digit(9), Key::Digit(9)]);
        assert_eq!(v(&s, 0, 1), Err(CalcError::Syntax), "row 99 is off the sheet");
    }
}
