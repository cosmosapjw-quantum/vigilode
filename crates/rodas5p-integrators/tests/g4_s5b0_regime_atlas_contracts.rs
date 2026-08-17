use rodas5p_integrators::{
    G4S5B0Family, G4S5B0Profile, run_g4_s5b0_regime_atlas, run_g4_s5b0_rjf_only,
};

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

#[test]
fn split_profiles_freeze_dimension_and_canonical_tolerances() {
    assert_eq!(G4S5B0Profile::Calibration128.dimensions(), &[128]);
    assert_eq!(G4S5B0Profile::Holdout512.dimensions(), &[512]);
    assert_eq!(G4S5B0Profile::Calibration128.as_str(), "calibration-128");
    assert_eq!(G4S5B0Profile::Holdout512.as_str(), "holdout-512");
    assert!(G4S5B0Profile::Calibration128.uses_canonical_tolerances());
    assert!(G4S5B0Profile::Holdout512.uses_canonical_tolerances());
}

#[test]
fn rjf_only_smoke_skips_every_exponential_shadow() {
    let report = run_g4_s5b0_rjf_only(G4S5B0Profile::Smoke).unwrap();
    assert_eq!(report.schema, "g4-s5b0-rjf-only-regime-replay-v1");
    assert_eq!(report.trajectories.len(), 6);
    assert!(!report.rows.is_empty());
    assert!(report.rows.iter().all(|row| {
        !row.exponential_completed
            && row.exponential_total_error.is_none()
            && row.exponential_wall_seconds.is_none()
            && row.exponential_prefix_wall_seconds.is_none()
            && row.exponential_rhs_evaluations.is_none()
            && row.exponential_jvp_vectors.is_none()
            && row.exponential_failure.is_none()
    }));
    assert!(report.trajectories.iter().all(|row| {
        row.explicit_jacobian_builds == 0
            && row.direct_factorizations == 0
            && row.newton_iterations == 0
    }));
}

#[test]
fn family_shard_registry_matches_the_six_atlas_families() {
    let names = G4S5B0Family::ALL.map(G4S5B0Family::as_str);
    assert_eq!(
        names,
        [
            "robertson-ramped",
            "hires-ramped",
            "van-der-pol-ramped",
            "rotating-nonnormal",
            "nonautonomous-stiff-forcing",
            "semilinear-advection-diffusion-ramped",
        ]
    );
}
