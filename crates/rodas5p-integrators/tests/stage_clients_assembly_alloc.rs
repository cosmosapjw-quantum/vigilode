//! The sequential modes of the finite-difference assembly client allocate
//! nothing per accepted step (loop node V08). This binary holds exactly one
//! test: the allocation counter is process-global, so a second test running
//! beside it would be counted.
//!
//! As in `stage_clients_alloc.rs`: the accept control of the client driver
//! reads the counter after every accepted step into a buffer reserved
//! before the run; the difference of two consecutive readings is
//! everything allocated between two accepted steps (controller, rejected
//! attempts, the assembly with its right-hand sides, the copy into the
//! back end, factorization, eight stage solves, acceptance). It must be
//! zero from the step after the warm-up to the last step before `tf`, and
//! the total of a run must not depend on the number of steps. The first
//! linearization, with its check columns, lies before the first accepted
//! step.
//!
//! The threaded modes hand their right-hand sides to rayon, which may
//! allocate; their allocations per step are reported, not asserted.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_integrators::stage_clients::assembly::{
    AssemblyOptions, CheckColumns, ColumnRoute, LinearBackend, SparsityPattern,
};
use rodas5p_integrators::stage_clients::{DenseLuMode, TimedStageClient};
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, OutputSchedule, Rodas5pFastLu, Rodas5pFastResult,
    StageLinearSystem, integrate_rodas5p_fast_client_observed_with_atol_and_control,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{Arm, ClientSpec, FdProblem, RunConfig, brusselator, nlkpp_band, nlkpp_dense};

struct Counting;

/// Allocation requests of the whole process (alloc, alloc_zeroed, realloc).
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

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

fn allocations() -> u64 {
    ALLOCATIONS.load(Ordering::Relaxed)
}

/// Accepted steps whose allocations are not asserted: the first step runs
/// together with the driver's set-up and the first-use check.
const WARM_UP_STEPS: usize = 1;
/// Readings reserved per run; a run with more accepted steps fails.
const READINGS: usize = 1 << 16;

struct Measured {
    result: Rodas5pFastResult,
    /// Allocations of the whole run, the client's construction included.
    run_total: u64,
    /// Between accepted step `k` and `k + 1` (index `k`).
    between_steps: Vec<u64>,
}

fn measure(
    fd: &FdProblem,
    spec: &ClientSpec,
    timed: bool,
    rtol: f64,
    readings: &mut Vec<u64>,
) -> Measured {
    let config = RunConfig {
        rtol,
        atol: 1.0e-2 * rtol,
        max_attempts: 1_000_000,
    };
    let adaptive = config.adaptive(fd.span);
    let atol = vec![config.atol; fd.n];
    let output = OutputSchedule::new(vec![fd.span.0, fd.span.1]).unwrap();
    readings.clear();
    assert!(readings.capacity() >= READINGS);
    let mut overflow = false;
    let mut control = |_view: &AcceptedStepView<'_>| {
        if readings.len() < READINGS {
            // Within the reserved capacity: no allocation.
            readings.push(allocations());
        } else {
            overflow = true;
        }
        AcceptControl::Continue
    };
    let before = allocations();
    let client = spec.build(fd.n);
    let boxed: Box<dyn StageLinearSystem> = if timed {
        Box::new(TimedStageClient::new(client))
    } else {
        Box::new(client)
    };
    let (result, partial, boxed) = integrate_rodas5p_fast_client_observed_with_atol_and_control(
        &fd.problem,
        boxed,
        fd.span,
        &fd.y0,
        &adaptive,
        &output,
        &atol,
        &mut control,
    )
    .unwrap();
    let run_total = allocations() - before;
    drop(boxed);
    assert!(!overflow, "{}: more than {READINGS} accepted steps", fd.id);
    assert!(result.observed.success && partial.is_none(), "{}", fd.id);
    assert_eq!(result.lu, Rodas5pFastLu::Client);
    assert_eq!(readings.len(), result.accepted_steps);
    Measured {
        run_total,
        between_steps: readings.windows(2).map(|pair| pair[1] - pair[0]).collect(),
        result,
    }
}

/// The steady-state part of `between_steps`: after the warm-up, before the
/// step that reaches `tf`.
fn steady(measured: &Measured) -> &[u64] {
    let between = &measured.between_steps;
    assert!(
        between.len() > WARM_UP_STEPS + 1,
        "too few steps to measure: {}",
        between.len()
    );
    &between[WARM_UP_STEPS - 1..between.len() - 1]
}

fn spec(route: ColumnRoute, arm: Arm, backend: LinearBackend) -> ClientSpec {
    ClientSpec {
        route,
        arm,
        backend,
        first_use_check: CheckColumns::Sample(AssemblyOptions::DEFAULT_CHECK_COLUMNS),
        fingerprints: false,
    }
}

fn small_brusselator() -> FdProblem {
    brusselator(16, false)
}

fn small_band() -> FdProblem {
    nlkpp_band(96, 4, false)
}

fn band_route(fd: &FdProblem) -> (ColumnRoute, usize, usize) {
    let (lower, upper) = fd.band.unwrap();
    (
        ColumnRoute::Coloured(SparsityPattern::banded(fd.n, lower, upper).unwrap()),
        lower,
        upper,
    )
}

#[test]
fn sequential_assembly_allocates_nothing_per_accepted_step() {
    let mut readings: Vec<u64> = Vec::with_capacity(READINGS);
    let mut report = Vec::new();
    let in_place = LinearBackend::DenseLu(DenseLuMode::InPlace);

    // (problem, client, name): every sequential mode on every back end
    // that does not allocate itself.
    let mut cases: Vec<(FdProblem, ClientSpec, &str)> = vec![
        (
            nlkpp_dense(24, false),
            spec(ColumnRoute::Dense, Arm::Seq, in_place),
            "dense route, DenseLu(InPlace)",
        ),
        (
            small_brusselator(),
            spec(ColumnRoute::Dense, Arm::Seq, in_place),
            "dense route, DenseLu(InPlace)",
        ),
        (
            small_brusselator(),
            spec(band_route(&small_brusselator()).0, Arm::Seq, in_place),
            "coloured route, DenseLu(InPlace)",
        ),
    ];
    for make in [small_brusselator as fn() -> FdProblem, small_band] {
        let fd = make();
        let (route, lower, upper) = band_route(&fd);
        cases.push((
            make(),
            spec(
                route.clone(),
                Arm::Seq,
                LinearBackend::BandedProtected { lower, upper },
            ),
            "coloured route, BandedStageLu (protected kernel)",
        ));
        cases.push((
            fd,
            spec(route, Arm::Seq, LinearBackend::BandedTight { lower, upper }),
            "coloured route, BandedTightStageLu",
        ));
    }

    for (fd, client, name) in &cases {
        for timed in [false, true] {
            let name = if timed {
                format!("TimedStageClient({name})")
            } else {
                (*name).to_string()
            };
            // Warm the coefficient cache and any lazy statics.
            measure(fd, client, timed, 1.0e-4, &mut readings);
            let mut totals = Vec::new();
            for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
                let measured = measure(fd, client, timed, rtol, &mut readings);
                let steady = steady(&measured);
                let worst = steady.iter().copied().max().unwrap();
                let steps_with_allocations = steady.iter().filter(|d| **d != 0).count();
                assert_eq!(
                    worst,
                    0,
                    "{} with {name} at rtol {rtol:e}: {steps_with_allocations} of {} steady \
                     steps allocate (worst {worst})",
                    fd.id,
                    steady.len()
                );
                totals.push((measured.result.accepted_steps, measured.run_total));
                report.push(serde_json::json!({
                    "case": fd.id,
                    "n": fd.n,
                    "client": name,
                    "execution": "sequential",
                    "asserted_zero": true,
                    "rtol": rtol,
                    "attempts": measured.result.attempts,
                    "accepted_steps": measured.result.accepted_steps,
                    "rejected_steps": measured.result.rejected_steps,
                    "warm_up_steps": WARM_UP_STEPS,
                    "steady_steps_measured": steady.len(),
                    "steady_allocations_total": steady.iter().sum::<u64>(),
                    "steady_allocations_max_per_step": worst,
                    "run_total_allocations": measured.run_total,
                }));
            }
            // The run total does not depend on the number of steps.
            assert!(
                totals[2].0 > totals[0].0,
                "{}: the step counts do not differ: {totals:?}",
                fd.id
            );
            assert!(
                totals.iter().all(|(_, total)| *total == totals[0].1),
                "{} with {name}: the run total depends on the step count: {totals:?}",
                fd.id
            );
        }
    }

    // Threaded execution: reported.
    let fd = brusselator(16, false);
    let (route, lower, upper) = band_route(&fd);
    for (client, name) in [
        (
            spec(ColumnRoute::Dense, Arm::Threaded(2), in_place),
            "dense route, 2 workers, DenseLu(InPlace)",
        ),
        (
            spec(
                route,
                Arm::Threaded(2),
                LinearBackend::BandedTight { lower, upper },
            ),
            "coloured route, 2 workers, BandedTightStageLu",
        ),
    ] {
        measure(&fd, &client, false, 1.0e-4, &mut readings);
        for rtol in [1.0e-4, 1.0e-6] {
            let measured = measure(&fd, &client, false, rtol, &mut readings);
            let steady = steady(&measured);
            report.push(serde_json::json!({
                "case": fd.id,
                "n": fd.n,
                "client": name,
                "execution": "threaded",
                "asserted_zero": false,
                "rtol": rtol,
                "attempts": measured.result.attempts,
                "accepted_steps": measured.result.accepted_steps,
                "rejected_steps": measured.result.rejected_steps,
                "warm_up_steps": WARM_UP_STEPS,
                "steady_steps_measured": steady.len(),
                "steady_allocations_total": steady.iter().sum::<u64>(),
                "steady_allocations_min_per_step": steady.iter().copied().min(),
                "steady_allocations_max_per_step": steady.iter().copied().max(),
                "run_total_allocations": measured.run_total,
            }));
        }
    }

    for row in &report {
        println!("STAGE_CLIENTS_ASSEMBLY_ALLOC_JSON {row}");
    }
}
