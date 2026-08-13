use rodas5p_core::{DenseMatrix, dense_fused_phi_action, dense_phi_action};

fn max_abs(x: &[f64], y: &[f64]) -> f64 {
    x.iter()
        .zip(y)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}

#[test]
fn fused_dense_action_matches_separate_phi_actions() {
    let a = DenseMatrix::from_vec_rows(vec![
        vec![-3.0, 4.0, 0.0],
        vec![0.0, -5.0, 2.0],
        vec![0.0, 0.0, -7.0],
    ])
    .unwrap();
    let scale = 0.37;
    let vectors = vec![
        vec![0.4, -0.1, 0.2],
        vec![1.0, 0.25, -0.5],
        vec![-0.2, 0.75, 0.1],
        vec![0.3, -0.4, 0.9],
    ];
    let fused = dense_fused_phi_action(&a, scale, &vectors).unwrap();
    let mut separate = dense_phi_action(&a, scale, 0, &vectors[0]).unwrap();
    for (k, vector) in vectors.iter().enumerate().skip(1) {
        let action = dense_phi_action(&a, scale, k, vector).unwrap();
        let factor = scale.powi(k as i32);
        for (out, value) in separate.iter_mut().zip(action) {
            *out += factor * value;
        }
    }
    assert!(
        max_abs(&fused, &separate) < 3.0e-13,
        "fused={fused:?} separate={separate:?}"
    );
}

#[test]
fn fused_dense_action_handles_only_b0() {
    let a = DenseMatrix::from_vec_rows(vec![vec![-2.0, 1.0], vec![0.0, -3.0]]).unwrap();
    let b0 = vec![1.0, -0.5];
    let fused = dense_fused_phi_action(&a, 0.2, std::slice::from_ref(&b0)).unwrap();
    let expected = dense_phi_action(&a, 0.2, 0, &b0).unwrap();
    assert!(max_abs(&fused, &expected) < 1.0e-14);
}
