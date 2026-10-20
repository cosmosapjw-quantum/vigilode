//! Solver options: every setting that changes a numerical result.

use serde::{Deserialize, Serialize};

/// The absolute tolerance of the error test.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
pub enum Atol {
    /// One absolute tolerance for every component.
    Scalar(f64),
    /// One absolute tolerance per component (length `n`).
    ///
    /// No protected driver of this release takes a per-component absolute
    /// tolerance, so [`crate::Rodas5p::solve`] refuses it with
    /// [`crate::SolveError::Unsupported`] instead of using a scalar in its
    /// place.
    Vector(Vec<f64>),
}

/// The tier of the production contract (plan §3) a run is executed under.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    /// P0: only the protected RODAS5P drivers run.
    #[default]
    Protected,
    /// P1: the tier that may run admitted candidates under the P0
    /// acceptance test.
    ///
    /// In this release no candidate is attached, so a `Performance` run
    /// executes exactly the `Protected` drivers (bit for bit); the record
    /// keeps both the requested and the executed tier
    /// ([`crate::Stats::tier_requested`], [`crate::Stats::tier_executed`]).
    Performance,
}

/// Options of [`crate::Rodas5p::solve`].
///
/// The defaults are those of the protected drivers'
/// `AdaptiveStepConfig::default()`: `rtol = 1e-6`, `atol = 1e-9`,
/// `h0 = 1e-3`, `max_steps = 100_000`. Every other controller setting
/// (safety factor, step-ratio bounds, minimum and maximum step, controller
/// kind) keeps the driver default and is recorded in
/// [`crate::Stats::adaptive`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Options {
    /// Relative tolerance of the weighted error test.
    pub rtol: f64,
    /// Absolute tolerance of the weighted error test.
    pub atol: Atol,
    /// The first trial step (clipped to the span).
    pub h0: f64,
    /// The maximum number of step attempts, accepted and rejected together
    /// (the drivers' `max_attempts`).
    pub max_steps: usize,
    /// Return the state at every accepted step instead of at `t0` and `t1`
    /// only.
    ///
    /// No protected driver of this release returns per-step states without
    /// changing its step sequence, so `true` is refused with
    /// [`crate::SolveError::Unsupported`]. With `false` the solution holds
    /// `t0` and `t1` (only `t0` for a failed run).
    pub dense_output: bool,
    /// The contract tier; see [`Tier`].
    pub tier: Tier,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            rtol: 1.0e-6,
            atol: Atol::Scalar(1.0e-9),
            h0: 1.0e-3,
            max_steps: 100_000,
            dense_output: false,
            tier: Tier::Protected,
        }
    }
}
