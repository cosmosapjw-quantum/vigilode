use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, TransactionalQ1Q2Config,
    integrate_transactional_q1_q2_adaptive_observed, prothero_robinson_problem,
    scalar_linear_problem,
};

fn endpoint_schedule(tf: f64) -> OutputSchedule {
    OutputSchedule::new(vec![0.0, tf]).unwrap()
}

#[test]
fn adaptive_transactional_lane_reaches_the_scalar_endpoint_without_explicit_jacobian() {
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-10,
        rtol: 1.0e-7,
        initial_step: 0.01,
        min_step: 1.0e-12,
        max_step: 0.02,
        max_attempts: 10_000,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_transactional_q1_q2_adaptive_observed(
        &problem,
        (0.0, 0.05),
        &y0,
        &TransactionalQ1Q2Config::default(),
        &adaptive,
        &endpoint_schedule(0.05),
    )
    .unwrap();
    assert!(result.observed.success);
    assert_eq!(result.observed.counters.jacobian_builds, 0);
    assert_eq!(result.observed.counters.direct_factorizations, 0);
    assert_eq!(result.observed.counters.nonlinear_iterations, 0);
    assert_eq!(
        result.transactional.accepted_steps(),
        result.observed.internal_steps
    );
    assert!(result.transactional.total_w_solve_batches > 0);
    assert_eq!(
        result.transactional.q1_path_attempts,
        result.diagnostics.attempts
    );
    assert_eq!(
        result.transactional.q2_path_attempts,
        result.transactional.selected_q2_escalated_attempts
            + result.transactional.selected_sequential_fallback_attempts
    );
    let exact = problem.exact(0.05).unwrap();
    assert!((result.observed.y.last().unwrap()[0] - exact[0]).abs() < 1.0e-6);
}

#[test]
fn adaptive_rejections_do_not_commit_outputs_or_hide_attempted_work() {
    let (problem, y0) = prothero_robinson_problem(-10_000.0, 1_000.0, 0.0);
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-7,
        initial_step: 0.02,
        min_step: 1.0e-12,
        max_step: 0.02,
        max_attempts: 20_000,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_transactional_q1_q2_adaptive_observed(
        &problem,
        (0.0, 0.02),
        &y0,
        &TransactionalQ1Q2Config::default(),
        &adaptive,
        &endpoint_schedule(0.02),
    )
    .unwrap();
    assert!(result.observed.success);
    assert_eq!(result.observed.t, vec![0.0, 0.02]);
    assert_eq!(result.observed.y.len(), 2);
    assert!(result.diagnostics.attempts >= result.observed.internal_steps);
    assert!(result.observed.counters.jvp_vectors > 0);
    assert_eq!(result.observed.counters.jacobian_builds, 0);
    assert_eq!(result.observed.counters.direct_factorizations, 0);
}
