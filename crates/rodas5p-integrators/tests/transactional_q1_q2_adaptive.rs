use rodas5p_core::{LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, KrylovState, OutputSchedule, TransactionalQ1Q2Config,
    constant_affine_mass_problem, integrate_transactional_q1_q2_adaptive_observed,
    prothero_robinson_problem, scalar_linear_problem, sequential_matrix_free_step,
    transactional_q1_q2_step,
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

#[test]
fn matrix_free_shifted_applications_are_folded_into_physical_jvp_and_mass_work() {
    let (problem, mut y, _, _) = constant_affine_mass_problem();
    let config = LinearSolverConfig {
        method: LinearMethod::Gcrodr,
        preconditioner: PreconditionerKind::None,
        restart: 2,
        maxiter: 8,
        recycle_dim: 1,
        ..LinearSolverConfig::default()
    };
    let mut counters = WorkCounters::default();
    let mut recycle = KrylovState::for_method(config.method);
    for time in [0.0, 0.01] {
        let before = counters;
        let step = sequential_matrix_free_step(
            &problem,
            time,
            &y,
            0.01,
            &config,
            recycle.as_mut(),
            1.0e-10,
            1.0e-8,
            true,
            &mut counters,
        )
        .unwrap();
        let delta = counters.delta(before);
        let shifted = delta.shifted_operator_applications_since(WorkCounters::default());
        assert!(delta.jvp_vectors >= shifted);
        assert_eq!(delta.jvp_calls, delta.jvp_vectors);
        assert_eq!(delta.mass_matvecs, shifted);
        y = step.y_new;
    }
    assert!(counters.recycle_refresh_matvecs > 0);
}

#[test]
fn matrix_free_solver_failure_keeps_successful_shifted_applications_accounted() {
    let (problem, y0, _, _) = constant_affine_mass_problem();
    let config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        restart: 1,
        maxiter: 1,
        ..LinearSolverConfig::default()
    };
    let mut counters = WorkCounters::default();
    let error = sequential_matrix_free_step(
        &problem,
        0.0,
        &y0,
        0.1,
        &config,
        None,
        1.0e-10,
        1.0e-8,
        false,
        &mut counters,
    )
    .unwrap_err();
    assert!(error.to_string().contains("linear solve failed"));
    let shifted = counters.shifted_operator_applications_since(WorkCounters::default());
    assert!(shifted > 0);
    assert_eq!(counters.jvp_vectors, shifted);
    assert_eq!(counters.jvp_calls, shifted);
    assert_eq!(counters.mass_matvecs, shifted);
}

#[test]
fn transactional_solver_failure_keeps_successful_shifted_applications_accounted() {
    let (problem, y0, _, _) = constant_affine_mass_problem();
    let mut counters = WorkCounters::default();
    let error = transactional_q1_q2_step(
        &problem,
        0.0,
        &y0,
        0.1,
        &TransactionalQ1Q2Config {
            gmres_restart: 1,
            gmres_max_arnoldi: 1,
            ..TransactionalQ1Q2Config::default()
        },
        1.0e-10,
        1.0e-8,
        false,
        &mut counters,
    )
    .unwrap_err();
    assert!(error.to_string().contains("linear solve failed"));
    let shifted = counters.shifted_operator_applications_since(WorkCounters::default());
    assert!(shifted > 0);
    assert!(counters.jvp_vectors >= shifted);
    assert_eq!(counters.jvp_calls, counters.jvp_vectors);
    assert!(counters.mass_matvecs >= shifted);
}
