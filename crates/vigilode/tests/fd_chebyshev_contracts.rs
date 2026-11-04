use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use vigilode::{CallbackError, jvp_chebyshev4_guarded, jvp_chebyshev4_guarded_with_delta};

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

#[test]
fn test_chebyshev4_convergence_order_smooth() {
    // Test on smooth function f(x) = exp(x)
    let f = |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = y[0].exp();
        Ok(())
    };

    let y = [1.0];
    let v = [1.0];
    let exact = 1.0f64.exp();

    let mut out = [0.0];
    let mut scratch_y = [0.0];
    let mut scratch_f = [0.0; 4];

    let deltas = [0.08, 0.04, 0.02];
    let mut errors = [0.0; 3];

    for (k, &delta) in deltas.iter().enumerate() {
        jvp_chebyshev4_guarded_with_delta(
            &f,
            0.0,
            &y,
            &v,
            delta,
            &mut out,
            &mut scratch_y,
            &mut scratch_f,
        )
        .expect("chebyshev4 evaluation should succeed on smooth function");
        errors[k] = (out[0] - exact).abs();
    }

    let order1 = (errors[0] / errors[1]).log2();
    let order2 = (errors[1] / errors[2]).log2();

    assert!(
        order1 >= 3.8,
        "Convergence order {order1} should be >= 3.8 (expected ~ 4.0)"
    );
    assert!(
        order2 >= 3.8,
        "Convergence order {order2} should be >= 3.8 (expected ~ 4.0)"
    );
}

#[test]
fn test_chebyshev4_kink_detection() {
    // Function with kink at x = 0: f(x) = |x|
    let f_kink = |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = y[0].abs();
        Ok(())
    };

    let y = [0.0];
    let v = [1.0];
    let mut out = [0.0];
    let mut scratch_y = [0.0];
    let mut scratch_f = [0.0; 4];

    let res = jvp_chebyshev4_guarded(
        &f_kink,
        0.0,
        &y,
        &v,
        &mut out,
        &mut scratch_y,
        &mut scratch_f,
    );

    match res {
        Err(CallbackError::Recoverable(msg)) => {
            assert!(
                msg.contains("Chebyshev4 non-smoothness or kink detected"),
                "Expected non-smoothness or kink error message, got: {msg}"
            );
        }
        other => panic!("Expected Err(CallbackError::Recoverable), got {other:?}"),
    }

    // Also test step discontinuity: f(x) = 1.0 if x >= 0.0 else 0.0
    let f_step = |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = if y[0] >= 0.0 { 1.0 } else { 0.0 };
        Ok(())
    };

    let res_step = jvp_chebyshev4_guarded(
        &f_step,
        0.0,
        &y,
        &v,
        &mut out,
        &mut scratch_y,
        &mut scratch_f,
    );

    match res_step {
        Err(CallbackError::Recoverable(msg)) => {
            assert!(
                msg.contains("Chebyshev4 non-smoothness or kink detected"),
                "Expected non-smoothness or kink error message for step, got: {msg}"
            );
        }
        other => panic!("Expected Err(CallbackError::Recoverable), got {other:?}"),
    }
}

#[test]
fn test_chebyshev4_zero_allocation() {
    let f = |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = y[0].sin();
        out[1] = y[1].cos();
        Ok(())
    };

    let y = [0.5, -0.3];
    let v = [1.0, 0.5];
    let mut out = [0.0; 2];
    let mut scratch_y = [0.0; 2];
    let mut scratch_f = [0.0; 8];

    let (res, allocs) = allocation_scope(|| {
        jvp_chebyshev4_guarded(&f, 0.0, &y, &v, &mut out, &mut scratch_y, &mut scratch_f)
    });

    assert!(res.is_ok(), "guarded evaluation should succeed");
    assert_eq!(
        allocs, 0,
        "jvp_chebyshev4_guarded must not allocate heap memory"
    );
}
