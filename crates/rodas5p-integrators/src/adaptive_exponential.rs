use rodas5p_core::{CoreError, CoreResult, WorkCounters, error_scale, wrms};
use serde::{Deserialize, Serialize};

use crate::output::OutputCollector;
use crate::{
    AdaptiveControllerState, AdaptiveStepConfig, EarlyFlowDefectTelemetry,
    EarlyFlowDefectTelemetryMode, FusedPhiKrylovConfig, ObservedIntegrationResult, OdeProblem,
    OutputSchedule, ParallelExecution, exponential::pexprb54s4_fused_step_charged,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptiveEarlyFlowDefectOutcome {
    #[default]
    LegacyUnclassified,
    Accepted,
    RejectedErrorControl,
    TrialFailureUnscorable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveEarlyFlowDefectAttempt {
    pub t: f64,
    pub step_size: f64,
    pub output_clipped: bool,
    #[serde(default)]
    pub outcome: AdaptiveEarlyFlowDefectOutcome,
    #[serde(default)]
    pub telemetry: Option<EarlyFlowDefectTelemetry>,
    #[serde(default)]
    pub time_error_norm: Option<f64>,
    #[serde(default)]
    pub phi_error_norm: Option<f64>,
    #[serde(default)]
    pub total_error_norm: Option<f64>,
    #[serde(default)]
    pub candidate_state_finite: Option<bool>,
    #[serde(default)]
    pub maximum_krylov_dimension: Option<usize>,
    #[serde(default)]
    pub phi_substeps: Option<usize>,
    #[serde(default)]
    pub trial_work: Option<WorkCounters>,
    #[serde(default)]
    pub failure: Option<String>,
}

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub early_flow_defect_attempts: Vec<AdaptiveEarlyFlowDefectAttempt>,
}

#[derive(Clone, Debug)]
pub struct AdaptiveFusedExponentialResult {
    pub observed: ObservedIntegrationResult,
    pub diagnostics: AdaptiveFusedExponentialDiagnostics,
}

#[derive(Clone, Copy, Debug)]
enum AdaptiveEarlyFlowTelemetryRequest {
    Legacy(EarlyFlowDefectTelemetryMode),
    ToleranceScaled { norm_component_count: usize },
}

impl AdaptiveEarlyFlowTelemetryRequest {
    fn enabled(self) -> bool {
        !matches!(self, Self::Legacy(EarlyFlowDefectTelemetryMode::Disabled))
    }
}

fn phi_error_proxy(reports: &[crate::FusedPhiActionReport], h: f64, scale: &[f64]) -> f64 {
    let estimate = reports
        .iter()
        .filter_map(|report| {
            report
                .error_estimate
                .is_finite()
                .then_some(report.error_estimate)
        })
        .sum::<f64>();
    // The Krylov estimates are Euclidean. Convert them to a WRMS bound with
    // ||e||_wrms <= ||e||_2 / (sqrt(n) min_i w_i), not by dividing by the
    // norm of the weight vector, which understates the error by a factor up
    // to n max w / min w (audit F-043).
    let smallest_weight = scale
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min)
        .max(f64::MIN_POSITIVE);
    let root_n = (scale.len().max(1) as f64).sqrt();
    h.abs() * estimate / (root_n * smallest_weight)
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
    integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode(
        problem,
        t_span,
        y0,
        adaptive,
        output,
        phi_config,
        execution,
        EarlyFlowDefectTelemetryMode::Disabled,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    phi_config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
    telemetry_mode: EarlyFlowDefectTelemetryMode,
) -> CoreResult<AdaptiveFusedExponentialResult> {
    integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_request(
        problem,
        t_span,
        y0,
        adaptive,
        output,
        phi_config,
        execution,
        AdaptiveEarlyFlowTelemetryRequest::Legacy(telemetry_mode),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_pexprb54s4_fused_adaptive_observed_with_tolerance_scaled_telemetry(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    phi_config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
    norm_component_count: usize,
) -> CoreResult<AdaptiveFusedExponentialResult> {
    integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_request(
        problem,
        t_span,
        y0,
        adaptive,
        output,
        phi_config,
        execution,
        AdaptiveEarlyFlowTelemetryRequest::ToleranceScaled {
            norm_component_count,
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_request(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    phi_config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
    telemetry_request: AdaptiveEarlyFlowTelemetryRequest,
) -> CoreResult<AdaptiveFusedExponentialResult> {
    adaptive.validate()?;
    if y0.len() != problem.dimension || t_span.1 < t_span.0 {
        return Err(CoreError::Dimension(
            "adaptive fused exponential integration shape/span mismatch".into(),
        ));
    }
    let (mut t, tf) = t_span;
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut collector = OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.max_step);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut diagnostics = AdaptiveFusedExponentialDiagnostics::default();

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        h = crate::output::end_step_capped(t, h.min(adaptive.max_step), tf, adaptive.max_step)?;
        // A final piece that lands on tf is taken even below min_step.
        if (h < adaptive.min_step && t + h < tf) || t + h == t {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        diagnostics.attempts += 1;
        // The trial charges a local ledger that survives a failed trial, so
        // rejected-trial work stays in the run counters (audit F-039).
        let mut trial_work = WorkCounters::default();
        let trial = match telemetry_request {
            AdaptiveEarlyFlowTelemetryRequest::Legacy(telemetry_mode) => {
                pexprb54s4_fused_step_charged(
                    problem,
                    t,
                    &y,
                    trial_h,
                    phi_config,
                    execution,
                    telemetry_mode,
                    None,
                    &mut trial_work,
                )
            }
            AdaptiveEarlyFlowTelemetryRequest::ToleranceScaled {
                norm_component_count,
            } => pexprb54s4_fused_step_charged(
                problem,
                t,
                &y,
                trial_h,
                phi_config,
                execution,
                EarlyFlowDefectTelemetryMode::ReadOnly {
                    norm_component_count,
                },
                Some((adaptive.atol, adaptive.rtol)),
                &mut trial_work,
            ),
        };
        let report = match trial {
            Ok(report) => report,
            Err(error @ (CoreError::NonFinite(_) | CoreError::LinearSolve(_))) => {
                if telemetry_request.enabled() {
                    diagnostics
                        .early_flow_defect_attempts
                        .push(AdaptiveEarlyFlowDefectAttempt {
                            t,
                            step_size: trial_h,
                            output_clipped: clipped,
                            outcome: AdaptiveEarlyFlowDefectOutcome::TrialFailureUnscorable,
                            telemetry: None,
                            time_error_norm: None,
                            phi_error_norm: None,
                            total_error_norm: None,
                            candidate_state_finite: None,
                            maximum_krylov_dimension: None,
                            phi_substeps: None,
                            // The telemetry row stays unscorable; the measured
                            // work is charged to the run counters below.
                            trial_work: None,
                            failure: Some(error.to_string()),
                        });
                }
                counters.accumulate(trial_work);
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
        let candidate_state_finite = report.y_new.iter().all(|value| value.is_finite());
        let accepted = total_error.is_finite() && total_error <= 1.0 && candidate_state_finite;
        if telemetry_request.enabled() {
            diagnostics
                .early_flow_defect_attempts
                .push(AdaptiveEarlyFlowDefectAttempt {
                    t,
                    step_size: trial_h,
                    output_clipped: clipped,
                    outcome: if accepted {
                        AdaptiveEarlyFlowDefectOutcome::Accepted
                    } else {
                        AdaptiveEarlyFlowDefectOutcome::RejectedErrorControl
                    },
                    telemetry: report.early_flow_defect.clone(),
                    time_error_norm: Some(time_error),
                    phi_error_norm: Some(phi_error),
                    total_error_norm: Some(total_error),
                    candidate_state_finite: Some(candidate_state_finite),
                    maximum_krylov_dimension: Some(max_dimension),
                    phi_substeps: Some(substeps),
                    trial_work: Some(report.work),
                    failure: None,
                });
        }
        if accepted {
            t += trial_h;
            y = report.y_new;
            collector.accept(t, &y, clipped)?;
            diagnostics.accepted_steps += 1;
            diagnostics.accepted_step_sizes.push(trial_h);
            counters.accepted_steps += 1;
            controller.record_acceptance(total_error)?;
            h = trial_h * controller.propose_factor(adaptive, total_error.max(1.0e-16), 5, true)?;
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
    let success = t >= tf;
    let (times, states, output_clipped_steps) = if success {
        collector.finish()?
    } else {
        collector.finish_partial()
    };
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
