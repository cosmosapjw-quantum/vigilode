//! The stage equations integrate the interval the clock records (re-audit
//! R3 of 2026-10-01: R3-TIME-01, R3-TIME-02).
//!
//! At t0 = +-1e12 one ULP is 1.22e-4. A nominal step of 1e-4 moved the clock
//! by one ULP while the stages integrated 1e-4, so eight steps over an 8-ULP
//! span of y' = 1/span returned y(tf) = 0.8192 with success.

use std::sync::Arc;

use rodas5p_core::DenseMatrix;
use rodas5p_integrators::{
    AdaptiveStepConfig, BdfConfig, BdfOrder, FusedPhiKrylovConfig, IntegrationMethod,
    ObservedIntegrationResult, OdeProblem, OutputSamplingPlan, OutputSchedule, ParallelExecution,
    RadauConfig, integrate_adaptive_observed_with_config, integrate_bdf_fixed,
    integrate_bdf_fixed_observed, integrate_fixed, integrate_fixed_dense_observed,
    integrate_fixed_observed, integrate_pexprb54s4_fused_adaptive_observed,
    integrate_radau_fixed_observed,
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

/// y' = cos(t - shift), y(t0) = sin(t0 - shift).
fn shifted_sine(shift: f64) -> OdeProblem {
    OdeProblem::new(
        "shifted-sine",
        1,
        Arc::new(move |t: f64, _, out: &mut [f64]| {
            out[0] = (t - shift).cos();
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| DenseMatrix::from_rows(&[&[0.0]]))),
        Some(Arc::new(move |t: f64, _, _, out: &mut [f64]| {
            out[0] = -(t - shift).sin();
            Ok(())
        })),
        None,
        false,
        None,
        None,
    )
    .unwrap()
}

fn ulps_after(t: f64, n: u64) -> f64 {
    (0..n).fold(t, |value, _| value.next_up())
}

fn assert_endpoint_or_time_resolution(
    label: &str,
    result: Result<ObservedIntegrationResult, String>,
) {
    match result {
        Ok(observed) => {
            assert!(observed.success, "{label}: {}", observed.message);
            let y = observed.y.last().unwrap()[0];
            assert!((y - 1.0).abs() <= 1.0e-12, "{label}: y(tf) = {y}");
        }
        Err(error) => assert!(error.contains("time resolution"), "{label}: {error}"),
    }
}

#[test]
fn an_8_ulp_span_with_a_sub_ulp_nominal_step_integrates_its_represented_length() {
    for origin in [1.0e12, -1.0e12] {
        // Eight ULPs toward +inf from either origin.
        let (t0, tf) = (origin, ulps_after(origin, 8));
        let velocity = 1.0 / (tf - t0);
        let problem = flow(velocity);
        let output = OutputSchedule::new(vec![t0, tf]).unwrap();
        let sampling = OutputSamplingPlan::dense(output.clone());
        let fixed = integrate_fixed(
            &problem,
            (t0, tf),
            &[0.0],
            1.0e-4,
            IntegrationMethod::Sequential,
            None,
            None,
            1.0e-10,
            1.0e-9,
        )
        .unwrap();
        assert!(fixed.success);
        assert!((fixed.y.last().unwrap()[0] - 1.0).abs() <= 1.0e-12);
        // Receipt: every step integrated the interval between its times.
        for (k, h) in fixed.step_sizes.iter().enumerate() {
            assert_eq!(fixed.t[k] + h, fixed.t[k + 1]);
            assert_eq!(*h, fixed.t[k + 1] - fixed.t[k]);
        }
        let fixed_observed = |result: Result<ObservedIntegrationResult, String>, label: &str| {
            assert_endpoint_or_time_resolution(&format!("{label} at {origin:e}"), result)
        };
        fixed_observed(
            integrate_radau_fixed_observed(
                &problem,
                (t0, tf),
                &[0.0],
                1.0e-4,
                &RadauConfig::default(),
                &output,
            )
            .map_err(|e| e.to_string()),
            "fixed Radau",
        );
        fixed_observed(
            integrate_bdf_fixed_observed(
                &problem,
                (t0, tf),
                &[0.0],
                1.0e-4,
                &BdfConfig::default(),
                &output,
            )
            .map_err(|e| e.to_string()),
            "fixed BDF",
        );
        fixed_observed(
            integrate_fixed_dense_observed(
                &problem,
                (t0, tf),
                &[0.0],
                1.0e-4,
                IntegrationMethod::Sequential,
                None,
                None,
                1.0e-10,
                1.0e-9,
                &sampling,
            )
            .map_err(|e| e.to_string()),
            "fixed dense Rodas",
        );
        let adaptive = AdaptiveStepConfig {
            atol: 1.0e-10,
            rtol: 1.0e-9,
            initial_step: 1.0e-4,
            min_step: 1.0e-14,
            max_step: 1.0e-4,
            max_attempts: 100,
            ..AdaptiveStepConfig::default()
        };
        // max_step 1e-4 is below one ULP: a typed time-resolution failure,
        // never a silently enlarged step or a 0.8192 endpoint.
        fixed_observed(
            integrate_adaptive_observed_with_config(
                &problem,
                (t0, tf),
                &[0.0],
                IntegrationMethod::Sequential,
                None,
                None,
                &adaptive,
                &output,
            )
            .map(|r| r.observed)
            .map_err(|e| e.to_string()),
            "adaptive Rodas",
        );
        fixed_observed(
            integrate_pexprb54s4_fused_adaptive_observed(
                &problem,
                (t0, tf),
                &[0.0],
                &adaptive,
                &output,
                FusedPhiKrylovConfig::default(),
                &ParallelExecution::sequential(),
            )
            .map(|r| r.observed)
            .map_err(|e| e.to_string()),
            "adaptive fused exponential",
        );
        let wider = AdaptiveStepConfig {
            max_step: 1.0e-3,
            ..adaptive
        };
        fixed_observed(
            integrate_adaptive_observed_with_config(
                &problem,
                (t0, tf),
                &[0.0],
                IntegrationMethod::Sequential,
                None,
                None,
                &wider,
                &output,
            )
            .map(|r| r.observed)
            .map_err(|e| e.to_string()),
            "adaptive Rodas, max_step above one ULP",
        );
    }
}

#[test]
fn power_of_two_rescaling_and_origin_shift_separate_representation_from_truncation() {
    // The same local problem at origin 0 and after exact power-of-two
    // changes of the time unit gives the same state bit for bit; at a large
    // origin the only difference is representational.
    let run = |t0: f64, tf: f64, h: f64, velocity: f64| {
        integrate_fixed(
            &flow(velocity),
            (t0, tf),
            &[0.0],
            h,
            IntegrationMethod::Sequential,
            None,
            None,
            1.0e-10,
            1.0e-9,
        )
        .unwrap()
    };
    let base = run(0.0, 1.0, 0.125, 1.0);
    for power in [-20, -3, 3, 20] {
        let unit = 2.0_f64.powi(power);
        let scaled = run(0.0, unit, 0.125 * unit, 1.0 / unit);
        assert_eq!(scaled.y.last().unwrap()[0], base.y.last().unwrap()[0]);
        assert_eq!(scaled.t.len(), base.t.len());
    }
    // A nonautonomous analytic problem at origin 0 and at 2^30: the error
    // against sin is truncation at 0 and truncation plus the conditioning
    // eps * 2^30 of the represented clock at 2^30.
    for shift in [0.0, 2.0_f64.powi(30)] {
        let result = integrate_fixed(
            &shifted_sine(shift),
            (shift, shift + 1.0),
            &[0.0],
            1.0 / 64.0,
            IntegrationMethod::Sequential,
            None,
            None,
            1.0e-10,
            1.0e-9,
        )
        .unwrap();
        let error = (result.y.last().unwrap()[0] - 1.0_f64.sin()).abs();
        let conditioning = 64.0 * f64::EPSILON * shift;
        assert!(
            error <= 1.0e-9 + conditioning,
            "shift {shift}: error {error:e}"
        );
        for (k, h) in result.step_sizes.iter().enumerate() {
            assert_eq!(result.t[k] + h, result.t[k + 1]);
        }
    }
}

#[test]
fn bdf_feeds_the_represented_steps_across_a_power_of_two_boundary() {
    // Below 2^40 one ULP is 1.22e-4, above it 2.44e-4; a nominal 1.5e-4
    // step is one ULP on either side, so the represented step doubles at the
    // boundary. The fixed-step BDF2 then uses the variable-step coefficients
    // of the actual steps instead of assuming equal spacing, and stays exact
    // on a linear solution.
    let boundary = 2.0_f64.powi(40);
    let t0 = boundary - 8.0 * (boundary - boundary.next_down());
    let tf = boundary + 8.0 * (boundary.next_up() - boundary);
    let velocity = 1.0 / (tf - t0);
    let result = integrate_bdf_fixed(
        &flow(velocity),
        (t0, tf),
        &[0.0],
        1.5e-4,
        &BdfConfig::default(),
    )
    .unwrap();
    assert_eq!(*result.t.last().unwrap(), tf);
    assert!((result.y.last().unwrap()[0] - 1.0).abs() <= 1.0e-12);
    // The indexed grid t0 + 1.5e-4 k is represented at one or two ULPs per
    // step; the represented step changes size across the boundary.
    let steps = result
        .t
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect::<Vec<_>>();
    assert!(steps.iter().any(|h| *h > 2.0e-4) && steps.iter().any(|h| *h < 1.3e-4));
    assert_eq!(result.applied_orders[0], BdfOrder::One);
    assert!(
        result.applied_orders[1..]
            .iter()
            .all(|order| *order == BdfOrder::Two),
        "{:?}",
        result.applied_orders
    );
}

#[test]
fn nonzero_spans_need_their_exact_endpoints() {
    let origin = 1.0e12_f64;
    let tf = ulps_after(origin, 4);
    let span = tf - origin;
    // A 4-ULP span with spacing 1 has no intervals: rejected, not [tf].
    assert!(OutputSchedule::uniform(origin, tf, 1.0).is_err());
    let problem = flow(1.0 / span);
    let run = |output: &OutputSchedule| {
        (
            integrate_fixed_observed(
                &problem,
                (origin, tf),
                &[0.0],
                span / 2.0,
                IntegrationMethod::Sequential,
                None,
                None,
                1.0e-10,
                1.0e-9,
                output,
            )
            .map_err(|e| e.to_string()),
            integrate_fixed_dense_observed(
                &problem,
                (origin, tf),
                &[0.0],
                span / 2.0,
                IntegrationMethod::Sequential,
                None,
                None,
                1.0e-10,
                1.0e-9,
                &OutputSamplingPlan::dense(output.clone()),
            )
            .map_err(|e| e.to_string()),
        )
    };
    // [tf] for t0 < tf, and endpoints one ULP off, are invalid schedules.
    for times in [
        vec![tf],
        vec![origin.next_up(), tf],
        vec![origin, tf.next_up()],
    ] {
        let output = OutputSchedule::new(times.clone()).unwrap();
        let (clipped, dense) = run(&output);
        assert!(clipped.is_err(), "{times:?}");
        assert!(dense.is_err(), "{times:?}");
    }
    let (clipped, dense) = run(&OutputSchedule::new(vec![origin, tf]).unwrap());
    for result in [clipped.unwrap(), dense.unwrap()] {
        assert_eq!(result.t, vec![origin, tf]);
        assert!((result.y[1][0] - 1.0).abs() <= 1.0e-12);
    }
    // Adjacent distinct endpoints stay distinct: a subnormal span and a
    // one-ULP span at a large epoch.
    for (t0, t1) in [(0.0, f64::from_bits(1)), (origin, origin.next_up())] {
        let output = OutputSchedule::new(vec![t0, t1]).unwrap();
        let result = integrate_fixed_observed(
            &flow(1.0),
            (t0, t1),
            &[0.0],
            t1 - t0,
            IntegrationMethod::Sequential,
            None,
            None,
            1.0e-10,
            1.0e-9,
            &output,
        )
        .unwrap();
        assert_eq!(result.t, vec![t0, t1]);
    }
    // A single-point schedule is valid only for a zero span.
    let zero = integrate_fixed_observed(
        &flow(1.0),
        (origin, origin),
        &[0.0],
        1.0,
        IntegrationMethod::Sequential,
        None,
        None,
        1.0e-10,
        1.0e-9,
        &OutputSchedule::new(vec![origin]).unwrap(),
    )
    .unwrap();
    assert_eq!(zero.t, vec![origin]);
}

#[test]
fn a_long_fixed_run_steps_on_the_indexed_grid() {
    // Re-audit R3, R3-TIME-03: 1000 steps of 0.01 on (0, 10) used to end
    // 1.7e-13 short of 10 and add a micro-step.
    let problem = shifted_sine(0.0);
    let result = integrate_fixed(
        &problem,
        (0.0, 10.0),
        &[0.0],
        0.01,
        IntegrationMethod::Sequential,
        None,
        None,
        1.0e-10,
        1.0e-9,
    )
    .unwrap();
    assert_eq!(result.t.len(), 1001);
    for (k, t) in result.t.iter().enumerate() {
        let indexed = if k == 1000 { 10.0 } else { k as f64 * 0.01 };
        assert_eq!(*t, indexed, "grid point {k}");
    }
    assert!((result.y.last().unwrap()[0] - 10.0_f64.sin()).abs() <= 1.0e-8);
    let bdf =
        integrate_bdf_fixed(&problem, (0.0, 10.0), &[0.0], 0.01, &BdfConfig::default()).unwrap();
    assert_eq!(bdf.t.len(), 1001);
    assert_eq!(bdf.startup_steps, 1);
    assert_eq!(bdf.applied_orders[0], BdfOrder::One);
    assert!(bdf.applied_orders[1..].iter().all(|o| *o == BdfOrder::Two));
    assert!((bdf.y.last().unwrap()[0] - 10.0_f64.sin()).abs() <= 1.0e-4);
    assert_ne!(
        rodas5p_integrators::PRODUCTION_CLOCK_POLICY,
        rodas5p_integrators::RESEARCH_REPLAY_CLOCK_POLICY
    );
}

// Independent review of the first R3 patch.

#[test]
fn adaptive_runs_cruising_at_max_step_land_on_outputs_without_micro_steps() {
    let output = OutputSchedule::uniform(0.0, 1.0, 0.01).unwrap();
    for (min_step, label) in [(1.0e-14, "min 1e-14"), (0.01, "min == max")] {
        let config = AdaptiveStepConfig {
            atol: 1.0e-10,
            rtol: 1.0e-9,
            initial_step: 0.01,
            min_step,
            max_step: 0.01,
            max_attempts: 1_000,
            ..AdaptiveStepConfig::default()
        };
        let result = integrate_adaptive_observed_with_config(
            &flow(1.0),
            (0.0, 1.0),
            &[0.0],
            IntegrationMethod::Sequential,
            None,
            None,
            &config,
            &output,
        )
        .unwrap()
        .observed;
        assert!(result.success, "{label}: {}", result.message);
        assert_eq!(result.internal_steps, 100, "{label}");
        assert!((result.y.last().unwrap()[0] - 1.0).abs() <= 1.0e-12);
    }
}

#[test]
fn decimal_uniform_grids_are_admitted() {
    for (start, end, spacing) in [(1.1, 1.2, 0.1), (1000.1, 1000.7, 0.2), (0.3, 2.7, 0.3)] {
        let schedule = OutputSchedule::uniform(start, end, spacing).unwrap();
        assert_eq!(schedule.times()[0], start);
        assert_eq!(*schedule.times().last().unwrap(), end);
    }
}

#[test]
fn literal_output_times_do_not_add_steps_to_the_indexed_grid() {
    // Outputs i / 100 differ from the indexed 0.01 * i by an ULP at some i;
    // the grid point just after a clipped landing counts as reached.
    let output = OutputSchedule::new((0..=100).map(|i| i as f64 / 100.0).collect()).unwrap();
    let problem = shifted_sine(0.0);
    let rodas = integrate_fixed_observed(
        &problem,
        (0.0, 1.0),
        &[0.0],
        0.01,
        IntegrationMethod::Sequential,
        None,
        None,
        1.0e-10,
        1.0e-9,
        &output,
    )
    .unwrap();
    assert_eq!(rodas.internal_steps, 100);
    let bdf = integrate_bdf_fixed_observed(
        &problem,
        (0.0, 1.0),
        &[0.0],
        0.01,
        &BdfConfig::default(),
        &output,
    )
    .unwrap();
    assert_eq!(bdf.internal_steps, 100);
    assert!((bdf.y.last().unwrap()[0] - 1.0_f64.sin()).abs() <= 1.0e-4);
}

#[test]
fn a_step_across_zero_integrates_its_represented_interval() {
    // Opposite-sign endpoints: -0.7 + 1.0 has no exact landing on 0.3; the
    // represented interval is integrated in every step.
    let result = integrate_fixed(
        &shifted_sine(0.0),
        (-0.7, 0.3),
        &[(-0.7_f64).sin()],
        0.1,
        IntegrationMethod::Sequential,
        None,
        None,
        1.0e-10,
        1.0e-9,
    )
    .unwrap();
    assert_eq!(*result.t.last().unwrap(), 0.3);
    for (k, h) in result.step_sizes.iter().enumerate() {
        assert_eq!(result.t[k] + h, result.t[k + 1]);
    }
    assert!((result.y.last().unwrap()[0] - 0.3_f64.sin()).abs() <= 1.0e-9);
}
