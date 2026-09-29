//! Run the n = 96 calibration rows under the two-arm admissibility protocol v3
//! with manufactured exact references (audit F-007 part C, F-033).
//!
//!     cargo run --release -p rodas5p-fair-ab --example two_arm_v3_calibration96 -- OUT.json
//!
//! The references are the families' exact solutions, so reference
//! uncertainty is zero. Records are typed `SyntheticCiSmoke`: this is
//! diagnostic evidence, never a canonical campaign row.

use rodas5p_fair_ab::{
    CommonOutputGrid, ExternalErrorScale, NUMERICAL_REFERENCE_V2_ARTIFACT_SCHEMA_VERSION,
    NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION, NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID,
    NumericalReferenceBundleV2, NumericalReferenceChecksums, NumericalReferenceConvergence,
    NumericalReferenceGeneratorPins, NumericalReferenceMethod, NumericalReferenceProvenance,
    NumericalReferenceWrmsScale, ReferenceSolutionProvenance, ReferenceSourceKind,
    ReferenceTrajectory, ReferenceWrmsBasis, numerical_reference_grid_checksum,
    numerical_reference_state_checksum, run_two_arm_admissibility_v3_case_synthetic_smoke,
    scientific_validity_v2_detected_revision,
};
use rodas5p_integrators::ScientificCorpusV2;

fn method(label: &str, method: &str, rtol: f64, atol: f64) -> NumericalReferenceMethod {
    NumericalReferenceMethod {
        label: label.into(),
        method: method.into(),
        rtol,
        atol,
    }
}

fn exact_reference(
    spec: &rodas5p_integrators::ScientificCaseSpec,
) -> Option<NumericalReferenceBundleV2> {
    let case = spec.build().unwrap();
    let states = spec
        .output_times
        .iter()
        .map(|time| case.problem.exact(*time))
        .collect::<Option<Vec<_>>>()?;
    let output_grid = CommonOutputGrid::new(spec.output_times.clone()).unwrap();
    let state_sha256 = numerical_reference_state_checksum(&states);
    let grid_sha256 = numerical_reference_grid_checksum(&spec.output_times);
    let error_scale =
        ExternalErrorScale::with_reference_uncertainty(vec![1.0e-10; spec.dimension], 1.0e-8, 0.0)
            .unwrap();
    let canonical = method("manufactured-exact", "exact", 0.0, 0.0);
    let independent = method("manufactured-exact", "exact", 0.0, 0.0);
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
        problem_id: format!("manufactured-exact:{}", spec.id),
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
    Some(NumericalReferenceBundleV2 {
        case_id: spec.id.clone(),
        problem_id: trajectory.provenance.problem_id.clone(),
        reference_checksum_sha256,
        implementation_revision: scientific_validity_v2_detected_revision().into(),
        trajectory,
        error_scale,
        wrms_basis,
    })
}

fn main() {
    let out = std::env::args().nth(1).expect("output path");
    let mut records = Vec::new();
    for spec in ScientificCorpusV2::calibration_specs()
        .into_iter()
        .filter(|spec| spec.dimension == 96)
    {
        let Some(reference) = exact_reference(&spec) else {
            eprintln!("{:<60} SKIPPED: no manufactured exact solution", spec.id);
            continue;
        };
        let record =
            run_two_arm_admissibility_v3_case_synthetic_smoke(&spec, &reference, true).unwrap();
        let row = record.admissibility.as_ref();
        eprintln!(
            "{:<60} status={:?} clipped={:.3e} dense={:.3e} delta={:?} r_inner={:?} ({:.1}s)",
            record.case_id,
            row.map(|row| row.status),
            row.map_or(f64::NAN, |row| row.clipped_case_wrms),
            row.map_or(f64::NAN, |row| row.dense_case_wrms),
            record.interpolant.max_delta_wrms,
            record.attribution.as_ref().and_then(|a| a.r_inner),
            record.wall_seconds,
        );
        records.push(record);
    }
    std::fs::write(&out, serde_json::to_vec_pretty(&records).unwrap()).unwrap();
}
