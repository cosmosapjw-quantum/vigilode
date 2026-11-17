//! Explicit PPSW1-6b end-to-end adapter (U07), with the adapter-side repairs of audit
//! findings COD-02 and COD-04 (research loop L1, node V03).
//!
//! Never selected by `GeneralOde`. Admission reads the Jacobian provider form of the problem,
//! so what is admitted is what the lowering runs. The Peer six-stage history is method-owned
//! and never shared with another method (no switching API exists).
//!
//! `crates/rodas5p-integrators/src/ppsw1.rs` is protected, so every repair is made here:
//!
//! * the starter interval `[t0, t0 + 2 h0]` is error controlled by step doubling
//!   ([`PeerStarterRecord`]); the first step is halved until the estimate passes;
//! * a first step that is not finite and positive is an input error;
//! * a failed run carries a typed [`PeerFailure`], and a callback error keeps its kind,
//!   callback and message;
//! * a recoverable Jacobian or JVP error stops the run: the linearization point does not
//!   move when the step is halved, so a smaller step cannot cure it;
//! * the end point is landed on exactly, and `success` is the landing flag.

use std::{
    fmt,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::ppsw1;
use serde::Serialize;

use super::{CapabilitySet, MethodAdmission, MethodRegistry, MethodRequest, RegistryRefusal};
use crate::{Callback, CallbackError, Jacobian, Problem};

/// Options of [`PeerOde::solve`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct PeerSolveOptions {
    /// Relative tolerance of the PPSW1 controller.
    pub rtol: f64,
    /// Absolute tolerance of the PPSW1 controller.
    pub atol: f64,
    /// Upper bound passed to the PPSW1 h0 rule (`h0 = min(h_max, 1/||J||)`).
    pub h0_rule_h_max: f64,
    /// Caller-chosen first step instead of the rule (still capped by the endpoint, and
    /// halved by the starter error control when its estimate exceeds the tolerance).
    /// Must be finite and positive.
    pub h0: Option<f64>,
    /// Threads of the stage-parallel RHS/solves (1 = sequential).
    pub threads: usize,
    /// Maximum `try_step` attempts.
    pub max_attempts: usize,
}

impl Default for PeerSolveOptions {
    fn default() -> Self {
        Self {
            rtol: 1e-6,
            atol: 1e-8,
            h0_rule_h_max: f64::INFINITY,
            h0: None,
            threads: 1,
            max_attempts: 1_000_000,
        }
    }
}

/// Why [`PeerOde::solve`] did not start. Returned before any user callback.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum PeerSolveError {
    /// The registry refused PPSW1 for this request.
    #[error("PPSW1 refused by the method registry: {0:?}")]
    Refused(Vec<RegistryRefusal>),
    /// The facade problem uses a Jacobian form the Peer adapter does not lower.
    #[error("PPSW1 adapter needs Jacobian::Dense or Jacobian::Jvp without a band declaration")]
    UnsupportedJacobian,
    /// Invalid span, state or options.
    #[error("invalid PPSW1 input: {0}")]
    InvalidInput(String),
    /// Method switching needs a typed history-transfer certificate that does not exist.
    #[error("switching from PPSW1 history is unsupported: no transfer certificate")]
    MethodSwitchUnsupported,
}

/// Part of a [`PeerOde`] run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PeerPhase {
    /// The `h0 = min(h_max, 1/||J||)` rule (one Jacobian evaluation).
    H0Rule,
    /// The Radau IIA starter and its step-doubling error control.
    Starter,
    /// The PPSW1 step loop.
    Step,
}

impl fmt::Display for PeerPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::H0Rule => "h0 rule",
            Self::Starter => "starter",
            Self::Step => "step",
        })
    }
}

/// Typed reason of a [`PeerOde::solve`] that started and did not reach `t_end`.
/// [`PeerSolution::failure`] is its display text.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[non_exhaustive]
pub enum PeerFailure {
    /// A user callback failed. The kind, the callback and the message are the callback's own.
    Callback {
        /// Where the run was.
        phase: PeerPhase,
        /// The callback that failed.
        callback: Callback,
        /// `true` for [`CallbackError::Recoverable`], `false` for [`CallbackError::Fatal`].
        recoverable: bool,
        /// Time argument of the failing call (a stage or linearization time).
        t: f64,
        /// The callback's message.
        message: String,
    },
    /// The step-doubling estimate of the starter stayed above the tolerance after the
    /// allowed halvings of the first step.
    StarterErrorAboveTolerance {
        /// The last first step tried.
        h0: f64,
        /// Its estimate in tolerance units (`<= 1` is accepted).
        estimate: f64,
        /// Halvings applied.
        halvings: usize,
    },
    /// [`PeerSolveOptions::max_attempts`] step attempts were used up.
    AttemptBudget {
        /// The budget.
        max_attempts: usize,
    },
    /// The integrator stopped for a reason no callback caused (a failed nonlinear or linear
    /// solve of the starter, a step that cannot advance the time, a refused step ratio).
    Integrator {
        /// Where the run was.
        phase: PeerPhase,
        /// Last accepted time (the initial time before the starter completed).
        t: f64,
        /// The integrator's message.
        message: String,
    },
}

impl fmt::Display for PeerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Callback {
                phase,
                callback,
                recoverable,
                t,
                message,
            } => write!(
                f,
                "{} callback error ({callback:?}, {phase} phase, t = {t:e}): {message}",
                if *recoverable { "recoverable" } else { "fatal" }
            ),
            Self::StarterErrorAboveTolerance {
                h0,
                estimate,
                halvings,
            } => write!(
                f,
                "starter error estimate {estimate:e} tolerance units is above 1 after \
                 {halvings} halvings of the first step (h0 = {h0:e})"
            ),
            Self::AttemptBudget { max_attempts } => {
                write!(f, "attempt budget exhausted ({max_attempts} attempts)")
            }
            Self::Integrator { phase, t, message } => {
                write!(f, "{phase} failed at t = {t:e}: {message}")
            }
        }
    }
}

impl std::error::Error for PeerFailure {}

/// Largest number of halvings of the first step by the starter error control.
pub const PEER_STARTER_MAX_HALVINGS: usize = 40;

/// Record of the starter error control of one [`PeerOde::solve`].
///
/// `ppsw1::start` is one 5-stage Radau IIA step over `[t0, t0 + 2 h0]` that is accepted on
/// Newton convergence alone. The adapter adds a step-doubling estimate: the starter state
/// `F` of step `h0` is compared at its five non-initial nodes with two starter solves of
/// step `h0 / 2` (the degree-5 collocation polynomials of the two halves, which pass through
/// their six node values). The estimate is the largest `ppsw1::error_norm` of the five node
/// differences under the caller's tolerances; `F` is used iff it is `<= 1`, otherwise `h0`
/// is halved (the first half becomes the new `F`) up to [`PEER_STARTER_MAX_HALVINGS`] times.
/// It is an estimate, not a bound.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct PeerStarterRecord {
    /// First step from the caller or the h0 rule, after the endpoint cap.
    pub h0_requested: f64,
    /// Halvings applied by the error control.
    pub halvings: usize,
    /// Estimate of the accepted starter in tolerance units (NaN if the control did not finish).
    pub estimate: f64,
    /// The starter interval reaches `t_end`: the solve has no Peer step.
    pub covers_span: bool,
}

/// Result of [`PeerOde::solve`]: accepted-step endpoints and separate work buckets.
#[derive(Clone, Debug)]
pub struct PeerSolution {
    /// Registry admission used.
    pub admission: MethodAdmission,
    /// Accepted output times (starter endpoint `t0 + 2 h0`, then each accepted step). The
    /// last one is `t_end` bit for bit when `success` is true.
    pub t: Vec<f64>,
    /// States at `t` (stage 6 of each accepted step).
    pub y: Vec<Vec<f64>>,
    /// Whether `t_end` was reached (the landing flag of the starter or of the last step).
    pub success: bool,
    /// Display text of `reason` when not successful; the accepted prefix above stays valid.
    pub failure: Option<String>,
    /// Typed failure reason; `Some` exactly when `success` is false.
    pub reason: Option<PeerFailure>,
    /// First step used by the starter (after the error control).
    pub h0: f64,
    /// Starter error control record.
    pub starter: PeerStarterRecord,
    /// Work of the h0 rule and of every starter solve, including the control solves.
    pub startup_work: WorkCounters,
    /// Work of all step attempts, accepted and rejected.
    pub step_work: WorkCounters,
}

// Signatures of `rodas5p_integrators::problem::{RhsFn, JacobianFn, JvpFn}` (not re-exported).
type CoreRhsFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type CoreJacobianFn = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
type CoreJvpFn = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// One callback error as the user reported it.
#[derive(Clone, Debug)]
struct CallbackEvent {
    callback: Callback,
    t: f64,
    message: String,
}

#[derive(Default)]
struct LatchState {
    fatal: Option<CallbackEvent>,
    last_recoverable: Option<CallbackEvent>,
    /// Recoverable errors of every callback.
    recoverable: u64,
    /// Recoverable errors of the Jacobian and JVP callbacks.
    recoverable_linearization: u64,
}

/// Callback boundary of one solve: keeps the original error of a failed callback, and stops
/// entering user code after a fatal one.
#[derive(Default)]
struct Latch {
    poisoned: AtomicBool,
    state: Mutex<LatchState>,
}

impl Latch {
    fn lock(&self) -> MutexGuard<'_, LatchState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn call(
        &self,
        callback: Callback,
        t: f64,
        user: impl FnOnce() -> Result<(), CallbackError>,
    ) -> CoreResult<()> {
        if self.poisoned.load(Ordering::Acquire) {
            return Err(CoreError::FatalCallback(format!(
                "{callback:?} callback not entered: an earlier callback failed fatally"
            )));
        }
        match user() {
            Ok(()) => Ok(()),
            // a recoverable failure rejects the attempt (PPSW1 halves the step)
            Err(CallbackError::Recoverable(message)) => {
                let text = format!("recoverable callback error: {message}");
                let mut state = self.lock();
                state.recoverable += 1;
                if callback != Callback::Rhs {
                    state.recoverable_linearization += 1;
                }
                state.last_recoverable = Some(CallbackEvent {
                    callback,
                    t,
                    message,
                });
                Err(CoreError::NonFinite(text))
            }
            Err(CallbackError::Fatal(message)) => {
                let text = format!("fatal callback error: {message}");
                let mut state = self.lock();
                if state.fatal.is_none() {
                    state.fatal = Some(CallbackEvent {
                        callback,
                        t,
                        message,
                    });
                }
                self.poisoned.store(true, Ordering::Release);
                Err(CoreError::FatalCallback(text))
            }
        }
    }

    /// `(recoverable errors, recoverable Jacobian/JVP errors)` so far.
    fn recoverable_counts(&self) -> (u64, u64) {
        let state = self.lock();
        (state.recoverable, state.recoverable_linearization)
    }

    fn last_recoverable(&self) -> Option<CallbackEvent> {
        self.lock().last_recoverable.clone()
    }

    /// The typed reason of an integrator error: the fatal callback error if one was latched,
    /// the recoverable one if it was raised since `recoverable_before`, the integrator's own
    /// message otherwise.
    fn failure(
        &self,
        phase: PeerPhase,
        t: f64,
        recoverable_before: u64,
        error: &CoreError,
    ) -> PeerFailure {
        let state = self.lock();
        if let Some(event) = &state.fatal {
            return callback_failure(phase, event, false);
        }
        if state.recoverable > recoverable_before
            && let Some(event) = &state.last_recoverable
        {
            return callback_failure(phase, event, true);
        }
        PeerFailure::Integrator {
            phase,
            t,
            message: error.to_string(),
        }
    }
}

fn callback_failure(phase: PeerPhase, event: &CallbackEvent, recoverable: bool) -> PeerFailure {
    PeerFailure::Callback {
        phase,
        callback: event.callback,
        recoverable,
        t: event.t,
        message: event.message.clone(),
    }
}

/// Step-doubling estimate of the starter state `full` (step `h0`) against the two half
/// starters `first` over `[t0, t0 + h0]` and `second` over `[t0 + h0, t0 + 2 h0]`, in
/// tolerance units. NaN when the PPSW1 error norm is undefined (a non-positive scale).
fn starter_estimate(
    full: &ppsw1::State,
    first: &ppsw1::State,
    second: &ppsw1::State,
    tol: &ppsw1::Tolerances,
) -> f64 {
    // node k of a starter state sits at the fraction (1 + c_k) / 2 of its interval
    let fraction = ppsw1::nodes().map(|c| 0.5 * (1.0 + c));
    let n = full.y[0].len();
    let mut reference = vec![0.0; n];
    let mut difference = vec![0.0; n];
    let mut worst = 0.0_f64;
    for k in 1..ppsw1::STAGES {
        let theta = fraction[k];
        let (half, x) = if theta <= 0.5 {
            (first, 2.0 * theta)
        } else {
            (second, 2.0 * theta - 1.0)
        };
        // the half starter's degree-5 collocation polynomial through its six node values;
        // at a node the weights are exactly 0 and 1
        reference.fill(0.0);
        for j in 0..ppsw1::STAGES {
            let mut weight = 1.0;
            for m in 0..ppsw1::STAGES {
                if m != j {
                    weight *= (x - fraction[m]) / (fraction[j] - fraction[m]);
                }
            }
            for (value, node) in reference.iter_mut().zip(&half.y[j]) {
                *value += weight * node;
            }
        }
        for ((delta, value), reference) in difference.iter_mut().zip(&full.y[k]).zip(&reference) {
            *delta = value - reference;
        }
        let estimate = ppsw1::error_norm(&full.y[k], &difference, tol.atol, tol.rtol);
        if estimate.is_nan() {
            return f64::NAN;
        }
        worst = worst.max(estimate);
    }
    worst
}

/// Explicit PPSW1-6b solver over the facade [`Problem`].
pub struct PeerOde;

impl PeerOde {
    /// Label of the registry variant that the lowering runs for this provider form, if any.
    fn variant_label(problem: &Problem) -> Option<&'static str> {
        match (&problem.jacobian, problem.band) {
            (Jacobian::Dense(_), None) => Some("explicit-jacobian-dense"),
            (Jacobian::Jvp(_), None) => Some("jvp-columns-dense"),
            _ => None,
        }
    }

    /// Registry admission for `problem` under `allowed`; pure. Only the variant that the
    /// lowering runs for the problem's Jacobian provider form is considered: a dense
    /// callback needs the explicit-Jacobian variant, a JVP callback the JVP-columns variant.
    /// Any other provider form, and any declared band, is refused with
    /// [`RegistryRefusal::ProviderFormUnsupported`].
    pub fn admission(problem: &Problem, allowed: CapabilitySet) -> MethodAdmission {
        let request = MethodRequest {
            allowed,
            needs_mass: problem.mass.is_some(),
            needs_events: !problem.events.is_empty(),
            needs_dense_output: false,
            admit_unqualified: false,
        };
        let mut descriptor = MethodRegistry::descriptors()
            .into_iter()
            .find(|d| d.method_id == ppsw1::METHOD_ID)
            .expect("PPSW1 descriptor");
        match Self::variant_label(problem) {
            Some(label) => {
                descriptor.variants.retain(|v| v.label == label);
                MethodRegistry::admit(&descriptor, &request)
            }
            None => {
                let mut reasons = vec![RegistryRefusal::ProviderFormUnsupported];
                if let MethodAdmission::Refused { reasons: more, .. } =
                    MethodRegistry::admit(&descriptor, &request)
                {
                    reasons.extend(
                        more.into_iter()
                            .filter(|r| !matches!(r, RegistryRefusal::Permission { .. })),
                    );
                }
                MethodAdmission::Refused {
                    method_id: descriptor.method_id,
                    reasons,
                }
            }
        }
    }

    /// Lower the Peer subset of a facade problem (no callback is made here).
    fn lower(
        problem: &Problem,
        latch: &Arc<Latch>,
    ) -> Result<rodas5p_integrators::OdeProblem, PeerSolveError> {
        let n = problem.dimension;
        if problem.band.is_some() {
            return Err(PeerSolveError::UnsupportedJacobian);
        }
        let rhs: CoreRhsFn = {
            let (user, latch) = (problem.rhs.clone(), latch.clone());
            Arc::new(move |t, y, out| latch.call(Callback::Rhs, t, || user(t, y, out)))
        };
        let (jac, jvp): (Option<CoreJacobianFn>, Option<CoreJvpFn>) = match &problem.jacobian {
            Jacobian::Dense(f) => {
                let (f, latch) = (f.clone(), latch.clone());
                (
                    Some(Arc::new(move |t, y| {
                        let mut m = DenseMatrix::zeros(n, n);
                        latch.call(Callback::Jacobian, t, || f(t, y, m.as_mut_slice()))?;
                        Ok(m)
                    })),
                    None,
                )
            }
            Jacobian::Jvp(f) => {
                let (f, latch) = (f.clone(), latch.clone());
                (
                    None,
                    Some(Arc::new(move |t, y, v, out| {
                        latch.call(Callback::Jvp, t, || f(t, y, v, out))
                    })),
                )
            }
            _ => return Err(PeerSolveError::UnsupportedJacobian),
        };
        rodas5p_integrators::OdeProblem::new(
            "vigilode-peer-adapter",
            n,
            rhs,
            None,
            jac,
            jvp,
            None,
            problem.autonomous,
            None,
            None,
        )
        .map_err(|e| PeerSolveError::InvalidInput(e.to_string()))
    }

    /// Switching away from a Peer history is refused (no transfer certificate).
    pub fn switch_history_to(_method_id: &str) -> Result<(), PeerSolveError> {
        Err(PeerSolveError::MethodSwitchUnsupported)
    }

    /// Refusals that need no callback: admission, then the span, state and options.
    fn validate(
        problem: &Problem,
        (t0, t_end): (f64, f64),
        y0: &[f64],
        options: &PeerSolveOptions,
        allowed: CapabilitySet,
    ) -> Result<MethodAdmission, PeerSolveError> {
        let admission = Self::admission(problem, allowed);
        if let MethodAdmission::Refused { reasons, .. } = &admission {
            // the provider form alone keeps its own error (the lowering refusal of U07)
            if reasons
                .iter()
                .all(|r| matches!(r, RegistryRefusal::ProviderFormUnsupported))
            {
                return Err(PeerSolveError::UnsupportedJacobian);
            }
            return Err(PeerSolveError::Refused(reasons.clone()));
        }
        let invalid = |what: &str| Err(PeerSolveError::InvalidInput(what.into()));
        if !(t0.is_finite() && t_end.is_finite() && t_end > t0) {
            return invalid("the span must be finite with t_end > t0");
        }
        if y0.len() != problem.dimension || !y0.iter().all(|v| v.is_finite()) {
            return invalid("y0 must be finite and of the problem dimension");
        }
        let finite_nonnegative = |v: f64| v.is_finite() && v >= 0.0;
        if !(finite_nonnegative(options.rtol)
            && finite_nonnegative(options.atol)
            && options.rtol + options.atol > 0.0)
        {
            return invalid("rtol and atol must be finite, nonnegative and not both zero");
        }
        if options.threads == 0 {
            return invalid("threads must be positive");
        }
        if let Some(h0) = options.h0
            && !(h0.is_finite() && h0 > 0.0)
        {
            return invalid("h0 must be finite and positive");
        }
        // +infinity (no cap) is the default; NaN and non-positive values are refused
        if options.h0_rule_h_max.is_nan() || options.h0_rule_h_max <= 0.0 {
            return invalid("h0_rule_h_max must be positive");
        }
        Ok(admission)
    }

    /// Integrate `problem` over `span` from `y0`. Refusals happen before any callback; a run
    /// that fails after starting returns `success == false` with its typed
    /// [`PeerSolution::reason`] and its accepted prefix.
    pub fn solve(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &PeerSolveOptions,
        allowed: CapabilitySet,
    ) -> Result<PeerSolution, PeerSolveError> {
        let admission = Self::validate(problem, span, y0, options, allowed)?;
        let (t0, t_end) = span;
        let latch = Arc::new(Latch::default());
        let ode = Self::lower(problem, &latch)?;
        let exec = rodas5p_integrators::ParallelExecution::rayon(options.threads)
            .map_err(|e| PeerSolveError::InvalidInput(e.to_string()))?;
        let mut startup = WorkCounters::default();
        let mut out = PeerSolution {
            admission,
            t: Vec::new(),
            y: Vec::new(),
            success: false,
            failure: None,
            reason: None,
            h0: f64::NAN,
            starter: PeerStarterRecord {
                h0_requested: f64::NAN,
                halvings: 0,
                estimate: f64::NAN,
                covers_span: false,
            },
            startup_work: startup,
            step_work: WorkCounters::default(),
        };
        let fail = |mut out: PeerSolution,
                    startup: WorkCounters,
                    reason: PeerFailure|
         -> Result<PeerSolution, PeerSolveError> {
            out.failure = Some(reason.to_string());
            out.reason = Some(reason);
            out.startup_work = startup;
            Ok(out)
        };
        let tol = ppsw1::Tolerances {
            atol: options.atol,
            rtol: options.rtol,
        };

        // the starter covers [t0, t0 + 2 h0]; never let it pass t_end
        let endpoint_cap = 0.5 * (t_end - t0);
        let mut h0 = match options.h0 {
            Some(h) => h.min(endpoint_cap),
            None => {
                let rule = ppsw1::jacobian_norm_bound(&ode, t0, y0, &mut startup).and_then(|b| {
                    ppsw1::select_h0(b, options.h0_rule_h_max.min(endpoint_cap))
                });
                match rule {
                    Ok(h) => h,
                    Err(e) => {
                        let reason = latch.failure(PeerPhase::H0Rule, t0, 0, &e);
                        return fail(out, startup, reason);
                    }
                }
            }
        };
        out.h0 = h0;
        out.starter.h0_requested = h0;

        // starter with step-doubling error control (see `PeerStarterRecord`)
        let recoverable_before = latch.recoverable_counts().0;
        let starter = |t: f64, y: &[f64], h: f64, startup: &mut WorkCounters| {
            ppsw1::start(&ode, t, y, h, startup)
                .map_err(|e| latch.failure(PeerPhase::Starter, t0, recoverable_before, &e))
        };
        let mut state = match starter(t0, y0, h0, &mut startup) {
            Ok(state) => state,
            Err(reason) => return fail(out, startup, reason),
        };
        let covers_span = loop {
            let half = 0.5 * h0;
            let first = match starter(t0, y0, half, &mut startup) {
                Ok(state) => state,
                Err(reason) => return fail(out, startup, reason),
            };
            let second = match starter(first.t, &first.y[5], half, &mut startup) {
                Ok(state) => state,
                Err(reason) => return fail(out, startup, reason),
            };
            let estimate = starter_estimate(&state, &first, &second, &tol);
            out.starter.estimate = estimate;
            if ppsw1::accepts(estimate) {
                // sized to reach t_end (h0 is the endpoint cap), or reaching it by rounding
                break (out.starter.halvings == 0 && h0 == endpoint_cap) || state.t >= t_end;
            }
            if out.starter.halvings == PEER_STARTER_MAX_HALVINGS {
                let reason = PeerFailure::StarterErrorAboveTolerance {
                    h0,
                    estimate,
                    halvings: out.starter.halvings,
                };
                return fail(out, startup, reason);
            }
            out.starter.halvings += 1;
            h0 = half;
            out.h0 = h0;
            // bit for bit the state `ppsw1::start(t0, y0, h0 / 2)` returns: reuse it
            state = first;
        };
        out.startup_work = startup;
        out.starter.covers_span = covers_span;
        // land on t_end exactly when the starter reaches it
        out.t.push(if covers_span { t_end } else { state.t });
        out.y.push(state.y[5].clone());
        if covers_span {
            out.success = true;
            return Ok(out);
        }

        let mut session = match ppsw1::Session::new(&ode, state, ppsw1::Config::default(), exec) {
            Ok(s) => s,
            Err(e) => {
                let reason = latch.failure(PeerPhase::Step, out.t[0], u64::MAX, &e);
                out.failure = Some(reason.to_string());
                out.reason = Some(reason);
                return Ok(out);
            }
        };
        let mut h = h0;
        let mut attempts = 0usize;
        let mut landed = false;
        // the recoverable callback error behind the current chain of rejections, if any
        let mut pending_recoverable: Option<CallbackEvent> = None;
        let mut reason = None;
        while !landed {
            if attempts >= options.max_attempts {
                reason = Some(PeerFailure::AttemptBudget {
                    max_attempts: options.max_attempts,
                });
                break;
            }
            attempts += 1;
            let t = session.state().t;
            let remaining = t_end - t;
            let last = h >= remaining;
            let hh = h.min(remaining);
            let (recoverable, linearization) = latch.recoverable_counts();
            match session.try_step(hh, &tol) {
                Ok(ppsw1::Decision::Accepted { next_ratio, .. }) => {
                    h = hh * next_ratio;
                    pending_recoverable = None;
                    let t_new = session.state().t;
                    landed = last || t_new >= t_end;
                    out.t.push(if landed { t_end } else { t_new });
                    out.y.push(session.state().y[5].clone());
                }
                Ok(ppsw1::Decision::Rejected { next_ratio, .. }) => {
                    h = hh * next_ratio;
                    let (recoverable_now, linearization_now) = latch.recoverable_counts();
                    if recoverable_now > recoverable {
                        pending_recoverable = latch.last_recoverable();
                    }
                    // the Jacobian (or its JVP columns) is evaluated at the last accepted
                    // state, which a smaller step does not move: stop with that reason
                    if linearization_now > linearization
                        && let Some(event) = &pending_recoverable
                    {
                        reason = Some(callback_failure(PeerPhase::Step, event, true));
                        break;
                    }
                }
                Err(e) => {
                    // read the latch in its own statement: `Latch::failure` locks it again
                    let fatal = latch.lock().fatal.is_some();
                    reason = Some(match &pending_recoverable {
                        // the step size ran out while a recoverable error rejected every
                        // attempt: report that error, not the underflow it caused
                        Some(event) if !fatal => callback_failure(PeerPhase::Step, event, true),
                        _ => latch.failure(PeerPhase::Step, t, recoverable, &e),
                    });
                    break;
                }
            }
        }
        out.step_work = session.counters();
        out.success = landed && reason.is_none();
        out.failure = reason.as_ref().map(ToString::to_string);
        out.reason = reason;
        Ok(out)
    }
}
