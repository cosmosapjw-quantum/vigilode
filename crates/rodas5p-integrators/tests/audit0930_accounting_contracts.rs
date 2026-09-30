//! Step ledgers record the final disposition (audit 2026-09-30, AD-02).

use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, DenseErrorControl, IntegrationMethod, OutputSamplingPlan, OutputSchedule,
    integrate_adaptive_dense_observed_with_dense_error_control, prothero_robinson_problem,
};

#[test]
fn dense_enforcement_rejections_are_counted_as_rejections() {
    // Before: Off 7/0, Enforce counters 26 accepted / 0 rejected for 12
    // retained steps and 14 dense rejections.
    let (problem, y0) = prothero_robinson_problem(-1.0e5, 0.0, 0.0);
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 1.0e-3,
        min_step: 1.0e-12,
        max_step: 1.0,
        max_attempts: 100_000,
        ..AdaptiveStepConfig::default()
    };
    let linear = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let sampling = OutputSamplingPlan::dense(
        OutputSchedule::new((0..=200).map(|i| i as f64 * 0.01).collect()).unwrap(),
    );
    let mut dense_rejections = Vec::new();
    for mode in [DenseErrorControl::Off, DenseErrorControl::Enforce] {
        let (result, report) = integrate_adaptive_dense_observed_with_dense_error_control(
            &problem,
            (0.0, 2.0),
            &y0,
            IntegrationMethod::Sequential,
            Some(&linear),
            None,
            &adaptive,
            &sampling,
            mode,
        )
        .unwrap();
        assert!(result.observed.success);
        let counters = result.observed.counters;
        assert_eq!(
            counters.accepted_steps as usize, result.observed.internal_steps,
            "{mode:?}"
        );
        assert_eq!(
            counters.accepted_steps as usize, result.diagnostics.accepted_macro_steps,
            "{mode:?}"
        );
        assert_eq!(
            counters.rejected_steps as usize, result.diagnostics.rejected_macro_steps,
            "{mode:?}"
        );
        dense_rejections.push(
            report
                .samples
                .iter()
                .filter(|sample| !sample.accepted)
                .count(),
        );
    }
    assert_eq!(dense_rejections[0], 0);
    assert!(
        dense_rejections[1] > 0,
        "the case must exercise enforcement"
    );
}
