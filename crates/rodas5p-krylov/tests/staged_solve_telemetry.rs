//! Solver telemetry of research node
//! `research/as04_stage_residual_telemetry_20261011` (AS04):
//! [`solve_staged_gmres_observed`] is [`solve_staged_gmres`] bit for bit
//! (report, output, counters) and fills a [`StagedSolveTelemetry`] whose
//! fields are consistent with the report and with independent counts.
//! Telemetry only: no field is an error certificate.

use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::{CoreResult, DenseMatrix, DenseOperator, LinearOperator, WorkCounters, safe_l2};
use rodas5p_krylov::{
    StagedAcceptanceReason, StagedGmresConfig, StagedGmresFailure, StagedGmresFallback,
    StagedGmresOutcome, StagedGmresReport, StagedGmresWorkspace, StagedGuardAbort,
    StagedSolveTelemetry, TargetAttainment, VerifiedBound, solve_staged_gmres,
    solve_staged_gmres_observed,
};

// The telemetry is `Copy` (registered compile-time assertion).
const _: fn() = || {
    fn assert_copy<T: Copy>() {}
    assert_copy::<StagedSolveTelemetry>();
};

fn tridiagonal(n: usize, sub: f64, diag: f64, sup: f64) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = diag;
        if i > 0 {
            a[(i, i - 1)] = sub;
        }
        if i + 1 < n {
            a[(i, i + 1)] = sup;
        }
    }
    a
}

fn cyclic_shift(n: usize) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[((i + 1) % n, i)] = 1.0;
    }
    a
}

/// A boxed fallback test, and a factory of them (one per solve).
type BoxedFallback = Box<dyn FnMut(&[f64], &[f64]) -> bool>;
type FallbackFactory = dyn FnMut() -> Option<BoxedFallback>;

fn bits(x: &[f64]) -> Vec<u64> {
    x.iter().map(|v| v.to_bits()).collect()
}

/// Debug form (reports hold NaN fields).
fn same<T: std::fmt::Debug>(a: &T, b: &T) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

/// `x -> A x`, counting every application (the counted ones and the
/// shadow continuation's).
struct Counting {
    inner: DenseOperator,
    applications: AtomicU64,
}

impl Counting {
    fn new(a: DenseMatrix) -> Self {
        Self {
            inner: DenseOperator::new(a).unwrap(),
            applications: AtomicU64::new(0),
        }
    }
    fn count(&self) -> u64 {
        self.applications.load(Ordering::SeqCst)
    }
}

impl LinearOperator for Counting {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.applications.fetch_add(1, Ordering::SeqCst);
        self.inner.apply(x, y)
    }
    fn token(&self) -> u64 {
        u64::MAX - 7
    }
}

/// `A x` for the first `finite` applications, `+Inf` afterwards.
struct TurnsInfinite {
    inner: DenseOperator,
    finite: u64,
    applications: AtomicU64,
}

impl LinearOperator for TurnsInfinite {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let k = self.applications.fetch_add(1, Ordering::SeqCst);
        self.inner.apply(x, y)?;
        if k >= self.finite {
            y.fill(f64::INFINITY);
        }
        Ok(())
    }
    fn token(&self) -> u64 {
        u64::MAX - 8
    }
}

struct Solved {
    report: StagedGmresReport,
    output: Vec<f64>,
    counters: WorkCounters,
    telemetry: StagedSolveTelemetry,
}

/// The observed solve; and the plain solve on a fresh operator built by
/// `make`, which must give the same report, output and counters.
fn observed_and_plain<O: LinearOperator>(
    make: &dyn Fn() -> O,
    rhs: &[f64],
    config: &StagedGmresConfig,
    fallback: &mut FallbackFactory,
    requested: f64,
) -> Solved {
    let n = rhs.len();
    let op = make();
    let mut output = vec![f64::NAN; n];
    let mut counters = WorkCounters::default();
    let mut telemetry = StagedSolveTelemetry::default();
    let mut f = fallback();
    let report = solve_staged_gmres_observed(
        &op,
        rhs,
        config,
        f.as_mut()
            .map(|f| &mut **f as &mut dyn FnMut(&[f64], &[f64]) -> bool),
        &mut output,
        &mut StagedGmresWorkspace::default(),
        &mut counters,
        requested,
        &mut telemetry,
    )
    .unwrap();
    let plain_op = make();
    let mut plain_output = vec![f64::NAN; n];
    let mut plain_counters = WorkCounters::default();
    let mut g = fallback();
    let plain = solve_staged_gmres(
        &plain_op,
        rhs,
        config,
        g.as_mut()
            .map(|f| &mut **f as &mut dyn FnMut(&[f64], &[f64]) -> bool),
        &mut plain_output,
        &mut StagedGmresWorkspace::default(),
        &mut plain_counters,
    )
    .unwrap();
    assert!(same(&report, &plain), "{report:?} != {plain:?}");
    assert_eq!(bits(&output), bits(&plain_output));
    assert_eq!(counters, plain_counters);
    Solved {
        report,
        output,
        counters,
        telemetry,
    }
}

fn dense(a: &DenseMatrix) -> impl Fn() -> DenseOperator + '_ {
    move || DenseOperator::new(a.clone()).unwrap()
}

fn no_fallback() -> Option<BoxedFallback> {
    None
}

/// Consistency of the telemetry with its report (every solve).
fn check_consistent(s: &Solved) {
    let (r, t) = (&s.report, &s.telemetry);
    assert_eq!(t.reason, StagedAcceptanceReason::from(r.outcome));
    assert_eq!(t.raised_target.to_bits(), r.threshold.to_bits());
    assert_eq!(t.tightened_target.to_bits(), r.final_threshold.to_bits());
    assert_eq!(t.actual_residual.to_bits(), r.residual_norm.to_bits());
    assert_eq!(
        t.attainment,
        TargetAttainment::classify(t.actual_residual, t.requested_target)
    );
    assert_eq!(t.heuristic_gain.nu_max.to_bits(), r.nu_max.to_bits());
    assert_eq!(t.heuristic_gain.nu_evaluations, r.nu_evaluations);
    assert_eq!(t.verified_bound, VerifiedBound::Unavailable);
    assert_eq!(t.nu_flops, r.nu_flops);
    assert_eq!(t.columns, r.columns);
    assert_eq!(t.matvecs, r.matvecs);
    assert_eq!(t.guard_abort, r.guard_abort);
    assert_eq!(t.budget_exhausted, r.budget_exhausted);
    assert_eq!(t.residual_jvps, r.true_residuals);
    // No hidden application: Arnoldi plus true residuals are the counted
    // Krylov applications.
    assert_eq!(t.arnoldi_jvps + t.residual_jvps, r.matvecs);
    assert_eq!(s.counters.linear_matvecs, r.matvecs);
    assert_eq!(t.acceptance_vector_copies, 0, "the solver copies nothing");
    assert!(t.tightened_target <= t.raised_target);
    match t.reason {
        StagedAcceptanceReason::Converged => {
            assert!(t.actual_residual <= t.tightened_target);
            assert!(t.attainable_floor.is_nan());
        }
        StagedAcceptanceReason::StallAccepted | StagedAcceptanceReason::FloorAccepted => {
            assert!(t.tightened_target < t.actual_residual);
            assert!(t.actual_residual <= t.attainable_floor);
        }
        StagedAcceptanceReason::FallbackAccepted => {
            assert!(t.guard_abort.is_some() || t.budget_exhausted);
            assert_eq!(t.fallback_evaluations, 1);
            assert!(t.attainable_floor.is_nan());
        }
        StagedAcceptanceReason::Failed => assert!(t.attainable_floor.is_nan()),
    }
}

/// The infinity norm of `b - A x` through the same operator arithmetic.
fn residual_inf(op: &dyn LinearOperator, b: &[f64], x: &[f64]) -> f64 {
    let mut ax = vec![0.0; b.len()];
    op.apply(x, &mut ax).unwrap();
    b.iter()
        .zip(&ax)
        .map(|(p, q)| (p - q).abs())
        .fold(0.0, f64::max)
}

// ------------------------------------------------------ S1 (NATIVE.json)

fn s1_systems() -> Vec<(&'static str, DenseMatrix, Vec<f64>)> {
    vec![
        ("identity2", DenseMatrix::identity(2), vec![1.0, 0.0]),
        (
            "triangular2",
            DenseMatrix::from_rows(&[&[3.1, 0.7], &[0.0, 2.3]]).unwrap(),
            vec![0.13, 0.29],
        ),
        (
            "triangular4",
            DenseMatrix::from_rows(&[
                &[1.3, 0.31, 0.17, 0.19],
                &[0.0, 2.7, 0.31, 0.17],
                &[0.0, 0.0, 3.1, 0.31],
                &[0.0, 0.0, 0.0, 4.9],
            ])
            .unwrap(),
            vec![0.7, 1.1, -0.3, 0.9],
        ),
    ]
}

/// The nine `stage_acceptance` configurations: exactly the stall and floor
/// acceptances of triangular2 and the floor acceptance of triangular4 are
/// above the requested target (the recorded threshold).
#[test]
fn s1_accepted_residual_above_target_is_visible() {
    let mut exceeded = Vec::new();
    for (id, a, b) in s1_systems() {
        for mode in ["strict", "stall", "floor"] {
            let mut config = StagedGmresConfig::new(b.len(), 16, 1.0e-30, 0.0);
            config.stall_rule = mode == "stall";
            config.floor_at_confirmations = mode == "floor";
            let requested = config.atol.max(config.rtol * safe_l2(&b));
            let s = observed_and_plain(&dense(&a), &b, &config, &mut no_fallback, requested);
            check_consistent(&s);
            assert_eq!(s.telemetry.requested_target, s.report.threshold);
            assert!(s.report.accepted());
            let op = DenseOperator::new(a.clone()).unwrap();
            assert_eq!(
                s.telemetry.actual_residual_inf.to_bits(),
                residual_inf(&op, &b, &s.output).to_bits(),
                "{id}_{mode}"
            );
            match s.telemetry.attainment {
                TargetAttainment::BudgetExceeded => exceeded.push(format!("{id}_{mode}")),
                TargetAttainment::Met => {}
                TargetAttainment::Unverified => panic!("{id}_{mode} unverified"),
            }
        }
    }
    assert_eq!(
        exceeded,
        [
            "triangular2_stall",
            "triangular2_floor",
            "triangular4_floor"
        ]
    );
}

// ---------------------------------------------------- generic identity

#[test]
fn observed_solve_is_the_plain_solve_bit_for_bit() {
    // Restarted, in-cycle, guard, nonnormal, stall and floor cases.
    let n = 60;
    let a = tridiagonal(n, -1.0, 2.5, -1.2);
    let b = vec![1.0; n];
    for (restart, budget) in [(5, 2000), (40, 200), (60, 60)] {
        let config = StagedGmresConfig::new(restart, budget, 1.0e-11, 0.0);
        let s = observed_and_plain(&dense(&a), &b, &config, &mut no_fallback, 1.0e-11);
        check_consistent(&s);
    }
    // Nonnormal VIG-type blocks with the nu guard.
    let (k, h) = (12, 0.05);
    let mut w = DenseMatrix::identity(8);
    for j in 0..4 {
        let lam = 10f64.powi(j as i32);
        let (p, q) = (2 * j, 2 * j + 1);
        w[(p, p)] += 2.0 * h * lam;
        w[(p, q)] -= h * lam * 2f64.powi(k);
        w[(q, p)] -= h * lam * 2f64.powi(-k);
        w[(q, q)] += 2.0 * h * lam;
    }
    let bw: Vec<f64> = (0..8).map(|i| 1.0 / (1.0 + i as f64)).collect();
    let mut config = StagedGmresConfig::new(8, 64, 1.0e-6, 0.0);
    config.nu_guard = true;
    config.small_system_exhaustion = true;
    let s = observed_and_plain(&dense(&w), &bw, &config, &mut no_fallback, 1.0e-7);
    check_consistent(&s);
    assert!(s.telemetry.heuristic_gain.nu_max > 1.0);
    assert!(s.telemetry.tightened_target < s.telemetry.raised_target);
    // Stall and floor at the attainable accuracy.
    let t = tridiagonal(8, 0.3, 3.0, -0.7);
    let bt: Vec<f64> = (0..8).map(|i| (i as f64).sin() + 1.0).collect();
    for (stall, floor) in [(true, false), (false, true), (true, true)] {
        let mut config = StagedGmresConfig::new(8, 64, 0.0, 0.0);
        config.stall_rule = stall;
        config.floor_at_confirmations = floor;
        let s = observed_and_plain(&dense(&t), &bt, &config, &mut no_fallback, 0.0);
        check_consistent(&s);
        assert!(s.report.accepted());
        assert_eq!(s.telemetry.attainment, TargetAttainment::BudgetExceeded);
    }
    // Zero right-hand side.
    let s = observed_and_plain(
        &dense(&t),
        &[0.0; 8],
        &StagedGmresConfig::new(4, 8, 1.0e-10, 0.0),
        &mut no_fallback,
        0.0,
    );
    check_consistent(&s);
    assert_eq!(s.telemetry.attainment, TargetAttainment::Met);
    assert_eq!(s.telemetry.actual_residual_inf, 0.0);
}

// ------------------------------------------------- S3 guard and shadow

/// The contraction-abort fixture: the shadow classification's operator
/// applications are reported as uncounted and equal an independent count.
#[test]
fn s3_shadow_applications_are_reported_not_counted() {
    let n = 50;
    let a = cyclic_shift(n);
    let mut b = vec![0.0; n];
    b[0] = 1.0;
    for classify in [false, true] {
        let mut config = StagedGmresConfig::new(10, 100, 1.0e-10, 0.0);
        config.stagnation_guard = true;
        config.classify_guard_aborts = classify;
        config.classify_accepted_guard_aborts = classify;
        let make = || Counting::new(a.clone());
        let op = make();
        let mut output = vec![f64::NAN; n];
        let mut counters = WorkCounters::default();
        let mut telemetry = StagedSolveTelemetry::default();
        let mut accept = |_: &[f64], _: &[f64]| true;
        let report = solve_staged_gmres_observed(
            &op,
            &b,
            &config,
            Some(&mut accept),
            &mut output,
            &mut StagedGmresWorkspace::default(),
            &mut counters,
            1.0e-10,
            &mut telemetry,
        )
        .unwrap();
        assert_eq!(report.outcome, StagedGmresOutcome::FallbackAccepted);
        assert_eq!(report.guard_abort, Some(StagedGuardAbort::Contraction));
        let independent = op.count() - report.matvecs;
        assert_eq!(telemetry.uncounted_shadow_matvecs, independent);
        assert_eq!(counters.linear_matvecs, report.matvecs);
        if classify {
            assert!(independent > 0);
            assert_eq!(report.shadow_converged, Some(false));
        } else {
            assert_eq!(independent, 0);
            assert_eq!(telemetry.uncounted_shadow_matvecs, 0);
        }
        // The inf norm is the returned iterate's, read before the shadow.
        let plain = DenseOperator::new(a.clone()).unwrap();
        assert_eq!(
            telemetry.actual_residual_inf.to_bits(),
            residual_inf(&plain, &b, &output).to_bits()
        );
        let s = Solved {
            report,
            output,
            counters,
            telemetry,
        };
        check_consistent(&s);
        assert_eq!(s.telemetry.attainment, TargetAttainment::BudgetExceeded);
        let mut accept: Box<FallbackFactory> =
            Box::new(|| Some(Box::new(|_: &[f64], _: &[f64]| true)));
        let again = observed_and_plain(&make, &b, &config, &mut *accept, 1.0e-10);
        assert!(same(&again.report, &s.report));
    }
}

// ------------------------------------------------- S4 raised target

/// `Converged` against the raised threshold `max(atol, rtol ||b||)` above
/// the requested target `atol` is `BudgetExceeded`.
#[test]
fn s4_converged_above_requested_target_is_budget_exceeded() {
    let a = DenseMatrix::from_rows(&[&[1.0, 0.0], &[0.0, 2.0]]).unwrap();
    let b = [1.0, 1.0];
    let config = StagedGmresConfig::new(2, 4, 0.5, 1.0e-30);
    let s = observed_and_plain(&dense(&a), &b, &config, &mut no_fallback, config.atol);
    check_consistent(&s);
    assert_eq!(s.report.outcome, StagedGmresOutcome::Converged);
    assert_eq!(s.telemetry.reason, StagedAcceptanceReason::Converged);
    assert_eq!(s.telemetry.attainment, TargetAttainment::BudgetExceeded);
    assert!(s.telemetry.requested_target < s.telemetry.raised_target);
    assert!(s.telemetry.actual_residual > s.telemetry.requested_target);
}

// ------------------------------------------------------- S5 failures

#[test]
fn s5_failures_are_recorded() {
    // NonFinite: the restart residual turns infinite.
    let a = tridiagonal(4, 0.3, 3.0, -0.7);
    let b = [1.0, 0.5, -0.25, 2.0];
    let config = StagedGmresConfig::new(1, 4, 1.0e-14, 0.0);
    let make = || TurnsInfinite {
        inner: DenseOperator::new(a.clone()).unwrap(),
        finite: 1,
        applications: AtomicU64::new(0),
    };
    let s = observed_and_plain(&make, &b, &config, &mut no_fallback, 1.0e-14);
    check_consistent(&s);
    assert_eq!(s.report.failure, Some(StagedGmresFailure::NonFinite));
    assert_eq!(s.telemetry.reason, StagedAcceptanceReason::Failed);
    assert_eq!(s.telemetry.attainment, TargetAttainment::Unverified);
    assert!(s.telemetry.actual_residual.is_infinite());
    assert!(s.telemetry.actual_residual_inf.is_infinite());
    // Breakdown: a zero operator; the Arnoldi application is counted
    // though no column is formed.
    let zero = DenseMatrix::zeros(3, 3);
    let s = observed_and_plain(
        &dense(&zero),
        &[1.0, 2.0, 3.0],
        &StagedGmresConfig::new(3, 9, 1.0e-10, 0.0),
        &mut no_fallback,
        1.0e-10,
    );
    check_consistent(&s);
    assert_eq!(s.report.failure, Some(StagedGmresFailure::Breakdown));
    assert_eq!(s.telemetry.columns, 0);
    assert_eq!(s.telemetry.arnoldi_jvps, 1);
    assert_eq!(s.telemetry.attainment, TargetAttainment::BudgetExceeded);
    // BudgetExhausted without a fallback.
    let mut b4 = [1.5, 1.5, 0.0, 0.0];
    let s = observed_and_plain(
        &dense(&cyclic_shift(4)),
        &b4,
        &StagedGmresConfig::new(1, 1, 1.0e-14, 0.0),
        &mut no_fallback,
        1.0e-14 * safe_l2(&b4),
    );
    check_consistent(&s);
    assert_eq!(s.report.failure, Some(StagedGmresFailure::BudgetExhausted));
    assert_eq!(s.telemetry.fallback_evaluations, 0);
    assert!(s.output.iter().all(|x| x.is_nan()));
    // The same with a rejecting and an accepting fallback.
    for accept in [false, true] {
        let mut fallback: Box<FallbackFactory> =
            Box::new(move || Some(Box::new(move |_: &[f64], _: &[f64]| accept)));
        let s = observed_and_plain(
            &dense(&cyclic_shift(4)),
            &b4,
            &StagedGmresConfig::new(1, 1, 1.0e-14, 0.0),
            &mut *fallback,
            1.0e-14 * safe_l2(&b4),
        );
        check_consistent(&s);
        assert_eq!(s.telemetry.fallback_evaluations, 1);
        assert_eq!(s.report.accepted(), accept);
    }
    // An invalid configuration: the error leaves only the requested target.
    let mut telemetry = StagedSolveTelemetry {
        arnoldi_jvps: 99,
        ..StagedSolveTelemetry::default()
    };
    b4[0] = 1.0;
    let error = solve_staged_gmres_observed(
        &DenseOperator::new(cyclic_shift(4)).unwrap(),
        &b4,
        &StagedGmresConfig::new(0, 1, 1.0e-14, 0.0),
        None::<StagedGmresFallback<'_>>,
        &mut [0.0; 4],
        &mut StagedGmresWorkspace::default(),
        &mut WorkCounters::default(),
        0.5,
        &mut telemetry,
    );
    assert!(error.is_err());
    assert_eq!(
        format!("{telemetry:?}"),
        format!(
            "{:?}",
            StagedSolveTelemetry {
                requested_target: 0.5,
                ..StagedSolveTelemetry::default()
            }
        )
    );
}

#[test]
fn attainment_classification() {
    assert_eq!(TargetAttainment::classify(1.0, 1.0), TargetAttainment::Met);
    assert_eq!(
        TargetAttainment::classify(1.0, 0.5),
        TargetAttainment::BudgetExceeded
    );
    for (a, r) in [
        (f64::NAN, 1.0),
        (1.0, f64::NAN),
        (f64::INFINITY, 1.0),
        (0.0, f64::INFINITY),
    ] {
        assert_eq!(
            TargetAttainment::classify(a, r),
            TargetAttainment::Unverified
        );
    }
}
