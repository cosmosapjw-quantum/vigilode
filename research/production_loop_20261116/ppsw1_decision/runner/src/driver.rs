//! The step loop of `vigilode::PeerOde::solve` (crates/vigilode/src/selection/peer.rs, sha256
//! 8053325f939da490ee043e0f3b097f17da151d05bad0ca18bcfaf6614e75fc17) over the research copy.
//!
//! Same arithmetic on every success path: the h0 rule (`h_max = h0_rule_h_max min endpoint
//! cap`), the starter with step-doubling control (up to 40 halvings), the `try_step` loop with
//! `h = hh * next_ratio`, the endpoint clip and the exact landing. The callback latch of the
//! adapter is left out: the declared problems have no failing callbacks, and a failure here is
//! reported by its integrator message. What differs from the adapter is the configuration
//! (`ppsw1::Config`: cap and tableau cache) and that the execution context is borrowed.

use rodas5p_core::WorkCounters;

use crate::ppsw1;
use crate::{OdeProblem, ParallelExecution};

/// `PEER_STARTER_MAX_HALVINGS` of the adapter.
pub const STARTER_MAX_HALVINGS: usize = 40;

/// The options of `vigilode::PeerSolveOptions` that the loop reads (threads are the pool).
#[derive(Clone, Copy, Debug)]
pub struct PeerOptions {
    pub rtol: f64,
    pub atol: f64,
    pub h0_rule_h_max: f64,
    pub h0: Option<f64>,
    pub max_attempts: usize,
}

/// `vigilode::PeerStarterRecord`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarterRecord {
    pub h0_requested: f64,
    pub halvings: usize,
    pub estimate: f64,
    pub covers_span: bool,
}

/// One attempt of the step loop (diagnostics of the ratio sequence).
#[derive(Clone, Copy, Debug)]
pub struct AttemptTrace {
    pub sigma: f64,
    pub accepted: bool,
    pub error_norm: f64,
}

/// What a run returns: the fields of `vigilode::PeerSolution` the identity check compares, plus
/// the attempt trace and the tableau-cache counters.
#[derive(Clone, Debug)]
pub struct PeerRun {
    pub t: Vec<f64>,
    pub y: Vec<Vec<f64>>,
    pub success: bool,
    pub failure: Option<String>,
    pub h0: f64,
    pub starter: StarterRecord,
    pub startup_work: WorkCounters,
    pub step_work: WorkCounters,
    pub attempts: usize,
    pub trace: Vec<AttemptTrace>,
    pub cache: Option<ppsw1::TableauCacheStats>,
}

/// `starter_estimate` of the adapter (unchanged arithmetic).
fn starter_estimate(
    full: &ppsw1::State,
    first: &ppsw1::State,
    second: &ppsw1::State,
    tol: &ppsw1::Tolerances,
) -> f64 {
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

/// Integrate `ode` over `span` from `y0` with the research copy (the PeerOde loop).
pub fn solve(
    ode: &OdeProblem,
    span: (f64, f64),
    y0: &[f64],
    options: &PeerOptions,
    config: ppsw1::Config,
    exec: &ParallelExecution,
) -> PeerRun {
    let (t0, t_end) = span;
    let mut startup = WorkCounters::default();
    let mut out = PeerRun {
        t: Vec::new(),
        y: Vec::new(),
        success: false,
        failure: None,
        h0: f64::NAN,
        starter: StarterRecord {
            h0_requested: f64::NAN,
            halvings: 0,
            estimate: f64::NAN,
            covers_span: false,
        },
        startup_work: startup,
        step_work: WorkCounters::default(),
        attempts: 0,
        trace: Vec::new(),
        cache: None,
    };
    let tol = ppsw1::Tolerances {
        atol: options.atol,
        rtol: options.rtol,
    };

    let endpoint_cap = 0.5 * (t_end - t0);
    let mut h0 = match options.h0 {
        Some(h) => h.min(endpoint_cap),
        None => {
            let rule = ppsw1::jacobian_norm_bound(ode, t0, y0, &mut startup)
                .and_then(|b| ppsw1::select_h0(b, options.h0_rule_h_max.min(endpoint_cap)));
            match rule {
                Ok(h) => h,
                Err(e) => {
                    out.failure = Some(format!("h0 rule: {e}"));
                    out.startup_work = startup;
                    return out;
                }
            }
        }
    };
    out.h0 = h0;
    out.starter.h0_requested = h0;

    let mut state = match ppsw1::start(ode, t0, y0, h0, &mut startup) {
        Ok(state) => state,
        Err(e) => {
            out.failure = Some(format!("starter: {e}"));
            out.startup_work = startup;
            return out;
        }
    };
    let covers_span = loop {
        let half = 0.5 * h0;
        let first = match ppsw1::start(ode, t0, y0, half, &mut startup) {
            Ok(state) => state,
            Err(e) => {
                out.failure = Some(format!("starter: {e}"));
                out.startup_work = startup;
                return out;
            }
        };
        let second = match ppsw1::start(ode, first.t, &first.y[5], half, &mut startup) {
            Ok(state) => state,
            Err(e) => {
                out.failure = Some(format!("starter: {e}"));
                out.startup_work = startup;
                return out;
            }
        };
        let estimate = starter_estimate(&state, &first, &second, &tol);
        out.starter.estimate = estimate;
        if ppsw1::accepts(estimate) {
            break (out.starter.halvings == 0 && h0 == endpoint_cap) || state.t >= t_end;
        }
        if out.starter.halvings == STARTER_MAX_HALVINGS {
            out.failure = Some(format!(
                "starter error estimate {estimate:e} above 1 after {} halvings",
                out.starter.halvings
            ));
            out.startup_work = startup;
            return out;
        }
        out.starter.halvings += 1;
        h0 = half;
        out.h0 = h0;
        state = first;
    };
    out.startup_work = startup;
    out.starter.covers_span = covers_span;
    out.t.push(if covers_span { t_end } else { state.t });
    out.y.push(state.y[5].clone());
    if covers_span {
        out.success = true;
        return out;
    }

    let mut session = match ppsw1::Session::new(ode, state, config, exec) {
        Ok(s) => s,
        Err(e) => {
            out.failure = Some(format!("session: {e}"));
            return out;
        }
    };
    let mut h = h0;
    let mut attempts = 0usize;
    let mut landed = false;
    let mut failure = None;
    while !landed {
        if attempts >= options.max_attempts {
            failure = Some(format!(
                "attempt budget exhausted ({} attempts)",
                options.max_attempts
            ));
            break;
        }
        attempts += 1;
        let t = session.state().t;
        let remaining = t_end - t;
        let last = h >= remaining;
        let hh = h.min(remaining);
        let sigma = hh / session.state().h_prev;
        match session.try_step(hh, &tol) {
            Ok(ppsw1::Decision::Accepted {
                next_ratio,
                error_norm,
                ..
            }) => {
                out.trace.push(AttemptTrace {
                    sigma,
                    accepted: true,
                    error_norm,
                });
                h = hh * next_ratio;
                let t_new = session.state().t;
                landed = last || t_new >= t_end;
                out.t.push(if landed { t_end } else { t_new });
                out.y.push(session.state().y[5].clone());
            }
            Ok(ppsw1::Decision::Rejected {
                next_ratio,
                error_norm,
                ..
            }) => {
                out.trace.push(AttemptTrace {
                    sigma,
                    accepted: false,
                    error_norm,
                });
                h = hh * next_ratio;
            }
            Err(e) => {
                failure = Some(format!("step failed at t = {t:e}: {e}"));
                break;
            }
        }
    }
    out.attempts = attempts;
    out.step_work = session.counters();
    out.cache = session.tableau_cache_stats();
    out.success = landed && failure.is_none();
    out.failure = failure;
    out
}
