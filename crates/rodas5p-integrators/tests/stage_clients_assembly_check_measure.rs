//! Cost program of loop node V26 (the full pattern check of the assembly
//! client, owner decision OD-L1-16): the four timed cells of node V08 with
//! every check arm. One ignored test driven by environment variables; every
//! result is one JSON object per line behind a tag. The protocol is frozen in
//! `research/production_loop_20261116/assembly_check/PREDECLARED.json`
//! (`cost_cells`), the logs are read by `parse_results.py` of that directory.
//!
//! ```text
//! V26_MODE=counts  V26_CELL=<cell> [V26_ARMS=<arm,..>]       counts and bit identity
//! V26_MODE=perf    V26_CELL=<cell> V26_ARM=<arm> V26_ACTION=<problem|setup|verify|solve>
//!                                                            one action, for perf stat -e instructions:u
//! V26_MODE=timing  V26_CELL=<cell> V26_WHAT=<verify|solve> V26_ARMS=<arm,..> [V26_ROUNDS=<w>+<t>]
//!                                                            interleaved rounds, 2 warm-up + 7 timed
//! V26_MODE=batched V26_ACTION=<identity|setup|single|batched|timing>
//!                                                            FA_B on nlkpp-dense-n400 (exploratory)
//! cargo test --release -p rodas5p-integrators --test stage_clients_assembly_check_measure -- --ignored --nocapture
//! ```
//!
//! Cells: `nlkpp-dense-n400` (dense route, the dense LU the protected driver
//! picks), `nlkpp-band-n4096-r24`, `bruss1d-n131072` and
//! `bruss1d-n128-coloured` (coloured route, the full band, back end
//! `banded_tight`), as timed in node V08. Arms are `<check>@<execution>`:
//! checks `None`, `S8` (`Sample(8)`), `FA` (`All`), `FA_P` (`AllPooled`),
//! `MC` (`MultiColouring`), `MC_P` (`MultiColouringPooled`) and `MCe` (the
//! multi-colouring check requested before every linearization of a solve,
//! exploratory); executions `seq` and `t<P>`.
//!
//! The first linearization of an arm is measured as the build and the check
//! at `(t0, y0)` on a fresh client ([`AssemblyStageClient::verify`]; the
//! copy into the back end is not included). The whole capped solve is
//! measured as in node V08: construction of the client and the driver call,
//! 48 attempts. The check of a solve is requested through the report handle
//! for the first linearization, so it applies on every route (the default
//! client makes no first-use check on the sequential dense route).

use std::hint::black_box;
use std::time::Instant;

use rodas5p_core::{CoreResult, WorkCounters};
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyOptions, AssemblyReportHandle, AssemblyStageClient, CheckColumns, ColumnRoute,
    FINGERPRINT_SEED, LinearBackend, RhsPurity, SparsityPattern, fingerprint,
};
use rodas5p_integrators::stage_clients::{PhaseTimesHandle, TimedStageClient};
use rodas5p_integrators::{
    OdeProblem, OutputSchedule, StageLinearSystem, integrate_rodas5p_fast_client_observed_with_atol,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{FdProblem, RunConfig, declared, final_fingerprint, final_state};

const RHS_BATCHES: usize = 7;
const RHS_BATCH_MIN_NS: u64 = 5_000_000;
const RHS_BATCH_MIN_CALLS: usize = 16;
/// Repetitions of the exploratory batched right-hand side.
const BATCHED_REPETITIONS: usize = 5;

fn hex(value: u64) -> String {
    format!("{value:016x}")
}

fn nanos(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn loadavg() -> serde_json::Value {
    let text = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let fields: Vec<f64> = text
        .split_whitespace()
        .take(3)
        .filter_map(|field| field.parse().ok())
        .collect();
    serde_json::json!({
        "load_1m": fields.first(),
        "load_5m": fields.get(1),
        "load_15m": fields.get(2),
        "raw": text.trim(),
    })
}

/// A cost cell of node V08.
struct Cell {
    fd: FdProblem,
    route: ColumnRoute,
    backend: LinearBackend,
    backend_name: String,
}

fn cell(id: &str) -> Cell {
    let fd = declared(id, false).unwrap_or_else(|| panic!("unknown cell {id}"));
    if id == "nlkpp-dense-n400" {
        let mode =
            AssemblyStageClient::protected_dense_mode(&fd.problem, fd.span.0, &fd.y0).unwrap();
        return Cell {
            route: ColumnRoute::Dense,
            backend: LinearBackend::DenseLu(mode),
            backend_name: format!("dense_lu({mode:?})"),
            fd,
        };
    }
    assert_ne!(id, "bruss1d-n128-dense", "not a cost cell of node V26");
    let (lower, upper) = fd.band.expect("a banded cell");
    Cell {
        route: ColumnRoute::Coloured(SparsityPattern::banded(fd.n, lower, upper).unwrap()),
        backend: LinearBackend::BandedTight { lower, upper },
        backend_name: "banded_tight".into(),
        fd,
    }
}

/// The check of an arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Check {
    None,
    S8,
    Full,
    FullPooled,
    Multi,
    MultiPooled,
    MultiEveryBuild,
}

impl Check {
    fn columns(self) -> CheckColumns {
        match self {
            Self::None => CheckColumns::None,
            Self::S8 => CheckColumns::Sample(AssemblyOptions::DEFAULT_CHECK_COLUMNS),
            Self::Full => CheckColumns::All,
            Self::FullPooled => CheckColumns::AllPooled,
            Self::Multi | Self::MultiEveryBuild => CheckColumns::MultiColouring,
            Self::MultiPooled => CheckColumns::MultiColouringPooled,
        }
    }

    fn multi(self) -> bool {
        matches!(
            self,
            Self::Multi | Self::MultiPooled | Self::MultiEveryBuild
        )
    }
}

#[derive(Clone, Debug)]
struct ArmSpec {
    label: String,
    check: Check,
    workers: Option<usize>,
}

fn parse_arm(text: &str) -> ArmSpec {
    let (name, execution) = text
        .split_once('@')
        .unwrap_or_else(|| panic!("an arm is <check>@<execution>, got {text}"));
    let check = match name {
        "None" => Check::None,
        "S8" => Check::S8,
        "FA" => Check::Full,
        "FA_P" => Check::FullPooled,
        "MC" => Check::Multi,
        "MC_P" => Check::MultiPooled,
        "MCe" => Check::MultiEveryBuild,
        _ => panic!("unknown check {name}"),
    };
    let workers = match execution {
        "seq" => None,
        _ => Some(
            execution
                .strip_prefix('t')
                .and_then(|workers| workers.parse().ok())
                .unwrap_or_else(|| panic!("unknown execution {execution}")),
        ),
    };
    ArmSpec {
        label: text.to_string(),
        check,
        workers,
    }
}

fn parse_arms(text: &str) -> Vec<ArmSpec> {
    text.split(',')
        .filter(|arm| !arm.is_empty())
        .map(parse_arm)
        .collect()
}

/// Every arm of a cell (the multi-colouring arms on the coloured route only).
fn default_arms(cell: &Cell) -> Vec<ArmSpec> {
    let coloured = matches!(cell.route, ColumnRoute::Coloured(_));
    let mut arms: Vec<String> = vec!["None@seq".into(), "S8@seq".into(), "FA@seq".into()];
    if coloured {
        arms.extend(["MC@seq".into(), "MCe@seq".into()]);
    }
    for workers in ["t2", "t4", "t6"] {
        for check in ["None", "S8", "FA_P"] {
            arms.push(format!("{check}@{workers}"));
        }
    }
    if coloured {
        arms.push("MC_P@t6".into());
    }
    arms.iter().map(|arm| parse_arm(arm)).collect()
}

fn options(cell: &Cell, arm: &ArmSpec, fingerprints: bool) -> AssemblyOptions {
    let options = match &cell.route {
        ColumnRoute::Dense => AssemblyOptions::dense(),
        ColumnRoute::Coloured(pattern) => AssemblyOptions::coloured(pattern.clone()),
    };
    let options = match arm.workers {
        None => options,
        Some(workers) => options.threaded(workers, RhsPurity::pure_and_reentrant()),
    };
    let options = options.with_first_use_check(CheckColumns::None);
    if fingerprints {
        options.with_fingerprints()
    } else {
        options
    }
}

fn client(cell: &Cell, arm: &ArmSpec, fingerprints: bool) -> AssemblyStageClient {
    AssemblyStageClient::new(cell.fd.n, cell.backend, options(cell, arm, fingerprints)).unwrap()
}

/// The first linearization of an arm: one build and the arm's check at
/// `(t0, y0)` on a fresh client (constructed outside the timed region).
fn first_linearization(cell: &Cell, arm: &ArmSpec) -> serde_json::Value {
    let fd = &cell.fd;
    let mut client = client(cell, arm, false);
    let mut counters = WorkCounters::default();
    let start = Instant::now();
    let outcome = client.verify(
        &fd.problem,
        fd.span.0,
        &fd.y0,
        arm.check.columns(),
        &mut counters,
    );
    let wall_ns = nanos(start);
    let stats = client.handle().stats();
    serde_json::json!({
        "cell": fd.id,
        "arm": arm.label,
        "what": "first_linearization",
        "wall_ns": wall_ns,
        "outcome": match &outcome {
            Ok(columns) => format!("Ok({columns})"),
            Err(error) => format!("{error:?}"),
        },
        "rhs_calls": counters.rhs_calls,
        "assembly_rhs_calls": stats.assembly_rhs_calls,
        "verification_rhs_calls": stats.verification_rhs_calls,
        "verified_columns": stats.verified_columns,
        "jacobian_fingerprint": client
            .jacobian_values()
            .map(|values| hex(fingerprint(FINGERPRINT_SEED, &values))),
    })
}

/// A client that requests a check before every linearization (the
/// exploratory arm `MCe`).
struct EveryBuild {
    inner: AssemblyStageClient,
    handle: AssemblyReportHandle,
    check: CheckColumns,
}

impl StageLinearSystem for EveryBuild {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.handle.request_check(self.check);
        self.inner.linearize(problem, t, y, counters)
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        self.inner.factor(inv_h_gamma, counters)
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        self.inner.solve_in_place(b, counters)
    }
}

/// The whole capped solve of an arm (V08 protocol): construction of the
/// client and the driver call, timed together.
fn solve(cell: &Cell, arm: &ArmSpec, fingerprints: bool) -> serde_json::Value {
    let fd = &cell.fd;
    let config = RunConfig::declared();
    let adaptive = config.adaptive(fd.span);
    let atol = vec![config.atol; fd.n];
    let output = OutputSchedule::new(vec![fd.span.0, fd.span.1]).unwrap();
    let start = Instant::now();
    let client = client(cell, arm, fingerprints);
    let handle = client.handle();
    let check = arm.check.columns();
    let phases: PhaseTimesHandle;
    let boxed: Box<dyn StageLinearSystem> = if arm.check == Check::MultiEveryBuild {
        let timed = TimedStageClient::new(EveryBuild {
            inner: client,
            handle: handle.clone(),
            check,
        });
        phases = timed.handle();
        Box::new(timed)
    } else {
        if check != CheckColumns::None {
            handle.request_check(check);
        }
        let timed = TimedStageClient::new(client);
        phases = timed.handle();
        Box::new(timed)
    };
    let construct_ns = nanos(start);
    let outcome = integrate_rodas5p_fast_client_observed_with_atol(
        &fd.problem,
        boxed,
        fd.span,
        &fd.y0,
        &adaptive,
        &output,
        &atol,
    );
    let wall_ns = nanos(start);
    let stats = handle.stats();
    let times = phases.snapshot();
    match outcome {
        Ok((result, partial, _client)) => {
            let (t_end, _) = final_state(&result, &partial);
            let counters = &result.observed.counters;
            serde_json::json!({
                "cell": fd.id,
                "arm": arm.label,
                "what": "solve",
                "wall_ns": wall_ns,
                "construct_ns": construct_ns,
                "linearize_ns": times.linearize_ns,
                "factor_ns": times.factor_ns,
                "solve_ns": times.solve_ns,
                "success": result.observed.success,
                "stop_reason": partial.as_ref().map(|partial| format!("{:?}", partial.reason)),
                "attempts": result.attempts,
                "accepted_steps": result.accepted_steps,
                "rejected_steps": result.rejected_steps,
                "t_end": t_end,
                "final_fingerprint": hex(final_fingerprint(&result, &partial)),
                "rhs_calls": counters.rhs_calls,
                "jacobian_builds": counters.jacobian_builds,
                "builds": stats.builds,
                "assembly_rhs_calls": stats.assembly_rhs_calls,
                "verifications": stats.verifications,
                "verified_columns": stats.verified_columns,
                "verification_rhs_calls": stats.verification_rhs_calls,
                "state_fingerprint": hex(stats.state_fingerprint),
                "jacobian_fingerprint": hex(stats.jacobian_fingerprint),
            })
        }
        Err(error) => serde_json::json!({
            "cell": fd.id,
            "arm": arm.label,
            "what": "solve",
            "wall_ns": wall_ns,
            "error": error.to_string(),
            "typed_error": handle.last_error().map(|error| format!("{error:?}")),
        }),
    }
}

/// Deterministic counts and bit identity of every arm: the first
/// linearization and the whole capped solve with fingerprints.
fn counts(cell: &Cell, arms: &[ArmSpec]) {
    for arm in arms {
        if arm.check.multi() && matches!(cell.route, ColumnRoute::Dense) {
            continue;
        }
        if arm.check != Check::MultiEveryBuild {
            let mut record = first_linearization(cell, arm);
            record["backend"] = serde_json::json!(cell.backend_name);
            println!("V26_COUNT_JSON {record}");
        }
        let mut record = solve(cell, arm, true);
        record["backend"] = serde_json::json!(cell.backend_name);
        println!("V26_COUNT_JSON {record}");
    }
}

/// Wall time per right-hand side at `y0` (node V08): one warm-up batch, then
/// [`RHS_BATCHES`] batches of at least [`RHS_BATCH_MIN_NS`].
fn rhs_cost(fd: &FdProblem) -> serde_json::Value {
    let mut out = vec![0.0; fd.n];
    let mut counters = WorkCounters::default();
    let mut batch = |calls: usize| -> u64 {
        let start = Instant::now();
        for _ in 0..calls {
            fd.problem
                .eval_rhs_into(fd.span.0, black_box(&fd.y0), &mut out, &mut counters)
                .unwrap();
            black_box(&out);
        }
        nanos(start)
    };
    let probe = batch(RHS_BATCH_MIN_CALLS).max(1);
    let per_call = probe as f64 / RHS_BATCH_MIN_CALLS as f64;
    let calls = ((RHS_BATCH_MIN_NS as f64 / per_call).ceil() as usize).max(RHS_BATCH_MIN_CALLS);
    batch(calls);
    let per_call_ns: Vec<f64> = (0..RHS_BATCHES)
        .map(|_| batch(calls) as f64 / calls as f64)
        .collect();
    serde_json::json!({
        "cell": fd.id,
        "n": fd.n,
        "calls_per_batch": calls,
        "batches": RHS_BATCHES,
        "per_call_ns": per_call_ns,
    })
}

fn rounds() -> (usize, usize) {
    let text = std::env::var("V26_ROUNDS").unwrap_or_else(|_| "2+7".into());
    let (warm, timed) = text.split_once('+').expect("V26_ROUNDS is <warm>+<timed>");
    (warm.parse().unwrap(), timed.parse().unwrap())
}

/// Interleaved rounds (round `r` starts with arm `r mod arms`).
fn timing(cell: &Cell, what: &str, arms: &[ArmSpec]) {
    let (warm, timed) = rounds();
    println!(
        "V26_LOAD_JSON {}",
        serde_json::json!({"cell": cell.fd.id, "what": what, "when": "before", "loadavg": loadavg()})
    );
    if what == "verify" {
        println!("V26_RHS_COST_JSON {}", rhs_cost(&cell.fd));
    }
    for round in 0..warm + timed {
        for position in 0..arms.len() {
            let arm = &arms[(round + position) % arms.len()];
            let mut record = match what {
                "verify" => first_linearization(cell, arm),
                "solve" => solve(cell, arm, false),
                _ => panic!("V26_WHAT is verify or solve"),
            };
            record["round"] = serde_json::json!(round);
            record["position_in_round"] = serde_json::json!(position);
            record["warm_up"] = serde_json::json!(round < warm);
            record["backend"] = serde_json::json!(cell.backend_name);
            println!("V26_TIMING_JSON {record}");
        }
    }
    println!(
        "V26_LOAD_JSON {}",
        serde_json::json!({"cell": cell.fd.id, "what": what, "when": "after", "loadavg": loadavg()})
    );
}

/// One action of one arm, for `perf stat -e instructions:u` (the
/// instructions of an action are differences between actions: setup minus
/// problem is the construction of the client, verify minus setup the first
/// linearization, solve minus problem the construction and the capped
/// solve).
fn perf(id: &str, arm: &ArmSpec, action: &str) {
    let cell = cell(id);
    let record = match action {
        "problem" => serde_json::json!({"cell": id, "arm": arm.label, "action": action}),
        "setup" => {
            let client = black_box(client(&cell, arm, false));
            serde_json::json!({"cell": id, "arm": arm.label, "action": action, "rhs_per_build": client.rhs_per_build()})
        }
        "verify" => {
            let mut record = first_linearization(&cell, arm);
            record["action"] = serde_json::json!(action);
            record
        }
        "solve" => {
            let mut record = solve(&cell, arm, false);
            record["action"] = serde_json::json!(action);
            record
        }
        _ => panic!("V26_ACTION is problem, setup, verify or solve"),
    };
    println!("V26_PERF_JSON {record}");
}

/// The nonlocal Fisher-KPP right-hand side of `nlkpp-dense-n400`
/// (`problems::nlkpp_dense`), transcribed: `h = 0.1`, Laplace kernel.
struct Nlkpp {
    n: usize,
    h: f64,
    phi: Vec<f64>,
}

impl Nlkpp {
    fn new(n: usize) -> Self {
        let h = 0.1;
        let sigma = 5.0;
        let phi = (0..n)
            .map(|d| (-(d as f64 * h) / sigma).exp() / (2.0 * sigma))
            .collect();
        Self { n, h, phi }
    }

    /// One state, the statement order of `problems::nlkpp_dense`.
    fn single(&self, u: &[f64], out: &mut [f64]) {
        let (n, h, phi) = (self.n, self.h, &self.phi);
        for i in 0..n {
            let mut conv = 0.0;
            for (j, uj) in u[..i].iter().enumerate() {
                conv += phi[i - j] * uj;
            }
            for (d, uj) in u[i..].iter().enumerate() {
                conv += phi[d] * uj;
            }
            let left = if i == 0 { u[0] } else { u[i - 1] };
            let right = if i + 1 == n { u[n - 1] } else { u[i + 1] };
            out[i] = (left - 2.0 * u[i] + right) / (h * h) + u[i] * (1.0 - h * conv);
        }
    }

    /// `k` states at once: `states[j k + s]` is component `j` of state `s`,
    /// `out[i k + s]` component `i` of its right-hand side. Every state is
    /// summed in the order of [`Nlkpp::single`]; the loop over the states
    /// is the inner one (vectorised across states).
    fn batched(&self, k: usize, states: &[f64], out: &mut [f64], conv: &mut [f64]) {
        let (n, h, phi) = (self.n, self.h, &self.phi);
        for i in 0..n {
            conv.fill(0.0);
            for j in 0..i {
                let p = phi[i - j];
                for (acc, uj) in conv.iter_mut().zip(&states[j * k..(j + 1) * k]) {
                    *acc += p * uj;
                }
            }
            for (d, &p) in phi[..n - i].iter().enumerate() {
                let j = i + d;
                for (acc, uj) in conv.iter_mut().zip(&states[j * k..(j + 1) * k]) {
                    *acc += p * uj;
                }
            }
            let left_row = if i == 0 { 0 } else { i - 1 };
            let right_row = if i + 1 == n { n - 1 } else { i + 1 };
            for s in 0..k {
                let ui = states[i * k + s];
                let left = states[left_row * k + s];
                let right = states[right_row * k + s];
                out[i * k + s] = (left - 2.0 * ui + right) / (h * h) + ui * (1.0 - h * conv[s]);
            }
        }
    }
}

/// The states of the full check of `y`: state `s` is `y` with column `s`
/// shifted by the increment of `crates/vigilode/src/fd.rs:443-455`.
fn full_check_states(y: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = y.len();
    let mut states = vec![0.0; n * n];
    let mut deltas = vec![0.0; n];
    for j in 0..n {
        for s in 0..n {
            states[j * n + s] = y[j];
        }
    }
    for s in 0..n {
        let raw = f64::EPSILON.sqrt() * y[s].abs().max(1.0);
        let forward = y[s] + raw;
        let shifted = if forward.is_finite() {
            forward
        } else {
            y[s] - raw
        };
        states[s * n + s] = shifted;
        deltas[s] = shifted - y[s];
    }
    (states, deltas)
}

/// FA_B (exploratory): the full check of `nlkpp-dense-n400` with its `n`
/// shifted states evaluated by one batched right-hand side.
fn batched(action: &str) {
    let fd = declared("nlkpp-dense-n400", false).unwrap();
    let n = fd.n;
    let rhs = Nlkpp::new(n);
    let (states, deltas) = full_check_states(&fd.y0);
    let mut out = vec![0.0; n * n];
    let mut conv = vec![0.0; n];
    let mut single_out = vec![0.0; n];
    let record = match action {
        "identity" => {
            // The transcription against the problem's right-hand side, and
            // every batched state against the single evaluation.
            let mut problem_out = vec![0.0; n];
            let mut transcribed = vec![0.0; n];
            let mut counters = WorkCounters::default();
            let mut state = vec![0.0; n];
            let mut transcription_differences = 0;
            let mut batched_differences = 0;
            rhs.batched(n, &states, &mut out, &mut conv);
            for s in 0..n {
                for j in 0..n {
                    state[j] = states[j * n + s];
                }
                fd.problem
                    .eval_rhs_into(fd.span.0, &state, &mut problem_out, &mut counters)
                    .unwrap();
                rhs.single(&state, &mut transcribed);
                transcription_differences += problem_out
                    .iter()
                    .zip(&transcribed)
                    .filter(|(a, b)| a.to_bits() != b.to_bits())
                    .count();
                batched_differences += (0..n)
                    .filter(|&i| out[i * n + s].to_bits() != problem_out[i].to_bits())
                    .count();
            }
            // The full check with the batched states against the Jacobian
            // the client assembles (dense route, sequential).
            let mut client = AssemblyStageClient::new(
                n,
                LinearBackend::DenseLu(
                    AssemblyStageClient::protected_dense_mode(&fd.problem, fd.span.0, &fd.y0)
                        .unwrap(),
                ),
                AssemblyOptions::dense().with_first_use_check(CheckColumns::None),
            )
            .unwrap();
            client
                .verify(
                    &fd.problem,
                    fd.span.0,
                    &fd.y0,
                    CheckColumns::None,
                    &mut WorkCounters::default(),
                )
                .unwrap();
            let jacobian = client.jacobian_values().unwrap();
            let mut base = vec![0.0; n];
            fd.problem
                .eval_rhs_into(fd.span.0, &fd.y0, &mut base, &mut counters)
                .unwrap();
            let mut first_failure = None;
            'columns: for j in 0..n {
                for i in 0..n {
                    let single = (out[i * n + j] - base[i]) / deltas[j];
                    if single.to_bits() != jacobian[i * n + j].to_bits() {
                        first_failure = Some([i, j]);
                        break 'columns;
                    }
                }
            }
            serde_json::json!({
                "action": action,
                "states": n,
                "transcription_components_with_other_bits": transcription_differences,
                "batched_components_with_other_bits": batched_differences,
                "full_check_with_batched_states": match first_failure {
                    None => "pass".to_string(),
                    Some([i, j]) => format!("entry ({i}, {j}) differs"),
                },
            })
        }
        "setup" => serde_json::json!({"action": action, "states": n}),
        "single" => {
            let mut state = vec![0.0; n];
            for _ in 0..BATCHED_REPETITIONS {
                for s in 0..n {
                    for j in 0..n {
                        state[j] = states[j * n + s];
                    }
                    rhs.single(black_box(&state), &mut single_out);
                    black_box(&single_out);
                }
            }
            serde_json::json!({"action": action, "evaluations": BATCHED_REPETITIONS * n})
        }
        "batched" => {
            for _ in 0..BATCHED_REPETITIONS {
                rhs.batched(n, black_box(&states), &mut out, &mut conv);
                black_box(&out);
            }
            serde_json::json!({"action": action, "evaluations": BATCHED_REPETITIONS * n})
        }
        "timing" => {
            // Interleaved: 2 warm-up + 7 timed rounds of one batched call
            // (n states) and n single calls.
            let mut rows = Vec::new();
            let mut state = vec![0.0; n];
            for round in 0..9 {
                let start = Instant::now();
                rhs.batched(n, black_box(&states), &mut out, &mut conv);
                black_box(&out);
                let batched_ns = nanos(start);
                let start = Instant::now();
                for s in 0..n {
                    for j in 0..n {
                        state[j] = states[j * n + s];
                    }
                    rhs.single(black_box(&state), &mut single_out);
                    black_box(&single_out);
                }
                let single_ns = nanos(start);
                rows.push(serde_json::json!({
                    "round": round,
                    "warm_up": round < 2,
                    "batched_ns_for_n_states": batched_ns,
                    "single_ns_for_n_states": single_ns,
                }));
            }
            serde_json::json!({
                "action": action,
                "states": n,
                "loadavg_after": loadavg(),
                "rounds": rows,
            })
        }
        _ => panic!("V26_ACTION is identity, setup, single, batched or timing"),
    };
    println!("V26_BATCHED_JSON {record}");
}

#[test]
#[ignore = "cost program of loop node V26; run it alone with V26_MODE set"]
fn v26_measure() {
    let mode = std::env::var("V26_MODE").unwrap_or_default();
    let id = std::env::var("V26_CELL").unwrap_or_default();
    println!(
        "V26_META_JSON {}",
        serde_json::json!({
            "mode": mode,
            "cell": id,
            "arm_env": std::env::var("V26_ARM").ok(),
            "arms_env": std::env::var("V26_ARMS").ok(),
            "action_env": std::env::var("V26_ACTION").ok(),
            "what_env": std::env::var("V26_WHAT").ok(),
            "rounds_env": std::env::var("V26_ROUNDS").ok(),
            "max_attempts": RunConfig::declared().max_attempts,
            "rtol": RunConfig::declared().rtol,
            "atol": RunConfig::declared().atol,
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
        "counts" => {
            let cell = cell(&id);
            let arms = match std::env::var("V26_ARMS") {
                Ok(text) => parse_arms(&text),
                Err(_) => default_arms(&cell),
            };
            counts(&cell, &arms);
        }
        "perf" => {
            let arm = parse_arm(&std::env::var("V26_ARM").expect("V26_ARM"));
            perf(&id, &arm, &std::env::var("V26_ACTION").expect("V26_ACTION"));
        }
        "timing" => {
            let cell = cell(&id);
            let arms = parse_arms(&std::env::var("V26_ARMS").expect("V26_ARMS"));
            timing(&cell, &std::env::var("V26_WHAT").expect("V26_WHAT"), &arms);
        }
        "batched" => batched(&std::env::var("V26_ACTION").expect("V26_ACTION")),
        _ => println!("V26_USAGE set V26_MODE; see the file header"),
    }
}
