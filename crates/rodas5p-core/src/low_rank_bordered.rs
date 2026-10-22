//! Original-dimension inverse for a band-plus-low-rank phase block and dense border.
//!
//! Represents `M = [[A + U V^T, B], [C, D]]` without forming its dense phase block.
//! It factors `Q = [[A,B,U],[C,D,0],[-V^T,0,I]]`, embeds a right-hand side as
//! `(f,g,0)`, and restricts the solution to `(u,v)`. Thus the Krylov dimension
//! remains `n+m`, although the internal Schur complement has size `m+r`.
//! A must satisfy the sufficient strict row diagonal dominance condition of
//! [`BorderedBandedPreconditioner`]. No such condition is imposed on `A+UV^T`.
//! Successful refactor and application allocate no heap memory after construction.
//! Floating-point residuals must still be evaluated against the original operator.

use std::sync::Mutex;

use crate::bordered::BorderedBandedPreconditioner;
use crate::{CoreError, CoreResult, Preconditioner, PreconditionerSide};

fn dimension_error() -> CoreError {
    CoreError::Dimension("low-rank bordered shape/capacity mismatch".into())
}

fn storage(len: usize) -> CoreResult<Vec<f64>> {
    if len
        .checked_mul(std::mem::size_of::<f64>())
        .is_none_or(|bytes| bytes > isize::MAX as usize)
    {
        return Err(dimension_error());
    }
    let mut values = Vec::new();
    values.try_reserve_exact(len).map_err(|_| {
        CoreError::LinearSolve("low-rank bordered storage reservation failed".into())
    })?;
    values.resize(len, 0.0);
    Ok(values)
}

/// Separate workspaces allow concurrent applications of the same immutable factors.
#[derive(Debug)]
pub struct LowRankApplyWorkspace {
    rhs: Vec<f64>,
    solution: Vec<f64>,
}

impl LowRankApplyWorkspace {
    /// Reserve for the augmented size `max_phase + max_matter + max_rank`.
    pub fn new(max_phase: usize, max_matter: usize, max_rank: usize) -> CoreResult<Self> {
        let len = max_phase
            .checked_add(max_matter)
            .and_then(|x| x.checked_add(max_rank))
            .ok_or_else(dimension_error)?;
        Ok(Self {
            rhs: storage(len)?,
            solution: storage(len)?,
        })
    }

    pub fn capacity(&self) -> usize {
        self.rhs.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LowRankBorderedShape {
    pub phase: usize,
    pub matter: usize,
    pub rank: usize,
}

/// Capacity-owned factors with a safe original-dimension [`Preconditioner`] adapter.
///
/// The trait adapter serializes concurrent calls on its internal scratch mutex.
/// Use [`Self::apply_with_workspace`] with one workspace per caller to avoid that
/// serialization. A failed refactor invalidates all old factors. Every failed
/// application leaves the caller's output unchanged; the factors remain usable.
#[derive(Debug)]
pub struct LowRankBorderedPreconditioner {
    max_phase: usize,
    max_matter: usize,
    max_rank: usize,
    stride: usize,
    shape: Option<LowRankBorderedShape>,
    inner: BorderedBandedPreconditioner,
    upper_border: Vec<f64>,
    lower_border: Vec<f64>,
    dense_border: Vec<f64>,
    workspace: Mutex<LowRankApplyWorkspace>,
}

impl LowRankBorderedPreconditioner {
    pub fn new(
        max_phase: usize,
        max_matter: usize,
        max_rank: usize,
        lower: usize,
        upper: usize,
    ) -> CoreResult<Self> {
        let border = max_matter
            .checked_add(max_rank)
            .ok_or_else(dimension_error)?;
        let rectangle = max_phase.checked_mul(border).ok_or_else(dimension_error)?;
        let square = border.checked_mul(border).ok_or_else(dimension_error)?;
        let stride = lower
            .checked_add(upper)
            .and_then(|x| x.checked_add(1))
            .ok_or_else(dimension_error)?;
        Ok(Self {
            max_phase,
            max_matter,
            max_rank,
            stride,
            shape: None,
            inner: BorderedBandedPreconditioner::new(max_phase, border, lower, upper)?,
            upper_border: storage(rectangle)?,
            lower_border: storage(rectangle)?,
            dense_border: storage(square)?,
            workspace: Mutex::new(LowRankApplyWorkspace::new(max_phase, max_matter, max_rank)?),
        })
    }

    pub fn is_valid(&self) -> bool {
        self.shape.is_some()
    }

    pub fn active_shape(&self) -> Option<LowRankBorderedShape> {
        self.shape
    }

    /// Factor an active shape within capacity. A uses the packed band convention
    /// of [`BorderedBandedPreconditioner`]; B,C,D are row-major n*m,m*n,m*m.
    /// U and V are both row-major n*r (V is supplied before transposition).
    /// Rank zero and an empty matter border are supported. Inputs, including
    /// unused padding in A, must be finite. No rank-compression is performed.
    #[allow(clippy::too_many_arguments)]
    pub fn refactor(
        &mut self,
        n: usize,
        m: usize,
        r: usize,
        a: &[f64],
        b: &[f64],
        c: &[f64],
        d: &[f64],
        u: &[f64],
        v: &[f64],
    ) -> CoreResult<()> {
        self.shape = None;
        // Constructor capacities ensure that all products below fit usize.
        if n == 0
            || n > self.max_phase
            || m > self.max_matter
            || r > self.max_rank
            || a.len() != n * self.stride
            || b.len() != n * m
            || c.len() != m * n
            || d.len() != m * m
            || u.len() != n * r
            || v.len() != n * r
        {
            return Err(dimension_error());
        }
        if a.iter()
            .chain(b)
            .chain(c)
            .chain(d)
            .chain(u)
            .chain(v)
            .any(|x| !x.is_finite())
        {
            return Err(CoreError::NonFinite(
                "low-rank bordered input contains NaN/Inf".into(),
            ));
        }
        let s = m + r;
        for i in 0..n {
            self.upper_border[i * s..i * s + m].copy_from_slice(&b[i * m..(i + 1) * m]);
            self.upper_border[i * s + m..(i + 1) * s].copy_from_slice(&u[i * r..(i + 1) * r]);
        }
        self.lower_border[..m * n].copy_from_slice(c);
        for k in 0..r {
            for i in 0..n {
                self.lower_border[(m + k) * n + i] = -v[i * r + k];
            }
        }
        self.dense_border[..s * s].fill(0.0);
        for i in 0..m {
            self.dense_border[i * s..i * s + m].copy_from_slice(&d[i * m..(i + 1) * m]);
        }
        for k in 0..r {
            self.dense_border[(m + k) * s + m + k] = 1.0;
        }
        self.inner.refactor(
            n,
            s,
            a,
            &self.upper_border[..n * s],
            &self.lower_border[..s * n],
            &self.dense_border[..s * s],
        )?;
        self.shape = Some(LowRankBorderedShape {
            phase: n,
            matter: m,
            rank: r,
        });
        Ok(())
    }

    /// Apply without locking using caller-owned scratch. Successful application
    /// allocates nothing; every failure leaves output unchanged. A workspace may
    /// be reused across active shapes.
    pub fn apply_with_workspace(
        &self,
        x: &[f64],
        y: &mut [f64],
        workspace: &mut LowRankApplyWorkspace,
    ) -> CoreResult<()> {
        let shape = self.shape.ok_or_else(|| {
            CoreError::LinearSolve("low-rank bordered factors are invalid".into())
        })?;
        let original = shape.phase + shape.matter;
        let augmented = original + shape.rank;
        if x.len() != original || y.len() != original || workspace.capacity() < augmented {
            return Err(dimension_error());
        }
        workspace.rhs[..original].copy_from_slice(x);
        workspace.rhs[original..augmented].fill(0.0);
        self.inner.apply(
            &workspace.rhs[..augmented],
            &mut workspace.solution[..augmented],
        )?;
        y.copy_from_slice(&workspace.solution[..original]);
        Ok(())
    }
}

impl Preconditioner for LowRankBorderedPreconditioner {
    fn dimension(&self) -> usize {
        self.shape.map_or(0, |s| s.phase + s.matter)
    }

    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let mut workspace = self.workspace.lock().map_err(|_| {
            CoreError::LinearSolve("low-rank bordered scratch mutex poisoned".into())
        })?;
        self.apply_with_workspace(x, y, &mut workspace)
    }
}
