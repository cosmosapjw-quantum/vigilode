//! CLI and in-process contracts of the component-triage harness (research
//! node `research/component_triage_20261019`, `HARNESS_CONTRACT.md`).

#[path = "../src/stiff_benchmark.rs"]
#[allow(dead_code, unused_imports)]
mod stiff_benchmark;

#[path = "../src/triage_arms.rs"]
#[allow(dead_code, unused_imports)]
mod triage_arms;

#[path = "../src/triage.rs"]
#[allow(dead_code, unused_imports)]
mod triage;

use std::{collections::HashSet, process::Command};

use serde_json::{Value, json};

const FAKE_SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// The triage binary with the driver's environment set, so no row hashes
/// the (large, unoptimized) test binary.
fn rodas5p() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rodas5p"));
    command
        .env("VIGILODE_BINARY_SHA256", FAKE_SHA)
        .env("VIGILODE_BUILD_PROFILE", "dev")
        .env("VIGILODE_SOURCE_COMMIT", "a".repeat(40))
        .env("VIGILODE_TREE_CLEAN", "false");
    command
}

fn run_json(command: &mut Command) -> Value {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "exit {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn decode_hex64(text: &str) -> f64 {
    assert_eq!(text.len(), 16, "{text}");
    assert!(
        text.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
    f64::from_bits(u64::from_str_radix(text, 16).unwrap())
}

/// Every field and type of `vigilode-triage-row-v1`.
fn assert_row_schema(row: &Value) {
    assert_eq!(row["schema"], "vigilode-triage-row-v1");
    assert!(row["arm"].is_string());
    let components = row["components"].as_array().unwrap();
    assert!(!components.is_empty() && components.iter().all(Value::is_string));
    assert!(row["problem"].is_string());
    assert!(["G", "P"].contains(&row["tier"].as_str().unwrap()));
    let dimension = row["dimension"].as_u64().unwrap() as usize;
    for key in ["rtol", "atol", "h0_scale", "h0"] {
        assert!(row[key].as_f64().unwrap() > 0.0, "{key}");
    }
    assert!(["endpoint", "uniform101"].contains(&row["grid"].as_str().unwrap()));
    let status = row["status"].as_str().unwrap();
    assert!(["ok", "solver_failure", "refused", "error"].contains(&status));
    assert_eq!(row["success"].as_bool().unwrap(), status == "ok");
    assert_eq!(row["error"].is_null(), status == "ok", "{row}");
    for key in [
        "attempts",
        "accepted_steps",
        "rejected_steps",
        "repetitions",
    ] {
        assert!(row[key].is_u64(), "{key}");
    }
    let times = row["output_times"].as_array().unwrap();
    let states = row["states_hex"].as_array().unwrap();
    assert_eq!(times.len(), states.len());
    assert!(times.iter().all(Value::is_f64));
    for state in states {
        let state = state.as_array().unwrap();
        assert_eq!(state.len(), dimension);
        for value in state {
            decode_hex64(value.as_str().unwrap());
        }
    }
    let counters = row["counters"].as_object().unwrap();
    for field in triage::work_counter_fields() {
        assert!(counters[*field].is_u64(), "counter {field}");
    }
    assert_eq!(counters.len(), triage::work_counter_fields().len());
    assert!(row["extra_work"].is_null() || row["extra_work"].is_object());
    assert_eq!(row["counter_semantics"], "work-counters-v2-attempted");
    assert!(row["deterministic"].is_boolean());
    assert!(row["binary"]["sha256"].is_string() || row["binary"]["sha256"].is_null());
    assert!(row["binary"]["profile"].is_string());
    assert!(row["source"].get("commit").is_some() && row["source"].get("tree_clean").is_some());
}

#[test]
fn triage_list_parses_with_unique_ids() {
    let list = run_json(rodas5p().arg("triage-list"));
    assert_eq!(list["schema"], "vigilode-triage-list-v1");
    let arms = list["arms"].as_array().unwrap();
    let problems = list["problems"].as_array().unwrap();
    let arm_ids: HashSet<_> = arms.iter().map(|a| a["id"].as_str().unwrap()).collect();
    let problem_ids: HashSet<_> = problems.iter().map(|p| p["id"].as_str().unwrap()).collect();
    assert_eq!(arm_ids.len(), arms.len(), "duplicate arm id");
    assert_eq!(problem_ids.len(), problems.len(), "duplicate problem id");
    for arm in arms {
        let components = arm["components"].as_array().unwrap();
        assert!(!components.is_empty());
        let unique: HashSet<_> = components.iter().map(|c| c.as_str().unwrap()).collect();
        assert_eq!(unique.len(), components.len(), "{}", arm["id"]);
        assert!(
            ["analytic", "jvp", "any"].contains(&arm["requires"]["jacobian"].as_str().unwrap())
        );
        assert!(arm["requires"]["mass"].is_boolean());
        if arm["wired"] == true {
            assert!(arm["unwired_reason"].is_null());
            assert!(
                !arm["applicable_problems"].as_array().unwrap().is_empty(),
                "{} applies to no problem",
                arm["id"]
            );
        } else {
            assert!(arm["unwired_reason"].is_string());
        }
        for p in arm["applicable_problems"].as_array().unwrap() {
            assert!(problem_ids.contains(p.as_str().unwrap()));
        }
    }
    for problem in problems {
        let span = problem["t_span"].as_array().unwrap();
        assert!(span[1].as_f64().unwrap() > span[0].as_f64().unwrap());
        assert!(problem["dimension"].as_u64().unwrap() >= 1);
        assert!(["analytic", "jvp"].contains(&problem["jacobian"].as_str().unwrap()));
        for key in ["jvp", "mass", "exact_solution", "oscillatory"] {
            assert!(problem[key].is_boolean(), "{key}");
        }
        assert!(problem["atol_scale"].as_f64().unwrap() > 0.0);
        assert!(problem["breakpoints"].is_array() && problem["source"].is_string());
    }
    // The registered corpus is present.
    for id in [
        "robertson",
        "prothero-robinson-nl",
        "brusselator-1d-1000",
        "gray-scott-64",
        "semilinear-advdiff-2d-1536",
    ] {
        assert!(problem_ids.contains(id), "{id}");
    }
}

#[test]
fn seq_direct_on_robertson_gives_101_finite_states() {
    let row = run_json(rodas5p().args([
        "triage-run",
        "--arm",
        "seq-direct",
        "--problem",
        "robertson",
        "--rtol",
        "1e-4",
        "--grid",
        "uniform101",
        "--h0-scale",
        "1",
        "--repetitions",
        "2",
        "--campaign",
        "ct-test",
    ]));
    assert_row_schema(&row);
    assert_eq!(row["status"], "ok", "{row}");
    assert_eq!(row["campaign"], "ct-test");
    assert_eq!(row["deterministic"], true);
    assert_eq!(row["repetitions"], 2);
    assert_eq!(row["binary"]["sha256"], FAKE_SHA);
    assert_eq!(row["source"]["tree_clean"], false);
    assert_eq!(row["atol"].as_f64().unwrap(), 1e-4 * 1e-4);
    assert_eq!(row["h0"].as_f64().unwrap(), 1e-6 * 40.0);
    let times: Vec<f64> = row["output_times"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_f64().unwrap())
        .collect();
    assert_eq!(times, triage::Grid::Uniform101.times((0.0, 40.0)));
    let states = row["states_hex"].as_array().unwrap();
    assert_eq!(states.len(), 101);
    for state in states {
        for value in state.as_array().unwrap() {
            assert!(decode_hex64(value.as_str().unwrap()).is_finite());
        }
    }
    assert!(row["counters"]["rhs_evaluations"].as_u64().unwrap() > 0);
}

#[test]
fn usage_errors_exit_with_code_2() {
    for args in [
        &["--arm", "no-such-arm", "--problem", "robertson"][..],
        &["--arm", "seq-direct", "--problem", "no-such-problem"][..],
        // Inapplicable: the banded arm needs a declared band.
        &["--arm", "fast-banded-indexed", "--problem", "robertson"][..],
        // Unwired.
        &["--arm", "fast-small-batch4", "--problem", "robertson"][..],
    ] {
        let output = rodas5p()
            .arg("triage-run")
            .args(args)
            .args(["--rtol", "1e-3", "--grid", "endpoint"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
    }
}

/// The first applicable G-tier problem of dimension at most 32, else the
/// first applicable problem.
fn first_small_problem(arm: &triage_arms::ArmSpec) -> &'static str {
    let built: Vec<_> = [
        "robertson",
        "hires",
        "van-der-pol-mu1000",
        "oregonator",
        "pollution",
        "prothero-robinson-nl",
        "scalar-linear",
        "brusselator-1d-50",
    ]
    .iter()
    .map(|id| triage::TriageProblem::build(triage::find_problem(id).unwrap()).unwrap())
    .collect();
    built
        .iter()
        .find(|p| {
            (p.spec.tier == "G" || p.spec.id == "brusselator-1d-50")
                && triage_arms::applicable(&arm.requires, &p.features())
        })
        .map(|p| p.spec.id)
        .unwrap_or_else(|| panic!("no small problem for {}", arm.id))
}

#[test]
fn every_wired_arm_yields_a_schema_valid_row() {
    let mut statuses = Vec::new();
    for arm in triage_arms::arms().iter().filter(|arm| arm.wired) {
        let problem = first_small_problem(arm);
        let args = triage::CellArgs {
            arm: arm.id.into(),
            problem: problem.into(),
            rtol: 1.0e-3,
            grid: triage::Grid::Endpoint,
            h0_scale: 1.0,
        };
        let mut cell = triage::Cell::new(&args).unwrap();
        let run = cell.integrate();
        let row = triage::row_json(
            &cell,
            &run,
            true,
            1,
            None,
            json!({"sha256": FAKE_SHA, "profile": "dev"}),
            json!({"commit": null, "tree_clean": null}),
        );
        assert_row_schema(&row);
        assert_eq!(row["arm"], arm.id);
        assert_eq!(
            row["components"].as_array().unwrap().len(),
            arm.components.len()
        );
        statuses.push((arm.id, problem, row["status"].as_str().unwrap().to_owned()));
    }
    // The registered refusal of the sequential solver aside, the arms
    // complete their first small problem at rtol 1e-3.
    for (arm, problem, status) in &statuses {
        if !arm.starts_with("seq-mf-gmres-x0-prevstep") {
            assert_eq!(status, "ok", "{arm} on {problem}");
        }
    }
}

#[test]
fn prothero_robinson_nl_matches_its_exact_solution_at_rtol_1e_8() {
    let exact = run_json(rodas5p().args([
        "triage-exact",
        "--problem",
        "prothero-robinson-nl",
        "--grid",
        "uniform101",
    ]));
    assert_eq!(exact["schema"], "vigilode-triage-exact-v1");
    let row = run_json(rodas5p().args([
        "triage-run",
        "--arm",
        "seq-direct",
        "--problem",
        "prothero-robinson-nl",
        "--rtol",
        "1e-8",
        "--grid",
        "uniform101",
    ]));
    assert_eq!(row["status"], "ok", "{row}");
    assert_eq!(exact["output_times"], row["output_times"]);
    let decode = |v: &Value| -> Vec<f64> {
        v.as_array()
            .unwrap()
            .iter()
            .flat_map(|s| {
                s.as_array()
                    .unwrap()
                    .iter()
                    .map(|x| decode_hex64(x.as_str().unwrap()))
            })
            .collect()
    };
    let (reference, solution) = (decode(&exact["states_hex"]), decode(&row["states_hex"]));
    assert_eq!(reference.len(), 101);
    for (t, (a, b)) in reference.iter().zip(&solution).enumerate() {
        assert!((a - b).abs() <= 1.0e-7, "output {t}: exact {a}, solver {b}");
    }
    // exact sin(t) at the grid
    assert_eq!(reference[100], 1.0_f64.sin());
    // A problem without an exact solution is a usage error.
    let output = rodas5p()
        .args([
            "triage-exact",
            "--problem",
            "robertson",
            "--grid",
            "endpoint",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn parity_samples_are_hex_with_the_analytic_jacobian() {
    let parity = run_json(rodas5p().args(["triage-parity", "--problem", "robertson"]));
    assert_eq!(parity["schema"], "vigilode-triage-parity-v1");
    let samples = parity["samples"].as_array().unwrap();
    assert_eq!(samples.len(), 4);
    for sample in samples {
        assert_eq!(sample["f_hex"].as_array().unwrap().len(), 3);
        assert_eq!(sample["jacobian"]["format"], "dense");
        assert_eq!(sample["jacobian"]["rows_hex"].as_array().unwrap().len(), 3);
        assert_eq!(sample["jvp_hex"].as_array().unwrap().len(), 3);
    }
    let jvp_only = run_json(rodas5p().args(["triage-parity", "--problem", "pollution"]));
    assert!(jvp_only["samples"][0]["jacobian"].is_null());
    assert_eq!(
        jvp_only["samples"][0]["jvp_hex"].as_array().unwrap().len(),
        20
    );
}

#[test]
fn profile_run_reports_the_first_run_and_accepts_zero_repetitions() {
    let zero = run_json(rodas5p().args([
        "triage-profile-run",
        "--arm",
        "fast",
        "--problem",
        "robertson",
        "--rtol",
        "1e-4",
        "--grid",
        "endpoint",
        "--repetitions",
        "0",
    ]));
    assert_eq!(zero["schema"], "vigilode-triage-profile-v1");
    assert!(zero["counters"].is_null() && zero["success"].is_null());
    let two = run_json(rodas5p().args([
        "triage-profile-run",
        "--arm",
        "fast",
        "--problem",
        "robertson",
        "--rtol",
        "1e-4",
        "--grid",
        "endpoint",
        "--repetitions",
        "2",
    ]));
    assert_eq!(two["success"], true);
    assert_eq!(two["deterministic"], true);
    assert!(two["attempts"].as_u64().unwrap() > 0);
    assert!(two["counters"]["rhs_evaluations"].as_u64().unwrap() > 0);
}

#[test]
fn segmented_problems_drop_unrequested_breakpoints() {
    // brusselator-2d integrates [0, 1.1] and [1.1, 11.5]; 1.1 is not on the
    // endpoint grid, so the row reports t0 and tf only.
    let args = triage::CellArgs {
        arm: "mf-into".into(),
        problem: "brusselator-2d".into(),
        rtol: 1.0e-3,
        grid: triage::Grid::Endpoint,
        h0_scale: 1.0,
    };
    let mut cell = triage::Cell::new(&args).unwrap();
    assert_eq!(cell.segment_count(), 2);
    let run = cell.integrate();
    assert_eq!(run.status, triage::Status::Ok, "{:?}", run.error);
    assert_eq!(run.output_times, vec![0.0, 11.5]);
    assert_eq!(run.segments_completed, 2);
}

#[test]
fn allocation_audit_triage_prints_raw_windows() {
    let output = Command::new(env!("CARGO_BIN_EXE_rodas5p-allocation-audit"))
        .args([
            "triage",
            "--arm",
            "fast",
            "--problem",
            "robertson",
            "--rtol",
            "1e-4",
            "--grid",
            "endpoint",
            "--h0-scale",
            "1",
            "--warmups",
            "1",
            "--windows",
            "3",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(doc["schema"], "vigilode-triage-alloc-v1");
    assert_eq!(doc["warmups"], 1);
    assert_eq!(doc["status"], "ok");
    let windows = doc["windows"].as_array().unwrap();
    assert_eq!(windows.len(), 3);
    for window in windows.iter().chain([&doc["cold"]]) {
        for key in ["allocations", "deallocations", "allocated_bytes"] {
            assert!(window[key].is_u64(), "{key}: {window}");
        }
        assert!(window["peak_live_bytes"].is_i64(), "{window}");
    }
    // The cold window holds the first run; a warm window is one integration.
    assert!(doc["cold"]["allocations"].as_u64().unwrap() > 0);
    assert_eq!(windows[1], windows[2]);
}

#[test]
fn work_counter_fields_include_the_skipped_ones() {
    let fields = triage::work_counter_fields();
    for field in ["rhs_calls", "jacobian_matvecs", "recycle_update_refreshes"] {
        assert!(fields.contains(&field), "{field}");
    }
    let value = triage::counters_json(&rodas5p_core::WorkCounters::default());
    assert_eq!(value.as_object().unwrap().len(), fields.len());
}
