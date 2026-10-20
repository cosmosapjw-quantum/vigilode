//! Error types of the facade: what a user callback may report, and why a
//! solve was refused or failed.

use crate::solution::{FailureReason, Solution};

/// The error a user callback (right-hand side, Jacobian, JVP or time
/// derivative) returns.
///
/// The two variants differ in what the solver does next:
///
/// * [`CallbackError::Recoverable`] rejects the current step attempt, and
///   the step-size controller retries with a smaller step (for example a
///   right-hand side that is undefined outside a domain the large step left).
///   The driver counts it as a non-finite step failure
///   (`WorkCounters::nonfinite_step_failures`); the facade also counts it in
///   [`crate::CallbackWork::recoverable_errors`].
/// * [`CallbackError::Fatal`] ends the run. The facade records the first
///   fatal error and from then on returns an error to the driver without
///   entering user code, so the driver stops at its minimum step; the solve
///   returns the partial solution with
///   [`FailureReason::CallbackFailed`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CallbackError {
    /// The step attempt fails; a smaller step may succeed.
    #[error("recoverable callback error: {0}")]
    Recoverable(String),
    /// The run cannot continue.
    #[error("fatal callback error: {0}")]
    Fatal(String),
}

/// Why [`crate::Rodas5p::solve`] refused a problem, or (through
/// [`Solution::ensure_success`]) why a run did not reach its end.
#[derive(Debug, thiserror::Error)]
pub enum SolveError {
    /// The problem, span, initial state or options are malformed; nothing
    /// was integrated.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// The request is well formed but no protected driver of this release
    /// implements it (for example a vector `atol`); nothing was integrated.
    /// The facade refuses rather than substituting a different behaviour.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// A declared structure was refused before the first step (for example a
    /// band that failed verification at `(t0, y0)`).
    #[error("structure refused: {0}")]
    StructureRefused(String),
    /// The underlying driver returned an error instead of a (partial)
    /// result, so no partial solution exists.
    #[error("driver error: {0}")]
    Driver(String),
    /// The run ended before `t1`; the partial solution and the typed reason
    /// are kept. Only [`Solution::ensure_success`] produces this variant:
    /// [`crate::Rodas5p::solve`] itself returns a failed run as `Ok` with
    /// [`crate::Status::Failed`].
    #[error("integration failed: {reason}")]
    Failed {
        /// Why the run stopped.
        reason: FailureReason,
        /// What was computed before it stopped, with its full stats.
        partial: Box<Solution>,
    },
}
