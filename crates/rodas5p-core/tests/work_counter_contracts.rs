use std::sync::Arc;

use rodas5p_core::{
    ApplyCategory, DenseMatrix, DenseOperator, LinearOperator, ShiftedOperator, WorkCounters,
    apply_counted,
};

#[test]
fn work_counters_accumulate_componentwise_and_saturate() {
    let mut left = WorkCounters {
        rhs_calls: 3,
        jvp_vectors: u64::MAX - 1,
        linear_iterations: 5,
        linear_solve_failures: 2,
        ..WorkCounters::default()
    };
    left.accumulate(WorkCounters {
        rhs_calls: 4,
        jvp_vectors: 9,
        linear_iterations: 7,
        linear_solve_failures: 3,
        ..WorkCounters::default()
    });
    assert_eq!(left.rhs_calls, 7);
    assert_eq!(left.jvp_vectors, u64::MAX);
    assert_eq!(left.linear_iterations, 12);
    assert_eq!(left.linear_solve_failures, 5);
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

#[test]
fn shifted_and_total_operator_application_helpers_count_vectors_without_block_call_duplication() {
    let before = WorkCounters {
        linear_matvecs: 3,
        diagnostic_matvecs: 5,
        recycle_refresh_matvecs: 7,
        block_matvecs: 11,
        ..WorkCounters::default()
    };
    let after = WorkCounters {
        linear_matvecs: 13,
        diagnostic_matvecs: 17,
        recycle_refresh_matvecs: 19,
        block_matvecs: 23,
        ..WorkCounters::default()
    };

    assert_eq!(after.shifted_operator_applications_since(before), 34);
    assert_eq!(after.operator_applications(), 49);
}

#[test]
fn counted_shifted_applications_charge_jvp_mass_and_refresh_at_the_apply_site() {
    // Defect caught: post-hoc folding omitted refresh applications and let
    // callers outside the sequential lane silently under-report JVP work.
    let jacobian: Arc<dyn LinearOperator> =
        Arc::new(DenseOperator::new(DenseMatrix::identity(2)).unwrap());
    let shifted =
        ShiftedOperator::new_counted_jvp(Some(DenseMatrix::identity(2)), jacobian, 0.1, 0.25)
            .unwrap();
    let mut output = [0.0; 2];
    let mut work = WorkCounters::default();

    apply_counted(
        &shifted,
        &[1.0, -2.0],
        &mut output,
        &mut work,
        ApplyCategory::Refresh,
    )
    .unwrap();

    assert_eq!(work.recycle_refresh_matvecs, 1);
    assert_eq!(work.jvp_calls, 1);
    assert_eq!(work.jvp_vectors, 1);
    assert_eq!(work.mass_matvecs, 1);
}

#[test]
fn shifted_exact_identity_uses_jacobian_mass_and_exact_h_gamma_bits() {
    let jacobian: Arc<dyn LinearOperator> =
        Arc::new(DenseOperator::new(DenseMatrix::identity(2)).unwrap());
    let first = ShiftedOperator::new_counted_jvp(
        Some(DenseMatrix::identity(2)),
        Arc::clone(&jacobian),
        0.1,
        0.25,
    )
    .unwrap();
    let same = ShiftedOperator::new_counted_jvp(
        Some(DenseMatrix::identity(2)),
        Arc::clone(&jacobian),
        0.1,
        0.25,
    )
    .unwrap();
    let changed_h = ShiftedOperator::new_counted_jvp(
        Some(DenseMatrix::identity(2)),
        Arc::clone(&jacobian),
        f64::from_bits(0.1_f64.to_bits() + 1),
        0.25,
    )
    .unwrap();
    let changed_mass = ShiftedOperator::new_counted_jvp(
        Some(DenseMatrix::from_rows(&[&[2.0, 0.0], &[0.0, 1.0]]).unwrap()),
        jacobian,
        0.1,
        0.25,
    )
    .unwrap();

    assert_ne!(first.token(), same.token());
    assert_eq!(first.exact_identity(), same.exact_identity());
    assert_ne!(first.exact_identity(), changed_h.exact_identity());
    assert_ne!(first.exact_identity(), changed_mass.exact_identity());
}
