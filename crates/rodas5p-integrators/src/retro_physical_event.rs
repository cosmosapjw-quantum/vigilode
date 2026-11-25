//! A posteriori physical event enclosures for a source-owned affine IVP.
//! Exact represented source, initial state and native dense packet are the
//! authority. A represented polynomial root alone never authorizes a claim.
//! This research path integrates the full IVP, then certifies its first root.
use crate::{
    AdaptiveStepConfig, FastDenseSegments, FastInteriorControl, OdeProblem,
    affine_event_certificate::{
        self as ac, AffineEventBinding, AffineEventCarry, AffineEventCertificate, AffineEventSpec,
        AffineScalarModel,
    },
    integrate_rodas5p_fast_dense_observed,
    retro_causal::{self, SlabSpec},
    retro_event_poly::{DeclaredPolynomial, EventTerm, RepresentedPolynomial, RootKind, RootRoute},
    retro_foundation::nilpotent_green,
    retro_green::RetroTable,
    retro_portfolio::{Portfolio, Route, Selection},
};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, WorkCounters,
    directed::{Interval, add_up, mul_up, sub_up},
    retro_dictionary::{Dictionary, RealMode},
};
use serde::Serialize;
use std::sync::Arc;
type Iv = Interval;
fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("physical event: {s}"))
}
fn pt(x: f64) -> CoreResult<Iv> {
    Iv::point(x)
}
fn z() -> Iv {
    Iv { lo: 0., hi: 0. }
}
fn ball(r: f64) -> CoreResult<Iv> {
    Iv::new(-r, r)
}
fn sign(v: Iv) -> i8 {
    if v.lo > 0. {
        1
    } else if v.hi < 0. {
        -1
    } else {
        0
    }
}
fn intersection(a: Iv, b: Iv) -> CoreResult<Iv> {
    Iv::new(a.lo.max(b.lo), a.hi.min(b.hi))
}
/// One physical interval-Newton contraction. The caller owns the first-root
/// existence proof and a derivative enclosure on the complete bracket.
pub(crate) fn contract_root_once(
    lo: f64,
    hi: f64,
    x: f64,
    fx: Iv,
    d: Iv,
    left: i8,
    right: i8,
) -> CoreResult<[f64; 2]> {
    let mut nx = Iv::new(lo, hi)?;
    if !d.contains_zero() {
        nx = intersection(nx, pt(x)?.sub(fx.div(d)?)?)?;
    }
    if sign(fx) == left {
        nx.lo = nx.lo.max(x);
    } else if sign(fx) == right {
        nx.hi = nx.hi.min(x);
    }
    Iv::new(nx.lo, nx.hi).map(Iv::as_pair)
}
fn hull(b: &[Iv]) -> CoreResult<Iv> {
    Iv::new(
        b.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
        b.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max),
    )
}
fn horner(p: &[Iv; 5], x: Iv) -> CoreResult<Iv> {
    p.iter().rev().try_fold(z(), |a, &b| a.mul(x)?.add(b))
}
fn exact_product(a: f64, b: f64) -> CoreResult<f64> {
    let x = pt(a)?.scale(b)?;
    if x.lo != x.hi {
        return Err(bad("source action scaling not exact"));
    }
    Ok(x.lo)
}
/// Immutable linear source. No external RHS callback is admitted.
#[derive(Clone, Debug)]
pub struct AffineSource {
    a: DenseMatrix,
    b: Vec<f64>,
    rate: f64,
}
impl AffineSource {
    pub fn new(a: DenseMatrix, b: Vec<f64>) -> CoreResult<Self> {
        ac::check_affine_event_arithmetic()?;
        let n = b.len();
        if !(1..=8).contains(&n)
            || a.nrows() != n
            || a.ncols() != n
            || a.as_slice().iter().chain(&b).any(|v| !v.is_finite())
        {
            return Err(bad("source shape/nonfinite"));
        }
        let mut mu = f64::NEG_INFINITY;
        for i in 0..n {
            let mut r = a[(i, i)];
            for j in 0..n {
                if i != j {
                    r = add_up(r, a[(i, j)].abs())?;
                }
            }
            mu = mu.max(r);
        }
        if mu > 0. {
            return Err(bad("nonpositive infinity logarithmic norm not proved"));
        }
        Ok(Self { a, b, rate: -mu })
    }
    pub fn rate_lower(&self) -> f64 {
        self.rate
    }
    pub fn dimension(&self) -> usize {
        self.b.len()
    }
    fn flow_box(&self, y: &[Iv]) -> CoreResult<Vec<Iv>> {
        (0..self.dimension())
            .map(|i| {
                self.a
                    .row(i)
                    .iter()
                    .zip(y)
                    .try_fold(pt(self.b[i])?, |s, (&a, &v)| s.add(v.scale(a)?))
            })
            .collect()
    }
    fn scalar_spec(&self) -> CoreResult<AffineEventSpec> {
        if self.dimension() != 1 || self.rate <= 0. {
            return Err(bad("scalar affine certificate admission"));
        }
        AffineEventSpec::new(
            AffineEventBinding::new(
                "physical-affine-source",
                "owned-represented-A-b",
                ac::AFFINE_EVENT_CLOCK_ID,
                "caller-clock-state",
                "physical-event-proof",
                0,
            )?,
            AffineScalarModel::new(pt(self.rate)?, pt(self.b[0])?)?,
            0.,
        )
    }
    fn source_action_admitted(&self) -> bool {
        let n = self.dimension();
        (n == 1 && self.rate > 0.)
            || (n <= 3
                && n > 1
                && self.a[(0, 0)] <= 0.
                && (0..n).all(|i| {
                    self.a[(i, i)] == self.a[(0, 0)] && (0..i).all(|j| self.a[(i, j)] == 0.)
                }))
    }
    /// Integrate the original declared source, with actual native dense
    /// packets. Physical certificate is independent of adaptive LTE.
    pub fn integrate(
        &self,
        y0: &[f64],
        end: f64,
        config: &AdaptiveStepConfig,
    ) -> CoreResult<PhysicalPath> {
        ac::check_affine_event_arithmetic()?;
        if y0.len() != self.dimension()
            || y0.iter().any(|v| !v.is_finite())
            || !end.is_finite()
            || end <= 0.
        {
            return Err(bad("initial state or clock"));
        }
        let n = self.dimension();
        let a = self.a.clone();
        let b = self.b.clone();
        let jac = a.clone();
        let ode = OdeProblem::new(
            "physical-owned-affine",
            n,
            Arc::new(move |_, y, f| {
                for (i, v) in f.iter_mut().enumerate() {
                    *v = a
                        .row(i)
                        .iter()
                        .zip(y)
                        .fold(b[i], |s, (&a, &y)| a.mul_add(y, s));
                }
                Ok(())
            }),
            None,
            Some(Arc::new(move |_, _| Ok(jac.clone()))),
            None,
            None,
            true,
            None,
            None,
        )?;
        let r = integrate_rodas5p_fast_dense_observed(
            &ode,
            (0., end),
            y0,
            config,
            FastInteriorControl::Off,
        )?;
        if !r.fast.observed.success || r.segments.times().last() != Some(&end) {
            return Err(bad("incomplete native trajectory"));
        }
        Ok(PhysicalPath {
            source: self.clone(),
            initial: y0.to_vec(),
            segments: r.segments,
            counters: r.fast.observed.counters,
        })
    }
}
/// Path with immutable source and initial value, not caller-injected bounds.
pub struct PhysicalPath {
    source: AffineSource,
    initial: Vec<f64>,
    segments: FastDenseSegments,
    counters: WorkCounters,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PhysicalRoute {
    Unity,
    AffineCarry,
    Green,
    Weighted,
    WeightedHint,
    SourceAction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PhysicalStatus {
    NoRoot,
    Unique,
    Unresolved,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct PhysicalWork {
    pub slabs: usize,
    pub affine_carries: usize,
    pub green_queries: usize,
    pub dictionary_moments: usize,
    pub source_flow_queries: usize,
    pub nilpotent_queries: usize,
    pub source_action_fallbacks: usize,
    pub represented_hints: usize,
    pub event_queries: usize,
    pub newton_contracts: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct SlabProof {
    pub times: [f64; 2],
    pub incoming: f64,
    pub outgoing: f64,
    pub whole_radius: f64,
    pub defect_bernstein: [f64; 5],
    pub state_range: Vec<[f64; 2]>,
    pub event_range: [f64; 2],
    pub derivative: [f64; 2],
    pub event_gap_upper: f64,
    pub endpoint_signs: [i8; 2],
    pub disposition: PhysicalStatus,
}
#[derive(Clone, Debug, Serialize)]
pub struct PhysicalReport {
    pub route: PhysicalRoute,
    pub status: PhysicalStatus,
    pub qualified: bool,
    pub bracket: Option<[f64; 2]>,
    pub width: Option<f64>,
    pub kind: Option<RootKind>,
    pub structural_multiplicity: Option<usize>,
    pub reason: Option<String>,
    pub work: PhysicalWork,
    pub proofs: Vec<SlabProof>,
    pub native_counters: WorkCounters,
}
fn power(packet: &ac::RepresentedAffineSegment) -> CoreResult<[Iv; 5]> {
    let (h, u) = packet.exact_inputs();
    let mut d = [z(); 3];
    for i in 0..3 {
        for j in 0..8 {
            d[i] = d[i].add(pt(h[i][j])?.scale(u[j])?)?;
        }
    }
    let [a, b] = packet.states();
    let a = pt(a)?;
    let b = pt(b)?;
    Ok([
        a,
        b.sub(a)?.add(d[0])?,
        d[1].sub(d[0])?,
        d[2].sub(d[1])?,
        -d[2],
    ])
}
struct Slab {
    times: [f64; 2],
    h: Iv,
    p: Vec<[Iv; 5]>,
    defect: [f64; 5],
    incoming: f64,
    outgoing: f64,
    whole: f64,
}
fn prefix_coeffs(c: [f64; 5], theta: Iv) -> CoreResult<[Iv; 5]> {
    let mut a = c.map(|x| Iv { lo: x, hi: x });
    let mut v = [z(); 5];
    v[0] = a[0];
    let complement = pt(1.)?.sub(theta)?;
    for (i, value) in v.iter_mut().enumerate().skip(1) {
        for j in 0..5 - i {
            a[j] = a[j].mul(complement)?.add(a[j + 1].mul(theta)?)?;
        }
        *value = a[0];
    }
    Ok(v)
}
// Keep the normalized clock, transport rate, incoming radius and residual
// weights explicit at this proof boundary.
#[allow(clippy::too_many_arguments)]
fn envelope(
    table: &RetroTable,
    rate: f64,
    h: Iv,
    theta: Iv,
    incoming: f64,
    defect: [f64; 5],
    route: PhysicalRoute,
    w: &mut PhysicalWork,
) -> CoreResult<f64> {
    let delta = defect.iter().copied().fold(0., f64::max);
    if route == PhysicalRoute::Unity {
        return add_up(incoming, mul_up(theta.hi, delta)?);
    }
    let t = h.mul(theta)?;
    let x = t.scale(rate)?;
    let x = Iv::new(x.lo.max(0.), x.hi)?;
    w.green_queries += 1;
    let gain = retro_causal::gain(table, rate, t)?;
    let force = if matches!(
        route,
        PhysicalRoute::Weighted | PhysicalRoute::WeightedHint | PhysicalRoute::SourceAction
    ) {
        let d = Dictionary::get()?;
        let coeff = prefix_coeffs(defect, theta)?;
        let mut s = z();
        for (k, c) in coeff.iter().enumerate() {
            w.dictionary_moments += 1;
            s = s.add(d.moment(x, 4, k, RealMode::Hybrid)?.scale(c.mag())?)?;
        }
        theta.mul(s)?.hi
    } else {
        theta.mul(table.phi(x, 1)?.scale(delta)?)?.hi
    };
    add_up(mul_up(gain, incoming)?, force)
}
impl Slab {
    fn new(
        path: &PhysicalPath,
        k: usize,
        incoming: f64,
        route: PhysicalRoute,
        table: &RetroTable,
        carry: &mut Option<AffineEventCarry>,
        w: &mut PhysicalWork,
    ) -> CoreResult<Self> {
        let n = path.source.dimension();
        let times = [path.segments.t_old(k), path.segments.t_new(k)];
        let h = pt(times[1])?.sub(pt(times[0])?)?;
        if h.lo <= 0. {
            return Err(bad("clock"));
        }
        let p = (0..n)
            .map(|i| power(&path.segments.represented_component(k, i)?))
            .collect::<CoreResult<Vec<_>>>()?;
        let mut defect = [0.0_f64; 5];
        for i in 0..n {
            let mut r = [z(); 5];
            for j in 0..5 {
                let mut a = if j == 0 { pt(path.source.b[i])? } else { z() };
                for (l, c) in p.iter().enumerate() {
                    a = a.add(c[j].scale(path.source.a[(i, l)])?)?;
                }
                r[j] = -h.mul(a)?;
                if j < 4 {
                    r[j] = r[j].add(p[i][j + 1].scale((j + 1) as f64)?)?;
                }
            }
            for (j, v) in ac::bernstein(&r)?.iter().enumerate() {
                defect[j] = defect[j].max(v.mag());
            }
        }
        let (outgoing, whole) = if route == PhysicalRoute::AffineCarry {
            let spec = path.source.scalar_spec()?;
            let packet = path.segments.represented_component(k, 0)?;
            let c = if let Some(prev) = carry.as_ref() {
                prev.certify_next(packet, &spec)?
            } else {
                AffineEventCertificate::certify(packet, &spec, incoming)?
            };
            w.affine_carries += 1;
            *carry = Some(c.carry()?);
            (c.endpoint_error_upper(), c.whole_slab_error_upper())
        } else {
            // Uniform comparison solution is monotone, so its maximum is
            // attained at an endpoint. A weighted endpoint alone cannot
            // bound the full interior: use the uniform envelope separately.
            let coarse = envelope(
                table,
                path.source.rate,
                h,
                pt(1.)?,
                incoming,
                defect,
                if route == PhysicalRoute::Unity {
                    PhysicalRoute::Unity
                } else {
                    PhysicalRoute::Green
                },
                w,
            )?;
            let out = if matches!(
                route,
                PhysicalRoute::Weighted | PhysicalRoute::WeightedHint | PhysicalRoute::SourceAction
            ) {
                envelope(
                    table,
                    path.source.rate,
                    h,
                    pt(1.)?,
                    incoming,
                    defect,
                    route,
                    w,
                )?
                .min(coarse)
            } else {
                coarse
            };
            (out, incoming.max(coarse))
        };
        Ok(Self {
            times,
            h,
            p,
            defect,
            incoming,
            outgoing,
            whole,
        })
    }
    fn theta(&self, t: Iv) -> CoreResult<Iv> {
        let v = t.sub(pt(self.times[0])?)?.div(self.h)?;
        Iv::new(v.lo.max(0.), v.hi.min(1.))
    }
    fn polynomial(&self, x: Iv) -> CoreResult<Vec<Iv>> {
        if x.lo == 0. && x.hi == 1. {
            return self.p.iter().map(|p| hull(&ac::bernstein(p)?)).collect();
        }
        self.p.iter().map(|p| horner(p, x)).collect()
    }
    fn boxes(
        &self,
        path: &PhysicalPath,
        t: Iv,
        route: PhysicalRoute,
        table: &RetroTable,
        w: &mut PhysicalWork,
    ) -> CoreResult<(Vec<Iv>, Vec<Iv>, f64)> {
        let theta = self.theta(t)?;
        let u = self.polynomial(theta)?;
        let mut e = self.whole;
        if t.lo == t.hi {
            if t.lo == self.times[0] {
                e = self.incoming;
            } else if t.lo == self.times[1] {
                e = self.outgoing;
            } else if route != PhysicalRoute::AffineCarry {
                e = e.min(envelope(
                    table,
                    path.source.rate,
                    self.h,
                    theta,
                    self.incoming,
                    self.defect,
                    route,
                    w,
                )?);
            }
        }
        let mut y = u
            .iter()
            .map(|v| v.add(ball(e)?))
            .collect::<CoreResult<Vec<_>>>()?;
        if route == PhysicalRoute::SourceAction && t.lo == t.hi {
            match path.source_action(t.lo, table, w) {
                Ok(v) => {
                    for (a, b) in y.iter_mut().zip(v) {
                        *a = intersection(*a, b)?;
                    }
                }
                Err(_) => {
                    w.source_action_fallbacks += 1;
                }
            }
        }
        Ok((u, y, e))
    }
}
struct Effective {
    poly: DeclaredPolynomial,
    multiplicity: Option<usize>,
    orientation: f64,
}
fn effective(spec: &DeclaredPolynomial) -> CoreResult<Effective> {
    // Exact structural reduction, not numerical factorization. A nonzero
    // constant times an exact repeated affine factor has the same zero set.
    let ts = spec.terms();
    if ts.len() == 1 && ts[0].coefficient != 0. && !ts[0].factors.is_empty() {
        let f = &ts[0].factors[0];
        if ts[0]
            .factors
            .iter()
            .all(|g| g.constant == f.constant && g.time == f.time && g.state == f.state)
        {
            return Ok(Effective {
                poly: DeclaredPolynomial::new(vec![EventTerm {
                    coefficient: 1.,
                    factors: vec![f.clone()],
                }])?,
                multiplicity: Some(ts[0].factors.len()),
                orientation: ts[0].coefficient.signum(),
            });
        }
    }
    Ok(Effective {
        poly: spec.clone(),
        multiplicity: None,
        orientation: 1.,
    })
}
/// Evaluate g and its gradient / total physical derivative with product rule.
fn event(poly: &DeclaredPolynomial, t: Iv, y: &[Iv], flow: &[Iv]) -> CoreResult<(Iv, Iv, f64)> {
    let mut g = z();
    let mut dg = z();
    let mut grad = vec![z(); y.len()];
    for term in poly.terms() {
        let mut vals = Vec::new();
        let mut deriv = Vec::new();
        for f in &term.factors {
            let mut v = pt(f.constant)?.add(t.scale(f.time)?)?;
            let mut d = pt(f.time)?;
            for &(i, a) in &f.state {
                v = v.add(y[i].scale(a)?)?;
                d = d.add(flow[i].scale(a)?)?;
            }
            vals.push(v);
            deriv.push(d);
        }
        let c = pt(term.coefficient)?;
        let product = vals.iter().try_fold(c, |s, &v| s.mul(v))?;
        g = g.add(product)?;
        for (i, f) in term.factors.iter().enumerate() {
            let rest = vals
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .try_fold(c, |s, (_, &v)| s.mul(v))?;
            dg = dg.add(rest.mul(deriv[i])?)?;
            for &(j, a) in &f.state {
                grad[j] = grad[j].add(rest.scale(a)?)?;
            }
        }
    }
    let l = grad.iter().try_fold(0., |s, v| add_up(s, v.mag()))?;
    Ok((g, dg, l))
}
impl PhysicalPath {
    /// Read-only original accepted packets for independent evidence replay.
    pub fn segments(&self) -> &FastDenseSegments {
        &self.segments
    }
    pub fn native_steps(&self) -> usize {
        self.segments.len()
    }
    pub fn native_counters(&self) -> WorkCounters {
        self.counters
    }
    fn source_action(
        &self,
        t: f64,
        table: &RetroTable,
        w: &mut PhysicalWork,
    ) -> CoreResult<Vec<Iv>> {
        let n = self.source.dimension();
        if n == 1 && self.source.rate > 0. {
            w.source_flow_queries += 1;
            let s = SlabSpec {
                end: t,
                rate: self.source.rate,
                forcing: self.source.b[0],
                forecast: self.source.rate,
                radius: 0.,
            };
            return Ok(vec![retro_causal::flow(table, s, 0., t, self.initial[0])?]);
        }
        if !self.source.source_action_admitted() {
            return Err(bad("nilpotent source action admission"));
        }
        let mut e = DenseMatrix::zeros(n, n);
        for i in 0..n {
            for j in i + 1..n {
                e[(i, j)] = exact_product(t, self.source.a[(i, j)])?;
            }
        }
        let a = exact_product(t, -self.source.a[(0, 0)])?;
        let f = self
            .source
            .b
            .iter()
            .map(|&b| exact_product(t, b))
            .collect::<CoreResult<Vec<_>>>()?;
        w.nilpotent_queries += 1;
        nilpotent_green(
            a,
            &e,
            1.,
            &self.initial,
            &[f.clone(), f.clone(), f],
            RealMode::Hybrid,
        )
    }
    /// Certification with all original mathematical premises built from the
    /// owned source and path. "Unique" covers earliest root, not later roots.
    pub fn certify(
        &self,
        spec: &DeclaredPolynomial,
        route: PhysicalRoute,
        time_tol: f64,
    ) -> CoreResult<PhysicalReport> {
        ac::check_affine_event_arithmetic()?;
        spec.validate(self.source.dimension())?;
        if !time_tol.is_finite() || time_tol <= 0. {
            return Err(bad("time tolerance"));
        }
        if route == PhysicalRoute::AffineCarry {
            self.source.scalar_spec()?;
        }
        if route == PhysicalRoute::SourceAction && !self.source.source_action_admitted() {
            return Err(bad("source action unavailable"));
        }
        let eff = effective(spec)?;
        let table = if matches!(
            route,
            PhysicalRoute::Weighted | PhysicalRoute::WeightedHint | PhysicalRoute::SourceAction
        ) {
            RetroTable::with_dictionary(RealMode::Hybrid)?
        } else {
            RetroTable::new()?
        };
        let mut r = PhysicalReport {
            route,
            status: PhysicalStatus::Unresolved,
            qualified: false,
            bracket: None,
            width: None,
            kind: None,
            structural_multiplicity: eff.multiplicity,
            reason: None,
            work: PhysicalWork::default(),
            proofs: vec![],
            native_counters: self.counters,
        };
        let mut incoming = 0.;
        let mut carry = None;
        for k in 0..self.segments.len() {
            let s = Slab::new(self, k, incoming, route, &table, &mut carry, &mut r.work)?;
            incoming = s.outgoing;
            r.work.slabs += 1;
            let [a, b] = s.times;
            let eval = |t: Iv, w: &mut PhysicalWork| -> CoreResult<(Iv, Iv, f64, Vec<Iv>)> {
                w.event_queries += 1;
                let (u, y, e) = s.boxes(self, t, route, &table, w)?;
                let (v, d, _) = event(&eff.poly, t, &y, &self.source.flow_box(&y)?)?;
                let center = event(&eff.poly, t, &u, &vec![z(); u.len()])?.0;
                // Source-action intersection can exclude U. The mean-value
                // gradient must enclose the entire line from U to true y.
                let tube = u
                    .iter()
                    .zip(&y)
                    .map(|(u, y)| Iv::new(u.lo.min(y.lo), u.hi.max(y.hi)))
                    .collect::<CoreResult<Vec<_>>>()?;
                let (_, _, l) = event(&eff.poly, t, &tube, &vec![z(); u.len()])?;
                let gap = mul_up(l, e)?;
                Ok((intersection(v, center.add(ball(gap)?)?)?, d, gap, y))
            };
            let (g, d, gap, y) = eval(Iv::new(a, b)?, &mut r.work)?;
            let ga = eval(pt(a)?, &mut r.work)?.0;
            let gb = eval(pt(b)?, &mut r.work)?.0;
            let signs = [sign(ga), sign(gb)];
            let monotone = sign(d) != 0;
            let disposition = if sign(g) != 0 || (monotone && signs[0] != 0 && signs[0] == signs[1])
            {
                PhysicalStatus::NoRoot
            } else if monotone && signs[0] * signs[1] == -1 {
                PhysicalStatus::Unique
            } else {
                PhysicalStatus::Unresolved
            };
            r.proofs.push(SlabProof {
                times: s.times,
                incoming: s.incoming,
                outgoing: s.outgoing,
                whole_radius: s.whole,
                defect_bernstein: s.defect,
                state_range: y.iter().map(|v| v.as_pair()).collect(),
                event_range: g.as_pair(),
                derivative: d.as_pair(),
                event_gap_upper: gap,
                endpoint_signs: signs,
                disposition,
            });
            if disposition == PhysicalStatus::NoRoot {
                continue;
            }
            if disposition == PhysicalStatus::Unresolved {
                r.reason =
                    Some("cannot exclude earlier root or prove a transverse sign bracket".into());
                return Ok(r);
            }
            // At this point existence AND uniqueness are proved separately.
            // Optional represented root only seeds interval Newton.
            let mut lo = a;
            let mut hi = b;
            let mut hint = None;
            if route == PhysicalRoute::WeightedHint
                && let Ok(p) = RepresentedPolynomial::from_segment(&self.segments, k, &eff.poly)
                && let Ok(v) = p.isolate(RootRoute::DictionaryCompanion, 2f64.powi(-40))
                && let Some(v) = v.roots.first()
            {
                let x = v.time[0] * 0.5 + v.time[1] * 0.5;
                if lo < x && x < hi {
                    hint = Some(x);
                    r.work.represented_hints += 1;
                }
            }
            for _ in 0..80 {
                if sub_up(hi, lo)? <= time_tol {
                    break;
                }
                let x = hint.take().unwrap_or(lo + (hi - lo) * 0.5);
                if !(lo < x && x < hi) {
                    break;
                }
                let fx = eval(pt(x)?, &mut r.work)?.0;
                let old = (lo, hi);
                // MVT interval Newton with the certified derivative range
                // on the original whole slab, valid even if sign uncertain.
                [lo, hi] = contract_root_once(lo, hi, x, fx, d, signs[0], signs[1])?;
                if old == (lo, hi) {
                    break;
                }
                r.work.newton_contracts += 1;
            }
            let width = sub_up(hi, lo)?;
            let direction = if eff.multiplicity.is_some_and(|m| m % 2 == 0) {
                RootKind::Touch
            } else if (sign(d) > 0) == (eff.orientation > 0.) {
                RootKind::Up
            } else {
                RootKind::Down
            };
            r.status = PhysicalStatus::Unique;
            r.bracket = Some([lo, hi]);
            r.width = Some(width);
            r.kind = Some(direction);
            r.qualified = width <= time_tol;
            if !r.qualified {
                r.reason = Some("physical bracket wider than requested tolerance".into());
            }
            return Ok(r);
        }
        r.status = PhysicalStatus::NoRoot;
        r.qualified = true;
        Ok(r)
    }
    /// Same-contract RETRO candidate portfolio; no result cache. Only a
    /// fully certified no-root or requested-width earliest bracket admitted.
    pub fn adaptive(
        &self,
        spec: &DeclaredPolynomial,
        portfolio: &mut Portfolio,
        tol: f64,
    ) -> CoreResult<Selection<PhysicalReport>> {
        let mut routes = vec![
            Route::PhysicalWeighted,
            Route::PhysicalGreen,
            Route::PhysicalHint,
            Route::PhysicalUnity,
        ];
        if self.source.dimension() == 1 && self.source.rate > 0. {
            routes.insert(0, Route::PhysicalAffine);
        }
        if self.source.source_action_admitted() {
            routes.insert(0, Route::PhysicalSource);
        }
        portfolio.choose(
            "physical-root",
            &[
                self.source.dimension() as u64,
                self.source.rate.to_bits(),
                spec.terms().len() as u64,
            ],
            &routes,
            tol,
            |route| {
                let mode = match route {
                    Route::PhysicalUnity => PhysicalRoute::Unity,
                    Route::PhysicalAffine => PhysicalRoute::AffineCarry,
                    Route::PhysicalGreen => PhysicalRoute::Green,
                    Route::PhysicalWeighted => PhysicalRoute::Weighted,
                    Route::PhysicalHint => PhysicalRoute::WeightedHint,
                    Route::PhysicalSource => PhysicalRoute::SourceAction,
                    _ => return Err(bad("route")),
                };
                let r = self.certify(spec, mode, tol)?;
                let bound = if r.qualified {
                    r.width.unwrap_or(0.)
                } else {
                    f64::INFINITY
                };
                Ok((r, bound))
            },
        )
    }
}
