//! Error metrics and matched-error interpolation.

/// Max over the grid of the weighted RMS of `y - y_ref` with weights
/// `atol_i + rtol |y_ref_i|`. `None` unless both trajectories cover the
/// same, nonempty grid.
pub fn max_wrms(
    states: &[Vec<f64>],
    reference: &[Vec<f64>],
    atol: &[f64],
    rtol: f64,
) -> Option<f64> {
    if states.is_empty() || states.len() != reference.len() {
        return None;
    }
    let mut worst: f64 = 0.0;
    for (y, r) in states.iter().zip(reference) {
        if y.len() != r.len() || y.len() != atol.len() {
            return None;
        }
        let n = y.len() as f64;
        let s: f64 = y
            .iter()
            .zip(r)
            .zip(atol)
            .map(|((a, b), at)| {
                let e = (a - b) / (at + rtol * b.abs());
                e * e
            })
            .sum();
        let w = (s / n).sqrt();
        if !w.is_finite() {
            return Some(f64::INFINITY);
        }
        worst = worst.max(w);
    }
    Some(worst)
}

/// [`max_wrms`] over the leading `states.len()` grid points only (for
/// failed runs); `None` when nothing was produced.
pub fn max_wrms_prefix(
    states: &[Vec<f64>],
    reference: &[Vec<f64>],
    atol: &[f64],
    rtol: f64,
) -> Option<f64> {
    if states.is_empty() || states.len() > reference.len() {
        return None;
    }
    max_wrms(states, &reference[..states.len()], atol, rtol)
}

/// Median of a nonempty sample (mean of the two middle values when even).
pub fn median(xs: &[f64]) -> f64 {
    let mut v = xs.to_vec();
    v.sort_by(f64::total_cmp);
    let k = v.len();
    if k % 2 == 1 {
        v[k / 2]
    } else {
        0.5 * (v[k / 2 - 1] + v[k / 2])
    }
}

/// Interpolate `log(cost)` linearly in `log(error)` on a work–precision
/// curve at `level`. `points` are `(error, cost)` with positive finite
/// entries; returns `None` unless `min(error) <= level <= max(error)`.
/// Points with equal error are averaged (in log cost).
pub fn interpolate_log(points: &[(f64, f64)], level: f64) -> Option<f64> {
    // (log error, log cost, cost) with the original cost kept so that a
    // level equal to a measured point returns that point's cost exactly.
    let mut p: Vec<(f64, f64, f64)> = points
        .iter()
        .copied()
        .filter(|(e, c)| e.is_finite() && *e > 0.0 && c.is_finite() && *c > 0.0)
        .map(|(e, c)| (e.ln(), c.ln(), c))
        .collect();
    if p.is_empty() || !(level.is_finite() && level > 0.0) {
        return None;
    }
    p.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64, usize, f64)> = Vec::new();
    for (e, lc, c) in p {
        match merged.last_mut() {
            Some(last) if last.0 == e => {
                last.1 += lc;
                last.2 += 1;
            }
            _ => merged.push((e, lc, 1, c)),
        }
    }
    let q: Vec<(f64, f64, Option<f64>)> = merged
        .into_iter()
        .map(|(e, lc, k, c)| (e, lc / k as f64, (k == 1).then_some(c)))
        .collect();
    let exact = |i: usize| q[i].2.unwrap_or_else(|| q[i].1.exp());
    let l = level.ln();
    if l < q[0].0 || l > q[q.len() - 1].0 {
        return None;
    }
    if let Some(i) = q.iter().position(|x| x.0 == l) {
        return Some(exact(i));
    }
    for w in q.windows(2) {
        let ((e0, c0, _), (e1, c1, _)) = (w[0], w[1]);
        if l > e0 && l < e1 {
            let s = (l - e0) / (e1 - e0);
            return Some((c0 + s * (c1 - c0)).exp());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrms_matches_hand_value() {
        let y = vec![vec![1.0, 2.0], vec![1.1, 2.0]];
        let r = vec![vec![1.0, 2.0], vec![1.0, 2.2]];
        let e = max_wrms(&y, &r, &[0.0, 0.0], 1.0).unwrap();
        // second point: (0.1/1, -0.2/2.2)
        let want = ((0.1f64.powi(2) + (0.2f64 / 2.2).powi(2)) / 2.0).sqrt();
        assert!((e - want).abs() < 1e-15);
        assert!(max_wrms(&y[..1], &r, &[0.0, 0.0], 1.0).is_none());
    }

    #[test]
    fn interpolation_brackets_only() {
        let pts = [(1e-4, 10.0), (1e-6, 100.0)];
        let mid = interpolate_log(&pts, 1e-5).unwrap();
        assert!((mid - (1000.0f64).sqrt()).abs() < 1e-9);
        assert_eq!(interpolate_log(&pts, 1e-4).unwrap(), 10.0);
        assert!(interpolate_log(&pts, 1e-7).is_none());
        assert!(interpolate_log(&pts, 1e-3).is_none());
        assert_eq!(interpolate_log(&[(1e-5, 3.0)], 1e-5).unwrap(), 3.0);
        assert!(interpolate_log(&[(1e-5, 3.0)], 2e-5).is_none());
    }

    #[test]
    fn median_even_and_odd() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), 2.5);
    }
}
