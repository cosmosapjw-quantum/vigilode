use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use vigilode_coupled::trace_certificate::{TraceNode, certify_trace_path};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

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
    let _guard = TEST_LOCK.lock().unwrap();
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

#[test]
fn rejected_strict_tube_charges_only_pre_and_post_loop_allocations() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut candidate = nodes(16);
    // The endpoint is inside the closed declared domain. Its zero boundary
    // margin forces a strict-tube refusal after original-field slab work.
    candidate[1].state = [0.5; 2];
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ACTIVE.store(true, Ordering::Relaxed);
    let result = certify_trace_path(16, &candidate);
    ACTIVE.store(false, Ordering::Relaxed);
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);

    assert_eq!(result.status, "rejected");
    assert!(
        result
            .failure
            .as_deref()
            .unwrap()
            .contains("strict original-domain tube")
    );
    assert_eq!(result.work.slabs_attempted, 1);
    assert_eq!(result.work.slabs_completed, 0);
    assert_eq!(result.work.slabs_rejected, 1);
    assert_eq!(result.work.endpoint_field_enclosures, 4);
    assert_eq!(result.work.provider.requests_attempted, 4);
    assert!(result.records.is_empty());
    assert_eq!(result.records.capacity(), 16);
    // One allocation reserves all 16 records before the loop. One allocation
    // formats the typed domain refusal after the loop. No slab allocation.
    assert_eq!(allocations, 2);
}
