//! The fast RODAS5P driver specialized to a compile-time dimension with
//! statically dispatched problem functions (research node
//! `research/thread_transfer_smalln_cost_20261002`, thread-transfer DAG node
//! P1-SMALLN-COST). Research only.
//!
//! The arithmetic is that of [`crate::integrate_rodas5p_fast_observed`]
//! (v2): the transformed stages, the same coefficient order, the same
//! partial-pivoting LU with tracked row extents and zero-multiplier skipping,
//! the same reuse of `J`, `f(t, y)` and `f_t` after a rejection, the same
//! finite checks, counters, controller, represented-clock and output rules.
//! Only the storage (`[f64; N]`, `[[f64; N]; N]` on the stack) and the
//! dispatch of the problem (a [`SmallProblem`] type parameter instead of
//! `Arc<dyn Fn>` callbacks) differ, so its results equal v2's as values.

use rodas5p_core::{CoreError, CoreResult, WorkCounters, rodas5p_coefficients};

use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, ObservedIntegrationResult,
    OutputSchedule, output::OutputCollector, rodas_next_step_after_attempt,
};

/// Identifier of this driver in research records.
pub const RODAS5P_FAST_SMALL_DRIVER_ID: &str = "rodas5p-fast-small-static-v1";

const STAGES: usize = 8;

/// An autonomous ODE (time is not passed) of fixed dimension `N` with an in-place Jacobian that
/// writes a fixed sparsity pattern (as `OdeProblem::with_jacobian_into`).
pub trait SmallProblem<const N: usize> {
    fn rhs(&self, y: &[f64; N], out: &mut [f64; N]);
    fn jacobian(&self, y: &[f64; N], out: &mut [[f64; N]; N]);
}

#[derive(Clone, Debug)]
pub struct Rodas5pFastSmallResult {
    pub observed: ObservedIntegrationResult,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub jacobian_reuses: usize,
    pub driver: &'static str,
}

struct Workspace<const N: usize> {
    gamma: f64,
    a_nonzero: Vec<Vec<(usize, f64)>>,
    c_nonzero: Vec<Vec<(usize, f64)>>,
    b_nonzero: Vec<(usize, f64)>,
    w: [[f64; N]; N],
    pivots: [usize; N],
    row_end: [usize; N],
    l_start: [usize; N],
    jacobian: [[f64; N]; N],
    f0: [f64; N],
    u: [[f64; N]; STAGES],
    stage_state: [f64; N],
    stage_rhs: [f64; N],
    y_new: [f64; N],
    built: bool,
}

fn rhs_counted<const N: usize, P: SmallProblem<N>>(
    problem: &P,
    y: &[f64; N],
    out: &mut [f64; N],
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    problem.rhs(y, out);
    if !out.iter().all(|v| v.is_finite()) {
        return Err(CoreError::NonFinite("RHS produced NaN/Inf".into()));
    }
    counters.rhs_calls += 1;
    counters.rhs_evaluations += 1;
    Ok(())
}

impl<const N: usize> Workspace<N> {
    fn new() -> CoreResult<Self> {
        let coeffs = rodas5p_coefficients()?;
        if coeffs.stages() != STAGES {
            return Err(CoreError::Coefficients("RODAS5P has eight stages".into()));
        }
        for i in 0..STAGES {
            for j in i..STAGES {
                if coeffs.a[(i, j)] != 0.0 || coeffs.c_matrix[(i, j)] != 0.0 {
                    return Err(CoreError::Coefficients(
                        "RODAS5P transformed coefficients are not strictly lower triangular".into(),
                    ));
                }
            }
        }
        let nonzero_row = |m: &rodas5p_core::DenseMatrix, i: usize| {
            (0..i)
                .filter(|&j| m[(i, j)] != 0.0)
                .map(|j| (j, m[(i, j)]))
                .collect::<Vec<_>>()
        };
        Ok(Self {
            gamma: coeffs.gamma,
            a_nonzero: (0..STAGES).map(|i| nonzero_row(&coeffs.a, i)).collect(),
            c_nonzero: (0..STAGES)
                .map(|i| nonzero_row(&coeffs.c_matrix, i))
                .collect(),
            b_nonzero: coeffs
                .b_code
                .iter()
                .enumerate()
                .filter(|(_, b)| **b != 0.0)
                .map(|(j, b)| (j, *b))
                .collect(),
            w: [[0.0; N]; N],
            pivots: [0; N],
            row_end: [0; N],
            l_start: [N; N],
            jacobian: [[0.0; N]; N],
            f0: [0.0; N],
            u: [[0.0; N]; STAGES],
            stage_state: [0.0; N],
            stage_rhs: [0.0; N],
            y_new: [0.0; N],
            built: false,
        })
    }

    /// `W = I/(h gamma) - J` and its in-place factors (v2's operation order).
    fn factor(&mut self, h: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let inv = 1.0 / (h * self.gamma);
        for (w_row, j_row) in self.w.iter_mut().zip(&self.jacobian) {
            for (w, j) in w_row.iter_mut().zip(j_row) {
                *w = -j;
            }
        }
        for i in 0..N {
            self.w[i][i] += inv;
        }
        counters.direct_factorizations += 1;
        for i in 0..N {
            let last = self.w[i].iter().rposition(|v| *v != 0.0).unwrap_or(0);
            self.row_end[i] = last.max(i);
            self.l_start[i] = N;
        }
        lu_in_place(
            &mut self.w,
            &mut self.pivots,
            &mut self.row_end,
            &mut self.l_start,
        )
    }

    fn solve(&mut self, counters: &mut WorkCounters) -> CoreResult<()> {
        counters.linear_solves += 1;
        counters.direct_solve_calls += 1;
        lu_solve_in_place(
            &self.w,
            &self.pivots,
            &self.row_end,
            &self.l_start,
            &mut self.stage_rhs,
        );
        if self.stage_rhs.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::LinearSolve(
                "RODAS5P fast stage solve produced NaN/Inf".into(),
            ))
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn attempt<P: SmallProblem<N>>(
        &mut self,
        problem: &P,
        y: &[f64; N],
        h: f64,
        fresh: bool,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        if fresh {
            counters.jacobian_builds += 1;
            problem.jacobian(y, &mut self.jacobian);
            self.built = true;
            rhs_counted(problem, y, &mut self.f0, counters)?;
        }
        self.factor(h, counters)?;
        for i in 0..STAGES {
            if i == 0 {
                self.stage_rhs = self.f0;
            } else {
                self.stage_state = *y;
                for &(j, aij) in &self.a_nonzero[i] {
                    for (x, v) in self.stage_state.iter_mut().zip(&self.u[j]) {
                        *x += aij * v;
                    }
                }
                let state = self.stage_state;
                rhs_counted(problem, &state, &mut self.stage_rhs, counters)?;
            }
            for &(j, c) in &self.c_nonzero[i] {
                let cij = c / h;
                if cij != 0.0 {
                    for (x, v) in self.stage_rhs.iter_mut().zip(&self.u[j]) {
                        *x += cij * v;
                    }
                }
            }
            self.solve(counters)?;
            self.u[i] = self.stage_rhs;
        }
        self.y_new = *y;
        for &(j, bj) in &self.b_nonzero {
            for (x, v) in self.y_new.iter_mut().zip(&self.u[j]) {
                *x += bj * v;
            }
        }
        if !self.y_new.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "RODAS5P fast step produced NaN/Inf".into(),
            ));
        }
        let error = &self.u[STAGES - 1];
        let mut sum = 0.0;
        for ((e, a), b) in error.iter().zip(y).zip(&self.y_new) {
            let z = e / (atol + rtol * a.abs().max(b.abs()));
            sum += z * z;
        }
        let norm = (sum / N as f64).sqrt();
        Ok(if norm.is_finite() {
            norm
        } else {
            f64::INFINITY
        })
    }
}

/// v2's `lu_in_place` on `[[f64; N]; N]`, the same operations in the same
/// order.
fn lu_in_place<const N: usize>(
    a: &mut [[f64; N]; N],
    pivots: &mut [usize; N],
    row_end: &mut [usize; N],
    l_start: &mut [usize; N],
) -> CoreResult<()> {
    for k in 0..N {
        let mut p = k;
        let mut max = a[k][k].abs();
        for (i, row) in a.iter().enumerate().skip(k + 1) {
            let v = row[k].abs();
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
            let span = row_end[k].max(row_end[p]) + 1;
            let (upper, lower) = a.split_at_mut(p);
            upper[k][..span].swap_with_slice(&mut lower[0][..span]);
            row_end.swap(k, p);
            l_start.swap(k, p);
        }
        let pivot_end = row_end[k];
        let (top, bottom) = a.split_at_mut(k + 1);
        let pivot_row = &top[k];
        let pivot = pivot_row[k];
        for (offset, row) in bottom.iter_mut().enumerate() {
            if row[k] == 0.0 {
                continue;
            }
            let i = k + 1 + offset;
            let l = row[k] / pivot;
            row[k] = l;
            l_start[i] = l_start[i].min(k);
            for (x, r) in row[k + 1..=pivot_end]
                .iter_mut()
                .zip(&pivot_row[k + 1..=pivot_end])
            {
                *x -= l * r;
            }
            row_end[i] = row_end[i].max(pivot_end);
        }
    }
    Ok(())
}

/// v2's `lu_solve_in_place` on fixed arrays.
fn lu_solve_in_place<const N: usize>(
    lu: &[[f64; N]; N],
    pivots: &[usize; N],
    row_end: &[usize; N],
    l_start: &[usize; N],
    b: &mut [f64; N],
) {
    for (k, &p) in pivots.iter().enumerate() {
        b.swap(k, p);
    }
    for i in 0..N {
        let start = l_start[i].min(i);
        let mut sum = b[i];
        for (l, x) in lu[i][start..i].iter().zip(&b[start..i]) {
            sum -= l * x;
        }
        b[i] = sum;
    }
    for i in (0..N).rev() {
        let end = row_end[i];
        let row = &lu[i][i..=end];
        let mut sum = b[i];
        for (u, x) in row[1..].iter().zip(&b[i + 1..=end]) {
            sum -= u * x;
        }
        b[i] = sum / row[0];
    }
}

fn failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        _ => None,
    }
}

/// [`crate::integrate_rodas5p_fast_observed`] for a [`SmallProblem`] of
/// dimension `N`.
pub fn integrate_rodas5p_fast_small_observed<const N: usize, P: SmallProblem<N>>(
    problem: &P,
    t_span: (f64, f64),
    y0: &[f64; N],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Rodas5pFastSmallResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if tf < t || !y0.iter().all(|v| v.is_finite()) {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P fast integration input".into(),
        ));
    }
    let mut work = Workspace::<N>::new()?;
    let mut y = *y0;
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut collector =
        OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
    let (mut attempts, mut accepted_steps, mut rejected_steps, mut reuses) = (0, 0, 0, 0);
    let mut internal_steps = 0_usize;
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
            &y,
            trial_h,
            !fresh_state,
            adaptive.atol,
            adaptive.rtol,
            &mut counters,
        );
        fresh_state = work.built;
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
                y = work.y_new;
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
    Ok(Rodas5pFastSmallResult {
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
        driver: RODAS5P_FAST_SMALL_DRIVER_ID,
    })
}
