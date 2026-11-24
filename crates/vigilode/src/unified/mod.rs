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

/// Stage linear algebra of the protected RODAS5P fast drivers, for
/// [`VigilODE::solve_stages`]. Only `Shipped` is ever selected by a default route.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum StagePolicy {
    /// The protected drivers exactly as [`crate::Rodas5p::solve`] runs them.
    #[default]
    Shipped,
    /// A GMRES stage policy of the matrix-free route ([`VigilODE::solve_matrix_free`]).
    MatrixFree(MfStageSolvePolicy),
    /// A stage client of the banded fast route ([`VigilODE::solve_banded`]).
    Banded(crate::BandedStagePolicy),
}

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

    /// One entry for every explicitly selected stage linear algebra of the RODAS5P fast
    /// drivers ([`StagePolicy`]): `Shipped` is [`crate::Rodas5p::solve`], `MatrixFree` is
    /// [`VigilODE::solve_matrix_free`] and `Banded` is [`VigilODE::solve_banded`]. The
    /// eight stages, the tableau, the controller and the output rules are the protected
    /// driver's in every policy; a policy changes only how each stage's linear system
    /// `W u = r` is solved, and a request outside the policy's route is refused with a
    /// typed [`crate::Unsupported`] before any callback.
    pub fn solve_stages(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &crate::Options,
        policy: StagePolicy,
    ) -> Result<crate::Solution, SolveError> {
        match policy {
            StagePolicy::Shipped => crate::Rodas5p::solve(problem, span, y0, options),
            StagePolicy::MatrixFree(p) => Self::solve_matrix_free(problem, span, y0, options, p),
            StagePolicy::Banded(p) => Self::solve_banded(problem, span, y0, options, p),
        }
    }

    /// Solve on the banded fast RODAS5P route with an explicitly selected stage linear
    /// algebra ([`crate::BandedStagePolicy`]). The stages, the controller, the output and
    /// the rejection rules are those of the protected fast driver in every policy.
    ///
    /// `Shipped` is [`crate::Rodas5p::solve`]. A candidate policy needs the banded fast
    /// route: a band Jacobian (or a banded sparse-JVP pattern) or a finite-difference
    /// Jacobian with [`Problem::banded`], identity mass and endpoint output. Every other
    /// combination, and a worker count other than one or two, is refused with
    /// [`crate::Unsupported::BandedPolicyRequiresBandedRoute`] before any callback.
    ///
    /// `stats.route.entry_point` names the client driver, `stats.driver_work` is
    /// [`crate::DriverWork::BandedClient`] with the client identifier, and
    /// `stats.tier_executed` is the performance tier for a candidate policy. `Tight`
    /// returns the protected driver's states and counters bit for bit; `TwoWay` agrees
    /// with them to rounding (a different elimination order) and returns the same bits
    /// with one worker as with two.
    pub fn solve_banded(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &crate::Options,
        policy: crate::BandedStagePolicy,
    ) -> Result<crate::Solution, SolveError> {
        crate::Rodas5p::solve_with_policies(
            problem,
            span,
            y0,
            options,
            &[],
            None,
            MfStageSolvePolicy::Shipped,
            policy,
        )
    }

    /// Solve on the protected client driver with the separable (Fourier) stage solver of a
    /// [`crate::spatial::DiffusionGrid`] (stage-lattice task SL-N5; derivations in
    /// `research/kernel_spectral_foundation_20261124/THEORY.md`).
    ///
    /// `stages` declares the Jacobian `L + c I` ([`crate::spatial::DiffusionGrid::separable_stages`]).
    /// The DFT in longitude (sphere) or in every axis (box) turns each stage matrix into one
    /// tridiagonal system per Fourier mode or one scalar per wave vector, so no Krylov
    /// iteration runs. Every preparation reads the inertia of the mode systems and refuses a
    /// step beyond the stage pencil, and every solve checks the stage mass identity. In
    /// [`crate::spatial::SeparableMode::Verified`] (the default) every solve forms the true
    /// residual with the problem's JVP and corrects it.
    ///
    /// The problem must reach the matrix-free route: identity mass, endpoint output and the
    /// grid's dimension; `Verified` mode also needs a JVP. Every other request is refused
    /// with [`crate::Unsupported::SeparableStagesRequireMatrixFreeRoute`] before any
    /// callback. `stats.driver_work` is [`crate::DriverWork::SeparableClient`] with the
    /// client's report, and `stats.tier_executed` is the performance tier.
    pub fn solve_separable(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &crate::Options,
        stages: &crate::spatial::SeparableStages,
    ) -> Result<crate::Solution, SolveError> {
        crate::Rodas5p::solve_with_clients(
            problem,
            span,
            y0,
            options,
            &[],
            None,
            MfStageSolvePolicy::Shipped,
            crate::BandedStagePolicy::Shipped,
            Some(stages),
        )
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
