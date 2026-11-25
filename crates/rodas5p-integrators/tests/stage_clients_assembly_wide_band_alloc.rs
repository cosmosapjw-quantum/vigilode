//! A sequential client whose default first-use check became the full check
//! (loop node V31, owner decision OD-L1-27: `CheckColumns::All` where
//! `CheckColourings::total_groups` of the declared pattern is at least `n`)
//! allocates nothing for its check at the first linearization: the full
//! check works in the client's own scratch, allocated by
//! `AssemblyStageClient::new`. This binary holds exactly one test: the
//! allocation counter is process-global, so a second test running beside it
//! would be counted.
//!
//! As in `stage_clients_assembly_default_alloc.rs` (node V29), the first
//! linearization of a fresh client with the default check is compared with
//! the first linearization of a fresh client of the same configuration
//! without a check (the back end may allocate at its first use; both pay
//! that), and later linearizations with every check done are compared the
//! same way. A multi-colouring check requested through the handle of such a
//! client builds its colourings at that linearization, which the test shows
//! as the counter-example; a narrow control keeps the multi-colouring default
//! and allocates nothing for it either.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::{CoreResult, WorkCounters};
use rodas5p_integrators::StageLinearSystem;
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyOptions, AssemblyStageClient, CheckColourings, CheckColumns, LinearBackend,
    SparsityPattern,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{FdProblem, Rhs, from_rhs, nlkpp_band, ring, ring_pattern};

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

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The wide random pattern of `stage_clients_assembly_wide_band.rs`: order
/// 60, half-width 27, density 0.1, seed 1 (`total_groups` 111).
fn wide_rows() -> Vec<Vec<usize>> {
    let (n, half_width, density, seed) = (60_usize, 27_usize, 0.1, 1_u64);
    let uniform = |i: usize, j: usize| {
        let key = ((i as u64) << 32) | j as u64;
        (splitmix64(seed ^ splitmix64(key)) >> 11) as f64 / (1_u64 << 53) as f64
    };
    (0..n)
        .map(|i| {
            (i.saturating_sub(half_width)..=(i + half_width).min(n - 1))
                .filter(|&j| j == i || uniform(i, j) < density)
                .collect()
        })
        .collect()
}

/// Row `i` is the sum over the columns `rows[i]` of `w_ij y_j (1 + y_j / 10)`.
fn generic_problem(id: &str, rows: &[Vec<usize>]) -> FdProblem {
    let n = rows.len();
    let weights: Vec<Vec<(usize, f64)>> = rows
        .iter()
        .enumerate()
        .map(|(i, columns)| {
            columns
                .iter()
                .map(|&j| (j, 0.5 + 0.37 * ((3 * i + 7 * j) % 11) as f64 / 11.0))
                .collect()
        })
        .collect();
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            for (value, columns) in out.iter_mut().zip(&weights) {
                let mut acc = 0.0;
                for &(j, w) in columns {
                    acc += w * y[j] * (1.0 + 0.1 * y[j]);
                }
                *value = acc;
            }
            Ok(())
        },
    );
    let y0 = (0..n)
        .map(|j| 0.75 + 0.25 * (0.61 * j as f64 + 0.3).sin())
        .collect();
    from_rhs(id.into(), rhs, y0, (0.0, 1.0), None)
}

/// Allocations of three linearizations of a fresh client (the first, and
/// two later ones at other states), with `request` asked for through the
/// handle before the first, and the checks done.
fn linearizations(
    fd: &FdProblem,
    backend: LinearBackend,
    options: AssemblyOptions,
    request: Option<CheckColumns>,
) -> ([u64; 3], u64) {
    let mut client = AssemblyStageClient::new(fd.n, backend, options).unwrap();
    let handle = client.handle();
    let mut counters = WorkCounters::default();
    if let Some(check) = request {
        handle.request_check(check);
    }
    let later: Vec<f64> = fd.y0.iter().map(|y| y * 1.01 + 0.002).collect();
    let mut during = [0; 3];
    for (step, slot) in during.iter_mut().enumerate() {
        let y = if step == 1 { &later } else { &fd.y0 };
        let before = allocations();
        client
            .linearize(&fd.problem, 0.125 * step as f64, y, &mut counters)
            .unwrap();
        *slot = allocations() - before;
    }
    (during, handle.stats().verifications)
}

/// A client configuration: problem, declared pattern, back end, name and
/// the expected default check.
type Case<'a> = (
    &'a FdProblem,
    SparsityPattern,
    LinearBackend,
    &'a str,
    CheckColumns,
);

#[test]
fn a_full_default_check_allocates_nothing_at_the_first_linearization() {
    let in_place = LinearBackend::DenseLu(DenseLuMode::InPlace);
    let rows = wide_rows();
    let wide = generic_problem("wide-n60", &rows);
    let ring30 = ring(30, false, false);
    let ring34 = ring(34, false, false);
    let band60 = nlkpp_band(60, 15, false);
    let band = |n, lower, upper| SparsityPattern::banded(n, lower, upper).unwrap();
    let cases: Vec<Case<'_>> = vec![
        (
            &wide,
            SparsityPattern::from_rows(60, &rows).unwrap(),
            in_place,
            "wide random band (total_groups 111 > 60), dense LU",
            CheckColumns::All,
        ),
        (
            &ring30,
            ring_pattern(30),
            in_place,
            "ring 30 (total_groups 30 = n), dense LU",
            CheckColumns::All,
        ),
        (
            &band60,
            band(60, 15, 15),
            LinearBackend::BandedTight {
                lower: 15,
                upper: 15,
            },
            "band (15, 15) of order 60 (total_groups 60 = n), tight kernel",
            CheckColumns::All,
        ),
        (
            &band60,
            band(60, 15, 15),
            LinearBackend::BandedProtected {
                lower: 15,
                upper: 15,
            },
            "band (15, 15) of order 60 (total_groups 60 = n), protected kernel",
            CheckColumns::All,
        ),
        (
            &ring34,
            ring_pattern(34),
            in_place,
            "control: ring 34 (total_groups 33 < n), dense LU",
            CheckColumns::MultiColouring,
        ),
    ];
    for (fd, pattern, backend, name, expected) in &cases {
        let total = CheckColourings::of(pattern).unwrap().total_groups();
        assert_eq!(*expected == CheckColumns::All, total >= fd.n, "{name}");
        let default = AssemblyOptions::coloured(pattern.clone());
        assert_eq!(default.first_use_check, *expected, "{name}");
        let unchecked = default.clone().with_first_use_check(CheckColumns::None);
        // Warm the lazy statics of the back end and of the driver code.
        linearizations(fd, *backend, default.clone(), None);
        linearizations(fd, *backend, unchecked.clone(), None);
        let (with_check, checks) = linearizations(fd, *backend, default.clone(), None);
        let (without_check, none) = linearizations(fd, *backend, unchecked, None);
        let (requested, requested_checks) = linearizations(
            fd,
            *backend,
            default.clone(),
            Some(CheckColumns::MultiColouring),
        );
        println!(
            "STAGE_CLIENTS_ASSEMBLY_WIDE_BAND_ALLOC_JSON {}",
            serde_json::json!({
                "case": fd.id,
                "client": name,
                "default_check": format!("{:?}", default.first_use_check),
                "total_groups": total,
                "n": fd.n,
                "allocations_default_check": with_check,
                "allocations_without_check": without_check,
                "allocations_requested_multi_colouring": requested,
            })
        );
        assert_eq!((checks, none, requested_checks), (1, 0, 1), "{name}");
        assert_eq!(
            with_check, without_check,
            "{name}: the default check allocated at a linearization"
        );
        if *expected == CheckColumns::All {
            // The counter-example: a client whose default is the full check
            // has no further colourings; a multi-colouring check requested
            // through the handle builds them at that linearization.
            assert!(
                requested[0] > without_check[0],
                "{name}: {requested:?} against {without_check:?}"
            );
        }
    }
}
