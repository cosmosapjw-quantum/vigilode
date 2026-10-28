//! Fresh tests for the reconstructed sealed PR101 provider and global driver.
use rodas5p_core::WorkCounters;
use rodas5p_integrators::{
    AdaptiveStepConfig, DenseSegmentWork, Pr101Model, Rodas5pQuarticSegment, StepResult,
};

fn linear_curve(h: f64) -> Rodas5pQuarticSegment {
    let step = StepResult {
        t_old: 0.0,
        t_new: h,
        h,
        y_old: vec![0.0],
        y_new: vec![h],
        stages: vec![vec![0.0]; 8],
        error_vector: vec![0.0],
        error_norm: 0.0,
        accepted: true,
        method: "independent-linear-fixture".into(),
        used_fallback: false,
        certificate: None,
        counters: WorkCounters::default(),
    };
    Rodas5pQuarticSegment::from_step(&step, &mut DenseSegmentWork::default()).unwrap()
}

#[test]
fn cancellation_preserving_defect_encloses_fraction_oracle_fixture() {
    let model = Pr101Model::new(-1.0e6, 10.0).unwrap();
    let h = 2.0_f64.powi(-10);
    let mut work = DenseSegmentWork::default();
    let verified = model
        .verify_segment(linear_curve(h), 0.0, 1.0e-9, &mut work)
        .unwrap();
    let bound = verified.bounds();
    // 78 trig-at + 53 shifted trig + 13 difference + 27 difference bound
    // + 47 derivative/linear + 351 convolution + 50 coefficient norm
    // + 8 tails + 4 log norm + 8 contracting transport.
    assert_eq!(work.provider_operations, 639);
    assert!(bound.uniform_radius < 2.0e-10, "{:?}", bound);
    for k in 0..=16 {
        let t = h * k as f64 / 16.0;
        let output = verified.evaluate(t, &mut work).unwrap();
        assert!((output.center() - t.sin()).abs() <= output.radius());
    }
    println!(
        "PR101_ORACLE={{\"h\":{h:.17e},\"residual\":{:.17e},\"radius\":{:.17e},\"lambda\":-1000000,\"eta\":10}}",
        bound.residual_upper, bound.uniform_radius
    );
    assert!(work.provider_operations > 500);
}

#[test]
fn nonlinear_pr101_101_outputs_satisfy_original_absolute_tolerance() {
    let model = Pr101Model::new(-1.0e6, 10.0).unwrap();
    let times: Vec<_> = (0..=100).map(|k| k as f64 / 100.0).collect();
    let config = AdaptiveStepConfig {
        atol: 1.0e-11,
        rtol: 1.0e-8,
        initial_step: 0.1,
        max_step: 0.1,
        max_attempts: 20000,
        ..AdaptiveStepConfig::default()
    };
    let run = model
        .integrate_verified((0.0, 1.0), 0.0, 0.0, &times, &config)
        .unwrap();
    assert!(
        run.success,
        "failure={:?} attempts={} segments={}",
        run.failure,
        run.attempts,
        run.segments().len()
    );
    assert_eq!(run.outputs().len(), 101);
    assert!(
        run.rejected_attempts > 0,
        "adversarial coarse first attempt must reject"
    );
    let mut max_error = 0.0_f64;
    for output in run.outputs() {
        let error = (output.center() - output.time().sin()).abs();
        max_error = max_error.max(error);
        assert!(output.radius() <= config.atol);
        assert!(
            error <= output.radius(),
            "t={} err={error:e} radius={:e}",
            output.time(),
            output.radius()
        );
    }
    assert_eq!(
        run.solver_work.accepted_steps as usize,
        run.segments().len()
    );
    assert_eq!(
        run.solver_work.rejected_steps as usize,
        run.rejected_attempts
    );
    for pair in run.segments().windows(2) {
        assert_eq!(pair[0].span().1, pair[1].span().0);
        assert_eq!(
            pair[0].bounds().endpoint_radius,
            pair[1].bounds().initial_radius
        );
    }
    println!(
        "PR101_GLOBAL={{\"outputs\":{},\"accepted\":{},\"rejected\":{},\"max_error\":{max_error:.17e},\"rhs\":{},\"verification_ops\":{}}}",
        run.outputs().len(),
        run.segments().len(),
        run.rejected_attempts,
        run.solver_work.rhs_evaluations,
        run.verification_work.total_operations()
    );
}

#[test]
fn refused_attempt_commits_no_state_segment_or_output() {
    let model = Pr101Model::new(-1.0e6, 10.0).unwrap();
    let config = AdaptiveStepConfig {
        atol: 1.0e-11,
        rtol: 1.0e-8,
        initial_step: 1.0,
        max_step: 1.0,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    let run = model
        .integrate_verified((0.0, 1.0), 0.0, 0.0, &[0.0, 0.25, 0.5, 1.0], &config)
        .unwrap();
    assert!(!run.success);
    assert_eq!(run.attempts, 1);
    assert_eq!(run.rejected_attempts, 1);
    assert!(run.segments().is_empty());
    assert_eq!(run.outputs().len(), 1);
    assert_eq!(run.outputs()[0].time(), 0.0);
    assert!(run.verification_work.provider_operations > 0);
    assert!(run.solver_work.rhs_evaluations > 0);
}

#[test]
fn invalid_domain_configuration_and_pure_relative_are_explicit_refusals() {
    let model = Pr101Model::new(-10.0, 2.0).unwrap();
    let cfg = AdaptiveStepConfig::default();
    assert!(
        model
            .integrate_verified((-0.1, 1.0), 0.0, 0.0, &[], &cfg)
            .is_err()
    );
    assert!(
        model
            .integrate_verified((0.0, 2.0), 0.0, 0.0, &[], &cfg)
            .is_err()
    );
    assert!(
        model
            .integrate_verified(
                (0.0, 0.0),
                0.0,
                0.0,
                &[],
                &AdaptiveStepConfig {
                    atol: 0.0,
                    ..cfg.clone()
                }
            )
            .is_err()
    );
    assert!(
        model
            .integrate_verified(
                (0.0, 0.0),
                0.0,
                0.0,
                &[],
                &AdaptiveStepConfig {
                    rtol: f64::NAN,
                    ..cfg.clone()
                }
            )
            .is_err()
    );
    assert!(
        model
            .integrate_verified((0.0, 1.0), 0.0, 0.0, &[0.5, 0.5], &cfg)
            .is_err()
    );
    assert!(Pr101Model::new(f64::NAN, 1.0).is_err());
}

#[test]
fn nonzero_start_inherited_uncertainty_and_adjacent_outputs() {
    let model = Pr101Model::new(-1000.0, 10.0).unwrap();
    let t0 = 0.5_f64;
    let initial = t0.sin();
    let cfg = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-8,
        initial_step: 0.1,
        max_step: 0.1,
        ..AdaptiveStepConfig::default()
    };
    let times = [t0, t0.next_up(), 0.6];
    // Reference sin(t0) libm error is covered by the supplied 1e-15 disk.
    let run = model
        .integrate_verified((t0, 0.6), initial, 1.0e-15, &times, &cfg)
        .unwrap();
    assert!(run.success, "{:?}", run.failure);
    assert_eq!(run.outputs().len(), 3);
    assert_eq!(run.outputs()[0].time().to_bits(), times[0].to_bits());
    assert_eq!(run.outputs()[1].time().to_bits(), times[1].to_bits());
    for output in run.outputs() {
        assert!((output.center() - output.time().sin()).abs() <= output.radius());
    }
    assert_eq!(run.segments()[0].bounds().initial_radius, 1.0e-15);
}

#[test]
fn nonzero_nonlinear_orbit_uses_expanding_transport() {
    // y=sin(t)+z(t), z'=z^2, z(0)=1/16 gives z=1/(16-t).
    // This oracle is used only after the production run; no exact callback is
    // provided. Unlike the usual manufactured orbit, the nonlinear term is
    // nonzero throughout this experiment.
    let model = Pr101Model::new(0.0, 1.0).unwrap();
    let cfg = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-8,
        initial_step: 0.1,
        max_step: 0.1,
        ..AdaptiveStepConfig::default()
    };
    let times: Vec<_> = (0..=16).map(|i| i as f64 / 64.0).collect();
    let run = model
        .integrate_verified((0.0, 0.25), 1.0 / 16.0, 0.0, &times, &cfg)
        .unwrap();
    assert!(run.success, "{:?}", run.failure);
    assert!(
        run.segments()
            .iter()
            .all(|segment| segment.bounds().logarithmic_norm_upper > 0.0)
    );
    for output in run.outputs() {
        let exact = output.time().sin() + 1.0 / (16.0 - output.time());
        assert!((output.center() - exact).abs() <= output.radius());
        assert!(output.radius() <= cfg.atol);
    }
}

#[test]
fn native_candidate_failure_has_one_outer_rejection() {
    let model = Pr101Model::new(0.0, f64::MAX).unwrap();
    let cfg = AdaptiveStepConfig {
        initial_step: 1.0,
        max_step: 1.0,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    let run = model
        .integrate_verified((0.0, 1.0), 1.0, 0.0, &[1.0], &cfg)
        .unwrap();
    assert!(!run.success);
    assert_eq!(run.attempts, 1);
    assert_eq!(run.rejected_attempts, 1);
    assert_eq!(run.solver_work.rejected_steps, 1);
    assert_eq!(run.solver_work.accepted_steps, 0);
    assert!(run.outputs().is_empty());
}
