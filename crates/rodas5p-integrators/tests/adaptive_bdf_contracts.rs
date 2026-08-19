use std::sync::Arc;

use rodas5p_core::{DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    BdfConfig, BdfHistory, BdfOrder, OdeProblem, bdf_step, bdf_step_variable,
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
    assert!(tight.diagnostics.accepted_macro_steps >= loose.diagnostics.accepted_macro_steps);
    assert_eq!(
        loose.observed.counters.accepted_steps as usize,
        2 * loose.diagnostics.accepted_macro_steps
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
    assert!(
        result
            .diagnostics
            .estimator_ids
            .iter()
            .any(|id| id == "bdf2-step-doubling")
    );
    assert_eq!(
        result.observed.counters.accepted_steps as usize,
        2 * result.diagnostics.accepted_macro_steps
    );
    assert_eq!(
        result.observed.internal_steps,
        2 * result.diagnostics.accepted_macro_steps
    );
}
