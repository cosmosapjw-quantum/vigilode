use std::{fs, process::Command};

fn temp_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("rodas5p-rs-{}-{name}", std::process::id()));
    path
}

#[test]
fn validate_trace_and_benchmark_commands_emit_machine_readable_artifacts() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let validation = temp_path("validation.json");
    let trace = temp_path("trace.json");
    let benchmark = temp_path("benchmark.json");

    let status = Command::new(binary)
        .args(["validate", "--output"])
        .arg(&validation)
        .status()
        .unwrap();
    assert!(status.success());
    let validation_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&validation).unwrap()).unwrap();
    assert_eq!(validation_json["status"], "pass");

    let status = Command::new(binary)
        .args([
            "trace",
            "--kind",
            "fixed",
            "--dimension",
            "8",
            "--steps",
            "1",
            "--stages",
            "8",
            "--stiffness",
            "100",
            "--nonnormality",
            "0.05",
            "--output",
        ])
        .arg(&trace)
        .status()
        .unwrap();
    assert!(status.success());
    let trace_json: serde_json::Value = serde_json::from_slice(&fs::read(&trace).unwrap()).unwrap();
    assert_eq!(trace_json["schema"], "rodas5p-rust-trace-v1");
    assert_eq!(trace_json["cases"].as_array().unwrap().len(), 8);

    let status = Command::new(binary)
        .args(["benchmark", "--trace"])
        .arg(&trace)
        .args(["--output"])
        .arg(&benchmark)
        .args([
            "--repetitions",
            "1",
            "--warmups",
            "0",
            "--restart",
            "6",
            "--recycle-dim",
            "2",
            "--operator-budget",
            "300",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let benchmark_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&benchmark).unwrap()).unwrap();
    assert_eq!(benchmark_json["schema"], "rodas5p-rust-fair-ab-v1");
    assert!(benchmark_json["summary"].as_array().unwrap().len() >= 3);
    assert_eq!(benchmark_json["failures"], 0);

    let _ = fs::remove_file(validation);
    let _ = fs::remove_file(trace);
    let _ = fs::remove_file(benchmark);
}

#[test]
fn homotopy_design_check_is_deterministic_and_exposes_required_screens() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let first = temp_path("homotopy-design-check-first.json");
    let second = temp_path("homotopy-design-check-second.json");

    for output in [&first, &second] {
        let status = Command::new(binary)
            .args(["homotopy-design-check", "--output"])
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }

    let first_bytes = fs::read(&first).unwrap();
    let second_bytes = fs::read(&second).unwrap();
    assert_eq!(first_bytes, second_bytes);

    let report: serde_json::Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(report["schema"], "rodas5p-homotopy-design-check-v1");
    assert_eq!(report["status"], "pass");
    assert_eq!(report["stages"], 8);
    assert!(report["affine_endpoint_max_abs_error"].as_f64().unwrap() < 1e-10);
    assert!(report["flrh_lambda_spread"].as_f64().unwrap() < 1e-10);

    let q_rows = report["truncation_screen"].as_array().unwrap();
    assert_eq!(q_rows.len(), 4);
    assert_eq!(q_rows.last().unwrap()["q"], 7);
    assert!(q_rows.last().unwrap()["relative_error"].as_f64().unwrap() < 1e-10);
    assert!(
        q_rows.first().unwrap()["relative_error"].as_f64().unwrap()
            > q_rows.last().unwrap()["relative_error"].as_f64().unwrap()
    );

    let powers = report["official_l_power_norms"].as_array().unwrap();
    assert_eq!(powers.len(), 8);
    assert_eq!(powers.last().unwrap()["power"], 8);
    assert!(powers.last().unwrap()["frobenius_norm"].as_f64().unwrap() < 1e-10);

    let nonnormal = report["nonnormal_condition_screen"].as_array().unwrap();
    assert_eq!(nonnormal.len(), 5);
    assert_eq!(nonnormal.first().unwrap()["determinant"], 1.0);
    assert_eq!(nonnormal.last().unwrap()["determinant"], 1.0);
    assert!(
        nonnormal.last().unwrap()["condition_one"].as_f64().unwrap()
            > nonnormal.first().unwrap()["condition_one"]
                .as_f64()
                .unwrap()
    );

    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[test]
fn homotopy_experiment_screen_emits_deterministic_safety_and_control_rows() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let first = temp_path("homotopy-experiment-screen-first.json");
    let second = temp_path("homotopy-experiment-screen-second.json");

    for output in [&first, &second] {
        let status = Command::new(binary)
            .args([
                "homotopy-experiment-screen",
                "--profile",
                "smoke",
                "--output",
            ])
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }

    let first_bytes = fs::read(&first).unwrap();
    let second_bytes = fs::read(&second).unwrap();
    assert_eq!(first_bytes, second_bytes);

    let report: serde_json::Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(report["schema"], "rodas5p-homotopy-experiment-screen-v1");
    assert_eq!(report["profile"], "smoke");
    assert_eq!(report["output_wrms_budget"], 0.1);
    assert!(report.get("defect_budget_fraction").is_none());
    assert!(report["cases"].as_array().unwrap().len() >= 3);
    assert!(report["controls"].as_array().unwrap().len() >= 6);
    assert!(report["candidates"].as_array().unwrap().len() >= 8);
    assert!(report["order_screens"].as_array().unwrap().len() >= 4);
    let order_rows = report["order_screens"].as_array().unwrap();
    let sequential_order = order_rows
        .iter()
        .find(|row| row["method"] == "sequential-direct" && row["h"] == 0.02)
        .and_then(|row| row["observed_order"].as_f64())
        .unwrap();
    assert!(sequential_order > 4.5);
    let q7_order = order_rows
        .iter()
        .find(|row| row["method"] == "homotopy-theta1-q7-r2-ab2-c1" && row["h"] == 0.02)
        .unwrap();
    assert!(q7_order["all_fast"].as_bool().unwrap());
    assert!(q7_order["observed_order"].as_f64().unwrap() > 4.5);
    assert_eq!(report["summary"]["false_accepts"], 0);
    assert!(
        report["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["q"] == 7)
    );

    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[test]
fn homotopy_order_policy_screen_is_deterministic_and_thread_count_independent() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let first = temp_path("homotopy-order-policy-t1-first.json");
    let second = temp_path("homotopy-order-policy-t1-second.json");
    let parallel = temp_path("homotopy-order-policy-t4.json");

    for output in [&first, &second] {
        let status = Command::new(binary)
            .args([
                "homotopy-order-policy-screen",
                "--profile",
                "smoke",
                "--threads",
                "1",
                "--output",
            ])
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }

    let status = Command::new(binary)
        .args([
            "homotopy-order-policy-screen",
            "--profile",
            "smoke",
            "--threads",
            "4",
            "--output",
        ])
        .arg(&parallel)
        .status()
        .unwrap();
    assert!(status.success());

    let first_bytes = fs::read(&first).unwrap();
    let second_bytes = fs::read(&second).unwrap();
    assert_eq!(first_bytes, second_bytes);

    let serial: serde_json::Value = serde_json::from_slice(&first_bytes).unwrap();
    let parallel_report: serde_json::Value =
        serde_json::from_slice(&fs::read(&parallel).unwrap()).unwrap();

    assert_eq!(serial["schema"], "rodas5p-homotopy-order-policy-screen-v1");
    assert_eq!(serial["profile"], "smoke");
    assert_eq!(serial["execution"]["threads"], 1);
    assert_eq!(parallel_report["execution"]["threads"], 4);
    assert!(!serial["replay_rows"].as_array().unwrap().is_empty());
    assert_eq!(serial["family_winners"].as_array().unwrap().len(), 4);
    assert!(!serial["trajectory_rows"].as_array().unwrap().is_empty());
    assert!(!serial["trajectory_gates"].as_array().unwrap().is_empty());

    for field in [
        "source_summary",
        "policies",
        "replay_rows",
        "policy_summaries",
        "family_winners",
        "trajectory_rows",
        "trajectory_gates",
    ] {
        assert_eq!(serial[field], parallel_report[field], "field {field}");
    }

    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
    let _ = fs::remove_file(parallel);
}

#[test]
fn unified_candidate_screen_combines_linear_and_nonlinear_tiers() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = temp_path("unified-candidate-screen.json");
    let status = Command::new(binary)
        .args([
            "unified-candidate-screen",
            "--profile",
            "smoke",
            "--threads",
            "2",
            "--output",
        ])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    let report: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(report["schema"], "rodas5p-unified-candidate-screen-v4");
    assert_eq!(report["profile"], "smoke");
    assert!(!report["linear_suites"].as_array().unwrap().is_empty());
    assert!(!report["nonlinear"]["rows"].as_array().unwrap().is_empty());
    assert!(
        !report["nonlinear_assessments"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !report["scientific_gates"]["candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!report["catalog"]["entries"].as_array().unwrap().is_empty());
    assert_eq!(
        report["native_integrator_gates"]["rows"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let bdf2 = report["joint_assessments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["candidate_id"] == "bdf2-fixed")
        .unwrap();
    assert_eq!(bdf2["scientific_eligible"], true);
    assert_eq!(bdf2["verdict"], "hold");
    assert!(
        bdf2["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().contains("performance assessment"))
    );
    let _ = fs::remove_file(output);
}

#[test]
fn native_integrator_gates_command_emits_deterministic_bdf_and_radau_rows() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let first = temp_path("native-integrator-gates-first.json");
    let second = temp_path("native-integrator-gates-second.json");

    for output in [&first, &second] {
        let status = Command::new(binary)
            .args(["native-integrator-gates", "--output"])
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }

    let first_bytes = fs::read(&first).unwrap();
    let second_bytes = fs::read(&second).unwrap();
    assert_eq!(first_bytes, second_bytes);
    let report: serde_json::Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(report["schema"], "rodas5p-native-integrator-gates-v1");
    assert_eq!(report["rows"].as_array().unwrap().len(), 4);
    for row in report["rows"].as_array().unwrap() {
        assert_eq!(row["order_pass"], true);
        assert_eq!(row["stiff_pass"], true);
        assert_eq!(row["mass_pass"], true);
        assert_eq!(row["failures"], 0);
    }

    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[test]
fn global_error_pareto_command_emits_deterministic_method_independent_report() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let serial = temp_path("global-error-pareto-t1.json");
    let parallel = temp_path("global-error-pareto-t4.json");

    for (threads, output) in [("1", &serial), ("4", &parallel)] {
        let status = Command::new(binary)
            .args([
                "global-error-pareto",
                "--profile",
                "smoke",
                "--threads",
                threads,
                "--output",
            ])
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }

    let one: serde_json::Value = serde_json::from_slice(&fs::read(&serial).unwrap()).unwrap();
    let four: serde_json::Value = serde_json::from_slice(&fs::read(&parallel).unwrap()).unwrap();
    assert_eq!(one["schema"], "rodas5p-global-error-pareto-v2");
    assert_eq!(one["output_policy"]["save_internal_steps"], false);
    assert_eq!(one["output_policy"]["dense_output_used"], false);
    assert_eq!(one["output_policy"]["landing"], "step-clipping");
    assert_eq!(one["profile"], "smoke");
    assert_eq!(one["execution"]["threads"], 1);
    assert_eq!(four["execution"]["threads"], 4);
    assert_eq!(one["scientific_checksum"], four["scientific_checksum"]);
    assert!(!one["runs"].as_array().unwrap().is_empty());
    assert!(!one["fronts"].as_array().unwrap().is_empty());

    let _ = fs::remove_file(serial);
    let _ = fs::remove_file(parallel);
}
