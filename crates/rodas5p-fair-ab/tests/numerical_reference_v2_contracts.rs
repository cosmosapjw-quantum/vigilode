use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use rodas5p_core::sha256_hex;
use rodas5p_fair_ab::{
    CommonOutputGrid, ExternalErrorScale, NUMERICAL_REFERENCE_MANIFEST_SCHEMA_VERSION,
    NUMERICAL_REFERENCE_V2_ARTIFACT_SCHEMA_VERSION, NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION,
    NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID, NumericalReferenceArtifactV2,
    NumericalReferenceConvergenceV2, NumericalReferenceGenerationStatusV2,
    NumericalReferenceRunEvidenceV2, NumericalReferenceRunStatusV2, NumericalReferenceWrmsBasisV2,
    ReferenceWrmsBasis, load_numerical_reference, load_numerical_reference_v2,
    numerical_reference_artifact_set_checksum_v2, numerical_reference_binding_set_checksum_v2,
    numerical_reference_case_binding_checksum_v2, numerical_reference_grid_checksum,
    numerical_reference_state_checksum, numerical_reference_v2_not_run_manifest,
    validate_numerical_reference_manifest_v2,
};
use rodas5p_integrators::{ScientificCorpusV2, ScientificFamily};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn root() -> PathBuf {
    let suffix = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "vigilode-numerical-reference-v2-{}-{suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn refresh_manifest_digests(manifest: &mut rodas5p_fair_ab::NumericalReferenceManifestV2) {
    for binding in &mut manifest.bindings {
        let entry = manifest
            .artifacts
            .iter()
            .find(|entry| entry.problem.problem_id == binding.problem_id)
            .unwrap();
        binding.reference_checksum_sha256 =
            numerical_reference_case_binding_checksum_v2(binding, entry);
    }
    manifest.artifact_set_sha256 =
        numerical_reference_artifact_set_checksum_v2(&manifest.artifacts);
    manifest.binding_set_sha256 = numerical_reference_binding_set_checksum_v2(&manifest.bindings);
}

#[test]
fn v2_layout_is_exactly_twenty_two_physical_artifacts_and_sixty_six_case_bindings() {
    let manifest = numerical_reference_v2_not_run_manifest().unwrap();
    assert_eq!(
        manifest.schema_version,
        NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(manifest.corpus_version, ScientificCorpusV2::VERSION);
    assert_eq!(manifest.artifacts.len(), 22);
    assert_eq!(manifest.bindings.len(), 66);
    validate_numerical_reference_manifest_v2(&manifest).unwrap();

    let expected_cases = ScientificCorpusV2::all_specs()
        .into_iter()
        .map(|spec| spec.id)
        .collect::<std::collections::BTreeSet<_>>();
    let actual_cases = manifest
        .bindings
        .iter()
        .map(|binding| binding.case_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual_cases, expected_cases);
    assert!(actual_cases.iter().any(|id| id.contains("rtol-1e-4-v2.1")));
    assert!(actual_cases.iter().all(|id| !id.contains("1e-04")));

    for artifact in &manifest.artifacts {
        assert_eq!(
            manifest
                .bindings
                .iter()
                .filter(|binding| binding.problem_id == artifact.problem.problem_id)
                .count(),
            3,
            "{}",
            artifact.problem.problem_id
        );
        let siblings = manifest
            .bindings
            .iter()
            .filter(|binding| binding.problem_id == artifact.problem.problem_id)
            .collect::<Vec<_>>();
        assert_eq!(
            siblings
                .iter()
                .map(|binding| binding.reference_checksum_sha256.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3,
            "the exact case id must participate in each binding digest"
        );
        assert_eq!(
            artifact.problem.grid_shape.is_some(),
            artifact.problem.family
                == ScientificFamily::SemilinearAdvectionDiffusionRamped.as_str()
        );
    }
}

#[test]
fn checked_in_python_not_run_manifest_is_bit_for_bit_semantically_equal_to_the_rust_layout() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/reference_v2/artifacts_v2/reference_manifest_v2.json");
    let from_python: rodas5p_fair_ab::NumericalReferenceManifestV2 =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let from_rust = numerical_reference_v2_not_run_manifest().unwrap();
    assert_eq!(from_python, from_rust);
}

#[test]
fn historical_v1_manifest_and_artifact_bytes_remain_pinned() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/reference_v2");
    for (relative, expected) in [
        (
            "reference_manifest.json",
            "27e878e3b135eea734e05a273388a07a7d52174a62d21418d9386fa1fcdf1d27",
        ),
        (
            "artifacts/brusselator-2d-holdout-v2.json",
            "9a3229fa997264824705ee0f98ea68b77dffbffe6bf8542e94b9aebb7327e9b4",
        ),
        (
            "artifacts/medical-akzo-holdout-v2.json",
            "25f0b7953f889165690f1f9762cd4dad777220e0aee5faad95361c6772c53d86",
        ),
        (
            "artifacts/oregonator-holdout-v2.json",
            "0dc7b940322c7579113aadfde56423628753c41dd84f2c322499177b380df3b1",
        ),
        (
            "artifacts/pollution-holdout-v2.json",
            "3201ebece2e9a365c5ff20d14f28413a316a35708924e22fd89ebe2a0f28109a",
        ),
    ] {
        assert_eq!(
            sha256_hex(&fs::read(root.join(relative)).unwrap()),
            expected
        );
    }
}

#[test]
fn v2_manifest_rejects_missing_extra_duplicate_unreferenced_misbound_and_v1_shapes() {
    let canonical = numerical_reference_v2_not_run_manifest().unwrap();

    let mut missing = canonical.clone();
    missing.bindings.pop();
    assert!(validate_numerical_reference_manifest_v2(&missing).is_err());

    let mut missing_artifact = canonical.clone();
    missing_artifact.artifacts.remove(0);
    assert!(validate_numerical_reference_manifest_v2(&missing_artifact).is_err());

    let mut extra_binding = canonical.clone();
    extra_binding
        .bindings
        .push(extra_binding.bindings[0].clone());
    assert!(validate_numerical_reference_manifest_v2(&extra_binding).is_err());

    let mut extra = canonical.clone();
    extra.artifacts.push(extra.artifacts[0].clone());
    assert!(validate_numerical_reference_manifest_v2(&extra).is_err());

    let mut constant_cardinality_duplicate_artifact = canonical.clone();
    constant_cardinality_duplicate_artifact.artifacts[1] =
        constant_cardinality_duplicate_artifact.artifacts[0].clone();
    assert!(
        validate_numerical_reference_manifest_v2(&constant_cardinality_duplicate_artifact).is_err()
    );

    let mut duplicate = canonical.clone();
    duplicate.bindings[1] = duplicate.bindings[0].clone();
    assert!(validate_numerical_reference_manifest_v2(&duplicate).is_err());

    let mut binding_digest = canonical.clone();
    binding_digest.bindings[0].reference_checksum_sha256 = "f".repeat(64);
    assert!(validate_numerical_reference_manifest_v2(&binding_digest).is_err());

    let mut dishonest_complete = canonical.clone();
    dishonest_complete.generation_status = NumericalReferenceGenerationStatusV2::Complete;
    dishonest_complete.producer.implementation_revision = "1".repeat(40);
    assert!(validate_numerical_reference_manifest_v2(&dishonest_complete).is_err());

    let mut artifact_set_digest = canonical.clone();
    artifact_set_digest.artifact_set_sha256 = "f".repeat(64);
    assert!(validate_numerical_reference_manifest_v2(&artifact_set_digest).is_err());

    let mut binding_set_digest = canonical.clone();
    binding_set_digest.binding_set_sha256 = "f".repeat(64);
    assert!(validate_numerical_reference_manifest_v2(&binding_set_digest).is_err());

    let mut wrong_wrms_policy = canonical.clone();
    wrong_wrms_policy.wrms_policy.formula_id = "pairwise-max".into();
    assert!(validate_numerical_reference_manifest_v2(&wrong_wrms_policy).is_err());

    let mut non_l2_policy = canonical.clone();
    non_l2_policy.artifacts[0].canonical_method = non_l2_policy.generator.radau_ladder[1].clone();
    refresh_manifest_digests(&mut non_l2_policy);
    assert!(validate_numerical_reference_manifest_v2(&non_l2_policy).is_err());

    let mut unreferenced = canonical.clone();
    let orphan_problem = unreferenced.artifacts[0].problem.problem_id.clone();
    for binding in &mut unreferenced.bindings {
        if binding.problem_id == orphan_problem {
            binding.problem_id = unreferenced.artifacts[1].problem.problem_id.clone();
        }
    }
    refresh_manifest_digests(&mut unreferenced);
    assert!(validate_numerical_reference_manifest_v2(&unreferenced).is_err());

    let mut misbound = canonical.clone();
    misbound.bindings[0].problem_id = misbound.artifacts[1].problem.problem_id.clone();
    refresh_manifest_digests(&mut misbound);
    assert!(validate_numerical_reference_manifest_v2(&misbound).is_err());

    let mut topology = canonical.clone();
    let semilinear = topology
        .artifacts
        .iter_mut()
        .find(|entry| entry.problem.grid_shape == Some([8, 12]))
        .unwrap();
    semilinear.problem.grid_shape = Some([12, 8]);
    refresh_manifest_digests(&mut topology);
    assert!(validate_numerical_reference_manifest_v2(&topology).is_err());

    let mut duplicate_path = canonical.clone();
    duplicate_path.artifacts[1].artifact_path = duplicate_path.artifacts[0].artifact_path.clone();
    assert!(validate_numerical_reference_manifest_v2(&duplicate_path).is_err());

    let mut reordered = canonical.clone();
    reordered.artifacts.swap(0, 1);
    assert!(validate_numerical_reference_manifest_v2(&reordered).is_err());

    let mut unknown_case = canonical.clone();
    unknown_case.bindings[0].case_id = "not-a-scientific-case".into();
    refresh_manifest_digests(&mut unknown_case);
    assert!(validate_numerical_reference_manifest_v2(&unknown_case).is_err());

    let mut noncanonical_tolerance = canonical.clone();
    noncanonical_tolerance.bindings[0].case_id = noncanonical_tolerance.bindings[0]
        .case_id
        .replace("1e-4", "1e-04");
    refresh_manifest_digests(&mut noncanonical_tolerance);
    assert!(validate_numerical_reference_manifest_v2(&noncanonical_tolerance).is_err());

    let v1_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/reference_v2/reference_manifest.json");
    let bytes = fs::read(&v1_path).unwrap();
    let v1: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        v1["schema_version"],
        NUMERICAL_REFERENCE_MANIFEST_SCHEMA_VERSION
    );
    assert!(
        serde_json::from_slice::<rodas5p_fair_ab::NumericalReferenceManifestV2>(&bytes).is_err()
    );
}

#[test]
fn v1_loader_remains_live_while_v2_loader_dispatches_by_exact_case_id_and_is_lazy() {
    let v1_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/reference_v2/reference_manifest.json");
    let holdout = ScientificCorpusV2::holdout_specs()
        .into_iter()
        .find(|spec| spec.family == ScientificFamily::Oregonator)
        .unwrap();
    load_numerical_reference(&v1_path, &holdout).unwrap();
    assert!(load_numerical_reference_v2(&v1_path, &holdout).is_err());

    let selected = ScientificCorpusV2::calibration_specs()[0].clone();
    let mut manifest = numerical_reference_v2_not_run_manifest().unwrap();
    let root = root();
    let not_run_path = root.join("not-run-reference_manifest_v2.json");
    fs::write(&not_run_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(load_numerical_reference_v2(&not_run_path, &selected).is_err());

    manifest.generation_status = NumericalReferenceGenerationStatusV2::Complete;
    manifest.producer.implementation_revision = "1".repeat(40);
    for entry in &mut manifest.artifacts {
        entry.artifact_sha256 = "1".repeat(64);
        entry.grid_sha256 = "2".repeat(64);
        entry.state_sha256 = "3".repeat(64);
    }
    let binding = manifest
        .bindings
        .iter()
        .find(|binding| binding.case_id == selected.id)
        .unwrap()
        .clone();
    let entry_index = manifest
        .artifacts
        .iter()
        .position(|entry| entry.problem.problem_id == binding.problem_id)
        .unwrap();
    let times = selected.output_times.clone();
    let states = times
        .iter()
        .map(|time| vec![100.0 + time; selected.dimension])
        .collect::<Vec<_>>();
    let grid_sha256 = numerical_reference_grid_checksum(&times);
    let state_sha256 = numerical_reference_state_checksum(&states);
    let wrms_basis = NumericalReferenceWrmsBasisV2 {
        formula_id: NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID.into(),
        absolute: 1.0e-10,
        relative: 1.0e-8,
        anchor_state_sha256: state_sha256.clone(),
    };
    let convergence = NumericalReferenceConvergenceV2 {
        d0_max_grid_wrms: 4.0,
        d1_max_grid_wrms: 1.0,
        q: 0.25,
        richardson_uncertainty_wrms: 1.0 / 3.0,
        method_disagreement_wrms: 0.2,
        reference_uncertainty_wrms: 1.0 / 3.0 + 0.2,
        wrms_basis,
    };
    let methods = &manifest.generator;
    let run_evidence = methods
        .radau_ladder
        .iter()
        .chain(std::iter::once(&methods.tight_lsoda))
        .map(|method| NumericalReferenceRunEvidenceV2 {
            label: method.label.clone(),
            status: NumericalReferenceRunStatusV2::Complete,
            wall_seconds: Some(0.01),
            nfev: Some(1),
            njev: Some(1),
            nlu: Some(1),
            process_peak_rss_bytes_at_run_end: Some(1024),
            message: None,
        })
        .collect();
    let artifact = NumericalReferenceArtifactV2 {
        schema_version: NUMERICAL_REFERENCE_V2_ARTIFACT_SCHEMA_VERSION.into(),
        problem_id: binding.problem_id.clone(),
        requested_times: times,
        states,
        canonical_method: methods.radau_ladder[2].clone(),
        independent_method: methods.tight_lsoda.clone(),
        convergence,
        checksums: rodas5p_fair_ab::NumericalReferenceChecksums {
            grid_sha256: grid_sha256.clone(),
            state_sha256: state_sha256.clone(),
        },
        run_evidence,
    };
    let bytes = serde_json::to_vec(&artifact).unwrap();
    let artifact_path = root.join(&manifest.artifacts[entry_index].artifact_path);
    fs::create_dir_all(artifact_path.parent().unwrap()).unwrap();
    fs::write(&artifact_path, &bytes).unwrap();
    manifest.artifacts[entry_index].artifact_sha256 = sha256_hex(&bytes);
    manifest.artifacts[entry_index].grid_sha256 = grid_sha256;
    manifest.artifacts[entry_index].state_sha256 = state_sha256;
    refresh_manifest_digests(&mut manifest);
    let manifest_path = root.join("reference_manifest_v2.json");
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    // None of the other 21 artifact paths exists.  Loading one exact case must
    // read only its selected physical artifact.
    let loaded = load_numerical_reference_v2(&manifest_path, &selected).unwrap();
    assert_eq!(loaded.case_id, selected.id);
    assert_eq!(loaded.problem_id, binding.problem_id);
    assert_eq!(
        loaded.reference_checksum_sha256,
        manifest
            .bindings
            .iter()
            .find(|candidate| candidate.case_id == selected.id)
            .unwrap()
            .reference_checksum_sha256
    );
    assert_eq!(loaded.wrms_basis.reference_states, loaded.trajectory.states);
    assert_eq!(
        loaded.error_scale.absolute,
        vec![1.0e-10; selected.dimension]
    );
    assert_eq!(loaded.error_scale.relative.to_bits(), 1.0e-8_f64.to_bits());
    let provenance = loaded.trajectory.provenance.numerical.as_ref().unwrap();
    assert_eq!(
        provenance.corpus_version.as_deref(),
        Some(ScientificCorpusV2::VERSION)
    );
    assert_eq!(provenance.case_id.as_deref(), Some(selected.id.as_str()));
    assert_eq!(
        provenance.wrms_formula_id.as_deref(),
        Some(NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID)
    );
    assert_eq!(
        provenance.anchor_state_sha256.as_deref(),
        Some(manifest.artifacts[entry_index].state_sha256.as_str())
    );
    let lower = loaded
        .trajectory
        .states
        .iter()
        .map(|row| row.iter().map(|value| value - 1.0).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let upper = loaded
        .trajectory
        .states
        .iter()
        .map(|row| row.iter().map(|value| value + 1.0).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let gap = loaded
        .wrms_basis
        .discrepancy_wrms(
            &selected.output_times,
            &lower,
            &selected.output_times,
            &upper,
        )
        .unwrap();
    assert!(gap.is_finite() && gap > 0.0);
    let metrics = loaded
        .wrms_basis
        .metrics(&selected.output_times, &lower)
        .unwrap();
    assert!(metrics.max_grid_wrms.is_finite() && metrics.max_grid_wrms > 0.0);

    let sibling = ScientificCorpusV2::calibration_specs()
        .into_iter()
        .find(|spec| {
            spec.family == selected.family
                && spec.dimension == selected.dimension
                && spec.grid_shape == selected.grid_shape
                && spec.id != selected.id
        })
        .unwrap();
    let loaded_sibling = load_numerical_reference_v2(&manifest_path, &sibling).unwrap();
    assert_eq!(loaded_sibling.problem_id, loaded.problem_id);
    assert_eq!(loaded_sibling.trajectory.states, loaded.trajectory.states);
    assert_eq!(
        loaded_sibling
            .trajectory
            .provenance
            .numerical
            .as_ref()
            .unwrap()
            .artifact_sha256,
        provenance.artifact_sha256
    );
    assert_ne!(
        loaded_sibling.reference_checksum_sha256,
        loaded.reference_checksum_sha256
    );
    let mut altered = selected.clone();
    altered.id = altered.id.replace("1e-4", "1e-04");
    assert!(load_numerical_reference_v2(&manifest_path, &altered).is_err());

    let mut corrupt_raw = bytes.clone();
    corrupt_raw.push(b' ');
    fs::write(&artifact_path, &corrupt_raw).unwrap();
    assert!(load_numerical_reference_v2(&manifest_path, &selected).is_err());
    fs::write(&artifact_path, &bytes).unwrap();

    for mutation in ["inner-state", "formula", "anchor", "non-l2"] {
        let mut changed_artifact = artifact.clone();
        match mutation {
            "inner-state" => changed_artifact.states[0][0] += 1.0,
            "formula" => changed_artifact.convergence.wrms_basis.formula_id = "pairwise-max".into(),
            "anchor" => {
                changed_artifact.convergence.wrms_basis.anchor_state_sha256 = "f".repeat(64)
            }
            "non-l2" => {
                changed_artifact.canonical_method = manifest.generator.radau_ladder[1].clone()
            }
            _ => unreachable!(),
        }
        let changed_bytes = serde_json::to_vec(&changed_artifact).unwrap();
        let mut changed_manifest = manifest.clone();
        changed_manifest.artifacts[entry_index].artifact_sha256 = sha256_hex(&changed_bytes);
        refresh_manifest_digests(&mut changed_manifest);
        fs::write(&artifact_path, changed_bytes).unwrap();
        fs::write(
            &manifest_path,
            serde_json::to_vec(&changed_manifest).unwrap(),
        )
        .unwrap();
        assert!(
            load_numerical_reference_v2(&manifest_path, &selected).is_err(),
            "mutation {mutation} escaped v2 validation"
        );
    }
    fs::write(&artifact_path, &bytes).unwrap();
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn v2_grid_and_state_payload_checksums_remain_v1_compatible() {
    // Only aggregate artifact/binding checksums use v2-separated domains.  Raw
    // grid/state payload domains intentionally remain byte-compatible with v1.
    assert_eq!(
        numerical_reference_grid_checksum(&[0.0, 1.0]),
        "d5faaa8a445674f57b8cd51d266ebb915dbc9369d38343249b26f1f38e74a044"
    );
    assert_eq!(
        numerical_reference_state_checksum(&[vec![100.0], vec![101.0]]),
        "f852937fb3ab93c72b8ca1b149aadf9c4e84aa0fb57572314a7b0654b72e44ef"
    );
}

#[test]
fn every_v2_comparison_uses_the_tight_reference_state_as_the_only_wrms_anchor() {
    let grid = CommonOutputGrid::new(vec![0.0]).unwrap();
    let scale = ExternalErrorScale::new(vec![1.0], 1.0).unwrap();
    let basis = ReferenceWrmsBasis::new(grid.clone(), vec![vec![100.0]], scale.clone()).unwrap();
    let gap = basis
        .discrepancy_wrms(&[0.0], &[vec![99.0]], &[0.0], &[vec![101.0]])
        .unwrap();
    assert!((gap - 2.0 / 101.0).abs() <= 2.0 * f64::EPSILON);

    // Swapping the tight reference anchor is intentionally not symmetric:
    // this catches the old pairwise max(abs(left), abs(right)) formula.
    let swapped = ReferenceWrmsBasis::new(grid, vec![vec![99.0]], scale)
        .unwrap()
        .discrepancy_wrms(&[0.0], &[vec![100.0]], &[0.0], &[vec![101.0]])
        .unwrap();
    assert!((swapped - 1.0 / 100.0).abs() <= 2.0 * f64::EPSILON);
}
