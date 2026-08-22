use rodas5p_integrators::{
    G4S5B0Family, G4S5B0Profile, V37_CONTINUATION_JVP_CAP,
    run_g4_s5b0_v37_continuation_transaction_family,
};

#[test]
fn v37_completing_family_preserves_frozen_policy_and_rjf_authority() {
    let report = run_g4_s5b0_v37_continuation_transaction_family(
        G4S5B0Profile::StageGrowthCalibration96,
        G4S5B0Family::RobertsonRamped,
    )
    .unwrap();

    assert_eq!(report.schema, "g4-s5b0-v37-continuation-transaction-v1");
    assert_eq!(report.status, "complete");
    assert_eq!(report.profile, "stage-growth-calibration-96");
    assert!(!report.switching_active);
    assert_eq!(report.persistence_k, 3);
    assert_eq!(report.absolute_prefix_jvp_cap, 80);
    assert_eq!(report.absolute_continuation_jvp_cap, 80);
    assert_eq!(V37_CONTINUATION_JVP_CAP, 80);
    assert_eq!(report.frozen_cumulative_prefix_budget_fraction, 0.25);
    assert_eq!(report.frozen_zeta34_tau, 13.39706618860016);
    assert_eq!(report.recommendations, 2);
    assert_eq!(report.retained_level2_resumptions, 2);
    assert_eq!(report.shadow_full_e_completions, 2);
    assert_eq!(report.continuation_budget_exhaustions, 0);
    assert_eq!(report.shadow_full_e_failures, 0);
    assert_eq!(report.unsafe_recommendations, 0);
    assert_eq!(report.prefix_budget_breaches, 0);
    assert_eq!(report.continuation_budget_breaches, 0);
    assert!(report.rjf_parity.passed);
    assert!(report.hard_gates.passed);
    assert!(report.hard_gates.continuation_transactions_resolved);
    assert!(report.hard_gates.zero_continuation_budget_breaches);
    assert!(report.hard_gates.zero_continuation_numerical_failures);
    assert!(report.hard_gates.exhausted_rows_emit_no_endpoint_or_labels);
    assert!(report.hard_gates.shadow_implicit_expensive_work_zero);
    assert!(report.hard_gates.active_switching_false);

    let recommended = report
        .rows
        .iter()
        .filter(|row| row.recommended)
        .collect::<Vec<_>>();
    assert_eq!(recommended.len(), 2);
    for row in recommended {
        assert_eq!(row.continuation_jvp_cap, V37_CONTINUATION_JVP_CAP);
        assert_eq!(row.continuation_outcome, "complete");
        assert!(!row.continuation_budget_exhausted);
        assert!(row.shadow_full_e_completed);
        assert!(row.shadow_full_e_total_error.is_some());
        assert_eq!(row.shadow_full_e_locally_admissible, Some(true));
        assert!(row.shadow_full_e_failure.is_none());
        assert!(row.work_roundtrip_exact);
        let continuation = row.continuation_work.unwrap();
        assert_eq!(
            row.continuation_used_jvp_vectors,
            Some(continuation.jvp_vectors)
        );
        assert!(continuation.jvp_vectors < V37_CONTINUATION_JVP_CAP);
        assert_eq!(continuation.jacobian_builds, 0);
        assert_eq!(continuation.direct_factorizations, 0);
        assert_eq!(continuation.nonlinear_iterations, 0);
    }
}

#[test]
#[ignore = "long consumed N=192 replay; executed by the optimized v3.7 replay gate"]
fn v37_exhaustion_is_a_charged_abstention_without_endpoint_or_failure_label() {
    let report = run_g4_s5b0_v37_continuation_transaction_family(
        G4S5B0Profile::StageGrowthCalibration192,
        G4S5B0Family::SemilinearAdvectionDiffusionRamped,
    )
    .unwrap();

    assert_eq!(report.recommendations, 2);
    assert_eq!(report.retained_level2_resumptions, 2);
    assert_eq!(report.shadow_full_e_completions, 1);
    assert_eq!(report.continuation_budget_exhaustions, 1);
    assert_eq!(report.shadow_full_e_failures, 0);
    assert_eq!(report.unsafe_recommendations, 0);
    assert_eq!(report.continuation_budget_breaches, 0);
    assert!(report.rjf_parity.passed);
    assert!(report.hard_gates.passed);

    let exhausted = report
        .rows
        .iter()
        .find(|row| row.continuation_budget_exhausted)
        .expect("the sealed N=192 semilinear outlier must exhaust");
    assert_eq!(exhausted.target_attempt_index, 12);
    assert!(exhausted.recommended);
    assert!(exhausted.retained_level2_resumed);
    assert_eq!(exhausted.continuation_outcome, "budget-exhausted");
    assert_eq!(exhausted.continuation_jvp_cap, 80);
    assert_eq!(exhausted.continuation_used_jvp_vectors, Some(80));
    assert_eq!(exhausted.continuation_work.unwrap().jvp_vectors, 80);
    assert!(!exhausted.shadow_full_e_completed);
    assert!(exhausted.shadow_full_e_total_error.is_none());
    assert!(exhausted.shadow_full_e_locally_admissible.is_none());
    assert!(exhausted.shadow_full_e_failure.is_none());
    assert!(exhausted.work_roundtrip_exact);

    let completed = report
        .rows
        .iter()
        .find(|row| row.recommended && !row.continuation_budget_exhausted)
        .expect("the second frozen recommendation must complete");
    assert_eq!(completed.target_attempt_index, 18);
    assert_eq!(completed.continuation_outcome, "complete");
    assert_eq!(completed.continuation_work.unwrap().jvp_vectors, 36);
    assert!(completed.shadow_full_e_completed);
    assert_eq!(completed.shadow_full_e_locally_admissible, Some(true));

    assert_eq!(report.continuation_work.jvp_vectors, 116);
    assert_eq!(
        report.total_speculative_work.jvp_vectors,
        report.prefix_speculative_work.jvp_vectors + 116
    );
}
