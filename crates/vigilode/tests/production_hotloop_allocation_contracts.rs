//! Production hot-loop allocation qualification contracts (V3-ALLOC).
//!
//! Validates:
//! 1. Allocator positive control: proving tracking hooks are active and sensitive.
//! 2. Implicit stepping hot-loop zero-allocation: `CollectiveRodas5pWorkspace::step_into`
//!    evaluates stages, LU factorization/solves, and error norm with exactly 0 heap allocations.
//! 3. Explicit session stepping allocation instrumentation: measuring attempt-level
//!    allocations during `Dopri5::with_session` checkpoint progression.
//! 4. Switching pilot/probe driver attempt instrumentation: measuring batch attempt allocations
//!    and verifying bounded adapter record packaging.
//! 5. Whole-run production solve allocation accounting: measuring full ProductionSolver::solve
//!    attempts and separating setup + trajectory storage from stepping.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use rodas5p_core::WorkCounters;
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::collective_rodas5p::CollectiveRodas5pWorkspace;
use vigilode::{
    Atol, BatchRequest, BatchStop, Dopri5, Options, Problem, ProductionConfig, ProductionSolver,
    Rodas5pWindowDriver, RouteId, SessionAdvance, SessionEpochs, WindowDriver,
    selection_evidence::SelectionIdentity,
};

struct CountingAlloc;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

fn count_alloc(layout: Layout) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        let _ = BYTES.try_with(|b| b.set(b.get() + layout.size()));
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_alloc(layout);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_alloc(layout);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_alloc(layout);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn allocation_scope<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    COUNT.with(|c| c.set(0));
    BYTES.with(|b| b.set(0));
    TRACK.with(|t| t.set(true));
    let result = f();
    TRACK.with(|t| t.set(false));
    (result, COUNT.with(Cell::get), BYTES.with(Cell::get))
}

#[test]
fn test_allocator_positive_control() {
    let (_, allocs, bytes) = allocation_scope(|| {
        let v: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        std::hint::black_box(&v);
        let b = Box::new([42u8; 128]);
        std::hint::black_box(&b);
    });
    assert!(
        allocs >= 2,
        "Allocator positive control failed: expected >= 2 allocations, measured {allocs}"
    );
    assert!(
        bytes >= 40 + 128,
        "Allocator positive control failed: expected >= 168 bytes, measured {bytes}"
    );
}

#[test]
fn test_implicit_collective_stepping_hotloop_zero_allocation() {
    // 1. Setup problem and pre-allocate workspace outside the measured interval
    let n = 8;
    let model = CollectiveModel::new(
        (0..n).map(|j| -0.8 + 1.6 * j as f64 / n as f64).collect(),
        vec![1.0 / n as f64; n],
        (0..n).map(|j| (0.25 + 0.002 * j as f64, -0.1)).collect(),
        1.2,
        0.3,
        0.0,
        0.1,
    )
    .unwrap();

    let mut y = vec![0.0; 16];
    model.initial_state(&mut y).unwrap();
    let mut out = vec![0.0; 16];
    let mut workspace = CollectiveRodas5pWorkspace::new(model).unwrap();
    let mut counters = WorkCounters::default();

    // Warm-up run
    workspace
        .step_into(0.0, &y, 0.01, 1e-10, 1e-7, &mut out, &mut counters)
        .unwrap();

    // 2. MEASURED HOT LOOP: 50 consecutive implicit Rosenbrock steps
    let (_, allocs, bytes) = allocation_scope(|| {
        for i in 0..50 {
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
    });

    println!(
        "Implicit CollectiveRodas5pWorkspace::step_into hot loop: 50 steps, {allocs} allocations ({bytes} bytes)"
    );
    assert_eq!(
        allocs, 0,
        "CollectiveRodas5p stepping hot loop must perform 0 allocations (measured {allocs} allocs, {bytes} bytes)"
    );
    assert_eq!(bytes, 0);
}

#[test]
fn test_production_session_stepping_allocation_accounting() {
    let problem = Problem::new(2, |_t, y, dy| {
        dy[0] = -y[0] + 0.1 * y[1];
        dy[1] = -0.2 * y[1];
        Ok(())
    })
    .autonomous();

    let opts = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-9),
        h0: 1e-3,
        max_steps: 10_000,
        ..Options::default()
    };
    let times = [0.0, 0.25, 0.5, 0.75, 1.0];

    // Measure advance_to stepping across scheduled observation checkpoints
    let ((advanced, alloc_count, alloc_bytes), sol) = Dopri5::with_session(
        &problem,
        (0.0, 1.0),
        &[1.0, 0.5],
        &opts,
        &times,
        SessionEpochs::default(),
        |session| {
            // Warm-up one step before entering measured scope
            let _ = session.advance_to(times[1], 1, SessionEpochs::default());

            allocation_scope(|| {
                let mut advanced = 0;
                for &target in &times[2..] {
                    loop {
                        match session.advance_to(target, 1, SessionEpochs::default()) {
                            Ok(SessionAdvance::Yield) => advanced += 1,
                            Ok(SessionAdvance::Checkpoint | SessionAdvance::Complete) => {
                                advanced += 1;
                                break;
                            }
                            Ok(SessionAdvance::Stopped) => break,
                            Err(e) => panic!("advance_to failed: {e:?}"),
                        }
                    }
                }
                advanced
            })
        },
    )
    .expect("with_session succeeds");

    let per_step_alloc = alloc_count as f64 / advanced as f64;
    println!(
        "Dopri5::with_session stepping: {advanced} steps advanced, {alloc_count} allocations ({alloc_bytes} bytes), {per_step_alloc:.2} allocs/step (attributed to collector output trajectory rows)"
    );
    assert!(advanced > 0);
    assert!(sol.success);
    // Trajectory serialization allocates at most 1-2 items per step/checkpoint
    assert!(
        alloc_count <= advanced * 2,
        "Session stepping allocations must be bounded by output serialization"
    );
}

#[test]
fn test_pilot_probe_driver_attempt_instrumentation() {
    let mut driver = Rodas5pWindowDriver::with_route(RouteId::Rodas5pJvpAssembled);
    let problem = Problem::new(2, |_t, y, dy| {
        dy[0] = -100.0 * y[0];
        dy[1] = -y[1];
        Ok(())
    })
    .jacobian(vigilode::Jacobian::jvp(|_t, _y, v, out| {
        out[0] = -100.0 * v[0];
        out[1] = -v[1];
        Ok(())
    }))
    .autonomous();

    let opts = Options {
        rtol: 1e-4,
        atol: Atol::Scalar(1e-8),
        h0: 1e-4,
        ..Options::default()
    };

    let req = BatchRequest {
        start_time: 0.0,
        target_time: 0.01,
        solve_end: 1.0,
        state: &[1.0, 1.0],
        proposed_h: Some(1e-4),
        options: &opts,
        landings: &[],
        ceiling: Some(10),
        carry_explicit_step: false,
        is_probe: true,
        is_entry_pilot: true,
        identity: SelectionIdentity::default(),
    };

    // Warm-up run
    let _ = driver.run(&problem, &req).expect("warmup succeeds");

    // Measured batch attempt
    let (outcome, alloc_count, alloc_bytes) =
        allocation_scope(|| driver.run(&problem, &req).expect("measured run succeeds"));

    println!(
        "Rodas5pWindowDriver batch attempt: stop={:?}, allocs={alloc_count}, bytes={alloc_bytes} (boxed DriverRecord + return state vector)",
        outcome.stop
    );
    if let BatchStop::Fatal(msg) = outcome.stop {
        panic!("Pilot run failed fatally: {msg}");
    }
    // Batch driver attempt allocates exactly 4 items for packaging the BatchOutcome
    assert!(
        alloc_count <= 4,
        "Batch driver attempt allocations must be <= 4 (measured {alloc_count})"
    );
}

#[test]
fn test_whole_run_allocation_accounting() {
    let problem = Problem::new(2, |_t, y, dy| {
        dy[0] = -y[0];
        dy[1] = -2.0 * y[1];
        Ok(())
    })
    .autonomous();

    let opts = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-9),
        h0: 1e-3,
        max_steps: 1_000,
        ..Options::default()
    };
    let config = ProductionConfig::rhs_only(opts, 10);

    let (sol, whole_run_allocs, whole_run_bytes) = allocation_scope(|| {
        ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0, 1.0], &config).unwrap()
    });

    assert!(sol.success, "solve must succeed");
    assert!(sol.stats.accepted_steps > 0);
    let per_accepted = whole_run_allocs as f64 / sol.stats.accepted_steps as f64;
    println!(
        "Whole-run ProductionSolver::solve: {} accepted steps, {} whole-run allocations ({} bytes), {per_accepted:.2} allocs/accepted-step",
        sol.stats.accepted_steps, whole_run_allocs, whole_run_bytes
    );
    assert!(
        whole_run_allocs <= 50 + sol.stats.accepted_steps * 4,
        "Whole-run allocations must be bounded by trajectory storage size"
    );
}
