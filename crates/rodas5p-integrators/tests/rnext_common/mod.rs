//! Shared fixtures of the remaining-only DAG research tests (R-NEXT-01..03):
//! the L-0038 problems (copied unchanged from
//! `thread_transfer_mf_workspace.rs`), its sequential MF comparator, frozen
//! stage systems, a counting allocator and output writing.
#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{
    CoreResult, DenseMatrix, LinearMethod, LinearOperator, LinearSolverConfig, PreconditionerKind,
    ShiftedOperator, WorkCounters, rodas5p_coefficients,
};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, KrylovState, OdeProblem, QuadraticModel,
    Rodas5pMfFastWorkspace, prothero_robinson_problem, robertson_problem,
    rodas_next_step_after_attempt, sequential_matrix_free_step, stiff_van_der_pol_problem,
};

pub struct Counting;

pub static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

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

/// Allocations (including reallocations) during `f`; meaningful only in a
/// binary that installs [`Counting`] as its global allocator and runs one
/// test thread.
pub fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.load(Ordering::SeqCst);
    let value = f();
    (value, ALLOCATIONS.load(Ordering::SeqCst) - before)
}

/// Writes `value` to the path in `variable`, if set; a relative path is
/// taken from the workspace root. An existing file is never overwritten.
pub fn write_output(variable: &str, value: &serde_json::Value) {
    if let Ok(path) = std::env::var(variable) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string_pretty(value).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

/// SplitMix64, uniform in [-1, 1).
pub fn splitmix_vector(seed: u64, n: usize) -> Vec<f64> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^= z >> 31;
            2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
        })
        .collect()
}

/// `I - tau (D2 - Pe D1)` on n interior points of (0, 1), first-order upwind
/// convection, homogeneous Dirichlet boundaries.
pub fn convection_diffusion(n: usize, peclet: f64, tau: f64) -> DenseMatrix {
    let h = 1.0 / (n as f64 + 1.0);
    let (d, c) = (1.0 / (h * h), peclet / h);
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 1.0 + tau * (2.0 * d + c);
        if i > 0 {
            a[(i, i - 1)] = -tau * (d + c);
        }
        if i + 1 < n {
            a[(i, i + 1)] = -tau * d;
        }
    }
    a
}

/// `W = I - h gamma J(t, y)` with the problem's JVP (counted), as the U-form
/// driver builds it.
pub fn stage_operator(problem: &OdeProblem, t: f64, y: &[f64], h: f64) -> ShiftedOperator {
    let gamma = rodas5p_coefficients().unwrap().gamma;
    let jvp = problem.linearize_matrix_free(t, y).unwrap();
    ShiftedOperator::new_counted_jvp(None, jvp, h, gamma).unwrap()
}

/// `b_i = W U_i` for the eight stages `U_i` held by `work`.
pub fn stage_right_hand_sides(
    op: &dyn LinearOperator,
    work: &Rodas5pMfFastWorkspace,
) -> Vec<Vec<f64>> {
    (0..8)
        .map(|i| {
            let mut b = vec![0.0; op.dimension()];
            op.apply(work.stage(i), &mut b).unwrap();
            b
        })
        .collect()
}

pub fn linear_config(method: LinearMethod, rtol: f64) -> LinearSolverConfig {
    LinearSolverConfig {
        method,
        rtol,
        atol: 1.0e-14,
        preconditioner: PreconditionerKind::None,
        ..LinearSolverConfig::default()
    }
}

pub fn adaptive(rtol: f64, scale: f64, span: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * scale,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: span,
        max_attempts: 5_000,
        ..AdaptiveStepConfig::default()
    }
}

pub fn relative_error(y: &[f64], reference: &[f64]) -> f64 {
    let scale = reference.iter().map(|v| v.abs()).fold(0.0, f64::max);
    y.iter()
        .zip(reference)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
        / scale.max(f64::MIN_POSITIVE)
}

pub fn hires() -> CoreResult<(OdeProblem, Vec<f64>)> {
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
pub fn brusselator(cells: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
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
pub fn quadratic() -> (QuadraticModel, Vec<f64>) {
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

pub struct Case {
    pub id: &'static str,
    /// JVP-only clone.
    pub problem: OdeProblem,
    /// With an explicit Jacobian, for the reference run (if any).
    pub full: Option<OdeProblem>,
    pub exact: Option<Box<dyn Fn(f64) -> Vec<f64>>>,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    pub atol_scale: f64,
    /// One-step sizes.
    pub steps: [f64; 2],
}

pub fn cases() -> Vec<Case> {
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

/// The sequential non-forced matrix-free step under the same controller,
/// with the Krylov recycle state carried across steps (the step restores it
/// itself on rejection).
pub struct SequentialRun {
    pub y: Vec<f64>,
    pub success: bool,
    pub attempts: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub counters: WorkCounters,
}

pub fn sequential_adaptive(
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
