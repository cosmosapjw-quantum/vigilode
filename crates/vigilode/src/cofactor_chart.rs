//! Exact scalar cofactor transport for a caller-supplied triangular structure.
//!
//! If a coordinate `z` satisfies `z' = -rate*z` and a remaining coordinate
//! satisfies `y' = g(t,y) + coupling*z`, write `y = v + C(t)` with
//! `C(t) = coupling*z0/rate * (1-exp(-rate*(t-t0)))`. Then
//! `v' = g(t,v+C(t))`. The derivative of the transformed RHS with respect to
//! `v` is the derivative of `g`, independently of `rate`.
//!
//! This object transports supplied structure; it does not infer or certify
//! a cofactor identity from a few RHS samples. The caller validates the
//! identity and reconstructs every requested physical output.

/// Exact exponential carrier and its integrated contribution to a slow state.
#[derive(Clone, Copy, Debug)]
pub struct ExponentialCofactor {
    initial_time: f64,
    initial_value: f64,
    decay_rate: f64,
    primitive_amplitude: f64,
}

impl ExponentialCofactor {
    /// Construct a forward-decaying carrier. Every parameter and the stored
    /// primitive amplitude must be finite; `decay_rate` must be positive.
    pub fn new(
        initial_time: f64,
        initial_value: f64,
        decay_rate: f64,
        coupling: f64,
    ) -> Result<Self, &'static str> {
        if !initial_time.is_finite()
            || !initial_value.is_finite()
            || !decay_rate.is_finite()
            || decay_rate <= 0.0
            || !coupling.is_finite()
        {
            return Err("cofactor parameters must be finite and decay rate positive");
        }
        // Prefer the division-first path to avoid overflow for fast decay.
        let divided = initial_value / decay_rate;
        let direct = divided * coupling;
        let primitive_amplitude = if initial_value == 0.0 || coupling == 0.0 {
            0.0
        } else if divided.is_normal() && direct.is_normal() {
            direct
        } else {
            let magnitude =
                (initial_value.abs().ln() + coupling.abs().ln() - decay_rate.ln()).exp();
            magnitude.copysign(initial_value.signum() * coupling.signum())
        };
        if !primitive_amplitude.is_finite() {
            return Err("cofactor primitive amplitude is not representable");
        }
        Ok(Self {
            initial_time,
            initial_value,
            decay_rate,
            primitive_amplitude,
        })
    }

    fn exponent(&self, time: f64) -> Result<f64, &'static str> {
        if !time.is_finite() || time < self.initial_time {
            return Err("cofactor time must be finite and not precede its initial time");
        }
        // Positive overflow represents a carrier below the floating range;
        // exp(-infinity)=0 is its rounded asymptotic value, not a failure.
        let elapsed = time - self.initial_time;
        let scaled = if elapsed.is_finite() {
            self.decay_rate * elapsed
        } else {
            // Overflow in a difference of finite ordered endpoints requires
            // opposite signs. Scaling first is safe from cancellation here
            // and can recover a representable exponent for a small rate.
            self.decay_rate * time - self.decay_rate * self.initial_time
        };
        Ok(-scaled)
    }

    /// Evaluate `z0*exp(-rate*(time-t0))`; underflow yields zero.
    pub fn value(&self, time: f64) -> Result<f64, &'static str> {
        let exponent = self.exponent(time)?;
        let decay = exponent.exp();
        let direct = self.initial_value * decay;
        if self.initial_value == 0.0 || decay.is_normal() && direct.is_normal() {
            return Ok(direct);
        }
        Ok((self.initial_value.abs().ln() + exponent)
            .exp()
            .copysign(self.initial_value))
    }

    /// Evaluate the integrated forcing using `expm1` near the initial time.
    pub fn primitive(&self, time: f64) -> Result<f64, &'static str> {
        let exponent = self.exponent(time)?;
        if time == self.initial_time || self.primitive_amplitude == 0.0 {
            return Ok(0.0);
        }
        if !exponent.is_normal() && exponent.is_finite() {
            // A rounded-to-zero or subnormal rate*dt can still give a
            // representable integral after multiplication by a large amplitude.
            // Here (1-exp(-x))/x differs from 1 by less than 2^-1022;
            // combine the magnitude before the final exponential instead.
            let log_value = self.primitive_amplitude.abs().ln()
                + self.decay_rate.ln()
                + (time - self.initial_time).ln();
            return Ok(log_value.exp().copysign(self.primitive_amplitude));
        }
        Ok(-self.primitive_amplitude * exponent.exp_m1())
    }

    /// Recover the slow physical coordinate `y=v+C(time)`.
    pub fn physical_slow(&self, time: f64, reduced: f64) -> Result<f64, &'static str> {
        if !reduced.is_finite() {
            return Err("cofactor reduced state must be finite");
        }
        let y = reduced + self.primitive(time)?;
        if !y.is_finite() {
            return Err("cofactor reconstruction overflowed");
        }
        Ok(y)
    }
}
