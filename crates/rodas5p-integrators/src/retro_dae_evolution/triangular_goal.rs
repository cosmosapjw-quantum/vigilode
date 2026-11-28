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
fn positive(v: Iv) -> CoreResult<Iv> {
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
    let mut v = p(x)?;
    let mut k = 0;
    if route == TriangularGoalRoute::RootReduced {
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
        TriangularGoalRoute::Adaptive | TriangularGoalRoute::RootReduced
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
fn log1p(v: Iv, route: TriangularGoalRoute, w: &mut TriangularGoalWork) -> CoreResult<Iv> {
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
