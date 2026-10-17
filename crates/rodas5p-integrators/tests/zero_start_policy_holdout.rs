//! Holdout tests and data export for research node SP04
//! (`research/sp04_zero_start_recycling_20261011`).

use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, GcrodrRecyclePolicy, OdeProblem, OutputSchedule, Rodas5pMfFastResult,
    StageTargetPolicy, integrate_rodas5p_mf_fast_observed,
    integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_with_gcrodr_policy,
    integrate_rodas5p_mf_fast_observed_with_residual_accounting,
    semilinear_advection_diffusion_problem,
};
use rodas5p_krylov::ResidualAccounting;
use serde_json::{Value, json};
use std::path::PathBuf;

const DIMENSION: usize = 256;
const T_SPAN: (f64, f64) = (0.0, 1.0);
const INITIAL_STEP: f64 = 1.0e-6;
const MAX_ATTEMPTS: usize = 5000;

const LINEAR_RTOL: f64 = 1.0e-10;
const LINEAR_ATOL: f64 = 1.0e-14;
const RESTART: usize = 40;
const MAXITER: usize = 200;
const RECYCLE_DIM: usize = 8;
const INNER_M: usize = 30;
const OUTER_K: usize = 8;

pub struct ProblemDef {
    pub name: &'static str,
    pub diffusion: f64,
    pub advection: f64,
    pub reaction: f64,
    pub nonlinearity: f64,
}

pub const PROBLEMS: [ProblemDef; 3] = [
    ProblemDef {
        name: "E-normal",
        diffusion: 0.05,
        advection: 0.0,
        reaction: -1.0,
        nonlinearity: 0.5,
    },
    ProblemDef {
        name: "E-nonnormal",
        diffusion: 0.005,
        advection: 2.0,
        reaction: -1.0,
        nonlinearity: 0.5,
    },
    ProblemDef {
        name: "E-nonnormal-strong",
        diffusion: 0.001,
        advection: 2.0,
        reaction: -1.0,
        nonlinearity: 0.5,
    },
];

pub const RTOLS: [f64; 2] = [1.0e-6, 1.0e-8];

pub const COMPARATOR_ARMS: [&str; 8] = ["G-Z", "G-P", "G-Z2", "R-P", "C-P", "C-Z", "Lg-P", "L-P"];

pub fn make_problem(p: &ProblemDef) -> (OdeProblem, Vec<f64>, Vec<f64>) {
    let (prob, y0) = semilinear_advection_diffusion_problem(
        DIMENSION,
        p.diffusion,
        p.advection,
        p.reaction,
        p.nonlinearity,
        T_SPAN.0,
    )
    .expect("semilinear_advection_diffusion_problem construction");
    let y_exact = prob.exact(T_SPAN.1).expect("exact solution available");
    let jvp_prob = prob.jvp_only_clone().expect("jvp_only_clone");
    (jvp_prob, y0, y_exact)
}

pub fn make_adaptive_config(rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-2 * rtol,
        rtol,
        initial_step: INITIAL_STEP,
        max_attempts: MAX_ATTEMPTS,
        ..AdaptiveStepConfig::default()
    }
}

pub fn make_linear_config(method: LinearMethod, x0: InitialGuess) -> LinearSolverConfig {
    LinearSolverConfig {
        method,
        rtol: LINEAR_RTOL,
        atol: LINEAR_ATOL,
        restart: RESTART,
        maxiter: MAXITER,
        recycle_dim: RECYCLE_DIM,
        inner_m: INNER_M,
        outer_k: OUTER_K,
        preconditioner: PreconditionerKind::None,
        x0_strategy: x0,
        ..LinearSolverConfig::default()
    }
}

pub fn compute_cost(n: usize, counters: &WorkCounters) -> f64 {
    let f_jvp = (2 * n * n + 7 * n) as f64;
    let f_rhs = (2 * n * n + 8 * n) as f64;
    let two_n = (2 * n) as f64;
    f_jvp * (counters.jvp_vectors as f64)
        + f_rhs * (counters.rhs_evaluations as f64)
        + two_n
            * ((counters.orthogonalization_inner_products
                + counters.orthogonalization_vector_updates) as f64)
}

pub fn compute_endpoint_error(y_last: &[f64], y_exact: &[f64]) -> f64 {
    let max_exact = y_exact.iter().fold(0.0f64, |acc, &x| acc.max(x.abs()));
    let max_diff = y_last
        .iter()
        .zip(y_exact)
        .fold(0.0f64, |acc, (&y, &ye)| acc.max((y - ye).abs()));
    max_diff / max_exact
}

pub fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| format!("{:016x}", x.to_bits())).collect()
}

pub fn run_comparator_arm(
    arm: &str,
    problem: &OdeProblem,
    y0: &[f64],
    rtol: f64,
) -> rodas5p_core::CoreResult<Rodas5pMfFastResult> {
    let adaptive = make_adaptive_config(rtol);
    let schedule = OutputSchedule::new(vec![T_SPAN.0, T_SPAN.1]).expect("schedule");
    match arm {
        "G-Z" => {
            let linear = make_linear_config(LinearMethod::Gmres, InitialGuess::Zero);
            integrate_rodas5p_mf_fast_observed_gmres_into(
                problem, T_SPAN, y0, &linear, &adaptive, &schedule,
            )
        }
        "G-P" => {
            let linear = make_linear_config(LinearMethod::Gmres, InitialGuess::Previous);
            integrate_rodas5p_mf_fast_observed_gmres_into(
                problem, T_SPAN, y0, &linear, &adaptive, &schedule,
            )
        }
        "G-Z2" => {
            let linear = make_linear_config(LinearMethod::Gmres, InitialGuess::Zero);
            let res = integrate_rodas5p_mf_fast_observed_with_residual_accounting(
                problem,
                T_SPAN,
                y0,
                &linear,
                &adaptive,
                &schedule,
                true,
                StageTargetPolicy::Legacy,
                ResidualAccounting::ReuseConfirmed,
            )?;
            Ok(res.result)
        }
        "R-P" => {
            let linear = make_linear_config(LinearMethod::Gcrodr, InitialGuess::Previous);
            integrate_rodas5p_mf_fast_observed_with_gcrodr_policy(
                problem,
                T_SPAN,
                y0,
                &linear,
                &adaptive,
                &schedule,
                GcrodrRecyclePolicy::RefreshAfterUpdate,
            )
        }
        "C-P" => {
            let linear = make_linear_config(LinearMethod::Gcrodr, InitialGuess::Previous);
            integrate_rodas5p_mf_fast_observed_with_gcrodr_policy(
                problem,
                T_SPAN,
                y0,
                &linear,
                &adaptive,
                &schedule,
                GcrodrRecyclePolicy::Cold,
            )
        }
        "C-Z" => {
            let linear = make_linear_config(LinearMethod::Gcrodr, InitialGuess::Zero);
            integrate_rodas5p_mf_fast_observed_with_gcrodr_policy(
                problem,
                T_SPAN,
                y0,
                &linear,
                &adaptive,
                &schedule,
                GcrodrRecyclePolicy::Cold,
            )
        }
        "Lg-P" => {
            let linear = make_linear_config(LinearMethod::Gcrodr, InitialGuess::Previous);
            integrate_rodas5p_mf_fast_observed_with_gcrodr_policy(
                problem,
                T_SPAN,
                y0,
                &linear,
                &adaptive,
                &schedule,
                GcrodrRecyclePolicy::Legacy,
            )
        }
        "L-P" => {
            let linear = make_linear_config(LinearMethod::Lgmres, InitialGuess::Previous);
            integrate_rodas5p_mf_fast_observed(problem, T_SPAN, y0, &linear, &adaptive, &schedule)
        }
        other => panic!("unknown comparator arm {other}"),
    }
}

pub fn make_row(
    prob_def: &ProblemDef,
    arm: &str,
    rtol: f64,
    problem: &OdeProblem,
    y0: &[f64],
    y_exact: &[f64],
) -> Value {
    let result = run_comparator_arm(arm, problem, y0, rtol);
    match result {
        Ok(r) => {
            let y_last = r.observed.y.last().expect("last state");
            let error = compute_endpoint_error(y_last, y_exact);
            let cost = compute_cost(DIMENSION, &r.observed.counters);

            // Verify JVP accounting identity: jvp_vectors = linear_matvecs + diagnostic_matvecs + recycle_refresh_matvecs
            assert_eq!(
                r.observed.counters.jvp_vectors,
                r.observed.counters.linear_matvecs
                    + r.observed.counters.diagnostic_matvecs
                    + r.observed.counters.recycle_refresh_matvecs,
                "JVP identity failed for {}:{}:{}",
                prob_def.name,
                rtol,
                arm
            );

            json!({
                "problem": prob_def.name,
                "rtol": rtol,
                "atol": 1.0e-2 * rtol,
                "arm": arm,
                "dimension": DIMENSION,
                "ok": true,
                "success": r.observed.success,
                "message": r.observed.message,
                "attempts": r.attempts,
                "accepted": r.accepted_steps,
                "rejected": r.rejected_steps,
                "state_reuses": r.state_reuses,
                "internal_steps": r.observed.internal_steps,
                "output_clipped_steps": r.observed.output_clipped_steps,
                "cost": cost,
                "error": error,
                "t": hexes(&r.observed.t),
                "y_last": hexes(y_last),
                "y_exact": hexes(y_exact),
                "counters": serde_json::to_value(r.observed.counters).unwrap(),
            })
        }
        Err(e) => {
            json!({
                "problem": prob_def.name,
                "rtol": rtol,
                "atol": 1.0e-2 * rtol,
                "arm": arm,
                "dimension": DIMENSION,
                "ok": false,
                "success": false,
                "message": e.to_string(),
                "attempts": 0,
                "accepted": 0,
                "rejected": 0,
                "state_reuses": 0,
                "internal_steps": 0,
                "output_clipped_steps": 0,
                "cost": 0.0,
                "error": 0.0,
                "t": Vec::<String>::new(),
                "y_last": Vec::<String>::new(),
                "y_exact": hexes(y_exact),
                "counters": serde_json::to_value(WorkCounters::default()).unwrap(),
            })
        }
    }
}

pub fn get_git_status() -> (String, bool) {
    let commit_output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse HEAD");
    let commit = String::from_utf8(commit_output.stdout)
        .expect("utf8")
        .trim()
        .to_string();

    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .expect("git status");
    let clean = status_output.stdout.is_empty();

    (commit, clean)
}

#[test]
fn smoke_e_normal_g_z() {
    let p = &PROBLEMS[0];
    let (prob, y0, y_exact) = make_problem(p);
    let row = make_row(p, "G-Z", 1.0e-6, &prob, &y0, &y_exact);
    assert_eq!(row["ok"], true);
    assert_eq!(row["success"], true);
    let err = row["error"].as_f64().unwrap();
    assert!(err < 1.0e-3, "error {err} should be small");
}

#[test]
fn default_entry_point_equals_g_p() {
    let p = &PROBLEMS[0];
    let (prob, y0, _y_exact) = make_problem(p);
    let rtol = 1.0e-6;
    let adaptive = make_adaptive_config(rtol);
    let schedule = OutputSchedule::new(vec![T_SPAN.0, T_SPAN.1]).expect("schedule");
    let linear = make_linear_config(LinearMethod::Gmres, InitialGuess::Previous);

    let default_res =
        integrate_rodas5p_mf_fast_observed(&prob, T_SPAN, &y0, &linear, &adaptive, &schedule)
            .expect("default run");

    let gp_res = integrate_rodas5p_mf_fast_observed_gmres_into(
        &prob, T_SPAN, &y0, &linear, &adaptive, &schedule,
    )
    .expect("gp run");

    assert_eq!(default_res.attempts, gp_res.attempts);
    assert_eq!(default_res.accepted_steps, gp_res.accepted_steps);
    assert_eq!(default_res.rejected_steps, gp_res.rejected_steps);
    assert_eq!(
        default_res.observed.counters.jvp_vectors,
        gp_res.observed.counters.jvp_vectors
    );
    assert_eq!(default_res.observed.t, gp_res.observed.t);
    assert_eq!(default_res.observed.y, gp_res.observed.y);
}

#[test]
fn smoke_all_arms_e_normal() {
    let p = &PROBLEMS[0];
    let (prob, y0, y_exact) = make_problem(p);
    for arm in &COMPARATOR_ARMS {
        let row = make_row(p, arm, 1.0e-6, &prob, &y0, &y_exact);
        assert_eq!(row["ok"], true, "arm {arm} should succeed");
        assert_eq!(row["success"], true, "arm {arm} should have success true");
        let cost = row["cost"].as_f64().unwrap();
        assert!(cost > 0.0, "arm {arm} cost should be positive");
    }
}

#[test]
fn smoke_all_problems() {
    for p in &PROBLEMS {
        let (prob, y0, y_exact) = make_problem(p);
        let row = make_row(p, "G-Z", 1.0e-6, &prob, &y0, &y_exact);
        assert_eq!(row["ok"], true, "problem {} G-Z should succeed", p.name);
        assert_eq!(row["success"], true, "problem {} G-Z success", p.name);
    }
}

#[test]
#[ignore = "SP04 base export across 6 cells and 8 comparator arms; release build"]
fn export_base() {
    let (commit, clean) = get_git_status();
    let mut rows = Vec::new();

    for prob_def in &PROBLEMS {
        let (prob, y0, y_exact) = make_problem(prob_def);
        for &rtol in &RTOLS {
            for arm in &COMPARATOR_ARMS {
                let row = make_row(prob_def, arm, rtol, &prob, &y0, &y_exact);
                rows.push(row);
            }
        }
    }

    let base_doc = json!({
        "schema": "vigilode-sp04-base-v1",
        "node": "sp04_zero_start_recycling_20261011",
        "export": "export_base",
        "source_commit": commit,
        "tree_clean": clean,
        "faer_pinned": "0.24.4",
        "rayon_num_threads": "1",
        "openblas_num_threads": "1",
        "dimension": DIMENSION,
        "problems": PROBLEMS.iter().map(|p| p.name).collect::<Vec<_>>(),
        "rtols": RTOLS,
        "comparator_arms": COMPARATOR_ARMS,
        "rows": rows,
    });

    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/sp04_zero_start_recycling_20261011/BASE.json");
    let out_path = std::env::var("SP04_BASE")
        .map(|p| {
            let path = PathBuf::from(p);
            if path.is_absolute() {
                path
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../")
                    .join(path)
            }
        })
        .unwrap_or(default_path);

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("create parent directory");
    }

    let serialized = serde_json::to_string_pretty(&base_doc).expect("serialize base json");
    std::fs::write(&out_path, format!("{serialized}\n")).expect("write BASE.json");
    println!("Exported SP04 BASE to {}", out_path.display());
}
