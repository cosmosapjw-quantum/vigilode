//! Diagnostic comparative benchmark for collective mode packet methods (LP09).
//!
//! Compares the packet trajectory method (LP01-LP04) against the direct Rosenbrock/stiff
//! ODE baseline (`rodas5p_fast`) on the same declared physical model.
//!
//! Scope:
//! - Equal nominal tolerances do not establish equal physical accuracy.
//! - Comparison against properly hoisted O(N) baseline (refusing synthetic unhoisted baselines).
//! - Deterministic work counter reporting (Jacobians, JVPs, LU factorizations, linear solves vs packet moments).
//! - Allocations and independent reference errors are unmeasured and reported as null.
//! - The packet radius is a reported estimator, not an independently verified reference.
//! - Diagnostic timing authority ("diagnostic_only", R2-STAT-01 / R4-STAT-DEV-01).

use std::time::Instant;

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, CollectiveModel, OdeProblem, OutputPolicy, OutputSchedule,
    PacketWorkCounters, Rodas5pFastOptions, TrajectoryConfig,
    integrate_rodas5p_fast_observed_with_options, run_packet_trajectory,
};
use serde::{Deserialize, Serialize};

pub const PACKET_BENCHMARK_SCHEMA: &str = "vigilode-packet-benchmark-report-v2";

/// Benchmark candidate or comparator arm identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BenchmarkArm {
    /// Multi-window adaptive packet trajectory (LP04).
    PacketMultiwindow,
    /// Direct ODE solver baseline using RODAS5P fast dense solver.
    DirectSolverRodas5p,
    /// Attribution check: unhoisted baseline (must be refused as unfair comparison).
    UnhoistedRhsBaseline,
}

/// Configuration for a fair A/B benchmark run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PacketBenchmarkConfig {
    pub t_start: f64,
    pub t_end: f64,
    pub initial_h: f64,
    pub min_h: f64,
    pub max_h: f64,
    pub tolerance: f64,
    pub order: usize,
    pub degree: usize,
    pub sweeps: usize,
    pub output_policy: OutputPolicy,
    pub allow_unhoisted: bool,
}

impl Default for PacketBenchmarkConfig {
    fn default() -> Self {
        Self {
            t_start: 0.0,
            t_end: 1.0,
            initial_h: 0.1,
            min_h: 1e-6,
            max_h: 0.5,
            tolerance: 1e-6,
            order: 4,
            degree: 8,
            sweeps: 2,
            output_policy: OutputPolicy::MeanOnly,
            allow_unhoisted: false,
        }
    }
}

/// Per-arm execution summary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArmRunSummary {
    pub arm: BenchmarkArm,
    pub steps_taken: usize,
    pub final_t: f64,
    pub final_mean: (f64, f64),
    /// Requires an independent reference; this diagnostic runner supplies none.
    pub error_vs_reference: Option<f64>,
    /// Radius returned by the packet implementation, without independent validation here.
    pub reported_mean_radius: Option<f64>,
    pub work_counters: WorkCounters,
    pub work_counters_authority: &'static str,
    pub packet_counters: PacketWorkCounters,
    pub elapsed_seconds_diagnostic: f64,
    pub timing_authority: &'static str,
    /// None means not measured, rather than a zero-allocation result.
    pub hot_loop_allocations: Option<usize>,
}

/// Comprehensive benchmark comparison report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PacketBenchmarkReport {
    pub schema: &'static str,
    pub node_id: &'static str,
    pub mode_count: usize,
    pub tolerance: f64,
    pub packet_candidate: ArmRunSummary,
    pub direct_baseline: ArmRunSummary,
    pub speedup_ratio_diagnostic: f64,
    /// Distance between the two computed means; neither is an independent reference.
    pub mean_discrepancy_diagnostic: f64,
    pub physical_accuracy_status: &'static str,
    pub work_reduction: WorkReductionSummary,
    pub fairness_verified: bool,
    pub verdict: &'static str,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkReductionSummary {
    pub jacobian_evals_eliminated: u64,
    pub lu_decompositions_eliminated: u64,
    pub linear_solves_eliminated: u64,
    pub online_solvers_eliminated: bool,
}

/// Creates a standard 2N-dimensional OdeProblem representing the collective model
/// with properly hoisted O(N) mean evaluation.
pub fn build_hoisted_collective_ode_problem(model: &CollectiveModel) -> CoreResult<OdeProblem> {
    model.to_ode_problem()
}

/// Runs a diagnostic comparison. Physical accuracy and complete cost/output fairness
/// require a separate independent reference and a matched execution contract.
pub fn run_packet_benchmark(
    model: &CollectiveModel,
    config: &PacketBenchmarkConfig,
) -> CoreResult<PacketBenchmarkReport> {
    if config.t_end <= config.t_start {
        return Err(CoreError::InvalidInput("t_end must be > t_start".into()));
    }
    if config.tolerance <= 0.0 {
        return Err(CoreError::InvalidInput("tolerance must be positive".into()));
    }
    if config.allow_unhoisted {
        return Err(CoreError::InvalidInput(
            "Unhoisted baseline is unfair and strictly refused by LP09 gate".into(),
        ));
    }

    // 1. Run Packet Candidate Arm
    let traj_config = TrajectoryConfig {
        t_start: config.t_start,
        t_end: config.t_end,
        initial_h: config.initial_h,
        min_h: config.min_h,
        max_h: config.max_h,
        order: config.order,
        degree: config.degree,
        sweeps: config.sweeps,
        mean_tolerance: config.tolerance,
        mode_tolerance: config.tolerance,
        output_policy: config.output_policy,
        max_steps: 1000,
    };

    let mut packet_work = PacketWorkCounters::default();
    let packet_start = Instant::now();
    let packet_res = run_packet_trajectory(model, None, &traj_config, &mut packet_work)?;
    let packet_elapsed = packet_start.elapsed().as_secs_f64();

    let packet_summary = ArmRunSummary {
        arm: BenchmarkArm::PacketMultiwindow,
        steps_taken: packet_res.accepted_steps,
        final_t: packet_res.final_t,
        final_mean: packet_res.final_mean,
        error_vs_reference: None,
        reported_mean_radius: Some(packet_res.final_mean_radius),
        work_counters: WorkCounters::default(),
        work_counters_authority: "structural_zero_not_instrumented",
        packet_counters: packet_work,
        elapsed_seconds_diagnostic: packet_elapsed,
        timing_authority: "diagnostic_only",
        hot_loop_allocations: None,
    };

    // 2. Run Direct Solver Baseline (RODAS5P Fast)
    let ode_problem = build_hoisted_collective_ode_problem(model)?;
    let n_modes = model.num_modes();
    let mut y0 = vec![0.0; model.dimension()];
    model.initial_state(&mut y0)?;

    let adaptive_cfg = AdaptiveStepConfig {
        initial_step: config.initial_h,
        min_step: config.min_h,
        max_step: config.max_h,
        rtol: config.tolerance,
        atol: config.tolerance,
        ..Default::default()
    };

    let output_sched = OutputSchedule::new(vec![config.t_start, config.t_end])?;
    let direct_start = Instant::now();
    let direct_res = integrate_rodas5p_fast_observed_with_options(
        &ode_problem,
        (config.t_start, config.t_end),
        &y0,
        &adaptive_cfg,
        &output_sched,
        Rodas5pFastOptions::default(),
    )?;
    let direct_elapsed = direct_start.elapsed().as_secs_f64();

    // Compute direct final mean
    let final_y =
        direct_res.observed.y.last().ok_or_else(|| {
            CoreError::InvalidInput("missing final state from direct solver".into())
        })?;
    let (direct_mx, direct_my) = model.mean(final_y);

    let mean_diff = ((packet_res.final_mean.0 - direct_mx).powi(2)
        + (packet_res.final_mean.1 - direct_my).powi(2))
    .sqrt();

    let direct_work = direct_res.observed.counters;
    let final_time =
        *direct_res.observed.t.last().ok_or_else(|| {
            CoreError::InvalidInput("missing final time from direct solver".into())
        })?;

    let direct_summary = ArmRunSummary {
        arm: BenchmarkArm::DirectSolverRodas5p,
        steps_taken: direct_work.accepted_steps as usize,
        final_t: final_time,
        final_mean: (direct_mx, direct_my),
        error_vs_reference: None,
        reported_mean_radius: None,
        work_counters: direct_work,
        work_counters_authority: "instrumented",
        packet_counters: PacketWorkCounters::default(),
        elapsed_seconds_diagnostic: direct_elapsed,
        timing_authority: "diagnostic_only",
        hot_loop_allocations: None,
    };

    let speedup = if packet_elapsed > 0.0 {
        direct_elapsed / packet_elapsed
    } else {
        1.0
    };

    let work_red = WorkReductionSummary {
        jacobian_evals_eliminated: direct_work.jacobian_builds,
        lu_decompositions_eliminated: direct_work.direct_factorizations,
        linear_solves_eliminated: direct_work.linear_solves,
        online_solvers_eliminated: true,
    };

    Ok(PacketBenchmarkReport {
        schema: PACKET_BENCHMARK_SCHEMA,
        node_id: "LP09",
        mode_count: n_modes,
        tolerance: config.tolerance,
        packet_candidate: packet_summary,
        direct_baseline: direct_summary,
        speedup_ratio_diagnostic: speedup,
        mean_discrepancy_diagnostic: mean_diff,
        physical_accuracy_status: "UNVERIFIED_NO_INDEPENDENT_REFERENCE",
        work_reduction: work_red,
        fairness_verified: false,
        verdict: "DIAGNOSTIC_ONLY",
    })
}
