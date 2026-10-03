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
        Interval, add_down, add_up, div_up, mul_down, mul_up, sqrt_down, sqrt_up, sub_down,
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
        // exp rounded up by a relative margin of 4 eps (libm exp is within
        // 1 ulp); overflow gives infinity, which the caller treats as no bound.
        let e = growth_exponent.exp();
        if !e.is_finite() {
            return Err(invalid("growth factor overflows"));
        }
        mul_up(e, 1.0 + 4.0 * f64::EPSILON)?
    };
    let bound = mul_up(term, growth)?;
    if bound.is_finite() {
        Ok(bound)
    } else {
        Err(invalid("truncation bound overflows"))
    }
}
