use crate::{CoreError, CoreResult, DenseMatrix, LuFactorization};

const PADE_13_THETA: f64 = 5.371_920_351_148_152;
/// Above this 1-norm the unscaled sixth power could overflow, and the
/// squaring count falls back to the 1-norm alone.
const PADE_13_POWER_GUARD: f64 = 1.0e40;
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
    let identity = DenseMatrix::identity(n);
    let (squarings, scaled, a2, a4, a6) = if norm <= PADE_13_POWER_GUARD {
        // Squaring count from alpha_p = max(d_p, d_{p+1}), d_k >= ||A^k||^(1/k),
        // minimised over p(p-1) <= 27 (Al-Mohy & Higham 2009, Thm 4.2), with
        // d_3 and d_5 bounded by (||A^2|| ||A||)^(1/3), (||A^4|| ||A||)^(1/5).
        // For the nilpotent augmented chains of the fused phi combination,
        // alpha << ||A||_1 and the 1-norm alone over-squares (audit F-042).
        // The powers are formed once, unscaled, and rescaled by exact powers
        // of two, so no extra products are needed.
        let a2 = a.matmul(a)?;
        let a4 = a2.matmul(&a2)?;
        let a6 = a4.matmul(&a2)?;
        let (n2, n4, n6) = (
            matrix_one_norm(&a2),
            matrix_one_norm(&a4),
            matrix_one_norm(&a6),
        );
        let d2 = n2.sqrt();
        let d3 = (n2 * norm).powf(1.0 / 3.0);
        let d4 = n4.powf(0.25);
        let d5 = (n4 * norm).powf(0.2);
        let d6 = n6.powf(1.0 / 6.0);
        let alpha = [d2.max(d3), d3.max(d4), d4.max(d5), d5.max(d6)]
            .into_iter()
            .fold(norm, f64::min);
        let squarings = if alpha <= PADE_13_THETA || alpha == 0.0 {
            0_i32
        } else {
            (alpha / PADE_13_THETA).log2().ceil().max(0.0) as i32
        };
        let two_pow = |k: i32| 2.0_f64.powi(-k);
        (
            squarings as u32,
            a.scale(two_pow(squarings)),
            a2.scale(two_pow(2 * squarings)),
            a4.scale(two_pow(4 * squarings)),
            a6.scale(two_pow(6 * squarings)),
        )
    } else {
        // Unscaled sixth powers could overflow: scale by the 1-norm first.
        let squarings = (norm / PADE_13_THETA).log2().ceil().max(0.0) as u32;
        let scaled = a.scale(2.0_f64.powi(-(squarings as i32)));
        let a2 = scaled.matmul(&scaled)?;
        let a4 = a2.matmul(&a2)?;
        let a6 = a4.matmul(&a2)?;
        (squarings, scaled, a2, a4, a6)
    };
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
    // `phi_k(scale*A) v` is linear in `v`, but the Pade squaring count follows
    // the 1-norm of the augmented matrix, which contains `v`. An unscaled
    // column made accuracy depend on the physical scale of `v` (audit F-040,
    // E-06: 1e-8 relative error at ||v|| = 1e8). The column is scaled by a
    // power of two to max-norm near 1, which is exact, and undone on the
    // result. The exponent is clamped to the normal range so the scale itself
    // is always finite: `ceil(log2(1e308))` is 1024 (external audit VIG-A06).
    let vector_max = vector.iter().fold(0.0_f64, |m, value| m.max(value.abs()));
    if vector_max == 0.0 {
        return Ok(vec![0.0; n]);
    }
    let exponent = (vector_max.log2().ceil() as i64).clamp(-1022, 1023);
    let vector_scale = f64::from_bits(((exponent + 1023) as u64) << 52);
    let mut augmented = DenseMatrix::zeros(n + phi_index, n + phi_index);
    for i in 0..n {
        for j in 0..n {
            augmented[(i, j)] = scale * matrix[(i, j)];
        }
        augmented[(i, n)] = vector[i] / vector_scale;
    }
    for j in 0..phi_index.saturating_sub(1) {
        augmented[(n + j, n + j + 1)] = 1.0;
    }
    let exponential = matrix_exp_pade13(&augmented)?;
    let target_column = n + phi_index - 1;
    let out: Vec<f64> = (0..n)
        .map(|i| vector_scale * exponential[(i, target_column)])
        .collect();
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
///
/// The inputs are weighted to `w_k = scale^k b_k` and passed to
/// [`dense_phi_combination`], which never forms `scale^-k` (audit
/// 2026-09-30, PHI-P2).
///
/// A nonzero input whose weight underflows to 0 makes this a typed error
/// (TRANSFORM_ERROR_UNBOUNDED, re-audit R3, R3-ARITH-01): the value would be
/// the action of a different, rounded problem. [`dense_fused_phi_action_report`]
/// returns the value with the loss recorded instead.
pub fn dense_fused_phi_action(
    matrix: &DenseMatrix,
    scale: f64,
    vectors: &[Vec<f64>],
) -> CoreResult<Vec<f64>> {
    let report = dense_fused_phi_action_report(matrix, scale, vectors)?;
    if report.weight_underflows > 0 {
        return Err(CoreError::InvalidInput(format!(
            "TRANSFORM_ERROR_UNBOUNDED: {} nonzero phi input(s) underflowed to zero weight",
            report.weight_underflows
        )));
    }
    Ok(report.value)
}

/// [`dense_fused_phi_action`] with the amplitude state and the number of
/// inputs whose weight underflowed (re-audit R3, R3-ARITH-01/03).
pub fn dense_fused_phi_action_report(
    matrix: &DenseMatrix,
    scale: f64,
    vectors: &[Vec<f64>],
) -> CoreResult<DensePhiCombinationReport> {
    if !scale.is_finite() {
        return Err(CoreError::NonFinite(
            "dense fused phi-action input contains NaN/Inf".into(),
        ));
    }
    let (weighted, lost) = crate::weight_phi_vectors(scale, vectors)?;
    let mut report = dense_phi_combination_report(matrix, scale, &weighted)?;
    report.weight_underflows = u64::try_from(lost).unwrap_or(u64::MAX);
    Ok(report)
}

/// `exp(scale*A) w_0 + sum_{k=1}^p phi_k(scale*A) w_k` with one dense
/// exponential of the time-normalized augmented matrix
///
/// `M_hat = [[scale*A, [w_p, ..., w_1]], [0, J_p]]`, start `[w_0; e_p]`.
///
/// This is `D^-1 (scale M) D` for `D = diag(I, scale^(p-1), ..., scale, 1)`
/// and the unnormalized `M = [[A, [b_p, ..., b_1]], [0, J_p]]` with
/// `b_k = w_k / scale^k`; `D` fixes the start and the physical projection,
/// so the result is the same exactly. The unnormalized matrix grew like
/// `scale^-p` as `scale -> 0`: its 1-norm passed the Pade power guard near
/// `scale = 1e-14` and the result was 0 instead of about 2.36 for the
/// scalar `A = -1` case of the 2026-09-30 audit. `scale = 0` is valid and
/// returns `w_0 + sum_k w_k / k!`.
pub fn dense_phi_combination(
    matrix: &DenseMatrix,
    scale: f64,
    weighted: &[Vec<f64>],
) -> CoreResult<Vec<f64>> {
    dense_phi_combination_report(matrix, scale, weighted).map(|report| report.value)
}

/// Amplitude state of one dense phi combination (re-audit R2, PHI-R2).
#[derive(Clone, Debug, PartialEq)]
pub struct DensePhiCombinationReport {
    pub value: Vec<f64>,
    /// `e` in the exact normalization `w / 2^e` that brings the largest
    /// weight into [1/2, 1) before the exponential; the value is scaled
    /// back by `2^e`. 0 when every weight is 0.
    pub amplitude_exponent: i64,
    /// log2 of the largest over the smallest nonzero weight magnitude.
    pub weight_dynamic_range_log2: f64,
    /// The smallest nonzero weights lie below the rounding unit of the
    /// largest (dynamic range above 2^53): their contribution is resolved
    /// only normwise, relative to the largest weight.
    pub mixed_range: bool,
    /// The output amplitude is below 2^-26 of the largest weight
    /// (cancellation or strong decay): the value is accurate relative to
    /// the weights, and its own relative accuracy is not established.
    pub output_below_input_half_precision: bool,
    /// Nonzero inputs whose weight `scale^k b_k` underflowed to 0 before the
    /// combination (only from [`dense_fused_phi_action_report`]).
    pub weight_underflows: u64,
}

impl DensePhiCombinationReport {
    /// The value may serve as a precision reference for its own entries:
    /// no input was lost to weighting, the weights span less than the
    /// binary64 precision, and the output is not dominated by cancellation
    /// (re-audit R3, R3-ARITH-03). A consumer that needs a reference must
    /// treat anything else as not evaluated unless an independent oracle
    /// resolves it.
    pub fn is_reference_authoritative(&self) -> bool {
        self.weight_underflows == 0 && !self.mixed_range && !self.output_below_input_half_precision
    }
}

/// [`dense_phi_combination`] with its amplitude state.
///
/// The combination is linear in the weights, so it is evaluated on
/// `w / 2^e`, with `2^e` the binary order of the largest weight, and scaled
/// back; both scalings are exact apart from under- or overflow. Without the
/// normalization the weights sat unscaled in the augmented matrix, whose
/// 1-norm then drove the Pade scaling: with `A = -1`, `h = 0.1` and
/// `w_1 = c`, the relative error was 6.8e-10 at `c = 1e20`, 0.043 at
/// `c = 1e40`, and the result was exactly 0 at `c = 1e60`.
pub fn dense_phi_combination_report(
    matrix: &DenseMatrix,
    scale: f64,
    weighted: &[Vec<f64>],
) -> CoreResult<DensePhiCombinationReport> {
    if matrix.nrows() != matrix.ncols() || weighted.is_empty() {
        return Err(CoreError::Dimension(
            "dense fused phi-action requires a square matrix and at least b0".into(),
        ));
    }
    let n = matrix.nrows();
    if weighted.iter().any(|vector| vector.len() != n) {
        return Err(CoreError::Dimension(
            "dense fused phi-action vector shape mismatch".into(),
        ));
    }
    if !scale.is_finite()
        || !weighted
            .iter()
            .flat_map(|vector| vector.iter())
            .all(|value| value.is_finite())
    {
        return Err(CoreError::NonFinite(
            "dense fused phi-action input contains NaN/Inf".into(),
        ));
    }
    let magnitudes = weighted
        .iter()
        .flatten()
        .map(|value| value.abs())
        .filter(|value| *value > 0.0);
    let (largest, smallest) = magnitudes.fold((0.0_f64, f64::INFINITY), |(hi, lo), value| {
        (hi.max(value), lo.min(value))
    });
    if largest == 0.0 {
        return Ok(DensePhiCombinationReport {
            value: vec![0.0; n],
            amplitude_exponent: 0,
            weight_dynamic_range_log2: 0.0,
            mixed_range: false,
            output_below_input_half_precision: false,
            weight_underflows: 0,
        });
    }
    let (_, amplitude_exponent) = crate::binary_split(largest);
    let normalized = weighted
        .iter()
        .map(|vector| {
            vector
                .iter()
                .map(|value| times_power_of_two(*value, -amplitude_exponent))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let unit = dense_phi_combination_normalized(matrix, scale, &normalized)?;
    let value = unit
        .iter()
        .map(|value| times_power_of_two(*value, amplitude_exponent))
        .collect::<Vec<_>>();
    if !value.iter().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "dense fused phi-action produced NaN/Inf".into(),
        ));
    }
    let dynamic_range = largest.log2() - smallest.log2();
    let output_unit = unit.iter().fold(0.0_f64, |acc, value| acc.max(value.abs()));
    Ok(DensePhiCombinationReport {
        value,
        amplitude_exponent,
        weight_dynamic_range_log2: dynamic_range,
        mixed_range: dynamic_range > 53.0,
        output_below_input_half_precision: output_unit < 2.0_f64.powi(-26),
        weight_underflows: 0,
    })
}

/// `x * 2^e`, in steps that stay in the normal range until the last one.
fn times_power_of_two(x: f64, e: i64) -> f64 {
    let mut x = x;
    let mut e = e;
    while e > 1000 {
        x *= 2.0_f64.powi(1000);
        e -= 1000;
    }
    while e < -1000 {
        x *= 2.0_f64.powi(-1000);
        e += 1000;
    }
    x * 2.0_f64.powi(e as i32)
}

/// The dense combination for weights whose largest magnitude is O(1).
fn dense_phi_combination_normalized(
    matrix: &DenseMatrix,
    scale: f64,
    weighted: &[Vec<f64>],
) -> CoreResult<Vec<f64>> {
    let n = matrix.nrows();
    let p = weighted.len() - 1;
    if p == 0 {
        return matrix_exp_pade13(&matrix.scale(scale))?.matvec(&weighted[0]);
    }

    let mut augmented = DenseMatrix::zeros(n + p, n + p);
    for i in 0..n {
        for j in 0..n {
            augmented[(i, j)] = scale * matrix[(i, j)];
        }
        for column in 0..p {
            // W = [w_p, w_{p-1}, ..., w_1].
            augmented[(i, n + column)] = weighted[p - column][i];
        }
    }
    for j in 0..p.saturating_sub(1) {
        augmented[(n + j, n + j + 1)] = 1.0;
    }
    let mut start = vec![0.0; n + p];
    start[..n].copy_from_slice(&weighted[0]);
    start[n + p - 1] = 1.0;
    let value = matrix_exp_pade13(&augmented)?.matvec(&start)?;
    let out = value[..n].to_vec();
    if out.iter().all(|value| value.is_finite()) {
        Ok(out)
    } else {
        Err(CoreError::NonFinite(
            "dense fused phi-action produced NaN/Inf".into(),
        ))
    }
}
