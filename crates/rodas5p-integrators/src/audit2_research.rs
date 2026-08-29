//! Explicitly opt-in Audit-2 research diagnostics.
//!
//! This module is compiled only with the non-default `audit2-research` feature.
//! It is not called by an integration, gate, campaign, or production dispatch.
//! The common-W result is a linearized correction for an explicitly projected
//! coefficient target; its norm is never an acceptance or nonlinear-validity test.

use serde::{Deserialize, Serialize};

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearOperator, LuFactorization, ShiftedOperator,
    WorkCounters, inverse, safe_l2,
};

use crate::{NonlinearRemainderSnapshot, StepContext, StructuredBlockSystem};

/// Fixed before any correction result is evaluated.
///
/// The rule is an absolute f64 roundoff allowance for the official O(1)
/// coefficient snapshot. It is not fitted to correction residuals, errors, or
/// campaign outcomes.
pub const AUDIT2_STRUCTURE_PROJECTION_TOLERANCE: f64 = 64.0 * f64::EPSILON;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audit2CorrectionBackend {
    #[default]
    FullTargetOracle,
    CommonWBlockForward,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit2ResearchConfig {
    pub backend: Audit2CorrectionBackend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audit2FailurePhase {
    InputValidation,
    CoefficientProjection,
    JvpAccess,
    FullTargetAssembly,
    CommonWSetup,
    Factorization,
    CorrectionJvp,
    CorrectionRhs,
    Solve,
    LinearDiagnostic,
    NonlinearResidualAfter,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2CoefficientProjection {
    pub tolerance: f64,
    pub max_alpha_forbidden_abs: f64,
    pub max_gamma_upper_abs: f64,
    pub max_gamma_diagonal_error_abs: f64,
    pub projected_alpha_entries: u64,
    pub projected_gamma_entries: u64,
    pub projected_structure_bit_exact: bool,
    pub result_independent_fixed_rule: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Audit2CorrectionWork {
    pub common_w_setup_attempts: u64,
    pub common_w_setup_completed: u64,
    pub full_target_setup_attempts: u64,
    pub full_target_setup_completed: u64,
    pub factorization_attempts: u64,
    pub factorization_completed: u64,
    pub solve_attempts: u64,
    pub solve_completed: u64,
    pub correction_jvp_attempts: u64,
    pub correction_jvp_completed: u64,
    pub linear_diagnostic_apply_attempts: u64,
    pub linear_diagnostic_apply_completed: u64,
    pub diagnostic_shifted_apply_attempts: u64,
    pub diagnostic_shifted_apply_completed: u64,
    pub diagnostic_jvp_attempts: u64,
    pub diagnostic_jvp_completed: u64,
    pub nonlinear_residual_after_attempts: u64,
    pub nonlinear_residual_after_completed: u64,
    pub counters: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2CorrectionSuccess {
    pub backend: Audit2CorrectionBackend,
    pub projection: Audit2CoefficientProjection,
    /// Work used once to construct the shared target snapshot and residual.
    pub preparation_counters: WorkCounters,
    pub correction: Vec<Vec<f64>>,
    pub correction_l2: f64,
    pub initial_residual_l2: f64,
    pub linear_residual_l2: f64,
    pub nonlinear_residual_after_l2: f64,
    pub work: Audit2CorrectionWork,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2CorrectionFailure {
    pub backend: Audit2CorrectionBackend,
    pub projection: Option<Audit2CoefficientProjection>,
    pub phase: Audit2FailurePhase,
    pub message: String,
    pub preparation_counters: WorkCounters,
    pub partial_correction: Vec<Vec<f64>>,
    pub initial_residual_l2: Option<f64>,
    pub work: Audit2CorrectionWork,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Audit2CorrectionOutcome {
    Completed(Audit2CorrectionSuccess),
    Failed(Audit2CorrectionFailure),
}

impl Audit2CorrectionOutcome {
    pub fn completed(&self) -> Option<&Audit2CorrectionSuccess> {
        match self {
            Self::Completed(value) => Some(value),
            Self::Failed(_) => None,
        }
    }

    pub fn failed(&self) -> Option<&Audit2CorrectionFailure> {
        match self {
            Self::Completed(_) => None,
            Self::Failed(value) => Some(value),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2CorrectionComparison {
    pub projection: Audit2CoefficientProjection,
    /// This preparation belongs to the comparison once; do not add the copies
    /// retained inside both arm outcomes.
    pub shared_preparation_counters: WorkCounters,
    pub matching_trial_stage_states: bool,
    pub full_target: Audit2CorrectionOutcome,
    pub common_w: Audit2CorrectionOutcome,
    pub target_condition_f: Option<f64>,
    pub full_target_backward_error: Option<f64>,
    pub common_w_backward_error: Option<f64>,
    pub state_absolute_difference_l2: Option<f64>,
    pub state_relative_difference: Option<f64>,
    pub independent_validation_apply_attempts: u64,
    pub independent_validation_apply_completed: u64,
    pub independent_validation_counters: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2SharedFailure {
    pub phase: Audit2FailurePhase,
    pub message: String,
    pub preparation_counters: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Audit2ComparisonOutcome {
    Completed(Box<Audit2CorrectionComparison>),
    Failed(Box<Audit2SharedFailure>),
}

struct PreparedTarget<'a> {
    context: StepContext<'a>,
    snapshot: NonlinearRemainderSnapshot,
    residual: Vec<Vec<f64>>,
    projection: Audit2CoefficientProjection,
    preparation_counters: WorkCounters,
}

struct CommonWFailure {
    phase: Audit2FailurePhase,
    error: CoreError,
    work: Audit2CorrectionWork,
    partial_correction: Vec<Vec<f64>>,
}

type CommonWResult = Result<(Vec<Vec<f64>>, Audit2CorrectionWork), Box<CommonWFailure>>;

fn bump(value: &mut u64) {
    *value = value.saturating_add(1);
}

fn flatten(rows: &[Vec<f64>]) -> Vec<f64> {
    rows.iter().flatten().copied().collect()
}

fn unflatten(values: &[f64], stages: usize, dimension: usize) -> Vec<Vec<f64>> {
    (0..stages)
        .map(|stage| values[stage * dimension..(stage + 1) * dimension].to_vec())
        .collect()
}

fn rows_l2(rows: &[Vec<f64>]) -> f64 {
    safe_l2(&flatten(rows))
}

fn projected_context<'a>(
    context: &StepContext<'a>,
) -> CoreResult<(StepContext<'a>, Audit2CoefficientProjection)> {
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
            "official coefficient leakage exceeds fixed Audit-2 projection tolerance {}",
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
    let shifted = ShiftedOperator::new(
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

fn prepare_target<'a>(
    context: &StepContext<'a>,
    trial_stages: &[Vec<f64>],
) -> Result<PreparedTarget<'a>, Box<Audit2SharedFailure>> {
    let mut preparation_counters = WorkCounters::default();
    let n = context.problem.dimension;
    let s = context.coeffs.stages();
    if trial_stages.len() != s || trial_stages.iter().any(|row| row.len() != n) {
        return Err(Box::new(Audit2SharedFailure {
            phase: Audit2FailurePhase::InputValidation,
            message: "Audit-2 trial stage shape mismatch".into(),
            preparation_counters,
        }));
    }
    if trial_stages
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(Box::new(Audit2SharedFailure {
            phase: Audit2FailurePhase::InputValidation,
            message: "Audit-2 trial stages contain NaN/Inf".into(),
            preparation_counters,
        }));
    }
    let (projected, projection) = projected_context(context).map_err(|error| {
        Box::new(Audit2SharedFailure {
            phase: Audit2FailurePhase::CoefficientProjection,
            message: error.to_string(),
            preparation_counters,
        })
    })?;
    let block = StructuredBlockSystem::new(&projected);
    let snapshot = block
        .nonlinear_remainder_snapshot(trial_stages, &mut preparation_counters)
        .map_err(|error| {
            Box::new(Audit2SharedFailure {
                phase: Audit2FailurePhase::InputValidation,
                message: error.to_string(),
                preparation_counters,
            })
        })?;
    let lhs = block
        .apply(trial_stages, &mut preparation_counters)
        .map_err(|error| {
            Box::new(Audit2SharedFailure {
                phase: Audit2FailurePhase::InputValidation,
                message: error.to_string(),
                preparation_counters,
            })
        })?;
    let residual: Vec<Vec<f64>> = lhs
        .iter()
        .zip(&snapshot.rhs)
        .map(|(left, right)| left.iter().zip(right).map(|(a, b)| a - b).collect())
        .collect();
    if residual.iter().flatten().any(|value| !value.is_finite()) {
        return Err(Box::new(Audit2SharedFailure {
            phase: Audit2FailurePhase::InputValidation,
            message: "Audit-2 target residual contains NaN/Inf".into(),
            preparation_counters,
        }));
    }
    Ok(PreparedTarget {
        context: projected,
        snapshot,
        residual,
        projection,
        preparation_counters,
    })
}

fn failure(
    prepared: &PreparedTarget<'_>,
    backend: Audit2CorrectionBackend,
    phase: Audit2FailurePhase,
    error: impl ToString,
    work: Audit2CorrectionWork,
    partial_correction: Vec<Vec<f64>>,
) -> Audit2CorrectionOutcome {
    Audit2CorrectionOutcome::Failed(Audit2CorrectionFailure {
        backend,
        projection: Some(prepared.projection.clone()),
        phase,
        message: error.to_string(),
        preparation_counters: prepared.preparation_counters,
        partial_correction,
        initial_residual_l2: Some(rows_l2(&prepared.residual)),
        work,
    })
}

fn failed_preparation(
    backend: Audit2CorrectionBackend,
    shared: Box<Audit2SharedFailure>,
) -> Audit2CorrectionOutcome {
    let shared = *shared;
    Audit2CorrectionOutcome::Failed(Audit2CorrectionFailure {
        backend,
        projection: None,
        phase: shared.phase,
        message: shared.message,
        preparation_counters: shared.preparation_counters,
        partial_correction: Vec::new(),
        initial_residual_l2: None,
        work: Audit2CorrectionWork::default(),
    })
}

fn updated_stages(stages: &[Vec<f64>], correction: &[Vec<f64>]) -> CoreResult<Vec<Vec<f64>>> {
    if stages.len() != correction.len()
        || stages
            .iter()
            .zip(correction)
            .any(|(a, b)| a.len() != b.len())
    {
        return Err(CoreError::Dimension(
            "Audit-2 update stage shape mismatch".into(),
        ));
    }
    let updated: Vec<Vec<f64>> = stages
        .iter()
        .zip(correction)
        .map(|(stage, delta)| stage.iter().zip(delta).map(|(a, b)| a - b).collect())
        .collect();
    if updated.iter().flatten().all(|value| value.is_finite()) {
        Ok(updated)
    } else {
        Err(CoreError::NonFinite(
            "Audit-2 Newton update contains NaN/Inf".into(),
        ))
    }
}

fn nonlinear_residual_after(
    prepared: &PreparedTarget<'_>,
    trial_stages: &[Vec<f64>],
    correction: &[Vec<f64>],
    work: &mut Audit2CorrectionWork,
) -> CoreResult<f64> {
    bump(&mut work.nonlinear_residual_after_attempts);
    let updated = updated_stages(trial_stages, correction)?;
    let block = StructuredBlockSystem::new(&prepared.context);
    let residual = block.target_residual(&updated, &mut work.counters)?;
    bump(&mut work.nonlinear_residual_after_completed);
    Ok(rows_l2(&residual))
}

fn charge_jvp_attempt(work: &mut Audit2CorrectionWork, diagnostic: bool) {
    if diagnostic {
        bump(&mut work.diagnostic_jvp_attempts);
    } else {
        bump(&mut work.correction_jvp_attempts);
    }
    work.counters.jvp_calls = work.counters.jvp_calls.saturating_add(1);
    work.counters.jvp_vectors = work.counters.jvp_vectors.saturating_add(1);
}

fn apply_jvp_attempt(
    operator: &dyn LinearOperator,
    input: &[f64],
    output: &mut [f64],
    work: &mut Audit2CorrectionWork,
    diagnostic: bool,
) -> CoreResult<()> {
    charge_jvp_attempt(work, diagnostic);
    operator.apply(input, output)?;
    if diagnostic {
        bump(&mut work.diagnostic_jvp_completed);
    } else {
        bump(&mut work.correction_jvp_completed);
    }
    Ok(())
}

fn run_full_target(
    prepared: &PreparedTarget<'_>,
    trial_stages: &[Vec<f64>],
) -> (Audit2CorrectionOutcome, Option<DenseMatrix>) {
    let backend = Audit2CorrectionBackend::FullTargetOracle;
    let mut work = Audit2CorrectionWork::default();
    bump(&mut work.full_target_setup_attempts);
    let block = StructuredBlockSystem::new(&prepared.context);
    let matrix =
        match block.target_jacobian_matrix(trial_stages, &prepared.snapshot, &mut work.counters) {
            Ok(matrix) => {
                bump(&mut work.full_target_setup_completed);
                matrix
            }
            Err(error) => {
                return (
                    failure(
                        prepared,
                        backend,
                        Audit2FailurePhase::FullTargetAssembly,
                        error,
                        work,
                        Vec::new(),
                    ),
                    None,
                );
            }
        };
    bump(&mut work.factorization_attempts);
    work.counters.direct_factorizations = work.counters.direct_factorizations.saturating_add(1);
    let factor = match LuFactorization::new(&matrix) {
        Ok(factor) => {
            bump(&mut work.factorization_completed);
            factor
        }
        Err(error) => {
            return (
                failure(
                    prepared,
                    backend,
                    Audit2FailurePhase::Factorization,
                    error,
                    work,
                    Vec::new(),
                ),
                Some(matrix),
            );
        }
    };
    let rhs = flatten(&prepared.residual);
    bump(&mut work.solve_attempts);
    work.counters.direct_solve_calls = work.counters.direct_solve_calls.saturating_add(1);
    let correction_flat = match factor.solve(&rhs) {
        Ok(correction) => {
            bump(&mut work.solve_completed);
            correction
        }
        Err(error) => {
            return (
                failure(
                    prepared,
                    backend,
                    Audit2FailurePhase::Solve,
                    error,
                    work,
                    Vec::new(),
                ),
                Some(matrix),
            );
        }
    };
    let correction = unflatten(
        &correction_flat,
        prepared.context.coeffs.stages(),
        prepared.context.problem.dimension,
    );
    bump(&mut work.linear_diagnostic_apply_attempts);
    work.counters.diagnostic_matvecs = work.counters.diagnostic_matvecs.saturating_add(1);
    let image = match matrix.matvec(&correction_flat) {
        Ok(image) => {
            bump(&mut work.linear_diagnostic_apply_completed);
            image
        }
        Err(error) => {
            return (
                failure(
                    prepared,
                    backend,
                    Audit2FailurePhase::LinearDiagnostic,
                    error,
                    work,
                    correction,
                ),
                Some(matrix),
            );
        }
    };
    let linear_residual_l2 = safe_l2(
        &image
            .iter()
            .zip(&rhs)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    );
    let nonlinear_residual_after_l2 =
        match nonlinear_residual_after(prepared, trial_stages, &correction, &mut work) {
            Ok(value) => value,
            Err(error) => {
                return (
                    failure(
                        prepared,
                        backend,
                        Audit2FailurePhase::NonlinearResidualAfter,
                        error,
                        work,
                        correction,
                    ),
                    Some(matrix),
                );
            }
        };
    let success = Audit2CorrectionSuccess {
        backend,
        projection: prepared.projection.clone(),
        preparation_counters: prepared.preparation_counters,
        correction_l2: safe_l2(&correction_flat),
        correction,
        initial_residual_l2: safe_l2(&rhs),
        linear_residual_l2,
        nonlinear_residual_after_l2,
        work,
    };
    (Audit2CorrectionOutcome::Completed(success), Some(matrix))
}

fn common_w_correction(prepared: &PreparedTarget<'_>) -> CommonWResult {
    let context = &prepared.context;
    let mut work = Audit2CorrectionWork::default();
    if !context.problem.has_jvp() {
        return Err(Box::new(CommonWFailure {
            phase: Audit2FailurePhase::JvpAccess,
            error: CoreError::InvalidInput(
                "Audit-2 common-W research entry requires analytic JVP".into(),
            ),
            work,
            partial_correction: Vec::new(),
        }));
    }
    let w = match context.shifted.explicit() {
        Some(matrix) => matrix,
        None => {
            return Err(Box::new(CommonWFailure {
                phase: Audit2FailurePhase::CommonWSetup,
                error: CoreError::InvalidInput(
                    "Audit-2 common-W research entry requires explicit W".into(),
                ),
                work,
                partial_correction: Vec::new(),
            }));
        }
    };
    bump(&mut work.common_w_setup_attempts);
    bump(&mut work.factorization_attempts);
    work.counters.direct_factorizations = work.counters.direct_factorizations.saturating_add(1);
    let factor = match LuFactorization::new(w) {
        Ok(factor) => {
            bump(&mut work.factorization_completed);
            bump(&mut work.common_w_setup_completed);
            factor
        }
        Err(error) => {
            return Err(Box::new(CommonWFailure {
                phase: Audit2FailurePhase::Factorization,
                error,
                work,
                partial_correction: Vec::new(),
            }));
        }
    };
    let n = context.problem.dimension;
    let s = context.coeffs.stages();
    let mut correction: Vec<Vec<f64>> = Vec::with_capacity(s);
    let mut p = vec![0.0; n];
    let mut q = vec![0.0; n];
    let mut image = vec![0.0; n];
    for i in 0..s {
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
                    return Err(Box::new(CommonWFailure {
                        phase: Audit2FailurePhase::JvpAccess,
                        error,
                        work,
                        partial_correction: correction,
                    }));
                }
            };
            if let Err(error) =
                apply_jvp_attempt(stage_operator.as_ref(), &p, &mut image, &mut work, false)
            {
                return Err(Box::new(CommonWFailure {
                    phase: Audit2FailurePhase::CorrectionJvp,
                    error,
                    work,
                    partial_correction: correction,
                }));
            }
            for k in 0..n {
                corrected[k] += context.h * image[k];
            }
            if let Err(error) =
                apply_jvp_attempt(context.jacobian.as_ref(), &q, &mut image, &mut work, false)
            {
                return Err(Box::new(CommonWFailure {
                    phase: Audit2FailurePhase::CorrectionJvp,
                    error,
                    work,
                    partial_correction: correction,
                }));
            }
            for k in 0..n {
                corrected[k] += context.h * image[k];
            }
        }
        if corrected.iter().any(|value| !value.is_finite()) {
            return Err(Box::new(CommonWFailure {
                phase: Audit2FailurePhase::CorrectionRhs,
                error: CoreError::NonFinite("Audit-2 correction RHS contains NaN/Inf".into()),
                work,
                partial_correction: correction,
            }));
        }
        bump(&mut work.solve_attempts);
        work.counters.direct_solve_calls = work.counters.direct_solve_calls.saturating_add(1);
        let row = match factor.solve(&corrected) {
            Ok(row) => {
                bump(&mut work.solve_completed);
                row
            }
            Err(error) => {
                return Err(Box::new(CommonWFailure {
                    phase: Audit2FailurePhase::Solve,
                    error,
                    work,
                    partial_correction: correction,
                }));
            }
        };
        correction.push(row);
    }
    Ok((correction, work))
}

fn common_linear_diagnostic(
    prepared: &PreparedTarget<'_>,
    correction: &[Vec<f64>],
    work: &mut Audit2CorrectionWork,
) -> CoreResult<Vec<Vec<f64>>> {
    let context = &prepared.context;
    let n = context.problem.dimension;
    let s = context.coeffs.stages();
    let mut out = vec![vec![0.0; n]; s];
    let mut p = vec![0.0; n];
    let mut q = vec![0.0; n];
    let mut image = vec![0.0; n];
    bump(&mut work.linear_diagnostic_apply_attempts);
    for i in 0..s {
        bump(&mut work.diagnostic_shifted_apply_attempts);
        work.counters.diagnostic_matvecs = work.counters.diagnostic_matvecs.saturating_add(1);
        // `ShiftedOperator::new` does not attach matrix-free work metadata,
        // but its apply path still executes one frozen-J action and, when
        // present, one mass-matrix action. Charge attempts before calling so a
        // failed diagnostic cannot erase requested work.
        work.counters.jvp_calls = work.counters.jvp_calls.saturating_add(1);
        work.counters.jvp_vectors = work.counters.jvp_vectors.saturating_add(1);
        if context.problem.mass_matrix.is_some() {
            work.counters.mass_matvecs = work.counters.mass_matvecs.saturating_add(1);
        }
        context.shifted.apply(&correction[i], &mut out[i])?;
        bump(&mut work.diagnostic_shifted_apply_completed);
        if i == 0 {
            continue;
        }
        p.fill(0.0);
        q.fill(0.0);
        for (j, z) in correction.iter().take(i).enumerate() {
            for k in 0..n {
                p[k] += context.coeffs.alpha[(i, j)] * z[k];
                q[k] += context.coeffs.gamma_matrix[(i, j)] * z[k];
            }
        }
        let stage_operator = context.problem.linearize_matrix_free(
            context.t + context.coeffs.c[i] * context.h,
            &prepared.snapshot.states[i],
        )?;
        apply_jvp_attempt(stage_operator.as_ref(), &p, &mut image, work, true)?;
        for k in 0..n {
            out[i][k] -= context.h * image[k];
        }
        apply_jvp_attempt(context.jacobian.as_ref(), &q, &mut image, work, true)?;
        for k in 0..n {
            out[i][k] -= context.h * image[k];
        }
    }
    bump(&mut work.linear_diagnostic_apply_completed);
    Ok(out)
}

fn run_common_w(
    prepared: &PreparedTarget<'_>,
    trial_stages: &[Vec<f64>],
) -> Audit2CorrectionOutcome {
    let backend = Audit2CorrectionBackend::CommonWBlockForward;
    let (correction, mut work) = match common_w_correction(prepared) {
        Ok(value) => value,
        Err(failure_report) => {
            let failure_report = *failure_report;
            return failure(
                prepared,
                backend,
                failure_report.phase,
                failure_report.error,
                failure_report.work,
                failure_report.partial_correction,
            );
        }
    };
    let image = match common_linear_diagnostic(prepared, &correction, &mut work) {
        Ok(image) => image,
        Err(error) => {
            return failure(
                prepared,
                backend,
                Audit2FailurePhase::LinearDiagnostic,
                error,
                work,
                correction,
            );
        }
    };
    let difference: Vec<Vec<f64>> = image
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
                    backend,
                    Audit2FailurePhase::NonlinearResidualAfter,
                    error,
                    work,
                    correction,
                );
            }
        };
    let flat = flatten(&correction);
    Audit2CorrectionOutcome::Completed(Audit2CorrectionSuccess {
        backend,
        projection: prepared.projection.clone(),
        preparation_counters: prepared.preparation_counters,
        correction,
        correction_l2: safe_l2(&flat),
        initial_residual_l2: rows_l2(&prepared.residual),
        linear_residual_l2: rows_l2(&difference),
        nonlinear_residual_after_l2,
        work,
    })
}

fn independent_backward_error(
    matrix: &DenseMatrix,
    matrix_norm: f64,
    rhs: &[f64],
    outcome: &Audit2CorrectionOutcome,
    attempts: &mut u64,
    completed: &mut u64,
    counters: &mut WorkCounters,
) -> Option<f64> {
    let success = outcome.completed()?;
    bump(attempts);
    counters.diagnostic_matvecs = counters.diagnostic_matvecs.saturating_add(1);
    let correction = flatten(&success.correction);
    let image = matrix.matvec(&correction).ok()?;
    bump(completed);
    let residual = safe_l2(
        &image
            .iter()
            .zip(rhs)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    );
    let rhs_norm = safe_l2(rhs);
    let denominator = matrix_norm * success.correction_l2 + rhs_norm;
    if denominator == 0.0 {
        Some(if residual == 0.0 { 0.0 } else { f64::INFINITY })
    } else {
        let value = residual / denominator;
        value.is_finite().then_some(value)
    }
}

/// Run one explicitly selected research backend. The default config selects
/// the full-target oracle; production solver dispatch is not involved.
pub fn run_audit2_research_correction(
    context: &StepContext<'_>,
    trial_stages: &[Vec<f64>],
    config: Audit2ResearchConfig,
) -> Audit2CorrectionOutcome {
    let prepared = match prepare_target(context, trial_stages) {
        Ok(prepared) => prepared,
        Err(shared) => return failed_preparation(config.backend, shared),
    };
    match config.backend {
        Audit2CorrectionBackend::FullTargetOracle => run_full_target(&prepared, trial_stages).0,
        Audit2CorrectionBackend::CommonWBlockForward => run_common_w(&prepared, trial_stages),
    }
}

/// Evaluate both research backends against one projected target snapshot and
/// one initial residual at exactly the same supplied trial stage values.
pub fn compare_audit2_research_corrections(
    context: &StepContext<'_>,
    trial_stages: &[Vec<f64>],
) -> Audit2ComparisonOutcome {
    let prepared = match prepare_target(context, trial_stages) {
        Ok(prepared) => prepared,
        Err(shared) => return Audit2ComparisonOutcome::Failed(shared),
    };
    let (full_target, target_matrix) = run_full_target(&prepared, trial_stages);
    let common_w = run_common_w(&prepared, trial_stages);
    let mut target_condition_f = None;
    let mut full_target_backward_error = None;
    let mut common_w_backward_error = None;
    let mut state_absolute_difference_l2 = None;
    let mut state_relative_difference = None;
    let mut independent_validation_apply_attempts = 0;
    let mut independent_validation_apply_completed = 0;
    let mut independent_validation_counters = WorkCounters::default();
    if let Some(matrix) = target_matrix {
        let matrix_norm = safe_l2(matrix.as_slice());
        if matrix_norm.is_finite()
            && let Ok(matrix_inverse) = inverse(&matrix)
        {
            let value = matrix_norm * safe_l2(matrix_inverse.as_slice());
            if value.is_finite() {
                target_condition_f = Some(value);
            }
        }
        let rhs = flatten(&prepared.residual);
        full_target_backward_error = independent_backward_error(
            &matrix,
            matrix_norm,
            &rhs,
            &full_target,
            &mut independent_validation_apply_attempts,
            &mut independent_validation_apply_completed,
            &mut independent_validation_counters,
        );
        common_w_backward_error = independent_backward_error(
            &matrix,
            matrix_norm,
            &rhs,
            &common_w,
            &mut independent_validation_apply_attempts,
            &mut independent_validation_apply_completed,
            &mut independent_validation_counters,
        );
    }
    if let (Some(reference), Some(candidate)) = (full_target.completed(), common_w.completed()) {
        let reference_flat = flatten(&reference.correction);
        let candidate_flat = flatten(&candidate.correction);
        let difference = safe_l2(
            &candidate_flat
                .iter()
                .zip(&reference_flat)
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>(),
        );
        state_absolute_difference_l2 = Some(difference);
        let reference_norm = safe_l2(&reference_flat);
        if reference_norm > 0.0 {
            let relative = difference / reference_norm;
            if relative.is_finite() {
                state_relative_difference = Some(relative);
            }
        }
    }
    Audit2ComparisonOutcome::Completed(Box::new(Audit2CorrectionComparison {
        projection: prepared.projection.clone(),
        shared_preparation_counters: prepared.preparation_counters,
        matching_trial_stage_states: true,
        full_target,
        common_w,
        target_condition_f,
        full_target_backward_error,
        common_w_backward_error,
        state_absolute_difference_l2,
        state_relative_difference,
        independent_validation_apply_attempts,
        independent_validation_apply_completed,
        independent_validation_counters,
    }))
}
