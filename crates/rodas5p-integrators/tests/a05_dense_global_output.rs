//! A05 contracts that can be exercised without an interval-residual witness:
//! failed callback work is charged, and zero-span entry still validates all
//! public adaptive/linear configuration.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::directed::Interval;
use rodas5p_core::{CoreError, DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, ContinuousResidualProvider, ContinuousResidualVerificationError,
    ContinuousResidualWorkspace, DenseErrorControl, DenseOutputError, DenseOutputResult,
    DenseSegmentWork, IntegrationMethod, OdeProblem, OutputSamplingPlan, OutputSchedule,
    Rodas5pQuarticSegment, StepResult, integrate_adaptive_dense_observed_with_dense_error_control,
    prothero_robinson_problem, rodas5p_dense_output,
};

type TestBatchRhs =
    Arc<dyn Fn(&[f64], &[Vec<f64>]) -> Result<Vec<Vec<f64>>, CoreError> + Send + Sync>;

fn problem_with_rhs(
    rhs: impl Fn(f64, &[f64], &mut [f64]) -> Result<(), CoreError> + Send + Sync + 'static,
) -> OdeProblem {
    OdeProblem::new(
        "a05-rhs-counter",
        1,
        Arc::new(rhs),
        None,
        Some(Arc::new(|_, _| Ok(DenseMatrix::zeros(1, 1)))),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn problem_with_counted_rhs() -> (OdeProblem, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = Arc::clone(&calls);
    let problem = problem_with_rhs(move |_, _, out| {
        callback_calls.fetch_add(1, Ordering::SeqCst);
        out[0] = 1.0;
        Ok(())
    });
    (problem, calls)
}

#[test]
fn scalar_rhs_attempts_are_counted_on_err_and_nonfinite_output() {
    let err_problem = problem_with_rhs(|_, _, _| {
        Err(CoreError::InvalidInput("synthetic callback failure".into()))
    });
    let mut counters = WorkCounters::default();
    assert!(err_problem.eval_rhs(0.0, &[1.0], &mut counters).is_err());
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (1, 1));

    let nan_problem = problem_with_rhs(|_, _, out| {
        out[0] = f64::NAN;
        Ok(())
    });
    let mut counters = WorkCounters::default();
    assert!(nan_problem.eval_rhs(0.0, &[1.0], &mut counters).is_err());
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (1, 1));

    let mut out = [0.0];
    let mut counters = WorkCounters::default();
    assert!(
        nan_problem
            .eval_rhs_into(0.0, &[1.0], &mut out, &mut counters)
            .is_err()
    );
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (1, 1));
}

#[test]
fn invalid_scalar_rhs_shapes_do_not_charge_or_enter_callbacks() {
    let (problem, calls) = problem_with_counted_rhs();
    let mut counters = WorkCounters::default();
    assert!(problem.eval_rhs(0.0, &[], &mut counters).is_err());
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (0, 0));
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let mut out = [];
    assert!(
        problem
            .eval_rhs_into(0.0, &[1.0], &mut out, &mut counters)
            .is_err()
    );
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (0, 0));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

fn batch_problem(batch: Option<TestBatchRhs>) -> OdeProblem {
    OdeProblem::new(
        "a05-batch-counter",
        1,
        Arc::new(|_, _, out| {
            out[0] = 2.0;
            Ok(())
        }),
        batch,
        Some(Arc::new(|_, _| Ok(DenseMatrix::zeros(1, 1)))),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn batch_callback_failure_success_and_scalar_fallback_are_accounted() {
    let failed_calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = Arc::clone(&failed_calls);
    let failed = batch_problem(Some(Arc::new(move |_, _| {
        callback_calls.fetch_add(1, Ordering::SeqCst);
        Err(CoreError::InvalidInput("synthetic batch failure".into()))
    })));
    let times = [0.0, 0.5];
    let states = vec![vec![1.0], vec![2.0]];
    let mut counters = WorkCounters::default();
    assert!(
        failed
            .eval_rhs_batch(&times, &[vec![1.0], vec![]], &mut counters)
            .is_err()
    );
    assert_eq!((counters.rhs_batch_calls, counters.rhs_evaluations), (0, 0));
    assert_eq!(failed_calls.load(Ordering::SeqCst), 0);
    assert!(
        failed
            .eval_rhs_batch(&times, &states, &mut counters)
            .is_err()
    );
    assert_eq!((counters.rhs_batch_calls, counters.rhs_evaluations), (1, 2));
    assert_eq!(failed_calls.load(Ordering::SeqCst), 1);

    let successful_calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = Arc::clone(&successful_calls);
    let successful = batch_problem(Some(Arc::new(move |_, states| {
        callback_calls.fetch_add(1, Ordering::SeqCst);
        Ok(states.iter().map(|_| vec![3.0]).collect())
    })));
    let mut counters = WorkCounters::default();
    let values = successful
        .eval_rhs_batch(&times, &states, &mut counters)
        .unwrap();
    assert_eq!(values, vec![vec![3.0], vec![3.0]]);
    assert_eq!((counters.rhs_batch_calls, counters.rhs_evaluations), (1, 2));
    assert_eq!(successful_calls.load(Ordering::SeqCst), 1);

    let fallback = batch_problem(None);
    let mut counters = WorkCounters::default();
    let values = fallback
        .eval_rhs_batch(&times, &states, &mut counters)
        .unwrap();
    assert_eq!(values, vec![vec![2.0], vec![2.0]]);
    assert_eq!(
        (
            counters.rhs_batch_calls,
            counters.rhs_calls,
            counters.rhs_evaluations
        ),
        (0, 2, 2)
    );

    let fallback_error = problem_with_rhs(|_, _, _| {
        Err(CoreError::InvalidInput(
            "synthetic scalar fallback failure".into(),
        ))
    });
    let mut counters = WorkCounters::default();
    assert!(
        fallback_error
            .eval_rhs_batch(&times, &states, &mut counters)
            .is_err()
    );
    assert_eq!(
        (
            counters.rhs_batch_calls,
            counters.rhs_calls,
            counters.rhs_evaluations
        ),
        (0, 1, 1)
    );
}

#[test]
fn zero_span_dense_entry_rejects_invalid_adaptive_and_linear_config() {
    let (problem, y0) = prothero_robinson_problem(-10.0, 0.0, 0.0);
    let sampling = OutputSamplingPlan::dense(OutputSchedule::new(vec![0.0]).unwrap());
    let invalid_adaptive = AdaptiveStepConfig {
        rtol: f64::NAN,
        ..AdaptiveStepConfig::default()
    };
    let linear = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    assert!(
        integrate_adaptive_dense_observed_with_dense_error_control(
            &problem,
            (0.0, 0.0),
            &y0,
            IntegrationMethod::Sequential,
            Some(&linear),
            None,
            &invalid_adaptive,
            &sampling,
            DenseErrorControl::Off,
        )
        .is_err()
    );

    let invalid_linear = LinearSolverConfig {
        rtol: f64::INFINITY,
        ..linear
    };
    assert!(
        integrate_adaptive_dense_observed_with_dense_error_control(
            &problem,
            (0.0, 0.0),
            &y0,
            IntegrationMethod::Sequential,
            Some(&invalid_linear),
            None,
            &AdaptiveStepConfig::default(),
            &sampling,
            DenseErrorControl::Off,
        )
        .is_err()
    );
}

fn dense_step(t_old: f64, t_new: f64, h: f64) -> StepResult {
    StepResult {
        t_old,
        t_new,
        y_old: vec![0.25],
        y_new: vec![0.75],
        h,
        // A nonzero transformed-stage row makes the stored continuous
        // extension genuinely quartic when the dense table has d2 != 0.
        stages: vec![
            vec![1.0],
            vec![0.0],
            vec![0.0],
            vec![0.0],
            vec![0.0],
            vec![0.0],
            vec![0.0],
            vec![0.0],
        ],
        error_vector: vec![0.0],
        error_norm: 0.0,
        accepted: true,
        method: "a05-dense-segment".into(),
        used_fallback: false,
        certificate: None,
        counters: WorkCounters::default(),
    }
}

#[test]
fn immutable_quartic_segment_encloses_its_endpoint_and_interior_centers() {
    let step = dense_step(1.0, 2.0, 1.0);
    let mut work = DenseSegmentWork::default();
    let segment = Rodas5pQuarticSegment::from_step(&step, &mut work).unwrap();
    let mut center = [0.0];
    let mut value = [Interval::point(0.0).unwrap()];
    let mut derivative = [Interval::point(0.0).unwrap()];

    let observations: [(f64, f64); 3] = [(1.0, 0.25), (1.5, 0.0), (2.0, 0.75)];
    for (time, expected) in observations {
        segment
            .evaluate_center_at_into(time, &mut center, &mut work)
            .unwrap();
        segment
            .evaluate_value_at_into(time, &mut value, &mut work)
            .unwrap();
        segment
            .evaluate_physical_derivative_at_into(time, &mut derivative, &mut work)
            .unwrap();
        assert!(
            value[0].contains(center[0]),
            "time={time}, center={:?}, interval={:?}",
            center[0],
            value[0]
        );
        assert!(derivative[0].lo <= derivative[0].hi);
        if time != 1.5 {
            assert_eq!(center[0].to_bits(), expected.to_bits());
        } else {
            // A point RHS that happened to return f=0 at this midpoint would
            // miss this nonzero curve derivative. The segment exposes the
            // derivative enclosure, while residual authority remains typed
            // unavailable without an interval provider.
            assert!(!derivative[0].contains_zero());
        }
    }
    assert!(work.construction_items > 0);
    assert!(work.identity_comparison_items > 0);
    assert!(work.normalization_operations > 0);
    assert!(work.interval_value_operations > 0);
    assert!(work.interval_derivative_operations > 0);
}

#[test]
fn dense_segment_arithmetic_counters_match_attempted_scalar_operations() {
    const N: usize = 3;
    let mut step = dense_step(0.0, 1.0, 1.0);
    step.y_old = vec![0.25, -0.5, 0.75];
    step.y_new = vec![0.75, 0.25, -0.125];
    step.error_vector = vec![0.0; N];
    for row in &mut step.stages {
        row.resize(N, 0.0);
    }
    let mut construction = DenseSegmentWork::default();
    let segment = Rodas5pQuarticSegment::from_step(&step, &mut construction).unwrap();

    let mut centers = [0.0; N];
    let mut center_work = DenseSegmentWork::default();
    segment
        .evaluate_center_at_into(0.5, &mut centers, &mut center_work)
        .unwrap();
    assert_eq!(center_work.center_operations, 1 + 9 * N as u64);
    let center_before_endpoint = center_work.center_operations;
    segment
        .evaluate_center_at_into(0.0, &mut centers, &mut center_work)
        .unwrap();
    assert_eq!(center_work.center_operations, center_before_endpoint);

    let mut values = [Interval::point(0.0).unwrap(); N];
    let mut value_work = DenseSegmentWork::default();
    segment
        .evaluate_value_at_into(0.5, &mut values, &mut value_work)
        .unwrap();
    assert_eq!(value_work.interval_value_operations, 1 + 9 * N as u64);
    let value_before_endpoint = value_work.interval_value_operations;
    segment
        .evaluate_value_at_into(0.0, &mut values, &mut value_work)
        .unwrap();
    assert_eq!(value_work.interval_value_operations, value_before_endpoint);

    let mut derivatives = [Interval::point(0.0).unwrap(); N];
    let mut derivative_work = DenseSegmentWork::default();
    segment
        .evaluate_physical_derivative_at_into(0.5, &mut derivatives, &mut derivative_work)
        .unwrap();
    assert_eq!(
        derivative_work.interval_derivative_operations,
        2 + 14 * N as u64
    );
    let derivative_before_endpoint = derivative_work.interval_derivative_operations;
    segment
        .evaluate_physical_derivative_at_into(0.0, &mut derivatives, &mut derivative_work)
        .unwrap();
    assert_eq!(
        derivative_work.interval_derivative_operations - derivative_before_endpoint,
        2 + 14 * N as u64
    );
}

#[test]
fn represented_clock_delta_controls_normalization_not_step_metadata() {
    let t_old = 2_f64.powi(53);
    let step = dense_step(t_old, t_old + 3.0, 3.0);
    assert_eq!(
        step.t_new - step.t_old,
        4.0,
        "represented span differs from h"
    );
    let mut work = DenseSegmentWork::default();
    let segment = Rodas5pQuarticSegment::from_step(&step, &mut work).unwrap();
    let mut center = [0.0];
    let mut value = [Interval::point(0.0).unwrap()];
    segment
        .evaluate_center_at_into(t_old + 2.0, &mut center, &mut work)
        .unwrap();
    segment
        .evaluate_value_at_into(t_old + 2.0, &mut value, &mut work)
        .unwrap();
    assert!(value[0].contains(center[0]));
}

#[test]
fn immutable_segment_center_matches_the_existing_public_dense_convention() {
    let step = dense_step(0.0, 1.0, 1.0);
    let mut work = DenseSegmentWork::default();
    let segment = Rodas5pQuarticSegment::from_step(&step, &mut work).unwrap();
    let mut center = [0.0];
    let mut interval = [Interval::point(0.0).unwrap()];
    for theta in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let existing = rodas5p_dense_output(&step, theta).unwrap();
        segment
            .evaluate_center_at_into(theta, &mut center, &mut work)
            .unwrap();
        segment
            .evaluate_value_at_into(theta, &mut interval, &mut work)
            .unwrap();
        assert_eq!(center[0].to_bits(), existing[0].to_bits(), "theta={theta}");
        assert!(interval[0].contains(existing[0]), "theta={theta}");
    }
}

#[test]
fn dense_segment_refuses_bad_spans_and_unavailable_residual_provider() {
    let mut work = DenseSegmentWork::default();
    let nonfinite = dense_step(f64::NAN, 1.0, 1.0);
    assert!(matches!(
        Rodas5pQuarticSegment::from_step(&nonfinite, &mut work),
        Err(DenseOutputError::InvalidSegment(_))
    ));
    let subnormal = dense_step(0.0, f64::from_bits(1), f64::from_bits(1));
    assert!(matches!(
        Rodas5pQuarticSegment::from_step(&subnormal, &mut work),
        Err(DenseOutputError::InvalidSegment(_))
    ));
    // The exact real span is 1 + 2^-54, but ordinary subtraction rounds it
    // to 1. Refuse rather than let center and interval paths define curves
    // with different normalizations.
    let inexact_delta = dense_step(-2_f64.powi(-54), 1.0, 1.0);
    assert!(matches!(
        Rodas5pQuarticSegment::from_step(&inexact_delta, &mut work),
        Err(DenseOutputError::InvalidSegment(_))
    ));

    let segment = Rodas5pQuarticSegment::from_step(&dense_step(0.0, 1.0, 1.0), &mut work).unwrap();
    let mut value = [Interval::point(0.0).unwrap()];
    assert!(matches!(
        segment.evaluate_value_at_into(f64::NAN, &mut value, &mut work),
        Err(DenseOutputError::InvalidSegment(_))
    ));
    assert!(matches!(
        segment.request_continuous_residual_provider(&mut work),
        Err(DenseOutputError::ContinuousResidualProviderUnavailable)
    ));
    assert!(work.provider_unavailable_requests > 0);
    assert!(work.total_operations() > 0);
}

struct ConstantIntervalField(f64);

impl ContinuousResidualProvider for ConstantIntervalField {
    fn enclose_rhs(
        &self,
        _time: Interval,
        _state: &[Interval],
        rhs: &mut [Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<()> {
        work.charge_provider_operations(rhs.len() as u64);
        rhs.fill(Interval::point(0.0)?);
        Ok(())
    }

    fn one_sided_log_norm_upper(
        &self,
        _time: Interval,
        tube: &[Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<f64> {
        work.charge_provider_operations(tube.len() as u64);
        Ok(self.0)
    }
}

struct AffineIntervalField {
    forcing: f64,
    mu: f64,
}

impl ContinuousResidualProvider for AffineIntervalField {
    fn enclose_rhs(
        &self,
        _time: Interval,
        state: &[Interval],
        rhs: &mut [Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<()> {
        assert_eq!(state.len(), rhs.len());
        for (state_i, rhs_i) in state.iter().zip(rhs.iter_mut()) {
            work.charge_provider_operations(2);
            *rhs_i = Interval::point(self.mu)?
                .mul(*state_i)?
                .add(Interval::point(self.forcing)?)?;
        }
        Ok(())
    }

    fn one_sided_log_norm_upper(
        &self,
        _time: Interval,
        tube: &[Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<f64> {
        work.charge_provider_operations(tube.len() as u64);
        Ok(self.mu)
    }
}

struct RefusingIntervalField;

impl ContinuousResidualProvider for RefusingIntervalField {
    fn enclose_rhs(
        &self,
        _time: Interval,
        _state: &[Interval],
        _rhs: &mut [Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<()> {
        work.charge_provider_operations(7);
        Err(CoreError::InvalidInput("synthetic interval RHS refusal".into()).into())
    }

    fn one_sided_log_norm_upper(
        &self,
        _time: Interval,
        _tube: &[Interval],
        _work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<f64> {
        Ok(0.0)
    }
}

#[test]
fn continuous_segment_residual_checks_interval_curve_and_fixed_tube() {
    let mut step = dense_step(0.0, 1.0, 1.0);
    step.y_old[0] = 0.0;
    step.y_new[0] = 1.0;
    step.stages = vec![vec![0.0]; 8];
    let mut work = DenseSegmentWork::default();
    let segment = Rodas5pQuarticSegment::from_step(&step, &mut work).unwrap();
    let mut workspace = ContinuousResidualWorkspace::new(1, &mut work).unwrap();

    let mut curve = [Interval::point(0.0).unwrap()];
    segment
        .enclose_value_on_interval(0.0, 1.0, &mut curve, &mut work)
        .unwrap();
    assert!(curve[0].contains(0.25));
    assert!(curve[0].contains(0.5));

    let bound = segment
        .verify_continuous_residual_tube(
            &ConstantIntervalField(0.0),
            0.125,
            2.0,
            &mut workspace,
            &mut work,
        )
        .unwrap();
    assert!(bound.residual_l2_upper >= 1.0);
    assert!(bound.flow_gain_upper >= 1.0);
    assert!(bound.transported_initial_radius_upper >= 0.125);
    assert!(bound.residual_transport_upper > 0.0);
    assert!(bound.error_radius_upper <= bound.tube_radius);
    assert_eq!(work.interval_rhs_requests, 1);
    assert_eq!(work.one_sided_log_norm_requests, 1);
    assert!(work.provider_operations >= 2);

    let refusal = segment.verify_continuous_residual_tube(
        &ConstantIntervalField(0.0),
        0.0,
        0.0,
        &mut workspace,
        &mut work,
    );
    assert!(matches!(
        refusal,
        Err(ContinuousResidualVerificationError::TubeNotVerified { .. })
    ));
    assert_eq!(work.interval_rhs_requests, 2);
    assert_eq!(work.one_sided_log_norm_requests, 2);
    assert!(work.residual_operations > 0 && work.gain_operations > 0);
}

#[test]
fn signed_one_sided_gain_uses_outward_residual_integral_and_refuses_overflow() {
    let mut step = dense_step(0.0, 1.0, 1.0);
    step.y_old[0] = 0.0;
    step.y_new[0] = 0.0;
    step.stages = vec![vec![0.0]; 8];
    let mut work = DenseSegmentWork::default();
    let segment = Rodas5pQuarticSegment::from_step(&step, &mut work).unwrap();
    let mut workspace = ContinuousResidualWorkspace::new(1, &mut work).unwrap();

    let before_zero_gain = work.gain_operations;
    let zero_rate = segment
        .verify_continuous_residual_tube(
            &AffineIntervalField {
                forcing: 1.0,
                mu: 0.0,
            },
            0.25,
            2.0,
            &mut workspace,
            &mut work,
        )
        .unwrap();
    assert_eq!(work.gain_operations - before_zero_gain, 3);
    assert!(zero_rate.residual_l2_upper >= 1.0);

    let before_contracting_gain = work.gain_operations;
    let contracting = segment
        .verify_continuous_residual_tube(
            &AffineIntervalField {
                forcing: 1.0,
                mu: -1.0,
            },
            0.25,
            2.0,
            &mut workspace,
            &mut work,
        )
        .unwrap();
    assert_eq!(work.gain_operations - before_contracting_gain, 8);
    assert!(contracting.flow_gain_upper < 1.0);
    assert!(contracting.residual_transport_upper > 0.6);
    assert!(contracting.residual_transport_upper < 0.7);

    let before_expanding_gain = work.gain_operations;
    let expanding = segment
        .verify_continuous_residual_tube(
            &AffineIntervalField {
                forcing: 1.0,
                mu: 1.0,
            },
            0.25,
            10.0,
            &mut workspace,
            &mut work,
        )
        .unwrap();
    assert_eq!(work.gain_operations - before_expanding_gain, 8);
    assert!(expanding.flow_gain_upper > 2.7);
    assert!(expanding.residual_transport_upper > 1.7);

    let subnormal_rate = segment
        .verify_continuous_residual_tube(
            &AffineIntervalField {
                forcing: 1.0,
                mu: -f64::from_bits(1),
            },
            0.0,
            10.0,
            &mut workspace,
            &mut work,
        )
        .unwrap();
    assert!(subnormal_rate.flow_gain_upper.is_finite());
    assert!(subnormal_rate.residual_transport_upper.is_finite());
    assert!(subnormal_rate.residual_transport_upper > 0.0);

    let subnormal_expansion = segment
        .verify_continuous_residual_tube(
            &AffineIntervalField {
                forcing: 1.0,
                mu: f64::from_bits(1),
            },
            0.0,
            10.0,
            &mut workspace,
            &mut work,
        )
        .unwrap();
    assert!(subnormal_expansion.flow_gain_upper.is_finite());
    assert!(subnormal_expansion.residual_transport_upper.is_finite());
    assert!(subnormal_expansion.residual_transport_upper > 0.0);

    let before_gain_work = work.gain_operations;
    let overflow_refusal = segment.verify_continuous_residual_tube(
        &AffineIntervalField {
            forcing: 1.0,
            mu: 710.0,
        },
        0.0,
        1.0,
        &mut workspace,
        &mut work,
    );
    assert!(
        matches!(
            overflow_refusal,
            Err(ContinuousResidualVerificationError::Core(
                CoreError::NonFinite(_)
            ))
        ),
        "unexpected overflow refusal: {overflow_refusal:?}"
    );
    assert_eq!(work.gain_operations - before_gain_work, 2);
}

#[test]
fn nonfinite_log_norm_and_workspace_capacity_refusals_keep_attempted_work() {
    let mut step = dense_step(0.0, 1.0, 1.0);
    step.y_old[0] = 0.0;
    step.y_new[0] = 0.0;
    step.stages = vec![vec![0.0]; 8];
    let mut work = DenseSegmentWork::default();
    let segment = Rodas5pQuarticSegment::from_step(&step, &mut work).unwrap();
    let mut workspace = ContinuousResidualWorkspace::new(1, &mut work).unwrap();

    assert!(matches!(
        segment.verify_continuous_residual_tube(
            &ConstantIntervalField(f64::NAN),
            0.0,
            1.0,
            &mut workspace,
            &mut work,
        ),
        Err(ContinuousResidualVerificationError::NonFiniteLogNorm)
    ));
    assert_eq!(work.one_sided_log_norm_requests, 1);
    assert!(work.provider_operations > 0);

    let before = work.workspace_items;
    assert!(ContinuousResidualWorkspace::new(usize::MAX, &mut work).is_err());
    assert!(work.workspace_items > before);
}

#[test]
fn continuous_provider_refusal_preserves_attempted_work() {
    let mut step = dense_step(0.0, 1.0, 1.0);
    step.y_old[0] = 0.0;
    step.y_new[0] = 1.0;
    step.stages = vec![vec![0.0]; 8];
    let mut work = DenseSegmentWork::default();
    let segment = Rodas5pQuarticSegment::from_step(&step, &mut work).unwrap();
    let mut workspace = ContinuousResidualWorkspace::new(1, &mut work).unwrap();

    assert!(
        segment
            .verify_continuous_residual_tube(
                &RefusingIntervalField,
                0.0,
                1.0,
                &mut workspace,
                &mut work,
            )
            .is_err()
    );
    assert_eq!(work.interval_rhs_requests, 1);
    assert_eq!(work.one_sided_log_norm_requests, 0);
    assert_eq!(work.provider_operations, 7);
}

#[test]
fn derivative_refusal_charges_attempted_interval_work_before_overflow() {
    let mut step = dense_step(0.0, 1.0, 1.0);
    step.y_old[0] = f64::MAX;
    step.y_new[0] = -f64::MAX;
    step.stages = vec![vec![0.0]; 8];
    let mut work = DenseSegmentWork::default();
    let segment = Rodas5pQuarticSegment::from_step(&step, &mut work).unwrap();
    let before = work.interval_derivative_operations;
    let mut output = [Interval::point(0.0).unwrap()];
    assert!(
        segment
            .evaluate_physical_derivative_at_into(0.5, &mut output, &mut work)
            .is_err()
    );
    assert!(work.interval_derivative_operations > before);
}

#[test]
fn construction_refusal_charges_attempted_span_work_before_exact_sum_overflow() {
    let step = dense_step(-f64::MAX, f64::MAX, 1.0);
    let mut work = DenseSegmentWork::default();
    assert!(Rodas5pQuarticSegment::from_step(&step, &mut work).is_err());
    assert!(work.construction_items > 0);
}
