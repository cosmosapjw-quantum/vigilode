//! Source-bound terminal goal for exact triangular reduced clock blocks.
//! This kernel is conditional on the caller's exact initial differential
//! state; the retained facade independently requires physical qualification.
use super::clock_tube::{matches_terms, source_kind};
use super::*;
use rodas5p_core::directed::{sqrt_down, sqrt_up, sub_up};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TriangularGoalRoute {
    Atanh12,
    Atanh24,
    Adaptive,
    RootReduced,
    TinyAdaptive,
    TinyRootReduced,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct TriangularGoalWork {
    pub blocks: usize,
    pub series_terms: usize,
    pub square_roots: usize,
    pub max_tail_upper: f64,
    pub signed_error: [f64; 2],
    pub width_upper: f64,
}
pub(super) fn positive(v: Iv) -> CoreResult<Iv> {
    Iv::new(v.lo.max(0.), v.hi)
}
// Evaluate monotonic log1p at represented scalar endpoints; avoid dependency
// between numerator and denominator of s=x/(2+x). Positive series tail.
fn log_point(x: f64, route: TriangularGoalRoute, w: &mut TriangularGoalWork) -> CoreResult<Iv> {
    if !x.is_finite() || x < 0. {
        return Err(bad("triangular log domain"));
    }
    if x == 0. {
        return p(0.);
    }
    // Small positive argument: integrate 1-u <= 1/(1+u) <= 1.
    // No relative stopping threshold underflows at the smallest subnormal.
    if matches!(
        route,
        TriangularGoalRoute::TinyAdaptive | TriangularGoalRoute::TinyRootReduced
    ) && x <= 2f64.powi(-52)
    {
        let tail = positive(p(x)?.mul(p(x)?)?.scale(0.5)?)?.hi;
        w.max_tail_upper = w.max_tail_upper.max(tail);
        return Iv::new(p(x)?.sub(p(tail)?)?.lo.max(0.), x);
    }
    let mut v = p(x)?;
    let mut k = 0;
    if matches!(
        route,
        TriangularGoalRoute::RootReduced | TriangularGoalRoute::TinyRootReduced
    ) {
        while v.hi > 0.25 && k < 8 {
            let q = p(1.)?.add(v)?;
            let root = Iv::new(sqrt_down(q.lo)?, sqrt_up(q.hi)?)?;
            v = positive(v.div(root.add(p(1.)?)?)?)?;
            k += 1;
            w.square_roots += 1;
        }
    }
    // For large arguments direct atanh is allowed to refuse; no rounded
    // library logarithm is silently treated as exact.
    let ratio = |a: f64| -> CoreResult<Iv> { positive(p(a)?.div(p(2.)?.add(p(a)?)?)?) };
    let s = Iv::new(ratio(v.lo)?.lo, ratio(v.hi)?.hi)?;
    if s.hi >= 1. {
        return Err(bad("triangular log tail denominator"));
    }
    let s2 = positive(s.mul(s)?)?;
    let gap = p(1.)?.sub(p(s2.hi)?)?;
    if gap.lo <= 0. {
        return Err(bad("triangular log tail gap"));
    }
    let cap = match route {
        TriangularGoalRoute::Atanh12 => 12,
        TriangularGoalRoute::Atanh24 => 24,
        _ => 64,
    };
    let adaptive = matches!(
        route,
        TriangularGoalRoute::Adaptive
            | TriangularGoalRoute::RootReduced
            | TriangularGoalRoute::TinyAdaptive
            | TriangularGoalRoute::TinyRootReduced
    );
    let mut power = s;
    let mut sum = p(0.)?;
    let factor = (1u32 << k) as f64 * 2.;
    for i in 0..cap {
        sum = sum.add(power.div(p((2 * i + 1) as f64)?)?)?;
        w.series_terms += 1;
        let next = positive(power.mul(s2)?)?;
        let tail = positive(next.div(p((2 * i + 3) as f64)?.mul(gap)?)?.scale(factor)?)?.hi;
        if i + 1 == cap || (adaptive && tail <= 2f64.powi(-55) * x.min(1.)) {
            let a = sum.scale(factor)?;
            w.max_tail_upper = w.max_tail_upper.max(tail);
            return Iv::new(a.lo.max(0.), add_up(a.hi, tail)?);
        }
        power = next;
    }
    unreachable!()
}
pub(super) fn log1p(
    v: Iv,
    route: TriangularGoalRoute,
    w: &mut TriangularGoalWork,
) -> CoreResult<Iv> {
    if v.lo < 0. {
        return Err(bad("triangular log negative interval"));
    }
    let lo = log_point(v.lo, route, w)?;
    if v.lo == v.hi {
        return Ok(lo);
    }
    let hi = log_point(v.hi, route, w)?;
    Iv::new(lo.lo, hi.hi)
}
/// Exact represented source proof, independent blocks or the existing
/// coupled coordinate variant. Returns a signed exact-flow minus numerical
/// physical observation enclosure. No authority for a nonzero initial error.
pub fn triangular_clock_goal(
    src: &DaeSource,
    start: f64,
    end: f64,
    initial: &[f64],
    terminal: &[f64],
    weights: &[f64],
    route: TriangularGoalRoute,
    out: &mut Iv,
) -> CoreResult<TriangularGoalWork> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let n = src.nx();
    if n == 0
        || n % 2 != 0
        || src.nz() != n
        || initial.len() != n
        || terminal.len() != 2 * n
        || weights.len() != 2 * n
        || !start.is_finite()
        || !end.is_finite()
        || start < 0.
        || end < start
        || initial
            .iter()
            .chain(terminal)
            .chain(weights)
            .any(|v| !v.is_finite())
    {
        return Err(bad("triangular clock shape/clock"));
    }
    for i in 0..n {
        if !matches_terms(
            &src.g[i],
            &[(1., &[(n + i + 1) as u8]), (-1., &[(i + 1) as u8])],
        ) {
            return Err(bad("triangular clock algebraic graph"));
        }
    }
    let coupled = source_kind(src) == Some(true);
    let dt = p(end)?
        .sub(p(start)?)?
        .mul(p(end)?.add(p(start)?)?)?
        .scale(0.5)?;
    let dt = positive(dt)?;
    let mut work = TriangularGoalWork::default();
    let mut result = p(0.)?;
    for i in (0..n).step_by(2) {
        let a = if coupled {
            1.
        } else {
            let eq = &src.f[i];
            let a = -eq.terms()[0].coefficient;
            let j = (n + i + 1) as u8;
            if !(a > 0. && a.is_finite())
                || !matches_terms(eq, &[(-a, &[0, j, j])])
                || !matches_terms(&src.f[i + 1], &[(1., &[0, j])])
            {
                return Err(bad("triangular clock unsupported differential source"));
            }
            a
        };
        let q0 = p(initial[i + 1])?;
        let p0 = if coupled {
            p(initial[i])?.sub(q0)?
        } else {
            p(initial[i])?
        };
        if p0.lo < 0. {
            return Err(bad("triangular clock initial p negative"));
        }
        let v = positive(dt.mul(p0)?.scale(a)?)?;
        let pe = p0.div(p(1.)?.add(v)?)?;
        let qe = q0.add(log1p(v, route, &mut work)?.div(p(a)?)?)?;
        let ue = if coupled { pe.add(qe)? } else { pe };
        // Subtract each actual numerical physical coordinate before
        // weighting. Preserve the original observation, not a projection.
        for (j, x) in [(i, ue), (i + 1, qe), (n + i, ue), (n + i + 1, qe)] {
            result = result.add(x.sub(p(terminal[j])?)?.scale(weights[j])?)?;
        }
        work.blocks += 1;
    }
    work.signed_error = result.as_pair();
    work.width_upper = sub_up(result.hi, result.lo)?;
    *out = result;
    Ok(work)
}

// Exact syntactic proof for a shared monomial clock; do not infer a
// coefficient by fitting or by a rounded ratio of source terms.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct StructuredClockWork {
    pub blocks: usize,
    pub max_gradient_width: f64,
    pub clock_powers: [usize; 3],
}
pub(super) fn clock_partition(
    src: &DaeSource,
    initial: &[f64],
    weights: &[f64],
    start: f64,
    end: f64,
) -> CoreResult<bool> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let n = src.nx();
    if n == 0
        || n % 2 != 0
        || n > 16
        || src.nz() != n
        || initial.len() != n
        || weights.len() != 2 * n
        || !start.is_finite()
        || !end.is_finite()
        || start < 0.
        || end < start
        || initial.iter().chain(weights).any(|v| !v.is_finite())
    {
        return Err(bad("structured clock shape"));
    }
    for i in 0..n {
        if !matches_terms(
            &src.g[i],
            &[(1., &[(n + i + 1) as u8]), (-1., &[(i + 1) as u8])],
        ) {
            return Err(bad("structured clock graph"));
        }
    }
    Ok(source_kind(src) == Some(true))
}
fn block_clock(src: &DaeSource, i: usize, coupled: bool) -> CoreResult<(f64, f64, usize)> {
    if coupled {
        return Ok((1., 1., 1));
    }
    let j = (src.nx() + i + 1) as u8;
    let a = -src.f[i].terms()[0].coefficient;
    let b = src.f[i + 1].terms()[0].coefficient;
    if a <= 0. || !a.is_finite() || !b.is_finite() {
        return Err(bad("structured clock coefficients"));
    }
    for k in 0..=2 {
        let mut pf = [0u8; 4];
        pf[k] = j;
        pf[k + 1] = j;
        let mut qf = [0u8; 3];
        qf[k] = j;
        if matches_terms(&src.f[i], &[(-a, &pf[..k + 2])])
            && matches_terms(&src.f[i + 1], &[(b, &qf[..k + 1])])
        {
            return Ok((a, b, k));
        }
    }
    Err(bad("structured clock unsupported monomial source"))
}
pub(super) fn clock_measure(start: f64, end: f64, k: usize) -> CoreResult<Iv> {
    let t = p(end)?;
    let s = p(start)?;
    let d = t.sub(s)?;
    positive(match k {
        0 => d,
        1 => d.mul(t.add(s)?)?.scale(0.5)?,
        2 => d
            .mul(t.mul(t)?.add(t.mul(s)?)?.add(s.mul(s)?)?)?
            .div(p(3.)?)?,
        _ => return Err(bad("structured clock power")),
    })
}
pub(super) fn initial_block(initial: &[f64], i: usize, coupled: bool) -> CoreResult<(Iv, Iv)> {
    let q = p(initial[i + 1])?;
    let pp = if coupled {
        p(initial[i])?.sub(q)?
    } else {
        p(initial[i])?
    };
    if pp.lo < 0. {
        return Err(bad("structured clock negative initial p"));
    }
    Ok((pp, q))
}
/// Exact true-flow initial differential gradient for a represented linear
/// terminal functional. Algebraic graph z=x; no parameter or discrete gradient.
/// On failure the output slice is unchanged, including late block refusal.
pub fn structured_clock_adjoint_into(
    src: &DaeSource,
    start: f64,
    end: f64,
    initial: &[f64],
    weights: &[f64],
    out: &mut [Iv],
) -> CoreResult<StructuredClockWork> {
    let coupled = clock_partition(src, initial, weights, start, end)?;
    let n = src.nx();
    if out.len() != n {
        return Err(bad("structured clock gradient shape"));
    }
    let mut result = [Iv { lo: 0., hi: 0. }; 16];
    let mut work = StructuredClockWork::default();
    for i in (0..n).step_by(2) {
        let (a, b, k) = block_clock(src, i, coupled)?;
        let (p0, _) = initial_block(initial, i, coupled)?;
        let m = clock_measure(start, end, k)?;
        let d = p(1.)?.add(positive(m.mul(p0)?.scale(a)?)?)?;
        let wu = p(weights[i])?.add(p(weights[n + i])?)?;
        let wv = p(weights[i + 1])?.add(p(weights[n + i + 1])?)?;
        let wq = if coupled { wu.add(wv)? } else { wv };
        let dp = p(1.)?.div(d)?.div(d)?;
        let dq = m.scale(b)?.div(d)?;
        let lp = wu.mul(dp)?.add(wq.mul(dq)?)?;
        result[i] = lp;
        result[i + 1] = if coupled { wq.sub(lp)? } else { wq };
        work.blocks += 1;
        work.clock_powers[k] += 1;
    }
    for v in &result[..n] {
        work.max_gradient_width = work.max_gradient_width.max(sub_up(v.hi, v.lo)?);
    }
    out.copy_from_slice(&result[..n]);
    Ok(work)
}
/// Signed exact-flow minus represented physical terminal observation, for
/// the same generalized monomial source class as the gradient.
pub fn structured_clock_goal_into(
    src: &DaeSource,
    start: f64,
    end: f64,
    initial: &[f64],
    terminal: &[f64],
    weights: &[f64],
    route: TriangularGoalRoute,
    out: &mut Iv,
) -> CoreResult<TriangularGoalWork> {
    let coupled = clock_partition(src, initial, weights, start, end)?;
    let n = src.nx();
    if terminal.len() != 2 * n || terminal.iter().any(|v| !v.is_finite()) {
        return Err(bad("structured clock terminal shape"));
    }
    let mut result = p(0.)?;
    let mut work = TriangularGoalWork::default();
    for i in (0..n).step_by(2) {
        let (a, b, k) = block_clock(src, i, coupled)?;
        let (p0, q0) = initial_block(initial, i, coupled)?;
        let m = clock_measure(start, end, k)?;
        let v = positive(m.mul(p0)?.scale(a)?)?;
        let pe = p0.div(p(1.)?.add(v)?)?;
        let qe = q0.add(log1p(v, route, &mut work)?.div(p(a)?)?.scale(b)?)?;
        let ue = if coupled { pe.add(qe)? } else { pe };
        for (j, x) in [(i, ue), (i + 1, qe), (n + i, ue), (n + i + 1, qe)] {
            result = result.add(x.sub(p(terminal[j])?)?.scale(weights[j])?)?;
        }
        work.blocks += 1;
    }
    work.signed_error = result.as_pair();
    work.width_upper = sub_up(result.hi, result.lo)?;
    *out = result;
    Ok(work)
}
