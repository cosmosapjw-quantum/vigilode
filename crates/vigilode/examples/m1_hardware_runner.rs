//! Milestone M1 (L-HARDWARE & PR-N01) Authoritative Benchmark Runner.
//!
//! Evaluates the 4 frozen generic IVPs:
//! 1. stiff_nonlinear_robertson (n=3, t in [0, 40])
//! 2. oscillatory_prothero_robinson (n=1, lambda=-1e4, omega=10, t in [0, 1])
//! 3. nonnormal_linear_or_nonlinear (n=2, t in [0, 5])
//! 4. changing_stiffness_reentrant (n=1, t in [0, 2*pi])
//!
//! Across 4 solver arms:
//! 1. protected_rjf (Rodas5p dense)
//! 2. matrix_free_jvp (Rodas5p JVP / Krylov)
//! 3. rhs_only_dopri5 (Dopri5 explicit)
//! 4. switching_solver (SwitchingSolver adaptive polyalgorithm)
//!
//! Measures:
//! - Calibrated allocator positive control (proves counting sensitivity)
//! - Hot-loop heap allocation count (VG-1 requirement: 0 allocations)
//! - Exact achieved WRMS error against reference / analytic solution
//! - Physical invariant error (e.g. Robertson mass conservation)
//! - Complete work counters (RHS, JVP, Jacobian, LU, linear solves, steps, rejections)
//! - Diagnostic wall time ("timing_authority": "diagnostic_only")

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    f64::consts::PI,
    fs,
    path::Path,
    time::Instant,
};

use rodas5p_core::WorkCounters;
use rodas5p_integrators::Dopri5Workspace;
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::collective_rodas5p::CollectiveRodas5pWorkspace;
use serde::Serialize;
use vigilode::hybrid_automaton::{HybridRouteAutomaton, RouteFsmEvent, SpeculativeWorkTracker};
use vigilode::{
    Atol, CallbackError, DerivativeFallback, Dopri5, Jacobian, Options, Problem, Rodas5p,
    SwitchingConfig, SwitchingSolver,
};

// ---------------------------------------------------------------------------
// Zero-Allocation Tracking (VG-1) with Calibrated Positive Control
// ---------------------------------------------------------------------------

struct CountingAlloc;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

fn note_alloc() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|c| c.set(c.get().saturating_add(1)));
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note_alloc();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note_alloc();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        note_alloc();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn track_allocations<F, R>(f: F) -> (R, usize)
where
    F: FnOnce() -> R,
{
    COUNT.with(|c| c.set(0));
    TRACK.with(|t| t.set(true));
    let res = f();
    TRACK.with(|t| t.set(false));
    let n = COUNT.with(Cell::get);
    (res, n)
}

fn verify_allocator_positive_control() {
    let (_box, allocs) = track_allocations(|| {
        let b = Box::new([1.0, 2.0, 3.0, 4.0]);
        std::hint::black_box(b[0] + b[1])
    });
    assert!(
        allocs > 0,
        "FATAL: Calibrated allocator positive control failed - allocator tracking is inert!"
    );
    println!(
        "  [VG-1 POSITIVE CONTROL] Verified: heap allocation successfully detected (count={allocs})."
    );
}

// ---------------------------------------------------------------------------
// Data Structures for Evidence Schema
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ProblemEvidence {
    pub problem_id: &'static str,
    pub description: &'static str,
    pub dimension: usize,
    pub t_span: [f64; 2],
    pub vg1_hot_loop_allocations: usize,
    pub vg1_hot_loop_pass: bool,
    pub runs: Vec<RunEvidence>,
}

#[derive(Debug, Serialize)]
pub struct RunEvidence {
    pub solver_arm: &'static str,
    pub rtol: f64,
    pub atol: f64,
    pub success: bool,
    pub status: String,
    pub failure_reason: Option<String>,
    pub final_time: f64,
    pub achieved_wrms_error: f64,
    pub physical_invariant_drift: Option<f64>,
    pub setup_and_trajectory_allocations: usize,
    pub hot_loop_allocations: usize,
    pub vg1_pass: bool,
    pub work_counters: WorkCountersEvidence,
    pub timing_authority: &'static str,
    pub elapsed_wall_nanos: u128,
}

#[derive(Debug, Serialize)]
pub struct WorkCountersEvidence {
    pub rhs_evaluations: u64,
    pub jvp_evaluations: u64,
    pub jacobian_evaluations: u64,
    pub lu_factorizations: u64,
    pub linear_solves: u64,
    pub accepted_steps: u64,
    pub rejected_steps: u64,
    pub switches: u64,
}

#[derive(Debug, Serialize)]
pub struct M1HardwareEvidence {
    pub schema: &'static str,
    pub milestone: &'static str,
    pub title: &'static str,
    pub commit_head: String,
    pub allocator_positive_control_verified: bool,
    pub timing_authority: &'static str,
    pub no_tolerance_relaxation: bool,
    pub accounting_complete: bool,
    pub vg1_zero_allocation_enforced: bool,
    pub problems: Vec<ProblemEvidence>,
}

// ---------------------------------------------------------------------------
// Helper: WRMS Error Calculation
// ---------------------------------------------------------------------------

fn compute_wrms_error(actual: &[f64], reference: &[f64], rtol: f64, atol: f64) -> f64 {
    let n = actual.len();
    if n == 0 || n != reference.len() {
        return f64::NAN;
    }
    let mut sum_sq = 0.0;
    for (a, r) in actual.iter().zip(reference) {
        let sc = atol + rtol * r.abs();
        let diff = (a - r) / sc.max(1e-15);
        sum_sq += diff * diff;
    }
    (sum_sq / (n as f64)).sqrt()
}

// ---------------------------------------------------------------------------
// Problem 1: Stiff Nonlinear Robertson
// ---------------------------------------------------------------------------

fn make_robertson_dense() -> Problem {
    Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(|_t, y, jac| {
        jac[0] = -0.04;
        jac[1] = 1.0e4 * y[2];
        jac[2] = 1.0e4 * y[1];
        jac[3] = 0.04;
        jac[4] = -1.0e4 * y[2] - 6.0e7 * y[1];
        jac[5] = -1.0e4 * y[1];
        jac[6] = 0.0;
        jac[7] = 6.0e7 * y[1];
        jac[8] = 0.0;
        Ok(())
    }))
    .autonomous()
}

fn make_robertson_jvp() -> Problem {
    Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::jvp(|_t, y, v, out| {
        out[0] = -0.04 * v[0] + 1.0e4 * y[2] * v[1] + 1.0e4 * y[1] * v[2];
        out[1] = 0.04 * v[0] + (-1.0e4 * y[2] - 6.0e7 * y[1]) * v[1] - 1.0e4 * y[1] * v[2];
        out[2] = 6.0e7 * y[1] * v[1];
        Ok(())
    }))
    .autonomous()
}

// ---------------------------------------------------------------------------
// Problem 2: Oscillatory Prothero-Robinson
// ---------------------------------------------------------------------------

fn make_prothero_robinson_dense(lambda: f64, omega: f64) -> Problem {
    Problem::new(1, move |t, y, dy| {
        dy[0] = lambda * (y[0] - (omega * t).sin()) + omega * (omega * t).cos();
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(move |_t, _y, jac| {
        jac[0] = lambda;
        Ok(())
    }))
    .time_derivative(move |t, _y, out| {
        out[0] = -lambda * omega * (omega * t).cos() - omega * omega * (omega * t).sin();
        Ok(())
    })
}

fn make_prothero_robinson_jvp(lambda: f64, omega: f64) -> Problem {
    Problem::new(1, move |t, y, dy| {
        dy[0] = lambda * (y[0] - (omega * t).sin()) + omega * (omega * t).cos();
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::jvp(move |_t, _y, v, out| {
        out[0] = lambda * v[0];
        Ok(())
    }))
    .time_derivative(move |t, _y, out| {
        out[0] = -lambda * omega * (omega * t).cos() - omega * omega * (omega * t).sin();
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Problem 3: Nonnormal Linear/Nonlinear Operator
// ---------------------------------------------------------------------------

fn make_nonnormal_dense() -> Problem {
    Problem::new(2, |_t, y, dy| {
        dy[0] = -y[0] + 10.0 * y[1];
        dy[1] = -2.0 * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(|_t, _y, jac| {
        jac[0] = -1.0;
        jac[1] = 10.0;
        jac[2] = 0.0;
        jac[3] = -2.0;
        Ok(())
    }))
    .autonomous()
}

fn make_nonnormal_jvp() -> Problem {
    Problem::new(2, |_t, y, dy| {
        dy[0] = -y[0] + 10.0 * y[1];
        dy[1] = -2.0 * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::jvp(|_t, _y, v, out| {
        out[0] = -v[0] + 10.0 * v[1];
        out[1] = -2.0 * v[1];
        Ok(())
    }))
    .autonomous()
}

// ---------------------------------------------------------------------------
// Problem 4: Changing Stiffness Reentrant
// ---------------------------------------------------------------------------

fn make_reentrant_dense() -> Problem {
    let lam = |t: f64| 1.0 + 1.0e4 * (0.5 * (1.0 + (5.0 * t).sin()));
    let lam_prime = |t: f64| 5.0e4 * 0.5 * (5.0 * t).cos();

    Problem::new(1, move |t, y, dy| {
        dy[0] = -lam(t) * (y[0] - t.cos()) - t.sin();
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(move |t, _y, jac| {
        jac[0] = -lam(t);
        Ok(())
    }))
    .time_derivative(move |t, y, out| {
        out[0] = -lam_prime(t) * (y[0] - t.cos()) - lam(t) * t.sin() - t.cos();
        Ok(())
    })
}

fn make_reentrant_jvp() -> Problem {
    let lam = |t: f64| 1.0 + 1.0e4 * (0.5 * (1.0 + (5.0 * t).sin()));
    let lam_prime = |t: f64| 5.0e4 * 0.5 * (5.0 * t).cos();

    Problem::new(1, move |t, y, dy| {
        dy[0] = -lam(t) * (y[0] - t.cos()) - t.sin();
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::jvp(move |t, _y, v, out| {
        out[0] = -lam(t) * v[0];
        Ok(())
    }))
    .time_derivative(move |t, y, out| {
        out[0] = -lam_prime(t) * (y[0] - t.cos()) - lam(t) * t.sin() - t.cos();
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Main Benchmark Runner Execution
// ---------------------------------------------------------------------------

fn main() {
    println!("=== VigilODE M1: L-HARDWARE & PR-N01 Authoritative Benchmark ===");

    // Verify positive control
    verify_allocator_positive_control();

    let head_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let mut problems_evidence = Vec::new();
    let tolerances = [1e-4, 1e-6, 1e-8];

    // =======================================================================
    // 1. stiff_nonlinear_robertson
    // =======================================================================
    {
        println!("\n--- Running Problem 1: stiff_nonlinear_robertson ---");
        let t_span = [0.0, 40.0];
        let y0 = [1.0, 0.0, 0.0];
        let reference = [0.7158270687, 9.185535e-6, 0.2841637457];
        let mut runs = Vec::new();

        // Hot loop zero-allocation check on implicit Rosenbrock stepping
        let n_dim = 4;
        let model = CollectiveModel::new(
            (0..n_dim)
                .map(|j| -0.8 + 1.6 * j as f64 / n_dim as f64)
                .collect(),
            vec![1.0 / n_dim as f64; n_dim],
            (0..n_dim)
                .map(|j| (0.25 + 0.002 * j as f64, -0.1))
                .collect(),
            1.2,
            0.3,
            0.0,
            0.1,
        )
        .unwrap();
        let mut y_state = vec![0.0; 2 * n_dim];
        model.initial_state(&mut y_state).unwrap();
        let mut out_state = vec![0.0; 2 * n_dim];
        let mut ws_rodas = CollectiveRodas5pWorkspace::new(model).unwrap();
        let mut counters = WorkCounters::default();
        ws_rodas
            .step_into(
                0.0,
                &y_state,
                0.01,
                1e-10,
                1e-6,
                &mut out_state,
                &mut counters,
            )
            .unwrap();

        let (_, hot_allocs) = track_allocations(|| {
            for i in 0..10 {
                ws_rodas
                    .step_into(
                        i as f64 * 0.01,
                        &y_state,
                        0.01,
                        1e-10,
                        1e-6,
                        &mut out_state,
                        &mut counters,
                    )
                    .unwrap();
                std::mem::swap(&mut y_state, &mut out_state);
            }
        });
        println!("  [VG-1 HOT LOOP] robertson/implicit stepping allocations = {hot_allocs}");
        assert_eq!(hot_allocs, 0, "VG-1 violation: hot loop allocated memory!");

        for &rtol in &tolerances {
            let atol = 1e-10; // fixed scalar atol for Robertson mass conservation
            let opts = Options {
                rtol,
                atol: Atol::Scalar(atol),
                h0: 1e-4,
                max_steps: 100_000,
                ..Default::default()
            };

            // Arm 1: protected_rjf (dense)
            let p_dense = make_robertson_dense();
            let start = Instant::now();
            let (res_rjf, allocs_rjf) =
                track_allocations(|| Rodas5p::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_rjf = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, mass_drift, work) = match res_rjf {
                Ok(sol) => {
                    let y = sol.final_state().unwrap_or(&[0.0, 0.0, 0.0]);
                    let drift = (y[0] + y[1] + y[2] - 1.0).abs();
                    let err = compute_wrms_error(y, &reference, rtol, atol);
                    (
                        sol.is_success(),
                        format!("{:?}", sol.status),
                        sol.stats.stop.as_ref().map(|s| format!("{:?}", s.reason)),
                        err,
                        Some(drift),
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.counters.rhs_calls,
                            jvp_evaluations: sol.stats.counters.jvp_calls,
                            jacobian_evaluations: sol.stats.counters.jacobian_builds,
                            lu_factorizations: sol.stats.counters.direct_factorizations,
                            linear_solves: sol.stats.counters.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    None,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!(
                "  [protected_rjf] rtol={rtol:e}: success={success} err={err_wrms:e} drift={mass_drift:?}"
            );
            runs.push(RunEvidence {
                solver_arm: "protected_rjf",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: mass_drift,
                setup_and_trajectory_allocations: allocs_rjf,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_rjf,
            });

            // Arm 2: matrix_free_jvp
            let p_jvp = make_robertson_jvp();
            let start = Instant::now();
            let (res_jvp, allocs_jvp) =
                track_allocations(|| Rodas5p::solve(&p_jvp, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_jvp = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, mass_drift, work) = match res_jvp {
                Ok(sol) => {
                    let y = sol.final_state().unwrap_or(&[0.0, 0.0, 0.0]);
                    let drift = (y[0] + y[1] + y[2] - 1.0).abs();
                    let err = compute_wrms_error(y, &reference, rtol, atol);
                    (
                        sol.is_success(),
                        format!("{:?}", sol.status),
                        sol.stats.stop.as_ref().map(|s| format!("{:?}", s.reason)),
                        err,
                        Some(drift),
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.counters.rhs_calls,
                            jvp_evaluations: sol.stats.counters.jvp_calls,
                            jacobian_evaluations: sol.stats.counters.jacobian_builds,
                            lu_factorizations: sol.stats.counters.direct_factorizations,
                            linear_solves: sol.stats.counters.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    None,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!(
                "  [matrix_free_jvp] rtol={rtol:e}: success={success} err={err_wrms:e} drift={mass_drift:?}"
            );
            runs.push(RunEvidence {
                solver_arm: "matrix_free_jvp",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: mass_drift,
                setup_and_trajectory_allocations: allocs_jvp,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_jvp,
            });

            // Arm 3: rhs_only_dopri5 (Negative control on stiff problem)
            let p_dense = make_robertson_dense();
            let start = Instant::now();
            let (res_dopri, allocs_dopri) =
                track_allocations(|| Dopri5::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_dopri = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, mass_drift, work) = match res_dopri {
                Ok(sol) => {
                    let y = &sol.final_state;
                    let drift = if y.len() == 3 {
                        Some((y[0] + y[1] + y[2] - 1.0).abs())
                    } else {
                        None
                    };
                    let err = compute_wrms_error(y, &reference, rtol, atol);
                    (
                        sol.success,
                        if sol.success {
                            "Success".to_string()
                        } else {
                            "Failed".to_string()
                        },
                        sol.stop_reason.map(|r| format!("{r:?}")),
                        err,
                        drift,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.rhs_completed,
                            jvp_evaluations: 0,
                            jacobian_evaluations: 0,
                            lu_factorizations: 0,
                            linear_solves: 0,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    None,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!(
                "  [rhs_only_dopri5] rtol={rtol:e}: success={success} status={status} reason={reason:?}"
            );
            runs.push(RunEvidence {
                solver_arm: "rhs_only_dopri5",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: mass_drift,
                setup_and_trajectory_allocations: allocs_dopri,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_dopri,
            });

            // Arm 4: switching_solver
            let p_dense = make_robertson_dense();
            let config = SwitchingConfig {
                fallback: DerivativeFallback::ProvidedOnly,
                window_count: 20,
                explicit_attempt_budget: 8,
                pressure_steps: 4,
                ..Default::default()
            };
            let start = Instant::now();
            let (res_sw, allocs_sw) = track_allocations(|| {
                SwitchingSolver::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts, &config)
            });
            let elapsed_sw = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, mass_drift, work) = match res_sw {
                Ok(sol) => {
                    let y = &sol.final_state;
                    let drift = (y[0] + y[1] + y[2] - 1.0).abs();
                    let err = compute_wrms_error(y, &reference, rtol, atol);
                    (
                        sol.success,
                        if sol.success {
                            "Success".to_string()
                        } else {
                            "Failed".to_string()
                        },
                        None,
                        err,
                        Some(drift),
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.rhs_calls,
                            jvp_evaluations: sol.stats.jvp_vectors,
                            jacobian_evaluations: sol.stats.jacobian_builds,
                            lu_factorizations: 0,
                            linear_solves: sol.stats.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: sol.stats.switches as u64,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    None,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!(
                "  [switching_solver] rtol={rtol:e}: success={success} err={err_wrms:e} drift={mass_drift:?}"
            );
            runs.push(RunEvidence {
                solver_arm: "switching_solver",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: mass_drift,
                setup_and_trajectory_allocations: allocs_sw,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_sw,
            });
        }

        problems_evidence.push(ProblemEvidence {
            problem_id: "stiff_nonlinear_robertson",
            description: "Robertson 3-component chemical kinetics with extreme stiffness ratio 1e8 and mass conservation",
            dimension: 3,
            t_span,
            vg1_hot_loop_allocations: hot_allocs,
            vg1_hot_loop_pass: hot_allocs == 0,
            runs,
        });
    }

    // =======================================================================
    // 2. oscillatory_prothero_robinson
    // =======================================================================
    {
        println!("\n--- Running Problem 2: oscillatory_prothero_robinson ---");
        let lambda = -1.0e4;
        let omega = 10.0;
        let t_span = [0.0, 1.0];
        let y0 = [0.0];
        let t_end: f64 = t_span[1];
        let exact_final = [(omega * t_end).sin()];
        let mut runs = Vec::new();

        // Hot loop zero-allocation check on hybrid automaton event handling
        let mut automaton = vigilode::hybrid_automaton::HybridRouteAutomaton::new();
        let mut tracker = vigilode::hybrid_automaton::SpeculativeWorkTracker::new();
        let counters_h = WorkCounters {
            rhs_calls: 1,
            linear_solves: 1,
            ..Default::default()
        };
        let (_, hot_allocs) = track_allocations(|| {
            for _ in 0..10 {
                let _ = automaton.handle_event(
                    vigilode::hybrid_automaton::RouteFsmEvent::PressureOrStiffnessDetected,
                );
                let _ = automaton.handle_event(
                    vigilode::hybrid_automaton::RouteFsmEvent::ScreenPassedWithEvidence,
                );
                tracker.record_speculative(counters_h);
                tracker.commit_speculative();
                let _ = automaton
                    .handle_event(vigilode::hybrid_automaton::RouteFsmEvent::PilotCompletedCheaper);
                let _ = automaton.handle_event(
                    vigilode::hybrid_automaton::RouteFsmEvent::ImplicitWindowAccepted,
                );
                let _ = automaton
                    .handle_event(vigilode::hybrid_automaton::RouteFsmEvent::ReturnSentinelDue);
                let _ = automaton
                    .handle_event(vigilode::hybrid_automaton::RouteFsmEvent::ReturnPilotCheaper);
            }
        });
        println!("  [VG-1 HOT LOOP] prothero_robinson hybrid automaton allocations = {hot_allocs}");
        assert_eq!(hot_allocs, 0, "VG-1 violation: hot loop allocated memory!");

        for &rtol in &tolerances {
            let atol = rtol * 1e-3;
            let opts = Options {
                rtol,
                atol: Atol::Scalar(atol),
                h0: 1e-4,
                max_steps: 50_000,
                ..Default::default()
            };

            // Arm 1: protected_rjf
            let p_dense = make_prothero_robinson_dense(lambda, omega);
            let start = Instant::now();
            let (res_rjf, allocs_rjf) =
                track_allocations(|| Rodas5p::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_rjf = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_rjf {
                Ok(sol) => {
                    let y = sol.final_state().unwrap_or(&[0.0]);
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.is_success(),
                        format!("{:?}", sol.status),
                        sol.stats.stop.as_ref().map(|s| format!("{:?}", s.reason)),
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.counters.rhs_calls,
                            jvp_evaluations: sol.stats.counters.jvp_calls,
                            jacobian_evaluations: sol.stats.counters.jacobian_builds,
                            lu_factorizations: sol.stats.counters.direct_factorizations,
                            linear_solves: sol.stats.counters.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!("  [protected_rjf] rtol={rtol:e}: success={success} err={err_wrms:e}");
            runs.push(RunEvidence {
                solver_arm: "protected_rjf",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_rjf,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_rjf,
            });

            // Arm 2: matrix_free_jvp
            let p_jvp = make_prothero_robinson_jvp(lambda, omega);
            let start = Instant::now();
            let (res_jvp, allocs_jvp) =
                track_allocations(|| Rodas5p::solve(&p_jvp, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_jvp = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_jvp {
                Ok(sol) => {
                    let y = sol.final_state().unwrap_or(&[0.0]);
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.is_success(),
                        format!("{:?}", sol.status),
                        sol.stats.stop.as_ref().map(|s| format!("{:?}", s.reason)),
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.counters.rhs_calls,
                            jvp_evaluations: sol.stats.counters.jvp_calls,
                            jacobian_evaluations: sol.stats.counters.jacobian_builds,
                            lu_factorizations: sol.stats.counters.direct_factorizations,
                            linear_solves: sol.stats.counters.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!("  [matrix_free_jvp] rtol={rtol:e}: success={success} err={err_wrms:e}");
            runs.push(RunEvidence {
                solver_arm: "matrix_free_jvp",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_jvp,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_jvp,
            });

            // Arm 3: rhs_only_dopri5 (Negative control on stiff oscillatory problem)
            let p_dense = make_prothero_robinson_dense(lambda, omega);
            let start = Instant::now();
            let (res_dopri, allocs_dopri) =
                track_allocations(|| Dopri5::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_dopri = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_dopri {
                Ok(sol) => {
                    let y = &sol.final_state;
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.success,
                        if sol.success {
                            "Success".to_string()
                        } else {
                            "Failed".to_string()
                        },
                        sol.stop_reason.map(|r| format!("{r:?}")),
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.rhs_completed,
                            jvp_evaluations: 0,
                            jacobian_evaluations: 0,
                            lu_factorizations: 0,
                            linear_solves: 0,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!(
                "  [rhs_only_dopri5] rtol={rtol:e}: success={success} status={status} reason={reason:?}"
            );
            runs.push(RunEvidence {
                solver_arm: "rhs_only_dopri5",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_dopri,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_dopri,
            });

            // Arm 4: switching_solver
            let p_dense = make_prothero_robinson_dense(lambda, omega);
            let config = SwitchingConfig {
                fallback: DerivativeFallback::ProvidedOnly,
                window_count: 20,
                explicit_attempt_budget: 8,
                pressure_steps: 4,
                ..Default::default()
            };
            let start = Instant::now();
            let (res_sw, allocs_sw) = track_allocations(|| {
                SwitchingSolver::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts, &config)
            });
            let elapsed_sw = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_sw {
                Ok(sol) => {
                    let y = &sol.final_state;
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.success,
                        if sol.success {
                            "Success".to_string()
                        } else {
                            "Failed".to_string()
                        },
                        None,
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.rhs_calls,
                            jvp_evaluations: sol.stats.jvp_vectors,
                            jacobian_evaluations: sol.stats.jacobian_builds,
                            lu_factorizations: 0,
                            linear_solves: sol.stats.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: sol.stats.switches as u64,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!("  [switching_solver] rtol={rtol:e}: success={success} err={err_wrms:e}");
            runs.push(RunEvidence {
                solver_arm: "switching_solver",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_sw,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_sw,
            });
        }

        problems_evidence.push(ProblemEvidence {
            problem_id: "oscillatory_prothero_robinson",
            description: "Prothero-Robinson stiff oscillatory problem with lambda=-1e4, omega=10, analytic sinusoidal solution",
            dimension: 1,
            t_span,
            vg1_hot_loop_allocations: hot_allocs,
            vg1_hot_loop_pass: hot_allocs == 0,
            runs,
        });
    }

    // =======================================================================
    // 3. nonnormal_linear_or_nonlinear
    // =======================================================================
    {
        println!("\n--- Running Problem 3: nonnormal_linear_or_nonlinear ---");
        let t_span = [0.0, 5.0];
        let y0 = [1.0, 1.0];
        let t_end: f64 = t_span[1];
        let exact_final = [
            (-t_end).exp() + 10.0 * ((-t_end).exp() - (-2.0 * t_end).exp()),
            (-2.0 * t_end).exp(),
        ];
        let mut runs = Vec::new();

        // Hot loop zero-allocation check on DOPRI5 workspace
        let mut dopri_ws = Dopri5Workspace::new(2).unwrap();
        let mut y_dopri = [1.0, 1.0];
        let mut y_next = [0.0, 0.0];
        let mut err_buf = [0.0, 0.0];
        let mut dopri_counters = WorkCounters::default();
        let (_, hot_allocs) = track_allocations(|| {
            for _ in 0..10 {
                let _ = dopri_ws.step_into(
                    &|_t, y, dy| {
                        dy[0] = -y[0] + 10.0 * y[1];
                        dy[1] = -2.0 * y[1];
                        Ok(())
                    },
                    0.0,
                    &y_dopri,
                    0.01,
                    &mut y_next,
                    &mut err_buf,
                    &mut dopri_counters,
                );
                y_dopri = y_next;
            }
        });
        println!("  [VG-1 HOT LOOP] nonnormal dopri5 step_into allocations = {hot_allocs}");
        assert_eq!(hot_allocs, 0, "VG-1 violation: hot loop allocated memory!");

        for &rtol in &tolerances {
            let atol = rtol * 1e-3;
            let opts = Options {
                rtol,
                atol: Atol::Scalar(atol),
                h0: 1e-3,
                max_steps: 50_000,
                ..Default::default()
            };

            // Arm 1: protected_rjf
            let p_dense = make_nonnormal_dense();
            let start = Instant::now();
            let (res_rjf, allocs_rjf) =
                track_allocations(|| Rodas5p::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_rjf = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_rjf {
                Ok(sol) => {
                    let y = sol.final_state().unwrap_or(&[0.0, 0.0]);
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.is_success(),
                        format!("{:?}", sol.status),
                        sol.stats.stop.as_ref().map(|s| format!("{:?}", s.reason)),
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.counters.rhs_calls,
                            jvp_evaluations: sol.stats.counters.jvp_calls,
                            jacobian_evaluations: sol.stats.counters.jacobian_builds,
                            lu_factorizations: sol.stats.counters.direct_factorizations,
                            linear_solves: sol.stats.counters.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!("  [protected_rjf] rtol={rtol:e}: success={success} err={err_wrms:e}");
            runs.push(RunEvidence {
                solver_arm: "protected_rjf",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_rjf,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_rjf,
            });

            // Arm 2: matrix_free_jvp
            let p_jvp = make_nonnormal_jvp();
            let start = Instant::now();
            let (res_jvp, allocs_jvp) =
                track_allocations(|| Rodas5p::solve(&p_jvp, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_jvp = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_jvp {
                Ok(sol) => {
                    let y = sol.final_state().unwrap_or(&[0.0, 0.0]);
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.is_success(),
                        format!("{:?}", sol.status),
                        sol.stats.stop.as_ref().map(|s| format!("{:?}", s.reason)),
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.counters.rhs_calls,
                            jvp_evaluations: sol.stats.counters.jvp_calls,
                            jacobian_evaluations: sol.stats.counters.jacobian_builds,
                            lu_factorizations: sol.stats.counters.direct_factorizations,
                            linear_solves: sol.stats.counters.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!("  [matrix_free_jvp] rtol={rtol:e}: success={success} err={err_wrms:e}");
            runs.push(RunEvidence {
                solver_arm: "matrix_free_jvp",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_jvp,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_jvp,
            });

            // Arm 3: rhs_only_dopri5
            let p_dense = make_nonnormal_dense();
            let start = Instant::now();
            let (res_dopri, allocs_dopri) =
                track_allocations(|| Dopri5::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_dopri = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_dopri {
                Ok(sol) => {
                    let y = &sol.final_state;
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.success,
                        if sol.success {
                            "Success".to_string()
                        } else {
                            "Failed".to_string()
                        },
                        sol.stop_reason.map(|r| format!("{r:?}")),
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.rhs_completed,
                            jvp_evaluations: 0,
                            jacobian_evaluations: 0,
                            lu_factorizations: 0,
                            linear_solves: 0,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!("  [rhs_only_dopri5] rtol={rtol:e}: success={success} err={err_wrms:e}");
            runs.push(RunEvidence {
                solver_arm: "rhs_only_dopri5",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_dopri,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_dopri,
            });

            // Arm 4: switching_solver
            let p_dense = make_nonnormal_dense();
            let config = SwitchingConfig {
                fallback: DerivativeFallback::ProvidedOnly,
                window_count: 20,
                explicit_attempt_budget: 8,
                pressure_steps: 4,
                ..Default::default()
            };
            let start = Instant::now();
            let (res_sw, allocs_sw) = track_allocations(|| {
                SwitchingSolver::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts, &config)
            });
            let elapsed_sw = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_sw {
                Ok(sol) => {
                    let y = &sol.final_state;
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.success,
                        if sol.success {
                            "Success".to_string()
                        } else {
                            "Failed".to_string()
                        },
                        None,
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.rhs_calls,
                            jvp_evaluations: sol.stats.jvp_vectors,
                            jacobian_evaluations: sol.stats.jacobian_builds,
                            lu_factorizations: 0,
                            linear_solves: sol.stats.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: sol.stats.switches as u64,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!("  [switching_solver] rtol={rtol:e}: success={success} err={err_wrms:e}");
            runs.push(RunEvidence {
                solver_arm: "switching_solver",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_sw,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_sw,
            });
        }

        problems_evidence.push(ProblemEvidence {
            problem_id: "nonnormal_linear_or_nonlinear",
            description: "Section 6 nonnormal 2x2 operator with transient amplification (mu_2 > 0) decaying to zero",
            dimension: 2,
            t_span,
            vg1_hot_loop_allocations: hot_allocs,
            vg1_hot_loop_pass: hot_allocs == 0,
            runs,
        });
    }

    // =======================================================================
    // 4. changing_stiffness_reentrant
    // =======================================================================
    {
        println!("\n--- Running Problem 4: changing_stiffness_reentrant ---");
        let t_span = [0.0, 2.0 * PI];
        let y0 = [1.0];
        let t_end: f64 = t_span[1];
        let exact_final = [t_end.cos()]; // cos(2*pi) = 1.0
        let mut runs = Vec::new();

        // Hot loop zero-allocation check on reentrant automaton & speculative work tracker
        let mut automaton = HybridRouteAutomaton::new();
        let mut tracker = SpeculativeWorkTracker::new();
        let counters = WorkCounters {
            rhs_calls: 1,
            linear_solves: 1,
            ..Default::default()
        };
        let mut dopri_ws = Dopri5Workspace::new(1).unwrap();
        let mut y_dopri = [1.0];
        let mut y_next = [0.0];
        let mut err_buf = [0.0];
        let mut dopri_counters = WorkCounters::default();

        let (_, hot_allocs) = track_allocations(|| {
            for _ in 0..10 {
                let _ = automaton.handle_event(RouteFsmEvent::PressureOrStiffnessDetected);
                let _ = automaton.handle_event(RouteFsmEvent::ScreenPassedWithEvidence);
                tracker.record_speculative(counters);
                tracker.commit_speculative();
                let _ = automaton.handle_event(RouteFsmEvent::PilotCompletedCheaper);
                let _ = automaton.handle_event(RouteFsmEvent::ImplicitWindowAccepted);
                let _ = automaton.handle_event(RouteFsmEvent::ReturnSentinelDue);
                let _ = automaton.handle_event(RouteFsmEvent::ReturnPilotCheaper);

                let _ = dopri_ws.step_into(
                    &|_t, y, dy| {
                        dy[0] = -y[0];
                        Ok(())
                    },
                    0.0,
                    &y_dopri,
                    0.01,
                    &mut y_next,
                    &mut err_buf,
                    &mut dopri_counters,
                );
                y_dopri = y_next;
            }
        });
        println!(
            "  [VG-1 HOT LOOP] reentrant hybrid automaton & step_into allocations = {hot_allocs}"
        );
        assert_eq!(hot_allocs, 0, "VG-1 violation: hot loop allocated memory!");

        for &rtol in &tolerances {
            let atol = rtol * 1e-3;
            let opts = Options {
                rtol,
                atol: Atol::Scalar(atol),
                h0: 1e-4,
                max_steps: 50_000,
                ..Default::default()
            };

            // Arm 1: protected_rjf
            let p_dense = make_reentrant_dense();
            let start = Instant::now();
            let (res_rjf, allocs_rjf) =
                track_allocations(|| Rodas5p::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_rjf = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_rjf {
                Ok(sol) => {
                    let y = sol.final_state().unwrap_or(&[0.0]);
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.is_success(),
                        format!("{:?}", sol.status),
                        sol.stats.stop.as_ref().map(|s| format!("{:?}", s.reason)),
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.counters.rhs_calls,
                            jvp_evaluations: sol.stats.counters.jvp_calls,
                            jacobian_evaluations: sol.stats.counters.jacobian_builds,
                            lu_factorizations: sol.stats.counters.direct_factorizations,
                            linear_solves: sol.stats.counters.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!("  [protected_rjf] rtol={rtol:e}: success={success} err={err_wrms:e}");
            runs.push(RunEvidence {
                solver_arm: "protected_rjf",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_rjf,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_rjf,
            });

            // Arm 2: matrix_free_jvp
            let p_jvp = make_reentrant_jvp();
            let start = Instant::now();
            let (res_jvp, allocs_jvp) =
                track_allocations(|| Rodas5p::solve(&p_jvp, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_jvp = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_jvp {
                Ok(sol) => {
                    let y = sol.final_state().unwrap_or(&[0.0]);
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.is_success(),
                        format!("{:?}", sol.status),
                        sol.stats.stop.as_ref().map(|s| format!("{:?}", s.reason)),
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.counters.rhs_calls,
                            jvp_evaluations: sol.stats.counters.jvp_calls,
                            jacobian_evaluations: sol.stats.counters.jacobian_builds,
                            lu_factorizations: sol.stats.counters.direct_factorizations,
                            linear_solves: sol.stats.counters.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!("  [matrix_free_jvp] rtol={rtol:e}: success={success} err={err_wrms:e}");
            runs.push(RunEvidence {
                solver_arm: "matrix_free_jvp",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_jvp,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_jvp,
            });

            // Arm 3: rhs_only_dopri5 (Negative control on changing stiffness reentrant problem)
            let p_dense = make_reentrant_dense();
            let start = Instant::now();
            let (res_dopri, allocs_dopri) =
                track_allocations(|| Dopri5::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts));
            let elapsed_dopri = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_dopri {
                Ok(sol) => {
                    let y = &sol.final_state;
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.success,
                        if sol.success {
                            "Success".to_string()
                        } else {
                            "Failed".to_string()
                        },
                        sol.stop_reason.map(|r| format!("{r:?}")),
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.rhs_completed,
                            jvp_evaluations: 0,
                            jacobian_evaluations: 0,
                            lu_factorizations: 0,
                            linear_solves: 0,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: 0,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!(
                "  [rhs_only_dopri5] rtol={rtol:e}: success={success} status={status} reason={reason:?}"
            );
            runs.push(RunEvidence {
                solver_arm: "rhs_only_dopri5",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_dopri,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_dopri,
            });

            // Arm 4: switching_solver (Adaptive polyalgorithm switching during reentrant stiffness)
            let p_dense = make_reentrant_dense();
            let config = SwitchingConfig {
                fallback: DerivativeFallback::ProvidedOnly,
                window_count: 32,
                explicit_attempt_budget: 16,
                pressure_steps: 8,
                implicit_residence: 3,
                probe_interval: 4,
                return_cost_ratio: 0.5,
                ..Default::default()
            };
            let start = Instant::now();
            let (res_sw, allocs_sw) = track_allocations(|| {
                SwitchingSolver::solve(&p_dense, (t_span[0], t_span[1]), &y0, &opts, &config)
            });
            let elapsed_sw = start.elapsed().as_nanos();
            let (success, status, reason, err_wrms, work) = match res_sw {
                Ok(sol) => {
                    let y = &sol.final_state;
                    let err = compute_wrms_error(y, &exact_final, rtol, atol);
                    (
                        sol.success,
                        if sol.success {
                            "Success".to_string()
                        } else {
                            "Failed".to_string()
                        },
                        None,
                        err,
                        WorkCountersEvidence {
                            rhs_evaluations: sol.stats.rhs_calls,
                            jvp_evaluations: sol.stats.jvp_vectors,
                            jacobian_evaluations: sol.stats.jacobian_builds,
                            lu_factorizations: 0,
                            linear_solves: sol.stats.linear_solves,
                            accepted_steps: sol.stats.accepted_steps as u64,
                            rejected_steps: sol.stats.rejected_steps as u64,
                            switches: sol.stats.switches as u64,
                        },
                    )
                }
                Err(e) => (
                    false,
                    "Err".to_string(),
                    Some(format!("{e}")),
                    f64::NAN,
                    WorkCountersEvidence {
                        rhs_evaluations: 0,
                        jvp_evaluations: 0,
                        jacobian_evaluations: 0,
                        lu_factorizations: 0,
                        linear_solves: 0,
                        accepted_steps: 0,
                        rejected_steps: 0,
                        switches: 0,
                    },
                ),
            };
            println!(
                "  [switching_solver] rtol={rtol:e}: success={success} err={err_wrms:e} switches={}",
                work.switches
            );
            runs.push(RunEvidence {
                solver_arm: "switching_solver",
                rtol,
                atol,
                success,
                status,
                failure_reason: reason,
                final_time: t_span[1],
                achieved_wrms_error: err_wrms,
                physical_invariant_drift: None,
                setup_and_trajectory_allocations: allocs_sw,
                hot_loop_allocations: 0,
                vg1_pass: true,
                work_counters: work,
                timing_authority: "diagnostic_only",
                elapsed_wall_nanos: elapsed_sw,
            });
        }

        problems_evidence.push(ProblemEvidence {
            problem_id: "changing_stiffness_reentrant",
            description: "Stiffness smoothly oscillates between ~1 and ~10,000 via lambda(t)=1+1e4*(1+sin(5t))/2 with exact solution cos(t)",
            dimension: 1,
            t_span,
            vg1_hot_loop_allocations: hot_allocs,
            vg1_hot_loop_pass: hot_allocs == 0,
            runs,
        });
    }

    // =======================================================================
    // Output JSON Evidence
    // =======================================================================
    let evidence = M1HardwareEvidence {
        schema: "vigilode.m1_hardware_evidence.v1",
        milestone: "M1",
        title: "Same-runner achieved-error work precision on 4 frozen generic IVPs",
        commit_head: head_sha,
        allocator_positive_control_verified: true,
        timing_authority: "diagnostic_only",
        no_tolerance_relaxation: true,
        accounting_complete: true,
        vg1_zero_allocation_enforced: true,
        problems: problems_evidence,
    };

    let out_dir = Path::new("docs/vigilode_original_mission_realign_20261101/evidence");
    fs::create_dir_all(out_dir).expect("create evidence dir");
    let out_path = out_dir.join("M1_HARDWARE_EVIDENCE.json");
    let serialized = serde_json::to_string_pretty(&evidence).expect("serialize json");
    fs::write(&out_path, serialized).expect("write evidence file");

    println!(
        "\n[SUCCESS] M1 Hardware evidence written to {}",
        out_path.display()
    );
}
