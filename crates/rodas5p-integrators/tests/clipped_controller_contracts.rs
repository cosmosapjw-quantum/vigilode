//! WU-6 (audit F-006): a clipped acceptance may update the step request.
//!
//! Before this change, every accepted clipped step returned the pre-clip
//! request unchanged and discarded the error sample, so the request stayed
//! frozen between rejections.  An informative clipped sample
//! (`trial_h / requested_h >= 0.5`) now enters the controller history.  It may
//! raise the request, or lower it when it predicts rejection of the remembered
//! request, but never below the accepted trial.  A sliver landing
//! (`trial_h / requested_h < 0.5`) never lowers the request.

use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, ControllerKind, RODAS5P_ESTIMATOR_ORDER,
    rodas_next_step_after_attempt,
};

fn pi_config() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        controller: ControllerKind::Pi,
        safety: 0.5,
        min_factor: 0.1,
        max_factor: 10.0,
        ..AdaptiveStepConfig::default()
    }
}

#[test]
fn informative_clipped_sample_with_small_error_raises_the_request() {
    let config = pi_config();
    let mut state = AdaptiveControllerState::default();
    let next =
        rodas_next_step_after_attempt(&mut state, &config, 0.1, 0.05, 1.0e-12, true, true).unwrap();
    // candidate = 0.05 * max_factor = 0.5 > requested 0.1
    assert_eq!(next.to_bits(), 0.5_f64.to_bits());
    assert_eq!(state.previous_accepted_error(), Some(1.0e-12));
}

#[test]
fn informative_clipped_sample_predicting_rejection_lowers_but_not_below_trial() {
    let config = pi_config();
    let mut state = AdaptiveControllerState::default();
    // predicted error at the remembered request: 0.9 * (0.1 / 0.06)^5 > 1
    let next =
        rodas_next_step_after_attempt(&mut state, &config, 0.1, 0.06, 0.9, true, true).unwrap();
    assert!(next < 0.1, "remembered request must shrink, got {next}");
    assert!(next >= 0.06, "never below the accepted trial, got {next}");
    assert_eq!(state.previous_accepted_error(), Some(0.9));

    // Moderate error that does not predict rejection keeps the request.
    let mut state = AdaptiveControllerState::default();
    let next =
        rodas_next_step_after_attempt(&mut state, &config, 0.1, 0.09, 0.3, true, true).unwrap();
    assert!(next >= 0.1, "got {next}");
}

#[test]
fn sliver_landing_never_collapses_the_request() {
    let config = pi_config();
    for error in [0.0, 1.0e-16, 0.5, 0.99] {
        let mut state = AdaptiveControllerState::default();
        let next = rodas_next_step_after_attempt(&mut state, &config, 0.4, 0.01, error, true, true)
            .unwrap();
        assert!(next >= 0.4, "error {error}: {next}");
        assert_eq!(
            state.previous_accepted_error(),
            None,
            "sliver is not a sample"
        );
    }
}

#[test]
fn pi_factor_uses_the_previous_accepted_error_not_the_current_one() {
    let config = pi_config();
    let mut state = AdaptiveControllerState::default();
    state.record_acceptance(0.25).unwrap();
    let trial = 0.02;
    let next =
        rodas_next_step_after_attempt(&mut state, &config, trial, trial, 0.5, true, false).unwrap();
    let order = RODAS5P_ESTIMATOR_ORDER as f64;
    let factor = (config.safety * 0.5_f64.powf(-0.7 / order) * 0.25_f64.powf(0.4 / order))
        .clamp(config.min_factor, config.max_factor);
    assert!(
        (next - trial * factor).abs() <= 1.0e-15,
        "{next} vs {}",
        trial * factor
    );
    assert_eq!(state.previous_accepted_error(), Some(0.5));
}

#[test]
fn integral_controller_unclipped_acceptance_is_unchanged() {
    let config = AdaptiveStepConfig::default();
    let mut state = AdaptiveControllerState::default();
    let next =
        rodas_next_step_after_attempt(&mut state, &config, 0.01, 0.01, 0.2, true, false).unwrap();
    let order = RODAS5P_ESTIMATOR_ORDER as f64;
    let factor =
        (config.safety * 0.2_f64.powf(-1.0 / order)).clamp(config.min_factor, config.max_factor);
    assert_eq!(next.to_bits(), (0.01 * factor).to_bits());
}
