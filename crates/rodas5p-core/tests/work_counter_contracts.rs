use rodas5p_core::WorkCounters;

#[test]
fn work_counters_accumulate_componentwise_and_saturate() {
    let mut left = WorkCounters {
        rhs_calls: 3,
        jvp_vectors: u64::MAX - 1,
        linear_iterations: 5,
        ..WorkCounters::default()
    };
    left.accumulate(WorkCounters {
        rhs_calls: 4,
        jvp_vectors: 9,
        linear_iterations: 7,
        ..WorkCounters::default()
    });
    assert_eq!(left.rhs_calls, 7);
    assert_eq!(left.jvp_vectors, u64::MAX);
    assert_eq!(left.linear_iterations, 12);
}

#[test]
fn checked_delta_rejects_any_negative_component_and_round_trips_exactly() {
    let prefix = WorkCounters {
        rhs_calls: 3,
        jvp_vectors: 5,
        ..WorkCounters::default()
    };
    let full = WorkCounters {
        rhs_calls: 7,
        jvp_vectors: 13,
        ..WorkCounters::default()
    };
    let continuation = full.checked_delta(prefix).unwrap();
    assert_eq!(continuation.rhs_calls, 4);
    assert_eq!(continuation.jvp_vectors, 8);
    let mut reconstructed = prefix;
    reconstructed.accumulate(continuation);
    assert_eq!(reconstructed, full);

    let invalid_full = WorkCounters {
        rhs_calls: 2,
        jvp_vectors: 13,
        ..WorkCounters::default()
    };
    assert!(invalid_full.checked_delta(prefix).is_none());
}
