//! Contracts of the adaptive stage-solve policies of the matrix-free driver
//! (loop node V24, owner decision OD-L1-11), on small problems:
//!
//! * every stage solve returns an iterate whose true residual meets the
//!   threshold of the shipped driver, by an independent rebuild of the stage
//!   system (`ProtectedAudit` of node V17) and by the driver's own records;
//! * a solve that cannot meet it is a failed attempt, as in the shipped
//!   driver;
//! * the run stays within the band of the frozen golden bound of the node
//!   around the shipped run (end state within 10 tolerance units, attempts
//!   within 20 percent) with fewer Arnoldi columns;
//! * the router reports the identifier of the policy; the policies are
//!   refused where they do not apply.

#[path = "stage_clients_krylov_v17_problems.rs"]
mod problems;

use problems::{Family, ProtectedAudit, adaptive, declared, v06};
use rodas5p_core::{CoreError, CoreResult, LinearMethod};
use rodas5p_integrators::{
    MfStageSolvePolicy, MfStageSolveStatistics, OutputSchedule, RODAS5P_MF_FAST_DRIVER_ID,
    Rodas5pMfFastResult, Rodas5pMfFastWorkspace, RoutedRun, RoutingOptions, StageTargetOptions,
    StageTargetPolicy, integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_with_stage_solve_policy,
    integrate_rodas5p_routed_observed_with_options,
};

const ADAPTIVE: [MfStageSolvePolicy; 2] = [
    MfStageSolvePolicy::ColumnTested,
    MfStageSolvePolicy::ColumnTestedDoubling,
];

const SMALL: [&str; 5] = [
    "dev_fhn3d",
    "dev_cusp1d",
    "dev_burgers2d",
    "dev_bruss1d",
    "v06:dev_adr2d",
];

const RTOLS: [f64; 2] = [1.0e-4, 1.0e-6];

fn family(id: &str) -> CoreResult<Family> {
    match id.strip_prefix("v06:") {
        Some(base) => v06::declared(base),
        None => declared(id),
    }
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}

/// What an audited run of a policy returns.
struct Audited {
    run: Rodas5pMfFastResult,
    /// Stage solves checked by the independent rebuild, and failed checks.
    checks: u64,
    failures: u64,
    max_ratio: f64,
    /// The driver's records: their number and the violations among them.
    records: u64,
    record_violations: u64,
    statistics: MfStageSolveStatistics,
}

/// The driver under `policy` with the solve log, the stage-solve records and
/// the independent residual audit.
fn audited(family: &Family, rtol: f64, policy: MfStageSolvePolicy) -> Audited {
    let config = adaptive(family, rtol);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
    let linear = RoutingOptions::default().matrix_free;
    let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear).unwrap();
    work.set_gmres_into(true);
    work.set_stage_solve_policy(policy).unwrap();
    work.set_solve_log(true);
    work.set_stage_solve_records(true);
    let mut audit =
        ProtectedAudit::new(family.problem.dimension, linear.rtol, linear.atol).unwrap();
    let mut observer =
        |t: f64, y: &[f64], h: f64, _error: Option<f64>, work: &Rodas5pMfFastWorkspace| {
            audit.observe(&family.problem, t, y, h, work);
        };
    let (run, ..) = work
        .integrate_observed_with_observer(
            &family.problem,
            family.span,
            &family.y0,
            &config,
            &output,
            &mut observer,
        )
        .unwrap();
    let records = work.stage_solve_records().unwrap();
    Audited {
        run,
        checks: audit.summary.checks,
        failures: audit.summary.failures,
        max_ratio: audit.summary.max_ratio,
        records: records.len() as u64,
        record_violations: records
            .iter()
            .filter(|r| !(r.residual_norm.is_finite() && r.residual_norm <= r.threshold))
            .count() as u64,
        statistics: work.stage_solve_policy_statistics(),
    }
}

/// The audit-off entry of a policy.
fn plain(family: &Family, rtol: f64, policy: MfStageSolvePolicy) -> Rodas5pMfFastResult {
    let config = adaptive(family, rtol);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
    integrate_rodas5p_mf_fast_observed_with_stage_solve_policy(
        &family.problem,
        family.span,
        &family.y0,
        &RoutingOptions::default().matrix_free,
        &config,
        &output,
        policy,
        false,
    )
    .unwrap()
}

#[test]
fn every_adaptive_stage_solve_meets_the_threshold_of_the_shipped_driver() {
    let mut solves = 0;
    for id in SMALL {
        let family = family(id).unwrap();
        for rtol in RTOLS {
            for policy in ADAPTIVE {
                let audited = audited(&family, rtol, policy);
                let counters = audited.run.observed.counters;
                let what = format!("{id} {rtol:e} {}", policy.id());
                assert!(audited.run.observed.success, "{what}");
                // The independent rebuild of every completed stage solve.
                assert_eq!(audited.failures, 0, "{what}");
                assert!(audited.max_ratio <= 1.0, "{what}: {}", audited.max_ratio);
                assert_eq!(audited.checks, counters.linear_solves, "{what}");
                // The driver's own record of every returned iterate.
                assert_eq!(audited.records, counters.linear_solves, "{what}");
                assert_eq!(audited.record_violations, 0, "{what}");
                // Counts close: every operator application is a Krylov
                // column, a true residual at the top of a pass or the final
                // residual of a completed solve.
                assert_eq!(
                    counters.jvp_calls,
                    counters.linear_matvecs + counters.diagnostic_matvecs,
                    "{what}"
                );
                assert_eq!(
                    counters.diagnostic_matvecs, counters.linear_solves,
                    "{what}"
                );
                assert_eq!(counters.preconditioner_apps, 0, "{what}");
                let statistics = audited.statistics;
                assert_eq!(
                    statistics.completed_solves, counters.linear_solves,
                    "{what}"
                );
                assert_eq!(
                    statistics.budget_exhausted_solves + statistics.other_linear_failures,
                    counters.linear_solve_failures,
                    "{what}"
                );
                assert!(statistics.projected_exits > 0, "{what}");
                assert!(statistics.longest_cycle <= 40, "{what}");
                // The records and the observer change nothing: the audit-off
                // entry gives the same run, bit for bit.
                let plain = plain(&family, rtol, policy);
                assert_eq!(plain.observed.counters, counters, "{what}");
                assert_eq!(
                    bits(plain.observed.y.last().unwrap()),
                    bits(audited.run.observed.y.last().unwrap()),
                    "{what}"
                );
                assert_eq!(plain.attempts, audited.run.attempts, "{what}");
                assert_eq!(plain.driver, policy.driver_id(), "{what}");
                assert_ne!(plain.driver, RODAS5P_MF_FAST_DRIVER_ID, "{what}");
                solves += counters.linear_solves;
            }
        }
    }
    assert!(solves > 5000, "{solves} audited stage solves");
}

#[test]
fn an_adaptive_run_stays_within_the_golden_band_of_the_shipped_run_with_fewer_columns() {
    for id in SMALL {
        let family = family(id).unwrap();
        for rtol in RTOLS {
            let config = adaptive(&family, rtol);
            let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
            let shipped = integrate_rodas5p_mf_fast_observed_gmres_into(
                &family.problem,
                family.span,
                &family.y0,
                &RoutingOptions::default().matrix_free,
                &config,
                &output,
            )
            .unwrap();
            let reference = shipped.observed.y.last().unwrap();
            for policy in ADAPTIVE {
                let run = plain(&family, rtol, policy);
                let what = format!("{id} {rtol:e} {}", policy.id());
                assert!(run.observed.success, "{what}");
                // The frozen bounds of discriminator G: 10 units of
                // atol + rtol |y| per component, 20 percent in the attempts.
                let units = run
                    .observed
                    .y
                    .last()
                    .unwrap()
                    .iter()
                    .zip(reference)
                    .map(|(y, r)| (y - r).abs() / (config.atol + config.rtol * r.abs()))
                    .fold(0.0, f64::max);
                assert!(units <= 10.0, "{what}: {units} tolerance units");
                let attempts = run.attempts as f64 / shipped.attempts as f64;
                assert!(
                    (0.8..=1.2).contains(&attempts),
                    "{what}: attempts {attempts}"
                );
                // Never more Arnoldi columns than the shipped policy.
                assert!(
                    run.observed.counters.linear_iterations
                        <= shipped.observed.counters.linear_iterations,
                    "{what}"
                );
                assert!(
                    run.observed.counters.orthogonalization_inner_products
                        < shipped.observed.counters.orthogonalization_inner_products,
                    "{what}"
                );
            }
        }
    }
}

#[test]
fn a_solve_that_exhausts_the_budget_is_a_failed_attempt_under_every_policy() {
    // A heat equation whose first steps are far too stiff for 200 Arnoldi
    // vectors without a preconditioner.
    let family = v06::heat1d(300, 100.0).unwrap();
    let rtol = 1.0e-4;
    for policy in [
        MfStageSolvePolicy::Shipped,
        MfStageSolvePolicy::ColumnTested,
        MfStageSolvePolicy::ColumnTestedDoubling,
    ] {
        let audited = audited(&family, rtol, policy);
        let counters = audited.run.observed.counters;
        let what = policy.id();
        assert!(audited.run.observed.success, "{what}");
        // The attempt whose solve fails is rejected and counted as the
        // shipped driver counts it.
        assert!(counters.linear_solve_failures > 0, "{what}");
        assert_eq!(
            audited.statistics.budget_exhausted_solves, counters.linear_solve_failures,
            "{what}"
        );
        assert_eq!(audited.statistics.other_linear_failures, 0, "{what}");
        assert!(
            audited.run.rejected_steps as u64 >= counters.linear_solve_failures,
            "{what}"
        );
        // No iterate of a failed solve was returned: every returned iterate
        // is audited and meets the threshold.
        assert_eq!(audited.failures, 0, "{what}");
        assert_eq!(audited.checks, counters.linear_solves, "{what}");
        assert_eq!(audited.records, counters.linear_solves, "{what}");
        assert_eq!(audited.record_violations, 0, "{what}");
    }
}

#[test]
fn the_router_runs_and_reports_the_policy_it_is_given() {
    let family = family("dev_bruss1d").unwrap();
    let rtol = 1.0e-5;
    let config = adaptive(&family, rtol);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
    for policy in ADAPTIVE {
        for krylov_ls_workspace in [false, true] {
            let options = RoutingOptions {
                matrix_free_policy: policy,
                krylov_ls_workspace,
                ..RoutingOptions::default()
            };
            let routed = integrate_rodas5p_routed_observed_with_options(
                &family.problem,
                family.span,
                &family.y0,
                &config,
                &output,
                &options,
            )
            .unwrap();
            assert_eq!(routed.routing.driver_id, policy.driver_id());
            let RoutedRun::MatrixFree(run) = &routed.run else {
                panic!("the JVP-only problem is routed to the matrix-free driver");
            };
            assert_eq!(run.driver, policy.driver_id());
            // The least-squares workspace of the facade is not used by an
            // adaptive policy: the run is the same, only its label differs.
            let direct = plain(&family, rtol, policy);
            assert_eq!(run.observed.counters, direct.observed.counters);
            assert_eq!(
                bits(run.observed.y.last().unwrap()),
                bits(direct.observed.y.last().unwrap())
            );
            assert_eq!(run.attempts, direct.attempts);
        }
    }
}

#[test]
fn the_adaptive_policies_are_refused_where_they_do_not_apply() {
    let family = family("dev_bruss1d").unwrap();
    let config = adaptive(&family, 1.0e-4);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
    let linear = RoutingOptions::default().matrix_free;
    let integrate = |work: &mut Rodas5pMfFastWorkspace| {
        work.integrate_observed(&family.problem, family.span, &family.y0, &config, &output)
    };
    for policy in ADAPTIVE {
        // Not GMRES.
        let lgmres = rodas5p_core::LinearSolverConfig {
            method: LinearMethod::Lgmres,
            ..linear.clone()
        };
        let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &lgmres).unwrap();
        assert!(matches!(
            work.set_stage_solve_policy(policy),
            Err(CoreError::InvalidInput(_))
        ));
        assert_eq!(work.stage_solve_policy(), MfStageSolvePolicy::Shipped);

        // GMRES, but not through GMRES into.
        let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear).unwrap();
        work.set_stage_solve_policy(policy).unwrap();
        assert!(matches!(
            integrate(&mut work),
            Err(CoreError::InvalidInput(_))
        ));

        // A staged stage target or the DupFix target.
        for target in [StageTargetPolicy::ProjL2, StageTargetPolicy::DupFix] {
            let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear).unwrap();
            work.set_gmres_into(true);
            work.set_stage_target(StageTargetOptions::new(target))
                .unwrap();
            work.set_stage_solve_policy(policy).unwrap();
            assert!(
                matches!(integrate(&mut work), Err(CoreError::InvalidInput(_))),
                "{target:?}"
            );
        }

        // Back to the shipped policy the same workspace runs.
        let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear).unwrap();
        work.set_gmres_into(true);
        work.set_stage_solve_policy(policy).unwrap();
        let adaptive_run = integrate(&mut work).unwrap();
        work.set_stage_solve_policy(MfStageSolvePolicy::Shipped)
            .unwrap();
        let shipped_run = integrate(&mut work).unwrap();
        assert_eq!(shipped_run.driver, RODAS5P_MF_FAST_DRIVER_ID);
        let reference = integrate_rodas5p_mf_fast_observed_gmres_into(
            &family.problem,
            family.span,
            &family.y0,
            &linear,
            &config,
            &output,
        )
        .unwrap();
        // A workspace that ran an adaptive policy before gives the shipped
        // bits afterwards.
        assert_eq!(shipped_run.observed.counters, reference.observed.counters);
        assert_eq!(
            bits(shipped_run.observed.y.last().unwrap()),
            bits(reference.observed.y.last().unwrap())
        );
        assert!(adaptive_run.observed.success);
    }
}
