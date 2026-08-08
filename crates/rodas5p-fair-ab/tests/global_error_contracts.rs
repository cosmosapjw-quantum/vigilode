use rodas5p_fair_ab::{
    CommonOutputGrid, ExternalErrorScale, GlobalErrorMetric, GlobalErrorParetoProfile,
    ParetoObservation, compute_global_error_metrics, nondominated_observation_ids,
    run_global_error_pareto_screen, select_cheapest_below_target,
};

#[test]
fn exact_and_known_perturbed_trajectories_have_expected_external_errors() {
    let grid = CommonOutputGrid::new(vec![0.0, 1.0]).unwrap();
    let reference = vec![vec![0.0, 0.0], vec![1.0, 1.0]];
    let scale = ExternalErrorScale::new(vec![1.0, 1.0], 0.0).unwrap();

    let exact =
        compute_global_error_metrics(&grid, &[0.0, 1.0], &reference, &reference, &scale).unwrap();
    assert_eq!(exact.endpoint_l2, 0.0);
    assert_eq!(exact.max_grid_l2, 0.0);
    assert_eq!(exact.rms_grid_l2, 0.0);

    let candidate = vec![vec![0.0, 0.0], vec![2.0, 1.0]];
    let error =
        compute_global_error_metrics(&grid, &[0.0, 1.0], &candidate, &reference, &scale).unwrap();
    assert!((error.endpoint_l2 - 1.0).abs() < 1.0e-14);
    assert!((error.max_grid_l2 - 1.0).abs() < 1.0e-14);
    assert!((error.rms_grid_l2 - 0.5_f64.sqrt()).abs() < 1.0e-14);
    assert!((error.endpoint_wrms - 0.5_f64.sqrt()).abs() < 1.0e-14);
    assert!((error.max_grid_wrms - 0.5_f64.sqrt()).abs() < 1.0e-14);
    assert!((error.rms_grid_wrms - 0.5).abs() < 1.0e-14);
}

#[test]
fn common_grid_rejects_missing_output_times_instead_of_interpolating() {
    let grid = CommonOutputGrid::new(vec![0.0, 0.5, 1.0]).unwrap();
    let reference = vec![vec![0.0], vec![0.5], vec![1.0]];
    let candidate = vec![vec![0.0], vec![1.0]];
    let scale = ExternalErrorScale::new(vec![1.0], 0.0).unwrap();
    let error = compute_global_error_metrics(&grid, &[0.0, 1.0], &candidate, &reference, &scale)
        .unwrap_err();
    assert!(error.to_string().contains("missing common output time"));
}

#[test]
fn pareto_and_target_selection_preserve_failures_and_choose_true_nondominated_points() {
    let points = vec![
        ParetoObservation::success("a", 1.0e-2, 10.0).unwrap(),
        ParetoObservation::success("b", 1.0e-3, 20.0).unwrap(),
        ParetoObservation::success("c", 2.0e-2, 30.0).unwrap(),
        ParetoObservation::failure("failed"),
        ParetoObservation::success("d", 1.0e-3, 25.0).unwrap(),
    ];
    assert_eq!(nondominated_observation_ids(&points), vec!["a", "b"]);
    assert_eq!(select_cheapest_below_target(&points, 5.0e-3), Some("b"));
    assert_eq!(select_cheapest_below_target(&points, 5.0e-4), None);
}

#[test]
fn fixed_anchor_screen_is_thread_deterministic_and_exposes_all_five_candidates() {
    let one = run_global_error_pareto_screen(GlobalErrorParetoProfile::Smoke, 1).unwrap();
    let four = run_global_error_pareto_screen(GlobalErrorParetoProfile::Smoke, 4).unwrap();
    assert_eq!(one.scientific_checksum, four.scientific_checksum);
    assert!(one.timing_authoritative);
    assert!(!four.timing_authoritative);
    let ids: std::collections::BTreeSet<_> = one
        .runs
        .iter()
        .map(|row| row.candidate_id.as_str())
        .collect();
    assert_eq!(
        ids,
        std::collections::BTreeSet::from([
            "bdf1-fixed",
            "bdf2-fixed",
            "radau-iia1-fixed",
            "radau-iia3-fixed",
            "sequential-rodas5p-direct",
        ])
    );
    assert!(
        one.runs
            .iter()
            .all(|row| row.reference_checksum.len() == 64)
    );
    assert!(one.runs.iter().all(|row| row.status.is_success()));
    assert!(one.fronts.iter().any(|front| {
        front.error_metric == GlobalErrorMetric::MaxGridL2 && !front.record_ids.is_empty()
    }));
    let bdf2 = one
        .runs
        .iter()
        .find(|row| row.candidate_id == "bdf2-fixed")
        .unwrap();
    assert!(bdf2.work.counters.accepted_steps > 0);
    assert!(bdf2.work.counters.nonlinear_solves > 0);
    assert_eq!(
        bdf2.timing.wall_samples_seconds.len(),
        one.timing_protocol.repetitions
    );
    assert!(bdf2.timing.batch_iterations >= 1);
    assert!(bdf2.timing.wall_q25_seconds <= bdf2.timing.wall_median_seconds);
    assert!(bdf2.timing.wall_median_seconds <= bdf2.timing.wall_q75_seconds);
    assert!(one.execution.scientific_suite_wall_seconds > 0.0);
    assert!(four.execution.scientific_suite_wall_seconds > 0.0);
    assert!(one.execution.timing_campaign_wall_seconds.is_some());
    assert!(four.execution.timing_campaign_wall_seconds.is_none());
    assert!(one.references.iter().all(|reference| {
        reference.source_kind == rodas5p_fair_ab::ReferenceSourceKind::AnalyticExact
    }));
    assert!(!one.targets.is_empty());
    assert!(!one.attainments.is_empty());
}

#[test]
fn pareto_screen_stores_only_common_outputs_and_reports_step_clipping_protocol() {
    let report = run_global_error_pareto_screen(GlobalErrorParetoProfile::Smoke, 1).unwrap();
    assert!(!report.output_policy.save_internal_steps);
    assert!(!report.output_policy.dense_output_used);
    assert_eq!(report.output_policy.landing, "step-clipping");

    let mut grouped = std::collections::BTreeMap::<(String, u64), Vec<_>>::new();
    for row in &report.runs {
        grouped
            .entry((row.problem_id.clone(), row.step_size.to_bits()))
            .or_default()
            .push(row);
        assert_eq!(row.work.internal_steps, row.work.counters.accepted_steps);
    }
    for rows in grouped.values() {
        let bytes: std::collections::BTreeSet<_> =
            rows.iter().map(|row| row.work.stored_state_bytes).collect();
        assert_eq!(
            bytes.len(),
            1,
            "all candidates must retain the same output-grid storage"
        );
    }
}
