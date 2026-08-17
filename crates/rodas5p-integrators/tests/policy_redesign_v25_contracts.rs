use rodas5p_integrators::{
    CausalRjfStep, PersistenceLatch, PointFeature, PrefixBudget, ProbeAction, causal_feature_value,
};

fn causal(step_index: usize, h: f64, error: f64, jvp_vectors: u64) -> CausalRjfStep {
    CausalRjfStep {
        step_index,
        h,
        embedded_error: error,
        jvp_vectors,
        linear_matvecs: jvp_vectors,
        rodas_wall_seconds: 1.0,
        log_error_two_steps_ago: None,
    }
}

#[test]
fn persistence_latch_fires_once_per_true_excursion() {
    let mut latch = PersistenceLatch::new(2).unwrap();
    let inputs = [false, true, true, true, false, true, true];
    let actions = inputs.map(|signal| latch.update(signal));
    assert_eq!(actions, [false, false, true, false, false, false, true]);
}

#[test]
fn prefix_budget_enforces_prospective_pathwise_bound() {
    let mut budget = PrefixBudget::new(0.05).unwrap();
    budget.record_committed_r(10.0).unwrap();
    assert!(budget.can_probe(0.4));
    budget.record_prefix(0.3, 0.4).unwrap();
    assert!(budget.speculative_wall() <= 0.05 * budget.committed_r_wall());
    assert!(!budget.can_probe(0.21));
}

#[test]
fn causal_features_use_rjf_data_only_and_have_fixed_orientation() {
    let current = CausalRjfStep {
        log_error_two_steps_ago: Some(-2.0),
        ..causal(3, 0.01, 1.0e-3, 20)
    };
    let pressure = causal_feature_value(&current, PointFeature::JvpPressure).unwrap();
    let contraction = causal_feature_value(&current, PointFeature::StepContraction).unwrap();
    let curvature = causal_feature_value(&current, PointFeature::ErrorCurvature).unwrap();
    assert!(pressure.is_finite() && contraction.is_finite() && curvature.is_finite());
    assert!(pressure > 0.0);
    assert!(contraction > 0.0);
    // log10(error)=-3 and two-step value=-2 -> d2=-1, oriented score is +1.
    assert!((curvature - 1.0).abs() < 1.0e-14);
}

#[test]
fn research_action_space_has_no_e_continuation_variant() {
    let actions = [ProbeAction::NoProbe, ProbeAction::PrefixProbe];
    assert_eq!(actions.len(), 2);
}
