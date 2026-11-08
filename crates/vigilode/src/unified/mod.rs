//! # VigilODE Unified Production Solver Package
//!
//! Integrates all mathematical pillars of the VigilODE research lineage:
//! - **Causal Depth & Stage Batching ($L^8 = 0$):** Nilpotent homotopy sweep replacing sequential stage waits.
//! - **Matrix-Free Krylov with Subspace Recycling:** Directional JVP-only staged GMRES and GCRO-DR.
//! - **Newton-Raphson-Free Homotopy / Exponential Actions:** Direct flow without algebraic equation stalling.
//! - **Nonnormal Dynamical Observers:** Log-norm $\mu_2$ and resolvent analysis for proactive stiffness detection.
//! - **Fail-Closed Protected Fallback:** Zero data loss, complete work accounting, and transactional rollback.
//! - **Zero-Allocation Hot Loops (`VG-1`):** Pre-allocated workspace guarantees 0 heap allocations during stepping.

mod analyzer;
mod fallback;
mod options;
mod router;
mod solution;
mod workspace;

pub use analyzer::{PreflightAnalysis, PreflightAnalyzer, SelectedTier};
pub use fallback::FallbackEngine;
pub use options::{PerformanceProfile, SolverStrategy, VigilodeOptions};
pub use router::VigilodeRouter;
pub use solution::{RouteTransitionRecord, VigilodeSolution};
pub use workspace::VigilodeWorkspace;

use crate::error::SolveError;
use crate::problem::Problem;

/// Unified production solver frontend for VigilODE.
///
/// One single, intuitive, high-performance interface for all ordinary differential
/// equations (stiff, nonstiff, oscillatory, DAE index-1, matrix-free).
#[derive(Clone, Copy, Debug, Default)]
pub struct VigilODE;

impl VigilODE {
    /// Solve an initial value problem `y' = f(t, y)` across `span` starting from `y0`.
    ///
    /// Automatically performs preflight system analysis and routes to the optimal
    /// numerical engine, falling back seamlessly to protected RODAS5P if numerical distress is encountered.
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

    /// Solve with a caller-provided pre-allocated workspace enforcing zero heap allocations
    /// in integration hot loops (`VG-1`).
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
