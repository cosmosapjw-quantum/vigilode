//! Two-arm admissibility runner under `TWO_ARM_ADMISSIBILITY_PROTOCOL_ID`
//! (audit F-007, all three parts; F-033 attribution arm).
//!
//! The clipped and dense arms are executed exactly as in the v2 campaign
//! (same solver protocol, same configuration). The dense arm additionally
//! carries the same-step interpolant audit (part C), and an optional third
//! arm solves every stage to the fixed fallback inner tolerance so that a
//! global-error exceedance can be attributed. The committed v2 artifacts and
//! their frozen `gap <= 0.1 E_dense` rule are not touched: this runner writes
//! its own record type.

use std::time::Instant;

use rodas5p_core::{WorkCounters, sha256_hex};
use rodas5p_integrators::{
    InterpolantAudit, ScientificCaseSpec, ScientificFamily, V2EvidenceAuthority,
};
use serde::{Deserialize, Serialize};

use crate::scientific_validity_v2_campaign::{
    DenseArmVariant, SOLVER_PROTOCOL_ID, V2CampaignArmEvidence, V2CampaignArmStatus,
    V2CampaignOutputMode, arm_failure_from_error, campaign_config, campaign_reference_binding,
    execute_arm_variant, scientific_validity_v2_compiled_revision,
    scientific_validity_v2_detected_revision,
};
use crate::{
    FairError, FairResult, NumericalReferenceBundleV2, TWO_ARM_ADMISSIBILITY_PROTOCOL_ID,
    TwoArmAdmissibility, classify_two_arm_v3_states,
};

pub const TWO_ARM_V3_CASE_SCHEMA: &str = "vigilode-two-arm-admissibility-case-v3";

/// `R_inner = E_dense / E_tight` above this attributes the dense excess to
/// inexact stage solves (fix design for F-033).
pub const TWO_ARM_V3_INEXACT_STAGE_RATIO: f64 = 2.0;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TwoArmV3ArmSummary {
    pub status: V2CampaignArmStatus,
    pub message: String,
    pub committed_output_count: usize,
    pub internal_steps: usize,
    pub attempts: usize,
    pub rejected_steps: usize,
    pub counters: WorkCounters,
    pub output_checksum_sha256: String,
}

impl TwoArmV3ArmSummary {
    fn from_arm(arm: &V2CampaignArmEvidence) -> Self {
        Self {
            status: arm.status,
            message: arm.message.clone(),
            committed_output_count: arm.committed_output_count,
            internal_steps: arm.internal_steps,
            attempts: arm.diagnostics.attempts,
            rejected_steps: arm.diagnostics.rejected_macro_steps,
            counters: arm.counters,
            output_checksum_sha256: arm.output_checksum_sha256.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TwoArmV3InterpolantSummary {
    /// Interior output times that the dense arm interpolated.
    pub interior_samples: usize,
    pub failed_substeps: usize,
    pub max_delta_wrms: Option<f64>,
    pub max_delta_time: Option<f64>,
    /// Work of the audit sub-steps; never part of either arm's counters.
    pub audit_counters: WorkCounters,
}

impl TwoArmV3InterpolantSummary {
    fn from_audit(audit: &InterpolantAudit) -> Self {
        let argmax = audit
            .samples
            .iter()
            .max_by(|a, b| a.delta_wrms.total_cmp(&b.delta_wrms));
        Self {
            interior_samples: audit.samples.len(),
            failed_substeps: audit.failed_substeps,
            max_delta_wrms: audit.max_delta_wrms(),
            max_delta_time: argmax.map(|sample| sample.t),
            audit_counters: audit.counters,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TwoArmV3AttributionReading {
    /// `R_inner > 2`: the forcing rule's inexact stages cause the excess.
    InexactStageSolves,
    /// `R_inner <= 2`: the exact-stage method makes the same error, so the
    /// excess is method or order reduction, to be settled by a fixed-h study.
    MethodOrOrderReduction,
    /// The tight-inner arm failed or had zero error; no ratio.
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TwoArmV3Attribution {
    pub tight_inner: TwoArmV3ArmSummary,
    pub dense_case_wrms: Option<f64>,
    pub tight_inner_case_wrms: Option<f64>,
    pub r_inner: Option<f64>,
    pub reading: TwoArmV3AttributionReading,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TwoArmAdmissibilityCaseV3 {
    pub schema_version: String,
    pub protocol_id: String,
    /// The arms run under the unchanged v2 solver protocol.
    pub solver_protocol_id: String,
    pub authority: V2EvidenceAuthority,
    pub code_revision: String,
    pub case_id: String,
    pub family: ScientificFamily,
    pub dimension: usize,
    pub atol: f64,
    pub rtol: f64,
    pub reference_checksum_sha256: String,
    pub clipped: TwoArmV3ArmSummary,
    pub dense: TwoArmV3ArmSummary,
    pub interpolant: TwoArmV3InterpolantSummary,
    pub attribution: Option<TwoArmV3Attribution>,
    /// `None` when an arm failed; the failure is in the arm summaries.
    pub admissibility: Option<TwoArmAdmissibility>,
    pub wall_seconds: f64,
    pub record_checksum_sha256: String,
}

/// Canonical runner: requires the clean compiled revision, as the v2 runner.
pub fn run_two_arm_admissibility_v3_case(
    spec: &ScientificCaseSpec,
    reference: &NumericalReferenceBundleV2,
    with_attribution: bool,
) -> FairResult<TwoArmAdmissibilityCaseV3> {
    let revision = scientific_validity_v2_compiled_revision()?;
    run_with_authority(
        spec,
        reference,
        with_attribution,
        revision,
        V2EvidenceAuthority::CanonicalV2Runner,
    )
}

/// Dirty-tree path; its records are permanently typed synthetic.
pub fn run_two_arm_admissibility_v3_case_synthetic_smoke(
    spec: &ScientificCaseSpec,
    reference: &NumericalReferenceBundleV2,
    with_attribution: bool,
) -> FairResult<TwoArmAdmissibilityCaseV3> {
    run_with_authority(
        spec,
        reference,
        with_attribution,
        scientific_validity_v2_detected_revision(),
        V2EvidenceAuthority::SyntheticCiSmoke,
    )
}

fn run_with_authority(
    spec: &ScientificCaseSpec,
    reference: &NumericalReferenceBundleV2,
    with_attribution: bool,
    code_revision: &str,
    authority: V2EvidenceAuthority,
) -> FairResult<TwoArmAdmissibilityCaseV3> {
    let started = Instant::now();
    let reference_binding = campaign_reference_binding(spec, reference, code_revision)?;
    let config = campaign_config(spec);
    let case = spec.build()?;
    let run = |mode, variant, audit: Option<&mut InterpolantAudit>| {
        let arm_started = Instant::now();
        execute_arm_variant(&case, &config, mode, variant, audit).unwrap_or_else(|error| {
            arm_failure_from_error(&case, mode, &error, arm_started.elapsed().as_secs_f64())
        })
    };
    let clipped = run(
        V2CampaignOutputMode::Clipped,
        DenseArmVariant::Production,
        None,
    );
    let mut audit = InterpolantAudit::default();
    let dense = run(
        V2CampaignOutputMode::Dense,
        DenseArmVariant::Production,
        Some(&mut audit),
    );
    let tight = with_attribution.then(|| {
        run(
            V2CampaignOutputMode::Dense,
            DenseArmVariant::FixedInnerTolerance,
            None,
        )
    });

    let succeeded = |arm: &V2CampaignArmEvidence| arm.status == V2CampaignArmStatus::Success;
    let interpolant = TwoArmV3InterpolantSummary::from_audit(&audit);
    let admissibility = if succeeded(&clipped) && succeeded(&dense) {
        Some(classify_two_arm_v3_states(
            &reference.wrms_basis,
            (&clipped.output_times, &clipped.states),
            (&dense.output_times, &dense.states),
            spec.atol,
            spec.rtol,
            interpolant.max_delta_wrms,
        )?)
    } else {
        None
    };
    let attribution = match tight {
        None => None,
        Some(tight) => {
            let case_basis = reference
                .wrms_basis
                .with_case_tolerance(spec.atol, spec.rtol)?;
            let dense_case_wrms = admissibility.as_ref().map(|row| row.dense_case_wrms);
            let tight_inner_case_wrms = if succeeded(&tight) {
                Some(
                    case_basis
                        .metrics(&tight.output_times, &tight.states)?
                        .max_grid_wrms,
                )
            } else {
                None
            };
            let r_inner = match (dense_case_wrms, tight_inner_case_wrms) {
                (Some(dense), Some(tight)) if tight > 0.0 => Some(dense / tight),
                _ => None,
            };
            let reading = match r_inner {
                Some(ratio) if ratio > TWO_ARM_V3_INEXACT_STAGE_RATIO => {
                    TwoArmV3AttributionReading::InexactStageSolves
                }
                Some(_) => TwoArmV3AttributionReading::MethodOrOrderReduction,
                None => TwoArmV3AttributionReading::Unavailable,
            };
            Some(TwoArmV3Attribution {
                tight_inner: TwoArmV3ArmSummary::from_arm(&tight),
                dense_case_wrms,
                tight_inner_case_wrms,
                r_inner,
                reading,
            })
        }
    };

    let mut record = TwoArmAdmissibilityCaseV3 {
        schema_version: TWO_ARM_V3_CASE_SCHEMA.into(),
        protocol_id: TWO_ARM_ADMISSIBILITY_PROTOCOL_ID.into(),
        solver_protocol_id: SOLVER_PROTOCOL_ID.into(),
        authority,
        code_revision: code_revision.into(),
        case_id: spec.id.clone(),
        family: spec.family,
        dimension: spec.dimension,
        atol: spec.atol,
        rtol: spec.rtol,
        reference_checksum_sha256: reference_binding.reference_checksum_sha256,
        clipped: TwoArmV3ArmSummary::from_arm(&clipped),
        dense: TwoArmV3ArmSummary::from_arm(&dense),
        interpolant,
        attribution,
        admissibility,
        wall_seconds: started.elapsed().as_secs_f64(),
        record_checksum_sha256: ZERO_SHA256.into(),
    };
    record.record_checksum_sha256 = record_checksum(&record)?;
    Ok(record)
}

/// Checksum over the record with its checksum zeroed and wall time removed,
/// so byte identity is scientific identity.
pub fn two_arm_v3_record_checksum(record: &TwoArmAdmissibilityCaseV3) -> FairResult<String> {
    record_checksum(record)
}

fn record_checksum(record: &TwoArmAdmissibilityCaseV3) -> FairResult<String> {
    let mut payload = record.clone();
    payload.record_checksum_sha256 = ZERO_SHA256.into();
    payload.wall_seconds = 0.0;
    let mut bytes = b"vigilode-two-arm-admissibility-case-v3\0".to_vec();
    bytes.extend_from_slice(&serde_json::to_vec(&payload).map_err(FairError::from)?);
    Ok(sha256_hex(&bytes))
}
