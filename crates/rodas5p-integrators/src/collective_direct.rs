//! O(N) direct solve for one frozen rotating-frame collective stage matrix.
//!
//! For `A = I - s J(y)`, the collective cubic Jacobian is block diagonal plus
//! rank two. This module applies the corresponding 2x2 Woodbury Schur solve.
//! It is a stage linear algebra primitive; it does not establish trajectory
//! accuracy or a runtime comparison.

use rodas5p_core::directed::{Interval, add_up, div_up, mul_up, sqrt_up, sub_down};
use rodas5p_core::{CoreError, CoreResult};

use crate::collective_model::CollectiveModel;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CollectiveDirectWork {
    /// Conservative accounting proxies, not measured scalar-operation counts.
    pub setup_ops: u64,
    pub setup_allocations: u64,
    pub setup_bytes: u64,
    pub identity_check_ops: u64,
    pub schur_certificate_ops: u64,
    pub rhs_ops: u64,
    pub reconstruction_ops: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollectiveResidualWitness {
    pub weighted_residual_upper: f64,
    pub inverse_norm_upper: f64,
    pub solution_error_upper: f64,
}

#[derive(Clone, Copy, Debug)]
struct Block {
    p: [f64; 4],
}

/// Frozen factors for `A = I - s J(y)` in rotating coordinates (`omega=width*xi`).
#[derive(Clone, Debug)]
pub struct CollectiveStageFactors {
    n: usize,
    s: f64,
    weights: Vec<f64>,
    blocks: Vec<Block>,
    c: [f64; 4],
    schur_inverse: [f64; 4],
    schur_det_interval: Interval,
    inverse_norm_upper: f64,
    source_identity: Vec<u64>,
}

fn point(x: f64) -> CoreResult<Interval> {
    Interval::point(x)
}
impl CollectiveStageFactors {
    /// Build the block inverses and Schur complement for a frozen state.
    pub fn factorize(
        model: &CollectiveModel,
        y: &[f64],
        s: f64,
        work: &mut CollectiveDirectWork,
    ) -> CoreResult<Self> {
        let n = model.num_modes();
        if y.len() != 2 * n {
            return Err(CoreError::Dimension(
                "collective direct factorize state dimension mismatch".into(),
            ));
        }
        if model.carrier() != 0.0 {
            return Err(CoreError::InvalidInput(
                "collective direct adapter supports rotating-frame carrier=0 only".into(),
            ));
        }
        if !s.is_finite() || s < 0.0 || y.iter().any(|v| !v.is_finite()) {
            return Err(CoreError::InvalidInput(
                "collective direct factorize requires finite state and s>=0".into(),
            ));
        }
        work.setup_ops += 12 * n as u64 + 5;
        work.setup_allocations += 1;
        work.setup_bytes += (8 * n) as u64;
        let weights = model.weights().to_vec();
        if !exact_sum_is_one(&weights) {
            return Err(CoreError::InvalidInput(
                "collective direct requires exact represented weight sum 1".into(),
            ));
        }
        // Charge the bounded setup envelope before entering arithmetic that
        // can refuse, so overflow/singularity paths retain consumed work.
        work.setup_ops += 18 * n as u64 + 14;
        work.schur_certificate_ops += 40 * n as u64 + 32;
        work.setup_allocations += 2;
        work.setup_bytes +=
            (std::mem::size_of::<Block>() * n + std::mem::size_of::<u64>() * (5 + 6 * n)) as u64;
        let source_identity = source_state_identity(model, y);
        let mut mean = [0.0; 2];
        for (j, weight) in weights.iter().enumerate() {
            mean[0] += *weight * y[2 * j];
            mean[1] += *weight * y[2 * j + 1];
        }
        let (u, v) = (mean[0], mean[1]);
        let chi = model.chi();
        let c = [
            -2.0 * chi * u * v,
            -chi * (u * u + 3.0 * v * v),
            chi * (3.0 * u * u + v * v),
            2.0 * chi * u * v,
        ];
        let a = 1.0 + s * model.kappa();
        if !a.is_finite() || c.iter().any(|v| !v.is_finite()) {
            return Err(CoreError::NonFinite(
                "collective direct Jacobian overflow".into(),
            ));
        }
        let mut blocks = Vec::with_capacity(n);
        let mut h = [0.0; 4];
        for (j, weight) in weights.iter().enumerate() {
            let b = s * model.width() * model.xi()[j];
            let den = a * a + b * b;
            if !(den.is_finite() && den > 0.0) {
                return Err(CoreError::InvalidInput(
                    "collective direct block inverse singular or overflowed".into(),
                ));
            }
            let p = [a / den, -b / den, b / den, a / den];
            for k in 0..4 {
                h[k] += *weight * p[k];
            }
            blocks.push(Block { p });
        }
        let schur = [
            1.0 - s * (h[0] * c[0] + h[1] * c[2]),
            -s * (h[0] * c[1] + h[1] * c[3]),
            -s * (h[2] * c[0] + h[3] * c[2]),
            1.0 - s * (h[2] * c[1] + h[3] * c[3]),
        ];
        let (si, det_i) = schur_interval(model, y, s, &blocks, &weights)?;
        if det_i.contains_zero() {
            return Err(CoreError::InvalidInput(
                "collective direct Schur determinant interval contains zero".into(),
            ));
        }
        let det = schur[0] * schur[3] - schur[1] * schur[2];
        if !det.is_finite() || det == 0.0 {
            return Err(CoreError::InvalidInput(
                "collective direct Schur matrix singular".into(),
            ));
        }
        let inv = [
            schur[3] / det,
            -schur[1] / det,
            -schur[2] / det,
            schur[0] / det,
        ];
        let (mean_i, _) = certified_mean(model, y)?;
        let r2 = add_up(
            mul_up(mean_i[0].mag(), mean_i[0].mag())?,
            mul_up(mean_i[1].mag(), mean_i[1].mag())?,
        )?;
        let l = mul_up(mul_up(3.0, chi.abs())?, r2)?;
        let ai =
            Interval::point(1.0)?.add(Interval::point(s)?.mul(Interval::point(model.kappa())?)?)?;
        let sl = mul_up(s, l)?;
        if !matches!(sl.partial_cmp(&ai.lo), Some(std::cmp::Ordering::Less)) {
            return Err(CoreError::InvalidInput(
                "collective direct WB2 inverse bound unavailable (q* >= 1)".into(),
            ));
        }
        let inverse_norm_upper = div_up(1.0, sub_down(ai.lo, sl)?)?;
        Ok(Self {
            n,
            s,
            weights,
            blocks,
            c,
            schur_inverse: inv,
            schur_det_interval: si,
            inverse_norm_upper,
            source_identity,
        })
    }

    pub fn schur_determinant_enclosure(&self) -> [f64; 2] {
        self.schur_det_interval.as_pair()
    }
    pub fn dimension(&self) -> usize {
        2 * self.n
    }

    /// Solve into caller-owned output. No heap allocation occurs in this method.
    pub fn solve_into(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        work: &mut CollectiveDirectWork,
    ) -> CoreResult<()> {
        if rhs.len() != 2 * self.n || out.len() != 2 * self.n {
            return Err(CoreError::Dimension(
                "collective direct solve dimension mismatch".into(),
            ));
        }
        if rhs.iter().any(|x| !x.is_finite()) {
            return Err(CoreError::NonFinite(
                "collective direct RHS nonfinite".into(),
            ));
        }
        work.rhs_ops += 16 * self.n as u64 + 18;
        work.reconstruction_ops += 6 * self.n as u64;
        let mut mean = [0.0; 2];
        for j in 0..self.n {
            let p = self.blocks[j].p;
            let x = rhs[2 * j];
            let y = rhs[2 * j + 1];
            let q = [p[0] * x + p[1] * y, p[2] * x + p[3] * y];
            out[2 * j] = q[0];
            out[2 * j + 1] = q[1];
            mean[0] += self.weights[j] * q[0];
            mean[1] += self.weights[j] * q[1];
        }
        let q = [
            self.schur_inverse[0] * mean[0] + self.schur_inverse[1] * mean[1],
            self.schur_inverse[2] * mean[0] + self.schur_inverse[3] * mean[1],
        ];
        let w = [
            self.s * (self.c[0] * q[0] + self.c[1] * q[1]),
            self.s * (self.c[2] * q[0] + self.c[3] * q[1]),
        ];
        for j in 0..self.n {
            let p = self.blocks[j].p;
            out[2 * j] += p[0] * w[0] + p[1] * w[1];
            out[2 * j + 1] += p[2] * w[0] + p[3] * w[1];
        }
        if out.iter().any(|x| !x.is_finite()) {
            return Err(CoreError::NonFinite(
                "collective direct solution nonfinite".into(),
            ));
        }
        Ok(())
    }

    /// Outward enclosure of the weighted residual and WB2 solution error.
    pub fn residual_witness(
        &self,
        model: &CollectiveModel,
        y: &[f64],
        rhs: &[f64],
        x: &[f64],
        work: &mut CollectiveDirectWork,
    ) -> CoreResult<CollectiveResidualWitness> {
        if y.len() != 2 * self.n || rhs.len() != 2 * self.n || x.len() != 2 * self.n {
            return Err(CoreError::Dimension(
                "collective residual witness dimension mismatch".into(),
            ));
        }
        if model.num_modes() != self.n {
            return Err(CoreError::Dimension(
                "collective residual witness model dimension differs from factorization".into(),
            ));
        }
        work.identity_check_ops += 5 + 6 * self.n as u64;
        if !source_state_matches(model, y, &self.source_identity) {
            return Err(CoreError::InvalidInput(
                "collective residual witness model/state identity differs from factorization"
                    .into(),
            ));
        }
        work.reconstruction_ops += 20 * self.n as u64 + 16;
        let (mean_y, _) = certified_mean(model, y)?;
        let (mean_x, _) = certified_mean(model, x)?;
        let (u, v) = (mean_y[0], mean_y[1]);
        let chi = point(model.chi())?;
        let c = [
            chi.mul(u)?.mul(v)?.scale(-2.0)?,
            chi.mul(u.mul(u)?.add(v.mul(v)?.scale(3.0)?)?)?
                .scale(-1.0)?,
            chi.mul(u.mul(u)?.scale(3.0)?.add(v.mul(v)?)?)?,
            chi.mul(u)?.mul(v)?.scale(2.0)?,
        ];
        let mut norm2 = 0.0;
        for j in 0..self.n {
            let w = point(model.width())?.mul(point(model.xi()[j])?)?;
            let kappa = point(model.kappa())?;
            // J block action plus collective C times weighted mean, all interval arithmetic.
            let xx = point(x[2 * j])?;
            let yy = point(x[2 * j + 1])?;
            let j0 = xx
                .mul(kappa)?
                .scale(-1.0)?
                .sub(yy.mul(w)?)?
                .add(c[0].mul(mean_x[0])?)?
                .add(c[1].mul(mean_x[1])?)?;
            let j1 = yy
                .mul(kappa)?
                .scale(-1.0)?
                .add(xx.mul(w)?)?
                .add(c[2].mul(mean_x[0])?)?
                .add(c[3].mul(mean_x[1])?)?;
            let r0 = point(rhs[2 * j])?.sub(xx.sub(j0.scale(self.s)?)?)?;
            let r1 = point(rhs[2 * j + 1])?.sub(yy.sub(j1.scale(self.s)?)?)?;
            let term = mul_up(
                self.weights[j],
                add_up(mul_up(r0.mag(), r0.mag())?, mul_up(r1.mag(), r1.mag())?)?,
            )?;
            norm2 = add_up(norm2, term)?;
        }
        let residual = sqrt_up(norm2)?;
        let err = mul_up(self.inverse_norm_upper, residual)?;
        Ok(CollectiveResidualWitness {
            weighted_residual_upper: residual,
            inverse_norm_upper: self.inverse_norm_upper,
            solution_error_upper: err,
        })
    }
}

fn source_state_identity(model: &CollectiveModel, y: &[f64]) -> Vec<u64> {
    let mut id = Vec::with_capacity(5 + 4 * model.num_modes() + y.len());
    id.push(model.width().to_bits());
    id.push(model.chi().to_bits());
    id.push(model.carrier().to_bits());
    id.push(model.kappa().to_bits());
    id.push(model.num_modes() as u64);
    id.extend(model.xi().iter().map(|x| x.to_bits()));
    id.extend(model.weights().iter().map(|w| w.to_bits()));
    id.extend(
        model
            .initial_amplitudes()
            .iter()
            .flat_map(|z| [z.0.to_bits(), z.1.to_bits()]),
    );
    id.extend(y.iter().map(|x| x.to_bits()));
    id
}

fn source_state_matches(model: &CollectiveModel, y: &[f64], expected: &[u64]) -> bool {
    if model.num_modes().checked_mul(2) != Some(y.len()) {
        return false;
    }
    let mut index = 0;
    let mut check = |bits: u64| {
        let ok = expected.get(index) == Some(&bits);
        index += 1;
        ok
    };
    if !check(model.width().to_bits())
        || !check(model.chi().to_bits())
        || !check(model.carrier().to_bits())
        || !check(model.kappa().to_bits())
        || !check(model.num_modes() as u64)
    {
        return false;
    }
    for x in model.xi() {
        if !check(x.to_bits()) {
            return false;
        }
    }
    for w in model.weights() {
        if !check(w.to_bits()) {
            return false;
        }
    }
    for z in model.initial_amplitudes() {
        if !check(z.0.to_bits()) || !check(z.1.to_bits()) {
            return false;
        }
    }
    for x in y {
        if !check(x.to_bits()) {
            return false;
        }
    }
    index == expected.len()
}

fn schur_interval(
    model: &CollectiveModel,
    y: &[f64],
    s: f64,
    blocks: &[Block],
    weights: &[f64],
) -> CoreResult<(Interval, Interval)> {
    let (m, _) = certified_mean(model, y)?;
    let (u, v) = (m[0], m[1]);
    let ch = point(model.chi())?;
    let c = [
        ch.mul(u)?.mul(v)?.scale(-2.0)?,
        ch.mul(u.mul(u)?.add(v.mul(v)?.scale(3.0)?)?)?.scale(-1.0)?,
        ch.mul(u.mul(u)?.scale(3.0)?.add(v.mul(v)?)?)?,
        ch.mul(u)?.mul(v)?.scale(2.0)?,
    ];
    let mut h = [point(0.0)?, point(0.0)?, point(0.0)?, point(0.0)?];
    let ai = point(1.0)?.add(point(s)?.mul(point(model.kappa())?)?)?;
    for (j, weight) in weights.iter().enumerate() {
        let b = point(s)?
            .mul(point(model.width())?)?
            .mul(point(model.xi()[j])?)?;
        let den = ai.mul(ai)?.add(b.mul(b)?)?;
        let p = [ai.div(den)?, (-b).div(den)?, b.div(den)?, ai.div(den)?];
        for k in 0..4 {
            h[k] = h[k].add(p[k].scale(*weight)?)?;
        }
    }
    let sc = [
        point(1.0)?.sub(point(s)?.mul(h[0].mul(c[0])?.add(h[1].mul(c[2])?)?)?)?,
        point(0.0)?.sub(point(s)?.mul(h[0].mul(c[1])?.add(h[1].mul(c[3])?)?)?)?,
        point(0.0)?.sub(point(s)?.mul(h[2].mul(c[0])?.add(h[3].mul(c[2])?)?)?)?,
        point(1.0)?.sub(point(s)?.mul(h[2].mul(c[1])?.add(h[3].mul(c[3])?)?)?)?,
    ];
    let det = sc[0].mul(sc[3])?.sub(sc[1].mul(sc[2])?)?;
    let _ = blocks;
    Ok((det, det))
}

fn certified_mean(model: &CollectiveModel, y: &[f64]) -> CoreResult<([Interval; 2], f64)> {
    let n = model.num_modes();
    let mut m = [point(0.0)?, point(0.0)?];
    for j in 0..n {
        for k in 0..2 {
            m[k] = m[k].add(point(y[2 * j + k])?.scale(model.weights()[j])?)?;
        }
    }
    Ok((m, 0.0))
}

fn exact_sum_is_one(values: &[f64]) -> bool {
    // Exact fixed-point accumulation in units of 2^-1074. Binary64 positive
    // inputs have at most bit 2097 in these units; 34 limbs also leave room
    // for summing any practically representable mode count. No heap use.
    let mut limbs = [0_u64; 34];
    for &x in values {
        if !(x.is_finite() && x > 0.0) {
            return false;
        }
        let bits = x.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as usize;
        let fraction = bits & ((1_u64 << 52) - 1);
        let significand = if exponent == 0 {
            fraction
        } else {
            (1_u64 << 52) | fraction
        };
        let shift = if exponent == 0 { 0 } else { exponent - 1 };
        let index = shift / 64;
        let offset = shift % 64;
        let shifted = (significand as u128) << offset;
        if index + 1 >= limbs.len() {
            return false;
        }
        let (lo, carry0) = limbs[index].overflowing_add(shifted as u64);
        limbs[index] = lo;
        let (hi0, carry1) = limbs[index + 1].overflowing_add((shifted >> 64) as u64);
        let (hi, carry2) = hi0.overflowing_add(carry0 as u64);
        limbs[index + 1] = hi;
        let mut carry = carry1 || carry2;
        let mut j = index + 2;
        while carry && j < limbs.len() {
            let (v, c) = limbs[j].overflowing_add(1);
            limbs[j] = v;
            carry = c;
            j += 1;
        }
        if carry {
            return false;
        }
    }
    let target = 1074_usize;
    limbs.iter().enumerate().all(|(i, &v)| {
        v == if i == target / 64 {
            1_u64 << (target % 64)
        } else {
            0
        }
    })
}
