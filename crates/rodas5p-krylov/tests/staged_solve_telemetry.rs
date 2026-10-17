use rodas5p_core::{DenseMatrix, DenseOperator, LinearOperator, WorkCounters, safe_l2};
use rodas5p_krylov::{
    HeuristicGain, StagedAcceptanceReason, StagedGmresConfig, StagedGmresFailure,
    StagedGmresOutcome, StagedGmresWorkspace, StagedGuardAbort, StagedSolveTelemetry,
    TargetAttainment, VerifiedBound, solve_staged_gmres_observed,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
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

const PRODUCTION_RTOL: f64 = 1.0e-10;
const PRODUCTION_ATOL: f64 = 1.0e-14;

fn init_tel(requested_target: f64) -> StagedSolveTelemetry {
    StagedSolveTelemetry {
        reason: StagedAcceptanceReason::Failed,
        requested_target,
        raised_target: f64::NAN,
        tightened_target: f64::NAN,
        attainable_floor: f64::NAN,
        actual_residual: f64::NAN,
        actual_residual_inf: f64::NAN,
        attainment: TargetAttainment::Unverified,
        heuristic_gain: HeuristicGain::default(),
        verified_bound: VerifiedBound::Unavailable,
        arnoldi_jvps: 0,
        residual_jvps: 0,
        fallback_evaluations: 0,
        acceptance_vector_copies: 0,
        nu_flops: 0,
        uncounted_shadow_matvecs: 0,
    }
}

// ---------------------------------------------------------------------------
// S1: The nine stage_acceptance configurations of NATIVE.json
// ---------------------------------------------------------------------------

struct S1Case {
    id: &'static str,
    matrix: DenseMatrix,
    rhs: Vec<f64>,
    mode: &'static str,
    expected_outcome: StagedGmresOutcome,
    expected_columns: u64,
    expected_residual_norm: f64,
    expected_threshold: f64,
    expected_matvecs: u64,
}

fn s1_cases() -> Vec<S1Case> {
    let id2 = DenseMatrix::identity(2);
    let tri2 = DenseMatrix::from_rows(&[&[3.1, 0.7], &[0.0, 2.3]]).unwrap();
    let tri4 = DenseMatrix::from_rows(&[
        &[1.3, 0.31, 0.17, 0.19],
        &[0.0, 2.7, 0.31, 0.17],
        &[0.0, 0.0, 3.1, 0.31],
        &[0.0, 0.0, 0.0, 4.9],
    ])
    .unwrap();

    vec![
        S1Case {
            id: "identity2",
            matrix: id2.clone(),
            rhs: vec![1.0, 0.0],
            mode: "strict",
            expected_outcome: StagedGmresOutcome::Converged,
            expected_columns: 1,
            expected_residual_norm: 0.0,
            expected_threshold: 1e-30,
            expected_matvecs: 2,
        },
        S1Case {
            id: "identity2",
            matrix: id2.clone(),
            rhs: vec![1.0, 0.0],
            mode: "stall",
            expected_outcome: StagedGmresOutcome::Converged,
            expected_columns: 1,
            expected_residual_norm: 0.0,
            expected_threshold: 1e-30,
            expected_matvecs: 2,
        },
        S1Case {
            id: "identity2",
            matrix: id2,
            rhs: vec![1.0, 0.0],
            mode: "floor",
            expected_outcome: StagedGmresOutcome::Converged,
            expected_columns: 1,
            expected_residual_norm: 0.0,
            expected_threshold: 1e-30,
            expected_matvecs: 2,
        },
        S1Case {
            id: "triangular2",
            matrix: tri2.clone(),
            rhs: vec![0.13, 0.29],
            mode: "strict",
            expected_outcome: StagedGmresOutcome::Converged,
            expected_columns: 5,
            expected_residual_norm: 0.0,
            expected_threshold: 3.1780497164141403e-31,
            expected_matvecs: 8,
        },
        S1Case {
            id: "triangular2",
            matrix: tri2.clone(),
            rhs: vec![0.13, 0.29],
            mode: "stall",
            expected_outcome: StagedGmresOutcome::StallAccepted,
            expected_columns: 4,
            expected_residual_norm: 2.7755575615628914e-17,
            expected_threshold: 3.1780497164141403e-31,
            expected_matvecs: 6,
        },
        S1Case {
            id: "triangular2",
            matrix: tri2,
            rhs: vec![0.13, 0.29],
            mode: "floor",
            expected_outcome: StagedGmresOutcome::FloorAccepted,
            expected_columns: 2,
            expected_residual_norm: 7.850462293418876e-17,
            expected_threshold: 3.1780497164141403e-31,
            expected_matvecs: 3,
        },
        S1Case {
            id: "triangular4",
            matrix: tri4.clone(),
            rhs: vec![0.7, 1.1, -0.3, 0.9],
            mode: "strict",
            expected_outcome: StagedGmresOutcome::Converged,
            expected_columns: 12,
            expected_residual_norm: 0.0,
            expected_threshold: 1.61245154965971e-30,
            expected_matvecs: 15,
        },
        S1Case {
            id: "triangular4",
            matrix: tri4.clone(),
            rhs: vec![0.7, 1.1, -0.3, 0.9],
            mode: "stall",
            expected_outcome: StagedGmresOutcome::Converged,
            expected_columns: 12,
            expected_residual_norm: 0.0,
            expected_threshold: 1.61245154965971e-30,
            expected_matvecs: 15,
        },
        S1Case {
            id: "triangular4",
            matrix: tri4,
            rhs: vec![0.7, 1.1, -0.3, 0.9],
            mode: "floor",
            expected_outcome: StagedGmresOutcome::FloorAccepted,
            expected_columns: 4,
            expected_residual_norm: 6.684427777288335e-16,
            expected_threshold: 1.61245154965971e-30,
            expected_matvecs: 5,
        },
    ]
}

#[test]
fn s1_nine_native_cases_match_anchor() {
    let cases = s1_cases();
    assert_eq!(cases.len(), 9);

    for c in &cases {
        let expected_id = format!("{}_{}", c.id, c.mode);

        let mut config = StagedGmresConfig::new(c.rhs.len(), 16, 1.0e-30, 0.0);
        config.stall_rule = c.mode == "stall";
        config.floor_at_confirmations = c.mode == "floor";

        let op = DenseOperator::new(c.matrix.clone()).unwrap();
        let mut output = vec![f64::NAN; c.rhs.len()];
        let mut counters = WorkCounters::default();
        let right_norm = safe_l2(&c.rhs);
        let requested_target = config.atol.max(config.rtol * right_norm);

        let mut tel = init_tel(requested_target);
        let report = solve_staged_gmres_observed(
            &op,
            &c.rhs,
            &config,
            None,
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            requested_target,
            &mut tel,
        )
        .expect("solve succeeds");

        // Check report matches NATIVE.json bitwise
        assert_eq!(report.outcome, c.expected_outcome);
        assert_eq!(report.columns, c.expected_columns);
        assert_eq!(
            report.residual_norm.to_bits(),
            c.expected_residual_norm.to_bits()
        );
        assert_eq!(report.threshold.to_bits(), c.expected_threshold.to_bits());
        assert_eq!(report.matvecs, c.expected_matvecs);

        // Check telemetry fields
        assert_eq!(tel.verified_bound, VerifiedBound::Unavailable);
        assert!(tel.requested_target <= tel.raised_target);
        assert!(tel.tightened_target <= tel.raised_target);
        assert_eq!(tel.arnoldi_jvps + tel.residual_jvps, report.matvecs as u64);

        if expected_id == "triangular2_stall"
            || expected_id == "triangular2_floor"
            || expected_id == "triangular4_floor"
        {
            assert_eq!(
                tel.attainment,
                TargetAttainment::BudgetExceeded,
                "Case {expected_id} must be BudgetExceeded"
            );
            assert_eq!(
                tel.actual_residual.to_bits(),
                report.residual_norm.to_bits()
            );
            assert!(tel.actual_residual > tel.requested_target);
            assert!(tel.attainable_floor.is_finite());
            assert!(tel.actual_residual <= tel.attainable_floor);
        } else {
            assert_eq!(
                tel.attainment,
                TargetAttainment::Met,
                "Case {expected_id} must be Met"
            );
            assert_eq!(tel.reason, StagedAcceptanceReason::Converged);
            assert!(tel.actual_residual <= tel.tightened_target);
        }
    }
}

// ---------------------------------------------------------------------------
// S2: Fallback configurations
// ---------------------------------------------------------------------------

#[test]
fn s2_fallback_seam_overflow_fails_and_control_fails() {
    let mut matrix = DenseMatrix::zeros(4, 4);
    for j in 0..4 {
        matrix[((j + 1) % 4, j)] = 1.0;
    }
    let op = DenseOperator::new(matrix).unwrap();
    let scaled_rhs = [1.5, 1.5, 0.0, 0.0];

    // Case 1: Overflowing scale 1e308
    {
        let scale = 1.0e308;
        let physical_rhs: Vec<f64> = scaled_rhs.iter().map(|b| b * scale).collect();
        let requested_target = PRODUCTION_ATOL.max(PRODUCTION_RTOL * safe_l2(&physical_rhs));
        assert_eq!(requested_target, f64::INFINITY);

        let mut fallback = |_: &[f64], _: &[f64]| false;
        let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
        let mut output = [f64::NAN; 4];
        let mut counters = WorkCounters::default();
        let mut tel = init_tel(1.0e-14 * safe_l2(&scaled_rhs));

        let report = solve_staged_gmres_observed(
            &op,
            &scaled_rhs,
            &config,
            Some(&mut fallback),
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            tel.requested_target,
            &mut tel,
        )
        .unwrap();

        assert_eq!(report.outcome, StagedGmresOutcome::Failed);
        assert_eq!(report.failure, Some(StagedGmresFailure::BudgetExhausted));
        assert_eq!(tel.reason, StagedAcceptanceReason::Failed);
        assert_eq!(tel.fallback_evaluations, 1);
    }

    // Case 2: Finite scale 1e100
    {
        let mut fallback = |_: &[f64], _: &[f64]| false;
        let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
        let mut output = [f64::NAN; 4];
        let mut counters = WorkCounters::default();
        let mut tel = init_tel(1.0e-14 * safe_l2(&scaled_rhs));

        let report = solve_staged_gmres_observed(
            &op,
            &scaled_rhs,
            &config,
            Some(&mut fallback),
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            tel.requested_target,
            &mut tel,
        )
        .unwrap();

        assert_eq!(report.outcome, StagedGmresOutcome::Failed);
        assert_eq!(report.failure, Some(StagedGmresFailure::BudgetExhausted));
        assert_eq!(tel.reason, StagedAcceptanceReason::Failed);
        assert_eq!(tel.fallback_evaluations, 1);
    }
}

#[test]
fn s2_fallback_accepted_above_target_finite_and_homogeneous() {
    let mut matrix = DenseMatrix::zeros(4, 4);
    for j in 0..4 {
        matrix[((j + 1) % 4, j)] = 1.0;
    }
    let op = DenseOperator::new(matrix).unwrap();
    let scaled_rhs = [1.5, 1.5, 0.0, 0.0];

    // 1. Finite cyclic with loose fallback rule
    {
        let mut fallback = |_: &[f64], _: &[f64]| true;
        let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
        let mut output = [f64::NAN; 4];
        let mut counters = WorkCounters::default();
        let requested_target = 1.0e-14 * safe_l2(&scaled_rhs);
        let mut tel = init_tel(requested_target);

        let report = solve_staged_gmres_observed(
            &op,
            &scaled_rhs,
            &config,
            Some(&mut fallback),
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            requested_target,
            &mut tel,
        )
        .unwrap();

        assert_eq!(report.outcome, StagedGmresOutcome::FallbackAccepted);
        assert_eq!(tel.reason, StagedAcceptanceReason::FallbackAccepted);
        assert!(tel.actual_residual > tel.requested_target);
        assert_eq!(tel.attainment, TargetAttainment::BudgetExceeded);
        assert_eq!(tel.fallback_evaluations, 1);
    }

    // 2. Homogeneous-branch right-hand side (components 1.5e308, AS01)
    {
        let mut fallback = |_: &[f64], _: &[f64]| true;
        let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
        let mut output = [f64::NAN; 4];
        let mut counters = WorkCounters::default();
        let requested_target = 1.0e-14;
        let mut tel = init_tel(requested_target);

        let report = solve_staged_gmres_observed(
            &op,
            &scaled_rhs,
            &config,
            Some(&mut fallback),
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            requested_target,
            &mut tel,
        )
        .unwrap();

        assert_eq!(report.outcome, StagedGmresOutcome::FallbackAccepted);
        assert_eq!(tel.reason, StagedAcceptanceReason::FallbackAccepted);
        assert_eq!(tel.attainment, TargetAttainment::BudgetExceeded);
        assert_eq!(tel.fallback_evaluations, 1);
    }
}

// ---------------------------------------------------------------------------
// S3: Contraction-abort fixture with shadow classification
// ---------------------------------------------------------------------------

struct CountingOp<T: LinearOperator> {
    inner: T,
    calls: AtomicU64,
}

impl<T: LinearOperator> CountingOp<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
            calls: AtomicU64::new(0),
        }
    }
    fn calls(&self) -> u64 {
        self.calls.load(Ordering::SeqCst)
    }
}

impl<T: LinearOperator> LinearOperator for CountingOp<T> {
    fn apply(&self, x: &[f64], y: &mut [f64]) -> Result<(), rodas5p_core::CoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.apply(x, y)
    }
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
    fn token(&self) -> u64 {
        self.inner.token()
    }
}

#[test]
fn s3_guard_and_shadow_telemetry() {
    let n = 50;
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[((i + 1) % n, i)] = 1.0;
    }
    let dense_op = DenseOperator::new(a).unwrap();
    let mut b = vec![0.0; n];
    b[0] = 1.0;

    // Classification ON
    {
        let counting_op = CountingOp::new(dense_op.clone());
        let mut config = StagedGmresConfig::new(10, 100, 1.0e-10, 0.0);
        config.stagnation_guard = true;
        config.classify_guard_aborts = true;
        config.classify_accepted_guard_aborts = true;

        let mut accept = |_: &[f64], _: &[f64]| true;
        let mut output = vec![f64::NAN; n];
        let mut counters = WorkCounters::default();
        let requested_target = 1.0e-10;
        let mut tel = init_tel(requested_target);

        let report = solve_staged_gmres_observed(
            &counting_op,
            &b,
            &config,
            Some(&mut accept),
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            requested_target,
            &mut tel,
        )
        .unwrap();

        assert_eq!(report.outcome, StagedGmresOutcome::FallbackAccepted);
        assert_eq!(report.guard_abort, Some(StagedGuardAbort::Contraction));
        assert_eq!(report.shadow_converged, Some(false));

        // WorkCounters should only record counted matvecs
        assert_eq!(counters.linear_matvecs, report.matvecs);
        // Telemetry uncounted_shadow_matvecs must match the independent shadow count
        assert!(tel.uncounted_shadow_matvecs > 0);
        assert_eq!(
            counting_op.calls(),
            counters.linear_matvecs + tel.uncounted_shadow_matvecs
        );
    }

    // Classification OFF
    {
        let counting_op = CountingOp::new(dense_op);
        let mut config = StagedGmresConfig::new(10, 100, 1.0e-10, 0.0);
        config.stagnation_guard = true;
        config.classify_guard_aborts = false;
        config.classify_accepted_guard_aborts = false;

        let mut accept = |_: &[f64], _: &[f64]| true;
        let mut output = vec![f64::NAN; n];
        let mut counters = WorkCounters::default();
        let requested_target = 1.0e-10;
        let mut tel = init_tel(requested_target);

        let report = solve_staged_gmres_observed(
            &counting_op,
            &b,
            &config,
            Some(&mut accept),
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            requested_target,
            &mut tel,
        )
        .unwrap();

        assert_eq!(report.outcome, StagedGmresOutcome::FallbackAccepted);
        assert_eq!(tel.uncounted_shadow_matvecs, 0);
        assert_eq!(counting_op.calls(), counters.linear_matvecs);
    }
}

// ---------------------------------------------------------------------------
// S4: Raised target yields Converged with BudgetExceeded
// ---------------------------------------------------------------------------

#[test]
fn s4_raised_target_converged_budget_exceeded() {
    let matrix = DenseMatrix::from_rows(&[&[2.0, 0.0], &[0.0, 1.0]]).unwrap();
    let op = DenseOperator::new(matrix).unwrap();
    let b = vec![10.0, 10.0];

    // Tiny atol, large rtol
    let mut config = StagedGmresConfig::new(2, 2, 0.2, 1.0e-15);
    config.stall_rule = false;
    config.floor_at_confirmations = false;

    let requested_target = 1.0e-15;
    let mut output = vec![f64::NAN; 2];
    let mut counters = WorkCounters::default();
    let mut tel = init_tel(requested_target);

    let report = solve_staged_gmres_observed(
        &op,
        &b,
        &config,
        None,
        &mut output,
        &mut StagedGmresWorkspace::default(),
        &mut counters,
        requested_target,
        &mut tel,
    )
    .unwrap();

    assert_eq!(report.outcome, StagedGmresOutcome::Converged);
    assert_eq!(tel.reason, StagedAcceptanceReason::Converged);
    assert_eq!(tel.attainment, TargetAttainment::BudgetExceeded);
    assert!(tel.actual_residual > tel.requested_target);
    assert!(tel.actual_residual <= tel.tightened_target);
    assert!(tel.tightened_target <= tel.raised_target);
    assert!(tel.requested_target < tel.raised_target);
}

// ---------------------------------------------------------------------------
// S5: Failures (NonFinite, Breakdown, BudgetExhausted)
// ---------------------------------------------------------------------------

#[test]
fn s5_failures() {
    // 1. NonFinite
    {
        struct NanOnResidualOp(AtomicU64);
        impl LinearOperator for NanOnResidualOp {
            fn apply(&self, x: &[f64], y: &mut [f64]) -> Result<(), rodas5p_core::CoreError> {
                let count = self.0.fetch_add(1, Ordering::SeqCst);
                if count == 1 {
                    y.fill(f64::NAN);
                } else {
                    y[0] = 0.5 * x[0];
                    y[1] = 0.5 * x[1];
                }
                Ok(())
            }
            fn dimension(&self) -> usize {
                2
            }
            fn token(&self) -> u64 {
                42
            }
        }
        let op = NanOnResidualOp(AtomicU64::new(0));
        let b = vec![1.0, 1.0];
        let config = StagedGmresConfig::new(1, 2, 1.0e-10, 0.0);
        let mut output = vec![f64::NAN; 2];
        let mut counters = WorkCounters::default();
        let mut tel = init_tel(1.0e-10);

        let report = solve_staged_gmres_observed(
            &op,
            &b,
            &config,
            None,
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            1.0e-10,
            &mut tel,
        )
        .unwrap();

        assert_eq!(report.outcome, StagedGmresOutcome::Failed);
        assert_eq!(report.failure, Some(StagedGmresFailure::NonFinite));
        assert_eq!(tel.reason, StagedAcceptanceReason::Failed);
        assert_eq!(tel.attainment, TargetAttainment::Unverified);
    }

    // 2. Breakdown (zero matrix -> Arnoldi breaks down at j = 0)
    {
        let matrix = DenseMatrix::zeros(2, 2);
        let op = DenseOperator::new(matrix).unwrap();
        let b = vec![1.0, 1.0];
        let config = StagedGmresConfig::new(2, 2, 1.0e-10, 0.0);
        let mut output = vec![f64::NAN; 2];
        let mut counters = WorkCounters::default();
        let mut tel = init_tel(1.0e-10);

        let report = solve_staged_gmres_observed(
            &op,
            &b,
            &config,
            None,
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            1.0e-10,
            &mut tel,
        )
        .unwrap();

        assert_eq!(report.outcome, StagedGmresOutcome::Failed);
        assert_eq!(report.failure, Some(StagedGmresFailure::Breakdown));
        assert_eq!(tel.reason, StagedAcceptanceReason::Failed);
        assert_eq!(tel.attainment, TargetAttainment::Unverified);
    }

    // 3. BudgetExhausted without fallback
    {
        let mut matrix = DenseMatrix::zeros(4, 4);
        for j in 0..4 {
            matrix[((j + 1) % 4, j)] = 1.0;
        }
        let op = DenseOperator::new(matrix).unwrap();
        let b = vec![1.0, 1.0, 0.0, 0.0];
        let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
        let mut output = vec![f64::NAN; 4];
        let mut counters = WorkCounters::default();
        let mut tel = init_tel(1.0e-14);

        let report = solve_staged_gmres_observed(
            &op,
            &b,
            &config,
            None,
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            1.0e-14,
            &mut tel,
        )
        .unwrap();

        assert_eq!(report.outcome, StagedGmresOutcome::Failed);
        assert_eq!(report.failure, Some(StagedGmresFailure::BudgetExhausted));
        assert_eq!(tel.reason, StagedAcceptanceReason::Failed);
    }
}

// ---------------------------------------------------------------------------
// VG-1: Zero-Allocation Contract Test for Warmed Staged Solve Observed
// ---------------------------------------------------------------------------

#[test]
fn warmed_staged_solve_observed_allocates_zero() {
    let n = 20;
    let mut matrix = DenseMatrix::zeros(n, n);
    for i in 0..n {
        matrix[(i, i)] = 2.0;
        if i + 1 < n {
            matrix[(i, i + 1)] = 0.5;
        }
    }
    let op = DenseOperator::new(matrix).unwrap();
    let b = vec![1.0; n];
    let config = StagedGmresConfig::new(n, 20, 1.0e-10, 0.0);
    let mut ws = StagedGmresWorkspace::default();
    let mut output = vec![0.0; n];
    let mut counters = WorkCounters::default();
    let requested_target = 1.0e-10;
    let mut tel = init_tel(requested_target);

    // Warm the workspace on this system without nu_guard
    let report_warm = solve_staged_gmres_observed(
        &op,
        &b,
        &config,
        None,
        &mut output,
        &mut ws,
        &mut counters,
        requested_target,
        &mut tel,
    )
    .expect("warmup solve succeeds");
    assert_eq!(report_warm.outcome, StagedGmresOutcome::Converged);

    // Count allocations on second solve
    let (report, allocs) = allocation_count(|| {
        solve_staged_gmres_observed(
            &op,
            &b,
            &config,
            None,
            &mut output,
            &mut ws,
            &mut counters,
            requested_target,
            &mut tel,
        )
    });
    let report = report.expect("observed solve succeeds");
    assert_eq!(report.outcome, StagedGmresOutcome::Converged);
    assert_eq!(
        allocs, 0,
        "warmed solve_staged_gmres_observed must perform 0 heap allocations"
    );
}
