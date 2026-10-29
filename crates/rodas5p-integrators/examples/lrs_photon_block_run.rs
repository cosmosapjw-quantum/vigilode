//! JSON runner of the structured stage solvers of the LRS photon transport
//! adapter (BASS/rei DAG node P03). Usage:
//!
//! `lrs_photon_block_run --request PATH [--measurement-mode timing|allocation]`
//!
//! The request declares the P02 model (`layout`, `geometry`, `gamma`,
//! `absorber`, as `lrs_photon_fv_run`), optionally a `border` (`{k, alpha,
//! beta, kappa, tau, gamma0}`: the bordered model, whose own `gamma` must be
//! 0), and one mode:
//!
//! ```text
//! integrate (default): arm: banded | dense | client_exact | client_krylov |
//!     client_textbook | fixed_banded | fixed_client (P02 model);
//!     bordered_dense | bordered_client (bordered model);
//!     initial_state, t_span, output_times, rtol, atol (vector),
//!     initial_step, max_attempts, fixed_steps (fixed arms),
//!     krylov: {restart, max_iterations, rtol} (client_krylov),
//!     sample: {every, max, condition} (client arms): record every
//!     `every`-th stage system and compare it with a dense LU of W
//! opcount: cases [{nq, nm}], q_range, inv: one linearization,
//!     factorization and solve of the exact structured solver per case,
//!     its counted flops against the closed forms
//! probe: t, states (bordered model): rhs, partial_t, dense Jacobian,
//!     D_gamma; riccati: {x0, times}
//! ```
//!
//! Wall time is `diagnostic_only`. In allocation mode the integration (the
//! second, measured call; sampling analysis excluded) is counted on its
//! thread with a positive control. Statuses as `lrs_photon_fv_run`.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{env, fs};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LuFactorization, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, StageLinearSystem,
    integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_client_observed_with_atol, integrate_rodas5p_fast_observed_with_atol,
};
use serde_json::{Value, json};

#[path = "lrs_photon_fv_support/mod.rs"]
mod support;
use support::block_solver::{
    BlockStageSolver, Capacitance, Mode, OpCounts, exact_solve_flops, factor_flops,
};
use support::border::{BorderParams, Bordered, BorderedStageSolver};
use support::{Absorber, Boundary, Coordinate, Geometry, Layout, Model};

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
        .map(|x| x as usize)
        .ok_or_else(|| bad(format!("missing integer {key}")))
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

fn geometry_of(req: &Value) -> CoreResult<Geometry> {
    let g = req.get("geometry").ok_or_else(|| bad("missing geometry"))?;
    match g.get("kind").and_then(Value::as_str) {
        Some("constant") => Ok(Geometry::Constant {
            h: f64_of(g, "h")?,
            s: f64_of(g, "s")?,
        }),
        Some("power_law") => Ok(Geometry::PowerLaw {
            hh: f64_of(g, "hh")?,
            s0: f64_of(g, "s0")?,
            p: f64_of(g, "p")?,
        }),
        _ => Err(bad("geometry kind must be constant or power_law")),
    }
}

fn absorber_of(req: &Value) -> CoreResult<Option<Absorber>> {
    match req.get("absorber") {
        None | Some(Value::Null) => Ok(None),
        Some(a) => Ok(Some(Absorber {
            k0: f64_of(a, "k0")?,
            q_chi: f64_of(a, "q_chi")?,
        })),
    }
}

fn model_of(req: &Value) -> CoreResult<Model> {
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
    let layout = Layout::new(
        vec_of(l, "q_edges")?,
        vec_of(l, "mu_edges")?,
        coordinate,
        boundary,
    )?;
    Model::new(
        layout,
        geometry_of(req)?,
        f64_of(req, "gamma")?,
        absorber_of(req)?,
        false,
    )
}

fn border_of(req: &Value) -> CoreResult<Option<BorderParams>> {
    match req.get("border") {
        None | Some(Value::Null) => Ok(None),
        Some(b) => Ok(Some(BorderParams {
            k: f64_of(b, "k")?,
            alpha: f64_of(b, "alpha")?,
            beta: f64_of(b, "beta")?,
            kappa: f64_of(b, "kappa")?,
            tau: f64_of(b, "tau")?,
            gamma0: f64_of(b, "gamma0")?,
        })),
    }
}

/// One recorded stage system: the linearization `(t, y)`, `inv`, the
/// right-hand side and the client's solution.
struct Sample {
    t: f64,
    inv: f64,
    y: Vec<f64>,
    b: Vec<f64>,
    x: Vec<f64>,
}

/// A client shared with the runner (to read its counts after the run) that
/// records every `every`-th stage system into preallocated samples.
struct Recorded<C> {
    client: C,
    every: usize,
    solves: usize,
    used: usize,
    t: f64,
    inv: f64,
    y: Vec<f64>,
    samples: Vec<Sample>,
}

struct Shared<C>(Arc<Mutex<Recorded<C>>>);

impl<C: StageLinearSystem> Shared<C> {
    fn new(client: C, every: usize, max: usize) -> (Self, Arc<Mutex<Recorded<C>>>) {
        let n = client.dimension();
        let samples = (0..if every > 0 { max } else { 0 })
            .map(|_| Sample {
                t: 0.0,
                inv: 0.0,
                y: vec![0.0; n],
                b: vec![0.0; n],
                x: vec![0.0; n],
            })
            .collect();
        let inner = Arc::new(Mutex::new(Recorded {
            client,
            every,
            solves: 0,
            used: 0,
            t: 0.0,
            inv: 0.0,
            y: vec![0.0; n],
            samples,
        }));
        (Self(inner.clone()), inner)
    }
}

impl<C: StageLinearSystem> StageLinearSystem for Shared<C> {
    fn dimension(&self) -> usize {
        self.0.lock().unwrap().client.dimension()
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        let mut r = self.0.lock().unwrap();
        r.t = t;
        r.y.copy_from_slice(y);
        r.client.linearize(problem, t, y, counters)
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let mut r = self.0.lock().unwrap();
        r.inv = inv_h_gamma;
        r.client.factor(inv_h_gamma, counters)
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        let mut guard = self.0.lock().unwrap();
        let r = &mut *guard;
        let record = r.every > 0 && r.solves.is_multiple_of(r.every) && r.used < r.samples.len();
        r.solves += 1;
        if record {
            r.samples[r.used].b.copy_from_slice(b);
        }
        r.client.solve_in_place(b, counters)?;
        if record {
            let k = r.used;
            let (t, inv) = (r.t, r.inv);
            let s = &mut r.samples[k];
            s.x.copy_from_slice(b);
            s.t = t;
            s.inv = inv;
            s.y.copy_from_slice(&r.y);
            r.used += 1;
        }
        Ok(())
    }
}

/// What the runner reads back from a client after its run.
enum Client {
    Block(Arc<Mutex<Recorded<BlockStageSolver>>>),
    Bordered(Arc<Mutex<Recorded<BorderedStageSolver>>>),
}

struct Run {
    times: Vec<f64>,
    states: Vec<Vec<f64>>,
    success: bool,
    message: String,
    counters: WorkCounters,
    details: Value,
    client: Option<Client>,
}

fn ops_json(o: &OpCounts) -> Value {
    json!({"linearize":o.linearize,"factor":o.factor,"solve":o.solve,
        "linearizations":o.linearizations,"factorizations":o.factorizations,"solves":o.solves,
        "krylov_iterations":o.krylov_iterations,"krylov_matvecs":o.krylov_matvecs,
        "sweeps":o.sweeps,"max_krylov_iterations":o.max_krylov_iterations})
}

fn adaptive_of(req: &Value, t0: f64, tf: f64, atol_min: f64) -> CoreResult<AdaptiveStepConfig> {
    let initial_step = req
        .get("initial_step")
        .and_then(Value::as_f64)
        .unwrap_or(1.0e-4 * (tf - t0));
    let max_attempts = req
        .get("max_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(1_000_000) as usize;
    let mut adaptive = AdaptiveStepConfig {
        atol: atol_min,
        rtol: f64_of(req, "rtol")?,
        initial_step,
        max_step: tf - t0,
        max_attempts,
        ..AdaptiveStepConfig::default()
    };
    if req
        .get("arm")
        .and_then(Value::as_str)
        .is_some_and(|a| a.starts_with("fixed_"))
    {
        let steps = usize_of(req, "fixed_steps")?;
        if steps == 0 {
            return Err(bad("fixed arms need fixed_steps > 0"));
        }
        let h = (tf - t0) / steps as f64;
        adaptive.rtol = 1.0e10;
        adaptive.initial_step = h;
        adaptive.max_step = h;
    }
    Ok(adaptive)
}

fn integrate(req: &Value, model: Arc<Model>, border: Option<Arc<Bordered>>) -> CoreResult<Run> {
    let arm = req
        .get("arm")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("missing arm"))?;
    let bordered_arm = arm.starts_with("bordered_");
    let n = match (&border, bordered_arm) {
        (Some(b), true) => b.n(),
        (None, false) => model.layout.n(),
        _ => {
            return Err(bad(
                "bordered arms need a border, P02 arms must not have one",
            ));
        }
    };
    let y0 = vec_of(req, "initial_state")?;
    let span = vec_of(req, "t_span")?;
    if span.len() != 2 || y0.len() != n {
        return Err(bad(
            "t_span must have 2 entries and initial_state n entries",
        ));
    }
    let (t0, tf) = (span[0], span[1]);
    let atol = vec_of(req, "atol")?;
    if atol.len() != n {
        return Err(bad("atol must be a vector of length n"));
    }
    let atol_min = atol.iter().copied().fold(f64::INFINITY, f64::min);
    let output = OutputSchedule::new(vec_of(req, "output_times")?)?;
    let adaptive = adaptive_of(req, t0, tf, atol_min)?;
    let (every, max) = match req.get("sample") {
        None | Some(Value::Null) => (0, 0),
        Some(s) => (usize_of(s, "every")?, usize_of(s, "max")?),
    };
    let problem = match &border {
        Some(b) => support::border::problem(b.clone())?,
        None => support::problem(model.clone())?,
    };
    let fast_details = |fast: &rodas5p_integrators::Rodas5pFastResult, partial: Value| {
        json!({"attempts":fast.attempts,"accepted_steps":fast.accepted_steps,
            "rejected_steps":fast.rejected_steps,"jacobian_reuses":fast.jacobian_reuses,
            "lu":format!("{:?}",fast.lu),"driver":fast.driver,
            "internal_steps":fast.observed.internal_steps,
            "output_clipped_steps":fast.observed.output_clipped_steps,"partial":partial})
    };
    let partial_json = |p: Option<rodas5p_integrators::DriverPartial>| match p {
        None => Value::Null,
        Some(p) => json!({"t":p.t,"reason":format!("{:?}",p.reason),
            "last_failure":format!("{:?}",p.last_failure)}),
    };
    let client_run = |client: Box<dyn StageLinearSystem>, handle: Client| -> CoreResult<Run> {
        let (fast, partial, _client) = integrate_rodas5p_fast_client_observed_with_atol(
            &problem,
            client,
            (t0, tf),
            &y0,
            &adaptive,
            &output,
            &atol,
        )?;
        Ok(Run {
            details: fast_details(&fast, partial_json(partial)),
            times: fast.observed.t,
            states: fast.observed.y,
            success: fast.observed.success,
            message: fast.observed.message,
            counters: fast.observed.counters,
            client: Some(handle),
        })
    };
    match arm {
        "dense" | "bordered_dense" => {
            let (fast, partial) = integrate_rodas5p_fast_observed_with_atol(
                &problem,
                (t0, tf),
                &y0,
                &adaptive,
                &output,
                &atol,
            )?;
            Ok(Run {
                details: fast_details(&fast, partial_json(partial)),
                times: fast.observed.t,
                states: fast.observed.y,
                success: fast.observed.success,
                message: fast.observed.message,
                counters: fast.observed.counters,
                client: None,
            })
        }
        "banded" | "fixed_banded" => {
            let band = support::band(model.clone());
            let r = integrate_rodas5p_fast_banded_observed_with_atol(
                &problem,
                &band,
                (t0, tf),
                &y0,
                &adaptive,
                &output,
                &atol,
            )?;
            let mut details = fast_details(&r.fast, partial_json(r.partial));
            details["band_factor_operations"] = json!(r.work.factor_operations);
            details["band_solve_operations"] = json!(r.work.solve_operations);
            details["band_stored_slots"] = json!(r.work.stored_slots);
            let fast = r.fast;
            Ok(Run {
                details,
                times: fast.observed.t,
                states: fast.observed.y,
                success: fast.observed.success,
                message: fast.observed.message,
                counters: fast.observed.counters,
                client: None,
            })
        }
        "client_exact" | "client_krylov" | "client_textbook" | "fixed_client" => {
            let mode = if arm == "client_krylov" {
                let k = req.get("krylov").ok_or_else(|| bad("missing krylov"))?;
                Mode::Krylov {
                    restart: usize_of(k, "restart")?,
                    max_iterations: usize_of(k, "max_iterations")?,
                    rtol: f64_of(k, "rtol")?,
                    two_level: k.get("two_level").and_then(Value::as_bool).unwrap_or(true),
                }
            } else {
                Mode::Exact
            };
            let capacitance = if arm == "client_textbook" {
                Capacitance::Textbook
            } else {
                Capacitance::Conservation
            };
            let solver = BlockStageSolver::new(model.clone(), 0, mode, capacitance)?;
            let (shared, handle) = Shared::new(solver, every, max);
            client_run(Box::new(shared), Client::Block(handle))
        }
        "bordered_client" => {
            let solver = BorderedStageSolver::new(border.clone().expect("border"), 0)?;
            let (shared, handle) = Shared::new(solver, every, max);
            client_run(Box::new(shared), Client::Bordered(handle))
        }
        _ => Err(bad("unknown arm")),
    }
}

fn inf_norm(v: &[f64]) -> f64 {
    v.iter().fold(0.0_f64, |m, x| m.max(x.abs()))
}

fn l2_norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Compares every recorded stage system with a dense LU of `W = inv I - J`.
fn analyze(
    samples: &[Sample],
    model: &Model,
    border: Option<&Bordered>,
    condition: bool,
) -> CoreResult<Value> {
    let mut rows = Vec::new();
    for s in samples {
        let n = s.b.len();
        let mut j = DenseMatrix::zeros(n, n);
        match border {
            Some(b) => b.dense_fill(s.t, &s.y, &mut j)?,
            None => model.dense_fill(s.t, &mut j)?,
        }
        let mut w = DenseMatrix::zeros(n, n);
        let mut w_norm = 0.0_f64;
        for r in 0..n {
            let mut row = 0.0;
            for c in 0..n {
                let v = if r == c { s.inv } else { 0.0 } - j[(r, c)];
                w[(r, c)] = v;
                row += v.abs();
            }
            w_norm = w_norm.max(row);
        }
        let residual = |x: &[f64]| -> Vec<f64> {
            (0..n)
                .map(|r| {
                    let mut acc = s.b[r];
                    for c in 0..n {
                        acc -= w[(r, c)] * x[c];
                    }
                    acc
                })
                .collect()
        };
        let lu = LuFactorization::new(&w)?;
        let x_lu = lu.solve(&s.b)?;
        let (r_c, r_lu) = (residual(&s.x), residual(&x_lu));
        let eta = |r: &[f64], x: &[f64]| inf_norm(r) / (w_norm * inf_norm(x) + inf_norm(&s.b));
        // componentwise (Oettli-Prager) backward error: max_i |r_i| / (|W| |x| + |b|)_i
        let omega = |r: &[f64], x: &[f64]| -> f64 {
            (0..n)
                .map(|i| {
                    let mut den = s.b[i].abs();
                    for c in 0..n {
                        den += (w[(i, c)] * x[c]).abs();
                    }
                    if den > 0.0 {
                        r[i].abs() / den
                    } else {
                        r[i].abs()
                    }
                })
                .fold(0.0_f64, f64::max)
        };
        let diff: Vec<f64> = s.x.iter().zip(&x_lu).map(|(a, b)| a - b).collect();
        // per block: the photon block against its own size, and each border
        // unknown against its own magnitude
        let n_ph = model.layout.n();
        let photon_fwd = inf_norm(&diff[..n_ph]) / inf_norm(&x_lu[..n_ph]);
        let border_fwd: Vec<f64> = (n_ph..n)
            .map(|k| {
                if x_lu[k] != 0.0 {
                    diff[k].abs() / x_lu[k].abs()
                } else {
                    diff[k].abs()
                }
            })
            .collect();
        let kappa = if condition {
            let mut rows_abs = vec![0.0; n];
            for c in 0..n {
                let mut e = vec![0.0; n];
                e[c] = 1.0;
                let col = lu.solve(&e)?;
                for (r, v) in col.iter().enumerate() {
                    rows_abs[r] += v.abs();
                }
            }
            json!(w_norm * rows_abs.iter().fold(0.0_f64, |m, v| m.max(*v)))
        } else {
            Value::Null
        };
        rows.push(json!({"t":s.t,"inv":s.inv,
            "backward_error_client":eta(&r_c, &s.x),"backward_error_lu":eta(&r_lu, &x_lu),
            "relative_residual_client":l2_norm(&r_c)/l2_norm(&s.b),
            "componentwise_backward_error_client":omega(&r_c, &s.x),
            "componentwise_backward_error_lu":omega(&r_lu, &x_lu),
            "photon_block_forward_vs_lu":photon_fwd,"border_forward_vs_lu":border_fwd,
            "forward_difference_vs_lu":inf_norm(&diff)/inf_norm(&x_lu),
            "kappa_inf":kappa,"b_norm":inf_norm(&s.b)}));
    }
    Ok(Value::from(rows))
}

fn uniform(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    (0..=n)
        .map(|k| {
            if k == n {
                hi
            } else {
                lo + (hi - lo) * k as f64 / n as f64
            }
        })
        .collect()
}

/// Counted flops of one exact linearization, factorization and solve per
/// grid against the closed forms.
fn opcount(req: &Value) -> CoreResult<Value> {
    let cases = req
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("missing cases"))?;
    let q = vec_of(req, "q_range")?;
    let inv = f64_of(req, "inv")?;
    let mut out = Vec::new();
    for c in cases {
        let (nq, nm) = (usize_of(c, "nq")?, usize_of(c, "nm")?);
        let layout = Layout::new(
            uniform(q[0], q[1], nq),
            uniform(-1.0, 1.0, nm),
            Coordinate::Physical,
            Boundary::Outflow,
        )?;
        let model = Arc::new(Model::new(
            layout,
            geometry_of(req)?,
            f64_of(req, "gamma")?,
            absorber_of(req)?,
            false,
        )?);
        let n = model.layout.n();
        let mut solver = BlockStageSolver::new(model, 0, Mode::Exact, Capacitance::Conservation)?;
        solver.linearize_at(0.0)?;
        solver.factor_with(inv)?;
        let mut b: Vec<f64> = (0..n).map(|k| 1.0 + (k % 7) as f64 * 0.125).collect();
        let mut counters = WorkCounters::default();
        solver.solve_with(&mut b, &mut counters)?;
        let o = solver.ops;
        out.push(json!({"nq":nq,"nm":nm,"n":n,"direction":format!("{:?}",solver.direction()),
            "factor_counted":o.factor,"factor_formula":factor_flops(nq, nm, Capacitance::Conservation),
            "solve_counted":o.solve,"solve_formula":exact_solve_flops(nq, nm, true)}));
    }
    Ok(json!({"status":"OK","cases":out}))
}

fn probe(req: &Value, border: Arc<Bordered>) -> CoreResult<Value> {
    let n = border.n();
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
    let (mut rhs, mut ft, mut dense, mut dg) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut out = vec![0.0; n];
    for s in &states {
        border.rhs(t, s, &mut out)?;
        rhs.push(out.clone());
        border.partial_t(t, s, &mut out)?;
        ft.push(out.clone());
        let mut m = DenseMatrix::zeros(n, n);
        border.dense_fill(t, s, &mut m)?;
        dense.push(
            (0..n)
                .map(|r| (0..n).map(|c| m[(r, c)]).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        );
        dg.push(border.d_gamma(s));
    }
    let riccati = match req.get("riccati") {
        None | Some(Value::Null) => Value::Null,
        Some(r) => {
            let x0 = f64_of(r, "x0")?;
            Value::from(
                vec_of(r, "times")?
                    .iter()
                    .map(|&tt| border.riccati(x0, tt))
                    .collect::<Vec<_>>(),
            )
        }
    };
    Ok(
        json!({"status":"OK","n":n,"rhs":rhs,"partial_t":ft,"dense":dense,"d_gamma":dg,
        "w":border.w,"riccati":riccati}),
    )
}

fn result(status: &str, reason: &str) -> Value {
    json!({"status":status,"reason":reason,"times":[],"states":[],"counters":null,
        "timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"}})
}

fn failure(e: &CoreError) -> Value {
    let status = match e {
        CoreError::InvalidInput(_) | CoreError::Dimension(_) => "REFUSED",
        _ => "NUMERICAL_FAILURE",
    };
    result(status, &e.to_string())
}

fn run() -> Value {
    let args: Vec<String> = env::args().collect();
    if !(args.len() == 3 || (args.len() == 5 && args[3] == "--measurement-mode"))
        || args[1] != "--request"
    {
        return result(
            "ENVIRONMENT_FAILURE",
            "usage: lrs_photon_block_run --request PATH [--measurement-mode timing|allocation]",
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
    if req.get("mode").and_then(Value::as_str) == Some("opcount") {
        return opcount(&req).unwrap_or_else(|e| failure(&e));
    }
    let model = match model_of(&req) {
        Ok(m) => Arc::new(m),
        Err(e) => return failure(&e),
    };
    let border = match border_of(&req) {
        Ok(None) => None,
        Ok(Some(p)) => match Bordered::new(model.clone(), p) {
            Ok(b) => Some(Arc::new(b)),
            Err(e) => return failure(&e),
        },
        Err(e) => return failure(&e),
    };
    if req.get("mode").and_then(Value::as_str) == Some("probe") {
        return match border {
            Some(b) => probe(&req, b).unwrap_or_else(|e| failure(&e)),
            None => failure(&bad("probe needs a border")),
        };
    }
    // One warm run, then the measured run (timing or allocation count).
    if let Err(e) = integrate(&req, model.clone(), border.clone()) {
        return failure(&e);
    }
    let before = allocations();
    let start = Instant::now();
    let run = integrate(&req, model.clone(), border.clone());
    let elapsed = start.elapsed().as_nanos() as u64;
    let counted = allocations() - before;
    let control_before = allocations();
    let positive = vec![1_u8; 64];
    std::hint::black_box(&positive);
    let control = allocations() - control_before;
    let condition = req
        .get("sample")
        .and_then(|s| s.get("condition"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    match run {
        Ok(r) => {
            let (ops, direction, samples) = match &r.client {
                None => (Value::Null, Value::Null, Ok(Value::Null)),
                Some(Client::Block(h)) => {
                    let g = h.lock().unwrap();
                    (
                        ops_json(&g.client.ops),
                        json!(format!("{:?}", g.client.direction())),
                        analyze(&g.samples[..g.used], &model, None, condition),
                    )
                }
                Some(Client::Bordered(h)) => {
                    let g = h.lock().unwrap();
                    (
                        ops_json(&g.client.inner.ops),
                        json!(format!("{:?}", g.client.inner.direction())),
                        analyze(&g.samples[..g.used], &model, border.as_deref(), condition),
                    )
                }
            };
            let samples = match samples {
                Ok(s) => s,
                Err(e) => return failure(&e),
            };
            json!({"status": if r.success {"OK"} else {"NUMERICAL_FAILURE"}, "reason": r.message,
                "times": r.times, "states": r.states, "counters": r.counters, "details": r.details,
                "client_ops": ops, "direction": direction, "samples": samples,
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
