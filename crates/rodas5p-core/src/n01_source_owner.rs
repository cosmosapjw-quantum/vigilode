//! Opt-in N01 represented-point certificate; NOT a physical JVP enclosure.
//!
//! This module owns an immutable *represented dense* J0, b0, clock, epoch,
//! physical weights and sigma. No generic callback or externally asserted
//! delta-J/delta-b/eta can enter the admission path. Actual physical-model
//! ownership and verified JVP uncertainty are deliberately NOT claimed.
//!
//! For A0=I-sigma*J0, sigma>=0 and weighted infinity norm
//! ||x||_w=max_i |x_i|/w_i, mu_w(J)=max_i[J_ii+sum_{j!=i}|J_ij|w_j/w_i],
//! if 1-sigma*mu_w(J0)>0, the represented-point forward error is bounded by
//! C_wp*||b0-A0*xhat||_w/(1-sigma*mu_w(J0)), C_wp=max_i w_i/p_i.
//! All computed bounds use the project's fallible outward arithmetic.

use crate::directed::{Interval, add_up, div_up, mul_up, sub_down};
use crate::{CoreError, CoreResult, DenseMatrix, DenseOperator, LinearOperator};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct N01SourceClock {
    pub source_id: [u8; 32],
    pub epoch: u64,
    pub clock_bits: u64,
}

impl N01SourceClock {
    pub fn new(source_id: [u8; 32], epoch: u64, time: f64) -> CoreResult<Self> {
        if !time.is_finite() {
            return Err(CoreError::NonFinite("N01 physical clock".into()));
        }
        Ok(Self {
            source_id,
            epoch,
            clock_bits: time.to_bits(),
        })
    }
}

/// Snapshot is private-by-construction; changes in any represented input,
/// norm, clock or epoch invalidate use. This does not authenticate a claimed
/// physical provider's source_id: full physics-source admission remains HOLD.
#[derive(Clone, Debug)]
pub struct N01PointOwner {
    stamp: N01SourceClock,
    jacobian: DenseMatrix,
    rhs: Vec<f64>,
    sigma: f64,
    w: Vec<f64>,
    p: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct N01PointReport {
    pub mu_upper: f64,
    pub resolvent_upper: f64,
    pub weighted_residual_upper: f64,
    pub physical_error_upper: f64,
    pub accepted: bool,
}

fn exact_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
}

fn positive_finite(xs: &[f64]) -> bool {
    xs.iter().all(|x| x.is_finite() && *x > 0.0)
}

impl N01PointOwner {
    pub fn new(
        stamp: N01SourceClock,
        op: &DenseOperator,
        b0: &[f64],
        sigma: f64,
        w: &[f64],
        p: &[f64],
    ) -> CoreResult<Self> {
        let j0 = op
            .explicit()
            .ok_or_else(|| CoreError::InvalidInput("N01_REQUIRES_NATIVE_DENSE_OPERATOR".into()))?;
        let n = b0.len();
        if !f64::from_bits(stamp.clock_bits).is_finite() {
            return Err(CoreError::NonFinite("N01 physical clock".into()));
        }
        if n == 0 || j0.nrows() != n || j0.ncols() != n || w.len() != n || p.len() != n {
            return Err(CoreError::Dimension("N01 represented-point shapes".into()));
        }
        if !(sigma.is_finite()
            && sigma >= 0.0
            && b0.iter().all(|x| x.is_finite())
            && j0.as_slice().iter().all(|x| x.is_finite())
            && positive_finite(w)
            && positive_finite(p))
        {
            return Err(CoreError::InvalidInput(
                "N01 nonfinite/negative model or weights".into(),
            ));
        }
        Ok(Self {
            stamp,
            jacobian: j0.clone(),
            rhs: b0.to_vec(),
            sigma,
            w: w.to_vec(),
            p: p.to_vec(),
        })
    }

    /// This *must* be called with the current source's model and norm, not a
    /// cached capsule. Exact binary64 identity prevents silent stale reuse.
    #[allow(clippy::too_many_arguments)]
    pub fn check_live(
        &self,
        stamp: N01SourceClock,
        op: &DenseOperator,
        b0: &[f64],
        sigma: f64,
        w: &[f64],
        p: &[f64],
    ) -> CoreResult<()> {
        let j0 = op
            .explicit()
            .ok_or_else(|| CoreError::InvalidInput("N01_REQUIRES_NATIVE_DENSE_OPERATOR".into()))?;
        if stamp != self.stamp
            || j0.nrows() != self.jacobian.nrows()
            || j0.ncols() != self.jacobian.ncols()
            || !exact_bits(j0.as_slice(), self.jacobian.as_slice())
            || !exact_bits(b0, &self.rhs)
            || sigma.to_bits() != self.sigma.to_bits()
            || !exact_bits(w, &self.w)
            || !exact_bits(p, &self.p)
        {
            return Err(CoreError::InvalidInput(
                "N01_STALE_SOURCE_CLOCK_EPOCH_NORM".into(),
            ));
        }
        Ok(())
    }

    /// Computes an outward original residual and a *represented-point*
    /// physical stage-error bound. A generic/JVP source is unsupported.
    /// Both numerical gates must pass. Does not mutate a solver output.
    fn certify_point(
        &self,
        xhat: &[f64],
        original_tolerance: f64,
        physical_stage_tolerance: f64,
    ) -> CoreResult<N01PointReport> {
        let n = self.rhs.len();
        if xhat.len() != n {
            return Err(CoreError::Dimension("N01 candidate dimension".into()));
        }
        if !(xhat.iter().all(|x| x.is_finite())
            && original_tolerance.is_finite()
            && original_tolerance >= 0.0
            && physical_stage_tolerance.is_finite()
            && physical_stage_tolerance >= 0.0)
        {
            return Err(CoreError::InvalidInput(
                "N01 nonfinite candidate/budget".into(),
            ));
        }
        let mut mu_upper = f64::NEG_INFINITY;
        let mut cwp_upper = 0.0_f64;
        let mut residual_upper = 0.0_f64;
        for i in 0..n {
            let mut row_mu = self.jacobian[(i, i)];
            for j in 0..n {
                if i != j {
                    let ratio_up = div_up(self.w[j], self.w[i])?;
                    let term_up = mul_up(self.jacobian[(i, j)].abs(), ratio_up)?;
                    row_mu = add_up(row_mu, term_up)?;
                }
            }
            mu_upper = mu_upper.max(row_mu);
            cwp_upper = cwp_upper.max(div_up(self.w[i], self.p[i])?);
            // Direct interval evaluation of b0 - (I-sigma J0)xhat.
            // eta_apply=0 here because this includes *all* represented
            // arithmetic error instead of trusting a floating callback.
            let mut jx = Interval::point(0.0)?;
            for (j, &xj) in xhat.iter().enumerate() {
                jx = jx.add(Interval::point(self.jacobian[(i, j)])?.mul(Interval::point(xj)?)?)?;
            }
            let r = Interval::point(self.rhs[i])?
                .sub(Interval::point(xhat[i])?)?
                .add(Interval::point(self.sigma)?.mul(jx)?)?;
            residual_upper = residual_upper.max(div_up(r.mag(), self.w[i])?);
        }
        let sigma_mu_upper = mul_up(self.sigma, mu_upper)?;
        let margin_lower = sub_down(1.0, sigma_mu_upper)?;
        if !(margin_lower.is_finite() && margin_lower > 0.0) {
            return Err(CoreError::InvalidInput(
                "N01_NONPOSITIVE_RESOLVENT_MARGIN".into(),
            ));
        }
        let resolvent_upper = div_up(1.0, margin_lower)?;
        let physical_error_upper = mul_up(mul_up(cwp_upper, resolvent_upper)?, residual_upper)?;
        Ok(N01PointReport {
            mu_upper,
            resolvent_upper,
            weighted_residual_upper: residual_upper,
            physical_error_upper,
            accepted: residual_upper <= original_tolerance
                && physical_error_upper <= physical_stage_tolerance,
        })
    }

    /// A caller cannot convert a guessed callback/JVP uncertainty into an
    /// admitted N01 certificate. Native physical bounds owner is absent.
    pub fn refuse_unverified_jvp(
        &self,
        _claimed_delta_j: f64,
        _claimed_eta: f64,
    ) -> CoreResult<()> {
        Err(CoreError::InvalidInput(
            "N01_JVP_UNCERTAINTY_OWNER_NOT_VERIFIED".into(),
        ))
    }

    /// Two-phase transactional commit. Any stale source, invalid input,
    /// or unsuccessful numerical gate leaves output bitwise unchanged.
    #[allow(clippy::too_many_arguments)]
    pub fn try_commit_point(
        &self,
        live_stamp: N01SourceClock,
        live_op: &DenseOperator,
        live_b0: &[f64],
        live_sigma: f64,
        live_w: &[f64],
        live_p: &[f64],
        xhat: &[f64],
        original_tolerance: f64,
        physical_stage_tolerance: f64,
        output: &mut [f64],
    ) -> CoreResult<N01PointReport> {
        if output.len() != self.rhs.len() {
            return Err(CoreError::Dimension("N01 output dimension".into()));
        }
        self.check_live(live_stamp, live_op, live_b0, live_sigma, live_w, live_p)?;
        let report = self.certify_point(xhat, original_tolerance, physical_stage_tolerance)?;
        if report.accepted {
            output.copy_from_slice(xhat);
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> (
        N01SourceClock,
        DenseOperator,
        Vec<f64>,
        Vec<f64>,
        Vec<f64>,
        N01PointOwner,
    ) {
        let stamp = N01SourceClock::new([13; 32], 7, 0.125).unwrap();
        let j = DenseMatrix::from_rows(&[&[-1.0, 10.0], &[0.0, -1.0]]).unwrap();
        let b = vec![1.0, -2.0];
        let w = vec![100.0, 1.0];
        let p = vec![1.0, 1.0];
        let op = DenseOperator::new(j).unwrap();
        let owner = N01PointOwner::new(stamp, &op, &b, 0.25, &w, &p).unwrap();
        (stamp, op, b, w, p, owner)
    }
    #[test]
    fn nonnormal_lognorm_and_conversion() {
        let (_, _, _, _, _, o) = sample();
        let rep = o.certify_point(&[-2.4, -1.6], 1e-12, 1e-9).unwrap();
        assert!(rep.mu_upper <= -0.9 + 1e-14);
        assert!(rep.resolvent_upper <= 40.0 / 49.0 + 1e-12);
        assert!(rep.accepted);
    }
    #[test]
    fn residual_only_does_not_bypass_physical_gate() {
        let (_, _, _, _, _, o) = sample();
        let rep = o.certify_point(&[-2.399999, -1.6], 1e-5, 1e-10).unwrap();
        assert!(!rep.accepted);
    }
    #[test]
    fn rejected_physical_gate_keeps_output() {
        let (stamp, op, b, w, p, owner) = sample();
        let mut output = vec![11.0, 22.0];
        let rep = owner
            .try_commit_point(
                stamp,
                &op,
                &b,
                0.25,
                &w,
                &p,
                &[-2.399999, -1.6],
                1e-5,
                1e-10,
                &mut output,
            )
            .unwrap();
        assert!(!rep.accepted);
        assert_eq!(output, [11.0, 22.0]);
    }
    #[test]
    fn stale_epoch_clock_and_source_refuse_transactionally() {
        let (stamp, j, b, w, p, o) = sample();
        let mut output = vec![321.0, 654.0];
        let mut stale = stamp;
        stale.epoch += 1;
        assert!(
            o.try_commit_point(
                stale,
                &j,
                &b,
                0.25,
                &w,
                &p,
                &[-2.4, -1.6],
                1.0,
                1.0,
                &mut output
            )
            .is_err()
        );
        stale = stamp;
        stale.clock_bits = 0;
        assert!(
            o.try_commit_point(
                stale,
                &j,
                &b,
                0.25,
                &w,
                &p,
                &[-2.4, -1.6],
                1.0,
                1.0,
                &mut output
            )
            .is_err()
        );
        stale = stamp;
        stale.source_id = [7; 32];
        assert!(
            o.try_commit_point(
                stale,
                &j,
                &b,
                0.25,
                &w,
                &p,
                &[-2.4, -1.6],
                1.0,
                1.0,
                &mut output
            )
            .is_err()
        );
        assert_eq!(output, vec![321.0, 654.0]);
    }
    #[test]
    fn mutated_norm_rhs_and_jacobian_refuse() {
        let (stamp, j, b, w, p, o) = sample();
        let mut output = vec![5.0; 2];
        let mut w2 = w.clone();
        w2[0] *= 2.0;
        assert!(
            o.try_commit_point(
                stamp,
                &j,
                &b,
                0.25,
                &w2,
                &p,
                &[-2.4, -1.6],
                1.0,
                1.0,
                &mut output
            )
            .is_err()
        );
        let mut b2 = b.clone();
        b2[0] += 1.0;
        assert!(
            o.try_commit_point(
                stamp,
                &j,
                &b2,
                0.25,
                &w,
                &p,
                &[-2.4, -1.6],
                1.0,
                1.0,
                &mut output
            )
            .is_err()
        );
        let mut j2 = j.explicit().unwrap().clone();
        j2.as_mut_slice()[0] += 1.0;
        let op2 = DenseOperator::new(j2).unwrap();
        assert!(
            o.try_commit_point(
                stamp,
                &op2,
                &b,
                0.25,
                &w,
                &p,
                &[-2.4, -1.6],
                1.0,
                1.0,
                &mut output
            )
            .is_err()
        );
        assert_eq!(output, vec![5.0; 2]);
    }
    #[test]
    fn rejects_unbounded_and_unverified_callback() {
        let (_, j, b, w, p, o) = sample();
        assert!(N01PointOwner::new(o.stamp, &j, &b, f64::NAN, &w, &p).is_err());
        assert!(N01PointOwner::new(o.stamp, &j, &b, 0.25, &[0.0, 1.0], &p).is_err());
        assert!(o.refuse_unverified_jvp(0.0, 0.0).is_err());
        assert!(o.certify_point(&[f64::NAN, 0.0], 1.0, 1.0).is_err());
        assert!(o.certify_point(&[1.0, 1.0], f64::INFINITY, 1.0).is_err());
        let mut forged = o.stamp;
        forged.clock_bits = f64::NAN.to_bits();
        assert!(N01PointOwner::new(forged, &j, &b, 0.25, &w, &p).is_err());
    }
    #[test]
    fn resolvent_margin_and_no_output_mutation() {
        let (stamp, _, b, w, p, _) = sample();
        let j = DenseMatrix::from_rows(&[&[2.0, 0.0], &[0.0, 1.0]]).unwrap();
        let op = DenseOperator::new(j).unwrap();
        let o = N01PointOwner::new(stamp, &op, &b, 1.0, &w, &p).unwrap();
        let mut output = vec![8.0, 9.0];
        assert!(
            o.try_commit_point(
                stamp,
                &op,
                &b,
                1.0,
                &w,
                &p,
                &[0.0, 0.0],
                1e3,
                1e3,
                &mut output
            )
            .is_err()
        );
        assert_eq!(output, vec![8.0, 9.0]);
    }
}
