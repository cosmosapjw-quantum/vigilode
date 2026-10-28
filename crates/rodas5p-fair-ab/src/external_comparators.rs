//! Strict, read-only evidence contract for independently executed external comparators.
//!
//! This module never launches SciPy, SUNDIALS, Python, or a native executable.  It only admits
//! an already-produced JSON artifact when every runtime, source, problem, grid, reference,
//! tolerance, output-policy, checksum, lineage, status, and native-work field matches the caller's
//! frozen expectation.

use std::{collections::BTreeSet, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    FairError, FairResult, NumericalReferenceRuntimeIdentityV2, numerical_reference_grid_checksum,
    numerical_reference_state_checksum, numerical_reference_v2_not_run_manifest,
};

pub const EXTERNAL_COMPARATOR_EVIDENCE_SCHEMA_VERSION: &str =
    "vigilode-external-comparator-evidence-v1";
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExternalComparatorKind {
    ScipyRadau,
    SundialsCvode,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExternalRuntimeIdentity {
    ScipyPython {
        identity: NumericalReferenceRuntimeIdentityV2,
    },
    SundialsHostProbe {
        cvode_available: bool,
        executable_names_checked: Vec<String>,
        pkg_config_modules_checked: Vec<String>,
        header_paths_checked: Vec<String>,
        library_names_checked: Vec<String>,
        python_modules_checked: Vec<String>,
        ida_only_version: Option<String>,
        probe_findings: Vec<SundialsProbeFinding>,
        probe_evidence_sha256: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SundialsProbeFinding {
    pub category: String,
    pub target: String,
    pub observed: bool,
    pub detail: String,
}

#[derive(Serialize)]
struct SundialsProbeChecksumPayload<'a> {
    cvode_available: bool,
    executable_names_checked: &'a [String],
    pkg_config_modules_checked: &'a [String],
    header_paths_checked: &'a [String],
    library_names_checked: &'a [String],
    python_modules_checked: &'a [String],
    ida_only_version: &'a Option<String>,
    probe_findings: &'a [SundialsProbeFinding],
}

#[allow(clippy::too_many_arguments)]
pub fn sundials_probe_evidence_checksum(
    cvode_available: bool,
    executable_names_checked: &[String],
    pkg_config_modules_checked: &[String],
    header_paths_checked: &[String],
    library_names_checked: &[String],
    python_modules_checked: &[String],
    ida_only_version: &Option<String>,
    probe_findings: &[SundialsProbeFinding],
) -> FairResult<String> {
    let payload = SundialsProbeChecksumPayload {
        cvode_available,
        executable_names_checked,
        pkg_config_modules_checked,
        header_paths_checked,
        library_names_checked,
        python_modules_checked,
        ida_only_version,
        probe_findings,
    };
    let mut bytes = b"vigilode-sundials-host-probe-v1\0".to_vec();
    bytes.extend_from_slice(&serde_json::to_vec(&payload)?);
    Ok(rodas5p_core::sha256_hex(&bytes))
}

#[allow(clippy::too_many_arguments)]
fn validate_sundials_probe_coverage(
    cvode_available: bool,
    executable_names_checked: &[String],
    pkg_config_modules_checked: &[String],
    header_paths_checked: &[String],
    library_names_checked: &[String],
    python_modules_checked: &[String],
    probe_findings: &[SundialsProbeFinding],
) -> FairResult<()> {
    let mut expected = BTreeSet::new();
    for (category, targets) in [
        ("executable", executable_names_checked),
        ("pkg-config", pkg_config_modules_checked),
        ("header", header_paths_checked),
        ("library", library_names_checked),
        ("python-module", python_modules_checked),
    ] {
        for target in targets {
            if target.trim().is_empty() || !expected.insert((category, target.as_str())) {
                return invalid("SUNDIALS host probe target coverage is empty or duplicated");
            }
        }
    }
    let mut observed = BTreeSet::new();
    for finding in probe_findings {
        if finding.detail.trim().is_empty()
            || !observed.insert((finding.category.as_str(), finding.target.as_str()))
        {
            return invalid("SUNDIALS host probe finding is empty or duplicated");
        }
        if !cvode_available && finding.observed {
            return invalid("CVODE-unavailable probe contains a positive CVODE finding");
        }
    }
    if observed != expected {
        return invalid("SUNDIALS host probe findings do not cover every declared target exactly");
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalRunnerBinding {
    pub runner_id: String,
    pub version: String,
    pub build_id: String,
    pub implementation_lineage_id: String,
    pub script_path: String,
    pub script_sha256: String,
    /// Domain-separated digest of every local source/oracle byte dependency
    /// imported by the external runner, distinct from its entrypoint digest.
    pub dependency_closure_sha256: String,
    pub source_repository: String,
    pub source_revision: String,
    pub source_sha256: String,
    /// Whether version/build/source fields identify an actually observed
    /// installed upstream implementation rather than a requested target.
    pub observed_upstream_identity: bool,
    pub runtime: ExternalRuntimeIdentity,
    pub runtime_identity_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalRunnerDependency {
    pub path: String,
    pub sha256: String,
}

pub fn external_runner_dependency_closure_checksum(
    dependencies: &[ExternalRunnerDependency],
) -> FairResult<String> {
    if dependencies.is_empty() {
        return invalid("external runner dependency closure is empty");
    }
    let mut previous: Option<&str> = None;
    for dependency in dependencies {
        let path = dependency.path.as_str();
        if path.trim().is_empty()
            || path.starts_with('/')
            || path.contains('\\')
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || !valid_sha256(&dependency.sha256)
            || previous.is_some_and(|value| value >= path)
        {
            return invalid(
                "external runner dependency closure is malformed, duplicated, or unsorted",
            );
        }
        previous = Some(path);
    }
    let mut bytes = b"vigilode-external-runner-dependency-closure-v1\0".to_vec();
    bytes.extend_from_slice(&serde_json::to_vec(dependencies)?);
    Ok(rodas5p_core::sha256_hex(&bytes))
}

pub fn external_runtime_identity_checksum(runtime: &ExternalRuntimeIdentity) -> FairResult<String> {
    let mut bytes = b"vigilode-external-runtime-identity-v1\0".to_vec();
    bytes.extend_from_slice(&serde_json::to_vec(runtime)?);
    Ok(rodas5p_core::sha256_hex(&bytes))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalProblemBinding {
    pub case_id: String,
    pub problem_id: String,
    pub implementation_revision: String,
    pub dimension: usize,
    pub t_span: [f64; 2],
    pub problem_source_sha256: String,
    pub has_mass_matrix: bool,
    pub requested_times: Vec<f64>,
    pub output_grid_id: String,
    pub reference_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalToleranceBinding {
    pub rtol: f64,
    pub atol: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalDenseOutputPolicy {
    pub interpolation: String,
    pub solver_dense_output: bool,
    pub controller_step_clipping: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExternalMassTreatment {
    Identity,
    TransformedIdentity {
        transform_id: String,
        transform_source_sha256: String,
    },
    NonApplicable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalReferenceDependency {
    pub reference_lineage_id: String,
    pub runner_lineage_id: String,
    pub shares_implementation_lineage: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "detail",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum ExternalRunStatus {
    Success,
    Unavailable { reason: String },
    NotRun { reason: String },
    SolverFailure { reason: String },
    NonApplicable { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExternalNativeWork {
    ScipyRadau {
        nfev: u64,
        njev: u64,
        nlu: u64,
    },
    SundialsCvode {
        nst: u64,
        nfe: u64,
        nje: u64,
        nni: u64,
        ncfn: u64,
        netf: u64,
        nli: u64,
        nsetups: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalEvidenceChecksums {
    pub grid_sha256: String,
    pub committed_grid_sha256: Option<String>,
    pub state_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalComparatorContract {
    pub comparator: ExternalComparatorKind,
    pub runner: ExternalRunnerBinding,
    pub problem: ExternalProblemBinding,
    pub tolerance: ExternalToleranceBinding,
    pub dense_output: ExternalDenseOutputPolicy,
    pub mass_treatment: ExternalMassTreatment,
    pub reference_lineage_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalComparatorEvidence {
    pub schema_version: String,
    pub comparator: ExternalComparatorKind,
    pub runner: ExternalRunnerBinding,
    pub problem: ExternalProblemBinding,
    pub tolerance: ExternalToleranceBinding,
    pub dense_output: ExternalDenseOutputPolicy,
    pub mass_treatment: ExternalMassTreatment,
    pub reference_dependency: ExternalReferenceDependency,
    pub status: ExternalRunStatus,
    pub checksums: ExternalEvidenceChecksums,
    pub committed_times: Option<Vec<f64>>,
    pub states: Option<Vec<Vec<f64>>>,
    pub native_work: Option<ExternalNativeWork>,
}

pub fn load_external_comparator_evidence(
    path: impl AsRef<Path>,
    contract: &ExternalComparatorContract,
) -> FairResult<ExternalComparatorEvidence> {
    validate_contract(contract)?;
    let bytes = fs::read(path)?;
    let evidence: ExternalComparatorEvidence = serde_json::from_slice(&bytes)?;
    validate_evidence(&evidence, contract)?;
    Ok(evidence)
}

fn validate_contract(contract: &ExternalComparatorContract) -> FairResult<()> {
    validate_runner(contract.comparator, &contract.runner)?;
    validate_problem(&contract.problem)?;
    if !(contract.tolerance.rtol.is_finite()
        && contract.tolerance.rtol > 0.0
        && contract.tolerance.atol.is_finite()
        && contract.tolerance.atol > 0.0)
    {
        return invalid("external comparator tolerances must be finite and positive");
    }
    if contract.dense_output.interpolation.trim().is_empty()
        || !contract.dense_output.solver_dense_output
        || contract.dense_output.controller_step_clipping
    {
        return invalid(
            "external comparator must declare solver dense output without controller clipping",
        );
    }
    validate_mass_treatment(contract.problem.has_mass_matrix, &contract.mass_treatment)?;
    if contract.reference_lineage_id.trim().is_empty() {
        return invalid("external comparator reference lineage must be nonempty");
    }
    Ok(())
}

fn validate_runner(
    comparator: ExternalComparatorKind,
    runner: &ExternalRunnerBinding,
) -> FairResult<()> {
    if [
        runner.runner_id.as_str(),
        runner.version.as_str(),
        runner.build_id.as_str(),
        runner.implementation_lineage_id.as_str(),
        runner.script_path.as_str(),
        runner.source_repository.as_str(),
        runner.source_revision.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
        || !valid_sha256(&runner.script_sha256)
        || !valid_sha256(&runner.dependency_closure_sha256)
        || !valid_sha256(&runner.source_sha256)
        || !valid_sha256(&runner.runtime_identity_sha256)
        || runner.runtime_identity_sha256 != external_runtime_identity_checksum(&runner.runtime)?
    {
        return invalid("external comparator runner provenance is incomplete or malformed");
    }
    match comparator {
        ExternalComparatorKind::ScipyRadau
            if runner.runner_id != "scipy-solve-ivp-radau"
                || !runner.observed_upstream_identity
                || runner.version != "1.17.0"
                || runner.source_repository != "https://github.com/scipy/scipy"
                || runner.source_revision != "8c75ae75176236f233824e9a0483c26a69e6dfec" =>
        {
            invalid("SciPy Radau runner is outside the pinned 1.17.0 source contract")
        }
        ExternalComparatorKind::ScipyRadau => {
            let ExternalRuntimeIdentity::ScipyPython { identity } = &runner.runtime else {
                return invalid("SciPy Radau runner lacks the exact Python runtime identity");
            };
            let expected = numerical_reference_v2_not_run_manifest()?.runtime;
            if identity != &expected {
                return invalid("SciPy Radau runtime differs from the pinned v2 runtime identity");
            }
            Ok(())
        }
        ExternalComparatorKind::SundialsCvode if runner.runner_id != "sundials-cvode" => {
            invalid("SUNDIALS comparator runner must identify CVODE explicitly")
        }
        ExternalComparatorKind::SundialsCvode => match &runner.runtime {
            ExternalRuntimeIdentity::SundialsHostProbe {
                cvode_available,
                executable_names_checked,
                pkg_config_modules_checked,
                header_paths_checked,
                library_names_checked,
                python_modules_checked,
                ida_only_version,
                probe_findings,
                probe_evidence_sha256,
                ..
            } if !executable_names_checked.is_empty()
                && !pkg_config_modules_checked.is_empty()
                && !header_paths_checked.is_empty()
                && !library_names_checked.is_empty()
                && !python_modules_checked.is_empty()
                && !probe_findings.is_empty()
                && valid_sha256(probe_evidence_sha256)
                && *probe_evidence_sha256
                    == sundials_probe_evidence_checksum(
                        *cvode_available,
                        executable_names_checked,
                        pkg_config_modules_checked,
                        header_paths_checked,
                        library_names_checked,
                        python_modules_checked,
                        ida_only_version,
                        probe_findings,
                    )? =>
            {
                validate_sundials_probe_coverage(
                    *cvode_available,
                    executable_names_checked,
                    pkg_config_modules_checked,
                    header_paths_checked,
                    library_names_checked,
                    python_modules_checked,
                    probe_findings,
                )?;
                if *cvode_available && !runner.observed_upstream_identity {
                    return invalid("available CVODE lacks an observed upstream identity");
                }
                if !*cvode_available
                    && (runner.observed_upstream_identity
                        || runner.version != "not-installed"
                        || runner.build_id != "not-installed"
                        || runner.source_revision != "not-observed"
                        || runner.source_sha256 != ZERO_SHA256)
                {
                    return invalid(
                        "unavailable CVODE must use explicit non-observed identity sentinels",
                    );
                }
                Ok(())
            }
            _ => invalid("SUNDIALS CVODE runner lacks a complete typed host probe"),
        },
    }
}

fn validate_problem(problem: &ExternalProblemBinding) -> FairResult<()> {
    if problem.case_id.trim().is_empty()
        || problem.problem_id.trim().is_empty()
        || problem.implementation_revision.len() != 40
        || !problem
            .implementation_revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || problem.dimension == 0
        || !problem.t_span.iter().all(|value| value.is_finite())
        || problem.t_span[1] <= problem.t_span[0]
        || !valid_sha256(&problem.problem_source_sha256)
        || problem.output_grid_id.trim().is_empty()
        || !valid_sha256(&problem.reference_checksum)
        || problem.requested_times.is_empty()
        || !problem
            .requested_times
            .iter()
            .all(|value| value.is_finite())
        || problem
            .requested_times
            .windows(2)
            .any(|pair| pair[1] <= pair[0])
        || problem.requested_times[0].to_bits() != problem.t_span[0].to_bits()
        || problem.requested_times.last().expect("nonempty").to_bits()
            != problem.t_span[1].to_bits()
    {
        return invalid("external comparator problem/grid binding is malformed");
    }
    Ok(())
}

fn validate_mass_treatment(
    has_mass_matrix: bool,
    treatment: &ExternalMassTreatment,
) -> FairResult<()> {
    match (has_mass_matrix, treatment) {
        (false, ExternalMassTreatment::Identity) => Ok(()),
        (
            true,
            ExternalMassTreatment::TransformedIdentity {
                transform_id,
                transform_source_sha256,
            },
        ) if !transform_id.trim().is_empty() && valid_sha256(transform_source_sha256) => Ok(()),
        (true, ExternalMassTreatment::NonApplicable) => Ok(()),
        (false, _) => invalid("identity-mass comparator contract declares a mass transformation"),
        (true, _) => invalid(
            "mass-matrix comparator requires a pinned transformed-identity declaration or non-applicable status",
        ),
    }
}

fn validate_evidence(
    evidence: &ExternalComparatorEvidence,
    contract: &ExternalComparatorContract,
) -> FairResult<()> {
    if evidence.schema_version != EXTERNAL_COMPARATOR_EVIDENCE_SCHEMA_VERSION {
        return invalid("unsupported external comparator evidence schema");
    }
    if evidence.comparator != contract.comparator
        || evidence.runner != contract.runner
        || !same_problem(&evidence.problem, &contract.problem)
        || !same_tolerance(&evidence.tolerance, &contract.tolerance)
        || evidence.dense_output != contract.dense_output
        || evidence.mass_treatment != contract.mass_treatment
    {
        return invalid("external comparator evidence differs from the frozen contract");
    }
    validate_runner(evidence.comparator, &evidence.runner)?;
    validate_problem(&evidence.problem)?;
    validate_mass_treatment(evidence.problem.has_mass_matrix, &evidence.mass_treatment)?;

    let dependency = &evidence.reference_dependency;
    let shares_lineage = dependency.reference_lineage_id == dependency.runner_lineage_id;
    if dependency.reference_lineage_id != contract.reference_lineage_id
        || dependency.runner_lineage_id != evidence.runner.implementation_lineage_id
        || dependency.shares_implementation_lineage != shares_lineage
    {
        return invalid("external comparator reference dependency is inconsistent");
    }

    let expected_grid_checksum =
        numerical_reference_grid_checksum(&evidence.problem.requested_times);
    if !valid_sha256(&evidence.checksums.grid_sha256)
        || evidence.checksums.grid_sha256 != expected_grid_checksum
    {
        return invalid("external comparator requested-grid checksum mismatch");
    }
    if matches!(
        &evidence.runner.runtime,
        ExternalRuntimeIdentity::SundialsHostProbe {
            cvode_available: false,
            ..
        }
    ) && !matches!(evidence.status, ExternalRunStatus::Unavailable { .. })
    {
        return invalid("a CVODE-unavailable host probe requires typed unavailable evidence");
    }
    if evidence.comparator == ExternalComparatorKind::ScipyRadau
        && (!dependency.shares_implementation_lineage
            || dependency.reference_lineage_id != evidence.runner.implementation_lineage_id)
    {
        return invalid(
            "SciPy Radau is a shared-lineage cross-check, not independent ranking evidence",
        );
    }

    match &evidence.status {
        ExternalRunStatus::Success => validate_success(evidence),
        ExternalRunStatus::Unavailable { reason } | ExternalRunStatus::NotRun { reason } => {
            validate_reason(reason)?;
            if evidence.mass_treatment == ExternalMassTreatment::NonApplicable {
                return invalid(
                    "non-applicable mass treatment requires a non-applicable run status",
                );
            }
            validate_absent_run_payload(evidence)
        }
        ExternalRunStatus::NonApplicable { reason } => {
            validate_reason(reason)?;
            if evidence.mass_treatment != ExternalMassTreatment::NonApplicable {
                return invalid(
                    "non-applicable external comparator must declare non-applicable mass treatment",
                );
            }
            validate_absent_run_payload(evidence)
        }
        ExternalRunStatus::SolverFailure { reason } => {
            validate_reason(reason)?;
            if evidence.mass_treatment == ExternalMassTreatment::NonApplicable {
                return invalid(
                    "non-applicable mass treatment requires a non-applicable run status",
                );
            }
            validate_failure_payload(evidence)
        }
    }
}

fn validate_success(evidence: &ExternalComparatorEvidence) -> FairResult<()> {
    if evidence.problem.has_mass_matrix
        && !matches!(
            evidence.mass_treatment,
            ExternalMassTreatment::TransformedIdentity { .. }
        )
    {
        return invalid(
            "successful mass-matrix comparator lacks a pinned transformed-identity declaration",
        );
    }
    let states = evidence
        .states
        .as_ref()
        .ok_or_else(|| FairError::Invalid("successful external comparator lacks states".into()))?;
    let committed_times = evidence.committed_times.as_ref().ok_or_else(|| {
        FairError::Invalid("successful external comparator lacks committed times".into())
    })?;
    if !same_f64_vectors(committed_times, &evidence.problem.requested_times) {
        return invalid("successful external comparator committed grid is not the requested grid");
    }
    if states.len() != evidence.problem.requested_times.len()
        || states.iter().any(|state| {
            state.len() != evidence.problem.dimension
                || !state.iter().all(|value| value.is_finite())
        })
    {
        return invalid("external comparator states have invalid shape or non-finite values");
    }
    let state_checksum = evidence.checksums.state_sha256.as_ref().ok_or_else(|| {
        FairError::Invalid("successful external comparator lacks a state checksum".into())
    })?;
    if !valid_sha256(state_checksum)
        || *state_checksum != numerical_reference_state_checksum(states)
    {
        return invalid("external comparator state checksum mismatch");
    }
    let committed_grid_checksum = evidence
        .checksums
        .committed_grid_sha256
        .as_ref()
        .ok_or_else(|| {
            FairError::Invalid(
                "successful external comparator lacks committed-grid checksum".into(),
            )
        })?;
    if !valid_sha256(committed_grid_checksum)
        || *committed_grid_checksum != numerical_reference_grid_checksum(committed_times)
    {
        return invalid("external comparator committed-grid checksum mismatch");
    }
    let work = evidence.native_work.as_ref().ok_or_else(|| {
        FairError::Invalid("successful external comparator lacks native work counters".into())
    })?;
    validate_work_kind(evidence.comparator, work)
}

fn validate_absent_run_payload(evidence: &ExternalComparatorEvidence) -> FairResult<()> {
    if evidence.committed_times.is_some()
        || evidence.states.is_some()
        || evidence.checksums.committed_grid_sha256.is_some()
        || evidence.checksums.state_sha256.is_some()
        || evidence.native_work.is_some()
    {
        return invalid("unexecuted external comparator contains fabricated run payload");
    }
    Ok(())
}

fn validate_failure_payload(evidence: &ExternalComparatorEvidence) -> FairResult<()> {
    let work = evidence.native_work.as_ref().ok_or_else(|| {
        FairError::Invalid("external solver failure lacks native work counters".into())
    })?;
    validate_work_kind(evidence.comparator, work)?;
    match (&evidence.committed_times, &evidence.states) {
        (None, None) => {
            if evidence.checksums.committed_grid_sha256.is_some()
                || evidence.checksums.state_sha256.is_some()
            {
                return invalid("external solver failure has checksums without a committed prefix");
            }
            Ok(())
        }
        (Some(times), Some(states)) => {
            if times.is_empty()
                || times.len() != states.len()
                || times.len() > evidence.problem.requested_times.len()
                || !same_f64_vectors(times, &evidence.problem.requested_times[..times.len()])
                || states.iter().any(|state| {
                    state.len() != evidence.problem.dimension
                        || !state.iter().all(|value| value.is_finite())
                })
            {
                return invalid(
                    "external solver failure does not preserve a finite requested-grid prefix",
                );
            }
            let grid = evidence
                .checksums
                .committed_grid_sha256
                .as_ref()
                .ok_or_else(|| FairError::Invalid("failure prefix lacks grid checksum".into()))?;
            let state =
                evidence.checksums.state_sha256.as_ref().ok_or_else(|| {
                    FairError::Invalid("failure prefix lacks state checksum".into())
                })?;
            if !valid_sha256(grid)
                || *grid != numerical_reference_grid_checksum(times)
                || !valid_sha256(state)
                || *state != numerical_reference_state_checksum(states)
            {
                return invalid("external solver failure prefix checksum mismatch");
            }
            Ok(())
        }
        _ => invalid("external solver failure prefix times/states are incomplete"),
    }
}

fn validate_work_kind(
    comparator: ExternalComparatorKind,
    work: &ExternalNativeWork,
) -> FairResult<()> {
    match (comparator, work) {
        (ExternalComparatorKind::ScipyRadau, ExternalNativeWork::ScipyRadau { .. })
        | (ExternalComparatorKind::SundialsCvode, ExternalNativeWork::SundialsCvode { .. }) => {
            Ok(())
        }
        _ => invalid("external comparator native work counters belong to another runner"),
    }
}

fn validate_reason(reason: &str) -> FairResult<()> {
    if reason.trim().is_empty() {
        invalid("external comparator non-success status requires a reason")
    } else {
        Ok(())
    }
}

fn same_problem(left: &ExternalProblemBinding, right: &ExternalProblemBinding) -> bool {
    left.case_id == right.case_id
        && left.problem_id == right.problem_id
        && left.implementation_revision == right.implementation_revision
        && left.dimension == right.dimension
        && left.t_span[0].to_bits() == right.t_span[0].to_bits()
        && left.t_span[1].to_bits() == right.t_span[1].to_bits()
        && left.problem_source_sha256 == right.problem_source_sha256
        && left.has_mass_matrix == right.has_mass_matrix
        && same_f64_vectors(&left.requested_times, &right.requested_times)
        && left.output_grid_id == right.output_grid_id
        && left.reference_checksum == right.reference_checksum
}

fn same_tolerance(left: &ExternalToleranceBinding, right: &ExternalToleranceBinding) -> bool {
    left.rtol.to_bits() == right.rtol.to_bits() && left.atol.to_bits() == right.atol.to_bits()
}

fn same_f64_vectors(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn invalid<T>(message: &str) -> FairResult<T> {
    Err(FairError::Invalid(message.into()))
}
