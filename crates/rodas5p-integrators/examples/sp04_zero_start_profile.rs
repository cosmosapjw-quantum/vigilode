//! Profiling workload of research node `research/sp04_zero_start_recycling_20261011`
//! (SP04): one registered cell, integrated `--repetitions` times in one
//! process under one arm, so that callgrind's
//! 2-minus-1-repetition difference is one trajectory (`tools/sp04_zero_start_profile.py`).
//!
//!     sp04_zero_start_profile --arm R-Z --problem E-normal --rtol 1e-6 --repetitions 2
//!
//! Prints one JSON line with the last repetition's work and final state;
//! every repetition must give the same record.

use rodas5p_core::{InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind};
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

struct ProblemDef {
    diffusion: f64,
    advection: f64,
    reaction: f64,
    nonlinearity: f64,
}

fn problem_def(name: &str) -> ProblemDef {
    match name {
        "E-normal" => ProblemDef {
            diffusion: 0.05,
            advection: 0.0,
            reaction: -1.0,
            nonlinearity: 0.5,
        },
        "E-nonnormal" => ProblemDef {
            diffusion: 0.005,
            advection: 2.0,
            reaction: -1.0,
            nonlinearity: 0.5,
        },
        "E-nonnormal-strong" => ProblemDef {
            diffusion: 0.001,
            advection: 2.0,
            reaction: -1.0,
            nonlinearity: 0.5,
        },
        other => panic!("unknown problem {other}"),
    }
}

fn make_problem(p: &ProblemDef) -> (OdeProblem, Vec<f64>) {
    let (prob, y0) = semilinear_advection_diffusion_problem(
        DIMENSION,
        p.diffusion,
        p.advection,
        p.reaction,
        p.nonlinearity,
        T_SPAN.0,
    )
    .expect("semilinear_advection_diffusion_problem construction");
    let jvp_prob = prob.jvp_only_clone().expect("jvp_only_clone");
    (jvp_prob, y0)
}

fn make_adaptive_config(rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-2 * rtol,
        rtol,
        initial_step: INITIAL_STEP,
        max_attempts: MAX_ATTEMPTS,
        ..AdaptiveStepConfig::default()
    }
}

fn make_linear_config(method: LinearMethod, x0: InitialGuess) -> LinearSolverConfig {
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

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| format!("{:016x}", x.to_bits())).collect()
}

fn run_arm(
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

fn record(r: &Rodas5pMfFastResult) -> Value {
    let o = &r.observed;
    json!({
        "success": o.success,
        "attempts": r.attempts,
        "accepted_steps": r.accepted_steps,
        "rejected_steps": r.rejected_steps,
        "counters": serde_json::to_value(o.counters).unwrap(),
        "final_state": hexes(o.y.last().expect("last state")),
        "output_times": hexes(&o.t),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        let i = args
            .iter()
            .position(|a| a == flag)
            .unwrap_or_else(|| panic!("missing {flag}"));
        args[i + 1].clone()
    };

    let arm = value("--arm");
    let problem_name = value("--problem");
    let rtol: f64 = value("--rtol").parse().expect("rtol");
    let repetitions: usize = value("--repetitions").parse().expect("repetitions");
    assert!(repetitions >= 1, "repetitions must be at least 1");

    let def = problem_def(&problem_name);
    let (problem, y0) = make_problem(&def);

    let mut last: Option<Value> = None;
    for _ in 0..repetitions {
        let res = run_arm(&arm, &problem, &y0, rtol).expect("run_arm failed");
        let rec = record(&res);
        if let Some(previous) = &last {
            assert_eq!(previous, &rec, "repetitions differ");
        }
        last = Some(rec);
    }

    let mut out = last.unwrap();
    out["arm"] = json!(arm);
    out["problem"] = json!(problem_name);
    out["rtol"] = json!(rtol);
    out["repetitions"] = json!(repetitions);
    println!("{out}");
}
