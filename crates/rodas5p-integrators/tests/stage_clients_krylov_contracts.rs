//! Contracts of the Krylov stage clients (loop node V06): the problem
//! families are what they claim, every arm reproduces itself bit for bit,
//! the residual audit checks every solve without changing the run and
//! reports a solve that misses the threshold, the client with the shipped
//! cadence reproduces the protected matrix-free driver, and every JVP is
//! accounted for. The measured comparison of the arms is not here; it is
//! `stage_clients_krylov_measure.rs`.

#[path = "stage_clients_krylov_problems.rs"]
mod problems;

use problems::{Arm, ArmOptions, ArmRun, Family, adaptive, declared, run_arm};
use rodas5p_core::{CoreError, CoreResult, WorkCounters, safe_l2};
use rodas5p_integrators::stage_clients::krylov::{
    KrylovPreconditioner, KrylovStageClient, PerStageGmresOptions, ResidualAuditClient,
    SharedBasisOptions, StageStoppingRule,
};
use rodas5p_integrators::{
    OdeProblem, OutputSchedule, StageLinearSystem, integrate_rodas5p_fast_client_observed_with_atol,
};
use rodas5p_krylov::shared_basis::{SharedBasis, SharedBasisConfig};

const DEV_FAMILIES: [&str; 5] = [
    "dev_bruss1d",
    "dev_grayscott2d",
    "dev_adr2d",
    "dev_robertson_rd1d",
    "dev_heat1d",
];

/// Deterministic values in (-1, 1) (SplitMix64).
fn pseudo_random(seed: u64, n: usize) -> Vec<f64> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^= z >> 31;
            (z >> 11) as f64 / (1u64 << 52) as f64 - 1.0
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
    let mut out = vec![0.0; y.len()];
    problem.jvp_callback().expect("JVP")(0.0, y, v, &mut out).unwrap();
    out
}

fn same_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

fn assert_same_run(first: &ArmRun, second: &ArmRun, what: &str) {
    assert!(
        same_bits(&first.y_end, &second.y_end),
        "{what}: final states differ"
    );
    assert_eq!(first.counters, second.counters, "{what}: counters differ");
    assert_eq!(first.stats, second.stats, "{what}: statistics differ");
    assert_eq!(
        (
            first.attempts,
            first.accepted,
            first.rejected,
            first.success
        ),
        (
            second.attempts,
            second.accepted,
            second.rejected,
            second.success
        ),
        "{what}: step counts differ"
    );
}

/// The client configurations of the node: B0, B, C, both preconditioner
/// options, and the exploratory variants of C.
fn client_configurations() -> Vec<(&'static str, ArmOptions)> {
    let with = |arm, jacobi, max_dim, keep| ArmOptions {
        jacobi,
        max_dim,
        keep,
        ..ArmOptions::new(arm)
    };
    vec![
        ("B0", with(Arm::PerStageShipped, false, 200, false)),
        ("B", with(Arm::PerStage, false, 200, false)),
        ("C", with(Arm::Shared, false, 200, false)),
        ("B jacobi", with(Arm::PerStage, true, 200, false)),
        ("C jacobi", with(Arm::Shared, true, 200, false)),
        ("C60", with(Arm::Shared, false, 60, false)),
        ("C20", with(Arm::Shared, false, 20, false)),
        ("Ckeep", with(Arm::Shared, false, 200, true)),
    ]
}

#[test]
fn family_jvps_and_diagonals_are_the_derivative_of_the_right_hand_side() {
    for id in DEV_FAMILIES {
        let family = declared(id).unwrap();
        let n = family.problem.dimension;
        // Away from the start, where some families have exact zeros.
        let y: Vec<f64> = family
            .y0
            .iter()
            .zip(pseudo_random(7, n))
            .map(|(value, noise)| value + 1.0e-3 * noise.abs())
            .collect();
        let direction = pseudo_random(11, n);
        let product = jvp(&family.problem, &y, &direction);
        let eps = 1.0e-6;
        let plus: Vec<f64> = y.iter().zip(&direction).map(|(a, d)| a + eps * d).collect();
        let minus: Vec<f64> = y.iter().zip(&direction).map(|(a, d)| a - eps * d).collect();
        let difference: Vec<f64> = rhs(&family.problem, &plus)
            .iter()
            .zip(rhs(&family.problem, &minus))
            .map(|(p, m)| (p - m) / (2.0 * eps))
            .collect();
        let error: Vec<f64> = product
            .iter()
            .zip(&difference)
            .map(|(a, b)| a - b)
            .collect();
        assert!(
            safe_l2(&error) <= 1.0e-6 * safe_l2(&product),
            "{id}: JVP differs from the central difference: {:e} of {:e}",
            safe_l2(&error),
            safe_l2(&product)
        );
        let mut diagonal = vec![0.0; n];
        (family.diagonal)(0.0, &y, &mut diagonal).unwrap();
        for i in 0..n {
            let mut unit = vec![0.0; n];
            unit[i] = 1.0;
            let column = jvp(&family.problem, &y, &unit);
            assert!(
                (column[i] - diagonal[i]).abs() <= 1.0e-12 * column[i].abs().max(1.0),
                "{id}: diagonal entry {i} is {} but J e_i gives {}",
                diagonal[i],
                column[i]
            );
        }
    }
}

#[test]
fn every_arm_reproduces_itself_bit_for_bit() {
    for id in ["dev_bruss1d", "dev_adr2d", "dev_robertson_rd1d"] {
        let family = declared(id).unwrap();
        let first = run_arm(&family, 1.0e-4, &ArmOptions::new(Arm::Protected)).unwrap();
        let second = run_arm(&family, 1.0e-4, &ArmOptions::new(Arm::Protected)).unwrap();
        assert!(first.success);
        assert_same_run(&first, &second, &format!("{id} arm A"));
        for (name, options) in client_configurations() {
            let first = run_arm(&family, 1.0e-4, &options).unwrap();
            let second = run_arm(&family, 1.0e-4, &options).unwrap();
            assert!(first.success, "{id} {name}");
            assert_same_run(&first, &second, &format!("{id} {name}"));
        }
    }
}

#[test]
fn the_audit_checks_every_solve_and_leaves_the_run_unchanged() {
    for id in DEV_FAMILIES {
        let family = declared(id).unwrap();
        for rtol in [1.0e-4, 1.0e-6] {
            for (name, options) in client_configurations() {
                let plain = run_arm(&family, rtol, &options).unwrap();
                let audited = run_arm(
                    &family,
                    rtol,
                    &ArmOptions {
                        audit: true,
                        ..options
                    },
                )
                .unwrap();
                let what = format!("{id} {name} rtol {rtol:e}");
                assert!(plain.success, "{what}");
                assert_same_run(&plain, &audited, &what);
                let stats = audited.stats.unwrap();
                let audit = audited.audit.unwrap();
                assert_eq!(audit.checks, stats.solves - stats.failed_solves, "{what}");
                assert_eq!(audit.jvp_calls, audit.checks, "{what}");
                assert_eq!(audit.failures, 0, "{what}: {audit:?}");
                assert!(audit.max_ratio <= 1.0, "{what}: {audit:?}");
                assert!(plain.audit.is_none());
            }
        }
    }
}

/// A client whose solutions are off by a relative 1e-6: far above the
/// threshold 1e-10, far below anything the integration notices.
struct Perturbed(KrylovStageClient);

impl StageLinearSystem for Perturbed {
    fn dimension(&self) -> usize {
        self.0.dimension()
    }
    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.0.linearize(problem, t, y, counters)
    }
    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        self.0.factor(inv_h_gamma, counters)
    }
    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        self.0.solve_in_place(b, counters)?;
        for value in b.iter_mut() {
            *value *= 1.0 + 1.0e-6;
        }
        Ok(())
    }
}

#[test]
fn the_audit_reports_solves_that_miss_the_threshold() {
    let family = declared("dev_bruss1d").unwrap();
    let n = family.problem.dimension;
    let rule = StageStoppingRule::protected_production().unwrap();
    let config = adaptive(&family, 1.0e-4);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
    for shared in [false, true] {
        let client = if shared {
            KrylovStageClient::shared_basis(
                n,
                rule,
                SharedBasisOptions::baseline(),
                PerStageGmresOptions::baseline(),
                KrylovPreconditioner::None,
            )
        } else {
            KrylovStageClient::per_stage(
                n,
                rule,
                PerStageGmresOptions::baseline(),
                KrylovPreconditioner::None,
            )
        }
        .unwrap();
        let stats = client.stats_handle();
        let audited = ResidualAuditClient::new(Perturbed(client), rule);
        let handle = audited.handle();
        let (run, _, _) = integrate_rodas5p_fast_client_observed_with_atol(
            &family.problem,
            Box::new(audited),
            family.span,
            &family.y0,
            &config,
            &output,
            &vec![config.atol; n],
        )
        .unwrap();
        assert!(run.observed.success);
        let (stats, audit) = (stats.snapshot(), handle.snapshot());
        assert_eq!(audit.checks, stats.solves);
        assert_eq!(audit.first_failure, 1, "{audit:?}");
        // A relative error of 1e-6 leaves a residual of about 1e-6 ||b||,
        // 1e4 thresholds. Only the last stages of the first tiny steps
        // pass: their right-hand sides are so small that the absolute part
        // of the threshold covers the perturbation.
        assert!(
            audit.failures < audit.checks && audit.failures * 10 >= audit.checks * 9,
            "{audit:?} {stats:?}"
        );
        assert!(audit.max_ratio > 1.0e3, "{audit:?}");
        // The audit's JVPs are its own: the run's counters hold the
        // client's only.
        assert_eq!(
            run.observed.counters.jvp_calls,
            stats.krylov_vectors + stats.residual_jvps
        );
    }
}

#[test]
fn the_shipped_cadence_client_reproduces_the_protected_driver() {
    for id in DEV_FAMILIES {
        let family = declared(id).unwrap();
        for rtol in [1.0e-4, 1.0e-6] {
            let protected = run_arm(&family, rtol, &ArmOptions::new(Arm::Protected)).unwrap();
            let client = run_arm(&family, rtol, &ArmOptions::new(Arm::PerStageShipped)).unwrap();
            let what = format!("{id} rtol {rtol:e}");
            assert!(protected.success && client.success, "{what}");
            assert_eq!(
                (protected.attempts, protected.accepted, protected.rejected),
                (client.attempts, client.accepted, client.rejected),
                "{what}: step counts"
            );
            // The two drivers round the stage right-hand sides differently
            // (scaled and unscaled form), so the states agree to the linear
            // tolerance, not bit for bit.
            let difference: Vec<f64> = protected
                .y_end
                .iter()
                .zip(&client.y_end)
                .map(|(a, b)| a - b)
                .collect();
            assert!(
                safe_l2(&difference) <= 1.0e-7 * safe_l2(&protected.y_end),
                "{what}: states differ by {:e}",
                safe_l2(&difference)
            );
            // Problems without a symmetry that ends Arnoldi early: every
            // solve runs the same passes in both drivers.
            if id == "dev_bruss1d" || id == "dev_heat1d" {
                let (a, b) = (&protected.counters, &client.counters);
                assert_eq!(
                    (
                        a.jvp_calls,
                        a.linear_iterations,
                        a.linear_matvecs,
                        a.diagnostic_matvecs,
                        a.rhs_calls
                    ),
                    (
                        b.jvp_calls,
                        b.linear_iterations,
                        b.linear_matvecs,
                        b.diagnostic_matvecs,
                        b.rhs_calls
                    ),
                    "{what}: counters"
                );
            }
        }
    }
}

#[test]
fn every_jvp_is_accounted_for() {
    for id in DEV_FAMILIES {
        let family = declared(id).unwrap();
        for (name, options) in client_configurations() {
            let run = run_arm(&family, 1.0e-4, &options).unwrap();
            let stats = run.stats.unwrap();
            let what = format!("{id} {name}");
            assert_eq!(
                run.counters.jvp_calls,
                stats.krylov_vectors + stats.residual_jvps,
                "{what}"
            );
            assert_eq!(run.counters.jvp_calls, run.counters.jvp_vectors, "{what}");
            assert_eq!(
                run.counters.linear_iterations, stats.krylov_vectors,
                "{what}"
            );
            assert_eq!(
                stats.stage_krylov_vectors.iter().sum::<u64>(),
                stats.krylov_vectors,
                "{what}"
            );
            assert_eq!(
                stats.stage_solves.iter().sum::<u64>(),
                stats.solves,
                "{what}"
            );
            assert_eq!(stats.solves, run.counters.linear_solves, "{what}");
            assert_eq!(stats.attempts, run.attempts as u64, "{what}");
            assert_eq!(stats.failed_solves, 0, "{what}");
            assert_eq!(run.counters.linear_solve_failures, 0, "{what}");
            if options.arm == Arm::Shared {
                assert_eq!(
                    stats.zero_iterate_accepts
                        + stats.shared_accepted_solves
                        + stats.fallback_solves,
                    stats.solves,
                    "{what}"
                );
                assert_eq!(
                    stats.krylov_vectors,
                    stats.shared_vectors + stats.fallback_krylov_vectors,
                    "{what}"
                );
                assert!(stats.basis_dim_max <= options.max_dim as u64, "{what}");
                assert_eq!(stats.basis_dim_attempts, stats.attempts, "{what}");
                if options.keep {
                    // Kept exactly after the attempts the error estimate
                    // rejected (none failed otherwise).
                    assert_eq!(stats.basis_kept, run.rejected as u64, "{what}");
                } else {
                    assert_eq!(stats.basis_kept, 0, "{what}");
                }
            } else {
                assert_eq!(stats.shared_vectors + stats.fallback_solves, 0, "{what}");
            }
            let preconditioned = run.counters.preconditioner_apps > 0;
            assert_eq!(preconditioned, options.jacobi, "{what}");
        }
    }
}

/// `b - W x` with the problem's own JVP callback.
fn residual_norm(family: &Family, y: &[f64], inv: f64, b: &[f64], x: &[f64]) -> f64 {
    let jx = jvp(&family.problem, y, x);
    let r: Vec<f64> = b
        .iter()
        .zip(x)
        .zip(&jx)
        .map(|((bi, xi), ji)| bi - (inv * xi - ji))
        .collect();
    safe_l2(&r)
}

#[test]
fn single_solves_meet_the_rule_and_the_shared_basis_is_reused() {
    let family = declared("dev_bruss1d").unwrap();
    let n = family.problem.dimension;
    let rule = StageStoppingRule::protected_production().unwrap();
    let y = &family.y0;
    let inv = 1.0 / (0.05 * 0.21193756319429014);
    for jacobi in [false, true] {
        let preconditioner = || {
            if jacobi {
                KrylovPreconditioner::RightJacobi(family.diagonal.clone())
            } else {
                KrylovPreconditioner::None
            }
        };
        let mut per_stage = KrylovStageClient::per_stage(
            n,
            rule,
            PerStageGmresOptions::baseline(),
            preconditioner(),
        )
        .unwrap();
        let mut shared = KrylovStageClient::shared_basis(
            n,
            rule,
            SharedBasisOptions::baseline(),
            PerStageGmresOptions::baseline(),
            preconditioner(),
        )
        .unwrap();
        let mut counters = WorkCounters::default();
        for client in [&mut per_stage, &mut shared] {
            client
                .linearize(&family.problem, 0.0, y, &mut counters)
                .unwrap();
            client.factor(inv, &mut counters).unwrap();
        }
        // The first right-hand side, then one in the span of the first and
        // its solution (what a linear problem's next stage looks like),
        // then an unrelated one.
        let b1 = rhs(&family.problem, y);
        let mut x1 = b1.clone();
        per_stage.solve_in_place(&mut x1, &mut counters).unwrap();
        let b2: Vec<f64> = b1.iter().zip(&x1).map(|(b, x)| 0.5 * b + 3.0 * x).collect();
        let b3 = pseudo_random(3, n);
        let mut shared_new = Vec::new();
        for b in [&b1, &b2, &b3] {
            let tau = rule.threshold(inv, safe_l2(b)).unwrap();
            for client in [&mut per_stage, &mut shared] {
                let mut x = b.clone();
                client.solve_in_place(&mut x, &mut counters).unwrap();
                let residual = residual_norm(&family, y, inv, b, &x);
                assert!(
                    residual <= tau,
                    "jacobi {jacobi}: residual {residual:e} above {tau:e}"
                );
            }
            shared_new.push(shared.stats().krylov_vectors);
        }
        let per_stage_stats = per_stage.stats();
        // Four per-stage solves (b1 twice), each with its own space.
        assert_eq!(per_stage_stats.solves, 4);
        let first = shared_new[0];
        let second = shared_new[1] - shared_new[0];
        assert!(first > 5, "jacobi {jacobi}: {shared_new:?}");
        // Without a preconditioner the second system is solved almost
        // inside the first space. Under Jacobi its solution involves
        // `D^-1` between two resolvents, which is no polynomial of
        // `W D^-1` applied to the first right-hand side: it only has to
        // need fewer vectors than a new space.
        if jacobi {
            assert!(second < first, "jacobi: {shared_new:?}");
        } else {
            assert!(second * 4 <= first, "no preconditioner: {shared_new:?}");
        }
        assert_eq!(shared.stats().fallback_solves, 0);
        // A new shift keeps nothing by default.
        shared.factor(2.0 * inv, &mut counters).unwrap();
        let before = shared.stats().krylov_vectors;
        let mut x = b1.clone();
        shared.solve_in_place(&mut x, &mut counters).unwrap();
        assert!(shared.stats().krylov_vectors - before > 5);
        assert!(
            residual_norm(&family, y, 2.0 * inv, &b1, &x)
                <= rule.threshold(2.0 * inv, safe_l2(&b1)).unwrap()
        );
    }
}

#[test]
fn an_exhausted_budget_and_a_missing_setup_are_typed_errors() {
    let family = declared("dev_bruss1d").unwrap();
    let n = family.problem.dimension;
    let rule = StageStoppingRule::protected_production().unwrap();
    let tight = PerStageGmresOptions {
        restart: 2,
        max_arnoldi: 4,
        ..PerStageGmresOptions::baseline()
    };
    let mut counters = WorkCounters::default();
    let b = rhs(&family.problem, &family.y0);
    for shared in [false, true] {
        let mut client = if shared {
            KrylovStageClient::shared_basis(
                n,
                rule,
                SharedBasisOptions {
                    max_dim: 3,
                    ..SharedBasisOptions::baseline()
                },
                tight,
                KrylovPreconditioner::None,
            )
        } else {
            KrylovStageClient::per_stage(n, rule, tight, KrylovPreconditioner::None)
        }
        .unwrap();
        let mut x = b.clone();
        // No linearization, then no shift.
        assert!(matches!(
            client.factor(1.0, &mut counters),
            Err(CoreError::LinearSolve(_))
        ));
        assert!(matches!(
            client.solve_in_place(&mut x, &mut counters),
            Err(CoreError::LinearSolve(_))
        ));
        client
            .linearize(&family.problem, 0.0, &family.y0, &mut counters)
            .unwrap();
        assert!(matches!(
            client.solve_in_place(&mut x, &mut counters),
            Err(CoreError::LinearSolve(_))
        ));
        assert!(matches!(
            client.factor(f64::NAN, &mut counters),
            Err(CoreError::InvalidInput(_))
        ));
        client.factor(1.0, &mut counters).unwrap();
        let before = counters.jvp_calls;
        let mut x = b.clone();
        assert!(matches!(
            client.solve_in_place(&mut x, &mut counters),
            Err(CoreError::LinearSolve(_))
        ));
        let stats = client.stats();
        // The work of the failed solve stays charged and counted.
        assert!(counters.jvp_calls > before);
        assert_eq!(stats.failed_solves, 3);
        assert_eq!(
            stats.krylov_vectors + stats.residual_jvps,
            counters.jvp_calls
        );
        assert_eq!(stats.fallback_solves, u64::from(shared));
        let mut wrong = vec![0.0; n + 1];
        assert!(matches!(
            client.solve_in_place(&mut wrong, &mut counters),
            Err(CoreError::Dimension(_))
        ));
        counters = WorkCounters::default();
    }
    assert!(StageStoppingRule::new(f64::NAN, 0.0).is_err());
    assert!(StageStoppingRule::new(1.0e-10, -1.0).is_err());
    assert!(
        KrylovStageClient::per_stage(
            0,
            rule,
            PerStageGmresOptions::baseline(),
            KrylovPreconditioner::None
        )
        .is_err()
    );
}

/// One stage-1 solve with the merged kernel exactly as the hook of the
/// protected driver calls it (`rodas5p_matrix_free_fast.rs`, the hook in
/// `attempt_inner`: weights `1/(atol + rtol |y_i|)`, right-hand side
/// `rhs/(h gamma)`, the default `theta = 1e-3` and 60 vectors). Returns
/// whether the kernel reported convergence, the 2-norm of the original
/// residual and the protected driver's threshold for that system.
fn merged_hook_stage_solve() -> (bool, f64, f64) {
    let family = declared("dev_bruss1d").unwrap();
    let n = family.problem.dimension;
    let y = &family.y0;
    let (rtol, atol) = (1.0e-4, 1.0e-4);
    let h_gamma = 0.05 * 0.21193756319429014;
    let b = rhs(&family.problem, y);
    let weights: Vec<f64> = y
        .iter()
        .map(|value| 1.0 / (atol + rtol * value.abs()))
        .collect();
    let mut basis = SharedBasis::new(n, SharedBasisConfig::default()).unwrap();
    let mut x = vec![0.0; n];
    let callback = family.problem.jvp_callback().expect("JVP").clone();
    let mut product = |v: &[f64], out: &mut [f64]| -> CoreResult<()> { callback(0.0, y, v, out) };
    let stats = basis
        .solve_shifted(h_gamma, &b, &weights, &mut product, &mut x)
        .unwrap();
    let rule = StageStoppingRule::protected_production().unwrap();
    (
        stats.converged,
        residual_norm(&family, y, 1.0 / h_gamma, &b, &x),
        rule.threshold(1.0 / h_gamma, safe_l2(&b)).unwrap(),
    )
}

/// The merged hook accepts a stage solution on the kernel's own test
/// (`h gamma ||r||_WRMS <= 1e-3` in the step controller's weights), which
/// is not the stopping test of the driver it is hooked into: the accepted
/// residual is orders of magnitude above the driver's threshold. A JVP
/// count of the hook is therefore no measurement against the driver's
/// GMRES. The clients of this node encode the driver's threshold in the
/// kernel's weights and confirm every accepted iterate instead.
#[test]
fn the_merged_hook_accepts_residuals_far_above_the_protected_threshold() {
    let (converged, residual, threshold) = merged_hook_stage_solve();
    assert!(converged);
    assert!(
        residual > 1.0e3 * threshold,
        "residual {residual:e}, protected threshold {threshold:e}"
    );
}

/// The same fact as a failing test: run it with `--ignored` to see the
/// numbers. It asserts what a fair comparison with the driver's GMRES
/// would need and the merged hook does not provide.
#[test]
#[ignore = "fails by design: the merged hook's stopping test is not the protected driver's"]
fn merged_hook_solution_meets_the_protected_threshold() {
    let (converged, residual, threshold) = merged_hook_stage_solve();
    assert!(converged);
    assert!(
        residual <= threshold,
        "the merged hook accepted a stage solution with residual {residual:e}; the protected \
         driver's threshold for this system is {threshold:e} ({:.1e} times smaller)",
        residual / threshold
    );
}
