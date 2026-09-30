//! Output bound for rounded phi weights (re-audit R3 of 2026-10-01, ARITH-02).

use rodas5p_core::{
    DenseMatrix,
    transform_bound::{TransformBound, TransformOperatorClass, bound_transform_error},
    weight_phi_vectors,
};

fn nilpotent(off: f64) -> DenseMatrix {
    DenseMatrix::from_rows(&[&[0.0, off], &[0.0, 0.0]]).unwrap()
}

fn witness() -> Vec<Vec<f64>> {
    vec![vec![1.0e-300, 0.0], vec![0.0, 0.0], vec![0.0, 1.0e-310]]
}

fn bound(matrix: &DenseMatrix, h: f64, vectors: &[Vec<f64>]) -> TransformBound {
    let (stored, _) = weight_phi_vectors(h, vectors).unwrap();
    bound_transform_error(matrix, h, vectors, &stored).unwrap()
}

#[test]
fn the_nilpotent_witness_is_bounded_and_rejected_for_both_signs_of_h() {
    // Exact transform error (first component) 1.66666666666666169779916e-27
    // for h = +-1e-8, A12 = 1e308, b2 = 1e-310 (h^2 b2 underflows to 0).
    for h in [1.0e-8, -1.0e-8] {
        let result = bound(&nilpotent(1.0e308), h, &witness());
        let TransformBound::Bounded { class, upper, .. } = &result else {
            panic!("h = {h}: {result:?}");
        };
        assert_eq!(*class, TransformOperatorClass::Nilpotent { index: 2 });
        let value = upper.to_f64_up();
        assert!(value >= 1.6666666666666617e-27, "h = {h}: {value:e}");
        assert!(value <= 1.6666666666666680e-27, "h = {h}: {value:e}");
        // Against the relative tolerance 1e-12 of the output (about
        // 1.67e-27) the bound is far too large: rejected.
        assert!(!result.admits(1.0e-12 * 1.6666666666666617e-27));
    }
}

#[test]
fn the_zero_operator_control_is_bounded_nonzero_and_admitted() {
    // A = 0: the lost weight contributes h^2 b2 / 2 = 5e-327; the bound
    // stays nonzero below the subnormal range and fits 1e-12 * 1e-300.
    let zero = DenseMatrix::from_rows(&[&[0.0, 0.0], &[0.0, 0.0]]).unwrap();
    let result = bound(&zero, 1.0e-8, &witness());
    let TransformBound::Bounded { upper, .. } = &result else {
        panic!("{result:?}");
    };
    assert!(!upper.is_zero());
    assert!(upper.exponent < -1074 && upper.exponent > -1090);
    assert_eq!(upper.to_f64_up(), f64::from_bits(1));
    assert!(result.admits(1.0e-12 * 1.0e-300));
}

#[test]
fn every_branch_is_exercised_and_unverified_operators_fail_closed() {
    let diagonal = DenseMatrix::from_rows(&[&[-1.0, 0.0], &[0.0, -2.0]]).unwrap();
    // Normal weights: rounding only.
    let normal = bound(
        &diagonal,
        0.1,
        &[vec![0.0; 2], vec![0.0; 2], vec![1.0, 1.0]],
    );
    let TransformBound::Bounded { class, upper, .. } = &normal else {
        panic!("{normal:?}");
    };
    assert_eq!(*class, TransformOperatorClass::Dissipative);
    assert!(upper.to_f64_up() < 1.0e-17);
    // Subnormal weight: h = 1e-160, b = 3e-2 gives 3e-322.
    let subnormal = bound(
        &diagonal,
        1.0e-160,
        &[vec![0.0; 2], vec![0.0; 2], vec![3.0e-2, 0.0]],
    );
    let TransformBound::Bounded { upper, .. } = &subnormal else {
        panic!("{subnormal:?}");
    };
    assert!(upper.to_f64_up() > 0.0 && upper.to_f64_up() < 1.0e-322);
    // Near-overflow operator bound: C_2 about 1.7e317, above f64::MAX.
    let huge = bound(&nilpotent(1.0e308), 1.0e10, &witness());
    let TransformBound::Bounded { operator, .. } = &huge else {
        panic!("{huge:?}");
    };
    assert_eq!(operator[2].to_f64_up(), f64::INFINITY);
    assert!(!huge.admits(1.0));
    // A general operator: no verified class.
    let general = DenseMatrix::from_rows(&[&[1.0, 2.0], &[3.0, 4.0]]).unwrap();
    assert!(matches!(
        bound(&general, 0.1, &witness()),
        TransformBound::Unbounded { .. }
    ));
}
