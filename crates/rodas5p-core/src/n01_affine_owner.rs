//! N01 opt-in time-affine *source-owned* certificate, NOT a generic nonlinear
//! or arbitrary matrix-free JVP certificate. The mathematical source family is
//! f(t,z) = (J0 + (t-t0) K) z + b0 + (t-t0) q, where all displayed
//! coefficients and time arguments denote exact real values of binary64 inputs.
//!
//! Source-owned clocks, model coefficient bits, physical weights and epoch
//! cannot be supplied to the admission function by an independent caller.
//! A snapshot is valid only for the current source revision and clock.
//!
//! For the linear stage A* = I-sigma J(t), b* = b0+(t-t0)q, the source
//! independently derives delta_J, delta_b and the actual reference-operator
//! application error eta by outward interval evaluation. The candidate's
//! *original*, unpreconditioned residual and physical forward bound BOTH
//! must pass. Rejected solves never modify output. This is a deliberately
//! narrow but real time-dependent source, not a proof for a BASS nonlinear
//! client, nonlinear PC or unconstrained callback.

use crate::directed::{Interval, add_up, div_up, mul_up, sub_down};
use crate::{CoreError, CoreResult, DenseMatrix, sha256_f64_slice_bytes};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SourceStamp {
    digest: [u8; 32],
    epoch: u64,
    time_bits: u64,
}

/// Opaque snapshot: no user-supplied numerical bound or source-id field.
#[derive(Clone, Debug)]
pub struct N01AffineTrial {
    stamp: SourceStamp,
}

/// Owner of an exactly specified, nonautonomous affine physical RHS/JVP law.
#[derive(Clone, Debug)]
pub struct N01AffineOwner {
    j0: DenseMatrix,
    slope_j: DenseMatrix,
    b0: Vec<f64>,
    slope_b: Vec<f64>,
    sigma: f64,
    w: Vec<f64>,
    p: Vec<f64>,
    reference_time: f64,
    time: f64,
    epoch: u64,
    digest: [u8; 32],
}

/// An enclosure in the physical units specified by `p`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct N01AffineReport {
    pub delta_j_upper: f64,
    pub delta_b_upper: f64,
    pub eta_apply_upper: f64,
    pub mu_upper: f64,
    pub margin_lower: f64,
    pub original_residual_upper: f64,
    pub physical_error_upper: f64,
    pub accepted: bool,
}

fn valid_positive(v: &[f64]) -> bool {
    v.iter().all(|x| x.is_finite() && *x > 0.0)
}

impl N01AffineOwner {
    /// Construct the source itself, not a caller-provided numerical certificate.
    /// All data are private and cannot be mutated without a new owner.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        j0: DenseMatrix,
        slope_j: DenseMatrix,
        b0: Vec<f64>,
        slope_b: Vec<f64>,
        sigma: f64,
        w: Vec<f64>,
        p: Vec<f64>,
        reference_time: f64,
    ) -> CoreResult<Self> {
        let n = b0.len();
        if n == 0
            || j0.nrows() != n
            || j0.ncols() != n
            || slope_j.nrows() != n
            || slope_j.ncols() != n
            || slope_b.len() != n
            || w.len() != n
            || p.len() != n
        {
            return Err(CoreError::Dimension("N01_AFFINE_SOURCE_SHAPE".into()));
        }
        if !reference_time.is_finite()
            || !sigma.is_finite()
            || sigma < 0.0
            || !b0.iter().chain(&slope_b).all(|x| x.is_finite())
            || !j0
                .as_slice()
                .iter()
                .chain(slope_j.as_slice())
                .all(|x| x.is_finite())
            || !valid_positive(&w)
            || !valid_positive(&p)
        {
            return Err(CoreError::InvalidInput(
                "N01_AFFINE_SOURCE_NONFINITE_OR_WEIGHTS".into(),
            ));
        }
        let mut represented = Vec::with_capacity(4 + 2 * n * n + 4 * n);
        represented.extend([n as f64, sigma, reference_time, -1.0]);
        represented.extend_from_slice(j0.as_slice());
        represented.extend_from_slice(slope_j.as_slice());
        represented.extend_from_slice(&b0);
        represented.extend_from_slice(&slope_b);
        represented.extend_from_slice(&w);
        represented.extend_from_slice(&p);
        let digest = sha256_f64_slice_bytes(&represented);
        Ok(Self {
            j0,
            slope_j,
            b0,
            slope_b,
            sigma,
            w,
            p,
            reference_time,
            time: reference_time,
            epoch: 0,
            digest,
        })
    }

    /// Change the physical model's clock; every actual transition increments
    /// the model epoch, invalidating all older snapshots without a caller-supplied ID.
    pub fn advance_time(&mut self, time: f64) -> CoreResult<()> {
        if !time.is_finite() {
            return Err(CoreError::NonFinite("N01_AFFINE_CLOCK".into()));
        }
        if time.to_bits() != self.time.to_bits() {
            let next = self
                .epoch
                .checked_add(1)
                .ok_or_else(|| CoreError::InvalidInput("N01_AFFINE_EPOCH_OVERFLOW".into()))?;
            self.time = time;
            self.epoch = next;
        }
        Ok(())
    }

    pub fn freeze(&self) -> N01AffineTrial {
        N01AffineTrial {
            stamp: SourceStamp {
                digest: self.digest,
                epoch: self.epoch,
                time_bits: self.time.to_bits(),
            },
        }
    }

    /// Evaluate the *owned* native JVP at the live clock, with ordinary
    /// binary64 arithmetic. It is a runtime callback, not itself a bound.
    /// Output is committed only after every component is finite.
    pub fn jvp_at_current(&self, v: &[f64], output: &mut [f64]) -> CoreResult<()> {
        let n = self.b0.len();
        if v.len() != n || output.len() != n {
            return Err(CoreError::Dimension("N01_AFFINE_JVP_SHAPE".into()));
        }
        if !v.iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite("N01_AFFINE_JVP_VECTOR".into()));
        }
        let dt = self.time - self.reference_time;
        if !dt.is_finite() {
            return Err(CoreError::NonFinite("N01_AFFINE_DT".into()));
        }
        let mut tmp = vec![0.0; n];
        for (i, yi) in tmp.iter_mut().enumerate() {
            for (j, &vj) in v.iter().enumerate() {
                *yi += (self.j0[(i, j)] + dt * self.slope_j[(i, j)]) * vj;
            }
            if !yi.is_finite() {
                return Err(CoreError::NonFinite("N01_AFFINE_JVP_OVERFLOW".into()));
            }
        }
        output.copy_from_slice(&tmp);
        Ok(())
    }

    /// Evaluate the owned nonautonomous affine physical RHS. Like `jvp`,
    /// binary64 callback output is not itself used as a rigorous enclosure.
    pub fn rhs_at_current(&self, x: &[f64], output: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.b0.len() || output.len() != x.len() {
            return Err(CoreError::Dimension("N01_AFFINE_RHS_SHAPE".into()));
        }
        let mut tmp = vec![0.0; x.len()];
        self.jvp_at_current(x, &mut tmp)?;
        let dt = self.time - self.reference_time;
        for (i, yi) in tmp.iter_mut().enumerate() {
            *yi += self.b0[i] + dt * self.slope_b[i];
            if !yi.is_finite() {
                return Err(CoreError::NonFinite("N01_AFFINE_RHS_OVERFLOW".into()));
            }
        }
        output.copy_from_slice(&tmp);
        Ok(())
    }

    /// Source-owned reference stage A0*xhat, and a separately certified
    /// application-error eta between this rounded result and exact A0*xhat.
    fn reference_applied(&self, xhat: &[f64]) -> CoreResult<Vec<f64>> {
        let mut result = vec![0.0; xhat.len()];
        for (i, yi) in result.iter_mut().enumerate() {
            let mut jx = 0.0;
            for (j, &xj) in xhat.iter().enumerate() {
                jx += self.j0[(i, j)] * xj;
            }
            *yi = xhat[i] - self.sigma * jx;
            if !yi.is_finite() {
                return Err(CoreError::NonFinite("N01_AFFINE_APPLY_OVERFLOW".into()));
            }
        }
        Ok(result)
    }

    /// Certify a candidate for the time-affine source under the *current*
    /// physical clock. The only external tolerances are thresholds, not
    /// estimates of model/JVP error. Both gates are enforced transactionally.
    pub fn try_commit(
        &self,
        trial: &N01AffineTrial,
        xhat: &[f64],
        original_tolerance: f64,
        physical_stage_tolerance: f64,
        output: &mut [f64],
    ) -> CoreResult<N01AffineReport> {
        let n = self.b0.len();
        if output.len() != n || xhat.len() != n {
            return Err(CoreError::Dimension("N01_AFFINE_COMMIT_SHAPE".into()));
        }
        if trial.stamp != self.freeze().stamp {
            return Err(CoreError::InvalidInput(
                "N01_AFFINE_STALE_SOURCE_CLOCK_EPOCH".into(),
            ));
        }
        if !(xhat.iter().all(|x| x.is_finite())
            && original_tolerance.is_finite()
            && original_tolerance >= 0.0
            && physical_stage_tolerance.is_finite()
            && physical_stage_tolerance >= 0.0)
        {
            return Err(CoreError::InvalidInput(
                "N01_AFFINE_BAD_CANDIDATE_OR_BUDGET".into(),
            ));
        }
        let dt = Interval::point(self.time)?.sub(Interval::point(self.reference_time)?)?;
        let dt_up = dt.mag();
        let y = self.reference_applied(xhat)?;
        let mut mu = f64::NEG_INFINITY;
        let mut delta_j_row = 0.0_f64;
        let mut delta_b = 0.0_f64;
        let mut eta = 0.0_f64;
        let mut r0 = 0.0_f64;
        let mut xnorm = 0.0_f64;
        let mut cwp = 0.0_f64;
        for i in 0..n {
            let mut mu_row = self.j0[(i, i)];
            let mut slope_row = 0.0;
            let mut jx = Interval::point(0.0)?;
            for (j, &xj) in xhat.iter().enumerate() {
                jx = jx.add(Interval::point(self.j0[(i, j)])?.mul(Interval::point(xj)?)?)?;
                let ratio = div_up(self.w[j], self.w[i])?;
                slope_row = add_up(slope_row, mul_up(self.slope_j[(i, j)].abs(), ratio)?)?;
                if i != j {
                    mu_row = add_up(mu_row, mul_up(self.j0[(i, j)].abs(), ratio)?)?;
                }
            }
            mu = mu.max(mu_row);
            delta_j_row = delta_j_row.max(slope_row);
            delta_b = delta_b.max(div_up(mul_up(dt_up, self.slope_b[i].abs())?, self.w[i])?);
            xnorm = xnorm.max(div_up(xhat[i].abs(), self.w[i])?);
            cwp = cwp.max(div_up(self.w[i], self.p[i])?);
            let exact_y = Interval::point(xhat[i])?.sub(Interval::point(self.sigma)?.mul(jx)?)?;
            eta = eta.max(div_up(
                Interval::point(y[i])?.sub(exact_y)?.mag(),
                self.w[i],
            )?);
            r0 = r0.max(div_up(
                Interval::point(self.b0[i])?
                    .sub(Interval::point(y[i])?)?
                    .mag(),
                self.w[i],
            )?);
        }
        let delta_j = mul_up(dt_up, delta_j_row)?;
        let model_res = add_up(add_up(r0, delta_b)?, eta)?;
        let delta_term = mul_up(mul_up(self.sigma, delta_j)?, xnorm)?;
        let rho = add_up(model_res, delta_term)?;
        let mu_perturbed = add_up(mu, delta_j)?;
        let margin = sub_down(1.0, mul_up(self.sigma, mu_perturbed)?)?;
        if !(margin.is_finite() && margin > 0.0) {
            return Err(CoreError::InvalidInput(
                "N01_AFFINE_NONPOSITIVE_RESOLVENT_MARGIN".into(),
            ));
        }
        let err = mul_up(mul_up(cwp, div_up(1.0, margin)?)?, rho)?;
        let accepted = rho <= original_tolerance && err <= physical_stage_tolerance;
        if accepted {
            output.copy_from_slice(xhat);
        }
        Ok(N01AffineReport {
            delta_j_upper: delta_j,
            delta_b_upper: delta_b,
            eta_apply_upper: eta,
            mu_upper: mu,
            margin_lower: margin,
            original_residual_upper: rho,
            physical_error_upper: err,
            accepted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn specimen(p: Vec<f64>) -> N01AffineOwner {
        let j0 = DenseMatrix::from_rows(&[&[-1., 10.], &[0., -1.]]).unwrap();
        let slope = DenseMatrix::from_rows(&[&[-1., 0.], &[0., -1.]]).unwrap();
        N01AffineOwner::new(
            j0,
            slope,
            vec![1., -2.],
            vec![0.5, -0.25],
            0.25,
            vec![100., 1.],
            p,
            0.,
        )
        .unwrap()
    }
    #[test]
    fn exact_dt_source_owned_delta_b_delta_j() {
        let mut owner = specimen(vec![100., 1.]);
        owner.advance_time(0.0625).unwrap();
        let frozen = owner.freeze();
        let mut output = [7., 8.];
        let report = owner
            .try_commit(&frozen, &[0., 0.], 10., 10., &mut output)
            .unwrap();
        assert!(report.delta_j_upper >= 0.0625);
        assert!(report.delta_b_upper >= 0.015625);
        assert!(report.mu_upper <= -0.9 + 1e-12);
        assert!(report.margin_lower > 1.0);
        assert!(report.eta_apply_upper < 1.0e-300);
    }
    #[test]
    fn output_only_commits_after_both_gates() {
        let mut o = specimen(vec![100., 1.]);
        o.advance_time(0.0625).unwrap();
        let f = o.freeze();
        let mut out = [77., 88.];
        let rep = o.try_commit(&f, &[1., -1.], 10., 0., &mut out).unwrap();
        assert!(!rep.accepted);
        assert_eq!(out, [77., 88.]);
        let rep2 = o.try_commit(&f, &[1., -1.], 10., 1000., &mut out).unwrap();
        assert!(rep2.accepted);
        assert_eq!(out, [1., -1.]);
    }
    #[test]
    fn stale_clock_epoch_rejected_even_when_coefficients_same() {
        let mut o = specimen(vec![100., 1.]);
        let t0 = o.freeze();
        o.advance_time(0.0625).unwrap();
        let mut out = [13., 21.];
        assert!(o.try_commit(&t0, &[0., 0.], 100., 100., &mut out).is_err());
        assert_eq!(out, [13., 21.]);
        let t1 = o.freeze();
        o.advance_time(0.0).unwrap();
        assert!(o.try_commit(&t1, &[0., 0.], 100., 100., &mut out).is_err());
        assert_eq!(out, [13., 21.]);
    }
    #[test]
    fn foreign_source_and_norm_rejected() {
        let o = specimen(vec![100., 1.]);
        let other = specimen(vec![1., 1.]);
        let mut out = [5., 6.];
        assert!(
            other
                .try_commit(&o.freeze(), &[0., 0.], 1., 1., &mut out)
                .is_err()
        );
        assert_eq!(out, [5., 6.]);
    }
    #[test]
    fn actual_rhs_and_jvp_current_time() {
        let mut o = specimen(vec![100., 1.]);
        o.advance_time(0.0625).unwrap();
        let mut j = [f64::NAN; 2];
        o.jvp_at_current(&[1., 1.], &mut j).unwrap();
        assert_eq!(j, [8.9375, -1.0625]);
        let mut b = [f64::NAN; 2];
        o.rhs_at_current(&[1., 1.], &mut b).unwrap();
        assert_eq!(b, [9.96875, -3.078125]);
    }
    #[test]
    fn bad_source_and_nonpositive_resolvent_refused() {
        let j = DenseMatrix::from_rows(&[&[2., 0.], &[0., 2.]]).unwrap();
        let z = DenseMatrix::from_rows(&[&[0., 0.], &[0., 0.]]).unwrap();
        let own = N01AffineOwner::new(
            j,
            z,
            vec![1., 0.],
            vec![0., 0.],
            1.,
            vec![1., 1.],
            vec![1., 1.],
            0.,
        )
        .unwrap();
        let mut out = [8., 9.];
        assert!(
            own.try_commit(&own.freeze(), &[0., 0.], 100., 100., &mut out)
                .is_err()
        );
        assert_eq!(out, [8., 9.]);
        let j = DenseMatrix::from_rows(&[&[-1., 0.], &[0., -1.]]).unwrap();
        let z = DenseMatrix::from_rows(&[&[0., 0.], &[0., 0.]]).unwrap();
        assert!(
            N01AffineOwner::new(
                j,
                z,
                vec![1., 0.],
                vec![0., 0.],
                0.25,
                vec![1., 0.],
                vec![1., 1.],
                0.
            )
            .is_err()
        );
    }
    #[test]
    fn nonfinite_clock_inputs_and_tolerances_fail_closed() {
        let mut o = specimen(vec![100., 1.]);
        assert!(o.advance_time(f64::NAN).is_err());
        let f = o.freeze();
        let mut out = [12., 34.];
        assert!(o.try_commit(&f, &[1., 1.], f64::NAN, 1., &mut out).is_err());
        assert!(o.try_commit(&f, &[1., 1.], 1., -1., &mut out).is_err());
        assert_eq!(out, [12., 34.]);
    }
    #[test]
    fn overflow_in_callback_is_transactional() {
        let j = DenseMatrix::from_rows(&[&[1e308]]).unwrap();
        let z = DenseMatrix::from_rows(&[&[0.]]).unwrap();
        let own =
            N01AffineOwner::new(j, z, vec![1.], vec![0.], 0.25, vec![1.], vec![1.], 0.).unwrap();
        let mut out = [55.];
        assert!(
            own.try_commit(&own.freeze(), &[1e308], 1e308, 1e308, &mut out)
                .is_err()
        );
        assert_eq!(out, [55.]);
    }
}
