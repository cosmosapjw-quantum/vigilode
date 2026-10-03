//! GCRO-DR failure attribution on frozen Brusselator systems (research node
//! `research/rnext03_gcrodr_attribution_20261003`, remaining-only DAG node
//! R-NEXT-03). Run with `--ignored --release --test-threads=1`.

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
            "matvecs": c.matvecs, "reset": c.reset,
        })).collect::<Vec<_>>(),
    })
}

/// Per-cycle accounting: Arnoldi columns plus the projection residual (if
/// the cycle started with a recycle space) plus the end residual; outside
/// the cycles at most one residual per loop entry, the refreshes and the
/// final check.
fn accounted(trace: &GcrodrTrace, refresh: u64) -> bool {
    let cycles_ok = trace.cycles.iter().all(|c| {
        let projection = u64::from(c.recycle_rank > 0);
        if c.arnoldi_columns == 0 {
            c.matvecs == projection
        } else {
            c.matvecs == c.arnoldi_columns as u64 + projection + 1
        }
    });
    let loops = trace.cycles.len() as u64 + 1;
    cycles_ok && trace.matvecs_outside_cycles <= loops + refresh + 1
}

#[derive(Default)]
struct Tally {
    failures: usize,
    solves: usize,
    matvecs: u64,
}

/// Runs the four controls on one sequence; returns per-solve records.
fn run_sequence(seq: &Sequence, tallies: &mut [Tally; 4], gate3: &mut bool) -> Vec<Value> {
    let config = gcrodr_config(seq.rtol, seq.budget);
    let gmres = GmresConfig {
        restart: 40,
        max_arnoldi: seq.budget,
        rtol: config.rtol,
        atol: config.atol,
    };
    let mut carried_c = GcrodrState::default();
    let mut carried_d = GcrodrState::default();
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
                    *gate3 &= r <= limit * (1.0 + 1.0e-12);
                    json!({"ok": true, "iterations": report.iterations, "matvecs": matvecs, "true_residual": r})
                }
                Err(error) => {
                    tallies[0].failures += 1;
                    json!({"ok": false, "error": error.to_string(), "matvecs": matvecs})
                }
            };
            // B: cold GCRO-DR; C: carried; D: carried with the reset rule.
            for (slot, name, state, reset) in [
                (1usize, "B_cold_gcrodr", None, None),
                (2, "C_recycled_gcrodr", Some(&mut carried_c), None),
                (3, "D_recycled_reset", Some(&mut carried_d), Some(0.5)),
            ] {
                let mut fresh = GcrodrState::default();
                let state = match state {
                    Some(s) => s,
                    None => &mut fresh,
                };
                let mut trace = GcrodrTrace::default();
                let mut counters = WorkCounters::default();
                let result = solve_gcrodr_traced(
                    op.as_ref(),
                    &pc,
                    b,
                    None,
                    &config,
                    state,
                    None,
                    &mut ws,
                    reset,
                    &mut trace,
                    &mut counters,
                );
                let charged = counters.linear_matvecs
                    + counters.diagnostic_matvecs
                    + counters.recycle_refresh_matvecs;
                *gate3 &= charged == trace.matvecs_total
                    && accounted(&trace, counters.recycle_refresh_matvecs);
                tallies[slot].solves += 1;
                tallies[slot].matvecs += trace.matvecs_total;
                entry[name] = match &result {
                    Ok(report) => {
                        let r = independent_residual(op.as_ref(), b, &report.x);
                        *gate3 &= r <= limit * (1.0 + 1.0e-12);
                        json!({"ok": true, "iterations": report.iterations, "true_residual": r,
                               "cycles": trace.cycles.len(), "matvecs": trace.matvecs_total,
                               "resets": trace.cycles.iter().filter(|c| c.reset).count()})
                    }
                    Err(error) => {
                        tallies[slot].failures += 1;
                        json!({"ok": false, "error": error.to_string(), "trace": trace_json(&trace),
                               "matvecs": trace.matvecs_total,
                               "resets": trace.cycles.iter().filter(|c| c.reset).count()})
                    }
                };
            }
            records.push(entry);
        }
    }
    records
}

/// R1-R5 for every failure of control C.
fn classify(records: &[Value]) -> (Vec<Value>, usize) {
    let mut out = Vec::new();
    let mut unclassified = 0;
    for r in records {
        let c = &r["C_recycled_gcrodr"];
        if c["ok"].as_bool().unwrap() {
            continue;
        }
        let mut labels = Vec::new();
        if !r["B_cold_gcrodr"]["ok"].as_bool().unwrap() {
            labels.push("R1_restart_length_stagnation");
        } else {
            labels.push("R2_recycle_induced");
        }
        let cycles = c["trace"]["cycles"].as_array().unwrap();
        if cycles.iter().any(|cy| {
            let ls = cy["ls_residual"].as_f64().unwrap();
            cy["arnoldi_columns"].as_u64().unwrap() > 0
                && cy["residual_end"].as_f64().unwrap() > 10.0 * ls
        }) {
            labels.push("R3_residual_gap");
        }
        if cycles.iter().any(|cy| {
            cy["CtC_defect"].as_f64().unwrap() > 1.0e-8 || cy["CtV"].as_f64().unwrap() > 1.0e-8
        }) {
            labels.push("R4_orthogonality_loss");
        }
        if c["error"].as_str().unwrap().contains("least-squares") {
            labels.push("R5_nonfinite_small_problem");
        }
        if labels.is_empty() {
            unclassified += 1;
        }
        out.push(
            json!({"group": r["group"], "rhs": r["rhs"], "error": c["error"], "labels": labels}),
        );
    }
    (out, unclassified)
}

fn study() -> Value {
    let (p, y0) = brusselator(50).unwrap();
    let problem = p.jvp_only_clone().unwrap();
    let (mut sequences, info) = trajectory_and_one_step(&problem, &y0);
    sequences.extend(held_out());
    let mut by_set = serde_json::Map::new();
    let mut gate3 = true;
    let mut all_records = Vec::new();
    for set in ["trajectory", "one_step", "held_out"] {
        let mut tallies: [Tally; 4] = Default::default();
        let mut set_records = Vec::new();
        for seq in sequences.iter().filter(|s| s.set == set) {
            let records = run_sequence(seq, &mut tallies, &mut gate3);
            set_records.push(json!({"sequence": seq.label, "solves": records}));
            if set != "held_out" {
                all_records.extend(records);
            }
        }
        let names = [
            "A_cold_gmres",
            "B_cold_gcrodr",
            "C_recycled_gcrodr",
            "D_recycled_reset",
        ];
        let summary: serde_json::Map<String, Value> = names
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
    let (classified, unclassified) = classify(&all_records);
    json!({"info": info, "sets": by_set, "classification": classified,
           "unclassified": unclassified, "gate3_holds": gate3})
}

#[test]
#[ignore = "research run of research/rnext03_gcrodr_attribution_20261003; release build"]
fn gcrodr_attribution() {
    let first = study();
    let second = study();
    let reproducible = first == second;
    let failures = |set: &str, control: &str| {
        first["sets"][set]["summary"][control]["failures"]
            .as_u64()
            .unwrap()
    };
    let matvecs = |set: &str, control: &str| {
        first["sets"][set]["summary"][control]["matvecs"]
            .as_u64()
            .unwrap()
    };
    let c_fail =
        failures("trajectory", "C_recycled_gcrodr") + failures("one_step", "C_recycled_gcrodr");
    let d_fail =
        failures("trajectory", "D_recycled_reset") + failures("one_step", "D_recycled_reset");
    let gate = json!({
        "1_reproduction": c_fail > 0 && reproducible,
        "2_attribution": first["unclassified"].as_u64().unwrap() == 0,
        "3_no_false_convergence_and_accounting": first["gate3_holds"].as_bool().unwrap(),
        "4_reset_rule": d_fail < c_fail
            && failures("held_out", "D_recycled_reset") <= failures("held_out", "C_recycled_gcrodr")
            && (matvecs("held_out", "D_recycled_reset") as f64)
                <= 1.10 * matvecs("held_out", "C_recycled_gcrodr") as f64,
        "control_C_failures_trajectory_plus_one_step": c_fail,
        "control_D_failures_trajectory_plus_one_step": d_fail,
        "reproducible_in_process": reproducible,
    });
    let mut report = first;
    report["schema"] = json!("vigilode-rnext03-gcrodr-attribution-v1");
    report["gate"] = gate;
    write_output("RNEXT03_OUTPUT", &report);
    println!("{}", serde_json::to_string_pretty(&report["gate"]).unwrap());
    for set in ["trajectory", "one_step", "held_out"] {
        println!("{set}: {}", report["sets"][set]["summary"]);
    }
}
