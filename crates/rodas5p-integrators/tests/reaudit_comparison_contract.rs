#[path = "../examples/reaudit_comparison_support/mod.rs"]
mod support;

#[test]
fn frozen_grid_and_cli_refuse_unknown_or_rebudgeted_inputs() {
    let manifest = support::manifest();
    assert_eq!(manifest["suites"].as_array().unwrap().len(), 3);
    assert_eq!(manifest["full_curve_certified"], false);
    let args = vec![
        "--suite",
        "polynomial",
        "--case",
        "p4",
        "--arm",
        "protected",
        "--threads",
        "1",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    assert!(support::parse_args(&args).is_ok());
    let mut bad = args.clone();
    bad[7] = "0".into();
    assert!(support::parse_args(&bad).is_err());
    bad = args.clone();
    bad[0] = "--budget".into();
    assert!(support::parse_args(&bad).is_err());
    assert!(support::request("polynomial", "p5").is_err());
}

#[test]
fn protected_and_krylov_share_original_ode_endpoint_certificate() {
    let direct = support::run("polynomial", "p4", "protected", 1);
    let krylov = support::run("polynomial", "p4", "krylov", 1);
    assert_eq!(direct["status"], "PASS_CERTIFIED_ENDPOINTS");
    assert_eq!(krylov["status"], "PASS_CERTIFIED_ENDPOINTS");
    assert_eq!(direct["input_sha256"], krylov["input_sha256"]);
    assert_eq!(direct["original_budget"], krylov["original_budget"]);
    assert_eq!(direct["full_curve_certified"], false);
    for row in [&direct, &krylov] {
        assert_eq!(row["outputs"].as_array().unwrap().len(), 4);
        assert!(row["achieved_bound"].as_f64().unwrap() <= 1e-7);
        assert_eq!(row["work"]["endpoint_oracle_components"], 16);
        assert_eq!(row["speed_promotion"], "HOLD");
    }
}

#[test]
fn nonlinear_and_stage_scopes_and_inputs_cannot_be_conflated() {
    let h = support::request("homotopy", "h3").unwrap();
    let k = support::request("actual-k", "k2").unwrap();
    assert_ne!(h["original_budget_bits"], k["original_budget_bits"]);
    assert_eq!(h["schedule_choices"][0], serde_json::json!([1.0]));
    assert_eq!(h["schedule_choices"][1], serde_json::json!([0.0, 0.5, 1.0]));
    let refused = support::run("actual-k", "k2", "polynomial", 1);
    assert_eq!(refused["status"], "REFUSED");
    assert_eq!(refused["achieved_bound"], serde_json::Value::Null);
    assert_eq!(refused["full_curve_certified"], false);
}

#[test]
fn guarded_polynomial_trajectory_keeps_budget_and_all_requested_endpoints() {
    let row = support::run("polynomial", "p4", "router", 1);
    assert_eq!(row["status"], "PASS_CERTIFIED_ENDPOINTS", "{row}");
    assert_eq!(row["outputs"].as_array().unwrap().len(), 4);
    assert!(row["work"]["cumulative_action_radius"].as_f64().unwrap() <= 1e-7);
    let reports = row["work"]["attempt_reports"].as_array().unwrap();
    assert_eq!(reports.len(), 4);
    for report in reports {
        assert_eq!(report["attempts"].as_array().unwrap().len(), 3);
    }
    assert!(row["work"]["local_action_budget"].as_f64().unwrap() <= 1e-7 / 8.0);
    assert_eq!(row["process_end_to_end_timed"], false);
}
