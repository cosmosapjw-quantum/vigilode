use rodas5p_core::{
    directed::Interval,
    laguerre_adjoint::{LaguerreEnvelopeCache, laguerre_adjoint_envelopes_interval},
};
#[test]
fn reversed_intervals_cannot_be_widened_into_accepted_coefficients() {
    let c = [
        Interval { lo: 1e-40, hi: 0.0 },
        Interval { lo: 0.0, hi: 1.0 },
    ];
    let result = laguerre_adjoint_envelopes_interval(&c, 1.0, 3);
    assert!(
        result.is_err(),
        "invalid input became an envelope: {result:?}"
    );
}
#[test]
fn a_failed_invalid_input_does_not_enter_the_envelope_cache() {
    let mut cache = LaguerreEnvelopeCache::default();
    let c = [
        Interval { lo: 1e-40, hi: 0.0 },
        Interval { lo: 0.0, hi: 1.0 },
    ];
    assert!(cache.envelopes(&c, 1.0, 3).is_err());
    assert!(cache.is_empty());
}
#[test]
fn ordered_interval_coefficients_still_work() {
    let c = [
        Interval::new(-1.0, 1.0).unwrap(),
        Interval::point(0.5).unwrap(),
    ];
    let (bounds, _) = laguerre_adjoint_envelopes_interval(&c, 1.0, 3).unwrap();
    assert!(bounds.iter().all(|x| x.is_finite() && *x >= 0.0));
}
