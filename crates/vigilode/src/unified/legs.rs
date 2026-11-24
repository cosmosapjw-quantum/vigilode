//! Executed legs: what ran, what it cost and why it stopped, read from the run record of
//! the driver that ran.
//!
//! DOPRI5 and RODAS5P legs run through the method registry (`MethodRegistry::route` and
//! `MethodRegistry::solve`), whose adapters are the direct calls `Dopri5::solve` and
//! `Rodas5p::solve`. The switching adapter is called directly; the registry does not list
//! it.

use std::sync::{Arc, OnceLock};

use rodas5p_core::WorkCounters;
use rodas5p_integrators::{AdaptiveStepConfig, Dopri5StopReason};
use serde::Serialize;

use super::analyzer::{SelectedTier, refusal_error};
use crate::error::{CallbackError, SolveError};
use crate::explicit::{ExplicitCallbackFailure, ExplicitSolution, ExplicitSolveError};
use crate::options::Options;
use crate::problem::{Jacobian, Problem};
use crate::solution::{Callback, FailureReason, StopRecord};
use crate::switching::{
    DerivativeFallback, SwitchingConfig, SwitchingDriverStats, SwitchingError, SwitchingMethod,
    SwitchingSolution, SwitchingSolver,
};
use crate::{
    MethodChoice, MethodFailure, MethodRegistry, MethodResult, MethodSolveError, StepFailureKind,
    StopReason, WorkRecord,
};

/// A callback that ended a run: which one, the time argument of the failing call and the
/// message it returned.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CallbackStop {
    /// The callback.
    pub callback: Callback,
    /// The time argument of the failing call.
    pub t: f64,
    /// The message of the callback.
    pub message: String,
}

/// Why a unified solve stopped before the end time, as the driver that stopped reported
/// it. Nothing is translated here; [`UnifiedStop::facade_reason`] is the translation to
/// the facade's [`FailureReason`], and says where it is not exact.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "source")]
#[non_exhaustive]
pub enum UnifiedStop {
    /// `Rodas5p::solve` stopped.
    Rodas5p {
        /// The status reason of its solution.
        reason: FailureReason,
    },
    /// `Dopri5::solve` stopped.
    Dopri5 {
        /// The stop reason of the driver.
        reason: Option<Dopri5StopReason>,
        /// The callback failure, when a callback ended the run.
        callback: Option<ExplicitCallbackFailure>,
        /// The message of the driver.
        message: String,
    },
    /// `SwitchingSolver::solve` stopped.
    Switching {
        /// The message of the adapter.
        message: String,
        /// Method of the last segment of the adapter's record, the batch that stopped.
        method: Option<SwitchingMethod>,
        /// Decision text of that segment.
        decision: Option<&'static str>,
        /// The stop record of that segment's driver, when it is RODAS5P. The adapter keeps
        /// no stop reason of a DOPRI5 batch.
        driver_stop: Option<StopRecord>,
        /// The first fatal callback error, observed where the adapter calls user code.
        callback: Option<CallbackStop>,
        /// Attempts the adapter charged.
        attempts: usize,
        /// Its attempt budget.
        max_steps: usize,
        /// Whether every driver it entered returned a work record.
        accounting_complete: bool,
    },
    /// The entry point of a further leg refused its input after an earlier leg had
    /// integrated a prefix. The prefix and its work are kept.
    Refused {
        /// The entry point that refused.
        tier: SelectedTier,
        /// Its refusal.
        error: String,
        /// The stop of the leg before it.
        prior: Option<Box<UnifiedStop>>,
    },
    /// A stop that the registry reports without a typed reason.
    Other {
        /// The message.
        message: String,
    },
}

impl UnifiedStop {
    /// The facade [`FailureReason`] of this stop, for `VigilodeSolution::status`.
    ///
    /// `budget` is the attempt budget that the stop exhausted, if it is a budget stop.
    /// Exact for RODAS5P, for a callback failure, for an exhausted attempt budget, and for
    /// a switching stop whose last segment is RODAS5P. Not exact, and then
    /// `StepSizeTooSmall` with the drivers' minimum step, for: a DOPRI5 stop by its output
    /// schedule or controller; a switching stop whose last segment is DOPRI5 and that is
    /// neither a callback failure nor the attempt budget (the adapter keeps no DOPRI5 stop
    /// reason); a stop without a typed reason. [`UnifiedStop::is_exact`] tells the cases
    /// apart.
    pub fn facade_reason(&self, budget: usize) -> FailureReason {
        let min_step = AdaptiveStepConfig::default().min_step;
        let too_small = FailureReason::StepSizeTooSmall { min_step };
        let max_steps = FailureReason::MaxStepsReached { max_steps: budget };
        let failed = |stop: &CallbackStop| FailureReason::CallbackFailed {
            callback: stop.callback,
            t: stop.t,
            message: stop.message.clone(),
        };
        match self {
            Self::Rodas5p { reason } => match reason {
                FailureReason::MaxStepsReached { .. } => max_steps,
                other => other.clone(),
            },
            Self::Dopri5 {
                reason, callback, ..
            } => match (reason, callback) {
                (_, Some(failure)) => FailureReason::CallbackFailed {
                    callback: Callback::Rhs,
                    t: failure.t,
                    message: failure.message.clone(),
                },
                (Some(Dopri5StopReason::AttemptBudget), None) => max_steps,
                (Some(Dopri5StopReason::NonFinite), None) => FailureReason::NonFinite { min_step },
                _ => too_small,
            },
            Self::Switching {
                driver_stop,
                callback,
                attempts,
                max_steps: adapter_budget,
                ..
            } => {
                if let Some(stop) = callback {
                    return failed(stop);
                }
                if attempts >= adapter_budget {
                    return max_steps;
                }
                match driver_stop {
                    Some(stop) if stop.reason == StopReason::AttemptBudget => max_steps,
                    Some(stop) if stop.reason == StopReason::MinimumStep => {
                        match stop.last_failure {
                            Some(StepFailureKind::LinearSolve) => {
                                FailureReason::LinearSolveFailed { min_step }
                            }
                            Some(StepFailureKind::NonFinite) => {
                                FailureReason::NonFinite { min_step }
                            }
                            Some(StepFailureKind::NonlinearSolve) => {
                                FailureReason::NonlinearSolveFailed { min_step }
                            }
                            Some(StepFailureKind::LocalError) | None => too_small,
                        }
                    }
                    _ => too_small,
                }
            }
            Self::Refused { prior, .. } => match prior {
                Some(prior) => prior.facade_reason(budget),
                None => too_small,
            },
            Self::Other { .. } => too_small,
        }
    }

    /// Whether [`UnifiedStop::facade_reason`] states this stop exactly.
    pub fn is_exact(&self) -> bool {
        match self {
            Self::Rodas5p { .. } => true,
            Self::Dopri5 {
                reason, callback, ..
            } => {
                callback.is_some()
                    || matches!(
                        reason,
                        Some(
                            Dopri5StopReason::AttemptBudget
                                | Dopri5StopReason::NonFinite
                                | Dopri5StopReason::StepUnderflow
                        )
                    )
            }
            Self::Switching {
                driver_stop,
                callback,
                attempts,
                max_steps,
                ..
            } => {
                callback.is_some()
                    || attempts >= max_steps
                    || driver_stop.as_ref().is_some_and(|stop| {
                        matches!(
                            stop.reason,
                            StopReason::AttemptBudget | StopReason::MinimumStep
                        )
                    })
            }
            Self::Refused { prior, .. } => prior.as_ref().is_some_and(|prior| prior.is_exact()),
            Self::Other { .. } => false,
        }
    }

    /// Whether the stop is an exhausted attempt budget of the driver that stopped.
    pub fn is_attempt_budget(&self) -> bool {
        match self {
            Self::Rodas5p { reason } => matches!(reason, FailureReason::MaxStepsReached { .. }),
            Self::Dopri5 {
                reason, callback, ..
            } => callback.is_none() && *reason == Some(Dopri5StopReason::AttemptBudget),
            Self::Switching {
                callback,
                attempts,
                max_steps,
                ..
            } => callback.is_none() && attempts >= max_steps,
            Self::Refused { prior, .. } => prior
                .as_ref()
                .is_some_and(|prior| prior.is_attempt_budget()),
            Self::Other { .. } => false,
        }
    }
}

/// One executed leg of a unified solve. Every field is read from the run record of the
/// driver that ran, after it ran.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct UnifiedLeg {
    /// The entry point that was called.
    pub tier: SelectedTier,
    /// The driver identifier of the leg's run record: `Stats::route.driver_id` of
    /// `Rodas5p::solve`, `ExplicitStats::method` of `Dopri5::solve`, the schema of the
    /// switching adapter's record.
    pub driver_id: String,
    /// The route `<method id>/<variant>` as the registry reads it from the run record
    /// (`SolutionView::route_id`); for the switching adapter `switching/` and the methods
    /// of its committed segments in order of first use.
    pub route_id: String,
    /// Driver identifiers of the committed segments of the switching adapter, in order,
    /// without immediate repetitions; empty for the other entry points.
    pub segment_drivers: Vec<String>,
    /// Start time of the leg.
    pub t_start: f64,
    /// Last accepted time of the leg.
    pub t_end: f64,
    /// The end time the leg was asked to reach.
    pub target: f64,
    /// Whether it reached it.
    pub success: bool,
    /// Attempt budget the leg was given.
    pub attempt_budget: usize,
    /// Attempts it made.
    pub attempts: usize,
    /// Accepted steps.
    pub accepted_steps: usize,
    /// Rejected attempts.
    pub rejected_steps: usize,
    /// The full work of the leg.
    pub counters: WorkRecord,
    /// Why it stopped, when it did not reach its end time.
    pub stop: Option<UnifiedStop>,
}

fn stop_of(failure: MethodFailure) -> UnifiedStop {
    match failure {
        MethodFailure::Driver(reason) => UnifiedStop::Rodas5p { reason },
        MethodFailure::Explicit {
            stop,
            callback,
            message,
        } => UnifiedStop::Dopri5 {
            reason: stop,
            callback,
            message,
        },
        other => UnifiedStop::Other {
            message: other.to_string(),
        },
    }
}

fn solve_error(error: MethodSolveError) -> SolveError {
    match error {
        MethodSolveError::Implicit(error) => error,
        MethodSolveError::Explicit(ExplicitSolveError::InvalidInput(message)) => {
            SolveError::InvalidInput(message)
        }
        MethodSolveError::Explicit(ExplicitSolveError::Driver(message)) => {
            SolveError::Driver(message)
        }
        other => SolveError::Driver(other.to_string()),
    }
}

/// Run one DOPRI5 or RODAS5P leg through the registry: route `method_id` for the declared
/// facts, execute the plan, and read the leg from the run record. `Err` means that the
/// entry point refused before integrating.
pub(super) fn registry_leg(
    problem: &Problem,
    tier: SelectedTier,
    method_id: &'static str,
    span: (f64, f64),
    y: &[f64],
    options: &Options,
) -> Result<(UnifiedLeg, MethodResult), SolveError> {
    let plan = MethodRegistry::route(problem, options, &MethodChoice::Explicit(method_id))
        .map_err(refusal_error)?;
    let solution = MethodRegistry::solve(problem, span, y, options, &plan).map_err(solve_error)?;
    let view = solution.view();
    let (accepted, rejected) = view.steps();
    let attempts = match view.native() {
        MethodResult::Rodas5p(native) => native.stats.attempts,
        MethodResult::Explicit(native) => native.stats.attempts,
        _ => (accepted + rejected) as usize,
    };
    let leg = UnifiedLeg {
        tier,
        driver_id: view.executed_driver().to_owned(),
        route_id: view.route_id(),
        segment_drivers: Vec::new(),
        t_start: span.0,
        t_end: view.final_time().unwrap_or(span.0),
        target: span.1,
        success: view.is_success(),
        attempt_budget: options.max_steps,
        attempts,
        accepted_steps: accepted as usize,
        rejected_steps: rejected as usize,
        counters: view.work(),
        stop: view.failure().map(stop_of),
    };
    Ok((leg, solution.into_result()))
}

/// The leg of a DOPRI5 run that the router drove itself (the break-even route), read from
/// its record as [`registry_leg`] reads the record of `Dopri5::solve`. `route_id` is the
/// registry's route of the explicit method for the same request.
pub(super) fn explicit_leg(
    native: &ExplicitSolution,
    route_id: String,
    span: (f64, f64),
    attempt_budget: usize,
) -> UnifiedLeg {
    let stats = &native.stats;
    UnifiedLeg {
        tier: SelectedTier::Tier1Explicit,
        driver_id: stats.method.to_owned(),
        route_id,
        segment_drivers: Vec::new(),
        t_start: span.0,
        t_end: native.final_time,
        target: span.1,
        success: native.success,
        attempt_budget,
        attempts: stats.attempts,
        accepted_steps: stats.accepted_steps,
        rejected_steps: stats.rejected_steps,
        counters: (&stats.counters).into(),
        stop: (!native.success).then(|| UnifiedStop::Dopri5 {
            reason: native.stop_reason,
            callback: native.callback_failure.clone(),
            message: native.message.clone(),
        }),
    }
}

/// One RODAS5P leg from an accepted pilot and the RODAS5P run that continued from its end
/// point: the pilot is the leg's first step. Span, attempts, steps and work are those of
/// both runs; driver, route, success and stop are those of the run that continued.
pub(super) fn rodas5p_leg_after_pilot(pilot: UnifiedLeg, rest: UnifiedLeg) -> UnifiedLeg {
    let mut counters = WorkCounters::from(&pilot.counters);
    counters.accumulate(WorkCounters::from(&rest.counters));
    UnifiedLeg {
        t_start: pilot.t_start,
        attempt_budget: pilot.attempt_budget + rest.attempt_budget,
        attempts: pilot.attempts + rest.attempts,
        accepted_steps: pilot.accepted_steps + rest.accepted_steps,
        rejected_steps: pilot.rejected_steps + rest.rejected_steps,
        counters: (&counters).into(),
        ..rest
    }
}

type Callback3 = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;
type Callback4 =
    Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;

fn note(
    first: &OnceLock<CallbackStop>,
    callback: Callback,
    t: f64,
    result: &Result<(), CallbackError>,
) {
    if let Err(CallbackError::Fatal(message)) = result {
        // only the first fatal error is kept
        let _ = first.set(CallbackStop {
            callback,
            t,
            message: message.clone(),
        });
    }
}

fn watch3(inner: Callback3, callback: Callback, first: &Arc<OnceLock<CallbackStop>>) -> Callback3 {
    let first = first.clone();
    Arc::new(move |t, y, out| {
        let result = inner(t, y, out);
        note(&first, callback, t, &result);
        result
    })
}

fn watch4(inner: Callback4, callback: Callback, first: &Arc<OnceLock<CallbackStop>>) -> Callback4 {
    let first = first.clone();
    Arc::new(move |t, y, v, out| {
        let result = inner(t, y, v, out);
        note(&first, callback, t, &result);
        result
    })
}

/// `problem` with every callback forwarded unchanged; the first fatal callback error is
/// also written to `first`. The switching adapter keeps the message of a fatal callback
/// in no record, so the router reads it where the adapter calls user code.
fn observed(problem: &Problem, first: &Arc<OnceLock<CallbackStop>>) -> Problem {
    let mut observed = problem.clone();
    observed.rhs = watch3(problem.rhs.clone(), Callback::Rhs, first);
    observed.jacobian = match &problem.jacobian {
        Jacobian::Dense(f) => Jacobian::Dense(watch3(f.clone(), Callback::Jacobian, first)),
        Jacobian::Banded { lower, upper, fill } => Jacobian::Banded {
            lower: *lower,
            upper: *upper,
            fill: watch3(fill.clone(), Callback::Jacobian, first),
        },
        Jacobian::Jvp(f) => Jacobian::Jvp(watch4(f.clone(), Callback::Jvp, first)),
        Jacobian::SparseJvp { pattern, jvp } => Jacobian::SparseJvp {
            pattern: pattern.clone(),
            jvp: watch4(jvp.clone(), Callback::Jvp, first),
        },
        Jacobian::FiniteDifference => Jacobian::FiniteDifference,
    };
    observed.time_derivative = problem
        .time_derivative
        .as_ref()
        .map(|f| watch3(f.clone(), Callback::TimeDerivative, first));
    observed
}

/// The switching configuration of the router: the default policy with finite-difference
/// derivatives allowed, as `Rodas5p::solve` takes them for a problem without a Jacobian.
pub(super) fn switching_config() -> SwitchingConfig {
    SwitchingConfig {
        fallback: DerivativeFallback::AllowFiniteDifference,
        ..SwitchingConfig::default()
    }
}

/// The full work of a switching solve: the counters of every segment's own driver record
/// (discarded trials included), and the adapter's totals where they are larger (its
/// initial screen charges calls outside every segment).
fn switching_work(solution: &SwitchingSolution) -> WorkCounters {
    let mut total = WorkCounters::default();
    for segment in &solution.stats.segments {
        match &segment.driver {
            SwitchingDriverStats::Dopri5(stats) => total.accumulate(stats.counters),
            SwitchingDriverStats::Rodas5p(stats) => total.accumulate(stats.counters.into()),
        }
    }
    let stats = &solution.stats;
    total.rhs_calls = total.rhs_calls.max(stats.rhs_calls);
    total.rhs_evaluations = total.rhs_evaluations.max(stats.rhs_calls);
    total.jacobian_builds = total.jacobian_builds.max(stats.jacobian_builds);
    total.jvp_vectors = total.jvp_vectors.max(stats.jvp_vectors);
    total.ft_calls = total.ft_calls.max(stats.ft_calls);
    total.linear_solves = total.linear_solves.max(stats.linear_solves);
    total.accepted_steps = stats.accepted_steps as u64;
    total.rejected_steps = stats.rejected_steps as u64;
    total
}

/// Run the switching adapter as one leg. `Err` is its refusal, made before any callback.
pub(super) fn switching_leg(
    problem: &Problem,
    span: (f64, f64),
    y0: &[f64],
    options: &Options,
) -> Result<(UnifiedLeg, SwitchingSolution), SwitchingError> {
    let first = Arc::new(OnceLock::new());
    let solution = SwitchingSolver::solve(
        &observed(problem, &first),
        span,
        y0,
        options,
        &switching_config(),
    )?;
    let stats = &solution.stats;
    let mut methods: Vec<&'static str> = Vec::new();
    let mut segment_drivers: Vec<String> = Vec::new();
    for segment in stats.segments.iter().filter(|segment| segment.committed) {
        let (method, driver) = match &segment.driver {
            SwitchingDriverStats::Dopri5(record) => ("dopri5", record.method),
            SwitchingDriverStats::Rodas5p(record) => ("rodas5p", record.route.driver_id),
        };
        if !methods.contains(&method) {
            methods.push(method);
        }
        if segment_drivers.last().map(String::as_str) != Some(driver) {
            segment_drivers.push(driver.to_owned());
        }
    }
    let stop = (!solution.success).then(|| {
        let last = stats.segments.last();
        UnifiedStop::Switching {
            message: solution.message.clone(),
            method: last.map(|segment| segment.method),
            decision: last.map(|segment| segment.decision),
            driver_stop: last.and_then(|segment| match &segment.driver {
                SwitchingDriverStats::Rodas5p(record) => record.stop.clone(),
                SwitchingDriverStats::Dopri5(_) => None,
            }),
            callback: first.get().cloned(),
            attempts: stats.attempts,
            max_steps: options.max_steps,
            accounting_complete: stats.accounting_complete,
        }
    });
    let leg = UnifiedLeg {
        tier: SelectedTier::Tier5AdaptiveSwitching,
        driver_id: solution.schema.to_owned(),
        route_id: format!("switching/{}", methods.join("+")),
        segment_drivers,
        t_start: span.0,
        t_end: solution.final_time,
        target: span.1,
        success: solution.success,
        attempt_budget: options.max_steps,
        attempts: stats.attempts,
        accepted_steps: stats.accepted_steps,
        rejected_steps: stats.rejected_steps,
        counters: (&switching_work(&solution)).into(),
        stop,
    };
    Ok((leg, solution))
}
