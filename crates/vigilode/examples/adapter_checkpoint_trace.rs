//! Same-checkpoint, same-interval attempt trace of the adapter's chosen method
//! and one alternative. Diagnostic only: it replays a registered fixture to
//! recover the accepted checkpoint, then steps DOPRI5 one attempt at a time
//! through a native session and RODAS5P through deterministic attempt-prefix
//! runs. True local error is measured against two tight independent
//! references; it is not the native embedded estimate. Wall time is
//! diagnostic only.
#[path = "local_bottleneck_support/models.rs"]
mod models;
use serde_json::{Value, json};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use vigilode::{
    Atol, DerivativeFallback, Dopri5, Jacobian, Options, Problem, Rodas5p, SessionAdvance,
    SessionEpochs, SwitchingConfig, SwitchingSolver,
};

struct CountingAlloc;
thread_local! {
    static COUNT: Cell<u64> = const { Cell::new(0) };
}
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;
fn allocations() -> u64 {
    COUNT.with(Cell::get)
}

// RHS evaluation times of the traced run. The buffer is reserved before
// tracing, so recording never allocates inside a counted interval.
static REC_ON: AtomicBool = AtomicBool::new(false);
static REC: Mutex<Vec<f64>> = Mutex::new(Vec::new());
fn rec(t: f64) {
    if REC_ON.load(Ordering::Relaxed)
        && let Ok(mut v) = REC.lock()
        && v.len() < v.capacity()
    {
        v.push(t);
    }
}
fn rec_reset(on: bool) {
    let mut v = REC.lock().unwrap();
    v.clear();
    REC_ON.store(on, Ordering::Relaxed);
}
fn rec_snapshot() -> Vec<f64> {
    REC.lock().unwrap().clone()
}

fn model(name: &str) -> (Problem, Vec<f64>, f64, Vec<f64>, f64, f64) {
    match name {
        "decay" => (
            Problem::new(1, |t, y, o| {
                rec(t);
                o[0] = -y[0];
                Ok(())
            })
            .jacobian(Jacobian::dense(|_, _, j| {
                j[0] = -1.0;
                Ok(())
            }))
            .autonomous(),
            vec![1.0],
            1.0,
            vec![(-1.0_f64).exp()],
            1e-5,
            1e-14,
        ),
        "rotation" => (
            Problem::new(2, |t, y, o| {
                rec(t);
                o[0] = -100.0 * y[1];
                o[1] = 100.0 * y[0];
                Ok(())
            })
            .jacobian(Jacobian::jvp(|_, _, v, o| {
                o[0] = -100.0 * v[1];
                o[1] = 100.0 * v[0];
                Ok(())
            }))
            .autonomous(),
            vec![1.0, 0.0],
            1.0,
            vec![100.0_f64.cos(), 100.0_f64.sin()],
            1e-5,
            1e-13,
        ),
        "stiff-forced" => (
            Problem::new(1, |t, y, o| {
                rec(t);
                o[0] = -1e4 * (y[0] - t.cos()) - t.sin();
                Ok(())
            })
            .jacobian(Jacobian::dense(|_, _, j| {
                j[0] = -1e4;
                Ok(())
            }))
            .time_derivative(|t, _, o| {
                o[0] = -1e4 * t.sin() - t.cos();
                Ok(())
            }),
            vec![1.0],
            1.0,
            vec![1.0_f64.cos()],
            1e-5,
            1e-14,
        ),
        "relaxation" => (
            Problem::new(1, |t, y, o| {
                rec(t);
                let l = 1.0 + 4999.5 * (1.0 - (40.0 * (t - 0.4)).tanh());
                o[0] = -l * (y[0] - t.cos()) - t.sin();
                Ok(())
            })
            .jacobian(Jacobian::dense(|t, _, j| {
                j[0] = -(1.0 + 4999.5 * (1.0 - (40.0 * (t - 0.4)).tanh()));
                Ok(())
            }))
            .time_derivative(|t, y, o| {
                let q = (40.0 * (t - 0.4)).tanh();
                let l = 1.0 + 4999.5 * (1.0 - q);
                let lp = -199980.0 * (1.0 - q * q);
                o[0] = -lp * (y[0] - t.cos()) - l * t.sin() - t.cos();
                Ok(())
            }),
            vec![1.0],
            1.0,
            vec![1.0_f64.cos()],
            1e-5,
            1e-14,
        ),
        "case5" => {
            let reference:Value=serde_json::from_str(include_str!(
                "../../../research/local_frontier_native_diffsol_20261024/stop_audit_20261025/evidence/case_references/case05/EXTERNAL_REFERENCE_PRECOMMIT.json")).unwrap();
            (
                Problem::new(2, |t, y, o| {
                    rec(t);
                    o[1] = -y[1] / (1.0 + t) + 0.2 * y[0];
                    o[0] = -(y[0] - y[1].cos()) / 1e-6 - y[1].sin() * o[1];
                    Ok(())
                })
                .jacobian(Jacobian::dense(|t, y, j| {
                    let f = -y[1] / (1.0 + t) + 0.2 * y[0];
                    j[0] = -1e6 - 0.2 * y[1].sin();
                    j[1] = -1e6 * y[1].sin() - y[1].cos() * f + y[1].sin() / (1.0 + t);
                    j[2] = 0.2;
                    j[3] = -1.0 / (1.0 + t);
                    Ok(())
                }))
                .time_derivative(|t, y, o| {
                    o[1] = y[1] / (1.0 + t).powi(2);
                    o[0] = -y[1].sin() * o[1];
                    Ok(())
                }),
                vec![0.0, 1.0],
                2.0,
                reference["reference_state"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect(),
                1e-4,
                1e-12,
            )
        }
        // Stiff relaxation to a smooth path, then a nonstiff accuracy-limited
        // oscillation. The exact solution is y=g(t) for every lambda(t), so
        // the implicit history is cheap while the current explicit route wins.
        "stale-return" => {
            fn lam(t: f64) -> (f64, f64) {
                let q = (200.0 * (t - 0.5)).tanh();
                (1.0 + 4999.5 * (1.0 - q), -4999.5 * 200.0 * (1.0 - q * q))
            }
            fn g(t: f64) -> (f64, f64, f64) {
                let (a, w) = (1e-2, 150.0);
                let q = (200.0 * (t - 0.5)).tanh();
                let (s, s1, s2) = (
                    0.5 * (1.0 + q),
                    100.0 * (1.0 - q * q),
                    -40000.0 * q * (1.0 - q * q),
                );
                let (sn, cs) = (w * t).sin_cos();
                (
                    t.cos() + a * s * sn,
                    -t.sin() + a * (s1 * sn + s * w * cs),
                    -t.cos() + a * (s2 * sn + 2.0 * s1 * w * cs - s * w * w * sn),
                )
            }
            let (g1, _, _) = g(1.0);
            (
                Problem::new(1, |t, y, o| {
                    rec(t);
                    let (l, _) = lam(t);
                    let (g0, g1, _) = g(t);
                    o[0] = -l * (y[0] - g0) + g1;
                    Ok(())
                })
                .jacobian(Jacobian::dense(|t, _, j| {
                    j[0] = -lam(t).0;
                    Ok(())
                }))
                .time_derivative(|t, y, o| {
                    let (l, lp) = lam(t);
                    let (g0, g1, g2) = g(t);
                    o[0] = -lp * (y[0] - g0) + l * g1 + g2;
                    Ok(())
                }),
                vec![g(0.0).0],
                1.0,
                vec![g1],
                1e-5,
                1e-14,
            )
        }
        _ if name.starts_with("transition-") => {
            let tau: f64 = name
                .strip_prefix("transition-")
                .unwrap()
                .parse::<f64>()
                .unwrap()
                / 100.0;
            (
                Problem::new(2, move |t, y, o| {
                    rec(t);
                    let l = 5e4 * (1.0 + (60.0 * (t - tau)).tanh());
                    let (s, c) = (100.0 * t).sin_cos();
                    o[0] = -100.0 * y[1] - l * (y[0] - c);
                    o[1] = 100.0 * y[0] - l * (y[1] - s);
                    Ok(())
                })
                .jacobian(Jacobian::jvp(move |t, _, v, o| {
                    let l = 5e4 * (1.0 + (60.0 * (t - tau)).tanh());
                    o[0] = -100.0 * v[1] - l * v[0];
                    o[1] = 100.0 * v[0] - l * v[1];
                    Ok(())
                }))
                .time_derivative(move |t, y, o| {
                    let q = (60.0 * (t - tau)).tanh();
                    let l = 5e4 * (1.0 + q);
                    let lp = 3e6 * (1.0 - q * q);
                    let (s, c) = (100.0 * t).sin_cos();
                    o[0] = -lp * (y[0] - c) - 100.0 * l * s;
                    o[1] = -lp * (y[1] - s) + 100.0 * l * c;
                    Ok(())
                }),
                vec![1.0, 0.0],
                1.0,
                vec![100.0_f64.cos(), 100.0_f64.sin()],
                1e-5,
                1e-13,
            )
        }
        _ => panic!("unknown case {name}"),
    }
}

// Reuse the published nonlinear fixtures, including the directional dual
// arithmetic. Dense and matrix-free case5 controls differ only in provider.
fn catalog_problem(m: &models::Model, dense: bool) -> Problem {
    let mr = m.clone();
    let mut p = Problem::new(m.n, move |t, y, out| {
        rec(t);
        mr.apply(t, y, None, false, out, false);
        Ok(())
    });
    let mj = m.clone();
    if dense {
        p = p.jacobian(Jacobian::dense(move |t, y, a| {
            // These tiny fixed fixtures have n<=6. Avoid provider allocation
            // differences while preserving exactly the same dual derivatives.
            let mut v = [0.0; 6];
            let mut col = [0.0; 6];
            for j in 0..mj.n {
                v[j] = 1.0;
                mj.apply(t, y, Some(&v[..mj.n]), false, &mut col[..mj.n], false);
                for i in 0..mj.n {
                    a[i * mj.n + j] = col[i];
                }
                v[j] = 0.0;
            }
            Ok(())
        }));
    } else {
        p = p.jacobian(Jacobian::jvp(move |t, y, v, out| {
            mj.apply(t, y, Some(v), false, out, false);
            Ok(())
        }));
    }
    if m.autonomous() {
        p.autonomous()
    } else {
        let mt = m.clone();
        p.time_derivative(move |t, y, out| {
            mt.apply(t, y, None, true, out, false);
            Ok(())
        })
    }
}

struct Fixture {
    problem: Problem,
    dense_problem: Option<Problem>,
    // Diagnostic control only: the same RHS with the facade's default
    // finite-difference Jacobian (no provider), for allocation audits.
    fd_problem: Option<Problem>,
    y0: Vec<f64>,
    end: f64,
    reference: Vec<f64>,
    cap: f64,
    uncertainty: f64,
    options: Options,
    report_only: bool,
}

fn fixture(case: &str) -> Fixture {
    let gyro = case.starts_with("gyro-");
    let (mut problem, mut y0, mut end, mut reference, mut cap, mut uncertainty) =
        if gyro { model("decay") } else { model(case) };
    let mut dense_problem = None;
    let mut fd_problem = None;
    let mut options = Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-7,
        max_steps: 100_000,
        ..Default::default()
    };
    if case.starts_with("transition-") || case == "relaxation" || case == "stale-return" {
        options.h0 = 1e-4;
    }
    if case == "case5" || gyro {
        let m = models::Model::new(if gyro { 10 } else { 5 }, false).unwrap();
        problem = catalog_problem(&m, false);
        dense_problem = Some(catalog_problem(&m, true));
        let (mr, mt) = (m.clone(), m.clone());
        let fd = Problem::new(m.n, move |t, y, out| {
            rec(t);
            mr.apply(t, y, None, false, out, false);
            Ok(())
        });
        fd_problem = Some(if m.autonomous() {
            fd.autonomous()
        } else {
            fd.time_derivative(move |t, y, out| {
                mt.apply(t, y, None, true, out, false);
                Ok(())
            })
        });
        y0 = m.initial();
        end = m.end;
        cap = 1e-4;
        options.h0 = 1e-4;
        options.rtol = 1e-6;
        options.atol = Atol::Scalar(1e-9);
        if gyro {
            reference = m.exact(end).unwrap();
            uncertainty = 1e-13;
        }
        if case == "gyro-tight" {
            options.rtol = 1e-9;
            options.atol = Atol::Scalar(1e-12);
        }
    }
    Fixture {
        problem,
        dense_problem,
        fd_problem,
        y0,
        end,
        reference,
        cap,
        uncertainty,
        options,
        report_only: case == "gyro-default",
    }
}

fn scaled(o: &Options) -> f64 {
    match &o.atol {
        Atol::Scalar(a) => *a,
        _ => panic!("trace fixtures use scalar atol"),
    }
}

// Tight references from the accepted start of one step over its exact trial.
// Two independent methods bound the reference uncertainty.
fn reference(f: &Fixture, t0: f64, t1: f64, y0: &[f64]) -> (Vec<f64>, f64) {
    let was = REC_ON.swap(false, Ordering::Relaxed);
    let tight = |rtol: f64| Options {
        rtol,
        atol: Atol::Scalar(scaled(&f.options) * 1e-4),
        h0: ((t1 - t0) * 1e-3).max(1e-14),
        max_steps: 2_000_000,
        ..Default::default()
    };
    let p = f.dense_problem.as_ref().unwrap_or(&f.problem);
    let a = Rodas5p::solve(p, (t0, t1), y0, &tight(1e-12))
        .ok()
        .and_then(|r| r.is_success().then(|| r.final_state().unwrap().to_vec()));
    let b = Dopri5::solve(&f.problem, (t0, t1), y0, &tight(1e-12))
        .ok()
        .and_then(|r| r.success.then_some(r.final_state));
    REC_ON.store(was, Ordering::Relaxed);
    match (a, b) {
        (Some(a), Some(b)) => {
            let d = a
                .iter()
                .zip(&b)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0, f64::max);
            (a, d)
        }
        (Some(a), None) => (a, f64::NAN),
        (None, Some(b)) => (b, f64::NAN),
        (None, None) => (vec![f64::NAN; y0.len()], f64::NAN),
    }
}

fn local_error(f: &Fixture, t0: f64, t1: f64, y0: &[f64], y1: &[f64]) -> Value {
    let (r, uncertainty) = reference(f, t0, t1, y0);
    let (atol, rtol) = (scaled(&f.options), f.options.rtol);
    let mut max = 0.0_f64;
    let mut sum = 0.0;
    for i in 0..y1.len() {
        let w = atol + rtol * y0[i].abs().max(y1[i].abs());
        let e = (y1[i] - r[i]).abs() / w;
        max = max.max(e);
        sum += e * e;
    }
    json!({"true_local_error_ratio_max": max,
        "true_local_error_ratio_rms": (sum / y1.len() as f64).sqrt(),
        "reference_disagreement_abs": uncertainty})
}

fn work_units(c: &vigilode::WorkRecord, n: usize) -> f64 {
    c.rhs_calls as f64
        + c.jvp_vectors as f64
        + c.ft_calls as f64
        + c.jacobian_builds as f64 * n as f64
        + c.linear_solves as f64
}

fn counters_delta(a: &vigilode::WorkRecord, b: &vigilode::WorkRecord) -> Value {
    json!({"rhs": b.rhs_calls - a.rhs_calls, "jvp_vectors": b.jvp_vectors - a.jvp_vectors,
        "jacobian_builds": b.jacobian_builds - a.jacobian_builds, "ft": b.ft_calls - a.ft_calls,
        "linear_solves": b.linear_solves - a.linear_solves,
        "linear_iterations": b.linear_iterations - a.linear_iterations,
        "linear_matvecs": b.linear_matvecs - a.linear_matvecs,
        "direct_factorizations": b.direct_factorizations - a.direct_factorizations,
        "direct_solve_calls": b.direct_solve_calls - a.direct_solve_calls,
        "local_error_failures": b.local_error_failures - a.local_error_failures,
        "linear_solve_failures": b.linear_solve_failures - a.linear_solve_failures,
        "nonlinear_solve_failures": b.nonlinear_solve_failures - a.nonlinear_solve_failures,
        "nonfinite_step_failures": b.nonfinite_step_failures - a.nonfinite_step_failures})
}

fn reason(d: &Value, accepted: bool) -> &'static str {
    if accepted {
        return "accepted";
    }
    for (k, r) in [
        ("linear_solve_failures", "linear-solve failure"),
        ("nonlinear_solve_failures", "nonlinear failure"),
        ("nonfinite_step_failures", "nonfinite stage"),
        ("local_error_failures", "local error test"),
    ] {
        if d[k].as_u64().unwrap_or(0) > 0 {
            return r;
        }
    }
    "rejected (error test; driver does not split the reason)"
}

// One attempt at a time through the native session used by switching probes.
fn trace_dopri(
    f: &Fixture,
    start: f64,
    target: f64,
    end: f64,
    y: &[f64],
    h0: f64,
    max_attempts: usize,
) -> Value {
    let mut o = f.options.clone();
    o.h0 = h0;
    o.max_steps = max_attempts;
    let times = if target < end {
        vec![start, target, end]
    } else {
        vec![start, target]
    };
    let epochs = SessionEpochs::default();
    rec_reset(true);
    let mut rows = Vec::new();
    let a0 = allocations();
    let t_total = Instant::now();
    let (_, sol) = Dopri5::with_session(&f.problem, (start, end), y, &o, &times, epochs, |s| {
        loop {
            let (t_before, h_req, acc0, rej0, c0) = {
                let c = s.checkpoint();
                (
                    c.t,
                    c.proposed_h,
                    c.accepted_steps,
                    c.rejected_steps,
                    vigilode::WorkRecord::from(c.counters),
                )
            };
            let y_before = s.checkpoint().y.to_vec();
            let calls0 = REC.lock().unwrap().len();
            let al = allocations();
            let clock = Instant::now();
            let adv = s.advance_to(target, 1, epochs);
            let ns = clock.elapsed().as_nanos() as u64;
            let al = allocations() - al;
            let c = s.checkpoint();
            let (acc, rej) = (c.accepted_steps - acc0, c.rejected_steps - rej0);
            if acc + rej == 0 {
                break;
            }
            let calls = REC.lock().unwrap()[calls0..].to_vec();
            let stage_max = calls.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let trial = stage_max - t_before;
            let d = counters_delta(&c0, &vigilode::WorkRecord::from(c.counters));
            let mut row = json!({"t_before": t_before, "h_proposed": h_req, "h_trial": trial,
                "landing_clipped": trial < h_req * (1.0 - 1e-12), "accepted": acc == 1,
                "reason": reason(&d, acc == 1), "t_after": c.t, "next_proposal": c.proposed_h,
                "work": d, "allocations": al, "attempt_ns_diagnostic": ns});
            if acc == 1 {
                let y_after = c.y.to_vec();
                row["error"] = local_error(f, t_before, c.t, &y_before, &y_after);
            }
            rows.push(row);
            if !matches!(adv, Ok(SessionAdvance::Yield)) || c.t >= target {
                break;
            }
        }
        Ok::<(), vigilode::ExplicitSolveError>(())
    })
    .expect("session");
    let total_ns = t_total.elapsed().as_nanos() as u64;
    rec_reset(false);
    let c = vigilode::WorkRecord::from(&sol.stats.counters);
    json!({"method":"dopri5","h0":h0,"reached":sol.final_time,"reached_target": sol.final_time >= target,
        "attempts": sol.stats.attempts, "accepted": sol.stats.accepted_steps,
        "rejected": sol.stats.rejected_steps, "rhs": c.rhs_calls, "jvp_vectors": c.jvp_vectors,
        "linear_solves": c.linear_solves, "work_units": work_units(&c, y.len()),
        "allocations_total": allocations() - a0, "total_ns_diagnostic_incl_reference": total_ns,
        "attempt_rows": rows})
}

// Deterministic prefix runs: run k stops after attempt k. Attempt k is the
// difference between prefixes k and k-1, all from the same checkpoint.
fn trace_rodas(
    f: &Fixture,
    start: f64,
    target: f64,
    y: &[f64],
    h0: f64,
    max_attempts: usize,
    dense: bool,
) -> Value {
    let p = if dense {
        f.dense_problem.as_ref().expect("dense control")
    } else {
        &f.problem
    };
    let mut o = f.options.clone();
    o.h0 = h0;
    let mut rows = Vec::new();
    let mut prev_t = start;
    let mut prev_y = y.to_vec();
    let mut prev_c = vigilode::WorkRecord::default();
    let mut prev_calls = 0usize;
    let mut prev_alloc = 0u64;
    let mut prev_ns = 0u64;
    let mut prev_acc = 0usize;
    let mut last = None;
    for k in 1..=max_attempts {
        o.max_steps = k;
        let mut best_ns = u64::MAX;
        let mut out = None;
        for rep in 0..3 {
            rec_reset(rep == 0);
            let al = allocations();
            let clock = Instant::now();
            let r = Rodas5p::solve(p, (start, target), y, &o).expect("rodas");
            best_ns = best_ns.min(clock.elapsed().as_nanos() as u64);
            if rep == 0 {
                out = Some((r, allocations() - al, rec_snapshot()));
            }
        }
        rec_reset(false);
        let (r, alloc, calls) = out.unwrap();
        if r.stats.attempts < k {
            break;
        }
        let c = r.stats.counters;
        let t_after = *r.t.last().unwrap();
        let acc = r.stats.accepted_steps - prev_acc;
        let attempt_calls = &calls[prev_calls.min(calls.len())..];
        let stage_max = attempt_calls
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let trial = stage_max - prev_t;
        let d = counters_delta(&prev_c, &c);
        let clipped = acc == 1 && t_after == target;
        let mut row = json!({"t_before": prev_t, "h_trial": trial,
            "h_proposed": if k == 1 { json!(h0) } else if !clipped && acc == 1 { json!(trial) } else { Value::Null },
            "landing_clipped": clipped, "accepted": acc == 1, "reason": reason(&d, acc == 1),
            "t_after": t_after, "work": d,
            "allocations_marginal": alloc as i64 - prev_alloc as i64,
            "attempt_ns_marginal_diagnostic": best_ns as i64 - prev_ns as i64});
        let y_after = r
            .final_state()
            .map(<[f64]>::to_vec)
            .unwrap_or_else(|| prev_y.clone());
        if acc == 1 {
            row["error"] = local_error(f, prev_t, t_after, &prev_y, &y_after);
            prev_y = y_after;
        }
        rows.push(row);
        prev_t = t_after;
        prev_c = c;
        prev_calls = calls.len();
        prev_alloc = alloc;
        prev_ns = best_ns;
        prev_acc = r.stats.accepted_steps;
        let done = r.is_success();
        last = Some((r, alloc, best_ns));
        if done {
            break;
        }
    }
    let (r, alloc, ns) = last.expect("at least one attempt");
    let c = r.stats.counters;
    json!({"method": if dense {"rodas5p-dense"} else {"rodas5p"}, "h0": h0,
        "reached": r.t.last(), "reached_target": r.is_success(),
        "attempts": r.stats.attempts, "accepted": r.stats.accepted_steps,
        "rejected": r.stats.rejected_steps, "rhs": c.rhs_calls, "jvp_vectors": c.jvp_vectors,
        "linear_solves": c.linear_solves, "direct_factorizations": c.direct_factorizations,
        "linear_iterations": c.linear_iterations, "work_units": work_units(&c, y.len()),
        "allocations_total": alloc, "total_ns_min_of_3_diagnostic": ns, "attempt_rows": rows})
}

// The solver call of one arm alone, result dropped (timed/profiled path).
fn solve_only(f: &Fixture, arm: &str) {
    let span = (0.0, f.end);
    match arm {
        "dopri" => {
            std::hint::black_box(
                Dopri5::solve(&f.problem, span, &f.y0, &f.options).expect("dopri"),
            );
        }
        "rodas" | "rodas-dense" | "rodas-fd" => {
            let p = match arm {
                "rodas-dense" => f.dense_problem.as_ref().unwrap_or(&f.problem),
                "rodas-fd" => f.fd_problem.as_ref().expect("fd control"),
                _ => &f.problem,
            };
            std::hint::black_box(Rodas5p::solve(p, span, &f.y0, &f.options).expect("rodas"));
        }
        _ => {
            let c = switching_config(arm);
            std::hint::black_box(
                SwitchingSolver::solve(&f.problem, span, &f.y0, &f.options, &c).expect("switching"),
            );
        }
    }
}

fn switching_config(arm: &str) -> SwitchingConfig {
    let mut c = if arm == "evidence" {
        SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
    } else {
        SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    };
    if arm == "adaptive-sentinel" {
        c.return_sentinel_interval = 1;
    }
    c
}

// One registered arm: (rhs, max component error, work summary).
fn solve_arm(f: &Fixture, arm: &str) -> (u64, f64, Value) {
    let span = (0.0, f.end);
    let err = |y: &[f64]| {
        y.iter()
            .zip(&f.reference)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max)
    };
    match arm {
        "dopri" => {
            let r = Dopri5::solve(&f.problem, span, &f.y0, &f.options).expect("dopri");
            let c = vigilode::WorkRecord::from(&r.stats.counters);
            (
                c.rhs_calls,
                err(&r.final_state),
                json!({"attempts": r.stats.attempts,
                "work_units": work_units(&c, f.y0.len()), "success": r.success}),
            )
        }
        "rodas" | "rodas-dense" | "rodas-fd" => {
            let p = match arm {
                "rodas-dense" => f.dense_problem.as_ref().unwrap_or(&f.problem),
                "rodas-fd" => f.fd_problem.as_ref().expect("fd control"),
                _ => &f.problem,
            };
            let r = Rodas5p::solve(p, span, &f.y0, &f.options).expect("rodas");
            let c = r.stats.counters;
            (
                c.rhs_calls,
                err(r.final_state().unwrap_or(&[])),
                json!({"attempts": r.stats.attempts,
                "work_units": work_units(&c, f.y0.len()), "jvp_vectors": c.jvp_vectors,
                "linear_solves": c.linear_solves, "linear_iterations": c.linear_iterations,
                "direct_factorizations": c.direct_factorizations, "success": r.is_success()}),
            )
        }
        _ => {
            let c = switching_config(arm);
            let r =
                SwitchingSolver::solve(&f.problem, span, &f.y0, &f.options, &c).expect("switching");
            let mut kinds = serde_json::Map::new();
            for s in &r.stats.segments {
                let k = format!(
                    "{}{}{}",
                    if s.is_probe { "probe-" } else { "" },
                    if s.method == vigilode::SwitchingMethod::Dopri5 {
                        "dopri"
                    } else {
                        "rodas"
                    },
                    if s.committed {
                        "-committed"
                    } else {
                        "-discarded"
                    }
                );
                let e = kinds
                    .entry(k)
                    .or_insert(json!({"segments": 0, "attempts": 0, "rhs": 0,
                    "jvp": 0, "linear_solves": 0, "work_units": 0.0}));
                e["segments"] = json!(e["segments"].as_u64().unwrap() + 1);
                e["attempts"] = json!(e["attempts"].as_u64().unwrap() + s.attempts as u64);
                e["rhs"] = json!(e["rhs"].as_u64().unwrap() + s.rhs_calls);
                e["jvp"] = json!(e["jvp"].as_u64().unwrap() + s.jvp_vectors);
                e["linear_solves"] = json!(e["linear_solves"].as_u64().unwrap() + s.linear_solves);
                e["work_units"] = json!(e["work_units"].as_f64().unwrap() + s.work_units);
            }
            (
                r.stats.rhs_calls,
                err(&r.final_state),
                json!({"attempts": r.stats.attempts,
                "segments": r.stats.segments.len(), "switches": r.stats.switches,
                "output_rows": r.t.len(), "success": r.success, "by_kind": kinds}),
            )
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 2 {
        eprintln!("usage: adapter_checkpoint_trace CASE ARM [START TARGET DOPRI_H0 RODAS_H0]...");
        std::process::exit(2);
    }
    if a[1] == "bench" || a[1] == "repeat-arm" {
        // Light benchmark / profiler loop of one registered arm. `bench`
        // calibrates to >= 50 ms batches and reports seven batches;
        // `repeat-arm` runs a fixed count for an external profiler.
        let f = fixture(&a[2]);
        let arm = a[3].as_str();
        // Only the solver call is timed or profiled; the summary is built
        // once, outside every timed batch (harness v2).
        let once = || {
            let a0 = allocations();
            solve_only(&f, arm);
            allocations() - a0
        };
        if a[1] == "repeat-arm" {
            let n: usize = a[4].parse().unwrap();
            let clock = Instant::now();
            for _ in 0..n {
                once();
            }
            eprintln!(
                "repeat-arm {} {} n={} ns/solve={}",
                a[2],
                arm,
                n,
                clock.elapsed().as_nanos() as f64 / n as f64
            );
            return;
        }
        let cold = once();
        let warm = once();
        let (rhs, err, summary) = solve_arm(&f, arm);
        let mut n = 1usize;
        loop {
            let clock = Instant::now();
            for _ in 0..n {
                once();
            }
            if clock.elapsed().as_secs_f64() >= 0.05 || n >= 1 << 20 {
                break;
            }
            n *= 2;
        }
        let mut per = Vec::new();
        for _ in 0..7 {
            let clock = Instant::now();
            for _ in 0..n {
                once();
            }
            per.push(clock.elapsed().as_nanos() as f64 / n as f64);
        }
        let mut sorted = per.clone();
        sorted.sort_by(f64::total_cmp);
        println!(
            "{}",
            json!({"schema": "vigilode.adapter-light-bench.v2", "case": a[2], "arm": arm,
            "repetitions_per_batch": n, "ns_per_solve_batches": per, "ns_per_solve_median": sorted[3],
            "ns_per_solve_min": sorted[0], "batch_spread": (sorted[6] - sorted[0]) / sorted[3],
            "allocations_cold_first_call": cold, "allocations_warm": warm, "rhs": rhs,
            "max_component_absolute_error": err, "accuracy_cap": f.cap, "summary": summary,
            "timing_scope": "solver call only; result destruction included; summary built outside timing",
            "timing_authority": "diagnostic_only"})
        );
        return;
    }
    if a[1] == "repeat-rodas" {
        // Allocation-site profiling aid: one fixture, one provider, K attempts, R repeats.
        let f = fixture(&a[2]);
        let mut o = f.options.clone();
        o.max_steps = a[4].parse().unwrap();
        let p = match a[3].as_str() {
            "dense" => f.dense_problem.as_ref().unwrap(),
            "fd" => f.fd_problem.as_ref().unwrap(),
            _ => &f.problem,
        };
        let mut attempts = 0;
        for _ in 0..a[5].parse::<usize>().unwrap() {
            let a0 = allocations();
            let r = Rodas5p::solve(p, (0.0, f.end), &f.y0, &o).expect("rodas");
            attempts = r.stats.attempts;
            eprintln!("attempts={} allocations={}", attempts, allocations() - a0);
        }
        println!("{attempts}");
        return;
    }
    let (case, arm) = (a[1].as_str(), a[2].as_str());
    {
        let mut v = REC.lock().unwrap();
        v.reserve(1 << 24);
    }
    let f = fixture(case);
    let mut config = if arm.starts_with("adaptive") {
        SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    } else {
        SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
    };
    if arm == "adaptive-unseeded" {
        config.reuse_entry_pilot_step = false;
    }
    if arm == "adaptive-no-startup-retry" {
        config.retry_unresolved_entry_once = false;
    }
    let sol = SwitchingSolver::solve(&f.problem, (0.0, f.end), &f.y0, &f.options, &config)
        .expect("switching rerun");
    if a.get(3).map(String::as_str) == Some("tenure") {
        // Uninterrupted RODAS from a committed checkpoint over a whole tenure:
        // a lower bound on the work a fully preserved session could reach.
        let p: Vec<f64> = a[4..7].iter().map(|s| s.parse().unwrap()).collect();
        let i = sol
            .t
            .iter()
            .position(|t| *t == p[0])
            .expect("committed checkpoint");
        let mut o = f.options.clone();
        o.h0 = p[2];
        let a0 = allocations();
        let clock = Instant::now();
        let r = Rodas5p::solve(&f.problem, (p[0], p[1]), &sol.y[i], &o).expect("rodas");
        let ns = clock.elapsed().as_nanos() as u64;
        let c = r.stats.counters;
        println!(
            "{}",
            json!({"start": p[0], "end": p[1], "h0": p[2], "success": r.is_success(),
            "attempts": r.stats.attempts, "rejected": r.stats.rejected_steps, "rhs": c.rhs_calls,
            "jvp_vectors": c.jvp_vectors, "linear_solves": c.linear_solves,
            "work_units": work_units(&c, f.y0.len()), "allocations": allocations() - a0,
            "ns_diagnostic": ns})
        );
        return;
    }
    let mut points = Vec::new();
    for spec in a[3..].chunks(4) {
        let p: Vec<f64> = spec.iter().map(|s| s.parse().unwrap()).collect();
        let (start, target, hd, hr) = (p[0], p[1], p[2], p[3]);
        let Some(i) = sol.t.iter().position(|t| *t == start) else {
            points.push(json!({"start": start, "error": "checkpoint not among committed rows"}));
            continue;
        };
        let y = sol.y[i].clone();
        let mut point = json!({"start": start, "target": target, "checkpoint_state": y,
            "dopri": trace_dopri(&f, start, target, f.end, &y, hd, f.options.max_steps),
            "rodas": trace_rodas(&f, start, target, &y, hr, 400, false)});
        if f.dense_problem.is_some() {
            point["rodas_dense_unavailable_to_selector"] =
                trace_rodas(&f, start, target, &y, hr, 400, true);
        }
        points.push(point);
    }
    println!(
        "{}",
        json!({"schema": "vigilode.adapter-checkpoint-trace.v1", "case": case, "arm": arm,
            "declared_reference_uncertainty": f.uncertainty, "reference_report_only": f.report_only,
        "rerun": {"rhs": sol.stats.rhs_calls, "attempts": sol.stats.attempts,
            "final_state": sol.final_state, "segments": sol.stats.segments},
        "points": points,
        "error_scope": "true local error of accepted steps against tight RODAS and DOPRI references from the same accepted start; not the native embedded estimate; rejected trials carry no state",
        "proposal_scope": "DOPRI proposals are native session checkpoints; RODAS proposals are known only for the first attempt and unclipped accepted attempts",
        "timing_authority": "diagnostic_only"})
    );
}
