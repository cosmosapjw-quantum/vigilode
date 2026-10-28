use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use vigilode_coupled::trace_certificate::{TraceNode, certify_trace_path};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ACTIVE.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn nodes(slabs: usize) -> Vec<TraceNode> {
    (0..=slabs)
        .map(|k| {
            let t = -1.0 + (2 * k) as f64 / slabs as f64;
            let t2 = t * t;
            let t4 = t2 * t2;
            let p = 0.75 + t / 8.0 + t4 * t / 64.0;
            TraceNode { t, state: [p; 2] }
        })
        .collect()
}

fn counted_certification(nodes: &[TraceNode], slabs: usize) -> (usize, &'static str) {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ACTIVE.store(true, Ordering::Relaxed);
    let result = certify_trace_path(slabs, nodes);
    ACTIVE.store(false, Ordering::Relaxed);
    (ALLOCATIONS.load(Ordering::Relaxed), result.status)
}

#[test]
fn successful_slab_arithmetic_allocates_no_heap_objects() {
    let small = nodes(16);
    let large = nodes(256);
    let (small_allocations, small_status) = counted_certification(&small, 16);
    let (large_allocations, large_status) = counted_certification(&large, 256);
    assert_eq!(small_status, "completed");
    assert_eq!(large_status, "completed");
    // Each call reserves its complete output array once before the slab loop.
    assert_eq!(small_allocations, 1);
    assert_eq!(large_allocations, 1);
}
