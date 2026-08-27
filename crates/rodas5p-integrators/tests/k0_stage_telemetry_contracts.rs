#[path = "../src/k0_stage_telemetry.rs"]
mod k0_stage_telemetry;

use k0_stage_telemetry::{
    K0_CLAIM_CLASS, K0_STAGE_RECEIPT_SCHEMA, K0ExecutionState, K0InitialGuessSource, K0KernelArm,
    K0ResidualSignCoverage, K0StageError, K0StageGeometry, K0StageIdentity, K0StageReceipt,
    K0StageWork,
};

fn identity() -> K0StageIdentity {
    K0StageIdentity {
        case_id: "hires-legacy-fixed".into(),
        family: "Hires".into(),
        kernel_arm: K0KernelArm::LegacyRestartedGmres,
        attempt_id: 1,
        stage_index: 3,
    }
}

fn complete_receipt() -> K0StageReceipt {
    K0StageReceipt {
        schema: K0_STAGE_RECEIPT_SCHEMA.into(),
        identity: identity(),
        claim_class: K0_CLAIM_CLASS.into(),
        execution_state: K0ExecutionState::Complete,
        solver_method: Some("legacy-restarted-gmres".into()),
        initial_guess_source: Some(K0InitialGuessSource::Zero),
        initial_true_residual_norm: Some(4.0),
        final_true_residual_norm: Some(1.0e-12),
        work: Some(K0StageWork::new(5, 2, 1).expect("checked work")),
        geometry: Some(K0StageGeometry {
            scaled_nonlinear_remainder: Some(0.25),
            rhs_jacobian_angle_radians: Some(0.5),
            singular_values: vec![2.0, 0.5],
        }),
        error: None,
    }
}

#[test]
fn schema_accepts_exact_contract_and_rejects_wrong_schema_claim_or_identity() {
    let receipt = complete_receipt();
    assert_eq!(receipt.validate(), Ok(()));
    assert_eq!(
        receipt.identity.kernel_arm.as_str(),
        "legacy-restarted-gmres"
    );
    assert_eq!(
        K0KernelArm::IncrementalGivensCandidate.as_str(),
        "incremental-givens-candidate"
    );

    let mut wrong_schema = receipt.clone();
    wrong_schema.schema = "vigilode-k0-stage-receipt/v0".into();
    assert!(wrong_schema.validate().is_err());

    let mut wrong_claim = receipt.clone();
    wrong_claim.claim_class = "AUTHORITATIVE".into();
    assert!(wrong_claim.validate().is_err());

    let mut missing_family = receipt;
    missing_family.identity.family.clear();
    assert!(missing_family.validate().is_err());
}

#[test]
fn accounting_checked_total_accepts_exact_and_rejects_diagnostic_omission() {
    assert_eq!(
        K0StageWork::new(11, 7, 3).unwrap().operator_applies_total,
        18
    );

    let omitted_diagnostic = K0StageWork {
        linear_matvecs: 11,
        diagnostic_matvecs: 7,
        operator_applies_total: 11,
        telemetry_jvp_overhead: 3,
    };
    assert!(omitted_diagnostic.validate().is_err());
}

#[test]
fn accounting_counter_overflow_is_rejected() {
    assert!(K0StageWork::new(u64::MAX, 1, 0).is_err());
}

#[test]
fn execution_state_error_preserves_identity_work_and_error() {
    let mut receipt = complete_receipt();
    receipt.execution_state = K0ExecutionState::Error;
    receipt.error = Some(K0StageError {
        kind: "LINEAR_SOLVE_FAILURE".into(),
        message: "true residual remained above tolerance".into(),
    });
    assert_eq!(receipt.validate(), Ok(()));
    assert_eq!(receipt.identity(), &identity());
    assert!(receipt.work.is_some());
}

#[test]
fn execution_state_error_without_work_is_rejected() {
    let mut receipt = complete_receipt();
    receipt.execution_state = K0ExecutionState::Error;
    receipt.error = Some(K0StageError {
        kind: "LINEAR_SOLVE_FAILURE".into(),
        message: "failed before completion".into(),
    });
    receipt.work = None;
    assert!(receipt.validate().is_err());
}

#[test]
fn schema_rejects_nan_and_infinite_norms_geometry() {
    let mut nan_norm = complete_receipt();
    nan_norm.final_true_residual_norm = Some(f64::NAN);
    assert!(nan_norm.validate().is_err());

    let mut infinite_angle = complete_receipt();
    infinite_angle
        .geometry
        .as_mut()
        .unwrap()
        .rhs_jacobian_angle_radians = Some(f64::INFINITY);
    assert!(infinite_angle.validate().is_err());

    let mut negative_singular_value = complete_receipt();
    negative_singular_value
        .geometry
        .as_mut()
        .unwrap()
        .singular_values = vec![1.0, -0.1];
    assert!(negative_singular_value.validate().is_err());
}

#[test]
fn residual_sign_norm_telemetry_is_explicitly_non_authoritative() {
    assert_eq!(
        K0StageReceipt::residual_sign_coverage(),
        K0ResidualSignCoverage::NotCoveredByNormTelemetry
    );
}
