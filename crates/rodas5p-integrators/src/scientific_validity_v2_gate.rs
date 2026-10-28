//! Scientific-validity-v2 calibration freeze and sealed Oregonator replay contracts.
//!
//! This module is intentionally independent of every legacy G4/S5B0 threshold.  It consumes
//! typed v2 measurement rows and never executes a numerical campaign itself. Raw-row
//! derivation is therefore smoke-only; canonical envelopes are built by the source-bound
//! campaign layer after it validates the complete case-artifact set.

use std::collections::{BTreeMap, BTreeSet};

use rodas5p_core::{CoreError, CoreResult, sha256_hex};
use serde::{Deserialize, Serialize};

use crate::{CorpusPartition, ScientificCaseSpec, ScientificCorpusV2, ScientificFamily};

pub const V2_THRESHOLD_DERIVATION_ID: &str = "scientific-validity-v2-conservative-max-wrms-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum V2GateProfile {
    Smoke,
    Canonical,
}

impl V2GateProfile {
    pub const fn campaign_label(self) -> &'static str {
        match self {
            Self::Smoke => "ci-smoke-nonauthoritative",
            Self::Canonical => "canonical-scientific-campaign",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum V2GateRowStatus {
    Pass,
    Fail,
    ReferenceDominated,
    OutputPolicyDominated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum V2EvidenceAuthority {
    SyntheticCiSmoke,
    CanonicalV2Runner,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V2CampaignBinding {
    pub authority: V2EvidenceAuthority,
    pub runner_schema: String,
    pub candidate_id: String,
    pub code_revision: String,
    pub solver_config_sha256: String,
    pub wrms_scale_sha256: String,
    pub output_policy_protocol_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V2RowEvidenceBinding {
    pub campaign: V2CampaignBinding,
    pub reference_checksum_sha256: String,
    pub clipped_output_checksum_sha256: String,
    pub dense_output_checksum_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V2GateRow {
    pub case_id: String,
    pub family: ScientificFamily,
    pub partition: CorpusPartition,
    pub dimension: usize,
    pub atol: f64,
    pub rtol: f64,
    pub status: V2GateRowStatus,
    /// Absent for failure-preserving rows whose measurement never produced a finite metric.
    pub conservative_max_wrms: Option<f64>,
    /// Machine-verifiable lineage for the exact run that produced this row.
    /// Canonical rows cannot be deserialized or frozen without it.
    pub binding: V2RowEvidenceBinding,
    pub evidence: String,
    /// Operational timing is retained for diagnostics but excluded from scientific checksums.
    pub wall_seconds: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V2CalibrationFreezePayload {
    pub schema: String,
    pub corpus_version: String,
    pub profile: V2GateProfile,
    pub campaign_label: String,
    pub threshold_derivation_id: String,
    pub campaign_binding: V2CampaignBinding,
    pub predeclared_holdout_family: ScientificFamily,
    pub sealed_remaining_holdout_families: Vec<ScientificFamily>,
    pub conservative_threshold_wrms: f64,
    pub conservative_threshold_bits: u64,
    pub rows: Vec<V2GateRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V2CalibrationFreezeEnvelope {
    pub payload: V2CalibrationFreezePayload,
    pub checksum_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V2OregonatorReplayPayload {
    pub schema: String,
    pub corpus_version: String,
    pub profile: V2GateProfile,
    pub campaign_label: String,
    pub calibration_checksum_sha256: String,
    pub campaign_binding: V2CampaignBinding,
    pub frozen_threshold_wrms: f64,
    pub frozen_threshold_bits: u64,
    pub rows: Vec<V2OregonatorReplayRow>,
    pub overall_pass: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V2OregonatorReplayRow {
    #[serde(flatten)]
    pub measurement: V2GateRow,
    pub within_frozen_threshold: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V2OregonatorReplayEnvelope {
    pub payload: V2OregonatorReplayPayload,
    pub checksum_sha256: String,
}

/// Derive a nonauthoritative smoke freeze from typed rows.
///
/// `Canonical` is rejected because row metadata and unkeyed hashes do not authenticate
/// execution. The canonical producer must validate every complete case artifact first.
pub fn freeze_v2_calibration(
    profile: V2GateProfile,
    rows: Vec<V2GateRow>,
) -> CoreResult<V2CalibrationFreezeEnvelope> {
    if profile == V2GateProfile::Canonical {
        return Err(CoreError::InvalidInput(
            "canonical calibration cannot be frozen from raw rows; the source-bound campaign producer must validate all 54 case artifacts"
                .into(),
        ));
    }
    let rows = validate_and_sort_rows(
        rows,
        &expected_calibration_specs(profile),
        profile,
        true,
        None,
    )?;
    let campaign_binding = rows
        .first()
        .expect("v2 profiles always contain calibration rows")
        .binding
        .campaign
        .clone();
    let conservative_threshold_wrms = rows
        .iter()
        .map(|row| {
            row.conservative_max_wrms
                .expect("validated Pass calibration row has a metric")
        })
        .reduce(f64::max)
        .ok_or_else(|| CoreError::InvalidInput("v2 calibration rows cannot be empty".into()))?;
    let payload = V2CalibrationFreezePayload {
        schema: "scientific-validity-v2-calibration-freeze-v1".into(),
        corpus_version: ScientificCorpusV2::VERSION.into(),
        profile,
        campaign_label: profile.campaign_label().into(),
        threshold_derivation_id: V2_THRESHOLD_DERIVATION_ID.into(),
        campaign_binding,
        predeclared_holdout_family: ScientificFamily::Oregonator,
        sealed_remaining_holdout_families: sealed_holdout_families(),
        conservative_threshold_wrms,
        conservative_threshold_bits: conservative_threshold_wrms.to_bits(),
        rows,
    };
    let checksum_sha256 = v2_calibration_payload_checksum(&payload);
    Ok(V2CalibrationFreezeEnvelope {
        payload,
        checksum_sha256,
    })
}

pub fn verify_v2_calibration_freeze(envelope: &V2CalibrationFreezeEnvelope) -> CoreResult<()> {
    let payload = &envelope.payload;
    if payload.schema != "scientific-validity-v2-calibration-freeze-v1"
        || payload.corpus_version != ScientificCorpusV2::VERSION
        || payload.campaign_label != payload.profile.campaign_label()
        || payload.threshold_derivation_id != V2_THRESHOLD_DERIVATION_ID
        || payload.predeclared_holdout_family != ScientificFamily::Oregonator
        || payload.sealed_remaining_holdout_families != sealed_holdout_families()
    {
        return Err(CoreError::InvalidInput(
            "v2 calibration freeze metadata does not match the frozen protocol".into(),
        ));
    }
    let rows = validate_and_sort_rows(
        payload.rows.clone(),
        &expected_calibration_specs(payload.profile),
        payload.profile,
        true,
        Some(&payload.campaign_binding),
    )?;
    if rows != payload.rows {
        return Err(CoreError::InvalidInput(
            "v2 calibration freeze rows are not in canonical order".into(),
        ));
    }
    let threshold = rows
        .iter()
        .map(|row| {
            row.conservative_max_wrms
                .expect("validated Pass calibration row has a metric")
        })
        .reduce(f64::max)
        .expect("v2 profiles always contain calibration rows");
    if payload.conservative_threshold_bits != threshold.to_bits()
        || payload.conservative_threshold_wrms.to_bits() != threshold.to_bits()
    {
        return Err(CoreError::InvalidInput(
            "v2 calibration freeze threshold does not match eligible rows".into(),
        ));
    }
    if envelope.checksum_sha256 != v2_calibration_payload_checksum(payload) {
        return Err(CoreError::InvalidInput(
            "v2 calibration freeze checksum mismatch".into(),
        ));
    }
    Ok(())
}

/// Replay nonauthoritative smoke rows against a verified smoke freeze.
///
/// Canonical replay is artifact-only and lives in the source-bound campaign layer.
pub fn replay_v2_oregonator_holdout(
    freeze: &V2CalibrationFreezeEnvelope,
    rows: Vec<V2GateRow>,
) -> CoreResult<V2OregonatorReplayEnvelope> {
    // Checksum and complete freeze validation deliberately precede all holdout inspection.
    verify_v2_calibration_freeze(freeze)?;
    if freeze.payload.profile == V2GateProfile::Canonical {
        return Err(CoreError::InvalidInput(
            "canonical Oregonator replay cannot consume raw rows; the source-bound campaign producer must validate all three case artifacts"
                .into(),
        ));
    }
    let rows = validate_and_sort_rows(
        rows,
        &expected_oregonator_specs(freeze.payload.profile),
        freeze.payload.profile,
        false,
        Some(&freeze.payload.campaign_binding),
    )?;
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
    Ok(V2OregonatorReplayEnvelope {
        payload,
        checksum_sha256,
    })
}

pub fn verify_v2_oregonator_replay(
    replay: &V2OregonatorReplayEnvelope,
    freeze: &V2CalibrationFreezeEnvelope,
) -> CoreResult<()> {
    verify_v2_calibration_freeze(freeze)?;
    let payload = &replay.payload;
    if payload.schema != "scientific-validity-v2-oregonator-holdout-replay-v1"
        || payload.corpus_version != freeze.payload.corpus_version
        || payload.profile != freeze.payload.profile
        || payload.campaign_label != freeze.payload.campaign_label
        || payload.calibration_checksum_sha256 != freeze.checksum_sha256
        || payload.campaign_binding != freeze.payload.campaign_binding
        || payload.frozen_threshold_bits != freeze.payload.conservative_threshold_bits
        || payload.frozen_threshold_wrms.to_bits()
            != freeze.payload.conservative_threshold_wrms.to_bits()
    {
        return Err(CoreError::InvalidInput(
            "v2 Oregonator replay metadata does not match its calibration freeze".into(),
        ));
    }
    let measurements = payload
        .rows
        .iter()
        .map(|row| row.measurement.clone())
        .collect::<Vec<_>>();
    let measurements = validate_and_sort_rows(
        measurements,
        &expected_oregonator_specs(payload.profile),
        payload.profile,
        false,
        Some(&payload.campaign_binding),
    )?;
    if payload
        .rows
        .iter()
        .map(|row| row.measurement.case_id.as_str())
        .ne(measurements.iter().map(|row| row.case_id.as_str()))
    {
        return Err(CoreError::InvalidInput(
            "v2 Oregonator replay rows are not in canonical order".into(),
        ));
    }
    for row in &payload.rows {
        let expected_within = row
            .measurement
            .conservative_max_wrms
            .is_some_and(|value| value <= payload.frozen_threshold_wrms);
        if row.within_frozen_threshold != expected_within {
            return Err(CoreError::InvalidInput(format!(
                "v2 Oregonator replay threshold classification mismatch: {}",
                row.measurement.case_id
            )));
        }
    }
    let expected_overall = payload
        .rows
        .iter()
        .all(|row| row.measurement.status == V2GateRowStatus::Pass && row.within_frozen_threshold);
    if payload.overall_pass != expected_overall {
        return Err(CoreError::InvalidInput(
            "v2 Oregonator replay overall verdict mismatch".into(),
        ));
    }
    if replay.checksum_sha256 != v2_oregonator_replay_payload_checksum(payload) {
        return Err(CoreError::InvalidInput(
            "v2 Oregonator replay checksum mismatch".into(),
        ));
    }
    Ok(())
}

fn sealed_holdout_families() -> Vec<ScientificFamily> {
    vec![
        ScientificFamily::Pollution,
        ScientificFamily::MedicalAkzo,
        ScientificFamily::Brusselator2d,
    ]
}

fn expected_calibration_specs(profile: V2GateProfile) -> Vec<ScientificCaseSpec> {
    ScientificCorpusV2::calibration_threshold_specs()
        .into_iter()
        .filter(|spec| {
            profile == V2GateProfile::Canonical
                || (spec.dimension == 96 && spec.rtol.to_bits() == 1.0e-4_f64.to_bits())
        })
        .collect()
}

fn expected_oregonator_specs(profile: V2GateProfile) -> Vec<ScientificCaseSpec> {
    ScientificCorpusV2::holdout_specs()
        .into_iter()
        .filter(|spec| spec.family == ScientificFamily::Oregonator)
        .filter(|spec| {
            profile == V2GateProfile::Canonical || spec.rtol.to_bits() == 1.0e-4_f64.to_bits()
        })
        .collect()
}

fn validate_and_sort_rows(
    mut rows: Vec<V2GateRow>,
    expected_specs: &[ScientificCaseSpec],
    profile: V2GateProfile,
    require_pass: bool,
    expected_campaign: Option<&V2CampaignBinding>,
) -> CoreResult<Vec<V2GateRow>> {
    let expected = expected_specs
        .iter()
        .map(|spec| (spec.id.as_str(), spec))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    for row in &rows {
        if !seen.insert(row.case_id.as_str()) {
            return Err(CoreError::InvalidInput(format!(
                "duplicate v2 gate row: {}",
                row.case_id
            )));
        }
        let Some(spec) = expected.get(row.case_id.as_str()) else {
            return Err(CoreError::InvalidInput(format!(
                "unexpected v2 gate row: {}",
                row.case_id
            )));
        };
        if row.family != spec.family
            || row.partition != spec.partition
            || row.dimension != spec.dimension
            || row.atol.to_bits() != spec.atol.to_bits()
            || row.rtol.to_bits() != spec.rtol.to_bits()
        {
            return Err(CoreError::InvalidInput(format!(
                "v2 gate row metadata does not match corpus spec: {}",
                row.case_id
            )));
        }
        if row.evidence.trim().is_empty() {
            return Err(CoreError::InvalidInput(format!(
                "v2 gate row lacks failure-preserving evidence: {}",
                row.case_id
            )));
        }
        validate_evidence_binding(profile, &row.binding)?;
        if expected_campaign.is_some_and(|binding| binding != &row.binding.campaign) {
            return Err(CoreError::InvalidInput(format!(
                "v2 gate row campaign binding differs from the frozen campaign: {}",
                row.case_id
            )));
        }
        if row
            .wall_seconds
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(CoreError::InvalidInput(format!(
                "v2 gate row wall time must be finite and nonnegative: {}",
                row.case_id
            )));
        }
        if row
            .conservative_max_wrms
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(CoreError::InvalidInput(format!(
                "v2 gate row metric must be finite and nonnegative when present: {}",
                row.case_id
            )));
        }
        if row.status == V2GateRowStatus::Pass && row.conservative_max_wrms.is_none() {
            return Err(CoreError::InvalidInput(format!(
                "passing v2 gate row requires a finite metric: {}",
                row.case_id
            )));
        }
        if require_pass && row.status != V2GateRowStatus::Pass {
            return Err(CoreError::InvalidInput(format!(
                "all v2 calibration rows must pass before threshold freeze: {} is {:?}",
                row.case_id, row.status
            )));
        }
    }
    let missing = expected
        .keys()
        .filter(|case_id| !seen.contains(**case_id))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(CoreError::InvalidInput(format!(
            "missing expected v2 gate rows: {}",
            missing.join(",")
        )));
    }
    if let Some(first) = rows.first()
        && rows
            .iter()
            .any(|row| row.binding.campaign != first.binding.campaign)
    {
        return Err(CoreError::InvalidInput(
            "v2 gate rows do not share one campaign binding".into(),
        ));
    }
    rows.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    Ok(rows)
}

fn validate_evidence_binding(
    profile: V2GateProfile,
    binding: &V2RowEvidenceBinding,
) -> CoreResult<()> {
    let campaign = &binding.campaign;
    let authority_valid = match profile {
        V2GateProfile::Smoke => {
            campaign.authority == V2EvidenceAuthority::SyntheticCiSmoke
                && campaign.runner_schema == "scientific-validity-v2-synthetic-smoke-v1"
        }
        V2GateProfile::Canonical => {
            campaign.authority == V2EvidenceAuthority::CanonicalV2Runner
                && campaign.runner_schema == "scientific-validity-v2-campaign-runner-v1"
                && is_lower_hex(&campaign.code_revision, 40)
        }
    };
    if !authority_valid || campaign.candidate_id.trim().is_empty() {
        return Err(CoreError::InvalidInput(
            "v2 gate row has invalid profile-bound measurement authority".into(),
        ));
    }
    for (label, checksum) in [
        ("solver config", &campaign.solver_config_sha256),
        ("WRMS scale", &campaign.wrms_scale_sha256),
        (
            "output policy protocol",
            &campaign.output_policy_protocol_sha256,
        ),
        ("reference", &binding.reference_checksum_sha256),
        ("clipped output", &binding.clipped_output_checksum_sha256),
        ("dense output", &binding.dense_output_checksum_sha256),
    ] {
        if !is_lower_hex(checksum, 64) {
            return Err(CoreError::InvalidInput(format!(
                "v2 gate row {label} checksum is not lowercase SHA-256"
            )));
        }
    }
    if binding.clipped_output_checksum_sha256 == binding.dense_output_checksum_sha256 {
        return Err(CoreError::InvalidInput(
            "v2 gate row clipped and dense evidence must be distinct bound artifacts".into(),
        ));
    }
    Ok(())
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_str(bytes: &mut Vec<u8>, value: &str) {
    push_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn push_profile(bytes: &mut Vec<u8>, profile: V2GateProfile) {
    bytes.push(match profile {
        V2GateProfile::Smoke => 0,
        V2GateProfile::Canonical => 1,
    });
}

fn push_family(bytes: &mut Vec<u8>, family: ScientificFamily) {
    push_str(bytes, family.as_str());
}

fn push_campaign_binding(bytes: &mut Vec<u8>, binding: &V2CampaignBinding) {
    bytes.push(match binding.authority {
        V2EvidenceAuthority::SyntheticCiSmoke => 0,
        V2EvidenceAuthority::CanonicalV2Runner => 1,
    });
    push_str(bytes, &binding.runner_schema);
    push_str(bytes, &binding.candidate_id);
    push_str(bytes, &binding.code_revision);
    push_str(bytes, &binding.solver_config_sha256);
    push_str(bytes, &binding.wrms_scale_sha256);
    push_str(bytes, &binding.output_policy_protocol_sha256);
}

fn push_evidence_binding(bytes: &mut Vec<u8>, binding: &V2RowEvidenceBinding) {
    push_campaign_binding(bytes, &binding.campaign);
    push_str(bytes, &binding.reference_checksum_sha256);
    push_str(bytes, &binding.clipped_output_checksum_sha256);
    push_str(bytes, &binding.dense_output_checksum_sha256);
}

fn push_row(bytes: &mut Vec<u8>, row: &V2GateRow) {
    push_str(bytes, &row.case_id);
    push_family(bytes, row.family);
    bytes.push(match row.partition {
        CorpusPartition::Calibration => 0,
        CorpusPartition::Holdout => 1,
    });
    push_u64(bytes, row.dimension as u64);
    push_u64(bytes, row.atol.to_bits());
    push_u64(bytes, row.rtol.to_bits());
    bytes.push(match row.status {
        V2GateRowStatus::Pass => 0,
        V2GateRowStatus::Fail => 1,
        V2GateRowStatus::ReferenceDominated => 2,
        V2GateRowStatus::OutputPolicyDominated => 3,
    });
    match row.conservative_max_wrms {
        Some(value) => {
            bytes.push(1);
            push_u64(bytes, value.to_bits());
        }
        None => bytes.push(0),
    }
    push_evidence_binding(bytes, &row.binding);
    push_str(bytes, &row.evidence);
}

/// Deterministic payload hash used by source-bound artifact producers and verifiers.
/// Computing this hash does not by itself confer scientific authority.
pub fn v2_calibration_payload_checksum(payload: &V2CalibrationFreezePayload) -> String {
    let mut bytes = Vec::new();
    push_str(&mut bytes, &payload.schema);
    push_str(&mut bytes, &payload.corpus_version);
    push_profile(&mut bytes, payload.profile);
    push_str(&mut bytes, &payload.campaign_label);
    push_str(&mut bytes, &payload.threshold_derivation_id);
    push_campaign_binding(&mut bytes, &payload.campaign_binding);
    push_family(&mut bytes, payload.predeclared_holdout_family);
    push_u64(
        &mut bytes,
        payload.sealed_remaining_holdout_families.len() as u64,
    );
    for family in &payload.sealed_remaining_holdout_families {
        push_family(&mut bytes, *family);
    }
    push_u64(&mut bytes, payload.conservative_threshold_bits);
    let mut rows = payload.rows.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    push_u64(&mut bytes, rows.len() as u64);
    for row in rows {
        push_row(&mut bytes, row);
    }
    sha256_hex(&bytes)
}

/// Deterministic payload hash used by source-bound artifact producers and verifiers.
/// Computing this hash does not by itself confer scientific authority.
pub fn v2_oregonator_replay_payload_checksum(payload: &V2OregonatorReplayPayload) -> String {
    let mut bytes = Vec::new();
    push_str(&mut bytes, &payload.schema);
    push_str(&mut bytes, &payload.corpus_version);
    push_profile(&mut bytes, payload.profile);
    push_str(&mut bytes, &payload.campaign_label);
    push_str(&mut bytes, &payload.calibration_checksum_sha256);
    push_campaign_binding(&mut bytes, &payload.campaign_binding);
    push_u64(&mut bytes, payload.frozen_threshold_bits);
    bytes.push(u8::from(payload.overall_pass));
    let mut rows = payload.rows.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.measurement.case_id.cmp(&right.measurement.case_id));
    push_u64(&mut bytes, rows.len() as u64);
    for row in rows {
        push_row(&mut bytes, &row.measurement);
        bytes.push(u8::from(row.within_frozen_threshold));
    }
    sha256_hex(&bytes)
}
