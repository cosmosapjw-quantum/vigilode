//! Route registry, WindowDriver trait, and formal solver state machine (Phase 2, plan §5.2, §5.3, §6.1, §10.4).

use crate::{
    Dopri5, Options, Problem, Rodas5p, SessionEpochs,
    selection_evidence::SelectionIdentity,
    switching::{SwitchingDriverStats, SwitchingEvidence, SwitchingMethod, SwitchingSegment},
};
use rodas5p_core::WorkCounters;
use rodas5p_integrators::Dopri5StopReason;
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Identity of an executable route; serialised in `vigilode.switching.v2`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum RouteId {
    /// Explicit RHS-only DOPRI5 route.
    Dopri5Rhs,
    /// Implicit RODAS5P dense route with user-provided Jacobian.
    Rodas5pDense,
    /// Implicit RODAS5P banded route with verified band.
    Rodas5pBanded,
    /// Implicit RODAS5P route with finite-difference Jacobian.
    Rodas5pFiniteDifference,
    /// Implicit RODAS5P matrix-free route with Krylov solver.
    Rodas5pMatrixFree,
    /// Implicit RODAS5P route with JVP-assembled Jacobian for small dimensions.
    Rodas5pJvpAssembled,
    /// Fused adaptive exponential integrator route.
    Pexprb54s4Fused,
}

impl RouteId {
    /// Return the string identifier of this route.
    pub fn as_str(self) -> &'static str {
        match self {
            RouteId::Dopri5Rhs => "dopri5-rhs",
            RouteId::Rodas5pDense => "rodas5p-dense",
            RouteId::Rodas5pBanded => "rodas5p-banded",
            RouteId::Rodas5pFiniteDifference => "rodas5p-finite-difference",
            RouteId::Rodas5pMatrixFree => "rodas5p-matrix-free",
            RouteId::Rodas5pJvpAssembled => "rodas5p-jvp-assembled",
            RouteId::Pexprb54s4Fused => "pexprb54s4-fused",
        }
    }
}

/// Why a batch execution stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchStop {
    /// Batch completed all requested work up to the target.
    Completed,
    /// Batch stopped because the attempt budget was exhausted.
    Budget,
    /// Batch stopped because work exceeded the allocated ceiling.
    Ceiling,
    /// Batch stopped because the progress bound was violated.
    ProgressBound,
    /// Batch failed with a fatal error.
    Fatal(String),
}

/// Everything the scheduler and comparator need from one bounded batch.
#[derive(Clone, Debug)]
pub struct BatchOutcome {
    /// Public segment record of this batch.
    pub record: SwitchingSegment,
    /// Full deterministic counters of this batch.
    pub counters: WorkCounters,
    /// Last accepted state of this batch.
    pub state: Vec<f64>,
    /// Intermediate window endpoints landed during coalesced execution.
    pub landed: Vec<(f64, Vec<f64>)>,
    /// Stop reason of this batch.
    pub stop: BatchStop,
    /// Proposed next step size from the driver.
    pub proposed_next_step: Option<f64>,
}

/// Permissions and requirements of a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteRequirements {
    /// Whether this route requires derivative callbacks (Jacobian or JVP).
    pub needs_derivatives: bool,
    /// Maximum dense dimension supported by this route, if bounded.
    pub max_dense_dimension: Option<usize>,
    /// Whether this route requires callbacks to be replay-safe.
    pub replay_safety: bool,
}

/// Minimum guard-callback units charged by any accepted RODAS5P attempt:
/// stages 2..=8 each evaluate the RHS once (stage 1 may reuse f(y_n)).
pub const RODAS5P_MIN_CALLBACKS_PER_ACCEPTED_ATTEMPT: u64 = 7;

/// Upper bound on the distance an adaptive run can cover before its guard
/// stops it: at most `ceiling / min_callbacks` accepted attempts fit under the
/// ceiling, the first trial step is at most `h0`, and each accepted step grows
/// the proposal by at most `max_growth`. Returns `INFINITY` when no bound applies.
pub fn progress_reach_upper_bound(
    h0: f64,
    max_growth: f64,
    ceiling_callbacks: u64,
    min_callbacks_per_accepted_attempt: u64,
) -> f64 {
    if !(h0.is_finite() && h0 > 0.0 && max_growth.is_finite() && max_growth >= 1.0)
        || min_callbacks_per_accepted_attempt == 0
    {
        return f64::INFINITY;
    }
    let m = ceiling_callbacks / min_callbacks_per_accepted_attempt;
    if m == 0 {
        return 0.0;
    }
    if m > 400 {
        return f64::INFINITY;
    }
    let reach = if max_growth == 1.0 {
        h0 * m as f64
    } else {
        h0 * (max_growth.powi(m as i32) - 1.0) / (max_growth - 1.0)
    };
    if reach.is_finite() {
        reach.next_up()
    } else {
        f64::INFINITY
    }
}

/// Progress model for the progress-bound screen (Section 8.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressModel {
    /// Upper bound on step growth per accepted attempt.
    pub max_step_growth: f64,
    /// Minimum physical work performed per attempt.
    pub min_work_per_attempt: u64,
}

/// Parameters for one window batch execution.
pub struct BatchRequest<'a> {
    /// Start time of the window batch.
    pub start_time: f64,
    /// Target time of the window batch.
    pub target_time: f64,
    /// Terminal time of the overall solve.
    pub solve_end: f64,
    /// Initial state at `start_time`.
    pub state: &'a [f64],
    /// Proposed initial step size for this batch.
    pub proposed_h: Option<f64>,
    /// Solver options.
    pub options: &'a Options,
    /// Intermediate window boundaries to land.
    pub landings: &'a [f64],
    /// Maximum work ceiling for exploratory pilots.
    pub ceiling: Option<u64>,
    /// Whether to carry step proposals across batches.
    pub carry_explicit_step: bool,
    /// Whether this batch is an exploratory probe.
    pub is_probe: bool,
    /// Whether this batch is an entry pilot from a checkpoint.
    pub is_entry_pilot: bool,
    /// Identity metadata of the window.
    pub identity: SelectionIdentity,
}

/// Trait implemented by executable routes behind the registry.
pub trait WindowDriver {
    /// Route identifier for this driver.
    fn route(&self) -> RouteId;
    /// Requirements and permissions needed by this driver.
    fn requires(&self) -> RouteRequirements;
    /// Progress model for bounding pilot progress.
    fn progress_model(&self, n: usize, autonomous: bool) -> Option<ProgressModel>;
    /// Run one bounded window batch.
    fn run(&mut self, problem: &Problem, req: &BatchRequest<'_>) -> Result<BatchOutcome, String>;
}

/// Dopri5 RHS-only window driver.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dopri5WindowDriver;

impl WindowDriver for Dopri5WindowDriver {
    fn route(&self) -> RouteId {
        RouteId::Dopri5Rhs
    }

    fn requires(&self) -> RouteRequirements {
        RouteRequirements {
            needs_derivatives: false,
            max_dense_dimension: None,
            replay_safety: true,
        }
    }

    fn progress_model(&self, _n: usize, _autonomous: bool) -> Option<ProgressModel> {
        None
    }

    fn run(&mut self, problem: &Problem, req: &BatchRequest<'_>) -> Result<BatchOutcome, String> {
        let start = req.start_time;
        let target = req.target_time;
        let solve_end = req.solve_end;
        let y = req.state;
        let opts = req.options;
        let ceiling = req.ceiling;
        let carry_explicit_step = req.carry_explicit_step;
        let probe = req.is_probe;

        let (next, r, ceiling_stop) = if carry_explicit_step || probe {
            let epochs = SessionEpochs {
                model: req.identity.model_epoch,
                provider: compute_provider_epoch(RouteId::Dopri5Rhs, problem),
                norm: req.identity.physical_norm_epoch,
                tolerance: compute_tolerance_epoch(opts),
            };
            let times = if target < solve_end {
                vec![start, target, solve_end]
            } else {
                vec![start, target]
            };
            let (outcome, r) = Dopri5::with_session(
                problem,
                (start, solve_end),
                y,
                opts,
                &times,
                epochs,
                |session| {
                    let mut stopped = false;
                    match ceiling {
                        None => {
                            session.advance_to(target, opts.max_steps, epochs)?;
                        }
                        Some(cap) => {
                            let base = session.checkpoint().counters.rhs_calls;
                            loop {
                                let step = session.advance_to(target, 1, epochs)?;
                                if step != crate::SessionAdvance::Yield {
                                    break;
                                }
                                if session.checkpoint().counters.rhs_calls - base > cap {
                                    stopped = true;
                                    break;
                                }
                            }
                        }
                    }
                    Ok::<(f64, bool), crate::ExplicitSolveError>((
                        session.checkpoint().proposed_h,
                        stopped,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;
            let (next, stopped) = outcome.map_err(|e| e.to_string())?;
            (
                (carry_explicit_step && r.final_time < solve_end).then_some(next),
                r,
                stopped,
            )
        } else {
            (
                None,
                Dopri5::solve(problem, (start, target), y, opts).map_err(|e| e.to_string())?,
                false,
            )
        };
        let c = r.stats.counters;
        let success = if carry_explicit_step || probe {
            r.final_time == target
                && matches!(r.stop_reason, None | Some(Dopri5StopReason::AttemptBudget))
                && r.callback_failure.is_none()
        } else {
            r.success
        };
        let fatal = r.callback_failure.is_some();
        let budget =
            r.final_time < target && r.stop_reason == Some(Dopri5StopReason::AttemptBudget);
        let stop = if fatal {
            BatchStop::Fatal("callback failure".to_string())
        } else if budget {
            BatchStop::Budget
        } else if ceiling_stop {
            BatchStop::Ceiling
        } else {
            BatchStop::Completed
        };
        let reached = r.final_time;
        let state = r.final_state;
        let attempts = r.stats.attempts;
        let accepted = r.stats.accepted_steps;
        let rejected = r.stats.rejected_steps;
        let driver = SwitchingDriverStats::Dopri5(Box::new(r.stats));
        let proposed_next_step = next.filter(|h| h.is_finite() && *h > 0.0);
        let work_units = c.rhs_calls as f64
            + c.jvp_vectors as f64
            + c.ft_calls as f64
            + c.jacobian_builds as f64 * y.len() as f64
            + c.linear_solves as f64;
        let record = SwitchingSegment {
            method: SwitchingMethod::Dopri5,
            start_time: start,
            target_time: target,
            reached_time: reached,
            windows: 1,
            is_probe: probe,
            committed: false,
            success,
            attempts,
            accepted_steps: accepted,
            rejected_steps: rejected,
            rhs_calls: c.rhs_calls,
            jacobian_builds: c.jacobian_builds,
            jvp_vectors: c.jvp_vectors,
            ft_calls: c.ft_calls,
            linear_solves: c.linear_solves,
            work_units,
            decision: "",
            evidence: SwitchingEvidence::Uncompared,
            initial_step: opts.h0,
            proposed_next_step,
            comparator_work_ratio: None,
            entry_probe_reason: None,
            driver,
        };
        Ok(BatchOutcome {
            record,
            counters: c,
            state,
            landed: Vec::new(),
            stop,
            proposed_next_step,
        })
    }
}

/// Rodas5p implicit window driver.
#[derive(Clone, Copy, Debug)]
pub struct Rodas5pWindowDriver {
    route: RouteId,
}

impl Rodas5pWindowDriver {
    /// Create a Rodas5p driver for the given problem's Jacobian configuration.
    pub fn for_problem(problem: &Problem) -> Self {
        let route = match &problem.jacobian {
            crate::problem::Jacobian::Dense(_) => RouteId::Rodas5pDense,
            crate::problem::Jacobian::Banded { .. } => RouteId::Rodas5pBanded,
            crate::problem::Jacobian::Jvp(_) => RouteId::Rodas5pMatrixFree,
            crate::problem::Jacobian::FiniteDifference => RouteId::Rodas5pFiniteDifference,
        };
        Self { route }
    }

    /// Create a Rodas5p driver with an explicit route.
    pub fn with_route(route: RouteId) -> Self {
        Self { route }
    }
}

impl WindowDriver for Rodas5pWindowDriver {
    fn route(&self) -> RouteId {
        self.route
    }

    fn requires(&self) -> RouteRequirements {
        RouteRequirements {
            needs_derivatives: true,
            max_dense_dimension: if self.route == RouteId::Rodas5pJvpAssembled {
                Some(8)
            } else {
                None
            },
            replay_safety: true,
        }
    }

    fn progress_model(&self, _n: usize, _autonomous: bool) -> Option<ProgressModel> {
        Some(ProgressModel {
            max_step_growth: 5.0,
            min_work_per_attempt: RODAS5P_MIN_CALLBACKS_PER_ACCEPTED_ATTEMPT,
        })
    }

    fn run(&mut self, problem: &Problem, req: &BatchRequest<'_>) -> Result<BatchOutcome, String> {
        let start = req.start_time;
        let target = req.target_time;
        let y = req.state;
        let opts = req.options;
        let ceiling = req.ceiling;
        let probe = req.is_probe;

        let has_band_verification = problem.band.is_some()
            && matches!(problem.jacobian, crate::problem::Jacobian::Dense(_));

        if req.is_entry_pilot
            && !has_band_verification
            && let Some(ceiling) = req.ceiling
        {
            let h0 = req.proposed_h.unwrap_or(req.options.h0);
            let reach = progress_reach_upper_bound(
                h0,
                rodas5p_integrators::AdaptiveStepConfig::default().max_factor,
                ceiling,
                RODAS5P_MIN_CALLBACKS_PER_ACCEPTED_ATTEMPT,
            );
            if reach < (req.target_time - req.start_time) * (1.0 - 1e-12) {
                let record = SwitchingSegment {
                    method: SwitchingMethod::Rodas5p,
                    start_time: start,
                    target_time: target,
                    reached_time: start,
                    windows: 1,
                    is_probe: probe,
                    committed: false,
                    success: false,
                    attempts: 0,
                    accepted_steps: 0,
                    rejected_steps: 0,
                    rhs_calls: 0,
                    jacobian_builds: 0,
                    jvp_vectors: 0,
                    ft_calls: 0,
                    linear_solves: 0,
                    work_units: 0.0,
                    decision: "same-checkpoint implicit pilot discarded: unreachable under step growth bound",
                    evidence: SwitchingEvidence::Uncompared,
                    initial_step: req.options.h0,
                    proposed_next_step: None,
                    comparator_work_ratio: None,
                    entry_probe_reason: None,
                    driver: SwitchingDriverStats::Rodas5p(Box::new(Rodas5p::empty_stats(
                        problem,
                        req.options,
                        start,
                        y,
                    ))),
                };
                return Ok(BatchOutcome {
                    record,
                    counters: WorkCounters::default(),
                    state: req.state.to_vec(),
                    landed: Vec::new(),
                    stop: BatchStop::ProgressBound,
                    proposed_next_step: None,
                });
            }
        }

        let r = if self.route == RouteId::Rodas5pJvpAssembled {
            if problem.dimension > 8 {
                return Err(format!(
                    "JvpAssembled route supports dimension <= 8, got {}",
                    problem.dimension
                ));
            }
            let jvp = match &problem.jacobian {
                crate::problem::Jacobian::Jvp(jvp) => jvp.clone(),
                _ => return Err("JvpAssembled route requires JVP Jacobian".to_string()),
            };
            let dim = problem.dimension;
            let dense_cb =
                move |t: f64, y: &[f64], out: &mut [f64]| -> Result<(), crate::CallbackError> {
                    let mut ej = [0.0; 8];
                    let mut col = [0.0; 8];
                    for j in 0..dim {
                        ej.fill(0.0);
                        ej[j] = 1.0;
                        col.fill(0.0);
                        (jvp)(t, y, &ej[..dim], &mut col[..dim])?;
                        for i in 0..dim {
                            out[i * dim + j] = col[i];
                        }
                    }
                    Ok(())
                };
            let mut p = problem.clone();
            p.jacobian = crate::problem::Jacobian::dense(dense_cb);
            Rodas5p::solve_observed_bounded(&p, (start, target), y, opts, req.landings, ceiling)
                .map_err(|e| e.to_string())?
        } else {
            Rodas5p::solve_observed_bounded(
                problem,
                (start, target),
                y,
                opts,
                req.landings,
                ceiling,
            )
            .map_err(|e| e.to_string())?
        };

        let c = r.stats.counters;
        let success = r.status == crate::solution::Status::Success;
        let fatal = matches!(
            r.status,
            crate::solution::Status::Failed(crate::FailureReason::CallbackFailed { .. })
        );
        let budget = matches!(
            r.status,
            crate::solution::Status::Failed(crate::FailureReason::MaxStepsReached { .. })
        );
        let ceiling_stop = matches!(
            r.status,
            crate::solution::Status::Failed(crate::FailureReason::WorkCeilingReached { .. })
        );
        let stop = if fatal {
            BatchStop::Fatal("callback failure".to_string())
        } else if budget {
            BatchStop::Budget
        } else if ceiling_stop {
            BatchStop::Ceiling
        } else {
            BatchStop::Completed
        };
        let reached = *r.t.last().unwrap_or(&start);
        let crate::Solution {
            t: times,
            y: states,
            stats,
            ..
        } = r;
        let rows = times.len();
        let mut landed = Vec::new();
        let mut final_state = None;
        for (index, (t, row)) in times.into_iter().zip(states).enumerate() {
            if index + 1 == rows {
                final_state = Some(row);
            } else if t > start && t < reached {
                landed.push((t, row));
            }
        }
        let state = final_state.unwrap_or_else(|| y.to_vec());
        let work_units = c.rhs_calls as f64
            + c.jvp_vectors as f64
            + c.ft_calls as f64
            + c.jacobian_builds as f64 * y.len() as f64
            + c.linear_solves as f64;
        let record = SwitchingSegment {
            method: SwitchingMethod::Rodas5p,
            start_time: start,
            target_time: target,
            reached_time: reached,
            windows: 1,
            is_probe: probe,
            committed: false,
            success,
            attempts: stats.attempts,
            accepted_steps: stats.accepted_steps,
            rejected_steps: stats.rejected_steps,
            rhs_calls: c.rhs_calls,
            jacobian_builds: c.jacobian_builds,
            jvp_vectors: c.jvp_vectors,
            ft_calls: c.ft_calls,
            linear_solves: c.linear_solves,
            work_units,
            decision: "",
            evidence: SwitchingEvidence::Uncompared,
            initial_step: opts.h0,
            proposed_next_step: None,
            comparator_work_ratio: None,
            entry_probe_reason: None,
            driver: SwitchingDriverStats::Rodas5p(Box::new(stats)),
        };
        Ok(BatchOutcome {
            record,
            counters: WorkCounters::from(&c),
            state,
            landed,
            stop,
            proposed_next_step: None,
        })
    }
}

type SystemRhsFn =
    Arc<dyn Fn(f64, &[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync>;
type SystemJacFn =
    Arc<dyn Fn(f64, &[f64]) -> rodas5p_core::CoreResult<rodas5p_core::DenseMatrix> + Send + Sync>;
type SystemJvpFn =
    Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync>;
type SystemPartialTFn =
    Arc<dyn Fn(f64, &[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync>;

/// Fused adaptive exponential integrator window driver.
#[derive(Clone, Copy, Debug, Default)]
pub struct PexprbWindowDriver;

impl WindowDriver for PexprbWindowDriver {
    fn route(&self) -> RouteId {
        RouteId::Pexprb54s4Fused
    }

    fn requires(&self) -> RouteRequirements {
        RouteRequirements {
            needs_derivatives: true,
            max_dense_dimension: None,
            replay_safety: true,
        }
    }

    fn progress_model(&self, _n: usize, _autonomous: bool) -> Option<ProgressModel> {
        Some(ProgressModel {
            max_step_growth: 5.0,
            min_work_per_attempt: 12,
        })
    }

    fn run(&mut self, problem: &Problem, req: &BatchRequest<'_>) -> Result<BatchOutcome, String> {
        let start = req.start_time;
        let target = req.target_time;
        let y = req.state;
        let opts = req.options;
        let ceiling = req.ceiling;
        let probe = req.is_probe;

        if req.is_entry_pilot
            && let Some(ceiling) = req.ceiling
        {
            let c_min = 12u64;
            let m = (ceiling / c_min) as usize;
            let h0 = req.proposed_h.unwrap_or(req.options.h0);
            let g = 5.0f64;
            let reach = if m == 0 {
                0.0
            } else if m > 400 {
                f64::INFINITY
            } else {
                h0 * (g.powi(m as i32) - 1.0) / (g - 1.0)
            };
            if reach < (req.target_time - req.start_time) * (1.0 - 1e-12) {
                let record = SwitchingSegment {
                    method: SwitchingMethod::Rodas5p,
                    start_time: start,
                    target_time: target,
                    reached_time: start,
                    windows: 1,
                    is_probe: probe,
                    committed: false,
                    success: false,
                    attempts: 0,
                    accepted_steps: 0,
                    rejected_steps: 0,
                    rhs_calls: 0,
                    jacobian_builds: 0,
                    jvp_vectors: 0,
                    ft_calls: 0,
                    linear_solves: 0,
                    work_units: 0.0,
                    decision: "same-checkpoint implicit pilot discarded: unreachable under step growth bound",
                    evidence: SwitchingEvidence::Uncompared,
                    initial_step: req.options.h0,
                    proposed_next_step: None,
                    comparator_work_ratio: None,
                    entry_probe_reason: None,
                    driver: SwitchingDriverStats::Rodas5p(Box::new(Rodas5p::empty_stats(
                        problem,
                        req.options,
                        start,
                        y,
                    ))),
                };
                return Ok(BatchOutcome {
                    record,
                    counters: WorkCounters::default(),
                    state: req.state.to_vec(),
                    landed: Vec::new(),
                    stop: BatchStop::ProgressBound,
                    proposed_next_step: None,
                });
            }
        }

        let n = problem.dimension;
        let user_rhs = problem.rhs.clone();
        let rhs: SystemRhsFn = Arc::new(move |t, y, out| {
            user_rhs(t, y, out).map_err(|e| rodas5p_core::CoreError::FatalCallback(e.to_string()))
        });

        let (jacobian, jvp): (Option<SystemJacFn>, Option<SystemJvpFn>) = match &problem.jacobian {
            crate::problem::Jacobian::Jvp(user_jvp) => {
                let user_jvp = user_jvp.clone();
                let jvp_cb = move |t: f64,
                                   y: &[f64],
                                   v: &[f64],
                                   out: &mut [f64]|
                      -> rodas5p_core::CoreResult<()> {
                    user_jvp(t, y, v, out)
                        .map_err(|e| rodas5p_core::CoreError::FatalCallback(e.to_string()))
                };
                (None, Some(Arc::new(jvp_cb) as SystemJvpFn))
            }
            crate::problem::Jacobian::Dense(user_dense) => {
                let user_dense_mat = user_dense.clone();
                let user_dense_jvp = user_dense.clone();
                let jac_cb = move |t: f64,
                                   y: &[f64]|
                      -> rodas5p_core::CoreResult<rodas5p_core::DenseMatrix> {
                    let mut m = rodas5p_core::DenseMatrix::zeros(n, n);
                    user_dense_mat(t, y, m.as_mut_slice())
                        .map_err(|e| rodas5p_core::CoreError::FatalCallback(e.to_string()))?;
                    Ok(m)
                };
                let jvp_cb = move |t: f64,
                                   y: &[f64],
                                   v: &[f64],
                                   out: &mut [f64]|
                      -> rodas5p_core::CoreResult<()> {
                    let mut m = vec![0.0; n * n];
                    user_dense_jvp(t, y, &mut m)
                        .map_err(|e| rodas5p_core::CoreError::FatalCallback(e.to_string()))?;
                    for i in 0..n {
                        let mut sum = 0.0;
                        for j in 0..n {
                            sum += m[i * n + j] * v[j];
                        }
                        out[i] = sum;
                    }
                    Ok(())
                };
                (
                    Some(Arc::new(jac_cb) as SystemJacFn),
                    Some(Arc::new(jvp_cb) as SystemJvpFn),
                )
            }
            _ => {
                return Err("Pexprb route requires an explicit Jacobian or JVP".to_string());
            }
        };

        let partial_t: Option<SystemPartialTFn> = problem.time_derivative.as_ref().map(|user_ft| {
            let user_ft = user_ft.clone();
            let ft_cb = move |t: f64, y: &[f64], out: &mut [f64]| -> rodas5p_core::CoreResult<()> {
                user_ft(t, y, out)
                    .map_err(|e| rodas5p_core::CoreError::FatalCallback(e.to_string()))
            };
            Arc::new(ft_cb) as SystemPartialTFn
        });

        let mass_matrix = problem
            .mass
            .as_ref()
            .map(|m| rodas5p_core::DenseMatrix::new(n, n, m.clone()))
            .transpose()
            .map_err(|e| e.to_string())?;

        let ode = rodas5p_integrators::OdeProblem::new(
            "vigilode-pexprb",
            n,
            rhs,
            None,
            jacobian,
            jvp,
            partial_t,
            problem.autonomous,
            mass_matrix,
            None,
        )
        .map_err(|e| e.to_string())?;

        let atol = match &req.options.atol {
            crate::options::Atol::Scalar(s) => *s,
            crate::options::Atol::Vector(v) => v.iter().copied().fold(0.0, f64::max),
        };
        let adaptive = rodas5p_integrators::AdaptiveStepConfig {
            rtol: req.options.rtol,
            atol,
            initial_step: req.proposed_h.unwrap_or(req.options.h0),
            max_attempts: req.options.max_steps,
            ..rodas5p_integrators::AdaptiveStepConfig::default()
        };

        let mut times = Vec::with_capacity(req.landings.len() + 2);
        times.push(start);
        times.extend(
            req.landings
                .iter()
                .copied()
                .filter(|t| *t > start && *t < target),
        );
        if target > start {
            times.push(target);
        }
        let output = rodas5p_integrators::OutputSchedule::new(times).map_err(|e| e.to_string())?;

        let phi_config = rodas5p_integrators::FusedPhiKrylovConfig::default();
        let execution = rodas5p_integrators::ParallelExecution::sequential();
        let run_res = rodas5p_integrators::integrate_pexprb54s4_fused_adaptive_observed(
            &ode,
            (start, target),
            y,
            &adaptive,
            &output,
            phi_config,
            &execution,
        );

        let res = match run_res {
            Ok(r) => r,
            Err(e) => return Err(e.to_string()),
        };

        let c = res.observed.counters;
        let attempts = res.diagnostics.attempts;
        let success = res.observed.success;
        let budget = !success && attempts >= adaptive.max_attempts;
        let ceiling_stop = ceiling.is_some_and(|cap| {
            let work = c.rhs_calls
                + c.jvp_vectors
                + c.ft_calls
                + c.jacobian_builds * y.len() as u64
                + c.linear_solves;
            work > cap
        });
        let stop = if success {
            BatchStop::Completed
        } else if budget {
            BatchStop::Budget
        } else if ceiling_stop {
            BatchStop::Ceiling
        } else {
            BatchStop::Fatal(res.observed.message.clone())
        };

        let reached = *res.observed.t.last().unwrap_or(&start);
        let rows = res.observed.t.len();
        let mut landed = Vec::new();
        let mut final_state = None;
        for (index, (t, row)) in res.observed.t.into_iter().zip(res.observed.y).enumerate() {
            if index + 1 == rows {
                final_state = Some(row);
            } else if t > start && t < reached {
                landed.push((t, row));
            }
        }
        let state = final_state.unwrap_or_else(|| y.to_vec());
        let proposed_next_step = res
            .diagnostics
            .accepted_step_sizes
            .last()
            .copied()
            .filter(|h| h.is_finite() && *h > 0.0);
        let work_units = c.rhs_calls as f64
            + c.jvp_vectors as f64
            + c.ft_calls as f64
            + c.jacobian_builds as f64 * y.len() as f64
            + c.linear_solves as f64;
        let mut stats = Rodas5p::empty_stats(problem, req.options, start, y);
        stats.attempts = attempts;
        stats.accepted_steps = res.diagnostics.accepted_steps;
        stats.rejected_steps = res.diagnostics.rejected_steps;
        stats.counters = (&c).into();
        stats.driver_counters = (&c).into();
        stats.route = crate::solution::RouteRecord {
            entry_point: "integrate_pexprb54s4_fused_adaptive_observed",
            driver_id: "pexprb54s4-fused",
            routing: None,
        };
        let record = SwitchingSegment {
            method: SwitchingMethod::Rodas5p,
            start_time: start,
            target_time: target,
            reached_time: reached,
            windows: 1,
            is_probe: probe,
            committed: false,
            success,
            attempts,
            accepted_steps: res.diagnostics.accepted_steps,
            rejected_steps: res.diagnostics.rejected_steps,
            rhs_calls: c.rhs_calls,
            jacobian_builds: c.jacobian_builds,
            jvp_vectors: c.jvp_vectors,
            ft_calls: c.ft_calls,
            linear_solves: c.linear_solves,
            work_units,
            decision: "",
            evidence: SwitchingEvidence::Uncompared,
            initial_step: opts.h0,
            proposed_next_step,
            comparator_work_ratio: None,
            entry_probe_reason: None,
            driver: SwitchingDriverStats::Rodas5p(Box::new(stats)),
        };
        Ok(BatchOutcome {
            record,
            counters: c,
            state,
            landed,
            stop,
            proposed_next_step,
        })
    }
}

/// Compute the 64-bit tolerance epoch digest from `Options`.
pub fn compute_tolerance_epoch(opts: &Options) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    opts.rtol.to_bits().hash(&mut hasher);
    match &opts.atol {
        crate::options::Atol::Scalar(a) => {
            0u8.hash(&mut hasher);
            a.to_bits().hash(&mut hasher);
        }
        crate::options::Atol::Vector(v) => {
            1u8.hash(&mut hasher);
            v.len().hash(&mut hasher);
            for x in v {
                x.to_bits().hash(&mut hasher);
            }
        }
    }
    std::mem::discriminant(&opts.interior_error).hash(&mut hasher);
    hasher.finish()
}

/// Compute the 64-bit provider epoch digest from `RouteId` and `Problem`.
pub fn compute_provider_epoch(route: RouteId, problem: &Problem) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (route as usize).hash(&mut hasher);
    match &problem.jacobian {
        crate::problem::Jacobian::Dense(_) => 1u8.hash(&mut hasher),
        crate::problem::Jacobian::Banded { lower, upper, .. } => {
            2u8.hash(&mut hasher);
            lower.hash(&mut hasher);
            upper.hash(&mut hasher);
        }
        crate::problem::Jacobian::Jvp(_) => 3u8.hash(&mut hasher),
        crate::problem::Jacobian::FiniteDifference => 4u8.hash(&mut hasher),
    }
    hasher.finish()
}

/// Compute the 64-bit window epoch digest from checkpoint index and window endpoints.
pub fn compute_window_epoch(checkpoint_index: usize, start: f64, target: f64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    checkpoint_index.hash(&mut hasher);
    start.to_bits().hash(&mut hasher);
    target.to_bits().hash(&mut hasher);
    hasher.finish()
}

/// Formal state machine states (Section 5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SolverState {
    /// Initial unstarted state before integration.
    Initial,
    /// Main tenure executing explicit batches.
    ExplicitTenure,
    /// Exploratory implicit entry trial.
    EntryTrial,
    /// Main tenure executing implicit batches.
    ImplicitTenure,
    /// Exploratory return trial toward explicit execution.
    ReturnTrial,
    /// Periodic sentinel check to prevent lock-in.
    ReturnSentinel,
    /// Terminal stopped state.
    Stopped,
}

/// Transition causes of the solver state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionCause {
    /// Initial start of the solver.
    InitialStart,
    /// Explicit batch completed normally without pressure.
    ExplicitBatchDone,
    /// Explicit batch encountered pressure and entry probe is due.
    PressureAndDue,
    /// Entry pilot won with strict lower work.
    PilotWonStrict,
    /// Entry pilot lost or was stopped by ceiling.
    PilotLostOrCeiling,
    /// Implicit batch completed normally.
    ImplicitBatchDone,
    /// Return recheck is due.
    RecheckDue,
    /// Sentinel check is due.
    SentinelDue,
    /// Return trial won with strict lower work.
    ReturnWonStrict,
    /// Return trial lost to the incumbent.
    ReturnLost,
    /// Sentinel trial won against the implicit window.
    SentinelWon,
    /// Sentinel trial lost to the implicit window.
    SentinelLost,
    /// Fatal error encountered during execution.
    FatalStop,
    /// Attempt budget exhausted during execution.
    BudgetStop,
}

/// Transition function for the formal solver state machine.
pub fn next_solver_state(
    current: SolverState,
    cause: TransitionCause,
) -> Result<SolverState, &'static str> {
    use SolverState::*;
    use TransitionCause::*;

    match (current, cause) {
        (Initial, InitialStart) => Ok(ExplicitTenure),
        (ExplicitTenure, ExplicitBatchDone) => Ok(ExplicitTenure),
        (ExplicitTenure, PressureAndDue) => Ok(EntryTrial),
        (ExplicitTenure, FatalStop | BudgetStop) => Ok(Stopped),

        (EntryTrial, PilotWonStrict) => Ok(ImplicitTenure),
        (EntryTrial, PilotLostOrCeiling) => Ok(ExplicitTenure),
        (EntryTrial, FatalStop) => Ok(Stopped),

        (ImplicitTenure, ImplicitBatchDone) => Ok(ImplicitTenure),
        (ImplicitTenure, RecheckDue) => Ok(ReturnTrial),
        (ImplicitTenure, SentinelDue) => Ok(ReturnSentinel),
        (ImplicitTenure, FatalStop | BudgetStop) => Ok(Stopped),

        (ReturnTrial, ReturnWonStrict) => Ok(ExplicitTenure),
        (ReturnTrial, ReturnLost) => Ok(ImplicitTenure),

        (ReturnSentinel, SentinelWon) => Ok(ExplicitTenure),
        (ReturnSentinel, SentinelLost) => Ok(ImplicitTenure),

        (Stopped, _) => Err("cannot transition out of Stopped state"),
        _ => Err("illegal state transition"),
    }
}
