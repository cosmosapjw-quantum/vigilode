use rodas5p_core::{CoreError, CoreResult, error_scale, wrms};
use serde::Serialize;

use crate::ObservedIntegrationResult;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerKind {
    #[default]
    Integral,
    Pi,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct AdaptiveRunDiagnostics {
    pub attempts: usize,
    pub accepted_macro_steps: usize,
    pub rejected_macro_steps: usize,
    pub accepted_step_sizes: Vec<f64>,
    pub rejected_step_sizes: Vec<f64>,
    pub error_norms: Vec<f64>,
    pub estimator_orders: Vec<usize>,
    pub estimator_ids: Vec<String>,
    pub fallback_steps: usize,
}

impl AdaptiveRunDiagnostics {
    pub(crate) fn record(
        &mut self,
        step: f64,
        error: f64,
        estimator_order: usize,
        estimator_id: &str,
        accepted: bool,
    ) {
        self.attempts += 1;
        self.error_norms.push(error);
        self.estimator_orders.push(estimator_order);
        self.estimator_ids.push(estimator_id.to_owned());
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
