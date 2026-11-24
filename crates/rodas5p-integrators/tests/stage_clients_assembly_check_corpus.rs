//! Corpus program of loop node V26 (the full pattern check of the assembly
//! client, owner decision OD-L1-16). One ignored test driven by environment
//! variables; every result is one JSON object per line behind a tag. The
//! corpus and the arms are frozen in
//! `research/production_loop_20261116/assembly_check/PREDECLARED.json`, the
//! logs are read by `parse_results.py` of that directory.
//!
//! ```text
//! V26_MODE=corpus V26_GROUP=<group> V26_ARMS=<arm>@<execution>,...
//! cargo test --release -p rodas5p-integrators --test stage_clients_assembly_check_corpus -- --ignored --nocapture
//! ```
//!
//! Groups: `controls` (the seven negative controls of node V08, which use
//! four problem and pattern pairs), `ring` (the periodic ring declared
//! tridiagonal), `omit128`, `omit1024`, `omit131072` (the true Brusselator
//! pattern minus one entry), `narrow` (bands one too narrow), `correct` (the
//! true patterns and the patterns of the four cost cells).
//!
//! Arms: `S8` (`CheckColumns::Sample(8)`), `FA` (`CheckColumns::All`),
//! `FA_P` (`CheckColumns::AllPooled`), `MC` (`CheckColumns::MultiColouring`),
//! `MC_P` (`CheckColumns::MultiColouringPooled`); the last three exist from
//! the client edit of node V26 on (commit A recorded `S8` and `FA` before it).
//! Executions: `seq` (sequential client) and `t<P>` (threaded client with
//! `P` workers).
//!
//! The decision of an arm on a case is the outcome of
//! [`AssemblyStageClient::verify`] at `(t0, y0)` on a fresh client: the first
//! linearization of a driver run is the same build followed by the same
//! check at the same state.
//!
//! Evidence of a miss: when an arm passes a case whose declaration omits
//! entries of the true pattern, every omitted entry `(i, k)` is evaluated
//! directly at every state the arm evaluated with column `k` shifted, with
//! and without that shift, and the bits of row `i` are compared (the
//! rounding-limited rule of the pre-declaration).

use std::time::Instant;

use rodas5p_core::WorkCounters;
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyError, AssemblyOptions, AssemblyStageClient, CheckColourings, CheckColumns,
    ColumnRoute, LinearBackend, RhsPurity, SparsityPattern, colour_columns,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{
    FdProblem, brusselator, impure, lower_coupled_chain, nlkpp_band, nlkpp_dense, ring,
    ring_pattern, upper_bidiagonal,
};

const DENSE_IN_PLACE: LinearBackend = LinearBackend::DenseLu(DenseLuMode::InPlace);
const RING_ORDERS: [usize; 13] = [12, 13, 14, 15, 16, 32, 33, 34, 64, 100, 127, 128, 1000];
const LARGE_OMISSIONS: usize = 200;
/// Omitted entries of one case for which a miss is evaluated in full; the
/// rest of a larger list is only counted.
const EVIDENCE_ENTRIES_MAX: usize = 64;

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

/// Where the check runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Execution {
    Sequential,
    Threaded(usize),
}

/// One arm: a check and the execution of the client.
#[derive(Clone, Debug)]
struct ArmSpec {
    label: String,
    name: String,
    check: CheckColumns,
    execution: Execution,
}

fn parse_arm(text: &str) -> ArmSpec {
    let (name, execution) = text
        .split_once('@')
        .unwrap_or_else(|| panic!("an arm is <name>@<execution>, got {text}"));
    let check = match name {
        "S8" => CheckColumns::Sample(AssemblyOptions::DEFAULT_CHECK_COLUMNS),
        "FA" => CheckColumns::All,
        "FA_P" => CheckColumns::AllPooled,
        "MC" => CheckColumns::MultiColouring,
        "MC_P" => CheckColumns::MultiColouringPooled,
        _ => panic!("unknown arm {name}"),
    };
    let execution = match execution {
        "seq" => Execution::Sequential,
        _ => Execution::Threaded(
            execution
                .strip_prefix('t')
                .and_then(|workers| workers.parse().ok())
                .unwrap_or_else(|| panic!("unknown execution {execution}")),
        ),
    };
    ArmSpec {
        label: text.to_string(),
        name: name.to_string(),
        check,
        execution,
    }
}

/// One case of the corpus.
struct Case {
    id: String,
    /// Index into the problems of the group.
    problem: usize,
    route: ColumnRoute,
    backend: LinearBackend,
    backend_name: &'static str,
    /// Entries of the true pattern the declaration omits; `None` for a
    /// right-hand side without a pattern (the impure control).
    omitted: Option<Vec<(usize, usize)>>,
    /// The negative controls of node V08 this case carries.
    controls: Vec<&'static str>,
    note: &'static str,
}

/// The true pattern of the Brusselator of `cells` cells (unknowns
/// interleaved): `u_i` reads `u_{i-1}`, `u_i`, `v_i`, `u_{i+1}`; `v_i` reads
/// `v_{i-1}`, `u_i`, `v_i`, `v_{i+1}`.
fn brusselator_rows(cells: usize) -> Vec<Vec<usize>> {
    let n = 2 * cells;
    let mut rows = vec![Vec::new(); n];
    for i in 0..cells {
        let (u, v) = (2 * i, 2 * i + 1);
        if i > 0 {
            rows[u].push(u - 2);
            rows[v].push(v - 2);
        }
        rows[u].extend([u, v]);
        rows[v].extend([u, v]);
        if i + 1 < cells {
            rows[u].push(u + 2);
            rows[v].push(v + 2);
        }
    }
    for row in &mut rows {
        row.sort_unstable();
    }
    rows
}

/// The true pattern of [`lower_coupled_chain`]: row 0 reads `y_0, y_1`,
/// rows `1 ..= n - 2` read `y_{i-1}, y_i, y_{i+1}`, row `n - 1` reads `y_{n-1}`.
fn chain_rows(n: usize) -> Vec<Vec<usize>> {
    (0..n)
        .map(|i| {
            if i == 0 {
                vec![0, 1]
            } else if i + 1 < n {
                vec![i - 1, i, i + 1]
            } else {
                vec![i]
            }
        })
        .collect()
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

fn pattern(n: usize, rows: &[Vec<usize>]) -> SparsityPattern {
    SparsityPattern::from_rows(n, rows).unwrap()
}

fn band(n: usize, lower: usize, upper: usize) -> SparsityPattern {
    SparsityPattern::banded(n, lower, upper).unwrap()
}

fn coloured_case(
    id: String,
    problem: usize,
    declared: SparsityPattern,
    truth: &SparsityPattern,
    backend: (LinearBackend, &'static str),
    note: &'static str,
) -> Case {
    Case {
        id,
        problem,
        omitted: Some(omitted(truth, &declared)),
        route: ColumnRoute::Coloured(declared),
        backend: backend.0,
        backend_name: backend.1,
        controls: Vec::new(),
        note,
    }
}

fn tight(lower: usize, upper: usize) -> (LinearBackend, &'static str) {
    (LinearBackend::BandedTight { lower, upper }, "banded_tight")
}

fn dense_lu() -> (LinearBackend, &'static str) {
    (DENSE_IN_PLACE, "dense_lu(in-place)")
}

/// The declared controls of node V08: (1) ring of 12 declared tridiagonal,
/// sequential and 3 workers; (2) the chain declared upper bidiagonal,
/// sequential and 3 workers; (3) the impure right-hand side, dense and
/// tridiagonal, 3 workers; (4) the chain declared upper bidiagonal with a
/// full check, sequential.
fn controls() -> (Vec<FdProblem>, Vec<Case>) {
    let problems = vec![ring(12, false, false), lower_coupled_chain(24), impure(16)];
    let mut ring_case = coloured_case(
        "controls/ring12-tridiagonal".into(),
        0,
        band(12, 1, 1),
        &ring_pattern(12),
        dense_lu(),
        "V08 control 1: omitted periodic corners reach rows their colour does not cover",
    );
    ring_case.controls = vec!["1 seq S8", "1 t3 S8"];
    let mut chain_case = coloured_case(
        "controls/chain24-upper-bidiagonal".into(),
        1,
        upper_bidiagonal(24),
        &pattern(24, &chain_rows(24)),
        dense_lu(),
        "V08 controls 2 and 4: the omitted lower diagonal is aliased into declared entries",
    );
    chain_case.controls = vec!["2 seq S8", "2 t3 S8", "4 seq FA"];
    let impure_dense = Case {
        id: "controls/impure16-dense".into(),
        problem: 2,
        route: ColumnRoute::Dense,
        backend: DENSE_IN_PLACE,
        backend_name: "dense_lu(in-place)",
        omitted: None,
        controls: vec!["3 t3 S8 dense"],
        note: "V08 control 3: the right-hand side is not a function of its input",
    };
    let impure_coloured = Case {
        id: "controls/impure16-tridiagonal".into(),
        problem: 2,
        route: ColumnRoute::Coloured(band(16, 1, 1)),
        backend: DENSE_IN_PLACE,
        backend_name: "dense_lu(in-place)",
        omitted: None,
        controls: vec!["3 t3 S8 coloured"],
        note: "V08 control 3: the right-hand side is not a function of its input",
    };
    (
        problems,
        vec![ring_case, chain_case, impure_dense, impure_coloured],
    )
}

fn ring_group() -> (Vec<FdProblem>, Vec<Case>) {
    let problems: Vec<FdProblem> = RING_ORDERS.iter().map(|&n| ring(n, false, false)).collect();
    let cases = RING_ORDERS
        .iter()
        .enumerate()
        .map(|(index, &n)| {
            coloured_case(
                format!("ring/n{n}-tridiagonal"),
                index,
                band(n, 1, 1),
                &ring_pattern(n),
                dense_lu(),
                "periodic ring declared tridiagonal (V08 review)",
            )
        })
        .collect();
    (problems, cases)
}

/// The true Brusselator pattern minus one entry: every entry, or the entries
/// `floor(s nnz / count)` for `s = 0 .. count` in row-major order.
fn omission_group(cells: usize, count: Option<usize>) -> (Vec<FdProblem>, Vec<Case>) {
    let n = 2 * cells;
    let rows = brusselator_rows(cells);
    let truth = pattern(n, &rows);
    let entries: Vec<(usize, usize)> = rows
        .iter()
        .enumerate()
        .flat_map(|(i, row)| row.iter().map(move |&k| (i, k)))
        .collect();
    let nnz = entries.len();
    let chosen: Vec<usize> = match count {
        None => (0..nnz).collect(),
        Some(count) => (0..count).map(|s| s * nnz / count).collect(),
    };
    let cases = chosen
        .into_iter()
        .map(|index| {
            let (i, k) = entries[index];
            let mut wrong = rows.clone();
            wrong[i].retain(|&column| column != k);
            coloured_case(
                format!("omit{n}/e{index}-r{i}-c{k}"),
                0,
                pattern(n, &wrong),
                &truth,
                tight(2, 2),
                "true Brusselator pattern minus one entry",
            )
        })
        .collect();
    (vec![brusselator(cells, false)], cases)
}

fn narrow_group() -> (Vec<FdProblem>, Vec<Case>) {
    let problems = vec![
        nlkpp_band(4096, 24, false),
        brusselator(64, false),
        brusselator(512, false),
    ];
    let mut cases = Vec::new();
    let truth = band(4096, 24, 24);
    for (lower, upper) in [(23, 24), (24, 23)] {
        cases.push(coloured_case(
            format!("narrow/nlkpp-band-n4096-r24-declared-{lower}-{upper}"),
            0,
            band(4096, lower, upper),
            &truth,
            tight(lower, upper),
            "band one too narrow",
        ));
    }
    for (index, cells) in [(1, 64), (2, 512)] {
        let n = 2 * cells;
        let truth = pattern(n, &brusselator_rows(cells));
        for (lower, upper) in [(1, 2), (2, 1)] {
            cases.push(coloured_case(
                format!("narrow/bruss1d-n{n}-declared-{lower}-{upper}"),
                index,
                band(n, lower, upper),
                &truth,
                tight(lower, upper),
                "band one too narrow",
            ));
        }
    }
    (problems, cases)
}

fn correct_group() -> (Vec<FdProblem>, Vec<Case>) {
    let mut problems = Vec::new();
    let mut cases = Vec::new();
    for &n in &RING_ORDERS {
        let truth = ring_pattern(n);
        cases.push(coloured_case(
            format!("correct/ring-n{n}-true"),
            problems.len(),
            truth.clone(),
            &truth,
            dense_lu(),
            "true pattern",
        ));
        problems.push(ring(n, false, false));
    }
    let chain = pattern(24, &chain_rows(24));
    cases.push(coloured_case(
        "correct/chain-n24-true".into(),
        problems.len(),
        chain.clone(),
        &chain,
        dense_lu(),
        "true pattern",
    ));
    problems.push(lower_coupled_chain(24));
    for cells in [64, 512, 65536] {
        let n = 2 * cells;
        let truth = pattern(n, &brusselator_rows(cells));
        cases.push(coloured_case(
            format!("correct/bruss1d-n{n}-true"),
            problems.len(),
            truth.clone(),
            &truth,
            tight(2, 2),
            "true pattern",
        ));
        cases.push(coloured_case(
            format!("correct/bruss1d-n{n}-band-2-2"),
            problems.len(),
            band(n, 2, 2),
            &truth,
            tight(2, 2),
            "declared full band (2, 2), a superset of the true pattern (cost cells bruss1d-n131072 and bruss1d-n128-coloured)",
        ));
        problems.push(brusselator(cells, false));
    }
    let truth = band(4096, 24, 24);
    cases.push(coloured_case(
        "correct/nlkpp-band-n4096-r24-band-24-24".into(),
        problems.len(),
        truth.clone(),
        &truth,
        tight(24, 24),
        "true pattern, cost cell nlkpp-band-n4096-r24",
    ));
    problems.push(nlkpp_band(4096, 24, false));
    for (fd, note) in [
        (
            nlkpp_dense(400, false),
            "dense route, cost cell nlkpp-dense-n400",
        ),
        (
            brusselator(64, false),
            "dense route, cost cell bruss1d-n128-dense",
        ),
    ] {
        let mode =
            AssemblyStageClient::protected_dense_mode(&fd.problem, fd.span.0, &fd.y0).unwrap();
        cases.push(Case {
            id: format!("correct/{}-dense-route", fd.id),
            problem: problems.len(),
            route: ColumnRoute::Dense,
            backend: LinearBackend::DenseLu(mode),
            backend_name: match mode {
                DenseLuMode::InPlace => "dense_lu(in-place)",
                DenseLuMode::Faer => "dense_lu(faer)",
            },
            omitted: Some(Vec::new()),
            controls: Vec::new(),
            note,
        });
        problems.push(fd);
    }
    (problems, cases)
}

fn group(name: &str) -> (Vec<FdProblem>, Vec<Case>) {
    match name {
        "controls" => controls(),
        "ring" => ring_group(),
        "omit128" => omission_group(64, None),
        "omit1024" => omission_group(512, None),
        "omit131072" => omission_group(65536, Some(LARGE_OMISSIONS)),
        "narrow" => narrow_group(),
        "correct" => correct_group(),
        _ => panic!("unknown group {name}"),
    }
}

/// The decision of a check: pass, or the typed error with its row and its
/// column or colour.
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

fn options(route: &ColumnRoute, execution: Execution) -> AssemblyOptions {
    let options = match route {
        ColumnRoute::Dense => AssemblyOptions::dense(),
        ColumnRoute::Coloured(pattern) => AssemblyOptions::coloured(pattern.clone()),
    };
    let options = match execution {
        Execution::Sequential => options,
        Execution::Threaded(workers) => options.threaded(workers, RhsPurity::pure_and_reentrant()),
    };
    options.with_first_use_check(CheckColumns::None)
}

/// One build and one check on a fresh client.
fn run_arm(fd: &FdProblem, case: &Case, arm: &ArmSpec) -> serde_json::Value {
    let start = Instant::now();
    let client = AssemblyStageClient::new(fd.n, case.backend, options(&case.route, arm.execution));
    let mut client = match client {
        Ok(client) => client,
        Err(error) => {
            return serde_json::json!({
                "outcome": "refused",
                "typed": format!("{error:?}"),
            });
        }
    };
    let mut counters = WorkCounters::default();
    let result = client.verify(&fd.problem, fd.span.0, &fd.y0, arm.check, &mut counters);
    let wall_ns = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let stats = client.handle().stats();
    let mut record = decision(&result);
    record["rhs_calls"] = serde_json::json!(counters.rhs_calls);
    record["assembly_rhs_calls"] = serde_json::json!(stats.assembly_rhs_calls);
    record["verification_rhs_calls"] = serde_json::json!(stats.verification_rhs_calls);
    record["failed_build_rhs_calls"] = serde_json::json!(stats.failed_build_rhs_calls);
    record["verified_columns"] = serde_json::json!(stats.verified_columns);
    record["wall_ns_diagnostic"] = serde_json::json!(wall_ns);
    record
}

/// The columns an arm evaluated alone at `y0` (sample or all), besides the
/// groups of the build.
fn single_columns(check: CheckColumns, n: usize) -> Vec<usize> {
    match check {
        CheckColumns::None => Vec::new(),
        CheckColumns::Sample(count) => {
            let count = count.min(n);
            (0..count)
                .map(|sample| {
                    if count == n {
                        sample
                    } else {
                        (2 * sample + 1) * n / (2 * count)
                    }
                })
                .collect()
        }
        CheckColumns::All | CheckColumns::AllPooled => (0..n).collect(),
        CheckColumns::MultiColouring | CheckColumns::MultiColouringPooled => Vec::new(),
    }
}

/// Whether an arm is a multi-colouring check.
fn multi_colouring(check: CheckColumns) -> bool {
    matches!(
        check,
        CheckColumns::MultiColouring | CheckColumns::MultiColouringPooled
    )
}

/// Row `i` at `y0` with the columns `shifted` carrying their increments,
/// and the same with column `k` unshifted.
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

/// The rounding-limited evidence of a miss of `arm` on `case`: for every
/// omitted entry `(i, k)` (at most [`EVIDENCE_ENTRIES_MAX`]), the bits of row
/// `i` with and without the shift of `k` at every state the arm evaluated
/// with `k` shifted (the group of `k` in the client's colouring; the
/// column `k` alone if the arm checked it; the group of `k` in every
/// further colouring of a multi-colouring check).
fn miss_evidence(
    fd: &FdProblem,
    case: &Case,
    arm: &ArmSpec,
    missing: &[(usize, usize)],
) -> serde_json::Value {
    let ColumnRoute::Coloured(declared) = &case.route else {
        return serde_json::json!({"note": "dense route: no omitted entries"});
    };
    let (colour, _) = colour_columns(declared);
    let singles = single_columns(arm.check, fd.n);
    let plan = multi_colouring(arm.check).then(|| CheckColourings::of(declared).unwrap());
    let mut entries = Vec::new();
    let mut all_limited = true;
    for &(i, k) in missing.iter().take(EVIDENCE_ENTRIES_MAX) {
        let group: Vec<usize> = (0..fd.n).filter(|&j| colour[j] == colour[k]).collect();
        let mut states = Vec::new();
        let (with, without) = row_with_and_without(fd, &group, i, k);
        states.push(serde_json::json!({
            "state": format!("build: group {} of the client's colouring ({} columns)", colour[k], group.len()),
            "row_with_shift": with,
            "row_without_shift": without,
            "bits_equal": with.to_bits() == without.to_bits(),
        }));
        if singles.binary_search(&k).is_ok() {
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
        "arm": arm.label,
        "omitted_entries": missing.len(),
        "evaluated_entries": entries.len(),
        "complete": entries.len() == missing.len(),
        "rounding_limited": all_limited && entries.len() == missing.len(),
        "entries": entries,
    })
}

/// The further colourings of the multi-colouring check of `pattern`.
fn colourings_record(pattern: &SparsityPattern) -> serde_json::Value {
    match CheckColourings::of(pattern) {
        Ok(plan) => serde_json::json!({
            "construction": format!("{:?}", plan.construction()),
            "colourings": plan.colourings(),
            "groups": (0..plan.colourings()).map(|r| plan.groups(r)).collect::<Vec<_>>(),
            "total_groups": plan.total_groups(),
            "total_groups_band": plan.total_groups_band(),
            "total_groups_refinement": plan.total_groups_refinement(),
        }),
        Err(error) => serde_json::json!({"error": format!("{error:?}")}),
    }
}

fn run_group(name: &str, arms: &[ArmSpec]) {
    let start = Instant::now();
    let (problems, cases) = group(name);
    let mut passes = std::collections::BTreeMap::<String, usize>::new();
    for case in &cases {
        let fd = &problems[case.problem];
        let mut decisions = serde_json::Map::new();
        let mut evidence = serde_json::Map::new();
        for arm in arms {
            let record = run_arm(fd, case, arm);
            let passed = record["outcome"] == "pass";
            if passed {
                *passes.entry(arm.label.clone()).or_default() += 1;
            }
            let wrong = case
                .omitted
                .as_ref()
                .is_some_and(|missing| !missing.is_empty());
            if passed && wrong && arm.name != "S8" {
                let missing = case.omitted.as_ref().unwrap();
                evidence.insert(arm.label.clone(), miss_evidence(fd, case, arm, missing));
            }
            decisions.insert(arm.label.clone(), record);
        }
        let with_multi = arms.iter().any(|arm| multi_colouring(arm.check));
        let check_colourings = match &case.route {
            ColumnRoute::Coloured(pattern) if with_multi => colourings_record(pattern),
            _ => serde_json::Value::Null,
        };
        let (declared_nnz, colours) = match &case.route {
            ColumnRoute::Dense => (None, None),
            ColumnRoute::Coloured(pattern) => {
                (Some(pattern.nnz()), Some(colour_columns(pattern).1))
            }
        };
        println!(
            "V26_CASE_JSON {}",
            serde_json::json!({
                "group": name,
                "case": case.id,
                "problem": fd.id,
                "n": fd.n,
                "route": if matches!(case.route, ColumnRoute::Dense) { "dense" } else { "coloured" },
                "backend": case.backend_name,
                "declared_nnz": declared_nnz,
                "client_colours": colours,
                "pure": case.omitted.is_some(),
                "correct": case.omitted.as_ref().is_some_and(Vec::is_empty),
                "omitted_entries": case.omitted.as_ref().map(Vec::len),
                "omitted_first": case.omitted.as_ref().map(|missing| missing.iter().take(8).map(|&(i, k)| [i, k]).collect::<Vec<_>>()),
                "controls": case.controls,
                "note": case.note,
                "arms": decisions,
                "check_colourings": check_colourings,
                "miss_evidence": evidence,
            })
        );
    }
    println!(
        "V26_GROUP_JSON {}",
        serde_json::json!({
            "group": name,
            "cases": cases.len(),
            "arms": arms.iter().map(|arm| arm.label.clone()).collect::<Vec<_>>(),
            "passes_per_arm": passes,
            "wall_s_diagnostic": start.elapsed().as_secs_f64(),
        })
    );
}

#[test]
#[ignore = "corpus program of loop node V26; run it alone with V26_MODE set"]
fn v26_corpus() {
    let mode = std::env::var("V26_MODE").unwrap_or_default();
    let group = std::env::var("V26_GROUP").unwrap_or_default();
    let arms: Vec<ArmSpec> = std::env::var("V26_ARMS")
        .unwrap_or_default()
        .split(',')
        .filter(|text| !text.is_empty())
        .map(parse_arm)
        .collect();
    println!(
        "V26_META_JSON {}",
        serde_json::json!({
            "mode": mode,
            "group": group,
            "arms": arms.iter().map(|arm| arm.label.clone()).collect::<Vec<_>>(),
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
        "corpus" => {
            for name in group.split(',') {
                run_group(name, &arms);
            }
        }
        _ => println!("V26_USAGE set V26_MODE=corpus, V26_GROUP and V26_ARMS; see the file header"),
    }
}
