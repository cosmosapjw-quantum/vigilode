//! Allocation counters.
//!
//! The `vigilode-bench` binary installs two counters (see `src/main.rs`):
//! a counting `#[global_allocator]` around `System` (Rust allocations), and
//! an interposer on the C allocator entry points (`malloc`, `calloc`,
//! `realloc`, `posix_memalign`, `aligned_alloc`, `memalign`) that forwards to
//! glibc's `__libc_*` and counts every heap allocation call of the process,
//! including the Fortran `ALLOCATE`s and SUNDIALS' `malloc`s (and, through
//! `System`, the Rust ones). Library tests and other binaries leave both
//! counters at zero and report `measured = false`.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Rust global-allocator calls (alloc, alloc_zeroed, realloc).
pub static RUST_CALLS: AtomicU64 = AtomicU64::new(0);
/// Bytes requested from the Rust global allocator.
pub static RUST_BYTES: AtomicU64 = AtomicU64::new(0);
/// C heap allocation calls (all languages).
pub static HEAP_CALLS: AtomicU64 = AtomicU64::new(0);
/// Bytes requested through the C heap entry points.
pub static HEAP_BYTES: AtomicU64 = AtomicU64::new(0);
/// Set by the bench binary once it has verified both counters move.
pub static MEASURED: AtomicBool = AtomicBool::new(false);

/// A reading of the counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocCounts {
    /// Every heap allocation call in the process (C interposer).
    pub heap_calls: u64,
    /// Bytes requested by those calls.
    pub heap_bytes: u64,
    /// The subset made by Rust code through the global allocator.
    pub rust_calls: u64,
    /// Bytes requested by Rust code.
    pub rust_bytes: u64,
}

/// Read the counters.
pub fn snapshot() -> AllocCounts {
    AllocCounts {
        heap_calls: HEAP_CALLS.load(Ordering::SeqCst),
        heap_bytes: HEAP_BYTES.load(Ordering::SeqCst),
        rust_calls: RUST_CALLS.load(Ordering::SeqCst),
        rust_bytes: RUST_BYTES.load(Ordering::SeqCst),
    }
}

impl AllocCounts {
    /// `self - earlier`.
    pub fn since(self, earlier: AllocCounts) -> AllocCounts {
        AllocCounts {
            heap_calls: self.heap_calls - earlier.heap_calls,
            heap_bytes: self.heap_bytes - earlier.heap_bytes,
            rust_calls: self.rust_calls - earlier.rust_calls,
            rust_bytes: self.rust_bytes - earlier.rust_bytes,
        }
    }
}

/// The end of a measured window: wall clock and allocation counters, taken
/// by an arm once its grid states are final and the solver is torn down,
/// before it formats statistics (which is not solver work).
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    /// Wall clock.
    pub at: std::time::Instant,
    /// Counters.
    pub alloc: AllocCounts,
}

impl Mark {
    /// Take a mark now.
    pub fn now() -> Self {
        let alloc = snapshot();
        Self {
            at: std::time::Instant::now(),
            alloc,
        }
    }
}

/// Whether the counters were verified live in this process.
pub fn measured() -> bool {
    MEASURED.load(Ordering::SeqCst)
}
