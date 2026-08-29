//! Policy-resolved measurements; the historical relative-gap gate is not an accuracy certificate.
//!
//! Supplied errors, uncertainties and budgets must use one norm/scale. Reference
//! uncertainty is externally estimated, not certified by this module. A missing
//! global budget must never be inferred from local solver tolerance or observed
//! errors. Legacy v2 readers remain unchanged for historical replay.
use crate::{
    FairError, FairResult, GlobalErrorMetric, ReferenceDominance, classify_reference_dominance,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputSamplingPolicy {
    Clipped,
    Dense,
}
/// An explicit experimental stratum, not a different exact physical observable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPolicyMetricKey {
    pub problem_id: String,
    pub output_grid_id: String,
    pub scale_id: String,
    pub metric: GlobalErrorMetric,
    pub policy: OutputSamplingPolicy,
}
impl OutputPolicyMetricKey {
    pub fn comparable_with(&self, other: &Self) -> bool {
        self == other
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeasurementResolution {
    Resolved,
    ReferenceLimited,
    MissingEvidence,
    RunFailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccuracyAssessment {
    NotRequested,
    NotAssessed,
    WithinDeclaredBudget,
    ExceedsDeclaredBudget,
    InconclusiveAtBudget,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolicyMeasurementAssessment {
    pub policy: OutputSamplingPolicy,
    pub resolution: MeasurementResolution,
    pub accuracy: AccuracyAssessment,
    pub measured_error: Option<f64>,
    pub reference_uncertainty: Option<f64>,
    pub declared_budget: Option<f64>,
    pub conservative_error: Option<f64>,
}
/// Scalar assessment only: no source authentication, trajectory validation,
/// method-order claim, campaign freeze, or production/ranking admission.
pub fn assess_policy_measurement(
    policy: OutputSamplingPolicy,
    successful: bool,
    error: Option<f64>,
    uncertainty: Option<f64>,
    budget: Option<f64>,
) -> FairResult<PolicyMeasurementAssessment> {
    for (name, v) in [
        ("error", error),
        ("uncertainty", uncertainty),
        ("budget", budget),
    ] {
        if v.is_some_and(|x| !x.is_finite() || x < 0.0) {
            return Err(FairError::Invalid(format!(
                "{name} must be finite and nonnegative"
            )));
        }
    }
    let mut a = PolicyMeasurementAssessment {
        policy,
        resolution: if successful {
            MeasurementResolution::MissingEvidence
        } else {
            MeasurementResolution::RunFailed
        },
        accuracy: AccuracyAssessment::NotAssessed,
        measured_error: error,
        reference_uncertainty: uncertainty,
        declared_budget: budget,
        conservative_error: None,
    };
    if !successful {
        return Ok(a);
    }
    let (Some(e), Some(u)) = (error, uncertainty) else {
        return Ok(a);
    };
    let upper = e + u;
    if !upper.is_finite() {
        return Err(FairError::Invalid("conservative error overflow".into()));
    }
    a.conservative_error = Some(upper);
    a.resolution = match classify_reference_dominance(u, e)? {
        ReferenceDominance::Admissible => MeasurementResolution::Resolved,
        ReferenceDominance::Dominated => MeasurementResolution::ReferenceLimited,
    };
    a.accuracy = match budget {
        None => AccuracyAssessment::NotRequested,
        Some(b) if upper <= b => AccuracyAssessment::WithinDeclaredBudget,
        Some(b) if (e - u).max(0.0) > b => AccuracyAssessment::ExceedsDeclaredBudget,
        Some(_) => AccuracyAssessment::InconclusiveAtBudget,
    };
    Ok(a)
}
