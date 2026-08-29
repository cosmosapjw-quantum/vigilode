//! Strict loader and provenance types for externally generated numerical references.
//!
//! The generator lives in `tools/reference_v2` and is intentionally independent of the Rust
//! implementation.  This module treats its JSON as untrusted evidence: every pinned runtime,
//! source identity, grid, state shape, convergence value, and checksum is checked before a
//! trajectory is made available to a scientific screen.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    ops::Deref,
    path::Path,
};

use rodas5p_core::sha256_hex;
use rodas5p_integrators::{
    CorpusPartition, ScientificCaseSpec, ScientificCorpusV2, ScientificFamily,
    ScientificSourceProvenance,
};
use serde::{Deserialize, Serialize};

use crate::{
    CommonOutputGrid, ExternalErrorScale, FairError, FairResult, ReferenceSolutionProvenance,
    ReferenceSourceKind, ReferenceTrajectory, ReferenceWrmsBasis,
};

pub const NUMERICAL_REFERENCE_MANIFEST_SCHEMA_VERSION: &str =
    "vigilode-numerical-reference-manifest-v1";
pub const NUMERICAL_REFERENCE_ARTIFACT_SCHEMA_VERSION: &str =
    "vigilode-numerical-reference-artifact-v1";
pub const NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION: &str =
    "vigilode-numerical-reference-manifest-v2";
pub const NUMERICAL_REFERENCE_V2_ARTIFACT_SCHEMA_VERSION: &str =
    "vigilode-numerical-reference-artifact-v2";
pub const NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID: &str = "wrms-tight-radau-l2-anchor-v1";
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceGeneratorPins {
    /// Python major/minor identity.  Patch releases are deliberately not part of the contract.
    pub python: String,
    pub numpy: String,
    pub scipy: String,
    pub blas_threads: usize,
    pub radau_ladder: Vec<NumericalReferenceMethod>,
    pub tight_lsoda: NumericalReferenceMethod,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceMethod {
    pub label: String,
    pub method: String,
    pub rtol: f64,
    pub atol: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceSourceDefinition {
    pub source_definition_id: String,
    pub source_repository: String,
    pub source_revision: String,
    pub source_path: String,
    pub source_blob: Option<String>,
    pub source_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceProblem {
    pub problem_id: String,
    pub family: String,
    pub dimension: usize,
    pub t_span: [f64; 2],
    pub uniform_output_points: usize,
    pub mandatory_breakpoints: Vec<f64>,
    pub source: NumericalReferenceSourceDefinition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceChecksums {
    pub grid_sha256: String,
    pub state_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceWrmsScale {
    pub absolute: f64,
    pub relative: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceConvergence {
    pub d0_max_grid_wrms: f64,
    pub d1_max_grid_wrms: f64,
    pub q: f64,
    pub richardson_uncertainty_wrms: f64,
    pub method_disagreement_wrms: f64,
    pub reference_uncertainty_wrms: f64,
    pub wrms_scale: NumericalReferenceWrmsScale,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceArtifact {
    pub schema_version: String,
    pub problem: NumericalReferenceProblem,
    pub requested_times: Vec<f64>,
    pub states: Vec<Vec<f64>>,
    pub canonical_method: NumericalReferenceMethod,
    pub independent_method: NumericalReferenceMethod,
    pub convergence: NumericalReferenceConvergence,
    pub checksums: NumericalReferenceChecksums,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceManifestEntry {
    pub problem: NumericalReferenceProblem,
    pub artifact_path: String,
    pub artifact_sha256: String,
    pub grid_sha256: String,
    pub state_sha256: String,
    pub canonical_method: NumericalReferenceMethod,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceManifest {
    pub schema_version: String,
    /// `--generate` is explicit; `--self-check` only validates immutable existing evidence.
    pub generation_mode: String,
    pub generator: NumericalReferenceGeneratorPins,
    pub artifacts: Vec<NumericalReferenceManifestEntry>,
    /// SHA-256 of the canonically ordered `(problem_id, artifact, grid, state)` digest table.
    pub artifact_set_sha256: String,
}

/// v2 keeps physical trajectories and tolerance-specific case bindings
/// separate.  Eighteen calibration problems plus four holdouts therefore
/// occupy 22 artifacts while the exact ScientificCorpusV2.1 case surface has
/// 66 bindings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NumericalReferenceGenerationStatusV2 {
    NotRun,
    Complete,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceProblemV2 {
    pub problem_id: String,
    pub family: String,
    pub partition: CorpusPartition,
    pub dimension: usize,
    pub grid_shape: Option<[usize; 2]>,
    pub t_span: [f64; 2],
    pub uniform_output_points: usize,
    pub mandatory_breakpoints: Vec<f64>,
    pub source: ScientificSourceProvenance,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceWrmsPolicyV2 {
    pub formula_id: String,
    pub absolute: f64,
    pub relative: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceRuntimeLibraryV2 {
    pub role: String,
    pub basename: String,
    pub version: String,
    pub configuration: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceRuntimeIdentityV2 {
    pub python_executable: String,
    pub python_version: String,
    pub python_sha256: String,
    pub numpy_record_sha256: String,
    pub numpy_record_verified_file_count: usize,
    pub numpy_git_revision: String,
    pub scipy_record_sha256: String,
    pub scipy_record_verified_file_count: usize,
    pub scipy_git_revision: String,
    pub scipy_release: bool,
    pub scipy_version_module_sha256: String,
    pub scipy_radau_module_sha256: String,
    pub blas_libraries: Vec<NumericalReferenceRuntimeLibraryV2>,
    pub thread_environment: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceProducerIdentityV2 {
    pub script_path: String,
    pub script_sha256: String,
    pub implementation_revision: String,
    pub problem_definition_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceWrmsBasisV2 {
    pub formula_id: String,
    pub absolute: f64,
    pub relative: f64,
    /// The anchor is the canonical tight-Radau L2 state table itself.
    pub anchor_state_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceConvergenceV2 {
    pub d0_max_grid_wrms: f64,
    pub d1_max_grid_wrms: f64,
    pub q: f64,
    pub richardson_uncertainty_wrms: f64,
    pub method_disagreement_wrms: f64,
    pub reference_uncertainty_wrms: f64,
    pub wrms_basis: NumericalReferenceWrmsBasisV2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NumericalReferenceRunStatusV2 {
    Complete,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceRunEvidenceV2 {
    pub label: String,
    pub status: NumericalReferenceRunStatusV2,
    pub wall_seconds: Option<f64>,
    pub nfev: Option<u64>,
    pub njev: Option<u64>,
    pub nlu: Option<u64>,
    pub process_peak_rss_bytes_at_run_end: Option<u64>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceArtifactV2 {
    pub schema_version: String,
    pub problem_id: String,
    pub requested_times: Vec<f64>,
    pub states: Vec<Vec<f64>>,
    pub canonical_method: NumericalReferenceMethod,
    pub independent_method: NumericalReferenceMethod,
    pub convergence: NumericalReferenceConvergenceV2,
    pub checksums: NumericalReferenceChecksums,
    pub run_evidence: Vec<NumericalReferenceRunEvidenceV2>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceManifestEntryV2 {
    pub problem: NumericalReferenceProblemV2,
    pub artifact_path: String,
    pub artifact_sha256: String,
    pub grid_sha256: String,
    pub state_sha256: String,
    pub canonical_method: NumericalReferenceMethod,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceCaseBindingV2 {
    pub case_id: String,
    pub problem_id: String,
    pub reference_checksum_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceManifestV2 {
    pub schema_version: String,
    pub corpus_version: String,
    pub generation_status: NumericalReferenceGenerationStatusV2,
    pub generation_mode: String,
    pub generator: NumericalReferenceGeneratorPins,
    pub runtime: NumericalReferenceRuntimeIdentityV2,
    pub producer: NumericalReferenceProducerIdentityV2,
    pub wrms_policy: NumericalReferenceWrmsPolicyV2,
    pub artifacts: Vec<NumericalReferenceManifestEntryV2>,
    pub bindings: Vec<NumericalReferenceCaseBindingV2>,
    pub artifact_set_sha256: String,
    pub binding_set_sha256: String,
}

/// Evidence copied from a validated manifest/artifact pair into a scientific report.
///
/// This deliberately contains no local path: reports remain deterministic when a validated
/// artifact directory is relocated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceProvenance {
    pub manifest_schema_version: String,
    pub artifact_schema_version: String,
    pub artifact_sha256: String,
    pub source_definition_id: String,
    pub generator: NumericalReferenceGeneratorPins,
    pub canonical_method: NumericalReferenceMethod,
    pub independent_method: NumericalReferenceMethod,
    pub checksums: NumericalReferenceChecksums,
    pub convergence: NumericalReferenceConvergence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corpus_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_checksum_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrms_formula_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_state_sha256: Option<String>,
}

/// A numerical trajectory and the exact WRMS normalization under which its
/// uncertainty was measured.
///
/// Keeping these values in one return object prevents callers from comparing
/// the stored uncertainty with candidate errors normalized by an unrelated
/// tolerance scale.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceBundle {
    pub trajectory: ReferenceTrajectory,
    pub error_scale: ExternalErrorScale,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalReferenceBundleV2 {
    pub case_id: String,
    pub problem_id: String,
    pub reference_checksum_sha256: String,
    pub implementation_revision: String,
    pub trajectory: ReferenceTrajectory,
    pub error_scale: ExternalErrorScale,
    pub wrms_basis: ReferenceWrmsBasis,
}

impl Deref for NumericalReferenceBundle {
    type Target = ReferenceTrajectory;

    fn deref(&self) -> &Self::Target {
        &self.trajectory
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReferenceDominance {
    Admissible,
    Dominated,
}

/// Applies the numerical-reference validity rule to a measured scientific error.
///
/// Equality is intentionally admissible: only uncertainty *strictly greater* than ten percent
/// of the measured maximum-grid WRMS error invalidates a row.
pub fn classify_reference_dominance(
    reference_uncertainty_wrms: f64,
    measured_max_grid_wrms: f64,
) -> FairResult<ReferenceDominance> {
    if !(reference_uncertainty_wrms.is_finite()
        && reference_uncertainty_wrms >= 0.0
        && measured_max_grid_wrms.is_finite()
        && measured_max_grid_wrms >= 0.0)
    {
        return Err(FairError::Invalid(
            "reference dominance inputs must be finite and nonnegative".into(),
        ));
    }
    if reference_uncertainty_wrms > 0.1 * measured_max_grid_wrms {
        Ok(ReferenceDominance::Dominated)
    } else {
        Ok(ReferenceDominance::Admissible)
    }
}

/// Verify that a candidate-error computation uses the same WRMS basis as the
/// numerical-reference uncertainty.
pub fn validate_numerical_reference_error_scale(
    provenance: &ReferenceSolutionProvenance,
    scale: &ExternalErrorScale,
) -> FairResult<()> {
    let numerical = provenance
        .numerical
        .as_ref()
        .ok_or_else(|| FairError::Invalid("numerical reference provenance is missing".into()))?;
    let expected = &numerical.convergence.wrms_scale;
    if !scale.absolute.is_empty()
        && scale
            .absolute
            .iter()
            .all(|value| value.to_bits() == expected.absolute.to_bits())
        && scale.relative.to_bits() == expected.relative.to_bits()
        && scale.reference_uncertainty_wrms.to_bits()
            == numerical.convergence.reference_uncertainty_wrms.to_bits()
        && provenance.reference_uncertainty_wrms.to_bits()
            == numerical.convergence.reference_uncertainty_wrms.to_bits()
    {
        Ok(())
    } else {
        Err(FairError::Invalid(
            "candidate error scale is not the numerical reference uncertainty scale".into(),
        ))
    }
}

/// Stable binary checksum of an exact requested-time grid.
pub fn numerical_reference_grid_checksum(times: &[f64]) -> String {
    let mut bytes = b"vigilode-reference-grid-v1\0".to_vec();
    bytes.extend_from_slice(&(times.len() as u64).to_le_bytes());
    for time in times {
        bytes.extend_from_slice(&time.to_bits().to_le_bytes());
    }
    sha256_hex(&bytes)
}

/// Stable binary checksum of a rectangular f64 state table.
pub fn numerical_reference_state_checksum(states: &[Vec<f64>]) -> String {
    let mut bytes = b"vigilode-reference-states-v1\0".to_vec();
    bytes.extend_from_slice(&(states.len() as u64).to_le_bytes());
    for state in states {
        bytes.extend_from_slice(&(state.len() as u64).to_le_bytes());
        for value in state {
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
    }
    sha256_hex(&bytes)
}

/// Aggregate checksum for a manifest's complete artifact set.
///
/// The payload deliberately excludes the manifest itself, so it is not self-referential.  Entries
/// are sorted by problem id and then encoded as length-prefixed UTF-8 fields, avoiding any JSON
/// formatter dependence between the Python producer and Rust consumer.
pub fn numerical_reference_artifact_set_checksum(
    entries: &[NumericalReferenceManifestEntry],
) -> String {
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.problem.problem_id.cmp(&right.problem.problem_id));
    let mut bytes = b"vigilode-reference-artifact-set-v1\0".to_vec();
    bytes.extend_from_slice(&(ordered.len() as u64).to_le_bytes());
    for entry in ordered {
        for field in [
            entry.problem.problem_id.as_str(),
            entry.artifact_sha256.as_str(),
            entry.grid_sha256.as_str(),
            entry.state_sha256.as_str(),
        ] {
            bytes.extend_from_slice(&(field.len() as u64).to_le_bytes());
            bytes.extend_from_slice(field.as_bytes());
        }
    }
    sha256_hex(&bytes)
}

fn append_checksum_field(bytes: &mut Vec<u8>, field: &str) {
    bytes.extend_from_slice(&(field.len() as u64).to_le_bytes());
    bytes.extend_from_slice(field.as_bytes());
}

fn append_grid_shape(bytes: &mut Vec<u8>, grid_shape: Option<[usize; 2]>) {
    match grid_shape {
        None => bytes.push(0),
        Some([nx, ny]) => {
            bytes.push(1);
            bytes.extend_from_slice(&(nx as u64).to_le_bytes());
            bytes.extend_from_slice(&(ny as u64).to_le_bytes());
        }
    }
}

fn append_optional_checksum_field(bytes: &mut Vec<u8>, field: Option<&str>) {
    match field {
        None => bytes.push(0),
        Some(value) => {
            bytes.push(1);
            append_checksum_field(bytes, value);
        }
    }
}

pub fn numerical_reference_problem_definition_checksum_v2(
    entries: &[NumericalReferenceManifestEntryV2],
) -> String {
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.problem.problem_id.cmp(&right.problem.problem_id));
    let mut bytes = b"vigilode-reference-problem-definitions-v2\0".to_vec();
    bytes.extend_from_slice(&(ordered.len() as u64).to_le_bytes());
    for entry in ordered {
        let problem = &entry.problem;
        for field in [
            problem.problem_id.as_str(),
            problem.family.as_str(),
            match problem.partition {
                CorpusPartition::Calibration => "calibration",
                CorpusPartition::Holdout => "holdout",
            },
        ] {
            append_checksum_field(&mut bytes, field);
        }
        bytes.extend_from_slice(&(problem.dimension as u64).to_le_bytes());
        append_grid_shape(&mut bytes, problem.grid_shape);
        for time in problem.t_span {
            bytes.extend_from_slice(&time.to_bits().to_le_bytes());
        }
        bytes.extend_from_slice(&(problem.uniform_output_points as u64).to_le_bytes());
        bytes.extend_from_slice(&(problem.mandatory_breakpoints.len() as u64).to_le_bytes());
        for breakpoint in &problem.mandatory_breakpoints {
            bytes.extend_from_slice(&breakpoint.to_bits().to_le_bytes());
        }
        let source = &problem.source;
        for field in [
            source.source_repository.as_str(),
            source.source_revision.as_str(),
            source.source_path.as_str(),
        ] {
            append_checksum_field(&mut bytes, field);
        }
        append_optional_checksum_field(&mut bytes, source.source_blob.as_deref());
        append_optional_checksum_field(&mut bytes, source.source_sha256.as_deref());
        append_checksum_field(&mut bytes, &source.license_or_terms);
        append_optional_checksum_field(&mut bytes, source.interpretation_note.as_deref());
    }
    sha256_hex(&bytes)
}

/// Domain-separated checksum binding one tolerance-specific case to one
/// authenticated physical trajectory and its typed topology.
pub fn numerical_reference_case_binding_checksum_v2(
    binding: &NumericalReferenceCaseBindingV2,
    entry: &NumericalReferenceManifestEntryV2,
) -> String {
    let mut bytes = b"vigilode-reference-case-binding-v2\0".to_vec();
    for field in [
        binding.case_id.as_str(),
        binding.problem_id.as_str(),
        entry.artifact_sha256.as_str(),
        entry.grid_sha256.as_str(),
        entry.state_sha256.as_str(),
    ] {
        append_checksum_field(&mut bytes, field);
    }
    append_grid_shape(&mut bytes, entry.problem.grid_shape);
    sha256_hex(&bytes)
}

pub fn numerical_reference_artifact_set_checksum_v2(
    entries: &[NumericalReferenceManifestEntryV2],
) -> String {
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.problem.problem_id.cmp(&right.problem.problem_id));
    let mut bytes = b"vigilode-reference-artifact-set-v2\0".to_vec();
    bytes.extend_from_slice(&(ordered.len() as u64).to_le_bytes());
    for entry in ordered {
        for field in [
            entry.problem.problem_id.as_str(),
            entry.artifact_sha256.as_str(),
            entry.grid_sha256.as_str(),
            entry.state_sha256.as_str(),
        ] {
            append_checksum_field(&mut bytes, field);
        }
        append_grid_shape(&mut bytes, entry.problem.grid_shape);
    }
    sha256_hex(&bytes)
}

pub fn numerical_reference_binding_set_checksum_v2(
    bindings: &[NumericalReferenceCaseBindingV2],
) -> String {
    let mut ordered = bindings.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    let mut bytes = b"vigilode-reference-binding-set-v2\0".to_vec();
    bytes.extend_from_slice(&(ordered.len() as u64).to_le_bytes());
    for binding in ordered {
        for field in [
            binding.case_id.as_str(),
            binding.problem_id.as_str(),
            binding.reference_checksum_sha256.as_str(),
        ] {
            append_checksum_field(&mut bytes, field);
        }
    }
    sha256_hex(&bytes)
}

fn numerical_reference_v2_generator_pins() -> NumericalReferenceGeneratorPins {
    NumericalReferenceGeneratorPins {
        python: "3.12".into(),
        numpy: "2.4.2".into(),
        scipy: "1.17.0".into(),
        blas_threads: 1,
        radau_ladder: vec![
            NumericalReferenceMethod {
                label: "L0".into(),
                method: "Radau".into(),
                rtol: 1.0e-8,
                atol: 1.0e-10,
            },
            NumericalReferenceMethod {
                label: "L1".into(),
                method: "Radau".into(),
                rtol: 1.0e-10,
                atol: 1.0e-12,
            },
            NumericalReferenceMethod {
                label: "L2".into(),
                method: "Radau".into(),
                rtol: 1.0e-12,
                atol: 1.0e-14,
            },
        ],
        tight_lsoda: NumericalReferenceMethod {
            label: "tight-lsoda".into(),
            method: "LSODA".into(),
            rtol: 3.0e-14,
            atol: 3.0e-16,
        },
    }
}

fn numerical_reference_v2_runtime_identity() -> NumericalReferenceRuntimeIdentityV2 {
    NumericalReferenceRuntimeIdentityV2 {
        python_executable: "/usr/bin/python3.12".into(),
        python_version: "3.12.3".into(),
        python_sha256: "a92f0f95e883390c7256b2e441484aac06b1002dbe1d924141a77c8d82f96223".into(),
        numpy_record_sha256: "41e1145d39013f7d909361f1fd4e74c46493bcf426797898b0fb499f670204c5"
            .into(),
        numpy_record_verified_file_count: 916,
        numpy_git_revision: "c81c49f77451340651a751e76bca607d85e4fd55".into(),
        scipy_record_sha256: "81c576349363842874f8638770240686a20ef21499da9987901f94f8e2179ac2"
            .into(),
        scipy_record_verified_file_count: 1425,
        scipy_git_revision: "8c75ae75176236f233824e9a0483c26a69e6dfec".into(),
        scipy_release: true,
        scipy_version_module_sha256:
            "d6a223e725b2f146a5f6d4bc578e5ff77c7165f0a70351e1d1ea3ca1bf95d61a".into(),
        scipy_radau_module_sha256:
            "d0aa4593431ef39ee07825db6ef0324e4a9bacef0e23fda42d377318ba6a6256".into(),
        blas_libraries: vec![
            NumericalReferenceRuntimeLibraryV2 {
                role: "numpy-ilp64".into(),
                basename: "libscipy_openblas64_-096271d3.so".into(),
                version: "0.3.31.dev".into(),
                configuration: "USE64BITINT DYNAMIC_ARCH NO_AFFINITY Haswell MAX_THREADS=64".into(),
                sha256: "c0f0784c075afdeb2d57cb78e6225221f7c97ef8d03e512b3c98e105054e73c2".into(),
            },
            NumericalReferenceRuntimeLibraryV2 {
                role: "scipy-lp64".into(),
                basename: "libscipy_openblas-6cdc3b4a.so".into(),
                version: "0.3.30".into(),
                configuration: "DYNAMIC_ARCH NO_AFFINITY Haswell MAX_THREADS=64".into(),
                sha256: "8fb864c29cac4b25f6e2c139491ea96f2724dde42d51394f84e9c4a622e34790".into(),
            },
        ],
        thread_environment: [
            ("MKL_NUM_THREADS".into(), "1".into()),
            ("OMP_NUM_THREADS".into(), "1".into()),
            ("OPENBLAS_NUM_THREADS".into(), "1".into()),
            ("VECLIB_MAXIMUM_THREADS".into(), "1".into()),
        ]
        .into_iter()
        .collect(),
    }
}

fn numerical_reference_problem_v2(
    spec: &ScientificCaseSpec,
) -> FairResult<NumericalReferenceProblemV2> {
    let built = spec.build()?;
    Ok(NumericalReferenceProblemV2 {
        problem_id: built.problem.name,
        family: spec.family.as_str().into(),
        partition: spec.partition,
        dimension: spec.dimension,
        grid_shape: spec.grid_shape,
        t_span: [spec.t_span.0, spec.t_span.1],
        uniform_output_points: spec.uniform_output_points,
        mandatory_breakpoints: spec.mandatory_breakpoints.clone(),
        source: spec.provenance.clone(),
    })
}

/// Deterministic NOT_RUN layout used by the producer and by schema tests.
/// Zero digests are placeholders, not scientific evidence; the loader admits
/// only a manifest explicitly transitioned to `Complete` after generation.
pub fn numerical_reference_v2_not_run_manifest() -> FairResult<NumericalReferenceManifestV2> {
    let generator = numerical_reference_v2_generator_pins();
    let zero = "0".repeat(64);
    let mut artifact_map = BTreeMap::<String, NumericalReferenceManifestEntryV2>::new();
    let mut bindings = Vec::with_capacity(66);
    for spec in ScientificCorpusV2::all_specs() {
        let problem = numerical_reference_problem_v2(&spec)?;
        let problem_id = problem.problem_id.clone();
        artifact_map.entry(problem_id.clone()).or_insert_with(|| {
            NumericalReferenceManifestEntryV2 {
                artifact_path: format!("artifacts/{problem_id}.json"),
                artifact_sha256: zero.clone(),
                grid_sha256: zero.clone(),
                state_sha256: zero.clone(),
                canonical_method: generator.radau_ladder[2].clone(),
                problem,
            }
        });
        bindings.push(NumericalReferenceCaseBindingV2 {
            case_id: spec.id,
            problem_id,
            reference_checksum_sha256: zero.clone(),
        });
    }
    let artifacts = artifact_map.into_values().collect::<Vec<_>>();
    bindings.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    for binding in &mut bindings {
        let entry = artifacts
            .iter()
            .find(|entry| entry.problem.problem_id == binding.problem_id)
            .expect("artifact map was built from the same cases");
        binding.reference_checksum_sha256 =
            numerical_reference_case_binding_checksum_v2(binding, entry);
    }
    let artifact_set_sha256 = numerical_reference_artifact_set_checksum_v2(&artifacts);
    let binding_set_sha256 = numerical_reference_binding_set_checksum_v2(&bindings);
    let producer = NumericalReferenceProducerIdentityV2 {
        script_path: "tools/reference_v2/generate_references_v2.py".into(),
        script_sha256: sha256_hex(include_bytes!(
            "../../../tools/reference_v2/generate_references_v2.py"
        )),
        implementation_revision: "NOT_RUN".into(),
        problem_definition_sha256: numerical_reference_problem_definition_checksum_v2(&artifacts),
    };
    Ok(NumericalReferenceManifestV2 {
        schema_version: NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION.into(),
        corpus_version: ScientificCorpusV2::VERSION.into(),
        generation_status: NumericalReferenceGenerationStatusV2::NotRun,
        generation_mode: "partition-resume-create-new; self-check-never-regenerates".into(),
        generator,
        runtime: numerical_reference_v2_runtime_identity(),
        producer,
        wrms_policy: NumericalReferenceWrmsPolicyV2 {
            formula_id: NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID.into(),
            absolute: 1.0e-10,
            relative: 1.0e-8,
        },
        artifacts,
        bindings,
        artifact_set_sha256,
        binding_set_sha256,
    })
}

pub fn validate_numerical_reference_manifest_v2(
    manifest: &NumericalReferenceManifestV2,
) -> FairResult<()> {
    if manifest.schema_version != NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION
        || manifest.corpus_version != ScientificCorpusV2::VERSION
        || manifest.generation_mode != "partition-resume-create-new; self-check-never-regenerates"
    {
        return Err(FairError::Invalid(
            "numerical reference v2 manifest identity mismatch".into(),
        ));
    }
    validate_generator_pins(&manifest.generator)?;
    if manifest.runtime != numerical_reference_v2_runtime_identity() {
        return Err(FairError::Invalid(
            "numerical reference v2 exact runtime identity mismatch".into(),
        ));
    }
    let expected_script_sha = sha256_hex(include_bytes!(
        "../../../tools/reference_v2/generate_references_v2.py"
    ));
    if manifest.producer.script_path != "tools/reference_v2/generate_references_v2.py"
        || manifest.producer.script_sha256 != expected_script_sha
        || manifest.producer.problem_definition_sha256
            != numerical_reference_problem_definition_checksum_v2(&manifest.artifacts)
        || match manifest.generation_status {
            NumericalReferenceGenerationStatusV2::NotRun => {
                manifest.producer.implementation_revision != "NOT_RUN"
            }
            NumericalReferenceGenerationStatusV2::Complete => {
                !valid_git_revision(&manifest.producer.implementation_revision)
            }
        }
    {
        return Err(FairError::Invalid(
            "numerical reference v2 producer/source identity mismatch".into(),
        ));
    }
    if manifest.wrms_policy.formula_id != NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID
        || manifest.wrms_policy.absolute.to_bits() != 1.0e-10_f64.to_bits()
        || manifest.wrms_policy.relative.to_bits() != 1.0e-8_f64.to_bits()
    {
        return Err(FairError::Invalid(
            "numerical reference v2 WRMS policy is not tight-L2 anchored".into(),
        ));
    }
    if manifest.artifacts.len() != 22 || manifest.bindings.len() != 66 {
        return Err(FairError::Invalid(
            "numerical reference v2 requires exactly 22 artifacts and 66 bindings".into(),
        ));
    }

    let expected = numerical_reference_v2_not_run_manifest()?;
    let expected_artifacts = expected
        .artifacts
        .iter()
        .map(|entry| (entry.problem.problem_id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let expected_bindings = expected
        .bindings
        .iter()
        .map(|binding| (binding.case_id.as_str(), binding.problem_id.as_str()))
        .collect::<BTreeMap<_, _>>();

    let mut artifacts = BTreeMap::new();
    if !manifest
        .artifacts
        .windows(2)
        .all(|pair| pair[0].problem.problem_id.as_str() < pair[1].problem.problem_id.as_str())
        || !manifest
            .bindings
            .windows(2)
            .all(|pair| pair[0].case_id.as_str() < pair[1].case_id.as_str())
    {
        return Err(FairError::Invalid(
            "numerical reference v2 manifest is not in canonical order".into(),
        ));
    }
    for entry in &manifest.artifacts {
        validate_relative_artifact_path(&entry.artifact_path)?;
        if !valid_sha256(&entry.artifact_sha256)
            || !valid_sha256(&entry.grid_sha256)
            || !valid_sha256(&entry.state_sha256)
            || !methods_match(&entry.canonical_method, &manifest.generator.radau_ladder[2])
        {
            return Err(FairError::Invalid(
                "numerical reference v2 artifact metadata is invalid".into(),
            ));
        }
        let all_not_run_sentinels = entry.artifact_sha256 == ZERO_SHA256
            && entry.grid_sha256 == ZERO_SHA256
            && entry.state_sha256 == ZERO_SHA256;
        let has_any_not_run_sentinel = entry.artifact_sha256 == ZERO_SHA256
            || entry.grid_sha256 == ZERO_SHA256
            || entry.state_sha256 == ZERO_SHA256;
        if match manifest.generation_status {
            NumericalReferenceGenerationStatusV2::NotRun => !all_not_run_sentinels,
            NumericalReferenceGenerationStatusV2::Complete => has_any_not_run_sentinel,
        } {
            return Err(FairError::Invalid(
                "numerical reference v2 artifact digests contradict generation status".into(),
            ));
        }
        let Some(expected_entry) = expected_artifacts.get(entry.problem.problem_id.as_str()) else {
            return Err(FairError::Invalid(
                "numerical reference v2 contains an extra physical problem".into(),
            ));
        };
        if entry.problem != expected_entry.problem
            || entry.artifact_path != expected_entry.artifact_path
        {
            return Err(FairError::Invalid(
                "numerical reference v2 physical problem metadata/path mismatch".into(),
            ));
        }
        if artifacts
            .insert(entry.problem.problem_id.as_str(), entry)
            .is_some()
        {
            return Err(FairError::Invalid(
                "numerical reference v2 has a duplicate physical problem".into(),
            ));
        }
    }
    if artifacts.len() != expected_artifacts.len() {
        return Err(FairError::Invalid(
            "numerical reference v2 physical problem set is incomplete".into(),
        ));
    }

    let mut case_ids = BTreeSet::new();
    let mut indegree = BTreeMap::<&str, usize>::new();
    for binding in &manifest.bindings {
        if !case_ids.insert(binding.case_id.as_str()) {
            return Err(FairError::Invalid(
                "numerical reference v2 has a duplicate case binding".into(),
            ));
        }
        let Some(expected_problem_id) = expected_bindings.get(binding.case_id.as_str()) else {
            return Err(FairError::Invalid(
                "numerical reference v2 contains an extra case binding".into(),
            ));
        };
        if binding.problem_id != *expected_problem_id {
            return Err(FairError::Invalid(
                "numerical reference v2 case is bound to the wrong physical problem".into(),
            ));
        }
        let entry = artifacts.get(binding.problem_id.as_str()).ok_or_else(|| {
            FairError::Invalid("numerical reference v2 binding targets a missing artifact".into())
        })?;
        if !valid_sha256(&binding.reference_checksum_sha256)
            || binding.reference_checksum_sha256
                != numerical_reference_case_binding_checksum_v2(binding, entry)
        {
            return Err(FairError::Invalid(
                "numerical reference v2 case binding checksum mismatch".into(),
            ));
        }
        *indegree.entry(binding.problem_id.as_str()).or_default() += 1;
    }
    if case_ids.len() != expected_bindings.len()
        || artifacts
            .keys()
            .any(|problem_id| indegree.get(problem_id).copied() != Some(3))
    {
        return Err(FairError::Invalid(
            "numerical reference v2 has missing, unreferenced, or non-triplicate bindings".into(),
        ));
    }
    if manifest.artifact_set_sha256
        != numerical_reference_artifact_set_checksum_v2(&manifest.artifacts)
        || manifest.binding_set_sha256
            != numerical_reference_binding_set_checksum_v2(&manifest.bindings)
    {
        return Err(FairError::Invalid(
            "numerical reference v2 aggregate checksum mismatch".into(),
        ));
    }
    Ok(())
}

pub fn validate_numerical_reference_convergence(
    convergence: &NumericalReferenceConvergence,
) -> FairResult<()> {
    let d0 = convergence.d0_max_grid_wrms;
    let d1 = convergence.d1_max_grid_wrms;
    let q = convergence.q;
    if !(d0.is_finite() && d1.is_finite() && d0 > d1 && d1 >= 0.0) {
        return Err(FairError::Invalid(
            "numerical reference requires finite D0 > D1 >= 0".into(),
        ));
    }
    if !(q.is_finite() && (0.0..=0.5).contains(&q)) {
        return Err(FairError::Invalid(
            "numerical reference convergence q must be finite and at most 0.5".into(),
        ));
    }
    let expected_q = d1 / d0;
    if !same_f64(q, expected_q) {
        return Err(FairError::Invalid(
            "numerical reference q does not equal D1 / D0".into(),
        ));
    }
    let expected_richardson = d1 * q / (1.0 - q);
    if !same_f64(convergence.richardson_uncertainty_wrms, expected_richardson) {
        return Err(FairError::Invalid(
            "numerical reference Richardson uncertainty does not equal D1*q/(1-q)".into(),
        ));
    }
    if !(convergence.method_disagreement_wrms.is_finite()
        && convergence.method_disagreement_wrms >= 0.0)
    {
        return Err(FairError::Invalid(
            "numerical reference method disagreement must be finite and nonnegative".into(),
        ));
    }
    let expected_total =
        convergence.richardson_uncertainty_wrms + convergence.method_disagreement_wrms;
    if !same_f64(convergence.reference_uncertainty_wrms, expected_total) {
        return Err(FairError::Invalid(
            "numerical reference uncertainty must equal Richardson plus method disagreement".into(),
        ));
    }
    if !(convergence.wrms_scale.absolute.is_finite()
        && convergence.wrms_scale.absolute > 0.0
        && convergence.wrms_scale.relative.is_finite()
        && convergence.wrms_scale.relative >= 0.0)
    {
        return Err(FairError::Invalid(
            "numerical reference WRMS scale must be finite with positive absolute scale".into(),
        ));
    }
    Ok(())
}

/// Parses, authenticates, and validates the canonical reference for one D holdout spec.
pub fn load_numerical_reference(
    manifest_path: impl AsRef<Path>,
    spec: &ScientificCaseSpec,
) -> FairResult<NumericalReferenceBundle> {
    let manifest_path = manifest_path.as_ref();
    validate_scientific_spec(spec)?;
    let manifest_bytes = fs::read(manifest_path)?;
    let manifest: NumericalReferenceManifest = serde_json::from_slice(&manifest_bytes)?;
    validate_numerical_reference_manifest(&manifest)?;

    let expected_problem_id = expected_holdout(spec.family)?.problem_id;
    let entry = manifest
        .artifacts
        .iter()
        .find(|entry| entry.problem.problem_id == expected_problem_id)
        .ok_or_else(|| {
            FairError::Invalid("numerical reference manifest lacks requested holdout".into())
        })?;
    let manifest_dir = manifest_path.parent().ok_or_else(|| {
        FairError::Invalid("numerical reference manifest has no parent directory".into())
    })?;
    let artifact_path = manifest_dir.join(&entry.artifact_path);
    let artifact_bytes = fs::read(&artifact_path)?;
    if sha256_hex(&artifact_bytes) != entry.artifact_sha256 {
        return Err(FairError::Invalid(format!(
            "numerical reference artifact checksum mismatch: {}",
            artifact_path.display()
        )));
    }
    let artifact: NumericalReferenceArtifact = serde_json::from_slice(&artifact_bytes)?;
    validate_numerical_reference_artifact(&manifest, entry, &artifact, spec)?;

    let output_grid = CommonOutputGrid::new(artifact.requested_times.clone())?;
    let provenance = ReferenceSolutionProvenance {
        problem_id: artifact.problem.problem_id.clone(),
        source_kind: ReferenceSourceKind::HighAccuracyNumerical,
        output_grid_id: output_grid.grid_id.clone(),
        state_checksum: artifact.checksums.state_sha256.clone(),
        reference_uncertainty_wrms: artifact.convergence.reference_uncertainty_wrms,
        numerical: Some(NumericalReferenceProvenance {
            manifest_schema_version: manifest.schema_version,
            artifact_schema_version: artifact.schema_version,
            artifact_sha256: entry.artifact_sha256.clone(),
            source_definition_id: artifact.problem.source.source_definition_id.clone(),
            generator: manifest.generator,
            canonical_method: artifact.canonical_method.clone(),
            independent_method: artifact.independent_method.clone(),
            checksums: artifact.checksums.clone(),
            convergence: artifact.convergence.clone(),
            corpus_version: None,
            case_id: None,
            reference_checksum_sha256: None,
            wrms_formula_id: None,
            anchor_state_sha256: None,
        }),
    };
    let error_scale = ExternalErrorScale::with_reference_uncertainty(
        vec![artifact.convergence.wrms_scale.absolute; artifact.problem.dimension],
        artifact.convergence.wrms_scale.relative,
        artifact.convergence.reference_uncertainty_wrms,
    )?;
    let trajectory = ReferenceTrajectory {
        output_grid,
        states: artifact.states,
        provenance,
    };
    validate_numerical_reference_error_scale(&trajectory.provenance, &error_scale)?;
    Ok(NumericalReferenceBundle {
        trajectory,
        error_scale,
    })
}

fn validate_scientific_spec_v2(spec: &ScientificCaseSpec) -> FairResult<()> {
    let expected = ScientificCorpusV2::all_specs()
        .into_iter()
        .find(|expected| expected.id == spec.id)
        .ok_or_else(|| FairError::Invalid("case id is outside ScientificCorpusV2.1".into()))?;
    if &expected != spec {
        return Err(FairError::Invalid(
            "scientific case metadata differs from its exact v2.1 case binding".into(),
        ));
    }
    Ok(())
}

fn convergence_v2_as_legacy(
    convergence: &NumericalReferenceConvergenceV2,
) -> NumericalReferenceConvergence {
    NumericalReferenceConvergence {
        d0_max_grid_wrms: convergence.d0_max_grid_wrms,
        d1_max_grid_wrms: convergence.d1_max_grid_wrms,
        q: convergence.q,
        richardson_uncertainty_wrms: convergence.richardson_uncertainty_wrms,
        method_disagreement_wrms: convergence.method_disagreement_wrms,
        reference_uncertainty_wrms: convergence.reference_uncertainty_wrms,
        wrms_scale: NumericalReferenceWrmsScale {
            absolute: convergence.wrms_basis.absolute,
            relative: convergence.wrms_basis.relative,
        },
    }
}

fn validate_numerical_reference_artifact_v2(
    manifest: &NumericalReferenceManifestV2,
    entry: &NumericalReferenceManifestEntryV2,
    artifact: &NumericalReferenceArtifactV2,
    spec: &ScientificCaseSpec,
) -> FairResult<()> {
    if artifact.schema_version != NUMERICAL_REFERENCE_V2_ARTIFACT_SCHEMA_VERSION
        || artifact.problem_id != entry.problem.problem_id
        || !methods_match(
            &artifact.canonical_method,
            &manifest.generator.radau_ladder[2],
        )
        || !methods_match(&artifact.canonical_method, &entry.canonical_method)
        || !methods_match(
            &artifact.independent_method,
            &manifest.generator.tight_lsoda,
        )
    {
        return Err(FairError::Invalid(
            "numerical reference v2 artifact identity or method mismatch".into(),
        ));
    }
    validate_requested_times(&artifact.requested_times, spec)?;
    if artifact.states.len() != artifact.requested_times.len()
        || artifact.states.iter().any(|state| {
            state.len() != spec.dimension || !state.iter().all(|value| value.is_finite())
        })
    {
        return Err(FairError::Invalid(
            "numerical reference v2 state table has invalid shape or values".into(),
        ));
    }
    let grid_sha256 = numerical_reference_grid_checksum(&artifact.requested_times);
    let state_sha256 = numerical_reference_state_checksum(&artifact.states);
    if artifact.checksums.grid_sha256 != grid_sha256
        || artifact.checksums.state_sha256 != state_sha256
        || entry.grid_sha256 != grid_sha256
        || entry.state_sha256 != state_sha256
    {
        return Err(FairError::Invalid(
            "numerical reference v2 state/grid checksum mismatch".into(),
        ));
    }
    let basis = &artifact.convergence.wrms_basis;
    if basis.formula_id != manifest.wrms_policy.formula_id
        || basis.absolute.to_bits() != manifest.wrms_policy.absolute.to_bits()
        || basis.relative.to_bits() != manifest.wrms_policy.relative.to_bits()
        || basis.anchor_state_sha256 != state_sha256
    {
        return Err(FairError::Invalid(
            "numerical reference v2 WRMS basis is not the tight L2 state table".into(),
        ));
    }
    validate_numerical_reference_convergence(&convergence_v2_as_legacy(&artifact.convergence))?;

    let expected_labels = manifest
        .generator
        .radau_ladder
        .iter()
        .chain(std::iter::once(&manifest.generator.tight_lsoda))
        .map(|method| method.label.as_str())
        .collect::<BTreeSet<_>>();
    let mut labels = BTreeSet::new();
    for run in &artifact.run_evidence {
        let complete = run.status == NumericalReferenceRunStatusV2::Complete
            && run
                .wall_seconds
                .is_some_and(|value| value.is_finite() && value >= 0.0)
            && run.nfev.is_some()
            && run.njev.is_some()
            && run.nlu.is_some()
            && run.process_peak_rss_bytes_at_run_end.is_some()
            && run.message.is_none();
        if !complete || !labels.insert(run.label.as_str()) {
            return Err(FairError::Invalid(
                "numerical reference v2 run evidence is incomplete or duplicated".into(),
            ));
        }
    }
    if labels != expected_labels {
        return Err(FairError::Invalid(
            "numerical reference v2 run evidence does not contain the complete solver ladder"
                .into(),
        ));
    }
    Ok(())
}

/// Loads one exact v2.1 case binding and only its selected physical artifact.
/// The manifest is authenticated in full, but unrelated artifact files are not
/// opened; this is what keeps calibration code from touching holdout states.
pub fn load_numerical_reference_v2(
    manifest_path: impl AsRef<Path>,
    spec: &ScientificCaseSpec,
) -> FairResult<NumericalReferenceBundleV2> {
    validate_scientific_spec_v2(spec)?;
    let manifest_path = manifest_path.as_ref();
    let manifest_bytes = fs::read(manifest_path)?;
    let wire: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;
    if wire
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        != Some(NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION)
    {
        return Err(FairError::Invalid(
            "v2 loader rejects legacy or unknown numerical-reference schemas".into(),
        ));
    }
    let manifest: NumericalReferenceManifestV2 = serde_json::from_slice(&manifest_bytes)?;
    validate_numerical_reference_manifest_v2(&manifest)?;
    if manifest.generation_status != NumericalReferenceGenerationStatusV2::Complete {
        return Err(FairError::Invalid(
            "numerical reference v2 manifest is NOT_RUN".into(),
        ));
    }
    let binding = manifest
        .bindings
        .iter()
        .find(|binding| binding.case_id == spec.id)
        .ok_or_else(|| {
            FairError::Invalid("v2 manifest lacks the exact requested case id".into())
        })?;
    let entry = manifest
        .artifacts
        .iter()
        .find(|entry| entry.problem.problem_id == binding.problem_id)
        .ok_or_else(|| FairError::Invalid("v2 binding targets a missing artifact".into()))?;
    let manifest_dir = manifest_path.parent().ok_or_else(|| {
        FairError::Invalid("numerical reference v2 manifest has no parent directory".into())
    })?;
    let artifact_path = manifest_dir.join(&entry.artifact_path);
    let artifact_bytes = fs::read(&artifact_path)?;
    if sha256_hex(&artifact_bytes) != entry.artifact_sha256 {
        return Err(FairError::Invalid(format!(
            "numerical reference v2 artifact checksum mismatch: {}",
            artifact_path.display()
        )));
    }
    let artifact: NumericalReferenceArtifactV2 = serde_json::from_slice(&artifact_bytes)?;
    validate_numerical_reference_artifact_v2(&manifest, entry, &artifact, spec)?;

    let output_grid = CommonOutputGrid::new(artifact.requested_times.clone())?;
    let legacy_convergence = convergence_v2_as_legacy(&artifact.convergence);
    let provenance = ReferenceSolutionProvenance {
        problem_id: entry.problem.problem_id.clone(),
        source_kind: ReferenceSourceKind::HighAccuracyNumerical,
        output_grid_id: output_grid.grid_id.clone(),
        state_checksum: artifact.checksums.state_sha256.clone(),
        reference_uncertainty_wrms: artifact.convergence.reference_uncertainty_wrms,
        numerical: Some(NumericalReferenceProvenance {
            manifest_schema_version: manifest.schema_version.clone(),
            artifact_schema_version: artifact.schema_version.clone(),
            artifact_sha256: entry.artifact_sha256.clone(),
            source_definition_id: entry.problem.source.source_path.clone(),
            generator: manifest.generator.clone(),
            canonical_method: artifact.canonical_method.clone(),
            independent_method: artifact.independent_method.clone(),
            checksums: artifact.checksums.clone(),
            convergence: legacy_convergence,
            corpus_version: Some(manifest.corpus_version.clone()),
            case_id: Some(binding.case_id.clone()),
            reference_checksum_sha256: Some(binding.reference_checksum_sha256.clone()),
            wrms_formula_id: Some(artifact.convergence.wrms_basis.formula_id.clone()),
            anchor_state_sha256: Some(artifact.convergence.wrms_basis.anchor_state_sha256.clone()),
        }),
    };
    let error_scale = ExternalErrorScale::with_reference_uncertainty(
        vec![artifact.convergence.wrms_basis.absolute; spec.dimension],
        artifact.convergence.wrms_basis.relative,
        artifact.convergence.reference_uncertainty_wrms,
    )?;
    let trajectory = ReferenceTrajectory {
        output_grid: output_grid.clone(),
        states: artifact.states,
        provenance,
    };
    validate_numerical_reference_error_scale(&trajectory.provenance, &error_scale)?;
    let wrms_basis =
        ReferenceWrmsBasis::new(output_grid, trajectory.states.clone(), error_scale.clone())?;
    Ok(NumericalReferenceBundleV2 {
        case_id: binding.case_id.clone(),
        problem_id: binding.problem_id.clone(),
        reference_checksum_sha256: binding.reference_checksum_sha256.clone(),
        implementation_revision: manifest.producer.implementation_revision.clone(),
        trajectory,
        error_scale,
        wrms_basis,
    })
}

pub fn validate_numerical_reference_manifest(
    manifest: &NumericalReferenceManifest,
) -> FairResult<()> {
    if manifest.schema_version != NUMERICAL_REFERENCE_MANIFEST_SCHEMA_VERSION {
        return Err(FairError::Invalid(
            "unsupported numerical reference manifest schema".into(),
        ));
    }
    if manifest.generation_mode != "explicit-generate; self-check-never-regenerates" {
        return Err(FairError::Invalid(
            "numerical reference manifest generation mode is not explicit and immutable".into(),
        ));
    }
    validate_generator_pins(&manifest.generator)?;
    if manifest.artifacts.len() != ScientificFamily::HOLDOUT.len() {
        return Err(FairError::Invalid(
            "numerical reference manifest must contain exactly the four holdouts".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for entry in &manifest.artifacts {
        let family = family_from_name(&entry.problem.family)?;
        let expected = expected_holdout(family)?;
        validate_problem_against_expected(&entry.problem, expected)?;
        validate_relative_artifact_path(&entry.artifact_path)?;
        if !valid_sha256(&entry.artifact_sha256)
            || !valid_sha256(&entry.grid_sha256)
            || !valid_sha256(&entry.state_sha256)
        {
            return Err(FairError::Invalid(
                "numerical reference manifest checksums must be lowercase SHA-256".into(),
            ));
        }
        if !methods_match(&entry.canonical_method, &manifest.generator.radau_ladder[2]) {
            return Err(FairError::Invalid(
                "numerical reference manifest canonical method is not tight Radau L2".into(),
            ));
        }
        if !seen.insert(entry.problem.family.clone()) {
            return Err(FairError::Invalid(
                "numerical reference manifest has duplicate holdout entries".into(),
            ));
        }
    }
    let expected_families = ScientificFamily::HOLDOUT
        .iter()
        .map(|family| family.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    if seen != expected_families {
        return Err(FairError::Invalid(
            "numerical reference manifest holdout set does not match ScientificCorpusV2".into(),
        ));
    }
    if !valid_sha256(&manifest.artifact_set_sha256)
        || manifest.artifact_set_sha256
            != numerical_reference_artifact_set_checksum(&manifest.artifacts)
    {
        return Err(FairError::Invalid(
            "numerical reference aggregate artifact-set checksum mismatch".into(),
        ));
    }
    Ok(())
}

fn validate_numerical_reference_artifact(
    manifest: &NumericalReferenceManifest,
    entry: &NumericalReferenceManifestEntry,
    artifact: &NumericalReferenceArtifact,
    spec: &ScientificCaseSpec,
) -> FairResult<()> {
    if artifact.schema_version != NUMERICAL_REFERENCE_ARTIFACT_SCHEMA_VERSION {
        return Err(FairError::Invalid(
            "unsupported numerical reference artifact schema".into(),
        ));
    }
    if !problems_match(&artifact.problem, &entry.problem) {
        return Err(FairError::Invalid(
            "numerical reference artifact problem identity differs from manifest".into(),
        ));
    }
    validate_problem_against_spec(&artifact.problem, spec)?;
    if !methods_match(
        &artifact.canonical_method,
        &manifest.generator.radau_ladder[2],
    ) || !methods_match(&artifact.canonical_method, &entry.canonical_method)
    {
        return Err(FairError::Invalid(
            "numerical reference canonical source is not tight Radau L2".into(),
        ));
    }
    if !methods_match(
        &artifact.independent_method,
        &manifest.generator.tight_lsoda,
    ) {
        return Err(FairError::Invalid(
            "numerical reference independent source is not the pinned tight LSODA run".into(),
        ));
    }
    validate_requested_times(&artifact.requested_times, spec)?;
    if artifact.states.len() != artifact.requested_times.len()
        || artifact.states.iter().any(|state| {
            state.len() != artifact.problem.dimension
                || !state.iter().all(|value| value.is_finite())
        })
    {
        return Err(FairError::Invalid(
            "numerical reference state table has invalid shape or non-finite values".into(),
        ));
    }
    if !valid_sha256(&artifact.checksums.grid_sha256)
        || !valid_sha256(&artifact.checksums.state_sha256)
    {
        return Err(FairError::Invalid(
            "numerical reference state/grid checksums must be lowercase SHA-256".into(),
        ));
    }
    if artifact.checksums.grid_sha256
        != numerical_reference_grid_checksum(&artifact.requested_times)
        || artifact.checksums.state_sha256 != numerical_reference_state_checksum(&artifact.states)
    {
        return Err(FairError::Invalid(
            "numerical reference state or grid checksum mismatch".into(),
        ));
    }
    if entry.grid_sha256 != artifact.checksums.grid_sha256
        || entry.state_sha256 != artifact.checksums.state_sha256
    {
        return Err(FairError::Invalid(
            "numerical reference manifest state/grid checksum differs from artifact".into(),
        ));
    }
    validate_numerical_reference_convergence(&artifact.convergence)
}

fn validate_generator_pins(pins: &NumericalReferenceGeneratorPins) -> FairResult<()> {
    if pins.python != "3.12"
        || pins.numpy != "2.4.2"
        || pins.scipy != "1.17.0"
        || pins.blas_threads != 1
    {
        return Err(FairError::Invalid(
            "numerical reference generator runtime pins do not match the contract".into(),
        ));
    }
    if pins.radau_ladder.len() != 3 {
        return Err(FairError::Invalid(
            "numerical reference requires exactly three Radau tolerance levels".into(),
        ));
    }
    for (expected_label, level) in ["L0", "L1", "L2"].iter().zip(&pins.radau_ladder) {
        if level.label != *expected_label || level.method != "Radau" || !valid_tolerance(level) {
            return Err(FairError::Invalid(
                "numerical reference Radau ladder is invalid".into(),
            ));
        }
    }
    for pair in pins.radau_ladder.windows(2) {
        if !(pair[1].rtol < pair[0].rtol && pair[1].atol < pair[0].atol) {
            return Err(FairError::Invalid(
                "numerical reference Radau ladder is not progressively tighter".into(),
            ));
        }
    }
    let tight_radau = &pins.radau_ladder[2];
    if !(tight_radau.rtol <= 1.0e-10 && tight_radau.atol <= 1.0e-12) {
        return Err(FairError::Invalid(
            "numerical reference tight Radau tolerance is insufficiently strict".into(),
        ));
    }
    if pins.tight_lsoda.label != "tight-lsoda"
        || pins.tight_lsoda.method != "LSODA"
        || !valid_tolerance(&pins.tight_lsoda)
    {
        return Err(FairError::Invalid(
            "numerical reference tight LSODA pin is invalid".into(),
        ));
    }
    Ok(())
}

fn valid_tolerance(method: &NumericalReferenceMethod) -> bool {
    method.rtol.is_finite() && method.rtol > 0.0 && method.atol.is_finite() && method.atol > 0.0
}

fn validate_scientific_spec(spec: &ScientificCaseSpec) -> FairResult<()> {
    let expected = expected_holdout(spec.family)?;
    if spec.partition != CorpusPartition::Holdout
        || spec.dimension != expected.dimension
        || !same_f64_bits(spec.t_span.0, expected.t_span[0])
        || !same_f64_bits(spec.t_span.1, expected.t_span[1])
        || spec.uniform_output_points != 101
        || !same_f64_vectors(&spec.mandatory_breakpoints, expected.mandatory_breakpoints)
    {
        return Err(FairError::Invalid(
            "scientific holdout spec is outside the numerical-reference contract".into(),
        ));
    }
    let expected_times = canonical_requested_times(expected.t_span, expected.mandatory_breakpoints);
    if !same_f64_vectors(&spec.output_times, &expected_times) {
        return Err(FairError::Invalid(
            "scientific holdout output grid is outside the numerical-reference contract".into(),
        ));
    }
    validate_source_against_expected(&spec.provenance, expected.source)
}

fn validate_problem_against_spec(
    problem: &NumericalReferenceProblem,
    spec: &ScientificCaseSpec,
) -> FairResult<()> {
    validate_scientific_spec(spec)?;
    let expected = expected_holdout(spec.family)?;
    validate_problem_against_expected(problem, expected)
}

fn validate_problem_against_expected(
    problem: &NumericalReferenceProblem,
    expected: ExpectedHoldout,
) -> FairResult<()> {
    if problem.problem_id != expected.problem_id
        || problem.family != expected.family.as_str()
        || problem.dimension != expected.dimension
        || !same_f64_bits(problem.t_span[0], expected.t_span[0])
        || !same_f64_bits(problem.t_span[1], expected.t_span[1])
        || problem.uniform_output_points != 101
        || !same_f64_vectors(
            &problem.mandatory_breakpoints,
            expected.mandatory_breakpoints,
        )
        || !source_matches_expected(&problem.source, expected.source)
    {
        return Err(FairError::Invalid(
            "numerical reference problem identity is outside the holdout contract".into(),
        ));
    }
    Ok(())
}

fn validate_source_against_expected(
    source: &rodas5p_integrators::ScientificSourceProvenance,
    expected: ExpectedSource,
) -> FairResult<()> {
    if source.source_repository != expected.source_repository
        || source.source_revision != expected.source_revision
        || source.source_path != expected.source_path
        || source.source_blob.as_deref() != expected.source_blob
        || source.source_sha256.as_deref() != Some(expected.source_sha256)
    {
        return Err(FairError::Invalid(
            "scientific holdout source provenance is outside the numerical-reference contract"
                .into(),
        ));
    }
    Ok(())
}

fn source_matches_expected(
    source: &NumericalReferenceSourceDefinition,
    expected: ExpectedSource,
) -> bool {
    source.source_definition_id == expected.source_definition_id
        && source.source_repository == expected.source_repository
        && source.source_revision == expected.source_revision
        && source.source_path == expected.source_path
        && source.source_blob.as_deref() == expected.source_blob
        && source.source_sha256 == expected.source_sha256
}

fn validate_requested_times(times: &[f64], spec: &ScientificCaseSpec) -> FairResult<()> {
    if !same_f64_vectors(times, &spec.output_times)
        || times.iter().any(|time| !time.is_finite())
        || times.windows(2).any(|pair| pair[0] >= pair[1])
        || spec.mandatory_breakpoints.iter().any(|breakpoint| {
            !times
                .iter()
                .any(|time| time.to_bits() == breakpoint.to_bits())
        })
    {
        return Err(FairError::Invalid(
            "numerical reference requested times are missing, reordered, duplicated, or lack a breakpoint"
                .into(),
        ));
    }
    Ok(())
}

fn validate_relative_artifact_path(path: &str) -> FairResult<()> {
    let path = Path::new(path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(FairError::Invalid(
            "numerical reference artifact path must be a manifest-relative path".into(),
        ));
    }
    Ok(())
}

fn problems_match(left: &NumericalReferenceProblem, right: &NumericalReferenceProblem) -> bool {
    left.problem_id == right.problem_id
        && left.family == right.family
        && left.dimension == right.dimension
        && same_f64_bits(left.t_span[0], right.t_span[0])
        && same_f64_bits(left.t_span[1], right.t_span[1])
        && left.uniform_output_points == right.uniform_output_points
        && same_f64_vectors(&left.mandatory_breakpoints, &right.mandatory_breakpoints)
        && left.source == right.source
}

fn methods_match(left: &NumericalReferenceMethod, right: &NumericalReferenceMethod) -> bool {
    left.label == right.label
        && left.method == right.method
        && same_f64_bits(left.rtol, right.rtol)
        && same_f64_bits(left.atol, right.atol)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_git_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn same_f64(left: f64, right: f64) -> bool {
    if !left.is_finite() || !right.is_finite() {
        return false;
    }
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 128.0 * f64::EPSILON * scale
}

fn same_f64_bits(left: f64, right: f64) -> bool {
    left.is_finite() && right.is_finite() && left.to_bits() == right.to_bits()
}

fn same_f64_vectors(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn family_from_name(name: &str) -> FairResult<ScientificFamily> {
    ScientificFamily::HOLDOUT
        .into_iter()
        .find(|family| family.as_str() == name)
        .ok_or_else(|| FairError::Invalid("unknown numerical-reference holdout family".into()))
}

fn canonical_requested_times(t_span: [f64; 2], breakpoints: &[f64]) -> Vec<f64> {
    let mut times = (0..101)
        .map(|index| t_span[0] + (t_span[1] - t_span[0]) * index as f64 / 100.0)
        .collect::<Vec<_>>();
    for &breakpoint in breakpoints {
        if !times
            .iter()
            .any(|time| time.to_bits() == breakpoint.to_bits())
        {
            times.push(breakpoint);
        }
    }
    times.sort_by(f64::total_cmp);
    times
}

#[derive(Clone, Copy)]
struct ExpectedSource {
    source_definition_id: &'static str,
    source_repository: &'static str,
    source_revision: &'static str,
    source_path: &'static str,
    source_blob: Option<&'static str>,
    source_sha256: &'static str,
}

#[derive(Clone, Copy)]
struct ExpectedHoldout {
    family: ScientificFamily,
    problem_id: &'static str,
    dimension: usize,
    t_span: [f64; 2],
    mandatory_breakpoints: &'static [f64],
    source: ExpectedSource,
}

fn expected_holdout(family: ScientificFamily) -> FairResult<ExpectedHoldout> {
    let (problem_id, dimension, t_span, mandatory_breakpoints, source): (
        &'static str,
        usize,
        [f64; 2],
        &'static [f64],
        ExpectedSource,
    ) = match family {
        ScientificFamily::Oregonator => (
            "oregonator-holdout-v2",
            3,
            [0.0, 360.0],
            &[][..],
            ExpectedSource {
                source_definition_id: "bari-stiff-ode/orego.f@aa58d9090f1f581f2e60e29b02b409466197981f5399120ce66bfb2d34f41c27",
                source_repository: "Bari stiff ODE test set",
                source_revision: "orego.f file identity",
                source_path: "orego.f",
                source_blob: None,
                source_sha256: "aa58d9090f1f581f2e60e29b02b409466197981f5399120ce66bfb2d34f41c27",
            },
        ),
        ScientificFamily::Pollution => (
            "pollution-holdout-v2",
            20,
            [0.0, 60.0],
            &[][..],
            ExpectedSource {
                source_definition_id: "bari-stiff-ode/pollu.f@2aba777ee6de34e0ee074951375e029ad5171e937dabb7ab4c6461c0736e6c20",
                source_repository: "Bari stiff ODE test set",
                source_revision: "pollu.f file identity",
                source_path: "pollu.f",
                source_blob: None,
                source_sha256: "2aba777ee6de34e0ee074951375e029ad5171e937dabb7ab4c6461c0736e6c20",
            },
        ),
        ScientificFamily::MedicalAkzo => (
            "medical-akzo-holdout-v2",
            400,
            [0.0, 20.0],
            &[5.0],
            ExpectedSource {
                source_definition_id: "bari-stiff-ode/medakzo.f@3b5a4aa80769cd752e17a64a2ae15b4b07ba2a15f037aed48b7c2158d739861a",
                source_repository: "Bari stiff ODE test set",
                source_revision: "medakzo.f file identity",
                source_path: "medakzo.f",
                source_blob: None,
                source_sha256: "3b5a4aa80769cd752e17a64a2ae15b4b07ba2a15f037aed48b7c2158d739861a",
            },
        ),
        ScientificFamily::Brusselator2d => (
            "brusselator-2d-holdout-v2",
            512,
            [0.0, 11.5],
            &[1.1],
            ExpectedSource {
                source_definition_id: "sciml-scimlsensitivity/brusselator.md@fea9aaa141f224a97f112e024082966a1a5ee6c2",
                source_repository: "SciML/SciMLSensitivity.jl",
                source_revision: "63a13a7301a17feb8cb5e3a4b3ccef4487ae0c52",
                source_path: "docs/src/examples/pde/brusselator.md",
                source_blob: Some("fea9aaa141f224a97f112e024082966a1a5ee6c2"),
                source_sha256: "688e4642b669e4181cca67d0d7cd9d663e2322d70923daf0240e5a995627351e",
            },
        ),
        _ => {
            return Err(FairError::Invalid(
                "numerical reference is only defined for ScientificCorpusV2 holdouts".into(),
            ));
        }
    };
    Ok(ExpectedHoldout {
        family,
        problem_id,
        dimension,
        t_span,
        mandatory_breakpoints,
        source,
    })
}
