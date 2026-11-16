//! Solution record and trajectory representation for `VigilODE`.

use serde::{Deserialize, Serialize};

use crate::solution::{Solution, Stats, Status};

/// Record of an autonomous polyalgorithm tier transition during integration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteTransitionRecord {
    /// Name of the originating tier/driver.
    pub from_tier: String,
    /// Name of the destination tier/driver.
    pub to_tier: String,
    /// Time $t$ at which the transition occurred.
    pub time: f64,
    /// Diagnostic explanation for the transition.
    pub reason: String,
    /// Step index at the transition.
    pub step_index: usize,
}

/// Unified solution structure produced by [`super::VigilODE`].
#[derive(Clone, Debug, Serialize)]
pub struct VigilodeSolution {
    /// Whether integration reached the final time within configured tolerances.
    pub success: bool,
    /// Operational status of the integration run.
    pub status: Status,
    /// Final physical time reached.
    pub final_time: f64,
    /// Final state vector $y(t_{\text{final}})$.
    pub final_state: Vec<f64>,
    /// Accumulated runtime statistics and complete work counters.
    pub stats: Stats,
    /// Chronological list of polyalgorithm method transitions.
    pub route_history: Vec<RouteTransitionRecord>,
    /// Maximum absolute drift observed for each registered invariant $\max_t |I(y(t)) - I(y_0)|$.
    pub invariant_drift: Vec<(String, f64)>,
    /// Statistical timing authority flag (`"diagnostic_only"` under R2-STAT-01).
    pub timing_authority: &'static str,
    /// Wall-clock elapsed time in nanoseconds (diagnostic only).
    pub elapsed_nanos: u128,
    /// Underlying driver solution (if dense continuous extension or events are attached).
    #[serde(skip)]
    pub inner_solution: Option<Solution>,
}

impl VigilodeSolution {
    /// Returns the final state vector if available.
    pub fn final_state(&self) -> Option<&[f64]> {
        if self.final_state.is_empty() {
            None
        } else {
            Some(&self.final_state)
        }
    }

    /// Evaluate the continuous solution at physical time `t` in the span.
    ///
    /// If continuous dense output was enabled in [`super::VigilodeOptions`],
    /// evaluates the Hermite / stage polynomial interpolant.
    pub fn at(&self, t: f64) -> Option<Vec<f64>> {
        if let Some(ref inner) = self.inner_solution {
            inner.at(t)
        } else if (t - self.final_time).abs() < 1e-14 {
            Some(self.final_state.clone())
        } else {
            None
        }
    }

    /// Whether the solve completed successfully.
    pub fn success(&self) -> bool {
        self.success && self.status == Status::Success
    }

    /// Operational statistics and work counters.
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Maximum observed drift for all registered physical invariants.
    pub fn invariant_drift(&self) -> &[(String, f64)] {
        &self.invariant_drift
    }

    /// Maximum invariant drift across all tracked quantities.
    pub fn max_invariant_drift(&self) -> f64 {
        self.invariant_drift
            .iter()
            .map(|(_, d)| *d)
            .fold(0.0, f64::max)
    }

    /// Require that the run succeeded, otherwise return [`crate::SolveError`].
    pub fn ensure_success(self) -> Result<Self, crate::SolveError> {
        if self.success() {
            Ok(self)
        } else if let Some(ref inner) = self.inner_solution {
            inner.clone().ensure_success()?;
            Ok(self)
        } else {
            Err(crate::SolveError::Driver(format!(
                "integration stopped at t = {} before final target",
                self.final_time
            )))
        }
    }
}
