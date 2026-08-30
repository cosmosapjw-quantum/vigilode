//! Strictly matrix-free common-W Audit-2 research backend.
//!
//! This child module is compiled only with the parent `audit2-research` feature.
//! It reuses one GCRO-DR state across the eight sequential common-W solves at
//! one frozen projected target. It is never selected by production dispatch.

use serde::{Deserialize, Serialize};

use rodas5p_core::{CoreError, CoreResult, IdentityPreconditioner, WorkCounters, safe_l2};
use rodas5p_krylov::{GcrodrConfig, GcrodrState, solve_gcrodr};

use crate::StepContext;

use super::{
    Audit2CoefficientProjection, Audit2CorrectionWork, Audit2FailurePhase, Audit2SharedFailure,
    PreparedTarget, apply_jvp_attempt, bump, common_linear_diagnostic, flatten,
    nonlinear_residual_after, prepare_target, rows_l2,
};

/// Explicit numerical controls for the strictly matrix-free research solve.
///
/// No defaults are inferred by the public entry point: callers must supply
/// this object. `max_completed_rows` is a research-only bounded-work stop used
/// to verify partial-result preservation; `None` means no artificial stop.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2MatrixFreeLinearConfig {
    pub restart: usize,
    pub max_arnoldi: usize,
    pub recycle_dim: usize,
    pub rank_tol: f64,
    pub rtol: f64,
    pub atol: f64,
    pub max_completed_rows: Option<usize>,
}

impl Audit2MatrixFreeLinearConfig {
    fn validate(&self) -> CoreResult<()> {
        if self.restart == 0 {
            return Err(CoreError::InvalidInput(
                "Audit-2 matrix-free GCRO-DR restart must be positive".into(),
            ));
        }
        if self.max_arnoldi < self.restart {
            return Err(CoreError::InvalidInput(
                "Audit-2 matrix-free GCRO-DR max_arnoldi must be at least restart".into(),
            ));
        }
        if self.recycle_dim == 0 || self.recycle_dim >= self.restart {
            return Err(CoreError::InvalidInput(
                "Audit-2 matrix-free GCRO-DR recycle_dim must lie in 1..restart".into(),
            ));
        }
        if !self.rank_tol.is_finite() || self.rank_tol <= 0.0 {
            return Err(CoreError::InvalidInput(
                "Audit-2 matrix-free GCRO-DR rank_tol must be finite and positive".into(),
            ));
        }
        if !self.rtol.is_finite() || self.rtol < 0.0 {
            return Err(CoreError::InvalidInput(
                "Audit-2 matrix-free GCRO-DR rtol must be finite and nonnegative".into(),
            ));
        }
        if !self.atol.is_finite() || self.atol < 0.0 || (self.rtol == 0.0 && self.atol == 0.0)
        {
            return Err(CoreError::InvalidInput(
                "Audit-2 matrix-free GCRO-DR tolerances must be finite, nonnegative, and not both zero"
                    .into(),
            ));
        }
        Ok(())
    }

    fn gcrodr(self) -> GcrodrConfig {
        GcrodrConfig {
            restart: self.restart,
            max_arnoldi: self.max_arnoldi,
            recycle_dim: self.recycle_dim,
            rank_tol: self.rank_tol,
            rtol: self.rtol,
            atol: self.atol,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2MatrixFreeSolveReport {
    pub stage_index: usize,
    pub converged: bool,
    pub iterations: usize,
    pub matvecs: u64,
    pub preconditioner_apps: u64,
    pub residual_norm: f64,
    pub relative_residual: f64,
    pub recycle_reused: bool,
    pub recycle_same_operator_uses_delta: u64,
    pub recycle_cross_operator_refreshes_delta: u64,
    pub recycle_updates_delta: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2MatrixFreeCommonWSuccess {
    pub projection: Audit2CoefficientProjection,
    pub preparation_counters: WorkCounters,
    pub correction: Vec<Vec<f64>>,
    pub correction_l2: f64,
    pub initial_residual_l2: f64,
    pub linear_residual_l2: f64,
    pub nonlinear_residual_after_l2: f64,
    pub solve_reports: Vec<Audit2MatrixFreeSolveReport>,
    pub work: Audit2CorrectionWork,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2MatrixFreeCommonWFailure {
    pub projection: Option<Audit2CoefficientProjection>,
    pub phase: Audit2FailurePhase,
    pub message: String,
    pub preparation_counters: WorkCounters,
    pub partial_correction: Vec<Vec<f64>>,
    pub partial_solve_reports: Vec<Audit2MatrixFreeSolveReport>,
    pub initial_residual_l2: Option<f64>,
    pub work: Audit2CorrectionWork,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Audit2MatrixFreeCommonWOutcome {
    Completed(Box<Audit2MatrixFreeCommonWSuccess>),
    Failed(Box<Audit2MatrixFreeCommonWFailure>),
}

impl Audit2MatrixFreeCommonWOutcome {
    pub fn completed(&self) -> Option<&Audit2MatrixFreeCommonWSuccess> {
        match self {
            Self::Completed(value) => Some(value),
            Self::Failed(_) => None,
        }
    }

    pub fn failed(&self) -> Option<&Audit2MatrixFreeCommonWFailure> {
        match self {
            Self::Completed(_) => None,
            Self::Failed(value) => Some(value),
        }
    }
}

fn failure_without_preparation(
    phase: Audit2FailurePhase,
    error: impl ToString,
) -> Audit2MatrixFreeCommonWOutcome {
    Audit2MatrixFreeCommonWOutcome::Failed(Box::new(Audit2MatrixFreeCommonWFailure {
        projection: None,
        phase,
        message: error.to_string(),
        preparation_counters: WorkCounters::default(),
        partial_correction: Vec::new(),
        partial_solve_reports: Vec::new(),
        initial_residual_l2: None,
        work: Audit2CorrectionWork::default(),
    }))
}

fn failed_preparation(shared: Box<Audit2SharedFailure>) -> Audit2MatrixFreeCommonWOutcome {
    let shared = *shared;
    Audit2MatrixFreeCommonWOutcome::Failed(Box::new(Audit2MatrixFreeCommonWFailure {
        projection: None,
        phase: shared.phase,
        message: shared.message,
        preparation_counters: shared.preparation_counters,
        partial_correction: Vec::new(),
        partial_solve_reports: Vec::new(),
        initial_residual_l2: None,
        work: Audit2CorrectionWork::default(),
    }))
}

fn failure(
    prepared: &PreparedTarget<'_>,
    phase: Audit2FailurePhase,
    error: impl ToString,
    partial_correction: Vec<Vec<f64>>,
    partial_solve_reports: Vec<Audit2MatrixFreeSolveReport>,
    work: Audit2CorrectionWork,
) -> Audit2MatrixFreeCommonWOutcome {
    Audit2MatrixFreeCommonWOutcome::Failed(Box::new(Audit2MatrixFreeCommonWFailure {
        projection: Some(prepared.projection.clone()),
        phase,
        message: error.to_string(),
        preparation_counters: prepared.preparation_counters,
        partial_correction,
        partial_solve_reports,
        initial_residual_l2: Some(rows_l2(&prepared.residual)),
        work,
    }))
}

fn matrix_free_common_w_correction(
    prepared: &PreparedTarget<'_>,
    trial_stages: &[Vec<f64>],
    config: Audit2MatrixFreeLinearConfig,
) -> Audit2MatrixFreeCommonWOutcome {
    let context = &prepared.context;
    let mut work = Audit2CorrectionWork::default();
    let mut correction = Vec::<Vec<f64>>::with_capacity(context.coeffs.stages());
    let mut solve_reports = Vec::<Audit2MatrixFreeSolveReport>::with_capacity(context.coeffs.stages());

    bump(&mut work.common_w_setup_attempts);
    if context.shifted.explicit().is_some() {
        return failure(
            prepared,
            Audit2FailurePhase::CommonWSetup,
            "Audit-2 matrix-free common-W backend forbids an explicit shifted matrix",
            correction,
            solve_reports,
            work,
        );
    }
    if !context.problem.has_jvp() {
        return failure(
            prepared,
            Audit2FailurePhase::JvpAccess,
            "Audit-2 matrix-free common-W backend requires an analytic JVP",
            correction,
            solve_reports,
            work,
        );
    }
    let n = context.problem.dimension;
    let s = context.coeffs.stages();
    let preconditioner = IdentityPreconditioner::new(n);
    let gcrodr = config.gcrodr();
    let mut recycle = GcrodrState::default();
    bump(&mut work.common_w_setup_completed);

    let mut p = vec![0.0; n];
    let mut q = vec![0.0; n];
    let mut image = vec![0.0; n];
    for i in 0..s {
        if config.max_completed_rows == Some(i) {
            return failure(
                prepared,
                Audit2FailurePhase::Solve,
                format!("Audit-2 matrix-free bounded row limit reached before stage {i}"),
                correction,
                solve_reports,
                work,
            );
        }

        p.fill(0.0);
        q.fill(0.0);
        for (j, z) in correction.iter().enumerate() {
            for k in 0..n {
                p[k] += context.coeffs.alpha[(i, j)] * z[k];
                q[k] += context.coeffs.gamma_matrix[(i, j)] * z[k];
            }
        }
        let mut corrected = prepared.residual[i].clone();
        if i > 0 {
            let stage_operator = match context.problem.linearize_matrix_free(
                context.t + context.coeffs.c[i] * context.h,
                &prepared.snapshot.states[i],
            ) {
                Ok(operator) => operator,
                Err(error) => {
                    return failure(
                        prepared,
                        Audit2FailurePhase::JvpAccess,
                        error,
                        correction,
                        solve_reports,
                        work,
                    );
                }
            };
            if let Err(error) =
                apply_jvp_attempt(stage_operator.as_ref(), &p, &mut image, &mut work, false)
            {
                return failure(
                    prepared,
                    Audit2FailurePhase::CorrectionJvp,
                    error,
                    correction,
                    solve_reports,
                    work,
                );
            }
            for k in 0..n {
                corrected[k] += context.h * image[k];
            }
            if let Err(error) = apply_jvp_attempt(
                context.jacobian.as_ref(),
                &q,
                &mut image,
                &mut work,
                false,
            ) {
                return failure(
                    prepared,
                    Audit2FailurePhase::CorrectionJvp,
                    error,
                    correction,
                    solve_reports,
                    work,
                );
            }
            for k in 0..n {
                corrected[k] += context.h * image[k];
            }
        }
        if corrected.iter().any(|value| !value.is_finite()) {
            return failure(
                prepared,
                Audit2FailurePhase::CorrectionRhs,
                "Audit-2 matrix-free correction RHS contains NaN/Inf",
                correction,
                solve_reports,
                work,
            );
        }

        bump(&mut work.solve_attempts);
        let before = work.counters;
        let report = match solve_gcrodr(
            &context.shifted,
            &preconditioner,
            &corrected,
            None,
            &gcrodr,
            &mut recycle,
            &mut work.counters,
        ) {
            Ok(report) => report,
            Err(error) => {
                return failure(
                    prepared,
                    Audit2FailurePhase::Solve,
                    error,
                    correction,
                    solve_reports,
                    work,
                );
            }
        };
        let delta = work.counters.delta(before);
        let summary = Audit2MatrixFreeSolveReport {
            stage_index: i,
            converged: report.converged,
            iterations: report.iterations,
            matvecs: report.matvecs,
            preconditioner_apps: report.preconditioner_apps,
            residual_norm: report.residual_norm,
            relative_residual: report.relative_residual,
            recycle_reused: delta.recycle_same_operator_uses > 0,
            recycle_same_operator_uses_delta: delta.recycle_same_operator_uses,
            recycle_cross_operator_refreshes_delta: delta.recycle_cross_operator_refreshes,
            recycle_updates_delta: delta.recycle_updates,
        };
        solve_reports.push(summary);
        if !report.converged {
            return failure(
                prepared,
                Audit2FailurePhase::Solve,
                format!("Audit-2 matrix-free GCRO-DR did not converge at stage {i}"),
                correction,
                solve_reports,
                work,
            );
        }
        if report.x.len() != n || report.x.iter().any(|value| !value.is_finite()) {
            return failure(
                prepared,
                Audit2FailurePhase::Solve,
                "Audit-2 matrix-free GCRO-DR returned malformed or nonfinite correction",
                correction,
                solve_reports,
                work,
            );
        }
        bump(&mut work.solve_completed);
        correction.push(report.x);
    }

    let diagnostic = match common_linear_diagnostic(prepared, &correction, &mut work) {
        Ok(value) => value,
        Err(error) => {
            return failure(
                prepared,
                Audit2FailurePhase::LinearDiagnostic,
                error,
                correction,
                solve_reports,
                work,
            );
        }
    };
    let linear_difference: Vec<Vec<f64>> = diagnostic
        .iter()
        .zip(&prepared.residual)
        .map(|(left, right)| left.iter().zip(right).map(|(a, b)| a - b).collect())
        .collect();
    let nonlinear_residual_after_l2 =
        match nonlinear_residual_after(prepared, trial_stages, &correction, &mut work) {
            Ok(value) => value,
            Err(error) => {
                return failure(
                    prepared,
                    Audit2FailurePhase::NonlinearResidualAfter,
                    error,
                    correction,
                    solve_reports,
                    work,
                );
            }
        };
    let correction_flat = flatten(&correction);
    Audit2MatrixFreeCommonWOutcome::Completed(Box::new(Audit2MatrixFreeCommonWSuccess {
        projection: prepared.projection.clone(),
        preparation_counters: prepared.preparation_counters,
        correction,
        correction_l2: safe_l2(&correction_flat),
        initial_residual_l2: rows_l2(&prepared.residual),
        linear_residual_l2: rows_l2(&linear_difference),
        nonlinear_residual_after_l2,
        solve_reports,
        work,
    }))
}

/// Run the explicitly configured strict matrix-free common-W research arm.
///
/// The input context must have been built with a matrix-free JVP path. The
/// method shares one frozen shifted operator and one GCRO-DR recycle state
/// across all stage-row solves. It performs no acceptance or production action.
pub fn run_audit2_matrix_free_common_w_reuse(
    context: &StepContext<'_>,
    trial_stages: &[Vec<f64>],
    config: Option<Audit2MatrixFreeLinearConfig>,
) -> Audit2MatrixFreeCommonWOutcome {
    let config = match config {
        Some(config) => config,
        None => {
            return failure_without_preparation(
                Audit2FailurePhase::CommonWSetup,
                "Audit-2 matrix-free common-W backend requires explicit linear configuration",
            );
        }
    };
    if let Err(error) = config.validate() {
        return failure_without_preparation(Audit2FailurePhase::InputValidation, error);
    }
    if context.shifted.explicit().is_some() {
        return failure_without_preparation(
            Audit2FailurePhase::CommonWSetup,
            "Audit-2 matrix-free common-W backend requires a strict matrix-free StepContext",
        );
    }
    if !context.problem.has_jvp() {
        return failure_without_preparation(
            Audit2FailurePhase::JvpAccess,
            "Audit-2 matrix-free common-W backend requires an analytic JVP",
        );
    }
    let prepared = match prepare_target(context, trial_stages) {
        Ok(prepared) => prepared,
        Err(shared) => return failed_preparation(shared),
    };
    matrix_free_common_w_correction(&prepared, trial_stages, config)
}
