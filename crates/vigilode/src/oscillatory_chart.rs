//! Opt-in carrier coordinates for a declared exponential-chirp family.
//!
//! This is a coordinate adapter, not frequency inference from black-box samples.
//! The target is `x'=v+r_x`, `v'=-(2γ-β)v-(ω²+γ²-γβ)x+r_v`,
//! with `ω(t)=ω0 exp(βt)`. The caller supplies only the extra residual `(r_x,r_v)`.
//! No Jacobian, JVP, matrix factorization or Newton solve is used.

/// Checked time-varying chart and common oscillatory carrier.
#[derive(Clone, Copy, Debug)]
pub struct ExponentialChirpChart {
    omega0: f64,
    beta: f64,
    gamma: f64,
    start: f64,
    end: f64,
    maximum_lift: f64,
}

impl ExponentialChirpChart {
    /// Declare the coefficients and closed forward time interval.
    /// Positive frequency and nonnegative damping are required by this pilot.
    pub fn new(omega0: f64, beta: f64, gamma: f64, span: (f64, f64)) -> Result<Self, String> {
        let (start, end) = span;
        if ![omega0, beta, gamma, start, end]
            .iter()
            .all(|x| x.is_finite())
            || omega0 <= 0.0
            || gamma < 0.0
            || end < start
        {
            return Err("invalid chirp coefficients or time interval".into());
        }
        let w0 = omega0 * (beta * start).exp();
        let w1 = omega0 * (beta * end).exp();
        let maximum_lift = 1.0_f64.max(gamma + w0.max(w1));
        if !w0.is_finite() || !w1.is_finite() || w0 <= 0.0 || w1 <= 0.0 || !maximum_lift.is_finite()
        {
            return Err("frequency or physical lift is outside finite range".into());
        }
        let chart = Self {
            omega0,
            beta,
            gamma,
            start,
            end,
            maximum_lift,
        };
        chart.phase(end)?;
        Ok(chart)
    }

    fn checked_time(&self, t: f64) -> Result<(), String> {
        if !t.is_finite() || t < self.start || t > self.end {
            Err("time outside declared chirp interval".into())
        } else {
            Ok(())
        }
    }

    /// Frequency at a time in the declared interval.
    pub fn frequency(&self, t: f64) -> Result<f64, String> {
        self.checked_time(t)?;
        Ok(self.omega0 * (self.beta * t).exp())
    }

    /// Integrated carrier phase from the declared start; uses `exprel` near β=0.
    /// Binary64 trigonometric error is not a directed phase enclosure.
    pub fn phase(&self, t: f64) -> Result<f64, String> {
        self.checked_time(t)?;
        let dt = t - self.start;
        if !dt.is_finite() {
            return Err("chirp time difference outside finite range".into());
        }
        let w = self.omega0 * (self.beta * self.start).exp();
        let x = self.beta * dt;
        let phase = if self.beta == 0.0 || x == 0.0 {
            w * dt
        } else if x.abs() <= 0.5 {
            let exprel = x.exp_m1() / x;
            let first = (w * dt) * exprel;
            if first.is_finite() {
                first
            } else {
                (w * exprel) * dt
            }
        } else if x > 0.0 {
            // Integrate from the larger endpoint to avoid exp(beta*dt) overflow.
            (self.frequency(t)? / self.beta) * -(-x).exp_m1()
        } else {
            (w / -self.beta) * -x.exp_m1()
        };
        if !phase.is_finite() {
            return Err("nonfinite carrier phase".into());
        }
        Ok(phase)
    }

    /// Convert physical `(x,v)` into `w=R(phase)(x,(v+γx)/ω)`.
    pub fn to_rotating(&self, t: f64, y: [f64; 2]) -> Result<[f64; 2], String> {
        checked_pair(y)?;
        let p = (y[1] + self.gamma * y[0]) / self.frequency(t)?;
        let out = rotate(self.phase(t)?, [y[0], p]);
        checked_pair(out)?;
        Ok(out)
    }

    /// Reconstruct physical `(x,v)` and retain the time-dependent velocity scale.
    pub fn to_physical(&self, t: f64, w: [f64; 2]) -> Result<[f64; 2], String> {
        checked_pair(w)?;
        let z = rotate(-self.phase(t)?, w);
        let out = [z[0], self.frequency(t)? * z[1] - self.gamma * z[0]];
        checked_pair(out)?;
        Ok(out)
    }

    /// Map the caller's additional physical residual into the rotating RHS.
    /// Includes the known damping `-γw`; never subtracts large full-RHS terms.
    pub fn rotating_rhs(
        &self,
        t: f64,
        w: [f64; 2],
        residual: [f64; 2],
    ) -> Result<[f64; 2], String> {
        checked_pair(w)?;
        checked_pair(residual)?;
        let r = [
            residual[0],
            (residual[1] + self.gamma * residual[0]) / self.frequency(t)?,
        ];
        let rr = rotate(self.phase(t)?, r);
        let out = [rr[0] - self.gamma * w[0], rr[1] - self.gamma * w[1]];
        checked_pair(out)?;
        Ok(out)
    }

    /// Exact-arithmetic bound on physical infinity error from rotating Euclidean error.
    /// If `||δw||₂≤eta`, then `||δy||∞≤max(1,γ+ω(t))*eta`.
    /// This does not include floating-point phase or reconstruction error.
    pub fn physical_error_bound(&self, t: f64, rotating_l2_error: f64) -> Result<f64, String> {
        if !rotating_l2_error.is_finite() || rotating_l2_error < 0.0 {
            return Err("invalid rotating error".into());
        }
        let value = 1.0_f64.max(self.gamma + self.frequency(t)?) * rotating_l2_error;
        if !value.is_finite() {
            return Err("physical error bound overflow".into());
        }
        Ok(value)
    }

    /// Sufficient scalar absolute tolerance for a two-component WRMS norm.
    /// Set the transformed solver's `rtol=0`; WRMS≤1 then implies the
    /// *embedded local estimate* in physical infinity norm is ≤`physical_atol`
    /// over this interval, in exact arithmetic. This is not a global-error theorem.
    pub fn conservative_local_atol(&self, physical_atol: f64) -> Result<f64, String> {
        if !physical_atol.is_finite() || physical_atol <= 0.0 {
            return Err("physical absolute tolerance must be positive".into());
        }
        let value = physical_atol / (std::f64::consts::SQRT_2 * self.maximum_lift);
        if value <= 0.0 || !value.is_finite() {
            return Err("transported tolerance underflow".into());
        }
        Ok(value)
    }
}

fn checked_pair(v: [f64; 2]) -> Result<(), String> {
    if v.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err("nonfinite chart state or residual".into())
    }
}

fn rotate(theta: f64, v: [f64; 2]) -> [f64; 2] {
    let (s, c) = theta.sin_cos();
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

/// Apply opposite rotations to independent amplitudes using one `sin_cos` call.
/// Both involution sectors and both independently supplied states are retained.
pub fn paired_carrier_rotation(
    theta: f64,
    plus: [f64; 2],
    minus: [f64; 2],
) -> Result<[[f64; 2]; 2], String> {
    if !theta.is_finite() {
        return Err("nonfinite carrier phase".into());
    }
    checked_pair(plus)?;
    checked_pair(minus)?;
    let (s, c) = theta.sin_cos();
    let out = [
        [c * plus[0] - s * plus[1], s * plus[0] + c * plus[1]],
        [c * minus[0] + s * minus[1], -s * minus[0] + c * minus[1]],
    ];
    checked_pair(out[0])?;
    checked_pair(out[1])?;
    Ok(out)
}
