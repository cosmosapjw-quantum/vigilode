//! Exact real roots of the represented accepted dense polynomial. This is
//! not a bound on the physical IVP, nor a generic opaque callback solver.
//! All inferences use exact binary64 inputs converted to rational numbers.
//! Numerical screening or companion eigenvalues never authorize a root.
use crate::{
    FastDenseSegments,
    affine_event_certificate::{RepresentedAffineSegment, bernstein},
    conditioned_polynomial::eval_poly_interval,
    retro_portfolio::{Portfolio, Route, Selection},
};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, ToPrimitive, Zero};
use rodas5p_core::{
    CoreError, CoreResult,
    directed::{Interval, check_binary64_environment, sub_up},
    retro_dictionary::Dictionary,
};
use serde::Serialize;
type P = Vec<Q>;
const MAX_DEG: usize = 16;
const MAX_BITS: u64 = 32768;
const MAX_NODES: usize = 16384;
const MAX_DEPTH: usize = 256;
fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("represented polynomial event: {s}"))
}
fn q(x: f64) -> CoreResult<Q> {
    Q::from_float(x).ok_or_else(|| bad("nonfinite coefficient"))
}
fn int(n: usize) -> Q {
    Q::from_integer(n.into())
}
fn zero() -> Q {
    Q::zero()
}
fn sign(x: &Q) -> i8 {
    if x.is_positive() {
        1
    } else if x.is_negative() {
        -1
    } else {
        0
    }
}
fn trim(mut p: P) -> P {
    while p.last().is_some_and(Zero::is_zero) {
        p.pop();
    }
    p
}
fn checked(p: P) -> CoreResult<P> {
    let p = trim(p);
    if p.len() > MAX_DEG + 1
        || p.iter()
            .any(|x| x.numer().bits() > MAX_BITS || x.denom().bits() > MAX_BITS)
    {
        return Err(bad("degree or rational size cap"));
    }
    Ok(p)
}
fn add(a: &[Q], b: &[Q]) -> CoreResult<P> {
    let mut v = vec![zero(); a.len().max(b.len())];
    for (i, x) in a.iter().enumerate() {
        v[i] += x;
    }
    for (i, x) in b.iter().enumerate() {
        v[i] += x;
    }
    checked(v)
}
fn scale(a: &[Q], b: &Q) -> CoreResult<P> {
    checked(a.iter().map(|x| x * b).collect())
}
fn mul(a: &[Q], b: &[Q]) -> CoreResult<P> {
    if a.is_empty() || b.is_empty() {
        return Ok(vec![]);
    }
    if a.len() + b.len() - 2 > MAX_DEG {
        return Err(bad("composed degree cap"));
    }
    let mut v = vec![zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            v[i + j] += x * y;
        }
    }
    checked(v)
}
fn eval(p: &[Q], x: &Q) -> Q {
    p.iter().rev().fold(zero(), |a, b| a * x + b)
}
fn derivative(p: &[Q]) -> P {
    p.iter()
        .enumerate()
        .skip(1)
        .map(|(i, x)| x * int(i))
        .collect()
}
fn normalized(p: P) -> CoreResult<P> {
    if let Some(a) = p.last() {
        scale(&p, &a.abs().recip())
    } else {
        Ok(p)
    }
}
fn divide(a: &[Q], b: &[Q]) -> CoreResult<(P, P)> {
    if b.is_empty() {
        return Err(bad("division by zero polynomial"));
    }
    let mut r = a.to_vec();
    let mut out = vec![zero(); a.len().saturating_sub(b.len()) + 1];
    while r.len() >= b.len() {
        let k = r.len() - b.len();
        let s = r.last().unwrap() / b.last().unwrap();
        out[k] = s.clone();
        for (i, x) in b.iter().enumerate() {
            r[k + i] -= &s * x;
        }
        r = checked(r)?;
    }
    Ok((checked(out)?, r))
}
fn gcd(mut a: P, mut b: P) -> CoreResult<P> {
    while !b.is_empty() {
        let r = normalized(divide(&a, &b)?.1)?;
        a = b;
        b = r;
    }
    normalized(a)
}
fn squarefree(p: &[Q]) -> CoreResult<(P, P)> {
    let g = gcd(p.to_vec(), derivative(p))?;
    let (a, r) = divide(p, &g)?;
    if !r.is_empty() {
        return Err(bad("nonzero exact quotient remainder"));
    }
    Ok((normalized(a)?, g))
}
fn sturm(p: P) -> CoreResult<Vec<P>> {
    if p.len() < 2 {
        return Ok(vec![p]);
    }
    let mut s = vec![normalized(p)?];
    s.push(normalized(derivative(&s[0]))?);
    loop {
        let n = s.len();
        let r = divide(&s[n - 2], &s[n - 1])?.1;
        if r.is_empty() {
            break;
        }
        s.push(normalized(scale(&r, &-Q::one())?)?);
    }
    Ok(s)
}
fn variations(s: &[P], x: &Q, w: &mut RootWork) -> usize {
    w.variation_calls += 1;
    let mut last = 0;
    let mut n = 0;
    for p in s {
        let t = sign(&eval(p, x));
        if t == 0 {
            continue;
        }
        if last != 0 && last != t {
            n += 1;
        }
        last = t;
    }
    n
}
fn count(s: &[P], a: &Q, b: &Q, w: &mut RootWork) -> CoreResult<usize> {
    variations(s, a, w)
        .checked_sub(variations(s, b, w))
        .ok_or_else(|| bad("Sturm count invariant"))
}
/// Outward binary64 enclosure of a rational. Exact comparisons check the
/// conversion, including subnormal and endpoint rounding.
fn iv(x: &Q) -> CoreResult<Interval> {
    let v = x
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| bad("rational projection overflow"))?;
    let mut lo = v;
    let mut hi = v;
    for _ in 0..4 {
        if q(lo)? <= *x && q(hi)? >= *x {
            return Interval::new(lo, hi);
        }
        if q(lo)? > *x {
            lo = lo.next_down();
        }
        if q(hi)? < *x {
            hi = hi.next_up();
        }
    }
    Err(bad("rational projection enclosure"))
}
/// An immutable source-declared affine function of physical time and state.
#[derive(Clone, Debug, Serialize)]
pub struct AffineFactor {
    pub constant: f64,
    pub time: f64,
    pub state: Vec<(usize, f64)>,
}
impl AffineFactor {
    pub fn clock(root: f64) -> Self {
        Self {
            constant: -root,
            time: 1.,
            state: vec![],
        }
    }
    pub fn component(index: usize, root: f64) -> Self {
        Self {
            constant: -root,
            time: 0.,
            state: vec![(index, 1.)],
        }
    }
}
/// One term c * product(affine factors). Empty factors denote a constant.
#[derive(Clone, Debug, Serialize)]
pub struct EventTerm {
    pub coefficient: f64,
    pub factors: Vec<AffineFactor>,
}
/// No opaque callbacks. Repeated factors retain exact source multiplicity.
#[derive(Clone, Debug, Serialize)]
pub struct DeclaredPolynomial {
    terms: Vec<EventTerm>,
}
impl DeclaredPolynomial {
    pub fn new(terms: Vec<EventTerm>) -> CoreResult<Self> {
        let s = Self { terms };
        s.validate(usize::MAX)?;
        Ok(s)
    }
    pub fn terms(&self) -> &[EventTerm] {
        &self.terms
    }
    pub fn validate(&self, n: usize) -> CoreResult<()> {
        if self.terms.is_empty()
            || self.terms.len() > 16
            || self.terms.iter().any(|t| {
                !t.coefficient.is_finite()
                    || t.factors.len() > 4
                    || t.factors.iter().any(|f| {
                        !f.constant.is_finite()
                            || !f.time.is_finite()
                            || f.state.len() > 16
                            || f.state.iter().any(|&(i, w)| i >= n || !w.is_finite())
                    })
            })
        {
            return Err(bad("source declaration cap/shape"));
        }
        Ok(())
    }
}
/// The exact-real normalized polynomial before rounded dense evaluation.
#[derive(Clone, Debug)]
pub struct RepresentedPolynomial {
    p: P,
    t0: Q,
    h: Q,
}
fn component(packet: &RepresentedAffineSegment) -> CoreResult<P> {
    let (h, u) = packet.exact_inputs();
    let mut d = [zero(), zero(), zero()];
    for i in 0..3 {
        for j in 0..8 {
            d[i] += q(h[i][j])? * q(u[j])?;
        }
    }
    let [a, b] = packet.states();
    let a = q(a)?;
    let b = q(b)?;
    checked(vec![
        a.clone(),
        b - &a + &d[0],
        &d[1] - &d[0],
        &d[2] - &d[1],
        -&d[2],
    ])
}
impl RepresentedPolynomial {
    /// Explicit polynomial fixture in normalized theta (binary64 exact).
    pub fn from_power(c: &[f64]) -> CoreResult<Self> {
        if c.len() > MAX_DEG + 1 {
            return Err(bad("input degree cap"));
        }
        Ok(Self {
            p: checked(c.iter().map(|&v| q(v)).collect::<CoreResult<P>>()?)?,
            t0: zero(),
            h: Q::one(),
        })
    }
    /// Compose source on a synthetic normalized time interval; useful for
    /// testing exact factored clock roots without a numerical integrator.
    pub fn clock_only(spec: &DeclaredPolynomial, t0: f64, t1: f64) -> CoreResult<Self> {
        spec.validate(0)?;
        Self::compose_inner(spec, t0, t1, |_| Err(bad("clock-only state")))
    }
    /// Freeze the physical time and differential state exactly, and vary
    /// the last (scalar algebraic) state on the supplied branch interval.
    /// The report's time coordinate is the algebraic coordinate here.
    pub fn frozen_scalar(
        spec: &DeclaredPolynomial,
        t: f64,
        x: &[f64],
        branch: [f64; 2],
    ) -> CoreResult<Self> {
        spec.validate(x.len() + 1)?;
        let t = q(t)?;
        let x = x.iter().map(|&v| q(v)).collect::<CoreResult<Vec<_>>>()?;
        let a = q(branch[0])?;
        let h = q(branch[1])? - &a;
        if !h.is_positive() {
            return Err(bad("scalar branch width"));
        }
        let mut p = vec![];
        for term in &spec.terms {
            let mut v = vec![q(term.coefficient)?];
            for f in &term.factors {
                let mut b = vec![q(f.constant)? + &t * q(f.time)?];
                for &(i, w) in &f.state {
                    let c = if i == x.len() {
                        vec![a.clone(), h.clone()]
                    } else {
                        vec![x[i].clone()]
                    };
                    b = add(&b, &scale(&c, &q(w)?)?)?;
                }
                v = mul(&v, &b)?;
            }
            p = add(&p, &v)?;
        }
        Ok(Self { p, t0: a, h })
    }
    /// Compose the actual accepted native packet, not a fitted polynomial.
    pub fn from_segment(
        s: &FastDenseSegments,
        k: usize,
        spec: &DeclaredPolynomial,
    ) -> CoreResult<Self> {
        spec.validate(s.dimension())?;
        if k >= s.len() {
            return Err(bad("segment index"));
        }
        let mut cache = std::collections::BTreeMap::new();
        Self::compose_inner(spec, s.t_old(k), s.t_new(k), |i| {
            if let Some(p) = cache.get(&i) {
                return Ok(P::clone(p));
            }
            let p = component(&s.represented_component(k, i)?)?;
            cache.insert(i, p.clone());
            Ok(p)
        })
    }
    fn compose_inner(
        spec: &DeclaredPolynomial,
        t0: f64,
        t1: f64,
        mut state: impl FnMut(usize) -> CoreResult<P>,
    ) -> CoreResult<Self> {
        let t0 = q(t0)?;
        let h = q(t1)? - &t0;
        if !h.is_positive() {
            return Err(bad("clock width"));
        }
        let clock = vec![t0.clone(), h.clone()];
        let mut p = vec![];
        for term in &spec.terms {
            let mut v = vec![q(term.coefficient)?];
            for f in &term.factors {
                let mut a = add(&[q(f.constant)?], &scale(&clock, &q(f.time)?)?)?;
                for &(i, w) in &f.state {
                    if w != 0. {
                        a = add(&a, &scale(&state(i)?, &q(w)?)?)?;
                    }
                }
                v = mul(&v, &a)?;
            }
            p = add(&p, &v)?;
        }
        Ok(Self { p, t0, h })
    }
    pub fn exact_coefficients(&self) -> Vec<String> {
        self.p.iter().map(ToString::to_string).collect()
    }
    pub fn exact_clock(&self) -> [String; 2] {
        [self.t0.to_string(), self.h.to_string()]
    }
    pub fn degree(&self) -> Option<usize> {
        self.p.len().checked_sub(1)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RootRoute {
    Sturm,
    NativeBernstein,
    DictionaryBernstein,
    Companion,
    DictionaryCompanion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RootKind {
    Up,
    Down,
    Touch,
}
#[derive(Clone, Debug, Serialize)]
pub struct RootInterval {
    /// Exact half-open isolation interval (lo,hi], or exact singleton.
    pub exact_theta: [String; 2],
    /// Outward binary64 intervals for normalized coordinate and real clock.
    pub theta: [f64; 2],
    pub time: [f64; 2],
    pub multiplicity: usize,
    pub kind: RootKind,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct RootWork {
    pub variation_calls: u64,
    pub isolation_nodes: usize,
    pub sturm_chains: usize,
    pub screen_attempts: usize,
    pub screened_intervals: usize,
    pub dictionary_uses: usize,
    pub native_bernstein_uses: usize,
    pub conditioned_horner_uses: usize,
    pub companion_attempts: usize,
    pub companion_hints: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct RootReport {
    pub route: RootRoute,
    pub roots: Vec<RootInterval>,
    pub max_theta_width: f64,
    pub work: RootWork,
}
fn binom(n: usize, k: usize) -> usize {
    if k > n {
        0
    } else {
        (0..k).fold(1, |a, i| a * (n - i) / (i + 1))
    }
}
fn interval_horner(p: &[Q], x: Interval, w: &mut RootWork) -> CoreResult<Interval> {
    let coeff = p.iter().map(iv).collect::<CoreResult<Vec<_>>>()?;
    if coeff.iter().all(|v| v.lo == v.hi) {
        w.conditioned_horner_uses += 1;
        return eval_poly_interval(&coeff.iter().map(|v| v.lo).collect::<Vec<_>>(), x)
            .map_err(|_| bad("conditioned Horner"));
    }
    coeff
        .iter()
        .rev()
        .try_fold(Interval::point(0.)?, |a, &b| a.mul(x)?.add(b))
}
fn bernstein_coeffs(p: &[Q], dictionary: bool, w: &mut RootWork) -> CoreResult<Vec<Interval>> {
    let n = p.len() - 1;
    if dictionary && n <= 6 {
        let d = Dictionary::get()?;
        let m = if n <= 2 {
            3
        } else if n <= 4 {
            5
        } else {
            7
        };
        let b = d.basis(m)?;
        w.dictionary_uses += 1;
        let y = b
            .nodes
            .iter()
            .map(|v| interval_horner(p, v.enclosure, w))
            .collect::<CoreResult<Vec<_>>>()?;
        let mut out = vec![Interval::point(0.)?; m];
        for (j, v) in y.iter().enumerate() {
            for (i, x) in out.iter_mut().enumerate() {
                *x = x.add(v.mul(b.bernstein[j][i].enclosure)?)?;
            }
        }
        return Ok(out);
    }
    if n <= 4 {
        w.native_bernstein_uses += 1;
        let mut c = [Interval::point(0.)?; 5];
        for (i, v) in p.iter().enumerate() {
            c[i] = iv(v)?;
        }
        return Ok(bernstein(&c)?.to_vec());
    }
    (0..=n)
        .map(|i| {
            iv(&(0..=i).fold(zero(), |a, j| {
                a + &p[j] * int(binom(i, j)) / int(binom(n, j))
            }))
        })
        .collect()
}
fn strict_sign(b: &[Interval]) -> bool {
    b.iter().all(|x| x.lo > 0.) || b.iter().all(|x| x.hi < 0.)
}
fn half(b: &[Interval]) -> CoreResult<(Vec<Interval>, Vec<Interval>)> {
    let mut a = b.to_vec();
    let mut l = vec![a[0]];
    let mut r = vec![*a.last().unwrap()];
    while a.len() > 1 {
        a = a
            .windows(2)
            .map(|v| v[0].scale(0.5)?.add(v[1].scale(0.5)?))
            .collect::<CoreResult<Vec<_>>>()?;
        l.push(a[0]);
        r.push(*a.last().unwrap());
    }
    r.reverse();
    Ok((l, r))
}
/// Optional outward sign screens; failures of optional arithmetic are
/// conservatively discarded, never interpreted as no root.
fn screened_partitions(p: &[Q], dict: bool, w: &mut RootWork) -> Vec<(Q, Q)> {
    w.screen_attempts += 1;
    let all = vec![(zero(), Q::one())];
    let Some(m) = p.iter().map(Signed::abs).max() else {
        return all;
    };
    let Ok(c) = scale(p, &m.recip()) else {
        return all;
    };
    let Ok(b) = bernstein_coeffs(&c, dict, w) else {
        return all;
    };
    if strict_sign(&b) {
        w.screened_intervals += 1;
        return vec![];
    }
    let Ok((l, r)) = half(&b) else {
        return all;
    };
    let mid = Q::one() / int(2);
    let mut out = vec![];
    for (a, z, v) in [(zero(), mid.clone(), l), (mid, Q::one(), r)] {
        if strict_sign(&v) {
            w.screened_intervals += 1;
        } else {
            out.push((a, z));
        }
    }
    out
}
fn companion_hints(p: &[Q], tol: f64, w: &mut RootWork) -> Vec<Q> {
    let n = p.len() - 1;
    if n < 2 {
        return vec![];
    }
    w.companion_attempts += 1;
    let Some(c) = p[..n]
        .iter()
        .map(|v| {
            (v / &p[n])
                .to_f64()
                .filter(|x| x.is_finite() && x.abs() < 1e100)
        })
        .collect::<Option<Vec<_>>>()
    else {
        return vec![];
    };
    let a = faer::Mat::<f64>::from_fn(n, n, |i, j| {
        if j + 1 == n {
            -c[i]
        } else if i == j + 1 {
            1.
        } else {
            0.
        }
    });
    let Ok(e) = a.eigenvalues() else {
        return vec![];
    };
    let mut x = Vec::new();
    for v in e {
        // Even a bad eigenvalue may only propose a split; never omit a root.
        if v.re.is_finite() && v.re > 0. && v.re < 1. {
            for f in [v.re - tol * 0.125, v.re + tol * 0.125] {
                if f > 0.
                    && f < 1.
                    && let Ok(r) = q(f)
                {
                    x.push(r);
                }
            }
        }
    }
    x.sort();
    x.dedup();
    w.companion_hints += x.len();
    x
}
fn positive_after(p: &[Q], b: &Q) -> CoreResult<bool> {
    let mut r = p.to_vec();
    while !r.is_empty() {
        let v = eval(&r, b);
        if !v.is_zero() {
            return Ok(v.is_positive());
        }
        r = derivative(&r);
    }
    Err(bad("zero event polynomial"))
}
impl RepresentedPolynomial {
    /// Complete exact distinct-root isolation in (0,1]. A zero polynomial,
    /// resource cap or unrepresentable output is a refusal, not an empty set.
    pub fn isolate(&self, route: RootRoute, tol: f64) -> CoreResult<RootReport> {
        check_binary64_environment()?;
        if !tol.is_finite() || !(2f64.powi(-50)..=1e-3).contains(&tol) {
            return Err(bad("theta tolerance"));
        }
        if self.p.is_empty() {
            return Err(bad("identically zero event (nonisolated roots)"));
        }
        let mut w = RootWork::default();
        let mut roots = vec![];
        let mut parts = match route {
            RootRoute::NativeBernstein => screened_partitions(&self.p, false, &mut w),
            RootRoute::DictionaryBernstein | RootRoute::DictionaryCompanion => {
                screened_partitions(&self.p, true, &mut w)
            }
            _ => vec![(zero(), Q::one())],
        };
        if self.p.len() == 1 || parts.is_empty() {
            return Ok(RootReport {
                route,
                roots,
                max_theta_width: 0.,
                work: w,
            });
        }
        let (sf, mut g) = squarefree(&self.p)?;
        let s = sturm(sf.clone())?;
        w.sturm_chains += 1;
        let mut layers = Vec::new();
        while g.len() > 1 {
            let (f, ng) = squarefree(&g)?;
            layers.push(sturm(f)?);
            w.sturm_chains += 1;
            g = ng;
        }
        if matches!(route, RootRoute::Companion | RootRoute::DictionaryCompanion) {
            let hints = companion_hints(&sf, tol, &mut w);
            let mut split = vec![];
            for (a, b) in parts {
                let mut l = a;
                for x in &hints {
                    if *x > l && *x < b {
                        split.push((l, x.clone()));
                        l = x.clone();
                    }
                }
                split.push((l, b));
            }
            parts = split;
        }
        let target = q(tol * 0.5)?;
        let mut stack = Vec::new();
        for (a, b) in parts.into_iter().rev() {
            let n = count(&s, &a, &b, &mut w)?;
            if n > 0 {
                stack.push((a, b, n, 0));
            }
        }
        let mut max_width = 0.0_f64;
        while let Some((a, b, n, depth)) = stack.pop() {
            w.isolation_nodes += 1;
            if depth > MAX_DEPTH || w.isolation_nodes > MAX_NODES {
                return Err(bad("isolation resource cap"));
            }
            if n == 1 && (eval(&sf, &b).is_zero() || &b - &a <= target) {
                let mut multiplicity = 1;
                for l in &layers {
                    multiplicity += count(l, &a, &b, &mut w)?;
                }
                let kind = if multiplicity % 2 == 0 {
                    RootKind::Touch
                } else if positive_after(&self.p, &b)? {
                    RootKind::Up
                } else {
                    RootKind::Down
                };
                let l = if eval(&sf, &b).is_zero() {
                    b.clone()
                } else {
                    a
                };
                let time_l = &self.t0 + &self.h * &l;
                let time_b = &self.t0 + &self.h * &b;
                let theta = [iv(&l)?.lo, iv(&b)?.hi];
                let time = [iv(&time_l)?.lo, iv(&time_b)?.hi];
                let width = sub_up(theta[1], theta[0])?;
                if width > tol {
                    return Err(bad("binary64 bracket width"));
                }
                max_width = max_width.max(width);
                roots.push(RootInterval {
                    exact_theta: [l.to_string(), b.to_string()],
                    theta,
                    time,
                    multiplicity,
                    kind,
                });
                continue;
            }
            let mid = (&a + &b) / int(2);
            let left = count(&s, &a, &mid, &mut w)?;
            let right = n
                .checked_sub(left)
                .ok_or_else(|| bad("split count invariant"))?;
            if right > 0 {
                stack.push((mid.clone(), b, right, depth + 1));
            }
            if left > 0 {
                stack.push((a, mid, left, depth + 1));
            }
        }
        Ok(RootReport {
            route,
            roots,
            max_theta_width: max_width,
            work: w,
        })
    }
    /// Existing RETRO portfolio ranks actual certified routes; every call
    /// recomputes the complete certificate for this represented polynomial.
    pub fn adaptive_isolate(
        &self,
        portfolio: &mut Portfolio,
        tol: f64,
    ) -> CoreResult<Selection<RootReport>> {
        let routes = if self.p.len() <= 7 {
            [
                Route::EventDictionary,
                Route::EventBernstein,
                Route::EventSturm,
                Route::EventDictionaryCompanion,
                Route::EventCompanion,
            ]
        } else {
            [
                Route::EventBernstein,
                Route::EventSturm,
                Route::EventCompanion,
                Route::EventDictionaryCompanion,
                Route::EventDictionary,
            ]
        };
        portfolio.choose(
            "represented-event",
            &[self.p.len() as u64],
            &routes,
            tol,
            |r| {
                let mode = match r {
                    Route::EventSturm => RootRoute::Sturm,
                    Route::EventBernstein => RootRoute::NativeBernstein,
                    Route::EventDictionary => RootRoute::DictionaryBernstein,
                    Route::EventCompanion => RootRoute::Companion,
                    Route::EventDictionaryCompanion => RootRoute::DictionaryCompanion,
                    _ => return Err(bad("route")),
                };
                let v = self.isolate(mode, tol)?;
                let e = v.max_theta_width;
                Ok((v, e))
            },
        )
    }
}
