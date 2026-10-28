//! Time-geometry contracts of re-audit R4 of 2026-10-01 (R4-TIME-DEV-01..04).

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveObservedIntegrationResult, AdaptiveStepConfig, BDF_STARTUP_BDF1_BDF1_ESTIMATOR_ID,
    BDF_STARTUP_BDF1_BDF2_ESTIMATOR_ID, BdfConfig, BdfHistory, BdfOrder, IntegrationMethod,
    MaxStepPolicy, OdeProblem, OutputSamplingPlan, OutputSchedule, RadauConfig, RadauIiaStages,
    bdf_startup_divisor, bdf_step, integrate_adaptive_observed_with_config,
    integrate_bdf_adaptive_observed, integrate_bdf_fixed, integrate_bdf_fixed_dense_observed,
    integrate_bdf_fixed_observed, integrate_radau_adaptive_dense_observed,
    integrate_radau_adaptive_observed, radau_step, step_doubling_divisor,
    step_doubling_wrms_error_for_halves,
};

fn flow(velocity: f64) -> OdeProblem {
    OdeProblem::new(
        "r4-constant-flow",
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

/// The R4 witness: 8 ULPs below to 8 ULPs above `2^power`, nominal step
/// 0.675 of the upper ULP, velocity 1/span.
fn boundary(power: i32) -> (f64, f64, f64, f64) {
    let b = 2.0_f64.powi(power);
    let t0 = b - 8.0 * (b - b.next_down());
    let tf = b + 8.0 * (b.next_up() - b);
    let h = 0.675 * (b.next_up() - b);
    (t0, tf, h, 1.0 / (tf - t0))
}

/// `v (t - t0)`: `t - t0` is exact (Sterbenz) and the product is rounded
/// once, far inside the 1e-12 gate.
fn exact_flow(t0: f64, t: f64, velocity: f64) -> f64 {
    velocity * (t - t0)
}

#[test]
fn fixed_bdf_is_exact_on_constant_flow_at_every_unit_boundary() {
    // R4-TIME-01: at power 0 the absolute floor classified 5 unequal steps
    // as equal and returned 0.97917 for 1.
    for power in [-40, 0, 10, 40] {
        let (t0, tf, h, velocity) = boundary(power);
        let problem = flow(velocity);
        let result =
            integrate_bdf_fixed(&problem, (t0, tf), &[0.0], h, &BdfConfig::default()).unwrap();
        assert_eq!(*result.t.last().unwrap(), tf);
        for (t, y) in result.t.iter().zip(&result.y) {
            let error = (y[0] - exact_flow(t0, *t, velocity)).abs();
            assert!(error <= 1.0e-12, "power {power}: t = {t:e}: {error:e}");
        }
        // The wrappers share the kernel rule.
        let output = OutputSchedule::new(result.t.clone()).unwrap();
        let observed = integrate_bdf_fixed_observed(
            &problem,
            (t0, tf),
            &[0.0],
            h,
            &BdfConfig::default(),
            &output,
        )
        .unwrap();
        let dense = integrate_bdf_fixed_dense_observed(
            &problem,
            (t0, tf),
            &[0.0],
            h,
            &BdfConfig::default(),
            &OutputSamplingPlan::dense(output.clone()),
        )
        .unwrap();
        for wrapper in [&observed, &dense] {
            assert!(wrapper.success, "power {power}: {}", wrapper.message);
            for (t, y) in wrapper.t.iter().zip(&wrapper.y) {
                assert!(
                    (y[0] - exact_flow(t0, *t, velocity)).abs() <= 1.0e-12,
                    "power {power}: wrapper at {t:e}"
                );
            }
        }
    }
}

/// Step through `times` with the fixed kernel; per step, whether the
/// variable coefficients were used (`step_ratio` is `Some`).
fn classification(problem: &OdeProblem, times: &[f64]) -> (Vec<f64>, Vec<bool>) {
    let mut history = BdfHistory::default();
    let mut work = WorkCounters::default();
    let mut state = vec![0.0];
    let mut values = vec![0.0];
    let mut variable = Vec::new();
    for pair in times.windows(2) {
        let report = bdf_step(
            problem,
            pair[0],
            &state,
            pair[1] - pair[0],
            &BdfConfig::default(),
            &mut history,
            &mut work,
        )
        .unwrap();
        variable.push(report.step_ratio.is_some());
        state = report.y_new;
        values.push(state[0]);
    }
    (values, variable)
}

#[test]
fn power_of_two_unit_changes_preserve_solution_and_classification() {
    let (t0, tf, h, velocity) = boundary(0);
    let base =
        integrate_bdf_fixed(&flow(velocity), (t0, tf), &[0.0], h, &BdfConfig::default()).unwrap();
    let (values, variable) = classification(&flow(velocity), &base.t);
    for q in [-40, -1, 1, 10, 40] {
        let scale = 2.0_f64.powi(q);
        let times = base.t.iter().map(|t| t * scale).collect::<Vec<_>>();
        let (scaled_values, scaled_variable) = classification(&flow(velocity / scale), &times);
        assert_eq!(scaled_variable, variable, "q = {q}");
        for (a, b) in values.iter().zip(&scaled_values) {
            assert!((a - b).abs() <= 4.0 * f64::EPSILON, "q = {q}: {a} vs {b}");
        }
    }
    // The witness has unequal steps; they take the variable coefficients.
    assert!(variable.iter().skip(1).any(|v| *v));
}

#[test]
fn equal_steps_keep_the_constant_path_and_large_ratios_restart() {
    let problem = flow(1.0);
    // Exactly equal steps: no variable coefficients after the startup.
    let times = (0..=8).map(|k| k as f64 * 0.125).collect::<Vec<_>>();
    let (values, variable) = classification(&problem, &times);
    assert!(variable.iter().all(|v| !v), "{variable:?}");
    for (t, y) in times.iter().zip(&values) {
        assert!((y - t).abs() <= 1.0e-15);
    }
    // Unequal steps take the variable path; a ratio above 1 + sqrt(2)
    // restarts with BDF1.
    let mut history = BdfHistory::default();
    let mut work = WorkCounters::default();
    let config = BdfConfig::default();
    let first = bdf_step(
        &problem,
        0.0,
        &[0.0],
        0.125,
        &config,
        &mut history,
        &mut work,
    )
    .unwrap();
    let second = bdf_step(
        &problem,
        0.125,
        &first.y_new,
        0.25,
        &config,
        &mut history,
        &mut work,
    )
    .unwrap();
    assert_eq!(second.step_ratio, Some(2.0));
    assert!(!second.used_startup);
    let third = bdf_step(
        &problem,
        0.375,
        &second.y_new,
        0.75,
        &config,
        &mut history,
        &mut work,
    )
    .unwrap();
    assert!(third.used_startup, "ratio 3 > 1 + sqrt(2) restarts");
    assert!((third.y_new[0] - 1.125).abs() <= 1.0e-15);
}

/// y' = 2 (t - shift) / unit^2: the exact solution from 0 is
/// ((t - shift) / unit)^2; the RHS is state independent, so implicit Euler
/// is exact quadrature at the right endpoint.
fn quadratic(shift: f64, unit: f64) -> OdeProblem {
    OdeProblem::new(
        "r4-nonautonomous-quadratic",
        1,
        Arc::new(move |t: f64, _, out: &mut [f64]| {
            out[0] = 2.0 * (t - shift) / (unit * unit);
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| DenseMatrix::from_rows(&[&[0.0]]))),
        Some(Arc::new(|_, _, _, out: &mut [f64]| {
            out[0] = 0.0;
            Ok(())
        })),
        Some(Arc::new(move |_, _, out: &mut [f64]| {
            out[0] = 2.0 / (unit * unit);
            Ok(())
        })),
        false,
        None,
        None,
    )
    .unwrap()
}

fn radau1() -> RadauConfig {
    RadauConfig {
        stages: RadauIiaStages::One,
        ..Default::default()
    }
}

/// The R4-TIME-02 witness: 3 ULPs at 1e12, split 2 + 1 ULPs.
fn three_ulp_witness() -> (f64, f64, f64, f64, f64) {
    let t0 = 1.0e12_f64;
    let tf = t0.next_up().next_up().next_up();
    let h = tf - t0;
    let mid = t0 + 0.5 * h;
    (t0, tf, h, mid - t0, tf - mid)
}

#[test]
fn unequal_represented_halves_get_the_geometry_divisor() {
    let (t0, _, h, h1, h2) = three_ulp_witness();
    assert_eq!(h1, 2.0 * h2);
    let problem = quadratic(t0, h);
    let config = radau1();
    let mut work = WorkCounters::default();
    let coarse = radau_step(&problem, t0, &[0.0], h, &config, &mut work).unwrap();
    let first = radau_step(&problem, t0, &[0.0], h1, &config, &mut work).unwrap();
    let fine = radau_step(&problem, t0 + h1, &first.y_new, h2, &config, &mut work).unwrap();
    // Exact: coarse 2, fine 14/9, fine error 5/9, |fine - coarse| = 4/9.
    assert!((coarse.y_new[0] - 2.0).abs() <= 1.0e-12);
    assert!((fine.y_new[0] - 14.0 / 9.0).abs() <= 1.0e-12);
    let estimate = step_doubling_wrms_error_for_halves(
        &[0.0],
        &coarse.y_new,
        &fine.y_new,
        0.5,
        1.0e-12,
        1,
        h1,
        h2,
    )
    .unwrap();
    assert!((estimate.error_vector[0].abs() - 5.0 / 9.0).abs() <= 1.0e-12);
    assert!((estimate.divisor - 0.8).abs() <= 1.0e-15);
    assert_eq!(estimate.halves, Some((h1, h2)));
    // Normalized against atol 0.5: about 10/9 > 1 (the old rule gave 8/9).
    assert!(estimate.error_norm > 1.0);
}

fn assert_rejected_or_unresolved(
    label: &str,
    result: Result<AdaptiveObservedIntegrationResult, String>,
) {
    match result {
        Err(error) => assert!(error.contains("time resolution"), "{label}: {error}"),
        Ok(run) => {
            // Not accepted as one 3-ULP macro step with the 8/9 estimate.
            let first = run.diagnostics.error_norms.first().copied().unwrap();
            assert!(first > 1.0, "{label}: first norm {first}");
            if run.observed.success {
                let y = run.observed.y.last().unwrap()[0];
                assert!((y - 1.0).abs() <= 0.5 + 1.0e-12, "{label}: y(tf) = {y}");
            } else {
                assert!(
                    run.observed.message.contains("time resolution"),
                    "{label}: {}",
                    run.observed.message
                );
            }
        }
    }
}

#[test]
fn the_three_ulp_witness_is_rejected_by_both_radau1_wrappers() {
    let (t0, tf, h, _, _) = three_ulp_witness();
    let problem = quadratic(t0, h);
    let adaptive = AdaptiveStepConfig {
        atol: 0.5,
        rtol: 1.0e-12,
        initial_step: h,
        min_step: 1.0e-20,
        max_step: h,
        max_attempts: 100,
        ..Default::default()
    };
    let output = OutputSchedule::new(vec![t0, tf]).unwrap();
    assert_rejected_or_unresolved(
        "clipped",
        integrate_radau_adaptive_observed(
            &problem,
            (t0, tf),
            &[0.0],
            &radau1(),
            &adaptive,
            &output,
        )
        .map_err(|e| e.to_string()),
    );
    assert_rejected_or_unresolved(
        "dense",
        integrate_radau_adaptive_dense_observed(
            &problem,
            (t0, tf),
            &[0.0],
            &radau1(),
            &adaptive,
            &OutputSamplingPlan::dense(output),
        )
        .map_err(|e| e.to_string()),
    );
}

#[test]
fn equal_halves_reproduce_the_old_divisor_and_near_equal_ones_are_continuous() {
    for p in 1..=5 {
        let old = 2.0_f64.powi(p as i32) - 1.0;
        assert_eq!(step_doubling_divisor(0.375, 0.375, p).unwrap(), old);
        let near = step_doubling_divisor(0.375, 0.375 * (1.0 + 1.0e-9), p).unwrap();
        assert!((near - old).abs() <= 1.0e-8 * old, "p = {p}: {near}");
    }
    // eta / (1 - eta) for theta = 2/3, p = 1: divisor 4/5.
    assert!((step_doubling_divisor(2.0, 1.0, 1).unwrap() - 0.8).abs() <= 1.0e-15);
    for (h1, h2) in [
        (0.0, 1.0),
        (-1.0, 1.0),
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
    ] {
        assert!(step_doubling_divisor(h1, h2, 1).is_err());
    }
    assert!(step_doubling_divisor(1.0, 1.0, 0).is_err());
}

#[test]
fn a_rejected_step_is_never_retried_at_the_same_represented_size() {
    let (t0, tf, h, _, _) = three_ulp_witness();
    let problem = quadratic(t0, h);
    let adaptive = AdaptiveStepConfig {
        atol: 0.5,
        rtol: 1.0e-12,
        initial_step: h,
        min_step: 1.0e-20,
        max_step: h,
        max_attempts: 100,
        ..Default::default()
    };
    let output = OutputSchedule::new(vec![t0, tf]).unwrap();
    // 3 ULPs are rejected (10/9), the retry is 2 ULPs, not 3 again; the
    // last ULP holds no midpoint, so the run ends on a typed time-resolution
    // error after a handful of attempts instead of 100 identical retries.
    let error = integrate_radau_adaptive_observed(
        &problem,
        (t0, tf),
        &[0.0],
        &radau1(),
        &adaptive,
        &output,
    )
    .unwrap_err();
    assert!(error.to_string().contains("time resolution"), "{error}");
}

fn capped(max_step: f64, policy: MaxStepPolicy) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-10,
        rtol: 1.0e-9,
        initial_step: max_step,
        min_step: 1.0e-20,
        max_step,
        max_attempts: 100,
        max_step_policy: policy,
        ..Default::default()
    }
}

#[test]
fn the_cap_grid_obeys_each_named_policy() {
    // R4-TIME-DEV-04: 8 ULPs at 1e12 with caps of 0.49 .. 2 ULPs. The
    // endpoint and the cap are separate assertions: a correct endpoint
    // does not satisfy a cap refusal.
    let t0 = 1.0e12_f64;
    let tf = (0..8).fold(t0, |t, _| t.next_up());
    let ulp = t0.next_up() - t0;
    let velocity = 1.0 / (tf - t0);
    let problem = flow(velocity);
    let output = OutputSchedule::new(vec![t0, tf]).unwrap();
    for multiplier in [0.49, 0.75, 1.0, 1.5, 2.0] {
        let cap = multiplier * ulp;
        for policy in [
            MaxStepPolicy::StrictRepresentedCap,
            MaxStepPolicy::AllowClockResolutionSlack,
        ] {
            let label = format!("{multiplier} ULP, {policy:?}");
            let run = integrate_adaptive_observed_with_config(
                &problem,
                (t0, tf),
                &[0.0],
                IntegrationMethod::Sequential,
                None,
                None,
                &capped(cap, policy),
                &output,
            )
            .map_err(|e| e.to_string());
            match (policy, run) {
                (MaxStepPolicy::StrictRepresentedCap, Err(error)) => {
                    assert!(multiplier < 1.0, "{label}: {error}");
                    assert!(error.contains("time resolution"), "{label}: {error}");
                }
                (MaxStepPolicy::StrictRepresentedCap, Ok(run)) => {
                    assert!(multiplier >= 1.0, "{label}: a sub-ULP cap succeeded");
                    for h in &run.diagnostics.accepted_step_sizes {
                        assert!(*h <= cap, "{label}: h/max_step = {}", h / cap);
                    }
                    assert!(run.observed.success, "{label}");
                    let y = run.observed.y.last().unwrap()[0];
                    assert!((y - 1.0).abs() <= 1.0e-12, "{label}: y(tf) = {y}");
                }
                (MaxStepPolicy::AllowClockResolutionSlack, run) => {
                    let run = run.unwrap_or_else(|e| panic!("{label}: {e}"));
                    if multiplier < 0.5 {
                        // t + max_step rounds back to t: no step is taken
                        // and the run reports failure (not a full-ULP step).
                        assert!(!run.observed.success, "{label}");
                        assert!(run.diagnostics.accepted_step_sizes.is_empty(), "{label}");
                        continue;
                    }
                    let mut t = t0;
                    for h in &run.diagnostics.accepted_step_sizes {
                        let end = t + h;
                        // The documented allowance: one resolution at the end.
                        assert!(
                            *h <= cap + (end.next_up() - end),
                            "{label}: h/max_step = {}",
                            h / cap
                        );
                        t = end;
                    }
                    assert!(run.observed.success, "{label}");
                    let y = run.observed.y.last().unwrap()[0];
                    assert!((y - 1.0).abs() <= 1.0e-12, "{label}: y(tf) = {y}");
                }
            }
        }
    }
}

#[test]
fn decimal_grid_controls_stay_accurate_under_both_policies() {
    // Strict: 0.07 + 0.01 represents 0.010000000000000009 > 0.01, so the
    // step ends one representable time early and the output is reached by
    // a roundoff-sized landing step; slack takes the represented step.
    let problem = flow(1.0);
    let output = OutputSchedule::new((0..=10).map(|k| k as f64 * 0.1).collect()).unwrap();
    for policy in [
        MaxStepPolicy::StrictRepresentedCap,
        MaxStepPolicy::AllowClockResolutionSlack,
    ] {
        let run = integrate_adaptive_observed_with_config(
            &problem,
            (0.0, 1.0),
            &[0.0],
            IntegrationMethod::Sequential,
            None,
            None,
            &AdaptiveStepConfig {
                max_attempts: 1000,
                ..capped(0.01, policy)
            },
            &output,
        )
        .unwrap();
        assert!(run.observed.success, "{policy:?}: {}", run.observed.message);
        for (t, y) in run.observed.t.iter().zip(&run.observed.y) {
            assert!((y[0] - t).abs() <= 1.0e-12, "{policy:?}: t = {t}");
        }
        if policy == MaxStepPolicy::StrictRepresentedCap {
            assert!(
                run.diagnostics
                    .accepted_step_sizes
                    .iter()
                    .all(|h| *h <= 0.01)
            );
        }
    }
}

/// y' = c (t - shift)^(d-1) d / unit^d, so y = ((t - shift) / unit)^d from 0.
fn primitive(degree: i32, shift: f64, unit: f64) -> OdeProblem {
    OdeProblem::new(
        "r4-polynomial-primitive",
        1,
        Arc::new(move |t: f64, _, out: &mut [f64]| {
            out[0] = f64::from(degree) * ((t - shift) / unit).powi(degree - 1) / unit;
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| DenseMatrix::from_rows(&[&[0.0]]))),
        Some(Arc::new(|_, _, _, out: &mut [f64]| {
            out[0] = 0.0;
            Ok(())
        })),
        Some(Arc::new(move |t: f64, _, out: &mut [f64]| {
            out[0] = f64::from(degree * (degree - 1))
                * ((t - shift) / unit).powi((degree - 2).max(0))
                / (unit * unit);
            Ok(())
        })),
        false,
        None,
        None,
    )
    .unwrap()
}

/// The first adaptive BDF attempt over one step `[t0, t0 + span]` with
/// `atol = 1`, `rtol = 0`: its error norm is the estimate itself.
fn first_startup(
    problem: &OdeProblem,
    t0: f64,
    span: f64,
    order: BdfOrder,
    atol: f64,
) -> AdaptiveObservedIntegrationResult {
    let adaptive = AdaptiveStepConfig {
        atol,
        rtol: 0.0,
        initial_step: span,
        min_step: 1.0e-30,
        max_step: span,
        max_attempts: 3,
        ..Default::default()
    };
    let tf = t0 + span;
    integrate_bdf_adaptive_observed(
        problem,
        (t0, tf),
        &[0.0],
        &BdfConfig {
            order,
            ..BdfConfig::default()
        },
        &adaptive,
        &OutputSchedule::new(vec![t0, tf]).unwrap(),
    )
    .unwrap()
}

#[test]
fn startup_divisors_follow_the_derivation() {
    // BDF1 + BDF2: D = 2 h2 / h1; BDF1 + BDF1: the Radau1 geometry rule.
    assert_eq!(bdf_startup_divisor(0.25, 0.25, BdfOrder::Two).unwrap(), 2.0);
    assert_eq!(bdf_startup_divisor(2.0, 1.0, BdfOrder::Two).unwrap(), 1.0);
    assert_eq!(bdf_startup_divisor(0.25, 0.25, BdfOrder::One).unwrap(), 1.0);
    assert!((bdf_startup_divisor(2.0, 1.0, BdfOrder::One).unwrap() - 0.8).abs() <= 1.0e-15);
    for (h1, h2) in [(0.0, 1.0), (1.0, f64::NAN), (-1.0, 1.0)] {
        assert!(bdf_startup_divisor(h1, h2, BdfOrder::Two).is_err());
    }
}

#[test]
fn quadratic_primitives_make_the_startup_estimate_exact() {
    // Degree 2: y'' is constant and the RHS ignores the state, so the
    // derived relation is exact (both orders, equal and 2 + 1 ULP halves).
    let cases: [(f64, f64, f64); 2] = [(0.0, 0.5, 0.5), (1.0e12, 0.0, 0.0)];
    for (t0, span, unit) in cases {
        let (span, unit) = if t0 == 0.0 {
            (span, unit)
        } else {
            let tf = t0.next_up().next_up().next_up();
            (tf - t0, tf - t0)
        };
        let problem = primitive(2, t0, unit);
        for order in [BdfOrder::One, BdfOrder::Two] {
            let run = first_startup(&problem, t0, span, order, 1.0e6);
            let d = &run.diagnostics;
            let expected_id = match order {
                BdfOrder::One => BDF_STARTUP_BDF1_BDF1_ESTIMATOR_ID,
                BdfOrder::Two => BDF_STARTUP_BDF1_BDF2_ESTIMATOR_ID,
            };
            assert_eq!(d.estimator_ids[0], expected_id, "{t0:e} {order:?}");
            assert!(run.observed.success);
            let y = run.observed.y.last().unwrap()[0];
            let actual = (y - (span / unit).powi(2)).abs();
            let estimate = d.error_norms[0] * 1.0e6;
            assert!(
                (estimate - actual).abs() <= 1.0e-12 * actual.max(1.0e-300) + 1.0e-15,
                "{t0:e} {order:?}: estimate {estimate:e}, actual {actual:e}"
            );
        }
    }
    // Degree 1: every startup sequence is exact and so is the estimate.
    let linear = first_startup(&primitive(1, 0.0, 1.0), 0.0, 0.5, BdfOrder::Two, 1.0);
    assert!(linear.diagnostics.error_norms[0] <= 1.0e-15);
}

#[test]
fn cubic_primitives_approach_the_derived_ratio_asymptotically() {
    // y = (t + 1)^3 - 1 from 0: y''(0) = 6 is the leading term the
    // derivation needs (at y''(0) = 0 the cubic term leads instead), and the
    // relative deviation from the derived ratio falls with the step.
    let mut previous = f64::INFINITY;
    for span in [1.0e-1, 1.0e-2, 1.0e-3] {
        let problem = primitive(3, -1.0, 1.0);
        let run = first_startup(&problem, 0.0, span, BdfOrder::Two, 1.0e6);
        let y = run.observed.y.last().unwrap()[0];
        let actual = (y - ((1.0 + span).powi(3) - 1.0)).abs();
        let ratio = run.diagnostics.error_norms[0] * 1.0e6 / actual;
        let distance = (ratio - 1.0).abs();
        assert!(distance < previous, "span {span}: ratio {ratio}");
        previous = distance;
    }
    assert!(previous < 0.01, "{previous}");
}

#[test]
fn rejected_startup_trials_do_not_advance_history() {
    // A tolerance no startup can meet: every attempt is a rejected
    // startup trial from the initial state.
    let problem = primitive(2, 0.0, 1.0);
    let run = first_startup(&problem, 0.0, 0.5, BdfOrder::Two, 1.0e-30);
    let d = &run.diagnostics;
    assert!(!run.observed.success);
    assert!(d.accepted_step_sizes.is_empty());
    assert!(d.attempts >= 2);
    assert!(
        d.estimator_ids
            .iter()
            .all(|id| id == BDF_STARTUP_BDF1_BDF2_ESTIMATOR_ID),
        "{:?}",
        d.estimator_ids
    );
}
