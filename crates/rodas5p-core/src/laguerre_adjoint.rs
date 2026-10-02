//! Signed output-adjoint bound of the Laguerre recurrence error (research
//! node `research/thread_transfer_laguerre_adjoint_20261002`, thread-transfer
//! DAG node P1-LAGUERRE-CERT). Research only.
//!
//! The Laguerre action computes `t_hat_n` by
//! `t_hat_(n+1) = a_n(X) t_hat_n - n/(n+1) t_hat_(n-1) + delta_(n+1)` with
//! `a_n(x) = (2n+1 - x)/(n+1)` and exact local defects `delta_j`, and outputs
//! `sum_n c_n t_hat_n` for the stored (binary64) coefficients `c_n`. With
//!
//! ```text
//! z_(m+1) = z_(m+2) = 0,
//! z_j = c_j + a_j(x) z_(j+1) - (j+1)/(j+2) z_(j+2),
//! ```
//!
//! summation by parts gives exactly
//! `sum_n c_n (t_hat_n - t_n) = sum_{j=1}^m z_j(X) delta_j`, and for a
//! symmetric `X` with spectrum in `[0, L']`, `||z_j(X)|| <= sup_[0, L'] |z_j|`.
//! So `sum_j beta_j eps_j`, with `beta_j >= sup |z_j|` and
//! `eps_j >= ||delta_j||`, bounds the recurrence error of the finite stored
//! polynomial. Unlike the scalar majorant it keeps the signs of `c_n` and the
//! three-term cancellation.
//!
//! `beta_j` comes from Bernstein coefficients on `[0, L']`: the backward
//! recurrence runs in the Bernstein basis (degree elevation and the product
//! with `a_j` are convex combinations, so no monomial cancellation), in
//! outward interval arithmetic, and the range of a polynomial lies in the
//! hull of its coefficients on every piece of a de Casteljau subdivision.
//!
//! This bounds one error component. Truncation, coefficient enclosure,
//! summation and normalization are separate components; the total stays
//! `EstimateOnly` in `polynomial_action` until a complete native total is
//! verified.

use crate::{
    CoreError, CoreResult,
    directed::{Interval, add_up, mul_up},
};

/// Largest degree for which envelopes are computed (a policy: the setup is
/// `O(m^2 2^depth)` interval operations per polynomial).
pub const LAGUERRE_ADJOINT_DEGREE_LIMIT: usize = 128;
/// De Casteljau halving depth (`2^depth` pieces).
pub const LAGUERRE_ADJOINT_DEPTH: usize = 3;

type Bernstein = Vec<Interval>;

/// Interval operations of an envelope setup (each `+ - * /` or scaling of
/// an interval counts one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Ops(u64);

impl Ops {
    fn tick(&mut self, count: u64) {
        self.0 += count;
    }
}

fn point(value: f64) -> CoreResult<Interval> {
    Interval::point(value)
}

fn ratio(numerator: f64, denominator: f64, ops: &mut Ops) -> CoreResult<Interval> {
    ops.tick(1);
    point(numerator)?.div(point(denominator)?)
}

/// Degree elevation by one: `e_k = k/(d+1) b_(k-1) + (d+1-k)/(d+1) b_k`.
fn elevate(b: &[Interval], ops: &mut Ops) -> CoreResult<Bernstein> {
    let d = b.len() - 1;
    let n = (d + 1) as f64;
    let mut out = Vec::with_capacity(d + 2);
    for k in 0..=d + 1 {
        let mut value = point(0.0)?;
        if k > 0 {
            value = value.add(ratio(k as f64, n, ops)?.mul(b[k - 1])?)?;
            ops.tick(2);
        }
        if k <= d {
            value = value.add(ratio((d + 1 - k) as f64, n, ops)?.mul(b[k])?)?;
            ops.tick(2);
        }
        out.push(value);
    }
    Ok(out)
}

fn elevate_to(mut b: Bernstein, degree: usize, ops: &mut Ops) -> CoreResult<Bernstein> {
    while b.len() - 1 < degree {
        b = elevate(&b, ops)?;
    }
    Ok(b)
}

/// `(alpha - g t) p(t)` for `t in [0, 1]`, degree `d + 1`:
/// `alpha e_k - g k/(d+1) b_(k-1)` with `e` the elevation of `b`.
fn mul_affine(
    b: &[Interval],
    alpha: Interval,
    g: Interval,
    ops: &mut Ops,
) -> CoreResult<Bernstein> {
    let d = b.len() - 1;
    let n = (d + 1) as f64;
    let elevated = elevate(b, ops)?;
    let mut out = Vec::with_capacity(d + 2);
    for (k, e) in elevated.iter().enumerate() {
        let mut value = alpha.mul(*e)?;
        ops.tick(1);
        if k > 0 {
            value = value.sub(g.mul(ratio(k as f64, n, ops)?)?.mul(b[k - 1])?)?;
            ops.tick(3);
        }
        out.push(value);
    }
    Ok(out)
}

fn add(p: Bernstein, q: Bernstein, ops: &mut Ops) -> CoreResult<Bernstein> {
    let degree = (p.len() - 1).max(q.len() - 1);
    let (p, q) = (elevate_to(p, degree, ops)?, elevate_to(q, degree, ops)?);
    ops.tick(p.len() as u64);
    p.iter().zip(&q).map(|(a, b)| a.add(*b)).collect()
}

/// De Casteljau halving at `t = 1/2`.
fn split(b: &[Interval], ops: &mut Ops) -> CoreResult<(Bernstein, Bernstein)> {
    let mut row = b.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![row[row.len() - 1]];
    while row.len() > 1 {
        ops.tick(2 * (row.len() as u64 - 1));
        row = row
            .windows(2)
            .map(|w| w[0].add(w[1])?.scale(0.5))
            .collect::<CoreResult<Vec<_>>>()?;
        left.push(row[0]);
        right.push(row[row.len() - 1]);
    }
    right.reverse();
    Ok((left, right))
}

/// `max |coefficient|` over the `2^depth` pieces: an upper bound of `|p|` on
/// `[0, 1]` by the convex-hull property.
fn bound(b: &[Interval], depth: usize, ops: &mut Ops) -> CoreResult<f64> {
    if depth == 0 {
        return Ok(b.iter().map(Interval::mag).fold(0.0, f64::max));
    }
    let (left, right) = split(b, ops)?;
    Ok(bound(&left, depth - 1, ops)?.max(bound(&right, depth - 1, ops)?))
}

fn envelopes_impl(
    coefficients: &[Interval],
    extent: f64,
    depth: usize,
    ops: &mut Ops,
) -> CoreResult<Vec<f64>> {
    if coefficients.is_empty() || coefficients.len() - 1 > LAGUERRE_ADJOINT_DEGREE_LIMIT {
        return Err(CoreError::InvalidInput(format!(
            "LAGUERRE_ADJOINT_UNSUPPORTED: degree must be in 0..={LAGUERRE_ADJOINT_DEGREE_LIMIT}"
        )));
    }
    if !(extent.is_finite()
        && extent > 0.0
        && coefficients
            .iter()
            .all(|c| c.lo.is_finite() && c.hi.is_finite()))
    {
        return Err(CoreError::InvalidInput(
            "LAGUERRE_ADJOINT_UNSUPPORTED: the extent must be finite and positive, the coefficients finite"
                .into(),
        ));
    }
    let m = coefficients.len() - 1;
    let zero = vec![point(0.0)?];
    // z[j] for j = 0 ..= m + 2.
    let mut z: Vec<Bernstein> = vec![zero.clone(); m + 3];
    for j in (0..=m).rev() {
        let jf = j as f64;
        let alpha = ratio(2.0 * jf + 1.0, jf + 1.0, ops)?;
        // x = extent t, so a_j = alpha - (extent / (j+1)) t.
        let g = ratio(extent, jf + 1.0, ops)?;
        let linear = mul_affine(&z[j + 1], alpha, g, ops)?;
        let damping = ratio(jf + 1.0, jf + 2.0, ops)?;
        ops.tick(z[j + 2].len() as u64);
        let damped = z[j + 2]
            .iter()
            .map(|v| damping.mul(*v).map(|x| -x))
            .collect::<CoreResult<Vec<_>>>()?;
        z[j] = add(add(vec![coefficients[j]], linear, ops)?, damped, ops)?;
    }
    let envelopes = z[..=m]
        .iter()
        .map(|p| bound(p, depth, ops))
        .collect::<CoreResult<Vec<_>>>()?;
    if !envelopes.iter().all(|b| b.is_finite()) {
        return Err(CoreError::NonFinite(
            "LAGUERRE_ADJOINT_UNSUPPORTED: an envelope is not finite".into(),
        ));
    }
    Ok(envelopes)
}

/// `beta_0 .. beta_m` with `beta_j >= sup_{x in [0, extent]} |z_j(x)|` for
/// the stored coefficients `c_0 .. c_m` (taken as exact reals).
pub fn laguerre_adjoint_envelopes(
    stored: &[f64],
    extent: f64,
    depth: usize,
) -> CoreResult<Vec<f64>> {
    Ok(laguerre_adjoint_envelopes_counted(stored, extent, depth)?.0)
}

/// [`laguerre_adjoint_envelopes`] and the number of interval operations of
/// the setup (thread-transfer node P2-LAGUERRE-CACHE).
pub fn laguerre_adjoint_envelopes_counted(
    stored: &[f64],
    extent: f64,
    depth: usize,
) -> CoreResult<(Vec<f64>, u64)> {
    let coefficients = stored
        .iter()
        .map(|c| {
            if c.is_finite() {
                point(*c)
            } else {
                Err(CoreError::InvalidInput(
                    "LAGUERRE_ADJOINT_UNSUPPORTED: the coefficients must be finite".into(),
                ))
            }
        })
        .collect::<CoreResult<Vec<_>>>()?;
    laguerre_adjoint_envelopes_interval(&coefficients, extent, depth)
}

/// The envelopes for coefficients known only as intervals (each exact
/// coefficient lies in its interval), e.g. an exact-real combination
/// `sum_k s_k c_(n,k)`: every `z_j` is enclosed for every choice of the
/// coefficients, so `beta_j` bounds them all. Also returns the interval
/// operation count.
pub fn laguerre_adjoint_envelopes_interval(
    coefficients: &[Interval],
    extent: f64,
    depth: usize,
) -> CoreResult<(Vec<f64>, u64)> {
    let mut ops = Ops::default();
    let envelopes = envelopes_impl(coefficients, extent, depth, &mut ops)?;
    Ok((envelopes, ops.0))
}

/// What the envelopes depend on, and nothing else: the degree, the extent
/// and depth bits, the coefficient interval bits and the proof version.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EnvelopeKey {
    pub degree: usize,
    pub extent_bits: u64,
    pub depth: usize,
    pub coefficients_sha256: String,
    pub proof_version: &'static str,
}

/// Version of the envelope construction; a change of the proof or the
/// arithmetic changes it and so misses every cached entry.
pub const LAGUERRE_ADJOINT_PROOF_VERSION: &str = "laguerre-adjoint-bernstein-interval-v1";

impl EnvelopeKey {
    pub fn new(
        coefficients: &[Interval],
        extent: f64,
        depth: usize,
        proof_version: &'static str,
    ) -> Self {
        let text = coefficients
            .iter()
            .map(|c| format!("{:016x}:{:016x}", c.lo.to_bits(), c.hi.to_bits()))
            .collect::<Vec<_>>()
            .join(",");
        Self {
            degree: coefficients.len().saturating_sub(1),
            extent_bits: extent.to_bits(),
            depth,
            coefficients_sha256: crate::sha256_hex(text.as_bytes()),
            proof_version,
        }
    }
}

/// A cache of envelopes keyed by [`EnvelopeKey`], with hit and miss counts.
/// Reuse is valid only because the envelopes are a function of the key: the
/// operator and the vectors do not enter them (the domain check of the
/// operator stays with the caller).
#[derive(Clone, Debug, Default)]
pub struct LaguerreEnvelopeCache {
    entries: std::collections::HashMap<EnvelopeKey, Vec<f64>>,
    pub hits: u64,
    pub misses: u64,
}

impl LaguerreEnvelopeCache {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The envelopes, whether they were cached, and the interval operations
    /// spent (zero on a hit).
    pub fn envelopes(
        &mut self,
        coefficients: &[Interval],
        extent: f64,
        depth: usize,
    ) -> CoreResult<(Vec<f64>, bool, u64)> {
        let key = EnvelopeKey::new(coefficients, extent, depth, LAGUERRE_ADJOINT_PROOF_VERSION);
        if let Some(found) = self.entries.get(&key) {
            self.hits += 1;
            return Ok((found.clone(), true, 0));
        }
        let (envelopes, ops) = laguerre_adjoint_envelopes_interval(coefficients, extent, depth)?;
        self.misses += 1;
        self.entries.insert(key, envelopes.clone());
        Ok((envelopes, false, ops))
    }
}

/// `sum_{j=1}^m beta_j eps_(j-1)`, rounded upward, where `local[n]` bounds
/// the exact local residual of the step that produced `t_hat_(n+1)`.
pub fn laguerre_adjoint_recurrence_bound(
    stored: &[f64],
    extent: f64,
    local: &[f64],
    depth: usize,
) -> CoreResult<f64> {
    if local.len() + 1 != stored.len() || !local.iter().all(|e| e.is_finite() && *e >= 0.0) {
        return Err(CoreError::InvalidInput(
            "LAGUERRE_ADJOINT_UNSUPPORTED: one finite nonnegative residual per step".into(),
        ));
    }
    let envelopes = laguerre_adjoint_envelopes(stored, extent, depth)?;
    let mut total = 0.0;
    for (beta, eps) in envelopes[1..].iter().zip(local) {
        total = add_up(total, mul_up(*beta, *eps)?)?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `z_j(x)` by the scalar backward recurrence.
    fn scalar(stored: &[f64], x: f64) -> Vec<f64> {
        let m = stored.len() - 1;
        let mut z = vec![0.0; m + 3];
        for j in (0..=m).rev() {
            let jf = j as f64;
            z[j] = stored[j] + (2.0 * jf + 1.0 - x) / (jf + 1.0) * z[j + 1]
                - (jf + 1.0) / (jf + 2.0) * z[j + 2];
        }
        z.truncate(m + 1);
        z
    }

    #[test]
    fn envelopes_cover_the_adjoint_polynomials() {
        let stored = (0..=24)
            .map(|n| if n % 3 == 0 { -0.7 } else { 0.25 } * 0.9_f64.powi(n))
            .collect::<Vec<_>>();
        for extent in [1.0, 4.0, 16.0] {
            let beta = laguerre_adjoint_envelopes(&stored, extent, 3).unwrap();
            for i in 0..=400 {
                let x = extent * i as f64 / 400.0;
                for (j, value) in scalar(&stored, x).iter().enumerate() {
                    assert!(
                        value.abs() <= beta[j] * (1.0 + 1.0e-9) + 1.0e-12,
                        "j={j} x={x}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_constant_coefficient_gives_its_magnitude() {
        // m = 0: z_0 = c_0.
        assert_eq!(
            laguerre_adjoint_envelopes(&[-3.0], 2.0, 3).unwrap(),
            vec![3.0]
        );
        // Invalid inputs are refused.
        assert!(laguerre_adjoint_envelopes(&[], 1.0, 3).is_err());
        assert!(laguerre_adjoint_envelopes(&[1.0], 0.0, 3).is_err());
        assert!(laguerre_adjoint_envelopes(&[f64::NAN], 1.0, 3).is_err());
        assert!(laguerre_adjoint_envelopes(&vec![1.0; 130], 1.0, 3).is_err());
        assert!(laguerre_adjoint_recurrence_bound(&[1.0, 1.0], 1.0, &[], 3).is_err());
        assert!(laguerre_adjoint_recurrence_bound(&[1.0, 1.0], 1.0, &[-1.0], 3).is_err());
    }
}
