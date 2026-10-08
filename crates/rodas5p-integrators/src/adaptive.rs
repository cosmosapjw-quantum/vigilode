use rodas5p_core::{CoreError, CoreResult, error_scale, wrms};
use serde::{Deserialize, Serialize};

use crate::ObservedIntegrationResult;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerKind {
    #[default]
    Integral,
    Pi,
    /// Hairer's predictive (Gustafsson) controller, research node ALG02
    /// (`research/alg02_predictive_controller_20261008`). On an accepted,
    /// non-clipped step with `err > 0` it takes
    /// `min(f_I, f_P)` with `f_I = clamp(safety err^(-1/k))` and
    /// `f_P = clamp(safety (h / h_acc) (max(1e-2, err_prev) / err^2)^(1/k))`
    /// once a previous accepted step exists, and remembers `(h, err)`.
    /// `err = 0`, rejections and failures follow the production rule. The
    /// predictive term needs the step sizes, so it is applied only by the
    /// shared update [`adaptive_next_step_after_attempt`]; the history-free
    /// [`AdaptiveControllerState::propose_factor`] gives `f_I`. Opt-in.
    Predictive,
    /// [`ControllerKind::Predictive`] that does not grow the step (factor at
    /// most one) on the first acceptance after a rejected or failed attempt
    /// (Hairer's RODAS; audit F-078). Opt-in.
    PredictiveCapped,
}

impl ControllerKind {
    fn is_predictive(self) -> bool {
        matches!(self, Self::Predictive | Self::PredictiveCapped)
    }
}

/// The floor of the previous accepted error in the predictive term
/// (`err_acc = max(1e-2, err_prev)`, Hairer's RODAS/RADAU5).
pub const PREDICTIVE_ERROR_FLOOR: f64 = 1.0e-2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdaptiveEstimatorMetadata {
    pub name: &'static str,
    pub order: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdaptiveMethodMetadata {
    pub method: &'static str,
    pub estimator: AdaptiveEstimatorMetadata,
}

/// Method-bound adaptive metadata for the protected RODAS5P embedded pair.
pub const RODAS5P_ADAPTIVE_METHOD: AdaptiveMethodMetadata = AdaptiveMethodMetadata {
    method: "rodas5p",
    estimator: AdaptiveEstimatorMetadata {
        name: "rodas5p-embedded",
        order: 5,
    },
};

/// Compatibility alias for callers that have not yet adopted method metadata.
pub const RODAS5P_ESTIMATOR_ORDER: usize = RODAS5P_ADAPTIVE_METHOD.estimator.order;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptiveFailureKind {
    LocalError,
    LinearSolve,
    NonlinearSolve,
    NonFinite,
}

/// Reconcile the step ledger with the trial's final disposition. Step
/// kernels count their own endpoint verdict in `accepted_steps` /
/// `rejected_steps`; an outer gate (dense-error enforcement, the algebraic
/// error term, a non-finite state) can still reject the trial afterwards,
/// and the ledger must then show one more rejection, not an acceptance. The
/// work itself stays charged (audit 2026-09-30, AD-02: Enforce reported
/// 26 accepted and 0 rejected for 12 retained steps and 14 dense
/// rejections).
pub(crate) fn reconcile_outer_rejection(
    counters: &mut rodas5p_core::WorkCounters,
    endpoint_accepted: bool,
    final_accepted: bool,
) {
    if endpoint_accepted && !final_accepted {
        counters.accepted_steps = counters.accepted_steps.saturating_sub(1);
        counters.rejected_steps = counters.rejected_steps.saturating_add(1);
    }
}

pub(crate) fn record_adaptive_work_failure(
    counters: &mut rodas5p_core::WorkCounters,
    kind: AdaptiveFailureKind,
) {
    let target = match kind {
        AdaptiveFailureKind::LocalError => &mut counters.local_error_failures,
        AdaptiveFailureKind::LinearSolve => &mut counters.linear_solve_failures,
        AdaptiveFailureKind::NonlinearSolve => &mut counters.nonlinear_solve_failures,
        AdaptiveFailureKind::NonFinite => &mut counters.nonfinite_step_failures,
    };
    *target = target.saturating_add(1);
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdaptiveStepConfig {
    pub atol: f64,
    pub rtol: f64,
    pub initial_step: f64,
    pub min_step: f64,
    pub max_step: f64,
    pub max_attempts: usize,
    pub safety: f64,
    pub min_factor: f64,
    pub max_factor: f64,
    pub reject_max_factor: f64,
    pub controller: ControllerKind,
    /// How `max_step` binds the represented step (re-audit R4,
    /// R4-TIME-DEV-04); the default allows one clock resolution of slack.
    pub max_step_policy: crate::MaxStepPolicy,
}

impl Default for AdaptiveStepConfig {
    fn default() -> Self {
        Self {
            atol: 1.0e-9,
            rtol: 1.0e-6,
            initial_step: 1.0e-3,
            min_step: 1.0e-14,
            max_step: f64::MAX,
            max_attempts: 100_000,
            safety: 0.9,
            min_factor: 0.2,
            max_factor: 5.0,
            reject_max_factor: 0.9,
            controller: ControllerKind::Integral,
            max_step_policy: crate::MaxStepPolicy::AllowClockResolutionSlack,
        }
    }
}

impl AdaptiveStepConfig {
    /// `max_step` with its policy, for the represented clock.
    pub(crate) fn step_cap(&self) -> crate::output::StepCap {
        crate::output::StepCap {
            max: self.max_step,
            policy: self.max_step_policy,
        }
    }

    pub fn validate(&self) -> CoreResult<()> {
        if !(self.atol >= 0.0 && self.atol.is_finite()) {
            return Err(CoreError::InvalidInput(
                "adaptive atol must be finite and nonnegative".into(),
            ));
        }
        if !(self.rtol >= 0.0 && self.rtol.is_finite()) {
            return Err(CoreError::InvalidInput(
                "adaptive rtol must be finite and nonnegative".into(),
            ));
        }
        if self.atol == 0.0 && self.rtol == 0.0 {
            return Err(CoreError::InvalidInput(
                "adaptive atol and rtol cannot both be zero".into(),
            ));
        }
        if !(self.initial_step > 0.0 && self.initial_step.is_finite()) {
            return Err(CoreError::InvalidInput(
                "adaptive initial step must be finite and positive".into(),
            ));
        }
        if !(self.min_step > 0.0 && self.min_step.is_finite()) {
            return Err(CoreError::InvalidInput(
                "adaptive minimum step must be finite and positive".into(),
            ));
        }
        if !(self.max_step >= self.min_step && self.max_step.is_finite()) {
            return Err(CoreError::InvalidInput(
                "adaptive maximum step must be finite and at least the minimum step".into(),
            ));
        }
        if !(self.initial_step >= self.min_step && self.initial_step <= self.max_step) {
            return Err(CoreError::InvalidInput(
                "adaptive initial step must lie within the configured step bounds".into(),
            ));
        }
        if self.max_attempts == 0 {
            return Err(CoreError::InvalidInput(
                "adaptive attempt budget must be positive".into(),
            ));
        }
        if !(self.safety > 0.0 && self.safety <= 1.0 && self.safety.is_finite()) {
            return Err(CoreError::InvalidInput(
                "adaptive safety factor must lie in (0, 1]".into(),
            ));
        }
        if !(self.min_factor > 0.0 && self.min_factor <= 1.0 && self.min_factor.is_finite()) {
            return Err(CoreError::InvalidInput(
                "adaptive minimum factor must lie in (0, 1]".into(),
            ));
        }
        if !(self.max_factor >= 1.0
            && self.max_factor >= self.min_factor
            && self.max_factor.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "adaptive maximum factor must be finite and at least one".into(),
            ));
        }
        if !(self.reject_max_factor >= self.min_factor
            && self.reject_max_factor <= 1.0
            && self.reject_max_factor.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "adaptive rejection factor cap must lie between the minimum factor and one".into(),
            ));
        }
        Ok(())
    }

    pub fn legacy_rodas(
        atol: f64,
        rtol: f64,
        initial_step: f64,
        max_attempts: usize,
        max_step: f64,
    ) -> CoreResult<Self> {
        let config = Self {
            atol,
            rtol,
            initial_step,
            min_step: f64::MIN_POSITIVE,
            max_step,
            max_attempts,
            safety: 0.9,
            min_factor: 0.2,
            max_factor: 5.0,
            reject_max_factor: 0.9,
            controller: ControllerKind::Integral,
            max_step_policy: crate::MaxStepPolicy::AllowClockResolutionSlack,
        };
        config.validate()?;
        Ok(config)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct AdaptiveControllerState {
    previous_accepted_error: Option<f64>,
    /// The represented trial step of the last attempt when it was rejected
    /// (re-audit R4, TIME-DEV-02): the next attempt from the same time must
    /// be strictly shorter, see [`crate::output::adaptive_end_step`].
    #[serde(skip)]
    last_rejected_trial: Option<f64>,
    /// The trial step of the last accepted attempt that entered the history
    /// (`h_acc` of the predictive kinds; research node ALG02). Kept only by
    /// the predictive kinds and never serialized.
    #[serde(skip)]
    last_accepted_step: Option<f64>,
}

impl AdaptiveControllerState {
    pub fn previous_accepted_error(&self) -> Option<f64> {
        self.previous_accepted_error
    }

    pub fn last_rejected_trial(&self) -> Option<f64> {
        self.last_rejected_trial
    }

    /// `h_acc` of the predictive kinds: the trial step of the last accepted
    /// attempt that entered the history (research node ALG02).
    pub fn last_accepted_step(&self) -> Option<f64> {
        self.last_accepted_step
    }

    /// A controller state after a rejected trial of `rejected`, for the
    /// landing property test of speed research node SPD01.
    #[cfg(test)]
    pub(crate) fn with_last_rejected_trial(rejected: Option<f64>) -> Self {
        Self {
            last_rejected_trial: rejected,
            ..Self::default()
        }
    }

    pub fn propose_factor(
        &self,
        config: &AdaptiveStepConfig,
        error: f64,
        estimator_order: usize,
        accepted: bool,
    ) -> CoreResult<f64> {
        config.validate()?;
        self.propose_factor_prevalidated(config, error, estimator_order, accepted)
    }

    /// [`Self::propose_factor`] without the configuration check: the caller
    /// must have validated `config` (speed research node SPD01; the fast
    /// drivers validate once at entry). Same arithmetic, same result.
    pub(crate) fn propose_factor_prevalidated(
        &self,
        config: &AdaptiveStepConfig,
        error: f64,
        estimator_order: usize,
        accepted: bool,
    ) -> CoreResult<f64> {
        if estimator_order == 0 {
            return Err(CoreError::InvalidInput(
                "adaptive estimator order must be positive".into(),
            ));
        }
        if !(error >= 0.0 && error.is_finite()) {
            return Err(CoreError::NonFinite(
                "adaptive error estimate must be finite and nonnegative".into(),
            ));
        }
        if error == 0.0 {
            return Ok(if accepted {
                config.max_factor
            } else {
                config.reject_max_factor
            });
        }
        let order = estimator_order as f64;
        let raw = match (config.controller, accepted, self.previous_accepted_error) {
            (ControllerKind::Pi, true, Some(previous)) if previous > 0.0 => {
                config.safety * error.powf(-0.7 / order) * previous.powf(0.4 / order)
            }
            _ => config.safety * error.powf(-1.0 / order),
        };
        if !raw.is_finite() || raw <= 0.0 {
            return Err(CoreError::NonFinite(
                "adaptive controller produced a non-finite factor".into(),
            ));
        }
        Ok(if accepted {
            raw.clamp(config.min_factor, config.max_factor)
        } else {
            raw.clamp(config.min_factor, config.reject_max_factor)
        })
    }

    /// The accepted-step factor of the predictive kinds (research node
    /// ALG02) for the accepted trial step `h`; `previous_failed` is whether
    /// the attempt before this one was rejected or failed. Must be called
    /// before the step enters the history. `f_I` is the production
    /// accepted-step factor (including `err = 0 -> max_factor`).
    fn predictive_factor(
        &self,
        config: &AdaptiveStepConfig,
        h: f64,
        error: f64,
        estimator_order: usize,
        previous_failed: bool,
        prevalidated: bool,
    ) -> CoreResult<f64> {
        let integral = if prevalidated {
            self.propose_factor_prevalidated(config, error, estimator_order, true)?
        } else {
            self.propose_factor(config, error, estimator_order, true)?
        };
        if error == 0.0 {
            return Ok(integral);
        }
        let order = estimator_order as f64;
        let mut factor = integral;
        if let (Some(h_acc), Some(err_prev)) =
            (self.last_accepted_step, self.previous_accepted_error)
        {
            let err_acc = err_prev.max(PREDICTIVE_ERROR_FLOOR);
            let raw = config.safety * (h / h_acc) * (err_acc / (error * error)).powf(1.0 / order);
            if raw.is_nan() || raw <= 0.0 {
                return Err(CoreError::NonFinite(
                    "adaptive predictive controller produced an invalid factor".into(),
                ));
            }
            factor = factor.min(raw.clamp(config.min_factor, config.max_factor));
        }
        if config.controller == ControllerKind::PredictiveCapped && previous_failed {
            factor = factor.min(1.0);
        }
        Ok(factor)
    }

    pub fn record_acceptance(&mut self, error: f64) -> CoreResult<()> {
        if !(error >= 0.0 && error.is_finite()) {
            return Err(CoreError::NonFinite(
                "accepted adaptive error must be finite and nonnegative".into(),
            ));
        }
        self.previous_accepted_error = Some(error.max(1.0e-16));
        Ok(())
    }

    pub fn record_rejection(&mut self, error: f64) -> CoreResult<()> {
        if !(error >= 0.0 && error.is_finite()) {
            return Err(CoreError::NonFinite(
                "rejected adaptive error must be finite and nonnegative".into(),
            ));
        }
        Ok(())
    }
}

/// A clipped trial at least this fraction of the remembered request is an
/// informative error sample; a shorter landing (a sliver) is not.
pub const CLIPPED_SAMPLE_INFORMATIVE_RATIO: f64 = 0.5;

/// Update a method-bound controller after one attempted step.
///
/// The factor is proposed before an accepted error is recorded, so a PI
/// controller combines the current error with the previous accepted one.
///
/// A forced output landing remembers the pre-clip request.  A sliver landing
/// (`trial_h < CLIPPED_SAMPLE_INFORMATIVE_RATIO * requested_h`) returns that
/// request unchanged and leaves PI history untouched.  An
/// informative clipped sample enters the history; if it predicts rejection of
/// the remembered request it lowers the request, never below the accepted
/// trial, and otherwise it may only raise it.  Rejections always scale the
/// actual trial.
#[allow(clippy::too_many_arguments)]
pub fn adaptive_next_step_after_attempt(
    controller: &mut AdaptiveControllerState,
    config: &AdaptiveStepConfig,
    requested_h: f64,
    trial_h: f64,
    error: f64,
    estimator_order: usize,
    accepted: bool,
    forced_output_clipped: bool,
) -> CoreResult<f64> {
    next_step_after_attempt_impl(
        controller,
        config,
        requested_h,
        trial_h,
        error,
        estimator_order,
        accepted,
        forced_output_clipped,
        false,
    )
}

/// [`adaptive_next_step_after_attempt`] for a caller that has validated
/// `config` once (speed research node SPD01): the configuration check inside
/// the factor proposal is skipped; the arithmetic is the same.
#[allow(clippy::too_many_arguments)]
pub(crate) fn adaptive_next_step_after_attempt_prevalidated(
    controller: &mut AdaptiveControllerState,
    config: &AdaptiveStepConfig,
    requested_h: f64,
    trial_h: f64,
    error: f64,
    estimator_order: usize,
    accepted: bool,
    forced_output_clipped: bool,
) -> CoreResult<f64> {
    next_step_after_attempt_impl(
        controller,
        config,
        requested_h,
        trial_h,
        error,
        estimator_order,
        accepted,
        forced_output_clipped,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn next_step_after_attempt_impl(
    controller: &mut AdaptiveControllerState,
    config: &AdaptiveStepConfig,
    requested_h: f64,
    trial_h: f64,
    error: f64,
    estimator_order: usize,
    accepted: bool,
    forced_output_clipped: bool,
    prevalidated: bool,
) -> CoreResult<f64> {
    let previous_failed = controller.last_rejected_trial.is_some();
    controller.last_rejected_trial = (!accepted).then_some(trial_h);
    if accepted {
        let predictive = config.controller.is_predictive();
        let factor = if predictive {
            controller.predictive_factor(
                config,
                trial_h,
                error,
                estimator_order,
                previous_failed,
                prevalidated,
            )?
        } else if prevalidated {
            controller.propose_factor_prevalidated(config, error, estimator_order, true)?
        } else {
            controller.propose_factor(config, error, estimator_order, true)?
        };
        if !forced_output_clipped {
            controller.record_acceptance(error)?;
            if predictive {
                controller.last_accepted_step = Some(trial_h);
            }
            return Ok(trial_h * factor);
        }
        let ratio = trial_h / requested_h;
        if ratio < CLIPPED_SAMPLE_INFORMATIVE_RATIO {
            return Ok(requested_h);
        }
        controller.record_acceptance(error)?;
        if predictive {
            controller.last_accepted_step = Some(trial_h);
        }
        let candidate = trial_h * factor;
        let predicted = error * ratio.powf(-(estimator_order as f64));
        if predicted > 1.0 {
            return Ok(candidate.max(trial_h).min(requested_h));
        }
        return Ok(requested_h.max(candidate));
    }
    if error.is_finite() {
        controller.record_rejection(error)?;
        let factor = if prevalidated {
            controller.propose_factor_prevalidated(
                config,
                error.max(1.0e-16),
                estimator_order,
                false,
            )?
        } else {
            controller.propose_factor(config, error.max(1.0e-16), estimator_order, false)?
        };
        Ok(trial_h * factor)
    } else {
        Ok(trial_h * config.min_factor)
    }
}

/// [`rodas_next_step_after_attempt`] for a caller that has validated
/// `config` once (speed research node SPD01).
pub(crate) fn rodas_next_step_after_attempt_prevalidated(
    controller: &mut AdaptiveControllerState,
    config: &AdaptiveStepConfig,
    requested_h: f64,
    trial_h: f64,
    error: f64,
    accepted: bool,
    forced_output_clipped: bool,
) -> CoreResult<f64> {
    adaptive_next_step_after_attempt_prevalidated(
        controller,
        config,
        requested_h,
        trial_h,
        error,
        RODAS5P_ADAPTIVE_METHOD.estimator.order,
        accepted,
        forced_output_clipped,
    )
}

/// Compatibility wrapper for the protected RODAS5P embedded pair.
pub fn rodas_next_step_after_attempt(
    controller: &mut AdaptiveControllerState,
    config: &AdaptiveStepConfig,
    requested_h: f64,
    trial_h: f64,
    error: f64,
    accepted: bool,
    forced_output_clipped: bool,
) -> CoreResult<f64> {
    adaptive_next_step_after_attempt(
        controller,
        config,
        requested_h,
        trial_h,
        error,
        RODAS5P_ADAPTIVE_METHOD.estimator.order,
        accepted,
        forced_output_clipped,
    )
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StepDoublingEstimate {
    pub method_order: usize,
    pub estimator_order: usize,
    pub error_vector: Vec<f64>,
    pub error_norm: f64,
    /// The represented fine intervals `(h1, h2)` the estimate was formed
    /// for; `None` for the equal-halves rule (re-audit R4, TIME-DEV-02).
    pub halves: Option<(f64, f64)>,
    /// `(1 - eta) / eta` with `eta = theta^(p+1) + (1 - theta)^(p+1)`,
    /// `theta = h1 / (h1 + h2)`: the divisor of `fine - coarse`. Equal
    /// halves give `2^p - 1`.
    pub divisor: f64,
}

/// `(1 - eta) / eta` for the represented halves `(h1, h2)` of a step of
/// method order `p` (re-audit R4, R4-TIME-DEV-02): under the leading-order
/// local error model `C H^(p+1)` with one `C` on both halves, the coarse
/// error is `C H^(p+1)`, the fine one `eta C H^(p+1)`, so the fine error is
/// `(fine - coarse) eta / (1 - eta)`. Equal halves give exactly `2^p - 1`;
/// the R3 split of a 3-ULP step into 2 + 1 ULPs gives `eta = 5/9` for
/// Radau1 and a divisor 4/5, not 1. An asymptotic estimate, not a
/// certificate. Halves that are not positive and finite fail closed.
pub fn step_doubling_divisor(h1: f64, h2: f64, method_order: usize) -> CoreResult<f64> {
    if method_order == 0 {
        return Err(CoreError::InvalidInput(
            "step-doubling method order must be positive".into(),
        ));
    }
    if !(h1.is_finite() && h2.is_finite() && h1 > 0.0 && h2 > 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "time resolution: step-doubling halves ({h1:e}, {h2:e}) must be positive and finite"
        )));
    }
    if h1 == h2 {
        return Ok(2.0_f64.powi(method_order as i32) - 1.0);
    }
    let total = h1 + h2;
    let theta = h1 / total;
    let rest = h2 / total;
    let exponent = method_order as i32 + 1;
    let eta = theta.powi(exponent) + rest.powi(exponent);
    let divisor = (1.0 - eta) / eta;
    if !(divisor.is_finite() && divisor > 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "time resolution: step-doubling halves ({h1:e}, {h2:e}) give no finite Richardson divisor"
        )));
    }
    Ok(divisor)
}

/// [`step_doubling_wrms_error`] for represented halves `(h1, h2)` of the
/// coarse step, with the geometry-aware [`step_doubling_divisor`]. Equal
/// halves are bit for bit the equal-halves rule.
#[allow(clippy::too_many_arguments)]
pub fn step_doubling_wrms_error_for_halves(
    old_state: &[f64],
    coarse_state: &[f64],
    fine_state: &[f64],
    atol: f64,
    rtol: f64,
    method_order: usize,
    h1: f64,
    h2: f64,
) -> CoreResult<StepDoublingEstimate> {
    let divisor = step_doubling_divisor(h1, h2, method_order)?;
    let mut estimate = step_doubling_estimate(
        old_state,
        coarse_state,
        fine_state,
        atol,
        rtol,
        method_order,
        divisor,
    )?;
    estimate.halves = Some((h1, h2));
    Ok(estimate)
}

pub fn step_doubling_wrms_error(
    old_state: &[f64],
    coarse_state: &[f64],
    fine_state: &[f64],
    atol: f64,
    rtol: f64,
    method_order: usize,
) -> CoreResult<StepDoublingEstimate> {
    if method_order == 0 {
        return Err(CoreError::InvalidInput(
            "step-doubling method order must be positive".into(),
        ));
    }
    if old_state.len() != coarse_state.len()
        || old_state.len() != fine_state.len()
        || old_state.is_empty()
    {
        return Err(CoreError::Dimension(
            "step-doubling state shape mismatch".into(),
        ));
    }
    if !old_state
        .iter()
        .chain(coarse_state)
        .chain(fine_state)
        .all(|value| value.is_finite())
    {
        return Err(CoreError::NonFinite(
            "step-doubling state contains NaN/Inf".into(),
        ));
    }
    step_doubling_estimate(
        old_state,
        coarse_state,
        fine_state,
        atol,
        rtol,
        method_order,
        2.0_f64.powi(method_order as i32) - 1.0,
    )
}

pub(crate) fn step_doubling_estimate(
    old_state: &[f64],
    coarse_state: &[f64],
    fine_state: &[f64],
    atol: f64,
    rtol: f64,
    method_order: usize,
    denominator: f64,
) -> CoreResult<StepDoublingEstimate> {
    if method_order == 0 {
        return Err(CoreError::InvalidInput(
            "step-doubling method order must be positive".into(),
        ));
    }
    if old_state.len() != coarse_state.len()
        || old_state.len() != fine_state.len()
        || old_state.is_empty()
    {
        return Err(CoreError::Dimension(
            "step-doubling state shape mismatch".into(),
        ));
    }
    if !old_state
        .iter()
        .chain(coarse_state)
        .chain(fine_state)
        .all(|value| value.is_finite())
    {
        return Err(CoreError::NonFinite(
            "step-doubling state contains NaN/Inf".into(),
        ));
    }
    let error_vector = fine_state
        .iter()
        .zip(coarse_state)
        .map(|(fine, coarse)| (fine - coarse) / denominator)
        .collect::<Vec<_>>();
    let scale = error_scale(old_state, fine_state, &[atol], rtol)?;
    let error_norm = wrms(&error_vector, &scale)?;
    Ok(StepDoublingEstimate {
        method_order,
        estimator_order: method_order + 1,
        error_vector,
        error_norm,
        halves: None,
        divisor: denominator,
    })
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveRunDiagnostics {
    pub attempts: usize,
    pub accepted_macro_steps: usize,
    pub rejected_macro_steps: usize,
    pub accepted_step_sizes: Vec<f64>,
    pub rejected_step_sizes: Vec<f64>,
    pub error_norms: Vec<f64>,
    pub estimator_orders: Vec<usize>,
    pub estimator_ids: Vec<String>,
    /// One cause entry per attempt; accepted attempts use `None`.
    pub failure_kinds: Vec<Option<AdaptiveFailureKind>>,
    pub local_error_failures: usize,
    pub linear_solve_failures: usize,
    pub nonlinear_solve_failures: usize,
    pub non_finite_failures: usize,
    pub fallback_steps: usize,
}

impl AdaptiveRunDiagnostics {
    /// Whether all per-attempt vectors and typed failure counts form one
    /// complete, internally consistent adaptive ledger.
    pub fn is_structurally_consistent(&self) -> bool {
        self.attempts == self.error_norms.len()
            && self.attempts == self.estimator_orders.len()
            && self.attempts == self.estimator_ids.len()
            && self.attempts == self.failure_kinds.len()
            && self.attempts == self.accepted_macro_steps + self.rejected_macro_steps
            && self.accepted_macro_steps == self.accepted_step_sizes.len()
            && self.rejected_macro_steps == self.rejected_step_sizes.len()
            && self.rejected_macro_steps
                == self.local_error_failures
                    + self.linear_solve_failures
                    + self.nonlinear_solve_failures
                    + self.non_finite_failures
    }

    /// Append an independently restarted segment while rejecting malformed
    /// diagnostics and integer overflow.
    pub fn checked_accumulate(&mut self, other: &Self) -> CoreResult<()> {
        if !self.is_structurally_consistent() || !other.is_structurally_consistent() {
            return Err(CoreError::InvalidInput(
                "cannot accumulate structurally inconsistent adaptive diagnostics".into(),
            ));
        }
        let mut next = self.clone();
        macro_rules! checked_add {
            ($($field:ident),* $(,)?) => {
                $(next.$field = next.$field.checked_add(other.$field).ok_or_else(|| {
                    CoreError::InvalidInput("adaptive diagnostic counter overflow".into())
                })?;)*
            };
        }
        checked_add!(
            attempts,
            accepted_macro_steps,
            rejected_macro_steps,
            local_error_failures,
            linear_solve_failures,
            nonlinear_solve_failures,
            non_finite_failures,
            fallback_steps,
        );
        next.accepted_step_sizes
            .extend_from_slice(&other.accepted_step_sizes);
        next.rejected_step_sizes
            .extend_from_slice(&other.rejected_step_sizes);
        next.error_norms.extend_from_slice(&other.error_norms);
        next.estimator_orders
            .extend_from_slice(&other.estimator_orders);
        next.estimator_ids.extend_from_slice(&other.estimator_ids);
        next.failure_kinds.extend_from_slice(&other.failure_kinds);
        if !next.is_structurally_consistent() {
            return Err(CoreError::InvalidInput(
                "accumulated adaptive diagnostics are inconsistent".into(),
            ));
        }
        *self = next;
        Ok(())
    }

    pub(crate) fn record(
        &mut self,
        step: f64,
        error: f64,
        estimator_order: usize,
        estimator_id: &str,
        accepted: bool,
    ) {
        self.record_with_failure(step, error, estimator_order, estimator_id, accepted, None);
    }

    pub(crate) fn record_with_failure(
        &mut self,
        step: f64,
        error: f64,
        estimator_order: usize,
        estimator_id: &str,
        accepted: bool,
        failure: Option<AdaptiveFailureKind>,
    ) {
        self.attempts += 1;
        self.error_norms.push(error);
        self.estimator_orders.push(estimator_order);
        self.estimator_ids.push(estimator_id.to_owned());
        let failure = if accepted {
            None
        } else {
            failure.or(Some(AdaptiveFailureKind::LocalError))
        };
        self.failure_kinds.push(failure);
        match failure {
            Some(AdaptiveFailureKind::LocalError) => self.local_error_failures += 1,
            Some(AdaptiveFailureKind::LinearSolve) => self.linear_solve_failures += 1,
            Some(AdaptiveFailureKind::NonlinearSolve) => self.nonlinear_solve_failures += 1,
            Some(AdaptiveFailureKind::NonFinite) => self.non_finite_failures += 1,
            None => {}
        }
        if accepted {
            self.accepted_macro_steps += 1;
            self.accepted_step_sizes.push(step);
        } else {
            self.rejected_macro_steps += 1;
            self.rejected_step_sizes.push(step);
        }
    }
}

#[derive(Clone, Debug)]
pub struct AdaptiveObservedIntegrationResult {
    pub observed: ObservedIntegrationResult,
    pub diagnostics: AdaptiveRunDiagnostics,
}

#[cfg(test)]
mod method_metadata_tests {
    use super::*;

    #[test]
    fn rodas_controller_order_comes_from_method_estimator_metadata() {
        assert_eq!(RODAS5P_ADAPTIVE_METHOD.method, "rodas5p");
        assert_eq!(RODAS5P_ADAPTIVE_METHOD.estimator.name, "rodas5p-embedded");
        assert_eq!(RODAS5P_ADAPTIVE_METHOD.estimator.order, 5);
        assert_eq!(
            RODAS5P_ESTIMATOR_ORDER,
            RODAS5P_ADAPTIVE_METHOD.estimator.order
        );
    }
}

/// Research node ALG02 (`research/alg02_predictive_controller_20261008`):
/// the predictive kinds on hand-computed cases, and the production kinds
/// unchanged.
#[cfg(test)]
mod predictive_controller_tests {
    use super::*;

    const ORDER: usize = RODAS5P_ESTIMATOR_ORDER;

    fn config(controller: ControllerKind) -> AdaptiveStepConfig {
        AdaptiveStepConfig {
            controller,
            ..AdaptiveStepConfig::default()
        }
    }

    /// One attempt through the shared update; `requested == trial` unless
    /// the step was clipped.
    fn step(
        state: &mut AdaptiveControllerState,
        config: &AdaptiveStepConfig,
        requested: f64,
        trial: f64,
        error: f64,
        accepted: bool,
        clipped: bool,
    ) -> f64 {
        adaptive_next_step_after_attempt(
            state, config, requested, trial, error, ORDER, accepted, clipped,
        )
        .unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1.0e-14 * b.abs()
    }

    /// The pre-ALG02 arithmetic of the shared update without clipping,
    /// written out: accepted `clamp(s err^(-1/k))` (PI: `s err^(-0.7/k)
    /// prev^(0.4/k)`), `err = 0 -> max_factor`, rejected `clamp(s
    /// max(err, 1e-16)^(-1/k), min, reject_max)`, non-finite `min_factor`.
    fn production(
        kind: ControllerKind,
        previous: Option<f64>,
        trial: f64,
        error: f64,
        accepted: bool,
    ) -> f64 {
        let c = config(kind);
        let k = ORDER as f64;
        if !error.is_finite() {
            return trial * c.min_factor;
        }
        let e = if accepted { error } else { error.max(1.0e-16) };
        if e == 0.0 {
            return trial * c.max_factor;
        }
        let raw = match (kind, accepted, previous) {
            (ControllerKind::Pi, true, Some(p)) if p > 0.0 => {
                c.safety * e.powf(-0.7 / k) * p.powf(0.4 / k)
            }
            _ => c.safety * e.powf(-1.0 / k),
        };
        let cap = if accepted {
            c.max_factor
        } else {
            c.reject_max_factor
        };
        trial * raw.clamp(c.min_factor, cap)
    }

    /// A fixed attempt sequence: (trial, error, accepted).
    const SEQUENCE: [(f64, f64, bool); 9] = [
        (1.0e-3, 0.5, true),
        (1.1e-3, 3.0, false),
        (0.8e-3, 0.02, true),
        (2.0e-3, f64::INFINITY, false),
        (0.4e-3, 0.0, true),
        (2.0e-3, 0.9, true),
        (2.1e-3, 1.0e-7, true),
        (1.0e-2, 40.0, false),
        (2.0e-3, 0.3, true),
    ];

    #[test]
    fn integral_and_pi_are_unchanged_bit_for_bit() {
        for kind in [ControllerKind::Integral, ControllerKind::Pi] {
            let c = config(kind);
            let mut state = AdaptiveControllerState::default();
            let mut previous: Option<f64> = None;
            for (trial, error, accepted) in SEQUENCE {
                let expected = production(kind, previous, trial, error, accepted);
                let next = step(&mut state, &c, trial, trial, error, accepted, false);
                assert_eq!(
                    next.to_bits(),
                    expected.to_bits(),
                    "{kind:?} {trial} {error}"
                );
                if accepted {
                    previous = Some(error.max(1.0e-16));
                }
                assert_eq!(state.previous_accepted_error(), previous);
                assert_eq!(state.last_accepted_step(), None);
            }
            assert_eq!(
                serde_json::to_value(&state).unwrap(),
                serde_json::json!({"previous_accepted_error": previous.unwrap()})
            );
        }
    }

    #[test]
    fn predictive_kinds_serialize_like_the_production_state() {
        for kind in [ControllerKind::Predictive, ControllerKind::PredictiveCapped] {
            let c = config(kind);
            let mut state = AdaptiveControllerState::default();
            for (trial, error, accepted) in SEQUENCE {
                step(&mut state, &c, trial, trial, error, accepted, false);
            }
            assert_eq!(state.last_accepted_step(), Some(2.0e-3));
            assert_eq!(
                serde_json::to_value(&state).unwrap(),
                serde_json::json!({"previous_accepted_error": 0.3})
            );
        }
        assert_eq!(
            serde_json::to_value(ControllerKind::Predictive).unwrap(),
            serde_json::json!("predictive")
        );
        assert_eq!(
            serde_json::to_value(ControllerKind::PredictiveCapped).unwrap(),
            serde_json::json!("predictive-capped")
        );
    }

    #[test]
    fn first_acceptance_is_the_integral_factor() {
        let mut state = AdaptiveControllerState::default();
        let next = step(
            &mut state,
            &config(ControllerKind::Predictive),
            0.1,
            0.1,
            0.5,
            true,
            false,
        );
        // 0.9 * 0.5^(-1/5)
        assert!(close(next, 0.1 * 1.033_828_519_497_331_6), "{next}");
        assert_eq!(
            next.to_bits(),
            production(ControllerKind::Integral, None, 0.1, 0.5, true).to_bits()
        );
        assert_eq!(state.last_accepted_step(), Some(0.1));
        assert_eq!(state.previous_accepted_error(), Some(0.5));
    }

    #[test]
    fn predictive_factor_on_hand_computed_cases() {
        let c = config(ControllerKind::Predictive);
        // (h_acc, err_prev, h, err, expected factor)
        let cases = [
            // f_I = 0.9 * 0.25^(-1/5) = 1.18756 < f_P = 0.9 * 2 * 8^(1/5) = 2.72829.
            (0.1, 0.5, 0.2, 0.25, 1.187_557_119_695_604_7),
            // f_P = 0.9 * (0.05 / 0.25)^(1/5) = 0.65230 < f_I = 1.03383.
            (0.1, 0.05, 0.1, 0.5, 0.652_301_697_309_926),
            // err_acc floored at 1e-2: 0.9 * (0.01 / 0.25)^(1/5) = 0.47278
            // (without the floor 0.9 * 0.0004^(1/5) = 0.188 -> 0.2).
            (0.1, 1.0e-4, 0.1, 0.5, 0.472_775_004_792_678_1),
            // f_P = 0.9 * 0.1 * (1 / 0.25)^(1/5) = 0.1188 clamps to 0.2.
            (1.0, 1.0, 0.1, 0.5, 0.2),
            // Both terms clamp at 5: 0.9 * 1e-8^(-1/5) = 35.8, f_P larger.
            (0.1, 0.5, 0.1, 1.0e-8, 5.0),
        ];
        for (h_acc, err_prev, h, err, expected) in cases {
            let mut state = AdaptiveControllerState::default();
            step(&mut state, &c, h_acc, h_acc, err_prev, true, false);
            assert_eq!(state.last_accepted_step(), Some(h_acc));
            let next = step(&mut state, &c, h, h, err, true, false);
            assert!(
                close(next / h, expected),
                "{h_acc} {err_prev} {h} {err}: {}",
                next / h
            );
            assert_eq!(state.last_accepted_step(), Some(h));
            assert_eq!(state.previous_accepted_error(), Some(err));
        }
    }

    #[test]
    fn rejections_failures_and_zero_error_follow_the_production_rule() {
        for kind in [ControllerKind::Predictive, ControllerKind::PredictiveCapped] {
            let c = config(kind);
            let mut state = AdaptiveControllerState::default();
            step(&mut state, &c, 0.1, 0.1, 0.5, true, false);
            for (trial, error) in [(0.12, 3.0), (0.05, 1.0e3), (0.02, f64::INFINITY)] {
                let next = step(&mut state, &c, trial, trial, error, false, false);
                let expected = production(ControllerKind::Integral, None, trial, error, false);
                assert_eq!(next.to_bits(), expected.to_bits());
                // Rejections leave (h_acc, err_acc) alone.
                assert_eq!(state.last_accepted_step(), Some(0.1));
                assert_eq!(state.previous_accepted_error(), Some(0.5));
            }
            // err = 0 after a failure: max_factor, also for the capped kind.
            let next = step(&mut state, &c, 0.01, 0.01, 0.0, true, false);
            assert_eq!(next, 0.01 * c.max_factor);
            assert_eq!(state.last_accepted_step(), Some(0.01));
            assert_eq!(state.previous_accepted_error(), Some(1.0e-16));
        }
    }

    #[test]
    fn the_cap_holds_the_first_acceptance_after_a_rejection_or_failure() {
        // After accepting (h 0.1, err 0.5): at h = 0.05 with err = 0.01,
        // f_I = 0.9 * 0.01^(-1/5) = 2.26070 and f_P = 0.45 * 5000^(1/5) =
        // 2.47176, so the uncapped factor is 2.26070.
        for failure in [3.0, f64::INFINITY] {
            for kind in [ControllerKind::Predictive, ControllerKind::PredictiveCapped] {
                let c = config(kind);
                let mut state = AdaptiveControllerState::default();
                step(&mut state, &c, 0.1, 0.1, 0.5, true, false);
                step(&mut state, &c, 0.11, 0.11, failure, false, false);
                let next = step(&mut state, &c, 0.05, 0.05, 0.01, true, false);
                let expected = if kind == ControllerKind::PredictiveCapped {
                    1.0
                } else {
                    2.260_697_788_358_622_3
                };
                assert!(close(next / 0.05, expected), "{kind:?} {failure}: {next}");
                // The cap binds only the first acceptance after the failure.
                let after = step(&mut state, &c, next, next, 0.01, true, false);
                let f_i = 2.260_697_788_358_622_3;
                // f_P = 0.9 * (h / h_acc) * (0.01 / 1e-4)^(1/5) > f_I here.
                assert!(close(after / next, f_i), "{kind:?}: {}", after / next);
            }
        }
        // Without a preceding rejection the capped kind may grow.
        let c = config(ControllerKind::PredictiveCapped);
        let mut state = AdaptiveControllerState::default();
        let next = step(&mut state, &c, 0.1, 0.1, 0.01, true, false);
        assert!(close(next / 0.1, 2.260_697_788_358_622_3));
    }

    #[test]
    fn clipped_samples_update_the_history_only_when_informative() {
        let c = config(ControllerKind::PredictiveCapped);
        // A sliver (trial < 0.5 request): the request comes back and
        // neither h_acc nor err_acc moves.
        let mut state = AdaptiveControllerState::default();
        step(&mut state, &c, 0.1, 0.1, 0.5, true, false);
        let next = step(&mut state, &c, 0.4, 0.01, 0.2, true, true);
        assert_eq!(next, 0.4);
        assert_eq!(state.last_accepted_step(), Some(0.1));
        assert_eq!(state.previous_accepted_error(), Some(0.5));
        // An informative clipped sample enters the history with its trial.
        let next = step(&mut state, &c, 0.4, 0.3, 0.2, true, true);
        assert_eq!(state.last_accepted_step(), Some(0.3));
        assert_eq!(state.previous_accepted_error(), Some(0.2));
        // f_I = 0.9 * 0.2^(-1/5) = 1.24177, f_P = 0.9 * 3 * (0.5 / 0.04)^(1/5)
        // = 4.40585; candidate 0.3 * 1.24177 = 0.37253 < request 0.4 and the
        // sample predicts 0.2 * (0.75)^(-5) = 0.843 <= 1: the request stays.
        assert_eq!(next, 0.4);
        // The clipped trial is not a rejection: no cap on the next step.
        let after = step(&mut state, &c, 0.4, 0.4, 0.2, true, false);
        // f_I = 1.24177, f_P = 0.9 * (0.4 / 0.3) * (0.2 / 0.04)^(1/5) = 1.65569.
        assert!(close(after / 0.4, 0.9 * 0.2_f64.powf(-0.2)), "{after}");
    }

    #[test]
    fn history_free_proposal_of_the_predictive_kinds_is_the_integral_factor() {
        let mut state = AdaptiveControllerState::default();
        state.record_acceptance(0.05).unwrap();
        for kind in [ControllerKind::Predictive, ControllerKind::PredictiveCapped] {
            for accepted in [true, false] {
                let p = state
                    .propose_factor(&config(kind), 0.5, ORDER, accepted)
                    .unwrap();
                let i = state
                    .propose_factor(&config(ControllerKind::Integral), 0.5, ORDER, accepted)
                    .unwrap();
                assert_eq!(p.to_bits(), i.to_bits());
            }
        }
    }
}
