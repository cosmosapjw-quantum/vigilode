//! Output bound for the rounding of phi-combination weights (re-audit R3 of
//! 2026-10-01, ARITH-02). Research only.
//!
//! The weights `w_k = h^k b_k` are formed in binary64; the stored `w~_k`
//! differ from the exact ones by `delta_k`, and the action changes by
//! `e = sum_k phi_k(hA) delta_k`, so `||e|| <= sum_k C_k ||delta_k||` for any
//! `C_k >= ||phi_k(hA)||`. [`bound_transform_error`] evaluates this with
//! [`ExpBound`] (a mantissa and an integer exponent, rounded upward), so a
//! bound far below the smallest subnormal stays nonzero, and supplies `C_k`
//! only for operator classes it can verify:
//!
//! * [`TransformOperatorClass::Nilpotent`]: `A` strictly triangular, so
//!   `A^n = 0` and `||phi_k(hA)|| <= sum_{j<n} (|h| N)^j / (j + k)!` with
//!   `N = max(||A||_1, ||A||_inf) >= ||A||_2`;
//! * [`TransformOperatorClass::Dissipative`]: the Gershgorin bound on the
//!   symmetric part of `sign(h) A` is at most 0, so the 2-norm logarithmic
//!   norm of `hA` is at most 0 and `||phi_k(hA)||_2 <= 1 / k!`.
//!
//! Any other operator is [`TransformBound::Unbounded`]; there is no generic
//! matrix-free bound here, and a bound that exceeds the caller's tolerance is
//! a rejection, never a reason to relax it.

use crate::directed::{add_up, mul_up, sub_up};
use crate::{CoreError, CoreResult, DenseMatrix, binary_split};
use serde::{Deserialize, Serialize};

/// An upper bound `m 2^e` with `m` in [1/2, 1), or exactly 0.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExpBound {
    pub mantissa: f64,
    pub exponent: i64,
}

impl ExpBound {
    pub const ZERO: Self = Self {
        mantissa: 0.0,
        exponent: 0,
    };

    /// The exact nonnegative value `|x|`.
    pub fn exact(x: f64) -> CoreResult<Self> {
        if !x.is_finite() {
            return Err(CoreError::NonFinite(
                "ExpBound of a non-finite value".into(),
            ));
        }
        if x == 0.0 {
            return Ok(Self::ZERO);
        }
        let (mantissa, exponent) = binary_split(x.abs());
        Ok(Self { mantissa, exponent })
    }

    fn normalized(mantissa: f64, exponent: i64) -> Self {
        if mantissa == 0.0 {
            return Self::ZERO;
        }
        let (m, shift) = binary_split(mantissa);
        Self {
            mantissa: m,
            exponent: exponent + shift,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.mantissa == 0.0
    }

    // Fallible (an overflow is an error), so not `std::ops`.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Self) -> CoreResult<Self> {
        if self.is_zero() || other.is_zero() {
            return Ok(Self::ZERO);
        }
        Ok(Self::normalized(
            mul_up(self.mantissa, other.mantissa)?,
            self.exponent + other.exponent,
        ))
    }

    /// Upper bound on the sum. A term too small to shift into the larger
    /// one's scale adds one unit of its last place, never 0.
    // Fallible (an overflow is an error), so not `std::ops`.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Self) -> CoreResult<Self> {
        if self.is_zero() {
            return Ok(other);
        }
        if other.is_zero() {
            return Ok(self);
        }
        let (big, small) = if self.exponent >= other.exponent {
            (self, other)
        } else {
            (other, self)
        };
        let shift = big.exponent - small.exponent;
        let scaled = if shift > 1000 {
            0.0
        } else {
            small.mantissa * 2.0_f64.powi(-(shift as i32))
        };
        let scaled = if scaled == 0.0 || scaled * 2.0_f64.powi(shift as i32) != small.mantissa {
            // The shifted term was rounded or lost: bound it by one ulp of
            // the result's scale.
            scaled.next_up()
        } else {
            scaled
        };
        Ok(Self::normalized(
            add_up(big.mantissa, scaled)?,
            big.exponent,
        ))
    }

    /// Divide by a positive integer, rounding up.
    pub fn div_integer(self, divisor: f64) -> CoreResult<Self> {
        if self.is_zero() {
            return Ok(Self::ZERO);
        }
        Ok(Self::normalized(
            crate::directed::div_up(self.mantissa, divisor)?,
            self.exponent,
        ))
    }

    /// The smallest binary64 value at least this bound: `5e-324` for any
    /// nonzero bound below the subnormal range, `+inf` above `f64::MAX`.
    pub fn to_f64_up(&self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        if self.exponent > 1024 {
            return f64::INFINITY;
        }
        if self.exponent < -1074 {
            return f64::from_bits(1);
        }
        let value = crate::binary_scale(self.mantissa, self.exponent);
        if value == 0.0 {
            return f64::from_bits(1);
        }
        if crate::binary_scale(value, -self.exponent) == self.mantissa {
            value
        } else {
            value.next_up()
        }
    }

    /// `self <= bound` for a nonnegative binary64 `bound`.
    pub fn at_most(&self, bound: f64) -> bool {
        if self.is_zero() {
            return bound >= 0.0;
        }
        match ExpBound::exact(bound) {
            Ok(other) if !other.is_zero() => {
                self.exponent < other.exponent
                    || (self.exponent == other.exponent && self.mantissa <= other.mantissa)
            }
            _ => false,
        }
    }
}

/// The operator class that supplied `C_k`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum TransformOperatorClass {
    Nilpotent { index: usize },
    Dissipative,
}

/// The bound, or why there is none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum TransformBound {
    Bounded {
        class: TransformOperatorClass,
        /// `||e||_2 <= upper`.
        upper: ExpBound,
        /// `||delta_k||_2` bounds per order.
        delta: Vec<ExpBound>,
        /// `C_k` per order.
        operator: Vec<ExpBound>,
    },
    Unbounded {
        reason: String,
    },
}

impl TransformBound {
    /// Accept iff bounded with `upper <= tolerance`.
    pub fn admits(&self, tolerance: f64) -> bool {
        matches!(self, Self::Bounded { upper, .. } if upper.at_most(tolerance))
    }
}

/// `|h|^k` as a mantissa interval and exponent, by upward/downward squaring.
fn power_enclosure(h: f64, k: u32) -> CoreResult<(f64, f64, i64)> {
    use crate::directed::mul_down;
    let (m, e) = binary_split(h.abs());
    let (mut lo, mut hi, mut exponent) = (1.0_f64, 1.0_f64, 0_i64);
    let (mut sq_lo, mut sq_hi, mut sq_e) = (m, m, e);
    let mut remaining = k;
    let renormalize = |lo: f64, hi: f64, e: i64| {
        let (_, shift) = binary_split(hi);
        let factor = 2.0_f64.powi(-(shift as i32));
        (lo * factor, hi * factor, e + shift)
    };
    while remaining > 0 {
        if remaining & 1 == 1 {
            (lo, hi, exponent) =
                renormalize(mul_down(lo, sq_lo)?, mul_up(hi, sq_hi)?, exponent + sq_e);
        }
        remaining >>= 1;
        if remaining > 0 {
            (sq_lo, sq_hi, sq_e) =
                renormalize(mul_down(sq_lo, sq_lo)?, mul_up(sq_hi, sq_hi)?, 2 * sq_e);
        }
    }
    Ok((lo, hi, exponent))
}

/// Upper bound on `|w_exact - w_stored|` for `w_exact = h^k b`.
fn entry_delta(h: f64, k: u32, b: f64, stored: f64) -> CoreResult<ExpBound> {
    use crate::directed::mul_down;
    if b == 0.0 || (h == 0.0 && k > 0) {
        return ExpBound::exact(stored);
    }
    if k == 0 {
        // w_0 = b_0 is stored as is.
        return ExpBound::exact(if stored == b {
            0.0
        } else {
            sub_up(b.abs().max(stored.abs()), b.abs().min(stored.abs()))?
        });
    }
    let (p_lo, p_hi, p_e) = power_enclosure(h, k)?;
    let (m_b, e_b) = binary_split(b.abs());
    let lo = mul_down(m_b, p_lo)?;
    let hi = mul_up(m_b, p_hi)?;
    let exponent = e_b + p_e;
    let exact_sign =
        if b < 0.0 { -1.0 } else { 1.0 } * if h < 0.0 && k % 2 == 1 { -1.0 } else { 1.0 };
    if stored == 0.0 {
        return Ok(ExpBound::normalized(hi, exponent));
    }
    let (m_w, e_w) = binary_split(stored.abs());
    let shift = e_w - exponent;
    let relative = if shift < -1000 {
        0.0
    } else {
        m_w * 2.0_f64.powi(shift as i32)
    };
    let shifted_exactly = relative != 0.0 && relative * 2.0_f64.powi(-(shift as i32)) == m_w;
    if shifted_exactly && lo == hi && relative == lo && stored.signum() == exact_sign {
        // The stored weight is the exact one.
        return Ok(ExpBound::ZERO);
    }
    let difference = if stored.signum() != exact_sign {
        add_up(hi, relative.next_up())?
    } else {
        sub_up(hi, relative)?.max(sub_up(relative, lo)?).max(0.0)
    };
    // A shifted value that was rounded widens the difference by its ulp.
    let difference = if shifted_exactly {
        difference
    } else {
        add_up(difference, relative.next_up() - relative)?
    };
    Ok(ExpBound::normalized(difference, exponent))
}

fn factorial(n: usize) -> f64 {
    (1..=n).fold(1.0, |acc, value| acc * value as f64)
}

/// `C_k` for `k = 0..=p` when the class is verified.
fn operator_bounds(
    matrix: &DenseMatrix,
    h: f64,
    p: usize,
) -> CoreResult<Option<(TransformOperatorClass, Vec<ExpBound>)>> {
    let n = matrix.nrows();
    let strictly_lower = (0..n).all(|i| (i..n).all(|j| matrix[(i, j)] == 0.0));
    let strictly_upper = (0..n).all(|i| (0..=i).all(|j| matrix[(i, j)] == 0.0));
    let mut candidates = Vec::new();
    if strictly_lower || strictly_upper {
        let mut row_max = ExpBound::ZERO;
        let mut column_max = ExpBound::ZERO;
        for i in 0..n {
            let mut row = ExpBound::ZERO;
            let mut column = ExpBound::ZERO;
            for j in 0..n {
                row = row.add(ExpBound::exact(matrix[(i, j)])?)?;
                column = column.add(ExpBound::exact(matrix[(j, i)])?)?;
            }
            for (acc, value) in [(&mut row_max, row), (&mut column_max, column)] {
                if value.exponent > acc.exponent
                    || (value.exponent == acc.exponent && value.mantissa > acc.mantissa)
                    || acc.is_zero()
                {
                    *acc = value;
                }
            }
        }
        let norm = if row_max.exponent > column_max.exponent
            || (row_max.exponent == column_max.exponent && row_max.mantissa >= column_max.mantissa)
        {
            row_max
        } else {
            column_max
        };
        let scaled = norm.mul(ExpBound::exact(h)?)?;
        let mut bounds = Vec::with_capacity(p + 1);
        for k in 0..=p {
            let mut total = ExpBound::ZERO;
            let mut power = ExpBound::exact(1.0)?;
            for j in 0..n.max(1) {
                total = total.add(power.div_integer(factorial(j + k))?)?;
                power = power.mul(scaled)?;
            }
            bounds.push(total);
        }
        candidates.push((
            TransformOperatorClass::Nilpotent { index: n.max(1) },
            bounds,
        ));
    }
    let sign = if h < 0.0 { -1.0 } else { 1.0 };
    let mut dissipative = true;
    for i in 0..n {
        let mut radius = sign * matrix[(i, i)];
        for j in 0..n {
            if j != i {
                let half = (0.5 * (matrix[(i, j)] + matrix[(j, i)])).abs();
                radius = add_up(radius, half.next_up())?;
            }
        }
        if radius > 0.0 {
            dissipative = false;
        }
    }
    if dissipative {
        let bounds = (0..=p)
            .map(|k| ExpBound::exact(1.0)?.div_integer(factorial(k)))
            .collect::<CoreResult<Vec<_>>>()?;
        candidates.push((TransformOperatorClass::Dissipative, bounds));
    }
    // Take the class with the smaller total weight on the highest order.
    Ok(candidates.into_iter().min_by(|left, right| {
        let a = left.1.last().copied().unwrap_or(ExpBound::ZERO);
        let b = right.1.last().copied().unwrap_or(ExpBound::ZERO);
        (a.exponent, a.mantissa)
            .partial_cmp(&(b.exponent, b.mantissa))
            .unwrap_or(std::cmp::Ordering::Equal)
    }))
}

/// Bound `||sum_k phi_k(hA) (w_exact_k - w_stored_k)||_2` for
/// `w_exact_k = h^k b_k`.
pub fn bound_transform_error(
    matrix: &DenseMatrix,
    h: f64,
    vectors: &[Vec<f64>],
    stored: &[Vec<f64>],
) -> CoreResult<TransformBound> {
    let n = matrix.nrows();
    if matrix.ncols() != n
        || vectors.len() != stored.len()
        || vectors.iter().chain(stored).any(|vector| vector.len() != n)
        || !h.is_finite()
    {
        return Err(CoreError::Dimension(
            "transform bound: shapes or step invalid".into(),
        ));
    }
    let Some((class, operator)) = operator_bounds(matrix, h, vectors.len().saturating_sub(1))?
    else {
        return Ok(TransformBound::Unbounded {
            reason: "TRANSFORM_ERROR_UNBOUNDED: no verified operator class".into(),
        });
    };
    let mut delta = Vec::with_capacity(vectors.len());
    let mut upper = ExpBound::ZERO;
    for (k, (exact, rounded)) in vectors.iter().zip(stored).enumerate() {
        let mut norm = ExpBound::ZERO;
        for (b, w) in exact.iter().zip(rounded) {
            norm = norm.add(entry_delta(h, k as u32, *b, *w)?)?;
        }
        upper = upper.add(operator[k].mul(norm)?)?;
        delta.push(norm);
    }
    Ok(TransformBound::Bounded {
        class,
        upper,
        delta,
        operator,
    })
}
