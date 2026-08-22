use std::{fs, process::Command};

fn temp_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("rodas5p-v37-{}-{name}", std::process::id()));
    path
}

#[test]
fn v37_command_emits_dedicated_schema_without_mutating_v36_fields() {
    let output = temp_path("continuation.json");
    let status = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "generic-v37-continuation-transaction",
            "--profile",
            "calibration96",
            "--family",
            "robertson",
            "--output",
        ])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());

    let report: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(report["schema"], "g4-s5b0-v37-continuation-transaction-v1");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["switching_active"], false);
    assert_eq!(report["absolute_continuation_jvp_cap"], 80);
    assert_eq!(report["recommendations"], 2);
    assert_eq!(report["shadow_full_e_completions"], 2);
    assert_eq!(report["continuation_budget_exhaustions"], 0);
    assert_eq!(report["shadow_full_e_failures"], 0);
    assert_eq!(report["hard_gates"]["passed"], true);

    let rows = report["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| {
        row["continuation_outcome"] == "complete"
            && row["continuation_budget_exhausted"] == false
            && row["shadow_full_e_completed"] == true
            && row["shadow_full_e_total_error"].is_number()
            && row["shadow_full_e_locally_admissible"] == true
            && row["shadow_full_e_failure"].is_null()
    }));

    let _ = fs::remove_file(output);
}
