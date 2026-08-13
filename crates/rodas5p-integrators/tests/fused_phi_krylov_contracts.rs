use std::sync::Arc;
use rodas5p_core::{DenseMatrix, DenseOperator, WorkCounters, dense_fused_phi_action, safe_l2};
use rodas5p_integrators::{FusedOrthogonalization, FusedPhiKrylovConfig, fused_phi_action};

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
    ]).unwrap();
    let vectors = vec![
        vec![0.4, -0.1, 0.3, 0.2],
        vec![1.0, -0.25, 0.5, -0.75],
        vec![-0.2, 0.8, 0.1, -0.4],
        vec![0.3, -0.2, 0.6, 0.9],
    ];
    let scale = 0.17;
    let expected = dense_fused_phi_action(&matrix, scale, &vectors).unwrap();
    for orth in [FusedOrthogonalization::FullMgs, FusedOrthogonalization::Incomplete { length: 2 }] {
        let mut counters = WorkCounters::default();
        let report = fused_phi_action(
            Arc::new(DenseOperator::new(matrix.clone()).unwrap()),
            scale,
            &vectors,
            config(orth, matrix.nrows() + vectors.len() - 1),
            &mut counters,
        ).unwrap();
        assert!(report.converged);
        let defect = report.value.iter().zip(&expected).map(|(a,b)| a-b).collect::<Vec<_>>();
        assert!(safe_l2(&defect) / safe_l2(&expected).max(1e-300) < 2e-10, "orth={orth:?} report={report:?}");
        assert_eq!(counters.phi_actions, 1);
        assert!(counters.jvp_vectors > 0);
    }
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
    ).unwrap();
    assert!(report.converged);
    assert_eq!(report.value, vec![0.0; 3]);
    assert_eq!(counters.jvp_vectors, 0);
    assert_eq!(counters.phi_actions, 1);
}
