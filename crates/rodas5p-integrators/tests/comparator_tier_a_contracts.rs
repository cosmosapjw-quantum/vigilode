//! Audit F-052/F-056 Tier-A opt-in comparator fairness flags.  The defaults
//! stay the frozen reference configuration; these contracts exercise only the
//! opt-in paths.

use rodas5p_integrators::{
    AdaptiveObservedIntegrationResult, AdaptiveStepConfig, BdfConfig, BdfOrder, ComparatorFidelity,
    NewtonTolerancePolicy, OdeProblem, OutputSchedule, RadauConfig,
    integrate_bdf_adaptive_observed, integrate_radau_adaptive_observed,
    manufactured_mass_nonlinear_problem, prothero_robinson_problem, radau_newton_tolerance_factor,
};

fn adaptive(rtol: f64, span: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-2 * rtol,
        rtol,
        initial_step: 1.0e-3,
        min_step: 1.0e-12,
        max_step: span,
        max_attempts: 50_000,
        ..AdaptiveStepConfig::default()
    }
}

fn endpoint_error(problem: &OdeProblem, tf: f64, state: &[f64]) -> f64 {
    let exact = problem.exact(tf).unwrap();
    state
        .iter()
        .zip(&exact)
        .map(|(value, reference)| (value - reference).abs())
        .fold(0.0, f64::max)
}

fn iterations_per_solve(run: &AdaptiveObservedIntegrationResult) -> f64 {
    let counters = run.observed.counters;
    counters.nonlinear_iterations as f64 / counters.nonlinear_solves as f64
}

fn radau(
    problem: &OdeProblem,
    y0: &[f64],
    tf: f64,
    config: &RadauConfig,
    rtol: f64,
) -> AdaptiveObservedIntegrationResult {
    let schedule = OutputSchedule::new(vec![0.0, tf]).unwrap();
    let run = integrate_radau_adaptive_observed(
        problem,
        (0.0, tf),
        y0,
        config,
        &adaptive(rtol, tf),
        &schedule,
    )
    .unwrap();
    assert!(run.observed.success);
    run
}

#[test]
fn tier_a_flags_are_opt_in_and_relabel_the_comparator() {
    let reference = RadauConfig::default();
    assert!(!reference.reuse_stage_lu_for_error_estimate);
    assert_eq!(
        reference.newton_tolerance,
        NewtonTolerancePolicy::FixedNewtonConfig
    );
    assert_eq!(
        reference.comparator_fidelity(),
        ComparatorFidelity::ReferenceImplementationOnly
    );
    assert_eq!(
        BdfConfig::default().comparator_fidelity(),
        ComparatorFidelity::ReferenceImplementationOnly
    );
    let reuse = RadauConfig {
        reuse_stage_lu_for_error_estimate: true,
        ..RadauConfig::default()
    };
    assert_eq!(
        reuse.comparator_fidelity(),
        ComparatorFidelity::TierAModifiedNewton
    );
    let bdf = BdfConfig {
        newton_tolerance: NewtonTolerancePolicy::ScaledToOuterTolerance,
        ..BdfConfig::default()
    };
    assert_eq!(
        bdf.comparator_fidelity(),
        ComparatorFidelity::TierAModifiedNewton
    );
    // Tier A without cross-step LU reuse still forbids a relative reading.
    assert!(!ComparatorFidelity::TierAModifiedNewton.admits_relative_performance_reading());
    // SciPy Radau rule: max(10 eps / rtol, min(0.03, sqrt(rtol))).
    assert_eq!(radau_newton_tolerance_factor(1.0e-2), 0.03);
    assert!((radau_newton_tolerance_factor(1.0e-4) - 1.0e-2).abs() < 1.0e-15);
    assert_eq!(
        radau_newton_tolerance_factor(1.0e-15),
        10.0 * f64::EPSILON / 1.0e-15
    );
}

#[test]
fn radau3_stage_lu_estimator_reduces_whole_run_factorizations() {
    // The exact per-trial accounting (one factorization fewer per estimator
    // call, same estimate) is pinned by the radau.rs unit test on identical
    // trial inputs.  Whole runs may diverge at roundoff near Newton failures,
    // so this contract checks only the end-to-end effect.
    let (pr, pr_y0) = prothero_robinson_problem(-1.0e4, 50.0, 0.0);
    let (mass, mass_y0, _, _) = manufactured_mass_nonlinear_problem(20.0, 1.0, 0.2, 0.0).unwrap();
    for (problem, y0, tf) in [(&pr, &pr_y0, 2.0), (&mass, &mass_y0, 1.0)] {
        for rtol in [1.0e-4, 1.0e-7] {
            let separate = radau(problem, y0, tf, &RadauConfig::default(), rtol);
            let reused = radau(
                problem,
                y0,
                tf,
                &RadauConfig {
                    reuse_stage_lu_for_error_estimate: true,
                    ..RadauConfig::default()
                },
                rtol,
            );
            let per_attempt = |run: &AdaptiveObservedIntegrationResult| {
                run.observed.counters.direct_factorizations as f64 / run.diagnostics.attempts as f64
            };
            assert!(
                per_attempt(&reused) < per_attempt(&separate),
                "{} rtol={rtol:e}",
                problem.name
            );
            let separate_error = endpoint_error(problem, tf, separate.observed.y.last().unwrap());
            let reused_error = endpoint_error(problem, tf, reused.observed.y.last().unwrap());
            assert!(
                reused_error <= 10.0 * separate_error.max(1.0e-3 * rtol),
                "{} rtol={rtol:e}: separate={separate_error:e} reused={reused_error:e}",
                problem.name
            );
        }
    }
}

#[test]
fn scaled_newton_tolerance_loosens_with_the_outer_tolerance_on_prothero_robinson() {
    let (problem, y0) = prothero_robinson_problem(-1.0e4, 50.0, 0.0);
    let tf = 2.0;
    let scaled = RadauConfig {
        newton_tolerance: NewtonTolerancePolicy::ScaledToOuterTolerance,
        ..RadauConfig::default()
    };
    let loose = radau(&problem, &y0, tf, &scaled, 1.0e-4);
    let tight = radau(&problem, &y0, tf, &scaled, 1.0e-8);
    let fixed_loose = radau(&problem, &y0, tf, &RadauConfig::default(), 1.0e-4);
    assert!(
        iterations_per_solve(&loose) < iterations_per_solve(&tight),
        "loose={} tight={}",
        iterations_per_solve(&loose),
        iterations_per_solve(&tight)
    );
    assert!(iterations_per_solve(&loose) < iterations_per_solve(&fixed_loose));
    // The looser Newton stop must not leave the outer tolerance band
    // (|y| <= 1 and atol = rtol / 100, so the band is ~rtol).
    for (run, rtol) in [(&loose, 1.0e-4), (&tight, 1.0e-8)] {
        let error = endpoint_error(&problem, tf, run.observed.y.last().unwrap());
        assert!(error <= rtol, "rtol={rtol:e}: endpoint error {error:e}");
    }

    // BDF2 uses CVODE's 0.1 factor.  Its iterations per step are dominated
    // by predictor accuracy at small steps, so the contract compares the two
    // policies at the loose tolerance where the fixed stop over-solves.
    let schedule = OutputSchedule::new(vec![0.0, tf]).unwrap();
    let bdf = |policy| {
        let run = integrate_bdf_adaptive_observed(
            &problem,
            (0.0, tf),
            &y0,
            &BdfConfig {
                order: BdfOrder::Two,
                newton_tolerance: policy,
                ..BdfConfig::default()
            },
            &adaptive(1.0e-4, tf),
            &schedule,
        )
        .unwrap();
        assert!(run.observed.success);
        run
    };
    let bdf_fixed = bdf(NewtonTolerancePolicy::FixedNewtonConfig);
    let bdf_scaled = bdf(NewtonTolerancePolicy::ScaledToOuterTolerance);
    assert!(iterations_per_solve(&bdf_scaled) < iterations_per_solve(&bdf_fixed));
    let fixed_error = endpoint_error(&problem, tf, bdf_fixed.observed.y.last().unwrap());
    let scaled_error = endpoint_error(&problem, tf, bdf_scaled.observed.y.last().unwrap());
    assert!(scaled_error <= 1.0e-4);
    assert!(scaled_error <= 10.0 * fixed_error.max(1.0e-12));
}
