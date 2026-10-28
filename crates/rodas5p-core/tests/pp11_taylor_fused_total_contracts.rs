//! Contracts of research node `research/pp11_taylor_fused_total_20261004`
//! (RVJ DAG node PP11): the total-target certificate of the fused phi
//! action. Hand cases only.

use rodas5p_core::DenseMatrix;
use rodas5p_core::directed::Interval;
use rodas5p_core::nonnormal_certificate::{NonnormalExpCertificate, certify_exp_action_auto};
use rodas5p_core::polynomial_action::scalar_phi_enclosure;
use rodas5p_core::taylor_phi::taylor_phi_action;
use rodas5p_core::taylor_phi_total::{
    FUSED_PHI_TOTAL_SCHEMA, FusedPhiCertificate, FusedPhiStatus, FusedPhiTerm,
    certify_fused_phi_total,
};

fn vectors() -> [Vec<f64>; 5] {
    [
        vec![1.0, -0.5],
        vec![0.25, 1.0],
        vec![-0.5, 0.125],
        vec![0.0, 0.75],
        vec![0.5, -0.25],
    ]
}

/// A nonsymmetric 2x2 matrix whose entries times 0.1 round in binary64.
fn nonsymmetric() -> Vec<Vec<f64>> {
    vec![vec![-1.3, 0.7], vec![0.3, -2.9]]
}

fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// `sum_k phi_k(h a_ii) w_k[i]` for diagonal `A`, interval-enclosed.
fn diagonal_reference(diag: &[f64], h: f64, w: &[Vec<f64>; 5]) -> Vec<f64> {
    diag.iter()
        .enumerate()
        .map(|(i, d)| {
            // h d enclosed (exact for the power-of-two h used here).
            let z = h * d;
            let phi = scalar_phi_enclosure(Interval::new(z, z).unwrap()).unwrap();
            (0..5)
                .map(|k| (0.5 * phi[k].lo + 0.5 * phi[k].hi) * w[k][i])
                .sum()
        })
        .collect()
}

/// What a caller requiring the fused target accepts. A
/// [`NonnormalExpCertificate`] does not coerce to it (see the module's
/// `compile_fail` doctest).
fn requires_fused(cert: &FusedPhiCertificate) -> (f64, FusedPhiStatus) {
    (cert.bound(), cert.status())
}

/// What a caller of the exp-only certificate sees: a different type with a
/// different target (`exp(M~) v` for the stored matrix).
fn exp_only_bound(cert: &NonnormalExpCertificate) -> f64 {
    cert.error_upper
}

#[test]
fn power_of_two_h_has_exactly_zero_perturbation() {
    let w = vectors();
    for h in [0.125, 0.5, 2.0] {
        let cert = certify_fused_phi_total(&nonsymmetric(), h, &w, &[0.0, 0.0]).unwrap();
        assert_eq!(cert.schema(), FUSED_PHI_TOTAL_SCHEMA);
        assert_eq!(cert.delta_two_norm(), 0.0, "h = {h}");
        assert_eq!(cert.perturbation(), 0.0, "h = {h}");
        assert_eq!(cert.status(), FusedPhiStatus::Bounded);
    }
}

#[test]
fn non_power_of_two_h_has_positive_perturbation() {
    let w = vectors();
    for h in [0.1, 0.3, 1.0 / 3.0] {
        let cert = certify_fused_phi_total(&nonsymmetric(), h, &w, &[0.0, 0.0]).unwrap();
        assert!(cert.delta_two_norm() > 0.0, "h = {h}");
        assert!(cert.perturbation() > 0.0, "h = {h}");
        assert!(cert.perturbation().is_finite());
        assert!(cert.omega() >= 0.0);
        assert_eq!(cert.status(), FusedPhiStatus::Bounded);
        // The total is at least each reported term.
        assert!(cert.bound() >= cert.perturbation());
        assert!(cert.bound() >= cert.stepped_error());
        assert!(cert.bound() >= cert.candidate_distance());
    }
}

#[test]
fn invalid_inputs_are_refused() {
    let w = vectors();
    let a = nonsymmetric();
    let u = [0.0, 0.0];
    assert!(certify_fused_phi_total(&[], 0.1, &w, &u).is_err());
    assert!(certify_fused_phi_total(&[vec![1.0, 2.0]], 0.1, &w, &u).is_err());
    assert!(certify_fused_phi_total(&[vec![1.0], vec![2.0, 3.0]], 0.1, &w, &u).is_err());
    for h in [-0.1, f64::NAN, f64::INFINITY] {
        assert!(certify_fused_phi_total(&a, h, &w, &u).is_err(), "h = {h}");
    }
    assert!(certify_fused_phi_total(&a, 0.1, &w, &[0.0]).is_err());
    assert!(certify_fused_phi_total(&a, 0.1, &w, &[0.0, f64::NAN]).is_err());
    let mut short = vectors();
    short[3] = vec![1.0];
    assert!(certify_fused_phi_total(&a, 0.1, &short, &u).is_err());
    let mut bad = vectors();
    bad[2][1] = f64::INFINITY;
    assert!(certify_fused_phi_total(&a, 0.1, &bad, &u).is_err());
    let mut nan_a = nonsymmetric();
    nan_a[1][0] = f64::NAN;
    assert!(certify_fused_phi_total(&nan_a, 0.1, &w, &u).is_err());
    // fl(h a) overflows.
    let huge = vec![vec![f64::MAX, 0.0], vec![0.0, -1.0]];
    assert!(certify_fused_phi_total(&huge, 4.0, &w, &u).is_err());
}

#[test]
fn a_wrong_candidate_gets_a_bound_at_least_its_distance() {
    let w = vectors();
    let diag = [-1.0, -4.0];
    let a = vec![vec![diag[0], 0.0], vec![0.0, diag[1]]];
    let h = 0.5;
    let reference = diagonal_reference(&diag, h, &w);
    let first = certify_fused_phi_total(&a, h, &w, &reference).unwrap();
    assert_eq!(first.status(), FusedPhiStatus::Bounded);
    assert!(first.bound() < 1e-13, "{}", first.bound());
    let wrong = vec![reference[0] + 0.75, reference[1] - 0.5];
    let cert = certify_fused_phi_total(&a, h, &w, &wrong).unwrap();
    // The reference midpoints are within 1e-15 of F.
    let true_distance = distance(&wrong, &reference) - 1e-15;
    assert!(
        cert.bound() >= true_distance,
        "{} < {true_distance}",
        cert.bound()
    );
    assert!(cert.candidate_distance() >= distance(&wrong, cert.stepped_top()));
    assert_eq!(cert.dominant(), FusedPhiTerm::CandidateDistance);
    assert_eq!(cert.candidate(), wrong.as_slice());
}

#[test]
fn the_taylor_output_is_certified_against_the_diagonal_reference() {
    let w = vectors();
    let diag = [-0.75, -3.0];
    let a = vec![vec![diag[0], 0.0], vec![0.0, diag[1]]];
    let h = 0.25;
    let dense = DenseMatrix::from_rows(&[&a[0], &a[1]]).unwrap();
    let report = taylor_phi_action(&dense, h, &w, 1e-14).unwrap();
    let cert = certify_fused_phi_total(&a, h, &w, &report.fused).unwrap();
    let reference = diagonal_reference(&diag, h, &w);
    // The reference midpoints are within a few ulps of F.
    assert!(distance(&report.fused, &reference) <= cert.bound() + 1e-15);
    assert!(cert.bound() < 1e-13, "{}", cert.bound());
}

#[test]
fn an_exp_certificate_is_a_different_type_from_a_fused_certificate() {
    let w = vectors();
    let a = nonsymmetric();
    let fused = certify_fused_phi_total(&a, 0.1, &w, &[0.0, 0.0]).unwrap();
    // The exp-only certificate of the augmented stored matrix (a different
    // target: exp(M~) v, without the rounding of hA).
    let (_, exp_only, _, _) = certify_exp_action_auto(&a, &w[0], 0.1).unwrap();
    let (bound, status) = requires_fused(&fused);
    assert_eq!(status, FusedPhiStatus::Bounded);
    assert!(bound.is_finite());
    assert!(exp_only_bound(&exp_only).is_finite());
    // The fused certificate carries its own decomposition; nothing converts
    // one into the other (no From/Into, no Deserialize, private fields).
    assert_eq!(fused.attempts().len(), 2);
    assert_eq!(fused.stepped_candidate().len(), 2 + 4);
}

#[test]
fn an_overflowing_term_is_unbounded_not_an_error() {
    // mu(M) ~ 1e300 makes e^omega overflow while Delta != 0.
    let mut w = vectors();
    for wk in w.iter_mut() {
        for x in wk.iter_mut() {
            *x *= 1e300;
        }
    }
    let cert = certify_fused_phi_total(&nonsymmetric(), 0.1, &w, &[0.0, 0.0]).unwrap();
    assert_eq!(cert.status(), FusedPhiStatus::Unbounded);
    assert!(cert.bound().is_infinite());
    assert!(cert.unbounded_reason().is_some());
}
