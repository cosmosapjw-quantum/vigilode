//! Feature-gated reusable-preconditioner transaction substrate for Audit-2.
//!
//! This module is research-only. It does not alter a production dispatcher or
//! admit an accuracy, timing, or scalability claim.

use std::sync::Arc;

use rodas5p_core::{
    CoreError, CoreResult, ExactOperatorIdentity, ExactPreconditionerIdentity, InitialGuess,
    LinearMethod, LinearOperator, LinearSolverConfig, Preconditioner, PreconditionerKind,
    WorkCounters, safe_l2,
};
use serde::{Deserialize, Serialize};

use crate::audit2_matrix_free_research::{
    Audit2MatrixFreeCommonWConfig, Audit2MatrixFreeCorrectionFailure,
    Audit2MatrixFreeCorrectionOutcome, Audit2MatrixFreeCorrectionSuccess,
    Audit2MatrixFreeCorrectionWork, run_audit2_matrix_free_common_w_correction_with_preconditioner,
};
use crate::{StepContext, StepResult, StructuredBlockSystem, finish_step, sequential_stages};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit2ReusablePreconditionerIdentity {
    pub provider: String,
    pub revision: u64,
    pub configuration_bits: Vec<u64>,
    pub expected_inverse_diagonal_bits: Vec<u64>,
}

impl Audit2ReusablePreconditionerIdentity {
    fn validate(&self, dimension: usize) -> CoreResult<()> {
        if self.provider.trim().is_empty()
            || self.configuration_bits.is_empty()
            || self.expected_inverse_diagonal_bits.len() != dimension
            || self
                .expected_inverse_diagonal_bits
                .iter()
                .any(|bits| !f64::from_bits(*bits).is_finite())
        {
            return Err(CoreError::InvalidInput(
                "Audit-2 reusable diagonal preconditioner identity is invalid".into(),
            ));
        }
        Ok(())
    }
}

/// Caller-declared, durable identity for the frozen W dependency.
///
/// This is receipt provenance only. Runtime reuse additionally requires exact
/// in-process operator identity and never relies on a process-local token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit2FrozenWSemanticIdentity {
    pub schema: String,
    pub sha256: String,
}

impl Audit2FrozenWSemanticIdentity {
    fn validate(&self) -> CoreResult<()> {
        let lowercase_hex = self.sha256.len() == 64
            && self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if self.schema.trim().is_empty() || !lowercase_hex {
            return Err(CoreError::InvalidInput(
                "Audit-2 frozen-W semantic identity requires a schema and lowercase SHA-256".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit2ReusablePreconditionerBinding {
    pub dimension: usize,
    pub h_gamma_bits: u64,
    pub frozen_w_semantic: Audit2FrozenWSemanticIdentity,
    pub preconditioner: Audit2ReusablePreconditionerIdentity,
    pub returned_inverse_diagonal_bits: Vec<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Audit2ReusablePreconditionerCacheSnapshot {
    pub attempts: u64,
    pub setup_attempts: u64,
    pub setup_completed: u64,
    pub setup_failures: u64,
    pub same_binding_reuses: u64,
    pub changed_operator_invalidations: u64,
    pub changed_preconditioner_invalidations: u64,
    pub commits: u64,
    pub rollbacks: u64,
    pub setup_work: WorkCounters,
    pub last_setup_failure: Option<String>,
    pub committed_binding: Option<Audit2ReusablePreconditionerBinding>,
    pub pending_binding: Option<Audit2ReusablePreconditionerBinding>,
}

#[derive(Clone)]
struct Audit2ReusablePreconditionerEntry {
    binding: Audit2ReusablePreconditionerBinding,
    exact_operator_identity: ExactOperatorIdentity,
    preconditioner: Arc<Audit2VerifiedDiagonalPreconditioner>,
}

fn exact_nonidentity_diagonal_bits(
    preconditioner: &dyn Preconditioner,
    dimension: usize,
) -> Option<Vec<u64>> {
    if preconditioner.dimension() != dimension || preconditioner.is_identity() {
        return None;
    }
    let ExactPreconditionerIdentity::Jacobi {
        inverse_diagonal_bits,
    } = preconditioner.exact_identity()?
    else {
        return None;
    };
    let exact_nonidentity = inverse_diagonal_bits.len() == dimension
        && inverse_diagonal_bits
            .iter()
            .all(|bits| f64::from_bits(*bits).is_finite())
        && inverse_diagonal_bits
            .iter()
            .any(|bits| f64::from_bits(*bits) != 1.0);
    exact_nonidentity.then_some(inverse_diagonal_bits)
}

/// Cache-owned immutable numerical map with a guarded provider failure path.
///
/// The provider is never trusted to define the effective map after setup. Its
/// output must still match the frozen exact diagonal bit-for-bit on every
/// successful apply, and caller output is untouched on drift or failure.
struct Audit2VerifiedDiagonalPreconditioner {
    inverse_diagonal_bits: Vec<u64>,
    provider: Arc<dyn Preconditioner>,
}

impl Audit2VerifiedDiagonalPreconditioner {
    fn provider_still_matches(&self) -> bool {
        exact_nonidentity_diagonal_bits(self.provider.as_ref(), self.dimension())
            .is_some_and(|bits| bits == self.inverse_diagonal_bits)
    }
}

impl Preconditioner for Audit2VerifiedDiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.inverse_diagonal_bits.len()
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        if input.len() != self.dimension() || output.len() != input.len() {
            return Err(CoreError::Dimension(
                "Audit-2 verified diagonal preconditioner shape mismatch".into(),
            ));
        }
        let mut provider_output = vec![0.0; output.len()];
        self.provider.apply(input, &mut provider_output)?;
        for (((provider_value, input_value), inverse_bits), index) in provider_output
            .iter()
            .zip(input)
            .zip(&self.inverse_diagonal_bits)
            .zip(0..)
        {
            let expected = f64::from_bits(*inverse_bits) * input_value;
            if provider_value.to_bits() != expected.to_bits() {
                return Err(CoreError::LinearSolve(format!(
                    "Audit-2 reusable preconditioner provider drifted from the verified exact map at index {index}"
                )));
            }
        }
        output.copy_from_slice(&provider_output);
        Ok(())
    }

    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        Some(ExactPreconditionerIdentity::Jacobi {
            inverse_diagonal_bits: self.inverse_diagonal_bits.clone(),
        })
    }
}

#[derive(Default)]
pub struct Audit2ReusablePreconditionerCache {
    committed: Option<Audit2ReusablePreconditionerEntry>,
    pending: Option<Audit2ReusablePreconditionerEntry>,
    snapshot: Audit2ReusablePreconditionerCacheSnapshot,
}

impl Audit2ReusablePreconditionerCache {
    fn setup_failed<T>(&mut self, error: CoreError) -> CoreResult<T> {
        self.pending = None;
        self.snapshot.setup_failures = self.snapshot.setup_failures.saturating_add(1);
        self.snapshot.last_setup_failure = Some(error.to_string());
        Err(error)
    }

    pub fn snapshot(&self) -> Audit2ReusablePreconditionerCacheSnapshot {
        let mut snapshot = self.snapshot.clone();
        snapshot.committed_binding = self.committed.as_ref().map(|entry| entry.binding.clone());
        snapshot.pending_binding = self.pending.as_ref().map(|entry| entry.binding.clone());
        snapshot
    }

    pub fn begin_attempt<F>(
        &mut self,
        context: &StepContext<'_>,
        frozen_w_semantic: Audit2FrozenWSemanticIdentity,
        preconditioner_identity: Audit2ReusablePreconditionerIdentity,
        setup: F,
    ) -> CoreResult<Arc<dyn Preconditioner>>
    where
        F: FnOnce(&StepContext<'_>, &mut WorkCounters) -> CoreResult<Arc<dyn Preconditioner>>,
    {
        if self.pending.is_some() {
            return Err(CoreError::InvalidInput(
                "Audit-2 reusable preconditioner attempt already pending".into(),
            ));
        }
        let dimension = context.shifted.dimension();
        frozen_w_semantic.validate()?;
        preconditioner_identity.validate(dimension)?;
        let exact_operator_identity = context.shifted.exact_identity().ok_or_else(|| {
            CoreError::InvalidInput(
                "Audit-2 reusable preconditioner requires exact frozen-W identity".into(),
            )
        })?;
        self.snapshot.attempts = self.snapshot.attempts.saturating_add(1);
        let binding = Audit2ReusablePreconditionerBinding {
            dimension,
            h_gamma_bits: context.shifted.h_gamma().to_bits(),
            frozen_w_semantic,
            returned_inverse_diagonal_bits: preconditioner_identity
                .expected_inverse_diagonal_bits
                .clone(),
            preconditioner: preconditioner_identity,
        };
        let committed_preconditioner_still_matches = self
            .committed
            .as_ref()
            .is_some_and(|committed| committed.preconditioner.provider_still_matches());

        if let Some(committed) = &self.committed
            && committed.exact_operator_identity == exact_operator_identity
            && committed.binding.frozen_w_semantic == binding.frozen_w_semantic
            && committed.binding.preconditioner == binding.preconditioner
            && committed_preconditioner_still_matches
        {
            self.snapshot.same_binding_reuses = self.snapshot.same_binding_reuses.saturating_add(1);
            self.pending = Some(committed.clone());
            let preconditioner = Arc::clone(&committed.preconditioner);
            let preconditioner: Arc<dyn Preconditioner> = preconditioner;
            return Ok(preconditioner);
        }

        if let Some(committed) = &self.committed {
            if committed.exact_operator_identity != exact_operator_identity
                || committed.binding.frozen_w_semantic != binding.frozen_w_semantic
            {
                self.snapshot.changed_operator_invalidations = self
                    .snapshot
                    .changed_operator_invalidations
                    .saturating_add(1);
            }
            if committed.binding.preconditioner != binding.preconditioner
                || !committed_preconditioner_still_matches
            {
                self.snapshot.changed_preconditioner_invalidations = self
                    .snapshot
                    .changed_preconditioner_invalidations
                    .saturating_add(1);
            }
        }

        self.snapshot.setup_attempts = self.snapshot.setup_attempts.saturating_add(1);
        self.snapshot.last_setup_failure = None;
        let preconditioner = match setup(context, &mut self.snapshot.setup_work) {
            Ok(value) => value,
            Err(error) => return self.setup_failed(error),
        };
        if preconditioner.dimension() != binding.dimension {
            return self.setup_failed(CoreError::Dimension(
                "Audit-2 reusable preconditioner dimension mismatch".into(),
            ));
        }
        let returned_inverse_diagonal_bits =
            match exact_nonidentity_diagonal_bits(preconditioner.as_ref(), binding.dimension) {
                Some(bits) => bits,
                None => {
                    return self.setup_failed(CoreError::InvalidInput(
                        "Audit-2 reusable preconditioner requires an exact nonidentity diagonal map"
                            .into(),
                    ));
                }
            };
        if returned_inverse_diagonal_bits != binding.preconditioner.expected_inverse_diagonal_bits {
            return self.setup_failed(CoreError::InvalidInput(
                "Audit-2 reusable preconditioner returned a map different from its declared identity"
                    .into(),
            ));
        }
        let verified_preconditioner = Arc::new(Audit2VerifiedDiagonalPreconditioner {
            inverse_diagonal_bits: returned_inverse_diagonal_bits,
            provider: preconditioner,
        });
        self.snapshot.setup_completed = self.snapshot.setup_completed.saturating_add(1);
        self.pending = Some(Audit2ReusablePreconditionerEntry {
            binding,
            exact_operator_identity,
            preconditioner: Arc::clone(&verified_preconditioner),
        });
        let preconditioner: Arc<dyn Preconditioner> = verified_preconditioner;
        Ok(preconditioner)
    }

    pub fn commit_attempt(&mut self) -> CoreResult<()> {
        let pending = self.pending.take().ok_or_else(|| {
            CoreError::InvalidInput(
                "Audit-2 reusable preconditioner has no pending attempt to commit".into(),
            )
        })?;
        self.committed = Some(pending);
        self.snapshot.commits = self.snapshot.commits.saturating_add(1);
        Ok(())
    }

    pub fn rollback_attempt(&mut self) -> CoreResult<()> {
        self.pending.take().ok_or_else(|| {
            CoreError::InvalidInput(
                "Audit-2 reusable preconditioner has no pending attempt to roll back".into(),
            )
        })?;
        self.snapshot.rollbacks = self.snapshot.rollbacks.saturating_add(1);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2TransactionalAttemptConfig {
    pub common_w: Audit2MatrixFreeCommonWConfig,
    pub outer_atol: f64,
    pub outer_rtol: f64,
}

impl Audit2TransactionalAttemptConfig {
    fn validate(&self) -> CoreResult<()> {
        self.common_w.validate()?;
        if !self.outer_atol.is_finite()
            || !self.outer_rtol.is_finite()
            || self.outer_atol < 0.0
            || self.outer_rtol < 0.0
            || self.outer_atol + self.outer_rtol <= 0.0
        {
            return Err(CoreError::InvalidInput(
                "Audit-2 transactional outer tolerances are invalid".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2IndependentStepBudget {
    pub identifier: String,
    pub output_atol_l2: f64,
    pub output_rtol: f64,
    pub max_embedded_l2: f64,
    pub max_original_target_residual_l2: f64,
    pub max_original_target_contraction: f64,
}

impl Audit2IndependentStepBudget {
    fn validate(&self) -> CoreResult<()> {
        if self.identifier.trim().is_empty()
            || ![
                self.output_atol_l2,
                self.output_rtol,
                self.max_embedded_l2,
                self.max_original_target_residual_l2,
                self.max_original_target_contraction,
            ]
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            return Err(CoreError::InvalidInput(
                "Audit-2 independent step budget is invalid".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2ExternalOutputReference {
    pub source: String,
    pub state: Vec<f64>,
    pub uncertainty_l2: f64,
}

impl Audit2ExternalOutputReference {
    fn validate(&self, dimension: usize) -> CoreResult<()> {
        if self.source.trim().is_empty()
            || self.state.len() != dimension
            || self.state.iter().any(|value| !value.is_finite())
            || !self.uncertainty_l2.is_finite()
            || self.uncertainty_l2 < 0.0
        {
            return Err(CoreError::InvalidInput(
                "Audit-2 external output reference is invalid".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audit2TransactionalSelection {
    Candidate,
    ProtectedFallback,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2IndependentBudgetReceipt {
    pub identifier: String,
    pub reference_source: String,
    pub output_error_l2: f64,
    pub output_bound_l2: f64,
    pub embedded_l2: f64,
    pub original_target_residual_l2: f64,
    pub original_target_contraction: f64,
    pub output_accepted: bool,
    pub embedded_accepted: bool,
    pub original_target_accepted: bool,
    pub accepted: bool,
}

#[derive(Debug)]
pub struct Audit2TransactionalCandidateReceipt {
    pub correction: Audit2MatrixFreeCorrectionSuccess,
    pub updated_stages: Vec<Vec<f64>>,
    pub step: StepResult,
    pub original_target_residual_before_l2: f64,
    pub original_target_residual_after_l2: f64,
    pub budget: Audit2IndependentBudgetReceipt,
}

#[derive(Debug)]
pub struct Audit2TransactionalAttemptSuccess {
    pub selection: Audit2TransactionalSelection,
    pub committed: bool,
    pub committed_state: Vec<f64>,
    pub selected_step: Option<StepResult>,
    pub candidate: Option<Audit2TransactionalCandidateReceipt>,
    pub candidate_failure: Option<Audit2MatrixFreeCorrectionFailure>,
    pub fallback_step: Option<StepResult>,
    pub cache: Audit2ReusablePreconditionerCacheSnapshot,
    pub work: WorkCounters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audit2TransactionalFailurePhase {
    InputValidation,
    PreconditionerSetup,
    OriginalTargetDiagnostic,
    Fallback,
}

#[derive(Debug)]
pub struct Audit2TransactionalAttemptFailure {
    pub phase: Audit2TransactionalFailurePhase,
    pub message: String,
    pub committed_state: Vec<f64>,
    pub candidate_failure: Option<Audit2MatrixFreeCorrectionFailure>,
    pub cache: Audit2ReusablePreconditionerCacheSnapshot,
    pub work: WorkCounters,
}

#[derive(Debug)]
pub enum Audit2TransactionalAttemptOutcome {
    Completed(Box<Audit2TransactionalAttemptSuccess>),
    Failed(Box<Audit2TransactionalAttemptFailure>),
}

fn rows_l2(rows: &[Vec<f64>]) -> f64 {
    safe_l2(&rows.iter().flatten().copied().collect::<Vec<_>>())
}

fn accumulate_correction_work(
    counters: &mut WorkCounters,
    work: &Audit2MatrixFreeCorrectionWork,
) -> CoreResult<()> {
    let mut correction = work.preparation_counters;
    correction
        .checked_accumulate(work.coupling_counters)
        .ok_or_else(|| {
            CoreError::InvalidInput("Audit-2 correction work counter overflow".into())
        })?;
    if let Some(session) = &work.session {
        correction
            .checked_accumulate(session.counters)
            .ok_or_else(|| {
                CoreError::InvalidInput("Audit-2 session work counter overflow".into())
            })?;
    }
    counters.checked_accumulate(correction).ok_or_else(|| {
        CoreError::InvalidInput("Audit-2 transactional work counter overflow".into())
    })
}

fn updated_stages(trial: &[Vec<f64>], correction: &[Vec<f64>]) -> CoreResult<Vec<Vec<f64>>> {
    if trial.len() != correction.len()
        || trial
            .iter()
            .zip(correction)
            .any(|(left, right)| left.len() != right.len())
    {
        return Err(CoreError::Dimension(
            "Audit-2 transactional update shape mismatch".into(),
        ));
    }
    let updated = trial
        .iter()
        .zip(correction)
        .map(|(stage, delta)| stage.iter().zip(delta).map(|(a, b)| a - b).collect())
        .collect::<Vec<Vec<f64>>>();
    if updated.iter().flatten().all(|value| value.is_finite()) {
        Ok(updated)
    } else {
        Err(CoreError::NonFinite(
            "Audit-2 transactional candidate stages contain NaN/Inf".into(),
        ))
    }
}

fn failure(
    phase: Audit2TransactionalFailurePhase,
    error: impl ToString,
    context: &StepContext<'_>,
    candidate_failure: Option<Audit2MatrixFreeCorrectionFailure>,
    cache: &Audit2ReusablePreconditionerCache,
    before: WorkCounters,
    counters: &WorkCounters,
) -> Audit2TransactionalAttemptOutcome {
    Audit2TransactionalAttemptOutcome::Failed(Box::new(Audit2TransactionalAttemptFailure {
        phase,
        message: error.to_string(),
        committed_state: context.y.clone(),
        candidate_failure,
        cache: cache.snapshot(),
        work: counters.delta(before),
    }))
}

fn run_protected_fallback(
    context: &StepContext<'_>,
    config: &Audit2TransactionalAttemptConfig,
    before: WorkCounters,
    counters: &mut WorkCounters,
) -> CoreResult<StepResult> {
    counters.fallback_steps = counters.fallback_steps.saturating_add(1);
    let fallback_config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: config.common_w.rtol,
        atol: config.common_w.atol,
        restart: config.common_w.restart,
        maxiter: config.common_w.max_arnoldi,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
        ..LinearSolverConfig::default()
    };
    let stages = sequential_stages(context, &fallback_config, None, counters)?.stages;
    finish_step(
        context,
        stages,
        config.outer_atol,
        config.outer_rtol,
        "RODAS5P-audit2-protected-sequential-JF-fallback".into(),
        None,
        true,
        None,
        before,
        counters,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_audit2_reusable_preconditioner_transactional_attempt<F>(
    context: &StepContext<'_>,
    trial_stages: &[Vec<f64>],
    config: &Audit2TransactionalAttemptConfig,
    budget: &Audit2IndependentStepBudget,
    reference: &Audit2ExternalOutputReference,
    cache: &mut Audit2ReusablePreconditionerCache,
    frozen_w_semantic: Audit2FrozenWSemanticIdentity,
    preconditioner_identity: Audit2ReusablePreconditionerIdentity,
    setup: F,
    counters: &mut WorkCounters,
) -> Audit2TransactionalAttemptOutcome
where
    F: FnOnce(&StepContext<'_>, &mut WorkCounters) -> CoreResult<Arc<dyn Preconditioner>>,
{
    let before = *counters;
    if let Err(error) = config
        .validate()
        .and_then(|_| budget.validate())
        .and_then(|_| reference.validate(context.problem.dimension))
    {
        return failure(
            Audit2TransactionalFailurePhase::InputValidation,
            error,
            context,
            None,
            cache,
            before,
            counters,
        );
    }

    let initial_residual =
        match StructuredBlockSystem::new(context).target_residual(trial_stages, counters) {
            Ok(value) => value,
            Err(error) => {
                return failure(
                    Audit2TransactionalFailurePhase::OriginalTargetDiagnostic,
                    error,
                    context,
                    None,
                    cache,
                    before,
                    counters,
                );
            }
        };
    let initial_residual_l2 = rows_l2(&initial_residual);
    let setup_work_before = cache.snapshot().setup_work;
    let preconditioner_result =
        cache.begin_attempt(context, frozen_w_semantic, preconditioner_identity, setup);
    let setup_work_after = cache.snapshot().setup_work;
    let setup_work = match setup_work_after.checked_delta(setup_work_before) {
        Some(value) => value,
        None => {
            if cache.snapshot().pending_binding.is_some() {
                let _ = cache.rollback_attempt();
            }
            return failure(
                Audit2TransactionalFailurePhase::PreconditionerSetup,
                "Audit-2 reusable preconditioner setup work is nonmonotone",
                context,
                None,
                cache,
                before,
                counters,
            );
        }
    };
    if counters.checked_accumulate(setup_work).is_none() {
        if cache.snapshot().pending_binding.is_some() {
            let _ = cache.rollback_attempt();
        }
        return failure(
            Audit2TransactionalFailurePhase::PreconditionerSetup,
            "Audit-2 reusable preconditioner setup work overflow",
            context,
            None,
            cache,
            before,
            counters,
        );
    }
    let preconditioner = match preconditioner_result {
        Ok(value) => value,
        Err(error) => {
            return failure(
                Audit2TransactionalFailurePhase::PreconditionerSetup,
                error,
                context,
                None,
                cache,
                before,
                counters,
            );
        }
    };

    let mut candidate_receipt = None;
    let mut candidate_failure = None;
    let correction = run_audit2_matrix_free_common_w_correction_with_preconditioner(
        context,
        trial_stages,
        config.common_w,
        preconditioner,
    );
    let correction_work = match &correction {
        Audit2MatrixFreeCorrectionOutcome::Completed(value) => &value.work,
        Audit2MatrixFreeCorrectionOutcome::Failed(value) => &value.work,
    };
    if let Err(error) = accumulate_correction_work(counters, correction_work) {
        let _ = cache.rollback_attempt();
        return failure(
            Audit2TransactionalFailurePhase::OriginalTargetDiagnostic,
            error,
            context,
            None,
            cache,
            before,
            counters,
        );
    }
    if let Audit2MatrixFreeCorrectionOutcome::Completed(correction) = correction {
        let correction = *correction;
        let updated = match updated_stages(trial_stages, &correction.correction) {
            Ok(value) => value,
            Err(error) => {
                let _ = cache.rollback_attempt();
                return failure(
                    Audit2TransactionalFailurePhase::OriginalTargetDiagnostic,
                    error,
                    context,
                    None,
                    cache,
                    before,
                    counters,
                );
            }
        };
        let final_residual =
            match StructuredBlockSystem::new(context).target_residual(&updated, counters) {
                Ok(value) => value,
                Err(error) => {
                    let _ = cache.rollback_attempt();
                    return failure(
                        Audit2TransactionalFailurePhase::OriginalTargetDiagnostic,
                        error,
                        context,
                        None,
                        cache,
                        before,
                        counters,
                    );
                }
            };
        let final_residual_l2 = rows_l2(&final_residual);
        let contraction = if initial_residual_l2 == 0.0 {
            if final_residual_l2 == 0.0 {
                0.0
            } else {
                f64::INFINITY
            }
        } else {
            final_residual_l2 / initial_residual_l2
        };
        let step = match finish_step(
            context,
            updated.clone(),
            config.outer_atol,
            config.outer_rtol,
            "RODAS5P-audit2-reusable-preconditioner-candidate".into(),
            None,
            false,
            None,
            before,
            counters,
        ) {
            Ok(value) => value,
            Err(error) => {
                let _ = cache.rollback_attempt();
                return failure(
                    Audit2TransactionalFailurePhase::OriginalTargetDiagnostic,
                    error,
                    context,
                    None,
                    cache,
                    before,
                    counters,
                );
            }
        };
        let output_error_l2 = safe_l2(
            &step
                .y_new
                .iter()
                .zip(&reference.state)
                .map(|(actual, expected)| actual - expected)
                .collect::<Vec<_>>(),
        );
        let output_bound_l2 = budget.output_atol_l2
            + budget.output_rtol * safe_l2(&reference.state)
            + reference.uncertainty_l2;
        let embedded_l2 = safe_l2(&step.error_vector);
        let output_accepted = output_error_l2.is_finite()
            && output_bound_l2.is_finite()
            && output_error_l2 <= output_bound_l2;
        let embedded_accepted = embedded_l2.is_finite() && embedded_l2 <= budget.max_embedded_l2;
        let original_target_accepted = final_residual_l2.is_finite()
            && contraction.is_finite()
            && final_residual_l2 <= budget.max_original_target_residual_l2
            && contraction <= budget.max_original_target_contraction;
        let budget_receipt = Audit2IndependentBudgetReceipt {
            identifier: budget.identifier.clone(),
            reference_source: reference.source.clone(),
            output_error_l2,
            output_bound_l2,
            embedded_l2,
            original_target_residual_l2: final_residual_l2,
            original_target_contraction: contraction,
            output_accepted,
            embedded_accepted,
            original_target_accepted,
            accepted: output_accepted && embedded_accepted && original_target_accepted,
        };
        candidate_receipt = Some(Audit2TransactionalCandidateReceipt {
            correction,
            updated_stages: updated,
            step,
            original_target_residual_before_l2: initial_residual_l2,
            original_target_residual_after_l2: final_residual_l2,
            budget: budget_receipt,
        });
    } else if let Audit2MatrixFreeCorrectionOutcome::Failed(value) = correction {
        candidate_failure = Some(*value);
    }

    if candidate_receipt
        .as_ref()
        .is_some_and(|candidate| candidate.step.accepted && candidate.budget.accepted)
    {
        if let Err(error) = cache.commit_attempt() {
            return failure(
                Audit2TransactionalFailurePhase::PreconditionerSetup,
                error,
                context,
                candidate_failure,
                cache,
                before,
                counters,
            );
        }
        counters.accepted_steps = counters.accepted_steps.saturating_add(1);
        let mut selected_step = candidate_receipt
            .as_ref()
            .expect("checked candidate")
            .step
            .clone();
        selected_step.counters = counters.delta(before);
        return Audit2TransactionalAttemptOutcome::Completed(Box::new(
            Audit2TransactionalAttemptSuccess {
                selection: Audit2TransactionalSelection::Candidate,
                committed: true,
                committed_state: selected_step.y_new.clone(),
                selected_step: Some(selected_step),
                candidate: candidate_receipt,
                candidate_failure,
                fallback_step: None,
                cache: cache.snapshot(),
                work: counters.delta(before),
            },
        ));
    }

    if let Err(error) = cache.rollback_attempt() {
        return failure(
            Audit2TransactionalFailurePhase::PreconditionerSetup,
            error,
            context,
            candidate_failure,
            cache,
            before,
            counters,
        );
    }
    let mut fallback = match run_protected_fallback(context, config, before, counters) {
        Ok(value) => value,
        Err(error) => {
            return failure(
                Audit2TransactionalFailurePhase::Fallback,
                error,
                context,
                candidate_failure,
                cache,
                before,
                counters,
            );
        }
    };
    let (selection, committed, committed_state) = if fallback.accepted {
        counters.accepted_steps = counters.accepted_steps.saturating_add(1);
        (
            Audit2TransactionalSelection::ProtectedFallback,
            true,
            fallback.y_new.clone(),
        )
    } else {
        counters.rejected_steps = counters.rejected_steps.saturating_add(1);
        (
            Audit2TransactionalSelection::Rejected,
            false,
            context.y.clone(),
        )
    };
    fallback.counters = counters.delta(before);
    let selected_step = committed.then(|| fallback.clone());
    Audit2TransactionalAttemptOutcome::Completed(Box::new(Audit2TransactionalAttemptSuccess {
        selection,
        committed,
        committed_state,
        selected_step,
        candidate: candidate_receipt,
        candidate_failure,
        fallback_step: Some(fallback),
        cache: cache.snapshot(),
        work: counters.delta(before),
    }))
}
