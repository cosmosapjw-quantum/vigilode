use crate::{CoreError, CoreResult};

pub fn safe_l2(x: &[f64]) -> f64 {
    if x.is_empty() {
        return 0.0;
    }
    let m = x.iter().fold(0.0_f64, |acc, &v| acc.max(v.abs()));
    if m == 0.0 {
        return 0.0;
    }
    if !m.is_finite() {
        return f64::INFINITY;
    }
    m * x.iter().map(|&v| (v / m).powi(2)).sum::<f64>().sqrt()
}

pub fn error_scale(y0: &[f64], y1: &[f64], atol: &[f64], rtol: f64) -> CoreResult<Vec<f64>> {
    if y0.len() != y1.len() || !(atol.len() == 1 || atol.len() == y0.len()) {
        return Err(CoreError::Dimension("error-scale shape mismatch".into()));
    }
    if rtol < 0.0 || !rtol.is_finite() {
        return Err(CoreError::InvalidInput(
            "rtol must be finite and nonnegative".into(),
        ));
    }
    let mut out = Vec::with_capacity(y0.len());
    for i in 0..y0.len() {
        let a = if atol.len() == 1 { atol[0] } else { atol[i] };
        let s = a + rtol * y0[i].abs().max(y1[i].abs());
        if !(s > 0.0 && s.is_finite()) {
            return Err(CoreError::InvalidInput("invalid error scale".into()));
        }
        out.push(s);
    }
    Ok(out)
}

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
