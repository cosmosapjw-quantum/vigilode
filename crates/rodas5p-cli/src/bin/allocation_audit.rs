use std::{
    alloc::{GlobalAlloc, Layout, System},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering},
    time::Instant,
};

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use rodas5p_fair_ab::{
    BenchmarkCell, FairSolveConfig, PreconditionerKind, RecycleLifetime, SolverKind, TraceDocument,
    run_trace,
};
use serde::Serialize;

struct CountingAllocator;
static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
// Live bytes relative to the window start, signed: a window may free memory
// allocated before it opened, which must not wrap (the unsigned counter
// wrapped to ~2^64 and the peak then held the wrapped value).
static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static PEAK_LIVE_BYTES: AtomicI64 = AtomicI64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            let size = layout.size() as u64;
            ALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
            let live = LIVE_BYTES.fetch_add(size as i64, Ordering::Relaxed) + size as i64;
            PEAK_LIVE_BYTES.fetch_max(live, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() && TRACK.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            let size = layout.size() as u64;
            ALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
            let live = LIVE_BYTES.fetch_add(size as i64, Ordering::Relaxed) + size as i64;
            PEAK_LIVE_BYTES.fetch_max(live, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if TRACK.load(Ordering::Relaxed) {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            let size = layout.size() as u64;
            DEALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
            LIVE_BYTES.fetch_sub(size as i64, Ordering::Relaxed);
        }
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        let new_pointer = unsafe { System.realloc(pointer, old_layout, new_size) };
        if !new_pointer.is_null() && TRACK.load(Ordering::Relaxed) {
            let old_size = old_layout.size() as u64;
            let new_sz = new_size as u64;
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(old_size, Ordering::Relaxed);
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_sz, Ordering::Relaxed);
            let diff = new_sz as i64 - old_size as i64;
            let live = LIVE_BYTES.fetch_add(diff, Ordering::Relaxed) + diff;
            PEAK_LIVE_BYTES.fetch_max(live, Ordering::Relaxed);
        }
        new_pointer
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
struct Snapshot {
    allocations: u64,
    deallocations: u64,
    allocated_bytes: u64,
    deallocated_bytes: u64,
    /// Peak of live bytes inside the window minus live bytes at its start.
    peak_live_bytes: i64,
}

fn reset() {
    TRACK.store(false, Ordering::SeqCst);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    DEALLOCATED_BYTES.store(0, Ordering::Relaxed);
    LIVE_BYTES.store(0, Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(0, Ordering::Relaxed);
}

struct Guard;
impl Guard {
    fn start() -> Self {
        reset();
        TRACK.store(true, Ordering::SeqCst);
        Self
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        TRACK.store(false, Ordering::SeqCst);
    }
}

fn snapshot() -> Snapshot {
    Snapshot {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
        allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
        deallocated_bytes: DEALLOCATED_BYTES.load(Ordering::Relaxed),
        peak_live_bytes: PEAK_LIVE_BYTES.load(Ordering::Relaxed),
    }
}

#[path = "../stiff_benchmark.rs"]
#[allow(dead_code, unused_imports)]
mod stiff_benchmark;

#[path = "../triage.rs"]
#[allow(dead_code, unused_imports)]
mod triage;

#[path = "../triage_arms.rs"]
#[allow(dead_code, unused_imports)]
mod triage_arms;

#[derive(Parser)]
#[command(name = "rodas5p-allocation-audit", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<AuditSubcommand>,

    #[arg(long)]
    trace: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value_t = 3)]
    repetitions: usize,
    #[arg(long, default_value_t = 1)]
    warmups: usize,
    #[arg(long, default_value_t = 20)]
    restart: usize,
    #[arg(long, default_value_t = 6)]
    recycle_dim: usize,
    #[arg(long, default_value_t = 2000)]
    operator_budget: u64,
    #[arg(long, default_value_t = 1e-9)]
    rtol: f64,
    #[arg(long, default_value_t = 1e-12)]
    atol: f64,
    #[arg(long, value_enum, default_value_t = CliPreconditioner::None)]
    preconditioner: CliPreconditioner,
    #[arg(long)]
    zero_guess: bool,
}

#[derive(clap::Subcommand)]
enum AuditSubcommand {
    /// The ME01 stiff-arm cells: per cell one cold integration window, then
    /// `warmups` untimed integrations, then `windows` integrations each in
    /// its own raw counting window (the problem is built outside every
    /// window).
    #[command(name = "stiff-arms")]
    StiffArms {
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 2)]
        warmups: usize,
        #[arg(long, default_value_t = 3)]
        windows: usize,
    },
    /// Component triage (`research/component_triage_20261019`,
    /// `HARNESS_CONTRACT.md` section 2): one cell, one cold integration,
    /// `warmups` untimed integrations, then `windows` integrations each in
    /// its own raw counting window; prints `vigilode-triage-alloc-v1`.
    #[command(name = "triage")]
    Triage {
        #[command(flatten)]
        cell: triage::CellArgs,
        #[arg(long, default_value_t = 2)]
        warmups: usize,
        #[arg(long, default_value_t = 3)]
        windows: usize,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliPreconditioner {
    None,
    Jacobi,
}
impl From<CliPreconditioner> for PreconditionerKind {
    fn from(value: CliPreconditioner) -> Self {
        match value {
            CliPreconditioner::None => Self::None,
            CliPreconditioner::Jacobi => Self::Jacobi,
        }
    }
}

#[derive(Serialize)]
struct Row {
    solver: SolverKind,
    lifetime: RecycleLifetime,
    repetitions: usize,
    failures: usize,
    allocations: u64,
    deallocations: u64,
    allocated_bytes: u64,
    deallocated_bytes: u64,
    operator_total: u64,
    wall_seconds: f64,
}

#[derive(Serialize)]
struct Document {
    schema: &'static str,
    trace_id: String,
    diagnostic_scope: &'static str,
    allocator: &'static str,
    config: FairSolveConfig,
    rows: Vec<Row>,
}

fn cells() -> [BenchmarkCell; 7] {
    [
        BenchmarkCell::new(SolverKind::Gmres, RecycleLifetime::Off),
        BenchmarkCell::new(SolverKind::Lgmres, RecycleLifetime::Off),
        BenchmarkCell::new(SolverKind::Lgmres, RecycleLifetime::Stage),
        BenchmarkCell::new(SolverKind::Lgmres, RecycleLifetime::Persistent),
        BenchmarkCell::new(SolverKind::Gcrodr, RecycleLifetime::Off),
        BenchmarkCell::new(SolverKind::Gcrodr, RecycleLifetime::Stage),
        BenchmarkCell::new(SolverKind::Gcrodr, RecycleLifetime::Persistent),
    ]
}

fn median_u64(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    values[values.len() / 2]
}
fn median_f64(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}
fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)
        .with_context(|| format!("writing {}", path.display()))
}

/// The raw counts of one counting window (one integration).
#[derive(Clone, Copy, Debug, Default, Serialize)]
struct WindowCounts {
    allocations: u64,
    deallocations: u64,
    allocated_bytes: u64,
    /// Peak of live bytes inside the window minus live bytes at its start.
    peak_live_bytes: i64,
}

impl From<Snapshot> for WindowCounts {
    fn from(s: Snapshot) -> Self {
        Self {
            allocations: s.allocations,
            deallocations: s.deallocations,
            allocated_bytes: s.allocated_bytes,
            peak_live_bytes: s.peak_live_bytes,
        }
    }
}

/// The figures of the ME01 consumers (`tools/me01_full_cost_check.py`
/// reads `allocations` and `allocated_bytes` of `warm_2_minus_1` and
/// `cold_1_minus_0`). The key names are kept; the values are now raw window
/// counts, not differences (see [`StiffAllocationEntry`]).
#[derive(Clone, Copy, Debug, Default, Serialize)]
struct DiffSnapshot {
    allocations: u64,
    allocated_bytes: u64,
    peak_live_bytes: i64,
}

impl From<WindowCounts> for DiffSnapshot {
    fn from(w: WindowCounts) -> Self {
        Self {
            allocations: w.allocations,
            allocated_bytes: w.allocated_bytes,
            peak_live_bytes: w.peak_live_bytes,
        }
    }
}

/// One stiff-arm cell. `cold_1_minus_0` is the raw count of the first
/// integration window and `warm_2_minus_1` that of the first window after
/// the warmups (the earlier tool subtracted cumulative snapshots with
/// `saturating_sub`, so one-time lazy allocations of the first run hid the
/// warm figure); `cold_window` and `warm_windows` are every raw window.
#[derive(Serialize)]
struct StiffAllocationEntry {
    problem: String,
    arm: String,
    rtol: f64,
    success: bool,
    warm_2_minus_1: DiffSnapshot,
    cold_1_minus_0: DiffSnapshot,
    cold_window: WindowCounts,
    warm_windows: Vec<WindowCounts>,
}

#[derive(Serialize)]
struct StiffAllocationDocument {
    schema: &'static str,
    windowing: &'static str,
    warmups: usize,
    windows: usize,
    binary_sha256: String,
    entries: Vec<StiffAllocationEntry>,
}

/// Count one call of `f` in its own window: counters reset at the start,
/// raw counts at the end, no subtraction.
fn counted<T>(f: impl FnOnce() -> T) -> (T, WindowCounts) {
    let guard = Guard::start();
    let value = f();
    drop(guard);
    (value, snapshot().into())
}

fn all_stiff_cells() -> Vec<(&'static str, &'static str, f64)> {
    let mut cells = Vec::new();
    // Small problems at 1e-6, 1e-8
    for &prob in &["robertson", "hires", "van-der-pol-mu1000"] {
        for &rtol in &[1.0e-6, 1.0e-8] {
            cells.push((prob, "rodas5p-fast-small", rtol));
            cells.push((prob, "rodas5p-fast-small-ovh", rtol));
            cells.push((prob, "rodas5p-fast-small-static-stages", rtol));
            cells.push((prob, "rodas5p-fast", rtol));
        }
    }
    // Brusselator-200 for E3
    for &rtol in &[1.0e-6, 1.0e-8] {
        cells.push(("brusselator-1d-200", "rodas5p-fast-ovh", rtol));
    }
    // Brusselators for E4, E6 at 1e-6, 1e-8
    for &prob in &[
        "brusselator-1d-50",
        "brusselator-1d-200",
        "brusselator-1d-500",
    ] {
        for &rtol in &[1.0e-6, 1.0e-8] {
            cells.push((prob, "rodas5p-fast-banded", rtol));
            cells.push((prob, "rodas5p-fast-colext64", rtol));
            cells.push((prob, "rodas5p-fast", rtol));
        }
    }
    // Matrix-free candidates (E1, E7, E8) on JVP-only Brusselator at 1e-6, 1e-8
    for &prob in &["brusselator-1d-50", "brusselator-1d-160"] {
        for &rtol in &[1.0e-6, 1.0e-8] {
            cells.push((prob, "rodas5p-mf-legacy-v2", rtol));
            cells.push((prob, "rodas5p-mf-legacy", rtol));
            cells.push((prob, "rodas5p-mf-ls-ws", rtol));
            cells.push((prob, "rodas5p-mf-into", rtol));
            cells.push((prob, "rodas5p-mf", rtol));
        }
    }
    // E2: CT01 half-decade rungs:
    // van der Pol and Brusselator-50: 9 rungs 1e-3 to 1e-7 (q = 6..=14)
    for &prob in &["van-der-pol-mu1000", "brusselator-1d-50"] {
        for q in 6..=14 {
            let rtol = 10.0_f64.powf(-(q as f64) / 2.0);
            cells.push((prob, "rodas5p-fast-pred2", rtol));
            cells.push((prob, "rodas5p-fast-int-grid", rtol));
        }
    }
    // HIRES and Robertson: 15 rungs 1e-3 to 1e-10 (q = 6..=20)
    for &prob in &["hires", "robertson"] {
        for q in 6..=20 {
            let rtol = 10.0_f64.powf(-(q as f64) / 2.0);
            cells.push((prob, "rodas5p-fast-pred2", rtol));
            cells.push((prob, "rodas5p-fast-int-grid", rtol));
        }
    }
    // Deduplicate preserving order
    let mut deduped = Vec::new();
    for cell in cells {
        if !deduped.iter().any(|c: &(&str, &str, f64)| {
            c.0 == cell.0 && c.1 == cell.1 && (c.2 - cell.2).abs() < 1e-15 * c.2
        }) {
            deduped.push(cell);
        }
    }
    deduped
}

fn run_stiff_arms_audit(output: &Path, warmups: usize, windows: usize) -> Result<()> {
    anyhow::ensure!(windows >= 1, "at least one warm window");
    let cells = all_stiff_cells();
    let mut entries = Vec::with_capacity(cells.len());
    for (problem_id, arm, rtol) in cells {
        // Built outside every window: a window counts one integration.
        let problem = stiff_benchmark::stiff_profile_problem(problem_id)?;
        let mut success = true;
        let (run, cold) =
            counted(|| stiff_benchmark::run_stiff_integration_on(&problem, arm, rtol));
        success &= run.is_ok();
        drop(run);
        for _ in 0..warmups {
            success &= stiff_benchmark::run_stiff_integration_on(&problem, arm, rtol).is_ok();
        }
        let mut warm = Vec::with_capacity(windows);
        for _ in 0..windows {
            let (run, counts) =
                counted(|| stiff_benchmark::run_stiff_integration_on(&problem, arm, rtol));
            success &= run.is_ok();
            drop(run);
            warm.push(counts);
        }
        entries.push(StiffAllocationEntry {
            problem: problem_id.to_string(),
            arm: arm.to_string(),
            rtol,
            success,
            warm_2_minus_1: warm[0].into(),
            cold_1_minus_0: cold.into(),
            cold_window: cold,
            warm_windows: warm,
        });
    }

    let exe = std::env::current_exe()?;
    let exe_bytes = fs::read(&exe).with_context(|| format!("reading {}", exe.display()))?;
    let binary_sha256 = rodas5p_core::sha256_hex(&exe_bytes);

    write_json(
        output,
        &StiffAllocationDocument {
            schema: "rodas5p-stiff-allocations-v1",
            windowing: "raw-window-per-integration-after-warmups-v2",
            warmups,
            windows,
            binary_sha256,
            entries,
        },
    )
}

/// `triage` mode (`HARNESS_CONTRACT.md` section 2): one cold window, then
/// `warmups` untimed integrations, then `windows` windows of one
/// integration each. The cell (problem, configurations, output schedules)
/// is built before the first window and the segment-result buffer is
/// reserved and cleared outside the windows, so a window holds the
/// integrator calls only.
fn run_triage_audit(
    args: &triage::CellArgs,
    warmups: usize,
    windows: usize,
) -> Result<serde_json::Value> {
    let mut cell = triage::Cell::new(args)?;
    let mut runs = Vec::with_capacity(cell.segment_count());
    let ((), cold) = counted(|| cell.integrate_into(&mut runs));
    let status = cell.merge(std::mem::take(&mut runs)).status;
    runs.reserve(cell.segment_count());
    for _ in 0..warmups {
        cell.integrate_into(&mut runs);
        runs.clear();
    }
    let mut counts = Vec::with_capacity(windows);
    for _ in 0..windows {
        let ((), window) = counted(|| cell.integrate_into(&mut runs));
        runs.clear();
        counts.push(window);
    }
    Ok(serde_json::json!({
        "schema": "vigilode-triage-alloc-v1",
        "arm": args.arm,
        "problem": args.problem,
        "rtol": args.rtol,
        "grid": args.grid.id(),
        "h0_scale": args.h0_scale,
        "warmups": warmups,
        "status": status.id(),
        "cold": cold,
        "windows": counts,
    }))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(AuditSubcommand::StiffArms {
            output,
            warmups,
            windows,
        }) => return run_stiff_arms_audit(&output, warmups, windows),
        Some(AuditSubcommand::Triage {
            cell,
            warmups,
            windows,
        }) => return triage::print_or_exit(run_triage_audit(&cell, warmups, windows)),
        None => {}
    }
    let trace_path = cli
        .trace
        .ok_or_else(|| anyhow::anyhow!("--trace required when not running subcommand"))?;
    let output_path = cli
        .output
        .ok_or_else(|| anyhow::anyhow!("--output required when not running subcommand"))?;
    if cli.repetitions == 0 {
        anyhow::bail!("repetitions must be positive");
    }
    let trace_document: TraceDocument = serde_json::from_slice(
        &fs::read(&trace_path).with_context(|| format!("reading {}", trace_path.display()))?,
    )?;
    let trace = trace_document.into_trace()?;
    let template = FairSolveConfig {
        solver: SolverKind::Gmres,
        rtol: cli.rtol,
        atol: cli.atol,
        restart: cli.restart,
        recycle_dim: cli.recycle_dim,
        hard_operator_budget: cli.operator_budget,
        preconditioner: cli.preconditioner.into(),
        use_previous_oracle_guess: !cli.zero_guess,
    };
    template.validate()?;

    let mut rows = Vec::with_capacity(cells().len());
    for cell in cells() {
        let config = FairSolveConfig {
            solver: cell.solver,
            ..template.clone()
        };
        for _ in 0..cli.warmups {
            let _ = run_trace(&trace, &config, cell.lifetime, usize::MAX)?;
        }
        let mut snapshots = Vec::with_capacity(cli.repetitions);
        let mut operator_totals = Vec::with_capacity(cli.repetitions);
        let mut wall_times = Vec::with_capacity(cli.repetitions);
        let mut failures = 0usize;
        for repetition in 0..cli.repetitions {
            let timer = Instant::now();
            let guard = Guard::start();
            let run = run_trace(&trace, &config, cell.lifetime, repetition);
            drop(guard);
            let elapsed = timer.elapsed().as_secs_f64();
            let allocation = snapshot();
            let run = run?;
            snapshots.push(allocation);
            operator_totals.push(run.ledger.operator_total());
            wall_times.push(elapsed);
            failures += run.failures;
        }
        rows.push(Row {
            solver: cell.solver,
            lifetime: cell.lifetime,
            repetitions: cli.repetitions,
            failures,
            allocations: median_u64(snapshots.iter().map(|value| value.allocations).collect()),
            deallocations: median_u64(snapshots.iter().map(|value| value.deallocations).collect()),
            allocated_bytes: median_u64(
                snapshots
                    .iter()
                    .map(|value| value.allocated_bytes)
                    .collect(),
            ),
            deallocated_bytes: median_u64(
                snapshots
                    .iter()
                    .map(|value| value.deallocated_bytes)
                    .collect(),
            ),
            operator_total: median_u64(operator_totals),
            wall_seconds: median_f64(wall_times),
        });
    }

    write_json(
        &output_path,
        &Document {
            schema: "rodas5p-rust-allocation-audit-v1",
            trace_id: trace.trace_id,
            diagnostic_scope: "full_trace_run_including_result_materialization",
            allocator: "std::alloc::System_with_atomic_event_counters",
            config: template,
            rows,
        },
    )
}
