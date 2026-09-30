//! Output sampling is invariant under a shift of the time origin and a
//! change of time unit (audit 2026-09-30, AD-01).
//!
//! y' = v, y(t0) = 0, exact y(t) = v (t - t0). Before the fix the collector
//! merged times within 128 eps max(|t|, 1) of a step endpoint: at t0 = 1e12
//! outputs 0.01 apart were snapped onto endpoints (max error 0.020), and on
//! a span of 1e-15 every request was consumed at the first endpoint, so the
//! successful run returned y(tf) = 0.25 instead of 1.

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, IntegrationMethod, OdeProblem, OutputSamplingPlan, OutputSchedule,
    integrate_adaptive, integrate_adaptive_observed, integrate_fixed,
    integrate_sequential_matrix_free_adaptive_dense_observed,
};

fn constant_velocity(velocity: f64) -> OdeProblem {
    OdeProblem::new(
        "constant-velocity",
        1,
        Arc::new(move |_, _, out: &mut [f64]| {
            out[0] = velocity;
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| DenseMatrix::from_rows(&[&[0.0]]))),
        Some(Arc::new(|_, _, _, out: &mut [f64]| {
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

const CASES: [(f64, f64, f64); 3] = [(0.0, 1.0, 1.0), (1.0e12, 1.0, 1.0), (0.0, 1.0e-15, 1.0e15)];

fn schedule(origin: f64, duration: f64) -> Vec<f64> {
    (0..=100)
        .map(|i| origin + duration * (i as f64) / 100.0)
        .collect()
}

#[test]
fn dense_outputs_do_not_depend_on_the_time_origin_or_unit() {
    for (origin, duration, velocity) in CASES {
        let problem = constant_velocity(velocity);
        let sampling =
            OutputSamplingPlan::dense(OutputSchedule::new(schedule(origin, duration)).unwrap());
        let adaptive = AdaptiveStepConfig {
            atol: 1.0e-10,
            rtol: 1.0e-9,
            initial_step: duration / 4.0,
            min_step: duration * 1.0e-10,
            max_step: duration / 4.0,
            max_attempts: 1000,
            ..AdaptiveStepConfig::default()
        };
        let linear = LinearSolverConfig {
            method: LinearMethod::Gmres,
            ..LinearSolverConfig::default()
        };
        let run = integrate_sequential_matrix_free_adaptive_dense_observed(
            &problem,
            (origin, origin + duration),
            &[0.0],
            &linear,
            &adaptive,
            &sampling,
        )
        .unwrap();
        let observed = run.observed;
        assert!(observed.success, "{origin:e}, {duration:e}");
        assert_eq!(observed.t.len(), 101);
        // Errors relative to the solution scale v * duration = 1. At t0 = 1e12
        // the requested times themselves are quantized to ulp(1e12) = 1.2e-4
        // of a unit, so the exact value is taken at the represented time.
        let worst = observed
            .t
            .iter()
            .zip(&observed.y)
            .map(|(t, y)| (y[0] - velocity * (t - origin)).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            worst <= 1.0e-12,
            "origin {origin:e}, duration {duration:e}: max error {worst:e}"
        );
        let last = observed.y.last().unwrap()[0];
        assert!((last - 1.0).abs() <= 1.0e-12, "final value {last}");
    }
}

#[test]
fn clipped_outputs_do_not_depend_on_the_time_origin_or_unit() {
    for (origin, duration, velocity) in CASES {
        let problem = constant_velocity(velocity);
        let output = OutputSchedule::new(schedule(origin, duration)).unwrap();
        let observed = integrate_adaptive_observed(
            &problem,
            (origin, origin + duration),
            &[0.0],
            duration / 4.0,
            IntegrationMethod::Sequential,
            None,
            None,
            1.0e-10,
            1.0e-9,
            10_000,
            duration / 4.0,
            &output,
        )
        .unwrap();
        assert!(observed.success, "{origin:e}, {duration:e}");
        assert_eq!(observed.t.len(), 101);
        let worst = observed
            .t
            .iter()
            .zip(&observed.y)
            .map(|(t, y)| (y[0] - velocity * (t - origin)).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            worst <= 1.0e-12,
            "origin {origin:e}, duration {duration:e}: max error {worst:e}"
        );
    }
}

#[test]
fn a_span_shorter_than_one_time_unit_is_integrated() {
    // The fixed-step loops ran `while t < tf - 10 eps max(|tf|, 1)`: on
    // [0, 1e-15] that is 0 < -1.2e-15, so no step was taken and the run
    // returned y(tf) = y0 = 0 with success. The adaptive drivers loop on
    // `t < tf` but share the same success test.
    let problem = constant_velocity(1.0e15);
    let result = integrate_fixed(
        &problem,
        (0.0, 1.0e-15),
        &[0.0],
        2.5e-16,
        IntegrationMethod::Sequential,
        None,
        None,
        1.0e-10,
        1.0e-9,
    )
    .unwrap();
    assert!(result.success);
    assert_eq!(result.t.len(), 5, "{:?}", result.t);
    let last = result.y.last().unwrap()[0];
    assert!((last - 1.0).abs() <= 1.0e-12, "y(tf) = {last}");

    let adaptive = integrate_adaptive(
        &problem,
        (0.0, 1.0e-15),
        &[0.0],
        2.5e-16,
        IntegrationMethod::Sequential,
        None,
        None,
        1.0e-10,
        1.0e-9,
        1000,
        2.5e-16,
    )
    .unwrap();
    assert!(adaptive.success);
    let last = adaptive.y.last().unwrap()[0];
    assert!((last - 1.0).abs() <= 1.0e-12, "adaptive y(tf) = {last}");
}
