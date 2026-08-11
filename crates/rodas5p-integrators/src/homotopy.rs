use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearMethod, LinearOperator, LinearSolverConfig,
    LuFactorization, PreconditionerKind, WorkCounters, error_scale, inverse,
    load_rodas5p_coefficients, safe_l2, wrms,
};
use serde::Serialize;

use crate::{
    KrylovState, OdeProblem, OutputBudgetPolicy, StepCertificate, StepResult,
    StructuredBlockSystem, build_step_context, finish_step, flatten, sequential_stages,
};

/// Validated dimensionless partial-coupling parameters.
///
/// Direct construction is intentionally forbidden so callers cannot bypass the `[0, 1]`
/// validation performed by [`PartialCouplingParameters::new`].
///
/// ```compile_fail
/// use rodas5p_integrators::PartialCouplingParameters;
/// let _ = PartialCouplingParameters { theta: 2.0, lambda: 0.5 };
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartialCouplingParameters {
    theta: f64,
    lambda: f64,
}

impl PartialCouplingParameters {
    pub fn new(theta: f64, lambda: f64) -> CoreResult<Self> {
        if !theta.is_finite() || !lambda.is_finite() {
            return Err(CoreError::NonFinite(
                "partial-coupling parameters contain NaN/Inf".into(),
            ));
        }
        if !(0.0..=1.0).contains(&theta) || !(0.0..=1.0).contains(&lambda) {
            return Err(CoreError::InvalidInput(
                "partial-coupling parameters must lie in [0, 1]".into(),
            ));
        }
        Ok(Self { theta, lambda })
    }

    pub fn theta(self) -> f64 {
        self.theta
    }

    pub fn lambda(self) -> f64 {
        self.lambda
    }

    pub fn eta(self) -> f64 {
        self.theta + self.lambda * (1.0 - self.theta)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AffineOutputCertificate {
    pub residual_norm: f64,
    pub relative_residual: f64,
    pub output_wrms: f64,
    pub correction_norm: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NonlinearOutputCertificate {
    pub stage_residual_norm: f64,
    pub relative_stage_residual: f64,
    pub output_wrms: f64,
    pub correction_norm: f64,
    pub embedded_error: f64,
    pub combined_error: f64,
}

/// Certify an approximate nonlinear stage vector against the original RODAS5P equations.
///
/// This is intentionally an exact-reference certificate: it assembles and factors the
/// current nonlinear target Jacobian.  The later speculative fast path may use cheaper
/// bounds, but those bounds must be calibrated against this oracle rather than replacing it.
pub fn certify_nonlinear_target(
    block: &StructuredBlockSystem<'_, '_>,
    stages: &[Vec<f64>],
    atol: f64,
    rtol: f64,
    counters: &mut WorkCounters,
) -> CoreResult<NonlinearOutputCertificate> {
    if !(atol > 0.0 && atol.is_finite()) {
        return Err(CoreError::InvalidInput(
            "nonlinear target certificate atol must be finite and positive".into(),
        ));
    }
    if !(rtol >= 0.0 && rtol.is_finite()) {
        return Err(CoreError::InvalidInput(
            "nonlinear target certificate rtol must be finite and nonnegative".into(),
        ));
    }

    let snapshot = block.nonlinear_remainder_snapshot(stages, counters)?;
    let applied = block.apply(stages, counters)?;
    let residual_rows: Vec<Vec<f64>> = applied
        .iter()
        .zip(&snapshot.rhs)
        .map(|(lhs, rhs)| lhs.iter().zip(rhs).map(|(a, b)| a - b).collect())
        .collect();
    let residual = flatten(&residual_rows);
    let rhs = flatten(&snapshot.rhs);

    let target_jacobian = block.target_jacobian_matrix(stages, &snapshot, counters)?;
    counters.direct_factorizations += 1;
    let factor = LuFactorization::new(&target_jacobian)?;
    counters.direct_solve_calls += 1;
    let correction = factor.solve(&residual)?;

    let mut output_error = vec![0.0; block.n];
    let mut y_candidate = block.context.y.clone();
    let mut embedded_error_vector = vec![0.0; block.n];
    for stage in 0..block.s {
        for component in 0..block.n {
            output_error[component] +=
                block.context.coeffs.b[stage] * correction[stage * block.n + component];
            y_candidate[component] += block.context.coeffs.b[stage] * stages[stage][component];
            embedded_error_vector[component] +=
                block.context.coeffs.btilde[stage] * stages[stage][component];
        }
    }

    let scale = error_scale(&block.context.y, &y_candidate, &[atol], rtol)?;
    let stage_residual_norm = safe_l2(&residual);
    let certificate = NonlinearOutputCertificate {
        stage_residual_norm,
        relative_stage_residual: stage_residual_norm / safe_l2(&rhs).max(f64::MIN_POSITIVE),
        output_wrms: wrms(&output_error, &scale)?,
        correction_norm: safe_l2(&correction),
        embedded_error: wrms(&embedded_error_vector, &scale)?,
        combined_error: 0.0,
    };
    let certificate = NonlinearOutputCertificate {
        combined_error: certificate.embedded_error + certificate.output_wrms,
        ..certificate
    };
    if [
        certificate.stage_residual_norm,
        certificate.relative_stage_residual,
        certificate.output_wrms,
        certificate.correction_norm,
        certificate.embedded_error,
        certificate.combined_error,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        Ok(certificate)
    } else {
        Err(CoreError::NonFinite(
            "nonlinear target certificate contains NaN/Inf".into(),
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum HomotopyPredictor {
    Euler,
    AdamsBashforth2,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyPathConfig {
    theta: f64,
    q: usize,
    path_rounds: usize,
    predictor: HomotopyPredictor,
    corrections_per_point: usize,
}

impl HomotopyPathConfig {
    pub fn new(
        theta: f64,
        q: usize,
        path_rounds: usize,
        predictor: HomotopyPredictor,
        corrections_per_point: usize,
    ) -> CoreResult<Self> {
        if !theta.is_finite() {
            return Err(CoreError::NonFinite(
                "homotopy path theta contains NaN/Inf".into(),
            ));
        }
        if !(0.0..=1.0).contains(&theta) {
            return Err(CoreError::InvalidInput(
                "homotopy path theta must lie in [0, 1]".into(),
            ));
        }
        if q >= 8 {
            return Err(CoreError::InvalidInput(
                "homotopy path truncation depth q must lie in 0..8".into(),
            ));
        }
        if path_rounds == 0 {
            return Err(CoreError::InvalidInput(
                "homotopy path needs at least one round".into(),
            ));
        }
        if corrections_per_point > 8 {
            return Err(CoreError::InvalidInput(
                "homotopy path corrections per point must not exceed 8".into(),
            ));
        }
        Ok(Self {
            theta,
            q,
            path_rounds,
            predictor,
            corrections_per_point,
        })
    }

    pub fn theta(&self) -> f64 {
        self.theta
    }

    pub fn q(&self) -> usize {
        self.q
    }

    pub fn path_rounds(&self) -> usize {
        self.path_rounds
    }

    pub fn predictor(&self) -> HomotopyPredictor {
        self.predictor
    }

    pub fn corrections_per_point(&self) -> usize {
        self.corrections_per_point
    }
}

/// One bounded nonstationary homotopy round.
///
/// `lambda_end`, `theta`, and `damping` are dimensionless.  Changing `theta` between
/// rounds is interpreted as a nonstationary preconditioned sweep, not as one smooth
/// classical homotopy curve.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyRoundSpec {
    lambda_end: f64,
    theta: f64,
    q: usize,
    damping: f64,
    corrections: usize,
}

impl HomotopyRoundSpec {
    pub fn new(
        lambda_end: f64,
        theta: f64,
        q: usize,
        damping: f64,
        corrections: usize,
    ) -> CoreResult<Self> {
        if ![lambda_end, theta, damping]
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(CoreError::NonFinite(
                "homotopy round parameters contain NaN/Inf".into(),
            ));
        }
        if !(lambda_end > 0.0 && lambda_end <= 1.0) {
            return Err(CoreError::InvalidInput(
                "homotopy round lambda endpoint must lie in (0, 1]".into(),
            ));
        }
        if !(0.0..=1.0).contains(&theta) {
            return Err(CoreError::InvalidInput(
                "homotopy round theta must lie in [0, 1]".into(),
            ));
        }
        if q >= 8 {
            return Err(CoreError::InvalidInput(
                "homotopy round truncation depth q must lie in 0..8".into(),
            ));
        }
        if !(damping > 0.0 && damping <= 1.0) {
            return Err(CoreError::InvalidInput(
                "homotopy round damping must lie in (0, 1]".into(),
            ));
        }
        if corrections > 2 {
            return Err(CoreError::InvalidInput(
                "homotopy round corrections must not exceed two".into(),
            ));
        }
        Ok(Self {
            lambda_end,
            theta,
            q,
            damping,
            corrections,
        })
    }

    pub fn lambda_end(&self) -> f64 {
        self.lambda_end
    }

    pub fn theta(&self) -> f64 {
        self.theta
    }

    pub fn q(&self) -> usize {
        self.q
    }

    pub fn damping(&self) -> f64 {
        self.damping
    }

    pub fn corrections(&self) -> usize {
        self.corrections
    }
}

/// A validated bounded nonstationary schedule ending at the original RODAS target.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyScheduleConfig {
    rounds: Vec<HomotopyRoundSpec>,
    predictor: HomotopyPredictor,
}

impl HomotopyScheduleConfig {
    pub fn new(rounds: Vec<HomotopyRoundSpec>, predictor: HomotopyPredictor) -> CoreResult<Self> {
        if rounds.is_empty() {
            return Err(CoreError::InvalidInput(
                "homotopy schedule needs at least one round".into(),
            ));
        }
        if rounds.len() > 4 {
            return Err(CoreError::InvalidInput(
                "homotopy research schedule must not exceed four rounds".into(),
            ));
        }
        let mut previous = 0.0;
        for round in &rounds {
            if round.lambda_end <= previous {
                return Err(CoreError::InvalidInput(
                    "homotopy schedule lambda endpoints must be strictly increasing".into(),
                ));
            }
            previous = round.lambda_end;
        }
        let endpoint_tolerance = 64.0 * f64::EPSILON;
        if (previous - 1.0).abs() > endpoint_tolerance {
            return Err(CoreError::InvalidInput(
                "homotopy schedule must end at lambda=1".into(),
            ));
        }
        Ok(Self { rounds, predictor })
    }

    pub fn rounds(&self) -> &[HomotopyRoundSpec] {
        &self.rounds
    }

    pub fn predictor(&self) -> HomotopyPredictor {
        self.predictor
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScheduledHomotopyRoundPoint {
    pub round: usize,
    pub lambda_start: f64,
    pub lambda_end: f64,
    pub theta: f64,
    pub eta_start: f64,
    pub eta_end: f64,
    pub q: usize,
    pub damping: f64,
    pub corrections: usize,
    pub homotopy_residual_before: f64,
    pub homotopy_residual_after: f64,
    pub homotopy_residual_ratio: f64,
    pub target_residual_before: f64,
    pub target_residual_after: f64,
    pub target_residual_ratio: f64,
    pub predictor_step_norm: f64,
    pub ab2_history_used: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScheduledHomotopyPathReport {
    pub schedule: HomotopyScheduleConfig,
    pub stages: Vec<Vec<f64>>,
    pub points: Vec<ScheduledHomotopyRoundPoint>,
    pub work: HomotopyWorkLedger,
    pub completed: bool,
    pub failure: Option<String>,
    pub last_lambda: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyStepConfig {
    path: HomotopyPathConfig,
    output_policy: OutputBudgetPolicy,
}

impl HomotopyStepConfig {
    pub fn new(path: HomotopyPathConfig, output_wrms_budget: f64) -> CoreResult<Self> {
        Self::with_policy(path, OutputBudgetPolicy::absolute(output_wrms_budget)?)
    }

    pub fn with_policy(
        path: HomotopyPathConfig,
        output_policy: OutputBudgetPolicy,
    ) -> CoreResult<Self> {
        // Evaluate once at a benign point so deserialized or future variants cannot defer
        // a malformed parameter set until a speculative step is already in progress.
        output_policy.budget(0.0, 1.0)?;
        Ok(Self {
            path,
            output_policy,
        })
    }

    pub fn path(&self) -> &HomotopyPathConfig {
        &self.path
    }

    pub fn output_policy(&self) -> &OutputBudgetPolicy {
        &self.output_policy
    }

    pub fn output_wrms_budget(&self) -> Option<f64> {
        match self.output_policy {
            OutputBudgetPolicy::Absolute { epsilon } => Some(epsilon),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyPathPoint {
    pub round: usize,
    pub lambda: f64,
    pub eta: f64,
    pub homotopy_residual_norm: f64,
    pub target_residual_norm: f64,
    pub predictor_step_norm: f64,
    pub corrections: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct HomotopyWorkLedger {
    pub path_rounds: usize,
    pub correction_rounds: usize,
    pub tangent_evaluations: u64,
    pub nonlinear_snapshots: u64,
    pub coupling_actions: u64,
    pub w_factorizations: u64,
    pub w_solve_batches: u64,
    pub w_solve_vectors: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyPathReport {
    pub config: HomotopyPathConfig,
    pub stages: Vec<Vec<f64>>,
    pub points: Vec<HomotopyPathPoint>,
    pub work: HomotopyWorkLedger,
}

#[derive(Clone, Debug)]
pub struct HomotopyStepReport {
    pub step: StepResult,
    pub path: Option<HomotopyPathReport>,
    pub output_certificate: Option<NonlinearOutputCertificate>,
    pub fast_accepted: bool,
    pub fallback_reason: Option<String>,
}

pub(crate) struct HomotopyBatchEvent<'a> {
    pub phase: &'a str,
    pub round: usize,
    pub propagation_level: usize,
    pub iteration: Option<usize>,
    pub raw: &'a [Vec<f64>],
    pub transformed: &'a [Vec<f64>],
}

pub(crate) trait HomotopyBatchObserver {
    fn observe(&mut self, event: HomotopyBatchEvent<'_>) -> CoreResult<()>;
}

pub(crate) struct PathEvaluation {
    pub(crate) homotopy_residual: Vec<Vec<f64>>,
    pub(crate) target_residual_norm: f64,
    pub(crate) target_rhs_norm: f64,
    pub(crate) tangent_rhs: Vec<Vec<f64>>,
}

fn solve_common_w_rows(
    factor: &LuFactorization,
    rhs: &[Vec<f64>],
    dimension: usize,
    counters: &mut WorkCounters,
    work: &mut HomotopyWorkLedger,
) -> CoreResult<Vec<Vec<f64>>> {
    if rhs.iter().any(|row| row.len() != dimension) {
        return Err(CoreError::Dimension(
            "common-W batched RHS shape mismatch".into(),
        ));
    }
    let solved = factor.solve_rows(rhs)?;
    counters.direct_solve_calls += 1;
    work.w_solve_batches += 1;
    work.w_solve_vectors += rhs.len() as u64;
    Ok(solved)
}

#[allow(clippy::too_many_arguments)]
fn truncated_partial_inverse_rows(
    block: &StructuredBlockSystem<'_, '_>,
    factor: &LuFactorization,
    eta: f64,
    q: usize,
    rhs: &[Vec<f64>],
    counters: &mut WorkCounters,
    work: &mut HomotopyWorkLedger,
    observer: &mut Option<&mut dyn HomotopyBatchObserver>,
    phase: &str,
    round: usize,
    iteration: Option<usize>,
) -> CoreResult<Vec<Vec<f64>>> {
    if !eta.is_finite() {
        return Err(CoreError::NonFinite(
            "partial inverse eta contains NaN/Inf".into(),
        ));
    }
    if q >= block.s {
        return Err(CoreError::InvalidInput(format!(
            "truncated inverse depth q={q} must be less than {} stages",
            block.s
        )));
    }
    let mut term = solve_common_w_rows(factor, rhs, block.n, counters, work)?;
    if let Some(observer) = observer.as_deref_mut() {
        observer.observe(HomotopyBatchEvent {
            phase,
            round,
            propagation_level: 0,
            iteration,
            raw: rhs,
            transformed: &term,
        })?;
    }
    let mut total = term.clone();
    if eta == 0.0 {
        return Ok(total);
    }
    for level in 1..=q {
        let mut coupled = block.coupling_apply(&term, counters)?;
        work.coupling_actions += 1;
        for row in &mut coupled {
            for value in row {
                *value *= eta;
            }
        }
        term = solve_common_w_rows(factor, &coupled, block.n, counters, work)?;
        if let Some(observer) = observer.as_deref_mut() {
            observer.observe(HomotopyBatchEvent {
                phase,
                round,
                propagation_level: level,
                iteration,
                raw: &coupled,
                transformed: &term,
            })?;
        }
        for (sum_row, term_row) in total.iter_mut().zip(&term) {
            for (sum, value) in sum_row.iter_mut().zip(term_row) {
                *sum += value;
            }
        }
    }
    if total.iter().flatten().all(|value| value.is_finite()) {
        Ok(total)
    } else {
        Err(CoreError::NonFinite(
            "truncated partial inverse produced NaN/Inf".into(),
        ))
    }
}

pub(crate) fn evaluate_partial_path(
    block: &StructuredBlockSystem<'_, '_>,
    stages: &[Vec<f64>],
    theta: f64,
    lambda: f64,
    counters: &mut WorkCounters,
    work: &mut HomotopyWorkLedger,
) -> CoreResult<PathEvaluation> {
    let parameters = PartialCouplingParameters::new(theta, lambda)?;
    let snapshot = block.nonlinear_remainder_snapshot(stages, counters)?;
    work.nonlinear_snapshots += 1;
    let partial = block.partial_linear_apply(stages, parameters.eta(), counters)?;
    let base = block.rhs_base();
    let mut homotopy_residual = vec![vec![0.0; block.n]; block.s];
    for stage in 0..block.s {
        for component in 0..block.n {
            homotopy_residual[stage][component] = partial[stage][component]
                - base[stage][component]
                - lambda * block.context.h * snapshot.remainder[stage][component];
        }
    }

    let target_applied = block.apply(stages, counters)?;
    let target_rhs_norm = safe_l2(&flatten(&snapshot.rhs));
    let target_residual: Vec<Vec<f64>> = target_applied
        .iter()
        .zip(&snapshot.rhs)
        .map(|(lhs, rhs)| lhs.iter().zip(rhs).map(|(a, b)| a - b).collect())
        .collect();

    let mut tangent_rhs = block.coupling_apply(stages, counters)?;
    work.coupling_actions += 1;
    for (tangent_row, remainder_row) in tangent_rhs.iter_mut().zip(&snapshot.remainder) {
        for (tangent, remainder) in tangent_row.iter_mut().zip(remainder_row) {
            *tangent = (1.0 - theta) * *tangent + block.context.h * *remainder;
        }
    }
    work.tangent_evaluations += 1;

    Ok(PathEvaluation {
        homotopy_residual,
        target_residual_norm: safe_l2(&flatten(&target_residual)),
        target_rhs_norm,
        tangent_rhs,
    })
}

pub(crate) fn add_scaled_rows(
    base: &[Vec<f64>],
    first_scale: f64,
    first: &[Vec<f64>],
    second_scale: f64,
    second: Option<&[Vec<f64>]>,
) -> CoreResult<Vec<Vec<f64>>> {
    if base.len() != first.len()
        || base.iter().zip(first).any(|(a, b)| a.len() != b.len())
        || second.is_some_and(|rows| {
            rows.len() != base.len() || base.iter().zip(rows).any(|(a, b)| a.len() != b.len())
        })
    {
        return Err(CoreError::Dimension(
            "homotopy row combination shape mismatch".into(),
        ));
    }
    let mut out = base.to_vec();
    for stage in 0..out.len() {
        for component in 0..out[stage].len() {
            out[stage][component] += first_scale * first[stage][component];
            if let Some(rows) = second {
                out[stage][component] += second_scale * rows[stage][component];
            }
        }
    }
    if out.iter().flatten().all(|value| value.is_finite()) {
        Ok(out)
    } else {
        Err(CoreError::NonFinite(
            "homotopy row update produced NaN/Inf".into(),
        ))
    }
}

fn run_fixed_homotopy_path_internal(
    block: &StructuredBlockSystem<'_, '_>,
    config: &HomotopyPathConfig,
    counters: &mut WorkCounters,
    observer: &mut Option<&mut dyn HomotopyBatchObserver>,
) -> CoreResult<HomotopyPathReport> {
    if block.s != 8 {
        return Err(CoreError::InvalidInput(format!(
            "fixed homotopy path currently requires the 8-stage RODAS5P system, got {}",
            block.s
        )));
    }
    let shifted = block.context.shifted.explicit().ok_or_else(|| {
        CoreError::LinearSolve("fixed homotopy path needs explicit common W".into())
    })?;
    counters.direct_factorizations += 1;
    let factor = LuFactorization::new(shifted)?;
    let mut work = HomotopyWorkLedger {
        path_rounds: config.path_rounds,
        w_factorizations: 1,
        ..HomotopyWorkLedger::default()
    };

    let mut stages = truncated_partial_inverse_rows(
        block,
        &factor,
        config.theta,
        config.q,
        &block.rhs_base(),
        counters,
        &mut work,
        observer,
        "path-start",
        0,
        None,
    )?;
    let mut current =
        evaluate_partial_path(block, &stages, config.theta, 0.0, counters, &mut work)?;
    let mut points = vec![HomotopyPathPoint {
        round: 0,
        lambda: 0.0,
        eta: config.theta,
        homotopy_residual_norm: safe_l2(&flatten(&current.homotopy_residual)),
        target_residual_norm: current.target_residual_norm,
        predictor_step_norm: 0.0,
        corrections: 0,
    }];
    let delta_lambda = 1.0 / config.path_rounds as f64;
    let mut previous_tangent: Option<Vec<Vec<f64>>> = None;

    for round in 0..config.path_rounds {
        let lambda = round as f64 * delta_lambda;
        let parameters = PartialCouplingParameters::new(config.theta, lambda)?;
        let tangent = truncated_partial_inverse_rows(
            block,
            &factor,
            parameters.eta(),
            config.q,
            &current.tangent_rhs,
            counters,
            &mut work,
            observer,
            "tangent",
            round,
            None,
        )?;
        let predictor_increment = match (config.predictor, previous_tangent.as_deref()) {
            (HomotopyPredictor::AdamsBashforth2, Some(previous)) => add_scaled_rows(
                &vec![vec![0.0; block.n]; block.s],
                1.5 * delta_lambda,
                &tangent,
                -0.5 * delta_lambda,
                Some(previous),
            )?,
            _ => add_scaled_rows(
                &vec![vec![0.0; block.n]; block.s],
                delta_lambda,
                &tangent,
                0.0,
                None,
            )?,
        };
        let predictor_step_norm = safe_l2(&flatten(&predictor_increment));
        let mut candidate = add_scaled_rows(&stages, 1.0, &predictor_increment, 0.0, None)?;
        let next_lambda = (round + 1) as f64 * delta_lambda;
        let next_eta = config.theta + next_lambda * (1.0 - config.theta);

        for correction_index in 0..config.corrections_per_point {
            let evaluation = evaluate_partial_path(
                block,
                &candidate,
                config.theta,
                next_lambda,
                counters,
                &mut work,
            )?;
            let correction = truncated_partial_inverse_rows(
                block,
                &factor,
                next_eta,
                config.q,
                &evaluation.homotopy_residual,
                counters,
                &mut work,
                observer,
                "correction",
                round + 1,
                Some(correction_index + 1),
            )?;
            candidate = add_scaled_rows(&candidate, -1.0, &correction, 0.0, None)?;
            work.correction_rounds += 1;
        }

        let next = evaluate_partial_path(
            block,
            &candidate,
            config.theta,
            next_lambda,
            counters,
            &mut work,
        )?;
        points.push(HomotopyPathPoint {
            round: round + 1,
            lambda: next_lambda,
            eta: next_eta,
            homotopy_residual_norm: safe_l2(&flatten(&next.homotopy_residual)),
            target_residual_norm: next.target_residual_norm,
            predictor_step_norm,
            corrections: config.corrections_per_point,
        });
        previous_tangent = Some(tangent);
        stages = candidate;
        current = next;
    }

    Ok(HomotopyPathReport {
        config: config.clone(),
        stages,
        points,
        work,
    })
}

fn scheduled_failure_report(
    schedule: &HomotopyScheduleConfig,
    stages: Vec<Vec<f64>>,
    points: Vec<ScheduledHomotopyRoundPoint>,
    work: HomotopyWorkLedger,
    last_lambda: f64,
    error: &CoreError,
) -> ScheduledHomotopyPathReport {
    ScheduledHomotopyPathReport {
        schedule: schedule.clone(),
        stages,
        points,
        work,
        completed: false,
        failure: Some(error.to_string()),
        last_lambda,
    }
}

/// Execute a bounded nonstationary partial-coupling schedule.
///
/// This is a correctness/reference engine: it deliberately uses one explicit common-`W`
/// factorization so the nonlinear path-controller behavior can be isolated from the linear
/// backend.  A changing `theta` is a nonstationary sweep and is not advertised as one smooth
/// classical continuation curve.
pub fn run_scheduled_homotopy_path(
    block: &StructuredBlockSystem<'_, '_>,
    schedule: &HomotopyScheduleConfig,
    counters: &mut WorkCounters,
) -> CoreResult<ScheduledHomotopyPathReport> {
    if block.s != 8 {
        return Err(CoreError::InvalidInput(format!(
            "scheduled homotopy path currently requires the 8-stage RODAS5P system, got {}",
            block.s
        )));
    }
    let shifted = block.context.shifted.explicit().ok_or_else(|| {
        CoreError::LinearSolve("scheduled homotopy reference needs explicit common W".into())
    })?;
    counters.direct_factorizations += 1;
    let factor = match LuFactorization::new(shifted) {
        Ok(factor) => factor,
        Err(error) => {
            return Ok(scheduled_failure_report(
                schedule,
                vec![vec![0.0; block.n]; block.s],
                Vec::new(),
                HomotopyWorkLedger {
                    path_rounds: schedule.rounds.len(),
                    w_factorizations: 1,
                    ..HomotopyWorkLedger::default()
                },
                0.0,
                &error,
            ));
        }
    };
    let mut work = HomotopyWorkLedger {
        path_rounds: schedule.rounds.len(),
        w_factorizations: 1,
        ..HomotopyWorkLedger::default()
    };
    let mut observer: Option<&mut dyn HomotopyBatchObserver> = None;
    let first = &schedule.rounds[0];
    let mut stages = match truncated_partial_inverse_rows(
        block,
        &factor,
        first.theta,
        first.q,
        &block.rhs_base(),
        counters,
        &mut work,
        &mut observer,
        "scheduled-path-start",
        0,
        None,
    ) {
        Ok(stages) => stages,
        Err(error) => {
            return Ok(scheduled_failure_report(
                schedule,
                vec![vec![0.0; block.n]; block.s],
                Vec::new(),
                work,
                0.0,
                &error,
            ));
        }
    };
    let mut current =
        match evaluate_partial_path(block, &stages, first.theta, 0.0, counters, &mut work) {
            Ok(evaluation) => evaluation,
            Err(error) => {
                return Ok(scheduled_failure_report(
                    schedule,
                    stages,
                    Vec::new(),
                    work,
                    0.0,
                    &error,
                ));
            }
        };
    let initial_homotopy = safe_l2(&flatten(&current.homotopy_residual));
    let mut points = vec![ScheduledHomotopyRoundPoint {
        round: 0,
        lambda_start: 0.0,
        lambda_end: 0.0,
        theta: first.theta,
        eta_start: first.theta,
        eta_end: first.theta,
        q: first.q,
        damping: 1.0,
        corrections: 0,
        homotopy_residual_before: initial_homotopy,
        homotopy_residual_after: initial_homotopy,
        homotopy_residual_ratio: 1.0,
        target_residual_before: current.target_residual_norm,
        target_residual_after: current.target_residual_norm,
        target_residual_ratio: 1.0,
        predictor_step_norm: 0.0,
        ab2_history_used: false,
    }];

    let mut lambda = 0.0;
    let mut previous_tangent: Option<Vec<Vec<f64>>> = None;
    let mut previous_signature: Option<(u64, usize)> = None;
    let mut active_theta = first.theta;

    for (round_index, round) in schedule.rounds.iter().enumerate() {
        if round.theta.to_bits() != active_theta.to_bits() {
            current = match evaluate_partial_path(
                block,
                &stages,
                round.theta,
                lambda,
                counters,
                &mut work,
            ) {
                Ok(evaluation) => evaluation,
                Err(error) => {
                    return Ok(scheduled_failure_report(
                        schedule, stages, points, work, lambda, &error,
                    ));
                }
            };
            active_theta = round.theta;
        }
        let homotopy_before = safe_l2(&flatten(&current.homotopy_residual));
        let target_before = current.target_residual_norm;
        let eta_start = round.theta + lambda * (1.0 - round.theta);
        let eta_end = round.theta + round.lambda_end * (1.0 - round.theta);
        let tangent = match truncated_partial_inverse_rows(
            block,
            &factor,
            eta_start,
            round.q,
            &current.tangent_rhs,
            counters,
            &mut work,
            &mut observer,
            "scheduled-tangent",
            round_index,
            None,
        ) {
            Ok(tangent) => tangent,
            Err(error) => {
                return Ok(scheduled_failure_report(
                    schedule, stages, points, work, lambda, &error,
                ));
            }
        };
        let signature = (round.theta.to_bits(), round.q);
        let ab2_history_used = schedule.predictor == HomotopyPredictor::AdamsBashforth2
            && previous_signature == Some(signature)
            && previous_tangent.is_some();
        let delta_lambda = round.lambda_end - lambda;
        let predictor_increment = if ab2_history_used {
            add_scaled_rows(
                &vec![vec![0.0; block.n]; block.s],
                1.5 * delta_lambda,
                &tangent,
                -0.5 * delta_lambda,
                previous_tangent.as_deref(),
            )
        } else {
            add_scaled_rows(
                &vec![vec![0.0; block.n]; block.s],
                delta_lambda,
                &tangent,
                0.0,
                None,
            )
        };
        let mut predictor_increment = match predictor_increment {
            Ok(increment) => increment,
            Err(error) => {
                return Ok(scheduled_failure_report(
                    schedule, stages, points, work, lambda, &error,
                ));
            }
        };
        for value in predictor_increment.iter_mut().flatten() {
            *value *= round.damping;
        }
        let predictor_step_norm = safe_l2(&flatten(&predictor_increment));
        let mut candidate = match add_scaled_rows(&stages, 1.0, &predictor_increment, 0.0, None) {
            Ok(candidate) => candidate,
            Err(error) => {
                return Ok(scheduled_failure_report(
                    schedule, stages, points, work, lambda, &error,
                ));
            }
        };

        for correction_index in 0..round.corrections {
            let evaluation = match evaluate_partial_path(
                block,
                &candidate,
                round.theta,
                round.lambda_end,
                counters,
                &mut work,
            ) {
                Ok(evaluation) => evaluation,
                Err(error) => {
                    return Ok(scheduled_failure_report(
                        schedule, candidate, points, work, lambda, &error,
                    ));
                }
            };
            let correction = match truncated_partial_inverse_rows(
                block,
                &factor,
                eta_end,
                round.q,
                &evaluation.homotopy_residual,
                counters,
                &mut work,
                &mut observer,
                "scheduled-correction",
                round_index + 1,
                Some(correction_index + 1),
            ) {
                Ok(correction) => correction,
                Err(error) => {
                    return Ok(scheduled_failure_report(
                        schedule, candidate, points, work, lambda, &error,
                    ));
                }
            };
            candidate = match add_scaled_rows(&candidate, -1.0, &correction, 0.0, None) {
                Ok(updated) => updated,
                Err(error) => {
                    return Ok(scheduled_failure_report(
                        schedule, candidate, points, work, lambda, &error,
                    ));
                }
            };
            work.correction_rounds += 1;
        }

        let next = match evaluate_partial_path(
            block,
            &candidate,
            round.theta,
            round.lambda_end,
            counters,
            &mut work,
        ) {
            Ok(evaluation) => evaluation,
            Err(error) => {
                return Ok(scheduled_failure_report(
                    schedule, candidate, points, work, lambda, &error,
                ));
            }
        };
        let homotopy_after = safe_l2(&flatten(&next.homotopy_residual));
        let target_after = next.target_residual_norm;
        points.push(ScheduledHomotopyRoundPoint {
            round: round_index + 1,
            lambda_start: lambda,
            lambda_end: round.lambda_end,
            theta: round.theta,
            eta_start,
            eta_end,
            q: round.q,
            damping: round.damping,
            corrections: round.corrections,
            homotopy_residual_before: homotopy_before,
            homotopy_residual_after: homotopy_after,
            homotopy_residual_ratio: homotopy_after / homotopy_before.max(f64::MIN_POSITIVE),
            target_residual_before: target_before,
            target_residual_after: target_after,
            target_residual_ratio: target_after / target_before.max(f64::MIN_POSITIVE),
            predictor_step_norm,
            ab2_history_used,
        });
        previous_tangent = Some(tangent);
        previous_signature = Some(signature);
        stages = candidate;
        current = next;
        lambda = round.lambda_end;
    }

    Ok(ScheduledHomotopyPathReport {
        schedule: schedule.clone(),
        stages,
        points,
        work,
        completed: true,
        failure: None,
        last_lambda: lambda,
    })
}

pub fn run_fixed_homotopy_path(
    block: &StructuredBlockSystem<'_, '_>,
    config: &HomotopyPathConfig,
    counters: &mut WorkCounters,
) -> CoreResult<HomotopyPathReport> {
    let mut observer = None;
    run_fixed_homotopy_path_internal(block, config, counters, &mut observer)
}

pub(crate) fn run_fixed_homotopy_path_observed(
    block: &StructuredBlockSystem<'_, '_>,
    config: &HomotopyPathConfig,
    counters: &mut WorkCounters,
    observer: &mut dyn HomotopyBatchObserver,
) -> CoreResult<HomotopyPathReport> {
    let mut observer = Some(observer);
    run_fixed_homotopy_path_internal(block, config, counters, &mut observer)
}

#[allow(clippy::too_many_arguments)]
pub fn homotopy_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &HomotopyStepConfig,
    fallback_config: Option<&LinearSolverConfig>,
    mut recycle: Option<&mut KrylovState>,
    atol: f64,
    rtol: f64,
    force_accept: bool,
    counters: &mut WorkCounters,
) -> CoreResult<HomotopyStepReport> {
    let before = *counters;
    let recycle_snapshot = recycle.as_deref().cloned();
    let context = build_step_context(problem, t, y, h, counters)?;
    let fallback = fallback_config
        .cloned()
        .unwrap_or_else(|| LinearSolverConfig {
            method: if context.shifted.explicit().is_some() {
                LinearMethod::Direct
            } else {
                LinearMethod::Gmres
            },
            preconditioner: PreconditionerKind::None,
            ..LinearSolverConfig::default()
        });

    counters.fast_attempts += 1;
    let block = StructuredBlockSystem::new(&context);
    let mut path_report: Option<HomotopyPathReport> = None;
    let mut output_certificate: Option<NonlinearOutputCertificate> = None;
    let fallback_reason: Option<String>;

    match run_fixed_homotopy_path(&block, config.path(), counters) {
        Ok(path) => {
            match certify_nonlinear_target(&block, &path.stages, atol, rtol, counters) {
                Ok(certificate) => {
                    let policy_decision = config.output_policy().decide(
                        certificate.output_wrms,
                        certificate.embedded_error,
                        h,
                    )?;
                    let defect_ok = policy_decision.accepted;
                    let step_ok = force_accept || certificate.combined_error <= 1.0;
                    let finite = path.stages.iter().flatten().all(|value| value.is_finite());
                    if defect_ok && step_ok && finite {
                        let step_certificate = StepCertificate {
                            accepted: true,
                            reason: "original RODAS output certificate passed".into(),
                            iterations: path.work.path_rounds + path.work.correction_rounds,
                            embedded_error: certificate.embedded_error,
                            fixed_point_error: certificate.output_wrms,
                            residual_proxy_error: certificate.output_wrms,
                            contraction_tail_error: None,
                            observed_contraction: None,
                            stage_residual_norm: certificate.stage_residual_norm,
                            stage_relative_residual: certificate.relative_stage_residual,
                        };
                        let mut step = finish_step(
                            &context,
                            path.stages.clone(),
                            atol,
                            rtol,
                            format!(
                                "RODAS5P-homotopy-theta{:.3}-q{}-{:?}",
                                config.path.theta, config.path.q, config.path.predictor
                            ),
                            Some(true),
                            false,
                            Some(step_certificate),
                            before,
                            counters,
                        )?;
                        if step.accepted {
                            counters.fast_accepts += 1;
                            counters.accepted_steps += 1;
                            step.counters = counters.delta(before);
                            return Ok(HomotopyStepReport {
                                step,
                                path: Some(path),
                                output_certificate: Some(certificate),
                                fast_accepted: true,
                                fallback_reason: None,
                            });
                        }
                        fallback_reason =
                            Some("certificate passed but final step was non-finite".into());
                    } else {
                        fallback_reason = Some(format!(
                            "certificate failed: output_wrms={:.6e}, combined={:.6e}, budget={:.6e}, policy={}",
                            certificate.output_wrms,
                            certificate.combined_error,
                            policy_decision.budget,
                            config.output_policy().id()
                        ));
                    }
                    output_certificate = Some(certificate);
                }
                Err(error) => {
                    fallback_reason = Some(format!("certificate failure: {error}"));
                }
            }
            path_report = Some(path);
        }
        Err(error) => {
            fallback_reason = Some(format!("homotopy path failure: {error}"));
        }
    }

    counters.fallback_steps += 1;
    let data = match sequential_stages(&context, &fallback, recycle.as_deref_mut(), counters) {
        Ok(data) => data,
        Err(error) => {
            if let (Some(target), Some(snapshot)) = (recycle, recycle_snapshot) {
                *target = snapshot;
            }
            return Err(error);
        }
    };
    let reason = fallback_reason
        .clone()
        .unwrap_or_else(|| "homotopy fast path not accepted".into());
    let mut step = finish_step(
        &context,
        data.stages,
        atol,
        rtol,
        format!("RODAS5P-homotopy-fallback-{:?}", fallback.method),
        if force_accept { Some(true) } else { None },
        true,
        Some(StepCertificate {
            reason,
            ..StepCertificate::default()
        }),
        before,
        counters,
    )?;
    if step.accepted {
        counters.accepted_steps += 1;
    } else {
        counters.rejected_steps += 1;
        if let (Some(target), Some(snapshot)) = (recycle, recycle_snapshot) {
            *target = snapshot;
        }
    }
    step.counters = counters.delta(before);
    Ok(HomotopyStepReport {
        step,
        path: path_report,
        output_certificate,
        fast_accepted: false,
        fallback_reason,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TruncationScreenRow {
    pub q: usize,
    pub error_norm: f64,
    pub relative_error: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PowerNormRow {
    pub power: usize,
    pub frobenius_norm: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NonnormalConditionRow {
    pub coupling: f64,
    pub operator_one_norm: f64,
    pub inverse_one_norm: f64,
    pub condition_one: f64,
    pub determinant: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyDesignCheckReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub stages: usize,
    pub dimension: usize,
    pub affine_endpoint_max_abs_error: f64,
    pub flrh_lambda_spread: f64,
    pub truncation_screen: Vec<TruncationScreenRow>,
    pub official_l_power_norms: Vec<PowerNormRow>,
    pub nonnormal_condition_screen: Vec<NonnormalConditionRow>,
    pub perturbed_endpoint_certificate: AffineOutputCertificate,
}

#[derive(Clone, Debug)]
pub struct AffinePartialCouplingOracle {
    stages: usize,
    dimension: usize,
    d: DenseMatrix,
    c: DenseMatrix,
    target: DenseMatrix,
    target_factor: LuFactorization,
    w_factor: LuFactorization,
    rhs: Vec<f64>,
    weights: Vec<f64>,
}

impl AffinePartialCouplingOracle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mass: DenseMatrix,
        jacobian: DenseMatrix,
        beta: DenseMatrix,
        gamma: f64,
        h: f64,
        rhs_rows: Vec<Vec<f64>>,
        weights: Vec<f64>,
    ) -> CoreResult<Self> {
        if mass.nrows() == 0 || mass.nrows() != mass.ncols() {
            return Err(CoreError::Dimension(
                "mass matrix must be nonempty and square".into(),
            ));
        }
        let dimension = mass.nrows();
        if jacobian.nrows() != dimension || jacobian.ncols() != dimension {
            return Err(CoreError::Dimension(
                "Jacobian and mass matrix shapes differ".into(),
            ));
        }
        if beta.nrows() == 0 || beta.nrows() != beta.ncols() {
            return Err(CoreError::Dimension(
                "stage coupling matrix must be nonempty and square".into(),
            ));
        }
        let stages = beta.nrows();
        if rhs_rows.len() != stages || rhs_rows.iter().any(|row| row.len() != dimension) {
            return Err(CoreError::Dimension(
                "affine stage RHS shape mismatch".into(),
            ));
        }
        if weights.len() != stages {
            return Err(CoreError::Dimension(
                "stage output weights shape mismatch".into(),
            ));
        }
        if !gamma.is_finite() || !h.is_finite() {
            return Err(CoreError::NonFinite(
                "gamma and step size must be finite".into(),
            ));
        }
        if !weights.iter().all(|value| value.is_finite())
            || !rhs_rows.iter().flatten().all(|value| value.is_finite())
        {
            return Err(CoreError::NonFinite(
                "affine homotopy data contain NaN/Inf".into(),
            ));
        }

        let scale = beta
            .as_slice()
            .iter()
            .fold(1.0_f64, |acc, value| acc.max(value.abs()));
        let structural_tolerance = 512.0 * f64::EPSILON * scale;
        for i in 0..stages {
            for j in i..stages {
                let expected = if i == j { gamma } else { 0.0 };
                if (beta[(i, j)] - expected).abs() > structural_tolerance {
                    return Err(CoreError::InvalidInput(
                        "beta must equal gamma I plus a strictly lower matrix".into(),
                    ));
                }
            }
        }

        let w = mass.combine(&jacobian, -h * gamma)?;
        // Check the common diagonal block before constructing the larger systems.
        let w_factor = LuFactorization::new(&w)?;
        for column in 0..dimension {
            let mut unit = vec![0.0; dimension];
            unit[column] = 1.0;
            w_factor.solve(&unit)?;
        }

        let l = beta.sub(&DenseMatrix::identity(stages).scale(gamma))?;
        let full_dimension = stages * dimension;
        let mut d = DenseMatrix::zeros(full_dimension, full_dimension);
        let mut c = DenseMatrix::zeros(full_dimension, full_dimension);
        for si in 0..stages {
            for sj in 0..stages {
                for i in 0..dimension {
                    for j in 0..dimension {
                        if si == sj {
                            d[(si * dimension + i, sj * dimension + j)] = w[(i, j)];
                        }
                        c[(si * dimension + i, sj * dimension + j)] =
                            h * l[(si, sj)] * jacobian[(i, j)];
                    }
                }
            }
        }
        let target = d.sub(&c)?;
        let target_factor = LuFactorization::new(&target)?;
        let rhs = rhs_rows.into_iter().flatten().collect();

        Ok(Self {
            stages,
            dimension,
            d,
            c,
            target,
            target_factor,
            w_factor,
            rhs,
            weights,
        })
    }

    pub fn stages(&self) -> usize {
        self.stages
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    fn validate_full_vector(&self, vector: &[f64], label: &str) -> CoreResult<()> {
        if vector.len() != self.stages * self.dimension {
            return Err(CoreError::Dimension(format!(
                "{label} shape mismatch: expected {}, got {}",
                self.stages * self.dimension,
                vector.len()
            )));
        }
        if !vector.iter().all(|value| value.is_finite()) {
            return Err(CoreError::NonFinite(format!("{label} contains NaN/Inf")));
        }
        Ok(())
    }

    fn d_inverse_apply(&self, rhs: &[f64]) -> CoreResult<Vec<f64>> {
        self.validate_full_vector(rhs, "block-diagonal inverse RHS")?;
        let mut out = vec![0.0; rhs.len()];
        for stage in 0..self.stages {
            let start = stage * self.dimension;
            let end = start + self.dimension;
            let solved = self.w_factor.solve(&rhs[start..end])?;
            out[start..end].copy_from_slice(&solved);
        }
        Ok(out)
    }

    pub fn normalized_coupling_apply(
        &self,
        parameters: PartialCouplingParameters,
        vector: &[f64],
    ) -> CoreResult<Vec<f64>> {
        self.validate_full_vector(vector, "normalized-coupling vector")?;
        let eta = parameters.eta();
        if eta == 0.0 {
            return Ok(vec![0.0; vector.len()]);
        }
        let coupled = self.c.matvec(vector)?;
        Ok(self
            .d_inverse_apply(&coupled)?
            .into_iter()
            .map(|value| eta * value)
            .collect())
    }

    pub fn truncated_inverse_apply(
        &self,
        parameters: PartialCouplingParameters,
        q: usize,
        rhs: &[f64],
    ) -> CoreResult<Vec<f64>> {
        if q >= self.stages {
            return Err(CoreError::InvalidInput(format!(
                "nilpotent truncation depth q={q} must be less than stage count {}",
                self.stages
            )));
        }
        let mut term = self.d_inverse_apply(rhs)?;
        let mut total = term.clone();
        for _ in 0..q {
            term = self.normalized_coupling_apply(parameters, &term)?;
            for (sum, value) in total.iter_mut().zip(&term) {
                *sum += value;
            }
        }
        if total.iter().all(|value| value.is_finite()) {
            Ok(total)
        } else {
            Err(CoreError::NonFinite(
                "truncated nilpotent inverse produced NaN/Inf".into(),
            ))
        }
    }

    pub fn path_operator(&self, parameters: PartialCouplingParameters) -> CoreResult<DenseMatrix> {
        self.d.combine(&self.c, -parameters.eta())
    }

    pub fn solve_path(&self, parameters: PartialCouplingParameters) -> CoreResult<Vec<f64>> {
        LuFactorization::new(&self.path_operator(parameters)?)?.solve(&self.rhs)
    }

    pub fn homotopy_residual(
        &self,
        parameters: PartialCouplingParameters,
        stages: &[f64],
    ) -> CoreResult<Vec<f64>> {
        if stages.len() != self.stages * self.dimension {
            return Err(CoreError::Dimension(
                "homotopy stage vector shape mismatch".into(),
            ));
        }
        Ok(self
            .path_operator(parameters)?
            .matvec(stages)?
            .into_iter()
            .zip(&self.rhs)
            .map(|(lhs, rhs)| lhs - rhs)
            .collect())
    }

    pub fn target_residual(&self, stages: &[f64]) -> CoreResult<Vec<f64>> {
        if stages.len() != self.stages * self.dimension {
            return Err(CoreError::Dimension(
                "target stage vector shape mismatch".into(),
            ));
        }
        Ok(self
            .target
            .matvec(stages)?
            .into_iter()
            .zip(&self.rhs)
            .map(|(lhs, rhs)| lhs - rhs)
            .collect())
    }

    pub fn certify_target(
        &self,
        stages: &[f64],
        scale: &[f64],
    ) -> CoreResult<AffineOutputCertificate> {
        self.validate_full_vector(stages, "target certificate stage vector")?;
        if scale.len() != self.dimension {
            return Err(CoreError::Dimension(format!(
                "target certificate scale shape mismatch: expected {}, got {}",
                self.dimension,
                scale.len()
            )));
        }
        let residual = self.target_residual(stages)?;
        let correction = self.target_factor.solve(&residual)?;
        let mut output_error = vec![0.0; self.dimension];
        for stage in 0..self.stages {
            let weight = self.weights[stage];
            for component in 0..self.dimension {
                output_error[component] += weight * correction[stage * self.dimension + component];
            }
        }
        let residual_norm = safe_l2(&residual);
        let rhs_norm = safe_l2(&self.rhs);
        let certificate = AffineOutputCertificate {
            residual_norm,
            relative_residual: residual_norm / rhs_norm.max(f64::MIN_POSITIVE),
            output_wrms: wrms(&output_error, scale)?,
            correction_norm: safe_l2(&correction),
        };
        if [
            certificate.residual_norm,
            certificate.relative_residual,
            certificate.output_wrms,
            certificate.correction_norm,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            Ok(certificate)
        } else {
            Err(CoreError::NonFinite(
                "target certificate contains NaN/Inf".into(),
            ))
        }
    }

    pub fn output_weights(&self) -> &[f64] {
        &self.weights
    }
}

fn max_abs_difference(left: &[f64], right: &[f64]) -> CoreResult<f64> {
    if left.len() != right.len() {
        return Err(CoreError::Dimension(
            "maximum-difference vector shape mismatch".into(),
        ));
    }
    Ok(left
        .iter()
        .zip(right)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max))
}

fn frobenius_norm(matrix: &DenseMatrix) -> f64 {
    safe_l2(matrix.as_slice())
}

fn matrix_one_norm(matrix: &DenseMatrix) -> f64 {
    (0..matrix.ncols())
        .map(|column| {
            (0..matrix.nrows())
                .map(|row| matrix[(row, column)].abs())
                .sum::<f64>()
        })
        .fold(0.0, f64::max)
}

pub fn run_homotopy_design_check() -> CoreResult<HomotopyDesignCheckReport> {
    let coefficients = load_rodas5p_coefficients()?;
    let stages = coefficients.stages();
    let dimension = 2;
    let mass = DenseMatrix::from_rows(&[&[2.0, 0.3], &[-0.1, 1.5]])?;
    let jacobian = DenseMatrix::from_rows(&[&[-3.0, 2.0], &[0.5, -1.0]])?;
    let rhs_rows: Vec<Vec<f64>> = (0..stages)
        .map(|stage| {
            let x = stage as f64 + 1.0;
            vec![0.1 * x.sin(), 0.1 * x.cos()]
        })
        .collect();
    let flat_rhs: Vec<f64> = rhs_rows.iter().flatten().copied().collect();
    let oracle = AffinePartialCouplingOracle::new(
        mass,
        jacobian,
        coefficients.beta.clone(),
        coefficients.gamma,
        0.05,
        rhs_rows,
        coefficients.b.clone(),
    )?;

    let target = oracle.solve_path(PartialCouplingParameters::new(0.0, 1.0)?)?;
    let mut affine_endpoint_max_abs_error = 0.0_f64;
    for theta in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let endpoint = oracle.solve_path(PartialCouplingParameters::new(theta, 1.0)?)?;
        affine_endpoint_max_abs_error =
            affine_endpoint_max_abs_error.max(max_abs_difference(&endpoint, &target)?);
    }

    let flrh_reference = oracle.solve_path(PartialCouplingParameters::new(1.0, 0.0)?)?;
    let mut flrh_lambda_spread = 0.0_f64;
    for lambda in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let point = oracle.solve_path(PartialCouplingParameters::new(1.0, lambda)?)?;
        flrh_lambda_spread = flrh_lambda_spread.max(max_abs_difference(&point, &flrh_reference)?);
    }

    let screen_parameters = PartialCouplingParameters::new(0.4, 0.7)?;
    let exact =
        LuFactorization::new(&oracle.path_operator(screen_parameters)?)?.solve(&flat_rhs)?;
    let exact_norm = safe_l2(&exact).max(f64::MIN_POSITIVE);
    let mut truncation_screen = Vec::new();
    for q in [0, 1, 2, stages - 1] {
        let approximation = oracle.truncated_inverse_apply(screen_parameters, q, &flat_rhs)?;
        let error: Vec<f64> = exact
            .iter()
            .zip(&approximation)
            .map(|(a, b)| a - b)
            .collect();
        let error_norm = safe_l2(&error);
        truncation_screen.push(TruncationScreenRow {
            q,
            error_norm,
            relative_error: error_norm / exact_norm,
        });
    }

    let mut official_l_power_norms = Vec::with_capacity(stages);
    let mut power = DenseMatrix::identity(stages);
    for exponent in 1..=stages {
        power = power.matmul(&coefficients.l)?;
        official_l_power_norms.push(PowerNormRow {
            power: exponent,
            frobenius_norm: frobenius_norm(&power),
        });
    }

    let mut nonnormal_condition_screen = Vec::new();
    for coupling in [0.5, 1.0, 2.0, 5.0, 10.0] {
        let mut operator = DenseMatrix::identity(stages);
        for row in 1..stages {
            operator[(row, row - 1)] = -coupling;
        }
        let operator_inverse = inverse(&operator)?;
        let operator_one_norm = matrix_one_norm(&operator);
        let inverse_one_norm = matrix_one_norm(&operator_inverse);
        nonnormal_condition_screen.push(NonnormalConditionRow {
            coupling,
            operator_one_norm,
            inverse_one_norm,
            condition_one: operator_one_norm * inverse_one_norm,
            // A unit lower-triangular matrix has determinant equal to the product of its diagonal.
            determinant: 1.0,
        });
    }

    let mut perturbed = target.clone();
    for stage in 0..stages {
        perturbed[stage * dimension] += 1e-6 * (stage as f64 + 1.0);
        perturbed[stage * dimension + 1] -= 5e-7 * (stage as f64 + 1.0).powi(2);
    }
    let perturbed_endpoint_certificate = oracle.certify_target(&perturbed, &[2e-4, 3e-4])?;

    let q_last = truncation_screen
        .last()
        .ok_or_else(|| CoreError::InvalidInput("empty truncation screen".into()))?;
    let l_last = official_l_power_norms
        .last()
        .ok_or_else(|| CoreError::InvalidInput("empty L-power screen".into()))?;
    let condition_grows = nonnormal_condition_screen
        .first()
        .zip(nonnormal_condition_screen.last())
        .is_some_and(|(first, last)| last.condition_one > first.condition_one);
    let passed = affine_endpoint_max_abs_error < 1e-10
        && flrh_lambda_spread < 1e-10
        && q_last.relative_error < 1e-10
        && l_last.frobenius_norm < 1e-10
        && condition_grows
        && perturbed_endpoint_certificate.output_wrms.is_finite();

    Ok(HomotopyDesignCheckReport {
        schema: "rodas5p-homotopy-design-check-v1",
        status: if passed { "pass" } else { "fail" },
        stages,
        dimension,
        affine_endpoint_max_abs_error,
        flrh_lambda_spread,
        truncation_screen,
        official_l_power_norms,
        nonnormal_condition_screen,
        perturbed_endpoint_certificate,
    })
}
