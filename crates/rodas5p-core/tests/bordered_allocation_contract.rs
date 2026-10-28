use rodas5p_core::Preconditioner;
use rodas5p_core::bordered::BorderedBandedPreconditioner;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Counter;
static TRACK: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicUsize = AtomicUsize::new(0);
#[global_allocator]
static ALLOC: Counter = Counter;
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if TRACK.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, n: usize) -> *mut u8 {
        if TRACK.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(p, layout, n) }
    }
}

#[test]
fn successful_refactor_and_apply_allocate_zero_for_variable_active_dimension() {
    let mut pc = BorderedBandedPreconditioner::new(32, 2, 0, 0).unwrap();
    let a = [4.0; 32];
    let b = [0.125; 64];
    let c = [-0.125; 64];
    let d = [2.0, 0.0, 0.0, 2.0];
    let rhs = [1.0; 34];
    let mut out = [0.0; 34];
    TRACK.store(true, Ordering::SeqCst);
    for (n, m) in [(32, 2), (8, 1), (1, 0), (17, 2), (32, 2)] {
        pc.refactor(n, m, &a[..n], &b[..n * m], &c[..n * m], &d[..m * m])
            .unwrap();
        pc.apply(&rhs[..n + m], &mut out[..n + m]).unwrap();
    }
    TRACK.store(false, Ordering::SeqCst);
    assert_eq!(COUNT.load(Ordering::Relaxed), 0);
}
