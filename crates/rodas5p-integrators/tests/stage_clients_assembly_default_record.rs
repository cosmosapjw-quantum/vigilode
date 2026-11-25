//! Recording program of loop node V29 (owner decision OD-L1-24: the default
//! first-use check of the assembly client). One ignored test driven by
//! environment variables; every result is one JSON object per line behind a
//! tag. The logs are read by `parse_results.py` of
//! `research/production_loop_20261116/assembly_default/`.
//!
//! ```text
//! V29_MODE=identity V29_CELL=<cell>
//!     the identity cells of node V08, every client with the default options
//! V29_MODE=corpus   V29_GROUP=<group>[,<group>..] V29_EXECUTIONS=<seq|tP>[,..]
//!     the decision of the default first-use check on the corpus of node V26
//! cargo test --release -p rodas5p-integrators --test stage_clients_assembly_default_record -- --ignored --nocapture
//! ```
//!
//! Every client is built with the options a caller gets without naming a
//! check: `AssemblyOptions::dense()` or `AssemblyOptions::coloured(pattern)`,
//! then `threaded(..)` and `with_fingerprints()` where an arm asks for them;
//! `with_first_use_check` is never called. The check in use is read back from
//! `AssemblyOptions::first_use_check` and recorded. The program is run
//! unchanged before the change of the default (commit A of node V29) and after
//! it, so the two recordings differ only by the code of the client.
//!
//! Identity cells (`research/production_loop_20261116/jacobian_assembly/
//! PREDECLARED.json`, node V08): `nlkpp-dense-n400` and `bruss1d-n128-dense`
//! (dense route), `nlkpp-band-n4096-r24` (coloured route, back ends
//! `banded_tight` and `banded_protected`), `bruss1d-n131072` and
//! `bruss1d-n128-coloured` (coloured route, `banded_tight`); arms `prot` (the
//! protected driver with the reference finite differences) and `seq`, `t1`,
//! `t2`, `t4`, `t6`; the tolerances and the run length (48 attempts) of node
//! V08.
//!
//! Corpus: the cases of `stage_clients_assembly_check_corpus.rs` (node V26,
//! `Case` to `group`), transcribed here; the parser compares the case ids,
//! orders, declared entries and omissions with the V26 logs. The decision of
//! the default on a case is recorded twice, each on a fresh client:
//! `first_use`, the first `linearize` at `(t0, y0)` (what a driver run does:
//! the build, the first-use check where the client makes one, the copy into
//! the back end), and `verify`, [`AssemblyStageClient::verify`] at `(t0, y0)`
//! with the check of the default options (the decision convention of node
//! V26). A wrong case that a multi-colouring or full default check passes gets
//! the rounding evidence of node V26 (the bits of the row with and without the
//! shift of the omitted column at every state the check evaluated).

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyError, AssemblyOptions, AssemblyStageClient, AssemblyStats, CheckColourings,
    CheckColumns, ColumnRoute, FINGERPRINT_SEED, LinearBackend, RhsPurity, SparsityPattern,
    colour_columns, fingerprint,
};
use rodas5p_integrators::{
    DriverPartial, OutputSchedule, Rodas5pFastResult, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{
    FdProblem, ProtectedRun, RunConfig, brusselator, declared, final_fingerprint, final_state,
    impure, lower_coupled_chain, nlkpp_band, nlkpp_dense, ring, ring_pattern, run_protected_banded,
    run_protected_dense, upper_bidiagonal,
};

const DENSE_IN_PLACE: LinearBackend = LinearBackend::DenseLu(DenseLuMode::InPlace);
const RING_ORDERS: [usize; 13] = [12, 13, 14, 15, 16, 32, 33, 34, 64, 100, 127, 128, 1000];
const LARGE_OMISSIONS: usize = 200;
/// Omitted entries of one case for which a miss is evaluated in full.
const EVIDENCE_ENTRIES_MAX: usize = 64;

fn hex(value: u64) -> String {
    format!("{value:016x}")
}

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

/// The options a caller gets for `route` without naming a check.
fn default_options(
    route: &ColumnRoute,
    execution: Execution,
    fingerprints: bool,
) -> AssemblyOptions {
    let options = match route {
        ColumnRoute::Dense => AssemblyOptions::dense(),
        ColumnRoute::Coloured(pattern) => AssemblyOptions::coloured(pattern.clone()),
    };
    let options = match execution {
        Execution::Sequential => options,
        Execution::Threaded(workers) => options.threaded(workers, RhsPurity::pure_and_reentrant()),
    };
    if fingerprints {
        options.with_fingerprints()
    } else {
        options
    }
}

fn stats_record(stats: &AssemblyStats) -> serde_json::Value {
    serde_json::json!({
        "builds": stats.builds,
        "failed_builds": stats.failed_builds,
        "assembly_rhs_calls": stats.assembly_rhs_calls,
        "failed_build_rhs_calls": stats.failed_build_rhs_calls,
        "verifications": stats.verifications,
        "verified_columns": stats.verified_columns,
        "verification_rhs_calls": stats.verification_rhs_calls,
        "state_fingerprint": hex(stats.state_fingerprint),
        "jacobian_fingerprint": hex(stats.jacobian_fingerprint),
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

// ---------------------------------------------------------------------------
// Identity cells of node V08.
// ---------------------------------------------------------------------------

/// A cell of node V08: its problem, route and back end.
struct Cell {
    fd: FdProblem,
    route: ColumnRoute,
    backend: LinearBackend,
    backend_name: String,
    /// The band of the protected banded driver; `None`: the dense driver.
    band: Option<(usize, usize)>,
}

fn cell(id: &str, protected_backend: bool) -> Cell {
    let fd = declared(id, true).unwrap_or_else(|| panic!("unknown cell {id}"));
    if matches!(id, "nlkpp-dense-n400" | "bruss1d-n128-dense") {
        assert!(!protected_backend, "the dense cells have one back end");
        let mode =
            AssemblyStageClient::protected_dense_mode(&fd.problem, fd.span.0, &fd.y0).unwrap();
        return Cell {
            route: ColumnRoute::Dense,
            backend: LinearBackend::DenseLu(mode),
            backend_name: match mode {
                DenseLuMode::InPlace => "dense_lu(in-place)".into(),
                DenseLuMode::Faer => "dense_lu(faer)".into(),
            },
            band: None,
            fd,
        };
    }
    let (lower, upper) = fd.band.expect("a banded cell");
    let pattern = SparsityPattern::banded(fd.n, lower, upper).unwrap();
    let (backend, backend_name) = if protected_backend {
        (
            LinearBackend::BandedProtected { lower, upper },
            "banded_protected",
        )
    } else {
        (LinearBackend::BandedTight { lower, upper }, "banded_tight")
    };
    Cell {
        route: ColumnRoute::Coloured(pattern),
        backend,
        backend_name: backend_name.into(),
        band: Some((lower, upper)),
        fd,
    }
}

/// [`fingerprint`] of every output time and state of a run.
fn output_fingerprint(result: &Rodas5pFastResult) -> u64 {
    let observed = &result.observed;
    let times = fingerprint(FINGERPRINT_SEED, &observed.t);
    observed
        .y
        .iter()
        .fold(times, |hash, state| fingerprint(hash, state))
}

/// What a run of the driver did, for the client and for the protected driver.
fn run_record(result: &Rodas5pFastResult, partial: &Option<DriverPartial>) -> serde_json::Value {
    let (t_end, _) = final_state(result, partial);
    let counters = result.observed.counters;
    serde_json::json!({
        "success": result.observed.success,
        "stop_reason": partial.as_ref().map(|partial| format!("{:?}", partial.reason)),
        "attempts": result.attempts,
        "accepted_steps": result.accepted_steps,
        "rejected_steps": result.rejected_steps,
        "jacobian_reuses": result.jacobian_reuses,
        "lu": format!("{:?}", result.lu),
        "driver": result.driver,
        "t_end": t_end,
        "t_end_bits": hex(t_end.to_bits()),
        "final_fingerprint": hex(final_fingerprint(result, partial)),
        "output_fingerprint": hex(output_fingerprint(result)),
        "outputs": result.observed.t.len(),
        "counters": serde_json::to_value(counters).unwrap(),
    })
}

fn protected_record(cell: &Cell, run: &ProtectedRun) -> serde_json::Value {
    let mut record = run_record(&run.result, &run.partial);
    record["cell"] = serde_json::json!(cell.fd.id);
    record["n"] = serde_json::json!(cell.fd.n);
    record["backend"] = serde_json::json!(cell.backend_name);
    record["arm"] = serde_json::json!("prot");
    record["route"] = serde_json::json!(if cell.band.is_some() {
        "protected banded driver, reference finite differences"
    } else {
        "protected dense driver, reference finite differences"
    });
    record["builds"] = serde_json::json!(run.trace.builds);
    record["state_fingerprint"] = serde_json::json!(hex(run.trace.state_fingerprint));
    record["jacobian_fingerprint"] = serde_json::json!(hex(run.trace.jacobian_fingerprint));
    record
}

/// One run of the client driver with a client of the default options.
fn client_record(cell: &Cell, arm: &str, config: &RunConfig) -> serde_json::Value {
    let fd = &cell.fd;
    let execution = Execution::parse(arm);
    let options = default_options(&cell.route, execution, true);
    let check = options.first_use_check;
    let mut record = serde_json::json!({
        "cell": fd.id,
        "n": fd.n,
        "backend": cell.backend_name,
        "arm": arm,
        "route": if matches!(cell.route, ColumnRoute::Dense) { "dense" } else { "coloured" },
        "first_use_check": format!("{check:?}"),
    });
    fd.calls.store(0, std::sync::atomic::Ordering::Relaxed);
    let client = match AssemblyStageClient::new(fd.n, cell.backend, options) {
        Ok(client) => client,
        Err(error) => {
            record["outcome"] = serde_json::json!("refused");
            record["typed_error"] = serde_json::json!(format!("{error:?}"));
            return record;
        }
    };
    let handle = client.handle();
    record["colours"] = serde_json::json!(client.colours());
    record["rhs_per_build"] = serde_json::json!(client.rhs_per_build());
    record["workers"] = serde_json::json!(client.workers());
    let boxed: Box<dyn StageLinearSystem> = Box::new(client);
    let adaptive = config.adaptive(fd.span);
    let atol = vec![config.atol; fd.n];
    let output = OutputSchedule::new(vec![fd.span.0, fd.span.1]).unwrap();
    let outcome = integrate_rodas5p_fast_client_observed_with_atol(
        &fd.problem,
        boxed,
        fd.span,
        &fd.y0,
        &adaptive,
        &output,
        &atol,
    );
    let stats = handle.stats();
    match outcome {
        Ok((result, partial, _client)) => {
            let run = run_record(&result, &partial);
            for (key, value) in run.as_object().unwrap() {
                record[key] = value.clone();
            }
            record["outcome"] = serde_json::json!("ran");
        }
        Err(error) => {
            record["outcome"] = serde_json::json!("error");
            record["error"] = serde_json::json!(error.to_string());
            record["typed_error"] =
                serde_json::json!(handle.last_error().map(|error| format!("{error:?}")));
        }
    }
    record["stats"] = stats_record(&stats);
    record["rhs_calls_counted_by_the_callback"] =
        serde_json::json!(fd.calls.load(std::sync::atomic::Ordering::Relaxed));
    record
}

/// Every arm of a cell with the default options, and the protected driver.
fn identity(id: &str) {
    let config = RunConfig::declared();
    for protected_backend in [false, true] {
        if protected_backend && id != "nlkpp-band-n4096-r24" {
            continue;
        }
        let cell = cell(id, protected_backend);
        let check_colourings = match &cell.route {
            ColumnRoute::Coloured(pattern) => colourings_record(pattern),
            ColumnRoute::Dense => serde_json::Value::Null,
        };
        let defaults: Vec<serde_json::Value> = ["seq", "t1", "t2", "t4", "t6"]
            .iter()
            .map(|arm| {
                let options = default_options(&cell.route, Execution::parse(arm), true);
                serde_json::json!({"arm": arm, "first_use_check": format!("{:?}", options.first_use_check)})
            })
            .collect();
        println!(
            "V29_CELL_JSON {}",
            serde_json::json!({
                "cell": cell.fd.id,
                "n": cell.fd.n,
                "backend": cell.backend_name,
                "route": if matches!(cell.route, ColumnRoute::Dense) { "dense" } else { "coloured" },
                "declared_nnz": match &cell.route {
                    ColumnRoute::Coloured(pattern) => Some(pattern.nnz()),
                    ColumnRoute::Dense => None,
                },
                "client_colours": match &cell.route {
                    ColumnRoute::Coloured(pattern) => Some(colour_columns(pattern).1),
                    ColumnRoute::Dense => None,
                },
                "check_colourings": check_colourings,
                "default_checks": defaults,
                "max_attempts": config.max_attempts,
                "rtol": config.rtol,
                "atol": config.atol,
            })
        );
        cell.fd.calls.store(0, std::sync::atomic::Ordering::Relaxed);
        let protected = match cell.band {
            Some((lower, upper)) => run_protected_banded(&cell.fd, lower, upper, &config).unwrap(),
            None => run_protected_dense(&cell.fd, &config).unwrap(),
        };
        println!("V29_IDENTITY_JSON {}", protected_record(&cell, &protected));
        for arm in ["seq", "t1", "t2", "t4", "t6"] {
            println!("V29_IDENTITY_JSON {}", client_record(&cell, arm, &config));
        }
    }
}

// ---------------------------------------------------------------------------
// The corpus of node V26, transcribed from
// `stage_clients_assembly_check_corpus.rs` (`increment` and `Case` to `group`).
// ---------------------------------------------------------------------------

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
/// interleaved).
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

/// The true pattern of [`lower_coupled_chain`].
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
/// column or colour (the record of node V26).
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
    record["assembly_rhs_calls"] = serde_json::json!(stats.assembly_rhs_calls);
    record["verification_rhs_calls"] = serde_json::json!(stats.verification_rhs_calls);
    record["failed_build_rhs_calls"] = serde_json::json!(stats.failed_build_rhs_calls);
    record["verifications"] = serde_json::json!(stats.verifications);
    record["verified_columns"] = serde_json::json!(stats.verified_columns);
    record
}

/// The first linearization of a fresh client with the default options.
fn first_use(fd: &FdProblem, case: &Case, options: AssemblyOptions) -> serde_json::Value {
    let mut client = match AssemblyStageClient::new(fd.n, case.backend, options) {
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
/// client with the default options.
fn verify(
    fd: &FdProblem,
    case: &Case,
    options: AssemblyOptions,
    check: CheckColumns,
) -> serde_json::Value {
    let mut client = match AssemblyStageClient::new(fd.n, case.backend, options) {
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

/// The columns a check evaluated alone at `y0`, besides the groups of the
/// build.
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

fn multi_colouring(check: CheckColumns) -> bool {
    matches!(
        check,
        CheckColumns::MultiColouring | CheckColumns::MultiColouringPooled
    )
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

/// The rounding evidence of a miss of `check` on `case` (node V26): for every
/// omitted entry `(i, k)`, the bits of row `i` with and without the shift of
/// `k` at every state the check evaluated with `k` shifted.
fn miss_evidence(
    fd: &FdProblem,
    case: &Case,
    check: CheckColumns,
    missing: &[(usize, usize)],
) -> serde_json::Value {
    let ColumnRoute::Coloured(declared) = &case.route else {
        return serde_json::json!({"note": "dense route: no omitted entries"});
    };
    let (colour, _) = colour_columns(declared);
    let singles = single_columns(check, fd.n);
    let plan = multi_colouring(check).then(|| CheckColourings::of(declared).unwrap());
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
        "check": format!("{check:?}"),
        "omitted_entries": missing.len(),
        "evaluated_entries": entries.len(),
        "complete": entries.len() == missing.len(),
        "rounding_limited": all_limited && entries.len() == missing.len(),
        "entries": entries,
    })
}

fn run_group(name: &str, executions: &[Execution]) {
    let start = std::time::Instant::now();
    let (problems, cases) = group(name);
    let mut passes = std::collections::BTreeMap::<String, usize>::new();
    for case in &cases {
        let fd = &problems[case.problem];
        let wrong = case
            .omitted
            .as_ref()
            .is_some_and(|missing| !missing.is_empty());
        let mut decisions = serde_json::Map::new();
        let mut evidence = serde_json::Map::new();
        let mut with_multi = false;
        for &execution in executions {
            let options = default_options(&case.route, execution, false);
            let check = options.first_use_check;
            with_multi |= multi_colouring(check);
            let first = first_use(fd, case, options.clone());
            let checked = verify(fd, case, options, check);
            for (path, record) in [("first_use", &first), ("verify", &checked)] {
                if record["outcome"] == "pass" {
                    *passes
                        .entry(format!("{}:{path}", execution.name()))
                        .or_default() += 1;
                }
            }
            let sampled = matches!(check, CheckColumns::Sample(_) | CheckColumns::None);
            if checked["outcome"] == "pass" && wrong && !sampled {
                let missing = case.omitted.as_ref().unwrap();
                evidence.insert(execution.name(), miss_evidence(fd, case, check, missing));
            }
            decisions.insert(
                execution.name(),
                serde_json::json!({
                    "first_use_check": format!("{check:?}"),
                    "first_use": first,
                    "verify": checked,
                }),
            );
        }
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
            "V29_CASE_JSON {}",
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
                "executions": decisions,
                "check_colourings": check_colourings,
                "miss_evidence": evidence,
            })
        );
    }
    println!(
        "V29_GROUP_JSON {}",
        serde_json::json!({
            "group": name,
            "cases": cases.len(),
            "executions": executions.iter().map(|execution| execution.name()).collect::<Vec<_>>(),
            "passes": passes,
            "wall_s_diagnostic": start.elapsed().as_secs_f64(),
        })
    );
}

#[test]
#[ignore = "recording program of loop node V29; run it alone with V29_MODE set"]
fn v29_record() {
    let mode = std::env::var("V29_MODE").unwrap_or_default();
    let cell = std::env::var("V29_CELL").unwrap_or_default();
    let groups = std::env::var("V29_GROUP").unwrap_or_default();
    let executions: Vec<Execution> = std::env::var("V29_EXECUTIONS")
        .unwrap_or_else(|_| "seq,t6".into())
        .split(',')
        .filter(|text| !text.is_empty())
        .map(Execution::parse)
        .collect();
    println!(
        "V29_META_JSON {}",
        serde_json::json!({
            "mode": mode,
            "cell": cell,
            "groups": groups,
            "executions": executions.iter().map(|execution| execution.name()).collect::<Vec<_>>(),
            "default_dense": format!("{:?}", AssemblyOptions::dense().first_use_check),
            "default_coloured": format!(
                "{:?}",
                AssemblyOptions::coloured(SparsityPattern::banded(4, 1, 1).unwrap()).first_use_check
            ),
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
        "identity" => identity(&cell),
        "corpus" => {
            for name in groups.split(',').filter(|name| !name.is_empty()) {
                run_group(name, &executions);
            }
        }
        _ => println!(
            "V29_USAGE set V29_MODE (identity with V29_CELL, corpus with V29_GROUP and \
             V29_EXECUTIONS); see the file header"
        ),
    }
}
