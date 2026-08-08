use std::{
    alloc::{GlobalAlloc, Layout, System},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
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

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() && TRACK.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if TRACK.load(Ordering::Relaxed) {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        let new_pointer = unsafe { System.realloc(pointer, old_layout, new_size) };
        if !new_pointer.is_null() && TRACK.load(Ordering::Relaxed) {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(old_layout.size() as u64, Ordering::Relaxed);
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        }
        new_pointer
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Snapshot {
    allocations: u64,
    deallocations: u64,
    allocated_bytes: u64,
    deallocated_bytes: u64,
}

fn reset() {
    TRACK.store(false, Ordering::SeqCst);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    DEALLOCATED_BYTES.store(0, Ordering::Relaxed);
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
    }
}

#[derive(Parser)]
#[command(name = "rodas5p-allocation-audit", version)]
struct Args {
    #[arg(long)]
    trace: PathBuf,
    #[arg(long)]
    output: PathBuf,
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

fn main() -> Result<()> {
    let args = Args::parse();
    if args.repetitions == 0 {
        anyhow::bail!("repetitions must be positive");
    }
    let trace_document: TraceDocument = serde_json::from_slice(
        &fs::read(&args.trace).with_context(|| format!("reading {}", args.trace.display()))?,
    )?;
    let trace = trace_document.into_trace()?;
    let template = FairSolveConfig {
        solver: SolverKind::Gmres,
        rtol: args.rtol,
        atol: args.atol,
        restart: args.restart,
        recycle_dim: args.recycle_dim,
        hard_operator_budget: args.operator_budget,
        preconditioner: args.preconditioner.into(),
        use_previous_oracle_guess: !args.zero_guess,
    };
    template.validate()?;

    let mut rows = Vec::with_capacity(cells().len());
    for cell in cells() {
        let config = FairSolveConfig {
            solver: cell.solver,
            ..template.clone()
        };
        for _ in 0..args.warmups {
            let _ = run_trace(&trace, &config, cell.lifetime, usize::MAX)?;
        }
        let mut snapshots = Vec::with_capacity(args.repetitions);
        let mut operator_totals = Vec::with_capacity(args.repetitions);
        let mut wall_times = Vec::with_capacity(args.repetitions);
        let mut failures = 0usize;
        for repetition in 0..args.repetitions {
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
            repetitions: args.repetitions,
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
        &args.output,
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
