use rodas5p_integrators::{G3FusedAdaptiveProfile, run_g3_fused_adaptive_gate};

#[test]
fn smoke_g3_gate_is_matrix_free_newton_free_and_complete() {
    let report=run_g3_fused_adaptive_gate(G3FusedAdaptiveProfile::Smoke).unwrap();
    assert_eq!(report.status,"pass");
    assert_eq!(report.summary.phi_completed,report.summary.phi_rows);
    assert_eq!(report.summary.adaptive_successes,report.summary.adaptive_rows);
    assert_eq!(report.summary.explicit_jacobian_builds_in_primary,0);
    assert_eq!(report.summary.direct_factorizations_in_primary,0);
    assert_eq!(report.summary.newton_iterations_in_primary,0);
    assert_eq!(report.summary.legacy_to_fused_phi_action_ratio,3.0);
    assert!(report.summary.median_fused_phi_wall_speedup.unwrap()>0.0);
}
