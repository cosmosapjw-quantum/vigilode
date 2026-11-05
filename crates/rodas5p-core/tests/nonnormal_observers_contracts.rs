//! Verification contracts for Node P4 (P4_NONNORMAL_OBSERVERS).
//!
//! 1. Section 6 nonnormal test: J = [[-1.0, 10.0], [0.0, -2.0]].
//!    Eigenvalues: -1.0, -2.0.
//!    mu_2(J) = (-3 + sqrt(101))/2 ~ 3.5249378.
//!    Max norm ||exp(t J)||_2 ~ 2.5635 at t ~ 0.28 (value 2.064) / peak 2.5635 at t ~ 0.66.
//!    Verify all 4 observers on this test problem.
//! 2. Time-dependent metric test: verify that Hdot(t) transport term contributes non-trivially and correctly.
//! 3. Metric jump factor test: verify gamma_jump accumulation on discontinuous metric switch.
//! 4. VG-1 Zero-allocation test: 1,000 steps with 0 allocations.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::atomic::{AtomicU64, Ordering},
};

use rodas5p_core::{
    FixedHLogNormObserver, GrowthBoundStatus, MovingHMetricObserver, QrSackerSellObserver,
    ResolventNumericalAbscissaObserver,
};

// ---------------------------------------------------------------------------
// Counting Allocator for VG-1 Zero-Allocation Enforcement
// ---------------------------------------------------------------------------

struct CountingAllocator;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
}
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.try_with(|t| t.get()).unwrap_or(false) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn count_allocations<T>(operation: impl FnOnce() -> T) -> (T, u64, u64) {
    let _ = TRACK.try_with(|t| t.set(false));
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    let _ = TRACK.try_with(|t| t.set(true));
    let result = operation();
    let _ = TRACK.try_with(|t| t.set(false));
    (
        result,
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
    )
}

/// Computes the exact Euclidean matrix 2-norm of exp(t J) for J = [[-1, 10], [0, -2]].
///
/// exp(t J) = [[exp(-t), 10 (exp(-t) - exp(-2t))], [0, exp(-2t)]].
fn exact_exp_norm_section6(t: f64) -> f64 {
    let e_neg_t = (-t).exp();
    let e_neg_2t = (-2.0 * t).exp();
    let a = e_neg_t;
    let b = 10.0 * (e_neg_t - e_neg_2t);
    let d = e_neg_2t;

    // E E^T = [[a^2 + b^2, b d], [b d, d^2]]
    let tr = a * a + b * b + d * d;
    let det = (a * d) * (a * d);
    let disc = (tr * tr - 4.0 * det).max(0.0).sqrt();
    let lam_max = 0.5 * (tr + disc);
    lam_max.sqrt()
}

// ---------------------------------------------------------------------------
// Test 1: Section 6 Nonnormal Benchmark Problem (All 4 Observers)
// ---------------------------------------------------------------------------

#[test]
fn test_section6_nonnormal_matrix_all_observers() {
    let j = [[-1.0, 10.0], [0.0, -2.0]];

    // 1. Eigenvalues and Spectral Abscissa
    let alpha = ResolventNumericalAbscissaObserver::spectral_abscissa_2x2(&j);
    assert_eq!(
        alpha, -1.0,
        "eigenvalues of upper triangular J are -1.0 and -2.0"
    );

    // 2. Exact Euclidean Numerical Abscissa
    let expected_mu2 = 0.5 * (-3.0 + (101.0_f64).sqrt());
    assert!((expected_mu2 - 3.5249378106).abs() < 1e-9);

    let mu2 = FixedHLogNormObserver::compute_mu_euclidean_2x2(&j);
    assert!(
        (mu2 - expected_mu2).abs() < 1e-14,
        "mu_2(J) must match (-3 + sqrt(101))/2 ~ 3.5249378"
    );

    // 3. Matrix exponential norm behavior
    // At t = 0: ||exp(0)||_2 = 1.0
    assert!((exact_exp_norm_section6(0.0) - 1.0).abs() < 1e-14);

    // At t = 0.28: ||exp(0.28 J)||_2 ~ 2.0641
    let norm_at_028 = exact_exp_norm_section6(0.28);
    assert!((norm_at_028 - 2.06410408).abs() < 1e-6);

    // Find peak over [0, 2.0]
    let mut max_norm = 0.0_f64;
    let mut t_at_max = 0.0_f64;
    for step in 0..2000 {
        let t = step as f64 * 0.001;
        let norm = exact_exp_norm_section6(t);
        if norm > max_norm {
            max_norm = norm;
            t_at_max = t;
        }
    }
    // Peak norm is ~ 2.563493 at t ~ 0.662
    assert!(
        (max_norm - 2.56349288).abs() < 1e-5,
        "max norm ||exp(t J)||_2 must be approximately 2.5635, got {max_norm}"
    );
    assert!((t_at_max - 0.662).abs() < 0.01);

    // Observer 1: FixedHLogNormObserver
    let mut fixed_euclidean = FixedHLogNormObserver::identity(2);
    let mu_i = fixed_euclidean.compute_mu_2x2(&j).unwrap();
    assert!((mu_i - expected_mu2).abs() < 1e-14);
    assert_eq!(fixed_euclidean.status(), GrowthBoundStatus::Certified);
    assert_eq!(fixed_euclidean.work_counters.eigen_solves, 1);

    // With a properly scaled metric H = diag(1, 100), log norm becomes negative!
    let mut fixed_scaled = FixedHLogNormObserver::diagonal(&[1.0, 100.0]).unwrap();
    let mu_scaled = fixed_scaled.compute_mu_2x2(&j).unwrap();
    let expected_scaled = 0.5 * (-3.0 + 2.0_f64.sqrt()); // (-3 + sqrt(2))/2 ~ -0.79289
    assert!(
        (mu_scaled - expected_scaled).abs() < 1e-12,
        "in metric H = diag(1, 100), mu_H(J) = (-3 + sqrt(2))/2 ~ -0.79289, got {mu_scaled}"
    );
    assert!(mu_scaled < 0.0, "scaled metric proves monotonic decay");

    // Observer 2: MovingHMetricObserver
    let mut moving_h = MovingHMetricObserver::identity_2x2();
    let h_id = [[1.0, 0.0], [0.0, 1.0]];
    let hdot_zero = [[0.0, 0.0], [0.0, 0.0]];
    let dt = 0.001;
    for _ in 0..280 {
        moving_h.step_2x2(dt, &j, &h_id, &hdot_zero).unwrap();
    }
    // Bound at t = 0.28: exp(mu_2 * 0.28) ~ exp(3.5249378 * 0.28) ~ 2.6831 >= 2.0641
    let bound_at_028 = moving_h.cumulative_growth_bound();
    assert!(
        bound_at_028 >= norm_at_028,
        "bound {bound_at_028} must upper bound true norm {norm_at_028}"
    );
    assert!((bound_at_028 - (expected_mu2 * 0.28).exp()).abs() < 1e-8);
    assert_eq!(moving_h.status(), GrowthBoundStatus::Certified);

    // Observer 3: QrSackerSellObserver
    let mut qr_obs = QrSackerSellObserver::new_2x2();
    let (rates, alpha_max_0) = qr_obs.compute_rates_2x2(&j);
    assert_eq!(rates[0], -1.0);
    assert_eq!(rates[1], -2.0);
    assert_eq!(alpha_max_0, -1.0);
    assert_eq!(qr_obs.status(), GrowthBoundStatus::Certified);

    // Step continuous QR for 1,000 steps
    for _ in 0..1000 {
        qr_obs.step_2x2(dt, &j).unwrap();
        let q = qr_obs.current_q_2x2();
        // Verify Q remains orthonormal
        let norm_col0 = (q[0][0] * q[0][0] + q[1][0] * q[1][0]).sqrt();
        let norm_col1 = (q[0][1] * q[0][1] + q[1][1] * q[1][1]).sqrt();
        let dot = (q[0][0] * q[0][1] + q[1][0] * q[1][1]).abs();
        assert!((norm_col0 - 1.0).abs() < 1e-12);
        assert!((norm_col1 - 1.0).abs() < 1e-12);
        assert!(dot < 1e-12);
    }
    assert_eq!(qr_obs.work_counters.qr_steps, 1000);

    // Observer 4: ResolventNumericalAbscissaObserver
    let mut res_obs = ResolventNumericalAbscissaObserver::new();
    let analysis = res_obs.analyze_2x2(&j).unwrap();

    assert_eq!(analysis.spectral_abscissa, -1.0);
    assert!((analysis.numerical_abscissa - expected_mu2).abs() < 1e-12);
    assert!(
        analysis.has_transient_amplification,
        "transient amplification must be detected: alpha < 0 but mu_2 > 0"
    );
    assert_eq!(analysis.kreiss_status, GrowthBoundStatus::EstimateOnly);
    assert_eq!(analysis.upper_status, GrowthBoundStatus::EstimateOnly);

    // Kreiss lower bound: sup_{s > 0} s ||(s I - J)^-1||_2 ~ 1.865 > 1.0
    assert!(
        analysis.kreiss_lower_bound > 1.8,
        "Kreiss lower bound must exceed 1.8, got {}",
        analysis.kreiss_lower_bound
    );
    assert!(
        analysis.kreiss_lower_bound <= max_norm + 1e-6,
        "Kreiss lower bound {} must not exceed true max norm {}",
        analysis.kreiss_lower_bound,
        max_norm
    );
    assert!(
        analysis.resolvent_upper_estimate >= max_norm,
        "resolvent upper estimate {} must upper bound true max norm {}",
        analysis.resolvent_upper_estimate,
        max_norm
    );
}

// ---------------------------------------------------------------------------
// Test 2: Time-Dependent Metric Transport Term (Hdot Contribution)
// ---------------------------------------------------------------------------

#[test]
fn test_timedependent_metric_transport_term() {
    let j = [[-1.0, 10.0], [0.0, -2.0]];
    let mu2 = FixedHLogNormObserver::compute_mu_euclidean_2x2(&j);

    // For H(t) = e^{2 sigma t} I, Hdot(t) = 2 sigma e^{2 sigma t} I.
    // M_H(t) = 1/2 H^{-1/2} [Hdot + J^T H + H J] H^{-1/2}
    //        = sigma I + 1/2 (J^T + J)
    // lambda_max(M_H(t)) = mu_2(J) + sigma.
    for sigma in [1.5_f64, -2.5, 0.75, 4.0] {
        let mut obs = MovingHMetricObserver::identity_2x2();
        let t = 0.5;
        let scale = (2.0 * sigma * t).exp();
        let h = [[scale, 0.0], [0.0, scale]];
        let hdot = [[2.0 * sigma * scale, 0.0], [0.0, 2.0 * sigma * scale]];

        let mu_with_hdot = obs.compute_mu_h_2x2(&j, &h, &hdot).unwrap();
        let expected = mu2 + sigma;

        assert!(
            (mu_with_hdot - expected).abs() < 1e-12,
            "transport term mismatch for sigma={sigma}: expected {expected}, got {mu_with_hdot}"
        );

        // Negative control: omitting Hdot gives wrong growth rate
        let hdot_zero = [[0.0, 0.0], [0.0, 0.0]];
        let mu_without_hdot = obs.compute_mu_h_2x2(&j, &h, &hdot_zero).unwrap();
        assert!(
            (mu_without_hdot - mu2).abs() < 1e-12,
            "without Hdot, recovers mu_2 = {mu2}, got {mu_without_hdot}"
        );
        assert!(
            (mu_with_hdot - mu_without_hdot - sigma).abs() < 1e-12,
            "difference between with and without Hdot must exactly equal sigma={sigma}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 3: Metric Jump Factor Accumulation on Discontinuous Switch
// ---------------------------------------------------------------------------

#[test]
fn test_metric_jump_factor() {
    let mut obs = MovingHMetricObserver::identity_2x2();
    assert_eq!(obs.cumulative_jump_factor(), 1.0);
    assert_eq!(obs.cumulative_growth_bound(), 1.0);

    // Discontinuous switch 1: H_- = I, H_+ = diag(4.0, 1.0)
    // gamma_jump = sqrt(lambda_max(I^{-1/2} diag(4, 1) I^{-1/2})) = sqrt(4.0) = 2.0
    let h_plus_1 = [[4.0, 0.0], [0.0, 1.0]];
    let gamma1 = obs.switch_metric_2x2(h_plus_1).unwrap();
    assert_eq!(gamma1, 2.0, "jump factor from I to diag(4,1) must be 2.0");
    assert_eq!(obs.cumulative_jump_factor(), 2.0);
    assert_eq!(obs.cumulative_growth_bound(), 2.0);
    assert_eq!(obs.work_counters.jump_factor_computations, 1);
    assert_eq!(obs.work_counters.metric_updates, 1);

    // Discontinuous switch 2: H_- = diag(4.0, 1.0), H_+ = diag(4.0, 9.0)
    // H_-^{-1/2} H_+ H_-^{-1/2} = diag(1/2, 1) diag(4, 9) diag(1/2, 1) = diag(1, 9)
    // gamma_jump = sqrt(9.0) = 3.0
    let h_plus_2 = [[4.0, 0.0], [0.0, 9.0]];
    let gamma2 = obs.switch_metric_2x2(h_plus_2).unwrap();
    assert_eq!(
        gamma2, 3.0,
        "jump factor from diag(4,1) to diag(4,9) must be 3.0"
    );
    assert_eq!(
        obs.cumulative_jump_factor(),
        6.0,
        "accumulated jump factor 2.0 * 3.0 = 6.0"
    );
    assert_eq!(obs.cumulative_growth_bound(), 6.0);
    assert_eq!(obs.work_counters.jump_factor_computations, 2);
    assert_eq!(obs.work_counters.metric_updates, 2);

    // Discontinuous switch 3 with non-diagonal SPD matrix:
    // H_- = diag(4, 9), H_+ = [[5.0, 2.0], [2.0, 5.0]] (eigenvalues 7.0 and 3.0)
    let h_plus_3 = [[5.0, 2.0], [2.0, 5.0]];
    let gamma3 = obs.switch_metric_2x2(h_plus_3).unwrap();
    assert!(gamma3.is_finite() && gamma3 > 0.0);
    assert_eq!(obs.work_counters.jump_factor_computations, 3);
    assert_eq!(obs.work_counters.metric_updates, 3);
}

// ---------------------------------------------------------------------------
// Test 4: VG-1 Zero-Allocation Compliance (1,000 Steps with 0 Allocations)
// ---------------------------------------------------------------------------

#[test]
fn test_vg1_zero_allocation() {
    let j = [[-1.0, 10.0], [0.0, -2.0]];
    let h = [[1.5, 0.2], [0.2, 2.0]];
    let hdot = [[0.05, 0.01], [0.01, 0.02]];
    let dt = 0.001;
    let s = 1.5;

    // Warm up / construct observers outside the counting window
    let mut fixed_h = FixedHLogNormObserver::from_2x2(h).unwrap();
    let mut moving_h = MovingHMetricObserver::new_2x2(h).unwrap();
    let mut qr_obs = QrSackerSellObserver::new_2x2();
    let res_obs = ResolventNumericalAbscissaObserver::new();

    // Hot loop: 1,000 steps executing all 4 observers in-place
    let ((mu_fixed, mu_moving, alpha_qr, norm_res), allocations, allocated_bytes) =
        count_allocations(|| {
            let mut last_mu_fixed = 0.0;
            let mut last_mu_moving = 0.0;
            let mut last_alpha_qr = 0.0;
            let mut last_norm_res = 0.0;

            for _step in 0..1000 {
                last_mu_fixed = fixed_h.compute_mu_2x2(&j).unwrap();
                last_mu_moving = moving_h.step_2x2(dt, &j, &h, &hdot).unwrap();
                last_alpha_qr = qr_obs.step_2x2(dt, &j).unwrap();
                last_norm_res = res_obs.resolvent_norm_2x2(&j, s).unwrap();
            }

            (last_mu_fixed, last_mu_moving, last_alpha_qr, last_norm_res)
        });

    assert_eq!(
        allocations, 0,
        "VG-1 VIOLATION: hot loop allocated {allocations} times ({allocated_bytes} bytes)"
    );
    assert_eq!(allocated_bytes, 0);

    // Verify all counters recorded expected progress
    assert_eq!(fixed_h.work_counters.eigen_solves, 1000);
    assert_eq!(moving_h.work_counters.eigen_solves, 1000);
    assert_eq!(moving_h.work_counters.metric_updates, 1000);
    assert_eq!(qr_obs.work_counters.qr_steps, 1000);

    assert_eq!(fixed_h.work_counters.hot_loop_allocations, 0);
    assert_eq!(moving_h.work_counters.hot_loop_allocations, 0);
    assert_eq!(qr_obs.work_counters.hot_loop_allocations, 0);
    assert_eq!(res_obs.work_counters.hot_loop_allocations, 0);

    assert!(mu_fixed.is_finite());
    assert!(mu_moving.is_finite());
    assert!(alpha_qr.is_finite());
    assert!(norm_res.is_finite());
}

#[test]
fn finite_precision_observer_status_requires_outward_error_enclosure() {
    let j = [[-1.0, 10.0], [0.0, -2.0]];
    let dt = 0.001;

    // 1. Continuous-time theorem on exact identity metric yields Certified:
    let fixed_euclidean = FixedHLogNormObserver::identity(2);
    assert_eq!(
        fixed_euclidean.status(),
        GrowthBoundStatus::Certified,
        "continuous-time theorem on exact identity metric is certified"
    );

    // General floating-point metric point estimate without outward error envelope is EstimateOnly:
    let h_general = [[2.0, 0.5], [0.5, 3.0]];
    let fixed_general = FixedHLogNormObserver::from_2x2(h_general).unwrap();
    assert_eq!(
        fixed_general.status(),
        GrowthBoundStatus::EstimateOnly,
        "general floating-point metric point estimate without error envelope must be EstimateOnly"
    );

    // 2. Moving-H: time integration with moving metric without outward quadrature envelope becomes EstimateOnly:
    let mut moving_h = MovingHMetricObserver::identity_2x2();
    let h_moving = [[1.0, 0.0], [0.0, 2.0]];
    let hdot_moving = [[0.0, 0.0], [0.0, 1.0]];
    moving_h.step_2x2(dt, &j, &h_moving, &hdot_moving).unwrap();
    assert_eq!(
        moving_h.status(),
        GrowthBoundStatus::EstimateOnly,
        "discrete time integration on moving metric without quadrature envelope must be EstimateOnly"
    );

    // 3. QR Sacker-Sell: finite-step integration without method-order/global error bounds becomes EstimateOnly:
    let mut qr_obs = QrSackerSellObserver::new_2x2();
    assert_eq!(qr_obs.status(), GrowthBoundStatus::Certified);
    qr_obs.step_2x2(dt, &j).unwrap();
    assert_eq!(
        qr_obs.status(),
        GrowthBoundStatus::EstimateOnly,
        "finite-step QR integration without error envelope must be EstimateOnly"
    );

    // 4. Negative controls: indefinite and singular metrics must be rejected
    let indefinite_metric = [[1.0, 2.0], [2.0, 1.0]];
    assert!(
        FixedHLogNormObserver::from_2x2(indefinite_metric).is_err(),
        "indefinite metric must be rejected"
    );
    assert!(
        MovingHMetricObserver::new_2x2(indefinite_metric).is_err(),
        "indefinite metric must be rejected by Moving-H"
    );

    let singular_metric = [[1.0, 1.0], [1.0, 1.0]];
    assert!(
        FixedHLogNormObserver::from_2x2(singular_metric).is_err(),
        "singular metric must be rejected"
    );
    assert!(
        MovingHMetricObserver::new_2x2(singular_metric).is_err(),
        "singular metric must be rejected by Moving-H"
    );
}
