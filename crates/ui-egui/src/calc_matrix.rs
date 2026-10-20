//! Matrices and vectors for the calculator's Matrix and Vector apps, over exact values.
#![allow(clippy::should_implement_trait, clippy::needless_range_loop)]

use crate::calculator::{Angle, CalcError, Value, add, div, mul, neg, sqrt, sub};

/// A matrix of exact values, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Mat {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<Value>,
}

impl Mat {
    pub fn new(rows: usize, cols: usize) -> Mat {
        Mat { rows, cols, data: vec![Value::int(0); rows * cols] }
    }

    pub fn from_rows(rows: &[&[i128]]) -> Mat {
        let (r, c) = (rows.len(), rows.first().map_or(0, |x| x.len()));
        Mat { rows: r, cols: c, data: rows.iter().flat_map(|row| row.iter().map(|v| Value::int(*v))).collect() }
    }

    pub fn identity(n: usize) -> Mat {
        let mut m = Mat::new(n, n);
        for i in 0..n {
            m.set(i, i, Value::int(1));
        }
        m
    }

    pub fn at(&self, r: usize, c: usize) -> Value {
        self.data[r * self.cols + c]
    }

    pub fn set(&mut self, r: usize, c: usize, v: Value) {
        self.data[r * self.cols + c] = v;
    }

    fn same_shape(&self, o: &Mat) -> Result<(), CalcError> {
        if self.rows == o.rows && self.cols == o.cols { Ok(()) } else { Err(CalcError::Math) }
    }

    pub fn plus(&self, o: &Mat) -> Result<Mat, CalcError> {
        self.same_shape(o)?;
        let data = self.data.iter().zip(&o.data).map(|(a, b)| add(*a, *b)).collect::<Result<_, _>>()?;
        Ok(Mat { data, ..self.clone() })
    }

    pub fn minus(&self, o: &Mat) -> Result<Mat, CalcError> {
        self.same_shape(o)?;
        let data = self.data.iter().zip(&o.data).map(|(a, b)| sub(*a, *b)).collect::<Result<_, _>>()?;
        Ok(Mat { data, ..self.clone() })
    }

    pub fn scale(&self, k: Value) -> Result<Mat, CalcError> {
        let data = self.data.iter().map(|a| mul(*a, k)).collect::<Result<_, _>>()?;
        Ok(Mat { data, ..self.clone() })
    }

    pub fn times(&self, o: &Mat) -> Result<Mat, CalcError> {
        if self.cols != o.rows {
            return Err(CalcError::Math);
        }
        let mut out = Mat::new(self.rows, o.cols);
        for i in 0..self.rows {
            for j in 0..o.cols {
                let mut sum = Value::int(0);
                for k in 0..self.cols {
                    sum = add(sum, mul(self.at(i, k), o.at(k, j))?)?;
                }
                out.set(i, j, sum);
            }
        }
        Ok(out)
    }

    pub fn transpose(&self) -> Mat {
        let mut out = Mat::new(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                out.set(j, i, self.at(i, j));
            }
        }
        out
    }

    pub fn det(&self) -> Result<Value, CalcError> {
        if self.rows != self.cols {
            return Err(CalcError::Math);
        }
        let n = self.rows;
        let mut m = self.clone();
        let mut det = Value::int(1);
        for col in 0..n {
            let Some(p) = (col..n).find(|&r| !m.at(r, col).is_zero()) else { return Ok(Value::int(0)) };
            if p != col {
                for k in 0..n {
                    let (a, b) = (m.at(p, k), m.at(col, k));
                    m.set(p, k, b);
                    m.set(col, k, a);
                }
                det = neg(det)?;
            }
            let pivot = m.at(col, col);
            det = mul(det, pivot)?;
            for r in col + 1..n {
                let f = div(m.at(r, col), pivot)?;
                for k in col..n {
                    let t = mul(f, m.at(col, k))?;
                    m.set(r, k, sub(m.at(r, k), t)?);
                }
            }
        }
        Ok(det)
    }

    pub fn inverse(&self) -> Result<Mat, CalcError> {
        if self.rows != self.cols || self.det()?.is_zero() {
            return Err(CalcError::Math);
        }
        let n = self.rows;
        let mut a = self.clone();
        let mut inv = Mat::identity(n);
        for col in 0..n {
            let p = (col..n).find(|&r| !a.at(r, col).is_zero()).ok_or(CalcError::Math)?;
            for k in 0..n {
                let (x, y) = (a.at(p, k), a.at(col, k));
                a.set(p, k, y);
                a.set(col, k, x);
                let (x, y) = (inv.at(p, k), inv.at(col, k));
                inv.set(p, k, y);
                inv.set(col, k, x);
            }
            let pivot = a.at(col, col);
            for k in 0..n {
                a.set(col, k, div(a.at(col, k), pivot)?);
                inv.set(col, k, div(inv.at(col, k), pivot)?);
            }
            for r in 0..n {
                if r != col && !a.at(r, col).is_zero() {
                    let f = a.at(r, col);
                    for k in 0..n {
                        let t = mul(f, a.at(col, k))?;
                        a.set(r, k, sub(a.at(r, k), t)?);
                        let t = mul(f, inv.at(col, k))?;
                        inv.set(r, k, sub(inv.at(r, k), t)?);
                    }
                }
            }
        }
        Ok(inv)
    }
}

/// A vector is a one-row matrix; these are the operations that are only for vectors.
pub fn dot(a: &[Value], b: &[Value]) -> Result<Value, CalcError> {
    if a.len() != b.len() {
        return Err(CalcError::Math);
    }
    a.iter().zip(b).try_fold(Value::int(0), |acc, (x, y)| add(acc, mul(*x, *y)?))
}

pub fn cross(a: &[Value], b: &[Value]) -> Result<Vec<Value>, CalcError> {
    if a.len() != 3 || b.len() != 3 {
        return Err(CalcError::Math);
    }
    let c = |i: usize, j: usize| -> Result<Value, CalcError> { sub(mul(a[i], b[j])?, mul(a[j], b[i])?) };
    Ok(vec![c(1, 2)?, c(2, 0)?, c(0, 1)?])
}

pub fn norm(a: &[Value]) -> Result<Value, CalcError> {
    sqrt(dot(a, a)?)
}

pub fn unit(a: &[Value]) -> Result<Vec<Value>, CalcError> {
    let n = norm(a)?;
    if n.is_zero() {
        return Err(CalcError::Math);
    }
    a.iter().map(|x| div(*x, n)).collect()
}

/// The angle between two vectors, in the calculator's angle unit.
pub fn angle_between(a: &[Value], b: &[Value], unit: Angle) -> Result<Value, CalcError> {
    let denom = mul(norm(a)?, norm(b)?)?;
    if denom.is_zero() {
        return Err(CalcError::Math);
    }
    let c = div(dot(a, b)?, denom)?.to_f64().clamp(-1.0, 1.0);
    Ok(Value::Float(unit.from_radians(c.acos())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(n: &[i128]) -> Vec<Value> {
        n.iter().map(|x| Value::int(*x)).collect()
    }

    #[test]
    fn products_sums_and_transposes() {
        let a = Mat::from_rows(&[&[1, 2], &[3, 4]]);
        let b = Mat::from_rows(&[&[5, 6], &[7, 8]]);
        assert_eq!(a.times(&b).unwrap(), Mat::from_rows(&[&[19, 22], &[43, 50]]));
        assert_eq!(a.plus(&b).unwrap(), Mat::from_rows(&[&[6, 8], &[10, 12]]));
        assert_eq!(b.minus(&a).unwrap(), Mat::from_rows(&[&[4, 4], &[4, 4]]));
        assert_eq!(a.transpose(), Mat::from_rows(&[&[1, 3], &[2, 4]]));
        assert_eq!(a.scale(Value::int(3)).unwrap(), Mat::from_rows(&[&[3, 6], &[9, 12]]));
        assert_eq!(a.times(&Mat::new(3, 1)), Err(CalcError::Math), "shapes that do not fit");
        assert_eq!(a.plus(&Mat::new(3, 3)), Err(CalcError::Math));
    }

    #[test]
    fn determinants_are_exact() {
        assert_eq!(Mat::from_rows(&[&[1, 2], &[3, 4]]).det().unwrap(), Value::int(-2));
        assert_eq!(Mat::from_rows(&[&[2, 0, 1], &[1, 3, 2], &[1, 1, 1]]).det().unwrap(), Value::int(0));
        assert_eq!(Mat::from_rows(&[&[0, 1], &[1, 0]]).det().unwrap(), Value::int(-1), "a row swap flips the sign");
        assert_eq!(Mat::from_rows(&[&[1, 2, 3]]).det(), Err(CalcError::Math));
    }

    #[test]
    fn inverses_are_exact_fractions() {
        let a = Mat::from_rows(&[&[1, 2], &[3, 4]]);
        let inv = a.inverse().unwrap();
        assert_eq!(a.times(&inv).unwrap(), Mat::identity(2));
        assert_eq!(inv.at(0, 0), Value::int(-2));
        assert!(Mat::from_rows(&[&[1, 2], &[2, 4]]).inverse().is_err(), "singular");
    }

    #[test]
    fn vector_products() {
        assert_eq!(dot(&v(&[1, 2, 3]), &v(&[4, 5, 6])).unwrap(), Value::int(32));
        assert_eq!(cross(&v(&[1, 2, 3]), &v(&[4, 5, 6])).unwrap(), v(&[-3, 6, -3]));
        assert_eq!(norm(&v(&[3, 4])).unwrap(), Value::int(5));
        assert_eq!(cross(&v(&[1, 2]), &v(&[3, 4])), Err(CalcError::Math), "cross is for 3D");
        assert_eq!(dot(&v(&[1, 2]), &v(&[1, 2, 3])), Err(CalcError::Math));
    }

    #[test]
    fn angles_between_vectors() {
        let a = angle_between(&v(&[1, 0]), &v(&[0, 1]), Angle::Degree).unwrap().to_f64();
        assert!((a - 90.0).abs() < 1e-9);
        let a = angle_between(&v(&[1, 0]), &v(&[1, 1]), Angle::Degree).unwrap().to_f64();
        assert!((a - 45.0).abs() < 1e-9);
        assert!(angle_between(&v(&[0, 0]), &v(&[1, 1]), Angle::Degree).is_err());
    }

    #[test]
    fn unit_vectors_keep_roots_exact() {
        let u = unit(&v(&[3, 4])).unwrap();
        assert_eq!(u[0].to_f64(), 0.6);
        assert!(unit(&v(&[0, 0])).is_err());
    }
}
