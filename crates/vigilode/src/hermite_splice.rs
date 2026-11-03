//! Cubic Hermite dense-output splice at route switches (WP-11).
//!
//! At switch time $t_s$, both routes have access to $f(t_s, y_s)$. If the
//! younger window's interpolant derivative at $t_s$ differs from $f(t_s, y_s)$,
//! this module provides a cubic Hermite correction on that window $[t_0, t_1]$
//! that matches value and derivative at both ends, restoring $C^1$ continuity
//! without perturbing endpoint states or degrading the interpolant's asymptotic order.
//!
//! # Mathematical formulation
//!
//! With $h = t_1 - t_0$ and $\theta = (t - t_0) / h \in [0, 1]$:
//! - $C(0) = 0$
//! - $C(1) = 0$
//! - $C'(0) = h \Delta f_0$ where $\Delta f_0 = f(t_0, y_0) - P'(t_0^+)$
//! - $C'(1) = h \Delta f_1$ where $\Delta f_1 = f(t_1, y_1) - P'(t_1^-)$
//!
//! The cubic correction polynomial is:
//! $$C(\theta) = h \left[ \theta (1 - \theta)^2 \Delta f_0 - \theta^2 (1 - \theta) \Delta f_1 \right]$$
//!
//! Its derivative with respect to time $t$ is:
//! $$\frac{dC}{dt} = (1 - \theta)(1 - 3\theta) \Delta f_0 - \theta(2 - 3\theta) \Delta f_1$$
//!
//! At $t_0$ ($\theta = 0$): $\frac{dC}{dt} = \Delta f_0$, so $(P + C)'(t_0^+) = P'(t_0^+) + \Delta f_0 = f(t_0, y_0)$.
//! At $t_1$ ($\theta = 1$): $\frac{dC}{dt} = \Delta f_1$, so $(P + C)'(t_1^-) = P'(t_1^-) + \Delta f_1 = f(t_1, y_1)$.
//! Endpoints are unperturbed: $C(0) = 0, C(1) = 0$ bit-for-bit.
//! The correction magnitude is $O(h^{p+1}) = O(\text{local error})$, preserving interpolant order.

use serde::{Deserialize, Serialize};

/// Error returned when constructing a [`CubicHermiteSplice`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HermiteSpliceError {
    /// Window span is invalid ($t_{\text{end}} \le t_{\text{start}}$).
    #[error("splice interval length must be positive: t_start={t_start}, t_end={t_end}")]
    InvalidInterval {
        /// Start of the window.
        t_start: String,
        /// End of the window.
        t_end: String,
    },
    /// Discrepancy vector lengths do not match.
    #[error(
        "derivative discrepancy dimension mismatch: delta_f0 len {dim0} != delta_f1 len {dim1}"
    )]
    DimensionMismatch {
        /// Dimension of delta_f0.
        dim0: usize,
        /// Dimension of delta_f1.
        dim1: usize,
    },
    /// Zero dimension is disallowed.
    #[error("derivative dimension must be non-zero")]
    ZeroDimension,
    /// Non-finite parameter encountered.
    #[error("non-finite value in splice parameters")]
    NonFinite,
}

/// A cubic Hermite correction polynomial $C(\theta)$ on $[t_{\text{start}}, t_{\text{end}}]$
/// to enforce $C^1$ continuity across route switches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CubicHermiteSplice {
    /// Start of the splice window ($t_0$).
    pub t_start: f64,
    /// End of the splice window ($t_1$).
    pub t_end: f64,
    /// Derivative discrepancy at window start: $f(t_0, y_0) - P'(t_0^+)$.
    pub delta_f0: Vec<f64>,
    /// Derivative discrepancy at window end: $f(t_1, y_1) - P'(t_1^-)$ (or zeros).
    pub delta_f1: Vec<f64>,
}

impl CubicHermiteSplice {
    /// Create a new cubic Hermite splice on $[t_{\text{start}}, t_{\text{end}}]$.
    ///
    /// # Errors
    /// Returns [`HermiteSpliceError`] if:
    /// - $t_{\text{end}} \le t_{\text{start}}$ or times are not finite,
    /// - `delta_f0.len() != delta_f1.len()`,
    /// - `delta_f0.is_empty()`,
    /// - any discrepancy component is NaN or infinite.
    pub fn new(
        t_start: f64,
        t_end: f64,
        delta_f0: Vec<f64>,
        delta_f1: Vec<f64>,
    ) -> Result<Self, HermiteSpliceError> {
        if !t_start.is_finite() || !t_end.is_finite() {
            return Err(HermiteSpliceError::NonFinite);
        }
        if t_end <= t_start {
            return Err(HermiteSpliceError::InvalidInterval {
                t_start: t_start.to_string(),
                t_end: t_end.to_string(),
            });
        }
        if delta_f0.len() != delta_f1.len() {
            return Err(HermiteSpliceError::DimensionMismatch {
                dim0: delta_f0.len(),
                dim1: delta_f1.len(),
            });
        }
        if delta_f0.is_empty() {
            return Err(HermiteSpliceError::ZeroDimension);
        }
        if delta_f0.iter().any(|v| !v.is_finite()) || delta_f1.iter().any(|v| !v.is_finite()) {
            return Err(HermiteSpliceError::NonFinite);
        }
        Ok(Self {
            t_start,
            t_end,
            delta_f0,
            delta_f1,
        })
    }

    /// Create a cubic Hermite splice matching the initial derivative discrepancy $\Delta f_0$
    /// and preserving the segment right derivative ($\Delta f_1 = 0$).
    pub fn new_initial(
        t_start: f64,
        t_end: f64,
        delta_f0: Vec<f64>,
    ) -> Result<Self, HermiteSpliceError> {
        let dim = delta_f0.len();
        Self::new(t_start, t_end, delta_f0, vec![0.0; dim])
    }

    /// Dimension of the state / derivative vectors.
    #[inline]
    pub fn dimension(&self) -> usize {
        self.delta_f0.len()
    }

    /// Step size (length) of the splice window $h = t_{\text{end}} - t_{\text{start}}$.
    #[inline]
    pub fn step_size(&self) -> f64 {
        self.t_end - self.t_start
    }

    /// Whether the given time $t$ lies inside the splice window $[t_{\text{start}}, t_{\text{end}}]$.
    #[inline]
    pub fn contains(&self, t: f64) -> bool {
        t >= self.t_start && t <= self.t_end
    }

    /// Evaluate the cubic Hermite correction $C(t)$ into `out` in place (zero allocation).
    ///
    /// If $t$ is outside the window, or exactly at $t_{\text{start}}$ or $t_{\text{end}}$,
    /// `out` is filled with zeros (preserving endpoints bit-for-bit).
    pub fn evaluate_correction_into(&self, t: f64, out: &mut [f64]) {
        assert_eq!(
            out.len(),
            self.dimension(),
            "output buffer dimension mismatch"
        );
        if t <= self.t_start || t >= self.t_end {
            out.fill(0.0);
            return;
        }
        let h = self.t_end - self.t_start;
        let theta = (t - self.t_start) / h;
        let one_minus_theta = 1.0 - theta;
        let b0 = theta * one_minus_theta * one_minus_theta;
        let b1 = theta * theta * one_minus_theta;
        for (i, v) in out.iter_mut().enumerate() {
            *v = h * (b0 * self.delta_f0[i] - b1 * self.delta_f1[i]);
        }
    }

    /// Evaluate the time derivative of the correction $\frac{dC}{dt}(t)$ into `out` in place (zero allocation).
    ///
    /// At $t_{\text{start}}$, `out` is set to $\Delta f_0$ bit-for-bit.
    /// At $t_{\text{end}}$, `out` is set to $\Delta f_1$ bit-for-bit.
    /// Outside the window, `out` is filled with zeros.
    pub fn evaluate_derivative_correction_into(&self, t: f64, out: &mut [f64]) {
        assert_eq!(
            out.len(),
            self.dimension(),
            "output buffer dimension mismatch"
        );
        if t < self.t_start || t > self.t_end {
            out.fill(0.0);
            return;
        }
        if t == self.t_start {
            out.copy_from_slice(&self.delta_f0);
            return;
        }
        if t == self.t_end {
            out.copy_from_slice(&self.delta_f1);
            return;
        }
        let h = self.t_end - self.t_start;
        let theta = (t - self.t_start) / h;
        let d0 = (1.0 - theta) * (1.0 - 3.0 * theta);
        let d1 = theta * (2.0 - 3.0 * theta);
        for (i, v) in out.iter_mut().enumerate() {
            *v = d0 * self.delta_f0[i] - d1 * self.delta_f1[i];
        }
    }

    /// Add the cubic Hermite correction $C(t)$ to `state` in place (zero allocation).
    ///
    /// If $t \le t_{\text{start}}$ or $t \ge t_{\text{end}}$, `state` is left unmodified bit-for-bit.
    pub fn apply_correction(&self, state: &mut [f64], t: f64) {
        assert_eq!(
            state.len(),
            self.dimension(),
            "state buffer dimension mismatch"
        );
        if t <= self.t_start || t >= self.t_end {
            return;
        }
        let h = self.t_end - self.t_start;
        let theta = (t - self.t_start) / h;
        let one_minus_theta = 1.0 - theta;
        let b0 = theta * one_minus_theta * one_minus_theta;
        let b1 = theta * theta * one_minus_theta;
        for (i, v) in state.iter_mut().enumerate() {
            *v += h * (b0 * self.delta_f0[i] - b1 * self.delta_f1[i]);
        }
    }

    /// Evaluate the cubic Hermite correction $C(t)$ returning a new `Vec<f64>`.
    ///
    /// Returns `None` if $t$ falls outside $[t_{\text{start}}, t_{\text{end}}]$.
    pub fn evaluate_correction(&self, t: f64) -> Option<Vec<f64>> {
        if !self.contains(t) {
            return None;
        }
        let mut out = vec![0.0; self.dimension()];
        self.evaluate_correction_into(t, &mut out);
        Some(out)
    }

    /// Evaluate the time derivative of the correction $\frac{dC}{dt}(t)$ returning a new `Vec<f64>`.
    ///
    /// Returns `None` if $t$ falls outside $[t_{\text{start}}, t_{\text{end}}]$.
    pub fn evaluate_derivative_correction(&self, t: f64) -> Option<Vec<f64>> {
        if !self.contains(t) {
            return None;
        }
        let mut out = vec![0.0; self.dimension()];
        self.evaluate_derivative_correction_into(t, &mut out);
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_splice_construction() {
        assert!(CubicHermiteSplice::new(1.0, 1.0, vec![1.0], vec![1.0]).is_err());
        assert!(CubicHermiteSplice::new(2.0, 1.0, vec![1.0], vec![1.0]).is_err());
        assert!(CubicHermiteSplice::new(0.0, 1.0, vec![1.0, 2.0], vec![1.0]).is_err());
        assert!(CubicHermiteSplice::new(0.0, 1.0, vec![], vec![]).is_err());
        assert!(CubicHermiteSplice::new(f64::NAN, 1.0, vec![1.0], vec![1.0]).is_err());
        assert!(CubicHermiteSplice::new(0.0, 1.0, vec![f64::INFINITY], vec![1.0]).is_err());
    }

    #[test]
    fn test_endpoint_preservation_bit_for_bit() {
        let t0 = 0.5;
        let t1 = 1.25;
        let delta_f0 = vec![1.2345e6, -9.8765e5];
        let delta_f1 = vec![-4.5678e5, 3.2109e6];
        let splice = CubicHermiteSplice::new(t0, t1, delta_f0, delta_f1).unwrap();

        let mut out = [999.0, -999.0];

        // At t0:
        splice.evaluate_correction_into(t0, &mut out);
        assert_eq!(out[0].to_bits(), 0.0f64.to_bits());
        assert_eq!(out[1].to_bits(), 0.0f64.to_bits());

        // At t1:
        splice.evaluate_correction_into(t1, &mut out);
        assert_eq!(out[0].to_bits(), 0.0f64.to_bits());
        assert_eq!(out[1].to_bits(), 0.0f64.to_bits());

        // evaluate_correction:
        assert_eq!(splice.evaluate_correction(t0).unwrap(), vec![0.0, 0.0]);
        assert_eq!(splice.evaluate_correction(t1).unwrap(), vec![0.0, 0.0]);

        // apply_correction leaves state bit-for-bit identical:
        let original_state = [std::f64::consts::PI, std::f64::consts::E];
        let mut state = original_state;
        splice.apply_correction(&mut state, t0);
        assert_eq!(state[0].to_bits(), original_state[0].to_bits());
        assert_eq!(state[1].to_bits(), original_state[1].to_bits());

        splice.apply_correction(&mut state, t1);
        assert_eq!(state[0].to_bits(), original_state[0].to_bits());
        assert_eq!(state[1].to_bits(), original_state[1].to_bits());
    }

    #[test]
    fn test_derivative_continuity_at_switch() {
        let ts = 1.0;
        let t_end = 1.4;
        let f_actual = [-0.42, 1.35];
        let p_prime_right = [-0.38, 1.20]; // P'(t_s^+) before correction
        let delta_f0 = vec![
            f_actual[0] - p_prime_right[0],
            f_actual[1] - p_prime_right[1],
        ];
        let delta_f1 = vec![0.05, -0.02];

        let splice = CubicHermiteSplice::new(ts, t_end, delta_f0, delta_f1).unwrap();

        let mut d_corr = [0.0; 2];
        splice.evaluate_derivative_correction_into(ts, &mut d_corr);

        // Corrected derivative: (P + C)'(t_s^+) = P'(t_s^+) + dC/dt(t_s)
        let corrected_deriv = [p_prime_right[0] + d_corr[0], p_prime_right[1] + d_corr[1]];

        for i in 0..2 {
            let rel_err = (corrected_deriv[i] - f_actual[i]).abs() / f_actual[i].abs();
            assert!(
                rel_err < 1e-12,
                "dim {i}: corrected_deriv = {}, f_actual = {}, rel_err = {rel_err:e}",
                corrected_deriv[i],
                f_actual[i]
            );
        }

        // Test finite-difference agreement for dC/dt:
        let eps = 1e-7;
        let c_plus = splice.evaluate_correction(ts + eps).unwrap();
        let c_zero = splice.evaluate_correction(ts).unwrap();
        for i in 0..2 {
            let fd_deriv = (c_plus[i] - c_zero[i]) / eps;
            let rel_diff = (fd_deriv - d_corr[i]).abs() / (d_corr[i].abs() + 1e-10);
            assert!(
                rel_diff < 1e-6,
                "dim {i}: fd_deriv={fd_deriv}, d_corr={}",
                d_corr[i]
            );
        }
    }

    #[test]
    fn test_interior_point_accuracy() {
        // Test with y(t) = sin(t), y'(t) = cos(t).
        // On [0, h] with h = 0.1, consider an interpolant P(t).
        let t0 = 0.0;
        let h = 0.1;
        let t1 = t0 + h;

        // Suppose P(t) is a 4th-order polynomial with an O(h^4) derivative error at t0:
        // P(t) = sin(t) + alpha * t^4 (alpha = 0.5)
        // P'(0) = cos(0) + 0 = 1.0; discrepancy at t0 is 0.
        // Let's add a perturbation P(t) = sin(t) + delta * t * (h - t)^3
        // So P'(0) = 1.0 + delta * h^3.
        let delta = 0.5;
        let p = |t: f64| t.sin() + delta * t * (h - t).powi(3);
        let p_prime_0 = 1.0 + delta * h.powi(3);
        let p_prime_1 = h.cos() - delta * h.powi(3);

        let delta_f0 = vec![1.0 - p_prime_0];
        let delta_f1 = vec![h.cos() - p_prime_1];

        let splice = CubicHermiteSplice::new(t0, t1, delta_f0, delta_f1).unwrap();

        // Sample 100 interior points
        let mut max_uncorrected_err = 0.0f64;
        let mut max_corrected_err = 0.0f64;

        for step in 1..100 {
            let t = t0 + (step as f64 / 100.0) * h;
            let exact = t.sin();
            let uncorrected = p(t);
            let corr = splice.evaluate_correction(t).unwrap()[0];
            let corrected = uncorrected + corr;

            max_uncorrected_err = max_uncorrected_err.max((uncorrected - exact).abs());
            max_corrected_err = max_corrected_err.max((corrected - exact).abs());
        }

        // Interior error does not degrade: error with correction should be comparable or better,
        // and strictly not worse by more than 2x (WP-11 specification).
        assert!(
            max_corrected_err <= 2.0 * max_uncorrected_err,
            "max_corrected_err = {max_corrected_err:e}, max_uncorrected_err = {max_uncorrected_err:e}"
        );
    }
}
