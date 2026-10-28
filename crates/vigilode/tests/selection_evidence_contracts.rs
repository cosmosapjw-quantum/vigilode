use vigilode::selection_evidence::*;

fn evidence(lower: f64, upper: f64) -> CostEvidence {
    CostEvidence {
        identity: SelectionIdentity::default(),
        cost: CostInterval { lower, upper },
        unit: CostUnit::Work,
        kind: CostEvidenceKind::ObservedWork,
        admissible: true,
    }
}
#[test]
fn strict_qualified_gap_and_future_transition() {
    let old = evidence(10.0, 12.0);
    let new = evidence(4.0, 5.0);
    assert_eq!(
        compare_qualified_costs(&old, &new, 0.0, 1, 0.1),
        CostComparison::PreferCandidate
    );
    assert_eq!(
        compare_qualified_costs(&old, &new, 10.0, 1, 0.1),
        CostComparison::RetainIncumbent
    );
    assert_eq!(
        compare_qualified_costs(&old, &new, 10.0, 10, 0.1),
        CostComparison::PreferCandidate
    );
}
#[test]
fn overlapping_intervals_do_not_invent_a_gain() {
    assert_eq!(
        compare_qualified_costs(&evidence(8.0, 12.0), &evidence(5.0, 10.0), 0.0, 1, 0.1),
        CostComparison::RetainIncumbent
    );
}
#[test]
fn units_outputs_and_scope_are_independent_qualifications() {
    let old = evidence(10.0, 10.0);
    let mut new = evidence(1.0, 1.0);
    new.unit = CostUnit::Seconds;
    assert_eq!(
        compare_qualified_costs(&old, &new, 0.0, 1, 0.1),
        CostComparison::Incomparable(CostIncomparability::UnitMismatch)
    );
    new.unit = CostUnit::Work;
    new.admissible = false;
    assert_eq!(
        compare_qualified_costs(&old, &new, 0.0, 1, 0.1),
        CostComparison::Incomparable(CostIncomparability::OutputNotQualified)
    );
    new.admissible = true;
    new.identity.window_epoch = 1;
    assert_eq!(
        compare_qualified_costs(&old, &new, 0.0, 1, 0.1),
        CostComparison::Incomparable(CostIncomparability::IdentityMismatch)
    );
}
#[test]
fn invalid_numbers_and_zero_horizon_refuse() {
    for bad in [f64::NAN, f64::INFINITY, -1.0] {
        assert_eq!(
            compare_qualified_costs(&evidence(10.0, 10.0), &evidence(bad, bad), 0.0, 1, 0.1),
            CostComparison::Incomparable(CostIncomparability::InvalidInterval)
        );
    }
    assert_eq!(
        compare_qualified_costs(&evidence(10.0, 10.0), &evidence(1.0, 1.0), 0.0, 0, 0.1),
        CostComparison::Incomparable(CostIncomparability::InvalidPolicy)
    );
}
#[test]
fn equality_at_rounded_boundary_is_conservative() {
    let old = evidence(10.0, 10.0);
    let new = evidence(9.0, 9.0);
    assert_eq!(
        compare_qualified_costs(&old, &new, 0.0, 1, 0.1),
        CostComparison::RetainIncumbent
    );
    assert_eq!(
        compare_qualified_costs(&old, &evidence(8.99, 8.99), 0.0, 1, 0.1),
        CostComparison::PreferCandidate
    );
}
