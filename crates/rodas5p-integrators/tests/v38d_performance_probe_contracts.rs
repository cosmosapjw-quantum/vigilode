use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    V38D_EXPLORATORY_PROBE_SCHEMA, V38D_EXPLORATORY_PROBE_STATUS, V38D_MEASURED_REPETITIONS,
    V38D_WARMUP_REPETITIONS, V38dCandidateId, V38dProbeCaseId, V38dProbeReport, V38dProbeSample,
    run_v38d_probe,
};

fn sample(repetition: usize) -> V38dProbeSample {
    V38dProbeSample {
        repetition,
        wall_seconds: 1.0,
        allocations: 0,
        allocated_bytes: 0,
        work: WorkCounters::default(),
        output_checksum: "0".repeat(64),
        authority_wrms_defect: 0.0,
        residual_estimate: 0.0,
        converged: true,
    }
}

#[test]
fn authority_probe_schema_is_explicitly_non_authority_and_retains_samples() {
    let report = V38dProbeReport::new(
        V38dProbeCaseId::StiffDiagonal96,
        V38dCandidateId::FullMgsAuthority,
        vec![sample(0)],
        (0..7).map(sample).collect(),
    )
    .expect("schema-only report");

    assert_eq!(report.schema(), V38D_EXPLORATORY_PROBE_SCHEMA);
    assert_eq!(report.status(), V38D_EXPLORATORY_PROBE_STATUS);
    assert_eq!(report.case_id(), V38dProbeCaseId::StiffDiagonal96);
    assert_eq!(report.candidate_id(), V38dCandidateId::FullMgsAuthority);
    assert_eq!(report.warmups().len(), V38D_WARMUP_REPETITIONS);
    assert_eq!(report.measured().len(), V38D_MEASURED_REPETITIONS);
    assert!(!report.timing_authority());
    assert!(!report.speedup_claim_authorized());
    assert!(!report.active_switching_authorized());
    assert!(!report.policy_retuning_authorized());
    assert!(!report.release_claim_authorized());
    assert!(!report.n2048_authorized());
}

#[test]
fn case_and_candidate_identities_are_stable_and_complete() {
    let cases = V38dProbeCaseId::ALL.map(V38dProbeCaseId::as_str);
    assert_eq!(
        cases,
        [
            "stiff-diagonal-96",
            "nonnormal-jordan-96",
            "oscillatory-blocks-96",
            "diffusion-like-192",
            "mixed-forcing-192",
        ]
    );
    assert_eq!(
        V38dCandidateId::FullMgsAuthority.as_str(),
        "full-mgs-authority"
    );
}

#[test]
fn schema_report_rejects_wrong_cardinality_and_repetition_labels() {
    let wrong_warmups = V38dProbeReport::new(
        V38dProbeCaseId::StiffDiagonal96,
        V38dCandidateId::FullMgsAuthority,
        Vec::new(),
        (0..7).map(sample).collect(),
    )
    .expect_err("missing warmup must fail closed");
    assert!(matches!(wrong_warmups, CoreError::InvalidInput(_)));

    let wrong_measured = V38dProbeReport::new(
        V38dProbeCaseId::StiffDiagonal96,
        V38dCandidateId::FullMgsAuthority,
        vec![sample(0)],
        (0..6).map(sample).collect(),
    )
    .expect_err("missing measured repetition must fail closed");
    assert!(matches!(wrong_measured, CoreError::InvalidInput(_)));

    let mut mislabeled = (0..7).map(sample).collect::<Vec<_>>();
    mislabeled[4].repetition = 99;
    let wrong_label = V38dProbeReport::new(
        V38dProbeCaseId::StiffDiagonal96,
        V38dCandidateId::FullMgsAuthority,
        vec![sample(0)],
        mislabeled,
    )
    .expect_err("mislabeled repetition must fail closed");
    assert!(matches!(wrong_label, CoreError::InvalidInput(_)));
}

#[test]
fn exact_probe_runner_remains_fail_closed_until_cases_are_implemented() {
    let error = run_v38d_probe(
        V38dProbeCaseId::StiffDiagonal96,
        V38dCandidateId::FullMgsAuthority,
        V38D_WARMUP_REPETITIONS,
        V38D_MEASURED_REPETITIONS,
    )
    .expect_err("Task 1 must not fabricate a benchmark report");

    assert!(matches!(
        error,
        CoreError::InvalidInput(message) if message == "v3.8-D probe case not implemented"
    ));
}

#[test]
fn runner_rejects_non_contract_sample_counts_before_not_implemented_boundary() {
    for (warmups, measured) in [(0, 7), (1, 6), (2, 7), (1, 8)] {
        let error = run_v38d_probe(
            V38dProbeCaseId::StiffDiagonal96,
            V38dCandidateId::FullMgsAuthority,
            warmups,
            measured,
        )
        .expect_err("non-contract sample counts must fail closed");
        assert!(matches!(error, CoreError::InvalidInput(_)));
    }
}
