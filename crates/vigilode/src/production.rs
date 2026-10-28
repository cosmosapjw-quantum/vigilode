//! Explicit, serializable deployment contract for the RHS-first adapter.
//!
//! This surface narrows provider permissions and preserves the native partial
//! solution, failure reason and work ledger. It does not change local error
//! tests or turn their estimates into global error certificates. The complete
//! effective switching schedule is serialized: pressure/backoff controls are
//! operational choices, whereas strict entry/return comparisons use exact work.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    Atol, DerivativeFallback, InteriorError, Jacobian, Options, Problem, SwitchingConfig,
    SwitchingError, SwitchingSolution, SwitchingSolver, Tier,
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

/// Deployment entry point preserving all native solution and accounting data.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProductionSolver;

impl ProductionSolver {
    /// Validate the provider contract before callbacks, then run the adapter.
    ///
    /// A completed call may return `Ok` with `success == false`. Consumers must
    /// inspect `success`, `final_time`, `message` and `stats.accounting_complete`;
    /// its accepted prefix and all returned work remain available. Smooth
    /// forward identity-mass IVPs with endpoint observations are supported.
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
}
