//! JSON runner of the photon chemistry adapter (BASS/rei DAG node P04,
//! `lrs_photon_fv_support/chem.rs`). Usage:
//!
//! `lrs_photon_chem_run --request PATH [--measurement-mode timing|allocation]`
//!
//! The request (one JSON object) declares the model and either an
//! integration arm or a probe:
//!
//! ```text
//! table_path: the Case-A CSV (relative to the working directory or absolute)
//! grid:   {n1, n2, n3, nm}
//! params: {H0, amp, tau_m}
//! initial: {nH0, xHI0, fHeI, fHeII, T0, Dm0, s0_over_H0, photons_per_H, alpha, a2}
//!          | initial_state: [...]   (one of them; probe may omit both)
//! mode: integrate (default) | probe
//! probe:     t, states: [[...]] (default: [initial state]), temperatures,
//!            energies -> per state rhs, partial_t, dense Jacobian (row major,
//!            n <= 600), diagnostics, ledgers; Case-A values and dv/dT per
//!            temperature; Verner sigma per energy; the grid, Ebar, sbar,
//!            Gauss-Legendre rule, constants and the initial state
//! integrate: arm: dense|fixed_dense, t_span, output_times, rtol, atol
//!            (vector, length n), initial_step (default 1e-4 span),
//!            max_attempts (default 1e6), fixed_steps (fixed_dense),
//!            require_in_place_lu (dense; default true)
//! ```
//!
//! The dense arm is `integrate_rodas5p_fast_observed_with_atol`; it refuses
//! (`LU_NOT_IN_PLACE`) a run whose LU is not the in-place zero-skipping LU
//! unless `require_in_place_lu` is false (the driver picks faer for
//! `n > 64` when more than a tenth of the first Jacobian is nonzero). The
//! fixed arm repeats `rodas5p_fast_step`. Wall time is `diagnostic_only`. In
//! allocation mode the integration is counted on its thread with a positive
//! control.
//!
//! Statuses: `OK`; `REFUSED` for an invalid request, declaration, table or
//! shape (`CoreError::InvalidInput`, `CoreError::Dimension`);
//! `NUMERICAL_FAILURE` for every other error (`NonFinite`, including the
//! typed `CASE_A_T_DOMAIN` refusal, `LinearSolve`, ...) and for an
//! unsuccessful run (a `DriverPartial`); `ENVIRONMENT_FAILURE` for usage and
//! request or table I/O.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::time::Instant;
use std::{env, fs};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, Rodas5pFastLu, integrate_rodas5p_fast_observed_with_atol,
    rodas5p_fast_step,
};
use serde_json::{Value, json};

#[path = "lrs_photon_fv_support/mod.rs"]
mod support;
use support::Layout;
use support::chem::{
    self, CASE_A_SHA256, CaseATable, ChemGrid, ChemModel, ChemParams, Diagnostics, InitialData,
    Ledgers, slot,
};

struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations() -> u64 {
    ALLOCATIONS.with(Cell::get)
}

fn bad(msg: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(msg.into())
}

fn f64_of(v: &Value, key: &str) -> CoreResult<f64> {
    v.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| bad(format!("missing number {key}")))
}

fn usize_of(v: &Value, key: &str) -> CoreResult<usize> {
    v.get(key)
        .and_then(Value::as_u64)
        .and_then(|x| usize::try_from(x).ok())
        .ok_or_else(|| bad(format!("missing nonnegative integer {key}")))
}

fn numbers(v: &Value, what: &str) -> CoreResult<Vec<f64>> {
    v.as_array()
        .ok_or_else(|| bad(format!("{what} must be an array")))?
        .iter()
        .map(|x| {
            x.as_f64()
                .ok_or_else(|| bad(format!("non-number in {what}")))
        })
        .collect()
}

fn vec_of(v: &Value, key: &str) -> CoreResult<Vec<f64>> {
    numbers(
        v.get(key)
            .ok_or_else(|| bad(format!("missing array {key}")))?,
        key,
    )
}

fn model_of(req: &Value, table: CaseATable) -> CoreResult<ChemModel> {
    let g = req.get("grid").ok_or_else(|| bad("missing grid"))?;
    let grid = ChemGrid {
        n1: usize_of(g, "n1")?,
        n2: usize_of(g, "n2")?,
        n3: usize_of(g, "n3")?,
        nm: usize_of(g, "nm")?,
    };
    let p = req.get("params").ok_or_else(|| bad("missing params"))?;
    let params = ChemParams {
        h0: f64_of(p, "H0")?,
        amp: f64_of(p, "amp")?,
        tau_m: f64_of(p, "tau_m")?,
    };
    ChemModel::new(table, grid, params)
}

/// The initial state, from `initial_state` or the §9 `initial` data.
fn initial_of(req: &Value, model: &ChemModel) -> CoreResult<Option<Vec<f64>>> {
    match (req.get("initial_state"), req.get("initial")) {
        (Some(_), Some(_)) => Err(bad("give initial or initial_state, not both")),
        (Some(s), None) => {
            let y = numbers(s, "initial_state")?;
            if y.len() != model.n() {
                return Err(CoreError::Dimension(format!(
                    "initial_state has {} entries, n = {}",
                    y.len(),
                    model.n()
                )));
            }
            Ok(Some(y))
        }
        (None, Some(i)) => {
            let init = InitialData {
                n_h0: f64_of(i, "nH0")?,
                x_hi0: f64_of(i, "xHI0")?,
                f_hei: f64_of(i, "fHeI")?,
                f_heii: f64_of(i, "fHeII")?,
                temperature0: f64_of(i, "T0")?,
                d_m0: f64_of(i, "Dm0")?,
                s0_over_h0: f64_of(i, "s0_over_H0")?,
                photons_per_h: f64_of(i, "photons_per_H")?,
                alpha: f64_of(i, "alpha")?,
                a2: f64_of(i, "a2")?,
            };
            model.initial_state(&init).map(Some)
        }
        (None, None) => Ok(None),
    }
}

fn error_kind(e: &CoreError) -> (&'static str, &str) {
    match e {
        CoreError::Dimension(m) => ("Dimension", m),
        CoreError::InvalidInput(m) => ("InvalidInput", m),
        CoreError::NonFinite(m) => ("NonFinite", m),
        CoreError::LinearSolve(m) => ("LinearSolve", m),
        CoreError::NonlinearSolve(m) => ("NonlinearSolve", m),
        CoreError::Coefficients(m) => ("Coefficients", m),
    }
}

/// Typed refusals of the request (declaration, shape) versus numerical
/// failures of a well-posed evaluation or run.
fn status_of(e: &CoreError) -> &'static str {
    match e {
        CoreError::InvalidInput(_) | CoreError::Dimension(_) => "REFUSED",
        _ => "NUMERICAL_FAILURE",
    }
}

fn error_value(e: &CoreError) -> Value {
    let (kind, message) = error_kind(e);
    json!({"status": status_of(e), "error_kind": kind, "message": message})
}

fn ledgers_value(l: &Ledgers) -> Value {
    json!({"number_ledger": l.number, "energy_ledger": l.energy, "H_ledger": l.hydrogen,
        "He_ledger": l.helium, "photon_ledger": l.photon})
}

fn diagnostics_value(d: &Diagnostics) -> Value {
    json!({"n_gamma": d.moments.n_gamma, "E_gamma": d.moments.e_gamma,
        "D_gamma": d.moments.d_gamma, "T": d.temperature, "x_HII": d.x_hii,
        "x_HeII": d.x_heii, "x_HeIII": d.x_heiii, "s_over_H": d.s_over_h,
        "ledgers": ledgers_value(&d.ledgers)})
}

fn static_value(model: &ChemModel, initial: &Option<Vec<f64>>) -> Value {
    let l: &Layout = model.layout();
    let rule = chem::gauss_legendre_16();
    let slots: serde_json::Map<String, Value> = slot::NAMES
        .iter()
        .enumerate()
        .map(|(k, name)| (name.to_string(), Value::from(model.slot(k))))
        .collect();
    json!({
        "n": model.n(), "n_ph": model.n_ph, "nq": l.nq, "nm": l.nm,
        "q_edges": l.q_edges, "mu_edges": l.mu_edges, "dq": l.dq, "dm": l.dm, "p2": l.p2,
        "Ebar": model.ebar, "photon_energy_weights": model.weights, "sbar": model.sbar,
        "slots": slots, "L_lo": Layout::LO, "L_hi": l.hi(),
        "cells": (0..l.nq).map(|i| (0..l.nm).map(|j| l.cell(i, j)).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        "absorbed": (0..l.nq).map(|i| l.absorbed(i)).collect::<Vec<_>>(),
        "gauss_legendre": {"nodes": rule.nodes, "weights": rule.weights},
        "constants": {"C": chem::C, "KB": chem::KB, "EV_ERG": chem::EV_ERG, "BIND": chem::BIND,
            "SIGMA_T": chem::SIGMA_T, "G": chem::G, "K_D": chem::K_D, "K_D_amp": model.k_d_amp,
            "FIT": chem::FIT, "t0": model.age0},
        "table": {"sha256": model.table.sha256, "sha256_matches_spec":
            model.table.sha256 == CASE_A_SHA256, "T_min": model.table.t_min(),
            "T_max": model.table.t_max()},
        "initial_state": initial,
    })
}

fn rows(m: &DenseMatrix) -> Value {
    Value::from(
        (0..m.nrows())
            .map(|r| m.row(r).to_vec())
            .collect::<Vec<_>>(),
    )
}

/// One state of the probe: every callback and the diagnostics, each with
/// its own status.
fn probe_state(model: &ChemModel, t: f64, y: &[f64]) -> Value {
    let n = model.n();
    let mut status = "OK";
    let mut record = |r: CoreResult<Value>| match r {
        Ok(v) => v,
        Err(e) => {
            if status == "OK" {
                status = status_of(&e);
            }
            error_value(&e)
        }
    };
    let mut out = vec![0.0; n];
    let rhs = record(model.rhs(t, y, &mut out).map(|_| Value::from(out.clone())));
    let ft = record(
        model
            .partial_t(t, y, &mut out)
            .map(|_| Value::from(out.clone())),
    );
    let jacobian = if n <= 600 {
        let mut m = DenseMatrix::zeros(n, n);
        record(model.dense_fill(t, y, &mut m).map(|_| rows(&m)))
    } else {
        Value::Null
    };
    let diagnostics = record(model.diagnostics(t, y).map(|d| diagnostics_value(&d)));
    json!({"status": status, "rhs": rhs, "partial_t": ft, "jacobian": jacobian,
        "diagnostics": diagnostics})
}

fn probe(req: &Value, model: &ChemModel, initial: &Option<Vec<f64>>) -> CoreResult<Value> {
    let t = f64_of(req, "t")?;
    let states: Vec<Vec<f64>> = match req.get("states") {
        Some(s) => s
            .as_array()
            .ok_or_else(|| bad("states must be an array"))?
            .iter()
            .map(|y| numbers(y, "state"))
            .collect::<CoreResult<_>>()?,
        None => initial.iter().cloned().collect(),
    };
    let temperatures = match req.get("temperatures") {
        Some(v) => numbers(v, "temperatures")?,
        None => Vec::new(),
    };
    let energies = match req.get("energies") {
        Some(v) => numbers(v, "energies")?,
        None => Vec::new(),
    };
    let refusals_before = model.domain_refusals();
    let per_state: Vec<Value> = states.iter().map(|y| probe_state(model, t, y)).collect();
    let case_a: Vec<Value> = temperatures
        .iter()
        .map(|&temperature| match model.table.eval(temperature) {
            Ok((v, dv)) => json!({"T": temperature, "status": "OK", "values": v, "dvdT": dv}),
            Err(e) => {
                let mut value = error_value(&e);
                value["T"] = Value::from(temperature);
                value
            }
        })
        .collect();
    let sigma: Vec<[f64; 3]> = energies
        .iter()
        .map(|&e| [0, 1, 2].map(|s| chem::verner_sigma(s, e)))
        .collect();
    let bg = model.background(t);
    Ok(
        json!({"status": "OK", "t": t, "grid": static_value(model, initial),
        "background": {"H": bg.h, "dH_dt": bg.dh, "a_inv3": bg.a_inv3,
            "da_inv3_dt": bg.da_inv3, "ln_a": model.ln_a(t)},
        "states": per_state, "case_a": case_a, "verner_sigma": sigma,
        "domain_refusals": model.domain_refusals() - refusals_before}),
    )
}

struct Run {
    times: Vec<f64>,
    states: Vec<Vec<f64>>,
    success: bool,
    message: String,
    counters: WorkCounters,
    details: Value,
    /// The last accepted state of a partial run.
    partial_y: Option<(f64, Vec<f64>)>,
}

fn integrate(req: &Value, model: Arc<ChemModel>, y0: &[f64]) -> CoreResult<Run> {
    let n = model.n();
    let arm = req
        .get("arm")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("missing arm"))?;
    let span = vec_of(req, "t_span")?;
    if span.len() != 2 || y0.len() != n {
        return Err(bad(
            "t_span must have 2 entries and the initial state n entries",
        ));
    }
    let (t0, tf) = (span[0], span[1]);
    let rtol = f64_of(req, "rtol")?;
    let atol = vec_of(req, "atol")?;
    if atol.len() != n {
        return Err(bad("atol must be a vector of length n"));
    }
    let atol_min = atol.iter().copied().fold(f64::INFINITY, f64::min);
    let output = OutputSchedule::new(vec_of(req, "output_times")?)?;
    let initial_step = req
        .get("initial_step")
        .and_then(Value::as_f64)
        .unwrap_or(1.0e-4 * (tf - t0));
    let max_attempts = req
        .get("max_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(1_000_000) as usize;
    let adaptive = AdaptiveStepConfig {
        atol: atol_min,
        rtol,
        initial_step,
        max_step: tf - t0,
        max_attempts,
        ..AdaptiveStepConfig::default()
    };
    let problem = chem::problem(model.clone())?;
    match arm {
        "dense" => {
            let require = req
                .get("require_in_place_lu")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let (fast, partial) = integrate_rodas5p_fast_observed_with_atol(
                &problem,
                (t0, tf),
                y0,
                &adaptive,
                &output,
                &atol,
            )?;
            if require && fast.lu != Rodas5pFastLu::InPlaceZeroSkipping {
                return Err(bad(format!(
                    "LU_NOT_IN_PLACE: the dense fast driver chose {:?} for n = {n}",
                    fast.lu
                )));
            }
            Ok(Run {
                details: json!({"attempts":fast.attempts,"accepted_steps":fast.accepted_steps,
                    "rejected_steps":fast.rejected_steps,"jacobian_reuses":fast.jacobian_reuses,
                    "lu":format!("{:?}",fast.lu),
                    "lu_in_place_zero_skipping": fast.lu == Rodas5pFastLu::InPlaceZeroSkipping,
                    "driver":fast.driver,
                    "internal_steps":fast.observed.internal_steps,
                    "output_clipped_steps":fast.observed.output_clipped_steps,
                    "partial":partial}),
                partial_y: partial.map(|p| (p.t, p.y)),
                times: fast.observed.t,
                states: fast.observed.y,
                success: fast.observed.success,
                message: fast.observed.message,
                counters: fast.observed.counters,
            })
        }
        "fixed_dense" => {
            let fixed_steps = req.get("fixed_steps").and_then(Value::as_u64).unwrap_or(0) as usize;
            if fixed_steps == 0 {
                return Err(bad("fixed_dense needs fixed_steps"));
            }
            let h = (tf - t0) / fixed_steps as f64;
            let mut counters = WorkCounters::default();
            let mut y = y0.to_vec();
            let mut lu = String::new();
            for k in 0..fixed_steps {
                let t = t0 + k as f64 * h;
                let (y_new, _, kind) =
                    rodas5p_fast_step(&problem, t, &y, h, atol_min, rtol, &mut counters)?;
                y = y_new;
                lu = format!("{kind:?}");
            }
            Ok(Run {
                details: json!({"fixed_steps":fixed_steps,"h":h,"lu":lu}),
                partial_y: None,
                times: vec![t0, tf],
                states: vec![y0.to_vec(), y],
                success: true,
                message: "fixed steps".into(),
                counters,
            })
        }
        _ => Err(bad("unknown arm (dense|fixed_dense)")),
    }
}

/// Diagnostics of every output state and the drift of each ledger from the
/// initial state.
fn summary(model: &ChemModel, y0: &[f64], run: &Run) -> CoreResult<Value> {
    let initial = model.ledgers(y0).as_array();
    let mut max_drift = [0.0_f64; 5];
    let mut diagnostics = Vec::with_capacity(run.states.len());
    for (t, y) in run.times.iter().zip(&run.states) {
        let d = model.diagnostics(*t, y)?;
        for (k, value) in d.ledgers.as_array().iter().enumerate() {
            max_drift[k] = max_drift[k].max((value - initial[k]).abs());
        }
        diagnostics.push(diagnostics_value(&d));
    }
    let last = match (&run.partial_y, run.states.last(), run.times.last()) {
        (Some((t, y)), _, _) => Some((*t, y.as_slice())),
        (None, Some(y), Some(t)) => Some((*t, y.as_slice())),
        _ => None,
    };
    let last = match last {
        Some((t, y)) => {
            let d = model.diagnostics(t, y)?;
            json!({"t": t, "T": d.temperature, "ledgers": ledgers_value(&d.ledgers)})
        }
        None => Value::Null,
    };
    let drift: serde_json::Map<String, Value> = Ledgers::NAMES
        .iter()
        .enumerate()
        .map(|(k, name)| {
            (
                name.to_string(),
                json!({"initial": initial[k], "max_abs_drift": max_drift[k],
                    "max_rel_drift": max_drift[k] / initial[k].abs()}),
            )
        })
        .collect();
    Ok(json!({"diagnostics": diagnostics, "ledger_drift": drift, "last": last}))
}

fn result(status: &str, reason: &str) -> Value {
    json!({"status":status,"reason":reason,"times":[],"states":[],"counters":null,
        "timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"}})
}

fn failure(e: &CoreError) -> Value {
    let mut value = result(status_of(e), &e.to_string());
    value["error_kind"] = Value::from(error_kind(e).0);
    value
}

fn run() -> Value {
    let args: Vec<String> = env::args().collect();
    if !(args.len() == 3 || (args.len() == 5 && args[3] == "--measurement-mode"))
        || args[1] != "--request"
    {
        return result(
            "ENVIRONMENT_FAILURE",
            "usage: lrs_photon_chem_run --request PATH [--measurement-mode timing|allocation]",
        );
    }
    let mode = if args.len() == 5 {
        args[4].as_str()
    } else {
        "timing"
    };
    if mode != "timing" && mode != "allocation" {
        return result("ENVIRONMENT_FAILURE", "invalid measurement mode");
    }
    let req: Value = match fs::read_to_string(&args[2])
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(v) => v,
        None => return result("ENVIRONMENT_FAILURE", "request read/parse failed"),
    };
    let Some(table_path) = req.get("table_path").and_then(Value::as_str) else {
        return failure(&bad("missing table_path"));
    };
    let text = match fs::read_to_string(table_path) {
        Ok(text) => text,
        Err(e) => {
            return result(
                "ENVIRONMENT_FAILURE",
                &format!("table read failed: {table_path}: {e}"),
            );
        }
    };
    let model = match CaseATable::from_csv_str(&text).and_then(|table| model_of(&req, table)) {
        Ok(m) => Arc::new(m),
        Err(e) => return failure(&e),
    };
    let initial = match initial_of(&req, &model) {
        Ok(y) => y,
        Err(e) => return failure(&e),
    };
    if req.get("mode").and_then(Value::as_str) == Some("probe") {
        return probe(&req, &model, &initial).unwrap_or_else(|e| failure(&e));
    }
    let Some(y0) = initial else {
        return failure(&bad("integrate needs initial or initial_state"));
    };
    // One warm run, then the measured run (timing or allocation count).
    if let Err(e) = integrate(&req, model.clone(), &y0) {
        return failure(&e);
    }
    let refusals_before = model.domain_refusals();
    let before = allocations();
    let start = Instant::now();
    let run = integrate(&req, model.clone(), &y0);
    let elapsed = start.elapsed().as_nanos() as u64;
    let counted = allocations() - before;
    let control_before = allocations();
    let positive = vec![1_u8; 64];
    std::hint::black_box(&positive);
    let control = allocations() - control_before;
    let refusals = model.domain_refusals() - refusals_before;
    match run {
        Ok(r) => {
            let summary = match summary(&model, &y0, &r) {
                Ok(s) => s,
                Err(e) => return failure(&e),
            };
            json!({"status": if r.success {"OK"} else {"NUMERICAL_FAILURE"}, "reason": r.message,
                "n": model.n(), "times": r.times, "states": r.states, "counters": r.counters,
                "details": r.details, "summary": summary,
                "domain_refusals": refusals, "domain_refusals_total": model.domain_refusals(),
                "table_sha256": model.table.sha256,
                "measurement_mode": mode,
                "timing": {"elapsed_ns": elapsed, "timing_authority": "diagnostic_only",
                    "timing_scope": "second in-process integrate call incl. problem build and output"},
                "allocations": if mode == "allocation" {
                    json!({"integrate": counted, "positive_control": control, "calibrated": control > 0,
                        "scope": "second integrate call on the calling thread incl. problem build/output"})
                } else { Value::Null }})
        }
        Err(e) => failure(&e),
    }
}

fn main() {
    println!("{}", run());
}
