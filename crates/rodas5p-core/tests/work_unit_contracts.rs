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
