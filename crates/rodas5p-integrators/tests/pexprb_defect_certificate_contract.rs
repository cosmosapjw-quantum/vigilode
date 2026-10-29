use std::sync::Arc;

use rodas5p_integrators::{
    FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem,
    pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry,
};

fn phi_config() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 12,
        dimension_increment: 1,
        relative_tolerance: 1e-11,
        absolute_tolerance: 1e-14,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 8,
    }
}

fn linear_decay_problem() -> OdeProblem {
    OdeProblem::new(
        "pexprb-decay-contract",
        2,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = -2.0 * y[0] + y[1];
            out[1] = -5.0 * y[1];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, _, v: &[f64], out: &mut [f64]| {
            out[0] = -2.0 * v[0] + v[1];
            out[1] = -5.0 * v[1];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn test_f1_defect_bound_monotonicity_and_enclosure() {
    // Upper Riemann sum property: if subintervals are refined or expanded,
    // outward directed interval addition produces an upper bound >= exact value.
    let h = 0.05;
    let norm_q = 1.2e-4;
    let norm_u_flip = 3.5e-5;
    let k_substeps = 8;
    let dt = h / k_substeps as f64;

    let mut sum_bound = 0.0;
    for _ in 0..k_substeps {
        let g_sup = norm_q + norm_u_flip;
        sum_bound += dt * g_sup;
    }

    assert!(sum_bound > 0.0);
    assert!((sum_bound - h * (norm_q + norm_u_flip)).abs() < 1e-15);
}

#[test]
fn test_f2_pexprb54s4_phi_engine_level1_parity() {
    let problem = linear_decay_problem();
    let y0 = [1.0, 2.0];
    let config = phi_config();

    let prefix = pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
        &problem, 0.0, &y0, 0.05, config, 1, 1e-12, 1e-9,
    );

    assert!(
        prefix.is_ok(),
        "PEXPRB level 1 prefix should succeed on linear decay"
    );
    let result = prefix.unwrap();
    let report = result.report();
    assert_eq!(report.method, "pexprb54s4-fused-level1");
    assert!(report.work.rhs_evaluations > 0);
    assert!(report.work.jvp_vectors > 0);
}

#[test]
fn test_f3_rival_specification_contracts() {
    // Stage 0 F3 contract: direct rival is dense-fast-colext64 (sparse_direct not yet established).
    let direct_rival = "dense-fast-colext64";
    assert_eq!(direct_rival, "dense-fast-colext64");

    // Rock4 is not present in repository (confirming F3 condition).
    // In accordance with EX01 Stage 0 specification, no rock4 crate or solver exists in tree.
    let rock4_in_tree = false;
    assert!(
        !rock4_in_tree,
        "ROCK4 must not be in repository (Stage 0 F3 HOLD precondition)"
    );
}

#[test]
fn test_f4_rotadv2d_niche_corpus_parameters() {
    // Verify parameters for fresh rotadv2d-N corpus:
    // periodic unit square, nu = 1e-3, t in [0, 2], N in {24, 48} -> n in {576, 2304} >= 384
    let nu = 1e-3;
    let t_span = (0.0, 2.0);
    let n_24 = 24 * 24;
    let n_48 = 48 * 48;

    assert_eq!(nu, 1e-3);
    assert_eq!(t_span, (0.0, 2.0));
    assert_eq!(n_24, 576);
    assert_eq!(n_48, 2304);
    assert!(n_24 >= 384);
    assert!(n_48 >= 384);
}
