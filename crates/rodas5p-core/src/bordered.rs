//! Reusable band-plus-dense-border Schur preconditioner.
//!
//! For `M = [[A,B],[C,D]]`, stores band LU of A, `W=A^-1 B`, and pivoted
//! dense LU of `S=D-CW`. A must be strictly row diagonally dominant. This
//! sufficient restriction makes no-pivot band elimination admissible; it is
//! checked at every refactor. It does not certify floating-point forward error.
//! Successful refactors and applications allocate no heap storage after `new`.
//! The represented inverse may be used directly or as an approximate inverse
//! for a different operator. Krylov convergence must use that operator's true
//! residual. No fixed iteration count follows from the block layout alone.

use crate::{CoreError, CoreResult, Preconditioner, PreconditionerSide};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderedShape {
    pub phase: usize,
    pub matter: usize,
    pub lower: usize,
    pub upper: usize,
}

/// Capacity-owned numeric factors. Refactor failure invalidates every old factor.
/// Packed A row `i` stores A[i,j] at `i*(lower+upper+1) + j+lower-i`.
/// B, C, D are row-major with sizes n*m, m*n, m*m.
#[derive(Debug)]
pub struct BorderedBandedPreconditioner {
    max_phase: usize,
    max_matter: usize,
    lower: usize,
    upper: usize,
    stride: usize,
    n: usize,
    m: usize,
    valid: bool,
    band_lu: Vec<f64>,
    inverse_border: Vec<f64>,
    lower_border: Vec<f64>,
    schur_lu: Vec<f64>,
    pivots: Vec<usize>,
    column: Vec<f64>,
}

fn dimension_error() -> CoreError {
    CoreError::Dimension("bordered band shape/capacity mismatch".into())
}

fn numeric_error(message: &'static str) -> CoreError {
    CoreError::LinearSolve(message.into())
}

fn storage<T: Clone>(len: usize, value: T) -> CoreResult<Vec<T>> {
    if len
        .checked_mul(std::mem::size_of::<T>())
        .is_none_or(|bytes| bytes > isize::MAX as usize)
    {
        return Err(dimension_error());
    }
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| numeric_error("bordered storage reservation failed"))?;
    out.resize(len, value);
    Ok(out)
}

fn band_solve(n: usize, lower: usize, upper: usize, lu: &[f64], x: &mut [f64]) {
    let stride = lower + upper + 1;
    for i in 0..n {
        for j in i.saturating_sub(lower)..i {
            x[i] -= lu[i * stride + j + lower - i] * x[j];
        }
    }
    for i in (0..n).rev() {
        for j in (i + 1)..n.min(i.saturating_add(upper).saturating_add(1)) {
            x[i] -= lu[i * stride + j + lower - i] * x[j];
        }
        x[i] /= lu[i * stride + lower];
    }
}

impl BorderedBandedPreconditioner {
    pub fn new(
        max_phase: usize,
        max_matter: usize,
        lower: usize,
        upper: usize,
    ) -> CoreResult<Self> {
        if max_phase == 0 || lower >= max_phase || upper >= max_phase {
            return Err(dimension_error());
        }
        let stride = lower
            .checked_add(upper)
            .and_then(|x| x.checked_add(1))
            .ok_or_else(dimension_error)?;
        let band_len = max_phase.checked_mul(stride).ok_or_else(dimension_error)?;
        let border_len = max_phase
            .checked_mul(max_matter)
            .ok_or_else(dimension_error)?;
        let schur_len = max_matter
            .checked_mul(max_matter)
            .ok_or_else(dimension_error)?;
        max_phase
            .checked_add(max_matter)
            .ok_or_else(dimension_error)?;
        Ok(Self {
            max_phase,
            max_matter,
            lower,
            upper,
            stride,
            n: 0,
            m: 0,
            valid: false,
            band_lu: storage(band_len, 0.0)?,
            inverse_border: storage(border_len, 0.0)?,
            lower_border: storage(border_len, 0.0)?,
            schur_lu: storage(schur_len, 0.0)?,
            pivots: storage(max_matter, 0)?,
            column: storage(max_phase, 0.0)?,
        })
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn active_shape(&self) -> Option<BorderedShape> {
        self.valid.then_some(BorderedShape {
            phase: self.n,
            matter: self.m,
            lower: self.lower,
            upper: self.upper,
        })
    }

    /// Number of allocated floating-point slots (usize pivots are additional).
    pub fn allocated_f64_slots(&self) -> usize {
        self.band_lu.len()
            + self.inverse_border.len()
            + self.lower_border.len()
            + self.schur_lu.len()
            + self.column.len()
    }

    /// Prepare M at an active shape no larger than constructor capacity.
    /// Supports decreasing/increasing n,m without reallocating. The bandwidth
    /// stays fixed; entries outside active n are ignored but must be finite.
    #[allow(clippy::too_many_arguments)]
    pub fn refactor(
        &mut self,
        n: usize,
        m: usize,
        a: &[f64],
        b: &[f64],
        c: &[f64],
        d: &[f64],
    ) -> CoreResult<()> {
        self.valid = false;
        self.n = 0;
        self.m = 0;
        if n == 0
            || n > self.max_phase
            || m > self.max_matter
            || a.len() != n * self.stride
            || b.len() != n * m
            || c.len() != m * n
            || d.len() != m * m
        {
            return Err(dimension_error());
        }
        if a.iter().chain(b).chain(c).chain(d).any(|v| !v.is_finite()) {
            return Err(CoreError::NonFinite(
                "bordered band input contains NaN/Inf".into(),
            ));
        }
        for i in 0..n {
            let mut off = 0.0;
            for j in
                i.saturating_sub(self.lower)..n.min(i.saturating_add(self.upper).saturating_add(1))
            {
                if j != i {
                    // Upward enclosure prevents rounded-down row sums from
                    // falsely admitting a merely weakly dominant matrix.
                    off = (off + a[i * self.stride + j + self.lower - i].abs()).next_up();
                }
            }
            if !off.is_finite() || a[i * self.stride + self.lower].abs() <= off {
                return Err(numeric_error(
                    "bordered phase block requires strict row diagonal dominance",
                ));
            }
        }
        self.band_lu[..a.len()].copy_from_slice(a);
        for k in 0..n {
            let pivot = self.band_lu[k * self.stride + self.lower];
            if !pivot.is_finite() || pivot == 0.0 {
                return Err(numeric_error("bordered band pivot failed"));
            }
            for i in (k + 1)..n.min(k.saturating_add(self.lower).saturating_add(1)) {
                let ik = i * self.stride + k + self.lower - i;
                self.band_lu[ik] /= pivot;
                for j in (k + 1)..n.min(k.saturating_add(self.upper).saturating_add(1)) {
                    let ij = i * self.stride + j + self.lower - i;
                    let kj = k * self.stride + j + self.lower - k;
                    self.band_lu[ij] -= self.band_lu[ik] * self.band_lu[kj];
                }
            }
        }
        if self.band_lu[..a.len()].iter().any(|x| !x.is_finite()) {
            return Err(numeric_error("bordered band factor overflow"));
        }
        self.lower_border[..c.len()].copy_from_slice(c);
        for j in 0..m {
            for i in 0..n {
                self.column[i] = b[i * m + j];
            }
            band_solve(
                n,
                self.lower,
                self.upper,
                &self.band_lu,
                &mut self.column[..n],
            );
            if self.column[..n].iter().any(|x| !x.is_finite()) {
                return Err(numeric_error("bordered inverse border overflow"));
            }
            for i in 0..n {
                self.inverse_border[i * m + j] = self.column[i];
            }
        }
        for i in 0..m {
            for j in 0..m {
                let mut s = d[i * m + j];
                for k in 0..n {
                    s -= c[i * n + k] * self.inverse_border[k * m + j];
                }
                if !s.is_finite() {
                    return Err(numeric_error("bordered Schur assembly overflow"));
                }
                self.schur_lu[i * m + j] = s;
            }
        }
        for k in 0..m {
            let mut p = k;
            for i in (k + 1)..m {
                if self.schur_lu[i * m + k].abs() > self.schur_lu[p * m + k].abs() {
                    p = i;
                }
            }
            self.pivots[k] = p;
            let pivot = self.schur_lu[p * m + k];
            if !pivot.is_finite() || pivot == 0.0 {
                return Err(numeric_error("bordered Schur pivot failed"));
            }
            for j in 0..m {
                self.schur_lu.swap(k * m + j, p * m + j);
            }
            for i in (k + 1)..m {
                self.schur_lu[i * m + k] /= pivot;
                for j in (k + 1)..m {
                    self.schur_lu[i * m + j] -= self.schur_lu[i * m + k] * self.schur_lu[k * m + j];
                }
            }
        }
        if self.schur_lu[..m * m].iter().any(|x| !x.is_finite()) {
            return Err(numeric_error("bordered Schur factor overflow"));
        }
        self.n = n;
        self.m = m;
        self.valid = true;
        Ok(())
    }
}

impl Preconditioner for BorderedBandedPreconditioner {
    fn dimension(&self) -> usize {
        self.n + self.m
    }
    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }

    /// Allocation-free successful application. Invalid state/shape/input refuses
    /// before writing y. Arithmetic overflow can leave partial y; discard y on
    /// every error, as for a fallible operator application. The factors stay valid.
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if !self.valid {
            return Err(numeric_error("bordered factors are invalid"));
        }
        let (n, m) = (self.n, self.m);
        if x.len() != n + m || y.len() != n + m {
            return Err(dimension_error());
        }
        if x.iter().any(|v| !v.is_finite()) {
            return Err(CoreError::NonFinite("bordered rhs contains NaN/Inf".into()));
        }
        y[..n].copy_from_slice(&x[..n]);
        band_solve(n, self.lower, self.upper, &self.band_lu, &mut y[..n]);
        for i in 0..m {
            let mut rhs = x[n + i];
            for (k, value) in y[..n].iter().enumerate() {
                rhs -= self.lower_border[i * n + k] * value;
            }
            y[n + i] = rhs;
        }
        for k in 0..m {
            y.swap(n + k, n + self.pivots[k]);
        }
        for i in 0..m {
            for j in 0..i {
                y[n + i] -= self.schur_lu[i * m + j] * y[n + j];
            }
        }
        for i in (0..m).rev() {
            for j in (i + 1)..m {
                y[n + i] -= self.schur_lu[i * m + j] * y[n + j];
            }
            y[n + i] /= self.schur_lu[i * m + i];
        }
        for i in 0..n {
            for j in 0..m {
                y[i] -= self.inverse_border[i * m + j] * y[n + j];
            }
        }
        if y.iter().any(|v| !v.is_finite()) {
            return Err(numeric_error("bordered solve overflow"));
        }
        Ok(())
    }
}
