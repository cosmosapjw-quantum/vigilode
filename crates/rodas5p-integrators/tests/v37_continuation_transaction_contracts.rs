use rodas5p_integrators::{
    G4S5B0Family, G4S5B0Profile, V37_CONTINUATION_JVP_CAP,
    run_g4_s5b0_v37_continuation_transaction_family,
    run_g4_s5b0_v37_continuation_transaction_family_with_cap,
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

/// The consumed N=192 semilinear replay (audit F-003).
fn consumed_n192_semilinear_report() -> rodas5p_integrators::G4S5B0V37ContinuationTransactionReport
{
    run_g4_s5b0_v37_continuation_transaction_family(
        G4S5B0Profile::StageGrowthCalibration192,
        G4S5B0Family::SemilinearAdvectionDiffusionRamped,
    )
    .unwrap()
}

/// Invariants that hold for every recommended row and the report,
/// whatever the trajectory.
fn assert_transaction_invariants(
    report: &rodas5p_integrators::G4S5B0V37ContinuationTransactionReport,
) {
    assert_eq!(report.shadow_full_e_failures, 0);
    assert_eq!(report.unsafe_recommendations, 0);
    assert_eq!(report.continuation_budget_breaches, 0);
    assert!(report.rjf_parity.passed);
    assert!(report.hard_gates.passed);
    assert_eq!(
        report.recommendations,
        report.shadow_full_e_completions + report.continuation_budget_exhaustions
    );
    for row in report.rows.iter().filter(|row| row.recommended) {
        assert!(row.retained_level2_resumed);
        assert!(row.work_roundtrip_exact);
        if row.continuation_budget_exhausted {
            // A charged abstention: the whole cap is spent and recorded, and
            // no endpoint, error or failure label is emitted.
            assert_eq!(row.continuation_outcome, "budget-exhausted");
            assert_eq!(
                row.continuation_used_jvp_vectors,
                Some(row.continuation_jvp_cap)
            );
            assert_eq!(
                row.continuation_work.unwrap().jvp_vectors,
                row.continuation_jvp_cap
            );
            assert!(!row.shadow_full_e_completed);
            assert!(row.shadow_full_e_total_error.is_none());
            assert!(row.shadow_full_e_locally_admissible.is_none());
            assert!(row.shadow_full_e_failure.is_none());
        } else {
            assert_eq!(row.continuation_outcome, "complete");
            assert!(row.continuation_work.unwrap().jvp_vectors < row.continuation_jvp_cap);
            assert!(row.shadow_full_e_completed);
            assert_eq!(row.shadow_full_e_locally_admissible, Some(true));
        }
    }
    let row_continuation = report
        .rows
        .iter()
        .filter_map(|row| row.continuation_work)
        .map(|work| work.jvp_vectors)
        .sum::<u64>();
    assert_eq!(report.continuation_work.jvp_vectors, row_continuation);
    assert_eq!(
        report.total_speculative_work.jvp_vectors,
        report.prefix_speculative_work.jvp_vectors + report.continuation_work.jvp_vectors
    );
}

/// Exhaustion is a charged abstention without endpoint or failure label.
///
/// Until the time-normalized phi augmentation of the 2026-09-30 audit, the
/// consumed N=192 semilinear replay exhausted its 80-JVP continuation at the
/// frozen cap. It no longer does (see the V4 snapshot and
/// `ADDENDUM_20260930_PHI_NORMALIZATION.md`), and no calibration replay
/// does, so the abstention path is exercised with a reduced cap of 24,
/// below the 48 JVP vectors the recommended row needs.
#[test]
#[ignore = "long consumed N=192 replay; run by the ignored-tests CI job in the measurement profile"]
fn v37_exhaustion_is_a_charged_abstention_without_endpoint_or_failure_label() {
    let at_frozen_cap = consumed_n192_semilinear_report();
    assert_eq!(
        at_frozen_cap.absolute_continuation_jvp_cap,
        V37_CONTINUATION_JVP_CAP
    );
    assert_transaction_invariants(&at_frozen_cap);

    let reduced = run_g4_s5b0_v37_continuation_transaction_family_with_cap(
        G4S5B0Profile::StageGrowthCalibration192,
        G4S5B0Family::SemilinearAdvectionDiffusionRamped,
        24,
    )
    .unwrap();
    assert_transaction_invariants(&reduced);
    assert!(reduced.continuation_budget_exhaustions >= 1);
    let exhausted = reduced
        .rows
        .iter()
        .find(|row| row.continuation_budget_exhausted)
        .expect("a cap below the completing work must exhaust");
    assert_eq!(exhausted.continuation_jvp_cap, 24);
    // The prefix and the committed trajectory do not depend on the cap.
    assert_eq!(
        reduced.prefix_speculative_work,
        at_frozen_cap.prefix_speculative_work
    );
    assert_eq!(reduced.recommendations, at_frozen_cap.recommendations);
}

/// Trajectory literals of the same replay, checked against the latest
/// versioned snapshot. The sealed v3.7 literals (18, 36, 116) are tombstoned
/// in `research/generic_timing_replication_continuation_transaction_v37/results/
/// V37_TRAJECTORY_LITERALS_TOMBSTONE_20260929.json`; a numerical change adds a
/// new snapshot file, it never edits an old one. V2 holds the audit-base
/// values, V3 the integration-branch values (WU-3 and WU-10 move the JVP
/// counts), V4 the values after the time-normalized phi augmentation, under
/// which the replay recommends once and completes (see its `attribution`).
#[test]
#[ignore = "long consumed N=192 replay; run by the ignored-tests CI job in the measurement profile"]
fn v37_trajectory_literals_match_the_latest_snapshot() {
    let snapshot: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/generic_timing_replication_continuation_transaction_v37/results/V37_TRAJECTORY_SNAPSHOT_V4_20260930.json"
    ))
    .unwrap();
    let expected = &snapshot["values"];
    let report = consumed_n192_semilinear_report();
    let recommended = report
        .rows
        .iter()
        .filter(|row| row.recommended)
        .map(|row| {
            serde_json::json!({
                "target_attempt_index": row.target_attempt_index,
                "continuation_outcome": row.continuation_outcome,
                "continuation_jvp_vectors": row.continuation_work.map(|work| work.jvp_vectors),
            })
        })
        .collect::<Vec<_>>();
    let observed = serde_json::json!({
        "recommendations": report.recommendations,
        "shadow_full_e_completions": report.shadow_full_e_completions,
        "continuation_budget_exhaustions": report.continuation_budget_exhaustions,
        "recommended_rows": recommended,
        "report_continuation_jvp_vectors": report.continuation_work.jvp_vectors,
        "report_prefix_jvp_vectors": report.prefix_speculative_work.jvp_vectors,
    });
    assert_eq!(
        &observed, expected,
        "the trajectory moved; add a new versioned snapshot with provenance"
    );
}
