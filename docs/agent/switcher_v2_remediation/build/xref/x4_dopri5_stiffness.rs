//! Secant stiffness estimate from one DOPRI5 trial (pure, no RHS calls).

/// Exact coefficients of `Y7 - Y6 = h * sum_j W[j] * k_j` (fifth-order output
/// minus the sixth stage state; both at `t + h`). The weights sum to zero.
pub const DOPRI5_Y7_MINUS_Y6: [f64; 6] = [
    -34913.0 / 12672.0,
    355.0 / 33.0,
    -310624.0 / 36729.0,
    787.0 / 2112.0,
    -3645.0 / 74624.0,
    11.0 / 84.0,
];

/// Magnitude of the point where the DOPRI5 stability region meets the
/// negative real axis: R(x) = -1 at x = -3.306568... (25-digit Wolfram root,
/// rounded down so the threshold below is never above the true boundary).
pub const DOPRI5_REAL_AXIS_BOUNDARY: f64 = 3.306_568;

/// Secant estimate `h * q` with `q = <d, g> / <d, d>`, `d = Y7 - Y6`,
/// `g = k7 - k6 = f(t+h, Y7) - f(t+h, Y6)`. `stages` holds the seven stage
/// derivatives of one trial, `k_j = stages[j*n..(j+1)*n]`.
/// Returns `None` when `n == 0`, the slice is too short, `d` vanishes or a
/// value is not finite.
pub fn dopri5_secant_h_lambda(h: f64, stages: &[f64], n: usize) -> Option<f64> {
    if n == 0 || stages.len() < 7 * n || !(h.is_finite() && h > 0.0) {
        return None;
    }
    let (mut dd, mut dg) = (0.0f64, 0.0f64);
    for i in 0..n {
        let mut d = 0.0;
        for (j, &w) in DOPRI5_Y7_MINUS_Y6.iter().enumerate() {
            d += w * stages[j * n + i];
        }
        d *= h;
        let g = stages[6 * n + i] - stages[5 * n + i];
        dd += d * d;
        dg += d * g;
    }
    if !(dd.is_finite() && dg.is_finite()) || dd == 0.0 {
        return None;
    }
    let h_lambda = h * dg / dd;
    h_lambda.is_finite().then_some(h_lambda)
}

/// True when the secant estimate is dissipative and at least `fraction` of the
/// real-axis stability boundary (`fraction = 0.8` in the preregistered gate).
pub fn dopri5_stability_limited(h_lambda: f64, fraction: f64) -> bool {
    h_lambda < 0.0 && -h_lambda >= fraction * DOPRI5_REAL_AXIS_BOUNDARY
}

#[cfg(test)]
mod tests {
    use super::*;
    const A: [[f64; 7]; 7] = [
        [0.0; 7],
        [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0, 0.0],
        [
            19372.0 / 6561.0,
            -25360.0 / 2187.0,
            64448.0 / 6561.0,
            -212.0 / 729.0,
            0.0,
            0.0,
            0.0,
        ],
        [
            9017.0 / 3168.0,
            -355.0 / 33.0,
            46732.0 / 5247.0,
            49.0 / 176.0,
            -5103.0 / 18656.0,
            0.0,
            0.0,
        ],
        [
            35.0 / 384.0,
            0.0,
            500.0 / 1113.0,
            125.0 / 192.0,
            -2187.0 / 6784.0,
            11.0 / 84.0,
            0.0,
        ],
    ];

    /// Stage derivatives of one DOPRI5 trial for the linear system y' = diag(lam) y.
    fn linear_stages(h: f64, y: &[f64], lam: &[f64]) -> Vec<f64> {
        let n = y.len();
        let mut k = vec![0.0; 7 * n];
        for s in 0..7 {
            for i in 0..n {
                let mut yi = y[i];
                for j in 0..s {
                    yi += h * A[s][j] * k[j * n + i];
                }
                k[s * n + i] = lam[i] * yi;
            }
        }
        k
    }

    #[test]
    fn weights_equal_tableau_difference_and_sum_to_zero() {
        let mut sum = 0.0;
        for j in 0..6 {
            let diff = A[6][j] - A[5][j];
            assert!(
                (DOPRI5_Y7_MINUS_Y6[j] - diff).abs() <= 4.0 * f64::EPSILON * diff.abs().max(1.0)
            );
            sum += DOPRI5_Y7_MINUS_Y6[j];
        }
        assert!(sum.abs() < 1e-14);
    }

    #[test]
    fn scalar_linear_mode_is_recovered() {
        for (h, lam) in [(1e-3, -1.0e4), (3.2e-4, -1.0e4), (0.01, -2.0), (0.01, 3.0)] {
            let k = linear_stages(h, &[1.0], &[lam]);
            let hl = dopri5_secant_h_lambda(h, &k, 1).expect("valid test input");
            assert!(
                (hl - h * lam).abs() <= 1e-9 * (h * lam).abs(),
                "h={h} lam={lam} got {hl}"
            );
        }
    }

    #[test]
    fn gate_flags_stiff_forced_regime_and_not_rotation() {
        // stiff-forced: DOPRI5 settles at h*lambda ~ -3.2 .. -3.30.
        let k = linear_stages(3.2e-4, &[1.0], &[-1.0e4]);
        assert!(dopri5_stability_limited(
            dopri5_secant_h_lambda(3.2e-4, &k, 1).expect("valid test input"),
            0.8
        ));
        // rotation y' = [-100 y1, 100 y0]: no real dissipative direction.
        let h = 1.0e-3;
        let n = 2;
        let mut kk = vec![0.0; 7 * n];
        let y = [1.0, 0.0];
        for s in 0..7 {
            let mut ys = y;
            for j in 0..s {
                ys[0] += h * A[s][j] * kk[j * n];
                ys[1] += h * A[s][j] * kk[j * n + 1];
            }
            kk[s * n] = -100.0 * ys[1];
            kk[s * n + 1] = 100.0 * ys[0];
        }
        let hl = dopri5_secant_h_lambda(h, &kk, n).expect("valid test input");
        assert!(hl.abs() < 0.05, "rotation secant {hl}");
        assert!(!dopri5_stability_limited(hl, 0.8));
    }

    #[test]
    fn degenerate_inputs_return_none() {
        assert_eq!(dopri5_secant_h_lambda(1e-3, &[0.0; 7], 1), None);
        assert_eq!(dopri5_secant_h_lambda(1e-3, &[1.0; 6], 1), None);
        assert_eq!(dopri5_secant_h_lambda(0.0, &[1.0; 7], 1), None);
        assert_eq!(dopri5_secant_h_lambda(1e-3, &[1.0; 7], 0), None);
        assert!(!dopri5_stability_limited(f64::NAN, 0.8));
        assert!(!dopri5_stability_limited(-2.0, 0.8));
        assert!(dopri5_stability_limited(-2.7, 0.8));
    }
}
