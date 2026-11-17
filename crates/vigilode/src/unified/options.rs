//! Options, strategies, and execution profiles for the unified production solver.

use serde::{Deserialize, Serialize};

use crate::options::{Atol, InteriorError, Options, Tier};

/// Routing strategy for the unified production solver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SolverStrategy {
    /// Autonomous routing based on preflight system diagnosis
    /// (dimension, sparsity, nonnormality mu_2, JVP availability, and stiffness).
    #[default]
    Auto,
    /// High-performance mode: prioritizes fast lanes (Matrix-Free Krylov / DOPRI5)
    /// with automatic fail-closed protected fallback.
    HighPerformance,
    /// Robust stiff mode: enters directly into L-stable RODAS5P.
    RobustStiff,
    /// Adaptive reentrant mode: dynamic switching between explicit and implicit
    /// across changing regimes.
    AdaptiveReentrant,
}

/// Performance profile to balance accuracy vs speed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PerformanceProfile {
    /// Balanced standard operational profile.
    #[default]
    Standard,
    /// Conservative profile with tight tolerances and strict verification.
    Conservative,
    /// Aggressive profile with enlarged step growth and relaxed linear thresholds.
    Aggressive,
}

/// Unified options for [`super::VigilODE`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VigilodeOptions {
    /// Underlying standard solver options.
    pub options: Options,
    /// High-level routing strategy.
    pub strategy: SolverStrategy,
    /// Performance and verification profile.
    pub profile: PerformanceProfile,
    /// Maximum Krylov subspace dimension before restart.
    pub max_krylov_dim: usize,
    /// Maximum number of Krylov restart cycles.
    pub max_krylov_restarts: usize,
    /// Permissible physical invariant absolute drift tolerance.
    pub invariant_drift_tol: Option<f64>,
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
        }
    }
}

impl VigilodeOptions {
    /// Wrap standard [`Options`] with default unified strategy settings.
    pub fn new(options: Options) -> Self {
        Self {
            options,
            ..Default::default()
        }
    }

    /// Set the solver routing strategy.
    pub fn strategy(mut self, strategy: SolverStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the performance profile.
    pub fn profile(mut self, profile: PerformanceProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Set the relative tolerance.
    pub fn rtol(mut self, rtol: f64) -> Self {
        self.options.rtol = rtol;
        self
    }

    /// Set scalar absolute tolerance.
    pub fn atol(mut self, atol: f64) -> Self {
        self.options.atol = Atol::Scalar(atol);
        self
    }

    /// Set vector absolute tolerance.
    pub fn atol_vector(mut self, atol: Vec<f64>) -> Self {
        self.options.atol = Atol::Vector(atol);
        self
    }

    /// Set initial step size.
    pub fn h0(mut self, h0: f64) -> Self {
        self.options.h0 = h0;
        self
    }

    /// Set maximum allowed step attempts.
    pub fn max_steps(mut self, max_steps: usize) -> Self {
        self.options.max_steps = max_steps;
        self
    }

    /// Enable continuous dense output.
    pub fn dense_output(mut self, dense: bool) -> Self {
        self.options.dense_output = dense;
        self
    }

    /// Set interior dense output error control mode.
    pub fn interior_error(mut self, mode: InteriorError) -> Self {
        self.options.interior_error = mode;
        self
    }

    /// Set the underlying driver tier.
    pub fn tier(mut self, tier: Tier) -> Self {
        self.options.tier = tier;
        self
    }

    /// Set maximum Krylov subspace dimension.
    pub fn max_krylov_dim(mut self, dim: usize) -> Self {
        self.max_krylov_dim = dim;
        self
    }

    /// Set maximum Krylov restart cycles.
    pub fn max_krylov_restarts(mut self, restarts: usize) -> Self {
        self.max_krylov_restarts = restarts;
        self
    }

    /// Set maximum allowed invariant drift.
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
