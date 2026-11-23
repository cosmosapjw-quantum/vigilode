//! Measurement program of loop node V24 (adaptive matrix-free stage solve,
//! owner decision OD-L1-11).
//!
//! One process does one thing, chosen by `V24_MODE`, and prints
//! machine-readable lines. Nothing is compared or judged here;
//! `research/production_loop_20261116/mf_adaptive/parse_results.py` builds
//! the results from these lines.
//!
//! ```text
//! V24_MODE=RUN V24_FAMILY=fhn3d V24_RTOL=1e-5 V24_ARM=S V24_ENTRY=route \
//!   <test binary> --ignored --nocapture --exact v24_run
//! ```
//!
//! Modes:
//!
//! * `SETUP`: builds the family and its start state, nothing else (the
//!   instruction baseline every run is charged with).
//! * `RHS0`: prints the start state and `f(t0, y0)` as hex (`V24_RHS0_JSON`).
//! * `RUN`: one arm (`V24_ARM`) at `V24_RTOL` through one entry
//!   (`V24_ENTRY`). Prints `V24_RUN_JSON` and `V24_STATE_JSON` (end state,
//!   hex).
//! * `TIMING`: `V24_ROUNDS` rounds (default 9) of the arms `V24_ARMS`, each
//!   arm once per round in the order rotated by the round index plus
//!   `V24_ROUND_OFFSET`, through the entry `route`. Prints one
//!   `V24_TIMING_JSON` per run and a `V24_TIMING_META_JSON`.
//!
//! Families (`V24_FAMILY`): the deciding families `fhn3d`, `cusp1d`,
//! `burgers2d`, `bruss1d` (the declared instances of node V17) and the
//! development problems `dev06_bruss1d`, `dev06_adr2d` (the declared
//! instances of node V06, run with the integrator configuration of V17).
//!
//! Arms: `S` is the driver with the shipped stage-solve policy, `P1` and
//! `P2` the driver with the adaptive policies (`MfStageSolvePolicy`). The
//! exploratory arms `SR`, `P1R`, `P2R` are the same policies under
//! `ResidualAccounting::ReuseConfirmed` (no duplicate final residual).
//!
//! Entries:
//!
//! * `route`: the audit-off entry. Arm `S`:
//!   `integrate_rodas5p_mf_fast_observed_gmres_into`, what the router calls;
//!   `P1`, `P2`: `integrate_rodas5p_mf_fast_observed_with_stage_solve_policy`,
//!   what the router calls under an adaptive policy; the `R` arms: the
//!   workspace with that accounting, no log and no observer.
//! * `route_ls`: the same with the least-squares workspace of the facade
//!   (arm `S`: `integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace`).
//! * `policy_shipped` (arm `S` only): the policy entry with the shipped
//!   policy named explicitly.
//! * `audit`: the workspace with the solve log, the driver's stage-solve
//!   records and an attempt observer that checks every completed stage solve
//!   against the original stage residual (`ProtectedAudit` of node V17,
//!   unchanged). The driver's records are audited as well: the reported true
//!   residual of every returned iterate against the threshold it used.

#[path = "stage_clients_krylov_v17_problems.rs"]
mod problems;

use std::time::Instant;

use problems::{AuditSummary, Family, ProtectedAudit, adaptive, declared, v06};
use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::{
    MfStageSolvePolicy, MfStageSolveRecord, MfStageSolveStatistics, OutputSchedule,
    Rodas5pMfFastResult, Rodas5pMfFastWorkspace, RoutingOptions, StageSolveLogEntry,
    integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace,
    integrate_rodas5p_mf_fast_observed_with_stage_solve_policy,
};
use rodas5p_krylov::ResidualAccounting;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
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

fn digest_hex(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sha256(values: &[f64]) -> String {
    let mut hasher = Sha256::new();
    for value in values {
        hasher.update(value.to_le_bytes());
    }
    digest_hex(hasher)
}

/// On-CPU time of the calling thread in seconds (`/proc/thread-self/schedstat`).
fn thread_cpu_seconds() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    let nanoseconds: u64 = text.split_whitespace().next()?.parse().ok()?;
    Some(nanoseconds as f64 * 1.0e-9)
}

fn loadavg() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
}

/// The family `id`: a declared instance of node V17 or, with the prefix
/// `dev06_`, a declared instance of node V06.
fn family(id: &str) -> CoreResult<Family> {
    match id.strip_prefix("dev06_") {
        Some(base) => {
            let mut family = v06::declared(base)?;
            family.id = id.to_string();
            Ok(family)
        }
        None => declared(id),
    }
}

/// The arms of the node: a stage-solve policy and a residual accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Arm {
    label: &'static str,
    policy: MfStageSolvePolicy,
    accounting: ResidualAccounting,
}

impl Arm {
    fn parse(label: &str) -> Self {
        let (label, policy, accounting) = match label {
            "S" => (
                "S",
                MfStageSolvePolicy::Shipped,
                ResidualAccounting::DEFAULT,
            ),
            "P1" => (
                "P1",
                MfStageSolvePolicy::ColumnTested,
                ResidualAccounting::DEFAULT,
            ),
            "P2" => (
                "P2",
                MfStageSolvePolicy::ColumnTestedDoubling,
                ResidualAccounting::DEFAULT,
            ),
            "SR" => (
                "SR",
                MfStageSolvePolicy::Shipped,
                ResidualAccounting::ReuseConfirmed,
            ),
            "P1R" => (
                "P1R",
                MfStageSolvePolicy::ColumnTested,
                ResidualAccounting::ReuseConfirmed,
            ),
            "P2R" => (
                "P2R",
                MfStageSolvePolicy::ColumnTestedDoubling,
                ResidualAccounting::ReuseConfirmed,
            ),
            other => panic!("unknown arm {other}"),
        };
        Self {
            label,
            policy,
            accounting,
        }
    }

    /// The accounting of the route.
    fn default_accounting(self) -> bool {
        self.accounting == ResidualAccounting::DEFAULT
    }
}

/// How a run enters the driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Route,
    RouteLs,
    PolicyShipped,
    Audit,
}

impl Entry {
    fn parse(label: &str) -> Self {
        match label {
            "route" => Self::Route,
            "route_ls" => Self::RouteLs,
            "policy_shipped" => Self::PolicyShipped,
            "audit" => Self::Audit,
            other => panic!("unknown entry {other}"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::RouteLs => "route_ls",
            Self::PolicyShipped => "policy_shipped",
            Self::Audit => "audit",
        }
    }
}

/// The driver's solve log of one run: its length, the Arnoldi columns and a
/// hash of every entry.
struct SolveLogDigest {
    solves: u64,
    iterations: u64,
    max_iterations: u64,
    sha256: String,
}

fn solve_log_digest(log: &[StageSolveLogEntry]) -> SolveLogDigest {
    let mut hasher = Sha256::new();
    let (mut iterations, mut max_iterations) = (0, 0);
    for entry in log {
        hasher.update(entry.residual_norm_bits.to_le_bytes());
        hasher.update(entry.relative_residual_bits.to_le_bytes());
        hasher.update(entry.iterations.to_le_bytes());
        iterations += entry.iterations;
        max_iterations = max_iterations.max(entry.iterations);
    }
    SolveLogDigest {
        solves: log.len() as u64,
        iterations,
        max_iterations,
        sha256: digest_hex(hasher),
    }
}

/// The audit of the driver's own stage-solve records: for every returned
/// iterate the true residual norm the kernel reported against the threshold
/// it used.
#[derive(Clone, Copy, Debug, Default)]
struct RecordAudit {
    records: u64,
    /// Records whose reported residual is not finite or exceeds the
    /// threshold.
    violations: u64,
    max_ratio: f64,
    zero_threshold_records: u64,
    columns: u64,
    max_columns: u64,
    cycles: u64,
    max_cycles: u64,
    failed_confirmations: u64,
    /// Records whose projected residual (adaptive policies) exceeds the
    /// threshold although the solve ended: the last cycle ran to its bound
    /// and the true residual met the threshold all the same.
    projected_above_threshold: u64,
}

fn audit_records(records: &[MfStageSolveRecord]) -> RecordAudit {
    let mut audit = RecordAudit::default();
    for record in records {
        audit.records += 1;
        let passed = record.residual_norm.is_finite() && record.residual_norm <= record.threshold;
        if !passed {
            audit.violations += 1;
        }
        if record.threshold > 0.0 {
            let ratio = record.residual_norm / record.threshold;
            if ratio.is_finite() {
                audit.max_ratio = audit.max_ratio.max(ratio);
            }
        } else {
            audit.zero_threshold_records += 1;
        }
        audit.columns += record.columns;
        audit.max_columns = audit.max_columns.max(record.columns);
        audit.cycles += record.cycles;
        audit.max_cycles = audit.max_cycles.max(record.cycles);
        audit.failed_confirmations += record.failed_confirmations;
        if record.projected_residual > record.threshold {
            audit.projected_above_threshold += 1;
        }
    }
    audit
}

/// The result of one run.
struct CellRun {
    result: Rodas5pMfFastResult,
    audit: Option<AuditSummary>,
    solve_log: Option<SolveLogDigest>,
    record_audit: Option<RecordAudit>,
    statistics: Option<MfStageSolveStatistics>,
}

impl CellRun {
    fn plain(result: Rodas5pMfFastResult) -> Self {
        Self {
            result,
            audit: None,
            solve_log: None,
            record_audit: None,
            statistics: None,
        }
    }

    fn end_state(&self) -> &[f64] {
        self.result.observed.y.last().map_or(&[], Vec::as_slice)
    }
}

/// Run `arm` on `family` at `rtol` through `entry`, output schedule
/// `[t0, t1]`, linear configuration of the route.
fn run_cell(family: &Family, rtol: f64, arm: Arm, entry: Entry) -> CoreResult<CellRun> {
    let config = adaptive(family, rtol);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1])?;
    let linear = RoutingOptions::default().matrix_free;
    let shipped = arm.policy == MfStageSolvePolicy::Shipped;
    match entry {
        Entry::Route | Entry::RouteLs if arm.default_accounting() => {
            let ls_workspace = entry == Entry::RouteLs;
            let result = if shipped {
                let integrate = if ls_workspace {
                    integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace
                } else {
                    integrate_rodas5p_mf_fast_observed_gmres_into
                };
                integrate(
                    &family.problem,
                    family.span,
                    &family.y0,
                    &linear,
                    &config,
                    &output,
                )?
            } else {
                integrate_rodas5p_mf_fast_observed_with_stage_solve_policy(
                    &family.problem,
                    family.span,
                    &family.y0,
                    &linear,
                    &config,
                    &output,
                    arm.policy,
                    ls_workspace,
                )?
            };
            Ok(CellRun::plain(result))
        }
        Entry::Route => {
            // An accounting the entries do not take: the workspace, without
            // a log and without an observer.
            let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear)?;
            work.set_gmres_into(true);
            work.set_residual_accounting(arm.accounting);
            work.set_stage_solve_policy(arm.policy)?;
            let result = work.integrate_observed(
                &family.problem,
                family.span,
                &family.y0,
                &config,
                &output,
            )?;
            Ok(CellRun {
                statistics: Some(work.stage_solve_policy_statistics()),
                ..CellRun::plain(result)
            })
        }
        Entry::RouteLs => Err(CoreError::InvalidInput(
            "entry route_ls is declared for the default accounting only".into(),
        )),
        Entry::PolicyShipped => {
            if !(shipped && arm.default_accounting()) {
                return Err(CoreError::InvalidInput(
                    "entry policy_shipped is the shipped arm".into(),
                ));
            }
            Ok(CellRun::plain(
                integrate_rodas5p_mf_fast_observed_with_stage_solve_policy(
                    &family.problem,
                    family.span,
                    &family.y0,
                    &linear,
                    &config,
                    &output,
                    MfStageSolvePolicy::Shipped,
                    false,
                )?,
            ))
        }
        Entry::Audit => {
            // What the audit-off entry does, plus the solve log, the
            // stage-solve records and a non-empty observer.
            let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear)?;
            work.set_gmres_into(true);
            work.set_residual_accounting(arm.accounting);
            work.set_stage_solve_policy(arm.policy)?;
            work.set_solve_log(true);
            work.set_stage_solve_records(true);
            let mut audit =
                ProtectedAudit::new(family.problem.dimension, linear.rtol, linear.atol)?;
            let mut observer =
                |t: f64, y: &[f64], h: f64, _error: Option<f64>, work: &Rodas5pMfFastWorkspace| {
                    audit.observe(&family.problem, t, y, h, work);
                };
            let (result, _statistics, _charges, log) = work.integrate_observed_with_observer(
                &family.problem,
                family.span,
                &family.y0,
                &config,
                &output,
                &mut observer,
            )?;
            let log = log.ok_or_else(|| {
                CoreError::InvalidInput("the driver returned no solve log".into())
            })?;
            let records = work.stage_solve_records().ok_or_else(|| {
                CoreError::InvalidInput("the driver kept no stage-solve records".into())
            })?;
            Ok(CellRun {
                result,
                audit: Some(audit.summary),
                solve_log: Some(solve_log_digest(&log)),
                record_audit: Some(audit_records(records)),
                statistics: Some(work.stage_solve_policy_statistics()),
            })
        }
    }
}

fn counters_json(counters: &WorkCounters) -> Value {
    serde_json::to_value(counters).expect("counters")
}

fn run_json(run: &CellRun) -> Value {
    let result = &run.result;
    let t_end = result.observed.t.last().copied().unwrap_or(f64::NAN);
    let mut record = json!({
        "success": result.observed.success,
        "attempts": result.attempts,
        "accepted": result.accepted_steps,
        "rejected": result.rejected_steps,
        "state_reuses": result.state_reuses,
        "t_end": t_end,
        "t_end_bits": format!("{:016x}", t_end.to_bits()),
        "output_rows": result.observed.t.len(),
        "jvp_calls": result.observed.counters.jvp_calls,
        "counters": counters_json(&result.observed.counters),
        "state_sha256": sha256(run.end_state()),
        "driver": result.driver,
        "krylov_workspace": result.krylov_workspace,
    });
    let fields = record.as_object_mut().expect("object");
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
                "sha256": log.sha256,
            }),
        );
    }
    if let Some(audit) = &run.record_audit {
        fields.insert(
            "record_audit".into(),
            json!({
                "records": audit.records,
                "violations": audit.violations,
                "max_ratio": audit.max_ratio,
                "zero_threshold_records": audit.zero_threshold_records,
                "columns": audit.columns,
                "max_columns": audit.max_columns,
                "cycles": audit.cycles,
                "max_cycles": audit.max_cycles,
                "failed_confirmations": audit.failed_confirmations,
                "projected_above_threshold": audit.projected_above_threshold,
            }),
        );
    }
    if let Some(statistics) = &run.statistics {
        fields.insert(
            "policy_statistics".into(),
            serde_json::to_value(statistics).expect("statistics"),
        );
    }
    record
}

fn header(mode: &str, family_id: &str, family: &Family) -> Value {
    json!({
        "schema": "vigilode.loop-l1.v24.run.v1",
        "label": variable("V24_LABEL").unwrap_or_default(),
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
    let rtol: f64 = number("V24_RTOL").expect("V24_RTOL");
    let arm = Arm::parse(&variable("V24_ARM").expect("V24_ARM"));
    let entry = Entry::parse(&variable("V24_ENTRY").expect("V24_ENTRY"));
    let config = adaptive(family, rtol);
    let mut record = header("RUN", family_id, family);
    merge(
        &mut record,
        json!({
            "rtol": config.rtol,
            "atol": config.atol,
            "arm": arm.label,
            "policy": arm.policy.id(),
            "accounting": arm.accounting.id(),
            "entry": entry.label(),
            "audit": entry == Entry::Audit,
        }),
    );
    let start = Instant::now();
    let outcome = run_cell(family, rtol, arm, entry);
    let wall = start.elapsed().as_secs_f64();
    merge(&mut record, json!({"wall_seconds_diagnostic_only": wall}));
    match outcome {
        Ok(run) => {
            merge(&mut record, json!({"completed": true}));
            merge(&mut record, run_json(&run));
            println!("V24_RUN_JSON {record}");
            println!(
                "V24_STATE_JSON {}",
                json!({
                    "label": record["label"],
                    "family": family_id,
                    "rtol": config.rtol,
                    "arm": arm.label,
                    "entry": entry.label(),
                    "n": run.end_state().len(),
                    "state_sha256": record["state_sha256"],
                    "state_hex": hex(run.end_state()),
                })
            );
        }
        Err(error) => {
            merge(
                &mut record,
                json!({"completed": false, "error": error.to_string()}),
            );
            println!("V24_RUN_JSON {record}");
        }
    }
}

fn mode_timing(family_id: &str, family: &Family) {
    let rtol: f64 = number("V24_RTOL").expect("V24_RTOL");
    let rounds: usize = number("V24_ROUNDS").unwrap_or(9);
    let offset: usize = number("V24_ROUND_OFFSET").unwrap_or(0);
    let labels = variable("V24_ARMS").expect("V24_ARMS");
    let arms: Vec<Arm> = labels.split(',').map(Arm::parse).collect();
    let config = adaptive(family, rtol);
    let load_start = loadavg();
    let clock = Instant::now();
    for round in 0..rounds {
        let global_round = round + offset;
        for position in 0..arms.len() {
            let arm = arms[(global_round + position) % arms.len()];
            let cpu_start = thread_cpu_seconds();
            let start = Instant::now();
            let outcome = run_cell(family, rtol, arm, Entry::Route);
            let wall = start.elapsed().as_secs_f64();
            let cpu = cpu_start
                .zip(thread_cpu_seconds())
                .map(|(before, after)| after - before);
            let mut record = header("TIMING", family_id, family);
            merge(
                &mut record,
                json!({
                    "rtol": config.rtol,
                    "atol": config.atol,
                    "arm": arm.label,
                    "entry": Entry::Route.label(),
                    "round": global_round,
                    "position": position,
                    "wall_seconds": wall,
                    "thread_cpu_seconds": cpu,
                }),
            );
            match outcome {
                Ok(run) => merge(
                    &mut record,
                    json!({
                        "completed": true,
                        "success": run.result.observed.success,
                        "attempts": run.result.attempts,
                        "jvp_calls": run.result.observed.counters.jvp_calls,
                        "state_sha256": sha256(run.end_state()),
                    }),
                ),
                Err(error) => merge(
                    &mut record,
                    json!({"completed": false, "error": error.to_string()}),
                ),
            }
            println!("V24_TIMING_JSON {record}");
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
    println!("V24_TIMING_META_JSON {meta}");
}

#[test]
#[ignore = "measurement program: run with --ignored --nocapture and the V24_* variables"]
fn v24_run() {
    let Some(mode) = variable("V24_MODE") else {
        println!("V24_USAGE set V24_MODE, V24_FAMILY, ...; see the file header");
        return;
    };
    let family_id = variable("V24_FAMILY").expect("V24_FAMILY");
    let family = family(&family_id).expect("family");
    match mode.as_str() {
        "SETUP" => {
            let mut record = header("SETUP", &family_id, &family);
            merge(&mut record, json!({"setup_only": true}));
            println!("V24_RUN_JSON {record}");
        }
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
            println!("V24_RHS0_JSON {record}");
        }
        "RUN" => mode_run(&family_id, &family),
        "TIMING" => mode_timing(&family_id, &family),
        other => panic!("unknown V24_MODE {other}"),
    }
}
