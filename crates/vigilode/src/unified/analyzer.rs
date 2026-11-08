//! Preflight problem analyzer: structural discovery, stiffness micro-probe, and log-norm observation.

use rodas5p_core::DenseMatrix;
use rodas5p_core::nonnormal_observers::FixedHLogNormObserver;

use super::options::{SolverStrategy, VigilodeOptions};
use crate::error::SolveError;
use crate::problem::{Jacobian, Problem};

/// Selected candidate numerical engine tier determined by preflight analysis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedTier {
    /// Nonstiff high-speed Dormand–Prince 5(4) explicit engine.
    Tier1Explicit,
    /// Matrix-free Krylov (staged GMRES with subspace recycling) engine.
    Tier2MatrixFreeKrylov,
    /// Fused exponential / Leja polynomial action engine.
    Tier3Exponential,
    /// Staged vectorized $L^8=0$ causal sweep or banded RODAS5P engine.
    Tier4StagedRodas5p,
    /// Protected standard L-stable RODAS5P direct-LU reference driver.
    TierProtectedReference,
    /// Adaptive reentrant switching engine between explicit and implicit.
    Tier5AdaptiveSwitching,
}

/// Structural and numerical characteristics discovered during preflight inspection.
#[derive(Clone, Debug)]
pub struct PreflightAnalysis {
    /// Recommended engine tier.
    pub suggested_tier: SelectedTier,
    /// Whether the system was identified as stiff.
    pub is_stiff: bool,
    /// Euclidean logarithmic norm $\mu_2(J)$ at $t_0$, if computable.
    pub estimated_mu2: Option<f64>,
    /// Whether a Jacobian-vector product operator is available.
    pub has_jvp: bool,
    /// Whether a banded structure was declared.
    pub has_band: bool,
    /// Whether a mass matrix is present.
    pub has_mass: bool,
    /// Whether index-1 DAE algebraic equations are present.
    pub requires_dae_handling: bool,
}

/// Analyzer that inspects the mathematical problem and selects the optimal solver engine.
pub struct PreflightAnalyzer;

impl PreflightAnalyzer {
    /// Perform structural inspection and mathematical probing at $(t_0, y_0)$.
    pub fn analyze(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        opts: &VigilodeOptions,
    ) -> Result<PreflightAnalysis, SolveError> {
        let n = problem.dimension();
        if n != y0.len() {
            return Err(SolveError::InvalidInput(format!(
                "initial state length {} does not match problem dimension {}",
                y0.len(),
                n
            )));
        }
        if !span.0.is_finite() || !span.1.is_finite() || span.1 <= span.0 {
            return Err(SolveError::InvalidInput(format!(
                "invalid integration interval ({}, {})",
                span.0, span.1
            )));
        }

        let has_mass = problem.mass_matrix().is_some();
        let has_band = problem.band().is_some();

        // Check if DAE handling is required
        if has_mass {
            return Ok(PreflightAnalysis {
                suggested_tier: SelectedTier::TierProtectedReference,
                is_stiff: true,
                estimated_mu2: None,
                has_jvp: false,
                has_band,
                has_mass: true,
                requires_dae_handling: true,
            });
        }

        // Check manual strategy overrides
        if opts.strategy == SolverStrategy::RobustStiff {
            return Ok(PreflightAnalysis {
                suggested_tier: SelectedTier::TierProtectedReference,
                is_stiff: true,
                estimated_mu2: None,
                has_jvp: matches!(problem.jacobian_provider(), Jacobian::Jvp(_)),
                has_band,
                has_mass: false,
                requires_dae_handling: false,
            });
        }

        if opts.strategy == SolverStrategy::AdaptiveReentrant {
            return Ok(PreflightAnalysis {
                suggested_tier: SelectedTier::Tier5AdaptiveSwitching,
                is_stiff: false,
                estimated_mu2: None,
                has_jvp: matches!(problem.jacobian_provider(), Jacobian::Jvp(_)),
                has_band,
                has_mass: false,
                requires_dae_handling: false,
            });
        }

        // Evaluate initial RHS to verify callback validity
        let mut f0 = vec![0.0; n];
        (problem.rhs_fn())(span.0, y0, &mut f0)
            .map_err(|e| SolveError::InvalidInput(format!("initial RHS evaluation failed: {e}")))?;

        match problem.jacobian_provider() {
            Jacobian::Jvp(_) => {
                // High dimension or explicit matrix-free request
                let tier = if n >= 16 || opts.strategy == SolverStrategy::HighPerformance {
                    SelectedTier::Tier2MatrixFreeKrylov
                } else {
                    SelectedTier::TierProtectedReference
                };
                Ok(PreflightAnalysis {
                    suggested_tier: tier,
                    is_stiff: true,
                    estimated_mu2: None,
                    has_jvp: true,
                    has_band: false,
                    has_mass: false,
                    requires_dae_handling: false,
                })
            }
            Jacobian::SparseJvp { pattern, .. } => {
                let is_banded = pattern.is_banded();
                let tier = if is_banded {
                    SelectedTier::Tier4StagedRodas5p
                } else {
                    SelectedTier::Tier2MatrixFreeKrylov
                };
                Ok(PreflightAnalysis {
                    suggested_tier: tier,
                    is_stiff: true,
                    estimated_mu2: None,
                    has_jvp: true,
                    has_band: is_banded,
                    has_mass: false,
                    requires_dae_handling: false,
                })
            }
            Jacobian::Banded { .. } => Ok(PreflightAnalysis {
                suggested_tier: SelectedTier::Tier4StagedRodas5p,
                is_stiff: true,
                estimated_mu2: None,
                has_jvp: false,
                has_band: true,
                has_mass: false,
                requires_dae_handling: false,
            }),
            Jacobian::Dense(jac_fn) => {
                let mut j_data = vec![0.0; n * n];
                jac_fn(span.0, y0, &mut j_data).map_err(|e| {
                    SolveError::InvalidInput(format!("initial Jacobian evaluation failed: {e}"))
                })?;

                let dense_mat = DenseMatrix::new(n, n, j_data)
                    .map_err(|e| SolveError::InvalidInput(e.to_string()))?;

                let mut observer = FixedHLogNormObserver::identity(n);
                let mu2 = observer.compute_mu(&dense_mat).ok();

                let is_contracting = mu2.is_some_and(|m| m <= 0.0);
                let dt = span.1 - span.0;

                // Nonstiff detection: contractive flow with moderate interval
                let suggested_tier = if is_contracting
                    && dt <= 100.0
                    && opts.strategy != SolverStrategy::RobustStiff
                {
                    SelectedTier::Tier1Explicit
                } else if has_band {
                    SelectedTier::Tier4StagedRodas5p
                } else {
                    SelectedTier::TierProtectedReference
                };

                Ok(PreflightAnalysis {
                    suggested_tier,
                    is_stiff: !is_contracting,
                    estimated_mu2: mu2,
                    has_jvp: false,
                    has_band,
                    has_mass: false,
                    requires_dae_handling: false,
                })
            }
            Jacobian::FiniteDifference => {
                if has_band {
                    Ok(PreflightAnalysis {
                        suggested_tier: SelectedTier::Tier4StagedRodas5p,
                        is_stiff: true,
                        estimated_mu2: None,
                        has_jvp: false,
                        has_band: true,
                        has_mass: false,
                        requires_dae_handling: false,
                    })
                } else if n <= 10 && opts.strategy != SolverStrategy::RobustStiff {
                    // Small system without declared structure: try high-speed explicit DOPRI5 first,
                    // with guaranteed automatic fail-closed fallback to Protected RODAS5P if stiffness is detected.
                    Ok(PreflightAnalysis {
                        suggested_tier: SelectedTier::Tier1Explicit,
                        is_stiff: false,
                        estimated_mu2: None,
                        has_jvp: false,
                        has_band: false,
                        has_mass: false,
                        requires_dae_handling: false,
                    })
                } else {
                    Ok(PreflightAnalysis {
                        suggested_tier: SelectedTier::TierProtectedReference,
                        is_stiff: true,
                        estimated_mu2: None,
                        has_jvp: false,
                        has_band: false,
                        has_mass: false,
                        requires_dae_handling: false,
                    })
                }
            }
        }
    }
}
