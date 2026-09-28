#![cfg(feature = "audit2-stage-certificate")]

use rodas5p_integrators::{
    Audit2StageCertificateError, FrozenJsonDocument, StageCertificateDecision,
    StageCertificateInput, StageCertificateNorm, StageCertificateProvenance,
    StageCertificateReceiptAuthority, StageCertificateStageTrace, StageCertificateTrace,
    StageCertificateWork, audit2_stage_certificate_digest_f64_bits, canonical_json_sha256,
    evaluate_audit2_stage_certificate, retain_completed_stage_traces,
};
use serde_json::json;

fn digest_bits(bits: &[u64]) -> String {
    audit2_stage_certificate_digest_f64_bits(bits)
}

fn frozen(document: serde_json::Value) -> FrozenJsonDocument {
    let canonical_sha256 = canonical_json_sha256(&document).unwrap();
    FrozenJsonDocument {
        document,
        canonical_sha256,
    }
}

fn input() -> StageCertificateInput {
    let coefficients: Vec<f64> = vec![1.0, 0.5];
    let w: Vec<Vec<f64>> = vec![vec![2.0, 0.0], vec![0.0, 4.0]];
    let preconditioner: Vec<Vec<f64>> = vec![vec![0.5, 0.0], vec![0.0, 0.25]];
    let rhs: Vec<f64> = vec![0.0, 0.0];
    let coefficient_bits: Vec<u64> = coefficients.iter().map(|value| value.to_bits()).collect();
    let w_bits: Vec<u64> = w.iter().flatten().map(|value| value.to_bits()).collect();
    let p_bits: Vec<u64> = preconditioner
        .iter()
        .flatten()
        .map(|value| value.to_bits())
        .collect();
    let rhs_bits: Vec<u64> = rhs.iter().map(|value| value.to_bits()).collect();
    let provenance = StageCertificateProvenance {
        coefficient_digest: digest_bits(&coefficient_bits),
        operator_identity: digest_bits(&w_bits),
        preconditioner_identity: digest_bits(&p_bits),
        rhs_digest: digest_bits(&rhs_bits),
        restart: 2,
        max_arnoldi: 4,
        iteration_limit: 8,
    };
    let trace = StageCertificateTrace {
        frozen_trace: frozen(json!({"trace":"complete", "stage_count":2})),
        stage_traces: vec![
            StageCertificateStageTrace {
                stage_index: 0,
                residual_history: vec![2.0, 1.0, 0.5],
                complete: true,
                work: StageCertificateWork {
                    operator_applies: 2,
                    preconditioner_applies: 2,
                    arnoldi_iterations: 2,
                },
            },
            StageCertificateStageTrace {
                stage_index: 1,
                residual_history: vec![0.5, 0.25, 0.0],
                complete: true,
                work: StageCertificateWork {
                    operator_applies: 2,
                    preconditioner_applies: 2,
                    arnoldi_iterations: 2,
                },
            },
        ],
        completed_work: StageCertificateWork {
            operator_applies: 4,
            preconditioner_applies: 4,
            arnoldi_iterations: 4,
        },
        partial_failure: None,
    };
    StageCertificateInput {
        frozen_plan: frozen(json!({"plan":"strict-lower-v1", "stages":2})),
        trace,
        provenance,
        norm: StageCertificateNorm::canonical(2),
        coefficients,
        operator: w,
        preconditioner,
        rhs,
        approximate_solution: vec![0.0, 0.0],
        kappa_upper: 0.25,
        caller_product_upper: 0.0,
        caller_q_upper: 0.0,
        strict_lower: vec![vec![0.0, 0.0], vec![0.25, 0.0]],
        endpoint_weights: vec![1.0, 0.0],
        estimator_weights: vec![0.0, 1.0],
        ehat: 0.1,
    }
}

#[test]
fn strict_lower_fixture_recomputes_a_safe_accept() {
    let mut candidate = input();
    candidate.rhs = vec![2.0, 8.0];
    candidate.provenance.rhs_digest = digest_bits(
        &candidate
            .rhs
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
    );
    candidate.caller_product_upper = 3.0;
    candidate.caller_q_upper = 3.0;
    candidate.endpoint_weights = vec![0.0, 0.0];
    candidate.estimator_weights = vec![0.0, 0.0];
    let receipt = evaluate_audit2_stage_certificate(candidate).unwrap();
    assert_eq!(
        receipt.decision,
        StageCertificateDecision::SyntheticConsistentAccept
    );
    assert_eq!(
        receipt.authority,
        StageCertificateReceiptAuthority::SyntheticSchemaConsistencyOnly
    );
    assert!(receipt.stage_majorant[0] > 0.0);
    assert!(receipt.stage_majorant[1] > receipt.stage_majorant[0]);
    assert_eq!(receipt.endpoint_contamination, 0.0);
    assert_eq!(receipt.estimator_contamination, 0.0);
    assert_eq!(
        receipt.ehat_plus_theta,
        f64::from_bits(0.1_f64.to_bits() + 1)
    );
}

#[test]
fn canonical_plan_or_trace_hash_tampering_is_rejected() {
    let mut plan = input();
    plan.frozen_plan.canonical_sha256.replace_range(..1, "0");
    assert!(evaluate_audit2_stage_certificate(plan).is_err());
    let mut trace = input();
    trace.trace.frozen_trace.document = json!({"trace":"tampered"});
    assert!(evaluate_audit2_stage_certificate(trace).is_err());
}

#[test]
fn missing_nonfinite_negative_and_inconsistent_fields_fail_closed() {
    let mut missing = input();
    missing.trace.stage_traces.clear();
    assert!(evaluate_audit2_stage_certificate(missing).is_err());
    let mut nonfinite = input();
    nonfinite.kappa_upper = f64::NAN;
    assert!(evaluate_audit2_stage_certificate(nonfinite).is_err());
    let mut negative = input();
    negative.ehat = -1.0;
    assert!(evaluate_audit2_stage_certificate(negative).is_err());
    let mut inconsistent = input();
    inconsistent.trace.completed_work.operator_applies = 3;
    assert!(evaluate_audit2_stage_certificate(inconsistent).is_err());
}

#[test]
fn non_unit_norm_scale_is_rejected() {
    let mut candidate = input();
    candidate.norm.scale_bits[0] = 0.5_f64.to_bits();
    assert!(evaluate_audit2_stage_certificate(candidate).is_err());
}

#[test]
fn norm_scale_cardinality_must_match_the_synthetic_dimension() {
    let mut candidate = input();
    candidate.norm.scale_bits.pop();
    assert!(evaluate_audit2_stage_certificate(candidate).is_err());
}

#[test]
fn downward_rounded_product_and_sum_are_rejected() {
    let mut product = input();
    product.rhs = vec![2.0, 8.0];
    product.provenance.rhs_digest = digest_bits(
        &product
            .rhs
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
    );
    product.caller_product_upper = 0.0;
    product.caller_q_upper = 0.0;
    // Residual is nonzero, so zero is a downward caller product bound.
    assert!(evaluate_audit2_stage_certificate(product).is_err());

    let mut sum = input();
    sum.rhs = vec![2.0, 8.0];
    sum.provenance.rhs_digest = digest_bits(
        &sum.rhs
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
    );
    sum.caller_product_upper = 3.0;
    sum.caller_q_upper = 0.0;
    assert!(evaluate_audit2_stage_certificate(sum).is_err());
}

#[test]
fn positive_underflowing_product_is_rounded_up_not_to_zero() {
    let mut product = input();
    product.rhs = vec![f64::from_bits(1), 0.0];
    product.provenance.rhs_digest = digest_bits(
        &product
            .rhs
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
    );
    product.kappa_upper = 0.5;
    product.caller_product_upper = 0.0;
    product.caller_q_upper = 0.0;
    assert!(matches!(
        evaluate_audit2_stage_certificate(product),
        Err(Audit2StageCertificateError::DownwardRoundedBound)
    ));
}

#[test]
fn coefficient_bits_and_operator_preconditioner_bindings_are_verified() {
    let mut coefficient = input();
    coefficient.coefficients[0] = 1.25;
    assert!(evaluate_audit2_stage_certificate(coefficient).is_err());
    let mut operator = input();
    operator.operator[0][0] = 3.0;
    assert!(evaluate_audit2_stage_certificate(operator).is_err());
    let mut preconditioner = input();
    preconditioner.preconditioner[0][0] = 0.25;
    assert!(evaluate_audit2_stage_certificate(preconditioner).is_err());
}

#[test]
fn incomplete_or_final_only_residual_history_has_no_decision() {
    let mut incomplete = input();
    incomplete.trace.stage_traces[1].complete = false;
    assert!(matches!(
        evaluate_audit2_stage_certificate(incomplete),
        Err(Audit2StageCertificateError::IncompleteResidualHistory)
    ));
    let mut final_only = input();
    final_only.trace.stage_traces[0].residual_history = vec![0.5];
    assert!(matches!(
        evaluate_audit2_stage_certificate(final_only),
        Err(Audit2StageCertificateError::IncompleteResidualHistory)
    ));
}

#[test]
fn safe_reject_is_recomputed_and_an_indeterminate_interval_is_not_admitted() {
    let mut reject = input();
    reject.ehat = 1.1;
    let receipt = evaluate_audit2_stage_certificate(reject).unwrap();
    assert_eq!(
        receipt.decision,
        StageCertificateDecision::SyntheticConsistentReject
    );
    let mut indeterminate = input();
    indeterminate.ehat = 1.0;
    assert!(matches!(
        evaluate_audit2_stage_certificate(indeterminate),
        Err(Audit2StageCertificateError::InconclusiveInterval)
    ));
}

#[test]
fn late_failure_retains_only_completed_stage_traces_and_work() {
    let original = input().trace;
    let partial = retain_completed_stage_traces(
        original.frozen_trace,
        original.stage_traces,
        Some((1, "injected late failure".into())),
    )
    .unwrap();
    assert_eq!(partial.stage_traces.len(), 1);
    assert_eq!(partial.completed_work.operator_applies, 2);
    assert_eq!(partial.partial_failure.unwrap().stage_index, 1);
}

#[test]
fn receipt_readback_preserves_plan_trace_hashes_and_provenance() {
    let original = input();
    let receipt = evaluate_audit2_stage_certificate(original.clone()).unwrap();
    assert_eq!(receipt.frozen_plan, original.frozen_plan);
    assert_eq!(receipt.trace, original.trace);
    assert_eq!(receipt.provenance, original.provenance);
    assert_eq!(
        canonical_json_sha256(&receipt.frozen_plan.document).unwrap(),
        receipt.frozen_plan.canonical_sha256
    );
}

#[test]
fn isolated_feature_and_source_do_not_reference_prohibited_execution_surfaces() {
    let manifest = include_str!("../Cargo.toml");
    let source = include_str!("../src/audit2_stage_certificate_research.rs");
    assert!(manifest.contains("audit2-stage-certificate = [\"audit2-research\""));
    assert!(!manifest.contains("audit2-stage-certificate = [\"audit2-bateman-authority\""));
    assert!(!source.contains(concat!("audit2", "_bateman", "_real_client")));
    assert!(!source.contains(concat!("hold", "out")));
    assert!(!source.contains("run_"));
}
