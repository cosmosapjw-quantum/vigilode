//! Source-bound, value-only Robertson diagonal implicit RK stage bridge.
//!
//! This is an auxiliary **single implicit RK/BE stage**, NOT the original
//! 8-stage RODAS5P raw-U root. Its root iteration calls no derivative, JVP,
//! dense Jacobian or Newton step. Original full physical stage defect and
//! a source-specific forward-error upper bound are separately checked by
//! outward interval arithmetic before the caller's output is committed.
//!
//! Predecessor: PR117 R5 exact nonnegative Robertson stage theorem.
use crate::rhs_homotopy::{RhsHomotopyAttempt, RhsHomotopyConfig, RhsOnlyMap, solve_rhs_homotopy};
use rodas5p_core::directed::{Interval, add_up, div_up};
use rodas5p_core::{CoreError, CoreResult, N01MassActionOwner, N01MassActionTrial};

fn point(x: f64) -> CoreResult<Interval> {
    Interval::point(x)
}
fn fail(msg: &'static str) -> CoreError {
    CoreError::InvalidInput(msg.into())
}

/// Immutable source-owned stage data. The rates, reference state and weights
/// originate in the same N01MassActionOwner and opaque trial; a caller can
/// supply only the implicit-RK diagonal coefficient tau.
pub struct N01ValueStage<'a> {
    owner: &'a N01MassActionOwner,
    trial: &'a N01MassActionTrial,
    rates: [f64; 3],
    carrier: [f64; 3],
    w: [f64; 3],
    p: [f64; 3],
    tau: f64,
    mass_upper: f64,
    derivative_upper: f64,
    la_upper: f64,
    lc_upper: f64,
    epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct N01ValueCertificate {
    pub original_residual_upper: f64,
    pub physical_stage_error_upper: f64,
    pub scalar_residual_upper: f64,
    pub accepted: bool,
}

pub struct N01ValueAttempt {
    pub homotopy: RhsHomotopyAttempt,
    pub certificate: Option<N01ValueCertificate>,
    /// Explicit certificate error, distinct from a numerical gate rejection.
    pub certificate_error: Option<String>,
    pub admitted: bool,
}

impl<'a> N01ValueStage<'a> {
    pub fn new(
        owner: &'a N01MassActionOwner,
        trial: &'a N01MassActionTrial,
        tau: f64,
    ) -> CoreResult<Self> {
        let (rates, carrier, w, p) = owner.value_only_stage_inputs(trial)?;
        if !(tau.is_finite() && tau >= 0.0)
            || rates.iter().any(|k| !k.is_finite() || *k < 0.0)
            || carrier.iter().any(|y| !y.is_finite() || *y < 0.0)
            || w.iter().chain(&p).any(|s| !s.is_finite() || *s <= 0.0)
        {
            return Err(fail("N01_DF_INVALID_SOURCE_OR_TAU"));
        }
        let mass = point(carrier[0])?
            .add(point(carrier[1])?)?
            .add(point(carrier[2])?)?;
        let mass_upper = mass.as_pair()[1];
        let t = point(tau)?;
        let k1 = point(rates[0])?;
        let k2 = point(rates[1])?;
        let k3 = point(rates[2])?;
        let m = point(mass_upper)?;
        let denominator = point(1.0)?.add(t.mul(k1)?)?;
        let la = t
            .mul(k2)?
            .mul(point(carrier[2])?.add(point(3.0)?.mul(t)?.mul(k3)?.mul(m)?.mul(m)?)?)?
            .div(denominator)?;
        let lc = point(2.0)?.mul(t)?.mul(k3)?.mul(m)?;
        let la_upper = la.as_pair()[1];
        let lc_upper = lc.as_pair()[1];
        let derivative_upper = add_up(add_up(1.0, la_upper)?, lc_upper)?;
        if !mass_upper.is_finite()
            || !derivative_upper.is_finite()
            || derivative_upper < 1.0
            || !la_upper.is_finite()
            || !lc_upper.is_finite()
        {
            return Err(fail("N01_DF_INVALID_MAJORANT"));
        }
        Ok(Self {
            owner,
            trial,
            rates,
            carrier,
            w,
            p,
            tau,
            mass_upper,
            derivative_upper,
            la_upper,
            lc_upper,
            epoch: owner.model_epoch(),
        })
    }

    fn live(&self) -> CoreResult<()> {
        self.owner.check_value_trial(self.trial)?;
        if self.owner.model_epoch() != self.epoch {
            return Err(fail("N01_DF_STALE_EPOCH"));
        }
        Ok(())
    }

    /// Return (a(b),c(b),F(b)) evaluated in real intervals for the *exact*
    /// mathematical polynomial with represented binary64 input data.
    fn reconstruct_interval(&self, b: f64) -> CoreResult<(Interval, Interval, Interval)> {
        let t = point(self.tau)?;
        let b_i = point(b)?;
        let c = point(self.carrier[2])?.add(t.mul(point(self.rates[2])?)?.mul(b_i)?.mul(b_i)?)?;
        let d = point(1.0)?.add(t.mul(point(self.rates[0])?)?)?;
        let a = point(self.carrier[0])?
            .add(t.mul(point(self.rates[1])?)?.mul(b_i)?.mul(c)?)?
            .div(d)?;
        let m = point(self.carrier[0])?
            .add(point(self.carrier[1])?)?
            .add(point(self.carrier[2])?)?;
        let f = a.add(b_i)?.add(c)?.sub(m)?;
        Ok((a, c, f))
    }

    /// Operational floating map G=b-F/L, safeguarded on [0,M_upper].
    /// Its residual is **not** a rigorous physical residual; the latter is
    /// always recomputed from source-bound intervals before output mutation.
    fn operational_map(&self, b: f64) -> CoreResult<f64> {
        self.live()?;
        if !(b.is_finite() && b >= 0.0 && b <= self.mass_upper) {
            return Err(fail("N01_DF_B_OUTSIDE_INTERVAL"));
        }
        let [k1, k2, k3] = self.rates;
        let [a0, b0, c0] = self.carrier;
        let c = c0 + self.tau * k3 * b * b;
        let d = 1.0 + self.tau * k1;
        let a = (a0 + self.tau * k2 * b * c) / d;
        let f = ((a + b) + c) - ((a0 + b0) + c0);
        let mapped = (b - f / self.derivative_upper).clamp(0.0, self.mass_upper);
        if !c.is_finite()
            || !d.is_finite()
            || !a.is_finite()
            || !f.is_finite()
            || !mapped.is_finite()
        {
            return Err(CoreError::NonFinite("N01_DF_VALUE_MAP_OVERFLOW".into()));
        }
        Ok(mapped)
    }

    /// Source-bound original stage residual and source-specific p-norm bound.
    /// If accepted, write the entire 3-vector atomically. Error/rejection
    /// leaves caller output bitwise unchanged.
    pub fn try_commit(
        &self,
        b: f64,
        original_tolerance: f64,
        physical_tolerance: f64,
        output: &mut [f64; 3],
    ) -> CoreResult<N01ValueCertificate> {
        self.live()?;
        if !b.is_finite()
            || b < 0.0
            || b > self.mass_upper
            || !original_tolerance.is_finite()
            || original_tolerance < 0.0
            || !physical_tolerance.is_finite()
            || physical_tolerance < 0.0
        {
            return Err(fail("N01_DF_BAD_CANDIDATE_OR_BUDGET"));
        }
        let [k1, k2, k3] = self.rates;
        let [a0, _bb0, c0] = self.carrier;
        let c = c0 + self.tau * k3 * b * b;
        let a = (a0 + self.tau * k2 * b * c) / (1.0 + self.tau * k1);
        let z = [a, b, c];
        if !z.iter().all(|v| v.is_finite() && *v >= 0.0) {
            return Err(CoreError::NonFinite(
                "N01_DF_RECONSTRUCTION_NONFINITE".into(),
            ));
        }
        let (a_exact, c_exact, f_exact) = self.reconstruct_interval(b)?;
        let scalar_residual_upper = f_exact.mag();
        let rounding_a = point(a)?.sub(a_exact)?.mag();
        let rounding_c = point(c)?.sub(c_exact)?.mag();
        let ea = add_up(
            rounding_a,
            rodas5p_core::directed::mul_up(self.la_upper, scalar_residual_upper)?,
        )?;
        let ec = add_up(
            rounding_c,
            rodas5p_core::directed::mul_up(self.lc_upper, scalar_residual_upper)?,
        )?;
        let physical_error_upper = div_up(ea, self.p[0])?
            .max(div_up(scalar_residual_upper, self.p[1])?)
            .max(div_up(ec, self.p[2])?);
        // Independently evaluate the exact-real original full stage defect
        // r=z-y-tau*f(z), not the homotopy projected/operational residual.
        let ai = point(a)?;
        let bi = point(b)?;
        let ci = point(c)?;
        let u = point(k1)?.mul(ai)?;
        let v = point(k2)?.mul(bi)?.mul(ci)?;
        let q = point(k3)?.mul(bi)?.mul(bi)?;
        let rhs = [v.sub(u)?, u.sub(v)?.sub(q)?, q];
        let mut original_residual_upper = 0.0_f64;
        for i in 0..3 {
            let residual = point(z[i])?
                .sub(point(self.carrier[i])?)?
                .sub(point(self.tau)?.mul(rhs[i])?)?;
            original_residual_upper =
                original_residual_upper.max(div_up(residual.mag(), self.w[i])?);
        }
        let accepted = original_residual_upper <= original_tolerance
            && physical_error_upper <= physical_tolerance;
        self.live()?;
        if accepted {
            output.copy_from_slice(&z);
        }
        Ok(N01ValueCertificate {
            original_residual_upper,
            physical_stage_error_upper: physical_error_upper,
            scalar_residual_upper,
            accepted,
        })
    }

    pub fn solve(
        &self,
        cfg: &RhsHomotopyConfig,
        original_tolerance: f64,
        physical_tolerance: f64,
        output: &mut [f64; 3],
    ) -> N01ValueAttempt {
        let attempt = solve_rhs_homotopy(self, &[self.carrier[1]], cfg);
        let mut certificate = None;
        let mut certificate_error = None;
        let mut admitted = false;
        if let Some(ref root) = attempt.root
            && root.len() == 1
        {
            match self.try_commit(root[0], original_tolerance, physical_tolerance, output) {
                Ok(result) => {
                    admitted = result.accepted;
                    certificate = Some(result);
                }
                Err(error) => certificate_error = Some(error.to_string()),
            }
        }
        N01ValueAttempt {
            homotopy: attempt,
            certificate,
            certificate_error,
            admitted,
        }
    }
}

impl RhsOnlyMap for N01ValueStage<'_> {
    fn dimension(&self) -> usize {
        1
    }
    fn model_epoch(&self) -> Option<u64> {
        Some(self.owner.model_epoch())
    }
    fn admits(&self, x: &[f64]) -> bool {
        x.len() == 1 && x[0].is_finite() && x[0] >= 0.0 && x[0] <= self.mass_upper
    }
    fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
        if x.len() != 1 || out.len() != 1 {
            return Err("N01_DF_SHAPE".into());
        }
        let mapped = self.operational_map(x[0]).map_err(|e| e.to_string())?;
        out[0] = mapped;
        Ok(())
    }
}
