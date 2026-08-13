use rodas5p_integrators::{G3FusedAdaptiveProfile, run_g3_fused_adaptive_gate};

#[test]
fn smoke_g3_gate_is_matrix_free_newton_free_and_complete() {
    let report = run_g3_fused_adaptive_gate(G3FusedAdaptiveProfile::Smoke).unwrap();
    assert_eq!(report.status, "pass");
    assert_eq!(report.summary.phi_completed, report.summary.phi_rows);
    assert_eq!(
        report.summary.adaptive_successes,
        report.summary.adaptive_rows
    );
    assert_eq!(report.summary.explicit_jacobian_builds_in_primary, 0);
    assert_eq!(report.summary.direct_factorizations_in_primary, 0);
    assert_eq!(report.summary.newton_iterations_in_primary, 0);
    assert_eq!(report.summary.legacy_to_fused_phi_action_ratio, 3.0);
    assert!(report.summary.median_fused_phi_wall_speedup.unwrap() > 0.0);

    for problem in ["quadratic", "complex-dahlquist", "oscillatory-pr"] {
        let t1 = report
            .adaptive_rows
            .iter()
            .find(|row| {
                row.problem_id == problem && row.candidate_id == "pexprb54s4-fused-full-mgs-t1"
            })
            .unwrap();
        let t4 = report
            .adaptive_rows
            .iter()
            .find(|row| {
                row.problem_id == problem && row.candidate_id == "pexprb54s4-fused-full-mgs-t4"
            })
            .unwrap();
        assert_eq!(t1.success, t4.success);
        assert_eq!(t1.failure, t4.failure);
        assert_eq!(t1.endpoint_l2_error, t4.endpoint_l2_error);
        assert_eq!(t1.accepted_steps, t4.accepted_steps);
        assert_eq!(t1.rejected_steps, t4.rejected_steps);
        assert_eq!(t1.maximum_time_error, t4.maximum_time_error);
        assert_eq!(t1.maximum_phi_error, t4.maximum_phi_error);
        assert_eq!(t1.maximum_total_error, t4.maximum_total_error);
        assert_eq!(t1.work, t4.work);
    }
}
