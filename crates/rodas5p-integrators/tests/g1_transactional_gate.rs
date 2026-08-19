use rodas5p_integrators::{G1TransactionalGateProfile, run_g1_transactional_gate};

#[test]
fn canonical_g1_gate_covers_the_declared_generic_families_without_explicit_jacobians() {
    let report = run_g1_transactional_gate(G1TransactionalGateProfile::Canonical).unwrap();
    assert_eq!(report.cases.len(), 8);
    assert_eq!(report.rows.len(), 8);
    assert_eq!(report.summary.completed, 8);
    assert_eq!(report.summary.false_accepts, 0);
    assert_eq!(report.summary.explicit_jacobian_builds, 0);
    assert_eq!(report.summary.direct_factorizations, 0);
    assert_eq!(report.summary.fast_path_newton_iterations, 0);
    let families = report
        .cases
        .iter()
        .map(|case| case.family.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for required in [
        "complex-dahlquist",
        "oscillatory-prothero-robinson",
        "stiff-van-der-pol",
        "robertson",
        "nonlinear-nonnormal-block",
        "diffusion-reaction",
        "advection-diffusion-reaction",
        "constant-noncommuting-mass",
    ] {
        assert!(families.contains(required), "missing {required}");
    }
}
