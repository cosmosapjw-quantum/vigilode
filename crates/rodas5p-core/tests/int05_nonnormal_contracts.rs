//! Contracts of the nonnormal exponential-action certificate (research node
//! `research/int05_nonnormal_metric_bound_20261003`, integrated DAG node
//! INT-05): typed rejections and a scalar sanity check.

use rodas5p_core::nonnormal_certificate::{NonnormalBoundStatus, certify_exp_action};

#[test]
fn a_scalar_decay_is_enclosed_from_both_sides() {
    let a = vec![vec![-1.0]];
    let exact = (-1.0_f64).exp();
    for degree in [5usize, 20] {
        let own = certify_exp_action(&a, &[1.0], 1.0, None, degree, None).unwrap();
        assert_eq!(own.status, NonnormalBoundStatus::Bounded);
        let error = (own.candidate[0] - exact).abs();
        assert!(own.error_lower <= error && error <= own.error_upper * (1.0 + 1e-12) + 1e-16);
        // A wrong candidate is exposed.
        let wrong = certify_exp_action(&a, &[1.0], 1.0, None, degree, Some(&[0.5])).unwrap();
        assert!(wrong.error_lower > 0.1 && wrong.error_upper >= (0.5 - exact).abs());
    }
}

#[test]
fn invalid_inputs_are_rejected() {
    let a = vec![vec![-2.0, 1.0], vec![0.5, -3.0]];
    let v = [1.0, 0.0];
    assert!(certify_exp_action(&a, &v, 1.0, None, 10, None).is_ok());
    assert!(certify_exp_action(&a, &[1.0], 1.0, None, 10, None).is_err());
    assert!(certify_exp_action(&[vec![1.0, 2.0]], &v, 1.0, None, 10, None).is_err());
    for degree in [0usize, 61] {
        assert!(certify_exp_action(&a, &v, 1.0, None, degree, None).is_err());
    }
    for tau in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(certify_exp_action(&a, &v, tau, None, 10, None).is_err());
    }
    let mut bad = a.clone();
    bad[1][0] = f64::NAN;
    assert!(certify_exp_action(&bad, &v, 1.0, None, 10, None).is_err());
    for metric in [[1.0, 0.0], [1.0, -2.0], [1.0, f64::INFINITY]] {
        assert!(certify_exp_action(&a, &v, 1.0, Some(&metric), 10, None).is_err());
    }
    assert!(certify_exp_action(&a, &v, 1.0, Some(&[1.0]), 10, None).is_err());
    assert!(certify_exp_action(&a, &v, 1.0, None, 10, Some(&[1.0])).is_err());
    assert!(certify_exp_action(&a, &v, 1.0, None, 10, Some(&[1.0, f64::NAN])).is_err());
}

#[test]
fn an_overflowing_growth_factor_is_unbounded_not_a_number() {
    // Euclidean numerical range of the VIG-A02 matrix at k = 46 reaches 3.5e13.
    let k = 46;
    let a = vec![vec![-2.0, 2.0_f64.powi(k)], vec![2.0_f64.powi(-k), -2.0]];
    let cert = certify_exp_action(&a, &[1.0, 0.0], 1.0, None, 30, None).unwrap();
    assert_eq!(cert.status, NonnormalBoundStatus::Unbounded);
    assert!(cert.error_upper.is_infinite() && cert.error_lower == 0.0);
}
