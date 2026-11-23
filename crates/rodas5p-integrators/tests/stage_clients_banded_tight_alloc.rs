//! The opt-in banded stage client allocates nothing per accepted step (loop L1 node V21, stage
//! b). This binary holds exactly one test: the allocation counter is process-global, so a
//! second test running beside it would be counted (the pattern of `stage_clients_alloc.rs`).
//!
//! The accept control reads the counter after every accepted step into a buffer reserved
//! before the run; the difference of two consecutive readings is everything allocated between
//! two accepted steps (controller, rejected attempts, linearisation, factorisation, eight
//! stage solves, acceptance). It must be zero from the step after the warm-up to the last step
//! before the end.

use std::alloc::{GlobalAlloc, Layout, System};
use std::f64::consts::PI;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::CoreResult;
use rodas5p_integrators::stage_clients::BandedTightStageLu;
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, AdaptiveStepConfig, BandedJacobian, OdeProblem,
    OutputSchedule, integrate_rodas5p_fast_client_observed_with_atol_and_control,
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

type Callback3 = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type Callback4 = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// The one-dimensional Brusselator of loop node V04 (band lower = upper = 2).
fn brusselator(cells: usize) -> (OdeProblem, BandedJacobian, Vec<f64>) {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let rhs: Callback3 = Arc::new(move |_t, y, out| {
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
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
        }
        Ok(())
    });
    let fill: Callback3 = Arc::new(move |_t, y, band| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (da, db) = (2 * i * 5 + 2, (2 * i + 1) * 5 + 2);
            band[da] = 2.0 * u * v - 4.0 - 2.0 * c;
            band[da + 1] = u * u;
            band[db - 1] = 3.0 - 2.0 * u * v;
            band[db] = -u * u - 2.0 * c;
            if i > 0 {
                band[da - 2] = c;
                band[db - 2] = c;
            }
            if i + 1 < cells {
                band[da + 2] = c;
                band[db + 2] = c;
            }
        }
        Ok(())
    });
    // The banded client never calls the JVP; the problem constructor asks for one.
    let jvp: Callback4 = Arc::new(|_t, _y, _w, out| {
        out.fill(0.0);
        Ok(())
    });
    let problem = OdeProblem::new(
        "brusselator-1d",
        2 * cells,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * PI * x).sin(), 3.0]
        })
        .collect();
    (
        problem,
        BandedJacobian {
            lower: 2,
            upper: 2,
            fill,
        },
        y0,
    )
}

/// Accepted steps whose allocations are not asserted: the first step runs together with the
/// driver's set-up.
const WARM_UP_STEPS: usize = 1;
/// Readings reserved for the run; a run with more accepted steps fails.
const READINGS: usize = 1 << 16;

#[test]
fn banded_tight_client_allocates_nothing_per_accepted_step() {
    let (problem, band, y0) = brusselator(500);
    let n = problem.dimension;
    let rtol = 1.0e-6;
    let adaptive = AdaptiveStepConfig {
        atol: rtol,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: 10.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    };
    let atol = vec![rtol; n];
    let output = OutputSchedule::new(vec![0.0, 10.0]).unwrap();
    let mut readings: Vec<u64> = Vec::with_capacity(READINGS);
    let mut overflow = false;
    let mut control = |_view: &AcceptedStepView<'_>| {
        if readings.len() < READINGS {
            // Within the reserved capacity: no allocation.
            readings.push(ALLOCATIONS.load(Ordering::Relaxed));
        } else {
            overflow = true;
        }
        AcceptControl::Continue
    };
    let client = Box::new(BandedTightStageLu::new(n, &band).unwrap());
    assert!(client.uses_fixed_offsets());
    let (result, partial, client) = integrate_rodas5p_fast_client_observed_with_atol_and_control(
        &problem,
        client,
        (0.0, 10.0),
        &y0,
        &adaptive,
        &output,
        &atol,
        &mut control,
    )
    .unwrap();
    drop(client);
    assert!(!overflow, "more than {READINGS} accepted steps");
    assert!(result.observed.success && partial.is_none());
    assert_eq!(readings.len(), result.accepted_steps);
    let between: Vec<u64> = readings.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!(
        between.len() > WARM_UP_STEPS + 1,
        "too few steps to measure"
    );
    let steady = &between[WARM_UP_STEPS - 1..between.len() - 1];
    let worst = steady.iter().copied().max().unwrap();
    println!(
        "V21B_ALLOC n={n} accepted={} rejected={} steady_intervals={} max_allocations_per_step={worst}",
        result.accepted_steps,
        result.rejected_steps,
        steady.len()
    );
    assert_eq!(worst, 0, "allocations between accepted steps: {steady:?}");
}
