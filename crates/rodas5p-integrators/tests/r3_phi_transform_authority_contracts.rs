//! Weighting loss binds convergence, and flagged dense references are not
//! precision oracles (re-audit R3 of 2026-10-01: R3-ARITH-01, ARITH-03).
//!
//! A = [[0, 1e308], [0, 0]] (A^2 = 0), h = +-1e-8, b0 = (1e-300, 0),
//! b2 = (0, 1e-310): h^2 b2 underflows to 0, and the exact output's first
//! component is b0 + h^3 A12 b2 / 6 = 1.6666666666666617e-27, while the
//! fused and prefix actions reported `converged` with 1e-300.

use std::sync::Arc;

use rodas5p_core::{
    DenseMatrix, DenseOperator, WorkCounters, dense_fused_phi_action,
    dense_fused_phi_action_report, dense_phi_combination_report,
};
use rodas5p_integrators::{
    FusedPhiKrylovConfig, PhiTransformStatus, compare_fused_phi_to_dense_reference,
    fused_phi_action, fused_phi_action_incremental,
};

fn witness() -> (DenseMatrix, Vec<Vec<f64>>) {
    (
        DenseMatrix::from_rows(&[&[0.0, 1.0e308], &[0.0, 0.0]]).unwrap(),
        vec![vec![1.0e-300, 0.0], vec![0.0, 0.0], vec![0.0, 1.0e-310]],
    )
}

fn config() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        relative_tolerance: 1.0e-12,
        absolute_tolerance: 0.0,
        maximum_substeps: 1,
        ..FusedPhiKrylovConfig::default()
    }
}

#[test]
fn partial_weight_loss_is_never_original_target_convergence() {
    let (matrix, vectors) = witness();
    for h in [1.0e-8, -1.0e-8] {
        let operator = Arc::new(DenseOperator::new(matrix.clone()).unwrap());
        let mut counters = WorkCounters::default();
        let fused = fused_phi_action(operator.clone(), h, &vectors, config(), &mut counters);
        let prefix = fused_phi_action_incremental(operator, h, &vectors, config(), &mut counters);
        for (label, report) in [("fused", fused), ("prefix", prefix)] {
            match report {
                Ok(report) => {
                    assert!(
                        !report.converged,
                        "{label} h={h}: converged after weight loss"
                    );
                    assert_eq!(
                        report.transform_status,
                        PhiTransformStatus::TransformErrorUnbounded { lost: 1 },
                        "{label} h={h}"
                    );
                }
                Err(error) => panic!("{label} h={h}: {error}"),
            }
        }
        assert_eq!(counters.phi_weight_underflows, 2);
        let error = dense_fused_phi_action(&matrix, h, &vectors).unwrap_err();
        assert!(
            error.to_string().contains("TRANSFORM_ERROR_UNBOUNDED"),
            "{error}"
        );
        let report = dense_fused_phi_action_report(&matrix, h, &vectors).unwrap();
        assert_eq!(report.weight_underflows, 1);
        assert!(!report.is_reference_authoritative());
    }
}

#[test]
fn the_zero_operator_control_rejects_conservatively() {
    // A = 0: the lost input would contribute h^2 b2 / 2, itself below the
    // subnormal range; no bound is propagated, so the action is rejected.
    let matrix = DenseMatrix::from_rows(&[&[0.0, 0.0], &[0.0, 0.0]]).unwrap();
    let (_, vectors) = witness();
    let operator = Arc::new(DenseOperator::new(matrix).unwrap());
    let mut counters = WorkCounters::default();
    let report = fused_phi_action(operator, 1.0e-8, &vectors, config(), &mut counters).unwrap();
    assert!(!report.converged);
    assert!(!report.transform_status.is_exact());
}

#[test]
fn a_flagged_dense_reference_is_not_evaluated_by_the_gate_consumer() {
    // diag(700, -700) with w0 = (1e-300, 1e300): the exact first component is
    // about 10142.32; the dense value is 0 and both amplitude flags are set.
    let matrix = DenseMatrix::from_rows(&[&[700.0, 0.0], &[0.0, -700.0]]).unwrap();
    let reference = dense_phi_combination_report(&matrix, 1.0, &[vec![1.0e-300, 1.0e300]]).unwrap();
    assert!(reference.mixed_range);
    assert!(!reference.is_reference_authoritative());
    let operator = Arc::new(DenseOperator::new(matrix).unwrap());
    let mut counters = WorkCounters::default();
    let fused = fused_phi_action(
        operator,
        1.0,
        &[vec![1.0e-300, 1.0e300]],
        FusedPhiKrylovConfig::default(),
        &mut counters,
    );
    let row = compare_fused_phi_to_dense_reference(&fused, true, &reference);
    assert!(!row.completed);
    assert_eq!(row.relative_error_vs_dense, None);
    assert!(row.failure.unwrap().contains("not evaluated"));
    assert!(row.reference.mixed_range);

    // A homogeneous scalar amplitude sweep keeps both accuracy and
    // authority: A = -1, h = 0.1, w1 = c, exact c phi1(-0.1).
    let scalar = DenseMatrix::from_rows(&[&[-1.0]]).unwrap();
    let phi1 = (1.0 - (-0.1_f64).exp()) / 0.1;
    for exponent in [-200, -60, 0, 60, 300] {
        let c = 10.0_f64.powi(exponent);
        let report = dense_phi_combination_report(&scalar, 0.1, &[vec![0.0], vec![c]]).unwrap();
        assert!(report.is_reference_authoritative(), "c = {c:e}");
        assert!(
            ((report.value[0] / c) / phi1 - 1.0).abs() <= 1.0e-12,
            "c = {c:e}"
        );
        let operator = Arc::new(DenseOperator::new(scalar.clone()).unwrap());
        let fused = fused_phi_action(
            operator,
            0.1,
            &[vec![0.0], vec![c / 0.1]],
            FusedPhiKrylovConfig::default(),
            &mut counters,
        );
        let row = compare_fused_phi_to_dense_reference(&fused, true, &report);
        assert!(row.completed, "c = {c:e}: {:?}", row.failure);
        assert!(row.reference.authoritative);
        assert!(row.relative_error_vs_dense.unwrap() <= 1.0e-10, "c = {c:e}");
    }
}

#[test]
fn a_weight_rounded_into_the_subnormal_range_is_a_loss() {
    // h = 1e-8, b2 = 3e-308: h^2 b2 = 3e-324 rounds to 5e-324 (65% error)
    // and dominates the first output component (independent review).
    let matrix = DenseMatrix::from_rows(&[&[0.0, 1.0e308], &[0.0, 0.0]]).unwrap();
    let vectors = vec![vec![1.0e-300, 0.0], vec![0.0, 0.0], vec![0.0, 3.0e-308]];
    let operator = Arc::new(DenseOperator::new(matrix).unwrap());
    let mut counters = WorkCounters::default();
    let report = fused_phi_action(operator, 1.0e-8, &vectors, config(), &mut counters).unwrap();
    assert!(!report.converged);
    assert!(
        report
            .substep_reports
            .iter()
            .all(|substep| !substep.converged)
    );
    assert_eq!(
        report.transform_status,
        PhiTransformStatus::TransformErrorUnbounded { lost: 1 }
    );
}

#[test]
fn a_non_authoritative_reference_holds_the_g3_gate() {
    use rodas5p_integrators::{ComparativeReading, G3FusedAdaptiveSummary, g3_gate_status};
    let mut summary = G3FusedAdaptiveSummary {
        phi_rows: 1,
        phi_completed: 1,
        adaptive_rows: 1,
        adaptive_successes: 1,
        explicit_jacobian_builds_in_primary: 0,
        direct_factorizations_in_primary: 0,
        newton_iterations_in_primary: 0,
        legacy_to_fused_phi_action_ratio: 3.0,
        median_fused_phi_wall_speedup: None,
        maximum_fresh_jvp_half_disagreement: 0.0,
        maximum_fresh_jvp_error_vs_supplied: 0.0,
        comparative_reading: ComparativeReading::for_participants(std::iter::empty()),
        phi_reference_not_evaluated: 0,
    };
    assert_eq!(g3_gate_status(&summary), "pass");
    summary.phi_reference_not_evaluated = 1;
    assert_eq!(g3_gate_status(&summary), "hold");
}
