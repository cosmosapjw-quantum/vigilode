use rodas5p_integrators::{
    G4S5B0Family, G4S5B0Profile, enforced_prefix_jvp_cap,
    run_g4_s5b0_enforced_prefix_budget_family, run_g4_s5b0_rjf_attempt_trace_family,
};

#[test]
fn remaining_budget_is_the_frozen_absolute_and_cumulative_minimum() {
    assert_eq!(enforced_prefix_jvp_cap(29_007, 65), 80);
    assert_eq!(enforced_prefix_jvp_cap(400, 99), 1);
    assert_eq!(enforced_prefix_jvp_cap(400, 100), 0);
    assert_eq!(enforced_prefix_jvp_cap(3, 0), 0);
    assert_eq!(enforced_prefix_jvp_cap(u64::MAX, u64::MAX), 0);
}

#[test]
fn enforced_budget_runner_is_read_only_and_never_records_a_breach() {
    let audited = run_g4_s5b0_enforced_prefix_budget_family(
        G4S5B0Profile::StageGrowthCalibration96,
        G4S5B0Family::RobertsonRamped,
    )
    .unwrap();
    let baseline = run_g4_s5b0_rjf_attempt_trace_family(
        G4S5B0Profile::StageGrowthCalibration96,
        G4S5B0Family::RobertsonRamped,
    )
    .unwrap();

    assert_eq!(audited.schema, "g4-s5b0-enforced-prefix-budget-v1");
    assert!(!audited.switching_active);
    assert_eq!(audited.runtime_full_e_continuations, 0);
    assert_eq!(audited.budget_breaches, 0);
    assert_eq!(audited.attempt_rows.len(), baseline.attempt_rows.len());
    for (left, right) in audited.attempt_rows.iter().zip(&baseline.attempt_rows) {
        assert_eq!(left.trajectory_id, right.trajectory_id);
        assert_eq!(left.attempt_index, right.attempt_index);
        assert_eq!(left.accepted_steps_before, right.accepted_steps_before);
        assert_eq!(left.t_start.to_bits(), right.t_start.to_bits());
        assert_eq!(left.h.to_bits(), right.h.to_bits());
        assert_eq!(
            left.error_norm.map(f64::to_bits),
            right.error_norm.map(f64::to_bits)
        );
        assert_eq!(left.accepted, right.accepted);
        assert_eq!(left.recoverable_failure, right.recoverable_failure);
        assert_eq!(left.failure, right.failure);
        assert_eq!(left.rhs_evaluations, right.rhs_evaluations);
        assert_eq!(left.jvp_vectors, right.jvp_vectors);
        assert_eq!(left.linear_matvecs, right.linear_matvecs);
    }
    assert_eq!(audited.accepted_rows.len(), baseline.accepted_rows.len());
    for (left, right) in audited.accepted_rows.iter().zip(&baseline.accepted_rows) {
        assert_eq!(left.trajectory_id, right.trajectory_id);
        assert_eq!(left.step_index, right.step_index);
        assert_eq!(left.t_start.to_bits(), right.t_start.to_bits());
        assert_eq!(left.h.to_bits(), right.h.to_bits());
        assert_eq!(
            left.rodas_embedded_error.to_bits(),
            right.rodas_embedded_error.to_bits()
        );
        assert_eq!(left.rodas_rhs_evaluations, right.rodas_rhs_evaluations);
        assert_eq!(left.rodas_jvp_vectors, right.rodas_jvp_vectors);
        assert_eq!(left.rodas_linear_matvecs, right.rodas_linear_matvecs);
    }
    assert_eq!(audited.trajectories, baseline.trajectories);
    assert!(audited.rows.iter().all(|row| {
        row.actual_prefix_jvp_vectors.unwrap_or(0) <= row.budget_cap_jvp
            && !row.budget_breached
            && (!row.budget_exhausted
                || (!row.prefix_succeeded
                    && row.quadratic_drift_zeta34.is_none()
                    && !row.audit_full_e_completed))
    }));
}

#[test]
fn fresh_enforced_budget_holdout320_profile_is_frozen_before_output() {
    let profile = rodas5p_integrators::G4S5B0Profile::EnforcedBudgetHoldout320;
    assert_eq!(profile.as_str(), "enforced-budget-holdout-320");
    assert_eq!(profile.dimensions(), &[320]);
    assert_eq!(profile.tolerances(), (1.0e-7, 1.0e-5));
}
