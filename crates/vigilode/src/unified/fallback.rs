//! The fallback rule of the router: when a second leg starts, and with which budget.
//!
//! A fallback leg is `Rodas5p::solve` from the last accepted state of the leg that
//! stopped. It is not a transaction and nothing is rolled back: the accepted prefix of the
//! stopped leg is kept, and so is its work. The legs share one attempt budget,
//! `Options::max_steps`.

use rodas5p_integrators::Dopri5StopReason;

/// Largest attempt budget of an explicit trial: a problem that DOPRI5 does not finish in
/// this many attempts continues on RODAS5P.
pub const EXPLICIT_TRIAL_ATTEMPT_LIMIT: usize = 1000;

/// The fallback rule. Three pure functions; the router calls them and nothing else decides
/// a fallback.
#[derive(Clone, Copy, Debug, Default)]
pub struct FallbackEngine;

impl FallbackEngine {
    /// Attempt budget of an explicit trial under a total budget of `max_steps`:
    /// `min(EXPLICIT_TRIAL_ATTEMPT_LIMIT, max_steps / 2)`, so that at least half of the
    /// total is left for the leg that follows. Zero means that no trial is made.
    pub fn explicit_trial_attempts(max_steps: usize) -> usize {
        EXPLICIT_TRIAL_ATTEMPT_LIMIT.min(max_steps / 2)
    }

    /// Attempts left for a further leg after `used` attempts of a total of `max_steps`.
    pub fn remaining_attempts(max_steps: usize, used: usize) -> usize {
        max_steps.saturating_sub(used)
    }

    /// Whether RODAS5P may continue after DOPRI5 stopped for `reason`.
    ///
    /// Only a numerical stop qualifies: the attempt budget, a step-size underflow, a
    /// non-finite attempt or a controller failure. A callback that failed fatally or
    /// returned a malformed output is not called again by another method, and a stop
    /// without a reason or one of the output schedule is left as it is.
    pub fn follows_explicit_stop(reason: Option<Dopri5StopReason>, callback_failed: bool) -> bool {
        !callback_failed
            && matches!(
                reason,
                Some(
                    Dopri5StopReason::AttemptBudget
                        | Dopri5StopReason::StepUnderflow
                        | Dopri5StopReason::NonFinite
                        | Dopri5StopReason::Controller
                )
            )
    }
}
