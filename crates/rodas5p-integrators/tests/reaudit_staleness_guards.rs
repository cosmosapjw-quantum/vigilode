//! Regression of a finite-input weighted-norm NaN masking defect.
use rodas5p_core::DenseMatrix;
use rodas5p_integrators::rodas5p_staleness_bound;

#[test]
fn overflowed_weight_ratio_cannot_hide_a_nonzero_drift_row() {
    let jp = DenseMatrix::from_rows(&[&[-1., 0., 0.], &[0., -1., 0.], &[0., 0., -1.]]).unwrap();
    let mut j = jp.clone();
    j[(0, 2)] = 1.0;
    let weights = [f64::MIN_POSITIVE, f64::MAX, f64::MIN_POSITIVE];
    // Exact T has T[0,2] = 1/2, hence ||T||_S = 1/2.
    // 0*(MAX/MIN_POSITIVE) used to poison row 0, then f64::max hid it.
    let q = rodas5p_staleness_bound(1., 1., 1., &j, &jp, &weights);
    assert!(q >= 0.5, "nonzero drift was hidden: q={q}");
}

#[test]
fn empty_matrix_is_not_a_staleness_certificate() {
    let j = DenseMatrix::zeros(0, 0);
    assert_eq!(
        rodas5p_staleness_bound(1., 1., 1., &j, &j, &[]),
        f64::INFINITY
    );
}
