//! # One call over three entry points
//!
//! [`VigilODE::solve`] picks one of three existing entry points of this crate, runs it and
//! reports what ran. It has no integrator of its own.
//!
//! | [`SelectedTier`] | Entry point | Driver that runs |
//! | --- | --- | --- |
//! | `Tier1Explicit` | `Dopri5::solve`, or its persistent session on the break-even route | Dormand-Prince 5(4), right-hand side only |
//! | `TierProtectedReference` | `Rodas5p::solve` | the RODAS5P driver family that the facade selects from the declared structure: dense, banded, matrix-free (GMRES), fast dense with a continuous extension, or sequential dense (mass matrix) |
//! | `Tier5AdaptiveSwitching` | `SwitchingSolver::solve` | windows of DOPRI5 and RODAS5P |
//!
//! What the module does:
//!
//! * **Routing by declared facts.** [`PreflightAnalyzer::analyze`] reads the strategy, the
//!   declared regime ([`VigilodeOptions::regime`]) and the declared structure of the
//!   problem, and makes no callback. The method of a regime, the admission of the explicit
//!   method and the planned RODAS5P driver family come from the method registry
//!   (`MethodRegistry`). The logarithmic norm is not computed and contractivity is not
//!   taken for non-stiffness. A supplied Jacobian, Jacobian-vector product or band starts
//!   RODAS5P.
//! * **Forward break-even pilots** for a request that declares nothing, of any dimension
//!   ([`RouteBasis::BreakEven`]): DOPRI5 integrates; when it has made `b 2^k` attempts
//!   ([`break_even_length`] `b`), one RODAS5P attempt from its last accepted state asks
//!   whether RODAS5P covers in one attempt what DOPRI5 covered in its last `b`. An accepted
//!   pilot is the first step of a RODAS5P leg to the end time; a lost one leaves DOPRI5
//!   bit for bit unchanged. Every pilot is charged and recorded ([`PilotRecord`]). After a
//!   numerical DOPRI5 stop, RODAS5P continues from its last accepted state
//!   ([`FallbackEngine`]). Legs and pilots share one attempt budget, `Options::max_steps`.
//! * **The executed route.** Every executed leg is a [`UnifiedLeg`] whose driver
//!   identifier is read from the leg's own run record after it ran;
//!   [`VigilodeSolution::route_history`] carries the same identifiers.
//! * **Typed stops.** A run that stops early is `Ok` with its accepted prefix, the work of
//!   every leg and pilot, and [`VigilodeSolution::stop`]: the reason the driver that
//!   stopped gave ([`UnifiedStop`]). A fatal callback inside a pilot ends the solve.
//!
//! What it does not do: it does not batch or vectorize the eight RODAS5P stages, it has no
//! exponential integrator, it attaches no Krylov recycling of its own (the matrix-free
//! route is the one of `Rodas5p::solve`, unchanged), it does not monitor invariants during
//! a run (it reports their end-point drift), and it passes no workspace to a driver: each
//! driver allocates its own storage, a dense matrix included when its route needs one.
//! Finite-difference Jacobians are taken without a separate permission, as `Rodas5p::solve`
//! takes them for a problem without a Jacobian; they are counted in
//! `Stats::finite_difference` and in the counters.
//!
//! Three options are stored and read by nothing: [`VigilodeOptions::profile`],
//! [`VigilodeOptions::max_krylov_dim`] with [`VigilodeOptions::max_krylov_restarts`], and
//! [`VigilodeOptions::invariant_drift_tol`]. [`SolverStrategy::HighPerformance`] routes
//! exactly as [`SolverStrategy::Auto`].

mod analyzer;
mod breakeven;
mod fallback;
mod legs;
mod options;
mod router;
mod solution;
mod workspace;

pub use analyzer::{PreflightAnalysis, PreflightAnalyzer, RouteBasis, SelectedTier};
pub use breakeven::{PilotRecord, PilotVerdict, break_even_length};
pub use fallback::FallbackEngine;
pub use legs::{CallbackStop, UnifiedLeg, UnifiedStop};
pub use options::{PerformanceProfile, SolverStrategy, VigilodeOptions};
pub use router::VigilodeRouter;
pub use solution::{RouteTransitionRecord, VigilodeSolution};
pub use workspace::VigilodeWorkspace;

use crate::error::SolveError;
use crate::problem::Problem;

/// One call over `Dopri5::solve`, `Rodas5p::solve` and `SwitchingSolver::solve`; see the
/// [module documentation](self) for what it does and what it does not do.
#[derive(Clone, Copy, Debug, Default)]
pub struct VigilODE;

impl VigilODE {
    /// Solve `y' = f(t, y)` (or `M y' = f(t, y)`) over `span` from `y0`.
    ///
    /// The entry point is chosen by [`PreflightAnalyzer::analyze`]. `Err` means that
    /// nothing was integrated; a run that started and stopped early is `Ok` and carries
    /// its accepted prefix, its work and its typed stop.
    pub fn solve(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        opts: impl Into<VigilodeOptions>,
    ) -> Result<VigilodeSolution, SolveError> {
        let opts = opts.into();
        let mut ws = VigilodeWorkspace::new(problem.dimension(), opts.max_krylov_dim);
        VigilodeRouter::solve_with_workspace(problem, span, y0, &opts, &mut ws)
    }

    /// [`VigilODE::solve`] with a caller-provided [`VigilodeWorkspace`]: the checkpoint
    /// vector and the counters are reused across solves. The drivers still allocate their
    /// own storage on every solve.
    pub fn solve_with_workspace(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        opts: impl Into<VigilodeOptions>,
        ws: &mut VigilodeWorkspace,
    ) -> Result<VigilodeSolution, SolveError> {
        let opts = opts.into();
        VigilodeRouter::solve_with_workspace(problem, span, y0, &opts, ws)
    }
}
