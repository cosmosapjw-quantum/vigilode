//! A certified output bound for `exp(tau A) v` with nonnormal `A`
//! (integrated DAG node INT-05, external review V1/TF-09). Research only.
//!
//! With a diagonal metric `D` (the identity when none is given) and
//! `B = D A D^-1`, for any candidate `x`
//!
//! ```text
//! exp(tau A) v - x = D^-1 [e^{tau B} - p_m(tau B)] D v + (D^-1 p_m(tau B) D v - x),
//! ```
//!
//! `p_m` the degree-`m` Taylor polynomial. The first term is bounded by the
//! Crouzeix-Palencia theorem `||g(B)||_2 <= (1 + sqrt 2) sup_{W(B)} |g|`
//! over a verified box `Omega` containing the numerical range `W(B)`
//! (Gershgorin bounds of the symmetric part for `Re`, the max row sum of the
//! skew part for `|Im|`), with
//! `|e^z - p_m(z)| <= |z|^{m+1}/(m+1)! max(1, e^{Re z})`. The second term is
//! enclosed componentwise by evaluating `p_m(tau B) D v` by Horner in
//! outward interval arithmetic. Nothing sampled (residuals, Ritz values, a
//! declared spectrum) enters the bound.

use serde::Serialize;

use crate::{
    CoreError, CoreResult,
    directed::{
        Interval, add_down, add_up, div_up, exp_interval, mul_down, mul_up, sqrt_down, sqrt_up,
        sub_down, sub_up,
    },
};

/// Whether the bound is finite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NonnormalBoundStatus {
    Bounded,
    /// An upper bound overflowed: the certificate says nothing.
    Unbounded,
}

/// `Omega = [re_lo, re_hi] x i[-im, im]` containing `W(B)` (unscaled by
/// `tau`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct NumericalRangeBox {
    pub re_lo: f64,
    pub re_hi: f64,
    pub im: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NonnormalExpCertificate {
    pub status: NonnormalBoundStatus,
    /// The candidate the bounds are about: the caller's, or the method's own
    /// value `D^-1 mid(p_m(tau B) D v)`.
    pub candidate: Vec<f64>,
    /// `||exp(tau A) v - candidate||_2` lies in `[error_lower, error_upper]`.
    pub error_upper: f64,
    pub error_lower: f64,
    pub truncation_upper: f64,
    pub distance_upper: f64,
    pub distance_lower: f64,
    /// `||D^-1||_2 ||D v||_2`.
    pub transport: f64,
    pub numerical_range: NumericalRangeBox,
    pub degree: usize,
}

const MAX_DEGREE: usize = 60;

fn invalid(why: &str) -> CoreError {
    CoreError::InvalidInput(format!("nonnormal certificate: {why}"))
}

/// Certify `||exp(tau A) v - x||_2` for `x = candidate` (or the method's
/// own value when `None`); see the module documentation. `a` is a dense
/// row-major `n x n` matrix given as rows.
pub fn certify_exp_action(
    a: &[Vec<f64>],
    v: &[f64],
    tau: f64,
    metric: Option<&[f64]>,
    degree: usize,
    candidate: Option<&[f64]>,
) -> CoreResult<NonnormalExpCertificate> {
    let n = v.len();
    if n == 0 || a.len() != n || a.iter().any(|row| row.len() != n) {
        return Err(CoreError::Dimension(
            "nonnormal certificate: A must be n x n for a vector of length n".into(),
        ));
    }
    if !(1..=MAX_DEGREE).contains(&degree) {
        return Err(invalid("degree must lie in 1..=60"));
    }
    if !(tau.is_finite() && tau >= 0.0) {
        return Err(invalid("tau must be finite and nonnegative"));
    }
    if !a.iter().flatten().chain(v).all(|x| x.is_finite()) {
        return Err(invalid("non-finite A or v"));
    }
    if let Some(x) = candidate {
        if x.len() != n {
            return Err(CoreError::Dimension(
                "nonnormal certificate: candidate length".into(),
            ));
        }
        if !x.iter().all(|value| value.is_finite()) {
            return Err(invalid("non-finite candidate"));
        }
    }
    let ones = vec![1.0; n];
    let d = match metric {
        Some(d) => {
            if d.len() != n {
                return Err(CoreError::Dimension(
                    "nonnormal certificate: metric length".into(),
                ));
            }
            if !d.iter().all(|x| x.is_finite() && *x > 0.0) {
                return Err(invalid("metric entries must be finite and positive"));
            }
            d
        }
        None => &ones,
    };
    // B = D A D^-1, enclosed entrywise.
    let mut b = vec![vec![Interval::point(0.0)?; n]; n];
    for i in 0..n {
        for j in 0..n {
            b[i][j] = Interval::point(d[i])?
                .mul(Interval::point(a[i][j])?)?
                .div(Interval::point(d[j])?)?;
        }
    }
    let range = numerical_range_box(&b)?;
    // D v and Horner p_m(tau B) D v = u + (tau/1) B (u + (tau/2) B (...)).
    let u = (0..n)
        .map(|i| Interval::point(d[i])?.mul(Interval::point(v[i])?))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut w = u.clone();
    for j in (1..=degree).rev() {
        let factor = Interval::point(tau)?.div(Interval::point(j as f64)?)?;
        let mut next = Vec::with_capacity(n);
        for i in 0..n {
            let mut bw = Interval::point(0.0)?;
            for k in 0..n {
                bw = bw.add(b[i][k].mul(w[k])?)?;
            }
            next.push(u[i].add(factor.mul(bw)?)?);
        }
        w = next;
    }
    // Back to the physical coordinates.
    let p = (0..n)
        .map(|i| w[i].div(Interval::point(d[i])?))
        .collect::<CoreResult<Vec<_>>>()?;
    let own: Vec<f64> = p.iter().map(|x| 0.5 * x.lo + 0.5 * x.hi).collect();
    let x = candidate.map_or(own, <[f64]>::to_vec);
    let mut upper_sq = 0.0;
    let mut lower_sq = 0.0;
    for (xi, pi) in x.iter().zip(&p) {
        let gap = Interval::point(*xi)?.sub(*pi)?;
        upper_sq = add_up(upper_sq, mul_up(gap.mag(), gap.mag())?)?;
        let m = gap.mig();
        lower_sq = add_down(lower_sq, mul_down(m, m)?)?;
    }
    let distance_upper = sqrt_up(upper_sq)?;
    let distance_lower = sqrt_down(lower_sq.max(0.0))?;
    // Transport ||D^-1||_2 ||D v||_2.
    let inv_max = d
        .iter()
        .map(|x| div_up(1.0, *x))
        .collect::<CoreResult<Vec<_>>>()?
        .into_iter()
        .fold(0.0_f64, f64::max);
    let mut dv_sq = 0.0;
    for ui in &u {
        dv_sq = add_up(dv_sq, mul_up(ui.mag(), ui.mag())?)?;
    }
    let transport = mul_up(inv_max, sqrt_up(dv_sq)?)?;
    let truncation = truncation_upper(range, tau, degree)
        .and_then(|s| mul_up(mul_up(add_up(1.0, sqrt_up(2.0)?)?, s)?, transport));
    let finish = |status, error_upper, error_lower, truncation_upper| NonnormalExpCertificate {
        status,
        candidate: x.clone(),
        error_upper,
        error_lower,
        truncation_upper,
        distance_upper,
        distance_lower,
        transport,
        numerical_range: range,
        degree,
    };
    match truncation {
        Ok(t) if t.is_finite() => {
            let upper = add_up(distance_upper, t)?;
            let lower = sub_down(distance_lower, t)?.max(0.0);
            if upper.is_finite() {
                Ok(finish(NonnormalBoundStatus::Bounded, upper, lower, t))
            } else {
                Ok(finish(
                    NonnormalBoundStatus::Unbounded,
                    f64::INFINITY,
                    0.0,
                    t,
                ))
            }
        }
        // An overflowing remainder bound is no bound.
        _ => Ok(finish(
            NonnormalBoundStatus::Unbounded,
            f64::INFINITY,
            0.0,
            f64::INFINITY,
        )),
    }
}

/// `Omega` from the enclosed `B`: Gershgorin bounds of `H = (B + B^T)/2`
/// and the max absolute row sum of `S = (B - B^T)/2`.
#[allow(clippy::needless_range_loop)] // (i, j) and (j, i) are both read
fn numerical_range_box(b: &[Vec<Interval>]) -> CoreResult<NumericalRangeBox> {
    let n = b.len();
    let half = Interval::point(0.5)?;
    let mut re_lo = f64::INFINITY;
    let mut re_hi = f64::NEG_INFINITY;
    let mut im = 0.0_f64;
    for i in 0..n {
        let mut off = 0.0;
        let mut skew = 0.0;
        for j in 0..n {
            if i == j {
                continue;
            }
            let h = b[i][j].add(b[j][i])?.mul(half)?;
            let s = b[i][j].sub(b[j][i])?.mul(half)?;
            off = add_up(off, h.mag())?;
            skew = add_up(skew, s.mag())?;
        }
        re_lo = re_lo.min(sub_down(b[i][i].lo, off)?);
        re_hi = re_hi.max(add_up(b[i][i].hi, off)?);
        im = im.max(skew);
    }
    Ok(NumericalRangeBox { re_lo, re_hi, im })
}

/// `sup_{z in tau Omega} |e^z - p_m(z)| <= R^{m+1}/(m+1)! max(1, e^{tau re_hi})`
/// with `R` the largest modulus on `tau Omega`, rounded up.
fn truncation_upper(range: NumericalRangeBox, tau: f64, degree: usize) -> CoreResult<f64> {
    let re = range.re_lo.abs().max(range.re_hi.abs());
    let modulus = sqrt_up(add_up(mul_up(re, re)?, mul_up(range.im, range.im)?)?)?;
    let r = mul_up(tau, modulus)?;
    let mut term = 1.0_f64;
    for k in 1..=degree + 1 {
        term = mul_up(term, div_up(r, k as f64)?)?;
    }
    let growth_exponent = mul_up(tau, range.re_hi)?;
    let growth = if growth_exponent <= 0.0 {
        1.0
    } else {
        // REV-02: a directed enclosure of the exponential (overflow is no
        // bound).
        crate::directed::exp_interval(growth_exponent)
            .map_err(|_| invalid("growth factor overflows"))?
            .hi
    };
    let bound = mul_up(term, growth)?;
    if bound.is_finite() {
        Ok(bound)
    } else {
        Err(invalid("truncation bound overflows"))
    }
}

/// The Osborne balancing metric of `A` (review DAG node REV-02): powers of
/// two `d_i` such that, for every `i`, the off-diagonal 1-norms of row `i`
/// and column `i` of `D A D^-1` are within a factor of 2 (the LAPACK `gebal`
/// iteration without permutation; at most 100 sweeps). Computed from `A`
/// alone; `D A D^-1` is exact in binary64.
pub fn osborne_metric(a: &[Vec<f64>]) -> CoreResult<Vec<f64>> {
    let n = a.len();
    if n == 0 || a.iter().any(|row| row.len() != n) {
        return Err(CoreError::Dimension(
            "nonnormal certificate: A must be square".into(),
        ));
    }
    if !a.iter().flatten().all(|x| x.is_finite()) {
        return Err(invalid("non-finite A"));
    }
    let mut d = vec![1.0_f64; n];
    for _ in 0..100 {
        let mut changed = false;
        for i in 0..n {
            let (mut row, mut col) = (0.0_f64, 0.0_f64);
            for j in 0..n {
                if j != i {
                    row += (d[i] * a[i][j] / d[j]).abs();
                    col += (d[j] * a[j][i] / d[i]).abs();
                }
            }
            if row == 0.0 || col == 0.0 {
                continue;
            }
            // Scaling d_i by f multiplies the row sum by f and divides the
            // column sum by f.
            let mut f = 1.0_f64;
            while row * f < col / f / 2.0 {
                f *= 2.0;
            }
            while row * f > 2.0 * col / f {
                f /= 2.0;
            }
            if f != 1.0 {
                d[i] *= f;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if !d.iter().all(|x| x.is_finite() && *x > 0.0) {
        return Err(invalid("balancing produced an unusable metric"));
    }
    Ok(d)
}

/// The largest number of steps a stepped certificate takes; beyond it the
/// certificate is [`NonnormalBoundStatus::Unbounded`].
pub const MAX_STEPS: usize = 100_000;

/// A stepped certificate (review DAG node REV-02): `N = steps` steps of
/// `h = tau / N`, each a degree-`m` Taylor step `x_(j+1) = mid(P_j)` with
/// `P_j` the interval Horner enclosure of `p_m(h B) x_j`, `B = D A D^-1`.
/// The error in `B` coordinates is bounded by
/// `(1 + sqrt 2) [e^{N h a_hi} ||rad(D v)|| + sum_j e^{(N-1-j) h a_hi}
/// (t_j + rho_j)]`, Crouzeix-Palencia applied once per term, with the local
/// truncation `t_j = (1 + sqrt 2) S_h ||x_j||` and rounding
/// `rho_j = ||rad P_j||`; it is transported by `||D^-1||_2` and the
/// back-transform rounding is added. Certifies the method's own value.
pub fn certify_exp_action_stepped(
    a: &[Vec<f64>],
    v: &[f64],
    tau: f64,
    metric: Option<&[f64]>,
    degree: usize,
    steps: usize,
) -> CoreResult<NonnormalExpCertificate> {
    stepped_with_propagation(a, v, tau, metric, degree, steps, None)
}

/// The stepped certificate with either propagation: `None` is REV-02's
/// Crouzeix-Palencia factor with the numerical-range box (unchanged), and
/// `Some(mu)` is `||e^{tB}||_2 <= e^{t mu}` for a verified
/// `mu >= lambda_max((B + B^T)/2)` (RVJ DAG node PP12), which needs no
/// factor on propagation.
fn stepped_with_propagation(
    a: &[Vec<f64>],
    v: &[f64],
    tau: f64,
    metric: Option<&[f64]>,
    degree: usize,
    steps: usize,
    lognorm: Option<f64>,
) -> CoreResult<NonnormalExpCertificate> {
    // Validation, B and Omega as in the single-step certificate.
    let probe = certify_exp_action(a, v, 0.0, metric, degree, None)?;
    if steps == 0 {
        return Err(invalid("steps must be positive"));
    }
    if !(tau.is_finite() && tau >= 0.0) {
        return Err(invalid("tau must be finite and nonnegative"));
    }
    let n = v.len();
    let ones = vec![1.0; n];
    let d = metric.unwrap_or(&ones);
    let range = probe.numerical_range;
    let unbounded = |candidate: Vec<f64>| NonnormalExpCertificate {
        status: NonnormalBoundStatus::Unbounded,
        candidate,
        error_upper: f64::INFINITY,
        error_lower: 0.0,
        truncation_upper: f64::INFINITY,
        distance_upper: f64::INFINITY,
        distance_lower: 0.0,
        transport: probe.transport,
        numerical_range: range,
        degree,
    };
    if steps > MAX_STEPS {
        return Ok(unbounded(vec![f64::NAN; n]));
    }
    let mut b = vec![vec![Interval::point(0.0)?; n]; n];
    for i in 0..n {
        for j in 0..n {
            b[i][j] = Interval::point(d[i])?
                .mul(Interval::point(a[i][j])?)?
                .div(Interval::point(d[j])?)?;
        }
    }
    let h = tau / steps as f64;
    // The method integrates N steps of exactly this binary64 h; a (tau, N)
    // with N h != tau is refused rather than bounded.
    // N h = tau exactly in the reals: the FMA residual N h - tau is exact
    // unless it underflows, and a nonzero exact residual never rounds to 0.
    if h.mul_add(steps as f64, -tau) != 0.0 {
        return Err(invalid(
            "tau / steps is not exact; choose steps so that it is",
        ));
    }
    let local = match truncation_upper(range, h, degree) {
        Ok(s) => mul_up(add_up(1.0, sqrt_up(2.0)?)?, s)?,
        Err(_) => return Ok(unbounded(vec![f64::NAN; n])),
    };
    let cp = add_up(1.0, sqrt_up(2.0)?)?;
    let u = (0..n)
        .map(|i| Interval::point(d[i])?.mul(Interval::point(v[i])?))
        .collect::<CoreResult<Vec<_>>>()?;
    let radius_norm = |w: &[Interval]| -> CoreResult<f64> {
        let mut total = 0.0;
        for x in w {
            let (_, r) = midpoint_radius(*x)?;
            total = add_up(total, mul_up(r, r)?)?;
        }
        sqrt_up(total)
    };
    let point_norm = |w: &[f64]| -> CoreResult<f64> {
        let mut total = 0.0;
        for x in w {
            total = add_up(total, mul_up(*x, *x)?)?;
        }
        sqrt_up(total)
    };
    let rate = lognorm.unwrap_or(range.re_hi);
    let decay = |count: usize| decay_upper(count, h, rate);
    let mut x: Vec<f64> = u
        .iter()
        .map(|w| midpoint_radius(*w).map(|(m, _)| m))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut total = mul_up(decay(steps)?, radius_norm(&u)?)?;
    for j in 0..steps {
        let xi = x
            .iter()
            .map(|value| Interval::point(*value))
            .collect::<CoreResult<Vec<_>>>()?;
        let mut w = xi.clone();
        for k in (1..=degree).rev() {
            let factor = Interval::point(h)?.div(Interval::point(k as f64)?)?;
            let mut next = Vec::with_capacity(n);
            for i in 0..n {
                let mut bw = Interval::point(0.0)?;
                for (bik, wk) in b[i].iter().zip(&w) {
                    bw = bw.add(bik.mul(*wk)?)?;
                }
                next.push(xi[i].add(factor.mul(bw)?)?);
            }
            w = next;
        }
        let truncation = mul_up(local, point_norm(&x)?)?;
        let rounding = radius_norm(&w)?;
        total = add_up(
            total,
            mul_up(decay(steps - 1 - j)?, add_up(truncation, rounding)?)?,
        )?;
        x = w
            .iter()
            .map(|p| midpoint_radius(*p).map(|(m, _)| m))
            .collect::<CoreResult<Vec<_>>>()?;
        if !x.iter().all(|value| value.is_finite()) || !total.is_finite() {
            return Ok(unbounded(x));
        }
    }
    let error_b = if lognorm.is_some() {
        total
    } else {
        mul_up(cp, total)?
    };
    let inv_max = d
        .iter()
        .map(|value| div_up(1.0, *value))
        .collect::<CoreResult<Vec<_>>>()?
        .into_iter()
        .fold(0.0_f64, f64::max);
    let back = (0..n)
        .map(|i| Interval::point(x[i])?.div(Interval::point(d[i])?))
        .collect::<CoreResult<Vec<_>>>()?;
    let candidate: Vec<f64> = back.iter().map(|p| 0.5 * p.lo + 0.5 * p.hi).collect();
    let mut back_sq = 0.0;
    for (c, p) in candidate.iter().zip(&back) {
        let gap = Interval::point(*c)?.sub(*p)?.mag();
        back_sq = add_up(back_sq, mul_up(gap, gap)?)?;
    }
    let distance = sqrt_up(back_sq)?;
    let error_upper = add_up(mul_up(inv_max, error_b)?, distance)?;
    Ok(NonnormalExpCertificate {
        status: if error_upper.is_finite() {
            NonnormalBoundStatus::Bounded
        } else {
            NonnormalBoundStatus::Unbounded
        },
        candidate,
        error_upper,
        error_lower: 0.0,
        truncation_upper: mul_up(inv_max, error_b)?,
        distance_upper: distance,
        distance_lower: 0.0,
        transport: probe.transport,
        numerical_range: range,
        degree,
    })
}

/// Upper bound on `e^{count h a}` for the exact real `count * h` (RVJ DAG
/// node SAFE-ENCLOSURE, `research/safe_enclosure_composition_20261004`):
/// the exponent is the upper end of the interval product
/// `[mul_down(count, h), mul_up(count, h)] * a`, so a negative `a` takes the
/// lower time. Capped at 1 for `a <= 0`.
pub fn decay_upper(count: usize, h: f64, a: f64) -> CoreResult<f64> {
    let time = Interval::new(mul_down(count as f64, h)?, mul_up(count as f64, h)?)?;
    let exponent = time.mul(Interval::point(a)?)?.hi;
    if a <= 0.0 {
        // e^{t a} <= 1 for a <= 0; keep the exact decay when it is finite.
        Ok(exp_interval(exponent).map(|e| e.hi).unwrap_or(1.0).min(1.0))
    } else {
        Ok(exp_interval(exponent)?.hi)
    }
}

/// The rounded midpoint `m = 0.5 lo + 0.5 hi` of `x` and the outward
/// distance `max(m - lo, hi - m)` from it to the far endpoint (RVJ DAG node
/// SAFE-ENCLOSURE): the half width alone misses the rounding of `m`.
pub fn midpoint_radius(x: Interval) -> CoreResult<(f64, f64)> {
    let m = 0.5 * x.lo + 0.5 * x.hi;
    if !m.is_finite() {
        return Err(invalid("interval midpoint is not finite"));
    }
    Ok((m, sub_up(m, x.lo)?.max(sub_up(x.hi, m)?)))
}

/// The step rule of REV-02: `N = max(1, ceil(tau R))` with `R` the largest
/// modulus on `Omega`, rounded up to a power of two so that `tau / N` is
/// exact when `tau` is.
pub fn stepping_rule(range: NumericalRangeBox, tau: f64) -> usize {
    let re = range.re_lo.abs().max(range.re_hi.abs());
    let r = (re * re + range.im * range.im).sqrt() * (1.0 + 4.0 * f64::EPSILON);
    let needed = (tau * r).ceil().max(1.0);
    if !(needed.is_finite()) || needed > MAX_STEPS as f64 {
        return MAX_STEPS + 1;
    }
    (needed as usize).next_power_of_two()
}

/// Which metric an automatic certificate used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutoMetric {
    Identity,
    Osborne,
}

/// Which metric a log-norm certificate used (PP12, PP12b).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LognormMetric {
    Identity,
    Osborne,
    /// The tridiagonal chain symmetrizer (RVJ DAG node PP12b).
    ChainSymmetrizer,
}

/// The automatic certificate of REV-02: the stepped certificate (degree
/// 20, [`stepping_rule`]) under the identity and under
/// [`osborne_metric`], the one with the smaller `error_upper` (both are
/// valid bounds). Returns both, the chosen one first.
pub fn certify_exp_action_auto(
    a: &[Vec<f64>],
    v: &[f64],
    tau: f64,
) -> CoreResult<(
    AutoMetric,
    NonnormalExpCertificate,
    NonnormalExpCertificate,
    [usize; 2],
)> {
    let osborne = osborne_metric(a)?;
    let mut out = Vec::new();
    for metric in [None, Some(osborne.as_slice())] {
        let probe = certify_exp_action(a, v, 0.0, metric, 20, None)?;
        let steps = stepping_rule(probe.numerical_range, tau);
        let cert = certify_exp_action_stepped(a, v, tau, metric, 20, steps)?;
        out.push((cert, steps));
    }
    let (osb, osb_steps) = out.pop().unwrap();
    let (ident, ident_steps) = out.pop().unwrap();
    if osb.error_upper < ident.error_upper {
        Ok((AutoMetric::Osborne, osb, ident, [osb_steps, ident_steps]))
    } else {
        Ok((AutoMetric::Identity, ident, osb, [ident_steps, osb_steps]))
    }
}

/// A stepped certificate with log-norm propagation (RVJ DAG node PP12,
/// research node `research/pp12_lognorm_decay_20261004`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LognormCertificate {
    pub certificate: NonnormalExpCertificate,
    /// Verified `mu >= lambda_max((B + B^T)/2)` used for propagation.
    pub mu_up: f64,
    /// `interval-cholesky` or the `gershgorin` fallback.
    pub mu_source: &'static str,
    /// REV-02's propagation rate (numerical-range box), for comparison.
    pub gershgorin_re_hi: f64,
    pub metric: LognormMetric,
    pub steps: usize,
}

#[allow(clippy::needless_range_loop, clippy::neg_cmp_op_on_partial_ord)]
fn metric_matrix(a: &[Vec<f64>], d: &[f64]) -> CoreResult<Vec<Vec<Interval>>> {
    let n = a.len();
    let mut b = vec![vec![Interval::point(0.0)?; n]; n];
    for i in 0..n {
        for j in 0..n {
            b[i][j] = Interval::point(d[i])?
                .mul(Interval::point(a[i][j])?)?
                .div(Interval::point(d[j])?)?;
        }
    }
    Ok(b)
}

fn interval_square(x: Interval) -> CoreResult<Interval> {
    if x.contains_zero() {
        Interval::new(0.0, mul_up(x.mag(), x.mag())?)
    } else {
        Interval::new(mul_down(x.mig(), x.mig())?, mul_up(x.mag(), x.mag())?)
    }
}

/// Whether the interval Cholesky factorization of `mu I - S` is feasible:
/// then every symmetric matrix in `S` has all eigenvalues below `mu`
/// (Alefeld and Mayer: feasibility of the interval Cholesky method implies
/// positive definiteness of every symmetric member).
#[allow(clippy::needless_range_loop, clippy::neg_cmp_op_on_partial_ord)]
fn interval_cholesky_feasible(mu: f64, s: &[Vec<Interval>]) -> CoreResult<bool> {
    let n = s.len();
    let mut l = vec![vec![Interval::point(0.0)?; n]; n];
    for k in 0..n {
        let mut d = Interval::point(mu)?.sub(s[k][k])?;
        for j in 0..k {
            d = d.sub(interval_square(l[k][j])?)?;
        }
        if !(d.lo > 0.0) {
            return Ok(false);
        }
        let root = Interval::new(sqrt_down(d.lo)?, sqrt_up(d.hi)?)?;
        l[k][k] = root;
        for i in k + 1..n {
            let mut x = -s[i][k];
            for j in 0..k {
                x = x.sub(l[i][j].mul(l[k][j])?)?;
            }
            l[i][k] = x.div(root)?;
        }
    }
    Ok(true)
}

/// Largest eigenvalue of a symmetric binary64 matrix by cyclic Jacobi: an
/// estimate only, used to start the verified search.
#[allow(clippy::needless_range_loop, clippy::neg_cmp_op_on_partial_ord)]
fn jacobi_max_eigenvalue(m: &[Vec<f64>]) -> f64 {
    let n = m.len();
    let mut a = m.to_vec();
    for _ in 0..100 {
        let off: f64 = (0..n)
            .flat_map(|i| (0..n).filter(move |j| *j != i).map(move |j| (i, j)))
            .map(|(i, j)| a[i][j] * a[i][j])
            .sum();
        if off < 1e-30 {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if a[p][q] == 0.0 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let sn = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - sn * akq;
                    a[k][q] = sn * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - sn * aqk;
                    a[q][k] = sn * apk + c * aqk;
                }
            }
        }
    }
    (0..n).map(|i| a[i][i]).fold(f64::NEG_INFINITY, f64::max)
}

/// A verified `mu >= lambda_max` of the symmetric part of the interval
/// matrix `b`: the binary64 Jacobi estimate raised by `1e-12 (1 + |est|)`
/// times 10^k (k < 40) until the interval Cholesky of `mu I - S` is
/// feasible; otherwise the symmetric-part Gershgorin row bound.
#[allow(clippy::needless_range_loop, clippy::neg_cmp_op_on_partial_ord)]
pub fn symmetric_part_upper(b: &[Vec<Interval>]) -> CoreResult<(f64, &'static str)> {
    let n = b.len();
    let half = Interval::point(0.5)?;
    let mut s = vec![vec![Interval::point(0.0)?; n]; n];
    for i in 0..n {
        for j in 0..n {
            s[i][j] = b[i][j].add(b[j][i])?.mul(half)?;
        }
    }
    let mut gershgorin = f64::NEG_INFINITY;
    for i in 0..n {
        let mut row = s[i][i].hi;
        for j in 0..n {
            if j != i {
                row = add_up(row, s[i][j].mag())?;
            }
        }
        gershgorin = gershgorin.max(row);
    }
    let mid: Vec<Vec<f64>> = s
        .iter()
        .map(|row| row.iter().map(|x| 0.5 * x.lo + 0.5 * x.hi).collect())
        .collect();
    let estimate = jacobi_max_eigenvalue(&mid);
    if estimate.is_finite() {
        let mut offset = 1e-12 * (1.0 + estimate.abs());
        for _ in 0..40 {
            let mu = estimate + offset;
            if mu >= gershgorin {
                break;
            }
            if interval_cholesky_feasible(mu, &s)? {
                return Ok((mu, "interval-cholesky"));
            }
            offset *= 10.0;
        }
    }
    Ok((gershgorin, "gershgorin"))
}

/// The stepped certificate (same steps, degree and enclosures as REV-02)
/// with propagation `e^{t mu_up}`.
pub fn certify_exp_action_lognorm(
    a: &[Vec<f64>],
    v: &[f64],
    tau: f64,
    metric: Option<&[f64]>,
    degree: usize,
    steps: usize,
) -> CoreResult<LognormCertificate> {
    let label = if metric.is_some() {
        LognormMetric::Osborne
    } else {
        LognormMetric::Identity
    };
    lognorm_labelled(a, v, tau, metric, degree, steps, label)
}

#[allow(clippy::too_many_arguments)]
fn lognorm_labelled(
    a: &[Vec<f64>],
    v: &[f64],
    tau: f64,
    metric: Option<&[f64]>,
    degree: usize,
    steps: usize,
    label: LognormMetric,
) -> CoreResult<LognormCertificate> {
    let probe = certify_exp_action(a, v, 0.0, metric, degree, None)?;
    let ones = vec![1.0; v.len()];
    let b = metric_matrix(a, metric.unwrap_or(&ones))?;
    let (mu_up, mu_source) = symmetric_part_upper(&b)?;
    let certificate = stepped_with_propagation(a, v, tau, metric, degree, steps, Some(mu_up))?;
    Ok(LognormCertificate {
        certificate,
        mu_up,
        mu_source,
        gershgorin_re_hi: probe.numerical_range.re_hi,
        metric: label,
        steps,
    })
}

/// [`certify_exp_action_auto`] with log-norm propagation: identity and
/// Osborne metrics, the REV-02 step rule and degree 20; returns the smaller
/// bound first (both are valid).
pub fn certify_exp_action_lognorm_auto(
    a: &[Vec<f64>],
    v: &[f64],
    tau: f64,
) -> CoreResult<(LognormCertificate, LognormCertificate)> {
    let osborne = osborne_metric(a)?;
    let mut out = Vec::new();
    for metric in [None, Some(osborne.as_slice())] {
        let probe = certify_exp_action(a, v, 0.0, metric, 20, None)?;
        let steps = stepping_rule(probe.numerical_range, tau);
        out.push(certify_exp_action_lognorm(a, v, tau, metric, 20, steps)?);
    }
    let osb = out.pop().unwrap();
    let ident = out.pop().unwrap();
    if osb.certificate.error_upper < ident.certificate.error_upper {
        Ok((osb, ident))
    } else {
        Ok((ident, osb))
    }
}

/// The chain symmetrizer of the tridiagonal part (RVJ DAG node PP12b,
/// `research/pp12b_chain_symmetrizer_20261004`): `d_1 = 1` and `d_i =
/// d_{i-1} sqrt(|a_{i-1,i}| / |a_{i,i-1}|)` when both are nonzero, else
/// `d_{i-1}`. Then `|b_{i,i-1}| = |b_{i-1,i}|`, so opposite-sign pairs
/// leave the symmetric part. Real valued; the certificate encloses
/// `D A D^-1` in interval arithmetic.
pub fn chain_symmetrizer_metric(a: &[Vec<f64>]) -> CoreResult<Vec<f64>> {
    let n = a.len();
    if n == 0 || a.iter().any(|row| row.len() != n) {
        return Err(CoreError::Dimension(
            "nonnormal certificate: A must be square".into(),
        ));
    }
    let mut d = vec![1.0_f64; n];
    for i in 1..n {
        let (up, low) = (a[i - 1][i].abs(), a[i][i - 1].abs());
        d[i] = if up > 0.0 && low > 0.0 {
            d[i - 1] * (up / low).sqrt()
        } else {
            d[i - 1]
        };
        if !(d[i].is_finite() && d[i] > 0.0) {
            return Err(invalid("chain symmetrizer out of range"));
        }
    }
    Ok(d)
}

/// [`certify_exp_action_lognorm_auto`] with the chain symmetrizer as a
/// third metric (PP12b): all three certificates, the smallest bound first.
pub fn certify_exp_action_lognorm_auto3(
    a: &[Vec<f64>],
    v: &[f64],
    tau: f64,
) -> CoreResult<Vec<LognormCertificate>> {
    let osborne = osborne_metric(a)?;
    let chain = chain_symmetrizer_metric(a)?;
    let mut out = Vec::new();
    for (metric, label) in [
        (None, LognormMetric::Identity),
        (Some(osborne.as_slice()), LognormMetric::Osborne),
        (Some(chain.as_slice()), LognormMetric::ChainSymmetrizer),
    ] {
        let probe = certify_exp_action(a, v, 0.0, metric, 20, None)?;
        let steps = stepping_rule(probe.numerical_range, tau);
        out.push(lognorm_labelled(a, v, tau, metric, 20, steps, label)?);
    }
    out.sort_by(|x, y| {
        x.certificate
            .error_upper
            .total_cmp(&y.certificate.error_upper)
    });
    Ok(out)
}
