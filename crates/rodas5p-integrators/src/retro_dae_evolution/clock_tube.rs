//! Opt-in exact-source, two-dimensional clock reduction. The carry is in
//! a fixed weighted norm dominating the original coordinate infinity norm.
//! Original local branch/condition proof is mandatory on every slab.
use super::polynomial_clock::ClockPolynomial;
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ClockTubePlan {
    Sup4,
    Mean4,
    Mean8,
    Hybrid4,
    PolynomialMean4,
    PolynomialMean8,
    PolynomialHybrid4,
    BlockAdaptiveMean,
    BlockAdaptiveHybrid,
    BlockSplit,
    BlockSplitDecay1,
    BlockSplitDecay4,
    BlockSplitRational4,
    BlockSplitDecay2,
    BlockSplitRational2,
}
impl ClockTubePlan {
    fn block(self) -> bool {
        matches!(
            self,
            Self::BlockAdaptiveMean
                | Self::BlockAdaptiveHybrid
                | Self::BlockSplit
                | Self::BlockSplitDecay1
                | Self::BlockSplitDecay4
                | Self::BlockSplitRational4
                | Self::BlockSplitDecay2
                | Self::BlockSplitRational2
        )
    }
    fn decay(self) -> bool {
        matches!(
            self,
            Self::BlockSplitDecay1
                | Self::BlockSplitDecay4
                | Self::BlockSplitRational4
                | Self::BlockSplitDecay2
                | Self::BlockSplitRational2
        )
    }
    fn weight(self) -> f64 {
        if matches!(self, Self::Mean8 | Self::PolynomialMean8) {
            8.
        } else {
            4.
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ClockTubeWork {
    pub plan: ClockTubePlan,
    pub weight: f64,
    pub p_lower: f64,
    pub seam: f64,
    pub residual_integral_upper: f64,
    pub incoming: f64,
    pub outgoing: f64,
}
// Exact syntactic source check; unit factors only. Codes 0=time, 1..32=source state index+1.
// Do not replace with a numerical fit or rounded polynomial comparison.
pub(super) fn matches_terms(eq: &DeclaredPolynomial, want: &[(f64, &[u8])]) -> bool {
    if eq.terms().len() != want.len() || want.len() > 16 {
        return false;
    }
    let mut used = [false; 16];
    for term in eq.terms() {
        if term.factors.len() > 4 {
            return false;
        }
        let mut a = [0u8; 4];
        for (v, f) in a.iter_mut().zip(&term.factors) {
            if f.constant != 0. {
                return false;
            }
            *v = if f.time == 1. && f.state.is_empty() {
                0
            } else if f.time == 0. && f.state.len() == 1 && f.state[0].1 == 1. && f.state[0].0 < 32
            {
                1 + f.state[0].0 as u8
            } else {
                return false;
            };
        }
        let a = &mut a[..term.factors.len()];
        a.sort_unstable();
        let Some(k) = want
            .iter()
            .enumerate()
            .position(|(i, (c, b))| !used[i] && *c == term.coefficient && *b == a)
        else {
            return false;
        };
        used[k] = true;
    }
    true
}
pub(super) fn source_kind(src: &DaeSource) -> Option<bool> {
    if src.nx() != 2
        || src.nz() != 2
        || !matches_terms(&src.g[0], &[(1., &[3]), (-1., &[1])])
        || !matches_terms(&src.g[1], &[(1., &[4]), (-1., &[2])])
    {
        return None;
    }
    if matches_terms(&src.f[0], &[(-1., &[0, 3, 3])]) && matches_terms(&src.f[1], &[(1., &[0, 3])])
    {
        return Some(false);
    }
    if matches_terms(
        &src.f[0],
        &[
            (-1., &[0, 3, 3]),
            (2., &[0, 3, 4]),
            (-1., &[0, 4, 4]),
            (1., &[0, 3]),
            (-1., &[0, 4]),
        ],
    ) && matches_terms(&src.f[1], &[(1., &[0, 3]), (-1., &[0, 4])])
    {
        return Some(true);
    }
    None
}
fn derivative(a: &ScratchPoly) -> CoreResult<ScratchPoly> {
    let mut out = ScratchPoly::new(&[zero()])?;
    out.len = (a.len - 1).max(1);
    for (j, &v) in a.slice().iter().enumerate().skip(1) {
        out.a[j - 1] = v.scale(j as f64)?;
    }
    Ok(out)
}
fn transform_norm(u: Iv, v: Iv, coupled: bool, w: f64) -> CoreResult<f64> {
    add_up(
        mul_up(w, if coupled { u.sub(v)?.mag() } else { u.mag() })?,
        v.mag(),
    )
}
fn bound(a: &ScratchPoly, plan: ClockTubePlan) -> CoreResult<f64> {
    let b = bernstein_magnitude(a.slice(), plan != ClockTubePlan::Sup4)?;
    if matches!(
        plan,
        ClockTubePlan::Hybrid4
            | ClockTubePlan::PolynomialHybrid4
            | ClockTubePlan::BlockAdaptiveHybrid
    ) {
        // Reuse prior spectral enclosure on the *same* normalized polynomial.
        // |P_j|<=1 gives a second upper bound on its absolute integral.
        Ok(b.min(legendre_norm(&power_legendre(a.slice())?)?))
    } else {
        Ok(b)
    }
}
// Exact represented dyadic subcells. Residual polynomials are in normalized
// slab time, so their absolute integrals acquire only the subcell width.
fn restrict_poly(a: &ScratchPoly, left: f64, width: f64) -> CoreResult<ScratchPoly> {
    let mut coeff = a.slice().iter().rev();
    let mut v = ScratchPoly::new(&[*coeff.next().ok_or_else(|| bad("empty decay residual"))?])?;
    let t = [p(left)?, p(width)?];
    for &c in coeff {
        v = v.mul(&t)?.add(&[c])?;
    }
    Ok(v)
}
fn subcell_integral(a: &ScratchPoly, left: f64, width: f64, h: Iv) -> CoreResult<Iv> {
    let a = restrict_poly(a, left, width)?;
    let mut m = zero();
    for (i, &v) in a.slice().iter().enumerate() {
        m = m.add(v.div(p((i + 1) as f64)?)?)?;
    }
    super::triangular_goal::positive(m.scale(width)?.mul(h)?)
}
// P>=0 and beta<=p_exact+p_hat. For a cell,
// E_p(end)<=exp(-beta*M)*E_in+R, E_p(prefix)<=E_in+R.
// Integral P*|e_p| <= (E_in+R)*phi_beta(M).
fn decay_factors(beta: f64, m: Iv, rational: bool) -> CoreResult<(f64, f64)> {
    use rodas5p_core::{
        directed::{mul_down, sub_up},
        polynomial_action::exp_neg_enclosure,
    };
    if beta == 0. || m.hi == 0. {
        return Ok((1., m.hi));
    }
    let z = mul_down(beta, m.lo)?.max(0.);
    let decay = if rational {
        p(1.)?.div(p(1.)?.add(p(z)?)?)?.hi.min(1.)
    } else {
        exp_neg_enclosure(z.min(700.))?.hi.min(1.)
    };
    let zhi = mul_up(beta, m.hi)?;
    let mut phi = m.hi;
    // Avoid reciprocal overflow and cancellation at vanishing damping.
    if zhi > 1. {
        phi = phi.min(div_up(1., beta)?);
    }
    if !rational && zhi >= 2f64.powi(-20) && zhi <= 700. {
        phi = phi.min(div_up(sub_up(1., exp_neg_enclosure(zhi)?.lo)?, beta)?);
    }
    Ok((decay, phi))
}
// Fixed-capacity per-block carry. A changing norm is converted at the
// seam, never silently reinterpreted. Split carries two component radii.
#[derive(Clone, Copy)]
struct BlockCarry {
    n: usize,
    coupled: bool,
    start: f64,
    initial: [Iv; 16],
    previous: [Iv; 16],
    clocks: [ClockPolynomial; 8],
    weights: [f64; 8],
    radius: [f64; 8],
    q_radius: [f64; 8],
}
impl BlockCarry {
    fn new(src: &DaeSource, start: f64, x: &[f64]) -> CoreResult<Self> {
        let n = src.nx();
        if n > 16 || src.nz() != n {
            return Err(bad("block clock partition"));
        }
        let coupled =
            super::triangular_goal::clock_partition(src, x, &[0.; 32][..2 * n], start, start)?;
        let mut a = Self {
            n,
            coupled,
            start,
            initial: [zero(); 16],
            previous: [zero(); 16],
            clocks: [ClockPolynomial::coupled(); 8],
            weights: [1.; 8],
            radius: [0.; 8],
            q_radius: [0.; 8],
        };
        for i in (0..n).step_by(2) {
            a.clocks[i / 2] = if coupled {
                ClockPolynomial::coupled()
            } else {
                ClockPolynomial::parse(src, i)?
            };
            let (pp, q) = super::triangular_goal::initial_block(x, i, coupled)?;
            a.initial[i] = pp;
            a.initial[i + 1] = q;
            a.previous[i] = pp;
            a.previous[i + 1] = q;
        }
        Ok(a)
    }
    fn step(
        &mut self,
        plan: ClockTubePlan,
        times: [f64; 2],
        y: &[Poly],
        tube: f64,
        incoming: f64,
        out: &mut SlabCertificate,
    ) -> CoreResult<ClockTubeWork> {
        let h = p(times[1])?.sub(p(times[0])?)?;
        if h.lo <= 0. || y.len() != 2 * self.n {
            return Err(bad("block clock slab shape/clock"));
        }
        let mut next = *self;
        let mut physical = 0_f64;
        let mut endpoint_physical = 0_f64;
        let mut max_weight = 1_f64;
        let mut min_p = f64::INFINITY;
        let mut seam_max = 0_f64;
        let mut res_max = 0_f64;
        for i in (0..self.n).step_by(2) {
            let j = i / 2;
            let c = self.clocks[j];
            let u = ScratchPoly::new(&y[i])?;
            let q = ScratchPoly::new(&y[i + 1])?;
            let pp = if self.coupled {
                u.add(q.scale(p(-1.)?)?.slice())?
            } else {
                u
            };
            let m = c.measure(self.start, times[1], super::PolynomialClockRoute::Factored)?;
            let p0 = self.initial[i];
            let exact_lower = p0
                .div(p(1.)?.add(super::triangular_goal::positive(p0.mul(m)?)?)?)?
                .lo
                .max(0.);
            let approx_lower = range(pp.slice())?.lo;
            let lower = exact_lower.min(approx_lower);
            min_p = min_p.min(lower);
            let a = ClockPolynomial::normalized(c.a, times[0], h)?;
            let b = ClockPolynomial::normalized(c.b, times[0], h)?;
            let rp = derivative(&pp)?.add(a.mul(pp.slice())?.mul(pp.slice())?.scale(h)?.slice())?;
            let rq = derivative(&q)?.add(b.mul(pp.slice())?.scale(h.scale(-1.)?)?.slice())?;
            let sp = self.previous[i].sub(pp.a[0])?.mag();
            let sq = self.previous[i + 1].sub(q.a[0])?.mag();
            if plan.decay() {
                let beta = rodas5p_core::directed::add_down(exact_lower, approx_lower)?;
                if beta < 0. {
                    return Err(bad("block decay dissipativity not proved"));
                }
                let cells = match plan {
                    ClockTubePlan::BlockSplitDecay1 => 1,
                    ClockTubePlan::BlockSplitDecay2 | ClockTubePlan::BlockSplitRational2 => 2,
                    _ => 4,
                };
                let width = 1. / cells as f64;
                let mut ep = add_up(self.radius[j], sp)?;
                let mut eq = add_up(self.q_radius[j], sq)?;
                let mut peak = if self.coupled {
                    add_up(ep, eq)?
                } else {
                    ep.max(eq)
                };
                let mut residual = 0.;
                for k in 0..cells {
                    let left = k as f64 * width;
                    let pr = if cells == 1 {
                        bound(&rp, plan)?
                    } else {
                        mul_up(width, bound(&restrict_poly(&rp, left, width)?, plan)?)?
                    };
                    let qr = if cells == 1 {
                        bound(&rq, plan)?
                    } else {
                        mul_up(width, bound(&restrict_poly(&rq, left, width)?, plan)?)?
                    };
                    let mass = if cells == 1 {
                        c.measure(times[0], times[1], super::PolynomialClockRoute::Factored)?
                    } else {
                        subcell_integral(&a, left, width, h)?
                    };
                    let (decay, phi) = decay_factors(
                        beta,
                        mass,
                        matches!(
                            plan,
                            ClockTubePlan::BlockSplitRational4 | ClockTubePlan::BlockSplitRational2
                        ),
                    )?;
                    let epeak = add_up(ep, pr)?;
                    let drift = mul_up(mul_up(c.ratio.mag(), phi)?, epeak)?;
                    eq = add_up(add_up(eq, qr)?, drift)?;
                    peak = peak.max(if self.coupled {
                        add_up(epeak, eq)?
                    } else {
                        epeak.max(eq)
                    });
                    ep = epeak.min(add_up(mul_up(decay, ep)?, pr)?);
                    residual = add_up(residual, add_up(add_up(pr, qr)?, drift)?)?;
                }
                next.radius[j] = ep;
                next.q_radius[j] = eq;
                endpoint_physical = endpoint_physical.max(if self.coupled {
                    add_up(ep, eq)?
                } else {
                    ep.max(eq)
                });
                physical = physical.max(peak);
                seam_max = seam_max.max(add_up(sp, sq)?);
                res_max = res_max.max(residual);
                next.previous[i] = eval(pp.slice(), p(1.)?)?;
                next.previous[i + 1] = eval(q.slice(), p(1.)?)?;
                continue;
            }
            let rp = bound(&rp, plan)?;
            let rq = bound(&rq, plan)?;
            if plan == ClockTubePlan::BlockSplit {
                // e_p'=-P(p_exact+p_hat)e_p-r_p. Its scalar log norm
                // is nonpositive if both trajectories' sum is nonnegative.
                if rodas5p_core::directed::add_down(exact_lower, approx_lower)? < 0. {
                    return Err(bad("block split dissipativity not proved"));
                }
                let ep = add_up(add_up(self.radius[j], sp)?, rp)?;
                let measure =
                    c.measure(times[0], times[1], super::PolynomialClockRoute::Factored)?;
                let drift = mul_up(mul_up(c.ratio.mag(), measure.hi)?, ep)?;
                let eq = add_up(add_up(add_up(self.q_radius[j], sq)?, rq)?, drift)?;
                next.radius[j] = ep;
                next.q_radius[j] = eq;
                physical = physical.max(if self.coupled {
                    add_up(ep, eq)?
                } else {
                    ep.max(eq)
                });
                seam_max = seam_max.max(add_up(sp, sq)?);
                res_max = res_max.max(add_up(add_up(rp, rq)?, drift)?);
            } else {
                if lower < 0. || lower == 0. && c.ratio.mag() > 0. {
                    return Err(bad("block adaptive positive lower bound"));
                }
                let w = if c.ratio.mag() == 0. {
                    1.
                } else {
                    div_up(div_up(c.ratio.mag(), lower)?, 2.)?.max(1.)
                };
                let converted = mul_up(self.radius[j], div_up(w, self.weights[j])?.max(1.))?;
                let seam = add_up(mul_up(w, sp)?, sq)?;
                let res = add_up(mul_up(w, rp)?, rq)?;
                let r = add_up(add_up(converted, seam)?, res)?;
                next.radius[j] = r;
                next.weights[j] = w;
                physical = physical.max(r);
                max_weight = max_weight.max(w);
                seam_max = seam_max.max(seam);
                res_max = res_max.max(res);
            }
            next.previous[i] = eval(pp.slice(), p(1.)?)?;
            next.previous[i + 1] = eval(q.slice(), p(1.)?)?;
        }
        if !plan.decay() {
            endpoint_physical = physical;
        }
        let ez = add_up(out.algebraic_defect, mul_up(out.zx_norm_upper, physical)?)?;
        let qualified = physical < tube;
        out.incoming = incoming;
        out.outgoing = Some(endpoint_physical);
        out.whole_x_error = Some(physical);
        out.whole_z_error = Some(ez);
        out.green_used = false;
        out.global_qualified = qualified;
        out.global_reason =
            (!qualified).then(|| "block clock strict bootstrap tube exceeded".into());
        if qualified {
            *self = next;
        }
        Ok(ClockTubeWork {
            plan,
            weight: max_weight,
            p_lower: min_p,
            seam: seam_max,
            residual_integral_upper: res_max,
            incoming,
            outgoing: endpoint_physical,
        })
    }
}
/// Single-epoch, source-bound carry. The constructor accepts only the
/// zero-incoming physical epoch. Caller supplies its authoritative initial x.
pub struct ClockTubeCarry<'a> {
    src: &'a DaeSource,
    plan: ClockTubePlan,
    coupled: bool,
    polynomial: Option<ClockPolynomial>,
    block: Option<BlockCarry>,
    t: f64,
    previous: [Iv; 2],
    radius: f64,
    valid: bool,
}
impl<'a> ClockTubeCarry<'a> {
    pub fn new(
        src: &'a DaeSource,
        plan: ClockTubePlan,
        start: f64,
        x: &[f64],
        incoming: Option<f64>,
    ) -> CoreResult<Self> {
        if plan.block() {
            if incoming != Some(0.) {
                return Err(bad("block clock requires zero incoming"));
            }
            let block = BlockCarry::new(src, start, x)?;
            return Ok(Self {
                src,
                plan,
                coupled: block.coupled,
                polynomial: None,
                block: Some(block),
                t: start,
                previous: [zero(); 2],
                radius: 0.,
                valid: true,
            });
        }
        let polynomial_plan = matches!(
            plan,
            ClockTubePlan::PolynomialMean4
                | ClockTubePlan::PolynomialMean8
                | ClockTubePlan::PolynomialHybrid4
        );
        let (coupled, polynomial) = if polynomial_plan {
            if src.nx() != 2
                || src.nz() != 2
                || !matches_terms(&src.g[0], &[(1., &[3]), (-1., &[1])])
                || !matches_terms(&src.g[1], &[(1., &[4]), (-1., &[2])])
            {
                return Err(bad("polynomial weighted graph"));
            }
            if source_kind(src) == Some(true) {
                (true, None)
            } else {
                (false, Some(ClockPolynomial::parse(src, 0)?))
            }
        } else {
            (
                source_kind(src).ok_or_else(|| bad("weighted clock source unsupported"))?,
                None,
            )
        };
        if !start.is_finite() || start < 0. || x.len() != 2 || incoming != Some(0.) {
            return Err(bad(
                "weighted clock requires zero incoming nonnegative epoch",
            ));
        }
        Ok(Self {
            src,
            plan,
            coupled,
            polynomial,
            block: None,
            t: start,
            previous: [p(x[0])?, p(x[1])?],
            radius: 0.,
            valid: true,
        })
    }
    pub fn invalidate(&mut self) {
        self.valid = false;
    }
    /// Full local source proof then outward weighted propagation. Refusal
    /// poisons the carry. The report uses bounds in original physical units.
    #[allow(clippy::too_many_arguments)]
    pub fn certify_into(
        &mut self,
        times: [f64; 2],
        y: &[Poly],
        branch: &[[f64; 2]],
        tube: f64,
        green: bool,
        condition_cap: f64,
        route: SlabWorkspaceRoute,
        scratch: &mut SlabCertificateWorkspace,
        out: &mut SlabCertificate,
    ) -> CoreResult<ClockTubeWork> {
        // A refused call must never leave a stale qualified report.
        out.global_qualified = false;
        out.outgoing = None;
        out.whole_x_error = None;
        out.whole_z_error = None;
        if !self.valid {
            return Err(bad("weighted clock carry invalid"));
        }
        self.valid = false; // no retry/restart of a failed trajectory
        if times[0] != self.t || times[0] < 0. {
            return Err(bad("weighted clock seam/clock"));
        }
        certify_slab_workspace_into(
            self.src,
            times,
            y,
            branch,
            tube,
            Some(0.),
            green,
            condition_cap,
            route,
            scratch,
            out,
        )?;
        // The local zero-incoming proof establishes the branch, not this
        // epoch's global authority. Clear it before any fallible weighted work.
        out.global_qualified = false;
        out.outgoing = None;
        out.whole_x_error = None;
        out.whole_z_error = None;
        if let Some(block) = self.block.as_mut() {
            let work = block.step(self.plan, times, y, tube, self.radius, out)?;
            if out.global_qualified {
                self.radius = work.outgoing;
                self.t = times[1];
                self.valid = true;
            }
            return Ok(work);
        }
        let w = self.plan.weight();
        let h = p(times[1])?.sub(p(times[0])?)?;
        let u = ScratchPoly::new(&y[0])?;
        let v = ScratchPoly::new(&y[1])?;
        let pp = if self.coupled {
            u.add(v.scale(p(-1.)?)?.slice())?
        } else {
            u
        };
        // The full original infinity tube implies |delta p|<=2*tube
        // in the coupled coordinates; includes the approximate trajectory.
        let lower = sub_down(
            range(pp.slice())?.lo,
            mul_up(if self.coupled { 2. } else { 1. }, tube)?,
        )?;
        let required = if let Some(poly) = self.polynomial {
            div_up(poly.ratio.mag(), 2. * w)?
        } else {
            1. / (2. * w)
        };
        if lower < required {
            return Err(bad("weighted clock positivity/contraction not proved"));
        }
        let (rp, rq) = if let Some(poly) = self.polynomial {
            let a = ClockPolynomial::normalized(poly.a, times[0], h)?;
            let b = ClockPolynomial::normalized(poly.b, times[0], h)?;
            (
                derivative(&pp)?.add(a.mul(pp.slice())?.mul(pp.slice())?.scale(h)?.slice())?,
                derivative(&v)?.add(b.mul(pp.slice())?.scale(h.scale(-1.)?)?.slice())?,
            )
        } else {
            let clock = ScratchPoly::new(&[p(times[0])?, h])?;
            let tp = clock.mul(pp.slice())?;
            (
                derivative(&pp)?.add(tp.mul(pp.slice())?.scale(h)?.slice())?,
                derivative(&v)?.add(tp.scale(-h)?.slice())?,
            )
        };
        let res = add_up(mul_up(w, bound(&rp, self.plan)?)?, bound(&rq, self.plan)?)?;
        let seam = transform_norm(
            self.previous[0].sub(u.a[0])?,
            self.previous[1].sub(v.a[0])?,
            self.coupled,
            w,
        )?;
        let incoming = self.radius;
        let outgoing = add_up(add_up(incoming, seam)?, res)?;
        let ez = add_up(out.algebraic_defect, mul_up(out.zx_norm_upper, outgoing)?)?;
        let end = [eval(u.slice(), p(1.)?)?, eval(v.slice(), p(1.)?)?];
        // Strict old bootstrap radius; no physical tolerance widening.
        let qualified = outgoing < tube;
        out.incoming = incoming;
        out.outgoing = Some(outgoing);
        out.whole_x_error = Some(outgoing);
        out.whole_z_error = Some(ez);
        // Keep original local mu/defect diagnostics, do not relabel them.
        out.green_used = false;
        out.global_qualified = qualified;
        out.global_reason =
            (!qualified).then(|| "weighted clock physical bootstrap tube exceeded".into());
        if qualified {
            self.t = times[1];
            self.previous = end;
            self.radius = outgoing;
            self.valid = true;
        }
        Ok(ClockTubeWork {
            plan: self.plan,
            weight: w,
            p_lower: lower,
            seam,
            residual_integral_upper: res,
            incoming,
            outgoing,
        })
    }
}
