//! Facade-owned, lossless records of the protected drivers.
//!
//! Field and variant order deliberately preserve `vigilode.stats.v4` JSON.
//! Conversion destructures every source field without `..` and matches every
//! source variant without a wildcard: upstream additions require an explicit
//! review here instead of silently losing diagnostics. Public record fields
//! recursively contain only facade-owned or standard-library types.
use serde::Serialize;

// Preserve the source WorkCounters omission of additive zero counters.
fn is_zero(value: &u64) -> bool {
    *value == 0
}

/// Recorded linear method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum LinearMethodRecord {
    /// The `Direct` policy or outcome.
    Direct,
    /// The `Gmres` policy or outcome.
    Gmres,
    /// The `Lgmres` policy or outcome.
    Lgmres,
    /// The `Gcrodr` policy or outcome.
    Gcrodr,
}

impl From<&rodas5p_core::LinearMethod> for LinearMethodRecord {
    fn from(source: &rodas5p_core::LinearMethod) -> Self {
        match source {
            rodas5p_core::LinearMethod::Direct => Self::Direct,
            rodas5p_core::LinearMethod::Gmres => Self::Gmres,
            rodas5p_core::LinearMethod::Lgmres => Self::Lgmres,
            rodas5p_core::LinearMethod::Gcrodr => Self::Gcrodr,
        }
    }
}

impl PartialEq<rodas5p_core::LinearMethod> for LinearMethodRecord {
    fn eq(&self, source: &rodas5p_core::LinearMethod) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<LinearMethodRecord> for rodas5p_core::LinearMethod {
    fn eq(&self, record: &LinearMethodRecord) -> bool {
        record == self
    }
}

/// Recorded preconditioner kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum PreconditionerRecord {
    /// The `None` policy or outcome.
    None,
    /// The `Jacobi` policy or outcome.
    Jacobi,
    /// The `Direct` policy or outcome.
    Direct,
}

impl From<&rodas5p_core::PreconditionerKind> for PreconditionerRecord {
    fn from(source: &rodas5p_core::PreconditionerKind) -> Self {
        match source {
            rodas5p_core::PreconditionerKind::None => Self::None,
            rodas5p_core::PreconditionerKind::Jacobi => Self::Jacobi,
            rodas5p_core::PreconditionerKind::Direct => Self::Direct,
        }
    }
}

impl PartialEq<rodas5p_core::PreconditionerKind> for PreconditionerRecord {
    fn eq(&self, source: &rodas5p_core::PreconditionerKind) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<PreconditionerRecord> for rodas5p_core::PreconditionerKind {
    fn eq(&self, record: &PreconditionerRecord) -> bool {
        record == self
    }
}

/// Recorded initial guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum InitialGuessRecord {
    /// The `Zero` policy or outcome.
    Zero,
    /// The `Previous` policy or outcome.
    Previous,
    /// The `PreviousStep` policy or outcome.
    PreviousStep,
    /// The `PreviousStepScaled` policy or outcome.
    PreviousStepScaled,
}

impl From<&rodas5p_core::InitialGuess> for InitialGuessRecord {
    fn from(source: &rodas5p_core::InitialGuess) -> Self {
        match source {
            rodas5p_core::InitialGuess::Zero => Self::Zero,
            rodas5p_core::InitialGuess::Previous => Self::Previous,
            rodas5p_core::InitialGuess::PreviousStep => Self::PreviousStep,
            rodas5p_core::InitialGuess::PreviousStepScaled => Self::PreviousStepScaled,
        }
    }
}

impl PartialEq<rodas5p_core::InitialGuess> for InitialGuessRecord {
    fn eq(&self, source: &rodas5p_core::InitialGuess) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<InitialGuessRecord> for rodas5p_core::InitialGuess {
    fn eq(&self, record: &InitialGuessRecord) -> bool {
        record == self
    }
}

/// Recorded controller kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum ControllerRecord {
    /// The `Integral` policy or outcome.
    Integral,
    /// The `Pi` policy or outcome.
    Pi,
    /// The `Predictive` policy or outcome.
    Predictive,
    /// The `PredictiveCapped` policy or outcome.
    PredictiveCapped,
    /// The `PredictiveCapped2` policy or outcome.
    PredictiveCapped2,
}

impl From<&rodas5p_integrators::ControllerKind> for ControllerRecord {
    fn from(source: &rodas5p_integrators::ControllerKind) -> Self {
        match source {
            rodas5p_integrators::ControllerKind::Integral => Self::Integral,
            rodas5p_integrators::ControllerKind::Pi => Self::Pi,
            rodas5p_integrators::ControllerKind::Predictive => Self::Predictive,
            rodas5p_integrators::ControllerKind::PredictiveCapped => Self::PredictiveCapped,
            rodas5p_integrators::ControllerKind::PredictiveCapped2 => Self::PredictiveCapped2,
        }
    }
}

impl PartialEq<rodas5p_integrators::ControllerKind> for ControllerRecord {
    fn eq(&self, source: &rodas5p_integrators::ControllerKind) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<ControllerRecord> for rodas5p_integrators::ControllerKind {
    fn eq(&self, record: &ControllerRecord) -> bool {
        record == self
    }
}

/// Recorded max step policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum MaxStepPolicyRecord {
    /// The `StrictRepresentedCap` policy or outcome.
    StrictRepresentedCap,
    /// The `AllowClockResolutionSlack` policy or outcome.
    AllowClockResolutionSlack,
}

impl From<&rodas5p_integrators::MaxStepPolicy> for MaxStepPolicyRecord {
    fn from(source: &rodas5p_integrators::MaxStepPolicy) -> Self {
        match source {
            rodas5p_integrators::MaxStepPolicy::StrictRepresentedCap => Self::StrictRepresentedCap,
            rodas5p_integrators::MaxStepPolicy::AllowClockResolutionSlack => {
                Self::AllowClockResolutionSlack
            }
        }
    }
}

impl PartialEq<rodas5p_integrators::MaxStepPolicy> for MaxStepPolicyRecord {
    fn eq(&self, source: &rodas5p_integrators::MaxStepPolicy) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<MaxStepPolicyRecord> for rodas5p_integrators::MaxStepPolicy {
    fn eq(&self, record: &MaxStepPolicyRecord) -> bool {
        record == self
    }
}

/// Recorded adaptive failure kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum StepFailureKind {
    /// The `LocalError` policy or outcome.
    LocalError,
    /// The `LinearSolve` policy or outcome.
    LinearSolve,
    /// The `NonlinearSolve` policy or outcome.
    NonlinearSolve,
    /// The `NonFinite` policy or outcome.
    NonFinite,
}

impl From<&rodas5p_integrators::AdaptiveFailureKind> for StepFailureKind {
    fn from(source: &rodas5p_integrators::AdaptiveFailureKind) -> Self {
        match source {
            rodas5p_integrators::AdaptiveFailureKind::LocalError => Self::LocalError,
            rodas5p_integrators::AdaptiveFailureKind::LinearSolve => Self::LinearSolve,
            rodas5p_integrators::AdaptiveFailureKind::NonlinearSolve => Self::NonlinearSolve,
            rodas5p_integrators::AdaptiveFailureKind::NonFinite => Self::NonFinite,
        }
    }
}

impl PartialEq<rodas5p_integrators::AdaptiveFailureKind> for StepFailureKind {
    fn eq(&self, source: &rodas5p_integrators::AdaptiveFailureKind) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<StepFailureKind> for rodas5p_integrators::AdaptiveFailureKind {
    fn eq(&self, record: &StepFailureKind) -> bool {
        record == self
    }
}

/// Recorded driver stop reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum StopReason {
    /// The `AttemptBudget` policy or outcome.
    AttemptBudget,
    /// The `MinimumStep` policy or outcome.
    MinimumStep,
}

impl From<&rodas5p_integrators::DriverStopReason> for StopReason {
    fn from(source: &rodas5p_integrators::DriverStopReason) -> Self {
        match source {
            rodas5p_integrators::DriverStopReason::AttemptBudget => Self::AttemptBudget,
            rodas5p_integrators::DriverStopReason::MinimumStep => Self::MinimumStep,
        }
    }
}

impl PartialEq<rodas5p_integrators::DriverStopReason> for StopReason {
    fn eq(&self, source: &rodas5p_integrators::DriverStopReason) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<StopReason> for rodas5p_integrators::DriverStopReason {
    fn eq(&self, record: &StopReason) -> bool {
        record == self
    }
}

/// Recorded rodas5p fast lu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum LuRecord {
    /// The `InPlaceZeroSkipping` policy or outcome.
    InPlaceZeroSkipping,
    /// The `InPlaceColumnExtents` policy or outcome.
    InPlaceColumnExtents,
    /// The `Faer` policy or outcome.
    Faer,
    /// The `Banded` policy or outcome.
    Banded,
}

impl From<&rodas5p_integrators::Rodas5pFastLu> for LuRecord {
    fn from(source: &rodas5p_integrators::Rodas5pFastLu) -> Self {
        match source {
            rodas5p_integrators::Rodas5pFastLu::InPlaceZeroSkipping => Self::InPlaceZeroSkipping,
            rodas5p_integrators::Rodas5pFastLu::InPlaceColumnExtents => Self::InPlaceColumnExtents,
            rodas5p_integrators::Rodas5pFastLu::Faer => Self::Faer,
            rodas5p_integrators::Rodas5pFastLu::Banded => Self::Banded,
        }
    }
}

impl PartialEq<rodas5p_integrators::Rodas5pFastLu> for LuRecord {
    fn eq(&self, source: &rodas5p_integrators::Rodas5pFastLu) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<LuRecord> for rodas5p_integrators::Rodas5pFastLu {
    fn eq(&self, record: &LuRecord) -> bool {
        record == self
    }
}

/// Recorded dense error control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum InteriorControlRecord {
    /// The `Off` policy or outcome.
    Off,
    /// The `ReportDefect` policy or outcome.
    ReportDefect,
    /// The `Report` policy or outcome.
    Report,
    /// The `Enforce` policy or outcome.
    Enforce,
    /// The `VerifiedResidual` policy or outcome.
    VerifiedResidual,
    /// The `WFilteredReport` policy or outcome.
    WFilteredReport,
    /// The `WFilteredEnforce` policy or outcome.
    WFilteredEnforce,
}

impl From<&rodas5p_integrators::DenseErrorControl> for InteriorControlRecord {
    fn from(source: &rodas5p_integrators::DenseErrorControl) -> Self {
        match source {
            rodas5p_integrators::DenseErrorControl::Off => Self::Off,
            rodas5p_integrators::DenseErrorControl::ReportDefect => Self::ReportDefect,
            rodas5p_integrators::DenseErrorControl::Report => Self::Report,
            rodas5p_integrators::DenseErrorControl::Enforce => Self::Enforce,
            rodas5p_integrators::DenseErrorControl::VerifiedResidual => Self::VerifiedResidual,
            rodas5p_integrators::DenseErrorControl::WFilteredReport => Self::WFilteredReport,
            rodas5p_integrators::DenseErrorControl::WFilteredEnforce => Self::WFilteredEnforce,
        }
    }
}

impl PartialEq<rodas5p_integrators::DenseErrorControl> for InteriorControlRecord {
    fn eq(&self, source: &rodas5p_integrators::DenseErrorControl) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<InteriorControlRecord> for rodas5p_integrators::DenseErrorControl {
    fn eq(&self, record: &InteriorControlRecord) -> bool {
        record == self
    }
}

/// Recorded interior sampling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum InteriorSamplingRecord {
    /// The `OutputTimes` policy or outcome.
    OutputTimes,
    /// The `EveryStep` policy or outcome.
    EveryStep,
}

impl From<&rodas5p_integrators::InteriorSampling> for InteriorSamplingRecord {
    fn from(source: &rodas5p_integrators::InteriorSampling) -> Self {
        match source {
            rodas5p_integrators::InteriorSampling::OutputTimes => Self::OutputTimes,
            rodas5p_integrators::InteriorSampling::EveryStep => Self::EveryStep,
        }
    }
}

impl PartialEq<rodas5p_integrators::InteriorSampling> for InteriorSamplingRecord {
    fn eq(&self, source: &rodas5p_integrators::InteriorSampling) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<InteriorSamplingRecord> for rodas5p_integrators::InteriorSampling {
    fn eq(&self, record: &InteriorSamplingRecord) -> bool {
        record == self
    }
}

/// Recorded band reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum BandReferenceRecord {
    /// The `Jvp` policy or outcome.
    Jvp,
    /// The `DenseJacobian` policy or outcome.
    DenseJacobian,
}

impl From<&rodas5p_integrators::BandReference> for BandReferenceRecord {
    fn from(source: &rodas5p_integrators::BandReference) -> Self {
        match source {
            rodas5p_integrators::BandReference::Jvp => Self::Jvp,
            rodas5p_integrators::BandReference::DenseJacobian => Self::DenseJacobian,
        }
    }
}

impl PartialEq<rodas5p_integrators::BandReference> for BandReferenceRecord {
    fn eq(&self, source: &rodas5p_integrators::BandReference) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<BandReferenceRecord> for rodas5p_integrators::BandReference {
    fn eq(&self, record: &BandReferenceRecord) -> bool {
        record == self
    }
}

/// Recorded routing reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum RoutingReasonRecord {
    /// The `DeclaredBand` policy or outcome.
    DeclaredBand,
    /// The `DeclaredDenseSmall` policy or outcome.
    DeclaredDenseSmall,
    /// The `ExplicitJacobian` policy or outcome.
    ExplicitJacobian,
    /// The `UnstructuredJvp` policy or outcome.
    UnstructuredJvp,
}

impl From<&rodas5p_integrators::RoutingReason> for RoutingReasonRecord {
    fn from(source: &rodas5p_integrators::RoutingReason) -> Self {
        match source {
            rodas5p_integrators::RoutingReason::DeclaredBand => Self::DeclaredBand,
            rodas5p_integrators::RoutingReason::DeclaredDenseSmall => Self::DeclaredDenseSmall,
            rodas5p_integrators::RoutingReason::ExplicitJacobian => Self::ExplicitJacobian,
            rodas5p_integrators::RoutingReason::UnstructuredJvp => Self::UnstructuredJvp,
        }
    }
}

impl PartialEq<rodas5p_integrators::RoutingReason> for RoutingReasonRecord {
    fn eq(&self, source: &rodas5p_integrators::RoutingReason) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<RoutingReasonRecord> for rodas5p_integrators::RoutingReason {
    fn eq(&self, record: &RoutingReasonRecord) -> bool {
        record == self
    }
}

/// Recorded routed driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum RoutedDriverRecord {
    /// The `Banded` policy or outcome.
    Banded,
    /// The `Dense` policy or outcome.
    Dense,
    /// The `MatrixFree` policy or outcome.
    MatrixFree,
}

impl From<&rodas5p_integrators::RoutedDriver> for RoutedDriverRecord {
    fn from(source: &rodas5p_integrators::RoutedDriver) -> Self {
        match source {
            rodas5p_integrators::RoutedDriver::Banded => Self::Banded,
            rodas5p_integrators::RoutedDriver::Dense => Self::Dense,
            rodas5p_integrators::RoutedDriver::MatrixFree => Self::MatrixFree,
        }
    }
}

impl PartialEq<rodas5p_integrators::RoutedDriver> for RoutedDriverRecord {
    fn eq(&self, source: &rodas5p_integrators::RoutedDriver) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<RoutedDriverRecord> for rodas5p_integrators::RoutedDriver {
    fn eq(&self, record: &RoutedDriverRecord) -> bool {
        record == self
    }
}

/// Recorded work counters; field order matches the protected driver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct WorkRecord {
    /// Recorded `rhs_calls` value.
    pub rhs_calls: u64,
    /// Recorded `rhs_batch_calls` value.
    pub rhs_batch_calls: u64,
    /// Recorded `rhs_evaluations` value.
    pub rhs_evaluations: u64,
    /// Recorded `ft_calls` value.
    pub ft_calls: u64,
    /// Recorded `jacobian_builds` value.
    pub jacobian_builds: u64,
    /// Recorded `jvp_calls` value.
    pub jvp_calls: u64,
    /// Recorded `jvp_vectors` value.
    pub jvp_vectors: u64,
    /// Recorded `mass_matvecs` value.
    pub mass_matvecs: u64,
    /// Recorded `nonlinear_solves` value.
    pub nonlinear_solves: u64,
    /// Recorded `nonlinear_iterations` value.
    pub nonlinear_iterations: u64,
    /// Recorded `nonlinear_residual_evaluations` value.
    pub nonlinear_residual_evaluations: u64,
    /// Recorded `nonlinear_jacobian_evaluations` value.
    pub nonlinear_jacobian_evaluations: u64,
    /// Recorded `nonlinear_failures` value.
    pub nonlinear_failures: u64,
    /// Recorded `linear_solves` value.
    pub linear_solves: u64,
    /// Recorded `linear_iterations` value.
    pub linear_iterations: u64,
    /// Recorded `linear_matvecs` value.
    pub linear_matvecs: u64,
    /// Recorded `preconditioner_apps` value.
    pub preconditioner_apps: u64,
    /// Recorded `direct_factorizations` value.
    pub direct_factorizations: u64,
    /// Recorded `direct_solve_calls` value.
    pub direct_solve_calls: u64,
    /// Recorded `recycle_projection_calls` value.
    pub recycle_projection_calls: u64,
    /// Recorded `recycle_same_operator_uses` value.
    pub recycle_same_operator_uses: u64,
    /// Recorded `recycle_cross_operator_refreshes` value.
    pub recycle_cross_operator_refreshes: u64,
    /// Recorded `recycle_refresh_matvecs` value.
    pub recycle_refresh_matvecs: u64,
    /// Recorded `recycle_updates` value.
    pub recycle_updates: u64,
    /// Recorded `recycle_vectors_selected` value.
    pub recycle_vectors_selected: u64,
    /// Recorded `recycle_dropped_vectors` value.
    pub recycle_dropped_vectors: u64,
    /// Recorded `harmonic_ritz_solves` value.
    pub harmonic_ritz_solves: u64,
    /// Recorded `orthogonalization_inner_products` value.
    pub orthogonalization_inner_products: u64,
    /// Recorded `orthogonalization_vector_updates` value.
    pub orthogonalization_vector_updates: u64,
    /// Recorded `diagnostic_matvecs` value.
    pub diagnostic_matvecs: u64,
    /// Recorded `phi_actions` value.
    pub phi_actions: u64,
    /// Recorded `phi_krylov_vectors` value.
    pub phi_krylov_vectors: u64,
    /// Recorded `phi_projected_exponentials` value.
    pub phi_projected_exponentials: u64,
    /// Recorded `phi_restarts` value.
    pub phi_restarts: u64,
    /// Recorded `phi_dense_oracle_calls` value.
    pub phi_dense_oracle_calls: u64,
    /// Recorded `block_linear_solves` value.
    pub block_linear_solves: u64,
    /// Recorded `block_linear_iterations` value.
    pub block_linear_iterations: u64,
    /// Recorded `block_matvecs` value.
    pub block_matvecs: u64,
    /// Recorded `block_preconditioner_apps` value.
    pub block_preconditioner_apps: u64,
    /// Recorded `fast_attempts` value.
    pub fast_attempts: u64,
    /// Recorded `fast_accepts` value.
    pub fast_accepts: u64,
    /// Recorded `fallback_steps` value.
    pub fallback_steps: u64,
    /// Recorded `accepted_steps` value.
    pub accepted_steps: u64,
    /// Recorded `rejected_steps` value.
    pub rejected_steps: u64,
    /// Recorded `local_error_failures` value.
    pub local_error_failures: u64,
    /// Recorded `linear_solve_failures` value.
    pub linear_solve_failures: u64,
    /// Recorded `nonlinear_solve_failures` value.
    pub nonlinear_solve_failures: u64,
    /// Recorded `nonfinite_step_failures` value.
    pub nonfinite_step_failures: u64,
    /// Recorded `forced_stage_solves` value.
    pub forced_stage_solves: u64,
    /// Recorded `jacobian_matvecs` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub jacobian_matvecs: u64,
    /// Recorded `linear_matvec_vectors` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub linear_matvec_vectors: u64,
    /// Recorded `preconditioner_vectors` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub preconditioner_vectors: u64,
    /// Recorded `merged_unknown_vector_calls` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub merged_unknown_vector_calls: u64,
    /// Recorded `phi_weight_underflows` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub phi_weight_underflows: u64,
    /// Recorded `poly_block_products` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_block_products: u64,
    /// Recorded `poly_vector_products` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_vector_products: u64,
    /// Recorded `poly_coefficient_setups` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_coefficient_setups: u64,
    /// Recorded `poly_coefficient_reuses` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_coefficient_reuses: u64,
    /// Recorded `poly_block_allocations` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_block_allocations: u64,
    /// Recorded `poly_fallbacks` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_fallbacks: u64,
    /// Recorded `recycle_update_refreshes` value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub recycle_update_refreshes: u64,
}

impl From<&rodas5p_core::WorkCounters> for WorkRecord {
    fn from(source: &rodas5p_core::WorkCounters) -> Self {
        let rodas5p_core::WorkCounters {
            rhs_calls,
            rhs_batch_calls,
            rhs_evaluations,
            ft_calls,
            jacobian_builds,
            jvp_calls,
            jvp_vectors,
            mass_matvecs,
            nonlinear_solves,
            nonlinear_iterations,
            nonlinear_residual_evaluations,
            nonlinear_jacobian_evaluations,
            nonlinear_failures,
            linear_solves,
            linear_iterations,
            linear_matvecs,
            preconditioner_apps,
            direct_factorizations,
            direct_solve_calls,
            recycle_projection_calls,
            recycle_same_operator_uses,
            recycle_cross_operator_refreshes,
            recycle_refresh_matvecs,
            recycle_updates,
            recycle_vectors_selected,
            recycle_dropped_vectors,
            harmonic_ritz_solves,
            orthogonalization_inner_products,
            orthogonalization_vector_updates,
            diagnostic_matvecs,
            phi_actions,
            phi_krylov_vectors,
            phi_projected_exponentials,
            phi_restarts,
            phi_dense_oracle_calls,
            block_linear_solves,
            block_linear_iterations,
            block_matvecs,
            block_preconditioner_apps,
            fast_attempts,
            fast_accepts,
            fallback_steps,
            accepted_steps,
            rejected_steps,
            local_error_failures,
            linear_solve_failures,
            nonlinear_solve_failures,
            nonfinite_step_failures,
            forced_stage_solves,
            jacobian_matvecs,
            linear_matvec_vectors,
            preconditioner_vectors,
            merged_unknown_vector_calls,
            phi_weight_underflows,
            poly_block_products,
            poly_vector_products,
            poly_coefficient_setups,
            poly_coefficient_reuses,
            poly_block_allocations,
            poly_fallbacks,
            recycle_update_refreshes,
        } = source;
        Self {
            rhs_calls: *rhs_calls,
            rhs_batch_calls: *rhs_batch_calls,
            rhs_evaluations: *rhs_evaluations,
            ft_calls: *ft_calls,
            jacobian_builds: *jacobian_builds,
            jvp_calls: *jvp_calls,
            jvp_vectors: *jvp_vectors,
            mass_matvecs: *mass_matvecs,
            nonlinear_solves: *nonlinear_solves,
            nonlinear_iterations: *nonlinear_iterations,
            nonlinear_residual_evaluations: *nonlinear_residual_evaluations,
            nonlinear_jacobian_evaluations: *nonlinear_jacobian_evaluations,
            nonlinear_failures: *nonlinear_failures,
            linear_solves: *linear_solves,
            linear_iterations: *linear_iterations,
            linear_matvecs: *linear_matvecs,
            preconditioner_apps: *preconditioner_apps,
            direct_factorizations: *direct_factorizations,
            direct_solve_calls: *direct_solve_calls,
            recycle_projection_calls: *recycle_projection_calls,
            recycle_same_operator_uses: *recycle_same_operator_uses,
            recycle_cross_operator_refreshes: *recycle_cross_operator_refreshes,
            recycle_refresh_matvecs: *recycle_refresh_matvecs,
            recycle_updates: *recycle_updates,
            recycle_vectors_selected: *recycle_vectors_selected,
            recycle_dropped_vectors: *recycle_dropped_vectors,
            harmonic_ritz_solves: *harmonic_ritz_solves,
            orthogonalization_inner_products: *orthogonalization_inner_products,
            orthogonalization_vector_updates: *orthogonalization_vector_updates,
            diagnostic_matvecs: *diagnostic_matvecs,
            phi_actions: *phi_actions,
            phi_krylov_vectors: *phi_krylov_vectors,
            phi_projected_exponentials: *phi_projected_exponentials,
            phi_restarts: *phi_restarts,
            phi_dense_oracle_calls: *phi_dense_oracle_calls,
            block_linear_solves: *block_linear_solves,
            block_linear_iterations: *block_linear_iterations,
            block_matvecs: *block_matvecs,
            block_preconditioner_apps: *block_preconditioner_apps,
            fast_attempts: *fast_attempts,
            fast_accepts: *fast_accepts,
            fallback_steps: *fallback_steps,
            accepted_steps: *accepted_steps,
            rejected_steps: *rejected_steps,
            local_error_failures: *local_error_failures,
            linear_solve_failures: *linear_solve_failures,
            nonlinear_solve_failures: *nonlinear_solve_failures,
            nonfinite_step_failures: *nonfinite_step_failures,
            forced_stage_solves: *forced_stage_solves,
            jacobian_matvecs: *jacobian_matvecs,
            linear_matvec_vectors: *linear_matvec_vectors,
            preconditioner_vectors: *preconditioner_vectors,
            merged_unknown_vector_calls: *merged_unknown_vector_calls,
            phi_weight_underflows: *phi_weight_underflows,
            poly_block_products: *poly_block_products,
            poly_vector_products: *poly_vector_products,
            poly_coefficient_setups: *poly_coefficient_setups,
            poly_coefficient_reuses: *poly_coefficient_reuses,
            poly_block_allocations: *poly_block_allocations,
            poly_fallbacks: *poly_fallbacks,
            recycle_update_refreshes: *recycle_update_refreshes,
        }
    }
}

impl PartialEq<rodas5p_core::WorkCounters> for WorkRecord {
    fn eq(&self, source: &rodas5p_core::WorkCounters) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<WorkRecord> for rodas5p_core::WorkCounters {
    fn eq(&self, record: &WorkRecord) -> bool {
        record == self
    }
}

/// Recorded linear solver config; field order matches the protected driver.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct LinearSolverRecord {
    /// Recorded `method` value.
    pub method: LinearMethodRecord,
    /// Recorded `rtol` value.
    pub rtol: f64,
    /// Recorded `atol` value.
    pub atol: f64,
    /// Recorded `restart` value.
    pub restart: usize,
    /// Recorded `maxiter` value.
    pub maxiter: usize,
    /// Recorded `inner_m` value.
    pub inner_m: usize,
    /// Recorded `outer_k` value.
    pub outer_k: usize,
    /// Recorded `recycle_dim` value.
    pub recycle_dim: usize,
    /// Recorded `recycle_rank_tol` value.
    pub recycle_rank_tol: f64,
    /// Recorded `preconditioner` value.
    pub preconditioner: PreconditionerRecord,
    /// Recorded `x0_strategy` value.
    pub x0_strategy: InitialGuessRecord,
}

impl From<&rodas5p_core::LinearSolverConfig> for LinearSolverRecord {
    fn from(source: &rodas5p_core::LinearSolverConfig) -> Self {
        let rodas5p_core::LinearSolverConfig {
            method,
            rtol,
            atol,
            restart,
            maxiter,
            inner_m,
            outer_k,
            recycle_dim,
            recycle_rank_tol,
            preconditioner,
            x0_strategy,
        } = source;
        Self {
            method: method.into(),
            rtol: *rtol,
            atol: *atol,
            restart: *restart,
            maxiter: *maxiter,
            inner_m: *inner_m,
            outer_k: *outer_k,
            recycle_dim: *recycle_dim,
            recycle_rank_tol: *recycle_rank_tol,
            preconditioner: preconditioner.into(),
            x0_strategy: x0_strategy.into(),
        }
    }
}

impl PartialEq<rodas5p_core::LinearSolverConfig> for LinearSolverRecord {
    fn eq(&self, source: &rodas5p_core::LinearSolverConfig) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<LinearSolverRecord> for rodas5p_core::LinearSolverConfig {
    fn eq(&self, record: &LinearSolverRecord) -> bool {
        record == self
    }
}

/// Recorded adaptive step config; field order matches the protected driver.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AdaptiveRecord {
    /// Recorded `atol` value.
    pub atol: f64,
    /// Recorded `rtol` value.
    pub rtol: f64,
    /// Recorded `initial_step` value.
    pub initial_step: f64,
    /// Recorded `min_step` value.
    pub min_step: f64,
    /// Recorded `max_step` value.
    pub max_step: f64,
    /// Recorded `max_attempts` value.
    pub max_attempts: usize,
    /// Recorded `safety` value.
    pub safety: f64,
    /// Recorded `min_factor` value.
    pub min_factor: f64,
    /// Recorded `max_factor` value.
    pub max_factor: f64,
    /// Recorded `reject_max_factor` value.
    pub reject_max_factor: f64,
    /// Recorded `controller` value.
    pub controller: ControllerRecord,
    /// How `max_step` binds the represented step (re-audit R4,
    /// R4-TIME-DEV-04); the default allows one clock resolution of slack.
    pub max_step_policy: MaxStepPolicyRecord,
}

impl From<&rodas5p_integrators::AdaptiveStepConfig> for AdaptiveRecord {
    fn from(source: &rodas5p_integrators::AdaptiveStepConfig) -> Self {
        let rodas5p_integrators::AdaptiveStepConfig {
            atol,
            rtol,
            initial_step,
            min_step,
            max_step,
            max_attempts,
            safety,
            min_factor,
            max_factor,
            reject_max_factor,
            controller,
            max_step_policy,
        } = source;
        Self {
            atol: *atol,
            rtol: *rtol,
            initial_step: *initial_step,
            min_step: *min_step,
            max_step: *max_step,
            max_attempts: *max_attempts,
            safety: *safety,
            min_factor: *min_factor,
            max_factor: *max_factor,
            reject_max_factor: *reject_max_factor,
            controller: controller.into(),
            max_step_policy: max_step_policy.into(),
        }
    }
}

impl PartialEq<rodas5p_integrators::AdaptiveStepConfig> for AdaptiveRecord {
    fn eq(&self, source: &rodas5p_integrators::AdaptiveStepConfig) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<AdaptiveRecord> for rodas5p_integrators::AdaptiveStepConfig {
    fn eq(&self, record: &AdaptiveRecord) -> bool {
        record == self
    }
}

/// Recorded banded work; field order matches the protected driver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct BandedWorkRecord {
    /// Multiply-subtracts count 2, divisions 1, `W` assembly 1 per band entry.
    pub factor_operations: u64,
    /// Recorded `solve_operations` value.
    pub solve_operations: u64,
    /// Eight-byte slots of the Jacobian band, factors and optional exact key.
    pub stored_slots: usize,
}

impl From<&rodas5p_integrators::BandedWork> for BandedWorkRecord {
    fn from(source: &rodas5p_integrators::BandedWork) -> Self {
        let rodas5p_integrators::BandedWork {
            factor_operations,
            solve_operations,
            stored_slots,
        } = source;
        Self {
            factor_operations: *factor_operations,
            solve_operations: *solve_operations,
            stored_slots: *stored_slots,
        }
    }
}

impl PartialEq<rodas5p_integrators::BandedWork> for BandedWorkRecord {
    fn eq(&self, source: &rodas5p_integrators::BandedWork) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<BandedWorkRecord> for rodas5p_integrators::BandedWork {
    fn eq(&self, record: &BandedWorkRecord) -> bool {
        record == self
    }
}

/// Recorded fast interior work; field order matches the protected driver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FastInteriorRecord {
    /// Estimates formed (one per attempt the embedded estimate accepted
    /// whose midpoint right-hand side and back-solve succeeded).
    pub samples: u64,
    /// Attempts the embedded estimate accepted and the interior estimate
    /// rejected (included in the run's `rejected_steps`).
    pub interior_rejections: u64,
    /// Right-hand-side calls of the estimates (attempted calls; also in the
    /// run counters, since the estimates decide steps).
    pub rhs_calls: u64,
    /// Back-solves with the step's factors (attempted; also in the run
    /// counters as linear and direct solves, and on the banded route in
    /// `BandedWork::solve_operations`).
    pub back_solves: u64,
    /// The largest estimate of an accepted step, in tolerance units (0 when
    /// none was formed). An estimate, not a bound.
    pub max_estimate: f64,
}

impl From<&rodas5p_integrators::FastInteriorWork> for FastInteriorRecord {
    fn from(source: &rodas5p_integrators::FastInteriorWork) -> Self {
        let rodas5p_integrators::FastInteriorWork {
            samples,
            interior_rejections,
            rhs_calls,
            back_solves,
            max_estimate,
        } = source;
        Self {
            samples: *samples,
            interior_rejections: *interior_rejections,
            rhs_calls: *rhs_calls,
            back_solves: *back_solves,
            max_estimate: *max_estimate,
        }
    }
}

impl PartialEq<rodas5p_integrators::FastInteriorWork> for FastInteriorRecord {
    fn eq(&self, source: &rodas5p_integrators::FastInteriorWork) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<FastInteriorRecord> for rodas5p_integrators::FastInteriorWork {
    fn eq(&self, record: &FastInteriorRecord) -> bool {
        record == self
    }
}

/// Recorded routing record; field order matches the protected driver.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RoutingDecisionRecord {
    /// Recorded `declared` value.
    pub declared: StructureRecord,
    /// Recorded `dimension` value.
    pub dimension: usize,
    /// Recorded `reason` value.
    pub reason: RoutingReasonRecord,
    /// Recorded `driver` value.
    pub driver: RoutedDriverRecord,
    /// Recorded `driver_id` value.
    pub driver_id: &'static str,
    /// Recorded `fallback` value.
    pub fallback: Option<RoutingFallbackRecord>,
    /// Recorded `verification` value.
    pub verification: Option<BandVerificationRecord>,
}

impl From<&rodas5p_integrators::RoutingRecord> for RoutingDecisionRecord {
    fn from(source: &rodas5p_integrators::RoutingRecord) -> Self {
        let rodas5p_integrators::RoutingRecord {
            declared,
            dimension,
            reason,
            driver,
            driver_id,
            fallback,
            verification,
        } = source;
        Self {
            declared: declared.into(),
            dimension: *dimension,
            reason: reason.into(),
            driver: driver.into(),
            driver_id: *driver_id,
            fallback: fallback.as_ref().map(Into::into),
            verification: verification.as_ref().map(Into::into),
        }
    }
}

impl PartialEq<rodas5p_integrators::RoutingRecord> for RoutingDecisionRecord {
    fn eq(&self, source: &rodas5p_integrators::RoutingRecord) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<RoutingDecisionRecord> for rodas5p_integrators::RoutingRecord {
    fn eq(&self, record: &RoutingDecisionRecord) -> bool {
        record == self
    }
}

/// Recorded band verification; field order matches the protected driver.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BandVerificationRecord {
    /// Recorded `lower` value.
    pub lower: usize,
    /// Recorded `upper` value.
    pub upper: usize,
    /// Recorded `reference` value.
    pub reference: BandReferenceRecord,
    /// Recorded `vectors` value.
    pub vectors: usize,
    /// Recorded `seed` value.
    pub seed: u64,
    /// Recorded `tolerance` value.
    pub tolerance: f64,
    /// `||J_band v - J_ref v||_2 / ||J_ref v||_2` per vector (0 when both
    /// products are zero).
    pub relative_mismatch: Vec<f64>,
    /// Recorded `band_fills` value.
    pub band_fills: u64,
    /// Recorded `band_products` value.
    pub band_products: u64,
    /// Recorded `jvp_products` value.
    pub jvp_products: u64,
    /// Recorded `dense_jacobian_builds` value.
    pub dense_jacobian_builds: u64,
    /// Recorded `dense_products` value.
    pub dense_products: u64,
    /// The counters charged to the run: the band fill and the dense
    /// Jacobian in `jacobian_builds`, band and dense products in
    /// `jacobian_matvecs`, JVP products in `jvp_calls` / `jvp_vectors`.
    pub charged: WorkRecord,
}

impl From<&rodas5p_integrators::BandVerification> for BandVerificationRecord {
    fn from(source: &rodas5p_integrators::BandVerification) -> Self {
        let rodas5p_integrators::BandVerification {
            lower,
            upper,
            reference,
            vectors,
            seed,
            tolerance,
            relative_mismatch,
            band_fills,
            band_products,
            jvp_products,
            dense_jacobian_builds,
            dense_products,
            charged,
        } = source;
        Self {
            lower: *lower,
            upper: *upper,
            reference: reference.into(),
            vectors: *vectors,
            seed: *seed,
            tolerance: *tolerance,
            relative_mismatch: relative_mismatch.clone(),
            band_fills: *band_fills,
            band_products: *band_products,
            jvp_products: *jvp_products,
            dense_jacobian_builds: *dense_jacobian_builds,
            dense_products: *dense_products,
            charged: charged.into(),
        }
    }
}

impl PartialEq<rodas5p_integrators::BandVerification> for BandVerificationRecord {
    fn eq(&self, source: &rodas5p_integrators::BandVerification) -> bool {
        self == &Self::from(source)
    }
}

impl PartialEq<BandVerificationRecord> for rodas5p_integrators::BandVerification {
    fn eq(&self, record: &BandVerificationRecord) -> bool {
        record == self
    }
}

/// Structure declared to the protected router.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[non_exhaustive]
pub enum StructureRecord {
    /// No structure was declared.
    Unstructured,
    /// Dense structure was declared.
    Dense,
    /// Band structure was declared.
    Banded {
        /// Sub-diagonals.
        lower: usize,
        /// Super-diagonals.
        upper: usize,
    },
    /// A sparse linear part was declared.
    LinearPart {
        /// Stored nonzero entries.
        nnz: usize,
    },
}
impl From<&rodas5p_integrators::ProblemStructure> for StructureRecord {
    fn from(source: &rodas5p_integrators::ProblemStructure) -> Self {
        match source {
            rodas5p_integrators::ProblemStructure::Unstructured => Self::Unstructured,
            rodas5p_integrators::ProblemStructure::Dense => Self::Dense,
            rodas5p_integrators::ProblemStructure::Banded { lower, upper } => Self::Banded {
                lower: *lower,
                upper: *upper,
            },
            rodas5p_integrators::ProblemStructure::LinearPart { nnz } => {
                Self::LinearPart { nnz: *nnz }
            }
        }
    }
}
/// A router fallback, distinct from a refused band verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[non_exhaustive]
pub enum RoutingFallbackRecord {
    /// A declared dense system exceeded the small-dense cutoff.
    DenseDeclaredAboveSmallMax {
        /// System dimension.
        dimension: usize,
        /// Small-dense dimension cutoff.
        small_max: usize,
    },
}
impl From<&rodas5p_integrators::RoutingFallback> for RoutingFallbackRecord {
    fn from(source: &rodas5p_integrators::RoutingFallback) -> Self {
        match source {
            rodas5p_integrators::RoutingFallback::DenseDeclaredAboveSmallMax {
                dimension,
                small_max,
            } => Self::DenseDeclaredAboveSmallMax {
                dimension: *dimension,
                small_max: *small_max,
            },
        }
    }
}

// The solve path consumes router diagnostics, preserving their vector storage.
// Borrowed conversions above are for read-only inspection and compatibility.
impl From<rodas5p_integrators::BandVerification> for BandVerificationRecord {
    fn from(source: rodas5p_integrators::BandVerification) -> Self {
        let rodas5p_integrators::BandVerification {
            lower,
            upper,
            reference,
            vectors,
            seed,
            tolerance,
            relative_mismatch,
            band_fills,
            band_products,
            jvp_products,
            dense_jacobian_builds,
            dense_products,
            charged,
        } = source;
        Self {
            lower,
            upper,
            reference: (&reference).into(),
            vectors,
            seed,
            tolerance,
            relative_mismatch,
            band_fills,
            band_products,
            jvp_products,
            dense_jacobian_builds,
            dense_products,
            charged: (&charged).into(),
        }
    }
}
impl From<rodas5p_integrators::RoutingRecord> for RoutingDecisionRecord {
    fn from(source: rodas5p_integrators::RoutingRecord) -> Self {
        let rodas5p_integrators::RoutingRecord {
            declared,
            dimension,
            reason,
            driver,
            driver_id,
            fallback,
            verification,
        } = source;
        Self {
            declared: (&declared).into(),
            dimension,
            reason: (&reason).into(),
            driver: (&driver).into(),
            driver_id,
            fallback: fallback.as_ref().map(Into::into),
            verification: verification.map(Into::into),
        }
    }
}
