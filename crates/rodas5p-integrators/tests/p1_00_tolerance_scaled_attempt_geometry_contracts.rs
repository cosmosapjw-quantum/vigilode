use rodas5p_integrators::{G4S5B3Profile, run_p1_00_tolerance_scaled_early_defect};

#[test]
fn smoke_tolerance_scaled_geometry_is_read_only_complete_and_threshold_free() {
    let report = run_p1_00_tolerance_scaled_early_defect(G4S5B3Profile::Smoke).unwrap();
    assert_eq!(report.schema, "p1-00-tolerance-scaled-early-defect-raw-v1");
    assert_eq!(report.status, "pass");
    assert!(!report.active_switching);
    assert!(!report.early_abort);
    assert!(!report.threshold_selected);
    assert!(report.selected_threshold.is_none());
    assert!(report.hard_gates.passed);
    assert!(!report.attempts.is_empty());
    assert!(report.attempts.iter().all(|row| {
        row.eta_c2.is_some_and(f64::is_finite)
            && row.rho_c2_wrms.is_some_and(f64::is_finite)
            && row.tolerance_scale_atol == Some(row.atol)
            && row.tolerance_scale_rtol == Some(row.rtol)
            && row.failure.is_none()
            && row.diagnostic_work.is_some_and(|work| {
                work.component_scale_evaluations > 0
                    && work.wrms_norm_evaluations == 1
                    && work.added_rhs_calls == 0
                    && work.added_jvp_calls == 0
                    && work.added_phi_actions == 0
                    && work.added_jacobian_builds == 0
                    && work.added_newton_iterations == 0
            })
    }));
    assert!(
        report
            .overhead
            .as_ref()
            .is_some_and(|row| row.all_suite_identities_passed)
    );
}
