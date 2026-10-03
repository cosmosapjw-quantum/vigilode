//! Contracts of research node `research/rev02_nonnormal_stepping_20261003`:
//! the directed exponential, the Osborne metric and the stepped certificate.

use rodas5p_core::directed::exp_interval;
use rodas5p_core::nonnormal_certificate::{
    NonnormalBoundStatus, certify_exp_action_stepped, osborne_metric,
};

#[test]
fn the_directed_exponential_brackets_the_libm_value_tightly() {
    for k in -700..=700 {
        let x = k as f64 * 0.999;
        let e = exp_interval(x).unwrap();
        let reference = x.exp();
        assert!(e.lo <= e.hi);
        if reference.is_normal() {
            // libm is within a few ulps; the enclosure must reach it.
            assert!(
                e.lo <= reference * (1.0 + 4e-16) && e.hi >= reference * (1.0 - 4e-16),
                "{x}"
            );
            assert!(
                e.hi - e.lo <= 1e-14 * reference,
                "{x}: width {}",
                e.hi - e.lo
            );
        }
    }
    assert!(exp_interval(f64::NAN).is_err());
    assert!(exp_interval(710.0).is_err());
    let tiny = exp_interval(-800.0).unwrap();
    assert!(tiny.lo == 0.0 && tiny.hi > 0.0 && tiny.hi < 1e-300);
    let one = exp_interval(0.0).unwrap();
    assert!(one.lo <= 1.0 && one.hi >= 1.0);
}

#[test]
fn osborne_balances_the_vig_a02_matrix() {
    let k = 20;
    let a = vec![vec![-2.0, 2.0_f64.powi(k)], vec![2.0_f64.powi(-k), -2.0]];
    let d = osborne_metric(&a).unwrap();
    let b12 = d[0] * a[0][1] / d[1];
    let b21 = d[1] * a[1][0] / d[0];
    assert!(b12 / b21 <= 4.0 && b21 / b12 <= 4.0, "{b12} {b21}");
    assert!(d.iter().all(|x| x.log2().fract() == 0.0));
    assert!(osborne_metric(&[vec![1.0, 2.0]]).is_err());
    assert!(osborne_metric(&[vec![f64::NAN]]).is_err());
}

#[test]
fn the_stepped_certificate_encloses_a_scalar_decay_and_rejects_bad_input() {
    let a = vec![vec![-50.0]];
    let cert = certify_exp_action_stepped(&a, &[1.0], 1.0, None, 20, 64).unwrap();
    assert_eq!(cert.status, NonnormalBoundStatus::Bounded);
    let exact = (-50.0_f64).exp();
    assert!((cert.candidate[0] - exact).abs() <= cert.error_upper + 1e-30);
    assert!(cert.error_upper < 1e-20);
    assert!(certify_exp_action_stepped(&a, &[1.0], 1.0, None, 20, 0).is_err());
    assert!(certify_exp_action_stepped(&a, &[1.0], 0.1, None, 20, 3).is_err()); // 0.1/3 inexact
    assert!(certify_exp_action_stepped(&a, &[1.0], 1.0, None, 0, 4).is_err());
    assert!(certify_exp_action_stepped(&a, &[1.0], -1.0, None, 20, 4).is_err());
    assert!(certify_exp_action_stepped(&a, &[1.0], 1.0, Some(&[0.0]), 20, 4).is_err());
    assert!(certify_exp_action_stepped(&a, &[1.0, 2.0], 1.0, None, 20, 4).is_err());
    let capped = certify_exp_action_stepped(&a, &[1.0], 1.0, None, 20, 1 << 17).unwrap();
    assert_eq!(capped.status, NonnormalBoundStatus::Unbounded);
}
