use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveEarlyFlowDefectOutcome, AdaptiveStepConfig, ControllerKind,
    EarlyFlowDefectTelemetryMode, FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem,
    OutputSchedule, ParallelExecution, adaptive_next_step_after_attempt,
    integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode,
};
use std::sync::Arc;

fn problem() -> OdeProblem {
    OdeProblem::new(
        "square",
        1,
        Arc::new(|_, y, out| {
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y, v, out| {
            out[0] = 2.0 * y[0] * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}
fn run(
    controller: ControllerKind,
    initial_step: f64,
    times: Vec<f64>,
) -> (
    AdaptiveStepConfig,
    rodas5p_integrators::AdaptiveFusedExponentialResult,
) {
    let config = AdaptiveStepConfig {
        atol: 1e-7,
        rtol: 1e-5,
        initial_step,
        max_step: 0.25,
        max_attempts: 1000,
        controller,
        ..Default::default()
    };
    let output = OutputSchedule::new(times).unwrap();
    let run = integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode(
        &problem(),
        (0.0, 0.25),
        &[1.0],
        &config,
        &output,
        FusedPhiKrylovConfig {
            minimum_dimension: 1,
            maximum_dimension: 12,
            dimension_increment: 1,
            relative_tolerance: 1e-10,
            absolute_tolerance: 1e-13,
            orthogonalization: FusedOrthogonalization::FullMgs,
            maximum_substeps: 8,
        },
        &ParallelExecution::sequential(),
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count: 1,
        },
    )
    .unwrap();
    (config, run)
}

#[test]
fn fused_pi_uses_previous_accepted_error_and_shared_step_policy() {
    let (config, run) = run(ControllerKind::Pi, 0.01, vec![0.0, 0.25]);
    assert!(run.observed.success);
    let rows = &run.diagnostics.early_flow_defect_attempts;
    assert!(rows.len() >= 3);
    let mut control = AdaptiveControllerState::default();
    let mut request = config.initial_step;
    for pair in rows.windows(2) {
        let row = &pair[0];
        let accepted = row.outcome == AdaptiveEarlyFlowDefectOutcome::Accepted;
        let expected = adaptive_next_step_after_attempt(
            &mut control,
            &config,
            request,
            row.step_size,
            row.total_error_norm.unwrap_or(f64::INFINITY),
            5,
            accepted,
            row.output_clipped,
        )
        .unwrap();
        request = expected.min(config.max_step).min(0.25 - pair[1].t);
        let actual = pair[1].step_size;
        assert!(
            (actual - request).abs() < 1e-13 * actual.max(request).max(1e-3),
            "actual={actual:e} shared={request:e}"
        );
    }
}

#[test]
fn fused_sliver_output_landing_preserves_preclip_request() {
    let (_, run) = run(ControllerKind::Integral, 0.05, vec![0.0, 1e-6, 0.25]);
    assert!(run.observed.success);
    let rows = &run.diagnostics.early_flow_defect_attempts;
    assert!(rows[0].output_clipped);
    assert_eq!(rows[0].step_size, 1e-6);
    assert!((rows[1].step_size - 0.05).abs() < 1e-14);
}
