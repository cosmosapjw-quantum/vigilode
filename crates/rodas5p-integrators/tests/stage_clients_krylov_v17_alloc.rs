//! The client arms of loop node V17 allocate nothing per accepted step.
//! This binary holds exactly one test: the allocation counter is
//! process-global, so a second test running beside it would be counted.
//!
//! The method is that of `stage_clients_krylov_alloc.rs` (node V06): the
//! accept control reads the counter after every accepted step into a buffer
//! reserved before the run; the difference of two consecutive readings is
//! everything allocated between two accepted steps (controller, rejected
//! attempts, linearization with the weight floor of the step-tied
//! tolerance, shift, eight stage solves, the residual audit when it is on,
//! acceptance). It must be zero from the step after the warm-up to the last
//! step before `tf`, and the total of a run must not depend on its number
//! of steps. The runs have no failed linear solve (asserted): a failure
//! allocates its error message.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "stage_clients_krylov_v17_problems.rs"]
mod problems;

use problems::{Arm, ArmOptions, Family, adaptive, build_client, declared};
use rodas5p_integrators::stage_clients::krylov::KrylovStats;
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, OutputSchedule, Rodas5pFastResult,
    integrate_rodas5p_fast_client_observed_with_atol_and_control,
};

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
/// together with the driver's set-up and the first linearization.
const WARM_UP_STEPS: usize = 1;
/// Readings reserved per run; a run with more accepted steps fails.
const READINGS: usize = 1 << 16;

struct Measured {
    result: Rodas5pFastResult,
    stats: KrylovStats,
    run_total: u64,
    to_first_step: u64,
    between_steps: Vec<u64>,
}

fn measure(family: &Family, rtol: f64, options: &ArmOptions, readings: &mut Vec<u64>) -> Measured {
    let n = family.problem.dimension;
    let config = adaptive(family, rtol);
    let atol = vec![config.atol; n];
    let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
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
    let (client, stats, _audit) = build_client(family, &config, options).unwrap();
    let (result, partial, client) = integrate_rodas5p_fast_client_observed_with_atol_and_control(
        &family.problem,
        client,
        family.span,
        &family.y0,
        &config,
        &output,
        &atol,
        &mut control,
    )
    .unwrap();
    let run_total = allocations() - before;
    drop(client);
    assert!(
        !overflow,
        "{}: more than {READINGS} accepted steps",
        family.id
    );
    assert!(
        result.observed.success && partial.is_none(),
        "{}",
        family.id
    );
    assert_eq!(readings.len(), result.accepted_steps);
    Measured {
        run_total,
        to_first_step: readings[0] - before,
        between_steps: readings.windows(2).map(|pair| pair[1] - pair[0]).collect(),
        stats: stats.snapshot(),
        result,
    }
}

#[test]
fn v17_clients_allocate_nothing_per_accepted_step() {
    let mut readings: Vec<u64> = Vec::with_capacity(READINGS);
    let with = |arm, jacobi, audit, restart| ArmOptions {
        jacobi,
        audit,
        restart,
        ..ArmOptions::new(arm)
    };
    let configurations = [
        ("B", with(Arm::PerStage, false, false, None)),
        ("B audited", with(Arm::PerStage, false, true, None)),
        ("B restart 10", with(Arm::PerStage, false, false, Some(10))),
        ("C", with(Arm::StepTied, false, false, None)),
        ("C audited", with(Arm::StepTied, false, true, None)),
        ("C jacobi", with(Arm::StepTied, true, false, None)),
    ];
    let mut report = Vec::new();
    let mut policy_bound_solves = 0;
    for id in ["dev_fhn3d", "dev_burgers2d", "dev_bruss1d"] {
        let family = declared(id).unwrap();
        for (name, options) in &configurations {
            // Warm the coefficient cache and any lazy statics.
            measure(&family, 1.0e-4, options, &mut readings);
            let mut totals = Vec::new();
            for rtol in [1.0e-4, 1.0e-6] {
                let measured = measure(&family, rtol, options, &mut readings);
                let between = &measured.between_steps;
                assert!(
                    between.len() > WARM_UP_STEPS + 1,
                    "{id} {name}: too few steps"
                );
                let steady = &between[WARM_UP_STEPS - 1..between.len() - 1];
                let worst = steady.iter().copied().max().unwrap();
                let allocating = steady.iter().filter(|d| **d != 0).count();
                assert_eq!(
                    measured.result.observed.counters.linear_solve_failures, 0,
                    "{id} {name}: the run has failed linear solves"
                );
                assert_eq!(
                    worst,
                    0,
                    "{id} {name} at rtol {rtol:e}: {allocating} of {} steady steps allocate \
                     (worst {worst})",
                    steady.len()
                );
                if options.arm == Arm::StepTied {
                    policy_bound_solves += measured.stats.policy_bound_solves;
                } else {
                    assert_eq!(measured.stats.policy_bound_solves, 0, "{id} {name}");
                }
                totals.push((measured.result.accepted_steps, measured.run_total));
                report.push(serde_json::json!({
                    "family": id,
                    "n": family.problem.dimension,
                    "client": name,
                    "rtol": rtol,
                    "attempts": measured.result.attempts,
                    "accepted_steps": measured.result.accepted_steps,
                    "rejected_steps": measured.result.rejected_steps,
                    "solves": measured.stats.solves,
                    "policy_bound_solves": measured.stats.policy_bound_solves,
                    "warm_up_steps": WARM_UP_STEPS,
                    "steady_steps_measured": steady.len(),
                    "steady_allocations_total": steady.iter().sum::<u64>(),
                    "steady_allocations_max_per_step": worst,
                    "allocations_to_first_accepted_step": measured.to_first_step,
                    "allocations_in_last_step_to_tf": between.last().copied(),
                    "run_total_allocations": measured.run_total,
                }));
            }
            // The run total does not depend on the number of steps.
            assert!(
                totals[1].0 > totals[0].0,
                "{id} {name}: the step counts do not differ: {totals:?}"
            );
            assert_eq!(
                totals[0].1, totals[1].1,
                "{id} {name}: the run total depends on the step count: {totals:?}"
            );
        }
    }
    // The step-tied bound was the threshold in the measured runs.
    assert!(policy_bound_solves > 0);
    for row in &report {
        println!("STAGE_CLIENTS_KRYLOV_V17_ALLOC_JSON {row}");
    }
}
