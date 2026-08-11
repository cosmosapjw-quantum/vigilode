use std::sync::Arc;

use rodas5p_core::{DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    OdeProblem, TransactionalQ1Q2Config, TransactionalQ1Q2Lane, prothero_robinson_problem,
    scalar_linear_problem, transactional_q1_q2_step,
};

#[test]
fn strict_matrix_free_contract_rejects_a_problem_without_jvp() {
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -2.0 * y[0];
        Ok(())
    });
    let jac = Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![-2.0]));
    let problem = OdeProblem::new(
        "explicit-only",
        1,
        rhs,
        None,
        Some(jac),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let mut counters = WorkCounters::default();
    let error = transactional_q1_q2_step(
        &problem,
        0.0,
        &[1.0],
        0.1,
        &TransactionalQ1Q2Config::default(),
        1.0e-9,
        1.0e-7,
        false,
        &mut counters,
    )
    .unwrap_err();
    assert!(error.to_string().contains("user-supplied JVP"));
    assert_eq!(counters.jacobian_builds, 0);
    assert_eq!(counters.direct_factorizations, 0);
}

#[test]
fn affine_scalar_path_is_accepted_without_explicit_jacobian_or_newton() {
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let mut counters = WorkCounters::default();
    let report = transactional_q1_q2_step(
        &problem,
        0.0,
        &y0,
        0.01,
        &TransactionalQ1Q2Config::default(),
        1.0e-10,
        1.0e-7,
        false,
        &mut counters,
    )
    .unwrap();
    assert_eq!(report.lane, TransactionalQ1Q2Lane::Q2Escalated);
    assert!(!report.q1_gate.accepted);
    assert!(report.q2_gate.as_ref().is_some_and(|gate| gate.accepted));
    assert!(report.fast_accepted);
    assert!(report.step.accepted);
    assert_eq!(report.step.counters.jacobian_builds, 0);
    assert_eq!(report.step.counters.direct_factorizations, 0);
    assert_eq!(report.step.counters.nonlinear_iterations, 0);
    assert_eq!(report.work.w_solve_batches, 8);
    assert_eq!(report.critical_path_depth, 8);
    assert!(!report.q1_candidate_y.is_empty());
    assert!(report.q2_candidate_y.is_some());
}

#[test]
fn q1_fast_lane_uses_six_common_w_batches() {
    let (problem, y0) = prothero_robinson_problem(-100.0, 1_000.0, 0.0);
    let mut counters = WorkCounters::default();
    let report = transactional_q1_q2_step(
        &problem,
        0.0,
        &y0,
        1.0e-4,
        &TransactionalQ1Q2Config::default(),
        1.0e-12,
        1.0e-8,
        true,
        &mut counters,
    )
    .unwrap();
    assert_eq!(report.lane, TransactionalQ1Q2Lane::Q1Fast);
    assert!(report.q1_gate.accepted);
    assert_eq!(report.work.w_solve_batches, 6);
    assert_eq!(report.critical_path_depth, 6);
    assert!(report.q2_gate.is_none());
    assert!(report.q2_candidate_y.is_none());
}

#[test]
fn q1_failure_preserves_speculative_work_before_escalation_or_fallback() {
    let (problem, y0) = prothero_robinson_problem(-10_000.0, 1_000.0, 0.0);
    let mut counters = WorkCounters::default();
    let report = transactional_q1_q2_step(
        &problem,
        0.0,
        &y0,
        0.001,
        &TransactionalQ1Q2Config::default(),
        1.0e-7,
        1.0e-6,
        true,
        &mut counters,
    )
    .unwrap();
    assert!(!report.q1_gate.accepted);
    assert!(report.escalated);
    assert!(report.work.w_solve_batches >= 8);
    assert!(report.step.counters.jvp_vectors > 0);
    assert_eq!(report.step.counters.jacobian_builds, 0);
    assert_eq!(report.step.counters.direct_factorizations, 0);
    assert_eq!(report.step.counters.nonlinear_iterations, 0);
    assert!(report.step.accepted);
    if report.lane == TransactionalQ1Q2Lane::SequentialFallback {
        assert_eq!(report.work.w_solve_batches, 8);
        assert_eq!(report.critical_path_depth, 16);
        assert!(report.q2_candidate_y.is_some());
    }
}

#[test]
fn actual_transactional_path_retains_fifth_order_on_the_mild_pr_problem() {
    let (problem, y0) = prothero_robinson_problem(-20.0, 1.0, 0.0);
    let problem = problem.jvp_only_clone().unwrap();
    let mut errors = Vec::new();
    for h in [0.04_f64, 0.02, 0.01, 0.005] {
        let mut t = 0.0;
        let mut y = y0.clone();
        let mut counters = WorkCounters::default();
        while t < 0.2 - 100.0 * f64::EPSILON {
            let step_h = h.min(0.2 - t);
            let report = transactional_q1_q2_step(
                &problem,
                t,
                &y,
                step_h,
                &TransactionalQ1Q2Config::default(),
                1.0e-12,
                1.0e-10,
                true,
                &mut counters,
            )
            .unwrap();
            t = report.step.t_new;
            y = report.step.y_new;
        }
        let exact = problem.exact(0.2).unwrap();
        errors.push((y[0] - exact[0]).abs());
    }
    let orders = errors
        .windows(2)
        .map(|pair| (pair[0] / pair[1]).log2())
        .collect::<Vec<_>>();
    assert!(orders[0] > 4.5, "errors={errors:?}, orders={orders:?}");
    assert!(
        errors[1..].iter().all(|error| *error < 1.0e-10),
        "the remaining refinements are already at the binary64/Krylov floor: {errors:?}"
    );
}
