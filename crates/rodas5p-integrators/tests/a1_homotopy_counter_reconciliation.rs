//! Final review item 4 (production plan 2026-10-20, section 6, "the same
//! pattern in the dense variant, legacy SABR and homotopy drivers"): the
//! homotopy drivers gate an attempt twice, by the step kernel's verdict
//! (counted in the work counters) and by the outer `error <= 1` test on
//! `error_norm + fixed_point_error`. Both drivers now call
//! `reconcile_outer_rejection`, so the work counters equal the attempt
//! ledger whatever gate rejects.
//!
//! Note: through the public API the outer gate coincides with the kernel's
//! certificate gate (`embedded + output_wrms <= 1`) up to summation-order
//! roundoff, so an attempt the kernel accepts and the outer test rejects
//! cannot be forced deterministically. These tests pin the ledger on runs
//! with fast-path and fallback rejections in both drivers.

use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveObservedIntegrationResult, AdaptiveStepConfig, HomotopyPathConfig, HomotopyPredictor,
    HomotopyStepConfig, OutputSamplingPlan, OutputSchedule,
    integrate_homotopy_adaptive_dense_observed, integrate_homotopy_adaptive_observed,
    scalar_linear_problem,
};

fn adaptive(initial_step: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-11,
        rtol: 1.0e-7,
        initial_step,
        min_step: 1.0e-12,
        max_step: initial_step,
        max_attempts: 20_000,
        ..AdaptiveStepConfig::default()
    }
}

fn check_ledger(result: &AdaptiveObservedIntegrationResult, what: &str) {
    let c = &result.observed.counters;
    let d = &result.diagnostics;
    assert!(
        result.observed.success,
        "{what}: {}",
        result.observed.message
    );
    assert!(d.is_structurally_consistent(), "{what}: ledger");
    assert!(d.rejected_macro_steps > 0, "{what}: no rejection exercised");
    assert_eq!(
        c.accepted_steps + c.rejected_steps,
        d.attempts as u64,
        "{what}: accepted {} + rejected {} != attempts {}",
        c.accepted_steps,
        c.rejected_steps,
        d.attempts
    );
    assert_eq!(
        (c.accepted_steps, c.rejected_steps),
        (d.accepted_macro_steps as u64, d.rejected_macro_steps as u64),
        "{what}: counters (accepted, rejected) vs ledger"
    );
    assert_eq!(
        c.accepted_steps, result.observed.internal_steps as u64,
        "{what}: accepted steps vs retained steps"
    );
}

/// (homotopy configuration, fallback, lambda, initial step, label)
fn cases() -> Vec<(
    HomotopyStepConfig,
    LinearSolverConfig,
    f64,
    f64,
    &'static str,
)> {
    let fast = HomotopyPathConfig::new(0.4, 7, 2, HomotopyPredictor::Euler, 1).unwrap();
    let fallback_only = HomotopyPathConfig::new(0.0, 0, 1, HomotopyPredictor::Euler, 0).unwrap();
    vec![
        (
            HomotopyStepConfig::new(fast, 0.1).unwrap(),
            LinearSolverConfig {
                method: LinearMethod::Direct,
                ..LinearSolverConfig::default()
            },
            -100.0,
            0.1,
            "fast path with fallback rejections",
        ),
        (
            HomotopyStepConfig::new(fallback_only, 0.0).unwrap(),
            LinearSolverConfig {
                method: LinearMethod::Lgmres,
                ..LinearSolverConfig::default()
            },
            -20.0,
            0.15,
            "fallback",
        ),
    ]
}

#[test]
fn homotopy_observed_driver_counters_equal_the_attempt_ledger() {
    for (homotopy, fallback, lambda, h0, what) in cases() {
        let (problem, y0) = scalar_linear_problem(lambda, 1.0);
        let result = integrate_homotopy_adaptive_observed(
            &problem,
            (0.0, 0.2),
            &y0,
            &homotopy,
            Some(&fallback),
            &adaptive(h0),
            &OutputSchedule::uniform(0.0, 0.2, 0.02).unwrap(),
        )
        .unwrap();
        check_ledger(&result, what);
    }
}

#[test]
fn homotopy_dense_driver_counters_equal_the_attempt_ledger() {
    for (homotopy, fallback, lambda, h0, what) in cases() {
        let (problem, y0) = scalar_linear_problem(lambda, 1.0);
        let sampling = OutputSamplingPlan::dense(OutputSchedule::uniform(0.0, 0.2, 0.02).unwrap());
        let result = integrate_homotopy_adaptive_dense_observed(
            &problem,
            (0.0, 0.2),
            &y0,
            &homotopy,
            Some(&fallback),
            &adaptive(h0),
            &sampling,
        )
        .unwrap();
        check_ledger(&result, what);
    }
}
