use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use rodas5p_krylov::SecantPreconditioner;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;

fn count_allocation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
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

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn allocation_count<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    COUNT.with(|c| c.set(0));
    TRACK.with(|t| t.set(true));
    let result = operation();
    TRACK.with(|t| t.set(false));
    let count = COUNT.with(|c| c.get());
    (result, count)
}

#[test]
fn secant_apply_into_is_zero_allocation_hot_loop() {
    let n = 16;
    let k = 4;
    let h_gamma = 0.21193756319429014;

    let s_displacements: Vec<f64> = (0..n * k).map(|i| (i as f64 * 0.41).sin()).collect();
    let y_residuals: Vec<f64> = (0..n * k).map(|i| (i as f64 * 0.53).cos()).collect();

    let pc = SecantPreconditioner::build(&s_displacements, &y_residuals, n, k, h_gamma)
        .expect("build valid secant preconditioner");

    let r: Vec<f64> = (0..n).map(|i| (i as f64 * 0.17 + 0.1).sin()).collect();
    let mut out = vec![0.0; n];

    // Warm up
    pc.apply_into(&r, &mut out, h_gamma);

    // Verify zero allocations during hot loop application
    let (_, allocs) = allocation_count(|| {
        for _ in 0..100 {
            pc.apply_into(&r, &mut out, h_gamma);
        }
    });

    assert_eq!(
        allocs, 0,
        "VG-1 violation: apply_into must perform exactly 0 heap allocations, got {allocs}"
    );
}
