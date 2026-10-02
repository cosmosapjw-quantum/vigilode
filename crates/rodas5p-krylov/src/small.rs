use faer::{Mat, c64, linalg::solvers::SolveLstsq};
use rodas5p_core::{CoreError, CoreResult, DenseMatrix};

pub fn least_squares(a: &DenseMatrix, b: &[f64]) -> CoreResult<Vec<f64>> {
    if a.nrows() != b.len() {
        return Err(CoreError::Dimension(
            "least-squares RHS shape mismatch".into(),
        ));
    }
    if a.ncols() == 0 {
        return Ok(Vec::new());
    }
    let fa = a.to_faer();
    let rhs = Mat::from_fn(b.len(), 1, |i, _| b[i]);
    let x = fa.col_piv_qr().solve_lstsq(&rhs);
    let out: Vec<f64> = (0..a.ncols()).map(|i| x[(i, 0)]).collect();
    if out.iter().all(|v| v.is_finite()) {
        Ok(out)
    } else {
        Err(CoreError::LinearSolve(format!(
            "least-squares solve produced NaN/Inf for {}x{} system",
            a.nrows(),
            a.ncols()
        )))
    }
}

pub fn generalized_eigen(
    a: &DenseMatrix,
    b: &DenseMatrix,
) -> CoreResult<(Vec<c64>, Vec<Vec<c64>>)> {
    if a.nrows() != a.ncols() || b.nrows() != b.ncols() || a.nrows() != b.nrows() {
        return Err(CoreError::Dimension(
            "generalized eigenproblem shape mismatch".into(),
        ));
    }
    let n = a.nrows();
    if n == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    if n == 1 {
        let denominator = b[(0, 0)];
        let value = if denominator == 0.0 {
            c64::new(f64::INFINITY, 0.0)
        } else {
            c64::new(a[(0, 0)] / denominator, 0.0)
        };
        return Ok((vec![value], vec![vec![c64::new(1.0, 0.0)]]));
    }
    // faer's `Mat::generalized_eigen` sizes its scratch with `gevd_scratch`,
    // which omits the eigenvector back-substitution of `qz_to_gevd_real`
    // (two `n x 1` norms, an `n x 2` right-hand side and an `n x 2`
    // product). For small pairs with a complex eigenvalue pair the buffer is
    // too short and faer panics ("buffer is not large enough"; found by the
    // thread-transfer matrix-free driver, research node
    // `research/thread_transfer_mf_workspace_20261002`). Call the same
    // routine with that requirement added.
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::linalg::evd::ComputeEigenvectors;
    let mut fa = a.to_faer();
    let mut fb = b.to_faer();
    let par = faer::get_global_parallelism();
    let scratch = faer::linalg::gevd::gevd_scratch::<f64>(
        n,
        ComputeEigenvectors::No,
        ComputeEigenvectors::Yes,
        par,
        Default::default(),
    )
    .or(faer::linalg::temp_mat_scratch::<f64>(n, 8));
    let mut u_real = Mat::<f64>::zeros(n, n);
    let mut s_re = faer::diag::Diag::<f64>::zeros(n);
    let mut s_im = faer::diag::Diag::<f64>::zeros(n);
    let mut s_b = faer::diag::Diag::<f64>::zeros(n);
    faer::linalg::gevd::gevd_real(
        fa.as_mut(),
        fb.as_mut(),
        s_re.as_mut(),
        s_im.as_mut(),
        s_b.as_mut(),
        None,
        Some(u_real.as_mut()),
        par,
        MemStack::new(&mut MemBuffer::new(scratch)),
        Default::default(),
    )
    .map_err(|e| CoreError::LinearSolve(format!("generalized eigensolve failed: {e:?}")))?;
    // A complex pair (j, j + 1) is stored as the real and imaginary parts of
    // one eigenvector, as in faer's `real_to_cplx`.
    let sa = |j: usize| c64::new(s_re[j], s_im[j]);
    let mut values = Vec::with_capacity(n);
    let mut vectors = Vec::with_capacity(n);
    let mut j = 0;
    while j < n {
        // The same complex division by `beta + 0i` as faer's result.
        let den = c64::new(s_b[j], 0.0);
        let ratio = |value: c64| {
            if den.norm() == 0.0 {
                c64::new(f64::INFINITY, 0.0)
            } else {
                value / den
            }
        };
        if s_im[j] == 0.0 || j + 1 == n {
            values.push(ratio(sa(j)));
            vectors.push((0..n).map(|i| c64::new(u_real[(i, j)], 0.0)).collect());
            j += 1;
        } else {
            values.push(ratio(sa(j)));
            values.push(ratio(c64::new(s_re[j], -s_im[j])));
            vectors.push(
                (0..n)
                    .map(|i| c64::new(u_real[(i, j)], u_real[(i, j + 1)]))
                    .collect(),
            );
            vectors.push(
                (0..n)
                    .map(|i| c64::new(u_real[(i, j)], -u_real[(i, j + 1)]))
                    .collect(),
            );
            j += 2;
        }
    }
    Ok((values, vectors))
}

#[cfg(test)]
mod generalized_eigen_tests {
    use super::*;

    fn matrix(rows: &[&[f64]]) -> DenseMatrix {
        DenseMatrix::from_rows(rows).unwrap()
    }

    fn bits(hex: &str) -> f64 {
        f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
    }

    /// max_j ||A v_j - lambda_j B v_j|| / (||A|| + |lambda_j| ||B||) ||v_j||.
    fn worst_residual(a: &DenseMatrix, b: &DenseMatrix) -> f64 {
        let (values, vectors) = generalized_eigen(a, b).unwrap();
        let n = a.nrows();
        assert_eq!(values.len(), n);
        let norm = |m: &DenseMatrix| m.as_slice().iter().map(|x| x.abs()).fold(0.0, f64::max);
        let mut worst = 0.0_f64;
        for (lambda, v) in values.iter().zip(&vectors) {
            let mut residual = 0.0_f64;
            for i in 0..n {
                let mut r = c64::new(0.0, 0.0);
                for k in 0..n {
                    r += v[k] * a[(i, k)] - *lambda * v[k] * b[(i, k)];
                }
                residual = residual.max(r.norm());
            }
            let vnorm = v.iter().map(|z| z.norm()).fold(0.0, f64::max);
            worst = worst.max(residual / ((norm(a) + lambda.norm() * norm(b)) * vnorm));
        }
        worst
    }

    #[test]
    fn a_complex_pair_of_a_small_pencil_does_not_panic() {
        // The 2 x 2 pencil from the GCRO-DR harmonic Ritz step that made
        // faer 0.24.4's `Mat::generalized_eigen` panic (scratch too short).
        let a = matrix(&[
            &[bits("3feffffffffffd80"), bits("bd30084eafffff60")],
            &[bits("bd30084eafffff60"), bits("3ff00000000001da")],
        ]);
        let b = matrix(&[
            &[bits("3feffffffffffec4"), bits("3cb7ec23ffffff11")],
            &[bits("bd3ed1b14c0000dd"), bits("3ff00000000000f3")],
        ]);
        assert!(worst_residual(&a, &b) <= 1.0e-12);
        let (values, _) = generalized_eigen(&a, &b).unwrap();
        assert!(values.iter().all(|v| v.re.is_finite() && v.im.is_finite()));
    }

    #[test]
    fn real_and_complex_spectra_are_eigenpairs() {
        // Rotation-scaled block: eigenvalues 1 +- 2i, and a real 3.
        let a = matrix(&[&[1.0, -2.0, 0.0], &[2.0, 1.0, 0.0], &[0.0, 0.0, 3.0]]);
        let b = DenseMatrix::identity(3);
        assert!(worst_residual(&a, &b) <= 1.0e-13);
        let (mut values, _) = generalized_eigen(&a, &b).unwrap();
        values.sort_by(|x, y| x.re.total_cmp(&y.re).then(x.im.total_cmp(&y.im)));
        let expected = [c64::new(1.0, -2.0), c64::new(1.0, 2.0), c64::new(3.0, 0.0)];
        for (v, e) in values.iter().zip(expected) {
            assert!((v - e).norm() <= 1.0e-12, "{values:?}");
        }
        // A general pencil with a nontrivial B.
        let a = matrix(&[&[2.0, 1.0, 0.5], &[-1.0, 3.0, 0.2], &[0.3, -0.4, 1.0]]);
        let b = matrix(&[&[1.0, 0.1, 0.0], &[0.0, 2.0, 0.3], &[0.2, 0.0, 1.5]]);
        assert!(worst_residual(&a, &b) <= 1.0e-12);
    }
}
