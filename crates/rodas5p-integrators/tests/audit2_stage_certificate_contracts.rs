#![cfg(feature = "audit2-stage-certificate")]

use rodas5p_integrators::{
    Audit2StageCertificateError, FrozenJsonDocument, StageCertificateDecision,
    StageCertificateInput, StageCertificateNorm, StageCertificateProvenance,
    StageCertificateReceipt, StageCertificateReceiptAuthority, StageCertificateStageSolve,
    StageCertificateStageTrace, StageCertificateTrace, StageCertificateWork,
    audit2_stage_certificate_decision_input_digest, audit2_stage_certificate_digest_f64_bits,
    audit2_stage_certificate_rhs_digest, audit2_upper_add, audit2_upper_mul, canonical_json_sha256,
    evaluate_audit2_stage_certificate, retain_completed_stage_traces,
};
use serde_json::json;

fn digest_matrix(matrix: &[Vec<f64>]) -> String {
    audit2_stage_certificate_digest_f64_bits(
        &matrix
            .iter()
            .flatten()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
    )
}

fn frozen(document: serde_json::Value) -> FrozenJsonDocument {
    let canonical_sha256 = canonical_json_sha256(&document).unwrap();
    FrozenJsonDocument {
        document,
        canonical_sha256,
    }
}

/// Recompute every provenance digest from the current input.
fn seal(mut candidate: StageCertificateInput) -> StageCertificateInput {
    candidate.provenance.coefficient_digest = audit2_stage_certificate_digest_f64_bits(
        &candidate
            .coefficients
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
    );
    candidate.provenance.operator_identity = digest_matrix(&candidate.operator);
    candidate.provenance.preconditioner_identity = digest_matrix(&candidate.preconditioner);
    candidate.provenance.rhs_digest = audit2_stage_certificate_rhs_digest(&candidate.stages);
    candidate.provenance.decision_input_digest =
        audit2_stage_certificate_decision_input_digest(&candidate);
    candidate
}

fn stage_trace(stage_index: usize, iterations: u64) -> StageCertificateStageTrace {
    StageCertificateStageTrace {
        stage_index,
        residual_history: (0..=iterations).map(|k| 2.0 / (k as f64 + 1.0)).collect(),
        complete: true,
        work: StageCertificateWork {
            operator_applies: iterations,
            preconditioner_applies: iterations,
            arnoldi_iterations: iterations,
        },
    }
}

fn trace(iterations: &[u64]) -> StageCertificateTrace {
    let stage_traces: Vec<_> = iterations
        .iter()
        .enumerate()
        .map(|(index, count)| stage_trace(index, *count))
        .collect();
    let total: u64 = iterations.iter().sum();
    StageCertificateTrace {
        frozen_trace: frozen(json!({"trace":"complete", "stage_count": iterations.len()})),
        stage_traces,
        completed_work: StageCertificateWork {
            operator_applies: total,
            preconditioner_applies: total,
            arnoldi_iterations: total,
        },
        partial_failure: None,
    }
}

fn stage(rhs: [f64; 2], solution: [f64; 2], caller_upper: f64) -> StageCertificateStageSolve {
    StageCertificateStageSolve {
        rhs: rhs.to_vec(),
        approximate_solution: solution.to_vec(),
        caller_product_upper: caller_upper,
        caller_q_upper: caller_upper,
    }
}

/// Two stages of a two-dimensional synthetic system `W = diag(2, 4)`.
///
/// `||W^{-1}||_2 = 0.5`; the evaluator verifies a bound from the witness
/// `V = diag(1/2, 1/4)`, so `kappa = 0.75` is a true premise.
fn input() -> StageCertificateInput {
    seal(StageCertificateInput {
        frozen_plan: frozen(json!({"plan":"strict-lower-v1", "stages":2})),
        trace: trace(&[2, 2]),
        provenance: StageCertificateProvenance {
            coefficient_digest: String::new(),
            operator_identity: String::new(),
            preconditioner_identity: String::new(),
            rhs_digest: String::new(),
            decision_input_digest: String::new(),
            restart: 2,
            max_arnoldi: 4,
            iteration_limit: 8,
        },
        norm: StageCertificateNorm::canonical(2),
        coefficients: vec![1.0, 0.5],
        operator: vec![vec![2.0, 0.0], vec![0.0, 4.0]],
        preconditioner: vec![vec![0.5, 0.0], vec![0.0, 0.25]],
        inverse_witness: vec![vec![0.5, 0.0], vec![0.0, 0.25]],
        stages: vec![
            stage([0.0, 0.0], [0.0, 0.0], 0.0),
            stage([0.0, 0.0], [0.0, 0.0], 0.0),
        ],
        kappa_upper: 0.75,
        strict_lower: vec![vec![0.0, 0.0], vec![0.25, 0.0]],
        endpoint_weights: vec![1.0, 0.0],
        estimator_weights: vec![0.0, 1.0],
        ehat: 0.1,
    })
}

/// Nonzero stage residuals with zero weights: a safe accept.
fn accept_input() -> StageCertificateInput {
    let mut candidate = input();
    candidate.stages = vec![
        stage([2.0, 8.0], [0.0, 0.0], 7.0),
        stage([1.0, -2.0], [0.25, 0.0], 7.0),
    ];
    candidate.endpoint_weights = vec![0.0, 0.0];
    candidate.estimator_weights = vec![0.0, 0.0];
    seal(candidate)
}

#[test]
fn strict_lower_fixture_recomputes_a_safe_accept() {
    let receipt = evaluate_audit2_stage_certificate(accept_input()).unwrap();
    assert_eq!(
        receipt.decision,
        StageCertificateDecision::SyntheticConsistentAccept
    );
    assert_eq!(
        receipt.authority,
        StageCertificateReceiptAuthority::SyntheticSchemaConsistencyOnly
    );
    assert!(receipt.stage_majorant[0] > 0.0);
    // z_1 = q_1 + T_10 z_0 with upward rounding. The former `z_1 > z_0`
    // held only because one q was copied to every stage (F-060).
    assert_eq!(receipt.stage_majorant[0], receipt.q_upper[0]);
    assert!(receipt.stage_majorant[1] >= receipt.q_upper[1] + 0.25 * receipt.stage_majorant[0]);
    assert!(receipt.stage_majorant[1] > receipt.q_upper[1]);
    assert_eq!(receipt.endpoint_contamination, 0.0);
    assert_eq!(receipt.estimator_contamination, 0.0);
    // WU-9 (F-101): 0.1 + 0 is exact and is no longer bumped by one ulp.
    assert_eq!(receipt.ehat_plus_theta.to_bits(), 0.1_f64.to_bits());
    // The verified inverse bound lies at or above the true ||W^-1||_2 = 0.5.
    assert!(receipt.inverse_norm_upper >= 0.5 && receipt.inverse_norm_upper <= 0.75);
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
    let mut product = accept_input();
    product.stages[0].caller_product_upper = 0.0;
    // Residual is nonzero, so zero is a downward caller product bound.
    assert!(matches!(
        evaluate_audit2_stage_certificate(seal(product)),
        Err(Audit2StageCertificateError::DownwardRoundedBound)
    ));

    let mut sum = accept_input();
    sum.stages[1].caller_q_upper = 0.0;
    assert!(matches!(
        evaluate_audit2_stage_certificate(seal(sum)),
        Err(Audit2StageCertificateError::DownwardRoundedBound)
    ));
}

#[test]
fn positive_underflowing_product_is_rounded_up_not_to_zero() {
    let mut product = input();
    product.stages[0].rhs = vec![f64::from_bits(1), 0.0];
    assert!(matches!(
        evaluate_audit2_stage_certificate(seal(product)),
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
    let mut rhs = input();
    rhs.stages[1].rhs[0] = 1.0;
    assert!(matches!(
        evaluate_audit2_stage_certificate(rhs),
        Err(Audit2StageCertificateError::ProvenanceMismatch)
    ));
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
    let receipt = evaluate_audit2_stage_certificate(seal(reject)).unwrap();
    assert_eq!(
        receipt.decision,
        StageCertificateDecision::SyntheticConsistentReject
    );
    // theta > 0 straddles the threshold: neither a safe accept nor a reject.
    let mut indeterminate = accept_input();
    indeterminate.endpoint_weights = vec![0.01, 0.0];
    indeterminate.ehat = 1.0;
    assert!(matches!(
        evaluate_audit2_stage_certificate(seal(indeterminate)),
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

// ---- WU-9 contracts (audit F-057, F-060, F-102, F-101, F-104, F-059, F-064) ----

#[test]
fn max_arnoldi_equality_is_legal() {
    // MAX_ARNOLDI_EQUALITY: max_arnoldi = 4 < iteration_limit = 8.
    let mut candidate = input();
    candidate.trace = trace(&[2, 4]);
    assert!(evaluate_audit2_stage_certificate(candidate).is_ok());
}

#[test]
fn max_arnoldi_exceeded_on_one_row_rejects_below_the_iteration_limit() {
    // F-057 MAX_ARNOLDI_EXCEEDED / MULTI_ROW_ONE_EXCEEDS: row 1 uses
    // max_arnoldi + 1 = 5 Arnoldi vectors, still below iteration_limit = 8.
    let mut candidate = input();
    candidate.trace = trace(&[2, 5]);
    assert!(matches!(
        evaluate_audit2_stage_certificate(candidate),
        Err(Audit2StageCertificateError::MaxArnoldiExceeded)
    ));
}

#[test]
fn restart_length_is_not_the_total_arnoldi_cap() {
    // RESTART_NOT_TOTAL_CAP: restart = 2 is per cycle; 4 vectors are legal.
    let mut candidate = input();
    candidate.trace = trace(&[4, 3]);
    assert!(evaluate_audit2_stage_certificate(candidate).is_ok());
}

#[test]
fn receipt_survives_a_serde_json_round_trip_structurally_and_canonically() {
    // F-102 RECEIPT_JSON_ROUNDTRIP on a receipt with nonzero upward bounds.
    let receipt = evaluate_audit2_stage_certificate(accept_input()).unwrap();
    let bytes = serde_json::to_vec(&receipt).unwrap();
    let back: StageCertificateReceipt = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(back, receipt);
    let canonical = canonical_json_sha256(&serde_json::to_value(&receipt).unwrap()).unwrap();
    let reparsed = canonical_json_sha256(&serde_json::to_value(&back).unwrap()).unwrap();
    assert_eq!(canonical, reparsed);
    assert_eq!(serde_json::to_vec(&back).unwrap(), bytes);
    for (left, right) in back.q_upper.iter().zip(&receipt.q_upper) {
        assert_eq!(left.to_bits(), right.to_bits());
    }
}

#[test]
fn an_exact_upper_sum_is_not_bumped_by_one_ulp() {
    // F-101 ROUNDING_EXACT_REPRESENTABLE through the evaluator.
    let receipt = evaluate_audit2_stage_certificate(input()).unwrap();
    assert_eq!(receipt.theta, 0.0);
    assert_eq!(receipt.ehat_plus_theta.to_bits(), 0.1_f64.to_bits());
}

#[test]
fn directed_add_and_multiply_are_correctly_rounded_upward() {
    // ROUNDING_EXACT_ZERO
    assert_eq!(audit2_upper_add(0.0, 0.0).unwrap(), 0.0);
    assert_eq!(audit2_upper_mul(0.0, 3.0).unwrap(), 0.0);
    assert_eq!(
        audit2_upper_add(0.0, 0.3).unwrap().to_bits(),
        0.3_f64.to_bits()
    );
    // ROUNDING_EXACT_REPRESENTABLE
    assert_eq!(audit2_upper_add(0.5, 0.25).unwrap(), 0.75);
    assert_eq!(audit2_upper_mul(0.5, 0.25).unwrap(), 0.125);
    // ROUNDING_INEXACT_UPWARD: 1 + 2^-60 and (1 + 2^-30)^2 round down to
    // nearest, so the upper value is the next binary64 above it.
    let tiny = 2.0_f64.powi(-60);
    assert_eq!(audit2_upper_add(1.0, tiny).unwrap(), 1.0_f64.next_up());
    let factor = 1.0 + 2.0_f64.powi(-30);
    let nearest = factor * factor;
    assert_eq!(audit2_upper_mul(factor, factor).unwrap(), nearest.next_up());
    // 0.1 + 0.2 rounds up to nearest already, so it is an upper value.
    let rounded_up: f64 = 0.1 + 0.2;
    assert_eq!(
        audit2_upper_add(0.1, 0.2).unwrap().to_bits(),
        rounded_up.to_bits()
    );
}

#[test]
fn overflowing_bounds_are_typed_rejections_without_a_decision() {
    // PRODUCT_OVERFLOW_REJECTED / SUM_OVERFLOW_REJECTED (F-104).
    assert_eq!(
        audit2_upper_mul(1.0e200, 1.0e200),
        Err(Audit2StageCertificateError::NonFiniteBound)
    );
    assert_eq!(
        audit2_upper_add(f64::MAX, f64::MAX),
        Err(Audit2StageCertificateError::NonFiniteBound)
    );
    let mut candidate = accept_input();
    candidate.kappa_upper = 1.0e300;
    candidate.stages[0].rhs = vec![1.0e10, 0.0];
    candidate.stages[0].caller_product_upper = f64::MAX;
    candidate.stages[0].caller_q_upper = f64::MAX;
    assert_eq!(
        evaluate_audit2_stage_certificate(seal(candidate)),
        Err(Audit2StageCertificateError::NonFiniteBound)
    );
}

#[test]
fn a_kappa_that_does_not_bound_the_inverse_is_rejected() {
    // F-059: ||W^-1||_2 = 0.5 for W = diag(2, 4); kappa = 0.25 is false.
    let mut candidate = accept_input();
    candidate.kappa_upper = 0.25;
    assert!(matches!(
        evaluate_audit2_stage_certificate(seal(candidate)),
        Err(Audit2StageCertificateError::KappaPremiseNotVerified)
    ));
    // A witness that is not an approximate inverse cannot verify any kappa.
    let mut witness = accept_input();
    witness.inverse_witness = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
    assert!(matches!(
        evaluate_audit2_stage_certificate(seal(witness)),
        Err(Audit2StageCertificateError::KappaPremiseNotVerified)
    ));
}

#[test]
fn stage_count_and_state_dimension_are_independent() {
    // F-060: three stages of a two-dimensional system, each with its own
    // residual bound; nothing is broadcast from one solve.
    let mut candidate = accept_input();
    candidate.stages.push(stage([0.0, 4.0], [0.0, 1.0], 7.0));
    candidate.trace = trace(&[2, 2, 3]);
    candidate.frozen_plan = frozen(json!({"plan":"strict-lower-v1", "stages":3}));
    candidate.strict_lower = vec![
        vec![0.0, 0.0, 0.0],
        vec![0.25, 0.0, 0.0],
        vec![0.1, 0.2, 0.0],
    ];
    candidate.endpoint_weights = vec![0.0; 3];
    candidate.estimator_weights = vec![0.0; 3];
    let receipt = evaluate_audit2_stage_certificate(seal(candidate)).unwrap();
    assert_eq!(receipt.q_upper.len(), 3);
    assert_eq!(receipt.stage_majorant.len(), 3);
    // Stage 2 solves its system exactly; its rigorous bound is only the
    // rounding allowance gamma_3 * (|b| + |W||x|).
    assert!(receipt.residual_l2_upper[2] > 0.0);
    assert!(receipt.residual_l2_upper[2] <= 1.0e-14);
    assert!(receipt.residual_l2_upper[0] > receipt.residual_l2_upper[1]);
    assert_ne!(receipt.q_upper[0].to_bits(), receipt.q_upper[1].to_bits());
    // A stage count that disagrees with the trace is rejected.
    let mut mismatch = accept_input();
    mismatch.trace = trace(&[2, 2, 2]);
    assert!(evaluate_audit2_stage_certificate(mismatch).is_err());
}

#[test]
fn a_residual_bound_dominates_the_exact_residual() {
    // F-061: the residual and its norm are upper bounds, not nearest values.
    let mut candidate = accept_input();
    candidate.stages[1] = stage([1.0, 1.0], [0.1, 0.3], 7.0);
    let receipt = evaluate_audit2_stage_certificate(seal(candidate)).unwrap();
    // b - W x with the binary64 inputs 0.1 and 0.3; 2 * 0.1 and 4 * 0.3 are exact.
    let exact = ((1.0_f64 - 2.0 * 0.1).powi(2) + (1.0_f64 - 4.0 * 0.3).powi(2)).sqrt();
    assert!(receipt.residual_l2_upper[1] >= exact);
    assert!(receipt.residual_l2_upper[1] <= exact * (1.0 + 1.0e-12));
}

#[test]
fn decision_driving_inputs_are_bound_by_the_provenance_digest() {
    // F-064: T, weights, kappa, ehat, x and the witness drive the decision.
    let tamper: [fn(&mut StageCertificateInput); 7] = [
        |c| c.strict_lower[1][0] = 0.5,
        |c| c.endpoint_weights[0] = 0.5,
        |c| c.estimator_weights[1] = 0.5,
        |c| c.kappa_upper = 0.8,
        |c| c.ehat = 0.2,
        |c| c.stages[0].approximate_solution[0] = 0.5,
        |c| c.inverse_witness[0][0] = 0.4,
    ];
    for (index, change) in tamper.iter().enumerate() {
        let mut candidate = input();
        change(&mut candidate);
        assert!(
            matches!(
                evaluate_audit2_stage_certificate(candidate),
                Err(Audit2StageCertificateError::ProvenanceMismatch)
            ),
            "tamper {index} was not detected as a provenance mismatch"
        );
    }
}
