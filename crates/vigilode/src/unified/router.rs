//! Autonomous multi-tier polyalgorithm router.
//!
//! Evaluates preflight system diagnosis and coordinates dispatch to the optimal
//! numerical engine (DOPRI5 explicit, matrix-free Krylov, L^8=0 staged sweep,
//! or adaptive reentrant switching) with fail-closed protected fallback.

use std::time::Instant;

use crate::error::SolveError;
use crate::explicit::Dopri5;
use crate::problem::Problem;
use crate::solution::{FailureReason, Status};
use crate::solve::Rodas5p;
use crate::switching::{DerivativeFallback, SwitchingConfig, SwitchingSolver};

use super::analyzer::{PreflightAnalysis, PreflightAnalyzer, SelectedTier};
use super::fallback::FallbackEngine;
use super::options::VigilodeOptions;
use super::solution::{RouteTransitionRecord, VigilodeSolution};
use super::workspace::VigilodeWorkspace;

/// Autonomous router and dispatcher for `VigilODE`.
pub struct VigilodeRouter;

impl VigilodeRouter {
    /// Execute the unified integration with caller-provided pre-allocated workspace.
    pub fn solve_with_workspace(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        opts: &VigilodeOptions,
        ws: &mut VigilodeWorkspace,
    ) -> Result<VigilodeSolution, SolveError> {
        let start_instant = Instant::now();
        ws.reset(y0);
        ws.checkpoint(span.0, y0);

        let mut route_history = Vec::new();

        // 1. Perform structural and numerical preflight analysis
        let analysis = PreflightAnalyzer::analyze(problem, span, y0, opts)?;

        // 2. Dispatch to the selected numerical tier
        match analysis.suggested_tier {
            SelectedTier::Tier1Explicit => Self::dispatch_explicit_tier(
                problem,
                span,
                y0,
                opts,
                ws,
                &mut route_history,
                start_instant,
            ),
            SelectedTier::Tier5AdaptiveSwitching => Self::dispatch_adaptive_switching(
                problem,
                span,
                y0,
                opts,
                ws,
                &mut route_history,
                start_instant,
            ),
            SelectedTier::Tier2MatrixFreeKrylov
            | SelectedTier::Tier4StagedRodas5p
            | SelectedTier::TierProtectedReference
            | SelectedTier::Tier3Exponential => Self::dispatch_rodas5p_tier(
                problem,
                span,
                y0,
                opts,
                ws,
                &analysis,
                &mut route_history,
                start_instant,
            ),
        }
    }

    fn dispatch_explicit_tier(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        opts: &VigilodeOptions,
        ws: &mut VigilodeWorkspace,
        route_history: &mut Vec<RouteTransitionRecord>,
        start_instant: Instant,
    ) -> Result<VigilodeSolution, SolveError> {
        route_history.push(RouteTransitionRecord {
            from_tier: "Preflight".to_string(),
            to_tier: "Tier1ExplicitDopri5".to_string(),
            time: span.0,
            reason: "Non-stiff contractive system selected for high-speed explicit integration"
                .to_string(),
            step_index: 0,
        });

        match Dopri5::solve(problem, span, y0, &opts.options) {
            Ok(exp_sol) if exp_sol.success => {
                ws.work_counters.accumulate(exp_sol.stats.counters);
                let final_time = exp_sol.final_time;
                let final_state = exp_sol.final_state.clone();

                let mut stats = Rodas5p::empty_stats(problem, &opts.options, span.0, y0);
                stats.counters = (&ws.work_counters).into();
                stats.accepted_steps = exp_sol.stats.accepted_steps;
                stats.rejected_steps = exp_sol.stats.rejected_steps;

                let elapsed_nanos = start_instant.elapsed().as_nanos();
                let invariant_drift = Self::calculate_invariant_drift(problem, y0, &final_state);

                Ok(VigilodeSolution {
                    success: true,
                    status: Status::Success,
                    final_time,
                    final_state,
                    stats,
                    route_history: route_history.clone(),
                    invariant_drift,
                    timing_authority: "diagnostic_only",
                    elapsed_nanos,
                    inner_solution: None,
                })
            }
            Ok(exp_sol) => {
                // Stopped before final time; perform fail-closed fallback from last accepted state
                ws.work_counters.accumulate(exp_sol.stats.counters);
                let checkpoint_t = exp_sol.final_time;
                let checkpoint_y = if exp_sol.final_state.is_empty() {
                    y0
                } else {
                    &exp_sol.final_state
                };
                ws.checkpoint(checkpoint_t, checkpoint_y);

                FallbackEngine::execute_fallback(
                    problem,
                    span,
                    checkpoint_y,
                    checkpoint_t,
                    opts,
                    ws,
                    "DOPRI5 explicit lane rejected steps / stiffness detected",
                    route_history,
                    start_instant,
                )
            }
            Err(e) => {
                // If explicit solve was refused or failed upfront, fail-closed fallback immediately
                FallbackEngine::execute_fallback(
                    problem,
                    span,
                    y0,
                    span.0,
                    opts,
                    ws,
                    &format!("DOPRI5 initialization refused: {e}"),
                    route_history,
                    start_instant,
                )
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_rodas5p_tier(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        opts: &VigilodeOptions,
        ws: &mut VigilodeWorkspace,
        analysis: &PreflightAnalysis,
        route_history: &mut Vec<RouteTransitionRecord>,
        start_instant: Instant,
    ) -> Result<VigilodeSolution, SolveError> {
        let tier_name = match analysis.suggested_tier {
            SelectedTier::Tier2MatrixFreeKrylov => "Tier2MatrixFreeKrylov",
            SelectedTier::Tier4StagedRodas5p => "Tier4StagedRodas5p",
            SelectedTier::Tier3Exponential => "Tier3Exponential",
            _ => "TierProtectedReference",
        };

        route_history.push(RouteTransitionRecord {
            from_tier: "Preflight".to_string(),
            to_tier: tier_name.to_string(),
            time: span.0,
            reason: format!(
                "Stiff / structured problem routed to {tier_name} (jvp={}, band={}, mass={})",
                analysis.has_jvp, analysis.has_band, analysis.has_mass
            ),
            step_index: 0,
        });

        match Rodas5p::solve(problem, span, y0, &opts.options) {
            Ok(sol) => {
                ws.work_counters.accumulate(sol.stats.counters.into());
                let final_time = sol.t.last().copied().unwrap_or(span.1);
                let final_state = sol.final_state().map(|s| s.to_vec()).unwrap_or_default();
                let status = sol.status.clone();
                let success = status == Status::Success;

                let mut stats = sol.stats.clone();
                stats.counters = (&ws.work_counters).into();

                let elapsed_nanos = start_instant.elapsed().as_nanos();
                let invariant_drift = Self::calculate_invariant_drift(problem, y0, &final_state);

                Ok(VigilodeSolution {
                    success,
                    status,
                    final_time,
                    final_state,
                    stats,
                    route_history: route_history.clone(),
                    invariant_drift,
                    timing_authority: "diagnostic_only",
                    elapsed_nanos,
                    inner_solution: Some(sol),
                })
            }
            Err(e) => {
                // If Krylov route fails or is unsupported, and we have an alternate fallback, execute fallback
                if analysis.has_jvp && opts.strategy != super::options::SolverStrategy::RobustStiff
                {
                    // Fall back to protected direct solve if available
                    FallbackEngine::execute_fallback(
                        problem,
                        span,
                        y0,
                        span.0,
                        opts,
                        ws,
                        &format!("Matrix-free Krylov attempt returned: {e}"),
                        route_history,
                        start_instant,
                    )
                } else {
                    Err(e)
                }
            }
        }
    }

    fn dispatch_adaptive_switching(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        opts: &VigilodeOptions,
        ws: &mut VigilodeWorkspace,
        route_history: &mut Vec<RouteTransitionRecord>,
        start_instant: Instant,
    ) -> Result<VigilodeSolution, SolveError> {
        route_history.push(RouteTransitionRecord {
            from_tier: "Preflight".to_string(),
            to_tier: "Tier5AdaptiveSwitching".to_string(),
            time: span.0,
            reason: "Adaptive reentrant polyalgorithm switching selected".to_string(),
            step_index: 0,
        });

        let config = SwitchingConfig {
            fallback: DerivativeFallback::AllowFiniteDifference,
            ..SwitchingConfig::default()
        };

        match SwitchingSolver::solve(problem, span, y0, &opts.options, &config) {
            Ok(switch_sol) => {
                ws.work_counters.accepted_steps += switch_sol.stats.accepted_steps as u64;
                ws.work_counters.rejected_steps += switch_sol.stats.rejected_steps as u64;
                ws.work_counters.rhs_calls += switch_sol.stats.rhs_calls;
                ws.work_counters.jacobian_builds += switch_sol.stats.jacobian_builds;
                ws.work_counters.jvp_vectors += switch_sol.stats.jvp_vectors;
                ws.work_counters.ft_calls += switch_sol.stats.ft_calls;
                ws.work_counters.linear_solves += switch_sol.stats.linear_solves;
                let final_time = switch_sol.final_time;
                let final_state = switch_sol.final_state.clone();

                let mut stats = Rodas5p::empty_stats(problem, &opts.options, span.0, y0);
                stats.counters = (&ws.work_counters).into();

                let elapsed_nanos = start_instant.elapsed().as_nanos();
                let invariant_drift = Self::calculate_invariant_drift(problem, y0, &final_state);

                Ok(VigilodeSolution {
                    success: switch_sol.success,
                    status: if switch_sol.success {
                        Status::Success
                    } else {
                        Status::Failed(FailureReason::MaxStepsReached {
                            max_steps: opts.options.max_steps,
                        })
                    },
                    final_time,
                    final_state,
                    stats,
                    route_history: route_history.clone(),
                    invariant_drift,
                    timing_authority: "diagnostic_only",
                    elapsed_nanos,
                    inner_solution: None,
                })
            }
            Err(e) => FallbackEngine::execute_fallback(
                problem,
                span,
                y0,
                span.0,
                opts,
                ws,
                &format!("Adaptive switching failed: {e}"),
                route_history,
                start_instant,
            ),
        }
    }

    fn calculate_invariant_drift(
        problem: &Problem,
        y0: &[f64],
        final_state: &[f64],
    ) -> Vec<(String, f64)> {
        let mut invariant_drift = Vec::new();
        for (name, inv_fn) in problem.invariants() {
            let initial_val = inv_fn(y0);
            let final_val = inv_fn(final_state);
            let drift = (final_val - initial_val).abs();
            invariant_drift.push((name.to_string(), drift));
        }
        invariant_drift
    }
}
