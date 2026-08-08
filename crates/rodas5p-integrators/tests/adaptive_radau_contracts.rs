use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, RadauConfig, RadauIiaStages,
    integrate_radau_adaptive_observed, scalar_linear_problem,
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
fn adaptive_radau1_is_transactional_and_responds_to_tolerance() {
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.2, 0.02).unwrap();
    let radau = RadauConfig {
        stages: RadauIiaStages::One,
        ..Default::default()
    };
    let loose = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        &radau,
        &adaptive_config(1.0e-8, 1.0e-4, 0.15, 0.15),
        &schedule,
    )
    .unwrap();
    let tight = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        &radau,
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
fn adaptive_radau3_uses_sixth_order_step_doubling_estimator() {
    let (problem, y0) = scalar_linear_problem(-5.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.4, 0.04).unwrap();
    let radau = RadauConfig {
        stages: RadauIiaStages::Three,
        ..Default::default()
    };
    let result = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 0.4),
        &y0,
        &radau,
        &adaptive_config(1.0e-11, 1.0e-8, 0.12, 0.12),
        &schedule,
    )
    .unwrap();
    assert!(result.observed.success);
    assert_eq!(result.observed.t, schedule.times());
    assert!(
        result
            .diagnostics
            .estimator_orders
            .iter()
            .all(|order| *order == 6)
    );
    assert!(
        result
            .diagnostics
            .estimator_ids
            .iter()
            .all(|id| id == "radau-iia3-step-doubling")
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
