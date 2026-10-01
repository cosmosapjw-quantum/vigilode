use rodas5p_core::{
    CoreError, CoreResult, LinearSolverConfig, LuFactorization, WorkCounters, error_scale,
    rodas5p_coefficients, wrms,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::adaptive::record_adaptive_work_failure;
use crate::output::{HardStopCursor, OutputCollector};
use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveObservedIntegrationResult,
    AdaptiveRunDiagnostics, AdaptiveStepConfig, BdfConfig, BdfHistory, BdfOrder, BdfStepReport,
    HomotopyStepConfig, IntegrationMethod, KrylovState, ObservedIntegrationResult, OdeProblem,
    OutputSamplingPlan, RadauConfig, RadauIiaStages, SabrConfig, StageHistory, StepResult,
    TransactionalQ1Q2AdaptiveResult, TransactionalQ1Q2Config, TransactionalQ1Q2RunDiagnostics,
    adaptive_next_step_after_attempt, bdf_step, homotopy_step, radau_step,
    rodas_next_step_after_attempt, sabr_step, sequential_matrix_free_step,
    sequential_matrix_free_step_with_inner_forcing, sequential_step, transactional_q1_q2_step,
};
use crate::{bdf::adaptive_bdf_trial, radau::adaptive_radau_trial};

#[derive(Debug, Error)]
pub enum DenseOutputError {
    #[error(transparent)]
    Core(#[from] CoreError),
}

pub type DenseOutputResult<T> = Result<T, DenseOutputError>;

fn dense_output_core_error(error: DenseOutputError) -> CoreError {
    match error {
        DenseOutputError::Core(error) => error,
    }
}

fn validate_theta(theta: f64) -> CoreResult<()> {
    if !(theta.is_finite() && (0.0..=1.0).contains(&theta)) {
        return Err(CoreError::InvalidInput(
            "dense-output theta must be finite and in [0, 1]".into(),
        ));
    }
    Ok(())
}

fn effective_step_error(step: &StepResult) -> f64 {
    step.error_norm
        + step
            .certificate
            .as_ref()
            .map_or(0.0, |certificate| certificate.fixed_point_error)
}

fn adaptive_rejection_kind(error: f64, state: &[f64]) -> AdaptiveFailureKind {
    if error.is_finite() && state.iter().all(|value| value.is_finite()) {
        AdaptiveFailureKind::LocalError
    } else {
        AdaptiveFailureKind::NonFinite
    }
}

/// Evaluate the official RODAS5P third-degree continuous extension.
///
/// `StepResult::stages` are VigilODE's transformed K rows.  The core loader
/// precomputes D = H·Gamma, so the polynomial below intentionally has no
/// extra h multiplier.
pub fn rodas5p_dense_output(step: &StepResult, theta: f64) -> DenseOutputResult<Vec<f64>> {
    validate_theta(theta)?;
    if theta == 0.0 {
        return Ok(step.y_old.clone());
    }
    if theta == 1.0 {
        return Ok(step.y_new.clone());
    }
    let dimension = step.y_old.len();
    if dimension == 0
        || step.y_new.len() != dimension
        || step.stages.len() != 8
        || step.stages.iter().any(|row| row.len() != dimension)
    {
        return Err(
            CoreError::Dimension("RODAS5P dense-output stage shape mismatch".into()).into(),
        );
    }
    let coefficients = rodas5p_coefficients()?;
    if coefficients.dense_d.nrows() != 3 || coefficients.dense_d.ncols() != step.stages.len() {
        return Err(CoreError::Coefficients(
            "RODAS5P dense-output coefficient shape mismatch".into(),
        )
        .into());
    }
    let complement = 1.0 - theta;
    let mut output = vec![0.0; dimension];
    for component in 0..dimension {
        let d0 = coefficients
            .dense_d
            .row(0)
            .iter()
            .zip(&step.stages)
            .map(|(coefficient, stage)| coefficient * stage[component])
            .sum::<f64>();
        let d1 = coefficients
            .dense_d
            .row(1)
            .iter()
            .zip(&step.stages)
            .map(|(coefficient, stage)| coefficient * stage[component])
            .sum::<f64>();
        let d2 = coefficients
            .dense_d
            .row(2)
            .iter()
            .zip(&step.stages)
            .map(|(coefficient, stage)| coefficient * stage[component])
            .sum::<f64>();
        output[component] = complement * step.y_old[component]
            + theta * (step.y_new[component] + complement * (d0 + theta * (d1 + theta * d2)));
    }
    if !output.iter().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite("RODAS5P dense output contains NaN/Inf".into()).into());
    }
    Ok(output)
}

/// How the adaptive RODAS5P dense path treats its interior (interpolant)
/// error (audit F-002). The endpoint embedded estimate never controls the
/// interpolant: RODAS5P dense output is order 4 nonstiff and about order 3
/// on stiff problems, and it is not tolerance-controlled under `Off`.
///
/// `Off` is the default. The filtered modes cost one Jacobian and one LU per
/// sampled step, which a matrix-free run must not pay by default
/// (re-audit RA-07); `ReportDefect` needs one right-hand side only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DenseErrorControl {
    /// No interior estimate; bit-for-bit the legacy dense path.
    #[default]
    Off,
    /// Record the unfiltered defect estimate (h/2) M^{-1} d: one right-hand
    /// side, no Jacobian, no LU (a mass-matrix solve when M != I). It is
    /// state-unit comparable but unfiltered, so it overstates the error of
    /// stiff components by up to |h lambda|. A pointwise sample, not a bound.
    ReportDefect,
    /// Record the Jacobian-filtered estimate of every accepted step that
    /// contains an output time; never change the step sequence.
    Report,
    /// As `Report`, and reject such a step when its estimate exceeds one
    /// tolerance unit.
    Enforce,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DenseErrorSample {
    pub step_start: f64,
    pub step_size: f64,
    /// Interior estimate at theta = 1/2, WRMS tolerance units: filtered under
    /// `Report` and `Enforce`, unfiltered under `ReportDefect`.
    pub estimate_wrms: f64,
    /// The step was accepted (under `Enforce` a large estimate rejects it).
    pub accepted: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DenseErrorReport {
    pub control: DenseErrorControl,
    pub samples: Vec<DenseErrorSample>,
    /// Steps whose estimate could not be formed (no explicit Jacobian or a
    /// singular filter); they are neither accepted nor rejected on it.
    pub unavailable: usize,
    /// Work of the estimates. Under `Report` it is charged here only; under
    /// `Enforce` it is also in the run counters, since it decides steps.
    pub counters: WorkCounters,
}

impl DenseErrorReport {
    pub fn max_accepted_estimate(&self) -> Option<f64> {
        self.samples
            .iter()
            .filter(|sample| sample.accepted)
            .map(|sample| sample.estimate_wrms)
            .reduce(f64::max)
    }
}

/// Interior error estimate of one RODAS5P dense step (audit F-002).
///
/// With u(theta) the continuous extension and t_m = t_old + h/2, the defect
/// is d = M u'(t_m) - f(t_m, u(1/2)). The interior error e obeys
/// M e' = J e + d; one implicit-Euler step of that equation over the half
/// step gives (M - (h/2) J) e = (h/2) d. The filter keeps the estimate
/// bounded on stiff components (e -> -J^{-1} d) and reduces to (h/2) M^{-1} d
/// on nonstiff ones. Cost: one right-hand side, one Jacobian, one LU.
pub fn rodas5p_dense_interior_error_estimate(
    problem: &OdeProblem,
    step: &StepResult,
    atol: f64,
    rtol: f64,
    counters: &mut WorkCounters,
) -> CoreResult<f64> {
    let (defect, midpoint, mass) = scaled_midpoint_defect(problem, step, counters)?;
    let t_mid = step.t_old + 0.5 * step.h;
    let jacobian = problem.dense_jacobian(t_mid, &midpoint, counters)?;
    let filter = LuFactorization::new(&mass.combine(&jacobian, -0.5 * step.h)?)?;
    counters.direct_factorizations += 1;
    let estimate = filter.solve(&defect)?;
    let scale = error_scale(&step.y_old, &step.y_new, &[atol], rtol)?;
    wrms(&estimate, &scale)
}

/// Unfiltered interior defect estimate WRMS((h/2) M^{-1} d) (re-audit
/// RA-07): one right-hand side and no Jacobian. With M = I no solve is
/// needed; otherwise M is factorized once per call and charged.
pub fn rodas5p_dense_interior_defect_estimate(
    problem: &OdeProblem,
    step: &StepResult,
    atol: f64,
    rtol: f64,
    counters: &mut WorkCounters,
) -> CoreResult<f64> {
    let (defect, _, mass) = scaled_midpoint_defect(problem, step, counters)?;
    let estimate = if problem.mass_matrix.is_some() {
        counters.direct_factorizations += 1;
        LuFactorization::new(&mass)?.solve(&defect)?
    } else {
        defect
    };
    let scale = error_scale(&step.y_old, &step.y_new, &[atol], rtol)?;
    wrms(&estimate, &scale)
}

/// (h/2) d at theta = 1/2, the midpoint state and the mass matrix.
fn scaled_midpoint_defect(
    problem: &OdeProblem,
    step: &StepResult,
    counters: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, Vec<f64>, rodas5p_core::DenseMatrix)> {
    let n = step.y_old.len();
    if step.stages.len() != 8 || step.y_new.len() != n {
        return Err(CoreError::Dimension(
            "RODAS5P dense-output stage shape mismatch".into(),
        ));
    }
    let coefficients = rodas5p_coefficients()?;
    let h = step.h;
    let midpoint = rodas5p_dense_output(step, 0.5).map_err(dense_output_core_error)?;
    // du/dtheta at theta = 1/2 is y_new - y_old + (d1 + d2) / 4.
    let mut derivative = vec![0.0; n];
    for (component, value) in derivative.iter_mut().enumerate() {
        let d = |row: usize| {
            coefficients
                .dense_d
                .row(row)
                .iter()
                .zip(&step.stages)
                .map(|(coefficient, stage)| coefficient * stage[component])
                .sum::<f64>()
        };
        *value = (step.y_new[component] - step.y_old[component] + 0.25 * (d(1) + d(2))) / h;
    }
    let t_mid = step.t_old + 0.5 * h;
    let mass = problem.mass_or_identity();
    let flow = problem.eval_rhs(t_mid, &midpoint, counters)?;
    let defect = mass
        .matvec(&derivative)?
        .iter()
        .zip(&flow)
        .map(|(m_du, f)| 0.5 * h * (m_du - f))
        .collect::<Vec<_>>();
    Ok((defect, midpoint, mass))
}

fn radau_iia3_weights(theta: f64) -> [f64; 3] {
    let sqrt6 = 6.0_f64.sqrt();
    let c1 = (4.0 - sqrt6) / 10.0;
    let c2 = (4.0 + sqrt6) / 10.0;
    let theta2 = theta * theta;
    let theta3 = theta2 * theta;
    [
        25.0 / (3.0 * (1.0 + sqrt6)) * (theta3 / 3.0 - (1.0 + c2) * theta2 / 2.0 + c2 * theta),
        25.0 / (3.0 * (1.0 - sqrt6)) * (theta3 / 3.0 - (1.0 + c1) * theta2 / 2.0 + c1 * theta),
        10.0 * theta3 / 9.0 - 4.0 * theta2 / 3.0 + theta / 3.0,
    ]
}

/// Evaluate a Radau IIA3 collocation cubic from its K=h*f stage increments.
/// At theta=1 the integral weights are exactly the Radau endpoint weights.
pub fn radau_dense_output(
    stages: RadauIiaStages,
    y_old: &[f64],
    y_new: &[f64],
    stage_increments: &[Vec<f64>],
    theta: f64,
) -> DenseOutputResult<Vec<f64>> {
    validate_theta(theta)?;
    match stages {
        RadauIiaStages::One => {
            if y_old.is_empty()
                || y_old.len() != y_new.len()
                || stage_increments.len() != 1
                || stage_increments[0].len() != y_old.len()
            {
                return Err(CoreError::Dimension(
                    "Radau IIA1 dense-output stage shape mismatch".into(),
                )
                .into());
            }
            return Ok(y_old
                .iter()
                .zip(y_new)
                .map(|(old, new)| old + theta * (new - old))
                .collect());
        }
        RadauIiaStages::Three => {}
    }
    if theta == 0.0 {
        return Ok(y_old.to_vec());
    }
    if theta == 1.0 {
        return Ok(y_new.to_vec());
    }
    if y_old.is_empty()
        || y_old.len() != y_new.len()
        || stage_increments.len() != 3
        || stage_increments.iter().any(|row| row.len() != y_old.len())
    {
        return Err(
            CoreError::Dimension("Radau IIA3 dense-output stage shape mismatch".into()).into(),
        );
    }
    let weights = radau_iia3_weights(theta);
    let mut output = y_old.to_vec();
    for (weight, increment) in weights.iter().zip(stage_increments) {
        for (value, stage_value) in output.iter_mut().zip(increment) {
            *value += weight * stage_value;
        }
    }
    if !output.iter().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite("Radau dense output contains NaN/Inf".into()).into());
    }
    Ok(output)
}

/// Evaluate the BDF interval polynomial.  BDF1 and startup intervals use the
/// endpoint line.  A BDF2 interval uses the degree-two history polynomial
/// through `(t_n-h_prev, y_{n-1})`, `(t_n, y_n)`, and
/// `(t_n+h, y_{n+1})` with the actual step ratio.
pub fn bdf_dense_output(
    report: &BdfStepReport,
    y_old: &[f64],
    theta: f64,
) -> DenseOutputResult<Vec<f64>> {
    validate_theta(theta)?;
    if y_old.is_empty() || y_old.len() != report.y_new.len() {
        return Err(CoreError::Dimension("BDF dense-output state shape mismatch".into()).into());
    }
    if theta == 0.0 {
        return Ok(y_old.to_vec());
    }
    if theta == 1.0 {
        return Ok(report.y_new.clone());
    }
    let output = match (
        report.applied_order,
        report.interpolation_previous_state.as_deref(),
        report.interpolation_previous_step,
    ) {
        (BdfOrder::Two, Some(previous), Some(previous_h)) => {
            if previous.len() != y_old.len() || !(previous_h > 0.0 && previous_h.is_finite()) {
                return Err(CoreError::Dimension(
                    "BDF2 dense-output history shape mismatch".into(),
                )
                .into());
            }
            let current_h = report
                .step_ratio
                .map(|ratio| ratio * previous_h)
                .unwrap_or(previous_h);
            if !(current_h > 0.0 && current_h.is_finite()) {
                return Err(CoreError::NonFinite(
                    "BDF2 dense-output step ratio is non-finite".into(),
                )
                .into());
            }
            let q = previous_h / current_h;
            let previous_weight = theta * (theta - 1.0) / (q * (q + 1.0));
            let old_weight = (theta + q) * (1.0 - theta) / q;
            let new_weight = theta * (theta + q) / (1.0 + q);
            previous
                .iter()
                .zip(y_old)
                .zip(&report.y_new)
                .map(|((oldest, old), new)| {
                    previous_weight * oldest + old_weight * old + new_weight * new
                })
                .collect::<Vec<_>>()
        }
        _ => y_old
            .iter()
            .zip(&report.y_new)
            .map(|(old, new)| old + theta * (new - old))
            .collect::<Vec<_>>(),
    };
    if !output.iter().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite("BDF dense output contains NaN/Inf".into()).into());
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_fixed_dense_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    atol: f64,
    rtol: f64,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<ObservedIntegrationResult> {
    let (mut t, tf) = t_span;
    if h <= 0.0 || tf < t {
        return Err(CoreError::InvalidInput("invalid fixed-step interval".into()).into());
    }
    let config = linear_config.cloned().unwrap_or_default();
    let mut counters = WorkCounters::default();
    let mut history = StageHistory::default();
    let mut recycle = KrylovState::for_method(config.method);
    let mut state = y0.to_vec();
    let mut collector = OutputCollector::new(sampling.output(), t_span, y0)?;
    let mut hard_stops = HardStopCursor::new(sampling, t_span)?;
    let sabr_config = sabr_config.unwrap_or_default();
    let mut internal_steps = 0_usize;
    // Indexed grid t0 + k h (re-audit R3, R3-TIME-03).
    let grid = crate::output::FixedGrid::new(t_span.0, tf, h)?;
    while t < tf {
        let (step, _hard_stop_landing) = hard_stops.limit_step(t, grid.step_from(t)?, tf)?;
        let report = match method {
            IntegrationMethod::Sequential => {
                let report = sequential_step(
                    problem,
                    t,
                    &state,
                    step,
                    &config,
                    recycle.as_mut(),
                    atol,
                    rtol,
                    true,
                    &mut counters,
                )?;
                history.push(step, report.stages.clone());
                report
            }
            IntegrationMethod::Sabr => sabr_step(
                problem,
                t,
                &state,
                step,
                &sabr_config,
                Some(&config),
                &mut history,
                recycle.as_mut(),
                atol,
                rtol,
                true,
                &mut counters,
            )?,
        };
        let old_t = t;
        t = report.t_new;
        collector.accept_dense_interval(old_t, t, &report.y_new, |theta| {
            rodas5p_dense_output(&report, theta).map_err(dense_output_core_error)
        })?;
        state = report.y_new;
        internal_steps += 1;
    }
    let (t, y, output_clipped_steps) = collector.finish()?;
    Ok(ObservedIntegrationResult {
        t,
        y,
        success: true,
        message: "success".into(),
        counters,
        internal_steps,
        output_clipped_steps,
    })
}

/// Integrate the ordinary sequential or SABR adaptive path while sampling the
/// accepted RODAS5P continuous extension.  Output times are observations only;
/// only explicit `sampling.hard_stops()` may shorten a trial step.
#[allow(clippy::too_many_arguments)]
pub fn integrate_adaptive_dense_observed_with_config(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<AdaptiveObservedIntegrationResult> {
    integrate_adaptive_dense_observed_with_dense_error_control(
        problem,
        t_span,
        y0,
        method,
        linear_config,
        sabr_config,
        adaptive,
        sampling,
        DenseErrorControl::Off,
    )
    .map(|(result, _)| result)
}

/// The adaptive dense path with an explicit interior-error mode (audit
/// F-002). `Off` is the legacy path bit for bit; `Report` records the
/// filtered-defect estimate of every accepted step that contains an interior
/// output time; `Enforce` also rejects such a step when the estimate
/// exceeds one tolerance unit.
#[allow(clippy::too_many_arguments)]
pub fn integrate_adaptive_dense_observed_with_dense_error_control(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
    control: DenseErrorControl,
) -> DenseOutputResult<(AdaptiveObservedIntegrationResult, DenseErrorReport)> {
    let mut dense_report = DenseErrorReport {
        control,
        ..DenseErrorReport::default()
    };
    let output_times = sampling.output().times().to_vec();
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(
            CoreError::InvalidInput("invalid adaptive dense-output interval".into()).into(),
        );
    }
    let config = linear_config.cloned().unwrap_or_default();
    let mut state = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut history = StageHistory::default();
    let mut recycle = KrylovState::for_method(config.method);
    let sabr_config = sabr_config.unwrap_or_default();
    let mut collector =
        OutputCollector::new(sampling.output(), t_span, y0)?.with_max_step(adaptive.max_step);
    let mut hard_stops = HardStopCursor::new(sampling, t_span)?.with_max_step(adaptive.max_step);
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut internal_steps = 0_usize;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        let Some(next_h) = crate::output::adaptive_end_step(t, h, tf, adaptive.max_step)? else {
            break;
        };
        h = next_h;
        // A final piece that lands on tf is taken even below min_step.
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf) || t + h == t {
            break;
        }
        let requested_h = h;
        let (trial_h, hard_stop_landing) = hard_stops.limit_step(t, requested_h, tf)?;
        let recycle_snapshot = recycle.clone();
        let history_snapshot = history.clone();
        let trial = match method {
            IntegrationMethod::Sequential => sequential_step(
                problem,
                t,
                &state,
                trial_h,
                &config,
                recycle.as_mut(),
                adaptive.atol,
                adaptive.rtol,
                false,
                &mut counters,
            ),
            IntegrationMethod::Sabr => sabr_step(
                problem,
                t,
                &state,
                trial_h,
                &sabr_config,
                Some(&config),
                &mut history,
                recycle.as_mut(),
                adaptive.atol,
                adaptive.rtol,
                false,
                &mut counters,
            ),
        };
        let report = match trial {
            Ok(report) => report,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("failure kind checked");
                counters.rejected_steps += 1;
                record_adaptive_work_failure(&mut counters, failure);
                recycle = recycle_snapshot;
                history = history_snapshot;
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    crate::RODAS5P_ESTIMATOR_ORDER,
                    "rodas5p-embedded",
                    false,
                    Some(failure),
                );
                h = rodas_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    requested_h,
                    trial_h,
                    f64::INFINITY,
                    false,
                    hard_stop_landing,
                )?;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let mut error = effective_step_error(&report);
        let mut accepted =
            report.accepted && error <= 1.0 && report.y_new.iter().all(|value| value.is_finite());
        if accepted
            && control != DenseErrorControl::Off
            && output_times
                .iter()
                .any(|&time| time > report.t_old && time < report.t_new)
        {
            let mut estimate_work = WorkCounters::default();
            let estimate = if control == DenseErrorControl::ReportDefect {
                rodas5p_dense_interior_defect_estimate(
                    problem,
                    &report,
                    adaptive.atol,
                    adaptive.rtol,
                    &mut estimate_work,
                )
            } else {
                rodas5p_dense_interior_error_estimate(
                    problem,
                    &report,
                    adaptive.atol,
                    adaptive.rtol,
                    &mut estimate_work,
                )
            };
            match estimate {
                Ok(estimate) => {
                    let rejected = control == DenseErrorControl::Enforce
                        && !(estimate.is_finite() && estimate <= 1.0);
                    if rejected {
                        accepted = false;
                        error = if estimate.is_finite() {
                            estimate
                        } else {
                            f64::INFINITY
                        };
                    }
                    dense_report.samples.push(DenseErrorSample {
                        step_start: report.t_old,
                        step_size: report.h,
                        estimate_wrms: estimate,
                        accepted: !rejected,
                    });
                }
                Err(_) => dense_report.unavailable += 1,
            }
            dense_report.counters.accumulate(estimate_work);
            if control == DenseErrorControl::Enforce {
                counters.accumulate(estimate_work);
            }
        }
        crate::adaptive::reconcile_outer_rejection(&mut counters, report.accepted, accepted);
        let failure = (!accepted).then_some(adaptive_rejection_kind(error, &report.y_new));
        diagnostics.record_with_failure(
            trial_h,
            error,
            crate::RODAS5P_ESTIMATOR_ORDER,
            "rodas5p-embedded-plus-algebraic",
            accepted,
            failure,
        );
        if let Some(failure) = failure {
            record_adaptive_work_failure(&mut counters, failure);
        }
        if accepted {
            let old_t = t;
            t = report.t_new;
            collector.accept_dense_interval(old_t, t, &report.y_new, |theta| {
                rodas5p_dense_output(&report, theta).map_err(dense_output_core_error)
            })?;
            state = report.y_new.clone();
            internal_steps += 1;
            if method == IntegrationMethod::Sequential {
                history.push(report.h, report.stages.clone());
            }
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                error,
                true,
                hard_stop_landing,
            )?;
        } else {
            recycle = recycle_snapshot;
            history = history_snapshot;
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                error,
                false,
                hard_stop_landing,
            )?;
        }
    }
    diagnostics.fallback_steps = counters.fallback_steps as usize;
    dense_adaptive_result(t, tf, collector, counters, internal_steps, diagnostics)
        .map(|result| (result, dense_report))
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_sequential_matrix_free_adaptive_dense_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<AdaptiveObservedIntegrationResult> {
    integrate_sequential_matrix_free_adaptive_dense_observed_with(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        sampling,
        MatrixFreeDenseStep::InnerForcing,
        |report, theta| rodas5p_dense_output(report, theta).map_err(dense_output_core_error),
    )
}

/// The same protected dense run with every stage solved to the fixed inner
/// tolerance `linear_config.{rtol, atol}` instead of the WRMS forcing rule.
///
/// Diagnostic attribution arm for audit F-033: comparing its global error
/// with the forcing arm separates error caused by inexact stage solves from
/// error the exact-stage method would also make.
pub fn integrate_sequential_matrix_free_adaptive_dense_fixed_inner_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<AdaptiveObservedIntegrationResult> {
    integrate_sequential_matrix_free_adaptive_dense_observed_with(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        sampling,
        MatrixFreeDenseStep::FixedInnerTolerance,
        |report, theta| rodas5p_dense_output(report, theta).map_err(dense_output_core_error),
    )
}

/// One same-step comparison at an interior output time (audit F-007, part C).
///
/// `delta_wrms` is the WRMS difference, in the run's own tolerance weights,
/// between the dense interpolant at `t = step_start + theta * step_size` and
/// one hard-stop step of length `theta * step_size` restarted from the same
/// accepted state. Both are local approximations from the same state, so the
/// difference is bounded by the interpolant defect plus the sub-step's local
/// error and needs no reference solution.
///
/// It is an attribution diagnostic, not a bound on the dense error: only
/// `||u_dense - y|| <= delta + ||u_sub - y||` holds, and the second term is
/// not estimated here. The sub-step shares the method, coefficients and
/// forcing with the run and is force-accepted (audit 2026-09-30).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InterpolantAuditSample {
    pub t: f64,
    pub step_start: f64,
    pub step_size: f64,
    pub theta: f64,
    pub delta_wrms: f64,
}

/// Same-step interpolant comparisons of one dense run. The sub-steps are
/// charged to `counters` only, never to the run they audit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InterpolantAudit {
    pub samples: Vec<InterpolantAuditSample>,
    /// Sub-steps that returned an error; each also records `delta_wrms = inf`.
    pub failed_substeps: usize,
    pub counters: WorkCounters,
}

impl InterpolantAudit {
    /// Largest delta, `None` when no interior output time was interpolated.
    pub fn max_delta_wrms(&self) -> Option<f64> {
        self.samples
            .iter()
            .map(|sample| sample.delta_wrms)
            .reduce(|a, b| if a.is_nan() || b > a { b } else { a })
    }

    /// Append another segment's audit.
    pub fn extend(&mut self, other: InterpolantAudit) {
        self.samples.extend(other.samples);
        self.failed_substeps += other.failed_substeps;
        self.counters.accumulate(other.counters);
    }

    fn record(
        &mut self,
        problem: &OdeProblem,
        report: &StepResult,
        theta: f64,
        interpolated: &[f64],
        linear_config: &LinearSolverConfig,
        adaptive: &AdaptiveStepConfig,
    ) {
        let step_size = theta * report.h;
        let mut recycle = KrylovState::for_method(linear_config.method);
        let substep = sequential_matrix_free_step_with_inner_forcing(
            problem,
            report.t_old,
            &report.y_old,
            step_size,
            linear_config,
            recycle.as_mut(),
            adaptive.atol,
            adaptive.rtol,
            true,
            &mut self.counters,
        );
        let delta_wrms = match substep {
            Ok(substep) => {
                let difference = interpolated
                    .iter()
                    .zip(&substep.step.y_new)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>();
                error_scale(
                    &report.y_old,
                    &substep.step.y_new,
                    &[adaptive.atol],
                    adaptive.rtol,
                )
                .and_then(|scale| wrms(&difference, &scale))
                .unwrap_or(f64::INFINITY)
            }
            Err(_) => {
                self.failed_substeps += 1;
                f64::INFINITY
            }
        };
        self.samples.push(InterpolantAuditSample {
            t: report.t_old + step_size,
            step_start: report.t_old,
            step_size: report.h,
            theta,
            delta_wrms,
        });
    }
}

/// `integrate_sequential_matrix_free_adaptive_dense_observed` plus the
/// same-step interpolant audit of audit F-007 part C. The integration itself
/// is unchanged bit for bit; every interior output time additionally gets one
/// hard-stop sub-step from the accepted state that produced it.
pub fn integrate_sequential_matrix_free_adaptive_dense_with_interpolant_audit(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<(AdaptiveObservedIntegrationResult, InterpolantAudit)> {
    integrate_sequential_matrix_free_adaptive_dense_with_interpolant_audit_and_evaluator(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        sampling,
        |report, theta| rodas5p_dense_output(report, theta).map_err(dense_output_core_error),
    )
}

/// Test seam: the audited run with a substitute dense evaluator, so a
/// corrupted interpolant can be shown to raise the audit's delta.
#[doc(hidden)]
pub fn integrate_sequential_matrix_free_adaptive_dense_with_interpolant_audit_and_evaluator<F>(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
    mut dense_evaluator: F,
) -> DenseOutputResult<(AdaptiveObservedIntegrationResult, InterpolantAudit)>
where
    F: FnMut(&StepResult, f64) -> CoreResult<Vec<f64>>,
{
    let mut audit = InterpolantAudit::default();
    let result = integrate_sequential_matrix_free_adaptive_dense_observed_with(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        sampling,
        MatrixFreeDenseStep::InnerForcing,
        |report, theta| {
            let value = dense_evaluator(report, theta)?;
            if theta > 0.0 && theta < 1.0 {
                audit.record(problem, report, theta, &value, linear_config, adaptive);
            }
            Ok(value)
        },
    )?;
    Ok((result, audit))
}

/// Stage-solve rule of the protected matrix-free dense loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatrixFreeDenseStep {
    /// WRMS stage-residual forcing (production).
    InnerForcing,
    /// Fixed inner tolerance from the linear configuration (attribution arm).
    FixedInnerTolerance,
}

fn protected_dense_adaptive_failure(
    collector: OutputCollector,
    counters: WorkCounters,
    mut diagnostics: AdaptiveRunDiagnostics,
    internal_steps: usize,
    error: &CoreError,
) -> AdaptiveObservedIntegrationResult {
    diagnostics.fallback_steps = counters.fallback_steps as usize;
    let (times, states, output_clipped_steps) = collector.finish_partial();
    AdaptiveObservedIntegrationResult {
        observed: ObservedIntegrationResult {
            t: times,
            y: states,
            success: false,
            message: format!("post-start dense integration error: {error}"),
            counters,
            internal_steps,
            output_clipped_steps,
        },
        diagnostics,
    }
}

#[allow(clippy::too_many_arguments)]
fn integrate_sequential_matrix_free_adaptive_dense_observed_with<F>(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
    step_kind: MatrixFreeDenseStep,
    mut dense_evaluator: F,
) -> DenseOutputResult<AdaptiveObservedIntegrationResult>
where
    F: FnMut(&StepResult, f64) -> CoreResult<Vec<f64>>,
{
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid protected matrix-free RODAS5P integration input".into(),
        )
        .into());
    }
    let mut state = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut recycle = KrylovState::for_method(linear_config.method);
    let mut collector =
        OutputCollector::new(sampling.output(), t_span, y0)?.with_max_step(adaptive.max_step);
    let mut hard_stops = HardStopCursor::new(sampling, t_span)?.with_max_step(adaptive.max_step);
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut internal_steps = 0_usize;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        let Some(next_h) = crate::output::adaptive_end_step(t, h, tf, adaptive.max_step)? else {
            break;
        };
        h = next_h;
        // A final piece that lands on tf is taken even below min_step.
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf) || t + h == t {
            break;
        }
        let requested_h = h;
        let (trial_h, hard_stop_landing) = match hard_stops.limit_step(t, requested_h, tf) {
            Ok(value) => value,
            Err(error) => {
                return Ok(protected_dense_adaptive_failure(
                    collector,
                    counters,
                    diagnostics,
                    internal_steps,
                    &error,
                ));
            }
        };
        let recycle_snapshot = recycle.clone();
        let trial = match step_kind {
            MatrixFreeDenseStep::InnerForcing => sequential_matrix_free_step_with_inner_forcing(
                problem,
                t,
                &state,
                trial_h,
                linear_config,
                recycle.as_mut(),
                adaptive.atol,
                adaptive.rtol,
                false,
                &mut counters,
            )
            .map(|report| report.step),
            MatrixFreeDenseStep::FixedInnerTolerance => sequential_matrix_free_step(
                problem,
                t,
                &state,
                trial_h,
                linear_config,
                recycle.as_mut(),
                adaptive.atol,
                adaptive.rtol,
                false,
                &mut counters,
            ),
        };
        let report = match trial {
            Ok(report) => report,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("failure kind checked");
                recycle = recycle_snapshot;
                counters.rejected_steps += 1;
                record_adaptive_work_failure(&mut counters, failure);
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    crate::RODAS5P_ESTIMATOR_ORDER,
                    "protected-matrix-free-rodas5p-step-failed",
                    false,
                    Some(failure),
                );
                h = match rodas_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    requested_h,
                    trial_h,
                    f64::INFINITY,
                    false,
                    hard_stop_landing,
                ) {
                    Ok(next) => next,
                    Err(error) => {
                        return Ok(protected_dense_adaptive_failure(
                            collector,
                            counters,
                            diagnostics,
                            internal_steps,
                            &error,
                        ));
                    }
                };
                continue;
            }
            Err(error) => {
                return Ok(protected_dense_adaptive_failure(
                    collector,
                    counters,
                    diagnostics,
                    internal_steps,
                    &error,
                ));
            }
        };
        let error = report.error_norm;
        let accepted =
            report.accepted && error <= 1.0 && report.y_new.iter().all(|value| value.is_finite());
        crate::adaptive::reconcile_outer_rejection(&mut counters, report.accepted, accepted);
        if accepted {
            diagnostics.record(
                trial_h,
                error,
                crate::RODAS5P_ESTIMATOR_ORDER,
                "protected-matrix-free-rodas5p-embedded",
                true,
            );
            let old_t = t;
            t = report.t_new;
            state = report.y_new.clone();
            internal_steps += 1;
            if let Err(error) = collector
                .accept_dense_interval(old_t, t, &state, |theta| dense_evaluator(&report, theta))
            {
                return Ok(protected_dense_adaptive_failure(
                    collector,
                    counters,
                    diagnostics,
                    internal_steps,
                    &error,
                ));
            }
            h = match rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                error,
                true,
                hard_stop_landing,
            ) {
                Ok(next) => next,
                Err(error) => {
                    return Ok(protected_dense_adaptive_failure(
                        collector,
                        counters,
                        diagnostics,
                        internal_steps,
                        &error,
                    ));
                }
            };
        } else {
            recycle = recycle_snapshot;
            let failure = if error.is_finite() && report.y_new.iter().all(|value| value.is_finite())
            {
                AdaptiveFailureKind::LocalError
            } else {
                AdaptiveFailureKind::NonFinite
            };
            diagnostics.record_with_failure(
                trial_h,
                error,
                crate::RODAS5P_ESTIMATOR_ORDER,
                "protected-matrix-free-rodas5p-embedded",
                false,
                Some(failure),
            );
            record_adaptive_work_failure(&mut counters, failure);
            h = match rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                error,
                false,
                hard_stop_landing,
            ) {
                Ok(next) => next,
                Err(error) => {
                    return Ok(protected_dense_adaptive_failure(
                        collector,
                        counters,
                        diagnostics,
                        internal_steps,
                        &error,
                    ));
                }
            };
        }
    }
    diagnostics.fallback_steps = counters.fallback_steps as usize;
    let success = t >= tf;
    let observed = if success && collector.is_complete() {
        let (t, y, output_clipped_steps) = collector
            .finish()
            .expect("complete protected dense collector must finish");
        ObservedIntegrationResult {
            t,
            y,
            success: true,
            message: "success".into(),
            counters,
            internal_steps,
            output_clipped_steps,
        }
    } else if !success {
        let (times, states, output_clipped_steps) = collector.finish_partial();
        ObservedIntegrationResult {
            t: times,
            y: states,
            success: false,
            message: "maximum step count or minimum step reached".into(),
            counters,
            internal_steps,
            output_clipped_steps,
        }
    } else {
        return Ok(protected_dense_adaptive_failure(
            collector,
            counters,
            diagnostics,
            internal_steps,
            &CoreError::InvalidInput(
                "dense integration reached endpoint before all requested outputs were recorded"
                    .into(),
            ),
        ));
    };
    Ok(AdaptiveObservedIntegrationResult {
        observed,
        diagnostics,
    })
}

/// Integrate the homotopy lane while sampling each accepted RODAS5P stage
/// polynomial.  The dense and clipped callers construct independent solver,
/// controller, recycle, and fallback state.
#[allow(clippy::too_many_arguments)]
pub fn integrate_homotopy_adaptive_dense_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    homotopy_config: &HomotopyStepConfig,
    fallback_config: Option<&LinearSolverConfig>,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<AdaptiveObservedIntegrationResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid adaptive dense Homotopy integration input".into(),
        )
        .into());
    }
    let mut state = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut recycle = fallback_config.and_then(|config| KrylovState::for_method(config.method));
    let mut collector =
        OutputCollector::new(sampling.output(), t_span, y0)?.with_max_step(adaptive.max_step);
    let mut hard_stops = HardStopCursor::new(sampling, t_span)?.with_max_step(adaptive.max_step);
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut internal_steps = 0_usize;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        let Some(next_h) = crate::output::adaptive_end_step(t, h, tf, adaptive.max_step)? else {
            break;
        };
        h = next_h;
        // A final piece that lands on tf is taken even below min_step.
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf) || t + h == t {
            break;
        }
        let requested_h = h;
        let (trial_h, hard_stop_landing) = hard_stops.limit_step(t, requested_h, tf)?;
        let recycle_snapshot = recycle.clone();
        let trial = homotopy_step(
            problem,
            t,
            &state,
            trial_h,
            homotopy_config,
            fallback_config,
            recycle.as_mut(),
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut counters,
        );
        let report = match trial {
            Ok(report) => report,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("failure kind checked");
                recycle = recycle_snapshot;
                counters.rejected_steps += 1;
                record_adaptive_work_failure(&mut counters, failure);
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    crate::RODAS5P_ESTIMATOR_ORDER,
                    "homotopy-step-failed",
                    false,
                    Some(failure),
                );
                h = rodas_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    requested_h,
                    trial_h,
                    f64::INFINITY,
                    false,
                    hard_stop_landing,
                )?;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if !report.fast_accepted {
            diagnostics.fallback_steps += 1;
        }
        let error = effective_step_error(&report.step);
        let accepted = report.step.accepted
            && error <= 1.0
            && report.step.y_new.iter().all(|value| value.is_finite());
        let failure = (!accepted).then_some(adaptive_rejection_kind(error, &report.step.y_new));
        diagnostics.record_with_failure(
            trial_h,
            error,
            crate::RODAS5P_ESTIMATOR_ORDER,
            "homotopy-native-rodas-endpoint",
            accepted,
            failure,
        );
        if let Some(failure) = failure {
            record_adaptive_work_failure(&mut counters, failure);
        }
        if accepted {
            let old_t = t;
            t = report.step.t_new;
            collector.accept_dense_interval(old_t, t, &report.step.y_new, |theta| {
                rodas5p_dense_output(&report.step, theta).map_err(dense_output_core_error)
            })?;
            state = report.step.y_new.clone();
            internal_steps += 1;
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                error,
                true,
                hard_stop_landing,
            )?;
        } else {
            recycle = recycle_snapshot;
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                error,
                false,
                hard_stop_landing,
            )?;
        }
    }

    dense_adaptive_result(t, tf, collector, counters, internal_steps, diagnostics)
}

/// Integrate the transactional q1/q2 lane with dense sampling of the final
/// accepted stage set for each attempt.
#[allow(clippy::too_many_arguments)]
pub fn integrate_transactional_q1_q2_adaptive_dense_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    step_config: &TransactionalQ1Q2Config,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<TransactionalQ1Q2AdaptiveResult> {
    adaptive.validate()?;
    step_config.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid adaptive dense transactional q1/q2 integration input".into(),
        )
        .into());
    }
    let mut state = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut collector =
        OutputCollector::new(sampling.output(), t_span, y0)?.with_max_step(adaptive.max_step);
    let mut hard_stops = HardStopCursor::new(sampling, t_span)?.with_max_step(adaptive.max_step);
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut transactional = TransactionalQ1Q2RunDiagnostics::default();
    let mut internal_steps = 0_usize;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        let Some(next_h) = crate::output::adaptive_end_step(t, h, tf, adaptive.max_step)? else {
            break;
        };
        h = next_h;
        // A final piece that lands on tf is taken even below min_step.
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf) || t + h == t {
            break;
        }
        let requested_h = h;
        let (trial_h, hard_stop_landing) = hard_stops.limit_step(t, requested_h, tf)?;
        let trial = transactional_q1_q2_step(
            problem,
            t,
            &state,
            trial_h,
            step_config,
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut counters,
        );
        let report = match trial {
            Ok(report) => report,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("failure kind checked");
                counters.rejected_steps += 1;
                record_adaptive_work_failure(&mut counters, failure);
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    crate::RODAS5P_ESTIMATOR_ORDER,
                    "transactional-q1-q2-step-failed",
                    false,
                    Some(failure),
                );
                h = rodas_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    requested_h,
                    trial_h,
                    f64::INFINITY,
                    false,
                    hard_stop_landing,
                )?;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let error = effective_step_error(&report.step);
        let accepted = report.step.accepted
            && error <= 1.0
            && report.step.y_new.iter().all(|value| value.is_finite());
        let failure = (!accepted).then_some(adaptive_rejection_kind(error, &report.step.y_new));
        diagnostics.record_with_failure(
            trial_h,
            error,
            crate::RODAS5P_ESTIMATOR_ORDER,
            "rodas5p-embedded-plus-transactional-algebraic",
            accepted,
            failure,
        );
        if let Some(failure) = failure {
            record_adaptive_work_failure(&mut counters, failure);
        }
        transactional.record(&report, accepted);
        if accepted {
            let old_t = t;
            t = report.step.t_new;
            collector.accept_dense_interval(old_t, t, &report.step.y_new, |theta| {
                rodas5p_dense_output(&report.step, theta).map_err(dense_output_core_error)
            })?;
            state = report.step.y_new.clone();
            internal_steps += 1;
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                error,
                true,
                hard_stop_landing,
            )?;
        } else {
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                error,
                false,
                hard_stop_landing,
            )?;
        }
    }

    diagnostics.fallback_steps = transactional.accepted_sequential_fallback_steps;
    let adaptive_result =
        dense_adaptive_result(t, tf, collector, counters, internal_steps, diagnostics)?;
    Ok(TransactionalQ1Q2AdaptiveResult {
        observed: adaptive_result.observed,
        diagnostics: adaptive_result.diagnostics,
        transactional,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_radau_fixed_dense_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    config: &RadauConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<ObservedIntegrationResult> {
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || !(h > 0.0 && h.is_finite()) || tf < t {
        return Err(
            CoreError::InvalidInput("invalid fixed-step Radau integration input".into()).into(),
        );
    }
    let mut state = y0.to_vec();
    let mut counters = WorkCounters::default();
    let mut collector = OutputCollector::new(sampling.output(), t_span, y0)?;
    let mut hard_stops = HardStopCursor::new(sampling, t_span)?;
    let mut internal_steps = 0_usize;
    // Indexed grid t0 + k h (re-audit R3, R3-TIME-03).
    let grid = crate::output::FixedGrid::new(t_span.0, tf, h)?;
    while t < tf {
        let (step, _hard_stop_landing) = hard_stops.limit_step(t, grid.step_from(t)?, tf)?;
        let report = radau_step(problem, t, &state, step, config, &mut counters)?;
        let old_t = t;
        t = report.t_new;
        collector.accept_dense_interval(old_t, t, &report.y_new, |theta| {
            radau_dense_output(
                report.stages,
                &state,
                &report.y_new,
                &report.stage_increments,
                theta,
            )
            .map_err(dense_output_core_error)
        })?;
        state = report.y_new;
        counters.accepted_steps += 1;
        internal_steps += 1;
    }
    let (t, y, output_clipped_steps) = collector.finish()?;
    Ok(ObservedIntegrationResult {
        t,
        y,
        success: true,
        message: "success".into(),
        counters,
        internal_steps,
        output_clipped_steps,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_bdf_fixed_dense_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    config: &BdfConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<ObservedIntegrationResult> {
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || !(h > 0.0 && h.is_finite()) || tf < t {
        return Err(
            CoreError::InvalidInput("invalid fixed-step BDF integration input".into()).into(),
        );
    }
    let mut state = y0.to_vec();
    let mut history = BdfHistory::default();
    let mut counters = WorkCounters::default();
    let mut collector = OutputCollector::new(sampling.output(), t_span, y0)?;
    let mut hard_stops = HardStopCursor::new(sampling, t_span)?;
    let mut internal_steps = 0_usize;
    // Indexed grid t0 + k h (re-audit R3, R3-TIME-03).
    let grid = crate::output::FixedGrid::new(t_span.0, tf, h)?;
    while t < tf {
        let (step, _hard_stop_shortened) = hard_stops.limit_step(t, grid.step_from(t)?, tf)?;
        let report = bdf_step(
            problem,
            t,
            &state,
            step,
            config,
            &mut history,
            &mut counters,
        )?;
        let old_t = t;
        t = report.t_new;
        collector.accept_dense_interval(old_t, t, &report.y_new, |theta| {
            bdf_dense_output(&report, &state, theta).map_err(dense_output_core_error)
        })?;
        state = report.y_new;
        if hard_stops.consume_landing(t)? {
            history.clear();
        }
        counters.accepted_steps += 1;
        internal_steps += 1;
    }
    let (t, y, output_clipped_steps) = collector.finish()?;
    Ok(ObservedIntegrationResult {
        t,
        y,
        success: true,
        message: "success".into(),
        counters,
        internal_steps,
        output_clipped_steps,
    })
}

pub fn integrate_radau_adaptive_dense_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    config: &RadauConfig,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<AdaptiveObservedIntegrationResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(
            CoreError::InvalidInput("invalid adaptive Radau integration input".into()).into(),
        );
    }
    let mut state = y0.to_vec();
    let mut counters = WorkCounters::default();
    let mut controller = AdaptiveControllerState::default();
    let mut collector =
        OutputCollector::new(sampling.output(), t_span, y0)?.with_max_step(adaptive.max_step);
    let mut hard_stops = HardStopCursor::new(sampling, t_span)?.with_max_step(adaptive.max_step);
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut internal_steps = 0_usize;
    let (estimator_order, estimator_id) = match config.stages {
        RadauIiaStages::One => (2, "radau-iia1-step-doubling"),
        RadauIiaStages::Three => (4, "radau-iia3-scipy-1.17.0-embedded-order3"),
    };
    let mut previous_local_rejection = false;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        let Some(next_h) = crate::output::adaptive_end_step(t, h, tf, adaptive.max_step)? else {
            break;
        };
        h = next_h;
        // A final piece that lands on tf is taken even below min_step.
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf)
            || 0.5 * h <= f64::MIN_POSITIVE
            || t + h == t
        {
            break;
        }
        let requested_h = h;
        let (trial_h, hard_stop_landing) = hard_stops.limit_step(t, requested_h, tf)?;
        let trial = match adaptive_radau_trial(
            problem,
            t,
            &state,
            trial_h,
            config,
            adaptive,
            previous_local_rejection,
            &mut counters,
        ) {
            Ok(trial) => trial,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("failure kind checked");
                counters.rejected_steps += 1;
                record_adaptive_work_failure(&mut counters, failure);
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    estimator_order,
                    estimator_id,
                    false,
                    Some(failure),
                );
                h = adaptive_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    requested_h,
                    trial_h,
                    f64::INFINITY,
                    estimator_order,
                    false,
                    hard_stop_landing,
                )?;
                previous_local_rejection = true;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let accepted =
            trial.error_norm <= 1.0 && trial.y_new().iter().all(|value| value.is_finite());
        diagnostics.record(
            trial_h,
            trial.error_norm,
            trial.estimator_order,
            trial.estimator_id,
            accepted,
        );
        if accepted {
            let accepted_internal_steps = trial.accepted_internal_steps();
            let mut piece_t = t;
            let mut piece_state = state.clone();
            for report in &trial.accepted_reports {
                collector.accept_dense_interval(piece_t, report.t_new, &report.y_new, |theta| {
                    radau_dense_output(
                        report.stages,
                        &piece_state,
                        &report.y_new,
                        &report.stage_increments,
                        theta,
                    )
                    .map_err(dense_output_core_error)
                })?;
                piece_t = report.t_new;
                piece_state = report.y_new.clone();
            }
            t += trial_h;
            state = trial.y_new().to_vec();
            counters.accepted_steps += accepted_internal_steps as u64;
            internal_steps += accepted_internal_steps;
            h = adaptive_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                trial.error_norm,
                trial.estimator_order,
                true,
                hard_stop_landing,
            )?;
            previous_local_rejection = false;
        } else {
            counters.rejected_steps += 1;
            record_adaptive_work_failure(&mut counters, AdaptiveFailureKind::LocalError);
            h = adaptive_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                trial.error_norm,
                trial.estimator_order,
                false,
                hard_stop_landing,
            )?;
            previous_local_rejection = true;
        }
    }
    dense_adaptive_result(t, tf, collector, counters, internal_steps, diagnostics)
}

pub fn integrate_bdf_adaptive_dense_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    config: &BdfConfig,
    adaptive: &AdaptiveStepConfig,
    sampling: &OutputSamplingPlan,
) -> DenseOutputResult<AdaptiveObservedIntegrationResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(
            CoreError::InvalidInput("invalid adaptive BDF integration input".into()).into(),
        );
    }
    let mut state = y0.to_vec();
    let mut history = BdfHistory::default();
    let mut counters = WorkCounters::default();
    let mut controller = AdaptiveControllerState::default();
    let mut collector =
        OutputCollector::new(sampling.output(), t_span, y0)?.with_max_step(adaptive.max_step);
    let mut hard_stops = HardStopCursor::new(sampling, t_span)?.with_max_step(adaptive.max_step);
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut internal_steps = 0_usize;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        let Some(next_h) = crate::output::adaptive_end_step(t, h, tf, adaptive.max_step)? else {
            break;
        };
        h = next_h;
        // A final piece that lands on tf is taken even below min_step.
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf)
            || 0.5 * h <= f64::MIN_POSITIVE
            || t + h == t
        {
            break;
        }
        let requested_h = h;
        let (trial_h, hard_stop_landing) = hard_stops.limit_step(t, requested_h, tf)?;
        let trial = match adaptive_bdf_trial(
            problem,
            t,
            &state,
            trial_h,
            config,
            adaptive,
            &history,
            &mut counters,
        ) {
            Ok(trial) => trial,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("failure kind checked");
                counters.rejected_steps += 1;
                record_adaptive_work_failure(&mut counters, failure);
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    config.order.value() + 1,
                    "bdf-trial-failed",
                    false,
                    Some(failure),
                );
                h = adaptive_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    requested_h,
                    trial_h,
                    f64::INFINITY,
                    config.order.value() + 1,
                    false,
                    hard_stop_landing,
                )?;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let accepted =
            trial.error_norm <= 1.0 && trial.y_new().iter().all(|value| value.is_finite());
        diagnostics.record(
            trial_h,
            trial.error_norm,
            trial.estimator_order,
            trial.estimator_id,
            accepted,
        );
        if accepted {
            let accepted_internal_steps = trial.accepted_internal_steps();
            let mut piece_t = t;
            let mut piece_state = state.clone();
            for report in &trial.accepted_reports {
                collector.accept_dense_interval(piece_t, report.t_new, &report.y_new, |theta| {
                    bdf_dense_output(report, &piece_state, theta).map_err(dense_output_core_error)
                })?;
                piece_t = report.t_new;
                piece_state = report.y_new.clone();
            }
            t += trial_h;
            state = trial.y_new().to_vec();
            history = trial.accepted_history;
            if hard_stops.consume_landing(t)? {
                // Values before a declared discontinuity must never define the
                // post-stop BDF polynomial.  Re-enter through the explicit
                // startup estimator on the next interval.
                history.clear();
            }
            counters.accepted_steps += accepted_internal_steps as u64;
            internal_steps += accepted_internal_steps;
            h = adaptive_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                trial.error_norm,
                trial.estimator_order,
                true,
                hard_stop_landing,
            )?;
        } else {
            counters.rejected_steps += 1;
            record_adaptive_work_failure(&mut counters, AdaptiveFailureKind::LocalError);
            h = adaptive_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                trial.error_norm,
                trial.estimator_order,
                false,
                hard_stop_landing,
            )?;
        }
    }
    dense_adaptive_result(t, tf, collector, counters, internal_steps, diagnostics)
}

fn dense_adaptive_result(
    t: f64,
    tf: f64,
    collector: OutputCollector,
    counters: WorkCounters,
    internal_steps: usize,
    diagnostics: AdaptiveRunDiagnostics,
) -> DenseOutputResult<AdaptiveObservedIntegrationResult> {
    let success = t >= tf;
    let observed = if success {
        let (t, y, output_clipped_steps) = collector.finish()?;
        ObservedIntegrationResult {
            t,
            y,
            success: true,
            message: "success".into(),
            counters,
            internal_steps,
            output_clipped_steps,
        }
    } else {
        let (times, states, output_clipped_steps) = collector.finish_partial();
        ObservedIntegrationResult {
            t: times,
            y: states,
            success: false,
            message: "maximum step count or minimum step reached".into(),
            counters,
            internal_steps,
            output_clipped_steps,
        }
    };
    Ok(AdaptiveObservedIntegrationResult {
        observed,
        diagnostics,
    })
}

fn adaptive_failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        _ => None,
    }
}

#[cfg(test)]
mod failure_preservation_tests {
    use std::cell::Cell;

    use rodas5p_core::{LinearMethod, PreconditionerKind};

    use super::*;
    use crate::{ControllerKind, scalar_linear_problem};

    #[test]
    fn post_work_dense_interpolation_error_preserves_work_diagnostics_and_prefix() {
        let (problem, y0) = scalar_linear_problem(-1.0, 1.0);
        let sampling = OutputSamplingPlan::dense(
            crate::OutputSchedule::new(vec![0.0, 0.025, 0.05, 0.075, 0.1]).unwrap(),
        );
        let linear = LinearSolverConfig {
            method: LinearMethod::Gmres,
            restart: 32,
            maxiter: 256,
            preconditioner: PreconditionerKind::None,
            ..LinearSolverConfig::default()
        };
        let adaptive = AdaptiveStepConfig {
            atol: 1.0,
            rtol: 1.0,
            initial_step: 0.1,
            min_step: 1.0e-12,
            max_step: 0.1,
            max_attempts: 10,
            safety: 0.9,
            min_factor: 0.2,
            max_factor: 5.0,
            reject_max_factor: 0.9,
            controller: ControllerKind::Integral,
        };
        let calls = Cell::new(0_usize);
        let result = integrate_sequential_matrix_free_adaptive_dense_observed_with(
            &problem,
            (0.0, 0.1),
            &y0,
            &linear,
            &adaptive,
            &sampling,
            MatrixFreeDenseStep::InnerForcing,
            |report, theta| {
                let call = calls.get();
                calls.set(call + 1);
                if call == 1 {
                    Err(CoreError::InvalidInput(
                        "forced post-work dense interpolation error".into(),
                    ))
                } else {
                    rodas5p_dense_output(report, theta).map_err(dense_output_core_error)
                }
            },
        )
        .unwrap();

        assert!(!result.observed.success);
        assert!(
            result
                .observed
                .message
                .contains("forced post-work dense interpolation error")
        );
        assert_eq!(result.observed.t, vec![0.0, 0.025]);
        assert_eq!(result.observed.y.len(), 2);
        assert_eq!(result.observed.internal_steps, 1);
        assert_eq!(result.diagnostics.attempts, 1);
        assert_eq!(result.diagnostics.accepted_macro_steps, 1);
        assert_eq!(result.observed.counters.accepted_steps, 1);
        assert!(result.observed.counters.forced_stage_solves > 0);
        assert!(result.diagnostics.is_structurally_consistent());
    }
}
