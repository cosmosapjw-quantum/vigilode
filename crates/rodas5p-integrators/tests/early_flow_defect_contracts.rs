use std::sync::Arc;

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    AdaptiveEarlyFlowDefectOutcome, AdaptiveStepConfig, ControllerKind,
    EarlyFlowDefectTelemetryMode, FusedExponentialStepReport, FusedOrthogonalization,
    FusedPhiKrylovConfig, OdeProblem, OutputSchedule, ParallelExecution,
    integrate_pexprb54s4_fused_adaptive_observed,
    integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode, pexprb54s4_fused_step,
    pexprb54s4_fused_step_with_telemetry_mode, scalar_linear_problem,
};

fn square_problem() -> OdeProblem {
    OdeProblem::new(
        "square-early-defect",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = 2.0 * y[0] * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        Some(Arc::new(|t| vec![1.0 / (1.0 - t)])),
    )
    .unwrap()
}

fn zero_problem() -> OdeProblem {
    OdeProblem::new(
        "zero-early-defect",
        1,
        Arc::new(|_, _y: &[f64], out: &mut [f64]| {
            out[0] = 0.0;
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, _y: &[f64], _v: &[f64], out: &mut [f64]| {
            out[0] = 0.0;
            Ok(())
        })),
        None,
        true,
        None,
        Some(Arc::new(|_| vec![1.0])),
    )
    .unwrap()
}

fn nonautonomous_problem() -> OdeProblem {
    OdeProblem::new(
        "nonautonomous-early-defect",
        1,
        Arc::new(|t, y: &[f64], out: &mut [f64]| {
            out[0] = t * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|t, _y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = t * v[0];
            Ok(())
        })),
        Some(Arc::new(|_t, y: &[f64], out: &mut [f64]| {
            out[0] = y[0];
            Ok(())
        })),
        false,
        None,
        None,
    )
    .unwrap()
}

fn failing_after_base_rhs_problem() -> OdeProblem {
    OdeProblem::new(
        "stage-rhs-failure",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            if y[0].to_bits() == 1.0f64.to_bits() {
                out[0] = 1.0;
                Ok(())
            } else {
                Err(CoreError::LinearSolve(
                    "intentional stage RHS failure".into(),
                ))
            }
        }),
        None,
        None,
        Some(Arc::new(|_, _y: &[f64], _v: &[f64], out: &mut [f64]| {
            out[0] = 0.0;
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn phi_config() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 12,
        dimension_increment: 1,
        relative_tolerance: 1e-11,
        absolute_tolerance: 1e-14,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 8,
    }
}

fn adaptive_config() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1e-10,
        rtol: 1e-8,
        initial_step: 0.1,
        min_step: 1e-14,
        max_step: 0.25,
        max_attempts: 10_000,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 4.0,
        reject_max_factor: 0.8,
        controller: ControllerKind::Pi,
    }
}

fn assert_same_f64_bits(left: &[f64], right: &[f64]) {
    assert_eq!(left.len(), right.len());
    for (index, (a, b)) in left.iter().zip(right).enumerate() {
        assert_eq!(a.to_bits(), b.to_bits(), "float mismatch at {index}");
    }
}

fn assert_same_nested_f64_bits(left: &[Vec<f64>], right: &[Vec<f64>]) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_same_f64_bits(a, b);
    }
}

fn assert_step_numerically_identical(
    disabled: &FusedExponentialStepReport,
    read_only: &FusedExponentialStepReport,
) {
    assert_eq!(disabled.method, read_only.method);
    assert_same_f64_bits(&disabled.y_new, &read_only.y_new);
    assert_same_f64_bits(
        disabled.y_embedded.as_ref().unwrap(),
        read_only.y_embedded.as_ref().unwrap(),
    );
    assert_same_f64_bits(
        disabled.error_estimate.as_ref().unwrap(),
        read_only.error_estimate.as_ref().unwrap(),
    );
    assert_eq!(
        disabled.logical_critical_depth,
        read_only.logical_critical_depth
    );
    assert_eq!(disabled.fused_phi_reports, read_only.fused_phi_reports);
    assert_eq!(disabled.work, read_only.work);
    assert!(disabled.early_flow_defect.is_none());
}

#[test]
fn step_read_only_telemetry_is_bitwise_and_work_counter_neutral() {
    let problem = square_problem();
    let execution = ParallelExecution::sequential();
    let disabled =
        pexprb54s4_fused_step(&problem, 0.0, &[1.0], 0.05, phi_config(), &execution).unwrap();
    let read_only = pexprb54s4_fused_step_with_telemetry_mode(
        &problem,
        0.0,
        &[1.0],
        0.05,
        phi_config(),
        &execution,
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count: 1,
        },
    )
    .unwrap();
    assert_step_numerically_identical(&disabled, &read_only);
    let telemetry = read_only.early_flow_defect.unwrap();
    assert_eq!(telemetry.stage_fraction.to_bits(), 0.25f64.to_bits());
    assert_eq!(telemetry.state_dimension, 1);
    assert_eq!(telemetry.norm_component_count, 1);
    assert_eq!(telemetry.excluded_trailing_components, 0);
    assert!(telemetry.normalized_defect.is_some_and(|value| value > 0.0));
    assert_eq!(telemetry.diagnostic_work.conceptual_vector_differences, 1);
    assert_eq!(telemetry.diagnostic_work.l2_norm_evaluations, 2);
    assert_eq!(telemetry.diagnostic_work.scalar_normalizations, 1);
    assert_eq!(telemetry.diagnostic_work.added_rhs_calls, 0);
    assert_eq!(telemetry.diagnostic_work.added_jvp_calls, 0);
    assert_eq!(telemetry.diagnostic_work.added_jvp_vectors, 0);
    assert_eq!(telemetry.diagnostic_work.added_phi_actions, 0);
    assert_eq!(telemetry.diagnostic_work.added_partial_t_calls, 0);
    assert_eq!(telemetry.diagnostic_work.added_jacobian_builds, 0);
    assert_eq!(telemetry.diagnostic_work.added_newton_iterations, 0);
}

#[test]
fn telemetry_respects_linear_zero_and_time_augmented_limits() {
    let execution = ParallelExecution::sequential();
    let (linear, y0) = scalar_linear_problem(-3.0, 2.0);
    let linear_report = pexprb54s4_fused_step_with_telemetry_mode(
        &linear.jvp_only_clone().unwrap(),
        0.0,
        &y0,
        0.02,
        phi_config(),
        &execution,
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count: 1,
        },
    )
    .unwrap();
    let linear_telemetry = linear_report.early_flow_defect.unwrap();
    assert!(linear_telemetry.nonlinear_remainder_l2 <= 1e-14);
    assert!(
        linear_telemetry
            .normalized_defect
            .is_some_and(|value| value <= 1e-14)
    );

    let zero_report = pexprb54s4_fused_step_with_telemetry_mode(
        &zero_problem(),
        0.0,
        &[1.0],
        0.1,
        phi_config(),
        &execution,
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count: 1,
        },
    )
    .unwrap();
    let zero_telemetry = zero_report.early_flow_defect.unwrap();
    assert!(zero_telemetry.zero_increment);
    assert!(!zero_telemetry.degenerate_nonzero_remainder);
    assert_eq!(zero_telemetry.normalized_defect, Some(0.0));
    assert_eq!(zero_telemetry.diagnostic_work.scalar_normalizations, 0);

    let augmented = nonautonomous_problem()
        .jvp_only_clone()
        .and_then(|problem| problem.time_augmented_clone())
        .unwrap();
    let augmented_state = vec![2.0, 0.5];
    let disabled = pexprb54s4_fused_step(
        &augmented,
        0.0,
        &augmented_state,
        0.01,
        phi_config(),
        &execution,
    )
    .unwrap();
    let read_only = pexprb54s4_fused_step_with_telemetry_mode(
        &augmented,
        0.0,
        &augmented_state,
        0.01,
        phi_config(),
        &execution,
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count: 1,
        },
    )
    .unwrap();
    assert_step_numerically_identical(&disabled, &read_only);
    let telemetry = read_only.early_flow_defect.unwrap();
    assert_eq!(telemetry.state_dimension, 2);
    assert_eq!(telemetry.norm_component_count, 1);
    assert_eq!(telemetry.excluded_trailing_components, 1);
    assert!(!telemetry.native_partial_t_sampled);
    assert_eq!(read_only.work.ft_calls, 0);
}

#[test]
fn adaptive_read_only_mode_preserves_outputs_controller_and_existing_work() {
    let problem = square_problem();
    let output = OutputSchedule::new(vec![0.0, 0.125, 0.25]).unwrap();
    let execution = ParallelExecution::sequential();
    let disabled = integrate_pexprb54s4_fused_adaptive_observed(
        &problem,
        (0.0, 0.25),
        &[1.0],
        &adaptive_config(),
        &output,
        phi_config(),
        &execution,
    )
    .unwrap();
    let read_only = integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode(
        &problem,
        (0.0, 0.25),
        &[1.0],
        &adaptive_config(),
        &output,
        phi_config(),
        &execution,
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count: 1,
        },
    )
    .unwrap();

    assert_same_f64_bits(&disabled.observed.t, &read_only.observed.t);
    assert_same_nested_f64_bits(&disabled.observed.y, &read_only.observed.y);
    assert_eq!(disabled.observed.success, read_only.observed.success);
    assert_eq!(disabled.observed.message, read_only.observed.message);
    assert_eq!(disabled.observed.counters, read_only.observed.counters);
    assert_eq!(
        disabled.observed.internal_steps,
        read_only.observed.internal_steps
    );
    assert_eq!(
        disabled.observed.output_clipped_steps,
        read_only.observed.output_clipped_steps
    );
    assert_eq!(
        disabled.diagnostics.attempts,
        read_only.diagnostics.attempts
    );
    assert_eq!(
        disabled.diagnostics.accepted_steps,
        read_only.diagnostics.accepted_steps
    );
    assert_eq!(
        disabled.diagnostics.rejected_steps,
        read_only.diagnostics.rejected_steps
    );
    assert_same_f64_bits(
        &disabled.diagnostics.accepted_step_sizes,
        &read_only.diagnostics.accepted_step_sizes,
    );
    assert_same_f64_bits(
        &disabled.diagnostics.rejected_step_sizes,
        &read_only.diagnostics.rejected_step_sizes,
    );
    assert_same_f64_bits(
        &disabled.diagnostics.time_error_norms,
        &read_only.diagnostics.time_error_norms,
    );
    assert_same_f64_bits(
        &disabled.diagnostics.phi_error_norms,
        &read_only.diagnostics.phi_error_norms,
    );
    assert_same_f64_bits(
        &disabled.diagnostics.total_error_norms,
        &read_only.diagnostics.total_error_norms,
    );
    assert_eq!(
        disabled.diagnostics.maximum_krylov_dimensions,
        read_only.diagnostics.maximum_krylov_dimensions
    );
    assert_eq!(
        disabled.diagnostics.phi_substeps,
        read_only.diagnostics.phi_substeps
    );
    assert!(disabled.diagnostics.early_flow_defect_attempts.is_empty());
    assert_eq!(
        read_only.diagnostics.early_flow_defect_attempts.len(),
        read_only.diagnostics.attempts
    );
    assert!(
        read_only
            .diagnostics
            .early_flow_defect_attempts
            .iter()
            .all(|row| row.telemetry.is_some() && row.trial_work.is_some())
    );

    let mut summed_trial_work = WorkCounters::default();
    for row in &read_only.diagnostics.early_flow_defect_attempts {
        summed_trial_work.accumulate(row.trial_work.unwrap());
    }
    let mut aggregate_numerical_work = read_only.observed.counters;
    aggregate_numerical_work.accepted_steps = 0;
    aggregate_numerical_work.rejected_steps = 0;
    assert_eq!(summed_trial_work, aggregate_numerical_work);
}

#[test]
fn failed_trial_is_explicitly_unscorable_without_inferred_work_or_eta() {
    let problem = failing_after_base_rhs_problem();
    let output = OutputSchedule::new(vec![0.0, 0.1]).unwrap();
    let mut adaptive = adaptive_config();
    adaptive.initial_step = 0.1;
    adaptive.max_step = 0.1;
    adaptive.max_attempts = 1;
    let run = integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode(
        &problem,
        (0.0, 0.1),
        &[1.0],
        &adaptive,
        &output,
        phi_config(),
        &ParallelExecution::sequential(),
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count: 1,
        },
    )
    .unwrap();
    assert!(!run.observed.success);
    assert_eq!(run.diagnostics.attempts, 1);
    assert_eq!(run.diagnostics.rejected_steps, 1);
    assert_eq!(run.diagnostics.early_flow_defect_attempts.len(), 1);
    let row = &run.diagnostics.early_flow_defect_attempts[0];
    assert_eq!(
        row.outcome,
        AdaptiveEarlyFlowDefectOutcome::TrialFailureUnscorable
    );
    assert!(row.telemetry.is_none());
    assert!(row.trial_work.is_none());
    assert!(row.time_error_norm.is_none());
    assert!(
        row.failure
            .as_deref()
            .is_some_and(|message| message.contains("intentional"))
    );
}
