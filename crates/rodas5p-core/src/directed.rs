//! Directed rounding and outward intervals in binary64 (re-audit R3 of
//! 2026-10-01: ARITH-02, ARITH-04, HOM-02..04).
//!
//! Each operation rounds to nearest and then moves one step outward only
//! when the exact result lies beyond the rounded one. For addition the error
//! term of TwoSum is exact for every finite input. For multiplication,
//! division and square root the error term comes from a fused multiply-add,
//! which is exact while the product stays clear of the subnormal range; below
//! `2^-960` the result is moved outward unconditionally instead. A NaN or
//! infinite result is an error: an enclosure never silently becomes
//! unbounded or undefined (fail closed).

use crate::{CoreError, CoreResult};

/// Below this magnitude an FMA error term may itself be rounded, so the
/// outward step is taken unconditionally.
const EXACT_FMA_FLOOR: f64 = 1.0e-289;

fn finite(value: f64, operation: &str) -> CoreResult<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CoreError::NonFinite(format!(
            "directed rounding: {operation} is not finite"
        )))
    }
}

/// Exact error `e` of `s = fl(a + b)`: `a + b = s + e` (Knuth TwoSum).
fn two_sum_error(a: f64, b: f64, s: f64) -> f64 {
    let bb = s - a;
    (a - (s - bb)) + (b - bb)
}

/// Upper bound on `a + b`.
pub fn add_up(a: f64, b: f64) -> CoreResult<f64> {
    let s = finite(a + b, "sum")?;
    Ok(if two_sum_error(a, b, s) > 0.0 {
        s.next_up()
    } else {
        s
    })
}

/// Lower bound on `a + b`.
pub fn add_down(a: f64, b: f64) -> CoreResult<f64> {
    let s = finite(a + b, "sum")?;
    Ok(if two_sum_error(a, b, s) < 0.0 {
        s.next_down()
    } else {
        s
    })
}

pub fn sub_up(a: f64, b: f64) -> CoreResult<f64> {
    add_up(a, -b)
}

pub fn sub_down(a: f64, b: f64) -> CoreResult<f64> {
    add_down(a, -b)
}

/// Upper bound on `a * b`.
pub fn mul_up(a: f64, b: f64) -> CoreResult<f64> {
    let p = finite(a * b, "product")?;
    if a == 0.0 || b == 0.0 {
        return Ok(p);
    }
    if p.abs() < EXACT_FMA_FLOOR {
        return Ok(p.next_up());
    }
    Ok(if a.mul_add(b, -p) > 0.0 {
        p.next_up()
    } else {
        p
    })
}

/// Lower bound on `a * b`.
pub fn mul_down(a: f64, b: f64) -> CoreResult<f64> {
    let p = finite(a * b, "product")?;
    if a == 0.0 || b == 0.0 {
        return Ok(p);
    }
    if p.abs() < EXACT_FMA_FLOOR {
        return Ok(p.next_down());
    }
    Ok(if a.mul_add(b, -p) < 0.0 {
        p.next_down()
    } else {
        p
    })
}

/// Upper bound on `a / b` for `b != 0`.
pub fn div_up(a: f64, b: f64) -> CoreResult<f64> {
    if b == 0.0 {
        return Err(CoreError::InvalidInput(
            "directed rounding: division by zero".into(),
        ));
    }
    let q = finite(a / b, "quotient")?;
    if a == 0.0 {
        return Ok(q);
    }
    if q.abs() < EXACT_FMA_FLOOR || (q * b).abs() < EXACT_FMA_FLOOR {
        return Ok(q.next_up());
    }
    // a - q b > 0 with b > 0 (or < 0 with b < 0) means a / b > q.
    let remainder = (-q).mul_add(b, a);
    Ok(if remainder * b.signum() > 0.0 {
        q.next_up()
    } else {
        q
    })
}

/// Lower bound on `a / b` for `b != 0`.
pub fn div_down(a: f64, b: f64) -> CoreResult<f64> {
    Ok(-div_up(-a, b)?)
}

/// Upper bound on `sqrt(x)` for `x >= 0`.
pub fn sqrt_up(x: f64) -> CoreResult<f64> {
    if x.is_nan() || x < 0.0 {
        return Err(CoreError::InvalidInput(
            "directed rounding: square root of a negative number".into(),
        ));
    }
    let s = finite(x.sqrt(), "square root")?;
    if x == 0.0 {
        return Ok(0.0);
    }
    if x < EXACT_FMA_FLOOR {
        return Ok(s.next_up());
    }
    Ok(if s.mul_add(s, -x) < 0.0 {
        s.next_up()
    } else {
        s
    })
}

/// Lower bound on `sqrt(x)` for `x >= 0`.
pub fn sqrt_down(x: f64) -> CoreResult<f64> {
    if x.is_nan() || x < 0.0 {
        return Err(CoreError::InvalidInput(
            "directed rounding: square root of a negative number".into(),
        ));
    }
    let s = finite(x.sqrt(), "square root")?;
    if x == 0.0 {
        return Ok(0.0);
    }
    if x < EXACT_FMA_FLOOR {
        return Ok(s.next_down().max(0.0));
    }
    Ok(if s.mul_add(s, -x) > 0.0 {
        s.next_down()
    } else {
        s
    })
}

/// Upper bound on a sum of nonnegative terms, accumulated left to right.
pub fn sum_up(values: impl IntoIterator<Item = f64>) -> CoreResult<f64> {
    values.into_iter().try_fold(0.0, add_up)
}

/// A closed interval `[lo, hi]` of reals with binary64 endpoints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}

impl Interval {
    pub fn new(lo: f64, hi: f64) -> CoreResult<Self> {
        if !(lo.is_finite() && hi.is_finite() && lo <= hi) {
            return Err(CoreError::InvalidInput(format!(
                "interval [{lo:e}, {hi:e}] is not a finite ordered pair"
            )));
        }
        Ok(Self { lo, hi })
    }

    /// The degenerate interval of an exactly represented real.
    pub fn point(value: f64) -> CoreResult<Self> {
        Self::new(value, value)
    }

    /// The exact real sum `a + b` of two binary64 numbers, enclosed.
    pub fn exact_sum(a: f64, b: f64) -> CoreResult<Self> {
        Self::new(add_down(a, b)?, add_up(a, b)?)
    }

    pub fn contains(&self, value: f64) -> bool {
        self.lo <= value && value <= self.hi
    }

    pub fn contains_zero(&self) -> bool {
        self.contains(0.0)
    }

    /// Upper bound on `|x|` over the interval.
    pub fn mag(&self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }

    /// Lower bound on `|x|` over the interval.
    pub fn mig(&self) -> f64 {
        if self.contains_zero() {
            0.0
        } else {
            self.lo.abs().min(self.hi.abs())
        }
    }

    // Fallible (an overflow is an error), so not `std::ops`.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Self) -> CoreResult<Self> {
        Self::new(add_down(self.lo, other.lo)?, add_up(self.hi, other.hi)?)
    }

    // Fallible (an overflow is an error), so not `std::ops`.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, other: Self) -> CoreResult<Self> {
        Self::new(sub_down(self.lo, other.hi)?, sub_up(self.hi, other.lo)?)
    }

    // Fallible (an overflow is an error), so not `std::ops`.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Self) -> CoreResult<Self> {
        let corners = [
            (self.lo, other.lo),
            (self.lo, other.hi),
            (self.hi, other.lo),
            (self.hi, other.hi),
        ];
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for (a, b) in corners {
            lo = lo.min(mul_down(a, b)?);
            hi = hi.max(mul_up(a, b)?);
        }
        Self::new(lo, hi)
    }

    /// Quotient; an error when the divisor contains 0.
    // Fallible (an overflow is an error), so not `std::ops`.
    #[allow(clippy::should_implement_trait)]
    pub fn div(self, other: Self) -> CoreResult<Self> {
        if other.contains_zero() {
            return Err(CoreError::InvalidInput(
                "interval division by an interval containing 0".into(),
            ));
        }
        let corners = [
            (self.lo, other.lo),
            (self.lo, other.hi),
            (self.hi, other.lo),
            (self.hi, other.hi),
        ];
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for (a, b) in corners {
            lo = lo.min(div_down(a, b)?);
            hi = hi.max(div_up(a, b)?);
        }
        Self::new(lo, hi)
    }

    pub fn scale(self, factor: f64) -> CoreResult<Self> {
        self.mul(Self::point(factor)?)
    }
}

impl std::ops::Neg for Interval {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
}

/// An enclosure of `e^x` for finite `x` (review DAG node REV-02): with
/// `k = round(x / ln 2)` and `r = x - k [ln 2]` enclosed (Cody-Waite
/// two-part `ln 2`), `e^r` is the
/// degree-20 Taylor sum in interval arithmetic plus the remainder
/// `|r|^21 / 21! e^|r|` (`e^|r| <= 1.5` for `|r| <= 0.35`), scaled by `2^k`.
/// An overflowing result is an error; for `x < -707` the result is
/// enclosed by `[0, 2^-1019]` (`e^-707 < 2^-1019`; the earlier floor
/// `2^-1020` was below `e^x` on `(-707.0101, -707)`, research node
/// `research/safe_enclosure_exp_floor_20261004`).
pub fn exp_interval(x: f64) -> CoreResult<Interval> {
    if x.is_nan() {
        return Err(CoreError::NonFinite("directed rounding: exp of NaN".into()));
    }
    if x > 709.0 {
        return Err(CoreError::NonFinite(
            "directed rounding: exp overflows".into(),
        ));
    }
    if x < -707.0 {
        return Interval::new(0.0, f64::from_bits((1023_u64 - 1019) << 52));
    }
    // Cody-Waite: ln 2 = LN2_HI + LN2_LO + d with 0 < d < ulp(LN2_LO)
    // (checked at 60 digits: d = 1.16e-26, ulp = 2.58e-26). LN2_HI has 21
    // trailing zero bits, so k LN2_HI is exact for |k| <= 1024.
    let ln2_hi = f64::from_bits(0x3FE6_2E42_FEE0_0000);
    let ln2_lo = f64::from_bits(0x3DEA_39EF_3579_3C76);
    let k = (x / (ln2_hi + ln2_lo)).round();
    let r = Interval::point(x)?
        .sub(Interval::point(k * ln2_hi)?)?
        .sub(Interval::point(k)?.mul(Interval::new(ln2_lo, ln2_lo.next_up())?)?)?;
    let mut sum = Interval::point(1.0)?;
    let mut term = Interval::point(1.0)?;
    for j in 1..=20 {
        term = term.mul(r)?.div(Interval::point(j as f64)?)?;
        sum = sum.add(term)?;
    }
    let mut remainder = 1.5;
    for j in 1..=21 {
        remainder = mul_up(remainder, div_up(r.mag(), j as f64)?)?;
    }
    let sum = sum.add(Interval::new(-remainder, remainder)?)?;
    // 2^k exactly: k is in [-1021, 1023] here.
    let scale = f64::from_bits(((k as i64 + 1023) as u64) << 52);
    Interval::new(mul_down(sum.lo, scale)?, mul_up(sum.hi, scale)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directed_results_bracket_the_exact_value() {
        // 0.1 + 0.2 is 0.30000000000000004440892098500626..., rounded up.
        let (a, b) = (0.1, 0.2);
        assert!(add_down(a, b).unwrap() < add_up(a, b).unwrap());
        assert_eq!(add_up(1.0, 2.0).unwrap(), 3.0);
        assert_eq!(add_down(1.0, 2.0).unwrap(), 3.0);
        // 1/3 is not representable.
        assert!(div_down(1.0, 3.0).unwrap() < div_up(1.0, 3.0).unwrap());
        assert_eq!(div_up(1.0, 4.0).unwrap(), 0.25);
        assert!(mul_down(0.1, 0.1).unwrap() < mul_up(0.1, 0.1).unwrap());
        assert_eq!(sqrt_up(4.0).unwrap(), 2.0);
        assert!(sqrt_down(2.0).unwrap() < sqrt_up(2.0).unwrap());
        // Subnormal products move outward unconditionally.
        let tiny = f64::from_bits(1);
        assert!(mul_up(tiny, 0.5).unwrap() > 0.0);
        assert_eq!(mul_down(tiny, 0.5).unwrap(), 0.0_f64.next_down().max(-tiny));
        // Overflow fails closed.
        assert!(mul_up(1.0e308, 1.0e308).is_err());
        assert!(add_up(f64::MAX, f64::MAX).is_err());
        assert!(
            Interval::point(1.0)
                .unwrap()
                .div(Interval::new(-1.0, 1.0).unwrap())
                .is_err()
        );
    }
}
