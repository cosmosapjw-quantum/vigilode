//! Measurement program of loop node V17 (matrix-free stage solve).
//!
//! One process does one thing, chosen by `V17_MODE`, and prints
//! machine-readable lines. Nothing is compared or judged here;
//! `research/production_loop_20261116/mf_stage_solve/parse_results.py`
//! builds the results from these lines.
//!
//! ```text
//! V17_MODE=RUN V17_FAMILY=ac3d V17_RTOL=1e-5 V17_ARM=B V17_AUDIT=1 \
//!   <test binary> --ignored --nocapture --exact v17_run
//! ```
//!
//! Modes:
//!
//! * `SETUP`: builds the family and its start state, nothing else (the
//!   instruction baseline every run is charged with).
//! * `RUN`: one arm (`V17_ARM` = `A`, `B0`, `B`, `C`) at `V17_RTOL`;
//!   `V17_AUDIT=1` checks every stage solve against the original residual.
//!   Exploratory switches: `V17_RESTART` (columns per restart pass),
//!   `V17_PRECOND=jacobi` (client arms), `V17_KAPPA` (arm C).
//!   Prints `V17_RUN_JSON` and `V17_STATE_JSON` (final state, hex).
//! * `TIMING`: `V17_ROUNDS` rounds (default 9) of the arms `V17_ARMS`
//!   (default `A,B,C`), each arm once per round in the order rotated by the
//!   round index plus `V17_ROUND_OFFSET`. Prints one `V17_TIMING_JSON` per
//!   run and a `V17_TIMING_META_JSON` with `/proc/loadavg` at start and end.
//! * `CALIB_JVP`, `CALIB_RHS`: `V17_CALLS` (default 20000) JVPs or
//!   right-hand sides at the start state (instructions per call).
//! * `RHS0`: prints the start state and `f(t0, y0)` as hex (`V17_RHS0_JSON`).
//!
//! `V17_FAMILY` is an id of `problems::declared`; `V17_LABEL` is copied into
//! every record.

#[path = "stage_clients_krylov_v17_problems.rs"]
mod problems;

use std::time::Instant;

use problems::{Arm, ArmOptions, ArmRun, Family, adaptive, declared, run_arm};
use rodas5p_core::WorkCounters;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn flag(name: &str) -> bool {
    variable(name).is_some_and(|value| value == "1")
}

fn number<T: std::str::FromStr>(name: &str) -> Option<T> {
    variable(name).map(|value| {
        value
            .parse()
            .unwrap_or_else(|_| panic!("{name} is not a valid number: {value}"))
    })
}

fn hex(values: &[f64]) -> String {
    let mut text = String::with_capacity(16 * values.len());
    for value in values {
        text.push_str(&format!("{:016x}", value.to_bits()));
    }
    text
}

fn sha256(values: &[f64]) -> String {
    let mut hasher = Sha256::new();
    for value in values {
        hasher.update(value.to_le_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn loadavg() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
}

/// The options of the arm `label` with the exploratory switches of the
/// environment.
fn arm_options(label: &str) -> ArmOptions {
    let arm = Arm::parse(label).unwrap_or_else(|| panic!("unknown arm {label}"));
    let mut options = ArmOptions::new(arm);
    options.restart = number("V17_RESTART");
    options.jacobi = variable("V17_PRECOND").is_some_and(|value| value == "jacobi");
    if let Some(kappa) = number("V17_KAPPA") {
        options.kappa = kappa;
    }
    options
}

fn options_json(options: &ArmOptions) -> Value {
    json!({
        "arm": options.arm.label(),
        "audit": options.audit,
        "restart": options.gmres().restart,
        "restart_as_shipped": options.restart.is_none(),
        "precond": if options.jacobi { "jacobi" } else { "none" },
        "kappa": if options.arm == Arm::StepTied { json!(options.kappa) } else { Value::Null },
    })
}

fn run_json(run: &ArmRun) -> Value {
    let mut record = json!({
        "success": run.success,
        "attempts": run.attempts,
        "accepted": run.accepted,
        "rejected": run.rejected,
        "t_end": run.t_end,
        "jvp_calls": run.counters.jvp_calls,
        "counters": serde_json::to_value(run.counters).expect("counters"),
        "state_sha256": sha256(&run.y_end),
    });
    let fields = record.as_object_mut().expect("object");
    if let Some(stats) = &run.stats {
        fields.insert(
            "client".into(),
            json!({
                "linearizations": stats.linearizations,
                "attempts": stats.attempts,
                "solves": stats.solves,
                "failed_solves": stats.failed_solves,
                "krylov_vectors": stats.krylov_vectors,
                "residual_jvps": stats.residual_jvps,
                "restart_passes": stats.restart_passes,
                "max_solve_vectors": stats.max_solve_vectors,
                "zero_iterate_accepts": stats.zero_iterate_accepts,
                "stage_krylov_vectors": stats.stage_krylov_vectors,
                "stage_solves": stats.stage_solves,
                "policy_bound_solves": stats.policy_bound_solves,
                "gmres_storage_bytes": stats.gmres_storage_bytes,
                "jvp_accounting_closed":
                    stats.krylov_vectors + stats.residual_jvps == run.counters.jvp_calls,
            }),
        );
    }
    if let Some(audit) = &run.audit {
        fields.insert(
            "residual_audit".into(),
            json!({
                "checks": audit.checks,
                "failures": audit.failures,
                "max_ratio": audit.max_ratio,
                "zero_threshold_failures": audit.zero_threshold_failures,
                "first_failure": audit.first_failure,
                "jvp_calls": audit.jvp_calls,
                "rhs_calls": audit.rhs_calls,
            }),
        );
    }
    if let Some(log) = &run.solve_log {
        fields.insert(
            "solve_log".into(),
            json!({
                "solves": log.solves,
                "iterations": log.iterations,
                "max_iterations": log.max_iterations,
                "pass_histogram": log.pass_histogram,
            }),
        );
    }
    record
}

fn header(mode: &str, family_id: &str, family: &Family) -> Value {
    json!({
        "schema": "vigilode.loop-l1.v17.run.v1",
        "label": variable("V17_LABEL").unwrap_or_default(),
        "mode": mode,
        "family": family_id,
        "n": family.problem.dimension,
        "t_span": [family.span.0, family.span.1],
    })
}

fn merge(target: &mut Value, source: Value) {
    let fields = target.as_object_mut().expect("object");
    for (key, value) in source.as_object().expect("object") {
        fields.insert(key.clone(), value.clone());
    }
}

fn mode_run(family_id: &str, family: &Family) {
    let rtol: f64 = number("V17_RTOL").expect("V17_RTOL");
    let mut options = arm_options(&variable("V17_ARM").expect("V17_ARM"));
    options.audit = flag("V17_AUDIT");
    let config = adaptive(family, rtol);
    let mut record = header("RUN", family_id, family);
    merge(
        &mut record,
        json!({"rtol": config.rtol, "atol": config.atol}),
    );
    merge(&mut record, options_json(&options));
    let start = Instant::now();
    let outcome = run_arm(family, rtol, &options);
    let wall = start.elapsed().as_secs_f64();
    merge(&mut record, json!({"wall_seconds_diagnostic_only": wall}));
    match outcome {
        Ok(run) => {
            merge(&mut record, json!({"completed": true}));
            merge(&mut record, run_json(&run));
            println!("V17_RUN_JSON {record}");
            println!(
                "V17_STATE_JSON {}",
                json!({
                    "label": record["label"],
                    "family": family_id,
                    "rtol": config.rtol,
                    "arm": options.arm.label(),
                    "audit": options.audit,
                    "restart": options.gmres().restart,
                    "precond": record["precond"],
                    "kappa": record["kappa"],
                    "n": run.y_end.len(),
                    "state_sha256": record["state_sha256"],
                    "state_hex": hex(&run.y_end),
                })
            );
        }
        Err(error) => {
            merge(
                &mut record,
                json!({"completed": false, "error": error.to_string()}),
            );
            println!("V17_RUN_JSON {record}");
        }
    }
}

fn mode_timing(family_id: &str, family: &Family) {
    let rtol: f64 = number("V17_RTOL").expect("V17_RTOL");
    let rounds: usize = number("V17_ROUNDS").unwrap_or(9);
    let offset: usize = number("V17_ROUND_OFFSET").unwrap_or(0);
    let labels = variable("V17_ARMS").unwrap_or_else(|| "A,B,C".into());
    let arms: Vec<ArmOptions> = labels.split(',').map(arm_options).collect();
    let config = adaptive(family, rtol);
    let load_start = loadavg();
    let clock = Instant::now();
    for round in 0..rounds {
        let global_round = round + offset;
        for position in 0..arms.len() {
            let options = &arms[(global_round + position) % arms.len()];
            let start = Instant::now();
            let outcome = run_arm(family, rtol, options);
            let wall = start.elapsed().as_secs_f64();
            let mut record = header("TIMING", family_id, family);
            merge(
                &mut record,
                json!({"rtol": config.rtol, "atol": config.atol}),
            );
            merge(&mut record, options_json(options));
            merge(
                &mut record,
                json!({
                    "round": global_round,
                    "position": position,
                    "wall_seconds": wall,
                }),
            );
            match outcome {
                Ok(run) => merge(
                    &mut record,
                    json!({
                        "completed": true,
                        "success": run.success,
                        "attempts": run.attempts,
                        "jvp_calls": run.counters.jvp_calls,
                        "state_sha256": sha256(&run.y_end),
                    }),
                ),
                Err(error) => merge(
                    &mut record,
                    json!({"completed": false, "error": error.to_string()}),
                ),
            }
            println!("V17_TIMING_JSON {record}");
        }
    }
    let mut meta = header("TIMING", family_id, family);
    merge(
        &mut meta,
        json!({
            "rtol": config.rtol,
            "arms": labels,
            "rounds": rounds,
            "round_offset": offset,
            "loadavg_start": load_start,
            "loadavg_end": loadavg(),
            "process_seconds": clock.elapsed().as_secs_f64(),
        }),
    );
    println!("V17_TIMING_META_JSON {meta}");
}

fn mode_calibration(mode: &str, family_id: &str, family: &Family) {
    let calls: u64 = number("V17_CALLS").unwrap_or(20_000);
    let n = family.problem.dimension;
    let mut out = vec![0.0; n];
    let mut checksum = 0.0;
    let mut counters = WorkCounters::default();
    if mode == "CALIB_JVP" {
        let operator = family
            .problem
            .linearize_matrix_free(family.span.0, &family.y0)
            .expect("linearization");
        // A fixed direction; the next one is the scaled result, so that the
        // calls cannot be hoisted.
        let mut direction: Vec<f64> = (0..n).map(|i| 1.0 / (1.0 + i as f64)).collect();
        for _ in 0..calls {
            operator.apply(&direction, &mut out).expect("JVP");
            let scale = out.iter().fold(1.0_f64, |m, v| m.max(v.abs()));
            for (d, o) in direction.iter_mut().zip(&out) {
                *d = o / scale;
            }
            checksum += out[n / 2];
        }
    } else {
        let mut state = family.y0.clone();
        for call in 0..calls {
            family
                .problem
                .eval_rhs_into(family.span.0, &state, &mut out, &mut counters)
                .expect("right-hand side");
            // A tiny, bounded change of the state between calls.
            state[(call as usize) % n] += 1.0e-13 * out[(call as usize) % n].signum();
            checksum += out[n / 2];
        }
    }
    let mut record = header(mode, family_id, family);
    merge(
        &mut record,
        json!({"calls": calls, "checksum_bits": format!("{:016x}", checksum.to_bits())}),
    );
    println!("V17_CALIB_JSON {record}");
}

#[test]
#[ignore = "measurement program: run with --ignored --nocapture and the V17_* variables"]
fn v17_run() {
    let Some(mode) = variable("V17_MODE") else {
        println!("V17_USAGE set V17_MODE, V17_FAMILY, ...; see the file header");
        return;
    };
    let family_id = variable("V17_FAMILY").expect("V17_FAMILY");
    let family = declared(&family_id).expect("family");
    match mode.as_str() {
        "SETUP" => {
            let mut record = header("SETUP", &family_id, &family);
            merge(&mut record, json!({"setup_only": true}));
            println!("V17_RUN_JSON {record}");
        }
        "RUN" => mode_run(&family_id, &family),
        "TIMING" => mode_timing(&family_id, &family),
        "CALIB_JVP" | "CALIB_RHS" => mode_calibration(&mode, &family_id, &family),
        "RHS0" => {
            let mut f0 = vec![0.0; family.problem.dimension];
            family
                .problem
                .eval_rhs_into(
                    family.span.0,
                    &family.y0,
                    &mut f0,
                    &mut WorkCounters::default(),
                )
                .expect("right-hand side");
            let mut record = header("RHS0", &family_id, &family);
            merge(
                &mut record,
                json!({"y0_hex": hex(&family.y0), "f0_hex": hex(&f0)}),
            );
            println!("V17_RHS0_JSON {record}");
        }
        other => panic!("unknown V17_MODE {other}"),
    }
}
