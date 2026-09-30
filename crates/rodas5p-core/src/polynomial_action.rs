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
};

pub const JOINT_PHI_SCHEMA: &str = "vigilode-joint-phi-polynomial-v1";
/// Number of phi terms `w_0 .. w_4`.
pub const JOINT_PHI_TERMS: usize = 5;
pub const POLYNOMIAL_DOMAIN_UNSUPPORTED: &str = "POLYNOMIAL_DOMAIN_OR_ACCURACY_UNSUPPORTED";
pub const TOTAL_ERROR_NOT_CERTIFIED: &str = "TOTAL_ERROR_NOT_CERTIFIED";
/// Largest `|a|` and `b` for which the coefficient series stay in range.
pub const COEFFICIENT_RANGE_LIMIT: f64 = 600.0;
/// Laguerre scales `L` (`beta = rho / L`); the cap 16 is a policy.
pub const LAGUERRE_SCALES: [f64; 5] = [1.0, 2.0, 4.0, 8.0, 16.0];
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
fn exp_up(x: f64) -> CoreResult<f64> {
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
}

/// Upper bound on the Euclidean norm.
fn norm_up(values: impl IntoIterator<Item = f64>) -> CoreResult<f64> {
    let sum = values.into_iter().try_fold(0.0, |acc, value| {
        add_up(acc, mul_up(value.abs(), value.abs())?)
    })?;
    sqrt_up(sum)
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
    a: Interval,
    b: Interval,
}

fn choose_transform(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    basis: PolynomialBasis,
    weight_factor: f64,
    budget: f64,
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
            for scale in LAGUERRE_SCALES {
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

fn empty_components() -> ErrorComponents {
    ErrorComponents {
        truncation: 0.0,
        coefficient: 0.0,
        recurrence: Some(0.0),
        summation: 0.0,
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
        let transform = choose_transform(op, h, basis, weight_factor, budget)?;
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
                        sums_lo[k][column][i] = add_down(sums_lo[k][column][i], mul_down(c, x)?)?;
                        sums_hi[k][column][i] = add_up(sums_hi[k][column][i], mul_up(c, x)?)?;
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
                let (value, residual) = recurrence_step(
                    &op.matrix,
                    &transform,
                    step,
                    previous.as_ref().map(|p| p[column].as_slice()),
                    vector,
                )?;
                local[column].push(residual);
                next.push(value);
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
    let total_error = if !recurrence_bounded {
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
