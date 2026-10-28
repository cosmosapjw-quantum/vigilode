//! JSON runner of the accepted-step conservative remap of the LRS photon
//! transport adapter (BASS/rei DAG node P05,
//! `lrs_photon_fv_support/remesh.rs`). Usage:
//!
//! `lrs_photon_remap_run --request PATH [--measurement-mode timing|allocation]`
//!
//! The request declares the P02 model on a uniform base energy grid and one
//! mode:
//!
//! ```text
//! layout:   {q_lo, q_hi, nq, mu_edges, coordinate: physical|comoving,
//!            boundary: outflow|closed}
//! geometry: {kind: constant, h, s} | {kind: power_law, hh, s0, p}
//! gamma, absorber: null | {k0, q_chi}
//! mode: integrate (default) | cycle | inject
//! cycle:     initial_state (base mesh), cycles, x_chi (optional):
//!            split_all/merge_pairs cycles of the state; with x_chi, also
//!            remesher cycles between the aligned mesh of x_chi and its
//!            split (the state first remapped onto the aligned mesh)
//! inject:    initial_state, t, refine (default false): propose onto the
//!            aligned mesh of x_chi(t) with inject_failure (the output
//!            buffer, prefilled with a sentinel, must stay untouched), then
//!            a normal propose and commit onto the other variant of that
//!            mesh (refined if `refine` is false, unrefined otherwise),
//!            compared with a fresh remesher's remap (state_after)
//! integrate: arm: static | hook | schedule | fixed_schedule,
//!            linear: banded (default) | client_exact | client_krylov,
//!            krylov: {restart, max_iterations, rtol, two_level},
//!            initial_state, t_span, output_times, rtol,
//!            atol_scale (default 1e-2 rtol; the cell scale rule),
//!            initial_step (default 1e-4 span), max_attempts (default 1e6),
//!            trigger (hook), remap_times (schedule arms),
//!            fixed_steps (fixed_schedule, per segment),
//!            refine (default false), max_remaps (default 4096),
//!            restart: h_next (default) | fresh,
//!            inject_failure_at_remap (1-based, optional),
//!            record_remap_states (default false)
//! ```
//!
//! Wall time is `diagnostic_only`. In allocation mode the integration (the
//! second, measured call) is counted on its thread with a positive control;
//! the cycle and inject modes always count their remaps after warm-up.
//!
//! Statuses as `lrs_photon_fv_run`: `OK`; `REFUSED` for an invalid request,
//! declaration or shape; `NUMERICAL_FAILURE` for every other error, an
//! unsuccessful run and a refused remap during an integration;
//! `ENVIRONMENT_FAILURE` for usage and request I/O.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::time::Instant;
use std::{env, fs};

use rodas5p_core::{CoreError, CoreResult};
use serde_json::{Value, json};

#[path = "lrs_photon_fv_support/mod.rs"]
mod support;
use support::block_solver::{Mode, OpCounts};
use support::remesh::{
    Arm, BaseGrid, Linear, Physics, RemapRefusal, RemapReport, RemeshWork, Remesher, Restart,
    RunConfig, RunError, RunOutcome, TraceEvent, aligned_edges, compensated_total,
    integrate_segments, merge_pairs, split_all, threshold_position,
};
use support::{Absorber, Boundary, Coordinate, Geometry, Layout};

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

fn bool_or(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn physics_of(req: &Value) -> CoreResult<(BaseGrid, Physics)> {
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
    let base = BaseGrid {
        lo: f64_of(l, "q_lo")?,
        hi: f64_of(l, "q_hi")?,
        n: usize_of(l, "nq")?,
    };
    base.validate()?;
    let g = req.get("geometry").ok_or_else(|| bad("missing geometry"))?;
    let geometry = match g.get("kind").and_then(Value::as_str) {
        Some("constant") => Geometry::Constant {
            h: f64_of(g, "h")?,
            s: f64_of(g, "s")?,
        },
        Some("power_law") => Geometry::PowerLaw {
            hh: f64_of(g, "hh")?,
            s0: f64_of(g, "s0")?,
            p: f64_of(g, "p")?,
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
    let physics = Physics {
        mu_edges: vec_of(l, "mu_edges")?,
        coordinate,
        boundary,
        geometry,
        gamma: f64_of(req, "gamma")?,
        absorber,
    };
    Ok((base, physics))
}

fn base_layout(base: &BaseGrid, physics: &Physics) -> CoreResult<Layout> {
    let mut edges = Vec::new();
    base.edges_into(&mut edges);
    Ok(physics.model(edges)?.layout)
}

fn refusal_core(r: RemapRefusal) -> CoreError {
    match r {
        RemapRefusal::Shape(_) | RemapRefusal::Domain | RemapRefusal::StaleMesh => {
            CoreError::Dimension(r.to_string())
        }
        _ => CoreError::NonFinite(r.to_string()),
    }
}

fn report_json(r: &RemapReport) -> Value {
    json!({"source_cells":r.source_cells,"destination_cells":r.destination_cells,
        "columns":r.columns,"overlap_segments":r.overlap_segments,
        "total_before":r.total_before,"total_after":r.total_after,
        "relative_total_change":r.relative_total_change,"l1_change":r.l1_change})
}

fn work_json(w: &RemeshWork) -> Value {
    json!({"attempts":w.attempts,"commits":w.commits,"refusals":w.refusals,
        "overlap_segments":w.overlap_segments,"l1_change_sum":w.l1_change_sum})
}

fn ops_json(o: &OpCounts) -> Value {
    json!({"linearize":o.linearize,"factor":o.factor,"solve":o.solve,
        "linearizations":o.linearizations,"factorizations":o.factorizations,"solves":o.solves,
        "krylov_iterations":o.krylov_iterations,"krylov_matvecs":o.krylov_matvecs,
        "sweeps":o.sweeps,"max_krylov_iterations":o.max_krylov_iterations})
}

/// Distance in units in the last place (ordered integer representation).
fn ulps(a: f64, b: f64) -> u64 {
    let key = |x: f64| {
        let i = x.to_bits() as i64;
        if i < 0 { i64::MIN - i } else { i }
    };
    key(a).abs_diff(key(b))
}

fn max_ulps(a: &[f64], b: &[f64]) -> u64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| ulps(*x, *y))
        .max()
        .unwrap_or(0)
}

/// Max relative difference over nonzero reference components, and the max
/// absolute value where the reference is zero.
fn rel_diff(a: &[f64], reference: &[f64]) -> (f64, f64) {
    let (mut rel, mut zero) = (0.0_f64, 0.0_f64);
    for (x, r) in a.iter().zip(reference) {
        if *r != 0.0 {
            rel = rel.max((x - r).abs() / r.abs());
        } else {
            zero = zero.max(x.abs());
        }
    }
    (rel, zero)
}

fn bits_equal(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// Refine/coarsen cycles: the buffer helpers, and with `x_chi` the remesher
/// between an aligned mesh and its split.
fn cycle(req: &Value) -> CoreResult<Value> {
    let (base, physics) = physics_of(req)?;
    let layout = base_layout(&base, &physics)?;
    let nm = layout.nm;
    let y0 = vec_of(req, "initial_state")?;
    if y0.len() != layout.n() {
        return Err(CoreError::Dimension(
            "initial_state must have the base layout's length".into(),
        ));
    }
    let k = usize_of(req, "cycles")?;
    if k == 0 {
        return Err(bad("cycles must be positive"));
    }
    // helper path
    let cap_cells = 2 * base.n;
    let (mut e1, mut y1) = (
        Vec::with_capacity(cap_cells + 1),
        Vec::with_capacity(2 + cap_cells * (nm + 1)),
    );
    let (mut e2, mut y2) = (
        Vec::with_capacity(cap_cells + 1),
        Vec::with_capacity(2 + cap_cells * (nm + 1)),
    );
    let refuse = refusal_core;
    split_all(&layout.q_edges, nm, &y0, &mut e1, &mut y1).map_err(refuse)?;
    merge_pairs(&e1, nm, &y1, &mut e2, &mut y2).map_err(refuse)?;
    let one_ulps = max_ulps(&y2, &y0);
    let one_bitwise = bits_equal(&y2, &y0) && bits_equal(&e2, &layout.q_edges);
    let split_total_change =
        (compensated_total(&y1) - compensated_total(&y0)).abs() / compensated_total(&y0).abs();
    y2.clear();
    y2.extend_from_slice(&y0);
    e2.clear();
    e2.extend_from_slice(&layout.q_edges);
    let before = allocations();
    for _ in 0..k {
        split_all(&e2, nm, &y2, &mut e1, &mut y1).map_err(refuse)?;
        merge_pairs(&e1, nm, &y1, &mut e2, &mut y2).map_err(refuse)?;
    }
    let helper_allocations = allocations() - before;
    let (k_rel, k_zero) = rel_diff(&y2, &y0);
    let helper = json!({"max_ulps_one_cycle":one_ulps,"bitwise_one_cycle":one_bitwise,
        "split_relative_total_change":split_total_change,
        "max_relative_after_cycles":k_rel,"max_abs_at_zero_reference":k_zero,
        "bitwise_after_cycles":bits_equal(&y2, &y0) && bits_equal(&e2, &layout.q_edges),
        "allocations_after_warmup":helper_allocations,"cycles":k});
    let kernel = match req.get("x_chi").and_then(Value::as_f64) {
        None => Value::Null,
        Some(x) => {
            let mut coarse_edges = Vec::new();
            let face = aligned_edges(base.lo, base.hi, base.n, x, false, &mut coarse_edges);
            let coarse = physics.model(coarse_edges.clone())?.layout;
            let mut fine_edges = Vec::new();
            split_all(
                &coarse_edges,
                nm,
                &vec![0.0; coarse.n()],
                &mut fine_edges,
                &mut Vec::new(),
            )
            .map_err(refuse)?;
            let fine = physics.model(fine_edges.clone())?.layout;
            let mut remesher = Remesher::new(&layout.q_edges, nm, 2 * base.n, 0).map_err(refuse)?;
            let mut y = Vec::with_capacity(remesher.state_len(2 * base.n));
            remesher
                .propose(&layout, &y0, &coarse_edges, false)
                .map_err(refuse)?;
            remesher.commit(&mut y).map_err(refuse)?;
            let start = y.clone();
            let mut worst_total = 0.0_f64;
            let mut one = |y: &mut Vec<f64>, worst: &mut f64| -> CoreResult<()> {
                let r = remesher
                    .propose(&coarse, y, &fine_edges, false)
                    .map_err(refuse)?;
                *worst = worst.max(r.relative_total_change);
                remesher.commit(y).map_err(refuse)?;
                let r = remesher
                    .propose(&fine, y, &coarse_edges, false)
                    .map_err(refuse)?;
                *worst = worst.max(r.relative_total_change);
                remesher.commit(y).map_err(refuse)?;
                Ok(())
            };
            one(&mut y, &mut worst_total)?;
            let one_ulps = max_ulps(&y, &start);
            let one_bitwise = bits_equal(&y, &start);
            let before = allocations();
            for _ in 1..k {
                one(&mut y, &mut worst_total)?;
            }
            let kernel_allocations = allocations() - before;
            let (k_rel, k_zero) = rel_diff(&y, &start);
            let total_drift = (compensated_total(&y) - compensated_total(&start)).abs()
                / compensated_total(&start).abs();
            json!({"x_chi":x,"face":face,"coarse_cells":coarse.nq,"fine_cells":fine.nq,
                "max_ulps_one_cycle":one_ulps,"bitwise_one_cycle":one_bitwise,
                "max_relative_after_cycles":k_rel,"max_abs_at_zero_reference":k_zero,
                "max_relative_total_change_per_remap":worst_total,
                "relative_total_drift_after_cycles":total_drift,
                "allocations_after_warmup":kernel_allocations,
                "remesh_work":work_json(&remesher.work),"epoch":remesher.epoch()})
        }
    };
    Ok(
        json!({"status":"OK","mode":"cycle","helper":helper,"kernel":kernel,
        "timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"}}),
    )
}

/// An injected failure must leave the output buffer, mesh and epoch
/// bitwise unchanged while its work stays counted; then a normal propose
/// and commit onto a different mesh must equal a fresh remesher's remap
/// bitwise (no stale staging).
fn inject(req: &Value) -> CoreResult<Value> {
    let (base, physics) = physics_of(req)?;
    let model = physics.model({
        let mut e = Vec::new();
        base.edges_into(&mut e);
        e
    })?;
    let layout = &model.layout;
    let y0 = vec_of(req, "initial_state")?;
    if y0.len() != layout.n() {
        return Err(CoreError::Dimension(
            "initial_state must have the base layout's length".into(),
        ));
    }
    let t = f64_of(req, "t")?;
    let refine = bool_or(req, "refine", false);
    let x = threshold_position(&model, t).ok_or_else(|| bad("inject needs an absorber"))?;
    let max_cells = base.n + 2;
    let mut new_edges = Vec::with_capacity(max_cells + 1);
    let face = aligned_edges(base.lo, base.hi, base.n, x, refine, &mut new_edges);
    let mut other_edges = Vec::with_capacity(max_cells + 1);
    let other_face = aligned_edges(base.lo, base.hi, base.n, x, !refine, &mut other_edges);
    let mut remesher =
        Remesher::new(&layout.q_edges, layout.nm, max_cells, 0).map_err(refusal_core)?;
    const SENTINEL: f64 = -7.25;
    let mut y = vec![SENTINEL; remesher.state_len(max_cells)];
    let y_bits: Vec<u64> = y.iter().map(|v| v.to_bits()).collect();
    let edges_bits: Vec<u64> = remesher.edges().iter().map(|v| v.to_bits()).collect();
    let (epoch_before, work_before) = (remesher.epoch(), remesher.work);
    let before = allocations();
    let injected = remesher.propose(layout, &y0, &new_edges, true);
    let commit_after_refusal = remesher.commit(&mut y);
    let inject_allocations = allocations() - before;
    let work_after = remesher.work;
    let buffer_unchanged = y.iter().map(|v| v.to_bits()).eq(y_bits.iter().copied());
    let edges_unchanged = remesher
        .edges()
        .iter()
        .map(|v| v.to_bits())
        .eq(edges_bits.iter().copied());
    let epoch_unchanged = remesher.epoch() == epoch_before;
    let work_increased = work_after.attempts == work_before.attempts + 1
        && work_after.refusals == work_before.refusals + 1
        && work_after.commits == work_before.commits
        && work_after.overlap_segments > work_before.overlap_segments;
    let before = allocations();
    let normal = remesher.propose(layout, &y0, &other_edges, false);
    let committed = normal.map(|_| remesher.commit(&mut y));
    let normal_allocations = allocations() - before;
    let (report, epoch) = match (normal, committed) {
        (Ok(r), Ok(Ok(e))) => (r, e),
        (Err(r), _) | (Ok(_), Ok(Err(r))) | (_, Err(r)) => {
            return Ok(json!({"status":"NUMERICAL_FAILURE","reason":r.to_string()}));
        }
    };
    // a fresh remesher's remap of the same state onto the same mesh
    let mut fresh =
        Remesher::new(&layout.q_edges, layout.nm, max_cells, 0).map_err(refusal_core)?;
    let mut y_fresh = Vec::with_capacity(fresh.state_len(max_cells));
    fresh
        .propose(layout, &y0, &other_edges, false)
        .map_err(refusal_core)?;
    fresh.commit(&mut y_fresh).map_err(refusal_core)?;
    let equals_fresh = bits_equal(&y, &y_fresh) && bits_equal(remesher.edges(), fresh.edges());
    let ok = injected == Err(RemapRefusal::Injected)
        && buffer_unchanged
        && edges_unchanged
        && epoch_unchanged
        && work_increased
        && commit_after_refusal == Err(RemapRefusal::NothingStaged)
        && equals_fresh;
    Ok(
        json!({"status": if ok {"OK"} else {"NUMERICAL_FAILURE"},"mode":"inject",
            "timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"},
        "x_chi":x,"face":face,"new_edges":new_edges,"other_face":other_face,
        "other_edges":other_edges,
        "injected_refusal":format!("{injected:?}"),"buffer_unchanged":buffer_unchanged,
        "edges_unchanged":edges_unchanged,"epoch_unchanged":epoch_unchanged,
        "work_before":work_json(&work_before),"work_after_refusal":work_json(&work_after),
        "work_increased":work_increased,
        "commit_after_refusal":format!("{commit_after_refusal:?}"),
        "inject_allocations":inject_allocations,
        "normal":report_json(&report),"epoch":epoch,"work":work_json(&remesher.work),
        "normal_allocations":normal_allocations,"state_after":y,
        "equals_fresh_remesher":equals_fresh,
        "per_column_relative_change":column_changes(layout, &y0, &y, layout.nm)}),
    )
}

/// Per column (the angular columns, then the absorbed ledgers): relative
/// change of the column sum against the column's absolute mass.
fn column_changes(old: &Layout, y_old: &[f64], y_new: &[f64], nm: usize) -> Vec<f64> {
    let stride = nm + 1;
    let nq_new = (y_new.len() - 2) / stride;
    (0..stride)
        .map(|col| {
            let a: Vec<f64> = (0..old.nq).map(|i| y_old[1 + i * stride + col]).collect();
            let b: Vec<f64> = (0..nq_new).map(|i| y_new[1 + i * stride + col]).collect();
            let mass: f64 = a.iter().map(|v| v.abs()).sum();
            let d = (compensated_total(&b) - compensated_total(&a)).abs();
            if mass > 0.0 { d / mass } else { d }
        })
        .collect()
}

fn mode_of(req: &Value) -> CoreResult<Linear> {
    match req
        .get("linear")
        .and_then(Value::as_str)
        .unwrap_or("banded")
    {
        "banded" => Ok(Linear::Banded),
        "client_exact" => Ok(Linear::Client(Mode::Exact)),
        "client_krylov" => {
            let k = req.get("krylov").ok_or_else(|| bad("missing krylov"))?;
            Ok(Linear::Client(Mode::Krylov {
                restart: usize_of(k, "restart")?,
                max_iterations: usize_of(k, "max_iterations")?,
                rtol: f64_of(k, "rtol")?,
                two_level: bool_or(k, "two_level", true),
            }))
        }
        _ => Err(bad("linear must be banded, client_exact or client_krylov")),
    }
}

fn config_of(req: &Value) -> CoreResult<RunConfig> {
    let (base, physics) = physics_of(req)?;
    let span = vec_of(req, "t_span")?;
    if span.len() != 2 {
        return Err(bad("t_span must have 2 entries"));
    }
    let (t0, tf) = (span[0], span[1]);
    let rtol = f64_of(req, "rtol")?;
    let arm = match req.get("arm").and_then(Value::as_str) {
        Some("static") => Arm::Static,
        Some("hook") => Arm::Hook {
            trigger: f64_of(req, "trigger")?,
        },
        Some("schedule") => Arm::Schedule {
            times: vec_of(req, "remap_times")?,
        },
        Some("fixed_schedule") => Arm::FixedSchedule {
            times: vec_of(req, "remap_times")?,
            steps: usize_of(req, "fixed_steps")?,
        },
        _ => return Err(bad("arm must be static, hook, schedule or fixed_schedule")),
    };
    Ok(RunConfig {
        base,
        physics,
        y0: vec_of(req, "initial_state")?,
        t_span: (t0, tf),
        output_times: vec_of(req, "output_times")?,
        rtol,
        atol_scale: req
            .get("atol_scale")
            .and_then(Value::as_f64)
            .unwrap_or(1.0e-2 * rtol),
        initial_step: req
            .get("initial_step")
            .and_then(Value::as_f64)
            .unwrap_or(1.0e-4 * (tf - t0)),
        max_attempts: req
            .get("max_attempts")
            .and_then(Value::as_u64)
            .unwrap_or(1_000_000) as usize,
        linear: mode_of(req)?,
        arm,
        refine: bool_or(req, "refine", false),
        max_remaps: req
            .get("max_remaps")
            .and_then(Value::as_u64)
            .unwrap_or(4096) as usize,
        restart: match req
            .get("restart")
            .and_then(Value::as_str)
            .unwrap_or("h_next")
        {
            "h_next" => Restart::HNext,
            "fresh" => Restart::Fresh,
            _ => return Err(bad("restart must be h_next or fresh")),
        },
        inject_failure_at_remap: match req.get("inject_failure_at_remap") {
            None | Some(Value::Null) => None,
            Some(v) => Some(
                v.as_u64()
                    .filter(|k| *k >= 1)
                    .ok_or_else(|| bad("inject_failure_at_remap must be a positive integer"))?
                    as usize,
            ),
        },
        record_remap_states: bool_or(req, "record_remap_states", false),
    })
}

/// Mesh-independent scalars of a state.
fn summary(state: &[f64], cells: usize, nm: usize) -> Value {
    let stride = nm + 1;
    let (mut photons, mut absorbed) = (Vec::new(), Vec::new());
    for i in 0..cells {
        photons.extend_from_slice(&state[1 + i * stride..1 + i * stride + nm]);
        absorbed.push(state[1 + i * stride + nm]);
    }
    json!({"photons":compensated_total(&photons),"absorbed":compensated_total(&absorbed),
        "l_lo":state[0],"l_hi":state[state.len() - 1],"total":compensated_total(state)})
}

fn outcome_json(r: &RunOutcome, nm: usize) -> Value {
    let outputs: Vec<Value> = r
        .outputs
        .iter()
        .map(|o| {
            json!({"t":o.t,"epoch":o.epoch,"q_edges":o.q_edges,"state":o.state,
                "summary":summary(&o.state, o.q_edges.len() - 1, nm)})
        })
        .collect();
    let remaps: Vec<Value> = r
        .remaps
        .iter()
        .map(|e| {
            json!({"t":e.t,"epoch":e.epoch,"face":e.face,"h_restart":e.h_restart,
                "control_h_next":e.control_h_next,"attempts_before":e.attempts_before,
                "counters_before":e.counters_before,
                "relative_total_change":e.report.relative_total_change,
                "l1_change":e.report.l1_change,"overlap_segments":e.report.overlap_segments,
                "report":report_json(&e.report)})
        })
        .collect();
    let event = |e: TraceEvent| match e {
        TraceEvent::Start => "start",
        TraceEvent::Output => "output",
        TraceEvent::BeforeRemap => "before_remap",
        TraceEvent::AfterRemap => "after_remap",
        TraceEvent::End => "end",
    };
    let trace: Vec<Value> = r
        .trace
        .iter()
        .map(|p| json!({"t":p.t,"epoch":p.epoch,"total":p.total,"event":event(p.event)}))
        .collect();
    let total0 = r.trace.first().map(|p| p.total).unwrap_or(0.0);
    let drift = r
        .trace
        .iter()
        .map(|p| (p.total - total0).abs() / total0.abs())
        .fold(0.0_f64, f64::max);
    let remap_drift = r
        .remaps
        .iter()
        .map(|e| e.report.relative_total_change)
        .fold(0.0_f64, f64::max);
    let remap_states: Vec<Value> = r
        .remap_states
        .iter()
        .map(|s| {
            json!({"t":s.t,"old_edges":s.old_edges,"new_edges":s.new_edges,
                "y_before":s.y_before,"y_after":s.y_after})
        })
        .collect();
    json!({"outputs":outputs,"remaps":remaps,"trace":trace,"remap_states":remap_states,
        "invariant":{"total0":total0,"max_relative_drift":drift,
            "max_relative_change_per_remap":remap_drift},
        "remap_count":r.remaps.len(),"segments":r.segments,"attempts":r.attempts,
        "accepted_steps":r.accepted_steps,"rejected_steps":r.rejected_steps,
        "counters":r.counters,"remesh_work":work_json(&r.remesh),
        "client_ops":r.client_ops.as_ref().map(ops_json),"client_epoch":r.client_epoch,
        "final_t":r.final_t,"final_edges":r.final_edges,"final_epoch":r.final_epoch,
        "final_state":r.final_state,
        "final_summary":summary(&r.final_state, r.final_edges.len() - 1, nm)})
}

fn result(status: &str, reason: &str) -> Value {
    json!({"status":status,"reason":reason,"outputs":[],"counters":null,
        "timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"}})
}

fn failure(e: &CoreError) -> Value {
    let status = match e {
        CoreError::InvalidInput(_) | CoreError::Dimension(_) => "REFUSED",
        _ => "NUMERICAL_FAILURE",
    };
    result(status, &e.to_string())
}

fn run_failure(e: &RunError) -> Value {
    match e {
        RunError::Core(c) => failure(c),
        RunError::Remap { .. } => result("NUMERICAL_FAILURE", &e.to_string()),
    }
}

fn run() -> Value {
    let args: Vec<String> = env::args().collect();
    if !(args.len() == 3 || (args.len() == 5 && args[3] == "--measurement-mode"))
        || args[1] != "--request"
    {
        return result(
            "ENVIRONMENT_FAILURE",
            "usage: lrs_photon_remap_run --request PATH [--measurement-mode timing|allocation]",
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
    match req
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("integrate")
    {
        "cycle" => return cycle(&req).unwrap_or_else(|e| failure(&e)),
        "inject" => return inject(&req).unwrap_or_else(|e| failure(&e)),
        "integrate" => {}
        _ => return failure(&bad("mode must be integrate, cycle or inject")),
    }
    let cfg = match config_of(&req) {
        Ok(c) => c,
        Err(e) => return failure(&e),
    };
    let nm = cfg.physics.mu_edges.len() - 1;
    // One warm run, then the measured run (timing or allocation count).
    if let Err(e) = integrate_segments(&cfg) {
        return run_failure(&e);
    }
    let before = allocations();
    let start = Instant::now();
    let run = integrate_segments(&cfg);
    let elapsed = start.elapsed().as_nanos() as u64;
    let counted = allocations() - before;
    let control_before = allocations();
    let positive = vec![1_u8; 64];
    std::hint::black_box(&positive);
    let control = allocations() - control_before;
    match run {
        Ok(r) => {
            let mut v = outcome_json(&r, nm);
            v["status"] = json!(if r.success { "OK" } else { "NUMERICAL_FAILURE" });
            v["reason"] = json!(r.message);
            v["mode"] = json!("integrate");
            v["measurement_mode"] = json!(mode);
            v["timing"] = json!({"elapsed_ns": elapsed, "timing_authority": "diagnostic_only",
                "timing_scope": "second in-process integrate call incl. remaps, model builds and output"});
            v["allocations"] = if mode == "allocation" {
                json!({"integrate": counted, "positive_control": control, "calibrated": control > 0,
                    "segments": r.segments, "remaps": r.remaps.len(),
                    "scope": "second integrate call on the calling thread incl. remaps, restarts and output"})
            } else {
                Value::Null
            };
            v
        }
        Err(e) => run_failure(&e),
    }
}

fn main() {
    println!("{}", run());
}
