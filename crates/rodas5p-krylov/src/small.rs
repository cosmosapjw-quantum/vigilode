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
    let fa = a.to_faer();
    let fb = b.to_faer();
    let evd = fa
        .generalized_eigen(&fb)
        .map_err(|e| CoreError::LinearSolve(format!("generalized eigensolve failed: {e:?}")))?;
    let sa = evd.S_a().column_vector();
    let sb = evd.S_b().column_vector();
    let u = evd.U();
    let mut values = Vec::with_capacity(n);
    let mut vectors = Vec::with_capacity(n);
    for j in 0..n {
        let den = sb[j];
        values.push(if den.norm() == 0.0 {
            c64::new(f64::INFINITY, 0.0)
        } else {
            sa[j] / den
        });
        vectors.push((0..n).map(|i| u[(i, j)]).collect());
    }
    Ok((values, vectors))
}
