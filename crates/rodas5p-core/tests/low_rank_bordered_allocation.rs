use rodas5p_core::Preconditioner;
use rodas5p_core::low_rank_bordered::{LowRankApplyWorkspace, LowRankBorderedPreconditioner};
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
fn refactor_and_both_apply_paths_allocate_zero_across_active_shapes() {
    let mut pc = LowRankBorderedPreconditioner::new(32, 2, 3, 0, 0).unwrap();
    let mut workspace = LowRankApplyWorkspace::new(32, 2, 3).unwrap();
    let a = [4.0; 32];
    let b = [0.125; 64];
    let c = [-0.125; 64];
    let d = [2.0, 0.0, 0.0, 2.0];
    let u = [0.0625; 96];
    let v = [-0.03125; 96];
    let rhs = [1.0; 34];
    let mut out = [0.0; 34];
    TRACK.store(true, Ordering::SeqCst);
    for (n, m, r) in [(32, 2, 3), (8, 1, 2), (1, 0, 0), (17, 2, 1), (32, 2, 3)] {
        pc.refactor(
            n,
            m,
            r,
            &a[..n],
            &b[..n * m],
            &c[..n * m],
            &d[..m * m],
            &u[..n * r],
            &v[..n * r],
        )
        .unwrap();
        pc.apply(&rhs[..n + m], &mut out[..n + m]).unwrap();
        pc.apply_with_workspace(&rhs[..n + m], &mut out[..n + m], &mut workspace)
            .unwrap();
    }
    TRACK.store(false, Ordering::SeqCst);
    assert_eq!(COUNT.load(Ordering::Relaxed), 0);
}
