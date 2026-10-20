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

/// Interior (dense-output) error control of a run with
/// [`Options::dense_output`] (plan §5 T3). The estimate is formed at the
/// midpoint of every step the embedded estimate accepts, since
/// [`crate::Solution::at`] may evaluate the continuous extension anywhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InteriorError {
    /// The W-filtered estimate `(M - h gamma J0)^{-1} (h/2) d(1/2)` with the
    /// step's own stage matrix; the controller sees
    /// `max(embedded error, estimate)`, so an interior failure rejects and
    /// shrinks the step. One right-hand side and one back-solve per step; an
    /// estimate, not a bound (within 1 and 2.36 times the interior error of
    /// a scalar linear mode with constant defect).
    #[default]
    WFilteredEnforce,
    /// The shift-`h/2` filtered estimate (one Jacobian and one LU per step);
    /// a step whose estimate exceeds one tolerance unit is rejected.
    Enforce,
    /// No interior estimate: only the endpoint is error controlled, and the
    /// interior of a step may exceed the tolerance by orders of magnitude on
    /// stiff problems.
    Off,
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
    /// Return the state at every accepted step and the continuous extension
    /// ([`crate::Solution::at`]) instead of the states at `t0` and `t1` only.
    ///
    /// `true` runs the protected sequential dense driver with the interior
    /// error control of [`Options::interior_error`]; it needs an explicit or
    /// finite-difference Jacobian (a JVP-only problem is refused with
    /// [`crate::SolveError::Unsupported`]). With `false` the solution holds
    /// `t0` and `t1` (`t0` and the last accepted state for a failed run).
    pub dense_output: bool,
    /// Interior error control under [`Options::dense_output`]; ignored
    /// without dense output (only the endpoint is returned, and only the
    /// endpoint is controlled). Defaults to
    /// [`InteriorError::WFilteredEnforce`].
    #[serde(default)]
    pub interior_error: InteriorError,
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
            interior_error: InteriorError::WFilteredEnforce,
            tier: Tier::Protected,
        }
    }
}
