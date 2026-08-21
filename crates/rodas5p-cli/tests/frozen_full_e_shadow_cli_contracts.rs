use std::{fs, process::Command};

fn temp_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("rodas5p-v36-{}-{name}", std::process::id()));
    path
}

#[test]
fn frozen_full_e_shadow_command_emits_the_dedicated_v36_schema() {
    let output = temp_path("shadow.json");
    let status = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "generic-frozen-full-e-shadow",
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
    assert_eq!(report["schema"], "g4-s5b0-frozen-full-e-shadow-v1");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["profile"], "stage-growth-calibration-96");
    assert_eq!(report["switching_active"], false);
    assert_eq!(report["persistence_k"], 3);
    assert_eq!(report["absolute_prefix_jvp_cap"], 80);
    assert_eq!(report["frozen_cumulative_prefix_budget_fraction"], 0.25);
    assert!(report["realized_prefix_over_committed_rjf_jvp"].is_number());
    assert!(report["realized_continuation_over_committed_rjf_jvp"].is_number());
    assert!(report["realized_total_speculative_over_committed_rjf_jvp"].is_number());
    assert_eq!(report["frozen_zeta34_tau"], 13.39706618860016);
    assert_eq!(report["recommendations"], 2);
    assert_eq!(report["retained_level2_resumptions"], 2);
    assert_eq!(report["shadow_full_e_completions"], 2);
    assert_eq!(report["shadow_full_e_failures"], 0);
    assert_eq!(report["unsafe_recommendations"], 0);
    assert_eq!(report["prefix_speculative_work"]["jvp_vectors"], 42);
    assert_eq!(report["continuation_work"]["jvp_vectors"], 24);
    assert_eq!(report["total_speculative_work"]["jvp_vectors"], 66);
    assert_eq!(report["rjf_parity"]["passed"], true);
    assert_eq!(report["hard_gates"]["passed"], true);
    assert_eq!(report["rows"].as_array().unwrap().len(), 2);
    assert!(report["rows"].as_array().unwrap().iter().all(|row| {
        row.get("audit_full_e_completed").is_none()
            && row.get("runtime_full_e_continued").is_none()
            && row["recommended"] == true
            && row["retained_level2_resumed"] == true
            && row["work_roundtrip_exact"] == true
    }));

    let _ = fs::remove_file(output);
}

#[test]
fn paired_economics_rejects_a_non_measurement_binary_before_writing_output() {
    if std::path::Path::new(env!("CARGO_BIN_EXE_rodas5p"))
        .components()
        .any(|component| component.as_os_str() == "measurement")
    {
        return;
    }
    let output = temp_path("debug-economics-must-not-exist.json");
    let result = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "generic-frozen-full-e-shadow-economics",
            "--profile",
            "calibration96",
            "--output",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!output.exists());
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(stderr.contains("cargo run --profile measurement"));
}
