//! Options of the unified router.
//!
//! Three fields decide what runs: [`VigilodeOptions::options`] (tolerances, the attempt
//! budget and the output mode of the drivers), [`VigilodeOptions::strategy`] and
//! [`VigilodeOptions::regime`]. The other fields are recorded and read by nothing; their
//! documentation says so.

use serde::{Deserialize, Serialize};

use crate::ProblemRegime;
use crate::options::{Atol, InteriorError, Options, Tier};

/// What the router does with a request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SolverStrategy {
    /// Route by declared facts ([`super::PreflightAnalyzer::analyze`] lists the rules):
    /// the declared regime, what the explicit method admits, a supplied derivative or
    /// band; without any declaration DOPRI5 with forward break-even pilots of RODAS5P
    /// ([`super::RouteBasis::BreakEven`]).
    #[default]
    Auto,
    /// Routes exactly as [`SolverStrategy::Auto`]: no route of this release is faster
    /// than the one `Auto` picks. Kept so that existing callers compile.
    HighPerformance,
    /// Calls `Rodas5p::solve` directly.
    RobustStiff,
    /// Calls `SwitchingSolver::solve` with finite-difference derivatives allowed; if the
    /// adapter refuses the request before any callback, `Rodas5p::solve` runs instead.
    AdaptiveReentrant,
}

/// A profile name that is stored in the options and read by nothing: neither the router
/// nor a driver changes a tolerance, a step bound or a linear threshold because of it.
/// Kept so that existing callers and stored option files stay valid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PerformanceProfile {
    /// Stored only.
    #[default]
    Standard,
    /// Stored only.
    Conservative,
    /// Stored only.
    Aggressive,
}

mod regime_serde {
    //! `ProblemRegime` serializes as `unknown`, `nonstiff` or `stiff` and has no
    //! `Deserialize`; the options read the same three names.

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::ProblemRegime;

    pub fn serialize<S: Serializer>(
        regime: &ProblemRegime,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        regime.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ProblemRegime, D::Error> {
        let name = String::deserialize(deserializer)?;
        match name.as_str() {
            "unknown" => Ok(ProblemRegime::Unknown),
            "nonstiff" => Ok(ProblemRegime::Nonstiff),
            "stiff" => Ok(ProblemRegime::Stiff),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["unknown", "nonstiff", "stiff"],
            )),
        }
    }
}

/// Options of [`super::VigilODE`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VigilodeOptions {
    /// Options of the drivers. [`Options::max_steps`] is the attempt budget of the whole
    /// solve: every leg is charged to it.
    pub options: Options,
    /// What the router does with the request.
    pub strategy: SolverStrategy,
    /// Stored only; see [`PerformanceProfile`].
    pub profile: PerformanceProfile,
    /// Stored only. The matrix-free route of `Rodas5p::solve` uses its own fixed Krylov
    /// configuration, recorded in `Stats::matrix_free_linear`; this number does not reach
    /// it and sizes no buffer.
    pub max_krylov_dim: usize,
    /// Stored only; see [`VigilodeOptions::max_krylov_dim`].
    pub max_krylov_restarts: usize,
    /// Stored only. The solve is neither stopped nor flagged by it: compare it with
    /// [`super::VigilodeSolution::max_invariant_drift`] yourself.
    pub invariant_drift_tol: Option<f64>,
    /// What the caller knows about the problem, with the meaning it has for
    /// `GeneralOdePolicy`. Under [`SolverStrategy::Auto`], `Stiff` runs RODAS5P and
    /// `Nonstiff` runs DOPRI5 with the whole attempt budget and no second method;
    /// `Unknown` (the default) leaves the choice to the declared structure of the problem.
    #[serde(default, with = "regime_serde")]
    pub regime: ProblemRegime,
}

impl Default for VigilodeOptions {
    fn default() -> Self {
        Self {
            options: Options::default(),
            strategy: SolverStrategy::Auto,
            profile: PerformanceProfile::Standard,
            max_krylov_dim: 30,
            max_krylov_restarts: 5,
            invariant_drift_tol: None,
            regime: ProblemRegime::Unknown,
        }
    }
}

impl VigilodeOptions {
    /// Wrap driver [`Options`] with the default strategy and an undeclared regime.
    pub fn new(options: Options) -> Self {
        Self {
            options,
            ..Default::default()
        }
    }

    /// Set the strategy.
    pub fn strategy(mut self, strategy: SolverStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Declare the regime of the problem.
    pub fn regime(mut self, regime: ProblemRegime) -> Self {
        self.regime = regime;
        self
    }

    /// Store a profile name; nothing reads it ([`PerformanceProfile`]).
    pub fn profile(mut self, profile: PerformanceProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Set the relative tolerance.
    pub fn rtol(mut self, rtol: f64) -> Self {
        self.options.rtol = rtol;
        self
    }

    /// Set a scalar absolute tolerance.
    pub fn atol(mut self, atol: f64) -> Self {
        self.options.atol = Atol::Scalar(atol);
        self
    }

    /// Set a vector absolute tolerance.
    pub fn atol_vector(mut self, atol: Vec<f64>) -> Self {
        self.options.atol = Atol::Vector(atol);
        self
    }

    /// Set the initial step size.
    pub fn h0(mut self, h0: f64) -> Self {
        self.options.h0 = h0;
        self
    }

    /// Set the attempt budget of the whole solve.
    pub fn max_steps(mut self, max_steps: usize) -> Self {
        self.options.max_steps = max_steps;
        self
    }

    /// Ask for the continuous extension. Only RODAS5P returns one, so the request runs
    /// `Rodas5p::solve`.
    pub fn dense_output(mut self, dense: bool) -> Self {
        self.options.dense_output = dense;
        self
    }

    /// Set the interior error control of dense output.
    pub fn interior_error(mut self, mode: InteriorError) -> Self {
        self.options.interior_error = mode;
        self
    }

    /// Set the tier option of the RODAS5P facade.
    pub fn tier(mut self, tier: Tier) -> Self {
        self.options.tier = tier;
        self
    }

    /// Store a Krylov dimension; nothing reads it ([`VigilodeOptions::max_krylov_dim`]).
    pub fn max_krylov_dim(mut self, dim: usize) -> Self {
        self.max_krylov_dim = dim;
        self
    }

    /// Store a restart count; nothing reads it.
    pub fn max_krylov_restarts(mut self, restarts: usize) -> Self {
        self.max_krylov_restarts = restarts;
        self
    }

    /// Store an invariant drift tolerance; nothing reads it
    /// ([`VigilodeOptions::invariant_drift_tol`]).
    pub fn invariant_drift_tol(mut self, tol: f64) -> Self {
        self.invariant_drift_tol = Some(tol);
        self
    }
}

impl From<Options> for VigilodeOptions {
    fn from(options: Options) -> Self {
        Self::new(options)
    }
}

impl From<&Options> for VigilodeOptions {
    fn from(options: &Options) -> Self {
        Self::new(options.clone())
    }
}

impl From<&VigilodeOptions> for VigilodeOptions {
    fn from(options: &VigilodeOptions) -> Self {
        options.clone()
    }
}

impl From<SolverStrategy> for VigilodeOptions {
    fn from(strategy: SolverStrategy) -> Self {
        Self {
            strategy,
            ..Default::default()
        }
    }
}
