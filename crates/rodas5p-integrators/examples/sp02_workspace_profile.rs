//! Profiling workload of research node `research/sp02_krylov_workspace_caller_20261011`
//! (SP02): one registered cell, integrated `--repetitions` times in one
//! process under one arm (control or candidate), so that callgrind's
//! 2-minus-1-repetition difference is one trajectory (`tools/sp02_workspace_profile.py`).
//!
//!     sp02_workspace_profile --method gmres --arm control --problem hires --rtol 1e-6 \
//!         --repetitions 2
//!
//! Prints one JSON line with the last repetition's work and final state;
//! every repetition must give the same record.

#[path = "../tests/alg01_common/mod.rs"]
#[allow(unused_imports)]
mod alg01_common;

use alg01_common::*;
use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use rodas5p_integrators::{KrylovWorkspaceOptions, Rodas5pMfFastWorkspace};
use serde_json::{Value, json};

fn sp02_linear_config(method: LinearMethod) -> LinearSolverConfig {
    LinearSolverConfig {
        method,
        rtol: 1.0e-11,
        atol: 1.0e-14,
        restart: 40,
        inner_m: 30,
        outer_k: 8,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
        ..LinearSolverConfig::default()
    }
}

fn record(
    success: bool,
    attempts: usize,
    accepted: usize,
    rejected: usize,
    counters: WorkCounters,
    y: &[f64],
) -> Value {
    json!({
        "success": success,
        "attempts": attempts,
        "accepted_steps": accepted,
        "rejected_steps": rejected,
        "counters": serde_json::to_value(counters).unwrap(),
        "final_state": hexes(y),
    })
}

fn run(method: LinearMethod, arm: &str, p: &Problem, rtol: f64) -> Value {
    let linear = sp02_linear_config(method);
    let adaptive = p.adaptive(rtol);
    let schedule = p.schedule();
    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &linear).expect("workspace new");
    match (method, arm) {
        (LinearMethod::Gmres, "control") => {
            work.set_gmres_into(true);
        }
        (LinearMethod::Gmres, "candidate") => {
            work.set_krylov_workspace(KrylovWorkspaceOptions::ls_workspace())
                .expect("set_krylov_workspace");
        }
        (LinearMethod::Lgmres, "control") => {
            // legacy driver path
        }
        (LinearMethod::Lgmres, "candidate") => {
            work.set_krylov_workspace(KrylovWorkspaceOptions::all())
                .expect("set_krylov_workspace");
        }
        _ => panic!("unsupported method and arm combination: {method:?} {arm}"),
    }
    let res = work
        .integrate_observed(&p.problem, p.t_span, &p.y0, &adaptive, &schedule)
        .expect("integration");
    let o = &res.observed;
    record(
        o.success,
        res.attempts,
        res.accepted_steps,
        res.rejected_steps,
        o.counters,
        o.y.last().unwrap(),
    )
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
    let method_str = value("--method");
    let method = match method_str.as_str() {
        "gmres" => LinearMethod::Gmres,
        "lgmres" => LinearMethod::Lgmres,
        other => panic!("unknown method {other}"),
    };
    let arm = value("--arm");
    let problem = value("--problem");
    let rtol: f64 = value("--rtol").parse().expect("rtol");
    let repetitions: usize = value("--repetitions").parse().expect("repetitions");
    assert!(repetitions >= 1);
    let p = spd07_problem(&problem);
    let mut last: Option<Value> = None;
    for _ in 0..repetitions {
        let r = run(method, &arm, &p, rtol);
        if let Some(previous) = &last {
            assert_eq!(previous, &r, "repetitions differ");
        }
        last = Some(r);
    }
    let mut out = last.unwrap();
    out["method"] = json!(method_str);
    out["arm"] = json!(arm);
    out["problem"] = json!(problem);
    out["rtol"] = json!(rtol);
    out["repetitions"] = json!(repetitions);
    println!("{out}");
}
