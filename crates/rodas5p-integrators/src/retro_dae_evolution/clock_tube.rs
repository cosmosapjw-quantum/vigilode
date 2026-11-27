//! Opt-in exact-source, two-dimensional clock reduction. The carry is in
//! a fixed weighted norm dominating the original coordinate infinity norm.
//! Original local branch/condition proof is mandatory on every slab.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ClockTubePlan {
    Sup4,
    Mean4,
    Mean8,
    Hybrid4,
}
impl ClockTubePlan {
    fn weight(self) -> f64 {
        if self == Self::Mean8 { 8. } else { 4. }
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
// Exact syntactic source check; unit factors only. Codes 0=time, 1..4=x0,x1,z0,z1.
// Do not replace with a numerical fit or rounded polynomial comparison.
fn matches_terms(eq: &DeclaredPolynomial, want: &[(f64, &[u8])]) -> bool {
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
            } else if f.time == 0. && f.state.len() == 1 && f.state[0].1 == 1. && f.state[0].0 < 4 {
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
fn source_kind(src: &DaeSource) -> Option<bool> {
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
    if plan == ClockTubePlan::Hybrid4 {
        // Reuse prior spectral enclosure on the *same* normalized polynomial.
        // |P_j|<=1 gives a second upper bound on its absolute integral.
        Ok(b.min(legendre_norm(&power_legendre(a.slice())?)?))
    } else {
        Ok(b)
    }
}
/// Single-epoch, source-bound carry. The constructor accepts only the
/// zero-incoming physical epoch. Caller supplies its authoritative initial x.
pub struct ClockTubeCarry<'a> {
    src: &'a DaeSource,
    plan: ClockTubePlan,
    coupled: bool,
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
        let coupled = source_kind(src).ok_or_else(|| bad("weighted clock source unsupported"))?;
        if !start.is_finite() || start < 0. || x.len() != 2 || incoming != Some(0.) {
            return Err(bad(
                "weighted clock requires zero incoming nonnegative epoch",
            ));
        }
        Ok(Self {
            src,
            plan,
            coupled,
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
        if lower < 1. / (2. * w) {
            return Err(bad("weighted clock positivity/contraction not proved"));
        }
        let clock = ScratchPoly::new(&[p(times[0])?, h])?;
        let tp = clock.mul(pp.slice())?;
        // Reduced exact graph z=x. Normalized residuals: p_u+h*t*p^2,
        // q_u-h*t*p. Their weighted absolute integral bounds each prefix.
        let rp = derivative(&pp)?.add(tp.mul(pp.slice())?.scale(h)?.slice())?;
        let rq = derivative(&v)?.add(tp.scale(-h)?.slice())?;
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
