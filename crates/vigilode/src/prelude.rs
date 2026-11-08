//! Convenient prelude re-exporting the primary types and solvers of VigilODE.
//!
//! Bringing this into scope provides everything required to model and solve ODEs:
//!
//! ```rust
//! use vigilode::prelude::*;
//! ```

pub use crate::explicit::{Dopri5, ExplicitSolution, ExplicitSolveError};
pub use crate::production::{
    ProductionConfig, ProductionOutcomeError, ProductionSolveError, ProductionSolver,
};
pub use crate::switching::{SwitchingConfig, SwitchingSolution, SwitchingSolver};
pub use crate::unified::{
    PerformanceProfile, PreflightAnalysis, SelectedTier, SolverStrategy, VigilODE, VigilodeOptions,
    VigilodeSolution, VigilodeWorkspace,
};
pub use crate::{
    Atol, CallbackError, FailureReason, InteriorError, Jacobian, Options, Problem, ProblemRegime,
    Rodas5p, Solution, SolveError, Status, StopRecord, Tier,
};
