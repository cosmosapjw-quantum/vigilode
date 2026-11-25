//! The dense stage client allocates nothing per accepted step (loop node
//! V15). This binary holds exactly one test: the allocation counter is
//! process-global, so a second test running beside it would be counted.
//!
//! Two measurements per case, both through the client driver with the
//! output schedule `[t0, tf]` (the only stored output after the start is
//! the last step's):
//!
//! - per step: the accept control reads the counter after every accepted
//!   step into a buffer reserved before the run; the difference of two
//!   consecutive readings is everything allocated between two accepted
//!   steps (controller, rejected attempts, linearization, factorization,
//!   eight stage solves, acceptance). It must be zero from the step after
//!   the warm-up to the last step before `tf`.
//! - per run: the total of a run does not depend on the number of steps.
//!
//! The in-place mode and the timing wrapper around it must pass both. The
//! faer mode allocates inside every factorization, as the protected
//! driver's faer path does; its allocations per step are reported, not
//! asserted to be zero.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_integrators::stage_clients::{DenseLuMode, DenseLuStageClient, TimedStageClient};
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, OdeProblem, OutputSchedule, Rodas5pFastLu, Rodas5pFastResult,
    StageLinearSystem, integrate_rodas5p_fast_client_observed_with_atol_and_control,
};

#[path = "b1_problems/mod.rs"]
mod b1_problems;
use b1_problems::{Case, brusselator_problem, config, hires_case, robertson_case};

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

fn brusselator_case(cells: usize, id: &'static str) -> Case {
    let (problem, y0) = brusselator_problem(cells).unwrap();
    Case {
        id,
        problem,
        y0,
        span: (0.0, 10.0),
        atol_scale: 1.0,
    }
}

/// The Brusselator with a mean-field coupling of
/// `stage_clients_dense_identity.rs`: a half-full Jacobian, for which the
/// protected policy picks faer.
fn meanfield_brusselator_problem(cells: usize, kappa: f64) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let mean = kappa / cells as f64;
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (mut ubar, mut vbar) = (0.0, 0.0);
        for i in 0..cells {
            ubar += y[2 * i];
            vbar += y[2 * i + 1];
        }
        ubar /= cells as f64;
        vbar /= cells as f64;
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl) = if i == 0 {
                (1.0, 3.0)
            } else {
                (y[2 * i - 2], y[2 * i - 1])
            };
            let (ur, vr) = if i + 1 == cells {
                (1.0, 3.0)
            } else {
                (y[2 * i + 2], y[2 * i + 3])
            };
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur) + kappa * (ubar - u);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr) + kappa * (vbar - v);
        }
        Ok(())
    });
    let n = 2 * cells;
    let fill = move |y: &[f64], j: &mut DenseMatrix| {
        for i in 0..cells {
            for k in 0..cells {
                j[(2 * i, 2 * k)] = mean;
                j[(2 * i + 1, 2 * k + 1)] = mean;
            }
        }
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            j[(a, a)] = mean + 2.0 * u * v - 4.0 - 2.0 * c - kappa;
            j[(a, b)] = u * u;
            j[(b, a)] = 3.0 - 2.0 * u * v;
            j[(b, b)] = mean - u * u - 2.0 * c - kappa;
            if i > 0 {
                j[(a, a - 2)] = mean + c;
                j[(b, b - 2)] = mean + c;
            }
            if i + 1 < cells {
                j[(a, a + 2)] = mean + c;
                j[(b, b + 2)] = mean + c;
            }
        }
    };
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        fill(y, &mut j);
        Ok(j)
    });
    let jacobian_into = Arc::new(move |_t: f64, y: &[f64], j: &mut DenseMatrix| {
        fill(y, j);
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect();
    Ok((
        OdeProblem::new(
            format!("brusselator-1d-{cells}-meanfield"),
            n,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            None,
            None,
        )?
        .with_jacobian_into(jacobian_into),
        y0,
    ))
}

/// What one measured run allocated.
struct Measured {
    result: Rodas5pFastResult,
    /// Allocations of the whole run, the client's construction included.
    run_total: u64,
    /// From the start of the run to the first accepted step.
    to_first_step: u64,
    /// Between accepted step `k` and `k + 1` (index `k`); the last entry
    /// ends at the step that reaches `tf` and stores its output.
    between_steps: Vec<u64>,
}

#[derive(Clone, Copy)]
enum Client {
    Dense(DenseLuMode),
    Timed(DenseLuMode),
}

/// One client run of `case` at `rtol` with the counter read after every
/// accepted step. `readings` is the reserved buffer.
fn measure(case: &Case, rtol: f64, client: Client, readings: &mut Vec<u64>) -> Measured {
    let n = case.problem.dimension;
    let adaptive = config(case, rtol);
    let atol = vec![adaptive.atol; n];
    let output = OutputSchedule::new(vec![case.span.0, case.span.1]).unwrap();
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
    let boxed: Box<dyn StageLinearSystem> = match client {
        Client::Dense(mode) => Box::new(DenseLuStageClient::new(n, mode).unwrap()),
        Client::Timed(mode) => Box::new(TimedStageClient::new(
            DenseLuStageClient::new(n, mode).unwrap(),
        )),
    };
    let (result, partial, boxed) = integrate_rodas5p_fast_client_observed_with_atol_and_control(
        &case.problem,
        boxed,
        case.span,
        &case.y0,
        &adaptive,
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
        case.id
    );
    assert!(result.observed.success && partial.is_none(), "{}", case.id);
    assert_eq!(result.lu, Rodas5pFastLu::Client);
    assert_eq!(readings.len(), result.accepted_steps);
    Measured {
        run_total,
        to_first_step: readings[0] - before,
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

#[test]
fn dense_client_allocates_nothing_per_accepted_step() {
    let mut readings: Vec<u64> = Vec::with_capacity(READINGS);
    let mut report = Vec::new();

    let in_place_cases = [
        robertson_case(),
        hires_case(),
        brusselator_case(32, "brusselator-1d-32 (n = 64)"),
        brusselator_case(100, "brusselator-1d-100 (n = 200)"),
    ];
    for case in &in_place_cases {
        for client in [
            Client::Dense(DenseLuMode::InPlace),
            Client::Timed(DenseLuMode::InPlace),
        ] {
            let name = match client {
                Client::Dense(_) => "DenseLuStageClient(InPlace)",
                Client::Timed(_) => "TimedStageClient(DenseLuStageClient(InPlace))",
            };
            // Warm the coefficient cache and any lazy statics.
            measure(case, 1.0e-4, client, &mut readings);
            let mut totals = Vec::new();
            for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
                let measured = measure(case, rtol, client, &mut readings);
                let steady = steady(&measured);
                let worst = steady.iter().copied().max().unwrap();
                let steps_with_allocations = steady.iter().filter(|d| **d != 0).count();
                assert_eq!(
                    worst,
                    0,
                    "{} with {name} at rtol {rtol:e}: {steps_with_allocations} of {} steady \
                     steps allocate (worst {worst})",
                    case.id,
                    steady.len()
                );
                totals.push((measured.result.accepted_steps, measured.run_total));
                report.push(serde_json::json!({
                    "case": case.id,
                    "n": case.problem.dimension,
                    "client": name,
                    "rtol": rtol,
                    "attempts": measured.result.attempts,
                    "accepted_steps": measured.result.accepted_steps,
                    "rejected_steps": measured.result.rejected_steps,
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
                case.id
            );
            assert!(
                totals.iter().all(|(_, total)| *total == totals[0].1),
                "{} with {name}: the run total depends on the step count: {totals:?}",
                case.id
            );
        }
    }

    // faer: reported. Every factorization allocates inside
    // `LuFactorization::new`, in the protected driver as well.
    let (problem, y0) = meanfield_brusselator_problem(100, 0.1).unwrap();
    let meanfield = Case {
        id: "brusselator-1d-100-meanfield (n = 200, dense Jacobian)",
        problem,
        y0,
        span: (0.0, 10.0),
        atol_scale: 1.0,
    };
    measure(
        &meanfield,
        1.0e-4,
        Client::Dense(DenseLuMode::Faer),
        &mut readings,
    );
    for rtol in [1.0e-4, 1.0e-6] {
        let measured = measure(
            &meanfield,
            rtol,
            Client::Dense(DenseLuMode::Faer),
            &mut readings,
        );
        let steady = steady(&measured);
        report.push(serde_json::json!({
            "case": meanfield.id,
            "n": meanfield.problem.dimension,
            "client": "DenseLuStageClient(Faer)",
            "rtol": rtol,
            "asserted_zero": false,
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

    for row in &report {
        println!("STAGE_CLIENTS_ALLOC_JSON {row}");
    }
}
