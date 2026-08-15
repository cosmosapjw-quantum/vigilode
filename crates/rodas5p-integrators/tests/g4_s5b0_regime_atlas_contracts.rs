use rodas5p_integrators::{G4S5B0Profile, run_g4_s5b0_regime_atlas};

#[test]
fn smoke_atlas_is_read_only_and_matrix_free() {
    let report = run_g4_s5b0_regime_atlas(G4S5B0Profile::Smoke).unwrap();
    assert!(!report.switching_active);
    assert_eq!(
        report.committed_method,
        "protected-sequential-matrix-free-rodas5p"
    );
    assert_eq!(report.trajectories.len(), 6);
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.rodas_embedded_error.is_finite())
    );
    assert!(
        report
            .trajectories
            .iter()
            .all(|row| row.explicit_jacobian_builds == 0 && row.direct_factorizations == 0)
    );
}

#[test]
fn smoke_atlas_contains_all_transition_families() {
    let report = run_g4_s5b0_regime_atlas(G4S5B0Profile::Smoke).unwrap();
    let families = report
        .trajectories
        .iter()
        .map(|row| row.family.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(families.len(), 6);
    for required in [
        "robertson-ramped",
        "hires-ramped",
        "van-der-pol-ramped",
        "rotating-nonnormal",
        "nonautonomous-stiff-forcing",
        "semilinear-advection-diffusion-ramped",
    ] {
        assert!(families.contains(required));
    }
}
