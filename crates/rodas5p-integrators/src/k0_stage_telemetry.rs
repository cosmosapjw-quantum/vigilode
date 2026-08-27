use serde::{Deserialize, Serialize};
use std::f64::consts::FRAC_PI_2;
use std::fmt;

use rodas5p_core::safe_l2;

pub const K0_STAGE_RECEIPT_SCHEMA: &str = "vigilode-k0-stage-receipt/v1";
pub const K0_CLAIM_CLASS: &str = "EXPLORATORY_NONAUTHORITATIVE";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum K0ExecutionState {
    Complete,
    StopInvalid,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum K0KernelArm {
    #[serde(rename = "legacy-restarted-gmres")]
    LegacyRestartedGmres,
    #[serde(rename = "incremental-givens-candidate")]
    IncrementalGivensCandidate,
}

impl K0KernelArm {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LegacyRestartedGmres => "legacy-restarted-gmres",
            Self::IncrementalGivensCandidate => "incremental-givens-candidate",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum K0InitialGuessSource {
    Zero,
    ExternalPrevious,
    InternalLgmresPrevious,
    InternalGcrodrPrevious,
    RecycleProjection,
    Direct,
    NotApplicable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct K0StageIdentity {
    pub case_id: String,
    pub family: String,
    pub kernel_arm: K0KernelArm,
    pub attempt_id: u64,
    pub stage_index: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct K0StageWork {
    pub linear_matvecs: u64,
    pub diagnostic_matvecs: u64,
    pub operator_applies_total: u64,
    pub telemetry_jvp_overhead: u64,
}

impl K0StageWork {
    pub fn new(
        linear_matvecs: u64,
        diagnostic_matvecs: u64,
        telemetry_jvp_overhead: u64,
    ) -> Result<Self, K0ValidationError> {
        let operator_applies_total = linear_matvecs
            .checked_add(diagnostic_matvecs)
            .ok_or(K0ValidationError::CounterOverflow)?;
        Ok(Self {
            linear_matvecs,
            diagnostic_matvecs,
            operator_applies_total,
            telemetry_jvp_overhead,
        })
    }

    pub fn validate(&self) -> Result<(), K0ValidationError> {
        let expected = self
            .linear_matvecs
            .checked_add(self.diagnostic_matvecs)
            .ok_or(K0ValidationError::CounterOverflow)?;
        if self.operator_applies_total != expected {
            return Err(K0ValidationError::OperatorApplyTotalMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct K0StageGeometry {
    pub scaled_nonlinear_remainder: Option<f64>,
    #[serde(rename = "rhs_to_retained_angle_rad")]
    pub rhs_jacobian_angle_radians: Option<f64>,
    #[serde(rename = "rhs_singular_values")]
    pub singular_values: Vec<f64>,
}

impl K0StageGeometry {
    fn validate(&self) -> Result<(), K0ValidationError> {
        validate_nonnegative_finite(
            self.scaled_nonlinear_remainder,
            "scaled_nonlinear_remainder",
        )?;
        if let Some(angle) = self.rhs_jacobian_angle_radians
            && (!angle.is_finite() || !(0.0..=FRAC_PI_2).contains(&angle))
        {
            return Err(K0ValidationError::InvalidNumericValue(
                "rhs_jacobian_angle_radians",
            ));
        }
        if self
            .singular_values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(K0ValidationError::InvalidNumericValue("singular_values"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct K0StageError {
    pub kind: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct K0StageReceipt {
    pub schema: String,
    #[serde(flatten)]
    pub identity: K0StageIdentity,
    pub claim_class: String,
    pub execution_state: K0ExecutionState,
    pub solver_method: Option<String>,
    pub initial_guess_source: Option<K0InitialGuessSource>,
    #[serde(rename = "initial_true_residual")]
    pub initial_true_residual_norm: Option<f64>,
    #[serde(rename = "final_true_residual")]
    pub final_true_residual_norm: Option<f64>,
    pub work: Option<K0StageWork>,
    pub geometry: Option<K0StageGeometry>,
    pub error: Option<K0StageError>,
}

impl K0StageReceipt {
    pub fn identity(&self) -> &K0StageIdentity {
        &self.identity
    }

    pub const fn residual_sign_coverage() -> K0ResidualSignCoverage {
        K0ResidualSignCoverage::NotCoveredByNormTelemetry
    }

    pub fn validate(&self) -> Result<(), K0ValidationError> {
        if self.schema != K0_STAGE_RECEIPT_SCHEMA {
            return Err(K0ValidationError::SchemaMismatch);
        }
        if self.claim_class != K0_CLAIM_CLASS {
            return Err(K0ValidationError::ClaimBoundaryMismatch);
        }
        if self.identity.case_id.is_empty()
            || self.identity.family.is_empty()
            || self.identity.stage_index > 7
        {
            return Err(K0ValidationError::InvalidIdentity);
        }
        validate_nonnegative_finite(
            self.initial_true_residual_norm,
            "initial_true_residual_norm",
        )?;
        validate_nonnegative_finite(self.final_true_residual_norm, "final_true_residual_norm")?;
        if let Some(work) = &self.work {
            work.validate()?;
        }
        if let Some(geometry) = &self.geometry {
            geometry.validate()?;
        }
        if self.execution_state == K0ExecutionState::Complete
            && (self.work.is_none()
                || self.geometry.is_none()
                || self
                    .geometry
                    .as_ref()
                    .is_none_or(|geometry| geometry.scaled_nonlinear_remainder.is_none()))
        {
            return Err(K0ValidationError::CompleteStateMissingEvidence);
        }
        if self.execution_state == K0ExecutionState::Error {
            if self.work.is_none() {
                return Err(K0ValidationError::ErrorStateMissingWork);
            }
            match &self.error {
                Some(error) if !error.kind.is_empty() && !error.message.is_empty() => {}
                _ => return Err(K0ValidationError::ErrorStateMissingError),
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K0ResidualSignCoverage {
    NotCoveredByNormTelemetry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K0ValidationError {
    SchemaMismatch,
    ClaimBoundaryMismatch,
    InvalidIdentity,
    CounterOverflow,
    OperatorApplyTotalMismatch,
    InvalidNumericValue(&'static str),
    ErrorStateMissingWork,
    ErrorStateMissingError,
    CompleteStateMissingEvidence,
    ShapeMismatch,
    EmptyStageSequence,
    StageIdentityMismatch,
    NonExhaustiveStageSequence,
    ReceiptAfterTerminal,
}

impl fmt::Display for K0ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for K0ValidationError {}

fn validate_nonnegative_finite(
    value: Option<f64>,
    field: &'static str,
) -> Result<(), K0ValidationError> {
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return Err(K0ValidationError::InvalidNumericValue(field));
    }
    Ok(())
}

/// Evaluate the exact INV-K0-013 Euclidean nonlinear-remainder ratio.
pub fn scaled_nonlinear_remainder(
    rhs_value: &[f64],
    frozen_rhs_value: &[f64],
    frozen_jacobian_stage_increment: &[f64],
    time_derivative_increment: &[f64],
) -> Result<f64, K0ValidationError> {
    let dimension = rhs_value.len();
    if dimension == 0
        || frozen_rhs_value.len() != dimension
        || frozen_jacobian_stage_increment.len() != dimension
        || time_derivative_increment.len() != dimension
    {
        return Err(K0ValidationError::ShapeMismatch);
    }
    if rhs_value
        .iter()
        .chain(frozen_rhs_value)
        .chain(frozen_jacobian_stage_increment)
        .chain(time_derivative_increment)
        .any(|value| !value.is_finite())
    {
        return Err(K0ValidationError::InvalidNumericValue(
            "scaled_nonlinear_remainder_input",
        ));
    }
    let remainder = (0..dimension)
        .map(|index| {
            rhs_value[index]
                - frozen_rhs_value[index]
                - frozen_jacobian_stage_increment[index]
                - time_derivative_increment[index]
        })
        .collect::<Vec<_>>();
    let numerator = safe_l2(&remainder);
    let denominator = safe_l2(rhs_value)
        .max(safe_l2(frozen_rhs_value))
        .max(safe_l2(frozen_jacobian_stage_increment))
        .max(safe_l2(time_derivative_increment))
        .max(f64::MIN_POSITIVE);
    let ratio = numerator / denominator;
    if ratio.is_finite() && ratio >= 0.0 {
        Ok(ratio)
    } else {
        Err(K0ValidationError::InvalidNumericValue(
            "scaled_nonlinear_remainder",
        ))
    }
}

/// Validate deterministic, failure-preserving stage order across captured attempts.
pub fn validate_stage_receipt_sequence(
    receipts: &[K0StageReceipt],
) -> Result<(), K0ValidationError> {
    let first = receipts
        .first()
        .ok_or(K0ValidationError::EmptyStageSequence)?;
    let expected_case = &first.identity.case_id;
    let expected_family = &first.identity.family;
    let expected_arm = first.identity.kernel_arm;
    let mut attempt_id = 0_u64;
    let mut expected_stage = 0_u8;
    let mut terminal_seen = false;

    for receipt in receipts {
        receipt.validate()?;
        if &receipt.identity.case_id != expected_case
            || &receipt.identity.family != expected_family
            || receipt.identity.kernel_arm != expected_arm
        {
            return Err(K0ValidationError::StageIdentityMismatch);
        }
        if receipt.identity.attempt_id != attempt_id {
            if terminal_seen || expected_stage != 8 || receipt.identity.attempt_id != attempt_id + 1
            {
                return Err(K0ValidationError::NonExhaustiveStageSequence);
            }
            attempt_id += 1;
            expected_stage = 0;
        }
        if terminal_seen {
            return Err(K0ValidationError::ReceiptAfterTerminal);
        }
        if receipt.identity.stage_index != expected_stage {
            return Err(K0ValidationError::NonExhaustiveStageSequence);
        }
        expected_stage += 1;
        if receipt.execution_state != K0ExecutionState::Complete {
            terminal_seen = true;
        }
    }
    if !terminal_seen && expected_stage != 8 {
        return Err(K0ValidationError::NonExhaustiveStageSequence);
    }
    Ok(())
}
