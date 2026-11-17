//! The speculative Jacobian client allocates nothing per accepted step
//! (loop node V07, stage a). This binary holds exactly one test: the
//! allocation counter is process-global, so a second test running beside it
//! would be counted.
//!
//! As in `stage_clients_alloc.rs`: the accept control reads the counter
//! after every accepted step into a buffer reserved before the run; the
//! difference of two consecutive readings is everything allocated between
//! two accepted steps (controller, rejected attempts, the recording of the
//! stage arguments, the linearization at the speculative point, the
//! factorization, eight stage solves, the published statistics,
//! acceptance). It must be zero from the step after the warm-up to the last
//! step before `tf`, in every mode, around the in-place dense client. The
//! total of a run must not depend on the number of steps.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_integrators::stage_clients::speculative::{
    SpeculationMode, SpeculationStats, SpeculativeJacobianClient,
};
use rodas5p_integrators::stage_clients::{DenseLuMode, DenseLuStageClient};
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, Rodas5pFastLu, Rodas5pFastResult, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol_and_control,
};

#[path = "stage_clients_speculative_problems.rs"]
mod problems;
use problems::{Model, RTOLS, brusselator, hires, pr_nonlinear, robertson};

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
/// together with the driver's set-up.
const WARM_UP_STEPS: usize = 1;
/// Readings reserved per run; a run with more accepted steps fails.
const READINGS: usize = 1 << 16;

/// What one measured run allocated.
struct Measured {
    result: Rodas5pFastResult,
    stats: SpeculationStats,
    /// Allocations of the whole run, the client's construction included
    /// (the problem and its recorder are built before).
    run_total: u64,
    /// From the start of the run to the first accepted step.
    to_first_step: u64,
    /// Between accepted step `k` and `k + 1` (index `k`); the last entry
    /// ends at the step that reaches `tf` and stores its output.
    between_steps: Vec<u64>,
}

fn measure(model: &Model, mode: SpeculationMode, rtol: f64, readings: &mut Vec<u64>) -> Measured {
    let n = model.n;
    let config = model.adaptive_config(rtol);
    let atol = vec![config.atol; n];
    let output = model.end_points();
    let (problem, recorder) = model.recorded_problem();
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
    let client = SpeculativeJacobianClient::new(
        DenseLuStageClient::new(n, DenseLuMode::InPlace).unwrap(),
        mode,
        Arc::clone(&recorder),
    )
    .unwrap();
    let handle = client.handle();
    let boxed: Box<dyn StageLinearSystem> = Box::new(client);
    let (result, partial, boxed) = integrate_rodas5p_fast_client_observed_with_atol_and_control(
        &problem,
        boxed,
        model.t_span,
        &model.y0,
        &config,
        &output,
        &atol,
        &mut control,
    )
    .unwrap();
    let run_total = allocations() - before;
    drop(boxed);
    assert!(
        !overflow,
        "{}: more than {READINGS} accepted steps",
        model.id
    );
    assert!(result.observed.success && partial.is_none(), "{}", model.id);
    assert_eq!(result.lu, Rodas5pFastLu::Client);
    assert_eq!(readings.len(), result.accepted_steps);
    Measured {
        run_total,
        to_first_step: readings[0] - before,
        between_steps: readings.windows(2).map(|pair| pair[1] - pair[0]).collect(),
        stats: handle.snapshot(),
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

#[test]
fn speculative_client_allocates_nothing_per_accepted_step() {
    let mut readings: Vec<u64> = Vec::with_capacity(READINGS);
    let mut report = Vec::new();
    let models = [
        robertson(),
        hires(),
        // Non-autonomous: the driver's time derivative is part of a step.
        pr_nonlinear(10.0),
        brusselator(32),
        brusselator(100),
    ];
    for model in &models {
        assert_eq!(model.lu_mode(), DenseLuMode::InPlace, "{}", model.id);
        for mode in [
            SpeculationMode::Exact,
            SpeculationMode::G8,
            SpeculationMode::G7,
            SpeculationMode::LaggedOneStep,
        ] {
            // Warm the coefficient cache and any lazy statics.
            measure(model, mode, 1.0e-4, &mut readings);
            let mut totals = Vec::new();
            for rtol in RTOLS {
                let measured = measure(model, mode, rtol, &mut readings);
                let steady = steady(&measured);
                let worst = steady.iter().copied().max().unwrap();
                let steps_with_allocations = steady.iter().filter(|d| **d != 0).count();
                assert_eq!(
                    worst,
                    0,
                    "{} in mode {} at rtol {rtol:e}: {steps_with_allocations} of {} steady \
                     steps allocate (worst {worst})",
                    model.id,
                    mode.label(),
                    steady.len()
                );
                // The speculative modes did speculate.
                let accepted = measured.result.accepted_steps as u64;
                if mode == SpeculationMode::Exact {
                    assert_eq!(measured.stats.exact_by_mode, accepted);
                } else {
                    assert_eq!(measured.stats.exact_first, 1);
                    assert_eq!(measured.stats.exact_no_point, 0);
                    assert_eq!(
                        measured.stats.speculative + measured.stats.exact_after_failure,
                        measured.stats.linearizations - 1
                    );
                    assert!(measured.stats.speculative > 0);
                }
                totals.push((measured.result.accepted_steps, measured.run_total));
                report.push(serde_json::json!({
                    "model": model.id,
                    "n": model.n,
                    "mode": mode.label(),
                    "inner_client": "DenseLuStageClient(InPlace)",
                    "rtol": rtol,
                    "attempts": measured.result.attempts,
                    "accepted_steps": measured.result.accepted_steps,
                    "rejected_steps": measured.result.rejected_steps,
                    "speculative_linearizations": measured.stats.speculative,
                    "warm_up_steps": WARM_UP_STEPS,
                    "steady_steps_measured": steady.len(),
                    "steady_allocations_total": steady.iter().sum::<u64>(),
                    "steady_allocations_max_per_step": worst,
                    "allocations_to_first_accepted_step": measured.to_first_step,
                    "allocations_in_last_step_to_tf": measured.between_steps.last().copied(),
                    "run_total_allocations": measured.run_total,
                }));
            }
            // The run total does not depend on the number of steps.
            assert!(
                totals[2].0 > totals[0].0,
                "{}: the step counts do not differ: {totals:?}",
                model.id
            );
            assert!(
                totals.iter().all(|(_, total)| *total == totals[0].1),
                "{} in mode {}: the run total depends on the step count: {totals:?}",
                model.id,
                mode.label()
            );
        }
    }
    for row in &report {
        println!("SPECJ_ALLOC_JSON {row}");
    }
}
