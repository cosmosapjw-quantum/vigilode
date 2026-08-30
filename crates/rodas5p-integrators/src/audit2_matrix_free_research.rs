//! Reusable matrix-free common-W research session for Audit-2.
//!
//! The session reuses one shifted operator, one identity preconditioner, and one
//! GMRES workspace across several right-hand sides and across repeated batches.
//! It does not reuse Krylov bases, does not dispatch from an integrator, and
//! does not claim timing, speedup, nonlinear convergence, or output accuracy.

use std::fmt;

use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, DenseMatrix, IdentityPreconditioner, LinearOperator,
    LinearSolveReport, ShiftedOperator, WorkCounters, apply_counted, safe_l2,
};
use rodas5p_krylov::{GmresConfig, GmresWorkspace, solve_gmres_with_workspace};
use serde::{Deserialize, Serialize};

use crate::audit2_research::{AUDIT2_STRUCTURE_PROJECTION_TOLERANCE, Audit2CoefficientProjection};
use crate::{StepContext, StructuredBlockSystem};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2MatrixFreeCommonWConfig {
    pub restart: usize,
    pub max_arnoldi: usize,
    pub rtol: f64,
    pub atol: f64,
}

impl Default for Audit2MatrixFreeCommonWConfig {
    fn default() -> Self {
        Self {
            restart: 24,
            max_arnoldi: 192,
            rtol: 1.0e-11,
            atol: 1.0e-13,
        }
    }
}

impl Audit2MatrixFreeCommonWConfig {
    fn gmres(self) -> CoreResult<GmresConfig> {
        let config = GmresConfig {
            restart: self.restart,
            max_arnoldi: self.max_arnoldi,
            rtol: self.rtol,
            atol: self.atol,
        };
        config.validate()?;
        if !self.rtol.is_finite() || !self.atol.is_finite() {
            return Err(CoreError::InvalidInput(
                "Audit-2 matrix-free GMRES tolerances must be finite".into(),
            ));
        }
        Ok(config)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audit2MatrixFreeFailurePhase {
    InputValidation,
    Solve,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Audit2MatrixFreeSessionSnapshot {
    pub setup_attempts: u64,
    pub setup_completed: u64,
    pub identity_preconditioner_setups: u64,
    pub workspace_initializations: u64,
    pub batch_attempts: u64,
    pub batch_completed: u64,
    pub solve_attempts: u64,
    pub solve_completed: u64,
    pub operator_token: u64,
    pub dimension: usize,
    pub h_gamma_bits: u64,
    pub workspace_capacity_after_first_solve: Option<usize>,
    pub workspace_capacity_current: usize,
    pub workspace_capacity_growth_after_first: usize,
    pub counters: WorkCounters,
}

#[derive(Debug)]
pub struct Audit2MatrixFreeSessionSetupFailure {
    pub error: CoreError,
    pub session: Audit2MatrixFreeSessionSnapshot,
}

impl fmt::Display for Audit2MatrixFreeSessionSetupFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for Audit2MatrixFreeSessionSetupFailure {}

impl From<Audit2MatrixFreeSessionSetupFailure> for CoreError {
    fn from(failure: Audit2MatrixFreeSessionSetupFailure) -> Self {
        failure.error
    }
}

impl From<Box<Audit2MatrixFreeSessionSetupFailure>> for CoreError {
    fn from(failure: Box<Audit2MatrixFreeSessionSetupFailure>) -> Self {
        failure.error
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit2MatrixFreeBatchSuccess {
    pub solutions: Vec<Vec<f64>>,
    pub solve_reports: Vec<LinearSolveReport>,
    pub session: Audit2MatrixFreeSessionSnapshot,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit2MatrixFreeBatchFailure {
    pub phase: Audit2MatrixFreeFailurePhase,
    pub failed_row: Option<usize>,
    pub message: String,
    pub partial_solutions: Vec<Vec<f64>>,
    pub partial_reports: Vec<LinearSolveReport>,
    pub session: Audit2MatrixFreeSessionSnapshot,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Audit2MatrixFreeBatchOutcome {
    Completed(Box<Audit2MatrixFreeBatchSuccess>),
    Failed(Box<Audit2MatrixFreeBatchFailure>),
}

/// One reusable research session around a single exact operator instance.
///
/// `new` fails closed when the operator exposes an explicit matrix. This keeps
/// the research observation about matrix-free execution separate from the
/// existing explicit-LU common-W reference path.
pub struct Audit2MatrixFreeCommonWSession<'a> {
    operator: &'a ShiftedOperator,
    preconditioner: IdentityPreconditioner,
    config: GmresConfig,
    workspace: GmresWorkspace,
    snapshot: Audit2MatrixFreeSessionSnapshot,
}

impl fmt::Debug for Audit2MatrixFreeCommonWSession<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Audit2MatrixFreeCommonWSession")
            .field("config", &self.config)
            .field("snapshot", &self.snapshot)
            .finish_non_exhaustive()
    }
}

impl<'a> Audit2MatrixFreeCommonWSession<'a> {
    pub fn new(
        context: &'a StepContext<'_>,
        config: Audit2MatrixFreeCommonWConfig,
    ) -> Result<Self, Box<Audit2MatrixFreeSessionSetupFailure>> {
        let operator = &context.shifted;
        let mut snapshot = Audit2MatrixFreeSessionSnapshot {
            setup_attempts: 1,
            operator_token: operator.token(),
            dimension: operator.dimension(),
            h_gamma_bits: operator.h_gamma().to_bits(),
            ..Audit2MatrixFreeSessionSnapshot::default()
        };
        if snapshot.dimension == 0 {
            return Err(Box::new(Audit2MatrixFreeSessionSetupFailure {
                error: CoreError::InvalidInput(
                    "Audit-2 matrix-free common-W dimension must be positive".into(),
                ),
                session: snapshot,
            }));
        }
        if operator.explicit().is_some() {
            return Err(Box::new(Audit2MatrixFreeSessionSetupFailure {
                error: CoreError::InvalidInput(
                    "Audit-2 matrix-free common-W session rejects explicit W".into(),
                ),
                session: snapshot,
            }));
        }
        let config = match config.gmres() {
            Ok(value) => value,
            Err(error) => {
                return Err(Box::new(Audit2MatrixFreeSessionSetupFailure {
                    error,
                    session: snapshot,
                }));
            }
        };
        let preconditioner = IdentityPreconditioner::new(snapshot.dimension);
        snapshot.identity_preconditioner_setups = 1;
        let workspace = GmresWorkspace::default();
        snapshot.workspace_initializations = 1;
        snapshot.setup_completed = 1;
        Ok(Self {
            operator,
            preconditioner,
            config,
            workspace,
            snapshot,
        })
    }

    pub fn snapshot(&self) -> Audit2MatrixFreeSessionSnapshot {
        self.snapshot.clone()
    }

    fn refresh_workspace_capacity(&mut self) {
        let capacity = self.workspace.capacity_f64();
        if self.snapshot.workspace_capacity_after_first_solve.is_none()
            && self.snapshot.solve_completed > 0
        {
            self.snapshot.workspace_capacity_after_first_solve = Some(capacity);
        }
        self.snapshot.workspace_capacity_current = capacity;
        self.snapshot.workspace_capacity_growth_after_first = self
            .snapshot
            .workspace_capacity_after_first_solve
            .map_or(0, |first| capacity.saturating_sub(first));
    }

    fn failure(
        &self,
        phase: Audit2MatrixFreeFailurePhase,
        failed_row: Option<usize>,
        error: impl ToString,
        partial_solutions: Vec<Vec<f64>>,
        partial_reports: Vec<LinearSolveReport>,
    ) -> Audit2MatrixFreeBatchOutcome {
        Audit2MatrixFreeBatchOutcome::Failed(Box::new(Audit2MatrixFreeBatchFailure {
            phase,
            failed_row,
            message: error.to_string(),
            partial_solutions,
            partial_reports,
            session: self.snapshot(),
        }))
    }

    fn begin_batch(&mut self) {
        self.snapshot.batch_attempts = self.snapshot.batch_attempts.saturating_add(1);
        self.snapshot.counters.block_linear_solves =
            self.snapshot.counters.block_linear_solves.saturating_add(1);
    }

    fn complete_batch(&mut self) {
        self.snapshot.batch_completed = self.snapshot.batch_completed.saturating_add(1);
    }

    fn validate_rhs(&self, rhs: &[f64]) -> CoreResult<()> {
        if rhs.len() != self.snapshot.dimension {
            return Err(CoreError::Dimension(
                "Audit-2 matrix-free common-W RHS dimension mismatch".into(),
            ));
        }
        if rhs.iter().any(|value| !value.is_finite()) {
            return Err(CoreError::NonFinite(
                "Audit-2 matrix-free common-W RHS contains NaN/Inf".into(),
            ));
        }
        Ok(())
    }

    fn solve_validated_row(&mut self, rhs: &[f64]) -> CoreResult<LinearSolveReport> {
        self.validate_rhs(rhs)?;
        self.snapshot.solve_attempts = self.snapshot.solve_attempts.saturating_add(1);
        let report = match solve_gmres_with_workspace(
            self.operator,
            &self.preconditioner,
            rhs,
            None,
            &self.config,
            &mut self.workspace,
            &mut self.snapshot.counters,
        ) {
            Ok(report) => report,
            Err(error) => {
                self.snapshot.counters.linear_solve_failures = self
                    .snapshot
                    .counters
                    .linear_solve_failures
                    .saturating_add(1);
                self.refresh_workspace_capacity();
                return Err(error);
            }
        };
        self.snapshot.solve_completed = self.snapshot.solve_completed.saturating_add(1);
        self.snapshot.counters.block_linear_iterations = self
            .snapshot
            .counters
            .block_linear_iterations
            .saturating_add(report.iterations);
        self.refresh_workspace_capacity();
        Ok(report)
    }

    /// Solve a batch of independent right-hand sides using the same operator,
    /// preconditioner, and workspace allocation.
    ///
    /// Rows are validated immediately before they are attempted so a malformed
    /// late row retains every completed earlier solution and its spent work.
    pub fn solve_rows(&mut self, right_hand_sides: &[Vec<f64>]) -> Audit2MatrixFreeBatchOutcome {
        self.begin_batch();
        if right_hand_sides.is_empty() {
            return self.failure(
                Audit2MatrixFreeFailurePhase::InputValidation,
                None,
                "Audit-2 matrix-free common-W batch must contain at least one row",
                Vec::new(),
                Vec::new(),
            );
        }

        let mut solutions = Vec::with_capacity(right_hand_sides.len());
        let mut reports = Vec::with_capacity(right_hand_sides.len());
        for (row_index, rhs) in right_hand_sides.iter().enumerate() {
            if let Err(error) = self.validate_rhs(rhs) {
                return self.failure(
                    Audit2MatrixFreeFailurePhase::InputValidation,
                    Some(row_index),
                    error,
                    solutions,
                    reports,
                );
            }
            let report = match self.solve_validated_row(rhs) {
                Ok(report) => report,
                Err(error) => {
                    return self.failure(
                        Audit2MatrixFreeFailurePhase::Solve,
                        Some(row_index),
                        error,
                        solutions,
                        reports,
                    );
                }
            };
            solutions.push(report.x.clone());
            reports.push(report);
        }
        self.complete_batch();
        Audit2MatrixFreeBatchOutcome::Completed(Box::new(Audit2MatrixFreeBatchSuccess {
            solutions,
            solve_reports: reports,
            session: self.snapshot(),
        }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audit2MatrixFreeCorrectionFailurePhase {
    InputValidation,
    CoefficientProjection,
    ResidualPreparation,
    SnapshotPreparation,
    JvpAccess,
    CorrectionJvp,
    CorrectionRhs,
    Solve,
    LinearDiagnostic,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Audit2MatrixFreeCorrectionWork {
    pub preparation_counters: WorkCounters,
    pub correction_jvp_attempts: u64,
    pub correction_jvp_completed: u64,
    pub diagnostic_shifted_apply_attempts: u64,
    pub diagnostic_shifted_apply_completed: u64,
    pub diagnostic_jvp_attempts: u64,
    pub diagnostic_jvp_completed: u64,
    pub coupling_counters: WorkCounters,
    pub session: Option<Audit2MatrixFreeSessionSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit2MatrixFreeCorrectionSuccess {
    pub projection: Audit2CoefficientProjection,
    pub projected_residual: Vec<Vec<f64>>,
    pub correction: Vec<Vec<f64>>,
    pub solve_reports: Vec<LinearSolveReport>,
    pub initial_residual_l2: f64,
    pub linear_residual_l2: f64,
    pub work: Audit2MatrixFreeCorrectionWork,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit2MatrixFreeCorrectionFailure {
    pub phase: Audit2MatrixFreeCorrectionFailurePhase,
    pub message: String,
    pub projection: Option<Audit2CoefficientProjection>,
    pub projected_residual: Option<Vec<Vec<f64>>>,
    pub partial_correction: Vec<Vec<f64>>,
    pub partial_reports: Vec<LinearSolveReport>,
    pub work: Audit2MatrixFreeCorrectionWork,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Audit2MatrixFreeCorrectionOutcome {
    Completed(Box<Audit2MatrixFreeCorrectionSuccess>),
    Failed(Box<Audit2MatrixFreeCorrectionFailure>),
}

fn bump(value: &mut u64) {
    *value = value.saturating_add(1);
}

fn rows_l2(rows: &[Vec<f64>]) -> f64 {
    safe_l2(&rows.iter().flatten().copied().collect::<Vec<_>>())
}

fn matrix_free_projected_context<'a>(
    context: &StepContext<'a>,
) -> CoreResult<(StepContext<'a>, Audit2CoefficientProjection)> {
    if context.shifted.explicit().is_some() {
        return Err(CoreError::InvalidInput(
            "Audit-2 matrix-free correction requires a strict matrix-free StepContext".into(),
        ));
    }
    let mut coefficients = context.coeffs.clone();
    let stages = coefficients.stages();
    let mut report = Audit2CoefficientProjection {
        tolerance: AUDIT2_STRUCTURE_PROJECTION_TOLERANCE,
        max_alpha_forbidden_abs: 0.0,
        max_gamma_upper_abs: 0.0,
        max_gamma_diagonal_error_abs: 0.0,
        projected_alpha_entries: 0,
        projected_gamma_entries: 0,
        projected_structure_bit_exact: false,
        result_independent_fixed_rule: true,
    };
    for i in 0..stages {
        for j in i..stages {
            report.max_alpha_forbidden_abs = report
                .max_alpha_forbidden_abs
                .max(coefficients.alpha[(i, j)].abs());
            if coefficients.alpha[(i, j)] != 0.0 {
                bump(&mut report.projected_alpha_entries);
            }
        }
        report.max_gamma_diagonal_error_abs = report
            .max_gamma_diagonal_error_abs
            .max((coefficients.gamma_matrix[(i, i)] - coefficients.gamma).abs());
        if coefficients.gamma_matrix[(i, i)].to_bits() != coefficients.gamma.to_bits() {
            bump(&mut report.projected_gamma_entries);
        }
        for j in (i + 1)..stages {
            report.max_gamma_upper_abs = report
                .max_gamma_upper_abs
                .max(coefficients.gamma_matrix[(i, j)].abs());
            if coefficients.gamma_matrix[(i, j)] != 0.0 {
                bump(&mut report.projected_gamma_entries);
            }
        }
    }
    if report.max_alpha_forbidden_abs > report.tolerance
        || report.max_gamma_upper_abs > report.tolerance
        || report.max_gamma_diagonal_error_abs > report.tolerance
    {
        return Err(CoreError::Coefficients(format!(
            "coefficient leakage exceeds fixed Audit-2 matrix-free projection tolerance {}",
            report.tolerance
        )));
    }
    for i in 0..stages {
        for j in i..stages {
            coefficients.alpha[(i, j)] = 0.0;
        }
        coefficients.gamma_matrix[(i, i)] = coefficients.gamma;
        for j in (i + 1)..stages {
            coefficients.gamma_matrix[(i, j)] = 0.0;
        }
    }
    coefficients.beta = coefficients.alpha.add(&coefficients.gamma_matrix)?;
    coefficients.l = coefficients
        .beta
        .sub(&DenseMatrix::identity(stages).scale(coefficients.gamma))?;
    coefficients.gamma_rows = (0..stages)
        .map(|i| coefficients.gamma_matrix.row(i).iter().sum())
        .collect();
    report.projected_structure_bit_exact = (0..stages).all(|i| {
        coefficients.gamma_matrix[(i, i)].to_bits() == coefficients.gamma.to_bits()
            && (i..stages).all(|j| coefficients.alpha[(i, j)].to_bits() == 0.0f64.to_bits())
            && ((i + 1)..stages)
                .all(|j| coefficients.gamma_matrix[(i, j)].to_bits() == 0.0f64.to_bits())
    });
    let shifted = ShiftedOperator::new_counted_jvp(
        context.problem.mass_matrix.clone(),
        context.jacobian.clone(),
        context.h,
        coefficients.gamma,
    )?;
    Ok((
        StepContext {
            problem: context.problem,
            t: context.t,
            y: context.y.clone(),
            h: context.h,
            coeffs: coefficients,
            f0: context.f0.clone(),
            ft0: context.ft0.clone(),
            jacobian: context.jacobian.clone(),
            shifted,
        },
        report,
    ))
}

fn correction_failure(
    phase: Audit2MatrixFreeCorrectionFailurePhase,
    error: impl ToString,
    projection: Option<Audit2CoefficientProjection>,
    projected_residual: Option<Vec<Vec<f64>>>,
    partial_correction: Vec<Vec<f64>>,
    partial_reports: Vec<LinearSolveReport>,
    work: Audit2MatrixFreeCorrectionWork,
) -> Audit2MatrixFreeCorrectionOutcome {
    Audit2MatrixFreeCorrectionOutcome::Failed(Box::new(Audit2MatrixFreeCorrectionFailure {
        phase,
        message: error.to_string(),
        projection,
        projected_residual,
        partial_correction,
        partial_reports,
        work,
    }))
}

fn counted_coupling_jvp(
    operator: &dyn LinearOperator,
    input: &[f64],
    output: &mut [f64],
    work: &mut Audit2MatrixFreeCorrectionWork,
    diagnostic: bool,
) -> CoreResult<()> {
    if diagnostic {
        bump(&mut work.diagnostic_jvp_attempts);
    } else {
        bump(&mut work.correction_jvp_attempts);
    }
    work.coupling_counters.jvp_calls = work.coupling_counters.jvp_calls.saturating_add(1);
    work.coupling_counters.jvp_vectors = work.coupling_counters.jvp_vectors.saturating_add(1);
    operator.apply(input, output)?;
    if diagnostic {
        bump(&mut work.diagnostic_jvp_completed);
    } else {
        bump(&mut work.correction_jvp_completed);
    }
    Ok(())
}

/// Compute the projected common-W block-forward correction using only JVP and
/// mass-matrix actions for W solves. One operator/preconditioner/workspace setup
/// is reused across all stages. This remains an opt-in linearized diagnostic.
pub fn run_audit2_matrix_free_common_w_correction(
    context: &StepContext<'_>,
    trial_stages: &[Vec<f64>],
    config: Audit2MatrixFreeCommonWConfig,
) -> Audit2MatrixFreeCorrectionOutcome {
    let mut work = Audit2MatrixFreeCorrectionWork::default();
    let n = context.problem.dimension;
    let s = context.coeffs.stages();
    if trial_stages.len() != s || trial_stages.iter().any(|row| row.len() != n) {
        return correction_failure(
            Audit2MatrixFreeCorrectionFailurePhase::InputValidation,
            "Audit-2 matrix-free trial stage shape mismatch",
            None,
            None,
            Vec::new(),
            Vec::new(),
            work,
        );
    }
    if trial_stages
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return correction_failure(
            Audit2MatrixFreeCorrectionFailurePhase::InputValidation,
            "Audit-2 matrix-free trial stages contain NaN/Inf",
            None,
            None,
            Vec::new(),
            Vec::new(),
            work,
        );
    }
    if context.shifted.explicit().is_some() {
        return correction_failure(
            Audit2MatrixFreeCorrectionFailurePhase::InputValidation,
            "Audit-2 matrix-free correction requires a strict matrix-free StepContext",
            None,
            None,
            Vec::new(),
            Vec::new(),
            work,
        );
    }
    if !context.problem.has_jvp() {
        return correction_failure(
            Audit2MatrixFreeCorrectionFailurePhase::JvpAccess,
            "Audit-2 matrix-free common-W correction requires analytic JVP",
            None,
            None,
            Vec::new(),
            Vec::new(),
            work,
        );
    }
    let (projected, projection) = match matrix_free_projected_context(context) {
        Ok(value) => value,
        Err(error) => {
            return correction_failure(
                Audit2MatrixFreeCorrectionFailurePhase::CoefficientProjection,
                error,
                None,
                None,
                Vec::new(),
                Vec::new(),
                work,
            );
        }
    };
    let block = StructuredBlockSystem::new(&projected);
    let residual = match block.target_residual(trial_stages, &mut work.preparation_counters) {
        Ok(value) => value,
        Err(error) => {
            return correction_failure(
                Audit2MatrixFreeCorrectionFailurePhase::ResidualPreparation,
                error,
                Some(projection),
                None,
                Vec::new(),
                Vec::new(),
                work,
            );
        }
    };
    let snapshot =
        match block.nonlinear_remainder_snapshot(trial_stages, &mut work.preparation_counters) {
            Ok(value) => value,
            Err(error) => {
                return correction_failure(
                    Audit2MatrixFreeCorrectionFailurePhase::SnapshotPreparation,
                    error,
                    Some(projection),
                    Some(residual),
                    Vec::new(),
                    Vec::new(),
                    work,
                );
            }
        };
    let mut session = match Audit2MatrixFreeCommonWSession::new(&projected, config) {
        Ok(value) => value,
        Err(failure) => {
            let Audit2MatrixFreeSessionSetupFailure { error, session } = *failure;
            work.session = Some(session);
            return correction_failure(
                Audit2MatrixFreeCorrectionFailurePhase::Solve,
                error,
                Some(projection),
                Some(residual),
                Vec::new(),
                Vec::new(),
                work,
            );
        }
    };
    session.begin_batch();
    let mut correction: Vec<Vec<f64>> = Vec::with_capacity(s);
    let mut reports = Vec::with_capacity(s);
    let mut p = vec![0.0; n];
    let mut q = vec![0.0; n];
    let mut image = vec![0.0; n];
    for i in 0..s {
        p.fill(0.0);
        q.fill(0.0);
        for (j, z) in correction.iter().enumerate() {
            for k in 0..n {
                p[k] += projected.coeffs.alpha[(i, j)] * z[k];
                q[k] += projected.coeffs.gamma_matrix[(i, j)] * z[k];
            }
        }
        let mut corrected = residual[i].clone();
        if i > 0 {
            let stage_operator = match projected.problem.linearize_matrix_free(
                projected.t + projected.coeffs.c[i] * projected.h,
                &snapshot.states[i],
            ) {
                Ok(value) => value,
                Err(error) => {
                    work.session = Some(session.snapshot());
                    return correction_failure(
                        Audit2MatrixFreeCorrectionFailurePhase::JvpAccess,
                        error,
                        Some(projection),
                        Some(residual),
                        correction,
                        reports,
                        work,
                    );
                }
            };
            if let Err(error) =
                counted_coupling_jvp(stage_operator.as_ref(), &p, &mut image, &mut work, false)
            {
                work.session = Some(session.snapshot());
                return correction_failure(
                    Audit2MatrixFreeCorrectionFailurePhase::CorrectionJvp,
                    error,
                    Some(projection),
                    Some(residual),
                    correction,
                    reports,
                    work,
                );
            }
            for k in 0..n {
                corrected[k] += projected.h * image[k];
            }
            if let Err(error) = counted_coupling_jvp(
                projected.jacobian.as_ref(),
                &q,
                &mut image,
                &mut work,
                false,
            ) {
                work.session = Some(session.snapshot());
                return correction_failure(
                    Audit2MatrixFreeCorrectionFailurePhase::CorrectionJvp,
                    error,
                    Some(projection),
                    Some(residual),
                    correction,
                    reports,
                    work,
                );
            }
            for k in 0..n {
                corrected[k] += projected.h * image[k];
            }
        }
        if corrected.iter().any(|value| !value.is_finite()) {
            work.session = Some(session.snapshot());
            return correction_failure(
                Audit2MatrixFreeCorrectionFailurePhase::CorrectionRhs,
                "Audit-2 matrix-free correction RHS contains NaN/Inf",
                Some(projection),
                Some(residual),
                correction,
                reports,
                work,
            );
        }
        let report = match session.solve_validated_row(&corrected) {
            Ok(value) => value,
            Err(error) => {
                work.session = Some(session.snapshot());
                return correction_failure(
                    Audit2MatrixFreeCorrectionFailurePhase::Solve,
                    error,
                    Some(projection),
                    Some(residual),
                    correction,
                    reports,
                    work,
                );
            }
        };
        correction.push(report.x.clone());
        reports.push(report);
    }
    session.complete_batch();

    let mut diagnostic = vec![vec![0.0; n]; s];
    for i in 0..s {
        bump(&mut work.diagnostic_shifted_apply_attempts);
        if let Err(error) = apply_counted(
            &projected.shifted,
            &correction[i],
            &mut diagnostic[i],
            &mut work.coupling_counters,
            ApplyCategory::Diagnostic,
        ) {
            work.session = Some(session.snapshot());
            return correction_failure(
                Audit2MatrixFreeCorrectionFailurePhase::LinearDiagnostic,
                error,
                Some(projection),
                Some(residual),
                correction,
                reports,
                work,
            );
        }
        bump(&mut work.diagnostic_shifted_apply_completed);
        if i == 0 {
            continue;
        }
        p.fill(0.0);
        q.fill(0.0);
        for (j, z) in correction.iter().take(i).enumerate() {
            for k in 0..n {
                p[k] += projected.coeffs.alpha[(i, j)] * z[k];
                q[k] += projected.coeffs.gamma_matrix[(i, j)] * z[k];
            }
        }
        let stage_operator = match projected.problem.linearize_matrix_free(
            projected.t + projected.coeffs.c[i] * projected.h,
            &snapshot.states[i],
        ) {
            Ok(value) => value,
            Err(error) => {
                work.session = Some(session.snapshot());
                return correction_failure(
                    Audit2MatrixFreeCorrectionFailurePhase::JvpAccess,
                    error,
                    Some(projection),
                    Some(residual),
                    correction,
                    reports,
                    work,
                );
            }
        };
        if let Err(error) =
            counted_coupling_jvp(stage_operator.as_ref(), &p, &mut image, &mut work, true)
        {
            work.session = Some(session.snapshot());
            return correction_failure(
                Audit2MatrixFreeCorrectionFailurePhase::LinearDiagnostic,
                error,
                Some(projection),
                Some(residual),
                correction,
                reports,
                work,
            );
        }
        for k in 0..n {
            diagnostic[i][k] -= projected.h * image[k];
        }
        if let Err(error) =
            counted_coupling_jvp(projected.jacobian.as_ref(), &q, &mut image, &mut work, true)
        {
            work.session = Some(session.snapshot());
            return correction_failure(
                Audit2MatrixFreeCorrectionFailurePhase::LinearDiagnostic,
                error,
                Some(projection),
                Some(residual),
                correction,
                reports,
                work,
            );
        }
        for k in 0..n {
            diagnostic[i][k] -= projected.h * image[k];
        }
    }
    let difference: Vec<Vec<f64>> = diagnostic
        .iter()
        .zip(&residual)
        .map(|(left, right)| left.iter().zip(right).map(|(a, b)| a - b).collect())
        .collect();
    work.session = Some(session.snapshot());
    Audit2MatrixFreeCorrectionOutcome::Completed(Box::new(Audit2MatrixFreeCorrectionSuccess {
        projection,
        projected_residual: residual.clone(),
        correction,
        solve_reports: reports,
        initial_residual_l2: rows_l2(&residual),
        linear_residual_l2: rows_l2(&difference),
        work,
    }))
}
