//! A source-owned constant Schur derivative. No numerical sampling fit.
//! The gradient is a continuous IVP derivative on the consistent graph,
//! not a derivative of an adaptive solver or event/reset sequence.
use super::*;

#[derive(Clone, Debug)]
pub struct ConstantTerminalAdjoint {
    n: usize,
    matrix: [f64; 1024],
    goal: [f64; 32],
}
impl ConstantTerminalAdjoint {
    pub fn dimension(&self) -> usize {
        self.n
    }
    pub fn matrix(&self) -> &[f64] {
        &self.matrix[..self.n * self.n]
    }
    pub fn terminal_gradient(&self) -> &[f64] {
        &self.goal[..self.n]
    }
    /// Admit only state-affine terms with constant state coefficients.
    /// Time-only polynomial forcing is permitted. Constant algebraic
    /// derivative must be invertible, with exact represented reduced
    /// coefficients and terminal weights proved by directed arithmetic.
    pub fn from_source(src: &DaeSource, weights: &[f64]) -> CoreResult<Self> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let nx = src.nx();
        let nz = src.nz();
        let n = nx + nz;
        if weights.len() != n || weights.iter().any(|v| !v.is_finite()) {
            return Err(bad("adjoint physical goal shape"));
        }
        for eq in src.f.iter().chain(&src.g) {
            for term in eq.terms() {
                if term.coefficient != 0.
                    && term.factors.len() > 1
                    && term
                        .factors
                        .iter()
                        .any(|f| f.state.iter().any(|&(_, a)| a != 0.))
                {
                    return Err(bad("adjoint requires constant affine state derivative"));
                }
            }
        }
        let mut jac = [[zero(); 33]; 32];
        let mut values = [zero(); 32];
        let state = [zero(); 32];
        guard_source_rows(
            &src.g,
            zero(),
            &state[..n],
            &mut values[..nz],
            &mut jac[..nz],
        )?;
        let mut a = [0.; 1024];
        let mut c = [0.; 1024];
        for i in 0..nz {
            for j in 0..nz {
                a[i * nz + j] = di::mid(jac[i][nx + 1 + j]);
            }
        }
        GuardInverseWorkspace::new().invert_pivoted(nz, &a[..nz * nz], &mut c[..nz * nz], false)?;
        let c = GuardMatrix::new(nz, &c[..nz * nz])?;
        let mut defect = [zero(); 1024];
        let q = guard_defect_into(c, &jac[..nz], nx, &mut defect[..nz * nz])?;
        let gap = sub_down(1., q)?;
        if gap <= 0. {
            return Err(bad("adjoint algebraic rank not certified"));
        }
        let mut zx = [zero(); 1024];
        let mut rhs = [zero(); 32];
        let mut sol = [zero(); 32];
        for k in 0..nx {
            for i in 0..nz {
                rhs[i] = -jac[i][k + 1];
            }
            guard_solve_into(
                c,
                &rhs[..nz],
                q,
                gap,
                &mut sol[..nz],
                &defect[..nz * nz],
                GuardColumnRefinement::AdaptiveGaussSeidel2,
                &mut di::AttemptCount::default(),
            )?;
            for i in 0..nz {
                zx[i * nx + k] = sol[i];
            }
        }
        guard_source_rows(
            &src.f,
            zero(),
            &state[..n],
            &mut values[..nx],
            &mut jac[..nx],
        )?;
        let exact = |v: Iv| {
            if v.lo == v.hi && v.lo.is_finite() {
                Ok(v.lo)
            } else {
                Err(bad("adjoint reduced coefficient not exactly represented"))
            }
        };
        let mut out = Self {
            n: nx,
            matrix: [0.; 1024],
            goal: [0.; 32],
        };
        for i in 0..nx {
            for k in 0..nx {
                let v = (0..nz).try_fold(jac[i][k + 1], |s, j| {
                    s.add(jac[i][nx + 1 + j].mul(zx[j * nx + k])?)
                })?;
                out.matrix[i * nx + k] = exact(v)?;
            }
        }
        for k in 0..nx {
            let v = (0..nz).try_fold(p(weights[k])?, |s, j| {
                s.add(zx[j * nx + k].scale(weights[nx + j])?)
            })?;
            out.goal[k] = exact(v)?;
        }
        Ok(out)
    }
}
