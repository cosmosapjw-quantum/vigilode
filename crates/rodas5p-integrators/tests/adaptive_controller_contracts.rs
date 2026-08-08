use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, ControllerKind, step_doubling_wrms_error,
};

#[test]
fn adaptive_config_rejects_inconsistent_bounds() {
    let config = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-6,
        initial_step: 0.1,
        min_step: 0.2,
        max_step: 0.1,
        max_attempts: 10,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 5.0,
        reject_max_factor: 0.9,
        controller: ControllerKind::Integral,
    };
    assert!(config.validate().is_err());
}

#[test]
fn integral_controller_uses_estimator_order_and_rejection_cap() {
    let config = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-6,
        initial_step: 0.1,
        min_step: 1.0e-12,
        max_step: 1.0,
        max_attempts: 100,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 5.0,
        reject_max_factor: 0.8,
        controller: ControllerKind::Integral,
    };
    let state = AdaptiveControllerState::default();
    let accepted = state.propose_factor(&config, 0.25, 2, true).unwrap();
    let rejected = state.propose_factor(&config, 0.25, 2, false).unwrap();
    assert!((accepted - 1.8).abs() < 1.0e-14);
    assert!((rejected - 0.8).abs() < 1.0e-14);
}

#[test]
fn pi_controller_updates_only_after_acceptance() {
    let config = AdaptiveStepConfig {
        controller: ControllerKind::Pi,
        ..AdaptiveStepConfig::default()
    };
    let mut state = AdaptiveControllerState::default();
    assert_eq!(state.previous_accepted_error(), None);
    state.record_rejection(4.0).unwrap();
    assert_eq!(state.previous_accepted_error(), None);
    state.record_acceptance(0.25).unwrap();
    assert_eq!(state.previous_accepted_error(), Some(0.25));
    let factor = state.propose_factor(&config, 0.5, 5, true).unwrap();
    let expected = config.safety * 0.5_f64.powf(-0.7 / 5.0) * 0.25_f64.powf(0.4 / 5.0);
    assert!((factor - expected).abs() < 1.0e-14);
}

#[test]
fn step_doubling_returns_fine_path_wrms_error() {
    let estimate = step_doubling_wrms_error(&[0.0], &[0.8], &[1.0], 1.0, 0.0, 1).unwrap();
    assert_eq!(estimate.method_order, 1);
    assert_eq!(estimate.estimator_order, 2);
    assert!((estimate.error_vector[0] - 0.2).abs() < 1.0e-14);
    assert!((estimate.error_norm - 0.2).abs() < 1.0e-14);
}
