//! WU-7 (audit F-048, F-022): one work-counter contract on both RODAS5P lanes.
//!
//! `jvp_calls` and `jvp_vectors` count user JVP callbacks actually executed.
//! A product with an explicit Jacobian matrix is not a JVP callback; it is
//! counted as `jacobian_matvecs`.  E-08 measured the generic GMRES lane
//! reporting 140 `jvp_calls` against 760 real callbacks on Prothero-Robinson,
//! and 140 with an explicit Jacobian where no callback ran.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::{InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{
    AdaptiveStepConfig, IntegrationMethod, OdeProblem, OutputSchedule,
    integrate_adaptive_observed_with_config, integrate_sequential_matrix_free_adaptive_observed,
};

struct Counted {
    problem: OdeProblem,
    y0: Vec<f64>,
    rhs: Arc<AtomicU64>,
    jvp: Arc<AtomicU64>,
    jacobian: Arc<AtomicU64>,
}

/// Prothero-Robinson `y' = lambda (y - sin t) + cos t + mu (y - sin t)^2`
/// with every callback counted outside the solver.
fn counted_prothero_robinson(explicit_jacobian: bool) -> Counted {
    let (lambda, mu) = (-1.0e4, 1.0);
    let rhs_calls = Arc::new(AtomicU64::new(0));
    let jvp_calls = Arc::new(AtomicU64::new(0));
    let jacobian_calls = Arc::new(AtomicU64::new(0));
    let rhs = {
        let calls = rhs_calls.clone();
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            calls.fetch_add(1, Ordering::Relaxed);
            let d = y[0] - t.sin();
            out[0] = lambda * d + t.cos() + mu * d * d;
            Ok(())
        })
    };
    let jvp = {
        let calls = jvp_calls.clone();
        Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
            calls.fetch_add(1, Ordering::Relaxed);
            out[0] = (lambda + 2.0 * mu * (y[0] - t.sin())) * v[0];
            Ok(())
        })
    };
    let jacobian = {
        let calls = jacobian_calls.clone();
        Arc::new(move |t: f64, y: &[f64]| {
            calls.fetch_add(1, Ordering::Relaxed);
            rodas5p_core::DenseMatrix::new(1, 1, vec![lambda + 2.0 * mu * (y[0] - t.sin())])
        })
    };
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let d = y[0] - t.sin();
        out[0] = -lambda * t.cos() - t.sin() - 2.0 * mu * d * t.cos();
        Ok(())
    });
    let problem = OdeProblem::new(
        if explicit_jacobian {
            "counted-pr-with-jacobian"
        } else {
            "counted-pr-jvp-only"
        },
        1,
        rhs,
        None,
        explicit_jacobian.then_some(jacobian),
        Some(jvp),
        Some(partial_t),
        false,
        None,
        None,
    )
    .unwrap();
    Counted {
        problem,
        y0: vec![0.0],
        rhs: rhs_calls,
        jvp: jvp_calls,
        jacobian: jacobian_calls,
    }
}

fn gmres() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: 8,
        maxiter: 64,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
        ..LinearSolverConfig::default()
    }
}

fn adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-10,
        rtol: 1.0e-8,
        initial_step: 0.1,
        ..AdaptiveStepConfig::default()
    }
}

#[test]
fn jvp_calls_equal_user_jvp_callbacks_on_the_generic_gmres_lane() {
    let counted = counted_prothero_robinson(false);
    let output = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let run = integrate_adaptive_observed_with_config(
        &counted.problem,
        (0.0, 1.0),
        &counted.y0,
        IntegrationMethod::Sequential,
        Some(&gmres()),
        None,
        &adaptive(),
        &output,
    )
    .unwrap();
    assert!(run.observed.success);
    let work = run.observed.counters;
    let callbacks = counted.jvp.load(Ordering::Relaxed);
    assert!(callbacks > 0);
    assert_eq!(work.jvp_calls, callbacks, "generic lane jvp_calls");
    assert_eq!(work.jvp_vectors, callbacks, "generic lane jvp_vectors");
    assert_eq!(work.rhs_evaluations, counted.rhs.load(Ordering::Relaxed));
}

#[test]
fn jvp_calls_equal_user_jvp_callbacks_on_the_matrix_free_lane() {
    let counted = counted_prothero_robinson(false);
    let output = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let run = integrate_sequential_matrix_free_adaptive_observed(
        &counted.problem,
        (0.0, 1.0),
        &counted.y0,
        &gmres(),
        &adaptive(),
        &output,
    )
    .unwrap();
    assert!(run.observed.success);
    let work = run.observed.counters;
    let callbacks = counted.jvp.load(Ordering::Relaxed);
    assert!(callbacks > 0);
    assert_eq!(work.jvp_calls, callbacks, "matrix-free lane jvp_calls");
    assert_eq!(work.jvp_vectors, callbacks, "matrix-free lane jvp_vectors");
    assert_eq!(work.rhs_evaluations, counted.rhs.load(Ordering::Relaxed));
}

#[test]
fn an_explicit_jacobian_lane_reports_no_jvp_callbacks() {
    let counted = counted_prothero_robinson(true);
    let output = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let run = integrate_adaptive_observed_with_config(
        &counted.problem,
        (0.0, 1.0),
        &counted.y0,
        IntegrationMethod::Sequential,
        Some(&gmres()),
        None,
        &adaptive(),
        &output,
    )
    .unwrap();
    assert!(run.observed.success);
    let work = run.observed.counters;
    assert_eq!(counted.jvp.load(Ordering::Relaxed), 0);
    assert!(counted.jacobian.load(Ordering::Relaxed) > 0);
    assert_eq!(work.jvp_calls, 0, "no JVP callback ran");
    assert_eq!(work.jvp_vectors, 0, "no JVP callback ran");
    assert!(
        work.jacobian_matvecs > 0,
        "explicit Jacobian products are counted as jacobian_matvecs"
    );
}
