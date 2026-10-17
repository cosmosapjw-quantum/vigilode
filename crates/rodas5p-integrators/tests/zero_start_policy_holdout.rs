//! Holdout tests and data export for research node SP04
//! (`research/sp04_zero_start_recycling_20261011`).

use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, GcrodrRecyclePolicy, OdeProblem, OutputSchedule, Rodas5pMfFastResult,
    Rodas5pMfFastWorkspace, StageTargetPolicy, integrate_rodas5p_mf_fast_observed,
    integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_with_gcrodr_policy,
    integrate_rodas5p_mf_fast_observed_with_residual_accounting,
    semilinear_advection_diffusion_problem,
};
use rodas5p_krylov::ResidualAccounting;
use serde_json::{Value, json};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::path::PathBuf;

struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count_alloc() {
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_alloc();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_alloc();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_alloc();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.with(Cell::get);
    let value = f();
    (value, ALLOCATIONS.with(Cell::get) - before)
}

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
pub const ALL_ARMS: [&str; 11] = [
    "G-Z", "G-P", "G-Z2", "R-P", "C-P", "C-Z", "Lg-P", "L-P", "R-Z", "R-Zw", "L-Z",
];

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

pub fn run_arm(
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
        "R-Z" => {
            let linear = make_linear_config(LinearMethod::Gcrodr, InitialGuess::Zero);
            let mut work = Rodas5pMfFastWorkspace::new(problem, &linear)?;
            work.set_gcrodr_policy(GcrodrRecyclePolicy::RefreshAfterUpdate);
            work.set_explicit_zero(true);
            work.set_current_w(false);
            work.integrate_observed(problem, T_SPAN, y0, &adaptive, &schedule)
        }
        "R-Zw" => {
            let linear = make_linear_config(LinearMethod::Gcrodr, InitialGuess::Zero);
            let mut work = Rodas5pMfFastWorkspace::new(problem, &linear)?;
            work.set_gcrodr_policy(GcrodrRecyclePolicy::RefreshAfterUpdate);
            work.set_explicit_zero(true);
            work.set_current_w(true);
            work.integrate_observed(problem, T_SPAN, y0, &adaptive, &schedule)
        }
        "L-Z" => {
            let linear = make_linear_config(LinearMethod::Lgmres, InitialGuess::Zero);
            let mut work = Rodas5pMfFastWorkspace::new(problem, &linear)?;
            work.set_explicit_zero(true);
            work.set_current_w(false);
            work.integrate_observed(problem, T_SPAN, y0, &adaptive, &schedule)
        }
        other => panic!("unknown arm {other}"),
    }
}

pub fn run_comparator_arm(
    arm: &str,
    problem: &OdeProblem,
    y0: &[f64],
    rtol: f64,
) -> rodas5p_core::CoreResult<Rodas5pMfFastResult> {
    run_arm(arm, problem, y0, rtol)
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

#[test]
fn separation_contract_and_telemetry_parity() {
    let p = &PROBLEMS[0];
    let (prob, y0, _) = make_problem(p);
    let rtol = 1.0e-6;
    let adaptive = make_adaptive_config(rtol);
    let schedule = OutputSchedule::new(vec![T_SPAN.0, T_SPAN.1]).expect("schedule");

    // 1. Under ExplicitZero, start telemetry shows zero starts equal to linear_solves,
    // with no previous-stage and no solver-carried starts.
    let linear = make_linear_config(LinearMethod::Gcrodr, InitialGuess::Zero);
    let mut work_rz = Rodas5pMfFastWorkspace::new(&prob, &linear).expect("workspace");
    work_rz.set_gcrodr_policy(GcrodrRecyclePolicy::RefreshAfterUpdate);
    work_rz.set_explicit_zero(true);
    work_rz.set_start_telemetry_enabled(true);
    let res_rz = work_rz
        .integrate_observed(&prob, T_SPAN, &y0, &adaptive, &schedule)
        .expect("R-Z run");
    let tel = work_rz.start_telemetry();
    assert!(res_rz.observed.counters.linear_solves > 0);
    assert_eq!(tel.zero_starts, res_rz.observed.counters.linear_solves);
    assert_eq!(tel.previous_stage_starts, 0);
    assert_eq!(tel.solver_carried_starts, 0);

    // 2. ExplicitZero with GMRES-into equals G-Z bitwise
    let res_gz = run_arm("G-Z", &prob, &y0, rtol).expect("G-Z run");
    let linear_gmres = make_linear_config(LinearMethod::Gmres, InitialGuess::Zero);
    let mut work_gz_exp = Rodas5pMfFastWorkspace::new(&prob, &linear_gmres).expect("workspace");
    work_gz_exp.set_gmres_into(true);
    work_gz_exp.set_explicit_zero(true);
    let res_gz_exp = work_gz_exp
        .integrate_observed(&prob, T_SPAN, &y0, &adaptive, &schedule)
        .expect("GMRES ExplicitZero run");
    assert_eq!(res_gz.observed.t, res_gz_exp.observed.t);
    assert_eq!(res_gz.observed.y, res_gz_exp.observed.y);
    assert_eq!(res_gz.attempts, res_gz_exp.attempts);
    assert_eq!(res_gz.accepted_steps, res_gz_exp.accepted_steps);
    assert_eq!(res_gz.rejected_steps, res_gz_exp.rejected_steps);
    assert_eq!(
        res_gz.observed.counters.jvp_vectors,
        res_gz_exp.observed.counters.jvp_vectors
    );

    // 3. ExplicitZero with GCRO-DR Cold equals C-Z bitwise
    let res_cz = run_arm("C-Z", &prob, &y0, rtol).expect("C-Z run");
    let linear_gcrodr_cold = make_linear_config(LinearMethod::Gcrodr, InitialGuess::Zero);
    let mut work_cz_exp =
        Rodas5pMfFastWorkspace::new(&prob, &linear_gcrodr_cold).expect("workspace");
    work_cz_exp.set_gcrodr_policy(GcrodrRecyclePolicy::Cold);
    work_cz_exp.set_explicit_zero(true);
    let res_cz_exp = work_cz_exp
        .integrate_observed(&prob, T_SPAN, &y0, &adaptive, &schedule)
        .expect("C-Z ExplicitZero run");
    assert_eq!(res_cz.observed.t, res_cz_exp.observed.t);
    assert_eq!(res_cz.observed.y, res_cz_exp.observed.y);
    assert_eq!(res_cz.attempts, res_cz_exp.attempts);
    assert_eq!(res_cz.accepted_steps, res_cz_exp.accepted_steps);
    assert_eq!(res_cz.rejected_steps, res_cz_exp.rejected_steps);
    assert_eq!(
        res_cz.observed.counters.jvp_vectors,
        res_cz_exp.observed.counters.jvp_vectors
    );

    // 4. Telemetry-on run equals telemetry-off run bitwise
    let mut work_rz_notel = Rodas5pMfFastWorkspace::new(&prob, &linear).expect("workspace");
    work_rz_notel.set_gcrodr_policy(GcrodrRecyclePolicy::RefreshAfterUpdate);
    work_rz_notel.set_explicit_zero(true);
    work_rz_notel.set_start_telemetry_enabled(false);
    let res_rz_notel = work_rz_notel
        .integrate_observed(&prob, T_SPAN, &y0, &adaptive, &schedule)
        .expect("R-Z notel run");
    assert_eq!(res_rz.observed.t, res_rz_notel.observed.t);
    assert_eq!(res_rz.observed.y, res_rz_notel.observed.y);
    assert_eq!(res_rz.attempts, res_rz_notel.attempts);
    assert_eq!(res_rz.accepted_steps, res_rz_notel.accepted_steps);
    assert_eq!(res_rz.rejected_steps, res_rz_notel.rejected_steps);
    assert_eq!(res_rz.observed.counters, res_rz_notel.observed.counters);
}

#[test]
fn current_w_contract() {
    let p = &PROBLEMS[0];
    let (prob, y0, _) = make_problem(p);
    let rtol = 1.0e-6;

    // 1. In every R-Z row, recycle_update_refreshes equals recycle_updates
    let res_rz = run_arm("R-Z", &prob, &y0, rtol).expect("R-Z run");
    let c = &res_rz.observed.counters;
    assert!(c.recycle_updates > 0, "R-Z should have recycle updates");
    assert_eq!(
        c.recycle_update_refreshes, c.recycle_updates,
        "recycle_update_refreshes must equal recycle_updates"
    );

    // 2. Under CurrentW, the carried subspace is empty at every attempt start.
    // When start telemetry is on, cross_system_basis_starts should be 0 because
    // any carried basis from a previous attempt/system is emptied before starting the new attempt.
    let linear = make_linear_config(LinearMethod::Gcrodr, InitialGuess::Zero);
    let adaptive = make_adaptive_config(rtol);
    let schedule = OutputSchedule::new(vec![T_SPAN.0, T_SPAN.1]).expect("schedule");
    let mut work_rzw = Rodas5pMfFastWorkspace::new(&prob, &linear).expect("workspace");
    work_rzw.set_gcrodr_policy(GcrodrRecyclePolicy::RefreshAfterUpdate);
    work_rzw.set_explicit_zero(true);
    work_rzw.set_current_w(true);
    work_rzw.set_start_telemetry_enabled(true);
    let res_rzw = work_rzw
        .integrate_observed(&prob, T_SPAN, &y0, &adaptive, &schedule)
        .expect("R-Zw run");
    assert!(res_rzw.attempts > 1);
    assert_eq!(
        work_rzw.start_telemetry().cross_system_basis_starts,
        0,
        "CurrentW must empty carried basis before cross-system attempt start"
    );

    // 3. A carried GCRO-DR pair from W1 used under W2 has its images recomputed against W2:
    // bitwise the fresh products, charged as recycle_refresh_matvecs.
    use rodas5p_core::{IdentityPreconditioner, ShiftedOperator};
    use rodas5p_krylov::{
        GcrodrConfig, GcrodrSolveOptions, GcrodrState, GcrodrTrace, GcrodrWorkspace,
        solve_gcrodr_with_options,
    };
    let pc = IdentityPreconditioner::new(DIMENSION);
    let jvp = prob.linearize_matrix_free(0.0, &y0).expect("jvp");
    let h1 = 1.0e-5;
    let h2 = 2.0e-5;
    let gamma = 0.25;
    let w1 = ShiftedOperator::new_counted_jvp(None, jvp.clone(), h1, gamma).expect("w1");
    let w2 = ShiftedOperator::new_counted_jvp(None, jvp, h2, gamma).expect("w2");

    let mut state = GcrodrState::default();
    let mut work = GcrodrWorkspace::default();
    let mut counters = WorkCounters::default();
    let mut trace = GcrodrTrace::default();
    let gcrodr_cfg = GcrodrConfig {
        rtol: 1.0e-10,
        atol: 1.0e-14,
        restart: 40,
        max_arnoldi: 200,
        recycle_dim: 8,
        rank_tol: 1.0e-12,
    };
    let opts = GcrodrSolveOptions {
        refresh_after_update: true,
        ..GcrodrSolveOptions::default()
    };

    let rhs = vec![1.0; DIMENSION];
    // Solve on W1 to build a recycle basis
    let _ = solve_gcrodr_with_options(
        &w1,
        &pc,
        &rhs,
        None,
        &gcrodr_cfg,
        &mut state,
        None,
        &mut work,
        opts,
        &mut trace,
        &mut counters,
    )
    .expect("solve on W1");
    let basis_len = state.basis.len();
    assert!(basis_len > 0, "basis should have vectors");

    // Solve on W2: system identity changed, so images must be recomputed against W2
    let refresh_before = counters.recycle_refresh_matvecs;
    let _ = solve_gcrodr_with_options(
        &w2,
        &pc,
        &rhs,
        None,
        &gcrodr_cfg,
        &mut state,
        None,
        &mut work,
        opts,
        &mut trace,
        &mut counters,
    )
    .expect("solve on W2");
    let refresh_added = counters.recycle_refresh_matvecs - refresh_before;
    assert!(
        refresh_added >= basis_len as u64,
        "refresh matvecs must be charged"
    );

    // Verify carried pair satisfies BU=C invariant under W2
    state
        .verify_invariant(&w2, &pc, 1.0e-10)
        .expect("carried pair must satisfy BU=C under W2");
}

#[test]
fn zero_allocation_in_explicit_zero_hot_loop() {
    let (prob, y0) =
        rodas5p_integrators::robertson_problem().expect("robertson_problem construction");
    let linear = make_linear_config(LinearMethod::Gmres, InitialGuess::Zero);
    let mut work = Rodas5pMfFastWorkspace::new(&prob, &linear).expect("workspace new");
    work.set_explicit_zero(true);
    work.set_gmres_into(true);
    work.set_ls_workspace(true);

    let mut counters = WorkCounters::default();
    let atol = 1.0e-8;
    let rtol = 1.0e-6;
    let h = 1.0e-6;
    let t = 0.0;

    // Warm up the first attempt (caches, JVP linearization, workspace capacities)
    let warm_res = work.attempt(&prob, t, &y0, h, true, None, atol, rtol, &mut counters);
    assert!(warm_res.is_ok(), "warm attempt should succeed");

    // Hot loop: second attempt on the same state with fresh=false
    let (hot_res, allocs) = allocations_during(|| {
        work.attempt(&prob, t, &y0, h, false, None, atol, rtol, &mut counters)
    });
    assert!(hot_res.is_ok(), "hot attempt should succeed");
    assert_eq!(
        allocs, 0,
        "explicit zero hot loop attempt must perform 0 heap allocations, got {allocs}"
    );
}

#[test]
#[ignore = "SP04 runs export across 6 cells and 11 arms (8 comparator + 3 new); release build"]
fn export_runs() {
    let (commit, clean) = get_git_status();
    let mut rows = Vec::new();

    for prob_def in &PROBLEMS {
        let (prob, y0, y_exact) = make_problem(prob_def);
        for &rtol in &RTOLS {
            for arm in &ALL_ARMS {
                let row = make_row(prob_def, arm, rtol, &prob, &y0, &y_exact);
                rows.push(row);
            }
        }
    }

    let runs_doc = json!({
        "schema": "vigilode-sp04-runs-v1",
        "node": "sp04_zero_start_recycling_20261011",
        "export": "export_runs",
        "source_commit": commit,
        "tree_clean": clean,
        "faer_pinned": "0.24.4",
        "rayon_num_threads": "1",
        "openblas_num_threads": "1",
        "dimension": DIMENSION,
        "problems": PROBLEMS.iter().map(|p| p.name).collect::<Vec<_>>(),
        "rtols": RTOLS,
        "all_arms": ALL_ARMS,
        "comparator_arms": COMPARATOR_ARMS,
        "new_arms": ["R-Z", "R-Zw", "L-Z"],
        "rows": rows,
    });

    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/sp04_zero_start_recycling_20261011/RUNS.json");
    let out_path = std::env::var("SP04_RUNS")
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

    let serialized = serde_json::to_string_pretty(&runs_doc).expect("serialize runs json");
    std::fs::write(&out_path, format!("{serialized}\n")).expect("write RUNS.json");
    println!("Exported SP04 RUNS to {}", out_path.display());
}
