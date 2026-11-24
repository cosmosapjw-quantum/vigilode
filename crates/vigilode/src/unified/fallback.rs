//! The fallback rule of the router: when a RODAS5P leg follows a DOPRI5 leg that stopped,
//! and with which budget.
//!
//! A fallback leg is `Rodas5p::solve` from the last accepted state of the leg that
//! stopped. It is not a transaction and nothing is rolled back: the accepted prefix of the
//! stopped leg is kept, and so is its work. Every leg and every pilot of the break-even
//! route is charged to one attempt budget, `Options::max_steps`.

use rodas5p_integrators::Dopri5StopReason;

/// The fallback rule. Two pure functions; the router calls them and nothing else decides
/// a fallback. (A handover after an accepted pilot is the break-even route's own rule.)
#[derive(Clone, Copy, Debug, Default)]
pub struct FallbackEngine;

impl FallbackEngine {
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
