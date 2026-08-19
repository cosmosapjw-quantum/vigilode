use std::sync::Arc;

use rodas5p_core::{
    DenseMatrix, DenseOperator, IdentityPreconditioner, JacobiPreconditioner, LinearOperator,
    WorkCounters,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GmresConfig, LgmresConfig, LgmresState, solve_gcrodr, solve_gmres,
    solve_lgmres,
};

fn test_system() -> (Arc<dyn LinearOperator>, Vec<f64>, Vec<f64>) {
    let a =
        DenseMatrix::from_rows(&[&[4.0, 1.0, 0.0], &[2.0, 3.0, 1.0], &[0.0, -1.0, 2.0]]).unwrap();
    let x = vec![1.0, -2.0, 0.5];
    let b = a.matvec(&x).unwrap();
    (Arc::new(DenseOperator::new(a).unwrap()), b, x)
}

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

#[test]
fn restarted_gmres_matches_direct_solution_and_certifies_true_residual() {
    let (op, b, x_true) = test_system();
    let pc = IdentityPreconditioner::new(3);
    let mut counters = WorkCounters::default();
    let r = solve_gmres(
        op.as_ref(),
        &pc,
        &b,
        None,
        &GmresConfig {
            restart: 3,
            max_arnoldi: 20,
            rtol: 1e-12,
            atol: 1e-14,
        },
        &mut counters,
    )
    .unwrap();
    assert!(r.converged);
    assert!(r.residual_norm <= 1e-11);
    assert!(max_abs_diff(&r.x, &x_true) < 1e-10);
    assert!(counters.diagnostic_matvecs >= 1);
}

#[test]
fn gmres_uses_the_same_jacobi_preconditioner_contract() {
    let a =
        DenseMatrix::from_rows(&[&[10.0, 2.0, 0.0], &[1.0, 7.0, 1.0], &[0.0, 1.0, 5.0]]).unwrap();
    let x_true = vec![1.0, 2.0, -1.0];
    let b = a.matvec(&x_true).unwrap();
    let op = DenseOperator::new(a.clone()).unwrap();
    let pc = JacobiPreconditioner::from_matrix(&a).unwrap();
    let mut counters = WorkCounters::default();
    let r = solve_gmres(
        &op,
        &pc,
        &b,
        None,
        &GmresConfig {
            restart: 3,
            max_arnoldi: 20,
            rtol: 1e-12,
            atol: 1e-14,
        },
        &mut counters,
    )
    .unwrap();
    assert!(max_abs_diff(&r.x, &x_true) < 1e-10);
    assert!(counters.preconditioner_apps > 0);
}

#[test]
fn lgmres_commits_state_only_after_a_certified_solve() {
    let (op, b, x_true) = test_system();
    let pc = IdentityPreconditioner::new(3);
    let mut state = LgmresState::default();
    let mut counters = WorkCounters::default();
    let config = LgmresConfig {
        inner_m: 2,
        max_outer: 10,
        outer_k: 2,
        rtol: 1e-12,
        atol: 1e-14,
    };
    let r = solve_lgmres(
        op.as_ref(),
        &pc,
        &b,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert!(r.converged);
    assert!(max_abs_diff(&r.x, &x_true) < 1e-10);
    assert_eq!(state.operator_token, Some(op.token()));
    assert!(!state.directions.is_empty());
    let snapshot = state.clone();
    let bad = vec![f64::NAN, 0.0, 0.0];
    assert!(
        solve_lgmres(
            op.as_ref(),
            &pc,
            &bad,
            None,
            &config,
            &mut state,
            &mut counters
        )
        .is_err()
    );
    assert_eq!(state, snapshot);
}

#[test]
fn gcrodr_preserves_bu_equals_c_and_refreshes_changed_operator() {
    let (op, b, x_true) = test_system();
    let pc = IdentityPreconditioner::new(3);
    let config = GcrodrConfig {
        restart: 3,
        max_arnoldi: 30,
        recycle_dim: 1,
        rank_tol: 1e-12,
        rtol: 1e-12,
        atol: 1e-14,
    };
    let mut state = GcrodrState::default();
    let mut counters = WorkCounters::default();
    let r = solve_gcrodr(
        op.as_ref(),
        &pc,
        &b,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert!(r.converged);
    assert!(max_abs_diff(&r.x, &x_true) < 1e-9);
    assert_eq!(state.rank(), 1);
    state.verify_invariant(op.as_ref(), &pc, 1e-9).unwrap();

    let a2 =
        DenseMatrix::from_rows(&[&[4.1, 1.0, 0.0], &[2.0, 3.0, 1.0], &[0.0, -1.0, 2.0]]).unwrap();
    let b2 = a2.matvec(&x_true).unwrap();
    let op2 = DenseOperator::new(a2).unwrap();
    let before_cross = counters.recycle_cross_operator_refreshes;
    let before_refresh = counters.recycle_refresh_matvecs;
    let prior_rank = state.rank() as u64;
    let r2 = solve_gcrodr(&op2, &pc, &b2, None, &config, &mut state, &mut counters).unwrap();
    assert!(r2.converged);
    assert!(counters.recycle_cross_operator_refreshes > before_cross);
    assert_eq!(
        counters.recycle_refresh_matvecs - before_refresh,
        prior_rank,
        "each retained recycle vector must require exactly one refreshed operator image",
    );
    state.verify_invariant(&op2, &pc, 1e-8).unwrap();
}

#[test]
fn gcrodr_cross_operator_refresh_remains_scale_invariant() {
    let a =
        DenseMatrix::from_rows(&[&[0.8, 0.0, 0.0], &[0.0, 1.3, 0.0], &[0.0, 0.0, 2.0]]).unwrap();
    let op = DenseOperator::new(a.clone()).unwrap();
    let pc = IdentityPreconditioner::new(3);
    let config = GcrodrConfig {
        restart: 3,
        max_arnoldi: 30,
        recycle_dim: 1,
        rank_tol: 1e-12,
        rtol: 1e-9,
        atol: 1e-12,
    };
    let mut state = GcrodrState::default();
    let mut counters = WorkCounters::default();
    solve_gcrodr(
        &op,
        &pc,
        &[1.0, -0.4, 0.2],
        Some(&[0.0; 3]),
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert_eq!(state.rank(), 1);

    let rhs = state.image[0].clone();
    state.previous_solution = None;
    let scaled = a.scale(1e-14);
    let scaled_op = DenseOperator::new(scaled).unwrap();
    let before_refresh = counters.recycle_refresh_matvecs;
    solve_gcrodr(
        &scaled_op,
        &pc,
        &rhs,
        Some(&[0.0; 3]),
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert_eq!(counters.recycle_refresh_matvecs - before_refresh, 1);
    assert_eq!(state.rank(), 1);
    state.verify_invariant(&scaled_op, &pc, 2e-8).unwrap();
}

#[test]
fn gcrodr_handles_one_dimensional_recycle_updates_without_panicking() {
    let matrix = DenseMatrix::new(1, 1, vec![3.0]).unwrap();
    let operator = DenseOperator::new(matrix).unwrap();
    let preconditioner = IdentityPreconditioner::new(1);
    let config = GcrodrConfig {
        restart: 8,
        max_arnoldi: 40,
        recycle_dim: 6,
        rank_tol: 1e-12,
        rtol: 1e-12,
        atol: 1e-14,
    };
    let mut state = GcrodrState::default();
    let mut counters = WorkCounters::default();
    for rhs in [[3.0], [6.0], [-1.5], [0.75]] {
        let report = solve_gcrodr(
            &operator,
            &preconditioner,
            &rhs,
            None,
            &config,
            &mut state,
            &mut counters,
        )
        .unwrap();
        assert!(report.converged);
        assert!((3.0 * report.x[0] - rhs[0]).abs() <= 1e-11);
    }
}
