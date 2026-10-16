//! Research node SP02 (`research/sp02_krylov_workspace_caller_20261011`):
//! charge contract for least-squares, GMRES, and LGMRES-into workspaces.
//!
//! Verifies that retained bytes charged by the workspace are never below
//! the live heap bytes released on drop, and exceed them by at most 128 bytes
//! per faer-owned buffer (alignment padding allowance).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{DenseMatrix, IdentityPreconditioner, LinearOperator, WorkCounters};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresIntoOptions, GmresWorkspace, LeastSquaresWorkspace,
    LgmresConfig, LgmresIntoWorkspace, LgmresState, ResidualAccounting,
    solve_gmres_into_with_accounting, solve_lgmres_into,
};

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        LIVE_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        LIVE_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        if new_size >= layout.size() {
            LIVE_BYTES.fetch_add(new_size - layout.size(), Ordering::SeqCst);
        } else {
            LIVE_BYTES.fetch_sub(layout.size() - new_size, Ordering::SeqCst);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn live_bytes() -> usize {
    LIVE_BYTES.load(Ordering::SeqCst)
}

struct TestOperator {
    mat: DenseMatrix,
}

impl LinearOperator for TestOperator {
    fn apply(&self, x: &[f64], y: &mut [f64]) -> rodas5p_core::CoreResult<()> {
        self.mat.matvec_into(x, y)
    }
    fn dimension(&self) -> usize {
        self.mat.nrows()
    }
    fn exact_identity(&self) -> Option<rodas5p_core::ExactOperatorIdentity> {
        None
    }
    fn token(&self) -> u64 {
        1
    }
}

fn test_matrix(m: usize, n: usize) -> DenseMatrix {
    let mut mat = DenseMatrix::zeros(m, n);
    for i in 0..m {
        for j in 0..n {
            mat[(i, j)] = 1.0 / (1.0 + (i + j) as f64) + if i == j { 3.0 } else { 0.0 };
        }
    }
    mat
}

#[test]
fn least_squares_workspace_charge_contract() {
    let mut ws = LeastSquaresWorkspace::new();
    let shapes = [(10, 4), (25, 8), (15, 5), (32, 10), (12, 3)];
    let mut out = Vec::new();

    for &(m, n) in &shapes {
        let a = test_matrix(m, n);
        let b: Vec<f64> = (0..m).map(|i| 1.0 + (i as f64) * 0.1).collect();
        ws.solve_into(&a, &b, &mut out).unwrap();
        assert_eq!(out.len(), n);
    }
    drop(out);

    let charged = ws.retained_bytes();
    let before = live_bytes();
    drop(ws);
    let after = live_bytes();
    let released = before
        .checked_sub(after)
        .expect("underflow in released bytes");

    assert!(
        charged >= released,
        "charged {charged} < released {released}"
    );
    assert!(
        charged - released <= 128 * 5,
        "charged {charged} - released {released} = {} > 640 (allowance for 5 faer buffers)",
        charged - released
    );
}

#[test]
fn gmres_workspace_charge_contract() {
    let mut ws = GmresWorkspace::with_ls_workspace();
    let config = GmresConfig {
        restart: 15,
        max_arnoldi: 30,
        rtol: 1e-10,
        atol: 1e-12,
    };
    let dims = [10, 25, 15, 30, 12];

    for &dim in &dims {
        let mat = test_matrix(dim, dim);
        let op = TestOperator { mat };
        let pc = IdentityPreconditioner::new(dim);
        let rhs: Vec<f64> = (0..dim).map(|i| 1.0 + 0.1 * i as f64).collect();
        let mut x = vec![0.0; dim];
        let mut counters = WorkCounters::default();

        solve_gmres_into_with_accounting(
            &op,
            &pc,
            &rhs,
            None,
            &config,
            None,
            &mut x,
            &mut ws,
            GmresCapacity::unbounded(),
            GmresIntoOptions::default(),
            ResidualAccounting::RecomputeFinal,
            &mut counters,
        )
        .unwrap();
    }

    let charged = ws.retained_bytes();
    let before = live_bytes();
    drop(ws);
    let after = live_bytes();
    let released = before
        .checked_sub(after)
        .expect("underflow in released bytes");

    assert!(
        charged >= released,
        "charged {charged} < released {released}"
    );
    assert!(
        charged - released <= 128 * 5,
        "charged {charged} - released {released} = {} > 640",
        charged - released
    );
}

#[test]
fn lgmres_into_workspace_charge_contract() {
    let mut ws = LgmresIntoWorkspace::default();
    ws.set_least_squares_once(true);
    ws.set_ls_workspace(true);

    let config = LgmresConfig {
        inner_m: 10,
        max_outer: 5,
        outer_k: 3,
        rtol: 1e-10,
        atol: 1e-12,
    };
    let dims = [10, 25, 15, 30, 12];
    let mut state = LgmresState::default();

    for &dim in &dims {
        let mat = test_matrix(dim, dim);
        let op = TestOperator { mat };
        let pc = IdentityPreconditioner::new(dim);
        let rhs: Vec<f64> = (0..dim).map(|i| 1.0 + 0.1 * i as f64).collect();
        let mut x = vec![0.0; dim];
        let mut counters = WorkCounters::default();

        solve_lgmres_into(
            &op,
            &pc,
            &rhs,
            None,
            &config,
            &mut state,
            None,
            &mut x,
            &mut ws,
            &mut counters,
        )
        .unwrap();
    }
    drop(state);

    let charged = ws.retained_bytes();
    let before = live_bytes();
    drop(ws);
    let after = live_bytes();
    let released = before
        .checked_sub(after)
        .expect("underflow in released bytes");

    assert!(
        charged >= released,
        "charged {charged} < released {released}"
    );
    assert!(
        charged - released <= 128 * 5,
        "charged {charged} - released {released} = {} > 640",
        charged - released
    );
}
