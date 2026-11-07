use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::atomic::{AtomicU64, Ordering},
};

use rodas5p_core::{DenseMatrix, DenseOperator, IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{
    GmresConfig, GmresGivensWorkspace, GmresWorkspace, solve_gmres, solve_gmres_givens,
    solve_gmres_givens_with_workspace, solve_gmres_with_workspace,
};

struct CountingAllocator;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
}
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.try_with(|t| t.get()).unwrap_or(false) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn allocation_count<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    let _ = TRACK.try_with(|t| t.set(false));
    ALLOCATIONS.store(0, Ordering::Relaxed);
    let _ = TRACK.try_with(|t| t.set(true));
    let result = operation();
    let _ = TRACK.try_with(|t| t.set(false));
    (result, ALLOCATIONS.load(Ordering::Relaxed))
}

fn system(n: usize) -> (DenseOperator, IdentityPreconditioner, Vec<f64>) {
    let mut matrix = DenseMatrix::zeros(n, n);
    for i in 0..n {
        matrix[(i, i)] = 4.0 + i as f64 / n as f64;
        if i + 1 < n {
            matrix[(i, i + 1)] = 0.35;
        }
        if i > 0 {
            matrix[(i, i - 1)] = -0.05;
        }
    }
    let oracle: Vec<f64> = (0..n).map(|i| ((i + 1) as f64 * 0.17).sin()).collect();
    let rhs = matrix.matvec(&oracle).unwrap();
    (
        DenseOperator::new(matrix).unwrap(),
        IdentityPreconditioner::new(n),
        rhs,
    )
}

#[test]
fn warmed_gmres_workspace_reduces_heap_allocations_without_changing_operator_work() {
    let (operator, preconditioner, rhs) = system(32);
    let config = GmresConfig {
        restart: 12,
        max_arnoldi: 96,
        rtol: 1e-11,
        atol: 1e-13,
    };
    let mut workspace = GmresWorkspace::default();
    let mut warm_work = WorkCounters::default();
    solve_gmres_with_workspace(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &config,
        &mut workspace,
        &mut warm_work,
    )
    .unwrap();

    let ((workspace_report, workspace_work), workspace_allocations) = allocation_count(|| {
        let mut work = WorkCounters::default();
        let report = solve_gmres_with_workspace(
            &operator,
            &preconditioner,
            &rhs,
            None,
            &config,
            &mut workspace,
            &mut work,
        )
        .unwrap();
        (report, work)
    });
    let ((wrapper_report, wrapper_work), wrapper_allocations) = allocation_count(|| {
        let mut work = WorkCounters::default();
        let report =
            solve_gmres(&operator, &preconditioner, &rhs, None, &config, &mut work).unwrap();
        (report, work)
    });

    assert_eq!(workspace_work.linear_matvecs, wrapper_work.linear_matvecs);
    assert_eq!(
        workspace_work.diagnostic_matvecs,
        wrapper_work.diagnostic_matvecs
    );
    assert_eq!(workspace_report.x, wrapper_report.x);
    assert!(
        workspace_allocations < wrapper_allocations,
        "workspace allocations {workspace_allocations} >= wrapper allocations {wrapper_allocations}"
    );
}

#[test]
fn warmed_gmres_givens_workspace_reduces_heap_allocations() {
    let (operator, preconditioner, rhs) = system(32);
    let config = GmresConfig {
        restart: 12,
        max_arnoldi: 96,
        rtol: 1e-11,
        atol: 1e-13,
    };
    let mut workspace = GmresGivensWorkspace::default();
    let mut warm_work = WorkCounters::default();
    solve_gmres_givens_with_workspace(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &config,
        &mut workspace,
        &mut warm_work,
    )
    .unwrap();

    let ((workspace_report, workspace_work), workspace_allocations) = allocation_count(|| {
        let mut work = WorkCounters::default();
        let report = solve_gmres_givens_with_workspace(
            &operator,
            &preconditioner,
            &rhs,
            None,
            &config,
            &mut workspace,
            &mut work,
        )
        .unwrap();
        (report, work)
    });
    let ((wrapper_report, wrapper_work), wrapper_allocations) = allocation_count(|| {
        let mut work = WorkCounters::default();
        let report =
            solve_gmres_givens(&operator, &preconditioner, &rhs, None, &config, &mut work).unwrap();
        (report, work)
    });

    assert_eq!(workspace_work.linear_matvecs, wrapper_work.linear_matvecs);
    assert_eq!(workspace_report.x, wrapper_report.x);
    assert!(
        workspace_allocations < wrapper_allocations,
        "workspace allocations {workspace_allocations} >= wrapper allocations {wrapper_allocations}"
    );
}
