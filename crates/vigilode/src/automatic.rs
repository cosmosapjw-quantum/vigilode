//! Opt-in automatic method-fitness selection with explicit resource permissions.
use crate::{
    DerivativeFallback, GeneralOde, GeneralOdePolicy, Jacobian, Options, Problem, ProblemRegime,
    SelectionRefusal, SwitchingConfig, SwitchingError, SwitchingSolution, SwitchingSolver,
    Unsupported,
};

/// Permissions and resource ceilings of the automatic endpoint-only pilot.
#[derive(Clone, Copy, Debug)]
pub struct AutomaticOdePolicy {
    /// Derivative evaluation is forbidden by default. Permission makes an
    /// available implicit route eligible; it does not require its availability.
    pub derivatives: DerivativeFallback,
    /// Maximum dimension for a route that materializes a dense matrix.
    pub max_dense_dimension: usize,
    /// Permit supplied JVP/GMRES only where explicit work motivates a trial.
    pub allow_matrix_free: bool,
    /// Caller asserts callbacks are fixed, effect-free and safe to replay.
    /// Required only when an implicit challenger is actually eligible.
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
        // The requested ODE/output contract is independent of the availability
        // of an optional implicit challenger. Permissions are not obligations.
        let selection_policy = GeneralOdePolicy {
            regime: ProblemRegime::Unknown,
            derivatives: policy.derivatives,
            max_dense_dimension: policy.max_dense_dimension,
            allow_matrix_free: policy.allow_matrix_free,
        };
        GeneralOde::select(problem, options, &selection_policy)?;
        let unavailable = if policy.derivatives == DerivativeFallback::Forbidden {
            None
        } else {
            validate_optional_declarations(problem)?;
            match GeneralOde::select(
                problem,
                options,
                &GeneralOdePolicy {
                    regime: ProblemRegime::Stiff,
                    ..selection_policy
                },
            ) {
                Ok(_) => None,
                Err(reason) if optional_challenger_unavailable(reason) => Some(reason),
                Err(reason) => return Err(reason.into()),
            }
        };
        let fallback = if unavailable.is_some() {
            DerivativeFallback::Forbidden
        } else {
            policy.derivatives
        };
        if fallback != DerivativeFallback::Forbidden && !policy.replay_safe_callbacks {
            return Err(AutomaticOdeError::ReplayRequired);
        }
        let config = SwitchingConfig {
            max_dense_dimension: policy.max_dense_dimension,
            ..if adaptive {
                SwitchingConfig::adaptive_evidence(fallback)
            } else {
                SwitchingConfig::evidence_driven(fallback)
            }
        };
        let mut result = SwitchingSolver::solve(problem, span, y0, options, &config)?;
        if let Some(reason) = unavailable {
            // Keep the original completion/failure text and all charged native
            // work. This is a capability decision before callbacks, not a
            // numerical pilot or evidence that the IVP is nonstiff.
            result
                .message
                .push_str("; implicit challenger unavailable: ");
            result.message.push_str(&reason.to_string());
            for segment in &mut result.stats.segments {
                segment.decision = unavailable_decision(reason);
            }
        }
        Ok(result)
    }
}

// Only known route-specific limitations can disable the optional challenger.
// Global feature refusals and new/unknown restrictions remain explicit errors.
fn optional_challenger_unavailable(reason: SelectionRefusal) -> bool {
    matches!(
        reason,
        SelectionRefusal::MissingJacobian
            | SelectionRefusal::MissingTimeDerivative
            | SelectionRefusal::DenseDimensionLimit
            | SelectionRefusal::MatrixFreeNotEnabled
            | SelectionRefusal::Implicit(Unsupported::VectorAtolOnMatrixFreeRoute)
            | SelectionRefusal::Implicit(Unsupported::VectorAtolWithDeclaredBandOnDense)
    )
}

fn unavailable_decision(reason: SelectionRefusal) -> &'static str {
    match reason {
        SelectionRefusal::MissingJacobian => {
            "RHS-only: optional implicit challenger has no authorized Jacobian/JVP provider"
        }
        SelectionRefusal::MissingTimeDerivative => {
            "RHS-only: optional implicit challenger has no authorized time derivative"
        }
        SelectionRefusal::DenseDimensionLimit => {
            "RHS-only: optional implicit challenger exceeds dense dimension ceiling"
        }
        SelectionRefusal::MatrixFreeNotEnabled => {
            "RHS-only: optional implicit challenger requires matrix-free permission"
        }
        SelectionRefusal::Implicit(Unsupported::VectorAtolOnMatrixFreeRoute) => {
            "RHS-only: optional matrix-free challenger cannot preserve vector atol"
        }
        SelectionRefusal::Implicit(Unsupported::VectorAtolWithDeclaredBandOnDense) => {
            "RHS-only: optional declared-band dense challenger cannot preserve vector atol"
        }
        _ => "RHS-only: optional implicit challenger unavailable",
    }
}

// These checks precede capability downgrading so malformed declarations do
// not become silently acceptable merely because another provider is missing.
// They mirror SwitchingSolver's derivative-route input contract. With Forbidden
// the explicit facade continues to ignore unused derivative declarations.
fn validate_optional_declarations(problem: &Problem) -> Result<(), AutomaticOdeError> {
    if problem.autonomous && problem.time_derivative.is_some() {
        return Err(
            SwitchingError("autonomous derivative fallback cannot also have df/dt".into()).into(),
        );
    }
    if let Some((lower, upper)) = problem.band
        && (lower >= problem.dimension || upper >= problem.dimension)
    {
        return Err(SwitchingError("band widths exceed dimension".into()).into());
    }
    if let Jacobian::Banded { lower, upper, .. } = problem.jacobian
        && (lower >= problem.dimension
            || upper >= problem.dimension
            || problem.band.is_some_and(|band| band != (lower, upper)))
    {
        return Err(SwitchingError("band provider/declaration mismatch".into()).into());
    }
    if matches!(problem.jacobian, Jacobian::Jvp(_)) && problem.band.is_some() {
        return Err(SelectionRefusal::Implicit(Unsupported::BandDeclarationOnJvpProblem).into());
    }
    Ok(())
}
