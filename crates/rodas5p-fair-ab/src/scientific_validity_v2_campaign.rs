//! Canonical ScientificCorpusV2.1 RODAS5P campaign execution.
//!
//! The runner deliberately keeps clipped and dense sampling as independent
//! integrations.  Every source discontinuity is executed as branch-fixed
//! segments, and each segment starts with a fresh controller and Krylov state.

use std::time::Instant;

use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters, sha256_hex,
};
use rodas5p_integrators::{
    AdaptiveRunDiagnostics, AdaptiveStepConfig, ControllerKind, CorpusPartition,
    OutputSamplingPlan, OutputSchedule, ScientificCaseSpec, ScientificCorpusV2, ScientificFamily,
    ScientificProblemCase, V2_THRESHOLD_DERIVATION_ID, V2CalibrationFreezeEnvelope,
    V2CalibrationFreezePayload, V2CampaignBinding, V2EvidenceAuthority, V2GateProfile, V2GateRow,
    V2GateRowStatus, V2OregonatorReplayEnvelope, V2OregonatorReplayPayload, V2OregonatorReplayRow,
    V2RowEvidenceBinding, integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_sequential_matrix_free_adaptive_observed, v2_calibration_payload_checksum,
    v2_oregonator_replay_payload_checksum, verify_v2_calibration_freeze,
    verify_v2_oregonator_replay,
};
use serde::{Deserialize, Serialize};

use crate::{
    FairError, FairResult, GlobalErrorMetrics, NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID,
    NumericalReferenceBundleV2, OutputPolicyDominance, ReferenceDominance,
    classify_output_policy_dominance, classify_reference_dominance,
    numerical_reference_grid_checksum, numerical_reference_state_checksum,
};

pub const SCIENTIFIC_VALIDITY_V2_CAMPAIGN_SCHEMA: &str = "scientific-validity-v2-campaign-case-v1";
pub const SCIENTIFIC_VALIDITY_V2_RUNNER_SCHEMA: &str = "scientific-validity-v2-campaign-runner-v1";
pub const SCIENTIFIC_VALIDITY_V2_CANDIDATE_ID: &str = "sequential-rodas5p-gmres-wrms-forcing-v2";
pub const SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM: usize = 200_000;

const OUTPUT_PROTOCOL_ID: &str =
    "branch-fixed-controller-krylov-restart; clipped-and-rodas5p-dense-independent-v1";
const SOLVER_PROTOCOL_ID: &str = "rodas5p;gmres-restart32-max256;fallback-atol=1e-12;fallback-rtol=1e-10;inner-m=30;outer-k=8;recycle-dim=8;recycle-rank-tol=1e-12;pc-none;x0-previous;wrms-stage-residual-heuristic-v2;endpoint-bound=requires-resolvent-certificate;cross-step-recycle-images=refresh-per-linearization;outer=case-spec;initial=span/100;min=1e-12;max=span;controller=integral;safety=.9;factors=.2,5;reject=.9;total-attempts=200000";
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum V2CampaignOutputMode {
    Clipped,
    Dense,
}

impl V2CampaignOutputMode {
    fn tag(self) -> u8 {
        match self {
            Self::Clipped => 0,
            Self::Dense => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum V2CampaignArmStatus {
    Success,
    Failure,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V2Rodas5pCampaignConfig {
    pub method: String,
    pub linear_method: String,
    pub inner_tolerance_policy: String,
    pub inner_solve_claim_scope: String,
    pub cross_step_recycle_image_policy: String,
    pub restart: usize,
    pub max_arnoldi: usize,
    pub inner_m: usize,
    pub outer_k: usize,
    pub recycle_dim: usize,
    pub recycle_rank_tolerance: f64,
    pub preconditioner: String,
    pub initial_guess: String,
    pub fallback_inner_atol: f64,
    pub fallback_inner_rtol: f64,
    pub outer_atol: f64,
    pub outer_rtol: f64,
    pub initial_step: f64,
    pub min_step: f64,
    pub max_step: f64,
    pub max_attempts_per_arm: usize,
    pub controller: String,
    pub controller_safety: f64,
    pub controller_min_factor: f64,
    pub controller_max_factor: f64,
    pub controller_reject_max_factor: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V2CampaignReferenceBinding {
    pub case_id: String,
    pub problem_id: String,
    pub reference_checksum_sha256: String,
    pub implementation_revision: String,
    pub wrms_formula_id: String,
    pub anchor_state_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V2CampaignArmEvidence {
    pub mode: V2CampaignOutputMode,
    pub status: V2CampaignArmStatus,
    pub output_times: Vec<f64>,
    pub states: Vec<Vec<f64>>,
    pub committed_output_count: usize,
    pub counters: WorkCounters,
    pub diagnostics: AdaptiveRunDiagnostics,
    pub internal_steps: usize,
    pub output_clipped_steps: usize,
    pub completed_segments: usize,
    pub boundary_restarts: usize,
    pub message: String,
    pub metrics: Option<GlobalErrorMetrics>,
    pub wall_seconds: f64,
    pub output_checksum_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScientificValidityV2CaseArtifact {
    pub schema_version: String,
    pub corpus_version: String,
    pub candidate_id: String,
    pub code_revision: String,
    pub spec: ScientificCaseSpec,
    pub config: V2Rodas5pCampaignConfig,
    pub reference: V2CampaignReferenceBinding,
    pub reference_uncertainty_wrms: f64,
    pub clipped: V2CampaignArmEvidence,
    pub dense: V2CampaignArmEvidence,
    pub output_policy_discrepancy_wrms: Option<f64>,
    pub row: V2GateRow,
    pub artifact_checksum_sha256: String,
}

pub fn scientific_validity_v2_compiled_revision() -> FairResult<&'static str> {
    let revision = option_env!("VIGILODE_CODE_REVISION").ok_or_else(|| {
        FairError::Invalid("canonical v2 runner was built without VIGILODE_CODE_REVISION".into())
    })?;
    if revision.len() != 40
        || !revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(FairError::Invalid(
            "VIGILODE_CODE_REVISION is not a lowercase 40-hex revision".into(),
        ));
    }
    if revision != env!("VIGILODE_DETECTED_GIT_REVISION") {
        return Err(FairError::Invalid(
            "VIGILODE_CODE_REVISION differs from git rev-parse HEAD at build time".into(),
        ));
    }
    if env!("VIGILODE_SOURCE_DIRTY_AT_BUILD") != "false" {
        return Err(FairError::Invalid(
            "canonical v2 runner refuses a dirty source overlay".into(),
        ));
    }
    Ok(revision)
}

pub fn scientific_validity_v2_detected_revision() -> &'static str {
    env!("VIGILODE_DETECTED_GIT_REVISION")
}

pub fn scientific_validity_v2_source_dirty_at_build() -> bool {
    env!("VIGILODE_SOURCE_DIRTY_AT_BUILD") != "false"
}

fn campaign_config(spec: &ScientificCaseSpec) -> V2Rodas5pCampaignConfig {
    let span = spec.t_span.1 - spec.t_span.0;
    V2Rodas5pCampaignConfig {
        method: "RODAS5P".into(),
        linear_method: "GMRES".into(),
        inner_tolerance_policy: "wrms-stage-residual-heuristic-v2".into(),
        inner_solve_claim_scope:
            "residual-only; endpoint contamination requires an independent W-inverse resolvent certificate"
                .into(),
        cross_step_recycle_image_policy:
            "refresh-per-linearization; approximate cross-step image reuse is not admitted".into(),
        restart: 32,
        max_arnoldi: 256,
        inner_m: 30,
        outer_k: 8,
        recycle_dim: 8,
        recycle_rank_tolerance: 1.0e-12,
        preconditioner: "none".into(),
        initial_guess: "previous".into(),
        fallback_inner_atol: 1.0e-12,
        fallback_inner_rtol: 1.0e-10,
        outer_atol: spec.atol,
        outer_rtol: spec.rtol,
        initial_step: span / 100.0,
        min_step: 1.0e-12,
        max_step: span,
        max_attempts_per_arm: SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM,
        controller: "integral".into(),
        controller_safety: 0.9,
        controller_min_factor: 0.2,
        controller_max_factor: 5.0,
        controller_reject_max_factor: 0.9,
    }
}

fn linear_config(config: &V2Rodas5pCampaignConfig) -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        // Every stage replaces these fixed thresholds with the WRMS forcing
        // target.  The frozen nonzero values remain the explicit fallback.
        rtol: config.fallback_inner_rtol,
        atol: config.fallback_inner_atol,
        restart: config.restart,
        maxiter: config.max_arnoldi,
        inner_m: config.inner_m,
        outer_k: config.outer_k,
        recycle_dim: config.recycle_dim,
        recycle_rank_tol: config.recycle_rank_tolerance,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
    }
}

fn adaptive_config(config: &V2Rodas5pCampaignConfig, max_attempts: usize) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: config.outer_atol,
        rtol: config.outer_rtol,
        initial_step: config.initial_step,
        min_step: config.min_step,
        max_step: config.max_step,
        max_attempts,
        safety: config.controller_safety,
        min_factor: config.controller_min_factor,
        max_factor: config.controller_max_factor,
        reject_max_factor: config.controller_reject_max_factor,
        controller: ControllerKind::Integral,
    }
}

fn campaign_binding(code_revision: &str, authority: V2EvidenceAuthority) -> V2CampaignBinding {
    let solver_config_sha256 = sha256_hex(
        &[
            b"vigilode-scientific-v2-solver-protocol-v1\0".as_slice(),
            SOLVER_PROTOCOL_ID.as_bytes(),
        ]
        .concat(),
    );
    let wrms_scale_sha256 = sha256_hex(
        b"vigilode-scientific-v2-wrms-policy-v1\0wrms-tight-radau-l2-anchor-v1; absolute=1e-10; relative=1e-8",
    );
    let output_policy_protocol_sha256 = sha256_hex(
        &[
            b"vigilode-scientific-v2-output-protocol-v1\0".as_slice(),
            OUTPUT_PROTOCOL_ID.as_bytes(),
        ]
        .concat(),
    );
    V2CampaignBinding {
        authority,
        runner_schema: SCIENTIFIC_VALIDITY_V2_RUNNER_SCHEMA.into(),
        candidate_id: SCIENTIFIC_VALIDITY_V2_CANDIDATE_ID.into(),
        code_revision: code_revision.into(),
        solver_config_sha256,
        wrms_scale_sha256,
        output_policy_protocol_sha256,
    }
}

pub fn scientific_validity_v2_canonical_campaign_binding() -> FairResult<V2CampaignBinding> {
    let revision = scientific_validity_v2_compiled_revision()?;
    Ok(campaign_binding(
        revision,
        V2EvidenceAuthority::CanonicalV2Runner,
    ))
}

fn same_state_bits(left: &[f64], right: &[f64]) -> bool {
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

fn append_segment_outputs(
    aggregate_times: &mut Vec<f64>,
    aggregate_states: &mut Vec<Vec<f64>>,
    times: &[f64],
    states: &[Vec<f64>],
) -> FairResult<()> {
    if times.len() != states.len() || times.is_empty() {
        return Err(FairError::Invalid(
            "campaign segment returned an invalid committed prefix".into(),
        ));
    }
    let mut first = 0;
    if let (Some(last_time), Some(last_state)) = (aggregate_times.last(), aggregate_states.last()) {
        if last_time.to_bits() != times[0].to_bits() || !same_state_bits(last_state, &states[0]) {
            return Err(FairError::Invalid(
                "branch boundary time/state is not bit-identical across restarted segments".into(),
            ));
        }
        first = 1;
    }
    aggregate_times.extend_from_slice(&times[first..]);
    aggregate_states.extend_from_slice(&states[first..]);
    Ok(())
}

fn selected_segment_times(spec: &ScientificCaseSpec, span: (f64, f64)) -> FairResult<Vec<f64>> {
    let times = spec
        .output_times
        .iter()
        .copied()
        .filter(|time| *time >= span.0 && *time <= span.1)
        .collect::<Vec<_>>();
    if times.first().map(|time| time.to_bits()) != Some(span.0.to_bits())
        || times.last().map(|time| time.to_bits()) != Some(span.1.to_bits())
    {
        return Err(FairError::Invalid(
            "branch-fixed segment lacks exact endpoint outputs".into(),
        ));
    }
    Ok(times)
}

fn stored_state_bytes(states: &[Vec<f64>]) -> FairResult<u64> {
    states.iter().try_fold(0_u64, |total, state| {
        let row = u64::try_from(state.len())
            .ok()
            .and_then(|count| count.checked_mul(8))
            .ok_or_else(|| FairError::Invalid("stored-state byte count overflow".into()))?;
        total
            .checked_add(row)
            .ok_or_else(|| FairError::Invalid("stored-state byte count overflow".into()))
    })
}

fn output_checksum(
    mode: V2CampaignOutputMode,
    status: V2CampaignArmStatus,
    times: &[f64],
    states: &[Vec<f64>],
) -> String {
    let mut bytes = b"vigilode-scientific-v2-mode-output-v1\0".to_vec();
    bytes.push(mode.tag());
    bytes.push(match status {
        V2CampaignArmStatus::Success => 0,
        V2CampaignArmStatus::Failure => 1,
    });
    bytes.extend_from_slice(&(times.len() as u64).to_le_bytes());
    for (time, state) in times.iter().zip(states) {
        bytes.extend_from_slice(&time.to_bits().to_le_bytes());
        bytes.extend_from_slice(&(state.len() as u64).to_le_bytes());
        for value in state {
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
    }
    sha256_hex(&bytes)
}

fn arm_failure_from_error(
    case: &ScientificProblemCase,
    mode: V2CampaignOutputMode,
    error: &FairError,
    wall_seconds: f64,
) -> V2CampaignArmEvidence {
    let output_times = vec![case.spec.t_span.0];
    let states = vec![case.y0.clone()];
    V2CampaignArmEvidence {
        mode,
        status: V2CampaignArmStatus::Failure,
        committed_output_count: 1,
        output_checksum_sha256: output_checksum(
            mode,
            V2CampaignArmStatus::Failure,
            &output_times,
            &states,
        ),
        output_times,
        states,
        counters: WorkCounters::default(),
        diagnostics: AdaptiveRunDiagnostics::default(),
        internal_steps: 0,
        output_clipped_steps: 0,
        completed_segments: 0,
        boundary_restarts: 0,
        message: format!("arm returned before a preservable integrator result: {error}"),
        metrics: None,
        wall_seconds,
    }
}

fn execute_arm_pair_with<F>(
    case: &ScientificProblemCase,
    mut run: F,
) -> (V2CampaignArmEvidence, V2CampaignArmEvidence)
where
    F: FnMut(V2CampaignOutputMode) -> FairResult<V2CampaignArmEvidence>,
{
    let mut one = |mode| {
        let started = Instant::now();
        match run(mode) {
            Ok(arm) => arm,
            Err(error) => {
                arm_failure_from_error(case, mode, &error, started.elapsed().as_secs_f64())
            }
        }
    };
    let clipped = one(V2CampaignOutputMode::Clipped);
    let dense = one(V2CampaignOutputMode::Dense);
    (clipped, dense)
}

fn execute_arm(
    case: &ScientificProblemCase,
    config: &V2Rodas5pCampaignConfig,
    mode: V2CampaignOutputMode,
) -> FairResult<V2CampaignArmEvidence> {
    let started = Instant::now();
    let linear = linear_config(config);
    // Even a pre-stage solver failure retains the supplied initial condition
    // as an authenticated committed prefix.
    let mut aggregate_times = vec![case.spec.t_span.0];
    let mut aggregate_states = vec![case.y0.clone()];
    let mut counters = WorkCounters::default();
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut internal_steps = 0_usize;
    let mut output_clipped_steps = 0_usize;
    let mut completed_segments = 0_usize;
    let mut segments_started = 0_usize;
    let mut state = case.y0.clone();
    let mut status = V2CampaignArmStatus::Success;
    let mut message = "success".to_owned();

    for (segment_index, segment) in case.integration_segments.iter().enumerate() {
        let remaining = SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM
            .checked_sub(diagnostics.attempts)
            .ok_or_else(|| FairError::Invalid("campaign attempt ledger overflow".into()))?;
        if remaining == 0 {
            status = V2CampaignArmStatus::Failure;
            message = "maximum total attempts exhausted before next branch segment".into();
            break;
        }
        let output_times = selected_segment_times(&case.spec, segment.t_span)?;
        let schedule = OutputSchedule::new(output_times)?;
        let adaptive = adaptive_config(config, remaining);
        let matrix_free_problem = segment.problem.jvp_only_clone()?;
        segments_started = segments_started
            .checked_add(1)
            .ok_or_else(|| FairError::Invalid("campaign segment ledger overflow".into()))?;
        let result = match mode {
            V2CampaignOutputMode::Clipped => integrate_sequential_matrix_free_adaptive_observed(
                &matrix_free_problem,
                segment.t_span,
                &state,
                &linear,
                &adaptive,
                &schedule,
            )
            .map_err(|error| error.to_string()),
            V2CampaignOutputMode::Dense => {
                let sampling = OutputSamplingPlan::dense(schedule);
                integrate_sequential_matrix_free_adaptive_dense_observed(
                    &matrix_free_problem,
                    segment.t_span,
                    &state,
                    &linear,
                    &adaptive,
                    &sampling,
                )
                .map_err(|error| error.to_string())
            }
        };
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                status = V2CampaignArmStatus::Failure;
                message = format!("segment {segment_index} returned error: {error}");
                break;
            }
        };
        counters
            .checked_accumulate(result.observed.counters)
            .ok_or_else(|| FairError::Invalid("campaign WorkCounters overflow".into()))?;
        diagnostics.checked_accumulate(&result.diagnostics)?;
        internal_steps = internal_steps
            .checked_add(result.observed.internal_steps)
            .ok_or_else(|| FairError::Invalid("campaign internal-step overflow".into()))?;
        output_clipped_steps = output_clipped_steps
            .checked_add(result.observed.output_clipped_steps)
            .ok_or_else(|| FairError::Invalid("campaign clipped-step overflow".into()))?;
        append_segment_outputs(
            &mut aggregate_times,
            &mut aggregate_states,
            &result.observed.t,
            &result.observed.y,
        )?;
        if !result.observed.success {
            status = V2CampaignArmStatus::Failure;
            message = format!("segment {segment_index}: {}", result.observed.message);
            break;
        }
        let last = result.observed.y.last().ok_or_else(|| {
            FairError::Invalid("successful campaign segment returned no endpoint".into())
        })?;
        state = last.clone();
        completed_segments += 1;
    }

    if status == V2CampaignArmStatus::Success
        && (aggregate_times.len() != case.spec.output_times.len()
            || aggregate_times
                .iter()
                .zip(&case.spec.output_times)
                .any(|(actual, expected)| actual.to_bits() != expected.to_bits()))
    {
        return Err(FairError::Invalid(
            "successful campaign arm does not cover the exact requested grid".into(),
        ));
    }
    if diagnostics.attempts > SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM {
        return Err(FairError::Invalid(
            "campaign arm exceeded its total attempt budget".into(),
        ));
    }
    let checksum = output_checksum(mode, status, &aggregate_times, &aggregate_states);
    Ok(V2CampaignArmEvidence {
        mode,
        status,
        committed_output_count: aggregate_times.len(),
        output_times: aggregate_times,
        states: aggregate_states,
        counters,
        diagnostics,
        internal_steps,
        output_clipped_steps,
        completed_segments,
        boundary_restarts: segments_started.saturating_sub(1),
        message,
        metrics: None,
        wall_seconds: started.elapsed().as_secs_f64(),
        output_checksum_sha256: checksum,
    })
}

fn artifact_checksum(artifact: &ScientificValidityV2CaseArtifact) -> FairResult<String> {
    let mut payload = artifact.clone();
    payload.artifact_checksum_sha256 = ZERO_SHA256.into();
    // Operational timing is preserved in the artifact but deliberately does
    // not affect scientific byte identity.
    payload.clipped.wall_seconds = 0.0;
    payload.dense.wall_seconds = 0.0;
    payload.row.wall_seconds = None;
    let mut bytes = b"vigilode-scientific-v2-case-artifact-v1\0".to_vec();
    bytes.extend_from_slice(&serde_json::to_vec(&payload)?);
    Ok(sha256_hex(&bytes))
}

fn preserve_failed_pair(
    clipped: &mut V2CampaignArmEvidence,
    dense: &mut V2CampaignArmEvidence,
) -> Option<(V2GateRowStatus, Option<f64>, Option<f64>, String)> {
    if clipped.status == V2CampaignArmStatus::Success
        && dense.status == V2CampaignArmStatus::Success
    {
        return None;
    }
    // A failed arm never inherits stale measurements from a prior or partial
    // execution.  Its committed prefix and work remain in the raw evidence.
    clipped.metrics = None;
    dense.metrics = None;
    Some((
        V2GateRowStatus::Fail,
        None,
        None,
        format!(
            "failure-preserved: clipped={:?}({}); dense={:?}({})",
            clipped.status, clipped.message, dense.status, dense.message
        ),
    ))
}

pub fn run_scientific_validity_v2_case(
    spec: &ScientificCaseSpec,
    reference: &NumericalReferenceBundleV2,
) -> FairResult<ScientificValidityV2CaseArtifact> {
    let code_revision = scientific_validity_v2_compiled_revision()?;
    run_scientific_validity_v2_case_with_authority(
        spec,
        reference,
        code_revision,
        V2EvidenceAuthority::CanonicalV2Runner,
    )
}

/// Dirty-tree execution path used only to exercise wiring in CI/development.
/// Its evidence is permanently typed synthetic and is ineligible for a
/// canonical calibration freeze.
pub fn run_scientific_validity_v2_case_synthetic_smoke(
    spec: &ScientificCaseSpec,
    reference: &NumericalReferenceBundleV2,
) -> FairResult<ScientificValidityV2CaseArtifact> {
    run_scientific_validity_v2_case_with_authority(
        spec,
        reference,
        scientific_validity_v2_detected_revision(),
        V2EvidenceAuthority::SyntheticCiSmoke,
    )
}

fn run_scientific_validity_v2_case_with_authority(
    spec: &ScientificCaseSpec,
    reference: &NumericalReferenceBundleV2,
    code_revision: &str,
    authority: V2EvidenceAuthority,
) -> FairResult<ScientificValidityV2CaseArtifact> {
    if reference.case_id != spec.id || reference.implementation_revision != code_revision {
        return Err(FairError::Invalid(
            "reference case/revision does not match the compiled canonical runner".into(),
        ));
    }
    reference.wrms_basis.validate()?;
    if reference.wrms_basis.output_grid.times != spec.output_times
        || reference.trajectory.output_grid != reference.wrms_basis.output_grid
        || reference.trajectory.states != reference.wrms_basis.reference_states
        || reference.error_scale != reference.wrms_basis.error_scale
        || reference.trajectory.provenance.problem_id != reference.problem_id
        || reference.trajectory.provenance.output_grid_id
            != reference.trajectory.output_grid.grid_id
        || reference.trajectory.provenance.state_checksum
            != numerical_reference_state_checksum(&reference.trajectory.states)
        || numerical_reference_grid_checksum(&reference.trajectory.output_grid.times)
            != reference
                .trajectory
                .provenance
                .numerical
                .as_ref()
                .map(|numerical| numerical.checksums.grid_sha256.as_str())
                .unwrap_or_default()
        || !valid_sha256(&reference.reference_checksum_sha256)
    {
        return Err(FairError::Invalid(
            "reference trajectory/basis/checksum does not exactly match the scientific case".into(),
        ));
    }
    let numerical = reference
        .trajectory
        .provenance
        .numerical
        .as_ref()
        .ok_or_else(|| {
            FairError::Invalid("v2 campaign reference lacks numerical provenance".into())
        })?;
    let wrms_formula_id = numerical
        .wrms_formula_id
        .clone()
        .ok_or_else(|| FairError::Invalid("v2 campaign reference lacks WRMS formula id".into()))?;
    let anchor_state_sha256 = numerical
        .anchor_state_sha256
        .clone()
        .ok_or_else(|| FairError::Invalid("v2 campaign reference lacks anchor checksum".into()))?;
    if numerical.corpus_version.as_deref() != Some(ScientificCorpusV2::VERSION)
        || numerical.case_id.as_deref() != Some(spec.id.as_str())
        || numerical.reference_checksum_sha256.as_deref()
            != Some(reference.reference_checksum_sha256.as_str())
        || wrms_formula_id != NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID
        || anchor_state_sha256 != reference.trajectory.provenance.state_checksum
        || numerical.checksums.state_sha256 != reference.trajectory.provenance.state_checksum
        || numerical.convergence.reference_uncertainty_wrms.to_bits()
            != reference.error_scale.reference_uncertainty_wrms.to_bits()
        || reference
            .trajectory
            .provenance
            .reference_uncertainty_wrms
            .to_bits()
            != reference.error_scale.reference_uncertainty_wrms.to_bits()
    {
        return Err(FairError::Invalid(
            "v2 campaign numerical-reference provenance is internally inconsistent".into(),
        ));
    }
    let reference_binding = V2CampaignReferenceBinding {
        case_id: reference.case_id.clone(),
        problem_id: reference.problem_id.clone(),
        reference_checksum_sha256: reference.reference_checksum_sha256.clone(),
        implementation_revision: reference.implementation_revision.clone(),
        wrms_formula_id,
        anchor_state_sha256,
    };
    let config = campaign_config(spec);
    let case = spec.build()?;
    let (mut clipped, mut dense) =
        execute_arm_pair_with(&case, |mode| execute_arm(&case, &config, mode));
    let campaign = campaign_binding(code_revision, authority);
    let (status, conservative_max_wrms, output_policy_discrepancy_wrms, evidence) = if let Some(
        failure,
    ) =
        preserve_failed_pair(&mut clipped, &mut dense)
    {
        failure
    } else {
        let clipped_metrics = reference
            .wrms_basis
            .metrics(&clipped.output_times, &clipped.states)?;
        let dense_metrics = reference
            .wrms_basis
            .metrics(&dense.output_times, &dense.states)?;
        clipped.metrics = Some(clipped_metrics.clone());
        dense.metrics = Some(dense_metrics.clone());

        // Scientific admission order is fixed: reference uncertainty first,
        // then the clipped/dense output-policy discrepancy.
        let reference_decision = classify_reference_dominance(
            reference.error_scale.reference_uncertainty_wrms,
            dense_metrics.max_grid_wrms,
        )?;
        let gap = reference.wrms_basis.discrepancy_wrms(
            &clipped.output_times,
            &clipped.states,
            &dense.output_times,
            &dense.states,
        )?;
        let row_status = match reference_decision {
            ReferenceDominance::Dominated => V2GateRowStatus::ReferenceDominated,
            ReferenceDominance::Admissible => {
                match classify_output_policy_dominance(gap, dense_metrics.max_grid_wrms)? {
                    OutputPolicyDominance::Dominated => V2GateRowStatus::OutputPolicyDominated,
                    OutputPolicyDominance::Admissible => V2GateRowStatus::Pass,
                }
            }
        };
        (
            row_status,
            Some(dense_metrics.conservative_max_wrms),
            Some(gap),
            format!(
                "reference={reference_decision:?}; clipped_dense_gap_wrms={gap:.17e}; forcing_solves=({}, {})",
                clipped.counters.forced_stage_solves, dense.counters.forced_stage_solves
            ),
        )
    };

    let row = V2GateRow {
        case_id: spec.id.clone(),
        family: spec.family,
        partition: spec.partition,
        dimension: spec.dimension,
        atol: spec.atol,
        rtol: spec.rtol,
        status,
        conservative_max_wrms,
        binding: V2RowEvidenceBinding {
            campaign,
            reference_checksum_sha256: reference.reference_checksum_sha256.clone(),
            clipped_output_checksum_sha256: clipped.output_checksum_sha256.clone(),
            dense_output_checksum_sha256: dense.output_checksum_sha256.clone(),
        },
        evidence,
        wall_seconds: Some(clipped.wall_seconds + dense.wall_seconds),
    };
    let mut artifact = ScientificValidityV2CaseArtifact {
        schema_version: SCIENTIFIC_VALIDITY_V2_CAMPAIGN_SCHEMA.into(),
        corpus_version: ScientificCorpusV2::VERSION.into(),
        candidate_id: SCIENTIFIC_VALIDITY_V2_CANDIDATE_ID.into(),
        code_revision: code_revision.into(),
        spec: spec.clone(),
        config,
        reference: reference_binding,
        reference_uncertainty_wrms: reference.error_scale.reference_uncertainty_wrms,
        clipped,
        dense,
        output_policy_discrepancy_wrms,
        row,
        artifact_checksum_sha256: ZERO_SHA256.into(),
    };
    artifact.artifact_checksum_sha256 = artifact_checksum(&artifact)?;
    validate_scientific_validity_v2_case_artifact(&artifact)?;
    Ok(artifact)
}

pub fn validate_scientific_validity_v2_case_artifact(
    artifact: &ScientificValidityV2CaseArtifact,
) -> FairResult<()> {
    let authority = artifact.row.binding.campaign.authority;
    let code_revision = match authority {
        V2EvidenceAuthority::CanonicalV2Runner => scientific_validity_v2_compiled_revision()?,
        V2EvidenceAuthority::SyntheticCiSmoke => scientific_validity_v2_detected_revision(),
    };
    let expected_spec = ScientificCorpusV2::all_specs()
        .into_iter()
        .find(|spec| spec.id == artifact.spec.id)
        .ok_or_else(|| FairError::Invalid("campaign artifact case is outside v2.1".into()))?;
    if artifact.schema_version != SCIENTIFIC_VALIDITY_V2_CAMPAIGN_SCHEMA
        || artifact.corpus_version != ScientificCorpusV2::VERSION
        || artifact.candidate_id != SCIENTIFIC_VALIDITY_V2_CANDIDATE_ID
        || artifact.code_revision != code_revision
        || artifact.reference.implementation_revision != code_revision
        || artifact.spec != expected_spec
        || artifact.config != campaign_config(&artifact.spec)
        || artifact.reference.case_id != artifact.spec.id
        || artifact.row.case_id != artifact.spec.id
        || artifact.row.family != artifact.spec.family
        || artifact.row.partition != artifact.spec.partition
        || artifact.row.dimension != artifact.spec.dimension
        || artifact.row.atol.to_bits() != artifact.spec.atol.to_bits()
        || artifact.row.rtol.to_bits() != artifact.spec.rtol.to_bits()
        || artifact.row.binding.campaign != campaign_binding(code_revision, authority)
        || artifact.row.binding.reference_checksum_sha256
            != artifact.reference.reference_checksum_sha256
        || artifact.row.binding.clipped_output_checksum_sha256
            != artifact.clipped.output_checksum_sha256
        || artifact.row.binding.dense_output_checksum_sha256
            != artifact.dense.output_checksum_sha256
    {
        return Err(FairError::Invalid(
            "campaign artifact identity/binding mismatch".into(),
        ));
    }
    let expected_case = artifact.spec.build()?;
    for (expected_mode, arm) in [
        (V2CampaignOutputMode::Clipped, &artifact.clipped),
        (V2CampaignOutputMode::Dense, &artifact.dense),
    ] {
        if arm.mode != expected_mode
            || arm.committed_output_count != arm.output_times.len()
            || arm.output_times.len() != arm.states.len()
            || arm.diagnostics.attempts > SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM
            || arm.output_checksum_sha256
                != output_checksum(arm.mode, arm.status, &arm.output_times, &arm.states)
            || arm.wall_seconds < 0.0
            || !arm.wall_seconds.is_finite()
            || !arm.diagnostics.is_structurally_consistent()
            || arm.counters.accepted_steps != arm.diagnostics.accepted_macro_steps as u64
            || arm.counters.rejected_steps != arm.diagnostics.rejected_macro_steps as u64
            || arm.internal_steps != arm.diagnostics.accepted_macro_steps
            || arm.counters.local_error_failures != arm.diagnostics.local_error_failures as u64
            || arm.counters.linear_solve_failures != arm.diagnostics.linear_solve_failures as u64
            || arm.counters.nonlinear_solve_failures
                != arm.diagnostics.nonlinear_solve_failures as u64
            || arm.counters.nonfinite_step_failures != arm.diagnostics.non_finite_failures as u64
            || arm.counters.fallback_steps != arm.diagnostics.fallback_steps as u64
            || (arm.status == V2CampaignArmStatus::Success && arm.counters.forced_stage_solves == 0)
        {
            return Err(FairError::Invalid(
                "campaign arm evidence/checksum/forcing mismatch".into(),
            ));
        }
        if arm.output_times.is_empty()
            || arm.output_times[0].to_bits() != artifact.spec.t_span.0.to_bits()
            || !same_state_bits(&arm.states[0], &expected_case.y0)
            || arm.output_times.windows(2).any(|pair| pair[1] <= pair[0])
            || arm.states.iter().any(|state| {
                state.len() != artifact.spec.dimension
                    || !state.iter().all(|value| value.is_finite())
            })
        {
            return Err(FairError::Invalid(
                "campaign arm does not preserve a finite initial-state prefix".into(),
            ));
        }
        if arm.status == V2CampaignArmStatus::Success
            && (arm.output_times.len() != artifact.spec.output_times.len()
                || arm
                    .output_times
                    .iter()
                    .zip(&artifact.spec.output_times)
                    .any(|(actual, expected)| actual.to_bits() != expected.to_bits())
                || arm.completed_segments != artifact.spec.mandatory_breakpoints.len() + 1
                || arm.boundary_restarts != artifact.spec.mandatory_breakpoints.len())
        {
            return Err(FairError::Invalid(
                "successful campaign arm lacks exact full-grid/segment coverage".into(),
            ));
        }
        if arm.status == V2CampaignArmStatus::Failure
            && (arm.output_times.len() > artifact.spec.output_times.len()
                || arm
                    .output_times
                    .iter()
                    .zip(&artifact.spec.output_times)
                    .any(|(actual, expected)| actual.to_bits() != expected.to_bits()))
        {
            return Err(FairError::Invalid(
                "failed campaign arm does not preserve an exact requested-grid prefix".into(),
            ));
        }
        let _ = stored_state_bytes(&arm.states)?;
    }
    let both_success = artifact.clipped.status == V2CampaignArmStatus::Success
        && artifact.dense.status == V2CampaignArmStatus::Success;
    if both_success {
        let _clipped = artifact.clipped.metrics.as_ref().ok_or_else(|| {
            FairError::Invalid("successful clipped campaign arm lacks metrics".into())
        })?;
        let dense = artifact.dense.metrics.as_ref().ok_or_else(|| {
            FairError::Invalid("successful dense campaign arm lacks metrics".into())
        })?;
        let gap = artifact.output_policy_discrepancy_wrms.ok_or_else(|| {
            FairError::Invalid("successful paired campaign lacks output-policy gap".into())
        })?;
        let reference =
            classify_reference_dominance(artifact.reference_uncertainty_wrms, dense.max_grid_wrms)?;
        let expected_status = match reference {
            ReferenceDominance::Dominated => V2GateRowStatus::ReferenceDominated,
            ReferenceDominance::Admissible => {
                match classify_output_policy_dominance(gap, dense.max_grid_wrms)? {
                    OutputPolicyDominance::Dominated => V2GateRowStatus::OutputPolicyDominated,
                    OutputPolicyDominance::Admissible => V2GateRowStatus::Pass,
                }
            }
        };
        let expected_metric = dense.conservative_max_wrms;
        if artifact.row.status != expected_status
            || artifact.row.conservative_max_wrms.map(f64::to_bits)
                != Some(expected_metric.to_bits())
        {
            return Err(FairError::Invalid(
                "campaign row dominance/metric does not match paired evidence".into(),
            ));
        }
    } else if artifact.row.status != V2GateRowStatus::Fail
        || artifact.row.conservative_max_wrms.is_some()
        || artifact.output_policy_discrepancy_wrms.is_some()
        || artifact.clipped.metrics.is_some()
        || artifact.dense.metrics.is_some()
    {
        return Err(FairError::Invalid(
            "failed campaign pair is not represented by a failure row".into(),
        ));
    }
    if artifact.artifact_checksum_sha256 != artifact_checksum(artifact)? {
        return Err(FairError::Invalid(
            "campaign case artifact checksum mismatch".into(),
        ));
    }
    Ok(())
}

fn validated_artifact_rows(
    artifacts: &[ScientificValidityV2CaseArtifact],
    expected_specs: Vec<ScientificCaseSpec>,
) -> FairResult<Vec<V2GateRow>> {
    if artifacts.len() != expected_specs.len() {
        return Err(FairError::Invalid(format!(
            "source-bound campaign artifact cardinality mismatch: expected {}, got {}",
            expected_specs.len(),
            artifacts.len()
        )));
    }
    let expected_ids = expected_specs
        .into_iter()
        .map(|spec| spec.id)
        .collect::<std::collections::BTreeSet<_>>();
    let mut seen = std::collections::BTreeSet::new();
    let mut rows = Vec::with_capacity(artifacts.len());
    for artifact in artifacts {
        validate_scientific_validity_v2_case_artifact(artifact)?;
        if artifact.row.binding.campaign.authority != V2EvidenceAuthority::CanonicalV2Runner {
            return Err(FairError::Invalid(
                "canonical gate input contains a noncanonical campaign artifact".into(),
            ));
        }
        if !seen.insert(artifact.spec.id.clone()) {
            return Err(FairError::Invalid(
                "canonical gate input contains duplicate campaign artifacts".into(),
            ));
        }
        rows.push(artifact.row.clone());
    }
    if seen != expected_ids {
        return Err(FairError::Invalid(
            "canonical gate input differs from the predeclared artifact set".into(),
        ));
    }
    rows.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    if let Some(first) = rows.first()
        && rows
            .iter()
            .any(|row| row.binding.campaign != first.binding.campaign)
    {
        return Err(FairError::Invalid(
            "canonical gate artifacts do not share one campaign binding".into(),
        ));
    }
    Ok(rows)
}

/// Freeze the canonical calibration only from all 54 validated source-bound
/// case artifacts. Raw measurement rows are intentionally insufficient.
pub fn freeze_scientific_validity_v2_calibration_artifacts(
    artifacts: &[ScientificValidityV2CaseArtifact],
) -> FairResult<V2CalibrationFreezeEnvelope> {
    let rows = validated_artifact_rows(artifacts, ScientificCorpusV2::calibration_specs())?;
    if rows
        .iter()
        .any(|row| row.status != V2GateRowStatus::Pass || row.conservative_max_wrms.is_none())
    {
        return Err(FairError::Invalid(
            "all 54 validated calibration artifacts must pass before freeze".into(),
        ));
    }
    let campaign_binding = rows
        .first()
        .expect("the canonical calibration set contains 54 artifacts")
        .binding
        .campaign
        .clone();
    let conservative_threshold_wrms = rows
        .iter()
        .filter_map(|row| row.conservative_max_wrms)
        .reduce(f64::max)
        .expect("passing canonical calibration rows all contain metrics");
    let payload = V2CalibrationFreezePayload {
        schema: "scientific-validity-v2-calibration-freeze-v1".into(),
        corpus_version: ScientificCorpusV2::VERSION.into(),
        profile: V2GateProfile::Canonical,
        campaign_label: V2GateProfile::Canonical.campaign_label().into(),
        threshold_derivation_id: V2_THRESHOLD_DERIVATION_ID.into(),
        campaign_binding,
        predeclared_holdout_family: ScientificFamily::Oregonator,
        sealed_remaining_holdout_families: vec![
            ScientificFamily::Pollution,
            ScientificFamily::MedicalAkzo,
            ScientificFamily::Brusselator2d,
        ],
        conservative_threshold_wrms,
        conservative_threshold_bits: conservative_threshold_wrms.to_bits(),
        rows,
    };
    let checksum_sha256 = v2_calibration_payload_checksum(&payload);
    let envelope = V2CalibrationFreezeEnvelope {
        payload,
        checksum_sha256,
    };
    verify_v2_calibration_freeze(&envelope)?;
    Ok(envelope)
}

/// Build canonical Oregonator replay evidence only from the three validated
/// source-bound case artifacts and an already verified calibration freeze.
pub fn replay_scientific_validity_v2_oregonator_artifacts(
    freeze: &V2CalibrationFreezeEnvelope,
    artifacts: &[ScientificValidityV2CaseArtifact],
) -> FairResult<V2OregonatorReplayEnvelope> {
    verify_v2_calibration_freeze(freeze)?;
    if freeze.payload.profile != V2GateProfile::Canonical {
        return Err(FairError::Invalid(
            "canonical Oregonator artifact replay requires a canonical freeze".into(),
        ));
    }
    let expected = ScientificCorpusV2::holdout_specs()
        .into_iter()
        .filter(|spec| spec.family == ScientificFamily::Oregonator)
        .collect::<Vec<_>>();
    let rows = validated_artifact_rows(artifacts, expected)?;
    if rows
        .iter()
        .any(|row| row.binding.campaign != freeze.payload.campaign_binding)
    {
        return Err(FairError::Invalid(
            "Oregonator artifact campaign differs from the calibration freeze".into(),
        ));
    }
    let rows = rows
        .into_iter()
        .map(|measurement| V2OregonatorReplayRow {
            within_frozen_threshold: measurement
                .conservative_max_wrms
                .is_some_and(|value| value <= freeze.payload.conservative_threshold_wrms),
            measurement,
        })
        .collect::<Vec<_>>();
    let overall_pass = rows
        .iter()
        .all(|row| row.measurement.status == V2GateRowStatus::Pass && row.within_frozen_threshold);
    let payload = V2OregonatorReplayPayload {
        schema: "scientific-validity-v2-oregonator-holdout-replay-v1".into(),
        corpus_version: freeze.payload.corpus_version.clone(),
        profile: freeze.payload.profile,
        campaign_label: freeze.payload.campaign_label.clone(),
        calibration_checksum_sha256: freeze.checksum_sha256.clone(),
        campaign_binding: freeze.payload.campaign_binding.clone(),
        frozen_threshold_wrms: freeze.payload.conservative_threshold_wrms,
        frozen_threshold_bits: freeze.payload.conservative_threshold_bits,
        rows,
        overall_pass,
    };
    let checksum_sha256 = v2_oregonator_replay_payload_checksum(&payload);
    let replay = V2OregonatorReplayEnvelope {
        payload,
        checksum_sha256,
    };
    verify_v2_oregonator_replay(&replay, freeze)?;
    Ok(replay)
}

pub fn scientific_validity_v2_campaign_specs(
    partition: CorpusPartition,
) -> Vec<ScientificCaseSpec> {
    match partition {
        CorpusPartition::Calibration => ScientificCorpusV2::calibration_specs(),
        CorpusPartition::Holdout => ScientificCorpusV2::holdout_specs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodas5p_integrators::{ScientificProblemSegment, scalar_linear_problem};

    fn test_arm(mode: V2CampaignOutputMode, status: V2CampaignArmStatus) -> V2CampaignArmEvidence {
        V2CampaignArmEvidence {
            mode,
            status,
            output_times: vec![0.0],
            states: vec![vec![1.0]],
            committed_output_count: 1,
            counters: WorkCounters::default(),
            diagnostics: AdaptiveRunDiagnostics::default(),
            internal_steps: 0,
            output_clipped_steps: 0,
            completed_segments: 0,
            boundary_restarts: 0,
            message: "synthetic".into(),
            metrics: Some(GlobalErrorMetrics {
                endpoint_l2: 1.0,
                max_grid_l2: 1.0,
                rms_grid_l2: 1.0,
                endpoint_wrms: 1.0,
                max_grid_wrms: 1.0,
                rms_grid_wrms: 1.0,
                reference_uncertainty_wrms: 0.0,
                conservative_max_wrms: 1.0,
            }),
            wall_seconds: 0.0,
            output_checksum_sha256: "0".repeat(64),
        }
    }

    #[test]
    fn failed_pair_retains_raw_arms_but_has_no_scientific_metric() {
        let mut clipped = test_arm(V2CampaignOutputMode::Clipped, V2CampaignArmStatus::Failure);
        let mut dense = test_arm(V2CampaignOutputMode::Dense, V2CampaignArmStatus::Success);
        let (status, metric, gap, _) = preserve_failed_pair(&mut clipped, &mut dense).unwrap();
        assert_eq!(status, V2GateRowStatus::Fail);
        assert!(metric.is_none());
        assert!(gap.is_none());
        assert!(clipped.metrics.is_none());
        assert!(dense.metrics.is_none());
        assert_eq!(clipped.states, vec![vec![1.0]]);
    }

    #[test]
    fn dense_arm_error_does_not_discard_a_completed_clipped_arm() {
        let mut spec = ScientificCorpusV2::calibration_specs().remove(0);
        spec.dimension = 1;
        spec.t_span = (0.0, 0.1);
        spec.output_times = vec![0.0, 0.1];
        spec.uniform_output_points = 2;
        spec.mandatory_breakpoints.clear();
        let (problem, y0) = scalar_linear_problem(-1.0, 1.0);
        let case = ScientificProblemCase {
            spec,
            problem: problem.clone(),
            y0,
            integration_segments: vec![ScientificProblemSegment {
                t_span: (0.0, 0.1),
                problem,
            }],
        };
        let clipped = test_arm(V2CampaignOutputMode::Clipped, V2CampaignArmStatus::Success);
        let expected_clipped = clipped.clone();
        let (mut actual_clipped, mut dense) = execute_arm_pair_with(&case, |mode| match mode {
            V2CampaignOutputMode::Clipped => Ok(clipped.clone()),
            V2CampaignOutputMode::Dense => Err(FairError::Invalid(
                "forced dense arm error after clipped completion".into(),
            )),
        });

        assert_eq!(actual_clipped, expected_clipped);
        assert_eq!(dense.status, V2CampaignArmStatus::Failure);
        assert_eq!(dense.output_times, vec![0.0]);
        assert_eq!(dense.states, vec![vec![1.0]]);
        assert!(dense.message.contains("forced dense arm error"));
        let (status, metric, gap, _) =
            preserve_failed_pair(&mut actual_clipped, &mut dense).unwrap();
        assert_eq!(status, V2GateRowStatus::Fail);
        assert!(metric.is_none());
        assert!(gap.is_none());
        assert!(actual_clipped.metrics.is_none());
        assert!(dense.metrics.is_none());
    }

    #[test]
    fn branch_fixed_arm_restarts_and_deduplicates_the_exact_boundary() {
        let mut spec = ScientificCorpusV2::calibration_specs().remove(0);
        spec.dimension = 1;
        spec.t_span = (0.0, 0.2);
        spec.output_times = vec![0.0, 0.1, 0.2];
        spec.uniform_output_points = 3;
        spec.mandatory_breakpoints = vec![0.1];
        spec.atol = 1.0e-8;
        spec.rtol = 1.0e-6;
        let (problem, y0) = scalar_linear_problem(-1.0, 1.0);
        let case = ScientificProblemCase {
            spec: spec.clone(),
            problem: problem.clone(),
            y0,
            integration_segments: vec![
                ScientificProblemSegment {
                    t_span: (0.0, 0.1),
                    problem: problem.clone(),
                },
                ScientificProblemSegment {
                    t_span: (0.1, 0.2),
                    problem,
                },
            ],
        };
        let arm = execute_arm(&case, &campaign_config(&spec), V2CampaignOutputMode::Dense).unwrap();
        assert_eq!(arm.status, V2CampaignArmStatus::Success);
        assert_eq!(arm.output_times, vec![0.0, 0.1, 0.2]);
        assert_eq!(arm.states.len(), 3);
        assert_eq!(arm.completed_segments, 2);
        assert_eq!(arm.boundary_restarts, 1);
        assert!(arm.diagnostics.is_structurally_consistent());
    }
}
