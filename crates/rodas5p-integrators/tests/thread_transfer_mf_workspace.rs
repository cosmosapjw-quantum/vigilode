//! JVP-only raw-stage matrix-free workspace driver (research node
//! `research/thread_transfer_mf_workspace_20261002`, thread-transfer DAG node
//! P1-MF-WORKSPACE). Run with `--ignored --test-threads=1`: the counting
//! allocator is global to this binary, and the run is long in a debug build.

#[path = "thread_transfer_common/mod.rs"]
mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::write_output;
use rodas5p_core::{
    CoreResult, DenseMatrix, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, KrylovState, OdeProblem, OutputSchedule,
    QuadraticModel, Rodas5pMfFastWorkspace, integrate_rodas5p_fast_observed,
    integrate_rodas5p_mf_fast_observed, integrate_sequential_matrix_free_adaptive_observed,
    prothero_robinson_problem, robertson_problem, rodas_next_step_after_attempt,
    sequential_matrix_free_step, stiff_van_der_pol_problem,
};
use serde_json::{Value, json};

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let value = f();
    (value, ALLOCATIONS.load(Ordering::Relaxed) - before)
}

/// JVPs spent outside the Krylov operator (right-hand-side assembly).
fn rhs_assembly_jvps(c: &WorkCounters) -> u64 {
    c.jvp_calls - (c.linear_matvecs + c.diagnostic_matvecs + c.recycle_refresh_matvecs)
}

fn hires() -> CoreResult<(OdeProblem, Vec<f64>)> {
    fn fill(y: &[f64], j: &mut DenseMatrix) {
        let entries = [
            (0, 0, -1.71),
            (0, 1, 0.43),
            (0, 2, 8.32),
            (1, 0, 1.71),
            (1, 1, -8.75),
            (2, 2, -10.03),
            (2, 3, 0.43),
            (2, 4, 0.035),
            (3, 1, 8.32),
            (3, 2, 1.71),
            (3, 3, -1.12),
            (4, 4, -1.745),
            (4, 5, 0.43),
            (4, 6, 0.43),
            (5, 3, 0.69),
            (5, 4, 1.71),
            (5, 5, -280.0 * y[7] - 0.43),
            (5, 6, 0.69),
            (5, 7, -280.0 * y[5]),
            (6, 5, 280.0 * y[7]),
            (6, 6, -1.81),
            (6, 7, 280.0 * y[5]),
            (7, 5, -280.0 * y[7]),
            (7, 6, 1.81),
            (7, 7, -280.0 * y[5]),
        ];
        for (a, b, v) in entries {
            j[(a, b)] = v;
        }
    }
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
        out[1] = 1.71 * y[0] - 8.75 * y[1];
        out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
        out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
        out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
        out[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
        out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
        out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
        Ok(())
    });
    let jacobian = Arc::new(|_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(8, 8);
        fill(y, &mut j);
        Ok(j)
    });
    let jvp = Arc::new(|_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let mut j = DenseMatrix::zeros(8, 8);
        fill(y, &mut j);
        j.matvec_into(v, out)
    });
    Ok((
        OdeProblem::new(
            "hires",
            8,
            rhs,
            None,
            Some(jacobian),
            Some(jvp),
            None,
            true,
            None,
            None,
        )?,
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
    ))
}

/// The 1-D Brusselator (Hairer-Wanner II, IV.1), interleaved `u, v`.
fn brusselator(cells: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let n = 2 * cells;
    let neighbours = move |y: &[f64], i: usize| {
        let (ul, vl) = if i == 0 {
            (1.0, 3.0)
        } else {
            (y[2 * i - 2], y[2 * i - 1])
        };
        let (ur, vr) = if i + 1 == cells {
            (1.0, 3.0)
        } else {
            (y[2 * i + 2], y[2 * i + 3])
        };
        (ul, vl, ur, vr)
    };
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl, ur, vr) = neighbours(y, i);
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
        }
        Ok(())
    });
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            j[(a, a)] = 2.0 * u * v - 4.0 - 2.0 * c;
            j[(a, b)] = u * u;
            j[(b, a)] = 3.0 - 2.0 * u * v;
            j[(b, b)] = -u * u - 2.0 * c;
            if i > 0 {
                j[(a, a - 2)] = c;
                j[(b, b - 2)] = c;
            }
            if i + 1 < cells {
                j[(a, a + 2)] = c;
                j[(b, b + 2)] = c;
            }
        }
        Ok(j)
    });
    let jvp = Arc::new(move |_t: f64, y: &[f64], w: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (du, dv) = (w[2 * i], w[2 * i + 1]);
            let (dul, dvl) = if i == 0 {
                (0.0, 0.0)
            } else {
                (w[2 * i - 2], w[2 * i - 1])
            };
            let (dur, dvr) = if i + 1 == cells {
                (0.0, 0.0)
            } else {
                (w[2 * i + 2], w[2 * i + 3])
            };
            out[2 * i] = (2.0 * u * v - 4.0) * du + u * u * dv + c * (dul - 2.0 * du + dur);
            out[2 * i + 1] = (3.0 - 2.0 * u * v) * du - u * u * dv + c * (dvl - 2.0 * dv + dvr);
        }
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect();
    Ok((
        OdeProblem::new(
            format!("brusselator-1d-{cells}"),
            n,
            rhs,
            None,
            Some(jacobian),
            Some(jvp),
            None,
            true,
            None,
            None,
        )?,
        y0,
    ))
}

/// `z' = a z + q z^2` per component (the R4 quadratic, n = 4), exact
/// solution `a z0 e^(at) / (a - q z0 (e^(at) - 1))`.
fn quadratic() -> (QuadraticModel, Vec<f64>) {
    let n = 4;
    let a = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { -1.0 - i as f64 } else { 0.0 })
                .collect()
        })
        .collect::<Vec<Vec<f64>>>();
    let q = (0..n)
        .map(|i| -0.05 * (1 + i % 3) as f64)
        .collect::<Vec<_>>();
    let y0 = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect();
    (QuadraticModel::new("quadratic-4", a, q).unwrap(), y0)
}

struct Case {
    id: &'static str,
    /// JVP-only clone.
    problem: OdeProblem,
    /// With an explicit Jacobian, for the reference run (if any).
    full: Option<OdeProblem>,
    exact: Option<Box<dyn Fn(f64) -> Vec<f64>>>,
    y0: Vec<f64>,
    t_span: (f64, f64),
    atol_scale: f64,
    /// One-step sizes.
    steps: [f64; 2],
}

fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    let (p, y0) = robertson_problem().unwrap();
    out.push(Case {
        id: "robertson",
        problem: p.jvp_only_clone().unwrap(),
        full: Some(p),
        exact: None,
        y0,
        t_span: (0.0, 40.0),
        atol_scale: 1.0e-4,
        steps: [1.0e-4, 1.0e-2],
    });
    let (p, y0) = stiff_van_der_pol_problem(1000.0).unwrap();
    out.push(Case {
        id: "van-der-pol-mu1000",
        problem: p.jvp_only_clone().unwrap(),
        full: Some(p),
        exact: None,
        y0,
        t_span: (0.0, 2000.0),
        atol_scale: 1.0,
        steps: [1.0e-4, 1.0e-2],
    });
    let (p, y0) = hires().unwrap();
    out.push(Case {
        id: "hires",
        problem: p.jvp_only_clone().unwrap(),
        full: Some(p),
        exact: None,
        y0,
        t_span: (0.0, 321.8122),
        atol_scale: 1.0e-4,
        steps: [1.0e-4, 1.0e-2],
    });
    let (p, y0) = brusselator(50).unwrap();
    out.push(Case {
        id: "brusselator-1d-50",
        problem: p.jvp_only_clone().unwrap(),
        full: Some(p),
        exact: None,
        y0,
        t_span: (0.0, 10.0),
        atol_scale: 1.0,
        steps: [1.0e-4, 1.0e-2],
    });
    let (p, y0) = prothero_robinson_problem(-1.0e4, 0.0, 0.0);
    out.push(Case {
        id: "prothero-robinson-forced",
        problem: p.jvp_only_clone().unwrap(),
        full: None,
        exact: Some(Box::new(|t: f64| vec![t.sin()])),
        y0,
        t_span: (0.0, 2.0),
        atol_scale: 1.0,
        steps: [1.0e-4, 1.0e-2],
    });
    let (model, y0) = quadratic();
    let a = model.a.clone();
    let q = model.q.clone();
    let z0 = y0.clone();
    out.push(Case {
        id: "quadratic-4",
        problem: model.ode_problem().unwrap(),
        full: None,
        exact: Some(Box::new(move |t: f64| {
            (0..z0.len())
                .map(|i| {
                    let (ai, qi, zi) = (a[i][i], q[i], z0[i]);
                    let e = (ai * t).exp();
                    ai * zi * e / (ai - qi * zi * (e - 1.0))
                })
                .collect()
        })),
        y0,
        t_span: (0.0, 0.5),
        atol_scale: 1.0,
        steps: [1.0e-4, 5.0e-2],
    });
    out
}

fn config(method: LinearMethod, rtol: f64) -> LinearSolverConfig {
    LinearSolverConfig {
        method,
        rtol,
        atol: 1.0e-14,
        preconditioner: PreconditionerKind::None,
        ..LinearSolverConfig::default()
    }
}

fn adaptive(rtol: f64, scale: f64, span: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * scale,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: span,
        // Enough for every completing run here (at most ~500 attempts); a
        // configuration that keeps failing stops instead of running for hours.
        max_attempts: 5_000,
        ..AdaptiveStepConfig::default()
    }
}

/// The sequential non-forced matrix-free step under the same controller,
/// with the Krylov recycle state carried across steps (the step restores it
/// itself on rejection).
struct SequentialRun {
    y: Vec<f64>,
    success: bool,
    attempts: usize,
    accepted: usize,
    rejected: usize,
    counters: WorkCounters,
}

fn sequential_adaptive(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
) -> SequentialRun {
    let (mut t, tf) = t_span;
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step;
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut recycle = KrylovState::for_method(linear.method);
    let (mut attempts, mut accepted, mut rejected) = (0, 0, 0);
    while t < tf && attempts < adaptive.max_attempts {
        let trial = h.min(tf - t);
        if trial < adaptive.min_step && t + trial < tf {
            break;
        }
        attempts += 1;
        let step = sequential_matrix_free_step(
            problem,
            t,
            &y,
            trial,
            linear,
            recycle.as_mut(),
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut counters,
        );
        let (error, ok) = match step {
            Ok(report) if report.accepted => {
                t = if trial == tf - t { tf } else { t + trial };
                y = report.y_new;
                (report.error_norm, true)
            }
            Ok(report) => (report.error_norm, false),
            Err(_) => (f64::INFINITY, false),
        };
        if ok {
            accepted += 1;
        } else {
            rejected += 1;
        }
        h = rodas_next_step_after_attempt(&mut controller, adaptive, h, trial, error, ok, false)
            .unwrap();
    }
    SequentialRun {
        y,
        success: t >= tf,
        attempts,
        accepted,
        rejected,
        counters,
    }
}

fn relative_error(y: &[f64], reference: &[f64]) -> f64 {
    let scale = reference.iter().map(|v| v.abs()).fold(0.0, f64::max);
    y.iter()
        .zip(reference)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
        / scale.max(f64::MIN_POSITIVE)
}

fn per_attempt(c: &WorkCounters, attempts: usize) -> Value {
    let a = attempts.max(1) as f64;
    json!({
        "rhs_evaluations": c.rhs_evaluations as f64 / a,
        "rhs_assembly_jvps": rhs_assembly_jvps(c) as f64 / a,
        "jvp_calls": c.jvp_calls as f64 / a,
        "linear_matvecs": c.linear_matvecs as f64 / a,
        "diagnostic_matvecs": c.diagnostic_matvecs as f64 / a,
        "linear_iterations": c.linear_iterations as f64 / a,
        "preconditioner_apps": c.preconditioner_apps as f64 / a,
    })
}

#[test]
#[ignore = "research run of research/thread_transfer_mf_workspace_20261002 (about 2 min in release); run by the ignored-tests CI job in the measurement profile"]
fn matrix_free_u_form_driver() {
    let methods = [
        LinearMethod::Gmres,
        LinearMethod::Lgmres,
        LinearMethod::Gcrodr,
    ];
    let mut gate_jvp = true;
    let mut gate_strict = true;
    let mut gate_step = true;
    let mut gate_accuracy = true;
    let mut gate_alloc = true;
    let mut rows = Vec::new();
    for case in cases() {
        // 3. One step from the initial state, tight Krylov tolerance.
        let mut steps = Vec::new();
        for h in case.steps {
            for method in methods {
                // A generous iteration budget: the tight tolerance, not the
                // budget, is what the comparison is about.
                let linear = LinearSolverConfig {
                    maxiter: 4000,
                    ..config(method, 1.0e-12)
                };
                let (atol, rtol) = (1.0e-6 * case.atol_scale, 1.0e-6);
                let mut sc = WorkCounters::default();
                let mut recycle = KrylovState::for_method(method);
                let seq = sequential_matrix_free_step(
                    &case.problem,
                    case.t_span.0,
                    &case.y0,
                    h,
                    &linear,
                    recycle.as_mut(),
                    atol,
                    rtol,
                    true,
                    &mut sc,
                );
                let mut fc = WorkCounters::default();
                let mut work = Rodas5pMfFastWorkspace::new(&case.problem, &linear).unwrap();
                let mut recycle = KrylovState::for_method(method);
                let fast = work.attempt(
                    &case.problem,
                    case.t_span.0,
                    &case.y0,
                    h,
                    true,
                    recycle.as_mut(),
                    atol,
                    rtol,
                    &mut fc,
                );
                gate_strict &= fc.jacobian_builds == 0
                    && fc.direct_factorizations == 0
                    && sc.jacobian_builds == 0
                    && sc.direct_factorizations == 0;
                let (seq, error) = match (seq, fast) {
                    (Ok(seq), Ok(error)) => (seq, error),
                    (seq, fast) => {
                        // A Krylov failure at the tight tolerance is recorded
                        // as such; it is not an agreement.
                        gate_step = false;
                        steps.push(json!({
                            "h": h, "method": format!("{method:?}"),
                            "sequential_error": seq.err().map(|e| e.to_string()),
                            "mf_fast_error": fast.err().map(|e| e.to_string()),
                            "passes": false,
                        }));
                        continue;
                    }
                };
                let worst = work
                    .y_new()
                    .iter()
                    .zip(&seq.y_new)
                    .zip(&case.y0)
                    .map(|((a, b), y)| (a - b).abs() / (y.abs() + 1.0e-6))
                    .fold(0.0_f64, f64::max);
                let error_relative =
                    (error - seq.error_norm).abs() / seq.error_norm.max(f64::MIN_POSITIVE);
                let ok = worst <= 1.0e-9 && error_relative <= 1.0e-6;
                gate_step &= ok;
                let nonzero = (0..8).all(|i| work.stage(i).iter().any(|v| *v != 0.0));
                let jvp_ok =
                    rhs_assembly_jvps(&fc) == 0 && (!nonzero || rhs_assembly_jvps(&sc) == 7);
                gate_jvp &= jvp_ok;
                steps.push(json!({
                    "h": h, "method": format!("{method:?}"), "state_difference_scaled_max": worst,
                    "error_norm": {"sequential": seq.error_norm, "mf_fast": error, "relative_difference": error_relative},
                    "rhs_assembly_jvps": {"sequential": rhs_assembly_jvps(&sc), "mf_fast": rhs_assembly_jvps(&fc)},
                    "all_stages_nonzero": nonzero,
                    "counters": {"sequential": per_attempt(&sc, 1), "mf_fast": per_attempt(&fc, 1)},
                    "passes": ok && jvp_ok,
                }));
            }
        }
        // 4 and 6. Adaptive runs at rtol 1e-6.
        let span = case.t_span.1 - case.t_span.0;
        let adapt = adaptive(1.0e-6, case.atol_scale, span);
        let reference = match (&case.exact, &case.full) {
            (Some(exact), _) => exact(case.t_span.1),
            (None, Some(full)) => {
                let tight = AdaptiveStepConfig {
                    max_attempts: 1_000_000,
                    ..adaptive(1.0e-11, case.atol_scale, span)
                };
                let run = integrate_rodas5p_fast_observed(
                    full,
                    case.t_span,
                    &case.y0,
                    &tight,
                    &OutputSchedule::new(vec![case.t_span.0, case.t_span.1]).unwrap(),
                )
                .unwrap();
                assert!(run.observed.success, "{} reference", case.id);
                run.observed.y.last().unwrap().clone()
            }
            (None, None) => unreachable!(),
        };
        let mut adaptive_rows = Vec::new();
        for method in methods {
            let linear = config(method, 1.0e-11);
            let schedule = OutputSchedule::new(vec![case.t_span.0, case.t_span.1]).unwrap();
            let (fast, fast_alloc) = allocations_during(|| {
                integrate_rodas5p_mf_fast_observed(
                    &case.problem,
                    case.t_span,
                    &case.y0,
                    &linear,
                    &adapt,
                    &schedule,
                )
                .unwrap()
            });
            let (seq, seq_alloc) = allocations_during(|| {
                sequential_adaptive(&case.problem, case.t_span, &case.y0, &linear, &adapt)
            });
            let fast_y = fast.observed.y.last().unwrap();
            let (fast_error, seq_error) = (
                relative_error(fast_y, &reference),
                relative_error(&seq.y, &reference),
            );
            let accuracy_ok = fast.observed.success
                && seq.success
                && fast_error <= 3.0 * seq_error.max(1.0e-15)
                && fast_error <= 1.0e-3;
            gate_accuracy &= accuracy_ok;
            let fc = &fast.observed.counters;
            gate_jvp &= rhs_assembly_jvps(fc) == 0;
            gate_strict &= fc.jacobian_builds == 0
                && fc.direct_factorizations == 0
                && seq.counters.jacobian_builds == 0
                && seq.counters.direct_factorizations == 0;
            let (fast_per, seq_per) = (
                fast_alloc as f64 / fast.attempts.max(1) as f64,
                seq_alloc as f64 / seq.attempts.max(1) as f64,
            );
            let alloc_ok = fast_per <= 0.5 * seq_per;
            gate_alloc &= alloc_ok;
            // The protected (inner-forced) driver, descriptive only.
            let protected = integrate_sequential_matrix_free_adaptive_observed(
                &case.problem,
                case.t_span,
                &case.y0,
                &linear,
                &adapt,
                &schedule,
            )
            .ok()
            .filter(|run| run.observed.success)
            .map(|run| {
                json!({
                    "relative_error": relative_error(run.observed.y.last().unwrap(), &reference),
                    "attempts": run.diagnostics.attempts,
                    "per_attempt": per_attempt(&run.observed.counters, run.diagnostics.attempts),
                })
            });
            println!(
                "{} {method:?}: mf-fast err {fast_error:.2e} ({} att, {:.1} alloc/att) seq err {seq_error:.2e} ({} att, {:.1} alloc/att); rhs-JVP/att {:.2} vs {:.2}",
                case.id,
                fast.attempts,
                fast_per,
                seq.attempts,
                seq_per,
                rhs_assembly_jvps(fc) as f64 / fast.attempts.max(1) as f64,
                rhs_assembly_jvps(&seq.counters) as f64 / seq.attempts.max(1) as f64,
            );
            adaptive_rows.push(json!({
                "method": format!("{method:?}"),
                "mf_fast": {"success": fast.observed.success, "relative_error": fast_error, "attempts": fast.attempts,
                    "accepted": fast.accepted_steps, "rejected": fast.rejected_steps, "state_reuses": fast.state_reuses,
                    "allocations_per_attempt": fast_per, "per_attempt": per_attempt(fc, fast.attempts)},
                "sequential": {"success": seq.success, "relative_error": seq_error, "attempts": seq.attempts,
                    "accepted": seq.accepted, "rejected": seq.rejected,
                    "allocations_per_attempt": seq_per, "per_attempt": per_attempt(&seq.counters, seq.attempts)},
                "protected_inner_forced": protected,
                "accuracy_passes": accuracy_ok,
                "allocation_passes": alloc_ok,
            }));
        }
        rows.push(json!({"problem": case.id, "dimension": case.problem.dimension, "one_step": steps, "adaptive": adaptive_rows}));
    }
    let (semantics_ok, semantics) = semantics();
    let refusals_ok = refusals();
    let gate = json!({
        "rhs_assembly_jvps_7_to_0": gate_jvp,
        "strict_matrix_free": gate_strict && refusals_ok,
        "one_step_agrees": gate_step,
        "matched_accuracy": gate_accuracy,
        "semantics": semantics_ok,
        "allocations_at_most_half": gate_alloc,
    });
    let pass = gate
        .as_object()
        .unwrap()
        .values()
        .all(|v| v.as_bool() == Some(true));
    let result = json!({
        "schema": "vigilode-thread-transfer-mf-workspace-v1",
        "driver": rodas5p_integrators::RODAS5P_MF_FAST_DRIVER_ID,
        "gate": gate,
        "verdict": if pass { "PASS" } else { "FAIL" },
        "problems": rows,
        "semantics": semantics,
    });
    write_output("THREAD_TRANSFER_MF_WORKSPACE_OUTPUT", &result);
    println!(
        "{}",
        json!({"gate": result["gate"], "verdict": result["verdict"]})
    );
}

/// Direct and explicit-matrix configurations, a mass matrix and a problem
/// without a JVP are refused.
fn refusals() -> bool {
    let (p, _) = robertson_problem().unwrap();
    let jvp_only = p.jvp_only_clone().unwrap();
    let mut ok = true;
    for (method, pc) in [
        (LinearMethod::Direct, PreconditionerKind::None),
        (LinearMethod::Gmres, PreconditionerKind::Direct),
        (LinearMethod::Gmres, PreconditionerKind::Jacobi),
    ] {
        let linear = LinearSolverConfig {
            method,
            preconditioner: pc,
            ..LinearSolverConfig::default()
        };
        ok &= Rodas5pMfFastWorkspace::new(&jvp_only, &linear).is_err();
    }
    let gmres = config(LinearMethod::Gmres, 1.0e-10);
    // A problem with only an explicit Jacobian.
    let no_jvp = OdeProblem::new(
        "explicit-only",
        1,
        Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
            out[0] = -y[0];
            Ok(())
        }),
        None,
        Some(Arc::new(|_t: f64, _y: &[f64]| {
            DenseMatrix::new(1, 1, vec![-1.0])
        })),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    ok &= Rodas5pMfFastWorkspace::new(&no_jvp, &gmres).is_err();
    let mut massive = jvp_only.clone();
    massive.mass_matrix = Some(DenseMatrix::identity(3));
    ok &= Rodas5pMfFastWorkspace::new(&massive, &gmres).is_err();
    ok
}

fn semantics() -> (bool, Value) {
    let gmres = config(LinearMethod::Lgmres, 1.0e-11);
    let (p, y0) = brusselator(10).unwrap();
    let problem = p.jvp_only_clone().unwrap();
    // A rejected attempt mutates the recycle state; the driver restores the
    // snapshot, after which the next attempt equals one made without the
    // rejected attempt (the stage buffers carry nothing over).
    let (atol, rtol) = (1.0e-6, 1.0e-6);
    let mut c = WorkCounters::default();
    let mut work = Rodas5pMfFastWorkspace::new(&problem, &gmres).unwrap();
    let mut recycle = KrylovState::for_method(LinearMethod::Lgmres);
    work.attempt(
        &problem,
        0.0,
        &y0,
        1.0e-3,
        true,
        recycle.as_mut(),
        atol,
        rtol,
        &mut c,
    )
    .unwrap();
    let snapshot = recycle.clone();
    let rejected = work
        .attempt(
            &problem,
            0.0,
            &y0,
            5.0,
            false,
            recycle.as_mut(),
            atol,
            rtol,
            &mut c,
        )
        .map_or(true, |error| error > 1.0);
    let mutated = recycle != snapshot;
    let mut restored = snapshot.clone();
    work.attempt(
        &problem,
        0.0,
        &y0,
        2.0e-3,
        false,
        restored.as_mut(),
        atol,
        rtol,
        &mut c,
    )
    .unwrap();
    let after_rejection = work.y_new().to_vec();
    let mut clean_work = Rodas5pMfFastWorkspace::new(&problem, &gmres).unwrap();
    let mut clean = KrylovState::for_method(LinearMethod::Lgmres);
    clean_work
        .attempt(
            &problem,
            0.0,
            &y0,
            1.0e-3,
            true,
            clean.as_mut(),
            atol,
            rtol,
            &mut c,
        )
        .unwrap();
    clean_work
        .attempt(
            &problem,
            0.0,
            &y0,
            2.0e-3,
            false,
            clean.as_mut(),
            atol,
            rtol,
            &mut c,
        )
        .unwrap();
    let rollback = rejected
        && after_rejection
            .iter()
            .zip(clean_work.y_new())
            .all(|(a, b)| a.to_bits() == b.to_bits());
    // Output times are the schedule, bit for bit; the run also passes through
    // a rejected first attempt (initial step 2.5).
    let times = vec![0.0, 0.1, 0.37, 1.0, 2.5];
    let schedule = OutputSchedule::new(times.clone()).unwrap();
    let run = integrate_rodas5p_mf_fast_observed(
        &problem,
        (0.0, 2.5),
        &y0,
        &gmres,
        &AdaptiveStepConfig {
            initial_step: 2.5,
            ..adaptive(1.0e-6, 1.0, 2.5)
        },
        &schedule,
    )
    .unwrap();
    let output_exact = run.observed.success
        && run.observed.t.len() == times.len()
        && run
            .observed
            .t
            .iter()
            .zip(&times)
            .all(|(a, b)| a.to_bits() == b.to_bits());
    let had_rejection = run.rejected_steps > 0;
    // A zero right-hand side gives zero stages.
    let zero = OdeProblem::new(
        "zero",
        3,
        Arc::new(|_t: f64, _y: &[f64], out: &mut [f64]| {
            out.fill(0.0);
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(
            |_t: f64, _y: &[f64], _v: &[f64], out: &mut [f64]| {
                out.fill(0.0);
                Ok(())
            },
        )),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let mut zw = Rodas5pMfFastWorkspace::new(&zero, &gmres).unwrap();
    let mut zr = KrylovState::for_method(LinearMethod::Lgmres);
    let zero_error = zw
        .attempt(
            &zero,
            0.0,
            &[1.0, 2.0, 3.0],
            0.1,
            true,
            zr.as_mut(),
            atol,
            rtol,
            &mut c,
        )
        .unwrap();
    let zero_ok = zero_error == 0.0
        && (0..8).all(|i| zw.stage(i).iter().all(|v| *v == 0.0))
        && zw.y_new() == [1.0, 2.0, 3.0];
    // A very small step completes.
    let tiny = work
        .attempt(
            &problem,
            0.0,
            &y0,
            1.0e-12,
            true,
            KrylovState::for_method(LinearMethod::Lgmres).as_mut(),
            atol,
            rtol,
            &mut c,
        )
        .map(|error| error.is_finite() && work.y_new().iter().all(|v| v.is_finite()))
        .unwrap_or(false);
    // Nonautonomous: Prothero-Robinson against its exact solution.
    let (pr, pr0) = prothero_robinson_problem(-1.0e4, 0.0, 0.0);
    let pr = pr.jvp_only_clone().unwrap();
    let pr_run = integrate_rodas5p_mf_fast_observed(
        &pr,
        (0.0, 2.0),
        &pr0,
        &config(LinearMethod::Gmres, 1.0e-11),
        &adaptive(1.0e-8, 1.0, 2.0),
        &OutputSchedule::new(vec![0.0, 2.0]).unwrap(),
    )
    .unwrap();
    let pr_error = (pr_run.observed.y.last().unwrap()[0] - 2.0_f64.sin()).abs();
    let nonautonomous = pr_run.observed.success && pr_error <= 1.0e-6;
    let ok =
        rollback && mutated && output_exact && had_rejection && zero_ok && tiny && nonautonomous;
    (
        ok,
        json!({
            "rejected_attempt_mutates_recycle": mutated,
            "rollback_reproduces_clean_attempt": rollback,
            "output_times_exact": output_exact,
            "run_had_rejection": had_rejection,
            "zero_rhs_zero_stages": zero_ok,
            "tiny_step_completes": tiny,
            "prothero_robinson_error": pr_error,
            "nonautonomous_ok": nonautonomous,
        }),
    )
}
