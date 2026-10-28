use rodas5p_integrators::{
    AdaptiveStepConfig, CollectiveModel, DenseErrorControl, DenseOutputError, DenseSegmentWork,
    IntegrationMethod, OutputSamplingPlan, OutputSchedule, Pr101Model, VerifiedContinuousProblem,
    integrate_adaptive_dense_observed_with_dense_error_control, integrate_verified_residual,
    prothero_robinson_problem,
};

fn config() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-11,
        rtol: 1.0e-8,
        initial_step: 0.1,
        max_step: 0.1,
        max_attempts: 20000,
        ..AdaptiveStepConfig::default()
    }
}

#[test]
fn generic_pr101_original_101_point_witness_and_continuous_inheritance() {
    let model = Pr101Model::new(-1.0e6, 10.0).unwrap();
    let owner = VerifiedContinuousProblem::from_pr101(model).unwrap();
    let times: Vec<_> = (0..=100).map(|k| k as f64 / 100.0).collect();
    let cfg = config();
    let run = integrate_verified_residual(&owner, (0.0, 1.0), &[0.0], 0.0, &times, &cfg).unwrap();
    assert!(run.success, "{:?}", run.failure);
    assert_eq!(run.outputs().len(), 101);
    assert!(run.rejected_attempts > 0);
    let mut max_error = 0.0_f64;
    for output in run.outputs() {
        let error = (output.center()[0] - output.time().sin()).abs();
        max_error = max_error.max(error);
        assert!(output.radius_l2() <= cfg.atol);
        assert!(error <= output.radius_l2());
    }
    let mut work = DenseSegmentWork::default();
    for (i, segment) in run.segments().iter().enumerate() {
        let d = segment.diagnostics();
        assert!(d.uniform_radius <= cfg.atol * 0.5);
        if i > 0 {
            let previous = run.segments()[i - 1].diagnostics();
            assert_eq!(previous.span.1, d.span.0);
            assert_eq!(previous.endpoint_radius, d.initial_radius);
        }
        // Additional off-request checks probe the admitted continuous theorem.
        let midpoint = d.span.0 + (d.span.1 - d.span.0) * 0.5;
        let value = segment.evaluate(midpoint, &mut work).unwrap();
        assert!((value.center()[0] - midpoint.sin()).abs() <= value.radius_l2());
    }
    assert_eq!(
        run.solver_work.accepted_steps as usize,
        run.segments().len()
    );
    assert_eq!(
        run.solver_work.rejected_steps as usize,
        run.rejected_attempts
    );
    println!(
        "A05_GENERIC_PR101 outputs=101 accepted={} rejected={} rhs={} dense_ops={} max_error={max_error:e}",
        run.segments().len(),
        run.rejected_attempts,
        run.solver_work.rhs_evaluations,
        run.verification_work.total_operations()
    );
}

#[test]
fn one_failed_trial_exposes_only_initial_committed_output() {
    let owner =
        VerifiedContinuousProblem::from_pr101(Pr101Model::new(-1e6, 10.0).unwrap()).unwrap();
    let cfg = AdaptiveStepConfig {
        initial_step: 1.0,
        max_step: 1.0,
        max_attempts: 1,
        ..config()
    };
    let run = integrate_verified_residual(
        &owner,
        (0.0, 1.0),
        &[0.0],
        0.0,
        &[0.0, 0.25, 0.5, 1.0],
        &cfg,
    )
    .unwrap();
    assert!(!run.success);
    assert_eq!((run.attempts, run.rejected_attempts), (1, 1));
    assert!(run.segments().is_empty());
    assert_eq!(run.outputs().len(), 1);
    assert_eq!(run.outputs()[0].time(), 0.0);
    assert!(run.solver_work.rhs_evaluations > 0);
    assert!(run.verification_work.provider_operations > 0);
}

#[test]
fn collective_two_components_use_same_generic_verified_driver() {
    let model =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(1.0, 0.0)], 0.0, 0.0, 0.0, 1.0).unwrap();
    let owner =
        VerifiedContinuousProblem::from_collective(model.to_bound_continuous_problem().unwrap());
    let cfg = AdaptiveStepConfig {
        atol: 1e-3,
        rtol: 1e-6,
        initial_step: 0.0625,
        max_step: 0.0625,
        max_attempts: 20000,
        ..AdaptiveStepConfig::default()
    };
    let times = [0.0, 0.03125, 0.0625, 0.09375, 0.125];
    let run =
        integrate_verified_residual(&owner, (0.0, 0.125), &[1.0, 0.0], 0.0, &times, &cfg).unwrap();
    assert!(run.success, "{:?} {}", run.failure, run.attempts);
    for output in run.outputs() {
        let error = (output.center()[0] - (-output.time()).exp()).abs();
        assert!(error <= output.radius_l2());
        assert!(output.center()[1].abs() <= output.radius_l2());
        assert!(output.radius_l2() <= cfg.atol);
    }
    assert!(run.verification_work.interval_rhs_requests > 0);
    assert!(run.verification_work.one_sided_log_norm_requests > 0);
    println!(
        "A05_GENERIC_COLLECTIVE accepted={} rejected={} provider_ops={}",
        run.segments().len(),
        run.rejected_attempts,
        run.verification_work.provider_operations
    );
}

#[test]
fn generic_zero_span_still_validates_configuration_and_model_domain() {
    let owner =
        VerifiedContinuousProblem::from_pr101(Pr101Model::new(-10.0, 2.0).unwrap()).unwrap();
    let invalid = AdaptiveStepConfig {
        initial_step: f64::NAN,
        ..config()
    };
    assert!(
        integrate_verified_residual(&owner, (0.0, 0.0), &[0.0], 0.0, &[0.0], &invalid).is_err()
    );
    assert!(
        integrate_verified_residual(&owner, (-1.0, -1.0), &[0.0], 0.0, &[-1.0], &config()).is_err()
    );
    let pure_relative = AdaptiveStepConfig {
        atol: 0.0,
        ..config()
    };
    assert!(
        integrate_verified_residual(&owner, (0.0, 0.0), &[0.0], 0.0, &[0.0], &pure_relative)
            .is_err()
    );
    let run =
        integrate_verified_residual(&owner, (0.0, 0.0), &[0.0], 0.0, &[0.0], &config()).unwrap();
    assert!(run.success);
    assert_eq!(run.attempts, 0);
    assert_eq!(run.outputs().len(), 1);
}

#[test]
fn unbound_point_problem_cannot_request_verified_authority() {
    let (problem, _) = prothero_robinson_problem(-1e6, 1.0, 0.0);
    let sampling = OutputSamplingPlan::dense(OutputSchedule::new(vec![0.0, 1.0]).unwrap());
    let result = integrate_adaptive_dense_observed_with_dense_error_control(
        &problem,
        (0.0, 1.0),
        &[0.0],
        IntegrationMethod::Sequential,
        None,
        None,
        &config(),
        &sampling,
        DenseErrorControl::VerifiedResidual,
    );
    assert!(matches!(
        result,
        Err(DenseOutputError::UnsupportedUnboundModel)
    ));
}

#[test]
fn interval_power_form_retains_small_variation_on_large_offset() {
    use rodas5p_core::{WorkCounters, directed::Interval};
    use rodas5p_integrators::{Rodas5pQuarticSegment, StepResult};
    let step = StepResult {
        t_old: 0.0,
        t_new: 0.125,
        h: 0.125,
        y_old: vec![1e8],
        y_new: vec![1e8 - 0.125],
        stages: vec![vec![0.0]; 8],
        error_vector: vec![0.0],
        error_norm: 0.0,
        accepted: true,
        method: "exact-affine-fixture".into(),
        used_fallback: false,
        certificate: None,
        counters: WorkCounters::default(),
    };
    let mut work = DenseSegmentWork::default();
    let curve = Rodas5pQuarticSegment::from_step(&step, &mut work).unwrap();
    let mut value = [Interval::point(0.0).unwrap()];
    curve
        .enclose_value_on_interval(0.0, 0.125, &mut value, &mut work)
        .unwrap();
    assert!(value[0].lo <= 1e8 - 0.125 && value[0].hi >= 1e8);
    assert!(value[0].hi - value[0].lo < 0.126);
    curve
        .enclose_physical_derivative_on_interval(0.0, 0.125, &mut value, &mut work)
        .unwrap();
    assert!(value[0].contains(-1.0));
    assert!(value[0].hi - value[0].lo < 1e-12);
}

#[test]
fn generic_collective_cubic_orbit_is_enclosed() {
    let model =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(1.0, 0.0)], 0.0, 0.75, 0.0, 0.0).unwrap();
    let owner =
        VerifiedContinuousProblem::from_collective(model.to_bound_continuous_problem().unwrap());
    let cfg = AdaptiveStepConfig {
        atol: 1e-3,
        rtol: 1e-6,
        initial_step: 0.0625,
        max_step: 0.0625,
        max_attempts: 20000,
        ..AdaptiveStepConfig::default()
    };
    let times = [0.0, 0.03125, 0.0625, 0.09375, 0.125];
    let run =
        integrate_verified_residual(&owner, (0.0, 0.125), &[1.0, 0.0], 0.0, &times, &cfg).unwrap();
    assert!(run.success, "{:?}", run.failure);
    for output in run.outputs() {
        let phase = 0.75 * output.time();
        let error = (output.center()[0] - phase.cos()).hypot(output.center()[1] - phase.sin());
        assert!(error <= output.radius_l2());
        assert!(output.radius_l2() <= cfg.atol);
    }
}
