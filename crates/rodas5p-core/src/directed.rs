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

/// A power of two `sigma = 2^exponent`, held by its exponent only (research
/// node PY04, `research/py04_fused_normalization_20261011`). `2^exponent` is
/// never formed as a binary64 here; scaling by it goes through the staged
/// [`scale_pow2_staged`], so `|exponent| > 1022` (where `2^exponent` or its
/// reciprocal is not a normal binary64) never overflows or underflows a
/// factor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PowerOfTwo {
    pub exponent: i64,
}

impl PowerOfTwo {
    pub const ONE: Self = Self { exponent: 0 };

    pub fn new(exponent: i64) -> Self {
        Self { exponent }
    }

    /// `sigma^-1`: the exponent's negation (saturating). No reciprocal is
    /// computed, so it never overflows.
    pub fn inverse(self) -> Self {
        Self {
            exponent: self.exponent.saturating_neg(),
        }
    }

    /// The largest power of two not above `|x|` (`floor(log2 |x|)` as the
    /// exponent, subnormal `x` included); `None` for zero or non-finite `x`.
    pub fn floor_of(x: f64) -> Option<Self> {
        if x == 0.0 || !x.is_finite() {
            return None;
        }
        Some(Self {
            exponent: floor_log2(x),
        })
    }
}

/// The largest stage factor of [`scale_pow2_staged`] is `2^1000` in
/// magnitude (`2^+-1000` are normal binary64 numbers).
pub const POW2_STAGE_LIMIT: i64 = 1000;

/// Exponents beyond this magnitude give the same result as the clamped one
/// for every finite `x` (`2^-1074 2^4096` overflows, `f64::MAX 2^-4096`
/// rounds to zero), so they are clamped before staging.
const POW2_EXPONENT_CLAMP: i64 = 4096;

/// One staged scaling `x 2^e` of a finite `x` (see [`scale_pow2_staged`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pow2Scaling {
    /// `x 2^e` rounded to nearest (ties to even); `+-inf` on overflow.
    pub nearest: f64,
    /// The exact `x 2^e` compared with `nearest` (`Equal` iff exact).
    pub exact_vs_nearest: std::cmp::Ordering,
    /// `|x 2^e| >= 2^1024`: no binary64 encloses it from both sides.
    pub overflow: bool,
    /// Multiplications by a stage factor.
    pub stages: u32,
    /// The largest `|k|` of the stage factors `2^k` used (at most
    /// [`POW2_STAGE_LIMIT`]).
    pub largest_stage: u32,
}

impl Pow2Scaling {
    /// Upper bound on the exact `x 2^e`: `nearest`, one step up when the
    /// exact value lies above it. An overflow is an error.
    pub fn up(&self) -> CoreResult<f64> {
        if self.overflow {
            return Err(CoreError::NonFinite(
                "directed rounding: power-of-two scaling overflows".into(),
            ));
        }
        Ok(if self.exact_vs_nearest == std::cmp::Ordering::Greater {
            self.nearest.next_up()
        } else {
            self.nearest
        })
    }

    /// Lower bound on the exact `x 2^e` (see [`Pow2Scaling::up`]).
    pub fn down(&self) -> CoreResult<f64> {
        if self.overflow {
            return Err(CoreError::NonFinite(
                "directed rounding: power-of-two scaling overflows".into(),
            ));
        }
        Ok(if self.exact_vs_nearest == std::cmp::Ordering::Less {
            self.nearest.next_down()
        } else {
            self.nearest
        })
    }

    /// No overflow and no rounding.
    pub fn is_exact(&self) -> bool {
        !self.overflow && self.exact_vs_nearest == std::cmp::Ordering::Equal
    }
}

/// `floor(log2 |x|)` for finite nonzero `x`.
fn floor_log2(x: f64) -> i64 {
    let bits = x.abs().to_bits();
    let biased = (bits >> 52) as i64;
    if biased == 0 {
        let mantissa = bits & ((1_u64 << 52) - 1);
        -1074 + (63 - i64::from(mantissa.leading_zeros()))
    } else {
        biased - 1023
    }
}

/// `2^k` for `|k| <= POW2_STAGE_LIMIT` (a normal binary64, exact).
fn pow2_stage(k: i64) -> f64 {
    debug_assert!(k.abs() <= POW2_STAGE_LIMIT);
    f64::from_bits(((k + 1023) as u64) << 52)
}

/// `x 2^e` by staged multiplications with factors `2^k`, `|k| <= 1000`.
///
/// - `e > 0`: every stage is exact until the value overflows (a power-of-two
///   product of a finite binary64 is exact unless it reaches `2^1024`).
/// - `e < 0`: the value is first scaled exactly while it stays at or above
///   `2^-1022` (normal); the one remaining stage, into the subnormal range,
///   rounds once (IEEE multiplication rounds correctly, subnormal results
///   included). Staging stops when that stage gives zero: every further
///   factor would leave zero unchanged and keep the exact value's sign.
///
/// The rounding direction is read back exactly: the rounded value times
/// `2^-k` (a subnormal or zero scaled up by at most `2^1000`) is exact and is
/// compared with the value before the stage. A non-finite `x` is an error.
pub fn scale_pow2_staged(x: f64, e: i64) -> CoreResult<Pow2Scaling> {
    use std::cmp::Ordering;
    let x = finite(x, "scaled value")?;
    let mut out = Pow2Scaling {
        nearest: x,
        exact_vs_nearest: Ordering::Equal,
        overflow: false,
        stages: 0,
        largest_stage: 0,
    };
    if x == 0.0 || e == 0 {
        return Ok(out);
    }
    let e = e.clamp(-POW2_EXPONENT_CLAMP, POW2_EXPONENT_CLAMP);
    let mut stage = |y: f64, k: i64| {
        out.stages += 1;
        out.largest_stage = out.largest_stage.max(k.unsigned_abs() as u32);
        y * pow2_stage(k)
    };
    let mut y = x;
    if e > 0 {
        let mut rest = e;
        while rest > 0 {
            let k = rest.min(POW2_STAGE_LIMIT);
            y = stage(y, k);
            rest -= k;
            if !y.is_finite() {
                break;
            }
        }
        out.nearest = y;
        if !y.is_finite() {
            out.overflow = true;
            // The exact value is finite: below +inf, above -inf.
            out.exact_vs_nearest = if x > 0.0 {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        return Ok(out);
    }
    let top = floor_log2(x);
    // The exact part: down to the exponent -1022 at most (all of e when the
    // result is normal).
    let exact_part = if top + e >= -1022 {
        e
    } else {
        -(top + 1022).max(0)
    };
    let mut rest = exact_part;
    while rest < 0 {
        let k = rest.max(-POW2_STAGE_LIMIT);
        y = stage(y, k);
        rest -= k;
    }
    let mut rest = e - exact_part;
    while rest < 0 {
        let k = rest.max(-POW2_STAGE_LIMIT);
        let before = y;
        y = stage(y, k);
        rest -= k;
        // Exact read-back: |y| <= 2^-1022, so |y 2^-k| <= 2^-22.
        let back = y * pow2_stage(-k);
        out.exact_vs_nearest = before
            .partial_cmp(&back)
            .expect("finite values are ordered");
        if y == 0.0 {
            break;
        }
        // A nonzero result here is the final one: before >= 2^-1022 > |y|
        // means this stage reached the subnormal range, which only the last
        // stage does (rest is then 0).
        debug_assert_eq!(rest, 0);
    }
    out.nearest = y;
    Ok(out)
}

/// `x 2^e` rounded to nearest by staged scaling ([`scale_pow2_staged`]);
/// `+-inf` when it overflows; a non-finite `x` is returned unchanged.
pub fn scale_pow2_nearest(x: f64, e: i64) -> f64 {
    scale_pow2_staged(x, e).map_or(x, |s| s.nearest)
}

/// Upper bound on `x 2^e`: the nearest value, one step up when the exact
/// value lies above it. An overflow or a non-finite `x` is an error.
pub fn scale_pow2_up(x: f64, e: i64) -> CoreResult<f64> {
    scale_pow2_staged(x, e)?.up()
}

/// Lower bound on `x 2^e` (see [`scale_pow2_up`]).
pub fn scale_pow2_down(x: f64, e: i64) -> CoreResult<f64> {
    scale_pow2_staged(x, e)?.down()
}

/// Whether `x 2^e` is a binary64 number (finite `x`, no overflow, no
/// rounding).
pub fn scale_pow2_is_exact(x: f64, e: i64) -> bool {
    scale_pow2_staged(x, e).is_ok_and(|s| s.is_exact())
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
