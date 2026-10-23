//! The result of a solve: states, status and the full run record.

use std::fmt;

use crate::records::{
    AdaptiveRecord, BandedWorkRecord, FastInteriorRecord, InteriorControlRecord,
    InteriorSamplingRecord, LinearSolverRecord, LuRecord, RoutingDecisionRecord, StepFailureKind,
    StopReason, WorkRecord,
};
use rodas5p_integrators::{FastDenseSegments, Rodas5pDenseSegment};
use serde::Serialize;

use crate::{
    error::SolveError,
    events::EventRecord,
    options::{Options, Tier},
};

/// Schema tag of [`Stats`] (bumped when a field changes meaning).
///
/// v3 (wave 3): dense output runs the fast dense drivers, recorded as
/// [`DriverWork::FastDense`] (`route.driver_id` `rodas5p-fast-dense-v1` or
/// `rodas5p-fast-banded-dense-v1`); only a mass matrix or
/// [`crate::InteriorError::Enforce`] keeps the sequential dense route. v3
/// also covers the event fields [`Stats::terminal_event`] and
/// [`Stats::event_evaluations`] (wave-3 node `p0-events`, added under the
/// v2 tag). Further v2 to v3 changes in meaning: `options.atol` can be
/// `Vector` in a record of a run that ran (v2 refused every vector), and
/// `adaptive.atol` is then the largest entry, which no driver read; the
/// `route.entry_point` strings now include the four `..._with_atol` entry
/// points and the fast dense entry points; and `driver_work` can be
/// `FastDense { band_work, interior, .. }`.
/// v4 (production re-audit): failed enforcing sequential interior estimates
/// reject the attempt and increment `estimate_failures`; `unavailable` is
/// report-only. Event interpolation failures have their own typed reason.
/// FD perturbations reverse direction only when the forward shift overflows.
pub const STATS_SCHEMA: &str = "vigilode.stats.v4";

/// Which user callback reported an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Callback {
    /// The right-hand side (including its calls inside finite differences).
    Rhs,
    /// The dense or band Jacobian callback.
    Jacobian,
    /// The Jacobian-vector product.
    Jvp,
    /// The explicit time derivative.
    TimeDerivative,
}

/// Why a run stopped before `t1`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[non_exhaustive]
pub enum FailureReason {
    /// The attempt budget [`Options::max_steps`] was used up.
    MaxStepsReached {
        /// The budget.
        max_steps: usize,
    },
    /// The step size fell below the drivers' minimum step: after repeated
    /// rejections by the local error test (embedded or interior), or because
    /// the controller's proposed step fell below the minimum although no
    /// attempt was rejected after the last accepted step (for example near a
    /// blow-up of the solution).
    StepSizeTooSmall {
        /// The drivers' minimum step.
        min_step: f64,
    },
    /// The step size fell below the minimum step after repeated failures of
    /// the stage linear solves (a singular or non-factorizable `M - h gamma J`,
    /// or a Krylov solve that did not converge).
    LinearSolveFailed {
        /// The drivers' minimum step.
        min_step: f64,
    },
    /// The step size fell below the minimum step after repeated non-finite
    /// attempts (NaN/Inf in a stage or the new state, or a
    /// [`crate::CallbackError::Recoverable`] at every smaller step).
    NonFinite {
        /// The drivers' minimum step.
        min_step: f64,
    },
    /// The step size fell below the minimum step after repeated nonlinear
    /// solve failures (not produced by the Rosenbrock stages; kept so that
    /// every driver failure kind has its own reason).
    NonlinearSolveFailed {
        /// The drivers' minimum step.
        min_step: f64,
    },
    /// An event function returned NaN or an infinity (events: see
    /// [`crate::Problem::event`]). The solution is truncated at the start of
    /// the step on which it happened. If the driver had already failed, that
    /// failure's reason is kept instead (the outputs are still truncated).
    EventFunctionNonFinite,
    /// The accepted-step continuous extension could not supply the state
    /// required by event location. This is distinct from a non-finite user
    /// event function; outputs are truncated at the failed step's start.
    EventInterpolationFailed {
        /// Time at which the state was unavailable.
        t: f64,
    },
    /// A user callback returned [`crate::CallbackError::Fatal`].
    CallbackFailed {
        /// The callback.
        callback: Callback,
        /// The time argument of the failing call (a stage time, not
        /// necessarily an accepted one).
        t: f64,
        /// The callback's message.
        message: String,
    },
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxStepsReached { max_steps } => {
                write!(f, "attempt budget of {max_steps} steps reached")
            }
            Self::StepSizeTooSmall { min_step } => write!(
                f,
                "step size fell below the minimum step {min_step:e} after local error rejections, or the controller's step fell below the minimum"
            ),
            Self::LinearSolveFailed { min_step } => write!(
                f,
                "step size fell below the minimum step {min_step:e} after linear solve failures"
            ),
            Self::NonFinite { min_step } => write!(
                f,
                "step size fell below the minimum step {min_step:e} after non-finite attempts"
            ),
            Self::NonlinearSolveFailed { min_step } => write!(
                f,
                "step size fell below the minimum step {min_step:e} after nonlinear solve failures"
            ),
            Self::EventFunctionNonFinite => {
                write!(f, "an event function returned a non-finite value")
            }
            Self::EventInterpolationFailed { t } => {
                write!(f, "event state unavailable at t = {t:e}")
            }
            Self::CallbackFailed {
                callback,
                t,
                message,
            } => write!(f, "{callback:?} callback failed at t = {t:e}: {message}"),
        }
    }
}

/// The outcome of a run.
///
/// `Success` means the run reached `t1`, or a terminal event ended the outputs
/// (see [`Stats::terminal_event`]; [`Stats::stop`] keeps any later driver
/// stop), with every accepted step passing the embedded error *estimate*; it
/// is not a certificate of the global error.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
#[non_exhaustive]
pub enum Status {
    /// The run reached `t1`, or a terminal event ended the outputs.
    Success,
    /// The run stopped early; the solution holds what was computed.
    Failed(FailureReason),
}

/// The Jacobian provider a run used.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[non_exhaustive]
pub enum JacobianRecord {
    /// A user dense Jacobian.
    Dense,
    /// A user dense Jacobian read as a declared band (verified at `t0`).
    DenseDeclaredBand {
        /// Sub-diagonals.
        lower: usize,
        /// Super-diagonals.
        upper: usize,
    },
    /// A user band Jacobian.
    Banded {
        /// Sub-diagonals.
        lower: usize,
        /// Super-diagonals.
        upper: usize,
    },
    /// A user Jacobian-vector product.
    Jvp,
    /// Dense forward differences.
    FiniteDifferenceDense {
        /// The increment rule.
        increment: &'static str,
    },
    /// Grouped (Curtis–Powell–Reid) forward differences of a declared band.
    FiniteDifferenceBanded {
        /// Sub-diagonals.
        lower: usize,
        /// Super-diagonals.
        upper: usize,
        /// Column groups (right-hand-side calls per band, besides `f(t, y)`).
        groups: usize,
        /// The increment rule.
        increment: &'static str,
    },
}

/// The increment rule of the finite-difference Jacobians.
pub const FD_INCREMENT_RULE: &str =
    "sqrt(eps)*max(|y_j|,1), represented as (y_j+inc)-y_j; reverse inc if shift overflows";

/// Work of the finite-difference Jacobian (zero for user Jacobians).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct FiniteDifferenceWork {
    /// Jacobian (or band) builds attempted.
    pub jacobian_builds: u64,
    /// Right-hand-side calls made by those builds (attempted, including
    /// failed ones); already added to [`Stats::counters`].
    pub rhs_calls: u64,
}

/// What the facade observed of the user callbacks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CallbackWork {
    /// Callbacks that returned [`crate::CallbackError::Recoverable`].
    pub recoverable_errors: u64,
    /// Callbacks that returned [`crate::CallbackError::Fatal`] (at most one
    /// enters user code; see `short_circuited_calls`).
    pub fatal_errors: u64,
    /// Defensive guard refusals after a fatal error, without entering user
    /// code. Protected native drivers stop on the original fatal signal, so
    /// ordinary terminal runs report zero. Any such calls remain charged.
    pub short_circuited_calls: u64,
    /// The right-hand-side calls among `short_circuited_calls`: user RHS code
    /// was entered `counters.rhs_calls - short_circuited_rhs_calls` times.
    pub short_circuited_rhs_calls: u64,
}

/// Krylov start counts of a matrix-free run (the driver's opt-in SP04
/// start telemetry, copied field by field).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct KrylovStartRecord {
    /// Stage solves started from zero.
    pub zero_starts: u64,
    /// Started from the previous stage's solution.
    pub previous_stage_starts: u64,
    /// Started from the previous step's solution.
    pub previous_step_starts: u64,
    /// Started from a solver-carried vector.
    pub solver_carried_starts: u64,
    /// Started from a basis of another system.
    pub cross_system_basis_starts: u64,
}

/// The driver-specific work counters and policy identifiers the routed
/// driver returned, beyond [`Stats::driver_counters`] and the step counts.
// One value per solve, kept inline for serde: the size difference between
// variants costs nothing worth a box.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "driver")]
#[non_exhaustive]
pub enum DriverWork {
    /// The dense fast driver (`Rodas5pFastResult`).
    Dense {
        /// Attempts that reused the Jacobian, `f(t, y)` and `f_t` of a
        /// rejected attempt from the same state.
        jacobian_reuses: usize,
        /// The LU variant that ran.
        lu: LuRecord,
    },
    /// The banded fast driver (`Rodas5pFastBandedResult`).
    Banded {
        /// Attempts that reused the Jacobian of a rejected attempt.
        jacobian_reuses: usize,
        /// The LU variant that ran.
        lu: LuRecord,
        /// Counted band factor and solve operations and storage.
        banded: BandedWorkRecord,
    },
    /// The matrix-free U-form driver (`Rodas5pMfFastResult`).
    MatrixFree {
        /// Attempts that reused `f(t, y)`, `f_t` and the JVP operator of a
        /// rejected attempt from the same state.
        state_reuses: usize,
        /// GCRO-DR recycle policy id.
        gcrodr_policy: &'static str,
        /// Krylov workspace option id.
        krylov_workspace: &'static str,
        /// Krylov start policy id.
        start_policy: &'static str,
        /// Recycle scope id.
        recycle_scope: &'static str,
        /// Start telemetry, when the driver recorded it.
        start_telemetry: Option<KrylovStartRecord>,
    },
    /// The fast dense or banded driver with a continuous extension
    /// (`integrate_rodas5p_fast_dense_observed` or
    /// `integrate_rodas5p_fast_banded_dense_observed`, `..._with_atol` with a
    /// vector [`crate::Atol`]): the dense-output route without a mass matrix,
    /// under [`crate::InteriorError::WFilteredEnforce`] or
    /// [`crate::InteriorError::Off`].
    FastDense {
        /// Whether the banded pipeline ran.
        banded: bool,
        /// Attempts that reused the Jacobian, `f(t, y)` and `f_t` of a
        /// rejected attempt from the same state.
        jacobian_reuses: usize,
        /// The LU variant that ran.
        lu: LuRecord,
        /// Counted band factor and solve operations and storage (banded
        /// pipeline only; its solves include the interior estimates'
        /// back-solves).
        band_work: Option<BandedWorkRecord>,
        /// The W-filtered interior estimates (all zero under `Off`); their
        /// right-hand sides and solves are also in [`Stats::driver_counters`].
        interior: FastInteriorRecord,
        /// Accepted steps whose extension the driver returned (before any
        /// event truncation).
        segments: usize,
    },
    /// The sequential adaptive dense driver
    /// (`integrate_adaptive_dense_observed_with_segments`): the mass-matrix
    /// route, and dense output under [`crate::InteriorError::Enforce`].
    SequentialDense {
        /// The interior error control that ran (`Off` without dense output).
        control: InteriorControlRecord,
        /// Which steps the interior estimate was formed on.
        interior_sampling: InteriorSamplingRecord,
        /// The stage linear solver of the sequential step.
        linear_solver: LinearSolverRecord,
        /// Interior estimates formed (accepted and rejected steps).
        samples: usize,
        /// Steps the interior estimate rejected.
        interior_rejections: usize,
        /// Report-mode samples whose estimate could not be formed.
        unavailable: usize,
        /// Attempts rejected because an enforcing interior estimate failed.
        estimate_failures: usize,
        /// The largest interior estimate of an accepted step, in tolerance
        /// units (an estimate, not a bound).
        max_accepted_estimate: Option<f64>,
        /// Work of the estimates (also in [`Stats::driver_counters`] under
        /// the enforcing modes).
        estimate_counters: WorkRecord,
        /// Accepted steps whose extension the driver returned (before any
        /// event truncation).
        segments: usize,
    },
}

/// Where and why a run stopped before `t1`, as the driver reported it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct StopRecord {
    /// The driver's last accepted time (`t0` when no step was accepted).
    /// After a terminal event or an event-function failure, [`Solution::t`]
    /// is cut before it.
    pub t: f64,
    /// Why the driver's loop ended.
    pub reason: StopReason,
    /// The kind of the last rejected attempt after the last accepted step.
    pub last_failure: Option<StepFailureKind>,
}

/// Which driver ran and why.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct RouteRecord {
    /// The public entry point of `rodas5p-integrators` the facade called.
    pub entry_point: &'static str,
    /// The driver identifier the driver reported.
    pub driver_id: &'static str,
    /// The router's decision record, when the router was called.
    pub routing: Option<RoutingDecisionRecord>,
}

/// The full record of a run: every option and every work counter, so that a
/// run is reproducible from its record alone. Serializes with serde.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct Stats {
    /// [`STATS_SCHEMA`].
    pub schema: &'static str,
    /// The options as given.
    pub options: Options,
    /// The effective controller configuration passed to the driver. With a
    /// vector [`crate::Atol`] its scalar `atol` is only the largest entry
    /// (a uniform vector therefore records the scalar run's value); the
    /// per-component values the drivers used are in `options.atol`.
    pub adaptive: AdaptiveRecord,
    /// The linear configuration of the matrix-free route, when it ran.
    pub matrix_free_linear: Option<LinearSolverRecord>,
    /// The integration span.
    pub t_span: (f64, f64),
    /// The initial state.
    pub y0: Vec<f64>,
    /// The problem dimension.
    pub dimension: usize,
    /// Whether the problem was declared autonomous.
    pub autonomous: bool,
    /// Whether an explicit time derivative was supplied.
    pub time_derivative: bool,
    /// The Jacobian provider.
    pub jacobian: JacobianRecord,
    /// The tier the options requested.
    pub tier_requested: Tier,
    /// The tier that executed (always `Protected` in this release).
    pub tier_executed: Tier,
    /// The driver that ran.
    pub route: RouteRecord,
    /// Step attempts (accepted and rejected).
    pub attempts: usize,
    /// Accepted steps.
    pub accepted_steps: usize,
    /// Rejected steps (every kind of rejection).
    pub rejected_steps: usize,
    /// Internal steps the driver reported.
    pub internal_steps: usize,
    /// Steps shortened to land on an output time.
    pub output_clipped_steps: usize,
    /// All work: the driver's counters plus the finite-difference
    /// right-hand-side calls (`rhs_calls`, `rhs_evaluations`).
    pub counters: WorkRecord,
    /// The driver's own counters, as the driver returned them (including any
    /// band-verification charge of the router).
    pub driver_counters: WorkRecord,
    /// The driver-specific counters and policy ids.
    pub driver_work: DriverWork,
    /// Finite-difference Jacobian work.
    pub finite_difference: FiniteDifferenceWork,
    /// Callback error accounting.
    pub callbacks: CallbackWork,
    /// Where and why the run stopped, when it stopped before `t1`.
    pub stop: Option<StopRecord>,
    /// The driver's message.
    pub driver_message: String,
    /// The terminal event that ended the outputs: an index into
    /// [`Solution::events`], not a declaration index. The integration itself
    /// ran to its natural end, and the work after the event stays charged in
    /// the counters above.
    pub terminal_event: Option<usize>,
    /// Event-function evaluations (the located events' post-processing; not
    /// part of [`Stats::counters`]).
    pub event_evaluations: u64,
}

/// The solution of a run: output times, states, the run record and the
/// status.
///
/// A failed run is returned as a `Solution` too, with
/// [`Status::Failed`] and the outputs it reached; use
/// [`Solution::ensure_success`] to turn it into an error.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct Solution {
    /// Output times. Without [`crate::Options::dense_output`]: `t0` and
    /// `t1`, or `t0` and the last accepted time for a failed run. With it:
    /// `t0` and every accepted step's end, up to `t1` or the last accepted
    /// time, or `t_e` after a terminal event.
    pub t: Vec<f64>,
    /// States at [`Solution::t`].
    pub y: Vec<Vec<f64>>,
    /// The run record.
    pub stats: Stats,
    /// How the run ended.
    pub status: Status,
    /// The located events, in time order (see [`crate::Problem::event`]).
    /// Only sign changes between the three samples of an accepted step are
    /// seen: an even number of crossings in a half interval, and a tangent
    /// touch, are missed.
    pub events: Vec<EventRecord>,
    /// The accepted steps' continuous extension, under dense output.
    #[serde(skip)]
    pub(crate) extension: Option<Extension>,
}

/// The continuous extension of a dense-output run, in the store of the
/// driver that ran.
// One value per solve: the size difference between the variants is not
// worth a box.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub(crate) enum Extension {
    /// The sequential dense driver's segments (mass matrix or `Enforce`).
    Sequential(Vec<Rodas5pDenseSegment>),
    /// The fast dense drivers' flat store.
    Fast(FastDenseSegments),
}

impl Extension {
    /// The number of kept segments.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Sequential(segments) => segments.len(),
            Self::Fast(segments) => segments.len(),
        }
    }

    /// Whether no segment is kept.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The extension at `t` on the first segment whose end is at least `t`
    /// (`None` when there is none or the evaluation fails).
    fn evaluate(&self, t: f64) -> Option<Vec<f64>> {
        match self {
            Self::Sequential(segments) => {
                let index = segments.partition_point(|s| s.t_new < t);
                segments.get(index)?.evaluate(t).ok()
            }
            Self::Fast(segments) => {
                let mut out = vec![0.0; segments.dimension()];
                segments.evaluate_into(t, &mut out).ok()?;
                Some(out)
            }
        }
    }

    /// Drop the segments that start at or after `t`.
    fn truncate_after(&mut self, t: f64) {
        match self {
            Self::Sequential(segments) => {
                let keep = segments.partition_point(|s| s.t_old < t);
                segments.truncate(keep);
            }
            Self::Fast(segments) => {
                // times()[k] is the start of step k.
                let keep = segments.times()[..segments.len()].partition_point(|&t_old| t_old < t);
                segments.truncate(keep);
            }
        }
    }
}

impl Solution {
    /// Whether the run reached `t1` or a terminal event ended the outputs.
    pub fn is_success(&self) -> bool {
        self.status == Status::Success
    }

    /// The last output state.
    pub fn final_state(&self) -> Option<&[f64]> {
        self.y.last().map(Vec::as_slice)
    }

    /// The state at `t`.
    ///
    /// With [`crate::Options::dense_output`] this evaluates the RODAS5P
    /// continuous extension of the accepted step that contains `t`, for any
    /// `t` between `t0` and the last output time (at an accepted step's end,
    /// bit for bit the stored state). Without dense output it returns
    /// the stored state when `t` is one of [`Solution::t`]. Otherwise, and
    /// outside the integrated span, `None`.
    pub fn at(&self, t: f64) -> Option<Vec<f64>> {
        if let Some(segments) = &self.extension {
            let first = self.t.first().copied()?;
            // Beyond the last output (a terminal event truncates inside a
            // step) the kept segment must not be extrapolated.
            if t > *self.t.last()? {
                return None;
            }
            if t == first {
                return self.y.first().cloned();
            }
            return segments.evaluate(t);
        }
        self.t
            .iter()
            .position(|&time| time == t)
            .map(|index| self.y[index].clone())
    }

    /// Drop everything after `t`: the outputs with a time above `t` and the
    /// segments that start at or after `t`. The segment that contains `t`
    /// stays whole, and [`Solution::at`] is bounded by the last output.
    pub(crate) fn truncate_after(&mut self, t: f64) {
        let keep = self.t.partition_point(|&time| time <= t).max(1);
        self.t.truncate(keep);
        self.y.truncate(keep);
        if let Some(extension) = &mut self.extension {
            extension.truncate_after(t);
        }
    }

    /// `Ok(self)` for a successful run, otherwise
    /// [`SolveError::Failed`] carrying the reason and this partial solution.
    pub fn ensure_success(self) -> Result<Self, SolveError> {
        match &self.status {
            Status::Success => Ok(self),
            Status::Failed(reason) => Err(SolveError::Failed {
                reason: reason.clone(),
                partial: Box::new(self),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CallbackError, Problem, Rodas5p};

    fn solve() -> Solution {
        let problem = Problem::new(1, |_t, y, dy| {
            dy[0] = -y[0];
            Ok::<(), CallbackError>(())
        })
        .autonomous();
        let opts = Options {
            dense_output: true,
            ..Options::default()
        };
        Rodas5p::solve(&problem, (0.0, 3.0), &[1.0], &opts).unwrap()
    }

    #[test]
    fn at_is_none_beyond_the_last_output() {
        let mut sol = solve();
        let last = *sol.t.last().unwrap();
        assert!(sol.at(last).is_some());
        assert!(sol.at(f64::from_bits(last.to_bits() + 1)).is_none());
        // Shrinking the last output shrinks the domain of `at`.
        let k = sol.t.len() / 2;
        let mid = 0.5 * (sol.t[k] + sol.t[k + 1]);
        let inside = 0.5 * (sol.t[k - 1] + sol.t[k]);
        let before = sol.at(inside).unwrap();
        sol.t.truncate(k + 1);
        sol.y.truncate(k + 1);
        assert!(sol.at(mid).is_none());
        assert_eq!(sol.at(inside).unwrap(), before);
    }

    #[test]
    fn truncate_after_inside_a_step() {
        let mut sol = solve();
        let full = sol.clone();
        let k = sol.t.len() / 2;
        let te = 0.5 * (sol.t[k] + sol.t[k + 1]);
        sol.truncate_after(te);
        // Outputs up to t_k stay; the segment containing te stays whole.
        assert_eq!(sol.t, full.t[..=k]);
        assert_eq!(sol.y, full.y[..=k]);
        assert_eq!(sol.extension.as_ref().unwrap().len(), k + 1);
        // Nothing past the last output is evaluated, below it all is bitwise
        // the untruncated value.
        sol.t.push(te);
        sol.y.push(full.at(te).unwrap());
        assert!(sol.at(f64::from_bits(te.to_bits() + 1)).is_none());
        for &t in &[0.0, 0.3 * te, full.t[k], te] {
            assert_eq!(sol.at(t), full.at(t), "t = {t}");
        }
    }

    #[test]
    fn truncate_after_on_a_step_boundary() {
        let mut sol = solve();
        let full = sol.clone();
        let k = 3;
        sol.truncate_after(full.t[k]);
        assert_eq!(sol.t, full.t[..=k]);
        assert_eq!(sol.extension.as_ref().unwrap().len(), k);
        assert_eq!(sol.at(full.t[k]), full.at(full.t[k]));
        assert!(sol.at(full.t[k + 1]).is_none());
    }

    /// Ruling R-C3: `truncate_after` and the `at` bound on both stores.
    #[test]
    fn truncate_after_on_both_extension_stores() {
        let problem = Problem::new(1, |_t, y, dy| {
            dy[0] = -y[0];
            Ok::<(), CallbackError>(())
        })
        .autonomous();
        for interior_error in [
            crate::InteriorError::WFilteredEnforce,
            crate::InteriorError::Enforce,
        ] {
            let opts = Options {
                dense_output: true,
                interior_error,
                ..Options::default()
            };
            let mut sol = Rodas5p::solve(&problem, (0.0, 3.0), &[1.0], &opts).unwrap();
            let fast = matches!(sol.extension, Some(Extension::Fast(_)));
            assert_eq!(fast, interior_error != crate::InteriorError::Enforce);
            let full = sol.clone();
            let k = sol.t.len() / 2;
            let te = 0.5 * (sol.t[k] + sol.t[k + 1]);
            sol.truncate_after(te);
            assert_eq!(sol.t, full.t[..=k]);
            assert_eq!(sol.extension.as_ref().unwrap().len(), k + 1);
            assert!(sol.at(te).is_none());
            sol.t.push(te);
            sol.y.push(full.at(te).unwrap());
            assert_eq!(sol.at(te), full.at(te));
            assert!(sol.at(f64::from_bits(te.to_bits() + 1)).is_none());
        }
    }

    #[test]
    fn truncate_after_edges() {
        let mut sol = solve();
        let full = sol.clone();
        sol.truncate_after(f64::INFINITY);
        assert_eq!(sol.t, full.t);
        assert_eq!(
            sol.extension.as_ref().unwrap().len(),
            full.extension.as_ref().unwrap().len()
        );
        sol.truncate_after(-1.0);
        assert_eq!(sol.t, full.t[..1]);
        assert!(sol.extension.as_ref().unwrap().is_empty());
        assert_eq!(sol.at(0.0), full.at(0.0));
        assert!(sol.at(1e-3).is_none());
    }
}
