//! Dormand–Prince 5(4) integration for nonstiff, identity-mass ODEs.
//!
//! Only `f(t, y)` is evaluated. Seven stages produce an order-five endpoint
//! and an order-four embedded estimate. FSAL reuses the accepted endpoint
//! derivative; an error rejection retains the derivative at the unchanged
//! initial state. The RHS must describe the same deterministic model for the
//! duration of one integration. Setting `fsal = false` requests fresh first
//! derivatives for diagnostics; it does not permit the mathematical model to
//! change during the integration.
use rodas5p_core::{CoreError, CoreResult, WorkCounters, scaled_error_component};
use serde::Serialize;

use crate::adaptive::{
    AdaptiveEstimatorMetadata, AdaptiveMethodMetadata,
    adaptive_next_step_after_attempt_prevalidated,
};
use crate::{
    AdaptiveControllerState, AdaptiveOutputSession, AdaptiveStepConfig, ObservedIntegrationResult,
    OutputSchedule,
};

/// The embedded difference is O(h^5), hence the controller exponent is 1/5.
pub const DOPRI5_ADAPTIVE_METHOD: AdaptiveMethodMetadata = AdaptiveMethodMetadata {
    method: "dopri5",
    estimator: AdaptiveEstimatorMetadata {
        name: "dopri5-embedded-4",
        order: 5,
    },
};

/// DOPRI5 stage abscissae; the last two stages are evaluated at the endpoint.
pub const DOPRI5_C: [f64; 7] = [0.0, 1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0, 1.0];
/// Strictly lower-triangular DOPRI5 tableau.
pub const DOPRI5_A: [[f64; 7]; 7] = [
    [0.0; 7],
    [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0, 0.0],
    [
        19372.0 / 6561.0,
        -25360.0 / 2187.0,
        64448.0 / 6561.0,
        -212.0 / 729.0,
        0.0,
        0.0,
        0.0,
    ],
    [
        9017.0 / 3168.0,
        -355.0 / 33.0,
        46732.0 / 5247.0,
        49.0 / 176.0,
        -5103.0 / 18656.0,
        0.0,
        0.0,
    ],
    [
        35.0 / 384.0,
        0.0,
        500.0 / 1113.0,
        125.0 / 192.0,
        -2187.0 / 6784.0,
        11.0 / 84.0,
        0.0,
    ],
];
/// Fifth-order endpoint weights, equal to the final tableau row (FSAL).
pub const DOPRI5_B: [f64; 7] = DOPRI5_A[6];
/// Fourth-order embedded weights.
pub const DOPRI5_B_EMBEDDED: [f64; 7] = [
    5179.0 / 57600.0,
    0.0,
    7571.0 / 16695.0,
    393.0 / 640.0,
    -92097.0 / 339200.0,
    187.0 / 2100.0,
    1.0 / 40.0,
];
// Evaluate the difference directly to avoid subtracting two endpoint states.
const ERROR_WEIGHTS: [f64; 7] = [
    71.0 / 57600.0,
    0.0,
    -71.0 / 16695.0,
    71.0 / 1920.0,
    -17253.0 / 339200.0,
    22.0 / 525.0,
    -1.0 / 40.0,
];

#[derive(Clone, Debug)]
pub struct Dopri5Config {
    pub adaptive: AdaptiveStepConfig,
    /// `None` uses `adaptive.atol`; `Some` must have one entry per state.
    pub atol: Option<Vec<f64>>,
    /// Reuse derivatives only within this fixed-model integration session.
    pub fsal: bool,
    /// Retry a callback-returned `CoreError::NonFinite` at smaller h. This is
    /// an explicit callback protocol, used by the public recoverable-error
    /// adapter. Other callback errors and incomplete/nonfinite `Ok` output
    /// always terminate with the last committed state.
    pub recover_nonfinite_rhs: bool,
}
impl Default for Dopri5Config {
    fn default() -> Self {
        Self {
            adaptive: AdaptiveStepConfig::default(),
            atol: None,
            fsal: true,
            recover_nonfinite_rhs: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dopri5StopReason {
    AttemptBudget,
    StepUnderflow,
    Callback,
    NonFinite,
    Output,
    Controller,
}

/// A partial result retains both sampled output and the last accepted state.
/// The last accepted state need not itself be an observation time.
#[derive(Clone, Debug)]
pub struct Dopri5IntegrationResult {
    pub observed: ObservedIntegrationResult,
    pub final_time: f64,
    pub final_state: Vec<f64>,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    /// Calls that returned fully written finite output; attempted calls are
    /// `observed.counters.rhs_calls` (including failures).
    pub rhs_completed: u64,
    pub fsal_reuses: u64,
    pub stop_reason: Option<Dopri5StopReason>,
}

/// Eight n-vectors are allocated once: seven stage derivatives and scratch.
/// `step_into` itself allocates no memory on a successful trial. The caller
/// owns endpoint and embedded-error buffers. Its public trials always compute
/// their initial derivative, so changing callbacks cannot reuse stale data.
#[derive(Debug)]
pub struct Dopri5Workspace {
    n: usize,
    stages: Vec<f64>,
    stage_state: Vec<f64>,
    first_valid: bool,
}

struct TrialFailure {
    reason: Dopri5StopReason,
    error: CoreError,
    retryable: bool,
}
impl TrialFailure {
    fn numerical(message: &str) -> Self {
        Self {
            reason: Dopri5StopReason::NonFinite,
            error: CoreError::NonFinite(message.into()),
            retryable: true,
        }
    }
}

impl Dopri5Workspace {
    pub fn new(dimension: usize) -> CoreResult<Self> {
        if dimension == 0 || dimension.checked_mul(7).is_none() {
            return Err(CoreError::Dimension(
                "DOPRI5 requires a positive representable dimension".into(),
            ));
        }
        Ok(Self {
            n: dimension,
            stages: vec![0.0; dimension * 7],
            stage_state: vec![0.0; dimension],
            first_valid: false,
        })
    }

    pub fn dimension(&self) -> usize {
        self.n
    }

    /// An uncommitted fixed step. On `Err`, both output buffers are scratch
    /// and must not be accepted. No FSAL cache crosses this public call.
    pub fn step_into<F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized>(
        &mut self,
        rhs: &F,
        t: f64,
        y: &[f64],
        h: f64,
        endpoint: &mut [f64],
        error: &mut [f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.validate_trial(t, y, h, endpoint, error)?;
        self.first_valid = false;
        let mut completed = 0;
        self.trial(
            rhs,
            t,
            y,
            h,
            endpoint,
            error,
            counters,
            &mut completed,
            false,
        )
        .map_err(|failure| failure.error)
    }

    fn validate_trial(
        &self,
        t: f64,
        y: &[f64],
        h: f64,
        endpoint: &[f64],
        error: &[f64],
    ) -> CoreResult<()> {
        if y.len() != self.n || endpoint.len() != self.n || error.len() != self.n {
            return Err(CoreError::Dimension(
                "DOPRI5 step buffers have inconsistent dimensions".into(),
            ));
        }
        if !t.is_finite() || !h.is_finite() || h <= 0.0 || !(t + h).is_finite() || t + h <= t {
            return Err(CoreError::InvalidInput(
                "DOPRI5 needs a finite forward represented interval".into(),
            ));
        }
        if y.iter().any(|x| !x.is_finite()) {
            return Err(CoreError::NonFinite(
                "DOPRI5 initial state is nonfinite".into(),
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn trial<F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized>(
        &mut self,
        rhs: &F,
        t: f64,
        y: &[f64],
        h: f64,
        endpoint: &mut [f64],
        error: &mut [f64],
        counters: &mut WorkCounters,
        completed: &mut u64,
        recover_nonfinite_rhs: bool,
    ) -> Result<(), TrialFailure> {
        if !self.first_valid {
            evaluate_rhs(
                rhs,
                t,
                y,
                &mut self.stages[..self.n],
                counters,
                completed,
                recover_nonfinite_rhs,
            )?;
            self.first_valid = true;
        }
        for stage in 1..7 {
            self.stage_state.copy_from_slice(y);
            // Unit-stride state updates permit vectorization without assuming
            // that different state components of the RHS are independent.
            for previous in 0..stage {
                let weight = h * DOPRI5_A[stage][previous];
                if weight != 0.0 {
                    let derivative = &self.stages[previous * self.n..(previous + 1) * self.n];
                    for (value, &k) in self.stage_state.iter_mut().zip(derivative) {
                        *value += weight * k;
                    }
                }
            }
            if self.stage_state.iter().any(|x| !x.is_finite()) {
                return Err(TrialFailure::numerical("DOPRI5 stage state overflow"));
            }
            let stage_time = if stage >= 5 {
                t + h
            } else {
                t + DOPRI5_C[stage] * h
            };
            evaluate_rhs(
                rhs,
                stage_time,
                &self.stage_state,
                &mut self.stages[stage * self.n..(stage + 1) * self.n],
                counters,
                completed,
                recover_nonfinite_rhs,
            )?;
        }
        // The seventh stage is evaluated at exactly the fifth-order output.
        endpoint.copy_from_slice(&self.stage_state);
        error.fill(0.0);
        for (stage, &coefficient) in ERROR_WEIGHTS.iter().enumerate() {
            let weight = h * coefficient;
            if weight != 0.0 {
                for (value, &k) in error
                    .iter_mut()
                    .zip(&self.stages[stage * self.n..(stage + 1) * self.n])
                {
                    *value += weight * k;
                }
            }
        }
        if error.iter().any(|x| !x.is_finite()) {
            return Err(TrialFailure::numerical(
                "DOPRI5 embedded difference overflow",
            ));
        }
        Ok(())
    }

    fn accept_fsal(&mut self) {
        self.stages.copy_within(6 * self.n..7 * self.n, 0);
        self.first_valid = true;
    }
}

fn evaluate_rhs<F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized>(
    rhs: &F,
    t: f64,
    y: &[f64],
    out: &mut [f64],
    counters: &mut WorkCounters,
    completed: &mut u64,
    recover_nonfinite_rhs: bool,
) -> Result<(), TrialFailure> {
    // Poison reused buffers so an incomplete callback cannot inherit values.
    out.fill(f64::NAN);
    counters.rhs_calls = counters.rhs_calls.saturating_add(1);
    counters.rhs_evaluations = counters.rhs_evaluations.saturating_add(1);
    if let Err(error) = rhs(t, y, out) {
        let retryable = recover_nonfinite_rhs && matches!(&error, CoreError::NonFinite(_));
        return Err(TrialFailure {
            reason: Dopri5StopReason::Callback,
            error,
            retryable,
        });
    }
    if out.iter().any(|x| !x.is_finite()) {
        return Err(TrialFailure {
            reason: Dopri5StopReason::NonFinite,
            error: CoreError::NonFinite(
                "DOPRI5 RHS must overwrite every component with a finite value".into(),
            ),
            retryable: false,
        });
    }
    *completed = completed.saturating_add(1);
    Ok(())
}

// Streaming scaled sum-of-squares avoids a scale vector and overflow in z*z.
fn error_norm(
    y: &[f64],
    endpoint: &[f64],
    error: &[f64],
    scalar_atol: f64,
    atol: Option<&[f64]>,
    rtol: f64,
) -> CoreResult<f64> {
    let mut maximum = 0.0_f64;
    let mut sum = 0.0_f64;
    for i in 0..y.len() {
        let absolute = atol.map_or(scalar_atol, |a| a[i]);
        let scale = absolute + rtol * y[i].abs().max(endpoint[i].abs());
        if !scale.is_finite() || scale < 0.0 {
            return Err(CoreError::InvalidInput(
                "DOPRI5 error scale overflow".into(),
            ));
        }
        let z = scaled_error_component(error[i], scale).abs();
        if !z.is_finite() {
            return Ok(f64::INFINITY);
        }
        if z > maximum {
            sum = if maximum == 0.0 {
                1.0
            } else {
                1.0 + sum * (maximum / z).powi(2)
            };
            maximum = z;
        } else if maximum > 0.0 {
            sum += (z / maximum).powi(2);
        }
    }
    Ok(maximum * (sum / y.len() as f64).sqrt())
}

/// Integrate an identity-mass ODE using only RHS callbacks. Output times are
/// actual step endpoints. Input errors return `Err` before any callback;
/// runtime failures return `Ok` with `observed.success == false` and the exact
/// last accepted state. This function never constructs a Jacobian or matrix.
pub fn integrate_dopri5_observed<F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized>(
    rhs: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
) -> CoreResult<Dopri5IntegrationResult> {
    if y0.is_empty() {
        return Err(CoreError::Dimension("DOPRI5 state is empty".into()));
    }
    if y0.iter().any(|x| !x.is_finite()) {
        return Err(CoreError::NonFinite(
            "DOPRI5 initial state is nonfinite".into(),
        ));
    }
    let mut adaptive = config.adaptive.clone();
    if let Some(atol) = config.atol.as_deref() {
        if atol.len() != y0.len() {
            return Err(CoreError::Dimension(
                "DOPRI5 atol length differs from dimension".into(),
            ));
        }
        if atol.iter().any(|a| !a.is_finite() || *a < 0.0) {
            return Err(CoreError::InvalidInput(
                "DOPRI5 atol must be finite and nonnegative".into(),
            ));
        }
        adaptive.atol = atol.iter().copied().fold(0.0_f64, f64::max);
    }
    adaptive.validate()?;
    let mut output = AdaptiveOutputSession::new(&adaptive, output, t_span, y0)?;
    let mut workspace = Dopri5Workspace::new(y0.len())?;
    let mut state = y0.to_vec();
    let mut endpoint = vec![0.0; y0.len()];
    let mut error = vec![0.0; y0.len()];
    let mut counters = WorkCounters::default();
    let mut controller = AdaptiveControllerState::default();
    let mut t = t_span.0;
    let mut proposed_h = adaptive.initial_step;
    let mut attempts = 0_usize;
    let mut accepted_steps = 0_usize;
    let mut rejected_steps = 0_usize;
    let mut rhs_completed = 0_u64;
    let mut fsal_reuses = 0_u64;
    let mut stop_reason = None;
    let mut message = String::from("DOPRI5 reached the final time");
    while t < t_span.1 {
        if attempts == adaptive.max_attempts {
            stop_reason = Some(Dopri5StopReason::AttemptBudget);
            message = "DOPRI5 attempt budget exhausted".into();
            break;
        }
        let trial = match output.trial(t, proposed_h, &controller) {
            Ok(Some(trial)) => trial,
            Ok(None) => {
                stop_reason = Some(Dopri5StopReason::StepUnderflow);
                message = "DOPRI5 cannot advance under the represented/minimum step policy".into();
                break;
            }
            Err(error) => {
                stop_reason = Some(Dopri5StopReason::StepUnderflow);
                message = error.to_string();
                break;
            }
        };
        attempts += 1;
        if !config.fsal {
            workspace.first_valid = false;
        }
        if workspace.first_valid {
            fsal_reuses = fsal_reuses.saturating_add(1);
        }
        let trial_result = workspace.trial(
            rhs,
            t,
            &state,
            trial.h,
            &mut endpoint,
            &mut error,
            &mut counters,
            &mut rhs_completed,
            config.recover_nonfinite_rhs,
        );
        let norm = match trial_result {
            Ok(()) => match error_norm(
                &state,
                &endpoint,
                &error,
                adaptive.atol,
                config.atol.as_deref(),
                adaptive.rtol,
            ) {
                Ok(norm) => norm,
                Err(error) => {
                    rejected_steps += 1;
                    counters.rejected_steps = counters.rejected_steps.saturating_add(1);
                    stop_reason = Some(Dopri5StopReason::Controller);
                    message = error.to_string();
                    break;
                }
            },
            Err(failure) => {
                rejected_steps += 1;
                counters.rejected_steps = counters.rejected_steps.saturating_add(1);
                if failure.reason == Dopri5StopReason::NonFinite || failure.retryable {
                    counters.nonfinite_step_failures =
                        counters.nonfinite_step_failures.saturating_add(1);
                }
                if !failure.retryable {
                    stop_reason = Some(failure.reason);
                    message = failure.error.to_string();
                    break;
                }
                match adaptive_next_step_after_attempt_prevalidated(
                    &mut controller,
                    &adaptive,
                    trial.requested_h,
                    trial.h,
                    f64::INFINITY,
                    DOPRI5_ADAPTIVE_METHOD.estimator.order,
                    false,
                    trial.clipped,
                ) {
                    Ok(next) => {
                        proposed_h = next;
                        continue;
                    }
                    Err(error) => {
                        stop_reason = Some(Dopri5StopReason::Controller);
                        message = error.to_string();
                        break;
                    }
                }
            }
        };
        let accepted = norm <= 1.0;
        if accepted {
            if let Err(error) = output.accept(t + trial.h, &endpoint, trial.clipped) {
                rejected_steps += 1;
                counters.rejected_steps = counters.rejected_steps.saturating_add(1);
                stop_reason = Some(Dopri5StopReason::Output);
                message = error.to_string();
                break;
            }
            t += trial.h;
            state.copy_from_slice(&endpoint);
            accepted_steps += 1;
            counters.accepted_steps = counters.accepted_steps.saturating_add(1);
            if config.fsal {
                workspace.accept_fsal();
            }
        } else {
            rejected_steps += 1;
            counters.rejected_steps = counters.rejected_steps.saturating_add(1);
            counters.local_error_failures = counters.local_error_failures.saturating_add(1);
        }
        if t == t_span.1 {
            break;
        }
        match adaptive_next_step_after_attempt_prevalidated(
            &mut controller,
            &adaptive,
            trial.requested_h,
            trial.h,
            norm,
            DOPRI5_ADAPTIVE_METHOD.estimator.order,
            accepted,
            trial.clipped,
        ) {
            Ok(next) => proposed_h = next,
            Err(error) => {
                stop_reason = Some(Dopri5StopReason::Controller);
                message = error.to_string();
                break;
            }
        }
    }
    let success = t == t_span.1 && stop_reason.is_none();
    let observed = output.finish(success, message, counters, accepted_steps)?;
    Ok(Dopri5IntegrationResult {
        observed,
        final_time: t,
        final_state: state,
        attempts,
        accepted_steps,
        rejected_steps,
        rhs_completed,
        fsal_reuses,
        stop_reason,
    })
}
