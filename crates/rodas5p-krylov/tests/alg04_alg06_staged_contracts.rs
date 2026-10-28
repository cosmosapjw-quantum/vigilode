//! Contracts of the staged GMRES options of research nodes
//! `research/alg04_coupled_target_v2_20261010` (ALG04: small-system
//! exhaustion) and `research/alg06_guard_v2_20261010` (ALG06: `G1`
//! effective cycle length, `G2` attainable-accuracy floor at every
//! confirmation, classification of fallback-accepted guard aborts).

use rodas5p_core::{CoreResult, DenseMatrix, DenseOperator, LinearOperator, WorkCounters, safe_l2};
use rodas5p_krylov::{
    STAGED_STALL_FACTOR, StagedGmresConfig, StagedGmresFallback, StagedGmresOutcome,
    StagedGmresReport, StagedGmresWorkspace, StagedGuardAbort, solve_staged_gmres,
    staged_guard_test,
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

fn residual_norm(a: &DenseMatrix, b: &[f64], x: &[f64]) -> f64 {
    let ax = a.matvec(x).unwrap();
    safe_l2(&b.iter().zip(&ax).map(|(p, q)| p - q).collect::<Vec<_>>())
}

fn solve_op(
    op: &dyn LinearOperator,
    b: &[f64],
    config: &StagedGmresConfig,
    fallback: Option<StagedGmresFallback<'_>>,
) -> (StagedGmresReport, Vec<f64>, WorkCounters) {
    let mut x = vec![f64::NAN; b.len()];
    let mut counters = WorkCounters::default();
    let mut ws = StagedGmresWorkspace::default();
    let report =
        solve_staged_gmres(op, b, config, fallback, &mut x, &mut ws, &mut counters).unwrap();
    (report, x, counters)
}

fn solve(
    a: &DenseMatrix,
    b: &[f64],
    config: &StagedGmresConfig,
    fallback: Option<StagedGmresFallback<'_>>,
) -> (StagedGmresReport, Vec<f64>, WorkCounters) {
    solve_op(&DenseOperator::new(a.clone()).unwrap(), b, config, fallback)
}

/// Debug form (the report holds NaN fields, which are not equal to
/// themselves).
fn same<T: std::fmt::Debug>(a: &T, b: &T) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

fn bits(x: &[f64]) -> Vec<u64> {
    x.iter().map(|v| v.to_bits()).collect()
}

// ------------------------------------------------------------ exhaustion

#[test]
fn exhaustion_runs_a_small_system_to_the_end_of_its_krylov_space() {
    let n = 12;
    let a = tridiagonal(n, -1.0, 6.0, -2.0);
    let b: Vec<f64> = (0..n).map(|i| 1.0 + 0.3 * (i as f64).sin()).collect();
    let mut config = StagedGmresConfig::new(40, 200, 1.0e-4, 0.0);
    let (plain, _, _) = solve(&a, &b, &config, None);
    assert_eq!(plain.outcome, StagedGmresOutcome::Converged);
    assert!(!plain.exhaustion);
    assert!(plain.columns < n as u64, "the loose target exits early");
    config.small_system_exhaustion = true;
    let (full, x, counters) = solve(&a, &b, &config, None);
    assert!(full.exhaustion);
    assert_eq!(full.outcome, StagedGmresOutcome::Converged);
    assert!(full.columns > plain.columns && full.columns <= n as u64);
    // One confirmation at the end of the space, nothing else.
    assert_eq!(full.confirmations, 1);
    assert_eq!(full.true_residuals, 1);
    assert_eq!(full.cycles, 1);
    assert_eq!(counters.linear_matvecs, full.columns + 1);
    assert_eq!(counters.diagnostic_matvecs, 0);
    let r = residual_norm(&a, &b, &x);
    assert!(r <= 1.0e-12 * full.right_norm, "{r:e}");
    // The end of the space: the round-off floor of the projected residual,
    // a breakdown or the last column.
    assert!(
        full.projected_residual <= 16.0 * f64::EPSILON * full.right_norm
            || full.columns == n as u64,
        "{full:?}"
    );
}

#[test]
fn exhaustion_is_inactive_above_the_restart_length() {
    let n = 60;
    let a = tridiagonal(n, -1.0, 6.0, -2.0);
    let b: Vec<f64> = (0..n).map(|i| 1.0 + 0.3 * (i as f64).sin()).collect();
    for nu_guard in [false, true] {
        let mut config = StagedGmresConfig::new(40, 200, 1.0e-6, 0.0);
        config.nu_guard = nu_guard;
        config.stall_rule = true;
        let plain = solve(&a, &b, &config, None);
        config.small_system_exhaustion = true;
        let exhausted = solve(&a, &b, &config, None);
        assert!(!exhausted.0.exhaustion);
        assert!(same(&exhausted, &plain));
    }
}

/// With the nonnormality guard on, the exhausted solve evaluates `nu` at
/// every column whose projected residual meets the tightened threshold
/// outside the confirmation gap (ALG01's rule), up to the end of the space.
#[test]
fn exhaustion_keeps_the_alg01_nu_rule() {
    let (k, h) = (12, 0.05);
    let n = 8;
    let mut a = DenseMatrix::identity(n);
    for j in 0..n / 2 {
        let lam = 10f64.powi(j as i32);
        let (p, q) = (2 * j, 2 * j + 1);
        a[(p, p)] += 2.0 * h * lam;
        a[(p, q)] -= h * lam * 2f64.powi(k);
        a[(q, p)] -= h * lam * 2f64.powi(-k);
        a[(q, q)] += 2.0 * h * lam;
    }
    let b: Vec<f64> = (0..n).map(|i| 1.0 / (1.0 + i as f64)).collect();
    let mut config = StagedGmresConfig::new(8, 64, 1.0e-3, 0.0);
    config.nu_guard = true;
    let (plain, _, _) = solve(&a, &b, &config, None);
    config.small_system_exhaustion = true;
    let (full, x, _) = solve(&a, &b, &config, None);
    assert!(full.exhaustion);
    assert_eq!(full.outcome, StagedGmresOutcome::Converged);
    assert!(full.nu_evaluations >= plain.nu_evaluations);
    assert!(full.nu_max >= plain.nu_max);
    assert_eq!(full.final_threshold, full.threshold / full.nu_max);
    assert!(full.columns >= plain.columns);
    assert!(residual_norm(&a, &b, &x) <= 1.0001 * full.final_threshold);
}

// -------------------------------------------------------------------- G1

#[test]
fn guard_test_uses_the_given_cycle_length() {
    // q = 0.1, thr / rn = 1e-3: three more cycles.
    let (rn, prev, thr) = (1.0e-3, 1.0e-2, 1.0e-6);
    assert_eq!(
        staged_guard_test(rn, prev, thr, 6, 40, 60),
        Some(StagedGuardAbort::Overrun)
    );
    assert_eq!(staged_guard_test(rn, prev, thr, 6, 3, 60), None);
    assert_eq!(staged_guard_test(rn, prev, thr, 6, 18, 60), None);
    assert_eq!(
        staged_guard_test(rn, prev, thr, 6, 19, 60),
        Some(StagedGuardAbort::Overrun)
    );
    assert_eq!(
        staged_guard_test(0.99, 1.0, thr, 6, 3, 60),
        Some(StagedGuardAbort::Contraction)
    );
}

/// `x -> A x + delta`: the true residual of every iterate keeps an offset,
/// so restarted cycles contract slowly at the restart boundaries.
struct Affine {
    a: DenseMatrix,
    delta: Vec<f64>,
}

impl LinearOperator for Affine {
    fn dimension(&self) -> usize {
        self.a.nrows()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.a.matvec_into(x, y)?;
        for (v, d) in y.iter_mut().zip(&self.delta) {
            *v += d * safe_l2(x);
        }
        Ok(())
    }
    fn token(&self) -> u64 {
        u64::MAX
    }
}

/// On a system smaller than the restart length the nominal prediction
/// charges `restart` columns per cycle where only `n` exist; `G1` charges
/// `n`. Up to the nominal abort both solves are the same, so `G1` never
/// stops earlier, and here it continues where the nominal guard overran.
#[test]
fn effective_cycle_length_removes_the_small_system_overrun() {
    let mut differs = 0;
    for n in [3usize, 4, 6] {
        for scale in [0.05, 0.2, 0.5] {
            let a = tridiagonal(n, -1.0, 4.0, -0.5);
            let delta: Vec<f64> = (0..n)
                .map(|i| scale * (1.0 + i as f64) / n as f64)
                .collect();
            let op = Affine { a, delta };
            let b = vec![1.0; n];
            for budget in [60, 120, 400] {
                let mut config = StagedGmresConfig::new(40, budget, 1.0e-12, 0.0);
                config.stagnation_guard = true;
                let (nominal, _, _) = solve_op(&op, &b, &config, None);
                config.effective_cycle_overrun = true;
                let (effective, _, _) = solve_op(&op, &b, &config, None);
                assert!(effective.columns >= nominal.columns);
                if nominal.guard_abort == Some(StagedGuardAbort::Contraction) {
                    assert!(same(&effective, &nominal));
                }
                if nominal.guard_abort == Some(StagedGuardAbort::Overrun)
                    && effective.columns > nominal.columns
                {
                    differs += 1;
                }
            }
        }
    }
    assert!(differs > 0, "G1 changed at least one overrun abort");
}

// -------------------------------------------------------------------- G2

#[test]
fn floor_accepts_at_a_confirmation_without_a_slow_cycle() {
    let n = 40;
    let a = tridiagonal(n, -1.0, 10.0, -2.0);
    let b: Vec<f64> = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect();
    // A threshold below the attainable accuracy.
    let mut config = StagedGmresConfig::new(40, 80, 1.0e-18, 0.0);
    let (plain, x_plain, _) = solve(&a, &b, &config, None);
    assert_eq!(plain.outcome, StagedGmresOutcome::Failed);
    assert!(x_plain.iter().all(|v| v.is_nan()));
    config.floor_at_confirmations = true;
    let (floor, x, counters) = solve(&a, &b, &config, None);
    assert_eq!(
        floor.outcome,
        StagedGmresOutcome::FloorAccepted,
        "{floor:?}"
    );
    assert!(floor.accepted());
    assert!(floor.confirmations >= 1);
    assert_eq!(floor.cycles, 1, "accepted inside the first cycle");
    assert!(floor.residual_norm > floor.threshold);
    let ax = a.matvec(&x).unwrap();
    let defect = safe_l2(&x.iter().zip(&ax).map(|(p, q)| p - q).collect::<Vec<_>>());
    let bound = STAGED_STALL_FACTOR * (floor.right_norm + safe_l2(&x) + defect);
    assert!(floor.residual_norm <= bound);
    assert!((residual_norm(&a, &b, &x) - floor.residual_norm).abs() <= 1.0e-3 * bound);
    assert_eq!(counters.linear_solves, 1);
    assert_eq!(
        counters.linear_matvecs,
        floor.columns + floor.true_residuals,
        "the floor costs no operator application"
    );
}

#[test]
fn floor_is_inert_where_the_threshold_is_met() {
    let n = 60;
    let a = tridiagonal(n, -1.0, 2.5, -1.2);
    let b = vec![1.0; n];
    let mut config = StagedGmresConfig::new(5, 2000, 1.0e-9, 0.0);
    config.stall_rule = true;
    let plain = solve(&a, &b, &config, None);
    config.floor_at_confirmations = true;
    let floor = solve(&a, &b, &config, None);
    assert_eq!(plain.0.outcome, StagedGmresOutcome::Converged);
    assert!(same(&plain, &floor));
}

// ------------------------------------------------ accepted-abort shadows

fn cyclic_shift(n: usize) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[((i + 1) % n, i)] = 1.0;
    }
    a
}

#[test]
fn classifying_accepted_guard_aborts_changes_nothing_but_the_label() {
    let n = 50;
    let a = cyclic_shift(n);
    let mut b = vec![0.0; n];
    b[0] = 1.0;
    let mut config = StagedGmresConfig::new(10, 100, 1.0e-10, 0.0);
    config.stagnation_guard = true;
    config.classify_guard_aborts = true;
    let mut accept = |_: &[f64], _: &[f64]| true;
    let (plain, x_plain, c_plain) = solve(&a, &b, &config, Some(&mut accept));
    assert_eq!(plain.outcome, StagedGmresOutcome::FallbackAccepted);
    assert_eq!(plain.guard_abort, Some(StagedGuardAbort::Contraction));
    assert_eq!(
        plain.shadow_converged, None,
        "ALG03: accepted aborts unclassified"
    );
    config.classify_accepted_guard_aborts = true;
    let mut accept = |_: &[f64], _: &[f64]| true;
    let (classified, x, counters) = solve(&a, &b, &config, Some(&mut accept));
    assert_eq!(classified.shadow_converged, Some(false));
    assert_eq!(bits(&x), bits(&x_plain), "the accepted iterate is returned");
    assert_eq!(counters, c_plain, "the shadow is not counted");
    assert!(same(
        &StagedGmresReport {
            shadow_converged: None,
            ..classified
        },
        &plain
    ));
}
