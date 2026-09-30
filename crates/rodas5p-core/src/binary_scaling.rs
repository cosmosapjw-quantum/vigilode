//! Products with powers that are formed on binary exponents, so a finite
//! result is not lost to an intermediate overflow or underflow (re-audit R2
//! of 2026-09-30: R2-POL-01, PHI-R1).

use crate::{CoreError, CoreResult};

/// `x = m * 2^e` with `m` in [1/2, 1), for finite `x > 0`. Any other `x`
/// (zero, negative, NaN, infinite) is returned unchanged with exponent 0,
/// so the callers' products propagate it.
pub fn binary_split(x: f64) -> (f64, i64) {
    if !(x.is_finite() && x > 0.0) {
        return (x, 0);
    }
    let (x, offset) = if x < f64::MIN_POSITIVE {
        (x * 2.0_f64.powi(64), -64)
    } else {
        (x, 0)
    };
    let bits = x.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = f64::from_bits((bits & !(0x7ff_u64 << 52)) | (1022_u64 << 52));
    (mantissa, biased - 1022 + offset)
}

/// `m * 2^e` for `m` in [1/8, 1], saturating to +inf above `f64::MAX` and
/// rounding once into the subnormal range.
pub fn binary_scale(m: f64, e: i64) -> f64 {
    if e > 1100 {
        return f64::INFINITY;
    }
    if e < -1200 {
        return 0.0;
    }
    let mut value = m;
    let mut e = e;
    while e > 1000 {
        value *= 2.0_f64.powi(1000);
        e -= 1000;
    }
    if e < -1000 {
        value *= 2.0_f64.powi(-1000);
        e += 1000;
    }
    value * 2.0_f64.powi(e as i32)
}

/// `|x|^k` as a mantissa in [1/2, 1) and a binary exponent, by squaring on
/// renormalized mantissas; `x` finite and nonzero (otherwise the plain
/// `powi` value with exponent 0). The relative rounding error grows like
/// `k eps`.
pub fn binary_power(x: f64, k: u32) -> (f64, i64) {
    if !(x.is_finite() && x != 0.0) {
        return (x.abs().powi(k.min(i32::MAX as u32) as i32), 0);
    }
    let (base_m, base_e) = binary_split(x.abs());
    let (mut square_m, mut square_e) = (base_m, base_e);
    let (mut acc_m, mut acc_e) = (1.0_f64, 0_i64);
    let mut remaining = k;
    while remaining > 0 {
        if remaining & 1 == 1 {
            let (m, e) = binary_split(acc_m * square_m);
            acc_m = m;
            acc_e += square_e + e;
        }
        remaining >>= 1;
        if remaining > 0 {
            let (m, e) = binary_split(square_m * square_m);
            square_m = m;
            square_e = 2 * square_e + e;
        }
    }
    if acc_m == 1.0 {
        (0.5, 1)
    } else {
        (acc_m, acc_e)
    }
}

/// `value * scale^k` in binary64 with no intermediate overflow or
/// underflow: a true result inside the binary64 range is returned with a
/// relative error of order `k eps` (a few roundings for the small `k` of the
/// phi conventions), one above it is +-inf, one below it is +-0. NaN or
/// infinite operands give the plain product.
pub fn times_power(value: f64, scale: f64, k: u32) -> f64 {
    if k == 0 {
        return value;
    }
    if !(value.is_finite() && scale.is_finite()) {
        return value * scale.powi(k.min(i32::MAX as u32) as i32);
    }
    if value == 0.0 || scale == 0.0 {
        let negative = value.is_sign_negative() ^ (scale.is_sign_negative() && k % 2 == 1);
        return if negative { -0.0 } else { 0.0 };
    }
    let (power_m, power_e) = binary_power(scale, k);
    let (value_m, value_e) = binary_split(value.abs());
    let magnitude = binary_scale(value_m * power_m, value_e + power_e);
    let negative = (value < 0.0) ^ (scale < 0.0 && k % 2 == 1);
    if negative { -magnitude } else { magnitude }
}

/// `w_k = scale^k b_k` for the phi-combination conventions. The weights are
/// formed by [`times_power`], so `scale = +-1e-100` with `b_4 = 1e300` gives
/// `1e-100` rather than `0 * 1e300` (re-audit R2, PHI-R1), and `scale =
/// 1e100` with `b_4 = 1e-300` gives `1e100` rather than an overflow.
///
/// A weight whose true value lies above `f64::MAX` is an error. A nonzero
/// input whose weight lies below the smallest subnormal becomes 0 and is
/// counted in the returned total; when every nonzero input is lost this way
/// the result is an error, so a combination of nonzero inputs never comes
/// back as an exact zero from the transform alone.
pub fn weight_phi_vectors(scale: f64, vectors: &[Vec<f64>]) -> CoreResult<(Vec<Vec<f64>>, usize)> {
    if !scale.is_finite() || !vectors.iter().flatten().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "phi weighting input contains NaN/Inf".into(),
        ));
    }
    let mut lost = 0_usize;
    let mut nonzero_inputs = 0_usize;
    let mut nonzero_weights = 0_usize;
    let mut weighted = Vec::with_capacity(vectors.len());
    for (k, vector) in vectors.iter().enumerate() {
        let k = u32::try_from(k)
            .map_err(|_| CoreError::InvalidInput("phi weighting order exceeds u32".into()))?;
        let mut row = Vec::with_capacity(vector.len());
        for &value in vector {
            let weight = times_power(value, scale, k);
            if !weight.is_finite() {
                return Err(CoreError::NonFinite(format!(
                    "phi weighting: scale^{k} b_{k} exceeds the binary64 range"
                )));
            }
            if value != 0.0 && scale != 0.0 {
                nonzero_inputs += 1;
                if weight == 0.0 {
                    lost += 1;
                }
            }
            if weight != 0.0 {
                nonzero_weights += 1;
            }
            row.push(weight);
        }
        weighted.push(row);
    }
    if nonzero_inputs > 0 && nonzero_weights == 0 {
        return Err(CoreError::InvalidInput(
            "phi weighting: every nonzero scale^k b_k underflows below the smallest subnormal"
                .into(),
        ));
    }
    Ok((weighted, lost))
}
