//! Two-arm output admissibility under a new protocol id (audit F-007, F-029).
//!
//! The frozen v2 rule `gap <= 0.1 * dense error` in
//! [`crate::classify_output_policy_dominance`] compares two independently
//! stepped trajectories whose difference is itself O(tol), so it cannot be met
//! by construction; SciPy Radau violates it in 17/18 audit pairs.  That rule is
//! kept unchanged so the committed v2 records still validate.  This module
//! adds the replacement, evaluated in case-tolerance weights:
//!
//! * (A) each arm's max-grid error `E` against the reference, with reference
//!   uncertainty `U` in the same weights, is inside the budget `B` iff
//!   `E + U <= B`, outside iff `E - U > B`, and undecidable otherwise;
//! * (B) the clipped/dense gap satisfies `G <= E_c + E_d` identically through
//!   the reference, so it is an integrity assertion, not a gate;
//! * (C) output-policy sensitivity is judged by a same-step interpolant check
//!   supplied by the caller.  A row without it cannot pass.

use serde::{Deserialize, Serialize};

use crate::global_error::{DualOutputPolicyEvidence, ExternalErrorScale, ReferenceWrmsBasis};
use crate::{FairError, FairResult};

/// Budget `B` and interpolant limit are part of the protocol id and are fixed
/// before any row is run under it.
pub const TWO_ARM_ADMISSIBILITY_PROTOCOL_ID: &str = "two-arm-output-admissibility-v3;basis=case-tolerance(atol,rtol;anchor=reference);budget=10;uncertainty-bands=E+U<=B,E-U>B;gap=triangle-integrity;interpolant-delta<=2;order=reference,budget,interpolant";
pub const TWO_ARM_GLOBAL_ERROR_BUDGET: f64 = 10.0;
pub const TWO_ARM_INTERPOLANT_DELTA_LIMIT: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArmBudget {
    WithinBudget,
    ExceedsBudget,
    ReferenceUndecidable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputArmExceedance {
    pub clipped: bool,
    pub dense: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TwoArmRowStatus {
    ReferenceDominated,
    GlobalErrorExceedsBudget(OutputArmExceedance),
    InterpolantDominated,
    /// Both arms are inside the budget but the same-step interpolant check
    /// was not supplied; such a row is not a pass.
    InterpolantUnchecked,
    Pass,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TwoArmAdmissibility {
    pub protocol_id: String,
    pub basis_id: String,
    pub clipped_case_wrms: f64,
    pub dense_case_wrms: f64,
    pub reference_uncertainty_case_wrms: f64,
    pub gap_case_wrms: f64,
    pub clipped: ArmBudget,
    pub dense: ArmBudget,
    pub interpolant_delta: Option<f64>,
    pub status: TwoArmRowStatus,
}

pub fn classify_arm_budget(error: f64, uncertainty: f64, budget: f64) -> FairResult<ArmBudget> {
    if !(error.is_finite()
        && uncertainty.is_finite()
        && budget.is_finite()
        && error >= 0.0
        && uncertainty >= 0.0
        && budget > 0.0)
    {
        return Err(FairError::Invalid(
            "arm budget inputs must be finite and nonnegative with a positive budget".into(),
        ));
    }
    // The bands are defined on the exact values of E + U and E - U; a
    // nearest-rounded 10 + 1e-16 = 10 made a boundary row WithinBudget
    // (external audit VIG-A07).
    Ok(if exact_sum_cmp(error, uncertainty, budget).is_le() {
        ArmBudget::WithinBudget
    } else if exact_sum_cmp(error, -uncertainty, budget).is_gt() {
        ArmBudget::ExceedsBudget
    } else {
        ArmBudget::ReferenceUndecidable
    })
}

/// Below this, an FMA residual of a product or quotient may itself round.
const EXACT_RESIDUAL_THRESHOLD: f64 = f64::MIN_POSITIVE * 9_007_199_254_740_992.0; // 2^-969

/// `a / b` rounded upward, for finite positive `a` and `b`; conservative
/// (one step up) when the residual could be inexact.
fn upward_div(a: f64, b: f64) -> f64 {
    let quotient = a / b;
    if a < EXACT_RESIDUAL_THRESHOLD || quotient < EXACT_RESIDUAL_THRESHOLD {
        return quotient.next_up();
    }
    if quotient.mul_add(b, -a) < 0.0 {
        quotient.next_up()
    } else {
        quotient
    }
}

/// `a * b` rounded upward, for finite nonnegative `a` and `b`; conservative
/// (one step up) when the residual could be inexact.
fn upward_mul(a: f64, b: f64) -> f64 {
    let product = a * b;
    if product == 0.0 && (a == 0.0 || b == 0.0) {
        return 0.0;
    }
    if product < EXACT_RESIDUAL_THRESHOLD {
        return product.next_up();
    }
    if a.mul_add(b, -product) > 0.0 {
        product.next_up()
    } else {
        product
    }
}

/// Compare the exact real `a + b` with a finite `bound`, for finite `a`, `b`.
///
/// `s = fl(a + b)` is monotone in the exact sum and `bound` is representable,
/// so `s < bound` and `s > bound` already decide; on `s == bound` the TwoSum
/// error term decides. An overflowing sum exceeds every finite bound.
fn exact_sum_cmp(a: f64, b: f64, bound: f64) -> std::cmp::Ordering {
    let sum = a + b;
    if !sum.is_finite() {
        return if sum > 0.0 {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Less
        };
    }
    if sum < bound {
        return std::cmp::Ordering::Less;
    }
    if sum > bound {
        return std::cmp::Ordering::Greater;
    }
    let b_virtual = sum - a;
    let error = (a - (sum - b_virtual)) + (b - b_virtual);
    if error > 0.0 {
        std::cmp::Ordering::Greater
    } else if error < 0.0 {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Equal
    }
}

/// `y_c - y_d = (y_c - y_ref) - (y_d - y_ref)` gives `G <= E_c + E_d` at every
/// grid point in the same weights, so a violation means grid or weight
/// misalignment.  The slack covers the rounding of three WRMS evaluations of
/// length `dimension`.
pub fn check_policy_gap_triangle(
    gap: f64,
    clipped_error: f64,
    dense_error: f64,
    dimension: usize,
) -> FairResult<()> {
    if !(gap.is_finite()
        && clipped_error.is_finite()
        && dense_error.is_finite()
        && gap >= 0.0
        && clipped_error >= 0.0
        && dense_error >= 0.0)
    {
        return Err(FairError::Invalid(
            "policy gap and arm errors must be finite and nonnegative".into(),
        ));
    }
    let slack = 4.0 * (dimension as f64 + 4.0) * f64::EPSILON;
    if gap > (clipped_error + dense_error) * (1.0 + slack) {
        return Err(FairError::Invalid(
            "policy gap violates the triangle identity through the reference".into(),
        ));
    }
    Ok(())
}

pub fn classify_two_arm_row(
    clipped: ArmBudget,
    dense: ArmBudget,
    interpolant_delta: Option<f64>,
) -> FairResult<TwoArmRowStatus> {
    if let Some(delta) = interpolant_delta
        && !(delta.is_finite() && delta >= 0.0)
    {
        return Err(FairError::Invalid(
            "interpolant delta must be finite and nonnegative".into(),
        ));
    }
    use ArmBudget::*;
    Ok(match (clipped, dense) {
        (ReferenceUndecidable, _) | (_, ReferenceUndecidable) => {
            TwoArmRowStatus::ReferenceDominated
        }
        (ExceedsBudget, _) | (_, ExceedsBudget) => {
            TwoArmRowStatus::GlobalErrorExceedsBudget(OutputArmExceedance {
                clipped: clipped == ExceedsBudget,
                dense: dense == ExceedsBudget,
            })
        }
        (WithinBudget, WithinBudget) => match interpolant_delta {
            None => TwoArmRowStatus::InterpolantUnchecked,
            Some(delta) if delta > TWO_ARM_INTERPOLANT_DELTA_LIMIT => {
                TwoArmRowStatus::InterpolantDominated
            }
            Some(_) => TwoArmRowStatus::Pass,
        },
    })
}

impl ReferenceWrmsBasis {
    /// The same reference trajectory with case-tolerance weights
    /// `atol + rtol |y_ref|`.  The reference uncertainty is converted with
    /// `U_case <= U * max_{i,j} w_ij / w_case_ij`, which is exact when the two
    /// weight tables are proportional.
    pub fn with_case_tolerance(&self, atol: f64, rtol: f64) -> FairResult<Self> {
        if !(atol.is_finite() && rtol.is_finite() && atol > 0.0 && rtol >= 0.0) {
            return Err(FairError::Invalid(
                "case tolerances must be finite with atol > 0 and rtol >= 0".into(),
            ));
        }
        self.validate()?;
        let dimension = self.error_scale.absolute.len();
        let case_scale = ExternalErrorScale::new(vec![atol; dimension], rtol)?;
        // Both the ratio and the product are rounded upward, so the converted
        // uncertainty is never below U * max(w / w_case) of the computed
        // weights (external audit VIG-A07).
        let mut ratio = 0.0_f64;
        for state in &self.reference_states {
            let tight = self.error_scale.weights(state)?;
            let case = case_scale.weights(state)?;
            for (t, c) in tight.iter().zip(&case) {
                ratio = ratio.max(upward_div(*t, *c));
            }
        }
        let uncertainty = upward_mul(self.error_scale.reference_uncertainty_wrms, ratio);
        ReferenceWrmsBasis::new(
            self.output_grid.clone(),
            self.reference_states.clone(),
            ExternalErrorScale::with_reference_uncertainty(
                vec![atol; dimension],
                rtol,
                uncertainty,
            )?,
        )
    }
}

impl DualOutputPolicyEvidence {
    /// Classify this pair under [`TWO_ARM_ADMISSIBILITY_PROTOCOL_ID`].  The
    /// frozen [`DualOutputPolicyEvidence::classify`] is unchanged.
    pub fn classify_two_arm_v3(
        &self,
        case_atol: f64,
        case_rtol: f64,
        interpolant_delta: Option<f64>,
    ) -> FairResult<TwoArmAdmissibility> {
        self.validate()?;
        classify_two_arm_v3_states(
            &self.reference_wrms_basis,
            (&self.clipped.output_times, &self.clipped.states),
            (&self.dense.output_times, &self.dense.states),
            case_atol,
            case_rtol,
            interpolant_delta,
        )
    }
}

/// `DualOutputPolicyEvidence::classify_two_arm_v3` on raw arm trajectories,
/// for runners that hold states but no `IntegratorWorkReport`.
pub fn classify_two_arm_v3_states(
    reference_wrms_basis: &ReferenceWrmsBasis,
    clipped: (&[f64], &[Vec<f64>]),
    dense: (&[f64], &[Vec<f64>]),
    case_atol: f64,
    case_rtol: f64,
    interpolant_delta: Option<f64>,
) -> FairResult<TwoArmAdmissibility> {
    let basis = reference_wrms_basis.with_case_tolerance(case_atol, case_rtol)?;
    let gap = basis.discrepancy_wrms(clipped.0, clipped.1, dense.0, dense.1)?;
    let clipped = basis.metrics(clipped.0, clipped.1)?;
    let dense = basis.metrics(dense.0, dense.1)?;
    check_policy_gap_triangle(
        gap,
        clipped.max_grid_wrms,
        dense.max_grid_wrms,
        basis.error_scale.absolute.len(),
    )?;
    let uncertainty = basis.error_scale.reference_uncertainty_wrms;
    let clipped_budget = classify_arm_budget(
        clipped.max_grid_wrms,
        uncertainty,
        TWO_ARM_GLOBAL_ERROR_BUDGET,
    )?;
    let dense_budget = classify_arm_budget(
        dense.max_grid_wrms,
        uncertainty,
        TWO_ARM_GLOBAL_ERROR_BUDGET,
    )?;
    let status = classify_two_arm_row(clipped_budget, dense_budget, interpolant_delta)?;
    Ok(TwoArmAdmissibility {
        protocol_id: TWO_ARM_ADMISSIBILITY_PROTOCOL_ID.into(),
        basis_id: format!(
            "case-tolerance;atol={case_atol:e};rtol={case_rtol:e};anchor=reference;grid={}",
            basis.output_grid.grid_id
        ),
        clipped_case_wrms: clipped.max_grid_wrms,
        dense_case_wrms: dense.max_grid_wrms,
        reference_uncertainty_case_wrms: uncertainty,
        gap_case_wrms: gap,
        clipped: clipped_budget,
        dense: dense_budget,
        interpolant_delta,
        status,
    })
}
