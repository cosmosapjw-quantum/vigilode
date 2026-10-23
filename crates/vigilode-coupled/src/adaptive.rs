//! Opt-in adaptive trajectory using the existing raw-U coupled candidate.
//!
//! Observation times are physical landings, not dense-output interpolation.
//! Rejected attempts never replace the last accepted physical state. Operational
//! residual screening is not a global-error certificate; conditional bounds
//! retain the assumptions and limitations of `RootErrorPolicy`.
use crate::{
    ConditionalErrorEstimate, Config, Failure, FailureKind, RawStepSession, RootErrorPolicy, Work,
};
use rodas5p_core::{CoreError, CoreResult, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveOutputSession, AdaptiveStepConfig,
    ObservedIntegrationResult, OdeProblem, OutputSchedule, ParallelExecution,
    rodas_next_step_after_attempt,
};
use serde::Serialize;

pub const COUPLED_ADAPTIVE_DRIVER_ID: &str = "rodas5p-raw-u-coupled-adaptive-v1";

/// Compact receipts retain failed work without retaining eight full stage
/// vectors per attempt over a long trajectory. Receipt storage is O(max_attempts)
/// with the caller's finite `AdaptiveStepConfig::max_attempts` as its limit.
#[derive(Clone, Debug, Serialize)]
pub struct CoupledTrajectoryAttempt {
    pub t: f64,
    pub h: f64,
    pub requested_h: f64,
    pub output_clipped: bool,
    pub accepted: bool,
    pub selection: Option<&'static str>,
    pub target_id: Option<&'static str>,
    pub embedded_wrms: Option<f64>,
    pub effective_error: Option<f64>,
    pub full_residual_wrms: Option<f64>,
    pub candidate_work: Work,
    pub fallback_work: WorkCounters,
    pub fallback_callback_work: Work,
    pub fallback_validation_work: Work,
    pub fallback_attempted: bool,
    pub candidate_skipped: bool,
    pub switch_decision: Option<crate::CausalSwitchReceipt>,
    pub candidate_failure: Option<Failure>,
    pub candidate_bound: Option<ConditionalErrorEstimate>,
    pub selected_bound: Option<ConditionalErrorEstimate>,
    /// If true, the recorded failure ended this trial, but the driver can retry
    /// a smaller h. Callback failures never set this flag.
    pub retryable_failure: bool,
    pub retry_failure_kind: Option<AdaptiveFailureKind>,
    pub terminal_failure: Option<Failure>,
}

#[derive(Debug)]
pub struct CoupledIntegrationResult {
    pub driver: &'static str,
    pub root_error_policy: RootErrorPolicy,
    pub observed: ObservedIntegrationResult,
    pub attempts: Vec<CoupledTrajectoryAttempt>,
    /// Last accepted physical state, even if it was not an output request.
    pub last_t: f64,
    pub last_y: Vec<f64>,
    pub terminal_failure: Option<Failure>,
}

/// Compact common work mapping. Candidate-specific batch and counterfactual
/// savings remain in the attempt receipt. RHS/JVP counts are attempted vector
/// calls, including callbacks which returned an error.
fn common_work(work: &Work) -> WorkCounters {
    WorkCounters {
        rhs_calls: work.rhs_attempted,
        rhs_evaluations: work.rhs_attempted,
        rhs_batch_calls: work.stage_batches.saturating_add(work.validation_batches),
        jvp_calls: work.jvp_attempted,
        jvp_vectors: work.jvp_attempted,
        direct_factorizations: work.small_factorizations,
        direct_solve_calls: work.small_block_solves,
        orthogonalization_inner_products: work.orthogonalization_dots,
        nonlinear_iterations: work
            .newton_updates
            .saturating_add(work.causal_picard_updates),
        nonlinear_residual_evaluations: work.stage_batches,
        nonlinear_jacobian_evaluations: work.derivative_batches,
        ..WorkCounters::default()
    }
}

fn clock_failure(error: CoreError) -> Failure {
    Failure::new(
        FailureKind::Input,
        format!("adaptive clock/output: {error}"),
    )
}

/// Integrate autonomous identity-mass ODEs with a user JVP using the existing
/// residual-first coupled kernel and a same-target raw-U sequential fallback.
///
/// The adaptive configuration is the sole authority for `atol` and `rtol`;
/// those two fields in `coupled` are overwritten in a private clone. Numerical
/// kernel budgets are otherwise unchanged. Ordinary embedded-error rejection
/// shrinks the physical step; it does not invoke a second solver at the same h.
/// Callback/model/input errors stop immediately with partial outputs and work.
/// Policy assumptions are held fixed across the trajectory: the caller must
/// justify them for every attempted h and the relevant stage-state domains.
/// The candidate's storage budget does not bound the fallback workspace or the
/// O(max_attempts) compact receipts allocated by this driver.
/// No NCP or other external execution service is used by this API.
#[allow(clippy::too_many_arguments)]
pub fn integrate_coupled_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    coupled: &Config,
    execution: &ParallelExecution,
    linear: &LinearSolverConfig,
    policy: &RootErrorPolicy,
) -> CoreResult<CoupledIntegrationResult> {
    integrate_with_schedule(
        problem, t_span, y0, adaptive, output, coupled, execution, linear, policy, None, None,
        false,
    )
}

/// Opt-in adaptive trajectory with a causal projected corrector and the same
/// original-space raw-U fallback and error policy as `integrate_coupled_observed`.
/// Frozen JVPs remain required. For one thread, `Forward` avoids repeated batches.
#[allow(clippy::too_many_arguments)]
pub fn integrate_causal_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    coupled: &Config,
    execution: &ParallelExecution,
    linear: &LinearSolverConfig,
    policy: &RootErrorPolicy,
    schedule: crate::CausalSchedule,
) -> CoreResult<CoupledIntegrationResult> {
    integrate_with_schedule(
        problem,
        t_span,
        y0,
        adaptive,
        output,
        coupled,
        execution,
        linear,
        policy,
        Some(schedule),
        None,
        false,
    )
}

/// Use an explicitly chosen projected root when its measured work remains
/// competitive, and bypass it during bounded cooldown intervals. All routes
/// retain the existing raw-U residual, embedded and optional conditional checks.
#[allow(clippy::too_many_arguments)]
pub fn integrate_switching_causal_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    coupled: &Config,
    execution: &ParallelExecution,
    linear: &LinearSolverConfig,
    policy: &RootErrorPolicy,
    switching: &crate::CausalSwitchPolicy,
) -> CoreResult<CoupledIntegrationResult> {
    switching.validate()?;
    integrate_with_schedule(
        problem,
        t_span,
        y0,
        adaptive,
        output,
        coupled,
        execution,
        linear,
        policy,
        switching.candidate.schedule(),
        Some(switching),
        false,
    )
}

/// Always select the protected same-target raw-U solve; useful as a controlled
/// comparator without paying a deliberately unsuccessful projected trial.
#[allow(clippy::too_many_arguments)]
pub fn integrate_sequential_raw_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    coupled: &Config,
    execution: &ParallelExecution,
    linear: &LinearSolverConfig,
    policy: &RootErrorPolicy,
) -> CoreResult<CoupledIntegrationResult> {
    integrate_with_schedule(
        problem, t_span, y0, adaptive, output, coupled, execution, linear, policy, None, None, true,
    )
}

#[allow(clippy::too_many_arguments)]
fn integrate_with_schedule(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    coupled: &Config,
    execution: &ParallelExecution,
    linear: &LinearSolverConfig,
    policy: &RootErrorPolicy,
    schedule: Option<crate::CausalSchedule>,
    switching: Option<&crate::CausalSwitchPolicy>,
    protected_only: bool,
) -> CoreResult<CoupledIntegrationResult> {
    if y0.len() != problem.dimension {
        return Err(CoreError::Dimension(
            "coupled adaptive initial state dimension mismatch".into(),
        ));
    }
    let mut collector = AdaptiveOutputSession::new(adaptive, output, t_span, y0)?;
    let mut session = match schedule {
        Some(schedule) => RawStepSession::new_causal(problem, linear, schedule)?,
        None => RawStepSession::new(problem, linear)?,
    };
    let mut cfg = coupled.clone();
    cfg.atol = adaptive.atol;
    cfg.rtol = adaptive.rtol;
    let mut controller = AdaptiveControllerState::default();
    let (mut t, tf) = t_span;
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step;
    let mut attempts = Vec::new();
    let mut counters = WorkCounters::default();
    let mut terminal_failure = None;
    let mut accepted_steps = 0usize;
    let mut switch_state = switching.map(crate::switching::SwitchState::new);

    while t < tf && attempts.len() < adaptive.max_attempts {
        let trial = match collector.trial(t, h, &controller) {
            Ok(Some(trial)) => trial,
            Ok(None) => break,
            Err(error) => {
                terminal_failure = Some(clock_failure(error));
                break;
            }
        };
        let epoch = problem.model_epoch();
        let skip = protected_only || switch_state.as_ref().is_some_and(|s| s.skip());
        let mut attempt = if skip {
            session.attempt_protected(problem, t, &y, trial.h, &cfg, execution, policy)
        } else {
            session.attempt(problem, t, &y, trial.h, &cfg, execution, policy)
        };
        counters.accumulate(common_work(&attempt.candidate_attempt.work));
        counters.accumulate(common_work(&attempt.fallback_validation_work));
        let mut fallback_work = attempt.fallback_counters;
        // Physical acceptance belongs to this driver, even if a future fallback
        // implementation starts counting its own provisional endpoint verdict.
        fallback_work.accepted_steps = 0;
        fallback_work.rejected_steps = 0;
        counters.accumulate(fallback_work);
        counters.fast_attempts = counters.fast_attempts.saturating_add(1);
        if attempt.fallback_attempted {
            counters.fallback_steps = counters.fallback_steps.saturating_add(1);
        }

        let selected = attempt.selected.take();
        let mut record = CoupledTrajectoryAttempt {
            t,
            h: trial.h,
            requested_h: trial.requested_h,
            output_clipped: trial.clipped,
            accepted: false,
            selection: selected.as_ref().map(|step| step.selection),
            target_id: selected.as_ref().map(|step| step.target_id),
            embedded_wrms: selected.as_ref().map(|step| step.embedded_wrms),
            effective_error: selected.as_ref().map(|step| step.effective_error),
            full_residual_wrms: selected.as_ref().map(|step| step.full_residual_wrms),
            candidate_work: attempt.candidate_attempt.work,
            fallback_work: attempt.fallback_counters,
            fallback_callback_work: attempt.fallback_callback_work,
            fallback_validation_work: attempt.fallback_validation_work,
            fallback_attempted: attempt.fallback_attempted,
            candidate_skipped: attempt.candidate_skipped,
            switch_decision: None,
            candidate_failure: attempt.candidate_attempt.failure,
            candidate_bound: attempt.candidate_bound,
            selected_bound: selected.as_ref().and_then(|s| s.conditional_bound.clone()),
            retryable_failure: attempt.retryable_failure,
            retry_failure_kind: attempt.retry_failure_kind,
            terminal_failure: attempt.terminal_failure,
        };
        let Some(selected) = selected else {
            let failure = record.terminal_failure.clone().unwrap_or_else(|| {
                Failure::new(FailureKind::RootIteration, "both raw-U step paths declined")
            });
            record.terminal_failure = Some(failure.clone());
            counters.rejected_steps = counters.rejected_steps.saturating_add(1);
            match record.retry_failure_kind {
                Some(AdaptiveFailureKind::LinearSolve) => {
                    counters.linear_solve_failures =
                        counters.linear_solve_failures.saturating_add(1);
                }
                Some(AdaptiveFailureKind::NonFinite) => {
                    counters.nonfinite_step_failures =
                        counters.nonfinite_step_failures.saturating_add(1);
                }
                Some(AdaptiveFailureKind::NonlinearSolve) => {
                    counters.nonlinear_solve_failures =
                        counters.nonlinear_solve_failures.saturating_add(1);
                }
                Some(AdaptiveFailureKind::LocalError) => {
                    counters.local_error_failures = counters.local_error_failures.saturating_add(1);
                }
                None => {} // Callback/input failures are not numerical solve failures.
            }
            if record.retryable_failure {
                match rodas_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    trial.requested_h,
                    trial.h,
                    f64::INFINITY,
                    false,
                    trial.clipped,
                ) {
                    Ok(next) => {
                        h = next;
                        if let Some(state) = &mut switch_state {
                            record.switch_decision = Some(state.observe(&record));
                        }
                        attempts.push(record);
                        continue;
                    }
                    Err(error) => {
                        let clock = clock_failure(error);
                        record.terminal_failure = Some(clock.clone());
                        terminal_failure = Some(clock);
                    }
                }
            } else {
                terminal_failure = Some(failure);
            }
            if let Some(state) = &mut switch_state {
                record.switch_decision = Some(state.observe(&record));
            }
            attempts.push(record);
            break;
        };
        if record.terminal_failure.is_some()
            || selected.y_new.len() != y.len()
            || !selected.y_new.iter().all(|v| v.is_finite())
            || selected.effective_error.is_nan()
            || selected.effective_error < 0.0
        {
            let failure = record.terminal_failure.clone().unwrap_or_else(|| {
                Failure::new(
                    FailureKind::NonFinite,
                    "invalid selected raw-U endpoint/error",
                )
            });
            record.terminal_failure = Some(failure.clone());
            terminal_failure = Some(failure);
            counters.rejected_steps = counters.rejected_steps.saturating_add(1);
            counters.nonfinite_step_failures = counters.nonfinite_step_failures.saturating_add(1);
            if let Some(state) = &mut switch_state {
                record.switch_decision = Some(state.observe(&record));
            }
            attempts.push(record);
            break;
        }
        let accepted = selected.effective_error <= 1.0;
        let next_h = match rodas_next_step_after_attempt(
            &mut controller,
            adaptive,
            trial.requested_h,
            trial.h,
            selected.effective_error,
            accepted,
            trial.clipped,
        ) {
            Ok(next) => next,
            Err(error) => {
                let failure = clock_failure(error);
                record.terminal_failure = Some(failure.clone());
                terminal_failure = Some(failure);
                counters.rejected_steps = counters.rejected_steps.saturating_add(1);
                if let Some(state) = &mut switch_state {
                    record.switch_decision = Some(state.observe(&record));
                }
                attempts.push(record);
                break;
            }
        };
        if accepted {
            if problem.model_epoch() != epoch {
                let failure = Failure::new(
                    FailureKind::ModelChanged,
                    "model changed before adaptive endpoint commit",
                );
                record.terminal_failure = Some(failure.clone());
                terminal_failure = Some(failure);
                counters.rejected_steps = counters.rejected_steps.saturating_add(1);
                if let Some(state) = &mut switch_state {
                    record.switch_decision = Some(state.observe(&record));
                }
                attempts.push(record);
                break;
            }
            // The observer checks the new endpoint before the physical state
            // and reusable fallback history are committed.
            if let Err(error) = collector.accept(t + trial.h, &selected.y_new, trial.clipped) {
                let failure = clock_failure(error);
                record.terminal_failure = Some(failure.clone());
                terminal_failure = Some(failure);
                counters.rejected_steps = counters.rejected_steps.saturating_add(1);
                if let Some(state) = &mut switch_state {
                    record.switch_decision = Some(state.observe(&record));
                }
                attempts.push(record);
                break;
            }
            t += trial.h;
            y = selected.y_new;
            session.commit_accepted(trial.h, selected.effective_error);
            accepted_steps += 1;
            counters.accepted_steps = counters.accepted_steps.saturating_add(1);
            if !record.fallback_attempted {
                counters.fast_accepts = counters.fast_accepts.saturating_add(1);
            }
            record.accepted = true;
        } else {
            counters.rejected_steps = counters.rejected_steps.saturating_add(1);
            counters.local_error_failures = counters.local_error_failures.saturating_add(1);
        }
        if let Some(state) = &mut switch_state {
            record.switch_decision = Some(state.observe(&record));
        }
        attempts.push(record);
        h = next_h;
    }
    let success = t == tf && terminal_failure.is_none();
    let message = if success {
        "success".into()
    } else if let Some(failure) = &terminal_failure {
        failure.message.clone()
    } else {
        "maximum attempt count or minimum step reached".into()
    };
    let observed = collector.finish(success, message, counters, accepted_steps)?;
    Ok(CoupledIntegrationResult {
        driver: if switching.is_some() {
            "rodas5p-raw-u-work-switch-adaptive-v1"
        } else if protected_only {
            "rodas5p-raw-u-protected-adaptive-v1"
        } else {
            match schedule {
                Some(crate::CausalSchedule::Forward) => "rodas5p-raw-u-causal-forward-adaptive-v1",
                Some(crate::CausalSchedule::BatchedPicard) => {
                    "rodas5p-raw-u-causal-picard-adaptive-v1"
                }
                None => COUPLED_ADAPTIVE_DRIVER_ID,
            }
        },
        root_error_policy: policy.clone(),
        observed,
        attempts,
        last_t: t,
        last_y: y,
        terminal_failure,
    })
}
