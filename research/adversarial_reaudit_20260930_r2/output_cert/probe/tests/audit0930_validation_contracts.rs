//! Input validation (audit 2026-09-30: CERT-01, CERT-02).

use rodas5p_core::error_scale;
use rodas5p_integrators::OutputBudgetPolicy;

#[test]
fn malformed_budget_policies_are_rejected_at_evaluation() {
    // Each of these bypassed the validating constructors and was evaluated:
    // 0.25, 1, 2 (exponent u32::MAX cast to -1) and 0.015625 (NaN hidden by
    // f64::min) at embedded error 0.1, h = 0.5.
    let malformed = [
        OutputBudgetPolicy::StepPower {
            epsilon_ref: 1.0,
            h_ref: -1.0,
            exponent: 2,
        },
        OutputBudgetPolicy::StepPower {
            epsilon_ref: 1.0,
            h_ref: 1.0,
            exponent: 0,
        },
        OutputBudgetPolicy::StepPower {
            epsilon_ref: 1.0,
            h_ref: 1.0,
            exponent: u32::MAX,
        },
        OutputBudgetPolicy::Mixed {
            eta: f64::NAN,
            epsilon_ref: 1.0,
            h_ref: 1.0,
            exponent: 6,
        },
        OutputBudgetPolicy::Absolute { epsilon: -1.0 },
        OutputBudgetPolicy::EmbeddedRelative { eta: f64::INFINITY },
    ];
    for policy in malformed {
        assert!(policy.validate().is_err(), "{policy:?}");
        assert!(policy.budget(0.1, 0.5).is_err(), "{policy:?}");
    }
    // Deserialized policies take the same path.
    let json = r#"{"kind":"step-power","epsilon_ref":1.0,"h_ref":1.0,"exponent":4294967295}"#;
    let policy: OutputBudgetPolicy = serde_json::from_str(json).unwrap();
    assert!(policy.budget(0.1, 0.5).is_err());
    // A valid order-6 policy is unchanged: (1/2)^6.
    let valid = OutputBudgetPolicy::step_power(1.0, 1.0, 6).unwrap();
    assert_eq!(valid.budget(0.1, 0.5).unwrap(), 1.0 / 64.0);
}

#[test]
fn error_scale_checks_each_input() {
    // All three returned a finite scale before.
    assert!(error_scale(&[f64::NAN], &[1.0], &[0.1], 0.1).is_err());
    assert!(error_scale(&[1.0], &[f64::NAN], &[0.1], 0.1).is_err());
    assert!(error_scale(&[2.0], &[2.0], &[-1.0], 1.0).is_err());
    assert!(error_scale(&[2.0], &[2.0], &[f64::INFINITY], 1.0).is_err());
    // atol = 0 with rtol > 0 and a nonzero state stays valid.
    assert_eq!(error_scale(&[2.0], &[1.0], &[0.0], 0.5).unwrap(), vec![1.0]);
    assert_eq!(
        error_scale(&[1.0, -3.0], &[2.0, 1.0], &[0.1, 0.2], 0.1).unwrap(),
        vec![0.1 + 0.1 * 2.0, 0.2 + 0.1 * 3.0]
    );
}
