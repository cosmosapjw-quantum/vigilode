//! Scoped, persistent explicit integration with accepted-state checkpoints.
//!
//! Pausing preserves the native workspace, controller, FSAL derivative, output
//! schedule and cumulative attempt budget. The scoped API keeps the callback
//! and its diagnostics alive without a self-referential allocation.

use std::cell::{Cell, RefCell};

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::{
    Dopri5Session, NativeSessionAdvance, NativeSessionIdentity, NativeSessionMethod, OutputSchedule,
};
use serde::Serialize;

use crate::explicit::validate;
use crate::{
    CallbackError, Dopri5, ExplicitCallbackFailure, ExplicitCallbackWork, ExplicitErrorControl,
    ExplicitSolution, ExplicitSolveError, ExplicitStats, Options, Problem, EXPLICIT_STATS_SCHEMA,
};

/// Caller-declared versions of the bindings held fixed during a session.
///
/// Change an epoch whenever the corresponding definition changes, including
/// changes hidden behind interior mutability. A changed epoch is refused
/// before another RHS call; start a new session with the last accepted state.
/// Epochs detect declared changes, not mutations that the caller conceals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SessionEpochs {
    /// Version of the RHS or its physical parameters.
    pub model: u64,
    /// Version of the callback provider.
    pub provider: u64,
    /// Version of the state-space error norm.
    pub norm: u64,
    /// Version of the tolerance configuration.
    pub tolerance: u64,
}

impl SessionEpochs {
    fn identity(self, dimension: usize) -> NativeSessionIdentity {
        NativeSessionIdentity {
            method: NativeSessionMethod::Dopri5,
            dimension,
            model_epoch: self.model,
            provider_epoch: self.provider,
            norm_epoch: self.norm,
            tolerance_epoch: self.tolerance,
        }
    }
}

/// Outcome of one bounded call to [`ExplicitSession::advance_to`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionAdvance {
    /// The call used its attempt quota. The accepted checkpoint can resume.
    Yield,
    /// A requested observation was reached. The session can resume.
    Checkpoint,
    /// The final time was reached. No subsequent call evaluates the RHS.
    Complete,
    /// An integration failure or global attempt limit stopped the session.
    /// No subsequent call evaluates the RHS.
    Stopped,
}

/// A borrowed, allocation-free view of the last accepted physical state.
///
/// A rejected attempt never changes `t` or `y`. Work includes both accepted
/// and rejected attempts, and `proposed_h` is the controller's next proposal.
pub struct SessionCheckpoint<'a> {
    /// Last accepted time.
    pub t: f64,
    /// Last accepted state; no trial-stage state is exposed.
    pub y: &'a [f64],
    /// Proposed next step before output landing constraints.
    pub proposed_h: f64,
    /// Cumulative attempts across every call, including failures.
    pub attempts: usize,
    /// Cumulative accepted steps.
    pub accepted_steps: usize,
    /// Cumulative rejected or failed attempts.
    pub rejected_steps: usize,
    /// All work already performed by this session.
    pub counters: &'a WorkCounters,
}

/// Cumulative native and callback statistics at an accepted checkpoint.
/// Subtract successive snapshots to report one switching batch's actual work.
#[derive(Clone, Debug)]
pub(crate) struct SessionStatistics {
    pub stats: ExplicitStats,
    pub stop_reason: Option<rodas5p_integrators::Dopri5StopReason>,
    pub callback_failure: Option<ExplicitCallbackFailure>,
}

type SessionRhs<'a> = dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + 'a;

/// A persistent DOPRI5 computation borrowed inside [`Dopri5::with_session`].
///
/// The full landing schedule, dimensions, callback and tolerances are fixed
/// at construction. Dense output, events, mass matrices and affine physical
/// norms are not supported by this session API. Jacobian and explicit time
/// derivative providers are never called.
pub struct ExplicitSession<'a> {
    native: Dopri5Session<'a, SessionRhs<'a>>,
    dimension: usize,
    work: &'a Cell<ExplicitCallbackWork>,
    failure: &'a RefCell<Option<ExplicitCallbackFailure>>,
    options: Options,
    error_control: ExplicitErrorControl,
    adaptive: rodas5p_integrators::AdaptiveStepConfig,
}

impl ExplicitSession<'_> {
    /// Advance toward a registered observation with a bounded number of trials.
    ///
    /// `target` must be the current time or a time in the original schedule;
    /// requesting an unregistered intermediate time would alter the step path.
    /// A zero quota performs no integration work. `Options::max_steps` remains
    /// a global budget and is never reset by pausing. An invalid active-session
    /// target or changed epochs returns an error without advancing or invoking
    /// the RHS. After a terminal outcome, valid epochs return that outcome
    /// without inspecting the target or doing more work.
    pub fn advance_to(
        &mut self,
        target: f64,
        attempt_quota: usize,
        epochs: SessionEpochs,
    ) -> Result<SessionAdvance, ExplicitSolveError> {
        self.native
            .advance_to(target, attempt_quota, epochs.identity(self.dimension))
            .map(|outcome| match outcome {
                NativeSessionAdvance::Yield => SessionAdvance::Yield,
                NativeSessionAdvance::Checkpoint => SessionAdvance::Checkpoint,
                NativeSessionAdvance::Complete => SessionAdvance::Complete,
                NativeSessionAdvance::Stopped => SessionAdvance::Stopped,
            })
            .map_err(driver_error)
    }

    /// Inspect the accepted state and cumulative work without allocating.
    pub fn checkpoint(&self) -> SessionCheckpoint<'_> {
        let state = self.native.checkpoint();
        SessionCheckpoint {
            t: state.t,
            y: state.y,
            proposed_h: state.proposed_h,
            attempts: state.attempts,
            accepted_steps: state.accepted_steps,
            rejected_steps: state.rejected_steps,
            counters: state.counters,
        }
    }

    /// Cumulative callback diagnostics, including unsuccessful calls.
    pub fn callback_work(&self) -> ExplicitCallbackWork {
        self.work.get()
    }

    /// Clone the fatal callback detail, if present.
    pub fn callback_failure(&self) -> Option<ExplicitCallbackFailure> {
        self.failure.borrow().clone()
    }

    /// Return a cumulative, read-only telemetry snapshot without finalizing.
    pub(crate) fn statistics(&self) -> SessionStatistics {
        let checkpoint = self.native.checkpoint();
        let native = self.native.telemetry();
        SessionStatistics {
            stats: ExplicitStats {
                schema: EXPLICIT_STATS_SCHEMA,
                method: "dopri5-5(4)-fsal",
                options: self.options.clone(),
                error_control: self.error_control.clone(),
                adaptive: self.adaptive.clone(),
                observation_mode: "step-landing",
                counters: *checkpoint.counters,
                attempts: checkpoint.attempts,
                accepted_steps: checkpoint.accepted_steps,
                rejected_steps: checkpoint.rejected_steps,
                rhs_completed: native.rhs_completed,
                fsal_reuses: native.fsal_reuses,
                callback_work: self.work.get(),
                output_clipped_steps: native.output_clipped_steps,
            },
            stop_reason: native.stop_reason,
            callback_failure: self.failure.borrow().clone(),
        }
    }
}

impl Dopri5 {
    /// Run a driver with a persistent, scoped explicit session.
    ///
    /// Unlike restarting `solve_observed` for each interval, repeated
    /// `advance_to` calls preserve workspace, FSAL and controller history.
    /// Register every observation in `times` before starting. With unchanged
    /// bindings and the same schedule, pausing does not change the native step
    /// sequence. The driver may inspect a checkpoint after each attempt by
    /// using a quota of one; this is not continuous event location.
    ///
    /// Returning from `driver` finalizes the current run. The tuple retains its
    /// return value and a complete work/accepted-state record, even when the
    /// driver stops early. Such an early result has `success == false` and no
    /// failure reason if no integration failure occurred. The scoped session
    /// itself cannot escape the driver or resume after it returns.
    ///
    /// The callback must denote the same function throughout the session.
    /// Declare changed parameters using `SessionEpochs`; reconstruct a session
    /// to apply them. Unsupported inputs are rejected before the RHS or driver
    /// runs. RHS fatal errors and malformed output stop immediately and retain
    /// the last accepted state, with the same telemetry as `solve_observed`.
    pub fn with_session<R, F>(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
        epochs: SessionEpochs,
        driver: F,
    ) -> Result<(R, ExplicitSolution), ExplicitSolveError>
    where
        F: for<'session> FnOnce(&mut ExplicitSession<'session>) -> R,
    {
        let config = validate(problem, t_span, y0, opts)?;
        let schedule = OutputSchedule::new(times.to_vec()).map_err(driver_error)?;
        let work = Cell::new(ExplicitCallbackWork::default());
        let failure = RefCell::new(None);
        let rhs = |t: f64, y: &[f64], out: &mut [f64]| {
            let mut w = work.get();
            w.attempted = w.attempted.saturating_add(1);
            work.set(w);
            // The native kernel poisons reused stage buffers before invoking
            // this adapter. The finite scan supplies public failure details.
            match (problem.rhs)(t, y, out) {
                Ok(()) => {
                    if let Some(i) = out.iter().position(|v| !v.is_finite()) {
                        let message = format!("RHS output component {i} is unwritten or nonfinite");
                        let mut w = work.get();
                        w.malformed_outputs = w.malformed_outputs.saturating_add(1);
                        work.set(w);
                        *failure.borrow_mut() = Some(ExplicitCallbackFailure {
                            t,
                            message: message.clone(),
                            malformed_output: true,
                        });
                        Err(CoreError::InvalidInput(message))
                    } else {
                        let mut w = work.get();
                        w.completed = w.completed.saturating_add(1);
                        work.set(w);
                        Ok(())
                    }
                }
                Err(CallbackError::Recoverable(message)) => {
                    let mut w = work.get();
                    w.recoverable_errors = w.recoverable_errors.saturating_add(1);
                    work.set(w);
                    Err(CoreError::NonFinite(message))
                }
                Err(CallbackError::Fatal(message)) => {
                    let mut w = work.get();
                    w.fatal_errors = w.fatal_errors.saturating_add(1);
                    work.set(w);
                    *failure.borrow_mut() = Some(ExplicitCallbackFailure {
                        t,
                        message: message.clone(),
                        malformed_output: false,
                    });
                    Err(CoreError::InvalidInput(message))
                }
            }
        };
        let native: Dopri5Session<'_, SessionRhs<'_>> = Dopri5Session::new(
            &rhs as &SessionRhs<'_>,
            t_span,
            y0,
            &config,
            &schedule,
            epochs.identity(y0.len()),
        )
        .map_err(driver_error)?;
        let mut session = ExplicitSession {
            native,
            dimension: y0.len(),
            work: &work,
            failure: &failure,
            options: opts.clone(),
            error_control: ExplicitErrorControl {
                mode: "state-wrms",
                dimension: y0.len(),
                atol: config
                    .atol
                    .clone()
                    .unwrap_or_else(|| vec![config.adaptive.atol]),
                rtol: config.adaptive.rtol,
            },
            adaptive: config.adaptive.clone(),
        };
        let value = driver(&mut session);
        let mut result = session.native.finish().map_err(driver_error)?;
        if result.observed.t.last().copied() != Some(result.final_time) {
            result.observed.t.push(result.final_time);
            result.observed.y.push(result.final_state.clone());
        }
        let solution = ExplicitSolution {
            t: result.observed.t,
            y: result.observed.y,
            success: result.observed.success,
            message: result.observed.message,
            final_time: result.final_time,
            final_state: result.final_state,
            stop_reason: result.stop_reason,
            callback_failure: failure.into_inner(),
            stats: ExplicitStats {
                schema: EXPLICIT_STATS_SCHEMA,
                method: "dopri5-5(4)-fsal",
                options: opts.clone(),
                error_control: ExplicitErrorControl {
                    mode: "state-wrms",
                    dimension: y0.len(),
                    atol: config
                        .atol
                        .clone()
                        .unwrap_or_else(|| vec![config.adaptive.atol]),
                    rtol: config.adaptive.rtol,
                },
                adaptive: config.adaptive,
                observation_mode: "step-landing",
                counters: result.observed.counters,
                attempts: result.attempts,
                accepted_steps: result.accepted_steps,
                rejected_steps: result.rejected_steps,
                rhs_completed: result.rhs_completed,
                fsal_reuses: result.fsal_reuses,
                callback_work: work.get(),
                output_clipped_steps: result.observed.output_clipped_steps,
            },
        };
        Ok((value, solution))
    }
}

fn driver_error(error: CoreError) -> ExplicitSolveError {
    match error {
        CoreError::InvalidInput(_) | CoreError::Dimension(_) => {
            ExplicitSolveError::InvalidInput(error.to_string())
        }
        _ => ExplicitSolveError::Driver(error.to_string()),
    }
}
