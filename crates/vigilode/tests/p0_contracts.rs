//! Protected-tier completeness of the facade (wave-2 node `p0-facade`,
//! plan §3 P0 tier): mass matrices and dense output on the protected
//! sequential dense driver with interior error control, `Solution::at`, the
//! partial solution after a failure, and typed failure reasons.
//!
//! Bit-identity tests compare the facade with the direct driver call under
//! identical settings with exact `f64` equality (`to_bits`).

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveFailureKind, AdaptiveStepConfig, BandedJacobian, DenseErrorControl, DriverStopReason,
    IntegrationMethod, InteriorSampling, OdeProblem, OutputSamplingPlan, OutputSchedule,
    RODAS5P_SEQUENTIAL_DENSE_DRIVER_ID, RoutingOptions,
    integrate_adaptive_dense_observed_with_dense_error_control,
    integrate_adaptive_dense_observed_with_segments, integrate_rodas5p_fast_banded_observed,
    integrate_rodas5p_fast_observed_with_partial, integrate_rodas5p_mf_fast_observed_gmres_into,
    robertson_problem,
};
use vigilode::{
    Atol, Callback, CallbackError, DriverWork, FailureReason, InteriorError, Jacobian, Options,
    Problem, Rodas5p, Solution, SolveError, Status,
};

type JacobianFn = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
type JvpFn = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type BandFillFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

// ---------------------------------------------------------------- helpers

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|v| v.to_bits()).collect()
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

fn span_plan(t0: f64, t1: f64) -> OutputSamplingPlan {
    OutputSamplingPlan::dense(OutputSchedule::new(vec![t0, t1]).unwrap())
}

fn robertson_rhs(_t: f64, y: &[f64], dy: &mut [f64]) -> Result<(), CallbackError> {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = 3.0e7 * y[1] * y[1];
    Ok(())
}

fn robertson_jac(_t: f64, y: &[f64], jac: &mut [f64]) -> Result<(), CallbackError> {
    jac.copy_from_slice(&[
        -0.04,
        1.0e4 * y[2],
        1.0e4 * y[1],
        0.04,
        -1.0e4 * y[2] - 6.0e7 * y[1],
        -1.0e4 * y[1],
        0.0,
        6.0e7 * y[1],
        0.0,
    ]);
    Ok(())
}

fn robertson() -> Problem {
    Problem::new(3, robertson_rhs)
        .jacobian(Jacobian::dense(robertson_jac))
        .autonomous()
}

const ROBERTSON_Y0: [f64; 3] = [1.0, 0.0, 0.0];
const ROBERTSON_SPAN: (f64, f64) = (0.0, 40.0);

fn opts(rtol: f64, atol: f64) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(atol),
        ..Options::default()
    }
}

fn dense_opts(rtol: f64, atol: f64, interior_error: InteriorError) -> Options {
    Options {
        dense_output: true,
        interior_error,
        ..opts(rtol, atol)
    }
}

fn sequential_work(sol: &Solution) -> (DenseErrorControl, InteriorSampling, usize, usize) {
    match &sol.stats.driver_work {
        DriverWork::SequentialDense {
            control,
            interior_sampling,
            samples,
            segments,
            ..
        } => (*control, *interior_sampling, *samples, *segments),
        other => panic!("expected the sequential dense driver, got {other:?}"),
    }
}

/// Prothero–Robinson `y' = lambda (y - sin t) + cos t`, exact `y = sin t`.
fn prothero_robinson(lambda: f64) -> Problem {
    Problem::new(1, move |t, y, dy| {
        dy[0] = lambda * (y[0] - t.sin()) + t.cos();
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_t, _y, jac| {
        jac[0] = lambda;
        Ok(())
    }))
    .time_derivative(move |t, _y, out| {
        out[0] = -lambda * t.cos() - t.sin();
        Ok(())
    })
}

fn worst_interior(sol: &Solution, rtol: f64, atol: f64) -> f64 {
    (0..=200)
        .map(|i| {
            let t = i as f64 * 0.01;
            let y = sol.at(t).expect("inside the span")[0];
            (y - t.sin()).abs() / (atol + rtol * t.sin().abs())
        })
        .fold(0.0_f64, f64::max)
}

// ------------------------------------------------------------ dense output

#[test]
fn dense_output_is_bit_identical_to_the_direct_segment_driver() {
    let o = dense_opts(1.0e-6, 1.0e-10, InteriorError::default());
    assert_eq!(o.interior_error, InteriorError::WFilteredEnforce);
    let sol = Rodas5p::solve(&robertson(), ROBERTSON_SPAN, &ROBERTSON_Y0, &o).unwrap();
    assert_eq!(sol.status, Status::Success);

    let (problem, y0) = robertson_problem().unwrap();
    let direct = integrate_adaptive_dense_observed_with_segments(
        &problem,
        ROBERTSON_SPAN,
        &y0,
        IntegrationMethod::Sequential,
        None,
        None,
        &adaptive_for(&o),
        &span_plan(ROBERTSON_SPAN.0, ROBERTSON_SPAN.1),
        DenseErrorControl::WFilteredEnforce,
        InteriorSampling::EveryStep,
    )
    .unwrap();
    assert!(direct.result.observed.success);
    // The solution holds t0 and every accepted step.
    let mut t = vec![ROBERTSON_SPAN.0];
    t.extend(direct.segments.iter().map(|s| s.t_new));
    assert_eq!(bits(&sol.t), bits(&t));
    assert_eq!(bits(&sol.y[0]), bits(&y0));
    for (y, s) in sol.y[1..].iter().zip(&direct.segments) {
        assert_eq!(bits(y), bits(&s.y_new));
    }
    assert_eq!(sol.stats.driver_counters, direct.result.observed.counters);
    assert_eq!(sol.stats.counters, direct.result.observed.counters);
    assert_eq!(sol.stats.attempts, direct.result.diagnostics.attempts);
    assert_eq!(
        sol.stats.route.driver_id,
        RODAS5P_SEQUENTIAL_DENSE_DRIVER_ID
    );
    assert_eq!(
        sol.stats.route.entry_point,
        "integrate_adaptive_dense_observed_with_segments"
    );
    let (control, sampling, samples, segments) = sequential_work(&sol);
    assert_eq!(control, DenseErrorControl::WFilteredEnforce);
    assert_eq!(sampling, InteriorSampling::EveryStep);
    assert_eq!(samples, direct.report.samples.len());
    assert_eq!(segments, direct.segments.len());
    assert!(samples >= segments && segments > 0);

    // at(t): the continuous extension inside a step, bit for bit.
    for s in direct.segments.iter().step_by(7) {
        let mid = s.t_old + 0.37 * (s.t_new - s.t_old);
        assert_eq!(
            bits(&sol.at(mid).unwrap()),
            bits(&s.evaluate(mid).unwrap()),
            "t = {mid}"
        );
    }
    assert_eq!(bits(&sol.at(0.0).unwrap()), bits(&y0));
    assert_eq!(
        bits(&sol.at(40.0).unwrap()),
        bits(sol.final_state().unwrap())
    );
    assert_eq!(sol.at(-1.0), None);
    assert_eq!(sol.at(40.5), None);
    assert_eq!(sol.at(f64::NAN), None);
}

#[test]
fn interior_error_modes_route_with_the_named_control() {
    let (problem, y0) = robertson_problem().unwrap();
    // Off: the named driver `integrate_adaptive_dense_observed_with_dense_error_control`
    // sampled at the facade's step endpoints, bit for bit (output times
    // never shorten a step on this path).
    let o = dense_opts(1.0e-6, 1.0e-10, InteriorError::Off);
    let sol = Rodas5p::solve(&robertson(), ROBERTSON_SPAN, &ROBERTSON_Y0, &o).unwrap();
    assert!(sol.is_success());
    let (control, _, samples, _) = sequential_work(&sol);
    assert_eq!((control, samples), (DenseErrorControl::Off, 0));
    let plan = OutputSamplingPlan::dense(OutputSchedule::new(sol.t.clone()).unwrap());
    let (named, report) = integrate_adaptive_dense_observed_with_dense_error_control(
        &problem,
        ROBERTSON_SPAN,
        &y0,
        IntegrationMethod::Sequential,
        None,
        None,
        &adaptive_for(&o),
        &plan,
        DenseErrorControl::Off,
    )
    .unwrap();
    assert!(report.samples.is_empty());
    assert_eq!(bits(&named.observed.t), bits(&sol.t));
    for (a, b) in named.observed.y.iter().zip(&sol.y) {
        assert_eq!(bits(a), bits(b));
    }
    assert_eq!(named.observed.counters, sol.stats.counters);

    // Enforce: the shift-h/2 filter on every step.
    let o = dense_opts(1.0e-6, 1.0e-10, InteriorError::Enforce);
    let sol = Rodas5p::solve(&robertson(), ROBERTSON_SPAN, &ROBERTSON_Y0, &o).unwrap();
    let direct = integrate_adaptive_dense_observed_with_segments(
        &problem,
        ROBERTSON_SPAN,
        &y0,
        IntegrationMethod::Sequential,
        None,
        None,
        &adaptive_for(&o),
        &span_plan(ROBERTSON_SPAN.0, ROBERTSON_SPAN.1),
        DenseErrorControl::Enforce,
        InteriorSampling::EveryStep,
    )
    .unwrap();
    assert_eq!(sequential_work(&sol).0, DenseErrorControl::Enforce);
    assert_eq!(sol.t.len(), direct.segments.len() + 1);
    assert_eq!(
        bits(sol.final_state().unwrap()),
        bits(&direct.segments.last().unwrap().y_new)
    );
    assert_eq!(sol.stats.counters, direct.result.observed.counters);
}

#[test]
fn prothero_robinson_interior_error_is_controlled_through_the_facade() {
    let (rtol, atol) = (1.0e-6, 1.0e-8);
    let problem = prothero_robinson(-1.0e5);
    let solve = |interior| {
        Rodas5p::solve(
            &problem,
            (0.0, 2.0),
            &[0.0],
            &dense_opts(rtol, atol, interior),
        )
        .unwrap()
    };
    let wf = solve(InteriorError::WFilteredEnforce);
    let off = solve(InteriorError::Off);
    let (wf_worst, off_worst) = (
        worst_interior(&wf, rtol, atol),
        worst_interior(&off, rtol, atol),
    );
    eprintln!(
        "PR lambda -1e5 rtol 1e-6: interior max WFilteredEnforce {wf_worst:.4} ({} steps), \
         Off {off_worst:.4} ({} steps)",
        wf.stats.accepted_steps, off.stats.accepted_steps
    );
    assert!(wf_worst <= 2.0, "{wf_worst}");
    assert!(off_worst > 10.0, "{off_worst}");
}

#[test]
fn at_without_dense_output_returns_stored_samples_only() {
    let o = opts(1.0e-6, 1.0e-10);
    let sol = Rodas5p::solve(&robertson(), ROBERTSON_SPAN, &ROBERTSON_Y0, &o).unwrap();
    assert_eq!(sol.t, vec![0.0, 40.0]);
    assert_eq!(bits(&sol.at(0.0).unwrap()), bits(&ROBERTSON_Y0));
    assert_eq!(bits(&sol.at(40.0).unwrap()), bits(&sol.y[1]));
    assert_eq!(sol.at(20.0), None);
}

// ------------------------------------------------------------ mass matrix

const MASS: [f64; 4] = [2.0, 1.0, 1.0, 3.0];

/// `M y' = A y` with `A = diag(-1, -50)`.
fn mass_problem() -> Problem {
    Problem::new(2, |_t, y, dy| {
        dy[0] = -y[0];
        dy[1] = -50.0 * y[1];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, jac| {
        jac.copy_from_slice(&[-1.0, 0.0, 0.0, -50.0]);
        Ok(())
    }))
    .mass(MASS.to_vec())
    .autonomous()
}

fn mass_problem_direct() -> OdeProblem {
    OdeProblem::new(
        "mass-linear",
        2,
        Arc::new(|_t, y, out| {
            out[0] = -y[0];
            out[1] = -50.0 * y[1];
            Ok(())
        }),
        None,
        Some(
            Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::new(2, 2, vec![-1.0, 0.0, 0.0, -50.0]))
                as JacobianFn,
        ),
        None,
        None,
        true,
        Some(DenseMatrix::new(2, 2, MASS.to_vec()).unwrap()),
        None,
    )
    .unwrap()
}

#[test]
fn mass_matrix_route_is_bit_identical_to_the_direct_dense_driver() {
    let o = opts(1.0e-8, 1.0e-12);
    let span = (0.0, 2.0);
    let y0 = [1.0, 1.0];
    let sol = Rodas5p::solve(&mass_problem(), span, &y0, &o).unwrap();
    assert!(sol.is_success(), "{:?}", sol.status);
    let (named, _) = integrate_adaptive_dense_observed_with_dense_error_control(
        &mass_problem_direct(),
        span,
        &y0,
        IntegrationMethod::Sequential,
        None,
        None,
        &adaptive_for(&o),
        &span_plan(span.0, span.1),
        DenseErrorControl::Off,
    )
    .unwrap();
    assert!(named.observed.success);
    assert_eq!(bits(&sol.t), bits(&named.observed.t));
    for (a, b) in sol.y.iter().zip(&named.observed.y) {
        assert_eq!(bits(a), bits(b));
    }
    assert_eq!(sol.stats.counters, named.observed.counters);
    assert_eq!(
        sol.stats.route.driver_id,
        RODAS5P_SEQUENTIAL_DENSE_DRIVER_ID
    );
    // Without dense output the mass route controls the endpoint only.
    let (control, sampling, samples, _) = sequential_work(&sol);
    assert_eq!(
        (control, sampling, samples),
        (DenseErrorControl::Off, InteriorSampling::OutputTimes, 0)
    );
    match &sol.stats.driver_work {
        DriverWork::SequentialDense { linear_solver, .. } => {
            assert_eq!(*linear_solver, LinearSolverConfig::default())
        }
        other => panic!("{other:?}"),
    }

    // Against the same system with M folded in: y' = M^{-1} A y.
    let inv = {
        let det = MASS[0] * MASS[3] - MASS[1] * MASS[2];
        [MASS[3] / det, -MASS[1] / det, -MASS[2] / det, MASS[0] / det]
    };
    let folded = Problem::new(2, move |_t, y, dy| {
        let f = [-y[0], -50.0 * y[1]];
        dy[0] = inv[0] * f[0] + inv[1] * f[1];
        dy[1] = inv[2] * f[0] + inv[3] * f[1];
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_t, _y, jac| {
        jac.copy_from_slice(&[-inv[0], -50.0 * inv[1], -inv[2], -50.0 * inv[3]]);
        Ok(())
    }))
    .autonomous();
    let reference = Rodas5p::solve(&folded, span, &y0, &opts(1.0e-12, 1.0e-16)).unwrap();
    let (a, b) = (sol.final_state().unwrap(), reference.final_state().unwrap());
    for (x, r) in a.iter().zip(b) {
        // Within 10 tolerance units of the folded system's tight solution.
        assert!(
            (x - r).abs() <= 10.0 * (1.0e-12 + 1.0e-8 * r.abs()),
            "{a:?} {b:?}"
        );
    }
}

/// Robertson as an index-1 DAE: the third equation is `0 = y1 + y2 + y3 - 1`.
fn robertson_dae(jacobian: Jacobian) -> Problem {
    Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = y[0] + y[1] + y[2] - 1.0;
        Ok(())
    })
    .jacobian(jacobian)
    .mass(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0])
    .autonomous()
}

fn robertson_dae_jac(_t: f64, y: &[f64], jac: &mut [f64]) -> Result<(), CallbackError> {
    jac.copy_from_slice(&[
        -0.04,
        1.0e4 * y[2],
        1.0e4 * y[1],
        0.04,
        -1.0e4 * y[2] - 6.0e7 * y[1],
        -1.0e4 * y[1],
        1.0,
        1.0,
        1.0,
    ]);
    Ok(())
}

#[test]
fn robertson_as_an_index1_dae_with_a_singular_mass_matrix() {
    let (rtol, atol) = (1.0e-6, 1.0e-10);
    let ode = Rodas5p::solve(
        &robertson(),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &opts(1e-10, 1e-14),
    )
    .unwrap();
    let reference = ode.final_state().unwrap();
    for (name, jacobian) in [
        ("analytic", Jacobian::dense(robertson_dae_jac)),
        ("finite-difference", Jacobian::FiniteDifference),
    ] {
        for dense in [false, true] {
            let o = Options {
                dense_output: dense,
                ..opts(rtol, atol)
            };
            let sol = Rodas5p::solve(
                &robertson_dae(jacobian.clone()),
                ROBERTSON_SPAN,
                &ROBERTSON_Y0,
                &o,
            )
            .unwrap();
            assert!(sol.is_success(), "{name}: {:?}", sol.status);
            let y = sol.final_state().unwrap();
            // The algebraic constraint holds to roundoff at every step.
            for state in &sol.y {
                assert!(
                    (state.iter().sum::<f64>() - 1.0).abs() <= 1.0e-12,
                    "{state:?}"
                );
            }
            let err = y
                .iter()
                .zip(reference)
                .map(|(a, b)| (a - b).abs() / (atol + rtol * b.abs()))
                .fold(0.0_f64, f64::max);
            eprintln!(
                "Robertson DAE ({name}, dense {dense}): scaled error vs ODE reference {err:.4}, \
                 {} accepted, {} FD RHS calls",
                sol.stats.accepted_steps, sol.stats.finite_difference.rhs_calls
            );
            assert!(err <= 10.0, "{name}: {err}");
            if name == "finite-difference" {
                assert!(sol.stats.finite_difference.rhs_calls > 0);
            }
        }
    }
}

#[test]
fn mass_and_dense_output_refusals() {
    let unsupported = |problem: &Problem, o: &Options| match Rodas5p::solve(
        problem,
        (0.0, 1.0),
        &[1.0, 1.0],
        o,
    ) {
        Err(SolveError::Unsupported(_)) => {}
        other => panic!("expected Unsupported, got {other:?}"),
    };
    let invalid = |problem: &Problem, o: &Options| match Rodas5p::solve(
        problem,
        (0.0, 1.0),
        &[1.0, 1.0],
        o,
    ) {
        Err(SolveError::InvalidInput(_)) => {}
        other => panic!("expected InvalidInput, got {other:?}"),
    };
    let jvp = Problem::new(2, |_t, y, dy| {
        dy.copy_from_slice(y);
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_t, _y, v, out| {
        out.copy_from_slice(v);
        Ok(())
    }));
    // A mass matrix needs an explicit or finite-difference Jacobian.
    unsupported(&jvp.clone().mass(MASS.to_vec()), &Options::default());
    // So does dense output (the W-filtered estimate needs the stage matrix).
    for interior in [
        InteriorError::WFilteredEnforce,
        InteriorError::Enforce,
        InteriorError::Off,
    ] {
        unsupported(
            &jvp,
            &Options {
                dense_output: true,
                interior_error: interior,
                ..Options::default()
            },
        );
    }
    // A declared band on a dense Jacobian is verified only by the router;
    // the sequential route would ignore it, so it is refused there.
    let banded_dense = mass_problem().banded(0, 0);
    unsupported(&banded_dense, &Options::default());
    unsupported(
        &banded_dense
            .clone()
            .jacobian(Jacobian::dense(|_t, _y, jac| {
                jac.copy_from_slice(&[-1.0, 0.0, 0.0, -50.0]);
                Ok(())
            })),
        &Options::default(),
    );
    invalid(&mass_problem().mass(vec![1.0; 3]), &Options::default());
    invalid(
        &mass_problem().mass(vec![1.0, f64::NAN, 0.0, 1.0]),
        &Options::default(),
    );
}

// ------------------------------------------------- partial solutions

/// Facade `y' = -y` with a fatal RHS beyond `t = 0.5`.
fn fatal_rhs_decay() -> Problem {
    Problem::new(1, |t, y, dy| {
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

/// The direct twin of the facade's guard: the first call beyond 0.5 latches
/// every later call to an error.
fn latched(latch: &Arc<AtomicBool>, t: f64) -> CoreResult<()> {
    if t > 0.5 {
        latch.store(true, Ordering::Relaxed);
    }
    if latch.load(Ordering::Relaxed) {
        Err(CoreError::NonFinite("latched".into()))
    } else {
        Ok(())
    }
}

/// Every other callback answers with an error once the latch is set, as the
/// guard short-circuits every callback after the first fatal error.
fn after_latch(latch: &AtomicBool) -> CoreResult<()> {
    if latch.load(Ordering::Relaxed) {
        Err(CoreError::NonFinite("latched".into()))
    } else {
        Ok(())
    }
}

/// `y' = -y`; the RHS (`rhs_latch`) or the JVP sets the latch beyond 0.5.
fn latched_decay_direct(rhs_latch: bool, jvp: bool) -> OdeProblem {
    let latch = Arc::new(AtomicBool::new(false));
    let (l1, l2, l3) = (latch.clone(), latch.clone(), latch);
    OdeProblem::new(
        "latched-decay",
        1,
        Arc::new(move |t, y, out| {
            if rhs_latch {
                latched(&l1, t)?;
            } else {
                after_latch(&l1)?;
            }
            out[0] = -y[0];
            Ok(())
        }),
        None,
        (!jvp).then(|| {
            Arc::new(move |_t: f64, _y: &[f64]| {
                after_latch(&l3)?;
                DenseMatrix::new(1, 1, vec![-1.0])
            }) as JacobianFn
        }),
        jvp.then(|| {
            Arc::new(move |t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
                if rhs_latch {
                    after_latch(&l2)?;
                } else {
                    latched(&l2, t)?;
                }
                out[0] = -v[0];
                Ok(())
            }) as JvpFn
        }),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn assert_callback_failed(sol: &Solution, expected: Callback) {
    match &sol.status {
        Status::Failed(FailureReason::CallbackFailed { callback, t, .. }) => {
            assert_eq!(*callback, expected);
            assert!(*t > 0.5, "t = {t}");
        }
        other => panic!("unexpected status {other:?}"),
    }
}

#[test]
fn a_fatal_rhs_after_several_steps_returns_the_last_accepted_state() {
    let sol = Rodas5p::solve(&fatal_rhs_decay(), (0.0, 1.0), &[1.0], &Options::default()).unwrap();
    assert_callback_failed(&sol, Callback::Rhs);
    assert!(
        sol.stats.accepted_steps >= 3,
        "{}",
        sol.stats.accepted_steps
    );

    let (direct, partial) = integrate_rodas5p_fast_observed_with_partial(
        &latched_decay_direct(true, false),
        (0.0, 1.0),
        &[1.0],
        &adaptive_for(&Options::default()),
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
    )
    .unwrap();
    let partial = partial.unwrap();
    assert_eq!(sol.t.len(), 2);
    assert_eq!(bits(&sol.t), bits(&[0.0, partial.t]));
    assert_eq!(bits(&sol.y[1]), bits(&partial.y));
    assert_eq!(sol.stats.driver_counters, direct.observed.counters);
    assert_eq!(sol.stats.accepted_steps, direct.accepted_steps);
    let stop = sol.stats.stop.as_ref().unwrap();
    assert_eq!(stop.t.to_bits(), partial.t.to_bits());
    assert_eq!(stop.reason, DriverStopReason::MinimumStep);
    assert_eq!(stop.last_failure, Some(AdaptiveFailureKind::NonFinite));
    assert!(partial.t > 0.0 && partial.t <= 0.5);
    // ensure_success carries the same partial solution.
    match sol.clone().ensure_success().unwrap_err() {
        SolveError::Failed { partial: p, .. } => assert_eq!(bits(&p.t), bits(&sol.t)),
        other => panic!("{other:?}"),
    }
    assert_eq!(sol.at(partial.t).map(|y| bits(&y)), Some(bits(&partial.y)));
}

#[test]
fn a_fatal_rhs_on_the_banded_route_returns_the_last_accepted_state() {
    let n = 6;
    let rhs = move |t: f64, y: &[f64], dy: &mut [f64]| -> Result<(), CallbackError> {
        if t > 0.5 {
            return Err(CallbackError::Fatal("beyond 0.5".into()));
        }
        for i in 0..n {
            let l = if i > 0 { y[i - 1] } else { 0.0 };
            let r = if i + 1 < n { y[i + 1] } else { 0.0 };
            dy[i] = l - 2.0 * y[i] + r;
        }
        Ok(())
    };
    let band = move |_t: f64, _y: &[f64], band: &mut [f64]| -> Result<(), CallbackError> {
        for i in 0..n {
            if i > 0 {
                band[i * 3] = 1.0;
            }
            band[i * 3 + 1] = -2.0;
            if i + 1 < n {
                band[i * 3 + 2] = 1.0;
            }
        }
        Ok(())
    };
    let y0 = vec![1.0; n];
    let sol = Rodas5p::solve(
        &Problem::new(n, rhs)
            .jacobian(Jacobian::banded(1, 1, band))
            .autonomous(),
        (0.0, 1.0),
        &y0,
        &Options::default(),
    )
    .unwrap();
    assert_callback_failed(&sol, Callback::Rhs);

    let latch = Arc::new(AtomicBool::new(false));
    let fill_latch = latch.clone();
    let fill: BandFillFn = Arc::new(move |t, y, out| {
        after_latch(&fill_latch)?;
        band(t, y, out).map_err(|e| CoreError::NonFinite(e.to_string()))
    });
    let f2 = fill.clone();
    let direct_problem = OdeProblem::new(
        "latched-tridiagonal",
        n,
        Arc::new(move |t, y, out| {
            latched(&latch, t)?;
            rhs(t, y, out).map_err(|e| CoreError::NonFinite(e.to_string()))
        }),
        None,
        Some(Arc::new(move |t: f64, y: &[f64]| {
            let mut values = vec![0.0; n * 3];
            f2(t, y, &mut values)?;
            let mut m = DenseMatrix::zeros(n, n);
            for i in 0..n {
                for j in i.saturating_sub(1)..=(i + 1).min(n - 1) {
                    m[(i, j)] = values[i * 3 + (j + 1 - i)];
                }
            }
            Ok(m)
        }) as JacobianFn),
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
        (0.0, 1.0),
        &y0,
        &adaptive_for(&Options::default()),
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
    )
    .unwrap();
    let partial = direct.partial.unwrap();
    assert_eq!(bits(&sol.t), bits(&[0.0, partial.t]));
    assert_eq!(bits(&sol.y[1]), bits(&partial.y));
    assert_eq!(sol.stats.driver_counters, direct.fast.observed.counters);
}

#[test]
fn a_fatal_jvp_inside_gmres_returns_the_partial_solution() {
    let problem = Problem::new(1, |_t, y, dy| {
        dy[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|t, _y, v, out| {
        if t > 0.5 {
            return Err(CallbackError::Fatal("operator unavailable".into()));
        }
        out[0] = -v[0];
        Ok(())
    }))
    .autonomous();
    let sol = Rodas5p::solve(&problem, (0.0, 1.0), &[1.0], &Options::default()).unwrap();
    assert_callback_failed(&sol, Callback::Jvp);
    assert_eq!(sol.stats.callbacks.fatal_errors, 1);
    assert!(sol.stats.counters.jvp_calls > 0);

    let direct = integrate_rodas5p_mf_fast_observed_gmres_into(
        &latched_decay_direct(false, true),
        (0.0, 1.0),
        &[1.0],
        &RoutingOptions::default().matrix_free,
        &adaptive_for(&Options::default()),
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
    )
    .unwrap();
    let partial = direct.partial.unwrap();
    eprintln!(
        "fatal JVP: last accepted t = {}, {} accepted, last rejection {:?}",
        partial.t, direct.accepted_steps, partial.last_failure
    );
    // The JVP is evaluated at the step's start: the first step that starts
    // beyond 0.5 fails, so the last accepted time is the first one past 0.5.
    assert!(partial.t > 0.5 && partial.t < 1.0, "{}", partial.t);
    assert_eq!(bits(&sol.t), bits(&[0.0, partial.t]));
    assert_eq!(bits(&sol.y[1]), bits(&partial.y));
    assert_eq!(sol.stats.driver_counters, direct.observed.counters);
    assert_eq!(
        sol.stats.stop.as_ref().unwrap().last_failure,
        partial.last_failure
    );
}

#[test]
fn a_fatal_rhs_on_the_dense_output_route_keeps_every_accepted_step() {
    let o = Options {
        dense_output: true,
        ..Options::default()
    };
    let failed = Rodas5p::solve(&fatal_rhs_decay(), (0.0, 1.0), &[1.0], &o).unwrap();
    assert_callback_failed(&failed, Callback::Rhs);
    let clean = Rodas5p::solve(
        &Problem::new(1, |_t, y, dy| {
            dy[0] = -y[0];
            Ok(())
        })
        .jacobian(Jacobian::dense(|_t, _y, jac| {
            jac[0] = -1.0;
            Ok(())
        }))
        .autonomous(),
        (0.0, 1.0),
        &[1.0],
        &o,
    )
    .unwrap();
    assert!(clean.is_success());
    // Every accepted step before the failure is the clean run's step.
    let k = failed.t.len();
    assert!(k >= 3 && k < clean.t.len(), "{k}");
    assert_eq!(bits(&failed.t), bits(&clean.t[..k]));
    for (a, b) in failed.y.iter().zip(&clean.y) {
        assert_eq!(bits(a), bits(b));
    }
    let last = *failed.t.last().unwrap();
    assert!(last <= 0.5);
    assert_eq!(
        failed.stats.stop.as_ref().unwrap().t.to_bits(),
        last.to_bits()
    );
    // The extension of the partial solution covers [t0, last] only.
    assert!(failed.at(0.5 * last).is_some());
    assert_eq!(failed.at(last + 1e-3), None);
}

// ------------------------------------------------- failure reasons

#[test]
fn failure_reasons_name_the_drivers_failure_kind() {
    // Non-finite: a recoverable RHS error at every call beyond 0.5.
    let nonfinite = Problem::new(1, |t, y, dy| {
        if t > 0.5 {
            return Err(CallbackError::Recoverable("outside the domain".into()));
        }
        dy[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, jac| {
        jac[0] = -1.0;
        Ok(())
    }))
    .autonomous();
    let defaults = AdaptiveStepConfig::default();
    let sol = Rodas5p::solve(&nonfinite, (0.0, 1.0), &[1.0], &Options::default()).unwrap();
    assert_eq!(
        sol.status,
        Status::Failed(FailureReason::NonFinite {
            min_step: defaults.min_step
        })
    );
    assert!(sol.t.len() == 2 && sol.t[1] > 0.0 && sol.t[1] <= 0.5);

    // Linear solve: M = diag(1, 0) with a Jacobian whose algebraic row
    // vanishes beyond t = 0.5, so W = M - h gamma J is singular for every h.
    let singular = Problem::new(2, |_t, y, dy| {
        dy[0] = -y[0];
        dy[1] = y[0] - y[1];
        Ok(())
    })
    .jacobian(Jacobian::dense(|t, _y, jac| {
        jac[0] = -1.0;
        if t <= 0.5 {
            jac[2] = 1.0;
            jac[3] = -1.0;
        }
        Ok(())
    }))
    .mass(vec![1.0, 0.0, 0.0, 0.0])
    .autonomous();
    let sol = Rodas5p::solve(&singular, (0.0, 1.0), &[1.0, 1.0], &Options::default()).unwrap();
    assert_eq!(
        sol.status,
        Status::Failed(FailureReason::LinearSolveFailed {
            min_step: defaults.min_step
        })
    );
    assert_eq!(
        sol.stats.stop.as_ref().unwrap().last_failure,
        Some(AdaptiveFailureKind::LinearSolve)
    );
    // The Jacobian is evaluated at the step's start.
    assert!(sol.t[1] > 0.5 && sol.t[1] < 1.0, "{:?}", sol.t);

    // Attempt budget on the dense-output route.
    let sol = Rodas5p::solve(
        &robertson(),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            max_steps: 5,
            dense_output: true,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(
        sol.status,
        Status::Failed(FailureReason::MaxStepsReached { max_steps: 5 })
    );
    assert_eq!(
        sol.stats.stop.as_ref().unwrap().reason,
        DriverStopReason::AttemptBudget
    );
    assert_eq!(sol.t.len(), sol.stats.accepted_steps + 1);

    // Step size: y' = y^2 blows up at t = 1. The controller shrinks the
    // step after the accepted steps near the pole until it falls below the
    // minimum step (measured: no rejection after the last accepted step,
    // last accepted t = 1.0000007, past the pole of the exact solution).
    let blowup = Problem::new(1, |_t, y, dy| {
        dy[0] = y[0] * y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, y, jac| {
        jac[0] = 2.0 * y[0];
        Ok(())
    }))
    .autonomous();
    let sol = Rodas5p::solve(&blowup, (0.0, 2.0), &[1.0], &Options::default()).unwrap();
    eprintln!(
        "y' = y^2: {:?}, stop {:?}, last state {:e}",
        sol.status,
        sol.stats.stop,
        sol.final_state().unwrap()[0]
    );
    assert_eq!(
        sol.status,
        Status::Failed(FailureReason::StepSizeTooSmall {
            min_step: defaults.min_step
        })
    );
    let stop = sol.stats.stop.as_ref().unwrap();
    assert_eq!(stop.reason, DriverStopReason::MinimumStep);
    assert!(stop.t > 0.99 && stop.t < 2.0, "{}", stop.t);
    assert!(sol.final_state().unwrap()[0] > 1.0e6);
}

#[test]
fn options_without_interior_error_deserialize_to_the_default() {
    let mut value = serde_json::to_value(Options::default()).unwrap();
    value.as_object_mut().unwrap().remove("interior_error");
    let back: Options = serde_json::from_value(value).unwrap();
    assert_eq!(back, Options::default());
    assert_eq!(back.interior_error, InteriorError::WFilteredEnforce);
    let record = serde_json::to_value(
        Rodas5p::solve(
            &mass_problem(),
            (0.0, 1.0),
            &[1.0, 1.0],
            &Options::default(),
        )
        .unwrap()
        .stats,
    )
    .unwrap();
    assert_eq!(record["driver_work"]["driver"], "sequential-dense");
    assert_eq!(record["driver_work"]["control"], "off");
    assert_eq!(record["options"]["interior_error"], "w-filtered-enforce");
}
