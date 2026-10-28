//! Partial results and the segment-recording dense driver (wave-2 node
//! `p0-facade`, plan §3 P0 tier).
//!
//! * The fast dense, banded and matrix-free drivers report the last accepted
//!   `(t, y)`, why they stopped and the kind of the last rejection when a
//!   run ends before `tf`. Recording it changes no arithmetic: every run
//!   here is compared bit for bit with the plain entry point.
//! * `integrate_adaptive_dense_observed_with_segments` with
//!   `InteriorSampling::OutputTimes` is the named dense driver bit for bit,
//!   and its segments reproduce the driver's interior samples.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rodas5p_core::{CoreError, DenseMatrix, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveFailureKind, AdaptiveStepConfig, BandedJacobian, DenseErrorControl, DriverPartial,
    DriverStopReason, IntegrationMethod, InteriorSampling, ObservedIntegrationResult, OdeProblem,
    OutputSamplingPlan, OutputSchedule, RoutingOptions,
    integrate_adaptive_dense_observed_with_dense_error_control,
    integrate_adaptive_dense_observed_with_segments, integrate_rodas5p_fast_banded_observed,
    integrate_rodas5p_fast_observed, integrate_rodas5p_fast_observed_with_partial,
    integrate_rodas5p_mf_fast_observed_gmres_into, integrate_rodas5p_routed_observed,
    prothero_robinson_problem, robertson_problem,
};

type JacobianFn = Arc<dyn Fn(f64, &[f64]) -> rodas5p_core::CoreResult<DenseMatrix> + Send + Sync>;
type JvpFn =
    Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync>;

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|v| v.to_bits()).collect()
}

fn assert_observed_identical(a: &ObservedIntegrationResult, b: &ObservedIntegrationResult) {
    assert_eq!(bits(&a.t), bits(&b.t));
    assert_eq!(a.y.len(), b.y.len());
    for (x, y) in a.y.iter().zip(&b.y) {
        assert_eq!(bits(x), bits(y));
    }
    assert_eq!(a.counters, b.counters);
    assert_eq!(a.success, b.success);
    assert_eq!(a.internal_steps, b.internal_steps);
}

fn span_schedule(t0: f64, t1: f64) -> OutputSchedule {
    OutputSchedule::new(vec![t0, t1]).unwrap()
}

/// `y' = -y` whose RHS fails (`NonFinite`) from the first call with
/// `t > 0.5` on, at every later call: the latch of the facade's guard.
fn latched_decay(with_jvp: bool, latch: Arc<AtomicBool>) -> OdeProblem {
    let fail = latch.clone();
    OdeProblem::new(
        "latched-decay",
        1,
        Arc::new(move |t, y, out| {
            if t > 0.5 {
                fail.store(true, Ordering::Relaxed);
            }
            if fail.load(Ordering::Relaxed) {
                return Err(CoreError::NonFinite("latched".into()));
            }
            out[0] = -y[0];
            Ok(())
        }),
        None,
        (!with_jvp).then(|| {
            Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![-1.0])) as JacobianFn
        }),
        with_jvp.then(|| {
            Arc::new(|_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
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

fn assert_failed_partial(partial: &DriverPartial, last_sample_t: f64) {
    assert!(
        partial.t > last_sample_t && partial.t <= 0.5 + 1e-12,
        "{partial:?}"
    );
    assert_eq!(partial.reason, DriverStopReason::MinimumStep);
    assert_eq!(partial.last_failure, Some(AdaptiveFailureKind::NonFinite));
    // y' = -y: the last accepted state is close to exp(-t).
    assert!(
        (partial.y[0] - (-partial.t).exp()).abs() < 1e-6,
        "{partial:?}"
    );
}

// ------------------------------------------------------------ fast drivers

#[test]
fn fast_dense_partial_is_none_on_success_and_the_run_is_bit_identical() {
    let (problem, y0) = robertson_problem().unwrap();
    let adaptive = AdaptiveStepConfig {
        rtol: 1e-6,
        atol: 1e-10,
        ..AdaptiveStepConfig::default()
    };
    let schedule = span_schedule(0.0, 40.0);
    let plain =
        integrate_rodas5p_fast_observed(&problem, (0.0, 40.0), &y0, &adaptive, &schedule).unwrap();
    let (with, partial) = integrate_rodas5p_fast_observed_with_partial(
        &problem,
        (0.0, 40.0),
        &y0,
        &adaptive,
        &schedule,
    )
    .unwrap();
    assert!(plain.observed.success);
    assert_observed_identical(&plain.observed, &with.observed);
    assert_eq!(plain.attempts, with.attempts);
    assert_eq!(plain.jacobian_reuses, with.jacobian_reuses);
    assert_eq!(partial, None);
}

#[test]
fn fast_dense_partial_reports_the_attempt_budget_and_the_last_accepted_state() {
    let (problem, y0) = robertson_problem().unwrap();
    let adaptive = AdaptiveStepConfig {
        max_attempts: 7,
        ..AdaptiveStepConfig::default()
    };
    let schedule = span_schedule(0.0, 40.0);
    let plain =
        integrate_rodas5p_fast_observed(&problem, (0.0, 40.0), &y0, &adaptive, &schedule).unwrap();
    let (with, partial) = integrate_rodas5p_fast_observed_with_partial(
        &problem,
        (0.0, 40.0),
        &y0,
        &adaptive,
        &schedule,
    )
    .unwrap();
    assert!(!plain.observed.success);
    assert_observed_identical(&plain.observed, &with.observed);
    let partial = partial.expect("a failed run has a partial record");
    assert_eq!(partial.reason, DriverStopReason::AttemptBudget);
    assert!(partial.t > 0.0 && partial.t < 40.0);
    assert_eq!(with.attempts, 7);
    // Robertson conserves y1 + y2 + y3 = 1 to roundoff.
    assert!((partial.y.iter().sum::<f64>() - 1.0).abs() < 1e-12);
}

#[test]
fn fast_dense_partial_after_a_latched_rhs_failure() {
    let latch = Arc::new(AtomicBool::new(false));
    let problem = latched_decay(false, latch);
    let (result, partial) = integrate_rodas5p_fast_observed_with_partial(
        &problem,
        (0.0, 1.0),
        &[1.0],
        &AdaptiveStepConfig::default(),
        &span_schedule(0.0, 1.0),
    )
    .unwrap();
    assert!(!result.observed.success);
    assert_eq!(result.observed.t, vec![0.0]);
    assert_failed_partial(&partial.unwrap(), 0.0);
}

#[test]
fn router_carries_the_partial_record_of_the_routed_driver() {
    let latch = Arc::new(AtomicBool::new(false));
    let routed = integrate_rodas5p_routed_observed(
        &latched_decay(false, latch.clone()),
        (0.0, 1.0),
        &[1.0],
        &AdaptiveStepConfig::default(),
        &span_schedule(0.0, 1.0),
    )
    .unwrap();
    latch.store(false, Ordering::Relaxed);
    let (direct, partial) = integrate_rodas5p_fast_observed_with_partial(
        &latched_decay(false, latch),
        (0.0, 1.0),
        &[1.0],
        &AdaptiveStepConfig::default(),
        &span_schedule(0.0, 1.0),
    )
    .unwrap();
    assert_observed_identical(routed.run.observed(), &direct.observed);
    assert_eq!(routed.partial, partial);
    assert!(routed.partial.is_some());
}

/// `y_i' = y_{i-1} - 2 y_i + y_{i+1}` (n = 6), RHS latched beyond t = 0.5.
fn latched_tridiagonal(latch: Arc<AtomicBool>) -> (OdeProblem, BandedJacobian) {
    let n = 6;
    let fail = latch.clone();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        if t > 0.5 {
            fail.store(true, Ordering::Relaxed);
        }
        if fail.load(Ordering::Relaxed) {
            return Err(CoreError::NonFinite("latched".into()));
        }
        for i in 0..n {
            let l = if i > 0 { y[i - 1] } else { 0.0 };
            let r = if i + 1 < n { y[i + 1] } else { 0.0 };
            out[i] = l - 2.0 * y[i] + r;
        }
        Ok(())
    });
    let dense = Arc::new(move |_t: f64, _y: &[f64]| {
        let mut m = DenseMatrix::zeros(n, n);
        for i in 0..n {
            m[(i, i)] = -2.0;
            if i > 0 {
                m[(i, i - 1)] = 1.0;
            }
            if i + 1 < n {
                m[(i, i + 1)] = 1.0;
            }
        }
        Ok(m)
    });
    let fill = Arc::new(move |_t: f64, _y: &[f64], band: &mut [f64]| {
        for i in 0..n {
            band[i * 3] = if i > 0 { 1.0 } else { 0.0 };
            band[i * 3 + 1] = -2.0;
            band[i * 3 + 2] = if i + 1 < n { 1.0 } else { 0.0 };
        }
        Ok(())
    });
    let problem = OdeProblem::new(
        "latched-tridiagonal",
        n,
        rhs,
        None,
        Some(dense),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    (
        problem,
        BandedJacobian {
            lower: 1,
            upper: 1,
            fill,
        },
    )
}

#[test]
fn fast_banded_partial_after_a_latched_rhs_failure_and_none_on_success() {
    let latch = Arc::new(AtomicBool::new(false));
    let (problem, band) = latched_tridiagonal(latch);
    let y0 = vec![1.0; 6];
    let failed = integrate_rodas5p_fast_banded_observed(
        &problem,
        &band,
        (0.0, 1.0),
        &y0,
        &AdaptiveStepConfig::default(),
        &span_schedule(0.0, 1.0),
    )
    .unwrap();
    assert!(!failed.fast.observed.success);
    let partial = failed.partial.unwrap();
    assert!(partial.t > 0.0 && partial.t <= 0.5);
    assert_eq!(partial.reason, DriverStopReason::MinimumStep);
    assert_eq!(partial.last_failure, Some(AdaptiveFailureKind::NonFinite));

    let ok = integrate_rodas5p_fast_banded_observed(
        &problem_without_latch(),
        &band,
        (0.0, 0.4),
        &y0,
        &AdaptiveStepConfig::default(),
        &span_schedule(0.0, 0.4),
    )
    .unwrap();
    assert!(ok.fast.observed.success);
    assert_eq!(ok.partial, None);
}

fn problem_without_latch() -> OdeProblem {
    // The latch never fires before t = 0.5.
    latched_tridiagonal(Arc::new(AtomicBool::new(false))).0
}

// --------------------------------------------------------- matrix-free

#[test]
fn matrix_free_partial_after_a_latched_rhs_failure_and_none_on_success() {
    let linear: LinearSolverConfig = RoutingOptions::default().matrix_free;
    let latch = Arc::new(AtomicBool::new(false));
    let failed = integrate_rodas5p_mf_fast_observed_gmres_into(
        &latched_decay(true, latch),
        (0.0, 1.0),
        &[1.0],
        &linear,
        &AdaptiveStepConfig::default(),
        &span_schedule(0.0, 1.0),
    )
    .unwrap();
    assert!(!failed.observed.success);
    assert_failed_partial(failed.partial.as_ref().unwrap(), 0.0);

    let ok = integrate_rodas5p_mf_fast_observed_gmres_into(
        &latched_decay(true, Arc::new(AtomicBool::new(false))),
        (0.0, 0.4),
        &[1.0],
        &linear,
        &AdaptiveStepConfig::default(),
        &span_schedule(0.0, 0.4),
    )
    .unwrap();
    assert!(ok.observed.success);
    assert_eq!(ok.partial, None);
}

// --------------------------------------------------- segment dense driver

fn pr_adaptive(rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * 1.0e-2,
        rtol,
        ..AdaptiveStepConfig::default()
    }
}

fn pr_grid() -> OutputSamplingPlan {
    let times = (0..=200).map(|i| i as f64 * 0.01).collect::<Vec<_>>();
    OutputSamplingPlan::dense(OutputSchedule::new(times).unwrap())
}

#[test]
fn segment_driver_with_output_time_sampling_is_the_named_driver_bit_for_bit() {
    let (problem, y0) = prothero_robinson_problem(-1.0e5, 0.0, 0.0);
    for control in [
        DenseErrorControl::Off,
        DenseErrorControl::Enforce,
        DenseErrorControl::WFilteredEnforce,
    ] {
        let adaptive = pr_adaptive(1.0e-6);
        let (named, named_report) = integrate_adaptive_dense_observed_with_dense_error_control(
            &problem,
            (0.0, 2.0),
            &y0,
            IntegrationMethod::Sequential,
            None,
            None,
            &adaptive,
            &pr_grid(),
            control,
        )
        .unwrap();
        let run = integrate_adaptive_dense_observed_with_segments(
            &problem,
            (0.0, 2.0),
            &y0,
            IntegrationMethod::Sequential,
            None,
            None,
            &adaptive,
            &pr_grid(),
            control,
            InteriorSampling::OutputTimes,
        )
        .unwrap();
        assert_observed_identical(&named.observed, &run.result.observed);
        assert_eq!(named.diagnostics.attempts, run.result.diagnostics.attempts);
        assert_eq!(named_report, run.report, "{control:?}");
        assert_eq!(run.partial, None);
        assert_eq!(run.segments.len(), named.observed.internal_steps);
        // Every interior sample is the segment's extension at that time.
        for (t, y) in named.observed.t.iter().zip(&named.observed.y) {
            let segment = run
                .segments
                .iter()
                .find(|s| s.t_old <= *t && *t <= s.t_new)
                .unwrap();
            assert_eq!(bits(&segment.evaluate(*t).unwrap()), bits(y), "t = {t}");
        }
        // The segments tile the span.
        assert_eq!(run.segments[0].t_old, 0.0);
        assert_eq!(run.segments.last().unwrap().t_new, 2.0);
        for pair in run.segments.windows(2) {
            assert_eq!(pair[0].t_new, pair[1].t_old);
            assert_eq!(bits(&pair[0].y_new), bits(&pair[1].y_old));
        }
    }
}

#[test]
fn every_step_sampling_off_is_the_named_driver_and_wfiltered_samples_every_step() {
    let (problem, y0) = prothero_robinson_problem(-1.0e5, 0.0, 0.0);
    let adaptive = pr_adaptive(1.0e-6);
    let span_only = OutputSamplingPlan::dense(span_schedule(0.0, 2.0));
    // Off forms no estimate under either rule.
    let (named, _) = integrate_adaptive_dense_observed_with_dense_error_control(
        &problem,
        (0.0, 2.0),
        &y0,
        IntegrationMethod::Sequential,
        None,
        None,
        &adaptive,
        &span_only,
        DenseErrorControl::Off,
    )
    .unwrap();
    let off = integrate_adaptive_dense_observed_with_segments(
        &problem,
        (0.0, 2.0),
        &y0,
        IntegrationMethod::Sequential,
        None,
        None,
        &adaptive,
        &span_only,
        DenseErrorControl::Off,
        InteriorSampling::EveryStep,
    )
    .unwrap();
    assert_observed_identical(&named.observed, &off.result.observed);
    assert!(off.report.samples.is_empty());

    // WFilteredEnforce on every step: one sample per embedded-accepted
    // attempt, none unavailable (explicit Jacobian, direct LU).
    let wf = integrate_adaptive_dense_observed_with_segments(
        &problem,
        (0.0, 2.0),
        &y0,
        IntegrationMethod::Sequential,
        None,
        None,
        &adaptive,
        &span_only,
        DenseErrorControl::WFilteredEnforce,
        InteriorSampling::EveryStep,
    )
    .unwrap();
    assert!(wf.result.observed.success);
    assert_eq!(wf.report.unavailable, 0);
    let accepted_samples = wf.report.samples.iter().filter(|s| s.accepted).count();
    assert_eq!(accepted_samples, wf.segments.len());
    // The interior error of the returned extension is controlled: at most
    // 2 tolerance units at 201 points (the A2 interior criterion), against
    // hundreds of units for Off.
    let worst = |segments: &[rodas5p_integrators::Rodas5pDenseSegment]| {
        (0..=200)
            .map(|i| {
                let t = i as f64 * 0.01;
                let s = segments
                    .iter()
                    .find(|s| s.t_old <= t && t <= s.t_new)
                    .unwrap();
                let y = s.evaluate(t).unwrap()[0];
                (y - t.sin()).abs() / (adaptive.atol + adaptive.rtol * t.sin().abs())
            })
            .fold(0.0_f64, f64::max)
    };
    let (worst_wf, worst_off) = (worst(&wf.segments), worst(&off.segments));
    eprintln!("PR interior max: WFilteredEnforce every step {worst_wf:.4}, Off {worst_off:.4}");
    assert!(worst_wf <= 2.0, "{worst_wf}");
    assert!(worst_off > 10.0, "{worst_off}");
}

#[test]
fn segment_driver_reports_a_partial_record_on_failure() {
    let (problem, y0) = prothero_robinson_problem(-1.0e5, 0.0, 0.0);
    let adaptive = AdaptiveStepConfig {
        max_attempts: 4,
        ..pr_adaptive(1.0e-6)
    };
    let run = integrate_adaptive_dense_observed_with_segments(
        &problem,
        (0.0, 2.0),
        &y0,
        IntegrationMethod::Sequential,
        None,
        None,
        &adaptive,
        &OutputSamplingPlan::dense(span_schedule(0.0, 2.0)),
        DenseErrorControl::WFilteredEnforce,
        InteriorSampling::EveryStep,
    )
    .unwrap();
    assert!(!run.result.observed.success);
    let partial = run.partial.unwrap();
    assert_eq!(partial.reason, DriverStopReason::AttemptBudget);
    let last = run.segments.last().unwrap();
    assert_eq!(partial.t, last.t_new);
    assert_eq!(bits(&partial.y), bits(&last.y_new));
}
