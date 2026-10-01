//! Scaled-Taylor block-phi prototype (re-audit R4 of 2026-10-01,
//! POLY-DEV-06): exact-input diagonal, nilpotent and nonnormal cases,
//! capability rejection, and an estimate-only status.

use rodas5p_core::{
    DenseMatrix,
    directed::Interval,
    polynomial_action::scalar_phi_enclosure,
    taylor_phi::{
        DomainWitness, OperatorEpoch, TAYLOR_DOMAIN_UNSUPPORTED, TAYLOR_PHI_SCHEMA,
        TaylorPhiStatus, taylor_phi_action,
    },
};

fn matrix(rows: &[&[f64]]) -> DenseMatrix {
    DenseMatrix::from_rows(rows).unwrap()
}

fn vectors() -> [Vec<f64>; 5] {
    [
        vec![1.0, -0.5],
        vec![0.25, 1.0],
        vec![-0.5, 0.125],
        vec![0.0, 0.75],
        vec![0.5, -0.25],
    ]
}

/// `phi_k(z)` midpoints for real `z <= 0`.
fn phi(z: f64) -> [f64; 5] {
    scalar_phi_enclosure(Interval::new(z, z).unwrap())
        .unwrap()
        .map(|e| 0.5 * e.lo + 0.5 * e.hi)
}

fn truncation(status: &TaylorPhiStatus) -> f64 {
    match status {
        TaylorPhiStatus::EstimateOnly {
            truncation_bound, ..
        } => *truncation_bound,
        TaylorPhiStatus::Certified { .. } => panic!("the prototype never certifies"),
    }
}

fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

#[test]
fn diagonal_matches_the_scalar_enclosures() {
    let w = vectors();
    for h in [0.0, 0.01, 0.5, 3.0] {
        let a = matrix(&[&[-1.0, 0.0], &[0.0, -4.0]]);
        let report = taylor_phi_action(&a, h, &w, 1.0e-14).unwrap();
        assert_eq!(report.schema, TAYLOR_PHI_SCHEMA);
        let reference = (0..2)
            .map(|i| {
                let values = phi(h * a[(i, i)]);
                (0..5).map(|k| values[k] * w[k][i]).sum::<f64>()
            })
            .collect::<Vec<_>>();
        let error = distance(&report.fused, &reference);
        assert!(
            error <= truncation(&report.status) + 1.0e-13,
            "h = {h}: {error:e}"
        );
        assert_eq!(report.nonnormality, Some(0.0));
        assert_eq!(
            report.work.augmented_vector_products,
            report.work.scaling_steps * report.work.degree as u64
        );
    }
}

#[test]
fn nilpotent_is_exact_in_two_terms() {
    // phi_k(hN) = I/k! + hN/(k+1)! for N^2 = 0.
    let n = matrix(&[&[0.0, 1.0], &[0.0, 0.0]]);
    let w = vectors();
    let h = 2.0;
    let factorial = [1.0, 1.0, 2.0, 6.0, 24.0, 120.0];
    let mut reference = vec![0.0; 2];
    for k in 0..5 {
        reference[0] += w[k][0] / factorial[k] + h * w[k][1] / factorial[k + 1];
        reference[1] += w[k][1] / factorial[k];
    }
    let report = taylor_phi_action(&n, h, &w, 1.0e-14).unwrap();
    assert!(distance(&report.fused, &reference) <= truncation(&report.status) + 1.0e-13);
    assert!(report.nonnormality.unwrap() > 0.5);
}

#[test]
fn nonnormal_matches_the_divided_difference_and_reports_conditioning() {
    // f([[a, b], [0, d]]) = [[f(a), b (f(a) - f(d)) / (a - d)], [0, f(d)]].
    let (a, b, d, h) = (-1.0, 10.0, -3.0, 0.5);
    let op = matrix(&[&[a, b], &[0.0, d]]);
    let w = vectors();
    let (fa, fd) = (phi(h * a), phi(h * d));
    let mut reference = vec![0.0; 2];
    for k in 0..5 {
        let off = b * (fa[k] - fd[k]) / (a - d);
        reference[0] += fa[k] * w[k][0] + off * w[k][1];
        reference[1] += fd[k] * w[k][1];
    }
    let report = taylor_phi_action(&op, h, &w, 1.0e-14).unwrap();
    let error = distance(&report.fused, &reference);
    assert!(error <= truncation(&report.status) + 1.0e-12, "{error:e}");
    assert!(report.nonnormality.unwrap() > 0.5);
    // The witness is the explicit 1-norm; nothing is inferred.
    let DomainWitness::DenseOneNorm { one_norm_upper } = report.epoch.witness;
    assert!(one_norm_upper >= 13.0);
}

#[test]
fn outside_the_validated_domain_is_a_typed_rejection() {
    let w = vectors();
    let stiff = matrix(&[&[-1000.0, 0.0], &[0.0, -1.0]]);
    let error = taylor_phi_action(&stiff, 1.0, &w, 1.0e-12).unwrap_err();
    assert!(
        error.to_string().contains(TAYLOR_DOMAIN_UNSUPPORTED),
        "{error}"
    );
    let ok = matrix(&[&[-1.0, 0.0], &[0.0, -1.0]]);
    for (h, budget) in [
        (-1.0, 1.0e-12),
        (f64::NAN, 1.0e-12),
        (1.0, 0.0),
        (1.0, -1.0),
    ] {
        assert!(taylor_phi_action(&ok, h, &w, budget).is_err());
    }
    let short = [
        vec![1.0],
        vec![0.0; 2],
        vec![0.0; 2],
        vec![0.0; 2],
        vec![0.0; 2],
    ];
    assert!(taylor_phi_action(&ok, 1.0, &short, 1.0e-12).is_err());
    // A different entry is a different epoch.
    let other = matrix(&[&[-1.0, 0.0], &[0.0, -1.0 - f64::EPSILON]]);
    assert_ne!(
        OperatorEpoch::dense(&ok).unwrap().fingerprint,
        OperatorEpoch::dense(&other).unwrap().fingerprint
    );
}
