//! A JVP-only RODAS5P driver on the raw transformed stages (research node
//! `research/thread_transfer_mf_workspace_20261002`, thread-transfer DAG node
//! P1-MF-WORKSPACE). Research only.
//!
//! The strict matrix-free sequential path forms, in every stage after the
//! first, the `Gamma` mixture of the previous K stages and applies one JVP to
//! it for the right-hand side (seven per attempt), clones the tableau and
//! allocates stage, state, right-hand-side and report vectors per attempt.
//! This driver solves the raw U form of the same method
//! (`crate::raw_stage_target`, node P0-MF-TARGET) with `M = I`:
//!
//! ```text
//! (I - h gamma J) U_i = h gamma f(t + c_i h, y + sum_j A_ij U_j)
//!                       + gamma sum_j C_ij U_j + h^2 gamma gamma_i f_t,
//! y_new = y + sum_j b_code_j U_j,   embedded error U_(s-1),
//! ```
//!
//! so `J` is applied only inside the Krylov solver, through the same counted
//! JVP shifted operator. It keeps:
//!
//! * strict matrix-free semantics: a user JVP is required, and a direct
//!   solve, a direct or Jacobi preconditioner (both need an explicit matrix)
//!   or a mass matrix is refused;
//! * the configured GMRES, LGMRES or GCRO-DR with their true-residual check;
//!   the K path's relative tolerance applies to the U right-hand side's own
//!   norm, the absolute one is mapped to `|gamma| atol`
//!   ([`crate::raw_absolute_residual_budget`]);
//! * the Krylov recycle state, snapshotted before an attempt and restored on
//!   rejection, as in the sequential path;
//! * the sequential driver's step-size controller, represented-clock and
//!   output rules, unchanged.
//!
//! One workspace (stage vectors, Krylov workspaces, coefficient rows) serves
//! the whole integration; `f(t, y)`, `f_t` and the JVP operator are reused
//! after a rejected attempt from the same state. There is no inner forcing:
//! the protected driver's WRMS forcing and refinement passes are not ported.

use std::sync::Arc;

use rodas5p_core::{
    CoreError, CoreResult, IdentityPreconditioner, InitialGuess, LinearMethod, LinearOperator,
    LinearSolverConfig, PreconditionerKind, ShiftedOperator, WorkCounters, rodas5p_coefficients,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrWorkspace, GmresConfig, GmresWorkspace, LgmresConfig, LgmresWorkspace,
    solve_gcrodr_with_workspace, solve_gmres_with_workspace, solve_lgmres_with_workspace,
};

use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, KrylovState,
    ObservedIntegrationResult, OdeProblem, OutputSchedule, output::OutputCollector,
    problem::MatrixFreeCallbackIdentity, raw_absolute_residual_budget,
    rodas_next_step_after_attempt,
};

/// Identifier of this driver in research records.
pub const RODAS5P_MF_FAST_DRIVER_ID: &str = "rodas5p-mf-fast-transformed-v1";

#[derive(Clone, Debug)]
pub struct Rodas5pMfFastResult {
    pub observed: ObservedIntegrationResult,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    /// Attempts that reused `f(t, y)`, `f_t` and the JVP operator of a
    /// rejected attempt from the same state.
    pub state_reuses: usize,
    pub driver: &'static str,
}

/// The stage arithmetic and Krylov workspaces of one integration.
pub struct Rodas5pMfFastWorkspace {
    n: usize,
    s: usize,
    gamma: f64,
    c: Vec<f64>,
    gamma_rows: Vec<f64>,
    a_nonzero: Vec<Vec<(usize, f64)>>,
    c_nonzero: Vec<Vec<(usize, f64)>>,
    b_nonzero: Vec<(usize, f64)>,
    config: LinearSolverConfig,
    preconditioner: IdentityPreconditioner,
    gmres: GmresWorkspace,
    lgmres: LgmresWorkspace,
    gcrodr: GcrodrWorkspace,
    jvp: Option<Arc<dyn LinearOperator>>,
    cached_callbacks: Option<MatrixFreeCallbackIdentity>,
    cached_t_bits: u64,
    cached_y: Vec<f64>,
    cached_epoch: Option<u64>,
    f0: Vec<f64>,
    ft: Vec<f64>,
    u: Vec<f64>,
    stage_state: Vec<f64>,
    stage_rhs: Vec<f64>,
    y_new: Vec<f64>,
}

fn validate_strict(problem: &OdeProblem, config: &LinearSolverConfig) -> CoreResult<()> {
    config.validate().map_err(CoreError::InvalidInput)?;
    if config.method == LinearMethod::Direct
        || matches!(
            config.preconditioner,
            PreconditionerKind::Direct | PreconditionerKind::Jacobi
        )
    {
        return Err(CoreError::InvalidInput(
            "strict matrix-free RODAS5P (U form) forbids direct solves and explicit-matrix preconditioners".into(),
        ));
    }
    if !problem.supports_matrix_free_jvp() {
        return Err(CoreError::InvalidInput(
            "strict matrix-free integration requires a user-supplied JVP".into(),
        ));
    }
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "the matrix-free U-form driver supports the identity mass matrix only".into(),
        ));
    }
    Ok(())
}

impl Rodas5pMfFastWorkspace {
    pub fn new(problem: &OdeProblem, config: &LinearSolverConfig) -> CoreResult<Self> {
        validate_strict(problem, config)?;
        let n = problem.dimension;
        let coeffs = rodas5p_coefficients()?;
        let s = coeffs.stages();
        for i in 0..s {
            for j in i..s {
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
            n,
            s,
            gamma: coeffs.gamma,
            c: coeffs.c.clone(),
            gamma_rows: coeffs.gamma_rows.clone(),
            a_nonzero: (0..s).map(|i| nonzero_row(&coeffs.a, i)).collect(),
            c_nonzero: (0..s).map(|i| nonzero_row(&coeffs.c_matrix, i)).collect(),
            b_nonzero: coeffs
                .b_code
                .iter()
                .enumerate()
                .filter(|(_, b)| **b != 0.0)
                .map(|(j, b)| (j, *b))
                .collect(),
            config: config.clone(),
            preconditioner: IdentityPreconditioner::new(n),
            gmres: GmresWorkspace::default(),
            lgmres: LgmresWorkspace::default(),
            gcrodr: GcrodrWorkspace::default(),
            jvp: None,
            cached_callbacks: None,
            cached_t_bits: 0,
            cached_y: vec![0.0; n],
            cached_epoch: None,
            f0: vec![0.0; n],
            ft: vec![0.0; n],
            u: vec![0.0; s * n],
            stage_state: vec![0.0; n],
            stage_rhs: vec![0.0; n],
            y_new: vec![0.0; n],
        })
    }

    /// The new state of the last attempt.
    pub fn y_new(&self) -> &[f64] {
        &self.y_new
    }

    /// Stage `i` of the last attempt.
    pub fn stage(&self, i: usize) -> &[f64] {
        &self.u[i * self.n..(i + 1) * self.n]
    }

    /// One attempt from `(t, y)` with step `h`: the new state in
    /// [`Self::y_new`] and the WRMS norm of the embedded error `U_(s-1)` in
    /// the sequential path's scale `atol + rtol max(|y|, |y_new|)`. `fresh`
    /// evaluates `f(t, y)`, `f_t` and the JVP operator at this state;
    /// otherwise reuse is allowed only for the exact same time/state bits and
    /// retained callback identities. Changing `h` alone does not invalidate
    /// the frozen state. Interior callback-data changes require `fresh = true`,
    /// unless the problem carries a model epoch
    /// ([`OdeProblem::with_model_epoch`]): the epoch is read once per attempt
    /// and any change rebuilds `f(t, y)`, `f_t` and the operator (whose new
    /// token makes a carried recycle state refresh its images).
    /// A failed refresh invalidates all frozen data before any retry.
    #[allow(clippy::too_many_arguments)]
    pub fn attempt(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        fresh: bool,
        recycle: Option<&mut KrylovState>,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        let (n, s, gamma) = (self.n, self.s, self.gamma);
        validate_strict(problem, &self.config)?;
        if problem.dimension != n {
            return Err(CoreError::Dimension(
                "matrix-free workspace/problem dimensions differ".into(),
            ));
        }
        if !(t.is_finite() && atol.is_finite() && atol >= 0.0 && rtol.is_finite() && rtol >= 0.0) {
            return Err(CoreError::InvalidInput(
                "time must be finite and output tolerances finite and nonnegative".into(),
            ));
        }
        if !(h.is_finite() && h != 0.0) {
            return Err(CoreError::InvalidInput(
                "step size must be finite and nonzero".into(),
            ));
        }
        if y.len() != n || !y.iter().all(|value| value.is_finite()) {
            return Err(CoreError::InvalidInput("invalid initial state".into()));
        }
        let epoch = problem.model_epoch();
        let same_state = self.cached_t_bits == t.to_bits()
            && self.cached_epoch == epoch
            && self
                .cached_y
                .iter()
                .zip(y)
                .all(|(a, b)| a.to_bits() == b.to_bits())
            && self
                .cached_callbacks
                .as_ref()
                .is_some_and(|id| id.matches(problem));
        if fresh || self.jvp.is_none() || !same_state {
            // Invalidate before fallible callbacks: a partially written RHS or
            // failed f_t must never be paired with the old state's operator.
            self.jvp = None;
            self.cached_callbacks = None;
            problem.eval_rhs_into(t, y, &mut self.f0, counters)?;
            if problem.autonomous {
                self.ft.fill(0.0);
            } else {
                let ft = problem.eval_partial_t(t, y, counters)?;
                self.ft.copy_from_slice(&ft);
            }
            let jvp = problem.linearize_matrix_free(t, y)?;
            self.cached_y.copy_from_slice(y);
            self.cached_t_bits = t.to_bits();
            self.cached_epoch = epoch;
            self.cached_callbacks = Some(problem.matrix_free_callback_identity());
            self.jvp = Some(jvp);
        }
        let jvp = self.jvp.clone().expect("linearized above");
        let shifted = ShiftedOperator::new_counted_jvp(None, jvp, h, gamma)?;
        debug_assert!(shifted.explicit().is_none());
        let linear_atol = if self.config.atol == 0.0 {
            0.0
        } else {
            raw_absolute_residual_budget(self.config.atol, gamma)?
        };
        let mut owned = KrylovState::for_method(self.config.method);
        let state = match recycle {
            Some(state) => Some(state),
            None => owned.as_mut(),
        };
        let mut state = state;
        let hg = h * gamma;
        for i in 0..s {
            if i == 0 {
                self.stage_rhs.copy_from_slice(&self.f0);
            } else {
                self.stage_state.copy_from_slice(y);
                for &(j, aij) in &self.a_nonzero[i] {
                    let uj = &self.u[j * n..(j + 1) * n];
                    for (x, v) in self.stage_state.iter_mut().zip(uj) {
                        *x += aij * v;
                    }
                }
                problem.eval_rhs_into(
                    t + self.c[i] * h,
                    &self.stage_state,
                    &mut self.stage_rhs,
                    counters,
                )?;
            }
            // rhs = h gamma f_i + gamma sum_j C_ij U_j + h^2 gamma gamma_i f_t
            for x in self.stage_rhs.iter_mut() {
                *x *= hg;
            }
            for &(j, cij) in &self.c_nonzero[i] {
                let weight = gamma * cij;
                let uj = &self.u[j * n..(j + 1) * n];
                for (x, v) in self.stage_rhs.iter_mut().zip(uj) {
                    *x += weight * v;
                }
            }
            if !problem.autonomous {
                let g = hg * h * self.gamma_rows[i];
                for (x, v) in self.stage_rhs.iter_mut().zip(&self.ft) {
                    *x += g * v;
                }
            }
            let x0 = (i > 0 && self.config.x0_strategy == InitialGuess::Previous)
                .then(|| &self.u[(i - 1) * n..i * n]);
            let report = match self.config.method {
                LinearMethod::Gmres => solve_gmres_with_workspace(
                    &shifted,
                    &self.preconditioner,
                    &self.stage_rhs,
                    x0,
                    &GmresConfig {
                        restart: self.config.restart,
                        max_arnoldi: self.config.maxiter.max(self.config.restart),
                        rtol: self.config.rtol,
                        atol: linear_atol,
                    },
                    &mut self.gmres,
                    counters,
                )?,
                LinearMethod::Lgmres => {
                    let Some(KrylovState::Lgmres(st)) = state.as_deref_mut() else {
                        return Err(CoreError::InvalidInput(
                            "LGMRES needs an LGMRES recycle state".into(),
                        ));
                    };
                    solve_lgmres_with_workspace(
                        &shifted,
                        &self.preconditioner,
                        &self.stage_rhs,
                        x0,
                        &LgmresConfig {
                            inner_m: self.config.inner_m,
                            max_outer: self.config.maxiter,
                            outer_k: self.config.outer_k,
                            rtol: self.config.rtol,
                            atol: linear_atol,
                        },
                        st,
                        &mut self.lgmres,
                        counters,
                    )?
                }
                LinearMethod::Gcrodr => {
                    let Some(KrylovState::Gcrodr(st)) = state.as_deref_mut() else {
                        return Err(CoreError::InvalidInput(
                            "GCRO-DR needs a GCRO-DR recycle state".into(),
                        ));
                    };
                    solve_gcrodr_with_workspace(
                        &shifted,
                        &self.preconditioner,
                        &self.stage_rhs,
                        x0,
                        &GcrodrConfig {
                            restart: self.config.restart,
                            max_arnoldi: self.config.maxiter.max(self.config.restart),
                            recycle_dim: self.config.recycle_dim,
                            rank_tol: self.config.recycle_rank_tol,
                            rtol: self.config.rtol,
                            atol: linear_atol,
                        },
                        st,
                        &mut self.gcrodr,
                        counters,
                    )?
                }
                LinearMethod::Direct => unreachable!("refused in validate_strict"),
            };
            if !report.x.iter().all(|value| value.is_finite()) {
                return Err(CoreError::NonFinite(
                    "RODAS5P U-form stage solve produced NaN/Inf".into(),
                ));
            }
            self.u[i * n..(i + 1) * n].copy_from_slice(&report.x);
        }
        self.y_new.copy_from_slice(y);
        for &(j, bj) in &self.b_nonzero {
            let uj = &self.u[j * n..(j + 1) * n];
            for (x, v) in self.y_new.iter_mut().zip(uj) {
                *x += bj * v;
            }
        }
        if !self.y_new.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "RODAS5P U-form step produced NaN/Inf".into(),
            ));
        }
        let error = &self.u[(s - 1) * n..s * n];
        let mut sum = 0.0;
        for ((e, a), b) in error.iter().zip(y).zip(&self.y_new) {
            let scale = atol + rtol * a.abs().max(b.abs());
            if !(scale.is_finite() && scale > 0.0) {
                return Err(CoreError::InvalidInput(
                    "every output error scale must be finite and positive".into(),
                ));
            }
            let z = e / scale;
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

fn failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        _ => None,
    }
}

/// Adaptive JVP-only RODAS5P on the raw U stages with the sequential
/// driver's controller and output rules. Requires a user JVP and the
/// identity mass matrix; any explicit-matrix request is refused.
pub fn integrate_rodas5p_mf_fast_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Rodas5pMfFastResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if tf < t || y0.len() != problem.dimension {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P matrix-free U-form integration input".into(),
        ));
    }
    let mut work = Rodas5pMfFastWorkspace::new(problem, linear_config)?;
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut recycle = KrylovState::for_method(linear_config.method);
    let mut collector =
        OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
    let (mut attempts, mut accepted_steps, mut rejected_steps, mut reuses) = (0, 0, 0, 0);
    let mut internal_steps = 0_usize;
    // Whether f(t, y), f_t and the JVP in the workspace belong to (t, y).
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
        let snapshot = recycle.clone();
        let outcome = work.attempt(
            problem,
            t,
            &y,
            trial_h,
            !fresh_state,
            recycle.as_mut(),
            adaptive.atol,
            adaptive.rtol,
            &mut counters,
        );
        fresh_state = work.jvp.is_some();
        let (error, failure) = match outcome {
            Ok(error) if error <= 1.0 => (error, None),
            Ok(error) => (error, Some(AdaptiveFailureKind::LocalError)),
            Err(error) => match failure_kind(&error) {
                Some(kind) => (f64::INFINITY, Some(kind)),
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
                // A rejected attempt leaves no trace in the recycle state.
                recycle = snapshot;
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
    Ok(Rodas5pMfFastResult {
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
        state_reuses: reuses,
        driver: RODAS5P_MF_FAST_DRIVER_ID,
    })
}
