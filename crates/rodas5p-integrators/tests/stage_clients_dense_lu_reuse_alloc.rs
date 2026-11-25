//! The dense LU reuse client allocates nothing per attempt after warm-up
//! (loop node V18). This binary holds exactly one test: the allocation
//! counter is process-global, so a second test beside it would be counted.
//!
//! Method. Every driver evaluates the in-place Jacobian once per accepted
//! state, at the start of the first attempt from it. A hook in that callback
//! reads the allocation counter into a buffer reserved before the run, so
//! the difference of two consecutive readings is everything allocated from
//! one accepted state to the next: the factorization and the eight solves
//! of every attempt of that step (the rejected ones included), the stage
//! right-hand sides, the acceptance, and the next call of the controller.
//! The first [`WARM_UP_STEPS`] steps are not counted. A step without a
//! rejection is one attempt, so the smallest difference is the allocation
//! count of one attempt.
//!
//! The sequential reuse client must show zero in every counted step. The
//! protected driver and the client-kit client are reported (both allocate
//! inside every faer factorization), and so is the reuse client with a
//! worker pool (the job queue of the pool may allocate).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rodas5p_integrators::stage_clients::DenseLuMode;

#[path = "stage_clients_dense_lu_reuse_problems.rs"]
mod problems;

use problems::{Arm, Cell, SkewnetData, StepHook, adaptive, meanfield, run_arm, skewnet};

struct Counting;

/// Allocation requests of the whole process (alloc, alloc_zeroed, realloc)
/// and the bytes they asked for.
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// Accepted steps whose allocations are not counted: the first steps run
/// together with the set-up of the driver and of the client.
const WARM_UP_STEPS: usize = 2;
/// Readings reserved per run; a run with more accepted steps fails.
const READINGS: usize = 1 << 14;

type Readings = Arc<Mutex<Vec<(u64, u64)>>>;

/// A hook that appends `(allocations, bytes)` to a reserved buffer.
fn counting_hook() -> (StepHook, Readings) {
    let readings: Readings = Arc::new(Mutex::new(Vec::with_capacity(READINGS)));
    let sink = Arc::clone(&readings);
    let hook: StepHook = Arc::new(move |_t: f64, _y: &[f64]| {
        let mut readings = sink.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            readings.len() < READINGS,
            "more accepted steps than reserved readings"
        );
        // Within the reserved capacity: no allocation.
        readings.push((
            ALLOCATIONS.load(Ordering::Relaxed),
            BYTES.load(Ordering::Relaxed),
        ));
    });
    (hook, readings)
}

/// One run of `arm` on `cell`; the row of the report.
fn measure(cell: &Cell, readings: &Readings, arm: Arm, rtol: f64) -> serde_json::Value {
    let config = adaptive(cell, rtol, 1_000_000);
    readings.lock().unwrap().clear();
    let run = run_arm(cell, &cell.problem, arm, DenseLuMode::Faer, &config).unwrap();
    assert!(
        run.result.observed.success && run.partial.is_none(),
        "{}",
        cell.id
    );
    let readings = readings.lock().unwrap();
    assert_eq!(readings.len(), run.result.accepted_steps, "{}", cell.id);
    assert!(
        readings.len() > WARM_UP_STEPS + 2,
        "{}: too few steps to measure",
        cell.id
    );
    let counted = &readings[WARM_UP_STEPS..];
    let steps: Vec<(u64, u64)> = counted
        .windows(2)
        .map(|pair| (pair[1].0 - pair[0].0, pair[1].1 - pair[0].1))
        .collect();
    let total: u64 = steps.iter().map(|step| step.0).sum();
    let total_bytes: u64 = steps.iter().map(|step| step.1).sum();
    let least = steps.iter().map(|step| step.0).min().unwrap();
    let most = steps.iter().map(|step| step.0).max().unwrap();
    let least_bytes = steps.iter().map(|step| step.1).min().unwrap();
    let multiples = least == 0 || steps.iter().all(|step| step.0.is_multiple_of(least));
    serde_json::json!({
        "case": cell.id,
        "family": cell.family,
        "n": cell.n,
        "rtol": rtol,
        "arm": arm.name(),
        "attempts": run.result.attempts,
        "accepted_steps": run.result.accepted_steps,
        "rejected_steps": run.result.rejected_steps,
        "warm_up_steps": WARM_UP_STEPS,
        "counted_steps": steps.len(),
        "counted_allocations_total": total,
        "counted_bytes_total": total_bytes,
        "allocations_per_step_min": least,
        "allocations_per_step_max": most,
        "allocations_per_attempt": least,
        "bytes_per_attempt": least_bytes,
        "every_step_is_a_multiple_of_the_minimum": multiples,
    })
}

#[test]
fn reuse_client_allocates_nothing_per_attempt_after_warm_up() {
    let mut report = Vec::new();
    for n in [128, 512] {
        let (hook, mean_readings) = counting_hook();
        let mean_cell = meanfield(n, 10.0, Some(hook));
        let (hook, skew_readings) = counting_hook();
        let data = Arc::new(SkewnetData::new(n));
        let skew_cell = skewnet(&data, 2.0, Some(hook));
        for (cell, readings) in [(&mean_cell, &mean_readings), (&skew_cell, &skew_readings)] {
            assert_eq!(
                DenseLuMode::protected_choice_at(&cell.problem, cell.span.0, &cell.y0).unwrap(),
                DenseLuMode::Faer,
                "{}",
                cell.id
            );
            // Warm the coefficient tables and any lazy statics.
            measure(cell, readings, Arm::Protected, 1.0e-4);
            for rtol in [1.0e-4, 1.0e-6] {
                for arm in [
                    Arm::Protected,
                    Arm::Kit,
                    Arm::Reuse,
                    Arm::Threaded(2),
                    Arm::Threaded(6),
                ] {
                    let row = measure(cell, readings, arm, rtol);
                    let total = row["counted_allocations_total"].as_u64().unwrap();
                    match arm {
                        Arm::Reuse => assert_eq!(
                            total,
                            0,
                            "{} with arm {} at rtol {rtol:e} allocates after warm-up: {row}",
                            cell.id,
                            arm.name()
                        ),
                        Arm::Protected | Arm::Kit => assert!(
                            total > 0,
                            "{} with arm {}: the faer path was expected to allocate: {row}",
                            cell.id,
                            arm.name()
                        ),
                        // The pool's job queue may allocate: reported.
                        Arm::Threaded(_) => {}
                    }
                    report.push(row);
                }
            }
        }
    }
    for row in &report {
        println!("DENSE_LU_REUSE_ALLOC_JSON {row}");
    }
}
