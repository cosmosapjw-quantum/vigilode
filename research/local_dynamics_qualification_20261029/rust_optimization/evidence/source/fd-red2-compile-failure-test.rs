//! Allocation contract of the actual facade finite-difference source. This
//! test compiles `src/fd.rs` in a separate integration binary because the
//! library forbids unsafe code while a counting global allocator requires it.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::Arc,
};

mod error {
    pub use vigilode::CallbackError;
}
mod problem {
    pub use vigilode::RhsFn;
}
mod solution {
    pub use vigilode::Callback;

    // The public record types are non-exhaustive outside the library. The
    // local mirrors let the unchanged source module construct its records.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct FiniteDifferenceWork {
        pub jacobian_builds: u64,
        pub rhs_calls: u64,
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct CallbackWork {
        pub recoverable_errors: u64,
        pub fatal_errors: u64,
        pub short_circuited_calls: u64,
        pub short_circuited_rhs_calls: u64,
    }
}
#[path = "../src/fd.rs"]
mod fd;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

struct CountingAlloc;

fn count_allocation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
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

fn measured<T>(f: impl FnOnce() -> T) -> (T, usize) {
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let result = f();
    TRACK.with(|track| track.set(false));
    (result, COUNT.with(Cell::get))
}

fn rhs(n: usize) -> vigilode::RhsFn {
    Arc::new(move |_, y, out| {
        for i in 0..n {
            out[i] = -2.0 * y[i];
        }
        Ok(())
    })
}

#[test]
fn repeated_fd_builds_reuse_scratch_after_warmup() {
    let n = 7;
    let y = [0.1, -0.3, 0.6, 0.2, -0.8, 0.5, 1.1];
    let dense_guard = Arc::new(fd::Guard::default());
    let dense = fd::dense_fd_jacobian(dense_guard.clone(), rhs(n), n);
    dense(0.0, &y).unwrap();
    let (matrix, dense_allocations) = measured(|| dense(0.0, &y).unwrap());
    // The value-returning provider still returns one newly owned matrix.
    assert_eq!(dense_allocations, 1, "dense scratch was reallocated");
    assert_eq!(matrix.nrows(), n);
    assert_eq!(dense_guard.fd_work().jacobian_builds, 2);

    let band_guard = Arc::new(fd::Guard::default());
    let banded = fd::banded_fd_fill(band_guard.clone(), rhs(n), n, 1, 1);
    let mut band = [0.0; 21];
    banded(0.0, &y, &mut band).unwrap();
    let (_, band_allocations) = measured(|| banded(0.0, &y, &mut band).unwrap());
    assert_eq!(band_allocations, 0, "band scratch was reallocated");
    assert_eq!(band_guard.fd_work().jacobian_builds, 2);
    assert_eq!(band_guard.fd_work().rhs_calls, 8);
}
