use crate::{CoreError, CoreResult, DenseMatrix, LuFactorization};

const PADE_13_THETA: f64 = 5.371_920_351_148_152;
const PADE_13_COEFFICIENTS: [f64; 14] = [
    64_764_752_532_480_000.0,
    32_382_376_266_240_000.0,
    7_771_770_303_897_600.0,
    1_187_353_796_428_800.0,
    129_060_195_264_000.0,
    10_559_470_521_600.0,
    670_442_572_800.0,
    33_522_128_640.0,
    1_323_241_920.0,
    40_840_800.0,
    960_960.0,
    16_380.0,
    182.0,
    1.0,
];

fn matrix_one_norm(a: &DenseMatrix) -> f64 {
    (0..a.ncols())
        .map(|j| (0..a.nrows()).map(|i| a[(i, j)].abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn add_scaled(out: &mut DenseMatrix, source: &DenseMatrix, alpha: f64) -> CoreResult<()> {
    if out.nrows() != source.nrows() || out.ncols() != source.ncols() {
        return Err(CoreError::Dimension(
            "matrix-function linear combination shape mismatch".into(),
        ));
    }
    for (value, source_value) in out.as_mut_slice().iter_mut().zip(source.as_slice()) {
        *value += alpha * source_value;
    }
    Ok(())
}

fn solve_matrix_left(a: &DenseMatrix, b: &DenseMatrix) -> CoreResult<DenseMatrix> {
    if a.nrows() != a.ncols() || a.nrows() != b.nrows() {
        return Err(CoreError::Dimension(
            "matrix-function left solve shape mismatch".into(),
        ));
    }
    let factor = LuFactorization::new(a)?;
    let rhs_rows: Vec<Vec<f64>> = (0..b.ncols())
        .map(|j| (0..b.nrows()).map(|i| b[(i, j)]).collect())
        .collect();
    let solution_rows = factor.solve_rows(&rhs_rows)?;
    let mut out = DenseMatrix::zeros(b.nrows(), b.ncols());
    for j in 0..b.ncols() {
        for i in 0..b.nrows() {
            out[(i, j)] = solution_rows[j][i];
        }
    }
    Ok(out)
}

/// Dense reference matrix exponential using Higham's scaling-and-squaring Padé (13,13) formula.
///
/// This routine is the small projected-space oracle used by the exponential-integrator research
/// layer.  It is not the large-state production path: matrix-free Krylov methods call it only on
/// the projected Hessenberg matrix.
pub fn matrix_exp_pade13(a: &DenseMatrix) -> CoreResult<DenseMatrix> {
    if a.nrows() != a.ncols() {
        return Err(CoreError::Dimension(
            "matrix exponential requires a square matrix".into(),
        ));
    }
    let n = a.nrows();
    if n == 0 {
        return Ok(DenseMatrix::zeros(0, 0));
    }
    if !a.as_slice().iter().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "matrix exponential input contains NaN/Inf".into(),
        ));
    }

    let norm = matrix_one_norm(a);
    let squarings = if norm <= PADE_13_THETA || norm == 0.0 {
        0_u32
    } else {
        (norm / PADE_13_THETA).log2().ceil().max(0.0) as u32
    };
    let scaled = a.scale(2.0_f64.powi(-(squarings as i32)));
    let identity = DenseMatrix::identity(n);
    let a2 = scaled.matmul(&scaled)?;
    let a4 = a2.matmul(&a2)?;
    let a6 = a4.matmul(&a2)?;
    let b = PADE_13_COEFFICIENTS;

    let mut u_inner = a6.scale(b[13]);
    add_scaled(&mut u_inner, &a4, b[11])?;
    add_scaled(&mut u_inner, &a2, b[9])?;
    let mut u_poly = a6.matmul(&u_inner)?;
    add_scaled(&mut u_poly, &a6, b[7])?;
    add_scaled(&mut u_poly, &a4, b[5])?;
    add_scaled(&mut u_poly, &a2, b[3])?;
    add_scaled(&mut u_poly, &identity, b[1])?;
    let u = scaled.matmul(&u_poly)?;

    let mut v_inner = a6.scale(b[12]);
    add_scaled(&mut v_inner, &a4, b[10])?;
    add_scaled(&mut v_inner, &a2, b[8])?;
    let mut v = a6.matmul(&v_inner)?;
    add_scaled(&mut v, &a6, b[6])?;
    add_scaled(&mut v, &a4, b[4])?;
    add_scaled(&mut v, &a2, b[2])?;
    add_scaled(&mut v, &identity, b[0])?;

    let numerator = v.add(&u)?;
    let denominator = v.sub(&u)?;
    let mut result = solve_matrix_left(&denominator, &numerator)?;
    for _ in 0..squarings {
        result = result.matmul(&result)?;
    }
    if result.as_slice().iter().all(|value| value.is_finite()) {
        Ok(result)
    } else {
        Err(CoreError::NonFinite(
            "matrix exponential produced NaN/Inf".into(),
        ))
    }
}

/// Compute `phi_k(scale * A) v` through one augmented dense matrix exponential.
///
/// For `k >= 1`, the augmented matrix has the block form
/// `[[scale*A, v, 0, ...], [0, 0, 1, ...], ...]`.  The upper block of the last
/// column of its exponential equals `phi_k(scale*A) v`.
pub fn dense_phi_action(
    matrix: &DenseMatrix,
    scale: f64,
    phi_index: usize,
    vector: &[f64],
) -> CoreResult<Vec<f64>> {
    if matrix.nrows() != matrix.ncols() || vector.len() != matrix.nrows() {
        return Err(CoreError::Dimension(
            "dense phi-action shape mismatch".into(),
        ));
    }
    if !scale.is_finite() || !vector.iter().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "dense phi-action input contains NaN/Inf".into(),
        ));
    }
    if phi_index == 0 {
        return matrix_exp_pade13(&matrix.scale(scale))?.matvec(vector);
    }

    let n = matrix.nrows();
    let mut augmented = DenseMatrix::zeros(n + phi_index, n + phi_index);
    for i in 0..n {
        for j in 0..n {
            augmented[(i, j)] = scale * matrix[(i, j)];
        }
        augmented[(i, n)] = vector[i];
    }
    for j in 0..phi_index.saturating_sub(1) {
        augmented[(n + j, n + j + 1)] = 1.0;
    }
    let exponential = matrix_exp_pade13(&augmented)?;
    let target_column = n + phi_index - 1;
    let out: Vec<f64> = (0..n).map(|i| exponential[(i, target_column)]).collect();
    if out.iter().all(|value| value.is_finite()) {
        Ok(out)
    } else {
        Err(CoreError::NonFinite(
            "dense phi-action produced NaN/Inf".into(),
        ))
    }
}

/// Compute a fused linear combination
///
/// `exp(scale*A)b_0 + sum_{k=1}^p scale^k phi_k(scale*A)b_k`
///
/// with one augmented dense matrix exponential.  The coefficient ordering follows the
/// KIOPS/augmented-exponential convention `B=[b_p,...,b_1]`, while the lower Jordan chain is
/// seeded with its final basis vector.  This routine is the small projected-space oracle for the
/// matrix-free fused Krylov path; it is not used on the large physical state directly.
pub fn dense_fused_phi_action(
    matrix: &DenseMatrix,
    scale: f64,
    vectors: &[Vec<f64>],
) -> CoreResult<Vec<f64>> {
    if matrix.nrows() != matrix.ncols() || vectors.is_empty() {
        return Err(CoreError::Dimension(
            "dense fused phi-action requires a square matrix and at least b0".into(),
        ));
    }
    let n = matrix.nrows();
    if vectors.iter().any(|vector| vector.len() != n) {
        return Err(CoreError::Dimension(
            "dense fused phi-action vector shape mismatch".into(),
        ));
    }
    if !scale.is_finite()
        || !vectors
            .iter()
            .flat_map(|vector| vector.iter())
            .all(|value| value.is_finite())
    {
        return Err(CoreError::NonFinite(
            "dense fused phi-action input contains NaN/Inf".into(),
        ));
    }
    let p = vectors.len() - 1;
    if p == 0 {
        return matrix_exp_pade13(&matrix.scale(scale))?.matvec(&vectors[0]);
    }

    let mut augmented = DenseMatrix::zeros(n + p, n + p);
    for i in 0..n {
        for j in 0..n {
            augmented[(i, j)] = matrix[(i, j)];
        }
        for column in 0..p {
            // B = [b_p, b_{p-1}, ..., b_1].
            augmented[(i, n + column)] = vectors[p - column][i];
        }
    }
    for j in 0..p.saturating_sub(1) {
        augmented[(n + j, n + j + 1)] = 1.0;
    }
    let mut start = vec![0.0; n + p];
    start[..n].copy_from_slice(&vectors[0]);
    start[n + p - 1] = 1.0;
    let value = matrix_exp_pade13(&augmented.scale(scale))?.matvec(&start)?;
    let out = value[..n].to_vec();
    if out.iter().all(|value| value.is_finite()) {
        Ok(out)
    } else {
        Err(CoreError::NonFinite(
            "dense fused phi-action produced NaN/Inf".into(),
        ))
    }
}
