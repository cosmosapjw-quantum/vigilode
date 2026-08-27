use serde::{Deserialize, Serialize};
use std::f64::consts::FRAC_PI_2;
use std::fmt;

pub(crate) const K0_STAGE_RECEIPT_SCHEMA: &str = "vigilode-k0-stage-receipt/v1";
pub(crate) const K0_CLAIM_CLASS: &str = "EXPLORATORY_NONAUTHORITATIVE";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum K0ExecutionState {
    Complete,
    StopInvalid,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum K0KernelArm {
    #[serde(rename = "legacy-restarted-gmres")]
    LegacyRestartedGmres,
    #[serde(rename = "incremental-givens-candidate")]
    IncrementalGivensCandidate,
}

impl K0KernelArm {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::LegacyRestartedGmres => "legacy-restarted-gmres",
            Self::IncrementalGivensCandidate => "incremental-givens-candidate",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum K0InitialGuessSource {
    Zero,
    ExternalPrevious,
    InternalLgmresPrevious,
    InternalGcrodrPrevious,
    RecycleProjection,
    Direct,
    NotApplicable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct K0StageIdentity {
    pub(crate) case_id: String,
    pub(crate) family: String,
    pub(crate) kernel_arm: K0KernelArm,
    pub(crate) attempt_id: u64,
    pub(crate) stage_index: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct K0StageWork {
    pub(crate) linear_matvecs: u64,
    pub(crate) diagnostic_matvecs: u64,
    pub(crate) operator_applies_total: u64,
    pub(crate) telemetry_jvp_overhead: u64,
}

impl K0StageWork {
    pub(crate) fn new(
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

    pub(crate) fn validate(&self) -> Result<(), K0ValidationError> {
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
pub(crate) struct K0StageGeometry {
    pub(crate) scaled_nonlinear_remainder: Option<f64>,
    pub(crate) rhs_jacobian_angle_radians: Option<f64>,
    pub(crate) singular_values: Vec<f64>,
}

impl K0StageGeometry {
    fn validate(&self) -> Result<(), K0ValidationError> {
        validate_nonnegative_finite(
            self.scaled_nonlinear_remainder,
            "scaled_nonlinear_remainder",
        )?;
        if let Some(angle) = self.rhs_jacobian_angle_radians {
            if !angle.is_finite() || !(0.0..=FRAC_PI_2).contains(&angle) {
                return Err(K0ValidationError::InvalidNumericValue(
                    "rhs_jacobian_angle_radians",
                ));
            }
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
pub(crate) struct K0StageError {
    pub(crate) kind: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct K0StageReceipt {
    pub(crate) schema: String,
    #[serde(flatten)]
    pub(crate) identity: K0StageIdentity,
    pub(crate) claim_class: String,
    pub(crate) execution_state: K0ExecutionState,
    pub(crate) solver_method: Option<String>,
    pub(crate) initial_guess_source: Option<K0InitialGuessSource>,
    pub(crate) initial_true_residual_norm: Option<f64>,
    pub(crate) final_true_residual_norm: Option<f64>,
    pub(crate) work: Option<K0StageWork>,
    pub(crate) geometry: Option<K0StageGeometry>,
    pub(crate) error: Option<K0StageError>,
}

impl K0StageReceipt {
    pub(crate) fn identity(&self) -> &K0StageIdentity {
        &self.identity
    }

    pub(crate) const fn residual_sign_coverage() -> K0ResidualSignCoverage {
        K0ResidualSignCoverage::NotCoveredByNormTelemetry
    }

    pub(crate) fn validate(&self) -> Result<(), K0ValidationError> {
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
pub(crate) enum K0ResidualSignCoverage {
    NotCoveredByNormTelemetry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum K0ValidationError {
    SchemaMismatch,
    ClaimBoundaryMismatch,
    InvalidIdentity,
    CounterOverflow,
    OperatorApplyTotalMismatch,
    InvalidNumericValue(&'static str),
    ErrorStateMissingWork,
    ErrorStateMissingError,
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
