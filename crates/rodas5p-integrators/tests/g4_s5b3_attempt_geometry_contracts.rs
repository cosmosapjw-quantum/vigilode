use rodas5p_integrators::{G4S5B3Profile, run_g4_s5b3_attempt_geometry};

#[test]
fn smoke_attempt_geometry_is_read_only_complete_and_threshold_free() {
    let report = run_g4_s5b3_attempt_geometry(G4S5B3Profile::Smoke).unwrap();
    assert_eq!(report.schema, "g4-s5b3-attempt-geometry-raw-v1");
    assert_eq!(report.status, "pass");
    assert!(!report.active_switching);
    assert!(!report.early_abort);
    assert!(!report.threshold_selected);
    assert!(report.selected_threshold.is_none());
    assert_eq!(report.hard_gates.expected_trajectories, 3);
    assert_eq!(report.hard_gates.observed_trajectories, 3);
    assert!(report.hard_gates.passed);
    assert!(!report.attempts.is_empty());
    assert!(
        report
            .trajectories
            .iter()
            .all(|row| row.all_required_gates_pass)
    );
    assert!(
        report
            .trajectories
            .iter()
            .all(|row| row.unscorable_attempts == 0)
    );
    let overhead = report.overhead.expect("smoke overhead report");
    assert_eq!(overhead.measured_pairs, 1);
    assert_eq!(overhead.frozen_repetitions, 1);
    assert!(overhead.all_suite_identities_passed);
    assert_eq!(overhead.measured_rows.len(), 1);
}
