use rodas5p_core::{CoreError, CoreResult, WorkCounters, error_scale, safe_l2, wrms};
use serde::{Deserialize, Serialize};

use crate::output::OutputCollector;
use crate::{
    AdaptiveControllerState, AdaptiveStepConfig, FusedPhiKrylovConfig, OdeProblem,
    ObservedIntegrationResult, OutputSchedule, ParallelExecution, pexprb54s4_fused_step,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveFusedExponentialDiagnostics {
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub accepted_step_sizes: Vec<f64>,
    pub rejected_step_sizes: Vec<f64>,
    pub time_error_norms: Vec<f64>,
    pub phi_error_norms: Vec<f64>,
    pub total_error_norms: Vec<f64>,
    pub maximum_krylov_dimensions: Vec<usize>,
    pub phi_substeps: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct AdaptiveFusedExponentialResult {
    pub observed: ObservedIntegrationResult,
    pub diagnostics: AdaptiveFusedExponentialDiagnostics,
}

fn phi_error_proxy(
    reports: &[crate::FusedPhiActionReport],
    h: f64,
    scale: &[f64],
) -> f64 {
    let estimate = reports
        .iter()
        .filter_map(|report| report.error_estimate.is_finite().then_some(report.error_estimate))
        .sum::<f64>();
    h.abs() * estimate / safe_l2(scale).max(f64::MIN_POSITIVE)
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_pexprb54s4_fused_adaptive_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    phi_config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
) -> CoreResult<AdaptiveFusedExponentialResult> {
    adaptive.validate()?;
    if y0.len() != problem.dimension || t_span.1 < t_span.0 {
        return Err(CoreError::Dimension(
            "adaptive fused exponential integration shape/span mismatch".into(),
        ));
    }
    let (mut t, tf) = t_span;
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(tf - t);
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut diagnostics = AdaptiveFusedExponentialDiagnostics::default();
    let tolerance = 10.0 * f64::EPSILON * tf.abs().max(1.0);

    while t < tf - tolerance && diagnostics.attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        diagnostics.attempts += 1;
        let trial = pexprb54s4_fused_step(problem, t, &y, trial_h, phi_config, execution);
        let report = match trial {
            Ok(report) => report,
            Err(CoreError::NonFinite(_) | CoreError::LinearSolve(_)) => {
                diagnostics.rejected_steps += 1;
                diagnostics.rejected_step_sizes.push(trial_h);
                diagnostics.time_error_norms.push(f64::INFINITY);
                diagnostics.phi_error_norms.push(f64::INFINITY);
                diagnostics.total_error_norms.push(f64::INFINITY);
                counters.rejected_steps += 1;
                h = trial_h * adaptive.min_factor;
                continue;
            }
            Err(error) => return Err(error),
        };
        counters.accumulate(report.work);
        let error_vector = report.error_estimate.as_ref().ok_or_else(|| {
            CoreError::InvalidInput("pexprb54s4 fused step omitted embedded error".into())
        })?;
        let scale = error_scale(&y, &report.y_new, &[adaptive.atol], adaptive.rtol)?;
        let time_error = wrms(error_vector, &scale)?;
        let phi_error = phi_error_proxy(&report.fused_phi_reports, trial_h, &scale);
        let total_error = time_error.max(phi_error);
        let max_dimension = report
            .fused_phi_reports
            .iter()
            .map(|entry| entry.maximum_krylov_dimension)
            .max()
            .unwrap_or(0);
        let substeps = report
            .fused_phi_reports
            .iter()
            .map(|entry| entry.substeps)
            .sum();
        diagnostics.time_error_norms.push(time_error);
        diagnostics.phi_error_norms.push(phi_error);
        diagnostics.total_error_norms.push(total_error);
        diagnostics.maximum_krylov_dimensions.push(max_dimension);
        diagnostics.phi_substeps.push(substeps);
        let accepted = total_error.is_finite()
            && total_error <= 1.0
            && report.y_new.iter().all(|value| value.is_finite());
        if accepted {
            t += trial_h;
            y = report.y_new;
            collector.accept(t, &y, clipped)?;
            diagnostics.accepted_steps += 1;
            diagnostics.accepted_step_sizes.push(trial_h);
            counters.accepted_steps += 1;
            controller.record_acceptance(total_error)?;
            h = trial_h
                * controller.propose_factor(adaptive, total_error.max(1.0e-16), 5, true)?;
        } else {
            diagnostics.rejected_steps += 1;
            diagnostics.rejected_step_sizes.push(trial_h);
            counters.rejected_steps += 1;
            controller.record_rejection(total_error.max(1.0e-16))?;
            h = trial_h
                * if total_error.is_finite() {
                    controller.propose_factor(adaptive, total_error.max(1.0e-16), 5, false)?
                } else {
                    adaptive.min_factor
                };
        }
    }
    let success = t >= tf - tolerance;
    let (times, states, output_clipped_steps) = collector.finish()?;
    Ok(AdaptiveFusedExponentialResult {
        observed: ObservedIntegrationResult {
            t: times,
            y: states,
            success,
            message: if success {
                "success".into()
            } else {
                "maximum attempts or minimum step reached".into()
            },
            counters,
            internal_steps: diagnostics.accepted_steps,
            output_clipped_steps,
        },
        diagnostics,
    })
}
