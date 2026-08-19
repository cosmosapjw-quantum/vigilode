use rodas5p_integrators::G4S5B0Profile;

#[test]
fn independent_zeta34_calibration_profile_is_exactly_predeclared() {
    let profile = G4S5B0Profile::StageGrowthCalibration192;
    assert_eq!(profile.as_str(), "stage-growth-calibration-192");
    assert_eq!(profile.dimensions(), &[192]);
    assert_eq!(profile.tolerances(), (1.5e-7, 1.5e-5));
    assert!(!profile.uses_canonical_tolerances());
}
