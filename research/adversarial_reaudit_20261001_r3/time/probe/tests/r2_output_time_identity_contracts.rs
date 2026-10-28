//! Represented-time identity (re-audit R2 of 2026-09-30, R2-OUT-01).
//!
//! Before: a span of 8 ULP at t0 = +-1e12 finished with zero attempts and
//! y = 0, a dense request at next_up(t_endpoint) was answered with the
//! endpoint value (error -1 at velocity 8192), and two adjacent
//! representable output times were rejected as duplicates.

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, BdfConfig, IntegrationMethod, OdeProblem, OutputSamplingPlan,
    OutputSchedule, RadauConfig, integrate_adaptive_observed, integrate_bdf_fixed_observed,
    integrate_fixed, integrate_fixed_observed, integrate_radau_fixed,
    integrate_radau_fixed_observed, integrate_sequential_matrix_free_adaptive_dense_observed,
    scalar_linear_problem,
};

fn flow(velocity: f64) -> OdeProblem {
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

fn ulps_after(origin: f64, ulps: u64) -> f64 {
    if origin > 0.0 {
        f64::from_bits(origin.to_bits() + ulps)
    } else {
        f64::from_bits(origin.to_bits() - ulps)
    }
}

fn fixed(
    origin: f64,
    tf: f64,
    h: f64,
) -> rodas5p_core::CoreResult<rodas5p_integrators::IntegrationResult> {
    integrate_fixed(
        &flow(1.0 / (tf - origin)),
        (origin, tf),
        &[0.0],
        h,
        IntegrationMethod::Sequential,
        None,
        None,
        1.0e-10,
        1.0e-9,
    )
}

#[test]
fn a_short_representable_span_at_a_large_epoch_is_integrated() {
    for (origin, ulps, halves) in [
        (1.0e12, 8, true),
        (-1.0e12, 8, true),
        (1.0, 1, false),
        (-1.0, 3, false),
    ] {
        let tf = ulps_after(origin, ulps);
        let span = tf - origin;
        let h = if halves { span / 2.0 } else { span };
        let result = fixed(origin, tf, h).unwrap();
        assert!(result.success, "origin {origin:e}");
        assert!(
            result.attempts > 0,
            "origin {origin:e}: zero-attempt success"
        );
        assert_eq!(*result.t.last().unwrap(), tf, "origin {origin:e}");
        let y = result.y.last().unwrap()[0];
        assert!((y - 1.0).abs() <= 1.0e-12, "origin {origin:e}: y = {y}");
    }
}

#[test]
fn a_step_below_half_an_ulp_is_a_typed_failure() {
    // t + h == t: this looped once the end slack was removed, and reported
    // success with zero attempts before that.
    let tf = 1.0_f64.next_up();
    let error = fixed(1.0, tf, (tf - 1.0) / 2.0).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not advance the represented time"),
        "{error}"
    );
}

#[test]
fn a_request_one_ulp_after_an_endpoint_is_interpolated_not_snapped() {
    let origin = 1.0e12_f64;
    let velocity = 8192.0;
    let first = origin + 0.25;
    let after = first.next_up();
    let sampling = OutputSamplingPlan::dense(
        OutputSchedule::new(vec![origin, first, after, origin + 1.0]).unwrap(),
    );
    let config = AdaptiveStepConfig {
        atol: 1.0e-10,
        rtol: 1.0e-9,
        initial_step: 0.25,
        min_step: 1.0e-14,
        max_step: 0.25,
        max_attempts: 100,
        ..AdaptiveStepConfig::default()
    };
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        ..LinearSolverConfig::default()
    };
    let result = integrate_sequential_matrix_free_adaptive_dense_observed(
        &flow(velocity),
        (origin, origin + 1.0),
        &[0.0],
        &linear,
        &config,
        &sampling,
    )
    .unwrap()
    .observed;
    assert!(result.success);
    assert_eq!(result.t, vec![origin, first, after, origin + 1.0]);
    for (t, y) in result.t.iter().zip(&result.y) {
        let exact = velocity * (t - origin);
        assert!(
            (y[0] - exact).abs() <= 1.0e-9 * exact.max(1.0),
            "t = {t}: {} vs {exact}",
            y[0]
        );
    }
    // The request after the endpoint is 2049, not the endpoint's 2048.
    assert!(result.y[2][0] > result.y[1][0] + 0.5);
}

#[test]
fn adjacent_representable_output_times_are_distinct_requests() {
    let origin = 1.0e12_f64;
    let tf = origin + 1.0;
    let output = OutputSchedule::new(vec![origin, origin.next_up(), tf]).unwrap();
    let result = integrate_adaptive_observed(
        &flow(1.0),
        (origin, tf),
        &[0.0],
        0.25,
        IntegrationMethod::Sequential,
        None,
        None,
        1.0e-10,
        1.0e-9,
        1000,
        0.25,
        &output,
    )
    .unwrap();
    assert!(result.success);
    assert_eq!(result.t, vec![origin, origin.next_up(), tf]);
    let spacing = origin.next_up() - origin;
    assert!((result.y[1][0] - spacing).abs() <= 1.0e-12);
    assert!((result.y[2][0] - 1.0).abs() <= 1.0e-9);
}

// Ordinary runs whose accumulated time falls a rounding residue short of a
// target (independent review of the first R2 patch): they must land on the
// target in the same number of steps as before, not stop short and not take
// an extra 1e-16 step.

#[test]
fn fixed_steps_that_accumulate_below_the_end_land_on_it() {
    let result = fixed(0.0, 1.0, 0.1).unwrap();
    assert_eq!(result.t.len(), 11, "ten steps of 0.1, no 1e-16 step");
    assert_eq!(*result.t.last().unwrap(), 1.0);
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let radau = integrate_radau_fixed_observed(
        &problem,
        (0.0, 1.0),
        &y0,
        0.1,
        &RadauConfig::default(),
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
    )
    .unwrap();
    assert!(radau.success);
    assert_eq!(radau.t, vec![0.0, 1.0]);
    assert_eq!(radau.internal_steps, 10);
}

#[test]
fn a_uniform_output_grid_adds_no_micro_steps() {
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let grid = OutputSchedule::uniform(0.0, 0.2, 0.04).unwrap();
    let rodas = integrate_fixed_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.01,
        IntegrationMethod::Sequential,
        None,
        None,
        1.0e-10,
        1.0e-9,
        &grid,
    )
    .unwrap();
    assert!(rodas.success);
    assert_eq!(rodas.internal_steps, 20);
    assert_eq!(rodas.output_clipped_steps, 0);
    for h in [0.02, 0.01] {
        let bdf = integrate_bdf_fixed_observed(
            &problem,
            (0.0, 0.2),
            &y0,
            h,
            &BdfConfig::default(),
            &grid,
        )
        .unwrap();
        assert!(bdf.success);
        let steps = (0.2_f64 / h).round() as usize;
        assert_eq!(bdf.internal_steps, steps, "h = {h}");
        assert_eq!(bdf.output_clipped_steps, 0, "h = {h}");
        let exact = (-0.4_f64).exp();
        let error = (bdf.y.last().unwrap()[0] - exact).abs();
        // BDF2 after one BDF1 start: 6.8e-4 at h = 0.02, 1.7e-4 at h = 0.01.
        assert!(error <= 8.0e-4 * (h / 0.02).powi(2), "h = {h}: {error:e}");
    }
}

#[test]
fn radau_fixed_steps_cover_a_short_span_at_a_large_epoch() {
    let origin = 1.0e12_f64;
    let tf = ulps_after(origin, 8);
    let (problem, y0) = scalar_linear_problem(-1.0, 1.0);
    let result = integrate_radau_fixed(
        &problem,
        (origin, tf),
        &y0,
        (tf - origin) / 2.0,
        &RadauConfig::default(),
    )
    .unwrap();
    assert!(result.steps > 0);
    assert_eq!(*result.t.last().unwrap(), tf);
}

#[test]
fn an_adaptive_step_across_zero_reaches_the_end() {
    // No binary64 h has -0.7 + h == 0.3; the step lands one ULP short and a
    // final piece below min_step completes the span.
    for (t0, tf) in [(-0.7, 0.3), (-1.0, 0.1), (-0.1, 0.05)] {
        let sampling = OutputSamplingPlan::dense(OutputSchedule::new(vec![t0, tf]).unwrap());
        let config = AdaptiveStepConfig {
            atol: 1.0e-10,
            rtol: 1.0e-9,
            initial_step: 1.0,
            max_step: 1.0,
            max_attempts: 100,
            ..AdaptiveStepConfig::default()
        };
        let linear = LinearSolverConfig {
            method: LinearMethod::Gmres,
            ..LinearSolverConfig::default()
        };
        let result = integrate_sequential_matrix_free_adaptive_dense_observed(
            &flow(1.0),
            (t0, tf),
            &[0.0],
            &linear,
            &config,
            &sampling,
        )
        .unwrap()
        .observed;
        assert!(result.success, "({t0}, {tf})");
        assert_eq!(result.t, vec![t0, tf]);
        assert!((result.y[1][0] - (tf - t0)).abs() <= 1.0e-12);
    }
}

#[test]
fn grids_through_zero_from_negative_times_add_no_micro_steps() {
    // Second review, N1: the residue collapsed near zero, so
    // -0.3 + 3 * 0.1 = 5.6e-17 missed the request at 0 and a 2.8e-17 step
    // followed.
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    for (t0, tf, h, spacing) in [(-0.3, 0.7, 0.1, 0.1), (-0.2, 0.8, 0.05, 0.2)] {
        let grid = OutputSchedule::uniform(t0, tf, spacing).unwrap();
        let steps = ((tf - t0) / h).round() as usize;
        let rodas = integrate_fixed_observed(
            &problem,
            (t0, tf),
            &y0,
            h,
            IntegrationMethod::Sequential,
            None,
            None,
            1.0e-10,
            1.0e-9,
            &grid,
        )
        .unwrap();
        assert_eq!(rodas.internal_steps, steps, "({t0}, {tf})");
        assert_eq!(rodas.output_clipped_steps, 0, "({t0}, {tf})");
        let bdf =
            integrate_bdf_fixed_observed(&problem, (t0, tf), &y0, h, &BdfConfig::default(), &grid)
                .unwrap();
        assert_eq!(bdf.internal_steps, steps, "BDF ({t0}, {tf})");
    }
    for t0 in [-1.0, -0.5, -0.7, -1.3] {
        let result = fixed(t0, 0.0, 0.1).unwrap();
        assert_eq!(
            result.t.len(),
            (-t0 / 0.1).round() as usize + 1,
            "t0 = {t0}"
        );
        assert_eq!(*result.t.last().unwrap(), 0.0);
    }
}
