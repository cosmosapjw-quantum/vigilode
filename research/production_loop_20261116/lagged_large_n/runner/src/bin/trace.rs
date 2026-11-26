//! Exploratory trace program of loop L1 node V09_LAGGED_LARGE_N (`../../PREDECLARED.json`,
//! "trace"). Never used for a counted or timed measurement.
//!
//! It runs one integration of an arm with logging callbacks: every right-hand side (`R`),
//! Jacobian (`J`) and Jacobian-vector product (`V`) call is logged with its time argument and
//! with the number of matrix-sized heap allocations so far (a counting global allocator; on the
//! faer path every factorisation allocates two matrices, which places the factorisations between
//! the logged calls). The lagged arms without a JVP (`P`, `S`) are traced through a "tracing
//! JVP" that forms `J(t, y) x` with the problem's own dense fill and `DenseMatrix::matvec_into`,
//! the driver's own product: the trajectory is that of the arm without a JVP bit for bit (the
//! parser checks the end state and the lagged work against the counts run); only the counters
//! of the products differ.
//!
//! Usage: `v09-trace --family F --size S --rtol R --arm A`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rodas5p_core::{DenseMatrix, rodas5p_coefficients};
use serde_json::{Value, json};
use v09::harness::{
    Arm, Callbacks, build_problem, cpus_allowed, loadavg, problem_json, record_json, run_once,
    unix_time,
};
use v09::problems::{JvpFn, RhsFn, definition, sizes};

/// Allocations with a size in `[LOW, HIGH]` are counted (set before the run).
static LOW: AtomicUsize = AtomicUsize::new(usize::MAX);
static HIGH: AtomicUsize = AtomicUsize::new(0);
static LARGE: AtomicU64 = AtomicU64::new(0);
const SIZES_KEPT: usize = 16;
static SIZES: [AtomicUsize; SIZES_KEPT] = [const { AtomicUsize::new(0) }; SIZES_KEPT];

struct CountingAlloc;

impl CountingAlloc {
    #[inline]
    fn note(size: usize) {
        if size >= LOW.load(Ordering::Relaxed) && size <= HIGH.load(Ordering::Relaxed) {
            let k = LARGE.fetch_add(1, Ordering::Relaxed) as usize;
            if k < SIZES_KEPT {
                SIZES[k].store(size, Ordering::Relaxed);
            }
        }
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Self::note(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        Self::note(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        Self::note(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

#[derive(Default)]
struct Log {
    kinds: String,
    times: Vec<f64>,
    large: Vec<u64>,
}

fn push(log: &Mutex<Log>, kind: char, t: f64) {
    let mut log = log.lock().unwrap_or_else(|e| e.into_inner());
    log.kinds.push(kind);
    log.times.push(t);
    log.large.push(LARGE.load(Ordering::Relaxed));
}

fn fail(message: &str) -> ! {
    eprintln!("v09-trace: {message}");
    std::process::exit(2)
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let (mut family, mut size, mut rtol, mut arm) = (None, None, None, None);
    let mut it = argv.iter().skip(1);
    while let Some(flag) = it.next() {
        let value = it
            .next()
            .unwrap_or_else(|| fail(&format!("{flag} needs a value")));
        match flag.as_str() {
            "--family" => family = Some(value.clone()),
            "--size" => size = value.parse::<usize>().ok(),
            "--rtol" => rtol = value.parse::<f64>().ok(),
            "--arm" => arm = Arm::parse(value),
            other => fail(&format!("unknown flag {other}")),
        }
    }
    let family = family.unwrap_or_else(|| fail("--family is required"));
    let size = size.unwrap_or_else(|| fail("--size is required"));
    let rtol = rtol.unwrap_or_else(|| fail("--rtol is required"));
    let arm = arm.unwrap_or_else(|| fail("--arm is required"));
    if !sizes(&family).contains(&size) {
        fail("undeclared size");
    }
    let def = definition(&family, size).unwrap_or_else(|| fail("unknown family"));
    let n = def.n;
    let log = Arc::new(Mutex::new(Log::default()));

    let plain = Callbacks::plain(&def, arm.uses_jvp());
    let rhs: RhsFn = {
        let (inner, log) = (plain.rhs.clone(), Arc::clone(&log));
        Arc::new(move |t, y, out| {
            push(&log, 'R', t);
            inner(t, y, out)
        })
    };
    let jacobian_into = {
        let (inner, log) = (plain.jacobian_into.clone(), Arc::clone(&log));
        Arc::new(move |t: f64, y: &[f64], out: &mut DenseMatrix| {
            push(&log, 'J', t);
            inner(t, y, out)
        })
    };
    // The JVP of the traced problem: the analytic one for PJ and SJ, the tracing JVP (the
    // driver's dense product, see the module docs) for every other arm.
    let jvp: JvpFn = if arm.uses_jvp() {
        let (inner, log) = (def.jvp.clone(), Arc::clone(&log));
        Arc::new(move |t, y, v, out| {
            push(&log, 'V', t);
            inner(t, y, v, out)
        })
    } else {
        let fill = def.dense_fill.clone();
        let cache = Mutex::new((false, f64::NAN, vec![0.0_f64; n], DenseMatrix::zeros(n, n)));
        let log = Arc::clone(&log);
        Arc::new(move |t, y, v, out| {
            push(&log, 'V', t);
            let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
            let (valid, ct, cy, matrix) = &mut *guard;
            let same = *valid
                && ct.to_bits() == t.to_bits()
                && cy.iter().zip(y).all(|(a, b)| a.to_bits() == b.to_bits());
            if !same {
                fill(y, matrix);
                *ct = t;
                cy.copy_from_slice(y);
                *valid = true;
            }
            matrix.matvec_into(v, out)
        })
    };
    let callbacks = Callbacks {
        rhs,
        jacobian_into,
        jvp: Some(jvp),
    };
    let problem = build_problem(&def, &callbacks, arm).unwrap_or_else(|e| fail(&e.to_string()));
    let c = rodas5p_coefficients()
        .map(|k| k.c.clone())
        .unwrap_or_default();
    // Matrix-sized allocations: n^2 doubles, with room for faer's column padding.
    let bytes = n * n * std::mem::size_of::<f64>();
    let slack = 16 * n * std::mem::size_of::<f64>() + 4096;
    let large_before = LARGE.load(Ordering::Relaxed);
    {
        // Reserve the log before counting so that its growth stays small.
        let mut guard = log.lock().unwrap_or_else(|e| e.into_inner());
        guard.kinds.reserve(1 << 16);
        guard.times.reserve(1 << 16);
        guard.large.reserve(1 << 16);
    }
    LOW.store(bytes, Ordering::Relaxed);
    HIGH.store(bytes + slack, Ordering::Relaxed);
    let loadavg_start = loadavg();
    let outcome = run_once(arm, &problem, &def, rtol);
    let large_after = LARGE.load(Ordering::Relaxed);
    LOW.store(usize::MAX, Ordering::Relaxed);
    HIGH.store(0, Ordering::Relaxed);
    let record = outcome.unwrap_or_else(|e| fail(&e.to_string()));
    let guard = log.lock().unwrap_or_else(|e| e.into_inner());
    let sizes_seen: Vec<usize> = SIZES
        .iter()
        .map(|s| s.load(Ordering::Relaxed))
        .filter(|s| *s > 0)
        .collect();
    let mut result: Value = record_json(&record, true);
    result["type"] = json!("trace");
    result["arm"] = json!(arm.id());
    result["rtol"] = json!(rtol);
    result["problem"] = problem_json(&def);
    result["coefficients_c"] = json!(c);
    result["traced_jvp"] = json!(if arm.uses_jvp() {
        "analytic"
    } else {
        "tracing-dense-product"
    });
    result["large_allocation_window_bytes"] = json!([bytes, bytes + slack]);
    result["large_allocations_before_run"] = json!(large_before);
    result["large_allocations_after_run"] = json!(large_after);
    result["large_allocation_sizes_first"] = json!(sizes_seen);
    result["events_kinds"] = json!(guard.kinds);
    result["events_t"] = json!(guard.times);
    result["events_large"] = json!(guard.large);
    result["cpus_allowed_list"] = json!(cpus_allowed());
    result["loadavg_start"] = loadavg_start;
    result["loadavg_end"] = loadavg();
    result["unix_time_end"] = json!(unix_time());
    let mut out = std::io::stdout().lock();
    writeln!(out, "{result}").expect("stdout");
}
