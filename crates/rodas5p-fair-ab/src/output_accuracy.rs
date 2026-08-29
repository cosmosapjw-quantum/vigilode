//! Research diagnostic: observable accuracy and output-policy sensitivity are different.
//! The interval is conditional on u being a valid reference-error bound in the
//! same fixed WRMS norm. An empirical u is not automatically a rigorous bound.
//! No historical gate, freeze, holdout, or production solver route is changed.
use crate::{DualOutputPolicyEvidence, FairError, FairResult, OutputPolicyDominance};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccuracyBudgetVerdict {
    WithinBudget,
    OutsideBudget,
    ReferenceUnresolved,
    BudgetNotSpecified,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorBudgetAssessment {
    pub lower_error_wrms: f64,
    pub upper_error_wrms: f64,
    pub budget_wrms: Option<f64>,
    pub verdict: AccuracyBudgetVerdict,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputAccuracyAssessment {
    pub clipped: ErrorBudgetAssessment,
    pub dense: ErrorBudgetAssessment,
    pub policy_sensitivity: OutputPolicyDominance,
    pub trajectory_discrepancy_wrms: f64,
}
/// Reverse/ordinary triangle bounds, not a claim about provenance or reference validity.
/// A missing budget never passes; no budget is inferred from observed errors.
pub fn assess_error_budget(
    error: f64,
    uncertainty: f64,
    budget: Option<f64>,
) -> FairResult<ErrorBudgetAssessment> {
    if ![error, uncertainty]
        .iter()
        .all(|v| v.is_finite() && *v >= 0.0)
        || budget.is_some_and(|v| !v.is_finite() || v < 0.0)
    {
        return Err(FairError::Invalid(
            "finite nonnegative error, reference uncertainty and budget required".into(),
        ));
    }
    let lower_error_wrms = (error - uncertainty).max(0.0);
    let upper_error_wrms = error + uncertainty;
    if !upper_error_wrms.is_finite() {
        return Err(FairError::Invalid(
            "unrepresentable upper error bound".into(),
        ));
    }
    let verdict = match budget {
        None => AccuracyBudgetVerdict::BudgetNotSpecified,
        Some(b) if upper_error_wrms <= b => AccuracyBudgetVerdict::WithinBudget,
        Some(b) if lower_error_wrms > b => AccuracyBudgetVerdict::OutsideBudget,
        Some(_) => AccuracyBudgetVerdict::ReferenceUnresolved,
    };
    Ok(ErrorBudgetAssessment {
        lower_error_wrms,
        upper_error_wrms,
        budget_wrms: budget,
        verdict,
    })
}
/// Additive read-only assessment; the legacy direct-trajectory gate is retained.
pub fn assess_output_accuracy(
    evidence: &DualOutputPolicyEvidence,
    budget: Option<f64>,
) -> FairResult<OutputAccuracyAssessment> {
    let policy_sensitivity = evidence.classify()?;
    let u = evidence
        .reference_wrms_basis
        .error_scale
        .reference_uncertainty_wrms;
    Ok(OutputAccuracyAssessment {
        clipped: assess_error_budget(evidence.clipped.errors.max_grid_wrms, u, budget)?,
        dense: assess_error_budget(evidence.dense.errors.max_grid_wrms, u, budget)?,
        policy_sensitivity,
        trajectory_discrepancy_wrms: evidence.output_policy_discrepancy_wrms,
    })
}
