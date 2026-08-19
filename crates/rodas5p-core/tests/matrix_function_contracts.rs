use rodas5p_core::{DenseMatrix, dense_phi_action, matrix_exp_pade13, safe_l2};

fn scalar_phi(z: f64, k: usize) -> f64 {
    if k == 0 {
        return z.exp();
    }
    if z.abs() < 1.0e-7 {
        let mut term = 1.0 / (1..=k).product::<usize>() as f64;
        let mut sum = term;
        for j in 1..80 {
            term *= z / (k + j) as f64;
            sum += term;
            if term.abs() <= 1.0e-18 * sum.abs().max(1.0) {
                break;
            }
        }
        return sum;
    }
    let mut value = z.exp();
    let mut factorial = 1.0;
    for j in 0..k {
        if j > 0 {
            factorial *= j as f64;
        }
        value = (value - 1.0 / factorial) / z;
    }
    value
}

#[test]
fn pade13_matches_scalar_exponential_across_stiff_scales() {
    for z in [-100.0_f64, -10.0, -1.0, 0.0, 1.0, 5.0] {
        let matrix = DenseMatrix::new(1, 1, vec![z]).expect("matrix");
        let got = matrix_exp_pade13(&matrix).expect("matrix exponential")[(0, 0)];
        let reference = z.exp();
        let scale = reference.abs().max(1.0e-300);
        assert!(
            (got - reference).abs() / scale < 2.0e-13,
            "z={z}, got={got}, ref={reference}"
        );
    }
}

#[test]
fn dense_phi_action_matches_scalar_values() {
    for z in [-20.0_f64, -1.0, 0.0, 0.5, 3.0] {
        let matrix = DenseMatrix::new(1, 1, vec![z]).expect("matrix");
        for k in 1..=5 {
            let got = dense_phi_action(&matrix, 1.0, k, &[1.0]).expect("phi action")[0];
            let reference = scalar_phi(z, k);
            assert!(
                (got - reference).abs() <= 3.0e-12 * reference.abs().max(1.0),
                "z={z}, k={k}, got={got}, ref={reference}"
            );
        }
    }
}

#[test]
fn dense_phi_action_satisfies_matrix_recurrence() {
    let matrix = DenseMatrix::from_vec_rows(vec![
        vec![-3.0, 2.0, 0.0],
        vec![0.0, -4.0, 1.5],
        vec![0.0, 0.0, -5.0],
    ])
    .expect("matrix");
    let vector = vec![1.0, -0.5, 0.25];
    let scale = 0.4;
    for k in 1..=4 {
        let phi_k = dense_phi_action(&matrix, scale, k, &vector).expect("phi k");
        let phi_next = dense_phi_action(&matrix, scale, k + 1, &vector).expect("phi next");
        let mut applied = matrix.matvec(&phi_next).expect("matvec");
        for value in &mut applied {
            *value *= scale;
        }
        let factorial = (1..=k).product::<usize>() as f64;
        let rhs: Vec<f64> = phi_k
            .iter()
            .zip(&vector)
            .map(|(value, input)| value - input / factorial)
            .collect();
        let defect: Vec<f64> = applied.iter().zip(&rhs).map(|(a, b)| a - b).collect();
        assert!(safe_l2(&defect) <= 2.0e-11 * safe_l2(&rhs).max(1.0));
    }
}
