//! Range and amplitude of the phi combinations (re-audit R2 of 2026-09-30,
//! PHI-R1, PHI-R2).
//!
//! Before: with A = 0, h = +-1e-100 and b_4 = 1e300 the weight
//! `h.powi(4) * b_4` underflowed to 0 * 1e300 and the fused and dense
//! results were 0 instead of 1e-100 / 24; with h = 1e100 and b_4 = 1e-300 it
//! overflowed and the call failed. The dense oracle with A = -1, h = 0.1 and
//! w_1 = c returned 0 at c = 1e60 (relative error 0.043 at c = 1e40).

use std::sync::Arc;

use rodas5p_core::{
    ClosureOperator, DenseMatrix, LinearOperator, WorkCounters, dense_fused_phi_action,
    dense_phi_combination, dense_phi_combination_report, times_power, weight_phi_vectors,
};
use rodas5p_integrators::{FusedPhiKrylovConfig, FusedPhiPrefixSession, fused_phi_action};

fn scalar(value: f64) -> Arc<dyn LinearOperator> {
    Arc::new(ClosureOperator::new(1, move |x, y| {
        y[0] = value * x[0];
        Ok(())
    }))
}

fn config() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        relative_tolerance: 1.0e-12,
        absolute_tolerance: 0.0,
        ..FusedPhiKrylovConfig::default()
    }
}

/// b = (0, 0, 0, 0, b4): with A = 0 the combination is h^4 b4 / 4!.
fn b4_only(b4: f64) -> Vec<Vec<f64>> {
    let mut vectors = vec![vec![0.0]; 5];
    vectors[4][0] = b4;
    vectors
}

#[test]
fn representable_weights_survive_the_transform() {
    // Expected values from the audit's exact oracle.
    for (h, b4, expected) in [
        (1.0e-100_f64, 1.0e300_f64, 4.166_666_666_666_667e-102_f64),
        (-1.0e-100, 1.0e300, 4.166_666_666_666_667e-102),
        (1.0e100, 1.0e-300, 4.166_666_666_666_666_5e98),
    ] {
        let dense =
            dense_fused_phi_action(&DenseMatrix::from_rows(&[&[0.0]]).unwrap(), h, &b4_only(b4))
                .unwrap()[0];
        assert!(
            (dense / expected - 1.0).abs() <= 1.0e-12,
            "dense h = {h:e}: {dense:e}"
        );
        let fused = fused_phi_action(
            scalar(0.0),
            h,
            &b4_only(b4),
            config(),
            &mut WorkCounters::default(),
        )
        .unwrap();
        assert!(fused.converged);
        assert!(
            (fused.value[0] / expected - 1.0).abs() <= 1.0e-12,
            "fused h = {h:e}: {:e}",
            fused.value[0]
        );
        let single = FusedPhiKrylovConfig {
            maximum_substeps: 1,
            ..config()
        };
        let prefix = FusedPhiPrefixSession::begin(
            scalar(0.0),
            h,
            &b4_only(b4),
            single,
            0,
            &mut WorkCounters::default(),
        )
        .unwrap()
        .finish(&mut WorkCounters::default())
        .unwrap();
        assert!(
            (prefix.value[0] / expected - 1.0).abs() <= 1.0e-12,
            "prefix h = {h:e}: {:e}",
            prefix.value[0]
        );
    }
}

#[test]
fn the_weight_transform_reports_loss_instead_of_hiding_it() {
    assert_eq!(times_power(1.0e300, -1.0e-100, 4), 1.0e-100);
    assert_eq!(times_power(2.0, -3.0, 3), -54.0);
    assert_eq!(times_power(1.0e-300, 1.0e100, 4), 1.0e100);
    // Above f64::MAX: an error, not an infinite weight.
    assert!(weight_phi_vectors(1.0e100, &[vec![1.0], vec![0.0], vec![1.0e200]]).is_err());
    // Every nonzero input lost below the smallest subnormal: an error, not 0.
    assert!(weight_phi_vectors(1.0e-200, &[vec![0.0], vec![0.0], vec![1.0e-1]]).is_err());
    // A partial loss is counted; the representable weight is kept.
    let (weighted, lost) =
        weight_phi_vectors(1.0e-200, &[vec![1.0], vec![0.0], vec![1.0e-1]]).unwrap();
    assert_eq!(weighted[0][0], 1.0);
    assert_eq!(lost, 1);
    // scale 0 is exact: w_0 kept, w_k = 0 with nothing lost.
    let (weighted, lost) = weight_phi_vectors(0.0, &[vec![2.0], vec![3.0]]).unwrap();
    assert_eq!(weighted, vec![vec![2.0], vec![0.0]]);
    assert_eq!(lost, 0);
}

#[test]
fn the_dense_oracle_is_invariant_to_input_amplitude() {
    let matrix = DenseMatrix::from_rows(&[&[-1.0]]).unwrap();
    // phi_1(-0.1) = (1 - e^-0.1) / 0.1, independent oracle in closed form.
    let phi1 = -(-0.1_f64).exp_m1() / 0.1;
    let mut exponent = -200;
    while exponent <= 300 {
        let c = 10.0_f64.powi(exponent);
        let weighted = vec![vec![0.0], vec![c]];
        let value = dense_phi_combination(&matrix, 0.1, &weighted).unwrap()[0];
        let expected = c * phi1;
        assert!(
            (value / expected - 1.0).abs() <= 1.0e-12,
            "c = 1e{exponent}: {value:e} vs {expected:e}"
        );
        exponent += 20;
    }
}

#[test]
fn mixed_range_and_small_output_are_labelled() {
    let matrix = DenseMatrix::from_rows(&[&[-1.0]]).unwrap();
    let balanced = dense_phi_combination_report(&matrix, 0.1, &[vec![1.0], vec![2.0]]).unwrap();
    assert!(!balanced.mixed_range && !balanced.output_below_input_half_precision);
    let mixed = dense_phi_combination_report(&matrix, 0.1, &[vec![1.0e-30], vec![1.0e30]]).unwrap();
    assert!(mixed.mixed_range);
    // e^{-0.1} w0 + phi1(-0.1) w1 = 0 by construction: cancellation.
    let phi1 = -(-0.1_f64).exp_m1() / 0.1;
    let cancel =
        dense_phi_combination_report(&matrix, 0.1, &[vec![phi1], vec![-(-0.1_f64).exp()]]).unwrap();
    assert!(cancel.output_below_input_half_precision, "{cancel:?}");
    assert!(cancel.value[0].abs() <= 1.0e-15);
}
