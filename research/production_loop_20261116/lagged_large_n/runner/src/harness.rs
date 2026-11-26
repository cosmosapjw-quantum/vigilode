//! The arms of node V09 and one integration of an arm (`../../PREDECLARED.json`, "arms").
//!
//! * `F`: the protected fast dense driver, `integrate_rodas5p_fast_observed_with_partial`
//!   (default options; it picks its LU from the first Jacobian).
//! * `P`: `integrate_rodas5p_lagged_observed` with `Rodas5pLaggedOptions::default()` (Richardson,
//!   no staleness rule) on the problem without a JVP: the configuration of the unified
//!   frontend's method id `rodas5p-lagged-richardson-v1` (provider form Dense, no JVP).
//! * `S`: the lagged driver with `StageSolver::Gmres` and `StalenessRule::Bounded`, no JVP: the
//!   gated arm of the last kill gate of the lineage (node b1c-staleness).
//! * `PJ`, `SJ`: `P` and `S` on the same problem with its analytic JVP attached (the lagged
//!   driver then forms `W x` with the JVP instead of the dense matvec).
//! * `X`: the lagged driver with `refresh_every_attempt` (Richardson, no staleness rule, no
//!   JVP): the fast driver's arithmetic in the fast driver's order; the harness identity check.

use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rodas5p_core::{CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, JacobianIntoFn, OdeProblem, OutputSchedule, Rodas5pLaggedOptions,
    StageSolver, StalenessRule, integrate_rodas5p_fast_observed_with_partial,
    integrate_rodas5p_lagged_observed,
};
use serde_json::{Value, json};

use crate::problems::{JvpFn, ProblemDef, RhsFn};

/// Relative tolerances of the node; `atol = rtol` for every family.
pub const RTOLS: [f64; 4] = [1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7];
/// First step of every run.
pub const INITIAL_STEP: f64 = 1.0e-6;
/// Smallest step of every run.
pub const MIN_STEP: f64 = 1.0e-14;
/// Attempt budget of every run (never reached in a completed run).
pub const MAX_ATTEMPTS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    F,
    P,
    S,
    PJ,
    SJ,
    X,
}

impl Arm {
    pub const ALL: [Arm; 6] = [Arm::F, Arm::P, Arm::S, Arm::PJ, Arm::SJ, Arm::X];

    pub fn id(self) -> &'static str {
        match self {
            Arm::F => "F",
            Arm::P => "P",
            Arm::S => "S",
            Arm::PJ => "PJ",
            Arm::SJ => "SJ",
            Arm::X => "X",
        }
    }

    pub fn parse(text: &str) -> Option<Arm> {
        Arm::ALL.into_iter().find(|arm| arm.id() == text)
    }

    /// Whether the problem of this arm carries the analytic JVP.
    pub fn uses_jvp(self) -> bool {
        matches!(self, Arm::PJ | Arm::SJ)
    }

    /// The lagged driver's options, `None` for the fast arm.
    pub fn lagged_options(self) -> Option<Rodas5pLaggedOptions> {
        match self {
            Arm::F => None,
            Arm::P | Arm::PJ => Some(Rodas5pLaggedOptions::default()),
            Arm::S | Arm::SJ => Some(Rodas5pLaggedOptions {
                refresh_every_attempt: false,
                stage_solver: StageSolver::Gmres,
                staleness: StalenessRule::Bounded,
            }),
            Arm::X => Some(Rodas5pLaggedOptions {
                refresh_every_attempt: true,
                ..Rodas5pLaggedOptions::default()
            }),
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Arm::F => "protected fast dense driver, default options",
            Arm::P => {
                "lagged driver, Rodas5pLaggedOptions::default() (Richardson, StalenessRule::Off), no JVP (unified frontend configuration)"
            }
            Arm::S => {
                "lagged driver, StageSolver::Gmres + StalenessRule::Bounded, no JVP (gated arm of b1c-staleness)"
            }
            Arm::PJ => "arm P with the analytic JVP attached",
            Arm::SJ => "arm S with the analytic JVP attached",
            Arm::X => {
                "lagged driver with refresh_every_attempt (Richardson, Off), no JVP: identity check against F"
            }
        }
    }
}

/// The callbacks an `OdeProblem` is built from; the trace binary wraps them.
#[derive(Clone)]
pub struct Callbacks {
    pub rhs: RhsFn,
    pub jacobian_into: JacobianIntoFn,
    pub jvp: Option<JvpFn>,
}

impl Callbacks {
    /// The problem's own callbacks: right-hand side, dense fill (pattern only) and, when
    /// `with_jvp`, the analytic JVP.
    pub fn plain(def: &ProblemDef, with_jvp: bool) -> Self {
        let fill = def.dense_fill.clone();
        Self {
            rhs: def.rhs.clone(),
            jacobian_into: Arc::new(move |_t, y, out: &mut DenseMatrix| {
                fill(y, out);
                Ok(())
            }),
            jvp: with_jvp.then(|| def.jvp.clone()),
        }
    }
}

pub fn build_problem(def: &ProblemDef, callbacks: &Callbacks, arm: Arm) -> CoreResult<OdeProblem> {
    let n = def.n;
    let allocating = callbacks.jacobian_into.clone();
    Ok(OdeProblem::new(
        format!("v09 {} {} [{}]", def.family, def.size, arm.id()),
        n,
        callbacks.rhs.clone(),
        None,
        Some(Arc::new(move |t: f64, y: &[f64]| {
            let mut matrix = DenseMatrix::zeros(n, n);
            allocating(t, y, &mut matrix)?;
            Ok(matrix)
        })),
        callbacks.jvp.clone(),
        None,
        true,
        None,
        None,
    )?
    .with_jacobian_into(callbacks.jacobian_into.clone()))
}

pub fn adaptive(def: &ProblemDef, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol,
        rtol,
        initial_step: INITIAL_STEP,
        min_step: MIN_STEP,
        max_step: def.t_span.1 - def.t_span.0,
        max_attempts: MAX_ATTEMPTS,
        ..AdaptiveStepConfig::default()
    }
}

/// The record of one integration.
pub struct RunRecord {
    pub wall_ns: u64,
    pub driver_id: String,
    pub lu: String,
    pub success: bool,
    pub message: String,
    pub t_end: f64,
    pub y_end: Vec<f64>,
    pub end_state_source: &'static str,
    pub attempts: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub jacobian_reuses: usize,
    pub internal_steps: usize,
    pub counters: WorkCounters,
    pub lagged_work: Value,
    pub staleness_work: Value,
}

fn elapsed_ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// One integration of `arm`; the wall clock covers the driver call only (it builds and drops
/// its workspace inside the call).
#[inline(never)]
pub fn run_once(
    arm: Arm,
    problem: &OdeProblem,
    def: &ProblemDef,
    rtol: f64,
) -> CoreResult<RunRecord> {
    let config = adaptive(def, rtol);
    let output = OutputSchedule::new(vec![def.t_span.0, def.t_span.1])?;
    match arm.lagged_options() {
        None => {
            let start = Instant::now();
            let (result, partial) = integrate_rodas5p_fast_observed_with_partial(
                problem, def.t_span, &def.y0, &config, &output,
            )?;
            let wall_ns = elapsed_ns(start);
            let observed = &result.observed;
            let (t_end, y_end, source) = if observed.success {
                (
                    *observed.t.last().expect("output"),
                    observed.y.last().expect("output").clone(),
                    "output",
                )
            } else if let Some(partial) = &partial {
                (partial.t, partial.y.clone(), "partial")
            } else {
                (f64::NAN, Vec::new(), "none")
            };
            Ok(RunRecord {
                wall_ns,
                driver_id: result.driver.into(),
                lu: format!("{:?}", result.lu),
                success: observed.success,
                message: observed.message.clone(),
                t_end,
                y_end,
                end_state_source: source,
                attempts: result.attempts,
                accepted: result.accepted_steps,
                rejected: result.rejected_steps,
                jacobian_reuses: result.jacobian_reuses,
                internal_steps: observed.internal_steps,
                counters: observed.counters,
                lagged_work: Value::Null,
                staleness_work: Value::Null,
            })
        }
        Some(options) => {
            let start = Instant::now();
            let result = integrate_rodas5p_lagged_observed(
                problem, def.t_span, &def.y0, &config, &output, options,
            )?;
            let wall_ns = elapsed_ns(start);
            let observed = &result.fast.observed;
            let (t_end, y_end, source) = if observed.success {
                (
                    *observed.t.last().expect("output"),
                    observed.y.last().expect("output").clone(),
                    "output",
                )
            } else {
                // The lagged driver keeps no partial record: the last output sample reached.
                (
                    *observed.t.last().unwrap_or(&f64::NAN),
                    observed.y.last().cloned().unwrap_or_default(),
                    "last-output-sample",
                )
            };
            Ok(RunRecord {
                wall_ns,
                driver_id: result.fast.driver.into(),
                lu: format!("{:?}", result.fast.lu),
                success: observed.success,
                message: observed.message.clone(),
                t_end,
                y_end,
                end_state_source: source,
                attempts: result.fast.attempts,
                accepted: result.fast.accepted_steps,
                rejected: result.fast.rejected_steps,
                jacobian_reuses: result.fast.jacobian_reuses,
                internal_steps: observed.internal_steps,
                counters: observed.counters,
                lagged_work: serde_json::to_value(result.work).expect("lagged work"),
                staleness_work: serde_json::to_value(result.staleness).expect("staleness work"),
            })
        }
    }
}

pub fn hex(values: &[f64]) -> Vec<String> {
    values
        .iter()
        .map(|v| format!("{:016x}", v.to_bits()))
        .collect()
}

/// FNV-1a over the bit patterns (a compact identity of an end state).
pub fn fnv64(values: &[f64]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// The fields of a record that must agree between repetitions of one integration.
pub fn identity_key(record: &RunRecord) -> (String, usize, usize, usize, String, u64) {
    (
        fnv64(&record.y_end),
        record.attempts,
        record.accepted,
        record.rejected,
        format!("{:016x}", record.t_end.to_bits()),
        record.counters.direct_factorizations,
    )
}

pub fn record_json(record: &RunRecord, with_state: bool) -> Value {
    let mut value = json!({
        "driver_id": record.driver_id,
        "lu": record.lu,
        "success": record.success,
        "message": record.message,
        "t_end": record.t_end,
        "t_end_hex": format!("{:016x}", record.t_end.to_bits()),
        "end_state_source": record.end_state_source,
        "y_end_fnv64": fnv64(&record.y_end),
        "y_end_finite": record.y_end.iter().all(|v| v.is_finite()),
        "attempts": record.attempts,
        "accepted_steps": record.accepted,
        "rejected_steps": record.rejected,
        "jacobian_reuses": record.jacobian_reuses,
        "internal_steps": record.internal_steps,
        "counters": serde_json::to_value(record.counters).expect("counters"),
        "lagged_work": record.lagged_work,
        "staleness_work": record.staleness_work,
    });
    if with_state {
        value["y_end_hex"] = json!(hex(&record.y_end));
    }
    value
}

pub fn loadavg() -> Value {
    let text = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let values: Vec<f64> = text
        .split_whitespace()
        .take(3)
        .filter_map(|v| v.parse().ok())
        .collect();
    json!(values)
}

pub fn cpus_allowed() -> String {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

pub fn unix_time() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

pub fn problem_json(def: &ProblemDef) -> Value {
    json!({
        "family": def.family,
        "structure": def.structure,
        "size_parameter": def.size,
        "n": def.n,
        "t_span": [def.t_span.0, def.t_span.1],
        "description": def.description,
    })
}
