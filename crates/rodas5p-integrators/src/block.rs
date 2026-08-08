use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, IdentityPreconditioner, LinearOperator, LuFactorization,
    Preconditioner, WorkCounters, safe_l2,
};
use rodas5p_krylov::{GmresConfig, solve_gmres};

use crate::StepContext;

type StageRows = Vec<Vec<f64>>;
type NonlinearRhsData = (StageRows, StageRows, StageRows, StageRows);

#[derive(Clone, Debug, PartialEq)]
pub struct NonlinearRemainderSnapshot {
    pub rhs: StageRows,
    pub states: StageRows,
    pub rhs_values: StageRows,
    pub remainder: StageRows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockMethod {
    Forward,
    Explicit,
    Nilpotent,
    Gmres,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockPreconditioner {
    None,
    Direct,
    Jacobi,
}

#[derive(Clone, Debug)]
pub struct BlockSolveReport {
    pub stages: Vec<Vec<f64>>,
    pub converged: bool,
    pub info: i32,
    pub residual_norm: f64,
    pub relative_residual: f64,
    pub iterations: u64,
    pub matvecs: u64,
    pub preconditioner_apps: u64,
    pub method: String,
    pub polynomial_terms: usize,
}

pub struct StructuredBlockSystem<'ctx, 'p> {
    pub context: &'ctx StepContext<'p>,
    pub s: usize,
    pub n: usize,
}

impl<'ctx, 'p> StructuredBlockSystem<'ctx, 'p> {
    pub fn new(context: &'ctx StepContext<'p>) -> Self {
        Self {
            context,
            s: context.coeffs.stages(),
            n: context.problem.dimension,
        }
    }
    pub(crate) fn validate_rows(&self, x: &[Vec<f64>]) -> CoreResult<()> {
        if x.len() != self.s || x.iter().any(|r| r.len() != self.n) {
            Err(CoreError::Dimension("block row shape mismatch".into()))
        } else {
            Ok(())
        }
    }

    pub(crate) fn validate_stage_rows(&self, x: &[Vec<f64>]) -> CoreResult<()> {
        self.validate_rows(x)
    }
    fn stage_mix(&self, matrix: &DenseMatrix, k: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let mut out = vec![vec![0.0; self.n]; self.s];
        for i in 0..self.s {
            for j in 0..self.s {
                let a = matrix[(i, j)];
                if a != 0.0 {
                    for q in 0..self.n {
                        out[i][q] += a * k[j][q];
                    }
                }
            }
        }
        out
    }
    fn mass_rows(&self, k: &[Vec<f64>], counters: &mut WorkCounters) -> CoreResult<Vec<Vec<f64>>> {
        let mut out = Vec::with_capacity(self.s);
        for row in k {
            if let Some(m) = &self.context.problem.mass_matrix {
                out.push(m.matvec(row)?);
                counters.mass_matvecs += 1;
            } else {
                out.push(row.clone());
            }
        }
        Ok(out)
    }
    fn jacobian_rows(
        &self,
        k: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<Vec<f64>>> {
        let mut out = Vec::with_capacity(self.s);
        for row in k {
            let mut v = vec![0.0; self.n];
            self.context.jacobian.apply(row, &mut v)?;
            counters.jvp_calls += 1;
            counters.jvp_vectors += 1;
            out.push(v);
        }
        Ok(out)
    }

    pub fn apply(&self, k: &[Vec<f64>], counters: &mut WorkCounters) -> CoreResult<Vec<Vec<f64>>> {
        self.validate_rows(k)?;
        counters.block_matvecs += 1;
        let mixed = self.stage_mix(&self.context.coeffs.beta, k);
        let m = self.mass_rows(k, counters)?;
        let j = self.jacobian_rows(&mixed, counters)?;
        Ok(m.into_iter()
            .zip(j)
            .map(|(mut a, b)| {
                for q in 0..self.n {
                    a[q] -= self.context.h * b[q];
                }
                a
            })
            .collect())
    }

    pub fn diagonal_apply(
        &self,
        k: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<Vec<f64>>> {
        self.validate_rows(k)?;
        let mut mass = self.mass_rows(k, counters)?;
        let jacobian = self.jacobian_rows(k, counters)?;
        for (mass_row, jacobian_row) in mass.iter_mut().zip(jacobian) {
            for component in 0..self.n {
                mass_row[component] -=
                    self.context.h * self.context.coeffs.gamma * jacobian_row[component];
            }
        }
        Ok(mass)
    }

    pub fn coupling_apply(
        &self,
        k: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<Vec<f64>>> {
        self.validate_rows(k)?;
        let mixed = self.stage_mix(&self.context.coeffs.l, k);
        let mut jacobian = self.jacobian_rows(&mixed, counters)?;
        for row in &mut jacobian {
            for value in row {
                *value *= self.context.h;
            }
        }
        Ok(jacobian)
    }

    pub fn partial_linear_apply(
        &self,
        k: &[Vec<f64>],
        eta: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<Vec<f64>>> {
        if !eta.is_finite() {
            return Err(CoreError::NonFinite(
                "partial linear coupling contains NaN/Inf".into(),
            ));
        }
        let mut diagonal = self.diagonal_apply(k, counters)?;
        if eta == 0.0 {
            return Ok(diagonal);
        }
        let coupling = self.coupling_apply(k, counters)?;
        for (diagonal_row, coupling_row) in diagonal.iter_mut().zip(coupling) {
            for component in 0..self.n {
                diagonal_row[component] -= eta * coupling_row[component];
            }
        }
        Ok(diagonal)
    }
    pub fn raw_apply(&self, k: &[Vec<f64>]) -> CoreResult<Vec<Vec<f64>>> {
        self.validate_rows(k)?;
        let mixed = self.stage_mix(&self.context.coeffs.beta, k);
        let mut out = Vec::with_capacity(self.s);
        for i in 0..self.s {
            let mut m = if let Some(mm) = &self.context.problem.mass_matrix {
                mm.matvec(&k[i])?
            } else {
                k[i].clone()
            };
            let mut j = vec![0.0; self.n];
            self.context.jacobian.apply(&mixed[i], &mut j)?;
            for q in 0..self.n {
                m[q] -= self.context.h * j[q];
            }
            out.push(m);
        }
        Ok(out)
    }
    pub fn explicit_matrix(&self) -> CoreResult<DenseMatrix> {
        let j = self.context.jacobian.explicit().ok_or_else(|| {
            CoreError::LinearSolve("explicit block matrix needs explicit J".into())
        })?;
        let m = self
            .context
            .problem
            .mass_matrix
            .clone()
            .unwrap_or_else(|| DenseMatrix::identity(self.n));
        let mut out = DenseMatrix::zeros(self.s * self.n, self.s * self.n);
        for si in 0..self.s {
            for sj in 0..self.s {
                for i in 0..self.n {
                    for q in 0..self.n {
                        let mut v =
                            -self.context.h * self.context.coeffs.beta[(si, sj)] * j[(i, q)];
                        if si == sj {
                            v += m[(i, q)];
                        }
                        out[(si * self.n + i, sj * self.n + q)] = v;
                    }
                }
            }
        }
        Ok(out)
    }
    pub fn rhs_base(&self) -> Vec<Vec<f64>> {
        let mut out = vec![vec![0.0; self.n]; self.s];
        for (i, row) in out.iter_mut().enumerate() {
            let coeff = self.context.coeffs.c[i] + self.context.coeffs.gamma_rows[i];
            for (q, value) in row.iter_mut().enumerate() {
                *value = self.context.h * self.context.f0[q]
                    + self.context.h * self.context.h * coeff * self.context.ft0[q];
            }
        }
        out
    }
    pub fn nonlinear_remainder_snapshot(
        &self,
        k: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<NonlinearRemainderSnapshot> {
        self.validate_rows(k)?;
        let delta = self.stage_mix(&self.context.coeffs.alpha, k);
        let states: Vec<Vec<f64>> = delta
            .iter()
            .map(|d| self.context.y.iter().zip(d).map(|(a, b)| a + b).collect())
            .collect();
        if !states.iter().flatten().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "non-finite predicted stage states".into(),
            ));
        }
        let times: Vec<f64> = self
            .context
            .coeffs
            .c
            .iter()
            .map(|c| self.context.t + c * self.context.h)
            .collect();
        let rhs_values = self
            .context
            .problem
            .eval_rhs_batch(&times, &states, counters)?;
        let jdelta = self.jacobian_rows(&delta, counters)?;
        let mut rhs = self.rhs_base();
        let mut remainder = vec![vec![0.0; self.n]; self.s];
        for stage in 0..self.s {
            for component in 0..self.n {
                remainder[stage][component] = rhs_values[stage][component]
                    - self.context.f0[component]
                    - jdelta[stage][component]
                    - self.context.coeffs.c[stage] * self.context.h * self.context.ft0[component];
                rhs[stage][component] += self.context.h * remainder[stage][component];
            }
        }
        if rhs.iter().flatten().all(|value| value.is_finite()) {
            Ok(NonlinearRemainderSnapshot {
                rhs,
                states,
                rhs_values,
                remainder,
            })
        } else {
            Err(CoreError::NonFinite(
                "non-finite block nonlinear RHS".into(),
            ))
        }
    }

    pub fn nonlinear_rhs(
        &self,
        k: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<NonlinearRhsData> {
        let snapshot = self.nonlinear_remainder_snapshot(k, counters)?;
        Ok((
            snapshot.rhs,
            snapshot.states,
            snapshot.rhs_values,
            snapshot.remainder,
        ))
    }

    pub fn target_residual(
        &self,
        k: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<Vec<f64>>> {
        let applied = self.apply(k, counters)?;
        let snapshot = self.nonlinear_remainder_snapshot(k, counters)?;
        Ok(applied
            .iter()
            .zip(snapshot.rhs)
            .map(|(lhs, rhs)| lhs.iter().zip(rhs).map(|(a, b)| a - b).collect())
            .collect())
    }

    /// Assemble the exact Jacobian of the original nonlinear RODAS5P block residual.
    ///
    /// The frozen block matrix already contains the common Jacobian `J_n`.  The only
    /// nonlinear correction is
    ///
    /// `-h * alpha[i,j] * (J_i - J_n)` for `j < i`,
    ///
    /// where `J_i` is evaluated at the current stage state.  This routine deliberately
    /// requires explicit stage Jacobians: a JVP-only certificate would need a separate
    /// iterative inverse-error analysis and is outside the exact reference layer.
    pub fn target_jacobian_matrix(
        &self,
        k: &[Vec<f64>],
        snapshot: &NonlinearRemainderSnapshot,
        counters: &mut WorkCounters,
    ) -> CoreResult<DenseMatrix> {
        self.validate_rows(k)?;
        if snapshot.states.len() != self.s || snapshot.states.iter().any(|row| row.len() != self.n)
        {
            return Err(CoreError::Dimension(
                "nonlinear remainder snapshot state shape mismatch".into(),
            ));
        }
        let frozen_jacobian = self
            .context
            .jacobian
            .explicit()
            .ok_or_else(|| {
                CoreError::LinearSolve(
                    "exact nonlinear target Jacobian needs explicit frozen J".into(),
                )
            })?
            .clone();
        let mut out = self.explicit_matrix()?;
        for stage in 0..self.s {
            let time = self.context.t + self.context.coeffs.c[stage] * self.context.h;
            let stage_operator =
                self.context
                    .problem
                    .linearize(time, &snapshot.states[stage], counters)?;
            let stage_jacobian = stage_operator
                .explicit()
                .ok_or_else(|| {
                    CoreError::LinearSolve(
                        "exact nonlinear target Jacobian needs explicit stage J".into(),
                    )
                })?
                .clone();
            for previous in 0..stage {
                let alpha = self.context.coeffs.alpha[(stage, previous)];
                if alpha == 0.0 {
                    continue;
                }
                for row in 0..self.n {
                    for column in 0..self.n {
                        out[(stage * self.n + row, previous * self.n + column)] -= self.context.h
                            * alpha
                            * (stage_jacobian[(row, column)] - frozen_jacobian[(row, column)]);
                    }
                }
            }
        }
        if out.as_slice().iter().all(|value| value.is_finite()) {
            Ok(out)
        } else {
            Err(CoreError::NonFinite(
                "nonlinear target Jacobian contains NaN/Inf".into(),
            ))
        }
    }

    fn shifted_factor(&self, counters: &mut WorkCounters) -> CoreResult<LuFactorization> {
        let w =
            self.context.shifted.explicit().ok_or_else(|| {
                CoreError::LinearSolve("block direct action needs explicit W".into())
            })?;
        counters.direct_factorizations += 1;
        LuFactorization::new(w)
    }
    #[allow(clippy::too_many_arguments)]
    fn residual_report(
        &self,
        rhs: &[Vec<f64>],
        k: &[Vec<f64>],
        counters: &mut WorkCounters,
        method: String,
        iterations: u64,
        matvecs: u64,
        pcapps: u64,
        terms: usize,
    ) -> CoreResult<BlockSolveReport> {
        let ak = self.raw_apply(k)?;
        counters.diagnostic_matvecs += 1;
        let residual: Vec<f64> = flatten(rhs)
            .into_iter()
            .zip(flatten(&ak))
            .map(|(a, b)| a - b)
            .collect();
        let rn = safe_l2(&residual);
        let bn = safe_l2(&flatten(rhs));
        Ok(BlockSolveReport {
            stages: k.to_vec(),
            converged: rn.is_finite(),
            info: 0,
            residual_norm: rn,
            relative_residual: rn / bn.max(f64::MIN_POSITIVE),
            iterations,
            matvecs,
            preconditioner_apps: pcapps,
            method,
            polynomial_terms: terms,
        })
    }

    pub fn forward_solve(
        &self,
        rhs: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<BlockSolveReport> {
        self.validate_rows(rhs)?;
        counters.block_linear_solves += 1;
        let factor = self.shifted_factor(counters)?;
        let mut k = vec![vec![0.0; self.n]; self.s];
        for i in 0..self.s {
            let mut mix = vec![0.0; self.n];
            for (j, stage) in k.iter().enumerate().take(i) {
                let a = self.context.coeffs.l[(i, j)];
                for q in 0..self.n {
                    mix[q] += a * stage[q];
                }
            }
            let mut jmix = vec![0.0; self.n];
            if mix.iter().any(|v| *v != 0.0) {
                self.context.jacobian.apply(&mix, &mut jmix)?;
                counters.jvp_calls += 1;
                counters.jvp_vectors += 1;
            }
            let row: Vec<f64> = (0..self.n)
                .map(|q| rhs[i][q] + self.context.h * jmix[q])
                .collect();
            k[i] = factor.solve(&row)?;
            counters.direct_solve_calls += 1;
        }
        self.residual_report(rhs, &k, counters, "forward".into(), 0, 0, 0, 0)
    }
    pub fn explicit_solve(
        &self,
        rhs: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<BlockSolveReport> {
        self.validate_rows(rhs)?;
        counters.block_linear_solves += 1;
        counters.direct_factorizations += 1;
        let factor = LuFactorization::new(&self.explicit_matrix()?)?;
        let x = factor.solve(&flatten(rhs))?;
        counters.direct_solve_calls += 1;
        let k = unflatten(&x, self.s, self.n);
        self.residual_report(rhs, &k, counters, "explicit".into(), 0, 0, 0, 0)
    }
    pub fn nilpotent_solve(
        &self,
        rhs: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<BlockSolveReport> {
        self.validate_rows(rhs)?;
        counters.block_linear_solves += 1;
        let factor = self.shifted_factor(counters)?;
        let mut x = factor.solve_rows(rhs)?;
        counters.direct_solve_calls += 1;
        let mut total = x.clone();
        let mut terms = 1;
        for _ in 1..self.s {
            let mixed = self.stage_mix(&self.context.coeffs.l, &x);
            let j = self.jacobian_rows(&mixed, counters)?;
            let scaled: Vec<Vec<f64>> = j
                .into_iter()
                .map(|r| r.into_iter().map(|v| self.context.h * v).collect())
                .collect();
            x = factor.solve_rows(&scaled)?;
            counters.direct_solve_calls += 1;
            for i in 0..self.s {
                for q in 0..self.n {
                    total[i][q] += x[i][q];
                }
            }
            terms += 1;
        }
        self.residual_report(rhs, &total, counters, "nilpotent".into(), 0, 0, 0, terms)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn gmres_solve(
        &self,
        rhs: &[Vec<f64>],
        rtol: f64,
        atol: f64,
        restart: usize,
        max_arnoldi: usize,
        preconditioner: BlockPreconditioner,
        x0: Option<&[Vec<f64>]>,
        counters: &mut WorkCounters,
    ) -> CoreResult<BlockSolveReport> {
        self.validate_rows(rhs)?;
        counters.block_linear_solves += 1;
        let op = BlockOperator { system: self };
        let factor = if preconditioner == BlockPreconditioner::Direct {
            Some(self.shifted_factor(counters)?)
        } else {
            None
        };
        let pc: Box<dyn Preconditioner> = match preconditioner {
            BlockPreconditioner::None => Box::new(IdentityPreconditioner::new(self.s * self.n)),
            BlockPreconditioner::Direct => Box::new(BlockDirectPc {
                factor: factor.expect("factor"),
                s: self.s,
                n: self.n,
            }),
            BlockPreconditioner::Jacobi => {
                let w = self.context.shifted.explicit().ok_or_else(|| {
                    CoreError::LinearSolve("block Jacobi needs explicit W".into())
                })?;
                Box::new(BlockJacobiPc::new(w, self.s)?)
            }
        };
        let before = *counters;
        let report = solve_gmres(
            &op,
            pc.as_ref(),
            &flatten(rhs),
            x0.map(flatten_ref).as_deref(),
            &GmresConfig {
                restart,
                max_arnoldi,
                rtol,
                atol,
            },
            counters,
        )?;
        let d = counters.delta(before);
        counters.block_linear_iterations += report.iterations;
        counters.block_matvecs += d.linear_matvecs;
        counters.block_preconditioner_apps += d.preconditioner_apps;
        let k = unflatten(&report.x, self.s, self.n);
        let mut out = self.residual_report(
            rhs,
            &k,
            counters,
            format!("gmres-{preconditioner:?}"),
            report.iterations,
            d.linear_matvecs,
            d.preconditioner_apps,
            0,
        )?;
        out.converged = report.converged;
        Ok(out)
    }
    pub fn solve(
        &self,
        rhs: &[Vec<f64>],
        method: BlockMethod,
        counters: &mut WorkCounters,
    ) -> CoreResult<BlockSolveReport> {
        match method {
            BlockMethod::Forward => self.forward_solve(rhs, counters),
            BlockMethod::Explicit => self.explicit_solve(rhs, counters),
            BlockMethod::Nilpotent => self.nilpotent_solve(rhs, counters),
            BlockMethod::Gmres => self.gmres_solve(
                rhs,
                1e-11,
                1e-13,
                40,
                100,
                BlockPreconditioner::Direct,
                None,
                counters,
            ),
        }
    }
}

struct BlockOperator<'a, 'ctx, 'p> {
    system: &'a StructuredBlockSystem<'ctx, 'p>,
}
impl LinearOperator for BlockOperator<'_, '_, '_> {
    fn dimension(&self) -> usize {
        self.system.s * self.system.n
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let k = unflatten(x, self.system.s, self.system.n);
        let out = self.system.raw_apply(&k)?;
        y.copy_from_slice(&flatten(&out));
        Ok(())
    }
    fn token(&self) -> u64 {
        self.system.context.shifted.token() ^ 0xB10C_5A5A
    }
}
struct BlockDirectPc {
    factor: LuFactorization,
    s: usize,
    n: usize,
}
impl Preconditioner for BlockDirectPc {
    fn dimension(&self) -> usize {
        self.s * self.n
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let rows = unflatten(x, self.s, self.n);
        let out = self.factor.solve_rows(&rows)?;
        y.copy_from_slice(&flatten(&out));
        Ok(())
    }
}
struct BlockJacobiPc {
    inv: Vec<f64>,
    s: usize,
}
impl BlockJacobiPc {
    fn new(w: &DenseMatrix, s: usize) -> CoreResult<Self> {
        let diag = w.diagonal()?;
        let scale = diag.iter().fold(0.0_f64, |a, b| a.max(b.abs())).max(1.0);
        if diag.iter().any(|v| v.abs() <= f64::EPSILON * scale) {
            return Err(CoreError::LinearSolve("zero block-Jacobi diagonal".into()));
        }
        Ok(Self {
            inv: diag.into_iter().map(|v| 1.0 / v).collect(),
            s,
        })
    }
}
impl Preconditioner for BlockJacobiPc {
    fn dimension(&self) -> usize {
        self.s * self.inv.len()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.dimension() || y.len() != self.dimension() {
            return Err(CoreError::Dimension("block Jacobi shape mismatch".into()));
        }
        for i in 0..self.s {
            for q in 0..self.inv.len() {
                y[i * self.inv.len() + q] = self.inv[q] * x[i * self.inv.len() + q];
            }
        }
        Ok(())
    }
}

pub fn flatten(rows: &[Vec<f64>]) -> Vec<f64> {
    rows.iter().flatten().copied().collect()
}
fn flatten_ref(rows: &[Vec<f64>]) -> Vec<f64> {
    flatten(rows)
}
pub fn unflatten(x: &[f64], s: usize, n: usize) -> Vec<Vec<f64>> {
    (0..s).map(|i| x[i * n..(i + 1) * n].to_vec()).collect()
}
