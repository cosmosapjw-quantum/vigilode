//! Shared lowering and execution of the integrator-crate methods that the facade could not
//! reach before node V03: variable-step BDF1/2, Radau IIA, the exponential Rosenbrock method
//! PEXPRB54s4 and lagged-LU RODAS5P.
//!
//! One lowering serves all four: the facade's own `build` (the one `Rodas5p::solve` runs,
//! through `crate::solve::lowered_driver_problem`), applied to a copy of the problem whose
//! callbacks count their entries. Each driver is then called exactly as a direct caller
//! would call it: its default method configuration, an `AdaptiveStepConfig` that takes
//! `rtol`, the scalar `atol`, `h0` and `max_steps` from [`Options`] and every other value
//! from its default, and the output schedule `[t0, t1]`. The wrappers only count and check;
//! they do not change a number the driver sees.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, BdfConfig, FusedPhiKrylovConfig, ObservedIntegrationResult, OutputSchedule,
    ParallelExecution, RadauConfig, Rodas5pLaggedOptions, integrate_bdf_adaptive_observed,
    integrate_pexprb54s4_fused_adaptive_observed, integrate_radau_adaptive_observed,
    integrate_rodas5p_lagged_observed,
};
use serde::Serialize;

use super::frontend::MethodSolveError;
use crate::{
    Atol, CallbackWork, FailureReason, Jacobian, Options, Problem, TimeDerivativeFn, WorkRecord,
    fd::Guard,
};

/// The integrator-crate driver an adapter calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ObservedDriver {
    Bdf,
    Radau,
    Pexprb,
    Lagged,
}

impl ObservedDriver {
    /// Path of the driver entry point.
    pub(super) fn entry_point(self) -> &'static str {
        match self {
            Self::Bdf => "rodas5p_integrators::integrate_bdf_adaptive_observed",
            Self::Radau => "rodas5p_integrators::integrate_radau_adaptive_observed",
            Self::Pexprb => "rodas5p_integrators::integrate_pexprb54s4_fused_adaptive_observed",
            Self::Lagged => "rodas5p_integrators::integrate_rodas5p_lagged_observed",
        }
    }
}

/// Entries into user callbacks, counted at the facade boundary of one run. They are known
/// even when a driver returns an error and no work record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct MethodCallbackCounts {
    /// Right-hand-side calls.
    pub rhs: u64,
    /// Dense Jacobian calls.
    pub jacobian: u64,
    /// Jacobian-vector products.
    pub jvp: u64,
    /// Explicit time-derivative calls.
    pub time_derivative: u64,
}

#[derive(Default)]
struct BoundaryCounters {
    rhs: AtomicU64,
    jacobian: AtomicU64,
    jvp: AtomicU64,
    time_derivative: AtomicU64,
}

impl BoundaryCounters {
    fn snapshot(&self) -> MethodCallbackCounts {
        MethodCallbackCounts {
            rhs: self.rhs.load(Ordering::Relaxed),
            jacobian: self.jacobian.load(Ordering::Relaxed),
            jvp: self.jvp.load(Ordering::Relaxed),
            time_derivative: self.time_derivative.load(Ordering::Relaxed),
        }
    }
}

/// A copy of `problem` whose callbacks count their entries before calling the user's.
fn counted(problem: &Problem, counters: &Arc<BoundaryCounters>) -> Problem {
    let mut counted = problem.clone();
    counted.rhs = {
        let (user, counters) = (problem.rhs.clone(), counters.clone());
        Arc::new(move |t, y, out| {
            counters.rhs.fetch_add(1, Ordering::Relaxed);
            user(t, y, out)
        })
    };
    counted.jacobian = match &problem.jacobian {
        Jacobian::Dense(user) => {
            let (user, counters) = (user.clone(), counters.clone());
            Jacobian::Dense(Arc::new(move |t, y, jac| {
                counters.jacobian.fetch_add(1, Ordering::Relaxed);
                user(t, y, jac)
            }))
        }
        Jacobian::Jvp(user) => {
            let (user, counters) = (user.clone(), counters.clone());
            Jacobian::Jvp(Arc::new(move |t, y, v, out| {
                counters.jvp.fetch_add(1, Ordering::Relaxed);
                user(t, y, v, out)
            }))
        }
        other => other.clone(),
    };
    counted.time_derivative = problem.time_derivative.clone().map(|user| {
        let counters = counters.clone();
        let wrapped: TimeDerivativeFn = Arc::new(move |t, y, out| {
            counters.time_derivative.fetch_add(1, Ordering::Relaxed);
            user(t, y, out)
        });
        wrapped
    });
    counted
}

/// Record of one run of an integrator-crate adaptive driver through the registry.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct MethodObservedRun {
    /// Driver entry point that ran.
    pub driver: &'static str,
    /// Output times: `t0` and `t1` after a success. After a failure: the output samples the
    /// driver had reached (these drivers expose no last accepted state), which is `t0` alone
    /// for the end-point schedule.
    pub t: Vec<f64>,
    /// States at `t`.
    pub y: Vec<Vec<f64>>,
    /// Whether `t1` was reached.
    pub success: bool,
    /// The driver's message, or its error text when it returned no record.
    pub message: String,
    /// Typed reason of an unsuccessful run, when one is known: a fatal callback error, the
    /// attempt budget, or the step size below the driver minimum.
    pub failure: Option<FailureReason>,
    /// The driver's error text when it returned an error and no record.
    pub driver_error: Option<String>,
    /// Work counters: the driver's record, or (when the driver returned no record) the
    /// callback entries of `callbacks_entered` only.
    pub counters: WorkRecord,
    /// `true` when `counters` is the driver's own record.
    pub counters_from_driver: bool,
    /// Step attempts, accepted and rejected.
    pub attempts: usize,
    /// Accepted steps.
    pub accepted_steps: u64,
    /// Rejected attempts.
    pub rejected_steps: u64,
    /// Internal steps reported by the driver.
    pub internal_steps: usize,
    /// Accepted steps shortened to land on an output time.
    pub output_clipped_steps: usize,
    /// Entries into user callbacks, counted at the facade boundary.
    pub callbacks_entered: MethodCallbackCounts,
    /// Recoverable, fatal and short-circuited callback outcomes.
    pub callback_outcomes: CallbackWork,
}

/// The adaptive configuration of a direct call: tolerances, first step and attempt budget
/// from `options`, everything else default. Every check here precedes any callback.
fn adaptive_config(
    problem: &Problem,
    (t0, t1): (f64, f64),
    y0: &[f64],
    options: &Options,
) -> Result<AdaptiveStepConfig, MethodSolveError> {
    let invalid = |message: &str| Err(MethodSolveError::InvalidInput(message.into()));
    if problem.dimension == 0 || y0.len() != problem.dimension {
        return invalid("a positive problem dimension and a matching y0 are required");
    }
    if !y0.iter().all(|v| v.is_finite()) {
        return invalid("y0 must be finite");
    }
    if !(t0.is_finite() && t1.is_finite() && t1 >= t0) {
        return invalid("the span must be finite with t1 >= t0");
    }
    let Atol::Scalar(atol) = options.atol else {
        return invalid("this driver takes a scalar absolute tolerance");
    };
    let adaptive = AdaptiveStepConfig {
        atol,
        rtol: options.rtol,
        initial_step: options.h0,
        max_attempts: options.max_steps,
        ..AdaptiveStepConfig::default()
    };
    adaptive
        .validate()
        .map_err(|e| MethodSolveError::InvalidInput(e.to_string()))?;
    Ok(adaptive)
}

/// Run `driver` on the lowered problem. Input errors are returned before any callback; a
/// run that entered user code always returns a record.
pub(super) fn run_observed(
    driver: ObservedDriver,
    problem: &Problem,
    span: (f64, f64),
    y0: &[f64],
    options: &Options,
) -> Result<MethodObservedRun, MethodSolveError> {
    let adaptive = adaptive_config(problem, span, y0, options)?;
    let times = if span.1 > span.0 {
        vec![span.0, span.1]
    } else {
        vec![span.0]
    };
    let schedule =
        OutputSchedule::new(times).map_err(|e| MethodSolveError::InvalidInput(e.to_string()))?;
    let counters = Arc::new(BoundaryCounters::default());
    let guard = Arc::new(Guard::default());
    let ode = crate::solve::lowered_driver_problem(&counted(problem, &counters), &guard)?;

    let outcome: Result<(ObservedIntegrationResult, usize), CoreError> = match driver {
        ObservedDriver::Bdf => integrate_bdf_adaptive_observed(
            &ode,
            span,
            y0,
            &BdfConfig::default(),
            &adaptive,
            &schedule,
        )
        .map(|run| (run.observed, run.diagnostics.attempts)),
        ObservedDriver::Radau => integrate_radau_adaptive_observed(
            &ode,
            span,
            y0,
            &RadauConfig::default(),
            &adaptive,
            &schedule,
        )
        .map(|run| (run.observed, run.diagnostics.attempts)),
        ObservedDriver::Pexprb => integrate_pexprb54s4_fused_adaptive_observed(
            &ode,
            span,
            y0,
            &adaptive,
            &schedule,
            FusedPhiKrylovConfig::default(),
            &ParallelExecution::sequential(),
        )
        .map(|run| (run.observed, run.diagnostics.attempts)),
        ObservedDriver::Lagged => integrate_rodas5p_lagged_observed(
            &ode,
            span,
            y0,
            &adaptive,
            &schedule,
            Rodas5pLaggedOptions::default(),
        )
        .map(|run| (run.fast.observed, run.fast.attempts)),
    };

    let callbacks_entered = counters.snapshot();
    let callback_outcomes = guard.callback_work();
    let fatal = guard.fatal().map(|fatal| FailureReason::CallbackFailed {
        callback: fatal.callback,
        t: fatal.t,
        message: fatal.message,
    });
    match outcome {
        Ok((observed, attempts)) => {
            let failure = if observed.success {
                None
            } else if fatal.is_some() {
                fatal
            } else if attempts >= adaptive.max_attempts {
                Some(FailureReason::MaxStepsReached {
                    max_steps: adaptive.max_attempts,
                })
            } else {
                Some(FailureReason::StepSizeTooSmall {
                    min_step: adaptive.min_step,
                })
            };
            Ok(MethodObservedRun {
                driver: driver.entry_point(),
                t: observed.t,
                y: observed.y,
                success: observed.success,
                message: observed.message,
                failure,
                driver_error: None,
                counters: (&observed.counters).into(),
                counters_from_driver: true,
                attempts,
                accepted_steps: observed.counters.accepted_steps,
                rejected_steps: observed.counters.rejected_steps,
                internal_steps: observed.internal_steps,
                output_clipped_steps: observed.output_clipped_steps,
                callbacks_entered,
                callback_outcomes,
            })
        }
        Err(error) => {
            if callbacks_entered == MethodCallbackCounts::default() && fatal.is_none() {
                // nothing was integrated and no user code ran: an input error
                return Err(MethodSolveError::InvalidInput(error.to_string()));
            }
            // The driver returned no partial record. Keep the initial state as the accepted
            // prefix and the callback entries as the work of the failed run.
            let work = WorkCounters {
                rhs_calls: callbacks_entered.rhs,
                rhs_evaluations: callbacks_entered.rhs,
                jacobian_builds: callbacks_entered.jacobian,
                jvp_calls: callbacks_entered.jvp,
                jvp_vectors: callbacks_entered.jvp,
                ft_calls: callbacks_entered.time_derivative,
                ..WorkCounters::default()
            };
            Ok(MethodObservedRun {
                driver: driver.entry_point(),
                t: vec![span.0],
                y: vec![y0.to_vec()],
                success: false,
                message: error.to_string(),
                failure: fatal,
                driver_error: Some(error.to_string()),
                counters: (&work).into(),
                counters_from_driver: false,
                attempts: 0,
                accepted_steps: 0,
                rejected_steps: 0,
                internal_steps: 0,
                output_clipped_steps: 0,
                callbacks_entered,
                callback_outcomes,
            })
        }
    }
}
