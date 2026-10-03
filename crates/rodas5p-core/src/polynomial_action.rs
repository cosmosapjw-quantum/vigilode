//! Joint Chebyshev and Laguerre phi actions on a declared symmetric
//! nonpositive domain (re-audit R3 of 2026-10-01, POLY-01 and POLY-02).
//!
//! Research only: no integrator calls this module.
//!
//! The action is `F = sum_{k=0}^{4} phi_k(h A) w_k` for **given** vectors
//! `w_k` (`phi_0 = exp`). The scaled convention `w_k = h^k b_k` is a separate
//! transform with its own loss status ([`crate::weight_phi_vectors`]).
//!
//! # Operator domain
//!
//! `A` is a dense, exactly symmetric matrix with a spectral enclosure
//! `spec(A) in [-rho, -lambda]`, `0 <= lambda <= rho`
//! ([`SymmetricNonpositiveOperator`]). The enclosure is either verified
//! (Gershgorin discs computed with outward rounding lie inside it) or
//! declared by the caller and recorded as such. A non-symmetric matrix, a
//! diagonal entry outside the enclosure (impossible for a true enclosure,
//! since diagonal entries are Rayleigh quotients) or a degenerate enclosure
//! of a non-scalar matrix is rejected with
//! [`POLYNOMIAL_DOMAIN_UNSUPPORTED`].
//!
//! # Coefficients
//!
//! With `s` and `d` the binary64 shift and half-width, `X = (A + s I) / d`
//! (exact reals), `a = -h s` and `b = h d`, `h A = a I + b X` exactly, and
//! `phi_k(h A) = sum_n c_{n,k} T_n(X)` with (DLMF 10.35.1 and the integral
//! form of `phi_k`, then Kummer's transformation)
//!
//! `c_{n,k} = (2 - [n = 0]) sum_j (b/2)^{2j+n} / (j! (j+n)!) p!/(p+k)!
//! e^a 1F1(k; p+k+1; -a)`, `p = 2j + n`.
//!
//! Laguerre uses `X = -A / beta`, `beta = rho / L`, `a = h beta`,
//! `q = a / (1 + a)` and (Pfaff's transformation)
//! `c_{n,k} = n!/(n+k)! q^n (1 - q) 2F1(n+1, k; n+k+1; q)`,
//! `c_{n,0} = (1 - q) q^n`.
//!
//! Every series has positive terms and is evaluated with directed rounding
//! and a geometric tail bound, so each coefficient comes with an enclosure
//! that assumes only correctly rounded `+ - * /` (no library `exp`, `asinh`
//! or Bessel routine). `|a|` or `b` above [`COEFFICIENT_RANGE_LIMIT`] is an
//! explicit failure.
//!
//! # Error
//!
//! The truncation bound is a theorem in exact arithmetic (Chebyshev: the
//! Chernoff bound `2 exp(b (cosh t - 1) - (m+1)(t - eta))` for any `t >
//! eta`, where `eta` covers a transformed spectrum up to `1 + delta`;
//! Laguerre: `e^{L'/2} q^{m+1}`). It is **not** a total binary64
//! certificate. [`TotalErrorStatus::Certified`] adds, all rounded upward, the
//! coefficient enclosure radii, the exact local residual of every recurrence
//! step (enclosed with directed rounding) propagated by
//! `||U_k(X)|| <= (k+1) e^{k eta}`, and the enclosed summation error; it needs
//! a verified enclosure and the Chebyshev basis. The Laguerre recurrence has
//! no such propagation bound here, and a declared enclosure is not verified,
//! so both report [`TotalErrorStatus::EstimateOnly`] with the components that
//! are bounded.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    CoreError, CoreResult, DenseMatrix, WorkCounters, dense_phi_combination,
    directed::{
        Interval, add_down, add_up, div_down, div_up, mul_down, mul_up, sqrt_up, sub_down, sub_up,
    },
    sha256_hex,
    transform_bound::ExpBound,
};

pub const JOINT_PHI_SCHEMA: &str = "vigilode-joint-phi-polynomial-v1";
/// Number of phi terms `w_0 .. w_4`.
pub const JOINT_PHI_TERMS: usize = 5;
pub const POLYNOMIAL_DOMAIN_UNSUPPORTED: &str = "POLYNOMIAL_DOMAIN_OR_ACCURACY_UNSUPPORTED";
pub const TOTAL_ERROR_NOT_CERTIFIED: &str = "TOTAL_ERROR_NOT_CERTIFIED";
/// An input, budget or result outside the binary64 range the action can
/// represent after power-of-two normalization (re-audit R4, POLY-DEV-01).
pub const POLYNOMIAL_RANGE_UNSUPPORTED: &str = "POLYNOMIAL_RANGE_UNSUPPORTED";
/// Inputs whose largest entry has a binary exponent outside
/// `[-NORMALIZATION_WINDOW, NORMALIZATION_WINDOW]` are scaled by an exact
/// power of two before the action (and the result and bounds back after);
/// inside the window the action runs unscaled, bit for bit as before.
pub const NORMALIZATION_WINDOW: i64 = 500;
/// Largest `|a|` and `b` for which the coefficient series stay in range.
pub const COEFFICIENT_RANGE_LIMIT: f64 = 600.0;
/// Laguerre scales `L` (`beta = rho / L`); the cap 16 is a policy.
pub const LAGUERRE_SCALES: [f64; 5] = [1.0, 2.0, 4.0, 8.0, 16.0];
/// The largest Laguerre scale `L = rho / beta` any selection may use.
pub const LAGUERRE_SCALE_CAP: f64 = 16.0;
pub const MAX_POLYNOMIAL_DEGREE: usize = 4096;
const MAX_SERIES_TERMS: usize = 200_000;
/// Stop a positive series once the tail is below this fraction of the sum.
const SERIES_RELATIVE_TAIL: f64 = 1.0 / (1u64 << 60) as f64;

fn unsupported(reason: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("{POLYNOMIAL_DOMAIN_UNSUPPORTED}: {reason}"))
}

fn factorial(k: usize) -> f64 {
    (1..=k).fold(1.0, |acc, i| acc * i as f64)
}

// ---------------------------------------------------------------------------
// Positive series with directed rounding
// ---------------------------------------------------------------------------

/// Upper bound on the tail `t S / (1 - S)` of a positive series whose
/// remaining term ratios are all at most `S < 1`, after a last term `t`.
fn geometric_tail_up(term: f64, ratio: f64) -> CoreResult<f64> {
    div_up(mul_up(term, ratio)?, sub_down(1.0, ratio)?)
}

/// `e^x` for `x >= 0`, rounded down (`upward = false`) or up.
fn exp_nonneg(x: f64, upward: bool) -> CoreResult<f64> {
    if x.is_nan() || !(0.0..=709.0).contains(&x) {
        return Err(unsupported(format!("exponent {x:e} out of range")));
    }
    let mut sum = 1.0;
    let mut term = 1.0;
    for i in 1..MAX_SERIES_TERMS {
        let step = i as f64;
        term = if upward {
            div_up(mul_up(term, x)?, step)?
        } else {
            div_down(mul_down(term, x)?, step)?
        };
        sum = if upward {
            add_up(sum, term)?
        } else {
            add_down(sum, term)?
        };
        let ratio = x / (step + 1.0);
        if ratio < 0.5 && term <= sum * SERIES_RELATIVE_TAIL {
            return if upward {
                add_up(sum, geometric_tail_up(term, div_up(x, step + 1.0)?)?)
            } else {
                Ok(sum)
            };
        }
    }
    Err(unsupported("exponential series did not converge"))
}

/// Enclosure of `e^x` for an interval `x >= 0`.
#[cfg(test)]
fn exp_interval(x: Interval) -> CoreResult<Interval> {
    Interval::new(exp_nonneg(x.lo, false)?, exp_nonneg(x.hi, true)?)
}

/// Enclosure of `e^{-x}` for an interval `x >= 0`.
fn exp_neg_interval(x: Interval) -> CoreResult<Interval> {
    Interval::new(
        div_down(1.0, exp_nonneg(x.hi, true)?)?,
        div_up(1.0, exp_nonneg(x.lo, false)?)?,
    )
}

/// Upper bound on `e^x` for any real `x` (at most 709); `e^x <= e^-700`
/// is used below `-700`.
pub(crate) fn exp_up(x: f64) -> CoreResult<f64> {
    if x >= 0.0 {
        exp_nonneg(x, true)
    } else {
        div_up(1.0, exp_nonneg((-x).min(700.0), false)?)
    }
}

/// `1F1(k; c; x) = sum_i (k)_i / (c)_i x^i / i!` for `0 <= k <= c`, `x >= 0`,
/// rounded down or up. Term ratios are at most `x / (i + 1)`.
fn hyp1f1_nonneg(k: usize, c: usize, x: f64, upward: bool) -> CoreResult<f64> {
    if k == 0 || x == 0.0 {
        return Ok(1.0);
    }
    let mut sum = 1.0;
    let mut term = 1.0;
    for i in 0..MAX_SERIES_TERMS {
        let numerator = (k + i) as f64;
        let denominator = ((c + i) as f64) * ((i + 1) as f64);
        term = if upward {
            div_up(mul_up(mul_up(term, numerator)?, x)?, denominator)?
        } else {
            div_down(mul_down(mul_down(term, numerator)?, x)?, denominator)?
        };
        sum = if upward {
            add_up(sum, term)?
        } else {
            add_down(sum, term)?
        };
        let ratio = x / (i + 2) as f64;
        if ratio < 0.5 && term <= sum * SERIES_RELATIVE_TAIL {
            return if upward {
                add_up(sum, geometric_tail_up(term, div_up(x, (i + 2) as f64)?)?)
            } else {
                Ok(sum)
            };
        }
    }
    Err(unsupported("1F1 series did not converge"))
}

/// `2F1(n+1, k; n+k+1; q) = sum_i (n+1)_i (k)_i / ((n+k+1)_i i!) q^i` for
/// `k >= 1`, `0 <= q < 1`; term ratios after term `i` are at most
/// `q (k+i) / (i+1)`, which decreases in `i`.
fn hyp2f1_laguerre(n: usize, k: usize, q: f64, upward: bool) -> CoreResult<f64> {
    if q == 0.0 {
        return Ok(1.0);
    }
    let mut sum = 1.0;
    let mut term = 1.0;
    for i in 0..MAX_SERIES_TERMS {
        let numerator = ((n + 1 + i) as f64) * ((k + i) as f64);
        let denominator = ((n + k + 1 + i) as f64) * ((i + 1) as f64);
        term = if upward {
            div_up(mul_up(mul_up(term, numerator)?, q)?, denominator)?
        } else {
            div_down(mul_down(mul_down(term, numerator)?, q)?, denominator)?
        };
        sum = if upward {
            add_up(sum, term)?
        } else {
            add_down(sum, term)?
        };
        let ratio = div_up(mul_up(q, (k + i + 1) as f64)?, (i + 2) as f64)?;
        if ratio < 1.0 {
            let tail = geometric_tail_up(term, ratio)?;
            if tail <= sum * SERIES_RELATIVE_TAIL {
                return if upward { add_up(sum, tail) } else { Ok(sum) };
            }
        }
    }
    Err(unsupported("2F1 series did not converge"))
}

/// Enclosure of `1 / ((p+1)(p+2)...(p+k))` (1 for `k = 0`).
fn inverse_rising(p: usize, k: usize) -> CoreResult<Interval> {
    let mut lo = 1.0;
    let mut hi = 1.0;
    for i in 1..=k {
        lo = div_down(lo, (p + i) as f64)?;
        hi = div_up(hi, (p + i) as f64)?;
    }
    Interval::new(lo, hi)
}

fn positive_mul(x: Interval, y: Interval) -> CoreResult<Interval> {
    Interval::new(mul_down(x.lo, y.lo)?, mul_up(x.hi, y.hi)?)
}

// ---------------------------------------------------------------------------
// Coefficients
// ---------------------------------------------------------------------------

/// Chebyshev coefficient enclosures `c_{n,k}`, `n = 0..=degree`, for
/// `a <= 0 <= b` (intervals of the exact reals).
fn chebyshev_coefficients(
    a: Interval,
    b: Interval,
    degree: usize,
) -> CoreResult<Vec<[Interval; JOINT_PHI_TERMS]>> {
    let abs_a = Interval::new(-a.hi, -a.lo)?;
    if abs_a.lo < 0.0 || b.lo < 0.0 {
        return Err(unsupported("Chebyshev coefficients need a <= 0 <= b"));
    }
    if abs_a.hi > COEFFICIENT_RANGE_LIMIT || b.hi > COEFFICIENT_RANGE_LIMIT {
        return Err(unsupported(format!(
            "coefficient range: |a| = {:e}, b = {:e} exceed {COEFFICIENT_RANGE_LIMIT}",
            abs_a.hi, b.hi
        )));
    }
    let exp_a = exp_neg_interval(abs_a)?;
    let half_b = Interval::new(b.lo * 0.5, b.hi * 0.5)?;
    let quarter = positive_mul(half_b, half_b)?;
    // 1F1(k; p+k+1; |a|) by p, computed on demand.
    let mut hypergeometric: Vec<[Interval; JOINT_PHI_TERMS]> = Vec::new();
    let mut hyp = |p: usize| -> CoreResult<[Interval; JOINT_PHI_TERMS]> {
        while hypergeometric.len() <= p {
            let q = hypergeometric.len();
            let mut row = [Interval::point(1.0)?; JOINT_PHI_TERMS];
            for (k, value) in row.iter_mut().enumerate().skip(1) {
                *value = Interval::new(
                    hyp1f1_nonneg(k, q + k + 1, abs_a.lo, false)?,
                    hyp1f1_nonneg(k, q + k + 1, abs_a.hi, true)?,
                )?;
            }
            hypergeometric.push(row);
        }
        Ok(hypergeometric[p])
    };
    let mut out = Vec::with_capacity(degree + 1);
    // (b/2)^n / n!
    let mut leading = Interval::point(1.0)?;
    for n in 0..=degree {
        if n > 0 {
            leading = Interval::new(
                div_down(mul_down(leading.lo, half_b.lo)?, n as f64)?,
                div_up(mul_up(leading.hi, half_b.hi)?, n as f64)?,
            )?;
        }
        let mut sums = [Interval::point(0.0)?; JOINT_PHI_TERMS];
        let mut term = leading;
        let mut closed = false;
        for j in 0..MAX_SERIES_TERMS {
            let p = 2 * j + n;
            let hyp_row = hyp(p)?;
            let mut contributions = [0.0; JOINT_PHI_TERMS];
            for k in 0..JOINT_PHI_TERMS {
                let factor = positive_mul(inverse_rising(p, k)?, hyp_row[k])?;
                let contribution = positive_mul(term, factor)?;
                contributions[k] = contribution.hi;
                sums[k] = Interval::new(
                    add_down(sums[k].lo, contribution.lo)?,
                    add_up(sums[k].hi, contribution.hi)?,
                )?;
            }
            // Later terms: T_{j+1} = T_j (b/2)^2 / ((j+1)(j+n+1)), with the
            // other factors decreasing in p.
            let denominator = ((j + 1) as f64) * ((j + n + 1) as f64);
            let ratio = div_up(quarter.hi, denominator)?;
            if ratio < 0.5 {
                let small = (0..JOINT_PHI_TERMS).all(|k| {
                    contributions[k] <= sums[k].lo * SERIES_RELATIVE_TAIL || contributions[k] == 0.0
                });
                if small {
                    for k in 0..JOINT_PHI_TERMS {
                        let tail = geometric_tail_up(contributions[k], ratio)?;
                        sums[k] = Interval::new(sums[k].lo, add_up(sums[k].hi, tail)?)?;
                    }
                    closed = true;
                    break;
                }
            }
            term = Interval::new(
                div_down(mul_down(term.lo, quarter.lo)?, denominator)?,
                div_up(mul_up(term.hi, quarter.hi)?, denominator)?,
            )?;
        }
        if !closed {
            return Err(unsupported("Bessel series did not converge"));
        }
        let weight = if n == 0 { 1.0 } else { 2.0 };
        let mut row = [Interval::point(0.0)?; JOINT_PHI_TERMS];
        for k in 0..JOINT_PHI_TERMS {
            let value = positive_mul(exp_a, sums[k])?;
            row[k] = Interval::new(value.lo * weight, value.hi * weight)?;
        }
        out.push(row);
    }
    Ok(out)
}

/// Laguerre coefficient enclosures for `a = h beta >= 0`.
fn laguerre_coefficients(
    a: Interval,
    degree: usize,
) -> CoreResult<Vec<[Interval; JOINT_PHI_TERMS]>> {
    if a.lo < 0.0 {
        return Err(unsupported("Laguerre coefficients need h beta >= 0"));
    }
    let q = Interval::new(
        div_down(a.lo, add_up(1.0, a.lo)?)?,
        div_up(a.hi, add_down(1.0, a.hi)?)?,
    )?;
    if q.hi >= 1.0 {
        return Err(unsupported("Laguerre ratio q rounds to 1"));
    }
    let one_minus_q = Interval::new(
        div_down(1.0, add_up(1.0, a.hi)?)?,
        div_up(1.0, add_down(1.0, a.lo)?)?,
    )?;
    let mut out = Vec::with_capacity(degree + 1);
    let mut power = Interval::point(1.0)?;
    for n in 0..=degree {
        if n > 0 {
            power = positive_mul(power, q)?;
        }
        let base = positive_mul(one_minus_q, power)?;
        let mut row = [base; JOINT_PHI_TERMS];
        for (k, value) in row.iter_mut().enumerate().skip(1) {
            let series = Interval::new(
                hyp2f1_laguerre(n, k, q.lo, false)?,
                hyp2f1_laguerre(n, k, q.hi, true)?,
            )?;
            *value = positive_mul(positive_mul(base, inverse_rising(n, k)?)?, series)?;
        }
        out.push(row);
    }
    Ok(out)
}

/// Chebyshev coefficient enclosures for `h` and the enclosure
/// `[-rho, -lambda]`, with the binary64 shift `(rho + lambda) / 2` and
/// half-width `(rho - lambda) / 2` the action uses.
pub fn chebyshev_coefficient_enclosures(
    h: f64,
    lambda: f64,
    rho: f64,
    degree: usize,
) -> CoreResult<Vec<[Interval; JOINT_PHI_TERMS]>> {
    let shift = 0.5 * (rho + lambda);
    let half_width = 0.5 * (rho - lambda);
    let a = Interval::new(-mul_up(h, shift)?, -mul_down(h, shift)?)?;
    let b = Interval::new(mul_down(h, half_width)?, mul_up(h, half_width)?)?;
    chebyshev_coefficients(a, b, degree)
}

/// Laguerre coefficient enclosures for `h` and `beta`.
pub fn laguerre_coefficient_enclosures(
    h: f64,
    beta: f64,
    degree: usize,
) -> CoreResult<Vec<[Interval; JOINT_PHI_TERMS]>> {
    laguerre_coefficients(Interval::new(mul_down(h, beta)?, mul_up(h, beta)?)?, degree)
}

/// Enclosure of `phi_k(z)` for `z <= 0`: `e^z 1F1(k; k+1; -z) / k!`.
fn scalar_phi(z: Interval) -> CoreResult<[Interval; JOINT_PHI_TERMS]> {
    let abs_z = Interval::new(-z.hi, -z.lo)?;
    if abs_z.lo < 0.0 || abs_z.hi > COEFFICIENT_RANGE_LIMIT {
        return Err(unsupported("scalar phi needs -600 <= z <= 0"));
    }
    let exp_z = exp_neg_interval(abs_z)?;
    let mut out = [exp_z; JOINT_PHI_TERMS];
    for (k, value) in out.iter_mut().enumerate().skip(1) {
        let series = Interval::new(
            hyp1f1_nonneg(k, k + 1, abs_z.lo, false)?,
            hyp1f1_nonneg(k, k + 1, abs_z.hi, true)?,
        )?;
        let inverse = inverse_rising(0, k)?;
        *value = positive_mul(positive_mul(exp_z, series)?, inverse)?;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Operator domain
// ---------------------------------------------------------------------------

/// Where the spectral enclosure comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum EnclosureEvidence {
    /// Gershgorin discs, computed with outward rounding, lie inside the
    /// enclosure (valid for a symmetric matrix).
    Gershgorin,
    /// Supplied by the caller and recorded, not verified.
    Declared { source: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectralEnclosure {
    pub lambda: f64,
    pub rho: f64,
    pub evidence: EnclosureEvidence,
}

/// A dense, exactly symmetric matrix with `spec(A) in [-rho, -lambda]`.
#[derive(Clone, Debug)]
pub struct SymmetricNonpositiveOperator {
    matrix: DenseMatrix,
    enclosure: SpectralEnclosure,
    fingerprint: String,
}

/// Outward Gershgorin interval `[lo, hi]` of a symmetric matrix.
fn gershgorin(matrix: &DenseMatrix) -> CoreResult<(f64, f64)> {
    let n = matrix.nrows();
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for i in 0..n {
        let radius =
            crate::directed::sum_up((0..n).filter(|j| *j != i).map(|j| matrix[(i, j)].abs()))?;
        lo = lo.min(sub_down(matrix[(i, i)], radius)?);
        hi = hi.max(add_up(matrix[(i, i)], radius)?);
    }
    Ok((lo, hi))
}

impl SymmetricNonpositiveOperator {
    /// With an enclosure the Gershgorin discs verify when they can;
    /// otherwise it is recorded as declared by `source`.
    pub fn new(
        matrix: DenseMatrix,
        lambda: f64,
        rho: f64,
        source: impl Into<String>,
    ) -> CoreResult<Self> {
        Self::check_matrix(&matrix)?;
        if !(lambda.is_finite() && rho.is_finite() && 0.0 <= lambda && lambda <= rho) {
            return Err(unsupported(format!(
                "enclosure needs 0 <= lambda <= rho, got [{lambda:e}, {rho:e}]"
            )));
        }
        let n = matrix.nrows();
        if (0..n).any(|i| !(-rho <= matrix[(i, i)] && matrix[(i, i)] <= -lambda)) {
            return Err(unsupported(
                "a diagonal entry lies outside the declared enclosure, so it cannot enclose the spectrum",
            ));
        }
        let (lo, hi) = gershgorin(&matrix)?;
        let evidence = if -rho <= lo && hi <= -lambda {
            EnclosureEvidence::Gershgorin
        } else {
            EnclosureEvidence::Declared {
                source: source.into(),
            }
        };
        Ok(Self::with_enclosure(
            matrix,
            SpectralEnclosure {
                lambda,
                rho,
                evidence,
            },
        ))
    }

    /// The enclosure from the Gershgorin discs; an error when they do not
    /// show `A <= 0` (no spectral witness).
    pub fn gershgorin(matrix: DenseMatrix) -> CoreResult<Self> {
        Self::check_matrix(&matrix)?;
        let (lo, hi) = gershgorin(&matrix)?;
        if hi > 0.0 {
            return Err(unsupported(format!(
                "spectral witness unavailable: Gershgorin upper end {hi:e} is positive"
            )));
        }
        Ok(Self::with_enclosure(
            matrix,
            SpectralEnclosure {
                lambda: -hi,
                rho: -lo,
                evidence: EnclosureEvidence::Gershgorin,
            },
        ))
    }

    fn check_matrix(matrix: &DenseMatrix) -> CoreResult<()> {
        let n = matrix.nrows();
        if n == 0 || matrix.ncols() != n {
            return Err(unsupported("operator must be a nonempty square matrix"));
        }
        for i in 0..n {
            for j in 0..i {
                if matrix[(i, j)] != matrix[(j, i)] {
                    return Err(unsupported(
                        "operator is not exactly symmetric (nonnormal inputs are outside the domain)",
                    ));
                }
            }
        }
        Ok(())
    }

    fn with_enclosure(matrix: DenseMatrix, enclosure: SpectralEnclosure) -> Self {
        let mut bytes = Vec::with_capacity(16 + 8 * matrix.as_slice().len());
        bytes.extend_from_slice(&(matrix.nrows() as u64).to_le_bytes());
        for value in matrix.as_slice() {
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        bytes.extend_from_slice(&enclosure.lambda.to_bits().to_le_bytes());
        bytes.extend_from_slice(&enclosure.rho.to_bits().to_le_bytes());
        let fingerprint = sha256_hex(&bytes);
        Self {
            matrix,
            enclosure,
            fingerprint,
        }
    }

    pub fn matrix(&self) -> &DenseMatrix {
        &self.matrix
    }

    pub fn enclosure(&self) -> &SpectralEnclosure {
        &self.enclosure
    }

    /// SHA-256 of the matrix and enclosure bits: part of every coefficient
    /// cache key.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn dimension(&self) -> usize {
        self.matrix.nrows()
    }
}

// ---------------------------------------------------------------------------
// Requests, cache and report
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolynomialBasis {
    Chebyshev,
    Laguerre,
}

/// The vectors the action applies to.
#[derive(Clone, Copy, Debug)]
pub enum JointPhiInput<'a> {
    /// Five distinct vectors `w_0 .. w_4`: one block recurrence of width 5.
    Distinct(&'a [Vec<f64>; JOINT_PHI_TERMS]),
    /// `w_k = scales[k] v`: one vector recurrence reused for every `k`.
    SameVector {
        vector: &'a [f64],
        scales: [f64; JOINT_PHI_TERMS],
    },
}

/// Everything the coefficients depend on. A changed operator, enclosure,
/// `h`, degree or Laguerre scale is a different key, so no table (and no
/// bound derived from it) is reused across them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CoefficientKey {
    pub basis: PolynomialBasis,
    pub operator_fingerprint: String,
    pub h_bits: u64,
    pub degree: usize,
    pub laguerre_scale_bits: u64,
}

#[derive(Clone, Debug, Default)]
pub struct CoefficientCache {
    tables: HashMap<CoefficientKey, Vec<[Interval; JOINT_PHI_TERMS]>>,
}

impl CoefficientCache {
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }
}

/// Error components of one column `phi_k(hA) w_k`, each an upper bound in
/// the 2-norm; `None` when this basis has no bound for it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorComponents {
    /// Series truncation, exact arithmetic.
    pub truncation: f64,
    /// Coefficient enclosure radii times `||T_n(X)|| ||w_k||`.
    pub coefficient: f64,
    /// Rounding of the recurrence, propagated.
    pub recurrence: Option<f64>,
    /// Rounding of the coefficient-weighted sum and the final scaling.
    pub summation: f64,
    /// Bits lost when an out-of-range input was scaled by a power of two,
    /// propagated (re-audit R4, POLY-DEV-01); 0 inside the window.
    #[serde(default)]
    pub normalization: f64,
    /// Laguerre only: the scalar recurrence majorant of re-audit R4
    /// (POLY-DEV-03), `E_{n+1} <= d_n E_n + n/(n+1) E_{n-1} + ||eps_n||`
    /// with `d_n = max(2n+1, L' - 2n - 1)/(n+1)`, weighted by the
    /// coefficients. A baseline under review: it never enters
    /// [`TotalErrorStatus::Certified`] and `recurrence` stays `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence_majorant: Option<f64>,
    /// Laguerre only, degree at most
    /// [`crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT`]: the signed
    /// output-adjoint bound `sum_j beta_j eps_j` of the recurrence error of
    /// the stored finite polynomial (thread-transfer node P1-LAGUERRE-CERT).
    /// A proved component; it does not by itself make the total
    /// [`TotalErrorStatus::Certified`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence_adjoint: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum TotalErrorStatus {
    /// `||fused - F||_2 <= bound` under correctly rounded `+ - * /` and the
    /// verified enclosure.
    Certified { bound: f64 },
    /// TOTAL_ERROR_NOT_CERTIFIED: some component is not bounded (or the
    /// enclosure is only declared); `bounded_components` sums the others.
    EstimateOnly {
        reason: String,
        bounded_components: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JointPhiReport {
    pub schema: String,
    pub basis: PolynomialBasis,
    /// `distinct` or `same-vector`.
    pub input: String,
    /// `recurrence` or `scalar` (`h = 0`, `A = 0` or `A = -lambda I`).
    pub branch: String,
    pub dimension: usize,
    pub degree: usize,
    pub laguerre_scale: Option<f64>,
    /// `phi_k(hA) w_k` for `k = 0..4`.
    pub columns: Vec<Vec<f64>>,
    pub fused: Vec<f64>,
    /// Exact-arithmetic truncation bound of the fused sum; not a total
    /// binary64 certificate.
    pub truncation_bound_exact_arithmetic: f64,
    pub truncation_budget: f64,
    pub column_errors: Vec<ErrorComponents>,
    pub fused_summation: f64,
    pub total_error: TotalErrorStatus,
    /// `sum_k ||phi_k(hA) w_k|| / ||fused||`: cancellation in the fused sum,
    /// reported separately from the absolute bound.
    pub condition_proxy: f64,
    pub evidence: EnclosureEvidence,
    pub coefficient_cache_hit: bool,
    /// The power-of-two exponent `s` the input was scaled by (`2^-s`);
    /// 0 inside [`NORMALIZATION_WINDOW`] (re-audit R4, POLY-DEV-01).
    #[serde(default)]
    pub normalization_shift: i64,
    /// [`EXECUTION_CERTIFIED`] or [`EXECUTION_UNBOUNDED_TIMING`].
    #[serde(default)]
    pub execution: String,
    /// Laguerre: the bounded components plus every column's
    /// `recurrence_majorant` (re-audit R4, POLY-DEV-03). A candidate total
    /// for the tightness study, not a certificate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laguerre_majorant_total: Option<f64>,
    /// Laguerre with a verified enclosure and every column's
    /// `recurrence_adjoint`: the bounded components plus those bounds. A
    /// candidate total; [`JointPhiReport::total_error`] is unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laguerre_adjoint_total: Option<f64>,
}

/// How the action was executed: with the rounding enclosures
/// ([`joint_phi_action`]) or in plain binary64 for timing
/// ([`joint_phi_action_unbounded`]); the latter never certifies.
pub const EXECUTION_CERTIFIED: &str = "certified-enclosures";
pub const EXECUTION_UNBOUNDED_TIMING: &str = "unbounded-timing";

/// The outcome of [`JointPhiReport::admit_total_error`] (re-audit R4,
/// POLY-DEV-02). The truncation budget the degree was chosen for is a
/// separate quantity: meeting it says nothing about the total error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum TotalErrorAdmission {
    /// `||fused - F||_2 <= bound <= budget`, certified.
    Admitted {
        bound: f64,
        budget: f64,
        /// `bound / ||fused||_2` (infinite for a zero result): the relative
        /// accuracy, reported beside the conditioning, not used to admit.
        relative_bound: f64,
        condition_proxy: f64,
    },
    Rejected {
        reason: String,
    },
}

pub const TOTAL_ERROR_ABOVE_BUDGET: &str = "TOTAL_ERROR_ABOVE_BUDGET";

impl JointPhiReport {
    /// Admit the result iff its total error is
    /// [`TotalErrorStatus::Certified`] with `bound <= budget`, for an
    /// absolute budget in `[0, inf)`. An estimate, a negative, NaN or
    /// infinite budget, or a certified bound above the budget is rejected,
    /// whatever the truncation bound met.
    pub fn admit_total_error(&self, budget: f64) -> TotalErrorAdmission {
        if !(budget.is_finite() && budget >= 0.0) {
            return TotalErrorAdmission::Rejected {
                reason: format!("absolute budget {budget:e} outside [0, inf)"),
            };
        }
        match &self.total_error {
            TotalErrorStatus::EstimateOnly { reason, .. } => TotalErrorAdmission::Rejected {
                reason: reason.clone(),
            },
            TotalErrorStatus::Certified { bound } if *bound <= budget => {
                let norm = crate::safe_l2(&self.fused);
                TotalErrorAdmission::Admitted {
                    bound: *bound,
                    budget,
                    relative_bound: if norm > 0.0 {
                        bound / norm
                    } else if *bound == 0.0 {
                        0.0
                    } else {
                        f64::INFINITY
                    },
                    condition_proxy: self.condition_proxy,
                }
            }
            TotalErrorStatus::Certified { bound } => TotalErrorAdmission::Rejected {
                reason: format!(
                    "{TOTAL_ERROR_ABOVE_BUDGET}: certified bound {bound:e} > budget {budget:e} (truncation {:e} against its budget {:e})",
                    self.truncation_bound_exact_arithmetic, self.truncation_budget
                ),
            },
        }
    }
}

/// The components of a Laguerre total and the report fields that hold them
/// (research node `research/rnext04_laguerre_admission_20261003`). The
/// transform `X = -A / beta` with the stored `beta` and the coefficients for
/// the exact `a = h beta` make the transform exact, so it has no field.
pub const LAGUERRE_TOTAL_COMPONENTS: [(&str, &str); 6] = [
    (
        "transform",
        "none: exact by construction (X = -A/beta, coefficients for a = h beta)",
    ),
    ("tail", "column_errors[k].truncation"),
    ("coefficient", "column_errors[k].coefficient"),
    ("recurrence", "column_errors[k].recurrence_adjoint"),
    (
        "accumulation",
        "column_errors[k].summation and fused_summation",
    ),
    ("normalization", "column_errors[k].normalization"),
];

pub const LAGUERRE_TOTAL_NOT_ADMITTED: &str = "LAGUERRE_TOTAL_NOT_ADMITTED";

impl JointPhiReport {
    /// The registry's recomputation of the Laguerre total from the report
    /// fields, rounded upward in the action's order: `fused_summation`, then
    /// per column truncation, coefficient, summation, then every column's
    /// `recurrence_adjoint`, then every column's `normalization`. `None`
    /// unless every column has a `recurrence_adjoint`.
    pub fn laguerre_total_recomputed(&self) -> CoreResult<Option<f64>> {
        if self
            .column_errors
            .iter()
            .any(|c| c.recurrence_adjoint.is_none())
        {
            return Ok(None);
        }
        let mut total = self.fused_summation;
        for c in &self.column_errors {
            total = add_up(total, c.truncation)?;
            total = add_up(total, c.coefficient)?;
            total = add_up(total, c.summation)?;
        }
        for c in &self.column_errors {
            total = add_up(total, c.recurrence_adjoint.unwrap_or(0.0))?;
        }
        for c in &self.column_errors {
            total = add_up(total, c.normalization)?;
        }
        Ok(Some(total))
    }

    /// Opt-in admission of the Laguerre total (research node
    /// `research/rnext04_laguerre_admission_20261003`): `||fused - F||_2 <=
    /// laguerre_adjoint_total <= budget` for a Laguerre recurrence report with
    /// certified enclosures, a Gershgorin-verified enclosure, degree at most
    /// [`crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT`] and every
    /// column's adjoint bound. The scalar branch defers to
    /// [`Self::admit_total_error`]. Everything else is rejected;
    /// [`Self::total_error`] is not changed.
    pub fn admit_laguerre_total(&self, budget: f64) -> TotalErrorAdmission {
        let reject = |why: String| TotalErrorAdmission::Rejected {
            reason: format!("{LAGUERRE_TOTAL_NOT_ADMITTED}: {why}"),
        };
        if !(budget.is_finite() && budget >= 0.0) {
            return reject(format!("absolute budget {budget:e} outside [0, inf)"));
        }
        if self.basis != PolynomialBasis::Laguerre {
            return reject("not a Laguerre report".into());
        }
        if self.execution != EXECUTION_CERTIFIED {
            return reject(format!("execution {}", self.execution));
        }
        if !matches!(self.evidence, EnclosureEvidence::Gershgorin) {
            return reject("the spectral enclosure is not verified".into());
        }
        if self.branch == "scalar" {
            return self.admit_total_error(budget);
        }
        if self.degree > crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT {
            return reject(format!(
                "degree {} above the adjoint limit {}",
                self.degree,
                crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT
            ));
        }
        let Some(bound) = self.laguerre_adjoint_total.filter(|_| {
            self.column_errors
                .iter()
                .all(|c| c.recurrence_adjoint.is_some())
        }) else {
            return reject("a column has no adjoint bound".into());
        };
        if !bound.is_finite() {
            return reject("the total is not finite".into());
        }
        if bound > budget {
            return reject(format!(
                "{TOTAL_ERROR_ABOVE_BUDGET}: bound {bound:e} > budget {budget:e}"
            ));
        }
        let norm = crate::safe_l2(&self.fused);
        TotalErrorAdmission::Admitted {
            bound,
            budget,
            relative_bound: if norm > 0.0 {
                bound / norm
            } else if bound == 0.0 {
                0.0
            } else {
                f64::INFINITY
            },
            condition_proxy: self.condition_proxy,
        }
    }
}

/// Upper bound on the Euclidean norm, formed on a power-of-two scale
/// ([`ExpBound::l2_norm_upper`], re-audit R4 POLY-DEV-01): the squares of
/// `1e300` no longer overflow and those of `1e-300` no longer round up to a
/// subnormal that inflates the bound to `1e-162`. A norm above `f64::MAX`
/// is a typed range failure.
fn norm_up(values: impl IntoIterator<Item = f64>) -> CoreResult<f64> {
    let values = values.into_iter().collect::<Vec<_>>();
    let bound = ExpBound::l2_norm_upper(&values)?.to_f64_up();
    if bound.is_finite() {
        Ok(bound)
    } else {
        Err(CoreError::NonFinite(format!(
            "{POLYNOMIAL_RANGE_UNSUPPORTED}: a norm exceeds the binary64 range"
        )))
    }
}

/// `x 2^shift`, rounded once (to nearest) if it lands in the subnormal
/// range; `+-inf` above the range.
fn scale_pow2(x: f64, shift: i64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let (m, e) = crate::binary_split(x.abs());
    crate::binary_scale(m, e + shift).copysign(x)
}

/// Upper bound in binary64 on `bound 2^shift` for `bound >= 0`.
fn scale_bound_up(bound: f64, shift: i64) -> CoreResult<f64> {
    Ok(ExpBound::exact(bound)?.scaled_pow2(shift).to_f64_up())
}

/// `sqrt(count) 2^exponent`, rounded up, for `exponent >= -1074`.
fn rounding_norm_up(count: usize, exponent: i64) -> CoreResult<f64> {
    if count == 0 {
        return Ok(0.0);
    }
    let root = ExpBound::exact(sqrt_up(count as f64)?)?;
    Ok(root.scaled_pow2(exponent).to_f64_up())
}

/// Upper bound on `|computed - exact|` for an exact value enclosed in `e`.
fn distance_up(computed: f64, e: Interval) -> CoreResult<f64> {
    Ok(sub_up(computed, e.lo)?
        .max(sub_up(e.hi, computed)?)
        .max(0.0))
}

fn midpoint(e: Interval) -> f64 {
    let mid = 0.5 * e.lo + 0.5 * e.hi;
    mid.clamp(e.lo, e.hi)
}

// ---------------------------------------------------------------------------
// Degree selection
// ---------------------------------------------------------------------------

/// Upper bound on `cosh(t) - 1` for `t >= 0`.
fn cosh_minus_one_up(t: f64) -> CoreResult<f64> {
    let e = exp_nonneg(t, true)?;
    let inverse = div_up(1.0, exp_nonneg(t, false)?)?;
    let sum = mul_up(0.5, add_up(e, inverse)?)?;
    Ok(sub_up(sum, 1.0)?.max(0.0))
}

/// Rigorous Chebyshev tail factor for degree `m`:
/// `2 exp(b (cosh t - 1) - (m+1)(t - eta) + max(0, a + b))`.
fn chebyshev_tail(b_hi: f64, a_plus_b_hi: f64, eta: f64, degree: usize) -> CoreResult<f64> {
    let r = (degree + 1) as f64;
    if b_hi == 0.0 {
        return Ok(0.0);
    }
    // Any t > eta is valid; asinh(r/b) is the optimum without eta.
    let t = (r / b_hi).asinh().max(eta * 2.0 + f64::MIN_POSITIVE);
    let t = t.min(700.0);
    let growth = mul_up(b_hi, cosh_minus_one_up(t)?)?;
    let decay = mul_down(r, sub_down(t, eta)?)?;
    let exponent = add_up(sub_up(growth, decay)?, a_plus_b_hi.max(0.0))?;
    if exponent > 700.0 {
        // Too large to meet any budget; never clamp an upper bound.
        return Ok(f64::INFINITY);
    }
    mul_up(2.0, exp_up(exponent)?)
}

/// `tail * weight <= budget`, false when the product is not finite.
fn meets_budget(tail: f64, weight: f64, budget: f64) -> bool {
    tail.is_finite() && mul_up(tail, weight).is_ok_and(|value| value <= budget)
}

// ---------------------------------------------------------------------------
// The action
// ---------------------------------------------------------------------------

/// `E_0 .. E_m` with `E_0 = 0` and `E_{n+1} = d_n E_n + n/(n+1) E_{n-1} +
/// eps_n`, `d_n = max(2n+1, L' - (2n+1)) / (n+1)`, all rounded up: a
/// majorant of `||t_n - L_n(X) w||_2` when `spec(X) in [0, L']`, `X`
/// symmetric and `eps_n` bounds the exact local residual of step `n`
/// (re-audit R4, POLY-DEV-03). The error obeys `e_{n+1} = ((2n+1) I - X)
/// e_n / (n+1) - n e_{n-1} / (n+1) + eps_n`, and `||(2n+1) I - X||_2 =
/// max over [0, L'] of |2n+1-x|`. It ignores the three-term cancellation,
/// so it can grow like `(1 + sqrt 2)^n`: a correctness baseline whose
/// tightness is measured, not a certificate.
fn laguerre_majorant(local: &[f64], extent: f64) -> CoreResult<Vec<f64>> {
    let mut bounds = vec![0.0; local.len() + 1];
    for (n, eps) in local.iter().enumerate() {
        let k = n as f64;
        let center = 2.0 * k + 1.0;
        let d = div_up(center.max(sub_up(extent, center)?), k + 1.0)?;
        let mut next = add_up(mul_up(d, bounds[n])?, *eps)?;
        if n > 0 {
            next = add_up(next, mul_up(div_up(k, k + 1.0)?, bounds[n - 1])?)?;
        }
        bounds[n + 1] = next;
    }
    Ok(bounds)
}

struct Transform {
    basis: PolynomialBasis,
    /// Chebyshev: `X = (A + shift I) / half_width`; Laguerre: `X = -A / beta`.
    shift: f64,
    half_width: f64,
    beta: f64,
    degree: usize,
    laguerre_scale: Option<f64>,
    tail_factor: f64,
    /// Chebyshev: `||T_n(X)|| <= e^{n eta}`.
    eta: f64,
    /// Laguerre: `||L_n(X)|| <= e^{L'/2}`.
    laguerre_norm: f64,
    /// Laguerre: `L' >= rho / beta`, so `spec(X) in [0, L']`.
    laguerre_extent: f64,
    a: Interval,
    b: Interval,
}

fn choose_transform(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    basis: PolynomialBasis,
    weight_factor: f64,
    budget: f64,
    laguerre_scales: &[f64],
) -> CoreResult<Transform> {
    let SpectralEnclosure { lambda, rho, .. } = op.enclosure;
    match basis {
        PolynomialBasis::Chebyshev => {
            let shift = 0.5 * (rho + lambda);
            let half_width = 0.5 * (rho - lambda);
            // Transformed enclosure [x_lo, x_hi] of the exact X.
            let x_lo = div_down(sub_down(-rho, -shift)?, half_width)?;
            let x_hi = div_up(sub_up(-lambda, -shift)?, half_width)?;
            let delta = sub_up(-1.0, x_lo)?.max(sub_up(x_hi, 1.0)?).max(0.0);
            let eta = if delta > 0.0 {
                sqrt_up(mul_up(2.0, delta)?)?
            } else {
                0.0
            };
            let a = Interval::new(-mul_up(h, shift)?, -mul_down(h, shift)?)?;
            let b = Interval::new(mul_down(h, half_width)?, mul_up(h, half_width)?)?;
            let a_plus_b = add_up(a.hi, b.hi)?;
            for degree in 0..=MAX_POLYNOMIAL_DEGREE {
                let tail = chebyshev_tail(b.hi, a_plus_b, eta, degree)?;
                if meets_budget(tail, weight_factor, budget) {
                    return Ok(Transform {
                        basis,
                        shift,
                        half_width,
                        beta: 0.0,
                        degree,
                        laguerre_scale: None,
                        tail_factor: tail,
                        eta,
                        laguerre_norm: 0.0,
                        laguerre_extent: 0.0,
                        a,
                        b,
                    });
                }
            }
            Err(unsupported(format!(
                "Chebyshev degree above {MAX_POLYNOMIAL_DEGREE} for budget {budget:e}"
            )))
        }
        PolynomialBasis::Laguerre => {
            let mut best: Option<Transform> = None;
            for &scale in laguerre_scales {
                let beta = rho / scale;
                let a = Interval::new(mul_down(h, beta)?, mul_up(h, beta)?)?;
                let q_hi = div_up(a.hi, add_down(1.0, a.hi)?)?;
                let scale_up = div_up(rho, beta)?;
                if q_hi >= 1.0 || scale_up > 1400.0 {
                    continue;
                }
                let norm = exp_up(mul_up(scale_up, 0.5)?)?;
                let limit = best.as_ref().map_or(MAX_POLYNOMIAL_DEGREE, |current| {
                    current.degree.saturating_sub(1)
                });
                // Tail e^{L'/2} q^{m+1}, with the power kept incrementally.
                let mut power = q_hi;
                for degree in 0..=limit {
                    if degree > 0 {
                        power = mul_up(power, q_hi)?;
                    }
                    let tail = mul_up(norm, power)?;
                    if meets_budget(tail, weight_factor, budget) {
                        best = Some(Transform {
                            basis,
                            shift: 0.0,
                            half_width: 0.0,
                            beta,
                            degree,
                            laguerre_scale: Some(scale),
                            tail_factor: tail,
                            eta: 0.0,
                            laguerre_norm: norm,
                            laguerre_extent: scale_up,
                            a,
                            b: Interval::point(0.0)?,
                        });
                        break;
                    }
                }
            }
            best.ok_or_else(|| {
                unsupported(format!(
                    "Laguerre degree above {MAX_POLYNOMIAL_DEGREE} for budget {budget:e}"
                ))
            })
        }
    }
}

/// `X t` in floating point, and an enclosure of the exact `X t` for the
/// computed `t` (per component).
fn apply_x(
    matrix: &DenseMatrix,
    transform: &Transform,
    t: &[f64],
) -> CoreResult<(Vec<f64>, Vec<Interval>)> {
    let n = t.len();
    let mut value = vec![0.0; n];
    let mut exact = Vec::with_capacity(n);
    for i in 0..n {
        let row = matrix.row(i);
        let mut dot = 0.0;
        let mut lo = 0.0;
        let mut hi = 0.0;
        for (entry, x) in row.iter().zip(t) {
            dot += entry * x;
            lo = add_down(lo, mul_down(*entry, *x)?)?;
            hi = add_up(hi, mul_up(*entry, *x)?)?;
        }
        let dot_exact = Interval::new(lo, hi)?;
        let (computed, enclosure) = match transform.basis {
            PolynomialBasis::Chebyshev => {
                let shifted = dot + transform.shift * t[i];
                let shifted_exact =
                    dot_exact.add(Interval::point(t[i])?.scale(transform.shift)?)?;
                (
                    shifted / transform.half_width,
                    shifted_exact.div(Interval::point(transform.half_width)?)?,
                )
            }
            PolynomialBasis::Laguerre => (
                -dot / transform.beta,
                (-dot_exact).div(Interval::point(transform.beta)?)?,
            ),
        };
        value[i] = computed;
        exact.push(enclosure);
    }
    Ok((value, exact))
}

/// One recurrence step; returns the next vector and an upper bound on the
/// 2-norm of its exact local residual.
fn recurrence_step(
    matrix: &DenseMatrix,
    transform: &Transform,
    n: usize,
    previous: Option<&[f64]>,
    current: &[f64],
) -> CoreResult<(Vec<f64>, f64)> {
    let (x_current, x_exact) = apply_x(matrix, transform, current)?;
    let mut next = vec![0.0; current.len()];
    let mut residual = Vec::with_capacity(current.len());
    for i in 0..current.len() {
        let (computed, exact) = match (transform.basis, previous) {
            (PolynomialBasis::Chebyshev, None) => (x_current[i], x_exact[i]),
            (PolynomialBasis::Chebyshev, Some(prev)) => (
                2.0 * x_current[i] - prev[i],
                x_exact[i].scale(2.0)?.sub(Interval::point(prev[i])?)?,
            ),
            (PolynomialBasis::Laguerre, None) => (
                current[i] - x_current[i],
                Interval::point(current[i])?.sub(x_exact[i])?,
            ),
            (PolynomialBasis::Laguerre, Some(prev)) => {
                let k = n as f64;
                let computed =
                    ((2.0 * k + 1.0) * current[i] - x_current[i] - k * prev[i]) / (k + 1.0);
                let exact = Interval::point(current[i])?
                    .scale(2.0 * k + 1.0)?
                    .sub(x_exact[i])?
                    .sub(Interval::point(prev[i])?.scale(k)?)?
                    .div(Interval::point(k + 1.0)?)?;
                (computed, exact)
            }
        };
        if !computed.is_finite() {
            return Err(CoreError::NonFinite(
                "polynomial recurrence produced NaN/Inf".into(),
            ));
        }
        next[i] = computed;
        residual.push(distance_up(computed, exact)?);
    }
    Ok((next, norm_up(residual)?))
}

/// One recurrence step in plain binary64, without the residual enclosure.
fn recurrence_step_plain(
    matrix: &DenseMatrix,
    transform: &Transform,
    n: usize,
    previous: Option<&[f64]>,
    current: &[f64],
) -> CoreResult<Vec<f64>> {
    let mut next = vec![0.0; current.len()];
    for (i, out) in next.iter_mut().enumerate() {
        // The same left-to-right sum from +0.0 as the enclosing path, so
        // both modes return the same bits (`Iterator::sum` starts at -0.0).
        let dot = matrix
            .row(i)
            .iter()
            .zip(current)
            .fold(0.0, |acc, (entry, x)| acc + entry * x);
        let x_current = match transform.basis {
            PolynomialBasis::Chebyshev => {
                (dot + transform.shift * current[i]) / transform.half_width
            }
            PolynomialBasis::Laguerre => -dot / transform.beta,
        };
        *out = match (transform.basis, previous) {
            (PolynomialBasis::Chebyshev, None) => x_current,
            (PolynomialBasis::Chebyshev, Some(prev)) => 2.0 * x_current - prev[i],
            (PolynomialBasis::Laguerre, None) => current[i] - x_current,
            (PolynomialBasis::Laguerre, Some(prev)) => {
                let k = n as f64;
                ((2.0 * k + 1.0) * current[i] - x_current - k * prev[i]) / (k + 1.0)
            }
        };
        if !out.is_finite() {
            return Err(CoreError::NonFinite(
                "polynomial recurrence produced NaN/Inf".into(),
            ));
        }
    }
    Ok(next)
}

/// The Laguerre recurrence `t_0 = w`, `t_(n+1) = ((2n+1) t_n - X t_n - n
/// t_(n-1))/(n+1)` with `X = -A / beta`, exactly as the action runs it, with
/// an upper bound on the 2-norm of every step's exact local residual
/// (research access for thread-transfer node P1-LAGUERRE-CERT).
pub fn laguerre_recurrence_with_residuals(
    matrix: &DenseMatrix,
    beta: f64,
    w: &[f64],
    degree: usize,
) -> CoreResult<(Vec<Vec<f64>>, Vec<f64>)> {
    if matrix.nrows() != w.len() || matrix.ncols() != w.len() || !(beta.is_finite() && beta > 0.0) {
        return Err(unsupported("Laguerre recurrence shape or beta"));
    }
    let transform = Transform {
        basis: PolynomialBasis::Laguerre,
        shift: 0.0,
        half_width: 0.0,
        beta,
        degree,
        laguerre_scale: None,
        tail_factor: 0.0,
        eta: 0.0,
        laguerre_norm: 0.0,
        laguerre_extent: 0.0,
        a: Interval::point(0.0)?,
        b: Interval::point(0.0)?,
    };
    let mut vectors = vec![w.to_vec()];
    let mut local = Vec::with_capacity(degree);
    for step in 0..degree {
        let previous = (step > 0).then(|| vectors[step - 1].clone());
        let (next, residual) = recurrence_step(
            matrix,
            &transform,
            step,
            previous.as_deref(),
            &vectors[step],
        )?;
        vectors.push(next);
        local.push(residual);
    }
    Ok((vectors, local))
}

fn empty_components() -> ErrorComponents {
    ErrorComponents {
        truncation: 0.0,
        coefficient: 0.0,
        recurrence: Some(0.0),
        summation: 0.0,
        normalization: 0.0,
        recurrence_majorant: None,
        recurrence_adjoint: None,
    }
}

/// `sum_k phi_k(h A) w_k` by a Chebyshev or Laguerre recurrence.
///
/// `budget` bounds the exact-arithmetic truncation of the fused sum. With a
/// `cache`, coefficient tables are reused only for an identical
/// [`CoefficientKey`].
pub fn joint_phi_action(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    basis: PolynomialBasis,
    budget: f64,
    cache: Option<&mut CoefficientCache>,
    work: &mut WorkCounters,
) -> CoreResult<JointPhiReport> {
    joint_phi_action_impl(
        op,
        h,
        input,
        basis,
        budget,
        cache,
        work,
        true,
        &LAGUERRE_SCALES,
    )
}

/// [`joint_phi_action`] in the Laguerre basis with the scale `L` (`beta =
/// rho / L`) chosen among `scales` instead of [`LAGUERRE_SCALES`]: each
/// finite and in `(0, 16]`, the policy cap (re-audit R4, POLY-DEV-03 and
/// POLY-DEV-05 sweeps). Research only.
pub fn joint_phi_action_laguerre_scales(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    scales: &[f64],
    budget: f64,
    work: &mut WorkCounters,
) -> CoreResult<JointPhiReport> {
    if scales.is_empty()
        || !scales
            .iter()
            .all(|scale| scale.is_finite() && *scale > 0.0 && *scale <= LAGUERRE_SCALE_CAP)
    {
        return Err(unsupported(format!(
            "Laguerre scales must be nonempty and in (0, {LAGUERRE_SCALE_CAP}]"
        )));
    }
    joint_phi_action_impl(
        op,
        h,
        input,
        PolynomialBasis::Laguerre,
        budget,
        None,
        work,
        true,
        scales,
    )
}

/// The continuous Laguerre scale of re-audit R4 (R4-POLY-DEV-05) for a
/// fixed degree `m`: the exact-arithmetic tail `T(beta) = W exp(rho / (2
/// beta)) [h beta / (1 + h beta)]^(m+1)` has `d log T / d beta = -rho / (2
/// beta^2) + r / (beta (1 + h beta))`, `r = m + 1`, whose only zero for
/// `2r > h rho` is `beta* = rho / (2r - h rho)`, a minimum; for `2r <= h
/// rho` the tail decreases in `beta` towards `W` and no finite scale meets a
/// budget below `W`. The scale `L = rho / beta* = 2r - h rho` is capped at
/// [`LAGUERRE_SCALE_CAP`]; `None` when it is not positive or `h`, `rho`
/// are not finite and positive (the exact zero branches stay separate).
/// A selection rule, not a bound: the action re-derives its tail.
pub fn laguerre_scale_for_degree(h: f64, rho: f64, degree: usize) -> Option<f64> {
    if !(h.is_finite() && rho.is_finite() && h > 0.0 && rho > 0.0) {
        return None;
    }
    let scale = 2.0 * (degree + 1) as f64 - h * rho;
    (scale.is_finite() && scale > 0.0).then(|| scale.min(LAGUERRE_SCALE_CAP))
}

/// Upper bounds of `phi_0 .. phi_4` at a real `z` in `[-600, 0]` as
/// intervals (the scalar branch's enclosures), for references.
pub fn scalar_phi_enclosure(z: Interval) -> CoreResult<[Interval; JOINT_PHI_TERMS]> {
    scalar_phi(z)
}

/// [`joint_phi_action`] without the rounding enclosures: the same degree,
/// coefficients and values, a recurrence in plain binary64, and a total
/// error that is always [`TotalErrorStatus::EstimateOnly`]. For timing
/// comparisons whose results are verified separately (POLY-03).
pub fn joint_phi_action_unbounded(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    basis: PolynomialBasis,
    budget: f64,
    cache: Option<&mut CoefficientCache>,
    work: &mut WorkCounters,
) -> CoreResult<JointPhiReport> {
    joint_phi_action_impl(
        op,
        h,
        input,
        basis,
        budget,
        cache,
        work,
        false,
        &LAGUERRE_SCALES,
    )
}

/// The action with power-of-two input normalization (re-audit R4,
/// POLY-DEV-01). Inputs inside [`NORMALIZATION_WINDOW`] run unchanged. Out
/// of it, `w` is scaled by `2^-s` (`s` the exponent of the largest entry)
/// and the budget by the same factor rounded down, the action runs on
/// `[1/2, 1)`-sized data, and the results are scaled back with every bound
/// rounded up and three terms added: (1) entries that lost bits when
/// scaled down, `||delta_k|| <= sqrt(count) 2^(s-1075)`, propagated by
/// `||phi_k(hA)||_2 <= 1/k!` (spectrum in `(-inf, 0]`, `h >= 0`); (2) and
/// (3) the rounding of columns and fused entries that land in the
/// subnormal range when scaled back, `sqrt(count) 2^-1074` each. A result
/// or bound above the binary64 range is [`POLYNOMIAL_RANGE_UNSUPPORTED`];
/// no bound becomes zero unless the exact error is.
#[allow(clippy::too_many_arguments)]
fn joint_phi_action_impl(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    basis: PolynomialBasis,
    budget: f64,
    cache: Option<&mut CoefficientCache>,
    work: &mut WorkCounters,
    bounds: bool,
    laguerre_scales: &[f64],
) -> CoreResult<JointPhiReport> {
    let entries: Vec<f64> = match input {
        JointPhiInput::Distinct(vectors) => vectors.iter().flatten().copied().collect(),
        JointPhiInput::SameVector { vector, .. } => vector.to_vec(),
    };
    let max = entries.iter().fold(0.0_f64, |acc, x| acc.max(x.abs()));
    let shift = if max.is_finite() && max > 0.0 {
        crate::binary_split(max).1
    } else {
        0
    };
    if shift.abs() <= NORMALIZATION_WINDOW
        || !(budget.is_finite() && budget > 0.0)
        || !(h.is_finite() && h >= 0.0)
    {
        return joint_phi_action_core(
            op,
            h,
            input,
            basis,
            budget,
            cache,
            work,
            bounds,
            laguerre_scales,
        );
    }
    let range = |what: &str| {
        CoreError::NonFinite(format!(
            "{POLYNOMIAL_RANGE_UNSUPPORTED}: {what} outside the binary64 range after scaling by 2^{shift}"
        ))
    };
    // Scaled inputs and, per term, the count of entries that lost bits.
    let scale_vector = |vector: &[f64]| -> (Vec<f64>, usize) {
        let scaled = vector
            .iter()
            .map(|x| scale_pow2(*x, -shift))
            .collect::<Vec<_>>();
        let lossy = vector
            .iter()
            .zip(&scaled)
            .filter(|(x, y)| scale_pow2(**y, shift) != **x)
            .count();
        (scaled, lossy)
    };
    let mut budget_scaled = scale_pow2(budget, -shift);
    if budget_scaled.is_finite() && scale_pow2(budget_scaled, shift) > budget {
        budget_scaled = budget_scaled.next_down();
    }
    if budget_scaled == f64::INFINITY {
        // A smaller budget is stricter, never looser.
        budget_scaled = f64::MAX;
    }
    if budget_scaled.is_nan() || budget_scaled <= 0.0 {
        return Err(range("the truncation budget"));
    }
    let mut lossy = [0_usize; JOINT_PHI_TERMS];
    let distinct;
    let same;
    let scaled_input = match input {
        JointPhiInput::Distinct(vectors) => {
            let mut scaled: [Vec<f64>; JOINT_PHI_TERMS] = Default::default();
            for k in 0..JOINT_PHI_TERMS {
                (scaled[k], lossy[k]) = scale_vector(&vectors[k]);
            }
            distinct = scaled;
            JointPhiInput::Distinct(&distinct)
        }
        JointPhiInput::SameVector { vector, scales } => {
            let (scaled, count) = scale_vector(vector);
            lossy = [count; JOINT_PHI_TERMS];
            same = scaled;
            JointPhiInput::SameVector {
                vector: &same,
                scales,
            }
        }
    };
    let scales = match input {
        JointPhiInput::Distinct(_) => [1.0; JOINT_PHI_TERMS],
        JointPhiInput::SameVector { scales, .. } => scales,
    };
    let mut report = joint_phi_action_core(
        op,
        h,
        scaled_input,
        basis,
        budget_scaled,
        cache,
        work,
        bounds,
        laguerre_scales,
    )?;
    // Scale back; count entries rounded into the subnormal range.
    let unscale = |values: &mut Vec<f64>| -> CoreResult<usize> {
        let mut rounded = 0;
        for value in values.iter_mut() {
            let back = scale_pow2(*value, shift);
            if !back.is_finite() {
                return Err(range("a result"));
            }
            if scale_pow2(back, -shift) != *value {
                rounded += 1;
            }
            *value = back;
        }
        Ok(rounded)
    };
    let mut perturbation = 0.0;
    for k in 0..JOINT_PHI_TERMS {
        let rounded = unscale(&mut report.columns[k])?;
        let components = &mut report.column_errors[k];
        components.truncation = scale_bound_up(components.truncation, shift)?;
        components.coefficient = scale_bound_up(components.coefficient, shift)?;
        components.summation = add_up(
            scale_bound_up(components.summation, shift)?,
            rounding_norm_up(rounded, -1074)?,
        )?;
        components.recurrence = components
            .recurrence
            .map(|value| scale_bound_up(value, shift))
            .transpose()?;
        components.recurrence_majorant = components
            .recurrence_majorant
            .map(|value| scale_bound_up(value, shift))
            .transpose()?;
        components.recurrence_adjoint = components
            .recurrence_adjoint
            .map(|value| scale_bound_up(value, shift))
            .transpose()?;
        if lossy[k] > 0 {
            let delta = mul_up(scales[k].abs(), rounding_norm_up(lossy[k], shift - 1075)?)?;
            let term = div_up(delta, factorial(k))?;
            components.normalization = term;
            perturbation = add_up(perturbation, term)?;
        }
    }
    let fused_rounded = unscale(&mut report.fused)?;
    let added = add_up(perturbation, rounding_norm_up(fused_rounded, -1074)?)?;
    report.fused_summation = add_up(
        scale_bound_up(report.fused_summation, shift)?,
        rounding_norm_up(fused_rounded, -1074)?,
    )?;
    report.truncation_bound_exact_arithmetic =
        scale_bound_up(report.truncation_bound_exact_arithmetic, shift)?;
    report.truncation_budget = budget;
    report.total_error = match report.total_error {
        TotalErrorStatus::Certified { bound } => TotalErrorStatus::Certified {
            bound: add_up(scale_bound_up(bound, shift)?, added)?,
        },
        TotalErrorStatus::EstimateOnly {
            reason,
            bounded_components,
        } => TotalErrorStatus::EstimateOnly {
            reason,
            bounded_components: add_up(scale_bound_up(bounded_components, shift)?, added)?,
        },
    };
    if let TotalErrorStatus::Certified { bound } = &report.total_error
        && !bound.is_finite()
    {
        return Err(range("the total error bound"));
    }
    report.laguerre_majorant_total = report
        .laguerre_majorant_total
        .map(|value| -> CoreResult<f64> { add_up(scale_bound_up(value, shift)?, added) })
        .transpose()?;
    report.laguerre_adjoint_total = report
        .laguerre_adjoint_total
        .map(|value| -> CoreResult<f64> { add_up(scale_bound_up(value, shift)?, added) })
        .transpose()?;
    report.normalization_shift = shift;
    let fused_norm = crate::safe_l2(&report.fused);
    let column_norms = report
        .columns
        .iter()
        .map(|c| crate::safe_l2(c))
        .sum::<f64>();
    report.condition_proxy = if fused_norm > 0.0 {
        column_norms / fused_norm
    } else if column_norms == 0.0 {
        1.0
    } else {
        f64::INFINITY
    };
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn joint_phi_action_core(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    basis: PolynomialBasis,
    budget: f64,
    cache: Option<&mut CoefficientCache>,
    work: &mut WorkCounters,
    bounds: bool,
    laguerre_scales: &[f64],
) -> CoreResult<JointPhiReport> {
    let n = op.dimension();
    if !(h.is_finite() && h >= 0.0) {
        return Err(unsupported(format!(
            "step h = {h:e} must be finite and >= 0"
        )));
    }
    if !(budget.is_finite() && budget > 0.0) {
        return Err(unsupported("truncation budget must be positive"));
    }
    // Block columns and the per-term scales applied after the recurrence.
    let (block, scales, input_name): (Vec<Vec<f64>>, [f64; JOINT_PHI_TERMS], &str) = match input {
        JointPhiInput::Distinct(vectors) => {
            if vectors.iter().any(|w| w.len() != n) {
                return Err(unsupported("every w_k must have the operator dimension"));
            }
            (vectors.to_vec(), [1.0; JOINT_PHI_TERMS], "distinct")
        }
        JointPhiInput::SameVector { vector, scales } => {
            if vector.len() != n {
                return Err(unsupported("vector must have the operator dimension"));
            }
            (vec![vector.to_vec()], scales, "same-vector")
        }
    };
    if !block.iter().flatten().chain(&scales).all(|x| x.is_finite()) {
        return Err(CoreError::NonFinite(
            "joint phi input contains NaN/Inf".into(),
        ));
    }
    let width = block.len();
    // ||w_k|| (upper) for every term.
    let norms = (0..JOINT_PHI_TERMS)
        .map(|k| {
            if width == 1 {
                mul_up(scales[k].abs(), norm_up(block[0].iter().copied())?)
            } else {
                norm_up(block[k].iter().copied())
            }
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let weight_factor = norms.iter().enumerate().try_fold(0.0, |acc, (k, norm)| {
        add_up(acc, div_up(*norm, factorial(k))?)
    })?;
    let SpectralEnclosure { lambda, rho, .. } = op.enclosure.clone();
    let scalar_branch = h == 0.0 || rho == 0.0 || lambda == rho;
    if lambda == rho && rho > 0.0 {
        let scalar =
            (0..n).all(|i| (0..n).all(|j| op.matrix[(i, j)] == if i == j { -lambda } else { 0.0 }));
        if !scalar {
            return Err(unsupported("a degenerate enclosure needs A = -lambda I"));
        }
    }
    if rho == 0.0 && op.matrix.as_slice().iter().any(|x| *x != 0.0) {
        return Err(unsupported("the enclosure [0, 0] needs A = 0"));
    }
    let evidence = op.enclosure.evidence.clone();
    let mut column_errors = vec![empty_components(); JOINT_PHI_TERMS];
    let mut columns = vec![vec![0.0; n]; JOINT_PHI_TERMS];
    let mut degree = 0;
    let mut laguerre_scale = None;
    let mut truncation = 0.0;
    let mut cache_hit = false;
    let mut recurrence_bounded = true;

    if weight_factor == 0.0 {
        // Every w_k is zero.
    } else if scalar_branch {
        let z = Interval::new(-mul_up(h, lambda)?, -mul_down(h, lambda)?)?;
        let phi = scalar_phi(z)?;
        for k in 0..JOINT_PHI_TERMS {
            let source = if width == 1 { &block[0] } else { &block[k] };
            let factor = Interval::point(scales[k])?.mul(phi[k])?;
            let value = midpoint(factor);
            let mut errors = Vec::with_capacity(n);
            for (out, w) in columns[k].iter_mut().zip(source) {
                *out = value * w;
                errors.push(distance_up(*out, factor.scale(*w)?)?);
            }
            column_errors[k].summation = norm_up(errors)?;
        }
    } else {
        let transform = choose_transform(op, h, basis, weight_factor, budget, laguerre_scales)?;
        degree = transform.degree;
        laguerre_scale = transform.laguerre_scale;
        truncation = mul_up(transform.tail_factor, weight_factor)?;
        let key = CoefficientKey {
            basis,
            operator_fingerprint: op.fingerprint.clone(),
            h_bits: h.to_bits(),
            degree,
            laguerre_scale_bits: laguerre_scale.unwrap_or(0.0).to_bits(),
        };
        let build = || match basis {
            PolynomialBasis::Chebyshev => chebyshev_coefficients(transform.a, transform.b, degree),
            PolynomialBasis::Laguerre => laguerre_coefficients(transform.a, degree),
        };
        let coefficients = match cache {
            Some(cache) => {
                if let Some(table) = cache.tables.get(&key) {
                    cache_hit = true;
                    work.poly_coefficient_reuses += 1;
                    table.clone()
                } else {
                    work.poly_coefficient_setups += 1;
                    let table = build()?;
                    cache.tables.insert(key, table.clone());
                    table
                }
            }
            None => {
                work.poly_coefficient_setups += 1;
                build()?
            }
        };
        let chosen = coefficients
            .iter()
            .map(|row| row.map(midpoint))
            .collect::<Vec<_>>();
        // Exact sums of chosen * t_n per term and block column, enclosed,
        // and the computed sums.
        let mut sums_lo = vec![vec![vec![0.0; n]; width]; JOINT_PHI_TERMS];
        let mut sums_hi = sums_lo.clone();
        let mut sums = sums_lo.clone();
        let mut local = vec![Vec::with_capacity(degree); width];
        let mut accumulate = |index: usize, vectors: &[Vec<f64>]| -> CoreResult<()> {
            for k in 0..JOINT_PHI_TERMS {
                let c = chosen[index][k];
                for (column, vector) in vectors.iter().enumerate() {
                    if width > 1 && column != k {
                        continue;
                    }
                    for i in 0..n {
                        let x = vector[i];
                        sums[k][column][i] += c * x;
                        if bounds {
                            sums_lo[k][column][i] =
                                add_down(sums_lo[k][column][i], mul_down(c, x)?)?;
                            sums_hi[k][column][i] = add_up(sums_hi[k][column][i], mul_up(c, x)?)?;
                        }
                    }
                }
            }
            Ok(())
        };
        work.poly_block_allocations += 3;
        let mut previous: Option<Vec<Vec<f64>>> = None;
        let mut current = block.clone();
        accumulate(0, &current)?;
        for step in 0..degree {
            let mut next = Vec::with_capacity(width);
            for (column, vector) in current.iter().enumerate() {
                let prev = previous.as_ref().map(|p| p[column].as_slice());
                if bounds {
                    let (value, residual) =
                        recurrence_step(&op.matrix, &transform, step, prev, vector)?;
                    local[column].push(residual);
                    next.push(value);
                } else {
                    next.push(recurrence_step_plain(
                        &op.matrix, &transform, step, prev, vector,
                    )?);
                }
            }
            work.poly_block_products += 1;
            work.poly_vector_products += width as u64;
            accumulate(step + 1, &next)?;
            previous = Some(current);
            current = next;
        }
        // Error components per term.
        for k in 0..JOINT_PHI_TERMS {
            let column = if width == 1 { 0 } else { k };
            if !bounds {
                for i in 0..n {
                    columns[k][i] = scales[k] * sums[k][column][i];
                }
                continue;
            }
            let source_norm = norm_up(block[column].iter().copied())?;
            let norm_bound = |index: usize| -> CoreResult<f64> {
                match transform.basis {
                    PolynomialBasis::Chebyshev => exp_up(mul_up(index as f64, transform.eta)?),
                    PolynomialBasis::Laguerre => Ok(transform.laguerre_norm),
                }
            };
            let mut coefficient_error = 0.0;
            for (index, row) in coefficients.iter().enumerate() {
                let radius = distance_up(chosen[index][k], row[k])?;
                coefficient_error = add_up(coefficient_error, mul_up(radius, norm_bound(index)?)?)?;
            }
            coefficient_error = mul_up(coefficient_error, source_norm)?;
            let recurrence = match transform.basis {
                PolynomialBasis::Chebyshev => {
                    // ||t_n - T_n w|| <= sum_{j<n} (n-j) e^{(n-1-j) eta} eps_j.
                    let mut total = 0.0;
                    for (index, row) in chosen.iter().enumerate().take(degree + 1).skip(1) {
                        let mut propagated = 0.0;
                        for (j, eps) in local[column].iter().enumerate().take(index) {
                            let gain = mul_up(
                                (index - j) as f64,
                                exp_up(mul_up((index - 1 - j) as f64, transform.eta)?)?,
                            )?;
                            propagated = add_up(propagated, mul_up(gain, *eps)?)?;
                        }
                        total = add_up(total, mul_up(row[k].abs(), propagated)?)?;
                    }
                    Some(total)
                }
                PolynomialBasis::Laguerre => {
                    recurrence_bounded = false;
                    None
                }
            };
            let majorant = match transform.basis {
                PolynomialBasis::Chebyshev => None,
                PolynomialBasis::Laguerre => {
                    let propagated = laguerre_majorant(&local[column], transform.laguerre_extent)?;
                    let mut total = 0.0;
                    for (row, bound) in chosen.iter().zip(&propagated).skip(1) {
                        total = add_up(total, mul_up(row[k].abs(), *bound)?)?;
                    }
                    Some(mul_up(scales[k].abs(), total)?)
                }
            };
            let adjoint = match transform.basis {
                PolynomialBasis::Laguerre
                    if degree <= crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT =>
                {
                    let stored = chosen.iter().map(|row| row[k]).collect::<Vec<_>>();
                    let bound = crate::laguerre_adjoint::laguerre_adjoint_recurrence_bound(
                        &stored,
                        transform.laguerre_extent,
                        &local[column],
                        crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEPTH,
                    )?;
                    Some(mul_up(scales[k].abs(), bound)?)
                }
                _ => None,
            };
            // Summation of chosen * t_n and the scaling by scales[k].
            let scale = Interval::point(scales[k])?;
            let mut errors = Vec::with_capacity(n);
            for i in 0..n {
                let enclosed = Interval::new(sums_lo[k][column][i], sums_hi[k][column][i])?;
                let computed = scales[k] * sums[k][column][i];
                columns[k][i] = computed;
                errors.push(distance_up(computed, scale.mul(enclosed)?)?);
            }
            let scale_abs = scales[k].abs();
            column_errors[k] = ErrorComponents {
                truncation: mul_up(transform.tail_factor, div_up(norms[k], factorial(k))?)?,
                coefficient: mul_up(scale_abs, coefficient_error)?,
                recurrence: recurrence
                    .map(|value| mul_up(scale_abs, value))
                    .transpose()?,
                summation: norm_up(errors)?,
                normalization: 0.0,
                recurrence_majorant: majorant,
                recurrence_adjoint: adjoint,
            };
        }
    }

    // Fused sum and its rounding.
    let mut fused = vec![0.0; n];
    let mut fused_errors = Vec::with_capacity(n);
    for (i, out) in fused.iter_mut().enumerate() {
        let mut lo = 0.0;
        let mut hi = 0.0;
        for column in &columns {
            *out += column[i];
            lo = add_down(lo, column[i])?;
            hi = add_up(hi, column[i])?;
        }
        fused_errors.push(distance_up(*out, Interval::new(lo, hi)?)?);
    }
    if !fused.iter().all(|x| x.is_finite()) {
        return Err(CoreError::NonFinite(
            "joint phi action produced NaN/Inf".into(),
        ));
    }
    let fused_summation = norm_up(fused_errors)?;
    let mut bounded = fused_summation;
    for components in &column_errors {
        bounded = add_up(bounded, components.truncation)?;
        bounded = add_up(bounded, components.coefficient)?;
        bounded = add_up(bounded, components.summation)?;
        if let Some(recurrence) = components.recurrence {
            bounded = add_up(bounded, recurrence)?;
        }
    }
    let laguerre_majorant_total = if column_errors
        .iter()
        .any(|c| c.recurrence_majorant.is_some())
    {
        let mut total = bounded;
        for components in &column_errors {
            total = add_up(total, components.recurrence_majorant.unwrap_or(0.0))?;
        }
        Some(total)
    } else {
        None
    };
    let verified = matches!(evidence, EnclosureEvidence::Gershgorin);
    let laguerre_adjoint_total =
        if verified && column_errors.iter().all(|c| c.recurrence_adjoint.is_some()) {
            let mut total = bounded;
            for components in &column_errors {
                total = add_up(total, components.recurrence_adjoint.unwrap_or(0.0))?;
            }
            Some(total)
        } else {
            None
        };
    let total_error = if !bounds && !scalar_branch && weight_factor != 0.0 {
        TotalErrorStatus::EstimateOnly {
            reason: format!("{TOTAL_ERROR_NOT_CERTIFIED}: rounding bounds not computed"),
            bounded_components: 0.0,
        }
    } else if !recurrence_bounded {
        TotalErrorStatus::EstimateOnly {
            reason: format!(
                "{TOTAL_ERROR_NOT_CERTIFIED}: the Laguerre recurrence rounding is not propagated"
            ),
            bounded_components: bounded,
        }
    } else if let EnclosureEvidence::Declared { source } = &evidence {
        if scalar_branch && h == 0.0 {
            TotalErrorStatus::Certified { bound: bounded }
        } else {
            TotalErrorStatus::EstimateOnly {
                reason: format!(
                    "{TOTAL_ERROR_NOT_CERTIFIED}: spectral enclosure declared by {source}, not verified"
                ),
                bounded_components: bounded,
            }
        }
    } else {
        TotalErrorStatus::Certified { bound: bounded }
    };
    let fused_norm = crate::safe_l2(&fused);
    let column_norms = columns.iter().map(|c| crate::safe_l2(c)).sum::<f64>();
    let condition_proxy = if fused_norm > 0.0 {
        column_norms / fused_norm
    } else if column_norms == 0.0 {
        1.0
    } else {
        f64::INFINITY
    };
    Ok(JointPhiReport {
        schema: JOINT_PHI_SCHEMA.into(),
        basis,
        input: input_name.into(),
        branch: if scalar_branch {
            "scalar"
        } else {
            "recurrence"
        }
        .into(),
        dimension: n,
        degree,
        laguerre_scale,
        columns,
        fused,
        truncation_bound_exact_arithmetic: truncation,
        truncation_budget: budget,
        column_errors,
        fused_summation,
        total_error,
        condition_proxy,
        evidence,
        coefficient_cache_hit: cache_hit,
        normalization_shift: 0,
        execution: if bounds {
            EXECUTION_CERTIFIED
        } else {
            EXECUTION_UNBOUNDED_TIMING
        }
        .into(),
        laguerre_majorant_total,
        laguerre_adjoint_total,
    })
}

/// [`joint_phi_action`] on a dense matrix, falling back to the dense
/// augmented exponential ([`dense_phi_combination`]) when the matrix is
/// outside the polynomial domain. The fallback is counted and has no error
/// bound.
#[allow(clippy::too_many_arguments)]
pub fn joint_phi_action_or_dense(
    matrix: &DenseMatrix,
    lambda: f64,
    rho: f64,
    h: f64,
    vectors: &[Vec<f64>; JOINT_PHI_TERMS],
    basis: PolynomialBasis,
    budget: f64,
    work: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, Option<JointPhiReport>)> {
    let attempt = SymmetricNonpositiveOperator::new(matrix.clone(), lambda, rho, "caller")
        .and_then(|op| {
            joint_phi_action(
                &op,
                h,
                JointPhiInput::Distinct(vectors),
                basis,
                budget,
                None,
                work,
            )
        });
    match attempt {
        Ok(report) => Ok((report.fused.clone(), Some(report))),
        Err(CoreError::InvalidInput(message))
            if message.starts_with(POLYNOMIAL_DOMAIN_UNSUPPORTED) =>
        {
            work.poly_fallbacks += 1;
            Ok((dense_phi_combination(matrix, h, vectors)?, None))
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_enclose_known_values() {
        let e = exp_interval(Interval::point(1.0).unwrap()).unwrap();
        assert!(e.lo <= std::f64::consts::E && std::f64::consts::E <= e.hi);
        assert!(e.hi - e.lo <= 64.0 * f64::EPSILON * e.hi, "{e:?}");
        let phi = scalar_phi(Interval::point(-7.3).unwrap()).unwrap();
        // phi_1(-7.3) = (1 - e^-7.3) / 7.3.
        let expected = (1.0 - (-7.3_f64).exp()) / 7.3;
        assert!(
            (phi[1].lo..=phi[1].hi).contains(&expected) || (expected - phi[1].hi).abs() < 1e-15
        );
    }
}
