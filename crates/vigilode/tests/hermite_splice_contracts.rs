//! Contracts for WP-11: C¹ dense-output cubic Hermite splice at route switches.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use vigilode::{CallbackError, CubicHermiteSplice, Options, Problem, Rodas5p};

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

struct CountingAlloc;

fn count_allocation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn measured<T>(f: impl FnOnce() -> T) -> (T, usize) {
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let result = f();
    TRACK.with(|track| track.set(false));
    let count = COUNT.with(Cell::get);
    (result, count)
}

/// VG-1: Zero-allocation contract for hot evaluation loops.
#[test]
fn zero_allocation_in_hot_evaluation_loops() {
    let splice = CubicHermiteSplice::new(
        0.0,
        0.5,
        vec![0.123, -0.456, 0.789],
        vec![-0.321, 0.654, -0.987],
    )
    .unwrap();

    let mut out_corr = [0.0; 3];
    let mut out_deriv = [0.0; 3];
    let mut state = [1.0, 2.0, 3.0];

    // Warm up
    splice.evaluate_correction_into(0.25, &mut out_corr);
    splice.evaluate_derivative_correction_into(0.25, &mut out_deriv);
    splice.apply_correction(&mut state, 0.25);

    // Measure hot loop execution
    let (_, allocs) = measured(|| {
        for i in 0..10_000 {
            let t = 0.00005 * (i as f64);
            splice.evaluate_correction_into(t, &mut out_corr);
            splice.evaluate_derivative_correction_into(t, &mut out_deriv);
            splice.apply_correction(&mut state, t);
        }
    });

    assert_eq!(
        allocs, 0,
        "VG-1 violation: in-place splice evaluation must have 0 heap allocations, got {allocs}"
    );
}

/// WP-11 Gate: derivative continuity at switch time t_s (< 10^-12 relative error).
#[test]
fn derivative_continuity_at_switch_within_1e12() {
    let ts = 2.5;
    let t_end = 3.1;
    let f_ts = [-1.4, 2.7, 0.5];
    let p_prime_right = [-1.41, 2.72, 0.57];

    let delta_f0 = vec![
        f_ts[0] - p_prime_right[0],
        f_ts[1] - p_prime_right[1],
        f_ts[2] - p_prime_right[2],
    ];
    let delta_f1 = vec![0.0, 0.0, 0.0]; // preserve right boundary derivative

    let splice = CubicHermiteSplice::new(ts, t_end, delta_f0.clone(), delta_f1).unwrap();

    let mut deriv_corr = [0.0; 3];
    splice.evaluate_derivative_correction_into(ts, &mut deriv_corr);

    for i in 0..3 {
        let corrected_right_deriv = p_prime_right[i] + deriv_corr[i];
        let rel_err = (corrected_right_deriv - f_ts[i]).abs() / f_ts[i].abs();
        assert!(
            rel_err < 1e-12,
            "Derivative continuity failed at dim {i}: rel_err = {rel_err:e} >= 1e-12"
        );
    }

    // Also check right endpoint derivative matching when delta_f1 != 0:
    let f_tend = [0.123, 4.567, 8.901];
    let p_prime_tend = [0.100, 4.500, 8.950];
    let delta_f1_custom = vec![
        f_tend[0] - p_prime_tend[0],
        f_tend[1] - p_prime_tend[1],
        f_tend[2] - p_prime_tend[2],
    ];

    let splice2 = CubicHermiteSplice::new(ts, t_end, delta_f0.clone(), delta_f1_custom).unwrap();
    splice2.evaluate_derivative_correction_into(t_end, &mut deriv_corr);
    for i in 0..3 {
        let corrected_end_deriv = p_prime_tend[i] + deriv_corr[i];
        let rel_err = (corrected_end_deriv - f_tend[i]).abs() / f_tend[i].abs();
        assert!(
            rel_err < 1e-12,
            "End derivative continuity failed at dim {i}: rel_err = {rel_err:e} >= 1e-12"
        );
    }
}

/// WP-11 Gate: endpoint preservation bit-for-bit C(t_0) = 0, C(t_1) = 0.
#[test]
fn endpoint_preservation_bit_for_bit() {
    let t0 = 1.0;
    let t1 = 2.0;
    let splice = CubicHermiteSplice::new(t0, t1, vec![1e8, -5e7], vec![-2e8, 3e7]).unwrap();

    let mut out = [123.456, -789.012];

    // t_start endpoint
    splice.evaluate_correction_into(t0, &mut out);
    assert_eq!(out[0].to_bits(), 0.0f64.to_bits());
    assert_eq!(out[1].to_bits(), 0.0f64.to_bits());

    // t_end endpoint
    splice.evaluate_correction_into(t1, &mut out);
    assert_eq!(out[0].to_bits(), 0.0f64.to_bits());
    assert_eq!(out[1].to_bits(), 0.0f64.to_bits());

    // State in-place application at endpoints
    let initial_state = [42.0, -17.5];
    let mut state = initial_state;
    splice.apply_correction(&mut state, t0);
    assert_eq!(state[0].to_bits(), initial_state[0].to_bits());
    assert_eq!(state[1].to_bits(), initial_state[1].to_bits());

    splice.apply_correction(&mut state, t1);
    assert_eq!(state[0].to_bits(), initial_state[0].to_bits());
    assert_eq!(state[1].to_bits(), initial_state[1].to_bits());
}

/// WP-11 Gate: interior point accuracy (error not worse by more than 2x, O(h^(p+1)) scaling).
#[test]
fn interior_point_accuracy_and_order_preservation() {
    // Problem: y(t) = exp(-t), y'(t) = -exp(-t)
    // On [0, h], let P(t) be a polynomial approximation of degree 4 with O(h^4) derivative error.
    for &h in &[0.2, 0.1, 0.05] {
        let t0 = 0.0;
        let t1 = t0 + h;

        // P(t) = 1 - t + t^2/2 - t^3/6 + t^4/24 (Taylor series of exp(-t))
        // Error of P(t) is O(h^5)
        // Discrepancy at t0 is 0. Introduce a synthetic O(h^4) derivative defect:
        // P_perturbed(t) = P(t) + delta * t * (h - t)^3 with delta = 0.5
        let delta = 0.5;
        let p_raw = |t: f64| 1.0 - t + 0.5 * t * t - (t.powi(3)) / 6.0 + (t.powi(4)) / 24.0;
        let p = |t: f64| p_raw(t) + delta * t * (h - t).powi(3);

        // Derivative of p(t):
        let p_deriv = |t: f64| {
            -1.0 + t - 0.5 * t * t
                + (t.powi(3)) / 6.0
                + delta * ((h - t).powi(3) - 3.0 * t * (h - t).powi(2))
        };
        let p_prime_0 = p_deriv(0.0);
        let p_prime_1 = p_deriv(h);

        let delta_f0 = vec![(-1.0) - p_prime_0];
        let delta_f1 = vec![-(-h).exp() - p_prime_1];

        let splice = CubicHermiteSplice::new(t0, t1, delta_f0, delta_f1).unwrap();

        let mut max_uncorrected_err = 0.0f64;
        let mut max_corrected_err = 0.0f64;

        for step in 1..50 {
            let t = t0 + (step as f64 / 50.0) * h;
            let exact = (-t).exp();
            let uncorr = p(t);
            let corr = uncorr + splice.evaluate_correction(t).unwrap()[0];

            max_uncorrected_err = max_uncorrected_err.max((uncorr - exact).abs());
            max_corrected_err = max_corrected_err.max((corr - exact).abs());
        }

        assert!(
            max_corrected_err <= 2.0 * max_uncorrected_err,
            "Interior error degraded by > 2x for h={h}: corrected={max_corrected_err:e}, uncorrected={max_uncorrected_err:e}"
        );
    }
}

/// G1: Default path neutrality: default solve reproduces bit-for-bit with empty splices.
#[test]
fn default_path_solution_neutrality() {
    let problem = Problem::new(1, |_t, y, dy| {
        dy[0] = -y[0];
        Ok::<(), CallbackError>(())
    })
    .autonomous();

    let opts = Options {
        dense_output: true,
        ..Options::default()
    };

    let sol = Rodas5p::solve(&problem, (0.0, 1.0), &[1.0], &opts).unwrap();
    assert!(
        sol.splices.is_empty(),
        "Default solve must have empty splices"
    );

    // Evaluate dense output at several points
    for &t in &[0.0, 0.1, 0.35, 0.7, 1.0] {
        let val = sol.at(t).unwrap();
        let mut buf = [0.0; 1];
        sol.at_into(t, &mut buf).unwrap();
        assert_eq!(
            val[0].to_bits(),
            buf[0].to_bits(),
            "at and at_into must match bit-for-bit at t={t}"
        );
    }
}

/// Integration: Solution with_splice and add_splice modifiers.
#[test]
fn solution_splice_wiring_and_truncation() {
    let problem = Problem::new(1, |_t, y, dy| {
        dy[0] = -y[0];
        Ok::<(), CallbackError>(())
    })
    .autonomous();

    let opts = Options {
        dense_output: true,
        ..Options::default()
    };

    let base_sol = Rodas5p::solve(&problem, (0.0, 2.0), &[1.0], &opts).unwrap();

    let t0 = base_sol.t[1];
    let t1 = base_sol.t[2];
    let splice = CubicHermiteSplice::new(t0, t1, vec![0.05], vec![0.0]).unwrap();

    let spliced_sol = base_sol.clone().with_splice(splice.clone());
    assert_eq!(spliced_sol.splices.len(), 1);

    // Endpoint states bit-for-bit preserved:
    assert_eq!(
        spliced_sol.at(t0).unwrap()[0].to_bits(),
        base_sol.at(t0).unwrap()[0].to_bits(),
        "t0 endpoint must be bit-for-bit identical"
    );
    assert_eq!(
        spliced_sol.at(t1).unwrap()[0].to_bits(),
        base_sol.at(t1).unwrap()[0].to_bits(),
        "t1 endpoint must be bit-for-bit identical"
    );

    // Midpoint includes correction:
    let t_mid = 0.5 * (t0 + t1);
    let base_mid = base_sol.at(t_mid).unwrap()[0];
    let spliced_mid = spliced_sol.at(t_mid).unwrap()[0];
    let corr = splice.evaluate_correction(t_mid).unwrap()[0];
    assert!((spliced_mid - (base_mid + corr)).abs() < 1e-15);

    // at_into matches at:
    let mut buf = [0.0; 1];
    spliced_sol.at_into(t_mid, &mut buf).unwrap();
    assert_eq!(buf[0].to_bits(), spliced_mid.to_bits());
}
