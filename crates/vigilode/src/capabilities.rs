//! Executable feature combinations of the facade. This query performs no
//! callback, allocation of solver workspaces, or numerical attempt.

use crate::{Atol, InteriorError, Jacobian, Options, Problem, Rodas5p, Tier, Unsupported};
use serde::Serialize;

/// Protected driver family selected for a supported request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum FacadeRoute {
    /// Dense endpoint integration.
    DenseFast,
    /// Banded endpoint integration.
    BandedFast,
    /// JVP-only endpoint integration.
    MatrixFree,
    /// Dense integration with a continuous extension.
    FastDense,
    /// Banded integration with a continuous extension.
    FastBandedDense,
    /// Constant mass matrix or shift-h/2 interior control.
    SequentialDense,
}

/// How output accuracy is controlled on the selected route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum OutputAccuracy {
    /// Endpoint local error estimates; no continuous output is returned.
    EndpointLocalEstimate,
    /// Endpoint and midpoint local error estimates; no global bound is implied.
    EndpointAndInteriorLocalEstimates,
    /// Endpoint local estimate only, with an uncontrolled dense interior.
    EndpointEstimateWithUncheckedInterior,
}

/// Supported feature combination. Numerical input validation and any declared
/// band verification still happen in `solve`; this record is not an accuracy
/// certificate or a promise that the integration will succeed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Capabilities {
    /// Driver family selected from the problem and options.
    pub route: FacadeRoute,
    /// Requested contract tier.
    pub tier_requested: Tier,
    /// Executed contract tier (currently Protected for both requests).
    pub tier_executed: Tier,
    /// Local error-estimate contract.
    pub output_accuracy: OutputAccuracy,
    /// Declared events are located after integration from accepted segments.
    pub events_are_postprocessed: bool,
    /// Whether this facade supplies a global error certificate (currently false).
    pub global_error_certified: bool,
    /// Whether the facade constructs consistent DAE initial data (currently false).
    pub consistent_dae_initialization: bool,
}

impl Rodas5p {
    /// Query a feature combination without calling user code or running a
    /// solver. Unsupported combinations return the same typed reason as
    /// `solve` for well-formed inputs. Invalid dimensions, tolerances, spans,
    /// initial states or band declarations are checked by `solve` separately.
    pub fn capabilities(problem: &Problem, options: &Options) -> Result<Capabilities, Unsupported> {
        if let Some(reason) =
            primary_unsupported(problem, options).or_else(|| final_unsupported(problem))
        {
            return Err(reason);
        }
        let band = problem.band.is_some() || matches!(problem.jacobian, Jacobian::Banded { .. });
        let sequential = problem.mass.is_some()
            || (options.dense_output && options.interior_error == InteriorError::Enforce);
        let route = if sequential {
            FacadeRoute::SequentialDense
        } else if matches!(problem.jacobian, Jacobian::Jvp(_)) {
            FacadeRoute::MatrixFree
        } else if options.dense_output {
            if band {
                FacadeRoute::FastBandedDense
            } else {
                FacadeRoute::FastDense
            }
        } else if band {
            FacadeRoute::BandedFast
        } else {
            FacadeRoute::DenseFast
        };
        let output_accuracy = if !options.dense_output {
            OutputAccuracy::EndpointLocalEstimate
        } else if options.interior_error == InteriorError::Off {
            OutputAccuracy::EndpointEstimateWithUncheckedInterior
        } else {
            OutputAccuracy::EndpointAndInteriorLocalEstimates
        };
        Ok(Capabilities {
            route,
            tier_requested: options.tier,
            tier_executed: Tier::Protected,
            output_accuracy,
            events_are_postprocessed: !problem.events.is_empty(),
            global_error_certified: false,
            consistent_dae_initialization: false,
        })
    }
}

// Keep validation's original refusal order: the final JVP-band refusal occurs
// after malformed structure checks. Both phases are reused by the public query.
pub(crate) fn primary_unsupported(problem: &Problem, opts: &Options) -> Option<Unsupported> {
    let dense_or_mass = problem.mass.is_some() || opts.dense_output;
    let sequential = problem.mass.is_some()
        || (opts.dense_output && opts.interior_error == InteriorError::Enforce);
    if dense_or_mass && matches!(problem.jacobian, Jacobian::Jvp(_)) {
        return Some(Unsupported::JvpWithMassOrDenseOutput);
    }
    if dense_or_mass && matches!(problem.jacobian, Jacobian::Dense(_)) && problem.band.is_some() {
        return Some(Unsupported::DeclaredBandOnDenseWithMassOrDenseOutput);
    }
    if matches!(opts.atol, Atol::Vector(_)) {
        if sequential {
            return Some(Unsupported::VectorAtolOnSequentialRoute);
        }
        if matches!(problem.jacobian, Jacobian::Jvp(_)) {
            return Some(Unsupported::VectorAtolOnMatrixFreeRoute);
        }
        if matches!(problem.jacobian, Jacobian::Dense(_)) && problem.band.is_some() {
            return Some(Unsupported::VectorAtolWithDeclaredBandOnDense);
        }
    }
    if !problem.events.is_empty() && !opts.dense_output {
        return Some(Unsupported::EventsWithoutDenseOutput);
    }
    None
}

pub(crate) fn final_unsupported(problem: &Problem) -> Option<Unsupported> {
    (matches!(problem.jacobian, Jacobian::Jvp(_)) && problem.band.is_some())
        .then_some(Unsupported::BandDeclarationOnJvpProblem)
}
