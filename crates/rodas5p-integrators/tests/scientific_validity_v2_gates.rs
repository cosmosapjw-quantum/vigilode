use rodas5p_core::sha256_hex;
use rodas5p_integrators::{
    CorpusPartition, ScientificCaseSpec, ScientificCorpusV2, ScientificFamily, V2CampaignBinding,
    V2EvidenceAuthority, V2GateProfile, V2GateRow, V2GateRowStatus, V2RowEvidenceBinding,
    freeze_v2_calibration, replay_v2_oregonator_holdout, verify_v2_calibration_freeze,
    verify_v2_oregonator_replay,
};

fn digest(label: &str) -> String {
    sha256_hex(label.as_bytes())
}

fn campaign_binding(profile: V2GateProfile) -> V2CampaignBinding {
    V2CampaignBinding {
        authority: match profile {
            V2GateProfile::Smoke => V2EvidenceAuthority::SyntheticCiSmoke,
            V2GateProfile::Canonical => V2EvidenceAuthority::CanonicalV2Runner,
        },
        runner_schema: match profile {
            V2GateProfile::Smoke => "scientific-validity-v2-synthetic-smoke-v1",
            V2GateProfile::Canonical => "scientific-validity-v2-campaign-runner-v1",
        }
        .into(),
        candidate_id: "sequential-rodas5p-gmres-wrms-forcing-v2".into(),
        code_revision: match profile {
            V2GateProfile::Smoke => "synthetic-ci-smoke-not-a-revision",
            V2GateProfile::Canonical => "1234567890abcdef1234567890abcdef12345678",
        }
        .into(),
        solver_config_sha256: digest("solver-config-v2"),
        wrms_scale_sha256: digest("wrms-scale-v2"),
        output_policy_protocol_sha256: digest("clipped-dense-protocol-v2"),
    }
}

fn row(spec: &ScientificCaseSpec, value: f64, profile: V2GateProfile) -> V2GateRow {
    V2GateRow {
        case_id: spec.id.clone(),
        family: spec.family,
        partition: spec.partition,
        dimension: spec.dimension,
        atol: spec.atol,
        rtol: spec.rtol,
        status: V2GateRowStatus::Pass,
        conservative_max_wrms: Some(value),
        binding: V2RowEvidenceBinding {
            campaign: campaign_binding(profile),
            reference_checksum_sha256: digest(&format!("{}-reference", spec.id)),
            clipped_output_checksum_sha256: digest(&format!("{}-clipped", spec.id)),
            dense_output_checksum_sha256: digest(&format!("{}-dense", spec.id)),
        },
        evidence: "deterministic contract fixture".into(),
        wall_seconds: Some(123.0),
    }
}

fn canonical_calibration_rows() -> Vec<V2GateRow> {
    ScientificCorpusV2::calibration_specs()
        .into_iter()
        .enumerate()
        .map(|(index, spec)| row(&spec, 0.001 * (index + 1) as f64, V2GateProfile::Canonical))
        .collect()
}

fn oregonator_rows(profile: V2GateProfile) -> Vec<V2GateRow> {
    ScientificCorpusV2::holdout_specs()
        .into_iter()
        .filter(|spec| spec.family == ScientificFamily::Oregonator)
        .filter(|spec| {
            profile == V2GateProfile::Canonical || spec.rtol.to_bits() == 1.0e-4_f64.to_bits()
        })
        .enumerate()
        .map(|(index, spec)| row(&spec, 0.02 * (index + 1) as f64, profile))
        .collect()
}

fn smoke_calibration_rows() -> Vec<V2GateRow> {
    ScientificCorpusV2::calibration_specs()
        .into_iter()
        .filter(|spec| spec.dimension == 96 && spec.rtol.to_bits() == 1.0e-4_f64.to_bits())
        .enumerate()
        .map(|(index, spec)| row(&spec, 0.01 * (index + 1) as f64, V2GateProfile::Smoke))
        .collect()
}

#[test]
fn calibration_freeze_rejects_incomplete_expected_set() {
    // Defect caught: freezing a partial campaign silently lowers the conservative maximum.
    let mut rows = smoke_calibration_rows();
    assert_eq!(rows.len(), 6);
    rows.pop();
    assert!(freeze_v2_calibration(V2GateProfile::Smoke, rows).is_err());
}

#[test]
fn calibration_freeze_rejects_holdout_contamination() {
    // Defect caught: a holdout measurement can feed back into threshold selection.
    let mut rows = smoke_calibration_rows();
    let oregonator = ScientificCorpusV2::holdout_specs()
        .into_iter()
        .find(|spec| {
            spec.family == ScientificFamily::Oregonator
                && spec.rtol.to_bits() == 1.0e-4_f64.to_bits()
        })
        .unwrap();
    assert_eq!(oregonator.partition, CorpusPartition::Holdout);
    rows.push(row(&oregonator, 999.0, V2GateProfile::Smoke));
    assert!(freeze_v2_calibration(V2GateProfile::Smoke, rows).is_err());
}

#[test]
fn profiles_freeze_exact_sets_and_the_conservative_maximum() {
    // Defect caught: a profile silently selects the legacy dimensions/tolerances or an average.
    let smoke = freeze_v2_calibration(V2GateProfile::Smoke, smoke_calibration_rows()).unwrap();
    assert_eq!(smoke.payload.rows.len(), 6);
    assert_eq!(smoke.payload.campaign_label, "ci-smoke-nonauthoritative");
    assert_eq!(
        smoke.payload.predeclared_holdout_family,
        ScientificFamily::Oregonator
    );
    assert_eq!(
        smoke.payload.sealed_remaining_holdout_families,
        vec![
            ScientificFamily::Pollution,
            ScientificFamily::MedicalAkzo,
            ScientificFamily::Brusselator2d,
        ]
    );
    assert_eq!(
        smoke.payload.conservative_threshold_wrms.to_bits(),
        0.06_f64.to_bits()
    );
    assert_eq!(
        smoke.payload.conservative_threshold_bits,
        smoke.payload.conservative_threshold_wrms.to_bits()
    );
    assert_eq!(smoke.checksum_sha256.len(), 64);

    let error = freeze_v2_calibration(V2GateProfile::Canonical, canonical_calibration_rows())
        .unwrap_err()
        .to_string();
    assert!(error.contains("cannot be frozen from raw rows"), "{error}");
}

#[test]
fn duplicate_unexpected_and_nonpass_calibration_rows_cannot_freeze() {
    // Defect caught: malformed or ineligible rows influence a threshold before being rejected.
    let base = smoke_calibration_rows();

    let mut duplicate = base.clone();
    duplicate.push(base[0].clone());
    assert!(freeze_v2_calibration(V2GateProfile::Smoke, duplicate).is_err());

    let mut unexpected = base.clone();
    unexpected[0].case_id = "unexpected-calibration-case".into();
    assert!(freeze_v2_calibration(V2GateProfile::Smoke, unexpected).is_err());

    for status in [
        V2GateRowStatus::Fail,
        V2GateRowStatus::ReferenceDominated,
        V2GateRowStatus::OutputPolicyDominated,
    ] {
        let mut ineligible = base.clone();
        ineligible[0].status = status;
        ineligible[0].conservative_max_wrms = if status == V2GateRowStatus::Fail {
            None
        } else {
            Some(9_999.0)
        };
        assert!(freeze_v2_calibration(V2GateProfile::Smoke, ineligible).is_err());
    }
}

#[test]
fn canonical_raw_rows_cannot_create_authority_even_when_sha_shaped() {
    let rows = canonical_calibration_rows();
    assert!(rows.iter().all(|row| {
        row.binding.campaign.authority == V2EvidenceAuthority::CanonicalV2Runner
            && row.binding.reference_checksum_sha256.len() == 64
    }));
    assert!(freeze_v2_calibration(V2GateProfile::Canonical, rows).is_err());
}

#[test]
fn freeze_checksum_is_order_and_wall_time_invariant_but_payload_sensitive() {
    // Defect caught: nondeterministic row order or wall time changes an immutable scientific hash.
    let rows = smoke_calibration_rows();
    let first = freeze_v2_calibration(V2GateProfile::Smoke, rows.clone()).unwrap();
    let mut reordered = rows;
    reordered.reverse();
    for row in &mut reordered {
        row.wall_seconds = Some(9876.5);
    }
    let second = freeze_v2_calibration(V2GateProfile::Smoke, reordered).unwrap();
    assert_eq!(first.checksum_sha256, second.checksum_sha256);
    assert_eq!(
        first
            .payload
            .rows
            .iter()
            .map(|row| row.case_id.as_str())
            .collect::<Vec<_>>(),
        second
            .payload
            .rows
            .iter()
            .map(|row| row.case_id.as_str())
            .collect::<Vec<_>>()
    );
    verify_v2_calibration_freeze(&first).unwrap();

    let mut corrupt = first.clone();
    corrupt.payload.rows[0].conservative_max_wrms = Some(0.059);
    assert!(verify_v2_calibration_freeze(&corrupt).is_err());
}

#[test]
fn replay_verifies_freeze_first_and_accepts_only_the_exact_oregonator_set() {
    // Defect caught: malformed holdout input masks a corrupted freeze or opens sealed families.
    let freeze = freeze_v2_calibration(V2GateProfile::Smoke, smoke_calibration_rows()).unwrap();
    let mut corrupt = freeze.clone();
    corrupt.checksum_sha256 = "0".repeat(64);
    let pollution = ScientificCorpusV2::holdout_specs()
        .into_iter()
        .find(|spec| spec.family == ScientificFamily::Pollution)
        .unwrap();
    let error =
        replay_v2_oregonator_holdout(&corrupt, vec![row(&pollution, 0.01, V2GateProfile::Smoke)])
            .unwrap_err()
            .to_string();
    assert!(error.contains("checksum"), "{error}");

    assert!(replay_v2_oregonator_holdout(&freeze, Vec::new()).is_err());
    assert!(
        replay_v2_oregonator_holdout(&freeze, vec![row(&pollution, 0.01, V2GateProfile::Smoke)])
            .is_err()
    );
}

#[test]
fn replay_preserves_typed_evidence_and_never_changes_the_frozen_threshold() {
    // Defect caught: dominated/failure evidence is dropped or fed back into the frozen threshold.
    let freeze = freeze_v2_calibration(V2GateProfile::Smoke, smoke_calibration_rows()).unwrap();
    let mut rows = oregonator_rows(V2GateProfile::Smoke);
    assert_eq!(rows.len(), 1);
    rows[0].status = V2GateRowStatus::ReferenceDominated;
    rows[0].conservative_max_wrms = Some(0.02);

    let replay = replay_v2_oregonator_holdout(&freeze, rows.clone()).unwrap();
    assert_eq!(replay.payload.rows.len(), 1);
    assert!(!replay.payload.overall_pass);
    assert_eq!(
        replay.payload.calibration_checksum_sha256,
        freeze.checksum_sha256
    );
    assert_eq!(
        replay.payload.frozen_threshold_bits,
        freeze.payload.conservative_threshold_bits
    );
    assert_eq!(
        replay.payload.frozen_threshold_wrms.to_bits(),
        freeze.payload.conservative_threshold_wrms.to_bits()
    );
    for (reported, input) in replay.payload.rows.iter().zip(rows) {
        assert_eq!(reported.measurement, input);
    }
    assert!(replay.payload.rows[0].within_frozen_threshold);
    verify_v2_oregonator_replay(&replay, &freeze).unwrap();

    let mut corrupt = replay;
    corrupt.payload.rows[0].measurement.evidence = "mutated after replay".into();
    assert!(verify_v2_oregonator_replay(&corrupt, &freeze).is_err());
}
