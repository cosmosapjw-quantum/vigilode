//! Finite-state hybrid route controller, anti-chattering dwell/hysteresis,
//! route epoch invalidation, speculative work accounting, and fail-safe fallback.
//!
//! Node P8 (P8_HYBRID_CONTROLLER) implements a standalone / experimental discrete
//! verification model and contract reference for causal, bounded, zero-allocation
//! switching between DOPRI5 and RODAS5P integrators.
//!
//! # Architecture & Operational Dispatch
//!
//! - **Standalone / Experimental Verification Model**: [`HybridRouteAutomaton`] defines the
//!   formal discrete-event finite state machine (FSM), valid transition graphs, anti-chattering
//!   dwell/hysteresis constraints, epoch validation, and speculative work accounting invariants.
//!   It serves as the contract and offline verification oracle.
//! - **Live Operational Route Controller**: Runtime adaptive dispatch within [`crate::SwitchingSolver`]
//!   is driven directly by `DriveState` in [`crate::switching`], which manages live method selection
//!   ([`crate::switching::SwitchingMethod`]), window progression, step proposals, adaptive pilot trials,
//!   and recheck sentinels during active numerical integration.

use core::fmt;
use rodas5p_core::WorkCounters;
use serde::{Deserialize, Serialize};

/// Maximum circular buffer capacity for tracking transition history without heap allocation.
pub const MAX_TRANSITION_HISTORY: usize = 64;

/// Route finite-state automaton state for hybrid switching execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteFsmState {
    /// Pure DOPRI5 explicit stepping is active.
    ExplicitActive,
    /// Checking P5 secant screen or P4 nonnormal observer for stiffness.
    ScreeningActive,
    /// Speculative RODAS5P implicit pilot trial from the same checkpoint.
    ImplicitPilotTrial,
    /// Running under RODAS5P implicit integrator.
    ImplicitActive,
    /// Sentinel / probe checking return to DOPRI5 explicit integrator.
    ExplicitReturnPilot,
    /// Discarded speculative trial, rolling back state to checkpoint.
    RollbackRecovery,
    /// Fail-safe emergency fallback to protected original RODAS5P.
    ProtectedRodasFallback,
    /// Solve completed and reached termination.
    Terminated,
}

impl RouteFsmState {
    /// Returns true if this state is in the explicit execution route family.
    #[inline]
    pub fn is_explicit_route_family(&self) -> bool {
        matches!(
            self,
            RouteFsmState::ExplicitActive
                | RouteFsmState::ScreeningActive
                | RouteFsmState::ImplicitPilotTrial
                | RouteFsmState::RollbackRecovery
        )
    }

    /// Returns true if this state is in the implicit execution route family.
    #[inline]
    pub fn is_implicit_route_family(&self) -> bool {
        matches!(
            self,
            RouteFsmState::ImplicitActive
                | RouteFsmState::ExplicitReturnPilot
                | RouteFsmState::ProtectedRodasFallback
        )
    }

    /// Returns true if this state is terminal.
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(self, RouteFsmState::Terminated)
    }
}

/// Events driving transitions in the hybrid route automaton.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteFsmEvent {
    /// Initial start of integration solve.
    StartSolve,
    /// Work pressure or stiffness detected on the explicit path.
    PressureOrStiffnessDetected,
    /// Observer screen passed with certified or predictive evidence.
    ScreenPassedWithEvidence,
    /// Speculative implicit pilot trial completed cheaper than explicit baseline.
    PilotCompletedCheaper,
    /// Speculative implicit pilot trial failed or was more expensive.
    PilotFailedOrExpensive,
    /// An implicit step or window was accepted.
    ImplicitWindowAccepted,
    /// Sentinel schedule indicates a return-to-explicit probe is due.
    ReturnSentinelDue,
    /// Explicit return pilot completed cheaper than implicit path.
    ReturnPilotCheaper,
    /// Explicit return pilot failed or was more expensive.
    ReturnPilotFailedOrExpensive,
    /// Numerical deterioration, kink, or shock detected.
    DeteriorationOrKinkDetected,
    /// Fatal error or unrecoverable condition triggered emergency fallback.
    FatalFailureTriggered,
    /// End time reached or terminal event satisfied, completing the solve.
    CompleteSolve,
}

/// Errors occurring during hybrid route automaton evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HybridAutomatonError {
    /// Illegal transition attempted from the current state on the given event.
    IllegalTransition {
        /// Current state when event occurred.
        from: RouteFsmState,
        /// Event that was rejected.
        event: RouteFsmEvent,
    },
    /// Stale evidence ticket rejected due to route epoch mismatch.
    StaleEvidenceEpoch {
        /// Active route epoch.
        current_epoch: u64,
        /// Stale ticket epoch.
        ticket_epoch: u64,
    },
}

impl fmt::Display for HybridAutomatonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IllegalTransition { from, event } => {
                write!(
                    f,
                    "Illegal hybrid route transition: no valid edge from state {:?} on event {:?}",
                    from, event
                )
            }
            Self::StaleEvidenceEpoch {
                current_epoch,
                ticket_epoch,
            } => {
                write!(
                    f,
                    "Stale evidence epoch {ticket_epoch} does not match current route epoch {current_epoch}"
                )
            }
        }
    }
}

impl std::error::Error for HybridAutomatonError {}

/// An immutable record of an executed FSM transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionRecord {
    /// Originating state.
    pub from: RouteFsmState,
    /// Destination state.
    pub to: RouteFsmState,
    /// Triggering event.
    pub event: RouteFsmEvent,
    /// Monotonic sequence ID of this transition.
    pub transition_id: u64,
}

/// Anti-chattering and hysteresis policy to eliminate Zeno/oscillating route switches.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AntiChatteringPolicy {
    /// Minimum batches to dwell in a route before switching (default: 3).
    pub min_dwell_batches: usize,
    /// Stiffness metric threshold to enter implicit mode (default: 0.8).
    pub hysteresis_enter_threshold: f64,
    /// Cost ratio threshold to exit implicit mode back to explicit (default: 0.4).
    pub hysteresis_exit_threshold: f64,
}

impl Default for AntiChatteringPolicy {
    fn default() -> Self {
        Self {
            min_dwell_batches: 3,
            hysteresis_enter_threshold: 0.8,
            hysteresis_exit_threshold: 0.4,
        }
    }
}

impl AntiChatteringPolicy {
    /// Creates a new policy with explicit dwell and hysteresis parameters.
    pub fn new(min_dwell_batches: usize, enter_threshold: f64, exit_threshold: f64) -> Self {
        Self {
            min_dwell_batches,
            hysteresis_enter_threshold: enter_threshold,
            hysteresis_exit_threshold: exit_threshold,
        }
    }

    /// Evaluates whether switching to implicit mode is permitted.
    ///
    /// Requires dwell >= `min_dwell_batches` and stiffness metric >= `hysteresis_enter_threshold`.
    #[inline]
    pub fn can_switch_to_implicit(&self, dwell: usize, stiffness_metric: f64) -> bool {
        dwell >= self.min_dwell_batches && stiffness_metric >= self.hysteresis_enter_threshold
    }

    /// Evaluates whether switching back to explicit mode is permitted.
    ///
    /// Requires dwell >= `min_dwell_batches` and cost ratio metric <= `hysteresis_exit_threshold`.
    #[inline]
    pub fn can_switch_to_explicit(&self, dwell: usize, cost_ratio_metric: f64) -> bool {
        dwell >= self.min_dwell_batches && cost_ratio_metric <= self.hysteresis_exit_threshold
    }
}

/// Route epoch tracker to invalidate stale evidence across switches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEpochTracker {
    current_epoch: u64,
}

impl RouteEpochTracker {
    /// Creates a new route epoch tracker initialized at epoch 0.
    pub fn new() -> Self {
        Self { current_epoch: 0 }
    }

    /// Returns the current active route epoch.
    #[inline]
    pub fn current_epoch(&self) -> u64 {
        self.current_epoch
    }

    /// Increments the epoch on any route switch (Explicit <-> Implicit).
    pub fn increment_epoch(&mut self) -> u64 {
        self.current_epoch = self.current_epoch.saturating_add(1);
        self.current_epoch
    }

    /// Validates whether evidence or ticket generated at `ticket_epoch` remains valid.
    #[inline]
    pub fn is_evidence_valid(&self, ticket_epoch: u64) -> bool {
        ticket_epoch == self.current_epoch
    }

    /// Issues an evidence ticket stamped with the active route epoch.
    #[inline]
    pub fn issue_ticket(&self) -> u64 {
        self.current_epoch
    }
}

/// Tracks committed work and speculative/refused work to guarantee 100% work accounting.
///
/// On rollback or discard of candidate state, the spent work (RHS, JVP, solves)
/// is NEVER discarded: it is fully accumulated into total work (`accounting_complete`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeculativeWorkTracker {
    committed_work: WorkCounters,
    refused_work: WorkCounters,
    pending_speculative_work: WorkCounters,
}

impl SpeculativeWorkTracker {
    /// Creates a new tracker initialized with zero counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records work committed into the accepted trajectory.
    pub fn record_committed(&mut self, work: WorkCounters) {
        self.committed_work.accumulate(work);
    }

    /// Records work spent during speculative pilot trials.
    pub fn record_speculative(&mut self, work: WorkCounters) {
        self.pending_speculative_work.accumulate(work);
    }

    /// Commits pending speculative work into committed work on pilot acceptance.
    pub fn commit_speculative(&mut self) {
        self.committed_work
            .accumulate(self.pending_speculative_work);
        self.pending_speculative_work = WorkCounters::default();
    }

    /// Discards candidate state but retains all spent speculative work in refused work.
    pub fn rollback_and_retain_work(&mut self) {
        self.refused_work.accumulate(self.pending_speculative_work);
        self.pending_speculative_work = WorkCounters::default();
    }

    /// Returns a reference to committed work counters.
    pub fn committed_work(&self) -> &WorkCounters {
        &self.committed_work
    }

    /// Returns a reference to refused speculative work counters.
    pub fn refused_work(&self) -> &WorkCounters {
        &self.refused_work
    }

    /// Returns a reference to pending speculative work counters.
    pub fn pending_speculative_work(&self) -> &WorkCounters {
        &self.pending_speculative_work
    }

    /// Returns total work accumulated across committed, refused, and pending work.
    pub fn total_work(&self) -> WorkCounters {
        let mut total = self.committed_work;
        total.accumulate(self.refused_work);
        total.accumulate(self.pending_speculative_work);
        total
    }

    /// Checks that all spent work is fully accounted for without omission.
    pub fn accounting_complete(&self) -> bool {
        let mut sum = self.committed_work;
        sum.accumulate(self.refused_work);
        sum.accumulate(self.pending_speculative_work);
        sum == self.total_work()
    }
}

/// Guarantor ensuring protected original RODAS5P is always available as an unconditional fail-safe route.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedFallbackGuarantor {
    fallback_invocations: u64,
}

impl ProtectedFallbackGuarantor {
    /// Creates a new protected fallback guarantor.
    pub fn new() -> Self {
        Self {
            fallback_invocations: 0,
        }
    }

    /// Fallback to protected original RODAS5P is unconditionally permitted from ANY state.
    #[inline]
    pub fn is_fallback_permitted_from(&self, _state: RouteFsmState) -> bool {
        true
    }

    /// Triggers emergency fallback transition on the provided automaton.
    pub fn trigger_emergency_fallback(
        &mut self,
        automaton: &mut HybridRouteAutomaton,
    ) -> Result<(), HybridAutomatonError> {
        self.fallback_invocations = self.fallback_invocations.saturating_add(1);
        automaton
            .handle_event(RouteFsmEvent::FatalFailureTriggered)
            .map(|_| ())
    }

    /// Returns the number of times emergency fallback was invoked.
    #[inline]
    pub fn fallback_invocations(&self) -> u64 {
        self.fallback_invocations
    }
}

/// Hybrid route finite-state automaton governing explicit and implicit ODE integration.
///
/// **Role**: Standalone / experimental discrete verification model and contract reference (Node P8).
///
/// Note that live operational route dispatch in [`crate::SwitchingSolver`] is managed
/// directly by `DriveState` in `crates/vigilode/src/switching.rs`. This automaton provides
/// an independent, fully-instrumented discrete-event reference model for validating transition
/// legality, anti-chattering hysteresis, epoch invalidation, and speculative work conservation
/// without heap allocations (VG-1).
///
/// Pre-allocates all internal history buffers to ensure zero heap allocations during execution (VG-1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HybridRouteAutomaton {
    state: RouteFsmState,
    transition_count: u64,
    history: [Option<TransitionRecord>; MAX_TRANSITION_HISTORY],
    epoch_tracker: RouteEpochTracker,
    anti_chattering: AntiChatteringPolicy,
    speculative_tracker: SpeculativeWorkTracker,
    fallback_guarantor: ProtectedFallbackGuarantor,
}

impl Default for HybridRouteAutomaton {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridRouteAutomaton {
    /// Creates a new hybrid route automaton in the `ExplicitActive` state.
    pub fn new() -> Self {
        Self {
            state: RouteFsmState::ExplicitActive,
            transition_count: 0,
            history: [None; MAX_TRANSITION_HISTORY],
            epoch_tracker: RouteEpochTracker::new(),
            anti_chattering: AntiChatteringPolicy::default(),
            speculative_tracker: SpeculativeWorkTracker::new(),
            fallback_guarantor: ProtectedFallbackGuarantor::new(),
        }
    }

    /// Creates an automaton with a custom anti-chattering policy.
    pub fn with_anti_chattering(anti_chattering: AntiChatteringPolicy) -> Self {
        Self {
            anti_chattering,
            ..Self::new()
        }
    }

    /// Returns the current FSM state.
    #[inline]
    pub fn state(&self) -> RouteFsmState {
        self.state
    }

    /// Sets the FSM state directly for test harness setup.
    #[inline]
    pub fn set_state_for_testing(&mut self, state: RouteFsmState) {
        self.state = state;
    }

    /// Returns the total number of transitions executed.
    #[inline]
    pub fn transition_count(&self) -> u64 {
        self.transition_count
    }

    /// Returns a reference to the active route epoch tracker.
    #[inline]
    pub fn epoch_tracker(&self) -> &RouteEpochTracker {
        &self.epoch_tracker
    }

    /// Returns a mutable reference to the active route epoch tracker.
    #[inline]
    pub fn epoch_tracker_mut(&mut self) -> &mut RouteEpochTracker {
        &mut self.epoch_tracker
    }

    /// Returns a reference to the anti-chattering policy.
    #[inline]
    pub fn anti_chattering(&self) -> &AntiChatteringPolicy {
        &self.anti_chattering
    }

    /// Returns a mutable reference to the anti-chattering policy.
    #[inline]
    pub fn anti_chattering_mut(&mut self) -> &mut AntiChatteringPolicy {
        &mut self.anti_chattering
    }

    /// Returns a reference to the speculative work tracker.
    #[inline]
    pub fn speculative_tracker(&self) -> &SpeculativeWorkTracker {
        &self.speculative_tracker
    }

    /// Returns a mutable reference to the speculative work tracker.
    #[inline]
    pub fn speculative_tracker_mut(&mut self) -> &mut SpeculativeWorkTracker {
        &mut self.speculative_tracker
    }

    /// Returns a reference to the protected fallback guarantor.
    #[inline]
    pub fn fallback_guarantor(&self) -> &ProtectedFallbackGuarantor {
        &self.fallback_guarantor
    }

    /// Returns a mutable reference to the protected fallback guarantor.
    #[inline]
    pub fn fallback_guarantor_mut(&mut self) -> &mut ProtectedFallbackGuarantor {
        &mut self.fallback_guarantor
    }

    /// Returns the most recent transition record, if any transitions have occurred.
    pub fn last_transition(&self) -> Option<TransitionRecord> {
        if self.transition_count == 0 {
            None
        } else {
            let slot = ((self.transition_count - 1) as usize) % MAX_TRANSITION_HISTORY;
            self.history[slot]
        }
    }

    /// Returns the number of transitions preserved in the history buffer.
    pub fn history_len(&self) -> usize {
        (self.transition_count as usize).min(MAX_TRANSITION_HISTORY)
    }

    /// Iterates over recent transition records in chronological order without heap allocation.
    pub fn history(&self) -> impl Iterator<Item = TransitionRecord> + '_ {
        let count = self.transition_count as usize;
        let len = count.min(MAX_TRANSITION_HISTORY);
        let start = count.saturating_sub(len);
        (start..count).filter_map(move |idx| {
            let slot = idx % MAX_TRANSITION_HISTORY;
            self.history[slot]
        })
    }

    /// Evaluates an event against the current state, enforcing the explicit transition table.
    ///
    /// Returns `Ok(new_state)` on permitted transitions, or `Err(HybridAutomatonError::IllegalTransition)`
    /// on any invalid transition. Zero heap allocations (VG-1).
    pub fn handle_event(
        &mut self,
        event: RouteFsmEvent,
    ) -> Result<RouteFsmState, HybridAutomatonError> {
        let from = self.state;
        let next_state = match (from, event) {
            // 1. ExplicitActive
            (RouteFsmState::ExplicitActive, RouteFsmEvent::StartSolve) => {
                RouteFsmState::ExplicitActive
            }
            (RouteFsmState::ExplicitActive, RouteFsmEvent::PressureOrStiffnessDetected) => {
                RouteFsmState::ScreeningActive
            }
            (RouteFsmState::ExplicitActive, RouteFsmEvent::DeteriorationOrKinkDetected) => {
                RouteFsmState::ScreeningActive
            }
            (RouteFsmState::ExplicitActive, RouteFsmEvent::CompleteSolve) => {
                RouteFsmState::Terminated
            }
            (RouteFsmState::ExplicitActive, RouteFsmEvent::FatalFailureTriggered) => {
                RouteFsmState::ProtectedRodasFallback
            }

            // 2. ScreeningActive
            (RouteFsmState::ScreeningActive, RouteFsmEvent::ScreenPassedWithEvidence) => {
                RouteFsmState::ImplicitPilotTrial
            }
            (RouteFsmState::ScreeningActive, RouteFsmEvent::PilotFailedOrExpensive) => {
                RouteFsmState::ExplicitActive
            }
            (RouteFsmState::ScreeningActive, RouteFsmEvent::DeteriorationOrKinkDetected) => {
                RouteFsmState::ImplicitPilotTrial
            }
            (RouteFsmState::ScreeningActive, RouteFsmEvent::CompleteSolve) => {
                RouteFsmState::Terminated
            }
            (RouteFsmState::ScreeningActive, RouteFsmEvent::FatalFailureTriggered) => {
                RouteFsmState::ProtectedRodasFallback
            }

            // 3. ImplicitPilotTrial
            (RouteFsmState::ImplicitPilotTrial, RouteFsmEvent::PilotCompletedCheaper) => {
                RouteFsmState::ImplicitActive
            }
            (RouteFsmState::ImplicitPilotTrial, RouteFsmEvent::PilotFailedOrExpensive) => {
                RouteFsmState::RollbackRecovery
            }
            (RouteFsmState::ImplicitPilotTrial, RouteFsmEvent::DeteriorationOrKinkDetected) => {
                RouteFsmState::RollbackRecovery
            }
            (RouteFsmState::ImplicitPilotTrial, RouteFsmEvent::FatalFailureTriggered) => {
                RouteFsmState::ProtectedRodasFallback
            }

            // 4. ImplicitActive
            (RouteFsmState::ImplicitActive, RouteFsmEvent::ImplicitWindowAccepted) => {
                RouteFsmState::ImplicitActive
            }
            (RouteFsmState::ImplicitActive, RouteFsmEvent::ReturnSentinelDue) => {
                RouteFsmState::ExplicitReturnPilot
            }
            (RouteFsmState::ImplicitActive, RouteFsmEvent::DeteriorationOrKinkDetected) => {
                RouteFsmState::RollbackRecovery
            }
            (RouteFsmState::ImplicitActive, RouteFsmEvent::CompleteSolve) => {
                RouteFsmState::Terminated
            }
            (RouteFsmState::ImplicitActive, RouteFsmEvent::FatalFailureTriggered) => {
                RouteFsmState::ProtectedRodasFallback
            }

            // 5. ExplicitReturnPilot
            (RouteFsmState::ExplicitReturnPilot, RouteFsmEvent::ReturnPilotCheaper) => {
                RouteFsmState::ExplicitActive
            }
            (RouteFsmState::ExplicitReturnPilot, RouteFsmEvent::ReturnPilotFailedOrExpensive) => {
                RouteFsmState::ImplicitActive
            }
            (RouteFsmState::ExplicitReturnPilot, RouteFsmEvent::DeteriorationOrKinkDetected) => {
                RouteFsmState::RollbackRecovery
            }
            (RouteFsmState::ExplicitReturnPilot, RouteFsmEvent::FatalFailureTriggered) => {
                RouteFsmState::ProtectedRodasFallback
            }

            // 6. RollbackRecovery
            (RouteFsmState::RollbackRecovery, RouteFsmEvent::StartSolve) => {
                RouteFsmState::ExplicitActive
            }
            (RouteFsmState::RollbackRecovery, RouteFsmEvent::PilotFailedOrExpensive) => {
                RouteFsmState::ExplicitActive
            }
            (RouteFsmState::RollbackRecovery, RouteFsmEvent::ReturnPilotFailedOrExpensive) => {
                RouteFsmState::ImplicitActive
            }
            (RouteFsmState::RollbackRecovery, RouteFsmEvent::ImplicitWindowAccepted) => {
                RouteFsmState::ImplicitActive
            }
            (RouteFsmState::RollbackRecovery, RouteFsmEvent::CompleteSolve) => {
                RouteFsmState::Terminated
            }
            (RouteFsmState::RollbackRecovery, RouteFsmEvent::FatalFailureTriggered) => {
                RouteFsmState::ProtectedRodasFallback
            }

            // 7. ProtectedRodasFallback
            (RouteFsmState::ProtectedRodasFallback, RouteFsmEvent::ImplicitWindowAccepted) => {
                RouteFsmState::ProtectedRodasFallback
            }
            (RouteFsmState::ProtectedRodasFallback, RouteFsmEvent::CompleteSolve) => {
                RouteFsmState::Terminated
            }
            (RouteFsmState::ProtectedRodasFallback, RouteFsmEvent::FatalFailureTriggered) => {
                RouteFsmState::ProtectedRodasFallback
            }

            // 8. Terminated
            (RouteFsmState::Terminated, RouteFsmEvent::FatalFailureTriggered) => {
                RouteFsmState::ProtectedRodasFallback
            }

            // Fail-closed on all unpermitted transitions
            _ => return Err(HybridAutomatonError::IllegalTransition { from, event }),
        };

        // Automatic route epoch increment on route family changes (Explicit <-> Implicit)
        let switched_route = (from.is_explicit_route_family()
            && next_state.is_implicit_route_family())
            || (from.is_implicit_route_family() && next_state.is_explicit_route_family());
        if switched_route {
            self.epoch_tracker.increment_epoch();
        }

        // Automatic speculative work retention/commit on candidate trial resolution
        match (from, next_state) {
            (RouteFsmState::ImplicitPilotTrial, RouteFsmState::ImplicitActive) => {
                self.speculative_tracker.commit_speculative();
            }
            (RouteFsmState::ImplicitPilotTrial, RouteFsmState::RollbackRecovery) => {
                self.speculative_tracker.rollback_and_retain_work();
            }
            (RouteFsmState::ExplicitReturnPilot, RouteFsmState::ExplicitActive) => {
                self.speculative_tracker.commit_speculative();
            }
            (RouteFsmState::ExplicitReturnPilot, RouteFsmState::ImplicitActive) => {
                self.speculative_tracker.rollback_and_retain_work();
            }
            (_, RouteFsmState::ProtectedRodasFallback) | (_, RouteFsmState::RollbackRecovery) => {
                self.speculative_tracker.rollback_and_retain_work();
            }
            _ => {}
        }

        // Record transition in circular history buffer
        let record = TransitionRecord {
            from,
            to: next_state,
            event,
            transition_id: self.transition_count,
        };
        let slot = (self.transition_count as usize) % MAX_TRANSITION_HISTORY;
        self.history[slot] = Some(record);
        self.transition_count = self.transition_count.saturating_add(1);
        self.state = next_state;

        Ok(next_state)
    }
}
