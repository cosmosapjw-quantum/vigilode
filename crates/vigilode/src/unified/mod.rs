//! # Unified solve and registered method selection
//!
//! [`VigilODE::solve_method`] selects and executes any admitted registry method in one
//! call. [`crate::MethodChoice::Auto`] applies the caller's capabilities, qualification
//! requirements and optional fallback policy. Its result keeps all attempts and work.
//! [`VigilODE::solve`] retains the routing policy below.
//!
//! [`VigilODE::solve`] picks one of three existing entry points of this crate, runs it and
//! reports what ran. It has no integrator of its own.
//!
//! | [`SelectedTier`] | Entry point | Driver that runs |
//! | --- | --- | --- |
//! | `Tier1Explicit` | `Dopri5::solve` | Dormand-Prince 5(4), right-hand side only |
//! | `TierProtectedReference` | `Rodas5p::solve` | the RODAS5P driver family that the facade selects from the declared structure: dense, banded, matrix-free (GMRES), fast dense with a continuous extension, or sequential dense (mass matrix) |
//! | `Tier5AdaptiveSwitching` | `SwitchingSolver::solve` | windows of DOPRI5 and RODAS5P |
//!
//! What the module does:
//!
//! * **Routing by declared facts.** [`PreflightAnalyzer::analyze`] reads the strategy, the
//!   declared regime ([`VigilodeOptions::regime`]) and the declared structure of the
//!   problem. The method of a regime, the admission of the explicit method and the planned
//!   RODAS5P driver family come from the method registry (`MethodRegistry`). The
//!   logarithmic norm is not computed and contractivity is not taken for non-stiffness.
//! * **A charged stiffness screen** for a problem of dimension at most
//!   [`EXPLICIT_TRIAL_MAX_DIMENSION`] that declares nothing ([`StiffnessScreen`]): `n + 1`
//!   right-hand-side calls, all in the counters. It can only replace the explicit trial by
//!   RODAS5P.
//! * **A budgeted explicit trial with a fallback.** Without a declaration and without
//!   evidence of stiffness, DOPRI5 gets [`FallbackEngine::explicit_trial_attempts`]
//!   attempts; if it does not finish for a numerical reason, RODAS5P continues from its
//!   last accepted state with the attempts that are left of `Options::max_steps`.
//! * **The executed route.** Every executed leg is a [`UnifiedLeg`] whose driver
//!   identifier is read from the leg's own run record after it ran;
//!   [`VigilodeSolution::route_history`] carries the same identifiers.
//! * **Typed stops.** A run that stops early is `Ok` with its accepted prefix, the work of
//!   every leg and of the preflight, and [`VigilodeSolution::stop`]: the reason the driver
//!   that stopped gave ([`UnifiedStop`]).
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
mod fallback;
mod legs;
mod options;
mod router;
mod solution;
mod workspace;

pub use analyzer::{
    DOPRI5_REAL_STABILITY_BOUND, EXPLICIT_TRIAL_MAX_DIMENSION, PreflightAnalysis,
    PreflightAnalyzer, RouteBasis, SelectedTier, StiffnessScreen,
};
pub use fallback::{EXPLICIT_TRIAL_ATTEMPT_LIMIT, FallbackEngine};
pub use legs::{CallbackStop, UnifiedLeg, UnifiedStop};
pub use options::{PerformanceProfile, SolverStrategy, VigilodeOptions};
pub use rodas5p_integrators::MfStageSolvePolicy;
pub use router::VigilodeRouter;
pub use solution::{RouteTransitionRecord, VigilodeSolution};
pub use workspace::VigilodeWorkspace;

use crate::error::SolveError;
use crate::problem::Problem;

/// One call over `Dopri5::solve`, `Rodas5p::solve` and `SwitchingSolver::solve`; see the
/// [module documentation](self) for what it does and what it does not do.
#[derive(Clone, Copy, Debug, Default)]
pub struct VigilODE;

/// Failure to select or start a method through [`VigilODE::solve_method`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PortfolioError {
    /// A declared Lawson flow changes the supplied RHS to the residual of the split.
    #[error("a declared Lawson residual requires the explicit Lawson method")]
    SplitRequiresLawson,
    /// No method satisfied the requested choice and declared capabilities.
    #[error(transparent)]
    Route(#[from] crate::MethodRouteRefusal),
    /// The admitted driver could not start.
    #[error(transparent)]
    Solve(#[from] crate::MethodSolveError),
}

impl VigilODE {
    /// Solve with an explicitly selected GMRES stage policy on the actual matrix-free
    /// RODAS5P route. A supplied JVP (or general, nonbanded sparse JVP pattern) is
    /// required. The scope is identity mass, scalar absolute tolerance and endpoints.
    /// Ineligible combinations return a typed refusal before entering user code.
    ///
    /// `Shipped` follows the protected method. Column-tested policies opt into the
    /// performance tier and use the driver's true residual test before returning each
    /// stage. `stats.route.driver_id` identifies the executed policy, and
    /// `stats.counters` includes all charged JVPs and linear work. `options.tier`
    /// records the input option. The explicit policy determines the executed tier.
    pub fn solve_matrix_free(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &crate::Options,
        policy: MfStageSolvePolicy,
    ) -> Result<crate::Solution, SolveError> {
        let caps =
            crate::Rodas5p::capabilities(problem, options).map_err(SolveError::Unsupported)?;
        // General sparse JVPs can be assembled into a dense matrix for dense output
        // or vector tolerances even when the capability query reports matrix-free.
        if caps.route != crate::FacadeRoute::MatrixFree
            || problem.band.is_some()
            || options.dense_output
            || matches!(&options.atol, crate::Atol::Vector(_))
        {
            return Err(SolveError::Unsupported(
                crate::Unsupported::MatrixFreePolicyRequiresJvpRoute,
            ));
        }
        crate::Rodas5p::solve_with_stage_policy(problem, span, y0, options, &[], None, policy)
    }

    /// Select and run a registered integrator, preserving the driver's native result,
    /// route, work and typed stop. Use [`crate::MethodParams::default`] for ordinary
    /// ODEs. A Lawson split requires its declared flow and residual RHS.
    ///
    /// The optional portfolio fallback retries an admitted method from the initial
    /// state after a numerical stop, so the callback must support replay. Each method
    /// receives the configured per-run step budget. Inspect [`crate::MethodSolution::attempts`]
    /// and [`crate::MethodSolution::total_work`] for the costs of all attempts.
    pub fn solve_method(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &crate::Options,
        choice: crate::MethodChoice<'_>,
        params: &crate::MethodParams,
    ) -> Result<crate::MethodSolution, PortfolioError> {
        // Only Lawson reconstructs the full ODE from this residual and the declared flow.
        if params.lawson_flow.is_some()
            && !matches!(
                choice,
                crate::MethodChoice::Explicit(crate::METHOD_ID_LAWSON)
            )
        {
            return Err(PortfolioError::SplitRequiresLawson);
        }
        let plan = crate::MethodRegistry::route_with(problem, options, &choice, params)?;
        Ok(crate::MethodRegistry::solve(
            problem, span, y0, options, &plan,
        )?)
    }

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
