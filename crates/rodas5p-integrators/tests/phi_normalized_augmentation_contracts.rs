//! Time-normalized phi augmentation (audit 2026-09-30, PHI-P1, PHI-P2,
//! PHI-P3).
//!
//! The scalar physical operator A = -1 has nothing stiff, nonnormal or
//! complex about it. For u(h) = sum_k v_k phi_k(-h) the old augmentation
//! (b_k = v_k / h^k next to hA) converged at Krylov dimension 1 with a
//! relative error of 3.05e-3 at h = 1e-4, 1e-8 and 1e-12 (residual estimate
//! 5e-14 and below), and the dense Pade oracle returned 0 for h <= 1e-14.

use std::sync::Arc;

use rodas5p_core::{
    ClosureOperator, DenseMatrix, LinearOperator, WorkCounters, dense_fused_phi_action,
    dense_phi_combination,
};
use rodas5p_integrators::{
    ExponentialKrylovConfig, FusedPhiKrylovConfig, FusedPhiPrefixSession, FusedPhiTerm,
    fused_phi_action, fused_phi_linear_combination, krylov_phi_action,
};

/// phi_k(z) from its series; 80 terms are exact to roundoff for |z| <= 0.1.
fn phi(z: f64, k: usize) -> f64 {
    let mut term = (1..=k).fold(1.0, |acc, j| acc / j as f64);
    let mut sum = term;
    for j in 1..80 {
        term *= z / (k + j) as f64;
        sum += term;
    }
    sum
}

const V: [f64; 5] = [1.0, 0.5, 2.0, -1.0, 0.75];

fn exact(h: f64, with_b0: bool) -> f64 {
    (0..=4)
        .map(|k| {
            if k == 0 && !with_b0 {
                0.0
            } else {
                V[k] * phi(-h, k)
            }
        })
        .sum()
}

fn minus_one() -> Arc<dyn LinearOperator> {
    Arc::new(ClosureOperator::new(1, |x, y| {
        y[0] = -x[0];
        Ok(())
    }))
}

fn config() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        relative_tolerance: 1.0e-12,
        absolute_tolerance: 1.0e-14,
        ..FusedPhiKrylovConfig::default()
    }
}

const STEPS: [f64; 9] = [
    1.0e-1, 1.0e-4, 1.0e-8, 1.0e-12, 1.0e-14, 1.0e-16, 1.0e-20, 1.0e-40, 1.0e-70,
];

#[test]
fn fused_krylov_combination_is_accurate_for_every_step() {
    for with_b0 in [false, true] {
        let first = usize::from(!with_b0);
        let terms = (first..=4)
            .map(|k| FusedPhiTerm {
                coefficient: V[k],
                phi_index: k,
                vector: &[1.0],
            })
            .collect::<Vec<_>>();
        for h in STEPS {
            let report = fused_phi_linear_combination(
                minus_one(),
                h,
                &terms,
                config(),
                &mut WorkCounters::default(),
            )
            .unwrap();
            let expected = exact(h, with_b0);
            let relative = (report.value[0] / expected - 1.0).abs();
            assert!(report.converged, "h = {h:e}");
            assert!(
                relative <= 1.0e-12,
                "b0 {with_b0}, h = {h:e}: {} vs {expected} (relative {relative:e}, dimension {})",
                report.value[0],
                report.maximum_krylov_dimension
            );
        }
    }
}

#[test]
fn scaled_convention_and_prefix_session_agree_with_the_series() {
    // b_k = v_k / h^k in the scaled convention b0 + sum h^k phi_k(hA) b_k.
    for h in [1.0e-1_f64, 1.0e-4, 1.0e-8, 1.0e-12] {
        let vectors = (0..=4)
            .map(|k| vec![V[k] / h.powi(k as i32)])
            .collect::<Vec<_>>();
        let expected = exact(h, true);
        let report = fused_phi_action(
            minus_one(),
            h,
            &vectors,
            config(),
            &mut WorkCounters::default(),
        )
        .unwrap();
        assert!(
            (report.value[0] / expected - 1.0).abs() <= 1.0e-12,
            "fused h = {h:e}: {}",
            report.value[0]
        );
        let single = FusedPhiKrylovConfig {
            maximum_substeps: 1,
            ..config()
        };
        let session = FusedPhiPrefixSession::begin(
            minus_one(),
            h,
            &vectors,
            single,
            0,
            &mut WorkCounters::default(),
        )
        .unwrap()
        .finish(&mut WorkCounters::default())
        .unwrap();
        assert!(
            (session.value[0] / expected - 1.0).abs() <= 1.0e-12,
            "prefix h = {h:e}: {}",
            session.value[0]
        );
    }
}

#[test]
fn dense_oracle_does_not_vanish_at_small_steps() {
    let matrix = DenseMatrix::from_rows(&[&[-1.0]]).unwrap();
    for h in STEPS {
        let expected = exact(h, true);
        let weighted = V.iter().map(|value| vec![*value]).collect::<Vec<_>>();
        let combination = dense_phi_combination(&matrix, h, &weighted).unwrap();
        assert!(
            (combination[0] / expected - 1.0).abs() <= 1.0e-14,
            "combination h = {h:e}: {} vs {expected}",
            combination[0]
        );
        // The scaled convention stays representable down to h = 1e-70
        // (v_k / h^4 <= 1e280).
        let vectors = (0..=4)
            .map(|k| vec![V[k] / h.powi(k as i32)])
            .collect::<Vec<_>>();
        let scaled = dense_fused_phi_action(&matrix, h, &vectors).unwrap();
        assert!(
            (scaled[0] / expected - 1.0).abs() <= 1.0e-14,
            "scaled h = {h:e}: {} vs {expected}",
            scaled[0]
        );
    }
}

#[test]
fn zero_and_negative_scales_are_the_continuous_combination() {
    let matrix = DenseMatrix::from_rows(&[&[-1.0]]).unwrap();
    let weighted = V.iter().map(|value| vec![*value]).collect::<Vec<_>>();
    // h = 0: sum v_k / k!.
    let at_zero = dense_phi_combination(&matrix, 0.0, &weighted).unwrap()[0];
    let limit = 1.0 + 0.5 + 2.0 / 2.0 - 1.0 / 6.0 + 0.75 / 24.0;
    assert!((at_zero - limit).abs() <= 1.0e-15, "{at_zero} vs {limit}");
    let terms = (0..=4)
        .map(|k| FusedPhiTerm {
            coefficient: V[k],
            phi_index: k,
            vector: &[1.0],
        })
        .collect::<Vec<_>>();
    let krylov = fused_phi_linear_combination(
        minus_one(),
        0.0,
        &terms,
        config(),
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!((krylov.value[0] - limit).abs() <= 1.0e-14);
    // Negative scale: phi_k(+h) for A = -1.
    for h in [1.0e-1, 1.0e-6] {
        let expected = (0..=4).map(|k| V[k] * phi(h, k)).sum::<f64>();
        let krylov = fused_phi_linear_combination(
            minus_one(),
            -h,
            &terms,
            config(),
            &mut WorkCounters::default(),
        )
        .unwrap();
        assert!(
            (krylov.value[0] / expected - 1.0).abs() <= 1.0e-12,
            "h = -{h:e}: {}",
            krylov.value[0]
        );
    }
}

#[test]
fn unfused_happy_breakdown_below_the_minimum_dimension_is_projected() {
    // A = -I_8, v = e1: the first Arnoldi vector spans an invariant
    // subspace. Before the fix: value 0, dimension 0, unconverged.
    let operator = Arc::new(ClosureOperator::new(8, |x, y| {
        for (out, value) in y.iter_mut().zip(x) {
            *out = -value;
        }
        Ok(())
    })) as Arc<dyn LinearOperator>;
    let mut v = vec![0.0; 8];
    v[0] = 1.0;
    let report = krylov_phi_action(
        operator,
        0.1,
        1,
        &v,
        ExponentialKrylovConfig::default(),
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(report.converged);
    assert!(report.happy_breakdown);
    assert_eq!(report.krylov_dimension, 1);
    assert!((report.value[0] - phi(-0.1, 1)).abs() <= 1.0e-15);
    assert!(report.value[1..].iter().all(|value| *value == 0.0));
}
