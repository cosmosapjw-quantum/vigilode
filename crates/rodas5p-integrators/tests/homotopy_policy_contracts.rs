use rodas5p_integrators::{
    HomotopyPathConfig, HomotopyPredictor, HomotopyStepConfig, OutputBudgetPolicy,
};

#[test]
fn output_budget_policies_match_dimensionless_formulas() {
    let absolute = OutputBudgetPolicy::absolute(0.1).unwrap();
    assert!((absolute.budget(0.25, 0.02).unwrap() - 0.1).abs() < 1e-15);

    let relative = OutputBudgetPolicy::embedded_relative(0.2).unwrap();
    assert!((relative.budget(0.25, 0.02).unwrap() - 0.05).abs() < 1e-15);

    let power = OutputBudgetPolicy::step_power(0.1, 0.04, 6).unwrap();
    let expected = 0.1 * (0.5_f64).powi(6);
    assert!((power.budget(0.25, 0.02).unwrap() - expected).abs() < 1e-15);

    let mixed = OutputBudgetPolicy::mixed(0.2, 0.1, 0.04, 6).unwrap();
    assert!((mixed.budget(0.25, 0.02).unwrap() - expected.min(0.05)).abs() < 1e-15);
}

#[test]
fn output_budget_policies_reject_invalid_parameters() {
    assert!(OutputBudgetPolicy::absolute(f64::NAN).is_err());
    assert!(OutputBudgetPolicy::absolute(-1.0).is_err());
    assert!(OutputBudgetPolicy::embedded_relative(-0.1).is_err());
    assert!(OutputBudgetPolicy::step_power(0.1, 0.0, 6).is_err());
    assert!(OutputBudgetPolicy::step_power(0.1, 0.04, 0).is_err());
    assert!(OutputBudgetPolicy::mixed(0.1, 0.1, 0.04, 0).is_err());

    let policy = OutputBudgetPolicy::step_power(0.1, 0.04, 6).unwrap();
    assert!(policy.budget(0.2, 0.0).is_err());
    assert!(policy.budget(f64::INFINITY, 0.02).is_err());
}

#[test]
fn homotopy_step_config_preserves_absolute_constructor_and_accepts_policy() {
    let path = HomotopyPathConfig::new(0.0, 2, 2, HomotopyPredictor::Euler, 0).unwrap();
    let legacy = HomotopyStepConfig::new(path.clone(), 0.1).unwrap();
    assert_eq!(
        legacy.output_policy(),
        &OutputBudgetPolicy::absolute(0.1).unwrap()
    );

    let policy = OutputBudgetPolicy::mixed(0.1, 0.03, 0.04, 6).unwrap();
    let configured = HomotopyStepConfig::with_policy(path, policy.clone()).unwrap();
    assert_eq!(configured.output_policy(), &policy);
}

#[test]
fn local_rayon_execution_matches_sequential_ordered_results() {
    use rodas5p_integrators::ParallelExecution;

    let items: Vec<u64> = (0..128).collect();
    let sequential = ParallelExecution::sequential();
    let rayon = ParallelExecution::rayon(4).unwrap();
    let expected = sequential
        .map_ordered(&items, |value| Ok(value.wrapping_mul(*value)))
        .unwrap();
    let actual = rayon
        .map_ordered(&items, |value| Ok(value.wrapping_mul(*value)))
        .unwrap();
    assert_eq!(expected, actual);
    assert_eq!(rayon.threads(), 4);
    assert!(ParallelExecution::rayon(0).is_err());
}

#[test]
fn policy_replay_split_is_disjoint_and_rayon_scientific_output_is_deterministic() {
    use rodas5p_integrators::{HomotopyExperimentProfile, run_homotopy_order_policy_screen};

    let sequential = run_homotopy_order_policy_screen(HomotopyExperimentProfile::Smoke, 1).unwrap();
    let parallel = run_homotopy_order_policy_screen(HomotopyExperimentProfile::Smoke, 4).unwrap();

    assert_eq!(sequential.source_summary, parallel.source_summary);
    assert_eq!(sequential.replay_rows, parallel.replay_rows);
    assert_eq!(sequential.policy_summaries, parallel.policy_summaries);
    assert_eq!(sequential.family_winners, parallel.family_winners);
    assert_eq!(sequential.trajectory_rows, parallel.trajectory_rows);
    assert_eq!(sequential.trajectory_gates, parallel.trajectory_gates);
    assert_eq!(sequential.execution.threads, 1);
    assert_eq!(parallel.execution.threads, 4);

    let calibration: std::collections::BTreeSet<_> = sequential
        .replay_rows
        .iter()
        .filter(|row| row.split == "calibration")
        .map(|row| row.case_id.as_str())
        .collect();
    let holdout: std::collections::BTreeSet<_> = sequential
        .replay_rows
        .iter()
        .filter(|row| row.split == "holdout")
        .map(|row| row.case_id.as_str())
        .collect();
    assert!(!calibration.is_empty());
    assert!(!holdout.is_empty());
    assert!(calibration.is_disjoint(&holdout));
    assert_eq!(sequential.family_winners.len(), 4);
}

#[test]
fn order_policy_screen_keeps_the_protected_sequential_fifth_order_gate() {
    use rodas5p_integrators::{HomotopyExperimentProfile, run_homotopy_order_policy_screen};

    let report = run_homotopy_order_policy_screen(HomotopyExperimentProfile::Smoke, 2).unwrap();
    assert!(!report.trajectory_rows.is_empty());
    let orders: Vec<f64> = report
        .trajectory_rows
        .iter()
        .filter(|row| {
            row.problem_id == "manufactured-vector-order" && row.method == "sequential-direct"
        })
        .filter_map(|row| row.observed_order)
        .collect();
    assert!(!orders.is_empty(), "orders={orders:?}");
    assert!(
        orders.iter().all(|order| *order >= 4.8),
        "orders={orders:?}"
    );
    assert!(
        report
            .trajectory_gates
            .iter()
            .any(|gate| gate.method == "sequential-direct" && gate.fifth_order_pass)
    );
}

#[test]
fn trajectory_gate_false_accepts_are_scoped_to_the_matching_path_configuration() {
    use rodas5p_integrators::{HomotopyExperimentProfile, run_homotopy_order_policy_screen};

    let report = run_homotopy_order_policy_screen(HomotopyExperimentProfile::Canonical, 1).unwrap();
    for gate in report
        .trajectory_gates
        .iter()
        .filter(|gate| gate.method != "sequential-direct")
    {
        let representative = report
            .trajectory_rows
            .iter()
            .find(|row| {
                row.problem_id == gate.problem_id
                    && row.method == gate.method
                    && row.policy_id == gate.policy_id
            })
            .unwrap();
        let expected = report
            .replay_rows
            .iter()
            .filter(|row| row.split == "holdout" && row.false_accept)
            .filter(|row| Some(row.policy_id.as_str()) == gate.policy_id.as_deref())
            .filter(|row| Some(row.theta.to_bits()) == representative.theta.map(f64::to_bits))
            .filter(|row| Some(row.q) == representative.q)
            .filter(|row| Some(row.path_rounds) == representative.path_rounds)
            .filter(|row| Some(row.predictor) == representative.predictor)
            .filter(|row| Some(row.corrections_per_point) == representative.corrections_per_point)
            .count();
        assert_eq!(
            gate.holdout_false_accepts, expected,
            "gate={} policy={:?}",
            gate.method, gate.policy_id
        );
    }
}

#[test]
fn parallel_execution_mutates_outputs_in_order_without_intermediate_results() {
    use rodas5p_core::CoreResult;
    use rodas5p_integrators::ParallelExecution;

    let inputs = vec![1_u64, 2, 3, 4, 5, 6, 7, 8];
    let mut outputs = vec![0_u64; inputs.len()];
    ParallelExecution::rayon(4)
        .unwrap()
        .try_for_each_ordered_mut(&inputs, &mut outputs, |input, output| {
            *output = input.wrapping_mul(*input);
            Ok(())
        })
        .unwrap();
    assert_eq!(outputs, vec![1, 4, 9, 16, 25, 36, 49, 64]);

    let mut too_short = vec![0_u64; 2];
    let result: CoreResult<()> = ParallelExecution::sequential().try_for_each_ordered_mut(
        &inputs,
        &mut too_short,
        |input, output| {
            *output = *input;
            Ok(())
        },
    );
    assert!(result.is_err());
}
