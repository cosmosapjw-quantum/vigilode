//! The default first-use check of the coloured route, the multi-colouring
//! check (loop node V29), allocates nothing at the first linearization of a
//! sequential client: its further colourings are built by
//! `AssemblyStageClient::new`. This binary holds exactly one test: the
//! allocation counter is process-global, so a second test running beside it
//! would be counted.
//!
//! The first linearization of a fresh client with the default check is
//! compared with the first linearization of a fresh client of the same
//! configuration without a check (the back end may allocate at its first
//! use; both pay that). A check requested later through the handle builds
//! its colourings at that linearization, which the test shows as the
//! counter-example.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::WorkCounters;
use rodas5p_integrators::StageLinearSystem;
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyOptions, AssemblyStageClient, CheckColumns, LinearBackend, SparsityPattern,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{FdProblem, brusselator, nlkpp_band, ring, ring_pattern};

struct Counting;

/// Allocation requests of the whole process (alloc, alloc_zeroed, realloc).
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations() -> u64 {
    ALLOCATIONS.load(Ordering::Relaxed)
}

/// Allocations of the first linearization of a fresh client, and whether a
/// check ran.
fn first_linearization(
    fd: &FdProblem,
    backend: LinearBackend,
    options: AssemblyOptions,
    request: Option<CheckColumns>,
) -> (u64, u64) {
    let mut client = AssemblyStageClient::new(fd.n, backend, options).unwrap();
    let handle = client.handle();
    let mut counters = WorkCounters::default();
    if let Some(check) = request {
        handle.request_check(check);
    }
    let before = allocations();
    client
        .linearize(&fd.problem, fd.span.0, &fd.y0, &mut counters)
        .unwrap();
    let during = allocations() - before;
    (during, handle.stats().verifications)
}

#[test]
fn the_default_check_allocates_nothing_at_the_first_linearization() {
    let in_place = LinearBackend::DenseLu(DenseLuMode::InPlace);
    let band = |fd: &FdProblem| {
        let (lower, upper) = fd.band.unwrap();
        SparsityPattern::banded(fd.n, lower, upper).unwrap()
    };
    let bruss = brusselator(16, false);
    let wide = nlkpp_band(96, 4, false);
    let ring30 = ring(30, false, false);
    let cases: Vec<(&FdProblem, SparsityPattern, LinearBackend, &str)> = vec![
        (
            &ring30,
            ring_pattern(30),
            in_place,
            "ring pattern, dense LU",
        ),
        (&bruss, band(&bruss), in_place, "band (2, 2), dense LU"),
        (
            &bruss,
            band(&bruss),
            LinearBackend::BandedProtected { lower: 2, upper: 2 },
            "band (2, 2), protected kernel",
        ),
        (
            &wide,
            band(&wide),
            LinearBackend::BandedTight { lower: 4, upper: 4 },
            "band (4, 4), tight kernel",
        ),
    ];
    for (fd, pattern, backend, name) in &cases {
        let default = AssemblyOptions::coloured(pattern.clone());
        assert_eq!(
            default.first_use_check,
            if pattern == &ring_pattern(30) {
                CheckColumns::All
            } else {
                CheckColumns::MultiColouring
            },
            "{name}"
        );
        let unchecked = default.clone().with_first_use_check(CheckColumns::None);
        // Warm the lazy statics of the back end and of the driver code.
        first_linearization(fd, *backend, default.clone(), None);
        first_linearization(fd, *backend, unchecked.clone(), None);
        let (with_check, checks) = first_linearization(fd, *backend, default.clone(), None);
        let (without_check, none) = first_linearization(fd, *backend, unchecked.clone(), None);
        let (requested, requested_checks) =
            first_linearization(fd, *backend, unchecked, Some(CheckColumns::MultiColouring));
        println!(
            "STAGE_CLIENTS_ASSEMBLY_DEFAULT_ALLOC_JSON {}",
            serde_json::json!({
                "case": fd.id,
                "client": name,
                "first_linearization_allocations_default_check": with_check,
                "first_linearization_allocations_without_check": without_check,
                "first_linearization_allocations_requested_multi_colouring": requested,
            })
        );
        assert_eq!((checks, none, requested_checks), (1, 0, 1), "{name}");
        assert_eq!(
            with_check, without_check,
            "{name}: the default check allocated at the first linearization"
        );
        // The counter-example: a multi-colouring check requested through the
        // handle builds its colourings at that linearization.
        assert!(requested > without_check, "{name}: {requested}");
    }
}
