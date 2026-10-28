use std::{fs, process::Command};

use rodas5p_core::sha256_hex;
use rodas5p_integrators::{
    ScientificCaseSpec, ScientificCorpusV2, ScientificFamily, V2CalibrationFreezeEnvelope,
    V2CampaignBinding, V2EvidenceAuthority, V2GateProfile, V2GateRow, V2GateRowStatus,
    V2OregonatorReplayEnvelope, V2RowEvidenceBinding, freeze_v2_calibration,
    verify_v2_calibration_freeze, verify_v2_oregonator_replay,
};

fn digest(label: &str) -> String {
    sha256_hex(label.as_bytes())
}

fn smoke_binding(spec: &ScientificCaseSpec) -> V2RowEvidenceBinding {
    V2RowEvidenceBinding {
        campaign: V2CampaignBinding {
            authority: V2EvidenceAuthority::SyntheticCiSmoke,
            runner_schema: "scientific-validity-v2-synthetic-smoke-v1".into(),
            candidate_id: "synthetic-cli-smoke".into(),
            code_revision: "synthetic-ci-smoke-not-a-revision".into(),
            solver_config_sha256: digest("cli-solver-config"),
            wrms_scale_sha256: digest("cli-wrms-scale"),
            output_policy_protocol_sha256: digest("cli-output-policy"),
        },
        reference_checksum_sha256: digest(&format!("{}-reference", spec.id)),
        clipped_output_checksum_sha256: digest(&format!("{}-clipped", spec.id)),
        dense_output_checksum_sha256: digest(&format!("{}-dense", spec.id)),
    }
}

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rodas5p-scientific-validity-v2-{}-{name}",
        std::process::id()
    ))
}

fn row(spec: &ScientificCaseSpec, value: Option<f64>, status: V2GateRowStatus) -> V2GateRow {
    V2GateRow {
        case_id: spec.id.clone(),
        family: spec.family,
        partition: spec.partition,
        dimension: spec.dimension,
        atol: spec.atol,
        rtol: spec.rtol,
        status,
        conservative_max_wrms: value,
        binding: smoke_binding(spec),
        evidence: "CLI deterministic fixture evidence".into(),
        wall_seconds: Some(0.25),
    }
}

fn smoke_calibration_rows() -> Vec<V2GateRow> {
    ScientificCorpusV2::calibration_specs()
        .into_iter()
        .filter(|spec| spec.dimension == 96 && spec.rtol.to_bits() == 1.0e-4_f64.to_bits())
        .enumerate()
        .map(|(index, spec)| {
            row(
                &spec,
                Some(0.01 * (index + 1) as f64),
                V2GateRowStatus::Pass,
            )
        })
        .collect()
}

fn clean(paths: &[&std::path::Path]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn freeze_command_is_profile_bound_and_refuses_overwrite_or_family_selection() {
    // Defect caught: a gate command overwrites an immutable freeze or exposes holdout selection.
    let input = temp_path("freeze-input.json");
    let output = temp_path("freeze-output.json");
    let forbidden = temp_path("freeze-family-selector-must-not-exist.json");
    clean(&[&input, &output, &forbidden]);
    fs::write(
        &input,
        serde_json::to_vec_pretty(&smoke_calibration_rows()).unwrap(),
    )
    .unwrap();

    let first = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-freeze",
            "--profile",
            "smoke",
            "--input",
        ])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let frozen_bytes = fs::read(&output).unwrap();
    let frozen: serde_json::Value = serde_json::from_slice(&frozen_bytes).unwrap();
    assert_eq!(
        frozen["payload"]["campaign_label"],
        "ci-smoke-nonauthoritative"
    );
    assert_eq!(frozen["payload"]["rows"].as_array().unwrap().len(), 6);

    let second = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-freeze",
            "--profile",
            "smoke",
            "--input",
        ])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!second.status.success());
    assert_eq!(fs::read(&output).unwrap(), frozen_bytes);

    let selector = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-freeze",
            "--profile",
            "smoke",
            "--family",
            "oregonator",
            "--input",
        ])
        .arg(&input)
        .arg("--output")
        .arg(&forbidden)
        .output()
        .unwrap();
    assert!(!selector.status.success());
    assert!(!forbidden.exists());
    clean(&[&input, &output, &forbidden]);
}

#[test]
fn replay_command_preserves_dominated_evidence_and_refuses_overwrite() {
    // Defect caught: the CLI suppresses a scientifically dominated row or rewrites its freeze.
    let input = temp_path("replay-input.json");
    let freeze_path = temp_path("replay-freeze.json");
    let output = temp_path("replay-output.json");
    clean(&[&input, &freeze_path, &output]);
    let freeze = freeze_v2_calibration(V2GateProfile::Smoke, smoke_calibration_rows()).unwrap();
    let oregonator = ScientificCorpusV2::holdout_specs()
        .into_iter()
        .find(|spec| {
            spec.family == ScientificFamily::Oregonator
                && spec.rtol.to_bits() == 1.0e-4_f64.to_bits()
        })
        .unwrap();
    let mut dominated = row(&oregonator, Some(0.02), V2GateRowStatus::ReferenceDominated);
    dominated.evidence = "reference uncertainty exceeded one tenth of measured error".into();
    fs::write(&freeze_path, serde_json::to_vec_pretty(&freeze).unwrap()).unwrap();
    fs::write(&input, serde_json::to_vec_pretty(&vec![dominated]).unwrap()).unwrap();

    let run = || {
        Command::new(env!("CARGO_BIN_EXE_rodas5p"))
            .args([
                "scientific-validity-v2-holdout-replay",
                "--profile",
                "smoke",
                "--freeze",
            ])
            .arg(&freeze_path)
            .args(["--input"])
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap()
    };
    let first = run();
    assert!(!first.status.success());
    let replay_bytes = fs::read(&output).unwrap();
    let replay: serde_json::Value = serde_json::from_slice(&replay_bytes).unwrap();
    assert_eq!(replay["payload"]["overall_pass"], false);
    assert_eq!(
        replay["payload"]["rows"][0]["status"],
        "reference-dominated"
    );
    assert_eq!(
        replay["payload"]["rows"][0]["evidence"],
        "reference uncertainty exceeded one tenth of measured error"
    );
    assert_eq!(
        replay["payload"]["frozen_threshold_bits"],
        freeze.payload.conservative_threshold_bits
    );

    let second = run();
    assert!(!second.status.success());
    assert_eq!(fs::read(&output).unwrap(), replay_bytes);
    clean(&[&input, &freeze_path, &output]);
}

#[test]
fn corrupt_freeze_is_rejected_before_the_holdout_input_is_opened() {
    let freeze_path = temp_path("access-order-corrupt-freeze.json");
    let missing_input = temp_path("access-order-input-must-not-be-opened.json");
    let output = temp_path("access-order-output.json");
    clean(&[&freeze_path, &missing_input, &output]);
    let mut freeze = freeze_v2_calibration(V2GateProfile::Smoke, smoke_calibration_rows()).unwrap();
    freeze.checksum_sha256 = digest("intentionally-corrupt-freeze");
    fs::write(&freeze_path, serde_json::to_vec_pretty(&freeze).unwrap()).unwrap();
    assert!(!missing_input.exists());

    let run = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-holdout-replay",
            "--profile",
            "smoke",
            "--freeze",
        ])
        .arg(&freeze_path)
        .arg("--input")
        .arg(&missing_input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("checksum") || stderr.contains("freeze"),
        "{stderr}"
    );
    assert!(!stderr.contains("No such file"), "{stderr}");
    let failure: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(failure["holdout_input_accessed"], false);
    assert_eq!(failure["rows"].as_array().unwrap().len(), 0);
    clean(&[&freeze_path, &output]);
}

#[test]
fn oregonator_producer_rejects_freeze_before_reference_or_holdout_access() {
    let freeze_path = temp_path("oregonator-producer-corrupt-freeze.json");
    let missing_campaign = temp_path("oregonator-producer-campaign-must-not-open.json");
    let missing_reference = temp_path("oregonator-producer-reference-must-not-open.json");
    let output = temp_path("oregonator-producer-output.json");
    clean(&[&freeze_path, &missing_campaign, &missing_reference, &output]);
    let mut freeze = freeze_v2_calibration(V2GateProfile::Smoke, smoke_calibration_rows()).unwrap();
    freeze.checksum_sha256 = digest("corrupt-oregonator-producer-freeze");
    fs::write(&freeze_path, serde_json::to_vec_pretty(&freeze).unwrap()).unwrap();

    let run = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-run-oregonator",
            "--profile",
            "canonical",
            "--freeze",
        ])
        .arg(&freeze_path)
        .arg("--calibration-campaign")
        .arg(&missing_campaign)
        .arg("--reference-manifest")
        .arg(&missing_reference)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(!stderr.contains("No such file"), "{stderr}");
    let failure: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(failure["holdout_spec_accessed"], false);
    assert_eq!(failure["calibration_campaign_accessed"], false);
    assert_eq!(failure["reference_manifest_accessed"], false);
    assert_eq!(failure["records"].as_array().unwrap().len(), 0);
    clean(&[&freeze_path, &output]);
}

#[test]
fn oregonator_producer_rejects_smoke_before_opening_freeze_or_reference() {
    let missing_freeze = temp_path("oregonator-smoke-freeze-must-not-open.json");
    let missing_campaign = temp_path("oregonator-smoke-campaign-must-not-open.json");
    let missing_reference = temp_path("oregonator-smoke-reference-must-not-open.json");
    let output = temp_path("oregonator-smoke-output.json");
    clean(&[&missing_freeze, &missing_reference, &output]);

    let run = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-run-oregonator",
            "--profile",
            "smoke",
            "--freeze",
        ])
        .arg(&missing_freeze)
        .arg("--calibration-campaign")
        .arg(&missing_campaign)
        .arg("--reference-manifest")
        .arg(&missing_reference)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("canonical 3-row profile"), "{stderr}");
    assert!(!stderr.contains("No such file"), "{stderr}");
    let failure: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(failure["freeze_accessed"], false);
    assert_eq!(failure["calibration_campaign_accessed"], false);
    assert_eq!(failure["holdout_spec_accessed"], false);
    assert_eq!(failure["reference_manifest_accessed"], false);
    clean(&[&output]);
}

#[test]
fn invalid_freeze_writes_failure_preserving_rows_then_returns_nonzero() {
    // Defect caught: a rejected calibration disappears without its typed input evidence.
    let input = temp_path("invalid-freeze-input.json");
    let output = temp_path("invalid-freeze-output.json");
    clean(&[&input, &output]);
    let mut incomplete = smoke_calibration_rows();
    incomplete[0].status = V2GateRowStatus::ReferenceDominated;
    incomplete[0].conservative_max_wrms = Some(42.0);
    incomplete.pop();
    fs::write(&input, serde_json::to_vec_pretty(&incomplete).unwrap()).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-freeze",
            "--profile",
            "smoke",
            "--input",
        ])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let failure: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(failure["status"], "fail");
    assert_eq!(failure["rows"].as_array().unwrap().len(), 5);
    assert_eq!(failure["rows"][0]["status"], "reference-dominated");
    assert!(failure["error"].as_str().unwrap().contains("calibration"));
    clean(&[&input, &output]);
}

#[test]
fn canonical_freeze_rejects_bare_unbound_rows_before_creating_authority() {
    let input = temp_path("canonical-bare-input.json");
    let output = temp_path("canonical-bare-output.json");
    clean(&[&input, &output]);
    fs::write(
        &input,
        br#"[{"case_id":"self-asserted","status":"pass","conservative_max_wrms":0.0}]"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-freeze",
            "--profile",
            "canonical",
            "--input",
        ])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!output.exists());
    clean(&[&input, &output]);
}

#[test]
fn canonical_raw_freeze_is_rejected_before_input_access() {
    let missing_input = temp_path("canonical-freeze-input-must-not-open.json");
    let output = temp_path("canonical-freeze-output-must-not-exist.json");
    clean(&[&missing_input, &output]);
    let result = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-freeze",
            "--profile",
            "canonical",
            "--input",
        ])
        .arg(&missing_input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("canonical raw-row freeze is disabled"),
        "{stderr}"
    );
    assert!(!stderr.contains("No such file"), "{stderr}");
    assert!(!output.exists());
}

#[test]
fn canonical_raw_holdout_replay_is_rejected_before_any_input_access() {
    let missing_freeze = temp_path("canonical-replay-freeze-must-not-open.json");
    let missing_input = temp_path("canonical-replay-input-must-not-open.json");
    let output = temp_path("canonical-replay-output-must-not-exist.json");
    clean(&[&missing_freeze, &missing_input, &output]);
    let result = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-holdout-replay",
            "--profile",
            "canonical",
            "--freeze",
        ])
        .arg(&missing_freeze)
        .arg("--input")
        .arg(&missing_input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("canonical raw-row holdout replay is disabled"),
        "{stderr}"
    );
    assert!(!stderr.contains("No such file"), "{stderr}");
    assert!(!output.exists());
}

#[test]
fn canonical_calibration_requires_a_distinct_direct_freeze_output() {
    let missing_reference = temp_path("canonical-calibration-reference-must-not-open.json");
    let output = temp_path("canonical-calibration-output-must-not-exist.json");
    clean(&[&missing_reference, &output]);
    let missing_freeze_argument = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-run-calibration",
            "--reference-manifest",
        ])
        .arg(&missing_reference)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!missing_freeze_argument.status.success());
    let stderr = String::from_utf8_lossy(&missing_freeze_argument.stderr);
    assert!(stderr.contains("--freeze-output"), "{stderr}");
    assert!(!stderr.contains("No such file"), "{stderr}");
    assert!(!output.exists());

    let aliased = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
        .args([
            "scientific-validity-v2-run-calibration",
            "--reference-manifest",
        ])
        .arg(&missing_reference)
        .arg("--output")
        .arg(&output)
        .arg("--freeze-output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!aliased.status.success());
    let stderr = String::from_utf8_lossy(&aliased.stderr);
    assert!(stderr.contains("distinct paths"), "{stderr}");
    assert!(!output.exists());
}

#[test]
fn checked_in_smoke_artifacts_verify_under_the_current_source_bound_protocol() {
    // Defect caught: checked-in wiring evidence remains stale after a gate schema change.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture_dir = root.join("research/scientific_validity_v2_20260829");
    let freeze: V2CalibrationFreezeEnvelope = serde_json::from_slice(
        &fs::read(fixture_dir.join("CI_SMOKE_FREEZE_FIXTURE.json")).unwrap(),
    )
    .unwrap();
    let replay: V2OregonatorReplayEnvelope = serde_json::from_slice(
        &fs::read(fixture_dir.join("CI_SMOKE_OREGONATOR_REPLAY_FIXTURE.json")).unwrap(),
    )
    .unwrap();

    verify_v2_calibration_freeze(&freeze).unwrap();
    verify_v2_oregonator_replay(&replay, &freeze).unwrap();
    assert_eq!(
        freeze.payload.campaign_binding.authority,
        V2EvidenceAuthority::SyntheticCiSmoke
    );
    assert!(replay.payload.overall_pass);
}
