use rodas5p_core::WorkCounters;
use rodas5p_integrators::{OdeProblem, SparseLinearPart, gray_scott::gray_scott_2d};
use std::sync::Arc;
fn zero_problem() -> OdeProblem {
    OdeProblem::new(
        "zero",
        1,
        Arc::new(|_, _, o| {
            o[0] = 0.;
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, _, _, o| {
            o[0] = 0.;
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}
#[test]
fn split_nan_callback_fails_closed() {
    let p = zero_problem()
        .with_linear_part(
            SparseLinearPart::new(1, 1, vec![0, 1], vec![0], vec![0.]).unwrap(),
            Arc::new(|_, _, o| {
                o[0] = f64::NAN;
                Ok(())
            }),
        )
        .unwrap();
    assert!(
        p.verify_linear_part(0., &[0.], &mut WorkCounters::default())
            .is_err()
    );
}
#[test]
fn sparse_descriptor_is_checked_after_public_field_mutation() {
    let mut a = SparseLinearPart::new(1, 1, vec![0, 1], vec![0], vec![1.]).unwrap();
    a.row_idx[0] = 1;
    assert!(a.matvec(&[1.], &mut [0.]).is_err());
    assert!(
        zero_problem()
            .with_linear_part(a, Arc::new(|_, _, _| Ok(())))
            .is_err()
    );
    assert!(SparseLinearPart::new(1, 1, vec![0, 1], vec![0], vec![f64::NAN]).is_err());
}
#[test]
fn sparse_matvec_rejects_short_vectors() {
    let a = SparseLinearPart::new(1, 1, vec![0, 1], vec![0], vec![1.]).unwrap();
    assert!(a.matvec(&[], &mut [0.]).is_err());
    assert!(a.matvec(&[1.], &mut []).is_err());
}
#[test]
fn periodic_gray_scott_two_cell_linear_split_matches_full_rhs() {
    let (p, y) = gray_scott_2d(2).unwrap();
    p.verify_linear_part(0., &y, &mut WorkCounters::default())
        .unwrap();
}
