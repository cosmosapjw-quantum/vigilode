/// Dimensionless nonlinear remainder ratio.
///
/// ```text
/// ||f_i - f_n - J_n delta_i - c_i h f_t,n||_2
/// ------------------------------------------------
/// max(||f_i||_2, ||f_n||_2, ||J_n delta_i||_2,
///     ||c_i h f_t,n||_2, f64::MIN_POSITIVE)
/// ```
///
/// This reference is copied from the checksum-matched exploratory prototype
/// `VIGILODE_K0_TELEMETRY_CODING_LOOP_20260827.zip`, SHA-256
/// `04a3cd856501c0f7eda6c5f981e728999085fe4551d7acfcdbca6bfe1451700a`.
/// It freezes semantics only; it is not canonical integration evidence.
pub fn scaled_nonlinear_remainder_reference(
    rhs_value: &[f64],
    frozen_rhs_value: &[f64],
    frozen_jacobian_stage_increment: &[f64],
    time_derivative_increment: &[f64],
) -> Result<f64, ReferenceError> {
    let n = rhs_value.len();
    if n == 0
        || frozen_rhs_value.len() != n
        || frozen_jacobian_stage_increment.len() != n
        || time_derivative_increment.len() != n
    {
        return Err(ReferenceError::ShapeMismatch);
    }
    let mut remainder = vec![0.0; n];
    for i in 0..n {
        remainder[i] = rhs_value[i]
            - frozen_rhs_value[i]
            - frozen_jacobian_stage_increment[i]
            - time_derivative_increment[i];
    }
    let numerator = safe_l2(&remainder)?;
    let denominator = safe_l2(rhs_value)?
        .max(safe_l2(frozen_rhs_value)?)
        .max(safe_l2(frozen_jacobian_stage_increment)?)
        .max(safe_l2(time_derivative_increment)?)
        .max(f64::MIN_POSITIVE);
    Ok(numerator / denominator)
}
