//! The result of a solve: states, status and the full run record.

use std::fmt;

use rodas5p_core::{LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{AdaptiveStepConfig, RoutingRecord};
use serde::Serialize;

use crate::{
    error::SolveError,
    options::{Options, Tier},
};

/// Schema tag of [`Stats`] (bumped when a field changes meaning).
pub const STATS_SCHEMA: &str = "vigilode.stats.v1";

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
    /// The step size fell below the drivers' minimum step (repeated
    /// rejections, for example by local error or non-finite stages).
    StepSizeTooSmall {
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
            Self::StepSizeTooSmall { min_step } => {
                write!(f, "step size fell below the minimum step {min_step:e}")
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
    /// Finite-difference Jacobian work.
    pub finite_difference: FiniteDifferenceWork,
    /// Callback error accounting.
    pub callbacks: CallbackWork,
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
    /// Output times: `t0` and `t1` (only `t0` for a failed run).
    pub t: Vec<f64>,
    /// States at [`Solution::t`].
    pub y: Vec<Vec<f64>>,
    /// The run record.
    pub stats: Stats,
    /// How the run ended.
    pub status: Status,
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
