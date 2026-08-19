use std::sync::Arc;

use rodas5p_core::{ClosureOperator, LinearOperator, WorkCounters, safe_l2};
use rodas5p_integrators::{
    FusedOrthogonalization, FusedPhiKrylovConfig, FusedPhiPrefixSession,
    fused_phi_action_incremental,
};

fn nonnormal_generator(n: usize) -> Arc<dyn LinearOperator> {
    Arc::new(ClosureOperator::new(n, move |x, y| {
        for i in 0..n {
            let decay = -2.0 - 0.02 * i as f64;
            let upper = if i + 1 < n { 1.25 * x[i + 1] } else { 0.0 };
            let lower = if i > 0 { -0.05 * x[i - 1] } else { 0.0 };
            y[i] = decay * x[i] + upper + lower;
        }
        Ok(())
    }))
}

#[test]
fn fused_phi_prefix_resume_matches_cold_incremental_path() {
    let n = 40;
    let operator = nonnormal_generator(n);
    let vectors = vec![
        vec![0.0; n],
        (0..n).map(|i| (0.13 * (i + 1) as f64).sin()).collect(),
        (0..n)
            .map(|i| 0.1 * (0.07 * (i + 1) as f64).cos())
            .collect(),
        (0..n)
            .map(|i| 0.02 * (0.11 * (i + 1) as f64).sin())
            .collect(),
        (0..n)
            .map(|i| 0.01 * (0.09 * (i + 1) as f64).cos())
            .collect(),
    ];
    let config = FusedPhiKrylovConfig {
        minimum_dimension: 2,
        maximum_dimension: 24,
        dimension_increment: 1,
        relative_tolerance: 1.0e-11,
        absolute_tolerance: 1.0e-13,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 1,
    };

    let mut cold_work = WorkCounters::default();
    let cold =
        fused_phi_action_incremental(operator.clone(), 0.2, &vectors, config, &mut cold_work)
            .expect("cold fused phi");
    assert!(cold.converged);

    let mut resumed_work = WorkCounters::default();
    let session =
        FusedPhiPrefixSession::begin(operator, 0.2, &vectors, config, 2, &mut resumed_work)
            .expect("fused phi prefix");
    let prediction = session.prediction();
    assert_eq!(prediction.prefix_dimension, 2);
    assert!(prediction.predicted_total_dimension >= 2);
    assert!(prediction.predicted_total_dimension <= config.maximum_dimension);
    assert!(prediction.residual_error_estimate.is_finite());

    let resumed = session.finish(&mut resumed_work).expect("resume fused phi");
    assert!(resumed.converged);
    let defect: Vec<f64> = cold
        .value
        .iter()
        .zip(&resumed.value)
        .map(|(a, b)| a - b)
        .collect();
    assert!(safe_l2(&defect) <= 1.0e-12 * safe_l2(&cold.value).max(1.0));
    assert_eq!(
        cold.maximum_krylov_dimension,
        resumed.maximum_krylov_dimension
    );
    assert_eq!(cold_work.jvp_vectors, resumed_work.jvp_vectors);
    assert_eq!(
        cold_work.orthogonalization_inner_products,
        resumed_work.orthogonalization_inner_products
    );
}
