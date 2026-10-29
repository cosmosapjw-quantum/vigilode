//! The GCRO-DR trace hook does not change the solve, and the reset factor is
//! validated (research node `research/rnext03_gcrodr_attribution_20261003`).

use rodas5p_core::{DenseMatrix, DenseOperator, IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GcrodrTrace, GcrodrWorkspace, solve_gcrodr_traced,
    solve_gcrodr_with_workspace_and_residual_scale,
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

#[test]
fn tracing_without_reset_is_the_plain_solve() {
    let n = 60;
    let config = GcrodrConfig {
        restart: 12,
        recycle_dim: 4,
        ..GcrodrConfig::default()
    };
    let pc = IdentityPreconditioner::new(n);
    let (mut plain_state, mut traced_state) = (GcrodrState::default(), GcrodrState::default());
    let (mut plain_ws, mut traced_ws) = (GcrodrWorkspace::default(), GcrodrWorkspace::default());
    for k in 0..6 {
        // Slowly varying operators so the recycle space is refreshed and reused.
        let op = operator(n, 0.01 * (k / 2) as f64);
        let b: Vec<f64> = (0..n).map(|i| ((i * (k + 3)) % 7) as f64 - 3.0).collect();
        let mut plain_counters = WorkCounters::default();
        let plain = solve_gcrodr_with_workspace_and_residual_scale(
            &op,
            &pc,
            &b,
            None,
            &config,
            &mut plain_state,
            None,
            &mut plain_ws,
            &mut plain_counters,
        )
        .unwrap();
        let mut traced_counters = WorkCounters::default();
        let mut trace = GcrodrTrace::default();
        let traced = solve_gcrodr_traced(
            &op,
            &pc,
            &b,
            None,
            &config,
            &mut traced_state,
            None,
            &mut traced_ws,
            None,
            &mut trace,
            &mut traced_counters,
        )
        .unwrap();
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&plain.x), bits(&traced.x));
        assert_eq!(
            plain.residual_norm.to_bits(),
            traced.residual_norm.to_bits()
        );
        assert_eq!(plain.iterations, traced.iterations);
        assert_eq!(plain_counters, traced_counters);
        assert_eq!(plain_state, traced_state);
        assert!(!trace.cycles.is_empty());
        assert_eq!(
            trace.matvecs_total,
            traced_counters.linear_matvecs
                + traced_counters.diagnostic_matvecs
                + traced_counters.recycle_refresh_matvecs
        );
    }
}

#[test]
fn the_reset_factor_must_lie_in_the_open_unit_interval() {
    let n = 8;
    let op = operator(n, 0.0);
    let pc = IdentityPreconditioner::new(n);
    for factor in [0.0, 1.0, -0.5, f64::NAN] {
        let error = solve_gcrodr_traced(
            &op,
            &pc,
            &vec![1.0; n],
            None,
            &GcrodrConfig::default(),
            &mut GcrodrState::default(),
            None,
            &mut GcrodrWorkspace::default(),
            Some(factor),
            &mut GcrodrTrace::default(),
            &mut WorkCounters::default(),
        );
        assert!(error.is_err(), "factor {factor}");
    }
}
