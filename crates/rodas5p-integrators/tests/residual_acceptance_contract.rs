//! Re-audit node AS01 (finding F102 of
//! `docs/reviews/20261010_accuracy_speed/REVIEW_KO.md`): the production
//! fallback of the staged stage solves must compare finite quantities only.
//!
//! Before AS01 the driver accepted a fallback iterate iff
//! `||b - W x||_2 <= max(atol_lin, rtol_lin ||b||_2)` on the physical
//! (unscaled) vectors. With finite components whose L2 norms overflow, both
//! sides are `+Inf` and `Inf <= Inf` accepted a relative residual of 0.866
//! (`research/reaudit_accuracy_speed_20261010/NATIVE.json`,
//! `implementation_adversaries/fallback_seam`). The repaired rule is
//! [`ProductionFallbackRule`]; the tests below check
//!
//! 1. the recorded 4x4 callback counterexample is rejected;
//! 2. its `scale = 1e100` finite control behaves as before;
//! 3. the public driver rejects (transactionally, as a linear-solve failure)
//!    a huge-but-finite stage the old predicate accepted through
//!    `Inf <= Inf`, with finite and accurate controls;
//! 4. the decision equals the old one for every input whose old
//!    intermediates are finite (random property test), and a power-of-two
//!    rescaling into overflow never turns a rejection into an acceptance.

use rodas5p_core::{
    DenseMatrix, DenseOperator, InitialGuess, LinearMethod, LinearSolverConfig, WorkCounters,
    rodas5p_coefficients, safe_l2,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, ProductionFallbackRule,
    Rodas5pMfStageTargetResult, StageTargetOptions, StageTargetPolicy,
    integrate_rodas5p_mf_fast_observed_with_stage_target,
};
use rodas5p_krylov::{
    StagedGmresConfig, StagedGmresFailure, StagedGmresOutcome, StagedGmresReport,
    StagedGmresWorkspace, solve_staged_gmres,
};
use std::sync::Arc;

/// The production linear tolerances of the recorded counterexample.
const PRODUCTION_RTOL: f64 = 1.0e-10;
const PRODUCTION_ATOL: f64 = 1.0e-14;

/// The pre-AS01 predicate, literally (`rodas5p_matrix_free_fast.rs:810-811,
/// 843-849` at 4c64317): physical residual against
/// `max(atol, rtol ||b||_2)`, with no finiteness checks.
fn old_accepts(rhs: &[f64], rtol: f64, atol: f64, residual: &[f64], scale: Option<&[f64]>) -> bool {
    let threshold = atol.max(rtol * safe_l2(rhs));
    let unscaled: Vec<f64> = match scale {
        Some(scale) => residual.iter().zip(scale).map(|(r, s)| r * s).collect(),
        None => residual.to_vec(),
    };
    safe_l2(&unscaled) <= threshold
}

// ---------------------------------------------------------------------------
// (1), (2): the recorded staged-solver callback seam.
// ---------------------------------------------------------------------------

struct SeamOutcome {
    report: StagedGmresReport,
    old_decision: bool,
    new_decision: bool,
    stable_relative_residual: f64,
    output: Vec<f64>,
}

/// The `overflow_callback_case` of `examples/reaudit_20261010.rs`: cyclic
/// 4x4 shift, scaled right-hand side `[1.5, 1.5, 0, 0]`, physical scale
/// `scale`, restart = budget = 1, scaled rtol 1e-14; the callback now
/// applies [`ProductionFallbackRule`] (the old predicate is recorded too).
fn cyclic_seam(scale: f64) -> SeamOutcome {
    let mut matrix = DenseMatrix::zeros(4, 4);
    for j in 0..4 {
        matrix[((j + 1) % 4, j)] = 1.0;
    }
    seam(matrix, &[1.5, 1.5, 0.0, 0.0], scale)
}

fn seam(matrix: DenseMatrix, scaled_rhs: &[f64], scale: f64) -> SeamOutcome {
    let n = scaled_rhs.len();
    let op = DenseOperator::new(matrix).expect("finite matrix");
    let physical_rhs: Vec<f64> = scaled_rhs.iter().map(|b| b * scale).collect();
    let weights = vec![scale; n];
    let rule = ProductionFallbackRule::new(&physical_rhs, PRODUCTION_RTOL, PRODUCTION_ATOL);
    let mut seen = None;
    let mut fallback = |z: &[f64], scaled_residual: &[f64]| {
        let old = old_accepts(
            &physical_rhs,
            PRODUCTION_RTOL,
            PRODUCTION_ATOL,
            scaled_residual,
            Some(&weights),
        );
        let new = rule.accepts(z, scaled_residual, Some(&weights));
        let ratio = safe_l2(scaled_residual) / safe_l2(scaled_rhs);
        seen = Some((old, new, ratio));
        new
    };
    let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
    let mut output = vec![f64::NAN; n];
    let report = solve_staged_gmres(
        &op,
        scaled_rhs,
        &config,
        Some(&mut fallback),
        &mut output,
        &mut StagedGmresWorkspace::default(),
        &mut WorkCounters::default(),
    )
    .expect("finite scaled system");
    let (old_decision, new_decision, stable_relative_residual) =
        seen.expect("the fallback is consulted on budget exhaustion");
    SeamOutcome {
        report,
        old_decision,
        new_decision,
        stable_relative_residual,
        output,
    }
}

#[test]
fn recorded_overflow_counterexample_is_rejected() {
    let scale = 1.0e308;
    let physical_rhs: Vec<f64> = [1.5, 1.5, 0.0, 0.0].iter().map(|b| b * scale).collect();
    assert!(physical_rhs.iter().all(|b| b.is_finite()));
    // The recorded preconditions: finite components, overflowing norm and
    // literal threshold.
    assert_eq!(safe_l2(&physical_rhs), f64::INFINITY);
    assert_eq!(
        PRODUCTION_ATOL.max(PRODUCTION_RTOL * safe_l2(&physical_rhs)),
        f64::INFINITY
    );
    let rule = ProductionFallbackRule::new(&physical_rhs, PRODUCTION_RTOL, PRODUCTION_ATOL);
    assert_eq!(rule.literal_threshold(), None);
    assert!(rule.is_representable());

    let seam = cyclic_seam(scale);
    // NATIVE.json: relative residual sqrt(3/4) = 0.8660254037844385.
    assert!((seam.stable_relative_residual - 0.75_f64.sqrt()).abs() < 1.0e-12);
    assert!(seam.old_decision, "the recorded Inf <= Inf acceptance");
    assert!(!seam.new_decision, "AS01: rejected");
    assert_eq!(seam.report.outcome, StagedGmresOutcome::Failed);
    assert_eq!(
        seam.report.failure,
        Some(StagedGmresFailure::BudgetExhausted)
    );
    assert!(seam.report.budget_exhausted);
    assert!(!seam.report.accepted());
    // A failed solve leaves the output untouched.
    assert!(seam.output.iter().all(|x| x.is_nan()));
}

#[test]
fn finite_scale_control_behaves_as_before() {
    let seam = cyclic_seam(1.0e100);
    let physical_rhs: Vec<f64> = [1.5, 1.5, 0.0, 0.0].iter().map(|b| b * 1.0e100).collect();
    let rule = ProductionFallbackRule::new(&physical_rhs, PRODUCTION_RTOL, PRODUCTION_ATOL);
    assert_eq!(
        rule.literal_threshold(),
        Some(PRODUCTION_ATOL.max(PRODUCTION_RTOL * safe_l2(&physical_rhs)))
    );
    assert!(!seam.old_decision);
    assert_eq!(seam.new_decision, seam.old_decision);
    assert_eq!(seam.report.outcome, StagedGmresOutcome::Failed);
    assert_eq!(
        seam.report.failure,
        Some(StagedGmresFailure::BudgetExhausted)
    );
    // The scaled solve is the same system at both scales.
    let overflow = cyclic_seam(1.0e308);
    assert_eq!(
        seam.report.residual_norm.to_bits(),
        overflow.report.residual_norm.to_bits()
    );
}

/// An accurate stage (`diag(1, 1 + 1e-12)`, relative residual about
/// 5e-13 after one column) is accepted at a finite and at an overflowing
/// scale: the homogeneous comparison rejects inaccuracy, not size.
#[test]
fn accurate_seam_is_accepted_at_every_scale() {
    let matrix = DenseMatrix::from_rows(&[&[1.0, 0.0], &[0.0, 1.0 + 1.0e-12]]).unwrap();
    let finite = seam(matrix.clone(), &[1.5, 1.5], 1.0e100);
    let overflow = seam(matrix, &[1.5, 1.5], 1.0e308);
    for outcome in [&finite, &overflow] {
        assert!(outcome.stable_relative_residual > 1.0e-14);
        assert!(outcome.stable_relative_residual < PRODUCTION_RTOL);
        assert!(outcome.old_decision);
        assert!(outcome.new_decision);
        assert_eq!(outcome.report.outcome, StagedGmresOutcome::FallbackAccepted);
    }
    assert_eq!(
        finite
            .output
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>(),
        overflow
            .output
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// (3): the public driver.
// ---------------------------------------------------------------------------

/// The fixed step `h` of the first attempt.
const STEP: f64 = 5.0;

/// `y' = J (y - y_ref) + c` with `J = (I - P) / kappa` (`P` the cyclic shift
/// of the seam, `kappa = STEP gamma`), so the first stage operator is
/// `W = I - STEP gamma J = P` and its right-hand side is `kappa c` at
/// `y0 = y_ref`.
fn cyclic_problem(c: [f64; 4], kappa: f64) -> OdeProblem {
    let shift = move |v: &[f64], out: &mut [f64]| {
        for i in 0..4 {
            out[i] = (v[i] - v[(i + 3) % 4]) / kappa;
        }
    };
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        shift(y, out);
        for (o, ci) in out.iter_mut().zip(&c) {
            *o += ci;
        }
        Ok(())
    });
    let jvp = Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        shift(v, out);
        Ok(())
    });
    OdeProblem::new(
        "as01-cyclic-4",
        4,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

/// `y' = J (y - y_ref) + c` with `J = diag(0, -delta / kappa)`, so
/// `W = diag(1, 1 + delta)` in the first attempt.
fn diagonal_problem(c: [f64; 2], kappa: f64, delta: f64, y_ref: f64) -> OdeProblem {
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = c[0];
        out[1] = c[1] - delta / kappa * (y[1] - y_ref);
        Ok(())
    });
    let jvp = Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = 0.0;
        out[1] = -delta / kappa * v[1];
        Ok(())
    });
    OdeProblem::new(
        "as01-diagonal-2",
        2,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn kappa() -> f64 {
    STEP * rodas5p_coefficients().unwrap().gamma
}

/// One attempt of `h = STEP` with the coupled target (`D = 1e-6 |y| + 1`,
/// uniform for a constant `y0`), restart = budget = 1 and the production
/// fallback on.
fn drive(problem: &OdeProblem, y0: &[f64]) -> Rodas5pMfStageTargetResult {
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: PRODUCTION_RTOL,
        atol: PRODUCTION_ATOL,
        restart: 1,
        maxiter: 1,
        x0_strategy: InitialGuess::Zero,
        ..LinearSolverConfig::default()
    };
    let adaptive = AdaptiveStepConfig {
        atol: 1.0,
        rtol: 1.0e-6,
        initial_step: STEP,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    let options = StageTargetOptions {
        production_fallback: true,
        ..StageTargetOptions::new(StageTargetPolicy::Coupled)
    };
    integrate_rodas5p_mf_fast_observed_with_stage_target(
        problem,
        (0.0, 2.0 * STEP),
        y0,
        &linear,
        &adaptive,
        &OutputSchedule::new(vec![0.0, 2.0 * STEP]).unwrap(),
        options,
    )
    .expect("a failed stage solve is a rejected attempt, not an error")
}

/// The first stage's physical right-hand side `h gamma f(y0) = kappa c`.
fn first_stage_rhs(c: &[f64]) -> Vec<f64> {
    let hg = kappa();
    c.iter().map(|ci| ci * hg).collect()
}

#[test]
fn driver_rejects_overflowing_fallback_transactionally() {
    let kappa = kappa();
    assert!(kappa > 0.9, "the stage operator stays representable");
    let c = [1.5e308 / kappa, 1.5e308 / kappa, 0.0, 0.0];
    let b = first_stage_rhs(&c);
    // The old predicate's threshold overflows: it would have accepted any
    // residual of this stage (`Inf <= Inf`).
    assert!(b.iter().all(|v| v.is_finite()));
    assert_eq!(safe_l2(&b), f64::INFINITY);
    assert_eq!(
        ProductionFallbackRule::new(&b, PRODUCTION_RTOL, 1.0e-14).literal_threshold(),
        None
    );
    let y0 = [1.0e306; 4];
    let run = drive(&cyclic_problem(c, kappa), &y0);
    let stats = run.statistics;
    let observed = &run.result.observed;
    assert_eq!(stats.solves, 1);
    assert_eq!(stats.budget_exhausted, 1);
    assert_eq!(stats.fallback_accepted, 0, "no Inf <= Inf acceptance");
    assert_eq!(stats.failed, 1);
    assert_eq!(stats.failed_budget, 1);
    assert!(run.charges.is_empty());
    // Transactional: the attempt fails as a linear solve and is rejected.
    assert_eq!(run.result.attempts, 1);
    assert_eq!(run.result.accepted_steps, 0);
    assert_eq!(run.result.rejected_steps, 1);
    assert_eq!(observed.counters.linear_solve_failures, 1);
    assert_eq!(observed.counters.nonfinite_step_failures, 0);
    assert!(!observed.success);
    assert_eq!(observed.y, vec![y0.to_vec()]);
}

#[test]
fn driver_finite_control_is_unchanged() {
    // The same stage at a finite scale: old and new rules reject the
    // relative residual sqrt(3/4) alike (literal branch).
    let kappa = kappa();
    let c = [1.5e100 / kappa, 1.5e100 / kappa, 0.0, 0.0];
    let b = first_stage_rhs(&c);
    let rule = ProductionFallbackRule::new(&b, PRODUCTION_RTOL, 1.0e-14);
    assert!(rule.literal_threshold().is_some_and(f64::is_finite));
    let run = drive(&cyclic_problem(c, kappa), &[1.0e98; 4]);
    let stats = run.statistics;
    assert_eq!(stats.solves, 1);
    assert_eq!(stats.fallback_accepted, 0);
    assert_eq!(stats.failed_budget, 1);
    assert_eq!(run.result.observed.counters.linear_solve_failures, 1);
    assert_eq!(run.result.accepted_steps, 0);
}

#[test]
fn driver_accepts_accurate_fallback_at_finite_and_overflowing_scale() {
    // W = diag(1, 1 + 1e-12): one column leaves a relative residual near
    // 5e-13, above the coupled target, below rtol_lin = 1e-10. The first
    // stage is fallback-accepted at both scales (the overflowing one now
    // through the homogeneous comparison).
    let kappa = kappa();
    for (magnitude, y_ref) in [(1.5e100, 1.0e98), (1.5e308, 1.0e306)] {
        let c = [magnitude / kappa, magnitude / kappa];
        let b = first_stage_rhs(&c);
        let rule = ProductionFallbackRule::new(&b, PRODUCTION_RTOL, 1.0e-14);
        assert_eq!(
            rule.literal_threshold().is_some(),
            safe_l2(&b).is_finite(),
            "{magnitude:e}"
        );
        let run = drive(&diagonal_problem(c, kappa, 1.0e-12, y_ref), &[y_ref; 2]);
        assert!(
            run.statistics.fallback_accepted >= 1,
            "{magnitude:e}: {:?}",
            run.statistics
        );
    }
}

// ---------------------------------------------------------------------------
// (4): decision equivalence and rescaling.
// ---------------------------------------------------------------------------

/// SplitMix64 (no new dependency).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    /// `+-m 10^e`, `e` uniform in `[lo, hi]`, `m` in [1, 10).
    fn log_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        let e = lo + (hi - lo) * self.unit();
        let sign = if self.below(2) == 0 { 1.0 } else { -1.0 };
        sign * (1.0 + 9.0 * self.unit()) * 10f64.powf(e.floor())
    }
}

struct Case {
    rhs: Vec<f64>,
    rtol: f64,
    atol: f64,
    candidate: Vec<f64>,
    residual: Vec<f64>,
    scale: Option<Vec<f64>>,
}

fn random_case(rng: &mut Rng) -> Case {
    let n = 1 + rng.below(8) as usize;
    let (lo, hi) = match rng.below(4) {
        0 => (-320.0, -280.0),
        1 => (280.0, 308.0),
        _ => (-300.0, 300.0),
    };
    let rhs: Vec<f64> = (0..n)
        .map(|_| {
            if rng.below(6) == 0 {
                0.0
            } else {
                rng.log_uniform(lo, hi)
            }
        })
        .collect();
    let rtol = match rng.below(5) {
        0 => 0.0,
        1 => rng.log_uniform(0.0, 3.0).abs(),
        _ => rng.log_uniform(-16.0, -1.0).abs(),
    };
    let atol = match rng.below(4) {
        0 => 0.0,
        1 => rng.log_uniform(-320.0, -280.0).abs(),
        _ => rng.log_uniform(-300.0, 300.0).abs(),
    };
    let scale = (rng.below(2) == 0).then(|| {
        (0..n)
            .map(|_| rng.log_uniform(-150.0, 150.0).abs())
            .collect::<Vec<f64>>()
    });
    // Physical residual: random, or a multiple of the threshold near 1
    // (boundary cases, including exact multiples of the right-hand side).
    let threshold = atol.max(rtol * safe_l2(&rhs));
    let physical: Vec<f64> = match rng.below(4) {
        0 => (0..n).map(|_| rng.log_uniform(-320.0, 308.0)).collect(),
        1 => rhs.iter().map(|b| b * rtol).collect(),
        _ => {
            let direction: Vec<f64> = (0..n).map(|_| rng.log_uniform(-3.0, 0.0)).collect();
            let norm = safe_l2(&direction);
            let factor = 1.0 + (rng.unit() - 0.5) * 1.0e-15 * rng.below(3) as f64;
            direction
                .iter()
                .map(|d| d / norm * threshold * factor)
                .collect()
        }
    };
    let residual = match &scale {
        Some(scale) => physical.iter().zip(scale).map(|(r, s)| r / s).collect(),
        None => physical,
    };
    let candidate = (0..n).map(|_| rng.log_uniform(-100.0, 100.0)).collect();
    Case {
        rhs,
        rtol,
        atol,
        candidate,
        residual,
        scale,
    }
}

/// Every intermediate of the old formula is finite (and the physical
/// candidate, which the driver writes as the stage, too).
fn old_intermediates_finite(case: &Case) -> bool {
    let rhs_norm = safe_l2(&case.rhs);
    let threshold = case.atol.max(case.rtol * rhs_norm);
    let unscaled: Vec<f64> = match &case.scale {
        Some(scale) => case
            .residual
            .iter()
            .zip(scale)
            .map(|(r, s)| r * s)
            .collect(),
        None => case.residual.clone(),
    };
    let candidate_finite = match &case.scale {
        Some(scale) => case
            .candidate
            .iter()
            .zip(scale)
            .all(|(z, s)| (z * s).is_finite()),
        None => case.candidate.iter().all(|z| z.is_finite()),
    };
    rhs_norm.is_finite()
        && threshold.is_finite()
        && unscaled.iter().all(|r| r.is_finite())
        && safe_l2(&unscaled).is_finite()
        && candidate_finite
}

#[test]
fn decision_matches_old_rule_wherever_old_intermediates_are_finite() {
    let mut rng = Rng(0x0a50_1f10_2026_1010);
    let (mut compared, mut accepted, mut rejected, mut excluded) = (0_u64, 0_u64, 0_u64, 0_u64);
    let mut boundary_equal = 0_u64;
    for _ in 0..400_000 {
        let case = random_case(&mut rng);
        if !old_intermediates_finite(&case) {
            excluded += 1;
            continue;
        }
        let scale = case.scale.as_deref();
        let old = old_accepts(&case.rhs, case.rtol, case.atol, &case.residual, scale);
        let rule = ProductionFallbackRule::new(&case.rhs, case.rtol, case.atol);
        assert!(rule.literal_threshold().is_some());
        let new = rule.accepts(&case.candidate, &case.residual, scale);
        assert_eq!(
            old, new,
            "rhs {:?} rtol {:e} atol {:e} residual {:?} scale {:?}",
            case.rhs, case.rtol, case.atol, case.residual, case.scale
        );
        compared += 1;
        if new {
            accepted += 1;
        } else {
            rejected += 1;
        }
        let unscaled: Vec<f64> = match scale {
            Some(s) => case.residual.iter().zip(s).map(|(r, s)| r * s).collect(),
            None => case.residual.clone(),
        };
        if safe_l2(&unscaled) == case.atol.max(case.rtol * safe_l2(&case.rhs)) {
            boundary_equal += 1;
        }
    }
    println!(
        "compared {compared}, accepted {accepted}, rejected {rejected}, \
         exact-boundary {boundary_equal}, excluded (overflow) {excluded}"
    );
    assert!(compared > 200_000);
    assert!(accepted > 50_000 && rejected > 50_000);
    assert!(boundary_equal > 100);
    assert!(excluded > 1_000);
}

/// A uniform rescaling by `2^k` that makes the norms overflow: with no
/// absolute floor the decision is that of the unscaled problem, and with
/// one a rejection stays a rejection.
#[test]
fn power_of_two_rescaling_into_overflow_never_creates_acceptance() {
    let mut rng = Rng(0x0a50_2f10_2026_1010);
    let (mut overflowing, mut accepted, mut rejected) = (0_u64, 0_u64, 0_u64);
    for _ in 0..100_000 {
        let n = 1 + rng.below(8) as usize;
        // Comparable components, so the rescaled norm usually overflows.
        let magnitude = rng.log_uniform(-40.0, 40.0).abs();
        let rhs: Vec<f64> = (0..n)
            .map(|_| magnitude * rng.log_uniform(-1.0, -1.0))
            .collect();
        let rtol = rng.log_uniform(-14.0, -1.0).abs();
        let atol = if rng.below(2) == 0 {
            0.0
        } else {
            rng.log_uniform(-60.0, 60.0).abs()
        };
        let threshold = rtol * safe_l2(&rhs);
        let residual: Vec<f64> = (0..n)
            .map(|_| rng.log_uniform(-3.0, 0.0) * threshold * (0.1 + 4.0 * rng.unit()))
            .collect();
        let candidate = vec![1.0; n];
        // The unscaled problem is finite: the old rule decides it.
        let base = old_accepts(&rhs, rtol, atol, &residual, None);
        assert_eq!(
            base,
            ProductionFallbackRule::new(&rhs, rtol, atol).accepts(&candidate, &residual, None)
        );
        // 2^k (k > 0) with the largest component in [2^1023, 2^1024).
        let max = rhs.iter().fold(0.0_f64, |m, b| m.max(b.abs()));
        let k = 1023 - max.log2().floor() as i32;
        assert!(k > 0);
        let up = |x: &f64| x * 2.0_f64.powi(k - k / 2) * 2.0_f64.powi(k / 2);
        let rhs_up: Vec<f64> = rhs.iter().map(up).collect();
        let residual_up: Vec<f64> = residual.iter().map(up).collect();
        if !(rhs_up.iter().chain(&residual_up).all(|v| v.is_finite())) {
            continue;
        }
        let rule = ProductionFallbackRule::new(&rhs_up, rtol, atol);
        let scaled = rule.accepts(&candidate, &residual_up, None);
        if rule.literal_threshold().is_none() {
            overflowing += 1;
        }
        if atol == 0.0 {
            assert_eq!(base, scaled, "rhs {rhs:?} residual {residual:?} k {k}");
        } else if !base {
            assert!(!scaled, "rhs {rhs:?} residual {residual:?} k {k}");
        }
        if scaled {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    println!("overflowing {overflowing}, accepted {accepted}, rejected {rejected}");
    assert!(overflowing > 10_000);
    assert!(accepted > 1_000 && rejected > 1_000);
}

#[test]
fn unrepresentable_inputs_reject() {
    let candidate = [1.0, 1.0];
    let residual = [0.0, 0.0];
    for rhs in [
        [f64::INFINITY, 1.0],
        [f64::NAN, 1.0],
        [f64::NEG_INFINITY, 0.0],
    ] {
        let rule = ProductionFallbackRule::new(&rhs, PRODUCTION_RTOL, PRODUCTION_ATOL);
        assert!(!rule.is_representable());
        assert!(!rule.accepts(&candidate, &residual, None));
    }
    // A relative threshold that overflows even after the scaling by s.
    let rule = ProductionFallbackRule::new(&[1.5e308, 1.5e308], f64::MAX, 0.0);
    assert!(!rule.is_representable());
    assert!(!rule.accepts(&candidate, &residual, None));
    // A finite threshold, non-finite residual, candidate or physical
    // candidate.
    let rule = ProductionFallbackRule::new(&[1.0, 1.0], PRODUCTION_RTOL, PRODUCTION_ATOL);
    assert!(rule.accepts(&candidate, &residual, None));
    assert!(!rule.accepts(&candidate, &[f64::NAN, 0.0], None));
    assert!(!rule.accepts(&candidate, &[f64::INFINITY, 0.0], None));
    assert!(!rule.accepts(&[f64::INFINITY, 0.0], &residual, None));
    assert!(!rule.accepts(&[1.0e300, 0.0], &residual, Some(&[1.0e10, 1.0])));
    assert!(!rule.accepts(&candidate, &[1.0e300, 0.0], Some(&[1.0e10, 1.0])));
    // An overflowing threshold: residual norm overflow with finite
    // components is decided on the scaled vectors, never as Inf <= Inf.
    let rhs = [1.5e308, 1.5e308];
    let rule = ProductionFallbackRule::new(&rhs, PRODUCTION_RTOL, PRODUCTION_ATOL);
    assert_eq!(rule.literal_threshold(), None);
    assert!(rule.is_representable());
    assert!(!rule.accepts(&candidate, &[1.5e308, 1.5e308], None));
    assert!(rule.accepts(&candidate, &[1.5e297, -1.5e297], None));
    assert!(!rule.accepts(&candidate, &[1.5e299, -1.5e299], None));
    assert!(rule.accepts(&candidate, &[1.0e-15, 0.0], None));
}
