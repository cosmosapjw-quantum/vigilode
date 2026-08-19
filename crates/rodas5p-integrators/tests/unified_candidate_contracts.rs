use std::collections::BTreeSet;

use rodas5p_integrators::{CandidateCatalog, CandidateFamily, CandidateStatus};

#[test]
fn catalog_exposes_all_current_and_deferred_candidate_families() {
    let catalog = CandidateCatalog::research_default().unwrap();
    let ids: BTreeSet<_> = catalog
        .entries()
        .iter()
        .map(|c| c.id().to_string())
        .collect();
    assert_eq!(
        ids.len(),
        catalog.entries().len(),
        "candidate IDs must be unique"
    );

    for family in [
        CandidateFamily::Sequential,
        CandidateFamily::Sabr,
        CandidateFamily::Homotopy,
    ] {
        assert!(catalog.entries().iter().any(|c| {
            c.family() == family && matches!(c.status(), CandidateStatus::Executable)
        }));
    }

    for family in [
        CandidateFamily::Bdf,
        CandidateFamily::RadauIrk,
        CandidateFamily::PeerW,
        CandidateFamily::ParallelSdc,
        CandidateFamily::RosenbrockKrylov,
        CandidateFamily::Borok,
        CandidateFamily::ExponentialLeja,
    ] {
        assert!(catalog.entries().iter().any(|c| {
            c.family() == family && matches!(c.status(), CandidateStatus::Deferred { .. })
        }));
    }

    assert!(ids.contains("sequential-direct-off"));
    assert!(ids.contains("sequential-lgmres-stage"));
    assert!(ids.contains("sequential-lgmres-persistent"));
    assert!(ids.contains("sabr-forward-linear-history"));
    assert!(ids.contains("homotopy-theta0.000-q0-r2-euler-c0"));
}

#[test]
fn nonlinear_reference_certification_exposes_first_correction_underestimate() {
    use rodas5p_core::{LinearMethod, LinearSolverConfig, WorkCounters, error_scale, wrms};
    use rodas5p_integrators::{
        HomotopyPathConfig, HomotopyPredictor, RefinedRootConfig, StructuredBlockSystem,
        build_step_context, certify_nonlinear_target, certify_second_correction,
        manufactured_vector_problem, refine_target_root, run_fixed_homotopy_path,
        sequential_stages,
    };

    let (problem, y0) = manufactured_vector_problem(4, 10_000.0, 1_000.0, 0.9, 0.0).unwrap();
    let mut counters = WorkCounters::default();
    let context = build_step_context(&problem, 0.0, &y0, 1.0e-3, &mut counters).unwrap();
    let block = StructuredBlockSystem::new(&context);
    let path = run_fixed_homotopy_path(
        &block,
        &HomotopyPathConfig::new(0.0, 0, 3, HomotopyPredictor::AdamsBashforth2, 1).unwrap(),
        &mut counters,
    )
    .unwrap();

    let first = certify_nonlinear_target(&block, &path.stages, 1e-7, 1e-6, &mut counters).unwrap();
    let second =
        certify_second_correction(&block, &path.stages, 1e-7, 1e-6, true, &mut counters).unwrap();
    assert!(second.first_output_wrms.is_finite());
    assert!(second.second_output_wrms.is_finite());
    assert!(second.residual_ratio.is_finite());
    assert!(second.second_output_wrms < second.first_output_wrms);
    assert!(second.residual_after_second_norm < second.residual_after_first_norm);
    if let Some(tail) = second.empirical_tail_wrms {
        assert!(tail >= second.first_output_wrms);
    }

    let direct = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let oracle = sequential_stages(&context, &direct, None, &mut counters).unwrap();
    let output = |stages: &[Vec<f64>]| {
        let mut y = y0.clone();
        for (b, stage) in context.coeffs.b.iter().zip(stages) {
            for (value, increment) in y.iter_mut().zip(stage) {
                *value += b * increment;
            }
        }
        y
    };
    let oracle_y = output(&oracle.stages);
    let candidate_y = output(&path.stages);
    let scale = error_scale(&y0, &oracle_y, &[1e-7], 1e-6).unwrap();
    let actual = wrms(
        &candidate_y
            .iter()
            .zip(&oracle_y)
            .map(|(candidate, reference)| candidate - reference)
            .collect::<Vec<_>>(),
        &scale,
    )
    .unwrap();
    assert!(actual > 1.1 * first.output_wrms);

    let refined = refine_target_root(
        &block,
        &path.stages,
        1e-7,
        1e-6,
        &RefinedRootConfig::default(),
        &mut counters,
    )
    .unwrap();
    assert!(refined.converged, "{:?}", refined.termination);
    assert!(refined.relative_residual <= refined.residual_tolerance);
    assert!((refined.candidate_output_wrms - actual).abs() <= 1e-3 * actual.max(1e-14));
}

#[test]
fn uncertified_unified_candidate_row_requires_an_explicit_decision_reason() {
    use rodas5p_core::WorkCounters;
    use rodas5p_integrators::{CandidateFamily, UnifiedCandidateOutcome, UnifiedCandidateRow};

    let row = UnifiedCandidateRow {
        candidate_id: "uncertified".into(),
        family: CandidateFamily::Homotopy,
        case_id: "case".into(),
        outcome: UnifiedCandidateOutcome::Uncertified,
        scientifically_certified: false,
        reference_certificate_source: rodas5p_integrators::ReferenceCertificateSource::Unavailable,
        reference_fallback_used: false,
        certificate_failure: None,
        used_fallback: false,
        embedded_error: Some(0.0),
        oracle_output_wrms: Some(0.0),
        oracle_stage_l2: Some(0.0),
        first_output_wrms: Some(0.0),
        second_output_wrms: Some(0.0),
        second_output_ratio: Some(0.0),
        second_residual_ratio: Some(0.0),
        second_contraction_evidence: Some(true),
        refined_root_converged: Some(false),
        refined_root_termination: Some("iteration budget exhausted".into()),
        refined_root_output_wrms: Some(0.0),
        refined_root_relative_residual: Some(1.0),
        output_budget: 0.1,
        c3_output_budget_pass: None,
        oracle_output_budget_pass: Some(true),
        c3_false_accept: false,
        first_correction_false_accept: false,
        candidate_counters: WorkCounters::default(),
        certificate_counters: WorkCounters::default(),
        batch_depth: 0,
        batch_vectors: 0,
        compute_seconds: 0.0,
        certificate_seconds: 0.0,
        decision_reason: None,
        failure: None,
    };
    assert!(row.validate().is_err());
}

#[test]
fn protected_oracle_fallback_can_certify_research_output_when_c3_exhausts() {
    use rodas5p_core::WorkCounters;
    use rodas5p_integrators::{
        CandidateFamily, ReferenceCertificateSource, UnifiedCandidateOutcome, UnifiedCandidateRow,
    };

    let row = UnifiedCandidateRow {
        candidate_id: "c0-fallback".into(),
        family: CandidateFamily::Homotopy,
        case_id: "case".into(),
        outcome: UnifiedCandidateOutcome::Completed,
        scientifically_certified: true,
        reference_certificate_source: ReferenceCertificateSource::C0ProtectedOracleFallback,
        reference_fallback_used: true,
        certificate_failure: None,
        used_fallback: false,
        embedded_error: Some(0.0),
        oracle_output_wrms: Some(0.01),
        oracle_stage_l2: Some(0.01),
        first_output_wrms: Some(0.01),
        second_output_wrms: Some(0.01),
        second_output_ratio: Some(0.5),
        second_residual_ratio: Some(0.5),
        second_contraction_evidence: Some(true),
        refined_root_converged: Some(false),
        refined_root_termination: Some("iteration budget exhausted".into()),
        refined_root_output_wrms: None,
        refined_root_relative_residual: Some(1.0),
        output_budget: 0.1,
        c3_output_budget_pass: None,
        oracle_output_budget_pass: Some(true),
        c3_false_accept: false,
        first_correction_false_accept: false,
        candidate_counters: WorkCounters::default(),
        certificate_counters: WorkCounters::default(),
        batch_depth: 2,
        batch_vectors: 16,
        compute_seconds: 0.0,
        certificate_seconds: 0.0,
        decision_reason: Some("C3 unavailable; C0 protected oracle certified research row".into()),
        failure: None,
    };
    row.validate().unwrap();
}

#[test]
fn reference_budget_pass_does_not_override_an_independent_candidate_rejection() {
    use rodas5p_core::WorkCounters;
    use rodas5p_integrators::{
        CandidateFamily, ReferenceCertificateSource, UnifiedCandidateOutcome, UnifiedCandidateRow,
    };

    let row = UnifiedCandidateRow {
        candidate_id: "candidate-rejected".into(),
        family: CandidateFamily::Sabr,
        case_id: "case".into(),
        outcome: UnifiedCandidateOutcome::Rejected,
        scientifically_certified: true,
        reference_certificate_source: ReferenceCertificateSource::C0ProtectedOracleFallback,
        reference_fallback_used: true,
        certificate_failure: None,
        used_fallback: false,
        embedded_error: Some(2.0),
        oracle_output_wrms: Some(0.01),
        oracle_stage_l2: Some(0.01),
        first_output_wrms: Some(0.01),
        second_output_wrms: Some(0.01),
        second_output_ratio: Some(0.5),
        second_residual_ratio: Some(0.5),
        second_contraction_evidence: Some(true),
        refined_root_converged: Some(false),
        refined_root_termination: Some("iteration budget exhausted".into()),
        refined_root_output_wrms: None,
        refined_root_relative_residual: Some(1.0),
        output_budget: 0.1,
        c3_output_budget_pass: None,
        oracle_output_budget_pass: Some(true),
        c3_false_accept: false,
        first_correction_false_accept: false,
        candidate_counters: WorkCounters::default(),
        certificate_counters: WorkCounters::default(),
        batch_depth: 2,
        batch_vectors: 16,
        compute_seconds: 0.0,
        certificate_seconds: 0.0,
        decision_reason: Some("candidate step rejected".into()),
        failure: None,
    };
    row.validate().unwrap();
}

#[test]
fn c3_budget_pass_does_not_override_an_independent_candidate_rejection() {
    use rodas5p_core::WorkCounters;
    use rodas5p_integrators::{
        CandidateFamily, ReferenceCertificateSource, UnifiedCandidateOutcome, UnifiedCandidateRow,
    };

    let row = UnifiedCandidateRow {
        candidate_id: "candidate-rejected-c3".into(),
        family: CandidateFamily::Sabr,
        case_id: "case".into(),
        outcome: UnifiedCandidateOutcome::Rejected,
        scientifically_certified: true,
        reference_certificate_source: ReferenceCertificateSource::C3RefinedRoot,
        reference_fallback_used: false,
        certificate_failure: None,
        used_fallback: false,
        embedded_error: Some(2.0),
        oracle_output_wrms: Some(0.01),
        oracle_stage_l2: Some(0.01),
        first_output_wrms: Some(0.01),
        second_output_wrms: Some(0.01),
        second_output_ratio: Some(0.5),
        second_residual_ratio: Some(0.5),
        second_contraction_evidence: Some(true),
        refined_root_converged: Some(true),
        refined_root_termination: Some("residual and correction tolerances satisfied".into()),
        refined_root_output_wrms: Some(0.01),
        refined_root_relative_residual: Some(1.0e-12),
        output_budget: 0.1,
        c3_output_budget_pass: Some(true),
        oracle_output_budget_pass: Some(true),
        c3_false_accept: false,
        first_correction_false_accept: false,
        candidate_counters: WorkCounters::default(),
        certificate_counters: WorkCounters::default(),
        batch_depth: 2,
        batch_vectors: 16,
        compute_seconds: 0.0,
        certificate_seconds: 0.0,
        decision_reason: Some("candidate step rejected".into()),
        failure: None,
    };
    row.validate().unwrap();
}

#[test]
fn unified_candidate_row_rejects_certified_failure_state() {
    use rodas5p_core::WorkCounters;
    use rodas5p_integrators::{CandidateFamily, UnifiedCandidateOutcome, UnifiedCandidateRow};

    let row = UnifiedCandidateRow {
        candidate_id: "invalid".into(),
        family: CandidateFamily::Homotopy,
        case_id: "case".into(),
        outcome: UnifiedCandidateOutcome::NumericalFailure,
        scientifically_certified: true,
        reference_certificate_source:
            rodas5p_integrators::ReferenceCertificateSource::C3RefinedRoot,
        reference_fallback_used: false,
        certificate_failure: None,
        used_fallback: false,
        embedded_error: None,
        oracle_output_wrms: Some(0.0),
        oracle_stage_l2: Some(0.0),
        first_output_wrms: Some(0.0),
        second_output_wrms: Some(0.0),
        second_output_ratio: Some(0.0),
        second_residual_ratio: Some(0.0),
        second_contraction_evidence: Some(true),
        refined_root_converged: Some(true),
        refined_root_termination: Some("residual and correction tolerances satisfied".into()),
        refined_root_output_wrms: Some(0.0),
        refined_root_relative_residual: Some(0.0),
        output_budget: 0.1,
        c3_output_budget_pass: Some(true),
        oracle_output_budget_pass: Some(true),
        c3_false_accept: false,
        first_correction_false_accept: false,
        candidate_counters: WorkCounters::default(),
        certificate_counters: WorkCounters::default(),
        batch_depth: 0,
        batch_vectors: 0,
        compute_seconds: 0.0,
        certificate_seconds: 0.0,
        decision_reason: None,
        failure: Some("numerical failure".into()),
    };
    assert!(row.validate().is_err());
}

#[test]
fn unified_smoke_screen_runs_all_current_nonlinear_families_against_one_oracle() {
    use std::collections::{BTreeMap, BTreeSet};

    use rodas5p_integrators::{
        CandidateFamily, UnifiedScreenProfile, run_unified_nonlinear_screen,
    };

    let report = run_unified_nonlinear_screen(UnifiedScreenProfile::Smoke, 1).unwrap();
    assert!(!report.rows.is_empty());
    assert_eq!(report.summary.failures, 0);

    let mut families_by_case: BTreeMap<&str, BTreeSet<CandidateFamily>> = BTreeMap::new();
    for row in &report.rows {
        row.validate().unwrap();
        families_by_case
            .entry(&row.case_id)
            .or_default()
            .insert(row.family);
    }
    for families in families_by_case.values() {
        assert!(families.contains(&CandidateFamily::Sequential));
        assert!(families.contains(&CandidateFamily::Sabr));
        assert!(families.contains(&CandidateFamily::Homotopy));
    }
    assert!(
        report
            .rows
            .iter()
            .any(|row| row.candidate_id == "sequential-lgmres-stage")
    );
    assert!(
        report
            .rows
            .iter()
            .any(|row| row.candidate_id == "sequential-gcrodr-persistent")
    );
}

#[test]
fn unified_smoke_screen_is_scientifically_identical_across_one_and_four_threads() {
    use rodas5p_integrators::{UnifiedScreenProfile, run_unified_nonlinear_screen};

    let one = run_unified_nonlinear_screen(UnifiedScreenProfile::Smoke, 1).unwrap();
    let four = run_unified_nonlinear_screen(UnifiedScreenProfile::Smoke, 4).unwrap();
    assert_eq!(one.cases, four.cases);
    assert_eq!(one.summary, four.summary);
    assert_eq!(one.rows.len(), four.rows.len());
    for (left, right) in one.rows.iter().zip(&four.rows) {
        let mut left = left.clone();
        let mut right = right.clone();
        left.compute_seconds = 0.0;
        right.compute_seconds = 0.0;
        left.certificate_seconds = 0.0;
        right.certificate_seconds = 0.0;
        assert_eq!(left, right);
    }
}

#[test]
fn unified_screen_applies_the_same_c3_output_budget_to_every_nonlinear_candidate() {
    use rodas5p_integrators::{
        UnifiedCandidateOutcome, UnifiedScreenProfile, run_unified_nonlinear_screen,
    };

    let report = run_unified_nonlinear_screen(UnifiedScreenProfile::Smoke, 1).unwrap();
    assert_eq!(report.output_budget, 0.1);
    for row in &report.rows {
        row.validate().unwrap();
        if row.c3_output_budget_pass == Some(true) {
            assert!(matches!(
                row.outcome,
                UnifiedCandidateOutcome::Completed | UnifiedCandidateOutcome::CompletedWithFallback
            ));
        }
        if row.c3_output_budget_pass == Some(false) {
            assert_eq!(row.outcome, UnifiedCandidateOutcome::Rejected);
        }
        assert!(!row.c3_false_accept, "{}", row.candidate_id);
    }
}

#[test]
fn unified_scientific_gates_distinguish_protected_fifth_order_from_low_order_fast_path() {
    use rodas5p_integrators::{
        UnifiedScreenProfile, run_unified_nonlinear_screen, run_unified_scientific_gates,
    };

    let nonlinear = run_unified_nonlinear_screen(UnifiedScreenProfile::Smoke, 1).unwrap();
    let gates = run_unified_scientific_gates(UnifiedScreenProfile::Smoke, 1, &nonlinear).unwrap();
    let protected = gates
        .candidates
        .iter()
        .find(|row| row.candidate_id == "sequential-direct-off")
        .unwrap();
    assert!(protected.order_pass);
    assert!(protected.stiff_decay_pass);

    let low_order = gates
        .candidates
        .iter()
        .find(|row| row.candidate_id == "homotopy-theta0.000-q0-r2-ab2-c0")
        .unwrap();
    assert!(!low_order.order_pass);
}

#[test]
fn unified_scientific_gates_are_thread_deterministic() {
    use rodas5p_integrators::{
        UnifiedScreenProfile, run_unified_nonlinear_screen, run_unified_scientific_gates,
    };

    let nonlinear = run_unified_nonlinear_screen(UnifiedScreenProfile::Smoke, 1).unwrap();
    let mut one = run_unified_scientific_gates(UnifiedScreenProfile::Smoke, 1, &nonlinear).unwrap();
    let mut four =
        run_unified_scientific_gates(UnifiedScreenProfile::Smoke, 4, &nonlinear).unwrap();
    one.compute_seconds = 0.0;
    four.compute_seconds = 0.0;
    one.threads = 0;
    four.threads = 0;
    assert_eq!(one, four);
}

#[test]
fn refined_root_certificate_accepts_an_already_converged_protected_stage_vector() {
    use rodas5p_core::{LinearMethod, LinearSolverConfig, WorkCounters};
    use rodas5p_integrators::{
        RefinedRootConfig, StructuredBlockSystem, build_step_context, manufactured_vector_problem,
        refine_target_root, sequential_stages,
    };

    let (problem, y0) = manufactured_vector_problem(4, 1_000.0, 100.0, 0.2, 0.0).unwrap();
    let mut counters = WorkCounters::default();
    let context = build_step_context(&problem, 0.0, &y0, 0.002, &mut counters).unwrap();
    let direct = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let stages = sequential_stages(&context, &direct, None, &mut counters)
        .unwrap()
        .stages;
    let certificate = refine_target_root(
        &StructuredBlockSystem::new(&context),
        &stages,
        1e-7,
        1e-6,
        &RefinedRootConfig::default(),
        &mut counters,
    )
    .unwrap();
    assert!(certificate.converged);
    assert_eq!(certificate.iterations, 0);
    assert_eq!(certificate.last_correction_wrms, 0.0);
    assert!(certificate.candidate_output_wrms <= 1e-8);
}
