//! Opt-in, default-off telemetry of the adaptive controller's output-landing
//! and post-rejection events (research node CT01,
//! `research/ct01_controller_grid_holdout_20261010`).
//!
//! The telemetry is a pure observer: it reads the inputs and the result of
//! each controller update ([`crate::adaptive_next_step_after_attempt`]) and
//! never feeds anything back, so every state, decision and counter of a run
//! is the same with it on or off. Only
//! [`crate::integrate_rodas5p_fast_observed_with_telemetry`] turns it on;
//! every other entry point passes no observer.
//!
//! "Rejection pending" is the observer's own flag with the rule of
//! [`crate::ControllerKind::PredictiveCapped2`] (research node ALG05),
//! applied under every controller kind so the counts compare across kinds:
//! set by every rejected or failed attempt, cleared only by an accepted
//! non-sliver step (a non-clipped trial, or a clipped trial of at least
//! [`CLIPPED_SAMPLE_INFORMATIVE_RATIO`] times the request). Under
//! `PredictiveCapped2` it equals the controller's own
//! [`crate::AdaptiveControllerState::rejection_pending`] after every update.

use serde::Serialize;

use crate::CLIPPED_SAMPLE_INFORMATIVE_RATIO;

/// One controller update as the driver performed it: the inputs of
/// [`crate::adaptive_next_step_after_attempt`] and the step it returned.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ControllerDecision {
    pub requested_h: f64,
    pub trial_h: f64,
    pub error: f64,
    pub accepted: bool,
    pub clipped: bool,
    pub next_h: f64,
}

/// Per-run counts of the registered CT01 controller events.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ControllerTelemetry {
    /// Accepted attempts whose trial step the output schedule shortened.
    pub clipped_landings: usize,
    /// Clipped landings with `trial < CLIPPED_SAMPLE_INFORMATIVE_RATIO x
    /// request`.
    pub sliver_landings: usize,
    /// Sliver landings while a rejection is pending.
    pub sliver_landings_while_rejection_pending: usize,
    /// Informative (non-sliver) clipped landings while a rejection is
    /// pending.
    pub informative_clipped_landings_after_rejection: usize,
    /// Accepted attempts with `err = 0` while a rejection is pending.
    pub zero_error_accepts_while_rejection_pending: usize,
    /// Accepted attempts whose next requested step exceeds the actual trial
    /// step.
    pub accepted_next_request_exceeds_trial: usize,
    /// Controller updates observed (one per attempt).
    pub updates: usize,
    /// The observer's pending-rejection flag after the last update.
    pub rejection_pending: bool,
    /// Every update in order, when recording was requested
    /// ([`Self::with_trace`]); never serialized.
    #[serde(skip)]
    pub trace: Option<Vec<ControllerDecision>>,
}

/// The events one update produced, for [`ControllerTelemetry::observe`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ControllerEvents {
    pub clipped_landing: bool,
    pub sliver_landing: bool,
    pub sliver_landing_while_rejection_pending: bool,
    pub informative_clipped_landing_after_rejection: bool,
    pub zero_error_accept_while_rejection_pending: bool,
    pub accepted_next_request_exceeds_trial: bool,
}

impl ControllerTelemetry {
    /// Counting telemetry that also records every update.
    pub fn with_trace() -> Self {
        Self {
            trace: Some(Vec::new()),
            ..Self::default()
        }
    }

    /// Classify one controller update (pending flag taken before it), count
    /// its events, record it if tracing, and advance the pending flag.
    pub fn observe(&mut self, decision: ControllerDecision) -> ControllerEvents {
        let ControllerDecision {
            requested_h,
            trial_h,
            error,
            accepted,
            clipped,
            next_h,
        } = decision;
        let pending = self.rejection_pending;
        let mut events = ControllerEvents::default();
        if accepted {
            let sliver = clipped && trial_h / requested_h < CLIPPED_SAMPLE_INFORMATIVE_RATIO;
            events.clipped_landing = clipped;
            events.sliver_landing = sliver;
            events.sliver_landing_while_rejection_pending = sliver && pending;
            events.informative_clipped_landing_after_rejection = clipped && !sliver && pending;
            events.zero_error_accept_while_rejection_pending = error == 0.0 && pending;
            events.accepted_next_request_exceeds_trial = next_h > trial_h;
            if !sliver {
                self.rejection_pending = false;
            }
        } else {
            self.rejection_pending = true;
        }
        self.updates += 1;
        self.clipped_landings += usize::from(events.clipped_landing);
        self.sliver_landings += usize::from(events.sliver_landing);
        self.sliver_landings_while_rejection_pending +=
            usize::from(events.sliver_landing_while_rejection_pending);
        self.informative_clipped_landings_after_rejection +=
            usize::from(events.informative_clipped_landing_after_rejection);
        self.zero_error_accepts_while_rejection_pending +=
            usize::from(events.zero_error_accept_while_rejection_pending);
        self.accepted_next_request_exceeds_trial +=
            usize::from(events.accepted_next_request_exceeds_trial);
        if let Some(trace) = self.trace.as_mut() {
            trace.push(decision);
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision(requested_h: f64, trial_h: f64, error: f64, accepted: bool) -> ControllerDecision {
        ControllerDecision {
            requested_h,
            trial_h,
            error,
            accepted,
            clipped: trial_h < requested_h,
            next_h: trial_h,
        }
    }

    #[test]
    fn the_events_follow_the_registered_definitions() {
        let mut t = ControllerTelemetry::default();
        // A rejection sets the pending flag.
        assert_eq!(
            t.observe(decision(1.0, 1.0, 2.0, false)),
            ControllerEvents::default()
        );
        assert!(t.rejection_pending);
        // A sliver landing keeps it.
        let e = t.observe(decision(1.0, 0.25, 0.1, true));
        assert!(e.clipped_landing && e.sliver_landing && e.sliver_landing_while_rejection_pending);
        assert!(t.rejection_pending);
        // err = 0 accepted on a sliver while pending.
        let e = t.observe(decision(1.0, 0.25, 0.0, true));
        assert!(e.zero_error_accept_while_rejection_pending);
        // An informative clipped landing clears it, with request restoration.
        let e = t.observe(ControllerDecision {
            next_h: 1.0,
            ..decision(1.0, 0.75, 0.1, true)
        });
        assert!(e.informative_clipped_landing_after_rejection);
        assert!(e.accepted_next_request_exceeds_trial);
        assert!(!t.rejection_pending);
        // Without a pending rejection nothing post-rejection is counted.
        let e = t.observe(decision(1.0, 0.25, 0.0, true));
        assert!(e.sliver_landing && !e.sliver_landing_while_rejection_pending);
        assert!(!e.zero_error_accept_while_rejection_pending);
        assert_eq!(t.updates, 5);
        assert_eq!(t.clipped_landings, 4);
        assert_eq!(t.sliver_landings, 3);
        assert_eq!(t.sliver_landings_while_rejection_pending, 2);
        assert_eq!(t.informative_clipped_landings_after_rejection, 1);
        assert_eq!(t.zero_error_accepts_while_rejection_pending, 1);
        assert_eq!(t.accepted_next_request_exceeds_trial, 1);
        assert!(t.trace.is_none());
    }
}
