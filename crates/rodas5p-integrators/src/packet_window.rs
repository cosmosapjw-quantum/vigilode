//! Finite-window causal packet and native total certificate (LP02).
//!
//! Autonomous N-mode collective cubic model on [0, h]:
//!   \dot{a}_j = i * width * \xi_j * a_j + i * \chi * |\langle a \rangle|^2 * \langle a \rangle
//! with normalized positive weights w_j > 0, sum w_j = 1, frequencies \xi_j \in [-1, 1],
//! kappa = 0, and independent complex initial amplitudes a_j(0) = z_j.
//!
//! Provides moment prefix compilation, polynomial waveform Picard iteration,
//! exact causal factorial convolution via precomputed beta weights,
//! and original-model analytic error estimates for the mean and all modes.
//! Floating-point coefficient and reconstruction errors are not yet enclosed;
//! these estimates have diagnostic authority, not production certification.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

use rodas5p_core::directed::{Interval, add_up, sqrt_up};
use rodas5p_core::{CoreError, CoreResult};

use crate::collective_model::CollectiveModel;
use crate::packet_work::PacketWorkCounters;

#[path = "packet_curve.rs"]
mod packet_curve;
pub use packet_curve::{
    ContinuousPacketResidualBound, PacketCertifiedView, PacketComplexInterval, PacketCurveOutput,
    PacketCurveOutputView, PacketCurveOutputWorkspace, PacketFrozenOutput, RepresentedPacketCurve,
};

/// Errors arising during causal packet compilation, candidate iteration, or certification.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum PacketError {
    #[error(
        "the polynomial packet implements kappa = 0; damping {0} requires the damped packet route"
    )]
    UnsupportedDamping(f64),
    #[error("non-finite value in packet {0}")]
    NonFinite(&'static str),
    #[error("horizon must be strictly positive, got {0}")]
    InvalidHorizon(f64),
    #[error("degree {degree} must be >= kernel order {order}")]
    DegreeLessThanOrder { degree: usize, order: usize },
    #[error("sweeps must be strictly positive, got 0")]
    ZeroSweeps,
    #[error("Picard candidate leaves proved complex-mean unit domain: bound {bound} > 1.0")]
    DomainExceeded { bound: f64 },
    #[error("Gronwall contract violated: h * L = {h_times_l} >= 1.0 (unsupported)")]
    GronwallConditionViolated { h_times_l: f64 },
    #[error("reconstruction index {index} out of bounds for mode count {modes}")]
    IndexOutOfBounds { index: usize, modes: usize },
    #[error("error budget exceeded: max mode bound {max_bound} > tolerance {tolerance}")]
    BudgetExceeded { max_bound: f64, tolerance: f64 },
    #[error("compiled packet descriptor was modified after generation")]
    ModifiedDescriptor,
    #[error("candidate was generated for a different compiled packet")]
    CandidatePacketMismatch,
    #[error("packet size arithmetic overflow while allocating {0}")]
    SizeOverflow(&'static str),
    #[error("outward packet certificate refused: {0}")]
    OutwardRefusal(&'static str),
    #[error("Bernstein refinement budget exhausted with upper {upper} above threshold {threshold}")]
    BernsteinBudgetExhausted { upper: f64, threshold: f64 },
}

/// Opt-in physical-time requests for the local outward packet path.
///
/// `initial_radii[j]` bounds an independent complex-mode uncertainty in the
/// modulus norm.  This request is deliberately separate from the diagnostic
/// normalized-time evaluator.
#[derive(Clone, Copy, Debug)]
pub struct OutwardPacketRequest<'a> {
    pub times: &'a [f64],
    pub initial_radii: &'a [f64],
}

/// Returned centers and outward radii for an opt-in packet request.
/// Modes are flattened time-major: `modes[t * mode_count + j]`.
#[derive(Clone, Debug, PartialEq)]
pub struct OutwardPacketOutput {
    pub times: Vec<f64>,
    pub modes: Vec<(f64, f64)>,
    pub mode_radii: Vec<f64>,
    pub means: Vec<(f64, f64)>,
    pub mean_radii: Vec<f64>,
    /// This remains false until an independent review admits this path.
    pub roundoff_enclosed: bool,
}

#[derive(Clone, Copy, Debug)]
struct IComplex {
    re: Interval,
    im: Interval,
}

type OutwardReplay = (Vec<IComplex>, Vec<IComplex>, Vec<IComplex>);

fn checked_len<T>(len: usize, label: &'static str) -> Result<(), PacketError> {
    if len > isize::MAX as usize / std::mem::size_of::<T>().max(1) {
        return Err(PacketError::SizeOverflow(label));
    }
    Ok(())
}

fn checked_capacity<T>(len: usize, label: &'static str) -> Result<Vec<T>, PacketError> {
    checked_len::<T>(len, label)?;
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| PacketError::SizeOverflow(label))?;
    Ok(out)
}

#[cfg(test)]
static TEST_REFUSE_AFTER_OUTPUT_PRECHARGE: AtomicBool = AtomicBool::new(false);

fn precharge_outward_output(work: &mut PacketWorkCounters, coefficient_len: usize, order: usize) {
    work.output_copy_items += (2 * coefficient_len) as u64;
    work.output_restriction_multiplies += (3 * coefficient_len) as u64;
    work.reconstruction_terms += (order + 1) as u64;
}

impl IComplex {
    fn point(x: (f64, f64)) -> Result<Self, PacketError> {
        Ok(Self {
            re: Interval::point(x.0)
                .map_err(|_| PacketError::OutwardRefusal("non-finite complex input"))?,
            im: Interval::point(x.1)
                .map_err(|_| PacketError::OutwardRefusal("non-finite complex input"))?,
        })
    }
    fn mul(self, b: Self) -> Result<Self, PacketError> {
        let rr = self
            .re
            .mul(b.re)
            .map_err(|_| PacketError::OutwardRefusal("interval complex multiplication"))?;
        let ii = self
            .im
            .mul(b.im)
            .map_err(|_| PacketError::OutwardRefusal("interval complex multiplication"))?;
        let ri = self
            .re
            .mul(b.im)
            .map_err(|_| PacketError::OutwardRefusal("interval complex multiplication"))?;
        let ir = self
            .im
            .mul(b.re)
            .map_err(|_| PacketError::OutwardRefusal("interval complex multiplication"))?;
        Ok(Self {
            re: rr
                .sub(ii)
                .map_err(|_| PacketError::OutwardRefusal("interval complex subtraction"))?,
            im: ri
                .add(ir)
                .map_err(|_| PacketError::OutwardRefusal("interval complex addition"))?,
        })
    }
    fn scale(self, x: Interval) -> Result<Self, PacketError> {
        Ok(Self {
            re: self
                .re
                .mul(x)
                .map_err(|_| PacketError::OutwardRefusal("interval complex scale"))?,
            im: self
                .im
                .mul(x)
                .map_err(|_| PacketError::OutwardRefusal("interval complex scale"))?,
        })
    }
}

fn ic_poly_force(
    p: &[IComplex],
    chi: f64,
    work: &mut PacketWorkCounters,
) -> Result<Vec<IComplex>, PacketError> {
    let n = p.len();
    let zero = IComplex::point((0.0, 0.0))?;
    let mut p2 = vec![zero; n.saturating_mul(2).saturating_sub(1)];
    for (j, &a) in p.iter().enumerate() {
        for (k, &b) in p.iter().enumerate() {
            work.certificate_items += 1;
            let bc = IComplex {
                re: b.re,
                im: -b.im,
            };
            let product = a.mul(bc)?;
            p2[j + k] = IComplex {
                re: p2[j + k]
                    .re
                    .add(product.re)
                    .map_err(|_| PacketError::OutwardRefusal("force accumulation"))?,
                im: p2[j + k]
                    .im
                    .add(product.im)
                    .map_err(|_| PacketError::OutwardRefusal("force accumulation"))?,
            };
        }
    }
    let mut out = vec![zero; n.saturating_mul(3).saturating_sub(2)];
    for (j, &a) in p2.iter().enumerate() {
        for (k, &b) in p.iter().enumerate() {
            work.certificate_items += 1;
            let q = a.mul(b)?;
            out[j + k] = IComplex {
                re: out[j + k]
                    .re
                    .add(q.re)
                    .map_err(|_| PacketError::OutwardRefusal("force accumulation"))?,
                im: out[j + k]
                    .im
                    .add(q.im)
                    .map_err(|_| PacketError::OutwardRefusal("force accumulation"))?,
            };
        }
    }
    let c = Interval::point(chi).map_err(|_| PacketError::OutwardRefusal("chi"))?;
    for x in &mut out {
        let re = -x
            .im
            .mul(c)
            .map_err(|_| PacketError::OutwardRefusal("force scale"))?;
        let im =
            x.re.mul(c)
                .map_err(|_| PacketError::OutwardRefusal("force scale"))?;
        *x = IComplex { re, im };
    }
    Ok(out)
}

fn ic_beta(j: usize, k: usize) -> Result<Interval, PacketError> {
    if j + 1 > (1_u64 << 53) as usize || k + j + 1 > (1_u64 << 53) as usize {
        return Err(PacketError::OutwardRefusal(
            "beta integer exceeds exact binary64 range",
        ));
    }
    let mut v = Interval::point(1.0)
        .map_err(|_| PacketError::OutwardRefusal("beta setup"))?
        .div(
            Interval::point((j + 1) as f64)
                .map_err(|_| PacketError::OutwardRefusal("beta denominator"))?,
        )
        .map_err(|_| PacketError::OutwardRefusal("beta divide"))?;
    for m in 1..=k {
        let numerator =
            Interval::point(m as f64).map_err(|_| PacketError::OutwardRefusal("beta numerator"))?;
        let denominator = Interval::point((j + m + 1) as f64)
            .map_err(|_| PacketError::OutwardRefusal("beta denominator"))?;
        v = v
            .mul(numerator)
            .map_err(|_| PacketError::OutwardRefusal("beta multiply"))?
            .div(denominator)
            .map_err(|_| PacketError::OutwardRefusal("beta divide"))?;
    }
    Ok(v)
}

fn inverse_factorial_interval(n: usize) -> Result<Interval, PacketError> {
    if n > (1_u64 << 53) as usize {
        return Err(PacketError::OutwardRefusal(
            "factorial integer exceeds exact binary64 range",
        ));
    }
    let mut out =
        Interval::point(1.0).map_err(|_| PacketError::OutwardRefusal("factorial setup"))?;
    for i in 2..=n {
        out = out
            .div(
                Interval::point(i as f64)
                    .map_err(|_| PacketError::OutwardRefusal("factorial integer"))?,
            )
            .map_err(|_| PacketError::OutwardRefusal("factorial divide"))?;
    }
    Ok(out)
}

fn ic_convolve(
    kernel: &[IComplex],
    force: &[IComplex],
    work: &mut PacketWorkCounters,
) -> Result<Vec<IComplex>, PacketError> {
    let zero = IComplex::point((0.0, 0.0))?;
    let mut out = vec![zero; kernel.len() + force.len()];
    for (j, &a) in kernel.iter().enumerate() {
        for (k, &b) in force.iter().enumerate() {
            work.certificate_items += 1;
            let q = a.mul(b)?.scale(ic_beta(j, k)?)?;
            out[j + k + 1] = IComplex {
                re: out[j + k + 1]
                    .re
                    .add(q.re)
                    .map_err(|_| PacketError::OutwardRefusal("convolution accumulation"))?,
                im: out[j + k + 1]
                    .im
                    .add(q.im)
                    .map_err(|_| PacketError::OutwardRefusal("convolution accumulation"))?,
            };
        }
    }
    Ok(out)
}

fn outward_picard_replay(
    packet: &CompiledPacket,
    degree: usize,
    sweeps: usize,
    work: &mut PacketWorkCounters,
) -> Result<OutwardReplay, PacketError> {
    let porder = packet.order;
    let zero = IComplex::point((0.0, 0.0))?;
    let wh = Interval::point(packet.model.width())
        .map_err(|_| PacketError::OutwardRefusal("width"))?
        .mul(Interval::point(packet.horizon).map_err(|_| PacketError::OutwardRefusal("horizon"))?)
        .map_err(|_| PacketError::OutwardRefusal("width horizon"))?;
    let mut basis = IComplex::point((1.0, 0.0))?;
    let mut free = Vec::with_capacity(porder + 1);
    let mut kernel = Vec::with_capacity(porder + 1);
    let mut powers = checked_capacity::<Interval>(packet.model.num_modes(), "moment powers")?;
    powers.resize(
        packet.model.num_modes(),
        Interval::point(1.0).map_err(|_| PacketError::OutwardRefusal("moment"))?,
    );
    for k in 0..=porder {
        let mut mu = Interval::point(0.0).map_err(|_| PacketError::OutwardRefusal("moment"))?;
        let mut nu = IComplex::point((0.0, 0.0))?;
        for (j, power) in powers.iter_mut().enumerate() {
            let w = Interval::point(packet.model.weights()[j])
                .map_err(|_| PacketError::OutwardRefusal("weight"))?;
            let xp = *power;
            mu = mu
                .add(
                    w.mul(xp)
                        .map_err(|_| PacketError::OutwardRefusal("moment multiplication"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("moment sum"))?;
            let term = IComplex::point(packet.model.initial_amplitudes()[j])?.scale(
                w.mul(xp)
                    .map_err(|_| PacketError::OutwardRefusal("moment multiplication"))?,
            )?;
            nu = IComplex {
                re: nu
                    .re
                    .add(term.re)
                    .map_err(|_| PacketError::OutwardRefusal("moment sum"))?,
                im: nu
                    .im
                    .add(term.im)
                    .map_err(|_| PacketError::OutwardRefusal("moment sum"))?,
            };
            work.certificate_items += 1;
            if k < porder {
                work.outward_spectral_power_terms += 1;
                *power = xp
                    .mul(
                        Interval::point(packet.model.xi()[j])
                            .map_err(|_| PacketError::OutwardRefusal("xi"))?,
                    )
                    .map_err(|_| PacketError::OutwardRefusal("moment multiplication"))?;
            }
        }
        free.push(basis.mul(nu)?);
        kernel.push(basis.scale(mu)?);
        if k < porder {
            let f = wh
                .div(
                    Interval::point((k + 1) as f64)
                        .map_err(|_| PacketError::OutwardRefusal("factorial"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("basis"))?;
            basis = IComplex {
                re: -basis
                    .im
                    .mul(f)
                    .map_err(|_| PacketError::OutwardRefusal("basis"))?,
                im: basis
                    .re
                    .mul(f)
                    .map_err(|_| PacketError::OutwardRefusal("basis"))?,
            };
        }
    }
    let mut p = free.clone();
    p.resize(degree + 1, zero);
    for _ in 0..sweeps {
        let f = ic_poly_force(&p, packet.model.chi(), work)?;
        let c = ic_convolve(&kernel, &f, work)?;
        let mut next = free.clone();
        next.resize(degree + 1, zero);
        for (i, x) in c.into_iter().enumerate().take(degree + 1) {
            let y = x.scale(
                Interval::point(packet.horizon)
                    .map_err(|_| PacketError::OutwardRefusal("horizon"))?,
            )?;
            next[i] = IComplex {
                re: next[i]
                    .re
                    .add(y.re)
                    .map_err(|_| PacketError::OutwardRefusal("picard sum"))?,
                im: next[i]
                    .im
                    .add(y.im)
                    .map_err(|_| PacketError::OutwardRefusal("picard sum"))?,
            };
        }
        p = next;
    }
    Ok((p, free, kernel))
}

fn outward_residual_l1(
    p: &[IComplex],
    free: &[IComplex],
    kernel: &[IComplex],
    h: f64,
    chi: f64,
    work: &mut PacketWorkCounters,
) -> Result<f64, PacketError> {
    let force = ic_poly_force(p, chi, work)?;
    let convolution = ic_convolve(kernel, &force, work)?;
    let scale = Interval::point(h).map_err(|_| PacketError::OutwardRefusal("horizon"))?;
    let mut rho =
        Interval::point(0.0).map_err(|_| PacketError::OutwardRefusal("residual setup"))?;
    for i in 0..p.len().max(free.len()).max(convolution.len()) {
        let pi = p.get(i).copied().unwrap_or(IComplex::point((0.0, 0.0))?);
        let fi = free.get(i).copied().unwrap_or(IComplex::point((0.0, 0.0))?);
        let ci = convolution
            .get(i)
            .copied()
            .unwrap_or(IComplex::point((0.0, 0.0))?)
            .scale(scale)?;
        let re = pi
            .re
            .sub(fi.re)
            .map_err(|_| PacketError::OutwardRefusal("residual subtraction"))?
            .sub(ci.re)
            .map_err(|_| PacketError::OutwardRefusal("residual subtraction"))?;
        let im = pi
            .im
            .sub(fi.im)
            .map_err(|_| PacketError::OutwardRefusal("residual subtraction"))?
            .sub(ci.im)
            .map_err(|_| PacketError::OutwardRefusal("residual subtraction"))?;
        let term = Interval::new(0.0, outward_add(re.mag(), im.mag(), "residual magnitude")?)
            .map_err(|_| PacketError::OutwardRefusal("residual magnitude"))?;
        rho = rho
            .add(term)
            .map_err(|_| PacketError::OutwardRefusal("residual sum"))?;
        work.certificate_items += 1;
    }
    Ok(rho.hi)
}

fn ic_restrict_physical(
    p: &[IComplex],
    tau: Interval,
    work: &mut PacketWorkCounters,
) -> Result<Vec<IComplex>, PacketError> {
    let mut power = Interval::point(1.0).map_err(|_| PacketError::OutwardRefusal("tau"))?;
    let mut out = Vec::with_capacity(p.len());
    for &coefficient in p {
        out.push(coefficient.scale(power)?);
        power = power
            .mul(tau)
            .map_err(|_| PacketError::OutwardRefusal("tau power"))?;
        work.certificate_items += 1;
    }
    Ok(out)
}

fn ic_gamma_physical(
    force: &[IComplex],
    omega_time: Interval,
    time: Interval,
    order: usize,
    work: &mut PacketWorkCounters,
) -> Result<IComplex, PacketError> {
    let zero = IComplex::point((0.0, 0.0))?;
    let mut basis = IComplex::point((1.0, 0.0))?;
    let mut total = zero;
    for l in 0..=order {
        let mut acc = zero;
        for (k, &fk) in force.iter().enumerate() {
            let q = fk.scale(ic_beta(l, k)?)?;
            acc = IComplex {
                re: acc
                    .re
                    .add(q.re)
                    .map_err(|_| PacketError::OutwardRefusal("gamma sum"))?,
                im: acc
                    .im
                    .add(q.im)
                    .map_err(|_| PacketError::OutwardRefusal("gamma sum"))?,
            };
            work.certificate_items += 1;
        }
        let q = basis.mul(acc)?;
        total = IComplex {
            re: total
                .re
                .add(q.re)
                .map_err(|_| PacketError::OutwardRefusal("gamma sum"))?,
            im: total
                .im
                .add(q.im)
                .map_err(|_| PacketError::OutwardRefusal("gamma sum"))?,
        };
        if l < order {
            let divisor = Interval::point((l + 1) as f64)
                .map_err(|_| PacketError::OutwardRefusal("gamma factorial"))?;
            let f = omega_time
                .div(divisor)
                .map_err(|_| PacketError::OutwardRefusal("gamma basis"))?;
            basis = IComplex {
                re: -basis
                    .im
                    .mul(f)
                    .map_err(|_| PacketError::OutwardRefusal("gamma basis"))?,
                im: basis
                    .re
                    .mul(f)
                    .map_err(|_| PacketError::OutwardRefusal("gamma basis"))?,
            };
        }
    }
    total.scale(time)
}

fn outward_scalar_bounds(
    packet: &CompiledPacket,
    p: &[IComplex],
    eta: &[f64],
    rho: f64,
    work: &mut PacketWorkCounters,
) -> Result<(f64, f64, f64, f64), PacketError> {
    let mut s = Interval::point(0.0).map_err(|_| PacketError::OutwardRefusal("S setup"))?;
    let mut e = Interval::point(0.0).map_err(|_| PacketError::OutwardRefusal("E setup"))?;
    let mut a0 = Interval::point(0.0).map_err(|_| PacketError::OutwardRefusal("A0 setup"))?;
    let mut support = 0.0_f64;
    for (j, &eta_j) in eta.iter().enumerate() {
        let w = Interval::point(packet.model.weights()[j])
            .map_err(|_| PacketError::OutwardRefusal("weight"))?;
        s = s.add(w).map_err(|_| PacketError::OutwardRefusal("S sum"))?;
        let z = outward_add(
            outward_complex_upper(packet.model.initial_amplitudes()[j], "initial magnitude")?,
            eta_j,
            "initial uncertainty sum",
        )?;
        if !z.is_finite() {
            return Err(PacketError::OutwardRefusal("initial uncertainty overflow"));
        }
        let zi = Interval::point(z).map_err(|_| PacketError::OutwardRefusal("initial bound"))?;
        e = e
            .add(
                w.mul(
                    zi.mul(zi)
                        .map_err(|_| PacketError::OutwardRefusal("E product"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("E product"))?,
            )
            .map_err(|_| PacketError::OutwardRefusal("E sum"))?;
        a0 = a0
            .add(
                w.mul(zi)
                    .map_err(|_| PacketError::OutwardRefusal("A0 product"))?,
            )
            .map_err(|_| PacketError::OutwardRefusal("A0 sum"))?;
        support = support.max(packet.model.xi()[j].abs());
        work.certificate_items += 1;
    }
    let mut rp_i = Interval::point(0.0).map_err(|_| PacketError::OutwardRefusal("P norm setup"))?;
    for x in p {
        rp_i = rp_i
            .add(
                Interval::point(outward_add(x.re.mag(), x.im.mag(), "P norm")?)
                    .map_err(|_| PacketError::OutwardRefusal("P norm"))?,
            )
            .map_err(|_| PacketError::OutwardRefusal("P norm sum"))?;
    }
    let rp = rp_i.hi;
    let re = sqrt_up(
        s.mul(e)
            .map_err(|_| PacketError::OutwardRefusal("energy product"))?
            .hi,
    )
    .map_err(|_| PacketError::OutwardRefusal("energy square root"))?;
    let r = rp.max(re);
    let fail = |_| PacketError::OutwardRefusal("scalar interval arithmetic");
    let q = Interval::point(3.0)
        .map_err(fail)?
        .mul(Interval::point(packet.model.chi().abs()).map_err(fail)?)
        .map_err(fail)?
        .mul(s)
        .map_err(fail)?
        .mul(Interval::point(r).map_err(fail)?)
        .map_err(fail)?
        .mul(Interval::point(r).map_err(fail)?)
        .map_err(fail)?
        .mul(Interval::point(packet.horizon).map_err(fail)?)
        .map_err(fail)?
        .hi;
    if !q.is_finite() || q >= 1.0 {
        return Err(PacketError::OutwardRefusal("q >= 1"));
    }
    let x = Interval::point(packet.model.width().abs())
        .map_err(fail)?
        .mul(Interval::point(support).map_err(fail)?)
        .map_err(fail)?
        .mul(Interval::point(packet.horizon).map_err(fail)?)
        .map_err(fail)?
        .hi;
    if !x.is_finite() || x >= (packet.order + 2) as f64 {
        return Err(PacketError::OutwardRefusal("geometric tail condition"));
    }
    let mut bpow = Interval::point(1.0).map_err(fail)?;
    let xi = Interval::point(x).map_err(fail)?;
    for _ in 0..=packet.order {
        bpow = bpow.mul(xi).map_err(fail)?;
    }
    let denom = Interval::point(1.0)
        .map_err(fail)?
        .sub(
            xi.div(Interval::point((packet.order + 2) as f64).map_err(fail)?)
                .map_err(fail)?,
        )
        .map_err(fail)?;
    let b = bpow
        .mul(inverse_factorial_interval(packet.order + 1)?)
        .map_err(fail)?
        .div(denom)
        .map_err(fail)?
        .hi;
    let mut eta_mean = Interval::point(0.0).map_err(fail)?;
    for (&w, &v) in packet.model.weights().iter().zip(eta) {
        eta_mean = eta_mean
            .add(
                Interval::point(w)
                    .map_err(fail)?
                    .mul(Interval::point(v).map_err(fail)?)
                    .map_err(fail)?,
            )
            .map_err(fail)?;
    }
    let nonlinear = Interval::point(packet.horizon)
        .map_err(fail)?
        .mul(s)
        .map_err(fail)?
        .mul(Interval::point(packet.model.chi().abs()).map_err(fail)?)
        .map_err(fail)?
        .mul(Interval::point(rp).map_err(fail)?)
        .map_err(fail)?
        .mul(Interval::point(rp).map_err(fail)?)
        .map_err(fail)?
        .mul(Interval::point(rp).map_err(fail)?)
        .map_err(fail)?
        .mul(Interval::point(b).map_err(fail)?)
        .map_err(fail)?;
    let numerator = Interval::point(rho)
        .map_err(fail)?
        .add(a0.mul(Interval::point(b).map_err(fail)?).map_err(fail)?)
        .map_err(fail)?
        .add(nonlinear)
        .map_err(fail)?
        .add(eta_mean)
        .map_err(fail)?;
    let emean = numerator
        .div(
            Interval::point(1.0)
                .map_err(fail)?
                .sub(Interval::point(q).map_err(fail)?)
                .map_err(fail)?,
        )
        .map_err(|_| PacketError::OutwardRefusal("mean denominator"))?
        .hi;
    if !emean.is_finite() {
        return Err(PacketError::OutwardRefusal("mean radius overflow"));
    }
    Ok((emean, b, r, rp))
}

impl From<PacketError> for CoreError {
    fn from(err: PacketError) -> Self {
        CoreError::InvalidInput(err.to_string())
    }
}

// ---------------------------------------------------------------------------
// Complex arithmetic helpers on (f64, f64)
// ---------------------------------------------------------------------------

#[inline(always)]
fn c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

#[inline(always)]
fn c_sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 - b.0, a.1 - b.1)
}

#[inline(always)]
fn c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[inline(always)]
fn c_scale(a: (f64, f64), s: f64) -> (f64, f64) {
    (a.0 * s, a.1 * s)
}

#[inline(always)]
fn c_conj(a: (f64, f64)) -> (f64, f64) {
    (a.0, -a.1)
}

#[inline(always)]
fn c_upper(a: (f64, f64)) -> f64 {
    a.0.abs() + a.1.abs()
}

fn outward_add(a: f64, b: f64, label: &'static str) -> Result<f64, PacketError> {
    add_up(a, b).map_err(|_| PacketError::OutwardRefusal(label))
}

fn outward_complex_upper(a: (f64, f64), label: &'static str) -> Result<f64, PacketError> {
    outward_add(a.0.abs(), a.1.abs(), label)
}

// ---------------------------------------------------------------------------
// Polynomial helpers in normalized time \tau = t / h \in [0, 1]
// ---------------------------------------------------------------------------

fn poly_add(a: &[(f64, f64)], b: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let len = a.len().max(b.len());
    let mut out = vec![(0.0, 0.0); len];
    for (i, &c) in a.iter().enumerate() {
        out[i] = c_add(out[i], c);
    }
    for (i, &c) in b.iter().enumerate() {
        out[i] = c_add(out[i], c);
    }
    out
}

fn poly_sub(a: &[(f64, f64)], b: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let len = a.len().max(b.len());
    let mut out = vec![(0.0, 0.0); len];
    for (i, &c) in a.iter().enumerate() {
        out[i] = c_add(out[i], c);
    }
    for (i, &c) in b.iter().enumerate() {
        out[i] = c_sub(out[i], c);
    }
    out
}

fn poly_mul(a: &[(f64, f64)], b: &[(f64, f64)], work: &mut PacketWorkCounters) -> Vec<(f64, f64)> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![(0.0, 0.0); a.len() + b.len() - 1];
    for (j, &xj) in a.iter().enumerate() {
        for (k, &yk) in b.iter().enumerate() {
            work.poly_multiply_terms += 1;
            out[j + k] = c_add(out[j + k], c_mul(xj, yk));
        }
    }
    out
}

fn poly_bound(p: &[(f64, f64)]) -> f64 {
    p.iter().map(|&c| c_upper(c)).sum()
}

/// Outward upper bound for a represented complex polynomial on `[0, 1]`.
/// The cheap coefficient sum is returned without a transform when it already
/// meets `threshold`. Otherwise at most `max_depth` dyadic de Casteljau levels
/// are tried. Each level retains the minimum of two valid outward bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BernsteinSupBound {
    pub upper: f64,
    pub coefficient_upper: f64,
    pub transform_used: bool,
    pub refinement_levels: usize,
}

fn bernstein_norm_upper(c: (Interval, Interval)) -> Result<f64, PacketError> {
    outward_add(c.0.mag(), c.1.mag(), "Bernstein control norm")
}

fn bernstein_polygon_upper(
    points: &[(Interval, Interval)],
    work: &mut PacketWorkCounters,
) -> Result<f64, PacketError> {
    let mut upper = 0.0_f64;
    for &point in points {
        work.certificate_items += 1;
        upper = upper.max(bernstein_norm_upper(point)?);
    }
    Ok(upper)
}

type BernsteinPolygon = Vec<(Interval, Interval)>;

fn bernstein_split_half(
    points: &[(Interval, Interval)],
    work: &mut PacketWorkCounters,
) -> Result<(BernsteinPolygon, BernsteinPolygon), PacketError> {
    let n = points.len();
    let mut triangle = points.to_vec();
    let mut left = Vec::with_capacity(n);
    let mut right = vec![points[0]; n];
    left.push(triangle[0]);
    right[n - 1] = triangle[n - 1];
    let half = Interval::point(0.5).map_err(|_| PacketError::OutwardRefusal("Bernstein half"))?;
    for level in 1..n {
        for j in 0..n - level {
            work.bernstein_refinement_terms += 1;
            triangle[j] = (
                triangle[j]
                    .0
                    .add(triangle[j + 1].0)
                    .and_then(|v| v.mul(half))
                    .map_err(|_| PacketError::OutwardRefusal("Bernstein subdivision"))?,
                triangle[j]
                    .1
                    .add(triangle[j + 1].1)
                    .and_then(|v| v.mul(half))
                    .map_err(|_| PacketError::OutwardRefusal("Bernstein subdivision"))?,
            );
        }
        left.push(triangle[0]);
        right[n - level - 1] = triangle[n - level - 1];
    }
    Ok((left, right))
}

/// Bounds the exact polynomial whose coefficients lie in the supplied
/// intervals. A budget of zero permits the basis transform but no subdivision.
/// The depth cap limits worst-case work to a finite, explicitly reported scope.
pub fn outward_polynomial_sup_bound(
    coefficients: &[(Interval, Interval)],
    threshold: f64,
    max_depth: usize,
    work: &mut PacketWorkCounters,
) -> Result<BernsteinSupBound, PacketError> {
    if coefficients.is_empty() || !threshold.is_finite() || threshold < 0.0 || max_depth > 8 {
        return Err(PacketError::OutwardRefusal(
            "invalid Bernstein input or depth budget",
        ));
    }
    if coefficients.iter().any(|(re, im)| {
        !re.lo.is_finite()
            || !re.hi.is_finite()
            || re.lo > re.hi
            || !im.lo.is_finite()
            || !im.hi.is_finite()
            || im.lo > im.hi
    }) {
        return Err(PacketError::OutwardRefusal(
            "invalid Bernstein coefficient interval",
        ));
    }
    let mut cheap = 0.0;
    for &coefficient in coefficients {
        work.certificate_items += 1;
        cheap = outward_add(
            cheap,
            bernstein_norm_upper(coefficient)?,
            "Bernstein coefficient sum",
        )?;
    }
    if cheap <= threshold {
        return Ok(BernsteinSupBound {
            upper: cheap,
            coefficient_upper: cheap,
            transform_used: false,
            refinement_levels: 0,
        });
    }
    let degree = coefficients.len() - 1;
    let zero = Interval::point(0.0).map_err(|_| PacketError::OutwardRefusal("Bernstein zero"))?;
    let mut points = Vec::with_capacity(coefficients.len());
    for i in 0..=degree {
        let mut value = (zero, zero);
        let mut ratio =
            Interval::point(1.0).map_err(|_| PacketError::OutwardRefusal("Bernstein ratio"))?;
        for (j, &coefficient) in coefficients.iter().enumerate().take(i + 1) {
            work.bernstein_transform_terms += 1;
            if j > 0 {
                ratio = ratio
                    .mul(
                        Interval::point((i - j + 1) as f64)
                            .map_err(|_| PacketError::OutwardRefusal("Bernstein ratio"))?,
                    )
                    .and_then(|v| v.div(Interval::point((degree - j + 1) as f64)?))
                    .map_err(|_| PacketError::OutwardRefusal("Bernstein ratio"))?;
            }
            value.0 = value
                .0
                .add(
                    coefficient
                        .0
                        .mul(ratio)
                        .map_err(|_| PacketError::OutwardRefusal("Bernstein transform"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("Bernstein transform"))?;
            value.1 = value
                .1
                .add(
                    coefficient
                        .1
                        .mul(ratio)
                        .map_err(|_| PacketError::OutwardRefusal("Bernstein transform"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("Bernstein transform"))?;
        }
        points.push(value);
    }
    let mut upper = cheap.min(bernstein_polygon_upper(&points, work)?);
    if upper <= threshold {
        return Ok(BernsteinSupBound {
            upper,
            coefficient_upper: cheap,
            transform_used: true,
            refinement_levels: 0,
        });
    }
    for depth in 1..=max_depth {
        let cells = 1usize << depth;
        let mut level_upper = 0.0_f64;
        for index in 0..cells {
            let mut cell = points.clone();
            for bit in (0..depth).rev() {
                let (left, right) = bernstein_split_half(&cell, work)?;
                cell = if index & (1 << bit) == 0 { left } else { right };
            }
            level_upper = level_upper.max(bernstein_polygon_upper(&cell, work)?);
        }
        upper = upper.min(level_upper);
        if upper <= threshold {
            return Ok(BernsteinSupBound {
                upper,
                coefficient_upper: cheap,
                transform_used: true,
                refinement_levels: depth,
            });
        }
    }
    Err(PacketError::BernsteinBudgetExhausted { upper, threshold })
}

fn poly_eval(p: &[(f64, f64)], x: f64, work: &mut PacketWorkCounters) -> (f64, f64) {
    let mut val = (0.0, 0.0);
    for &c in p.iter().rev() {
        work.output_horner_terms += 1;
        val = c_add(c_scale(val, x), c);
    }
    val
}

/// Evaluates F(p) = i * chi * |p|^2 * p in polynomial algebra.
fn force(p: &[(f64, f64)], chi: f64, work: &mut PacketWorkCounters) -> Vec<(f64, f64)> {
    work.nonlinear_evaluations += 1;
    let p_conj: Vec<(f64, f64)> = p.iter().map(|&c| c_conj(c)).collect();
    let p2 = poly_mul(p, &p_conj, work);
    let p3 = poly_mul(&p2, p, work);

    // Multiply by i * chi: (u + i v) * (i * chi) = -chi * v + i * chi * u
    p3.into_iter().map(|(u, v)| (-chi * v, chi * u)).collect()
}

// ---------------------------------------------------------------------------
// Precomputed Causal Beta Weights (LC1/LC2 contracts)
// ---------------------------------------------------------------------------

/// Exact causal beta weights table B(j+1, k+1) = \frac{j! k!}{(j+k+1)!}.
#[derive(Clone, Debug, PartialEq)]
pub struct CausalWeights {
    kernel_degree: usize,
    forcing_degree: usize,
    rows: Vec<Vec<f64>>,
}

impl CausalWeights {
    /// Compiles a table of beta weights for orders up to `kernel_degree` and `forcing_degree`.
    pub fn compile(kernel_degree: usize, forcing_degree: usize) -> Self {
        let mut rows = Vec::with_capacity(kernel_degree + 1);
        for j in 0..=kernel_degree {
            let mut row = Vec::with_capacity(forcing_degree + 1);
            let first = 1.0 / ((j + 1) as f64);
            row.push(first);
            let mut current = first;
            for k in 1..=forcing_degree {
                current *= (k as f64) / ((j + k + 1) as f64);
                row.push(current);
            }
            rows.push(row);
        }
        Self {
            kernel_degree,
            forcing_degree,
            rows,
        }
    }

    /// Returns the beta weight B(j+1, k+1) = \frac{j! k!}{(j+k+1)!}.
    #[inline]
    pub fn get(&self, j: usize, k: usize) -> f64 {
        if j < self.rows.len() && k < self.rows[j].len() {
            self.rows[j][k]
        } else {
            let mut val = 1.0 / ((j + 1) as f64);
            for m in 1..=k {
                val *= (m as f64) / ((j + m + 1) as f64);
            }
            val
        }
    }
}

/// Computes the causal Volterra convolution \int_0^\tau K(\tau - \sigma) F(\sigma) d\sigma.
fn convolve(
    kernel: &[(f64, f64)],
    forcing: &[(f64, f64)],
    table: &CausalWeights,
    work: &mut PacketWorkCounters,
) -> Vec<(f64, f64)> {
    let mut out = vec![(0.0, 0.0); kernel.len() + forcing.len()];
    for (j, &a) in kernel.iter().enumerate() {
        for (k, &b) in forcing.iter().enumerate() {
            work.volterra_terms += 1;
            let w = table.get(j, k);
            let term = c_scale(c_mul(a, b), w);
            out[j + k + 1] = c_add(out[j + k + 1], term);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Packet Structures and Compilation
// ---------------------------------------------------------------------------

/// A compiled finite-window packet at horizon h and Taylor order p.
#[derive(Clone, Debug, PartialEq)]
pub struct CompiledPacket {
    model: CollectiveModel,
    horizon: f64,
    order: usize,
    mu: Vec<f64>,
    nu: Vec<(f64, f64)>,
    free: Vec<(f64, f64)>,
    kernel: Vec<(f64, f64)>,
    // The generated descriptor and its model-derived bounds are private. This
    // prevents callers from changing an object after compilation and then
    // presenting it to a certificate or reconstruction routine.
    identity: PacketIdentity,
    model_bounds: PacketModelBounds,
}

/// Immutable model quantities used by the packet's diagnostic bounds.
///
/// These are computed in the same pass over the stored binary64 model payload
/// that is bound into [`PacketIdentity`]. They remain ordinary binary64
/// quantities: their presence does not make the certificate outward.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PacketModelBounds {
    weight_sum: f64,
    spectral_radius: f64,
}

impl PacketModelBounds {
    fn from_model(
        model: &CollectiveModel,
        work: &mut PacketWorkCounters,
    ) -> Result<Self, PacketError> {
        let mut weight_sum = 0.0;
        let mut spectral_radius: f64 = 0.0;
        for (&weight, &xi) in model.weights().iter().zip(model.xi()) {
            work.model_bounds_snapshot_items += 2;
            weight_sum += weight;
            spectral_radius = spectral_radius.max(xi.abs());
        }
        if !weight_sum.is_finite() || !spectral_radius.is_finite() {
            return Err(PacketError::NonFinite("spectral support"));
        }
        Ok(Self {
            weight_sum,
            spectral_radius,
        })
    }

    fn matches_model(&self, model: &CollectiveModel, work: &mut PacketWorkCounters) -> bool {
        let mut weight_sum = 0.0;
        let mut spectral_radius: f64 = 0.0;
        for (&weight, &xi) in model.weights().iter().zip(model.xi()) {
            work.model_bounds_comparison_items += 2;
            weight_sum += weight;
            spectral_radius = spectral_radius.max(xi.abs());
        }
        weight_sum.to_bits() == self.weight_sum.to_bits()
            && spectral_radius.to_bits() == self.spectral_radius.to_bits()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PacketIdentity {
    model: CollectiveModel,
    horizon: u64,
    order: usize,
    mu: Vec<u64>,
    nu: Vec<(u64, u64)>,
    free: Vec<(u64, u64)>,
    kernel: Vec<(u64, u64)>,
}

impl PacketIdentity {
    fn model_payload_items(model: &CollectiveModel) -> u64 {
        // xi, weights, and two scalar components for each initial amplitude;
        // width, chi, carrier, kappa, and cached initial energy.
        4 * model.num_modes() as u64 + 5
    }

    fn scalar_items(&self) -> u64 {
        Self::model_payload_items(&self.model)
            + 1 // horizon
            + self.mu.len() as u64
            + 2 * (self.nu.len() + self.free.len() + self.kernel.len()) as u64
    }

    fn new(packet: &CompiledPacket) -> Self {
        Self {
            model: packet.model.clone(),
            horizon: packet.horizon.to_bits(),
            order: packet.order,
            mu: packet.mu.iter().map(|x| x.to_bits()).collect(),
            nu: packet
                .nu
                .iter()
                .map(|z| (z.0.to_bits(), z.1.to_bits()))
                .collect(),
            free: packet
                .free
                .iter()
                .map(|z| (z.0.to_bits(), z.1.to_bits()))
                .collect(),
            kernel: packet
                .kernel
                .iter()
                .map(|z| (z.0.to_bits(), z.1.to_bits()))
                .collect(),
        }
    }

    fn matches(&self, packet: &CompiledPacket) -> bool {
        Self::model_matches(&self.model, &packet.model)
            && self.horizon == packet.horizon.to_bits()
            && self.order == packet.order
            && self.mu.len() == packet.mu.len()
            && self
                .mu
                .iter()
                .zip(&packet.mu)
                .all(|(&a, &b)| a == b.to_bits())
            && self.nu.len() == packet.nu.len()
            && self
                .nu
                .iter()
                .zip(&packet.nu)
                .all(|(&(ar, ai), &(br, bi))| ar == br.to_bits() && ai == bi.to_bits())
            && self.free.len() == packet.free.len()
            && self
                .free
                .iter()
                .zip(&packet.free)
                .all(|(&(ar, ai), &(br, bi))| ar == br.to_bits() && ai == bi.to_bits())
            && self.kernel.len() == packet.kernel.len()
            && self
                .kernel
                .iter()
                .zip(&packet.kernel)
                .all(|(&(ar, ai), &(br, bi))| ar == br.to_bits() && ai == bi.to_bits())
    }

    fn matches_identity(&self, other: &Self) -> bool {
        Self::model_matches(&self.model, &other.model)
            && self.horizon == other.horizon
            && self.order == other.order
            && self.mu == other.mu
            && self.nu == other.nu
            && self.free == other.free
            && self.kernel == other.kernel
    }

    fn model_matches(a: &CollectiveModel, b: &CollectiveModel) -> bool {
        a.xi().len() == b.xi().len()
            && a.xi()
                .iter()
                .zip(b.xi())
                .all(|(x, y)| x.to_bits() == y.to_bits())
            && a.weights().len() == b.weights().len()
            && a.weights()
                .iter()
                .zip(b.weights())
                .all(|(x, y)| x.to_bits() == y.to_bits())
            && a.initial_amplitudes().len() == b.initial_amplitudes().len()
            && a.initial_amplitudes()
                .iter()
                .zip(b.initial_amplitudes())
                .all(|(x, y)| x.0.to_bits() == y.0.to_bits() && x.1.to_bits() == y.1.to_bits())
            && a.width().to_bits() == b.width().to_bits()
            && a.chi().to_bits() == b.chi().to_bits()
            && a.carrier().to_bits() == b.carrier().to_bits()
            && a.kappa().to_bits() == b.kappa().to_bits()
            && a.initial_energy().to_bits() == b.initial_energy().to_bits()
    }
}

impl CompiledPacket {
    fn validate_identity(&self, work: &mut PacketWorkCounters) -> Result<(), PacketError> {
        work.identity_comparison_items += self.identity.scalar_items();
        if self.identity.matches(self) && self.model_bounds.matches_model(&self.model, work) {
            Ok(())
        } else {
            Err(PacketError::ModifiedDescriptor)
        }
    }

    /// Taylor order of this immutable generated packet.
    #[inline]
    pub const fn order(&self) -> usize {
        self.order
    }
}

/// Compiles moment prefix and basis polynomials into a [`CompiledPacket`].
pub fn compile_packet(
    model: &CollectiveModel,
    horizon: f64,
    order: usize,
    work: &mut PacketWorkCounters,
) -> Result<CompiledPacket, PacketError> {
    let coefficient_len = order
        .checked_add(1)
        .ok_or(PacketError::SizeOverflow("packet coefficients"))?;
    if model.kappa() != 0.0 {
        return Err(PacketError::UnsupportedDamping(model.kappa()));
    }
    if horizon <= 0.0 || !horizon.is_finite() {
        return Err(PacketError::InvalidHorizon(horizon));
    }

    let mut mu = checked_capacity::<f64>(coefficient_len, "spectral moments")?;
    let mut nu = checked_capacity::<(f64, f64)>(coefficient_len, "weighted moments")?;
    for k in 0..=order {
        mu.push(model.spectral_moment(k));
        nu.push(model.weighted_moment(k));
        work.moment_terms += (model.num_modes() as u64) * 2;
    }

    let mut free = checked_capacity::<(f64, f64)>(coefficient_len, "free coefficients")?;
    let mut kernel = checked_capacity::<(f64, f64)>(coefficient_len, "kernel coefficients")?;

    // b_0 = 1.0, b_k = b_{k-1} * (i * width * h) / k
    let mut b = (1.0, 0.0);
    free.push(c_mul(b, nu[0]));
    kernel.push(c_scale(b, mu[0]));

    let width = model.width();
    for k in 1..=order {
        // Multiply by (i * width * h) / k:
        // (u + i v) * (i * factor) = -factor * v + i * factor * u
        let factor = (width * horizon) / (k as f64);
        b = (-factor * b.1, factor * b.0);

        free.push(c_mul(b, nu[k]));
        kernel.push(c_scale(b, mu[k]));
    }

    let model_bounds = PacketModelBounds::from_model(model, work)?;
    let mut result = CompiledPacket {
        model: model.clone(),
        horizon,
        order,
        mu,
        nu,
        free,
        kernel,
        identity: PacketIdentity {
            model: model.clone(),
            horizon: horizon.to_bits(),
            order,
            mu: Vec::new(),
            nu: Vec::new(),
            free: Vec::new(),
            kernel: Vec::new(),
        },
        model_bounds,
    };
    work.model_copy_items += PacketIdentity::model_payload_items(model);
    // The temporary identity initialized above clones only its model payload;
    // PacketIdentity::new then copies the full descriptor below.
    work.identity_snapshot_items += PacketIdentity::model_payload_items(model);
    result.identity = PacketIdentity::new(&result);
    work.identity_snapshot_items += result.identity.scalar_items();
    Ok(result)
}

// ---------------------------------------------------------------------------
// Candidate Picard Iteration
// ---------------------------------------------------------------------------

/// A generated Picard polynomial waveform candidate for the complex mean.
#[derive(Clone, Debug, PartialEq)]
pub struct PacketCandidate {
    pub mean: Vec<(f64, f64)>,
    pub degree: usize,
    pub sweeps: usize,
    identity: PacketIdentity,
}

/// Generates a Picard candidate polynomial for the complex mean in sweeps.
pub fn generate_candidate(
    packet: &CompiledPacket,
    degree: usize,
    sweeps: usize,
    table: &CausalWeights,
    work: &mut PacketWorkCounters,
) -> Result<PacketCandidate, PacketError> {
    packet.validate_identity(work)?;
    if degree < packet.order {
        return Err(PacketError::DegreeLessThanOrder {
            degree,
            order: packet.order,
        });
    }
    if sweeps == 0 {
        return Err(PacketError::ZeroSweeps);
    }

    let candidate_len = degree
        .checked_add(1)
        .ok_or(PacketError::SizeOverflow("candidate coefficients"))?;
    let force_len = candidate_len
        .checked_mul(3)
        .and_then(|n| n.checked_sub(2))
        .ok_or(PacketError::SizeOverflow("candidate cubic force"))?;
    let convolution_len = packet
        .kernel
        .len()
        .checked_add(force_len)
        .ok_or(PacketError::SizeOverflow("candidate convolution"))?;
    // Validate the largest intermediate before any candidate allocation.
    checked_len::<(f64, f64)>(convolution_len, "candidate convolution")?;
    let mut p = packet.free.clone();
    p.try_reserve_exact(candidate_len.saturating_sub(p.len()))
        .map_err(|_| PacketError::SizeOverflow("candidate coefficients"))?;
    p.resize(candidate_len, (0.0, 0.0));

    let h = packet.horizon;
    let chi = packet.model.chi();

    for _ in 0..sweeps {
        let f = force(&p, chi, work);
        let conv_res = convolve(&packet.kernel, &f, table, work);
        let scaled_conv: Vec<(f64, f64)> = conv_res.into_iter().map(|c| c_scale(c, h)).collect();
        let mut next = poly_add(&packet.free, &scaled_conv);
        next.truncate(candidate_len);
        if next.len() < candidate_len {
            next.resize(candidate_len, (0.0, 0.0));
        }

        // Exact original initial mean enforcement at t=0
        next[0] = packet.nu[0];
        p = next;
    }

    work.identity_snapshot_items += packet.identity.scalar_items();
    Ok(PacketCandidate {
        mean: p,
        degree,
        sweeps,
        identity: packet.identity.clone(),
    })
}

// ---------------------------------------------------------------------------
// Native Total Certificate
// ---------------------------------------------------------------------------

/// Analytic error estimate for the candidate on [0, h].
///
/// `roundoff_enclosed` is false until the whole coefficient, residual, norm,
/// tail and reconstruction calculation uses verified outward arithmetic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PacketCertificate {
    pub roundoff_enclosed: bool,
    pub mean_bound: f64,
    pub mode_bounds: Vec<f64>,
    pub volterra_defect: f64,
    pub kernel_model: f64,
    /// Bare one-mode exponential-series tail bound B (diagnostic arithmetic).
    pub bare_tail: f64,
    /// Weighted spectral-kernel tail S*B (diagnostic arithmetic).
    pub kernel_tail: f64,
    /// Weighted free-mean tail A0*B (diagnostic arithmetic).
    pub free_tail: f64,
    /// Compatibility alias for `kernel_tail`.
    pub tail: f64,
    pub candidate_domain_bound: f64,
    pub lipschitz_l: f64,
    /// Bare nonlinear Lipschitz factor L_F = 3|chi| for one microscopic mode.
    pub microscopic_lipschitz_l: f64,
    pub gain_g: f64,
    pub horizon: f64,
    pub order: usize,
}

fn factorial(n: usize) -> f64 {
    let mut f = 1.0;
    for i in 2..=n {
        f *= i as f64;
    }
    f
}

/// Produces a physical-time outward reconstruction for the kappa=0 packet
/// family, including nonlinear coupling. Coefficients are replayed with
/// intervals and the returned binary64 centers must lie in those intervals.
/// The function is an opt-in local enclosure primitive; its result remains
/// non-authoritative pending admission of the full arithmetic and theorem
/// premises. It does not change the authority of [`PacketCertificate`].
pub fn reconstruct_outward_at_times(
    cand: &PacketCandidate,
    packet: &CompiledPacket,
    request: OutwardPacketRequest<'_>,
    work: &mut PacketWorkCounters,
) -> Result<OutwardPacketOutput, PacketError> {
    packet.validate_identity(work)?;
    if !cand.identity.matches_identity(&packet.identity) {
        return Err(PacketError::CandidatePacketMismatch);
    }
    let expected_mean_len = cand
        .degree
        .checked_add(1)
        .ok_or(PacketError::OutwardRefusal(
            "candidate degree overflows length",
        ))?;
    if cand.mean.len() != expected_mean_len {
        return Err(PacketError::OutwardRefusal(
            "candidate mean/degree shape mismatch",
        ));
    }
    let n = packet.model.num_modes();
    if request.initial_radii.len() != n {
        return Err(PacketError::OutwardRefusal(
            "initial radius length mismatch",
        ));
    }
    for &eta in request.initial_radii {
        if !eta.is_finite() || eta < 0.0 {
            return Err(PacketError::OutwardRefusal(
                "initial radius must be finite and nonnegative",
            ));
        }
    }
    // Replay every generated coefficient from the immutable stored model in
    // directed arithmetic before a returned center can be used.  A mismatch
    // is a refusal, never a widened post-hoc radius.
    let (replayed, replayed_free, replayed_kernel) =
        outward_picard_replay(packet, cand.degree, cand.sweeps, work)?;
    if replayed.len() != cand.mean.len()
        || replayed
            .iter()
            .zip(&cand.mean)
            .any(|(x, &(re, im))| !x.re.contains(re) || !x.im.contains(im))
    {
        return Err(PacketError::OutwardRefusal(
            "candidate center is outside directed replay",
        ));
    }
    let rho = outward_residual_l1(
        &replayed,
        &replayed_free,
        &replayed_kernel,
        packet.horizon,
        packet.model.chi(),
        work,
    )?;
    let (emean, full_b, theorem_r, theorem_rp) =
        outward_scalar_bounds(packet, &replayed, request.initial_radii, rho, work)?;
    let h = packet.horizon;
    let mode_output_len = request
        .times
        .len()
        .checked_mul(n)
        .ok_or(PacketError::SizeOverflow("outward mode outputs"))?;
    let mut output = OutwardPacketOutput {
        times: checked_capacity(request.times.len(), "outward times")?,
        modes: checked_capacity(mode_output_len, "outward mode centers")?,
        mode_radii: checked_capacity(mode_output_len, "outward mode radii")?,
        means: checked_capacity(request.times.len(), "outward mean centers")?,
        mean_radii: checked_capacity(request.times.len(), "outward mean radii")?,
        roundoff_enclosed: false,
    };
    for &t in request.times {
        if !t.is_finite() || t < 0.0 || t > h {
            return Err(PacketError::OutwardRefusal(
                "physical time outside packet horizon",
            ));
        }
        let tau = if t.to_bits() == 0 {
            Interval::point(0.0)
        } else if t.to_bits() == h.to_bits() {
            Interval::point(1.0)
        } else {
            Interval::point(t).and_then(|a| a.div(Interval::point(h)?))
        }
        .map_err(|_| PacketError::OutwardRefusal("physical time normalization"))?;
        if tau.lo < 0.0 || tau.hi > 1.0 {
            return Err(PacketError::OutwardRefusal(
                "physical time normalization escapes unit interval",
            ));
        }
        let restricted_i = ic_restrict_physical(&replayed, tau, work)?;
        let force_i = ic_poly_force(&restricted_i, packet.model.chi(), work)?;
        precharge_outward_output(work, cand.mean.len(), packet.order);
        #[cfg(test)]
        if TEST_REFUSE_AFTER_OUTPUT_PRECHARGE.swap(false, Ordering::SeqCst) {
            return Err(PacketError::OutwardRefusal(
                "test refusal after output precharge",
            ));
        }
        let mut restricted_c = cand.mean.clone();
        let mut tau_power = 1.0;
        let tau_c = t / h;
        for coefficient in &mut restricted_c {
            *coefficient = c_scale(*coefficient, tau_power);
            tau_power *= tau_c;
        }
        let force_c = force(&restricted_c, packet.model.chi(), work);
        let x = Interval::point(packet.model.width())
            .and_then(|w| w.mul(Interval::point(t)?))
            .map_err(|_| PacketError::OutwardRefusal("physical frequency-time product"))?;
        output.times.push(t);
        let mut mean_re =
            Interval::point(0.0).map_err(|_| PacketError::OutwardRefusal("mean setup"))?;
        let mut mean_im = mean_re;
        let mut mean_radius =
            Interval::point(0.0).map_err(|_| PacketError::OutwardRefusal("mean setup"))?;
        for j in 0..n {
            // b_p(i width xi t), with both ordinary returned center and an
            // outward recurrence over the represented physical time.
            let xi = packet.model.xi()[j];
            let mut center_b = (1.0, 0.0);
            let mut ib = IComplex::point(center_b)?;
            let mut center_beta = center_b;
            let mut ibeta = ib;
            let ix = x
                .mul(Interval::point(xi).map_err(|_| PacketError::OutwardRefusal("frequency"))?)
                .map_err(|_| PacketError::OutwardRefusal("frequency-time product"))?;
            for k in 1..=packet.order {
                let f = (packet.model.width() * xi * t) / k as f64;
                center_b = (-f * center_b.1, f * center_b.0);
                let denominator = Interval::point(k as f64)
                    .map_err(|_| PacketError::OutwardRefusal("factorial"))?;
                let factor = ix
                    .div(denominator)
                    .map_err(|_| PacketError::OutwardRefusal("basis"))?;
                ib = IComplex {
                    re: -ib
                        .im
                        .mul(factor)
                        .map_err(|_| PacketError::OutwardRefusal("basis"))?,
                    im: ib
                        .re
                        .mul(factor)
                        .map_err(|_| PacketError::OutwardRefusal("basis"))?,
                };
                center_beta = c_add(center_beta, center_b);
                ibeta = IComplex {
                    re: ibeta
                        .re
                        .add(ib.re)
                        .map_err(|_| PacketError::OutwardRefusal("basis sum"))?,
                    im: ibeta
                        .im
                        .add(ib.im)
                        .map_err(|_| PacketError::OutwardRefusal("basis sum"))?,
                };
                work.certificate_items += 1;
            }
            let z = packet.model.initial_amplitudes()[j];
            let mut gamma_sum = (0.0, 0.0);
            let mut basis_center = (1.0, 0.0);
            let omega_time = packet.model.width() * xi * t;
            for l in 0..=packet.order {
                let mut acc = (0.0, 0.0);
                let mut beta_weight = 1.0 / (l + 1) as f64;
                for (k, &fk) in force_c.iter().enumerate() {
                    if k > 0 {
                        beta_weight *= k as f64 / (l + k + 1) as f64;
                    }
                    acc = c_add(acc, c_scale(fk, beta_weight));
                }
                gamma_sum = c_add(gamma_sum, c_mul(basis_center, acc));
                if l < packet.order {
                    let f = omega_time / (l + 1) as f64;
                    basis_center = (-f * basis_center.1, f * basis_center.0);
                }
            }
            let center = c_add(c_mul(center_beta, z), c_scale(gamma_sum, t));
            let iz = IComplex::point(z)?;
            let igamma = ic_gamma_physical(
                &force_i,
                ix,
                Interval::point(t).map_err(|_| PacketError::OutwardRefusal("physical time"))?,
                packet.order,
                work,
            )?;
            let linear_iv = ibeta.mul(iz)?;
            let iv = IComplex {
                re: linear_iv
                    .re
                    .add(igamma.re)
                    .map_err(|_| PacketError::OutwardRefusal("mode reconstruction"))?,
                im: linear_iv
                    .im
                    .add(igamma.im)
                    .map_err(|_| PacketError::OutwardRefusal("mode reconstruction"))?,
            };
            if !iv.re.contains(center.0) || !iv.im.contains(center.1) {
                return Err(PacketError::OutwardRefusal(
                    "returned mode center outside reconstruction interval",
                ));
            }
            let re_disp = iv
                .re
                .sub(
                    Interval::point(center.0)
                        .map_err(|_| PacketError::OutwardRefusal("mode center"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode displacement"))?
                .mag();
            let im_disp = iv
                .im
                .sub(
                    Interval::point(center.1)
                        .map_err(|_| PacketError::OutwardRefusal("mode center"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode displacement"))?
                .mag();
            let arithmetic = Interval::point(re_disp)
                .map_err(|_| PacketError::OutwardRefusal("mode displacement"))?
                .add(
                    Interval::point(im_disp)
                        .map_err(|_| PacketError::OutwardRefusal("mode displacement"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode displacement"))?;
            let nonlinear_a = Interval::point(3.0)
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .mul(
                    Interval::point(theorem_r)
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .mul(
                    Interval::point(theorem_r)
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .mul(
                    Interval::point(emean)
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?;
            let nonlinear_b = Interval::point(theorem_rp)
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .mul(
                    Interval::point(theorem_rp)
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .mul(
                    Interval::point(theorem_rp)
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .mul(
                    Interval::point(full_b)
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?;
            let nonlinear_radius = Interval::point(t)
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .mul(
                    Interval::point(packet.model.chi().abs())
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .mul(
                    nonlinear_a
                        .add(nonlinear_b)
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?;
            let radius = arithmetic
                .add(
                    Interval::point(outward_complex_upper(z, "mode initial magnitude")?)
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                        .mul(
                            Interval::point(full_b)
                                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                        )
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .add(nonlinear_radius)
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .add(
                    Interval::point(request.initial_radii[j])
                        .map_err(|_| PacketError::OutwardRefusal("mode radius"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mode radius"))?
                .hi;
            if !radius.is_finite() {
                return Err(PacketError::OutwardRefusal("mode radius overflow"));
            }
            output.modes.push(center);
            output.mode_radii.push(radius);
            let w = Interval::point(packet.model.weights()[j])
                .map_err(|_| PacketError::OutwardRefusal("weight"))?;
            mean_re = mean_re
                .add(
                    w.mul(
                        Interval::point(center.0)
                            .map_err(|_| PacketError::OutwardRefusal("center"))?,
                    )
                    .map_err(|_| PacketError::OutwardRefusal("mean accumulation"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mean accumulation"))?;
            mean_im = mean_im
                .add(
                    w.mul(
                        Interval::point(center.1)
                            .map_err(|_| PacketError::OutwardRefusal("center"))?,
                    )
                    .map_err(|_| PacketError::OutwardRefusal("mean accumulation"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mean accumulation"))?;
            mean_radius = mean_radius
                .add(
                    w.mul(
                        Interval::point(radius)
                            .map_err(|_| PacketError::OutwardRefusal("radius"))?,
                    )
                    .map_err(|_| PacketError::OutwardRefusal("mean radius accumulation"))?,
                )
                .map_err(|_| PacketError::OutwardRefusal("mean radius accumulation"))?;
            work.output_value_items += 2;
        }
        // A lower endpoint is a finite returned center without midpoint
        // overflow.  The directed widths enclose its displacement to every
        // point in the accumulated weighted-center intervals.
        let mean = (mean_re.lo, mean_im.lo);
        let re_width = mean_re
            .sub(Interval::point(mean.0).map_err(|_| PacketError::OutwardRefusal("mean center"))?)
            .map_err(|_| PacketError::OutwardRefusal("mean width"))?
            .hi;
        let im_width = mean_im
            .sub(Interval::point(mean.1).map_err(|_| PacketError::OutwardRefusal("mean center"))?)
            .map_err(|_| PacketError::OutwardRefusal("mean width"))?
            .hi;
        let rounding = Interval::point(re_width)
            .map_err(|_| PacketError::OutwardRefusal("mean width"))?
            .add(Interval::point(im_width).map_err(|_| PacketError::OutwardRefusal("mean width"))?)
            .map_err(|_| PacketError::OutwardRefusal("mean width"))?
            .hi;
        let radius = Interval::point(mean_radius.hi)
            .map_err(|_| PacketError::OutwardRefusal("mean radius"))?
            .add(
                Interval::point(rounding)
                    .map_err(|_| PacketError::OutwardRefusal("mean radius"))?,
            )
            .map_err(|_| PacketError::OutwardRefusal("mean radius"))?
            .hi;
        output.means.push(mean);
        output.mean_radii.push(radius);
    }
    Ok(output)
}

/// Computes the continuous-model tail and defect estimate in ordinary binary64.
/// The result explicitly reports `roundoff_enclosed = false`.
pub fn compute_certificate(
    cand: &PacketCandidate,
    packet: &CompiledPacket,
    table: &CausalWeights,
    work: &mut PacketWorkCounters,
) -> Result<PacketCertificate, PacketError> {
    packet.validate_identity(work)?;
    work.identity_comparison_items += packet.identity.scalar_items();
    if !cand.identity.matches_identity(&packet.identity) {
        return Err(PacketError::CandidatePacketMismatch);
    }
    if packet.model.kappa() != 0.0 {
        return Err(PacketError::UnsupportedDamping(packet.model.kappa()));
    }
    if !packet.horizon.is_finite() || packet.horizon <= 0.0 {
        return Err(PacketError::InvalidHorizon(packet.horizon));
    }
    if cand
        .mean
        .iter()
        .chain(packet.free.iter())
        .chain(packet.kernel.iter())
        .any(|&(re, im)| !re.is_finite() || !im.is_finite())
    {
        return Err(PacketError::NonFinite("coefficient"));
    }
    let h = packet.horizon;
    let mut bound_mean = poly_bound(&cand.mean);
    work.certificate_items += cand.mean.len() as u64;
    if !bound_mean.is_finite() {
        return Err(PacketError::NonFinite("candidate domain bound"));
    }
    if bound_mean > 1.0 {
        // The candidate center is a represented binary64 polynomial. Lift
        // those coefficients to exact point intervals before the optional
        // basis transform; the diagnostic certificate remains non-authoritative.
        let mut lifted = Vec::with_capacity(cand.mean.len());
        for &(re, im) in &cand.mean {
            lifted.push((
                Interval::point(re).map_err(|_| PacketError::NonFinite("candidate coefficient"))?,
                Interval::point(im).map_err(|_| PacketError::NonFinite("candidate coefficient"))?,
            ));
        }
        let refined = outward_polynomial_sup_bound(&lifted, 1.0, 4, work)?;
        bound_mean = refined.upper;
        if bound_mean > 1.0 {
            return Err(PacketError::DomainExceeded { bound: bound_mean });
        }
    }

    let chi = packet.model.chi();
    // These are the stored-model sum S = sum(w_j) and maximum support
    // max_j |xi_j| captured at generation. validate_identity above replays
    // the same binary64 reduction before they are used. They are diagnostic
    // binary64 values, not exact-real or outward interval bounds.
    let weight_sum = packet.model_bounds.weight_sum;
    let spectral_radius = packet.model_bounds.spectral_radius;
    let l = 3.0 * chi.abs() * weight_sum;
    let g = chi.abs();

    let hl = h * l;
    if hl >= 1.0 - 1e-14 {
        return Err(PacketError::GronwallConditionViolated { h_times_l: hl });
    }

    let f = force(&cand.mean, chi, work);
    let conv_res = convolve(&packet.kernel, &f, table, work);
    let scaled_conv: Vec<(f64, f64)> = conv_res.into_iter().map(|c| c_scale(c, h)).collect();
    let expected = poly_add(&packet.free, &scaled_conv);
    let residual = poly_sub(&cand.mean, &expected);
    let defect = poly_bound(&residual);

    // Bare exponential-series tail B for the actual maximum support.
    let width = packet.model.width();
    let p = packet.order;
    let bare_tail = (width * spectral_radius * h).powi((p + 1) as i32) / factorial(p + 1);
    let kernel_tail = bare_tail * weight_sum;

    // Initial amplitude upper bound sum w_j |z_j|
    let mut a0 = 0.0;
    for (&w, &z) in packet
        .model
        .weights()
        .iter()
        .zip(packet.model.initial_amplitudes().iter())
    {
        work.certificate_items += 1;
        a0 += w * c_upper(z);
    }

    let free_tail = a0 * bare_tail;
    let nonlinear_mean_tail = kernel_tail * (h * g) / ((p + 2) as f64);
    let kernel_model = free_tail + nonlinear_mean_tail;
    let mean_bound = (defect + kernel_model) / (1.0 - hl);

    let mut mode_bounds = Vec::with_capacity(packet.model.num_modes());
    let microscopic_lipschitz = 3.0 * chi.abs();
    for &z in packet.model.initial_amplitudes() {
        work.certificate_items += 1;
        let microscopic_tail = bare_tail * (c_upper(z) + (h * g) / ((p + 2) as f64));
        let emode = microscopic_tail + h * microscopic_lipschitz * mean_bound;
        mode_bounds.push(emode);
    }

    if [
        defect,
        bare_tail,
        kernel_tail,
        free_tail,
        nonlinear_mean_tail,
        kernel_model,
        mean_bound,
        l,
        g,
        microscopic_lipschitz,
    ]
    .iter()
    .any(|x| !x.is_finite())
        || mode_bounds.iter().any(|x| !x.is_finite())
    {
        return Err(PacketError::NonFinite("error estimate"));
    }

    Ok(PacketCertificate {
        roundoff_enclosed: false,
        mean_bound,
        mode_bounds,
        volterra_defect: defect,
        kernel_model,
        bare_tail,
        kernel_tail,
        free_tail,
        tail: kernel_tail,
        candidate_domain_bound: bound_mean,
        lipschitz_l: l,
        microscopic_lipschitz_l: microscopic_lipschitz,
        gain_g: g,
        horizon: h,
        order: p,
    })
}

// ---------------------------------------------------------------------------
// Microscopic Mode Reconstruction in O(p N)
// ---------------------------------------------------------------------------

/// Computes reconstruction polynomials beta(\xi) and gamma(\xi).
#[allow(clippy::type_complexity)]
pub fn reconstruction_polynomials(
    cand: &PacketCandidate,
    packet: &CompiledPacket,
    work: &mut PacketWorkCounters,
) -> CoreResult<(Vec<(f64, f64)>, Vec<(f64, f64)>)> {
    packet.validate_identity(work).map_err(CoreError::from)?;
    work.identity_comparison_items += packet.identity.scalar_items();
    if !cand.identity.matches_identity(&packet.identity) {
        return Err(CoreError::InvalidInput(
            PacketError::CandidatePacketMismatch.to_string(),
        ));
    }
    let p = packet.order;
    let h = packet.horizon;
    let width = packet.model.width();
    let chi = packet.model.chi();

    let f = force(&cand.mean, chi, work);

    let mut beta = Vec::with_capacity(p + 1);
    let mut gamma = Vec::with_capacity(p + 1);

    let mut b = (1.0, 0.0);
    for j in 0..=p {
        if j > 0 {
            let factor = (width * h) / (j as f64);
            b = (-factor * b.1, factor * b.0);
        }
        beta.push(b);

        let mut acc = (0.0, 0.0);
        let mut w = 1.0 / ((j + 1) as f64);
        for (k, &fk) in f.iter().enumerate() {
            work.reconstruction_terms += 1;
            if k > 0 {
                w *= (k as f64) / ((j + k + 1) as f64);
            }
            acc = c_add(acc, c_scale(fk, w));
        }
        gamma.push(c_scale(c_mul(b, acc), h));
    }

    Ok((beta, gamma))
}

/// Reconstructs all N microscopic mode amplitudes a_j(h) at the horizon end.
pub fn reconstruct_modes(
    cand: &PacketCandidate,
    packet: &CompiledPacket,
    work: &mut PacketWorkCounters,
) -> CoreResult<Vec<(f64, f64)>> {
    let (beta, gamma) = reconstruction_polynomials(cand, packet, work)?;
    let mut out = Vec::with_capacity(packet.model.num_modes());

    for (&xi, &z) in packet
        .model
        .xi()
        .iter()
        .zip(packet.model.initial_amplitudes().iter())
    {
        let b_val = poly_eval(&beta, xi, work);
        let g_val = poly_eval(&gamma, xi, work);
        out.push(c_add(c_mul(b_val, z), g_val));
        work.output_value_items += 2;
    }

    Ok(out)
}

/// Evaluates a single mode a_j(tau) at normalized time tau = t/h \in [0, 1].
pub fn evaluate_mode_at(
    cand: &PacketCandidate,
    packet: &CompiledPacket,
    mode_idx: usize,
    tau: f64,
    work: &mut PacketWorkCounters,
) -> CoreResult<(f64, f64)> {
    packet.validate_identity(work).map_err(CoreError::from)?;
    work.identity_comparison_items += packet.identity.scalar_items();
    if !cand.identity.matches_identity(&packet.identity) {
        return Err(CoreError::InvalidInput(
            PacketError::CandidatePacketMismatch.to_string(),
        ));
    }
    if mode_idx >= packet.model.num_modes() {
        return Err(CoreError::InvalidInput(format!(
            "evaluate_mode_at: mode_idx {mode_idx} >= num_modes {}",
            packet.model.num_modes()
        )));
    }
    if !(0.0..=1.0).contains(&tau) {
        return Err(CoreError::InvalidInput(format!(
            "evaluate_mode_at: tau {tau} out of range [0, 1]"
        )));
    }
    if tau == 0.0 {
        work.output_value_items += 2;
        return Ok(packet.model.initial_amplitudes()[mode_idx]);
    }

    let h = packet.horizon * tau;
    let sub_packet = compile_packet(&packet.model, h, packet.order, work)?;
    // For the shorter normalized coordinate s, the old waveform is P(tau*s).
    // Restrict coefficients before applying the nonlinear force and convolution.
    let mut restricted = cand.clone();
    work.output_copy_items += (2 * cand.mean.len()) as u64 + cand.identity.scalar_items();
    work.output_restriction_multiplies += (3 * cand.mean.len()) as u64;
    let mut scale = 1.0;
    for coefficient in &mut restricted.mean {
        *coefficient = c_scale(*coefficient, scale);
        scale *= tau;
    }
    // This is the intentional, exact descriptor transition for the restricted
    // waveform: P(tau*s) is reconstructed with the correspondingly shortened
    // linear kernel on [0, tau*h].
    restricted.identity = sub_packet.identity.clone();
    work.output_copy_items += sub_packet.identity.scalar_items();
    let modes = reconstruct_modes(&restricted, &sub_packet, work)?;
    Ok(modes[mode_idx])
}

#[cfg(test)]
mod outward_scalar_tests {
    use super::*;

    #[test]
    fn rp_greater_than_energy_radius_controls_nonlinear_tail() {
        let model =
            CollectiveModel::new(vec![1.0], vec![1.0], vec![(0.0, 0.0)], 0.2, 1.0, 0.0, 0.0)
                .unwrap();
        let mut work = PacketWorkCounters::new();
        let packet = compile_packet(&model, 1.0e-3, 2, &mut work).unwrap();
        let p = vec![IComplex::point((2.0, 0.0)).unwrap()]; // R_P=2 > R_E=A0=0
        let (e, b, r, rp) = outward_scalar_bounds(&packet, &p, &[0.0], 0.0, &mut work).unwrap();
        assert_eq!(r, rp);
        let lower = packet.horizon * packet.model.chi().abs() * rp.powi(3) * b;
        assert!(e >= lower, "e={e:e} lower={lower:e}");
    }

    #[test]
    fn output_precharge_survives_test_refusal_boundary() {
        let mut work = PacketWorkCounters::new();
        let model =
            CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.5, 0.0)], 0.0, 0.0, 0.0, 0.0)
                .unwrap();
        let packet = compile_packet(&model, 0.01, 3, &mut work).unwrap();
        let table = CausalWeights::compile(3, 9);
        let cand = generate_candidate(&packet, 3, 1, &table, &mut work).unwrap();
        let before = work;
        TEST_REFUSE_AFTER_OUTPUT_PRECHARGE.store(true, Ordering::SeqCst);
        assert!(matches!(
            reconstruct_outward_at_times(
                &cand,
                &packet,
                OutwardPacketRequest {
                    times: &[0.01],
                    initial_radii: &[0.0]
                },
                &mut work
            ),
            Err(PacketError::OutwardRefusal(_))
        ));
        assert!(work.output_copy_items > before.output_copy_items);
        assert!(work.output_restriction_multiplies > before.output_restriction_multiplies);
        assert!(work.reconstruction_terms > before.reconstruction_terms);
    }
}
