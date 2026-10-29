use rodas5p_core::WorkCounters;
use rodas5p_integrators::Dopri5Workspace;
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

#[test]
fn successful_workspace_trials_allocate_zero_heap_objects() {
    let n = 257;
    let mut ws = Dopri5Workspace::new(n).unwrap();
    let y = vec![1.0; n];
    let mut endpoint = vec![0.0; n];
    let mut error = vec![0.0; n];
    let mut work = WorkCounters::default();
    let rhs = |_: f64, y: &[f64], out: &mut [f64]| {
        for (out, y) in out.iter_mut().zip(y) {
            *out = -*y;
        }
        Ok(())
    };
    ALLOCATIONS.with(|count| count.set(0));
    ENABLED.with(|flag| flag.set(true));
    for k in 0..25 {
        ws.step_into(
            &rhs,
            k as f64 * 0.01,
            &y,
            0.01,
            &mut endpoint,
            &mut error,
            &mut work,
        )
        .unwrap();
    }
    ENABLED.with(|flag| flag.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(work.rhs_calls, 175);
}
