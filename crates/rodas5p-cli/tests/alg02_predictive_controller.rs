//! Research node `research/alg02_predictive_controller_20261008` (ALG02,
//! algorithmic-directions Tier 1 node N2): predictive (Gustafsson) step-size
//! control with a post-rejection cap, opt-in through
//! `AdaptiveStepConfig::controller`.
//!
//! Dense cells: the dense fast driver (`integrate_rodas5p_fast_observed`) on
//! van-der-pol-mu1000, hires, robertson and brusselator-1d-50 from the stiff
//! benchmark's `benchmark_problems()`, with the benchmark's adaptive
//! configuration (atol = rtol * atol_scale, min step 1e-14, max step = span,
//! 1,000,000 attempts, everything else the default) except the initial step,
//! which is the seed. Endpoint error: `max_i |y_i - r_i| / max(|r_i|, 1e-10)`
//! against the NATIVE.json references.
//!
//! `export_base` runs arm `I` on every dense cell and was run on the
//! registration commit's solver source before any source change.
//! `export_runs` runs arms `I`, `I725`, `PRED` and `PREDcap` on every dense
//! cell, and the reported matrix-free cells: the U-form driver through the
//! existing `gmres_into` entry point (the Legacy stage target: GMRES, linear
//! rtol 1e-10, atol 1e-14, zero start) on the JVP Brusselators of
//! `rnext_common` with 50 and 160 cells, half-decade ladder 1e-3 to 1e-7,
//! arms `I` and `PREDcap`, references from the dense fast driver at rtol
//! 1e-13.

#[path = "../src/stiff_benchmark.rs"]
#[allow(dead_code, unused_imports)]
mod stiff_benchmark;

#[path = "../../rodas5p-integrators/tests/rnext_common/mod.rs"]
mod common;

use rodas5p_core::{InitialGuess, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerKind, OdeProblem, OutputSchedule, Rodas5pFastResult,
    integrate_rodas5p_fast_observed, integrate_rodas5p_mf_fast_observed_gmres_into,
};
use serde_json::{Value, json};
use stiff_benchmark::{BenchmarkProblem, benchmark_problems};

/// The dense problems, in export order.
const DENSE_PROBLEMS: [&str; 4] = [
    "van-der-pol-mu1000",
    "hires",
    "robertson",
    "brusselator-1d-50",
];
/// Initial-step seeds.
const SEEDS: [f64; 3] = [1.0e-6, 1.0e-4, 1.0e-2];
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

/// The registered quarter-decade ladder of a dense problem: 1e-3 to 1e-7
/// (17 points) for van der Pol and Brusselator-50, 1e-3 to 1e-10 (29 points)
/// for HIRES and Robertson.
fn dense_ladder(problem: &str) -> Vec<f64> {
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

fn dense_problems() -> Vec<BenchmarkProblem> {
    let mut all = benchmark_problems().unwrap();
    DENSE_PROBLEMS
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
        "I725" => AdaptiveStepConfig {
            safety: 0.725,
            ..base
        },
        "PRED" => AdaptiveStepConfig {
            controller: ControllerKind::Predictive,
            ..base
        },
        "PREDcap" => AdaptiveStepConfig {
            controller: ControllerKind::PredictiveCapped,
            ..base
        },
        other => panic!("unknown arm {other}"),
    }
}

fn counters_json(c: &WorkCounters) -> Value {
    serde_json::to_value(c).unwrap()
}

fn fast_row(
    result: &rodas5p_core::CoreResult<Rodas5pFastResult>,
    reference: &[f64],
) -> serde_json::Map<String, Value> {
    let value = match result {
        Ok(r) => {
            let y = r.observed.y.last().unwrap();
            let complete = r.observed.success;
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
                "rhs_evaluations": r.observed.counters.rhs_evaluations,
                "lu_factorizations": r.observed.counters.direct_factorizations,
                "jacobian_builds": r.observed.counters.jacobian_builds,
                "linear_solve_failures": r.observed.counters.linear_solve_failures,
                "nonfinite_step_failures": r.observed.counters.nonfinite_step_failures,
                "counters": counters_json(&r.observed.counters),
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

fn dense_run(
    problem: &BenchmarkProblem,
    arm: &str,
    rtol: f64,
    seed: f64,
    reference: &[f64],
) -> Value {
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

/// Every dense cell of `arm`, in the registered order (problem, seed, rtol).
fn dense_rows(arm: &str) -> Vec<Value> {
    let mut rows = Vec::new();
    for problem in dense_problems() {
        let reference = native_reference(problem.id);
        for seed in SEEDS {
            for rtol in dense_ladder(problem.id) {
                let row = dense_run(&problem, arm, rtol, seed, &reference);
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
fn ladders_and_cells_are_the_registered_ones() {
    assert_eq!(dense_ladder("van-der-pol-mu1000").len(), 17);
    assert_eq!(dense_ladder("brusselator-1d-50").len(), 17);
    assert_eq!(dense_ladder("hires").len(), 29);
    assert_eq!(dense_ladder("robertson").len(), 29);
    let hires = dense_ladder("hires");
    assert_eq!(hires[0], 1.0e-3);
    assert_eq!(hires[8], 1.0e-5);
    assert_eq!(hires[28], 1.0e-10);
    assert_eq!(dense_ladder("van-der-pol-mu1000")[16], 1.0e-7);
    for ladder in DENSE_PROBLEMS.map(dense_ladder) {
        for pair in ladder.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!((ratio - 10.0_f64.powf(0.25)).abs() < 1.0e-12, "{pair:?}");
        }
    }
    let problems = dense_problems();
    assert_eq!(
        problems.iter().map(|p| p.id).collect::<Vec<_>>(),
        DENSE_PROBLEMS
    );
    let cells: usize = problems.iter().map(|p| dense_ladder(p.id).len()).sum();
    assert_eq!(cells * SEEDS.len(), 276);
    for problem in &problems {
        assert_eq!(native_reference(problem.id).len(), problem.y0.len());
        let config = base_config(problem, 1.0e-6, 1.0e-2);
        config.validate().unwrap();
        assert_eq!(
            config.controller,
            rodas5p_integrators::ControllerKind::Integral
        );
        assert_eq!(config.safety, 0.9);
    }
}

/// Base export (arm `I` on every dense cell), run on the registration
/// commit's solver source before any source change.
#[test]
#[ignore = "base export of research/alg02_predictive_controller_20261008; release build; set ALG02_BASE"]
fn export_base() {
    let rows = dense_rows("I");
    assert_eq!(rows.len(), 276);
    write_output(
        "ALG02_BASE",
        &json!({
            "schema": "vigilode-alg02-base-v1",
            "driver": "integrate_rodas5p_fast_observed",
            "max_attempts": MAX_ATTEMPTS,
            "seeds": SEEDS,
            "error_metric": "max_i |y_i - r_i| / max(|r_i|, 1e-10) against NATIVE.json references",
            "rows": rows,
        }),
    );
}

/// The dense arms of `export_runs`.
const ARMS: [&str; 4] = ["I", "I725", "PRED", "PREDcap"];
/// The arms of the reported matrix-free cells.
const MF_ARMS: [&str; 2] = ["I", "PREDcap"];
/// The reported matrix-free problems (cells of the JVP Brusselator).
const MF_CELLS: [usize; 2] = [50, 160];
/// The Legacy stage target of the U-form driver (SPD07's `gmres_into`
/// `Zero` arm).
const MF_LINEAR_RTOL: f64 = 1.0e-10;
const MF_REFERENCE_RTOL: f64 = 1.0e-13;

/// The half-decade matrix-free ladder, 1e-3 to 1e-7 (9 points).
fn mf_ladder() -> Vec<f64> {
    (12..=28).step_by(2).map(quarter_decade).collect()
}

struct MfProblem {
    id: String,
    /// JVP-only clone for the U-form driver.
    jvp_only: OdeProblem,
    /// With the explicit Jacobian, for the dense reference.
    full: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
}

fn mf_problem(cells: usize) -> MfProblem {
    let (full, y0) = common::brusselator(cells).unwrap();
    MfProblem {
        id: format!("brusselator-1d-{cells}"),
        jvp_only: full.jvp_only_clone().unwrap(),
        full,
        y0,
        t_span: (0.0, 10.0),
    }
}

/// The benchmark's adaptive configuration (atol scale 1) for the
/// matrix-free cells.
fn mf_config(p: &MfProblem, rtol: f64, seed: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol,
        rtol,
        initial_step: seed,
        min_step: 1.0e-14,
        max_step: p.t_span.1 - p.t_span.0,
        max_attempts: MAX_ATTEMPTS,
        ..AdaptiveStepConfig::default()
    }
}

/// The dense fast driver at rtol 1e-13 (initial step 1e-6).
fn mf_reference(p: &MfProblem) -> (Vec<f64>, Value) {
    let schedule = OutputSchedule::new(vec![p.t_span.0, p.t_span.1]).unwrap();
    let r = integrate_rodas5p_fast_observed(
        &p.full,
        p.t_span,
        &p.y0,
        &mf_config(p, MF_REFERENCE_RTOL, 1.0e-6),
        &schedule,
    )
    .unwrap();
    assert!(r.observed.success, "reference run of {} failed", p.id);
    let y = r.observed.y.last().unwrap().clone();
    let record = json!({
        "kind": "dense-fast-rtol-1e-13",
        "rtol": MF_REFERENCE_RTOL,
        "atol": MF_REFERENCE_RTOL,
        "initial_step": 1.0e-6,
        "attempts": r.attempts,
        "rejected": r.rejected_steps,
        "final_state": hexes(&y),
    });
    (y, record)
}

fn mf_run(p: &MfProblem, arm: &str, rtol: f64, seed: f64, reference: &[f64]) -> Value {
    let config = arm_config(arm, mf_config(p, rtol, seed));
    let schedule = OutputSchedule::new(vec![p.t_span.0, p.t_span.1]).unwrap();
    let linear = LinearSolverConfig {
        x0_strategy: InitialGuess::Zero,
        ..common::linear_config(LinearMethod::Gmres, MF_LINEAR_RTOL)
    };
    let result = integrate_rodas5p_mf_fast_observed_gmres_into(
        &p.jvp_only,
        p.t_span,
        &p.y0,
        &linear,
        &config,
        &schedule,
    );
    let mut row = serde_json::Map::new();
    row.insert("problem".into(), json!(p.id));
    row.insert("arm".into(), json!(arm));
    row.insert("rtol".into(), json!(rtol));
    row.insert("atol".into(), json!(config.atol));
    row.insert("seed".into(), json!(seed));
    let value = match &result {
        Ok(r) => {
            let y = r.observed.y.last().unwrap();
            let complete = r.observed.success;
            let c = &r.observed.counters;
            json!({
                "ok": true,
                "success": complete,
                "message": r.observed.message,
                "attempts": r.attempts,
                "accepted": r.accepted_steps,
                "rejected": r.rejected_steps,
                "state_reuses": r.state_reuses,
                "internal_steps": r.observed.internal_steps,
                "output_clipped_steps": r.observed.output_clipped_steps,
                "rhs_evaluations": c.rhs_evaluations,
                "jvp_vectors": c.jvp_vectors,
                "linear_matvecs": c.linear_matvecs,
                "linear_iterations": c.linear_iterations,
                "orthogonalization_inner_products": c.orthogonalization_inner_products,
                "linear_solve_failures": c.linear_solve_failures,
                "nonfinite_step_failures": c.nonfinite_step_failures,
                "counters": counters_json(c),
                "t_last": hexes(&[*r.observed.t.last().unwrap()]),
                "final_state": hexes(y),
                "error": if complete { json!(endpoint_error(y, reference)) } else { Value::Null },
            })
        }
        Err(e) => json!({"ok": false, "success": false, "error_message": e.to_string()}),
    };
    if let Value::Object(map) = value {
        row.extend(map);
    }
    Value::Object(row)
}

#[test]
fn the_arms_differ_only_in_the_registered_field() {
    let problem = &dense_problems()[0];
    let base = base_config(problem, 1.0e-5, 1.0e-4);
    assert_eq!(arm_config("I", base.clone()), base);
    let i725 = arm_config("I725", base.clone());
    assert_eq!(i725.safety, 0.725);
    assert_eq!(
        AdaptiveStepConfig {
            safety: base.safety,
            ..i725
        },
        base
    );
    for (arm, kind) in [
        ("PRED", ControllerKind::Predictive),
        ("PREDcap", ControllerKind::PredictiveCapped),
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
    assert_eq!(mf_ladder().len(), 9);
    assert_eq!(mf_ladder()[0], 1.0e-3);
    assert_eq!(mf_ladder()[8], 1.0e-7);
}

/// All arms on every dense cell, and the reported matrix-free cells.
#[test]
#[ignore = "run export of research/alg02_predictive_controller_20261008; release build; set ALG02_RUNS"]
fn export_runs() {
    let mut dense = Vec::new();
    for arm in ARMS {
        dense.extend(dense_rows(arm));
    }
    assert_eq!(dense.len(), 4 * 276);
    let mut mf_rows = Vec::new();
    let mut references = serde_json::Map::new();
    for cells in MF_CELLS {
        let p = mf_problem(cells);
        let (reference, record) = mf_reference(&p);
        references.insert(p.id.clone(), record);
        for arm in MF_ARMS {
            for seed in SEEDS {
                for rtol in mf_ladder() {
                    let row = mf_run(&p, arm, rtol, seed, &reference);
                    eprintln!(
                        "{} {arm} seed={seed:e} rtol={rtol:e}: attempts {} rejected {} jvp {} error {}",
                        p.id, row["attempts"], row["rejected"], row["jvp_vectors"], row["error"]
                    );
                    mf_rows.push(row);
                }
            }
        }
    }
    write_output(
        "ALG02_RUNS",
        &json!({
            "schema": "vigilode-alg02-runs-v1",
            "driver": "integrate_rodas5p_fast_observed",
            "mf_driver": "integrate_rodas5p_mf_fast_observed_gmres_into",
            "mf_linear": {"method": "gmres", "rtol": MF_LINEAR_RTOL, "atol": 1.0e-14, "x0": "zero"},
            "max_attempts": MAX_ATTEMPTS,
            "seeds": SEEDS,
            "arms": ARMS,
            "error_metric": "max_i |y_i - r_i| / max(|r_i|, 1e-10) against NATIVE.json references (dense) or the dense fast driver at rtol 1e-13 (matrix-free)",
            "dense_rows": dense,
            "mf_references": references,
            "mf_rows": mf_rows,
        }),
    );
}
