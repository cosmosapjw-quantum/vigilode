//! WU-1 (audit F-010): GCRO-DR state validity across dimension and system changes.
//!
//! A `GcrodrState` carries a warm start (`previous_solution`) and a recycle
//! space.  Both belong to one exact Krylov system identity.  These contracts
//! pin that a state reused on another system neither aborts nor seeds the new
//! system with the old solution, and that the recycle invariant survives rank
//! greater than one across operator and preconditioner changes.

use rodas5p_core::{
    CoreError, DenseMatrix, DenseOperator, IdentityPreconditioner, JacobiPreconditioner,
    WorkCounters, safe_l2,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, LgmresConfig, LgmresState, solve_gcrodr, solve_lgmres,
};

/// Upper-banded nonnormal matrix `D + s N` with a negative log-spaced diagonal.
///
/// The E-05 harness uses an orthogonal similarity of the same shape; the banded
/// form keeps the test independent of the harness crate and of random
/// orthogonal factors while still exercising a nonnormal recycle space.
fn nonnormal_matrix(n: usize, shift: f64) -> DenseMatrix {
    let mut rows = vec![vec![0.0; n]; n];
    for (i, row) in rows.iter_mut().enumerate() {
        let exponent = i as f64 / (n - 1) as f64;
        row[i] = -(10.0_f64.powf(exponent)) + shift;
        for offset in 1..=2 {
            if i + offset < n {
                row[i + offset] = 0.5 / offset as f64;
            }
        }
    }
    DenseMatrix::from_vec_rows(rows).unwrap()
}

fn deterministic_rhs(n: usize, seed: u64) -> Vec<f64> {
    let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 11) as f64 / (1u64 << 53) as f64) - 0.5
        })
        .collect()
}

fn true_residual_l2(matrix: &DenseMatrix, rhs: &[f64], x: &[f64]) -> f64 {
    let ax = matrix.matvec(x).unwrap();
    safe_l2(&rhs.iter().zip(&ax).map(|(b, y)| b - y).collect::<Vec<_>>())
}

fn wide_recycle_config() -> GcrodrConfig {
    GcrodrConfig {
        restart: 10,
        max_arnoldi: 400,
        recycle_dim: 4,
        rank_tol: 1e-12,
        rtol: 1e-10,
        atol: 0.0,
    }
}

#[test]
fn gcrodr_state_reused_across_a_dimension_change_returns_without_panicking() {
    // E-05 child scenario: an n=32 solve followed by an n=16 solve with the same state.
    let config = wide_recycle_config();
    let a32 = nonnormal_matrix(32, 0.0);
    let a16 = nonnormal_matrix(16, -0.25);
    let b32 = deterministic_rhs(32, 1);
    let b16 = deterministic_rhs(16, 2);
    let mut state = GcrodrState::default();
    let mut counters = WorkCounters::default();
    solve_gcrodr(
        &DenseOperator::new(a32).unwrap(),
        &IdentityPreconditioner::new(32),
        &b32,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert_eq!(state.previous_solution.as_ref().map(Vec::len), Some(32));

    let report = solve_gcrodr(
        &DenseOperator::new(a16.clone()).unwrap(),
        &IdentityPreconditioner::new(16),
        &b16,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .expect("a dimension change must drop the stale warm start, not abort");
    assert!(report.converged);
    assert!(true_residual_l2(&a16, &b16, &report.x) <= config.rtol * safe_l2(&b16));
    assert!(state.basis.iter().all(|vector| vector.len() == 16));
    assert_eq!(state.previous_solution.as_ref().map(Vec::len), Some(16));
}

#[test]
fn gcrodr_wrong_length_warm_start_under_matching_identity_is_a_typed_error() {
    // A hand-built corrupt state: identity of the n=16 system, warm start of length 32.
    let config = wide_recycle_config();
    let a16 = nonnormal_matrix(16, 0.0);
    let op = DenseOperator::new(a16).unwrap();
    let pc = IdentityPreconditioner::new(16);
    let rhs = deterministic_rhs(16, 3);
    let mut state = GcrodrState::default();
    let mut counters = WorkCounters::default();
    solve_gcrodr(&op, &pc, &rhs, None, &config, &mut state, &mut counters).unwrap();
    state.previous_solution = Some(vec![0.25; 32]);
    let snapshot = state.clone();

    let error = solve_gcrodr(&op, &pc, &rhs, None, &config, &mut state, &mut counters)
        .expect_err("a corrupt warm start must be rejected");
    assert!(
        matches!(error, CoreError::Dimension(_)),
        "unexpected error {error:?}"
    );
    assert_eq!(
        state, snapshot,
        "a rejected solve must leave the state untouched"
    );
}

#[test]
fn gcrodr_same_system_keeps_the_warm_start() {
    let config = wide_recycle_config();
    let a = nonnormal_matrix(24, 0.0);
    let op = DenseOperator::new(a.clone()).unwrap();
    let pc = IdentityPreconditioner::new(24);
    let rhs = deterministic_rhs(24, 4);
    let mut state = GcrodrState::default();
    let mut counters = WorkCounters::default();
    solve_gcrodr(&op, &pc, &rhs, None, &config, &mut state, &mut counters).unwrap();

    // Reconstructing the operator from the same matrix keeps the exact identity.
    let rebuilt = DenseOperator::new(a).unwrap();
    let again = solve_gcrodr(
        &rebuilt,
        &pc,
        &rhs,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert_eq!(
        again.iterations, 0,
        "the converged warm start of the identical system must be reused"
    );
}

#[test]
fn gcrodr_operator_identity_change_clears_the_stale_warm_start() {
    let config = wide_recycle_config();
    let first = DenseOperator::new(nonnormal_matrix(24, 0.0)).unwrap();
    let changed_matrix = nonnormal_matrix(24, -0.5);
    let changed = DenseOperator::new(changed_matrix.clone()).unwrap();
    let pc = IdentityPreconditioner::new(24);
    let rhs_first = deterministic_rhs(24, 5);
    let rhs_changed = deterministic_rhs(24, 6);
    let mut state = GcrodrState::default();
    let mut counters = WorkCounters::default();
    solve_gcrodr(
        &first,
        &pc,
        &rhs_first,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert!(state.previous_solution.is_some());

    // Same state without the stale warm start: the reference for the changed system.
    let mut without_warm_start = state.clone();
    without_warm_start.previous_solution = None;
    let mut reference_counters = WorkCounters::default();
    let reference = solve_gcrodr(
        &changed,
        &pc,
        &rhs_changed,
        None,
        &config,
        &mut without_warm_start,
        &mut reference_counters,
    )
    .unwrap();

    let report = solve_gcrodr(
        &changed,
        &pc,
        &rhs_changed,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert!(
        true_residual_l2(&changed_matrix, &rhs_changed, &report.x)
            <= config.rtol * safe_l2(&rhs_changed)
    );
    assert_eq!(
        report.x.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        reference.x.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        "the previous system's solution must not seed the changed system"
    );
    assert_eq!(report.iterations, reference.iterations);
    assert_eq!(state, without_warm_start);
}

#[test]
fn gcrodr_recycle_rank_above_one_keeps_its_invariant_across_system_changes() {
    let config = wide_recycle_config();
    let n = 32;
    let a = nonnormal_matrix(n, 0.0);
    let op = DenseOperator::new(a.clone()).unwrap();
    let identity = IdentityPreconditioner::new(n);
    let mut state = GcrodrState::default();
    let mut counters = WorkCounters::default();
    for seed in 0..4 {
        let rhs = deterministic_rhs(n, 10 + seed);
        let report = solve_gcrodr(
            &op,
            &identity,
            &rhs,
            None,
            &config,
            &mut state,
            &mut counters,
        )
        .unwrap();
        assert!(report.converged);
        assert!(true_residual_l2(&a, &rhs, &report.x) <= config.rtol * safe_l2(&rhs));
        assert!(state.rank() >= 1 && state.rank() <= config.recycle_dim);
        state.verify_invariant(&op, &identity, 1e-9).unwrap();
    }
    assert!(
        state.rank() > 1,
        "the test must exercise a rank > 1 recycle space"
    );

    let jacobi = JacobiPreconditioner::from_matrix(&a).unwrap();
    let rhs = deterministic_rhs(n, 20);
    let before = counters;
    let report =
        solve_gcrodr(&op, &jacobi, &rhs, None, &config, &mut state, &mut counters).unwrap();
    assert!(true_residual_l2(&a, &rhs, &report.x) <= config.rtol * safe_l2(&rhs));
    assert_eq!(counters.delta(before).recycle_cross_operator_refreshes, 1);
    state.verify_invariant(&op, &jacobi, 1e-9).unwrap();

    let shifted_matrix = nonnormal_matrix(n, -1e-3);
    let shifted = DenseOperator::new(shifted_matrix.clone()).unwrap();
    let shifted_jacobi = JacobiPreconditioner::from_matrix(&shifted_matrix).unwrap();
    let rhs = deterministic_rhs(n, 21);
    let before = counters;
    let report = solve_gcrodr(
        &shifted,
        &shifted_jacobi,
        &rhs,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert!(true_residual_l2(&shifted_matrix, &rhs, &report.x) <= config.rtol * safe_l2(&rhs));
    assert_eq!(counters.delta(before).recycle_cross_operator_refreshes, 1);
    state
        .verify_invariant(&shifted, &shifted_jacobi, 1e-9)
        .unwrap();
}

#[test]
fn lgmres_state_reused_across_a_dimension_change_returns_without_panicking() {
    let config = LgmresConfig {
        inner_m: 10,
        max_outer: 40,
        outer_k: 4,
        rtol: 1e-10,
        atol: 0.0,
    };
    let a32 = nonnormal_matrix(32, 0.0);
    let a16 = nonnormal_matrix(16, -0.25);
    let b32 = deterministic_rhs(32, 7);
    let b16 = deterministic_rhs(16, 8);
    let mut state = LgmresState::default();
    let mut counters = WorkCounters::default();
    solve_lgmres(
        &DenseOperator::new(a32).unwrap(),
        &IdentityPreconditioner::new(32),
        &b32,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    let report = solve_lgmres(
        &DenseOperator::new(a16.clone()).unwrap(),
        &IdentityPreconditioner::new(16),
        &b16,
        None,
        &config,
        &mut state,
        &mut counters,
    )
    .unwrap();
    assert!(true_residual_l2(&a16, &b16, &report.x) <= config.rtol * safe_l2(&b16));
}
