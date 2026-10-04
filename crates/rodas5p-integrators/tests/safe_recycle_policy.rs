//! Research node `research/safe_recycle_policy_20261004` (RVJ DAG node
//! SAFE-RECYCLE): the U-form driver's GCRO-DR stage solves under an explicit
//! recycle policy. The base export (legacy driver call, no policy) was run
//! on the base commit before any source change; its digest is in the
//! preregistration.

#[path = "rnext_common/mod.rs"]
mod common;

use common::{adaptive, brusselator, cases, linear_config, write_output};
use rodas5p_core::LinearMethod;
use rodas5p_integrators::{OdeProblem, OutputSchedule, integrate_rodas5p_mf_fast_observed};
use serde_json::{Value, json};

pub struct Run {
    pub id: &'static str,
    pub problem: OdeProblem,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    pub atol_scale: f64,
}

pub fn runs() -> Vec<Run> {
    let mut out: Vec<Run> = cases()
        .into_iter()
        .map(|c| Run {
            id: c.id,
            problem: c.problem,
            y0: c.y0,
            t_span: c.t_span,
            atol_scale: c.atol_scale,
        })
        .collect();
    let (p, y0) = brusselator(160).unwrap();
    out.push(Run {
        id: "brusselator-1d-160",
        problem: p.jvp_only_clone().unwrap(),
        y0,
        t_span: (0.0, 10.0),
        atol_scale: 1.0,
    });
    out
}

pub const RTOLS: [f64; 2] = [1.0e-6, 1.0e-8];
pub const LINEAR_RTOL: f64 = 1.0e-10;

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| format!("{:016x}", x.to_bits())).collect()
}

pub fn result_json(
    result: &rodas5p_core::CoreResult<rodas5p_integrators::Rodas5pMfFastResult>,
) -> Value {
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
            "counters": serde_json::to_value(r.observed.counters).unwrap(),
        }),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

#[test]
#[ignore = "base export of research/safe_recycle_policy_20261004; release build"]
fn export_legacy_base() {
    let mut rows = Vec::new();
    for run in runs() {
        for rtol in RTOLS {
            let span = run.t_span.1 - run.t_span.0;
            let adapt = adaptive(rtol, run.atol_scale, span);
            let linear = linear_config(LinearMethod::Gcrodr, LINEAR_RTOL);
            let schedule = OutputSchedule::new(vec![run.t_span.0, run.t_span.1]).unwrap();
            let result = integrate_rodas5p_mf_fast_observed(
                &run.problem,
                run.t_span,
                &run.y0,
                &linear,
                &adapt,
                &schedule,
            );
            rows.push(json!({"case": run.id, "rtol": rtol, "legacy": result_json(&result)}));
        }
    }
    write_output("SAFE_RECYCLE_BASE", &json!({"rows": rows}));
}
