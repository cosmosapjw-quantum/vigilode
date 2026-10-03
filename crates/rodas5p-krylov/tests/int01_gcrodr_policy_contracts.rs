//! Contracts of `solve_gcrodr_with_policy` (research node
//! `research/int01_gcrodr_verified_reuse_20261003`): the default policy is
//! the plain solve, a healthy carried pair passes the reuse check
//! unchanged, a corrupted one is rebuilt, the tolerance is validated, and a
//! solve that aborts inside a cycle keeps that cycle in its trace.

use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{
    CoreResult, DenseMatrix, DenseOperator, IdentityPreconditioner, LinearOperator, WorkCounters,
    safe_l2,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrReusePolicy, GcrodrState, GcrodrTrace, GcrodrWorkspace,
    solve_gcrodr_traced, solve_gcrodr_with_policy, solve_gcrodr_with_workspace_and_residual_scale,
};

fn matrix(n: usize, shift: f64) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 2.0 + shift + 0.05 * i as f64;
        if i > 0 {
            a[(i, i - 1)] = -1.2;
        }
        if i + 1 < n {
            a[(i, i + 1)] = -0.6;
        }
    }
    a
}

fn config() -> GcrodrConfig {
    GcrodrConfig {
        restart: 12,
        recycle_dim: 4,
        ..GcrodrConfig::default()
    }
}

fn rhs(n: usize, k: usize) -> Vec<f64> {
    (0..n).map(|i| ((i * (k + 3)) % 7) as f64 - 3.0).collect()
}

fn charged(c: &WorkCounters) -> u64 {
    c.linear_matvecs + c.diagnostic_matvecs + c.recycle_refresh_matvecs
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn the_default_policy_is_the_plain_and_the_traced_solve() {
    let n = 60;
    let pc = IdentityPreconditioner::new(n);
    let mut states = [
        GcrodrState::default(),
        GcrodrState::default(),
        GcrodrState::default(),
    ];
    let mut workspaces = [
        GcrodrWorkspace::default(),
        GcrodrWorkspace::default(),
        GcrodrWorkspace::default(),
    ];
    for k in 0..6 {
        let op = DenseOperator::new(matrix(n, 0.01 * (k / 2) as f64)).unwrap();
        let b = rhs(n, k);
        let [plain_state, traced_state, policy_state] = &mut states;
        let [plain_ws, traced_ws, policy_ws] = &mut workspaces;
        let mut plain_counters = WorkCounters::default();
        let plain = solve_gcrodr_with_workspace_and_residual_scale(
            &op,
            &pc,
            &b,
            None,
            &config(),
            plain_state,
            None,
            plain_ws,
            &mut plain_counters,
        )
        .unwrap();
        let mut traced_counters = WorkCounters::default();
        let traced = solve_gcrodr_traced(
            &op,
            &pc,
            &b,
            None,
            &config(),
            traced_state,
            None,
            traced_ws,
            None,
            &mut GcrodrTrace::default(),
            &mut traced_counters,
        )
        .unwrap();
        let mut policy_counters = WorkCounters::default();
        let mut trace = GcrodrTrace::default();
        let policy = solve_gcrodr_with_policy(
            &op,
            &pc,
            &b,
            None,
            &config(),
            policy_state,
            None,
            policy_ws,
            GcrodrReusePolicy::default(),
            &mut trace,
            &mut policy_counters,
        )
        .unwrap();
        for other in [&traced, &policy] {
            assert_eq!(bits(&plain.x), bits(&other.x));
            assert_eq!(plain.residual_norm.to_bits(), other.residual_norm.to_bits());
            assert_eq!(plain.iterations, other.iterations);
        }
        assert_eq!(plain_counters, traced_counters);
        assert_eq!(plain_counters, policy_counters);
        assert_eq!(states[0], states[1]);
        assert_eq!(states[0], states[2]);
        assert!(trace.reuse_checks.is_empty());
        assert!(trace.cycles.iter().all(|c| !c.aborted));
        assert_eq!(trace.matvecs_total, charged(&policy_counters));
    }
}

/// Solves `b` on `op` twice with a shared state: the second solve reuses
/// the carried pair (same operator object).
fn warm_state(op: &DenseOperator, n: usize) -> GcrodrState {
    let pc = IdentityPreconditioner::new(n);
    let mut state = GcrodrState::default();
    for k in 0..2 {
        solve_gcrodr_with_workspace_and_residual_scale(
            op,
            &pc,
            &rhs(n, k),
            None,
            &config(),
            &mut state,
            None,
            &mut GcrodrWorkspace::default(),
            &mut WorkCounters::default(),
        )
        .unwrap();
    }
    assert!(state.rank() > 0);
    state
}

#[test]
fn a_healthy_pair_passes_the_check_and_the_solve_is_unchanged() {
    let n = 60;
    let op = DenseOperator::new(matrix(n, 0.0)).unwrap();
    let pc = IdentityPreconditioner::new(n);
    let base = warm_state(&op, n);
    let b = rhs(n, 5);
    let (mut plain_state, mut checked_state) = (base.clone(), base.clone());
    let mut plain_counters = WorkCounters::default();
    let plain = solve_gcrodr_with_workspace_and_residual_scale(
        &op,
        &pc,
        &b,
        None,
        &config(),
        &mut plain_state,
        None,
        &mut GcrodrWorkspace::default(),
        &mut plain_counters,
    )
    .unwrap();
    let mut counters = WorkCounters::default();
    let mut trace = GcrodrTrace::default();
    let checked = solve_gcrodr_with_policy(
        &op,
        &pc,
        &b,
        None,
        &config(),
        &mut checked_state,
        None,
        &mut GcrodrWorkspace::default(),
        GcrodrReusePolicy {
            reset_factor: None,
            verify_reuse: Some(1.0e-8),
        },
        &mut trace,
        &mut counters,
    )
    .unwrap();
    assert_eq!(trace.reuse_checks.len(), 1);
    let check = trace.reuse_checks[0];
    assert_eq!(check.rank, base.rank());
    assert!(!check.rebuilt && check.defect <= 1.0e-8, "{check:?}");
    assert_eq!(bits(&plain.x), bits(&checked.x));
    assert_eq!(plain_state, checked_state);
    // The check costs exactly one product per carried vector, charged.
    assert_eq!(
        counters.recycle_refresh_matvecs,
        plain_counters.recycle_refresh_matvecs + base.rank() as u64
    );
    assert_eq!(trace.matvecs_total, charged(&counters));
}

#[test]
fn a_corrupted_image_is_detected_and_rebuilt() {
    let n = 60;
    let op = DenseOperator::new(matrix(n, 0.0)).unwrap();
    let pc = IdentityPreconditioner::new(n);
    let mut state = warm_state(&op, n);
    state.image[0][3] += 0.5;
    let b = rhs(n, 5);
    let mut counters = WorkCounters::default();
    let mut trace = GcrodrTrace::default();
    let report = solve_gcrodr_with_policy(
        &op,
        &pc,
        &b,
        None,
        &config(),
        &mut state,
        None,
        &mut GcrodrWorkspace::default(),
        GcrodrReusePolicy {
            reset_factor: None,
            verify_reuse: Some(1.0e-8),
        },
        &mut trace,
        &mut counters,
    )
    .unwrap();
    let check = trace.reuse_checks[0];
    assert!(check.rebuilt && check.defect >= 0.4, "{check:?}");
    let mut ax = vec![0.0; n];
    op.apply(&report.x, &mut ax).unwrap();
    let residual = safe_l2(&b.iter().zip(&ax).map(|(p, q)| p - q).collect::<Vec<_>>());
    let cfg = config();
    assert!(residual <= cfg.atol.max(cfg.rtol * safe_l2(&b)) * (1.0 + 1.0e-12));
    assert_eq!(trace.matvecs_total, charged(&counters));
}

#[test]
fn the_reuse_tolerance_is_validated() {
    let n = 8;
    let op = DenseOperator::new(matrix(n, 0.0)).unwrap();
    let pc = IdentityPreconditioner::new(n);
    for tol in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let result = solve_gcrodr_with_policy(
            &op,
            &pc,
            &vec![1.0; n],
            None,
            &GcrodrConfig::default(),
            &mut GcrodrState::default(),
            None,
            &mut GcrodrWorkspace::default(),
            GcrodrReusePolicy {
                reset_factor: None,
                verify_reuse: Some(tol),
            },
            &mut GcrodrTrace::default(),
            &mut WorkCounters::default(),
        );
        assert!(result.is_err(), "tolerance {tol}");
    }
}

/// An operator whose `fail_at`-th application returns NaN.
struct FailingOperator {
    inner: DenseOperator,
    calls: AtomicUsize,
    fail_at: usize,
}

impl LinearOperator for FailingOperator {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.inner.apply(x, y)?;
        if self.calls.fetch_add(1, Ordering::SeqCst) + 1 == self.fail_at {
            y[0] = f64::NAN;
        }
        Ok(())
    }
    fn token(&self) -> u64 {
        self.inner.token()
    }
}

#[test]
fn a_solve_aborted_inside_a_cycle_keeps_that_cycle_in_its_trace() {
    let n = 60;
    let op = FailingOperator {
        inner: DenseOperator::new(matrix(n, 0.0)).unwrap(),
        calls: AtomicUsize::new(0),
        fail_at: 5,
    };
    let pc = IdentityPreconditioner::new(n);
    let mut state = GcrodrState::default();
    let mut counters = WorkCounters::default();
    let mut trace = GcrodrTrace::default();
    let result = solve_gcrodr_with_policy(
        &op,
        &pc,
        &rhs(n, 1),
        None,
        &config(),
        &mut state,
        None,
        &mut GcrodrWorkspace::default(),
        GcrodrReusePolicy::default(),
        &mut trace,
        &mut counters,
    );
    assert!(result.is_err());
    let last = trace.cycles.last().expect("the aborted cycle is traced");
    assert!(last.aborted && last.matvecs > 0, "{last:?}");
    assert_eq!(trace.matvecs_total, charged(&counters));
    assert_eq!(
        trace.matvecs_total,
        trace.cycles.iter().map(|c| c.matvecs).sum::<u64>() + trace.matvecs_outside_cycles
    );
    assert_eq!(state, GcrodrState::default());
}
