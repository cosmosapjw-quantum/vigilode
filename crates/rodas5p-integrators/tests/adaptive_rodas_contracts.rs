use rodas5p_core::LinearSolverConfig;
use rodas5p_integrators::{
    AdaptiveStepConfig, IntegrationMethod, OutputSchedule, SabrConfig, integrate_adaptive_observed,
    integrate_adaptive_observed_with_config, prothero_robinson_problem, scalar_linear_problem,
};

fn assert_observed_equal(
    left: &rodas5p_integrators::ObservedIntegrationResult,
    right: &rodas5p_integrators::ObservedIntegrationResult,
) {
    assert_eq!(left.t, right.t);
    assert_eq!(left.y, right.y);
    assert_eq!(left.success, right.success);
    assert_eq!(left.message, right.message);
    assert_eq!(left.counters, right.counters);
    assert_eq!(left.internal_steps, right.internal_steps);
    assert_eq!(left.output_clipped_steps, right.output_clipped_steps);
}

#[test]
fn sequential_legacy_wrapper_matches_common_controller_report() {
    let (problem, y0) = scalar_linear_problem(-100.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.2, 0.04).unwrap();
    let linear = LinearSolverConfig::default();
    let legacy = integrate_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.15,
        IntegrationMethod::Sequential,
        Some(&linear),
        None,
        1.0e-12,
        1.0e-8,
        10_000,
        0.15,
        &schedule,
    )
    .unwrap();
    let config = AdaptiveStepConfig::legacy_rodas(1.0e-12, 1.0e-8, 0.15, 10_000, 0.15).unwrap();
    let report = integrate_adaptive_observed_with_config(
        &problem,
        (0.0, 0.2),
        &y0,
        IntegrationMethod::Sequential,
        Some(&linear),
        None,
        &config,
        &schedule,
    )
    .unwrap();
    assert_observed_equal(&legacy, &report.observed);
    assert_eq!(
        report.diagnostics.attempts,
        legacy.counters.accepted_steps as usize + legacy.counters.rejected_steps as usize
    );
    assert_eq!(
        report.diagnostics.accepted_macro_steps,
        legacy.internal_steps
    );
    assert_eq!(
        report.diagnostics.rejected_macro_steps,
        legacy.counters.rejected_steps as usize
    );
    assert!(
        report
            .diagnostics
            .estimator_orders
            .iter()
            .all(|order| *order == 5)
    );
}

#[test]
fn sabr_legacy_wrapper_matches_common_controller_report_after_rejections() {
    let (problem, y0) = prothero_robinson_problem(-1.0e4, 1.0e10, 1.0);
    let schedule = OutputSchedule::uniform(1.0, 1.05, 0.01).unwrap();
    let sabr = SabrConfig {
        max_iterations: 2,
        ..Default::default()
    };
    let legacy = integrate_adaptive_observed(
        &problem,
        (1.0, 1.05),
        &y0,
        0.05,
        IntegrationMethod::Sabr,
        None,
        Some(sabr.clone()),
        1.0e-9,
        1.0e-6,
        10_000,
        0.05,
        &schedule,
    )
    .unwrap();
    let config = AdaptiveStepConfig::legacy_rodas(1.0e-9, 1.0e-6, 0.05, 10_000, 0.05).unwrap();
    let report = integrate_adaptive_observed_with_config(
        &problem,
        (1.0, 1.05),
        &y0,
        IntegrationMethod::Sabr,
        None,
        Some(sabr),
        &config,
        &schedule,
    )
    .unwrap();
    assert_observed_equal(&legacy, &report.observed);
    assert!(report.diagnostics.rejected_macro_steps > 0);
    assert_eq!(
        report.diagnostics.accepted_macro_steps,
        legacy.internal_steps
    );
}
