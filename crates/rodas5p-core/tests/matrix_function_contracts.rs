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

/// 8x8 nonnormal upper-triangular matrix with a stiff diagonal.
fn nonnormal_matrix() -> DenseMatrix {
    let n = 8;
    let mut rows = vec![vec![0.0; n]; n];
    for (i, row) in rows.iter_mut().enumerate() {
        row[i] = -(10.0_f64.powf(3.0 * i as f64 / (n - 1) as f64));
        for (j, value) in row.iter_mut().enumerate().skip(i + 1) {
            *value = 5.0 * (((i * 7 + j * 3) % 11) as f64 / 11.0 - 0.5);
        }
    }
    DenseMatrix::from_vec_rows(rows).unwrap()
}

fn relative_deviation(a: &[f64], b: &[f64]) -> f64 {
    let difference: Vec<f64> = a.iter().zip(b).map(|(x, y)| x - y).collect();
    safe_l2(&difference) / safe_l2(b)
}

#[test]
fn dense_phi_action_is_invariant_under_the_physical_scale_of_the_vector() {
    // Audit F-040 / E-06: the vector entered the augmented matrix unscaled, so
    // ||v|| = 1e8 added about 24 Pade squarings and a 1e-8 relative error.
    let matrix = nonnormal_matrix();
    let vector: Vec<f64> = (0..8).map(|i| ((i as f64) * 0.37).sin() + 0.2).collect();
    for k in 1..=4 {
        let unit = dense_phi_action(&matrix, 1.0e-3, k, &vector).unwrap();
        for factor in [1.0e-8, 1.0e8] {
            let scaled_vector: Vec<f64> = vector.iter().map(|v| v * factor).collect();
            let scaled = dense_phi_action(&matrix, 1.0e-3, k, &scaled_vector).unwrap();
            let expected: Vec<f64> = unit.iter().map(|v| v * factor).collect();
            let deviation = relative_deviation(&scaled, &expected);
            assert!(
                deviation <= 1.0e3 * f64::EPSILON,
                "phi_{k} with ||v|| scaled by {factor:e}: relative deviation {deviation:.3e}"
            );
        }
    }
}

/// Reference `phi_k(z)` without cancellation: the Taylor series
/// `sum_j z^j / (j + k)!` for `|z| <= 1`, the recurrence otherwise.
fn scalar_phi_series_or_recurrence(z: f64, k: usize) -> f64 {
    if z.abs() > 1.0 {
        return scalar_phi(z, k);
    }
    let mut term = 1.0 / (1..=k).product::<usize>() as f64;
    let mut sum = term;
    for j in 1..60 {
        term *= z / (k + j) as f64;
        sum += term;
    }
    sum
}

#[test]
fn dense_phi_action_is_accurate_for_a_large_vector_on_a_diagonal_matrix() {
    let lambdas = [-1.0e3, -30.0, -1.0, -1.0e-3];
    let mut matrix = DenseMatrix::zeros(4, 4);
    for (i, lambda) in lambdas.iter().enumerate() {
        matrix[(i, i)] = *lambda;
    }
    let vector = [1.0e8, -2.0e8, 5.0e7, 3.0e8];
    for k in 1..=4 {
        let actual = dense_phi_action(&matrix, 1.0, k, &vector).unwrap();
        let expected: Vec<f64> = lambdas
            .iter()
            .zip(vector)
            .map(|(lambda, v)| v * scalar_phi_series_or_recurrence(*lambda, k))
            .collect();
        let error = relative_deviation(&actual, &expected);
        assert!(
            error <= 1.0e3 * f64::EPSILON,
            "phi_{k}: relative error {error:.3e}"
        );
    }
}

/// External audit VIG-A06: the power-of-two normalizer of the augmentation
/// column must stay finite for every finite vector. `phi_1(0) v = v`.
#[test]
fn dense_phi_action_accepts_the_largest_finite_vectors() {
    let zero = DenseMatrix::zeros(1, 1);
    for (k, value) in [(1, 1.0e308), (1, f64::MAX), (1, -f64::MAX), (2, 1.0e308)] {
        let out = dense_phi_action(&zero, 1.0, k, &[value])
            .unwrap_or_else(|error| panic!("phi_{k}(0) {value:e}: {error}"));
        let expected = value / if k == 1 { 1.0 } else { 2.0 };
        assert!(
            (out[0] / expected - 1.0).abs() <= 1.0e-15,
            "phi_{k}(0) {value:e}: {}",
            out[0]
        );
    }
}

#[test]
fn dense_phi_action_is_accurate_for_subnormal_and_power_of_two_vectors() {
    let zero = DenseMatrix::zeros(1, 1);
    for value in [
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        0.5,
        1.0,
        2.0,
        2.0_f64.powi(1023),
    ] {
        let out = dense_phi_action(&zero, 1.0, 1, &[value]).unwrap();
        // Pade gives the scaled column to within an ulp, and the rescale may
        // round once more in the subnormal range.
        let allowance = 4.0 * f64::EPSILON * value + f64::from_bits(1);
        assert!(
            (out[0] - value).abs() <= allowance,
            "phi_1(0) {value:e}: {:e}",
            out[0]
        );
    }
}

#[test]
fn dense_phi_action_reports_a_result_that_really_overflows() {
    // phi_1(1) = e - 1 > 1, so phi_1(1) f64::MAX is not representable.
    let one = DenseMatrix::from_rows(&[&[1.0]]).unwrap();
    assert!(dense_phi_action(&one, 1.0, 1, &[f64::MAX]).is_err());
}

/// phi_k(z) by its Taylor series; exact to rounding for |z| <= 0.5.
fn phi_series(z: f64, k: usize) -> f64 {
    let mut term = 1.0;
    for j in 1..=k {
        term /= j as f64;
    }
    let mut sum = term;
    for j in 1..40 {
        term *= z / (k + j) as f64;
        sum += term;
    }
    sum
}

/// External audit F-042: the fused combination divides its inputs by h^k, so
/// the augmented matrix's 1-norm is O(1/h^3) while the phi arguments are
/// O(h). Squaring by the 1-norm alone loses accuracy as h shrinks.
#[test]
fn fused_phi_combination_is_accurate_as_the_step_shrinks() {
    use rodas5p_core::dense_fused_phi_action;
    let diagonal = [-1.0, -3.0];
    let matrix = DenseMatrix::from_rows(&[&[diagonal[0], 0.0], &[0.0, diagonal[1]]]).unwrap();
    let v = [
        [1.0, 2.0],
        [0.5, -1.0],
        [2.0, 0.25],
        [-1.0, 1.0],
        [0.75, -0.5],
    ];
    for h in [1.0e-1_f64, 1.0e-2, 1.0e-3, 1.0e-4] {
        let vectors = (0..=4)
            .map(|k| {
                v[k].iter()
                    .map(|x| x / h.powi(k as i32))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let out = dense_fused_phi_action(&matrix, h, &vectors).unwrap();
        let mut worst = 0.0_f64;
        for i in 0..2 {
            let z = h * diagonal[i];
            let exact: f64 = (0..=4).map(|k| phi_series(z, k) * v[k][i]).sum();
            worst = worst.max(((out[i] - exact) / exact).abs());
        }
        assert!(worst <= 1.0e-14, "h {h:e}: relative error {worst:e}");
    }
}
