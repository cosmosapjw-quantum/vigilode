//! Research node `research/as04_stage_residual_telemetry_20261011` (AS04):
//! residual and budget telemetry of every accepted stage solve.
//!
//! Checks:
//! - D1: transactional linear solve failure preserves complete records and work counters.
//! - D2: operator epoch generation increments at re-linearizations and model epoch updates.
//! - R: real-driver cells parity on/off (Item 1a) and bitwise parity against ALG06 RUNS (Item 1b).
//! - Ledgers, work closure, vector copy formulas, and metric invariants.

#[allow(unused_imports)]
mod alg01_common;

use alg01_common::*;
use rodas5p_core::{
    DenseMatrix, DenseOperator, InitialGuess, LinearMethod, LinearOperator, LinearSolverConfig,
    WorkCounters, rodas5p_coefficients, safe_l2,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, AttemptResidualOutcome, OdeProblem, OutputSchedule, ResidualMetric,
    Rodas5pMfFastWorkspace, StageTargetOptions, StageTargetPolicy,
    integrate_rodas5p_mf_fast_observed_with_stage_target,
    integrate_rodas5p_mf_fast_observed_with_stage_telemetry,
};
use rodas5p_krylov::{
    HeuristicGain, StagedAcceptanceReason, StagedGmresConfig, StagedGmresOutcome,
    StagedGmresReport, StagedGmresWorkspace, StagedSolveTelemetry, TargetAttainment, VerifiedBound,
    solve_staged_gmres_observed,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const PRODUCTION_RTOL: f64 = 1.0e-10;
const PRODUCTION_ATOL: f64 = 1.0e-14;

fn alg06_options(g1: bool, g2: bool, g3: bool) -> StageTargetOptions {
    StageTargetOptions {
        stagnation_guard: true,
        classify_guard_aborts: true,
        classify_accepted_guard_aborts: true,
        effective_cycle_overrun: g1,
        floor_at_confirmations: g2,
        charge_fallback_residual: g3,
        ..StageTargetOptions::new(StageTargetPolicy::CoupledGuarded2)
    }
}

// ---------------------------------------------------------------------------
// D1: Transactional failure fixture (AS01 direct driver boundary)
// ---------------------------------------------------------------------------

const D1_STEP: f64 = 5.0;

fn d1_problem() -> (OdeProblem, Vec<f64>) {
    let kappa = D1_STEP * rodas5p_coefficients().unwrap().gamma;
    let shift = move |v: &[f64], out: &mut [f64]| {
        for i in 0..4 {
            out[i] = (v[i] - v[(i + 3) % 4]) / kappa;
        }
    };
    let c = [1.5e308 / kappa, 1.5e308 / kappa, 0.0, 0.0];
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
    let problem = OdeProblem::new(
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
    .unwrap();
    let y0 = vec![1.0e306; 4];
    (problem, y0)
}

fn d1_configs() -> (LinearSolverConfig, AdaptiveStepConfig, StageTargetOptions) {
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
        initial_step: D1_STEP,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    let options = StageTargetOptions {
        production_fallback: true,
        ..StageTargetOptions::new(StageTargetPolicy::Coupled)
    };
    (linear, adaptive, options)
}

#[test]
fn test_d1_transactional_failure() {
    let (problem, y0) = d1_problem();
    let (linear, adaptive, options) = d1_configs();
    let schedule = OutputSchedule::new(vec![0.0, 2.0 * D1_STEP]).unwrap();

    // Telemetry OFF
    let off = integrate_rodas5p_mf_fast_observed_with_stage_target(
        &problem,
        (0.0, 2.0 * D1_STEP),
        &y0,
        &linear,
        &adaptive,
        &schedule,
        options,
    )
    .unwrap();

    // Telemetry ON
    let on = integrate_rodas5p_mf_fast_observed_with_stage_telemetry(
        &problem,
        (0.0, 2.0 * D1_STEP),
        &y0,
        &linear,
        &adaptive,
        &schedule,
        options,
    )
    .unwrap();

    // Item 1a: bitwise identical on/off
    assert_eq!(on.result.attempts, off.result.attempts);
    assert_eq!(on.result.accepted_steps, off.result.accepted_steps);
    assert_eq!(on.result.rejected_steps, off.result.rejected_steps);
    assert_eq!(on.result.observed.counters, off.result.observed.counters);
    assert_eq!(on.statistics, off.statistics);
    assert_eq!(on.charges, off.charges);

    // D1 specific checks
    assert_eq!(on.result.attempts, 1);
    assert_eq!(on.result.accepted_steps, 0);
    assert_eq!(on.result.rejected_steps, 1);
    assert_eq!(on.result.observed.counters.linear_solve_failures, 1);

    let tel = &on.telemetry;
    assert_eq!(tel.attempt_records.len(), 1);
    assert_eq!(tel.stage_records.len(), 1);
    assert_eq!(
        tel.attempt_records[0].outcome,
        AttemptResidualOutcome::FailedLinear
    );
    assert_eq!(tel.attempt_records[0].failed, 1);
    assert!(tel.attempt_records[0].jvp_calls > 0);
    assert!(tel.work_all_attempts.jvp_calls > 0);
    assert_eq!(
        tel.work_all_attempts.jvp_calls + tel.jvp_outside_attempts,
        on.result.observed.counters.jvp_calls
    );
    assert_eq!(tel.committed_debit, 0.0);
}

// ---------------------------------------------------------------------------
// D2: Epoch and linearization generation fixture
// ---------------------------------------------------------------------------

fn d2_problem(epoch: Arc<AtomicU64>) -> (OdeProblem, Vec<f64>) {
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -1.0e5 * y[0];
        out[1] = -y[1];
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = -1.0e5 * v[0];
        out[1] = -v[1];
        Ok(())
    });
    let problem = OdeProblem::new(
        "d2-linear-2",
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
    .with_model_epoch(Arc::new(move || epoch.load(Ordering::SeqCst)));

    (problem, vec![1.0, 1.0])
}

fn d2_configs() -> (LinearSolverConfig, AdaptiveStepConfig, StageTargetOptions) {
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: PRODUCTION_RTOL,
        atol: PRODUCTION_ATOL,
        restart: 40,
        maxiter: 200,
        x0_strategy: InitialGuess::Zero,
        ..LinearSolverConfig::default()
    };
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-8,
        initial_step: 1.0,
        max_attempts: 4,
        ..AdaptiveStepConfig::default()
    };
    let options = StageTargetOptions::new(StageTargetPolicy::Coupled);
    (linear, adaptive, options)
}

#[test]
fn test_d2_epoch_and_generation() {
    let epoch_on = Arc::new(AtomicU64::new(100));
    let (problem_on, y0) = d2_problem(epoch_on.clone());
    let (linear, adaptive, options) = d2_configs();
    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();

    let mut work_on = Rodas5pMfFastWorkspace::new(&problem_on, &linear).unwrap();
    work_on.set_gmres_into(true);
    work_on.set_stage_telemetry(true);
    work_on.set_stage_target(options).unwrap();
    work_on.set_stage_target_span(1.0);

    let mut attempts_on = 0;
    let (result_on, stats_on, charges_on, _) = work_on
        .integrate_observed_with_observer(
            &problem_on,
            (0.0, 1.0),
            &y0,
            &adaptive,
            &schedule,
            &mut |_, _, _, _, _| {
                attempts_on += 1;
                if attempts_on == 3 {
                    epoch_on.store(200, Ordering::SeqCst);
                }
            },
        )
        .unwrap();

    let tel = work_on.stage_telemetry().unwrap();

    // Telemetry OFF counterpart
    let epoch_off = Arc::new(AtomicU64::new(100));
    let (problem_off, _) = d2_problem(epoch_off.clone());
    let mut work_off = Rodas5pMfFastWorkspace::new(&problem_off, &linear).unwrap();
    work_off.set_gmres_into(true);
    work_off.set_stage_telemetry(false);
    work_off.set_stage_target(options).unwrap();
    work_off.set_stage_target_span(1.0);

    let mut attempts_off = 0;
    let (result_off, stats_off, charges_off, _) = work_off
        .integrate_observed_with_observer(
            &problem_off,
            (0.0, 1.0),
            &y0,
            &adaptive,
            &schedule,
            &mut |_, _, _, _, _| {
                attempts_off += 1;
                if attempts_off == 3 {
                    epoch_off.store(200, Ordering::SeqCst);
                }
            },
        )
        .unwrap();

    // Item 1a: bitwise identical on/off
    assert_eq!(result_on.attempts, result_off.attempts);
    assert_eq!(result_on.accepted_steps, result_off.accepted_steps);
    assert_eq!(result_on.rejected_steps, result_off.rejected_steps);
    assert_eq!(result_on.observed.counters, result_off.observed.counters);
    assert_eq!(stats_on, stats_off);
    assert_eq!(charges_on, charges_off);

    // Gate Item 6: Generation increments exactly at re-linearizations, and model_epoch changes at 4th attempt
    assert_eq!(result_on.attempts, 4);
    assert_eq!(tel.attempt_records.len(), 4);

    for rec in &tel.stage_records {
        if rec.attempt < 3 {
            assert_eq!(rec.operator_epoch.generation, 1);
            assert_eq!(rec.operator_epoch.model_epoch, Some(100));
        } else {
            assert_eq!(rec.attempt, 3); // 4th attempt is index 3
            assert_eq!(rec.operator_epoch.generation, 2);
            assert_eq!(rec.operator_epoch.model_epoch, Some(200));
        }
    }
}

// ---------------------------------------------------------------------------
// R cells: Real-driver cells parity on/off and ALG06 anchor parity
// ---------------------------------------------------------------------------

struct RCell {
    name: &'static str,
    problem: Problem,
    rtol: f64,
    arm: &'static str,
    options: StageTargetOptions,
    budget: usize,
}

fn r_cells() -> Vec<RCell> {
    vec![
        RCell {
            name: "robertson-4e10_1e-5_b3",
            problem: robertson_long(),
            rtol: 1.0e-5,
            arm: "b3",
            options: alg06_options(true, true, true),
            budget: BIG_BUDGET,
        },
        RCell {
            name: "robertson-4e10_1e-9_b3",
            problem: robertson_long(),
            rtol: 1.0e-9,
            arm: "b3",
            options: alg06_options(true, true, true),
            budget: BIG_BUDGET,
        },
        RCell {
            name: "stosc-w1e4_1e-4_b3",
            problem: stosc(1.0e4),
            rtol: 1.0e-4,
            arm: "b3",
            options: alg06_options(true, true, true),
            budget: BIG_BUDGET,
        },
        RCell {
            name: "van-der-pol-mu1000_1e-6_b3",
            problem: spd07_problem("van-der-pol-mu1000"),
            rtol: 1.0e-6,
            arm: "b3",
            options: alg06_options(true, true, true),
            budget: BIG_BUDGET,
        },
        RCell {
            name: "brusselator-1d-50_1e-6_b3",
            problem: brusselator_problem(50),
            rtol: 1.0e-6,
            arm: "b3",
            options: alg06_options(true, true, true),
            budget: BIG_BUDGET,
        },
        RCell {
            name: "robertson-4e10_1e-5_b3b",
            problem: robertson_long(),
            rtol: 1.0e-5,
            arm: "b3b",
            options: alg06_options(true, true, false),
            budget: BIG_BUDGET,
        },
        RCell {
            name: "brusselator-1d-50_1e-6_proj_l2",
            problem: brusselator_problem(50),
            rtol: 1.0e-6,
            arm: "proj_l2",
            options: StageTargetOptions {
                production_fallback: true,
                ..StageTargetOptions::new(StageTargetPolicy::ProjL2)
            },
            budget: BIG_BUDGET,
        },
    ]
}

#[test]
fn test_r_parity_on_off_and_alg06_anchor() {
    let alg06_bytes = include_bytes!("../../../research/alg06_guard_v2_20261010/RUNS.json");
    let alg06_runs: Value = serde_json::from_slice(alg06_bytes).expect("valid alg06 RUNS.json");
    let alg06_rows = alg06_runs["rows"].as_array().expect("alg06 rows");

    let cells = r_cells();
    for cell in &cells {
        let p = &cell.problem;
        let linear = gmres_config(cell.budget);
        let adaptive = p.adaptive(cell.rtol);
        let schedule = p.schedule();

        // Run Telemetry OFF
        let off = integrate_rodas5p_mf_fast_observed_with_stage_target(
            &p.problem,
            p.t_span,
            &p.y0,
            &linear,
            &adaptive,
            &schedule,
            cell.options,
        )
        .unwrap();

        // Run Telemetry ON
        let on = integrate_rodas5p_mf_fast_observed_with_stage_telemetry(
            &p.problem,
            p.t_span,
            &p.y0,
            &linear,
            &adaptive,
            &schedule,
            cell.options,
        )
        .unwrap();

        // Item 1a: Telemetry on/off bitwise identical
        assert_eq!(
            on.result.attempts, off.result.attempts,
            "cell {}",
            cell.name
        );
        assert_eq!(
            on.result.accepted_steps, off.result.accepted_steps,
            "cell {}",
            cell.name
        );
        assert_eq!(
            on.result.rejected_steps, off.result.rejected_steps,
            "cell {}",
            cell.name
        );
        assert_eq!(
            on.result.observed.counters, off.result.observed.counters,
            "cell {}",
            cell.name
        );
        assert_eq!(
            on.result.observed.t, off.result.observed.t,
            "cell {}",
            cell.name
        );
        assert_eq!(
            on.result.observed.y, off.result.observed.y,
            "cell {}",
            cell.name
        );
        assert_eq!(on.statistics, off.statistics, "cell {}", cell.name);
        assert_eq!(on.charges, off.charges, "cell {}", cell.name);

        // Item 1b: Telemetry off equals ALG06 RUNS rows bitwise (where anchor exists)
        if cell.arm == "b3" || cell.arm == "b3b" {
            let matching_row = alg06_rows
                .iter()
                .find(|r| {
                    let case_match =
                        r["case"] == p.id || (p.id == "robertson" && r["case"] == "robertson-4e10");
                    let rtol_match = (r["rtol"].as_f64().unwrap() - cell.rtol).abs() < 1.0e-15;
                    case_match && rtol_match
                })
                .expect("matching alg06 row");

            let alg06_arm = &matching_row["arms"][cell.arm];
            assert_eq!(
                off.result.attempts,
                alg06_arm["attempts"].as_u64().unwrap() as usize
            );
            assert_eq!(
                off.result.accepted_steps,
                alg06_arm["accepted"].as_u64().unwrap() as usize
            );
            assert_eq!(
                off.result.rejected_steps,
                alg06_arm["rejected"].as_u64().unwrap() as usize
            );
            assert_eq!(
                off.result
                    .observed
                    .t
                    .iter()
                    .map(|x| format!("{:016x}", x.to_bits()))
                    .collect::<Vec<_>>(),
                alg06_arm["t"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                off.result
                    .observed
                    .y
                    .last()
                    .unwrap()
                    .iter()
                    .map(|x| format!("{:016x}", x.to_bits()))
                    .collect::<Vec<_>>(),
                alg06_arm["y_last"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect::<Vec<_>>()
            );
        }

        // Gate Item 2: No hidden JVP or copy
        let tel = &on.telemetry;
        assert_eq!(
            tel.work_all_attempts.jvp_calls + tel.jvp_outside_attempts,
            on.result.observed.counters.jvp_calls,
            "JVP closure failed on cell {}",
            cell.name
        );
        for eval in &tel.fallback_evaluations {
            let copies = eval.vector_copies();
            assert!(copies <= 2);
        }

        // Gate Item 4: Record completeness
        assert_eq!(tel.stage_records.len(), on.statistics.solves as usize);
        assert_eq!(tel.attempt_records.len(), on.result.attempts);
        for rec in &tel.stage_records {
            assert!(rec.telemetry.requested_target <= rec.telemetry.raised_target);
            assert!(rec.telemetry.tightened_target <= rec.telemetry.raised_target);
            assert_eq!(
                rec.telemetry.verified_bound,
                rodas5p_krylov::VerifiedBound::Unavailable
            );
        }

        // Gate Item 5: Ledgers
        assert_eq!(
            tel.committed_debit,
            tel.attempt_records
                .iter()
                .filter(|a| a.outcome == AttemptResidualOutcome::Committed)
                .map(|a| a.debit_all)
                .sum::<f64>()
        );
        for (ai, a) in tel.attempt_records.iter().enumerate() {
            if a.debit_all < a.debit_fallback_only {
                let stages: Vec<_> = tel
                    .stage_records
                    .iter()
                    .filter(|s| s.attempt == ai)
                    .collect();
                panic!(
                    "Cell {}: attempt {}: debit_all {} < debit_fallback_only {}\nAttempt: {:?}\nStages: {:#?}",
                    cell.name, ai, a.debit_all, a.debit_fallback_only, a, stages
                );
            }
            assert!(a.debit_all >= a.debit_fallback_only);
        }

        // Gate Item 6: Metric
        if matches!(
            cell.options.policy,
            StageTargetPolicy::Coupled
                | StageTargetPolicy::CoupledGuarded
                | StageTargetPolicy::CoupledGuarded2
        ) {
            for rec in &tel.stage_records {
                assert!(matches!(rec.metric, ResidualMetric::ScaledWrms { .. }));
            }
        } else if cell.options.policy == StageTargetPolicy::ProjL2 {
            for rec in &tel.stage_records {
                assert_eq!(rec.metric, ResidualMetric::UnscaledL2);
                assert_eq!(rec.tau_e, 0.0);
                assert_eq!(rec.heuristic_debit, 0.0);
            }
        }
    }
}

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

fn report_to_json(r: &StagedGmresReport) -> Value {
    json!({
        "outcome": format!("{:?}", r.outcome),
        "failure": r.failure.map(|f| format!("{:?}", f)),
        "guard_abort": r.guard_abort.map(|g| format!("{:?}", g)),
        "shadow_converged": r.shadow_converged,
        "exhaustion": r.exhaustion,
        "budget_exhausted": r.budget_exhausted,
        "right_norm": r.right_norm,
        "threshold": r.threshold,
        "final_threshold": r.final_threshold,
        "residual_norm": r.residual_norm,
        "residual_norm_bits": format!("{:016x}", r.residual_norm.to_bits()),
        "threshold_bits": format!("{:016x}", r.threshold.to_bits()),
        "columns": r.columns,
        "matvecs": r.matvecs,
        "cycles": r.cycles,
        "true_residuals": r.true_residuals,
        "confirmations": r.confirmations,
        "failed_confirmations": r.failed_confirmations,
        "nu_evaluations": r.nu_evaluations,
        "nu_max": r.nu_max,
        "nu_flops": r.nu_flops,
    })
}

#[test]
fn test_telemetry_off_and_reserved_capacity_contract() {
    let cells = r_cells();
    let cell = &cells[0]; // robertson-4e10 at 1e-5
    let p = &cell.problem;
    let linear = gmres_config(cell.budget);
    let adaptive = p.adaptive(cell.rtol);
    let schedule = p.schedule();

    // (a) Telemetry off: stage_telemetry() is None throughout
    {
        let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
        work.set_stage_telemetry(false);
        assert!(work.stage_telemetry().is_none());
        let _ = work
            .integrate_observed_with_observer(
                &p.problem,
                p.t_span,
                &p.y0,
                &adaptive,
                &schedule,
                &mut |_, _, _, _, _| {},
            )
            .unwrap();
        assert!(work.stage_telemetry().is_none());
    }

    // (b) Telemetry on with reserve_stage_telemetry: capacities do not grow during the run
    {
        let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
        work.set_gmres_into(true);
        let (res_attempts, res_stages, res_fallbacks) = (2000, 10000, 500);
        work.reserve_stage_telemetry(res_attempts, res_stages, res_fallbacks);
        assert!(work.stage_telemetry().is_some());

        let initial_cap_attempts = work.stage_telemetry().unwrap().attempt_records.capacity();
        let initial_cap_stages = work.stage_telemetry().unwrap().stage_records.capacity();
        let initial_cap_fallbacks = work
            .stage_telemetry()
            .unwrap()
            .fallback_evaluations
            .capacity();

        assert!(initial_cap_attempts >= res_attempts);
        assert!(initial_cap_stages >= res_stages);
        assert!(initial_cap_fallbacks >= res_fallbacks);

        work.set_stage_target(cell.options).unwrap();
        work.set_stage_target_span(p.t_span.1 - p.t_span.0);
        work.clear_accepted_error();

        let _ = work
            .integrate_observed_with_observer(
                &p.problem,
                p.t_span,
                &p.y0,
                &adaptive,
                &schedule,
                &mut |_, _, _, _, _| {},
            )
            .unwrap();

        let tel = work.stage_telemetry().unwrap();
        assert!(!tel.attempt_records.is_empty());
        assert!(!tel.stage_records.is_empty());
        assert_eq!(tel.attempt_records.capacity(), initial_cap_attempts);
        assert_eq!(tel.stage_records.capacity(), initial_cap_stages);
        assert_eq!(tel.fallback_evaluations.capacity(), initial_cap_fallbacks);
    }
}

#[test]
fn test_workspace_clone_telemetry_isolation() {
    let cells = r_cells();
    let cell = &cells[0];
    let p = &cell.problem;
    let linear = gmres_config(cell.budget);
    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
    work.set_stage_telemetry(true);

    let cloned = work.clone();
    assert!(cloned.stage_telemetry().is_some());
    assert_eq!(cloned.stage_telemetry().unwrap().stage_records.len(), 0);
    assert_eq!(cloned.stage_telemetry().unwrap().attempt_records.len(), 0);
    assert_eq!(
        cloned.stage_telemetry().unwrap().fallback_evaluations.len(),
        0
    );

    let mut work_off = Rodas5pMfFastWorkspace::new(&p.problem, &linear).unwrap();
    work_off.set_stage_telemetry(false);
    let cloned_off = work_off.clone();
    assert!(cloned_off.stage_telemetry().is_none());
}

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

fn export_solver_cases() -> Value {
    let id2 = DenseMatrix::identity(2);
    let tri2 = DenseMatrix::from_rows(&[&[3.1, 0.7], &[0.0, 2.3]]).unwrap();
    let tri4 = DenseMatrix::from_rows(&[
        &[1.3, 0.31, 0.17, 0.19],
        &[0.0, 2.7, 0.31, 0.17],
        &[0.0, 0.0, 3.1, 0.31],
        &[0.0, 0.0, 0.0, 4.9],
    ])
    .unwrap();

    let s1_specs = [
        ("identity2", id2.clone(), vec![1.0, 0.0]),
        ("triangular2", tri2.clone(), vec![0.13, 0.29]),
        ("triangular4", tri4.clone(), vec![0.7, 1.1, -0.3, 0.9]),
    ];
    let modes = ["strict", "stall", "floor"];

    let mut s1_cases = Vec::new();
    for (name, mat, rhs) in &s1_specs {
        for &mode in &modes {
            let mut config = StagedGmresConfig::new(rhs.len(), 16, 1.0e-30, 0.0);
            config.stall_rule = mode == "stall";
            config.floor_at_confirmations = mode == "floor";

            let op = DenseOperator::new(mat.clone()).unwrap();
            let mut output = vec![f64::NAN; rhs.len()];
            let mut counters = WorkCounters::default();
            let right_norm = safe_l2(rhs);
            let requested_target = config.atol.max(config.rtol * right_norm);
            let mut tel = init_tel(requested_target);

            let report = solve_staged_gmres_observed(
                &op,
                rhs,
                &config,
                None,
                &mut output,
                &mut StagedGmresWorkspace::default(),
                &mut counters,
                requested_target,
                &mut tel,
            )
            .expect("solve succeeds");

            s1_cases.push(json!({
                "id": format!("{}_{}", name, mode),
                "case": name,
                "mode": mode,
                "report": report_to_json(&report),
                "telemetry": tel,
                "output": hexes(&output),
            }));
        }
    }

    let mut cyclic4 = DenseMatrix::zeros(4, 4);
    for j in 0..4 {
        cyclic4[((j + 1) % 4, j)] = 1.0;
    }
    let cyclic4_op = DenseOperator::new(cyclic4).unwrap();

    let s2_cases = {
        let scaled_rhs = [1.5, 1.5, 0.0, 0.0];
        let seam_overflow = {
            let mut fallback = |_: &[f64], _: &[f64]| false;
            let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
            let mut output = [f64::NAN; 4];
            let mut counters = WorkCounters::default();
            let mut tel = init_tel(1.0e-14 * safe_l2(&scaled_rhs));
            let report = solve_staged_gmres_observed(
                &cyclic4_op,
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
            json!({
                "report": report_to_json(&report),
                "telemetry": tel,
                "failed": report.outcome == StagedGmresOutcome::Failed,
            })
        };

        let seam_finite = {
            let mut fallback = |_: &[f64], _: &[f64]| false;
            let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
            let mut output = [f64::NAN; 4];
            let mut counters = WorkCounters::default();
            let mut tel = init_tel(1.0e-14 * safe_l2(&scaled_rhs));
            let report = solve_staged_gmres_observed(
                &cyclic4_op,
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
            json!({
                "report": report_to_json(&report),
                "telemetry": tel,
                "failed": report.outcome == StagedGmresOutcome::Failed,
            })
        };

        let fallback_finite = {
            let mut fallback = |_: &[f64], _: &[f64]| true;
            let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
            let mut output = [f64::NAN; 4];
            let mut counters = WorkCounters::default();
            let requested_target = 1.0e-14 * safe_l2(&scaled_rhs);
            let mut tel = init_tel(requested_target);
            let report = solve_staged_gmres_observed(
                &cyclic4_op,
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
            json!({
                "report": report_to_json(&report),
                "telemetry": tel,
            })
        };

        let fallback_homog = {
            let mut fallback = |_: &[f64], _: &[f64]| true;
            let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
            let mut output = [f64::NAN; 4];
            let mut counters = WorkCounters::default();
            let requested_target = 1.0e-14;
            let mut tel = init_tel(requested_target);
            let report = solve_staged_gmres_observed(
                &cyclic4_op,
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
            json!({
                "report": report_to_json(&report),
                "telemetry": tel,
            })
        };

        json!({
            "seam_overflow": seam_overflow,
            "seam_finite": seam_finite,
            "fallback_accepted_finite": fallback_finite,
            "fallback_accepted_homogeneous": fallback_homog,
        })
    };

    let s3_cases = {
        let n = 50;
        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..n {
            a[((i + 1) % n, i)] = 1.0;
        }
        let dense_op = DenseOperator::new(a).unwrap();
        let mut b = vec![0.0; n];
        b[0] = 1.0;

        let classify_on = {
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

            let wrapper_shadow = counting_op.calls() - counters.linear_matvecs;
            json!({
                "report": report_to_json(&report),
                "telemetry": tel,
                "wrapper_shadow_matvecs": wrapper_shadow,
            })
        };

        let classify_off = {
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

            json!({
                "report": report_to_json(&report),
                "telemetry": tel,
                "wrapper_shadow_matvecs": 0,
            })
        };

        json!({
            "classify_on": classify_on,
            "classify_off": classify_off,
        })
    };

    let s4_case = {
        let matrix = DenseMatrix::from_rows(&[&[2.0, 0.0], &[0.0, 1.0]]).unwrap();
        let op = DenseOperator::new(matrix).unwrap();
        let b = vec![10.0, 10.0];
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

        json!({
            "report": report_to_json(&report),
            "telemetry": tel,
        })
    };

    let s5_cases = {
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
        let nan_op = NanOnResidualOp(AtomicU64::new(0));
        let b = vec![1.0, 1.0];
        let config = StagedGmresConfig::new(1, 2, 1.0e-10, 0.0);
        let mut output = vec![f64::NAN; 2];
        let mut counters = WorkCounters::default();
        let mut tel = init_tel(1.0e-10);
        let non_finite_report = solve_staged_gmres_observed(
            &nan_op,
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

        let zero_mat = DenseMatrix::zeros(2, 2);
        let zero_op = DenseOperator::new(zero_mat).unwrap();
        let mut tel_bd = init_tel(1.0e-10);
        let mut output_bd = vec![f64::NAN; 2];
        let mut counters_bd = WorkCounters::default();
        let config_bd = StagedGmresConfig::new(2, 2, 1.0e-10, 0.0);
        let breakdown_report = solve_staged_gmres_observed(
            &zero_op,
            &b,
            &config_bd,
            None,
            &mut output_bd,
            &mut StagedGmresWorkspace::default(),
            &mut counters_bd,
            1.0e-10,
            &mut tel_bd,
        )
        .unwrap();

        let mut tel_bg = init_tel(1.0e-14);
        let mut output_bg = vec![f64::NAN; 4];
        let mut counters_bg = WorkCounters::default();
        let budget_config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
        let budget_report = solve_staged_gmres_observed(
            &cyclic4_op,
            &[1.0, 1.0, 0.0, 0.0],
            &budget_config,
            None,
            &mut output_bg,
            &mut StagedGmresWorkspace::default(),
            &mut counters_bg,
            1.0e-14,
            &mut tel_bg,
        )
        .unwrap();

        json!({
            "non_finite": {
                "report": report_to_json(&non_finite_report),
                "telemetry": tel,
            },
            "breakdown": {
                "report": report_to_json(&breakdown_report),
                "telemetry": tel_bd,
            },
            "budget": {
                "report": report_to_json(&budget_report),
                "telemetry": tel_bg,
            },
        })
    };

    json!({
        "s1": s1_cases,
        "s2": s2_cases,
        "s3": s3_cases,
        "s4": s4_case,
        "s5": s5_cases,
    })
}

// ---------------------------------------------------------------------------
// Export runs
// ---------------------------------------------------------------------------

#[test]
#[ignore = "recorded run of research/as04_stage_residual_telemetry_20261011; release build"]
fn export_runs() {
    let runs_arg = std::env::var("AS04_RUNS")
        .unwrap_or_else(|_| "research/as04_stage_residual_telemetry_20261011/RUNS.json".into());
    let path = std::path::Path::new(&runs_arg);
    let runs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(path)
    };

    let commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain=v1", "--untracked-files=no"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let git_status = if status_output.is_empty() {
        "clean"
    } else {
        "dirty"
    };

    let mut rows = Vec::new();

    // 1..7: R cells
    for cell in r_cells() {
        let p = &cell.problem;
        let linear = gmres_config(cell.budget);
        let adaptive = p.adaptive(cell.rtol);
        let schedule = p.schedule();

        let off = integrate_rodas5p_mf_fast_observed_with_stage_target(
            &p.problem,
            p.t_span,
            &p.y0,
            &linear,
            &adaptive,
            &schedule,
            cell.options,
        )
        .unwrap();

        let on = integrate_rodas5p_mf_fast_observed_with_stage_telemetry(
            &p.problem,
            p.t_span,
            &p.y0,
            &linear,
            &adaptive,
            &schedule,
            cell.options,
        )
        .unwrap();

        rows.push(json!({
            "id": cell.name,
            "case": p.id,
            "arm": cell.arm,
            "rtol": cell.rtol,
            "dimension": p.y0.len(),
            "telemetry_on": {
                "observed": {
                    "success": on.result.observed.success,
                    "message": on.result.observed.message,
                    "t": hexes(&on.result.observed.t),
                    "y": on.result.observed.y.iter().map(|state| hexes(state)).collect::<Vec<_>>(),
                    "y_last": hexes(on.result.observed.y.last().unwrap()),
                    "attempts": on.result.attempts,
                    "accepted": on.result.accepted_steps,
                    "rejected": on.result.rejected_steps,
                    "state_reuses": on.result.state_reuses,
                    "internal_steps": on.result.observed.internal_steps,
                    "output_clipped_steps": on.result.observed.output_clipped_steps,
                    "counters": on.result.observed.counters,
                },
                "stage_statistics": on.statistics,
                "charges": on.charges,
                "telemetry": on.telemetry,
            },
            "telemetry_off": {
                "observed": {
                    "success": off.result.observed.success,
                    "message": off.result.observed.message,
                    "t": hexes(&off.result.observed.t),
                    "y": off.result.observed.y.iter().map(|state| hexes(state)).collect::<Vec<_>>(),
                    "y_last": hexes(off.result.observed.y.last().unwrap()),
                    "attempts": off.result.attempts,
                    "accepted": off.result.accepted_steps,
                    "rejected": off.result.rejected_steps,
                    "state_reuses": off.result.state_reuses,
                    "internal_steps": off.result.observed.internal_steps,
                    "output_clipped_steps": off.result.observed.output_clipped_steps,
                    "counters": off.result.observed.counters,
                },
                "stage_statistics": off.statistics,
                "charges": off.charges,
            }
        }));
    }

    // 8: D1
    {
        let (problem, y0) = d1_problem();
        let (linear, adaptive, options) = d1_configs();
        let schedule = OutputSchedule::new(vec![0.0, 2.0 * D1_STEP]).unwrap();

        let off = integrate_rodas5p_mf_fast_observed_with_stage_target(
            &problem,
            (0.0, 2.0 * D1_STEP),
            &y0,
            &linear,
            &adaptive,
            &schedule,
            options,
        )
        .unwrap();

        let on = integrate_rodas5p_mf_fast_observed_with_stage_telemetry(
            &problem,
            (0.0, 2.0 * D1_STEP),
            &y0,
            &linear,
            &adaptive,
            &schedule,
            options,
        )
        .unwrap();

        rows.push(json!({
            "id": "D1_transactional_failure",
            "case": problem.name,
            "arm": "D1",
            "rtol": 1.0e-6,
            "dimension": 4,
            "telemetry_on": {
                "observed": {
                    "success": on.result.observed.success,
                    "message": on.result.observed.message,
                    "t": hexes(&on.result.observed.t),
                    "y": on.result.observed.y.iter().map(|state| hexes(state)).collect::<Vec<_>>(),
                    "y_last": hexes(on.result.observed.y.last().unwrap_or(&vec![])),
                    "attempts": on.result.attempts,
                    "accepted": on.result.accepted_steps,
                    "rejected": on.result.rejected_steps,
                    "state_reuses": on.result.state_reuses,
                    "internal_steps": on.result.observed.internal_steps,
                    "output_clipped_steps": on.result.observed.output_clipped_steps,
                    "counters": on.result.observed.counters,
                },
                "stage_statistics": on.statistics,
                "charges": on.charges,
                "telemetry": on.telemetry,
            },
            "telemetry_off": {
                "observed": {
                    "success": off.result.observed.success,
                    "message": off.result.observed.message,
                    "t": hexes(&off.result.observed.t),
                    "y": off.result.observed.y.iter().map(|state| hexes(state)).collect::<Vec<_>>(),
                    "y_last": hexes(off.result.observed.y.last().unwrap_or(&vec![])),
                    "attempts": off.result.attempts,
                    "accepted": off.result.accepted_steps,
                    "rejected": off.result.rejected_steps,
                    "state_reuses": off.result.state_reuses,
                    "internal_steps": off.result.observed.internal_steps,
                    "output_clipped_steps": off.result.observed.output_clipped_steps,
                    "counters": off.result.observed.counters,
                },
                "stage_statistics": off.statistics,
                "charges": off.charges,
            }
        }));
    }

    // 9: D2
    {
        let epoch_on = Arc::new(AtomicU64::new(100));
        let (problem_on, y0) = d2_problem(epoch_on.clone());
        let (linear, adaptive, options) = d2_configs();
        let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();

        let mut work_on = Rodas5pMfFastWorkspace::new(&problem_on, &linear).unwrap();
        work_on.set_gmres_into(true);
        work_on.set_stage_telemetry(true);
        work_on.set_stage_target(options).unwrap();
        work_on.set_stage_target_span(1.0);

        let mut attempts_on = 0;
        let (result_on, stats_on, charges_on, _) = work_on
            .integrate_observed_with_observer(
                &problem_on,
                (0.0, 1.0),
                &y0,
                &adaptive,
                &schedule,
                &mut |_, _, _, _, _| {
                    attempts_on += 1;
                    if attempts_on == 3 {
                        epoch_on.store(200, Ordering::SeqCst);
                    }
                },
            )
            .unwrap();
        let tel_on = work_on.stage_telemetry().cloned().unwrap();

        let epoch_off = Arc::new(AtomicU64::new(100));
        let (problem_off, _) = d2_problem(epoch_off.clone());
        let mut work_off = Rodas5pMfFastWorkspace::new(&problem_off, &linear).unwrap();
        work_off.set_gmres_into(true);
        work_off.set_stage_telemetry(false);
        work_off.set_stage_target(options).unwrap();
        work_off.set_stage_target_span(1.0);

        let mut attempts_off = 0;
        let (result_off, stats_off, charges_off, _) = work_off
            .integrate_observed_with_observer(
                &problem_off,
                (0.0, 1.0),
                &y0,
                &adaptive,
                &schedule,
                &mut |_, _, _, _, _| {
                    attempts_off += 1;
                    if attempts_off == 3 {
                        epoch_off.store(200, Ordering::SeqCst);
                    }
                },
            )
            .unwrap();

        rows.push(json!({
            "id": "D2_epoch",
            "case": problem_on.name,
            "arm": "D2",
            "rtol": 1.0e-8,
            "dimension": 2,
            "telemetry_on": {
                "observed": {
                    "success": result_on.observed.success,
                    "message": result_on.observed.message,
                    "t": hexes(&result_on.observed.t),
                    "y": result_on.observed.y.iter().map(|state| hexes(state)).collect::<Vec<_>>(),
                    "y_last": hexes(result_on.observed.y.last().unwrap_or(&vec![])),
                    "attempts": result_on.attempts,
                    "accepted": result_on.accepted_steps,
                    "rejected": result_on.rejected_steps,
                    "state_reuses": result_on.state_reuses,
                    "internal_steps": result_on.observed.internal_steps,
                    "output_clipped_steps": result_on.observed.output_clipped_steps,
                    "counters": result_on.observed.counters,
                },
                "stage_statistics": stats_on,
                "charges": charges_on,
                "telemetry": tel_on,
            },
            "telemetry_off": {
                "observed": {
                    "success": result_off.observed.success,
                    "message": result_off.observed.message,
                    "t": hexes(&result_off.observed.t),
                    "y": result_off.observed.y.iter().map(|state| hexes(state)).collect::<Vec<_>>(),
                    "y_last": hexes(result_off.observed.y.last().unwrap_or(&vec![])),
                    "attempts": result_off.attempts,
                    "accepted": result_off.accepted_steps,
                    "rejected": result_off.rejected_steps,
                    "state_reuses": result_off.state_reuses,
                    "internal_steps": result_off.observed.internal_steps,
                    "output_clipped_steps": result_off.observed.output_clipped_steps,
                    "counters": result_off.observed.counters,
                },
                "stage_statistics": stats_off,
                "charges": charges_off,
            }
        }));
    }

    let runs_doc = json!({
        "schema": "vigilode-as04-runs-v1",
        "node": "as04_stage_residual_telemetry_20261011",
        "export": "export_runs",
        "commit": commit,
        "git_status": git_status,
        "rows": rows,
        "solver_cases": export_solver_cases(),
    });

    let target_path = std::path::Path::new(&runs_path);
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(
        target_path,
        serde_json::to_string_pretty(&runs_doc).unwrap() + "\n",
    )
    .unwrap();
    println!("wrote {}", target_path.display());
}
