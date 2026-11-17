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

/// Two-pass Classical Gram–Schmidt orthogonalization (CGS2).
///
/// Computes `h1 = V^T w`, `w = w - V h1`, followed by `h2 = V^T w`, `w = w - V h2`.
/// Delivers level-2 BLAS gemv structure while matching MGS2 numerical orthogonality
/// and preserving identical WorkCounters (WP-08).
#[allow(dead_code)]
pub fn two_pass_cgs_into(
    w: &mut [f64],
    basis: &[Vec<f64>],
    coefficients: &mut [f64],
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    if coefficients.len() < basis.len() {
        return Err(CoreError::Dimension(
            "CGS coefficient scratch mismatch".into(),
        ));
    }
    if basis.is_empty() {
        return Ok(());
    }
    let n = w.len();
    if basis.iter().any(|vector| vector.len() != n) {
        return Err(CoreError::Dimension("CGS basis shape mismatch".into()));
    }

    let j = basis.len();
    coefficients[..j].fill(0.0);

    // Pass 1: Project w onto basis
    for k in 0..j {
        let alpha = dot(&basis[k], w, counters)?;
        coefficients[k] += alpha;
    }
    for k in 0..j {
        axpy(-coefficients[k], &basis[k], w, counters)?;
    }

    // Pass 2: Re-orthogonalize w against basis
    for k in 0..j {
        let alpha = dot(&basis[k], w, counters)?;
        coefficients[k] += alpha;
        axpy(-alpha, &basis[k], w, counters)?;
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

#[cfg(test)]
mod cgs2_tests {
    use super::*;

    #[test]
    fn test_two_pass_cgs_into_orthogonality_and_counter_parity() {
        let mut counters_mgs = WorkCounters::default();
        let mut counters_cgs = WorkCounters::default();

        let v1 = vec![1.0, 0.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0, 0.0];
        let basis = vec![v1, v2];

        let mut w_mgs = vec![1.0, 2.0, 3.0, 4.0];
        let mut w_cgs = w_mgs.clone();
        let mut coeff_mgs = [0.0; 2];
        let mut coeff_cgs = [0.0; 2];

        two_pass_mgs_into(&mut w_mgs, &basis, &mut coeff_mgs, &mut counters_mgs).unwrap();
        two_pass_cgs_into(&mut w_cgs, &basis, &mut coeff_cgs, &mut counters_cgs).unwrap();

        // Check work counter parity
        assert_eq!(
            counters_mgs.orthogonalization_inner_products,
            counters_cgs.orthogonalization_inner_products
        );
        assert_eq!(
            counters_mgs.orthogonalization_vector_updates,
            counters_cgs.orthogonalization_vector_updates
        );
        assert_eq!(counters_cgs.orthogonalization_inner_products, 4); // 2 passes * 2 vectors
        assert_eq!(counters_cgs.orthogonalization_vector_updates, 4);

        // Check numerical orthogonality: V^T w < 1e-15
        for v in &basis {
            let mut c = WorkCounters::default();
            assert!(dot(v, &w_cgs, &mut c).unwrap().abs() < 1e-15);
        }
    }
}
