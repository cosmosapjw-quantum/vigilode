use rodas5p_krylov::{
    CertifiedInterval as Interval, ConditionalStageVerdict, N03SourceStamp as SourceStamp,
    StageIntervalBox,
};
use std::cell::Cell;
fn stamp() -> SourceStamp {
    SourceStamp {
        model: "point-J".into(),
        source: "frozen-test".into(),
        clock: "proper-time".into(),
        norm: "positive-winf".into(),
        epoch: 7,
    }
}
fn b(v: f64) -> Interval {
    Interval::point(v).unwrap()
}
fn valid() -> StageIntervalBox {
    // A=I-(1/4)J, J12 in [8,8.25], other entries fixed.
    StageIntervalBox::new(
        2,
        vec![
            b(-2.0),
            Interval::new(8.0, 8.25).unwrap(),
            b(0.125),
            b(-3.0),
        ],
        vec![Interval::new(1.0, 1.0 + 1.0 / 64.0).unwrap(), b(2.0)],
        vec![8.0, 1.0],
        vec![4.0, 2.0],
        0.25,
        stamp(),
    )
    .unwrap()
}
fn nominal() -> [f64; 2] {
    [92.0 / 41.0, 97.0 / 82.0]
}
#[test]
fn bound_expected_in_nonnormal_weighted_case() {
    let v = valid();
    let z = v.certify(&nominal(), 1.0, 1.0).unwrap();
    assert!(z.admitted_within_given_box);
    assert!(z.signed_lognorm_upper >= -31.0 / 32.0);
    assert!(z.weighted_inverse_upper >= 128.0 / 159.0);
    assert!(z.norm_transport_upper >= 2.0);
    assert!(z.original_residual_upper > 0.0);
    assert!(z.physical_error_upper >= z.original_residual_upper * 2.0 * 128.0 / 159.0);
}
#[test]
fn refuses_unproved_inverse_margin() {
    let j = vec![b(12.0)];
    let s = StageIntervalBox::new(1, j, vec![b(1.)], vec![1.], vec![1.], 0.5, stamp()).unwrap();
    assert!(s.certify(&[0.], 1., 1.).is_err());
}
#[test]
fn rejects_uncertain_candidate_without_output_mutation() {
    let v = valid();
    let mut out = [55., 55.];
    let s = stamp();
    let r = v
        .commit_if_admitted(&s, stamp, &[0., 0.], &mut out, 1e-12, 1e-12)
        .unwrap();
    assert!(!r.admitted_within_given_box);
    assert_eq!(out, [55., 55.]);
}
#[test]
fn successful_transaction_copies_candidate() {
    let v = valid();
    let s = stamp();
    let mut out = [55., 55.];
    assert!(
        v.commit_if_admitted(&s, stamp, &nominal(), &mut out, 1., 1.)
            .unwrap()
            .admitted_within_given_box
    );
    assert_eq!(out, nominal());
}
#[test]
fn stale_identity_before_commit_fails_without_output() {
    let v = valid();
    let s = stamp();
    let mut out = [55., 55.];
    let call = Cell::new(0);
    let err = v
        .commit_if_admitted(
            &s,
            || {
                call.set(call.get() + 1);
                let mut t = stamp();
                if call.get() > 1 {
                    t.epoch += 1;
                }
                t
            },
            &nominal(),
            &mut out,
            1.,
            1.,
        )
        .unwrap_err();
    assert!(err.contains("stale"));
    assert_eq!(out, [55., 55.]);
}
#[test]
fn zero_shift_and_exact_candidate() {
    let s = StageIntervalBox::new(1, vec![b(3.)], vec![b(2.)], vec![1.], vec![1.], 0., stamp())
        .unwrap();
    let v = s.certify(&[2.], 1e-10, 1e-10).unwrap();
    assert!(v.admitted_within_given_box);
    assert!(v.weighted_inverse_upper >= 1.0);
}
#[test]
fn invalid_source_and_inputs_refused() {
    for wrong in [0., f64::NAN, f64::INFINITY, -1.] {
        assert!(
            StageIntervalBox::new(
                1,
                vec![b(-1.)],
                vec![b(1.)],
                vec![wrong],
                vec![1.],
                0.5,
                stamp()
            )
            .is_err()
        );
    }
    assert!(
        StageIntervalBox::new(
            1,
            vec![b(-1.)],
            vec![b(1.)],
            vec![1.],
            vec![1.],
            -0.5,
            stamp()
        )
        .is_err()
    );
    assert!(Interval::new(2., 1.).is_err());
    assert!(Interval::new(0., f64::INFINITY).is_err());
    let s = valid();
    assert!(s.certify(&nominal(), f64::NAN, 1.).is_err());
    assert!(s.certify(&[f64::NAN, 0.], 1., 1.).is_err());
}
#[test]
fn overflow_is_not_an_accepted_certificate() {
    let s = StageIntervalBox::new(
        1,
        vec![b(1e308)],
        vec![b(1.)],
        vec![1.],
        vec![1.],
        1.,
        stamp(),
    )
    .unwrap();
    assert!(s.certify(&[1e308], 1e308, 1e308).is_err());
}
#[test]
fn narrowing_source_interval_cannot_worsen_exact_residual_enclosure() {
    let wide = valid().certify(&nominal(), 1., 1.).unwrap();
    let narrow = StageIntervalBox::new(
        2,
        vec![b(-2.), b(8.), b(0.125), b(-3.)],
        vec![b(1.), b(2.)],
        vec![8., 1.],
        vec![4., 2.],
        0.25,
        stamp(),
    )
    .unwrap()
    .certify(&nominal(), 1., 1.)
    .unwrap();
    assert!(wide.original_residual_upper >= narrow.original_residual_upper);
    assert!(wide.signed_lognorm_upper >= narrow.signed_lognorm_upper);
}
#[test]
fn contract_is_not_physical_source_proof() {
    let v = valid().certify(&nominal(), 1., 1.).unwrap();
    let _: ConditionalStageVerdict = v;
    // No API can infer underlying physical source interval membership.
    assert!(v.admitted_within_given_box);
}

#[test]
fn norm_transport_is_required_to_avoid_false_admission() {
    let s = StageIntervalBox::new(
        2,
        vec![b(0.); 4],
        vec![b(1.), b(0.)],
        vec![100., 1.],
        vec![1., 1.],
        0.,
        stamp(),
    )
    .unwrap();
    let v = s.certify(&[0., 0.], 0.02, 0.02).unwrap();
    assert!(v.original_residual_upper < 0.02);
    assert!(v.norm_transport_upper >= 100.);
    assert!(v.physical_error_upper >= 1.);
    assert!(!v.admitted_within_given_box);
}
