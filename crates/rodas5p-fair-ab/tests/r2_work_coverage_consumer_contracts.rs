//! The Pareto consumer reads merged unknown vector coverage as not
//! evaluated (re-audit R2 of 2026-09-30, R2-STAT-02).

use rodas5p_core::WorkCounters;
use rodas5p_fair_ab::{
    ComparatorFidelity, IntegratorRunRecord, IntegratorRunStatus, IntegratorTimingReport,
    IntegratorWorkReport, ParetoCostMetric,
};

fn record(counters: WorkCounters) -> IntegratorRunRecord {
    IntegratorRunRecord {
        record_id: "row".into(),
        candidate_id: "candidate".into(),
        comparator_fidelity: ComparatorFidelity::Production,
        problem_id: "problem".into(),
        step_size: 0.1,
        status: IntegratorRunStatus::Success,
        message: "ok".into(),
        errors: None,
        work: IntegratorWorkReport {
            counters,
            internal_steps: 1,
            output_clipped_steps: 0,
            stored_state_bytes: 0,
        },
        timing: IntegratorTimingReport {
            authoritative: false,
            batch_iterations: 0,
            wall_samples_seconds: Vec::new(),
            wall_median_seconds: None,
            wall_q25_seconds: None,
            wall_q75_seconds: None,
        },
        reference_checksum: "reference".into(),
        output_grid_id: "grid".into(),
    }
}

#[test]
fn a_merged_legacy_segment_leaves_the_vector_cost_unevaluated() {
    // A ledger written before the vector counters: it has calls and no
    // `linear_matvec_vectors` key.
    let json = serde_json::to_string(&WorkCounters {
        linear_matvecs: 16,
        ..WorkCounters::default()
    })
    .unwrap();
    assert!(!json.contains("linear_matvec_vectors"));
    let legacy: WorkCounters = serde_json::from_str(&json).unwrap();
    let current = WorkCounters {
        linear_matvecs: 1,
        linear_matvec_vectors: 1,
        ..WorkCounters::default()
    };
    let mut merged = current;
    merged.accumulate(legacy);
    let row = record(merged);
    assert_eq!(row.cost(ParetoCostMetric::OperatorStateVectors), None);
    assert_eq!(
        row.cost(ParetoCostMetric::OperatorApplications),
        Some(17.0),
        "call counts stay known"
    );
    assert_eq!(
        record(current).cost(ParetoCostMetric::OperatorStateVectors),
        Some(1.0)
    );
}
