//! Explicit allocation gate. Calibration failure is an environment measurement
//! failure and must not be interpreted as a solver numerical failure or zero.
use rodas5p_core::WorkCounters;
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::collective_rodas5p::CollectiveRodas5pWorkspace;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;
thread_local! { static ALLOCATIONS: Cell<usize> = const { Cell::new(0) }; }
fn count() {
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static COUNTING: Counting = Counting;

fn model(n: usize, chi: f64, kappa: f64) -> CollectiveModel {
    CollectiveModel::new(
        (0..n).map(|j| -0.8 + 1.6 * j as f64 / n as f64).collect(),
        vec![1.0 / n as f64; n],
        (0..n).map(|j| (0.25 + 0.002 * j as f64, -0.1)).collect(),
        1.2,
        chi,
        0.0,
        kappa,
    )
    .unwrap()
}
#[test]
fn calibrated_complete_attempts_allocate_nothing_after_construction() {
    let calibration = model(8, 0.3, 0.1);
    let before = ALLOCATIONS.with(Cell::get);
    let clone = std::hint::black_box(calibration.clone());
    std::hint::black_box(clone.xi().as_ptr());
    std::hint::black_box(clone.weights().as_ptr());
    std::hint::black_box(clone.initial_amplitudes().as_ptr());
    let measured = ALLOCATIONS.with(Cell::get) - before;
    assert!(
        measured >= 3,
        "ALLOCATION_CALIBRATION_FAILED: library model clone has three Vec allocations, measured {measured}; allocation evidence unavailable in this runtime"
    );
    drop(clone);

    let m = model(8, 0.3, 0.1);
    let mut y = vec![0.0; 16];
    m.initial_state(&mut y).unwrap();
    let mut out = vec![0.0; 16];
    let mut workspace = CollectiveRodas5pWorkspace::new(m).unwrap();
    let mut counters = WorkCounters::default();
    let before = ALLOCATIONS.with(Cell::get);
    for i in 0..10 {
        workspace
            .step_into(
                i as f64 * 0.01,
                &y,
                0.01,
                1e-10,
                1e-7,
                &mut out,
                &mut counters,
            )
            .unwrap();
        std::mem::swap(&mut y, &mut out);
    }
    assert_eq!(ALLOCATIONS.with(Cell::get) - before, 0);
    assert_eq!(counters.direct_solve_calls, 80);
    assert_eq!(workspace.linear_work().setup_allocations, 3);

    // The opt-in interval audit reuses the same buffers and fixed witnesses.
    let before = ALLOCATIONS.with(Cell::get);
    for i in 0..10 {
        workspace
            .step_into_with_residual_witnesses(
                0.1 + i as f64 * 0.01,
                &y,
                0.01,
                1e-10,
                1e-7,
                &mut out,
                &mut counters,
            )
            .unwrap();
        std::mem::swap(&mut y, &mut out);
    }
    assert_eq!(ALLOCATIONS.with(Cell::get) - before, 0);
    assert_eq!(counters.direct_solve_calls, 160);
    assert_eq!(workspace.linear_work().residual_witness_calls, 80);
}
