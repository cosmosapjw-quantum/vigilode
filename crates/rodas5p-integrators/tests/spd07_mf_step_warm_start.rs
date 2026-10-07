//! Research node `research/spd07_mf_step_warm_start_20261007` (speed
//! research node SPD07): stage-indexed warm start of the matrix-free U-form
//! driver's Krylov stage solves from the previous accepted step.
//! `export_base` was run on the registration commit's solver source before
//! any source change.

#[path = "rnext_common/mod.rs"]
mod common;

use common::{adaptive, brusselator, cases, linear_config, write_output};
use rodas5p_core::{InitialGuess, LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    GcrodrRecyclePolicy, OdeProblem, OutputSchedule, Rodas5pMfFastResult,
    integrate_rodas5p_fast_observed, integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_with_gcrodr_policy,
};
use serde_json::{Value, json};

struct Run {
    id: &'static str,
    /// JVP-only clone.
    problem: OdeProblem,
    /// With an explicit Jacobian, for the dense reference run.
    full: Option<OdeProblem>,
    exact: Option<Box<dyn Fn(f64) -> Vec<f64>>>,
    y0: Vec<f64>,
    t_span: (f64, f64),
    atol_scale: f64,
}

/// The SAFE-RECYCLE problem set.
fn runs() -> Vec<Run> {
    let mut out: Vec<Run> = cases()
        .into_iter()
        .map(|c| Run {
            id: c.id,
            problem: c.problem,
            full: c.full,
            exact: c.exact,
            y0: c.y0,
            t_span: c.t_span,
            atol_scale: c.atol_scale,
        })
        .collect();
    let (p, y0) = brusselator(160).unwrap();
    out.push(Run {
        id: "brusselator-1d-160",
        problem: p.jvp_only_clone().unwrap(),
        full: Some(p),
        exact: None,
        y0,
        t_span: (0.0, 10.0),
        atol_scale: 1.0,
    });
    out
}

const RTOLS: [f64; 2] = [1.0e-6, 1.0e-8];
const LINEAR_RTOL: f64 = 1.0e-10;
const REFERENCE_RTOL: f64 = 1.0e-12;

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| format!("{:016x}", x.to_bits())).collect()
}

fn result_json(result: &rodas5p_core::CoreResult<Rodas5pMfFastResult>) -> Value {
    match result {
        Ok(r) => json!({
            "ok": true,
            "success": r.observed.success,
            "message": r.observed.message,
            "t": hexes(&r.observed.t),
            "y_last": hexes(r.observed.y.last().unwrap()),
            "attempts": r.attempts,
            "accepted": r.accepted_steps,
            "rejected": r.rejected_steps,
            "state_reuses": r.state_reuses,
            "internal_steps": r.observed.internal_steps,
            "output_clipped_steps": r.observed.output_clipped_steps,
            "counters": serde_json::to_value(r.observed.counters).unwrap(),
        }),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

fn config(method: LinearMethod, x0: InitialGuess) -> LinearSolverConfig {
    LinearSolverConfig {
        x0_strategy: x0,
        ..linear_config(method, LINEAR_RTOL)
    }
}

fn gmres_into(run: &Run, rtol: f64, x0: InitialGuess) -> Value {
    let span = run.t_span.1 - run.t_span.0;
    let schedule = OutputSchedule::new(vec![run.t_span.0, run.t_span.1]).unwrap();
    result_json(&integrate_rodas5p_mf_fast_observed_gmres_into(
        &run.problem,
        run.t_span,
        &run.y0,
        &config(LinearMethod::Gmres, x0),
        &adaptive(rtol, run.atol_scale, span),
        &schedule,
    ))
}

fn gcrodr_cold(run: &Run, rtol: f64, x0: InitialGuess) -> Value {
    let span = run.t_span.1 - run.t_span.0;
    let schedule = OutputSchedule::new(vec![run.t_span.0, run.t_span.1]).unwrap();
    result_json(&integrate_rodas5p_mf_fast_observed_with_gcrodr_policy(
        &run.problem,
        run.t_span,
        &run.y0,
        &config(LinearMethod::Gcrodr, x0),
        &adaptive(rtol, run.atol_scale, span),
        &schedule,
        GcrodrRecyclePolicy::Cold,
    ))
}

/// The final-state reference: the exact solution where one exists, else the
/// dense fast driver at `REFERENCE_RTOL`.
fn reference(run: &Run) -> Value {
    let tf = run.t_span.1;
    if let Some(exact) = &run.exact {
        return json!({"kind": "exact", "y": hexes(&exact(tf))});
    }
    let full = run.full.as_ref().expect("reference problem");
    let span = tf - run.t_span.0;
    let schedule = OutputSchedule::new(vec![run.t_span.0, tf]).unwrap();
    let r = integrate_rodas5p_fast_observed(
        full,
        run.t_span,
        &run.y0,
        &rodas5p_integrators::AdaptiveStepConfig {
            max_attempts: 1_000_000,
            ..adaptive(REFERENCE_RTOL, run.atol_scale, span)
        },
        &schedule,
    )
    .expect("reference run");
    assert!(r.observed.success, "reference run of {} failed", run.id);
    json!({
        "kind": "dense-fast-rtol-1e-12",
        "attempts": r.attempts,
        "y": hexes(r.observed.y.last().unwrap()),
    })
}

/// Base export, run on the registration commit's solver source: GMRES
/// `solve_into` with `Zero` and `Previous`, GCRO-DR cold with `Previous`.
#[test]
#[ignore = "base export of research/spd07_mf_step_warm_start_20261007; release build"]
fn export_base() {
    let mut rows = Vec::new();
    for run in runs() {
        let reference = reference(&run);
        for rtol in RTOLS {
            rows.push(json!({
                "case": run.id,
                "rtol": rtol,
                "dimension": run.y0.len(),
                "reference": reference,
                "gmres_into_zero": gmres_into(&run, rtol, InitialGuess::Zero),
                "gmres_into_previous": gmres_into(&run, rtol, InitialGuess::Previous),
                "gcrodr_cold_previous": gcrodr_cold(&run, rtol, InitialGuess::Previous),
            }));
        }
    }
    write_output("SPD07_BASE", &json!({"rows": rows}));
}
