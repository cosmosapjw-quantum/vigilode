//! Line-tridiagonal right preconditioner for matrix-free stage solves on structured grids.
//!
//! For a stage matrix `W = s I - J` (`s = 1 / (h gamma)`), [`LineTridiagonalPreconditioner`]
//! keeps the diagonal of `J` and the couplings of `J` along one grid axis, drops every other
//! entry, and inverts the resulting tridiagonal matrix `M` line by line (the Thomas
//! algorithm; a periodic line of at least three unknowns adds its two corner entries through
//! the Sherman-Morrison formula). On a diffusion operator `M` keeps the whole diagonal, so it
//! is strictly diagonally dominant for every `s > 0`. It plays the role of the facade's
//! longitude line preconditioner, for any [`MolOperator`] that declares [`LineLayout`]s.
//!
//! The factors are built by [`LineTridiagonalPreconditioner::update`] (once per stage matrix,
//! no allocation) from the operator's assembled Jacobian values; [`Preconditioner::apply`]
//! allocates nothing and declares [`PreconditionerSide::Right`].

use rodas5p_core::{CoreError, CoreResult, Preconditioner, PreconditionerSide};

use super::{LineLayout, MolOperator, OperatorStructure, check_len, invalid};

const NONE: usize = usize::MAX;

/// Line-tridiagonal approximation of `s I - J` along one grid axis, factored once per update.
#[derive(Clone, Debug)]
pub struct LineTridiagonalPreconditioner {
    layout: LineLayout,
    n: usize,
    diag_pos: Vec<usize>,
    prev_pos: Vec<usize>,
    next_pos: Vec<usize>,
    values: Vec<f64>,
    lower: Vec<f64>,
    inv_pivot: Vec<f64>,
    upper: Vec<f64>,
    z: Vec<f64>,
    corner_ratio: Vec<f64>,
    corner_denominator: Vec<f64>,
    cyclic: bool,
    ready: bool,
}

impl LineTridiagonalPreconditioner {
    /// Storage for the lines of `axis` of an operator's structure. Allocates every buffer;
    /// nothing allocates afterwards.
    pub fn new(structure: &OperatorStructure, axis: usize) -> CoreResult<Self> {
        let layout = *structure
            .lines()
            .iter()
            .find(|l| l.axis == axis)
            .ok_or_else(|| invalid("the operator declares no line layout for this axis"))?;
        let n = structure.dimension();
        let m = layout.length;
        if m == 0 || layout.count * m != n {
            return Err(invalid("line layout does not cover the state"));
        }
        let pattern = structure.pattern();
        let cyclic = layout.periodic && m >= 3;
        let mut diag_pos = vec![NONE; n];
        let mut prev_pos = vec![NONE; n];
        let mut next_pos = vec![NONE; n];
        for line in 0..layout.count {
            for p in 0..m {
                let i = layout.index(line, p);
                diag_pos[i] = pattern.diagonal_positions()[i];
                let prev = if p > 0 {
                    Some(layout.index(line, p - 1))
                } else if cyclic {
                    Some(layout.index(line, m - 1))
                } else {
                    None
                };
                let next = if p + 1 < m {
                    Some(layout.index(line, p + 1))
                } else if cyclic {
                    Some(layout.index(line, 0))
                } else {
                    None
                };
                if let Some(j) = prev {
                    prev_pos[i] = pattern.position(i, j).unwrap_or(NONE);
                }
                if let Some(j) = next {
                    next_pos[i] = pattern.position(i, j).unwrap_or(NONE);
                }
            }
        }
        Ok(Self {
            layout,
            n,
            diag_pos,
            prev_pos,
            next_pos,
            values: vec![0.0; pattern.nnz()],
            lower: vec![0.0; n],
            inv_pivot: vec![0.0; n],
            upper: vec![0.0; n],
            z: if cyclic { vec![0.0; n] } else { Vec::new() },
            corner_ratio: vec![0.0; if cyclic { layout.count } else { 0 }],
            corner_denominator: vec![0.0; if cyclic { layout.count } else { 0 }],
            cyclic,
            ready: false,
        })
    }

    /// The lines this preconditioner inverts.
    pub fn layout(&self) -> LineLayout {
        self.layout
    }

    /// Evaluates the operator's Jacobian at `(t, u)` and factors `M` for the shift
    /// `s = 1 / (h gamma) > 0`. No allocation.
    pub fn update(&mut self, op: &dyn MolOperator, t: f64, u: &[f64], shift: f64) -> CoreResult<()> {
        self.ready = false;
        check_len(self.n, op.dimension(), "line preconditioner operator")?;
        op.jacobian_values_into(t, u, &mut self.values)?;
        self.factor(shift)
    }

    /// Factors `M` from Jacobian values given on the operator's pattern. No allocation.
    pub fn update_from_values(&mut self, values: &[f64], shift: f64) -> CoreResult<()> {
        self.ready = false;
        check_len(self.values.len(), values.len(), "line preconditioner values")?;
        self.values.copy_from_slice(values);
        self.factor(shift)
    }

    fn entry(&self, pos: usize) -> f64 {
        if pos == NONE { 0.0 } else { self.values[pos] }
    }

    fn singular(line: usize) -> CoreError {
        CoreError::LinearSolve(format!(
            "line preconditioner: zero or non-finite pivot on line {line}"
        ))
    }

    fn factor(&mut self, shift: f64) -> CoreResult<()> {
        if !shift.is_finite() {
            return Err(CoreError::NonFinite(
                "line preconditioner shift is not finite".into(),
            ));
        }
        let m = self.layout.length;
        for line in 0..self.layout.count {
            let first = self.layout.index(line, 0);
            let last = self.layout.index(line, m - 1);
            // Row p of M: a_p x_(p-1) + b_p x_p + c_p x_(p+1); corners of a cyclic line:
            // alpha = a_0 (column m - 1 of row 0), beta = c_(m-1) (column 0 of row m - 1).
            let b0 = shift - self.entry(self.diag_pos[first]);
            let (alpha, beta, gamma) = if self.cyclic {
                (
                    -self.entry(self.prev_pos[first]),
                    -self.entry(self.next_pos[last]),
                    -b0,
                )
            } else {
                (0.0, 0.0, 0.0)
            };
            let mut previous_upper = 0.0;
            for p in 0..m {
                let i = self.layout.index(line, p);
                let mut b = shift - self.entry(self.diag_pos[i]);
                let a = if p == 0 { 0.0 } else { -self.entry(self.prev_pos[i]) };
                let c = if p + 1 == m { 0.0 } else { -self.entry(self.next_pos[i]) };
                if self.cyclic {
                    if p == 0 {
                        b -= gamma;
                    }
                    if p + 1 == m {
                        b -= alpha * beta / gamma;
                    }
                }
                let pivot = b - a * previous_upper;
                if !(pivot.is_finite() && pivot != 0.0) {
                    return Err(Self::singular(line));
                }
                let inv = 1.0 / pivot;
                self.lower[i] = a;
                self.inv_pivot[i] = inv;
                self.upper[i] = c * inv;
                previous_upper = self.upper[i];
                if !(self.upper[i].is_finite() && inv.is_finite()) {
                    return Err(Self::singular(line));
                }
            }
            if self.cyclic {
                // z = B^-1 (gamma, 0, ..., 0, beta).
                let mut z = std::mem::take(&mut self.z);
                for p in 0..m {
                    let i = self.layout.index(line, p);
                    let rhs = if p == 0 {
                        gamma
                    } else if p + 1 == m {
                        beta
                    } else {
                        0.0
                    };
                    let carried = if p == 0 {
                        0.0
                    } else {
                        self.lower[i] * z[self.layout.index(line, p - 1)]
                    };
                    z[i] = (rhs - carried) * self.inv_pivot[i];
                }
                for p in (0..m - 1).rev() {
                    let i = self.layout.index(line, p);
                    z[i] -= self.upper[i] * z[self.layout.index(line, p + 1)];
                }
                let ratio = alpha / gamma;
                let denominator = 1.0 + z[first] + ratio * z[last];
                self.z = z;
                if !(denominator.is_finite() && denominator != 0.0 && ratio.is_finite()) {
                    return Err(Self::singular(line));
                }
                self.corner_ratio[line] = ratio;
                self.corner_denominator[line] = denominator;
            }
        }
        self.ready = true;
        Ok(())
    }
}

impl Preconditioner for LineTridiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.n
    }

    /// `y = M^-1 x`, line by line. No allocation.
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.n || y.len() != self.n {
            return Err(CoreError::Dimension(
                "line preconditioner/state length mismatch".into(),
            ));
        }
        if !self.ready {
            return Err(CoreError::LinearSolve(
                "line preconditioner has no factors".into(),
            ));
        }
        let m = self.layout.length;
        for line in 0..self.layout.count {
            let mut last = 0.0;
            for p in 0..m {
                let i = self.layout.index(line, p);
                last = (x[i] - self.lower[i] * last) * self.inv_pivot[i];
                y[i] = last;
            }
            for p in (0..m - 1).rev() {
                let i = self.layout.index(line, p);
                y[i] -= self.upper[i] * y[self.layout.index(line, p + 1)];
            }
            if self.cyclic {
                let first = self.layout.index(line, 0);
                let end = self.layout.index(line, m - 1);
                let factor =
                    (y[first] + self.corner_ratio[line] * y[end]) / self.corner_denominator[line];
                for p in 0..m {
                    let i = self.layout.index(line, p);
                    y[i] -= factor * self.z[i];
                }
            }
        }
        Ok(())
    }

    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
}
