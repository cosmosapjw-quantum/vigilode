use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use rodas5p_core::{DenseMatrix, DenseOperator, LinearOperator, ShiftedOperator};

struct CountingAllocator;

static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.load(Ordering::Relaxed) {
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
    TRACK.store(false, Ordering::SeqCst);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    TRACK.store(true, Ordering::SeqCst);
    let result = operation();
    TRACK.store(false, Ordering::SeqCst);
    (
        result,
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
    )
}

#[test]
fn shifted_operator_apply_is_allocation_free_after_construction() {
    let jacobian =
        DenseMatrix::from_rows(&[&[2.0, -1.0, 0.0], &[0.5, 3.0, 0.25], &[0.0, -0.75, 4.0]])
            .unwrap();
    let mass =
        DenseMatrix::from_rows(&[&[1.5, 0.1, 0.0], &[0.0, 1.25, 0.2], &[0.05, 0.0, 2.0]]).unwrap();
    let operator = ShiftedOperator::new(
        Some(mass),
        std::sync::Arc::new(DenseOperator::new(jacobian).unwrap()),
        0.01,
        0.25,
    )
    .unwrap();
    let x = [0.3, -0.2, 0.7];
    let mut y = [0.0; 3];

    operator.apply(&x, &mut y).unwrap();
    let (_, allocations, bytes) = count_allocations(|| {
        for _ in 0..128 {
            operator.apply(&x, &mut y).unwrap();
        }
    });

    assert_eq!(allocations, 0, "shifted apply allocated {bytes} bytes");
}
