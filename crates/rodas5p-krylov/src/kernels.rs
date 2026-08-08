use rodas5p_core::{CoreError, CoreResult, WorkCounters, safe_l2};

pub fn dot(x: &[f64], y: &[f64], counters: &mut WorkCounters) -> CoreResult<f64> {
    if x.len() != y.len() {
        return Err(CoreError::Dimension("dot-product shape mismatch".into()));
    }
    counters.orthogonalization_inner_products += 1;
    Ok(x.iter().zip(y).map(|(a, b)| a * b).sum())
}

pub fn axpy(alpha: f64, x: &[f64], y: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
    if x.len() != y.len() {
        return Err(CoreError::Dimension("axpy shape mismatch".into()));
    }
    for (yi, xi) in y.iter_mut().zip(x) {
        *yi += alpha * xi;
    }
    counters.orthogonalization_vector_updates += 1;
    Ok(())
}

pub fn scale(alpha: f64, x: &mut [f64]) {
    for v in x {
        *v *= alpha;
    }
}

pub fn linear_combination_into(
    columns: &[Vec<f64>],
    coeff: &[f64],
    out: &mut [f64],
) -> CoreResult<()> {
    if columns.len() != coeff.len() {
        return Err(CoreError::Dimension(
            "linear-combination column mismatch".into(),
        ));
    }
    let n = columns.first().map_or(out.len(), Vec::len);
    if out.len() != n || columns.iter().any(|column| column.len() != n) {
        return Err(CoreError::Dimension("ragged columns".into()));
    }
    out.fill(0.0);
    for (column, &coefficient) in columns.iter().zip(coeff) {
        for i in 0..n {
            out[i] += coefficient * column[i];
        }
    }
    Ok(())
}

pub fn two_pass_mgs_into(
    w: &mut [f64],
    basis: &[Vec<f64>],
    coefficients: &mut [f64],
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    if coefficients.len() < basis.len() {
        return Err(CoreError::Dimension(
            "MGS coefficient scratch mismatch".into(),
        ));
    }
    if basis.is_empty() {
        return Ok(());
    }
    let n = w.len();
    if basis.iter().any(|vector| vector.len() != n) {
        return Err(CoreError::Dimension("MGS basis shape mismatch".into()));
    }
    coefficients[..basis.len()].fill(0.0);
    for _ in 0..2 {
        for (j, vector) in basis.iter().enumerate() {
            let coefficient = dot(vector, w, counters)?;
            coefficients[j] += coefficient;
            axpy(-coefficient, vector, w, counters)?;
        }
    }
    Ok(())
}

pub fn two_pass_mgs(
    w: &mut [f64],
    basis: &[Vec<f64>],
    counters: &mut WorkCounters,
) -> CoreResult<Vec<f64>> {
    if basis.is_empty() {
        return Ok(Vec::new());
    }
    let n = w.len();
    if basis.iter().any(|v| v.len() != n) {
        return Err(CoreError::Dimension("MGS basis shape mismatch".into()));
    }
    let mut h = vec![0.0; basis.len()];
    for _ in 0..2 {
        for (j, v) in basis.iter().enumerate() {
            let a = dot(v, w, counters)?;
            h[j] += a;
            axpy(-a, v, w, counters)?;
        }
    }
    Ok(h)
}

pub fn normalize(v: &mut [f64]) -> CoreResult<f64> {
    let n = safe_l2(v);
    if !n.is_finite() {
        return Err(CoreError::NonFinite(
            "cannot normalize non-finite vector".into(),
        ));
    }
    if n > f64::MIN_POSITIVE {
        scale(1.0 / n, v);
    }
    Ok(n)
}
