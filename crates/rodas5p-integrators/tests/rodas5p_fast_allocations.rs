//! Allocation contract of the lean RODAS5P driver (research nodes
//! `research/stiff_rodas5p_fast_20261002` and `..._fast_v2_20261002`): after
//! its workspace is built, a step allocates nothing when the problem fills
//! its Jacobian in place. One test per binary, so the counting allocator sees
//! no other thread.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, IntegrationMethod, OutputSchedule, integrate_adaptive_observed_with_config,
    integrate_rodas5p_fast_observed, robertson_problem,
};

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let value = f();
    (value, ALLOCATIONS.load(Ordering::Relaxed) - before)
}

fn config(rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-4 * rtol,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: 40.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

#[test]
fn a_fast_step_allocates_nothing() {
    // Robertson supplies an in-place Jacobian, so after its workspace is
    // built the driver allocates nothing per step: the count is the same at
    // two tolerances whose attempt counts differ several times.
    let (problem, y0) = robertson_problem().unwrap();
    let output = OutputSchedule::new(vec![0.0, 40.0]).unwrap();
    // Warm the coefficient cache and any lazy statics first.
    integrate_rodas5p_fast_observed(&problem, (0.0, 40.0), &y0, &config(1.0e-6), &output).unwrap();
    let mut runs = Vec::new();
    for rtol in [1.0e-6, 1.0e-9] {
        let (fast, allocations) = allocations_during(|| {
            integrate_rodas5p_fast_observed(&problem, (0.0, 40.0), &y0, &config(rtol), &output)
                .unwrap()
        });
        runs.push((fast.attempts, allocations));
    }
    assert!(runs[1].0 >= 3 * runs[0].0, "{runs:?}");
    assert_eq!(
        runs[0].1, runs[1].1,
        "allocations depend on the step count: {runs:?}"
    );
    assert!(runs[0].1 <= 64, "{runs:?}");
    // The sequential driver allocates per stage.
    let direct = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let (sequential, sequential_allocations) = allocations_during(|| {
        integrate_adaptive_observed_with_config(
            &problem,
            (0.0, 40.0),
            &y0,
            IntegrationMethod::Sequential,
            Some(&direct),
            None,
            &config(1.0e-6),
            &output,
        )
        .unwrap()
    });
    let sequential_rate = sequential_allocations as f64 / sequential.diagnostics.attempts as f64;
    assert!(sequential_rate >= 100.0, "{sequential_rate}");
    eprintln!(
        "fast: {runs:?} (attempts, allocations); sequential: {sequential_rate:.1} per attempt"
    );
}
