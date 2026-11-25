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
    if states.is_empty()
        || states.len() != reference.len()
        || atol.is_empty()
        || !rtol.is_finite()
        || rtol < 0.0
        || atol.iter().any(|a| !a.is_finite() || *a < 0.0)
    {
        return None;
    }
    let mut worst: f64 = 0.0;
    for (y, r) in states.iter().zip(reference) {
        if y.len() != r.len() || y.len() != atol.len() {
            return None;
        }
        let sqrt_n = (y.len() as f64).sqrt();
        let mut norm = 0.0_f64;
        for ((a, b), at) in y.iter().zip(r).zip(atol) {
            if !(a.is_finite() && b.is_finite()) {
                return Some(f64::INFINITY);
            }
            let scale = at + rtol * b.abs();
            // Overflowing weights used to turn finite errors into zero.
            // An unrepresentable/nonpositive metric is unavailable, not a pass.
            if !(scale.is_finite() && scale > 0.0) {
                return None;
            }
            let difference = a - b;
            let e = if difference.is_finite() {
                difference / scale
            } else {
                // Opposite-sign finite states can overflow their subtraction
                // although the normalized difference is representable.
                a / scale - b / scale
            };
            // Divide by sqrt(n) first only when the original component
            // normalization overflowed. The final RMS can still be finite.
            let normalized = if e.is_finite() {
                e / sqrt_n
            } else if difference.is_finite() {
                (difference / sqrt_n) / scale
            } else {
                (a / sqrt_n) / scale - (b / sqrt_n) / scale
            };
            norm = norm.hypot(normalized);
        }
        let w = norm;
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
    // Compare represented errors, not their rounded logarithms. Adjacent
    // binary64 errors can have the same ln() even when genuinely distinct.
    let mut p: Vec<(f64, f64)> = points
        .iter()
        .copied()
        .filter(|(e, c)| e.is_finite() && *e > 0.0 && c.is_finite() && *c > 0.0)
        .collect();
    if p.is_empty() || !(level.is_finite() && level > 0.0) {
        return None;
    }
    p.sort_by(|a, b| a.0.total_cmp(&b.0));
    if level < p[0].0 || level > p[p.len() - 1].0 {
        return None;
    }
    // Merge only truly equal errors. Keep bounds on the original costs to
    // avoid overflow or underflow in a rounded exp of a log average.
    let mut q: Vec<(f64, f64, usize, f64, f64)> = Vec::new();
    for (e, c) in p {
        match q.last_mut() {
            Some(last) if last.0 == e => {
                last.2 += 1;
                last.1 += (c.ln() - last.1) / (last.2 as f64);
                last.3 = last.3.min(c);
                last.4 = last.4.max(c);
            }
            _ => q.push((e, c.ln(), 1, c, c)),
        }
    }
    let exact = |i: usize| {
        if q[i].2 == 1 {
            q[i].3
        } else {
            q[i].1.exp().clamp(q[i].3, q[i].4)
        }
    };
    if let Some(i) = q.iter().position(|r| r.0 == level) {
        return Some(exact(i));
    }
    fn log_distance(a: f64, b: f64) -> f64 {
        let d = (b - a) / a;
        if d.is_finite() && d < 0.5 {
            d.ln_1p()
        } else {
            b.ln() - a.ln()
        }
    }
    for (i, w) in q.windows(2).enumerate() {
        let (e0, c0, _, _, _) = w[0];
        let (e1, c1, _, _, _) = w[1];
        if e0 < level && level < e1 {
            let t = log_distance(e0, level) / log_distance(e0, e1);
            if !t.is_finite() || !(0. ..=1.).contains(&t) {
                return None;
            }
            let a = exact(i);
            let b = exact(i + 1);
            let value = (c0 + t * (c1 - c0)).exp().clamp(a.min(b), a.max(b));
            return (value.is_finite() && value > 0.).then_some(value);
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

    #[test]
    fn invalid_metric_cannot_be_reported_as_zero_error() {
        assert_eq!(max_wrms(&[vec![]], &[vec![]], &[], 1.0), None);
        assert_eq!(max_wrms(&[vec![1.0]], &[vec![2.0]], &[-1.0], 1.0), None);
        assert_eq!(max_wrms(&[vec![1.0]], &[vec![2.0]], &[1.0], f64::NAN), None);
        assert_eq!(max_wrms(&[vec![1.0]], &[vec![f64::MAX]], &[1.0], 2.0), None);
        assert_eq!(max_wrms(&[vec![0.0]], &[vec![0.0]], &[0.0], 1.0), None);
        assert_eq!(
            max_wrms(&[vec![f64::NAN]], &[vec![0.0]], &[1.0], 1.0),
            Some(f64::INFINITY)
        );
    }

    #[test]
    fn representable_large_normalized_errors_remain_finite() {
        assert_eq!(
            max_wrms(&[vec![f64::MAX]], &[vec![-f64::MAX]], &[f64::MAX], 0.0),
            Some(2.0)
        );
        let e = max_wrms(
            &[vec![1.0e200, 1.0e200]],
            &[vec![0.0, 0.0]],
            &[1.0, 1.0],
            0.0,
        )
        .unwrap();
        assert!(e.is_finite() && (e / 1.0e200 - 1.0).abs() < 3.0e-16);
    }
}

#[cfg(test)]
mod preflight_adversarial {
    use super::*;
    #[test]
    fn adjacent_binary64_errors_are_not_equal_or_extrapolated() {
        let e = 1e-300_f64;
        let lo = e.next_down();
        let hi = e.next_up();
        assert_eq!(lo.ln(), e.ln());
        assert_eq!(hi.ln(), e.ln());
        assert_eq!(interpolate_log(&[(e, 2.)], lo), None);
        assert_eq!(interpolate_log(&[(e, 2.), (hi, 8.)], e), Some(2.));
        assert_eq!(interpolate_log(&[(e, 2.), (hi, 8.)], hi), Some(8.));
        assert!((interpolate_log(&[(lo, 2.), (hi, 8.)], e).unwrap() - 4.).abs() < 4e-15);
    }
    #[test]
    fn normalized_rms_can_be_finite_when_component_ratio_overflows() {
        let got = max_wrms(&[vec![1e308, 0., 0., 0.]], &[vec![0.; 4]], &[0.5; 4], 0.).unwrap();
        assert_eq!(got, 1e308);
        let got = max_wrms(
            &[vec![f64::MAX, 0., 0., 0.]],
            &[vec![-f64::MAX, 0., 0., 0.]],
            &[1.; 4],
            0.,
        )
        .unwrap();
        assert_eq!(got, f64::MAX);
    }
}
