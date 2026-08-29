use rodas5p_fair_ab::{
    CommonOutputGrid, ExternalErrorScale, NUMERICAL_REFERENCE_V2_ARTIFACT_SCHEMA_VERSION,
    NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION, NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID,
    NumericalReferenceBundleV2, NumericalReferenceChecksums, NumericalReferenceConvergence,
    NumericalReferenceGeneratorPins, NumericalReferenceMethod, NumericalReferenceProvenance,
    NumericalReferenceWrmsScale, ReferenceSolutionProvenance, ReferenceSourceKind,
    ReferenceTrajectory, ReferenceWrmsBasis, SCIENTIFIC_VALIDITY_V2_CANDIDATE_ID,
    SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM, V2CampaignArmStatus,
    numerical_reference_grid_checksum, numerical_reference_state_checksum,
    run_scientific_validity_v2_case, run_scientific_validity_v2_case_synthetic_smoke,
    scientific_validity_v2_compiled_revision, scientific_validity_v2_detected_revision,
    scientific_validity_v2_source_dirty_at_build, validate_scientific_validity_v2_case_artifact,
};
use rodas5p_integrators::{ScientificCorpusV2, ScientificFamily, V2GateRowStatus};
use std::collections::BTreeSet;

#[test]
fn calibration_execution_surface_is_exactly_six_by_three_by_three() {
    let specs = rodas5p_fair_ab::scientific_validity_v2_campaign_specs(
        rodas5p_integrators::CorpusPartition::Calibration,
    );
    assert_eq!(specs.len(), 54);
    assert_eq!(
        specs
            .iter()
            .map(|spec| spec.id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        54
    );
    for family in ScientificFamily::CALIBRATION {
        for dimension in ScientificCorpusV2::calibration_dimensions() {
            assert_eq!(
                specs
                    .iter()
                    .filter(|spec| spec.family == family && spec.dimension == *dimension)
                    .count(),
                3
            );
        }
    }
}

fn method(label: &str, method: &str, rtol: f64, atol: f64) -> NumericalReferenceMethod {
    NumericalReferenceMethod {
        label: label.into(),
        method: method.into(),
        rtol,
        atol,
    }
}

fn exact_reference(spec: &rodas5p_integrators::ScientificCaseSpec) -> NumericalReferenceBundleV2 {
    let case = spec.build().unwrap();
    let states = spec
        .output_times
        .iter()
        .map(|time| case.problem.exact(*time).expect("manufactured exact state"))
        .collect::<Vec<_>>();
    let output_grid = CommonOutputGrid::new(spec.output_times.clone()).unwrap();
    let state_sha256 = numerical_reference_state_checksum(&states);
    let grid_sha256 = numerical_reference_grid_checksum(&spec.output_times);
    let error_scale =
        ExternalErrorScale::with_reference_uncertainty(vec![1.0e-10; spec.dimension], 1.0e-8, 0.0)
            .unwrap();
    let canonical = method("L2", "Radau", 1.0e-12, 1.0e-14);
    let independent = method("LSODA-tight", "LSODA", 1.0e-12, 1.0e-14);
    let convergence = NumericalReferenceConvergence {
        d0_max_grid_wrms: 1.0,
        d1_max_grid_wrms: 0.0,
        q: 0.0,
        richardson_uncertainty_wrms: 0.0,
        method_disagreement_wrms: 0.0,
        reference_uncertainty_wrms: 0.0,
        wrms_scale: NumericalReferenceWrmsScale {
            absolute: 1.0e-10,
            relative: 1.0e-8,
        },
    };
    let reference_checksum_sha256 = "b".repeat(64);
    let numerical = NumericalReferenceProvenance {
        manifest_schema_version: NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION.into(),
        artifact_schema_version: NUMERICAL_REFERENCE_V2_ARTIFACT_SCHEMA_VERSION.into(),
        artifact_sha256: "a".repeat(64),
        source_definition_id: spec.provenance.source_path.clone(),
        generator: NumericalReferenceGeneratorPins {
            python: "3.12".into(),
            numpy: "2.4.2".into(),
            scipy: "1.17.0".into(),
            blas_threads: 1,
            radau_ladder: vec![
                method("L0", "Radau", 1.0e-8, 1.0e-10),
                method("L1", "Radau", 1.0e-10, 1.0e-12),
                canonical.clone(),
            ],
            tight_lsoda: independent.clone(),
        },
        canonical_method: canonical,
        independent_method: independent,
        checksums: NumericalReferenceChecksums {
            grid_sha256,
            state_sha256: state_sha256.clone(),
        },
        convergence,
        corpus_version: Some(ScientificCorpusV2::VERSION.into()),
        case_id: Some(spec.id.clone()),
        reference_checksum_sha256: Some(reference_checksum_sha256.clone()),
        wrms_formula_id: Some(NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID.into()),
        anchor_state_sha256: Some(state_sha256.clone()),
    };
    let provenance = ReferenceSolutionProvenance {
        problem_id: format!("synthetic-smoke:{}", spec.id),
        source_kind: ReferenceSourceKind::HighAccuracyNumerical,
        output_grid_id: output_grid.grid_id.clone(),
        state_checksum: state_sha256,
        reference_uncertainty_wrms: 0.0,
        numerical: Some(numerical),
    };
    let trajectory = ReferenceTrajectory {
        output_grid: output_grid.clone(),
        states: states.clone(),
        provenance,
    };
    let wrms_basis = ReferenceWrmsBasis::new(output_grid, states, error_scale.clone()).unwrap();
    NumericalReferenceBundleV2 {
        case_id: spec.id.clone(),
        problem_id: trajectory.provenance.problem_id.clone(),
        reference_checksum_sha256,
        implementation_revision: scientific_validity_v2_detected_revision().into(),
        trajectory,
        error_scale,
        wrms_basis,
    }
}

#[test]
fn one_real_calibration_case_runs_independent_clipped_and_dense_arms() {
    let Some(supplied_revision) = option_env!("VIGILODE_CODE_REVISION") else {
        assert!(scientific_validity_v2_compiled_revision().is_err());
        eprintln!("NOT_RUN: campaign smoke requires VIGILODE_CODE_REVISION at build time");
        return;
    };
    assert_eq!(
        supplied_revision,
        scientific_validity_v2_detected_revision(),
        "the campaign smoke must bind the detected checkout revision"
    );
    let spec = ScientificCorpusV2::calibration_specs()
        .into_iter()
        .find(|spec| {
            spec.family == ScientificFamily::RotatingNonnormal
                && spec.dimension == 96
                && spec.rtol.to_bits() == 1.0e-4_f64.to_bits()
        })
        .unwrap();
    let reference = exact_reference(&spec);
    let artifact = if scientific_validity_v2_source_dirty_at_build() {
        assert!(
            scientific_validity_v2_compiled_revision().is_err(),
            "a dirty-tree exercise must never acquire canonical authority"
        );
        run_scientific_validity_v2_case_synthetic_smoke(&spec, &reference).unwrap()
    } else {
        assert_eq!(
            scientific_validity_v2_compiled_revision().unwrap(),
            supplied_revision
        );
        run_scientific_validity_v2_case(&spec, &reference).unwrap()
    };
    validate_scientific_validity_v2_case_artifact(&artifact).unwrap();

    assert_eq!(artifact.candidate_id, SCIENTIFIC_VALIDITY_V2_CANDIDATE_ID);
    assert_eq!(
        artifact.row.binding.campaign.authority,
        if scientific_validity_v2_source_dirty_at_build() {
            rodas5p_integrators::V2EvidenceAuthority::SyntheticCiSmoke
        } else {
            rodas5p_integrators::V2EvidenceAuthority::CanonicalV2Runner
        }
    );
    assert_eq!(artifact.config.max_attempts_per_arm, 200_000);
    assert_eq!(artifact.config.min_step.to_bits(), 1.0e-12_f64.to_bits());
    assert_eq!(artifact.config.method, "RODAS5P");
    assert_eq!(artifact.config.linear_method, "GMRES");
    assert_eq!(
        artifact.config.inner_tolerance_policy,
        "wrms-stage-residual-heuristic-v2"
    );
    assert!(
        artifact
            .config
            .inner_solve_claim_scope
            .contains("requires an independent W-inverse resolvent certificate")
    );
    assert!(
        artifact
            .config
            .cross_step_recycle_image_policy
            .contains("refresh-per-linearization")
    );
    assert_eq!(artifact.config.restart, 32);
    assert_eq!(artifact.config.max_arnoldi, 256);
    assert_eq!(artifact.config.inner_m, 30);
    assert_eq!(artifact.config.outer_k, 8);
    assert_eq!(artifact.config.recycle_dim, 8);
    assert_eq!(
        artifact.config.recycle_rank_tolerance.to_bits(),
        1.0e-12_f64.to_bits()
    );
    assert_eq!(
        artifact.config.fallback_inner_atol.to_bits(),
        1.0e-12_f64.to_bits()
    );
    assert_eq!(
        artifact.config.fallback_inner_rtol.to_bits(),
        1.0e-10_f64.to_bits()
    );
    assert_eq!(artifact.config.preconditioner, "none");
    assert_eq!(artifact.config.initial_guess, "previous");
    assert_eq!(artifact.config.controller, "integral");
    assert_eq!(
        artifact.config.controller_safety.to_bits(),
        0.9_f64.to_bits()
    );
    assert_eq!(
        artifact.config.controller_min_factor.to_bits(),
        0.2_f64.to_bits()
    );
    assert_eq!(
        artifact.config.controller_max_factor.to_bits(),
        5.0_f64.to_bits()
    );
    assert_eq!(
        artifact.config.controller_reject_max_factor.to_bits(),
        0.9_f64.to_bits()
    );
    assert_eq!(artifact.clipped.status, V2CampaignArmStatus::Success);
    assert_eq!(artifact.dense.status, V2CampaignArmStatus::Success);
    assert_eq!(artifact.clipped.output_times, spec.output_times);
    assert_eq!(artifact.dense.output_times, spec.output_times);
    assert!(artifact.clipped.output_clipped_steps > 0);
    assert_eq!(artifact.dense.output_clipped_steps, 0);
    assert!(artifact.clipped.counters.forced_stage_solves > 0);
    assert!(artifact.dense.counters.forced_stage_solves > 0);
    assert!(artifact.clipped.diagnostics.attempts <= SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM);
    assert!(artifact.dense.diagnostics.attempts <= SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM);
    assert_ne!(
        artifact.clipped.output_checksum_sha256, artifact.dense.output_checksum_sha256,
        "the independent output modes are domain separated even if states happen to coincide"
    );
    assert!(matches!(
        artifact.row.status,
        V2GateRowStatus::Pass | V2GateRowStatus::OutputPolicyDominated
    ));
    eprintln!(
        "SMOKE row={:?} dense_conservative_wrms={:.17e} gap_wrms={:.17e} clipped_steps={} dense_steps={} clipped_jvp={} dense_jvp={}",
        artifact.row.status,
        artifact
            .dense
            .metrics
            .as_ref()
            .unwrap()
            .conservative_max_wrms,
        artifact.output_policy_discrepancy_wrms.unwrap(),
        artifact.clipped.internal_steps,
        artifact.dense.internal_steps,
        artifact.clipped.counters.jvp_vectors,
        artifact.dense.counters.jvp_vectors,
    );

    let mut operational_retime = artifact.clone();
    operational_retime.clipped.wall_seconds += 10.0;
    operational_retime.dense.wall_seconds += 20.0;
    operational_retime.row.wall_seconds = Some(123.0);
    validate_scientific_validity_v2_case_artifact(&operational_retime).unwrap();

    let mut wrong_revision = artifact.clone();
    wrong_revision.code_revision = "2".repeat(40);
    assert!(validate_scientific_validity_v2_case_artifact(&wrong_revision).is_err());

    let mut diagnostics = artifact.clone();
    diagnostics.clipped.diagnostics.attempts += 1;
    assert!(validate_scientific_validity_v2_case_artifact(&diagnostics).is_err());

    let mut counters = artifact.clone();
    counters.clipped.counters.accepted_steps += 1;
    assert!(validate_scientific_validity_v2_case_artifact(&counters).is_err());

    let mut internal_steps = artifact.clone();
    internal_steps.dense.internal_steps += 1;
    assert!(validate_scientific_validity_v2_case_artifact(&internal_steps).is_err());

    let mut empty_prefix = artifact.clone();
    empty_prefix.clipped.output_times.clear();
    empty_prefix.clipped.states.clear();
    empty_prefix.clipped.committed_output_count = 0;
    assert!(validate_scientific_validity_v2_case_artifact(&empty_prefix).is_err());

    let mut wrong_dimension = artifact.clone();
    wrong_dimension.dense.states[0].pop();
    assert!(validate_scientific_validity_v2_case_artifact(&wrong_dimension).is_err());

    let mut nonfinite = artifact.clone();
    nonfinite.clipped.states[0][0] = f64::NAN;
    assert!(validate_scientific_validity_v2_case_artifact(&nonfinite).is_err());
}
