//! Post-hoc diagnostic of research node
//! `research/int01_gcrodr_verified_reuse_20261003` (after its recorded run;
//! not part of its gate): the same study with control E's reuse tolerance
//! taken from `INT01_POSTHOC_TOL`. Run with `--ignored --release
//! --test-threads=1`.

#[path = "rnext_common/mod.rs"]
mod common;

use common::{
    adaptive, brusselator, linear_config, splitmix_vector, stage_operator, stage_right_hand_sides,
};
use rodas5p_core::{
    DenseMatrix, DenseOperator, IdentityPreconditioner, LinearMethod, LinearOperator, WorkCounters,
    rodas5p_coefficients, safe_l2,
};
use rodas5p_integrators::{
    OdeProblem, OutputSchedule, Rodas5pMfFastWorkspace, integrate_rodas5p_mf_fast_observed_traced,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrReusePolicy, GcrodrState, GcrodrTrace, GcrodrWorkspace, GmresConfig,
    GmresWorkspace, solve_gcrodr_with_policy, solve_gmres_with_workspace,
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

/// `I - tau (D2 - Pe D1 - r I)` on n interior points of (0, 1), central
/// differences, homogeneous Dirichlet ends, r = 10.
fn convection_diffusion_reaction(n: usize, peclet: f64, tau: f64) -> DenseMatrix {
    let h = 1.0 / (n as f64 + 1.0);
    let (d, c, r) = (1.0 / (h * h), peclet / (2.0 * h), 10.0);
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 1.0 + tau * (2.0 * d + r);
        if i > 0 {
            a[(i, i - 1)] = -tau * (d + c);
        }
        if i + 1 < n {
            a[(i, i + 1)] = -tau * (d - c);
        }
    }
    a
}

/// The fresh family of the preregistration (not used by any earlier node).
fn fresh() -> Vec<Sequence> {
    let n = 120;
    let mut out = Vec::new();
    for peclet in [0.5, 5.0, 50.0] {
        for tau in [1.0e-3, 4.0e-3] {
            let groups = (0..6)
                .map(|k| {
                    let a = convection_diffusion_reaction(n, peclet, tau * (1.0 + 0.02 * k as f64));
                    let op = DenseOperator::new(a).unwrap();
                    let rhs = (0..8)
                        .map(|i| {
                            let noise = splitmix_vector(9001 + i as u64, n);
                            let u: Vec<f64> = (0..n)
                                .map(|j| {
                                    let x = (j + 1) as f64 / (n + 1) as f64;
                                    (std::f64::consts::PI * (i + 1) as f64 * x).sin()
                                        * (1.0 + 0.1 * i as f64)
                                        + 0.1 * noise[j]
                                })
                                .collect();
                            let mut b = vec![0.0; n];
                            op.apply(&u, &mut b).unwrap();
                            b
                        })
                        .collect();
                    let op: Box<dyn LinearOperator> = Box::new(op);
                    (op, rhs, format!("k={k}"))
                })
                .collect();
            out.push(Sequence {
                set: "fresh",
                label: format!("convection-diffusion-reaction-{n}-Pe{peclet}-tau{tau:e}"),
                groups,
                rtol: 1.0e-9,
                budget: 400,
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

fn trace_json(trace: &GcrodrTrace) -> Value {
    json!({
        "matvecs_total": trace.matvecs_total,
        "matvecs_outside_cycles": trace.matvecs_outside_cycles,
        "cycles": trace.cycles.iter().map(|c| json!({
            "rank": c.recycle_rank, "residual_start": c.residual_start,
            "retained_fraction": c.retained_fraction,
            "residual_after_projection": c.residual_after_projection,
            "arnoldi_columns": c.arnoldi_columns, "ls_residual": c.least_squares_residual,
            "residual_end": c.residual_end, "CtC_defect": c.recycle_gram_defect,
            "CtV": c.recycle_arnoldi_coupling, "VtV_defect": c.arnoldi_gram_defect,
            "matvecs": c.matvecs, "reset": c.reset, "aborted": c.aborted,
        })).collect::<Vec<_>>(),
    })
}

/// Per-cycle accounting (L-0046's rule for completed cycles; an aborted
/// cycle has at most `restart + 2` products); outside the cycles at most one
/// residual per loop entry, the refresh and check products, and the final
/// check.
fn accounted(trace: &GcrodrTrace, refresh: u64, restart: usize) -> bool {
    let cycles_ok = trace.cycles.iter().all(|c| {
        let projection = u64::from(c.recycle_rank > 0);
        if c.aborted {
            c.matvecs <= restart as u64 + 2
        } else if c.arnoldi_columns == 0 {
            c.matvecs == projection
        } else {
            c.matvecs == c.arnoldi_columns as u64 + projection + 1
        }
    });
    let loops = trace.cycles.len() as u64 + 1;
    cycles_ok && trace.matvecs_outside_cycles <= loops + refresh + 1
}

fn posthoc_tolerance() -> f64 {
    std::env::var("INT01_POSTHOC_TOL")
        .expect("INT01_POSTHOC_TOL")
        .parse()
        .unwrap()
}

const CONTROLS: [&str; 5] = [
    "A_cold_gmres",
    "B_cold_gcrodr",
    "C_recycled_gcrodr",
    "D_recycled_reset",
    "E_recycled_verified",
];

#[derive(Default)]
struct Tally {
    failures: usize,
    solves: usize,
    matvecs: u64,
}

/// Runs the controls on one sequence (only A with `gmres_only`); returns
/// per-solve records.
fn run_sequence(
    seq: &Sequence,
    tallies: &mut [Tally; 5],
    gate: &mut Gates,
    gmres_only: bool,
) -> Vec<Value> {
    let config = gcrodr_config(seq.rtol, seq.budget);
    let gmres = GmresConfig {
        restart: 40,
        max_arnoldi: seq.budget,
        rtol: config.rtol,
        atol: config.atol,
    };
    let mut carried_c = GcrodrState::default();
    let mut carried_d = GcrodrState::default();
    let mut carried_e = GcrodrState::default();
    let (mut ws_g, mut ws) = (GmresWorkspace::default(), GcrodrWorkspace::default());
    let mut records = Vec::new();
    for (op, rhs, group) in &seq.groups {
        let pc = IdentityPreconditioner::new(op.dimension());
        for (i, b) in rhs.iter().enumerate() {
            let limit = threshold(&config, b);
            let mut entry = json!({"group": group, "rhs": i, "threshold": limit});
            // A: cold GMRES.
            let mut counters = WorkCounters::default();
            let a = solve_gmres_with_workspace(
                op.as_ref(),
                &pc,
                b,
                None,
                &gmres,
                &mut ws_g,
                &mut counters,
            );
            let matvecs = counters.linear_matvecs + counters.diagnostic_matvecs;
            tallies[0].solves += 1;
            tallies[0].matvecs += matvecs;
            entry["A_cold_gmres"] = match &a {
                Ok(report) => {
                    let r = independent_residual(op.as_ref(), b, &report.x);
                    gate.no_false_convergence &= r <= limit * (1.0 + 1.0e-12);
                    json!({"ok": true, "iterations": report.iterations, "matvecs": matvecs, "true_residual": r})
                }
                Err(error) => {
                    tallies[0].failures += 1;
                    json!({"ok": false, "error": error.to_string(), "matvecs": matvecs})
                }
            };
            if gmres_only {
                records.push(entry);
                continue;
            }
            // B: cold GCRO-DR; C: carried; D: carried with the reset rule;
            // E: carried with the reuse check.
            let verified = GcrodrReusePolicy {
                reset_factor: None,
                verify_reuse: Some(posthoc_tolerance()),
            };
            let reset = GcrodrReusePolicy {
                reset_factor: Some(0.5),
                verify_reuse: None,
            };
            for (slot, name, state, policy) in [
                (1usize, "B_cold_gcrodr", None, GcrodrReusePolicy::default()),
                (
                    2,
                    "C_recycled_gcrodr",
                    Some(&mut carried_c),
                    GcrodrReusePolicy::default(),
                ),
                (3, "D_recycled_reset", Some(&mut carried_d), reset),
                (4, "E_recycled_verified", Some(&mut carried_e), verified),
            ] {
                let mut fresh = GcrodrState::default();
                let state = match state {
                    Some(s) => s,
                    None => &mut fresh,
                };
                let mut trace = GcrodrTrace::default();
                let mut counters = WorkCounters::default();
                let result = solve_gcrodr_with_policy(
                    op.as_ref(),
                    &pc,
                    b,
                    None,
                    &config,
                    state,
                    None,
                    &mut ws,
                    policy,
                    &mut trace,
                    &mut counters,
                );
                let charged = counters.linear_matvecs
                    + counters.diagnostic_matvecs
                    + counters.recycle_refresh_matvecs;
                gate.accounting &= charged == trace.matvecs_total
                    && accounted(&trace, counters.recycle_refresh_matvecs, config.restart);
                let checks = trace.reuse_checks.len();
                let rebuilt = trace.reuse_checks.iter().filter(|c| c.rebuilt).count();
                let max_defect = trace
                    .reuse_checks
                    .iter()
                    .map(|c| c.defect)
                    .fold(0.0_f64, f64::max);
                tallies[slot].solves += 1;
                tallies[slot].matvecs += trace.matvecs_total;
                entry[name] = match &result {
                    Ok(report) => {
                        let r = independent_residual(op.as_ref(), b, &report.x);
                        gate.no_false_convergence &= r <= limit * (1.0 + 1.0e-12);
                        json!({"ok": true, "iterations": report.iterations, "true_residual": r,
                               "cycles": trace.cycles.len(), "matvecs": trace.matvecs_total,
                               "resets": trace.cycles.iter().filter(|c| c.reset).count(),
                               "checks": checks, "rebuilt": rebuilt, "max_defect": max_defect})
                    }
                    Err(error) => {
                        tallies[slot].failures += 1;
                        json!({"ok": false, "error": error.to_string(), "trace": trace_json(&trace),
                               "matvecs": trace.matvecs_total,
                               "resets": trace.cycles.iter().filter(|c| c.reset).count(),
                               "checks": checks, "rebuilt": rebuilt, "max_defect": max_defect})
                    }
                };
            }
            records.push(entry);
        }
    }
    records
}

#[derive(Clone, Copy)]
struct Gates {
    accounting: bool,
    no_false_convergence: bool,
}

fn study(gmres_only: bool) -> Value {
    let (p, y0) = brusselator(50).unwrap();
    let problem = p.jvp_only_clone().unwrap();
    let mut sequences = Vec::new();
    let mut info = Value::Null;
    if !gmres_only {
        let (generated, generated_info) = trajectory_and_one_step(&problem, &y0);
        sequences = generated;
        info = generated_info;
    }
    sequences.extend(fresh());
    let mut by_set = serde_json::Map::new();
    let mut gate = Gates {
        accounting: true,
        no_false_convergence: true,
    };
    for set in ["trajectory", "one_step", "fresh"] {
        let mut tallies: [Tally; 5] = Default::default();
        let mut set_records = Vec::new();
        for seq in sequences.iter().filter(|s| s.set == set) {
            let records = run_sequence(seq, &mut tallies, &mut gate, gmres_only);
            set_records.push(json!({"sequence": seq.label, "solves": records}));
        }
        let summary: serde_json::Map<String, Value> = CONTROLS
            .iter()
            .zip(&tallies)
            .map(|(n, t)| {
                (
                    n.to_string(),
                    json!({"solves": t.solves, "failures": t.failures, "matvecs": t.matvecs}),
                )
            })
            .collect();
        by_set.insert(
            set.into(),
            json!({"summary": summary, "sequences": set_records}),
        );
    }
    json!({"info": info, "sets": by_set,
           "accounting_holds": gate.accounting,
           "no_false_convergence_holds": gate.no_false_convergence})
}

#[test]
#[ignore = "post-hoc diagnostic of research/int01_gcrodr_verified_reuse_20261003; release build"]
fn gcrodr_reuse_tolerance_posthoc() {
    // Without INT01_POSTHOC_TOL (the workspace's ignored-test run) there is
    // no tolerance to diagnose.
    if std::env::var("INT01_POSTHOC_TOL").is_err() {
        println!("INT01_POSTHOC_TOL not set: post-hoc diagnostic skipped");
        return;
    }
    let report = study(false);
    let mut out = serde_json::Map::new();
    for set in ["trajectory", "one_step", "fresh"] {
        out.insert(set.into(), report["sets"][set]["summary"].clone());
    }
    let summary = json!({
        "tolerance": posthoc_tolerance(),
        "summary": out,
        "accounting_holds": report["accounting_holds"],
        "no_false_convergence_holds": report["no_false_convergence_holds"],
    });
    println!("{summary}");
    if let Ok(path) = std::env::var("INT01_POSTHOC_OUTPUT") {
        let mut all: Value = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| json!([]));
        all.as_array_mut().unwrap().push(summary);
        std::fs::write(&path, serde_json::to_string_pretty(&all).unwrap() + "\n").unwrap();
    }
}
