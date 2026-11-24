//! Contract tests of loop node V25 (confirmation of the adaptive
//! matrix-free stage solve P1, owner decision OD-L1-15): the four families
//! of `mf_confirm_problems.rs` and the entries of the measurement program,
//! on reduced instances.

#[path = "mf_confirm_problems.rs"]
mod problems;

use problems::{
    CONSISTENCY_STATES, DECLARED, Family, adaptive, consistency_state, declared, with_end,
};
use rodas5p_core::WorkCounters;
use rodas5p_integrators::{
    MfStageSolvePolicy, OutputSchedule, RODAS5P_MF_FAST_DRIVER_ID, Rodas5pMfFastResult, RoutedRun,
    RoutingOptions, integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_with_stage_solve_policy,
    integrate_rodas5p_routed_observed_with_options,
};

const DEVELOPMENT: [&str; 4] = [
    "dev_ac3d",
    "dev_robertson_rd1d",
    "dev_schnakenberg2d",
    "dev_porous2d",
];

fn rhs(family: &Family, y: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; y.len()];
    family
        .problem
        .eval_rhs_into(family.span.0, y, &mut out, &mut WorkCounters::default())
        .unwrap();
    out
}

fn jvp(family: &Family, y: &[f64], v: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; y.len()];
    let callback = family.problem.jvp_callback().unwrap();
    callback(family.span.0, y, v, &mut out).unwrap();
    out
}

fn max_abs(values: &[f64]) -> f64 {
    values.iter().fold(0.0_f64, |m, v| m.max(v.abs()))
}

#[test]
fn the_declared_instances_have_the_frozen_sizes_and_spans() {
    let expected = [
        ("ac3d", 4096, 1.0),
        ("robertson_rd1d", 1200, 40.0),
        ("schnakenberg2d", 2048, 1.0),
        ("porous2d", 2304, 0.1),
    ];
    for (id, n, t1) in expected {
        let family = declared(id).unwrap();
        assert_eq!(family.problem.dimension, n, "{id}");
        assert_eq!(family.y0.len(), n, "{id}");
        assert_eq!(family.span, (0.0, t1), "{id}");
        assert_eq!(family.atol_over_rtol, 1.0, "{id}: atol equal to rtol");
        assert!(family.problem.autonomous, "{id}");
        assert!(family.problem.supports_matrix_free_jvp(), "{id}");
    }
    assert_eq!(DECLARED.len(), 4);
    let moved = with_end(declared("ac3d").unwrap(), 0.5).unwrap();
    assert_eq!(moved.span, (0.0, 0.5));
    assert!(with_end(declared("ac3d").unwrap(), 0.0).is_err());
}

#[test]
fn the_jvp_is_the_derivative_of_the_right_hand_side() {
    for id in DEVELOPMENT {
        let family = declared(id).unwrap();
        for k in 0..3 {
            let (y, v) = consistency_state(&family, k);
            let jv = jvp(&family, &y, &v);
            let scale = max_abs(&jv).max(1.0);
            // Central difference quotient; the right-hand sides are at most
            // cubic in the state, so the step 1e-4 leaves an error of order
            // 1e-8 times the third derivative.
            let eps = 1.0e-4;
            let plus: Vec<f64> = y.iter().zip(&v).map(|(a, b)| a + eps * b).collect();
            let minus: Vec<f64> = y.iter().zip(&v).map(|(a, b)| a - eps * b).collect();
            let (fp, fm) = (rhs(&family, &plus), rhs(&family, &minus));
            let worst = jv
                .iter()
                .zip(fp.iter().zip(&fm))
                .map(|(j, (p, m))| (j - (p - m) / (2.0 * eps)).abs())
                .fold(0.0_f64, f64::max);
            assert!(
                worst <= 1.0e-5 * scale,
                "{id} state {k}: JVP against the difference quotient {worst:e} (scale {scale:e})"
            );
        }
    }
}

#[test]
fn conservation_and_equilibria_of_the_families() {
    // Porous medium with zero flux: the sum of the right-hand side vanishes.
    let porous = declared("dev_porous2d").unwrap();
    let (y, _) = consistency_state(&porous, 0);
    let f = rhs(&porous, &y);
    let total: f64 = f.iter().sum();
    assert!(total.abs() <= 1.0e-10 * max_abs(&f).max(1.0), "{total:e}");
    // Robertson with zero flux: the three species are conserved together.
    let robertson = declared("dev_robertson_rd1d").unwrap();
    let f = rhs(&robertson, &robertson.y0);
    let total: f64 = f.iter().sum();
    assert!(total.abs() <= 1.0e-12 * max_abs(&f).max(1.0), "{total:e}");
    // Allen-Cahn: a uniform state has a zero Laplacian.
    let ac = declared("dev_ac3d").unwrap();
    let uniform = vec![0.5; ac.y0.len()];
    let f = rhs(&ac, &uniform);
    assert!(
        f.iter()
            .all(|v| (v - 10.0 * (0.5 - 0.125)).abs() <= 1.0e-12)
    );
    // Schnakenberg: the homogeneous equilibrium u = a + b, v = b/(a + b)^2.
    let sch = declared("dev_schnakenberg2d").unwrap();
    let equilibrium: Vec<f64> = (0..sch.y0.len())
        .map(|i| if i % 2 == 0 { 1.0 } else { 0.9 })
        .collect();
    let f = rhs(&sch, &equilibrium);
    assert!(max_abs(&f) <= 1.0e-12, "{:e}", max_abs(&f));
}

#[test]
fn the_consistency_states_are_deterministic() {
    for id in DEVELOPMENT {
        let family = declared(id).unwrap();
        for k in [0, CONSISTENCY_STATES - 1] {
            let first = consistency_state(&family, k);
            let second = consistency_state(&family, k);
            assert_eq!(first, second, "{id} {k}");
        }
        assert_ne!(
            consistency_state(&family, 0).0,
            consistency_state(&family, 1).0
        );
    }
}

fn line(run: &Rodas5pMfFastResult) -> (bool, usize, usize, usize, Vec<u64>, String) {
    (
        run.observed.success,
        run.attempts,
        run.accepted_steps,
        run.rejected_steps,
        run.observed
            .y
            .last()
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect(),
        serde_json::to_string(&run.observed.counters).unwrap(),
    )
}

/// The entries of the measurement program agree bit for bit: the shipped
/// entry, the explicit-policy entry and the router with the policy named,
/// with and without the least-squares workspace.
#[test]
fn the_entries_of_the_measurement_program_agree() {
    for id in DEVELOPMENT {
        let family = declared(id).unwrap();
        let config = adaptive(&family, 1.0e-5);
        let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
        let linear = RoutingOptions::default().matrix_free;
        for policy in [
            MfStageSolvePolicy::Shipped,
            MfStageSolvePolicy::ColumnTested,
            MfStageSolvePolicy::ColumnTestedDoubling,
        ] {
            let explicit = integrate_rodas5p_mf_fast_observed_with_stage_solve_policy(
                &family.problem,
                family.span,
                &family.y0,
                &linear,
                &config,
                &output,
                policy,
                false,
            )
            .unwrap();
            assert!(explicit.observed.success, "{id} {policy:?}");
            assert_eq!(explicit.driver, policy.driver_id());
            if policy == MfStageSolvePolicy::Shipped {
                let shipped = integrate_rodas5p_mf_fast_observed_gmres_into(
                    &family.problem,
                    family.span,
                    &family.y0,
                    &linear,
                    &config,
                    &output,
                )
                .unwrap();
                assert_eq!(line(&shipped), line(&explicit), "{id} shipped entry");
                assert_eq!(shipped.driver, RODAS5P_MF_FAST_DRIVER_ID);
            }
            for ls in [false, true] {
                let options = RoutingOptions {
                    matrix_free: linear.clone(),
                    krylov_ls_workspace: ls,
                    matrix_free_policy: policy,
                };
                let routed = integrate_rodas5p_routed_observed_with_options(
                    &family.problem,
                    family.span,
                    &family.y0,
                    &config,
                    &output,
                    &options,
                )
                .unwrap();
                assert_eq!(routed.routing.driver_id, policy.driver_id());
                let RoutedRun::MatrixFree(run) = &routed.run else {
                    panic!("{id}: a JVP-only problem takes the matrix-free route");
                };
                assert_eq!(line(run), line(&explicit), "{id} {policy:?} router {ls}");
            }
        }
    }
}
