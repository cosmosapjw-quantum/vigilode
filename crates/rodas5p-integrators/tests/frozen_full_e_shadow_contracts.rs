use rodas5p_integrators::{
    G4S5B0Family, G4S5B0Profile, V36_FROZEN_ZETA34_TAU, enforced_prefix_jvp_cap,
    frozen_full_e_shadow_recommended, run_g4_s5b0_frozen_full_e_shadow_family,
    run_g4_s5b0_rjf_attempt_trace_family,
};

#[test]
fn frozen_recommendation_uses_only_completed_prefix_and_sealed_zeta_threshold() {
    assert_eq!(V36_FROZEN_ZETA34_TAU.to_bits(), 0x402a_cb4c_426c_c526);
    assert!(frozen_full_e_shadow_recommended(
        true,
        false,
        false,
        Some(V36_FROZEN_ZETA34_TAU),
    ));
    assert!(frozen_full_e_shadow_recommended(
        true,
        false,
        false,
        Some(V36_FROZEN_ZETA34_TAU.next_down()),
    ));
    assert!(!frozen_full_e_shadow_recommended(
        true,
        false,
        false,
        Some(V36_FROZEN_ZETA34_TAU.next_up()),
    ));
    assert!(!frozen_full_e_shadow_recommended(true, false, false, None));
    assert!(!frozen_full_e_shadow_recommended(
        true,
        false,
        false,
        Some(f64::NAN),
    ));
    assert!(!frozen_full_e_shadow_recommended(
        true,
        false,
        false,
        Some(f64::INFINITY),
    ));
    assert!(!frozen_full_e_shadow_recommended(
        true,
        false,
        false,
        Some(f64::NEG_INFINITY),
    ));
    assert!(!frozen_full_e_shadow_recommended(
        false,
        false,
        false,
        Some(0.0),
    ));
    assert!(!frozen_full_e_shadow_recommended(
        true,
        true,
        false,
        Some(0.0),
    ));
    assert!(!frozen_full_e_shadow_recommended(
        true,
        false,
        true,
        Some(0.0),
    ));
}

#[test]
fn continuation_charges_total_ledger_without_reducing_the_next_prefix_cap() {
    let committed_rjf_jvp = 400_u64;
    let prefix_before = 20_u64;
    let total_before = 120_u64;
    assert_eq!(
        enforced_prefix_jvp_cap(committed_rjf_jvp, prefix_before),
        80
    );
    assert_eq!(enforced_prefix_jvp_cap(committed_rjf_jvp, total_before), 0);

    let charged_prefix = 5_u64;
    let charged_continuation = 100_u64;
    let prefix_after = prefix_before + charged_prefix;
    let total_after = total_before + charged_prefix + charged_continuation;
    assert_eq!(prefix_after, 25);
    assert_eq!(total_after, 225);
    assert_eq!(enforced_prefix_jvp_cap(committed_rjf_jvp, prefix_after), 75);
    assert_eq!(enforced_prefix_jvp_cap(committed_rjf_jvp, total_after), 0);
}

#[test]
fn retained_level2_shadow_is_complete_charged_safe_and_rjf_identical() {
    let report = run_g4_s5b0_frozen_full_e_shadow_family(
        G4S5B0Profile::StageGrowthCalibration96,
        G4S5B0Family::RobertsonRamped,
    )
    .unwrap();
    let baseline = run_g4_s5b0_rjf_attempt_trace_family(
        G4S5B0Profile::StageGrowthCalibration96,
        G4S5B0Family::RobertsonRamped,
    )
    .unwrap();

    assert_eq!(report.schema, "g4-s5b0-frozen-full-e-shadow-v1");
    assert_eq!(report.status, "complete");
    assert!(!report.switching_active);
    assert_eq!(
        report.frozen_zeta34_tau.to_bits(),
        V36_FROZEN_ZETA34_TAU.to_bits()
    );
    assert_eq!(report.recommendations, 2);
    assert_eq!(report.retained_level2_resumptions, 2);
    assert_eq!(report.shadow_full_e_completions, 2);
    assert_eq!(report.shadow_full_e_failures, 0);
    assert_eq!(report.unsafe_recommendations, 0);
    assert_eq!(report.budget_breaches, 0);
    assert_eq!(report.prefix_speculative_work.jvp_vectors, 42);
    assert_eq!(report.continuation_work.jvp_vectors, 24);
    assert_eq!(report.total_speculative_work.jvp_vectors, 66);
    assert_eq!(
        report.committed_rjf_jvp_vectors,
        report
            .attempt_rows
            .iter()
            .map(|row| row.jvp_vectors)
            .sum::<u64>()
    );
    assert_eq!(
        report.realized_prefix_over_committed_rjf_jvp.to_bits(),
        (42.0 / report.committed_rjf_jvp_vectors as f64).to_bits()
    );
    assert_eq!(
        report
            .realized_continuation_over_committed_rjf_jvp
            .to_bits(),
        (24.0 / report.committed_rjf_jvp_vectors as f64).to_bits()
    );
    assert_eq!(
        report
            .realized_total_speculative_over_committed_rjf_jvp
            .to_bits(),
        (66.0 / report.committed_rjf_jvp_vectors as f64).to_bits()
    );
    let mut aggregate_roundtrip = report.prefix_speculative_work;
    aggregate_roundtrip.accumulate(report.continuation_work);
    assert_eq!(aggregate_roundtrip, report.total_speculative_work);
    assert_eq!(report.attempt_rows.len(), 29);
    assert_eq!(report.accepted_rows.len(), 27);
    assert!(report.rjf_parity.passed);
    assert!(report.rjf_parity.attempt_rows_exact_excluding_wall);
    assert!(report.rjf_parity.accepted_rows_exact_excluding_wall);
    assert!(report.rjf_parity.trajectories_exact);
    assert!(report.hard_gates.passed);
    assert!(report.hard_gates.all_rjf_trajectories_successful);
    assert!(report.hard_gates.rjf_trace_exact_excluding_wall);
    assert!(report.hard_gates.zero_budget_breaches);
    assert!(report.hard_gates.prefix_transactions_resolved);
    assert!(report.hard_gates.zero_continuation_failures);
    assert!(report.hard_gates.zero_unsafe_recommendations);
    assert!(report.hard_gates.work_ledgers_exact);
    assert!(report.hard_gates.realized_work_ratios_finite);
    assert!(report.hard_gates.resume_cardinality_exact);
    assert!(report.hard_gates.shadow_implicit_expensive_work_zero);
    assert!(report.hard_gates.active_switching_false);

    assert_eq!(report.attempt_rows.len(), baseline.attempt_rows.len());
    for (shadow, baseline) in report.attempt_rows.iter().zip(&baseline.attempt_rows) {
        let mut shadow = shadow.clone();
        let mut baseline = baseline.clone();
        shadow.wall_seconds = 0.0;
        baseline.wall_seconds = 0.0;
        assert_eq!(shadow, baseline);
    }
    assert_eq!(report.accepted_rows.len(), baseline.accepted_rows.len());
    for (shadow, baseline) in report.accepted_rows.iter().zip(&baseline.accepted_rows) {
        let mut shadow = shadow.clone();
        let mut baseline = baseline.clone();
        shadow.rodas_wall_seconds = 0.0;
        baseline.rodas_wall_seconds = 0.0;
        assert_eq!(shadow, baseline);
    }
    assert_eq!(report.trajectories, baseline.trajectories);

    assert_eq!(report.rows.len(), 2);
    assert_eq!(
        report
            .rows
            .iter()
            .map(|row| row.target_attempt_index)
            .collect::<Vec<_>>(),
        vec![9, 24],
    );
    let mut expected_prefix_before = 0_u64;
    let mut expected_total_before = 0_u64;
    for row in &report.rows {
        assert!(row.recommended);
        assert!(row.retained_level2_resumed);
        assert!(row.shadow_full_e_completed);
        assert!(row.shadow_full_e_failure.is_none());
        assert!(row.shadow_full_e_locally_admissible);
        assert!(row.work_roundtrip_exact);
        assert_eq!(
            row.prefix_speculative_jvp_before_target,
            expected_prefix_before
        );
        assert_eq!(
            row.total_speculative_jvp_before_target,
            expected_total_before
        );

        let prefix = row.prefix_work.unwrap();
        let continuation = row.continuation_work.unwrap();
        let full = row.shadow_full_e_work.unwrap();
        let mut reconstructed = prefix;
        reconstructed.accumulate(continuation);
        assert_eq!(reconstructed, full);
        assert_eq!(full.checked_delta(prefix), Some(continuation));
        assert_eq!(
            row.prefix_speculative_jvp_after_target,
            expected_prefix_before + prefix.jvp_vectors,
        );
        assert_eq!(
            row.total_speculative_jvp_after_target,
            expected_total_before + full.jvp_vectors,
        );
        assert_eq!(row.target_rjf_jvp_vectors, Some(118));
        assert_eq!(prefix.jacobian_builds, 0);
        assert_eq!(prefix.direct_factorizations, 0);
        assert_eq!(prefix.nonlinear_iterations, 0);
        assert_eq!(continuation.jacobian_builds, 0);
        assert_eq!(continuation.direct_factorizations, 0);
        assert_eq!(continuation.nonlinear_iterations, 0);

        expected_prefix_before = row.prefix_speculative_jvp_after_target;
        expected_total_before = row.total_speculative_jvp_after_target;
    }
    assert_eq!(expected_prefix_before, 42);
    assert_eq!(expected_total_before, 66);
    assert!(report.trajectories.iter().all(|row| {
        row.success
            && row.explicit_jacobian_builds == 0
            && row.direct_factorizations == 0
            && row.newton_iterations == 0
    }));
}
