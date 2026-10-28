//! Deterministic work-aware selection between a projected root and the protected
//! same-target raw-U solve. Routing changes cost; acceptance remains external.
use crate::{CausalSchedule, CoupledTrajectoryAttempt, Work};
use rodas5p_core::{CoreError, CoreResult};
use serde::Serialize;

/// Projected root algorithm being reconsidered. Selection does not grant it an
/// accuracy exemption. Picard may use the caller's explicitly supplied workers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum CandidateRoute {
    #[default]
    Forward,
    Picard,
    /// Same projected target with fixed-input RHS and exact prefix reuse.
    PicardReuse,
    ResidualFirst,
}
impl CandidateRoute {
    pub(crate) fn schedule(self) -> Option<CausalSchedule> {
        match self {
            Self::Forward => Some(CausalSchedule::Forward),
            Self::Picard => Some(CausalSchedule::BatchedPicard),
            Self::PicardReuse => Some(CausalSchedule::BatchedPicardReuse),
            Self::ResidualFirst => None,
        }
    }
}

/// Cost proxy weights, in caller-chosen work units. Defaults count RHS and JVP
/// callback attempts equally. They are neither elapsed time nor flop counts.
/// Optional projection/factor weights let a measured workload price overhead.
#[derive(Clone, Debug, Serialize)]
pub struct CausalWorkWeights {
    pub rhs: f64,
    pub jvp: f64,
    pub orthogonalization_dot: f64,
    pub small_factorization: f64,
    pub small_block_solve: f64,
}
impl Default for CausalWorkWeights {
    fn default() -> Self {
        Self {
            rhs: 1.0,
            jvp: 1.0,
            orthogonalization_dot: 0.0,
            small_factorization: 0.0,
            small_block_solve: 0.0,
        }
    }
}
impl CausalWorkWeights {
    /// Construct a cost proxy from caller-measured *serial* mean seconds per
    /// event in the order RHS, JVP, dot product, small factorization, small solve.
    /// Measurements must use the actual callback/provider/dimensions in scope.
    /// Zero optional entries mean unpriced work; RHS and JVP must be positive.
    /// These means price attempted events and do not predict parallel wall time.
    pub fn from_measured_serial_means(seconds: [f64; 5]) -> CoreResult<Self> {
        if seconds.iter().any(|x| !x.is_finite() || *x < 0.0)
            || seconds[0] <= 0.0
            || seconds[1] <= 0.0
        {
            return Err(CoreError::InvalidInput(
                "invalid measured causal event means".into(),
            ));
        }
        Ok(Self {
            rhs: seconds[0],
            jvp: seconds[1],
            orthogonalization_dot: seconds[2],
            small_factorization: seconds[3],
            small_block_solve: seconds[4],
        })
    }
    fn score(&self, w: &Work) -> f64 {
        self.rhs * w.rhs_attempted as f64
            + self.jvp * w.jvp_attempted as f64
            + self.orthogonalization_dot * w.orthogonalization_dots as f64
            + self.small_factorization * w.small_factorizations as f64
            + self.small_block_solve * w.small_block_solves as f64
    }
    pub(crate) fn candidate_score(&self, r: &CoupledTrajectoryAttempt) -> f64 {
        self.score(&r.candidate_work)
    }
    pub(crate) fn protected_score(&self, r: &CoupledTrajectoryAttempt) -> f64 {
        let w = &r.fallback_work;
        self.rhs * w.rhs_calls as f64
            + self.jvp * w.jvp_calls as f64
            + self.orthogonalization_dot * w.orthogonalization_inner_products as f64
            + self.small_factorization * w.direct_factorizations as f64
            + self.small_block_solve * w.direct_solve_calls as f64
            + self.score(&r.fallback_validation_work)
    }
}

/// Optional locality conditions for reusing a protected cost reference.
/// These are routing heuristics, not stiffness or accuracy certificates. The
/// normalized state distance is max_i |y_i-y_ref_i|/(absolute + relative*max
/// (|y_i|,|y_ref_i|)). A distance at most one is admitted. Tests must price the
/// O(n) comparison and state copy; no additional RHS/JVP calls are used.
#[derive(Clone, Debug, Serialize)]
pub struct CausalReferenceGuard {
    /// Symmetric maximum ratio between actual attempted step sizes, >= 1.
    pub max_step_ratio: f64,
    /// Positive state scale in the client's state units.
    pub state_absolute_scale: f64,
    /// Nonnegative relative state scale.
    pub state_relative_scale: f64,
    /// Accepted steps since the reference before forcing a fresh calibration.
    pub max_accepted_age: usize,
}
impl Default for CausalReferenceGuard {
    fn default() -> Self {
        Self {
            max_step_ratio: 4.0,
            state_absolute_scale: 1e-6,
            state_relative_scale: 0.5,
            max_accepted_age: 32,
        }
    }
}

/// Opt-in bounded-exploration policy. One accepted protected step calibrates the
/// proxy. A candidate is retained at `retain_ratio` and re-enters after cooldown
/// only at the stricter `reentry_ratio`. Failed projected roots cool down
/// immediately, including failures followed by an accepted protected endpoint.
/// Cooldown counts accepted protected steps, not callback or local-error retries.
#[derive(Clone, Debug, Serialize)]
pub struct CausalSwitchPolicy {
    pub candidate: CandidateRoute,
    pub weights: CausalWorkWeights,
    pub retain_ratio: f64,
    pub reentry_ratio: f64,
    pub initial_cooldown: usize,
    pub max_cooldown: usize,
    /// Price failed exploration using C/((L+1)P) <= epsilon for representative
    /// stationary scores. None preserves the original exponential cooldown.
    /// A finite maximum L can prevent meeting this proxy target; receipts say so.
    pub exploration_overhead_fraction: Option<f64>,
    /// None preserves historical h/state transfer. Model-epoch changes always
    /// discard a reference. Some makes the cost reference local to actual h,
    /// state and accepted-step age and uses fresh (not averaged) scores.
    pub reference_guard: Option<CausalReferenceGuard>,
}
impl Default for CausalSwitchPolicy {
    fn default() -> Self {
        Self {
            candidate: CandidateRoute::Forward,
            weights: CausalWorkWeights::default(),
            retain_ratio: 1.25,
            reentry_ratio: 1.0,
            initial_cooldown: 4,
            max_cooldown: 64,
            exploration_overhead_fraction: Some(0.25),
            reference_guard: None,
        }
    }
}
impl CausalSwitchPolicy {
    pub(crate) fn validate(&self) -> CoreResult<()> {
        let w = &self.weights;
        if self.reference_guard.as_ref().is_some_and(|g| {
            !g.max_step_ratio.is_finite()
                || g.max_step_ratio < 1.0
                || !g.state_absolute_scale.is_finite()
                || g.state_absolute_scale <= 0.0
                || !g.state_relative_scale.is_finite()
                || g.state_relative_scale < 0.0
                || g.max_accepted_age == 0
        }) {
            return Err(CoreError::InvalidInput(
                "invalid causal reference guard".into(),
            ));
        }
        if [
            w.rhs,
            w.jvp,
            w.orthogonalization_dot,
            w.small_factorization,
            w.small_block_solve,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x < 0.0)
            || w.rhs <= 0.0
            || w.jvp <= 0.0
            || !self.retain_ratio.is_finite()
            || !self.reentry_ratio.is_finite()
            || self.reentry_ratio <= 0.0
            || self.reentry_ratio > self.retain_ratio
            || self.initial_cooldown == 0
            || self.max_cooldown < self.initial_cooldown
            || self
                .exploration_overhead_fraction
                .is_some_and(|e| !e.is_finite() || e <= 0.0 || e > 1.0)
        {
            return Err(CoreError::InvalidInput(
                "invalid causal switching policy".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CausalSwitchReceipt {
    pub phase: &'static str,
    pub candidate: CandidateRoute,
    pub protected_work_reference: Option<f64>,
    /// Reference admission observed before this attempt. Resetting a reference
    /// changes routing only; the physical acceptance checks remain mandatory.
    pub reference_status: &'static str,
    pub reference_age_accepted_steps: Option<usize>,
    pub reference_step_ratio: Option<f64>,
    pub reference_state_distance: Option<f64>,
    pub reference_model_epoch: Option<u64>,
    pub current_model_epoch: Option<u64>,
    /// State coordinates inspected/copied by prepare, charged even when the
    /// physical attempt is rejected. Reference promotion transfers ownership
    /// and copies no coordinates. Neither is a callback count.
    pub reference_state_coordinates_inspected: usize,
    pub reference_state_coordinates_copied: usize,
    pub candidate_work_units: Option<f64>,
    pub protected_work_units: Option<f64>,
    /// False makes an overflowed work score explicit in machine-readable output.
    pub work_scores_finite: bool,
    pub cooldown_rule: &'static str,
    pub cooldown_limited_by_maximum: bool,
    /// C/((L+1)P), meaningful as a budget prediction for representative work.
    pub predicted_exploration_overhead: Option<f64>,
    pub decision: &'static str,
    pub accepted_protected_steps_until_probe: usize,
}

pub(crate) struct SwitchState {
    policy: CausalSwitchPolicy,
    baseline: Option<f64>,
    calibration: bool,
    cooldown: usize,
    next_cooldown: usize,
    reentry: bool,
    accepted_clock: usize,
    baseline_clock: usize,
    baseline_h: Option<f64>,
    baseline_state: Vec<f64>,
    baseline_epoch: Option<Option<u64>>,
    pending_h: Option<f64>,
    pending_state: Vec<f64>,
    pending_epoch: Option<u64>,
    reference_status: &'static str,
    reference_age: Option<usize>,
    reference_step_ratio: Option<f64>,
    reference_state_distance: Option<f64>,
    reference_epoch: Option<u64>,
    coordinates_inspected: usize,
    coordinates_copied: usize,
}
impl SwitchState {
    pub(crate) fn new(policy: &CausalSwitchPolicy) -> Self {
        Self {
            policy: policy.clone(),
            baseline: None,
            calibration: true,
            cooldown: 0,
            next_cooldown: policy.initial_cooldown,
            reentry: false,
            accepted_clock: 0,
            baseline_clock: 0,
            baseline_h: None,
            baseline_state: Vec::new(),
            baseline_epoch: None,
            pending_h: None,
            pending_state: Vec::new(),
            pending_epoch: None,
            reference_status: "unprepared",
            reference_age: None,
            reference_step_ratio: None,
            reference_state_distance: None,
            reference_epoch: None,
            coordinates_inspected: 0,
            coordinates_copied: 0,
        }
    }
    /// Called once before any callbacks at an attempt's accepted start state.
    pub(crate) fn prepare(&mut self, h: f64, y: &[f64], epoch: Option<u64>) {
        self.pending_h = Some(h);
        self.pending_epoch = epoch;
        self.reference_epoch = self.baseline_epoch.flatten();
        self.reference_age = self
            .baseline
            .map(|_| self.accepted_clock.saturating_sub(self.baseline_clock));
        self.reference_step_ratio = None;
        self.reference_state_distance = None;
        self.coordinates_inspected = 0;
        self.coordinates_copied = 0;
        let mut status = if self.baseline.is_some() {
            "legacy-transfer"
        } else {
            "uncalibrated"
        };
        let mut invalidate = false;
        if self.baseline.is_some() && self.baseline_epoch.is_some_and(|old| old != epoch) {
            status = "model-epoch-changed";
            invalidate = true;
        } else if self.baseline.is_some()
            && let Some(g) = &self.policy.reference_guard
        {
            status = "local-reference-admitted";
            let ratio = self.baseline_h.map(|old| (h / old).max(old / h));
            self.reference_step_ratio = ratio.filter(|x| x.is_finite());
            if ratio.is_none_or(|x| !x.is_finite() || x > g.max_step_ratio) {
                status = "step-ratio-changed";
                invalidate = true;
            } else if self
                .reference_age
                .is_some_and(|age| age >= g.max_accepted_age)
            {
                status = "reference-age-expired";
                invalidate = true;
            } else if self.baseline_state.len() != y.len() {
                status = "state-dimension-changed";
                invalidate = true;
            } else {
                let mut distance = 0.0_f64;
                for (&now, &old) in y.iter().zip(&self.baseline_state) {
                    self.coordinates_inspected += 1;
                    let scale =
                        g.state_absolute_scale + g.state_relative_scale * now.abs().max(old.abs());
                    let delta = (now - old).abs() / scale;
                    if !scale.is_finite() || !delta.is_finite() {
                        distance = f64::INFINITY;
                        break;
                    }
                    distance = distance.max(delta);
                }
                self.reference_state_distance = distance.is_finite().then_some(distance);
                if !distance.is_finite() || distance > 1.0 {
                    status = "state-displacement-changed";
                    invalidate = true;
                }
            }
        }
        if invalidate {
            self.baseline = None;
            self.baseline_h = None;
            self.baseline_epoch = None;
            self.calibration = true;
            self.cooldown = 0;
            self.next_cooldown = self.policy.initial_cooldown;
            self.reentry = false;
        }
        self.reference_status = status;
        if self.policy.reference_guard.is_some() {
            // Reuse capacity: only initial/growing dimensions can allocate.
            self.pending_state.clear();
            self.pending_state.extend_from_slice(y);
            self.coordinates_copied = y.len();
        }
    }
    pub(crate) fn skip(&self) -> bool {
        self.calibration || self.cooldown > 0
    }
    fn decline(&mut self) {
        self.cooldown = self.next_cooldown;
        self.next_cooldown = self
            .next_cooldown
            .saturating_mul(2)
            .min(self.policy.max_cooldown);
        self.reentry = true;
    }
    pub(crate) fn observe(&mut self, r: &CoupledTrajectoryAttempt) -> CausalSwitchReceipt {
        let phase = if self.calibration {
            "calibration"
        } else if self.cooldown > 0 {
            "cooldown"
        } else if self.reentry {
            "reentry-probe"
        } else {
            "candidate"
        };
        let reference = self.baseline;
        if r.accepted {
            self.accepted_clock = self.accepted_clock.saturating_add(1);
        }
        let candidate = self.policy.weights.candidate_score(r);
        let protected = self.policy.weights.protected_score(r);
        // Only successful protected endpoints update the moving cost estimate.
        // Saturating/overflowed scores trigger conservative selection below.
        if r.accepted && r.fallback_attempted && protected.is_finite() && protected > 0.0 {
            self.baseline = Some(
                match (self.baseline, self.policy.reference_guard.is_some()) {
                    (Some(old), false) => 0.75 * old + 0.25 * protected,
                    _ => protected,
                },
            );
            self.baseline_clock = self.accepted_clock;
            self.baseline_h = self.pending_h;
            self.baseline_epoch = Some(self.pending_epoch);
            if self.policy.reference_guard.is_some() {
                // prepare owns the exact accepted-start snapshot. Move that
                // buffer into the reference; the old reference is scratch that
                // the next prepare clears before reading. This preserves the
                // guarded routing word without a second coordinate copy.
                std::mem::swap(&mut self.baseline_state, &mut self.pending_state);
            }
        }
        let decision;
        let mut cooldown_rule = "not-updated";
        let mut cooldown_limited_by_maximum = false;
        let mut predicted_exploration_overhead = None;
        if r.terminal_failure.is_some() && !r.retryable_failure {
            decision = "terminal-failure";
        } else if r.candidate_skipped {
            if r.accepted {
                if self.calibration {
                    self.calibration = false;
                } else {
                    self.cooldown = self.cooldown.saturating_sub(1);
                }
            }
            decision = if r.accepted {
                "protected-accepted"
            } else {
                "protected-retry"
            };
        } else if r.fallback_attempted {
            self.decline();
            cooldown_rule = "exponential";
            // Prefer a successful same-attempt protected score. For a rejected
            // physical step, use the last successful reference, not incomplete
            // failed GMRES work. No answer/reference error enters this decision.
            let cadence_reference = if r.accepted && protected.is_finite() && protected > 0.0 {
                Some(protected)
            } else {
                reference.filter(|p| p.is_finite() && *p > 0.0)
            };
            if let (Some(epsilon), Some(p)) =
                (self.policy.exploration_overhead_fraction, cadence_reference)
                && candidate.is_finite()
            {
                let relative = candidate / p;
                let required = (relative / epsilon).ceil() - 1.0;
                let desired = required.max(1.0);
                cooldown_limited_by_maximum =
                    !desired.is_finite() || desired > self.policy.max_cooldown as f64;
                self.cooldown = if cooldown_limited_by_maximum {
                    self.policy.max_cooldown
                } else {
                    desired as usize
                };
                cooldown_rule = "priced-failed-exploration";
                let predicted = relative / (self.cooldown as f64 + 1.0);
                predicted_exploration_overhead = predicted.is_finite().then_some(predicted);
            }
            decision = "candidate-declined-cooldown";
        } else if !r.accepted {
            // An embedded local-error rejection is corrected by h adaptation;
            // it is not evidence that another solver avoids truncation error.
            decision = "local-error-retry-same-route";
        } else {
            let ratio = if self.reentry {
                self.policy.reentry_ratio
            } else {
                self.policy.retain_ratio
            };
            let economical =
                candidate.is_finite() && self.baseline.is_some_and(|b| candidate <= ratio * b);
            if economical {
                self.reentry = false;
                self.next_cooldown = self.policy.initial_cooldown;
                decision = "candidate-retained";
            } else {
                self.decline();
                cooldown_rule = "exponential-valid-candidate-cost";
                decision = "work-proxy-cooldown";
            }
        }
        CausalSwitchReceipt {
            phase,
            candidate: self.policy.candidate,
            protected_work_reference: reference,
            reference_status: self.reference_status,
            reference_age_accepted_steps: self.reference_age,
            reference_step_ratio: self.reference_step_ratio,
            reference_state_distance: self.reference_state_distance,
            reference_model_epoch: self.reference_epoch,
            current_model_epoch: self.pending_epoch,
            reference_state_coordinates_inspected: self.coordinates_inspected,
            reference_state_coordinates_copied: self.coordinates_copied,
            candidate_work_units: candidate.is_finite().then_some(candidate),
            protected_work_units: protected.is_finite().then_some(protected),
            work_scores_finite: candidate.is_finite() && protected.is_finite(),
            cooldown_rule,
            cooldown_limited_by_maximum,
            predicted_exploration_overhead,
            decision,
            accepted_protected_steps_until_probe: self.cooldown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodas5p_core::WorkCounters;
    fn record(
        skipped: bool,
        fallback: bool,
        accepted: bool,
        candidate_rhs: u64,
        protected_rhs: u64,
    ) -> CoupledTrajectoryAttempt {
        CoupledTrajectoryAttempt {
            t: 0.0,
            h: 0.1,
            requested_h: 0.1,
            output_clipped: false,
            accepted,
            selection: None,
            target_id: None,
            embedded_wrms: Some(if accepted { 0.5 } else { 2.0 }),
            effective_error: Some(if accepted { 0.5 } else { 2.0 }),
            full_residual_wrms: Some(0.0),
            candidate_work: Work {
                rhs_attempted: candidate_rhs,
                ..Default::default()
            },
            fallback_work: WorkCounters {
                rhs_calls: protected_rhs,
                ..Default::default()
            },
            fallback_callback_work: Work::default(),
            fallback_validation_work: Work::default(),
            fallback_attempted: fallback,
            candidate_skipped: skipped,
            switch_decision: None,
            candidate_failure: None,
            candidate_bound: None,
            selected_bound: None,
            retryable_failure: false,
            retry_failure_kind: None,
            terminal_failure: None,
        }
    }
    #[test]
    fn cooldown_is_accepted_step_clock_and_reentry_can_restore_candidate() {
        let policy = CausalSwitchPolicy {
            initial_cooldown: 2,
            max_cooldown: 8,
            ..Default::default()
        };
        let mut s = SwitchState::new(&policy);
        assert!(s.skip());
        s.observe(&record(true, true, false, 0, 100));
        assert!(s.skip()); // rejected calibration does not advance state
        s.observe(&record(true, true, true, 0, 100));
        assert!(!s.skip());
        let r = s.observe(&record(false, false, true, 120, 0));
        assert_eq!(r.decision, "candidate-retained"); // retain threshold125
        let r = s.observe(&record(false, false, true, 130, 0));
        assert_eq!(r.decision, "work-proxy-cooldown");
        assert_eq!(r.accepted_protected_steps_until_probe, 2);
        s.observe(&record(true, true, false, 0, 100));
        assert_eq!(s.cooldown, 2);
        s.observe(&record(true, true, true, 0, 100));
        s.observe(&record(true, true, true, 0, 100));
        assert!(!s.skip());
        let r = s.observe(&record(false, false, true, 110, 0));
        assert_eq!(r.phase, "reentry-probe");
        assert_eq!(r.decision, "work-proxy-cooldown"); // stricter reentry threshold100
        assert_eq!(s.cooldown, 4);
        for _ in 0..4 {
            s.observe(&record(true, true, true, 0, 100));
        }
        let r = s.observe(&record(false, false, true, 90, 0));
        assert_eq!(r.decision, "candidate-retained");
        assert!(!s.skip());
        assert_eq!(s.next_cooldown, 2);
    }
    #[test]
    fn local_error_rejection_does_not_trigger_root_switch() {
        let mut s = SwitchState::new(&CausalSwitchPolicy::default());
        s.observe(&record(true, true, true, 0, 100));
        let r = s.observe(&record(false, false, false, 200, 0));
        assert_eq!(r.decision, "local-error-retry-same-route");
        assert!(!s.skip());
        let r = s.observe(&record(false, true, false, 200, 100));
        assert_eq!(r.decision, "candidate-declined-cooldown");
        assert!(s.skip()); // failed projected root independent of outer truncation verdict
    }
    #[test]
    fn weighted_proxy_includes_validation_without_double_counting_callback_receipt() {
        let w = CausalWorkWeights {
            rhs: 2.0,
            jvp: 3.0,
            orthogonalization_dot: 4.0,
            small_factorization: 5.0,
            small_block_solve: 6.0,
        };
        let mut r = record(true, true, true, 0, 7);
        r.fallback_work.jvp_calls = 11;
        r.fallback_work.orthogonalization_inner_products = 13;
        r.fallback_work.direct_factorizations = 17;
        r.fallback_work.direct_solve_calls = 19;
        r.fallback_callback_work.rhs_attempted = 7;
        r.fallback_callback_work.jvp_attempted = 11;
        r.fallback_validation_work.rhs_attempted = 23;
        r.fallback_validation_work.jvp_attempted = 29;
        assert_eq!(
            w.protected_score(&r),
            14. + 33. + 52. + 85. + 114. + 46. + 87.
        );
    }
    #[test]
    fn cheap_failed_probes_have_bounded_cooldown_and_legacy_policy_is_preserved() {
        let mut priced = SwitchState::new(&CausalSwitchPolicy::default());
        let mut legacy = SwitchState::new(&CausalSwitchPolicy {
            exploration_overhead_fraction: None,
            ..Default::default()
        });
        for state in [&mut priced, &mut legacy] {
            state.observe(&record(true, true, true, 0, 100));
        }
        for expected_legacy in [4, 8, 16] {
            let r = priced.observe(&record(false, true, true, 10, 100));
            assert_eq!(r.cooldown_rule, "priced-failed-exploration");
            assert_eq!(priced.cooldown, 1);
            assert!(!r.cooldown_limited_by_maximum);
            assert_eq!(r.predicted_exploration_overhead, Some(0.05));
            priced.observe(&record(true, true, true, 0, 100));
            let r = legacy.observe(&record(false, true, true, 10, 100));
            assert_eq!(r.cooldown_rule, "exponential");
            assert_eq!(legacy.cooldown, expected_legacy);
            for _ in 0..expected_legacy {
                legacy.observe(&record(true, true, true, 0, 100));
            }
        }
    }
    #[test]
    fn priced_failure_cadence_reports_proxy_budget_and_maximum_clipping() {
        let mut s = SwitchState::new(&CausalSwitchPolicy::default());
        s.observe(&record(true, true, true, 0, 100));
        let r = s.observe(&record(false, true, true, 200, 100));
        assert_eq!(s.cooldown, 7);
        assert_eq!(r.predicted_exploration_overhead, Some(0.25));
        assert!(!r.cooldown_limited_by_maximum);
        let mut s = SwitchState::new(&CausalSwitchPolicy {
            max_cooldown: 4,
            ..Default::default()
        });
        s.observe(&record(true, true, true, 0, 100));
        let r = s.observe(&record(false, true, true, 200, 100));
        assert_eq!(s.cooldown, 4);
        assert!(r.cooldown_limited_by_maximum);
        assert_eq!(r.predicted_exploration_overhead, Some(0.4));
    }
    #[test]
    fn rejected_protected_trial_uses_previous_successful_cadence_reference() {
        let mut s = SwitchState::new(&CausalSwitchPolicy::default());
        s.observe(&record(true, true, true, 0, 100));
        let r = s.observe(&record(false, true, false, 10, 1));
        assert_eq!(s.cooldown, 1);
        assert_eq!(r.predicted_exploration_overhead, Some(0.05));
    }
    #[test]
    fn overflowed_work_proxy_is_explicit_and_cannot_select_a_candidate() {
        let mut policy = CausalSwitchPolicy::default();
        policy.weights.rhs = f64::MAX;
        let mut s = SwitchState::new(&policy);
        s.observe(&record(true, true, true, 0, 1));
        let r = s.observe(&record(false, true, true, 2, 2));
        assert!(!r.work_scores_finite);
        assert!(r.candidate_work_units.is_none());
        assert!(r.protected_work_units.is_none());
        assert!(s.skip());
        assert_eq!(r.cooldown_rule, "exponential");
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["work_scores_finite"], false);
        assert!(json["candidate_work_units"].is_null());
    }
    #[test]
    fn invalid_exploration_fraction_is_rejected() {
        for epsilon in [0.0, -0.25, 1.01, f64::NAN, f64::INFINITY] {
            assert!(
                CausalSwitchPolicy {
                    exploration_overhead_fraction: Some(epsilon),
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
    }
    #[test]
    fn guard_recalibrates_step_state_age_and_epoch_without_sampling_callbacks() {
        let policy = CausalSwitchPolicy {
            reference_guard: Some(CausalReferenceGuard {
                max_step_ratio: 2.0,
                state_absolute_scale: 0.1,
                state_relative_scale: 0.0,
                max_accepted_age: 2,
            }),
            ..Default::default()
        };
        let mut s = SwitchState::new(&policy);
        s.prepare(0.1, &[1.0, 2.0], Some(7));
        s.observe(&record(true, true, true, 0, 100));
        s.prepare(0.2, &[1.05, 2.0], Some(7));
        assert!(!s.skip());
        let r = s.observe(&record(false, false, true, 90, 0));
        assert_eq!(r.reference_status, "local-reference-admitted");
        assert_eq!(r.reference_state_coordinates_inspected, 2);
        assert_eq!(r.reference_state_coordinates_copied, 2);
        s.prepare(0.201, &[1.05, 2.0], Some(7));
        assert!(s.skip());
        let r = s.observe(&record(true, true, true, 0, 200));
        assert_eq!(r.reference_status, "step-ratio-changed");
        assert_eq!(r.protected_work_reference, None);
        assert_eq!(s.baseline, Some(200.0));
        s.prepare(0.201, &[1.16, 2.0], Some(7));
        assert!(s.skip());
        assert_eq!(s.reference_status, "state-displacement-changed");
        s.observe(&record(true, true, true, 0, 70));
        s.prepare(0.201, &[1.16, 2.0], Some(8));
        assert!(s.skip());
        assert_eq!(s.reference_status, "model-epoch-changed");
        s.observe(&record(true, true, true, 0, 70));
        for _ in 0..2 {
            s.prepare(0.201, &[1.16, 2.0], Some(8));
            s.observe(&record(false, false, true, 20, 0));
        }
        s.prepare(0.201, &[1.16, 2.0], Some(8));
        assert!(s.skip());
        assert_eq!(s.reference_status, "reference-age-expired");
    }

    #[test]
    fn rejected_calibration_does_not_create_a_cost_reference() {
        let mut s = SwitchState::new(&CausalSwitchPolicy {
            reference_guard: Some(CausalReferenceGuard::default()),
            ..Default::default()
        });
        s.prepare(0.1, &[1.0], Some(1));
        s.observe(&record(true, true, false, 0, 100));
        assert!(s.baseline.is_none());
        assert!(s.skip());
        s.prepare(0.01, &[1.0], Some(1));
        let r = s.observe(&record(true, true, true, 0, 80));
        assert_eq!(r.reference_status, "uncalibrated");
        assert_eq!(s.baseline_h, Some(0.01));
        assert_eq!(s.baseline, Some(80.0));
    }

    #[test]
    fn epoch_invalidation_also_applies_without_locality_guard() {
        let mut s = SwitchState::new(&CausalSwitchPolicy::default());
        s.prepare(0.1, &[1.0], Some(4));
        s.observe(&record(true, true, true, 0, 100));
        s.prepare(100.0, &[100.0], Some(4));
        assert!(!s.skip()); // legacy h/state transfer explicitly retained
        assert_eq!(s.reference_status, "legacy-transfer");
        s.prepare(100.0, &[100.0], Some(5));
        assert!(s.skip());
        assert_eq!(s.reference_status, "model-epoch-changed");
        assert!(s.baseline.is_none());
    }

    #[test]
    fn guarded_reference_is_fresh_and_nonfinite_state_scale_declines_reuse() {
        let mut s = SwitchState::new(&CausalSwitchPolicy {
            reference_guard: Some(CausalReferenceGuard {
                state_absolute_scale: 1.0,
                state_relative_scale: f64::MAX,
                ..Default::default()
            }),
            ..Default::default()
        });
        s.prepare(0.1, &[2.0], None);
        s.observe(&record(true, true, true, 0, 100));
        s.prepare(0.1, &[2.0], None);
        assert!(s.skip());
        let r = s.observe(&record(true, true, true, 0, 200));
        assert_eq!(r.reference_status, "state-displacement-changed");
        assert!(r.reference_state_distance.is_none());
        assert_eq!(s.baseline, Some(200.0));
    }

    #[test]
    fn invalid_guard_and_measured_costs_are_rejected() {
        for g in [
            CausalReferenceGuard {
                max_step_ratio: 0.9,
                ..Default::default()
            },
            CausalReferenceGuard {
                state_absolute_scale: 0.0,
                ..Default::default()
            },
            CausalReferenceGuard {
                state_relative_scale: f64::NAN,
                ..Default::default()
            },
            CausalReferenceGuard {
                max_accepted_age: 0,
                ..Default::default()
            },
        ] {
            assert!(
                CausalSwitchPolicy {
                    reference_guard: Some(g),
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
        assert!(CausalWorkWeights::from_measured_serial_means([1e-6, 2e-6, 0.0, 0.0, 0.0]).is_ok());
        for bad in [
            [0.0, 1.0, 0.0, 0.0, 0.0],
            [1.0, f64::INFINITY, 0.0, 0.0, 0.0],
            [1.0, 1.0, -1.0, 0.0, 0.0],
        ] {
            assert!(CausalWorkWeights::from_measured_serial_means(bad).is_err());
        }
    }

    /// Frozen 87465835 observe implementation: a native historical comparator.
    /// prepare/skip are shared because this patch leaves them unchanged.
    struct LegacySwitchState(SwitchState);
    impl LegacySwitchState {
        fn observe(&mut self, r: &CoupledTrajectoryAttempt) -> CausalSwitchReceipt {
            let phase = if self.0.calibration {
                "calibration"
            } else if self.0.cooldown > 0 {
                "cooldown"
            } else if self.0.reentry {
                "reentry-probe"
            } else {
                "candidate"
            };
            let reference = self.0.baseline;
            if r.accepted {
                self.0.accepted_clock = self.0.accepted_clock.saturating_add(1);
            }
            let candidate = self.0.policy.weights.candidate_score(r);
            let protected = self.0.policy.weights.protected_score(r);
            // Only successful protected endpoints update the moving cost estimate.
            // Saturating/overflowed scores trigger conservative selection below.
            if r.accepted && r.fallback_attempted && protected.is_finite() && protected > 0.0 {
                self.0.baseline = Some(
                    match (self.0.baseline, self.0.policy.reference_guard.is_some()) {
                        (Some(old), false) => 0.75 * old + 0.25 * protected,
                        _ => protected,
                    },
                );
                self.0.baseline_clock = self.0.accepted_clock;
                self.0.baseline_h = self.0.pending_h;
                self.0.baseline_epoch = Some(self.0.pending_epoch);
                if self.0.policy.reference_guard.is_some() {
                    self.0.baseline_state.clear();
                    self.0
                        .baseline_state
                        .extend_from_slice(&self.0.pending_state);
                    self.0.coordinates_copied = self
                        .0
                        .coordinates_copied
                        .saturating_add(self.0.pending_state.len());
                }
            }
            let decision;
            let mut cooldown_rule = "not-updated";
            let mut cooldown_limited_by_maximum = false;
            let mut predicted_exploration_overhead = None;
            if r.terminal_failure.is_some() && !r.retryable_failure {
                decision = "terminal-failure";
            } else if r.candidate_skipped {
                if r.accepted {
                    if self.0.calibration {
                        self.0.calibration = false;
                    } else {
                        self.0.cooldown = self.0.cooldown.saturating_sub(1);
                    }
                }
                decision = if r.accepted {
                    "protected-accepted"
                } else {
                    "protected-retry"
                };
            } else if r.fallback_attempted {
                self.0.decline();
                cooldown_rule = "exponential";
                // Prefer a successful same-attempt protected score. For a rejected
                // physical step, use the last successful reference, not incomplete
                // failed GMRES work. No answer/reference error enters this decision.
                let cadence_reference = if r.accepted && protected.is_finite() && protected > 0.0 {
                    Some(protected)
                } else {
                    reference.filter(|p| p.is_finite() && *p > 0.0)
                };
                if let (Some(epsilon), Some(p)) = (
                    self.0.policy.exploration_overhead_fraction,
                    cadence_reference,
                ) {
                    if candidate.is_finite() {
                        let relative = candidate / p;
                        let required = (relative / epsilon).ceil() - 1.0;
                        let desired = required.max(1.0);
                        cooldown_limited_by_maximum =
                            !desired.is_finite() || desired > self.0.policy.max_cooldown as f64;
                        self.0.cooldown = if cooldown_limited_by_maximum {
                            self.0.policy.max_cooldown
                        } else {
                            desired as usize
                        };
                        cooldown_rule = "priced-failed-exploration";
                        let predicted = relative / (self.0.cooldown as f64 + 1.0);
                        predicted_exploration_overhead = predicted.is_finite().then_some(predicted);
                    }
                }
                decision = "candidate-declined-cooldown";
            } else if !r.accepted {
                // An embedded local-error rejection is corrected by h adaptation;
                // it is not evidence that another solver avoids truncation error.
                decision = "local-error-retry-same-route";
            } else {
                let ratio = if self.0.reentry {
                    self.0.policy.reentry_ratio
                } else {
                    self.0.policy.retain_ratio
                };
                let economical = candidate.is_finite()
                    && self.0.baseline.is_some_and(|b| candidate <= ratio * b);
                if economical {
                    self.0.reentry = false;
                    self.0.next_cooldown = self.0.policy.initial_cooldown;
                    decision = "candidate-retained";
                } else {
                    self.0.decline();
                    cooldown_rule = "exponential-valid-candidate-cost";
                    decision = "work-proxy-cooldown";
                }
            }
            CausalSwitchReceipt {
                phase,
                candidate: self.0.policy.candidate,
                protected_work_reference: reference,
                reference_status: self.0.reference_status,
                reference_age_accepted_steps: self.0.reference_age,
                reference_step_ratio: self.0.reference_step_ratio,
                reference_state_distance: self.0.reference_state_distance,
                reference_model_epoch: self.0.reference_epoch,
                current_model_epoch: self.0.pending_epoch,
                reference_state_coordinates_inspected: self.0.coordinates_inspected,
                reference_state_coordinates_copied: self.0.coordinates_copied,
                candidate_work_units: candidate.is_finite().then_some(candidate),
                protected_work_units: protected.is_finite().then_some(protected),
                work_scores_finite: candidate.is_finite() && protected.is_finite(),
                cooldown_rule,
                cooldown_limited_by_maximum,
                predicted_exploration_overhead,
                decision,
                accepted_protected_steps_until_probe: self.0.cooldown,
            }
        }
    }

    fn bits(values: &[f64]) -> Vec<u64> {
        values.iter().map(|x| x.to_bits()).collect()
    }

    #[test]
    fn guarded_symbolic_words_preserve_legacy_routing_and_snapshot_bits() {
        let policy = CausalSwitchPolicy {
            initial_cooldown: 1,
            max_cooldown: 4,
            reference_guard: Some(CausalReferenceGuard {
                max_step_ratio: 2.0,
                state_absolute_scale: 0.25,
                state_relative_scale: 0.0,
                max_accepted_age: 3,
            }),
            ..Default::default()
        };
        // Six outcome symbols, all words of length five. Guard perturbations
        // are applied in a second pass; terminal symbols end the legal word.
        for perturbed in [false, true] {
            for encoded in 0_u32..6_u32.pow(5) {
                let mut current = SwitchState::new(&policy);
                let mut legacy = LegacySwitchState(SwitchState::new(&policy));
                let mut word = encoded;
                let mut epoch = 0;
                for depth in 0..5 {
                    let symbol = word % 6;
                    word /= 6;
                    if perturbed && symbol == 2 {
                        epoch += 1;
                    }
                    let dimension = if perturbed && symbol == 4 { 7 } else { 2 };
                    let h = if perturbed && symbol == 3 { 0.8 } else { 0.1 };
                    let mut y = vec![0.0; dimension];
                    y[0] = if depth % 2 == 0 { -0.0 } else { 0.0 };
                    y[1] = if perturbed && symbol == 1 { 2.0 } else { 0.125 };
                    current.prepare(h, &y, Some(epoch));
                    legacy.0.prepare(h, &y, Some(epoch));
                    assert_eq!(current.skip(), legacy.0.skip());
                    assert_eq!(bits(&current.pending_state), bits(&legacy.0.pending_state));
                    let skipped = current.skip();
                    let accepted = matches!(symbol, 0 | 2 | 4);
                    let fallback = skipped || matches!(symbol, 2..=5);
                    let mut input = record(
                        skipped,
                        fallback,
                        accepted,
                        if skipped { 0 } else { 20 },
                        if fallback { 100 } else { 0 },
                    );
                    if symbol == 5 {
                        input.terminal_failure = Some(crate::Failure::new(
                            crate::FailureKind::Callback,
                            "symbolic-word terminal input",
                        ));
                    } else if !accepted && fallback {
                        input.retryable_failure = true;
                    }
                    let got = current.observe(&input);
                    let expected = legacy.observe(&input);
                    assert_eq!(got.reference_state_coordinates_copied, dimension);
                    assert_eq!(
                        expected.reference_state_coordinates_copied,
                        dimension * if accepted && fallback { 2 } else { 1 }
                    );
                    let mut got_json = serde_json::to_value(&got).unwrap();
                    let mut expected_json = serde_json::to_value(&expected).unwrap();
                    got_json
                        .as_object_mut()
                        .unwrap()
                        .remove("reference_state_coordinates_copied");
                    expected_json
                        .as_object_mut()
                        .unwrap()
                        .remove("reference_state_coordinates_copied");
                    assert_eq!(got_json, expected_json, "word={encoded}, depth={depth}");
                    assert_eq!(
                        bits(&current.baseline_state),
                        bits(&legacy.0.baseline_state)
                    );
                    assert_eq!(current.skip(), legacy.0.skip());
                    if symbol == 5 {
                        break;
                    }
                }
            }
        }
    }

    #[test]
    fn promoted_snapshot_survives_scratch_reuse_and_rejected_attempts() {
        let policy = CausalSwitchPolicy {
            reference_guard: Some(CausalReferenceGuard::default()),
            ..Default::default()
        };
        let mut state = SwitchState::new(&policy);
        state.prepare(0.1, &[-0.0, 0.125], Some(7));
        let first = state.observe(&record(true, true, true, 0, 100));
        assert_eq!(first.reference_state_coordinates_copied, 2);
        assert_eq!(bits(&state.baseline_state), bits(&[-0.0, 0.125]));
        let saved = bits(&state.baseline_state);
        state.prepare(0.1, &[0.0, 0.125], Some(7));
        state.observe(&record(false, false, false, 20, 0));
        assert_eq!(bits(&state.baseline_state), saved);
        state.prepare(0.1, &[0.0, 0.125], Some(7));
        state.observe(&record(false, false, true, 20, 0));
        assert_eq!(bits(&state.baseline_state), saved);
        state.prepare(0.1, &[0.0, 0.125], Some(8));
        assert!(state.skip());
        state.observe(&record(true, true, false, 0, 100));
        assert!(state.baseline.is_none());
        state.prepare(0.1, &[-0.0, 0.125, 0.25], Some(8));
        let grown = state.observe(&record(true, true, true, 0, 100));
        assert_eq!(grown.reference_state_coordinates_copied, 3);
        assert_eq!(bits(&state.baseline_state), bits(&[-0.0, 0.125, 0.25]));
    }

    #[test]
    #[ignore = "registered native paired benchmark; set VIGILODE_SWITCH_BENCH_OUT"]
    fn benchmark_snapshot_transfer_against_legacy() {
        use std::{hint::black_box, time::Instant};
        fn run(
            dimension: usize,
            iterations: usize,
            scenario: &str,
            legacy: bool,
        ) -> serde_json::Value {
            let policy = CausalSwitchPolicy {
                reference_guard: (scenario != "unguarded_protected").then(|| {
                    CausalReferenceGuard {
                        max_accepted_age: usize::MAX,
                        ..Default::default()
                    }
                }),
                ..Default::default()
            };
            let y = vec![0.125; dimension];
            let begin = Instant::now();
            // Both allocations of the reference buffers occur inside timing.
            let mut current = SwitchState::new(&policy);
            let mut historical = LegacySwitchState(SwitchState::new(&policy));
            let mut copied = 0_u64;
            let mut inspected = 0_u64;
            let mut protected_updates = 0_u64;
            let mut decisions = Vec::with_capacity(iterations);
            for index in 0..iterations {
                let state = if legacy {
                    &mut historical.0
                } else {
                    &mut current
                };
                state.prepare(0.1, black_box(&y), Some(1));
                let skipped = state.skip();
                let fallback = scenario != "candidate_only" || index == 0;
                let input = record(
                    skipped,
                    fallback,
                    true,
                    if skipped { 0 } else { 20 },
                    if fallback { 100 } else { 0 },
                );
                let output = if legacy {
                    historical.observe(&input)
                } else {
                    current.observe(&input)
                };
                copied += output.reference_state_coordinates_copied as u64;
                inspected += output.reference_state_coordinates_inspected as u64;
                protected_updates += u64::from(fallback);
                decisions.push((
                    output.phase,
                    output.decision,
                    output.accepted_protected_steps_until_probe,
                ));
                black_box(&output);
            }
            let elapsed = begin.elapsed().as_secs_f64();
            let state = if legacy { &historical.0 } else { &current };
            serde_json::json!({
                "seconds": elapsed,
                "copied_coordinates": copied,
                "inspected_coordinates": inspected,
                "protected_updates": protected_updates,
                "final_snapshot_bits": bits(&state.baseline_state),
                "routing_word": decisions,
            })
        }
        let mut rows = Vec::new();
        let mut rng = 0x51a7_2026_1028_u64;
        for (dimension, iterations) in [(2, 10000), (32, 5000), (4096, 256), (65536, 32)] {
            for scenario in ["guarded_protected", "candidate_only", "unguarded_protected"] {
                // One preregistered warmup per arm/case, excluded from rows.
                // Each measured arm still starts from empty owned buffers.
                black_box(run(dimension, iterations, scenario, true));
                black_box(run(dimension, iterations, scenario, false));
                for block in 0..7 {
                    rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let legacy_first = (rng >> 32) & 1 == 0;
                    let (mut old, mut new) = if legacy_first {
                        let old = run(dimension, iterations, scenario, true);
                        (old, run(dimension, iterations, scenario, false))
                    } else {
                        let new = run(dimension, iterations, scenario, false);
                        (run(dimension, iterations, scenario, true), new)
                    };
                    assert_eq!(old["routing_word"], new["routing_word"]);
                    assert_eq!(old["final_snapshot_bits"], new["final_snapshot_bits"]);
                    assert_eq!(old["inspected_coordinates"], new["inspected_coordinates"]);
                    let saved = if scenario == "unguarded_protected" {
                        0
                    } else {
                        dimension as u64 * new["protected_updates"].as_u64().unwrap()
                    };
                    assert_eq!(
                        old["copied_coordinates"].as_u64().unwrap()
                            - new["copied_coordinates"].as_u64().unwrap(),
                        saved
                    );
                    // Full words are checked in memory; raw timing/count rows
                    // retain sizes and parity without megabytes of repeated words.
                    for result in [&mut old, &mut new] {
                        result.as_object_mut().unwrap().remove("routing_word");
                        result
                            .as_object_mut()
                            .unwrap()
                            .remove("final_snapshot_bits");
                    }
                    rows.push(serde_json::json!({
                        "dimension": dimension, "iterations": iterations,
                        "scenario": scenario, "block": block,
                        "order": if legacy_first { "legacy,new" } else { "new,legacy" },
                        "legacy": old, "new": new,
                        "saved_coordinate_copies": saved,
                        "routing_and_snapshot_bit_parity": true
                    }));
                }
            }
        }
        let output = std::env::var("VIGILODE_SWITCH_BENCH_OUT")
            .expect("explicit benchmark output path required");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .unwrap();
        serde_json::to_writer_pretty(file, &serde_json::json!({
            "scope": "SwitchState prepare plus observe with synthetic work receipts and no RHS/ODE solve, includes buffer allocation; not an end-to-end solver speedup",
            "baseline": "87465835d9d21298d235917f036580b0b53ab645 switching.rs::observe",
            "blocks": 7, "order_seed": "0x51a720261028",
            "warmup_per_arm_per_case": 1, "total_warmup_runs": 24,
            "rows": rows
        })).unwrap();
    }
}
