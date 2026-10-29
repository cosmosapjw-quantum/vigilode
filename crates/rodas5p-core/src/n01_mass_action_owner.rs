//! Opt-in, source-owned N01 nonlinear Robertson-type mass-action stage certificate.
//!
//! This is a *real nonlinear polynomial model* (not a generic callback proof):
//!   f(a,b,c)=(-k1*a+k2*b*c, k1*a-k2*b*c-k3*b*b, k3*b*b).
//! Rates and inputs are exact real values of the supplied binary64 data.
//! A native source owns RHS, J, JVP, the clock, state, epoch, weights and
//! source instance. Its linear-stage problem is
//!   (I-sigma J(state)) x = f(state).
//! The reference J0 and b0 are the *represented binary64* Jacobian/RHS at
//! the anchor. Outward interval calculations include all reference-rounding
//! errors in deltaJ and deltaB. The rounded reference operator's application
//! error eta is independently enclosed. Both original residual and physical
//! forward-stage-error gates must pass before output changes.
//!
//! No arbitrary OdeProblem callback, whole-slab flow tube, global IVP error,
//! nonlinear Newton convergence or production BASS chemistry is certified.

use crate::directed::{Interval, add_up, div_up, mul_up, sub_down};
use crate::{CoreError, CoreResult};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_N01_MASS_OWNER: AtomicU64 = AtomicU64::new(1);
const N: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SourceStamp {
    owner: u64,
    epoch: u64,
    clock_bits: u64,
    state_bits: [u64; N],
}

/// Unforgeable through the public API: only the owning source produces it.
#[derive(Clone, Debug)]
pub struct N01MassActionTrial {
    stamp: SourceStamp,
}

/// Source parameters are private. `advance` is the only state/clock mutation.
#[derive(Debug)]
pub struct N01MassActionOwner {
    rates: [f64; N],
    reference: [f64; N],
    radius: [f64; N],
    current: [f64; N],
    clock: f64,
    epoch: u64,
    owner: u64,
    sigma: f64,
    w: [f64; N],
    p: [f64; N],
    reference_j: [[f64; N]; N],
    reference_b: [f64; N],
}

/// All upper/lower bounds are outward binary64 enclosures of exact reals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct N01MassActionReport {
    pub delta_j_upper: f64,
    pub delta_b_upper: f64,
    pub eta_apply_upper: f64,
    pub mu_upper: f64,
    pub margin_lower: f64,
    pub original_residual_upper: f64,
    pub physical_error_upper: f64,
    pub accepted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct N01MassActionJvpReport {
    /// Difference between the rounded JVP callback and exact J(current)*v.
    pub callback_rounding_upper: f64,
    /// Difference between the rounded callback and exact J0*v.
    pub reference_jvp_difference_upper: f64,
}

fn finite3(x: &[f64; N]) -> bool {
    x.iter().all(|v| v.is_finite())
}
fn positive3(x: &[f64; N]) -> bool {
    x.iter().all(|v| v.is_finite() && *v > 0.0)
}
fn nonnegative3(x: &[f64; N]) -> bool {
    x.iter().all(|v| v.is_finite() && *v >= 0.0)
}
fn point(x: f64) -> CoreResult<Interval> {
    Interval::point(x)
}

/// Floating callback with all intermediate nonfinite failures made explicit.
fn rhs_float(k: &[f64; N], z: &[f64; N]) -> CoreResult<[f64; N]> {
    if !nonnegative3(z) {
        return Err(CoreError::InvalidInput(
            "N01_MASS_RHS_PHYSICAL_STATE".into(),
        ));
    }
    let r1 = k[0] * z[0];
    let r2 = (k[1] * z[1]) * z[2];
    let r3 = (k[2] * z[1]) * z[1];
    let out = [-r1 + r2, r1 - r2 - r3, r3];
    if !finite3(&out) {
        return Err(CoreError::NonFinite("N01_MASS_RHS_OVERFLOW".into()));
    }
    Ok(out)
}

fn jac_float(k: &[f64; N], z: &[f64; N]) -> CoreResult<[[f64; N]; N]> {
    if !nonnegative3(z) {
        return Err(CoreError::InvalidInput("N01_MASS_J_PHYSICAL_STATE".into()));
    }
    let bc = k[1] * z[2];
    let bb = k[1] * z[1];
    let two_b = (2.0 * k[2]) * z[1];
    let out = [[-k[0], bc, bb], [k[0], -bc - two_b, -bb], [0.0, two_b, 0.0]];
    if !out.iter().flatten().all(|x| x.is_finite()) {
        return Err(CoreError::NonFinite("N01_MASS_J_OVERFLOW".into()));
    }
    Ok(out)
}

fn rhs_interval(k: &[f64; N], z: &[f64; N]) -> CoreResult<[Interval; N]> {
    let r1 = point(k[0])?.mul(point(z[0])?)?;
    let r2 = point(k[1])?.mul(point(z[1])?)?.mul(point(z[2])?)?;
    let r3 = point(k[2])?.mul(point(z[1])?)?.mul(point(z[1])?)?;
    Ok([(-r1).add(r2)?, r1.sub(r2)?.sub(r3)?, r3])
}

fn jac_interval(k: &[f64; N], z: &[f64; N]) -> CoreResult<[[Interval; N]; N]> {
    let zero = point(0.0)?;
    let bc = point(k[1])?.mul(point(z[2])?)?;
    let bb = point(k[1])?.mul(point(z[1])?)?;
    let two_b = point(2.0)?.mul(point(k[2])?)?.mul(point(z[1])?)?;
    Ok([
        [-point(k[0])?, bc, bb],
        [point(k[0])?, (-bc).sub(two_b)?, -bb],
        [zero, two_b, zero],
    ])
}

impl N01MassActionOwner {
    /// The exact reference domain is the represented anchor plus a fixed box.
    /// Rate units: inverse time for dimensionless concentrations.
    pub fn new(
        rates: [f64; N],
        reference: [f64; N],
        radius: [f64; N],
        sigma: f64,
        w: [f64; N],
        p: [f64; N],
        clock: f64,
    ) -> CoreResult<Self> {
        if !nonnegative3(&rates)
            || !nonnegative3(&reference)
            || !nonnegative3(&radius)
            || !positive3(&w)
            || !positive3(&p)
            || !sigma.is_finite()
            || sigma < 0.0
            || !clock.is_finite()
        {
            return Err(CoreError::InvalidInput("N01_MASS_SOURCE_INPUT".into()));
        }
        let reference_j = jac_float(&rates, &reference)?;
        let reference_b = rhs_float(&rates, &reference)?;
        let owner = NEXT_N01_MASS_OWNER.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            rates,
            reference,
            radius,
            current: reference,
            clock,
            epoch: 0,
            owner,
            sigma,
            w,
            p,
            reference_j,
            reference_b,
        })
    }

    /// Refuse an out-of-box state before updating clock, epoch or state.
    pub fn advance(&mut self, time: f64, state: [f64; N]) -> CoreResult<()> {
        if !time.is_finite() || !nonnegative3(&state) {
            return Err(CoreError::InvalidInput("N01_MASS_BAD_STATE_CLOCK".into()));
        }
        for (i, &value) in state.iter().enumerate() {
            let distance_up = point(value)?.sub(point(self.reference[i])?)?.mag();
            if distance_up > self.radius[i] {
                return Err(CoreError::InvalidInput(
                    "N01_MASS_OUTSIDE_CERTIFIED_BOX".into(),
                ));
            }
        }
        if time.to_bits() != self.clock.to_bits()
            || state
                .iter()
                .zip(&self.current)
                .any(|(a, b)| a.to_bits() != b.to_bits())
        {
            let next = self
                .epoch
                .checked_add(1)
                .ok_or_else(|| CoreError::InvalidInput("N01_MASS_EPOCH_OVERFLOW".into()))?;
            self.clock = time;
            self.current = state;
            self.epoch = next;
        }
        Ok(())
    }

    /// Native OdeProblem model_epoch integration; changes on clock or state update.
    pub fn model_epoch(&self) -> u64 {
        self.epoch
    }

    pub fn freeze(&self) -> N01MassActionTrial {
        N01MassActionTrial {
            stamp: SourceStamp {
                owner: self.owner,
                epoch: self.epoch,
                clock_bits: self.clock.to_bits(),
                state_bits: self.current.map(f64::to_bits),
            },
        }
    }

    fn live(&self, trial: &N01MassActionTrial) -> CoreResult<()> {
        if trial.stamp != self.freeze().stamp {
            Err(CoreError::InvalidInput(
                "N01_MASS_STALE_SOURCE_CLOCK_EPOCH_STATE".into(),
            ))
        } else {
            Ok(())
        }
    }

    /// An actual source-owned nonlinear RHS callback. No output mutation on error.
    pub fn rhs_at(&self, state: &[f64; N], output: &mut [f64; N]) -> CoreResult<()> {
        let value = rhs_float(&self.rates, state)?;
        output.copy_from_slice(&value);
        Ok(())
    }

    pub fn jacobian_at(&self, state: &[f64; N]) -> CoreResult<[[f64; N]; N]> {
        jac_float(&self.rates, state)
    }

    /// Actual JVP callback with the same immutable algebra as `rhs_at`.
    pub fn jvp_at(&self, state: &[f64; N], v: &[f64; N], output: &mut [f64; N]) -> CoreResult<()> {
        if !finite3(v) {
            return Err(CoreError::NonFinite("N01_MASS_JVP_VECTOR".into()));
        }
        let j = self.jacobian_at(state)?;
        let mut tmp = [0.0; N];
        for i in 0..N {
            for (jj, &vj) in v.iter().enumerate() {
                tmp[i] += j[i][jj] * vj;
            }
        }
        if !finite3(&tmp) {
            return Err(CoreError::NonFinite("N01_MASS_JVP_OVERFLOW".into()));
        }
        output.copy_from_slice(&tmp);
        Ok(())
    }

    pub fn source_owned_jvp_report(
        &self,
        trial: &N01MassActionTrial,
        v: &[f64; N],
    ) -> CoreResult<N01MassActionJvpReport> {
        self.live(trial)?;
        let mut computed = [0.0; N];
        self.jvp_at(&self.current, v, &mut computed)?;
        let exact = jac_interval(&self.rates, &self.current)?;
        let mut rounding = 0.0_f64;
        let mut reference = 0.0_f64;
        for i in 0..N {
            let mut real = point(0.0)?;
            let mut old = point(0.0)?;
            for (j, &x) in v.iter().enumerate() {
                real = real.add(exact[i][j].mul(point(x)?)?)?;
                old = old.add(point(self.reference_j[i][j])?.mul(point(x)?)?)?;
            }
            rounding = rounding.max(div_up(point(computed[i])?.sub(real)?.mag(), self.w[i])?);
            reference = reference.max(div_up(point(computed[i])?.sub(old)?.mag(), self.w[i])?);
        }
        Ok(N01MassActionJvpReport {
            callback_rounding_upper: rounding,
            reference_jvp_difference_upper: reference,
        })
    }

    fn reference_applied(&self, x: &[f64; N]) -> CoreResult<[f64; N]> {
        let mut y = [0.0; N];
        for i in 0..N {
            let mut jx = 0.0;
            for (j, &xx) in x.iter().enumerate() {
                jx += self.reference_j[i][j] * xx;
            }
            y[i] = x[i] - self.sigma * jx;
        }
        if !finite3(&y) {
            return Err(CoreError::NonFinite("N01_MASS_APPLY_OVERFLOW".into()));
        }
        Ok(y)
    }

    /// Certificate is valid for the exact-real stage matrix at *this* live state.
    /// It does not imply the nonlinear ODE step is accurate or globally stable.
    pub fn try_commit(
        &self,
        trial: &N01MassActionTrial,
        xhat: &[f64; N],
        original_tolerance: f64,
        physical_stage_tolerance: f64,
        output: &mut [f64; N],
    ) -> CoreResult<N01MassActionReport> {
        self.live(trial)?;
        if !finite3(xhat)
            || !original_tolerance.is_finite()
            || original_tolerance < 0.0
            || !physical_stage_tolerance.is_finite()
            || physical_stage_tolerance < 0.0
        {
            return Err(CoreError::InvalidInput(
                "N01_MASS_BAD_CANDIDATE_OR_BUDGET".into(),
            ));
        }
        let y = self.reference_applied(xhat)?;
        let real_j = jac_interval(&self.rates, &self.current)?;
        let real_b = rhs_interval(&self.rates, &self.current)?;
        let mut delta_j = 0.0_f64;
        let mut delta_b = 0.0_f64;
        let mut eta = 0.0_f64;
        let mut rho0 = 0.0_f64;
        let mut xnorm = 0.0_f64;
        let mut cwp = 0.0_f64;
        let mut mu = f64::NEG_INFINITY;
        for i in 0..N {
            let mut delta_row = 0.0;
            let mut mu_row = self.reference_j[i][i];
            let mut nominal_jx = point(0.0)?;
            for (j, &xx) in xhat.iter().enumerate() {
                let jj = self.reference_j[i][j];
                let ratio = div_up(self.w[j], self.w[i])?;
                let d = real_j[i][j].sub(point(jj)?)?;
                delta_row = add_up(delta_row, mul_up(d.mag(), ratio)?)?;
                nominal_jx = nominal_jx.add(point(jj)?.mul(point(xx)?)?)?;
                if i != j {
                    mu_row = add_up(mu_row, mul_up(jj.abs(), ratio)?)?;
                }
            }
            delta_j = delta_j.max(delta_row);
            mu = mu.max(mu_row);
            delta_b = delta_b.max(div_up(
                real_b[i].sub(point(self.reference_b[i])?)?.mag(),
                self.w[i],
            )?);
            let exact_y = point(xhat[i])?.sub(point(self.sigma)?.mul(nominal_jx)?)?;
            eta = eta.max(div_up(point(y[i])?.sub(exact_y)?.mag(), self.w[i])?);
            rho0 = rho0.max(div_up(
                point(self.reference_b[i])?.sub(point(y[i])?)?.mag(),
                self.w[i],
            )?);
            xnorm = xnorm.max(div_up(xhat[i].abs(), self.w[i])?);
            cwp = cwp.max(div_up(self.w[i], self.p[i])?);
        }
        let rho = add_up(
            add_up(add_up(rho0, delta_b)?, eta)?,
            mul_up(mul_up(self.sigma, delta_j)?, xnorm)?,
        )?;
        let margin = sub_down(1.0, mul_up(self.sigma, add_up(mu, delta_j)?)?)?;
        if !(margin > 0.0 && margin.is_finite()) {
            return Err(CoreError::InvalidInput(
                "N01_MASS_NONPOSITIVE_RESOLVENT_MARGIN".into(),
            ));
        }
        let forward = div_up(mul_up(cwp, rho)?, margin)?;
        let accepted = rho <= original_tolerance && forward <= physical_stage_tolerance;
        if accepted {
            output.copy_from_slice(xhat);
        }
        Ok(N01MassActionReport {
            delta_j_upper: delta_j,
            delta_b_upper: delta_b,
            eta_apply_upper: eta,
            mu_upper: mu,
            margin_lower: margin,
            original_residual_upper: rho,
            physical_error_upper: forward,
            accepted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owner() -> N01MassActionOwner {
        N01MassActionOwner::new(
            [0.0625, 8.0, 4.0],
            [0.875, 0.0625, 0.0625],
            [0.125, 0.125, 0.125],
            0.0625,
            [1.0, 1.0, 1.0],
            [1.0, 2.0, 1.0],
            0.0,
        )
        .unwrap()
    }
    #[test]
    fn nonlinear_mass_conservation_and_jvp() {
        let model = owner();
        let s = [0.859375, 0.078125, 0.0625];
        let mut f = [0.; 3];
        model.rhs_at(&s, &mut f).unwrap();
        assert!((f.iter().sum::<f64>()).abs() < 1e-15);
        let j = model.jacobian_at(&s).unwrap();
        for (c, _) in j[0].iter().enumerate() {
            assert!((j[0][c] + j[1][c] + j[2][c]).abs() < 1e-15);
        }
        let mut jv = [0.; 3];
        model.jvp_at(&s, &[1.0, -1.0, 0.5], &mut jv).unwrap();
        for r in 0..3 {
            assert!((jv[r] - (j[r][0] - j[r][1] + 0.5 * j[r][2])).abs() < 1e-15);
        }
    }
    #[test]
    fn finite_stage_and_jvp_gap() {
        let mut model = owner();
        model.advance(0.125, [0.859375, 0.078125, 0.0625]).unwrap();
        let snap = model.freeze();
        let res = model
            .try_commit(&snap, &[0.1, 0.2, 0.3], 10.0, 10.0, &mut [0.0; 3])
            .unwrap();
        assert!(res.margin_lower > 0.0 && res.delta_j_upper > 0.0 && res.delta_b_upper > 0.0);
        let jvp = model
            .source_owned_jvp_report(&snap, &[0.125, -0.25, 0.5])
            .unwrap();
        assert!(jvp.callback_rounding_upper.is_finite());
        assert!(jvp.reference_jvp_difference_upper >= 0.0);
    }
    #[test]
    fn original_gate_and_physical_gate_both_required() {
        let model = owner();
        let snap = model.freeze();
        let mut output = [7.0, 8.0, 9.0];
        let failed = model
            .try_commit(&snap, &[0.1, 0.2, 0.3], 0.0, 10.0, &mut output)
            .unwrap();
        assert!(!failed.accepted && output == [7.0, 8.0, 9.0]);
        let failed = model
            .try_commit(&snap, &[0.1, 0.2, 0.3], 10.0, 0.0, &mut output)
            .unwrap();
        assert!(!failed.accepted && output == [7.0, 8.0, 9.0]);
        let ok = model
            .try_commit(&snap, &[0.1, 0.2, 0.3], 10.0, 10.0, &mut output)
            .unwrap();
        assert!(ok.accepted && output == [0.1, 0.2, 0.3]);
    }
    #[test]
    fn stale_clock_and_state_refuse_without_output_mutation() {
        let mut model = owner();
        let snap = model.freeze();
        model.advance(1.0, [0.859375, 0.078125, 0.0625]).unwrap();
        let mut out = [15.0, 16.0, 17.0];
        assert!(
            model
                .try_commit(&snap, &[0.0; 3], 10.0, 10.0, &mut out)
                .is_err()
        );
        assert_eq!(out, [15.0, 16.0, 17.0]);
        let fresh = model.freeze();
        model.advance(2.0, [0.859375, 0.078125, 0.0625]).unwrap();
        assert!(model.source_owned_jvp_report(&fresh, &[1.0; 3]).is_err());
    }
    #[test]
    fn foreign_owner_cannot_reuse_snapshot() {
        let one = owner();
        let two = owner();
        let snap = one.freeze();
        let mut out = [12.0; 3];
        assert!(two.try_commit(&snap, &[0.; 3], 10., 10., &mut out).is_err());
        assert_eq!(out, [12.0; 3]);
    }
    #[test]
    fn negative_and_outside_box_state_fail_transactionally() {
        let mut model = owner();
        let snap = model.freeze();
        assert!(model.advance(1.0, [-0.1, 0.1, 1.0]).is_err());
        assert!(model.advance(1.0, [0.5, 0.1, 0.4]).is_err());
        assert_eq!(model.freeze().stamp, snap.stamp);
    }
    #[test]
    fn invalid_weights_rates_and_nonfinite_rejected() {
        let m = N01MassActionOwner::new(
            [0.04, -1., 2.],
            [1.0, 0.0, 0.0],
            [0.1; 3],
            0.1,
            [1.; 3],
            [1.; 3],
            0.,
        );
        assert!(m.is_err());
        let m = N01MassActionOwner::new(
            [0.04, 1., 2.],
            [1.0, 0.0, 0.0],
            [0.1; 3],
            0.1,
            [1., 0.0, 1.],
            [1.; 3],
            0.,
        );
        assert!(m.is_err());
        let model = owner();
        let mut out = [42.0; 3];
        assert!(
            model
                .jvp_at(&[1., 0., 0.], &[f64::NAN, 0., 0.], &mut out)
                .is_err()
        );
        assert_eq!(out, [42.0; 3]);
    }
    #[test]
    fn nonpositive_margin_refuses_even_generous_budget() {
        let model = N01MassActionOwner::new(
            [0.0625, 8.0, 4.0],
            [0.875, 0.0625, 0.0625],
            [0.125; 3],
            64.0,
            [1.; 3],
            [1.; 3],
            0.0,
        )
        .unwrap();
        let mut out = [7.0; 3];
        assert!(
            model
                .try_commit(&model.freeze(), &[0.5; 3], 1e9, 1e9, &mut out)
                .is_err()
        );
        assert_eq!(out, [7.; 3]);
    }
    #[test]
    fn canonical_robertson_rates_stay_source_owned() {
        let mut model = N01MassActionOwner::new(
            [0.04, 1e4, 3e7],
            [0.9, 1e-5, 0.09999],
            [0.01, 1e-6, 0.01],
            1e-5,
            [1.; 3],
            [1.; 3],
            0.,
        )
        .unwrap();
        model
            .advance(0.001, [0.9001, 1.0001e-5, 0.099889999])
            .unwrap();
        let trial = model.freeze();
        let rep = model
            .try_commit(&trial, &[0.0; 3], 1e6, 1e6, &mut [0.0; 3])
            .unwrap();
        assert!(rep.margin_lower > 0.0 && rep.original_residual_upper.is_finite());
        let jvp = model
            .source_owned_jvp_report(&trial, &[0.125, 0.25, -0.25])
            .unwrap();
        assert!(jvp.callback_rounding_upper.is_finite());
    }
    #[test]
    fn public_callbacks_reject_negative_concentrations() {
        let model = owner();
        let mut y = [42.; 3];
        assert!(model.rhs_at(&[-0.01, 0.0, 1.0], &mut y).is_err());
        assert!(model.jacobian_at(&[-0.01, 0.0, 1.0]).is_err());
        assert_eq!(y, [42.; 3]);
    }
    #[test]
    fn callback_overflow_refuses_without_writing() {
        let model = N01MassActionOwner::new(
            [1e250, 1e250, 1e250],
            [0.0, 0.0, 0.0],
            [1e250; 3],
            0.0,
            [1.; 3],
            [1.; 3],
            0.0,
        )
        .unwrap();
        let mut out = [3.0; 3];
        assert!(model.rhs_at(&[1e250; 3], &mut out).is_err());
        assert_eq!(out, [3.0; 3]);
    }
}