//! Profiling workload of research node `research/sp01_dupfix_adoption_20261010`
//! (SP01): one registered cell, integrated `--repetitions` times in one
//! process under one GMRES residual accounting, so that callgrind's
//! 2-minus-1-repetition difference is one trajectory
//! (`tools/sp01_dupfix_profile.py`). Both accountings run in this same
//! binary. The cells, configurations and entry points are those of
//! `tests/sp01_dupfix_adoption.rs` (the `alg01_common` problems).
//!
//!     sp01_dupfix_profile --path uform_into --problem hires --rtol 1e-6 \
//!         --accounting reuse_confirmed --repetitions 2
//!
//! Prints one JSON line with the last repetition's work and final state;
//! every repetition must give the same record.

#[path = "../tests/alg01_common/mod.rs"]
#[allow(unused_imports)]
mod alg01_common;

use alg01_common::*;
use rodas5p_core::{LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveControllerState, KrylovState, StageSolveAccounting, StageTargetPolicy,
    integrate_rodas5p_mf_fast_observed_with_residual_accounting,
    integrate_sequential_matrix_free_adaptive_observed_with_residual_accounting,
    rodas_next_step_after_attempt, sequential_matrix_free_step_with_residual_accounting,
};
use rodas5p_krylov::ResidualAccounting;
use serde_json::{Value, json};

fn kform_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
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
    solves: usize,
) -> Value {
    json!({
        "success": success,
        "attempts": attempts,
        "accepted_steps": accepted,
        "rejected_steps": rejected,
        "counters": serde_json::to_value(counters).unwrap(),
        "final_state": hexes(y),
        "logged_solves": solves,
    })
}

fn run(path: &str, p: &Problem, rtol: f64, accounting: ResidualAccounting) -> Value {
    match path {
        "uform_into" | "uform_default" => {
            let r = integrate_rodas5p_mf_fast_observed_with_residual_accounting(
                &p.problem,
                p.t_span,
                &p.y0,
                &gmres_config(BUDGET),
                &p.adaptive(rtol),
                &p.schedule(),
                path == "uform_into",
                StageTargetPolicy::Legacy,
                accounting,
            )
            .expect("integration");
            let o = &r.result.observed;
            record(
                o.success,
                r.result.attempts,
                r.result.accepted_steps,
                r.result.rejected_steps,
                o.counters,
                o.y.last().unwrap(),
                r.solve_log.len(),
            )
        }
        "kform_integrate" => {
            let r = integrate_sequential_matrix_free_adaptive_observed_with_residual_accounting(
                &p.problem,
                p.t_span,
                &p.y0,
                &kform_config(),
                &p.adaptive(rtol),
                &p.schedule(),
                accounting,
            )
            .expect("integration");
            let o = &r.result.observed;
            let d = &r.result.diagnostics;
            record(
                o.success,
                d.attempts,
                d.accepted_macro_steps,
                d.rejected_macro_steps,
                o.counters,
                o.y.last().unwrap(),
                r.solve_log.len(),
            )
        }
        "kform_step" => {
            // `tests/sp01_dupfix_adoption.rs::kform_step`.
            let linear = kform_config();
            let adaptive = p.adaptive(rtol);
            let (mut t, tf) = p.t_span;
            let mut y = p.y0.clone();
            let mut h = adaptive.initial_step;
            let mut controller = AdaptiveControllerState::default();
            let mut counters = WorkCounters::default();
            let mut recycle = KrylovState::for_method(linear.method);
            let (mut attempts, mut accepted, mut rejected) = (0, 0, 0);
            let mut log = Vec::new();
            while t < tf && attempts < adaptive.max_attempts {
                let trial = h.min(tf - t);
                if trial < adaptive.min_step && t + trial < tf {
                    break;
                }
                attempts += 1;
                let step = sequential_matrix_free_step_with_residual_accounting(
                    &p.problem,
                    t,
                    &y,
                    trial,
                    &linear,
                    recycle.as_mut(),
                    adaptive.atol,
                    adaptive.rtol,
                    false,
                    StageSolveAccounting {
                        accounting,
                        solve_log: Some(&mut log),
                    },
                    &mut counters,
                );
                let (error, ok) = match step {
                    Ok(report) if report.accepted => {
                        t = if trial == tf - t { tf } else { t + trial };
                        y = report.y_new;
                        (report.error_norm, true)
                    }
                    Ok(report) => (report.error_norm, false),
                    Err(_) => (f64::INFINITY, false),
                };
                if ok {
                    accepted += 1;
                } else {
                    rejected += 1;
                }
                h = rodas_next_step_after_attempt(
                    &mut controller,
                    &adaptive,
                    h,
                    trial,
                    error,
                    ok,
                    false,
                )
                .unwrap();
            }
            record(
                t >= tf,
                attempts,
                accepted,
                rejected,
                counters,
                &y,
                log.len(),
            )
        }
        other => panic!("unknown path {other}"),
    }
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
    let path = value("--path");
    let problem = value("--problem");
    let rtol: f64 = value("--rtol").parse().expect("rtol");
    let accounting = match value("--accounting").as_str() {
        "recompute_final" => ResidualAccounting::RecomputeFinal,
        "reuse_confirmed" => ResidualAccounting::ReuseConfirmed,
        other => panic!("unknown accounting {other}"),
    };
    let repetitions: usize = value("--repetitions").parse().expect("repetitions");
    assert!(repetitions >= 1);
    let p = spd07_problem(&problem);
    let mut last: Option<Value> = None;
    for _ in 0..repetitions {
        let r = run(&path, &p, rtol, accounting);
        if let Some(previous) = &last {
            assert_eq!(previous, &r, "repetitions differ");
        }
        last = Some(r);
    }
    let mut out = last.unwrap();
    out["path"] = json!(path);
    out["problem"] = json!(problem);
    out["rtol"] = json!(rtol);
    out["accounting"] = json!(accounting.id());
    out["repetitions"] = json!(repetitions);
    println!("{out}");
}
