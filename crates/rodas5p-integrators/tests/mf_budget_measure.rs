//! Measurement program of loop node V28 (rule B of the adaptive matrix-free
//! stage solve P1 tested alone; owner decision OD-L1-23, addendum A9 of
//! `research/production_loop_20261116/PREREGISTRATION.md`, frozen design
//! `research/production_loop_20261116/mf_p1_budget/PREDECLARED.json`).
//!
//! One process does one thing, chosen by `V28_MODE`, and prints
//! machine-readable lines. Nothing is compared or judged here;
//! `research/production_loop_20261116/mf_p1_budget/parse_results.py` builds
//! the results from these lines.
//!
//! ```text
//! V28_MODE=RUN V28_CELL=5 V28_ARM=P1 V28_J=3 \
//!   <test binary> --ignored --nocapture --exact v28_run
//! ```
//!
//! Cells (`V28_CELL`, 0 to 39, [`Cell::of`]): the 20 deciding cells of node
//! V25 (`mf_p1_confirm/PROTOCOL.json`: ac3d, robertson_rd1d, schnakenberg2d,
//! porous2d with their final spans) and the 20 deciding cells of node V24
//! (`mf_adaptive/PROTOCOL.json` deciding_grid: fhn3d, cusp1d, burgers2d,
//! bruss1d, the declared instances of node V17), five tolerances each,
//! `rtol = atol` from 1e-3 to 1e-7. The generators are those of the two
//! nodes, included unchanged: `mf_confirm_problems.rs` (V25), which includes
//! `stage_clients_krylov_v17_problems.rs` (V17/V24) as `problems::v17`. The
//! integrator configuration is `problems::adaptive` and the linear
//! configuration `RoutingOptions::default().matrix_free` for every cell.
//!
//! Arms: `S` is the driver with the shipped stage-solve policy
//! (`MfStageSolvePolicy::Shipped`), `P1` the adaptive policy
//! `MfStageSolvePolicy::ColumnTested`; each is named explicitly, never taken
//! from a default.
//!
//! Modes:
//!
//! * `CELLS`: prints the table of the 40 cells (`V28_CELL_JSON`, one line per
//!   cell: family, size, span, tolerances, start digest). Builds the
//!   generators, integrates nothing.
//! * `SETUP`: builds the cell `V28_CELL`, nothing else.
//! * `RUN`: one arm (`V28_ARM`) of the cell from the start perturbed with
//!   trajectory `V28_J` ([`perturbed_start`]; `0` is the unperturbed start)
//!   through the workspace configured as the policy entry (GMRES into, the
//!   policy named, no observer). Prints `V28_RUN_JSON` (counters, the
//!   driver's `MfStageSolveStatistics` with the budget-exhausting solves,
//!   the digests of the start and of the end state) and `V28_STATE_JSON`
//!   (end state, hex).
//! * `SHADOW`: one arm of the cell from the unperturbed start through the
//!   workspace with the solve log and an attempt observer that rebuilds
//!   every stage system of the run (the completed solves and the solve in
//!   which an attempt failed) and solves each of them again from the zero
//!   start with the shipped kernel and the P1 kernel, each with its own
//!   operator and counters (the replay of node V25,
//!   `mf_confirm_measure.rs`, with the two kernels of this node). Every
//!   returned iterate is checked against the stage threshold with a JVP of
//!   this program. One line per attempt and per replayed system goes to the
//!   file `V28_SYSTEMLOG`; `V28_SHADOW_JSON` summarises the run.

#[path = "mf_confirm_problems.rs"]
mod problems;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Arc;
use std::time::Instant;

use problems::{Family, SplitMix64, adaptive, declared, v17, with_end};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, DenseMatrix, IdentityPreconditioner, LinearOperator,
    LinearSolverConfig, ShiftedOperator, WorkCounters, apply_counted, rodas5p_coefficients,
    safe_l2,
};
use rodas5p_integrators::{
    MfStageSolvePolicy, MfStageSolveStatistics, OdeProblem, OutputSchedule, Rodas5pMfFastResult,
    Rodas5pMfFastWorkspace, RoutingOptions, raw_absolute_residual_budget,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresIntoOptions, GmresWorkspace, ResidualAccounting,
    StagedGmresWorkspace, solve_gmres_into_with_accounting,
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

fn merge(target: &mut Value, source: Value) {
    let fields = target.as_object_mut().expect("object");
    for (key, value) in source.as_object().expect("object") {
        fields.insert(key.clone(), value.clone());
    }
}

// ---------------------------------------------------------------- the cells

/// The tolerances of both cell sets; `atol = rtol` (`atol_over_rtol` is 1 in
/// every family of the two sets).
const RTOLS: [f64; 5] = [1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7];

/// The families of node V25 with the end of their final span
/// (`mf_p1_confirm/PROTOCOL.json`, after the sizing pilot), in cell order.
const V25_FAMILIES: [(&str, f64); 4] = [
    ("ac3d", 1.0),
    ("robertson_rd1d", 320.0),
    ("schnakenberg2d", 1.0),
    ("porous2d", 0.1),
];

/// The families of node V24 (`mf_adaptive/PROTOCOL.json` deciding_grid, the
/// declared instances of node V17 with their spans), in cell order.
const V24_FAMILIES: [&str; 4] = ["fhn3d", "cusp1d", "burgers2d", "bruss1d"];

/// The number of deciding cells (`PREDECLARED.json` cells.count).
const CELL_COUNT: usize = 40;

/// One deciding cell.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cell {
    index: usize,
    /// The node whose protocol declares the cell.
    node: &'static str,
    family: &'static str,
    /// The end of the span for a V25 cell; `None` for a V24 cell (the span
    /// of the V17 generator).
    end: Option<f64>,
    rtol: f64,
}

impl Cell {
    /// Cell `index`: V25 cells 0 to 19 (ac3d, robertson_rd1d,
    /// schnakenberg2d, porous2d, each with rtol 1e-3 to 1e-7), then V24 cells
    /// 20 to 39 (fhn3d, cusp1d, burgers2d, bruss1d, each with rtol 1e-3 to
    /// 1e-7).
    fn of(index: usize) -> Option<Self> {
        if index >= CELL_COUNT {
            return None;
        }
        let (block, rtol) = (index / RTOLS.len(), RTOLS[index % RTOLS.len()]);
        Some(match V25_FAMILIES.get(block) {
            Some(&(family, end)) => Self {
                index,
                node: "V25",
                family,
                end: Some(end),
                rtol,
            },
            None => Self {
                index,
                node: "V24",
                family: V24_FAMILIES[block - V25_FAMILIES.len()],
                end: None,
                rtol,
            },
        })
    }

    /// The family of the cell: for V25 what `mf_confirm_measure.rs::family`
    /// builds with `V25_T1` = the final end, for V24 what
    /// `mf_adaptive_measure.rs::family` builds.
    fn family(&self) -> CoreResult<Family> {
        match self.end {
            Some(end) => with_end(declared(self.family)?, end),
            None => v17::declared(self.family),
        }
    }
}

// ---------------------------------------------------------------- the perturbation

/// The relative size of the perturbation of the start (`PREDECLARED.json`
/// measurements.T_ensemble).
const PERTURBATION: f64 = 1.0e-8;
/// Perturbed trajectories per cell (`M = 8`).
const TRAJECTORIES: u64 = 8;

/// The seed of component `i` of trajectory `j` of cell `cell`:
/// `(cell << 48) | (j << 32) | i`.
fn perturbation_seed(cell: usize, j: u64, i: usize) -> u64 {
    assert!(
        cell < (1 << 16) && j < (1 << 16) && (i as u64) < (1 << 32),
        "a field of the perturbation seed is out of range"
    );
    ((cell as u64) << 48) | (j << 32) | i as u64
}

/// The start of trajectory `j` of cell `cell`: `y0_i (1 + 1e-8 xi_ij)` with
/// `xi_ij` the first output of SplitMix64 started at
/// [`perturbation_seed`]`(cell, j, i)`, mapped linearly onto `[-1, 1)`
/// (`SplitMix64::symmetric` of `mf_confirm_problems.rs`). `j = 0` is the
/// unperturbed start.
fn perturbed_start(y0: &[f64], cell: usize, j: u64) -> Vec<f64> {
    if j == 0 {
        return y0.to_vec();
    }
    y0.iter()
        .enumerate()
        .map(|(i, &value)| {
            let xi = SplitMix64(perturbation_seed(cell, j, i)).symmetric();
            value * (1.0 + PERTURBATION * xi)
        })
        .collect()
}

// ---------------------------------------------------------------- the arms and the plain run

/// The arms of the node: a stage-solve policy, always named explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Arm {
    label: &'static str,
    policy: MfStageSolvePolicy,
}

impl Arm {
    const S: Self = Self {
        label: "S",
        policy: MfStageSolvePolicy::Shipped,
    };
    const P1: Self = Self {
        label: "P1",
        policy: MfStageSolvePolicy::ColumnTested,
    };

    fn parse(label: &str) -> Self {
        match label {
            "S" => Self::S,
            "P1" => Self::P1,
            other => panic!("unknown arm {other}"),
        }
    }

    /// The kernel of the arm in the replay: 0 shipped, 1 P1.
    fn kernel(self) -> usize {
        usize::from(self.policy != MfStageSolvePolicy::Shipped)
    }
}

/// The run of `arm` on `family` from `y0` at `rtol` through the workspace
/// configured as the policy entry (GMRES into, the arm's policy named, no
/// observer), output schedule `[t0, t1]`: the outcome and the statistics the
/// workspace holds after it (also after an error).
fn run_arm(
    family: &Family,
    y0: &[f64],
    rtol: f64,
    arm: Arm,
) -> CoreResult<(CoreResult<Rodas5pMfFastResult>, MfStageSolveStatistics)> {
    let config = adaptive(family, rtol);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1])?;
    let linear = RoutingOptions::default().matrix_free;
    let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear)?;
    work.set_gmres_into(true);
    work.set_stage_solve_policy(arm.policy)?;
    let outcome = work.integrate_observed(&family.problem, family.span, y0, &config, &output);
    Ok((outcome, work.stage_solve_policy_statistics()))
}

fn end_state(result: &Rodas5pMfFastResult) -> &[f64] {
    result.observed.y.last().map_or(&[], Vec::as_slice)
}

fn counters_json(counters: &WorkCounters) -> Value {
    serde_json::to_value(counters).expect("counters")
}

fn statistics_json(statistics: &MfStageSolveStatistics) -> Value {
    serde_json::to_value(statistics).expect("statistics")
}

fn result_json(result: &Rodas5pMfFastResult) -> Value {
    let t_end = result.observed.t.last().copied().unwrap_or(f64::NAN);
    json!({
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
        "state_sha256": sha256(end_state(result)),
        "driver": result.driver,
    })
}

fn header(mode: &str, cell: &Cell, family: &Family) -> Value {
    let config = adaptive(family, cell.rtol);
    json!({
        "schema": "vigilode.loop-l1.v28.run.v1",
        "label": variable("V28_LABEL").unwrap_or_default(),
        "mode": mode,
        "cell": cell.index,
        "node": cell.node,
        "family": family.id,
        "n": family.problem.dimension,
        "t_span": [family.span.0, family.span.1],
        "t_span_bits": [bits(family.span.0), bits(family.span.1)],
        "rtol": config.rtol,
        "atol": config.atol,
        "rtol_bits": bits(config.rtol),
        "atol_bits": bits(config.atol),
    })
}

fn mode_cells() {
    for index in 0..CELL_COUNT {
        let cell = Cell::of(index).expect("cell");
        let family = cell.family().expect("family");
        let config = adaptive(&family, cell.rtol);
        let mut record = header("CELLS", &cell, &family);
        merge(
            &mut record,
            json!({
                "atol_over_rtol": family.atol_over_rtol,
                "initial_step": config.initial_step,
                "min_step": config.min_step,
                "max_step": config.max_step,
                "max_attempts": config.max_attempts,
                "y0_sha256": sha256(&family.y0),
                "linear": format!("{:?}", RoutingOptions::default().matrix_free),
            }),
        );
        println!("V28_CELL_JSON {record}");
    }
}

fn mode_run(cell: &Cell, family: &Family) {
    let arm = Arm::parse(&variable("V28_ARM").expect("V28_ARM"));
    let j: u64 = number("V28_J").expect("V28_J");
    assert!(
        j <= TRAJECTORIES,
        "V28_J is 0 (unperturbed) to {TRAJECTORIES}"
    );
    let y0 = perturbed_start(&family.y0, cell.index, j);
    let moved = y0
        .iter()
        .zip(&family.y0)
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    let mut record = header("RUN", cell, family);
    merge(
        &mut record,
        json!({
            "arm": arm.label,
            "policy": arm.policy.id(),
            "policy_driver_id": arm.policy.driver_id(),
            "j": j,
            "perturbation": PERTURBATION,
            "y0_sha256": sha256(&family.y0),
            "start_sha256": sha256(&y0),
            "start_components_moved": moved,
        }),
    );
    let start = Instant::now();
    let outcome = run_arm(family, &y0, cell.rtol, arm);
    let wall = start.elapsed().as_secs_f64();
    merge(&mut record, json!({"wall_seconds_diagnostic_only": wall}));
    match outcome {
        Ok((Ok(result), statistics)) => {
            merge(&mut record, json!({"completed": true}));
            merge(&mut record, result_json(&result));
            merge(
                &mut record,
                json!({"policy_statistics": statistics_json(&statistics)}),
            );
            println!("V28_RUN_JSON {record}");
            println!(
                "V28_STATE_JSON {}",
                json!({
                    "label": record["label"],
                    "cell": cell.index,
                    "family": family.id,
                    "rtol": cell.rtol,
                    "arm": arm.label,
                    "j": j,
                    "n": end_state(&result).len(),
                    "state_sha256": record["state_sha256"],
                    "state_hex": hex(end_state(&result)),
                })
            );
        }
        Ok((Err(error), statistics)) => {
            merge(
                &mut record,
                json!({
                    "completed": false,
                    "error": error.to_string(),
                    "policy_statistics": statistics_json(&statistics),
                }),
            );
            println!("V28_RUN_JSON {record}");
        }
        Err(error) => {
            merge(
                &mut record,
                json!({"completed": false, "error": error.to_string()}),
            );
            println!("V28_RUN_JSON {record}");
        }
    }
}

// ---------------------------------------------------------------- the replay

/// The outcome of one kernel on one replayed stage system
/// (`mf_confirm_measure.rs`, unchanged in substance).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Replay {
    /// An iterate was returned after this many Arnoldi columns.
    Solved(u64),
    /// `GMRES exhausted ... Arnoldi vectors`.
    Budget,
    /// Any other error.
    Other,
}

impl Replay {
    fn of<T>(outcome: &CoreResult<T>, columns: impl Fn(&T) -> u64) -> Self {
        match outcome {
            Ok(report) => Self::Solved(columns(report)),
            Err(CoreError::LinearSolve(text)) if text.starts_with("GMRES exhausted") => {
                Self::Budget
            }
            Err(_) => Self::Other,
        }
    }

    fn solved(self) -> bool {
        matches!(self, Self::Solved(_))
    }

    /// The column count, -1 (budget) or -2 (other).
    fn code(self) -> i64 {
        match self {
            Self::Solved(columns) => columns as i64,
            Self::Budget => -1,
            Self::Other => -2,
        }
    }
}

/// How the driver's attempt ended before all its stage solves completed,
/// from the change of its `MfStageSolveStatistics` over the attempt: in a
/// stage solve by the budget, in a stage solve by another linear-solve error,
/// or by an error the statistics do not classify (for example a non-finite
/// stage right-hand side; the system of that stage is replayed all the same).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DriverFailure {
    Budget,
    Other,
    Elsewhere,
}

impl DriverFailure {
    fn code(self) -> &'static str {
        match self {
            Self::Budget => "B",
            Self::Other => "O",
            Self::Elsewhere => "E",
        }
    }
}

/// One kernel on one replayed system.
#[derive(Clone, Copy, Debug)]
struct KernelRun {
    replay: Replay,
    /// `||rhs - (U - h gamma J U)||_2` of the returned iterate, formed here
    /// with a JVP of this program (`NaN` when no iterate was returned).
    residual: f64,
    /// The true residual norm the kernel reported (`NaN` on an error).
    reported: f64,
    /// The threshold the kernel reports using (P1 kernel; `NaN` otherwise).
    kernel_threshold: f64,
}

/// Counts over the replayed systems of one run; index 0 is the shipped
/// kernel, 1 the P1 kernel.
#[derive(Clone, Debug, Default)]
struct ShadowTally {
    attempts: u64,
    systems: u64,
    driver_completed: u64,
    driver_failed_budget: u64,
    driver_failed_other: u64,
    /// Replayed systems of the stage at which an attempt ended with an error
    /// the statistics do not classify.
    driver_failed_elsewhere: u64,
    /// Attempts that ended with such an error (with or without a replayed
    /// system).
    attempts_failed_elsewhere: u64,
    /// Replays of the driver's own kernel that do not reproduce the driver:
    /// another column count for a completed solve, or another outcome for
    /// the solve in which an attempt failed.
    replay_differs_from_driver: u64,
    /// Systems of completed or stage-solve-failed solves whose right-hand
    /// side or operator could not be rebuilt.
    rebuild_failures: u64,
    /// Systems of the stage at which an attempt ended with an error the
    /// statistics do not classify, whose right-hand side could not be
    /// rebuilt either (the driver's failure, reproduced; nothing to replay).
    elsewhere_not_rebuilt: u64,
    solved: [u64; 2],
    budget: [u64; 2],
    other: [u64; 2],
    columns: [u64; 2],
    /// Discriminator K: P1 exhausts the budget and the shipped kernel does
    /// not.
    p1_budget_shipped_not: u64,
    shipped_budget_p1_not: u64,
    both_budget: u64,
    /// Returned iterates whose independent residual is not finite or exceeds
    /// the stage threshold (index 1 decides discriminator V).
    violations: [u64; 2],
    /// Returned iterates whose residual as the kernel reported it is not
    /// finite or exceeds the stage threshold.
    reported_violations: [u64; 2],
    max_ratio: [f64; 2],
    /// P1 solves whose reported threshold differs in its bits from the stage
    /// threshold formed here.
    p1_threshold_differs: u64,
}

/// The replay of every stage system of a run.
struct Shadow {
    gamma: f64,
    a: DenseMatrix,
    c_matrix: DenseMatrix,
    c: Vec<f64>,
    /// The configuration the driver hands its kernel.
    config: GmresConfig,
    rtol_lin: f64,
    /// `gamma atol_lin` rounded down, as the driver forms it.
    atol_scaled: f64,
    state: Vec<f64>,
    rhs: Vec<f64>,
    out: Vec<f64>,
    jv: Vec<f64>,
    residual: Vec<f64>,
    /// The kernels and the rebuilt right-hand sides.
    counters: WorkCounters,
    /// The independent residual checks.
    check_counters: WorkCounters,
    shipped: GmresWorkspace,
    adaptive: StagedGmresWorkspace,
    logged: usize,
    seen_budget: u64,
    seen_other: u64,
    driver_kernel: usize,
    writer: Option<BufWriter<File>>,
    write_error: Option<String>,
    lines: u64,
    tally: ShadowTally,
}

impl Shadow {
    fn new(
        n: usize,
        linear: &LinearSolverConfig,
        arm: Arm,
        path: Option<&str>,
    ) -> CoreResult<Self> {
        let coeffs = rodas5p_coefficients()?;
        let atol_scaled = if linear.atol == 0.0 {
            0.0
        } else {
            raw_absolute_residual_budget(linear.atol, coeffs.gamma)?
        };
        let writer = match path {
            Some(path) => Some(BufWriter::new(File::create(path).map_err(|error| {
                CoreError::InvalidInput(format!("cannot create {path}: {error}"))
            })?)),
            None => None,
        };
        Ok(Self {
            gamma: coeffs.gamma,
            a: coeffs.a.clone(),
            c_matrix: coeffs.c_matrix.clone(),
            c: coeffs.c.clone(),
            config: GmresConfig {
                restart: linear.restart,
                max_arnoldi: linear.maxiter.max(linear.restart),
                rtol: linear.rtol,
                atol: raw_absolute_residual_budget(linear.atol, coeffs.gamma)?,
            },
            rtol_lin: linear.rtol,
            atol_scaled,
            state: vec![0.0; n],
            rhs: vec![0.0; n],
            out: vec![0.0; n],
            jv: vec![0.0; n],
            residual: vec![0.0; n],
            counters: WorkCounters::default(),
            check_counters: WorkCounters::default(),
            shipped: GmresWorkspace::default(),
            adaptive: StagedGmresWorkspace::default(),
            logged: 0,
            seen_budget: 0,
            seen_other: 0,
            driver_kernel: arm.kernel(),
            writer,
            write_error: None,
            lines: 0,
            tally: ShadowTally::default(),
        })
    }

    fn write(&mut self, line: &str) {
        self.lines += 1;
        if let Some(writer) = self.writer.as_mut()
            && let Err(error) = writer.write_all(line.as_bytes())
        {
            self.write_error.get_or_insert_with(|| error.to_string());
        }
    }

    fn finish(&mut self) {
        if let Some(writer) = self.writer.as_mut()
            && let Err(error) = writer.flush()
        {
            self.write_error.get_or_insert_with(|| error.to_string());
        }
    }

    /// The right-hand side of stage `i` of the attempt `(t, y, h)` from the
    /// stage equations of the driver's U form, in the driver's order of
    /// operations (`mf_confirm_measure.rs`, unchanged):
    ///
    /// ```text
    /// Y_i   = y + sum_j a_ij U_j
    /// rhs_i = h gamma f(t + c_i h, Y_i) + sum_j (gamma C_ij) U_j
    /// ```
    fn stage_rhs(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        i: usize,
        work: &Rodas5pMfFastWorkspace,
    ) -> CoreResult<()> {
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
        Ok(())
    }

    /// `||rhs - (U - h gamma J U)||_2` of the iterate in `self.out`, with the
    /// JVP `jacobian` and this program's own buffers and counters (the
    /// formula of `ProtectedAudit` of node V17).
    fn true_residual(&mut self, jacobian: &dyn LinearOperator, hg: f64) -> f64 {
        if apply_counted(
            jacobian,
            &self.out,
            &mut self.jv,
            &mut self.check_counters,
            ApplyCategory::Diagnostic,
        )
        .is_err()
        {
            return f64::NAN;
        }
        for (((r, b), x), jx) in self
            .residual
            .iter_mut()
            .zip(&self.rhs)
            .zip(&self.out)
            .zip(&self.jv)
        {
            *r = b - (x - hg * jx);
        }
        safe_l2(&self.residual)
    }

    /// The two kernels on the system in `self.rhs`, each from the zero
    /// start, each returned iterate checked at once.
    fn replay(
        &mut self,
        operator: &dyn LinearOperator,
        jacobian: &dyn LinearOperator,
        hg: f64,
    ) -> [KernelRun; 2] {
        let n = self.rhs.len();
        let outcome = solve_gmres_into_with_accounting(
            operator,
            &IdentityPreconditioner::new(n),
            &self.rhs,
            None,
            &self.config,
            None,
            &mut self.out,
            &mut self.shipped,
            GmresCapacity::unbounded(),
            GmresIntoOptions::default(),
            ResidualAccounting::DEFAULT,
            &mut self.counters,
        );
        let replay = Replay::of(&outcome, |report| report.iterations);
        let shipped = KernelRun {
            replay,
            residual: if replay.solved() {
                self.true_residual(jacobian, hg)
            } else {
                f64::NAN
            },
            reported: outcome
                .as_ref()
                .map_or(f64::NAN, |report| report.residual_norm),
            kernel_threshold: f64::NAN,
        };
        let outcome = self.adaptive.solve_column_tested_into(
            operator,
            &self.rhs,
            None,
            &self.config,
            self.config.restart,
            ResidualAccounting::DEFAULT,
            &mut self.out,
            &mut self.counters,
            None,
        );
        let replay = Replay::of(&outcome, |report| report.columns);
        let p1 = KernelRun {
            replay,
            residual: if replay.solved() {
                self.true_residual(jacobian, hg)
            } else {
                f64::NAN
            },
            reported: outcome
                .as_ref()
                .map_or(f64::NAN, |report| report.residual_norm),
            kernel_threshold: outcome.as_ref().map_or(f64::NAN, |report| report.threshold),
        };
        [shipped, p1]
    }

    /// Count one replayed system; `driver` is the driver's column count of a
    /// completed solve or the class of the failed solve.
    fn tally(
        &mut self,
        kernels: &[KernelRun; 2],
        threshold: f64,
        driver: Result<u64, DriverFailure>,
    ) {
        let tally = &mut self.tally;
        tally.systems += 1;
        let own = kernels[self.driver_kernel].replay;
        let reproduces = match driver {
            Ok(columns) => own == Replay::Solved(columns),
            Err(DriverFailure::Budget) => own == Replay::Budget,
            Err(DriverFailure::Other | DriverFailure::Elsewhere) => own == Replay::Other,
        };
        match driver {
            Ok(_) => tally.driver_completed += 1,
            Err(DriverFailure::Budget) => tally.driver_failed_budget += 1,
            Err(DriverFailure::Other) => tally.driver_failed_other += 1,
            Err(DriverFailure::Elsewhere) => tally.driver_failed_elsewhere += 1,
        }
        tally.replay_differs_from_driver += u64::from(!reproduces);
        for (k, kernel) in kernels.iter().enumerate() {
            match kernel.replay {
                Replay::Solved(columns) => {
                    tally.solved[k] += 1;
                    tally.columns[k] += columns;
                    let passed = kernel.residual.is_finite() && kernel.residual <= threshold;
                    tally.violations[k] += u64::from(!passed);
                    let reported = kernel.reported.is_finite() && kernel.reported <= threshold;
                    tally.reported_violations[k] += u64::from(!reported);
                    if threshold > 0.0 {
                        let ratio = kernel.residual / threshold;
                        if ratio.is_finite() {
                            tally.max_ratio[k] = tally.max_ratio[k].max(ratio);
                        }
                    }
                }
                Replay::Budget => tally.budget[k] += 1,
                Replay::Other => tally.other[k] += 1,
            }
        }
        let [shipped, p1] = kernels;
        let budget = |kernel: &KernelRun| kernel.replay == Replay::Budget;
        tally.p1_budget_shipped_not += u64::from(budget(p1) && !budget(shipped));
        tally.shipped_budget_p1_not += u64::from(budget(shipped) && !budget(p1));
        tally.both_budget += u64::from(budget(p1) && budget(shipped));
        if p1.replay.solved() && p1.kernel_threshold.to_bits() != threshold.to_bits() {
            tally.p1_threshold_differs += 1;
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
        let completed = log.len().saturating_sub(self.logged);
        let first_entry = self.logged;
        self.logged = log.len();
        let statistics = work.stage_solve_policy_statistics();
        let new_budget = statistics
            .budget_exhausted_solves
            .saturating_sub(self.seen_budget);
        let new_other = statistics
            .other_linear_failures
            .saturating_sub(self.seen_other);
        self.seen_budget = statistics.budget_exhausted_solves;
        self.seen_other = statistics.other_linear_failures;
        let attempt = self.tally.attempts;
        self.tally.attempts += 1;
        // The class of the failure of an attempt that failed (the driver
        // classified a stage-solve failure after the attempt), and the solve
        // in which it failed if fewer than all stage solves completed (the
        // rule of the V25 replay: that stage's system is replayed).
        let class = match (error, new_budget, new_other) {
            (Some(_), ..) => None,
            (None, 1, 0) => Some(DriverFailure::Budget),
            (None, 0, 1) => Some(DriverFailure::Other),
            (None, ..) => Some(DriverFailure::Elsewhere),
        };
        if class == Some(DriverFailure::Elsewhere) {
            self.tally.attempts_failed_elsewhere += 1;
        }
        let failure = class.filter(|_| completed < self.c.len());
        let failed = usize::from(failure.is_some());
        let failure_code = class.map_or("-", DriverFailure::code);
        self.write(&format!(
            "A {attempt} {} {} {completed} {failure_code} {}\n",
            bits(t),
            bits(h),
            error.map_or_else(|| "-".to_string(), bits)
        ));
        if completed + failed == 0 {
            return;
        }
        let rebuilt = problem.linearize_matrix_free(t, y).and_then(|jacobian| {
            ShiftedOperator::new_counted_jvp(None, Arc::clone(&jacobian), h, self.gamma)
                .map(|operator| (jacobian, operator))
        });
        let Ok((jacobian, operator)) = rebuilt else {
            for i in 0..completed + failed {
                self.tally.rebuild_failures += 1;
                self.write(&format!("R {attempt} {i} -\n"));
            }
            return;
        };
        let hg = h * self.gamma;
        for i in 0..completed + failed {
            if self.stage_rhs(problem, t, y, h, i, work).is_err() {
                if i >= completed && failure == Some(DriverFailure::Elsewhere) {
                    self.tally.elsewhere_not_rebuilt += 1;
                    self.write(&format!("R {attempt} {i} E\n"));
                } else {
                    self.tally.rebuild_failures += 1;
                    self.write(&format!("R {attempt} {i} -\n"));
                }
                continue;
            }
            let right_norm = safe_l2(&self.rhs);
            let threshold = self.atol_scaled.max(self.rtol_lin * right_norm);
            let kernels = self.replay(&operator, jacobian.as_ref(), hg);
            let (driver, driver_code, driver_columns) = if i < completed {
                let columns = log[first_entry + i].iterations;
                (Ok(columns), "C", columns.to_string())
            } else {
                let class = failure.expect("a failed solve has a class");
                (Err(class), class.code(), "-".to_string())
            };
            self.tally(&kernels, threshold, driver);
            let [shipped, p1] = kernels;
            self.write(&format!(
                "Y {attempt} {i} {driver_code} {driver_columns} {} {} {} {} {} {} {} {} {}\n",
                bits(right_norm),
                bits(threshold),
                shipped.replay.code(),
                bits(shipped.residual),
                bits(shipped.reported),
                p1.replay.code(),
                bits(p1.residual),
                bits(p1.reported),
                bits(p1.kernel_threshold),
            ));
        }
    }
}

/// The run of `arm` on `family` from its unperturbed start with the replay
/// of every stage system; the per-system lines go to `path` if given.
fn shadow_run(
    family: &Family,
    rtol: f64,
    arm: Arm,
    path: Option<&str>,
) -> CoreResult<(
    CoreResult<Rodas5pMfFastResult>,
    MfStageSolveStatistics,
    Shadow,
)> {
    let config = adaptive(family, rtol);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1])?;
    let linear = RoutingOptions::default().matrix_free;
    let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear)?;
    work.set_gmres_into(true);
    work.set_stage_solve_policy(arm.policy)?;
    work.set_solve_log(true);
    let mut shadow = Shadow::new(family.problem.dimension, &linear, arm, path)?;
    let mut observer =
        |t: f64, y: &[f64], h: f64, error: Option<f64>, work: &Rodas5pMfFastWorkspace| {
            shadow.observe(&family.problem, t, y, h, error, work);
        };
    let outcome = work
        .integrate_observed_with_observer(
            &family.problem,
            family.span,
            &family.y0,
            &config,
            &output,
            &mut observer,
        )
        .map(|(result, ..)| result);
    shadow.finish();
    Ok((outcome, work.stage_solve_policy_statistics(), shadow))
}

fn tally_json(shadow: &Shadow) -> Value {
    let tally = &shadow.tally;
    json!({
        "kernels": ["shipped", "P1"],
        "attempts_observed": tally.attempts,
        "systems": tally.systems,
        "driver_completed": tally.driver_completed,
        "driver_failed_budget": tally.driver_failed_budget,
        "driver_failed_other": tally.driver_failed_other,
        "driver_failed_elsewhere": tally.driver_failed_elsewhere,
        "attempts_failed_elsewhere": tally.attempts_failed_elsewhere,
        "replay_differs_from_driver": tally.replay_differs_from_driver,
        "rebuild_failures": tally.rebuild_failures,
        "elsewhere_not_rebuilt": tally.elsewhere_not_rebuilt,
        "solved": tally.solved,
        "budget": tally.budget,
        "other": tally.other,
        "columns_of_solved": tally.columns,
        "p1_budget_shipped_not": tally.p1_budget_shipped_not,
        "shipped_budget_p1_not": tally.shipped_budget_p1_not,
        "both_budget": tally.both_budget,
        "violations": tally.violations,
        "reported_violations": tally.reported_violations,
        "max_ratio": tally.max_ratio,
        "p1_threshold_differs": tally.p1_threshold_differs,
        "replay_jvp_calls": shadow.counters.jvp_calls,
        "replay_rhs_calls": shadow.counters.rhs_calls,
        "check_jvp_calls": shadow.check_counters.jvp_calls,
        "lines": shadow.lines,
        "write_error": shadow.write_error,
    })
}

fn mode_shadow(cell: &Cell, family: &Family) {
    let arm = Arm::parse(&variable("V28_ARM").expect("V28_ARM"));
    let path = variable("V28_SYSTEMLOG");
    let mut record = header("SHADOW", cell, family);
    merge(
        &mut record,
        json!({
            "arm": arm.label,
            "policy": arm.policy.id(),
            "policy_driver_id": arm.policy.driver_id(),
            "j": 0,
            "y0_sha256": sha256(&family.y0),
            "systemlog": path,
        }),
    );
    let start = Instant::now();
    let outcome = shadow_run(family, cell.rtol, arm, path.as_deref());
    let wall = start.elapsed().as_secs_f64();
    merge(&mut record, json!({"wall_seconds_diagnostic_only": wall}));
    match outcome {
        Ok((outcome, statistics, shadow)) => {
            match outcome {
                Ok(result) => {
                    merge(&mut record, json!({"completed": true}));
                    merge(&mut record, result_json(&result));
                }
                Err(error) => merge(
                    &mut record,
                    json!({"completed": false, "error": error.to_string()}),
                ),
            }
            merge(
                &mut record,
                json!({"policy_statistics": statistics_json(&statistics)}),
            );
            merge(&mut record, tally_json(&shadow));
        }
        Err(error) => merge(
            &mut record,
            json!({"completed": false, "error": error.to_string()}),
        ),
    }
    println!("V28_SHADOW_JSON {record}");
}

#[test]
#[ignore = "measurement program: run with --ignored --nocapture and the V28_* variables"]
fn v28_run() {
    let Some(mode) = variable("V28_MODE") else {
        println!("V28_USAGE set V28_MODE, V28_CELL, V28_ARM, V28_J, ...; see the file header");
        return;
    };
    if mode == "CELLS" {
        mode_cells();
        return;
    }
    let index: usize = number("V28_CELL").expect("V28_CELL");
    let cell = Cell::of(index).expect("V28_CELL is a cell index 0 to 39");
    let family = cell.family().expect("family");
    match mode.as_str() {
        "SETUP" => {
            let mut record = header("SETUP", &cell, &family);
            merge(&mut record, json!({"setup_only": true}));
            println!("V28_RUN_JSON {record}");
        }
        "RUN" => mode_run(&cell, &family),
        "SHADOW" => mode_shadow(&cell, &family),
        other => panic!("unknown V28_MODE {other}"),
    }
}

// ---------------------------------------------------------------- contract tests (plumbing; development
// instances only, none of the 40 cells is built or run here)

#[test]
fn the_cell_table_is_the_declared_one() {
    let cells: Vec<Cell> = (0..CELL_COUNT)
        .map(|k| Cell::of(k).expect("cell"))
        .collect();
    assert!(Cell::of(CELL_COUNT).is_none());
    let expected = [
        ("V25", "ac3d", Some(1.0)),
        ("V25", "robertson_rd1d", Some(320.0)),
        ("V25", "schnakenberg2d", Some(1.0)),
        ("V25", "porous2d", Some(0.1)),
        ("V24", "fhn3d", None),
        ("V24", "cusp1d", None),
        ("V24", "burgers2d", None),
        ("V24", "bruss1d", None),
    ];
    for (k, cell) in cells.iter().enumerate() {
        let (node, family, end) = expected[k / 5];
        assert_eq!(cell.index, k);
        assert_eq!((cell.node, cell.family, cell.end), (node, family, end));
        assert_eq!(cell.rtol, [1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7][k % 5]);
        assert_eq!(
            cell.rtol,
            format!("1e-{}", 3 + k % 5).parse::<f64>().unwrap()
        );
    }
}

#[test]
fn the_perturbation_is_the_declared_one() {
    assert_eq!(
        perturbation_seed(3, 2, 7),
        (3_u64 << 48) | (2_u64 << 32) | 7
    );
    assert_eq!(
        perturbation_seed(39, 8, 4095),
        (39_u64 << 48) | (8_u64 << 32) | 4095
    );
    let seed = perturbation_seed(3, 2, 7);
    let z = SplitMix64(seed).next_u64();
    let unit = (z >> 11) as f64 * (1.0 / (1_u64 << 53) as f64);
    assert_eq!(
        SplitMix64(seed).symmetric().to_bits(),
        (2.0 * unit - 1.0).to_bits()
    );
    let family = declared("dev_robertson_rd1d").expect("development family");
    let y0 = &family.y0;
    assert_eq!(hex(&perturbed_start(y0, 5, 0)), hex(y0));
    let first = perturbed_start(y0, 5, 1);
    assert_eq!(hex(&first), hex(&perturbed_start(y0, 5, 1)));
    assert_ne!(hex(&first), hex(&perturbed_start(y0, 5, 2)));
    assert_ne!(hex(&first), hex(&perturbed_start(y0, 6, 1)));
    let mut moved = 0;
    for (i, (&x, &p)) in y0.iter().zip(&first).enumerate() {
        let xi = SplitMix64(perturbation_seed(5, 1, i)).symmetric();
        assert!((-1.0..1.0).contains(&xi));
        assert_eq!(p.to_bits(), (x * (1.0 + 1.0e-8 * xi)).to_bits());
        if x == 0.0 {
            assert_eq!(p, 0.0);
        } else {
            assert!(((p - x) / x).abs() <= 1.0e-8 * (1.0 + 1.0e-12));
            moved += usize::from(p != x);
        }
    }
    assert!(moved > 0);
}

#[test]
fn the_replay_reproduces_the_driver_on_development_problems() {
    let families = [
        (declared("dev_robertson_rd1d").expect("family"), 1.0e-3),
        (v17::declared("dev_cusp1d").expect("family"), 1.0e-4),
    ];
    for (family, rtol) in &families {
        for arm in [Arm::S, Arm::P1] {
            let (outcome, statistics, shadow) =
                shadow_run(family, *rtol, arm, None).expect("shadow run");
            let result = outcome.expect("integration");
            assert!(result.observed.success, "{} {}", family.id, arm.label);
            let tally = &shadow.tally;
            let context = format!(
                "{} {}: {tally:?} {:?} {:?}",
                family.id, arm.label, statistics, result.observed.counters
            );
            assert_eq!(tally.attempts, result.attempts as u64, "{context}");
            assert_eq!(tally.replay_differs_from_driver, 0, "{context}");
            assert_eq!(tally.rebuild_failures, 0, "{context}");
            let counters = &result.observed.counters;
            assert_eq!(
                tally.attempts_failed_elsewhere, counters.nonfinite_step_failures,
                "{context}"
            );
            assert_eq!(tally.driver_completed, counters.linear_solves, "{context}");
            assert_eq!(
                tally.driver_failed_budget, statistics.budget_exhausted_solves,
                "{context}"
            );
            assert_eq!(
                tally.driver_failed_other, statistics.other_linear_failures,
                "{context}"
            );
            assert_eq!(
                tally.systems,
                tally.driver_completed
                    + tally.driver_failed_budget
                    + tally.driver_failed_other
                    + tally.driver_failed_elsewhere,
                "{context}"
            );
            assert_eq!(
                shadow.lines,
                tally.attempts + tally.systems + tally.elsewhere_not_rebuilt,
                "{context}"
            );
            assert_eq!(tally.violations, [0, 0]);
            assert_eq!(tally.reported_violations, [0, 0]);
            assert_eq!(tally.p1_threshold_differs, 0);
            // The plain run of the unperturbed start ends where the observed
            // run ends: the observer and the solve log change nothing.
            let (plain, plain_statistics) =
                run_arm(family, &family.y0, *rtol, arm).expect("plain run");
            let plain = plain.expect("integration");
            assert_eq!(sha256(end_state(&plain)), sha256(end_state(&result)));
            assert_eq!(plain.attempts, result.attempts);
            assert_eq!(plain_statistics, statistics);
        }
    }
}
