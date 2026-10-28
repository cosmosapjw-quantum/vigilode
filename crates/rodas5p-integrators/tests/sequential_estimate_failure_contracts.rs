//! Sequential dense enforcement must fail closed if its midpoint check fails.
use rodas5p_core::{CoreError, DenseMatrix, LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveDenseSegmentRun, AdaptiveStepConfig, DenseErrorControl, IntegrationMethod,
    InteriorSampling, OdeProblem, OutputSamplingPlan, OutputSchedule,
    integrate_adaptive_dense_observed_with_segments,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn decay(fail_at: Option<f64>, fatal: bool) -> OdeProblem {
    let fired = Arc::new(AtomicBool::new(false));
    OdeProblem::new(
        "midpoint-failure-decay",
        1,
        Arc::new(move |t, y, out| {
            if fail_at.is_some_and(|x| x.to_bits() == t.to_bits())
                && !fired.swap(true, Ordering::Relaxed)
            {
                return Err(if fatal {
                    CoreError::InvalidInput("injected midpoint error".into())
                } else {
                    CoreError::NonFinite("injected midpoint error".into())
                });
            }
            out[0] = -y[0];
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| DenseMatrix::new(1, 1, vec![-1.0]))),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn run(
    problem: &OdeProblem,
    control: DenseErrorControl,
) -> Result<AdaptiveDenseSegmentRun, rodas5p_integrators::DenseOutputError> {
    integrate_adaptive_dense_observed_with_segments(
        problem,
        (0.0, 2.0),
        &[1.0],
        IntegrationMethod::Sequential,
        Some(&LinearSolverConfig {
            method: LinearMethod::Direct,
            ..Default::default()
        }),
        None,
        &AdaptiveStepConfig {
            initial_step: 0.1,
            atol: 1e-8,
            rtol: 1e-6,
            ..Default::default()
        },
        &OutputSamplingPlan::dense(OutputSchedule::new(vec![0.0, 0.05, 0.5, 1.0, 2.0]).unwrap()),
        control,
        InteriorSampling::EveryStep,
    )
}

#[test]
fn estimate_failure_rejects_under_enforcing_modes() {
    for control in [
        DenseErrorControl::Enforce,
        DenseErrorControl::WFilteredEnforce,
    ] {
        let clean = run(&decay(None, false), control).unwrap();
        let first = &clean.segments[0];
        let midpoint = first.t_old + 0.5 * (first.t_new - first.t_old);
        let failed = run(&decay(Some(midpoint), false), control).unwrap();
        assert!(failed.result.observed.success);
        assert_eq!(
            failed.result.observed.counters.nonfinite_step_failures,
            clean.result.observed.counters.nonfinite_step_failures + 1
        );
        assert_eq!(
            failed.result.observed.counters.rejected_steps,
            clean.result.observed.counters.rejected_steps + 1
        );
        assert!(
            failed.segments[0].t_new < first.t_new,
            "retry must reduce the first step"
        );
        assert_eq!(failed.segments[0].t_old, 0.0);
        assert_eq!(failed.report.unavailable, 0);
        assert_eq!(failed.report.estimate_failures, 1);
        for segment in &failed.segments {
            assert_eq!(segment.evaluate(segment.t_new).unwrap(), segment.y_new);
        }
        assert_eq!(
            failed.result.observed.counters.accepted_steps as usize,
            failed.segments.len()
        );
        assert_eq!(
            failed.result.diagnostics.attempts as u64,
            failed.result.observed.counters.accepted_steps
                + failed.result.observed.counters.rejected_steps
        );
        assert!(
            failed.result.observed.counters.rhs_evaluations
                > clean.result.observed.counters.rhs_evaluations,
            "failed midpoint and all rejected-stage work remain charged"
        );
    }
}

#[test]
fn unclassified_estimate_error_stops_the_run() {
    for control in [
        DenseErrorControl::Enforce,
        DenseErrorControl::WFilteredEnforce,
    ] {
        let clean = run(&decay(None, false), control).unwrap();
        let first = &clean.segments[0];
        let midpoint = first.t_old + 0.5 * (first.t_new - first.t_old);
        assert!(matches!(
            run(&decay(Some(midpoint), true), control),
            Err(rodas5p_integrators::DenseOutputError::Core(
                CoreError::InvalidInput(_)
            ))
        ));
    }
}

#[test]
fn report_modes_still_count_unavailable_without_changing_the_run() {
    for control in [
        DenseErrorControl::Report,
        DenseErrorControl::ReportDefect,
        DenseErrorControl::WFilteredReport,
    ] {
        let clean = run(&decay(None, false), control).unwrap();
        let first = &clean.segments[0];
        let midpoint = first.t_old + 0.5 * (first.t_new - first.t_old);
        let failed = run(&decay(Some(midpoint), false), control).unwrap();
        assert_eq!(failed.report.unavailable, 1);
        assert_eq!(failed.report.estimate_failures, 0);
        assert_eq!(failed.result.observed.t, clean.result.observed.t);
        assert_eq!(failed.result.observed.y, clean.result.observed.y);
        assert_eq!(
            failed.result.observed.counters,
            clean.result.observed.counters
        );
        assert_eq!(
            failed.result.observed.success,
            clean.result.observed.success
        );
        assert_eq!(failed.result.diagnostics, clean.result.diagnostics);
        assert_eq!(failed.segments, clean.segments);
    }
}

#[test]
fn runs_without_estimate_errors_are_unchanged() {
    for (name, problem, y0, tf) in [
        {
            let (p, y) = rodas5p_integrators::prothero_robinson_problem(-1e5, 0.0, 0.0);
            ("pr", p, y, 2.0)
        },
        {
            let (p, y) = rodas5p_integrators::robertson_problem().unwrap();
            ("robertson", p, y, 40.0)
        },
    ] {
        for control in [
            DenseErrorControl::Enforce,
            DenseErrorControl::WFilteredEnforce,
        ] {
            let result = integrate_adaptive_dense_observed_with_segments(
                &problem,
                (0.0, tf),
                &y0,
                IntegrationMethod::Sequential,
                Some(&LinearSolverConfig {
                    method: LinearMethod::Direct,
                    ..Default::default()
                }),
                None,
                &AdaptiveStepConfig {
                    initial_step: 1e-3,
                    atol: 1e-8,
                    rtol: 1e-6,
                    ..Default::default()
                },
                &OutputSamplingPlan::dense(OutputSchedule::new(vec![0.0, tf]).unwrap()),
                control,
                InteriorSampling::EveryStep,
            )
            .unwrap();
            let bits: Vec<_> = result
                .result
                .observed
                .y
                .last()
                .unwrap()
                .iter()
                .map(|x| x.to_bits())
                .collect();
            eprintln!(
                "PIN {name} {control:?}: bits={bits:?} attempts={} samples={} rejected={}",
                result.result.diagnostics.attempts,
                result.report.samples.len(),
                result.result.observed.counters.rejected_steps
            );
            let expected = match (name, control) {
                ("pr", DenseErrorControl::Enforce) => (vec![4606365442643703262], 26, 26, 14),
                ("pr", DenseErrorControl::WFilteredEnforce) => {
                    (vec![4606365442650370772], 15, 15, 0)
                }
                ("robertson", DenseErrorControl::Enforce) => (
                    vec![
                        4604622811793021475,
                        4531539797996114594,
                        4598790668460325250,
                    ],
                    32,
                    30,
                    2,
                ),
                ("robertson", DenseErrorControl::WFilteredEnforce) => (
                    vec![
                        4604622814148447066,
                        4531539802224500844,
                        4598790663749345026,
                    ],
                    33,
                    31,
                    2,
                ),
                _ => unreachable!(),
            };
            assert_eq!(
                (
                    bits,
                    result.result.diagnostics.attempts,
                    result.report.samples.len(),
                    result.result.observed.counters.rejected_steps
                ),
                expected
            );
            assert_eq!(result.report.estimate_failures, 0);
            assert!(result.result.observed.success);
        }
    }
}
