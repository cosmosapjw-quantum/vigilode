use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use rodas5p_core::sha256_hex;
use rodas5p_fair_ab::{
    ExternalErrorScale, IntegratorRunStatus, NUMERICAL_REFERENCE_MANIFEST_SCHEMA_VERSION,
    NumericalReferenceArtifact, NumericalReferenceManifest, ReferenceDominance,
    ReferenceSolutionProvenance, ReferenceSourceKind, classify_reference_dominance,
    load_numerical_reference, numerical_reference_artifact_set_checksum,
    numerical_reference_grid_checksum, validate_numerical_reference_convergence,
    validate_numerical_reference_error_scale,
};
use rodas5p_integrators::{ScientificCaseSpec, ScientificCorpusV2, ScientificFamily};
use serde_json::{Value, json};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn pinned_numerical_reference_manifest_is_present_for_fair_ab_consumption() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/reference_v2/reference_manifest.json");
    assert!(
        manifest.is_file(),
        "the high-accuracy numerical reference manifest must be available to fair-ab"
    );
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn canonical_manifest_path() -> PathBuf {
    workspace_root().join("tools/reference_v2/reference_manifest.json")
}

fn holdout_spec(family: ScientificFamily) -> ScientificCaseSpec {
    ScientificCorpusV2::holdout_specs()
        .into_iter()
        .find(|spec| spec.family == family)
        .expect("every family has a holdout spec")
}

struct Fixture {
    root: PathBuf,
    manifest: Value,
}

impl Fixture {
    fn from_real() -> Self {
        let canonical = canonical_manifest_path();
        let manifest: Value = serde_json::from_slice(
            &fs::read(&canonical).expect("read checked-in numerical-reference manifest"),
        )
        .expect("parse checked-in numerical-reference manifest");
        let unique = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "rodas5p-numerical-reference-contracts-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create isolated fixture root");
        for entry in manifest["artifacts"]
            .as_array()
            .expect("manifest artifacts array")
        {
            let relative = entry["artifact_path"].as_str().expect("artifact path");
            let destination = root.join(relative);
            fs::create_dir_all(destination.parent().expect("artifact parent"))
                .expect("create fixture artifact parent");
            fs::copy(
                canonical
                    .parent()
                    .expect("canonical manifest parent")
                    .join(relative),
                destination,
            )
            .expect("copy canonical artifact into fixture");
        }
        let mut fixture = Self { root, manifest };
        fixture.write_manifest();
        fixture
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join("reference_manifest.json")
    }

    fn entry_mut(&mut self, family: &str) -> &mut Value {
        self.manifest["artifacts"]
            .as_array_mut()
            .expect("manifest artifacts array")
            .iter_mut()
            .find(|entry| entry["problem"]["family"] == family)
            .expect("fixture family exists")
    }

    fn artifact_path(&self, family: &str) -> PathBuf {
        let entry = self.manifest["artifacts"]
            .as_array()
            .expect("manifest artifacts array")
            .iter()
            .find(|entry| entry["problem"]["family"] == family)
            .expect("fixture family exists");
        self.root
            .join(entry["artifact_path"].as_str().expect("artifact path"))
    }

    fn artifact(&self, family: &str) -> Value {
        serde_json::from_slice(
            &fs::read(self.artifact_path(family)).expect("read fixture artifact"),
        )
        .expect("parse fixture artifact")
    }

    fn write_artifact(&mut self, family: &str, artifact: &Value, refresh_sha: bool) {
        let path = self.artifact_path(family);
        let bytes = serde_json::to_vec(artifact).expect("serialize fixture artifact");
        fs::write(&path, &bytes).expect("write fixture artifact");
        if refresh_sha {
            self.entry_mut(family)["artifact_sha256"] = json!(sha256_hex(&bytes));
        }
    }

    fn refresh_aggregate_checksum(&mut self) {
        let typed: NumericalReferenceManifest =
            serde_json::from_value(self.manifest.clone()).expect("typed fixture manifest");
        self.manifest["artifact_set_sha256"] =
            json!(numerical_reference_artifact_set_checksum(&typed.artifacts));
    }

    fn write_manifest(&mut self) {
        fs::write(
            self.manifest_path(),
            serde_json::to_vec(&self.manifest).expect("serialize fixture manifest"),
        )
        .expect("write fixture manifest");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn legacy_analytic_provenance_deserializes_without_changing_its_wire_shape() {
    let legacy = r#"{"problem_id":"analytic","source_kind":"analytic-exact","output_grid_id":"grid","state_checksum":"states","reference_uncertainty_wrms":0.0}"#;
    let provenance: ReferenceSolutionProvenance = serde_json::from_str(legacy).unwrap();
    assert_eq!(provenance.source_kind, ReferenceSourceKind::AnalyticExact);
    assert!(provenance.numerical.is_none());
    assert_eq!(serde_json::to_string(&provenance).unwrap(), legacy);
}

#[test]
fn numerical_reference_and_reference_dominated_wire_spellings_are_stable() {
    assert_eq!(
        serde_json::to_string(&ReferenceSourceKind::HighAccuracyNumerical).unwrap(),
        "\"high-accuracy-numerical\""
    );
    assert_eq!(
        serde_json::to_string(&IntegratorRunStatus::ReferenceDominated).unwrap(),
        "\"reference-dominated\""
    );
}

#[test]
fn checked_in_artifacts_load_for_each_holdout_and_preserve_numerical_provenance() {
    let manifest = canonical_manifest_path();
    for family in ScientificFamily::HOLDOUT {
        let reference = load_numerical_reference(&manifest, &holdout_spec(family)).unwrap();
        assert_eq!(reference.states.len(), reference.output_grid.times.len());
        assert_eq!(
            reference.provenance.source_kind,
            ReferenceSourceKind::HighAccuracyNumerical
        );
        assert_eq!(
            reference
                .provenance
                .numerical
                .as_ref()
                .unwrap()
                .manifest_schema_version,
            NUMERICAL_REFERENCE_MANIFEST_SCHEMA_VERSION
        );
        validate_numerical_reference_error_scale(&reference.provenance, &reference.error_scale)
            .unwrap();

        let mut wrong = reference.error_scale.clone();
        wrong.relative *= 10.0;
        assert!(validate_numerical_reference_error_scale(&reference.provenance, &wrong).is_err());
        let wrong_absolute = ExternalErrorScale::with_reference_uncertainty(
            vec![2.0e-10; reference.states[0].len()],
            reference.error_scale.relative,
            reference.error_scale.reference_uncertainty_wrms,
        )
        .unwrap();
        assert!(
            validate_numerical_reference_error_scale(&reference.provenance, &wrong_absolute)
                .is_err()
        );
    }
}

#[test]
fn loader_rejects_bad_q_nonfinite_convergence_runtime_pin_grid_checksum_and_canonical_source() {
    let mut fixture = Fixture::from_real();
    let mut artifact = fixture.artifact("oregonator");
    artifact["convergence"] = json!({
        "d0_max_grid_wrms": 4.0,
        "d1_max_grid_wrms": 3.0,
        "q": 0.75,
        "richardson_uncertainty_wrms": 9.0,
        "method_disagreement_wrms": 0.1,
        "reference_uncertainty_wrms": 9.1,
        "wrms_scale": {"absolute": 1e-10, "relative": 1e-8}
    });
    fixture.write_artifact("oregonator", &artifact, true);
    fixture.refresh_aggregate_checksum();
    fixture.write_manifest();
    assert!(
        load_numerical_reference(
            fixture.manifest_path(),
            &holdout_spec(ScientificFamily::Oregonator)
        )
        .is_err()
    );

    let mut typed: NumericalReferenceArtifact =
        serde_json::from_value(fixture.artifact("pollution")).unwrap();
    typed.convergence.d0_max_grid_wrms = f64::NAN;
    assert!(validate_numerical_reference_convergence(&typed.convergence).is_err());

    let mut fixture = Fixture::from_real();
    fixture.manifest["generator"]["numpy"] = json!("0.0.0");
    fixture.write_manifest();
    assert!(
        load_numerical_reference(
            fixture.manifest_path(),
            &holdout_spec(ScientificFamily::Oregonator)
        )
        .is_err()
    );

    let mut fixture = Fixture::from_real();
    let mut artifact = fixture.artifact("oregonator");
    artifact["requested_times"][1] = json!(3.61);
    let altered_times = artifact["requested_times"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_f64().unwrap())
        .collect::<Vec<_>>();
    artifact["checksums"]["grid_sha256"] = json!(numerical_reference_grid_checksum(&altered_times));
    fixture.entry_mut("oregonator")["grid_sha256"] = artifact["checksums"]["grid_sha256"].clone();
    fixture.write_artifact("oregonator", &artifact, true);
    fixture.refresh_aggregate_checksum();
    fixture.write_manifest();
    assert!(
        load_numerical_reference(
            fixture.manifest_path(),
            &holdout_spec(ScientificFamily::Oregonator)
        )
        .is_err()
    );

    let mut fixture = Fixture::from_real();
    let mut artifact = fixture.artifact("oregonator");
    artifact["states"][0][0] = json!(1.25);
    fixture.write_artifact("oregonator", &artifact, false);
    fixture.write_manifest();
    assert!(
        load_numerical_reference(
            fixture.manifest_path(),
            &holdout_spec(ScientificFamily::Oregonator)
        )
        .is_err()
    );

    let mut fixture = Fixture::from_real();
    let mut artifact = fixture.artifact("oregonator");
    artifact["states"][0][0] = json!(1.25);
    fixture.write_artifact("oregonator", &artifact, true);
    fixture.refresh_aggregate_checksum();
    fixture.write_manifest();
    assert!(
        load_numerical_reference(
            fixture.manifest_path(),
            &holdout_spec(ScientificFamily::Oregonator)
        )
        .is_err()
    );

    let mut fixture = Fixture::from_real();
    let mut artifact = fixture.artifact("oregonator");
    artifact["canonical_method"]["method"] = json!("LSODA");
    fixture.write_artifact("oregonator", &artifact, true);
    fixture.refresh_aggregate_checksum();
    fixture.write_manifest();
    assert!(
        load_numerical_reference(
            fixture.manifest_path(),
            &holdout_spec(ScientificFamily::Oregonator)
        )
        .is_err()
    );

    let mut fixture = Fixture::from_real();
    fixture.manifest["artifact_set_sha256"] = json!("0".repeat(64));
    fixture.write_manifest();
    assert!(
        load_numerical_reference(
            fixture.manifest_path(),
            &holdout_spec(ScientificFamily::Oregonator)
        )
        .is_err()
    );
}

#[test]
fn reference_dominance_is_strict_at_ten_percent_and_excludes_pareto_success() {
    assert_eq!(
        classify_reference_dominance(0.1, 1.0).unwrap(),
        ReferenceDominance::Admissible
    );
    assert_eq!(
        classify_reference_dominance(f64::from_bits(0.1_f64.to_bits() + 1), 1.0).unwrap(),
        ReferenceDominance::Dominated
    );
    assert!(!IntegratorRunStatus::ReferenceDominated.is_success());
}
