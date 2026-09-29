//! Scale behaviour of the fused phi combination (audit F-042, F-043).

use std::sync::Arc;

use rodas5p_core::{
    DenseMatrix, DenseOperator, LinearOperator, WorkCounters, dense_fused_phi_action,
};
use rodas5p_integrators::{
    FusedPhiKrylovConfig, FusedPhiTerm, fused_phi_action, fused_phi_linear_combination,
};

/// phi_k(z) by its Taylor series; exact to rounding for |z| <= 0.5.
fn phi_series(z: f64, k: usize) -> f64 {
    let mut term = 1.0;
    for j in 1..=k {
        term /= j as f64;
    }
    let mut sum = term;
    for j in 1..40 {
        term *= z / (k + j) as f64;
        sum += term;
    }
    sum
}

const DIAGONAL: [f64; 2] = [-1.0, -3.0];

fn operator() -> Arc<dyn LinearOperator> {
    Arc::new(
        DenseOperator::new(
            DenseMatrix::from_rows(&[&[DIAGONAL[0], 0.0], &[0.0, DIAGONAL[1]]]).unwrap(),
        )
        .unwrap(),
    )
}

/// 1-D diffusion, n = 64, spectrum in (-4 k, 0): Krylov stops well before
/// the full space, so the convergence threshold decides the result.
fn diffusion(k: f64) -> (Arc<dyn LinearOperator>, DenseMatrix) {
    let n = 64;
    let mut matrix = DenseMatrix::zeros(n, n);
    for i in 0..n {
        matrix[(i, i)] = -2.0 * k;
        if i > 0 {
            matrix[(i, i - 1)] = k;
        }
        if i + 1 < n {
            matrix[(i, i + 1)] = k;
        }
    }
    (
        Arc::new(DenseOperator::new(matrix.clone()).unwrap()),
        matrix,
    )
}

fn smooth(n: usize, phase: f64) -> Vec<f64> {
    (0..n)
        .map(|i| ((i as f64 + 1.0) * 0.1 + phase).sin())
        .collect()
}

fn tight() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        relative_tolerance: 1.0e-14,
        absolute_tolerance: 0.0,
        ..FusedPhiKrylovConfig::default()
    }
}

#[test]
fn fused_combination_matches_exact_phi_as_the_step_shrinks() {
    // sum_k c_k phi_k(h A) v_k, the pexprb54s4 endpoint shape (audit F-042):
    // the combination divides each v_k by h^k before the augmented Krylov.
    let v = [[1.0, 2.0], [0.5, -1.0], [2.0, 0.25], [-1.0, 1.0]];
    for h in [1.0e-2_f64, 1.0e-3, 1.0e-4] {
        let terms = (0..4)
            .map(|k| FusedPhiTerm {
                coefficient: 1.0,
                phi_index: k + 1,
                vector: &v[k],
            })
            .collect::<Vec<_>>();
        let report = fused_phi_linear_combination(
            operator(),
            h,
            &terms,
            tight(),
            &mut WorkCounters::default(),
        )
        .unwrap();
        assert!(report.converged);
        let mut worst = 0.0_f64;
        for i in 0..2 {
            let z = h * DIAGONAL[i];
            let exact: f64 = (0..4).map(|k| phi_series(z, k + 1) * v[k][i]).sum();
            worst = worst.max(((report.value[i] - exact) / exact).abs());
        }
        assert!(worst <= 1.0e-13, "h {h:e}: relative error {worst:e}");
    }
}

#[test]
fn fused_action_is_invariant_under_the_physical_scale_of_its_inputs() {
    // Audit F-043: the threshold was relative to the augmented start norm
    // sqrt(||b0||^2 + 1), so small inputs over-converged and large ones
    // under-converged. A linear action must not depend on that scale.
    let base = [vec![1.0, -2.0], vec![0.5, 0.25], vec![-1.0, 3.0]];
    let config = FusedPhiKrylovConfig {
        relative_tolerance: 1.0e-8,
        absolute_tolerance: 0.0,
        minimum_dimension: 1,
        dimension_increment: 1,
        ..FusedPhiKrylovConfig::default()
    };
    let run = |c: f64| {
        let vectors = base
            .iter()
            .map(|v| v.iter().map(|x| c * x).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        fused_phi_action(
            operator(),
            0.3,
            &vectors,
            config,
            &mut WorkCounters::default(),
        )
        .unwrap()
    };
    let reference = run(1.0);
    for c in [1.0 / 1024.0, 1024.0] {
        let scaled = run(c);
        assert_eq!(
            scaled.maximum_krylov_dimension, reference.maximum_krylov_dimension,
            "scale {c:e}: Krylov dimension"
        );
        assert_eq!(scaled.substeps, reference.substeps, "scale {c:e}: substeps");
        for (a, b) in scaled.value.iter().zip(&reference.value) {
            assert!(
                (a / c - b).abs() <= 1.0e-12 * b.abs(),
                "scale {c:e}: {a:e} vs {b:e}"
            );
        }
    }
}

#[test]
fn fused_combination_on_diffusion_matches_the_dense_oracle_as_the_step_shrinks() {
    let (operator, matrix) = diffusion(50.0);
    let v = (0..4).map(|k| smooth(64, k as f64)).collect::<Vec<_>>();
    let config = FusedPhiKrylovConfig {
        relative_tolerance: 1.0e-12,
        absolute_tolerance: 0.0,
        maximum_dimension: 48,
        ..FusedPhiKrylovConfig::default()
    };
    for h in [1.0e-2_f64, 1.0e-3, 1.0e-4] {
        let terms = (0..4)
            .map(|k| FusedPhiTerm {
                coefficient: 1.0,
                phi_index: k + 1,
                vector: &v[k],
            })
            .collect::<Vec<_>>();
        let report = fused_phi_linear_combination(
            operator.clone(),
            h,
            &terms,
            config,
            &mut WorkCounters::default(),
        )
        .unwrap();
        // Oracle: the same combination through one dense augmented
        // exponential, whose accuracy the core contract pins (F-042).
        let mut vectors = vec![vec![0.0; 64]];
        for (k, vector) in v.iter().enumerate() {
            vectors.push(vector.iter().map(|x| x / h.powi(k as i32 + 1)).collect());
        }
        let oracle = dense_fused_phi_action(&matrix, h, &vectors).unwrap();
        let error = report
            .value
            .iter()
            .zip(&oracle)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        let norm = oracle.iter().map(|b| b * b).sum::<f64>().sqrt();
        assert!(report.converged, "h {h:e}");
        assert!(
            error <= 1.0e-10 * norm,
            "h {h:e}: relative error {:e}",
            error / norm
        );
    }
}

#[test]
fn diffusion_action_is_invariant_under_the_physical_scale_of_its_inputs() {
    let (operator, _) = diffusion(50.0);
    let base = (0..3).map(|k| smooth(64, k as f64)).collect::<Vec<_>>();
    let config = FusedPhiKrylovConfig {
        relative_tolerance: 1.0e-8,
        absolute_tolerance: 0.0,
        maximum_dimension: 48,
        ..FusedPhiKrylovConfig::default()
    };
    let run = |c: f64| {
        let vectors = base
            .iter()
            .map(|v| v.iter().map(|x| c * x).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        fused_phi_action(
            operator.clone(),
            0.01,
            &vectors,
            config,
            &mut WorkCounters::default(),
        )
        .unwrap()
    };
    let reference = run(1.0);
    for c in [1.0 / 1024.0, 1024.0] {
        let scaled = run(c);
        assert_eq!(
            scaled.maximum_krylov_dimension, reference.maximum_krylov_dimension,
            "scale {c:e}: Krylov dimension"
        );
        assert_eq!(scaled.substeps, reference.substeps, "scale {c:e}: substeps");
        for (a, b) in scaled.value.iter().zip(&reference.value) {
            assert!(
                (a / c - b).abs() <= 1.0e-14 * b.abs().max(f64::MIN_POSITIVE),
                "scale {c:e}: {a:e} vs {b:e}"
            );
        }
    }
}
