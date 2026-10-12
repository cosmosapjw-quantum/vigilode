//! Research node `research/as04_stage_residual_telemetry_20261011` (AS04):
//! residual and budget telemetry of every staged stage solve and every
//! attempt of the U-form driver.
//!
//! The contract tests check that the opt-in telemetry changes nothing
//! (telemetry on and off are bitwise identical runs), that its records are
//! complete and consistent with `StageSolveStatistics`, the work counters
//! and the ALG06 charge log, and the registered driver fixtures D1
//! (transactional linear failure) and D2 (model epoch). The recorded run
//! (`export_runs`) writes `AS04_RUNS` only when it is set; the gate is
//! evaluated by `tools/as04_stage_telemetry_check.py`.
//!
//! Telemetry only: no record is an error certificate.

// (The cells and records of `alg01_common` are shared with ALG01-ALG06.)
#[allow(unused_imports)]
mod alg01_common;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use alg01_common::*;
use rodas5p_core::{
    CoreResult, DenseMatrix, DenseOperator, InitialGuess, LinearMethod, LinearOperator,
    LinearSolverConfig, WorkCounters, rodas5p_coefficients, safe_l2,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, AttemptOutcome, AttemptResidualRecord, FallbackBranch, FallbackEvaluation,
    FallbackExit, OdeProblem, OutputSchedule, ProductionFallbackRule, ResidualMetric,
    Rodas5pMfStageTargetResult, Rodas5pMfStageTelemetryResult, StageResidualRecord,
    StageResidualTelemetry, StageTargetOptions, StageTargetPolicy,
    integrate_rodas5p_mf_fast_observed_with_stage_target,
    integrate_rodas5p_mf_fast_observed_with_stage_telemetry,
};
use rodas5p_krylov::{
    StagedAcceptanceReason, StagedGmresConfig, StagedGmresReport, StagedGmresWorkspace,
    StagedSolveTelemetry, TargetAttainment, VerifiedBound, solve_staged_gmres_observed,
};
use serde_json::{Value, json};

// The record types are `Copy` (registered compile-time assertion).
const _: fn() = || {
    fn assert_copy<T: Copy>() {}
    assert_copy::<StageResidualRecord>();
    assert_copy::<AttemptResidualRecord>();
    assert_copy::<FallbackEvaluation>();
    assert_copy::<StagedSolveTelemetry>();
};

/// RODAS5P stages.
const STAGES: u64 = 8;

// ------------------------------------------------------------- runs

/// The ALG06 arms, cumulative on the `CoupledGuarded2` target
/// (`alg04_alg06.rs::alg06_options`, transcribed).
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

/// `ProjL2` with the production fallback.
fn proj_l2_fallback() -> StageTargetOptions {
    StageTargetOptions {
        production_fallback: true,
        ..StageTargetOptions::new(StageTargetPolicy::ProjL2)
    }
}

/// One driver configuration: problem, span, start, linear and adaptive
/// configuration, output schedule and options.
struct DriverCase<'a> {
    problem: &'a OdeProblem,
    t_span: (f64, f64),
    y0: &'a [f64],
    linear: LinearSolverConfig,
    adaptive: AdaptiveStepConfig,
    schedule: OutputSchedule,
    options: StageTargetOptions,
}

impl DriverCase<'_> {
    fn off(&self) -> CoreResult<Rodas5pMfStageTargetResult> {
        integrate_rodas5p_mf_fast_observed_with_stage_target(
            self.problem,
            self.t_span,
            self.y0,
            &self.linear,
            &self.adaptive,
            &self.schedule,
            self.options,
        )
    }
    fn on(&self) -> CoreResult<Rodas5pMfStageTelemetryResult> {
        integrate_rodas5p_mf_fast_observed_with_stage_telemetry(
            self.problem,
            self.t_span,
            self.y0,
            &self.linear,
            &self.adaptive,
            &self.schedule,
            self.options,
        )
    }
}

fn problem_case(
    p: &Problem,
    rtol: f64,
    budget: usize,
    options: StageTargetOptions,
) -> DriverCase<'_> {
    DriverCase {
        problem: &p.problem,
        t_span: p.t_span,
        y0: &p.y0,
        linear: gmres_config(budget),
        adaptive: p.adaptive(rtol),
        schedule: p.schedule(),
        options,
    }
}

// --------------------------------------------------------- serialization

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn hex_u64(x: u64) -> String {
    format!("{x:016x}")
}

fn debug<T: std::fmt::Debug>(x: T) -> String {
    format!("{x:?}")
}

/// The run's result: every output time and state, the step counts, the
/// counters, `StageSolveStatistics` (floats as bits) and the charge log.
fn result_value(run: &Rodas5pMfStageTargetResult) -> Value {
    let r = &run.result;
    let s = serde_json::to_value(run.statistics).unwrap();
    let mut statistics = serde_json::Map::new();
    for (k, v) in s.as_object().unwrap() {
        let value = match k.as_str() {
            "nu_max" | "charge_sum" | "charge_max" => json!(hex(v.as_f64().unwrap())),
            _ => v.clone(),
        };
        statistics.insert(k.clone(), value);
    }
    json!({
        "ok": true,
        "success": r.observed.success,
        "message": r.observed.message,
        "t": hexes(&r.observed.t),
        "y": r.observed.y.iter().map(|y| hexes(y)).collect::<Vec<_>>(),
        "attempts": r.attempts,
        "accepted": r.accepted_steps,
        "rejected": r.rejected_steps,
        "state_reuses": r.state_reuses,
        "internal_steps": r.observed.internal_steps,
        "output_clipped_steps": r.observed.output_clipped_steps,
        "counters": serde_json::to_value(r.observed.counters).unwrap(),
        "stage_statistics": statistics,
        "charges": run.charges.iter().map(|c| json!([hex(c.t), hex(c.h), hex(c.error), hex(c.charge), c.stages, c.charged])).collect::<Vec<_>>(),
    })
}

fn result_or_error(run: &CoreResult<Rodas5pMfStageTargetResult>) -> Value {
    match run {
        Ok(r) => result_value(r),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

fn solver_value(t: &StagedSolveTelemetry) -> Value {
    json!({
        "reason": debug(t.reason),
        "requested_target": hex(t.requested_target),
        "raised_target": hex(t.raised_target),
        "tightened_target": hex(t.tightened_target),
        "attainable_floor": hex(t.attainable_floor),
        "actual_residual": hex(t.actual_residual),
        "actual_residual_inf": hex(t.actual_residual_inf),
        "attainment": debug(t.attainment),
        "heuristic_gain": {
            "label": "heuristic",
            "nu_max": hex(t.heuristic_gain.nu_max),
            "nu_evaluations": t.heuristic_gain.nu_evaluations,
        },
        "verified_bound": debug(t.verified_bound),
        "arnoldi_jvps": t.arnoldi_jvps,
        "residual_jvps": t.residual_jvps,
        "fallback_evaluations": t.fallback_evaluations,
        "acceptance_vector_copies": t.acceptance_vector_copies,
        "nu_flops": t.nu_flops,
        "uncounted_shadow_matvecs": t.uncounted_shadow_matvecs,
        "columns": t.columns,
        "matvecs": t.matvecs,
        "guard_abort": t.guard_abort.map(debug),
        "budget_exhausted": t.budget_exhausted,
    })
}

fn evaluation_value(e: &FallbackEvaluation) -> Value {
    json!({
        "branch": debug(e.branch),
        "exit": debug(e.exit),
        "r_le_atol": e.r_le_atol,
        "vector_copies": e.vector_copies,
        "accepted": e.accepted,
    })
}

fn stage_value(r: &StageResidualRecord) -> Value {
    let metric = match r.metric {
        ResidualMetric::ScaledWrms {
            atol_bits,
            rtol_bits,
            state_digest,
        } => json!({
            "kind": "ScaledWrms",
            "atol": hex_u64(atol_bits),
            "rtol": hex_u64(rtol_bits),
            "state_digest": hex_u64(state_digest),
        }),
        ResidualMetric::UnscaledL2 => json!({"kind": "UnscaledL2"}),
    };
    json!({
        "attempt": r.attempt,
        "stage": r.stage,
        "solver": solver_value(&r.solver),
        "metric": metric,
        "operator_epoch": {
            "model_epoch": r.operator_epoch.model_epoch,
            "generation": r.operator_epoch.generation,
        },
        "h": hex_u64(r.h_bits),
        "gamma": hex_u64(r.gamma_bits),
        "tau_e": hex(r.tau_e),
        "debit": hex(r.debit),
        "fallback": r.fallback.as_ref().map(evaluation_value),
        "jvp_calls": r.jvp_calls,
    })
}

fn work_value(w: &rodas5p_integrators::AttemptWork) -> Value {
    json!({
        "jvp_calls": w.jvp_calls,
        "linear_matvecs": w.linear_matvecs,
        "rhs_calls": w.rhs_calls,
        "krylov_columns": w.krylov_columns,
        "arnoldi_jvps": w.arnoldi_jvps,
        "residual_jvps": w.residual_jvps,
        "shadow_matvecs": w.shadow_matvecs,
        "vector_copies": w.vector_copies,
    })
}

fn attempt_value(a: &AttemptResidualRecord) -> Value {
    json!({
        "attempt": a.attempt,
        "t": hex(a.t),
        "h": hex(a.h),
        "outcome": debug(a.outcome),
        "operator_epoch": {
            "model_epoch": a.operator_epoch.model_epoch,
            "generation": a.operator_epoch.generation,
        },
        "relinearized": a.relinearized,
        "stages": a.stages,
        "converged": a.converged,
        "stall_accepted": a.stall_accepted,
        "floor_accepted": a.floor_accepted,
        "fallback_accepted": a.fallback_accepted,
        "failed": a.failed,
        "budget_exceeded": a.budget_exceeded,
        "max_actual_over_requested": hex(a.max_actual_over_requested),
        "debit_all": hex(a.debit_all),
        "debit_fallback_only": hex(a.debit_fallback_only),
        "error_norm": hex(a.error_norm),
        "work": work_value(&a.work),
    })
}

fn telemetry_memory(t: &StageResidualTelemetry) -> usize {
    t.stages.len() * std::mem::size_of::<StageResidualRecord>()
        + t.attempts.len() * std::mem::size_of::<AttemptResidualRecord>()
}

fn records_value(t: &StageResidualTelemetry) -> Value {
    json!({
        "stages": t.stages.iter().map(stage_value).collect::<Vec<_>>(),
        "attempts": t.attempts.iter().map(attempt_value).collect::<Vec<_>>(),
        "committed_debit": hex(t.committed_debit),
        "work_all_attempts": work_value(&t.work_all_attempts),
        "jvp_outside_attempts": t.jvp_outside_attempts,
        "memory_bytes": telemetry_memory(t),
    })
}

fn report_value(r: &StagedGmresReport) -> Value {
    json!({
        "outcome": debug(r.outcome),
        "failure": r.failure.map(debug),
        "accepted": r.accepted(),
        "threshold": hex(r.threshold),
        "final_threshold": hex(r.final_threshold),
        "residual_norm": hex(r.residual_norm),
        "residual_is_finite": r.residual_norm.is_finite(),
        "projected_residual": hex(r.projected_residual),
        "right_norm": hex(r.right_norm),
        "columns": r.columns,
        "cycles": r.cycles,
        "true_residuals": r.true_residuals,
        "confirmations": r.confirmations,
        "failed_confirmations": r.failed_confirmations,
        "budget_exhausted": r.budget_exhausted,
        "guard_abort": r.guard_abort.map(debug),
        "matvecs": r.matvecs,
        "nu_max": hex(r.nu_max),
        "nu_flops": r.nu_flops,
        "nu_evaluations": r.nu_evaluations,
        "shadow_converged": r.shadow_converged,
        "exhaustion": r.exhaustion,
    })
}

// ------------------------------------------------------ in-test checks

/// The checks of the registered gate items on one telemetry-on run (the
/// recorded run is re-checked by the Python checker).
fn check_records(run: &Rodas5pMfStageTelemetryResult, n: usize) {
    let t = &run.telemetry;
    let s = run.run.statistics;
    let result = &run.run.result;
    // Item 4: complete records.
    assert_eq!(t.stages.len() as u64, s.solves);
    let count = |reason| {
        t.stages
            .iter()
            .filter(|r| r.solver.reason == reason)
            .count() as u64
    };
    assert_eq!(count(StagedAcceptanceReason::Converged), s.converged);
    assert_eq!(
        count(StagedAcceptanceReason::StallAccepted),
        s.stall_accepted
    );
    assert_eq!(
        count(StagedAcceptanceReason::FloorAccepted),
        s.floor_accepted
    );
    assert_eq!(
        count(StagedAcceptanceReason::FallbackAccepted),
        s.fallback_accepted
    );
    assert_eq!(count(StagedAcceptanceReason::Failed), s.failed);
    assert_eq!(t.attempts.len(), result.attempts);
    for (k, a) in t.attempts.iter().enumerate() {
        assert_eq!(a.attempt, k as u64);
    }
    for r in &t.stages {
        let v = &r.solver;
        assert_eq!(v.verified_bound, VerifiedBound::Unavailable);
        assert!(v.requested_target <= v.raised_target);
        assert!(v.tightened_target <= v.raised_target);
        // Item 2: no hidden application.
        assert_eq!(v.arnoldi_jvps + v.residual_jvps, v.matvecs);
        assert_eq!(r.jvp_calls, v.matvecs);
        let recorded = u64::from(r.fallback.is_some());
        assert_eq!(recorded, v.fallback_evaluations);
        assert_eq!(
            v.acceptance_vector_copies,
            r.fallback.map_or(0, |e| u64::from(e.vector_copies))
        );
        // Item 3.
        if v.reason != StagedAcceptanceReason::Failed && v.actual_residual > v.requested_target {
            assert_eq!(v.attainment, TargetAttainment::BudgetExceeded);
        }
        match v.reason {
            StagedAcceptanceReason::Converged => assert!(v.actual_residual <= v.tightened_target),
            StagedAcceptanceReason::StallAccepted | StagedAcceptanceReason::FloorAccepted => {
                assert!(v.tightened_target < v.actual_residual);
                assert!(v.actual_residual <= v.attainable_floor);
            }
            StagedAcceptanceReason::FallbackAccepted => {
                assert!(v.guard_abort.is_some() || v.budget_exhausted)
            }
            StagedAcceptanceReason::Failed => {}
        }
        match r.metric {
            ResidualMetric::ScaledWrms { .. } => {
                assert_eq!(
                    r.debit.to_bits(),
                    (r.tau_e * (v.actual_residual / (n as f64).sqrt())).to_bits()
                )
            }
            ResidualMetric::UnscaledL2 => assert!(r.debit.is_nan()),
        }
    }
    // Item 2: the attempts' JVPs and the JVPs outside attempts close.
    let attempt_jvps: u64 = t.attempts.iter().map(|a| a.work.jvp_calls).sum();
    assert_eq!(
        attempt_jvps + t.jvp_outside_attempts,
        result.observed.counters.jvp_calls
    );
    // Item 5: ledgers.
    let mut committed = 0.0;
    let mut work = rodas5p_integrators::AttemptWork::default();
    for a in &t.attempts {
        let stages: Vec<_> = t.stages.iter().filter(|r| r.attempt == a.attempt).collect();
        assert_eq!(stages.len() as u32, a.stages);
        let mut debit_all = 0.0;
        let mut debit_fallback = 0.0;
        for r in &stages {
            let accepted = r.solver.reason != StagedAcceptanceReason::Failed;
            if accepted
                && matches!(r.metric, ResidualMetric::ScaledWrms { .. })
                && r.solver.attainment != TargetAttainment::Met
            {
                debit_all += r.debit;
            }
            if r.solver.reason == StagedAcceptanceReason::FallbackAccepted {
                debit_fallback += r.debit;
            }
        }
        assert_eq!(debit_all.to_bits(), a.debit_all.to_bits());
        if matches!(
            stages.first().map(|r| r.metric),
            Some(ResidualMetric::ScaledWrms { .. })
        ) {
            assert_eq!(debit_fallback.to_bits(), a.debit_fallback_only.to_bits());
        }
        assert!(a.debit_all >= a.debit_fallback_only);
        if a.outcome == AttemptOutcome::Committed {
            committed += a.debit_all;
        }
        work.jvp_calls += a.work.jvp_calls;
        work.arnoldi_jvps += a.work.arnoldi_jvps;
        work.residual_jvps += a.work.residual_jvps;
        assert_eq!(
            a.work.arnoldi_jvps + a.work.residual_jvps,
            stages.iter().map(|r| r.solver.matvecs).sum::<u64>()
        );
    }
    assert_eq!(committed.to_bits(), t.committed_debit.to_bits());
    assert_eq!(work.jvp_calls, t.work_all_attempts.jvp_calls);
    assert_eq!(work.arnoldi_jvps, t.work_all_attempts.arnoldi_jvps);
    // The charge log: every charged attempt's charge is its
    // `debit_fallback_only`, in order.
    let charged: Vec<_> = t
        .attempts
        .iter()
        .filter(|a| a.fallback_accepted > 0 && !a.error_norm.is_nan())
        .collect();
    assert_eq!(charged.len(), run.run.charges.len());
    for (a, c) in charged.iter().zip(&run.run.charges) {
        assert_eq!(a.t.to_bits(), c.t.to_bits());
        assert_eq!(a.h.to_bits(), c.h.to_bits());
        assert_eq!(a.debit_fallback_only.to_bits(), c.charge.to_bits());
        assert_eq!(a.error_norm.to_bits(), c.error.to_bits());
    }
}

/// Telemetry on and off give the same run bit for bit.
fn assert_twins(case: &DriverCase<'_>) -> Rodas5pMfStageTelemetryResult {
    let off = case.off().unwrap();
    let on = case.on().unwrap();
    assert_eq!(result_value(&off), result_value(&on.run));
    assert_eq!(off.statistics, on.run.statistics);
    on
}

// ---------------------------------------------- fallback rule telemetry

/// SplitMix64.
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
    fn signed(&mut self, lo: f64, hi: f64) -> f64 {
        let e = lo + (hi - lo) * self.unit();
        let sign = if self.below(2) == 0 { 1.0 } else { -1.0 };
        sign * (1.0 + 9.0 * self.unit()) * 10f64.powf(e.floor())
    }
}

/// The documented copies of one evaluation (the registered table, and the
/// exits before the threshold comparison).
fn documented_copies(e: &FallbackEvaluation) -> u32 {
    let scaled = matches!(
        e.branch,
        FallbackBranch::ScaledLiteral
            | FallbackBranch::ScaledHomogeneous
            | FallbackBranch::ScaledUnrepresentable
    );
    match e.exit {
        FallbackExit::CandidateRejected => 0,
        FallbackExit::ResidualRejected => u32::from(scaled),
        FallbackExit::Compared => match (e.branch, e.r_le_atol) {
            (FallbackBranch::ScaledLiteral | FallbackBranch::ScaledUnrepresentable, _) => 1,
            (FallbackBranch::ScaledHomogeneous, Some(true)) => 1,
            (FallbackBranch::ScaledHomogeneous, _) => 2,
            (FallbackBranch::UnscaledLiteral | FallbackBranch::UnscaledUnrepresentable, _) => 0,
            (FallbackBranch::UnscaledHomogeneous, Some(true)) => 0,
            (FallbackBranch::UnscaledHomogeneous, _) => 1,
        },
    }
}

/// `evaluate` decides as `accepts` and makes the documented copies, on
/// every branch and exit.
#[test]
fn fallback_evaluation_is_the_rule_with_its_copies() {
    let mut rng = Rng(0x0a50_4f10_2026_1011);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200_000 {
        // A third of the cases have components near the overflow threshold
        // (the homogeneous branch).
        let huge = rng.below(3) == 0;
        let n = if huge { 2 } else { 1 } + rng.below(5) as usize;
        let (lo, hi) = match rng.below(3) {
            0 => (290.0, 308.0),
            _ => (-20.0, 20.0),
        };
        let mut rhs: Vec<f64> = if huge {
            (0..n).map(|_| (1.0 + 0.7 * rng.unit()) * 1.0e308).collect()
        } else {
            (0..n).map(|_| rng.signed(lo, hi)).collect()
        };
        if rng.below(20) == 0 {
            rhs[0] = f64::INFINITY;
        }
        let rtol = [0.0, 1.0e-10, 1.0, f64::MAX][rng.below(4) as usize];
        let atol = [0.0, 1.0e-14, 1.0e300][rng.below(3) as usize];
        let scale = (rng.below(2) == 0).then(|| {
            (0..n)
                .map(|_| rng.signed(-10.0, 10.0).abs())
                .collect::<Vec<f64>>()
        });
        let mut residual: Vec<f64> = (0..n).map(|_| rng.signed(lo - 12.0, hi)).collect();
        if rng.below(15) == 0 {
            residual[0] = f64::NAN;
        }
        let mut candidate: Vec<f64> = (0..n).map(|_| rng.signed(-5.0, 5.0)).collect();
        if rng.below(15) == 0 {
            candidate[0] = f64::INFINITY;
        }
        let rule = ProductionFallbackRule::new(&rhs, rtol, atol);
        let e = rule.evaluate(&candidate, &residual, scale.as_deref());
        assert_eq!(
            e.accepted,
            rule.accepts(&candidate, &residual, scale.as_deref())
        );
        assert_eq!(e.vector_copies, documented_copies(&e), "{e:?}");
        assert_eq!(e.r_le_atol.is_some(), e.exit == FallbackExit::Compared);
        if e.accepted {
            assert_eq!(e.exit, FallbackExit::Compared);
        }
        seen.insert(format!("{:?}/{:?}/{:?}", e.branch, e.exit, e.r_le_atol));
    }
    for branch in [
        "ScaledLiteral",
        "ScaledHomogeneous",
        "ScaledUnrepresentable",
        "UnscaledLiteral",
        "UnscaledHomogeneous",
        "UnscaledUnrepresentable",
    ] {
        for exit in ["CandidateRejected", "ResidualRejected", "Compared"] {
            assert!(
                seen.iter()
                    .any(|k| k.starts_with(&format!("{branch}/{exit}/"))),
                "{branch}/{exit} not exercised: {seen:?}"
            );
        }
    }
    for k in [
        "ScaledHomogeneous/Compared/Some(true)",
        "ScaledHomogeneous/Compared/Some(false)",
        "UnscaledHomogeneous/Compared/Some(false)",
    ] {
        assert!(seen.contains(k), "{k} not exercised");
    }
}

// ------------------------------------------------ driver contracts

/// A small Brusselator with restart 4 and budget 8: guard aborts the
/// production fallback accepts (`alg04_alg06.rs::fallback_case`).
fn fallback_case() -> (Problem, LinearSolverConfig, f64) {
    let mut p = brusselator_problem(30);
    p.t_span = (0.0, 1.0);
    let config = LinearSolverConfig {
        restart: 4,
        ..gmres_config(8)
    };
    (p, config, 1.0e-6)
}

#[test]
fn telemetry_on_and_off_runs_are_bitwise_identical() {
    let (p, config, rtol) = fallback_case();
    let arms = [
        alg06_options(true, true, true),
        alg06_options(true, true, false),
        alg06_options(false, false, false),
        proj_l2_fallback(),
        StageTargetOptions::new(StageTargetPolicy::ProjL2),
        StageTargetOptions::new(StageTargetPolicy::L2Coupled),
        StageTargetOptions::new(StageTargetPolicy::Coupled),
        StageTargetOptions::new(StageTargetPolicy::CoupledGuarded),
        StageTargetOptions::new(StageTargetPolicy::CoupledGuarded2),
    ];
    let mut fallback_records = 0;
    for options in arms {
        let case = DriverCase {
            linear: config.clone(),
            ..problem_case(&p, rtol, 8, options)
        };
        let on = assert_twins(&case);
        check_records(&on, p.y0.len());
        let coupled = matches!(
            options.policy,
            StageTargetPolicy::Coupled
                | StageTargetPolicy::CoupledGuarded
                | StageTargetPolicy::CoupledGuarded2
        );
        for r in &on.telemetry.stages {
            assert_eq!(
                matches!(r.metric, ResidualMetric::ScaledWrms { .. }),
                coupled
            );
            if !options.classify_guard_aborts {
                assert_eq!(r.solver.uncounted_shadow_matvecs, 0);
            }
            fallback_records += usize::from(r.fallback.is_some());
        }
        assert!(!on.telemetry.stages.is_empty());
    }
    assert!(fallback_records > 0);
    // `Legacy`: attempt records only, the run unchanged.
    let legacy = DriverCase {
        linear: config.clone(),
        ..problem_case(&p, rtol, 8, StageTargetOptions::default())
    };
    let on = assert_twins(&legacy);
    assert!(on.telemetry.stages.is_empty());
    assert_eq!(on.telemetry.attempts.len(), on.run.result.attempts);
}

/// ALG06 `B3` on the fallback case: accepted residuals above the target
/// are visible, the ledgers close, `G3`'s charge is `debit_fallback_only`.
#[test]
fn b3_records_expose_every_accepted_residual_above_target() {
    let (p, config, rtol) = fallback_case();
    let case = DriverCase {
        linear: config,
        ..problem_case(&p, rtol, 8, alg06_options(true, true, true))
    };
    let on = assert_twins(&case);
    check_records(&on, p.y0.len());
    let t = &on.telemetry;
    let exceeded = t
        .stages
        .iter()
        .filter(|r| r.solver.attainment == TargetAttainment::BudgetExceeded)
        .count();
    assert!(exceeded > 0);
    assert!(!on.run.charges.is_empty());
    assert!(
        t.attempts
            .iter()
            .any(|a| a.debit_all > a.debit_fallback_only)
    );
    assert!(t.committed_debit > 0.0);
    // The digest is that of the attempt's state: the first attempt starts
    // from y0.
    let first = t.stages[0];
    let ResidualMetric::ScaledWrms { state_digest, .. } = first.metric else {
        panic!("coupled metric")
    };
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for v in &p.y0 {
        for byte in v.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    assert_eq!(state_digest, hash);
}

// D1: the AS01 direct-driver boundary fixture
// (`residual_acceptance_contract.rs`, item 3, transcribed).

const D1_STEP: f64 = 5.0;

fn kappa() -> f64 {
    D1_STEP * rodas5p_coefficients().unwrap().gamma
}

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

/// D1: one attempt of `h = 5` with the coupled target and the production
/// fallback, restart = budget = 1, on the overflowing cyclic stage.
fn d1() -> (
    OdeProblem,
    Vec<f64>,
    LinearSolverConfig,
    AdaptiveStepConfig,
    StageTargetOptions,
) {
    let kappa = kappa();
    let c = [1.5e308 / kappa, 1.5e308 / kappa, 0.0, 0.0];
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-14,
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
    (
        cyclic_problem(c, kappa),
        vec![1.0e306; 4],
        linear,
        adaptive,
        options,
    )
}

fn d1_case<'a>(
    problem: &'a OdeProblem,
    y0: &'a [f64],
    linear: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    options: StageTargetOptions,
) -> DriverCase<'a> {
    DriverCase {
        problem,
        t_span: (0.0, 2.0 * D1_STEP),
        y0,
        linear: linear.clone(),
        adaptive: adaptive.clone(),
        schedule: OutputSchedule::new(vec![0.0, 2.0 * D1_STEP]).unwrap(),
        options,
    }
}

#[test]
fn d1_failed_linear_attempt_is_recorded_with_its_work() {
    let (problem, y0, linear, adaptive, options) = d1();
    let case = d1_case(&problem, &y0, &linear, &adaptive, options);
    let on = assert_twins(&case);
    check_records(&on, 4);
    let t = &on.telemetry;
    assert!(!on.run.result.observed.success);
    assert_eq!(t.attempts.len(), 1);
    let a = t.attempts[0];
    assert_eq!(a.outcome, AttemptOutcome::FailedLinear);
    assert_eq!(a.failed, 1);
    assert!(a.work.jvp_calls > 0);
    assert_eq!(t.work_all_attempts.jvp_calls, a.work.jvp_calls);
    assert_eq!(t.committed_debit, 0.0);
    let e = t.stages[0].fallback.expect("the fallback was consulted");
    assert_eq!(e.branch, FallbackBranch::ScaledHomogeneous);
    assert_eq!(e.r_le_atol, Some(false));
    assert_eq!(e.vector_copies, 2);
    assert!(!e.accepted);
    assert_eq!(t.stages[0].solver.acceptance_vector_copies, 2);
}

// D2: model epoch.

/// `y' = A y` with `A = [[-1, 1], [0, -1000]]`, the model epoch 1 for the
/// first three reads and 2 afterwards (the driver reads it once per
/// attempt).
fn d2_problem() -> OdeProblem {
    let a = [[-1.0, 1.0], [0.0, -1000.0]];
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..2 {
            out[i] = a[i][0] * y[0] + a[i][1] * y[1];
        }
        Ok(())
    });
    let jvp = Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        for i in 0..2 {
            out[i] = a[i][0] * v[0] + a[i][1] * v[1];
        }
        Ok(())
    });
    let reads = Arc::new(AtomicU64::new(0));
    OdeProblem::new(
        "as04-d2-linear-2",
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
    .with_model_epoch(Arc::new(move || {
        if reads.fetch_add(1, Ordering::SeqCst) < 3 {
            1
        } else {
            2
        }
    }))
}

const D2_Y0: [f64; 2] = [1.0, 1.0];

fn d2_case(problem: &OdeProblem) -> DriverCase<'_> {
    DriverCase {
        problem,
        t_span: (0.0, 1.0),
        y0: &D2_Y0,
        linear: gmres_config(BUDGET),
        adaptive: AdaptiveStepConfig {
            atol: 1.0e-8,
            rtol: 1.0e-6,
            initial_step: 0.5,
            max_step: 1.0,
            max_attempts: 5_000,
            ..AdaptiveStepConfig::default()
        },
        schedule: OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
        options: StageTargetOptions::new(StageTargetPolicy::CoupledGuarded2),
    }
}

#[test]
fn d2_generation_counts_relinearizations_and_the_epoch_changes_at_the_fourth_attempt() {
    // Separate problems: the epoch closure counts its own reads.
    let off_problem = d2_problem();
    let on_problem = d2_problem();
    let off = d2_case(&off_problem).off().unwrap();
    let on = d2_case(&on_problem).on().unwrap();
    assert_eq!(result_value(&off), result_value(&on.run));
    check_records(&on, 2);
    let t = &on.telemetry;
    assert!(t.attempts.len() >= 5);
    let mut generation = 0;
    for a in &t.attempts {
        generation += u64::from(a.relinearized);
        assert_eq!(a.operator_epoch.generation, generation);
        let expected = if a.attempt < 3 { 1 } else { 2 };
        assert_eq!(a.operator_epoch.model_epoch, Some(expected));
        // A re-linearization evaluates f(t, y) once more.
        assert_eq!(a.work.rhs_calls, STAGES - 1 + u64::from(a.relinearized));
        for r in t.stages.iter().filter(|r| r.attempt == a.attempt) {
            assert_eq!(r.operator_epoch, a.operator_epoch);
        }
    }
    assert!(t.attempts[3].relinearized);
    assert!(
        t.attempts.iter().any(|a| !a.relinearized),
        "a reused linearization keeps its generation"
    );
}

// -------------------------------------------------------- solver cells

/// `x -> A x`, counting every application.
struct Counting {
    inner: DenseOperator,
    applications: AtomicU64,
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

fn cyclic_shift(n: usize) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[((i + 1) % n, i)] = 1.0;
    }
    a
}

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

/// A production fallback of a solver cell: the rule and the scale of the
/// scaled variables (`None`: physical).
struct CellFallback {
    rule: ProductionFallbackRule,
    scale: Option<Vec<f64>>,
}

/// One solver cell: the observed solve with an optional production
/// fallback whose evaluations are recorded, the copies added to the
/// telemetry as the driver does.
fn solver_cell(
    cell: &str,
    id: &str,
    op: &dyn LinearOperator,
    rhs: &[f64],
    config: &StagedGmresConfig,
    fallback: Option<&CellFallback>,
    requested: f64,
) -> Value {
    let n = rhs.len();
    let mut output = vec![f64::NAN; n];
    let mut counters = WorkCounters::default();
    let mut telemetry = StagedSolveTelemetry::default();
    let mut evaluations: Vec<FallbackEvaluation> = Vec::new();
    let mut closure = |x: &[f64], r: &[f64]| {
        let f = fallback.expect("closure only with a fallback");
        let e = f.rule.evaluate(x, r, f.scale.as_deref());
        evaluations.push(e);
        e.accepted
    };
    let report = solve_staged_gmres_observed(
        op,
        rhs,
        config,
        fallback
            .is_some()
            .then_some(&mut closure as &mut dyn FnMut(&[f64], &[f64]) -> bool),
        &mut output,
        &mut StagedGmresWorkspace::default(),
        &mut counters,
        requested,
        &mut telemetry,
    )
    .unwrap();
    telemetry.acceptance_vector_copies =
        evaluations.iter().map(|e| u64::from(e.vector_copies)).sum();
    json!({
        "cell": cell,
        "id": id,
        "dimension": n,
        "config": {
            "restart": config.restart,
            "max_columns": config.max_columns,
            "rtol": hex(config.rtol),
            "atol": hex(config.atol),
            "stall_rule": config.stall_rule,
            "floor_at_confirmations": config.floor_at_confirmations,
            "stagnation_guard": config.stagnation_guard,
            "classify_guard_aborts": config.classify_guard_aborts,
            "classify_accepted_guard_aborts": config.classify_accepted_guard_aborts,
        },
        "report": report_value(&report),
        "telemetry": solver_value(&telemetry),
        "evaluations": evaluations.iter().map(evaluation_value).collect::<Vec<_>>(),
        "output": hexes(&output),
        "counters": serde_json::to_value(counters).unwrap(),
        "independent_shadow_matvecs": Value::Null,
    })
}

/// `max(atol, rtol ||rhs||)`: the requested target of the solver cells
/// other than S4.
fn l2_target(config: &StagedGmresConfig, rhs: &[f64]) -> f64 {
    config.atol.max(config.rtol * safe_l2(rhs))
}

/// The registered solver cells S1-S5.
fn solver_cells() -> Vec<Value> {
    let mut cells = Vec::new();
    // S1: NATIVE.json `stage_acceptance`, requested = recorded threshold.
    let systems = [
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
    ];
    for (id, a, b) in &systems {
        for mode in ["strict", "stall", "floor"] {
            let mut config = StagedGmresConfig::new(b.len(), 16, 1.0e-30, 0.0);
            config.stall_rule = mode == "stall";
            config.floor_at_confirmations = mode == "floor";
            let op = DenseOperator::new(a.clone()).unwrap();
            cells.push(solver_cell(
                "S1",
                &format!("{id}_{mode}"),
                &op,
                b,
                &config,
                None,
                l2_target(&config, b),
            ));
        }
    }
    // S2: the fallback seam (cyclic 4x4, restart = budget = 1, scaled rtol
    // 1e-14) with the production rule at two scales, then loose rules.
    let scaled_rhs = [1.5, 1.5, 0.0, 0.0];
    let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
    for (id, scale, rtol) in [
        ("overflowing_norms", 1.0e308, 1.0e-10),
        ("finite_norm_control", 1.0e100, 1.0e-10),
        ("loose_finite", 1.0, 1.0),
        ("loose_homogeneous", 1.0e308, 1.0),
    ] {
        let physical: Vec<f64> = scaled_rhs.iter().map(|b| b * scale).collect();
        let fallback = CellFallback {
            rule: ProductionFallbackRule::new(&physical, rtol, 1.0e-14),
            scale: Some(vec![scale; 4]),
        };
        let op = DenseOperator::new(cyclic_shift(4)).unwrap();
        cells.push(solver_cell(
            "S2",
            id,
            &op,
            &scaled_rhs,
            &config,
            Some(&fallback),
            l2_target(&config, &scaled_rhs),
        ));
    }
    // S3: the contraction-abort fixture, accepted by a loose production
    // rule (unscaled), classification off and on, with an independent
    // count of every operator application.
    let n = 50;
    let mut b = vec![0.0; n];
    b[0] = 1.0;
    for classify in [false, true] {
        let mut config = StagedGmresConfig::new(10, 100, 1.0e-10, 0.0);
        config.stagnation_guard = true;
        config.classify_guard_aborts = classify;
        config.classify_accepted_guard_aborts = classify;
        let op = Counting {
            inner: DenseOperator::new(cyclic_shift(n)).unwrap(),
            applications: AtomicU64::new(0),
        };
        let fallback = CellFallback {
            rule: ProductionFallbackRule::new(&b, 2.0, 0.0),
            scale: None,
        };
        let mut cell = solver_cell(
            "S3",
            if classify {
                "contraction_classified"
            } else {
                "contraction_unclassified"
            },
            &op,
            &b,
            &config,
            Some(&fallback),
            l2_target(&config, &b),
        );
        let counted = cell["report"]["matvecs"].as_u64().unwrap();
        cell["independent_shadow_matvecs"] =
            json!(op.applications.load(Ordering::SeqCst) - counted);
        cells.push(cell);
    }
    // S4: Converged against the raised threshold above the requested
    // target `atol`.
    let a = DenseMatrix::from_rows(&[&[1.0, 0.0], &[0.0, 2.0]]).unwrap();
    let config = StagedGmresConfig::new(2, 4, 0.5, 1.0e-30);
    let op = DenseOperator::new(a).unwrap();
    cells.push(solver_cell(
        "S4",
        "raised_target",
        &op,
        &[1.0, 1.0],
        &config,
        None,
        config.atol,
    ));
    // S5: failures.
    let b4 = [1.0, 0.5, -0.25, 2.0];
    let config = StagedGmresConfig::new(1, 4, 1.0e-14, 0.0);
    let op = TurnsInfinite {
        inner: DenseOperator::new(tridiagonal(4, 0.3, 3.0, -0.7)).unwrap(),
        finite: 1,
        applications: AtomicU64::new(0),
    };
    cells.push(solver_cell(
        "S5",
        "nonfinite",
        &op,
        &b4,
        &config,
        None,
        l2_target(&config, &b4),
    ));
    let b3 = [1.0, 2.0, 3.0];
    let config = StagedGmresConfig::new(3, 9, 1.0e-10, 0.0);
    let op = DenseOperator::new(DenseMatrix::zeros(3, 3)).unwrap();
    cells.push(solver_cell(
        "S5",
        "breakdown",
        &op,
        &b3,
        &config,
        None,
        l2_target(&config, &b3),
    ));
    let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
    let op = DenseOperator::new(cyclic_shift(4)).unwrap();
    cells.push(solver_cell(
        "S5",
        "budget_exhausted",
        &op,
        &scaled_rhs,
        &config,
        None,
        l2_target(&config, &scaled_rhs),
    ));
    cells
}

#[test]
fn solver_cells_have_the_registered_outcomes() {
    let cells = solver_cells();
    assert_eq!(cells.len(), 9 + 4 + 2 + 1 + 3);
    let get = |id: &str| cells.iter().find(|c| c["id"] == id).unwrap();
    for id in ["overflowing_norms", "finite_norm_control"] {
        assert_eq!(get(id)["report"]["outcome"], "Failed");
        assert_eq!(get(id)["report"]["failure"], "BudgetExhausted");
    }
    for (id, copies) in [("loose_finite", 1), ("loose_homogeneous", 2)] {
        let c = get(id);
        assert_eq!(c["report"]["outcome"], "FallbackAccepted");
        assert_eq!(c["telemetry"]["attainment"], "BudgetExceeded");
        assert_eq!(c["telemetry"]["acceptance_vector_copies"], copies);
    }
    assert_eq!(
        get("overflowing_norms")["evaluations"][0]["vector_copies"],
        2
    );
    for id in ["contraction_unclassified", "contraction_classified"] {
        let c = get(id);
        assert_eq!(c["report"]["outcome"], "FallbackAccepted");
        assert_eq!(
            c["telemetry"]["uncounted_shadow_matvecs"],
            c["independent_shadow_matvecs"]
        );
    }
    assert_eq!(
        get("contraction_unclassified")["independent_shadow_matvecs"],
        0
    );
    assert_eq!(get("raised_target")["telemetry"]["reason"], "Converged");
    assert_eq!(
        get("raised_target")["telemetry"]["attainment"],
        "BudgetExceeded"
    );
    assert_eq!(get("nonfinite")["report"]["failure"], "NonFinite");
    assert_eq!(get("breakdown")["report"]["failure"], "Breakdown");
    assert_eq!(
        get("budget_exhausted")["report"]["failure"],
        "BudgetExhausted"
    );
}

// ------------------------------------------------------- recorded run

fn git(args: &[&str]) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn tree_status() -> String {
    git(&["status", "--porcelain=v1", "--untracked-files=all"])
}

/// One registered driver run, telemetry on and off.
fn driver_runs(
    cell: &str,
    id: &str,
    arm: &str,
    case: &DriverCase<'_>,
    fresh_off: Option<&DriverCase<'_>>,
    budget: usize,
) -> Vec<Value> {
    let start = std::time::Instant::now();
    let off = fresh_off.unwrap_or(case).off();
    let on = case.on();
    println!("{cell} {id}: {:.1} s", start.elapsed().as_secs_f64());
    let header = |telemetry: bool| {
        json!({
            "cell": cell,
            "id": id,
            "arm": arm,
            "telemetry": telemetry,
            "dimension": case.y0.len(),
            "budget": budget,
            "options": case.options,
            "rtol": case.adaptive.rtol,
        })
    };
    let mut off_value = header(false);
    off_value["result"] = result_or_error(&off);
    off_value["records"] = Value::Null;
    let mut on_value = header(true);
    match &on {
        Ok(r) => {
            on_value["result"] = result_value(&r.run);
            on_value["records"] = records_value(&r.telemetry);
        }
        Err(e) => {
            on_value["result"] = json!({"ok": false, "error": e.to_string()});
            on_value["records"] = Value::Null;
        }
    }
    vec![on_value, off_value]
}

/// The RUNS document: the solver cells S1-S5, the driver fixtures D1 and
/// D2 and (with `real_cells`) the real-driver cells R, every driver run
/// with telemetry on and off.
fn runs_document(real_cells: bool) -> Value {
    let source_commit = git(&["rev-parse", "HEAD"]).trim().to_string();
    let status_before = tree_status();
    let mut runs = Vec::new();
    // D1.
    let (problem, y0, linear, adaptive, options) = d1();
    let case = d1_case(&problem, &y0, &linear, &adaptive, options);
    runs.extend(driver_runs(
        "D1",
        "as01-cyclic-4-overflow",
        "Coupled+fallback",
        &case,
        None,
        1,
    ));
    // D2 (each twin on its own problem: the epoch counts its reads).
    let (on_problem, off_problem) = (d2_problem(), d2_problem());
    runs.extend(driver_runs(
        "D2",
        "as04-d2-linear-2",
        "CoupledGuarded2",
        &d2_case(&on_problem),
        Some(&d2_case(&off_problem)),
        BUDGET,
    ));
    // R.
    if real_cells {
        let robertson = robertson_long();
        let stosc = stosc(1.0e4);
        let vdp = spd07_problem("van-der-pol-mu1000");
        let bruss = spd07_problem("brusselator-1d-50");
        let b3 = alg06_options(true, true, true);
        let b3b = alg06_options(true, true, false);
        let cells: [(&Problem, f64, &str, StageTargetOptions, usize); 7] = [
            (&robertson, 1.0e-5, "B3", b3, BIG_BUDGET),
            (&robertson, 1.0e-9, "B3", b3, BIG_BUDGET),
            (&stosc, 1.0e-4, "B3", b3, BIG_BUDGET),
            (&vdp, 1.0e-6, "B3", b3, BIG_BUDGET),
            (&bruss, 1.0e-6, "B3", b3, BIG_BUDGET),
            (&robertson, 1.0e-5, "B3b", b3b, BIG_BUDGET),
            (
                &bruss,
                1.0e-6,
                "ProjL2+fallback",
                proj_l2_fallback(),
                BUDGET,
            ),
        ];
        for (p, rtol, arm, options, budget) in cells {
            let case = problem_case(p, rtol, budget, options);
            runs.extend(driver_runs(
                "R",
                &format!("{}/{rtol:e}", p.id),
                arm,
                &case,
                None,
                budget,
            ));
        }
    }
    let solver = solver_cells();
    let status_after = tree_status();
    json!({
        "schema": "vigilode-as04-runs-v1",
        "node": "as04_stage_residual_telemetry_20261011",
        "export": "export_runs",
        "provenance": {
            "source_commit": source_commit,
            "tree_status_before": status_before,
            "tree_status_after": status_after,
            "rayon_num_threads": std::env::var("RAYON_NUM_THREADS").ok(),
            "openblas_num_threads": std::env::var("OPENBLAS_NUM_THREADS").ok(),
        },
        "stages_per_attempt": STAGES,
        "solver_cells": solver,
        "driver_runs": runs,
    })
}

/// Write `document` compactly to `path` (relative to the workspace root);
/// the output is immutable.
fn write_document(path: &str, document: &Value) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    assert!(
        !path.exists(),
        "immutable output exists: {}",
        path.display()
    );
    std::fs::write(&path, serde_json::to_string(document).unwrap() + "\n").unwrap();
    println!("wrote {}", path.display());
}

/// AS04 recorded run (all registered cells), written to `AS04_RUNS`.
#[test]
#[ignore = "recorded run of research/as04_stage_residual_telemetry_20261011; release build"]
fn export_runs() {
    let Ok(path) = std::env::var("AS04_RUNS") else {
        println!("AS04_RUNS is not set; export skipped");
        return;
    };
    write_document(&path, &runs_document(true));
}

/// A schema smoke export without the real-driver cells (S and D only), for
/// exercising the checker before the recorded run; never a recorded run
/// (the checker rejects it: the R runs are missing).
#[test]
#[ignore = "schema smoke export of research/as04_stage_residual_telemetry_20261011 (no R cells)"]
fn export_fixture_smoke() {
    let Ok(path) = std::env::var("AS04_SMOKE") else {
        println!("AS04_SMOKE is not set; export skipped");
        return;
    };
    write_document(&path, &runs_document(false));
}
