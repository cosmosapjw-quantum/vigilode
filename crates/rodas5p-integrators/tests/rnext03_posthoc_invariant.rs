//! Post-hoc diagnostic of research node
//! `research/rnext03_gcrodr_attribution_20261003` (written after its recorded
//! run; not part of its gate): the recycle invariant along control C.
#![allow(dead_code, unused_imports)]
#[path = "rnext_common/mod.rs"]
mod common;

use common::{
    adaptive, brusselator, convection_diffusion, linear_config, splitmix_vector, stage_operator,
    stage_right_hand_sides, write_output,
};
use rodas5p_core::{
    DenseOperator, IdentityPreconditioner, LinearMethod, LinearOperator, WorkCounters,
    rodas5p_coefficients, safe_l2,
};
use rodas5p_integrators::{
    OdeProblem, OutputSchedule, Rodas5pMfFastWorkspace, integrate_rodas5p_mf_fast_observed_traced,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GcrodrTrace, GcrodrWorkspace, GmresConfig, GmresWorkspace,
    solve_gcrodr_traced, solve_gmres_with_workspace,
};
use serde_json::{Value, json};

/// An operator, its right-hand sides and a label.
type Group = (Box<dyn LinearOperator>, Vec<Vec<f64>>, String);
/// `(t, y, h, stages)` of one recorded attempt.
type AttemptRecord = (f64, Vec<f64>, f64, Vec<Vec<f64>>);

/// One frozen system sequence: an operator per group of right-hand sides,
/// solved in order.
struct Sequence {
    set: &'static str,
    label: String,
    /// (operator, right-hand sides, group label)
    groups: Vec<Group>,
    rtol: f64,
    budget: usize,
}

fn trajectory_and_one_step(problem: &OdeProblem, y0: &[f64]) -> (Vec<Sequence>, Value) {
    // Trajectory: the first 40 attempts of the GMRES U-form run.
    let linear = linear_config(LinearMethod::Gmres, 1.0e-11);
    let adapt = adaptive(1.0e-6, 1.0, 10.0);
    let schedule = OutputSchedule::new(vec![0.0, 10.0]).unwrap();
    let mut records: Vec<AttemptRecord> = Vec::new();
    let mut failed_attempts = 0usize;
    let run = integrate_rodas5p_mf_fast_observed_traced(
        problem,
        (0.0, 10.0),
        y0,
        &linear,
        &adapt,
        &schedule,
        false,
        &mut |t, y, h, outcome, work: &Rodas5pMfFastWorkspace| {
            if records.len() + failed_attempts >= 40 {
                return;
            }
            match outcome {
                Some(_) => records.push((
                    t,
                    y.to_vec(),
                    h,
                    (0..8).map(|i| work.stage(i).to_vec()).collect(),
                )),
                None => failed_attempts += 1,
            }
        },
    )
    .unwrap();
    let mut trajectory = Sequence {
        set: "trajectory",
        label: "brusselator-50-gmres-trajectory".into(),
        groups: Vec::new(),
        rtol: 1.0e-11,
        budget: 200,
    };
    for (k, (t, y, h, stages)) in records.iter().enumerate() {
        let op = stage_operator(problem, *t, y, *h);
        let rhs = stages
            .iter()
            .map(|u| {
                let mut b = vec![0.0; u.len()];
                op.apply(u, &mut b).unwrap();
                b
            })
            .collect();
        trajectory
            .groups
            .push((Box::new(op), rhs, format!("attempt {k} t={t:e} h={h:e}")));
    }
    let mut sequences = vec![trajectory];
    let mut excluded = Vec::new();
    for h in [1.0e-4, 1.0e-2] {
        let mut work = Rodas5pMfFastWorkspace::new(problem, &linear).unwrap();
        if let Err(error) = work.attempt(
            problem,
            0.0,
            y0,
            h,
            true,
            None,
            1.0e-6,
            1.0e-6,
            &mut WorkCounters::default(),
        ) {
            excluded.push(json!({"h": h, "error": error.to_string()}));
            continue;
        }
        let op = stage_operator(problem, 0.0, y0, h);
        let rhs = stage_right_hand_sides(&op, &work);
        sequences.push(Sequence {
            set: "one_step",
            label: format!("brusselator-50-y0-h={h:e}"),
            groups: vec![(Box::new(op), rhs, format!("h={h:e}"))],
            rtol: 1.0e-12,
            budget: 4000,
        });
    }
    let info = json!({
        "trajectory_attempts_recorded": records.len(),
        "trajectory_failed_attempts_seen": failed_attempts,
        "generating_run": {"success": run.observed.success, "attempts": run.attempts},
        "one_step_excluded": excluded,
    });
    (sequences, info)
}

fn held_out() -> Vec<Sequence> {
    let mut out = Vec::new();
    for peclet in [1.0, 10.0, 100.0] {
        for tau in [1.0e-3, 1.0e-2] {
            let groups = (0..8)
                .map(|k| {
                    let a = convection_diffusion(200, peclet, tau * (1.0 + 0.01 * k as f64));
                    let op: Box<dyn LinearOperator> = Box::new(DenseOperator::new(a).unwrap());
                    let rhs = (0..8).map(|seed| splitmix_vector(seed, 200)).collect();
                    (op, rhs, format!("k={k}"))
                })
                .collect();
            out.push(Sequence {
                set: "held_out",
                label: format!("convection-diffusion-200-Pe{peclet}-tau{tau:e}"),
                groups,
                rtol: 1.0e-11,
                budget: 200,
            });
        }
    }
    out
}

fn gcrodr_config(rtol: f64, budget: usize) -> GcrodrConfig {
    let gamma = rodas5p_coefficients().unwrap().gamma;
    GcrodrConfig {
        restart: 40,
        max_arnoldi: budget,
        recycle_dim: 8,
        rank_tol: 1.0e-12,
        rtol,
        atol: 1.0e-14 * gamma.abs(),
    }
}

fn threshold(config: &GcrodrConfig, b: &[f64]) -> f64 {
    config.atol.max(config.rtol * safe_l2(b))
}

fn independent_residual(op: &dyn LinearOperator, b: &[f64], x: &[f64]) -> f64 {
    let mut ax = vec![0.0; b.len()];
    op.apply(x, &mut ax).unwrap();
    safe_l2(&b.iter().zip(&ax).map(|(p, q)| p - q).collect::<Vec<_>>())
}

/// Post-hoc diagnostic (not part of the preregistered gate): the recycle
/// invariant `A U = C` and `C^T C = I` of the carried state before each
/// solve of control C on the trajectory and one-step sets.
fn invariant(op: &dyn LinearOperator, state: &GcrodrState) -> (f64, f64) {
    let mut defect = 0.0_f64;
    for (u, c) in state.basis.iter().zip(&state.image) {
        let mut au = vec![0.0; u.len()];
        op.apply(u, &mut au).unwrap();
        let diff = safe_l2(&au.iter().zip(c).map(|(a, b)| a - b).collect::<Vec<_>>());
        defect = defect.max(diff / safe_l2(c).max(f64::MIN_POSITIVE));
    }
    let mut gram = 0.0_f64;
    for (i, a) in state.image.iter().enumerate() {
        for (j, b) in state.image.iter().enumerate() {
            let p: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
            gram = gram.max((p - if i == j { 1.0 } else { 0.0 }).abs());
        }
    }
    (defect, gram)
}

#[test]
#[ignore = "post-hoc diagnostic of research/rnext03_gcrodr_attribution_20261003; release build"]
fn recycle_invariant_probe() {
    let (p, y0) = brusselator(50).unwrap();
    let problem = p.jvp_only_clone().unwrap();
    let (sequences, _) = trajectory_and_one_step(&problem, &y0);
    let mut rows = Vec::new();
    for seq in &sequences {
        let config = gcrodr_config(seq.rtol, seq.budget);
        let mut state = GcrodrState::default();
        let mut ws = GcrodrWorkspace::default();
        for (op, rhs, group) in &seq.groups {
            let pc = IdentityPreconditioner::new(op.dimension());
            for (i, b) in rhs.iter().enumerate() {
                let (before_defect, before_gram) = invariant(op.as_ref(), &state);
                let mut trace = GcrodrTrace::default();
                let result = solve_gcrodr_traced(
                    op.as_ref(),
                    &pc,
                    b,
                    None,
                    &config,
                    &mut state,
                    None,
                    &mut ws,
                    None,
                    &mut trace,
                    &mut WorkCounters::default(),
                );
                let (after_defect, after_gram) = invariant(op.as_ref(), &state);
                rows.push(json!({
                    "sequence": seq.label, "group": group, "rhs": i, "rank_before": trace.cycles.first().map(|c| c.recycle_rank),
                    "invariant_defect_before": before_defect, "gram_defect_before": before_gram,
                    "invariant_defect_after": after_defect, "gram_defect_after": after_gram,
                    "ok": result.is_ok(),
                }));
            }
        }
    }
    write_output(
        "RNEXT03_POSTHOC_OUTPUT",
        &json!({"schema": "vigilode-rnext03-posthoc-invariant-v1", "rows": rows}),
    );
}
