//! Conservative, auditable selection for ordinary identity-mass ODEs.
//!
//! Selection uses declared information, not an uncharged timing pilot or an
//! inferred stiffness certificate. Experimental methods remain explicit APIs.

use crate::{
    DerivativeFallback, Dopri5, ExplicitSolution, ExplicitSolveError, FacadeRoute, Jacobian,
    Options, Problem, Rodas5p, Solution, SolveError, Unsupported,
};
use serde::Serialize;

/// User knowledge of the requested integration regime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProblemRegime {
    /// Try a budgeted explicit solve; no stiffness detection is implied.
    #[default]
    Unknown,
    /// The user expects an explicit method to be appropriate.
    Nonstiff,
    /// Require the Rosenbrock method and explicit derivative authorization.
    Stiff,
}

/// Resource and derivative permissions for [`GeneralOde`].
#[derive(Clone, Copy, Debug, Serialize)]
pub struct GeneralOdePolicy {
    /// Declared problem regime.
    pub regime: ProblemRegime,
    /// Permission to evaluate supplied derivatives or finite differences.
    pub derivatives: DerivativeFallback,
    /// Maximum dimension of dense matrix materialization or factorization. This is a resource ceiling,
    /// not an empirical or universal crossover estimate. Band storage is exempt.
    pub max_dense_dimension: usize,
    /// Explicit permission for the JVP/GMRES route; its benefit is problem dependent.
    pub allow_matrix_free: bool,
}

impl Default for GeneralOdePolicy {
    fn default() -> Self {
        Self {
            regime: ProblemRegime::Unknown,
            derivatives: DerivativeFallback::Forbidden,
            max_dense_dimension: 64,
            allow_matrix_free: false,
        }
    }
}

/// The two methods admitted to the ordinary selection surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProductionMethod {
    /// RHS-only Dormand--Prince 5(4), with FSAL reuse.
    Dopri5,
    /// Eight-stage Rosenbrock RODAS5P with an embedded local error estimate.
    Rodas5p,
}

/// Deterministic selection record; it is not a certificate of solution accuracy.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct GeneralSelection {
    /// Selected numerical method.
    pub method: ProductionMethod,
    /// User-declared regime.
    pub regime: ProblemRegime,
    /// Authorized derivative policy (explicit integration never uses it).
    pub derivatives: DerivativeFallback,
    /// Implicit route, absent for the RHS-only method.
    pub implicit_route: Option<FacadeRoute>,
}

/// A feature or permission outside the selected ordinary ODE surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectionRefusal {
    /// Use an explicit mass-matrix API instead.
    #[error("the selected general ODE surface requires identity mass")]
    MassMatrix,
    /// Legacy events are postprocessed, not in-loop stopping.
    #[error("event termination is not implemented by the selected surface")]
    Events,
    /// Interpolation contracts remain method-specific APIs.
    #[error("continuous output requires an explicit method-specific API")]
    DenseOutput,
    /// The stiff method requires derivative action.
    #[error("stiff integration requires explicit derivative authorization")]
    DerivativesForbidden,
    /// A default finite-difference Jacobian needs separate permission.
    #[error("ProvidedOnly requires a supplied Jacobian or JVP")]
    MissingJacobian,
    /// A nonautonomous RHS requires a supplied partial time derivative.
    #[error("ProvidedOnly requires autonomous() or a supplied time derivative")]
    MissingTimeDerivative,
    /// The selected route exceeds an explicit resource ceiling.
    #[error("the dense factorization dimension exceeds the configured resource ceiling")]
    DenseDimensionLimit,
    /// GMRES use must be explicitly enabled.
    #[error("the matrix-free route requires allow_matrix_free = true")]
    MatrixFreeNotEnabled,
    /// A restriction inherited from the underlying implicit implementation.
    #[error("unsupported implicit feature combination: {0}")]
    Implicit(Unsupported),
}

/// Why the selected solve could not be initialized, or an explicitly checked
/// partial solution failed. Partial trajectories remain in the wrapped error.
#[derive(Debug, thiserror::Error)]
pub enum GeneralSolveError {
    /// Selection refused before callbacks or workspace allocation.
    #[error(transparent)]
    Selection(#[from] SelectionRefusal),
    /// Explicit initialization or checked integration failure.
    #[error(transparent)]
    Explicit(#[from] ExplicitSolveError),
    /// Implicit initialization or checked integration failure.
    #[error(transparent)]
    Implicit(#[from] SolveError),
}

/// The full method-specific result, including partial states and work counters.
#[derive(Debug)]
pub enum GeneralResult {
    /// Dormand--Prince result.
    Explicit(ExplicitSolution),
    /// Rosenbrock result.
    Implicit(Solution),
}

/// Result and its deterministic selection receipt.
#[derive(Debug)]
pub struct GeneralSolution {
    /// How the method was selected.
    pub selection: GeneralSelection,
    /// Full result; callers can inspect method-specific diagnostics.
    pub result: GeneralResult,
}

impl GeneralSolution {
    /// Whether integration reached its requested end time.
    pub fn is_success(&self) -> bool {
        match &self.result {
            GeneralResult::Explicit(s) => s.success,
            GeneralResult::Implicit(s) => s.is_success(),
        }
    }
    /// Last accepted state, including for unsuccessful integration.
    pub fn final_state(&self) -> Option<&[f64]> {
        match &self.result {
            GeneralResult::Explicit(s) => Some(&s.final_state),
            GeneralResult::Implicit(s) => s.final_state(),
        }
    }
    /// Convert unsuccessful integration to a typed error retaining its partial result.
    pub fn ensure_success(self) -> Result<Self, GeneralSolveError> {
        let result = match self.result {
            GeneralResult::Explicit(s) => GeneralResult::Explicit(s.ensure_success()?),
            GeneralResult::Implicit(s) => GeneralResult::Implicit(s.ensure_success()?),
        };
        Ok(Self {
            selection: self.selection,
            result,
        })
    }
}

/// Explicitly scoped selection without hidden pilots or speculative methods.
pub struct GeneralOde;

impl GeneralOde {
    /// Select without calling user code or allocating numerical workspaces.
    /// Numerical input validity is checked by `solve`, not by this feature query.
    pub fn select(
        problem: &Problem,
        options: &Options,
        policy: &GeneralOdePolicy,
    ) -> Result<GeneralSelection, SelectionRefusal> {
        if problem.mass.is_some() {
            return Err(SelectionRefusal::MassMatrix);
        }
        if !problem.events.is_empty() {
            return Err(SelectionRefusal::Events);
        }
        if options.dense_output {
            return Err(SelectionRefusal::DenseOutput);
        }
        if policy.regime != ProblemRegime::Stiff {
            return Ok(GeneralSelection {
                method: ProductionMethod::Dopri5,
                regime: policy.regime,
                derivatives: policy.derivatives,
                implicit_route: None,
            });
        }
        match policy.derivatives {
            DerivativeFallback::Forbidden => return Err(SelectionRefusal::DerivativesForbidden),
            DerivativeFallback::ProvidedOnly => {
                if matches!(problem.jacobian, Jacobian::FiniteDifference) {
                    return Err(SelectionRefusal::MissingJacobian);
                }
                if !problem.autonomous && problem.time_derivative.is_none() {
                    return Err(SelectionRefusal::MissingTimeDerivative);
                }
            }
            DerivativeFallback::AllowFiniteDifference => {}
        }
        let caps = Rodas5p::capabilities(problem, options).map_err(SelectionRefusal::Implicit)?;
        if caps.route == FacadeRoute::MatrixFree && !policy.allow_matrix_free {
            return Err(SelectionRefusal::MatrixFreeNotEnabled);
        }
        // Dense callbacks with a band declaration still materialize/verify an
        // n-by-n matrix. Only native compact band or banded FD avoids that cost.
        let materializes_dense =
            caps.route == FacadeRoute::DenseFast || matches!(problem.jacobian, Jacobian::Dense(_));
        if materializes_dense && problem.dimension > policy.max_dense_dimension {
            return Err(SelectionRefusal::DenseDimensionLimit);
        }
        Ok(GeneralSelection {
            method: ProductionMethod::Rodas5p,
            regime: policy.regime,
            derivatives: policy.derivatives,
            implicit_route: Some(caps.route),
        })
    }

    /// Integrate using the recorded selection. Failed runs remain inspectable
    /// results; call [`GeneralSolution::ensure_success`] when failure must be an error.
    /// This does not automatically switch methods during a solve.
    pub fn solve(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &Options,
        policy: &GeneralOdePolicy,
    ) -> Result<GeneralSolution, GeneralSolveError> {
        let selection = Self::select(problem, options, policy)?;
        let result = match selection.method {
            ProductionMethod::Dopri5 => {
                GeneralResult::Explicit(Dopri5::solve(problem, span, y0, options)?)
            }
            ProductionMethod::Rodas5p => {
                GeneralResult::Implicit(Rodas5p::solve(problem, span, y0, options)?)
            }
        };
        Ok(GeneralSolution { selection, result })
    }
}
