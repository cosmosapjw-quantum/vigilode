use std::sync::Arc;

use rodas5p_core::{DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    BDF2_ZERO_STABILITY_RATIO_MAX, BdfConfig, BdfHistory, BdfOrder, OdeProblem, bdf_step,
    bdf_step_variable, bdf1_predictor_correction_lte_factor, bdf2_predictor_correction_lte_factor,
    variable_bdf2_coefficients, variable_bdf2_predictor,
};

fn quadratic_time_problem() -> OdeProblem {
    let rhs = Arc::new(|t: f64, _y: &[f64], out: &mut [f64]| {
        out[0] = 2.0 * t;
        Ok(())
    });
    let jac = Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![0.0]));
    let exact = Arc::new(|t: f64| vec![t * t]);
    OdeProblem::new(
        "quadratic-time",
        1,
        rhs,
        None,
        Some(jac),
        None,
        None,
        false,
        None,
        Some(exact),
    )
    .unwrap()
}

fn cubic_time_problem() -> OdeProblem {
    let rhs = Arc::new(|t: f64, _y: &[f64], out: &mut [f64]| {
        out[0] = 3.0 * t * t;
        Ok(())
    });
    let jac = Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![0.0]));
    let exact = Arc::new(|t: f64| vec![t * t * t]);
    OdeProblem::new(
        "cubic-time",
        1,
        rhs,
        None,
        Some(jac),
        None,
        None,
        false,
        None,
        Some(exact),
    )
    .unwrap()
}

#[test]
fn nonuniform_bdf2_coefficients_recover_constant_step_formula() {
    let coefficients = variable_bdf2_coefficients(0.1, 0.1).unwrap();
    assert!((coefficients.step_ratio - 1.0).abs() < 1.0e-15);
    assert!((2.0 * coefficients.a0 - 3.0).abs() < 1.0e-15);
    assert!((2.0 * coefficients.a1 + 4.0).abs() < 1.0e-15);
    assert!((2.0 * coefficients.a2 - 1.0).abs() < 1.0e-15);
}

#[test]
fn nonuniform_bdf2_predictor_uses_the_actual_step_ratio() {
    let predicted = variable_bdf2_predictor(&[1.0, 2.0], &[0.5, 1.5], 0.25).unwrap();
    assert_eq!(predicted, vec![1.125, 2.125]);
}

#[test]
fn variable_bdf2_accepts_the_closed_zero_stability_boundary() {
    let problem = quadratic_time_problem();
    let mut history = BdfHistory::with_previous(vec![0.0], 1.0).unwrap();
    let mut counters = WorkCounters::default();
    let report = bdf_step_variable(
        &problem,
        1.0,
        &[1.0],
        BDF2_ZERO_STABILITY_RATIO_MAX,
        &BdfConfig {
            order: BdfOrder::Two,
            ..Default::default()
        },
        &mut history,
        &mut counters,
    )
    .unwrap();
    assert_eq!(report.applied_order, BdfOrder::Two);
    assert!(!report.used_startup);
    assert_eq!(report.step_ratio, Some(BDF2_ZERO_STABILITY_RATIO_MAX));
}

#[test]
fn variable_bdf2_is_exact_for_a_quadratic_on_unequal_steps() {
    let problem = quadratic_time_problem();
    let mut history = BdfHistory::with_previous(vec![0.64], 0.2).unwrap();
    let config = BdfConfig {
        order: BdfOrder::Two,
        ..Default::default()
    };
    let mut counters = WorkCounters::default();
    let report = bdf_step_variable(
        &problem,
        1.0,
        &[1.0],
        0.1,
        &config,
        &mut history,
        &mut counters,
    )
    .unwrap();
    assert_eq!(report.applied_order, BdfOrder::Two);
    assert!(!report.used_startup);
    assert!(
        (report.y_new[0] - 1.21).abs() < 5.0e-14,
        "{:?}",
        report.y_new
    );
    assert_eq!(history.previous_step(), Some(0.1));
    assert_eq!(history.previous_state(), Some(&[1.0][..]));
}

#[test]
fn fixed_bdf_step_remains_identical_to_variable_kernel_at_ratio_one() {
    let problem = quadratic_time_problem();
    let config = BdfConfig {
        order: BdfOrder::Two,
        ..Default::default()
    };
    let mut fixed_history = BdfHistory::with_previous(vec![0.81], 0.1).unwrap();
    let mut variable_history = fixed_history.clone();
    let mut fixed_counters = WorkCounters::default();
    let mut variable_counters = WorkCounters::default();
    let fixed = bdf_step(
        &problem,
        1.0,
        &[1.0],
        0.1,
        &config,
        &mut fixed_history,
        &mut fixed_counters,
    )
    .unwrap();
    let variable = bdf_step_variable(
        &problem,
        1.0,
        &[1.0],
        0.1,
        &config,
        &mut variable_history,
        &mut variable_counters,
    )
    .unwrap();
    assert_eq!(fixed.y_new, variable.y_new);
    assert_eq!(fixed.applied_order, variable.applied_order);
    assert_eq!(fixed_history, variable_history);
}

#[test]
fn variable_bdf1_quadratic_oracle_validates_geometry_lte_factor_and_second_order_refinement() {
    let problem = quadratic_time_problem();
    let t = 1.0_f64;
    let mut errors: Vec<f64> = Vec::new();
    for h in [0.08_f64, 0.04, 0.02, 0.01] {
        let previous_h = 1.25 * h;
        let mut history =
            BdfHistory::with_previous(vec![(t - previous_h).powi(2)], previous_h).unwrap();
        let mut counters = WorkCounters::default();
        let report = bdf_step_variable(
            &problem,
            t,
            &[t.powi(2)],
            h,
            &BdfConfig {
                order: BdfOrder::One,
                ..Default::default()
            },
            &mut history,
            &mut counters,
        )
        .unwrap();
        let factor = bdf1_predictor_correction_lte_factor(h, previous_h).unwrap();
        let estimate = factor * (report.y_new[0] - report.predictor[0]);
        let true_local_error = (t + h).powi(2) - report.y_new[0];
        assert!(
            (estimate.abs() - true_local_error.abs()).abs()
                <= 2.0e-11 * true_local_error.abs().max(f64::EPSILON),
            "h={h:e}, estimate={estimate:e}, true={true_local_error:e}, factor={factor:e}"
        );
        errors.push(estimate.abs());
    }
    for pair in errors.windows(2) {
        let order = (pair[0] / pair[1]).log2();
        assert!(order > 1.99, "errors={errors:?}, observed order={order}");
    }
}

#[test]
fn variable_bdf2_excessive_growth_takes_one_order_one_restart_and_rebuilds_history() {
    let (problem, _y0) = scalar_linear_problem(-2.0, 1.0);
    let previous_h = 0.1;
    let h = (BDF2_ZERO_STABILITY_RATIO_MAX + 0.25) * previous_h;
    let mut history = BdfHistory::with_two_previous(
        vec![(0.2_f64).exp()],
        previous_h,
        vec![(0.4_f64).exp()],
        previous_h,
    )
    .unwrap();
    let config = BdfConfig {
        order: BdfOrder::Two,
        ..Default::default()
    };
    let mut counters = WorkCounters::default();

    let restart = bdf_step_variable(
        &problem,
        0.0,
        &[1.0],
        h,
        &config,
        &mut history,
        &mut counters,
    )
    .unwrap();
    assert_eq!(restart.applied_order, BdfOrder::One);
    assert!(restart.used_startup);
    assert!(restart.step_ratio.is_none());
    assert!((restart.y_new[0] - 1.0 / (1.0 + 2.0 * h)).abs() < 2.0e-14);
    assert_eq!(history.previous_state(), Some(&[1.0][..]));
    assert_eq!(history.previous_step(), Some(h));
    assert_eq!(history.older_state(), None);
    assert_eq!(history.older_step(), None);

    let resumed = bdf_step_variable(
        &problem,
        h,
        &restart.y_new,
        h,
        &config,
        &mut history,
        &mut counters,
    )
    .unwrap();
    assert_eq!(resumed.applied_order, BdfOrder::Two);
    assert!(!resumed.used_startup);
    assert_eq!(resumed.step_ratio, Some(1.0));
}

#[test]
fn variable_bdf2_cubic_oracle_validates_geometry_lte_factor_and_third_order_refinement() {
    let problem = cubic_time_problem();
    let t = 1.0_f64;
    let mut estimated_errors: Vec<f64> = Vec::new();
    let mut true_local_errors: Vec<f64> = Vec::new();
    for h in [0.08_f64, 0.04, 0.02, 0.01] {
        let previous_h = 1.25 * h;
        let older_h = 0.75 * h;
        let mut history = BdfHistory::with_two_previous(
            vec![(t - previous_h).powi(3)],
            previous_h,
            vec![(t - previous_h - older_h).powi(3)],
            older_h,
        )
        .unwrap();
        let mut counters = WorkCounters::default();
        let report = bdf_step_variable(
            &problem,
            t,
            &[t.powi(3)],
            h,
            &BdfConfig {
                order: BdfOrder::Two,
                ..Default::default()
            },
            &mut history,
            &mut counters,
        )
        .unwrap();
        assert_eq!(report.applied_order, BdfOrder::Two);

        let factor = bdf2_predictor_correction_lte_factor(h, previous_h, older_h).unwrap();
        let estimate = factor * (report.y_new[0] - report.predictor[0]);
        let true_local_error = (t + h).powi(3) - report.y_new[0];
        assert!(
            (estimate.abs() - true_local_error.abs()).abs()
                <= 2.0e-11 * true_local_error.abs().max(f64::EPSILON),
            "h={h:e}, estimate={estimate:e}, true={true_local_error:e}, factor={factor:e}"
        );
        estimated_errors.push(estimate.abs());
        true_local_errors.push(true_local_error.abs());
    }

    for errors in [&estimated_errors, &true_local_errors] {
        for pair in errors.windows(2) {
            let order = (pair[0] / pair[1]).log2();
            assert!(order > 2.99, "errors={errors:?}, observed order={order}");
        }
    }
}

use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, integrate_bdf_adaptive_observed, scalar_linear_problem,
};

fn adaptive_config(atol: f64, rtol: f64, initial_step: f64, max_step: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol,
        rtol,
        initial_step,
        min_step: 1.0e-12,
        max_step,
        max_attempts: 20_000,
        ..AdaptiveStepConfig::default()
    }
}

#[test]
fn adaptive_bdf1_is_transactional_and_responds_to_tolerance() {
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.2, 0.02).unwrap();
    let bdf = BdfConfig {
        order: BdfOrder::One,
        ..Default::default()
    };
    let loose = integrate_bdf_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        &bdf,
        &adaptive_config(1.0e-8, 1.0e-4, 0.15, 0.15),
        &schedule,
    )
    .unwrap();
    let tight = integrate_bdf_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        &bdf,
        &adaptive_config(1.0e-11, 1.0e-7, 0.15, 0.15),
        &schedule,
    )
    .unwrap();
    assert!(loose.observed.success && tight.observed.success);
    assert_eq!(loose.observed.t, schedule.times());
    assert_eq!(tight.observed.t, schedule.times());
    assert!(loose.diagnostics.rejected_macro_steps > 0);
    assert_eq!(
        loose.observed.counters.local_error_failures as usize,
        loose.diagnostics.local_error_failures
    );
    assert_eq!(
        loose.observed.counters.linear_solve_failures as usize,
        loose.diagnostics.linear_solve_failures
    );
    assert_eq!(
        loose.observed.counters.nonlinear_solve_failures as usize,
        loose.diagnostics.nonlinear_solve_failures
    );
    assert_eq!(
        loose.observed.counters.nonfinite_step_failures as usize,
        loose.diagnostics.non_finite_failures
    );
    assert!(tight.diagnostics.accepted_macro_steps >= loose.diagnostics.accepted_macro_steps);
    assert_eq!(
        loose.observed.counters.accepted_steps as usize,
        loose.diagnostics.accepted_macro_steps + 1
    );
    assert!(
        loose
            .diagnostics
            .estimator_ids
            .iter()
            .any(|id| id == "bdf1-pure-bdf-backward-difference-lte")
    );
    assert!(
        loose
            .diagnostics
            .estimator_orders
            .iter()
            .all(|order| *order == 2)
    );
    let exact = problem.exact(0.2).unwrap()[0];
    let loose_error = (loose.observed.y.last().unwrap()[0] - exact).abs();
    let tight_error = (tight.observed.y.last().unwrap()[0] - exact).abs();
    assert!(
        tight_error < loose_error,
        "loose={loose_error:e}, tight={tight_error:e}"
    );
}

#[test]
fn adaptive_bdf2_uses_startup_order_then_variable_step_order() {
    let (problem, y0) = scalar_linear_problem(-5.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.4, 0.04).unwrap();
    let bdf = BdfConfig {
        order: BdfOrder::Two,
        ..Default::default()
    };
    let result = integrate_bdf_adaptive_observed(
        &problem,
        (0.0, 0.4),
        &y0,
        &bdf,
        &adaptive_config(1.0e-10, 1.0e-7, 0.12, 0.12),
        &schedule,
    )
    .unwrap();
    assert!(result.observed.success);
    assert_eq!(result.diagnostics.estimator_orders.first(), Some(&2));
    assert!(result.diagnostics.estimator_orders.contains(&3));
    let first_steady = result
        .diagnostics
        .estimator_ids
        .iter()
        .position(|id| id == "bdf2-pure-bdf-backward-difference-lte")
        .expect("BDF2 must leave explicit startup");
    assert!(first_steady > 0);
    assert!(
        result.diagnostics.estimator_ids[..first_steady]
            .iter()
            .all(|id| id == "bdf-explicit-startup-step-doubling")
    );
    assert!(
        result.diagnostics.estimator_ids[first_steady..]
            .iter()
            .all(|id| matches!(
                id.as_str(),
                "bdf2-pure-bdf-backward-difference-lte"
                    | "bdf1-pure-bdf-backward-difference-lte"
                    | "bdf-explicit-startup-step-doubling"
            )),
        "post-startup trials may include typed order-one ratio restarts before rebuilding BDF2 history: {:?}",
        result.diagnostics.estimator_ids
    );
    assert!(
        result
            .diagnostics
            .estimator_ids
            .iter()
            .any(|id| id == "bdf2-pure-bdf-backward-difference-lte")
    );
    assert_eq!(
        result.observed.counters.accepted_steps as usize,
        result.diagnostics.accepted_macro_steps + 1
    );
    assert_eq!(
        result.observed.internal_steps,
        result.diagnostics.accepted_macro_steps + 1
    );
    assert!(
        result.observed.counters.nonlinear_solves < 3 * result.diagnostics.attempts as u64,
        "steady-state BDF must not retain three-solve step doubling"
    );
}

#[test]
fn accepted_output_clipping_restores_the_preclip_bdf_request() {
    let (problem, y0) = scalar_linear_problem(0.0, 1.0);
    let schedule = OutputSchedule::new(vec![0.0, 0.03, 1.0]).unwrap();
    let result = integrate_bdf_adaptive_observed(
        &problem,
        (0.0, 1.0),
        &y0,
        &BdfConfig {
            order: BdfOrder::Two,
            ..BdfConfig::default()
        },
        &adaptive_config(1.0, 0.0, 0.1, 1.0),
        &schedule,
    )
    .unwrap();
    assert!(result.observed.success);
    assert_eq!(
        result.diagnostics.accepted_step_sizes[0].to_bits(),
        0.03_f64.to_bits()
    );
    assert_eq!(
        result.diagnostics.accepted_step_sizes[1].to_bits(),
        0.1_f64.to_bits()
    );
    assert_eq!(
        result.diagnostics.estimator_ids[0],
        "bdf-explicit-startup-step-doubling"
    );
    assert_eq!(
        result.diagnostics.estimator_ids[1], "bdf1-pure-bdf-backward-difference-lte",
        "restoring the pre-clipped request must restart at order one when the BDF2 ratio cap is exceeded"
    );
}

#[test]
fn bdf_linear_and_nonlinear_failures_have_one_to_one_typed_counters() {
    let make_problem = |name: &str, rhs_value: f64, jacobian_value: f64, linear_rhs: bool| {
        let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
            out[0] = if linear_rhs {
                rhs_value * y[0]
            } else {
                rhs_value
            };
            Ok(())
        });
        let jacobian =
            Arc::new(move |_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![jacobian_value]));
        OdeProblem::new(
            name,
            1,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            None,
            None,
        )
        .unwrap()
    };
    let adaptive = AdaptiveStepConfig {
        initial_step: 0.1,
        max_step: 0.1,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    let schedule = OutputSchedule::new(vec![0.0, 0.1]).unwrap();

    let singular = make_problem("singular-bdf", 10.0, 10.0, true);
    let linear = integrate_bdf_adaptive_observed(
        &singular,
        (0.0, 0.1),
        &[1.0],
        &BdfConfig::default(),
        &adaptive,
        &schedule,
    )
    .unwrap();
    assert_eq!(linear.diagnostics.linear_solve_failures, 1);
    assert_eq!(linear.observed.counters.linear_solve_failures, 1);
    assert_eq!(linear.diagnostics.failure_kinds.len(), 1);

    let stale = make_problem("stale-bdf", 1.0, 20.0, false);
    let nonlinear = integrate_bdf_adaptive_observed(
        &stale,
        (0.0, 0.1),
        &[0.0],
        &BdfConfig::default(),
        &adaptive,
        &schedule,
    )
    .unwrap();
    assert_eq!(nonlinear.diagnostics.nonlinear_solve_failures, 1);
    assert_eq!(nonlinear.observed.counters.nonlinear_solve_failures, 1);
    assert_eq!(nonlinear.diagnostics.failure_kinds.len(), 1);
}
