//! CLI contracts for research node ME01 (research/me01_full_cost_judgement_20261011).

#[path = "../src/stiff_benchmark.rs"]
#[allow(dead_code, unused_imports)]
mod stiff_benchmark;

use std::process::Command;

#[test]
fn me01_arms_are_known_and_opt_in() {
    for arm in stiff_benchmark::ME01_ARMS {
        assert!(
            !stiff_benchmark::ARMS.contains(&arm),
            "{arm} should not be in default benchmark ARMS"
        );
    }
}

#[test]
fn me01_profile_run_repetitions_zero() {
    let out = stiff_benchmark::profile_run("robertson", "rodas5p", 1.0e-6, 0).unwrap();
    assert_eq!(out["repetitions"], 0);
    assert_eq!(out["attempts"], 0);
    assert_eq!(out["accepted_steps"], 0);
    assert_eq!(out["rejected_steps"], 0);
    assert!(out["final_state"].is_null());
    assert_eq!(out["deterministic"], true);
}

#[test]
fn me01_identity_candidates_parity() {
    // E1: rodas5p-mf-legacy-v2 vs rodas5p-mf-legacy
    let legacy =
        stiff_benchmark::profile_run("brusselator-1d-50", "rodas5p-mf-legacy", 1.0e-3, 1).unwrap();
    let v2 = stiff_benchmark::profile_run("brusselator-1d-50", "rodas5p-mf-legacy-v2", 1.0e-3, 1)
        .unwrap();
    assert_eq!(legacy["final_state"], v2["final_state"]);
    assert_eq!(legacy["counters"], v2["counters"]);
    assert_eq!(legacy["attempts"], v2["attempts"]);

    // E7: rodas5p-mf-ls-ws vs rodas5p-mf-into
    let into =
        stiff_benchmark::profile_run("brusselator-1d-50", "rodas5p-mf-into", 1.0e-3, 1).unwrap();
    let ls_ws =
        stiff_benchmark::profile_run("brusselator-1d-50", "rodas5p-mf-ls-ws", 1.0e-3, 1).unwrap();
    assert_eq!(into["final_state"], ls_ws["final_state"]);
    assert_eq!(into["counters"], ls_ws["counters"]);
    assert_eq!(into["attempts"], ls_ws["attempts"]);

    // E8: rodas5p-mf-into vs rodas5p-mf
    let mf = stiff_benchmark::profile_run("brusselator-1d-50", "rodas5p-mf", 1.0e-3, 1).unwrap();
    assert_eq!(into["final_state"], mf["final_state"]);
    assert_eq!(into["counters"], mf["counters"]);
    assert_eq!(into["attempts"], mf["attempts"]);
}

#[test]
fn me01_ct01_grid_arms_run() {
    let pred2 =
        stiff_benchmark::profile_run("brusselator-1d-50", "rodas5p-fast-pred2", 1.0e-3, 1).unwrap();
    assert_eq!(pred2["success"], true);
    assert!(pred2["grid_states"].is_array());
    assert_eq!(pred2["grid_states"].as_array().unwrap().len(), 41);

    let int_grid =
        stiff_benchmark::profile_run("brusselator-1d-50", "rodas5p-fast-int-grid", 1.0e-3, 1)
            .unwrap();
    assert_eq!(int_grid["success"], true);
    assert!(int_grid["grid_states"].is_array());
    assert_eq!(int_grid["grid_states"].as_array().unwrap().len(), 41);
}

#[test]
fn me01_cli_stiff_profile_run_repetitions_zero_binary() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = Command::new(binary)
        .args([
            "stiff-profile-run",
            "--problem",
            "robertson",
            "--arm",
            "rodas5p",
            "--rtol",
            "1e-6",
            "--repetitions",
            "0",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(doc["repetitions"], 0);
    assert_eq!(doc["attempts"], 0);
}
