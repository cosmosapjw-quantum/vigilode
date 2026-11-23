//! The router: runs the entry point that the preflight analysis names and, after an
//! explicit trial that stops for a numerical reason, RODAS5P from the last accepted state.
//!
//! It has three entry points and no driver of its own: `Dopri5::solve` and
//! `Rodas5p::solve` through the method registry, and `SwitchingSolver::solve`. What a
//! solution reports is read from the run record of the driver that ran.

use std::time::Instant;

use rodas5p_core::WorkCounters;

use super::analyzer::{PreflightAnalysis, PreflightAnalyzer, SelectedTier, method_id};
use super::fallback::FallbackEngine;
use super::legs::{UnifiedLeg, UnifiedStop, registry_leg, switching_leg};
use super::options::VigilodeOptions;
use super::solution::{RouteTransitionRecord, VigilodeSolution};
use super::workspace::VigilodeWorkspace;
use crate::explicit::ExplicitSolution;
use crate::options::Options;
use crate::problem::Problem;
use crate::solution::{RouteRecord, Solution, Status};
use crate::solve::Rodas5p;
use crate::switching::SwitchingSolution;
use crate::{MethodResult, ProblemRegime, SolveError};

/// The router of [`super::VigilODE`].
pub struct VigilodeRouter;

impl VigilodeRouter {
    /// Solve with a caller-provided workspace.
    ///
    /// `Err` means that nothing was integrated: malformed input, or a request that the
    /// first entry point refused. Every run that started is `Ok`; if it stopped early it
    /// carries its accepted prefix, the work of every leg and of the preflight, and the
    /// typed stop ([`VigilodeSolution::stop`]).
    ///
    /// The legs share one attempt budget, `Options::max_steps`: an explicit trial gets
    /// [`FallbackEngine::explicit_trial_attempts`] of it and the RODAS5P leg after it
    /// what is left. A leg that a fatal callback ended is not followed by another one.
    pub fn solve_with_workspace(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        opts: &VigilodeOptions,
        ws: &mut VigilodeWorkspace,
    ) -> Result<VigilodeSolution, SolveError> {
        let started = Instant::now();
        let analysis = PreflightAnalyzer::analyze(problem, span, y0, opts)?;
        ws.reset(y0);
        ws.checkpoint(span.0, y0);
        // the callbacks of the analysis are work of this solve
        ws.work_counters.rhs_calls += analysis.rhs_calls;
        ws.work_counters.rhs_evaluations += analysis.rhs_calls;

        let run = Run {
            problem,
            span,
            y0,
            opts,
            ws,
            started,
            analysis,
            legs: Vec::new(),
            history: Vec::new(),
            refusal: None,
            implicit: None,
            explicit: None,
            switching: None,
        };
        if run.analysis.callback_failure.is_some() {
            return Ok(run.finish());
        }
        match run.analysis.suggested_tier {
            SelectedTier::Tier1Explicit => run.explicit_then_fallback(),
            SelectedTier::TierProtectedReference => {
                let reason = run.analysis.basis.describe();
                let budget = run.analysis.first_leg_attempts;
                run.rodas5p(span.0, y0.to_vec(), budget, reason)
            }
            SelectedTier::Tier5AdaptiveSwitching => run.switching(),
        }
    }
}

/// The state of one unified solve.
struct Run<'a> {
    problem: &'a Problem,
    span: (f64, f64),
    y0: &'a [f64],
    opts: &'a VigilodeOptions,
    ws: &'a mut VigilodeWorkspace,
    started: Instant,
    analysis: PreflightAnalysis,
    legs: Vec<UnifiedLeg>,
    history: Vec<RouteTransitionRecord>,
    /// The entry point of a further leg that refused its input, with its message.
    refusal: Option<(SelectedTier, String)>,
    implicit: Option<Solution>,
    explicit: Option<ExplicitSolution>,
    switching: Option<SwitchingSolution>,
}

impl Run<'_> {
    fn leg_options(&self, attempts: usize) -> Options {
        let mut options = self.opts.options.clone();
        options.max_steps = attempts;
        options
    }

    fn used_attempts(&self) -> usize {
        self.legs.iter().map(|leg| leg.attempts).sum()
    }

    /// Record an executed leg: its history entry, its work and its end point.
    fn record(&mut self, leg: UnifiedLeg, end_state: &[f64], reason: String) {
        self.history.push(RouteTransitionRecord {
            from_tier: self
                .legs
                .last()
                .map_or_else(|| "preflight".to_owned(), |last| last.driver_id.clone()),
            to_tier: leg.driver_id.clone(),
            time: leg.t_start,
            reason,
            step_index: self.legs.iter().map(|done| done.accepted_steps).sum(),
        });
        self.ws
            .work_counters
            .accumulate(WorkCounters::from(&leg.counters));
        self.ws.checkpoint(leg.t_end, end_state);
        self.legs.push(leg);
    }

    /// The explicit leg and, when the analysis planned it and the stop allows it, RODAS5P
    /// from the last accepted state with the attempts that are left.
    fn explicit_then_fallback(mut self) -> Result<VigilodeSolution, SolveError> {
        let total = self.opts.options.max_steps;
        let options = self.leg_options(self.analysis.first_leg_attempts);
        let executed = registry_leg(
            self.problem,
            SelectedTier::Tier1Explicit,
            method_id(ProblemRegime::Nonstiff),
            self.span,
            self.y0,
            &options,
        );
        let (leg, native) = match executed {
            Ok((leg, MethodResult::Explicit(native))) => (leg, native),
            Ok(_) => {
                return Err(SolveError::Driver(
                    "the registry returned no DOPRI5 result for its explicit method".into(),
                ));
            }
            // nothing was integrated
            Err(error) if self.analysis.fallback_to_rodas5p => {
                self.ws.work_counters.fallback_steps += 1;
                let reason = format!("Dopri5::solve refused before integrating: {error}");
                let (start, state) = (self.span.0, self.y0.to_vec());
                return self.rodas5p(start, state, total, reason);
            }
            Err(error) => return Err(error),
        };
        let checkpoint_time = native.final_time;
        let checkpoint = if native.final_state.is_empty() {
            self.y0.to_vec()
        } else {
            native.final_state.clone()
        };
        let follow = !leg.success
            && self.analysis.fallback_to_rodas5p
            && FallbackEngine::follows_explicit_stop(
                native.stop_reason,
                native.callback_failure.is_some(),
            );
        let stop_text = native.message.clone();
        let reason = self.analysis.basis.describe();
        self.record(leg, &checkpoint, reason);
        self.explicit = Some(native);
        let remaining = FallbackEngine::remaining_attempts(total, self.used_attempts());
        if !follow || remaining == 0 {
            return Ok(self.finish());
        }
        self.ws.work_counters.fallback_steps += 1;
        let reason = format!(
            "DOPRI5 stopped at t = {checkpoint_time:e} ({stop_text}); RODAS5P continues from \
             that state with the {remaining} attempts left of {total}"
        );
        self.rodas5p(checkpoint_time, checkpoint, remaining, reason)
    }

    /// A RODAS5P leg from `(t, y)` with `attempts` attempts.
    fn rodas5p(
        mut self,
        t: f64,
        y: Vec<f64>,
        attempts: usize,
        reason: String,
    ) -> Result<VigilodeSolution, SolveError> {
        let options = self.leg_options(attempts);
        let executed = registry_leg(
            self.problem,
            SelectedTier::TierProtectedReference,
            method_id(ProblemRegime::Stiff),
            (t, self.span.1),
            &y,
            &options,
        );
        match executed {
            Ok((leg, MethodResult::Rodas5p(native))) => {
                let end_state = native.final_state().map_or(y, <[f64]>::to_vec);
                self.record(leg, &end_state, reason);
                self.implicit = Some(native);
                Ok(self.finish())
            }
            Ok(_) => Err(SolveError::Driver(
                "the registry returned no RODAS5P result for its stiff method".into(),
            )),
            // nothing was integrated at all: the refusal is the answer
            Err(error) if self.legs.is_empty() => Err(error),
            // a prefix exists: keep it, with its work and the refusal
            Err(error) => {
                self.refusal = Some((SelectedTier::TierProtectedReference, error.to_string()));
                Ok(self.finish())
            }
        }
    }

    /// The switching adapter; if it refuses the request (before any callback), RODAS5P.
    fn switching(mut self) -> Result<VigilodeSolution, SolveError> {
        match switching_leg(self.problem, self.span, self.y0, &self.opts.options) {
            Ok((leg, native)) => {
                let end_state = native.final_state.clone();
                let reason = self.analysis.basis.describe();
                self.record(leg, &end_state, reason);
                self.switching = Some(native);
                Ok(self.finish())
            }
            Err(error) => {
                self.ws.work_counters.fallback_steps += 1;
                let reason = format!(
                    "SwitchingSolver::solve refused the request before any callback: {error}"
                );
                let total = self.opts.options.max_steps;
                let (start, state) = (self.span.0, self.y0.to_vec());
                self.rodas5p(start, state, total, reason)
            }
        }
    }

    /// Build the solution from the executed legs.
    fn finish(self) -> VigilodeSolution {
        let dimension = self.y0.len();
        let final_time = self.ws.checkpoint_time;
        let final_state = self.ws.y_checkpoint[..dimension].to_vec();
        let total = self.opts.options.max_steps;
        let attempts = self.used_attempts();
        let accepted_steps: usize = self.legs.iter().map(|leg| leg.accepted_steps).sum();
        let rejected_steps: usize = self.legs.iter().map(|leg| leg.rejected_steps).sum();

        let last = self.legs.last();
        let success = self.refusal.is_none() && last.is_some_and(|leg| leg.success);
        let last_stop = last.and_then(|leg| leg.stop.clone());
        let stop = if success {
            None
        } else if let Some(callback) = self.analysis.callback_failure.clone() {
            Some(UnifiedStop::Preflight { callback })
        } else if let Some((tier, error)) = self.refusal.clone() {
            Some(UnifiedStop::Refused {
                tier,
                error,
                prior: last_stop.map(Box::new),
            })
        } else {
            Some(last_stop.unwrap_or_else(|| UnifiedStop::Other {
                message: "the driver stopped before the end time without a reason".into(),
            }))
        };
        let status = match &stop {
            None => Status::Success,
            Some(stop) => {
                // a budget stop states the budget that was used up: the total of the solve
                // when every attempt is spent, otherwise that of the leg that stopped
                let budget = if attempts >= total {
                    total
                } else {
                    last.map_or(total, |leg| leg.attempt_budget)
                };
                Status::Failed(stop.facade_reason(budget))
            }
        };

        let mut stats = match &self.implicit {
            Some(native) => native.stats.clone(),
            None => {
                let mut stats =
                    Rodas5p::empty_stats(self.problem, &self.opts.options, self.span.0, self.y0);
                let (entry_point, driver_id, message) = match (&self.explicit, &self.switching) {
                    (Some(native), _) => (
                        "integrate_dopri5_observed",
                        native.stats.method,
                        native.message.clone(),
                    ),
                    (None, Some(native)) => (
                        "SwitchingSolver::solve",
                        native.schema,
                        native.message.clone(),
                    ),
                    (None, None) => (
                        "none",
                        "none",
                        "stopped in the preflight analysis; no driver ran".to_owned(),
                    ),
                };
                stats.route = RouteRecord {
                    entry_point,
                    driver_id,
                    routing: None,
                };
                stats.driver_message = message;
                if let Some(native) = &self.explicit {
                    stats.output_clipped_steps = native.stats.output_clipped_steps;
                }
                stats
            }
        };
        stats.options = self.opts.options.clone();
        stats.t_span = self.span;
        stats.y0 = self.y0.to_vec();
        stats.counters = (&self.ws.work_counters).into();
        stats.attempts = attempts;
        stats.accepted_steps = accepted_steps;
        stats.rejected_steps = rejected_steps;

        let invariant_drift = self
            .problem
            .invariants()
            .iter()
            .map(|(name, invariant)| {
                (
                    (*name).to_string(),
                    (invariant(&final_state) - invariant(self.y0)).abs(),
                )
            })
            .collect();

        VigilodeSolution {
            success,
            status,
            final_time,
            final_state,
            stats,
            route_history: self.history,
            invariant_drift,
            timing_authority: "diagnostic_only",
            elapsed_nanos: self.started.elapsed().as_nanos(),
            stop,
            legs: self.legs,
            analysis: self.analysis,
            inner_solution: self.implicit,
            explicit_solution: self.explicit,
            switching_solution: self.switching,
        }
    }
}
