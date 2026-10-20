//! Contracts of the `vigilode` facade (wave-1 node A1-api).
//!
//! Bit-identity tests compare the facade with the direct driver call under
//! identical settings using exact `f64` equality (`to_bits`).

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, OdeProblem, OutputSchedule, RODAS5P_FAST_BANDED_DRIVER_ID,
    RODAS5P_FAST_DRIVER_ID, RODAS5P_MF_FAST_DRIVER_ID, RoutedDriver, RoutingOptions,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_observed,
    integrate_rodas5p_mf_fast_observed_gmres_into, integrate_rodas5p_routed_observed,
    robertson_problem,
};
use vigilode::{
    Atol, Callback, CallbackError, DriverWork, FailureReason, Jacobian, JacobianRecord, Options,
    Problem, Rodas5p, STATS_SCHEMA, Solution, SolveError, Status, Tier,
};

// ---------------------------------------------------------------- helpers

type CoreBandFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|v| v.to_bits()).collect()
}

fn assert_states_bit_identical(facade: &Solution, t: &[f64], y: &[Vec<f64>]) {
    assert_eq!(bits(&facade.t), bits(t), "output times differ");
    assert_eq!(facade.y.len(), y.len());
    for (a, b) in facade.y.iter().zip(y) {
        assert_eq!(bits(a), bits(b), "states differ: {a:?} vs {b:?}");
    }
}

fn adaptive_for(opts: &Options) -> AdaptiveStepConfig {
    let Atol::Scalar(atol) = opts.atol else {
        panic!("scalar atol expected")
    };
    AdaptiveStepConfig {
        rtol: opts.rtol,
        atol,
        initial_step: opts.h0,
        max_attempts: opts.max_steps,
        ..AdaptiveStepConfig::default()
    }
}

fn robertson_rhs(_t: f64, y: &[f64], dy: &mut [f64]) -> Result<(), CallbackError> {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = 3.0e7 * y[1] * y[1];
    Ok(())
}

fn robertson_jac(_t: f64, y: &[f64], jac: &mut [f64]) -> Result<(), CallbackError> {
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
}

fn robertson(jacobian: Jacobian) -> Problem {
    Problem::new(3, robertson_rhs)
        .jacobian(jacobian)
        .autonomous()
}

const ROBERTSON_Y0: [f64; 3] = [1.0, 0.0, 0.0];
const ROBERTSON_SPAN: (f64, f64) = (0.0, 40.0);

fn robertson_opts(rtol: f64, atol: f64) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(atol),
        ..Options::default()
    }
}

/// Scaled final-state error against a reference.
fn scaled_error(y: &[f64], reference: &[f64], rtol: f64, atol: f64) -> f64 {
    y.iter()
        .zip(reference)
        .map(|(a, b)| (a - b).abs() / (atol + rtol * b.abs()))
        .fold(0.0, f64::max)
}

/// The two-timescale relaxation of `examples/solve_stiff.rs`.
fn two_timescale_facade() -> Problem {
    Problem::new(2, |_t, y, dy| {
        dy[0] = -y[0];
        dy[1] = -1000.0 * y[1];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_t, _y, v, out| {
        out[0] = -v[0];
        out[1] = -1000.0 * v[1];
        Ok(())
    }))
    .autonomous()
}

fn two_timescale_direct() -> OdeProblem {
    OdeProblem::new(
        "two-timescale-relaxation",
        2,
        Arc::new(|_t, y, out| {
            out[0] = -y[0];
            out[1] = -1000.0 * y[1];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_t, _y, v, out| {
            out[0] = -v[0];
            out[1] = -1000.0 * v[1];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn stiff_example_opts() -> Options {
    // The tolerances and first step of examples/solve_stiff.rs.
    Options {
        rtol: 1.0e-8,
        atol: Atol::Scalar(1.0e-11),
        h0: 1.0e-3,
        max_steps: 10_000,
        ..Options::default()
    }
}

/// `y_i' = y_{i-1} - 2 y_i - 0.5 y_i^2 + y_{i+1}` (tridiagonal Jacobian).
fn tridiagonal_rhs(n: usize) -> impl Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> {
    move |_t, y, dy| {
        for i in 0..n {
            let left = if i > 0 { y[i - 1] } else { 0.0 };
            let right = if i + 1 < n { y[i + 1] } else { 0.0 };
            dy[i] = left - 2.0 * y[i] - 0.5 * y[i] * y[i] + right;
        }
        Ok(())
    }
}

fn tridiagonal_band(n: usize) -> impl Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> {
    move |_t, y, band| {
        // width 3: [sub, diag, super] per row.
        for i in 0..n {
            if i > 0 {
                band[i * 3] = 1.0;
            }
            band[i * 3 + 1] = -2.0 - y[i];
            if i + 1 < n {
                band[i * 3 + 2] = 1.0;
            }
        }
        Ok(())
    }
}

fn tridiagonal_y0(n: usize) -> Vec<f64> {
    (0..n).map(|i| 1.0 + 0.1 * i as f64).collect()
}

// ------------------------------------------------- bit identity: Robertson

#[test]
fn robertson_dense_jacobian_is_bit_identical_to_the_direct_fast_driver() {
    let opts = robertson_opts(1.0e-6, 1.0e-10);
    let sol = Rodas5p::solve(
        &robertson(Jacobian::dense(robertson_jac)),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &opts,
    )
    .unwrap();
    assert_eq!(sol.status, Status::Success);

    let (direct_problem, y0) = robertson_problem().unwrap();
    assert_eq!(y0, ROBERTSON_Y0);
    let schedule = OutputSchedule::new(vec![ROBERTSON_SPAN.0, ROBERTSON_SPAN.1]).unwrap();
    let adaptive = adaptive_for(&opts);
    let direct =
        integrate_rodas5p_fast_observed(&direct_problem, ROBERTSON_SPAN, &y0, &adaptive, &schedule)
            .unwrap();
    assert!(direct.observed.success);
    assert_states_bit_identical(&sol, &direct.observed.t, &direct.observed.y);
    assert_eq!(sol.stats.driver_counters, direct.observed.counters);
    assert_eq!(sol.stats.counters, direct.observed.counters);
    assert_eq!(sol.stats.attempts, direct.attempts);
    assert_eq!(sol.stats.accepted_steps, direct.accepted_steps);
    assert_eq!(sol.stats.rejected_steps, direct.rejected_steps);
    assert_eq!(sol.stats.adaptive, adaptive);
    assert_eq!(sol.stats.route.driver_id, RODAS5P_FAST_DRIVER_ID);
    assert_eq!(sol.stats.jacobian, JacobianRecord::Dense);

    // The router on the library problem takes the same driver.
    let routed = integrate_rodas5p_routed_observed(
        &direct_problem,
        ROBERTSON_SPAN,
        &y0,
        &adaptive,
        &schedule,
    )
    .unwrap();
    assert_eq!(routed.routing.driver, RoutedDriver::Dense);
    assert_states_bit_identical(&sol, &routed.run.observed().t, &routed.run.observed().y);
}

#[test]
fn performance_tier_executes_the_protected_route_bit_for_bit() {
    let problem = robertson(Jacobian::dense(robertson_jac));
    let protected = Rodas5p::solve(
        &problem,
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &robertson_opts(1.0e-6, 1.0e-10),
    )
    .unwrap();
    let performance = Rodas5p::solve(
        &problem,
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            tier: Tier::Performance,
            ..robertson_opts(1.0e-6, 1.0e-10)
        },
    )
    .unwrap();
    assert_states_bit_identical(&performance, &protected.t, &protected.y);
    assert_eq!(performance.stats.counters, protected.stats.counters);
    assert_eq!(performance.stats.tier_requested, Tier::Performance);
    assert_eq!(performance.stats.tier_executed, Tier::Protected);
    assert_eq!(protected.stats.tier_requested, Tier::Protected);
}

// ------------------------------------- bit identity: the solve_stiff example

#[test]
fn stiff_example_jvp_is_bit_identical_to_the_direct_matrix_free_driver() {
    let opts = stiff_example_opts();
    let span = (0.0, 1.0);
    let y0 = [1.0, 1.0];
    let sol = Rodas5p::solve(&two_timescale_facade(), span, &y0, &opts).unwrap();
    assert_eq!(sol.status, Status::Success);

    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let adaptive = adaptive_for(&opts);
    let linear = RoutingOptions::default().matrix_free;
    let direct = integrate_rodas5p_mf_fast_observed_gmres_into(
        &two_timescale_direct(),
        span,
        &y0,
        &linear,
        &adaptive,
        &schedule,
    )
    .unwrap();
    assert!(direct.observed.success);
    assert_states_bit_identical(&sol, &direct.observed.t, &direct.observed.y);
    assert_eq!(sol.stats.counters, direct.observed.counters);
    assert_eq!(sol.stats.attempts, direct.attempts);
    assert_eq!(sol.stats.route.driver_id, RODAS5P_MF_FAST_DRIVER_ID);
    assert_eq!(sol.stats.jacobian, JacobianRecord::Jvp);
    assert_eq!(sol.stats.counters.jacobian_builds, 0);
    assert!(sol.stats.matrix_free_linear.is_some());

    // The example's accuracy holds through the facade too.
    let y = sol.final_state().unwrap();
    assert!((y[0] - (-1.0_f64).exp()).abs() <= 1.0e-6);
    assert!((y[1] - (-1000.0_f64).exp()).abs() <= 1.0e-6);
}

// ------------------------------------------------- finite differences

#[test]
fn finite_difference_robertson_is_within_twice_the_analytic_error() {
    let (rtol, atol) = (1.0e-6, 1.0e-10);
    let reference = Rodas5p::solve(
        &robertson(Jacobian::dense(robertson_jac)),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &robertson_opts(1.0e-12, 1.0e-16),
    )
    .unwrap();
    assert!(reference.is_success());
    let reference = reference.final_state().unwrap().to_vec();

    let analytic = Rodas5p::solve(
        &robertson(Jacobian::dense(robertson_jac)),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &robertson_opts(rtol, atol),
    )
    .unwrap();
    let fd = Rodas5p::solve(
        &robertson(Jacobian::FiniteDifference),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &robertson_opts(rtol, atol),
    )
    .unwrap();
    assert!(analytic.is_success() && fd.is_success());

    let e_analytic = scaled_error(analytic.final_state().unwrap(), &reference, rtol, atol);
    let e_fd = scaled_error(fd.final_state().unwrap(), &reference, rtol, atol);
    eprintln!("robertson scaled error: analytic {e_analytic:e}, fd {e_fd:e}");
    assert!(
        e_fd.is_finite() && e_fd <= 2.0 * e_analytic,
        "fd {e_fd:e} vs analytic {e_analytic:e}"
    );

    // FD work is counted as RHS calls: n + 1 per dense build.
    let work = fd.stats.finite_difference;
    assert!(work.jacobian_builds > 0);
    assert_eq!(work.rhs_calls, 4 * work.jacobian_builds);
    assert_eq!(
        fd.stats.counters.rhs_calls,
        fd.stats.driver_counters.rhs_calls + work.rhs_calls
    );
    assert_eq!(
        fd.stats.counters.rhs_evaluations,
        fd.stats.driver_counters.rhs_evaluations + work.rhs_calls
    );
    assert_eq!(fd.stats.counters.jacobian_builds, work.jacobian_builds);
    assert!(matches!(
        fd.stats.jacobian,
        JacobianRecord::FiniteDifferenceDense { .. }
    ));
    assert_eq!(fd.stats.route.driver_id, RODAS5P_FAST_DRIVER_ID);
    assert_eq!(analytic.stats.finite_difference.rhs_calls, 0);
}

#[test]
fn banded_finite_difference_uses_grouped_columns_and_the_banded_driver() {
    let n = 20;
    let span = (0.0, 2.0);
    let y0 = tridiagonal_y0(n);
    let (rtol, atol) = (1.0e-6, 1.0e-10);
    let opts = robertson_opts(rtol, atol);

    let analytic_problem = Problem::new(n, tridiagonal_rhs(n))
        .jacobian(Jacobian::banded(1, 1, tridiagonal_band(n)))
        .autonomous();
    let fd_problem = Problem::new(n, tridiagonal_rhs(n))
        .banded(1, 1)
        .autonomous();
    let reference = Rodas5p::solve(&analytic_problem, span, &y0, &robertson_opts(1e-12, 1e-16))
        .unwrap()
        .final_state()
        .unwrap()
        .to_vec();
    let analytic = Rodas5p::solve(&analytic_problem, span, &y0, &opts).unwrap();
    let fd = Rodas5p::solve(&fd_problem, span, &y0, &opts).unwrap();
    assert!(analytic.is_success() && fd.is_success());
    assert_eq!(fd.stats.route.driver_id, RODAS5P_FAST_BANDED_DRIVER_ID);
    assert_eq!(
        analytic.stats.route.driver_id,
        RODAS5P_FAST_BANDED_DRIVER_ID
    );
    assert_eq!(
        fd.stats.jacobian,
        JacobianRecord::FiniteDifferenceBanded {
            lower: 1,
            upper: 1,
            groups: 3,
            increment: vigilode::FD_INCREMENT_RULE,
        }
    );
    let work = fd.stats.finite_difference;
    assert!(work.jacobian_builds > 0);
    assert_eq!(
        work.rhs_calls,
        4 * work.jacobian_builds,
        "3 groups + f(t, y)"
    );
    let e_analytic = scaled_error(analytic.final_state().unwrap(), &reference, rtol, atol);
    let e_fd = scaled_error(fd.final_state().unwrap(), &reference, rtol, atol);
    eprintln!("tridiagonal scaled error: analytic {e_analytic:e}, fd {e_fd:e}");
    assert!(
        e_fd.is_finite() && e_fd <= 2.0 * e_analytic,
        "fd {e_fd:e} vs analytic {e_analytic:e}"
    );
    // The banded driver never asks for a JVP.
    assert_eq!(fd.stats.counters.jvp_calls, 0);
}

#[test]
fn banded_user_jacobian_is_bit_identical_to_the_direct_banded_driver() {
    let n = 20;
    let span = (0.0, 2.0);
    let y0 = tridiagonal_y0(n);
    let opts = robertson_opts(1.0e-6, 1.0e-10);
    let sol = Rodas5p::solve(
        &Problem::new(n, tridiagonal_rhs(n))
            .jacobian(Jacobian::banded(1, 1, tridiagonal_band(n)))
            .autonomous(),
        span,
        &y0,
        &opts,
    )
    .unwrap();

    let rhs = tridiagonal_rhs(n);
    let band = tridiagonal_band(n);
    let fill: CoreBandFn = Arc::new(move |t, y, out| {
        band(t, y, out).unwrap();
        Ok(())
    });
    let fill_for_jac = fill.clone();
    let direct_problem = OdeProblem::new(
        "tridiagonal",
        n,
        Arc::new(move |t, y, out| {
            rhs(t, y, out).unwrap();
            Ok(())
        }),
        None,
        Some(Arc::new(move |t, y| {
            let mut values = vec![0.0; 3 * n];
            fill_for_jac(t, y, &mut values)?;
            let mut m = DenseMatrix::zeros(n, n);
            for i in 0..n {
                for j in i.saturating_sub(1)..=(i + 1).min(n - 1) {
                    m[(i, j)] = values[i * 3 + (j + 1 - i)];
                }
            }
            Ok(m)
        })),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let direct = integrate_rodas5p_fast_banded_observed(
        &direct_problem,
        &BandedJacobian {
            lower: 1,
            upper: 1,
            fill,
        },
        span,
        &y0,
        &adaptive_for(&opts),
        &OutputSchedule::new(vec![span.0, span.1]).unwrap(),
    )
    .unwrap();
    assert_states_bit_identical(&sol, &direct.fast.observed.t, &direct.fast.observed.y);
    assert_eq!(sol.stats.counters, direct.fast.observed.counters);
}

#[test]
fn dense_jacobian_with_declared_band_is_verified_and_routed_to_the_banded_driver() {
    let n = 12;
    let dense = move |_t: f64, y: &[f64], jac: &mut [f64]| -> Result<(), CallbackError> {
        for i in 0..n {
            if i > 0 {
                jac[i * n + i - 1] = 1.0;
            }
            jac[i * n + i] = -2.0 - y[i];
            if i + 1 < n {
                jac[i * n + i + 1] = 1.0;
            }
        }
        Ok(())
    };
    let problem = Problem::new(n, tridiagonal_rhs(n))
        .jacobian(Jacobian::dense(dense))
        .banded(1, 1)
        .autonomous();
    let sol = Rodas5p::solve(
        &problem,
        (0.0, 1.0),
        &tridiagonal_y0(n),
        &Options::default(),
    )
    .unwrap();
    assert!(sol.is_success());
    assert_eq!(sol.stats.route.driver_id, RODAS5P_FAST_BANDED_DRIVER_ID);
    let routing = sol.stats.route.routing.as_ref().expect("router record");
    assert_eq!(routing.driver, RoutedDriver::Banded);
    assert!(routing.verification.is_some());

    // A wrong band is refused before any step.
    let wrong = Problem::new(n, tridiagonal_rhs(n))
        .jacobian(Jacobian::dense(move |_t, _y, jac: &mut [f64]| {
            for i in 0..n {
                jac[i * n + i] = -2.0;
                jac[i * n + (i + 2) % n] = 1.0; // outside the declared band
            }
            Ok(())
        }))
        .banded(1, 1)
        .autonomous();
    let err =
        Rodas5p::solve(&wrong, (0.0, 1.0), &tridiagonal_y0(n), &Options::default()).unwrap_err();
    assert!(matches!(err, SolveError::StructureRefused(_)), "{err:?}");
}

// ------------------------------------------------- failures

/// `y' = -y`, failing fatally once `t > 0.5`.
fn failing_problem(calls: Arc<AtomicU64>) -> Problem {
    Problem::new(1, move |t, y, dy| {
        calls.fetch_add(1, Ordering::Relaxed);
        if t > 0.5 {
            return Err(CallbackError::Fatal(
                "model undefined beyond t = 0.5".into(),
            ));
        }
        dy[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, jac| {
        jac[0] = -1.0;
        Ok(())
    }))
    .autonomous()
}

#[test]
fn a_failing_rhs_returns_the_partial_solution_and_a_typed_reason() {
    let calls = Arc::new(AtomicU64::new(0));
    let sol = Rodas5p::solve(
        &failing_problem(calls.clone()),
        (0.0, 1.0),
        &[1.0],
        &Options::default(),
    )
    .expect("a failed run is a solution, not an error");
    match &sol.status {
        Status::Failed(FailureReason::CallbackFailed {
            callback,
            t,
            message,
        }) => {
            assert_eq!(*callback, Callback::Rhs);
            assert!(*t > 0.5 && *t <= 1.0, "t = {t}");
            assert!(message.contains("undefined"));
        }
        other => panic!("unexpected status {other:?}"),
    }
    // The partial solution: t0 only (t1 was not reached), with all work kept.
    assert_eq!(sol.t, vec![0.0]);
    assert_eq!(sol.y, vec![vec![1.0]]);
    assert!(sol.stats.accepted_steps > 0);
    assert!(sol.stats.rejected_steps > 0);
    assert_eq!(sol.stats.callbacks.fatal_errors, 1);
    assert!(sol.stats.callbacks.short_circuited_calls > 0);
    // User code was entered exactly rhs_calls - short_circuited times.
    assert_eq!(
        calls.load(Ordering::Relaxed),
        sol.stats.counters.rhs_calls - sol.stats.callbacks.short_circuited_rhs_calls
    );
    assert!(
        sol.stats.callbacks.short_circuited_rhs_calls <= sol.stats.callbacks.short_circuited_calls
    );
    let err = sol.ensure_success().unwrap_err();
    match err {
        SolveError::Failed { reason, partial } => {
            assert!(matches!(reason, FailureReason::CallbackFailed { .. }));
            assert_eq!(partial.t, vec![0.0]);
        }
        other => panic!("unexpected error {other:?}"),
    }
}

#[test]
fn a_recoverable_rhs_error_rejects_the_step_and_the_run_succeeds() {
    // y' = -y; the RHS refuses states it considers too far from the
    // solution's range once (a large first step), then the run continues.
    let refused = Arc::new(AtomicU64::new(0));
    let r = refused.clone();
    let problem = Problem::new(1, move |t, y, dy| {
        if t > 0.2 && r.load(Ordering::Relaxed) == 0 {
            r.fetch_add(1, Ordering::Relaxed);
            return Err(CallbackError::Recoverable("first stage beyond 0.2".into()));
        }
        dy[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, jac| {
        jac[0] = -1.0;
        Ok(())
    }))
    .autonomous();
    let sol = Rodas5p::solve(&problem, (0.0, 1.0), &[1.0], &Options::default()).unwrap();
    assert!(sol.is_success());
    assert_eq!(sol.stats.callbacks.recoverable_errors, 1);
    assert_eq!(sol.stats.counters.nonfinite_step_failures, 1);
    assert!((sol.final_state().unwrap()[0] - (-1.0_f64).exp()).abs() < 1.0e-6);
}

#[test]
fn an_exhausted_attempt_budget_is_a_typed_failure() {
    let sol = Rodas5p::solve(
        &robertson(Jacobian::dense(robertson_jac)),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            max_steps: 3,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(
        sol.status,
        Status::Failed(FailureReason::MaxStepsReached { max_steps: 3 })
    );
    assert_eq!(sol.stats.attempts, 3);
    assert_eq!(sol.t, vec![0.0]);
}

// ------------------------------------------------- refusals

#[test]
fn unsupported_requests_are_refused_not_substituted() {
    let problem = robertson(Jacobian::dense(robertson_jac));
    let refuse = |problem: &Problem, opts: &Options| match Rodas5p::solve(
        problem,
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        opts,
    ) {
        Err(SolveError::Unsupported(_)) => {}
        other => panic!("expected Unsupported, got {other:?}"),
    };
    refuse(
        &problem,
        &Options {
            atol: Atol::Vector(vec![1e-10, 1e-12, 1e-10]),
            ..Options::default()
        },
    );
    refuse(
        &problem,
        &Options {
            dense_output: true,
            ..Options::default()
        },
    );
    refuse(
        &problem
            .clone()
            .mass(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
        &Options::default(),
    );
    refuse(
        &robertson(Jacobian::jvp(|_t, _y, v, out| {
            out.copy_from_slice(v);
            Ok(())
        }))
        .banded(1, 1),
        &Options::default(),
    );
}

#[test]
fn malformed_input_is_refused_before_any_step() {
    let problem = robertson(Jacobian::dense(robertson_jac));
    let invalid =
        |problem: &Problem, span: (f64, f64), y0: &[f64], opts: &Options| match Rodas5p::solve(
            problem, span, y0, opts,
        ) {
            Err(SolveError::InvalidInput(_)) => {}
            other => panic!("expected InvalidInput, got {other:?}"),
        };
    let o = Options::default();
    invalid(&problem, ROBERTSON_SPAN, &[1.0, 0.0], &o);
    invalid(&problem, ROBERTSON_SPAN, &[f64::NAN, 0.0, 0.0], &o);
    invalid(&problem, (1.0, 0.0), &ROBERTSON_Y0, &o);
    invalid(&problem, (0.0, f64::INFINITY), &ROBERTSON_Y0, &o);
    invalid(
        &problem,
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            h0: 0.0,
            ..o.clone()
        },
    );
    invalid(
        &problem,
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            max_steps: 0,
            ..o.clone()
        },
    );
    invalid(
        &problem,
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            rtol: -1.0,
            ..o.clone()
        },
    );
    invalid(
        &problem,
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            atol: Atol::Vector(vec![1e-10; 2]),
            ..o.clone()
        },
    );
    invalid(&Problem::new(0, robertson_rhs), (0.0, 1.0), &[], &o);
    invalid(
        &problem.clone().time_derivative(|_t, _y, out| {
            out.fill(0.0);
            Ok(())
        }),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &o,
    );
    invalid(
        &robertson(Jacobian::banded(1, 1, |_t, _y, _b| Ok(()))).banded(1, 2),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &o,
    );
    invalid(
        &robertson(Jacobian::banded(3, 0, |_t, _y, _b| Ok(()))),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &o,
    );
}

// ------------------------------------------------- time dependence

#[test]
fn explicit_time_derivative_drives_a_nonautonomous_problem() {
    // y' = -50 (y - cos t), y(0) = 1: a stiff relaxation onto cos t.
    let problem = Problem::new(1, |t, y, dy| {
        dy[0] = -50.0 * (y[0] - t.cos());
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, jac| {
        jac[0] = -50.0;
        Ok(())
    }))
    .time_derivative(|t, _y, out| {
        out[0] = -50.0 * t.sin();
        Ok(())
    });
    let sol = Rodas5p::solve(&problem, (0.0, 2.0), &[1.0], &robertson_opts(1e-8, 1e-10)).unwrap();
    assert!(sol.is_success());
    assert!(sol.stats.time_derivative && !sol.stats.autonomous);
    assert!(sol.stats.counters.ft_calls > 0);
    // Exact: y = (2500 cos t + 50 sin t)/2501 + e^{-50 t}/2501.
    let t: f64 = 2.0;
    let exact = (2500.0 * t.cos() + 50.0 * t.sin()) / 2501.0 + (-50.0 * t).exp() / 2501.0;
    assert!((sol.final_state().unwrap()[0] - exact).abs() < 1.0e-6);
}

// ------------------------------------------------- record

#[test]
fn stats_serialize_every_option_and_counter() {
    let opts = Options {
        tier: Tier::Performance,
        ..robertson_opts(1.0e-6, 1.0e-10)
    };
    let sol = Rodas5p::solve(
        &robertson(Jacobian::FiniteDifference),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &opts,
    )
    .unwrap();
    let record = serde_json::to_value(&sol.stats).unwrap();
    assert_eq!(record["schema"], STATS_SCHEMA);
    assert_eq!(record["options"]["rtol"], 1.0e-6);
    assert_eq!(record["options"]["tier"], "performance");
    assert_eq!(record["tier_executed"], "protected");
    for key in [
        "atol",
        "rtol",
        "initial_step",
        "min_step",
        "max_step",
        "max_attempts",
        "safety",
        "min_factor",
        "max_factor",
        "reject_max_factor",
        "controller",
        "max_step_policy",
    ] {
        assert!(
            record["adaptive"].get(key).is_some(),
            "adaptive.{key} missing"
        );
    }
    for key in [
        "rhs_calls",
        "jacobian_builds",
        "direct_factorizations",
        "rejected_steps",
    ] {
        assert!(
            record["counters"].get(key).is_some(),
            "counters.{key} missing"
        );
    }
    assert!(record["finite_difference"]["rhs_calls"].as_u64().unwrap() > 0);
    assert_eq!(record["route"]["driver_id"], RODAS5P_FAST_DRIVER_ID);
    assert_eq!(record["y0"], serde_json::json!(ROBERTSON_Y0));

    // The options round-trip: the run is reproducible from its record.
    let replay_opts: Options = serde_json::from_value(record["options"].clone()).unwrap();
    assert_eq!(replay_opts, opts);
    let replay = Rodas5p::solve(
        &robertson(Jacobian::FiniteDifference),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &replay_opts,
    )
    .unwrap();
    assert_states_bit_identical(&replay, &sol.t, &sol.y);
    assert_eq!(replay.stats.counters, sol.stats.counters);
}

// ------------------------------------------------- driver work in the record

fn driver_work_json(sol: &Solution) -> serde_json::Value {
    serde_json::to_value(&sol.stats).unwrap()["driver_work"].clone()
}

#[test]
fn dense_route_records_jacobian_reuses_and_lu() {
    let opts = robertson_opts(1.0e-6, 1.0e-10);
    let sol = Rodas5p::solve(
        &robertson(Jacobian::dense(robertson_jac)),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &opts,
    )
    .unwrap();
    let (direct_problem, y0) = robertson_problem().unwrap();
    let direct = integrate_rodas5p_fast_observed(
        &direct_problem,
        ROBERTSON_SPAN,
        &y0,
        &adaptive_for(&opts),
        &OutputSchedule::new(vec![ROBERTSON_SPAN.0, ROBERTSON_SPAN.1]).unwrap(),
    )
    .unwrap();
    assert_eq!(
        sol.stats.driver_work,
        DriverWork::Dense {
            jacobian_reuses: direct.jacobian_reuses,
            lu: direct.lu,
        }
    );
    let record = driver_work_json(&sol);
    assert_eq!(record["driver"], "dense");
    assert_eq!(record["jacobian_reuses"], direct.jacobian_reuses);
    assert_eq!(record["lu"], serde_json::to_value(direct.lu).unwrap());
}

#[test]
fn banded_routes_record_band_work() {
    let n = 20;
    let span = (0.0, 2.0);
    let y0 = tridiagonal_y0(n);
    // Direct banded driver (user band).
    let direct_route = Rodas5p::solve(
        &Problem::new(n, tridiagonal_rhs(n))
            .jacobian(Jacobian::banded(1, 1, tridiagonal_band(n)))
            .autonomous(),
        span,
        &y0,
        &Options::default(),
    )
    .unwrap();
    // Router rule 1 (dense callback, declared band).
    let dense = move |_t: f64, y: &[f64], jac: &mut [f64]| -> Result<(), CallbackError> {
        for i in 0..n {
            if i > 0 {
                jac[i * n + i - 1] = 1.0;
            }
            jac[i * n + i] = -2.0 - y[i];
            if i + 1 < n {
                jac[i * n + i + 1] = 1.0;
            }
        }
        Ok(())
    };
    let routed = Rodas5p::solve(
        &Problem::new(n, tridiagonal_rhs(n))
            .jacobian(Jacobian::dense(dense))
            .banded(1, 1)
            .autonomous(),
        span,
        &y0,
        &Options::default(),
    )
    .unwrap();
    for sol in [&direct_route, &routed] {
        assert!(sol.is_success());
        let DriverWork::Banded { banded, .. } = sol.stats.driver_work else {
            panic!("banded work expected: {:?}", sol.stats.driver_work)
        };
        assert!(banded.factor_operations > 0 && banded.solve_operations > 0);
        assert!(banded.stored_slots > 0);
        let record = driver_work_json(sol);
        assert_eq!(record["driver"], "banded");
        for key in ["factor_operations", "solve_operations", "stored_slots"] {
            assert!(record["banded"].get(key).is_some(), "banded.{key} missing");
        }
        assert!(record.get("jacobian_reuses").is_some() && record.get("lu").is_some());
    }
}

#[test]
fn matrix_free_route_records_state_reuses_and_policy_ids() {
    let opts = stiff_example_opts();
    let sol = Rodas5p::solve(&two_timescale_facade(), (0.0, 1.0), &[1.0, 1.0], &opts).unwrap();
    let direct = integrate_rodas5p_mf_fast_observed_gmres_into(
        &two_timescale_direct(),
        (0.0, 1.0),
        &[1.0, 1.0],
        &RoutingOptions::default().matrix_free,
        &adaptive_for(&opts),
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
    )
    .unwrap();
    let DriverWork::MatrixFree {
        state_reuses,
        gcrodr_policy,
        krylov_workspace,
        start_policy,
        recycle_scope,
        start_telemetry,
    } = &sol.stats.driver_work
    else {
        panic!("matrix-free work expected")
    };
    assert_eq!(*state_reuses, direct.state_reuses);
    assert_eq!(*gcrodr_policy, direct.gcrodr_policy);
    assert_eq!(*krylov_workspace, direct.krylov_workspace);
    assert_eq!(*start_policy, direct.start_policy);
    assert_eq!(*recycle_scope, direct.recycle_scope);
    assert_eq!(start_telemetry.is_some(), direct.start_telemetry.is_some());
    let record = driver_work_json(&sol);
    assert_eq!(record["driver"], "matrix-free");
    for key in [
        "state_reuses",
        "gcrodr_policy",
        "krylov_workspace",
        "start_policy",
        "recycle_scope",
        "start_telemetry",
    ] {
        assert!(record.get(key).is_some(), "driver_work.{key} missing");
    }
    assert_eq!(record["gcrodr_policy"], direct.gcrodr_policy);
}
