use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use rodas5p_core::{DenseMatrix, DenseOperator, IdentityPreconditioner, WorkCounters, safe_l2};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GcrodrWorkspace, GmresConfig, GmresWorkspace, LgmresConfig,
    LgmresState, LgmresWorkspace, solve_gcrodr, solve_gcrodr_with_workspace, solve_gmres,
    solve_gmres_with_workspace, solve_lgmres, solve_lgmres_with_workspace,
};

struct CountingAllocator;
static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn allocation_count<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    TRACK.store(false, Ordering::SeqCst);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    TRACK.store(true, Ordering::SeqCst);
    let result = operation();
    TRACK.store(false, Ordering::SeqCst);
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

fn relative_difference(left: &[f64], right: &[f64]) -> f64 {
    let difference: Vec<f64> = left.iter().zip(right).map(|(a, b)| a - b).collect();
    safe_l2(&difference) / safe_l2(right).max(f64::MIN_POSITIVE)
}

#[test]
fn reusable_workspaces_preserve_all_three_solver_contracts() {
    let (operator, preconditioner, rhs) = system(24);

    let gmres_config = GmresConfig {
        restart: 10,
        max_arnoldi: 80,
        rtol: 1e-11,
        atol: 1e-13,
    };
    let mut wrapper_work = WorkCounters::default();
    let wrapper = solve_gmres(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gmres_config,
        &mut wrapper_work,
    )
    .unwrap();
    let mut workspace_work = WorkCounters::default();
    let workspace = solve_gmres_with_workspace(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gmres_config,
        &mut GmresWorkspace::default(),
        &mut workspace_work,
    )
    .unwrap();
    assert!(relative_difference(&workspace.x, &wrapper.x) < 1e-13);
    assert_eq!(workspace_work.linear_matvecs, wrapper_work.linear_matvecs);
    assert_eq!(
        workspace_work.diagnostic_matvecs,
        wrapper_work.diagnostic_matvecs
    );

    let lgmres_config = LgmresConfig {
        inner_m: 8,
        max_outer: 10,
        outer_k: 3,
        rtol: 1e-11,
        atol: 1e-13,
    };
    let mut wrapper_state = LgmresState::default();
    let mut wrapper_work = WorkCounters::default();
    let wrapper = solve_lgmres(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &lgmres_config,
        &mut wrapper_state,
        &mut wrapper_work,
    )
    .unwrap();
    let mut workspace_state = LgmresState::default();
    let mut workspace_work = WorkCounters::default();
    let workspace = solve_lgmres_with_workspace(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &lgmres_config,
        &mut workspace_state,
        &mut LgmresWorkspace::default(),
        &mut workspace_work,
    )
    .unwrap();
    assert!(relative_difference(&workspace.x, &wrapper.x) < 1e-13);
    assert_eq!(workspace_work.linear_matvecs, wrapper_work.linear_matvecs);
    assert_eq!(workspace_state.generation, wrapper_state.generation);

    let gcrodr_config = GcrodrConfig {
        restart: 10,
        max_arnoldi: 80,
        recycle_dim: 3,
        rank_tol: 1e-12,
        rtol: 1e-11,
        atol: 1e-13,
    };
    let mut wrapper_state = GcrodrState::default();
    let mut wrapper_work = WorkCounters::default();
    let wrapper = solve_gcrodr(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gcrodr_config,
        &mut wrapper_state,
        &mut wrapper_work,
    )
    .unwrap();
    let mut workspace_state = GcrodrState::default();
    let mut workspace_work = WorkCounters::default();
    let workspace = solve_gcrodr_with_workspace(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gcrodr_config,
        &mut workspace_state,
        &mut GcrodrWorkspace::default(),
        &mut workspace_work,
    )
    .unwrap();
    assert!(relative_difference(&workspace.x, &wrapper.x) < 1e-13);
    assert_eq!(workspace_work.linear_matvecs, wrapper_work.linear_matvecs);
    assert_eq!(workspace_state.generation, wrapper_state.generation);
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
