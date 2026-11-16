//! Fail-closed protected fallback engine.
//!
//! Provides transactional rollback to the last accepted checkpoint and seamlessly
//! delegates execution to the protected L-stable RODAS5P reference driver,
//! while strictly conserving all accumulated `WorkCounters`.

use super::options::VigilodeOptions;
use super::solution::{RouteTransitionRecord, VigilodeSolution};
use super::workspace::VigilodeWorkspace;
use crate::error::SolveError;
use crate::problem::Problem;
use crate::solution::Status;
use crate::solve::Rodas5p;

/// Fail-closed fallback coordinator.
pub struct FallbackEngine;

impl FallbackEngine {
    /// Execute an atomic fallback from an aggressive fast lane to the protected RODAS5P driver.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_fallback(
        problem: &Problem,
        span: (f64, f64),
        y_checkpoint: &[f64],
        t_checkpoint: f64,
        opts: &VigilodeOptions,
        ws: &mut VigilodeWorkspace,
        reason: &str,
        route_history: &mut Vec<RouteTransitionRecord>,
        start_instant: std::time::Instant,
    ) -> Result<VigilodeSolution, SolveError> {
        let step_idx = ws.work_counters.accepted_steps as usize;
        route_history.push(RouteTransitionRecord {
            from_tier: "FastLane".to_string(),
            to_tier: "ProtectedRODAS5P".to_string(),
            time: t_checkpoint,
            reason: reason.to_string(),
            step_index: step_idx,
        });

        ws.work_counters.fallback_steps += 1;

        let remaining_span = (t_checkpoint, span.1);
        let protected_opts = opts.options.clone();

        let sol = Rodas5p::solve(problem, remaining_span, y_checkpoint, &protected_opts)?;

        // Conserve and accumulate all work counters
        ws.work_counters.accumulate(sol.stats.counters.into());

        let final_time = sol.t.last().copied().unwrap_or(span.1);
        let final_state = sol.final_state().map(|s| s.to_vec()).unwrap_or_default();
        let status = sol.status.clone();
        let success = status == Status::Success;

        let mut final_stats = sol.stats.clone();
        final_stats.counters = (&ws.work_counters).into();

        let elapsed_nanos = start_instant.elapsed().as_nanos();

        // Calculate invariant drift if invariants are registered
        let mut invariant_drift = Vec::new();
        for (name, inv_fn) in problem.invariants() {
            let initial_val = inv_fn(y_checkpoint);
            let final_val = inv_fn(&final_state);
            let drift = (final_val - initial_val).abs();
            invariant_drift.push((name.to_string(), drift));
        }

        Ok(VigilodeSolution {
            success,
            status,
            final_time,
            final_state,
            stats: final_stats,
            route_history: route_history.clone(),
            invariant_drift,
            timing_authority: "diagnostic_only",
            elapsed_nanos,
            inner_solution: Some(sol),
        })
    }
}
