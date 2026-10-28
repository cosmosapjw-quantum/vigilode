//! Finite inputs must not corrupt the causal overhead account by accumulation.
use rodas5p_integrators::transaction::{AdmissionConfig, AdmissionRule, AttemptRecord, RegimeBin};

#[test]
fn protected_cost_overflow_is_refused_without_mutating_the_bin() {
    let mut rule = AdmissionRule::new(AdmissionConfig::default());
    let bin = RegimeBin(7);
    rule.record_protected_step(bin, 1e308).unwrap();
    let before = rule.clone();
    assert!(rule.record_protected_step(bin, 1e308).is_err());
    assert_eq!(rule, before);
}

#[test]
fn attempted_cost_overflow_is_refused_without_mutating_the_bin() {
    let mut rule = AdmissionRule::new(AdmissionConfig::default());
    let bin = RegimeBin(8);
    let attempt = AttemptRecord {
        charged_units: 1e308,
        success: false,
        replaced_protected_units: 0.0,
    };
    rule.record_attempt(bin, attempt).unwrap();
    let before = rule.clone();
    assert!(rule.record_attempt(bin, attempt).is_err());
    assert_eq!(rule, before);
}

#[test]
fn saved_cost_overflow_is_refused_without_mutating_the_bin() {
    let mut rule = AdmissionRule::new(AdmissionConfig::default());
    let bin = RegimeBin(9);
    rule.record_protected_step(bin, 1e308).unwrap();
    let before = rule.clone();
    assert!(
        rule.record_attempt(
            bin,
            AttemptRecord {
                charged_units: 1.0,
                success: true,
                replaced_protected_units: 1e308
            }
        )
        .is_err()
    );
    assert_eq!(rule, before);
}

#[test]
fn numerically_unrepresentable_posterior_configuration_is_refused() {
    assert!(
        AdmissionRule::try_new(AdmissionConfig {
            prior_alpha: 1e308,
            prior_beta: 1e308,
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        AdmissionRule::try_new(AdmissionConfig {
            z: 1e308,
            ..Default::default()
        })
        .is_err()
    );
}
