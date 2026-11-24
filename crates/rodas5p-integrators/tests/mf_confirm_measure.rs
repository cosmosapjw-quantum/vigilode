//! Measurement program of loop node V25 (confirmation of the adaptive
//! matrix-free stage solve P1 on unseen families, owner decision OD-L1-15).
//!
//! One process does one thing, chosen by `V25_MODE`, and prints
//! machine-readable lines. Nothing is compared or judged here;
//! `research/production_loop_20261116/mf_p1_confirm/parse_results.py` builds
//! the results from these lines.
//!
//! ```text
//! V25_MODE=RUN V25_FAMILY=ac3d V25_T1=1 V25_RTOL=1e-5 V25_ARM=P1 V25_ENTRY=route \
//!   <test binary> --ignored --nocapture --exact v25_run
//! ```
//!
//! Modes:
//!
//! * `SETUP`: builds the family and its start state, nothing else (the
//!   instruction baseline every run is charged with).
//! * `RHS0`: prints the start state and `f(t0, y0)` as hex (`V25_RHS0_JSON`).
//! * `CONSIST`: the frozen consistency check of the right-hand sides: writes
//!   the 20 deterministic states and directions with `f(y)` and `J(y) v`
//!   as hex to the file `V25_CONSIST_OUT`, one line per state, and prints a
//!   summary (`V25_CONSIST_JSON`).
//! * `RUN`: one arm (`V25_ARM`) at `V25_RTOL` through one entry
//!   (`V25_ENTRY`) over `[t0, V25_T1]`. Prints `V25_RUN_JSON` and
//!   `V25_STATE_JSON` (end state, hex).
//! * `TIMING`: `V25_ROUNDS` rounds (default 9) of the arms `V25_ARMS`, each
//!   arm once per round in the order rotated by the round index plus
//!   `V25_ROUND_OFFSET`, through the entry `route`. Prints one
//!   `V25_TIMING_JSON` per run and a `V25_TIMING_META_JSON`.
//!
//! Families (`V25_FAMILY`): `ac3d`, `robertson_rd1d`, `schnakenberg2d`,
//! `porous2d` (`mf_confirm_problems.rs`); the end time is `V25_T1`
//! (required in every mode that integrates).
//!
//! Arms: `S` is the driver with the shipped stage-solve policy, `P1` the
//! adaptive policy `MfStageSolvePolicy::ColumnTested`, `P2` the opt-in
//! policy `MfStageSolvePolicy::ColumnTestedDoubling`; each is named
//! explicitly in every entry, never taken from a default.
//!
//! Entries:
//!
//! * `route`: the audit-off entry of the instruction runs. Arm `S`:
//!   `integrate_rodas5p_mf_fast_observed_gmres_into` (the shipped entry);
//!   `P1`, `P2`: `integrate_rodas5p_mf_fast_observed_with_stage_solve_policy`
//!   with the policy named.
//! * `router`: `integrate_rodas5p_routed_observed_with_options` with
//!   `RoutingOptions::matrix_free_policy` set to the arm's policy.
//! * `router_ls`: the same with `krylov_ls_workspace` on (the facade's
//!   call, with the policy named).
//! * `audit`: the workspace with the solve log, the driver's stage-solve
//!   records and an attempt observer that checks every completed stage
//!   solve against the original stage residual twice: `ProtectedAudit` of
//!   node V17 (unchanged, a summary) and the per-solve recorder of this file,
//!   which writes every completed solve (independent residual and threshold,
//!   the driver's reported residual and threshold) to the file
//!   `V25_SOLVELOG`, so that discriminator V can be recounted from the log.

#[path = "mf_confirm_problems.rs"]
mod problems;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

use problems::{
    AuditSummary, CONSISTENCY_STATES, Family, ProtectedAudit, adaptive, consistency_state,
    declared, with_end,
};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, DenseMatrix, LinearOperator, WorkCounters, apply_counted,
    rodas5p_coefficients, safe_l2,
};
use rodas5p_integrators::{
    MfStageSolvePolicy, MfStageSolveRecord, MfStageSolveStatistics, OdeProblem, OutputSchedule,
    Rodas5pMfFastResult, Rodas5pMfFastWorkspace, RoutedRun, RoutingOptions, StageSolveLogEntry,
    integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_with_stage_solve_policy,
    integrate_rodas5p_routed_observed_with_options, raw_absolute_residual_budget,
};
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

fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
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

/// The family `id` over `[t0, V25_T1]`.
fn family(id: &str) -> CoreResult<Family> {
    let family = declared(id)?;
    match number::<f64>("V25_T1") {
        Some(t1) => with_end(family, t1),
        None => Ok(family),
    }
}

/// The arms of the node: a stage-solve policy, always named explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Arm {
    label: &'static str,
    policy: MfStageSolvePolicy,
}

impl Arm {
    fn parse(label: &str) -> Self {
        let (label, policy) = match label {
            "S" => ("S", MfStageSolvePolicy::Shipped),
            "P1" => ("P1", MfStageSolvePolicy::ColumnTested),
            "P2" => ("P2", MfStageSolvePolicy::ColumnTestedDoubling),
            other => panic!("unknown arm {other}"),
        };
        Self { label, policy }
    }
}

/// How a run enters the driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Route,
    Router,
    RouterLs,
    Audit,
}

impl Entry {
    fn parse(label: &str) -> Self {
        match label {
            "route" => Self::Route,
            "router" => Self::Router,
            "router_ls" => Self::RouterLs,
            "audit" => Self::Audit,
            other => panic!("unknown entry {other}"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::Router => "router",
            Self::RouterLs => "router_ls",
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

/// The per-solve recorder of the audit entry. In the attempt observer it
/// rebuilds every stage system the driver completed from the stage
/// equations of the driver's U form (as `ProtectedAudit` of node V17 does,
/// with its own buffers and counters),
///
/// ```text
/// Y_i   = y + sum_j a_ij U_j
/// rhs_i = h gamma f(t + c_i h, Y_i) + sum_j (gamma C_ij) U_j
/// r_i   = rhs_i - (U_i - h gamma J(t, y) U_i)
/// ```
///
/// and writes one line per completed solve: the attempt, the stage,
/// `||r_i||_2` and `max(gamma atol_lin, rtol_lin ||rhs_i||_2)` as bits,
/// and the driver's own record of the solve. One line per attempt
/// (`A ...`) records how many solves it completed and whether it produced
/// an error estimate. Autonomous problems only.
struct SolveRecorder {
    gamma: f64,
    a: DenseMatrix,
    c_matrix: DenseMatrix,
    c: Vec<f64>,
    rtol_lin: f64,
    atol_scaled: f64,
    state: Vec<f64>,
    rhs: Vec<f64>,
    jv: Vec<f64>,
    residual: Vec<f64>,
    counters: WorkCounters,
    logged: usize,
    recorded: usize,
    attempt: u64,
    writer: Option<BufWriter<File>>,
    lines: u64,
    checks: u64,
    failures: u64,
    max_ratio: f64,
    unmatched_records: u64,
}

impl SolveRecorder {
    fn new(n: usize, rtol_lin: f64, atol_lin: f64, path: Option<String>) -> CoreResult<Self> {
        let coeffs = rodas5p_coefficients()?;
        let atol_scaled = if atol_lin == 0.0 {
            0.0
        } else {
            raw_absolute_residual_budget(atol_lin, coeffs.gamma)?
        };
        let writer = match path {
            Some(path) => Some(BufWriter::new(File::create(&path).map_err(|error| {
                CoreError::InvalidInput(format!("cannot create {path}: {error}"))
            })?)),
            None => None,
        };
        Ok(Self {
            gamma: coeffs.gamma,
            a: coeffs.a.clone(),
            c_matrix: coeffs.c_matrix.clone(),
            c: coeffs.c.clone(),
            rtol_lin,
            atol_scaled,
            state: vec![0.0; n],
            rhs: vec![0.0; n],
            jv: vec![0.0; n],
            residual: vec![0.0; n],
            counters: WorkCounters::default(),
            logged: 0,
            recorded: 0,
            attempt: 0,
            writer,
            lines: 0,
            checks: 0,
            failures: 0,
            max_ratio: 0.0,
            unmatched_records: 0,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn stage_residual(
        &mut self,
        problem: &OdeProblem,
        jacobian: &dyn LinearOperator,
        t: f64,
        y: &[f64],
        h: f64,
        i: usize,
        work: &Rodas5pMfFastWorkspace,
    ) -> CoreResult<(f64, f64)> {
        let hg = h * self.gamma;
        if i == 0 {
            problem.eval_rhs_into(t, y, &mut self.rhs, &mut self.counters)?;
        } else {
            self.state.copy_from_slice(y);
            for j in 0..i {
                let aij = self.a[(i, j)];
                if aij != 0.0 {
                    for (x, v) in self.state.iter_mut().zip(work.stage(j)) {
                        *x += aij * v;
                    }
                }
            }
            problem.eval_rhs_into(
                t + self.c[i] * h,
                &self.state,
                &mut self.rhs,
                &mut self.counters,
            )?;
        }
        for x in self.rhs.iter_mut() {
            *x *= hg;
        }
        for j in 0..i {
            let cij = self.c_matrix[(i, j)];
            if cij != 0.0 {
                let weight = self.gamma * cij;
                for (x, v) in self.rhs.iter_mut().zip(work.stage(j)) {
                    *x += weight * v;
                }
            }
        }
        let u = work.stage(i);
        apply_counted(
            jacobian,
            u,
            &mut self.jv,
            &mut self.counters,
            ApplyCategory::Diagnostic,
        )?;
        for (((r, b), x), jx) in self.residual.iter_mut().zip(&self.rhs).zip(u).zip(&self.jv) {
            *r = b - (x - hg * jx);
        }
        let threshold = self.atol_scaled.max(self.rtol_lin * safe_l2(&self.rhs));
        Ok((safe_l2(&self.residual), threshold))
    }

    fn write(&mut self, line: &str) {
        if let Some(writer) = self.writer.as_mut() {
            writer
                .write_all(line.as_bytes())
                .expect("the solve log could not be written");
            self.lines += 1;
        }
    }

    fn observe(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        error: Option<f64>,
        work: &Rodas5pMfFastWorkspace,
    ) {
        let log = work.solve_log().unwrap_or_default();
        let records = work.stage_solve_records().unwrap_or_default();
        let completed = log.len().saturating_sub(self.logged);
        let first_entry = self.logged;
        let first_record = self.recorded;
        self.logged = log.len();
        self.recorded = records.len();
        let attempt = self.attempt;
        self.attempt += 1;
        if records.len().saturating_sub(first_record) != completed {
            self.unmatched_records += 1;
        }
        self.write(&format!(
            "A {attempt} {} {} {completed} {}\n",
            bits(t),
            bits(h),
            error.map_or_else(|| "-".to_string(), bits)
        ));
        if completed == 0 {
            return;
        }
        let jacobian = problem.linearize_matrix_free(t, y);
        for i in 0..completed {
            self.checks += 1;
            let verdict = match &jacobian {
                Ok(jacobian) if problem.autonomous && i < self.c.len() => {
                    self.stage_residual(problem, jacobian.as_ref(), t, y, h, i, work)
                }
                _ => Err(CoreError::InvalidInput(
                    "the stage system could not be rebuilt".into(),
                )),
            };
            let (residual_norm, threshold) = verdict.unwrap_or((f64::NAN, f64::NAN));
            let passed = residual_norm.is_finite() && residual_norm <= threshold;
            if !passed {
                self.failures += 1;
            }
            if threshold > 0.0 {
                let ratio = residual_norm / threshold;
                if ratio.is_finite() {
                    self.max_ratio = self.max_ratio.max(ratio);
                }
            }
            let entry = &log[first_entry + i];
            let line = match records.get(first_record + i) {
                Some(record) => format!(
                    "S {attempt} {i} {} {} {} {} {} {} {} {} {} {}\n",
                    bits(residual_norm),
                    bits(threshold),
                    bits(record.residual_norm),
                    bits(record.threshold),
                    bits(record.right_norm),
                    bits(record.projected_residual),
                    record.columns,
                    record.cycles,
                    record.failed_confirmations,
                    entry.iterations,
                ),
                None => format!(
                    "S {attempt} {i} {} {} - - - - - - - {}\n",
                    bits(residual_norm),
                    bits(threshold),
                    entry.iterations,
                ),
            };
            self.write(&line);
        }
    }

    fn finish(&mut self) -> CoreResult<()> {
        if let Some(writer) = self.writer.as_mut() {
            writer.flush().map_err(|error| {
                CoreError::InvalidInput(format!("the solve log could not be flushed: {error}"))
            })?;
        }
        Ok(())
    }
}

/// The result of one run.
struct CellRun {
    result: Rodas5pMfFastResult,
    routed_driver_id: Option<&'static str>,
    audit: Option<AuditSummary>,
    recorder: Option<Value>,
    solve_log: Option<SolveLogDigest>,
    record_audit: Option<RecordAudit>,
    statistics: Option<MfStageSolveStatistics>,
}

impl CellRun {
    fn plain(result: Rodas5pMfFastResult) -> Self {
        Self {
            result,
            routed_driver_id: None,
            audit: None,
            recorder: None,
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
    match entry {
        Entry::Route => {
            let result = if arm.policy == MfStageSolvePolicy::Shipped {
                integrate_rodas5p_mf_fast_observed_gmres_into(
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
                    false,
                )?
            };
            Ok(CellRun::plain(result))
        }
        Entry::Router | Entry::RouterLs => {
            let options = RoutingOptions {
                matrix_free: linear,
                krylov_ls_workspace: entry == Entry::RouterLs,
                matrix_free_policy: arm.policy,
            };
            let routed = integrate_rodas5p_routed_observed_with_options(
                &family.problem,
                family.span,
                &family.y0,
                &config,
                &output,
                &options,
            )
            .map_err(|error| CoreError::InvalidInput(format!("router: {error}")))?;
            let driver_id = routed.routing.driver_id;
            let RoutedRun::MatrixFree(result) = routed.run else {
                return Err(CoreError::InvalidInput(
                    "the router did not take the matrix-free route".into(),
                ));
            };
            Ok(CellRun {
                routed_driver_id: Some(driver_id),
                ..CellRun::plain(result)
            })
        }
        Entry::Audit => {
            // What the audit-off entries do, plus the solve log, the
            // stage-solve records and a non-empty observer.
            let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear)?;
            work.set_gmres_into(true);
            work.set_stage_solve_policy(arm.policy)?;
            work.set_solve_log(true);
            work.set_stage_solve_records(true);
            let mut audit =
                ProtectedAudit::new(family.problem.dimension, linear.rtol, linear.atol)?;
            let mut recorder = SolveRecorder::new(
                family.problem.dimension,
                linear.rtol,
                linear.atol,
                variable("V25_SOLVELOG"),
            )?;
            let mut observer =
                |t: f64, y: &[f64], h: f64, error: Option<f64>, work: &Rodas5pMfFastWorkspace| {
                    audit.observe(&family.problem, t, y, h, work);
                    recorder.observe(&family.problem, t, y, h, error, work);
                };
            let (result, _statistics, _charges, log) = work.integrate_observed_with_observer(
                &family.problem,
                family.span,
                &family.y0,
                &config,
                &output,
                &mut observer,
            )?;
            recorder.finish()?;
            let log = log.ok_or_else(|| {
                CoreError::InvalidInput("the driver returned no solve log".into())
            })?;
            let records = work.stage_solve_records().ok_or_else(|| {
                CoreError::InvalidInput("the driver kept no stage-solve records".into())
            })?;
            let recorder_summary = json!({
                "attempts_observed": recorder.attempt,
                "checks": recorder.checks,
                "failures": recorder.failures,
                "max_ratio": recorder.max_ratio,
                "unmatched_records": recorder.unmatched_records,
                "lines_written": recorder.lines,
                "file": variable("V25_SOLVELOG"),
                "jvp_calls": recorder.counters.jvp_calls,
                "rhs_calls": recorder.counters.rhs_calls,
            });
            Ok(CellRun {
                result,
                routed_driver_id: None,
                audit: Some(audit.summary),
                recorder: Some(recorder_summary),
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
        "t_end_bits": bits(t_end),
        "output_rows": result.observed.t.len(),
        "jvp_calls": result.observed.counters.jvp_calls,
        "counters": counters_json(&result.observed.counters),
        "state_sha256": sha256(run.end_state()),
        "driver": result.driver,
        "krylov_workspace": result.krylov_workspace,
    });
    let fields = record.as_object_mut().expect("object");
    if let Some(driver_id) = run.routed_driver_id {
        fields.insert("routed_driver_id".into(), json!(driver_id));
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
    if let Some(recorder) = &run.recorder {
        fields.insert("solve_recorder".into(), recorder.clone());
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
        "schema": "vigilode.loop-l1.v25.run.v1",
        "label": variable("V25_LABEL").unwrap_or_default(),
        "mode": mode,
        "family": family_id,
        "n": family.problem.dimension,
        "t_span": [family.span.0, family.span.1],
        "t_span_bits": [bits(family.span.0), bits(family.span.1)],
    })
}

fn merge(target: &mut Value, source: Value) {
    let fields = target.as_object_mut().expect("object");
    for (key, value) in source.as_object().expect("object") {
        fields.insert(key.clone(), value.clone());
    }
}

fn mode_run(family_id: &str, family: &Family) {
    let rtol: f64 = number("V25_RTOL").expect("V25_RTOL");
    let arm = Arm::parse(&variable("V25_ARM").expect("V25_ARM"));
    let entry = Entry::parse(&variable("V25_ENTRY").expect("V25_ENTRY"));
    let config = adaptive(family, rtol);
    let mut record = header("RUN", family_id, family);
    merge(
        &mut record,
        json!({
            "rtol": config.rtol,
            "atol": config.atol,
            "arm": arm.label,
            "policy": arm.policy.id(),
            "policy_driver_id": arm.policy.driver_id(),
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
            println!("V25_RUN_JSON {record}");
            println!(
                "V25_STATE_JSON {}",
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
            println!("V25_RUN_JSON {record}");
        }
    }
}

fn mode_timing(family_id: &str, family: &Family) {
    let rtol: f64 = number("V25_RTOL").expect("V25_RTOL");
    let rounds: usize = number("V25_ROUNDS").unwrap_or(9);
    let offset: usize = number("V25_ROUND_OFFSET").unwrap_or(0);
    let labels = variable("V25_ARMS").expect("V25_ARMS");
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
            println!("V25_TIMING_JSON {record}");
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
    println!("V25_TIMING_META_JSON {meta}");
}

fn mode_consist(family_id: &str, family: &Family) {
    let path = variable("V25_CONSIST_OUT").expect("V25_CONSIST_OUT");
    let n = family.problem.dimension;
    let jvp = family.problem.jvp_callback().expect("a JVP-only family");
    let mut writer = BufWriter::new(File::create(&path).expect("consistency output"));
    let mut f = vec![0.0; n];
    let mut jv = vec![0.0; n];
    let mut hasher = Sha256::new();
    for k in 0..CONSISTENCY_STATES {
        let (y, v) = consistency_state(family, k);
        family
            .problem
            .eval_rhs_into(family.span.0, &y, &mut f, &mut WorkCounters::default())
            .expect("right-hand side");
        jvp(family.span.0, &y, &v, &mut jv).expect("Jacobian-vector product");
        let line = format!("{k} {} {} {} {}\n", hex(&y), hex(&v), hex(&f), hex(&jv));
        hasher.update(line.as_bytes());
        writer.write_all(line.as_bytes()).expect("write");
    }
    writer.flush().expect("flush");
    let mut record = header("CONSIST", family_id, family);
    merge(
        &mut record,
        json!({
            "states": CONSISTENCY_STATES,
            "seed": format!("{:#x}", problems::CONSISTENCY_SEED),
            "file": path,
            "content_sha256": digest_hex(hasher),
            "line_format": "k y_hex v_hex f(t0, y)_hex J(t0, y) v_hex",
        }),
    );
    println!("V25_CONSIST_JSON {record}");
}

#[test]
#[ignore = "measurement program: run with --ignored --nocapture and the V25_* variables"]
fn v25_run() {
    let Some(mode) = variable("V25_MODE") else {
        println!("V25_USAGE set V25_MODE, V25_FAMILY, V25_T1, ...; see the file header");
        return;
    };
    let family_id = variable("V25_FAMILY").expect("V25_FAMILY");
    let family = family(&family_id).expect("family");
    match mode.as_str() {
        "SETUP" => {
            let mut record = header("SETUP", &family_id, &family);
            merge(&mut record, json!({"setup_only": true}));
            println!("V25_RUN_JSON {record}");
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
            println!("V25_RHS0_JSON {record}");
        }
        "CONSIST" => mode_consist(&family_id, &family),
        "RUN" => mode_run(&family_id, &family),
        "TIMING" => mode_timing(&family_id, &family),
        other => panic!("unknown V25_MODE {other}"),
    }
}
