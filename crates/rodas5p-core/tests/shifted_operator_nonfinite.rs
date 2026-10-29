//! A non-finite shifted matrix `M - h gamma J` is a typed error, not a panic.
//!
//! Release builds use `panic = "abort"`, so the former `expect` in
//! `ShiftedOperator` killed the process when a Jacobian turned non-finite
//! (component triage probe `nan-jac-late`, 48 arm pairs). The step must fail
//! closed instead.

use std::sync::Arc;

use rodas5p_core::{CoreError, DenseMatrix, DenseOperator, ShiftedOperator};

#[test]
fn overflowing_shift_is_a_non_finite_error() {
    let jacobian = DenseOperator::new(DenseMatrix::new(1, 1, vec![1.0e308]).unwrap()).unwrap();
    let shifted = ShiftedOperator::new(None, Arc::new(jacobian), 1.0e10, 1.0);
    assert!(matches!(shifted, Err(CoreError::NonFinite(_))));
}

#[test]
fn finite_shift_is_unchanged() {
    let jacobian = DenseOperator::new(DenseMatrix::new(1, 1, vec![-2.0]).unwrap()).unwrap();
    let shifted = ShiftedOperator::new(None, Arc::new(jacobian), 0.5, 1.0).unwrap();
    assert_eq!(shifted.h_gamma(), 0.5);
}
