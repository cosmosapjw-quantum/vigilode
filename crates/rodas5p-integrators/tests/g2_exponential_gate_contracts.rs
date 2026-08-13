use rodas5p_integrators::{G2ExponentialGateProfile, run_g2_exponential_gate};

#[test]
fn g2_foundation_smoke_closes_coefficient_phi_order_and_structure_gates() {
    let report = run_g2_exponential_gate(G2ExponentialGateProfile::Smoke)
        .expect("G2 exponential foundation report");
    assert!(report.summary.coefficient_gate_pass);
    assert!(report.summary.phi_oracle_gate_pass);
    assert!(report.summary.order_gate_pass);
    assert!(report.summary.structural_jf_newton_free_gate_pass);
    assert!(report.summary.g2_foundation_pass);
    assert!(!report.summary.performance_promotion_authorized);
    assert_eq!(report.summary.explicit_jacobian_builds, 0);
    assert_eq!(report.summary.direct_factorizations, 0);
    assert_eq!(report.summary.nonlinear_iterations, 0);
    assert_eq!(report.coefficient_authority.logical_critical_depth, 3);
}

#[test]
fn g2_foundation_report_is_deterministic() {
    let first = run_g2_exponential_gate(G2ExponentialGateProfile::Smoke).expect("first");
    let second = run_g2_exponential_gate(G2ExponentialGateProfile::Smoke).expect("second");
    assert_eq!(first, second);
}
