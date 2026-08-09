use rodas5p_integrators::{
    CommonWBackendChoice, HomotopyRhsTelemetryProfile, RhsTelemetryRisk, analyze_rhs_batch,
    analyze_rhs_directions, compare_rhs_subspaces, recommend_common_w_backend,
    run_homotopy_rhs_telemetry_screen,
};

fn orthogonal_rows(rank: usize, dimension: usize) -> Vec<Vec<f64>> {
    (0..8)
        .map(|row| {
            let mut values = vec![0.0; dimension];
            values[row % rank] = 1.0;
            values
        })
        .collect()
}

#[test]
fn rhs_telemetry_detects_rank_and_dispatch_regimes() {
    let rank_one = vec![vec![1.0, 2.0, -1.0]; 8];
    let one = analyze_rhs_batch(&rank_one, 1.0e-8).unwrap();
    assert_eq!(one.numerical_rank, 1);
    assert_eq!(one.energy_rank_99, 1);
    assert!(one.maximum_abs_pairwise_cosine > 0.999999999999);
    assert_eq!(
        recommend_common_w_backend(&one, RhsTelemetryRisk::Low),
        CommonWBackendChoice::SeededSharedGmres
    );

    let rank_four = orthogonal_rows(4, 8);
    let four = analyze_rhs_batch(&rank_four, 1.0e-8).unwrap();
    assert_eq!(four.numerical_rank, 4);
    assert_eq!(
        recommend_common_w_backend(&four, RhsTelemetryRisk::Low),
        CommonWBackendChoice::BlockGmres
    );

    let rank_eight = orthogonal_rows(8, 8);
    let eight = analyze_rhs_batch(&rank_eight, 1.0e-8).unwrap();
    assert_eq!(eight.numerical_rank, 8);
    assert_eq!(
        recommend_common_w_backend(&eight, RhsTelemetryRisk::Low),
        CommonWBackendChoice::IndependentRayonGmres
    );
    assert_eq!(
        recommend_common_w_backend(&four, RhsTelemetryRisk::High),
        CommonWBackendChoice::IndependentRayonGmres
    );

    // One dominant direction plus tiny, mutually orthogonal residual directions is
    // numerically rank one at this tolerance, but it is not a safe seed-sharing batch.
    let misleading_rank_one = vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0e-10, 0.0, 0.0],
        vec![0.0, 0.0, 1.0e-10, 0.0],
        vec![0.0, 0.0, 0.0, 1.0e-10],
        vec![0.0, -1.0e-10, 0.0, 0.0],
        vec![0.0, 0.0, -1.0e-10, 0.0],
        vec![0.0, 0.0, 0.0, -1.0e-10],
    ];
    let misleading = analyze_rhs_batch(&misleading_rank_one, 1.0e-8).unwrap();
    assert_eq!(misleading.numerical_rank, 1);
    assert!(misleading.maximum_abs_pairwise_cosine > 0.99);
    assert!(misleading.median_abs_pairwise_cosine < 0.5);
    assert_ne!(
        recommend_common_w_backend(&misleading, RhsTelemetryRisk::Low),
        CommonWBackendChoice::SeededSharedGmres
    );
    let directional = analyze_rhs_directions(&misleading_rank_one, 1.0e-8).unwrap();
    assert_eq!(directional.numerical_rank, 4);
    assert_eq!(
        recommend_common_w_backend(&directional, RhsTelemetryRisk::Low),
        CommonWBackendChoice::BlockGmres
    );
}

#[test]
fn rhs_telemetry_principal_angles_distinguish_same_and_rotated_subspaces() {
    let first = orthogonal_rows(2, 4);
    let same = compare_rhs_subspaces(&first, &first, 1.0e-8).unwrap();
    assert!(same.maximum_principal_angle_degrees < 1.0e-8);

    let mut orthogonal = vec![vec![0.0; 4]; 8];
    for (index, row) in orthogonal.iter_mut().enumerate() {
        row[2 + index % 2] = 1.0;
    }
    let rotated = compare_rhs_subspaces(&first, &orthogonal, 1.0e-8).unwrap();
    assert!(rotated.minimum_principal_angle_degrees > 89.999999);
    assert!(rotated.maximum_principal_angle_degrees > 89.999999);
}

#[test]
fn smoke_rhs_telemetry_screen_is_deterministic_and_covers_low_q_and_sabr() {
    let first = run_homotopy_rhs_telemetry_screen(HomotopyRhsTelemetryProfile::Smoke).unwrap();
    let second = run_homotopy_rhs_telemetry_screen(HomotopyRhsTelemetryProfile::Smoke).unwrap();
    assert_eq!(first.scientific_checksum, second.scientific_checksum);
    assert_eq!(first.rows, second.rows);
    assert!(!first.rows.is_empty());
    assert!(first.rows.iter().any(|row| row.method == "sabr"));
    for q in [0, 1, 2] {
        assert!(
            first
                .rows
                .iter()
                .any(|row| row.method == "homotopy" && row.q == Some(q))
        );
    }
    assert!(first.rows.iter().all(|row| row.rhs_count == 8));
    assert!(first.rows.iter().all(|row| row.raw.numerical_rank <= 8));
    assert!(
        first
            .rows
            .iter()
            .all(|row| row.transformed.numerical_rank <= 8)
    );
    assert_eq!(first.explicit_jacobian_builds_in_dispatch, 0);
    assert!(!first.dispatcher_active);
    assert!(first.backend_recommendations_advisory);
    assert!(first.reference_explicit_jacobian_builds > 0);
    assert!(first.reference_factorization_builds > 0);
    assert!(first.behavior_comparisons > 0);
    assert_eq!(first.behavior_mismatches, 0);
    assert!(!first.solver_behavior_changed);
}

#[test]
fn canonical_rhs_telemetry_preserves_hostile_path_failures_instead_of_aborting() {
    let report = run_homotopy_rhs_telemetry_screen(HomotopyRhsTelemetryProfile::Canonical)
        .expect("canonical telemetry must complete even when one speculative path diverges");
    assert!(!report.failures.is_empty());
    assert!(report.failures.iter().any(|failure| {
        failure.case_id == "mv-n32-s1e6-m1e3-eta0.9"
            && failure.method == "homotopy"
            && failure.q == Some(2)
            && failure.theta == Some(1.0)
            && failure.rows_preserved > 0
    }));
    assert_eq!(report.summary.failed_paths, report.failures.len());
    assert!(report.rows.iter().any(|row| {
        row.case_id == "mv-n32-s1e6-m1e3-eta0.9"
            && row.method == "homotopy"
            && row.q == Some(2)
            && row.theta == Some(1.0)
    }));
}

#[test]
fn telemetry_drift_compares_only_semantically_matching_batches() {
    let report = run_homotopy_rhs_telemetry_screen(HomotopyRhsTelemetryProfile::Smoke).unwrap();
    let mut rows = report
        .rows
        .iter()
        .filter(|row| {
            row.method == "homotopy"
                && row.q == Some(0)
                && row.theta == Some(0.5)
                && row.case_id == "affine-noncommuting-mass"
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| (row.phase.clone(), row.propagation_level, row.round));

    let start = rows
        .iter()
        .find(|row| row.phase == "path-start" && row.round == 0)
        .unwrap();
    assert!(start.transformed_drift.is_none());

    let tangent_zero = rows
        .iter()
        .find(|row| row.phase == "tangent" && row.round == 0)
        .unwrap();
    let tangent_one = rows
        .iter()
        .find(|row| row.phase == "tangent" && row.round == 1)
        .unwrap();
    assert!(tangent_zero.transformed_drift.is_none());
    assert!(tangent_one.transformed_drift.is_some());
}
