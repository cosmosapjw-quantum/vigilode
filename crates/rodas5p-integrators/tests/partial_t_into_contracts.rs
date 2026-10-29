//! Contracts of `OdeProblem::eval_partial_t_into` and its use by the RODAS5P
//! fast driver (BASS/rei port step L1): it equals the allocating
//! `eval_partial_t` bit for bit with the same counters, a fast run of a
//! non-autonomous problem is unchanged from the tree before the change
//! (golden digests recorded there), and a non-autonomous fast step with an
//! analytic `f_t` no longer allocates per step.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

use rodas5p_core::{CoreError, DenseMatrix, WorkCounters};
use rodas5p_integrators::{OdeProblem, integrate_rodas5p_fast_observed};

#[path = "partial_t_into_support/mod.rs"]
mod support;

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

/// Digests of `support::run_digest` produced by the identical support
/// module on the tree before this change (commit 5fd6e459, rustc 1.94.1,
/// release profile): (analytic f_t, rtol, digest).
const GOLDEN: [(bool, f64, u64); 4] = [
    (true, 1.0e-6, 0xdac4_7754_cb51_dfd8),
    (true, 1.0e-9, 0x6bd0_f25d_8215_86f3),
    (false, 1.0e-6, 0xa961_d1ff_7c55_0b41),
    (false, 1.0e-9, 0xd884_4ee6_2c32_3809),
];

#[test]
fn fast_runs_match_the_pre_change_golden_digests() {
    for (analytic, rtol, digest) in GOLDEN {
        assert_eq!(
            support::run_digest(analytic, rtol),
            digest,
            "analytic={analytic} rtol={rtol:e}"
        );
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// A 2-component problem whose `f_t` accumulates into `out` (so it relies
/// on receiving zeros) and is NaN at `t < 0`.
fn accumulating(autonomous: bool) -> OdeProblem {
    OdeProblem::new(
        "partial-t-accumulating",
        2,
        Arc::new(|t, y, out| {
            out[0] = t * y[1];
            out[1] = -y[0];
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| {
            DenseMatrix::new(2, 2, vec![0.0, 1.0, -1.0, 0.0])
        })),
        None,
        Some(Arc::new(|t, y, out| {
            out[0] += y[1] * t.sqrt() / t.sqrt();
            out[1] += 0.0;
            Ok(())
        })),
        autonomous,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn into_form_equals_allocating_form_bitwise_with_equal_counters() {
    let y = [0.75, -1.25];
    let cases = [
        ("autonomous", accumulating(true)),
        ("callback", accumulating(false)),
        ("finite-difference", support::prothero_robinson(false)),
        ("analytic", support::prothero_robinson(true)),
    ];
    for (name, problem) in cases {
        let n = problem.dimension;
        let state: Vec<f64> = y.iter().copied().cycle().take(n).collect();
        for t in [0.5, 1.75] {
            let mut a = WorkCounters::default();
            let mut b = WorkCounters::default();
            let reference = problem.eval_partial_t(t, &state, &mut a).unwrap();
            // Garbage in `out` must not leak into the result.
            let mut out: Vec<f64> = (0..n).map(|i| 1.0e300 * (i as f64 + 1.0)).collect();
            problem
                .eval_partial_t_into(t, &state, &mut out, &mut b)
                .unwrap();
            assert_eq!(bits(&out), bits(&reference), "{name} t={t}");
            assert_eq!(a, b, "{name} t={t}");
        }
    }
}

#[test]
fn into_form_refuses_shapes_and_non_finite_output() {
    let problem = accumulating(false);
    let mut counters = WorkCounters::default();
    let mut short = [0.0; 1];
    assert!(matches!(
        problem.eval_partial_t_into(1.0, &[0.0, 1.0], &mut short, &mut counters),
        Err(CoreError::Dimension(_))
    ));
    let mut out = [0.0; 2];
    assert!(matches!(
        problem.eval_partial_t_into(1.0, &[0.0], &mut out, &mut counters),
        Err(CoreError::Dimension(_))
    ));
    // sqrt(t)/sqrt(t) is NaN at t = -1 and the allocating form agrees.
    assert!(matches!(
        problem.eval_partial_t_into(-1.0, &[0.0, 1.0], &mut out, &mut counters),
        Err(CoreError::NonFinite(_))
    ));
    assert!(matches!(
        problem.eval_partial_t(-1.0, &[0.0, 1.0], &mut counters),
        Err(CoreError::NonFinite(_))
    ));
}

#[test]
fn a_non_autonomous_fast_step_with_analytic_ft_allocates_nothing() {
    let problem = support::prothero_robinson(true);
    let (y0, output) = (support::y0(), support::schedule());
    integrate_rodas5p_fast_observed(&problem, (0.0, 2.0), &y0, &support::config(1.0e-6), &output)
        .unwrap();
    let mut runs = Vec::new();
    for rtol in [1.0e-6, 1.0e-10] {
        let (fast, allocations) = allocations_during(|| {
            integrate_rodas5p_fast_observed(
                &problem,
                (0.0, 2.0),
                &y0,
                &support::config(rtol),
                &output,
            )
            .unwrap()
        });
        assert!(fast.observed.success);
        runs.push((fast.attempts, fast.observed.counters.ft_calls, allocations));
    }
    assert!(runs[1].0 >= 3 * runs[0].0, "{runs:?}");
    assert!(runs[1].1 > runs[0].1, "{runs:?}");
    assert_eq!(
        runs[0].2, runs[1].2,
        "allocations depend on the step count: {runs:?}"
    );
}
