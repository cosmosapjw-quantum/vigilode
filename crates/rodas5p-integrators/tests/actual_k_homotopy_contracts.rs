use rodas5p_integrators::{
    ActualKHomotopyConfig, ActualKHomotopyError, ActualKHomotopyWork, CollectiveModel,
    certify_collective_actual_k_homotopy,
};
fn model() -> CollectiveModel {
    CollectiveModel::new(
        vec![-1.0, 1.0],
        vec![0.4, 0.6],
        vec![(0.2, -0.1), (0.3, 0.05)],
        0.7,
        0.4,
        0.0,
        0.1,
    )
    .unwrap()
}
fn config() -> ActualKHomotopyConfig {
    ActualKHomotopyConfig {
        h: 1.0 / 1024.0,
        lambdas: vec![0.0, 0.5, 1.0],
        rounds_per_stage: 10,
        tube_radius: 1e-3,
        stage_output_tolerance: 1e-9,
    }
}
#[test]
fn full_represented_k_retains_leakage_and_closes_common_tube() {
    let source = model();
    let mut work = ActualKHomotopyWork::default();
    let result = certify_collective_actual_k_homotopy(&source, &config(), &mut work).unwrap();
    assert_eq!(result.source_model(), &source);
    assert!(result.leakage_q() < 1.0);
    assert!(result.inverse_residual() < 1.0);
    assert!(result.noncausal_alpha_max() > 0.0 || result.noncausal_linear_max() > 0.0);
    assert!(
        result
            .output_radii()
            .iter()
            .all(|&r| r <= config().stage_output_tolerance)
    );
    assert_eq!(work.retained_alpha_beta_entries, 128);
    assert_eq!(work.candidate_rhs_calls, 240);
}
#[test]
fn direct_and_continued_actual_stage_targets_have_overlapping_outputs() {
    let source = model();
    let mut cfg = config();
    let a =
        certify_collective_actual_k_homotopy(&source, &cfg, &mut ActualKHomotopyWork::default())
            .unwrap();
    cfg.lambdas = vec![1.0];
    let b =
        certify_collective_actual_k_homotopy(&source, &cfg, &mut ActualKHomotopyWork::default())
            .unwrap();
    for (((a, b), ar), br) in a
        .output()
        .iter()
        .zip(b.output())
        .zip(a.output_radii())
        .zip(b.output_radii())
    {
        assert!((a - b).abs() <= ar + br);
    }
}
#[test]
fn actual_stage_budgets_and_target_are_not_relaxed() {
    let source = model();
    let mut cfg = config();
    cfg.lambdas = vec![0.5];
    assert!(matches!(
        certify_collective_actual_k_homotopy(&source, &cfg, &mut ActualKHomotopyWork::default()),
        Err(ActualKHomotopyError::InvalidInput)
    ));
    cfg = config();
    cfg.stage_output_tolerance = 1e-30;
    let mut work = ActualKHomotopyWork::default();
    assert!(matches!(
        certify_collective_actual_k_homotopy(&source, &cfg, &mut work),
        Err(ActualKHomotopyError::OutputBudget { .. })
    ));
    assert!(work.candidate_rhs_calls > 0);
}
