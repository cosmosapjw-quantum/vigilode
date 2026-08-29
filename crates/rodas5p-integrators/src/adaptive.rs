use rodas5p_core::{CoreError, CoreResult, error_scale, wrms};
use serde::{Deserialize, Serialize};

use crate::ObservedIntegrationResult;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerKind {
    #[default]
    Integral,
    Pi,
}

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
        }
    }
}

impl AdaptiveStepConfig {
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
        };
        config.validate()?;
        Ok(config)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct AdaptiveControllerState {
    previous_accepted_error: Option<f64>,
}

impl AdaptiveControllerState {
    pub fn previous_accepted_error(&self) -> Option<f64> {
        self.previous_accepted_error
    }

    pub fn propose_factor(
        &self,
        config: &AdaptiveStepConfig,
        error: f64,
        estimator_order: usize,
        accepted: bool,
    ) -> CoreResult<f64> {
        config.validate()?;
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

/// Update a method-bound controller after one attempted step.
///
/// A forced output landing is a scheduling artifact, not an error-estimator
/// sample: a successful clipped trial restores the precise unclipped request
/// and leaves PI history untouched. Rejections always scale the actual trial.
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
    if accepted {
        if forced_output_clipped {
            return Ok(requested_h);
        }
        controller.record_acceptance(error)?;
        return Ok(trial_h * controller.propose_factor(config, error, estimator_order, true)?);
    }
    if error.is_finite() {
        controller.record_rejection(error)?;
        Ok(trial_h
            * controller.propose_factor(config, error.max(1.0e-16), estimator_order, false)?)
    } else {
        Ok(trial_h * config.min_factor)
    }
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
    let denominator = 2.0_f64.powi(method_order as i32) - 1.0;
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
