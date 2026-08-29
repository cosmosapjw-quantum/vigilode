use rodas5p_fair_ab::{
    AccuracyAssessment as A, GlobalErrorMetric, MeasurementResolution as R,
    OutputPolicyDominance as L, OutputPolicyMetricKey, OutputSamplingPolicy as P,
    assess_policy_measurement as assess, classify_output_policy_dominance as legacy,
};
#[test]
fn independent_accuracy_does_not_require_error_parity() {
    assert_eq!(legacy(0.5, 0.3).unwrap(), L::Dominated);
    for (p, e) in [(P::Clipped, 0.2), (P::Dense, 0.3)] {
        let a = assess(p, true, Some(e), Some(0.), Some(1.)).unwrap();
        assert_eq!(a.resolution, R::Resolved);
        assert_eq!(a.accuracy, A::WithinDeclaredBudget);
    }
}
#[test]
fn common_bias_is_not_accuracy() {
    assert_eq!(legacy(0., 100.).unwrap(), L::Admissible);
    assert_eq!(
        assess(P::Dense, true, Some(100.), Some(0.), Some(1.))
            .unwrap()
            .accuracy,
        A::ExceedsDeclaredBudget
    );
}
#[test]
fn missing_budget_never_inferred() {
    let a = assess(P::Dense, true, Some(1e5), Some(1e-3), None).unwrap();
    assert_eq!(a.resolution, R::Resolved);
    assert_eq!(a.accuracy, A::NotRequested);
}
#[test]
fn missing_and_failed_evidence_remain_explicit() {
    let a = assess(P::Dense, true, None, None, Some(1.)).unwrap();
    assert_eq!(a.resolution, R::MissingEvidence);
    assert_eq!(a.accuracy, A::NotAssessed);
    let b = assess(P::Dense, false, Some(0.), Some(0.), Some(1.)).unwrap();
    assert_eq!(b.resolution, R::RunFailed);
    assert_eq!(b.accuracy, A::NotAssessed);
}
#[test]
fn uncertainty_resolution_and_accuracy_are_independent() {
    let a = assess(P::Dense, true, Some(0.1), Some(0.2), Some(1.)).unwrap();
    assert_eq!(a.resolution, R::ReferenceLimited);
    assert_eq!(a.accuracy, A::WithinDeclaredBudget);
    assert_eq!(
        assess(P::Dense, true, Some(0.9), Some(0.2), Some(1.))
            .unwrap()
            .accuracy,
        A::InconclusiveAtBudget
    );
}
#[test]
fn exact_zero_is_valid() {
    let a = assess(P::Clipped, true, Some(0.), Some(0.), Some(0.)).unwrap();
    assert_eq!(a.resolution, R::Resolved);
    assert_eq!(a.accuracy, A::WithinDeclaredBudget);
}
#[test]
fn malformed_and_overflow_inputs_fail() {
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.] {
        assert!(assess(P::Dense, true, Some(x), Some(0.), None).is_err());
        assert!(assess(P::Dense, true, Some(0.), Some(x), None).is_err());
        assert!(assess(P::Dense, true, Some(0.), Some(0.), Some(x)).is_err());
    }
    assert!(assess(P::Dense, true, Some(f64::MAX), Some(f64::MAX), None).is_err());
}
#[test]
fn keys_separate_policy_scale_grid_and_observable() {
    let a = OutputPolicyMetricKey {
        problem_id: "p".into(),
        output_grid_id: "g".into(),
        scale_id: "s".into(),
        metric: GlobalErrorMetric::MaxGridWrms,
        policy: P::Dense,
    };
    assert!(a.comparable_with(&a));
    let mut b = a.clone();
    b.policy = P::Clipped;
    assert!(!a.comparable_with(&b));
    b = a.clone();
    b.scale_id = "other".into();
    assert!(!a.comparable_with(&b));
    b = a.clone();
    b.output_grid_id = "other".into();
    assert!(!a.comparable_with(&b));
    b = a.clone();
    b.metric = GlobalErrorMetric::EndpointWrms;
    assert!(!a.comparable_with(&b));
}
#[test]
fn existing_reference_fraction_is_preserved() {
    assert_eq!(
        assess(P::Dense, true, Some(1.), Some(0.1), None)
            .unwrap()
            .resolution,
        R::Resolved
    );
    assert_eq!(
        assess(P::Dense, true, Some(1.), Some(0.1001), None)
            .unwrap()
            .resolution,
        R::ReferenceLimited
    );
}
