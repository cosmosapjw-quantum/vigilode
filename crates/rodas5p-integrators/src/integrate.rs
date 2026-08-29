use crate::output::OutputCollector;
use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveObservedIntegrationResult,
    AdaptiveRunDiagnostics, AdaptiveStepConfig, HomotopyStepConfig, KrylovState,
    ObservedIntegrationResult, OdeProblem, OutputSchedule, RODAS5P_ESTIMATOR_ORDER, SabrConfig,
    StageHistory, StepResult, TransactionalQ1Q2Config, TransactionalQ1Q2RunDiagnostics,
    homotopy_step, rodas_next_step_after_attempt, sabr_step,
    sequential_matrix_free_step_with_inner_forcing, sequential_step, transactional_q1_q2_step,
};
use rodas5p_core::{CoreError, CoreResult, LinearSolverConfig, WorkCounters};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationMethod {
    Sequential,
    Sabr,
}

#[derive(Clone, Debug)]
pub struct IntegrationResult {
    pub t: Vec<f64>,
    pub y: Vec<Vec<f64>>,
    pub success: bool,
    pub message: String,
    pub counters: WorkCounters,
    pub step_methods: Vec<String>,
    pub step_sizes: Vec<f64>,
    pub error_norms: Vec<f64>,
    pub attempts: usize,
}
fn effective(r: &StepResult) -> f64 {
    r.error_norm + r.certificate.as_ref().map_or(0.0, |c| c.fixed_point_error)
}

fn adaptive_failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        _ => None,
    }
}

fn adaptive_rejection_kind(error: f64, state: &[f64]) -> AdaptiveFailureKind {
    if error.is_finite() && state.iter().all(|value| value.is_finite()) {
        AdaptiveFailureKind::LocalError
    } else {
        AdaptiveFailureKind::NonFinite
    }
}

fn record_work_failure(counters: &mut WorkCounters, kind: AdaptiveFailureKind) {
    let target = match kind {
        AdaptiveFailureKind::LocalError => &mut counters.local_error_failures,
        AdaptiveFailureKind::LinearSolve => &mut counters.linear_solve_failures,
        AdaptiveFailureKind::NonlinearSolve => &mut counters.nonlinear_solve_failures,
        AdaptiveFailureKind::NonFinite => &mut counters.nonfinite_step_failures,
    };
    *target = target.saturating_add(1);
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_fixed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    atol: f64,
    rtol: f64,
) -> CoreResult<IntegrationResult> {
    let (t0, tf) = t_span;
    if h <= 0.0 || tf < t0 {
        return Err(CoreError::InvalidInput(
            "invalid fixed-step interval".into(),
        ));
    }
    let config = linear_config.cloned().unwrap_or_default();
    let mut counters = WorkCounters::default();
    let mut history = StageHistory::default();
    let mut recycle = KrylovState::for_method(config.method);
    let mut t = t0;
    let mut y = y0.to_vec();
    let mut times = vec![t];
    let mut states = vec![y.clone()];
    let mut methods = Vec::new();
    let mut hs = Vec::new();
    let mut errors = Vec::new();
    let sabr_cfg = sabr_config.unwrap_or_default();
    let mut attempts = 0;
    while t < tf - 10.0 * f64::EPSILON * tf.abs().max(1.0) {
        attempts += 1;
        let step = h.min(tf - t);
        let r = match method {
            IntegrationMethod::Sequential => {
                let r = sequential_step(
                    problem,
                    t,
                    &y,
                    step,
                    &config,
                    recycle.as_mut(),
                    atol,
                    rtol,
                    true,
                    &mut counters,
                )?;
                history.push(step, r.stages.clone());
                r
            }
            IntegrationMethod::Sabr => sabr_step(
                problem,
                t,
                &y,
                step,
                &sabr_cfg,
                Some(&config),
                &mut history,
                recycle.as_mut(),
                atol,
                rtol,
                true,
                &mut counters,
            )?,
        };
        t = r.t_new;
        y = r.y_new.clone();
        times.push(t);
        states.push(y.clone());
        methods.push(r.method.clone());
        hs.push(step);
        errors.push(effective(&r));
    }
    Ok(IntegrationResult {
        t: times,
        y: states,
        success: true,
        message: "success".into(),
        counters,
        step_methods: methods,
        step_sizes: hs,
        error_norms: errors,
        attempts,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_adaptive(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h0: f64,
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    atol: f64,
    rtol: f64,
    max_steps: usize,
    max_step: f64,
) -> CoreResult<IntegrationResult> {
    let adaptive = AdaptiveStepConfig::legacy_rodas(atol, rtol, h0, max_steps, max_step)?;
    let (mut t, tf) = t_span;
    if tf < t {
        return Err(CoreError::InvalidInput("invalid adaptive interval".into()));
    }
    let config = linear_config.cloned().unwrap_or_default();
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(tf - t);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut history = StageHistory::default();
    let mut recycle = KrylovState::for_method(config.method);
    let sabr_cfg = sabr_config.unwrap_or_default();
    let mut times = vec![t];
    let mut states = vec![y.clone()];
    let mut methods = Vec::new();
    let mut hs = Vec::new();
    let mut errors = Vec::new();
    let mut attempts = 0;
    while t < tf && attempts < adaptive.max_attempts {
        attempts += 1;
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            break;
        }
        let state_snapshot = recycle.clone();
        let history_snapshot = history.clone();
        let trial = match method {
            IntegrationMethod::Sequential => sequential_step(
                problem,
                t,
                &y,
                h,
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
                &y,
                h,
                &sabr_cfg,
                Some(&config),
                &mut history,
                recycle.as_mut(),
                adaptive.atol,
                adaptive.rtol,
                false,
                &mut counters,
            ),
        };
        let r = match trial {
            Ok(value) => value,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("guarded adaptive failure");
                counters.rejected_steps += 1;
                record_work_failure(&mut counters, failure);
                recycle = state_snapshot;
                history = history_snapshot;
                h *= adaptive.min_factor;
                continue;
            }
            Err(error) => return Err(error),
        };
        let error = effective(&r);
        let accepted = r.accepted && error <= 1.0 && r.y_new.iter().all(|value| value.is_finite());
        if accepted {
            t = r.t_new;
            y = r.y_new.clone();
            times.push(t);
            states.push(y.clone());
            methods.push(r.method.clone());
            hs.push(r.h);
            errors.push(error);
            if method == IntegrationMethod::Sequential {
                history.push(r.h, r.stages.clone());
            }
            controller.record_acceptance(error)?;
            h *= controller.propose_factor(&adaptive, error, RODAS5P_ESTIMATOR_ORDER, true)?;
        } else {
            record_work_failure(&mut counters, adaptive_rejection_kind(error, &r.y_new));
            recycle = state_snapshot;
            history = history_snapshot;
            h *= if error.is_finite() {
                controller.record_rejection(error)?;
                controller.propose_factor(
                    &adaptive,
                    error.max(1.0e-16),
                    RODAS5P_ESTIMATOR_ORDER,
                    false,
                )?
            } else {
                adaptive.min_factor
            };
        }
    }
    let success = t >= tf - 10.0 * f64::EPSILON * tf.abs().max(1.0);
    Ok(IntegrationResult {
        t: times,
        y: states,
        success,
        message: if success {
            "success".into()
        } else {
            "maximum step count or minimum step reached".into()
        },
        counters,
        step_methods: methods,
        step_sizes: hs,
        error_norms: errors,
        attempts,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_fixed_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    atol: f64,
    rtol: f64,
    output: &OutputSchedule,
) -> CoreResult<ObservedIntegrationResult> {
    let (t0, tf) = t_span;
    if h <= 0.0 || tf < t0 {
        return Err(CoreError::InvalidInput(
            "invalid fixed-step interval".into(),
        ));
    }
    let config = linear_config.cloned().unwrap_or_default();
    let mut counters = WorkCounters::default();
    let mut history = StageHistory::default();
    let mut recycle = KrylovState::for_method(config.method);
    let mut t = t0;
    let mut y = y0.to_vec();
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let sabr_cfg = sabr_config.unwrap_or_default();
    let mut internal_steps = 0_usize;
    while t < tf - 10.0 * f64::EPSILON * tf.abs().max(1.0) {
        let (step, clipped) = collector.limit_step(t, h, tf)?;
        let r = match method {
            IntegrationMethod::Sequential => {
                let r = sequential_step(
                    problem,
                    t,
                    &y,
                    step,
                    &config,
                    recycle.as_mut(),
                    atol,
                    rtol,
                    true,
                    &mut counters,
                )?;
                history.push(step, r.stages.clone());
                r
            }
            IntegrationMethod::Sabr => sabr_step(
                problem,
                t,
                &y,
                step,
                &sabr_cfg,
                Some(&config),
                &mut history,
                recycle.as_mut(),
                atol,
                rtol,
                true,
                &mut counters,
            )?,
        };
        t = r.t_new;
        y = r.y_new;
        collector.accept(t, &y, clipped)?;
        internal_steps += 1;
    }
    let (times, states, output_clipped_steps) = collector.finish()?;
    Ok(ObservedIntegrationResult {
        t: times,
        y: states,
        success: true,
        message: "success".into(),
        counters,
        internal_steps,
        output_clipped_steps,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_adaptive_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h0: f64,
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    atol: f64,
    rtol: f64,
    max_steps: usize,
    max_step: f64,
    output: &OutputSchedule,
) -> CoreResult<ObservedIntegrationResult> {
    let adaptive = AdaptiveStepConfig::legacy_rodas(atol, rtol, h0, max_steps, max_step)?;
    Ok(integrate_adaptive_observed_with_config(
        problem,
        t_span,
        y0,
        method,
        linear_config,
        sabr_config,
        &adaptive,
        output,
    )?
    .observed)
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_adaptive_observed_with_config(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<AdaptiveObservedIntegrationResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if tf < t {
        return Err(CoreError::InvalidInput("invalid adaptive interval".into()));
    }
    let config = linear_config.cloned().unwrap_or_default();
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(tf - t);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut history = StageHistory::default();
    let mut recycle = KrylovState::for_method(config.method);
    let sabr_cfg = sabr_config.unwrap_or_default();
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut internal_steps = 0_usize;
    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        let state_snapshot = recycle.clone();
        let history_snapshot = history.clone();
        let trial = match method {
            IntegrationMethod::Sequential => sequential_step(
                problem,
                t,
                &y,
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
                &y,
                trial_h,
                &sabr_cfg,
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
            Ok(value) => value,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("guarded adaptive failure");
                counters.rejected_steps += 1;
                record_work_failure(&mut counters, failure);
                recycle = state_snapshot;
                history = history_snapshot;
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    RODAS5P_ESTIMATOR_ORDER,
                    "rodas5p-embedded",
                    false,
                    Some(failure),
                );
                h = rodas_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    h,
                    trial_h,
                    f64::INFINITY,
                    false,
                    clipped,
                )?;
                continue;
            }
            Err(error) => return Err(error),
        };
        let error = effective(&report);
        let accepted =
            report.accepted && error <= 1.0 && report.y_new.iter().all(|value| value.is_finite());
        let failure = (!accepted).then_some(adaptive_rejection_kind(error, &report.y_new));
        diagnostics.record_with_failure(
            trial_h,
            error,
            RODAS5P_ESTIMATOR_ORDER,
            "rodas5p-embedded-plus-algebraic",
            accepted,
            failure,
        );
        if let Some(failure) = failure {
            record_work_failure(&mut counters, failure);
        }
        if accepted {
            t = report.t_new;
            y = report.y_new;
            collector.accept(t, &y, clipped)?;
            internal_steps += 1;
            if method == IntegrationMethod::Sequential {
                history.push(report.h, report.stages);
            }
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                h,
                trial_h,
                error,
                true,
                clipped,
            )?;
        } else {
            recycle = state_snapshot;
            history = history_snapshot;
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                h,
                trial_h,
                error,
                false,
                clipped,
            )?;
        }
    }
    diagnostics.fallback_steps = counters.fallback_steps as usize;
    let success = t >= tf - 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let observed = if success {
        let (times, states, output_clipped_steps) = collector.finish()?;
        ObservedIntegrationResult {
            t: times,
            y: states,
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

#[allow(clippy::too_many_arguments)]
pub fn integrate_homotopy_adaptive_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    homotopy_config: &HomotopyStepConfig,
    fallback_config: Option<&LinearSolverConfig>,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<AdaptiveObservedIntegrationResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid adaptive Homotopy integration input".into(),
        ));
    }
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(tf - t);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut recycle = fallback_config.and_then(|config| KrylovState::for_method(config.method));
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut internal_steps = 0_usize;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        let recycle_snapshot = recycle.clone();
        let trial = homotopy_step(
            problem,
            t,
            &y,
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
                let failure = adaptive_failure_kind(&error).expect("guarded adaptive failure");
                recycle = recycle_snapshot;
                counters.rejected_steps += 1;
                record_work_failure(&mut counters, failure);
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    RODAS5P_ESTIMATOR_ORDER,
                    "homotopy-step-failed",
                    false,
                    Some(failure),
                );
                h = rodas_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    h,
                    trial_h,
                    f64::INFINITY,
                    false,
                    clipped,
                )?;
                continue;
            }
            Err(error) => return Err(error),
        };
        if !report.fast_accepted {
            diagnostics.fallback_steps += 1;
        }
        let error = effective(&report.step);
        let accepted = report.step.accepted
            && error <= 1.0
            && report.step.y_new.iter().all(|value| value.is_finite());
        let failure = (!accepted).then_some(adaptive_rejection_kind(error, &report.step.y_new));
        diagnostics.record_with_failure(
            trial_h,
            error,
            RODAS5P_ESTIMATOR_ORDER,
            "homotopy-native-rodas-endpoint",
            accepted,
            failure,
        );
        if let Some(failure) = failure {
            record_work_failure(&mut counters, failure);
        }
        if accepted {
            t = report.step.t_new;
            y = report.step.y_new;
            collector.accept(t, &y, clipped)?;
            internal_steps += 1;
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                h,
                trial_h,
                error,
                true,
                clipped,
            )?;
        } else {
            recycle = recycle_snapshot;
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                h,
                trial_h,
                error,
                false,
                clipped,
            )?;
        }
    }

    let success = t >= tf - 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let observed = if success {
        let (times, states, output_clipped_steps) = collector.finish()?;
        ObservedIntegrationResult {
            t: times,
            y: states,
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

fn protected_adaptive_failure(
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
            message: format!("post-start integration error: {error}"),
            counters,
            internal_steps,
            output_clipped_steps,
        },
        diagnostics,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_sequential_matrix_free_adaptive_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<AdaptiveObservedIntegrationResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid protected matrix-free RODAS5P integration input".into(),
        ));
    }
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(tf - t);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut recycle = KrylovState::for_method(linear_config.method);
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut internal_steps = 0_usize;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            break;
        }
        let (trial_h, clipped) = match collector.limit_step(t, h, tf) {
            Ok(value) => value,
            Err(error) => {
                return Ok(protected_adaptive_failure(
                    collector,
                    counters,
                    diagnostics,
                    internal_steps,
                    &error,
                ));
            }
        };
        let recycle_snapshot = recycle.clone();
        let trial = sequential_matrix_free_step_with_inner_forcing(
            problem,
            t,
            &y,
            trial_h,
            linear_config,
            recycle.as_mut(),
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut counters,
        )
        .map(|report| report.step);
        let report = match trial {
            Ok(report) => report,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("guarded adaptive failure");
                recycle = recycle_snapshot;
                counters.rejected_steps += 1;
                record_work_failure(&mut counters, failure);
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    RODAS5P_ESTIMATOR_ORDER,
                    "protected-matrix-free-rodas5p-step-failed",
                    false,
                    Some(failure),
                );
                h = match rodas_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    h,
                    trial_h,
                    f64::INFINITY,
                    false,
                    clipped,
                ) {
                    Ok(next) => next,
                    Err(error) => {
                        return Ok(protected_adaptive_failure(
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
                return Ok(protected_adaptive_failure(
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
        let failure = (!accepted).then_some(adaptive_rejection_kind(error, &report.y_new));
        diagnostics.record_with_failure(
            trial_h,
            error,
            RODAS5P_ESTIMATOR_ORDER,
            "protected-matrix-free-rodas5p-embedded",
            accepted,
            failure,
        );
        if let Some(failure) = failure {
            record_work_failure(&mut counters, failure);
        }
        if accepted {
            t = report.t_new;
            y = report.y_new;
            internal_steps += 1;
            if let Err(error) = collector.accept(t, &y, clipped) {
                return Ok(protected_adaptive_failure(
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
                h,
                trial_h,
                error,
                true,
                clipped,
            ) {
                Ok(next) => next,
                Err(error) => {
                    return Ok(protected_adaptive_failure(
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
            h = match rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                h,
                trial_h,
                error,
                false,
                clipped,
            ) {
                Ok(next) => next,
                Err(error) => {
                    return Ok(protected_adaptive_failure(
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

    let success = t >= tf - 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let observed = if success && collector.is_complete() {
        let (times, states, output_clipped_steps) = collector
            .finish()
            .expect("complete protected output collector must finish");
        ObservedIntegrationResult {
            t: times,
            y: states,
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
        return Ok(protected_adaptive_failure(
            collector,
            counters,
            diagnostics,
            internal_steps,
            &CoreError::InvalidInput(
                "integration reached endpoint before all requested outputs were recorded".into(),
            ),
        ));
    };
    Ok(AdaptiveObservedIntegrationResult {
        observed,
        diagnostics,
    })
}

#[derive(Clone, Debug)]
pub struct TransactionalQ1Q2AdaptiveResult {
    pub observed: ObservedIntegrationResult,
    pub diagnostics: AdaptiveRunDiagnostics,
    pub transactional: TransactionalQ1Q2RunDiagnostics,
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_transactional_q1_q2_adaptive_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    step_config: &TransactionalQ1Q2Config,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<TransactionalQ1Q2AdaptiveResult> {
    adaptive.validate()?;
    step_config.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid adaptive transactional q1/q2 integration input".into(),
        ));
    }
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(tf - t);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut transactional = TransactionalQ1Q2RunDiagnostics::default();
    let mut internal_steps = 0_usize;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        let trial = transactional_q1_q2_step(
            problem,
            t,
            &y,
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
                let failure = adaptive_failure_kind(&error).expect("guarded adaptive failure");
                counters.rejected_steps += 1;
                record_work_failure(&mut counters, failure);
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    RODAS5P_ESTIMATOR_ORDER,
                    "transactional-q1-q2-step-failed",
                    false,
                    Some(failure),
                );
                h = rodas_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    h,
                    trial_h,
                    f64::INFINITY,
                    false,
                    clipped,
                )?;
                continue;
            }
            Err(error) => return Err(error),
        };
        let error = effective(&report.step);
        let accepted = report.step.accepted
            && error <= 1.0
            && report.step.y_new.iter().all(|value| value.is_finite());
        let failure = (!accepted).then_some(adaptive_rejection_kind(error, &report.step.y_new));
        diagnostics.record_with_failure(
            trial_h,
            error,
            RODAS5P_ESTIMATOR_ORDER,
            "rodas5p-embedded-plus-transactional-algebraic",
            accepted,
            failure,
        );
        if let Some(failure) = failure {
            record_work_failure(&mut counters, failure);
        }
        transactional.record(&report, accepted);
        if accepted {
            t = report.step.t_new;
            y = report.step.y_new;
            collector.accept(t, &y, clipped)?;
            internal_steps += 1;
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                h,
                trial_h,
                error,
                true,
                clipped,
            )?;
        } else {
            h = rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                h,
                trial_h,
                error,
                false,
                clipped,
            )?;
        }
    }

    diagnostics.fallback_steps = transactional.accepted_sequential_fallback_steps;
    let success = t >= tf - 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let observed = if success {
        let (times, states, output_clipped_steps) = collector.finish()?;
        ObservedIntegrationResult {
            t: times,
            y: states,
            success: true,
            message: "success".into(),
            counters,
            internal_steps,
            output_clipped_steps,
        }
    } else {
        ObservedIntegrationResult {
            t: Vec::new(),
            y: Vec::new(),
            success: false,
            message: "maximum step count or minimum step reached".into(),
            counters,
            internal_steps,
            output_clipped_steps: 0,
        }
    };
    Ok(TransactionalQ1Q2AdaptiveResult {
        observed,
        diagnostics,
        transactional,
    })
}
