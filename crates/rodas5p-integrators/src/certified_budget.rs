//! Certified output-budget bounds (re-audit R3 of 2026-10-01, ARITH-04).
//!
//! [`OutputBudgetPolicy::budget`] evaluates the budget range-safely but in
//! round-to-nearest; it is a diagnostic. The step-power term rounded above
//! the exact value in four of the R3 boundary cases (for example
//! `epsilon_ref = 5e-324` with `h` one ULP below `h_ref`, whose exact value
//! 4.94065645841246489e-324 was returned as 5e-324), so a step with
//! `output_wrms = 5e-324` was accepted against a budget it exceeds.
//! [`OutputBudgetPolicy::certified_budget`] returns an enclosure
//! `[lower, upper]` of the exact budget with every operation rounded
//! outward, and [`OutputBudgetPolicy::certified_decide`] accepts only when an
//! upper bound on the output error is at most `lower`.
//!
//! A certified budget does not certify the error estimate it is compared
//! with; that is the caller's upper bound.

use rodas5p_core::{
    CoreError, CoreResult, binary_scale, binary_split,
    directed::{div_down, div_up, mul_down, mul_up},
};
use serde::{Deserialize, Serialize};

use crate::OutputBudgetPolicy;

/// How the enclosure was obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CertifiedBudgetStatus {
    /// `lower == upper` is the exact budget.
    Exact,
    /// A nondegenerate enclosure inside the binary64 range.
    Enclosed,
    /// The exact budget is 0 (`epsilon_ref = 0` or a zero component).
    ExactZero,
    /// The exact budget is below the smallest subnormal or rounds into it:
    /// the lower bound is 0, so only an exact zero output error is accepted.
    UnderflowLowerZero,
    /// The exact budget exceeds `f64::MAX`: the component does not bind.
    OverflowNonBinding,
}

/// An enclosure of the exact budget.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CertifiedBudget {
    pub lower: f64,
    pub upper: f64,
    pub status: CertifiedBudgetStatus,
}

impl CertifiedBudget {
    fn classify(lower: f64, upper: f64) -> Self {
        let status = if upper == f64::INFINITY {
            CertifiedBudgetStatus::OverflowNonBinding
        } else if upper == 0.0 {
            CertifiedBudgetStatus::ExactZero
        } else if lower == 0.0 {
            CertifiedBudgetStatus::UnderflowLowerZero
        } else if lower == upper {
            CertifiedBudgetStatus::Exact
        } else {
            CertifiedBudgetStatus::Enclosed
        };
        Self {
            lower,
            upper,
            status,
        }
    }

    fn point(value: f64) -> Self {
        Self::classify(value, value)
    }

    fn min(self, other: Self) -> Self {
        Self::classify(self.lower.min(other.lower), self.upper.min(other.upper))
    }
}

/// A mantissa interval `[lo, hi]` times `2^exponent`, normalized by the
/// exponent of `hi` (both ends share it, so the interval never straddles a
/// renormalization).
#[derive(Clone, Copy, Debug)]
struct ScaledInterval {
    lo: f64,
    hi: f64,
    exponent: i64,
}

impl ScaledInterval {
    fn normalized(lo: f64, hi: f64, exponent: i64) -> Self {
        let (_, shift) = binary_split(hi);
        // Scaling by 2^-shift is exact: hi and lo are normal and within a
        // few binades of 1.
        let factor = 2.0_f64.powi(-(shift as i32));
        Self {
            lo: lo * factor,
            hi: hi * factor,
            exponent: exponent + shift,
        }
    }

    fn mul(self, other: Self) -> CoreResult<Self> {
        Ok(Self::normalized(
            mul_down(self.lo, other.lo)?,
            mul_up(self.hi, other.hi)?,
            self.exponent + other.exponent,
        ))
    }
}

/// `x * 2^e` rounded down and up: one rounding of `binary_scale`, checked
/// exact by the round trip.
fn scale_enclosure(lo: f64, hi: f64, exponent: i64) -> (f64, f64) {
    let lower = binary_scale(lo, exponent);
    let lower = if lower == f64::INFINITY {
        f64::MAX
    } else if lower.is_finite() && binary_scale(lower, -exponent) == lo {
        lower
    } else {
        lower.next_down().max(0.0)
    };
    let upper = binary_scale(hi, exponent);
    let upper = if !upper.is_finite() {
        f64::INFINITY
    } else if binary_scale(upper, -exponent) == hi {
        upper
    } else {
        upper.next_up()
    };
    (lower, upper)
}

/// Enclosure of `epsilon_ref * (h / h_ref)^p` for `epsilon_ref >= 0`,
/// `h, h_ref > 0`.
pub fn step_power_enclosure(
    epsilon_ref: f64,
    h: f64,
    h_ref: f64,
    exponent: u32,
) -> CoreResult<CertifiedBudget> {
    if !(epsilon_ref >= 0.0 && epsilon_ref.is_finite() && h > 0.0 && h_ref > 0.0)
        || !(h.is_finite() && h_ref.is_finite())
    {
        return Err(CoreError::InvalidInput(
            "certified step-power budget needs finite nonnegative epsilon and positive steps"
                .into(),
        ));
    }
    if epsilon_ref == 0.0 {
        return Ok(CertifiedBudget::point(0.0));
    }
    let (m_eps, e_eps) = binary_split(epsilon_ref);
    let (m_h, e_h) = binary_split(h);
    let (m_ref, e_ref) = binary_split(h_ref);
    let ratio = ScaledInterval::normalized(div_down(m_h, m_ref)?, div_up(m_h, m_ref)?, 0);
    let mut power = ScaledInterval {
        lo: 1.0,
        hi: 1.0,
        exponent: 0,
    };
    let mut square = ratio;
    let mut remaining = exponent;
    while remaining > 0 {
        if remaining & 1 == 1 {
            power = power.mul(square)?;
        }
        remaining >>= 1;
        if remaining > 0 {
            square = square.mul(square)?;
        }
    }
    let product = ScaledInterval::normalized(
        mul_down(m_eps, power.lo)?,
        mul_up(m_eps, power.hi)?,
        power.exponent,
    );
    let total_exponent = e_eps + i64::from(exponent) * (e_h - e_ref) + product.exponent;
    let (lower, upper) = scale_enclosure(product.lo, product.hi, total_exponent);
    Ok(CertifiedBudget::classify(lower, upper))
}

/// Enclosure of `eta * embedded`.
fn product_enclosure(eta: f64, embedded: f64) -> CertifiedBudget {
    let lower = mul_down(eta, embedded).map_or(f64::MAX, |value| value.max(0.0));
    let upper = mul_up(eta, embedded).unwrap_or(f64::INFINITY);
    CertifiedBudget::classify(lower, upper)
}

/// A certified acceptance decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CertifiedBudgetDecision {
    pub budget: CertifiedBudget,
    /// The caller's upper bound on the output error.
    pub output_error_upper: f64,
    pub accepted: bool,
}

impl OutputBudgetPolicy {
    /// Enclosure of the exact budget for `embedded_error` taken as exact.
    pub fn certified_budget(&self, embedded_error: f64, h: f64) -> CoreResult<CertifiedBudget> {
        self.validate()?;
        if !(embedded_error >= 0.0 && embedded_error.is_finite() && h > 0.0 && h.is_finite()) {
            return Err(CoreError::InvalidInput(
                "certified budget needs a finite nonnegative embedded error and positive step"
                    .into(),
            ));
        }
        Ok(match self {
            Self::Absolute { epsilon } => CertifiedBudget::point(*epsilon),
            Self::EmbeddedRelative { eta } => product_enclosure(*eta, embedded_error),
            Self::StepPower {
                epsilon_ref,
                h_ref,
                exponent,
            } => step_power_enclosure(*epsilon_ref, h, *h_ref, *exponent)?,
            Self::Mixed {
                eta,
                epsilon_ref,
                h_ref,
                exponent,
            } => product_enclosure(*eta, embedded_error).min(step_power_enclosure(
                *epsilon_ref,
                h,
                *h_ref,
                *exponent,
            )?),
        })
    }

    /// Accept iff `output_error_upper <= certified lower budget`. With a
    /// budget that grows with the embedded estimate, pass a *lower* bound on
    /// that estimate.
    pub fn certified_decide(
        &self,
        output_error_upper: f64,
        embedded_error_lower: f64,
        h: f64,
    ) -> CoreResult<CertifiedBudgetDecision> {
        if !(output_error_upper >= 0.0) {
            return Err(CoreError::InvalidInput(
                "certified budget decision needs a nonnegative output error bound".into(),
            ));
        }
        let budget = self.certified_budget(embedded_error_lower, h)?;
        Ok(CertifiedBudgetDecision {
            budget,
            output_error_upper,
            accepted: output_error_upper <= budget.lower,
        })
    }
}
