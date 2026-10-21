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
#[non_exhaustive]
pub enum CallbackError {
    /// The step attempt fails; a smaller step may succeed.
    #[error("recoverable callback error: {0}")]
    Recoverable(String),
    /// The run cannot continue.
    #[error("fatal callback error: {0}")]
    Fatal(String),
}

/// A request that no protected driver of this release implements, one
/// variant per refusal site of [`crate::Rodas5p::solve`]. `Display` is the
/// message the refusal carried as a `String` before it was typed.
///
/// New refusals are added as variants, so match with a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Unsupported {
    /// A mass matrix or dense output on a JVP-only problem.
    #[error(
        "a mass matrix or dense output on a JVP-only problem: the protected sequential \
         dense driver needs an explicit or finite-difference Jacobian (the step's stage \
         matrix M - h gamma J)"
    )]
    JvpWithMassOrDenseOutput,
    /// A declared band on a [`crate::Jacobian::Dense`] provider together with
    /// a mass matrix or dense output.
    #[error(
        "a declared band on a dense Jacobian with a mass matrix or dense output: only the \
         router verifies such a declaration, and the sequential dense driver would ignore it"
    )]
    DeclaredBandOnDenseWithMassOrDenseOutput,
    /// A vector [`crate::Atol`] on the sequential dense driver (a mass
    /// matrix, or dense output under [`crate::InteriorError::Enforce`]).
    #[error(
        "a vector atol with a mass matrix or interior_error = Enforce: the \
         sequential dense driver takes a scalar absolute tolerance"
    )]
    VectorAtolOnSequentialRoute,
    /// A vector [`crate::Atol`] on a JVP-only (matrix-free) problem.
    #[error(
        "a vector atol on a JVP-only problem: the matrix-free driver's Krylov \
         weights take a scalar absolute tolerance"
    )]
    VectorAtolOnMatrixFreeRoute,
    /// A vector [`crate::Atol`] on a dense Jacobian with a declared band.
    #[error(
        "a vector atol on a dense Jacobian with a declared band: the router that \
         verifies the declaration takes a scalar absolute tolerance"
    )]
    VectorAtolWithDeclaredBandOnDense,
    /// Events on a run without [`crate::Options::dense_output`].
    #[error(
        "events without dense output: events are located on the continuous extension of \
         the accepted steps"
    )]
    EventsWithoutDenseOutput,
    /// A band declaration on a JVP-only problem.
    #[error("a band declaration on a JVP-only problem")]
    BandDeclarationOnJvpProblem,
}

/// Why [`crate::Rodas5p::solve`] refused a problem, or (through
/// [`Solution::ensure_success`]) why a run did not reach its end.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SolveError {
    /// The problem, span, initial state or options are malformed; nothing
    /// was integrated.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// The request is well formed but no protected driver of this release
    /// implements it (for example a vector `atol`); nothing was integrated.
    /// The facade refuses rather than substituting a different behaviour.
    #[error("unsupported: {0}")]
    Unsupported(Unsupported),
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
