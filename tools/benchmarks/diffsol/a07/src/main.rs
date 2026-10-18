#[cfg(feature = "allocation-audit")]
use rodas5p_core::DenseMatrix as NativeDenseMatrix;
use std::alloc::System;
#[cfg(feature = "allocation-audit")]
use std::alloc::{GlobalAlloc, Layout};
use std::io::Write;
#[cfg(feature = "allocation-audit")]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use diffsol::{DenseMatrix, FaerLU, FaerMat, MatrixCommon, OdeBuilder, OdeSolverMethod};
use rodas5p_integrators::{
    integrate_rodas5p_fast_observed, AdaptiveStepConfig, OdeProblem, OutputSchedule,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const RTOL: f64 = 1.0e-8;
const ATOL: f64 = 1.0e-11;
const POINTS: usize = 101;

#[cfg(feature = "allocation-audit")]
struct CountingAllocator;
#[cfg(feature = "allocation-audit")]
static TRACK: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "allocation-audit")]
static ALLOCS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "allocation-audit")]
static BYTES: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "allocation-audit")]
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACK.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if TRACK.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if TRACK.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(size as u64, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
#[cfg(feature = "allocation-audit")]
static ALLOCATOR: CountingAllocator = CountingAllocator;
#[global_allocator]
#[cfg(not(feature = "allocation-audit"))]
static ALLOCATOR: System = System;

#[derive(Serialize)]
struct Row {
    schema: &'static str,
    arm: String,
    protocol: String,
    status: String,
    elapsed_ns: Option<u128>,
    allocations: Option<u64>,
    allocated_bytes: Option<u64>,
    output_count: usize,
    times: Vec<f64>,
    states: Vec<[f64; 3]>,
    work_counters: Option<rodas5p_core::WorkCounters>,
    solver_counters: Option<serde_json::Value>,
    callback_counters: Option<CallbackReceipt>,
    unavailable_counters_reason: Option<String>,
    partial_solver_counters_reason: Option<String>,
    error: Option<String>,
    timing_authority: &'static str,
    grid_sha256: String,
    manifest_sha256: String,
    binary_sha256: String,
}

#[cfg(feature = "allocation-audit")]
#[derive(Default)]
struct CallbackCounters {
    rhs_calls: AtomicU64,
    rhs_batch_calls: AtomicU64,
    rhs_evaluations: AtomicU64,
    jacobian_builds: AtomicU64,
    jacobian_into_calls: AtomicU64,
    jvp_calls: AtomicU64,
    jvp_vectors: AtomicU64,
}

#[derive(Clone, Debug, Serialize)]
struct CallbackReceipt {
    rhs_calls: u64,
    rhs_batch_calls: u64,
    rhs_evaluations: u64,
    jacobian_builds: u64,
    jacobian_into_calls: u64,
    jvp_calls: u64,
    jvp_vectors: u64,
}

#[cfg(feature = "allocation-audit")]
impl CallbackCounters {
    fn snapshot(&self) -> CallbackReceipt {
        CallbackReceipt {
            rhs_calls: self.rhs_calls.load(Ordering::Relaxed),
            rhs_batch_calls: self.rhs_batch_calls.load(Ordering::Relaxed),
            rhs_evaluations: self.rhs_evaluations.load(Ordering::Relaxed),
            jacobian_builds: self.jacobian_builds.load(Ordering::Relaxed),
            jacobian_into_calls: self.jacobian_into_calls.load(Ordering::Relaxed),
            jvp_calls: self.jvp_calls.load(Ordering::Relaxed),
            jvp_vectors: self.jvp_vectors.load(Ordering::Relaxed),
        }
    }
}

struct RunFailure {
    message: String,
    solver_counters: Option<serde_json::Value>,
    #[cfg(feature = "allocation-audit")]
    callback_counters: Option<CallbackReceipt>,
    counter_unavailable_reason: Option<String>,
}

impl RunFailure {
    fn native(message: String) -> Self {
        Self {
            message,
            solver_counters: None,
            #[cfg(feature = "allocation-audit")]
            callback_counters: None,
            counter_unavailable_reason: Some(
                "native API returns no partial accepted/rejected/LU counters on failure".into(),
            ),
        }
    }
}

#[cfg(feature = "allocation-audit")]
fn counted_robertson_problem() -> Result<(OdeProblem, Vec<f64>, Arc<CallbackCounters>), String> {
    let counters = Arc::new(CallbackCounters::default());
    let rhs_counts = Arc::clone(&counters);
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        rhs_counts.rhs_calls.fetch_add(1, Ordering::Relaxed);
        rhs_counts.rhs_evaluations.fetch_add(1, Ordering::Relaxed);
        out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        out[2] = 3.0e7 * y[1] * y[1];
        Ok(())
    });
    let batch_counts = Arc::clone(&counters);
    let rhs_batch = Arc::new(move |_times: &[f64], states: &[Vec<f64>]| {
        batch_counts.rhs_batch_calls.fetch_add(1, Ordering::Relaxed);
        batch_counts
            .rhs_evaluations
            .fetch_add(states.len() as u64, Ordering::Relaxed);
        Ok(states
            .iter()
            .map(|y| {
                vec![
                    -0.04 * y[0] + 1.0e4 * y[1] * y[2],
                    0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1],
                    3.0e7 * y[1] * y[1],
                ]
            })
            .collect())
    });
    let jac_counts = Arc::clone(&counters);
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        jac_counts.jacobian_builds.fetch_add(1, Ordering::Relaxed);
        NativeDenseMatrix::new(
            3,
            3,
            vec![
                -0.04,
                1.0e4 * y[2],
                1.0e4 * y[1],
                0.04,
                -1.0e4 * y[2] - 6.0e7 * y[1],
                -1.0e4 * y[1],
                0.0,
                6.0e7 * y[1],
                0.0,
            ],
        )
    });
    let jvp_counts = Arc::clone(&counters);
    let jvp = Arc::new(move |_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        jvp_counts.jvp_calls.fetch_add(1, Ordering::Relaxed);
        jvp_counts.jvp_vectors.fetch_add(1, Ordering::Relaxed);
        out[0] = -0.04 * v[0] + 1.0e4 * y[2] * v[1] + 1.0e4 * y[1] * v[2];
        out[1] = 0.04 * v[0] + (-1.0e4 * y[2] - 6.0e7 * y[1]) * v[1] - 1.0e4 * y[1] * v[2];
        out[2] = 6.0e7 * y[1] * v[1];
        Ok(())
    });
    let jac_into_counts = Arc::clone(&counters);
    let jacobian_into = Arc::new(move |_t: f64, y: &[f64], out: &mut NativeDenseMatrix| {
        jac_into_counts
            .jacobian_into_calls
            .fetch_add(1, Ordering::Relaxed);
        out[(0, 0)] = -0.04;
        out[(0, 1)] = 1.0e4 * y[2];
        out[(0, 2)] = 1.0e4 * y[1];
        out[(1, 0)] = 0.04;
        out[(1, 1)] = -1.0e4 * y[2] - 6.0e7 * y[1];
        out[(1, 2)] = -1.0e4 * y[1];
        out[(2, 1)] = 6.0e7 * y[1];
        Ok(())
    });
    let problem = OdeProblem::new(
        "robertson",
        3,
        rhs,
        Some(rhs_batch),
        Some(jacobian),
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .map_err(|e| e.to_string())?
    .with_jacobian_into(jacobian_into);
    Ok((problem, vec![1.0, 0.0, 0.0], counters))
}

#[cfg(not(feature = "allocation-audit"))]
fn plain_robertson_problem() -> Result<(OdeProblem, Vec<f64>), String> {
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        out[2] = 3.0e7 * y[1] * y[1];
        Ok(())
    });
    let rhs_batch = Arc::new(|_times: &[f64], states: &[Vec<f64>]| {
        Ok(states
            .iter()
            .map(|y| {
                vec![
                    -0.04 * y[0] + 1.0e4 * y[1] * y[2],
                    0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1],
                    3.0e7 * y[1] * y[1],
                ]
            })
            .collect())
    });
    let jacobian = Arc::new(|_t: f64, y: &[f64]| {
        rodas5p_core::DenseMatrix::new(
            3,
            3,
            vec![
                -0.04,
                1.0e4 * y[2],
                1.0e4 * y[1],
                0.04,
                -1.0e4 * y[2] - 6.0e7 * y[1],
                -1.0e4 * y[1],
                0.0,
                6.0e7 * y[1],
                0.0,
            ],
        )
    });
    let jvp = Arc::new(|_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = -0.04 * v[0] + 1.0e4 * y[2] * v[1] + 1.0e4 * y[1] * v[2];
        out[1] = 0.04 * v[0] + (-1.0e4 * y[2] - 6.0e7 * y[1]) * v[1] - 1.0e4 * y[1] * v[2];
        out[2] = 6.0e7 * y[1] * v[1];
        Ok(())
    });
    let jacobian_into = Arc::new(|_t: f64, y: &[f64], out: &mut rodas5p_core::DenseMatrix| {
        out[(0, 0)] = -0.04;
        out[(0, 1)] = 1.0e4 * y[2];
        out[(0, 2)] = 1.0e4 * y[1];
        out[(1, 0)] = 0.04;
        out[(1, 1)] = -1.0e4 * y[2] - 6.0e7 * y[1];
        out[(1, 2)] = -1.0e4 * y[1];
        out[(2, 1)] = 6.0e7 * y[1];
        Ok(())
    });
    let problem = OdeProblem::new(
        "robertson",
        3,
        rhs,
        Some(rhs_batch),
        Some(jacobian),
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .map_err(|e| e.to_string())?
    .with_jacobian_into(jacobian_into);
    Ok((problem, vec![1.0, 0.0, 0.0]))
}

fn run_native(
    times: &[f64],
) -> Result<
    (
        Vec<f64>,
        Vec<[f64; 3]>,
        rodas5p_core::WorkCounters,
        Option<CallbackReceipt>,
    ),
    RunFailure,
> {
    #[cfg(feature = "allocation-audit")]
    let (problem, initial, callbacks) = counted_robertson_problem().map_err(RunFailure::native)?;
    #[cfg(not(feature = "allocation-audit"))]
    let (problem, initial) = plain_robertson_problem().map_err(RunFailure::native)?;
    let schedule =
        OutputSchedule::new(times.to_vec()).map_err(|e| RunFailure::native(e.to_string()))?;
    let config = AdaptiveStepConfig {
        atol: ATOL,
        rtol: RTOL,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: 40.0,
        ..AdaptiveStepConfig::default()
    };
    let result = match integrate_rodas5p_fast_observed(
        &problem,
        (
            times[0],
            *times
                .last()
                .ok_or_else(|| RunFailure::native("empty grid".into()))?,
        ),
        &initial,
        &config,
        &schedule,
    ) {
        Ok(result) => result,
        Err(error) => {
            return Err(RunFailure {
                message: error.to_string(),
                solver_counters: None,
                #[cfg(feature = "allocation-audit")]
                callback_counters: Some(callbacks.snapshot()),
                counter_unavailable_reason: Some(
                    "native API does not expose partial accepted/rejected/LU counters".into(),
                ),
            });
        }
    };
    let states = result
        .observed
        .y
        .iter()
        .map(|y| [y[0], y[1], y[2]])
        .collect();
    Ok((result.observed.t, states, result.observed.counters, {
        #[cfg(feature = "allocation-audit")]
        {
            Some(callbacks.snapshot())
        }
        #[cfg(not(feature = "allocation-audit"))]
        {
            None
        }
    }))
}

fn run_diffsol(
    times: &[f64],
) -> Result<
    (
        Vec<f64>,
        Vec<[f64; 3]>,
        serde_json::Value,
        Option<CallbackReceipt>,
    ),
    RunFailure,
> {
    #[cfg(feature = "allocation-audit")]
    let callbacks = Arc::new(CallbackCounters::default());
    #[cfg(feature = "allocation-audit")]
    let rhs_counts = Arc::clone(&callbacks);
    #[cfg(feature = "allocation-audit")]
    let jvp_counts = Arc::clone(&callbacks);
    let problem = OdeBuilder::<FaerMat<f64>>::new()
        .p([0.04, 1.0e4, 3.0e7])
        .rtol(RTOL)
        .atol([ATOL; 3])
        .h0(1.0e-6)
        .rhs_implicit(
            move |x, p, _t, out| {
                #[cfg(feature = "allocation-audit")]
                {
                    rhs_counts.rhs_calls.fetch_add(1, Ordering::Relaxed);
                    rhs_counts.rhs_evaluations.fetch_add(1, Ordering::Relaxed);
                }
                out[0] = -p[0] * x[0] + p[1] * x[1] * x[2];
                out[1] = p[0] * x[0] - p[1] * x[1] * x[2] - p[2] * x[1] * x[1];
                out[2] = p[2] * x[1] * x[1];
            },
            move |x, p, _t, v, out| {
                #[cfg(feature = "allocation-audit")]
                {
                    jvp_counts.jvp_calls.fetch_add(1, Ordering::Relaxed);
                    jvp_counts.jvp_vectors.fetch_add(1, Ordering::Relaxed);
                }
                let cross = v[1] * x[2] + x[1] * v[2];
                out[0] = -p[0] * v[0] + p[1] * cross;
                out[1] = p[0] * v[0] - p[1] * cross - 2.0 * p[2] * x[1] * v[1];
                out[2] = 2.0 * p[2] * x[1] * v[1];
            },
        )
        .init(
            |_p, _t, y| {
                y[0] = 1.0;
                y[1] = 0.0;
                y[2] = 0.0;
            },
            3,
        )
        .build()
        .map_err(|e| RunFailure {
            message: e.to_string(),
            solver_counters: None,
            #[cfg(feature = "allocation-audit")]
            callback_counters: Some(callbacks.snapshot()),
            counter_unavailable_reason: Some("solver was not constructed".into()),
        })?;
    let mut solver = problem.bdf::<FaerLU<f64>>().map_err(|e| RunFailure {
        message: e.to_string(),
        solver_counters: None,
        #[cfg(feature = "allocation-audit")]
        callback_counters: Some(callbacks.snapshot()),
        counter_unavailable_reason: Some("BDF construction failed".into()),
    })?;
    let (matrix, stop_reason) = match solver.solve_dense(times) {
        Ok(result) => result,
        Err(error) => {
            return Err(RunFailure {
                message: error.to_string(),
                solver_counters: serde_json::to_value(solver.get_statistics()).ok(),
                #[cfg(feature = "allocation-audit")]
                callback_counters: Some(callbacks.snapshot()),
                counter_unavailable_reason: None,
            });
        }
    };
    if matrix.ncols() != POINTS {
        return Err(RunFailure {
            message: format!(
                "Diffsol returned {} columns ({stop_reason:?})",
                matrix.ncols()
            ),
            solver_counters: serde_json::to_value(solver.get_statistics()).ok(),
            #[cfg(feature = "allocation-audit")]
            callback_counters: Some(callbacks.snapshot()),
            counter_unavailable_reason: None,
        });
    }
    let states = (0..POINTS)
        .map(|col| {
            [
                matrix.get_index(0, col),
                matrix.get_index(1, col),
                matrix.get_index(2, col),
            ]
        })
        .collect();
    let stats = serde_json::to_value(solver.get_statistics()).unwrap_or(serde_json::Value::Null);
    Ok((times.to_vec(), states, stats, {
        #[cfg(feature = "allocation-audit")]
        {
            Some(callbacks.snapshot())
        }
        #[cfg(not(feature = "allocation-audit"))]
        {
            None
        }
    }))
}

fn one(
    arm: &str,
    protocol: &str,
    measure_allocations: bool,
    times: &[f64],
    grid_sha256: &str,
    manifest_sha256: &str,
    binary_sha256: &str,
) -> Row {
    #[cfg(feature = "allocation-audit")]
    {
        ALLOCS.store(0, Ordering::Relaxed);
        BYTES.store(0, Ordering::Relaxed);
        TRACK.store(measure_allocations, Ordering::SeqCst);
    }
    let start = Instant::now();
    let result = match arm {
        "native" => {
            run_native(times).map(|(t, y, work, callbacks)| (t, y, Some(work), None, callbacks))
        }
        "diffsol" => run_diffsol(times)
            .map(|(t, y, solver, callbacks)| (t, y, None, Some(solver), callbacks)),
        _ => Err(RunFailure {
            message: format!("unknown arm: {arm}"),
            solver_counters: None,
            #[cfg(feature = "allocation-audit")]
            callback_counters: None,
            counter_unavailable_reason: Some("arm was not invoked".into()),
        }),
    };
    let elapsed = start.elapsed().as_nanos();
    #[cfg(feature = "allocation-audit")]
    TRACK.store(false, Ordering::SeqCst);
    #[cfg(feature = "allocation-audit")]
    let allocations = ALLOCS.load(Ordering::Relaxed);
    #[cfg(feature = "allocation-audit")]
    let allocated_bytes = BYTES.load(Ordering::Relaxed);
    #[cfg(not(feature = "allocation-audit"))]
    let (allocations, allocated_bytes) = (0, 0);
    match result {
        Ok((times, states, work, solver_counters, callback_counters)) => Row {
            schema: "vigilode-a07-run-v1",
            arm: arm.into(),
            protocol: protocol.into(),
            status: "ok".into(),
            elapsed_ns: (!measure_allocations).then_some(elapsed),
            allocations: measure_allocations.then_some(allocations),
            allocated_bytes: measure_allocations.then_some(allocated_bytes),
            output_count: states.len(),
            times,
            states,
            work_counters: work,
            solver_counters,
            callback_counters,
            unavailable_counters_reason: {
                #[cfg(feature = "allocation-audit")]
                {
                    None
                }
                #[cfg(not(feature = "allocation-audit"))]
                {
                    Some("callback counters disabled to preserve timing path".into())
                }
            },
            partial_solver_counters_reason: None,
            error: None,
            timing_authority: "diagnostic_only",
            grid_sha256: grid_sha256.into(),
            manifest_sha256: manifest_sha256.into(),
            binary_sha256: binary_sha256.into(),
        },
        Err(failure) => Row {
            schema: "vigilode-a07-run-v1",
            arm: arm.into(),
            protocol: protocol.into(),
            status: "failed".into(),
            elapsed_ns: (!measure_allocations).then_some(elapsed),
            allocations: measure_allocations.then_some(allocations),
            allocated_bytes: measure_allocations.then_some(allocated_bytes),
            output_count: 0,
            times: Vec::new(),
            states: Vec::new(),
            work_counters: None,
            solver_counters: failure.solver_counters,
            callback_counters: {
                #[cfg(feature = "allocation-audit")]
                {
                    failure.callback_counters
                }
                #[cfg(not(feature = "allocation-audit"))]
                {
                    None
                }
            },
            unavailable_counters_reason: {
                #[cfg(feature = "allocation-audit")]
                {
                    failure.counter_unavailable_reason.clone()
                }
                #[cfg(not(feature = "allocation-audit"))]
                {
                    Some("callback counters disabled to preserve timing path".into())
                }
            },
            partial_solver_counters_reason: failure.counter_unavailable_reason,
            error: Some(failure.message),
            timing_authority: "diagnostic_only",
            grid_sha256: grid_sha256.into(),
            manifest_sha256: manifest_sha256.into(),
            binary_sha256: binary_sha256.into(),
        },
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("");
    let value = args.get(2).map(String::as_str).unwrap_or("");
    if args.len() < 3 {
        eprintln!(
            "usage: --single ARM | --alloc ARM | --warm-abba BLOCKS [--grid FILE --manifest FILE]"
        );
        std::process::exit(2);
    }
    let options: Vec<_> = args[3..]
        .chunks_exact(2)
        .map(|pair| (pair[0].as_str(), pair[1].as_str()))
        .collect();
    let grid_path = options
        .iter()
        .find(|(k, _)| *k == "--grid")
        .map(|(_, v)| *v);
    let manifest_path = options
        .iter()
        .find(|(k, _)| *k == "--manifest")
        .map(|(_, v)| *v);
    let (Some(grid_path), Some(manifest_path)) = (grid_path, manifest_path) else {
        eprintln!("required: --grid GRID.json --manifest MANIFEST.json");
        std::process::exit(2);
    };
    let grid_bytes = std::fs::read(grid_path).expect("read frozen grid file");
    let grid_value: serde_json::Value =
        serde_json::from_slice(&grid_bytes).expect("parse frozen grid file");
    let times: Vec<f64> = serde_json::from_value(grid_value["times"].clone()).expect("grid times");
    if times.len() != POINTS || times.first() != Some(&0.0) || times.last() != Some(&40.0) {
        eprintln!("grid does not match frozen A07 domain and point count");
        std::process::exit(2);
    }
    let grid_sha256 = format!("{:x}", Sha256::digest(&grid_bytes));
    let manifest_bytes = std::fs::read(manifest_path).expect("read frozen manifest");
    let manifest_sha256 = format!("{:x}", Sha256::digest(&manifest_bytes));
    let executable = std::env::current_exe().expect("resolve executable path");
    let executable_bytes = std::fs::read(executable).expect("read executable for identity");
    let binary_sha256 = format!("{:x}", Sha256::digest(&executable_bytes));
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    let mut emit = |row: Row| {
        serde_json::to_writer(&mut stdout, &row).expect("serialize run row");
        stdout.write_all(b"\n").expect("write run row newline");
        stdout.flush().expect("flush run row");
    };
    if mode == "--single" {
        emit(one(
            value,
            "single_cold_process",
            false,
            &times,
            &grid_sha256,
            &manifest_sha256,
            &binary_sha256,
        ));
    } else if mode == "--alloc" {
        #[cfg(not(feature = "allocation-audit"))]
        {
            eprintln!("--alloc requires compilation with --features allocation-audit");
            std::process::exit(2);
        }
        #[cfg(feature = "allocation-audit")]
        emit(one(
            value,
            "separate_allocator_run",
            true,
            &times,
            &grid_sha256,
            &manifest_sha256,
            &binary_sha256,
        ));
    } else if mode == "--warm-abba" {
        let n: usize = value.parse().unwrap_or(0);
        let sequence = [
            "native", "diffsol", "diffsol", "native", "diffsol", "native", "native", "diffsol",
        ];
        for i in 0..n.saturating_mul(8) {
            emit(one(
                sequence[i % sequence.len()],
                "warm_abba",
                false,
                &times,
                &grid_sha256,
                &manifest_sha256,
                &binary_sha256,
            ));
        }
    } else {
        eprintln!(
            "usage: --single ARM | --alloc ARM | --warm-abba BLOCKS [--grid FILE --manifest FILE]"
        );
        std::process::exit(2);
    }
}
