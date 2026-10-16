//! Contract tests and run export of research node SP02
//! (`research/sp02_krylov_workspace_caller_20261011`).
//!
//! Evaluates the wiring of reusable least-squares workspace and LGMRES-into
//! into the matrix-free U-form driver caller (`Rodas5pMfFastWorkspace`).

#[allow(unused_imports)]
#[path = "alg01_common/mod.rs"]
mod alg01_common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use alg01_common::*;
use rodas5p_core::{
    CoreError, DenseMatrix, ExactOperatorIdentity, ExactPreconditionerIdentity,
    IdentityPreconditioner, InitialGuess, KrylovSystemIdentity, LinearMethod, LinearOperator,
    LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, AttemptLgmresSnapshot, KrylovState, KrylovWorkspaceBytes,
    KrylovWorkspaceOptions, OdeProblem, OutputSchedule, Rodas5pMfFastWorkspace,
    integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresIntoOptions, GmresWorkspace, LgmresConfig,
    LgmresIntoWorkspace, LgmresState, ResidualAccounting, solve_gmres_into_with_accounting,
    solve_lgmres_into,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub struct TrackingAllocator;

pub static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
pub static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
pub static PEAK_LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let live = LIVE_BYTES.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK_LIVE_BYTES.fetch_max(live, Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            let live = LIVE_BYTES.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK_LIVE_BYTES.fetch_max(live, Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            if new_size > layout.size() {
                let diff = new_size - layout.size();
                let live = LIVE_BYTES.fetch_add(diff, Ordering::SeqCst) + diff;
                PEAK_LIVE_BYTES.fetch_max(live, Ordering::SeqCst);
            } else {
                let diff = layout.size() - new_size;
                LIVE_BYTES.fetch_sub(diff, Ordering::SeqCst);
            }
        }
        new_ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

#[derive(Clone, Copy, Debug)]
pub struct AllocMeasure {
    pub events: usize,
    pub peak_bytes: usize,
    pub start_live_bytes: usize,
    pub end_live_bytes: usize,
}

pub fn measure_call<T>(f: impl FnOnce() -> T) -> (T, AllocMeasure) {
    let start_alloc = ALLOCATIONS.load(Ordering::SeqCst);
    let start_live = LIVE_BYTES.load(Ordering::SeqCst);
    PEAK_LIVE_BYTES.store(start_live, Ordering::SeqCst);
    let value = f();
    let end_alloc = ALLOCATIONS.load(Ordering::SeqCst);
    let end_live = LIVE_BYTES.load(Ordering::SeqCst);
    let peak = PEAK_LIVE_BYTES.load(Ordering::SeqCst);
    let peak_bytes = peak.saturating_sub(start_live);
    let events = end_alloc.saturating_sub(start_alloc);
    (
        value,
        AllocMeasure {
            events,
            peak_bytes,
            start_live_bytes: start_live,
            end_live_bytes: end_live,
        },
    )
}

fn sp02_linear_config(method: LinearMethod) -> LinearSolverConfig {
    LinearSolverConfig {
        method,
        rtol: 1.0e-11,
        atol: 1.0e-14,
        restart: 40,
        inner_m: 30,
        outer_k: 8,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
        ..LinearSolverConfig::default()
    }
}

fn solve_log_hash(log: Option<&[rodas5p_integrators::StageSolveLogEntry]>) -> String {
    let mut hasher = Sha256::new();
    if let Some(entries) = log {
        for entry in entries {
            hasher.update(entry.residual_norm_bits.to_le_bytes());
            hasher.update(entry.relative_residual_bits.to_le_bytes());
            hasher.update(entry.iterations.to_le_bytes());
        }
    }
    format!("{:x}", hasher.finalize())
}

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn caller_parity_attempt_and_trajectory() {
    let _lock = TEST_LOCK.lock().unwrap();
    let p = spd07_problem("quadratic-4");
    let h = 1.0e-3;

    // 1. One attempt parity: GMRES
    {
        let linear = sp02_linear_config(LinearMethod::Gmres);
        let mut work_ctrl = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
        work_ctrl.set_gmres_into(true);
        let mut counters_ctrl = WorkCounters::default();
        let err_ctrl = work_ctrl
            .attempt(
                &p.problem,
                p.t_span.0,
                &p.y0,
                h,
                true,
                None,
                1.0e-6,
                1.0e-6,
                &mut counters_ctrl,
            )
            .unwrap();

        let mut work_cand = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
        work_cand
            .set_krylov_workspace(KrylovWorkspaceOptions::ls_workspace())
            .unwrap();
        let mut counters_cand = WorkCounters::default();
        let err_cand = work_cand
            .attempt(
                &p.problem,
                p.t_span.0,
                &p.y0,
                h,
                true,
                None,
                1.0e-6,
                1.0e-6,
                &mut counters_cand,
            )
            .unwrap();

        assert_eq!(err_ctrl.to_bits(), err_cand.to_bits());
        assert_eq!(hexes(work_ctrl.y_new()), hexes(work_cand.y_new()));
        assert_eq!(counters_ctrl, counters_cand);
    }

    // 2. One attempt parity: LGMRES
    {
        let linear = sp02_linear_config(LinearMethod::Lgmres);
        let mut work_ctrl = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
        let mut state_ctrl = KrylovState::for_method(LinearMethod::Lgmres);
        let mut counters_ctrl = WorkCounters::default();
        let err_ctrl = work_ctrl
            .attempt(
                &p.problem,
                p.t_span.0,
                &p.y0,
                h,
                true,
                state_ctrl.as_mut(),
                1.0e-6,
                1.0e-6,
                &mut counters_ctrl,
            )
            .unwrap();

        let mut work_cand = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
        work_cand
            .set_krylov_workspace(KrylovWorkspaceOptions::all())
            .unwrap();
        let mut state_cand = KrylovState::for_method(LinearMethod::Lgmres);
        let mut counters_cand = WorkCounters::default();
        let err_cand = work_cand
            .attempt(
                &p.problem,
                p.t_span.0,
                &p.y0,
                h,
                true,
                state_cand.as_mut(),
                1.0e-6,
                1.0e-6,
                &mut counters_cand,
            )
            .unwrap();

        assert_eq!(err_ctrl.to_bits(), err_cand.to_bits());
        assert_eq!(hexes(work_ctrl.y_new()), hexes(work_cand.y_new()));
        assert_eq!(counters_ctrl, counters_cand);
        let Some(KrylovState::Lgmres(st_ctrl)) = &state_ctrl else {
            panic!("expected Lgmres state");
        };
        let Some(KrylovState::Lgmres(st_cand)) = &state_cand else {
            panic!("expected Lgmres state");
        };
        assert_eq!(st_ctrl.directions, st_cand.directions);
        assert_eq!(st_ctrl.images, st_cand.images);
        assert_eq!(st_ctrl.previous_solution, st_cand.previous_solution);
        assert_eq!(st_ctrl.generation, st_cand.generation);
    }

    // 3. Short trajectory parity: GMRES and LGMRES
    for method in [LinearMethod::Gmres, LinearMethod::Lgmres] {
        let linear = sp02_linear_config(method);
        let adaptive = AdaptiveStepConfig {
            max_attempts: 100,
            ..p.adaptive(1.0e-6)
        };
        let t_span = (p.t_span.0, p.t_span.0 + 0.05);
        let schedule = OutputSchedule::new(vec![t_span.0, t_span.1]).unwrap();

        // Control run
        let mut work_ctrl = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
        if method == LinearMethod::Gmres {
            work_ctrl.set_gmres_into(true);
        }
        let res_ctrl = work_ctrl
            .integrate_observed(&p.problem, t_span, &p.y0, &adaptive, &schedule)
            .unwrap();

        // Candidate run
        let mut work_cand = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
        let cand_opt = if method == LinearMethod::Gmres {
            KrylovWorkspaceOptions::ls_workspace()
        } else {
            KrylovWorkspaceOptions::all()
        };
        work_cand.set_krylov_workspace(cand_opt).unwrap();
        let res_cand = work_cand
            .integrate_observed(&p.problem, t_span, &p.y0, &adaptive, &schedule)
            .unwrap();

        assert_eq!(res_ctrl.observed.success, res_cand.observed.success);
        assert_eq!(res_ctrl.attempts, res_cand.attempts);
        assert_eq!(res_ctrl.accepted_steps, res_cand.accepted_steps);
        assert_eq!(res_ctrl.rejected_steps, res_cand.rejected_steps);
        assert_eq!(hexes(&res_ctrl.observed.t), hexes(&res_cand.observed.t));
        assert_eq!(
            res_ctrl
                .observed
                .y
                .iter()
                .map(|y| hexes(y))
                .collect::<Vec<_>>(),
            res_cand
                .observed
                .y
                .iter()
                .map(|y| hexes(y))
                .collect::<Vec<_>>()
        );
        assert_eq!(res_ctrl.observed.counters, res_cand.observed.counters);

        // For GMRES, also check equivalence with integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace
        if method == LinearMethod::Gmres {
            let legacy_ls = integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace(
                &p.problem, t_span, &p.y0, &linear, &adaptive, &schedule,
            )
            .unwrap();
            assert_eq!(hexes(&res_cand.observed.t), hexes(&legacy_ls.observed.t));
            assert_eq!(
                res_cand
                    .observed
                    .y
                    .iter()
                    .map(|y| hexes(y))
                    .collect::<Vec<_>>(),
                legacy_ls
                    .observed
                    .y
                    .iter()
                    .map(|y| hexes(y))
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn forced_failure_rollback_gmres_and_lgmres() {
    let _lock = TEST_LOCK.lock().unwrap();
    let p = spd07_problem("van-der-pol-mu1000");

    // LGMRES forced failure: linear rtol 1e-14 with max_outer 1
    {
        let forced_linear = LinearSolverConfig {
            method: LinearMethod::Lgmres,
            rtol: 1.0e-14,
            atol: 1.0e-14,
            maxiter: 1,
            inner_m: 1,
            outer_k: 8,
            preconditioner: PreconditionerKind::None,
            ..LinearSolverConfig::default()
        };
        let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &forced_linear).unwrap();
        work.set_krylov_workspace(KrylovWorkspaceOptions::all())
            .unwrap();

        let mut recycle = KrylovState::for_method(LinearMethod::Lgmres);
        let before_recycle = recycle.clone();
        let mut counters = WorkCounters::default();

        // Stage 0 before attempt
        let before_stage0 = work.stage(0).to_vec();

        // Attempt with large step h = 10.0 so linear solve cannot converge in 1 iteration
        let outcome = work.attempt(
            &p.problem,
            p.t_span.0,
            &p.y0,
            10.0,
            true,
            recycle.as_mut(),
            1.0e-6,
            1.0e-6,
            &mut counters,
        );
        assert!(outcome.is_err(), "forced failure must error");

        // Stage output is untouched on failure
        assert_eq!(
            work.stage(0),
            &before_stage0[..],
            "stage output must be untouched on failure"
        );

        // Under option, attempt-level rollback restores the state
        if let Some(KrylovState::Lgmres(st)) = recycle.as_mut() {
            work.attempt_snapshot_mut().restore(st);
        }
        assert_eq!(
            recycle, before_recycle,
            "carried state after rejected attempt must be bitwise identical"
        );
    }

    // GMRES forced failure: linear rtol 1e-14 with maxiter 1
    {
        let forced_linear = LinearSolverConfig {
            method: LinearMethod::Gmres,
            rtol: 1.0e-14,
            atol: 1.0e-14,
            maxiter: 1,
            restart: 1,
            preconditioner: PreconditionerKind::None,
            ..LinearSolverConfig::default()
        };
        let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &forced_linear).unwrap();
        work.set_krylov_workspace(KrylovWorkspaceOptions::ls_workspace())
            .unwrap();

        let mut counters = WorkCounters::default();
        let before_stage0 = work.stage(0).to_vec();

        let outcome = work.attempt(
            &p.problem,
            p.t_span.0,
            &p.y0,
            10.0,
            true,
            None,
            1.0e-6,
            1.0e-6,
            &mut counters,
        );
        assert!(outcome.is_err(), "forced GMRES failure must error");
        assert_eq!(
            work.stage(0),
            &before_stage0[..],
            "stage output must be untouched on failure"
        );
    }
}

#[test]
fn nonfinite_jvp_rejection_rollback() {
    let _lock = TEST_LOCK.lock().unwrap();
    let p = spd07_problem("robertson");

    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        out[2] = 3.0e7 * y[1] * y[1];
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, _y: &[f64], _v: &[f64], out: &mut [f64]| {
        out.fill(f64::NAN);
        Ok(())
    });
    let bad_problem = OdeProblem::new(
        "bad-robertson",
        3,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap();

    let linear = sp02_linear_config(LinearMethod::Lgmres);
    let mut work = Rodas5pMfFastWorkspace::new(&bad_problem, &linear).unwrap();
    work.set_krylov_workspace(KrylovWorkspaceOptions::all())
        .unwrap();

    let mut recycle = KrylovState::for_method(LinearMethod::Lgmres);
    let before_recycle = recycle.clone();
    let mut counters = WorkCounters::default();

    let outcome = work.attempt(
        &bad_problem,
        p.t_span.0,
        &p.y0,
        1.0e-4,
        true,
        recycle.as_mut(),
        1.0e-6,
        1.0e-6,
        &mut counters,
    );
    assert!(outcome.is_err(), "nonfinite JVP must reject attempt");

    if let Some(KrylovState::Lgmres(st)) = recycle.as_mut() {
        work.attempt_snapshot_mut().restore(st);
    }
    assert_eq!(
        recycle, before_recycle,
        "carried state after nonfinite rejection must be bitwise the state before it"
    );
}

#[test]
fn workspace_growth_reuse_and_clone() {
    let _lock = TEST_LOCK.lock().unwrap();
    let p = spd07_problem("quadratic-4");
    let linear = sp02_linear_config(LinearMethod::Lgmres);
    let adaptive = p.adaptive(1.0e-6);
    let schedule = p.schedule();

    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
    work.set_krylov_workspace(KrylovWorkspaceOptions::all())
        .unwrap();

    // First integration
    let res1 = work
        .integrate_observed(&p.problem, p.t_span, &p.y0, &adaptive, &schedule)
        .unwrap();
    assert!(res1.observed.success);
    let res1_growth = work.krylov_workspace_growth();
    assert!(
        res1_growth.total() > 0,
        "first integration must size buffers and record growth"
    );

    // Reset growth counters
    work.reset_krylov_workspace_growth();
    assert_eq!(work.krylov_workspace_growth().total(), 0);

    // Second integration with reused workspace
    let res2 = work
        .integrate_observed(&p.problem, p.t_span, &p.y0, &adaptive, &schedule)
        .unwrap();
    assert!(res2.observed.success);
    let second_growth = work.krylov_workspace_growth();
    println!("second growth: {:#?}", second_growth);
    assert_eq!(
        second_growth.total(),
        0,
        "reused workspace must experience 0 growth events"
    );
    assert_eq!(
        hexes(&res1.observed.t),
        hexes(&res2.observed.t),
        "reused workspace must produce bitwise identical times"
    );
    assert_eq!(
        res1.observed.y.iter().map(|y| hexes(y)).collect::<Vec<_>>(),
        res2.observed.y.iter().map(|y| hexes(y)).collect::<Vec<_>>(),
        "reused workspace must produce bitwise identical states"
    );

    // Verify disjoint total equality on reused workspace
    let b: KrylovWorkspaceBytes = work.krylov_workspace_bytes();
    let disjoint_sum = b.gmres_bytes
        + b.least_squares_bytes
        + b.lgmres_bytes
        + b.lgmres_into_bytes
        + b.attempt_snapshot_bytes
        + b.gcrodr_bytes;
    assert_eq!(
        b.total(),
        disjoint_sum,
        "krylov workspace bytes must be disjoint"
    );

    // Clone driver workspace: starts with empty least-squares buffers
    let mut cloned = work.clone();
    assert_eq!(
        cloned.krylov_workspace_bytes().least_squares_bytes,
        0,
        "cloned workspace starts with empty least-squares buffers"
    );
    assert_eq!(
        cloned.krylov_workspace_bytes().attempt_snapshot_bytes,
        0,
        "cloned workspace starts with empty snapshot buffers"
    );
    let cloned_disjoint_sum = cloned.krylov_workspace_bytes().gmres_bytes
        + cloned.krylov_workspace_bytes().least_squares_bytes
        + cloned.krylov_workspace_bytes().lgmres_bytes
        + cloned.krylov_workspace_bytes().lgmres_into_bytes
        + cloned.krylov_workspace_bytes().attempt_snapshot_bytes
        + cloned.krylov_workspace_bytes().gcrodr_bytes;
    assert_eq!(
        cloned.krylov_workspace_bytes().total(),
        cloned_disjoint_sum,
        "cloned workspace bytes must be disjoint"
    );

    cloned.reset_krylov_workspace_growth();
    let res_clone = cloned
        .integrate_observed(&p.problem, p.t_span, &p.y0, &adaptive, &schedule)
        .unwrap();
    assert!(res_clone.observed.success);
    assert_eq!(
        hexes(&res1.observed.t),
        hexes(&res_clone.observed.t),
        "cloned workspace must produce bitwise identical times"
    );
    assert_eq!(
        res1.observed.y.iter().map(|y| hexes(y)).collect::<Vec<_>>(),
        res_clone
            .observed
            .y
            .iter()
            .map(|y| hexes(y))
            .collect::<Vec<_>>(),
        "cloned workspace must produce bitwise identical states"
    );
    assert!(
        cloned.krylov_workspace_growth().total() > 0,
        "cloned workspace regrows empty least-squares buffers"
    );
    assert!(
        cloned.krylov_workspace_bytes().least_squares_bytes > 0,
        "cloned workspace has nonzero least-squares bytes after first solve"
    );
}

struct TestStageOp {
    mat: DenseMatrix,
    token: u64,
}

impl LinearOperator for TestStageOp {
    fn apply(&self, x: &[f64], y: &mut [f64]) -> rodas5p_core::CoreResult<()> {
        self.mat.matvec_into(x, y)
    }
    fn dimension(&self) -> usize {
        self.mat.nrows()
    }
    fn exact_identity(&self) -> Option<ExactOperatorIdentity> {
        Some(ExactOperatorIdentity::Instance(self.token))
    }
    fn token(&self) -> u64 {
        self.token
    }
}

fn test_matrix(n: usize) -> DenseMatrix {
    let mut mat = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            mat[(i, j)] = 1.0 / (1.0 + (i + j) as f64) + if i == j { 3.0 } else { 0.0 };
        }
    }
    mat
}

#[test]
fn warmed_krylov_solves_and_snapshot_rollback_allocate_zero() {
    let _lock = TEST_LOCK.lock().unwrap();
    let n = 10;
    let mat = test_matrix(n);
    let op = TestStageOp { mat, token: 1 };
    let pc = IdentityPreconditioner::new(n);
    let rhs = vec![1.0; n];
    let mut x = vec![0.0; n];

    // 1. GMRES: warm GmresWorkspace::with_ls_workspace(), then assert warmed solve allocates 0
    let mut gmres_ws = GmresWorkspace::with_ls_workspace();
    let gmres_config = GmresConfig {
        restart: 10,
        max_arnoldi: 20,
        rtol: 1e-10,
        atol: 1e-12,
    };
    let mut counters = WorkCounters::default();
    solve_gmres_into_with_accounting(
        &op,
        &pc,
        &rhs,
        None,
        &gmres_config,
        None,
        &mut x,
        &mut gmres_ws,
        GmresCapacity::unbounded(),
        GmresIntoOptions::default(),
        ResidualAccounting::RecomputeFinal,
        &mut counters,
    )
    .unwrap();

    let (_, measure_gmres) = measure_call(|| {
        solve_gmres_into_with_accounting(
            &op,
            &pc,
            &rhs,
            None,
            &gmres_config,
            None,
            &mut x,
            &mut gmres_ws,
            GmresCapacity::unbounded(),
            GmresIntoOptions::default(),
            ResidualAccounting::RecomputeFinal,
            &mut counters,
        )
        .unwrap();
    });
    assert_eq!(
        measure_gmres.events, 0,
        "warmed solve_gmres_into must allocate 0 heap blocks"
    );

    // 2. LGMRES: warm LgmresIntoWorkspace, then assert warmed solve allocates 0
    let mut lgmres_ws = LgmresIntoWorkspace::default();
    lgmres_ws.set_least_squares_once(true);
    lgmres_ws.set_ls_workspace(true);
    let lgmres_config = LgmresConfig {
        inner_m: 10,
        max_outer: 5,
        outer_k: 3,
        rtol: 1e-10,
        atol: 1e-12,
    };
    let mut state = LgmresState::default();
    // Warm LGMRES until recycle directions and snapshot buffers reach steady state
    for _ in 0..4 {
        solve_lgmres_into(
            &op,
            &pc,
            &rhs,
            None,
            &lgmres_config,
            &mut state,
            None,
            &mut x,
            &mut lgmres_ws,
            &mut counters,
        )
        .unwrap();
    }

    let (_, measure_lgmres) = measure_call(|| {
        solve_lgmres_into(
            &op,
            &pc,
            &rhs,
            None,
            &lgmres_config,
            &mut state,
            None,
            &mut x,
            &mut lgmres_ws,
            &mut counters,
        )
        .unwrap();
    });
    assert_eq!(
        measure_lgmres.events, 0,
        "warmed solve_lgmres_into must allocate 0 heap blocks"
    );

    // 3. AttemptLgmresSnapshot: warm with save + restore, then assert warmed save + restore allocates 0
    let mut snapshot = AttemptLgmresSnapshot::default();
    snapshot.save(&state);
    snapshot.restore(&mut state);

    let (_, measure_snap) = measure_call(|| {
        snapshot.save(&state);
        snapshot.restore(&mut state);
    });
    assert_eq!(
        measure_snap.events, 0,
        "warmed AttemptLgmresSnapshot save + restore must allocate 0 heap blocks"
    );
}

#[test]
fn snapshot_rollback_restores_system_identity_under_same_token() {
    let _lock = TEST_LOCK.lock().unwrap();
    let mut state = LgmresState {
        directions: vec![vec![1.0, 2.0, 3.0, 4.0, 5.0]],
        images: vec![Some(vec![0.1, 0.2, 0.3, 0.4, 0.5])],
        operator_token: Some(42),
        system_identity: Some(KrylovSystemIdentity {
            operator: ExactOperatorIdentity::Instance(100),
            preconditioner: ExactPreconditionerIdentity::Identity { dimension: 5 },
        }),
        previous_solution: Some(vec![0.5, 0.5, 0.5, 0.5, 0.5]),
        generation: 7,
    };

    let before_state = state.clone();

    let mut snapshot = AttemptLgmresSnapshot::default();
    snapshot.save(&state);

    // Simulate an attempt that mutates system_identity under the same operator_token
    state.system_identity = Some(KrylovSystemIdentity {
        operator: ExactOperatorIdentity::Instance(200),
        preconditioner: ExactPreconditionerIdentity::Identity { dimension: 5 },
    });
    state.generation = 8;
    state.directions.push(vec![6.0, 7.0, 8.0, 9.0, 10.0]);

    // Roll back rejected attempt
    snapshot.restore(&mut state);

    assert_eq!(state.operator_token, before_state.operator_token);
    assert_eq!(state.system_identity, before_state.system_identity);
    assert_eq!(state.generation, before_state.generation);
    assert_eq!(state.directions.len(), before_state.directions.len());
    for (d1, d2) in state.directions.iter().zip(&before_state.directions) {
        assert_eq!(hexes(d1), hexes(d2));
    }
    assert_eq!(state.images.len(), before_state.images.len());
    for (im1, im2) in state.images.iter().zip(&before_state.images) {
        assert_eq!(im1.is_some(), im2.is_some());
        if let (Some(v1), Some(v2)) = (im1, im2) {
            assert_eq!(hexes(v1), hexes(v2));
        }
    }
    if let (Some(p1), Some(p2)) = (&state.previous_solution, &before_state.previous_solution) {
        assert_eq!(hexes(p1), hexes(p2));
    }
}

#[test]
fn refusal_contracts() {
    let _lock = TEST_LOCK.lock().unwrap();
    let p = spd07_problem("robertson");

    // 1. GCRO-DR with ls_workspace
    let gcrodr_linear = LinearSolverConfig {
        method: LinearMethod::Gcrodr,
        ..LinearSolverConfig::default()
    };
    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &gcrodr_linear).unwrap();
    let err1 = work.set_krylov_workspace(KrylovWorkspaceOptions::ls_workspace());
    assert!(
        matches!(err1, Err(CoreError::InvalidInput(_))),
        "GCRO-DR with ls_workspace must be refused with InvalidInput"
    );

    // 2. GCRO-DR with lgmres_into
    let err2 = work.set_krylov_workspace(KrylovWorkspaceOptions::lgmres_into());
    assert!(
        matches!(err2, Err(CoreError::InvalidInput(_))),
        "GCRO-DR with lgmres_into must be refused with InvalidInput"
    );

    // 3. GMRES with lgmres_into
    let gmres_linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        ..LinearSolverConfig::default()
    };
    let mut work_gmres = Rodas5pMfFastWorkspace::new(&p.problem, &gmres_linear).unwrap();
    let err3 = work_gmres.set_krylov_workspace(KrylovWorkspaceOptions::lgmres_into());
    assert!(
        matches!(err3, Err(CoreError::InvalidInput(_))),
        "GMRES with lgmres_into must be refused with InvalidInput"
    );

    // 4. LGMRES with ls_workspace alone (without lgmres_into)
    let lgmres_linear = LinearSolverConfig {
        method: LinearMethod::Lgmres,
        ..LinearSolverConfig::default()
    };
    let mut work_lgmres = Rodas5pMfFastWorkspace::new(&p.problem, &lgmres_linear).unwrap();
    let err4 = work_lgmres.set_krylov_workspace(KrylovWorkspaceOptions::ls_workspace());
    assert!(
        matches!(err4, Err(CoreError::InvalidInput(_))),
        "LGMRES with ls_workspace without lgmres_into must be refused with InvalidInput"
    );
}

#[test]
fn faer_premise_pinned() {
    let _lock = TEST_LOCK.lock().unwrap();
    let cargo_lock = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock"),
    )
    .expect("Cargo.lock must exist");

    let mut found_faer_version = None;
    let mut lines = cargo_lock.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == "[[package]]" {
            let mut name = None;
            let mut version = None;
            while let Some(&next_line) = lines.peek() {
                if next_line.trim() == "[[package]]" {
                    break;
                }
                let next = lines.next().unwrap();
                if let Some(n) = next.strip_prefix("name = \"") {
                    name = n.strip_suffix('"');
                }
                if let Some(v) = next.strip_prefix("version = \"") {
                    version = v.strip_suffix('"');
                }
            }
            if name == Some("faer") {
                found_faer_version = version.map(|s| s.to_string());
                break;
            }
        }
    }
    assert_eq!(
        found_faer_version.as_deref(),
        Some("0.24.4"),
        "faer premise check: Cargo.lock must pin faer 0.24.4"
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunArm {
    Control,
    Candidate,
    Attribution,
}

impl RunArm {
    fn id(self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::Candidate => "candidate",
            Self::Attribution => "attribution",
        }
    }
}

/// Helper to execute and measure a single driver run
fn execute_run(p: &Problem, rtol: f64, method: LinearMethod, arm: RunArm) -> Value {
    let linear = sp02_linear_config(method);
    let adaptive = p.adaptive(rtol);
    let schedule = p.schedule();

    // 1. Log run with solve_log=true outside allocation measurement
    let mut log_work = Rodas5pMfFastWorkspace::new(&p.problem, &linear).expect("workspace new");
    log_work.set_solve_log(true);
    match (method, arm) {
        (LinearMethod::Gmres, RunArm::Control) => {
            log_work.set_gmres_into(true);
        }
        (LinearMethod::Gmres, RunArm::Candidate) => {
            log_work
                .set_krylov_workspace(KrylovWorkspaceOptions::ls_workspace())
                .expect("set_krylov_workspace");
        }
        (LinearMethod::Lgmres, RunArm::Control) => {
            // legacy path
        }
        (LinearMethod::Lgmres, RunArm::Candidate) => {
            log_work
                .set_krylov_workspace(KrylovWorkspaceOptions::all())
                .expect("set_krylov_workspace");
        }
        (LinearMethod::Lgmres, RunArm::Attribution) => {
            log_work
                .set_krylov_workspace(KrylovWorkspaceOptions::lgmres_into())
                .expect("set_krylov_workspace");
        }
        _ => panic!("unsupported method and arm combination"),
    }
    let (fast_res, _stats, _charges, log) = log_work
        .integrate_observed_with_observer(
            &p.problem,
            p.t_span,
            &p.y0,
            &adaptive,
            &schedule,
            &mut |_, _, _, _, _| {},
        )
        .expect("integration run");
    let log_hash = solve_log_hash(log.as_deref());

    // 2. Measure allocations cleanly with solve_log=false
    let (run_data, measure) = measure_call(|| {
        let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &linear).expect("workspace new");
        work.set_solve_log(false);
        match (method, arm) {
            (LinearMethod::Gmres, RunArm::Control) => {
                work.set_gmres_into(true);
            }
            (LinearMethod::Gmres, RunArm::Candidate) => {
                work.set_krylov_workspace(KrylovWorkspaceOptions::ls_workspace())
                    .expect("set_krylov_workspace");
            }
            (LinearMethod::Lgmres, RunArm::Control) => {
                // legacy path
            }
            (LinearMethod::Lgmres, RunArm::Candidate) => {
                work.set_krylov_workspace(KrylovWorkspaceOptions::all())
                    .expect("set_krylov_workspace");
            }
            (LinearMethod::Lgmres, RunArm::Attribution) => {
                work.set_krylov_workspace(KrylovWorkspaceOptions::lgmres_into())
                    .expect("set_krylov_workspace");
            }
            _ => panic!("unsupported method and arm combination"),
        }
        let _ = work.integrate_observed_with_observer(
            &p.problem,
            p.t_span,
            &p.y0,
            &adaptive,
            &schedule,
            &mut |_, _, _, _, _| {},
        );
        let b = work.krylov_workspace_bytes();
        let g = work.krylov_workspace_growth();
        (b, g)
    });

    let (bytes, growth) = run_data;

    json!({
        "problem": p.id,
        "rtol": rtol,
        "method": match method {
            LinearMethod::Gmres => "gmres",
            LinearMethod::Lgmres => "lgmres",
            _ => unreachable!(),
        },
        "arm": arm.id(),
        "success": fast_res.observed.success,
        "message": fast_res.observed.message,
        "attempts": fast_res.attempts,
        "accepted_steps": fast_res.accepted_steps,
        "rejected_steps": fast_res.rejected_steps,
        "state_reuses": fast_res.state_reuses,
        "counters": serde_json::to_value(fast_res.observed.counters).unwrap(),
        "final_state": hexes(fast_res.observed.y.last().unwrap()),
        "output_times": hexes(&fast_res.observed.t),
        "stage_solve_log_sha256": log_hash,
        "krylov_workspace": fast_res.krylov_workspace,
        "allocations": measure.events,
        "peak_live_bytes": measure.peak_bytes,
        "start_live_bytes": measure.start_live_bytes,
        "end_live_bytes": measure.end_live_bytes,
        "krylov_workspace_bytes": serde_json::to_value(bytes).unwrap(),
        "krylov_workspace_growth": serde_json::to_value(growth).unwrap(),
    })
}

#[test]
#[ignore = "export SP02 runs to RUNS.json"]
fn export_runs() {
    let _lock = TEST_LOCK.lock().unwrap();
    let problems = spd07_problems();
    let rtols = [1.0e-6, 1.0e-8];
    let methods = [LinearMethod::Gmres, LinearMethod::Lgmres];

    let mut rows = Vec::new();

    for p in &problems {
        for &rtol in &rtols {
            for &method in &methods {
                // Control
                let ctrl = execute_run(p, rtol, method, RunArm::Control);
                rows.push(ctrl);

                // Candidate
                let cand = execute_run(p, rtol, method, RunArm::Candidate);
                rows.push(cand);

                // Attribution for LGMRES
                if method == LinearMethod::Lgmres {
                    let attr = execute_run(p, rtol, method, RunArm::Attribution);
                    rows.push(attr);
                }
            }
        }
    }

    let report = json!({
        "schema": "vigilode-sp02-workspace-runs-v1",
        "node": "sp02_krylov_workspace_caller_20261011",
        "export": "export_runs",
        "test_threads": 1,
        "rayon_num_threads": std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "1".into()),
        "openblas_num_threads": std::env::var("OPENBLAS_NUM_THREADS").unwrap_or_else(|_| "1".into()),
        "faer_pinned": "0.24.4",
        "rows": rows,
    });

    write_output("SP02_RUNS", &report);
}
