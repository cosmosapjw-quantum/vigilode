//! Bounded, opt-in explicit/RODAS switching at accepted window boundaries.
//!
//! This adapter uses observed work, not a stiffness estimate. Its default is
//! RHS-only. Derivative fallbacks require a separate explicit authorization.

use rodas5p_integrators::{AdaptiveStepConfig, Dopri5StopReason};
use serde::Serialize;

use crate::{
    Atol, Dopri5, ExplicitStats, Jacobian, Options, Problem, Rodas5p, SessionEpochs, Stats, Status,
};

/// Authorization for derivative work after the explicit path shows pressure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DerivativeFallback {
    /// Use one persistent RHS-only DOPRI5 session with window observations.
    #[default]
    Forbidden,
    /// Permit only a supplied dense/banded Jacobian or JVP provider; require
    /// either autonomy or a supplied explicit time derivative as well.
    ProvidedOnly,
    /// Also permit the facade's finite-difference Jacobian provider.
    AllowFiniteDifference,
}

/// Controls of the opt-in window adapter; units of work are deterministic.
#[derive(Clone, Debug, Serialize)]
pub struct SwitchingConfig {
    /// Authorized derivative route. The default invokes no derivative callback.
    pub fallback: DerivativeFallback,
    /// Equal time windows; output includes each represented window endpoint.
    pub window_count: usize,
    /// Maximum attempts of one explicit batch before reconsidering its route.
    pub explicit_attempt_budget: usize,
    /// A completed explicit window with at least this many attempts is costly.
    pub pressure_steps: usize,
    /// Rejection fraction that marks a completed explicit window as costly.
    pub rejection_fraction: f64,
    /// Minimum completed implicit windows before an explicit recheck.
    pub implicit_residence: usize,
    /// Completed implicit windows between bounded explicit rechecks.
    pub probe_interval: usize,
    /// Attempts allowed for one explicit recheck (also charged globally).
    pub probe_attempt_budget: usize,
    /// Recheck work/time must be at most this fraction of the last implicit rate.
    pub return_cost_ratio: f64,
    /// Refuse a possible dense fallback above this dimension, before callbacks.
    pub max_dense_dimension: usize,
    /// Attempts permitted for an implicit pilot from the same checkpoint and
    /// over the same accepted time interval as an explicit batch. Zero retains
    /// the original pressure-only entry policy. Nonzero enables cost screening.
    pub implicit_pilot_attempt_budget: usize,
    /// Maximum implicit entry pilots during the entire solve. Exhausting this
    /// limit keeps the explicit route; it does not grant implicit permission.
    pub implicit_pilot_limit: usize,
    /// A successful entry pilot must cost at most this fraction of the explicit
    /// batch's deterministic work units. This is not a wall-clock guarantee.
    /// A pilot is also stopped as soon as its charged callback work alone
    /// exceeds this bound, since it could no longer pass.
    pub entry_cost_ratio: f64,
    /// Keep a native DOPRI session, including controller, FSAL state and
    /// workspace, across contiguous explicit windows. A committed RODAS step
    /// ends that tenure; an accepted explicit recheck may seed the new tenure's
    /// initial proposal. False retains the legacy per-batch explicit driver.
    pub carry_explicit_step: bool,
    /// Explicit batches between implicit entry pilots. Zero preserves legacy
    /// scheduling. This permits later re-evaluation without a pilot every batch.
    pub entry_pilot_cooldown_batches: usize,
    /// Growth factor of the entry-pilot cooldown after every discarded pilot
    /// beyond the first; 1 keeps the cooldown fixed. A zero cooldown grows
    /// from one batch. An explicit work rate at least `rearm_rate_ratio`
    /// times the rate observed at the last discarded pilot re-arms the pilot
    /// at once.
    pub pilot_backoff_factor: usize,
    /// Growth factor of the explicit recheck interval after every discarded
    /// recheck beyond the first, applied at most twice; 1 keeps the interval
    /// fixed. An implicit work rate falling to `1 / rearm_rate_ratio` of the
    /// rate observed at the last discarded recheck re-arms the recheck at once.
    pub probe_backoff_factor: usize,
    /// Work-rate trend ratio, above one, that re-arms a backed-off trial.
    pub rearm_rate_ratio: f64,
}

impl Default for SwitchingConfig {
    fn default() -> Self {
        Self {
            fallback: DerivativeFallback::Forbidden,
            window_count: 8,
            explicit_attempt_budget: 32,
            pressure_steps: 24,
            rejection_fraction: 0.3,
            implicit_residence: 2,
            probe_interval: 4,
            probe_attempt_budget: 16,
            return_cost_ratio: 0.8,
            max_dense_dimension: 64,
            implicit_pilot_attempt_budget: 0,
            implicit_pilot_limit: 1,
            entry_cost_ratio: 0.8,
            carry_explicit_step: false,
            entry_pilot_cooldown_batches: 0,
            pilot_backoff_factor: 2,
            probe_backoff_factor: 2,
            rearm_rate_ratio: 2.0,
        }
    }
}

impl SwitchingConfig {
    /// Bounded work-based selection with step-proposal carry and repeated,
    /// separated entry trials. Derivative permission remains explicit.
    /// This classifies observed method fitness, not physical stiffness.
    pub fn evidence_driven(fallback: DerivativeFallback) -> Self {
        Self {
            fallback,
            implicit_pilot_attempt_budget: 64,
            implicit_pilot_limit: 8,
            entry_pilot_cooldown_batches: 4,
            carry_explicit_step: true,
            ..Self::default()
        }
    }

    /// The entry-pilot cooldown after `discards` consecutive discarded
    /// pilots: a single discard keeps the registered cooldown, every further
    /// one multiplies it by the back-off factor.
    fn pilot_cooldown(&self, discards: usize) -> usize {
        if discards <= 1 || self.pilot_backoff_factor == 1 {
            return self.entry_pilot_cooldown_batches;
        }
        self.entry_pilot_cooldown_batches.max(1).saturating_mul(
            self.pilot_backoff_factor
                .saturating_pow((discards - 1).min(32) as u32),
        )
    }

    /// The explicit recheck interval after `discards` consecutive discarded
    /// rechecks: a single discard keeps the registered interval, every
    /// further one multiplies it by the factor, at most twice.
    fn recheck_interval(&self, discards: usize) -> usize {
        if discards <= 1 || self.probe_backoff_factor == 1 {
            return self.probe_interval;
        }
        self.probe_interval.saturating_mul(
            self.probe_backoff_factor
                .saturating_pow((discards - 1).min(2) as u32),
        )
    }
}

/// What the observed batch establishes about method fitness.
/// None of these variants certifies a physical stiff/nonstiff regime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SwitchingEvidence {
    /// No competitive comparison was performed.
    #[default]
    Uncompared,
    /// An explicit batch required enough work to trigger reconsideration.
    ExplicitWorkPressure,
    /// Same-checkpoint implicit trial completed with lower charged work.
    ImplicitCheaperOnPrefix,
    /// Completed implicit trial did not meet the required work advantage.
    ImplicitNotCheaperOnPrefix,
    /// Implicit trial did not reach the matched endpoint.
    ImplicitPilotIncomplete,
    /// Successful explicit return trial beat the previous implicit work rate.
    ExplicitCheaperThanPreviousWindow,
    /// The bounded explicit return trial did not justify returning.
    ExplicitReturnNotSupported,
    /// An implicit batch completed; its rate is recorded for later comparison.
    ImplicitWindowCompleted,
}

/// Executed method of an adapter batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SwitchingMethod {
    /// RHS-only Dormand–Prince 5(4).
    Dopri5,
    /// Protected RODAS5P with the authorized derivative provider.
    Rodas5p,
}

/// Original driver telemetry, including all callback and failure details.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "method", content = "stats", rename_all = "kebab-case")]
pub enum SwitchingDriverStats {
    /// Full explicit record.
    Dopri5(Box<ExplicitStats>),
    /// Full protected RODAS record.
    Rodas5p(Box<Stats>),
}

/// Work and disposition of one attempted batch; discarded probes remain here.
#[derive(Clone, Debug, Serialize)]
pub struct SwitchingSegment {
    /// Executed method.
    pub method: SwitchingMethod,
    /// Accepted initial time of the batch.
    pub start_time: f64,
    /// Requested window endpoint.
    pub target_time: f64,
    /// Last internally accepted time of this batch.
    pub reached_time: f64,
    /// Windows this batch was asked to cover: one, except for an implicit
    /// tenure that runs every window up to its next scheduled recheck in one
    /// native run, landing on each window endpoint.
    pub windows: usize,
    /// Whether this is a bounded explicit recheck or implicit entry replay.
    pub is_probe: bool,
    /// Whether its accepted endpoint became the adapter's physical state.
    pub committed: bool,
    /// Whether the batch reached its requested endpoint.
    pub success: bool,
    /// Actual accepted plus rejected attempts.
    pub attempts: usize,
    /// Internally accepted attempts; include discarded probe work.
    pub accepted_steps: usize,
    /// Internally rejected attempts.
    pub rejected_steps: usize,
    /// RHS calls, including finite differences and rejected attempts.
    pub rhs_calls: u64,
    /// Jacobian builds.
    pub jacobian_builds: u64,
    /// JVP vectors.
    pub jvp_vectors: u64,
    /// Explicit time-derivative calls.
    pub ft_calls: u64,
    /// Linear solves.
    pub linear_solves: u64,
    /// Deterministic proxy: RHS + JVP + time-derivative + n*Jacobian + solves.
    /// This is a routing score, not elapsed time or a FLOP count.
    pub work_units: f64,
    /// Reason for the disposition or route decision.
    pub decision: &'static str,
    /// Typed evidence for selection; distinct from a physical regime label.
    pub evidence: SwitchingEvidence,
    /// Initial proposal actually supplied to this batch, before driver clipping.
    pub initial_step: f64,
    /// Native DOPRI proposal at batch exit when step carry is enabled.
    /// Absent at full-span completion and for the implicit driver, whose
    /// controller is not exported here.
    pub proposed_next_step: Option<f64>,
    /// Pilot work divided by its explicit comparator, when a matched-prefix
    /// comparison was performed. Includes all charged attempts, not just accepts.
    pub comparator_work_ratio: Option<f64>,
    /// Driver telemetry for this batch. Persistent explicit work fields are
    /// differences of cumulative native snapshots; its `options` and
    /// `adaptive` describe the fixed tenure-wide native session. The batch
    /// quota is bounded by `attempts` and `SwitchingConfig`.
    pub driver: SwitchingDriverStats,
}

/// Full charged work of the adapter.
#[derive(Clone, Debug, Default, Serialize)]
pub struct SwitchingStats {
    /// All attempts, including discarded probes; bounded by Options::max_steps.
    pub attempts: usize,
    /// Internally accepted work, including discarded probes.
    pub accepted_steps: usize,
    /// Internally rejected work.
    pub rejected_steps: usize,
    /// Total charged RHS calls.
    pub rhs_calls: u64,
    /// Total Jacobian builds.
    pub jacobian_builds: u64,
    /// Total JVP vectors.
    pub jvp_vectors: u64,
    /// Total time-derivative calls.
    pub ft_calls: u64,
    /// Total linear solves.
    pub linear_solves: u64,
    /// Changes of the active route, excluding a rejected explicit probe.
    pub switches: usize,
    /// Whether every entered driver returned its work record.
    pub accounting_complete: bool,
    /// Every attempted batch, including uncommitted probes, in disposition
    /// order. An implicit entry replay precedes the explicit candidate it
    /// evaluates here, although the explicit candidate was executed first.
    pub segments: Vec<SwitchingSegment>,
}

/// Endpoint results of the opt-in adapter.
#[derive(Clone, Debug, Serialize)]
pub struct SwitchingSolution {
    /// Stable schema of this new, separately opted-in API.
    pub schema: &'static str,
    /// Initial and committed batch endpoint times.
    pub t: Vec<f64>,
    /// States corresponding to `t`.
    pub y: Vec<Vec<f64>>,
    /// Whether the final time was reached with local error acceptance.
    pub success: bool,
    /// Completion or stop detail.
    pub message: String,
    /// Last committed time.
    pub final_time: f64,
    /// Last committed state.
    pub final_state: Vec<f64>,
    /// Requested tolerance and total attempt limit.
    pub options: Options,
    /// Actual switching policy.
    pub config: SwitchingConfig,
    /// Charged work, routes, probes and original driver records.
    pub stats: SwitchingStats,
}

/// Invalid or unsupported request, always detected before user callbacks.
#[derive(Debug, thiserror::Error)]
#[error("invalid switching request: {0}")]
pub struct SwitchingError(pub String);

/// RHS-first, derivative-opt-in adaptive window adapter.
///
/// Smooth forward identity-mass IVPs with endpoint observations are supported.
/// Individual drivers retain their local error tests. Arbitrary discontinuities,
/// stiffness and oscillation frequency are not inferred from a work counter.
#[derive(Clone, Copy, Debug, Default)]
pub struct SwitchingSolver;

impl SwitchingSolver {
    /// Integrate and charge every trial to one total `Options::max_steps` budget.
    ///
    /// A switch takes place only at a committed endpoint. An explicit recheck
    /// from that endpoint is published only when it reaches the window and
    /// meets the work-rate hysteresis. Fatal callbacks stop the whole adapter;
    /// no second method retries them. Callbacks must represent a fixed function
    /// during the solve, since a discarded probe may evaluate the same region
    /// again. With `carry_explicit_step`, contiguous explicit windows share one
    /// native DOPRI5 session; committing an implicit step invalidates it.
    /// With a nonzero `implicit_pilot_attempt_budget`, entry requires a bounded
    /// replay over the explicit candidate's accepted prefix. This experimental
    /// comparison uses charged work and local tolerances, not measured elapsed
    /// time or a certified global-error equivalence. Rejected explicit attempts
    /// may have reached beyond that prefix, so its work comparison is heuristic.
    ///
    /// Discarded trials are bounded by the work they would need to pass: an
    /// entry pilot stops once its charged callback work exceeds
    /// `entry_cost_ratio` times the explicit batch's work, and an explicit
    /// recheck stops once its RHS work exceeds `return_cost_ratio` times the
    /// implicit rate over its window. Neither bound changes a decision. After
    /// a discarded trial the next one waits longer (`pilot_backoff_factor`,
    /// `probe_backoff_factor`), unless the observed work rate moves by
    /// `rearm_rate_ratio`. An implicit tenure integrates every window up to
    /// its next recheck in one native run, landing on each window endpoint,
    /// and seeds a later implicit run with its mean accepted step.
    pub fn solve(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        config: &SwitchingConfig,
    ) -> Result<SwitchingSolution, SwitchingError> {
        validate(problem, t_span, y0, opts, config)?;
        let (t0, t1) = t_span;
        let mut result = SwitchingSolution {
            schema: "vigilode.switching.v1",
            t: vec![t0],
            y: vec![y0.to_vec()],
            success: false,
            message: String::new(),
            final_time: t0,
            final_state: y0.to_vec(),
            options: opts.clone(),
            config: config.clone(),
            stats: SwitchingStats {
                accounting_complete: true,
                ..Default::default()
            },
        };
        let fallback = config.fallback != DerivativeFallback::Forbidden;
        if !fallback {
            return Ok(persistent_explicit(
                problem, t_span, y0, opts, config, result,
            ));
        }
        if t0 == t1 {
            result.success = true;
            result.message = "reached final time".into();
            return Ok(result);
        }
        let mut progress = Some(DriveState {
            result,
            method: SwitchingMethod::Dopri5,
            implicit_windows: 0,
            since_probe: 0,
            implicit_rate: None,
            implicit_pilots: 0,
            batches_since_entry_pilot: usize::MAX,
            explicit_step: None,
            next_window: 1,
            pilot_discards: 0,
            explicit_rate_at_discard: None,
            probe_discards: 0,
            implicit_rate_at_discard: None,
            implicit_seed: None,
        });
        while progress.as_ref().unwrap().next_window <= config.window_count {
            let current = progress.as_mut().unwrap();
            if current.result.stats.attempts == opts.max_steps {
                if current.result.final_time < t1 {
                    current.result.message = "total attempt budget exhausted".into();
                }
                break;
            }
            if progress.as_ref().unwrap().method == SwitchingMethod::Dopri5
                && config.carry_explicit_step
            {
                let current = progress.as_ref().unwrap();
                let start = current.result.final_time;
                let mut times = vec![start];
                for window in current.next_window..=config.window_count {
                    let target = window_target(t_span, window, config.window_count);
                    if target > *times.last().unwrap() {
                        times.push(target);
                    }
                }
                let mut session_opts = opts.clone();
                // Native max_steps remains a tenure-wide ceiling. The adapter
                // supplies each batch's smaller quota to advance_to.
                session_opts.max_steps = opts.max_steps - current.result.stats.attempts;
                session_opts.h0 = if config.carry_explicit_step {
                    current.explicit_step.unwrap_or(opts.h0)
                } else {
                    opts.h0
                };
                let initial = current.result.final_state.clone();
                let outcome = Dopri5::with_session(
                    problem,
                    (start, t1),
                    &initial,
                    &session_opts,
                    &times,
                    SessionEpochs::default(),
                    |active| {
                        let state = progress.take().unwrap();
                        progress = Some(drive_windows(
                            problem,
                            t_span,
                            opts,
                            config,
                            state,
                            Some(active),
                        ));
                    },
                );
                if let Err(error) = outcome {
                    let state = progress.as_mut().unwrap();
                    state.result.message = error.to_string();
                    state.result.stats.accounting_complete = false;
                    break;
                }
            } else {
                let state = progress.take().unwrap();
                progress = Some(drive_windows(problem, t_span, opts, config, state, None));
            }
        }
        let mut result = progress.unwrap().result;
        result.success = result.final_time == t1;
        if result.success {
            result.message = "reached final time".into();
        }
        if result.message.is_empty() {
            result.message = "no representable forward progress".into();
        }
        Ok(result)
    }
}

struct DriveState {
    result: SwitchingSolution,
    method: SwitchingMethod,
    implicit_windows: usize,
    since_probe: usize,
    implicit_rate: Option<f64>,
    implicit_pilots: usize,
    batches_since_entry_pilot: usize,
    explicit_step: Option<f64>,
    next_window: usize,
    /// Consecutive discarded entry pilots since the last route change.
    pilot_discards: usize,
    /// Explicit work rate of the batch whose pilot was last discarded.
    explicit_rate_at_discard: Option<f64>,
    /// Consecutive discarded explicit rechecks since the last route change.
    probe_discards: usize,
    /// Implicit work rate before the last discarded recheck.
    implicit_rate_at_discard: Option<f64>,
    /// Mean accepted step of the last completed implicit batch; the initial
    /// proposal of the next implicit batch of the same tenure.
    implicit_seed: Option<f64>,
}

fn window_target((t0, t1): (f64, f64), window: usize, count: usize) -> f64 {
    if window == count {
        t1
    } else {
        t0 + (t1 - t0) * (window as f64 / count as f64)
    }
}

fn drive_windows(
    problem: &Problem,
    (t0, t1): (f64, f64),
    opts: &Options,
    config: &SwitchingConfig,
    progress: DriveState,
    mut session: Option<&mut crate::session::ExplicitSession<'_>>,
) -> DriveState {
    let DriveState {
        mut result,
        mut method,
        mut implicit_windows,
        mut since_probe,
        mut implicit_rate,
        mut implicit_pilots,
        mut batches_since_entry_pilot,
        mut explicit_step,
        next_window,
        mut pilot_discards,
        mut explicit_rate_at_discard,
        mut probe_discards,
        mut implicit_rate_at_discard,
        mut implicit_seed,
    } = progress;
    let fallback = config.fallback != DerivativeFallback::Forbidden;
    // One mutable copy of the request serves every batch of this call.
    let mut local = opts.clone();
    'windows: for window in next_window..=config.window_count {
        if result.final_time == t1 {
            break;
        }
        let target = window_target((t0, t1), window, config.window_count);
        if target <= result.final_time {
            continue;
        }
        let mut probe = method == SwitchingMethod::Rodas5p
            && implicit_windows >= config.implicit_residence
            && since_probe >= config.recheck_interval(probe_discards);
        while result.final_time < target {
            // Leaving a tenure drops its native controller and FSAL state.
            // A later explicit tenure constructs a fresh session from the
            // committed post-implicit state.
            if (session.is_some() && method != SwitchingMethod::Dopri5)
                || (session.is_none()
                    && method == SwitchingMethod::Dopri5
                    && config.carry_explicit_step)
            {
                return DriveState {
                    result,
                    method,
                    implicit_windows,
                    since_probe,
                    implicit_rate,
                    implicit_pilots,
                    batches_since_entry_pilot,
                    explicit_step,
                    next_window: window,
                    pilot_discards,
                    explicit_rate_at_discard,
                    probe_discards,
                    implicit_rate_at_discard,
                    implicit_seed,
                };
            }
            let remaining = opts.max_steps - result.stats.attempts;
            if remaining == 0 {
                result.message = "total attempt budget exhausted".into();
                break 'windows;
            }
            let start = result.final_time;
            // A replay pilot is rooted at the identical accepted physical
            // checkpoint. It may never replace the already tested explicit
            // candidate, and every discarded trial is charged. Eligibility is
            // judged before the batch; a work-rate jump may re-arm it after.
            let pilot_eligible = config.implicit_pilot_attempt_budget > 0
                && method == SwitchingMethod::Dopri5
                && implicit_pilots < config.implicit_pilot_limit
                && batches_since_entry_pilot >= config.pilot_cooldown(pilot_discards);
            let selected = if probe {
                SwitchingMethod::Dopri5
            } else {
                method
            };
            // An implicit tenure runs every window up to its next scheduled
            // recheck in one native run, landing on each window endpoint.
            let (batch_target, windows) = if selected == SwitchingMethod::Rodas5p {
                let until_recheck = if implicit_windows < config.implicit_residence {
                    config.implicit_residence - implicit_windows
                } else {
                    config
                        .recheck_interval(probe_discards)
                        .saturating_sub(since_probe)
                        .max(1)
                };
                let last = window
                    .saturating_add(until_recheck - 1)
                    .min(config.window_count);
                (
                    window_target((t0, t1), last, config.window_count),
                    last - window + 1,
                )
            } else {
                (target, 1)
            };
            local.h0 = match selected {
                SwitchingMethod::Dopri5 if config.carry_explicit_step => {
                    explicit_step.unwrap_or(opts.h0)
                }
                SwitchingMethod::Dopri5 => opts.h0,
                SwitchingMethod::Rodas5p => implicit_seed.unwrap_or(opts.h0),
            }
            .min(batch_target - start);
            local.max_steps = remaining.min(match selected {
                SwitchingMethod::Dopri5 if probe => config.probe_attempt_budget,
                SwitchingMethod::Dopri5 if fallback => config.explicit_attempt_budget,
                // With no fallback/probes, a single driver can use the
                // entire remaining budget instead of needless batch restarts.
                _ => remaining,
            });
            // A recheck can only return when its work rate stays within the
            // hysteresis of the implicit rate: RHS work past that bound ends it.
            let return_ceiling = if probe {
                implicit_rate
                    .map(|rate| (config.return_cost_ratio * rate * (target - start)).floor() as u64)
            } else {
                None
            };
            let batch_result =
                if selected == SwitchingMethod::Dopri5 && !probe && config.carry_explicit_step {
                    match session.as_deref_mut() {
                        Some(active) => run_persistent_explicit(active, start, target, t1, &local),
                        None => Err("explicit tenure has no active native session".into()),
                    }
                } else {
                    let landings: Vec<f64> = if windows > 1 {
                        (window..window + windows - 1)
                            .map(|w| window_target((t0, t1), w, config.window_count))
                            .filter(|t| *t > start)
                            .collect()
                    } else {
                        Vec::new()
                    };
                    run(
                        problem,
                        start,
                        batch_target,
                        &result.final_state,
                        &local,
                        selected,
                        probe,
                        config.carry_explicit_step,
                        t1,
                        &landings,
                        return_ceiling,
                    )
                };
            let batch = match batch_result {
                Ok(batch) => batch,
                Err(message) => {
                    result.message = message;
                    result.stats.accounting_complete = false;
                    break 'windows;
                }
            };
            let Batch {
                mut record,
                state,
                landings,
                fatal,
                budget_stop,
                ceiling_stop,
            } = batch;
            record.windows = windows;
            charge(&mut result.stats, &record);
            if record.attempts == 0 && record.reached_time < batch_target && !fatal {
                record.decision = "driver stopped without progress or attempted step";
                result.message = record.decision.into();
                result.stats.segments.push(record);
                break 'windows;
            }
            if probe {
                since_probe = 0;
                let rate = record.work_units / (target - start);
                let economical = record.success
                    && implicit_rate.is_some_and(|old| rate <= config.return_cost_ratio * old);
                if economical {
                    record.decision = "explicit probe completed within return work-rate hysteresis";
                    record.evidence = SwitchingEvidence::ExplicitCheaperThanPreviousWindow;
                    explicit_step = record.proposed_next_step;
                    commit(&mut result, &mut record, state, landings);
                    method = SwitchingMethod::Dopri5;
                    result.stats.switches += 1;
                    implicit_windows = 0;
                    pilot_discards = 0;
                    explicit_rate_at_discard = None;
                    probe_discards = 0;
                    implicit_rate_at_discard = None;
                    implicit_seed = None;
                } else if fatal {
                    record.decision = "fatal explicit probe; stop without another driver";
                    commit(&mut result, &mut record, state, landings);
                    result.message = record.decision.into();
                    result.stats.segments.push(record);
                    break 'windows;
                } else {
                    record.decision = if ceiling_stop {
                        "bounded explicit probe discarded: exceeded return work ceiling"
                    } else {
                        "bounded explicit probe discarded; retain implicit route"
                    };
                    record.evidence = SwitchingEvidence::ExplicitReturnNotSupported;
                    probe_discards += 1;
                    implicit_rate_at_discard = implicit_rate;
                }
                result.stats.segments.push(record);
                probe = false;
                continue;
            }
            if fatal {
                commit(&mut result, &mut record, state, landings);
                record.decision = "fatal callback; stop without fallback";
                result.message = record.decision.into();
                result.stats.segments.push(record);
                break 'windows;
            }
            match selected {
                SwitchingMethod::Dopri5 => {
                    batches_since_entry_pilot = batches_since_entry_pilot.saturating_add(1);
                    let rejection_pressure = record.attempts > 0
                        && record.rejected_steps as f64 / record.attempts as f64
                            >= config.rejection_fraction;
                    let pressure = budget_stop
                        || record.attempts >= config.pressure_steps
                        || rejection_pressure;
                    if pressure {
                        record.evidence = SwitchingEvidence::ExplicitWorkPressure;
                    }
                    // Work per unit time of this batch; a jump over the rate
                    // seen at the last discarded pilot re-arms the pilot now.
                    let rate = if record.reached_time > start {
                        record.work_units / (record.reached_time - start)
                    } else {
                        f64::INFINITY
                    };
                    let rearm = rate.is_finite()
                        && explicit_rate_at_discard
                            .is_some_and(|old| rate >= config.rearm_rate_ratio * old);
                    if rearm {
                        pilot_discards = 0;
                    }
                    if fallback
                        && pressure
                        && (record.success || budget_stop)
                        && record.reached_time < t1
                    {
                        let mut enter = config.implicit_pilot_attempt_budget == 0;
                        if config.implicit_pilot_attempt_budget > 0 {
                            let remaining = opts.max_steps - result.stats.attempts;
                            if (pilot_eligible || rearm)
                                && implicit_pilots < config.implicit_pilot_limit
                                && remaining > 0
                                && record.reached_time > start
                            {
                                implicit_pilots += 1;
                                batches_since_entry_pilot = 0;
                                local.h0 = opts.h0.min(record.reached_time - start);
                                local.max_steps =
                                    remaining.min(config.implicit_pilot_attempt_budget);
                                // Charged callback work alone past this bound
                                // means the replay cannot pass: stop it there.
                                let entry_ceiling =
                                    (config.entry_cost_ratio * record.work_units).floor() as u64;
                                match run(
                                    problem,
                                    start,
                                    record.reached_time,
                                    &result.final_state,
                                    &local,
                                    SwitchingMethod::Rodas5p,
                                    true,
                                    false,
                                    t1,
                                    &[],
                                    Some(entry_ceiling),
                                ) {
                                    Ok(mut pilot) => {
                                        charge(&mut result.stats, &pilot.record);
                                        enter = pilot.record.success
                                            && pilot.record.work_units
                                                <= config.entry_cost_ratio * record.work_units;
                                        pilot.record.comparator_work_ratio =
                                            if record.work_units > 0.0 {
                                                Some(pilot.record.work_units / record.work_units)
                                            } else {
                                                None
                                            };
                                        pilot.record.evidence = if !pilot.record.success {
                                            SwitchingEvidence::ImplicitPilotIncomplete
                                        } else if enter {
                                            SwitchingEvidence::ImplicitCheaperOnPrefix
                                        } else {
                                            SwitchingEvidence::ImplicitNotCheaperOnPrefix
                                        };
                                        pilot.record.decision = if enter {
                                            "same-checkpoint implicit pilot passed entry work ratio"
                                        } else if pilot.ceiling_stop {
                                            "same-checkpoint implicit pilot discarded: exceeded entry work ceiling"
                                        } else {
                                            "same-checkpoint implicit pilot discarded: incomplete or uneconomical"
                                        };
                                        let pilot_fatal = pilot.fatal;
                                        if !enter && !pilot_fatal {
                                            pilot_discards += 1;
                                            explicit_rate_at_discard = Some(rate);
                                        }
                                        result.stats.segments.push(pilot.record);
                                        if pilot_fatal {
                                            record.decision = "fatal implicit pilot; retain accepted explicit state and stop";
                                            commit(&mut result, &mut record, state, landings);
                                            result.message = record.decision.into();
                                            result.stats.segments.push(record);
                                            break 'windows;
                                        }
                                    }
                                    Err(message) => {
                                        result.stats.accounting_complete = false;
                                        record.decision =
                                            "implicit pilot failed without a work record; stop";
                                        commit(&mut result, &mut record, state, landings);
                                        result.message = message;
                                        result.stats.segments.push(record);
                                        break 'windows;
                                    }
                                }
                            }
                        }
                        if enter {
                            record.decision = if config.implicit_pilot_attempt_budget == 0 {
                                "observed explicit work pressure; enter authorized RODAS route"
                            } else {
                                "observed explicit pressure and economical replay; enter authorized RODAS route"
                            };
                            method = SwitchingMethod::Rodas5p;
                            implicit_windows = 0;
                            since_probe = 0;
                            result.stats.switches += 1;
                            pilot_discards = 0;
                            explicit_rate_at_discard = None;
                            probe_discards = 0;
                            implicit_rate_at_discard = None;
                            implicit_seed = None;
                        } else {
                            record.decision = "implicit entry not supported by bounded pilot evidence; retain explicit route";
                        }
                    } else if !record.success && !budget_stop {
                        commit(&mut result, &mut record, state, landings);
                        record.decision = "explicit driver stopped; no eligible fallback";
                        result.message = record.decision.into();
                        result.stats.segments.push(record);
                        break 'windows;
                    } else {
                        record.decision = "explicit window completed below work pressure";
                    }
                }
                SwitchingMethod::Rodas5p => {
                    if !record.success {
                        commit(&mut result, &mut record, state, landings);
                        record.decision =
                            "implicit driver stopped; preserve partial accepted state";
                        result.message = record.decision.into();
                        result.stats.segments.push(record);
                        break 'windows;
                    }
                    record.evidence = SwitchingEvidence::ImplicitWindowCompleted;
                    let rate = record.work_units / (batch_target - start);
                    // An implicit rate falling well below the rate seen at
                    // the last discarded recheck re-arms the recheck now.
                    if implicit_rate_at_discard
                        .is_some_and(|old| rate * config.rearm_rate_ratio <= old)
                    {
                        probe_discards = 0;
                        implicit_rate_at_discard = None;
                        since_probe = config.probe_interval.max(windows);
                    } else {
                        since_probe += windows;
                    }
                    implicit_rate = Some(rate);
                    implicit_windows += windows;
                    implicit_seed = (record.accepted_steps > 0)
                        .then(|| (record.reached_time - start) / record.accepted_steps as f64);
                    record.decision =
                        "implicit window completed; retain route until bounded recheck";
                }
            }
            explicit_step = if selected == SwitchingMethod::Dopri5 {
                record.proposed_next_step
            } else {
                None
            };
            commit(&mut result, &mut record, state, landings);
            result.stats.segments.push(record);
        }
    }
    DriveState {
        result,
        method,
        implicit_windows,
        since_probe,
        implicit_rate,
        implicit_pilots,
        batches_since_entry_pilot,
        explicit_step,
        next_window: config.window_count + 1,
        pilot_discards,
        explicit_rate_at_discard,
        probe_discards,
        implicit_rate_at_discard,
        implicit_seed,
    }
}

// No method competition is possible under Forbidden. One native observed
// solve preserves its stage workspace, FSAL derivative and controller across
// all requested windows. This branch does not alter authorized FD/JVP routing.
fn persistent_explicit(
    problem: &Problem,
    (t0, t1): (f64, f64),
    y0: &[f64],
    opts: &Options,
    config: &SwitchingConfig,
    mut result: SwitchingSolution,
) -> SwitchingSolution {
    let mut times = Vec::with_capacity(config.window_count + 1);
    times.push(t0);
    for window in 1..=config.window_count {
        let t = if window == config.window_count {
            t1
        } else {
            t0 + (t1 - t0) * (window as f64 / config.window_count as f64)
        };
        if t > *times.last().unwrap() {
            times.push(t);
        }
    }
    let run = match Dopri5::solve_observed(problem, (t0, t1), y0, opts, &times) {
        Ok(run) => run,
        Err(error) => {
            // The native facade supplies no partial record for Err. Keep the
            // adapter's original finite state and make the accounting gap explicit.
            result.message = error.to_string();
            result.stats.accounting_complete = false;
            return result;
        }
    };
    let counters = run.stats.counters;
    let record = SwitchingSegment {
        method: SwitchingMethod::Dopri5,
        start_time: t0,
        target_time: t1,
        reached_time: run.final_time,
        windows: config.window_count,
        is_probe: false,
        committed: true,
        success: run.success,
        attempts: run.stats.attempts,
        accepted_steps: run.stats.accepted_steps,
        rejected_steps: run.stats.rejected_steps,
        rhs_calls: counters.rhs_calls,
        jacobian_builds: counters.jacobian_builds,
        jvp_vectors: counters.jvp_vectors,
        ft_calls: counters.ft_calls,
        linear_solves: counters.linear_solves,
        work_units: counters.rhs_calls as f64,
        decision: "derivative fallback forbidden; persistent RHS-only observed session",
        evidence: SwitchingEvidence::Uncompared,
        initial_step: opts.h0,
        proposed_next_step: None,
        comparator_work_ratio: None,
        driver: SwitchingDriverStats::Dopri5(Box::new(run.stats)),
    };
    charge(&mut result.stats, &record);
    result.stats.segments.push(record);
    result.t = run.t;
    result.y = run.y;
    result.success = run.success;
    result.message = run.message;
    result.final_time = run.final_time;
    result.final_state = run.final_state;
    result
}

struct Batch {
    record: SwitchingSegment,
    /// The batch's last accepted state.
    state: Vec<f64>,
    /// Landed window endpoints strictly between the start and the last
    /// accepted state of a coalesced implicit batch; empty otherwise.
    landings: Vec<(f64, Vec<f64>)>,
    fatal: bool,
    budget_stop: bool,
    /// The trial stopped because its charged work passed its ceiling.
    ceiling_stop: bool,
}

fn run_persistent_explicit(
    session: &mut crate::session::ExplicitSession<'_>,
    start: f64,
    target: f64,
    solve_end: f64,
    opts: &Options,
) -> Result<Batch, String> {
    let initial_step = session.checkpoint().proposed_h;
    let before = session.snapshot();
    let outcome = session
        .advance_to(target, opts.max_steps, SessionEpochs::default())
        .map_err(|error| error.to_string())?;
    let driver = session.stats_since(before);
    let after = session.snapshot();
    let checkpoint = session.checkpoint();
    let reached = checkpoint.t;
    let state = checkpoint.y.to_vec();
    let counters = driver.counters;
    let fatal = after.failed;
    let budget_stop = reached < target
        && (outcome == crate::SessionAdvance::Yield
            || after.stop_reason == Some(Dopri5StopReason::AttemptBudget));
    // A landing exactly on the requested observation is successful even if
    // that accepted trial also exhausted the native global attempt limit.
    let success = reached == target
        && !fatal
        && (matches!(
            outcome,
            crate::SessionAdvance::Checkpoint | crate::SessionAdvance::Complete
        ) || after.stop_reason == Some(Dopri5StopReason::AttemptBudget));
    Ok(Batch {
        record: SwitchingSegment {
            method: SwitchingMethod::Dopri5,
            start_time: start,
            target_time: target,
            reached_time: reached,
            windows: 1,
            is_probe: false,
            committed: false,
            success,
            attempts: driver.attempts,
            accepted_steps: driver.accepted_steps,
            rejected_steps: driver.rejected_steps,
            rhs_calls: counters.rhs_calls,
            jacobian_builds: counters.jacobian_builds,
            jvp_vectors: counters.jvp_vectors,
            ft_calls: counters.ft_calls,
            linear_solves: counters.linear_solves,
            work_units: counters.rhs_calls as f64
                + counters.jvp_vectors as f64
                + counters.ft_calls as f64
                + counters.jacobian_builds as f64 * state.len() as f64
                + counters.linear_solves as f64,
            decision: "",
            evidence: SwitchingEvidence::Uncompared,
            initial_step,
            proposed_next_step: (reached < solve_end
                && (after.stop_reason.is_none()
                    || after.stop_reason == Some(Dopri5StopReason::AttemptBudget)))
            .then_some(checkpoint.proposed_h)
            .filter(|h| h.is_finite() && *h > 0.0),
            comparator_work_ratio: None,
            driver: SwitchingDriverStats::Dopri5(Box::new(driver)),
        },
        state,
        landings: Vec::new(),
        fatal,
        budget_stop,
        ceiling_stop: false,
    })
}

#[allow(clippy::too_many_arguments)]
fn run(
    problem: &Problem,
    start: f64,
    target: f64,
    y: &[f64],
    opts: &Options,
    method: SwitchingMethod,
    probe: bool,
    carry_explicit_step: bool,
    solve_end: f64,
    landings: &[f64],
    ceiling: Option<u64>,
) -> Result<Batch, String> {
    let (
        reached,
        state,
        landed,
        success,
        attempts,
        accepted,
        rejected,
        rhs,
        jac,
        jvp,
        ft,
        solves,
        fatal,
        budget,
        ceiling_stop,
        driver,
        proposed_next_step,
    ) = match method {
        SwitchingMethod::Dopri5 => {
            // Probes always use a scoped native session: it can be advanced one
            // attempt at a time, so their RHS work can be bounded exactly.
            let (next, r, ceiling_stop) = if carry_explicit_step || probe {
                let epochs = SessionEpochs::default();
                // The batch target is an intermediate checkpoint, not the
                // native terminal time. At tf the native driver intentionally
                // skips the controller's next proposal, since none is needed.
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
                // No proposal was newly computed after the final accepted
                // step of the full solve. Do not expose its stale snapshot.
                // Without step carry no proposal is published at all.
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
            // A deliberate pause at this batch's endpoint is a successful
            // batch even though the native full-span solve is unfinished.
            let success = if carry_explicit_step || probe {
                r.final_time == target
                    && matches!(r.stop_reason, None | Some(Dopri5StopReason::AttemptBudget))
                    && r.callback_failure.is_none()
            } else {
                r.success
            };
            (
                r.final_time,
                r.final_state,
                Vec::new(),
                success,
                r.stats.attempts,
                r.stats.accepted_steps,
                r.stats.rejected_steps,
                c.rhs_calls,
                c.jacobian_builds,
                c.jvp_vectors,
                c.ft_calls,
                c.linear_solves,
                r.callback_failure.is_some(),
                r.final_time < target && r.stop_reason == Some(Dopri5StopReason::AttemptBudget),
                ceiling_stop,
                SwitchingDriverStats::Dopri5(Box::new(r.stats)),
                next.filter(|h| h.is_finite() && *h > 0.0),
            )
        }
        SwitchingMethod::Rodas5p => {
            let r = Rodas5p::solve_observed_bounded(
                problem,
                (start, target),
                y,
                opts,
                landings,
                ceiling,
            )
            .map_err(|e| e.to_string())?;
            let c = r.stats.counters;
            let success = r.status == Status::Success;
            let fatal = matches!(
                r.status,
                Status::Failed(crate::FailureReason::CallbackFailed { .. })
            );
            let budget = matches!(
                r.status,
                Status::Failed(crate::FailureReason::MaxStepsReached { .. })
            );
            let ceiling_stop = matches!(
                r.status,
                Status::Failed(crate::FailureReason::WorkCeilingReached { .. })
            );
            let reached = *r.t.last().unwrap_or(&start);
            let crate::Solution {
                t: times,
                y: states,
                stats,
                ..
            } = r;
            // The last row is the batch's accepted state; rows strictly
            // between the start and that state are landed window endpoints.
            let rows = times.len();
            let mut landed = Vec::new();
            let mut state = None;
            for (index, (t, row)) in times.into_iter().zip(states).enumerate() {
                if index + 1 == rows {
                    state = Some(row);
                } else if t > start && t < reached {
                    landed.push((t, row));
                }
            }
            (
                reached,
                state.unwrap_or_else(|| y.to_vec()),
                landed,
                success,
                stats.attempts,
                stats.accepted_steps,
                stats.rejected_steps,
                c.rhs_calls,
                c.jacobian_builds,
                c.jvp_vectors,
                c.ft_calls,
                c.linear_solves,
                fatal,
                budget,
                ceiling_stop,
                SwitchingDriverStats::Rodas5p(Box::new(stats)),
                None,
            )
        }
    };
    let work_units =
        rhs as f64 + jvp as f64 + ft as f64 + jac as f64 * y.len() as f64 + solves as f64;
    Ok(Batch {
        record: SwitchingSegment {
            method,
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
            rhs_calls: rhs,
            jacobian_builds: jac,
            jvp_vectors: jvp,
            ft_calls: ft,
            linear_solves: solves,
            work_units,
            decision: "",
            evidence: SwitchingEvidence::Uncompared,
            initial_step: opts.h0,
            proposed_next_step,
            comparator_work_ratio: None,
            driver,
        },
        state,
        landings: landed,
        fatal,
        budget_stop: budget,
        ceiling_stop,
    })
}

fn commit(
    result: &mut SwitchingSolution,
    record: &mut SwitchingSegment,
    state: Vec<f64>,
    landings: Vec<(f64, Vec<f64>)>,
) {
    record.committed = true;
    for (t, y) in landings {
        if t > result.final_time && t < record.reached_time {
            result.final_time = t;
            result.t.push(t);
            result.y.push(y);
        }
    }
    if record.reached_time > result.final_time {
        result.final_time = record.reached_time;
        result.final_state = state;
        result.t.push(result.final_time);
        result.y.push(result.final_state.clone());
    } else if !result.y.is_empty() && result.t.last() == Some(&result.final_time) {
        // A landing became the final row: keep the committed state in sync.
        result.final_state.clone_from(result.y.last().unwrap());
    }
}

fn charge(stats: &mut SwitchingStats, r: &SwitchingSegment) {
    stats.attempts += r.attempts;
    stats.accepted_steps += r.accepted_steps;
    stats.rejected_steps += r.rejected_steps;
    stats.rhs_calls = stats.rhs_calls.saturating_add(r.rhs_calls);
    stats.jacobian_builds = stats.jacobian_builds.saturating_add(r.jacobian_builds);
    stats.jvp_vectors = stats.jvp_vectors.saturating_add(r.jvp_vectors);
    stats.ft_calls = stats.ft_calls.saturating_add(r.ft_calls);
    stats.linear_solves = stats.linear_solves.saturating_add(r.linear_solves);
}

fn validate(
    p: &Problem,
    (t0, t1): (f64, f64),
    y: &[f64],
    o: &Options,
    c: &SwitchingConfig,
) -> Result<(), SwitchingError> {
    let bad = |s: &str| SwitchingError(s.into());
    if p.dimension == 0 || y.len() != p.dimension || !y.iter().all(|x| x.is_finite()) {
        return Err(bad(
            "finite state with matching positive dimension required",
        ));
    }
    if !(t0.is_finite() && t1.is_finite() && t1 >= t0 && (t1 - t0).is_finite()) {
        return Err(bad("finite representable forward span required"));
    }
    if p.mass.is_some() || !p.events.is_empty() || o.dense_output {
        return Err(bad("identity mass, no events and endpoint output required"));
    }
    if c.window_count == 0
        || c.window_count > 1_000_000
        || c.explicit_attempt_budget == 0
        || c.probe_attempt_budget == 0
        || c.pressure_steps == 0
        || c.implicit_residence == 0
        || c.probe_interval == 0
        || c.max_dense_dimension == 0
        || c.implicit_pilot_limit == 0
        || c.pilot_backoff_factor == 0
        || c.probe_backoff_factor == 0
        || !(c.rejection_fraction > 0.0 && c.rejection_fraction <= 1.0)
        || !(c.return_cost_ratio > 0.0 && c.return_cost_ratio < 1.0)
        || !(c.entry_cost_ratio > 0.0 && c.entry_cost_ratio < 1.0)
        || !(c.rearm_rate_ratio > 1.0 && c.rearm_rate_ratio.is_finite())
    {
        return Err(bad(
            "positive bounded window/work controls and hysteresis fractions required",
        ));
    }
    let atol = match &o.atol {
        Atol::Scalar(a) => *a,
        Atol::Vector(v) if v.len() == y.len() && v.iter().all(|a| a.is_finite() && *a >= 0.0) => {
            v.iter().copied().fold(0.0, f64::max)
        }
        _ => return Err(bad("invalid absolute tolerance vector")),
    };
    AdaptiveStepConfig {
        atol,
        rtol: o.rtol,
        initial_step: o.h0,
        max_attempts: o.max_steps,
        ..Default::default()
    }
    .validate()
    .map_err(|e| SwitchingError(e.to_string()))?;
    if c.fallback != DerivativeFallback::Forbidden {
        if c.fallback == DerivativeFallback::ProvidedOnly
            && matches!(p.jacobian, Jacobian::FiniteDifference)
        {
            return Err(bad(
                "ProvidedOnly fallback requires an explicit Jacobian/JVP provider",
            ));
        }
        if c.fallback == DerivativeFallback::ProvidedOnly
            && !p.autonomous
            && p.time_derivative.is_none()
        {
            return Err(bad(
                "ProvidedOnly fallback requires autonomy or an explicit time-derivative provider",
            ));
        }
        let caps = Rodas5p::capabilities(p, o).map_err(|e| SwitchingError(e.to_string()))?;
        // A declared band on a dense provider still allocates the user's dense
        // matrix at the route-verification boundary, so it also needs this cap.
        if (matches!(caps.route, crate::FacadeRoute::DenseFast)
            || matches!(p.jacobian, Jacobian::Dense(_)))
            && y.len() > c.max_dense_dimension
        {
            return Err(bad("possible dense fallback exceeds max_dense_dimension"));
        }
        if p.autonomous && p.time_derivative.is_some() {
            return Err(bad("autonomous derivative fallback cannot also have df/dt"));
        }
        if let Some((l, u)) = p.band
            && (l >= y.len() || u >= y.len())
        {
            return Err(bad("band widths exceed dimension"));
        }
        if let Jacobian::Banded { lower, upper, .. } = p.jacobian
            && (lower >= y.len() || upper >= y.len() || p.band.is_some_and(|b| b != (lower, upper)))
        {
            return Err(bad("band provider/declaration mismatch"));
        }
    }
    Ok(())
}
