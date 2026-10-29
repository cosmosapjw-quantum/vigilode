#[path = "../examples/reaudit_bound_clients_support/mod.rs"]
mod support;
#[test]
fn fixed_cli_and_scopes_do_not_allow_rebudgeting() {
    assert_eq!(support::manifest()["cases"].as_array().unwrap().len(), 3);
    let good = [
        "--case",
        "allocation",
        "--arm",
        "optimized",
        "--threads",
        "1",
    ]
    .map(String::from);
    assert!(support::parse(&good).is_ok());
    let mut bad = good;
    bad[0] = "--budget".into();
    assert!(support::parse(&bad).is_err());
}
#[test]
fn same_algebraic_error_budget_has_exact_32_vs_40_work_witness() {
    let a = support::run("allocation", "uniform", 1);
    let b = support::run("allocation", "optimized", 1);
    for row in [&a, &b] {
        assert_eq!(row["status"], "PASS_VALIDATED_ALLOCATION", "{row}");
        assert_eq!(row["full_curve_certified"], false);
        assert!(row["achieved_bound"].as_f64().unwrap() <= 0.5);
    }
    assert_eq!(a["input_sha256"], b["input_sha256"]);
    assert_eq!(a["work"]["validated_model_work_upper"], 40.0);
    assert_eq!(b["work"]["validated_model_work_upper"], 32.0);
}
#[test]
fn global_provider_and_replay_bind_same_completed_curve() {
    let a = support::run("global", "provider", 1);
    let b = support::run("global", "replay", 1);
    for row in [&a, &b] {
        assert_eq!(row["status"], "PASS_CERTIFIED_FULL_CURVE", "{row}");
        assert_eq!(row["full_curve_certified"], true);
        assert!(row["achieved_bound"].as_f64().unwrap() <= 1e-4);
    }
    assert_eq!(a["input_sha256"], b["input_sha256"]);
    assert_eq!(a["outputs"], b["outputs"]);
    assert_eq!(a["work"]["solver"], b["work"]["solver"]);
}
#[test]
fn actual_diagonal_driver_preserves_budget_without_resolvent_rhs_solves() {
    let a = support::run("diagonal", "resolvent", 1);
    let b = support::run("diagonal", "diagonal", 1);
    for row in [&a, &b] {
        assert_eq!(row["status"], "PASS_CERTIFIED_ENDPOINTS", "{row}");
        assert_eq!(row["full_curve_certified"], false);
        assert_eq!(row["work"]["fallback_count"], 0);
    }
    assert_eq!(a["input_sha256"], b["input_sha256"]);
    assert!(a["work"]["staleness"]["resolvent_solves"].as_u64().unwrap() > 0);
    assert_eq!(b["work"]["staleness"]["resolvent_solves"], 0);
    assert!(b["work"]["staleness"]["diagonal_checks"].as_u64().unwrap() > 0);
    assert_eq!(b["stage_policy"]["used_by_driver"], false);
}
