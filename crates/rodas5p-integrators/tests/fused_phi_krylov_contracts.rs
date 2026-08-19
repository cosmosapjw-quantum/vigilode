use rodas5p_core::{DenseMatrix, DenseOperator, WorkCounters, dense_fused_phi_action, safe_l2};
use rodas5p_integrators::{FusedOrthogonalization, FusedPhiKrylovConfig, fused_phi_action};
use std::sync::Arc;

fn config(orthogonalization: FusedOrthogonalization, dimension: usize) -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: dimension,
        dimension_increment: 1,
        relative_tolerance: 1.0e-12,
        absolute_tolerance: 1.0e-14,
        orthogonalization,
        maximum_substeps: 4,
    }
}

#[test]
fn fused_matrix_free_action_matches_dense_oracle() {
    let matrix = DenseMatrix::from_vec_rows(vec![
        vec![-4.0, 7.0, 0.0, 0.0],
        vec![0.0, -5.0, 6.0, 0.0],
        vec![0.0, 0.0, -6.0, 5.0],
        vec![0.0, 0.0, 0.0, -7.0],
    ])
    .unwrap();
    let vectors = vec![
        vec![0.4, -0.1, 0.3, 0.2],
        vec![1.0, -0.25, 0.5, -0.75],
        vec![-0.2, 0.8, 0.1, -0.4],
        vec![0.3, -0.2, 0.6, 0.9],
    ];
    let scale = 0.17;
    let expected = dense_fused_phi_action(&matrix, scale, &vectors).unwrap();

    let mut full_counters = WorkCounters::default();
    let full = fused_phi_action(
        Arc::new(DenseOperator::new(matrix.clone()).unwrap()),
        scale,
        &vectors,
        config(
            FusedOrthogonalization::FullMgs,
            matrix.nrows() + vectors.len() - 1,
        ),
        &mut full_counters,
    )
    .unwrap();
    assert!(full.converged);
    let full_defect = full
        .value
        .iter()
        .zip(&expected)
        .map(|(a, b)| a - b)
        .collect::<Vec<_>>();
    assert!(safe_l2(&full_defect) / safe_l2(&expected).max(1e-300) < 2e-10);

    // IOP is an inexact low-communication path.  It must either satisfy its explicit tolerance or
    // fail closed; the correctness oracle is full MGS, not the dimensionality of the short basis.
    let mut iop_config = config(
        FusedOrthogonalization::Incomplete { length: 2 },
        matrix.nrows() + vectors.len() - 1,
    );
    iop_config.relative_tolerance = 1e-5;
    iop_config.absolute_tolerance = 1e-8;
    iop_config.maximum_substeps = 16;
    let mut iop_counters = WorkCounters::default();
    let iop = fused_phi_action(
        Arc::new(DenseOperator::new(matrix).unwrap()),
        scale,
        &vectors,
        iop_config,
        &mut iop_counters,
    )
    .unwrap();
    if iop.converged {
        let defect = iop
            .value
            .iter()
            .zip(&expected)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>();
        assert!(
            safe_l2(&defect) / safe_l2(&expected).max(1e-300) < 2e-4,
            "iop={iop:?}"
        );
    } else {
        assert!(iop_counters.phi_restarts > 0);
    }
    assert_eq!(full_counters.phi_actions, 1);
    assert_eq!(iop_counters.phi_actions, 1);
}

#[test]
fn zero_fused_action_short_circuits_without_operator_work() {
    let matrix = DenseMatrix::identity(3);
    let vectors = vec![vec![0.0; 3], vec![0.0; 3], vec![0.0; 3]];
    let mut counters = WorkCounters::default();
    let report = fused_phi_action(
        Arc::new(DenseOperator::new(matrix).unwrap()),
        0.3,
        &vectors,
        config(FusedOrthogonalization::FullMgs, 5),
        &mut counters,
    )
    .unwrap();
    assert!(report.converged);
    assert_eq!(report.value, vec![0.0; 3]);
    assert_eq!(counters.jvp_vectors, 0);
    assert_eq!(counters.phi_actions, 1);
}

#[test]
fn fused_convergence_uses_residual_estimate_not_nested_difference() {
    let matrix = DenseMatrix::from_vec_rows(vec![vec![-1.0, 0.0], vec![0.0, -2.0]]).unwrap();
    let vectors = vec![vec![1.0, 1.0]];
    let mut cfg = config(FusedOrthogonalization::FullMgs, 2);
    cfg.minimum_dimension = 1;
    cfg.maximum_dimension = 2;
    cfg.relative_tolerance = 1.0e-2;
    cfg.absolute_tolerance = 1.0e-12;
    let mut counters = WorkCounters::default();
    let report = fused_phi_action(
        Arc::new(DenseOperator::new(matrix).unwrap()),
        1.0e-4,
        &vectors,
        cfg,
        &mut counters,
    )
    .unwrap();
    assert!(report.converged, "report={report:?}");
    let first = &report.substep_reports[0];
    assert_eq!(first.krylov_dimension, 1);
    assert!(first.error_estimate.is_finite());
    assert!(first.nested_difference_estimate.is_infinite());
    assert!(counters.phi_projected_exponentials >= 2);
}

#[test]
fn fused_error_estimate_matches_first_arnoldi_term() {
    let matrix = DenseMatrix::from_vec_rows(vec![vec![-1.0, 0.0], vec![0.0, -2.0]]).unwrap();
    let vectors = vec![vec![1.0, 1.0]];
    let scale = 0.1_f64;
    let mut cfg = config(FusedOrthogonalization::FullMgs, 1);
    cfg.minimum_dimension = 1;
    cfg.maximum_dimension = 1;
    cfg.relative_tolerance = 0.0;
    cfg.absolute_tolerance = 0.0;
    cfg.maximum_substeps = 1;
    let mut counters = WorkCounters::default();
    let report = fused_phi_action(
        Arc::new(DenseOperator::new(matrix).unwrap()),
        scale,
        &vectors,
        cfg,
        &mut counters,
    )
    .unwrap();
    let beta = 2.0_f64.sqrt();
    let h21 = 0.5_f64;
    let z = -1.5_f64 * scale;
    let phi1 = z.exp_m1() / z;
    let expected = scale.abs() * h21 * beta * phi1.abs();
    assert!((report.error_estimate - expected).abs() < 2e-15);
    assert!(
        report.nested_difference_estimate == 0.0 || report.nested_difference_estimate.is_infinite()
    );
}
