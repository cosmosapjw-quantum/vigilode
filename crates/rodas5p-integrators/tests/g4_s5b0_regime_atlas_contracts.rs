use rodas5p_integrators::{
    G4S5B0Family, G4S5B0Profile, run_g4_s5b0_regime_atlas, run_g4_s5b0_rjf_attempt_trace_family,
    run_g4_s5b0_rjf_only, run_g4_s5b0_rjf_only_family,
};

#[test]
fn smoke_atlas_is_read_only_and_matrix_free() {
    let report = run_g4_s5b0_regime_atlas(G4S5B0Profile::Smoke).unwrap();
    assert!(!report.switching_active);
    assert_eq!(
        report.committed_method,
        "protected-sequential-matrix-free-rodas5p"
    );
    assert_eq!(report.trajectories.len(), 6);
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.rodas_embedded_error.is_finite())
    );
    assert!(
        report
            .trajectories
            .iter()
            .all(|row| row.explicit_jacobian_builds == 0 && row.direct_factorizations == 0)
    );
}

#[test]
fn smoke_atlas_contains_all_transition_families() {
    let report = run_g4_s5b0_regime_atlas(G4S5B0Profile::Smoke).unwrap();
    let families = report
        .trajectories
        .iter()
        .map(|row| row.family.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(families.len(), 6);
    for required in [
        "robertson-ramped",
        "hires-ramped",
        "van-der-pol-ramped",
        "rotating-nonnormal",
        "nonautonomous-stiff-forcing",
        "semilinear-advection-diffusion-ramped",
    ] {
        assert!(families.contains(required));
    }
}

#[test]
fn split_profiles_freeze_dimension_and_canonical_tolerances() {
    assert_eq!(G4S5B0Profile::Calibration128.dimensions(), &[128]);
    assert_eq!(G4S5B0Profile::Holdout512.dimensions(), &[512]);
    assert_eq!(G4S5B0Profile::Calibration128.as_str(), "calibration-128");
    assert_eq!(G4S5B0Profile::Holdout512.as_str(), "holdout-512");
    assert!(G4S5B0Profile::Calibration128.uses_canonical_tolerances());
    assert!(G4S5B0Profile::Holdout512.uses_canonical_tolerances());
}

#[test]
fn v29_stage_growth_profiles_are_fresh_and_freeze_tolerances() {
    assert_eq!(G4S5B0Profile::StageGrowthCalibration96.dimensions(), &[96]);
    assert_eq!(
        G4S5B0Profile::StageGrowthCalibration256.dimensions(),
        &[256]
    );
    assert_eq!(G4S5B0Profile::StageGrowthHoldout384.dimensions(), &[384]);
    assert_eq!(
        G4S5B0Profile::StageGrowthCalibration96.tolerances(),
        (3.0e-8, 3.0e-6)
    );
    assert_eq!(
        G4S5B0Profile::StageGrowthCalibration256.tolerances(),
        (3.0e-7, 3.0e-5)
    );
    assert_eq!(
        G4S5B0Profile::StageGrowthHoldout384.tolerances(),
        (7.0e-8, 7.0e-6)
    );
    assert!(!G4S5B0Profile::StageGrowthCalibration96.uses_canonical_tolerances());
    assert!(!G4S5B0Profile::StageGrowthCalibration256.uses_canonical_tolerances());
    assert!(!G4S5B0Profile::StageGrowthHoldout384.uses_canonical_tolerances());
}

#[test]
fn rjf_only_smoke_skips_every_exponential_shadow() {
    let report = run_g4_s5b0_rjf_only(G4S5B0Profile::Smoke).unwrap();
    assert_eq!(report.schema, "g4-s5b0-rjf-only-regime-replay-v1");
    assert_eq!(report.trajectories.len(), 6);
    assert!(!report.rows.is_empty());
    assert!(report.rows.iter().all(|row| {
        !row.exponential_completed
            && row.exponential_total_error.is_none()
            && row.exponential_wall_seconds.is_none()
            && row.exponential_prefix_wall_seconds.is_none()
            && row.exponential_rhs_evaluations.is_none()
            && row.exponential_jvp_vectors.is_none()
            && row.exponential_failure.is_none()
    }));
    assert!(report.trajectories.iter().all(|row| {
        row.explicit_jacobian_builds == 0
            && row.direct_factorizations == 0
            && row.newton_iterations == 0
    }));
}

#[test]
fn family_shard_registry_matches_the_six_atlas_families() {
    let names = G4S5B0Family::ALL.map(G4S5B0Family::as_str);
    assert_eq!(
        names,
        [
            "robertson-ramped",
            "hires-ramped",
            "van-der-pol-ramped",
            "rotating-nonnormal",
            "nonautonomous-stiff-forcing",
            "semilinear-advection-diffusion-ramped",
        ]
    );
}

#[test]
fn attempt_trace_smoke_reconstructs_committed_rjf_rows_and_retains_trials() {
    let report =
        run_g4_s5b0_rjf_attempt_trace_family(G4S5B0Profile::Smoke, G4S5B0Family::RobertsonRamped)
            .unwrap();
    assert_eq!(report.schema, "g4-s5b0-rjf-attempt-trace-v1");
    assert!(!report.switching_active);
    assert_eq!(report.trajectories.len(), 1);
    assert_eq!(
        report.attempt_rows.len(),
        report
            .trajectories
            .iter()
            .map(|row| row.attempts)
            .sum::<usize>()
    );
    assert_eq!(
        report.accepted_rows.len(),
        report
            .trajectories
            .iter()
            .map(|row| row.accepted_steps)
            .sum::<usize>()
    );
    assert_eq!(
        report
            .attempt_rows
            .iter()
            .filter(|row| row.accepted)
            .count(),
        report.accepted_rows.len()
    );
    assert!(report.attempt_rows.iter().all(|row| {
        row.rhs_evaluations > 0
            && row.jvp_vectors > 0
            && row.linear_matvecs > 0
            && row.wall_seconds >= 0.0
    }));

    let baseline =
        run_g4_s5b0_rjf_only_family(G4S5B0Profile::Smoke, G4S5B0Family::RobertsonRamped).unwrap();
    assert_eq!(report.accepted_rows.len(), baseline.rows.len());
    for (attempt_row, baseline_row) in report.accepted_rows.iter().zip(&baseline.rows) {
        assert_eq!(attempt_row.trajectory_id, baseline_row.trajectory_id);
        assert_eq!(attempt_row.step_index, baseline_row.step_index);
        assert_eq!(
            attempt_row.t_start.to_bits(),
            baseline_row.t_start.to_bits()
        );
        assert_eq!(attempt_row.h.to_bits(), baseline_row.h.to_bits());
        assert_eq!(
            attempt_row.rodas_embedded_error.to_bits(),
            baseline_row.rodas_embedded_error.to_bits()
        );
        assert_eq!(
            attempt_row.rodas_rhs_evaluations,
            baseline_row.rodas_rhs_evaluations
        );
        assert_eq!(
            attempt_row.rodas_jvp_vectors,
            baseline_row.rodas_jvp_vectors
        );
        assert_eq!(
            attempt_row.rodas_linear_matvecs,
            baseline_row.rodas_linear_matvecs
        );
    }
}

#[test]
fn actual_level1_prefix_runner_is_read_only_and_targets_first_runtime_proposals() {
    use rodas5p_integrators::{G4S5B0PrefixProbePolicy, run_g4_s5b0_actual_level1_prefix_family};

    let probed = run_g4_s5b0_actual_level1_prefix_family(
        G4S5B0Profile::Calibration128,
        G4S5B0Family::RobertsonRamped,
        G4S5B0PrefixProbePolicy::FrozenK1Comparator,
    )
    .unwrap();
    let baseline = run_g4_s5b0_rjf_attempt_trace_family(
        G4S5B0Profile::Calibration128,
        G4S5B0Family::RobertsonRamped,
    )
    .unwrap();

    assert!(!probed.switching_active);
    assert_eq!(probed.full_e_continuations, 0);
    assert_eq!(probed.attempt_rows.len(), baseline.attempt_rows.len());
    for (left, right) in probed.attempt_rows.iter().zip(&baseline.attempt_rows) {
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
    assert_eq!(probed.accepted_rows.len(), baseline.accepted_rows.len());
    for (left, right) in probed.accepted_rows.iter().zip(&baseline.accepted_rows) {
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
    assert_eq!(probed.trajectories, baseline.trajectories);
    assert_eq!(probed.prefix_rows.len(), 4);

    for row in &probed.prefix_rows {
        assert_eq!(row.policy, "frozen-k1-comparator");
        assert!(
            row.prefix_succeeded,
            "prefix failure: {:?}",
            row.prefix_failure
        );
        let prefix_report = row.prefix_report.as_ref().unwrap();
        assert_eq!(prefix_report.method, "pexprb54s4-fused-level1");
        assert_eq!(prefix_report.logical_critical_depth, 1);
        assert_eq!(prefix_report.fused_phi_reports.len(), 1);
        let telemetry = prefix_report.early_flow_defect.as_ref().unwrap();
        assert!(telemetry.tolerance_scaled_defect_wrms.is_some());
        assert_eq!(telemetry.norm_component_count, 128);
        assert!(!row.full_e_continued);
        assert!(row.prefix_wall_seconds >= 0.0);
        assert_eq!(prefix_report.work.jacobian_builds, 0);
        assert_eq!(prefix_report.work.direct_factorizations, 0);
    }
}

#[test]
fn actual_level2_prefix_runner_is_read_only_and_exposes_later_stage_defects() {
    use rodas5p_integrators::{G4S5B0PrefixProbePolicy, run_g4_s5b0_actual_level2_prefix_family};

    let probed = run_g4_s5b0_actual_level2_prefix_family(
        G4S5B0Profile::Calibration128,
        G4S5B0Family::RobertsonRamped,
        G4S5B0PrefixProbePolicy::FrozenK1Comparator,
    )
    .unwrap();
    let baseline = run_g4_s5b0_rjf_attempt_trace_family(
        G4S5B0Profile::Calibration128,
        G4S5B0Family::RobertsonRamped,
    )
    .unwrap();

    assert_eq!(probed.schema, "g4-s5b0-actual-pexprb-level2-prefix-v1");
    assert!(!probed.switching_active);
    assert_eq!(probed.full_e_continuations, 0);
    assert_eq!(probed.attempt_rows.len(), baseline.attempt_rows.len());
    for (left, right) in probed.attempt_rows.iter().zip(&baseline.attempt_rows) {
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
    assert_eq!(probed.accepted_rows.len(), baseline.accepted_rows.len());
    for (left, right) in probed.accepted_rows.iter().zip(&baseline.accepted_rows) {
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
    assert_eq!(probed.trajectories, baseline.trajectories);
    assert_eq!(probed.prefix_rows.len(), 4);

    for row in &probed.prefix_rows {
        assert!(
            row.prefix_succeeded,
            "prefix failure: {:?}",
            row.prefix_failure
        );
        assert!(!row.full_e_continued);
        let report = row.prefix_report.as_ref().unwrap();
        assert_eq!(report.method, "pexprb54s4-fused-level2");
        assert_eq!(report.logical_critical_depth, 2);
        assert_eq!(report.level2_fused_phi_reports.len(), 2);
        assert_eq!(report.level1_report.logical_critical_depth, 1);
        assert!(
            report
                .stage3_flow_defect
                .as_ref()
                .unwrap()
                .tolerance_scaled_defect_wrms
                .is_some()
        );
        assert!(
            report
                .stage4_flow_defect
                .as_ref()
                .unwrap()
                .tolerance_scaled_defect_wrms
                .is_some()
        );
        assert!(report.level2_incremental_work.rhs_evaluations > 0);
        assert!(report.level2_incremental_work.jvp_vectors > 0);
        assert_eq!(report.cumulative_work.jacobian_builds, 0);
        assert_eq!(report.cumulative_work.direct_factorizations, 0);
    }
}

#[test]
fn v29_stage_growth_safety_audit_is_read_only_budgeted_and_explicit() {
    use rodas5p_integrators::run_g4_s5b0_stage_growth_safety_audit_family;

    let audited = run_g4_s5b0_stage_growth_safety_audit_family(
        G4S5B0Profile::StageGrowthCalibration96,
        G4S5B0Family::RobertsonRamped,
    )
    .unwrap();
    let baseline = run_g4_s5b0_rjf_attempt_trace_family(
        G4S5B0Profile::StageGrowthCalibration96,
        G4S5B0Family::RobertsonRamped,
    )
    .unwrap();

    assert_eq!(audited.schema, "g4-s5b0-stage-growth-safety-audit-v1");
    assert!(!audited.switching_active);
    assert_eq!(audited.runtime_full_e_continuations, 0);
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
    assert_eq!(audited.budget_breaches, 0);
    assert!(audited.rows.iter().all(|row| !row.runtime_full_e_continued));
    assert!(audited.rows.iter().all(|row| {
        !row.budget_admitted
            || (row.prefix_succeeded
                && row.actual_prefix_jvp_vectors.unwrap_or(u64::MAX) <= 80
                && row.normalized_stage_growth_a34.is_some()
                && row.audit_full_e_completed)
    }));
    assert_eq!(
        audited.audit_full_e_continuations,
        audited
            .rows
            .iter()
            .filter(|row| row.audit_full_e_completed)
            .count()
    );
    let geometry_rows: Vec<_> = audited
        .rows
        .iter()
        .filter(|row| row.prefix_succeeded && row.remainder_chi34.is_some())
        .collect();
    assert!(!geometry_rows.is_empty());
    for row in geometry_rows {
        assert!(row.remainder_chi23.unwrap().abs() <= 1.0);
        assert!(row.remainder_chi34.unwrap().abs() <= 1.0);
        assert!(row.remainder_chi24.unwrap().abs() <= 1.0);
        assert!((0.0..=1.0).contains(&row.remainder_q34_perp.unwrap()));
        assert!(row.remainder_delta_chi.unwrap().abs() <= 2.0);
        assert!(row.quadratic_drift_zeta23.is_some());
        assert!(row.quadratic_drift_zeta34.is_some());
        assert!(row.quadratic_drift_relative.is_some());
        assert!((-1.0..=1.0).contains(&row.quadratic_drift_relative.unwrap()));
    }
}
