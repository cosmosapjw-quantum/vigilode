//! GCRO-DR with the first Arnoldi vector orthogonalized against the recycle
//! images (research node `research/rev01_gcrodr_start_projection_20261003`,
//! review DAG node REV-01). Run with `--ignored --release --test-threads=1`.

mod rev01_common;

use rev01_common::{
    Sequence, common::brusselator, common::write_output, fresh, gcrodr_config,
    independent_residual, threshold, trajectory_and_one_step,
};
use rodas5p_core::{IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{
    GcrodrReusePolicy, GcrodrSolveOptions, GcrodrState, GcrodrTrace, GcrodrWorkspace, GmresConfig,
    GmresWorkspace, solve_gcrodr_with_options, solve_gmres_with_workspace,
};
use serde_json::{Value, json};

const CONTROLS: [&str; 6] = [
    "A_cold_gmres",
    "B_cold_gcrodr",
    "C_recycled",
    "E_reuse_check",
    "F_start_projection",
    "EF_both",
];

#[derive(Default, Clone, Copy)]
struct Tally {
    solves: usize,
    failures: usize,
    matvecs: u64,
}

struct Gates {
    accounting: bool,
    no_false_convergence: bool,
}

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
    cycles_ok && trace.matvecs_outside_cycles <= trace.cycles.len() as u64 + 1 + refresh + 1
}

fn run_sequence(seq: &Sequence, tallies: &mut [Tally; 6], gate: &mut Gates) -> Vec<Value> {
    let config = gcrodr_config(seq.rtol, seq.budget);
    let gmres = GmresConfig {
        restart: 40,
        max_arnoldi: seq.budget,
        rtol: config.rtol,
        atol: config.atol,
    };
    let check = GcrodrReusePolicy {
        reset_factor: None,
        verify_reuse: Some(1.0e-8),
    };
    let options = [
        GcrodrSolveOptions::default(),
        GcrodrSolveOptions::default(),
        GcrodrSolveOptions {
            policy: check,
            orthogonalize_start: false,
        },
        GcrodrSolveOptions {
            policy: GcrodrReusePolicy::default(),
            orthogonalize_start: true,
        },
        GcrodrSolveOptions {
            policy: check,
            orthogonalize_start: true,
        },
    ];
    let mut states: Vec<GcrodrState> = (0..5).map(|_| GcrodrState::default()).collect();
    let (mut ws_g, mut ws) = (GmresWorkspace::default(), GcrodrWorkspace::default());
    let mut records = Vec::new();
    for (op, rhs, group) in &seq.groups {
        let pc = IdentityPreconditioner::new(op.dimension());
        for (i, b) in rhs.iter().enumerate() {
            let limit = threshold(&config, b);
            let mut entry = json!({"group": group, "rhs": i});
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
            tallies[0].solves += 1;
            tallies[0].matvecs += counters.linear_matvecs + counters.diagnostic_matvecs;
            entry[CONTROLS[0]] = match &a {
                Ok(report) => {
                    let r = independent_residual(op.as_ref(), b, &report.x);
                    gate.no_false_convergence &= r <= limit * (1.0 + 1.0e-12);
                    json!({"ok": true})
                }
                Err(_) => {
                    tallies[0].failures += 1;
                    json!({"ok": false})
                }
            };
            for (slot, opts) in options.iter().enumerate() {
                let mut cold = GcrodrState::default();
                let state = if slot == 0 {
                    &mut cold
                } else {
                    &mut states[slot]
                };
                let mut trace = GcrodrTrace::default();
                let mut counters = WorkCounters::default();
                let result = solve_gcrodr_with_options(
                    op.as_ref(),
                    &pc,
                    b,
                    None,
                    &config,
                    state,
                    None,
                    &mut ws,
                    *opts,
                    &mut trace,
                    &mut counters,
                );
                let charged = counters.linear_matvecs
                    + counters.diagnostic_matvecs
                    + counters.recycle_refresh_matvecs;
                gate.accounting &= charged == trace.matvecs_total
                    && accounted(&trace, counters.recycle_refresh_matvecs, config.restart);
                let t = &mut tallies[slot + 1];
                t.solves += 1;
                t.matvecs += trace.matvecs_total;
                let max_ctv = trace
                    .cycles
                    .iter()
                    .map(|c| c.recycle_arnoldi_coupling)
                    .fold(0.0_f64, f64::max);
                entry[CONTROLS[slot + 1]] = match &result {
                    Ok(report) => {
                        let r = independent_residual(op.as_ref(), b, &report.x);
                        gate.no_false_convergence &= r <= limit * (1.0 + 1.0e-12);
                        json!({"ok": true, "matvecs": trace.matvecs_total, "max_CtV": max_ctv})
                    }
                    Err(error) => {
                        t.failures += 1;
                        json!({"ok": false, "error": error.to_string(), "matvecs": trace.matvecs_total,
                               "max_CtV": max_ctv, "cycles": trace.cycles.len()})
                    }
                };
            }
            records.push(entry);
        }
    }
    records
}

#[test]
#[ignore = "research run of research/rev01_gcrodr_start_projection_20261003; release build"]
fn gcrodr_start_projection() {
    let mut gate = Gates {
        accounting: true,
        no_false_convergence: true,
    };
    let mut sets = serde_json::Map::new();
    let mut recycle_induced_f = 0usize;
    let mut no_harm = true;
    let mut info = serde_json::Map::new();
    let mut groups: Vec<(String, Vec<Sequence>)> = Vec::new();
    for cells in [50usize, 80] {
        let (p, y0) = brusselator(cells).unwrap();
        let problem = p.jvp_only_clone().unwrap();
        let (sequences, generated) = trajectory_and_one_step(&problem, &y0, cells);
        info.insert(format!("brusselator-{cells}"), generated);
        let (traj, one): (Vec<_>, Vec<_>) =
            sequences.into_iter().partition(|s| s.set == "trajectory");
        groups.push((format!("brusselator-{cells}-trajectory"), traj));
        groups.push((format!("brusselator-{cells}-one-step"), one));
    }
    groups.push(("cdr-fresh".into(), fresh()));
    for (name, sequences) in &groups {
        let mut tallies = [Tally::default(); 6];
        let mut seq_records = Vec::new();
        for seq in sequences {
            let records = run_sequence(seq, &mut tallies, &mut gate);
            if name.starts_with("brusselator") {
                recycle_induced_f += records
                    .iter()
                    .filter(|r| {
                        r["B_cold_gcrodr"]["ok"] == json!(true)
                            && r["F_start_projection"]["ok"] == json!(false)
                    })
                    .count();
            }
            seq_records.push(json!({"sequence": seq.label, "solves": records}));
        }
        if name == "cdr-fresh" {
            no_harm = tallies[4].failures <= tallies[2].failures;
        }
        let summary: serde_json::Map<String, Value> = CONTROLS
            .iter()
            .zip(&tallies)
            .map(|(c, t)| {
                (
                    c.to_string(),
                    json!({"solves": t.solves, "failures": t.failures, "matvecs": t.matvecs}),
                )
            })
            .collect();
        println!("{name}: {}", Value::Object(summary.clone()));
        sets.insert(
            name.clone(),
            json!({"summary": summary, "sequences": seq_records}),
        );
    }
    let report = json!({
        "schema": "vigilode-rev01-gcrodr-start-projection-v1",
        "info": info,
        "sets": sets,
        "gate": {
            "1_defaults_unchanged": "contract tests rev01_gcrodr_options_contracts, int01_gcrodr_policy_contracts, rnext03_gcrodr_trace_contracts",
            "2_failures_removed": recycle_induced_f == 0,
            "3_no_false_convergence_and_accounting": gate.accounting && gate.no_false_convergence,
            "4_no_harm_fresh_cdr": no_harm,
            "recycle_induced_failures_of_F": recycle_induced_f,
            "accounting": gate.accounting,
            "no_false_convergence": gate.no_false_convergence,
        },
    });
    write_output("REV01_OUTPUT", &report);
    println!("{}", serde_json::to_string_pretty(&report["gate"]).unwrap());
}
