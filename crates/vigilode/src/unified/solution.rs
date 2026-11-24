//! The result of a unified solve: the accepted end point, the executed legs, the total
//! work and the typed stop.

use serde::{Deserialize, Serialize};

use super::analyzer::PreflightAnalysis;
use super::breakeven::PilotRecord;
use super::legs::{UnifiedLeg, UnifiedStop};
use crate::explicit::ExplicitSolution;
use crate::solution::{Solution, Stats, Status};
use crate::switching::SwitchingSolution;

/// One executed leg in the route history, written after the leg ran. On the break-even
/// route the entry of the RODAS5P leg after an accepted pilot is the handover: its `time` is
/// the pilot's start `t_D`, and its `reason` states `H`, the window and the pilot costs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteTransitionRecord {
    /// Driver identifier of the leg before this one; `preflight` for the first leg.
    pub from_tier: String,
    /// Driver identifier of this leg, as its own run record states it
    /// ([`UnifiedLeg::driver_id`]). It is not a tier name.
    pub to_tier: String,
    /// Start time of the leg.
    pub time: f64,
    /// Why the leg was started: the basis of the analysis, the stop of the leg before, or
    /// the handover after an accepted pilot.
    pub reason: String,
    /// Accepted steps of the legs before this one.
    pub step_index: usize,
}

/// Result of [`super::VigilODE::solve`].
///
/// `Ok` does not mean that the end time was reached: check [`VigilodeSolution::success`]
/// or use [`VigilodeSolution::ensure_success`]. A run that stops early keeps its accepted
/// prefix (`final_time`, `final_state`), all its work and its typed stop.
#[derive(Clone, Debug, Serialize)]
pub struct VigilodeSolution {
    /// Whether the last leg reached the end time.
    pub success: bool,
    /// `Success`, or `Failed` with the facade reason of [`VigilodeSolution::stop`]
    /// ([`UnifiedStop::facade_reason`]; it says where the translation is not exact).
    pub status: Status,
    /// Last accepted time.
    pub final_time: f64,
    /// Last accepted state.
    pub final_state: Vec<f64>,
    /// The record of the whole solve in the facade's `Stats` type: the span and the
    /// initial state of the solve; `counters`, `attempts`, `accepted_steps` and
    /// `rejected_steps` summed over every leg and every pilot; `route` naming the driver
    /// of the last leg. When the last leg is RODAS5P, the driver-specific fields
    /// (`driver_work`, `driver_counters`, `adaptive`, `stop`, `driver_message`) are those
    /// of that leg; otherwise they are the empty placeholders of the type, and the
    /// driver's own record is in [`VigilodeSolution::explicit_solution`] or
    /// [`VigilodeSolution::switching_solution`]. `counters.fallback_steps` counts the
    /// RODAS5P legs that were started because another route did not finish or after an
    /// accepted pilot.
    pub stats: Stats,
    /// One record per executed leg, in order.
    pub route_history: Vec<RouteTransitionRecord>,
    /// `|I(y_final) - I(y0)|` for every invariant registered on the problem: the drift
    /// between the two end points, not a maximum over the trajectory.
    pub invariant_drift: Vec<(String, f64)>,
    /// Always `"diagnostic_only"`: the elapsed time below is one unpinned measurement.
    pub timing_authority: &'static str,
    /// Wall time of the solve in nanoseconds; diagnostic only.
    pub elapsed_nanos: u128,
    /// Why the solve stopped before the end time, as the driver that stopped reported it;
    /// `None` after a success.
    pub stop: Option<UnifiedStop>,
    /// Every executed leg in order, with its driver, its span, its work and its stop. After
    /// a handover the RODAS5P leg starts with the accepted pilot: its span, attempts, steps
    /// and work include the pilot's step.
    pub legs: Vec<UnifiedLeg>,
    /// Every pilot of the break-even route in order (empty on the other routes). The work
    /// of the pilots that are not the first step of a leg is in `stats.counters` and
    /// `stats.attempts`, not in a leg.
    pub pilots: Vec<PilotRecord>,
    /// The preflight analysis: the first entry point and why.
    pub analysis: PreflightAnalysis,
    /// The solution of the last RODAS5P run, unchanged from `Rodas5p::solve`. It covers that
    /// run only: after DOPRI5 it starts where DOPRI5 stopped, after a handover where the
    /// accepted pilot ended.
    #[serde(skip)]
    pub inner_solution: Option<Solution>,
    /// The solution of the DOPRI5 leg, unchanged from `Dopri5::solve`.
    #[serde(skip)]
    pub explicit_solution: Option<ExplicitSolution>,
    /// The solution of the switching adapter, unchanged from `SwitchingSolver::solve`.
    #[serde(skip)]
    pub switching_solution: Option<SwitchingSolution>,
}

impl VigilodeSolution {
    /// The last accepted state.
    pub fn final_state(&self) -> Option<&[f64]> {
        if self.final_state.is_empty() {
            None
        } else {
            Some(&self.final_state)
        }
    }

    /// The state at `t`: from the continuous extension of the RODAS5P leg when that leg
    /// ran with dense output and covers `t`, or the final state at the final time.
    pub fn at(&self, t: f64) -> Option<Vec<f64>> {
        if let Some(ref inner) = self.inner_solution {
            inner.at(t)
        } else if (t - self.final_time).abs() < 1e-14 {
            Some(self.final_state.clone())
        } else {
            None
        }
    }

    /// Whether the solve reached the end time.
    pub fn success(&self) -> bool {
        self.success && self.status == Status::Success
    }

    /// The record of the whole solve; see [`VigilodeSolution::stats`].
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// The end-point drift of every registered invariant.
    pub fn invariant_drift(&self) -> &[(String, f64)] {
        &self.invariant_drift
    }

    /// The largest end-point drift over the registered invariants.
    pub fn max_invariant_drift(&self) -> f64 {
        self.invariant_drift
            .iter()
            .map(|(_, d)| *d)
            .fold(0.0, f64::max)
    }

    /// The driver identifiers of the executed legs, in order.
    pub fn executed_drivers(&self) -> Vec<&str> {
        self.legs.iter().map(|leg| leg.driver_id.as_str()).collect()
    }

    /// The route `<method id>/<variant>` of the last executed leg.
    pub fn executed_route(&self) -> Option<&str> {
        self.legs.last().map(|leg| leg.route_id.as_str())
    }

    /// The accepted prefix as a facade [`Solution`]: the solution of the RODAS5P leg when
    /// it is the only leg, otherwise the two end points with the record of the whole solve.
    pub fn partial_solution(&self) -> Solution {
        if let (Some(inner), 1) = (&self.inner_solution, self.legs.len()) {
            return inner.clone();
        }
        let t0 = self.stats.t_span.0;
        let (t, y) = if self.final_time > t0 {
            (
                vec![t0, self.final_time],
                vec![self.stats.y0.clone(), self.final_state.clone()],
            )
        } else {
            (vec![t0], vec![self.stats.y0.clone()])
        };
        Solution {
            t,
            y,
            stats: self.stats.clone(),
            status: self.status.clone(),
            events: Vec::new(),
            splices: Vec::new(),
            extension: None,
        }
    }

    /// `Ok(self)` after a success, otherwise [`crate::SolveError::Failed`] with the facade
    /// reason and the accepted prefix ([`VigilodeSolution::partial_solution`]).
    pub fn ensure_success(self) -> Result<Self, crate::SolveError> {
        match &self.status {
            Status::Success if self.success => Ok(self),
            Status::Failed(reason) => Err(crate::SolveError::Failed {
                reason: reason.clone(),
                partial: Box::new(self.partial_solution()),
            }),
            Status::Success => Err(crate::SolveError::Driver(format!(
                "integration stopped at t = {} before the end time",
                self.final_time
            ))),
        }
    }
}
