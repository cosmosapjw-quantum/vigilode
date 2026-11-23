//! Measurement program of loop L1 node V18_DENSE_LU_CLIENT.
//!
//! Every line of standard output is one JSON object with a `type`. The
//! protocol (cells, tolerances, arms, repetitions) is given on the command
//! line by `../run_measurements.py` from `../PREDECLARED.json`; this program
//! decides nothing about it. Numbers are produced here and nowhere else.
//!
//! Commands:
//!
//! * `pilot`: the protected driver alone (attempt counts for sizing);
//! * `identity`: every arm against the protected driver (end state,
//!   accepted trajectory, step counts, counters);
//! * `faults`: one arm, repeated, with the minor page faults of the
//!   process read at every accepted state;
//! * `measure`: interleaved wall times of several arms;
//! * `single`: one arm repeated without any reading, for `perf stat`;
//! * `reference-inputs`: what an independent evaluation of the exact
//!   solution of a skewnet cell needs.

#[path = "../../../../../crates/rodas5p-integrators/tests/stage_clients_dense_lu_reuse_problems.rs"]
mod problems;

use std::io::Write as _;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rodas5p_integrators::AdaptiveStepConfig;
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::dense_lu_reuse::{
    DENSE_LU_REUSE_CHUNK_FLOPS, DENSE_LU_REUSE_CHUNK_SWAPS, DENSE_LU_REUSE_SEQUENTIAL_PANEL_COLUMNS,
};
use serde_json::{Value, json};

use problems::{
    Arm, ArmRun, Cell, Fnv, SKEWNET_ALPHA_AMPLITUDE, SKEWNET_ALPHA_FREQUENCY, SKEWNET_ALPHA_PHASE,
    SKEWNET_BETA_AMPLITUDE, SKEWNET_BETA_FREQUENCY, SkewnetData, StepHook, adaptive, fingerprint,
    max_abs_difference, meanfield, run_arm, skewnet,
};

// ---------------------------------------------------------------------------
// Process readings.
// ---------------------------------------------------------------------------

#[repr(C)]
struct Timeval {
    seconds: i64,
    microseconds: i64,
}

/// `struct rusage` of Linux on x86-64.
#[repr(C)]
struct ResourceUsage {
    user_time: Timeval,
    system_time: Timeval,
    max_resident_kb: i64,
    shared_text: i64,
    unshared_data: i64,
    unshared_stack: i64,
    minor_faults: i64,
    major_faults: i64,
    swaps: i64,
    blocks_in: i64,
    blocks_out: i64,
    messages_sent: i64,
    messages_received: i64,
    signals: i64,
    voluntary_switches: i64,
    involuntary_switches: i64,
}

unsafe extern "C" {
    fn getrusage(who: i32, usage: *mut ResourceUsage) -> i32;
}

/// Minor and major page faults of the whole process (all threads) so far.
/// No allocation and no file access: safe to call between attempts.
fn page_faults() -> (u64, u64) {
    let mut usage = std::mem::MaybeUninit::<ResourceUsage>::zeroed();
    // SAFETY: `usage` is a writable `struct rusage`; RUSAGE_SELF is 0.
    let status = unsafe { getrusage(0, usage.as_mut_ptr()) };
    assert_eq!(status, 0, "getrusage failed");
    // SAFETY: the call succeeded and filled the structure.
    let usage = unsafe { usage.assume_init() };
    (usage.minor_faults as u64, usage.major_faults as u64)
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

fn unix_time() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn emit(value: Value) {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{value}").expect("stdout");
    out.flush().expect("stdout");
}

fn fail(message: &str) -> ! {
    eprintln!("v18-dense-lu-runner: {message}");
    std::process::exit(2);
}

fn hex(values: &[f64]) -> Vec<String> {
    values
        .iter()
        .map(|v| format!("{:016x}", v.to_bits()))
        .collect()
}

// ---------------------------------------------------------------------------
// The recorder behind the Jacobian hook.
// ---------------------------------------------------------------------------

/// Reserved readings per run; a run with more accepted states fails.
const READINGS: usize = 1 << 16;

/// What the hook has seen in the current run.
struct Recorder {
    /// Minor page faults of the process at every accepted state.
    minor_faults: Vec<u64>,
    /// The fingerprint of the accepted states.
    hash: Fnv,
}

impl Recorder {
    fn reset(&mut self) {
        self.minor_faults.clear();
        self.hash = Fnv::default();
    }
}

fn recorder() -> (StepHook, Arc<Mutex<Recorder>>) {
    let recorder = Arc::new(Mutex::new(Recorder {
        minor_faults: Vec::with_capacity(READINGS),
        hash: Fnv::default(),
    }));
    let sink = Arc::clone(&recorder);
    let hook: StepHook = Arc::new(move |t: f64, y: &[f64]| {
        let mut recorder = sink.lock().unwrap_or_else(|e| e.into_inner());
        if recorder.minor_faults.len() == READINGS {
            fail("more accepted states than reserved readings");
        }
        // Within the reserved capacity: no allocation.
        recorder.minor_faults.push(page_faults().0);
        recorder.hash.eat_point(t, y);
    });
    (hook, recorder)
}

// ---------------------------------------------------------------------------
// Cells.
// ---------------------------------------------------------------------------

struct Built {
    cell: Cell,
    recorder: Arc<Mutex<Recorder>>,
    skewnet: Option<Arc<SkewnetData>>,
}

/// The cell `family.n<order>` on `[0, t_end]` with the recording hook.
fn build(id: &str, t_end: f64) -> Built {
    let (family, order) = id
        .split_once(".n")
        .unwrap_or_else(|| fail("a cell is <family>.n<order>"));
    let n: usize = order
        .parse()
        .unwrap_or_else(|_| fail("the order of a cell is an integer"));
    let (hook, recorder) = recorder();
    match family {
        "meanfield" => Built {
            cell: meanfield(n, t_end, Some(hook)),
            recorder,
            skewnet: None,
        },
        "skewnet" => {
            let data = Arc::new(SkewnetData::new(n));
            Built {
                cell: skewnet(&data, t_end, Some(hook)),
                recorder,
                skewnet: Some(data),
            }
        }
        _ => fail("the families are meanfield and skewnet"),
    }
}

// ---------------------------------------------------------------------------
// One run.
// ---------------------------------------------------------------------------

/// Accepted states whose page faults are not counted (`faults` and
/// `measure`): the first steps run with the set-up of the driver and client.
struct Window {
    warm_up_steps: usize,
}

struct RunOut {
    run: ArmRun,
    wall_ns: u64,
    minor_faults: u64,
    major_faults: u64,
    /// Accepted steps after the warm-up, their minor faults, the largest
    /// count within one of them.
    steady_steps: usize,
    steady_minor_faults: u64,
    steady_minor_faults_max_step: u64,
    trace_states: usize,
    trace_hash: String,
}

/// One timed integration of `arm`. The page-fault counters are read outside
/// the timed interval; the hook reads them once per accepted state inside.
fn timed_run(built: &Built, arm: Arm, config: &AdaptiveStepConfig, window: &Window) -> RunOut {
    built
        .recorder
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .reset();
    let before = page_faults();
    let start = Instant::now();
    let run = run_arm(
        &built.cell,
        &built.cell.problem,
        arm,
        DenseLuMode::Faer,
        config,
    );
    let wall_ns = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let after = page_faults();
    let run = run.unwrap_or_else(|error| fail(&format!("arm {} failed: {error}", arm.name())));
    let recorder = built.recorder.lock().unwrap_or_else(|e| e.into_inner());
    let readings = &recorder.minor_faults;
    let counted = readings.get(window.warm_up_steps..).unwrap_or(&[]);
    let steps: Vec<u64> = counted.windows(2).map(|pair| pair[1] - pair[0]).collect();
    RunOut {
        run,
        wall_ns,
        minor_faults: after.0 - before.0,
        major_faults: after.1 - before.1,
        steady_steps: steps.len(),
        steady_minor_faults: steps.iter().sum(),
        steady_minor_faults_max_step: steps.iter().copied().max().unwrap_or(0),
        trace_states: readings.len(),
        trace_hash: recorder.hash.hex(),
    }
}

fn run_json(built: &Built, arm: Arm, out: &RunOut) -> Value {
    let result = &out.run.result;
    let (t_end, y_end) = out.run.end();
    let error = built
        .cell
        .exact_end
        .as_ref()
        .filter(|_| out.run.partial.is_none())
        .map(|exact| max_abs_difference(y_end, exact));
    json!({
        "cell": built.cell.id,
        "arm": arm.name(),
        "wall_ns": out.wall_ns,
        "attempts": result.attempts,
        "accepted_steps": result.accepted_steps,
        "rejected_steps": result.rejected_steps,
        "jacobian_reuses": result.jacobian_reuses,
        "success": result.observed.success,
        "complete": out.run.partial.is_none(),
        "lu": format!("{:?}", result.lu),
        "counters": result.observed.counters,
        "end_time": t_end,
        "end_state_fnv1a64": fingerprint(t_end, y_end),
        "accepted_states": out.trace_states,
        "accepted_trajectory_fnv1a64": out.trace_hash,
        "error_max_abs": error,
        "rows_moved_by_last_factorization": out.run.permuted_rows,
        "minor_faults": out.minor_faults,
        "major_faults": out.major_faults,
        "steady_steps": out.steady_steps,
        "steady_minor_faults": out.steady_minor_faults,
        "steady_minor_faults_max_step": out.steady_minor_faults_max_step,
    })
}

/// The first quantity in which `other` differs from `reference`, as text.
fn first_difference(reference: &RunOut, other: &RunOut) -> Option<String> {
    let (a, b) = (&reference.run.result, &other.run.result);
    let (end_a, end_b) = (reference.run.end(), other.run.end());
    let checks: [(&str, String, String); 9] = [
        ("attempts", a.attempts.to_string(), b.attempts.to_string()),
        (
            "accepted steps",
            a.accepted_steps.to_string(),
            b.accepted_steps.to_string(),
        ),
        (
            "rejected steps",
            a.rejected_steps.to_string(),
            b.rejected_steps.to_string(),
        ),
        (
            "Jacobian reuses",
            a.jacobian_reuses.to_string(),
            b.jacobian_reuses.to_string(),
        ),
        (
            "counters",
            format!("{:?}", a.observed.counters),
            format!("{:?}", b.observed.counters),
        ),
        (
            "success",
            a.observed.success.to_string(),
            b.observed.success.to_string(),
        ),
        (
            "accepted states",
            reference.trace_states.to_string(),
            other.trace_states.to_string(),
        ),
        (
            "accepted trajectory fingerprint",
            reference.trace_hash.clone(),
            other.trace_hash.clone(),
        ),
        (
            "end state fingerprint",
            fingerprint(end_a.0, end_a.1),
            fingerprint(end_b.0, end_b.1),
        ),
    ];
    checks
        .into_iter()
        .find(|(_, x, y)| x != y)
        .map(|(name, x, y)| format!("{name}: {x} vs {y}"))
}

fn identity_json(reference: &RunOut, other: &RunOut) -> Value {
    let (a, b) = (&reference.run.result, &other.run.result);
    let (end_a, end_b) = (reference.run.end(), other.run.end());
    let difference = first_difference(reference, other);
    json!({
        "bit_identical": difference.is_none(),
        "first_difference": difference,
        "attempts_equal": a.attempts == b.attempts,
        "accepted_equal": a.accepted_steps == b.accepted_steps,
        "rejected_equal": a.rejected_steps == b.rejected_steps,
        "counters_equal": a.observed.counters == b.observed.counters,
        "accepted_trajectory_equal": reference.trace_hash == other.trace_hash
            && reference.trace_states == other.trace_states,
        "end_state_equal": fingerprint(end_a.0, end_a.1) == fingerprint(end_b.0, end_b.1),
        "end_state_max_abs_difference": if end_a.1.len() == end_b.1.len() {
            Some(max_abs_difference(end_a.1, end_b.1))
        } else {
            None
        },
        "attempts_reference": a.attempts,
        "attempts_other": b.attempts,
    })
}

// ---------------------------------------------------------------------------
// Command line.
// ---------------------------------------------------------------------------

struct Args {
    cell: String,
    t_end: f64,
    rtol: f64,
    max_attempts: usize,
    arms: Vec<Arm>,
    warmups: usize,
    reps: usize,
    runs: usize,
    warm_up_steps: usize,
    emit_end_state: bool,
    none_arm: bool,
}

fn parse_args(rest: &[String]) -> Args {
    let mut args = Args {
        cell: String::new(),
        t_end: f64::NAN,
        rtol: f64::NAN,
        max_attempts: 1_000_000,
        arms: Vec::new(),
        warmups: 2,
        reps: 7,
        runs: 1,
        warm_up_steps: 2,
        emit_end_state: false,
        none_arm: false,
    };
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        let mut value = || {
            iter.next()
                .unwrap_or_else(|| fail(&format!("{flag} needs a value")))
                .as_str()
        };
        match flag.as_str() {
            "--cell" => args.cell = value().to_string(),
            "--t-end" => args.t_end = value().parse().unwrap_or_else(|_| fail("--t-end")),
            "--rtol" => args.rtol = value().parse().unwrap_or_else(|_| fail("--rtol")),
            "--max-attempts" => {
                args.max_attempts = value().parse().unwrap_or_else(|_| fail("--max-attempts"));
            }
            "--arms" => {
                for name in value().split(',') {
                    if name == "none" {
                        args.none_arm = true;
                    } else {
                        args.arms.push(
                            Arm::parse(name)
                                .unwrap_or_else(|| fail(&format!("unknown arm {name}"))),
                        );
                    }
                }
            }
            "--warmups" => args.warmups = value().parse().unwrap_or_else(|_| fail("--warmups")),
            "--reps" => args.reps = value().parse().unwrap_or_else(|_| fail("--reps")),
            "--runs" => args.runs = value().parse().unwrap_or_else(|_| fail("--runs")),
            "--warm-up-steps" => {
                args.warm_up_steps = value().parse().unwrap_or_else(|_| fail("--warm-up-steps"));
            }
            "--emit-end-state" => args.emit_end_state = true,
            other => fail(&format!("unknown flag {other}")),
        }
    }
    if args.cell.is_empty() || !args.t_end.is_finite() || !args.rtol.is_finite() {
        fail("--cell, --t-end and --rtol are required");
    }
    args
}

fn header(mode: &str, argv: &[String], args: &Args, built: &Built, config: &AdaptiveStepConfig) {
    let exe = std::fs::read_link("/proc/self/exe")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let cell = &built.cell;
    emit(json!({
        "type": "header",
        "mode": mode,
        "argv": argv,
        "argv0_length": argv[0].len(),
        "executable": exe,
        "pid": std::process::id(),
        "cpus_allowed_list": cpus_allowed(),
        "loadavg_before": loadavg(),
        "unix_time": unix_time(),
        "cell": {
            "id": cell.id,
            "family": cell.family,
            "n": cell.n,
            "t_span": [cell.span.0, cell.span.1],
            "rtol": config.rtol,
            "atol": config.atol,
            "initial_step": config.initial_step,
            "max_attempts": config.max_attempts,
            "has_exact_solution": cell.exact_end.is_some(),
            "protected_policy_mode": format!(
                "{:?}",
                DenseLuMode::protected_choice_at(&cell.problem, cell.span.0, &cell.y0)
                    .unwrap_or_else(|error| fail(&format!("mode choice failed: {error}")))
            ),
        },
        "protocol": {
            "arms": args.arms.iter().map(|arm| arm.name()).collect::<Vec<_>>(),
            "warmups": args.warmups,
            "repetitions": args.reps,
            "runs": args.runs,
            "warm_up_steps": args.warm_up_steps,
            "arm_order": "rotated by one position per repetition",
        },
        "client_constants": {
            "sequential_panel_columns": DENSE_LU_REUSE_SEQUENTIAL_PANEL_COLUMNS,
            "chunk_flops": DENSE_LU_REUSE_CHUNK_FLOPS,
            "chunk_swaps": DENSE_LU_REUSE_CHUNK_SWAPS,
        },
    }));
}

fn footer(started: Instant) {
    emit(json!({
        "type": "footer",
        "loadavg_after": loadavg(),
        "elapsed_s": started.elapsed().as_secs_f64(),
        "unix_time": unix_time(),
    }));
}

fn setup(mode: &str, argv: &[String]) -> (Args, Built, AdaptiveStepConfig, Instant) {
    let args = parse_args(&argv[2..]);
    let built = build(&args.cell, args.t_end);
    let config = adaptive(&built.cell, args.rtol, args.max_attempts);
    // The choice of the mode in the header evaluates the Jacobian once:
    // the recorder is reset at the start of every run.
    header(mode, argv, &args, &built, &config);
    (args, built, config, Instant::now())
}

// ---------------------------------------------------------------------------
// Commands.
// ---------------------------------------------------------------------------

fn command_pilot(argv: &[String]) {
    let (args, built, config, started) = setup("pilot", argv);
    let window = Window {
        warm_up_steps: args.warm_up_steps,
    };
    let out = timed_run(&built, Arm::Protected, &config, &window);
    let mut row = run_json(&built, Arm::Protected, &out);
    row["type"] = json!("pilot");
    emit(row);
    footer(started);
}

fn command_identity(argv: &[String]) {
    let (args, built, config, started) = setup("identity", argv);
    let window = Window {
        warm_up_steps: args.warm_up_steps,
    };
    let reference = timed_run(&built, Arm::Protected, &config, &window);
    let mut row = run_json(&built, Arm::Protected, &reference);
    row["type"] = json!("identity_reference");
    emit(row);
    for arm in &args.arms {
        let other = timed_run(&built, *arm, &config, &window);
        let mut row = run_json(&built, *arm, &other);
        row["type"] = json!("identity");
        row["against_protected"] = identity_json(&reference, &other);
        emit(row);
    }
    footer(started);
}

fn command_faults(argv: &[String]) {
    let (args, built, config, started) = setup("faults", argv);
    let window = Window {
        warm_up_steps: args.warm_up_steps,
    };
    let [arm] = args.arms[..] else {
        fail("faults takes exactly one arm");
    };
    for index in 0..args.runs {
        let out = timed_run(&built, arm, &config, &window);
        let mut row = run_json(&built, arm, &out);
        row["type"] = json!("faults");
        row["run_index"] = json!(index);
        row["warm_up_steps"] = json!(args.warm_up_steps);
        emit(row);
    }
    footer(started);
}

fn command_measure(argv: &[String]) {
    let (args, built, config, started) = setup("measure", argv);
    let window = Window {
        warm_up_steps: args.warm_up_steps,
    };
    if args.arms.first() != Some(&Arm::Protected) {
        fail("the first arm of measure is protected");
    }
    let count = args.arms.len();
    for rep in 0..args.warmups + args.reps {
        let kind = if rep < args.warmups {
            "warmup"
        } else {
            "timed"
        };
        let mut outs: Vec<Option<RunOut>> = (0..count).map(|_| None).collect();
        for position in 0..count {
            let index = (rep + position) % count;
            let arm = args.arms[index];
            let out = timed_run(&built, arm, &config, &window);
            let mut row = run_json(&built, arm, &out);
            row["type"] = json!("run");
            row["rep"] = json!(rep);
            row["kind"] = json!(kind);
            row["position"] = json!(position);
            emit(row);
            if args.emit_end_state && rep == args.warmups {
                let (t_end, y_end) = out.run.end();
                emit(json!({
                    "type": "end_state",
                    "cell": built.cell.id,
                    "arm": arm.name(),
                    "rtol": config.rtol,
                    "rep": rep,
                    "end_time": t_end,
                    "end_state_fnv1a64": fingerprint(t_end, y_end),
                    "y_hex": hex(y_end),
                }));
            }
            outs[index] = Some(out);
        }
        let reference = outs[0].as_ref().expect("the protected arm ran");
        for (arm, other) in args.arms.iter().zip(&outs).skip(1) {
            let other = other.as_ref().expect("every arm ran");
            let mut row = identity_json(reference, other);
            row["type"] = json!("pair");
            row["cell"] = json!(built.cell.id);
            row["arm"] = json!(arm.name());
            row["rep"] = json!(rep);
            row["kind"] = json!(kind);
            emit(row);
        }
    }
    footer(started);
}

fn command_single(argv: &[String]) {
    let (args, built, config, started) = setup("single", argv);
    let window = Window {
        warm_up_steps: args.warm_up_steps,
    };
    let (mut attempts, mut end) = (0, String::new());
    if !args.none_arm {
        let [arm] = args.arms[..] else {
            fail("single takes exactly one arm, or none");
        };
        for _ in 0..args.runs {
            let out = timed_run(&built, arm, &config, &window);
            attempts += out.run.result.attempts;
            let (t_end, y_end) = out.run.end();
            end = fingerprint(t_end, y_end);
        }
    }
    emit(json!({
        "type": "single",
        "cell": built.cell.id,
        "arm": if args.none_arm {
            "none".to_string()
        } else {
            args.arms[0].name()
        },
        "runs": if args.none_arm { 0 } else { args.runs },
        "attempts_total": attempts,
        "end_state_fnv1a64": end,
    }));
    footer(started);
}

fn command_reference_inputs(argv: &[String]) {
    let (_args, built, _config, started) = setup("reference-inputs", argv);
    let Some(data) = &built.skewnet else {
        fail("reference-inputs needs a skewnet cell");
    };
    let cell = &built.cell;
    emit(json!({
        "type": "reference_inputs",
        "cell": cell.id,
        "n": cell.n,
        "t_end": cell.span.1,
        "formula": "phi_j(t) = atanh(alpha(t) p_j + beta(t) q_j), alpha(t) = A sin(w t + c), beta(t) = B cos(v t)",
        "alpha_amplitude": SKEWNET_ALPHA_AMPLITUDE,
        "alpha_frequency": SKEWNET_ALPHA_FREQUENCY,
        "alpha_phase": SKEWNET_ALPHA_PHASE,
        "beta_amplitude": SKEWNET_BETA_AMPLITUDE,
        "beta_frequency": SKEWNET_BETA_FREQUENCY,
        "p_hex": hex(&data.p),
        "q_hex": hex(&data.q),
        "exact_end_hex": hex(cell.exact_end.as_ref().expect("skewnet has an exact solution")),
    }));
    footer(started);
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    match argv.get(1).map(String::as_str) {
        Some("pilot") => command_pilot(&argv),
        Some("identity") => command_identity(&argv),
        Some("faults") => command_faults(&argv),
        Some("measure") => command_measure(&argv),
        Some("single") => command_single(&argv),
        Some("reference-inputs") => command_reference_inputs(&argv),
        _ => fail(
            "usage: v18-dense-lu-runner <pilot|identity|faults|measure|single|reference-inputs> \
             --cell <family>.n<order> --t-end <t> --rtol <rtol> [--max-attempts k] \
             [--arms a,b,..] [--warmups k] [--reps k] [--runs k] [--warm-up-steps k] \
             [--emit-end-state]",
        ),
    }
}
