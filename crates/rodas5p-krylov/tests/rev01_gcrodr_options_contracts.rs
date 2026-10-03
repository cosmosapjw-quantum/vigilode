//! Contracts of `solve_gcrodr_with_options` (research node
//! `research/rev01_gcrodr_start_projection_20261003`): without the start
//! projection it is `solve_gcrodr_with_policy` bit for bit; with it, a
//! recycled solve converges and charges its extra work.

use rodas5p_core::{
    DenseMatrix, DenseOperator, IdentityPreconditioner, LinearOperator, WorkCounters, safe_l2,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrReusePolicy, GcrodrSolveOptions, GcrodrState, GcrodrTrace, GcrodrWorkspace,
    solve_gcrodr_with_options, solve_gcrodr_with_policy,
};

fn operator(n: usize, shift: f64) -> DenseOperator {
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
    DenseOperator::new(a).unwrap()
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

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn without_the_start_projection_the_options_are_the_policy() {
    let n = 60;
    let pc = IdentityPreconditioner::new(n);
    for policy in [
        GcrodrReusePolicy::default(),
        GcrodrReusePolicy {
            reset_factor: Some(0.5),
            verify_reuse: None,
        },
        GcrodrReusePolicy {
            reset_factor: None,
            verify_reuse: Some(1e-8),
        },
    ] {
        let (mut sa, mut sb) = (GcrodrState::default(), GcrodrState::default());
        let (mut wa, mut wb) = (GcrodrWorkspace::default(), GcrodrWorkspace::default());
        for k in 0..6 {
            let op = operator(n, 0.01 * (k / 2) as f64);
            let b = rhs(n, k);
            let (mut ca, mut cb) = (WorkCounters::default(), WorkCounters::default());
            let (mut ta, mut tb) = (GcrodrTrace::default(), GcrodrTrace::default());
            let a = solve_gcrodr_with_policy(
                &op,
                &pc,
                &b,
                None,
                &config(),
                &mut sa,
                None,
                &mut wa,
                policy,
                &mut ta,
                &mut ca,
            )
            .unwrap();
            let o = solve_gcrodr_with_options(
                &op,
                &pc,
                &b,
                None,
                &config(),
                &mut sb,
                None,
                &mut wb,
                GcrodrSolveOptions {
                    policy,
                    orthogonalize_start: false,
                },
                &mut tb,
                &mut cb,
            )
            .unwrap();
            assert_eq!(bits(&a.x), bits(&o.x));
            assert_eq!(a.iterations, o.iterations);
            assert_eq!(ca, cb);
            assert_eq!(sa, sb);
            assert_eq!(ta, tb);
        }
    }
}

#[test]
fn with_the_start_projection_recycled_solves_converge_and_charge_their_work() {
    let n = 60;
    let pc = IdentityPreconditioner::new(n);
    let cfg = config();
    let mut state = GcrodrState::default();
    let mut workspace = GcrodrWorkspace::default();
    let op = operator(n, 0.0);
    for k in 0..6 {
        let b = rhs(n, k);
        let mut counters = WorkCounters::default();
        let mut trace = GcrodrTrace::default();
        let report = solve_gcrodr_with_options(
            &op,
            &pc,
            &b,
            None,
            &cfg,
            &mut state,
            None,
            &mut workspace,
            GcrodrSolveOptions {
                policy: GcrodrReusePolicy::default(),
                orthogonalize_start: true,
            },
            &mut trace,
            &mut counters,
        )
        .unwrap();
        let mut ax = vec![0.0; n];
        op.apply(&report.x, &mut ax).unwrap();
        let r = safe_l2(&b.iter().zip(&ax).map(|(p, q)| p - q).collect::<Vec<_>>());
        assert!(r <= cfg.atol.max(cfg.rtol * safe_l2(&b)) * (1.0 + 1e-12));
        assert_eq!(
            trace.matvecs_total,
            counters.linear_matvecs
                + counters.diagnostic_matvecs
                + counters.recycle_refresh_matvecs
        );
    }
    assert!(state.rank() > 0);
}
