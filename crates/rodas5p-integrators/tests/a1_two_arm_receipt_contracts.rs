use rodas5p_integrators::{
    A1ScientificExecutionIdentity, G4S5B0AttemptTraceReport, G4S5B0Family,
    G4S5B0FrozenFullEShadowReport, G4S5B0LinearToleranceArm, G4S5B0Profile, GmresKernelArm,
    K0_SOURCE_BASE_SHA, K0_SOURCE_BASE_TREE, K0_SOURCE_PULL_REQUEST,
    K0_STAGE_TELEMETRY_CELL_SCHEMA, K0ExecutionState, K0StageTelemetryExecutionIdentity,
    committed_g4_s5b0_linear_tolerance_arm, g4_s5b0_rjf_trace_digest,
    run_a1_post_a2a3_kernel_receipt_cell, run_a1_two_arm_receipt_cell,
    run_g4_s5b0_frozen_full_e_shadow_family,
    run_g4_s5b0_rjf_attempt_trace_family_with_linear_tolerance_arm, run_k0_stage_telemetry_cell,
    validate_stage_receipt_sequence,
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

fn k0_execution_identity() -> K0StageTelemetryExecutionIdentity {
    K0StageTelemetryExecutionIdentity {
        repository: "cosmosapjw-quantum/vigilode".into(),
        source_pull_request: K0_SOURCE_PULL_REQUEST,
        source_base_sha: K0_SOURCE_BASE_SHA.into(),
        source_base_tree: K0_SOURCE_BASE_TREE.into(),
        execution_head_sha: "7777777777777777777777777777777777777777".into(),
        execution_head_tree: "8888888888888888888888888888888888888888".into(),
        rust_version: "rustc 1.94.1".into(),
        cargo_version: "cargo 1.94.1".into(),
    }
}

#[test]
fn k0_cell_is_explicit_legacy_fixed_failure_preserving_and_a1_neutral() {
    let family = G4S5B0Family::RobertsonRamped;
    let kernel = GmresKernelArm::LegacyRestartedGmres;
    let plain = run_a1_post_a2a3_kernel_receipt_cell(execution_identity(), family, kernel)
        .expect("plain frozen A1 evidence");
    let cell = run_k0_stage_telemetry_cell(k0_execution_identity(), family, kernel);

    assert_eq!(cell.schema, K0_STAGE_TELEMETRY_CELL_SCHEMA);
    assert_eq!(cell.claim_class, "EXPLORATORY_NONAUTHORITATIVE");
    assert_eq!(cell.execution_state, K0ExecutionState::Complete);
    assert_eq!(cell.case_id, "robertson-ramped-n320");
    assert_eq!(cell.family, "robertson-ramped");
    assert_eq!(cell.kernel_arm, "legacy-restarted-gmres");
    assert_eq!(cell.tolerance.kind, "LegacyFixed");
    assert_eq!(cell.tolerance.rtol, 1.0e-10);
    assert_eq!(cell.tolerance.atol, 1.0e-12);
    assert!(!cell.stages.is_empty());
    assert!(!cell.solver_configuration_tokens.is_empty());
    validate_stage_receipt_sequence(&cell.stages).expect("exhaustive K0 stage sequence");
    assert!(cell.error.is_none());

    let observed = cell.receipt.expect("complete K0 A1 evidence");
    assert_eq!(observed.profile, plain.receipt.profile);
    assert_eq!(observed.family, plain.receipt.family);
    assert_eq!(observed.tolerance_arm, plain.tolerance_arm);
    assert_eq!(observed.trace_digest, plain.receipt.trace_digest);
    assert_eq!(observed.attempts, plain.receipt.attempts);
    assert_eq!(observed.accepted_steps, plain.receipt.accepted_steps);
    assert_eq!(observed.rejected_steps, plain.receipt.rejected_steps);
    assert_eq!(observed.event_rows, plain.receipt.event_rows);
    assert_eq!(
        observed.recommendation_rows,
        plain.receipt.recommendation_rows
    );
    assert_eq!(observed.hard_gates, plain.receipt.hard_gates);
    assert_eq!(
        committed_g4_s5b0_linear_tolerance_arm(),
        G4S5B0LinearToleranceArm::LegacyFixed
    );
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
    assert_eq!(cell.schema, "vigilode-a1-two-arm-atomic-cell-v2");
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

fn runtime_trace_digest(report: &G4S5B0FrozenFullEShadowReport) -> String {
    g4_s5b0_rjf_trace_digest(&G4S5B0AttemptTraceReport {
        schema: "g4-s5b0-rjf-attempt-trace-v1",
        status: "read-only-rjf-attempt-trace",
        profile: report.profile,
        switching_active: report.switching_active,
        committed_method: report.committed_method,
        attempt_rows: report.attempt_rows.clone(),
        accepted_rows: report.accepted_rows.clone(),
        trajectories: report.trajectories.clone(),
        limitations: Vec::new(),
    })
}

fn runtime_policy_snapshot(
    report: &G4S5B0FrozenFullEShadowReport,
) -> Vec<(String, bool, u64, u64, bool, bool)> {
    report
        .rows
        .iter()
        .map(|row| {
            (
                format!(
                    "{}:{}:{}:{}",
                    row.trajectory_id,
                    row.decision_accepted_step,
                    row.target_attempt_index,
                    row.target_accepted_steps_before
                ),
                row.recommended,
                row.budget_cap_jvp,
                row.prefix_speculative_jvp_after_target,
                row.target_r_attempt_accepted,
                row.target_r_recoverable_failure,
            )
        })
        .collect()
}

#[test]
fn unrecommended_hires_event_has_arm_bound_independent_audit_evidence() {
    let cell = run_a1_two_arm_receipt_cell(
        execution_identity(),
        G4S5B0Family::HiresRamped,
        G4S5B0LinearToleranceArm::LegacyFixed,
    )
    .expect("Hires receipt cell");
    let positive = cell
        .event_rows
        .iter()
        .find(|row| {
            !row.recommended
                && row.zeta34_signed_margin.is_some_and(|margin| margin > 0.0)
                && row.audit_unsafe == Some(true)
        })
        .expect("completed independent Hires positive control");
    assert!(!positive.shadow_full_e_completed);
    assert_eq!(positive.audit_arm, "legacy-fixed");
    assert_eq!(positive.audit_family, "hires-ramped");
    assert_eq!(positive.audit_event_key, positive.event_key);
    assert!(positive.audit_full_e_eligible);
    assert!(positive.audit_full_e_attempted);
    assert!(positive.audit_full_e_completed);
    assert_eq!(positive.audit_evidence_status, "complete");
    assert_eq!(positive.audit_full_e_locally_admissible, Some(false));
    assert!(positive.audit_full_e_total_error.is_some());
    assert!(positive.audit_full_e_failure.is_none());
    assert!(positive.audit_full_e_work.is_some());
}

#[test]
fn audit_execution_is_neutral_to_runtime_policy_budgets_controller_and_trace() {
    let committed_before = committed_g4_s5b0_linear_tolerance_arm();
    let before = run_g4_s5b0_frozen_full_e_shadow_family(
        G4S5B0Profile::EnforcedBudgetHoldout320,
        G4S5B0Family::RobertsonRamped,
    )
    .expect("runtime before audit");
    let before_digest = runtime_trace_digest(&before);
    let before_policy = runtime_policy_snapshot(&before);

    let cell = run_a1_two_arm_receipt_cell(
        execution_identity(),
        G4S5B0Family::RobertsonRamped,
        G4S5B0LinearToleranceArm::LegacyFixed,
    )
    .expect("receipt cell with independent audit");

    let after = run_g4_s5b0_frozen_full_e_shadow_family(
        G4S5B0Profile::EnforcedBudgetHoldout320,
        G4S5B0Family::RobertsonRamped,
    )
    .expect("runtime after audit");
    assert_eq!(committed_before, G4S5B0LinearToleranceArm::LegacyFixed);
    assert_eq!(committed_g4_s5b0_linear_tolerance_arm(), committed_before);
    assert_eq!(runtime_trace_digest(&after), before_digest);
    assert_eq!(runtime_policy_snapshot(&after), before_policy);
    assert_eq!(after.hard_gates, before.hard_gates);
    assert_eq!(cell.trace_digest, before_digest);
    assert_eq!(cell.attempts, before.attempt_rows.len());
    assert_eq!(
        cell.jvp_vectors,
        before
            .attempt_rows
            .iter()
            .map(|row| row.jvp_vectors)
            .sum::<u64>()
    );
    assert!(
        cell.event_rows
            .iter()
            .filter_map(|row| row.audit_full_e_work)
            .map(|work| work.jvp_vectors)
            .sum::<u64>()
            > 0
    );
}

#[test]
fn all_two_by_six_cells_have_explicit_complete_or_reasoned_audit_states() {
    for arm in G4S5B0LinearToleranceArm::ALL {
        let mut hires_positive = false;
        for family in G4S5B0Family::ALL {
            let cell = run_a1_two_arm_receipt_cell(execution_identity(), family, arm)
                .expect("arm-specific audit cell");
            assert_eq!(cell.arm, arm.as_str());
            assert_eq!(cell.family, family.as_str());
            for event in &cell.event_rows {
                assert_eq!(event.audit_arm, arm.as_str());
                assert_eq!(event.audit_family, family.as_str());
                assert_eq!(event.audit_event_key, event.event_key);
                if event.audit_full_e_eligible {
                    assert!(event.audit_full_e_attempted);
                    assert!(event.audit_full_e_completed);
                    assert_eq!(event.audit_evidence_status, "complete");
                    assert!(event.audit_full_e_total_error.is_some());
                    assert!(event.audit_full_e_locally_admissible.is_some());
                    assert!(event.audit_full_e_failure.is_none());
                    assert!(event.audit_full_e_work.is_some());
                    assert!(event.audit_unsafe.is_some());
                } else {
                    assert!(!event.audit_full_e_attempted);
                    assert!(!event.audit_full_e_completed);
                    assert_eq!(event.audit_evidence_status, "ineligible");
                    assert!(event.audit_full_e_failure.is_some());
                    assert!(event.audit_full_e_total_error.is_none());
                    assert!(event.audit_full_e_locally_admissible.is_none());
                    assert!(event.audit_full_e_work.is_none());
                    assert!(event.audit_unsafe.is_none());
                }
                if family == G4S5B0Family::HiresRamped
                    && !event.recommended
                    && event
                        .zeta34_signed_margin
                        .is_some_and(|margin| margin > 0.0)
                    && event.audit_unsafe == Some(true)
                {
                    hires_positive = true;
                }
            }
        }
        assert!(
            hires_positive,
            "missing Hires positive control for {}",
            arm.as_str()
        );
    }
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

    let mut invalidated = execution_identity();
    invalidated.execution_workflow_run_id = 32_906_175_896;
    let error = run_a1_two_arm_receipt_cell(
        invalidated,
        G4S5B0Family::HiresRamped,
        G4S5B0LinearToleranceArm::LegacyFixed,
    )
    .unwrap_err();
    assert!(error.to_string().contains("diagnostic-only"));
}
