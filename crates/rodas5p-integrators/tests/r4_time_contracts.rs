//! Time-geometry contracts of re-audit R4 of 2026-10-01 (R4-TIME-DEV-01..04).

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    BdfConfig, BdfHistory, OdeProblem, OutputSamplingPlan, OutputSchedule, bdf_step,
    integrate_bdf_fixed, integrate_bdf_fixed_dense_observed, integrate_bdf_fixed_observed,
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
