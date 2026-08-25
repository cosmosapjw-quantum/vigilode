use rodas5p_core::LinearMethod;
use rodas5p_integrators::{
    FusedOrthogonalization, G4S5B0InnerToleranceLane, G4S5B0InnerTolerancePolicy,
    G4S5B0LinearToleranceArm, G4S5B0Profile,
};

fn outer_rtol(profile: G4S5B0Profile) -> f64 {
    profile.tolerances().1
}

#[test]
fn legacy_arm_preserves_the_protected_pre_a1_gmres_values() {
    for lane in G4S5B0InnerToleranceLane::ALL {
        let policy = G4S5B0InnerTolerancePolicy::try_for_lane(
            lane,
            G4S5B0LinearToleranceArm::LegacyFixed,
            1.0e-5,
        )
        .unwrap();
        let linear = policy.linear_config();

        assert_eq!(policy.lane(), lane);
        assert_eq!(policy.arm(), G4S5B0LinearToleranceArm::LegacyFixed);
        assert_eq!(linear.rtol.to_bits(), 1.0e-10_f64.to_bits());
        assert_eq!(linear.atol.to_bits(), 1.0e-12_f64.to_bits());
    }
}

#[test]
fn outer_scaled_arm_matches_the_preserved_phi_numbers_without_semantic_overclaim() {
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
        let rtol = outer_rtol(profile);
        for lane in G4S5B0InnerToleranceLane::ALL {
            let policy = G4S5B0InnerTolerancePolicy::try_for_lane(
                lane,
                G4S5B0LinearToleranceArm::OuterScaledNumericParity,
                rtol,
            )
            .unwrap();
            let linear = policy.linear_config();
            let phi = policy.phi_config(profile.dimensions()[0] + 4);

            assert_eq!(policy.lane(), lane);
            assert_eq!(
                policy.arm(),
                G4S5B0LinearToleranceArm::OuterScaledNumericParity
            );
            assert_eq!(
                policy.linear_relative_tolerance().to_bits(),
                policy.phi_relative_tolerance().to_bits()
            );
            assert_eq!(
                policy.linear_absolute_tolerance().to_bits(),
                policy.phi_absolute_tolerance().to_bits()
            );
            assert_eq!(linear.rtol.to_bits(), phi.relative_tolerance.to_bits());
            assert_eq!(linear.atol.to_bits(), phi.absolute_tolerance.to_bits());
        }
    }
}

#[test]
fn both_arms_preserve_the_exact_pre_a1_phi_arithmetic() {
    for outer_rtol in [1.0e-4, 1.0e-5, 3.0e-5, 7.0e-6, 1.0e-20] {
        for arm in G4S5B0LinearToleranceArm::ALL {
            let policy = G4S5B0InnerTolerancePolicy::try_for_lane(
                G4S5B0InnerToleranceLane::RegimeAtlas,
                arm,
                outer_rtol,
            )
            .unwrap();

            assert_eq!(
                policy.phi_relative_tolerance().to_bits(),
                (3.0e-2_f64 * outer_rtol).max(1.0e-12).to_bits()
            );
            assert_eq!(
                policy.phi_absolute_tolerance().to_bits(),
                (3.0e-4_f64 * outer_rtol).max(1.0e-14).to_bits()
            );
        }
    }
}

#[test]
fn all_six_runtime_lane_accessors_are_value_equivalent_for_a_given_arm() {
    for arm in G4S5B0LinearToleranceArm::ALL {
        let reference = G4S5B0InnerTolerancePolicy::try_for_lane(
            G4S5B0InnerToleranceLane::RegimeAtlas,
            arm,
            1.0e-5,
        )
        .unwrap();

        for lane in G4S5B0InnerToleranceLane::ALL {
            let candidate =
                G4S5B0InnerTolerancePolicy::try_for_lane(lane, arm, 1.0e-5).unwrap();
            assert_eq!(candidate.arm(), reference.arm());
            assert_eq!(
                candidate.linear_relative_tolerance().to_bits(),
                reference.linear_relative_tolerance().to_bits()
            );
            assert_eq!(
                candidate.linear_absolute_tolerance().to_bits(),
                reference.linear_absolute_tolerance().to_bits()
            );
            assert_eq!(
                candidate.phi_relative_tolerance().to_bits(),
                reference.phi_relative_tolerance().to_bits()
            );
            assert_eq!(
                candidate.phi_absolute_tolerance().to_bits(),
                reference.phi_absolute_tolerance().to_bits()
            );
        }
    }
}

#[test]
fn tolerance_construction_is_fallible_and_never_requires_abort() {
    for lane in G4S5B0InnerToleranceLane::ALL {
        for arm in G4S5B0LinearToleranceArm::ALL {
            for outer_rtol in [0.0, -1.0e-5, f64::NAN, f64::INFINITY] {
                assert!(
                    G4S5B0InnerTolerancePolicy::try_for_lane(lane, arm, outer_rtol).is_err()
                );
            }
        }
    }
}

#[test]
fn solver_structure_remains_arm_independent() {
    for arm in G4S5B0LinearToleranceArm::ALL {
        let policy = G4S5B0InnerTolerancePolicy::try_for_lane(
            G4S5B0InnerToleranceLane::RegimeAtlas,
            arm,
            1.0e-5,
        )
        .unwrap();
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
    }
}
