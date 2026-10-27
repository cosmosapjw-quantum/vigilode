//! Bounded, opt-in explicit/RODAS switching at accepted window boundaries.
//!
//! This adapter uses observed work, not a stiffness estimate. Its default is
//! RHS-only. Derivative fallbacks require a separate explicit authorization.

use crate::selection_evidence::{
    CostComparison, CostEvidence, CostEvidenceKind, CostInterval, CostUnit, SelectionIdentity,
    compare_qualified_costs,
};
use rodas5p_integrators::{AdaptiveStepConfig, Dopri5StopReason};
use serde::{Deserialize, Serialize};

use crate::{
    Atol, Dopri5, ExplicitStats, Jacobian, Options, Problem, Rodas5p, SessionEpochs, Stats, Status,
};

/// Authorization for derivative work after the explicit path shows pressure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// Reuse a losing entry comparison only while its observed regime remains
/// similar. These controls schedule probes; they do not certify stiffness.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryProbeAdaptation {
    /// Age in completed explicit batches after which a sentinel is due at the
    /// next eligible work-pressure batch before the final time.
    pub max_backoff_batches: usize,
    /// Renew evidence when work per advanced physical time changes by this
    /// factor and the carried step changes oppositely by `step_shrink_ratio`.
    /// Both deterioration and relaxation can invalidate a losing comparison.
    pub work_rate_growth: f64,
    /// Joint step-proposal threshold used with work-rate growth.
    pub step_shrink_ratio: f64,
    /// A rise in rejection fraction by this amount also renews evidence, after
    /// at least two rejected attempts. It never directly selects a method.
    pub rejection_fraction_increase: f64,
}

impl Default for EntryProbeAdaptation {
    fn default() -> Self {
        Self {
            max_backoff_batches: 64,
            work_rate_growth: 2.0,
            step_shrink_ratio: 0.5,
            rejection_fraction_increase: 0.2,
        }
    }
}

/// Why an optional adaptive entry comparison was executed or deferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryProbeReason {
    /// No prior losing comparison is available in this explicit tenure.
    InitialComparison,
    /// Recent negative evidence is retained until deterioration or expiry.
    StableEvidenceBackoff,
    /// The increasing retry gap has elapsed.
    BackoffExpired,
    /// The maximum evidence age has elapsed.
    SentinelExpired,
    /// Physical work rate rose while the native proposal fell.
    WorkDeteriorated,
    /// Physical work rate fell while the native proposal rose, indicating
    /// that the just-completed prefix crossed a regime boundary. Comparison
    /// is deferred so its next prefix starts at the newly accepted state.
    WorkRegimeRelaxed,
    /// A preceding batch requested comparison in the newly reached regime.
    /// This request persists until an eligible actual pilot is executed.
    PendingRegimeComparison,
    /// A zero-accepted, nonfatal entry pilot did not establish an economical
    /// step. Retry once from the subsequently accepted explicit checkpoint.
    /// This request survives ineligible batches, but may occur only once in
    /// an explicit tenure.
    UnresolvedEntryRetry,
    /// Rejections rose relative to the last compared batch.
    RejectionDeteriorated,
}

/// Controls of the opt-in window adapter; units of work are deterministic.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Stop a same-window implicit return comparator once its charged callback
    /// work alone proves that the already completed explicit candidate is
    /// cheaper by `return_cost_ratio`. The partial implicit state is discarded;
    /// this proves a work lower bound, not implicit endpoint admissibility.
    /// Opt in separately; false retains full-comparator execution.
    pub stop_dominated_return_comparator: bool,
    /// Select a completed explicit return only when its checked integer work
    /// is strictly below actual same-window implicit work. A nonfatal partial
    /// implicit replay supplies a monotone cost lower bound, not an admissible
    /// endpoint. Ties retain the implicit route. The return ratio is ignored;
    /// an ordinary probe uses the previous rate only as a scheduling screen,
    /// then runs a same-window comparator even without adaptive entry memory.
    /// A sentinel uses the completed implicit work to bound the explicit
    /// challenger exactly. All comparisons include discarded and failed work.
    /// Default false; scheduling and derivative permission stay unchanged.
    pub strict_return_work_improvement: bool,
    /// Refuse a possible dense fallback above this dimension, before callbacks.
    pub max_dense_dimension: usize,
    /// Attempts permitted for an implicit pilot from the same checkpoint and
    /// over the same accepted time interval as an explicit batch. Zero retains
    /// the original pressure-only entry policy. Nonzero enables cost screening.
    pub implicit_pilot_attempt_budget: usize,
    /// Maximum implicit entry pilots during a legacy solve. Adaptive entry
    /// scheduling instead uses bounded retry spacing and finite sentinels;
    /// every attempt remains charged to the common `Options::max_steps` cap.
    pub implicit_pilot_limit: usize,
    /// A successful entry pilot must cost at most this fraction of the explicit
    /// batch's deterministic work units. This is not a wall-clock guarantee.
    /// A pilot is also stopped as soon as its charged callback work alone
    /// exceeds this bound, since it could no longer pass.
    /// Ignored for entry when `strict_entry_work_improvement` is enabled.
    pub entry_cost_ratio: f64,
    /// Prefer an entry pilot exactly when its completed, qualified replay
    /// costs strictly less integer work than the accepted explicit prefix.
    /// Work is the existing proxy `rhs + jvp + ft + n * jacobians + solves`,
    /// calculated with checked integer arithmetic; ties keep the explicit
    /// method. This overrides the entry ratio and its callback ceiling only.
    /// It does not promise future work savings or a wall-clock speedup.
    /// Applies only when `implicit_pilot_attempt_budget > 0`; zero retains
    /// pressure-only direct entry.
    /// Default false, including factories; derivative permission, pressure,
    /// retry scheduling, error tests and return policy stay unchanged.
    pub strict_entry_work_improvement: bool,
    /// Keep a native DOPRI session, including controller, FSAL state and
    /// workspace, across contiguous explicit windows. A committed RODAS step
    /// ends that tenure; an accepted explicit recheck may seed the new tenure's
    /// initial proposal. False retains the legacy per-batch explicit driver.
    pub carry_explicit_step: bool,
    /// Explicit batches between implicit entry pilots. Zero preserves legacy
    /// scheduling. This permits later re-evaluation without a pilot every batch.
    pub entry_pilot_cooldown_batches: usize,
    /// Seed a newly entered implicit run with the mean accepted step of its
    /// completed economical entry pilot only when larger than `Options::h0`.
    /// This reuses only a step proposal;
    /// the accepted explicit state and all native error checks are preserved.
    /// The default is false; the adaptive factory enables this opt-in trial.
    pub reuse_entry_pilot_step: bool,
    /// Start a same-checkpoint implicit entry pilot from the larger of
    /// `Options::h0` and the explicit checkpoint's already observed proposal.
    /// Clip the proposal to the accepted replay interval, then retain the
    /// existing native minimum-step floor; the native driver endpoint-clips
    /// its actual trial even on shorter tails. All tolerances, work ceilings
    /// and acceptance tests remain unchanged. This does not use the explicit
    /// batch's future endpoint proposal. Default false, including factories.
    pub seed_entry_pilot_from_explicit_checkpoint: bool,
    /// Reuse the mean accepted step of any prior nonfatal entry pilot as a
    /// proposal for a later pilot, including an incomplete or uneconomical
    /// pilot. Only a finite positive accepted prefix contributes; its state
    /// and cost qualification are never reused. The next proposal is the
    /// maximum of this observed mean and the otherwise selected initial step,
    /// clipped by the native endpoint. Every new trial retains native error
    /// tests and strict cost evidence. Default false; no derivative work is
    /// authorized by this flag and the memory ends with the explicit tenure.
    pub reuse_observed_entry_step: bool,
    /// With adaptive entry memory, retry one nonfatal zero-accepted entry
    /// pilot from a later accepted explicit checkpoint. The retry bypasses
    /// backoff once per explicit tenure, while remaining subject to pressure,
    /// admissibility, callback ceilings, and the global attempt budget.
    pub retry_unresolved_entry_once: bool,
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
    /// Every `k`-th due explicit recheck of an implicit tenure (the first
    /// included) is a same-window sentinel; zero disables it. A sentinel runs
    /// the implicit window first as the productive advance and bounds the
    /// explicit return by `return_cost_ratio` times that current implicit
    /// work (plus one callback unit) instead of the previous window's rate and
    /// `probe_attempt_budget`. Its explicit trial is discarded unless it wins
    /// the same-window work comparison. A losing sentinel costs at most
    /// `(1 + return_cost_ratio) W + 1` units plus one DOPRI attempt, where `W`
    /// is the implicit window's work (Jacobian builds weigh `n`), and its
    /// attempts draw on the global `max_steps` without a separate cap; the
    /// committed implicit window keeps `is_probe = true` as the root of a
    /// comparison. Callbacks must be replay safe.
    pub return_sentinel_interval: usize,
    /// Optional adaptive negative-evidence reuse. None preserves the legacy
    /// rate-rearmed backoff/lifetime-count policy. When present, adaptive
    /// memory replaces entry `pilot_backoff_factor` and `rearm_rate_ratio`
    /// scheduling, so two independent entry backoffs are never stacked.
    /// Callbacks must be replay
    /// safe as for every competitive switching solve.
    pub entry_probe_adaptation: Option<EntryProbeAdaptation>,
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
            stop_dominated_return_comparator: false,
            strict_return_work_improvement: false,
            max_dense_dimension: 64,
            implicit_pilot_attempt_budget: 0,
            implicit_pilot_limit: 1,
            entry_cost_ratio: 0.8,
            strict_entry_work_improvement: false,
            carry_explicit_step: false,
            entry_pilot_cooldown_batches: 0,
            reuse_entry_pilot_step: false,
            seed_entry_pilot_from_explicit_checkpoint: false,
            reuse_observed_entry_step: false,
            retry_unresolved_entry_once: false,
            pilot_backoff_factor: 2,
            probe_backoff_factor: 2,
            rearm_rate_ratio: 2.0,
            return_sentinel_interval: 0,
            entry_probe_adaptation: None,
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

    /// Opt into adaptive probe backoff with deterioration-triggered renewal
    /// and finite evidence-age sentinels at eligible work-pressure batches.
    /// No derivative permission is added by this factory.
    /// Decisions still require an actual same-checkpoint competitive pilot.
    pub fn adaptive_evidence(fallback: DerivativeFallback) -> Self {
        Self {
            entry_probe_adaptation: Some(EntryProbeAdaptation::default()),
            reuse_entry_pilot_step: true,
            retry_unresolved_entry_once: true,
            ..Self::evidence_driven(fallback)
        }
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
    /// Successful explicit return trial beat an actual implicit trial from
    /// the same physical checkpoint to the identical window endpoint.
    ExplicitCheaperOnSameWindow,
    /// Completed explicit candidate beats a conservative lower bound on the
    /// same-window implicit comparator's cost. Its partial state is discarded.
    ExplicitCheaperThanComparatorLowerBound,
    /// A same-window implicit comparison was cheaper and its accepted state
    /// was committed directly, avoiding a duplicate implicit solve.
    ImplicitPreferredOnSameWindow,
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
    /// Matched-comparison work ratio: implicit/explicit on entry and
    /// explicit/implicit on return. An incomplete implicit comparator supplies
    /// a cost lower bound, so its return ratio bounds the completed ratio from
    /// above. An incomplete challenger (entry or sentinel explicit probe)
    /// has no completed-work ratio and leaves this field absent. Includes
    /// all charged attempts, not just
    /// accepted steps.
    pub comparator_work_ratio: Option<f64>,
    /// Scheduling evidence for the optional adaptive entry policy. This is
    /// separate from the actual competitive result in `evidence`.
    pub entry_probe_reason: Option<EntryProbeReason>,
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
    /// previous implicit rate over its window. The entry ceiling cannot remove
    /// a candidate satisfying that comparison. The historical return ceiling
    /// can postpone a useful explicit return if the current regime changed;
    /// it does not establish current-window optimality. Adaptive returns still
    /// require an actual same-window comparison after this preliminary screen.
    /// Legacy entry scheduling and all return rechecks back off after discarded
    /// trials, with work-rate rearming. Adaptive entry scheduling instead uses
    /// its own negative-evidence memory and finite sentinels. An implicit tenure
    /// integrates every window up to
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
            entry_memory: EntryProbeMemory::default(),
            explicit_step: None,
            next_window: 1,
            pilot_discards: 0,
            explicit_rate_at_discard: None,
            probe_discards: 0,
            implicit_rate_at_discard: None,
            implicit_seed: None,
            tenure_rechecks: 0,
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
                }
                .max(AdaptiveStepConfig::default().min_step);
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
    entry_memory: EntryProbeMemory,
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
    /// Due explicit rechecks in the current implicit tenure.
    tenure_rechecks: usize,
}

#[derive(Default)]
struct EntryProbeMemory {
    negative_comparisons: usize,
    anchor_rate: Option<f64>,
    anchor_h: Option<f64>,
    anchor_rejection: f64,
    pending_renewal: bool,
    pending_zero_progress_retry: bool,
    zero_progress_retry_used: bool,
    /// Step proposal only; never an accepted state or future-cost assertion.
    observed_entry_h: Option<f64>,
}

impl EntryProbeMemory {
    fn reason(
        &self,
        r: &SwitchingSegment,
        elapsed_batches: usize,
        c: &SwitchingConfig,
        a: &EntryProbeAdaptation,
    ) -> EntryProbeReason {
        if self.pending_zero_progress_retry {
            return EntryProbeReason::UnresolvedEntryRetry;
        }
        if self.pending_renewal {
            return EntryProbeReason::PendingRegimeComparison;
        }
        let Some(anchor_rate) = self.anchor_rate else {
            return EntryProbeReason::InitialComparison;
        };
        // Very short observation-clipped tails are poor rate comparators.
        // A meaningful accepted sample and a carried controller proposal are
        // required for this early invalidation; expiry remains unconditional.
        let dt = r.reached_time - r.start_time;
        if r.accepted_steps >= 4 && dt > 0.0 {
            let rate = r.work_units / dt;
            if rate.is_finite()
                && rate >= a.work_rate_growth * anchor_rate
                && self
                    .anchor_h
                    .zip(r.proposed_next_step)
                    .is_some_and(|(old, new)| new <= a.step_shrink_ratio * old)
            {
                return EntryProbeReason::WorkDeteriorated;
            }
            if rate.is_finite()
                && rate <= anchor_rate / a.work_rate_growth
                && self
                    .anchor_h
                    .zip(r.proposed_next_step)
                    .is_some_and(|(old, new)| new >= old / a.step_shrink_ratio)
            {
                return EntryProbeReason::WorkRegimeRelaxed;
            }
        }
        let rejected_fraction = r.rejected_steps as f64 / r.attempts.max(1) as f64;
        if r.rejected_steps >= 2
            && rejected_fraction >= self.anchor_rejection + a.rejection_fraction_increase
        {
            return EntryProbeReason::RejectionDeteriorated;
        }
        if elapsed_batches >= a.max_backoff_batches {
            return EntryProbeReason::SentinelExpired;
        }
        let multiplier = 1usize
            .checked_shl(self.negative_comparisons.min(usize::BITS as usize - 1) as u32)
            .unwrap_or(usize::MAX);
        let cooldown = c
            .entry_pilot_cooldown_batches
            .max(1)
            .saturating_mul(multiplier)
            .min(a.max_backoff_batches);
        if elapsed_batches >= cooldown {
            EntryProbeReason::BackoffExpired
        } else {
            EntryProbeReason::StableEvidenceBackoff
        }
    }

    fn schedule(
        &mut self,
        r: &SwitchingSegment,
        elapsed_batches: usize,
        c: &SwitchingConfig,
        a: &EntryProbeAdaptation,
    ) -> (EntryProbeReason, bool) {
        let reason = self.reason(r, elapsed_batches, c, a);
        match reason {
            EntryProbeReason::WorkRegimeRelaxed => {
                // The endpoint proposal describes the new regime; replaying
                // this just-ended prefix would still include the transient.
                // Keep the request until a later eligible prefix is compared.
                self.negative_comparisons = 0;
                self.pending_renewal = true;
                (reason, false)
            }
            EntryProbeReason::WorkDeteriorated | EntryProbeReason::RejectionDeteriorated => {
                self.negative_comparisons = 0;
                (reason, true)
            }
            EntryProbeReason::StableEvidenceBackoff => (reason, false),
            _ => (reason, true),
        }
    }

    fn compared(&mut self, r: &SwitchingSegment, entered: bool) {
        self.pending_renewal = false;
        self.pending_zero_progress_retry = false;
        if entered {
            *self = Self::default();
            return;
        }
        self.negative_comparisons = self.negative_comparisons.saturating_add(1);
        let dt = r.reached_time - r.start_time;
        self.anchor_rate = (dt > 0.0)
            .then_some(r.work_units / dt)
            .filter(|rate| rate.is_finite() && *rate > 0.0);
        self.anchor_h = r.proposed_next_step;
        self.anchor_rejection = r.rejected_steps as f64 / r.attempts.max(1) as f64;
    }

    fn retry_zero_progress_once(&mut self) {
        if !self.zero_progress_retry_used {
            self.zero_progress_retry_used = true;
            self.pending_zero_progress_retry = true;
        }
    }
}

fn entry_pilot_growth_seed(pilot: &SwitchingSegment, initial_step: f64) -> Option<f64> {
    if !pilot.success || pilot.accepted_steps == 0 {
        return None;
    }
    let mean = (pilot.reached_time - pilot.start_time) / pilot.accepted_steps as f64;
    (mean.is_finite() && mean > initial_step).then_some(mean)
}

fn observed_entry_step(pilot: &SwitchingSegment) -> Option<f64> {
    if pilot.accepted_steps == 0 {
        return None;
    }
    let step = (pilot.reached_time - pilot.start_time) / pilot.accepted_steps as f64;
    (step.is_finite() && step > 0.0).then_some(step)
}

fn entry_pilot_checkpoint_seed(
    explicit: &SwitchingSegment,
    initial_step: f64,
    enabled: bool,
) -> f64 {
    if enabled && explicit.initial_step.is_finite() && explicit.initial_step > 0.0 {
        initial_step.max(explicit.initial_step)
    } else {
        initial_step
    }
}

fn window_target((t0, t1): (f64, f64), window: usize, count: usize) -> f64 {
    if window == count {
        t1
    } else {
        t0 + (t1 - t0) * (window as f64 / count as f64)
    }
}

// A work rate may overflow on a very short but representable interval.
// Saturating float conversion already treats an unbounded cap as u64::MAX;
// the one-callback rounding allowance must retain that saturation as well.
fn return_work_ceiling(rate: f64, span: f64, ratio: f64) -> u64 {
    ((ratio * rate * span).floor() as u64).saturating_add(1)
}

// The existing work objective, before any floating conversion. Checked
// arithmetic prevents overflow from becoming evidence for a cheaper method.
fn integer_work(record: &SwitchingSegment, dimension: usize, callbacks_only: bool) -> Option<u128> {
    let mut cost = (record.jacobian_builds as u128).checked_mul(dimension as u128)?;
    for count in [
        record.rhs_calls,
        record.jvp_vectors,
        record.ft_calls,
        if callbacks_only {
            0
        } else {
            record.linear_solves
        },
    ] {
        cost = cost.checked_add(count as u128)?;
    }
    Some(cost)
}

// One outward adjacent float contains conversion rounding even above
// binary64's exact-integer range. This deliberately expands exactly
// represented positive counts too, so equality (or a sub-ulp margin) may
// be retained at ordinary count magnitudes on the legacy ratio path.
fn work_enclosure(
    record: &SwitchingSegment,
    dimension: usize,
    callbacks_only: bool,
) -> Option<(f64, f64)> {
    let cost = integer_work(record, dimension, callbacks_only)?;
    if cost == 0 {
        return Some((0.0, 0.0));
    }
    let rounded = cost as f64;
    Some((rounded.next_down(), rounded.next_up()))
}

// For integer callback prefix cost L and incumbent total E, L >= E rules
// out C_candidate < E. The guard charges the crossing callback and stops
// on L > cap, hence cap = E - 1 (zero also admits no strict winner).
fn strict_entry_callback_ceiling(incumbent: &SwitchingSegment, dimension: usize) -> Option<u64> {
    let ceiling = integer_work(incumbent, dimension, false)?.saturating_sub(1);
    // u64::MAX is the native unarmed sentinel; decline an unrepresentable
    // ceiling rather than truncate it. The global attempt cap still applies.
    (ceiling < u64::MAX as u128).then_some(ceiling as u64)
}

fn strict_entry_work_preferred(
    incumbent: &SwitchingSegment,
    candidate: &SwitchingSegment,
    dimension: usize,
) -> bool {
    // The caller replays from the same physical checkpoint. An accepted
    // explicit prefix can be valid even if its larger batch hit a budget.
    if incumbent.accepted_steps == 0
        || incumbent.reached_time <= incumbent.start_time
        || !candidate.success
        || candidate.start_time != incumbent.start_time
        || candidate.target_time != incumbent.reached_time
        || candidate.reached_time != incumbent.reached_time
    {
        return false;
    }
    match (
        integer_work(incumbent, dimension, false),
        integer_work(candidate, dimension, false),
    ) {
        (Some(incumbent_cost), Some(candidate_cost)) => candidate_cost < incumbent_cost,
        _ => false,
    }
}

// A completed explicit candidate E beats the same-window implicit completion
// whenever E < I. For an incomplete deterministic replay, its already charged
// work L is a lower bound on I, so E < L suffices without using its endpoint.
fn strict_return_work_preferred(
    incumbent: &SwitchingSegment,
    candidate: &SwitchingSegment,
    dimension: usize,
) -> bool {
    if !candidate.success
        || candidate.accepted_steps == 0
        || !candidate.start_time.is_finite()
        || !candidate.target_time.is_finite()
        || candidate.target_time <= candidate.start_time
        || candidate.reached_time != candidate.target_time
        || incumbent.start_time != candidate.start_time
        || incumbent.target_time != candidate.target_time
        || !incumbent.reached_time.is_finite()
        || incumbent.reached_time < incumbent.start_time
        || incumbent.reached_time > incumbent.target_time
        || (incumbent.success && incumbent.reached_time != incumbent.target_time)
    {
        return false;
    }
    match (
        integer_work(incumbent, dimension, false),
        integer_work(candidate, dimension, false),
    ) {
        (Some(incumbent_cost), Some(candidate_cost)) => candidate_cost < incumbent_cost,
        _ => false,
    }
}

// The callback guard stops after charging L > cap. Setting cap = E proves
// E < L <= I at the crossing callback; the strict inequality fixes the exact
// integer boundary without a ratio, rounding slack, or fitted safety factor.
fn strict_return_comparator_ceiling(candidate: &SwitchingSegment, dimension: usize) -> Option<u64> {
    let cost = integer_work(candidate, dimension, false)?;
    (cost < u64::MAX as u128).then_some(cost as u64)
}

fn dominated_comparator_ceiling(
    candidate: &SwitchingSegment,
    dimension: usize,
    ratio: f64,
) -> Option<u64> {
    let (_, upper) = work_enclosure(candidate, dimension, false)?;
    let threshold = (upper / ratio).next_up().ceil();
    // The native guard reserves u64::MAX as the unarmed sentinel. Declining
    // an unrepresentable cap falls back to the existing bounded attempt path.
    (threshold.is_finite() && threshold < (u64::MAX - 1) as f64).then_some(threshold as u64)
}

// A displayed incomplete-comparator ratio is an upper bound too, so its
// arithmetic must have the same outward semantics as the admission test.
fn prefix_ratio_upper(
    candidate: &SwitchingSegment,
    comparator: &SwitchingSegment,
    dimension: usize,
) -> Option<f64> {
    let (_, upper) = work_enclosure(candidate, dimension, false)?;
    let (lower, _) = work_enclosure(comparator, dimension, false)?;
    if lower <= 0.0 {
        return None;
    }
    let ratio = (upper / lower).next_up();
    ratio.is_finite().then_some(ratio)
}

fn prefix_cost_dominates(
    candidate: &SwitchingSegment,
    comparator: &SwitchingSegment,
    dimension: usize,
    ratio: f64,
) -> bool {
    let Some((_, upper)) = work_enclosure(candidate, dimension, false) else {
        return false;
    };
    let Some((lower, _)) = work_enclosure(comparator, dimension, false) else {
        return false;
    };
    candidate.success && upper <= (ratio * lower).next_down()
}

fn qualified_work_preferred(
    incumbent: &SwitchingSegment,
    candidate: &SwitchingSegment,
    incumbent_admissible: bool,
    ratio: f64,
    window_epoch: u64,
) -> bool {
    let identity = SelectionIdentity {
        window_epoch,
        ..Default::default()
    };
    let evidence = |work, admissible| CostEvidence {
        identity,
        cost: CostInterval {
            lower: work,
            upper: work,
        },
        unit: CostUnit::Work,
        kind: CostEvidenceKind::ObservedWork,
        admissible,
    };
    compare_qualified_costs(
        &evidence(incumbent.work_units, incumbent_admissible),
        &evidence(candidate.work_units, candidate.success),
        0.0,
        1,
        1.0 - ratio,
    ) == CostComparison::PreferCandidate
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
        mut entry_memory,
        mut explicit_step,
        next_window,
        mut pilot_discards,
        mut explicit_rate_at_discard,
        mut probe_discards,
        mut implicit_rate_at_discard,
        mut implicit_seed,
        mut tenure_rechecks,
    } = progress;
    let fallback = config.fallback != DerivativeFallback::Forbidden;
    let return_ratio = if config.strict_return_work_improvement {
        1.0
    } else {
        config.return_cost_ratio
    };
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
                    entry_memory,
                    explicit_step,
                    next_window: window,
                    pilot_discards,
                    explicit_rate_at_discard,
                    probe_discards,
                    implicit_rate_at_discard,
                    implicit_seed,
                    tenure_rechecks,
                };
            }
            let remaining = opts.max_steps - result.stats.attempts;
            if remaining == 0 {
                result.message = "total attempt budget exhausted".into();
                break 'windows;
            }
            let start = result.final_time;
            if probe
                && config.return_sentinel_interval > 0
                && tenure_rechecks % config.return_sentinel_interval == 0
            {
                // Same-window sentinel. The implicit window runs first and is
                // the productive advance unless the explicit return beats it;
                // the return is bounded by that current work, never by the
                // previous window's rate or the fixed probe attempt budget.
                tenure_rechecks += 1;
                since_probe = 0;
                probe = false;
                let mut implicit_opts = opts.clone();
                implicit_opts.h0 = implicit_seed
                    .unwrap_or(opts.h0)
                    .min(target - start)
                    .max(AdaptiveStepConfig::default().min_step);
                implicit_opts.max_steps = remaining;
                let mut implicit = match run(
                    problem,
                    start,
                    target,
                    &result.final_state,
                    &implicit_opts,
                    SwitchingMethod::Rodas5p,
                    true,
                    false,
                    t1,
                    &[],
                    None,
                ) {
                    Ok(batch) => batch,
                    Err(message) => {
                        result.stats.accounting_complete = false;
                        result.message = message;
                        break 'windows;
                    }
                };
                charge(&mut result.stats, &implicit.record);
                if implicit.fatal || !implicit.record.success {
                    // Exactly an ordinary implicit window's stop: keep the
                    // accepted prefix, no explicit trial after a failure.
                    implicit.record.decision = if implicit.fatal {
                        "fatal sentinel implicit window; stop without fallback"
                    } else {
                        "sentinel implicit window stopped; preserve partial accepted state"
                    };
                    commit(
                        &mut result,
                        &mut implicit.record,
                        implicit.state,
                        implicit.landings,
                    );
                    result.message = implicit.record.decision.into();
                    result.stats.segments.push(implicit.record);
                    break 'windows;
                }
                let remaining = opts.max_steps - result.stats.attempts;
                let mut explicit = if remaining > 0 {
                    local.h0 = explicit_step
                        .unwrap_or(opts.h0)
                        .min(target - start)
                        .max(AdaptiveStepConfig::default().min_step);
                    local.max_steps = remaining;
                    match run(
                        problem,
                        start,
                        target,
                        &result.final_state,
                        &local,
                        SwitchingMethod::Dopri5,
                        true,
                        config.carry_explicit_step,
                        t1,
                        &[],
                        if config.strict_return_work_improvement {
                            strict_entry_callback_ceiling(&implicit.record, problem.dimension)
                        } else {
                            Some(return_work_ceiling(
                                implicit.record.work_units,
                                1.0,
                                return_ratio,
                            ))
                        },
                    ) {
                        Ok(batch) => Some(batch),
                        Err(message) => {
                            // The completed implicit window is accepted state;
                            // the trial left no work record.
                            result.stats.accounting_complete = false;
                            implicit.record.decision =
                                "sentinel explicit return failed without work record; stop";
                            implicit.record.evidence = SwitchingEvidence::ImplicitWindowCompleted;
                            commit(
                                &mut result,
                                &mut implicit.record,
                                implicit.state,
                                implicit.landings,
                            );
                            result.message = message;
                            result.stats.segments.push(implicit.record);
                            break 'windows;
                        }
                    }
                } else {
                    None
                };
                if let Some(explicit) = explicit.as_mut() {
                    charge(&mut result.stats, &explicit.record);
                    explicit.record.comparator_work_ratio = (explicit.record.success
                        && implicit.record.work_units > 0.0)
                        .then_some(explicit.record.work_units / implicit.record.work_units);
                    if explicit.fatal {
                        // The implicit window completed before the trial and is
                        // the productive advance; the fatal trial is never
                        // published, and no further driver runs.
                        explicit.record.decision = "fatal sentinel explicit return; keep completed implicit window and stop";
                        implicit.record.decision = explicit.record.decision;
                        implicit.record.evidence = SwitchingEvidence::ImplicitWindowCompleted;
                        result.message = explicit.record.decision.into();
                        commit(
                            &mut result,
                            &mut implicit.record,
                            std::mem::take(&mut implicit.state),
                            std::mem::take(&mut implicit.landings),
                        );
                        result.stats.segments.push(implicit.record);
                        result.stats.segments.push(explicit.record.clone());
                        break 'windows;
                    }
                }
                let explicit_wins = explicit.as_ref().is_some_and(|e| {
                    e.record.success
                        && if config.strict_return_work_improvement {
                            strict_return_work_preferred(
                                &implicit.record,
                                &e.record,
                                problem.dimension,
                            )
                        } else {
                            qualified_work_preferred(
                                &implicit.record,
                                &e.record,
                                true,
                                return_ratio,
                                result.stats.segments.len() as u64,
                            )
                        }
                });
                if explicit_wins {
                    let mut explicit = explicit.expect("a winning explicit return exists");
                    implicit.record.decision =
                        "sentinel implicit comparator discarded after qualified explicit win";
                    result.stats.segments.push(implicit.record);
                    explicit.record.decision =
                        "explicit return passed sentinel same-window comparison";
                    explicit.record.evidence = SwitchingEvidence::ExplicitCheaperOnSameWindow;
                    explicit_step = explicit.record.proposed_next_step;
                    commit(
                        &mut result,
                        &mut explicit.record,
                        explicit.state,
                        explicit.landings,
                    );
                    result.stats.segments.push(explicit.record);
                    method = SwitchingMethod::Dopri5;
                    entry_memory = EntryProbeMemory::default();
                    batches_since_entry_pilot = usize::MAX;
                    result.stats.switches += 1;
                    implicit_windows = 0;
                    pilot_discards = 0;
                    explicit_rate_at_discard = None;
                    probe_discards = 0;
                    implicit_rate_at_discard = None;
                    implicit_seed = None;
                    tenure_rechecks = 0;
                    continue;
                }
                let rate_before = implicit_rate;
                if explicit.is_some() {
                    implicit.record.evidence = SwitchingEvidence::ImplicitPreferredOnSameWindow;
                    implicit.record.decision =
                        "sentinel implicit comparator retained as productive implicit advance";
                } else {
                    implicit.record.evidence = SwitchingEvidence::ImplicitWindowCompleted;
                    implicit.record.decision =
                        "sentinel implicit window completed; budget left no explicit trial";
                }
                implicit_rate = Some(implicit.record.work_units / (target - start));
                implicit_windows += 1;
                since_probe += 1;
                implicit_seed = (implicit.record.accepted_steps > 0).then(|| {
                    (implicit.record.reached_time - start) / implicit.record.accepted_steps as f64
                });
                explicit_step = None;
                commit(
                    &mut result,
                    &mut implicit.record,
                    implicit.state,
                    implicit.landings,
                );
                result.stats.segments.push(implicit.record);
                if let Some(mut explicit) = explicit {
                    explicit.record.decision = if explicit.ceiling_stop {
                        "sentinel explicit return discarded: exceeded current same-window implicit work"
                    } else {
                        "sentinel explicit return discarded; retain implicit route"
                    };
                    explicit.record.evidence = SwitchingEvidence::ExplicitReturnNotSupported;
                    probe_discards += 1;
                    // As the ordinary path: the anchor is the rate before this recheck.
                    implicit_rate_at_discard = rate_before;
                    result.stats.segments.push(explicit.record);
                }
                continue;
            }
            // A replay pilot is rooted at the identical accepted physical
            // checkpoint. It may never replace the already tested explicit
            // candidate, and every discarded trial is charged. Eligibility is
            // judged before the batch; a work-rate jump may re-arm it after.
            let pilot_eligible = config.implicit_pilot_attempt_budget > 0
                && method == SwitchingMethod::Dopri5
                && (config.entry_probe_adaptation.is_some()
                    || (implicit_pilots < config.implicit_pilot_limit
                        && batches_since_entry_pilot >= config.pilot_cooldown(pilot_discards)));
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
                SwitchingMethod::Dopri5 if config.carry_explicit_step => explicit_step
                    .unwrap_or(opts.h0)
                    .min(batch_target - start)
                    .max(AdaptiveStepConfig::default().min_step),
                SwitchingMethod::Dopri5 => opts
                    .h0
                    .min(batch_target - start)
                    .max(AdaptiveStepConfig::default().min_step),
                SwitchingMethod::Rodas5p => implicit_seed
                    .unwrap_or(opts.h0)
                    .min(batch_target - start)
                    .max(AdaptiveStepConfig::default().min_step),
            };
            // Each driver clips a valid initial proposal to its endpoint. Do
            // not turn a representable short tail into an invalid (< min_h)
            // input by clipping the configuration before native validation.
            local.max_steps = remaining.min(match selected {
                SwitchingMethod::Dopri5 if probe => config.probe_attempt_budget,
                SwitchingMethod::Dopri5 if fallback => config.explicit_attempt_budget,
                // With no fallback/probes, a single driver can use the
                // entire remaining budget instead of needless batch restarts.
                _ => remaining,
            });
            // A recheck can only return when its work rate stays within the
            // hysteresis of the implicit rate: RHS work past that bound ends it.
            // One unit of slack covers the rounding difference between this
            // product and the rate comparison that decides the return.
            let return_ceiling = if probe {
                implicit_rate.map(|rate| return_work_ceiling(rate, target - start, return_ratio))
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
                    // Interior window endpoints of a coalesced batch, strictly
                    // increasing: endpoints below the clock resolution collapse
                    // onto their neighbour instead of refusing the native run.
                    let mut landings: Vec<f64> = Vec::new();
                    for w in window..window + windows.saturating_sub(1) {
                        let t = window_target((t0, t1), w, config.window_count);
                        if t > start && landings.last().is_none_or(|last| t > *last) {
                            landings.push(t);
                        }
                    }
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
                tenure_rechecks += 1;
                let rate = record.work_units / (target - start);
                let mut economical =
                    record.success && implicit_rate.is_some_and(|old| rate <= return_ratio * old);
                let mut return_comparator = None;
                if economical
                    && (config.entry_probe_adaptation.is_some()
                        || config.strict_return_work_improvement)
                {
                    // A previous-window rate is only a cheap trigger. The new
                    // route is selected using a replay of the same physical
                    // window, with every comparator attempt charged.
                    let remaining = opts.max_steps - result.stats.attempts;
                    economical = false;
                    if remaining > 0 {
                        let mut replay_opts = opts.clone();
                        replay_opts.h0 = implicit_seed
                            .unwrap_or(opts.h0)
                            .min(target - start)
                            .max(AdaptiveStepConfig::default().min_step);
                        replay_opts.max_steps = if config.strict_return_work_improvement {
                            remaining
                        } else {
                            remaining.min(config.implicit_pilot_attempt_budget)
                        };
                        let comparator_ceiling = if config.strict_return_work_improvement {
                            strict_return_comparator_ceiling(&record, problem.dimension)
                        } else {
                            config
                                .stop_dominated_return_comparator
                                .then(|| {
                                    dominated_comparator_ceiling(
                                        &record,
                                        problem.dimension,
                                        return_ratio,
                                    )
                                })
                                .flatten()
                        };
                        match run(
                            problem,
                            start,
                            target,
                            &result.final_state,
                            &replay_opts,
                            SwitchingMethod::Rodas5p,
                            true,
                            false,
                            t1,
                            &[],
                            comparator_ceiling,
                        ) {
                            Ok(mut replay) => {
                                charge(&mut result.stats, &replay.record);
                                if replay.fatal {
                                    // Retain the last committed checkpoint. The
                                    // completed explicit probe is deliberately
                                    // not published after a fatal comparator.
                                    record.decision = "fatal same-window implicit comparator; retain committed checkpoint";
                                    replay.record.decision = record.decision;
                                    result.message = record.decision.into();
                                    result.stats.segments.push(record);
                                    result.stats.segments.push(replay.record);
                                    break 'windows;
                                }
                                // A nonfatal incomplete comparator's charged work is a
                                // lower bound on completing this same deterministic run
                                // (replay-safe callbacks; counters only grow; a budget
                                // stop is a prefix), so an explicit return below that
                                // bound wins without its output.
                                // Only a completed comparator is output-qualified.
                                // An incomplete comparator instead contributes a
                                // monotone prefix-cost lower bound; this separate
                                // predicate never labels its output admissible.
                                let prefix_preferred = !replay.record.success
                                    && if config.strict_return_work_improvement {
                                        strict_return_work_preferred(
                                            &replay.record,
                                            &record,
                                            problem.dimension,
                                        )
                                    } else {
                                        prefix_cost_dominates(
                                            &record,
                                            &replay.record,
                                            problem.dimension,
                                            return_ratio,
                                        )
                                    };
                                let dominated_comparator = (config
                                    .stop_dominated_return_comparator
                                    || config.strict_return_work_improvement)
                                    && replay.ceiling_stop
                                    && prefix_preferred;
                                economical = if config.strict_return_work_improvement {
                                    strict_return_work_preferred(
                                        &replay.record,
                                        &record,
                                        problem.dimension,
                                    )
                                } else if replay.record.success {
                                    qualified_work_preferred(
                                        &replay.record,
                                        &record,
                                        true,
                                        return_ratio,
                                        result.stats.segments.len() as u64,
                                    )
                                } else {
                                    prefix_preferred
                                };
                                record.comparator_work_ratio = if replay.record.success {
                                    (replay.record.work_units > 0.0)
                                        .then_some(record.work_units / replay.record.work_units)
                                } else {
                                    prefix_ratio_upper(&record, &replay.record, problem.dimension)
                                };
                                replay.record.decision = match (replay.record.success, economical) {
                                    _ if dominated_comparator => {
                                        "implicit comparator stopped after callback cost proves explicit dominance"
                                    }
                                    (false, false) => {
                                        "same-window implicit comparator incomplete; retain implicit route"
                                    }
                                    (false, true) => {
                                        "same-window implicit comparator incomplete above explicit work; discarded"
                                    }
                                    (true, true) => {
                                        "same-window implicit comparator discarded after qualified explicit win"
                                    }
                                    (true, false) => {
                                        "same-window implicit comparator retained as productive implicit advance"
                                    }
                                };
                                return_comparator = Some(replay);
                            }
                            Err(message) => {
                                result.stats.accounting_complete = false;
                                record.decision = "same-window implicit comparator failed without work record; stop";
                                result.message = message;
                                result.stats.segments.push(record);
                                break 'windows;
                            }
                        }
                    }
                }
                if economical {
                    record.decision = if return_comparator
                        .as_ref()
                        .is_some_and(|replay| !replay.record.success)
                    {
                        "explicit return passed same-window lower-bound comparison"
                    } else if return_comparator.is_some() {
                        "explicit return passed actual same-window work comparison"
                    } else {
                        "explicit probe completed within return work-rate hysteresis"
                    };
                    record.evidence = if return_comparator
                        .as_ref()
                        .is_some_and(|replay| !replay.record.success)
                    {
                        SwitchingEvidence::ExplicitCheaperThanComparatorLowerBound
                    } else if return_comparator.is_some() {
                        SwitchingEvidence::ExplicitCheaperOnSameWindow
                    } else {
                        SwitchingEvidence::ExplicitCheaperThanPreviousWindow
                    };
                    explicit_step = record.proposed_next_step;
                    commit(&mut result, &mut record, state, landings);
                    method = SwitchingMethod::Dopri5;
                    entry_memory = EntryProbeMemory::default();
                    batches_since_entry_pilot = usize::MAX;
                    result.stats.switches += 1;
                    implicit_windows = 0;
                    pilot_discards = 0;
                    explicit_rate_at_discard = None;
                    probe_discards = 0;
                    implicit_rate_at_discard = None;
                    implicit_seed = None;
                    tenure_rechecks = 0;
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
                if let Some(mut replay) = return_comparator {
                    if !economical && replay.record.success {
                        replay.record.evidence = SwitchingEvidence::ImplicitPreferredOnSameWindow;
                        implicit_rate = Some(replay.record.work_units / (target - start));
                        implicit_windows += 1;
                        since_probe += 1;
                        implicit_seed = (replay.record.accepted_steps > 0).then(|| {
                            (replay.record.reached_time - start)
                                / replay.record.accepted_steps as f64
                        });
                        explicit_step = None;
                        commit(
                            &mut result,
                            &mut replay.record,
                            replay.state,
                            replay.landings,
                        );
                    }
                    result.stats.segments.push(replay.record);
                }
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
                    let rearm = config.entry_probe_adaptation.is_none()
                        && rate.is_finite()
                        && explicit_rate_at_discard
                            .is_some_and(|old| rate >= config.rearm_rate_ratio * old);
                    if rearm {
                        pilot_discards = 0;
                    }
                    let adaptive_due = if let Some(a) = &config.entry_probe_adaptation {
                        let (reason, due) =
                            entry_memory.schedule(&record, batches_since_entry_pilot, config, a);
                        record.entry_probe_reason = Some(reason);
                        due
                    } else {
                        true
                    };
                    if fallback
                        && pressure
                        && (record.success || budget_stop)
                        && record.reached_time < t1
                    {
                        let mut enter = config.implicit_pilot_attempt_budget == 0;
                        let mut entry_seed = None;
                        if config.implicit_pilot_attempt_budget > 0 {
                            let remaining = opts.max_steps - result.stats.attempts;
                            if (pilot_eligible || rearm)
                                && (config.entry_probe_adaptation.is_some()
                                    || implicit_pilots < config.implicit_pilot_limit)
                                && adaptive_due
                                && remaining > 0
                                && record.reached_time > start
                            {
                                implicit_pilots += 1;
                                batches_since_entry_pilot = 0;
                                local.h0 = entry_pilot_checkpoint_seed(
                                    &record,
                                    opts.h0,
                                    config.seed_entry_pilot_from_explicit_checkpoint,
                                )
                                .max(if config.reuse_observed_entry_step {
                                    entry_memory.observed_entry_h.unwrap_or(opts.h0)
                                } else {
                                    opts.h0
                                })
                                .min(record.reached_time - start)
                                .max(AdaptiveStepConfig::default().min_step);
                                local.max_steps =
                                    remaining.min(config.implicit_pilot_attempt_budget);
                                // Charged callback work alone past this bound
                                // means the replay cannot pass: stop it there.
                                let entry_ceiling = if config.strict_entry_work_improvement {
                                    strict_entry_callback_ceiling(&record, problem.dimension)
                                } else {
                                    Some((config.entry_cost_ratio * record.work_units).floor()
                                        as u64)
                                };
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
                                    entry_ceiling,
                                ) {
                                    Ok(mut pilot) => {
                                        charge(&mut result.stats, &pilot.record);
                                        enter = if config.strict_entry_work_improvement {
                                            strict_entry_work_preferred(
                                                &record,
                                                &pilot.record,
                                                problem.dimension,
                                            )
                                        } else if config.entry_probe_adaptation.is_some() {
                                            // Both records are freshly constructed from the
                                            // same physical checkpoint and accepted prefix.
                                            // This is observed local-tolerance work evidence,
                                            // not a future-time or global-error guarantee.
                                            qualified_work_preferred(
                                                &record,
                                                &pilot.record,
                                                record.accepted_steps > 0,
                                                config.entry_cost_ratio,
                                                result.stats.segments.len() as u64,
                                            )
                                        } else {
                                            pilot.record.success
                                                && pilot.record.work_units
                                                    <= config.entry_cost_ratio * record.work_units
                                        };
                                        pilot.record.comparator_work_ratio =
                                            if pilot.record.success && record.work_units > 0.0 {
                                                Some(pilot.record.work_units / record.work_units)
                                            } else {
                                                None
                                            };
                                        pilot.record.entry_probe_reason = record.entry_probe_reason;
                                        if config.entry_probe_adaptation.is_some() {
                                            entry_memory.compared(&record, enter);
                                            if config.retry_unresolved_entry_once
                                                && !enter
                                                && !pilot.fatal
                                                && !pilot.record.success
                                                && pilot.record.accepted_steps == 0
                                            {
                                                entry_memory.retry_zero_progress_once();
                                            }
                                        }
                                        pilot.record.evidence = if !pilot.record.success {
                                            SwitchingEvidence::ImplicitPilotIncomplete
                                        } else if enter {
                                            SwitchingEvidence::ImplicitCheaperOnPrefix
                                        } else {
                                            SwitchingEvidence::ImplicitNotCheaperOnPrefix
                                        };
                                        pilot.record.decision = if enter
                                            && config.strict_entry_work_improvement
                                        {
                                            "same-checkpoint implicit pilot passed strict integer work comparison"
                                        } else if enter {
                                            "same-checkpoint implicit pilot passed entry work ratio"
                                        } else if pilot.ceiling_stop {
                                            "same-checkpoint implicit pilot discarded: exceeded entry work ceiling"
                                        } else {
                                            "same-checkpoint implicit pilot discarded: incomplete or uneconomical"
                                        };
                                        let pilot_fatal = pilot.fatal;
                                        if config.reuse_entry_pilot_step && enter && !pilot_fatal {
                                            entry_seed =
                                                entry_pilot_growth_seed(&pilot.record, opts.h0);
                                        }
                                        if config.reuse_observed_entry_step
                                            && !pilot_fatal
                                            && let Some(observed) =
                                                observed_entry_step(&pilot.record)
                                        {
                                            entry_memory.observed_entry_h = Some(observed);
                                            if enter {
                                                entry_seed = Some(
                                                    entry_seed.unwrap_or(opts.h0).max(observed),
                                                );
                                            }
                                        }
                                        if !enter
                                            && !pilot_fatal
                                            && config.entry_probe_adaptation.is_none()
                                        {
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
                            implicit_seed = entry_seed;
                            tenure_rechecks = 0;
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
        entry_memory,
        explicit_step,
        next_window: config.window_count + 1,
        pilot_discards,
        explicit_rate_at_discard,
        probe_discards,
        implicit_rate_at_discard,
        implicit_seed,
        tenure_rechecks,
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
        entry_probe_reason: None,
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
            entry_probe_reason: None,
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
            entry_probe_reason: None,
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
    if let Some(a) = &c.entry_probe_adaptation
        && (a.max_backoff_batches == 0
            || a.max_backoff_batches < c.entry_pilot_cooldown_batches.max(1)
            || !a.work_rate_growth.is_finite()
            || a.work_rate_growth <= 1.0
            || !(a.step_shrink_ratio > 0.0 && a.step_shrink_ratio < 1.0)
            || !(a.rejection_fraction_increase > 0.0 && a.rejection_fraction_increase < 1.0)
            || c.implicit_pilot_attempt_budget == 0
            || !c.carry_explicit_step)
    {
        return Err(bad(
            "adaptive entry requires finite deterioration thresholds, positive bounded backoff, competitive pilots and persistent explicit sessions",
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

// Private contract for a productive same-window implicit return comparator.
// Seed an adversarial stale previous-window rate to exercise the exact branch.
// This is a routing-state fixture, not an end-to-end performance observation.
#[cfg(test)]
mod adaptive_return_productive_contract {
    use super::*;

    #[test]
    fn entry_checkpoint_seed_uses_only_valid_start_proposal() {
        let p = Problem::new(1, |_, _, out| {
            out[0] = 0.0;
            Ok(())
        });
        let mut r = SwitchingSolver::solve(
            &p,
            (0.0, 0.0),
            &[0.0],
            &Options::default(),
            &SwitchingConfig::default(),
        )
        .unwrap();
        let mut explicit = r.stats.segments.remove(0);
        explicit.initial_step = 0.03;
        explicit.proposed_next_step = Some(100.0);
        assert_eq!(entry_pilot_checkpoint_seed(&explicit, 0.001, false), 0.001);
        assert_eq!(entry_pilot_checkpoint_seed(&explicit, 0.001, true), 0.03);
        explicit.proposed_next_step = Some(1e-30);
        assert_eq!(entry_pilot_checkpoint_seed(&explicit, 0.001, true), 0.03);
        for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            explicit.initial_step = invalid;
            assert_eq!(entry_pilot_checkpoint_seed(&explicit, 0.001, true), 0.001);
        }
        explicit.initial_step = 1e-6;
        assert_eq!(entry_pilot_checkpoint_seed(&explicit, 0.001, true), 0.001);
    }

    #[test]
    fn unresolved_retry_is_sticky_but_never_rearms_within_one_explicit_tenure() {
        let p = Problem::new(1, |_, _, out| {
            out[0] = 0.0;
            Ok(())
        });
        let config = SwitchingConfig::adaptive_evidence(DerivativeFallback::Forbidden);
        let mut r =
            SwitchingSolver::solve(&p, (0.0, 0.0), &[0.0], &Options::default(), &config).unwrap();
        let mut sample = r.stats.segments.remove(0);
        sample.start_time = 0.0;
        sample.reached_time = 1.0;
        sample.accepted_steps = 4;
        sample.attempts = 4;
        sample.work_units = 100.0;
        sample.proposed_next_step = Some(0.1);
        let a = config.entry_probe_adaptation.as_ref().unwrap();
        let mut memory = EntryProbeMemory::default();
        memory.compared(&sample, false);
        memory.retry_zero_progress_once();
        for elapsed in [1, 2, 3] {
            assert_eq!(
                memory.schedule(&sample, elapsed, &config, a),
                (EntryProbeReason::UnresolvedEntryRetry, true)
            );
        }
        for _ in 0..100 {
            memory.compared(&sample, false);
            memory.retry_zero_progress_once();
            assert!(memory.zero_progress_retry_used);
            assert!(!memory.pending_zero_progress_retry);
            assert_eq!(
                memory.schedule(&sample, 1, &config, a),
                (EntryProbeReason::StableEvidenceBackoff, false)
            );
        }
        // Dropping the explicit tenure, rather than a timer or another loss,
        // grants a later explicit tenure its own single retry.
        memory = EntryProbeMemory::default();
        memory.retry_zero_progress_once();
        assert!(memory.pending_zero_progress_retry);
    }

    #[test]
    fn entry_seed_never_reduces_the_requested_initial_proposal() {
        let p = Problem::new(1, |_, _, out| {
            out[0] = 0.0;
            Ok(())
        });
        let mut r = SwitchingSolver::solve(
            &p,
            (0.0, 0.0),
            &[0.0],
            &Options::default(),
            &SwitchingConfig::default(),
        )
        .unwrap();
        let mut pilot = r.stats.segments.remove(0);
        pilot.success = true;
        pilot.start_time = 0.0;
        pilot.reached_time = 1e-3;
        pilot.accepted_steps = 10;
        assert_eq!(entry_pilot_growth_seed(&pilot, 1e-3), None);
        assert_eq!(entry_pilot_growth_seed(&pilot, 1e-4), None);
        assert_eq!(entry_pilot_growth_seed(&pilot, 1e-5), Some(1e-4));
        pilot.accepted_steps = 0;
        assert_eq!(entry_pilot_growth_seed(&pilot, 1e-5), None);
        pilot.accepted_steps = 10;
        pilot.success = false;
        assert_eq!(entry_pilot_growth_seed(&pilot, 1e-5), None);
    }

    #[test]
    fn observed_entry_step_accepts_only_finite_positive_accepted_prefixes() {
        let p = Problem::new(1, |_, _, out| {
            out[0] = 0.0;
            Ok(())
        });
        let mut r = SwitchingSolver::solve(
            &p,
            (0.0, 0.0),
            &[0.0],
            &Options::default(),
            &SwitchingConfig::default(),
        )
        .unwrap()
        .stats
        .segments
        .remove(0);
        r.start_time = 0.25;
        r.reached_time = 0.375;
        r.accepted_steps = 4;
        r.success = false;
        assert_eq!(observed_entry_step(&r), Some(0.03125));
        r.success = true;
        assert_eq!(observed_entry_step(&r), Some(0.03125));
        r.accepted_steps = 0;
        assert_eq!(observed_entry_step(&r), None);
        r.accepted_steps = 4;
        for end in [r.start_time, 0.0, f64::NAN, f64::INFINITY] {
            r.reached_time = end;
            assert_eq!(observed_entry_step(&r), None);
        }
        r.start_time = -f64::MAX;
        r.reached_time = f64::MAX;
        assert_eq!(observed_entry_step(&r), None);
    }

    #[test]
    fn implicit_tiny_representable_tail_uses_native_endpoint_clipping() {
        let p = Problem::new(1, |_, _, out| {
            out[0] = 1.0;
            Ok(())
        })
        .jacobian(Jacobian::dense(|_, _, out| {
            out[0] = 0.0;
            Ok(())
        }))
        .autonomous();
        let opts = Options::default();
        let config = SwitchingConfig {
            window_count: 1,
            ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
        };
        let result = SwitchingSolver::solve(&p, (0.0, 0.0), &[0.0], &opts, &config).unwrap();
        let state = DriveState {
            result,
            method: SwitchingMethod::Rodas5p,
            implicit_windows: 0,
            since_probe: 0,
            implicit_rate: None,
            implicit_pilots: 0,
            batches_since_entry_pilot: 0,
            entry_memory: EntryProbeMemory::default(),
            explicit_step: None,
            next_window: 1,
            pilot_discards: 0,
            explicit_rate_at_discard: None,
            probe_discards: 0,
            implicit_rate_at_discard: None,
            implicit_seed: None,
            tenure_rechecks: 0,
        };
        let r = drive_windows(&p, (0.0, 1e-15), &opts, &config, state, None).result;
        assert_eq!(r.final_time, 1e-15, "{}", r.message);
        assert!(r.stats.accounting_complete);
        assert!((r.final_state[0] - 1e-15).abs() < 1e-28);
        assert_eq!(r.stats.segments.len(), 1);
        assert!(r.stats.segments[0].success && r.stats.segments[0].committed);
        assert_eq!(r.stats.segments[0].method, SwitchingMethod::Rodas5p);
    }

    #[test]
    fn return_work_ceiling_saturates_for_overflowing_short_interval_rate() {
        assert_eq!(return_work_ceiling(100.0, 0.5, 0.8), 41);
        assert_eq!(return_work_ceiling(f64::MAX, 2.0, 0.8), u64::MAX);
        assert_eq!(return_work_ceiling(f64::INFINITY, 1e-300, 0.8), u64::MAX);
    }

    #[test]
    fn comparison_memory_renews_on_joint_drift_in_either_direction() {
        let p = Problem::new(1, |_, _, out| {
            out[0] = 0.0;
            Ok(())
        });
        let opts = Options::default();
        let config = SwitchingConfig::adaptive_evidence(DerivativeFallback::Forbidden);
        let mut result = SwitchingSolver::solve(&p, (0.0, 0.0), &[1.0], &opts, &config).unwrap();
        let mut sample = result.stats.segments.remove(0);
        // Manufactured decision telemetry: no benchmark or physics claim.
        sample.start_time = 0.0;
        sample.reached_time = 1.0;
        sample.attempts = 4;
        sample.accepted_steps = 4;
        sample.rejected_steps = 0;
        let memory = EntryProbeMemory {
            negative_comparisons: 1,
            anchor_rate: Some(100.0),
            anchor_h: Some(1.0),
            anchor_rejection: 0.0,
            pending_renewal: false,
            pending_zero_progress_retry: false,
            zero_progress_retry_used: false,
            observed_entry_h: None,
        };
        let adapt = config.entry_probe_adaptation.as_ref().unwrap();
        for (rate, h, expected) in [
            (220.0, 0.4, EntryProbeReason::WorkDeteriorated),
            (40.0, 3.0, EntryProbeReason::WorkRegimeRelaxed),
            (40.0, 0.4, EntryProbeReason::StableEvidenceBackoff),
            (220.0, 3.0, EntryProbeReason::StableEvidenceBackoff),
            (100.0, 3.0, EntryProbeReason::StableEvidenceBackoff),
            (40.0, 1.0, EntryProbeReason::StableEvidenceBackoff),
        ] {
            sample.work_units = rate;
            sample.proposed_next_step = Some(h);
            assert_eq!(memory.reason(&sample, 1, &config, adapt), expected);
        }
        sample.work_units = 40.0;
        sample.proposed_next_step = Some(3.0);
        sample.accepted_steps = 3;
        assert_eq!(
            memory.reason(&sample, 1, &config, adapt),
            EntryProbeReason::StableEvidenceBackoff
        );
        let mut pending = memory;
        sample.accepted_steps = 4;
        assert_eq!(
            pending.schedule(&sample, 1, &config, adapt),
            (EntryProbeReason::WorkRegimeRelaxed, false)
        );
        assert!(pending.pending_renewal);
        // Scheduling alone does not consume the request: an ineligible batch
        // must leave it available for the next actual comparison.
        for elapsed in [2, 3] {
            assert_eq!(
                pending.schedule(&sample, elapsed, &config, adapt),
                (EntryProbeReason::PendingRegimeComparison, true)
            );
            assert!(pending.pending_renewal);
        }
        pending.compared(&sample, false);
        assert!(!pending.pending_renewal);
        assert_eq!(
            pending.schedule(&sample, 1, &config, adapt),
            (EntryProbeReason::StableEvidenceBackoff, false)
        );
    }

    #[test]
    fn stale_old_rate_cannot_replace_a_cheaper_same_window_implicit_candidate() {
        let p = Problem::new(1, |t, y, out| {
            out[0] = -10000.0 * (y[0] - t.cos()) - t.sin();
            Ok(())
        })
        .jacobian(Jacobian::dense(|_, _, j| {
            j[0] = -10000.0;
            Ok(())
        }))
        .time_derivative(|t, _, out| {
            out[0] = -10000.0 * t.sin() - t.cos();
            Ok(())
        });
        let opts = Options {
            rtol: 1e-7,
            atol: Atol::Scalar(1e-10),
            h0: 1e-4,
            max_steps: 1000,
            ..Default::default()
        };
        let config = SwitchingConfig {
            window_count: 1,
            implicit_residence: 1,
            probe_interval: 1,
            probe_attempt_budget: 64,
            return_cost_ratio: 0.99,
            ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
        };
        // Build only a finite initial result; zero-span invokes no stage.
        let mut result = SwitchingSolver::solve(&p, (0.0, 0.0), &[1.0], &opts, &config).unwrap();
        result.success = false;
        result.message.clear();
        let state = DriveState {
            result,
            method: SwitchingMethod::Rodas5p,
            implicit_windows: 1,
            since_probe: 1,
            implicit_rate: Some(1e12),
            implicit_pilots: 0,
            batches_since_entry_pilot: 0,
            entry_memory: EntryProbeMemory::default(),
            explicit_step: None,
            next_window: 1,
            pilot_discards: 0,
            explicit_rate_at_discard: None,
            probe_discards: 0,
            implicit_rate_at_discard: None,
            implicit_seed: None,
            tenure_rechecks: 0,
        };
        let after = drive_windows(&p, (0.0, 0.01), &opts, &config, state, None);
        let r = after.result;
        assert_eq!(r.final_time, 0.01);
        assert!((r.final_state[0] - 0.01_f64.cos()).abs() < 1e-6);
        assert_eq!(r.stats.segments.len(), 2);
        let explicit = &r.stats.segments[0];
        let implicit = &r.stats.segments[1];
        assert!(explicit.is_probe && explicit.success && !explicit.committed);
        assert!(implicit.is_probe && implicit.success && implicit.committed);
        assert_eq!(
            implicit.evidence,
            SwitchingEvidence::ImplicitPreferredOnSameWindow
        );
        assert_eq!(explicit.start_time, implicit.start_time);
        assert_eq!(explicit.target_time, implicit.target_time);
        assert!(implicit.work_units < explicit.work_units);
        assert_eq!(r.stats.switches, 0);
        assert_eq!(after.method, SwitchingMethod::Rodas5p);
        assert_eq!(after.since_probe, 1);
        assert_eq!(after.implicit_windows, 2);
        assert_eq!(
            r.stats.attempts,
            r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
        );
        assert_eq!(
            r.stats.rhs_calls,
            r.stats.segments.iter().map(|s| s.rhs_calls).sum::<u64>()
        );
        assert_eq!(
            r.stats.accepted_steps,
            r.stats
                .segments
                .iter()
                .map(|s| s.accepted_steps)
                .sum::<usize>()
        );
        assert_eq!(
            r.stats.rejected_steps,
            r.stats
                .segments
                .iter()
                .map(|s| s.rejected_steps)
                .sum::<usize>()
        );
        assert_eq!(
            r.stats.jacobian_builds,
            r.stats
                .segments
                .iter()
                .map(|s| s.jacobian_builds)
                .sum::<u64>()
        );
        assert_eq!(
            r.stats.jvp_vectors,
            r.stats.segments.iter().map(|s| s.jvp_vectors).sum::<u64>()
        );
        assert_eq!(
            r.stats.ft_calls,
            r.stats.segments.iter().map(|s| s.ft_calls).sum::<u64>()
        );
        assert_eq!(
            r.stats.linear_solves,
            r.stats
                .segments
                .iter()
                .map(|s| s.linear_solves)
                .sum::<u64>()
        );
    }
}

#[cfg(test)]
mod comparator_dominance_arithmetic_contract {
    use super::*;

    fn empty_record() -> SwitchingSegment {
        let problem = Problem::new(1, |_, _, out| {
            out[0] = 0.0;
            Ok(())
        });
        SwitchingSolver::solve(
            &problem,
            (0.0, 0.0),
            &[0.0],
            &Options::default(),
            &SwitchingConfig::default(),
        )
        .unwrap()
        .stats
        .segments
        .remove(0)
    }

    #[test]
    fn outward_cost_check_never_promotes_an_inexact_boundary() {
        let mut candidate = empty_record();
        let mut comparator = candidate.clone();
        candidate.success = true;
        candidate.rhs_calls = (1u64 << 53) + 1;
        comparator.rhs_calls = 2 * ((1u64 << 53) + 1);
        // The exact integer ratio is one half; the displayed enclosure
        // must not round below it even though neither integer is exact f64.
        assert!(prefix_ratio_upper(&candidate, &comparator, 1).unwrap() >= 0.5);
        assert!(!prefix_cost_dominates(&candidate, &comparator, 1, 0.5));
        comparator.rhs_calls += 16;
        assert!(prefix_cost_dominates(&candidate, &comparator, 1, 0.5));
        candidate.success = false;
        assert!(!prefix_cost_dominates(&candidate, &comparator, 1, 0.5));
        candidate.success = true;
        candidate.rhs_calls = 80;
        comparator.rhs_calls = 160;
        assert!(!prefix_cost_dominates(&candidate, &comparator, 1, 0.5));
        comparator.rhs_calls = 161;
        assert!(prefix_cost_dominates(&candidate, &comparator, 1, 0.5));
    }

    #[test]
    fn ceiling_uses_upper_candidate_cost_and_prefix_lower_bound() {
        let mut candidate = empty_record();
        let mut comparator = candidate.clone();
        candidate.success = true;
        candidate.rhs_calls = 20;
        candidate.linear_solves = 3;
        let ceiling = dominated_comparator_ceiling(&candidate, 1, 0.8).unwrap();
        assert!(ceiling >= 29);
        comparator.rhs_calls = ceiling + 1;
        assert!(prefix_cost_dominates(&candidate, &comparator, 1, 0.8));
        // Already-spent linear solves are also a valid completion-cost
        // lower bound even though the stopping guard meters only callbacks.
        comparator.rhs_calls = 10;
        comparator.linear_solves = 1000;
        assert!(prefix_cost_dominates(&candidate, &comparator, 1, 0.8));
    }

    fn replay_pair() -> (SwitchingSegment, SwitchingSegment) {
        let mut incumbent = empty_record();
        incumbent.start_time = 0.25;
        // The accepted prefix may stop before the requested batch endpoint.
        incumbent.target_time = 1.0;
        incumbent.reached_time = 0.5;
        incumbent.accepted_steps = 1;
        incumbent.success = false;
        let mut candidate = incumbent.clone();
        candidate.target_time = incumbent.reached_time;
        candidate.success = true;
        (incumbent, candidate)
    }

    #[test]
    fn strict_entry_keeps_ties_and_resolves_one_unit_above_float_precision() {
        let (mut incumbent, mut candidate) = replay_pair();
        // Displayed floating costs deliberately contain no useful difference.
        incumbent.work_units = 1.0;
        candidate.work_units = 1.0;
        for cost in [1, 192, 1 << 53, (1 << 53) + 1, u64::MAX - 1] {
            incumbent.rhs_calls = cost;
            candidate.rhs_calls = cost;
            assert!(!strict_entry_work_preferred(&incumbent, &candidate, 1));
            candidate.rhs_calls = cost - 1;
            assert!(strict_entry_work_preferred(&incumbent, &candidate, 1));
            candidate.rhs_calls = cost + 1;
            assert!(!strict_entry_work_preferred(&incumbent, &candidate, 1));
        }
    }

    #[test]
    fn strict_entry_counts_all_existing_work_categories_with_checked_arithmetic() {
        let (mut incumbent, mut candidate) = replay_pair();
        incumbent.rhs_calls = 5;
        incumbent.jvp_vectors = 7;
        incumbent.ft_calls = 11;
        incumbent.jacobian_builds = 13;
        incumbent.linear_solves = 19;
        assert_eq!(integer_work(&incumbent, 17, false), Some(263));
        assert_eq!(integer_work(&incumbent, 17, true), Some(244));
        candidate.rhs_calls = 262;
        assert!(strict_entry_work_preferred(&incumbent, &candidate, 17));
        candidate.linear_solves = 1;
        assert!(!strict_entry_work_preferred(&incumbent, &candidate, 17));
        if usize::BITS == 64 {
            candidate.rhs_calls = u64::MAX;
            candidate.jvp_vectors = u64::MAX;
            candidate.ft_calls = u64::MAX;
            candidate.jacobian_builds = u64::MAX;
            candidate.linear_solves = u64::MAX;
            assert_eq!(integer_work(&candidate, usize::MAX, false), None);
            assert!(!strict_entry_work_preferred(
                &incumbent,
                &candidate,
                usize::MAX
            ));
            assert_eq!(strict_entry_callback_ceiling(&candidate, usize::MAX), None);
        }
    }

    #[test]
    fn strict_entry_requires_qualified_completion_of_the_same_accepted_prefix() {
        let (mut incumbent, mut candidate) = replay_pair();
        incumbent.rhs_calls = 192;
        candidate.rhs_calls = 160;
        assert!(strict_entry_work_preferred(&incumbent, &candidate, 1));
        for changed in 0..5 {
            let mut rejected = candidate.clone();
            match changed {
                0 => rejected.success = false,
                1 => rejected.start_time = 0.0,
                2 => rejected.target_time = 0.75,
                3 => rejected.reached_time = 0.375,
                _ => rejected.start_time = f64::NAN,
            }
            assert!(!strict_entry_work_preferred(&incumbent, &rejected, 1));
        }
        incumbent.accepted_steps = 0;
        assert!(!strict_entry_work_preferred(&incumbent, &candidate, 1));
    }

    #[test]
    fn strict_entry_callback_ceiling_is_exact_and_never_truncates_large_costs() {
        let (mut incumbent, _) = replay_pair();
        for (cost, cap) in [
            (0, 0),
            (1, 0),
            (192, 191),
            ((1 << 53) + 1, 1 << 53),
            (u64::MAX, u64::MAX - 1),
        ] {
            incumbent.rhs_calls = cost;
            assert_eq!(strict_entry_callback_ceiling(&incumbent, 1), Some(cap));
        }
        incumbent.jvp_vectors = 1;
        assert_eq!(strict_entry_callback_ceiling(&incumbent, 1), None);
    }

    #[test]
    fn strict_return_requires_same_window_and_resolves_exact_integer_boundary() {
        let (mut incumbent, mut candidate) = replay_pair();
        incumbent.target_time = candidate.target_time;
        for cost in [1, 192, (1 << 53) + 1, u64::MAX - 1] {
            candidate.rhs_calls = cost;
            incumbent.rhs_calls = cost;
            assert!(!strict_return_work_preferred(&incumbent, &candidate, 1));
            incumbent.rhs_calls += 1;
            assert!(strict_return_work_preferred(&incumbent, &candidate, 1));
            // An incomplete prefix supplies cost evidence even at zero progress.
            incumbent.reached_time = incumbent.start_time;
            incumbent.accepted_steps = 0;
            assert!(strict_return_work_preferred(&incumbent, &candidate, 1));
        }
        for changed in 0..7 {
            let mut bad = incumbent.clone();
            match changed {
                0 => bad.start_time = 0.0,
                1 => bad.target_time = 0.75,
                2 => bad.reached_time = f64::NAN,
                3 => bad.reached_time = 0.0,
                4 => bad.reached_time = 0.75,
                5 => bad.success = true,
                _ => bad.start_time = f64::INFINITY,
            }
            assert!(!strict_return_work_preferred(&bad, &candidate, 1));
        }
        candidate.success = false;
        assert!(!strict_return_work_preferred(&incumbent, &candidate, 1));
    }

    #[test]
    fn strict_return_ceiling_is_candidate_cost_and_checked_overflow_refuses() {
        let (mut incumbent, mut candidate) = replay_pair();
        incumbent.target_time = candidate.target_time;
        for cost in [0, 1, 192, (1 << 53) + 1, u64::MAX - 1] {
            candidate.rhs_calls = cost;
            assert_eq!(strict_return_comparator_ceiling(&candidate, 1), Some(cost));
            incumbent.rhs_calls = cost + 1;
            assert!(strict_return_work_preferred(&incumbent, &candidate, 1));
        }
        candidate.rhs_calls = u64::MAX;
        assert_eq!(strict_return_comparator_ceiling(&candidate, 1), None);
        if usize::BITS == 64 {
            for r in [&mut incumbent, &mut candidate] {
                r.rhs_calls = u64::MAX;
                r.jacobian_builds = u64::MAX;
                r.jvp_vectors = u64::MAX;
                r.ft_calls = u64::MAX;
                r.linear_solves = u64::MAX;
            }
            assert_eq!(
                strict_return_comparator_ceiling(&candidate, usize::MAX),
                None
            );
            assert!(!strict_return_work_preferred(
                &incumbent,
                &candidate,
                usize::MAX
            ));
        }
    }
}
