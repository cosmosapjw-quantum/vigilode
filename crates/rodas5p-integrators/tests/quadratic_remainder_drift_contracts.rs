use rodas5p_integrators::pexprb54s4_quadratic_remainder_drift;

fn assert_bits_equal(a: Option<f64>, b: Option<f64>) {
    assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
}

#[test]
fn quadratic_leading_term_cancels_and_relative_drift_is_zero() {
    let c2 = 0.25_f64;
    let c3 = 0.5_f64;
    let c4 = 0.9_f64;
    let q = [2.0, -3.0, 0.5];
    let d2: Vec<_> = q.iter().map(|v| c2 * c2 * v).collect();
    let d3: Vec<_> = q.iter().map(|v| c3 * c3 * v).collect();
    let d4: Vec<_> = q.iter().map(|v| c4 * c4 * v).collect();
    let y = [1.0, -2.0, 0.25];
    let u2 = [1.01, -1.98, 0.26];
    let u3 = [1.02, -1.96, 0.27];
    let u4 = [1.04, -1.92, 0.29];

    let drift =
        pexprb54s4_quadratic_remainder_drift(&y, &u2, &u3, &u4, &d2, &d3, &d4, 0.1, 3, 1e-8, 1e-6)
            .unwrap();

    assert_eq!(drift.zeta23.map(f64::to_bits), Some(0.0f64.to_bits()));
    assert_eq!(drift.zeta34.map(f64::to_bits), Some(0.0f64.to_bits()));
    assert_eq!(
        drift.relative_drift.map(f64::to_bits),
        Some(0.0f64.to_bits())
    );
}

#[test]
fn drift_is_permutation_and_common_scaling_invariant() {
    let y = [1.0, -2.0, 0.5];
    let u2 = [1.1, -1.9, 0.55];
    let u3 = [1.2, -1.7, 0.65];
    let u4 = [1.4, -1.4, 0.8];
    let d2 = [0.2, -0.1, 0.05];
    let d3 = [0.9, -0.45, 0.17];
    let d4 = [3.2, -1.1, 0.71];
    let base =
        pexprb54s4_quadratic_remainder_drift(&y, &u2, &u3, &u4, &d2, &d3, &d4, 0.04, 3, 1e-7, 2e-5)
            .unwrap();

    let p = [2usize, 0, 1];
    let permute = |v: &[f64; 3]| [v[p[0]], v[p[1]], v[p[2]]];
    let py = permute(&y);
    let pu2 = permute(&u2);
    let pu3 = permute(&u3);
    let pu4 = permute(&u4);
    let pd2 = permute(&d2);
    let pd3 = permute(&d3);
    let pd4 = permute(&d4);
    let permuted = pexprb54s4_quadratic_remainder_drift(
        &py, &pu2, &pu3, &pu4, &pd2, &pd3, &pd4, 0.04, 3, 1e-7, 2e-5,
    )
    .unwrap();
    let perm_tol = 2e-12;
    assert!((base.zeta23.unwrap() - permuted.zeta23.unwrap()).abs() <= perm_tol);
    assert!((base.zeta34.unwrap() - permuted.zeta34.unwrap()).abs() <= perm_tol);
    assert!((base.relative_drift.unwrap() - permuted.relative_drift.unwrap()).abs() <= perm_tol);

    let lambda = 7.0;
    let scale = |v: &[f64; 3]| [lambda * v[0], lambda * v[1], lambda * v[2]];
    let scaled = pexprb54s4_quadratic_remainder_drift(
        &scale(&y),
        &scale(&u2),
        &scale(&u3),
        &scale(&u4),
        &scale(&d2),
        &scale(&d3),
        &scale(&d4),
        0.04,
        3,
        lambda * 1e-7,
        2e-5,
    )
    .unwrap();
    let tol = 2e-12;
    assert!((base.zeta23.unwrap() - scaled.zeta23.unwrap()).abs() <= tol);
    assert!((base.zeta34.unwrap() - scaled.zeta34.unwrap()).abs() <= tol);
    assert!((base.relative_drift.unwrap() - scaled.relative_drift.unwrap()).abs() <= tol);
}

#[test]
fn physical_prefix_excludes_clock_tail_and_invalid_inputs_fail_closed() {
    let y = [1.0, 2.0, 1e100];
    let u2 = [1.1, 2.1, -1e100];
    let u3 = [1.2, 2.2, 5e99];
    let u4 = [1.3, 2.3, -5e99];
    let d2 = [0.2, -0.1, 1e200];
    let d3 = [0.9, -0.4, -1e200];
    let d4 = [3.0, -1.2, 1e200];
    let full =
        pexprb54s4_quadratic_remainder_drift(&y, &u2, &u3, &u4, &d2, &d3, &d4, 0.03, 2, 1e-8, 1e-6)
            .unwrap();
    let trimmed = pexprb54s4_quadratic_remainder_drift(
        &y[..2],
        &u2[..2],
        &u3[..2],
        &u4[..2],
        &d2[..2],
        &d3[..2],
        &d4[..2],
        0.03,
        2,
        1e-8,
        1e-6,
    )
    .unwrap();
    assert_eq!(full.excluded_trailing_components, 1);
    assert_bits_equal(full.zeta23, trimmed.zeta23);
    assert_bits_equal(full.zeta34, trimmed.zeta34);
    assert_bits_equal(full.relative_drift, trimmed.relative_drift);

    let mut bad = d4;
    bad[0] = f64::NAN;
    let nonfinite = pexprb54s4_quadratic_remainder_drift(
        &y, &u2, &u3, &u4, &d2, &d3, &bad, 0.03, 2, 1e-8, 1e-6,
    )
    .unwrap();
    assert!(nonfinite.zeta34.is_none());
    assert!(nonfinite.relative_drift.is_none());

    assert!(
        pexprb54s4_quadratic_remainder_drift(&y, &u2, &u3, &u4, &d2, &d3, &d4, 0.03, 2, 0.0, 1e-6,)
            .is_err()
    );
}

#[test]
fn level2_report_carries_drift_without_changing_cumulative_work() {
    use rodas5p_integrators::{
        FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem, ParallelExecution,
        pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry,
        pexprb54s4_level2_prefix_resume_level1,
    };
    use std::sync::Arc;

    let problem = OdeProblem::new(
        "quadratic-drift-report",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = 2.0 * y[0] * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let config = FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 8,
        dimension_increment: 1,
        relative_tolerance: 1e-11,
        absolute_tolerance: 1e-14,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 8,
    };
    let level1 = pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
        &problem,
        0.0,
        &[1.0],
        0.04,
        config,
        1,
        1e-10,
        1e-8,
    )
    .unwrap();
    let level1_work = level1.report().work;
    let level2 =
        pexprb54s4_level2_prefix_resume_level1(level1, &ParallelExecution::sequential()).unwrap();
    let report = level2.report();
    let drift = report.quadratic_remainder_drift.as_ref().unwrap();
    assert!(drift.zeta23.is_some());
    assert!(drift.zeta34.is_some());
    assert!(drift.relative_drift.is_some());
    assert!(report.cumulative_work.jvp_vectors >= level1_work.jvp_vectors);
    assert_eq!(report.cumulative_work.jacobian_builds, 0);
    assert_eq!(report.cumulative_work.direct_factorizations, 0);
}
