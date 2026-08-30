#![cfg(feature = "audit2-bateman-authority")]

use rodas5p_core::CoreResult;
use rodas5p_integrators::{
    AUDIT2_BATEMAN_AUTHORITY_MANIFEST_SHA256, AUDIT2_BATEMAN_AUTHORITY_PROOF_SHA256,
    AUDIT2_BATEMAN_AUTHORITY_VERIFIER_SHA256, AUDIT2_BATEMAN_CHANGED_W_CASE_ID,
    AUDIT2_BATEMAN_CLIENT_ID, AUDIT2_BATEMAN_NOMINAL_CASE_ID, AUDIT2_BATEMAN_SCENARIO_IDS,
    Audit2BatemanRealClientManifest, Audit2BatemanScenarioKind,
    Audit2ReferenceUncertaintyTreatment, admit_audit2_bateman_real_client_authority,
    audit2_bateman_real_client_manifest, audit2_bateman_six_case_plan,
    audit2_bateman_verify_runtime_operator_bindings_candidate_free,
};

const MANIFEST_BYTES: &[u8] = include_bytes!(
    "../../../research/audit2_real_client_authority_construction_20260830/authority_manifest.json"
);
const VERIFIER_BYTES: &[u8] = include_bytes!(
    "../../../research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py"
);
const PROOF_BYTES: &[u8] = include_bytes!(
    "../../../research/audit2_real_client_authority_construction_20260830/evidence/AUTHORITY_VERIFICATION_RECEIPT.json"
);

#[test]
fn exact_manifest_verifier_and_proof_bytes_admit_an_opaque_candidate_free_authority()
-> CoreResult<()> {
    // Breaks if admission can skip the independently checked manifest, exact
    // verifier, or candidate-free proof receipt.
    let checked_in: Audit2BatemanRealClientManifest =
        serde_json::from_slice(MANIFEST_BYTES).expect("checked-in authority manifest JSON");
    assert_eq!(checked_in, audit2_bateman_real_client_manifest()?);

    let authority =
        admit_audit2_bateman_real_client_authority(MANIFEST_BYTES, VERIFIER_BYTES, PROOF_BYTES)?;
    assert_eq!(authority.manifest().client_id, AUDIT2_BATEMAN_CLIENT_ID);
    assert_eq!(authority.manifest().operator_cases.len(), 2);
    assert_eq!(
        authority.manifest().execution_scenarios,
        AUDIT2_BATEMAN_SCENARIO_IDS
    );
    assert_eq!(
        authority.manifest().holdout_access,
        "NOT_OPENED_OR_EXECUTED"
    );
    assert_eq!(
        authority.manifest_sha256(),
        AUDIT2_BATEMAN_AUTHORITY_MANIFEST_SHA256
    );
    assert_eq!(
        authority.verifier_sha256(),
        AUDIT2_BATEMAN_AUTHORITY_VERIFIER_SHA256
    );
    assert_eq!(
        authority.proof_sha256(),
        AUDIT2_BATEMAN_AUTHORITY_PROOF_SHA256
    );
    for case in &authority.manifest().operator_cases {
        assert_eq!(
            case.reference.uncertainty_treatment,
            Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound
        );
        assert_eq!(
            case.reference.uncertainty_l2.to_bits(),
            1.0e-15f64.to_bits()
        );
        assert!(case.budget.output_atol_l2 > case.reference.uncertainty_l2);
    }
    Ok(())
}

#[test]
fn admitted_cases_rebind_to_exact_runtime_contexts_without_running_a_candidate() -> CoreResult<()> {
    let authority =
        admit_audit2_bateman_real_client_authority(MANIFEST_BYTES, VERIFIER_BYTES, PROOF_BYTES)?;
    let receipts = audit2_bateman_verify_runtime_operator_bindings_candidate_free(&authority)?;
    assert_eq!(receipts.len(), 2);
    assert_eq!(receipts[0].case_id, AUDIT2_BATEMAN_NOMINAL_CASE_ID);
    assert_eq!(receipts[1].case_id, AUDIT2_BATEMAN_CHANGED_W_CASE_ID);
    for (receipt, expected) in receipts.iter().zip(&authority.manifest().operator_cases) {
        assert_eq!(receipt.frozen_w_sha256, expected.frozen_w_semantic.sha256);
        assert_eq!(
            receipt.inverse_diagonal_bits,
            expected
                .preconditioner_identity
                .expected_inverse_diagonal_bits
        );
        assert_eq!(receipt.dimension, 4);
        assert_eq!(receipt.rhs_calls, 1);
        assert_eq!(receipt.candidate_executions, 0);
    }
    Ok(())
}

#[test]
fn one_byte_tamper_in_any_admission_artifact_fails_before_candidate_or_client_access() {
    // Breaks if only decoded JSON values are checked, allowing byte-level drift
    // in one of the frozen handoff artifacts.
    let mut manifest = MANIFEST_BYTES.to_vec();
    manifest[0] ^= 1;
    assert!(
        admit_audit2_bateman_real_client_authority(&manifest, VERIFIER_BYTES, PROOF_BYTES)
            .unwrap_err()
            .to_string()
            .contains("exact checked-in")
    );

    let mut verifier = VERIFIER_BYTES.to_vec();
    verifier[0] ^= 1;
    assert!(
        admit_audit2_bateman_real_client_authority(MANIFEST_BYTES, &verifier, PROOF_BYTES).is_err()
    );

    let mut proof = PROOF_BYTES.to_vec();
    proof[0] ^= 1;
    assert!(
        admit_audit2_bateman_real_client_authority(MANIFEST_BYTES, VERIFIER_BYTES, &proof).is_err()
    );
}

#[test]
fn frozen_orchestration_plan_has_six_ordered_scenarios_and_no_caller_knobs() {
    // This inspects orchestration metadata only. It deliberately does not call
    // the consuming local runner or generate any candidate output.
    use Audit2BatemanScenarioKind::{
        ChangedWCacheProbe, SameLiveContextCacheProbe, TransactionalLateApplyFailure,
        TransactionalNominal, TransactionalStrictFallback, TransactionalTerminalRejection,
    };
    let plan = audit2_bateman_six_case_plan();
    assert_eq!(plan.len(), 6);
    assert_eq!(
        plan.iter()
            .map(|row| row.scenario_id.as_str())
            .collect::<Vec<_>>(),
        AUDIT2_BATEMAN_SCENARIO_IDS
    );
    assert_eq!(
        plan.iter().map(|row| row.kind).collect::<Vec<_>>(),
        [
            SameLiveContextCacheProbe,
            ChangedWCacheProbe,
            TransactionalNominal,
            TransactionalStrictFallback,
            TransactionalLateApplyFailure,
            TransactionalTerminalRejection,
        ]
    );
    assert_eq!(plan[0].operator_case_id, AUDIT2_BATEMAN_NOMINAL_CASE_ID);
    assert_eq!(plan[1].operator_case_id, AUDIT2_BATEMAN_CHANGED_W_CASE_ID);
    assert!(
        plan[2..]
            .iter()
            .all(|row| row.operator_case_id == AUDIT2_BATEMAN_NOMINAL_CASE_ID)
    );
    assert_eq!(
        serde_json::to_value(&plan)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        6
    );
}

#[test]
fn local_example_loads_all_three_frozen_artifacts_and_only_the_consuming_runner() {
    // Candidate-free static contract: compilation checks the executable path;
    // this assertion keeps its only public inputs tied to checked-in bytes.
    let source = include_str!("../examples/audit2_bateman_local_six_case.rs");
    assert!(source.contains("authority_manifest.json"));
    assert!(source.contains("verify_authority_manifest.py"));
    assert!(source.contains("AUTHORITY_VERIFICATION_RECEIPT.json"));
    assert!(source.contains("run_audit2_bateman_local_six_case_suite(authority)"));
    assert!(source.contains("report.all_six_executed"));
    assert!(source.contains("report.all_contracts_satisfied"));
    assert!(source.contains("std::io::Error::other"));
    assert!(!source.contains("std::env::args"));

    let prompt = include_str!(
        "../../../research/audit2_real_client_authority_construction_20260830/CODEX_START_HERE.md"
    );
    let handoff = include_str!(
        "../../../research/audit2_real_client_authority_construction_20260830/handoff.json"
    );
    let exact_command =
        "cargo run --locked -p rodas5p-integrators --features audit2-bateman-authority";
    assert!(prompt.contains(exact_command));
    assert!(handoff.contains(exact_command));
}

#[test]
fn runner_receipts_cannot_label_mismatches_as_observed_or_hide_the_candidate_step() {
    // Candidate-free static contract for the two fresh runner-review P2s.
    let source = include_str!("../src/audit2_bateman_real_client_research.rs");
    assert!(source.contains("ContractMismatch"));
    assert!(source.contains("candidate_step"));
    assert!(source.contains("candidate.step.accepted && !candidate.budget.accepted"));
    assert!(source.contains("pub cache: Option<Audit2ReusablePreconditionerCacheSnapshot>"));
}
