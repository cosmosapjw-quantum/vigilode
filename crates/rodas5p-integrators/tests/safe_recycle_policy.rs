//! Research node `research/safe_recycle_policy_20261004` (RVJ DAG node
//! SAFE-RECYCLE): the U-form driver's GCRO-DR stage solves under an explicit
//! recycle policy. The base export (legacy driver call, no policy) was run
//! on the base commit before any source change; its digest is in the
//! preregistration.

#[path = "rnext_common/mod.rs"]
mod common;

use common::{
    adaptive, brusselator, cases, linear_config, splitmix_vector, stage_operator, write_output,
};
use rodas5p_core::{IdentityPreconditioner, LinearMethod, WorkCounters};
use rodas5p_integrators::{
    GcrodrRecyclePolicy, OdeProblem, OutputSchedule, gcrodr_stage_solve,
    integrate_rodas5p_mf_fast_observed, integrate_rodas5p_mf_fast_observed_with_gcrodr_policy,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrSolveOptions, GcrodrState, GcrodrTrace, GcrodrWorkspace,
    solve_gcrodr_with_options,
};
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

const POLICIES: [GcrodrRecyclePolicy; 3] = [
    GcrodrRecyclePolicy::Legacy,
    GcrodrRecyclePolicy::RefreshAfterUpdate,
    GcrodrRecyclePolicy::Cold,
];

/// The recorded run: default entry point and the three policies per row.
#[test]
#[ignore = "recorded run of research/safe_recycle_policy_20261004; release build"]
fn export_policies() {
    let mut rows = Vec::new();
    for run in runs() {
        for rtol in RTOLS {
            let span = run.t_span.1 - run.t_span.0;
            let adapt = adaptive(rtol, run.atol_scale, span);
            let linear = linear_config(LinearMethod::Gcrodr, LINEAR_RTOL);
            let schedule = OutputSchedule::new(vec![run.t_span.0, run.t_span.1]).unwrap();
            let default = integrate_rodas5p_mf_fast_observed(
                &run.problem,
                run.t_span,
                &run.y0,
                &linear,
                &adapt,
                &schedule,
            );
            let mut row = json!({"case": run.id, "rtol": rtol, "default": result_json(&default)});
            for policy in POLICIES {
                let result = integrate_rodas5p_mf_fast_observed_with_gcrodr_policy(
                    &run.problem,
                    run.t_span,
                    &run.y0,
                    &linear,
                    &adapt,
                    &schedule,
                    policy,
                );
                if let Ok(r) = &result {
                    assert_eq!(r.gcrodr_policy, policy.id());
                }
                row[policy.id()] = result_json(&result);
            }
            rows.push(row);
        }
    }
    write_output("SAFE_RECYCLE_OUTPUT", &json!({"rows": rows}));
}

fn frozen_system() -> (rodas5p_core::ShiftedOperator, Vec<Vec<f64>>, GcrodrConfig) {
    let (p, y0) = brusselator(160).unwrap();
    let problem = p.jvp_only_clone().unwrap();
    let op = stage_operator(&problem, 0.0, &y0, 1.0e-2);
    let rhs = (0..8)
        .map(|k| splitmix_vector(0x5afe_0000 + k, y0.len()))
        .collect();
    let linear = linear_config(LinearMethod::Gcrodr, LINEAR_RTOL);
    let config = GcrodrConfig {
        restart: linear.restart,
        max_arnoldi: linear.maxiter.max(linear.restart),
        recycle_dim: linear.recycle_dim,
        rank_tol: linear.recycle_rank_tol,
        rtol: linear.rtol,
        atol: linear.atol,
    };
    (op, rhs, config)
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// G2 contract: the driver's refreshed stage solve equals the REV-01c
/// traced call with `refresh_after_update = true`, solution and counters,
/// over a sequence of solves with a carried state.
#[test]
fn refreshed_stage_solve_equals_traced_rev01c_call() {
    let (op, rhs, config) = frozen_system();
    let pc = IdentityPreconditioner::new(op_dimension(&op));
    let (mut s1, mut s2) = (GcrodrState::default(), GcrodrState::default());
    let (mut w1, mut w2) = (GcrodrWorkspace::default(), GcrodrWorkspace::default());
    let (mut c1, mut c2) = (WorkCounters::default(), WorkCounters::default());
    for b in &rhs {
        let a = gcrodr_stage_solve(
            GcrodrRecyclePolicy::RefreshAfterUpdate,
            &op,
            &pc,
            b,
            None,
            &config,
            &mut s1,
            &mut w1,
            &mut c1,
        );
        let mut trace = GcrodrTrace::default();
        let t = solve_gcrodr_with_options(
            &op,
            &pc,
            b,
            None,
            &config,
            &mut s2,
            None,
            &mut w2,
            GcrodrSolveOptions {
                refresh_after_update: true,
                ..GcrodrSolveOptions::default()
            },
            &mut trace,
            &mut c2,
        );
        match (a, t) {
            (Ok(a), Ok(t)) => assert_eq!(bits(&a.x), bits(&t.x)),
            (Err(a), Err(t)) => assert_eq!(a.to_string(), t.to_string()),
            (a, t) => panic!("outcomes differ: {:?} vs {:?}", a.is_ok(), t.is_ok()),
        }
        assert_eq!(c1, c2);
        assert_eq!(s1, s2);
    }
    assert!(c1.recycle_updates > 0);
    assert!(c1.recycle_update_refreshes > 0);
}

/// G3 contract: a cold stage solve leaves the carried state as it was.
#[test]
fn cold_stage_solve_leaves_carried_state() {
    let (op, rhs, config) = frozen_system();
    let pc = IdentityPreconditioner::new(op_dimension(&op));
    let mut state = GcrodrState::default();
    let mut work = GcrodrWorkspace::default();
    let mut counters = WorkCounters::default();
    // Give the carried state content with two legacy solves.
    for b in &rhs[..2] {
        let _ = gcrodr_stage_solve(
            GcrodrRecyclePolicy::Legacy,
            &op,
            &pc,
            b,
            None,
            &config,
            &mut state,
            &mut work,
            &mut counters,
        );
    }
    assert!(state.rank() > 0);
    for b in &rhs[2..] {
        let before = state.clone();
        let _ = gcrodr_stage_solve(
            GcrodrRecyclePolicy::Cold,
            &op,
            &pc,
            b,
            None,
            &config,
            &mut state,
            &mut work,
            &mut counters,
        );
        assert_eq!(state, before);
    }
    assert_eq!(counters.recycle_update_refreshes, 0);
}

fn op_dimension(op: &rodas5p_core::ShiftedOperator) -> usize {
    use rodas5p_core::LinearOperator;
    op.dimension()
}
