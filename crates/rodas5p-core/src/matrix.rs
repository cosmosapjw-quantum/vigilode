use std::ops::{Index, IndexMut};

use faer::{
    Mat,
    linalg::solvers::{PartialPivLu, Solve},
};

use crate::{CoreError, CoreResult};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DenseMatrix {
    nrows: usize,
    ncols: usize,
    data: Vec<f64>,
}

impl DenseMatrix {
    pub fn new(nrows: usize, ncols: usize, data: Vec<f64>) -> CoreResult<Self> {
        if nrows.checked_mul(ncols) != Some(data.len()) {
            return Err(CoreError::Dimension(format!(
                "matrix shape {nrows}x{ncols} does not match {} entries",
                data.len()
            )));
        }
        if !data.iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite("matrix contains NaN/Inf".into()));
        }
        Ok(Self { nrows, ncols, data })
    }

    pub fn zeros(nrows: usize, ncols: usize) -> Self {
        Self {
            nrows,
            ncols,
            data: vec![0.0; nrows * ncols],
        }
    }

    pub fn resize_zeros(&mut self, nrows: usize, ncols: usize) -> CoreResult<()> {
        let len = nrows
            .checked_mul(ncols)
            .ok_or_else(|| CoreError::Dimension("matrix shape overflow".into()))?;
        self.data.resize(len, 0.0);
        self.data.fill(0.0);
        self.nrows = nrows;
        self.ncols = ncols;
        Ok(())
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    pub fn identity(n: usize) -> Self {
        let mut out = Self::zeros(n, n);
        for i in 0..n {
            out[(i, i)] = 1.0;
        }
        out
    }

    pub fn from_rows(rows: &[&[f64]]) -> CoreResult<Self> {
        let nrows = rows.len();
        let ncols = rows.first().map_or(0, |r| r.len());
        if rows.iter().any(|r| r.len() != ncols) {
            return Err(CoreError::Dimension("ragged matrix rows".into()));
        }
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            data.extend_from_slice(row);
        }
        Self::new(nrows, ncols, data)
    }

    pub fn from_vec_rows(rows: Vec<Vec<f64>>) -> CoreResult<Self> {
        let refs: Vec<&[f64]> = rows.iter().map(Vec::as_slice).collect();
        Self::from_rows(&refs)
    }

    #[inline]
    pub fn nrows(&self) -> usize {
        self.nrows
    }
    #[inline]
    pub fn ncols(&self) -> usize {
        self.ncols
    }
    #[inline]
    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f64] {
        &mut self.data
    }

    pub fn row(&self, i: usize) -> &[f64] {
        let start = i * self.ncols;
        &self.data[start..start + self.ncols]
    }

    pub fn matvec_into(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.ncols || y.len() != self.nrows {
            return Err(CoreError::Dimension(format!(
                "matvec {}x{} with x={}, y={}",
                self.nrows,
                self.ncols,
                x.len(),
                y.len()
            )));
        }
        for (i, yi) in y.iter_mut().enumerate() {
            *yi = self.row(i).iter().zip(x).map(|(a, b)| a * b).sum();
        }
        if !y.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "matrix-vector product produced NaN/Inf".into(),
            ));
        }
        Ok(())
    }

    pub fn matvec(&self, x: &[f64]) -> CoreResult<Vec<f64>> {
        let mut y = vec![0.0; self.nrows];
        self.matvec_into(x, &mut y)?;
        Ok(y)
    }

    pub fn matmul(&self, rhs: &Self) -> CoreResult<Self> {
        if self.ncols != rhs.nrows {
            return Err(CoreError::Dimension(
                "matrix multiply shape mismatch".into(),
            ));
        }
        let mut out = Self::zeros(self.nrows, rhs.ncols);
        for i in 0..self.nrows {
            for k in 0..self.ncols {
                let aik = self[(i, k)];
                for j in 0..rhs.ncols {
                    out[(i, j)] += aik * rhs[(k, j)];
                }
            }
        }
        Ok(out)
    }

    pub fn transpose(&self) -> Self {
        let mut out = Self::zeros(self.ncols, self.nrows);
        for i in 0..self.nrows {
            for j in 0..self.ncols {
                out[(j, i)] = self[(i, j)];
            }
        }
        out
    }

    pub fn add(&self, rhs: &Self) -> CoreResult<Self> {
        self.combine(rhs, 1.0)
    }

    pub fn sub(&self, rhs: &Self) -> CoreResult<Self> {
        self.combine(rhs, -1.0)
    }

    pub fn combine(&self, rhs: &Self, rhs_scale: f64) -> CoreResult<Self> {
        if self.nrows != rhs.nrows || self.ncols != rhs.ncols {
            return Err(CoreError::Dimension(
                "matrix combination shape mismatch".into(),
            ));
        }
        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(a, b)| a + rhs_scale * b)
            .collect();
        Self::new(self.nrows, self.ncols, data)
    }

    pub fn scale(&self, alpha: f64) -> Self {
        Self {
            nrows: self.nrows,
            ncols: self.ncols,
            data: self.data.iter().map(|v| alpha * v).collect(),
        }
    }

    pub fn diagonal(&self) -> CoreResult<Vec<f64>> {
        if self.nrows != self.ncols {
            return Err(CoreError::Dimension(
                "diagonal requires square matrix".into(),
            ));
        }
        Ok((0..self.nrows).map(|i| self[(i, i)]).collect())
    }

    pub fn to_faer(&self) -> Mat<f64> {
        Mat::from_fn(self.nrows, self.ncols, |i, j| self[(i, j)])
    }
}

impl Index<(usize, usize)> for DenseMatrix {
    type Output = f64;
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        &self.data[index.0 * self.ncols + index.1]
    }
}

impl IndexMut<(usize, usize)> for DenseMatrix {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        &mut self.data[index.0 * self.ncols + index.1]
    }
}

#[derive(Clone, Debug)]
pub struct LuFactorization {
    n: usize,
    lu: PartialPivLu<f64>,
}

impl LuFactorization {
    pub fn new(a: &DenseMatrix) -> CoreResult<Self> {
        if a.nrows != a.ncols {
            return Err(CoreError::Dimension(
                "LU factorization requires square matrix".into(),
            ));
        }
        Ok(Self {
            n: a.nrows,
            lu: a.to_faer().partial_piv_lu(),
        })
    }

    pub fn dimension(&self) -> usize {
        self.n
    }

    pub fn solve(&self, rhs: &[f64]) -> CoreResult<Vec<f64>> {
        if rhs.len() != self.n {
            return Err(CoreError::Dimension("LU RHS shape mismatch".into()));
        }
        if !rhs.iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite("LU RHS contains NaN/Inf".into()));
        }
        let b = Mat::from_fn(self.n, 1, |i, _| rhs[i]);
        let x = self.lu.solve(&b);
        let out: Vec<f64> = (0..self.n).map(|i| x[(i, 0)]).collect();
        if out.iter().all(|v| v.is_finite()) {
            Ok(out)
        } else {
            Err(CoreError::LinearSolve("LU produced NaN/Inf".into()))
        }
    }

    pub fn solve_rows(&self, rhs_rows: &[Vec<f64>]) -> CoreResult<Vec<Vec<f64>>> {
        if rhs_rows.iter().any(|r| r.len() != self.n) {
            return Err(CoreError::Dimension("LU batched RHS shape mismatch".into()));
        }
        let cols = rhs_rows.len();
        let b = Mat::from_fn(self.n, cols, |i, j| rhs_rows[j][i]);
        let x = self.lu.solve(&b);
        let mut out = vec![vec![0.0; self.n]; cols];
        for j in 0..cols {
            for i in 0..self.n {
                out[j][i] = x[(i, j)];
            }
        }
        if out.iter().flatten().all(|v| v.is_finite()) {
            Ok(out)
        } else {
            Err(CoreError::LinearSolve("batched LU produced NaN/Inf".into()))
        }
    }
}

pub fn direct_solve(a: &DenseMatrix, rhs: &[f64]) -> CoreResult<Vec<f64>> {
    if a.nrows != a.ncols || rhs.len() != a.nrows {
        return Err(CoreError::Dimension("direct solve shape mismatch".into()));
    }
    if !rhs.iter().all(|x| x.is_finite()) {
        return Err(CoreError::NonFinite(
            "direct solve RHS contains NaN/Inf".into(),
        ));
    }
    let factor = LuFactorization::new(a)?;
    let out = factor.solve(rhs)?;
    if !out.iter().all(|v| v.is_finite()) {
        return Err(CoreError::LinearSolve("LU produced NaN/Inf".into()));
    }
    Ok(out)
}

pub fn inverse(a: &DenseMatrix) -> CoreResult<DenseMatrix> {
    if a.nrows != a.ncols {
        return Err(CoreError::Dimension(
            "inverse requires square matrix".into(),
        ));
    }
    let n = a.nrows;
    let fa = a.to_faer();
    let lu = fa.partial_piv_lu();
    let eye = Mat::identity(n, n);
    let inv = lu.solve(&eye);
    let mut data = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            data[i * n + j] = inv[(i, j)];
        }
    }
    DenseMatrix::new(n, n, data)
}
