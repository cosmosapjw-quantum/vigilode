//! A client-owned stage linear solver for the fast RODAS5P driver (library
//! step L2): a client, for example a structured block solver, replaces the
//! Jacobian build, the factorization of `W` and the stage solves of
//! [`crate::integrate_rodas5p_fast_client_observed_with_atol`], while the
//! driver keeps its stages, controller, counters and output rules.
//! [`crate::BandedStageLu`] is the reference client: it reproduces the
//! banded driver bit for bit.

use rodas5p_core::{CoreResult, WorkCounters};

use crate::OdeProblem;

/// The stage linear algebra of the fast driver, owned by a client.
///
/// Per attempt the driver calls [`Self::linearize`] once when the state is
/// fresh (the first attempt from an accepted state, or the retry after an
/// attempt that failed with an error; the retry after a local-error
/// rejection reuses the linearization), then [`Self::factor`] once, then
/// [`Self::solve_in_place`] once per stage. None of the three should
/// allocate: they run inside the step loop.
///
/// Counters: the driver charges `jacobian_builds` before each
/// [`Self::linearize`] and `linear_solves` before each
/// [`Self::solve_in_place`]; the client charges its own work
/// (`direct_factorizations`, `direct_solve_calls`, `linear_iterations`,
/// `preconditioner_apps`, ... as appropriate). The driver never resets the
/// counters, so the work of a failed call stays charged.
///
/// Errors: [`rodas5p_core::CoreError::LinearSolve`],
/// [`rodas5p_core::CoreError::NonlinearSolve`] and
/// [`rodas5p_core::CoreError::NonFinite`] reject the attempt (the driver
/// books the failure kind, linearizes the accepted state again and retries
/// with the controller's smaller step); every other kind is fatal and ends
/// the run with that error. A solution with a NaN or infinite entry is a
/// `LinearSolve` rejection as well: the driver checks every solve.
pub trait StageLinearSystem: Send {
    /// The order of `W`; the driver requires the problem's dimension.
    fn dimension(&self) -> usize;

    /// Linearize at `(t, y)` on a fresh state (the driver charges
    /// `jacobian_builds` before the call).
    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()>;

    /// Factor or prepare `W = I/(h gamma) - J` for
    /// `inv_h_gamma = 1/(h gamma)` (the driver's unscaled convention),
    /// charging its own factorization counters.
    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()>;

    /// Solve `W x = b` in place (`b` has [`Self::dimension`] entries). The
    /// driver charges `linear_solves`; the client charges
    /// `direct_solve_calls`, `linear_iterations` or `preconditioner_apps`
    /// as appropriate.
    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()>;
}
