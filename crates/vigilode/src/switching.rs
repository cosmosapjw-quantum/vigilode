//! Bounded, opt-in explicit/RODAS switching at accepted window boundaries.
//!
//! This adapter uses observed work, not a stiffness estimate. Its default is
//! RHS-only. Derivative fallbacks require a separate explicit authorization.

use rodas5p_integrators::{AdaptiveStepConfig, Dopri5StopReason};
use serde::Serialize;

use crate::{Atol, Dopri5, ExplicitStats, Jacobian, Options, Problem, Rodas5p, Stats, Status};

/// Authorization for derivative work after the explicit path shows pressure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DerivativeFallback {
    /// Use RHS-only DOPRI5 throughout, including after a bounded batch ends.
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
        }
    }
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
    /// Whether this is a bounded recheck while the implicit route is active.
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
    pub decision: String,
    /// Original telemetry; useful when a partial run has stopped.
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
    /// Every attempted batch, including uncommitted probes.
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
    /// again. Window restarts currently reconstruct each driver's workspace.
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
        let mut method = SwitchingMethod::Dopri5;
        let mut implicit_windows = 0usize;
        let mut since_probe = 0usize;
        let mut implicit_rate: Option<f64> = None;
        'windows: for window in 1..=config.window_count {
            if result.final_time == t1 {
                break;
            }
            let target = if window == config.window_count {
                t1
            } else {
                t0 + (t1 - t0) * (window as f64 / config.window_count as f64)
            };
            if target <= result.final_time {
                continue;
            }
            let mut probe = method == SwitchingMethod::Rodas5p
                && implicit_windows >= config.implicit_residence
                && since_probe >= config.probe_interval;
            while result.final_time < target {
                let remaining = opts.max_steps - result.stats.attempts;
                if remaining == 0 {
                    result.message = "total attempt budget exhausted".into();
                    break 'windows;
                }
                let start = result.final_time;
                let selected = if probe {
                    SwitchingMethod::Dopri5
                } else {
                    method
                };
                let mut local = opts.clone();
                local.h0 = opts.h0.min(target - start);
                local.max_steps = remaining.min(match selected {
                    SwitchingMethod::Dopri5 if probe => config.probe_attempt_budget,
                    SwitchingMethod::Dopri5 if fallback => config.explicit_attempt_budget,
                    // With no fallback/probes, a single driver can use the
                    // entire remaining budget instead of needless batch restarts.
                    _ => remaining,
                });
                let batch = match run(
                    problem,
                    start,
                    target,
                    &result.final_state,
                    &local,
                    selected,
                    probe,
                ) {
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
                    fatal,
                    budget_stop,
                } = batch;
                charge(&mut result.stats, &record);
                if record.attempts == 0 && record.reached_time < target && !fatal {
                    record.decision = "driver stopped without progress or attempted step".into();
                    result.message = record.decision.clone();
                    result.stats.segments.push(record);
                    break 'windows;
                }
                if probe {
                    since_probe = 0;
                    let rate = record.work_units / (target - start);
                    let economical = record.success
                        && implicit_rate.is_some_and(|old| rate <= config.return_cost_ratio * old);
                    if economical {
                        record.decision =
                            "explicit probe completed within return work-rate hysteresis".into();
                        commit(&mut result, &mut record, state);
                        method = SwitchingMethod::Dopri5;
                        result.stats.switches += 1;
                        implicit_windows = 0;
                    } else if fatal {
                        record.decision =
                            "fatal explicit probe; stop without another driver".into();
                        commit(&mut result, &mut record, state);
                        result.message = record.decision.clone();
                        result.stats.segments.push(record);
                        break 'windows;
                    } else {
                        record.decision =
                            "bounded explicit probe discarded; retain implicit route".into();
                    }
                    result.stats.segments.push(record);
                    probe = false;
                    continue;
                }
                commit(&mut result, &mut record, state);
                if fatal {
                    record.decision = "fatal callback; stop without fallback".into();
                    result.message = record.decision.clone();
                    result.stats.segments.push(record);
                    break 'windows;
                }
                match selected {
                    SwitchingMethod::Dopri5 => {
                        let rejection_pressure = record.attempts > 0
                            && record.rejected_steps as f64 / record.attempts as f64
                                >= config.rejection_fraction;
                        let pressure = budget_stop
                            || record.attempts >= config.pressure_steps
                            || rejection_pressure;
                        if fallback
                            && pressure
                            && (record.success || budget_stop)
                            && result.final_time < t1
                        {
                            record.decision =
                                "observed explicit work pressure; enter authorized RODAS route"
                                    .into();
                            method = SwitchingMethod::Rodas5p;
                            implicit_windows = 0;
                            since_probe = 0;
                            result.stats.switches += 1;
                        } else if !record.success {
                            record.decision =
                                "explicit driver stopped; no eligible fallback".into();
                            result.message = record.decision.clone();
                            result.stats.segments.push(record);
                            break 'windows;
                        } else {
                            record.decision =
                                "explicit window completed below work pressure".into();
                        }
                    }
                    SwitchingMethod::Rodas5p => {
                        if !record.success {
                            record.decision =
                                "implicit driver stopped; preserve partial accepted state".into();
                            result.message = record.decision.clone();
                            result.stats.segments.push(record);
                            break 'windows;
                        }
                        implicit_rate = Some(record.work_units / (target - start));
                        implicit_windows += 1;
                        since_probe += 1;
                        record.decision =
                            "implicit window completed; retain route until bounded recheck".into();
                    }
                }
                result.stats.segments.push(record);
            }
        }
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

struct Batch {
    record: SwitchingSegment,
    state: Vec<f64>,
    fatal: bool,
    budget_stop: bool,
}

fn run(
    problem: &Problem,
    start: f64,
    target: f64,
    y: &[f64],
    opts: &Options,
    method: SwitchingMethod,
    probe: bool,
) -> Result<Batch, String> {
    let (
        reached,
        state,
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
        driver,
    ) = match method {
        SwitchingMethod::Dopri5 => {
            let r = Dopri5::solve(problem, (start, target), y, opts).map_err(|e| e.to_string())?;
            let c = r.stats.counters;
            (
                r.final_time,
                r.final_state,
                r.success,
                r.stats.attempts,
                r.stats.accepted_steps,
                r.stats.rejected_steps,
                c.rhs_calls,
                c.jacobian_builds,
                c.jvp_vectors,
                c.ft_calls,
                c.linear_solves,
                r.callback_failure.is_some(),
                r.stop_reason == Some(Dopri5StopReason::AttemptBudget),
                SwitchingDriverStats::Dopri5(Box::new(r.stats)),
            )
        }
        SwitchingMethod::Rodas5p => {
            let r = Rodas5p::solve(problem, (start, target), y, opts).map_err(|e| e.to_string())?;
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
            (
                *r.t.last().unwrap_or(&start),
                r.y.last().cloned().unwrap_or_else(|| y.to_vec()),
                success,
                r.stats.attempts,
                r.stats.accepted_steps,
                r.stats.rejected_steps,
                c.rhs_calls,
                c.jacobian_builds,
                c.jvp_vectors,
                c.ft_calls,
                c.linear_solves,
                fatal,
                budget,
                SwitchingDriverStats::Rodas5p(Box::new(r.stats)),
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
            decision: String::new(),
            driver,
        },
        state,
        fatal,
        budget_stop: budget,
    })
}

fn commit(result: &mut SwitchingSolution, record: &mut SwitchingSegment, state: Vec<f64>) {
    record.committed = true;
    if record.reached_time > result.final_time {
        result.final_time = record.reached_time;
        result.final_state = state;
        result.t.push(result.final_time);
        result.y.push(result.final_state.clone());
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
        || !(c.rejection_fraction > 0.0 && c.rejection_fraction <= 1.0)
        || !(c.return_cost_ratio > 0.0 && c.return_cost_ratio < 1.0)
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
        if let Some((l, u)) = p.band {
            if l >= y.len() || u >= y.len() {
                return Err(bad("band widths exceed dimension"));
            }
        }
        if let Jacobian::Banded { lower, upper, .. } = p.jacobian {
            if lower >= y.len() || upper >= y.len() || p.band.is_some_and(|b| b != (lower, upper)) {
                return Err(bad("band provider/declaration mismatch"));
            }
        }
    }
    Ok(())
}
