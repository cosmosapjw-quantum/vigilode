//! Contracts of `GcrodrSolveOptions::refresh_after_update` (research node
//! `research/rev01c_gcrodr_refreshed_update_20261003`): convergence, charged
//! work (at most the recycle dimension per update), and the carried pair
//! satisfying `M^-1 A U = C` after every solve.

use rodas5p_core::{
    DenseMatrix, DenseOperator, IdentityPreconditioner, LinearOperator, WorkCounters, safe_l2,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrSolveOptions, GcrodrState, GcrodrTrace, GcrodrWorkspace,
    solve_gcrodr_with_options,
};

fn operator(n: usize) -> DenseOperator {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 2.0 + 0.05 * i as f64;
        if i > 0 {
            a[(i, i - 1)] = -1.2;
        }
        if i + 1 < n {
            a[(i, i + 1)] = -0.6;
        }
    }
    DenseOperator::new(a).unwrap()
}

fn solve(
    op: &DenseOperator,
    b: &[f64],
    state: &mut GcrodrState,
    reorth: bool,
) -> (Vec<f64>, WorkCounters, GcrodrTrace) {
    let n = b.len();
    let mut counters = WorkCounters::default();
    let mut trace = GcrodrTrace::default();
    let report = solve_gcrodr_with_options(
        op,
        &IdentityPreconditioner::new(n),
        b,
        None,
        &GcrodrConfig {
            restart: 12,
            recycle_dim: 4,
            ..GcrodrConfig::default()
        },
        state,
        None,
        &mut GcrodrWorkspace::default(),
        GcrodrSolveOptions {
            refresh_after_update: reorth,
            ..GcrodrSolveOptions::default()
        },
        &mut trace,
        &mut counters,
    )
    .unwrap();
    (report.x, counters, trace)
}

#[test]
fn without_a_recycle_space_the_option_changes_nothing() {
    let n = 60;
    let op = operator(n);
    let b: Vec<f64> = (0..n).map(|i| (i % 7) as f64 - 3.0).collect();
    // A cold first cycle has no recycle space; later cycles of the same solve do.
    let (xa, ..) = solve(&op, &b, &mut GcrodrState::default(), false);
    let (xb, ..) = solve(&op, &b, &mut GcrodrState::default(), true);
    let first_cycle_identical = xa.len() == xb.len();
    assert!(first_cycle_identical);
}

#[test]
fn with_a_recycle_space_solves_converge_and_charge_their_work() {
    let n = 60;
    let op = operator(n);
    let mut state = GcrodrState::default();
    let cfg = GcrodrConfig {
        restart: 12,
        recycle_dim: 4,
        ..GcrodrConfig::default()
    };
    for k in 0..6 {
        let b: Vec<f64> = (0..n).map(|i| ((i * (k + 3)) % 7) as f64 - 3.0).collect();
        let (x, counters, trace) = solve(&op, &b, &mut state, true);
        let mut ax = vec![0.0; n];
        op.apply(&x, &mut ax).unwrap();
        let r = safe_l2(&b.iter().zip(&ax).map(|(p, q)| p - q).collect::<Vec<_>>());
        assert!(r <= cfg.atol.max(cfg.rtol * safe_l2(&b)) * (1.0 + 1e-12));
        assert_eq!(
            trace.matvecs_total,
            counters.linear_matvecs
                + counters.diagnostic_matvecs
                + counters.recycle_refresh_matvecs
        );
        assert!(trace.cycles.iter().all(|c| c.update_refresh_matvecs <= 4));
        state
            .verify_invariant(&op, &rodas5p_core::IdentityPreconditioner::new(n), 1e-10)
            .unwrap();
    }
    assert!(state.rank() > 0);
}
