//! The router: runs the entry point that the preflight analysis names; for a request that
//! declares nothing, the break-even route (DOPRI5 with forward break-even pilots, then
//! RODAS5P after an accepted pilot or a numerical DOPRI5 stop).
//!
//! It has three entry points and no driver of its own: `Dopri5::solve` (or its persistent
//! session, on the break-even route) and `Rodas5p::solve` through the method registry, and
//! `SwitchingSolver::solve`. What a solution reports is read from the run record of the
//! driver that ran.

use std::time::Instant;

use rodas5p_core::WorkCounters;
use rodas5p_integrators::Dopri5StopReason;

use super::analyzer::{PreflightAnalysis, PreflightAnalyzer, RouteBasis, SelectedTier, method_id};
use super::breakeven::{self, BreakEvenRun, Ending, PilotRecord};
use super::fallback::FallbackEngine;
use super::legs::{
    UnifiedLeg, UnifiedStop, explicit_leg, registry_leg, rodas5p_leg_after_pilot, switching_leg,
};
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
    /// carries its accepted prefix, the work of every leg and pilot, and the typed stop
    /// ([`VigilodeSolution::stop`]).
    ///
    /// The legs and the pilots share one attempt budget, `Options::max_steps`: a RODAS5P
    /// leg after DOPRI5 gets what is left of it. A leg or a pilot that a fatal callback
    /// ended is not followed by another one.
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
            pilots: Vec::new(),
            spare_attempts: 0,
            spare_accepted: 0,
            spare_rejected: 0,
            pilot_stop: None,
            refusal: None,
            implicit: None,
            explicit: None,
            switching: None,
        };
        match run.analysis.suggested_tier {
            SelectedTier::Tier1Explicit => match run.analysis.basis {
                RouteBasis::BreakEven { .. } => run.break_even(),
                _ => run.explicit(),
            },
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
    /// Every pilot of the break-even route.
    pilots: Vec<PilotRecord>,
    /// Attempts, accepted steps and rejected attempts of the pilots that are not the first
    /// step of a leg (lost, refused and fatal pilots); their work is in the counters.
    spare_attempts: usize,
    spare_accepted: usize,
    spare_rejected: usize,
    /// The stop of a pilot that a fatal callback ended (owner decision D1).
    pilot_stop: Option<UnifiedStop>,
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

    /// Attempts of every leg and of every pilot that is not part of a leg.
    fn used_attempts(&self) -> usize {
        self.legs.iter().map(|leg| leg.attempts).sum::<usize>() + self.spare_attempts
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

    /// The declared-nonstiff route: DOPRI5 with the whole attempt budget and no second
    /// method.
    fn explicit(mut self) -> Result<VigilodeSolution, SolveError> {
        let options = self.leg_options(self.analysis.first_leg_attempts);
        let (leg, native) = match registry_leg(
            self.problem,
            SelectedTier::Tier1Explicit,
            method_id(ProblemRegime::Nonstiff),
            self.span,
            self.y0,
            &options,
        )? {
            (leg, MethodResult::Explicit(native)) => (leg, native),
            _ => {
                return Err(SolveError::Driver(
                    "the registry returned no DOPRI5 result for its explicit method".into(),
                ));
            }
        };
        let end_state = if native.final_state.is_empty() {
            self.y0.to_vec()
        } else {
            native.final_state.clone()
        };
        let reason = self.analysis.basis.describe();
        self.record(leg, &end_state, reason);
        self.explicit = Some(native);
        Ok(self.finish())
    }

    /// The break-even route: DOPRI5 with forward break-even pilots, then RODAS5P after an
    /// accepted pilot or a numerical DOPRI5 stop, all under one attempt budget.
    fn break_even(mut self) -> Result<VigilodeSolution, SolveError> {
        let total = self.opts.options.max_steps;
        let options = self.leg_options(total);
        let run = match breakeven::run(self.problem, self.span, self.y0, &options) {
            Ok(run) => run,
            // nothing was integrated
            Err(error) => {
                self.ws.work_counters.fallback_steps += 1;
                let reason = format!("the DOPRI5 session refused before integrating: {error}");
                let (start, state) = (self.span.0, self.y0.to_vec());
                return self.rodas5p(start, state, total, reason);
            }
        };
        let BreakEvenRun {
            b,
            explicit,
            pilots,
            spare_work,
            spare_attempts,
            spare_accepted,
            spare_rejected,
            ending,
        } = run;
        self.pilots = pilots;
        self.ws.work_counters.accumulate(spare_work);
        self.spare_attempts = spare_attempts;
        self.spare_accepted = spare_accepted;
        self.spare_rejected = spare_rejected;

        let route_id = self
            .analysis
            .planned_route
            .map_or_else(|| method_id(ProblemRegime::Nonstiff).to_owned(), |r| r.id());
        let mut leg = explicit_leg(&explicit, route_id, self.span, total);
        let checkpoint_time = explicit.final_time;
        let checkpoint = if explicit.final_state.is_empty() {
            self.y0.to_vec()
        } else {
            explicit.final_state.clone()
        };
        let (stop_reason, callback_failed) =
            (explicit.stop_reason, explicit.callback_failure.is_some());
        let stop_text = explicit.message.clone();
        match &ending {
            Ending::Budget => {
                leg.stop = Some(UnifiedStop::Dopri5 {
                    reason: Some(Dopri5StopReason::AttemptBudget),
                    callback: None,
                    message: format!(
                        "the attempt budget of the solve ({total}) is used up: {} DOPRI5 \
                         attempts and {spare_attempts} pilot attempts",
                        leg.attempts
                    ),
                });
            }
            Ending::Session(message) => {
                leg.stop = Some(UnifiedStop::Other {
                    message: message.clone(),
                });
            }
            Ending::Dopri5 | Ending::Handover(_) | Ending::Fatal(_) => {}
        }
        let reason = self.analysis.basis.describe();
        self.record(leg, &checkpoint, reason);
        self.explicit = Some(explicit);

        match ending {
            Ending::Dopri5 => {
                let remaining = FallbackEngine::remaining_attempts(total, self.used_attempts());
                let follow = !self.legs[0].success
                    && FallbackEngine::follows_explicit_stop(stop_reason, callback_failed);
                if !follow || remaining == 0 {
                    return Ok(self.finish());
                }
                self.ws.work_counters.fallback_steps += 1;
                let reason = format!(
                    "DOPRI5 stopped at t = {checkpoint_time:e} ({stop_text}); RODAS5P continues \
                     from that state with the {remaining} attempts left of {total}"
                );
                self.rodas5p(checkpoint_time, checkpoint, remaining, reason)
            }
            Ending::Handover(handover) => {
                let (pilot_leg, pilot) = *handover;
                self.handover(b, pilot_leg, pilot)
            }
            Ending::Fatal(stop) => {
                self.pilot_stop = Some(stop);
                Ok(self.finish())
            }
            Ending::Budget | Ending::Session(_) => Ok(self.finish()),
        }
    }

    /// RODAS5P after an accepted pilot: the pilot is the first step of the RODAS5P leg, and
    /// RODAS5P continues from its end point with `h0 = H` and the attempts left.
    fn handover(
        mut self,
        b: usize,
        pilot_leg: UnifiedLeg,
        pilot: Solution,
    ) -> Result<VigilodeSolution, SolveError> {
        self.ws.work_counters.fallback_steps += 1;
        let total = self.opts.options.max_steps;
        let record = self.pilots.last().cloned();
        let (t_d, h) = record
            .as_ref()
            .map_or((pilot_leg.t_start, 0.0), |r| (r.t, r.h));
        let window = record.as_ref().map_or_else(String::new, |r| {
            format!(
                "window {} callbacks and {} operations, pilot {} callbacks and {} operations",
                r.window_callbacks, r.window_operations, r.pilot_callbacks, r.pilot_operations
            )
        });
        let t_pilot = pilot_leg.t_end;
        let state = pilot.final_state().map_or_else(Vec::new, <[f64]>::to_vec);
        // the pilot's attempt is charged with the leg it begins
        let remaining =
            FallbackEngine::remaining_attempts(total, self.used_attempts() + pilot_leg.attempts);
        let reason = format!(
            "handover at t = {t_d:e}: one RODAS5P attempt from DOPRI5's accepted state advanced \
             H = {h:e}, the length DOPRI5 advanced in its last {b} attempts ({window}); \
             RODAS5P continues from t = {t_pilot:e} with h0 = H and the {remaining} attempts \
             left of {total}"
        );
        if remaining == 0 || state.is_empty() {
            self.record(pilot_leg, &state, reason);
            self.implicit = Some(pilot);
            return Ok(self.finish());
        }
        let options = {
            let mut options = self.leg_options(remaining);
            options.h0 = h;
            options
        };
        match registry_leg(
            self.problem,
            SelectedTier::TierProtectedReference,
            method_id(ProblemRegime::Stiff),
            (t_pilot, self.span.1),
            &state,
            &options,
        ) {
            Ok((rest, MethodResult::Rodas5p(native))) => {
                let end_state = native.final_state().map_or(state, <[f64]>::to_vec);
                self.record(rodas5p_leg_after_pilot(pilot_leg, rest), &end_state, reason);
                self.implicit = Some(native);
                Ok(self.finish())
            }
            Ok(_) => Err(SolveError::Driver(
                "the registry returned no RODAS5P result for its stiff method".into(),
            )),
            // a prefix exists: keep it and the pilot's step, with the refusal
            Err(error) => {
                self.record(pilot_leg, &state, reason);
                self.implicit = Some(pilot);
                self.refusal = Some((SelectedTier::TierProtectedReference, error.to_string()));
                Ok(self.finish())
            }
        }
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

    /// Build the solution from the executed legs and pilots.
    fn finish(self) -> VigilodeSolution {
        let dimension = self.y0.len();
        let final_time = self.ws.checkpoint_time;
        let final_state = self.ws.y_checkpoint[..dimension].to_vec();
        let total = self.opts.options.max_steps;
        let attempts = self.used_attempts();
        let accepted_steps: usize = self
            .legs
            .iter()
            .map(|leg| leg.accepted_steps)
            .sum::<usize>()
            + self.spare_accepted;
        let rejected_steps: usize = self
            .legs
            .iter()
            .map(|leg| leg.rejected_steps)
            .sum::<usize>()
            + self.spare_rejected;

        let last = self.legs.last();
        let success = self.refusal.is_none()
            && self.pilot_stop.is_none()
            && last.is_some_and(|leg| leg.success);
        let last_stop = last.and_then(|leg| leg.stop.clone());
        let stop = if success {
            None
        } else if let Some(stop) = self.pilot_stop.clone() {
            Some(stop)
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
                    (None, None) => ("none", "none", "no driver ran".to_owned()),
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
            pilots: self.pilots,
            analysis: self.analysis,
            inner_solution: self.implicit,
            explicit_solution: self.explicit,
            switching_solution: self.switching,
        }
    }
}
