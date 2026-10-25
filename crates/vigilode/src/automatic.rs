//! Opt-in automatic method-fitness selection with explicit resource permissions.
use crate::{
    DerivativeFallback, GeneralOde, GeneralOdePolicy, Options, Problem, ProblemRegime,
    SelectionRefusal, SwitchingConfig, SwitchingError, SwitchingSolution, SwitchingSolver,
};

/// Permissions and resource ceilings of the automatic endpoint-only pilot.
#[derive(Clone, Copy, Debug)]
pub struct AutomaticOdePolicy {
    /// Derivative evaluation is forbidden by default.
    pub derivatives: DerivativeFallback,
    /// Maximum dimension for a route that materializes a dense matrix.
    pub max_dense_dimension: usize,
    /// Permit supplied JVP/GMRES only where explicit work motivates a trial.
    pub allow_matrix_free: bool,
    /// Caller asserts callbacks are fixed, effect-free and safe to replay.
    pub replay_safe_callbacks: bool,
}
impl Default for AutomaticOdePolicy {
    fn default() -> Self {
        Self {
            derivatives: DerivativeFallback::Forbidden,
            max_dense_dimension: 64,
            allow_matrix_free: false,
            replay_safe_callbacks: false,
        }
    }
}

/// Initialization failure before any solve callback.
#[derive(Debug, thiserror::Error)]
pub enum AutomaticOdeError {
    /// A feature or resource request cannot be supported.
    #[error(transparent)]
    Selection(#[from] SelectionRefusal),
    /// Competitive pilots require an explicit callback replay contract.
    #[error("automatic competitive trials require replay-safe callbacks")]
    ReplayRequired,
    /// Invalid numeric configuration or switching request.
    #[error(transparent)]
    Switching(#[from] SwitchingError),
}

/// RHS-first automatic pilot with observable method choices.
///
/// The solver chooses between DOPRI and an authorized RODAS route using
/// observed work. It does not infer a chart, certify stiffness, or choose a
/// thread count. Declared algebraic charts and execution policy remain composable
/// adapters; their setup and physical-error contracts are the caller's inputs.
pub struct AutomaticOde;
impl AutomaticOde {
    /// Solve a smooth forward identity-mass IVP with endpoint observations.
    /// All discarded pilots are charged. An incomplete integration is retained
    /// in `SwitchingSolution` with `success=false`, not silently upgraded.
    pub fn solve(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &Options,
        policy: &AutomaticOdePolicy,
    ) -> Result<SwitchingSolution, AutomaticOdeError> {
        Self::solve_with_entry_policy(problem, span, y0, options, policy, false)
    }

    /// Opt into deterioration-triggered probe renewal and bounded negative
    /// evidence backoff. The same replay and derivative permissions apply.
    pub fn solve_adaptive(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &Options,
        policy: &AutomaticOdePolicy,
    ) -> Result<SwitchingSolution, AutomaticOdeError> {
        Self::solve_with_entry_policy(problem, span, y0, options, policy, true)
    }

    fn solve_with_entry_policy(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &Options,
        policy: &AutomaticOdePolicy,
        adaptive: bool,
    ) -> Result<SwitchingSolution, AutomaticOdeError> {
        let regime = if policy.derivatives == DerivativeFallback::Forbidden {
            ProblemRegime::Unknown
        } else {
            ProblemRegime::Stiff
        };
        GeneralOde::select(
            problem,
            options,
            &GeneralOdePolicy {
                regime,
                derivatives: policy.derivatives,
                max_dense_dimension: policy.max_dense_dimension,
                allow_matrix_free: policy.allow_matrix_free,
            },
        )?;
        if policy.derivatives != DerivativeFallback::Forbidden && !policy.replay_safe_callbacks {
            return Err(AutomaticOdeError::ReplayRequired);
        }
        let config = SwitchingConfig {
            max_dense_dimension: policy.max_dense_dimension,
            ..if adaptive {
                SwitchingConfig::adaptive_evidence(policy.derivatives)
            } else {
                SwitchingConfig::evidence_driven(policy.derivatives)
            }
        };
        Ok(SwitchingSolver::solve(problem, span, y0, options, &config)?)
    }
}
