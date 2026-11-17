//! Contracts of loop node V17 (matrix-free stage solve): the threshold of
//! arm B is the protected driver's, the step-tied tolerance of arm C is the
//! declared formula, every arm reproduces itself, and the residual checks
//! see every solve without changing a run. Development instances only; the
//! declared cells are run by `stage_clients_krylov_v17_measure`.

#[path = "stage_clients_krylov_v17_problems.rs"]
mod problems;

use std::sync::Arc;

use problems::{
    Arm, ArmOptions, ArmRun, DEVELOPMENT, Family, ProtectedAudit, adaptive, declared, run_arm,
};
use rodas5p_core::{
    CoreResult, IdentityPreconditioner, LinearOperator, ShiftedOperator, WorkCounters,
    rodas5p_coefficients, safe_l2,
};
use rodas5p_integrators::stage_clients::krylov::{
    KrylovPreconditioner, KrylovStageClient, PerStageGmresOptions, ResidualAuditClient,
    StageStoppingRule, StepTiedTolerance,
};
use rodas5p_integrators::{
    OdeProblem, OutputSchedule, Rodas5pMfFastWorkspace, RoutingOptions, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol, raw_absolute_residual_budget,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresIntoOptions, GmresWorkspace, ResidualAccounting,
    solve_gmres_into_with_accounting,
};

const TOLERANCES: [f64; 2] = [1.0e-4, 1.0e-6];

fn pseudo_random(seed: u64, n: usize) -> Vec<f64> {
    let mut state = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
        })
        .collect()
}

fn rhs(problem: &OdeProblem, y: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; y.len()];
    problem
        .eval_rhs_into(0.0, y, &mut out, &mut WorkCounters::default())
        .unwrap();
    out
}

fn jvp(problem: &OdeProblem, y: &[f64], v: &[f64]) -> Vec<f64> {
    let operator = problem.linearize_matrix_free(0.0, y).unwrap();
    let mut out = vec![0.0; y.len()];
    operator.apply(v, &mut out).unwrap();
    out
}

fn same_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

fn assert_same_run(first: &ArmRun, second: &ArmRun, what: &str) {
    assert_eq!(first.success, second.success, "{what}: success");
    assert_eq!(first.attempts, second.attempts, "{what}: attempts");
    assert_eq!(first.accepted, second.accepted, "{what}: accepted");
    assert_eq!(first.rejected, second.rejected, "{what}: rejected");
    assert_eq!(first.counters, second.counters, "{what}: counters");
    assert_eq!(
        first.t_end.to_bits(),
        second.t_end.to_bits(),
        "{what}: t_end"
    );
    assert!(
        same_bits(&first.y_end, &second.y_end),
        "{what}: final state"
    );
    assert_eq!(first.stats, second.stats, "{what}: client statistics");
}

fn with_audit(arm: Arm) -> ArmOptions {
    ArmOptions {
        audit: true,
        ..ArmOptions::new(arm)
    }
}

/// The linear problem `y' = diag(d) y`.
fn diagonal_problem(d: Vec<f64>) -> OdeProblem {
    let n = d.len();
    let (d_rhs, d_jvp) = (d.clone(), d);
    OdeProblem::new(
        "diagonal".to_string(),
        n,
        Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
            for ((o, v), di) in out.iter_mut().zip(y).zip(&d_rhs) {
                *o = di * v;
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(
            move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
                for ((o, vi), di) in out.iter_mut().zip(v).zip(&d_jvp) {
                    *o = di * vi;
                }
                Ok(())
            },
        )),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn v17_family_jvps_and_diagonals_match_the_right_hand_side() {
    for id in DEVELOPMENT {
        let family = declared(id).unwrap();
        let n = family.problem.dimension;
        assert_eq!(family.y0.len(), n, "{id}");
        assert!(family.problem.autonomous, "{id}");
        for (label, state) in [
            ("start", family.y0.clone()),
            (
                "perturbed",
                family
                    .y0
                    .iter()
                    .zip(pseudo_random(7, n))
                    .map(|(y, r)| y + 0.05 * r)
                    .collect::<Vec<_>>(),
            ),
        ] {
            let direction = pseudo_random(11, n);
            let analytic = jvp(&family.problem, &state, &direction);
            let step = 1.0e-6;
            let plus: Vec<f64> = state
                .iter()
                .zip(&direction)
                .map(|(y, d)| y + step * d)
                .collect();
            let minus: Vec<f64> = state
                .iter()
                .zip(&direction)
                .map(|(y, d)| y - step * d)
                .collect();
            let (f_plus, f_minus) = (rhs(&family.problem, &plus), rhs(&family.problem, &minus));
            let scale = analytic.iter().fold(1.0_f64, |m, v| m.max(v.abs()));
            let worst = analytic
                .iter()
                .zip(f_plus.iter().zip(&f_minus))
                .map(|(a, (p, m))| (a - (p - m) / (2.0 * step)).abs())
                .fold(0.0_f64, f64::max);
            assert!(
                worst <= 1.0e-6 * scale,
                "{id} {label}: JVP differs from the central difference by {worst:e} (scale {scale:e})"
            );
            let mut diagonal = vec![0.0; n];
            (family.diagonal)(0.0, &state, &mut diagonal).unwrap();
            for i in 0..n {
                let mut unit = vec![0.0; n];
                unit[i] = 1.0;
                let column = jvp(&family.problem, &state, &unit);
                let tolerance = 1.0e-12 * column[i].abs().max(1.0);
                assert!(
                    (column[i] - diagonal[i]).abs() <= tolerance,
                    "{id} {label}: diagonal entry {i}: {} against {}",
                    diagonal[i],
                    column[i]
                );
            }
        }
    }
}

#[test]
fn the_routed_configuration_has_the_constants_of_arm_b() {
    let linear = RoutingOptions::default().matrix_free;
    let rule = StageStoppingRule::protected_production().unwrap();
    let gmres = PerStageGmresOptions::baseline();
    assert_eq!(format!("{:?}", linear.method), "Gmres");
    assert_eq!(format!("{:?}", linear.preconditioner), "None");
    assert_eq!(format!("{:?}", linear.x0_strategy), "Zero");
    assert_eq!(linear.rtol.to_bits(), rule.rtol().to_bits());
    assert_eq!(linear.atol.to_bits(), rule.atol().to_bits());
    assert_eq!(linear.rtol, 1.0e-10);
    assert_eq!(linear.atol, 1.0e-14);
    assert_eq!(linear.restart, gmres.restart);
    assert_eq!(linear.maxiter.max(linear.restart), gmres.max_arnoldi);
    assert!(gmres.in_cycle_exit && !gmres.recompute_final);
    // Arm B and its audit are built with this rule and these options.
    let family = declared("dev_ac3d").unwrap();
    let client = KrylovStageClient::per_stage(
        family.problem.dimension,
        rule,
        ArmOptions::new(Arm::PerStage).gmres(),
        KrylovPreconditioner::None,
    )
    .unwrap();
    assert_eq!(client.rule(), rule);
    assert!(client.step_tied_tolerance().is_none());
    assert_eq!(ArmOptions::new(Arm::PerStage).gmres(), gmres);
}

/// The protected threshold of the scaled system `(I - h gamma J) U = rhs`
/// (`rodas5p_matrix_free_fast.rs:2787-2791`, `2924-2929`;
/// `rodas5p-krylov/src/common.rs:39-47`).
fn protected_scaled_threshold(rhs_norm: f64) -> f64 {
    let linear = RoutingOptions::default().matrix_free;
    let gamma = rodas5p_coefficients().unwrap().gamma;
    let linear_atol = raw_absolute_residual_budget(linear.atol, gamma).unwrap();
    linear_atol.max(linear.rtol * rhs_norm)
}

#[test]
fn the_client_threshold_is_the_protected_threshold() {
    let gamma = rodas5p_coefficients().unwrap().gamma;
    let rule = StageStoppingRule::protected_production().unwrap();
    let samples = pseudo_random(3, 4000);
    let (mut relative_branch, mut absolute_branch) = (0, 0);
    for pair in samples.chunks_exact(2) {
        // h from 1e-12 to 1e2, ||rhs|| from 1e-12 to 1e6.
        let h = 10f64.powf(-5.0 + 7.0 * pair[0]);
        let rhs_norm = 10f64.powf(-3.0 + 9.0 * pair[1]);
        let hg = h * gamma;
        let protected = protected_scaled_threshold(rhs_norm);
        // The client is handed b = rhs / (h gamma).
        let client = rule.threshold(1.0 / hg, rhs_norm / hg).unwrap();
        let scaled_back = client * hg;
        if protected > RoutingOptions::default().matrix_free.rtol * rhs_norm {
            absolute_branch += 1;
        } else {
            relative_branch += 1;
        }
        let ulps = ((scaled_back - protected) / protected).abs() / f64::EPSILON;
        assert!(
            ulps <= 8.0,
            "h {h:e}, ||rhs|| {rhs_norm:e}: client {scaled_back:e} against protected {protected:e} \
             ({ulps} ulp)"
        );
    }
    assert!(relative_branch > 100 && absolute_branch > 100);
}

/// A 2x2 diagonal stage system whose residual after one Arnoldi column is
/// `factor` times `threshold_of(||rhs||)`: `rhs = beta (1, 1)`,
/// `I - h gamma J = diag(s, s (1 + eps))`.
struct Probe {
    problem: OdeProblem,
    h: f64,
    rhs: Vec<f64>,
}

fn probe(beta: f64, target_ratio: f64) -> Probe {
    let gamma = rodas5p_coefficients().unwrap().gamma;
    let h = 0.37;
    let hg = h * gamma;
    // rho_1 / ||rhs|| = |s1 - s2| / (sqrt(2) sqrt(s1^2 + s2^2)), about
    // eps / 2 for s2 = s1 (1 + eps).
    let eps = 2.0 * target_ratio;
    let s1: f64 = 3.0;
    let s2 = s1 * (1.0 + eps);
    // s = 1 - h gamma d.
    let d = vec![(1.0 - s1) / hg, (1.0 - s2) / hg];
    Probe {
        problem: diagonal_problem(d),
        h,
        rhs: vec![beta, beta],
    }
}

/// Arnoldi columns the protected kernel needs with one column per pass, so
/// that it tests its true residual after every column.
fn kernel_columns(probe: &Probe) -> u64 {
    let gamma = rodas5p_coefficients().unwrap().gamma;
    let linear = RoutingOptions::default().matrix_free;
    let jacobian = probe
        .problem
        .linearize_matrix_free(0.0, &[0.0, 0.0])
        .unwrap();
    let shifted = ShiftedOperator::new_counted_jvp(None, jacobian, probe.h, gamma).unwrap();
    let config = GmresConfig {
        restart: 1,
        max_arnoldi: linear.maxiter,
        rtol: linear.rtol,
        atol: raw_absolute_residual_budget(linear.atol, gamma).unwrap(),
    };
    let mut output = vec![0.0; 2];
    let report = solve_gmres_into_with_accounting(
        &shifted,
        &IdentityPreconditioner::new(2),
        &probe.rhs,
        None,
        &config,
        None,
        &mut output,
        &mut GmresWorkspace::default(),
        GmresCapacity::unbounded(),
        GmresIntoOptions::default(),
        ResidualAccounting::DEFAULT,
        &mut WorkCounters::default(),
    )
    .unwrap();
    report.iterations
}

/// Arnoldi columns of one solve of the client on the same system in its
/// unscaled form, and whether the policy bound was the threshold.
fn client_columns(
    probe: &Probe,
    policy: Option<(StepTiedTolerance, &[f64])>,
) -> (u64, u64, Vec<f64>) {
    let gamma = rodas5p_coefficients().unwrap().gamma;
    let rule = StageStoppingRule::protected_production().unwrap();
    let mut client = KrylovStageClient::per_stage(
        2,
        rule,
        PerStageGmresOptions::baseline(),
        KrylovPreconditioner::None,
    )
    .unwrap();
    let mut state = vec![0.0, 0.0];
    if let Some((policy, y)) = policy {
        client = client.with_step_tied_tolerance(policy);
        state.copy_from_slice(y);
    }
    let mut counters = WorkCounters::default();
    let inv = 1.0 / (probe.h * gamma);
    client
        .linearize(&probe.problem, 0.0, &state, &mut counters)
        .unwrap();
    client.factor(inv, &mut counters).unwrap();
    let mut b: Vec<f64> = probe.rhs.iter().map(|v| v * inv).collect();
    client.solve_in_place(&mut b, &mut counters).unwrap();
    let stats = client.stats();
    (stats.krylov_vectors, stats.policy_bound_solves, b)
}

#[test]
fn arm_b_and_the_protected_kernel_stop_at_the_same_residual() {
    let linear = RoutingOptions::default().matrix_free;
    // beta = 1: the relative term decides; beta = 1e-6: gamma atol decides.
    for beta in [1.0, 1.0e-6] {
        let rhs_norm = beta * 2.0_f64.sqrt();
        let threshold = protected_scaled_threshold(rhs_norm);
        let relative_decides = threshold == linear.rtol * rhs_norm;
        assert_eq!(relative_decides, beta == 1.0);
        for (factor, expected) in [(0.99, 1), (1.01, 2)] {
            let probe = probe(beta, factor * threshold / rhs_norm);
            let kernel = kernel_columns(&probe);
            let (client, policy_bound, _) = client_columns(&probe, None);
            assert_eq!(
                kernel, expected,
                "beta {beta:e}: the protected kernel at {factor} of its threshold"
            );
            assert_eq!(
                client, expected,
                "beta {beta:e}: arm B at {factor} of the protected threshold"
            );
            assert_eq!(policy_bound, 0);
        }
    }
}

#[test]
fn the_step_tied_bound_is_the_declared_formula() {
    assert_eq!(StepTiedTolerance::KAPPA, 1.0e-3);
    let policy = StepTiedTolerance::declared(1.0e-5, 2.0e-7).unwrap();
    assert_eq!(
        (policy.kappa(), policy.rtol(), policy.atol()),
        (1.0e-3, 1.0e-5, 2.0e-7)
    );
    let y = [3.0, -0.5, 0.25, -8.0];
    // min_i (atol + rtol |y_i|) is attained at |y| = 0.25.
    let floor = policy.weight_floor(&y).unwrap();
    assert_eq!(floor, 2.0e-7 + 1.0e-5 * 0.25);
    assert_eq!(policy.weight_floor(&[]).unwrap(), 2.0e-7);
    let inv = 123.0;
    let bound = policy.bound(4, floor, inv).unwrap();
    assert_eq!(bound, 1.0e-3 * 4.0_f64.sqrt() * floor * inv);
    // The bound implies ||h gamma r||_WRMS <= kappa: a residual of 2-norm
    // `bound` in any single component has a weighted RMS norm of at most
    // kappa / (h gamma).
    for (i, yi) in y.iter().enumerate() {
        let weight = 1.0 / (2.0e-7 + 1.0e-5 * yi.abs());
        let wrms = (bound * weight) / 4.0_f64.sqrt();
        assert!(
            wrms <= 1.0e-3 * inv * (1.0 + 1.0e-14),
            "component {i}: {wrms:e}"
        );
    }
    assert!(StepTiedTolerance::new(-1.0, 1.0e-4, 1.0e-4).is_err());
    assert!(StepTiedTolerance::new(1.0e-3, f64::NAN, 1.0e-4).is_err());
    assert!(StepTiedTolerance::new(1.0e-3, 1.0e-4, f64::INFINITY).is_err());
    assert!(policy.weight_floor(&[1.0, f64::NAN]).is_err());
    assert!(policy.bound(4, f64::MAX, f64::MAX).is_err());
}

#[test]
fn arm_c_accepts_at_the_larger_of_the_two_thresholds() {
    let gamma = rodas5p_coefficients().unwrap().gamma;
    let beta = 1.0;
    let rhs_norm = beta * 2.0_f64.sqrt();
    let protected = protected_scaled_threshold(rhs_norm);
    // Step tolerances for which the bound of the scaled system,
    // kappa sqrt(n) min_i (atol + rtol |y_i|), is 1e-6: far above the
    // protected 1.4e-10.
    let state = [0.5, -2.0];
    let loose = StepTiedTolerance::declared(1.0e-3, 2.0e-4).unwrap();
    let scaled_bound = 1.0e-3 * 2.0_f64.sqrt() * (2.0e-4 + 1.0e-3 * 0.5);
    assert!(scaled_bound > 1.0e3 * protected);
    for (factor, expected) in [(0.99, 1), (1.01, 2)] {
        // A first-column residual at `factor` of the policy bound.
        let probe = probe(beta, factor * scaled_bound / rhs_norm);
        let (columns, policy_bound, _) = client_columns(&probe, Some((loose, &state)));
        assert_eq!(columns, expected, "arm C at {factor} of its bound");
        assert_eq!(policy_bound, 1);
        // Arm B needs the second column in both cases, and so does the
        // protected kernel.
        assert_eq!(client_columns(&probe, None).0, 2);
        assert_eq!(kernel_columns(&probe), 2);
    }
    // A bound below the protected threshold changes nothing: same columns,
    // same solution bits, and the policy is not the threshold.
    let tight = StepTiedTolerance::declared(1.0e-12, 1.0e-12).unwrap();
    for factor in [0.99, 1.01] {
        let probe = probe(beta, factor * protected / rhs_norm);
        let (b_columns, _, b_solution) = client_columns(&probe, None);
        let (c_columns, policy_bound, c_solution) = client_columns(&probe, Some((tight, &state)));
        assert_eq!(b_columns, c_columns);
        assert_eq!(policy_bound, 0);
        assert!(same_bits(&b_solution, &c_solution));
    }
    // A right-hand side below the bound is solved by the zero iterate
    // without a JVP.
    let tiny = probe(1.0e-9, 0.25);
    assert!(1.0e-9 * 2.0_f64.sqrt() < scaled_bound);
    let (columns, policy_bound, solution) = client_columns(&tiny, Some((loose, &state)));
    assert_eq!((columns, policy_bound), (0, 1));
    assert!(solution.iter().all(|value| *value == 0.0));
    let _ = gamma;
}

#[test]
fn every_arm_reproduces_itself_bit_for_bit() {
    for id in DEVELOPMENT {
        let family = declared(id).unwrap();
        for rtol in TOLERANCES {
            for arm in Arm::ALL {
                let options = ArmOptions::new(arm);
                let first = run_arm(&family, rtol, &options).unwrap();
                let second = run_arm(&family, rtol, &options).unwrap();
                let what = format!("{id} rtol {rtol:e} arm {}", arm.label());
                assert!(first.success, "{what}: did not reach the end");
                assert_same_run(&first, &second, &what);
            }
        }
    }
}

#[test]
fn the_checks_cover_every_solve_and_leave_the_runs_unchanged() {
    let mut policy_bound_solves = 0;
    for id in DEVELOPMENT {
        let family = declared(id).unwrap();
        for rtol in TOLERANCES {
            for arm in Arm::ALL {
                let what = format!("{id} rtol {rtol:e} arm {}", arm.label());
                let plain = run_arm(&family, rtol, &ArmOptions::new(arm)).unwrap();
                let audited = run_arm(&family, rtol, &with_audit(arm)).unwrap();
                assert_same_run(&plain, &audited, &what);
                let audit = audited.audit.expect("audit");
                assert!(plain.audit.is_none());
                assert_eq!(audit.failures, 0, "{what}: {audit:?}");
                assert_eq!(audit.zero_threshold_failures, 0, "{what}");
                assert_eq!(audit.first_failure, 0, "{what}");
                assert!(
                    audit.max_ratio <= 1.0 && audit.max_ratio > 0.0,
                    "{what}: largest residual/threshold {}",
                    audit.max_ratio
                );
                // One JVP per checked solve, never charged to the run.
                assert_eq!(audit.jvp_calls, audit.checks, "{what}");
                match arm {
                    Arm::Protected => {
                        // The kernel books `linear_solves` per completed
                        // solve; the check rebuilds one right-hand side
                        // for each.
                        assert_eq!(audit.checks, audited.counters.linear_solves, "{what}");
                        assert_eq!(audit.rhs_calls, audit.checks, "{what}");
                        let log = audited.solve_log.as_ref().expect("solve log");
                        assert_eq!(log.solves, audit.checks, "{what}");
                        // The counter also holds the columns of solves that
                        // exhausted their budget; the log does not.
                        assert!(
                            log.iterations <= audited.counters.linear_iterations,
                            "{what}"
                        );
                        if audited.counters.linear_solve_failures == 0 {
                            assert_eq!(
                                log.iterations, audited.counters.linear_iterations,
                                "{what}"
                            );
                        }
                    }
                    _ => {
                        let stats = audited.stats.expect("client statistics");
                        assert_eq!(audit.checks, stats.solves - stats.failed_solves, "{what}");
                        assert_eq!(audit.rhs_calls, 0, "{what}");
                        assert_eq!(
                            stats.krylov_vectors + stats.residual_jvps,
                            audited.counters.jvp_calls,
                            "{what}: JVP accounting"
                        );
                        if arm == Arm::StepTied {
                            policy_bound_solves += stats.policy_bound_solves;
                        } else {
                            assert_eq!(stats.policy_bound_solves, 0, "{what}");
                        }
                    }
                }
                assert!(audit.checks >= 8, "{what}: {} checks", audit.checks);
            }
        }
    }
    // The policy was the threshold somewhere, so arm C was really tested
    // against its own threshold.
    assert!(policy_bound_solves > 0);
}

/// Arm C's client under an audit with or without the policy.
fn step_tied_run_with_audit(family: &Family, rtol: f64, audit_knows_policy: bool) -> (u64, u64) {
    let config = adaptive(family, rtol);
    let n = family.problem.dimension;
    let rule = StageStoppingRule::protected_production().unwrap();
    let policy = StepTiedTolerance::declared(config.rtol, config.atol).unwrap();
    let client = KrylovStageClient::per_stage(
        n,
        rule,
        PerStageGmresOptions::baseline(),
        KrylovPreconditioner::None,
    )
    .unwrap()
    .with_step_tied_tolerance(policy);
    let mut audited = ResidualAuditClient::new(client, rule);
    if audit_knows_policy {
        audited = audited.with_step_tied_tolerance(policy);
    }
    let handle = audited.handle();
    let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
    let atol = vec![config.atol; n];
    let (run, _partial, _client) = integrate_rodas5p_fast_client_observed_with_atol(
        &family.problem,
        Box::new(audited),
        family.span,
        &family.y0,
        &config,
        &output,
        &atol,
    )
    .unwrap();
    assert!(run.observed.success);
    let report = handle.snapshot();
    (report.checks, report.failures)
}

#[test]
fn the_audit_of_arm_c_tests_against_the_threshold_of_arm_c() {
    let mut protected_rule_failures = 0;
    for id in DEVELOPMENT {
        let family = declared(id).unwrap();
        let (checks, failures) = step_tied_run_with_audit(&family, 1.0e-4, true);
        assert!(checks >= 8, "{id}");
        assert_eq!(failures, 0, "{id}: arm C against its own threshold");
        // Against the protected threshold alone the same solves fail: the
        // two thresholds differ and the audit can tell.
        let (same_checks, failures) = step_tied_run_with_audit(&family, 1.0e-4, false);
        assert_eq!(same_checks, checks, "{id}");
        protected_rule_failures += failures;
    }
    assert!(protected_rule_failures > 0);
}

#[test]
fn the_protected_check_detects_a_mismatched_stage_system() {
    let family = declared("dev_ac3d").unwrap();
    let linear = RoutingOptions::default().matrix_free;
    let config = adaptive(&family, 1.0e-4);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
    let run = |step_factor: f64| -> CoreResult<(u64, u64)> {
        let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear)?;
        work.set_gmres_into(true);
        work.set_solve_log(true);
        let mut audit = ProtectedAudit::new(family.problem.dimension, linear.rtol, linear.atol)?;
        let mut observer =
            |t: f64, y: &[f64], h: f64, _error: Option<f64>, work: &Rodas5pMfFastWorkspace| {
                audit.observe(&family.problem, t, y, h * step_factor, work);
            };
        work.integrate_observed_with_observer(
            &family.problem,
            family.span,
            &family.y0,
            &config,
            &output,
            &mut observer,
        )?;
        Ok((audit.summary.checks, audit.summary.failures))
    };
    let (checks, failures) = run(1.0).unwrap();
    assert!(checks >= 8);
    assert_eq!(failures, 0);
    // The same stages against systems rebuilt with a step that is off by
    // one part in a million.
    let (same_checks, failures) = run(1.0 + 1.0e-6).unwrap();
    assert_eq!(same_checks, checks);
    assert!(
        failures * 2 > checks,
        "{failures} of {checks} mismatched systems were detected"
    );
}

#[test]
fn the_shipped_cadence_client_does_the_work_of_the_protected_driver() {
    for id in DEVELOPMENT {
        let family = declared(id).unwrap();
        for rtol in TOLERANCES {
            let a = run_arm(&family, rtol, &ArmOptions::new(Arm::Protected)).unwrap();
            let b0 = run_arm(&family, rtol, &ArmOptions::new(Arm::PerStageShipped)).unwrap();
            let what = format!("{id} rtol {rtol:e}");
            assert!(a.success && b0.success, "{what}");
            assert_eq!(a.attempts, b0.attempts, "{what}: attempts");
            assert_eq!(a.rejected, b0.rejected, "{what}: rejected");
            assert_eq!(a.counters.jvp_calls, b0.counters.jvp_calls, "{what}: JVPs");
            assert_eq!(
                a.counters.linear_iterations, b0.counters.linear_iterations,
                "{what}: Arnoldi columns"
            );
            assert_eq!(a.counters.rhs_calls, b0.counters.rhs_calls, "{what}: RHS");
            let scale = safe_l2(&a.y_end).max(1.0);
            let difference: Vec<f64> = a.y_end.iter().zip(&b0.y_end).map(|(x, y)| x - y).collect();
            assert!(
                safe_l2(&difference) <= 1.0e-9 * scale,
                "{what}: states differ by {:e}",
                safe_l2(&difference)
            );
        }
    }
}
