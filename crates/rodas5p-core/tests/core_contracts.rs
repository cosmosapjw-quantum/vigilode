use rodas5p_core::{DenseMatrix, direct_solve, load_rodas5p_coefficients, safe_l2, wrms};

#[test]
fn safe_l2_handles_extreme_scale() {
    let n = safe_l2(&[3.0e200, 4.0e200]);
    assert!((n / 5.0e200 - 1.0).abs() < 1.0e-15);
}

#[test]
fn dense_matrix_matvec_is_row_major_and_exact_for_small_case() {
    let a = DenseMatrix::from_rows(&[&[2.0, -1.0], &[3.0, 4.0]]).unwrap();
    assert_eq!(a.matvec(&[5.0, 2.0]).unwrap(), vec![8.0, 23.0]);
}

#[test]
fn wrms_matches_definition() {
    let got = wrms(&[1.0, -2.0], &[2.0, 4.0]).unwrap();
    assert!((got - 0.5).abs() < 1.0e-15);
}

#[test]
fn coefficient_transform_preserves_rodas5p_structure() {
    let c = load_rodas5p_coefficients().unwrap();
    assert_eq!(c.stages(), 8);
    assert!((c.gamma - 0.211_937_563_194_290_14).abs() < 1.0e-16);
    for i in 0..8 {
        assert!((c.beta[(i, i)] - c.gamma).abs() < 2.0e-13);
        for j in i..8 {
            if i != j {
                assert!(c.l[(i, j)].abs() < 2.0e-13);
            }
        }
    }
}

#[test]
fn direct_solve_agrees_with_known_solution() {
    let a = DenseMatrix::from_rows(&[&[4.0, 1.0], &[2.0, 3.0]]).unwrap();
    let x = direct_solve(&a, &[9.0, 8.0]).unwrap();
    assert!((x[0] - 1.9).abs() < 1.0e-14);
    assert!((x[1] - 1.4).abs() < 1.0e-14);
}

#[test]
fn linear_operator_default_row_application_preserves_order() {
    use rodas5p_core::{ClosureOperator, LinearOperator};

    let op = ClosureOperator::new(3, |x, y| {
        y[0] = 2.0 * x[0];
        y[1] = -x[1];
        y[2] = x[0] + x[2];
        Ok(())
    });
    let inputs = vec![vec![1.0, 2.0, 3.0], vec![4.0, -5.0, 6.0]];
    let mut outputs = vec![vec![0.0; 3]; 2];
    op.apply_rows(&inputs, &mut outputs).unwrap();
    assert_eq!(outputs, vec![vec![2.0, -2.0, 4.0], vec![8.0, 5.0, 10.0]]);
}

#[test]
fn linear_operator_row_application_rejects_ragged_shapes() {
    use rodas5p_core::{ClosureOperator, LinearOperator};

    let op = ClosureOperator::new(2, |x, y| {
        y.copy_from_slice(x);
        Ok(())
    });
    let inputs = vec![vec![1.0, 2.0], vec![3.0]];
    let mut outputs = vec![vec![0.0; 2]; 2];
    assert!(op.apply_rows(&inputs, &mut outputs).is_err());
}
