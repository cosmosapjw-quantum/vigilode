use std::{fs, process::Command};

fn temp_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("rodas5p-a1-receipt-{}-{name}", std::process::id()));
    path
}

fn provenance_args() -> [&'static str; 20] {
    [
        "--repository",
        "cosmosapjw-quantum/vigilode",
        "--pull-request",
        "18",
        "--scientific-execution-head-sha",
        "1111111111111111111111111111111111111111",
        "--scientific-execution-head-tree",
        "2222222222222222222222222222222222222222",
        "--base-sha",
        "3333333333333333333333333333333333333333",
        "--base-tree",
        "4444444444444444444444444444444444444444",
        "--tested-execution-merge-sha",
        "5555555555555555555555555555555555555555",
        "--tested-execution-merge-tree",
        "6666666666666666666666666666666666666666",
        "--execution-workflow-run-id",
        "123",
        "--execution-workflow-run-attempt",
        "1",
    ]
}

#[test]
fn receipt_cli_emits_one_deterministic_candidate_cell_without_late_bound_fields() {
    let output = temp_path("candidate.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_rodas5p"));
    command
        .arg("a1-two-arm-receipt-cell")
        .args(provenance_args())
        .args([
            "--family",
            "robertson-ramped",
            "--arm",
            "outer-scaled-numeric-parity",
            "--rust-version",
            "rustc 1.94.1",
            "--cargo-version",
            "cargo 1.94.1",
            "--output",
        ])
        .arg(&output);
    assert!(command.status().unwrap().success());

    let bytes = fs::read(&output).unwrap();
    let cell: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cell["schema"], "vigilode-a1-two-arm-atomic-cell-v1");
    assert_eq!(cell["profile"], "enforced-budget-holdout-320");
    assert_eq!(cell["family"], "robertson-ramped");
    assert_eq!(cell["arm"], "outer-scaled-numeric-parity");
    assert_eq!(cell["switching_active"], false);
    assert!(cell.get("receipt_commit_sha").is_none());
    assert!(cell.get("receipt_commit_tree").is_none());
    assert!(cell.get("external_verification_run_id").is_none());
    assert!(!String::from_utf8(bytes).unwrap().contains("wall_seconds"));
    let _ = fs::remove_file(output);
}

#[test]
fn receipt_cli_rejects_unknown_arm_family_and_profile_surface() {
    for args in [
        vec!["--family", "robertson-ramped", "--arm", "unknown-arm"],
        vec!["--family", "unknown-family", "--arm", "legacy-fixed"],
        vec![
            "--profile",
            "smoke",
            "--family",
            "robertson-ramped",
            "--arm",
            "legacy-fixed",
        ],
    ] {
        let output = temp_path("must-not-exist.json");
        let mut command = Command::new(env!("CARGO_BIN_EXE_rodas5p"));
        command
            .arg("a1-two-arm-receipt-cell")
            .args(provenance_args())
            .args(args)
            .args([
                "--rust-version",
                "rustc 1.94.1",
                "--cargo-version",
                "cargo 1.94.1",
                "--output",
            ])
            .arg(&output);
        assert!(!command.status().unwrap().success());
        assert!(!output.exists());
    }
}
