//! A lean adaptive RODAS5P driver (research node
//! `research/stiff_rodas5p_fast_20261002`).
//!
//! The profile of `research/stiff_rodas5p_profile_20261002` found the
//! sequential path spending most of its instructions outside the method:
//! 37-52% inside malloc/free on small problems (per-stage vectors, step
//! copies), 12-17% in a heap-allocating small-matrix solve, and, at n = 400,
//! 27% in unvectorized dense matrix-vector products (a diagnostic residual
//! after every direct stage solve and a Jacobian product for the gamma terms
//! of every stage). This driver integrates the same method with:
//!
//! * the transformed stage variables `u_i = sum_j Gamma_ij k_j` of the
//!   snapshot's own coefficients (`a`, `c_matrix`, `b_code`): with
//!   `W = I/(h gamma) - J`,
//!   `W u_i = f(t + c_i h, y + sum_j a_ij u_j) + sum_j (C_ij / h) u_j + gamma_i h f_t`,
//!   `y_new = y + sum_j b_code_j u_j`, and the embedded error is `u_s`
//!   (`btilde` is the last row of Gamma), so no Jacobian product is formed;
//! * no residual product after a direct solve (it never decided acceptance);
//! * one workspace for the whole integration, so a step allocates nothing
//!   beyond what the user's Jacobian callback returns;
//! * an in-place partial-pivoting LU that skips zero multipliers (banded and
//!   small matrices), or faer's blocked LU for dense matrices above 64 rows;
//! * the Jacobian, `f(t, y)` and `f_t` reused after a rejected attempt from
//!   the same state.
//!
//! The step-size controller, the represented-clock rules and the output
//! collection are the sequential driver's own functions, unchanged. The
//! sequential path itself is untouched; this driver has its own identifier
//! and records.

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LuFactorization, WorkCounters, rodas5p_coefficients,
};
use serde::Serialize;

use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, ObservedIntegrationResult,
    OdeProblem, OutputSchedule, output::OutputCollector, rodas_next_step_after_attempt,
};

/// Identifier of this driver in benchmark and research records.
pub const RODAS5P_FAST_DRIVER_ID: &str = "rodas5p-fast-transformed-v1";

/// Matrices with at most this many rows always use the in-place LU.
pub const RODAS5P_FAST_SMALL_LU_MAX: usize = 64;

/// Above [`RODAS5P_FAST_SMALL_LU_MAX`], the in-place (zero-skipping) LU is
/// used when at most this fraction of the first Jacobian's entries is
/// nonzero; denser matrices use faer.
pub const RODAS5P_FAST_SPARSE_DENSITY_MAX: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rodas5pFastLu {
    /// Row-major partial-pivoting LU in the workspace; zero multipliers skip
    /// their row update.
    InPlaceZeroSkipping,
    /// faer's blocked partial-pivoting LU (one allocation per factorization).
    Faer,
}

#[derive(Clone, Debug)]
pub struct Rodas5pFastResult {
    pub observed: ObservedIntegrationResult,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    /// Attempts that reused the Jacobian, `f(t, y)` and `f_t` of a rejected
    /// attempt from the same state.
    pub jacobian_reuses: usize,
    pub lu: Rodas5pFastLu,
    pub driver: &'static str,
}

/// The stage arithmetic of one integration, allocated once.
struct Workspace {
    n: usize,
    s: usize,
    gamma: f64,
    a: Vec<f64>,
    c_matrix: Vec<f64>,
    c: Vec<f64>,
    b_code: Vec<f64>,
    gamma_rows: Vec<f64>,
    lu_kind: Rodas5pFastLu,
    /// Row-major `W` and its in-place factors (in-place LU).
    w: Vec<f64>,
    pivots: Vec<usize>,
    /// `W` for faer, and its factorization.
    w_dense: DenseMatrix,
    faer_lu: Option<LuFactorization>,
    jacobian: Option<DenseMatrix>,
    f0: Vec<f64>,
    ft: Vec<f64>,
    u: Vec<f64>,
    stage_state: Vec<f64>,
    stage_rhs: Vec<f64>,
    y_new: Vec<f64>,
}

impl Workspace {
    fn new(n: usize) -> CoreResult<Self> {
        let coeffs = rodas5p_coefficients()?;
        let s = coeffs.stages();
        let flat = |m: &DenseMatrix| m.as_slice().to_vec();
        for i in 0..s {
            for j in i..s {
                if coeffs.a[(i, j)] != 0.0 || coeffs.c_matrix[(i, j)] != 0.0 {
                    return Err(CoreError::Coefficients(
                        "RODAS5P transformed coefficients are not strictly lower triangular".into(),
                    ));
                }
            }
        }
        Ok(Self {
            n,
            s,
            gamma: coeffs.gamma,
            a: flat(&coeffs.a),
            c_matrix: flat(&coeffs.c_matrix),
            c: coeffs.c.clone(),
            b_code: coeffs.b_code.clone(),
            gamma_rows: coeffs.gamma_rows.clone(),
            lu_kind: Rodas5pFastLu::InPlaceZeroSkipping,
            w: vec![0.0; n * n],
            pivots: vec![0; n],
            w_dense: DenseMatrix::zeros(0, 0),
            faer_lu: None,
            jacobian: None,
            f0: vec![0.0; n],
            ft: vec![0.0; n],
            u: vec![0.0; s * n],
            stage_state: vec![0.0; n],
            stage_rhs: vec![0.0; n],
            y_new: vec![0.0; n],
        })
    }

    fn choose_lu(&mut self, jacobian: &DenseMatrix) {
        let n = self.n;
        let nonzero = jacobian.as_slice().iter().filter(|v| **v != 0.0).count();
        let density = nonzero as f64 / (n * n) as f64;
        self.lu_kind =
            if n <= RODAS5P_FAST_SMALL_LU_MAX || density <= RODAS5P_FAST_SPARSE_DENSITY_MAX {
                Rodas5pFastLu::InPlaceZeroSkipping
            } else {
                self.w_dense = DenseMatrix::zeros(n, n);
                Rodas5pFastLu::Faer
            };
    }

    /// Form `W = I/(h gamma) - J` and factor it.
    fn factor(&mut self, h: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let n = self.n;
        let inv = 1.0 / (h * self.gamma);
        let jacobian = self
            .jacobian
            .as_ref()
            .expect("Jacobian before factorization");
        let target = match self.lu_kind {
            Rodas5pFastLu::InPlaceZeroSkipping => self.w.as_mut_slice(),
            Rodas5pFastLu::Faer => self.w_dense.as_mut_slice(),
        };
        for (w, j) in target.iter_mut().zip(jacobian.as_slice()) {
            *w = -j;
        }
        for i in 0..n {
            target[i * n + i] += inv;
        }
        counters.direct_factorizations += 1;
        match self.lu_kind {
            Rodas5pFastLu::InPlaceZeroSkipping => lu_in_place(&mut self.w, n, &mut self.pivots),
            Rodas5pFastLu::Faer => {
                self.faer_lu = Some(LuFactorization::new(&self.w_dense)?);
                Ok(())
            }
        }
    }

    /// Solve `W x = stage_rhs` in place.
    fn solve(&mut self, counters: &mut WorkCounters) -> CoreResult<()> {
        counters.linear_solves += 1;
        counters.direct_solve_calls += 1;
        match self.lu_kind {
            Rodas5pFastLu::InPlaceZeroSkipping => {
                lu_solve_in_place(&self.w, self.n, &self.pivots, &mut self.stage_rhs);
            }
            Rodas5pFastLu::Faer => {
                let x = self
                    .faer_lu
                    .as_ref()
                    .expect("factored")
                    .solve(&self.stage_rhs)?;
                self.stage_rhs.copy_from_slice(&x);
            }
        }
        if self.stage_rhs.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::LinearSolve(
                "RODAS5P fast stage solve produced NaN/Inf".into(),
            ))
        }
    }

    /// One attempt from `(t, y)` with step `h`: the new state in `y_new`
    /// and the WRMS norm of the embedded error. `fresh` builds the Jacobian,
    /// `f(t, y)` and `f_t` at this state; otherwise those of the previous
    /// attempt from the same state are reused.
    #[allow(clippy::too_many_arguments)]
    fn attempt(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        fresh: bool,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        let (n, s) = (self.n, self.s);
        if fresh {
            let jacobian = problem.dense_jacobian(t, y, counters)?;
            if self.jacobian.is_none() {
                self.choose_lu(&jacobian);
            }
            self.jacobian = Some(jacobian);
            problem.eval_rhs_into(t, y, &mut self.f0, counters)?;
            if !problem.autonomous {
                let ft = problem.eval_partial_t(t, y, counters)?;
                self.ft.copy_from_slice(&ft);
            }
        }
        self.factor(h, counters)?;
        for i in 0..s {
            if i == 0 {
                self.stage_rhs.copy_from_slice(&self.f0);
            } else {
                self.stage_state.copy_from_slice(y);
                for j in 0..i {
                    let aij = self.a[i * s + j];
                    if aij != 0.0 {
                        let uj = &self.u[j * n..(j + 1) * n];
                        for (x, v) in self.stage_state.iter_mut().zip(uj) {
                            *x += aij * v;
                        }
                    }
                }
                problem.eval_rhs_into(
                    t + self.c[i] * h,
                    &self.stage_state,
                    &mut self.stage_rhs,
                    counters,
                )?;
            }
            for j in 0..i {
                let cij = self.c_matrix[i * s + j] / h;
                if cij != 0.0 {
                    let uj = &self.u[j * n..(j + 1) * n];
                    for (x, v) in self.stage_rhs.iter_mut().zip(uj) {
                        *x += cij * v;
                    }
                }
            }
            if !problem.autonomous {
                let g = self.gamma_rows[i] * h;
                for (x, v) in self.stage_rhs.iter_mut().zip(&self.ft) {
                    *x += g * v;
                }
            }
            self.solve(counters)?;
            self.u[i * n..(i + 1) * n].copy_from_slice(&self.stage_rhs);
        }
        self.y_new.copy_from_slice(y);
        for j in 0..s {
            let bj = self.b_code[j];
            if bj != 0.0 {
                let uj = &self.u[j * n..(j + 1) * n];
                for (x, v) in self.y_new.iter_mut().zip(uj) {
                    *x += bj * v;
                }
            }
        }
        if !self.y_new.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "RODAS5P fast step produced NaN/Inf".into(),
            ));
        }
        // WRMS of the embedded error u_s in the sequential path's scale
        // atol + rtol max(|y|, |y_new|).
        let error = &self.u[(s - 1) * n..s * n];
        let mut sum = 0.0;
        for ((e, a), b) in error.iter().zip(y).zip(&self.y_new) {
            let z = e / (atol + rtol * a.abs().max(b.abs()));
            sum += z * z;
        }
        let norm = (sum / n as f64).sqrt();
        Ok(if norm.is_finite() {
            norm
        } else {
            f64::INFINITY
        })
    }
}

/// Row-major partial-pivoting LU of `a` in place; `pivots[k]` is the row
/// swapped with row `k`. Zero multipliers skip their row update.
fn lu_in_place(a: &mut [f64], n: usize, pivots: &mut [usize]) -> CoreResult<()> {
    for k in 0..n {
        let mut p = k;
        let mut max = a[k * n + k].abs();
        for i in k + 1..n {
            let v = a[i * n + k].abs();
            if v > max {
                max = v;
                p = i;
            }
        }
        if !(max > 0.0 && max.is_finite()) {
            return Err(CoreError::LinearSolve(format!(
                "RODAS5P fast LU: singular or non-finite pivot at column {k}"
            )));
        }
        pivots[k] = p;
        if p != k {
            for j in 0..n {
                a.swap(k * n + j, p * n + j);
            }
        }
        let (top, bottom) = a.split_at_mut((k + 1) * n);
        let pivot_row = &top[k * n..(k + 1) * n];
        let pivot = pivot_row[k];
        for row in bottom.chunks_exact_mut(n) {
            if row[k] == 0.0 {
                continue;
            }
            let l = row[k] / pivot;
            row[k] = l;
            for (x, r) in row[k + 1..].iter_mut().zip(&pivot_row[k + 1..]) {
                *x -= l * r;
            }
        }
    }
    Ok(())
}

/// Solve with the factors of [`lu_in_place`], overwriting `b`.
fn lu_solve_in_place(lu: &[f64], n: usize, pivots: &[usize], b: &mut [f64]) {
    for (k, &p) in pivots.iter().enumerate().take(n) {
        b.swap(k, p);
    }
    for i in 0..n {
        let row = &lu[i * n..i * n + i];
        let mut sum = b[i];
        for (l, x) in row.iter().zip(&b[..i]) {
            sum -= l * x;
        }
        b[i] = sum;
    }
    for i in (0..n).rev() {
        let row = &lu[i * n..(i + 1) * n];
        let mut sum = b[i];
        for (u, x) in row[i + 1..].iter().zip(&b[i + 1..]) {
            sum -= u * x;
        }
        b[i] = sum / row[i];
    }
}

/// One step of this driver from `(t, y)` with step `h`: the new state, the
/// WRMS norm of the embedded error and the LU used. For verification
/// against [`crate::sequential_step`]; the adaptive driver keeps its
/// workspace across steps instead.
pub fn rodas5p_fast_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    atol: f64,
    rtol: f64,
    counters: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, f64, Rodas5pFastLu)> {
    if y.len() != problem.dimension || problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P fast step input".into(),
        ));
    }
    let mut work = Workspace::new(problem.dimension)?;
    let error = work.attempt(problem, t, y, h, true, atol, rtol, counters)?;
    Ok((work.y_new, error, work.lu_kind))
}

fn failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        _ => None,
    }
}

/// Adaptive RODAS5P with the sequential driver's controller and output
/// rules, on the transformed stages and one workspace. Requires the
/// identity mass matrix.
pub fn integrate_rodas5p_fast_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Rodas5pFastResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if tf < t || y0.len() != problem.dimension {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P fast integration input".into(),
        ));
    }
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "the RODAS5P fast driver supports the identity mass matrix only".into(),
        ));
    }
    let mut work = Workspace::new(problem.dimension)?;
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut collector =
        OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
    let (mut attempts, mut accepted_steps, mut rejected_steps, mut reuses) = (0, 0, 0, 0);
    let mut internal_steps = 0_usize;
    // Whether the Jacobian, f(t, y) and f_t in the workspace belong to (t, y).
    let mut fresh_state = false;
    while t < tf && attempts < adaptive.max_attempts {
        let Some(next_h) =
            crate::output::adaptive_end_step(t, h, tf, adaptive.step_cap(), &controller)?
        else {
            break;
        };
        h = next_h;
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf) || t + h == t {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        attempts += 1;
        if fresh_state {
            reuses += 1;
        }
        let outcome = work.attempt(
            problem,
            t,
            &y,
            trial_h,
            !fresh_state,
            adaptive.atol,
            adaptive.rtol,
            &mut counters,
        );
        // Every path below has the state's Jacobian and f(t, y) in place,
        // unless they failed to build.
        fresh_state = work.jacobian.is_some();
        let (error, failure) = match outcome {
            Ok(error) if error <= 1.0 => (error, None),
            Ok(error) => (error, Some(AdaptiveFailureKind::LocalError)),
            Err(error) => match failure_kind(&error) {
                Some(kind) => {
                    fresh_state = false;
                    (f64::INFINITY, Some(kind))
                }
                None => return Err(error),
            },
        };
        match failure {
            None => {
                counters.accepted_steps += 1;
                accepted_steps += 1;
                t += trial_h;
                y.copy_from_slice(&work.y_new);
                collector.accept(t, &y, clipped)?;
                internal_steps += 1;
                fresh_state = false;
            }
            Some(kind) => {
                counters.rejected_steps += 1;
                rejected_steps += 1;
                match kind {
                    AdaptiveFailureKind::LocalError => counters.local_error_failures += 1,
                    AdaptiveFailureKind::LinearSolve => counters.linear_solve_failures += 1,
                    AdaptiveFailureKind::NonlinearSolve => counters.nonlinear_solve_failures += 1,
                    AdaptiveFailureKind::NonFinite => counters.nonfinite_step_failures += 1,
                }
            }
        }
        h = rodas_next_step_after_attempt(
            &mut controller,
            adaptive,
            h,
            trial_h,
            error,
            failure.is_none(),
            clipped,
        )?;
    }
    let success = t >= tf;
    let (times, states, output_clipped_steps) = if success {
        collector.finish()?
    } else {
        collector.finish_partial()
    };
    Ok(Rodas5pFastResult {
        observed: ObservedIntegrationResult {
            t: times,
            y: states,
            success,
            message: if success {
                "success".into()
            } else {
                "maximum step count or minimum step reached".into()
            },
            counters,
            internal_steps,
            output_clipped_steps,
        },
        attempts,
        accepted_steps,
        rejected_steps,
        jacobian_reuses: reuses,
        lu: work.lu_kind,
        driver: RODAS5P_FAST_DRIVER_ID,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_place_lu_solves_pivoted_and_banded_systems() {
        // Needs a row swap at the first column.
        let n = 4;
        let a = [
            0.0, 2.0, 1.0, 0.0, //
            3.0, 1.0, 0.0, 0.0, //
            1.0, 0.0, 4.0, 1.0, //
            0.0, 0.0, 1.0, 5.0,
        ];
        let x = [1.0, -2.0, 0.5, 3.0];
        let b: Vec<f64> = (0..n)
            .map(|i| (0..n).map(|j| a[i * n + j] * x[j]).sum())
            .collect();
        let mut lu = a.to_vec();
        let mut piv = vec![0; n];
        lu_in_place(&mut lu, n, &mut piv).unwrap();
        let mut sol = b.clone();
        lu_solve_in_place(&lu, n, &piv, &mut sol);
        for (s, e) in sol.iter().zip(x) {
            assert!((s - e).abs() <= 1e-14, "{sol:?}");
        }
        // A singular matrix is a typed linear-solve error.
        let mut singular = vec![1.0, 2.0, 2.0, 4.0];
        assert!(matches!(
            lu_in_place(&mut singular, 2, &mut [0, 0]),
            Err(CoreError::LinearSolve(_))
        ));
    }
}
