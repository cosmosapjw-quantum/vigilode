//! Causal raw-U RODAS5P solvers and direct-map research implementations.
//!
//! This companion crate does not change VigilODE's default dispatcher. A
//! candidate passes an operational full-space residual screen, NOT a certified
//! nonlinear/output/continuous-time error bound. The caller owns physical-step
//! acceptance. No n-by-n state Jacobian is assembled by this candidate path.
//! The coupled algorithms retain a supplied frozen JVP; `rhs_homotopy` uses
//! direct map values, and `structural_mean_field` uses an exact declared model
//! reduction without derivative callbacks or a state-matrix solve.
#![forbid(unsafe_code)]
mod adaptive;
mod algebra;
pub mod certified_fastfirst;
pub mod cyclic_closure;
mod fallback;
pub mod n01_derivative_free;
pub mod nonlinear_be;
pub mod rhs_homotopy;
mod solver;
pub mod structural_mean_field;
mod switching;
pub mod trace_certificate;
pub mod trace_quotient;
pub use adaptive::{
    COUPLED_ADAPTIVE_DRIVER_ID, CoupledIntegrationResult, CoupledTrajectoryAttempt,
    integrate_causal_observed, integrate_coupled_observed, integrate_sequential_raw_observed,
    integrate_switching_causal_observed,
};
pub use fallback::{
    FallbackResult, SelectedStep, coupled_or_sequential, coupled_or_sequential_budgeted,
};
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use serde::Serialize;
pub use switching::{
    CandidateRoute, CausalReferenceGuard, CausalSwitchPolicy, CausalSwitchReceipt,
    CausalWorkWeights,
};

pub const TARGET_ID: &str = "rodas5p-raw-u-coupled-operational-v1";

#[derive(Clone, Debug, Serialize)]
pub struct Config {
    pub initial_basis: usize,
    pub max_basis: usize,
    pub max_expansions: usize,
    pub max_newton: usize,
    pub max_subdivisions: usize,
    pub max_batches: usize,
    pub max_rhs: u64,
    pub max_jvp: u64,
    /// Conservative plan for principal arrays, not an allocator/RSS bound.
    pub max_storage_bytes: usize,
    pub atol: f64,
    pub rtol: f64,
    pub theta: f64,
    pub projected_relative_tol: f64,
    pub full_residual_wrms_limit: f64,
    pub rank_relative_tol: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            initial_basis: 8,
            max_basis: 32,
            max_expansions: 4,
            max_newton: 8,
            max_subdivisions: 6,
            max_batches: 128,
            max_rhs: 4096,
            max_jvp: 20000,
            max_storage_bytes: 256 * 1024 * 1024,
            atol: 1e-10,
            rtol: 1e-6,
            theta: 1.0,
            projected_relative_tol: 1e-12,
            full_residual_wrms_limit: 1e-4,
            rank_relative_tol: 1e-12,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureKind {
    Input,
    Unsupported,
    Budget,
    Callback,
    NonFinite,
    ProjectionSingular,
    RootIteration,
    ProjectionLeakage,
    ModelChanged,
}
#[derive(Clone, Debug, Serialize)]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
}
impl Failure {
    pub(crate) fn new(kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
pub(crate) type Result<T> = std::result::Result<T, Failure>;

#[derive(Clone, Debug, Default, Serialize)]
pub struct Work {
    pub rhs_attempted: u64,
    pub rhs_completed: u64,
    pub jvp_attempted: u64,
    pub jvp_completed: u64,
    pub basis_jvp: u64,
    pub stage_jvp: u64,
    pub validation_jvp: u64,
    /// Separate derivative dispatches used by the residual-first policy.
    pub derivative_batches: u64,
    /// Converged/exhausted iterates at which no Newton derivative was needed.
    pub residual_only_stops: u64,
    /// Counterfactual eager JVP vectors avoided, not performed work: the
    /// (S - 1) m stage JVPs an eager evaluation performs at that iterate.
    pub unused_stage_jvp_avoided: u64,
    pub stage_batches: u64,
    pub validation_batches: u64,
    pub stage_tasks_scheduled: u64,
    pub stage_tasks_completed: u64,
    pub small_factorizations: u64,
    /// Completed full projected solves (each contains eight block solves).
    pub small_solves: u64,
    /// Actual entries into m-by-m block back-solves, including failed calls.
    pub small_block_solves: u64,
    pub newton_updates: u64,
    /// Newton-free projected fixed-point updates performed by the causal mode.
    pub causal_picard_updates: u64,
    /// Projected nonlinear remainder evaluations reused at the identical
    /// represented input within one fixed model epoch.
    pub causal_rhs_reused: u64,
    /// Common-factor block solves reused from an unchanged causal RHS prefix.
    pub causal_block_solves_reused: u64,
    /// Direct projected stages solved using the common diagonal factorization.
    pub causal_forward_stages: u64,
    pub lambda_rejections: u64,
    pub basis_expansions: u64,
    pub orthogonalization_dots: u64,
    pub peak_parallel_tasks: usize,
    pub maximum_basis: usize,
    pub maximum_reduced_dimension: usize,
    pub planned_storage_bytes: usize,
}
impl Work {
    pub(crate) fn reserve(&self, c: &Config, rhs: u64, jvp: u64) -> Result<()> {
        if self
            .rhs_attempted
            .checked_add(rhs)
            .is_none_or(|v| v > c.max_rhs)
            || self
                .jvp_attempted
                .checked_add(jvp)
                .is_none_or(|v| v > c.max_jvp)
        {
            return Err(Failure::new(
                FailureKind::Budget,
                "callback budget exhausted before scheduling",
            ));
        }
        Ok(())
    }
    pub(crate) fn batch(&mut self, c: &Config, validation: bool) -> Result<()> {
        if self.stage_batches + self.derivative_batches + self.validation_batches
            >= c.max_batches as u64
        {
            return Err(Failure::new(
                FailureKind::Budget,
                "stage-batch budget exhausted",
            ));
        }
        if validation {
            self.validation_batches += 1;
        } else {
            self.stage_batches += 1;
        }
        self.stage_tasks_scheduled += 8;
        Ok(())
    }
    pub(crate) fn derivative_batch(&mut self, c: &Config) -> Result<()> {
        if self.stage_batches + self.derivative_batches + self.validation_batches
            >= c.max_batches as u64
        {
            return Err(Failure::new(
                FailureKind::Budget,
                "stage-batch budget exhausted before derivative dispatch",
            ));
        }
        self.derivative_batches += 1;
        self.stage_tasks_scheduled += 8;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct TracePoint {
    pub basis_dimension: usize,
    pub lambda: f64,
    pub iteration: usize,
    pub projected_relative_residual: Option<f64>,
    pub full_residual_wrms: Option<f64>,
    pub event: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Candidate {
    pub target_id: &'static str,
    pub y_new: Vec<f64>,
    pub stages_u: Vec<Vec<f64>>,
    pub error_vector: Vec<f64>,
    pub embedded_wrms: f64,
    pub full_residual_wrms: f64,
    /// Per-stage Euclidean norms from the existing original-space check.
    /// Retaining these scalars requires no additional RHS/JVP calls.
    pub full_residual_l2: [f64; 8],
    pub full_relative_residual: f64,
    pub basis_dimension: usize,
    /// Merely the raw-U embedded estimate test. It is not a root-error bound.
    pub embedded_test_passes: bool,
    pub accuracy_authority: &'static str,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Attempt {
    pub candidate: Option<Candidate>,
    pub failure: Option<Failure>,
    pub work: Work,
    pub trace: Vec<TracePoint>,
    /// Original-space stages of the last screened but declined candidate.
    pub partial_stages: Option<Vec<Vec<f64>>>,
}

/// One complete, autonomous identity-mass candidate step. The input state and
/// caller's operator are never mutated. All worker outcomes are joined and their
/// attempted work retained even if one callback fails. A changing client epoch
/// declines the attempt; callbacks without an epoch must be pure during a call.
///
/// Full-space residual is necessary, not sufficient for rigorous output accuracy.
/// No implicit assembly, interval certificate or adaptive integration is hidden
/// behind this function. See `coupled_or_sequential` for an explicit fallback.
pub fn coupled_step(
    p: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    cfg: &Config,
    execution: &ParallelExecution,
) -> Attempt {
    let mut out = Attempt::default();
    match solver::run(p, t, y, h, cfg, execution, EvaluationMode::Eager, &mut out) {
        Ok(candidate) => out.candidate = Some(candidate),
        Err(error) => out.failure = Some(error),
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvaluationMode {
    Eager,
    ResidualFirst,
    CausalForward,
    CausalPicard,
    CausalPicardReuse,
}

/// Opt-in residual-first version of `coupled_step` with identical mathematical
/// target and residual/embedded tolerances. An iteration that will not update z
/// does not request a current-stage Jacobian. Full-space verification still runs.
/// Callbacks must be pure at a fixed state/epoch: operation order across phases
/// differs from the eager path. Actual derivative dispatches consume max_batches.
pub fn coupled_step_residual_first(
    p: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    cfg: &Config,
    execution: &ParallelExecution,
) -> Attempt {
    let mut out = Attempt::default();
    match solver::run(
        p,
        t,
        y,
        h,
        cfg,
        execution,
        EvaluationMode::ResidualFirst,
        &mut out,
    ) {
        Ok(candidate) => out.candidate = Some(candidate),
        Err(error) => out.failure = Some(error),
    }
    out
}

mod raw_step;
pub use raw_step::{
    ConditionalErrorEstimate, L2Assumptions, RawSelectedStep, RawStepAttempt, RawStepSession,
    RootErrorPolicy, conditional_l2_error_estimate,
};

/// Schedule for the Newton-free *projected* raw-U target.
/// Frozen JVPs still construct the shared basis and verify the full-space target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CausalSchedule {
    /// One causal forward substitution; recommended for small/single-thread jobs.
    Forward,
    /// Simultaneous nonlinear remainder evaluations, bounded by eight updates.
    /// Uses an affine seed and one cached projected diagonal factorization.
    BatchedPicard,
    /// Same represented Picard recurrence, with bitwise input caches and reuse
    /// of the unchanged prefix in each common-factor triangular solve.
    /// Requires deterministic RHS callbacks at a fixed time/state/model epoch.
    BatchedPicardReuse,
}

/// Solve the same projected raw-U target without current-stage derivatives or
/// Newton updates. `Forward` eliminates the nonlinear causal stages directly;
/// `BatchedPicard` uses affine-preconditioned fixed-point updates with at most
/// eight updates and a final projected residual test. All schedules retain original-space
/// validation and basis expansion. Config::max_newton/max_subdivisions/theta
/// do not control these schedules, which go directly to the lambda=1 target.
/// `BatchedPicardReuse` follows the same represented recurrence and requires
/// deterministic numerical callback results at fixed `(model epoch, t, state)`.
/// Callback-owned observations see its reduced call count. A client must advance
/// its epoch whenever the numerical model changes; without an epoch the same
/// fixed-model assumption remains the caller's contract.
///
/// This is a Newton-free corrector, not a fully derivative-free ODE method:
/// supplied frozen JVPs and a small projected matrix are still used. No automatic
/// derivative fallback is performed. On failure, the input state is unchanged.
pub fn coupled_step_causal(
    p: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    cfg: &Config,
    execution: &ParallelExecution,
    schedule: CausalSchedule,
) -> Attempt {
    let mut out = Attempt::default();
    let mode = match schedule {
        CausalSchedule::Forward => EvaluationMode::CausalForward,
        CausalSchedule::BatchedPicard => EvaluationMode::CausalPicard,
        CausalSchedule::BatchedPicardReuse => EvaluationMode::CausalPicardReuse,
    };
    match solver::run(p, t, y, h, cfg, execution, mode, &mut out) {
        Ok(candidate) => out.candidate = Some(candidate),
        Err(error) => out.failure = Some(error),
    }
    out
}
