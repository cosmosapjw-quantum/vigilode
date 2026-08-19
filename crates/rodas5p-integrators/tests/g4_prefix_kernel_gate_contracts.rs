use rodas5p_integrators::{G4PrefixKernelProfile, run_g4_prefix_kernel_gate};

#[test]
fn smoke_prefix_kernel_gate_is_read_only_and_reuses_bases_exactly() {
    let report = run_g4_prefix_kernel_gate(G4PrefixKernelProfile::Smoke).expect("G4-S4 smoke gate");
    assert_eq!(report.status, "read-only-prefix-kernel-gate");
    assert!(!report.summary.active_switching_authorized);
    assert!(report.summary.completed > 0);
    assert!(report.summary.reuse_parity_gate_pass);
    assert_eq!(report.summary.selected_prefix_dimension, 2);
    assert!(report.summary.selected_prefix_cost_gate_pass);
    assert!(
        report
            .rows
            .iter()
            .filter(|row| row.completed)
            .all(|row| row.exponential_jvp_vectors.unwrap_or(0) > 0)
    );
}
