//! A paired-timing campaign measured in separate processes and consumed only
//! through its raw receipt (re-audit R3 of 2026-10-01, STAT-DEV-02).

use std::collections::BTreeMap;
use std::process::Command;

use rodas5p_fair_ab::{PAIRED_TIMING_MONOTONIC_CLOCK, PairedTimingEvidence};

#[test]
fn six_sessions_in_six_processes_form_a_verifiable_receipt() {
    let directory =
        std::env::temp_dir().join(format!("vigilode-paired-campaign-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let output = directory.join("evidence.json");
    let status = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .arg("paired-timing-campaign")
        .args(["--candidate", "sequential-gcrodr-persistent"])
        .args(["--profile", "smoke"])
        .args(["--sessions", "6"])
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    let map: BTreeMap<String, PairedTimingEvidence> =
        serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
    let evidence = &map["sequential-gcrodr-persistent"];
    let receipt = &evidence.receipt;
    assert_eq!(receipt.sessions.len(), 6);
    let processes = receipt
        .sessions
        .iter()
        .map(|session| session.process_id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(processes.len(), 6, "each session is its own process");
    assert!(!processes.contains(&std::process::id()));
    assert!(
        receipt
            .sessions
            .iter()
            .all(|session| session.clock == PAIRED_TIMING_MONOTONIC_CLOCK)
    );
    assert_eq!(receipt.reference.arm_id, "sequential-gmres-off");
    // The decision itself depends on this host; it must be recomputable.
    evidence.verified_decision().unwrap();
    assert_eq!(evidence.assessment.corpus.independent_blocks, 6);
    // A replayed receipt with a corrupted raw sample is rejected.
    let mut corrupted = evidence.clone();
    corrupted.receipt.session_records[1].cases[0].reference_seconds[3] *= 2.0;
    assert!(corrupted.verified_decision().is_err());
    let _ = std::fs::remove_dir_all(&directory);
}
