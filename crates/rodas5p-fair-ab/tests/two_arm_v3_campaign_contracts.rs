//! Two-arm admissibility runner v3 (audit F-007 parts A-C, F-033 attribution).

use rodas5p_fair_ab::{
    CommonOutputGrid, ExternalErrorScale, NUMERICAL_REFERENCE_V2_ARTIFACT_SCHEMA_VERSION,
    NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION, NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID,
    NumericalReferenceBundleV2, NumericalReferenceChecksums, NumericalReferenceConvergence,
    NumericalReferenceGeneratorPins, NumericalReferenceMethod, NumericalReferenceProvenance,
    NumericalReferenceWrmsScale, ReferenceSolutionProvenance, ReferenceSourceKind,
    ReferenceTrajectory, ReferenceWrmsBasis, TWO_ARM_ADMISSIBILITY_PROTOCOL_ID,
    TWO_ARM_V3_CASE_SCHEMA, TwoArmV3AttributionReading, V2CampaignArmStatus,
    numerical_reference_grid_checksum, numerical_reference_state_checksum,
    run_scientific_validity_v2_case_synthetic_smoke,
    run_two_arm_admissibility_v3_case_synthetic_smoke, scientific_validity_v2_detected_revision,
    two_arm_v3_record_checksum,
};
use rodas5p_integrators::{ScientificCorpusV2, ScientificFamily, V2EvidenceAuthority};

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

fn spec(family: ScientificFamily, rtol: f64) -> rodas5p_integrators::ScientificCaseSpec {
    ScientificCorpusV2::calibration_specs()
        .into_iter()
        .find(|spec| {
            spec.family == family && spec.dimension == 96 && spec.rtol.to_bits() == rtol.to_bits()
        })
        .unwrap()
}

#[test]
fn v3_record_runs_all_three_parts_on_unchanged_v2_arms() {
    let spec = spec(ScientificFamily::RotatingNonnormal, 1.0e-4);
    let reference = exact_reference(&spec);
    let record =
        run_two_arm_admissibility_v3_case_synthetic_smoke(&spec, &reference, true).unwrap();
    assert_eq!(record.schema_version, TWO_ARM_V3_CASE_SCHEMA);
    assert_eq!(record.protocol_id, TWO_ARM_ADMISSIBILITY_PROTOCOL_ID);
    assert_eq!(record.authority, V2EvidenceAuthority::SyntheticCiSmoke);
    assert_eq!(record.clipped.status, V2CampaignArmStatus::Success);
    assert_eq!(record.dense.status, V2CampaignArmStatus::Success);

    // Part C ran on every interior output time and was charged separately.
    assert!(record.interpolant.interior_samples > 0);
    assert_eq!(record.interpolant.failed_substeps, 0);
    let delta = record.interpolant.max_delta_wrms.unwrap();
    assert!(delta.is_finite());
    assert!(record.interpolant.audit_counters.rhs_evaluations > 0);

    // Parts A/B classified the pair with the audit's delta; no row is left
    // InterpolantUnchecked.
    let row = record.admissibility.as_ref().unwrap();
    assert_eq!(row.interpolant_delta, Some(delta));
    assert!(row.gap_case_wrms <= row.clipped_case_wrms + row.dense_case_wrms);

    // The arms are the v2 arms, bit for bit.
    let v2 = run_scientific_validity_v2_case_synthetic_smoke(&spec, &reference).unwrap();
    assert_eq!(
        record.clipped.output_checksum_sha256,
        v2.clipped.output_checksum_sha256
    );
    assert_eq!(
        record.dense.output_checksum_sha256,
        v2.dense.output_checksum_sha256
    );
    assert_eq!(record.dense.counters, v2.dense.counters);

    // F-033 attribution arm: fixed inner tolerance, no forcing.
    let attribution = record.attribution.as_ref().unwrap();
    assert_eq!(attribution.tight_inner.status, V2CampaignArmStatus::Success);
    assert_eq!(attribution.tight_inner.counters.forced_stage_solves, 0);
    let ratio = attribution.r_inner.unwrap();
    assert!(ratio.is_finite() && ratio > 0.0);
    assert_ne!(attribution.reading, TwoArmV3AttributionReading::Unavailable);

    assert_eq!(
        two_arm_v3_record_checksum(&record).unwrap(),
        record.record_checksum_sha256
    );
}

#[test]
fn v3_record_is_reproducible_bit_for_bit() {
    let spec = spec(ScientificFamily::RotatingNonnormal, 1.0e-4);
    let reference = exact_reference(&spec);
    let a = run_two_arm_admissibility_v3_case_synthetic_smoke(&spec, &reference, false).unwrap();
    let b = run_two_arm_admissibility_v3_case_synthetic_smoke(&spec, &reference, false).unwrap();
    assert_eq!(a.record_checksum_sha256, b.record_checksum_sha256);
    assert!(a.attribution.is_none());
}
