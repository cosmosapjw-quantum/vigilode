//! A lane-batched ensemble driver over the small RODAS5P driver (speed
//! research node `research/spd08_small_ensemble_lanes_20261007`). Research
//! only.
//!
//! `B` lanes integrate the members of an ensemble of same-`N` problems
//! concurrently. Each lane owns the scalar state of one member at a time, as
//! [`crate::integrate_rodas5p_fast_small_observed`] does: `(t, h)`, the
//! controller state, the output collector, the counters, the attempt
//! counts and the freshness flag of `J` and `f(t, y)`. Members are taken
//! from the queue in member order whenever a lane becomes free. In each
//! round every lane with a member makes one attempt at its own `(t, h)`.
//!
//! The elementwise stage work (the `A` and `C` combinations with per-lane
//! `c / h` quotients and the same `!= 0.0` guard, the right-hand sides
//! through [`BatchProblem::rhs_batch`], the solution update, the finiteness
//! scans and the error norm) runs over `[[f64; B]; N]` storage, lane
//! innermost. The Jacobian (the scalar [`SmallProblem::jacobian`]), the
//! factorization and the solves run per lane with the small driver's own
//! routines. Every member therefore performs exactly the floating-point
//! operations of the scalar driver in the same order and gets bitwise the
//! same result, step sequence, counters and outputs. A lane whose attempt
//! fails stops taking part in the rest of that attempt with the partial
//! counter updates of the scalar driver; the values the batched arithmetic
//! keeps computing in a stopped or idle lane are never read.
//!
//! The driver mirrors the legacy composition of the small driver
//! (`collector.limit_step`, `collector.accept`,
//! [`rodas_next_step_after_attempt`]), not the opt-in options of SPD01.

use rodas5p_core::{CoreError, CoreResult, WorkCounters};

use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, ObservedIntegrationResult,
    OutputSchedule,
    output::OutputCollector,
    rodas_next_step_after_attempt,
    rodas5p_fast_small::{
        Rodas5pFastSmallResult, STAGES, SmallProblem, Workspace, failure_kind, lu_in_place,
        lu_solve_in_place,
    },
};

/// Identifier of this driver in research records.
pub const RODAS5P_FAST_SMALL_BATCH_DRIVER_ID: &str = "rodas5p-fast-small-batch-v1";

/// A [`SmallProblem`] whose right-hand side can be evaluated for `B` lanes at
/// once over structure-of-arrays storage (`y[component][lane]`).
///
/// `rhs_batch` must perform, in every lane, the operations of
/// [`SmallProblem::rhs`] of that lane's member in the same expression order,
/// so the lanes' values are bitwise those of the scalar calls. Lanes without
/// a member keep the data of an earlier member; their values are ignored.
pub trait BatchProblem<const N: usize, const B: usize>: SmallProblem<N> {
    /// Per-lane data read by [`Self::rhs_batch`] (e.g. parameters as `[f64; B]`).
    type Lanes;
    /// Lane data with every lane holding `member`'s data.
    fn lanes(member: &Self) -> Self::Lanes;
    /// Puts `member`'s data into lane `lane`.
    fn set_lane(lanes: &mut Self::Lanes, lane: usize, member: &Self);
    /// [`SmallProblem::rhs`] in every lane.
    fn rhs_batch(lanes: &Self::Lanes, y: &[[f64; B]; N], out: &mut [[f64; B]; N]);
}

/// One ensemble member: the arguments of one scalar
/// [`crate::integrate_rodas5p_fast_small_observed`] call besides the shared
/// step configuration and output schedule.
#[derive(Clone, Debug)]
pub struct SmallBatchMember<const N: usize, P> {
    pub problem: P,
    pub t_span: (f64, f64),
    pub y0: [f64; N],
}

/// The scalar state of the member a lane is integrating (the locals of the
/// scalar driver).
struct Lane<'a, P> {
    member: usize,
    problem: &'a P,
    t: f64,
    tf: f64,
    h: f64,
    controller: AdaptiveControllerState,
    counters: WorkCounters,
    collector: OutputCollector,
    attempts: usize,
    accepted_steps: usize,
    rejected_steps: usize,
    reuses: usize,
    internal_steps: usize,
    fresh_state: bool,
    /// The scalar workspace's `built` flag (one workspace per member).
    built: bool,
    trial_h: f64,
    clipped: bool,
}

/// The per-lane linear algebra of the scalar workspace.
struct LaneLinear<const N: usize> {
    jacobian: [[f64; N]; N],
    w: [[f64; N]; N],
    pivots: [usize; N],
    row_end: [usize; N],
    l_start: [usize; N],
}

impl<const N: usize> LaneLinear<N> {
    /// The initial content of the scalar workspace (a Jacobian with a fixed
    /// sparsity pattern leaves the other entries at these zeros).
    fn new() -> Self {
        Self {
            jacobian: [[0.0; N]; N],
            w: [[0.0; N]; N],
            pivots: [0; N],
            row_end: [0; N],
            l_start: [N; N],
        }
    }

    /// `Workspace::factor` of the small driver on this lane.
    fn factor(&mut self, gamma: f64, h: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let inv = 1.0 / (h * gamma);
        for (w_row, j_row) in self.w.iter_mut().zip(&self.jacobian) {
            for (w, j) in w_row.iter_mut().zip(j_row) {
                *w = -j;
            }
        }
        for i in 0..N {
            self.w[i][i] += inv;
        }
        counters.direct_factorizations += 1;
        for i in 0..N {
            let last = self.w[i].iter().rposition(|v| *v != 0.0).unwrap_or(0);
            self.row_end[i] = last.max(i);
            self.l_start[i] = N;
        }
        lu_in_place(
            &mut self.w,
            &mut self.pivots,
            &mut self.row_end,
            &mut self.l_start,
        )
    }

    /// `Workspace::solve` of the small driver on this lane's column of `b`.
    fn solve<const B: usize>(
        &self,
        b: &mut [[f64; B]; N],
        lane: usize,
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        counters.linear_solves += 1;
        counters.direct_solve_calls += 1;
        let mut x = column(b, lane);
        lu_solve_in_place(&self.w, &self.pivots, &self.row_end, &self.l_start, &mut x);
        if x.iter().all(|v| v.is_finite()) {
            for (row, v) in b.iter_mut().zip(x) {
                row[lane] = v;
            }
            Ok(())
        } else {
            Err(CoreError::LinearSolve(
                "RODAS5P fast stage solve produced NaN/Inf".into(),
            ))
        }
    }
}

/// The structure-of-arrays vectors of the scalar workspace and the states.
struct Lanes<const N: usize, const B: usize> {
    y: [[f64; B]; N],
    f0: [[f64; B]; N],
    u: [[[f64; B]; N]; STAGES],
    stage_state: [[f64; B]; N],
    stage_rhs: [[f64; B]; N],
    y_new: [[f64; B]; N],
}

fn column<const N: usize, const B: usize>(v: &[[f64; B]; N], lane: usize) -> [f64; N] {
    std::array::from_fn(|k| v[k][lane])
}

fn column_finite<const N: usize, const B: usize>(v: &[[f64; B]; N], lane: usize) -> bool {
    v.iter().all(|row| row[lane].is_finite())
}

/// `x += a * v` elementwise (the scalar driver's `*x += aij * v`).
fn axpy<const N: usize, const B: usize>(x: &mut [[f64; B]; N], a: f64, v: &[[f64; B]; N]) {
    for (x_row, v_row) in x.iter_mut().zip(v) {
        for (x, v) in x_row.iter_mut().zip(v_row) {
            *x += a * v;
        }
    }
}

/// The prologue of the scalar driver for one member, in its order: the
/// configuration, the input, the coefficients, the first step, the
/// collector.
fn start<'a, const N: usize, P>(
    index: usize,
    member: &'a SmallBatchMember<N, P>,
    coefficients_ok: bool,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Lane<'a, P>> {
    adaptive.validate()?;
    let (t, tf) = member.t_span;
    if tf < t || !member.y0.iter().all(|v| v.is_finite()) {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P fast integration input".into(),
        ));
    }
    if !coefficients_ok {
        // The scalar driver builds its workspace here; the shared one failed,
        // so this member gets the same error.
        Workspace::<N>::new()?;
    }
    let h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let collector =
        OutputCollector::new(output, member.t_span, &member.y0)?.with_max_step(adaptive.step_cap());
    Ok(Lane {
        member: index,
        problem: &member.problem,
        t,
        tf,
        h,
        controller: AdaptiveControllerState::default(),
        counters: WorkCounters::default(),
        collector,
        attempts: 0,
        accepted_steps: 0,
        rejected_steps: 0,
        reuses: 0,
        internal_steps: 0,
        fresh_state: false,
        built: false,
        trial_h: 0.0,
        clipped: false,
    })
}

/// The head of one iteration of the scalar driver's loop: `Ok(true)` when
/// the lane attempts `lane.trial_h` next, `Ok(false)` when the loop ends.
fn prepare<P>(lane: &mut Lane<'_, P>, adaptive: &AdaptiveStepConfig) -> CoreResult<bool> {
    if !(lane.t < lane.tf && lane.attempts < adaptive.max_attempts) {
        return Ok(false);
    }
    let Some(next_h) = crate::output::adaptive_end_step(
        lane.t,
        lane.h,
        lane.tf,
        adaptive.step_cap(),
        &lane.controller,
    )?
    else {
        return Ok(false);
    };
    lane.h = next_h;
    let (t, h) = (lane.t, lane.h);
    if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < lane.tf) || t + h == t {
        return Ok(false);
    }
    (lane.trial_h, lane.clipped) = lane.collector.limit_step(t, h, lane.tf)?;
    lane.attempts += 1;
    if lane.fresh_state {
        lane.reuses += 1;
    }
    Ok(true)
}

/// The epilogue of the scalar driver.
fn finish<P>(lane: Lane<'_, P>) -> CoreResult<Rodas5pFastSmallResult> {
    let success = lane.t >= lane.tf;
    let (times, states, output_clipped_steps) = if success {
        lane.collector.finish()?
    } else {
        lane.collector.finish_partial()
    };
    Ok(Rodas5pFastSmallResult {
        observed: ObservedIntegrationResult {
            t: times,
            y: states,
            success,
            message: if success {
                "success".into()
            } else {
                "maximum step count or minimum step reached".into()
            },
            counters: lane.counters,
            internal_steps: lane.internal_steps,
            output_clipped_steps,
        },
        attempts: lane.attempts,
        accepted_steps: lane.accepted_steps,
        rejected_steps: lane.rejected_steps,
        jacobian_reuses: lane.reuses,
        driver: RODAS5P_FAST_SMALL_BATCH_DRIVER_ID,
    })
}

/// One attempt of every lane in `live` (`Workspace::attempt` of the small
/// driver per lane): the error norm, or the error that ended the lane's
/// attempt.
#[allow(clippy::too_many_arguments)]
fn attempt<const N: usize, const B: usize, P: BatchProblem<N, B>>(
    coefficients: &Workspace<N>,
    lanes: &mut [Option<Lane<'_, P>>; B],
    lane_data: &P::Lanes,
    linear: &mut [LaneLinear<N>; B],
    v: &mut Lanes<N, B>,
    mut live: [bool; B],
    atol: f64,
    rtol: f64,
) -> [CoreResult<f64>; B] {
    let mut outcome: [CoreResult<f64>; B] = std::array::from_fn(|_| Ok(0.0));
    let mut h = [1.0; B];
    let mut any_fresh = false;
    for l in 0..B {
        if !live[l] {
            continue;
        }
        let lane = lanes[l].as_mut().expect("a live lane has a member");
        h[l] = lane.trial_h;
        if !lane.fresh_state {
            lane.counters.jacobian_builds += 1;
            lane.problem
                .jacobian(&column(&v.y, l), &mut linear[l].jacobian);
            lane.built = true;
            any_fresh = true;
        }
    }
    if any_fresh {
        // `f(t, y)` of the fresh lanes (`stage_rhs` is free until stage 0).
        P::rhs_batch(lane_data, &v.y, &mut v.stage_rhs);
        for l in 0..B {
            let Some(lane) = lanes[l].as_mut().filter(|_| live[l]) else {
                continue;
            };
            if lane.fresh_state {
                continue;
            }
            if column_finite(&v.stage_rhs, l) {
                for (f0, rhs) in v.f0.iter_mut().zip(&v.stage_rhs) {
                    f0[l] = rhs[l];
                }
                lane.counters.rhs_calls += 1;
                lane.counters.rhs_evaluations += 1;
            } else {
                live[l] = false;
                outcome[l] = Err(CoreError::NonFinite("RHS produced NaN/Inf".into()));
            }
        }
    }
    for l in 0..B {
        if !live[l] {
            continue;
        }
        let lane = lanes[l].as_mut().expect("a live lane has a member");
        if let Err(error) = linear[l].factor(coefficients.gamma, h[l], &mut lane.counters) {
            live[l] = false;
            outcome[l] = Err(error);
        }
    }
    for i in 0..STAGES {
        if !live.contains(&true) {
            break;
        }
        if i == 0 {
            v.stage_rhs = v.f0;
        } else {
            v.stage_state = v.y;
            for &(j, aij) in &coefficients.a_nonzero[i] {
                axpy(&mut v.stage_state, aij, &v.u[j]);
            }
            P::rhs_batch(lane_data, &v.stage_state, &mut v.stage_rhs);
            for l in 0..B {
                if !live[l] {
                    continue;
                }
                if column_finite(&v.stage_rhs, l) {
                    let counters = &mut lanes[l].as_mut().expect("live").counters;
                    counters.rhs_calls += 1;
                    counters.rhs_evaluations += 1;
                } else {
                    live[l] = false;
                    outcome[l] = Err(CoreError::NonFinite("RHS produced NaN/Inf".into()));
                }
            }
        }
        for &(j, c) in &coefficients.c_nonzero[i] {
            let cij: [f64; B] = std::array::from_fn(|l| c / h[l]);
            for (x_row, u_row) in v.stage_rhs.iter_mut().zip(&v.u[j]) {
                for ((x, u), cij) in x_row.iter_mut().zip(u_row).zip(&cij) {
                    // The scalar guard: an exactly zero quotient adds nothing.
                    if *cij != 0.0 {
                        *x += cij * u;
                    }
                }
            }
        }
        for l in 0..B {
            if !live[l] {
                continue;
            }
            let counters = &mut lanes[l].as_mut().expect("live").counters;
            if let Err(error) = linear[l].solve(&mut v.stage_rhs, l, counters) {
                live[l] = false;
                outcome[l] = Err(error);
            }
        }
        v.u[i] = v.stage_rhs;
    }
    if !live.contains(&true) {
        return outcome;
    }
    v.y_new = v.y;
    for &(j, bj) in &coefficients.b_nonzero {
        axpy(&mut v.y_new, bj, &v.u[j]);
    }
    let mut sum = [0.0; B];
    let error = &v.u[STAGES - 1];
    for ((e_row, a_row), b_row) in error.iter().zip(&v.y).zip(&v.y_new) {
        for (((sum, e), a), b) in sum.iter_mut().zip(e_row).zip(a_row).zip(b_row) {
            let z = e / (atol + rtol * a.abs().max(b.abs()));
            *sum += z * z;
        }
    }
    for l in 0..B {
        if !live[l] {
            continue;
        }
        if !column_finite(&v.y_new, l) {
            outcome[l] = Err(CoreError::NonFinite(
                "RODAS5P fast step produced NaN/Inf".into(),
            ));
            continue;
        }
        let norm = (sum[l] / N as f64).sqrt();
        outcome[l] = Ok(if norm.is_finite() {
            norm
        } else {
            f64::INFINITY
        });
    }
    outcome
}

/// The tail of one iteration of the scalar driver's loop after the attempt.
fn conclude<const N: usize, const B: usize, P>(
    lane: &mut Lane<'_, P>,
    v: &mut Lanes<N, B>,
    l: usize,
    outcome: CoreResult<f64>,
    adaptive: &AdaptiveStepConfig,
) -> CoreResult<()> {
    lane.fresh_state = lane.built;
    let (error, failure) = match outcome {
        Ok(error) if error <= 1.0 => (error, None),
        Ok(error) => (error, Some(AdaptiveFailureKind::LocalError)),
        Err(error) => match failure_kind(&error) {
            Some(kind) => {
                lane.fresh_state = false;
                (f64::INFINITY, Some(kind))
            }
            None => return Err(error),
        },
    };
    match failure {
        None => {
            lane.counters.accepted_steps += 1;
            lane.accepted_steps += 1;
            lane.t += lane.trial_h;
            for (y, y_new) in v.y.iter_mut().zip(&v.y_new) {
                y[l] = y_new[l];
            }
            lane.collector
                .accept(lane.t, &column(&v.y, l), lane.clipped)?;
            lane.internal_steps += 1;
            lane.fresh_state = false;
        }
        Some(kind) => {
            let counters = &mut lane.counters;
            counters.rejected_steps += 1;
            lane.rejected_steps += 1;
            match kind {
                AdaptiveFailureKind::LocalError => counters.local_error_failures += 1,
                AdaptiveFailureKind::LinearSolve => counters.linear_solve_failures += 1,
                AdaptiveFailureKind::NonlinearSolve => counters.nonlinear_solve_failures += 1,
                AdaptiveFailureKind::NonFinite => counters.nonfinite_step_failures += 1,
            }
        }
    }
    lane.h = rodas_next_step_after_attempt(
        &mut lane.controller,
        adaptive,
        lane.h,
        lane.trial_h,
        error,
        failure.is_none(),
        lane.clipped,
    )?;
    Ok(())
}

/// Integrates every member of `members` as
/// [`crate::integrate_rodas5p_fast_small_observed`] would, with `B` lanes
/// (speed research node SPD08). The results are in member order; each equals
/// the scalar driver's return value for that member bitwise (its error for
/// a member the scalar driver rejects), apart from the `driver` field,
/// which is [`RODAS5P_FAST_SMALL_BATCH_DRIVER_ID`].
pub fn integrate_rodas5p_fast_small_batch<const N: usize, const B: usize, P: BatchProblem<N, B>>(
    members: &[SmallBatchMember<N, P>],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> Vec<CoreResult<Rodas5pFastSmallResult>> {
    assert!(B >= 1, "the batch driver needs at least one lane");
    let Some(first) = members.first() else {
        return Vec::new();
    };
    // The coefficients are constants: built once instead of once per member.
    let coefficients = Workspace::<N>::new();
    let coefficients_ok = coefficients.is_ok();
    let mut results: Vec<Option<CoreResult<Rodas5pFastSmallResult>>> =
        members.iter().map(|_| None).collect();
    let mut queue = members.iter().enumerate();
    let mut lanes: [Option<Lane<'_, P>>; B] = std::array::from_fn(|_| None);
    let mut lane_data = P::lanes(&first.problem);
    let mut linear: [LaneLinear<N>; B] = std::array::from_fn(|_| LaneLinear::new());
    let mut v = Lanes::<N, B> {
        y: [[0.0; B]; N],
        f0: [[0.0; B]; N],
        u: [[[0.0; B]; N]; STAGES],
        stage_state: [[0.0; B]; N],
        stage_rhs: [[0.0; B]; N],
        y_new: [[0.0; B]; N],
    };
    loop {
        let mut live = [false; B];
        for l in 0..B {
            loop {
                if lanes[l].is_none() {
                    let Some((index, member)) = queue.next() else {
                        break;
                    };
                    match start(index, member, coefficients_ok, adaptive, output) {
                        Ok(lane) => {
                            for (y, y0) in v.y.iter_mut().zip(&member.y0) {
                                y[l] = *y0;
                            }
                            P::set_lane(&mut lane_data, l, &member.problem);
                            linear[l] = LaneLinear::new();
                            lanes[l] = Some(lane);
                        }
                        Err(error) => {
                            results[index] = Some(Err(error));
                            continue;
                        }
                    }
                }
                let lane = lanes[l].as_mut().expect("the lane has a member");
                match prepare(lane, adaptive) {
                    Ok(true) => {
                        live[l] = true;
                        break;
                    }
                    Ok(false) => {
                        let lane = lanes[l].take().expect("the lane has a member");
                        let index = lane.member;
                        results[index] = Some(finish(lane));
                    }
                    Err(error) => {
                        let lane = lanes[l].take().expect("the lane has a member");
                        results[lane.member] = Some(Err(error));
                    }
                }
            }
        }
        if !live.contains(&true) {
            break;
        }
        let Ok(coefficients) = coefficients.as_ref() else {
            unreachable!("no member starts without coefficients");
        };
        let outcomes = attempt(
            coefficients,
            &mut lanes,
            &lane_data,
            &mut linear,
            &mut v,
            live,
            adaptive.atol,
            adaptive.rtol,
        );
        for (l, outcome) in outcomes.into_iter().enumerate() {
            if !live[l] {
                continue;
            }
            let lane = lanes[l].as_mut().expect("a live lane has a member");
            if let Err(error) = conclude(lane, &mut v, l, outcome, adaptive) {
                let lane = lanes[l].take().expect("a live lane has a member");
                results[lane.member] = Some(Err(error));
            }
        }
    }
    results
        .into_iter()
        .map(|result| result.expect("every member finished"))
        .collect()
}
