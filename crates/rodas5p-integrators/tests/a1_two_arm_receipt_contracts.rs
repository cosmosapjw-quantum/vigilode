use rodas5p_integrators::{
    A1ScientificExecutionIdentity, G4S5B0Family, G4S5B0LinearToleranceArm, G4S5B0Profile,
    committed_g4_s5b0_linear_tolerance_arm, run_a1_two_arm_receipt_cell,
    run_g4_s5b0_rjf_attempt_trace_family_with_linear_tolerance_arm,
};

fn execution_identity() -> A1ScientificExecutionIdentity {
    A1ScientificExecutionIdentity {
        repository: "cosmosapjw-quantum/vigilode".into(),
        pull_request: 18,
        scientific_execution_head_sha: "1111111111111111111111111111111111111111".into(),
        scientific_execution_head_tree: "2222222222222222222222222222222222222222".into(),
        base_sha: "3333333333333333333333333333333333333333".into(),
        base_tree: "4444444444444444444444444444444444444444".into(),
        tested_execution_merge_sha: "5555555555555555555555555555555555555555".into(),
        tested_execution_merge_tree: "6666666666666666666666666666666666666666".into(),
        execution_workflow_run_id: 123,
        execution_workflow_run_attempt: 1,
        rust_version: "rustc 1.94.1".into(),
        cargo_version: "cargo 1.94.1".into(),
    }
}

#[test]
fn receipt_only_candidate_is_explicit_and_ordinary_runtime_remains_legacy_fixed() {
    assert_eq!(
        committed_g4_s5b0_linear_tolerance_arm(),
        G4S5B0LinearToleranceArm::LegacyFixed
    );
    let ordinary = run_g4_s5b0_rjf_attempt_trace_family_with_linear_tolerance_arm(
        G4S5B0Profile::EnforcedBudgetHoldout320,
        G4S5B0Family::RobertsonRamped,
        G4S5B0LinearToleranceArm::OuterScaledNumericParity,
    );
    assert!(ordinary.is_err());

    let cell = run_a1_two_arm_receipt_cell(
        execution_identity(),
        G4S5B0Family::RobertsonRamped,
        G4S5B0LinearToleranceArm::OuterScaledNumericParity,
    )
    .expect("receipt-only candidate cell");
    assert_eq!(cell.schema, "vigilode-a1-two-arm-atomic-cell-v1");
    assert_eq!(cell.profile, "enforced-budget-holdout-320");
    assert_eq!(cell.family, "robertson-ramped");
    assert_eq!(cell.arm, "outer-scaled-numeric-parity");
    assert_eq!(cell.outer_rtol, 1.0e-5);
    assert_eq!(cell.linear_rtol, 3.0e-2 * 1.0e-5);
    assert_eq!(cell.linear_atol, 3.0e-4 * 1.0e-5);
    assert_eq!(cell.phi_relative_tolerance, 3.0e-2 * 1.0e-5);
    assert_eq!(cell.phi_absolute_tolerance, 3.0e-4 * 1.0e-5);
    assert!(!cell.trace_digest.is_empty());
    assert!(cell.attempts > 0);
    assert_eq!(cell.accepted_steps + cell.rejected_steps, cell.attempts);
}

#[test]
fn scientific_execution_identity_rejects_late_bound_or_malformed_values() {
    let mut malformed = execution_identity();
    malformed.scientific_execution_head_sha = "not-a-git-object".into();
    let error = run_a1_two_arm_receipt_cell(
        malformed,
        G4S5B0Family::HiresRamped,
        G4S5B0LinearToleranceArm::LegacyFixed,
    )
    .unwrap_err();
    assert!(error.to_string().contains("scientific_execution_head_sha"));
}
