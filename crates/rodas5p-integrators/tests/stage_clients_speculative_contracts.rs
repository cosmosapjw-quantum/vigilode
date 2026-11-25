//! Contracts of the speculative Jacobian client (loop node V07, stage a,
//! `src/stage_clients/speculative.rs`).
//!
//! (i) Mode `Exact` on a problem behind the recording wrapper reproduces
//! the protected dense driver on the plain problem bit for bit: step
//! statistics, every counter, every output and every linearization point
//! (in-place LU and faer; autonomous and non-autonomous, with an analytic
//! and with a finite-difference time derivative).
//!
//! (ii) The recording wrapper is result-neutral: call by call, and for a
//! whole run of the protected driver.
//!
//! (iii) Which callback call belongs to which stage: for every accepted
//! step the test re-computes the step in the driver's transformed form
//! with the same kernels. The re-computed new state is the driver's
//! accepted state bit for bit, and the k-th recorded call after `factor`
//! is the argument of stage k + 1 bit for bit, with time `t + c_{k+1} h`.
//! An independent re-computation in the untransformed k-form agrees to
//! rounding.
//!
//! (iv) Each mode evaluates the Jacobian where the module documentation
//! says: the first step exactly, then at g8, g7 or the previous state; a
//! local-error rejection keeps the linearization; an attempt that failed
//! with an error is retried with the exact Jacobian.
//!
//! (v) The time derivative is evaluated by the driver at accepted states
//! only.
//!
//! (vi) The models of the measurement are what they claim to be: analytic
//! Jacobians against finite differences, closed forms against their
//! equations, and bitwise agreement with the definitions already in the
//! repository.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rodas5p_core::{CoreError, DenseMatrix, WorkCounters, rodas5p_coefficients};
use rodas5p_integrators::stage_clients::speculative::{
    SPECULATIVE_JACOBIAN_STAGE_A_ID, SpeculationMode, SpeculationStats, SpeculativeJacobianClient,
    StageArgumentRecorder, StageRhs,
};
use rodas5p_integrators::stage_clients::{DenseLuMode, DenseLuStageClient};
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, AdaptiveStepConfig, DriverPartial, JacobianIntoFn, OdeProblem,
    OutputSchedule, RODAS5P_FAST_CLIENT_DRIVER_ID, RODAS5P_FAST_DRIVER_ID, Rodas5pFastLu,
    Rodas5pFastResult, ScientificCorpusV2, ScientificFamily, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol_and_control,
    integrate_rodas5p_fast_observed_with_atol, robertson_problem,
};

#[path = "stage_clients_speculative_problems.rs"]
mod problems;
use problems::{
    AtolRule, Model, ORDER_STEPS, RTOLS, adaptive_models, brusselator, hires, kaps, logistic_pair,
    order_models, oregonator, pollution, pr_linear, pr_nonlinear, probe_state, robertson,
};

#[path = "b1_problems/mod.rs"]
mod b1_problems;

/// RODAS5P stages.
const STAGES: usize = 8;

/// One point: the bits of a time and of a state.
type Point = (u64, Vec<u64>);
type Trace = Arc<Mutex<Vec<Point>>>;

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

fn point(t: f64, y: &[f64]) -> Point {
    (t.to_bits(), bits(y))
}

fn take(trace: &Trace) -> Vec<Point> {
    std::mem::take(&mut *trace.lock().unwrap())
}

/// `model` with every Jacobian evaluation recorded in the returned trace.
/// Both drivers build the Jacobian once per linearization.
fn traced(model: &Model) -> (Model, Trace) {
    let trace = Trace::default();
    let inner = Arc::clone(&model.fill);
    let sink = Arc::clone(&trace);
    let mut traced = model.clone();
    traced.fill = Arc::new(move |t: f64, y: &[f64], j: &mut DenseMatrix| {
        sink.lock().unwrap().push(point(t, y));
        inner(t, y, j)
    });
    (traced, trace)
}

/// Eleven output times over the span, the last one the span end.
fn schedule(span: (f64, f64)) -> OutputSchedule {
    let points = 10;
    let times = (0..=points)
        .map(|k| {
            if k == points {
                span.1
            } else {
                span.0 + (span.1 - span.0) * k as f64 / points as f64
            }
        })
        .collect();
    OutputSchedule::new(times).unwrap()
}

type Protected = (Rodas5pFastResult, Option<DriverPartial>);

fn run_protected(
    problem: &OdeProblem,
    model: &Model,
    config: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> Protected {
    let atol = vec![config.atol; model.n];
    integrate_rodas5p_fast_observed_with_atol(
        problem,
        model.t_span,
        &model.y0,
        config,
        output,
        &atol,
    )
    .unwrap()
}

struct ClientRun {
    result: Rodas5pFastResult,
    partial: Option<DriverPartial>,
    stats: SpeculationStats,
}

#[allow(clippy::too_many_arguments)]
fn run_client(
    problem: &OdeProblem,
    recorder: &Arc<StageArgumentRecorder>,
    model: &Model,
    lu: DenseLuMode,
    mode: SpeculationMode,
    config: &AdaptiveStepConfig,
    output: &OutputSchedule,
    control: &mut dyn FnMut(&AcceptedStepView<'_>) -> AcceptControl,
) -> ClientRun {
    let client = SpeculativeJacobianClient::new(
        DenseLuStageClient::new(model.n, lu).unwrap(),
        mode,
        Arc::clone(recorder),
    )
    .unwrap();
    assert_eq!(client.mode(), mode);
    let handle = client.handle();
    let atol = vec![config.atol; model.n];
    let (result, partial, _client) = integrate_rodas5p_fast_client_observed_with_atol_and_control(
        problem,
        Box::new(client),
        model.t_span,
        &model.y0,
        config,
        output,
        &atol,
        control,
    )
    .unwrap();
    ClientRun {
        result,
        partial,
        stats: handle.snapshot(),
    }
}

fn error_failures(counters: &WorkCounters) -> u64 {
    counters.linear_solve_failures
        + counters.nonlinear_solve_failures
        + counters.nonfinite_step_failures
}

/// The step statistics, counters and output of the client run are those of
/// the protected run, bit for bit.
fn assert_same_result(label: &str, lu: DenseLuMode, protected: &Protected, client: &ClientRun) {
    let (reference, reference_partial) = protected;
    let result = &client.result;
    assert!(reference.observed.success, "{label}");
    assert!(result.observed.success, "{label}");
    assert_eq!(reference_partial, &None, "{label}");
    assert_eq!(client.partial, None, "{label}");
    assert_eq!(reference.lu, lu.protected_lu(), "{label}");
    assert_eq!(reference.driver, RODAS5P_FAST_DRIVER_ID, "{label}");
    assert_eq!(result.lu, Rodas5pFastLu::Client, "{label}");
    assert_eq!(result.driver, RODAS5P_FAST_CLIENT_DRIVER_ID, "{label}");
    assert_eq!(result.attempts, reference.attempts, "{label}");
    assert_eq!(result.accepted_steps, reference.accepted_steps, "{label}");
    assert_eq!(result.rejected_steps, reference.rejected_steps, "{label}");
    assert_eq!(result.jacobian_reuses, reference.jacobian_reuses, "{label}");
    assert_eq!(
        result.observed.internal_steps, reference.observed.internal_steps,
        "{label}"
    );
    assert_eq!(
        result.observed.output_clipped_steps, reference.observed.output_clipped_steps,
        "{label}"
    );
    assert_eq!(
        result.observed.counters, reference.observed.counters,
        "{label}"
    );
    assert_eq!(
        bits(&result.observed.t),
        bits(&reference.observed.t),
        "{label}"
    );
    assert_eq!(
        result.observed.y.len(),
        reference.observed.y.len(),
        "{label}"
    );
    for (index, (a, b)) in result
        .observed
        .y
        .iter()
        .zip(&reference.observed.y)
        .enumerate()
    {
        assert_eq!(bits(a), bits(b), "{label}: output state {index}");
    }
}

/// The 1D Brusselator with a mean-field coupling of
/// `stage_clients_dense_identity.rs`: a half-full Jacobian, for which the
/// protected policy picks faer above order 64.
fn meanfield_brusselator(cells: usize, kappa: f64) -> Model {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let mean = kappa / cells as f64;
    let mut model = brusselator(cells);
    model.id = format!("brusselator-1d-{cells}-meanfield");
    model.rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (mut ubar, mut vbar) = (0.0, 0.0);
        for i in 0..cells {
            ubar += y[2 * i];
            vbar += y[2 * i + 1];
        }
        ubar /= cells as f64;
        vbar /= cells as f64;
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
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
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur) + kappa * (ubar - u);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr) + kappa * (vbar - v);
        }
        Ok(())
    });
    model.fill = Arc::new(move |_t: f64, y: &[f64], j: &mut DenseMatrix| {
        for i in 0..cells {
            for k in 0..cells {
                j[(2 * i, 2 * k)] = mean;
                j[(2 * i + 1, 2 * k + 1)] = mean;
            }
        }
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            j[(a, a)] = mean + 2.0 * u * v - 4.0 - 2.0 * c - kappa;
            j[(a, b)] = u * u;
            j[(b, a)] = 3.0 - 2.0 * u * v;
            j[(b, b)] = mean - u * u - 2.0 * c - kappa;
            if i > 0 {
                j[(a, a - 2)] = mean + c;
                j[(b, b - 2)] = mean + c;
            }
            if i + 1 < cells {
                j[(a, a + 2)] = mean + c;
                j[(b, b + 2)] = mean + c;
            }
        }
    });
    model
}

/// The nonlinear Prothero-Robinson problem without its analytic time
/// derivative: the driver differences the right-hand side in `t`.
fn pr_nonlinear_fd_time_derivative() -> Model {
    let mut model = pr_nonlinear(10.0);
    model.id = "pr-nonlinear (finite-difference f_t)".into();
    model.partial_t = None;
    model
}

/// Rejections and Jacobian reuses over the tolerances of one model.
#[derive(Default)]
struct Coverage {
    rejected: usize,
    reuses: usize,
}

/// (i) for one model at every tolerance.
fn assert_exact_reproduces_protected(model: &Model, expected: DenseLuMode) -> Coverage {
    let lu = model.lu_mode();
    assert_eq!(lu, expected, "{}: the protected policy's LU", model.id);
    let (traced_model, trace) = traced(model);
    let plain = traced_model.problem();
    let output = schedule(model.t_span);
    let mut coverage = Coverage::default();
    for rtol in RTOLS {
        let label = format!("{} at rtol {rtol:e}, {lu:?}", model.id);
        let config = model.adaptive_config(rtol);
        let protected = run_protected(&plain, model, &config, &output);
        let protected_trace = take(&trace);

        let (recorded, recorder) = traced_model.recorded_problem();
        let mut views: Vec<Point> = Vec::new();
        let mut control = |view: &AcceptedStepView<'_>| {
            views.push(point(view.t, view.y));
            AcceptControl::Continue
        };
        let client = run_client(
            &recorded,
            &recorder,
            model,
            lu,
            SpeculationMode::Exact,
            &config,
            &output,
            &mut control,
        );
        let client_trace = take(&trace);

        assert_same_result(&label, lu, &protected, &client);
        assert_eq!(
            protected_trace, client_trace,
            "{label}: linearization points"
        );
        let reference = &protected.0;
        let counters = &reference.observed.counters;
        assert_eq!(error_failures(counters), 0, "{label}");
        let accepted = reference.accepted_steps;
        // One linearization per accepted state: t_0 .. t_{N-1}, then the
        // accept control's view of every step.
        assert_eq!(protected_trace.len(), accepted, "{label}");
        assert_eq!(protected_trace[0], point(model.t_span.0, &model.y0));
        assert_eq!(views.len(), accepted, "{label}");
        assert_eq!(protected_trace[1..], views[..accepted - 1], "{label}");
        assert_eq!(
            client.stats,
            SpeculationStats {
                linearizations: accepted as u64,
                exact_by_mode: accepted as u64,
                attempts: reference.attempts as u64,
                ..SpeculationStats::default()
            },
            "{label}"
        );
        // The recorder saw every right-hand-side call of the run.
        assert_eq!(recorder.total_calls(), counters.rhs_calls, "{label}");
        assert_eq!(recorder.attempts_begun(), reference.attempts as u64);
        println!(
            "SPECJ_IDENTITY_JSON {}",
            serde_json::json!({
                "model": model.id,
                "n": model.n,
                "rtol": rtol,
                "atol": config.atol,
                "autonomous": model.autonomous,
                "analytic_time_derivative": model.partial_t.is_some(),
                "inner_lu": format!("{lu:?}"),
                "attempts": reference.attempts,
                "accepted_steps": accepted,
                "rejected_steps": reference.rejected_steps,
                "jacobian_reuses": reference.jacobian_reuses,
                "jacobian_builds": counters.jacobian_builds,
                "rhs_calls": counters.rhs_calls,
                "ft_calls": counters.ft_calls,
                "output_states_compared": reference.observed.y.len(),
                "linearization_points_compared": protected_trace.len(),
                "bit_identical": true,
            })
        );
        coverage.rejected += reference.rejected_steps;
        coverage.reuses += reference.jacobian_reuses;
    }
    coverage
}

#[test]
fn exact_mode_reproduces_protected_driver_robertson() {
    assert_exact_reproduces_protected(&robertson(), DenseLuMode::InPlace);
}

#[test]
fn exact_mode_reproduces_protected_driver_hires() {
    assert_exact_reproduces_protected(&hires(), DenseLuMode::InPlace);
}

#[test]
fn exact_mode_reproduces_protected_driver_brusselator_n32() {
    assert_exact_reproduces_protected(&brusselator(32), DenseLuMode::InPlace);
}

#[test]
fn exact_mode_reproduces_protected_driver_brusselator_n100() {
    let coverage = assert_exact_reproduces_protected(&brusselator(100), DenseLuMode::InPlace);
    // The rejection path and the Jacobian reuse after it were exercised.
    assert!(coverage.rejected > 0 && coverage.reuses > 0);
}

#[test]
fn exact_mode_reproduces_protected_driver_meanfield_brusselator_n100_faer() {
    let model = meanfield_brusselator(100, 0.1);
    assert_eq!(model.n, 200);
    let coverage = assert_exact_reproduces_protected(&model, DenseLuMode::Faer);
    assert!(coverage.rejected > 0 && coverage.reuses > 0);
}

#[test]
fn exact_mode_reproduces_protected_driver_nonautonomous() {
    let analytic = pr_nonlinear(10.0);
    assert!(!analytic.autonomous && analytic.partial_t.is_some());
    assert_exact_reproduces_protected(&analytic, DenseLuMode::InPlace);
    let differenced = pr_nonlinear_fd_time_derivative();
    assert!(!differenced.autonomous && differenced.partial_t.is_none());
    assert_exact_reproduces_protected(&differenced, DenseLuMode::InPlace);
}

/// (ii) call by call: the wrapper returns what the callback returns, for
/// every model, before and after an attempt has begun, and it passes an
/// error and a non-finite output through.
#[test]
fn recording_wrapper_is_result_neutral_call_by_call() {
    let mut models = adaptive_models();
    models.extend(order_models());
    for model in &models {
        let recorder = StageArgumentRecorder::new(model.n).unwrap();
        let wrapped = recorder.wrap_rhs(Arc::clone(&model.rhs));
        for (call, armed) in [false, true, true].into_iter().enumerate() {
            if armed {
                recorder.begin_attempt();
            }
            let y = probe_state(model, call);
            let t = model.t_span.0 + 0.37 * call as f64;
            let (mut plain, mut through) = (vec![0.5; model.n], vec![0.5; model.n]);
            (model.rhs)(t, &y, &mut plain).unwrap();
            wrapped(t, &y, &mut through).unwrap();
            assert_eq!(bits(&plain), bits(&through), "{} call {call}", model.id);
        }
        assert_eq!(recorder.total_calls(), 3);
    }

    let failing: StageRhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = f64::NAN;
        if y[0] > 1.0 {
            Err(CoreError::NonFinite("refused".into()))
        } else {
            Ok(())
        }
    });
    let recorder = StageArgumentRecorder::new(1).unwrap();
    let wrapped = recorder.wrap_rhs(failing);
    let mut out = [0.0];
    assert!(wrapped(0.0, &[0.5], &mut out).is_ok());
    assert!(out[0].is_nan());
    assert!(matches!(
        wrapped(0.0, &[2.0], &mut out),
        Err(CoreError::NonFinite(message)) if message == "refused"
    ));
}

/// (ii) for a whole run: the protected driver on the problem behind the
/// wrapper (no client, nobody starts an attempt) is the protected run on
/// the plain problem.
#[test]
fn recording_wrapper_is_result_neutral_for_the_protected_driver() {
    for model in [hires(), brusselator(32), pr_nonlinear_fd_time_derivative()] {
        let output = schedule(model.t_span);
        for rtol in RTOLS {
            let label = format!("{} at rtol {rtol:e}", model.id);
            let config = model.adaptive_config(rtol);
            let plain = run_protected(&model.problem(), &model, &config, &output).0;
            let (recorded, recorder) = model.recorded_problem();
            let behind = run_protected(&recorded, &model, &config, &output).0;
            assert!(plain.observed.success && behind.observed.success, "{label}");
            assert_eq!(behind.attempts, plain.attempts, "{label}");
            assert_eq!(behind.observed.counters, plain.observed.counters, "{label}");
            assert_eq!(bits(&behind.observed.t), bits(&plain.observed.t), "{label}");
            for (a, b) in behind.observed.y.iter().zip(&plain.observed.y) {
                assert_eq!(bits(a), bits(b), "{label}");
            }
            assert_eq!(recorder.total_calls(), plain.observed.counters.rhs_calls);
            // Nothing is stored before an attempt begins.
            assert_eq!(recorder.attempts_begun(), 0, "{label}");
            assert_eq!(recorder.calls_in_attempt(), 0, "{label}");
            assert!(!recorder.attempt_is_complete(), "{label}");
        }
    }
}

/// One step of the fast driver from `(t, y)` with step `h`, re-computed
/// with the public kernels in the driver's transformed form and order of
/// operations (`rodas5p_fast.rs`, `Workspace::attempt`).
struct MirrorStep {
    /// The arguments of the stage evaluations 2..=8 and their times.
    stage_times: Vec<f64>,
    stage_states: Vec<Vec<f64>>,
    /// The transformed stage vectors `u_1 .. u_8`.
    u: Vec<Vec<f64>>,
    y_new: Vec<f64>,
}

fn mirror_step(
    problem: &OdeProblem,
    client: &mut DenseLuStageClient,
    t: f64,
    y: &[f64],
    h: f64,
) -> MirrorStep {
    let coefficients = rodas5p_coefficients().unwrap();
    let (n, s) = (y.len(), coefficients.stages());
    assert_eq!(s, STAGES);
    let mut counters = WorkCounters::default();
    client.linearize(problem, t, y, &mut counters).unwrap();
    let mut f0 = vec![0.0; n];
    problem.eval_rhs_into(t, y, &mut f0, &mut counters).unwrap();
    let mut ft = vec![0.0; n];
    if !problem.autonomous {
        problem
            .eval_partial_t_into(t, y, &mut ft, &mut counters)
            .unwrap();
    }
    client
        .factor(1.0 / (h * coefficients.gamma), &mut counters)
        .unwrap();
    let mut u: Vec<Vec<f64>> = Vec::with_capacity(s);
    let (mut stage_times, mut stage_states) = (Vec::new(), Vec::new());
    let mut stage_state = vec![0.0; n];
    let mut stage_rhs = vec![0.0; n];
    for i in 0..s {
        if i == 0 {
            stage_rhs.copy_from_slice(&f0);
        } else {
            stage_state.copy_from_slice(y);
            for (j, uj) in u.iter().enumerate() {
                let aij = coefficients.a[(i, j)];
                if aij != 0.0 {
                    for (x, v) in stage_state.iter_mut().zip(uj) {
                        *x += aij * v;
                    }
                }
            }
            let stage_time = t + coefficients.c[i] * h;
            problem
                .eval_rhs_into(stage_time, &stage_state, &mut stage_rhs, &mut counters)
                .unwrap();
            stage_times.push(stage_time);
            stage_states.push(stage_state.clone());
        }
        for (j, uj) in u.iter().enumerate() {
            let c = coefficients.c_matrix[(i, j)];
            if c != 0.0 {
                let cij = c / h;
                if cij != 0.0 {
                    for (x, v) in stage_rhs.iter_mut().zip(uj) {
                        *x += cij * v;
                    }
                }
            }
        }
        if !problem.autonomous {
            let g = coefficients.gamma_rows[i] * h;
            for (x, v) in stage_rhs.iter_mut().zip(&ft) {
                *x += g * v;
            }
        }
        client
            .solve_in_place(&mut stage_rhs, &mut counters)
            .unwrap();
        u.push(stage_rhs.clone());
    }
    let mut y_new = y.to_vec();
    for (j, uj) in u.iter().enumerate() {
        let bj = coefficients.b_code[j];
        if bj != 0.0 {
            for (x, v) in y_new.iter_mut().zip(uj) {
                *x += bj * v;
            }
        }
    }
    MirrorStep {
        stage_times,
        stage_states,
        u,
        y_new,
    }
}

fn max_abs(v: &[f64]) -> f64 {
    v.iter().fold(0.0_f64, |m, x| m.max(x.abs()))
}

fn max_abs_difference(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .fold(0.0_f64, |m, (x, y)| m.max((x - y).abs()))
}

/// What (iii) found for one model.
struct MappingReport {
    steps: usize,
    steps_after_rejection: usize,
}

/// (iii) for one run in mode `Exact`: after every accepted step, the
/// recorder against the mirror of that step.
fn assert_recorded_calls_are_stage_arguments(model: &Model, rtol: f64) -> MappingReport {
    let coefficients = rodas5p_coefficients().unwrap();
    let lu = model.lu_mode();
    let plain = model.problem();
    let (recorded, recorder) = model.recorded_problem();
    assert_eq!(recorder.stage_evaluations(), STAGES - 1);
    assert_eq!(recorder.stages(), STAGES);
    let config = model.adaptive_config(rtol);
    let output = model.end_points();
    let mut mirror_client = DenseLuStageClient::new(model.n, lu).unwrap();
    let (mut t_previous, mut y_previous) = (model.t_span.0, model.y0.clone());
    let mut attempts_previous = 0;
    let mut report = MappingReport {
        steps: 0,
        steps_after_rejection: 0,
    };
    let mut slot = vec![0.0; model.n];
    let label = format!("{} at rtol {rtol:e}", model.id);
    let mut control = |view: &AcceptedStepView<'_>| {
        let step = view.accepted_steps;
        let mirror = mirror_step(
            &plain,
            &mut mirror_client,
            t_previous,
            &y_previous,
            view.h_used,
        );
        // The mirror is the driver's step: its new state is the accepted
        // state, bit for bit.
        assert_eq!(
            bits(&mirror.y_new),
            bits(view.y),
            "{label}: step {step}, the mirrored step is not the driver's"
        );
        assert_eq!((t_previous + view.h_used).to_bits(), view.t.to_bits());
        // Exactly seven calls since `factor`, and call k is stage k + 2.
        assert_eq!(
            recorder.calls_in_attempt(),
            STAGES - 1,
            "{label}: step {step}"
        );
        assert!(recorder.attempt_is_complete(), "{label}: step {step}");
        for stage in 2..=STAGES {
            let time = recorder
                .completed_stage_argument_into(stage, &mut slot)
                .unwrap();
            assert_eq!(
                time.to_bits(),
                mirror.stage_times[stage - 2].to_bits(),
                "{label}: step {step}, time of stage {stage}"
            );
            assert_eq!(
                time.to_bits(),
                (t_previous + coefficients.c[stage - 1] * view.h_used).to_bits()
            );
            assert_eq!(
                bits(&slot),
                bits(&mirror.stage_states[stage - 2]),
                "{label}: step {step}, argument of stage {stage}"
            );
        }
        // Stages 7 and 8 are evaluated at the accepted time.
        for stage in [STAGES - 1, STAGES] {
            let time = recorder
                .completed_stage_argument_into(stage, &mut slot)
                .unwrap();
            assert_eq!(time.to_bits(), view.t.to_bits(), "{label}: step {step}");
        }
        // Theory S5: rows 6, 7, 8 of `A` and `b_code` differ by unit
        // entries, so g7 = g6 + U6, g8 = g7 + U7 and y_new = g8 + U8 as
        // floating-point sums: g8 is the embedded solution and U8 the
        // error estimate.
        let (g6, g7, g8) = (
            &mirror.stage_states[STAGES - 4],
            &mirror.stage_states[STAGES - 3],
            &mirror.stage_states[STAGES - 2],
        );
        for (lower, upper, increment, name) in [
            (g6, g7, &mirror.u[STAGES - 3], "g7 = g6 + U6"),
            (g7, g8, &mirror.u[STAGES - 2], "g8 = g7 + U7"),
            (g8, &mirror.y_new, &mirror.u[STAGES - 1], "y_new = g8 + U8"),
        ] {
            let sum: Vec<f64> = lower.iter().zip(increment).map(|(a, b)| a + b).collect();
            assert_eq!(bits(upper), bits(&sum), "{label}: step {step}, {name}");
        }
        report.steps += 1;
        if view.attempts - attempts_previous > 1 {
            report.steps_after_rejection += 1;
        }
        attempts_previous = view.attempts;
        t_previous = view.t;
        y_previous.copy_from_slice(view.y);
        AcceptControl::Continue
    };
    let run = run_client(
        &recorded,
        &recorder,
        model,
        lu,
        SpeculationMode::Exact,
        &config,
        &output,
        &mut control,
    );
    assert!(
        run.result.observed.success && run.partial.is_none(),
        "{label}"
    );
    assert_eq!(error_failures(&run.result.observed.counters), 0, "{label}");
    assert_eq!(report.steps, run.result.accepted_steps, "{label}");
    println!(
        "SPECJ_STAGE_MAPPING_JSON {}",
        serde_json::json!({
            "model": model.id,
            "rtol": rtol,
            "autonomous": model.autonomous,
            "analytic_time_derivative": model.partial_t.is_some(),
            "accepted_steps_checked": report.steps,
            "steps_accepted_after_a_rejection": report.steps_after_rejection,
            "rhs_calls_per_fresh_attempt": if model.autonomous || model.partial_t.is_some() {
                8
            } else {
                10
            },
            "recorded_calls_per_attempt": STAGES - 1,
            "call_k_is_stage_k_plus_1_bitwise": true,
            "mirrored_step_equals_accepted_state_bitwise": true,
        })
    );
    report
}

#[test]
fn recorded_calls_are_the_stage_arguments_in_stage_order() {
    for rtol in [1.0e-4, 1.0e-8] {
        assert_recorded_calls_are_stage_arguments(&hires(), rtol);
        assert_recorded_calls_are_stage_arguments(&robertson(), rtol);
        // Non-autonomous: f(t, y) and the time derivative come before
        // `factor`, with one callback or with two more right-hand-side
        // calls; the seven recorded calls are the stages either way.
        assert_recorded_calls_are_stage_arguments(&pr_nonlinear(10.0), rtol);
        assert_recorded_calls_are_stage_arguments(&pr_nonlinear_fd_time_derivative(), rtol);
    }
    // Steps accepted after a local-error rejection: the recorder holds the
    // accepted retry, not the rejected attempt.
    let report = assert_recorded_calls_are_stage_arguments(&brusselator(100), 1.0e-6);
    assert!(report.steps_after_rejection > 0);
}

/// `a x = b` by Gaussian elimination with partial pivoting (test use only).
fn solve_small(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for k in 0..n {
        let pivot = (k..n)
            .max_by(|&i, &j| a[i][k].abs().total_cmp(&a[j][k].abs()))
            .unwrap();
        a.swap(k, pivot);
        b.swap(k, pivot);
        let (upper, lower) = a.split_at_mut(k + 1);
        let pivot_row = &upper[k];
        let (b_upper, b_lower) = b.split_at_mut(k + 1);
        for (row, value) in lower.iter_mut().zip(b_lower) {
            let factor = row[k] / pivot_row[k];
            for (entry, pivot_entry) in row[k..].iter_mut().zip(&pivot_row[k..]) {
                *entry -= factor * pivot_entry;
            }
            *value -= factor * b_upper[k];
        }
    }
    for k in (0..n).rev() {
        let tail: f64 = a[k][k + 1..]
            .iter()
            .zip(&b[k + 1..])
            .map(|(x, y)| x * y)
            .sum();
        b[k] = (b[k] - tail) / a[k][k];
    }
    b
}

/// The stage arguments `g_1 .. g_8` and the new state of one step in the
/// untransformed Rosenbrock form
/// `(I - h gamma J) k_i = h f(t + c_i h, g_i) + h J sum_{j<i} gamma_ij k_j + h^2 gamma_i f_t`,
/// `g_i = y + sum_{j<i} alpha_ij k_j`, `y_new = y + sum b_i k_i`.
fn kform_step(model: &Model, t: f64, y: &[f64], h: f64) -> (Vec<Vec<f64>>, Vec<f64>) {
    let coefficients = rodas5p_coefficients().unwrap();
    let (n, s) = (model.n, coefficients.stages());
    let jacobian = model.jacobian(t, y);
    let mut ft = vec![0.0; n];
    if let Some(partial_t) = &model.partial_t {
        partial_t(t, y, &mut ft).unwrap();
    }
    let matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let identity = if i == j { 1.0 } else { 0.0 };
                    identity - h * coefficients.gamma * jacobian[(i, j)]
                })
                .collect()
        })
        .collect();
    let mut k: Vec<Vec<f64>> = Vec::new();
    let mut arguments = Vec::new();
    for i in 0..s {
        let mut g = y.to_vec();
        let mut coupled = vec![0.0; n];
        for (j, kj) in k.iter().enumerate() {
            for d in 0..n {
                g[d] += coefficients.alpha[(i, j)] * kj[d];
                coupled[d] += coefficients.gamma_matrix[(i, j)] * kj[d];
            }
        }
        let mut f = vec![0.0; n];
        (model.rhs)(t + coefficients.c[i] * h, &g, &mut f).unwrap();
        let jacobian_coupled = jacobian.matvec(&coupled).unwrap();
        let rhs = (0..n)
            .map(|d| {
                h * f[d] + h * jacobian_coupled[d] + h * h * coefficients.gamma_rows[i] * ft[d]
            })
            .collect();
        k.push(solve_small(matrix.clone(), rhs));
        arguments.push(g);
    }
    let mut y_new = y.to_vec();
    for (i, ki) in k.iter().enumerate() {
        for d in 0..n {
            y_new[d] += coefficients.b[i] * ki[d];
        }
    }
    (arguments, y_new)
}

/// (iii), independent form: on constant-step runs with steps large enough
/// to tell the stage arguments apart, the recorded calls agree with the
/// k-form re-computation to rounding, and stages 7 and 8 differ from each
/// other and from the accepted state by far more than that.
#[test]
fn recorded_calls_match_an_independent_k_form_step() {
    for model in [kaps(1.0, "kaps-eps1"), logistic_pair(), pr_nonlinear(1.5)] {
        let steps = 12;
        let h = (model.t_span.1 - model.t_span.0) / steps as f64;
        let config = model.fixed_step_config(steps);
        let lu = model.lu_mode();
        let (recorded, recorder) = model.recorded_problem();
        let (mut t_previous, mut y_previous) = (model.t_span.0, model.y0.clone());
        let mut slot = vec![0.0; model.n];
        let (mut worst, mut smallest_gap) = (0.0_f64, f64::INFINITY);
        let stiff = model.id == "pr-nonlinear";
        let mut control = |view: &AcceptedStepView<'_>| {
            assert_eq!(view.h_used.to_bits(), h.to_bits(), "{}", model.id);
            let (arguments, y_new) = kform_step(&model, t_previous, &y_previous, view.h_used);
            let scale = 1.0 + max_abs(view.y);
            worst = worst.max(max_abs_difference(&y_new, view.y) / scale);
            let mut recorded_arguments = Vec::new();
            for stage in 2..=STAGES {
                recorder
                    .completed_stage_argument_into(stage, &mut slot)
                    .unwrap();
                worst = worst.max(max_abs_difference(&arguments[stage - 1], &slot) / scale);
                recorded_arguments.push(slot.clone());
            }
            let (g7, g8) = (
                &recorded_arguments[STAGES - 3],
                &recorded_arguments[STAGES - 2],
            );
            smallest_gap = smallest_gap
                .min(max_abs_difference(g7, g8) / scale)
                .min(max_abs_difference(g8, view.y) / scale);
            t_previous = view.t;
            y_previous.copy_from_slice(view.y);
            AcceptControl::Continue
        };
        let run = run_client(
            &recorded,
            &recorder,
            &model,
            lu,
            SpeculationMode::Exact,
            &config,
            &model.end_points(),
            &mut control,
        );
        assert_eq!(run.result.accepted_steps, steps, "{}", model.id);
        assert_eq!(run.result.rejected_steps, 0, "{}", model.id);
        // The k-form subtracts stiff terms of size L: its rounding is
        // larger there.
        let tolerance = if stiff { 1.0e-9 } else { 1.0e-12 };
        assert!(worst <= tolerance, "{}: k-form defect {worst:e}", model.id);
        // The agreement tells the stages apart: g7, g8 and the new state
        // are at least a thousand times further from each other than the
        // two computations of any one of them.
        assert!(
            smallest_gap >= 1.0e3 * worst.max(f64::EPSILON),
            "{}: stages 7, 8 and the new state are {smallest_gap:e} apart, the two forms \
             agree to {worst:e}: too close to tell",
            model.id
        );
        println!(
            "SPECJ_KFORM_JSON {}",
            serde_json::json!({
                "model": model.id,
                "steps": steps,
                "h": h,
                "largest_scaled_difference_recorded_vs_k_form": worst,
                "smallest_scaled_gap_between_g7_g8_and_new_state": smallest_gap,
                "tolerance": tolerance,
            })
        );
    }
}

/// What the accept control saw of one accepted step.
struct StepRecord {
    accepted: Point,
    g7: Point,
    g8: Point,
    attempts: usize,
}

/// A run in `mode` with its linearization points and its accepted steps.
struct ObservedRun {
    run: ClientRun,
    trace: Vec<Point>,
    steps: Vec<StepRecord>,
}

fn observe(model: &Model, mode: SpeculationMode, rtol: f64) -> ObservedRun {
    let lu = model.lu_mode();
    let (traced_model, trace) = traced(model);
    let (recorded, recorder) = traced_model.recorded_problem();
    let config = model.adaptive_config(rtol);
    let mut steps = Vec::new();
    let mut slot = vec![0.0; model.n];
    let mut control = |view: &AcceptedStepView<'_>| {
        let mut stage = |stage: usize| {
            let time = recorder
                .completed_stage_argument_into(stage, &mut slot)
                .expect("a complete accepted attempt");
            point(time, &slot)
        };
        steps.push(StepRecord {
            g7: stage(STAGES - 1),
            g8: stage(STAGES),
            accepted: point(view.t, view.y),
            attempts: view.attempts,
        });
        AcceptControl::Continue
    };
    let run = run_client(
        &recorded,
        &recorder,
        model,
        lu,
        mode,
        &config,
        &model.end_points(),
        &mut control,
    );
    ObservedRun {
        run,
        trace: take(&trace),
        steps,
    }
}

/// (iv) without failures: the linearization points of every mode.
#[test]
fn modes_evaluate_the_jacobian_at_the_declared_points() {
    assert_eq!(
        SPECULATIVE_JACOBIAN_STAGE_A_ID,
        "rodas5p-fast-client-speculative-jacobian-a1"
    );
    let cases = [
        (hires(), 1.0e-6),
        (pr_nonlinear(10.0), 1.0e-6),
        (brusselator(100), 1.0e-6),
    ];
    for (model, rtol) in &cases {
        let start = point(model.t_span.0, &model.y0);
        for mode in [
            SpeculationMode::Exact,
            SpeculationMode::G8,
            SpeculationMode::G7,
            SpeculationMode::LaggedOneStep,
        ] {
            let label = format!("{} at rtol {rtol:e}, {}", model.id, mode.label());
            let observed = observe(model, mode, *rtol);
            let result = &observed.run.result;
            let counters = &result.observed.counters;
            assert!(result.observed.success && observed.run.partial.is_none());
            assert_eq!(error_failures(counters), 0, "{label}");
            let accepted = result.accepted_steps;
            assert_eq!(observed.steps.len(), accepted, "{label}");
            // One linearization per accepted state, none for the retry
            // after a local-error rejection.
            assert_eq!(observed.trace.len(), accepted, "{label}");
            assert_eq!(counters.jacobian_builds, accepted as u64, "{label}");
            assert_eq!(result.jacobian_reuses, result.rejected_steps, "{label}");
            assert_eq!(counters.local_error_failures, result.rejected_steps as u64);
            // The first step is exact in every mode.
            assert_eq!(observed.trace[0], start, "{label}");
            // Linearizations whose point is not the accepted state, bit
            // for bit (on a tiny step the embedded error can vanish in
            // rounding, and then g8 is the accepted state).
            let mut away_from_accepted = 0_usize;
            for (k, step) in observed.steps[..accepted - 1].iter().enumerate() {
                let used = &observed.trace[k + 1];
                let expected = match mode {
                    SpeculationMode::Exact => &step.accepted,
                    SpeculationMode::G8 => &step.g8,
                    SpeculationMode::G7 => &step.g7,
                    SpeculationMode::LaggedOneStep => {
                        if k == 0 {
                            &start
                        } else {
                            &observed.steps[k - 1].accepted
                        }
                    }
                };
                assert_eq!(used, expected, "{label}: linearization {}", k + 1);
                // The stage times of g7 and g8 are the accepted time.
                assert_eq!(step.g7.0, step.accepted.0, "{label}");
                assert_eq!(step.g8.0, step.accepted.0, "{label}");
                if matches!(mode, SpeculationMode::G8 | SpeculationMode::G7) {
                    assert_eq!(used.0, step.accepted.0, "{label}: time argument");
                }
                away_from_accepted += usize::from(used.1 != step.accepted.1);
            }
            if mode == SpeculationMode::Exact {
                assert_eq!(away_from_accepted, 0, "{label}");
            } else {
                // The speculative modes did linearize somewhere else on
                // nearly every step.
                assert!(
                    10 * away_from_accepted >= 9 * (accepted - 1),
                    "{label}: only {away_from_accepted} of {} points differ from the accepted \
                     state",
                    accepted - 1
                );
            }
            let speculative = if mode == SpeculationMode::Exact {
                0
            } else {
                accepted as u64 - 1
            };
            assert_eq!(
                observed.run.stats,
                SpeculationStats {
                    linearizations: accepted as u64,
                    speculative,
                    exact_by_mode: if mode == SpeculationMode::Exact {
                        accepted as u64
                    } else {
                        0
                    },
                    exact_first: u64::from(mode != SpeculationMode::Exact),
                    exact_after_failure: 0,
                    exact_no_point: 0,
                    attempts: result.attempts as u64,
                },
                "{label}"
            );
            // Steps accepted after a rejection kept their linearization:
            // counted through the attempts of consecutive accepted steps.
            let after_rejection = observed
                .steps
                .windows(2)
                .filter(|pair| pair[1].attempts - pair[0].attempts > 1)
                .count();
            println!(
                "SPECJ_MODE_POINTS_JSON {}",
                serde_json::json!({
                    "model": model.id,
                    "rtol": rtol,
                    "mode": mode.label(),
                    "attempts": result.attempts,
                    "accepted_steps": accepted,
                    "rejected_steps": result.rejected_steps,
                    "steps_accepted_after_a_rejection": after_rejection,
                    "linearizations": observed.run.stats.linearizations,
                    "speculative_linearizations": speculative,
                    "linearization_points_checked_bitwise": accepted,
                    "linearization_points_not_the_accepted_state": away_from_accepted,
                })
            );
        }
    }
    // The Brusselator run exercised the rejection path in the speculative
    // modes as well.
    for mode in [SpeculationMode::G8, SpeculationMode::G7] {
        let observed = observe(&brusselator(100), mode, 1.0e-6);
        assert!(
            observed.run.result.rejected_steps > 0,
            "{}: no rejection exercised",
            mode.label()
        );
    }
}

/// How an attempt is made to fail with an error, once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Trap {
    /// A stage evaluation of the attempt after the `m`-th linearization
    /// (the fourth right-hand-side call after it: stage 4).
    StageEvaluation,
    /// `f(t, y)` of the attempt after the `m`-th linearization (the first
    /// right-hand-side call after it), before the factorization.
    FirstEvaluation,
    /// The `m`-th Jacobian evaluation itself.
    Jacobian,
}

/// A run of `model` in `mode` whose `linearization`-th (1-based)
/// linearization is followed by one failure of kind `trap`.
fn observe_with_trap(
    model: &Model,
    mode: SpeculationMode,
    rtol: f64,
    trap: Trap,
    linearization: u64,
) -> ObservedRun {
    let lu = model.lu_mode();
    let trace = Trace::default();
    // Jacobian evaluations so far, and right-hand-side calls since the
    // trapped linearization (armed by it).
    let jacobians = Arc::new(AtomicU64::new(0));
    let calls_since = Arc::new(AtomicU64::new(u64::MAX));
    let fired = Arc::new(AtomicBool::new(false));

    let recorder = StageArgumentRecorder::new(model.n).unwrap();
    let inner_rhs = Arc::clone(&model.rhs);
    let (calls, shot) = (Arc::clone(&calls_since), Arc::clone(&fired));
    let trapped_rhs: StageRhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let since = calls.load(Ordering::Relaxed);
        if since != u64::MAX {
            calls.store(since + 1, Ordering::Relaxed);
            let hit = match trap {
                Trap::FirstEvaluation => since == 0,
                Trap::StageEvaluation => since == 3,
                Trap::Jacobian => false,
            };
            if hit && !shot.swap(true, Ordering::Relaxed) {
                return Err(CoreError::NonFinite("trapped right-hand side".into()));
            }
        }
        inner_rhs(t, y, out)
    });
    let rhs = recorder.wrap_rhs(trapped_rhs);

    let n = model.n;
    let fill = Arc::clone(&model.fill);
    let jacobian = Arc::new(move |t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        fill(t, y, &mut j);
        Ok(j)
    });
    let fill = Arc::clone(&model.fill);
    let (sink, count, calls, shot) = (
        Arc::clone(&trace),
        Arc::clone(&jacobians),
        Arc::clone(&calls_since),
        Arc::clone(&fired),
    );
    let jacobian_into: JacobianIntoFn = Arc::new(move |t: f64, y: &[f64], j: &mut DenseMatrix| {
        sink.lock().unwrap().push(point(t, y));
        let index = count.fetch_add(1, Ordering::Relaxed) + 1;
        if index == linearization {
            calls.store(0, Ordering::Relaxed);
            if trap == Trap::Jacobian && !shot.swap(true, Ordering::Relaxed) {
                return Err(CoreError::NonFinite("trapped Jacobian".into()));
            }
        }
        fill(t, y, j);
        Ok(())
    });
    let problem = OdeProblem::new(
        model.id.clone(),
        n,
        rhs,
        None,
        Some(jacobian),
        None,
        model.partial_t.clone(),
        model.autonomous,
        None,
        None,
    )
    .unwrap()
    .with_jacobian_into(jacobian_into);

    let config = model.adaptive_config(rtol);
    let mut steps = Vec::new();
    let mut slot = vec![0.0; model.n];
    let mut control = |view: &AcceptedStepView<'_>| {
        let mut stage = |stage: usize| {
            let time = recorder
                .completed_stage_argument_into(stage, &mut slot)
                .expect("a complete accepted attempt");
            point(time, &slot)
        };
        steps.push(StepRecord {
            g7: stage(STAGES - 1),
            g8: stage(STAGES),
            accepted: point(view.t, view.y),
            attempts: view.attempts,
        });
        AcceptControl::Continue
    };
    let run = run_client(
        &problem,
        &recorder,
        model,
        lu,
        mode,
        &config,
        &model.end_points(),
        &mut control,
    );
    assert!(fired.load(Ordering::Relaxed), "the trap did not fire");
    ObservedRun {
        run,
        trace: take(&trace),
        steps,
    }
}

/// (iv) with a failure: the retry after an attempt that failed with an
/// error linearizes the accepted state exactly, whatever failed (a stage
/// evaluation, `f(t, y)` before the factorization, or the Jacobian at the
/// speculative point), and the run goes on speculatively.
#[test]
fn an_attempt_that_failed_with_an_error_is_retried_with_the_exact_jacobian() {
    let model = hires();
    let rtol = 1.0e-6;
    let start = point(model.t_span.0, &model.y0);
    // The failure follows the sixth linearization: the state after five
    // accepted steps.
    let trapped = 6_u64;
    for mode in [
        SpeculationMode::G8,
        SpeculationMode::G7,
        SpeculationMode::LaggedOneStep,
    ] {
        for trap in [Trap::StageEvaluation, Trap::FirstEvaluation, Trap::Jacobian] {
            let label = format!("{} {} {trap:?}", model.id, mode.label());
            let observed = observe_with_trap(&model, mode, rtol, trap, trapped);
            let result = &observed.run.result;
            let counters = &result.observed.counters;
            assert!(result.observed.success && observed.run.partial.is_none());
            assert_eq!(counters.nonfinite_step_failures, 1, "{label}");
            assert_eq!(error_failures(counters), 1, "{label}");
            let accepted = result.accepted_steps;
            // One linearization more than accepted steps: the retry.
            assert_eq!(observed.trace.len(), accepted + 1, "{label}");
            assert_eq!(counters.jacobian_builds, accepted as u64 + 1, "{label}");
            assert_eq!(observed.trace[0], start, "{label}");
            let failed = trapped as usize - 1;
            let state = &observed.steps[failed - 1];
            // The trapped linearization used the mode's point ...
            let speculative = match mode {
                SpeculationMode::G8 => &state.g8,
                SpeculationMode::G7 => &state.g7,
                _ => &observed.steps[failed - 2].accepted,
            };
            assert_eq!(&observed.trace[failed], speculative, "{label}");
            // ... and the retry used the accepted state itself.
            assert_eq!(observed.trace[failed + 1], state.accepted, "{label}");
            // Every other linearization is the mode's.
            for (k, step) in observed.steps[..accepted - 1].iter().enumerate() {
                let index = if k < failed { k + 1 } else { k + 2 };
                let expected = match mode {
                    SpeculationMode::G8 => &step.g8,
                    SpeculationMode::G7 => &step.g7,
                    _ => {
                        if k == 0 {
                            &start
                        } else {
                            &observed.steps[k - 1].accepted
                        }
                    }
                };
                assert_eq!(&observed.trace[index], expected, "{label}: step {k}");
            }
            assert_eq!(
                observed.run.stats,
                SpeculationStats {
                    linearizations: accepted as u64 + 1,
                    speculative: accepted as u64 - 1,
                    exact_by_mode: 0,
                    exact_first: 1,
                    exact_after_failure: 1,
                    exact_no_point: 0,
                    // A failed Jacobian or f(t, y) ends its attempt before
                    // the factorization.
                    attempts: result.attempts as u64 - u64::from(trap != Trap::StageEvaluation),
                },
                "{label}"
            );
            println!(
                "SPECJ_FAILURE_RETRY_JSON {}",
                serde_json::json!({
                    "model": model.id,
                    "rtol": rtol,
                    "mode": mode.label(),
                    "trap": format!("{trap:?}"),
                    "attempts": result.attempts,
                    "accepted_steps": accepted,
                    "nonfinite_step_failures": counters.nonfinite_step_failures,
                    "linearizations": observed.run.stats.linearizations,
                    "exact_after_failure": observed.run.stats.exact_after_failure,
                    "retry_point_is_the_accepted_state_bitwise": true,
                })
            );
        }
    }
}

/// (v): the driver evaluates the time derivative at accepted states only,
/// once per attempt from a fresh state, in the speculative modes as well.
#[test]
fn the_time_derivative_is_evaluated_at_accepted_states_only() {
    for mode in [SpeculationMode::G8, SpeculationMode::G7] {
        let mut model = pr_nonlinear(10.0);
        let derivative_trace = Trace::default();
        let inner = model.partial_t.clone().unwrap();
        let sink = Arc::clone(&derivative_trace);
        model.partial_t = Some(Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            sink.lock().unwrap().push(point(t, y));
            inner(t, y, out)
        }));
        let observed = observe(&model, mode, 1.0e-6);
        let result = &observed.run.result;
        assert!(result.observed.success);
        assert_eq!(error_failures(&result.observed.counters), 0);
        let accepted = result.accepted_steps;
        let derivatives = take(&derivative_trace);
        assert_eq!(derivatives.len(), accepted, "{}", mode.label());
        assert_eq!(result.observed.counters.ft_calls, accepted as u64);
        assert_eq!(derivatives[0], point(model.t_span.0, &model.y0));
        let mut jacobian_elsewhere = 0_usize;
        for (k, step) in observed.steps[..accepted - 1].iter().enumerate() {
            assert_eq!(
                derivatives[k + 1],
                step.accepted,
                "{} step {k}",
                mode.label()
            );
            // The Jacobian of the same attempt was evaluated at the same
            // time and (but for steps whose error vanishes in rounding)
            // at another state.
            assert_eq!(observed.trace[k + 1].0, step.accepted.0);
            jacobian_elsewhere += usize::from(observed.trace[k + 1].1 != step.accepted.1);
        }
        assert!(
            10 * jacobian_elsewhere >= 9 * (accepted - 1),
            "{}: {jacobian_elsewhere} of {}",
            mode.label(),
            accepted - 1
        );
        println!(
            "SPECJ_TIME_DERIVATIVE_JSON {}",
            serde_json::json!({
                "model": model.id,
                "mode": mode.label(),
                "accepted_steps": accepted,
                "time_derivative_calls": derivatives.len(),
                "time_derivative_points_are_accepted_states_bitwise": true,
                "jacobian_points_not_the_accepted_state": jacobian_elsewhere,
            })
        );
    }
}

/// Refusals, the reset, and the two ways a speculative point is lost.
#[test]
fn misuse_is_refused_and_a_lost_point_falls_back_to_the_exact_jacobian() {
    assert!(matches!(
        StageArgumentRecorder::new(0),
        Err(CoreError::InvalidInput(_))
    ));
    let model = hires();
    let wrong = StageArgumentRecorder::new(model.n + 1).unwrap();
    assert!(matches!(
        SpeculativeJacobianClient::new(
            DenseLuStageClient::new(model.n, DenseLuMode::InPlace).unwrap(),
            SpeculationMode::G8,
            wrong,
        ),
        Err(CoreError::InvalidInput(_))
    ));

    let (problem, recorder) = model.recorded_problem();
    let mut out = vec![0.0; model.n];
    // Nothing before an attempt; no stage 1 or 9; no short buffer.
    assert_eq!(recorder.completed_stage_argument_into(8, &mut out), None);
    let mut client = SpeculativeJacobianClient::new(
        DenseLuStageClient::new(model.n, DenseLuMode::InPlace).unwrap(),
        SpeculationMode::G8,
        Arc::clone(&recorder),
    )
    .unwrap();
    let handle = client.handle();
    let mut counters = WorkCounters::default();
    let y0 = model.y0.clone();
    let rhs_through_wrapper = |t: f64, y: &[f64]| {
        let mut f = vec![0.0; model.n];
        problem
            .eval_rhs_into(t, y, &mut f, &mut WorkCounters::default())
            .unwrap();
    };

    // A hand-driven attempt: linearize, factor, seven stage calls.
    client.linearize(&problem, 0.0, &y0, &mut counters).unwrap();
    assert_eq!(client.stats().exact_first, 1);
    client.factor(1.0e3, &mut counters).unwrap();
    for stage in 2..=STAGES {
        assert!(!recorder.attempt_is_complete());
        let y: Vec<f64> = y0.iter().map(|v| v + stage as f64).collect();
        rhs_through_wrapper(1.0, &y);
    }
    assert!(recorder.attempt_is_complete());
    assert_eq!(recorder.completed_stage_argument_into(1, &mut out), None);
    assert_eq!(recorder.completed_stage_argument_into(9, &mut out), None);
    assert_eq!(
        recorder.completed_stage_argument_into(8, &mut out[..model.n - 1]),
        None
    );
    assert_eq!(
        recorder.completed_stage_argument_into(8, &mut out),
        Some(1.0)
    );
    assert_eq!(out[0], y0[0] + 8.0);
    assert_eq!(
        recorder.completed_stage_argument_into(7, &mut out),
        Some(1.0)
    );
    assert_eq!(out[0], y0[0] + 7.0);

    // A linearization at the recorded stage time uses the point ...
    let accepted: Vec<f64> = y0.iter().map(|v| v + 8.5).collect();
    client
        .linearize(&problem, 1.0, &accepted, &mut counters)
        .unwrap();
    assert_eq!(client.stats().speculative, 1);
    // ... one at the same time again is the retry: exact ...
    client
        .linearize(&problem, 1.0, &accepted, &mut counters)
        .unwrap();
    assert_eq!(client.stats().exact_after_failure, 1);
    // ... and one at another time has no point: exact.
    client
        .linearize(&problem, 1.5, &accepted, &mut counters)
        .unwrap();
    assert_eq!(client.stats().exact_no_point, 1);

    // An extra call through the wrapper after the attempt loses the point.
    client.factor(1.0e3, &mut counters).unwrap();
    for _ in 2..=STAGES {
        rhs_through_wrapper(2.0, &y0);
    }
    assert!(recorder.attempt_is_complete());
    rhs_through_wrapper(2.0, &y0);
    assert!(!recorder.attempt_is_complete());
    assert_eq!(recorder.calls_in_attempt(), STAGES);
    client.linearize(&problem, 2.0, &y0, &mut counters).unwrap();
    assert_eq!(client.stats().exact_no_point, 2);
    assert_eq!(client.stats().speculative, 1);

    // A wrong shape through the wrapper never becomes a point.
    let other = StageArgumentRecorder::new(3).unwrap();
    let wrapped = other.wrap_rhs(Arc::new(|_t: f64, _y: &[f64], _out: &mut [f64]| Ok(())));
    other.begin_attempt();
    for _ in 2..=STAGES {
        wrapped(0.0, &[1.0, 2.0], &mut [0.0, 0.0]).unwrap();
    }
    assert_eq!(other.calls_in_attempt(), STAGES - 1);
    assert!(!other.attempt_is_complete());

    // The handle follows the client; the reset makes the next
    // linearization a first one.
    assert_eq!(handle.snapshot(), client.stats());
    assert_eq!(handle.snapshot().linearizations, 5);
    assert_eq!(handle.snapshot().attempts, 2);
    client.reset();
    assert_eq!(handle.snapshot(), SpeculationStats::default());
    client.factor(1.0e3, &mut counters).unwrap();
    for _ in 2..=STAGES {
        rhs_through_wrapper(3.0, &y0);
    }
    client.linearize(&problem, 3.0, &y0, &mut counters).unwrap();
    assert_eq!(client.stats().exact_first, 1);
    assert_eq!(client.stats().speculative, 0);
    assert_eq!(client.dimension(), model.n);
    assert_eq!(client.inner().mode(), DenseLuMode::InPlace);
    assert_eq!(client.into_inner().dimension(), model.n);
    // A linearization of a state of another length is refused.
    let mut client = SpeculativeJacobianClient::new(
        DenseLuStageClient::new(model.n, DenseLuMode::InPlace).unwrap(),
        SpeculationMode::LaggedOneStep,
        recorder,
    )
    .unwrap();
    assert!(matches!(
        client.linearize(&problem, 0.0, &y0[..model.n - 1], &mut counters),
        Err(CoreError::Dimension(_))
    ));
}

/// (vi) analytic Jacobians against central differences of the right-hand
/// side, entry by entry, at two states per model.
#[test]
fn model_jacobians_match_finite_differences() {
    let mut models = adaptive_models();
    models.extend(order_models());
    models.push(meanfield_brusselator(8, 0.1));
    for model in &models {
        for seed in 0..2 {
            let y = probe_state(model, seed);
            let t = model.t_span.0 + 0.37 * (seed as f64 + 1.0) * (model.t_span.1 - model.t_span.0);
            let jacobian = model.jacobian(t, &y);
            let scale = max_abs(&y);
            let mut f = vec![0.0; model.n];
            (model.rhs)(t, &y, &mut f).unwrap();
            let (mut worst, mut entries) = (0.0_f64, 0_usize);
            for column in 0..model.n {
                let delta = 1.0e-4 * (y[column].abs() + 1.0e-3 * scale);
                let (mut plus, mut minus) = (y.clone(), y.clone());
                plus[column] += delta;
                minus[column] -= delta;
                let (mut f_plus, mut f_minus) = (vec![0.0; model.n], vec![0.0; model.n]);
                (model.rhs)(t, &plus, &mut f_plus).unwrap();
                (model.rhs)(t, &minus, &mut f_minus).unwrap();
                let column_scale = (0..model.n)
                    .map(|row| jacobian[(row, column)].abs())
                    .fold(0.0_f64, f64::max);
                for row in 0..model.n {
                    let difference = (f_plus[row] - f_minus[row]) / (2.0 * delta);
                    // Rounding of the difference quotient: the terms of
                    // row `row` are bounded by sum |J_rj| |y_j| + |f_row|.
                    let terms = (0..model.n)
                        .map(|j| jacobian[(row, j)].abs() * (y[j].abs() + delta))
                        .sum::<f64>()
                        + f[row].abs();
                    let tolerance = 1.0e-6 * column_scale + 256.0 * f64::EPSILON * terms / delta;
                    let defect = (difference - jacobian[(row, column)]).abs();
                    assert!(
                        defect <= tolerance,
                        "{} seed {seed}: J[{row}][{column}] = {:e}, difference quotient \
                         {difference:e} (defect {defect:e}, tolerance {tolerance:e})",
                        model.id,
                        jacobian[(row, column)]
                    );
                    worst = worst.max(defect / tolerance.max(f64::MIN_POSITIVE));
                    entries += 1;
                }
            }
            println!(
                "SPECJ_MODEL_JACOBIAN_JSON {}",
                serde_json::json!({
                    "model": model.id,
                    "seed": seed,
                    "entries_checked": entries,
                    "largest_defect_over_tolerance": worst,
                })
            );
        }
        // The analytic time derivative against a central difference in t.
        if let Some(partial_t) = &model.partial_t {
            let y = probe_state(model, 3);
            let t = 0.61;
            let mut analytic = vec![0.0; model.n];
            partial_t(t, &y, &mut analytic).unwrap();
            let delta = 1.0e-5;
            let (mut plus, mut minus) = (vec![0.0; model.n], vec![0.0; model.n]);
            (model.rhs)(t + delta, &y, &mut plus).unwrap();
            (model.rhs)(t - delta, &y, &mut minus).unwrap();
            for i in 0..model.n {
                let difference = (plus[i] - minus[i]) / (2.0 * delta);
                assert!(
                    (difference - analytic[i]).abs() <= 1.0e-6 * (1.0 + analytic[i].abs()),
                    "{}: f_t[{i}] = {:e}, difference quotient {difference:e}",
                    model.id,
                    analytic[i]
                );
            }
        }
    }
}

/// (vi) the closed forms solve their equations (a central difference of the
/// closed form against the right-hand side on it) and start at `y0`.
#[test]
fn closed_forms_solve_their_equations() {
    let mut models = order_models();
    models.push(pr_nonlinear(10.0));
    for model in &models {
        let exact = model.exact.as_ref().expect("a closed form");
        assert_eq!(
            bits(&exact(model.t_span.0)),
            bits(&model.y0),
            "{}",
            model.id
        );
        for k in 1..=8 {
            let t = model.t_span.0 + (model.t_span.1 - model.t_span.0) * k as f64 / 8.0;
            let y = exact(t);
            let mut f = vec![0.0; model.n];
            (model.rhs)(t, &y, &mut f).unwrap();
            let delta = 1.0e-5;
            let (plus, minus) = (exact(t + delta), exact(t - delta));
            for i in 0..model.n {
                let derivative = (plus[i] - minus[i]) / (2.0 * delta);
                // On the closed form the stiff terms cancel to rounding of
                // (y - phi) times the stiff rate.
                let stiffness = if model.id.starts_with("pr-") || model.id == "kaps-eps1e-6" {
                    1.0e6
                } else {
                    1.0
                };
                let tolerance = 1.0e-8 * (1.0 + derivative.abs())
                    + 64.0 * f64::EPSILON * stiffness * (1.0 + y[i].abs());
                assert!(
                    (derivative - f[i]).abs() <= tolerance,
                    "{} at t = {t}: y'[{i}] = {derivative:e}, f = {:e}",
                    model.id,
                    f[i]
                );
            }
        }
    }
}

/// (vi) the models are the definitions already in the repository, bit for
/// bit, at probe states: Robertson (library), HIRES and the Brusselator
/// (`b1_problems`), Oregonator and POLLU (scientific corpus v2).
#[test]
fn models_agree_bitwise_with_the_repository_definitions() {
    let corpus = |family: ScientificFamily| {
        ScientificCorpusV2::holdout_specs()
            .into_iter()
            .find(|spec| spec.family == family)
            .expect("a corpus case")
            .build()
            .expect("a corpus problem")
    };
    let oregonator_case = corpus(ScientificFamily::Oregonator);
    let pollution_case = corpus(ScientificFamily::Pollution);
    let pairs: Vec<(Model, OdeProblem, Vec<f64>, bool)> = vec![
        {
            let (problem, y0) = robertson_problem().unwrap();
            (robertson(), problem, y0, true)
        },
        {
            let (problem, y0) = b1_problems::hires_problem().unwrap();
            (hires(), problem, y0, true)
        },
        {
            let (problem, y0) = b1_problems::brusselator_problem(32).unwrap();
            (brusselator(32), problem, y0, true)
        },
        {
            let (problem, y0) = b1_problems::brusselator_problem(100).unwrap();
            (brusselator(100), problem, y0, true)
        },
        // The corpus problems carry a JVP, no explicit Jacobian.
        (
            oregonator(),
            oregonator_case.problem.clone(),
            oregonator_case.y0.clone(),
            false,
        ),
        (
            pollution(),
            pollution_case.problem.clone(),
            pollution_case.y0.clone(),
            false,
        ),
    ];
    for (model, problem, y0, compare_jacobian) in &pairs {
        assert_eq!(model.n, problem.dimension, "{}", model.id);
        assert_eq!(bits(&model.y0), bits(y0), "{}: y0", model.id);
        for seed in 0..3 {
            let y = probe_state(model, seed);
            let (mut mine, mut theirs) = (vec![0.0; model.n], vec![0.0; model.n]);
            (model.rhs)(0.0, &y, &mut mine).unwrap();
            problem
                .eval_rhs_into(0.0, &y, &mut theirs, &mut WorkCounters::default())
                .unwrap();
            assert_eq!(bits(&mine), bits(&theirs), "{} seed {seed}: f", model.id);
            if *compare_jacobian {
                let theirs = problem
                    .dense_jacobian(0.0, &y, &mut WorkCounters::default())
                    .unwrap();
                assert_eq!(
                    bits(model.jacobian(0.0, &y).as_slice()),
                    bits(theirs.as_slice()),
                    "{} seed {seed}: J",
                    model.id
                );
            } else {
                // J v against the corpus JVP, to rounding of the row sums.
                let v: Vec<f64> = (0..model.n).map(|i| ((i + 1) as f64).cos()).collect();
                let mine = model.jacobian(0.0, &y).matvec(&v).unwrap();
                let jvp = problem.jvp_callback().expect("a corpus JVP");
                let mut theirs = vec![0.0; model.n];
                jvp(0.0, &y, &v, &mut theirs).unwrap();
                let scale = max_abs(&mine).max(max_abs(&theirs));
                assert!(
                    max_abs_difference(&mine, &theirs) <= 1.0e-9 * scale,
                    "{} seed {seed}: J v",
                    model.id
                );
            }
        }
    }
    // The tolerance rules and the step counts of the measurement.
    assert_eq!(hires().atol, AtolRule::Scale(1.0e-4));
    assert_eq!(pr_linear().n, 1);
    assert_eq!(ORDER_STEPS.len(), 16);
    for steps in ORDER_STEPS {
        // Every step size 1.5 / N and every grid time is a dyadic rational.
        let h = 1.5 / steps as f64;
        assert_eq!(h * steps as f64, 1.5);
        let scaled = h * (1_u64 << 20) as f64;
        assert_eq!(scaled, scaled.round(), "1.5 / {steps} is not dyadic");
    }
}
