//! JSON runner of the fixed-grid full-sphere photon transport adapter
//! (BASS/rei DAG node P06). Usage:
//!
//! `sphere_photon_fv_run --request PATH [--measurement-mode timing|allocation]`
//!
//! The request (one JSON object) declares the model and either an
//! integration arm or a probe:
//!
//! ```text
//! layout:   {q_edges, mu_edges, phi_edges | phi_uniform: {n, offset},
//!            coordinate: physical|comoving, boundary: outflow|closed}
//! geometry: {kind: constant, h1, h2, h3} | {kind: power_law, hh1, hh2, hh3}
//! gamma, absorber: null | {k0, q_chi}, drop_partial_t (default false)
//! mode: integrate (default) | probe
//! integrate: arm: dense|banded|fixed_dense|fixed_banded,
//!            initial_state, t_span, output_times, rtol, atol (vector),
//!            initial_step (default 1e-4 span), max_attempts (default 1e6),
//!            fixed_steps (fixed arms)
//! probe:     t, states: [[...]] -> rhs, partial_t, jvp (directions = states
//!            at the state states[0]), kbar, dense (n <= 600), band,
//!            grid_constants (Python float.hex, the golden fixture's names),
//!            drift and face_velocities at t (numbers and hex);
//!            group_rates (optional, one per energy group) -> rhs_group_rates
//!            (the oracle's grid_state_rhs with explicit kbar)
//! ```
//!
//! Wall time is `diagnostic_only`. In allocation mode the integration is
//! counted on its thread with a positive control.
//!
//! Statuses: `OK`; `REFUSED` for an invalid request, declaration or shape
//! (`CoreError::InvalidInput`, `CoreError::Dimension`, with the adapter's
//! refusal code in `code`); `NUMERICAL_FAILURE` for every other error
//! (`NonFinite`, `LinearSolve`, ...) and for an unsuccessful run (step
//! limits); `ENVIRONMENT_FAILURE` for usage and request I/O.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::time::Instant;
use std::{env, fs};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_observed_with_atol, rodas5p_fast_step,
};
use serde_json::{Value, json};

#[path = "sphere_photon_fv_support/mod.rs"]
mod support;
use support::{
    Absorber, Boundary, Coordinate, Geometry3, Layout3, Model3, float_hex, refusal_code,
    uniform_phi_edges,
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
    CoreError::InvalidInput(format!("request.invalid: {}", msg.into()))
}

fn f64_of(v: &Value, key: &str) -> CoreResult<f64> {
    v.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| bad(format!("missing number {key}")))
}

fn vec_of(v: &Value, key: &str) -> CoreResult<Vec<f64>> {
    v.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| bad(format!("missing array {key}")))?
        .iter()
        .map(|x| {
            x.as_f64()
                .ok_or_else(|| bad(format!("non-number in {key}")))
        })
        .collect()
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| float_hex(*x)).collect()
}

fn model_of(req: &Value) -> CoreResult<Model3> {
    let l = req.get("layout").ok_or_else(|| bad("missing layout"))?;
    let coordinate = match l.get("coordinate").and_then(Value::as_str) {
        Some("physical") => Coordinate::Physical,
        Some("comoving") => Coordinate::Comoving,
        _ => return Err(bad("coordinate must be physical or comoving")),
    };
    let boundary = match l.get("boundary").and_then(Value::as_str) {
        Some("outflow") => Boundary::Outflow,
        Some("closed") => Boundary::Closed,
        _ => return Err(bad("boundary must be outflow or closed")),
    };
    let phi_edges = match (l.get("phi_edges"), l.get("phi_uniform")) {
        (Some(_), None) => vec_of(l, "phi_edges")?,
        (None, Some(u)) => {
            let n = u
                .get("n")
                .and_then(Value::as_u64)
                .ok_or_else(|| bad("phi_uniform.n must be a nonnegative integer"))?;
            uniform_phi_edges(n as usize, f64_of(u, "offset")?)?
        }
        _ => return Err(bad("give exactly one of phi_edges and phi_uniform")),
    };
    let layout = Layout3::new(
        vec_of(l, "q_edges")?,
        vec_of(l, "mu_edges")?,
        phi_edges,
        coordinate,
        boundary,
    )?;
    let g = req.get("geometry").ok_or_else(|| bad("missing geometry"))?;
    let geometry = match g.get("kind").and_then(Value::as_str) {
        Some("constant") => Geometry3::Constant {
            h1: f64_of(g, "h1")?,
            h2: f64_of(g, "h2")?,
            h3: f64_of(g, "h3")?,
        },
        Some("power_law") => Geometry3::PowerLaw {
            hh1: f64_of(g, "hh1")?,
            hh2: f64_of(g, "hh2")?,
            hh3: f64_of(g, "hh3")?,
        },
        _ => return Err(bad("geometry kind must be constant or power_law")),
    };
    let absorber = match req.get("absorber") {
        None | Some(Value::Null) => None,
        Some(a) => Some(Absorber {
            k0: f64_of(a, "k0")?,
            q_chi: f64_of(a, "q_chi")?,
        }),
    };
    let drop_partial_t = req
        .get("drop_partial_t")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Model3::new(
        layout,
        geometry,
        f64_of(req, "gamma")?,
        absorber,
        drop_partial_t,
    )
}

/// The grid constants under the golden fixture's names (`B` as five rows).
fn grid_constants(l: &Layout3) -> Value {
    let mut out = serde_json::Map::new();
    for (name, v) in [
        ("dq", &l.dq),
        ("dm", &l.dm),
        ("dphi", &l.dphi),
        ("m2", &l.m2),
        ("t2", &l.t2),
        ("p2", &l.p2),
        ("r", &l.r),
        ("e", &l.e),
        ("g", &l.g),
        ("cphi", &l.cphi),
        ("sphi", &l.sphi),
        ("c2", &l.c2),
        ("s2h", &l.s2h),
        ("wface", &l.wface),
        ("sn2", &l.sn2),
        ("om", &l.om),
    ] {
        out.insert(name.into(), json!(hexes(v)));
    }
    let rows: Vec<Vec<String>> = (0..5)
        .map(|m| l.b.iter().map(|b| float_hex(b[m])).collect())
        .collect();
    out.insert("B".into(), json!(rows));
    Value::Object(out)
}

fn probe(req: &Value, model: Arc<Model3>) -> CoreResult<Value> {
    let n = model.layout.n();
    let t = f64_of(req, "t")?;
    let states: Vec<Vec<f64>> = req
        .get("states")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("missing states"))?
        .iter()
        .map(|s| {
            s.as_array()
                .ok_or_else(|| bad("state must be an array"))?
                .iter()
                .map(|x| x.as_f64().ok_or_else(|| bad("non-number state")))
                .collect()
        })
        .collect::<CoreResult<_>>()?;
    if states.is_empty() {
        return Err(bad("states must not be empty"));
    }
    let group_rates = match req.get("group_rates") {
        None | Some(Value::Null) => None,
        Some(_) => Some(vec_of(req, "group_rates")?),
    };
    let problem = support::problem(model.clone())?;
    let jvp_fn = problem.jvp_callback().ok_or_else(|| bad("no jvp"))?;
    let mut rhs = Vec::new();
    let mut ft = Vec::new();
    let mut jvp = Vec::new();
    let mut with_rates = Vec::new();
    let mut out = vec![0.0; n];
    for s in &states {
        model.rhs(t, s, &mut out)?;
        rhs.push(out.clone());
        model.partial_t(t, s, &mut out)?;
        ft.push(out.clone());
        jvp_fn(t, &states[0], s, &mut out)?;
        jvp.push(out.clone());
        if let Some(k) = &group_rates {
            model.rhs_with_group_rates(t, s, k, &mut out)?;
            with_rates.push(hexes(&out));
        }
    }
    let kbar: Vec<[f64; 2]> = (0..model.layout.nq)
        .map(|i| {
            let (k, dk) = model.kbar(i, t);
            [k, dk]
        })
        .collect();
    let dense = if n <= 600 {
        let mut m = DenseMatrix::zeros(n, n);
        model.dense_fill(t, &mut m)?;
        Value::from(
            (0..n)
                .map(|r| (0..n).map(|c| m[(r, c)]).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        )
    } else {
        Value::Null
    };
    let bw = model.layout.bandwidth();
    let mut band = vec![0.0; n * (2 * bw + 1)];
    model.band_fill(t, &mut band)?;
    let (v, dv) = model.drift_at(t)?;
    let (vq, vmu, vphi) = model.layout.face_velocities(v);
    let face = json!({"vq": vq, "vmu": vmu, "vphi": vphi,
        "vq_hex": hexes(&vq), "vmu_hex": hexes(&vmu), "vphi_hex": hexes(&vphi),
        "vmu_index": "k*(nm-1) + l-1 for interior mu faces l = 1..nm-1"});
    Ok(
        json!({"status":"OK","n":n,"M":model.layout.m,"bandwidth":bw,
        "autonomous":model.autonomous(),"rhs":rhs,"rhs_hex":rhs.iter().map(|r| hexes(r)).collect::<Vec<_>>(),
        "partial_t":ft,"jvp":jvp,"kbar":kbar,"dense":dense,"band":band,
        "rhs_group_rates_hex": if group_rates.is_some() { json!(with_rates) } else { Value::Null },
        "drift":{"v":v,"dv":dv,"v_hex":hexes(&v),"dv_hex":hexes(&dv)},
        "face_velocities":face,"grid_constants":grid_constants(&model.layout),
        "inputs_hex":{"q_edges":hexes(&model.layout.q_edges),"mu_edges":hexes(&model.layout.mu_edges),
            "phi_edges":hexes(&model.layout.phi_edges),"t":float_hex(t)},
        "partial_t_hex":ft.iter().map(|r| hexes(r)).collect::<Vec<_>>()}),
    )
}

struct Run {
    times: Vec<f64>,
    states: Vec<Vec<f64>>,
    success: bool,
    message: String,
    counters: WorkCounters,
    details: Value,
}

fn integrate(req: &Value, model: Arc<Model3>) -> CoreResult<Run> {
    let n = model.layout.n();
    let arm = req
        .get("arm")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("missing arm"))?;
    let y0 = vec_of(req, "initial_state")?;
    let span = vec_of(req, "t_span")?;
    if span.len() != 2 || y0.len() != n {
        return Err(bad(
            "t_span must have 2 entries and initial_state n entries",
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
    let problem = support::problem(model.clone())?;
    let fixed_steps = req.get("fixed_steps").and_then(Value::as_u64).unwrap_or(0) as usize;
    match arm {
        "dense" => {
            let (fast, partial) = integrate_rodas5p_fast_observed_with_atol(
                &problem,
                (t0, tf),
                &y0,
                &adaptive,
                &output,
                &atol,
            )?;
            Ok(Run {
                details: json!({"attempts":fast.attempts,"accepted_steps":fast.accepted_steps,
                    "rejected_steps":fast.rejected_steps,"jacobian_reuses":fast.jacobian_reuses,
                    "lu":format!("{:?}",fast.lu),"driver":fast.driver,
                    "internal_steps":fast.observed.internal_steps,
                    "output_clipped_steps":fast.observed.output_clipped_steps,
                    "partial":partial.map(|p| format!("{p:?}"))}),
                times: fast.observed.t,
                states: fast.observed.y,
                success: fast.observed.success,
                message: fast.observed.message,
                counters: fast.observed.counters,
            })
        }
        "banded" | "fixed_banded" => {
            let config = if arm == "fixed_banded" {
                if fixed_steps == 0 {
                    return Err(bad("fixed_banded needs fixed_steps"));
                }
                let h = (tf - t0) / fixed_steps as f64;
                AdaptiveStepConfig {
                    rtol: 1.0e10,
                    initial_step: h,
                    max_step: h,
                    ..adaptive
                }
            } else {
                adaptive
            };
            let band = support::band(model.clone());
            let r = integrate_rodas5p_fast_banded_observed_with_atol(
                &problem,
                &band,
                (t0, tf),
                &y0,
                &config,
                &output,
                &atol,
            )?;
            let fast = r.fast;
            Ok(Run {
                details: json!({"attempts":fast.attempts,"accepted_steps":fast.accepted_steps,
                    "rejected_steps":fast.rejected_steps,"jacobian_reuses":fast.jacobian_reuses,
                    "lu":format!("{:?}",fast.lu),"driver":fast.driver,
                    "internal_steps":fast.observed.internal_steps,
                    "output_clipped_steps":fast.observed.output_clipped_steps,
                    "band_factor_operations":r.work.factor_operations,
                    "band_solve_operations":r.work.solve_operations,
                    "band_stored_slots":r.work.stored_slots,
                    "partial":r.partial.map(|p| format!("{p:?}"))}),
                times: fast.observed.t,
                states: fast.observed.y,
                success: fast.observed.success,
                message: fast.observed.message,
                counters: fast.observed.counters,
            })
        }
        "fixed_dense" => {
            if fixed_steps == 0 {
                return Err(bad("fixed_dense needs fixed_steps"));
            }
            let h = (tf - t0) / fixed_steps as f64;
            let mut counters = WorkCounters::default();
            let mut y = y0.clone();
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
                times: vec![t0, tf],
                states: vec![y0, y],
                success: true,
                message: "fixed steps".into(),
                counters,
            })
        }
        _ => Err(bad("unknown arm")),
    }
}

fn result(status: &str, reason: &str, code: Option<&str>) -> Value {
    json!({"status":status,"reason":reason,"code":code,"times":[],"states":[],"counters":null,
        "timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"}})
}

/// Typed refusals of the request (declaration, shape) versus numerical
/// failures of a well-posed run.
fn failure(e: &CoreError) -> Value {
    let status = match e {
        CoreError::InvalidInput(_) | CoreError::Dimension(_) => "REFUSED",
        _ => "NUMERICAL_FAILURE",
    };
    result(status, &e.to_string(), refusal_code(e))
}

fn run() -> Value {
    let args: Vec<String> = env::args().collect();
    if !(args.len() == 3 || (args.len() == 5 && args[3] == "--measurement-mode"))
        || args[1] != "--request"
    {
        return result(
            "ENVIRONMENT_FAILURE",
            "usage: sphere_photon_fv_run --request PATH [--measurement-mode timing|allocation]",
            None,
        );
    }
    let mode = if args.len() == 5 {
        args[4].as_str()
    } else {
        "timing"
    };
    if mode != "timing" && mode != "allocation" {
        return result("ENVIRONMENT_FAILURE", "invalid measurement mode", None);
    }
    let req: Value = match fs::read_to_string(&args[2])
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(v) => v,
        None => return result("ENVIRONMENT_FAILURE", "request read/parse failed", None),
    };
    let model = match model_of(&req) {
        Ok(m) => Arc::new(m),
        Err(e) => return failure(&e),
    };
    if req.get("mode").and_then(Value::as_str) == Some("probe") {
        return probe(&req, model).unwrap_or_else(|e| failure(&e));
    }
    // One warm run, then the measured run (timing or allocation count).
    if let Err(e) = integrate(&req, model.clone()) {
        return failure(&e);
    }
    let before = allocations();
    let start = Instant::now();
    let run = integrate(&req, model);
    let elapsed = start.elapsed().as_nanos() as u64;
    let counted = allocations() - before;
    let control_before = allocations();
    let positive = vec![1_u8; 64];
    std::hint::black_box(&positive);
    let control = allocations() - control_before;
    match run {
        Ok(r) => {
            json!({"status": if r.success {"OK"} else {"NUMERICAL_FAILURE"}, "reason": r.message,
                "times": r.times, "states": r.states, "counters": r.counters, "details": r.details,
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
