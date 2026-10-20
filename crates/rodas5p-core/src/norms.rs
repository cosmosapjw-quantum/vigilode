use crate::{CoreError, CoreResult};

/// Scaled Euclidean norm. Any NaN entry gives NaN and any infinite entry
/// gives infinity, so callers' `is_finite` checks fail closed. (`f64::max`
/// ignores a NaN operand, which made `[NaN, NaN]` a zero norm; external audit
/// VIG-A04.)
pub fn safe_l2(x: &[f64]) -> f64 {
    if x.is_empty() {
        return 0.0;
    }
    let mut m = 0.0_f64;
    for &v in x {
        let a = v.abs();
        if a.is_nan() {
            return f64::NAN;
        }
        if a > m {
            m = a;
        }
    }
    if m == 0.0 {
        return 0.0;
    }
    if !m.is_finite() {
        return f64::INFINITY;
    }
    m * x.iter().map(|&v| (v / m).powi(2)).sum::<f64>().sqrt()
}

/// One component `error / scale` of a weighted error norm under the
/// zero-scale rule shared by every driver (production plan 2026-10-20, A1):
/// a component whose scale is zero (`atol = 0` and the component is zero at
/// both ends of the step) contributes 0 when its error is exactly zero
/// (either sign), and +inf when its error is nonzero or NaN, so the norm
/// rejects the step and the controller shrinks it. A positive scale gives
/// exactly `error / scale`; the caller squares or takes the absolute value.
#[inline]
pub fn scaled_error_component(error: f64, scale: f64) -> f64 {
    if scale == 0.0 {
        if error == 0.0 { 0.0 } else { f64::INFINITY }
    } else {
        error / scale
    }
}

/// Per-component scale `atol_i + rtol max(|y0_i|, |y1_i|)`. A zero scale is
/// valid here and is resolved by [`step_error_wrms`] (end-of-step norms) or
/// refused by [`wrms`] (iteration-convergence norms); a scale that overflows
/// is a typed error.
pub fn error_scale(y0: &[f64], y1: &[f64], atol: &[f64], rtol: f64) -> CoreResult<Vec<f64>> {
    if y0.len() != y1.len() || !(atol.len() == 1 || atol.len() == y0.len()) {
        return Err(CoreError::Dimension("error-scale shape mismatch".into()));
    }
    if rtol < 0.0 || !rtol.is_finite() {
        return Err(CoreError::InvalidInput(
            "rtol must be finite and nonnegative".into(),
        ));
    }
    // Each input is checked on its own (audit 2026-09-30, CERT-02):
    // `abs().max()` drops a NaN on one side, and a negative atol can still
    // give a positive scale when the relative term is large.
    if !atol.iter().all(|value| value.is_finite() && *value >= 0.0) {
        return Err(CoreError::InvalidInput(
            "atol must be finite and nonnegative".into(),
        ));
    }
    if !y0.iter().chain(y1).all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "error-scale state contains NaN/Inf".into(),
        ));
    }
    let mut out = Vec::with_capacity(y0.len());
    for i in 0..y0.len() {
        let a = if atol.len() == 1 { atol[0] } else { atol[i] };
        let s = a + rtol * y0[i].abs().max(y1[i].abs());
        if !(s >= 0.0 && s.is_finite()) {
            return Err(CoreError::InvalidInput("invalid error scale".into()));
        }
        out.push(s);
    }
    Ok(out)
}

/// Weighted RMS norm with a strictly positive scale (the pre-wave contract,
/// kept by controller ruling on final-review item 5). Iteration-convergence
/// callers (Newton convergence, inner forcing, SABR residual estimates,
/// certification, Krylov weighted residuals) measure a contraction in this
/// norm, not a step acceptance, so a zero scale stays a typed
/// `InvalidInput`. End-of-step error norms use [`step_error_wrms`], which
/// carries the zero-scale rule.
pub fn wrms(error: &[f64], scale: &[f64]) -> CoreResult<f64> {
    if error.len() != scale.len() || error.is_empty() {
        return Err(CoreError::Dimension(
            "WRMS shape mismatch or empty vector".into(),
        ));
    }
    let mut sum = 0.0;
    for (&e, &s) in error.iter().zip(scale) {
        if !(s > 0.0 && s.is_finite()) {
            return Err(CoreError::InvalidInput(
                "WRMS scale must be finite and positive".into(),
            ));
        }
        let z = e / s;
        if !z.is_finite() {
            return Ok(f64::INFINITY);
        }
        sum += z * z;
    }
    Ok((sum / error.len() as f64).sqrt())
}

/// End-of-step (step-acceptance) error norm: the weighted RMS norm with a
/// zero scale resolved by [`scaled_error_component`] (production plan
/// 2026-10-20, A1). A zero-scale component with a zero error contributes 0;
/// with a nonzero or NaN error the norm is +inf, so the step is rejected. A
/// negative or non-finite scale is a typed error. On a positive scale the
/// result is bit-identical to [`wrms`].
pub fn step_error_wrms(error: &[f64], scale: &[f64]) -> CoreResult<f64> {
    if error.len() != scale.len() || error.is_empty() {
        return Err(CoreError::Dimension(
            "WRMS shape mismatch or empty vector".into(),
        ));
    }
    let mut sum = 0.0;
    for (&e, &s) in error.iter().zip(scale) {
        if !(s >= 0.0 && s.is_finite()) {
            return Err(CoreError::InvalidInput(
                "WRMS scale must be finite and nonnegative".into(),
            ));
        }
        let z = scaled_error_component(e, s);
        if !z.is_finite() {
            return Ok(f64::INFINITY);
        }
        sum += z * z;
    }
    Ok((sum / error.len() as f64).sqrt())
}

#[cfg(test)]
mod tests {
    use super::scaled_error_component;

    #[test]
    fn zero_scale_rule() {
        assert_eq!(scaled_error_component(0.0, 0.0).to_bits(), 0);
        assert_eq!(scaled_error_component(-0.0, 0.0).to_bits(), 0);
        assert_eq!(scaled_error_component(0.0, -0.0).to_bits(), 0);
        assert_eq!(scaled_error_component(1.0e-300, 0.0), f64::INFINITY);
        assert_eq!(scaled_error_component(-1.0, 0.0), f64::INFINITY);
        assert_eq!(scaled_error_component(f64::NAN, 0.0), f64::INFINITY);
        let (e, s) = (3.0e-7, 7.0e-6);
        assert_eq!(scaled_error_component(e, s).to_bits(), (e / s).to_bits());
        assert_eq!(scaled_error_component(-e, s).to_bits(), (-e / s).to_bits());
    }
}
