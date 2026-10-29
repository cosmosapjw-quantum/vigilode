use rodas5p_core::CoreError;
use rodas5p_integrators::AffineErrorNorm;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counted;
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
fn charge() {
    if ENABLED.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        charge();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        charge();
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Counted = Counted;

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 3e-15 * expected.abs().max(1.0),
        "actual {actual:.17e}, expected {expected:.17e}"
    );
}

#[test]
fn full_physical_metric_includes_eliminated_correlated_errors() {
    let n = 30;
    let m = 33;
    let mut lift = vec![0.0; m * n];
    for i in 0..n {
        lift[i * n + i] = 1.0;
        lift[(30 + i / 10) * n + i] = -1.0;
    }
    let norm = AffineErrorNorm::new(n, vec![0.0; m], lift, vec![1.0], 0.0).unwrap();
    let error = vec![0.4; n];
    let actual = norm.evaluate(&vec![0.0; n], &vec![0.0; n], &error).unwrap();
    close(actual, 0.4 * 10.0_f64.sqrt());
    assert!(0.4 <= 1.0 && actual > 1.0);
    assert_eq!(norm.nonzeros(), 60);
    assert_eq!(norm.physical_dimension(), 33);
    assert_eq!(norm.reduced_dimension(), 30);
}

#[test]
fn affine_offset_affects_scales_but_never_the_error() {
    let norm =
        AffineErrorNorm::new(1, vec![100.0, -20.0], vec![2.0, -3.0], vec![0.5, 0.25], 0.1).unwrap();
    let actual = norm.evaluate(&[1.0], &[3.0], &[0.01]).unwrap();
    let expected = ((0.02_f64 / 11.1).powi(2) + (0.03_f64 / 3.15).powi(2)).sqrt() / 2.0_f64.sqrt();
    close(actual, expected);
}

#[test]
fn reparameterization_preserves_the_same_physical_metric() {
    // v=[1,-2]+[2,3; -1,4]u, q=[2u0+1, 4u1-2].
    let original = AffineErrorNorm::new(
        2,
        vec![1.0, -2.0],
        vec![2.0, 3.0, -1.0, 4.0],
        vec![0.1, 0.2],
        0.03,
    )
    .unwrap();
    let transformed = AffineErrorNorm::new(
        2,
        vec![1.5, 0.5],
        vec![1.0, 0.75, -0.5, 1.0],
        vec![0.1, 0.2],
        0.03,
    )
    .unwrap();
    close(
        original
            .evaluate(&[0.5, -1.0], &[0.75, -0.5], &[0.01, -0.02])
            .unwrap(),
        transformed
            .evaluate(&[2.0, -6.0], &[2.5, -4.0], &[0.02, -0.08])
            .unwrap(),
    );
}

#[test]
fn redundant_reduced_coordinates_do_not_change_physical_dimension_or_norm() {
    let a = AffineErrorNorm::new(1, vec![0.0], vec![1.0], vec![1.0], 0.0).unwrap();
    let b = AffineErrorNorm::new(2, vec![0.0], vec![1.0, 1.0], vec![1.0], 0.0).unwrap();
    assert_eq!(
        a.evaluate(&[1.0], &[2.0], &[0.5]).unwrap(),
        b.evaluate(&[0.5, 0.5], &[1.5, 0.5], &[0.2, 0.3]).unwrap()
    );
    assert_eq!(
        b.evaluate(&[0.0, 0.0], &[0.0, 0.0], &[1.0, -1.0]).unwrap(),
        0.0
    );
}

#[test]
fn cancellation_retains_small_physical_error() {
    let norm = AffineErrorNorm::new(3, vec![0.0], vec![1.0, 1.0, 1.0], vec![1.0], 0.0).unwrap();
    assert_eq!(
        norm.evaluate(&[0.0; 3], &[0.0; 3], &[1e16, 1.0, -1e16])
            .unwrap(),
        1.0
    );
}

#[test]
fn stable_rms_does_not_overflow_or_underflow_representable_values() {
    let norm =
        AffineErrorNorm::new(2, vec![0.0; 2], vec![1.0, 0.0, 0.0, 1.0], vec![1.0], 0.0).unwrap();
    assert_eq!(
        norm.evaluate(&[0.0; 2], &[0.0; 2], &[1e300; 2]).unwrap(),
        1e300
    );
    assert_eq!(
        norm.evaluate(&[0.0; 2], &[0.0; 2], &[1e-300; 2]).unwrap(),
        1e-300
    );
}

#[test]
fn zero_scale_and_nonfinite_paths_fail_closed() {
    let norm = AffineErrorNorm::new(1, vec![0.0], vec![1.0], vec![0.0], 1.0).unwrap();
    assert_eq!(norm.evaluate(&[0.0], &[0.0], &[0.0]).unwrap(), 0.0);
    assert_eq!(
        norm.evaluate(&[0.0], &[0.0], &[1.0]).unwrap(),
        f64::INFINITY
    );
    assert!(matches!(
        norm.evaluate(&[0.0], &[0.0], &[f64::NAN]),
        Err(CoreError::NonFinite(_))
    ));
    let overflow = AffineErrorNorm::new(1, vec![0.0], vec![2.0], vec![1.0], 0.0).unwrap();
    assert!(matches!(
        overflow.evaluate(&[f64::MAX], &[0.0], &[0.0]),
        Err(CoreError::NonFinite(_))
    ));
    let scale = AffineErrorNorm::new(1, vec![0.0], vec![1.0], vec![f64::MAX], 2.0).unwrap();
    assert!(matches!(
        scale.evaluate(&[f64::MAX], &[0.0], &[0.0]),
        Err(CoreError::InvalidInput(_))
    ));
}

#[test]
fn malformed_dimensions_and_tolerances_are_rejected() {
    assert!(AffineErrorNorm::new(0, vec![0.0], vec![], vec![1.0], 0.0).is_err());
    assert!(AffineErrorNorm::new(1, vec![], vec![], vec![1.0], 0.0).is_err());
    assert!(AffineErrorNorm::new(2, vec![0.0], vec![1.0], vec![1.0], 0.0).is_err());
    assert!(AffineErrorNorm::new(1, vec![0.0], vec![1.0], vec![], 0.0).is_err());
    assert!(AffineErrorNorm::new(1, vec![0.0], vec![1.0], vec![-1.0], 0.0).is_err());
    assert!(AffineErrorNorm::new(1, vec![0.0], vec![1.0], vec![1.0], f64::NAN).is_err());
    assert!(AffineErrorNorm::new(1, vec![f64::NAN], vec![1.0], vec![1.0], 0.0).is_err());
    assert!(AffineErrorNorm::new(1, vec![0.0], vec![f64::INFINITY], vec![1.0], 0.0).is_err());
    let norm = AffineErrorNorm::new(1, vec![0.0], vec![1.0], vec![1.0], 0.0).unwrap();
    assert!(norm.evaluate(&[], &[0.0], &[0.0]).is_err());
    assert!(norm.evaluate(&[0.0], &[], &[0.0]).is_err());
    assert!(norm.evaluate(&[0.0], &[0.0], &[]).is_err());
}

#[test]
fn successful_physical_norm_evaluations_allocate_no_heap_objects() {
    let norm = AffineErrorNorm::new(
        2,
        vec![0.0; 3],
        vec![1.0, 0.0, 0.0, 1.0, -1.0, -1.0],
        vec![1e-6],
        1e-4,
    )
    .unwrap();
    ALLOCATIONS.with(|count| count.set(0));
    ENABLED.with(|flag| flag.set(true));
    let mut last = 0.0;
    for _ in 0..100 {
        last = norm
            .evaluate(&[1.0, 2.0], &[1.1, 2.1], &[1e-7, -2e-7])
            .unwrap();
    }
    ENABLED.with(|flag| flag.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert!(last.is_finite() && last > 0.0);
}
