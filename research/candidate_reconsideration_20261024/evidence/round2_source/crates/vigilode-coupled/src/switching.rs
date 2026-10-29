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
    ResidualFirst,
}
impl CandidateRoute {
    pub(crate) fn schedule(self) -> Option<CausalSchedule> {
        match self {
            Self::Forward => Some(CausalSchedule::Forward),
            Self::Picard => Some(CausalSchedule::BatchedPicard),
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
        }
    }
}
impl CausalSwitchPolicy {
    pub(crate) fn validate(&self) -> CoreResult<()> {
        let w = &self.weights;
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
    pub candidate_work_units: f64,
    pub protected_work_units: f64,
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
        let candidate = self.policy.weights.candidate_score(r);
        let protected = self.policy.weights.protected_score(r);
        // Only successful protected endpoints update the moving cost estimate.
        // Saturating/overflowed scores trigger conservative selection below.
        if r.accepted && r.fallback_attempted && protected.is_finite() && protected > 0.0 {
            self.baseline = Some(match self.baseline {
                Some(old) => 0.75 * old + 0.25 * protected,
                None => protected,
            });
        }
        let decision;
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
                decision = "work-proxy-cooldown";
            }
        }
        CausalSwitchReceipt {
            phase,
            candidate: self.policy.candidate,
            protected_work_reference: reference,
            candidate_work_units: candidate,
            protected_work_units: protected,
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
}
