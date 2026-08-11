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
