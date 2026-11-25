use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use rodas5p_integrators::dopri5_stiffness::{
    SecantCalibrationReport, SecantClassification, SecantScreenGuard, SecantStatus,
    calibrate_secant_screen, dopri5_secant_h_lambda_guarded,
};

struct CountingAlloc;
thread_local! {
    static TRACK_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
    static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
}
fn count_allocation() {
    if TRACK_ALLOCATIONS.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATION_COUNT.try_with(|count| count.set(count.get() + 1));
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

fn allocation_scope<T>(f: impl FnOnce() -> T) -> (T, usize) {
    ALLOCATION_COUNT.with(|count| count.set(0));
    TRACK_ALLOCATIONS.with(|track| track.set(true));
    let result = f();
    TRACK_ALLOCATIONS.with(|track| track.set(false));
    (result, ALLOCATION_COUNT.with(Cell::get))
}

const A: [[f64; 7]; 7] = [
    [0.0; 7],
    [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0, 0.0],
    [
        19372.0 / 6561.0,
        -25360.0 / 2187.0,
        64448.0 / 6561.0,
        -212.0 / 729.0,
        0.0,
        0.0,
        0.0,
    ],
    [
        9017.0 / 3168.0,
        -355.0 / 33.0,
        46732.0 / 5247.0,
        49.0 / 176.0,
        -5103.0 / 18656.0,
        0.0,
        0.0,
    ],
    [
        35.0 / 384.0,
        0.0,
        500.0 / 1113.0,
        125.0 / 192.0,
        -2187.0 / 6784.0,
        11.0 / 84.0,
        0.0,
    ],
];

fn linear_stages(h: f64, y: &[f64], lam: &[f64]) -> Vec<f64> {
    let n = y.len();
    let mut k = vec![0.0; 7 * n];
    for s in 0..7 {
        for i in 0..n {
            let mut yi = y[i];
            for j in 0..s {
                yi += h * A[s][j] * k[j * n + i];
            }
            k[s * n + i] = lam[i] * yi;
        }
    }
    k
}

#[test]
fn test_degenerate_input() {
    let mut guard = SecantScreenGuard::new();
    // Zero stages -> ||d||_2 == 0 < 1e-15 -> Degenerate
    let stages_zero = [0.0; 7];
    assert_eq!(
        dopri5_secant_h_lambda_guarded(1e-3, &stages_zero, 1, &mut guard),
        Err(SecantStatus::Degenerate)
    );

    // Negative or zero h
    let k = linear_stages(1e-3, &[1.0], &[-10.0]);
    assert_eq!(
        dopri5_secant_h_lambda_guarded(0.0, &k, 1, &mut guard),
        Err(SecantStatus::Degenerate)
    );
    assert_eq!(
        dopri5_secant_h_lambda_guarded(-1e-3, &k, 1, &mut guard),
        Err(SecantStatus::Degenerate)
    );

    // Short slice or n == 0
    assert_eq!(
        dopri5_secant_h_lambda_guarded(1e-3, &k[..6], 1, &mut guard),
        Err(SecantStatus::Degenerate)
    );
    assert_eq!(
        dopri5_secant_h_lambda_guarded(1e-3, &k, 0, &mut guard),
        Err(SecantStatus::Degenerate)
    );
}

#[test]
fn test_divergent_input() {
    let mut guard = SecantScreenGuard::new();

    // NaN / Inf in stages
    let mut k = linear_stages(1e-3, &[1.0], &[-10.0]);
    k[6] = f64::NAN;
    assert_eq!(
        dopri5_secant_h_lambda_guarded(1e-3, &k, 1, &mut guard),
        Err(SecantStatus::NonFiniteOrDivergent)
    );

    k[6] = f64::INFINITY;
    assert_eq!(
        dopri5_secant_h_lambda_guarded(1e-3, &k, 1, &mut guard),
        Err(SecantStatus::NonFiniteOrDivergent)
    );

    // Divergent slope: ||g||_2 / ||d||_2 > 1e15
    // Small d (e.g. 1e-10) with huge g (1e10) gives slope 1e20 > 1e15
    let mut k_div = vec![0.0; 7];
    // Stage 5 and 6 differ by 1e10, but earlier stages small
    k_div[0] = 1e-10;
    k_div[5] = 0.0;
    k_div[6] = 1e10;
    assert_eq!(
        dopri5_secant_h_lambda_guarded(1.0, &k_div, 1, &mut guard),
        Err(SecantStatus::NonFiniteOrDivergent)
    );
}

#[test]
fn test_kink_detection() {
    let mut guard = SecantScreenGuard::new();
    let k = linear_stages(1e-3, &[1.0], &[-10.0]);
    let val1 = dopri5_secant_h_lambda_guarded(1e-3, &k, 1, &mut guard)
        .expect("normal first step accepted");
    assert!((val1 - -0.01).abs() < 1e-6);

    // Case 1: Large step-to-step jump > 1e3
    guard.prev_h_lambda = Some(-2000.0);
    assert_eq!(
        dopri5_secant_h_lambda_guarded(1e-3, &k, 1, &mut guard),
        Err(SecantStatus::KinkDetected)
    );

    // Case 2: Sudden sign flip with high magnitude
    guard.prev_h_lambda = Some(50.0);
    // k_stiff has h_lambda = -3.2
    let k_stiff = linear_stages(3.2e-4, &[1.0], &[-1.0e4]);
    assert_eq!(
        dopri5_secant_h_lambda_guarded(3.2e-4, &k_stiff, 1, &mut guard),
        Err(SecantStatus::KinkDetected)
    );

    // Normal step with moderate change: no kink
    guard.prev_h_lambda = Some(-3.0);
    let val_stiff = dopri5_secant_h_lambda_guarded(3.2e-4, &k_stiff, 1, &mut guard)
        .expect("moderate step change accepted");
    assert!((val_stiff - -3.2).abs() < 1e-6);
}

#[test]
fn test_calibrate_secant_screen_four_regimes() {
    let report: SecantCalibrationReport = calibrate_secant_screen();

    // 1. Scalar linear
    assert!(report.scalar_linear.flags_stiffness);
    assert!(report.scalar_linear.relative_error < 1e-9);

    // 2. Nonlinear stiff (Robertson and Van der Pol)
    assert_eq!(report.nonlinear_stiff.len(), 2);
    for nl in &report.nonlinear_stiff {
        assert!(
            nl.detected_stiff_boundary,
            "problem {} must detect stiff boundary",
            nl.problem
        );
        assert!(nl.min_h_lambda < -1.0);
    }

    // 3. Nonnormal linear: J = [[-1, 10], [0, -2]]
    assert!(report.nonnormal_linear.depends_on_direction);
    assert!(report.nonnormal_linear.lies_outside_spectrum);
    assert_eq!(
        report.nonnormal_linear.classification,
        SecantClassification::EstimateOnly
    );

    // 4. Off-registry Brusselator
    assert_eq!(report.off_registry_brusselator.windows_evaluated, 24);
    assert_eq!(report.off_registry_brusselator.matching_windows, 14);
    assert!((report.off_registry_brusselator.agreement_fraction - 14.0 / 24.0).abs() < 1e-12);
    assert!(!report.off_registry_brusselator.meets_gate);
    assert_eq!(
        report.off_registry_brusselator.classification,
        SecantClassification::ObserverOnly
    );
}

#[test]
fn test_zero_allocation_guarded_evaluation() {
    let k = linear_stages(1e-3, &[1.0, 2.0], &[-1.0e4, -2.0e4]);
    let mut guard = SecantScreenGuard::new();

    let (res, allocs) =
        allocation_scope(|| dopri5_secant_h_lambda_guarded(1e-3, &k, 2, &mut guard));

    assert!(res.is_ok());
    assert_eq!(allocs, 0, "guarded secant must not allocate heap memory");
}
