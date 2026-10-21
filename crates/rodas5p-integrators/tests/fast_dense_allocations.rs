//! Allocation contract of the fast dense driver (wave-3 node `p0-fastdense`,
//! task brief test (g)): beyond the fast driver's own allocations, a run
//! allocates only for the growth of its segment store (three growable
//! vectors: times, states, stage rows), at most 3 ceil(log2(accepted + 1)).
//! The W-filtered estimate allocates nothing per step. Allocations are
//! counted per thread, following `rodas5p_fast_allocations.rs`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use rodas5p_integrators::{
    AdaptiveStepConfig, FastInteriorControl, OutputSchedule, integrate_rodas5p_fast_dense_observed,
    integrate_rodas5p_fast_observed, robertson_problem,
};

struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.with(Cell::get);
    let value = f();
    (value, ALLOCATIONS.with(Cell::get) - before)
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

fn ceil_log2(x: usize) -> usize {
    (usize::BITS - (x - 1).leading_zeros()) as usize
}

#[test]
fn fast_dense_allocations_after_setup() {
    let (problem, y0) = robertson_problem().unwrap();
    let output = OutputSchedule::new(vec![0.0, 40.0]).unwrap();
    // Warm the coefficient cache and any lazy statics first.
    integrate_rodas5p_fast_dense_observed(
        &problem,
        (0.0, 40.0),
        &y0,
        &config(1.0e-6),
        FastInteriorControl::WFilteredEnforce,
    )
    .unwrap();
    for rtol in [1.0e-6, 1.0e-9] {
        let (fast, fast_allocations) = allocations_during(|| {
            integrate_rodas5p_fast_observed(&problem, (0.0, 40.0), &y0, &config(rtol), &output)
                .unwrap()
        });
        for control in [
            FastInteriorControl::Off,
            FastInteriorControl::WFilteredEnforce,
        ] {
            let (dense, allocations) = allocations_during(|| {
                integrate_rodas5p_fast_dense_observed(
                    &problem,
                    (0.0, 40.0),
                    &y0,
                    &config(rtol),
                    control,
                )
                .unwrap()
            });
            let accepted = dense.fast.accepted_steps;
            let budget = fast_allocations + 3 * ceil_log2(accepted + 1);
            eprintln!(
                "(g) rtol {rtol:e} {control:?}: fast {fast_allocations} allocations \
                 ({} accepted); dense {allocations} ({accepted} accepted, {} samples); budget {budget}",
                fast.accepted_steps, dense.interior.samples
            );
            assert!(allocations <= budget, "{allocations} > {budget}");
        }
    }
}
