//! Recording program of loop node V31 (owner decision OD-L1-27: the default
//! first-use check of the coloured route of the assembly client is
//! `CheckColumns::All` where `CheckColourings::total_groups` of the declared
//! pattern is at least `n`, `CheckColumns::MultiColouring` otherwise). One
//! ignored test driven by environment variables; every result is one JSON
//! object per line behind a tag. The logs are read by `parse_results.py` of
//! `research/production_loop_20261116/assembly_wide_band/`.
//!
//! ```text
//! V31_MODE=survey
//!     total_groups and the default check of random banded declared patterns
//!     (no right-hand side is evaluated)
//! V31_MODE=wide V31_FAMILY=<family>[,<family>..] V31_EXECUTIONS=<seq|tP>[,..]
//!     the decisions of the default first-use check on the supplement
//! cargo test --release -p rodas5p-integrators --test stage_clients_assembly_wide_band_record -- --ignored --nocapture
//! ```
//!
//! The identity cells of node V08 and the corpus of node V26 are recorded with
//! the program of node V29 (`stage_clients_assembly_default_record.rs`),
//! unchanged. That corpus has six cases where the rule picks `All`, all
//! correct patterns with `total_groups == n`; this supplement adds declared
//! patterns where the rule picks `All` with `total_groups > n` and with
//! `total_groups == n`, correct and wrong, and narrow controls where the rule
//! keeps `MultiColouring`.
//!
//! Every client is built with the options a caller gets without naming a check
//! (`AssemblyOptions::coloured(pattern)`, then `threaded(..)`); the check in
//! use is read back from `AssemblyOptions::first_use_check` and recorded. The
//! program is run unchanged before the change of the default (commit A of node
//! V31) and after it. For reference every case also records the explicit full
//! check (`All`) and the explicit multi-colouring check, each on a fresh client
//! without a first-use check; they do not depend on the default.
//!
//! Families (deterministic: `splitmix64` of the seed, the row and the column):
//! - `random`: random banded true patterns of order 60, 96, 128 and 200 with
//!   half-width 0.2, 0.3 and 0.45 of the order and density 0.1 and 0.3 inside
//!   the band (the diagonal always), seed 1. Declared: the true pattern
//!   (correct), the full band of its bandwidth (correct, a superset) and the
//!   true pattern without one off-diagonal entry, 16 entries evenly spread in
//!   row-major order (wrong). Right-hand side: row `i` is the sum over its
//!   true columns of `w_ij y_j (1 + y_j / 10)`.
//! - `band`: the compact-kernel Fisher-KPP problem of order 60 and 128 (true
//!   pattern: the full band `(r, r)`) with `r` = `n / 4`, `0.3 n`, `0.45 n`
//!   (rounded) and `n - 1`. Declared: the band (correct) and the bands one too
//!   narrow, `(r - 1, r)` and `(r, r - 1)` (wrong).
//! - `ring`: the periodic ring of the V26 corpus at the orders where its true
//!   pattern has `total_groups == n` (12, 13, 14, 15, 16, 32); declared: the
//!   true pattern without one entry, every entry (wrong).
//! - `narrow`: controls where the rule keeps `MultiColouring`: random banded
//!   true patterns of order 96 and 200 with half-width 0.05 of the order and
//!   density 0.3 (the true pattern and 16 omissions), and the Fisher-KPP
//!   problem of order 128 with `r = 8` (the band and the two bands one too
//!   narrow).

use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::StageLinearSystem;
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyError, AssemblyOptions, AssemblyStageClient, AssemblyStats, CheckColourings,
    CheckColumns, CheckConstruction, LinearBackend, RhsPurity, SparsityPattern, colour_columns,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{FdProblem, Rhs, from_rhs, nlkpp_band, ring, ring_pattern};

const DENSE_IN_PLACE: LinearBackend = LinearBackend::DenseLu(DenseLuMode::InPlace);
/// Omitted entries of one random pattern.
const OMISSIONS: usize = 16;
/// The orders of the ring whose true pattern has `total_groups == n` in the
/// corpus of node V26.
const RING_EQUALITY_ORDERS: [usize; 6] = [12, 13, 14, 15, 16, 32];
/// Omitted entries of one case for which a miss is evaluated in full.
const EVIDENCE_ENTRIES_MAX: usize = 64;

/// Where the right-hand sides of a client run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Execution {
    Sequential,
    Threaded(usize),
}

impl Execution {
    fn parse(text: &str) -> Self {
        match text {
            "seq" => Self::Sequential,
            _ => Self::Threaded(
                text.strip_prefix('t')
                    .and_then(|workers| workers.parse().ok())
                    .unwrap_or_else(|| panic!("unknown execution {text}")),
            ),
        }
    }

    fn name(self) -> String {
        match self {
            Self::Sequential => "seq".into(),
            Self::Threaded(workers) => format!("t{workers}"),
        }
    }
}

/// The options a caller gets for `pattern` without naming a check.
fn default_options(pattern: &SparsityPattern, execution: Execution) -> AssemblyOptions {
    let options = AssemblyOptions::coloured(pattern.clone());
    match execution {
        Execution::Sequential => options,
        Execution::Threaded(workers) => options.threaded(workers, RhsPurity::pure_and_reentrant()),
    }
}

/// The further colourings of the multi-colouring check of `pattern`.
fn colourings_record(pattern: &SparsityPattern) -> serde_json::Value {
    match CheckColourings::of(pattern) {
        Ok(plan) => serde_json::json!({
            "construction": match plan.construction() {
                CheckConstruction::Band { .. } => "band",
                CheckConstruction::Refinement { .. } => "refinement",
            },
            "construction_detail": format!("{:?}", plan.construction()),
            "colourings": plan.colourings(),
            "total_groups": plan.total_groups(),
            "total_groups_band": plan.total_groups_band(),
            "total_groups_refinement": plan.total_groups_refinement(),
        }),
        Err(error) => serde_json::json!({"error": format!("{error:?}")}),
    }
}

// ---------------------------------------------------------------------------
// Random banded patterns.
// ---------------------------------------------------------------------------

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A uniform number in `[0, 1)` of `(seed, i, j)`.
fn uniform(seed: u64, i: usize, j: usize) -> f64 {
    let key = ((i as u64) << 32) | j as u64;
    let hash = splitmix64(seed ^ splitmix64(key));
    (hash >> 11) as f64 / (1_u64 << 53) as f64
}

/// The half-width `fraction * n`, rounded, at least 1 and below `n`.
fn half_width(n: usize, fraction: f64) -> usize {
    ((fraction * n as f64).round() as usize).clamp(1, n - 1)
}

/// The diagonal and every `(i, j)` with `0 < |i - j| <= half_width` drawn
/// with probability `density`.
fn random_band_rows(n: usize, half_width: usize, density: f64, seed: u64) -> Vec<Vec<usize>> {
    (0..n)
        .map(|i| {
            (i.saturating_sub(half_width)..=(i + half_width).min(n - 1))
                .filter(|&j| j == i || uniform(seed, i, j) < density)
                .collect()
        })
        .collect()
}

fn pattern(n: usize, rows: &[Vec<usize>]) -> SparsityPattern {
    SparsityPattern::from_rows(n, rows).unwrap()
}

fn band(n: usize, lower: usize, upper: usize) -> SparsityPattern {
    SparsityPattern::banded(n, lower, upper).unwrap()
}

/// The check of the default options of `pattern` and the right-hand sides it
/// spends at a sequential first use.
fn default_check_and_cost(pattern: &SparsityPattern, total_groups: usize) -> (CheckColumns, usize) {
    let check = AssemblyOptions::coloured(pattern.clone()).first_use_check;
    let cost = match check {
        CheckColumns::All | CheckColumns::AllPooled => pattern.n(),
        CheckColumns::MultiColouring | CheckColumns::MultiColouringPooled => total_groups,
        CheckColumns::Sample(count) => count.min(pattern.n()),
        CheckColumns::None => 0,
    };
    (check, cost)
}

/// total_groups and the default check of random banded declared patterns.
fn survey() {
    let start = std::time::Instant::now();
    let mut patterns = 0;
    for n in [60, 96, 128, 200, 300, 500] {
        for fraction in [0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45] {
            for density in [0.05, 0.1, 0.3, 1.0] {
                let seeds: &[u64] = if density < 1.0 { &[1, 2] } else { &[1] };
                for &seed in seeds {
                    let width = half_width(n, fraction);
                    let declared = pattern(n, &random_band_rows(n, width, density, seed));
                    let colourings = colourings_record(&declared);
                    let total_groups = colourings["total_groups"].as_u64().unwrap() as usize;
                    let (check, cost) = default_check_and_cost(&declared, total_groups);
                    println!(
                        "V31_SURVEY_JSON {}",
                        serde_json::json!({
                            "n": n,
                            "half_width_fraction": fraction,
                            "half_width": width,
                            "density": density,
                            "seed": seed,
                            "declared_nnz": declared.nnz(),
                            "bandwidth": declared.bandwidth(),
                            "client_colours": colour_columns(&declared).1,
                            "check_colourings": colourings,
                            "rule_picks_all": total_groups >= n,
                            "default_check": format!("{check:?}"),
                            "default_check_rhs": cost,
                        })
                    );
                    patterns += 1;
                }
            }
        }
    }
    println!(
        "V31_SURVEY_DONE_JSON {}",
        serde_json::json!({
            "patterns": patterns,
            "wall_s_diagnostic": start.elapsed().as_secs_f64(),
        })
    );
}

// ---------------------------------------------------------------------------
// The supplement: cases with a right-hand side.
// ---------------------------------------------------------------------------

/// One case of the supplement.
struct Case {
    id: String,
    /// Index into the problems of the family.
    problem: usize,
    declared: SparsityPattern,
    /// Entries of the true pattern the declaration omits.
    omitted: Vec<(usize, usize)>,
    kind: &'static str,
}

/// Entries of `truth` that `declared` does not hold, row-major.
fn omitted(truth: &SparsityPattern, declared: &SparsityPattern) -> Vec<(usize, usize)> {
    let mut missing = Vec::new();
    for i in 0..truth.n() {
        for &k in truth.row(i) {
            if !declared.contains(i, k) {
                missing.push((i, k));
            }
        }
    }
    missing
}

fn case(
    id: String,
    problem: usize,
    declared: SparsityPattern,
    truth: &SparsityPattern,
    kind: &'static str,
) -> Case {
    Case {
        id,
        problem,
        omitted: omitted(truth, &declared),
        declared,
        kind,
    }
}

/// A problem whose row `i` is the sum over the columns `rows[i]` of
/// `w_ij y_j (1 + y_j / 10)`, `w_ij = 0.5 + 0.37 ((3 i + 7 j) mod 11) / 11`.
fn generic_problem(id: String, rows: &[Vec<usize>]) -> FdProblem {
    let n = rows.len();
    let weights: Vec<Vec<(usize, f64)>> = rows
        .iter()
        .enumerate()
        .map(|(i, columns)| {
            columns
                .iter()
                .map(|&j| (j, 0.5 + 0.37 * ((3 * i + 7 * j) % 11) as f64 / 11.0))
                .collect()
        })
        .collect();
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            for (value, columns) in out.iter_mut().zip(&weights) {
                let mut acc = 0.0;
                for &(j, w) in columns {
                    acc += w * y[j] * (1.0 + 0.1 * y[j]);
                }
                *value = acc;
            }
            Ok(())
        },
    );
    let y0 = (0..n)
        .map(|j| 0.75 + 0.25 * (0.61 * j as f64 + 0.3).sin())
        .collect();
    from_rhs(id, rhs, y0, (0.0, 1.0), None)
}

/// A random banded true pattern: order, half-width as a fraction of the
/// order, density inside the band, seed.
#[derive(Clone, Copy, Debug)]
struct Random {
    n: usize,
    fraction: f64,
    density: f64,
    seed: u64,
}

/// The problems and the cases of one family.
#[derive(Default)]
struct Family {
    problems: Vec<FdProblem>,
    cases: Vec<Case>,
}

impl Family {
    /// The true pattern, the full band of its bandwidth (with `superset`)
    /// and `OMISSIONS` single-entry omissions of a random banded pattern.
    fn random(&mut self, family: &str, spec: Random, superset: bool) {
        let Random {
            n,
            fraction,
            density,
            seed,
        } = spec;
        let width = half_width(n, fraction);
        let rows = random_band_rows(n, width, density, seed);
        let truth = pattern(n, &rows);
        let name = format!("n{n}-h{fraction}-d{density}-s{seed}");
        let index = self.problems.len();
        self.problems
            .push(generic_problem(format!("{family}/{name}"), &rows));
        self.cases.push(case(
            format!("{family}/{name}/true"),
            index,
            truth.clone(),
            &truth,
            "correct",
        ));
        if superset {
            let (lower, upper) = truth.bandwidth();
            self.cases.push(case(
                format!("{family}/{name}/band-{lower}-{upper}"),
                index,
                band(n, lower, upper),
                &truth,
                "correct superset",
            ));
        }
        let off_diagonal: Vec<(usize, usize)> = rows
            .iter()
            .enumerate()
            .flat_map(|(i, row)| row.iter().filter(move |&&k| k != i).map(move |&k| (i, k)))
            .collect();
        let count = OMISSIONS.min(off_diagonal.len());
        for s in 0..count {
            let position = s * off_diagonal.len() / count;
            let (i, k) = off_diagonal[position];
            let mut wrong = rows.clone();
            wrong[i].retain(|&column| column != k);
            self.cases.push(case(
                format!("{family}/{name}/omit-e{position}-r{i}-c{k}"),
                index,
                pattern(n, &wrong),
                &truth,
                "omission",
            ));
        }
    }

    /// The band and the two bands one too narrow of the Fisher-KPP problem
    /// of order `n` and radius `r`.
    fn band(&mut self, family: &str, n: usize, r: usize) {
        let index = self.problems.len();
        self.problems.push(nlkpp_band(n, r, false));
        let truth = band(n, r, r);
        self.cases.push(case(
            format!("{family}/nlkpp-n{n}-r{r}/band-{r}-{r}"),
            index,
            truth.clone(),
            &truth,
            "correct",
        ));
        for (lower, upper) in [(r - 1, r), (r, r - 1)] {
            self.cases.push(case(
                format!("{family}/nlkpp-n{n}-r{r}/band-{lower}-{upper}"),
                index,
                band(n, lower, upper),
                &truth,
                "band one too narrow",
            ));
        }
    }
}

fn family(name: &str) -> Family {
    let mut family = Family::default();
    match name {
        "random" => {
            for n in [60, 96, 128, 200] {
                for fraction in [0.2, 0.3, 0.45] {
                    for density in [0.1, 0.3] {
                        let spec = Random {
                            n,
                            fraction,
                            density,
                            seed: 1,
                        };
                        family.random(name, spec, true);
                    }
                }
            }
        }
        "band" => {
            for n in [60, 128] {
                let mut radii = vec![n / 4, half_width(n, 0.3), half_width(n, 0.45), n - 1];
                radii.dedup();
                for r in radii {
                    family.band(name, n, r);
                }
            }
        }
        "ring" => {
            for n in RING_EQUALITY_ORDERS {
                let index = family.problems.len();
                family.problems.push(ring(n, false, false));
                let truth = ring_pattern(n);
                for i in 0..n {
                    for &k in truth.row(i) {
                        let rows: Vec<Vec<usize>> = (0..n)
                            .map(|row| {
                                let mut columns = truth.row(row).to_vec();
                                if row == i {
                                    columns.retain(|&column| column != k);
                                }
                                columns
                            })
                            .collect();
                        family.cases.push(case(
                            format!("ring/n{n}-true-without-r{i}-c{k}"),
                            index,
                            pattern(n, &rows),
                            &truth,
                            "omission",
                        ));
                    }
                }
            }
        }
        "narrow" => {
            for n in [96, 200] {
                let spec = Random {
                    n,
                    fraction: 0.05,
                    density: 0.3,
                    seed: 1,
                };
                family.random(name, spec, false);
            }
            family.band(name, 128, 8);
        }
        _ => panic!("unknown family {name}"),
    }
    family
}

/// The decision of a check: pass, or the typed error with its row and its
/// column or colour (the record of nodes V26 and V29).
fn decision(result: &Result<usize, AssemblyError>) -> serde_json::Value {
    match result {
        Ok(columns) => serde_json::json!({"outcome": "pass", "columns": columns}),
        Err(error) => {
            let mut colouring = None;
            let (kind, row, column, colour) = match error {
                AssemblyError::UncoveredRowChanged { row, colour } => {
                    ("UncoveredRowChanged", Some(*row), None, Some(*colour))
                }
                AssemblyError::MultiColouringUncoveredRow {
                    row,
                    colouring: index,
                    group,
                } => {
                    colouring = Some(*index);
                    ("MultiColouringUncoveredRow", Some(*row), None, Some(*group))
                }
                AssemblyError::MultiColouringEntryDiffers {
                    row,
                    column,
                    colouring: index,
                    group,
                    ..
                } => {
                    colouring = Some(*index);
                    (
                        "MultiColouringEntryDiffers",
                        Some(*row),
                        Some(*column),
                        Some(*group),
                    )
                }
                AssemblyError::UndeclaredEntry { row, column } => {
                    ("UndeclaredEntry", Some(*row), Some(*column), None)
                }
                AssemblyError::AliasedEntry { row, column, .. } => {
                    ("AliasedEntry", Some(*row), Some(*column), None)
                }
                AssemblyError::NotReproducible { row, column } => {
                    ("NotReproducible", Some(*row), *column, None)
                }
                AssemblyError::Configuration(_) => ("Configuration", None, None, None),
                AssemblyError::StaleRequest => ("StaleRequest", None, None, None),
                AssemblyError::RightHandSide(_) => ("RightHandSide", None, None, None),
            };
            serde_json::json!({
                "outcome": "error",
                "kind": kind,
                "row": row,
                "column": column,
                "colour": colour,
                "colouring": colouring,
                "contract_violation": error.is_contract_violation(),
                "typed": format!("{error:?}"),
            })
        }
    }
}

fn core_error_kind(error: &CoreError) -> &'static str {
    match error {
        CoreError::InvalidInput(_) => "InvalidInput",
        CoreError::NonFinite(_) => "NonFinite",
        CoreError::Dimension(_) => "Dimension",
        CoreError::FatalCallback(_) => "FatalCallback",
        CoreError::LinearSolve(_) => "LinearSolve",
        CoreError::NonlinearSolve(_) => "NonlinearSolve",
        CoreError::Coefficients(_) => "Coefficients",
    }
}

fn with_counters(
    mut record: serde_json::Value,
    counters: &WorkCounters,
    stats: &AssemblyStats,
) -> serde_json::Value {
    record["rhs_calls"] = serde_json::json!(counters.rhs_calls);
    record["rhs_evaluations"] = serde_json::json!(counters.rhs_evaluations);
    record["builds"] = serde_json::json!(stats.builds);
    record["failed_builds"] = serde_json::json!(stats.failed_builds);
    record["assembly_rhs_calls"] = serde_json::json!(stats.assembly_rhs_calls);
    record["verification_rhs_calls"] = serde_json::json!(stats.verification_rhs_calls);
    record["failed_build_rhs_calls"] = serde_json::json!(stats.failed_build_rhs_calls);
    record["verifications"] = serde_json::json!(stats.verifications);
    record["verified_columns"] = serde_json::json!(stats.verified_columns);
    record
}

/// The first linearization of a fresh client with `options`.
fn first_use(fd: &FdProblem, options: AssemblyOptions) -> serde_json::Value {
    let mut client = match AssemblyStageClient::new(fd.n, DENSE_IN_PLACE, options) {
        Ok(client) => client,
        Err(error) => {
            return serde_json::json!({"outcome": "refused", "typed": format!("{error:?}")});
        }
    };
    let handle = client.handle();
    let mut counters = WorkCounters::default();
    let outcome = client.linearize(&fd.problem, fd.span.0, &fd.y0, &mut counters);
    let stats = handle.stats();
    let record = match outcome {
        Ok(()) => serde_json::json!({"outcome": "pass", "columns": stats.verified_columns}),
        Err(error) => match handle.last_error() {
            Some(typed) => {
                let mut record = decision(&Err(typed));
                record["driver_error_kind"] = serde_json::json!(core_error_kind(&error));
                record
            }
            None => serde_json::json!({
                "outcome": "core_error",
                "kind": core_error_kind(&error),
                "message": error.to_string(),
            }),
        },
    };
    with_counters(record, &counters, &stats)
}

/// [`AssemblyStageClient::verify`] at `(t0, y0)` with `check` on a fresh
/// client with `options`.
fn verify(fd: &FdProblem, options: AssemblyOptions, check: CheckColumns) -> serde_json::Value {
    let mut client = match AssemblyStageClient::new(fd.n, DENSE_IN_PLACE, options) {
        Ok(client) => client,
        Err(error) => {
            return serde_json::json!({"outcome": "refused", "typed": format!("{error:?}")});
        }
    };
    let mut counters = WorkCounters::default();
    let result = client.verify(&fd.problem, fd.span.0, &fd.y0, check, &mut counters);
    let stats = client.handle().stats();
    with_counters(decision(&result), &counters, &stats)
}

/// The forward-difference increment of `crates/vigilode/src/fd.rs:443-455`.
fn increment(yj: f64) -> (f64, f64) {
    let raw = f64::EPSILON.sqrt() * yj.abs().max(1.0);
    let forward = yj + raw;
    let shifted = if forward.is_finite() {
        forward
    } else {
        yj - raw
    };
    (shifted, shifted - yj)
}

/// Row `i` at `y0` with the columns `shifted` carrying their increments, and
/// the same with column `k` unshifted.
fn row_with_and_without(fd: &FdProblem, shifted: &[usize], i: usize, k: usize) -> (f64, f64) {
    let mut state = fd.y0.clone();
    for &j in shifted {
        state[j] = increment(fd.y0[j]).0;
    }
    let mut out = vec![0.0; fd.n];
    let mut counters = WorkCounters::default();
    fd.problem
        .eval_rhs_into(fd.span.0, &state, &mut out, &mut counters)
        .unwrap();
    let with = out[i];
    state[k] = fd.y0[k];
    out.fill(0.0);
    fd.problem
        .eval_rhs_into(fd.span.0, &state, &mut out, &mut counters)
        .unwrap();
    (with, out[i])
}

/// The rounding evidence of a miss of `check` on `case` (nodes V26 and V29):
/// for every omitted entry `(i, k)`, the bits of row `i` with and without the
/// shift of `k` at every state the check evaluated with `k` shifted.
fn miss_evidence(fd: &FdProblem, case: &Case, check: CheckColumns) -> serde_json::Value {
    let (colour, _) = colour_columns(&case.declared);
    let multi = matches!(
        check,
        CheckColumns::MultiColouring | CheckColumns::MultiColouringPooled
    );
    let all = matches!(check, CheckColumns::All | CheckColumns::AllPooled);
    let plan = multi.then(|| CheckColourings::of(&case.declared).unwrap());
    let mut entries = Vec::new();
    let mut all_limited = true;
    for &(i, k) in case.omitted.iter().take(EVIDENCE_ENTRIES_MAX) {
        let group: Vec<usize> = (0..fd.n).filter(|&j| colour[j] == colour[k]).collect();
        let mut states = Vec::new();
        let (with, without) = row_with_and_without(fd, &group, i, k);
        states.push(serde_json::json!({
            "state": format!("build: group {} of the client's colouring ({} columns)", colour[k], group.len()),
            "row_with_shift": with,
            "row_without_shift": without,
            "bits_equal": with.to_bits() == without.to_bits(),
        }));
        if all {
            let (with, without) = row_with_and_without(fd, &[k], i, k);
            states.push(serde_json::json!({
                "state": format!("check: column {k} alone"),
                "row_with_shift": with,
                "row_without_shift": without,
                "bits_equal": with.to_bits() == without.to_bits(),
            }));
        }
        if let Some(plan) = &plan {
            for r in 0..plan.colourings() {
                let g = plan.colour(r, k);
                let columns = plan.columns(r, g);
                let (with, without) = row_with_and_without(fd, &columns, i, k);
                states.push(serde_json::json!({
                    "state": format!("check: group {g} of further colouring {} ({} columns)", r + 1, columns.len()),
                    "row_with_shift": with,
                    "row_without_shift": without,
                    "bits_equal": with.to_bits() == without.to_bits(),
                }));
            }
        }
        let limited = states.iter().all(|state| state["bits_equal"] == true);
        all_limited &= limited;
        entries.push(serde_json::json!({
            "entry": [i, k],
            "rounding_limited": limited,
            "states": states,
        }));
    }
    serde_json::json!({
        "check": format!("{check:?}"),
        "omitted_entries": case.omitted.len(),
        "evaluated_entries": entries.len(),
        "complete": entries.len() == case.omitted.len(),
        "rounding_limited": all_limited && entries.len() == case.omitted.len(),
        "entries": entries,
    })
}

fn run_family(name: &str, executions: &[Execution]) {
    let start = std::time::Instant::now();
    let Family { problems, cases } = family(name);
    let mut passes = std::collections::BTreeMap::<String, usize>::new();
    for case in &cases {
        let fd = &problems[case.problem];
        let wrong = !case.omitted.is_empty();
        let mut decisions = serde_json::Map::new();
        let mut evidence = serde_json::Map::new();
        for &execution in executions {
            let options = default_options(&case.declared, execution);
            let check = options.first_use_check;
            let first = first_use(fd, options.clone());
            let checked = verify(fd, options.clone(), check);
            let unchecked = options.with_first_use_check(CheckColumns::None);
            let explicit_all = verify(fd, unchecked.clone(), CheckColumns::All);
            let explicit_multi = verify(fd, unchecked, CheckColumns::MultiColouring);
            for (path, record) in [
                ("first_use", &first),
                ("verify", &checked),
                ("explicit_all", &explicit_all),
                ("explicit_multi_colouring", &explicit_multi),
            ] {
                if record["outcome"] == "pass" {
                    *passes
                        .entry(format!("{}:{path}", execution.name()))
                        .or_default() += 1;
                }
            }
            if checked["outcome"] == "pass" && wrong {
                evidence.insert(execution.name(), miss_evidence(fd, case, check));
            }
            decisions.insert(
                execution.name(),
                serde_json::json!({
                    "first_use_check": format!("{check:?}"),
                    "first_use": first,
                    "verify": checked,
                    "explicit_all": explicit_all,
                    "explicit_multi_colouring": explicit_multi,
                }),
            );
        }
        let colourings = colourings_record(&case.declared);
        let total_groups = colourings["total_groups"].as_u64().unwrap() as usize;
        println!(
            "V31_CASE_JSON {}",
            serde_json::json!({
                "family": name,
                "case": case.id,
                "problem": fd.id,
                "n": fd.n,
                "kind": case.kind,
                "backend": "dense_lu(in-place)",
                "declared_nnz": case.declared.nnz(),
                "bandwidth": case.declared.bandwidth(),
                "client_colours": colour_columns(&case.declared).1,
                "correct": !wrong,
                "omitted_entries": case.omitted.len(),
                "omitted_first": case.omitted.iter().take(8).map(|&(i, k)| [i, k]).collect::<Vec<_>>(),
                "check_colourings": colourings,
                "rule_picks_all": total_groups >= fd.n,
                "executions": decisions,
                "miss_evidence": evidence,
            })
        );
    }
    println!(
        "V31_FAMILY_JSON {}",
        serde_json::json!({
            "family": name,
            "cases": cases.len(),
            "executions": executions.iter().map(|execution| execution.name()).collect::<Vec<_>>(),
            "passes": passes,
            "wall_s_diagnostic": start.elapsed().as_secs_f64(),
        })
    );
}

#[test]
#[ignore = "recording program of loop node V31; run it alone with V31_MODE set"]
fn v31_record() {
    let mode = std::env::var("V31_MODE").unwrap_or_default();
    let families = std::env::var("V31_FAMILY").unwrap_or_default();
    let executions: Vec<Execution> = std::env::var("V31_EXECUTIONS")
        .unwrap_or_else(|_| "seq,t6".into())
        .split(',')
        .filter(|text| !text.is_empty())
        .map(Execution::parse)
        .collect();
    // The default of a few fixed patterns: the probe of the V29 program
    // (band (1, 1) of order 4), a band of order 16 and the ring of order 16.
    let probes: Vec<serde_json::Value> = [
        ("banded(4, 1, 1)", band(4, 1, 1)),
        ("banded(16, 2, 2)", band(16, 2, 2)),
        ("banded(60, 27, 27)", band(60, 27, 27)),
    ]
    .iter()
    .map(|(name, declared)| {
        let colourings = colourings_record(declared);
        serde_json::json!({
            "pattern": name,
            "n": declared.n(),
            "total_groups": colourings["total_groups"],
            "default_check": format!("{:?}", AssemblyOptions::coloured(declared.clone()).first_use_check),
        })
    })
    .collect();
    println!(
        "V31_META_JSON {}",
        serde_json::json!({
            "mode": mode,
            "families": families,
            "executions": executions.iter().map(|execution| execution.name()).collect::<Vec<_>>(),
            "default_dense": format!("{:?}", AssemblyOptions::dense().first_use_check),
            "default_coloured_probes": probes,
            "cpus_allowed_list": std::fs::read_to_string("/proc/self/status")
                .ok()
                .and_then(|status| {
                    status
                        .lines()
                        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
                        .map(|list| list.trim().to_string())
                }),
        })
    );
    match mode.as_str() {
        "survey" => survey(),
        "wide" => {
            for name in families.split(',').filter(|name| !name.is_empty()) {
                run_family(name, &executions);
            }
        }
        _ => println!(
            "V31_USAGE set V31_MODE (survey, or wide with V31_FAMILY and V31_EXECUTIONS); see the \
             file header"
        ),
    }
}
