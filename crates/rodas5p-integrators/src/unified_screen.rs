use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use serde::Serialize;

use crate::CandidateFamily;

const UNIFIED_OUTPUT_BUDGET: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnifiedCandidateOutcome {
    Completed,
    CompletedWithFallback,
    Rejected,
    Uncertified,
    NumericalFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReferenceCertificateSource {
    C3RefinedRoot,
    C0ProtectedOracleFallback,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnifiedCandidateRow {
    pub candidate_id: String,
    pub family: CandidateFamily,
    pub case_id: String,
    pub outcome: UnifiedCandidateOutcome,
    pub scientifically_certified: bool,
    pub reference_certificate_source: ReferenceCertificateSource,
    pub reference_fallback_used: bool,
    pub certificate_failure: Option<String>,
    pub used_fallback: bool,
    pub embedded_error: Option<f64>,
    pub oracle_output_wrms: Option<f64>,
    pub oracle_stage_l2: Option<f64>,
    pub first_output_wrms: Option<f64>,
    pub second_output_wrms: Option<f64>,
    pub second_output_ratio: Option<f64>,
    pub second_residual_ratio: Option<f64>,
    pub second_contraction_evidence: Option<bool>,
    pub refined_root_converged: Option<bool>,
    pub refined_root_termination: Option<String>,
    pub refined_root_output_wrms: Option<f64>,
    pub refined_root_relative_residual: Option<f64>,
    pub output_budget: f64,
    pub c3_output_budget_pass: Option<bool>,
    pub oracle_output_budget_pass: Option<bool>,
    pub c3_false_accept: bool,
    pub first_correction_false_accept: bool,
    pub candidate_counters: WorkCounters,
    pub certificate_counters: WorkCounters,
    pub batch_depth: u64,
    pub batch_vectors: u64,
    pub compute_seconds: f64,
    pub certificate_seconds: f64,
    pub decision_reason: Option<String>,
    pub failure: Option<String>,
}

impl UnifiedCandidateRow {
    pub fn sort_key(&self) -> (&str, &str) {
        (&self.case_id, &self.candidate_id)
    }

    pub fn validate(&self) -> CoreResult<()> {
        if self.candidate_id.is_empty() || self.case_id.is_empty() {
            return Err(CoreError::InvalidInput(
                "unified candidate row identifiers must be nonempty".into(),
            ));
        }
        for (name, value) in [
            ("embedded error", self.embedded_error),
            ("oracle output WRMS", self.oracle_output_wrms),
            ("oracle stage L2", self.oracle_stage_l2),
            ("first output WRMS", self.first_output_wrms),
            ("second output WRMS", self.second_output_wrms),
            ("second output ratio", self.second_output_ratio),
            ("second residual ratio", self.second_residual_ratio),
            ("refined output WRMS", self.refined_root_output_wrms),
            (
                "refined relative residual",
                self.refined_root_relative_residual,
            ),
        ] {
            if value.is_some_and(|number| !number.is_finite() || number < 0.0) {
                return Err(CoreError::NonFinite(format!(
                    "unified candidate {name} is invalid"
                )));
            }
        }
        if !self.output_budget.is_finite() || self.output_budget < 0.0 {
            return Err(CoreError::NonFinite(
                "unified candidate output budget is invalid".into(),
            ));
        }
        for (name, value) in [
            ("compute seconds", self.compute_seconds),
            ("certificate seconds", self.certificate_seconds),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(CoreError::NonFinite(format!(
                    "unified candidate {name} is invalid"
                )));
            }
        }
        if (self.outcome == UnifiedCandidateOutcome::CompletedWithFallback && !self.used_fallback)
            || (self.outcome == UnifiedCandidateOutcome::Completed && self.used_fallback)
        {
            return Err(CoreError::InvalidInput(
                "unified candidate fallback flag and completed outcome disagree".into(),
            ));
        }
        if self
            .decision_reason
            .as_ref()
            .is_some_and(|reason| reason.is_empty())
        {
            return Err(CoreError::InvalidInput(
                "unified candidate decision reason must be nonempty".into(),
            ));
        }
        if self
            .refined_root_termination
            .as_ref()
            .is_some_and(|termination| termination.is_empty())
        {
            return Err(CoreError::InvalidInput(
                "refined-root termination must be nonempty".into(),
            ));
        }
        if self.refined_root_converged.is_some() && self.refined_root_termination.is_none() {
            return Err(CoreError::InvalidInput(
                "refined-root status requires a termination reason".into(),
            ));
        }
        if self.reference_fallback_used
            != (self.reference_certificate_source
                == ReferenceCertificateSource::C0ProtectedOracleFallback)
        {
            return Err(CoreError::InvalidInput(
                "reference-fallback flag and certificate source disagree".into(),
            ));
        }
        if self.reference_certificate_source == ReferenceCertificateSource::C3RefinedRoot
            && self.refined_root_converged != Some(true)
        {
            return Err(CoreError::InvalidInput(
                "C3 reference source requires a converged refined root".into(),
            ));
        }
        if self.reference_certificate_source == ReferenceCertificateSource::Unavailable
            && self.scientifically_certified
        {
            return Err(CoreError::InvalidInput(
                "unavailable reference cannot scientifically certify a row".into(),
            ));
        }
        if self.outcome == UnifiedCandidateOutcome::Uncertified && self.decision_reason.is_none() {
            return Err(CoreError::InvalidInput(
                "uncertified candidate row requires a decision reason".into(),
            ));
        }
        if self.failure.is_some()
            && matches!(
                self.outcome,
                UnifiedCandidateOutcome::Completed | UnifiedCandidateOutcome::CompletedWithFallback
            )
        {
            return Err(CoreError::InvalidInput(
                "completed unified candidate row cannot contain a failure".into(),
            ));
        }
        if self.scientifically_certified {
            if self.failure.is_some()
                || self.reference_certificate_source == ReferenceCertificateSource::Unavailable
                || self.oracle_output_wrms.is_none()
            {
                return Err(CoreError::InvalidInput(
                    "scientifically certified row lacks an available reference certificate".into(),
                ));
            }
            if matches!(
                self.outcome,
                UnifiedCandidateOutcome::NumericalFailure | UnifiedCandidateOutcome::Uncertified
            ) {
                return Err(CoreError::InvalidInput(
                    "failed or uncertified row cannot be scientifically certified".into(),
                ));
            }
        }
        if let Some(c3_pass) = self.c3_output_budget_pass
            && c3_pass
                != self
                    .refined_root_output_wrms
                    .is_some_and(|value| value <= self.output_budget)
        {
            return Err(CoreError::InvalidInput(
                "C3 output-budget decision is inconsistent".into(),
            ));
        }
        if let Some(oracle_pass) = self.oracle_output_budget_pass
            && oracle_pass
                != self
                    .oracle_output_wrms
                    .is_some_and(|value| value <= self.output_budget)
        {
            return Err(CoreError::InvalidInput(
                "oracle output-budget decision is inconsistent".into(),
            ));
        }
        if self.c3_false_accept
            != (self.c3_output_budget_pass == Some(true)
                && self.oracle_output_budget_pass == Some(false))
        {
            return Err(CoreError::InvalidInput(
                "C3 false-accept flag is inconsistent".into(),
            ));
        }
        if self.c3_output_budget_pass == Some(false)
            && self.outcome != UnifiedCandidateOutcome::Rejected
        {
            return Err(CoreError::InvalidInput(
                "C3-rejected row must have a rejected outcome".into(),
            ));
        }
        if self.reference_certificate_source
            == ReferenceCertificateSource::C0ProtectedOracleFallback
        {
            let oracle_pass = self.oracle_output_budget_pass.ok_or_else(|| {
                CoreError::InvalidInput(
                    "C0 protected-oracle fallback requires an oracle budget decision".into(),
                )
            })?;
            let completed = matches!(
                self.outcome,
                UnifiedCandidateOutcome::Completed | UnifiedCandidateOutcome::CompletedWithFallback
            );
            if completed && !oracle_pass {
                return Err(CoreError::InvalidInput(
                    "C0 protected-oracle fallback completed outside the oracle budget".into(),
                ));
            }
            if !oracle_pass && self.outcome != UnifiedCandidateOutcome::Rejected {
                return Err(CoreError::InvalidInput(
                    "C0 protected-oracle fallback over budget must be rejected".into(),
                ));
            }
        }
        if self.first_correction_false_accept
            && !(self
                .first_output_wrms
                .is_some_and(|value| value <= self.output_budget)
                && self
                    .oracle_output_wrms
                    .is_some_and(|value| value > self.output_budget))
        {
            return Err(CoreError::InvalidInput(
                "first-correction false-accept flag is inconsistent".into(),
            ));
        }
        Ok(())
    }
}

use std::time::Instant;

use rodas5p_core::{LinearMethod, LinearSolverConfig, error_scale, safe_l2, wrms};

use crate::{
    BlockMethod, BlockPreconditioner, CandidateCatalog, CandidateExecution, CandidateSpec,
    HomotopyPathConfig, HomotopyPredictor, OdeProblem, ParallelExecution, PredictorKind,
    RefinedRootConfig, SabrConfig, StageHistory, StructuredBlockSystem, build_step_context,
    certify_nonlinear_target, certify_second_correction, constant_affine_mass_problem,
    manufactured_mass_nonlinear_problem, manufactured_vector_problem, prothero_robinson_problem,
    refine_target_root, run_fixed_homotopy_path, sabr_step, sequential_step,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnifiedScreenProfile {
    Smoke,
    Canonical,
}

impl UnifiedScreenProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Canonical => "canonical",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnifiedCaseDescriptor {
    pub case_id: String,
    pub family: String,
    pub dimension: usize,
    pub t: f64,
    pub h: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UnifiedNonlinearSummary {
    pub cases: usize,
    pub rows: usize,
    pub completed: usize,
    pub completed_with_fallback: usize,
    pub rejected: usize,
    pub uncertified: usize,
    pub reference_fallbacks: usize,
    pub failures: usize,
    pub first_correction_false_accepts: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnifiedNonlinearScreen {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub threads: usize,
    pub atol: f64,
    pub rtol: f64,
    pub first_correction_budget: f64,
    pub output_budget: f64,
    pub cases: Vec<UnifiedCaseDescriptor>,
    pub rows: Vec<UnifiedCandidateRow>,
    pub summary: UnifiedNonlinearSummary,
    pub compute_seconds: f64,
}

#[derive(Clone)]
struct RuntimeUnifiedCase {
    descriptor: UnifiedCaseDescriptor,
    problem: OdeProblem,
    y0: Vec<f64>,
}

struct CandidateExecutionResult {
    stages: Vec<Vec<f64>>,
    output: Vec<f64>,
    embedded_error: f64,
    used_fallback: bool,
    accepted: bool,
    counters: WorkCounters,
    batch_depth: u64,
    batch_vectors: u64,
    compute_seconds: f64,
}

struct CertificationBundle {
    first: Option<crate::NonlinearOutputCertificate>,
    second: Option<crate::CorrectionDiagnostic>,
    refined: Option<crate::RefinedRootCertificate>,
    counters: WorkCounters,
    seconds: f64,
    failure: Option<String>,
}

fn build_unified_cases(profile: UnifiedScreenProfile) -> CoreResult<Vec<RuntimeUnifiedCase>> {
    let mut cases = Vec::new();
    let (problem, y0, _, _) = constant_affine_mass_problem();
    cases.push(RuntimeUnifiedCase {
        descriptor: UnifiedCaseDescriptor {
            case_id: "affine-mass-t0.2-h0.03".into(),
            family: "affine-mass".into(),
            dimension: problem.dimension,
            t: 0.2,
            h: 0.03,
        },
        problem,
        y0,
    });

    let (problem, y0) = prothero_robinson_problem(-1_000.0, 100.0, 0.0);
    cases.push(RuntimeUnifiedCase {
        descriptor: UnifiedCaseDescriptor {
            case_id: "pr-l1000-m100-h0.005".into(),
            family: "prothero-robinson".into(),
            dimension: problem.dimension,
            t: 0.0,
            h: 0.005,
        },
        problem,
        y0,
    });

    let (problem, y0) = manufactured_vector_problem(4, 1_000.0, 100.0, 0.2, 0.0)?;
    cases.push(RuntimeUnifiedCase {
        descriptor: UnifiedCaseDescriptor {
            case_id: "mv-s1000-m100-eta0.2-h0.002".into(),
            family: "manufactured-vector".into(),
            dimension: problem.dimension,
            t: 0.0,
            h: 0.002,
        },
        problem,
        y0,
    });

    if profile == UnifiedScreenProfile::Canonical {
        let (problem, y0) = manufactured_vector_problem(4, 10_000.0, 1_000.0, 0.9, 0.0)?;
        cases.push(RuntimeUnifiedCase {
            descriptor: UnifiedCaseDescriptor {
                case_id: "mv-s10000-m1000-eta0.9-h0.001".into(),
                family: "manufactured-vector".into(),
                dimension: problem.dimension,
                t: 0.0,
                h: 0.001,
            },
            problem,
            y0,
        });
        let (problem, y0, _, _) = manufactured_mass_nonlinear_problem(10_000.0, 1_000.0, 0.9, 0.0)?;
        cases.push(RuntimeUnifiedCase {
            descriptor: UnifiedCaseDescriptor {
                case_id: "mm-s10000-m1000-eta0.9-h0.001".into(),
                family: "manufactured-mass".into(),
                dimension: problem.dimension,
                t: 0.0,
                h: 0.001,
            },
            problem,
            y0,
        });
        let (problem, y0) = prothero_robinson_problem(-1_000_000.0, 1_000.0, 0.0);
        cases.push(RuntimeUnifiedCase {
            descriptor: UnifiedCaseDescriptor {
                case_id: "pr-l1000000-m1000-h0.001".into(),
                family: "prothero-robinson".into(),
                dimension: problem.dimension,
                t: 0.0,
                h: 0.001,
            },
            problem,
            y0,
        });
    }
    Ok(cases)
}

fn nonlinear_candidates(profile: UnifiedScreenProfile) -> CoreResult<Vec<CandidateSpec>> {
    let catalog = CandidateCatalog::research_default()?;
    Ok(catalog
        .executable()
        .filter(|candidate| match candidate.execution() {
            CandidateExecution::Sequential { .. } => true,
            CandidateExecution::Sabr { .. } => true,
            CandidateExecution::Homotopy {
                theta,
                q,
                path_rounds,
                predictor,
                corrections_per_point,
            } => {
                profile == UnifiedScreenProfile::Canonical
                    || ([0.0, 1.0].contains(theta)
                        && [0, 2, 7].contains(q)
                        && *path_rounds == 2
                        && matches!(predictor, crate::HomotopyPredictorVariant::Euler)
                        && [0, 1].contains(corrections_per_point))
            }
            CandidateExecution::Bdf { .. } | CandidateExecution::RadauIrk { .. } => false,
            CandidateExecution::Deferred => false,
        })
        .cloned()
        .collect())
}

fn output_and_embedded_error(
    context: &crate::StepContext<'_>,
    stages: &[Vec<f64>],
    atol: f64,
    rtol: f64,
) -> CoreResult<(Vec<f64>, f64)> {
    if stages.len() != context.coeffs.stages()
        || stages
            .iter()
            .any(|stage| stage.len() != context.problem.dimension)
    {
        return Err(CoreError::Dimension(
            "unified candidate stage shape mismatch".into(),
        ));
    }
    let mut output = context.y.clone();
    let mut error = vec![0.0; context.problem.dimension];
    for (stage_index, stage) in stages.iter().enumerate() {
        for component in 0..context.problem.dimension {
            output[component] += context.coeffs.b[stage_index] * stage[component];
            error[component] += context.coeffs.btilde[stage_index] * stage[component];
        }
    }
    let scale = error_scale(&context.y, &output, &[atol], rtol)?;
    Ok((output, wrms(&error, &scale)?))
}

fn execute_candidate(
    case: &RuntimeUnifiedCase,
    candidate: &CandidateSpec,
    atol: f64,
    rtol: f64,
) -> CoreResult<CandidateExecutionResult> {
    let start = Instant::now();
    let mut counters = WorkCounters::default();
    match candidate.execution() {
        CandidateExecution::Sequential { linear_method, .. } => {
            let config = LinearSolverConfig {
                method: *linear_method,
                ..LinearSolverConfig::default()
            };
            let step = sequential_step(
                &case.problem,
                case.descriptor.t,
                &case.y0,
                case.descriptor.h,
                &config,
                None,
                atol,
                rtol,
                true,
                &mut counters,
            )?;
            Ok(CandidateExecutionResult {
                stages: step.stages,
                output: step.y_new,
                embedded_error: step.error_norm,
                used_fallback: false,
                accepted: step.accepted,
                counters,
                batch_depth: 8,
                batch_vectors: 8,
                compute_seconds: start.elapsed().as_secs_f64(),
            })
        }
        CandidateExecution::Sabr {
            block_method,
            predictor,
        } => {
            let block_method = match block_method {
                crate::SabrBlockVariant::Forward => BlockMethod::Forward,
                crate::SabrBlockVariant::Explicit => BlockMethod::Explicit,
                crate::SabrBlockVariant::Nilpotent => BlockMethod::Nilpotent,
                crate::SabrBlockVariant::Gmres => BlockMethod::Gmres,
            };
            let predictor = match predictor {
                crate::SabrPredictorVariant::Zero => PredictorKind::Zero,
                crate::SabrPredictorVariant::ScaledLast => PredictorKind::ScaledLast,
                crate::SabrPredictorVariant::LinearHistory => PredictorKind::LinearHistory,
            };
            let config = SabrConfig {
                block_method,
                predictor,
                block_preconditioner: BlockPreconditioner::Direct,
                ..SabrConfig::default()
            };
            let fallback = LinearSolverConfig {
                method: LinearMethod::Direct,
                ..LinearSolverConfig::default()
            };
            let mut history = StageHistory::default();
            let step = sabr_step(
                &case.problem,
                case.descriptor.t,
                &case.y0,
                case.descriptor.h,
                &config,
                Some(&fallback),
                &mut history,
                None,
                atol,
                rtol,
                true,
                &mut counters,
            )?;
            let fast_rounds = step
                .certificate
                .as_ref()
                .map_or(0_u64, |certificate| certificate.iterations as u64);
            Ok(CandidateExecutionResult {
                stages: step.stages,
                output: step.y_new,
                embedded_error: step.error_norm,
                used_fallback: step.used_fallback,
                accepted: step.accepted,
                counters,
                batch_depth: fast_rounds + if step.used_fallback { 8 } else { 0 },
                batch_vectors: counters.block_linear_solves.saturating_mul(8)
                    + if step.used_fallback { 8 } else { 0 },
                compute_seconds: start.elapsed().as_secs_f64(),
            })
        }
        CandidateExecution::Homotopy {
            theta,
            q,
            path_rounds,
            predictor,
            corrections_per_point,
        } => {
            let context = build_step_context(
                &case.problem,
                case.descriptor.t,
                &case.y0,
                case.descriptor.h,
                &mut counters,
            )?;
            let block = StructuredBlockSystem::new(&context);
            let predictor = match predictor {
                crate::HomotopyPredictorVariant::Euler => HomotopyPredictor::Euler,
                crate::HomotopyPredictorVariant::AdamsBashforth2 => {
                    HomotopyPredictor::AdamsBashforth2
                }
            };
            let config = HomotopyPathConfig::new(
                *theta,
                *q,
                *path_rounds,
                predictor,
                *corrections_per_point,
            )?;
            let path = run_fixed_homotopy_path(&block, &config, &mut counters)?;
            let (output, embedded_error) =
                output_and_embedded_error(&context, &path.stages, atol, rtol)?;
            Ok(CandidateExecutionResult {
                stages: path.stages,
                output,
                embedded_error,
                used_fallback: false,
                accepted: true,
                counters,
                batch_depth: path.work.w_solve_batches,
                batch_vectors: path.work.w_solve_vectors,
                compute_seconds: start.elapsed().as_secs_f64(),
            })
        }
        CandidateExecution::Bdf { .. } | CandidateExecution::RadauIrk { .. } => {
            Err(CoreError::InvalidInput(
                "complete-integrator candidate must use the native integrator gate".into(),
            ))
        }
        CandidateExecution::Deferred => Err(CoreError::InvalidInput(
            "deferred candidate cannot be executed".into(),
        )),
    }
}

fn certify_candidate(
    case: &RuntimeUnifiedCase,
    stages: &[Vec<f64>],
    atol: f64,
    rtol: f64,
) -> CertificationBundle {
    let start = Instant::now();
    let mut counters = WorkCounters::default();
    let result = (|| -> CoreResult<_> {
        let context = build_step_context(
            &case.problem,
            case.descriptor.t,
            &case.y0,
            case.descriptor.h,
            &mut counters,
        )?;
        let block = StructuredBlockSystem::new(&context);
        let first = certify_nonlinear_target(&block, stages, atol, rtol, &mut counters)?;
        let second = certify_second_correction(&block, stages, atol, rtol, true, &mut counters)?;
        let refined = refine_target_root(
            &block,
            stages,
            atol,
            rtol,
            &RefinedRootConfig::default(),
            &mut counters,
        )?;
        Ok((first, second, refined))
    })();
    match result {
        Ok((first, second, refined)) => CertificationBundle {
            first: Some(first),
            second: Some(second),
            refined: Some(refined),
            counters,
            seconds: start.elapsed().as_secs_f64(),
            failure: None,
        },
        Err(error) => CertificationBundle {
            first: None,
            second: None,
            refined: None,
            counters,
            seconds: start.elapsed().as_secs_f64(),
            failure: Some(error.to_string()),
        },
    }
}

fn oracle_metrics(
    case: &RuntimeUnifiedCase,
    oracle_stages: &[Vec<f64>],
    oracle_output: &[f64],
    candidate_stages: &[Vec<f64>],
    candidate_output: &[f64],
    atol: f64,
    rtol: f64,
) -> CoreResult<(f64, f64)> {
    let scale = error_scale(&case.y0, oracle_output, &[atol], rtol)?;
    let output_difference: Vec<f64> = candidate_output
        .iter()
        .zip(oracle_output)
        .map(|(candidate, oracle)| candidate - oracle)
        .collect();
    let stage_difference: Vec<f64> = candidate_stages
        .iter()
        .flatten()
        .zip(oracle_stages.iter().flatten())
        .map(|(candidate, oracle)| candidate - oracle)
        .collect();
    Ok((
        wrms(&output_difference, &scale)?,
        safe_l2(&stage_difference),
    ))
}

fn run_unified_case(
    case: &RuntimeUnifiedCase,
    candidates: &[CandidateSpec],
    atol: f64,
    rtol: f64,
) -> CoreResult<Vec<UnifiedCandidateRow>> {
    let oracle_spec = candidates
        .iter()
        .find(|candidate| candidate.id() == "sequential-direct-off")
        .ok_or_else(|| CoreError::InvalidInput("protected oracle candidate missing".into()))?;
    let oracle = execute_candidate(case, oracle_spec, atol, rtol)?;
    let mut rows = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let execution = execute_candidate(case, candidate, atol, rtol);
        let row = match execution {
            Ok(result) => {
                let (oracle_output_wrms, oracle_stage_l2) = oracle_metrics(
                    case,
                    &oracle.stages,
                    &oracle.output,
                    &result.stages,
                    &result.output,
                    atol,
                    rtol,
                )?;
                let certification = certify_candidate(case, &result.stages, atol, rtol);
                let first = certification.first.as_ref();
                let second = certification.second.as_ref();
                let refined = certification.refined.as_ref();
                let refined_converged = refined.map(|value| value.converged);
                let c3_output_budget_pass = refined
                    .filter(|value| value.converged)
                    .map(|value| value.candidate_output_wrms <= UNIFIED_OUTPUT_BUDGET);
                let oracle_output_budget_pass = Some(oracle_output_wrms <= UNIFIED_OUTPUT_BUDGET);
                let c3_false_accept =
                    c3_output_budget_pass == Some(true) && oracle_output_budget_pass == Some(false);
                let reference_certificate_source =
                    if refined_converged == Some(true) && certification.failure.is_none() {
                        ReferenceCertificateSource::C3RefinedRoot
                    } else {
                        ReferenceCertificateSource::C0ProtectedOracleFallback
                    };
                let reference_fallback_used = reference_certificate_source
                    == ReferenceCertificateSource::C0ProtectedOracleFallback;
                let scientifically_certified = true;
                let reference_over_budget = c3_output_budget_pass == Some(false)
                    || (reference_fallback_used && oracle_output_budget_pass == Some(false));
                let outcome = if !result.accepted || reference_over_budget {
                    UnifiedCandidateOutcome::Rejected
                } else if result.used_fallback {
                    UnifiedCandidateOutcome::CompletedWithFallback
                } else {
                    UnifiedCandidateOutcome::Completed
                };
                UnifiedCandidateRow {
                    candidate_id: candidate.id().to_string(),
                    family: candidate.family(),
                    case_id: case.descriptor.case_id.clone(),
                    outcome,
                    scientifically_certified,
                    reference_certificate_source,
                    reference_fallback_used,
                    certificate_failure: certification.failure.clone(),
                    used_fallback: result.used_fallback,
                    embedded_error: Some(result.embedded_error),
                    oracle_output_wrms: Some(oracle_output_wrms),
                    oracle_stage_l2: Some(oracle_stage_l2),
                    first_output_wrms: first.map(|value| value.output_wrms),
                    second_output_wrms: second.map(|value| value.second_output_wrms),
                    second_output_ratio: second.map(|value| value.output_ratio),
                    second_residual_ratio: second.map(|value| value.residual_ratio),
                    second_contraction_evidence: second.map(|value| value.contraction_evidence),
                    refined_root_converged: refined_converged,
                    refined_root_termination: refined.map(|value| value.termination.clone()),
                    refined_root_output_wrms: refined.map(|value| value.candidate_output_wrms),
                    refined_root_relative_residual: refined.map(|value| value.relative_residual),
                    output_budget: UNIFIED_OUTPUT_BUDGET,
                    c3_output_budget_pass,
                    oracle_output_budget_pass,
                    c3_false_accept,
                    first_correction_false_accept: first.is_some_and(|value| {
                        value.output_wrms <= UNIFIED_OUTPUT_BUDGET
                            && oracle_output_wrms > UNIFIED_OUTPUT_BUDGET
                    }),
                    candidate_counters: result.counters,
                    certificate_counters: certification.counters,
                    batch_depth: result.batch_depth,
                    batch_vectors: result.batch_vectors,
                    compute_seconds: result.compute_seconds,
                    certificate_seconds: certification.seconds,
                    decision_reason: match outcome {
                        UnifiedCandidateOutcome::Rejected => Some(
                            if c3_output_budget_pass == Some(false) {
                                "C3 refined-root output budget exceeded"
                            } else if reference_fallback_used
                                && oracle_output_budget_pass == Some(false)
                            {
                                "C3 unavailable; C0 protected-oracle output budget exceeded"
                            } else {
                                "candidate step rejected"
                            }
                            .into(),
                        ),
                        UnifiedCandidateOutcome::Completed
                        | UnifiedCandidateOutcome::CompletedWithFallback
                            if reference_fallback_used =>
                        {
                            Some(
                                "C3 unavailable; C0 protected oracle certified research row".into(),
                            )
                        }
                        _ => None,
                    },
                    failure: None,
                }
            }
            Err(error) => UnifiedCandidateRow {
                candidate_id: candidate.id().to_string(),
                family: candidate.family(),
                case_id: case.descriptor.case_id.clone(),
                outcome: UnifiedCandidateOutcome::NumericalFailure,
                scientifically_certified: false,
                reference_certificate_source: ReferenceCertificateSource::Unavailable,
                reference_fallback_used: false,
                certificate_failure: None,
                used_fallback: false,
                embedded_error: None,
                oracle_output_wrms: None,
                oracle_stage_l2: None,
                first_output_wrms: None,
                second_output_wrms: None,
                second_output_ratio: None,
                second_residual_ratio: None,
                second_contraction_evidence: None,
                refined_root_converged: None,
                refined_root_termination: None,
                refined_root_output_wrms: None,
                refined_root_relative_residual: None,
                output_budget: UNIFIED_OUTPUT_BUDGET,
                c3_output_budget_pass: None,
                oracle_output_budget_pass: None,
                c3_false_accept: false,
                first_correction_false_accept: false,
                candidate_counters: WorkCounters::default(),
                certificate_counters: WorkCounters::default(),
                batch_depth: 0,
                batch_vectors: 0,
                compute_seconds: 0.0,
                certificate_seconds: 0.0,
                decision_reason: None,
                failure: Some(error.to_string()),
            },
        };
        row.validate()?;
        rows.push(row);
    }
    Ok(rows)
}

pub fn run_unified_nonlinear_screen(
    profile: UnifiedScreenProfile,
    threads: usize,
) -> CoreResult<UnifiedNonlinearScreen> {
    const ATOL: f64 = 1.0e-7;
    const RTOL: f64 = 1.0e-6;
    let cases = build_unified_cases(profile)?;
    let candidates = nonlinear_candidates(profile)?;
    let execution = ParallelExecution::rayon(threads)?;
    let start = Instant::now();
    let nested = execution.map_ordered(&cases, |case| {
        run_unified_case(case, &candidates, ATOL, RTOL)
    })?;
    let mut rows: Vec<UnifiedCandidateRow> = nested.into_iter().flatten().collect();
    rows.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
    for row in &rows {
        row.validate()?;
    }
    let summary = UnifiedNonlinearSummary {
        cases: cases.len(),
        rows: rows.len(),
        completed: rows
            .iter()
            .filter(|row| row.outcome == UnifiedCandidateOutcome::Completed)
            .count(),
        completed_with_fallback: rows
            .iter()
            .filter(|row| row.outcome == UnifiedCandidateOutcome::CompletedWithFallback)
            .count(),
        rejected: rows
            .iter()
            .filter(|row| row.outcome == UnifiedCandidateOutcome::Rejected)
            .count(),
        uncertified: rows
            .iter()
            .filter(|row| row.outcome == UnifiedCandidateOutcome::Uncertified)
            .count(),
        reference_fallbacks: rows
            .iter()
            .filter(|row| row.reference_fallback_used)
            .count(),
        failures: rows
            .iter()
            .filter(|row| row.outcome == UnifiedCandidateOutcome::NumericalFailure)
            .count(),
        first_correction_false_accepts: rows
            .iter()
            .filter(|row| row.first_correction_false_accept)
            .count(),
    };
    Ok(UnifiedNonlinearScreen {
        schema: "rodas5p-unified-nonlinear-screen-v3",
        status: if summary.failures > 0 {
            "complete-with-failures"
        } else if summary.uncertified > 0 {
            "complete-with-uncertified"
        } else {
            "complete"
        },
        profile: profile.as_str(),
        threads,
        atol: ATOL,
        rtol: RTOL,
        first_correction_budget: UNIFIED_OUTPUT_BUDGET,
        output_budget: UNIFIED_OUTPUT_BUDGET,
        cases: cases.into_iter().map(|case| case.descriptor).collect(),
        rows,
        summary,
        compute_seconds: start.elapsed().as_secs_f64(),
    })
}
