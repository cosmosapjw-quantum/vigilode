use rodas5p_core::leja_action::CertifiedLejaCoeffCache;
use rodas5p_core::leja_setup_work::{
    CoefficientCacheObservation, LEJA_SETUP_RECEIPT_SCHEMA_V1, LejaSetupWork, OnceLockObservation,
    SetupCoverage,
};
use rodas5p_core::polynomial_action::{
    JointPhiInput, SymmetricNonpositiveOperator, TotalErrorStatus,
};
use rodas5p_core::{
    CertifiedLejaWorkspace, DenseMatrix, WorkCounters, leja_phi_action_certified_with_setup_work,
};

#[test]
fn counted_cache_cold_warm_and_refusal_retain_separate_work() {
    let mut cache = CertifiedLejaCoeffCache::new();
    let mut cold = LejaSetupWork::new();
    cache
        .precompute_certified_with_setup_work(-0.5, 0.125, 8, &mut cold)
        .unwrap();
    let cold = cold.snapshot();
    assert_eq!(cold.subtotal().cache_api_attempts(), 1);
    assert_eq!(cold.subtotal().cache_lookup_attempts(), 1);
    assert_eq!(cold.subtotal().cache_misses(), 1);
    assert_eq!(cold.subtotal().coefficient_tables_attempted(), 1);
    assert_eq!(cold.subtotal().coefficient_tables_completed(), 1);
    assert_eq!(cold.subtotal().cache_insert_attempts(), 1);
    assert_eq!(cold.subtotal().cache_inserts_completed(), 1);
    assert!(cold.subtotal().interval_kernel_invocations() > 0);
    assert_eq!(
        cold.coefficient_cache_observation(),
        CoefficientCacheObservation::PreparedByThisCall
    );
    assert_eq!(
        cold.coefficient_cache_key_bits(),
        Some([(-0.5_f64).to_bits(), 0.125_f64.to_bits(), 8])
    );
    assert!(cold.coefficient_cache_history().is_some());

    let mut warm = LejaSetupWork::new();
    cache
        .precompute_certified_with_setup_work(-0.5, 0.125, 8, &mut warm)
        .unwrap();
    let warm = warm.snapshot();
    assert_eq!(warm.subtotal().cache_api_attempts(), 1);
    assert_eq!(warm.subtotal().cache_lookup_attempts(), 1);
    assert_eq!(warm.subtotal().cache_hits(), 1);
    assert_eq!(warm.subtotal().coefficient_tables_attempted(), 0);
    assert_eq!(warm.subtotal().interval_kernel_invocations(), 0);
    assert_eq!(
        warm.coefficient_cache_observation(),
        CoefficientCacheObservation::WarmCountedEntry
    );
    assert!(warm.coefficient_cache_history().is_some());

    let mut legacy_cache = CertifiedLejaCoeffCache::new();
    legacy_cache.precompute_certified(-0.75, 0.125, 8).unwrap();
    let mut legacy_warm = LejaSetupWork::new();
    legacy_cache
        .precompute_certified_with_setup_work(-0.75, 0.125, 8, &mut legacy_warm)
        .unwrap();
    let legacy_warm = legacy_warm.snapshot();
    assert_eq!(
        legacy_warm.coefficient_cache_observation(),
        CoefficientCacheObservation::PreexistingUnattributed
    );
    assert_eq!(
        legacy_warm.setup_coverage(),
        SetupCoverage::PartialUnattributed
    );
    assert!(legacy_warm.coefficient_cache_history().is_none());
    assert_eq!(legacy_warm.subtotal().coefficient_tables_attempted(), 0);

    let mut refused_cache = CertifiedLejaCoeffCache::new();
    let mut refused = LejaSetupWork::new();
    assert!(
        refused_cache
            .precompute_certified_with_setup_work(0.0, f64::MAX, 8, &mut refused)
            .is_err()
    );
    let refused = refused.snapshot();
    assert_eq!(refused.subtotal().cache_api_attempts(), 1);
    assert_eq!(refused.subtotal().cache_lookup_attempts(), 1);
    assert_eq!(refused.subtotal().cache_misses(), 1);
    assert_eq!(refused.subtotal().coefficient_tables_attempted(), 1);
    assert_eq!(refused.subtotal().coefficient_tables_completed(), 0);
    assert_eq!(refused.subtotal().cache_insert_attempts(), 0);
    assert!(refused.subtotal().interval_kernel_invocations() > 0);
    assert!(refused_cache.is_empty());
}

#[test]
fn setup_receipt_serialization_is_versioned_and_immutable() {
    let receipt = LejaSetupWork::new().snapshot();
    let json = serde_json::to_value(receipt).unwrap();
    assert_eq!(json["schema"], LEJA_SETUP_RECEIPT_SCHEMA_V1);
    assert_eq!(json["setup_coverage"], "complete");
    assert_eq!(json["subtotal"]["interval_kernel_invocations"], 0);
    assert_eq!(json["counter_overflowed"], false);
    assert!(receipt.leja_points_history().is_none());
    assert_eq!(receipt.setup_coverage(), SetupCoverage::Complete);
}

#[test]
fn counted_action_records_cold_coefficient_table_without_changing_authority() {
    let op = SymmetricNonpositiveOperator::gershgorin(
        DenseMatrix::from_rows(&[&[-1.0, 0.0], &[0.0, -2.0]]).unwrap(),
    )
    .unwrap();
    let vector = [1.0, -0.25];
    let input = JointPhiInput::SameVector {
        vector: &vector,
        scales: [1.0, 0.0, 0.0, 0.0, 0.0],
    };
    let mut cache = CertifiedLejaCoeffCache::new();
    let mut work = WorkCounters::default();
    let mut setup = LejaSetupWork::new();
    let mut workspace = CertifiedLejaWorkspace::new(2, 1);
    let report = leja_phi_action_certified_with_setup_work(
        &op,
        1.0,
        input,
        1e-8,
        12,
        Some(&mut cache),
        &mut work,
        &mut workspace,
        &mut setup,
    )
    .unwrap();
    assert!(matches!(
        report.total_error(),
        TotalErrorStatus::EstimateOnly { .. }
    ));
    assert_eq!(report.fused().len(), 2);
    let receipt = setup.snapshot();
    assert_eq!(receipt.subtotal().coefficient_tables_attempted(), 1);
    assert_eq!(receipt.subtotal().coefficient_tables_completed(), 1);
    assert!(receipt.subtotal().interval_kernel_invocations() > 0);
    assert!(receipt.subtotal().scalar_proxy_units() > 0);
    assert!(receipt.subtotal().allocation_requests() > 0);
    assert!(receipt.leja_points_history_available());
    assert!(receipt.node_bounds_history_available());
    assert!(receipt.suffix_bounds_history_available());
    assert_eq!(receipt.subtotal().cache_misses(), 1);
    assert_eq!(receipt.subtotal().cache_hits(), 1); // preparation plus slice retrieval
    assert_eq!(
        receipt.coefficient_cache_observation(),
        CoefficientCacheObservation::PreparedByThisCall
    );
    assert!(receipt.coefficient_cache_history().is_some());
    assert!(matches!(
        receipt.leja_points_observation(),
        OnceLockObservation::InitializedByThisCall
            | OnceLockObservation::WarmCountedInitializer
            | OnceLockObservation::PreexistingUnattributed
    ));
}

#[test]
fn counted_cache_history_inherits_legacy_static_dependency_coverage() {
    // Run this test alone in a fresh test process: the legacy accessor makes
    // the process-global point table preexisting and unattributed.
    rodas5p_core::leja_action::leja_points();
    let mut cache = CertifiedLejaCoeffCache::new();
    let mut setup = LejaSetupWork::new();
    cache
        .precompute_certified_with_setup_work(-0.625, 0.125, 7, &mut setup)
        .unwrap();
    let first = setup.snapshot();
    if first.leja_points_observation() == OnceLockObservation::PreexistingUnattributed {
        assert_eq!(first.setup_coverage(), SetupCoverage::PartialUnattributed);
    }
    assert_eq!(
        first.coefficient_cache_observation(),
        CoefficientCacheObservation::PreparedByThisCall
    );

    let mut warm = LejaSetupWork::new();
    cache
        .precompute_certified_with_setup_work(-0.625, 0.125, 7, &mut warm)
        .unwrap();
    let warm = warm.snapshot();
    assert_eq!(
        warm.coefficient_cache_observation(),
        CoefficientCacheObservation::WarmCountedEntry
    );
    assert_eq!(warm.setup_coverage(), first.setup_coverage());
    assert_eq!(
        warm.coefficient_cache_history(),
        first.coefficient_cache_history()
    );
    assert!(warm.coefficient_cache_history().is_some());
}

#[test]
fn counted_and_legacy_success_preserve_report_and_work_counter_identity() {
    let op = SymmetricNonpositiveOperator::gershgorin(
        DenseMatrix::from_rows(&[&[-1.0, 0.0], &[0.0, -2.0]]).unwrap(),
    )
    .unwrap();
    let vector = [0.75, -0.125];
    let input = JointPhiInput::SameVector {
        vector: &vector,
        scales: [1.0, -0.5, 0.25, 0.0, 0.0],
    };
    let mut legacy_cache = CertifiedLejaCoeffCache::new();
    let mut legacy_workspace = CertifiedLejaWorkspace::new(2, 1);
    let mut legacy_work = WorkCounters::default();
    let legacy_report =
        rodas5p_core::leja_action::leja_phi_action_certified_with_cache_and_workspace(
            &op,
            0.125,
            input,
            1.0,
            8,
            Some(&mut legacy_cache),
            &mut legacy_work,
            &mut legacy_workspace,
        )
        .unwrap()
        .clone();

    let mut counted_cache = CertifiedLejaCoeffCache::new();
    let mut counted_workspace = CertifiedLejaWorkspace::new(2, 1);
    let mut counted_work = WorkCounters::default();
    let mut setup = LejaSetupWork::new();
    let counted_report = leja_phi_action_certified_with_setup_work(
        &op,
        0.125,
        input,
        1.0,
        8,
        Some(&mut counted_cache),
        &mut counted_work,
        &mut counted_workspace,
        &mut setup,
    )
    .unwrap()
    .clone();

    assert_eq!(counted_report, legacy_report);
    assert_eq!(counted_work, legacy_work);
}
