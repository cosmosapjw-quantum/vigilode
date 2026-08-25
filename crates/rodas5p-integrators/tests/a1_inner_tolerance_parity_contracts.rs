use rodas5p_core::LinearMethod;
use rodas5p_integrators::{FusedOrthogonalization, G4S5B0InnerTolerancePolicy, G4S5B0Profile};

fn assert_shared_tolerances(profile: G4S5B0Profile) {
    let (_, outer_rtol) = profile.tolerances();
    let policy = G4S5B0InnerTolerancePolicy::try_from_outer_rtol(outer_rtol).unwrap();
    let linear = policy.linear_config();
    let phi = policy.phi_config(profile.dimensions()[0] + 4);

    assert_eq!(linear.rtol.to_bits(), phi.relative_tolerance.to_bits());
    assert_eq!(linear.atol.to_bits(), phi.absolute_tolerance.to_bits());
    assert_eq!(policy.outer_rtol().to_bits(), outer_rtol.to_bits());
}

#[test]
fn canonical_holdout_derives_both_inner_solvers_from_one_outer_contract() {
    let policy = G4S5B0InnerTolerancePolicy::try_from_outer_rtol(1.0e-5).unwrap();
    let linear = policy.linear_config();
    let phi = policy.phi_config(516);

    assert_eq!(policy.relative_tolerance().to_bits(), 3.0e-7_f64.to_bits());
    assert_eq!(policy.absolute_tolerance().to_bits(), 3.0e-9_f64.to_bits());
    assert_eq!(linear.rtol.to_bits(), phi.relative_tolerance.to_bits());
    assert_eq!(linear.atol.to_bits(), phi.absolute_tolerance.to_bits());
}

#[test]
fn every_frozen_g4_s5b0_profile_uses_linear_phi_tolerance_parity() {
    for profile in [
        G4S5B0Profile::Smoke,
        G4S5B0Profile::Canonical,
        G4S5B0Profile::Calibration128,
        G4S5B0Profile::Holdout512,
        G4S5B0Profile::StageGrowthCalibration96,
        G4S5B0Profile::StageGrowthCalibration192,
        G4S5B0Profile::StageGrowthCalibration256,
        G4S5B0Profile::EnforcedBudgetHoldout320,
        G4S5B0Profile::StageGrowthHoldout384,
    ] {
        assert_shared_tolerances(profile);
    }
}

#[test]
fn inner_tolerance_floors_are_shared_without_underflow() {
    let policy = G4S5B0InnerTolerancePolicy::try_from_outer_rtol(1.0e-20).unwrap();
    let linear = policy.linear_config();
    let phi = policy.phi_config(8);

    assert_eq!(policy.relative_tolerance().to_bits(), 1.0e-12_f64.to_bits());
    assert_eq!(policy.absolute_tolerance().to_bits(), 1.0e-14_f64.to_bits());
    assert_eq!(linear.rtol.to_bits(), phi.relative_tolerance.to_bits());
    assert_eq!(linear.atol.to_bits(), phi.absolute_tolerance.to_bits());
}

#[test]
fn nonpositive_or_nonfinite_outer_tolerance_is_rejected() {
    for outer_rtol in [0.0, -1.0e-5, f64::NAN, f64::INFINITY] {
        assert!(G4S5B0InnerTolerancePolicy::try_from_outer_rtol(outer_rtol).is_err());
    }
}

#[test]
fn parity_policy_preserves_structure_and_is_wired_into_every_g4_s5b0_lane() {
    let policy = G4S5B0InnerTolerancePolicy::try_from_outer_rtol(1.0e-5).unwrap();
    let linear = policy.linear_config();
    let phi = policy.phi_config(128);

    assert_eq!(linear.method, LinearMethod::Gmres);
    assert_eq!(linear.restart, 32);
    assert_eq!(linear.maxiter, 256);
    assert_eq!(phi.minimum_dimension, 2);
    assert_eq!(phi.maximum_dimension, 32);
    assert_eq!(phi.dimension_increment, 2);
    assert_eq!(phi.orthogonalization, FusedOrthogonalization::FullMgs);
    assert_eq!(phi.maximum_substeps, 16);

    let atlas_source = include_str!("../src/g4_s5b0_regime_atlas.rs");
    assert_eq!(
        atlas_source
            .matches("linear_config(adaptive.rtol)")
            .count(),
        6
    );
    assert!(!atlas_source.contains("let linear = linear_config();"));
    assert!(atlas_source.contains("inner_tolerance_policy(rtol).phi_config(dimension)"));
    assert!(atlas_source.contains("inner_tolerance_policy(rtol).linear_config()"));
    assert!(!atlas_source.contains("rtol: 1.0e-10"));
    assert!(!atlas_source.contains("atol: 1.0e-12"));
}
