//! Cross-lane operator work is compared in state-vector units (audit
//! 2026-09-30, B-02).

use rodas5p_core::WorkCounters;

#[test]
fn cross_lane_operator_work_is_counted_in_state_vectors() {
    // Audit 2026-09-30, B-02: a sequential lane with 16 single-vector
    // applies and an 8-stage block lane with 3 block applies. As calls the
    // block lane looks cheaper (3 < 16); in state vectors it is not
    // (24 > 16).
    let sequential = WorkCounters {
        linear_matvecs: 16,
        linear_matvec_vectors: 16,
        ..WorkCounters::default()
    };
    let block = WorkCounters {
        linear_matvecs: 3,
        linear_matvec_vectors: 24,
        ..WorkCounters::default()
    };
    assert!(block.operator_applications() < sequential.operator_applications());
    assert_eq!(sequential.operator_state_vectors(), Some(16));
    assert_eq!(block.operator_state_vectors(), Some(24));
    // A legacy ledger without vector counters is unknown, not free.
    let legacy = WorkCounters {
        linear_matvecs: 16,
        ..WorkCounters::default()
    };
    assert_eq!(legacy.operator_state_vectors(), None);
    assert_eq!(WorkCounters::default().operator_state_vectors(), Some(0));
    // Refreshes are single-vector applies.
    let refreshed = WorkCounters {
        linear_matvecs: 2,
        linear_matvec_vectors: 2,
        recycle_refresh_matvecs: 3,
        ..WorkCounters::default()
    };
    assert_eq!(refreshed.operator_state_vectors(), Some(5));
}

#[test]
fn unknown_vector_coverage_survives_aggregation() {
    // Re-audit R2, R2-STAT-02: a legacy ledger (16 calls, no vector units)
    // merged with a current one (1 call, 1 vector) read as 17 calls costing
    // 1 vector.
    let legacy: WorkCounters = serde_json::from_str(
        &serde_json::to_string(&WorkCounters {
            linear_matvecs: 16,
            ..WorkCounters::default()
        })
        .unwrap(),
    )
    .unwrap();
    let current = WorkCounters {
        linear_matvecs: 1,
        linear_matvec_vectors: 1,
        ..WorkCounters::default()
    };
    let mut merged = WorkCounters::default();
    merged.accumulate(legacy);
    merged.accumulate(current);
    assert_eq!(merged.linear_matvecs, 17);
    assert_eq!(merged.unknown_vector_calls(), 16);
    assert_eq!(merged.operator_state_vectors(), None);
    let mut checked = current;
    checked.checked_accumulate(legacy).unwrap();
    assert_eq!(checked, merged, "order does not matter");
    // Grouping does not matter either, and the marker survives a round trip.
    let mut grouped = legacy;
    let mut pair = current;
    pair.accumulate(current);
    grouped.accumulate(pair);
    let mut sequential = legacy;
    sequential.accumulate(current);
    sequential.accumulate(current);
    assert_eq!(grouped, sequential);
    let round_trip: WorkCounters =
        serde_json::from_str(&serde_json::to_string(&grouped).unwrap()).unwrap();
    assert_eq!(round_trip.operator_state_vectors(), None);
    // Known plus known is the exact sum, and a known ledger's bytes do not
    // gain the marker.
    let mut known = current;
    known.accumulate(WorkCounters {
        linear_matvecs: 3,
        linear_matvec_vectors: 24,
        ..WorkCounters::default()
    });
    assert_eq!(known.operator_state_vectors(), Some(25));
    assert!(
        !serde_json::to_string(&known)
            .unwrap()
            .contains("merged_unknown_vector_calls")
    );
}
