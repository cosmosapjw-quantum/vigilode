//! The result of a solve: states, status and the full run record.

use std::fmt;

use rodas5p_core::{LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveFailureKind, AdaptiveStepConfig, BandedWork, DenseErrorControl, DriverStopReason,
    InteriorSampling, Rodas5pDenseSegment, Rodas5pFastLu, RoutingRecord,
};
use serde::Serialize;

use crate::{
    error::SolveError,
    options::{Options, Tier},
};

/// Schema tag of [`Stats`] (bumped when a field changes meaning).
pub const STATS_SCHEMA: &str = "vigilode.stats.v2";

/// Which user callback reported an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
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
pub enum FailureReason {
    /// The attempt budget [`Options::max_steps`] was used up.
    MaxStepsReached {
        /// The budget.
        max_steps: usize,
    },
    /// The step size fell below the drivers' minimum step after repeated
    /// rejections by the local error test (embedded or interior).
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
                "step size fell below the minimum step {min_step:e} after local error rejections"
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
/// `Success` means the run reached `t1` with every accepted step passing the
/// embedded error *estimate*; it is not a certificate of the global error.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum Status {
    /// The run reached `t1`.
    Success,
    /// The run stopped early; the solution holds what was computed.
    Failed(FailureReason),
}

/// The Jacobian provider a run used.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
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
pub const FD_INCREMENT_RULE: &str = "sqrt(eps)*max(|y_j|,1), represented as (y_j+inc)-y_j";

/// Work of the finite-difference Jacobian (zero for user Jacobians).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FiniteDifferenceWork {
    /// Jacobian (or band) builds attempted.
    pub jacobian_builds: u64,
    /// Right-hand-side calls made by those builds (attempted, including
    /// failed ones); already added to [`Stats::counters`].
    pub rhs_calls: u64,
}

/// What the facade observed of the user callbacks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CallbackWork {
    /// Callbacks that returned [`crate::CallbackError::Recoverable`].
    pub recoverable_errors: u64,
    /// Callbacks that returned [`crate::CallbackError::Fatal`] (at most one
    /// enters user code; see `short_circuited_calls`).
    pub fatal_errors: u64,
    /// Calls after a fatal error that the facade answered with an error
    /// without entering user code. The drivers still count them as attempted
    /// calls in their counters.
    pub short_circuited_calls: u64,
    /// The right-hand-side calls among `short_circuited_calls`: user RHS code
    /// was entered `counters.rhs_calls - short_circuited_rhs_calls` times.
    pub short_circuited_rhs_calls: u64,
}

/// Krylov start counts of a matrix-free run (the driver's opt-in SP04
/// start telemetry, copied field by field).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
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
pub enum DriverWork {
    /// The dense fast driver (`Rodas5pFastResult`).
    Dense {
        /// Attempts that reused the Jacobian, `f(t, y)` and `f_t` of a
        /// rejected attempt from the same state.
        jacobian_reuses: usize,
        /// The LU variant that ran.
        lu: Rodas5pFastLu,
    },
    /// The banded fast driver (`Rodas5pFastBandedResult`).
    Banded {
        /// Attempts that reused the Jacobian of a rejected attempt.
        jacobian_reuses: usize,
        /// The LU variant that ran.
        lu: Rodas5pFastLu,
        /// Counted band factor and solve operations and storage.
        banded: BandedWork,
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
    /// The sequential adaptive dense driver
    /// (`integrate_adaptive_dense_observed_with_segments`): the mass-matrix
    /// and dense-output route.
    SequentialDense {
        /// The interior error control that ran (`Off` without dense output).
        control: DenseErrorControl,
        /// Which steps the interior estimate was formed on.
        interior_sampling: InteriorSampling,
        /// The stage linear solver of the sequential step.
        linear_solver: LinearSolverConfig,
        /// Interior estimates formed (accepted and rejected steps).
        samples: usize,
        /// Steps the interior estimate rejected.
        interior_rejections: usize,
        /// Steps whose estimate could not be formed.
        unavailable: usize,
        /// The largest interior estimate of an accepted step, in tolerance
        /// units (an estimate, not a bound).
        max_accepted_estimate: Option<f64>,
        /// Work of the estimates (also in [`Stats::driver_counters`] under
        /// the enforcing modes).
        estimate_counters: WorkCounters,
        /// Accepted steps whose continuous extension was kept.
        segments: usize,
    },
}

/// Where and why a run stopped before `t1`, as the driver reported it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StopRecord {
    /// The last accepted time (`t0` when no step was accepted); the last
    /// entry of [`Solution::t`].
    pub t: f64,
    /// Why the driver's loop ended.
    pub reason: DriverStopReason,
    /// The kind of the last rejected attempt after the last accepted step.
    pub last_failure: Option<AdaptiveFailureKind>,
}

/// Which driver ran and why.
#[derive(Clone, Debug, Serialize)]
pub struct RouteRecord {
    /// The public entry point of `rodas5p-integrators` the facade called.
    pub entry_point: &'static str,
    /// The driver identifier the driver reported.
    pub driver_id: &'static str,
    /// The router's decision record, when the router was called.
    pub routing: Option<RoutingRecord>,
}

/// The full record of a run: every option and every work counter, so that a
/// run is reproducible from its record alone. Serializes with serde.
#[derive(Clone, Debug, Serialize)]
pub struct Stats {
    /// [`STATS_SCHEMA`].
    pub schema: &'static str,
    /// The options as given.
    pub options: Options,
    /// The effective controller configuration passed to the driver.
    pub adaptive: AdaptiveStepConfig,
    /// The linear configuration of the matrix-free route, when it ran.
    pub matrix_free_linear: Option<LinearSolverConfig>,
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
    pub counters: WorkCounters,
    /// The driver's own counters, as the driver returned them (including any
    /// band-verification charge of the router).
    pub driver_counters: WorkCounters,
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
}

/// The solution of a run: output times, states, the run record and the
/// status.
///
/// A failed run is returned as a `Solution` too, with
/// [`Status::Failed`] and the outputs it reached; use
/// [`Solution::ensure_success`] to turn it into an error.
#[derive(Clone, Debug, Serialize)]
pub struct Solution {
    /// Output times. Without [`crate::Options::dense_output`]: `t0` and
    /// `t1`, or `t0` and the last accepted time for a failed run. With it:
    /// `t0` and every accepted step's end, up to `t1` or the last accepted
    /// time.
    pub t: Vec<f64>,
    /// States at [`Solution::t`].
    pub y: Vec<Vec<f64>>,
    /// The run record.
    pub stats: Stats,
    /// How the run ended.
    pub status: Status,
    /// The accepted steps' continuous extension, under dense output.
    #[serde(skip)]
    pub(crate) extension: Option<Vec<Rodas5pDenseSegment>>,
}

impl Solution {
    /// Whether the run reached `t1`.
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
    /// `t` between `t0` and the last accepted time (bit for bit the value the
    /// driver samples at an output time). Without dense output it returns
    /// the stored state when `t` is one of [`Solution::t`]. Otherwise, and
    /// outside the integrated span, `None`.
    pub fn at(&self, t: f64) -> Option<Vec<f64>> {
        if let Some(segments) = &self.extension {
            let first = self.t.first().copied()?;
            if t == first {
                return self.y.first().cloned();
            }
            let index = segments.partition_point(|s| s.t_new < t);
            let segment = segments.get(index)?;
            return segment.evaluate(t).ok();
        }
        self.t
            .iter()
            .position(|&time| time == t)
            .map(|index| self.y[index].clone())
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
