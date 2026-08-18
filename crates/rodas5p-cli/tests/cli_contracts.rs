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

#[test]
fn adaptive_global_error_command_emits_current_family_report() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = temp_path("adaptive-global-error.json");
    let status = Command::new(binary)
        .args([
            "adaptive-global-error",
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
    assert_eq!(report["schema"], "rodas5p-adaptive-global-error-v1");
    assert_eq!(report["execution"]["threads"], 2);
    assert_eq!(report["candidates"].as_array().unwrap().len(), 10);
    assert_eq!(report["problems"].as_array().unwrap().len(), 2);
    assert_eq!(report["tolerance_ladder"].as_array().unwrap().len(), 2);
    assert_eq!(report["runs"].as_array().unwrap().len(), 40);
    assert_eq!(report["scientific_checksum"].as_str().unwrap().len(), 64);
    let _ = fs::remove_file(output);
}

#[test]
fn stage_batch_feasibility_cli_emits_deterministic_within_step_report() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let first = temp_path("stage-batch-feasibility-first.json");
    let second = temp_path("stage-batch-feasibility-second.json");

    for output in [&first, &second] {
        let status = Command::new(binary)
            .args(["stage-batch-feasibility", "--profile", "smoke", "--output"])
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }

    let first_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&first).unwrap()).unwrap();
    let second_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&second).unwrap()).unwrap();
    assert_eq!(
        first_value["scientific_checksum"],
        second_value["scientific_checksum"]
    );
    assert_eq!(first_value["schema"], "rodas5p-stage-batch-feasibility-v1");
    assert!(first_value["stage_parallelism_observed"].as_bool().unwrap());
    assert!(first_value["observed_max_parallel_tasks"].as_u64().unwrap() >= 2);
    assert!(
        first_value["rhs_and_jvp_paths_matrix_free"]
            .as_bool()
            .unwrap()
    );
    assert!(
        first_value["common_w_dense_reference_setup_used"]
            .as_bool()
            .unwrap()
    );
    assert!(
        !first_value["strict_jacobian_free_common_w_demonstrated"]
            .as_bool()
            .unwrap()
    );
    assert!(!first_value["explicit_simd_demonstrated"].as_bool().unwrap());
    assert_eq!(first_value["cases"].as_array().unwrap().len(), 2);
    assert!(first_value["rows"].as_array().unwrap().iter().any(|row| {
        row["kernel"] == "combined-round" && row["backend"] == "rayon-stage-4+multi-rhs"
    }));

    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[test]
fn matrix_free_common_w_cli_emits_strict_jf_report() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = temp_path("matrix-free-common-w.json");
    let status = Command::new(binary)
        .args([
            "matrix-free-common-w-gate",
            "--profile",
            "smoke",
            "--output",
        ])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());

    let report: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(report["schema"], "rodas5p-matrix-free-common-w-gate-v1");
    assert!(report["strict_jacobian_free"].as_bool().unwrap());
    assert_eq!(report["explicit_jacobian_builds"], 0);
    assert_eq!(report["factorization_builds"], 0);
    assert_eq!(report["scientific_checksum"].as_str().unwrap().len(), 64);
    assert!(report["rows"].as_array().unwrap().iter().any(|row| {
        row["solver"] == "block-gmres" && row["block_operator_calls"].as_u64().unwrap() > 0
    }));

    let _ = fs::remove_file(output);
}

#[test]
fn homotopy_rhs_telemetry_cli_is_deterministic_and_read_only() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let first = temp_path("homotopy-rhs-telemetry-first.json");
    let second = temp_path("homotopy-rhs-telemetry-second.json");

    for output in [&first, &second] {
        let status = Command::new(binary)
            .args(["homotopy-rhs-telemetry", "--profile", "smoke", "--output"])
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }

    let first_bytes = fs::read(&first).unwrap();
    assert_eq!(first_bytes, fs::read(&second).unwrap());
    let report: serde_json::Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(report["schema"], "rodas5p-homotopy-rhs-telemetry-v1");
    assert_eq!(report["profile"], "smoke");
    assert_eq!(report["solver_behavior_changed"], false);
    assert_eq!(report["explicit_jacobian_builds_in_dispatch"], 0);
    assert!(!report["rows"].as_array().unwrap().is_empty());
    assert!(
        report["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["rhs_count"] == 8)
    );

    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[test]
fn homotopy_path_controller_cli_is_deterministic_and_emits_schedule_telemetry() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let first = temp_path("homotopy-path-controller-first.json");
    let second = temp_path("homotopy-path-controller-second.json");

    for output in [&first, &second] {
        let status = Command::new(binary)
            .args(["homotopy-path-controller", "--profile", "smoke", "--output"])
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }

    let first_bytes = fs::read(&first).unwrap();
    let second_bytes = fs::read(&second).unwrap();
    assert_eq!(first_bytes, second_bytes);
    let report: serde_json::Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(report["schema"], "rodas5p-path-controller-screen-v1");
    assert_eq!(report["profile"], "smoke");
    assert!(report["summary"]["rows"].as_u64().unwrap() > 0);
    assert!(report["rows"].as_array().unwrap().iter().any(|row| {
        row["schedule_id"] == "escalate-q012" && row["points"].as_array().unwrap().len() >= 2
    }));

    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[test]
fn generic_q1_q2_adaptive_cli_emits_bounded_decision_report() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = temp_path("generic-q1q2-adaptive.json");
    let status = Command::new(binary)
        .args([
            "generic-q1q2-adaptive",
            "--profile",
            "smoke",
            "--threads",
            "1",
            "--output",
        ])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    let report: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(report["schema"], "generic-q1-q2-adaptive-global-error-v1");
    assert_eq!(report["candidates"].as_array().unwrap().len(), 5);
    assert_eq!(report["runs"].as_array().unwrap().len(), 20);
    let _ = fs::remove_file(output);
}

#[test]
fn generic_q1_q2_gate_is_deterministic_and_strictly_matrix_free() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let first = temp_path("generic-q1-q2-first.json");
    let second = temp_path("generic-q1-q2-second.json");

    for output in [&first, &second] {
        let status = Command::new(binary)
            .args(["generic-q1-q2-gate", "--profile", "smoke", "--output"])
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }

    let first_bytes = fs::read(&first).unwrap();
    assert_eq!(first_bytes, fs::read(&second).unwrap());
    let report: serde_json::Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(report["schema"], "generic-q1-q2-transactional-gate-v1");
    assert_eq!(report["summary"]["explicit_jacobian_builds"], 0);
    assert_eq!(report["summary"]["direct_factorizations"], 0);
    assert_eq!(report["summary"]["fast_path_newton_iterations"], 0);
    assert_eq!(report["summary"]["false_accepts"], 0);

    let _ = fs::remove_file(first);
    let _ = fs::remove_file(second);
}

#[test]
fn early_flow_defect_fields_are_backward_compatible_and_legacy_rows_fail_closed() {
    use rodas5p_core::WorkCounters;
    use rodas5p_integrators::{
        AdaptiveEarlyFlowDefectAttempt, AdaptiveEarlyFlowDefectOutcome,
        AdaptiveFusedExponentialDiagnostics, FusedExponentialStepReport,
    };

    let report = FusedExponentialStepReport {
        method: "legacy-fused-step".into(),
        y_new: vec![1.0],
        y_embedded: Some(vec![1.0]),
        error_estimate: Some(vec![0.0]),
        logical_critical_depth: 3,
        fused_phi_reports: Vec::new(),
        work: WorkCounters::default(),
        early_flow_defect: None,
    };
    let legacy_step_json = serde_json::to_value(&report).unwrap();
    assert!(legacy_step_json.get("early_flow_defect").is_none());
    let legacy_step: FusedExponentialStepReport = serde_json::from_value(legacy_step_json).unwrap();
    assert!(legacy_step.early_flow_defect.is_none());

    let legacy_diagnostics_json = serde_json::json!({
        "attempts": 1,
        "accepted_steps": 1,
        "rejected_steps": 0,
        "accepted_step_sizes": [0.01],
        "rejected_step_sizes": [],
        "time_error_norms": [0.0],
        "phi_error_norms": [0.0],
        "total_error_norms": [0.0],
        "maximum_krylov_dimensions": [1],
        "phi_substeps": [1]
    });
    let legacy_diagnostics: AdaptiveFusedExponentialDiagnostics =
        serde_json::from_value(legacy_diagnostics_json).unwrap();
    assert!(legacy_diagnostics.early_flow_defect_attempts.is_empty());

    let legacy_attempt_json = serde_json::json!({
        "t": 0.0,
        "step_size": 0.01,
        "output_clipped": false
    });
    let legacy_attempt: AdaptiveEarlyFlowDefectAttempt =
        serde_json::from_value(legacy_attempt_json).unwrap();
    assert_eq!(
        legacy_attempt.outcome,
        AdaptiveEarlyFlowDefectOutcome::LegacyUnclassified
    );
    assert!(legacy_attempt.telemetry.is_none());
    assert!(legacy_attempt.trial_work.is_none());
    assert!(legacy_attempt.failure.is_none());
}

#[test]
fn early_defect_attempt_geometry_cli_is_threshold_free_and_machine_readable() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = temp_path("early-defect-attempt-geometry.json");

    let status = Command::new(binary)
        .args([
            "generic-early-defect-attempt-geometry",
            "--profile",
            "smoke",
            "--output",
        ])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());

    let report: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(report["schema"], "g4-s5b3-attempt-geometry-raw-v1");
    assert_eq!(report["status"], "pass");
    assert_eq!(report["profile"], "smoke");
    assert_eq!(report["active_switching"], false);
    assert_eq!(report["early_abort"], false);
    assert_eq!(report["threshold_selected"], false);
    assert!(report["selected_threshold"].is_null());
    assert_eq!(report["hard_gates"]["expected_trajectories"], 3);
    assert_eq!(report["hard_gates"]["observed_trajectories"], 3);
    assert_eq!(report["hard_gates"]["passed"], true);
    assert_eq!(report["overhead"]["measured_pairs"], 1);

    let _ = fs::remove_file(output);
}

#[test]
fn tolerance_scaled_early_defect_cli_is_threshold_free_and_machine_readable() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = temp_path("tolerance-scaled-early-defect.json");

    let status = Command::new(binary)
        .args([
            "generic-tolerance-scaled-early-defect",
            "--profile",
            "smoke",
            "--output",
        ])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());

    let report: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(
        report["schema"],
        "p1-00-tolerance-scaled-early-defect-raw-v1"
    );
    assert_eq!(report["status"], "pass");
    assert_eq!(report["profile"], "smoke");
    assert_eq!(report["active_switching"], false);
    assert_eq!(report["early_abort"], false);
    assert_eq!(report["threshold_selected"], false);
    assert!(report["selected_threshold"].is_null());
    assert_eq!(report["hard_gates"]["expected_trajectories"], 3);
    assert_eq!(report["hard_gates"]["observed_trajectories"], 3);
    assert_eq!(report["hard_gates"]["passed"], true);
    assert_eq!(report["overhead"]["measured_pairs"], 1);

    let attempts = report["attempts"].as_array().unwrap();
    assert!(!attempts.is_empty());
    assert!(attempts.iter().all(|row| row["eta_c2"].is_number()));
    assert!(attempts.iter().all(|row| row["rho_c2_wrms"].is_number()));

    let _ = fs::remove_file(output);
}

#[test]
fn generic_policy_redesign_atlas_command_is_registered() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = std::process::Command::new(binary)
        .args(["generic-policy-redesign-atlas", "--help"])
        .output()
        .expect("run policy redesign help");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("calibration"));
    assert!(stdout.contains("holdout"));
    assert!(stdout.contains("--family"));
}

#[test]
fn generic_policy_redesign_attempt_trace_command_is_registered() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args(["generic-policy-redesign-attempt-trace", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("--family"));
    assert!(stdout.contains("--output"));
}

#[test]
fn generic_policy_redesign_actual_prefix_command_is_registered() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args(["generic-policy-redesign-actual-prefix", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("--family"));
    assert!(stdout.contains("--policy"));
    assert!(stdout.contains("--output"));
}

#[test]
fn generic_policy_redesign_level2_prefix_command_is_registered() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args(["generic-policy-redesign-level2-prefix", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("--family"));
    assert!(stdout.contains("--policy"));
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("discovery96"));
    assert!(stdout.contains("discovery256"));
}

#[test]
fn enforced_prefix_budget_cli_is_registered() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args(["generic-enforced-prefix-budget", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("--family"));
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("calibration96"));
    assert!(stdout.contains("calibration192"));
    assert!(stdout.contains("calibration256"));
    assert!(stdout.contains("holdout384"));
    assert!(stdout.contains("holdout320"));
}

#[test]
fn enforced_prefix_budget_cli_emits_transactional_fields() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = temp_path("enforced-prefix-budget.json");
    let status = Command::new(binary)
        .args([
            "generic-enforced-prefix-budget",
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
    assert_eq!(report["schema"], "g4-s5b0-enforced-prefix-budget-v1");
    assert_eq!(report["switching_active"], false);
    assert_eq!(report["runtime_full_e_continuations"], 0);
    assert_eq!(report["budget_breaches"], 0);
    assert!(report.get("budget_exhaustions").is_some());
    let _ = fs::remove_file(output);
}

#[test]
fn stage_growth_safety_cli_exposes_independent_calibration192_profile() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args(["generic-stage-growth-safety-audit", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("calibration192"));
}

#[test]
fn stage_growth_safety_audit_cli_emits_explicit_budget_and_audit_fields() {
    let binary = env!("CARGO_BIN_EXE_rodas5p");
    let output = temp_path("stage-growth-safety-audit.json");
    let status = Command::new(binary)
        .args([
            "generic-stage-growth-safety-audit",
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
    assert_eq!(report["schema"], "g4-s5b0-stage-growth-safety-audit-v1");
    assert_eq!(report["profile"], "stage-growth-calibration-96");
    assert_eq!(report["switching_active"], false);
    assert_eq!(report["runtime_full_e_continuations"], 0);
    assert_eq!(report["budget_breaches"], 0);
    let _ = fs::remove_file(output);
}
