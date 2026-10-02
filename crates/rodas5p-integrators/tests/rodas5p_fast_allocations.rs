//! Allocation contract of the lean RODAS5P driver (research node
//! `research/stiff_rodas5p_fast_20261002`): after its workspace is built, a
//! step allocates only what the problem's Jacobian callback returns. One test
//! per binary, so the counting allocator sees no other thread.

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

#[test]
fn a_fast_step_allocates_only_the_jacobian() {
    let (problem, y0) = robertson_problem().unwrap();
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-10,
        rtol: 1.0e-6,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: 40.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(vec![0.0, 40.0]).unwrap();
    // Warm the coefficient cache and any lazy statics first.
    integrate_rodas5p_fast_observed(&problem, (0.0, 40.0), &y0, &adaptive, &output).unwrap();
    let (fast, fast_allocations) = allocations_during(|| {
        integrate_rodas5p_fast_observed(&problem, (0.0, 40.0), &y0, &adaptive, &output).unwrap()
    });
    let jacobians = fast.observed.counters.jacobian_builds as usize;
    // Workspace, output collection and result: a fixed number, independent
    // of the step count. Every other allocation is a Jacobian matrix.
    assert!(
        fast_allocations <= jacobians + 40,
        "{fast_allocations} allocations for {jacobians} Jacobians over {} attempts",
        fast.attempts
    );
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
            &adaptive,
            &output,
        )
        .unwrap()
    });
    let per_attempt = |allocations: usize, attempts: usize| allocations as f64 / attempts as f64;
    let fast_rate = per_attempt(fast_allocations, fast.attempts);
    let sequential_rate = per_attempt(sequential_allocations, sequential.diagnostics.attempts);
    assert!(fast_rate <= 1.5, "{fast_rate} allocations per attempt");
    assert!(
        sequential_rate >= 20.0 * fast_rate,
        "{sequential_rate} vs {fast_rate}"
    );
    eprintln!("allocations per attempt: fast {fast_rate:.2}, sequential {sequential_rate:.1}");
}
