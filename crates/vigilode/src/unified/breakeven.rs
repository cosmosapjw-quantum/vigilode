//! Forward break-even pilots: the route of a request that declares nothing.
//!
//! DOPRI5 integrates in a persistent session, one attempt per call. The route never
//! classifies the problem; at geometrically spaced times it asks one question on the
//! trajectory it is integrating: *DOPRI5 just spent `b` attempts, at least the cost of one
//! RODAS5P attempt in every unit, to advance a length `H`. Can one RODAS5P attempt advance
//! `H` from here?* If yes, the attempt is kept as the first step of a RODAS5P leg and RODAS5P
//! continues to the end time (one way). If no, that one attempt is lost and DOPRI5 continues
//! bit for bit as if no question had been asked: a pilot never touches the session.
//!
//! The rules, all without a tunable number except the doubling base 2:
//!
//! * `b` is [`break_even_length`] of the dimension: one fresh finite-difference RODAS5P
//!   attempt costs at most `b` DOPRI5 attempts in right-hand-side calls and in operations.
//! * DOPRI5's attempts count only after it located its step size: its first accepted attempt
//!   that the controller did not cap at the largest growth factor, or that followed a
//!   rejection. The initial step search and the growth ramp from a small `h0` do not count.
//! * Pilot `k` (`k = 0, 1, ...`) is due when DOPRI5 has made at least `b 2^k` attempts and at
//!   least `b` located ones. `H` is the length DOPRI5 advanced in its last `b` attempts (the
//!   step it requested at the oldest of them when it advanced nothing). If the rest of the
//!   span is not longer than `H`, no further pilot is made: DOPRI5 finishes for less.
//! * The pilot is `Rodas5p::solve` from DOPRI5's last accepted state with `h0 = H` and
//!   `max_steps = 1`: the first attempt of a direct RODAS5P run from that state. It is charged
//!   to the counters and to the one attempt budget `Options::max_steps` of the solve.
//! * An accepted pilot hands over (owner decision D3: one way); a lost pilot changes
//!   nothing; a fatal callback inside a pilot ends the solve with its accepted prefix (owner
//!   decision D1); a recoverable callback error only rejects the pilot's attempt.
//!
//! The design and its properties are in
//! `research/production_loop_20261116/router_hyperparameter_free/DESIGN_RECOMMENDED.md`
//! (sections 3 and 4).

use std::collections::VecDeque;

use rodas5p_core::WorkCounters;
use serde::Serialize;

use super::analyzer::{SelectedTier, method_id};
use super::legs::{UnifiedLeg, UnifiedStop, registry_leg};
use crate::explicit::{ExplicitSolution, ExplicitSolveError};
use crate::options::Options;
use crate::problem::Problem;
use crate::session::{SessionAdvance, SessionEpochs};
use crate::solution::{FailureReason, Solution, Status};
use crate::{Dopri5, MethodResult, ProblemRegime, WorkRecord};

/// Right-hand-side calls of one DOPRI5 attempt after the first (FSAL: six new stages).
const DOPRI5_CALLBACKS: u128 = 6;
/// Multiply-adds per component of one DOPRI5 attempt: 20 in the stages, 6 in the error
/// estimate.
const DOPRI5_MULTIPLY_ADDS: u128 = 26;
/// Multiply-adds per component of one RODAS5P attempt: 28 entries of `A`, 28 of `C` and 8 of
/// the solution weights (the coefficient snapshot of the protected method).
const RODAS5P_MULTIPLY_ADDS: u128 = 64;

/// The break-even length `b` of the route for dimension `n`: the least number of DOPRI5
/// attempts that cost at least one fresh finite-difference RODAS5P attempt, both in
/// right-hand-side calls and in operations.
///
/// Per attempt, DOPRI5 makes 6 calls and `52 n` operations; a fresh finite-difference
/// RODAS5P attempt makes `n + 9` calls (`n` Jacobian columns, `f(t, y)` twice, 7 stages),
/// `n + 11` for a non-autonomous problem (the finite-difference time derivative), and
/// `(2 n^3 + 54 n^2 + 384 n) / 3` operations (the LU factorization `2 n^3 / 3`, eight solve
/// pairs `16 n^2`, the finite-difference assembly `2 n^2`, 64 multiply-adds per component).
/// `b` is the larger of the two ratios rounded up; the operation unit binds for every
/// `n >= 1`, so `b = ceil((2 n^2 + 54 n + 384) / 156)`: 3, 4, 4, 5, 7, 8, 78 for
/// `n = 1, 2, 3, 4, 8, 10, 64`. The LU count is an upper bound of the dense kernel, which
/// skips zeros, so `b` is conservative for a sparse Jacobian stored dense.
pub fn break_even_length(n: usize) -> usize {
    let n = n as u128;
    let callbacks = (n + 11).div_ceil(DOPRI5_CALLBACKS);
    let operations = (2 * n * n + 54 * n + 384).div_ceil(2 * DOPRI5_MULTIPLY_ADDS * 3);
    usize::try_from(callbacks.max(operations)).unwrap_or(usize::MAX)
}

/// Three times the operations of `attempts` DOPRI5 attempts (`52 n` each).
fn dopri5_operations3(n: usize, attempts: usize) -> u128 {
    3 * 2 * DOPRI5_MULTIPLY_ADDS * n as u128 * attempts as u128
}

/// Three times the operations of a RODAS5P run from its record: `128 n` per attempt,
/// `2 n^3 / 3` per factorization, `2 n^2` per solve pair and per finite-difference build.
fn rodas5p_operations3(n: usize, attempts: usize, counters: &WorkRecord) -> u128 {
    let n = n as u128;
    3 * 2 * RODAS5P_MULTIPLY_ADDS * n * attempts as u128
        + 2 * n.pow(3) * u128::from(counters.direct_factorizations)
        + 6 * n * n * (u128::from(counters.linear_solves) + u128::from(counters.jacobian_builds))
}

/// What one pilot showed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PilotVerdict {
    /// Its attempt was accepted: it is the first step of the RODAS5P leg (handover).
    Accepted,
    /// Its attempt was not accepted: the attempt is lost and DOPRI5 continues unchanged.
    Lost,
    /// A callback failed fatally inside it: the solve ends (owner decision D1).
    Fatal,
    /// `Rodas5p::solve` refused the request before any callback; no further pilot is made.
    Refused,
}

/// One pilot of the break-even route, written after it ran.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct PilotRecord {
    /// `k`: the pilot was due when DOPRI5 had made `b 2^k` attempts.
    pub index: usize,
    /// DOPRI5 attempts made when the pilot ran.
    pub dopri5_attempts: usize,
    /// `t_D`: DOPRI5's last accepted time, where the pilot started.
    pub t: f64,
    /// `H`: the pilot's step, the length DOPRI5 advanced in its last `b` attempts.
    pub h: f64,
    /// `b`: the DOPRI5 attempts the pilot is compared with.
    pub window_attempts: usize,
    /// Right-hand-side calls DOPRI5 made in those attempts (failed calls included).
    pub window_callbacks: u64,
    /// Operations of those attempts, `52 n` each.
    pub window_operations: f64,
    /// Attempts the pilot made.
    pub pilot_attempts: usize,
    /// Charged right-hand-side and time-derivative calls of the pilot.
    pub pilot_callbacks: u64,
    /// Operations of the pilot: `128 n` per attempt, `2 n^3 / 3` per factorization, `2 n^2`
    /// per solve pair and per finite-difference build.
    pub pilot_operations: f64,
    /// What it showed.
    pub verdict: PilotVerdict,
    /// Why its run ended without an accepted step: the stop of its `Rodas5p::solve` call
    /// or its refusal. `None` for an accepted pilot.
    pub stop: Option<UnifiedStop>,
}

/// How the DOPRI5 session and its pilots ended.
pub(super) enum Ending {
    /// DOPRI5 reached the end time or stopped by itself (a callback, a numerical stop, its
    /// attempt budget); its record says which.
    Dopri5,
    /// DOPRI5 and the lost pilots used up the attempt budget of the solve.
    Budget,
    /// A pilot was accepted: its leg record and its solution.
    Handover(Box<(UnifiedLeg, Solution)>),
    /// A callback failed fatally inside a pilot: the stop of its run.
    Fatal(UnifiedStop),
    /// The session refused a call (not expected: the route's target and epochs are fixed).
    Session(String),
}

/// The DOPRI5 session's record and what the pilots did.
pub(super) struct BreakEvenRun {
    /// The break-even length of the run.
    pub b: usize,
    /// DOPRI5's record: the whole run, or its prefix up to the time it was left.
    pub explicit: ExplicitSolution,
    /// Every pilot, in order.
    pub pilots: Vec<PilotRecord>,
    /// Work of the pilots that are not the first step of a RODAS5P leg.
    pub spare_work: WorkCounters,
    /// Their attempts, accepted steps and rejected attempts.
    pub spare_attempts: usize,
    /// See [`BreakEvenRun::spare_attempts`].
    pub spare_accepted: usize,
    /// See [`BreakEvenRun::spare_attempts`].
    pub spare_rejected: usize,
    /// How the session ended.
    pub ending: Ending,
}

/// One DOPRI5 attempt of the window: where it started, the step it asked for, its calls.
#[derive(Clone, Copy)]
struct WindowEntry {
    t_before: f64,
    requested_h: f64,
    callbacks: u64,
}

/// Run DOPRI5 from `(span.0, y0)` with forward break-even pilots under the attempt budget
/// `options.max_steps`. `Err` is a refusal of the DOPRI5 session before any callback.
pub(super) fn run(
    problem: &Problem,
    span: (f64, f64),
    y0: &[f64],
    options: &Options,
) -> Result<BreakEvenRun, ExplicitSolveError> {
    let n = y0.len();
    let b = break_even_length(n);
    let total = options.max_steps;
    let epochs = SessionEpochs::default();
    let mut pilots: Vec<PilotRecord> = Vec::new();
    let mut spare_work = WorkCounters::default();
    let (mut spare_attempts, mut spare_accepted, mut spare_rejected) = (0usize, 0usize, 0usize);

    let (ending, explicit) = Dopri5::with_session(
        problem,
        span,
        y0,
        options,
        &[span.0, span.1],
        epochs,
        |session| {
            session.set_attempt_telemetry(true);
            let max_factor = session.max_factor();
            // a pilot needs b DOPRI5 attempts; the window never holds more than b
            let mut pilots_on = b <= total;
            let mut window: VecDeque<WindowEntry> = VecDeque::new();
            let mut window_callbacks = 0u64;
            let (mut located, mut counted, mut previous_rejected) = (false, 0usize, false);
            let (mut index, mut due) = (0usize, b);
            loop {
                let dopri5_attempts = session.checkpoint().attempts;
                // the session enforces the budget alone while no pilot was made, so that
                // its record is the one of Dopri5::solve
                if spare_attempts > 0 && dopri5_attempts + spare_attempts >= total {
                    return Ending::Budget;
                }
                match session.advance_to(span.1, 1, epochs) {
                    Ok(SessionAdvance::Yield) => {}
                    Ok(_) => return Ending::Dopri5,
                    Err(error) => return Ending::Session(error.to_string()),
                }
                let Some(attempt) = session.last_attempt() else {
                    continue;
                };
                if pilots_on {
                    window.push_back(WindowEntry {
                        t_before: attempt.t_before,
                        requested_h: attempt.requested_h,
                        callbacks: attempt.rhs_delta,
                    });
                    window_callbacks += attempt.rhs_delta;
                    if window.len() > b
                        && let Some(oldest) = window.pop_front()
                    {
                        window_callbacks -= oldest.callbacks;
                    }
                }
                let checkpoint = session.checkpoint();
                // the controller returns trial_h * max_factor exactly when it clamps
                let capped = attempt.accepted
                    && !attempt.clipped
                    && checkpoint.proposed_h == attempt.trial_h * max_factor;
                if located {
                    counted += 1;
                } else if attempt.accepted && (!capped || previous_rejected) {
                    located = true;
                    counted = 1;
                }
                previous_rejected = !attempt.accepted;
                if !(pilots_on && counted >= b && checkpoint.attempts >= due) {
                    continue;
                }
                let Some(first) = window.front().copied() else {
                    continue;
                };
                let t_d = checkpoint.t;
                let mut h = t_d - first.t_before;
                if h <= 0.0 {
                    // DOPRI5 advanced nothing in the window: the step it tried
                    h = first.requested_h;
                }
                if span.1 - t_d <= h {
                    // the horizon: DOPRI5 finishes the rest for less than a pilot shows
                    pilots_on = false;
                    window.clear();
                    continue;
                }
                if checkpoint.attempts + spare_attempts >= total {
                    // no attempt left for a pilot; the next call ends the run
                    continue;
                }
                let y_d = checkpoint.y.to_vec();
                let dopri5_attempts = checkpoint.attempts;
                let mut pilot_options = options.clone();
                pilot_options.h0 = h;
                pilot_options.max_steps = 1;
                let pilot = registry_leg(
                    problem,
                    SelectedTier::TierProtectedReference,
                    method_id(ProblemRegime::Stiff),
                    (t_d, span.1),
                    &y_d,
                    &pilot_options,
                );
                let mut record = PilotRecord {
                    index,
                    dopri5_attempts,
                    t: t_d,
                    h,
                    window_attempts: window.len(),
                    window_callbacks,
                    window_operations: dopri5_operations3(n, window.len()) as f64 / 3.0,
                    pilot_attempts: 0,
                    pilot_callbacks: 0,
                    pilot_operations: 0.0,
                    verdict: PilotVerdict::Refused,
                    stop: None,
                };
                index += 1;
                due = due.saturating_mul(2);
                let (leg, native) = match pilot {
                    Ok((leg, MethodResult::Rodas5p(native))) => (leg, native),
                    Ok((leg, _)) => {
                        record.stop = Some(UnifiedStop::Other {
                            message: format!(
                                "the registry ran {} for a RODAS5P pilot",
                                leg.driver_id
                            ),
                        });
                        pilots.push(record);
                        pilots_on = false;
                        continue;
                    }
                    Err(error) => {
                        record.stop = Some(UnifiedStop::Refused {
                            tier: SelectedTier::TierProtectedReference,
                            error: error.to_string(),
                            prior: None,
                        });
                        pilots.push(record);
                        pilots_on = false;
                        continue;
                    }
                };
                let counters = &leg.counters;
                record.pilot_attempts = leg.attempts;
                record.pilot_callbacks = counters.rhs_calls + counters.ft_calls;
                record.pilot_operations =
                    rodas5p_operations3(n, leg.attempts, counters) as f64 / 3.0;
                let fatal = matches!(
                    native.status,
                    Status::Failed(FailureReason::CallbackFailed { .. })
                );
                if !fatal && leg.t_end > t_d {
                    record.verdict = PilotVerdict::Accepted;
                    pilots.push(record);
                    return Ending::Handover(Box::new((leg, native)));
                }
                spare_work.accumulate(WorkCounters::from(&leg.counters));
                spare_attempts += leg.attempts;
                spare_accepted += leg.accepted_steps;
                spare_rejected += leg.rejected_steps;
                record.stop = leg.stop.clone();
                if fatal {
                    record.verdict = PilotVerdict::Fatal;
                    pilots.push(record);
                    let stop = leg.stop.unwrap_or_else(|| UnifiedStop::Rodas5p {
                        reason: match native.status {
                            Status::Failed(reason) => reason,
                            Status::Success => FailureReason::MaxStepsReached { max_steps: 1 },
                        },
                    });
                    return Ending::Fatal(stop);
                }
                record.verdict = PilotVerdict::Lost;
                pilots.push(record);
            }
        },
    )?;
    Ok(BreakEvenRun {
        b,
        explicit,
        pilots,
        spare_work,
        spare_attempts,
        spare_accepted,
        spare_rejected,
        ending,
    })
}
