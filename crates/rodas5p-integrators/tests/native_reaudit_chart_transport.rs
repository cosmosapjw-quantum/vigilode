#![cfg(feature = "audit2-research")]
use rodas5p_integrators::chart_transport::{ReconstructionInput, certify_reconstruction};
fn valid() -> ReconstructionInput {
    ReconstructionInput {
        x: 1.0,
        w: 0.125,
        kappa: 2.0,
        x_error: 0.0625,
        w_error: 0.03125,
        stored_y: 0.625,
        denominator_min: 0.25,
    }
}
#[test]
fn reconstruction_encloses_every_dyadic_corner_and_interior_sample() {
    for sign in [-1.0, 1.0] {
        let mut input = valid();
        input.x *= sign;
        let cert = certify_reconstruction(&input).unwrap();
        for i in -4..=4 {
            for j in -4..=4 {
                let x = input.x + f64::from(i) * input.x_error / 4.0;
                let w = input.w + f64::from(j) * input.w_error / 4.0;
                // All numbers in this fixture and this degree-3 expression are
                // exactly representable dyadics, so this oracle has no rounding.
                let exact = x * x * (w + 0.5);
                assert!((exact - input.stored_y).abs() <= cert.physical_y_error_upper());
            }
        }
    }
}
#[test]
fn a_bad_stored_reconstruction_is_included_not_assumed_exact() {
    let mut input = valid();
    input.x_error = 0.0;
    input.w_error = 0.0;
    input.stored_y = 0.0;
    assert!(
        certify_reconstruction(&input)
            .unwrap()
            .physical_y_error_upper()
            >= 0.625
    );
}
#[test]
fn zero_coordinate_and_reconstruction_error_gives_zero() {
    let mut input = valid();
    input.x_error = 0.0;
    input.w_error = 0.0;
    assert_eq!(
        certify_reconstruction(&input)
            .unwrap()
            .physical_y_error_upper(),
        0.0
    );
}
#[test]
fn the_whole_box_not_just_its_center_must_remain_in_the_chart() {
    let mut input = valid();
    input.x_error = 1.0;
    assert!(certify_reconstruction(&input).is_err());
    input.x_error = 0.75; // no sign crossing, but determinant may be only 1/16.
    assert!(certify_reconstruction(&input).is_err());
}
#[test]
fn nonfinite_negative_or_overflowing_inputs_fail_closed() {
    let mut bad = Vec::new();
    let mut q = valid();
    q.kappa = f64::INFINITY;
    bad.push(q);
    let mut q = valid();
    q.x_error = -1.0;
    bad.push(q);
    let mut q = valid();
    q.w = f64::NAN;
    bad.push(q);
    let mut q = valid();
    q.stored_y = f64::INFINITY;
    bad.push(q);
    let mut q = valid();
    q.denominator_min = 0.0;
    bad.push(q);
    let mut q = valid();
    q.x = 1e300;
    bad.push(q);
    for q in bad {
        assert!(certify_reconstruction(&q).is_err());
    }
}
