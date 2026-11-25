//! Phase times of the dense LU arms (loop L1 node V18, exploratory).
//!
//! Every client arm is wrapped in the client kit's `TimedStageClient`, which
//! times the three calls of a stage client (linearization, factorization,
//! stage solves). The rest of a run (right-hand sides, stage arithmetic,
//! controller, construction and release of the client) is the wall time
//! minus the three phases. The arm `kit` is the client-kit faer client: the
//! linear algebra, the allocations and the bits of the protected driver's
//! faer path. Output: one JSON object per line, in the format of the
//! `measure` command of the main runner (`type` = header | run | footer).

#[path = "../../../../../../crates/rodas5p-integrators/tests/stage_clients_dense_lu_reuse_problems.rs"]
mod problems;

use std::io::Write as _;
use std::sync::Arc;
use std::time::Instant;

use rodas5p_integrators::stage_clients::dense_lu_reuse::DenseLuReuseStageClient;
use rodas5p_integrators::stage_clients::{DenseLuMode, DenseLuStageClient, TimedStageClient};
use rodas5p_integrators::{
    OutputSchedule, StageLinearSystem, integrate_rodas5p_fast_client_observed_with_atol,
};
use serde_json::{Value, json};

use problems::{Arm, Cell, SkewnetData, adaptive, fingerprint, meanfield, skewnet};

fn emit(value: Value) {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{value}").expect("stdout");
    out.flush().expect("stdout");
}

fn fail(message: &str) -> ! {
    eprintln!("v18-dense-lu-phases: {message}");
    std::process::exit(2);
}

fn loadavg() -> Value {
    let text = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let values: Vec<f64> = text
        .split_whitespace()
        .take(3)
        .filter_map(|v| v.parse().ok())
        .collect();
    json!(values)
}

fn cpus_allowed() -> String {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

fn build(id: &str, t_end: f64) -> Cell {
    let (family, order) = id
        .split_once(".n")
        .unwrap_or_else(|| fail("a cell is <family>.n<order>"));
    let n: usize = order
        .parse()
        .unwrap_or_else(|_| fail("the order of a cell is an integer"));
    match family {
        "meanfield" => meanfield(n, t_end, None),
        "skewnet" => skewnet(&Arc::new(SkewnetData::new(n)), t_end, None),
        _ => fail("the families are meanfield and skewnet"),
    }
}

/// One run of a client arm under the timing wrapper.
fn timed_run(cell: &Cell, arm: Arm, rtol: f64) -> Value {
    let n = cell.n;
    let config = adaptive(cell, rtol, 1_000_000);
    let output = OutputSchedule::new(vec![cell.span.0, cell.span.1])
        .unwrap_or_else(|error| fail(&format!("output schedule: {error}")));
    let atol = vec![config.atol; n];
    let start = Instant::now();
    let reuse = |workers: usize| {
        DenseLuReuseStageClient::new(n, DenseLuMode::Faer)
            .and_then(|client| client.with_factor_workers(workers))
            .unwrap_or_else(|error| fail(&format!("client: {error}")))
    };
    let (client, handle): (Box<dyn StageLinearSystem>, _) = match arm {
        Arm::Kit => {
            let inner = DenseLuStageClient::new(n, DenseLuMode::Faer)
                .unwrap_or_else(|error| fail(&format!("client: {error}")));
            let timed = TimedStageClient::new(inner);
            let handle = timed.handle();
            (Box::new(timed), handle)
        }
        Arm::Reuse => {
            let timed = TimedStageClient::new(reuse(1));
            let handle = timed.handle();
            (Box::new(timed), handle)
        }
        Arm::Threaded(workers) => {
            let timed = TimedStageClient::new(reuse(workers));
            let handle = timed.handle();
            (Box::new(timed), handle)
        }
        Arm::Protected => fail("the protected driver has no client to time: use kit"),
    };
    let (result, partial, client) = integrate_rodas5p_fast_client_observed_with_atol(
        &cell.problem,
        client,
        cell.span,
        &cell.y0,
        &config,
        &output,
        &atol,
    )
    .unwrap_or_else(|error| fail(&format!("arm {} failed: {error}", arm.name())));
    drop(client);
    let wall_ns = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let times = handle.snapshot();
    let end = result.observed.y.last().expect("an output state");
    json!({
        "cell": cell.id,
        "arm": arm.name(),
        "wall_ns": wall_ns,
        "attempts": result.attempts,
        "accepted_steps": result.accepted_steps,
        "complete": partial.is_none() && result.observed.success,
        "end_state_fnv1a64": fingerprint(cell.span.1, end),
        "linearize_ns": times.linearize_ns,
        "factor_ns": times.factor_ns,
        "solve_ns": times.solve_ns,
        "linearize_calls": times.linearize_calls,
        "factor_calls": times.factor_calls,
        "solve_calls": times.solve_calls,
    })
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let mut cell_id = String::new();
    let (mut t_end, mut rtol) = (f64::NAN, f64::NAN);
    let (mut warmups, mut reps) = (2_usize, 7_usize);
    let mut arms = vec![
        Arm::Kit,
        Arm::Reuse,
        Arm::Threaded(2),
        Arm::Threaded(4),
        Arm::Threaded(6),
    ];
    let mut iter = argv[1..].iter();
    while let Some(flag) = iter.next() {
        let mut value = || {
            iter.next()
                .unwrap_or_else(|| fail(&format!("{flag} needs a value")))
                .as_str()
        };
        match flag.as_str() {
            "--cell" => cell_id = value().to_string(),
            "--t-end" => t_end = value().parse().unwrap_or_else(|_| fail("--t-end")),
            "--rtol" => rtol = value().parse().unwrap_or_else(|_| fail("--rtol")),
            "--warmups" => warmups = value().parse().unwrap_or_else(|_| fail("--warmups")),
            "--reps" => reps = value().parse().unwrap_or_else(|_| fail("--reps")),
            "--arms" => {
                arms = value()
                    .split(',')
                    .map(|name| {
                        Arm::parse(name).unwrap_or_else(|| fail(&format!("unknown arm {name}")))
                    })
                    .collect();
            }
            other => fail(&format!("unknown flag {other}")),
        }
    }
    if cell_id.is_empty() || !t_end.is_finite() || !rtol.is_finite() {
        fail("--cell, --t-end and --rtol are required");
    }
    let cell = build(&cell_id, t_end);
    emit(json!({
        "type": "header",
        "mode": "phases",
        "argv": argv,
        "argv0_length": argv[0].len(),
        "cpus_allowed_list": cpus_allowed(),
        "loadavg_before": loadavg(),
        "cell": {
            "id": cell.id,
            "family": cell.family,
            "n": cell.n,
            "t_span": [cell.span.0, cell.span.1],
            "rtol": rtol,
            "max_attempts": 1_000_000,
        },
        "protocol": {
            "arms": arms.iter().map(|arm| arm.name()).collect::<Vec<_>>(),
            "warmups": warmups,
            "repetitions": reps,
            "arm_order": "rotated by one position per repetition",
        },
    }));
    let started = Instant::now();
    let count = arms.len();
    for rep in 0..warmups + reps {
        for position in 0..count {
            let arm = arms[(rep + position) % count];
            let mut row = timed_run(&cell, arm, rtol);
            row["type"] = json!("run");
            row["rep"] = json!(rep);
            row["kind"] = json!(if rep < warmups { "warmup" } else { "timed" });
            row["position"] = json!(position);
            emit(row);
        }
    }
    emit(json!({
        "type": "footer",
        "loadavg_after": loadavg(),
        "elapsed_s": started.elapsed().as_secs_f64(),
    }));
}
