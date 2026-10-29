//! Research node `research/alg05_predictive_controller_v2_20261010` (ALG05,
//! the second test of ALG02's predictive step-size control): the
//! `PredictiveCapped` gaps fixed in an opt-in `PredictiveCapped2`,
//! re-measured on fresh initial-step seeds.
//!
//! Dense cells, as ALG02: the dense fast driver
//! (`integrate_rodas5p_fast_observed`) on van-der-pol-mu1000, hires,
//! robertson and brusselator-1d-50 from the stiff benchmark's
//! `benchmark_problems()`, with the benchmark's adaptive configuration
//! (atol = rtol * atol_scale, min step 1e-14, max step = span, 1,000,000
//! attempts, everything else the default) except the initial step, which is
//! the seed. Endpoint error: `max_i |y_i - r_i| / max(|r_i|, 1e-10)` against
//! the NATIVE.json references. Ladders as ALG02 (quarter decades, 17 points
//! for van der Pol and Brusselator-50, 29 for HIRES and Robertson). Fresh
//! seeds h0 in {3e-6, 3e-5, 3e-3}, none of which ALG02 used.
//!
//! `export_base` runs arm `I` on every cell and was run on the test-only
//! commit's solver source, before any source change of this node.
//! `export_runs` runs arms `I`, `PREDcap` (`PredictiveCapped`, ALG02's rule,
//! reported) and `PREDcap2` (`PredictiveCapped2`, gated) on every cell.

#[path = "../src/stiff_benchmark.rs"]
#[allow(dead_code, unused_imports)]
mod stiff_benchmark;

use rodas5p_core::WorkCounters;
use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerKind, OutputSchedule, Rodas5pFastResult,
    integrate_rodas5p_fast_observed,
};
use serde_json::{Value, json};
use stiff_benchmark::{BenchmarkProblem, benchmark_problems};

/// The problems, in export order.
const PROBLEMS: [&str; 4] = [
    "van-der-pol-mu1000",
    "hires",
    "robertson",
    "brusselator-1d-50",
];
/// The fresh initial-step seeds (ALG02 used 1e-6, 1e-4, 1e-2).
const SEEDS: [f64; 3] = [3.0e-6, 3.0e-5, 3.0e-3];
/// The stiff benchmark's attempt budget.
const MAX_ATTEMPTS: usize = 1_000_000;
/// Floor of the benchmark's componentwise endpoint error.
const ERROR_FLOOR: f64 = 1.0e-10;

/// `10^(-q/4)`; whole decades are the exact literals.
fn quarter_decade(q: u32) -> f64 {
    if q.is_multiple_of(4) {
        format!("1e-{}", q / 4).parse().unwrap()
    } else {
        10.0_f64.powf(-(q as f64) / 4.0)
    }
}

/// The registered quarter-decade ladder of a problem: 1e-3 to 1e-7
/// (17 points) for van der Pol and Brusselator-50, 1e-3 to 1e-10 (29 points)
/// for HIRES and Robertson.
fn ladder(problem: &str) -> Vec<f64> {
    let last = match problem {
        "van-der-pol-mu1000" | "brusselator-1d-50" => 28,
        "hires" | "robertson" => 40,
        other => panic!("no ladder for {other}"),
    };
    (12..=last).map(quarter_decade).collect()
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| format!("{:016x}", x.to_bits())).collect()
}

/// The benchmark's endpoint error metric.
fn endpoint_error(y: &[f64], reference: &[f64]) -> f64 {
    assert_eq!(y.len(), reference.len());
    y.iter()
        .zip(reference)
        .map(|(a, r)| (a - r).abs() / r.abs().max(ERROR_FLOOR))
        .fold(0.0, f64::max)
}

fn workspace_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// NATIVE.json reference final states by problem id.
fn native_reference(problem: &str) -> Vec<f64> {
    let path = workspace_path("research/stiff_native_benchmark_20261001/NATIVE.json");
    let native: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    native["references"][problem]["final_state"]
        .as_array()
        .unwrap_or_else(|| panic!("no NATIVE reference for {problem}"))
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

/// Writes `value` to the path in `variable`, if set; a relative path is
/// taken from the workspace root. An existing file is never overwritten.
fn write_output(variable: &str, value: &Value) {
    if let Ok(path) = std::env::var(variable) {
        let path = workspace_path(&path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string_pretty(value).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

fn problems() -> Vec<BenchmarkProblem> {
    let mut all = benchmark_problems().unwrap();
    PROBLEMS
        .iter()
        .map(|id| {
            let at = all.iter().position(|p| p.id == *id).unwrap();
            all.swap_remove(at)
        })
        .collect()
}

/// The stiff benchmark's adaptive configuration with the seed as initial
/// step; the production Integral controller.
fn base_config(problem: &BenchmarkProblem, rtol: f64, seed: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * problem.atol_scale,
        rtol,
        initial_step: seed,
        min_step: 1.0e-14,
        max_step: problem.t_span.1 - problem.t_span.0,
        max_attempts: MAX_ATTEMPTS,
        ..AdaptiveStepConfig::default()
    }
}

/// The configuration of one arm.
fn arm_config(arm: &str, base: AdaptiveStepConfig) -> AdaptiveStepConfig {
    match arm {
        "I" => base,
        "PREDcap" => AdaptiveStepConfig {
            controller: ControllerKind::PredictiveCapped,
            ..base
        },
        "PREDcap2" => AdaptiveStepConfig {
            controller: ControllerKind::PredictiveCapped2,
            ..base
        },
        other => panic!("unknown arm {other}"),
    }
}

fn fast_row(
    result: &rodas5p_core::CoreResult<Rodas5pFastResult>,
    reference: &[f64],
) -> serde_json::Map<String, Value> {
    let value = match result {
        Ok(r) => {
            let y = r.observed.y.last().unwrap();
            let complete = r.observed.success;
            let c: &WorkCounters = &r.observed.counters;
            json!({
                "ok": true,
                "success": complete,
                "message": r.observed.message,
                "attempts": r.attempts,
                "accepted": r.accepted_steps,
                "rejected": r.rejected_steps,
                "jacobian_reuses": r.jacobian_reuses,
                "internal_steps": r.observed.internal_steps,
                "output_clipped_steps": r.observed.output_clipped_steps,
                "rhs_evaluations": c.rhs_evaluations,
                "lu_factorizations": c.direct_factorizations,
                "jacobian_builds": c.jacobian_builds,
                "linear_solve_failures": c.linear_solve_failures,
                "nonfinite_step_failures": c.nonfinite_step_failures,
                "counters": serde_json::to_value(c).unwrap(),
                "t_last": hexes(&[*r.observed.t.last().unwrap()]),
                "final_state": hexes(y),
                "error": if complete { json!(endpoint_error(y, reference)) } else { Value::Null },
            })
        }
        Err(e) => json!({"ok": false, "success": false, "error_message": e.to_string()}),
    };
    match value {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}

fn run(problem: &BenchmarkProblem, arm: &str, rtol: f64, seed: f64, reference: &[f64]) -> Value {
    let config = arm_config(arm, base_config(problem, rtol, seed));
    let schedule = OutputSchedule::new(vec![problem.t_span.0, problem.t_span.1]).unwrap();
    let result = integrate_rodas5p_fast_observed(
        &problem.problem,
        problem.t_span,
        &problem.y0,
        &config,
        &schedule,
    );
    let mut row = serde_json::Map::new();
    row.insert("problem".into(), json!(problem.id));
    row.insert("arm".into(), json!(arm));
    row.insert("rtol".into(), json!(rtol));
    row.insert("atol".into(), json!(config.atol));
    row.insert("seed".into(), json!(seed));
    row.extend(fast_row(&result, reference));
    Value::Object(row)
}

/// Every cell of `arm`, in the registered order (problem, seed, rtol).
fn rows(arm: &str) -> Vec<Value> {
    let mut rows = Vec::new();
    for problem in problems() {
        let reference = native_reference(problem.id);
        for seed in SEEDS {
            for rtol in ladder(problem.id) {
                let row = run(&problem, arm, rtol, seed, &reference);
                eprintln!(
                    "{} {arm} seed={seed:e} rtol={rtol:e}: attempts {} rejected {} error {}",
                    problem.id, row["attempts"], row["rejected"], row["error"]
                );
                rows.push(row);
            }
        }
    }
    rows
}

#[test]
fn ladders_seeds_and_cells_are_the_registered_ones() {
    assert_eq!(ladder("van-der-pol-mu1000").len(), 17);
    assert_eq!(ladder("brusselator-1d-50").len(), 17);
    assert_eq!(ladder("hires").len(), 29);
    assert_eq!(ladder("robertson").len(), 29);
    let hires = ladder("hires");
    assert_eq!(hires[0], 1.0e-3);
    assert_eq!(hires[8], 1.0e-5);
    assert_eq!(hires[28], 1.0e-10);
    assert_eq!(ladder("van-der-pol-mu1000")[16], 1.0e-7);
    for l in PROBLEMS.map(ladder) {
        for pair in l.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!((ratio - 10.0_f64.powf(0.25)).abs() < 1.0e-12, "{pair:?}");
        }
    }
    // Fresh seeds: none of ALG02's.
    for old in [1.0e-6, 1.0e-4, 1.0e-2] {
        assert!(!SEEDS.contains(&old));
    }
    let problems = problems();
    assert_eq!(problems.iter().map(|p| p.id).collect::<Vec<_>>(), PROBLEMS);
    let cells: usize = problems.iter().map(|p| ladder(p.id).len()).sum();
    assert_eq!(cells * SEEDS.len(), 276);
    for problem in &problems {
        assert_eq!(native_reference(problem.id).len(), problem.y0.len());
        for seed in SEEDS {
            let config = base_config(problem, 1.0e-6, seed);
            config.validate().unwrap();
            assert_eq!(config.controller, ControllerKind::Integral);
            assert_eq!(config.safety, 0.9);
            assert_eq!(config.initial_step, seed);
        }
    }
}

/// Base export (arm `I` on every cell), run on the test-only commit's
/// solver source before any source change of this node.
#[test]
#[ignore = "base export of research/alg05_predictive_controller_v2_20261010; release build; set ALG05_BASE"]
fn export_base() {
    let rows = rows("I");
    assert_eq!(rows.len(), 276);
    write_output(
        "ALG05_BASE",
        &json!({
            "schema": "vigilode-alg05-base-v1",
            "driver": "integrate_rodas5p_fast_observed",
            "max_attempts": MAX_ATTEMPTS,
            "seeds": SEEDS,
            "error_metric": "max_i |y_i - r_i| / max(|r_i|, 1e-10) against NATIVE.json references",
            "rows": rows,
        }),
    );
}

/// The arms of `export_runs`.
const ARMS: [&str; 3] = ["I", "PREDcap", "PREDcap2"];

#[test]
fn the_arms_differ_only_in_the_controller() {
    let problem = &problems()[0];
    let base = base_config(problem, 1.0e-5, SEEDS[1]);
    assert_eq!(arm_config("I", base.clone()), base);
    for (arm, kind) in [
        ("PREDcap", ControllerKind::PredictiveCapped),
        ("PREDcap2", ControllerKind::PredictiveCapped2),
    ] {
        let config = arm_config(arm, base.clone());
        assert_eq!(config.controller, kind);
        assert_eq!(
            AdaptiveStepConfig {
                controller: ControllerKind::Integral,
                ..config
            },
            base
        );
    }
}

/// All arms on every cell.
#[test]
#[ignore = "run export of research/alg05_predictive_controller_v2_20261010; release build; set ALG05_RUNS"]
fn export_runs() {
    let mut all = Vec::new();
    for arm in ARMS {
        all.extend(rows(arm));
    }
    assert_eq!(all.len(), ARMS.len() * 276);
    write_output(
        "ALG05_RUNS",
        &json!({
            "schema": "vigilode-alg05-runs-v1",
            "driver": "integrate_rodas5p_fast_observed",
            "max_attempts": MAX_ATTEMPTS,
            "seeds": SEEDS,
            "arms": ARMS,
            "error_metric": "max_i |y_i - r_i| / max(|r_i|, 1e-10) against NATIVE.json references",
            "rows": all,
        }),
    );
}
