//! Speculative next-step Jacobian client, stage a: sequential emulation
//! (loop node V07).
//!
//! In RODAS5P the arguments of stage 7 and stage 8 of a step from `u_n`,
//! `g7` and `g8`, approximate the next state `u_{n+1}` with order 3 and 4
//! (`g8` is the embedded solution). A driver that evaluates the Jacobian of
//! the next step at `g7` or `g8`, instead of at the accepted `u_{n+1}`,
//! could run that evaluation on another core while the last one or two
//! stages are still being solved. This module answers, without threads,
//! what the change does to the method. Nothing here runs concurrently and
//! nothing here is faster than the exact client.
//!
//! # Parts
//!
//! * [`StageArgumentRecorder`] and its right-hand-side wrapper
//!   ([`StageArgumentRecorder::wrap_rhs`]). `OdeProblem` keeps its callbacks
//!   private, so the wrapper is applied to the right-hand-side callback
//!   before the problem is built. It forwards every call unchanged and
//!   copies the time and state arguments of the first seven calls after
//!   each [`StageLinearSystem::factor`]: in the fast driver these are the
//!   evaluations of stages 2 to 8 of that attempt, in this order, and no
//!   other right-hand-side call falls between `factor` and the end of the
//!   attempt (`f(t, y)` and the time derivative are evaluated before
//!   `factor`). `tests/stage_clients_speculative_contracts.rs` proves the
//!   assignment by re-computing every stage argument of every accepted
//!   step.
//! * [`SpeculativeJacobianClient`], a [`StageLinearSystem`] around any
//!   inner client. It forwards `factor` and `solve_in_place`, and it
//!   forwards `linearize` with the point of its [`SpeculationMode`] in
//!   place of the accepted state the driver hands over.
//!
//! # What is evaluated where
//!
//! The driver calls `linearize(t, y)` with the accepted state `(t_{n+1},
//! u_{n+1})` of the step that just ended. The modes evaluate the Jacobian
//! at
//!
//! * [`SpeculationMode::Exact`]: `(t, y)` as given. Control mode: the run is
//!   the inner client's run, bit for bit.
//! * [`SpeculationMode::G8`]: the argument of stage 8 of the accepted
//!   attempt.
//! * [`SpeculationMode::G7`]: the argument of stage 7 of the accepted
//!   attempt.
//! * [`SpeculationMode::LaggedOneStep`]: the previous accepted state `(t_n,
//!   u_n)`. Not a speculation that could overlap anything: the contrast
//!   case of a Jacobian that is one full step old.
//!
//! The exact Jacobian is used instead
//!
//! 1. at the first linearization of a run (there is no previous attempt);
//! 2. when the driver linearizes at the time of the previous linearization.
//!    It does so only for the retry after an attempt that failed with an
//!    error (`LinearSolve`, `NonlinearSolve`, `NonFinite`). The failed
//!    attempt may have failed because of its linearization point, and a
//!    threaded driver holds the accepted state at that moment with nothing
//!    to overlap;
//! 3. when the recorder holds no complete attempt whose stage time is the
//!    linearization time, bit for bit (an extra right-hand-side call
//!    through the wrapper between two attempts, a client reused for another
//!    run without [`SpeculativeJacobianClient::reset`], a wrapper of
//!    another problem).
//!
//! [`SpeculationStats`] counts the linearizations of each kind.
//!
//! Local-error rejection. The driver keeps its linearization: before the
//! retry it calls `factor` with the smaller step, not `linearize`. The retry
//! therefore uses the Jacobian of the rejected attempt, which is the
//! speculative one (from the last accepted attempt) or the exact one of a
//! fallback. The stage arguments of the rejected attempt are overwritten by
//! those of the retry and are never used: they approximate a state that was
//! not accepted. Once a retry is accepted, the next linearization uses its
//! stage arguments.
//!
//! Time argument. The speculative Jacobian is evaluated at the time
//! argument of the recorded stage evaluation, `t_n + c_i h`. Stages 7 and 8
//! have `c_i = 1` ([`StageArgumentRecorder::new`] checks it), and the driver
//! advances its clock by the same sum, so this is the accepted time
//! `t_{n+1}` bit for bit; rule 3 falls back when it is not. The lagged mode
//! uses `t_n`.
//!
//! Time derivative. For a non-autonomous problem the driver evaluates
//! `f_t` itself, at the accepted `(t_{n+1}, u_{n+1})`, on every attempt from
//! a fresh state: through the problem's `partial_t` callback or, without
//! one, as a central difference of two right-hand-side calls in `t`. It is
//! not part of [`StageLinearSystem`]. Like `f(t_{n+1}, u_{n+1})` it is
//! exact in every mode; only the Jacobian is speculative.
//!
//! Allocation: every buffer is allocated at construction; recording,
//! linearizing and publishing the statistics allocate nothing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use rodas5p_core::{CoreError, CoreResult, WorkCounters, rodas5p_coefficients};

use crate::{OdeProblem, StageLinearSystem};

/// Identifier of runs made with [`SpeculativeJacobianClient`] in research
/// records. The client driver itself reports
/// [`crate::RODAS5P_FAST_CLIENT_DRIVER_ID`]; no route selects this client.
pub const SPECULATIVE_JACOBIAN_STAGE_A_ID: &str = "rodas5p-fast-client-speculative-jacobian-a1";

/// The right-hand-side callback of [`OdeProblem::new`].
pub type StageRhs = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// What the wrapper has seen since the last
/// [`StageArgumentRecorder::begin_attempt`].
struct Recorded {
    /// Whether an attempt has begun at all.
    armed: bool,
    /// Right-hand-side calls since the attempt began (saturating).
    calls: usize,
    /// A call since the attempt began carried a state of another length.
    shape_mismatch: bool,
    /// Time arguments of the first `evaluations` calls.
    times: Vec<f64>,
    /// State arguments of the first `evaluations` calls, `n` entries each.
    states: Vec<f64>,
    /// Right-hand-side calls through the wrapper, ever.
    total_calls: u64,
    /// Attempts begun, ever.
    attempts: u64,
}

/// The arguments of the stage evaluations of the current attempt, as seen
/// by a result-neutral wrapper around the right-hand side. See the module
/// documentation.
pub struct StageArgumentRecorder {
    n: usize,
    /// Stage evaluations of one attempt: every stage but the first.
    evaluations: usize,
    recorded: Mutex<Recorded>,
}

impl StageArgumentRecorder {
    /// A recorder for states of `n` entries. Refuses `n == 0` and a tableau
    /// whose last two stages are not evaluated at the end of the step
    /// (`c_7 = c_8 = 1`), which the client's time rule relies on.
    pub fn new(n: usize) -> CoreResult<Arc<Self>> {
        if n == 0 {
            return Err(CoreError::InvalidInput(
                "the stage argument recorder needs a positive dimension".into(),
            ));
        }
        let coefficients = rodas5p_coefficients()?;
        let stages = coefficients.stages();
        if stages < 3 || coefficients.c[stages - 2] != 1.0 || coefficients.c[stages - 1] != 1.0 {
            return Err(CoreError::Coefficients(
                "the speculative Jacobian needs a tableau whose last two stage times are the \
                 end of the step"
                    .into(),
            ));
        }
        let evaluations = stages - 1;
        Ok(Arc::new(Self {
            n,
            evaluations,
            recorded: Mutex::new(Recorded {
                armed: false,
                calls: 0,
                shape_mismatch: false,
                times: vec![0.0; evaluations],
                states: vec![0.0; evaluations * n],
                total_calls: 0,
                attempts: 0,
            }),
        }))
    }

    /// The length of the recorded states.
    pub fn dimension(&self) -> usize {
        self.n
    }

    /// Right-hand-side evaluations the fast driver makes inside the stage
    /// loop of one attempt: stages 2 to `stages` (seven for RODAS5P).
    pub fn stage_evaluations(&self) -> usize {
        self.evaluations
    }

    /// The number of stages of the tableau (eight).
    pub fn stages(&self) -> usize {
        self.evaluations + 1
    }

    fn lock(&self) -> MutexGuard<'_, Recorded> {
        self.recorded.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// `rhs` behind the recorder: every call is forwarded with its own
    /// arguments and its result is returned as it is; before the call the
    /// arguments are recorded. The wrapper never reads or writes `out`.
    pub fn wrap_rhs(self: &Arc<Self>, rhs: StageRhs) -> StageRhs {
        let recorder = Arc::clone(self);
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            recorder.record(t, y);
            rhs(t, y, out)
        })
    }

    fn record(&self, t: f64, y: &[f64]) {
        let n = self.n;
        let mut recorded = self.lock();
        recorded.total_calls = recorded.total_calls.saturating_add(1);
        if !recorded.armed {
            return;
        }
        let call = recorded.calls;
        if call < self.evaluations {
            if y.len() == n {
                recorded.times[call] = t;
                recorded.states[call * n..(call + 1) * n].copy_from_slice(y);
            } else {
                recorded.shape_mismatch = true;
            }
        }
        recorded.calls = call.saturating_add(1);
    }

    /// Start recording a new attempt. [`SpeculativeJacobianClient`] calls it
    /// from [`StageLinearSystem::factor`], which the driver calls once per
    /// attempt, after `f(t, y)` and the time derivative and before the
    /// stage loop.
    pub fn begin_attempt(&self) {
        let mut recorded = self.lock();
        recorded.armed = true;
        recorded.calls = 0;
        recorded.shape_mismatch = false;
        recorded.attempts = recorded.attempts.saturating_add(1);
    }

    /// Right-hand-side calls since the last [`Self::begin_attempt`] (zero
    /// before the first).
    pub fn calls_in_attempt(&self) -> usize {
        let recorded = self.lock();
        if recorded.armed { recorded.calls } else { 0 }
    }

    /// Whether exactly the stage evaluations of one attempt have been seen
    /// since the last [`Self::begin_attempt`].
    pub fn attempt_is_complete(&self) -> bool {
        let recorded = self.lock();
        recorded.armed && !recorded.shape_mismatch && recorded.calls == self.evaluations
    }

    /// Right-hand-side calls through the wrapper since construction.
    pub fn total_calls(&self) -> u64 {
        self.lock().total_calls
    }

    /// Attempts begun since construction.
    pub fn attempts_begun(&self) -> u64 {
        self.lock().attempts
    }

    /// The argument of stage `stage` (1-based, `2..=stages`) of the current
    /// attempt, copied into `out`, and its time argument; `None` unless the
    /// attempt is complete ([`Self::attempt_is_complete`]), `stage` is in
    /// range and `out` has [`Self::dimension`] entries. `out` is untouched
    /// on `None`.
    pub fn completed_stage_argument_into(&self, stage: usize, out: &mut [f64]) -> Option<f64> {
        let n = self.n;
        let recorded = self.lock();
        if !recorded.armed
            || recorded.shape_mismatch
            || recorded.calls != self.evaluations
            || stage < 2
            || stage > self.evaluations + 1
            || out.len() != n
        {
            return None;
        }
        let slot = stage - 2;
        out.copy_from_slice(&recorded.states[slot * n..(slot + 1) * n]);
        Some(recorded.times[slot])
    }
}

/// Where a [`SpeculativeJacobianClient`] evaluates the Jacobian of a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpeculationMode {
    /// At the accepted state the driver hands over (control).
    Exact,
    /// At the argument of stage 8 of the previous accepted attempt, the
    /// embedded solution (order 4).
    G8,
    /// At the argument of stage 7 of the previous accepted attempt
    /// (order 3).
    G7,
    /// At the previous accepted state: a Jacobian one full step old.
    LaggedOneStep,
}

impl SpeculationMode {
    /// A short lower-case name for records.
    pub fn label(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::G8 => "g8",
            Self::G7 => "g7",
            Self::LaggedOneStep => "lagged",
        }
    }
}

/// The linearizations of a [`SpeculativeJacobianClient`] by the point they
/// used, and its attempts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpeculationStats {
    /// Calls of [`StageLinearSystem::linearize`]: the sum of the five
    /// fields below.
    pub linearizations: u64,
    /// At the point of the mode (never in [`SpeculationMode::Exact`]).
    pub speculative: u64,
    /// At the accepted state because the mode is [`SpeculationMode::Exact`].
    pub exact_by_mode: u64,
    /// At the accepted state: the first linearization of the run.
    pub exact_first: u64,
    /// At the accepted state: a linearization at the time of the previous
    /// one (the driver's retry after an attempt that failed with an error).
    pub exact_after_failure: u64,
    /// At the accepted state: no complete recorded attempt ends at the
    /// linearization time.
    pub exact_no_point: u64,
    /// Calls of [`StageLinearSystem::factor`]: the attempts that reached
    /// their factorization.
    pub attempts: u64,
}

/// The published copy of a client's [`SpeculationStats`].
#[derive(Debug, Default)]
struct SharedStats {
    linearizations: AtomicU64,
    speculative: AtomicU64,
    exact_by_mode: AtomicU64,
    exact_first: AtomicU64,
    exact_after_failure: AtomicU64,
    exact_no_point: AtomicU64,
    attempts: AtomicU64,
}

impl SharedStats {
    fn publish(&self, stats: &SpeculationStats) {
        self.linearizations
            .store(stats.linearizations, Ordering::Relaxed);
        self.speculative.store(stats.speculative, Ordering::Relaxed);
        self.exact_by_mode
            .store(stats.exact_by_mode, Ordering::Relaxed);
        self.exact_first.store(stats.exact_first, Ordering::Relaxed);
        self.exact_after_failure
            .store(stats.exact_after_failure, Ordering::Relaxed);
        self.exact_no_point
            .store(stats.exact_no_point, Ordering::Relaxed);
        self.attempts.store(stats.attempts, Ordering::Relaxed);
    }

    fn snapshot(&self) -> SpeculationStats {
        SpeculationStats {
            linearizations: self.linearizations.load(Ordering::Relaxed),
            speculative: self.speculative.load(Ordering::Relaxed),
            exact_by_mode: self.exact_by_mode.load(Ordering::Relaxed),
            exact_first: self.exact_first.load(Ordering::Relaxed),
            exact_after_failure: self.exact_after_failure.load(Ordering::Relaxed),
            exact_no_point: self.exact_no_point.load(Ordering::Relaxed),
            attempts: self.attempts.load(Ordering::Relaxed),
        }
    }
}

/// A reader of a client's [`SpeculationStats`] that outlives the client's
/// move into the driver (which hands it back type-erased). Read it after
/// the run.
#[derive(Clone, Debug)]
pub struct SpeculationStatsHandle {
    shared: Arc<SharedStats>,
}

impl SpeculationStatsHandle {
    /// The statistics published so far.
    pub fn snapshot(&self) -> SpeculationStats {
        self.shared.snapshot()
    }
}

/// A [`StageLinearSystem`] that linearizes an inner client at the point of
/// a [`SpeculationMode`]. See the module documentation for the rules. One
/// client serves one run; [`Self::reset`] prepares it for another.
pub struct SpeculativeJacobianClient<C> {
    inner: C,
    mode: SpeculationMode,
    recorder: Arc<StageArgumentRecorder>,
    /// The point of a linearization that is not the driver's.
    point: Vec<f64>,
    /// The bits of the time of the previous linearization.
    last_time: Option<u64>,
    /// The accepted state at the previous linearization time and that time
    /// ([`SpeculationMode::LaggedOneStep`] only).
    held: Vec<f64>,
    held_time: f64,
    stats: SpeculationStats,
    shared: Arc<SharedStats>,
}

impl<C: StageLinearSystem> SpeculativeJacobianClient<C> {
    /// `inner` linearized at the point of `mode`, with the stage arguments
    /// taken from `recorder`, whose wrapper must be the right-hand side of
    /// the problem the driver integrates. Refuses a recorder of another
    /// dimension.
    pub fn new(
        inner: C,
        mode: SpeculationMode,
        recorder: Arc<StageArgumentRecorder>,
    ) -> CoreResult<Self> {
        let n = inner.dimension();
        if recorder.dimension() != n {
            return Err(CoreError::InvalidInput(format!(
                "the stage argument recorder has dimension {}, the stage client {n}",
                recorder.dimension()
            )));
        }
        let lagged = if mode == SpeculationMode::LaggedOneStep {
            n
        } else {
            0
        };
        Ok(Self {
            inner,
            mode,
            recorder,
            point: vec![0.0; n],
            last_time: None,
            held: vec![0.0; lagged],
            held_time: 0.0,
            stats: SpeculationStats::default(),
            shared: Arc::new(SharedStats::default()),
        })
    }

    /// The mode of this client.
    pub fn mode(&self) -> SpeculationMode {
        self.mode
    }

    /// A reader of the statistics for after the client has been boxed and
    /// moved into the driver.
    pub fn handle(&self) -> SpeculationStatsHandle {
        SpeculationStatsHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    /// The statistics so far.
    pub fn stats(&self) -> SpeculationStats {
        self.stats
    }

    /// Forget the previous run: the next linearization is a first one (the
    /// exact Jacobian) and the statistics start from zero (the handles see
    /// the reset).
    pub fn reset(&mut self) {
        self.last_time = None;
        self.stats = SpeculationStats::default();
        self.shared.publish(&self.stats);
    }

    /// The wrapped client.
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// The wrapped client.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C: StageLinearSystem> StageLinearSystem for SpeculativeJacobianClient<C> {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    /// The inner client's linearization at the point of the mode, or at
    /// `(t, y)` under the fallback rules of the module documentation. The
    /// counters are the inner client's: one Jacobian per call in every
    /// mode.
    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if y.len() != self.point.len() {
            return Err(CoreError::Dimension(
                "speculative linearization state shape mismatch".into(),
            ));
        }
        let time = t.to_bits();
        let first = self.last_time.is_none();
        let retry = self.last_time == Some(time);
        self.last_time = Some(time);
        self.stats.linearizations += 1;
        // The time of the point in `self.point`, when the mode's point is
        // used.
        let mut point_time = None;
        if self.mode == SpeculationMode::Exact {
            self.stats.exact_by_mode += 1;
        } else if first {
            self.stats.exact_first += 1;
        } else if retry {
            self.stats.exact_after_failure += 1;
        } else if self.mode == SpeculationMode::LaggedOneStep {
            self.point.copy_from_slice(&self.held);
            point_time = Some(self.held_time);
            self.stats.speculative += 1;
        } else {
            let stage = match self.mode {
                SpeculationMode::G8 => self.recorder.stages(),
                _ => self.recorder.stages() - 1,
            };
            match self
                .recorder
                .completed_stage_argument_into(stage, &mut self.point)
            {
                Some(stage_time) if stage_time.to_bits() == time => {
                    point_time = Some(stage_time);
                    self.stats.speculative += 1;
                }
                _ => self.stats.exact_no_point += 1,
            }
        }
        if self.mode == SpeculationMode::LaggedOneStep && !retry {
            // The state the next step's Jacobian is evaluated at.
            self.held.copy_from_slice(y);
            self.held_time = t;
        }
        self.shared.publish(&self.stats);
        match point_time {
            Some(point_time) => self
                .inner
                .linearize(problem, point_time, &self.point, counters),
            None => self.inner.linearize(problem, t, y, counters),
        }
    }

    /// The inner client's factorization; the recorder starts a new attempt.
    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        self.recorder.begin_attempt();
        self.stats.attempts += 1;
        self.shared.publish(&self.stats);
        self.inner.factor(inv_h_gamma, counters)
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        self.inner.solve_in_place(b, counters)
    }
}
