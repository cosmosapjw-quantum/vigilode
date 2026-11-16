//! Explicit, serializable deployment contract for the RHS-first adapter.
//!
//! This surface narrows provider permissions and preserves the native partial
//! solution, failure reason and work ledger. It does not change local error
//! tests or turn their estimates into global error certificates. The complete
//! effective switching schedule is serialized: pressure/backoff controls are
//! operational choices, whereas strict entry/return comparisons use exact work.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    Atol, DerivativeFallback, EntryPolicy, InteriorError, Jacobian, Options, Problem,
    SwitchingConfig, SwitchingError, SwitchingSolution, SwitchingSolver, Tier,
};

/// Configuration schema accepted by [`ProductionSolver`].
pub const PRODUCTION_CONFIG_SCHEMA: u32 = 1;

/// A complete deployment configuration without implicit serialized defaults.
///
/// Callers supply physical tolerances, an initial step and resource limits in
/// `options`, and all operational scheduling choices in `switching`. The
/// selective route permits a supplied JVP (including a caller-supplied AD JVP),
/// while this wrapper never enables finite differences or a state matrix.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionConfig {
    /// Must equal [`PRODUCTION_CONFIG_SCHEMA`]; unknown versions are refused.
    pub schema_version: u32,
    /// Physical tolerances, starting step, output contract and total attempts.
    #[serde(deserialize_with = "deserialize_options")]
    pub options: Options,
    /// Fully recorded operational schedule and derivative permission.
    pub switching: SwitchingConfig,
    /// The caller's contract that replayed callbacks describe the same function
    /// and have no externally committed effects. Required for selective probes.
    pub replay_safe_callbacks: bool,
}

// The legacy Options format accepts extra fields and defaults interior_error.
// Deployment files require explicit controls without changing that public
// legacy format for existing consumers.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictOptions {
    rtol: f64,
    atol: StrictAtol,
    h0: f64,
    max_steps: usize,
    dense_output: bool,
    interior_error: InteriorError,
    tier: Tier,
}

#[derive(Deserialize)]
#[serde(
    rename_all = "kebab-case",
    tag = "kind",
    content = "value",
    deny_unknown_fields
)]
enum StrictAtol {
    Scalar(f64),
    Vector(Vec<f64>),
}

fn deserialize_options<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Options, D::Error> {
    let value = StrictOptions::deserialize(deserializer)?;
    Ok(Options {
        rtol: value.rtol,
        atol: match value.atol {
            StrictAtol::Scalar(value) => Atol::Scalar(value),
            StrictAtol::Vector(value) => Atol::Vector(value),
        },
        h0: value.h0,
        max_steps: value.max_steps,
        dense_output: value.dense_output,
        interior_error: value.interior_error,
        tier: value.tier,
    })
}

impl ProductionConfig {
    /// Prepare one persistent RHS-only session with the requested observations.
    ///
    /// All numerical tolerances and attempt limits come from `options`.
    /// Inactive switching controls retain their recorded native values.
    pub fn rhs_only(options: Options, window_count: usize) -> Self {
        Self {
            schema_version: PRODUCTION_CONFIG_SCHEMA,
            options,
            switching: SwitchingConfig {
                fallback: DerivativeFallback::Forbidden,
                window_count,
                carry_explicit_step: true,
                ..SwitchingConfig::default()
            },
            replay_safe_callbacks: false,
        }
    }

    /// Prepare selective supplied-JVP entry and return with exact work decisions.
    ///
    /// The caller supplies the complete schedule, including a positive entry
    /// pilot budget, and explicitly declares callback replay safety. This
    /// factory adds no numerical thresholds. It selects the existing persistent
    /// explicit session, checkpoint proposal reuse, exact comparisons and the
    /// proven callback lower-bound stop. Other scheduling fields are preserved.
    pub fn selective_jvp(
        options: Options,
        mut switching: SwitchingConfig,
        replay_safe_callbacks: bool,
    ) -> Self {
        switching.fallback = DerivativeFallback::ProvidedOnly;
        switching.carry_explicit_step = true;
        switching.seed_entry_pilot_from_explicit_checkpoint = true;
        switching.strict_entry_work_improvement = true;
        switching.strict_return_work_improvement = true;
        switching.stop_dominated_return_comparator = true;
        Self {
            schema_version: PRODUCTION_CONFIG_SCHEMA,
            options,
            switching,
            replay_safe_callbacks,
        }
    }

    fn validate_provider(&self, problem: &Problem) -> Result<(), SwitchingError> {
        let refuse = |message: &str| SwitchingError(message.to_owned());
        if self.schema_version != PRODUCTION_CONFIG_SCHEMA {
            return Err(refuse("unsupported production configuration schema"));
        }
        if self.switching.entry_policy == Some(EntryPolicy::PressureOnly)
            && self.switching.fallback != DerivativeFallback::Forbidden
        {
            return Err(refuse(
                "production configuration refuses implicit pressure-only entry: campaign measured excessive overhead on rotation and nonstiff cells",
            ));
        }
        match self.switching.fallback {
            DerivativeFallback::Forbidden => Ok(()),
            DerivativeFallback::ProvidedOnly => {
                if !matches!(problem.jacobian, Jacobian::Jvp(_)) {
                    return Err(refuse(
                        "selective production profile requires a supplied JVP",
                    ));
                }
                if !self.replay_safe_callbacks {
                    return Err(refuse(
                        "selective production profile requires replay-safe callbacks",
                    ));
                }
                if self.switching.implicit_pilot_attempt_budget == 0
                    || !self.switching.strict_entry_work_improvement
                    || !self.switching.strict_return_work_improvement
                    || !self.switching.carry_explicit_step
                {
                    return Err(refuse(
                        "selective production profile requires positive pilots, persistent explicit state and exact entry/return work comparisons",
                    ));
                }
                Ok(())
            }
            DerivativeFallback::AllowFiniteDifference => Err(refuse(
                "production RHS-first profile permits only forbidden derivatives or a supplied JVP",
            )),
        }
    }
}

/// Error returned by [`ProductionSolver::solve_checked`].
#[derive(Debug, thiserror::Error)]
pub enum ProductionSolveError {
    /// Pre-solve validation refused the request before entering callbacks.
    #[error("invalid production solver request: {0}")]
    InvalidRequest(#[from] SwitchingError),

    /// Integration executed but did not complete successfully or accounting was incomplete.
    ///
    /// Preserves the partial accepted prefix, final time and state, stop message,
    /// and complete work accounting in the boxed [`SwitchingSolution`].
    #[error(
        "production solve unsuccessful (final_time = {}, success = {}, accounting_complete = {}): {}",
        .0.final_time,
        .0.success,
        .0.stats.accounting_complete,
        .0.message
    )]
    Unsuccessful(Box<SwitchingSolution>),
}

impl ProductionSolveError {
    /// Constructs an `Unsuccessful` error wrapping a partial solution.
    pub fn unsuccessful(solution: SwitchingSolution) -> Self {
        Self::Unsuccessful(Box::new(solution))
    }

    /// Returns a reference to the partial solution if integration executed.
    pub fn partial_solution(&self) -> Option<&SwitchingSolution> {
        match self {
            Self::Unsuccessful(sol) => Some(sol),
            Self::InvalidRequest(_) => None,
        }
    }

    /// Consumes the error and returns the partial solution if integration executed.
    pub fn into_partial_solution(self) -> Option<SwitchingSolution> {
        match self {
            Self::Unsuccessful(sol) => Some(*sol),
            Self::InvalidRequest(_) => None,
        }
    }

    /// Returns `true` if this error represents an unsuccessful solve preserving a partial prefix.
    pub fn is_unsuccessful(&self) -> bool {
        matches!(self, Self::Unsuccessful(_))
    }

    /// Returns `true` if this error was caused by pre-solve validation refusal.
    pub fn is_invalid_request(&self) -> bool {
        matches!(self, Self::InvalidRequest(_))
    }
}

/// Outcome error indicating that an integration run did not complete successfully or has incomplete accounting.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[error(
    "production outcome incomplete: success = {success}, final_time = {final_time}, accounting_complete = {accounting_complete}: {message}"
)]
pub struct ProductionOutcomeError {
    /// Whether the solver reported local error acceptance across all steps.
    pub success: bool,
    /// Last committed endpoint time reached.
    pub final_time: f64,
    /// Detail message from the solver or driver.
    pub message: String,
    /// Whether all trials, probes, and callback operations were accounted for.
    pub accounting_complete: bool,
}

impl ProductionOutcomeError {
    /// Whether the solver reported success.
    pub fn success(&self) -> bool {
        self.success
    }

    /// Last committed time reached.
    pub fn final_time(&self) -> f64 {
        self.final_time
    }

    /// Detail message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Whether accounting was complete.
    pub fn accounting_complete(&self) -> bool {
        self.accounting_complete
    }
}

impl SwitchingSolution {
    /// Returns `true` if and only if the solve completed successfully to `target_t` with complete accounting.
    ///
    /// Specifically verifies that:
    /// 1. `self.success == true`,
    /// 2. `self.stats.accounting_complete == true`,
    /// 3. `self.final_time == target_t`.
    pub fn is_successful_endpoint(&self, target_t: f64) -> bool {
        self.success && self.stats.accounting_complete && self.final_time == target_t
    }

    /// Verifies that the solve completed successfully with complete work accounting.
    ///
    /// Returns `Ok(())` if `self.success && self.stats.accounting_complete`.
    /// Otherwise returns `Err(ProductionOutcomeError)` describing the incomplete state.
    pub fn ensure_completed(&self) -> Result<(), ProductionOutcomeError> {
        if self.success && self.stats.accounting_complete {
            Ok(())
        } else {
            Err(ProductionOutcomeError {
                success: self.success,
                final_time: self.final_time,
                message: self.message.clone(),
                accounting_complete: self.stats.accounting_complete,
            })
        }
    }
}

/// Deployment entry point preserving all native solution and accounting data.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProductionSolver;

impl ProductionSolver {
    /// Validate the provider contract before callbacks, then run the adapter.
    ///
    /// # Important Result Semantics (Partial-Prefix & Unsuccessful Ok)
    ///
    /// Returning `Ok(solution)` from this method indicates that the adapter
    /// executed and produced a run record. It **does NOT guarantee** that
    /// integration reached the final time with error acceptance (`solution.success == true`).
    /// In particular, when integration terminates early due to:
    /// - a user callback error ([`crate::CallbackError::Fatal`]),
    /// - total attempt budget exhaustion ([`crate::Options::max_steps`]),
    /// - step size underflow or no representable forward progress,
    ///
    /// `solve` returns `Ok(solution)` containing the partial accepted prefix
    /// reached so far, with `solution.success == false`.
    ///
    /// Callers **must** inspect:
    /// - `solution.success`: whether the endpoint was reached with local error acceptance,
    /// - `solution.final_time`: the last committed time of the accepted prefix,
    /// - `solution.message`: human-readable explanation of completion or early stop reason,
    /// - `solution.stats.accounting_complete`: whether all callback invocations and trials were accounted for.
    ///
    /// For fail-closed semantics, callers should use [`Self::solve_checked`] or
    /// [`SwitchingSolution::ensure_completed`], which return an `Err` on unsuccessful
    /// or incomplete runs while preserving the partial solution.
    ///
    /// Smooth forward identity-mass IVPs with endpoint observations are supported.
    /// The underlying adapter refuses events, dense output, unsupported
    /// tolerance/provider combinations and invalid inputs before callbacks.
    pub fn solve(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        config: &ProductionConfig,
    ) -> Result<SwitchingSolution, SwitchingError> {
        config.validate_provider(problem)?;
        SwitchingSolver::solve(problem, t_span, y0, &config.options, &config.switching)
    }

    /// Validate the provider contract and run the adapter, returning an error if integration is unsuccessful.
    ///
    /// Returns:
    /// - `Ok(solution)` if and only if `solution.success == true && solution.stats.accounting_complete == true`.
    /// - `Err(ProductionSolveError::Unsuccessful(solution))` if the solve terminated early
    ///   (`success == false` or `accounting_complete == false`). The boxed [`SwitchingSolution`]
    ///   retains the partial accepted prefix (`t`, `y`, `final_time`, `final_state`), stop message,
    ///   and complete work accounting.
    /// - `Err(ProductionSolveError::InvalidRequest(err))` if pre-solve configuration validation fails before callbacks.
    pub fn solve_checked(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        config: &ProductionConfig,
    ) -> Result<SwitchingSolution, ProductionSolveError> {
        let solution = Self::solve(problem, t_span, y0, config)?;
        if solution.success && solution.stats.accounting_complete {
            Ok(solution)
        } else {
            Err(ProductionSolveError::Unsuccessful(Box::new(solution)))
        }
    }
}
