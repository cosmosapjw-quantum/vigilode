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

use crate::dopri5_dense::Dopri5OutputSession;

use crate::adaptive::{
    AdaptiveEstimatorMetadata, AdaptiveMethodMetadata,
    adaptive_next_step_after_attempt_prevalidated,
};
use crate::{
    AdaptiveControllerState, AdaptiveStepConfig, AffineErrorNorm, ObservedIntegrationResult,
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
    #[expect(
        clippy::too_many_arguments,
        reason = "public in-place step API uses caller-owned buffers and counters"
    )]
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
        // Multiplying h by an O(1) tableau coefficient first can round it
        // to zero on subnormal intervals, even when h*k is appreciable.
        // Keep the usual unit-stride fast path except at these tiny scales.
        let tiny_step = h < 1.0e-300;
        for stage in 1..7 {
            self.stage_state.copy_from_slice(y);
            // Unit-stride state updates permit vectorization without assuming
            // that different state components of the RHS are independent.
            for (previous, &coefficient) in DOPRI5_A[stage].iter().enumerate().take(stage) {
                if coefficient != 0.0 {
                    let derivative = &self.stages[previous * self.n..(previous + 1) * self.n];
                    if tiny_step {
                        for (value, &k) in self.stage_state.iter_mut().zip(derivative) {
                            *value += coefficient * (h * k);
                        }
                    } else {
                        let weight = h * coefficient;
                        for (value, &k) in self.stage_state.iter_mut().zip(derivative) {
                            *value += weight * k;
                        }
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
            if coefficient != 0.0 {
                let derivative = &self.stages[stage * self.n..(stage + 1) * self.n];
                if tiny_step {
                    for (value, &k) in error.iter_mut().zip(derivative) {
                        *value += coefficient * (h * k);
                    }
                } else {
                    let weight = h * coefficient;
                    for (value, &k) in error.iter_mut().zip(derivative) {
                        *value += weight * k;
                    }
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
    integrate_dopri5_impl(rhs, t_span, y0, config, output, false, None)
}

/// Integrate with a physical-space affine error norm. The norm's own absolute
/// and relative tolerances replace the reduced-state error norm; controller
/// options and all callback/error contracts remain unchanged.
pub fn integrate_dopri5_observed_with_norm<
    F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized,
>(
    rhs: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
    norm: &AffineErrorNorm,
) -> CoreResult<Dopri5IntegrationResult> {
    integrate_dopri5_impl(rhs, t_span, y0, config, output, false, Some(norm))
}

/// Sample the quartic continuous extension of accepted DOPRI5 steps.
///
/// Interior observation times do not constrain steps or add RHS calls. The
/// interpolation has a smooth-problem local interpolation error O(h^5), with no separate
/// interpolation error estimator. Endpoints are exact accepted-state copies;
/// failed trials and failed interpolants publish no observations. Events,
/// discontinuity hard stops, and backward integration are not supplied here.
pub fn integrate_dopri5_dense_observed<
    F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized,
>(
    rhs: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
) -> CoreResult<Dopri5IntegrationResult> {
    integrate_dopri5_impl(rhs, t_span, y0, config, output, true, None)
}

/// Dense sampling with a physical-space affine error norm at step acceptance.
/// The supplied norm controls the embedded endpoint difference, not a separate
/// interior interpolation defect.
pub fn integrate_dopri5_dense_observed_with_norm<
    F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized,
>(
    rhs: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
    norm: &AffineErrorNorm,
) -> CoreResult<Dopri5IntegrationResult> {
    integrate_dopri5_impl(rhs, t_span, y0, config, output, true, Some(norm))
}

fn integrate_dopri5_impl<F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized>(
    rhs: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
    dense: bool,
    physical_norm: Option<&AffineErrorNorm>,
) -> CoreResult<Dopri5IntegrationResult> {
    if let Some(norm) = physical_norm {
        norm.validate_dimension(y0.len())?;
    }
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
    let mut output = Dopri5OutputSession::new(&adaptive, output, t_span, y0, dense)?;
    let mut workspace = Dopri5Workspace::new(y0.len())?;
    let mut state = y0.to_vec();
    let mut endpoint = vec![0.0; y0.len()];
    let mut error = vec![0.0; y0.len()];
    if let Some(norm) = physical_norm {
        // Validate the lifted initial state even on a zero-length span, before
        // invoking any RHS callback. A finite reduced state can lift to Inf.
        norm.evaluate(y0, y0, &error)?;
    }
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
            Ok(()) => match physical_norm.map_or_else(
                || {
                    error_norm(
                        &state,
                        &endpoint,
                        &error,
                        adaptive.atol,
                        config.atol.as_deref(),
                        adaptive.rtol,
                    )
                },
                |norm| norm.evaluate(&state, &endpoint, &error),
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
            if let Err(error) = output.accept(
                t,
                &state,
                t + trial.h,
                &endpoint,
                &workspace.stages,
                trial.clipped,
            ) {
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

/// Immutable caller-declared binding for the native stage-1 sessions.
/// Changing a key requires constructing a new session, resetting all caches,
/// controller history and the initial step. Mutable closure captures cannot be
/// detected; the caller must change the relevant epoch when its model changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeSessionIdentity {
    pub method: NativeSessionMethod,
    pub dimension: usize,
    pub model_epoch: u64,
    pub provider_epoch: u64,
    pub norm_epoch: u64,
    pub tolerance_epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSessionMethod {
    Dopri5,
    Rodas5pFast,
}
impl NativeSessionIdentity {
    pub fn new(method: NativeSessionMethod, dimension: usize) -> Self {
        Self {
            method,
            dimension,
            model_epoch: 0,
            provider_epoch: 0,
            norm_epoch: 0,
            tolerance_epoch: 0,
        }
    }
    pub(crate) fn check(self, supplied: Self) -> CoreResult<()> {
        if self.dimension != supplied.dimension {
            return Err(CoreError::Dimension(
                "native session dimension change requires a new workspace".into(),
            ));
        }
        if self != supplied {
            return Err(CoreError::InvalidInput(
                "native session identity change requires an explicit restart".into(),
            ));
        }
        Ok(())
    }
}
/// A pause is resumable; only Complete/Stopped are sticky terminal outcomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSessionAdvance {
    Yield,
    Checkpoint,
    Complete,
    Stopped,
}
/// Borrowed accepted state and cumulative work; inspecting it allocates nothing.
pub struct NativeSessionCheckpoint<'a> {
    pub t: f64,
    pub y: &'a [f64],
    pub proposed_h: f64,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub counters: &'a WorkCounters,
}

/// Error estimate available for the most recent instrumented attempt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dopri5AttemptErrorNorm {
    Finite(f64),
    /// A nonfinite estimate or retryable failure prevented a finite estimate.
    NonFinite,
    /// A fatal failure prevented evaluation of the error norm.
    Unavailable,
}

/// Scalar snapshot of one actual trial; `accepted` means state was committed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dopri5Attempt {
    pub t_before: f64,
    pub requested_h: f64,
    pub trial_h: f64,
    pub error_norm: Dopri5AttemptErrorNorm,
    pub accepted: bool,
    pub clipped: bool,
    pub t_after: f64,
    /// Attempted RHS calls, including callbacks returning errors.
    pub rhs_delta: u64,
}

/// Cumulative native DOPRI5 statistics available without consuming the session.
/// Combine with [`NativeSessionCheckpoint`] for cumulative work and step counts.
#[derive(Clone, Copy, Debug)]
pub struct NativeSessionTelemetry {
    pub rhs_completed: u64,
    pub fsal_reuses: u64,
    pub output_clipped_steps: usize,
    pub stop_reason: Option<Dopri5StopReason>,
}

/// Persistent endpoint/landing DOPRI5 session. The full observation schedule,
/// callback, method, norm and tolerances are frozen at construction. Dense
/// observations, events and in-place provider replacement are not supported.
/// `advance_to` accepts only current time or a registered landing observation.
pub struct Dopri5Session<'a, F: ?Sized> {
    rhs: &'a F,
    identity: NativeSessionIdentity,
    config: Dopri5Config,
    adaptive: AdaptiveStepConfig,
    schedule: OutputSchedule,
    output: Dopri5OutputSession,
    workspace: Dopri5Workspace,
    state: Vec<f64>,
    endpoint: Vec<f64>,
    error: Vec<f64>,
    counters: WorkCounters,
    controller: AdaptiveControllerState,
    t: f64,
    tf: f64,
    proposed_h: f64,
    attempts: usize,
    accepted_steps: usize,
    rejected_steps: usize,
    rhs_completed: u64,
    fsal_reuses: u64,
    stop_reason: Option<Dopri5StopReason>,
    message: String,
    attempt_telemetry: bool,
    last_attempt: Option<Dopri5Attempt>,
}
impl<'a, F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized> Dopri5Session<'a, F> {
    pub fn new(
        rhs: &'a F,
        t_span: (f64, f64),
        y0: &[f64],
        config: &Dopri5Config,
        output: &OutputSchedule,
        identity: NativeSessionIdentity,
    ) -> CoreResult<Self> {
        NativeSessionIdentity {
            method: NativeSessionMethod::Dopri5,
            dimension: y0.len(),
            ..identity
        }
        .check(identity)?;
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
        let output_session = Dopri5OutputSession::new(&adaptive, output, t_span, y0, false)?;
        Ok(Self {
            rhs,
            identity,
            config: config.clone(),
            schedule: output.clone(),
            output: output_session,
            workspace: Dopri5Workspace::new(y0.len())?,
            state: y0.to_vec(),
            endpoint: vec![0.0; y0.len()],
            error: vec![0.0; y0.len()],
            counters: WorkCounters::default(),
            controller: AdaptiveControllerState::default(),
            t: t_span.0,
            tf: t_span.1,
            proposed_h: adaptive.initial_step,
            adaptive,
            attempts: 0,
            accepted_steps: 0,
            rejected_steps: 0,
            rhs_completed: 0,
            fsal_reuses: 0,
            stop_reason: None,
            attempt_telemetry: false,
            last_attempt: None,
            message: String::from("DOPRI5 reached the final time"),
        })
    }
    /// Opt in to scalar attempt telemetry. Changing this flag clears the snapshot.
    /// No additional RHS evaluations or changes to controller/work accounting occur.
    pub fn set_attempt_telemetry(&mut self, enabled: bool) {
        self.attempt_telemetry = enabled;
        self.last_attempt = None;
    }

    /// Most recent actual attempt since enabling telemetry. A quota yield or
    /// represented-step failure before a trial leaves this snapshot unchanged.
    pub fn last_attempt(&self) -> Option<Dopri5Attempt> {
        self.last_attempt
    }

    /// Read cumulative counters from the same storage used by `finish`.
    pub fn telemetry(&self) -> NativeSessionTelemetry {
        NativeSessionTelemetry {
            rhs_completed: self.rhs_completed,
            fsal_reuses: self.fsal_reuses,
            output_clipped_steps: self.output.output_clipped_steps(),
            stop_reason: self.stop_reason,
        }
    }

    pub fn checkpoint(&self) -> NativeSessionCheckpoint<'_> {
        NativeSessionCheckpoint {
            t: self.t,
            y: &self.state,
            proposed_h: self.proposed_h,
            attempts: self.attempts,
            accepted_steps: self.accepted_steps,
            rejected_steps: self.rejected_steps,
            counters: &self.counters,
        }
    }
    pub fn advance_to(
        &mut self,
        target: f64,
        attempt_quota: usize,
        identity: NativeSessionIdentity,
    ) -> CoreResult<NativeSessionAdvance> {
        self.identity.check(identity)?;
        if self.stop_reason.is_some() {
            return Ok(NativeSessionAdvance::Stopped);
        }
        if self.t == self.tf {
            return Ok(NativeSessionAdvance::Complete);
        }
        if !target.is_finite()
            || target < self.t
            || target > self.tf
            || (target != self.t
                && self
                    .schedule
                    .times()
                    .binary_search_by(|time| {
                        // Both operands are finite; partial_cmp preserves
                        // the previous +0 == -0 membership semantics.
                        time.partial_cmp(&target)
                            .expect("validated finite schedule and target")
                    })
                    .is_err())
        {
            return Err(CoreError::InvalidInput(
                "native session target must be current or a registered landing time".into(),
            ));
        }
        let call_start = self.attempts;
        while self.t < target {
            if self.attempts == self.adaptive.max_attempts {
                self.stop_reason = Some(Dopri5StopReason::AttemptBudget);
                self.message = "DOPRI5 attempt budget exhausted".into();
                break;
            }
            if self.attempts - call_start >= attempt_quota {
                return Ok(NativeSessionAdvance::Yield);
            }
            let trial = match self.output.trial(self.t, self.proposed_h, &self.controller) {
                Ok(Some(trial)) => trial,
                Ok(None) => {
                    self.stop_reason = Some(Dopri5StopReason::StepUnderflow);
                    self.message =
                        "DOPRI5 cannot advance under the represented/minimum step policy".into();
                    break;
                }
                Err(error) => {
                    self.stop_reason = Some(Dopri5StopReason::StepUnderflow);
                    self.message = error.to_string();
                    break;
                }
            };
            self.attempts += 1;
            let rhs_before = self.counters.rhs_calls;
            if self.attempt_telemetry {
                self.last_attempt = Some(Dopri5Attempt {
                    t_before: self.t,
                    requested_h: trial.requested_h,
                    trial_h: trial.h,
                    error_norm: Dopri5AttemptErrorNorm::Unavailable,
                    accepted: false,
                    clipped: trial.clipped,
                    t_after: self.t,
                    rhs_delta: 0,
                });
            }
            if !self.config.fsal {
                self.workspace.first_valid = false;
            }
            if self.workspace.first_valid {
                self.fsal_reuses = self.fsal_reuses.saturating_add(1);
            }
            let trial_result = self.workspace.trial(
                self.rhs,
                self.t,
                &self.state,
                trial.h,
                &mut self.endpoint,
                &mut self.error,
                &mut self.counters,
                &mut self.rhs_completed,
                self.config.recover_nonfinite_rhs,
            );
            if let Some(attempt) = self.last_attempt.as_mut() {
                attempt.rhs_delta = self.counters.rhs_calls.saturating_sub(rhs_before);
            }
            let norm = match trial_result {
                Ok(()) => match error_norm(
                    &self.state,
                    &self.endpoint,
                    &self.error,
                    self.adaptive.atol,
                    self.config.atol.as_deref(),
                    self.adaptive.rtol,
                ) {
                    Ok(norm) => norm,
                    Err(error) => {
                        self.rejected_steps += 1;
                        self.counters.rejected_steps =
                            self.counters.rejected_steps.saturating_add(1);
                        self.stop_reason = Some(Dopri5StopReason::Controller);
                        self.message = error.to_string();
                        break;
                    }
                },
                Err(failure) => {
                    if failure.retryable
                        && let Some(attempt) = self.last_attempt.as_mut()
                    {
                        attempt.error_norm = Dopri5AttemptErrorNorm::NonFinite;
                    }
                    self.rejected_steps += 1;
                    self.counters.rejected_steps = self.counters.rejected_steps.saturating_add(1);
                    if failure.reason == Dopri5StopReason::NonFinite || failure.retryable {
                        self.counters.nonfinite_step_failures =
                            self.counters.nonfinite_step_failures.saturating_add(1);
                    }
                    if !failure.retryable {
                        self.stop_reason = Some(failure.reason);
                        self.message = failure.error.to_string();
                        break;
                    }
                    match adaptive_next_step_after_attempt_prevalidated(
                        &mut self.controller,
                        &self.adaptive,
                        trial.requested_h,
                        trial.h,
                        f64::INFINITY,
                        DOPRI5_ADAPTIVE_METHOD.estimator.order,
                        false,
                        trial.clipped,
                    ) {
                        Ok(next) => {
                            self.proposed_h = next;
                            continue;
                        }
                        Err(error) => {
                            self.stop_reason = Some(Dopri5StopReason::Controller);
                            self.message = error.to_string();
                            break;
                        }
                    }
                }
            };
            if let Some(attempt) = self.last_attempt.as_mut() {
                attempt.error_norm = if norm.is_finite() {
                    Dopri5AttemptErrorNorm::Finite(norm)
                } else {
                    Dopri5AttemptErrorNorm::NonFinite
                };
            }
            let accepted = norm <= 1.0;
            if accepted {
                if let Err(error) = self.output.accept(
                    self.t,
                    &self.state,
                    self.t + trial.h,
                    &self.endpoint,
                    &self.workspace.stages,
                    trial.clipped,
                ) {
                    self.rejected_steps += 1;
                    self.counters.rejected_steps = self.counters.rejected_steps.saturating_add(1);
                    self.stop_reason = Some(Dopri5StopReason::Output);
                    self.message = error.to_string();
                    break;
                }
                self.t += trial.h;
                self.state.copy_from_slice(&self.endpoint);
                self.accepted_steps += 1;
                self.counters.accepted_steps = self.counters.accepted_steps.saturating_add(1);
                if self.config.fsal {
                    self.workspace.accept_fsal();
                }
            } else {
                self.rejected_steps += 1;
                self.counters.rejected_steps = self.counters.rejected_steps.saturating_add(1);
                self.counters.local_error_failures =
                    self.counters.local_error_failures.saturating_add(1);
            }
            if let Some(attempt) = self.last_attempt.as_mut() {
                attempt.accepted = accepted;
                attempt.t_after = self.t;
            }
            if self.t == self.tf {
                break;
            }
            match adaptive_next_step_after_attempt_prevalidated(
                &mut self.controller,
                &self.adaptive,
                trial.requested_h,
                trial.h,
                norm,
                DOPRI5_ADAPTIVE_METHOD.estimator.order,
                accepted,
                trial.clipped,
            ) {
                Ok(next) => self.proposed_h = next,
                Err(error) => {
                    self.stop_reason = Some(Dopri5StopReason::Controller);
                    self.message = error.to_string();
                    break;
                }
            }
        }

        if self.t == self.tf {
            Ok(NativeSessionAdvance::Complete)
        } else if self.stop_reason.is_some() {
            Ok(NativeSessionAdvance::Stopped)
        } else if self.attempts == self.adaptive.max_attempts {
            self.stop_reason = Some(Dopri5StopReason::AttemptBudget);
            self.message = "DOPRI5 attempt budget exhausted".into();
            Ok(NativeSessionAdvance::Stopped)
        } else {
            Ok(NativeSessionAdvance::Checkpoint)
        }
    }
    pub fn finish(self) -> CoreResult<Dopri5IntegrationResult> {
        let success = self.t == self.tf && self.stop_reason.is_none();
        let message = if !success && self.stop_reason.is_none() {
            "DOPRI5 paused at accepted checkpoint".into()
        } else {
            self.message
        };
        let observed = self
            .output
            .finish(success, message, self.counters, self.accepted_steps)?;
        Ok(Dopri5IntegrationResult {
            observed,
            final_time: self.t,
            final_state: self.state,
            attempts: self.attempts,
            accepted_steps: self.accepted_steps,
            rejected_steps: self.rejected_steps,
            rhs_completed: self.rhs_completed,
            fsal_reuses: self.fsal_reuses,
            stop_reason: self.stop_reason,
        })
    }
}
