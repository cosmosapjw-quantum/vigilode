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
    CoreError, FixedHLogNormObserver, GrowthBoundStatus, MovingHMetricObserver,
    QrSackerSellObserver, ResolventNumericalAbscissaObserver,
    directed::Interval,
    nonnormal_observers::{ValidatedSlabEnclosureObserver, point_interval_matrix_2x2},
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
    assert_eq!(
        fixed_euclidean.status(),
        GrowthBoundStatus::SampledDiagnostic
    );
    assert!(fixed_euclidean.status().is_sampled_or_estimate());
    assert!(!fixed_euclidean.status().is_certified());
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
    assert_eq!(moving_h.status(), GrowthBoundStatus::SampledDiagnostic);
    assert!(moving_h.status().is_sampled_or_estimate());
    assert!(!moving_h.status().is_certified());

    // Observer 3: QrSackerSellObserver
    let mut qr_obs = QrSackerSellObserver::new_2x2();
    let (rates, alpha_max_0) = qr_obs.compute_rates_2x2(&j);
    assert_eq!(rates[0], -1.0);
    assert_eq!(rates[1], -2.0);
    assert_eq!(alpha_max_0, -1.0);
    assert_eq!(qr_obs.status(), GrowthBoundStatus::SampledDiagnostic);
    assert!(qr_obs.status().is_sampled_or_estimate());
    assert!(!qr_obs.status().is_certified());

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
    assert!(qr_obs.orthogonality_defect() < 1e-12);

    // Observer 5: ValidatedSlabEnclosureObserver (Rigorous Full-Slab Enclosure)
    let mut validated_obs = ValidatedSlabEnclosureObserver::identity_2x2();
    assert_eq!(validated_obs.status(), GrowthBoundStatus::Certified);
    assert!(validated_obs.status().is_certified());
    assert!(!validated_obs.status().is_sampled_or_estimate());
    for _ in 0..280 {
        validated_obs.step_constant_j_2x2(dt, &j).unwrap();
    }
    let validated_bound_at_028 = validated_obs.cumulative_growth_bound();
    assert!(
        validated_bound_at_028 >= norm_at_028,
        "certified slab bound {validated_bound_at_028} must upper bound true norm {norm_at_028}"
    );
    assert_eq!(validated_obs.work_counters.slab_enclosures, 280);

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
    let mut validated_obs = ValidatedSlabEnclosureObserver::from_constant_h_2x2(h).unwrap();
    let j_int = point_interval_matrix_2x2(&j).unwrap();
    let h_int = point_interval_matrix_2x2(&h).unwrap();
    let hdot_int = point_interval_matrix_2x2(&hdot).unwrap();

    // Hot loop: 1,000 steps executing all 5 observers in-place
    let ((mu_fixed, mu_moving, alpha_qr, norm_res, mu_val), allocations, allocated_bytes) =
        count_allocations(|| {
            let mut last_mu_fixed = 0.0;
            let mut last_mu_moving = 0.0;
            let mut last_alpha_qr = 0.0;
            let mut last_norm_res = 0.0;
            let mut last_mu_val = 0.0;

            for _step in 0..1000 {
                last_mu_fixed = fixed_h.compute_mu_2x2(&j).unwrap();
                last_mu_moving = moving_h.step_2x2(dt, &j, &h, &hdot).unwrap();
                last_alpha_qr = qr_obs.step_2x2(dt, &j).unwrap();
                last_norm_res = res_obs.resolvent_norm_2x2(&j, s).unwrap();
                last_mu_val = validated_obs
                    .step_slab_2x2(dt, &j_int, &h_int, &hdot_int)
                    .unwrap();
            }

            (
                last_mu_fixed,
                last_mu_moving,
                last_alpha_qr,
                last_norm_res,
                last_mu_val,
            )
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
    assert_eq!(validated_obs.work_counters.slab_enclosures, 1000);

    assert_eq!(fixed_h.work_counters.hot_loop_allocations, 0);
    assert_eq!(moving_h.work_counters.hot_loop_allocations, 0);
    assert_eq!(qr_obs.work_counters.hot_loop_allocations, 0);
    assert_eq!(res_obs.work_counters.hot_loop_allocations, 0);
    assert_eq!(validated_obs.work_counters.hot_loop_allocations, 0);

    assert!(mu_fixed.is_finite());
    assert!(mu_moving.is_finite());
    assert!(alpha_qr.is_finite());
    assert!(norm_res.is_finite());
    assert!(mu_val.is_finite());
}

// ---------------------------------------------------------------------------
// Test 5: Negative Control - Missed Interior Peak by Pointwise Sampling
// ---------------------------------------------------------------------------

#[test]
fn test_negative_control_missed_peak() {
    // Problem: J(t) has mu_2(J(t_n)) <= 0 and mu_2(J(t_{n+1})) <= 0 at endpoints,
    // but in the interior of the slab t in (t_n, t_{n+1}), a large transient spike occurs
    // with peak mu_2(J) ~ +48.0.
    // Pointwise sampling at endpoints evaluates mu <= 0 and computes a cumulative bound <= 1.0 (decay).
    // The true continuous trajectory experiences amplification exp(48 * 0.1) ~ 121.5!
    // Sampled observers MUST NOT report Certified, but SampledDiagnostic.
    // The ValidatedSlabEnclosureObserver requires the interval enclosure over the slab [t_n, t_{n+1}],
    // correctly bounding the interior peak or refusing certification if the peak is unenclosed.

    let j_endpoint = [[-2.0, 0.0], [0.0, -2.0]];
    let h_id = [[1.0, 0.0], [0.0, 1.0]];
    let hdot_zero = [[0.0, 0.0], [0.0, 0.0]];
    let dt = 0.1;

    // 1. Sampled observer only evaluates at discrete endpoints
    let mut sampled_obs = MovingHMetricObserver::identity_2x2();
    let mu_sample = sampled_obs
        .step_2x2(dt, &j_endpoint, &h_id, &hdot_zero)
        .unwrap();
    assert_eq!(mu_sample, -2.0);
    assert!(
        sampled_obs.cumulative_growth_bound() < 1.0,
        "sampled observer falsely predicts decay"
    );
    assert_eq!(sampled_obs.status(), GrowthBoundStatus::SampledDiagnostic);
    assert!(!sampled_obs.status().is_certified());

    // 2. Validated observer with slab enclosure enclosing the interior peak:
    // Over the slab [t_n, t_{n+1}], J_01 has peak in [0.0, 100.0]
    let j_slab = [
        [
            Interval::new(-2.0, -2.0).unwrap(),
            Interval::new(0.0, 100.0).unwrap(),
        ],
        [
            Interval::new(0.0, 0.0).unwrap(),
            Interval::new(-2.0, -2.0).unwrap(),
        ],
    ];
    let h_slab = [
        [Interval::point(1.0).unwrap(), Interval::point(0.0).unwrap()],
        [Interval::point(0.0).unwrap(), Interval::point(1.0).unwrap()],
    ];
    let hdot_slab = [
        [Interval::point(0.0).unwrap(), Interval::point(0.0).unwrap()],
        [Interval::point(0.0).unwrap(), Interval::point(0.0).unwrap()],
    ];

    let mut validated_obs = ValidatedSlabEnclosureObserver::identity_2x2();
    let mu_slab_upper = validated_obs
        .step_slab_2x2(dt, &j_slab, &h_slab, &hdot_slab)
        .unwrap();
    // mu_2 of [[-2, 100], [0, -2]] is -2 + 50 = 48.0
    assert!(
        mu_slab_upper >= 48.0,
        "validated observer captures interior peak: {mu_slab_upper}"
    );
    assert!(
        validated_obs.cumulative_growth_bound() > 100.0,
        "validated bound certifies amplification"
    );
    assert_eq!(validated_obs.status(), GrowthBoundStatus::Certified);
    assert!(validated_obs.status().is_certified());
}

// ---------------------------------------------------------------------------
// Test 6: Negative Control - Fast Metric Oscillation Hdot Transport Term
// ---------------------------------------------------------------------------

#[test]
fn test_negative_control_fast_h_oscillation() {
    // If H(t) oscillates with high frequency omega = 1000,
    // Hdot(t) has amplitude omega * epsilon = 100.0.
    // Pointwise sampling at intervals Delta t >> 1/omega will sample arbitrary phase points,
    // missing the peak transport rate.
    // Validated observer requires interval enclosure of Hdot on the entire slab.
    let omega_eps = 100.0;
    let hdot_slab = [
        [
            Interval::new(-omega_eps, omega_eps).unwrap(),
            Interval::point(0.0).unwrap(),
        ],
        [
            Interval::point(0.0).unwrap(),
            Interval::new(-omega_eps, omega_eps).unwrap(),
        ],
    ];
    let j_slab = [
        [
            Interval::point(-1.0).unwrap(),
            Interval::point(0.0).unwrap(),
        ],
        [
            Interval::point(0.0).unwrap(),
            Interval::point(-1.0).unwrap(),
        ],
    ];
    let h_slab = [
        [
            Interval::new(0.8, 1.2).unwrap(),
            Interval::point(0.0).unwrap(),
        ],
        [
            Interval::point(0.0).unwrap(),
            Interval::new(0.8, 1.2).unwrap(),
        ],
    ];

    let mut validated_obs = ValidatedSlabEnclosureObserver::identity_2x2();
    let mu_upper = validated_obs
        .step_slab_2x2(0.01, &j_slab, &h_slab, &hdot_slab)
        .unwrap();
    assert!(
        mu_upper >= 40.0,
        "fast H oscillation transport term must be enclosed by slab observer, got {mu_upper}"
    );
    assert_eq!(validated_obs.status(), GrowthBoundStatus::Certified);
}

// ---------------------------------------------------------------------------
// Test 7: Negative Control - Singular or Indefinite Metric Rejection
// ---------------------------------------------------------------------------

#[test]
fn test_negative_control_singular_or_indefinite_h() {
    // 1. Singular matrix: det = 0
    let h_singular = [[1.0, 1.0], [1.0, 1.0]];
    assert!(MovingHMetricObserver::new_2x2(h_singular).is_err());
    assert!(ValidatedSlabEnclosureObserver::from_constant_h_2x2(h_singular).is_err());

    // 2. Indefinite matrix: det = -3 < 0
    let h_indefinite = [[1.0, 2.0], [2.0, 1.0]];
    assert!(MovingHMetricObserver::new_2x2(h_indefinite).is_err());
    assert!(ValidatedSlabEnclosureObserver::from_constant_h_2x2(h_indefinite).is_err());

    // 3. Interval matrix that degenerates to singularity on slab
    let h_slab_singular = [
        [
            Interval::new(0.0, 1.0).unwrap(),
            Interval::point(0.0).unwrap(),
        ],
        [Interval::point(0.0).unwrap(), Interval::point(1.0).unwrap()],
    ];
    assert!(ValidatedSlabEnclosureObserver::new_2x2(h_slab_singular).is_err());

    // 4. Non-finite matrix
    let h_nan = [[f64::NAN, 0.0], [0.0, 1.0]];
    assert!(MovingHMetricObserver::new_2x2(h_nan).is_err());
    assert!(ValidatedSlabEnclosureObserver::from_constant_h_2x2(h_nan).is_err());
}

// ---------------------------------------------------------------------------
// Test 8: Negative Control - Metric Jump Factor Rejection and Outward Bound
// ---------------------------------------------------------------------------

#[test]
fn test_negative_control_metric_jump_invalid() {
    let mut obs = ValidatedSlabEnclosureObserver::identity_2x2();

    // Invalid jump target: singular matrix must fail closed
    let h_bad = [[0.0, 0.0], [0.0, 0.0]];
    let res = obs.switch_metric_constant_2x2(h_bad);
    assert!(
        res.is_err(),
        "switching to singular metric must fail closed"
    );

    // Valid jump target: outward enclosure
    let h_good = [[4.0, 0.0], [0.0, 1.0]];
    let gamma = obs.switch_metric_constant_2x2(h_good).unwrap();
    assert!(
        gamma >= 2.0,
        "jump factor from I to diag(4,1) must be >= 2.0, got {gamma}"
    );
    assert_eq!(obs.cumulative_jump_factor(), gamma);
}

// ---------------------------------------------------------------------------
// Test 9: Negative Control - Continuous QR Orthogonality Defect
// ---------------------------------------------------------------------------

#[test]
fn test_negative_control_qr_loss_and_orthogonality() {
    let mut qr_obs = QrSackerSellObserver::new_2x2();
    assert_eq!(qr_obs.orthogonality_defect(), 0.0);
    assert_eq!(qr_obs.status(), GrowthBoundStatus::SampledDiagnostic);
    assert!(!qr_obs.status().is_certified());

    // Step with ill-conditioned matrix
    let stiff_j = [[-1000.0, 5000.0], [0.0, -2000.0]];
    for _ in 0..100 {
        let _ = qr_obs.step_2x2(0.001, &stiff_j);
    }
    let defect = qr_obs.orthogonality_defect();
    assert!(defect >= 0.0);
    assert!(defect.is_finite());
    // SampledDiagnostic status remains uncertified
    assert_eq!(qr_obs.status(), GrowthBoundStatus::SampledDiagnostic);
}

// ---------------------------------------------------------------------------
// Test 10: Negative Control - Directed Exponential Overflow Protection
// ---------------------------------------------------------------------------

#[test]
fn test_negative_control_exp_overflow() {
    // When integrated exponent exceeds 709.0, ordinary f64 exp() produces infinity.
    // ValidatedSlabEnclosureObserver uses directed exp_interval which fails closed
    // with CoreError::NonFinite rather than silently certifying infinity.
    let j_huge = [[800.0, 0.0], [0.0, 800.0]];
    let mut validated_obs = ValidatedSlabEnclosureObserver::identity_2x2();
    let res = validated_obs.step_constant_j_2x2(1.0, &j_huge);
    assert!(
        res.is_err(),
        "exponential overflow in validated observer must fail closed"
    );
    match res {
        Err(CoreError::NonFinite(msg)) => {
            assert!(
                msg.contains("exp overflow"),
                "expected exp overflow, got {msg}"
            );
        }
        other => panic!("expected CoreError::NonFinite, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 11: Positive Control - Diagonal Decay Certified Enclosure
// ---------------------------------------------------------------------------

#[test]
fn test_validated_slab_enclosure_diagonal_decay() {
    // Diagonal stable system J = [[-1.0, 0.0], [0.0, -3.0]]
    // True matrix exponential norm is exactly exp(-t) <= 1.0 for all t >= 0.
    let j = [[-1.0, 0.0], [0.0, -3.0]];
    let mut validated_obs = ValidatedSlabEnclosureObserver::identity_2x2();
    assert_eq!(validated_obs.status(), GrowthBoundStatus::Certified);

    let dt = 0.05;
    for _ in 0..20 {
        let mu = validated_obs.step_constant_j_2x2(dt, &j).unwrap();
        assert!(mu <= -1.0, "mu_2(J) must be <= -1.0, got {mu}");
    }
    let bound = validated_obs.cumulative_growth_bound();
    assert!(
        bound <= (-1.0_f64).exp() + 1e-12,
        "bound must certify decay <= exp(-1)"
    );
    assert!(bound < 1.0, "certified bound confirms monotonic decay");
    assert_eq!(validated_obs.status(), GrowthBoundStatus::Certified);
}
