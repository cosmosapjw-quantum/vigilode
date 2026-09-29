//! Interior (interpolant) error of the adaptive RODAS5P dense path (audit
//! F-002). The endpoint estimate never controls the interpolant; these tests
//! pin the opt-in estimate and its enforcement on stiff Prothero-Robinson.

use rodas5p_core::{LinearMethod, LinearSolverConfig, error_scale, wrms};
use rodas5p_integrators::{
    AdaptiveStepConfig, DenseErrorControl, IntegrationMethod, OutputSamplingPlan, OutputSchedule,
    integrate_adaptive_dense_observed_with_config,
    integrate_adaptive_dense_observed_with_dense_error_control, prothero_robinson_problem,
};

const LAMBDA: f64 = -1.0e5;
const RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];

fn run(
    rtol: f64,
    control: DenseErrorControl,
) -> (
    rodas5p_integrators::AdaptiveObservedIntegrationResult,
    rodas5p_integrators::DenseErrorReport,
    f64,
) {
    run_lambda(LAMBDA, rtol, control)
}

fn run_lambda(
    lambda: f64,
    rtol: f64,
    control: DenseErrorControl,
) -> (
    rodas5p_integrators::AdaptiveObservedIntegrationResult,
    rodas5p_integrators::DenseErrorReport,
    f64,
) {
    let (problem, y0) = prothero_robinson_problem(lambda, 0.0, 0.0);
    let atol = rtol * 1.0e-2;
    let adaptive = AdaptiveStepConfig {
        atol,
        rtol,
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
    let times = (0..=200).map(|i| i as f64 * 0.01).collect::<Vec<_>>();
    let sampling = OutputSamplingPlan::dense(OutputSchedule::new(times).unwrap());
    let (result, report) = integrate_adaptive_dense_observed_with_dense_error_control(
        &problem,
        (0.0, 2.0),
        &y0,
        IntegrationMethod::Sequential,
        Some(&linear),
        None,
        &adaptive,
        &sampling,
        control,
    )
    .unwrap();
    assert!(
        result.observed.success,
        "rtol {rtol:e}: {}",
        result.observed.message
    );
    // True interior error at the output times, in the case WRMS norm.
    let mut worst = 0.0_f64;
    for (t, y) in result.observed.t.iter().zip(&result.observed.y) {
        let exact = [t.sin()];
        let scale = error_scale(&exact, &exact, &[atol], rtol).unwrap();
        worst = worst.max(wrms(&[y[0] - exact[0]], &scale).unwrap());
    }
    (result, report, worst)
}

#[test]
fn off_mode_is_the_legacy_dense_path_bit_for_bit() {
    let (problem, y0) = prothero_robinson_problem(LAMBDA, 0.0, 0.0);
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 1.0e-3,
        ..AdaptiveStepConfig::default()
    };
    let linear = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let times = (0..=20).map(|i| i as f64 * 0.05).collect::<Vec<_>>();
    let sampling = OutputSamplingPlan::dense(OutputSchedule::new(times).unwrap());
    let legacy = integrate_adaptive_dense_observed_with_config(
        &problem,
        (0.0, 1.0),
        &y0,
        IntegrationMethod::Sequential,
        Some(&linear),
        None,
        &adaptive,
        &sampling,
    )
    .unwrap();
    for control in [DenseErrorControl::Off, DenseErrorControl::Report] {
        let (controlled, report) = integrate_adaptive_dense_observed_with_dense_error_control(
            &problem,
            (0.0, 1.0),
            &y0,
            IntegrationMethod::Sequential,
            Some(&linear),
            None,
            &adaptive,
            &sampling,
            control,
        )
        .unwrap();
        // Report never changes the run; its work is charged to the report.
        assert_eq!(controlled.observed.counters, legacy.observed.counters);
        assert_eq!(controlled.diagnostics, legacy.diagnostics);
        for (a, b) in controlled.observed.y.iter().zip(&legacy.observed.y) {
            assert_eq!(a[0].to_bits(), b[0].to_bits());
        }
        assert_eq!(report.samples.is_empty(), control == DenseErrorControl::Off);
    }
}

#[test]
fn adaptive_dense_interior_wrms_on_stiff_pr() {
    for rtol in RTOLS {
        let (_, report, true_interior) = run(rtol, DenseErrorControl::Report);
        let estimate = report.max_accepted_estimate().expect("interior samples");
        eprintln!("rtol {rtol:e}: true interior {true_interior:.3e}, estimate {estimate:.3e}");
        assert_eq!(report.unavailable, 0);
        assert!(
            estimate >= true_interior / 10.0 && estimate <= true_interior * 10.0,
            "rtol {rtol:e}: estimate {estimate:e} vs true {true_interior:e}"
        );
    }
}

#[test]
fn enforced_dense_error_bounds_the_true_interior_error() {
    for rtol in RTOLS {
        let (result, report, true_interior) = run(rtol, DenseErrorControl::Enforce);
        eprintln!(
            "rtol {rtol:e}: enforced true interior {true_interior:.3e}, {} steps, {} dense rejections",
            result.observed.internal_steps,
            report.samples.iter().filter(|s| !s.accepted).count()
        );
        assert!(true_interior <= 3.0, "rtol {rtol:e}: {true_interior:e}");
    }
}

#[test]
fn defect_report_needs_no_jacobian_and_overstates_stiff_components() {
    // Re-audit RA-07: a cheap mode for matrix-free runs. The unfiltered
    // defect costs one right-hand side per sampled step. On stiff
    // components it overstates the interior error; on nonstiff ones it
    // tracks it.
    for (lambda, rtol) in [(LAMBDA, 1.0e-6), (-1.0, 1.0e-6), (-1.0, 1.0e-8)] {
        let (_, report, true_interior) = run_lambda(lambda, rtol, DenseErrorControl::ReportDefect);
        let estimate = report.max_accepted_estimate().expect("interior samples");
        eprintln!(
            "lambda {lambda:e} rtol {rtol:e}: true interior {true_interior:.3e}, defect estimate {estimate:.3e}"
        );
        assert_eq!(report.counters.jacobian_builds, 0);
        assert_eq!(report.counters.direct_factorizations, 0);
        assert_eq!(report.counters.jvp_calls, 0);
        assert!(report.counters.rhs_evaluations > 0);
        assert!(estimate >= true_interior / 10.0, "lambda {lambda:e}");
        if lambda == -1.0 {
            assert!(estimate <= 10.0 * true_interior, "lambda {lambda:e}");
        }
    }
    assert_eq!(DenseErrorControl::default(), DenseErrorControl::Off);
}
