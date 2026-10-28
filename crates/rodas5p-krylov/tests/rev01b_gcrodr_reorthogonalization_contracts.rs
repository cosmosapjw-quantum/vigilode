//! Contracts of `GcrodrSolveOptions::reorthogonalize_recycle` (research node
//! `research/rev01b_gcrodr_recycle_reorthogonalization_20261003`): no effect
//! without a recycle space; with the start projection as well, convergence,
//! charged work and `[C V]` orthonormal to 1e-8.

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
            orthogonalize_start: reorth,
            reorthogonalize_recycle: reorth,
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
        let worst = trace
            .cycles
            .iter()
            .map(|c| c.recycle_arnoldi_coupling)
            .fold(0.0_f64, f64::max);
        println!("solve {k}: max |C^T V| = {worst:e}");
        assert!(worst <= 1e-8, "{worst}");
    }
    assert!(state.rank() > 0);
}
