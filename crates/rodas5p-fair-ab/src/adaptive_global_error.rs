use std::time::Instant;

use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, WorkCounters, sha256_hex};
use rodas5p_integrators::{
    AdaptiveObservedIntegrationResult, AdaptiveRunDiagnostics, AdaptiveStepConfig, BdfConfig,
    BdfOrder, DenseOutputError, HomotopyPathConfig, HomotopyPredictor, HomotopyStepConfig,
    IntegrationMethod, OdeProblem, OutputSamplingPlan, OutputSchedule, ParallelExecution,
    RadauConfig, RadauIiaStages, SabrConfig, TransactionalQ1Q2Config,
    TransactionalQ1Q2RunDiagnostics, complex_dahlquist_problem,
    integrate_adaptive_dense_observed_with_config, integrate_adaptive_observed_with_config,
    integrate_bdf_adaptive_dense_observed, integrate_bdf_adaptive_observed,
    integrate_homotopy_adaptive_dense_observed, integrate_homotopy_adaptive_observed,
    integrate_radau_adaptive_dense_observed, integrate_radau_adaptive_observed,
    integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_sequential_matrix_free_adaptive_observed,
    integrate_transactional_q1_q2_adaptive_dense_observed,
    integrate_transactional_q1_q2_adaptive_observed, manufactured_mass_nonlinear_problem,
    manufactured_vector_problem, oscillatory_prothero_robinson_problem, prothero_robinson_problem,
    scalar_linear_problem, semilinear_advection_diffusion_problem,
};
use serde::Serialize;

use crate::global_error::completed_reference_status;
use crate::{
    CommonOutputGrid, DualOutputPolicyEvidence, ExternalErrorScale, FairError, FairResult,
    GlobalErrorMetrics, GlobalErrorParetoProfile, IntegratorRunStatus, IntegratorWorkReport,
    OutputPolicyDominance, OutputPolicyMetadata, OutputPolicyRunEvidence,
    ReferenceSolutionProvenance, ReferenceSourceKind, ReferenceTrajectory, ReferenceWrmsBasis,
    apply_output_policy_dominance, compute_global_error_metrics,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptiveCandidateFamily {
    SequentialRodas5p,
    Sabr5p,
    HomotopyRodas5p,
    TransactionalRodas5p,
    Bdf,
    RadauIia,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AdaptiveCandidateDescriptor {
    pub candidate_id: String,
    pub family: AdaptiveCandidateFamily,
    pub linear_solver: Option<String>,
    pub estimator: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdaptiveProblemDescriptor {
    pub problem_id: String,
    pub dimension: usize,
    pub t_span: (f64, f64),
    pub output_grid_id: String,
    pub reference_checksum: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptiveOutputMode {
    Clipped,
    Dense,
}

impl AdaptiveOutputMode {
    fn id(self) -> &'static str {
        match self {
            Self::Clipped => "clipped",
            Self::Dense => "dense",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdaptiveRunRow {
    pub record_id: String,
    pub pair_id: String,
    pub output_mode: AdaptiveOutputMode,
    /// Only the dense row of a complete, reference-valid, output-policy-
    /// admissible pair may enter a same-error ranking.
    pub same_error_ranking_admissible: bool,
    pub candidate_id: String,
    pub problem_id: String,
    pub rtol: f64,
    pub atol: f64,
    pub status: IntegratorRunStatus,
    pub message: String,
    pub errors: Option<GlobalErrorMetrics>,
    pub work: IntegratorWorkReport,
    pub diagnostics: AdaptiveRunDiagnostics,
    pub transactional: Option<TransactionalQ1Q2RunDiagnostics>,
    pub wall_seconds: f64,
    pub reference_checksum: String,
    pub output_grid_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptiveOutputPolicyPairStatus {
    Admissible,
    OutputPolicyDominated,
    Incomplete,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdaptiveOutputPolicyPairRecord {
    pub pair_id: String,
    pub clipped_record_id: String,
    pub dense_record_id: String,
    pub status: AdaptiveOutputPolicyPairStatus,
    /// Dense row admitted to same-error ranking, or `None` when either arm,
    /// reference dominance, or output-policy sensitivity invalidates the pair.
    pub ranking_record_id: Option<String>,
    pub message: String,
    pub evidence: Option<DualOutputPolicyEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdaptiveScreenExecution {
    pub threads: usize,
    pub backend: String,
    pub scientific_suite_wall_seconds: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdaptiveGlobalErrorReport {
    pub schema: String,
    pub profile: GlobalErrorParetoProfile,
    pub execution: AdaptiveScreenExecution,
    pub candidates: Vec<AdaptiveCandidateDescriptor>,
    pub problems: Vec<AdaptiveProblemDescriptor>,
    pub tolerance_ladder: Vec<f64>,
    pub output_policy: OutputPolicyMetadata,
    pub output_policy_pairs: Vec<AdaptiveOutputPolicyPairRecord>,
    pub runs: Vec<AdaptiveRunRow>,
    pub scientific_checksum: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdaptiveCandidate {
    Sequential(LinearMethod),
    ProtectedSequentialJf,
    Sabr,
    Homotopy,
    Transactional1,
    Transactional4,
    Bdf1,
    Bdf2,
    Radau1,
    Radau3,
}

impl AdaptiveCandidate {
    // Keep the established all-methods authority candidate membership stable.
    // G1-specific matrix-free and transactional candidates are exercised only through
    // `run_g1_adaptive_global_error_screen`; injecting nested internal-thread candidates into
    // this catalog would change the comparator campaign and can also perturb deferred GCRO-DR
    // scratch lifetimes. Radau IIA3 itself uses the source-bound Cell-G embedded estimator.
    const ALL: [Self; 10] = [
        Self::Sequential(LinearMethod::Direct),
        Self::Sequential(LinearMethod::Gmres),
        Self::Sequential(LinearMethod::Lgmres),
        Self::Sequential(LinearMethod::Gcrodr),
        Self::Sabr,
        Self::Homotopy,
        Self::Bdf1,
        Self::Bdf2,
        Self::Radau1,
        Self::Radau3,
    ];

    fn descriptor(self) -> AdaptiveCandidateDescriptor {
        match self {
            Self::Sequential(method) => AdaptiveCandidateDescriptor {
                candidate_id: format!("sequential-rodas5p-{}-adaptive", linear_method_id(method)),
                family: AdaptiveCandidateFamily::SequentialRodas5p,
                linear_solver: Some(linear_method_id(method).into()),
                estimator: "rodas5p-embedded-plus-algebraic".into(),
            },
            Self::ProtectedSequentialJf => AdaptiveCandidateDescriptor {
                candidate_id: "protected-sequential-jf-rodas5p-gmres-adaptive".into(),
                family: AdaptiveCandidateFamily::SequentialRodas5p,
                linear_solver: Some("strict-matrix-free-gmres".into()),
                estimator: "rodas5p-embedded".into(),
            },
            Self::Sabr => AdaptiveCandidateDescriptor {
                candidate_id: "sabr5p-adaptive".into(),
                family: AdaptiveCandidateFamily::Sabr5p,
                linear_solver: Some("direct-fallback".into()),
                estimator: "rodas5p-embedded-plus-algebraic".into(),
            },
            Self::Homotopy => AdaptiveCandidateDescriptor {
                candidate_id: "homotopy-rodas5p-q7-adaptive".into(),
                family: AdaptiveCandidateFamily::HomotopyRodas5p,
                linear_solver: Some("direct-fallback".into()),
                estimator: "homotopy-native-rodas-endpoint".into(),
            },
            Self::Transactional1 | Self::Transactional4 => {
                let threads = if self == Self::Transactional1 { 1 } else { 4 };
                AdaptiveCandidateDescriptor {
                    candidate_id: format!("transactional-q1-q2-rodas5p-t{threads}-adaptive"),
                    family: AdaptiveCandidateFamily::TransactionalRodas5p,
                    linear_solver: Some(format!("matrix-free-common-w-gmres-t{threads}")),
                    estimator: "rodas5p-embedded-plus-operational-q1-q2-certificate".into(),
                }
            }
            Self::Bdf1 => AdaptiveCandidateDescriptor {
                candidate_id: "bdf1-adaptive-reference".into(),
                family: AdaptiveCandidateFamily::Bdf,
                linear_solver: Some("dense-newton".into()),
                estimator: "bdf1-pure-bdf-backward-difference-lte-with-explicit-startup".into(),
            },
            Self::Bdf2 => AdaptiveCandidateDescriptor {
                candidate_id: "bdf2-adaptive-reference".into(),
                family: AdaptiveCandidateFamily::Bdf,
                linear_solver: Some("dense-newton".into()),
                estimator: "bdf2-pure-bdf-backward-difference-lte-with-explicit-startup".into(),
            },
            Self::Radau1 => AdaptiveCandidateDescriptor {
                candidate_id: "radau-iia1-adaptive-reference".into(),
                family: AdaptiveCandidateFamily::RadauIia,
                linear_solver: Some("dense-newton".into()),
                estimator: "radau-iia1-step-doubling".into(),
            },
            Self::Radau3 => AdaptiveCandidateDescriptor {
                candidate_id: "radau-iia3-adaptive-reference".into(),
                family: AdaptiveCandidateFamily::RadauIia,
                linear_solver: Some("dense-newton".into()),
                estimator: "radau-iia3-scipy-1.17.0-embedded-order3".into(),
            },
        }
    }
}

fn linear_method_id(method: LinearMethod) -> &'static str {
    match method {
        LinearMethod::Direct => "direct",
        LinearMethod::Gmres => "gmres",
        LinearMethod::Lgmres => "lgmres",
        LinearMethod::Gcrodr => "gcrodr",
    }
}

#[derive(Clone)]
struct AdaptiveReferenceProblem {
    problem: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
    output_spacing: f64,
    reference: ReferenceTrajectory,
    scale: ExternalErrorScale,
}

#[derive(Clone)]
struct AdaptiveRunSpec {
    reference: AdaptiveReferenceProblem,
    candidate: AdaptiveCandidate,
    rtol: f64,
}

fn analytic_reference_problem(
    problem: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
    output_spacing: f64,
) -> FairResult<AdaptiveReferenceProblem> {
    let output_grid = CommonOutputGrid::uniform(t_span.0, t_span.1, output_spacing)?;
    let states = output_grid
        .times
        .iter()
        .map(|&time| {
            problem.exact(time).ok_or_else(|| {
                FairError::Invalid(format!("problem {} lacks analytic reference", problem.name))
            })
        })
        .collect::<FairResult<Vec<_>>>()?;
    let state_checksum = sha256_hex(&serde_json::to_vec(&(
        &problem.name,
        &output_grid.times,
        &states,
    ))?);
    let provenance = ReferenceSolutionProvenance {
        problem_id: problem.name.clone(),
        source_kind: ReferenceSourceKind::AnalyticExact,
        output_grid_id: output_grid.grid_id.clone(),
        state_checksum,
        reference_uncertainty_wrms: 0.0,
        numerical: None,
    };
    Ok(AdaptiveReferenceProblem {
        scale: ExternalErrorScale::new(vec![1.0e-10; problem.dimension], 1.0e-8)?,
        problem,
        y0,
        t_span,
        output_spacing,
        reference: ReferenceTrajectory {
            output_grid,
            states,
            provenance,
        },
    })
}

fn adaptive_corpus(profile: GlobalErrorParetoProfile) -> FairResult<Vec<AdaptiveReferenceProblem>> {
    let mut corpus = Vec::new();
    let (linear, linear_y0) = scalar_linear_problem(-2.0, 1.0);
    corpus.push(analytic_reference_problem(
        linear,
        linear_y0,
        (0.0, 0.2),
        0.04,
    )?);
    let (pr, pr_y0) = prothero_robinson_problem(-20.0, 1.0, 0.0);
    corpus.push(analytic_reference_problem(pr, pr_y0, (0.0, 0.2), 0.04)?);
    if profile == GlobalErrorParetoProfile::Canonical {
        let (vector, vector_y0) = manufactured_vector_problem(4, 20.0, 1.0, 0.2, 0.0)?;
        corpus.push(analytic_reference_problem(
            vector,
            vector_y0,
            (0.0, 0.2),
            0.04,
        )?);
        let (mass, mass_y0, _, _) = manufactured_mass_nonlinear_problem(20.0, 1.0, 0.2, 0.0)?;
        corpus.push(analytic_reference_problem(
            mass,
            mass_y0,
            (0.0, 0.08),
            0.02,
        )?);
    }
    Ok(corpus)
}

fn g1_adaptive_corpus(
    profile: GlobalErrorParetoProfile,
) -> FairResult<Vec<AdaptiveReferenceProblem>> {
    let mut corpus = adaptive_corpus(profile)?;
    if profile == GlobalErrorParetoProfile::Canonical {
        let (complex, complex_y0) = complex_dahlquist_problem(16, 120.0, 180.0, 0.0)?;
        corpus.push(analytic_reference_problem(
            complex,
            complex_y0,
            (0.0, 0.02),
            0.02,
        )?);
        let (oscillatory_pr, oscillatory_pr_y0) =
            oscillatory_prothero_robinson_problem(-10_000.0, 1_000.0, 140.0, 0.0)?;
        corpus.push(analytic_reference_problem(
            oscillatory_pr,
            oscillatory_pr_y0,
            (0.0, 0.02),
            0.02,
        )?);
        let (large_vector, large_vector_y0) =
            manufactured_vector_problem(32, 1_000.0, 100.0, 0.5, 0.0)?;
        corpus.push(analytic_reference_problem(
            large_vector,
            large_vector_y0,
            (0.0, 0.02),
            0.02,
        )?);
        let (advection, advection_y0) =
            semilinear_advection_diffusion_problem(32, 0.01, 5.0, -1.0, 10.0, 0.0)?;
        corpus.push(analytic_reference_problem(
            advection,
            advection_y0,
            (0.0, 0.02),
            0.02,
        )?);
    }
    Ok(corpus)
}

fn tolerance_ladder(profile: GlobalErrorParetoProfile) -> Vec<f64> {
    match profile {
        GlobalErrorParetoProfile::Smoke => vec![1.0e-4, 1.0e-6],
        GlobalErrorParetoProfile::Canonical => vec![1.0e-4, 1.0e-6, 1.0e-8],
    }
}

fn adaptive_config(reference: &AdaptiveReferenceProblem, rtol: f64) -> AdaptiveStepConfig {
    let integration_span = reference.t_span.1 - reference.t_span.0;
    AdaptiveStepConfig {
        atol: 0.01 * rtol,
        rtol,
        initial_step: reference.output_spacing,
        min_step: 1.0e-12,
        // Observation density is not an integrator stability or accuracy policy.
        // Dense-capable v2 paths may grow to the full domain span; legacy clipped
        // paths still report their clipping explicitly.
        max_step: integration_span,
        max_attempts: 200_000,
        ..AdaptiveStepConfig::default()
    }
}

fn linear_config(method: LinearMethod) -> LinearSolverConfig {
    LinearSolverConfig {
        method,
        ..LinearSolverConfig::default()
    }
}

struct CandidateExecution {
    result: AdaptiveObservedIntegrationResult,
    transactional: Option<TransactionalQ1Q2RunDiagnostics>,
}

fn dense_core<T>(result: Result<T, DenseOutputError>) -> Result<T, CoreError> {
    result.map_err(|error| match error {
        DenseOutputError::Core(error) => error,
    })
}

fn execute_candidate(
    spec: &AdaptiveRunSpec,
    output_mode: AdaptiveOutputMode,
) -> Result<CandidateExecution, CoreError> {
    let reference = &spec.reference;
    let output = OutputSchedule::new(reference.reference.output_grid.times.clone())?;
    let sampling = OutputSamplingPlan::dense(output.clone());
    let adaptive = adaptive_config(reference, spec.rtol);
    match spec.candidate {
        AdaptiveCandidate::Sequential(method) => {
            let linear = linear_config(method);
            let result = match output_mode {
                AdaptiveOutputMode::Clipped => integrate_adaptive_observed_with_config(
                    &reference.problem,
                    reference.t_span,
                    &reference.y0,
                    IntegrationMethod::Sequential,
                    Some(&linear),
                    None,
                    &adaptive,
                    &output,
                ),
                AdaptiveOutputMode::Dense => {
                    dense_core(integrate_adaptive_dense_observed_with_config(
                        &reference.problem,
                        reference.t_span,
                        &reference.y0,
                        IntegrationMethod::Sequential,
                        Some(&linear),
                        None,
                        &adaptive,
                        &sampling,
                    ))
                }
            }?;
            Ok(CandidateExecution {
                result,
                transactional: None,
            })
        }
        AdaptiveCandidate::ProtectedSequentialJf => {
            let matrix_free_problem = reference.problem.jvp_only_clone()?;
            let linear = linear_config(LinearMethod::Gmres);
            let result = match output_mode {
                AdaptiveOutputMode::Clipped => integrate_sequential_matrix_free_adaptive_observed(
                    &matrix_free_problem,
                    reference.t_span,
                    &reference.y0,
                    &linear,
                    &adaptive,
                    &output,
                ),
                AdaptiveOutputMode::Dense => {
                    dense_core(integrate_sequential_matrix_free_adaptive_dense_observed(
                        &matrix_free_problem,
                        reference.t_span,
                        &reference.y0,
                        &linear,
                        &adaptive,
                        &sampling,
                    ))
                }
            }?;
            Ok(CandidateExecution {
                result,
                transactional: None,
            })
        }
        AdaptiveCandidate::Sabr => {
            let linear = linear_config(LinearMethod::Direct);
            let result = match output_mode {
                AdaptiveOutputMode::Clipped => integrate_adaptive_observed_with_config(
                    &reference.problem,
                    reference.t_span,
                    &reference.y0,
                    IntegrationMethod::Sabr,
                    Some(&linear),
                    Some(SabrConfig::default()),
                    &adaptive,
                    &output,
                ),
                AdaptiveOutputMode::Dense => {
                    dense_core(integrate_adaptive_dense_observed_with_config(
                        &reference.problem,
                        reference.t_span,
                        &reference.y0,
                        IntegrationMethod::Sabr,
                        Some(&linear),
                        Some(SabrConfig::default()),
                        &adaptive,
                        &sampling,
                    ))
                }
            }?;
            Ok(CandidateExecution {
                result,
                transactional: None,
            })
        }
        AdaptiveCandidate::Homotopy => {
            let path = HomotopyPathConfig::new(1.0, 7, 2, HomotopyPredictor::AdamsBashforth2, 1)?;
            let homotopy = HomotopyStepConfig::new(path, 0.1)?;
            let linear = linear_config(LinearMethod::Direct);
            let result = match output_mode {
                AdaptiveOutputMode::Clipped => integrate_homotopy_adaptive_observed(
                    &reference.problem,
                    reference.t_span,
                    &reference.y0,
                    &homotopy,
                    Some(&linear),
                    &adaptive,
                    &output,
                ),
                AdaptiveOutputMode::Dense => {
                    dense_core(integrate_homotopy_adaptive_dense_observed(
                        &reference.problem,
                        reference.t_span,
                        &reference.y0,
                        &homotopy,
                        Some(&linear),
                        &adaptive,
                        &sampling,
                    ))
                }
            }?;
            Ok(CandidateExecution {
                result,
                transactional: None,
            })
        }
        AdaptiveCandidate::Transactional1 | AdaptiveCandidate::Transactional4 => {
            let matrix_free_problem = reference.problem.jvp_only_clone()?;
            let threads = if spec.candidate == AdaptiveCandidate::Transactional1 {
                1
            } else {
                4
            };
            let step_config = TransactionalQ1Q2Config {
                threads,
                ..TransactionalQ1Q2Config::default()
            };
            let transactional = match output_mode {
                AdaptiveOutputMode::Clipped => integrate_transactional_q1_q2_adaptive_observed(
                    &matrix_free_problem,
                    reference.t_span,
                    &reference.y0,
                    &step_config,
                    &adaptive,
                    &output,
                ),
                AdaptiveOutputMode::Dense => {
                    dense_core(integrate_transactional_q1_q2_adaptive_dense_observed(
                        &matrix_free_problem,
                        reference.t_span,
                        &reference.y0,
                        &step_config,
                        &adaptive,
                        &sampling,
                    ))
                }
            }?;
            Ok(CandidateExecution {
                result: AdaptiveObservedIntegrationResult {
                    observed: transactional.observed,
                    diagnostics: transactional.diagnostics,
                },
                transactional: Some(transactional.transactional),
            })
        }
        AdaptiveCandidate::Bdf1 | AdaptiveCandidate::Bdf2 => {
            let order = if spec.candidate == AdaptiveCandidate::Bdf1 {
                BdfOrder::One
            } else {
                BdfOrder::Two
            };
            let config = BdfConfig {
                order,
                ..BdfConfig::default()
            };
            let result = match output_mode {
                AdaptiveOutputMode::Clipped => integrate_bdf_adaptive_observed(
                    &reference.problem,
                    reference.t_span,
                    &reference.y0,
                    &config,
                    &adaptive,
                    &output,
                ),
                AdaptiveOutputMode::Dense => dense_core(integrate_bdf_adaptive_dense_observed(
                    &reference.problem,
                    reference.t_span,
                    &reference.y0,
                    &config,
                    &adaptive,
                    &sampling,
                )),
            }?;
            Ok(CandidateExecution {
                result,
                transactional: None,
            })
        }
        AdaptiveCandidate::Radau1 | AdaptiveCandidate::Radau3 => {
            let stages = if spec.candidate == AdaptiveCandidate::Radau1 {
                RadauIiaStages::One
            } else {
                RadauIiaStages::Three
            };
            let config = RadauConfig {
                stages,
                ..RadauConfig::default()
            };
            let result = match output_mode {
                AdaptiveOutputMode::Clipped => integrate_radau_adaptive_observed(
                    &reference.problem,
                    reference.t_span,
                    &reference.y0,
                    &config,
                    &adaptive,
                    &output,
                ),
                AdaptiveOutputMode::Dense => dense_core(integrate_radau_adaptive_dense_observed(
                    &reference.problem,
                    reference.t_span,
                    &reference.y0,
                    &config,
                    &adaptive,
                    &sampling,
                )),
            }?;
            Ok(CandidateExecution {
                result,
                transactional: None,
            })
        }
    }
}

fn retained_state_bytes(times: &[f64], states: &[Vec<f64>]) -> u64 {
    let scalars = times.len() + states.iter().map(Vec::len).sum::<usize>();
    (scalars * std::mem::size_of::<f64>()) as u64
}

fn pair_id(spec: &AdaptiveRunSpec) -> String {
    format!(
        "{}|{}|rtol{:016x}",
        spec.reference.problem.name,
        spec.candidate.descriptor().candidate_id,
        spec.rtol.to_bits()
    )
}

fn record_id(spec: &AdaptiveRunSpec, output_mode: AdaptiveOutputMode) -> String {
    format!("{}|output-{}", pair_id(spec), output_mode.id())
}

fn observed_work(
    times: &[f64],
    states: &[Vec<f64>],
    counters: WorkCounters,
    internal_steps: usize,
    output_clipped_steps: usize,
) -> IntegratorWorkReport {
    IntegratorWorkReport {
        counters,
        internal_steps: internal_steps as u64,
        output_clipped_steps: output_clipped_steps as u64,
        stored_state_bytes: retained_state_bytes(times, states),
    }
}

struct AdaptiveFailureEvidence {
    wall_seconds: f64,
    diagnostics: AdaptiveRunDiagnostics,
    work: IntegratorWorkReport,
    transactional: Option<TransactionalQ1Q2RunDiagnostics>,
}

fn failed_row(
    spec: &AdaptiveRunSpec,
    output_mode: AdaptiveOutputMode,
    status: IntegratorRunStatus,
    message: String,
    failure: AdaptiveFailureEvidence,
) -> AdaptiveRunRow {
    AdaptiveRunRow {
        record_id: record_id(spec, output_mode),
        pair_id: pair_id(spec),
        output_mode,
        same_error_ranking_admissible: false,
        candidate_id: spec.candidate.descriptor().candidate_id,
        problem_id: spec.reference.problem.name.clone(),
        rtol: spec.rtol,
        atol: 0.01 * spec.rtol,
        status,
        message,
        errors: None,
        work: failure.work,
        diagnostics: failure.diagnostics,
        transactional: failure.transactional,
        wall_seconds: failure.wall_seconds,
        reference_checksum: spec.reference.reference.provenance.state_checksum.clone(),
        output_grid_id: spec.reference.reference.output_grid.grid_id.clone(),
    }
}

struct CompletedAdaptiveRun {
    row: AdaptiveRunRow,
    evidence: Option<OutputPolicyRunEvidence>,
}

fn run_output_mode(
    spec: &AdaptiveRunSpec,
    output_mode: AdaptiveOutputMode,
) -> CompletedAdaptiveRun {
    let started = Instant::now();
    let result = match execute_candidate(spec, output_mode) {
        Ok(result) => result,
        Err(error) => {
            return CompletedAdaptiveRun {
                row: failed_row(
                    spec,
                    output_mode,
                    IntegratorRunStatus::SolverFailure,
                    error.to_string(),
                    AdaptiveFailureEvidence {
                        wall_seconds: started.elapsed().as_secs_f64(),
                        diagnostics: AdaptiveRunDiagnostics::default(),
                        work: observed_work(&[], &[], WorkCounters::default(), 0, 0),
                        transactional: None,
                    },
                ),
                evidence: None,
            };
        }
    };
    let wall_seconds = started.elapsed().as_secs_f64();
    let transactional = result.transactional;
    let observed = result.result.observed;
    let diagnostics = result.result.diagnostics;
    let work = observed_work(
        &observed.t,
        &observed.y,
        observed.counters,
        observed.internal_steps,
        observed.output_clipped_steps,
    );
    if !observed.success {
        return CompletedAdaptiveRun {
            row: failed_row(
                spec,
                output_mode,
                IntegratorRunStatus::SolverFailure,
                observed.message,
                AdaptiveFailureEvidence {
                    wall_seconds,
                    diagnostics,
                    work,
                    transactional,
                },
            ),
            evidence: None,
        };
    }
    let errors = match compute_global_error_metrics(
        &spec.reference.reference.output_grid,
        &observed.t,
        &observed.y,
        &spec.reference.reference.states,
        &spec.reference.scale,
    ) {
        Ok(errors) => errors,
        Err(error) => {
            return CompletedAdaptiveRun {
                row: failed_row(
                    spec,
                    output_mode,
                    IntegratorRunStatus::MissingOutput,
                    error.to_string(),
                    AdaptiveFailureEvidence {
                        wall_seconds,
                        diagnostics,
                        work,
                        transactional,
                    },
                ),
                evidence: None,
            };
        }
    };
    let (status, message) = completed_reference_status(
        &spec.reference.reference.provenance,
        &errors,
        &spec.reference.scale,
    );
    let evidence = OutputPolicyRunEvidence {
        output_times: observed.t.clone(),
        states: observed.y.clone(),
        errors: errors.clone(),
        work: work.clone(),
    };
    CompletedAdaptiveRun {
        row: AdaptiveRunRow {
            record_id: record_id(spec, output_mode),
            pair_id: pair_id(spec),
            output_mode,
            same_error_ranking_admissible: false,
            candidate_id: spec.candidate.descriptor().candidate_id,
            problem_id: spec.reference.problem.name.clone(),
            rtol: spec.rtol,
            atol: 0.01 * spec.rtol,
            status,
            message,
            errors: Some(errors),
            work,
            diagnostics,
            transactional,
            wall_seconds,
            reference_checksum: spec.reference.reference.provenance.state_checksum.clone(),
            output_grid_id: spec.reference.reference.output_grid.grid_id.clone(),
        },
        evidence: Some(evidence),
    }
}

struct AdaptivePairExecution {
    rows: [AdaptiveRunRow; 2],
    pair: AdaptiveOutputPolicyPairRecord,
}

fn run_spec_pair(spec: &AdaptiveRunSpec) -> AdaptivePairExecution {
    // These are deliberately two complete calls.  No controller, Krylov,
    // history, or transactional diagnostics object crosses the policy boundary.
    let mut clipped = run_output_mode(spec, AdaptiveOutputMode::Clipped);
    let mut dense = run_output_mode(spec, AdaptiveOutputMode::Dense);
    let pair_id = pair_id(spec);
    let clipped_record_id = clipped.row.record_id.clone();
    let dense_record_id = dense.row.record_id.clone();

    let (Some(clipped_evidence), Some(dense_evidence)) =
        (clipped.evidence.take(), dense.evidence.take())
    else {
        return AdaptivePairExecution {
            rows: [clipped.row, dense.row],
            pair: AdaptiveOutputPolicyPairRecord {
                pair_id,
                clipped_record_id,
                dense_record_id,
                status: AdaptiveOutputPolicyPairStatus::Incomplete,
                ranking_record_id: None,
                message: "same-error pair excluded: clipped or dense execution did not produce a complete trajectory".into(),
                evidence: None,
            },
        };
    };
    let basis = match ReferenceWrmsBasis::new(
        spec.reference.reference.output_grid.clone(),
        spec.reference.reference.states.clone(),
        spec.reference.scale.clone(),
    ) {
        Ok(basis) => basis,
        Err(error) => {
            return AdaptivePairExecution {
                rows: [clipped.row, dense.row],
                pair: AdaptiveOutputPolicyPairRecord {
                    pair_id,
                    clipped_record_id,
                    dense_record_id,
                    status: AdaptiveOutputPolicyPairStatus::Incomplete,
                    ranking_record_id: None,
                    message: format!(
                        "same-error pair excluded: reference WRMS basis failed: {error}"
                    ),
                    evidence: None,
                },
            };
        }
    };
    let evidence = match DualOutputPolicyEvidence::new(basis, clipped_evidence, dense_evidence) {
        Ok(evidence) => evidence,
        Err(error) => {
            return AdaptivePairExecution {
                rows: [clipped.row, dense.row],
                pair: AdaptiveOutputPolicyPairRecord {
                    pair_id,
                    clipped_record_id,
                    dense_record_id,
                    status: AdaptiveOutputPolicyPairStatus::Incomplete,
                    ranking_record_id: None,
                    message: format!("same-error pair excluded: paired evidence failed: {error}"),
                    evidence: None,
                },
            };
        }
    };
    let classification = match evidence.classify() {
        Ok(classification) => classification,
        Err(error) => {
            return AdaptivePairExecution {
                rows: [clipped.row, dense.row],
                pair: AdaptiveOutputPolicyPairRecord {
                    pair_id,
                    clipped_record_id,
                    dense_record_id,
                    status: AdaptiveOutputPolicyPairStatus::Incomplete,
                    ranking_record_id: None,
                    message: format!(
                        "same-error pair excluded: output-policy classification failed: {error}"
                    ),
                    evidence: Some(evidence),
                },
            };
        }
    };
    let (clipped_status, clipped_message) =
        match apply_output_policy_dominance(clipped.row.status.clone(), &evidence) {
            Ok(applied) => applied,
            Err(error) => {
                return AdaptivePairExecution {
                    rows: [clipped.row, dense.row],
                    pair: AdaptiveOutputPolicyPairRecord {
                        pair_id,
                        clipped_record_id,
                        dense_record_id,
                        status: AdaptiveOutputPolicyPairStatus::Incomplete,
                        ranking_record_id: None,
                        message: format!(
                            "same-error pair excluded: clipped policy application failed: {error}"
                        ),
                        evidence: Some(evidence),
                    },
                };
            }
        };
    let (dense_status, dense_message) =
        match apply_output_policy_dominance(dense.row.status.clone(), &evidence) {
            Ok(applied) => applied,
            Err(error) => {
                return AdaptivePairExecution {
                    rows: [clipped.row, dense.row],
                    pair: AdaptiveOutputPolicyPairRecord {
                        pair_id,
                        clipped_record_id,
                        dense_record_id,
                        status: AdaptiveOutputPolicyPairStatus::Incomplete,
                        ranking_record_id: None,
                        message: format!(
                            "same-error pair excluded: dense policy application failed: {error}"
                        ),
                        evidence: Some(evidence),
                    },
                };
            }
        };
    if clipped_status != clipped.row.status {
        clipped.row.status = clipped_status;
        clipped.row.message = clipped_message;
    }
    if dense_status != dense.row.status {
        dense.row.status = dense_status;
        dense.row.message = dense_message;
    }
    let both_success = clipped.row.status == IntegratorRunStatus::Success
        && dense.row.status == IntegratorRunStatus::Success;
    let ranking_record_id = if classification == OutputPolicyDominance::Admissible && both_success {
        dense.row.same_error_ranking_admissible = true;
        Some(dense_record_id.clone())
    } else {
        None
    };
    let (status, message) = match classification {
        OutputPolicyDominance::Admissible if both_success => (
            AdaptiveOutputPolicyPairStatus::Admissible,
            "paired clipped/dense evidence admissible; dense row admitted to same-error ranking"
                .into(),
        ),
        OutputPolicyDominance::Admissible => (
            AdaptiveOutputPolicyPairStatus::Admissible,
            "paired clipped/dense evidence admissible, but run or reference status excludes both rows from same-error ranking".into(),
        ),
        OutputPolicyDominance::Dominated => (
            AdaptiveOutputPolicyPairStatus::OutputPolicyDominated,
            "same-error pair excluded: clipped/dense max-grid WRMS gap exceeds 10% of dense measured error".into(),
        ),
    };
    AdaptivePairExecution {
        rows: [clipped.row, dense.row],
        pair: AdaptiveOutputPolicyPairRecord {
            pair_id,
            clipped_record_id,
            dense_record_id,
            status,
            ranking_record_id,
            message,
            evidence: Some(evidence),
        },
    }
}

#[derive(Serialize)]
struct ScientificAdaptiveRow<'a> {
    record_id: &'a str,
    pair_id: &'a str,
    output_mode: AdaptiveOutputMode,
    same_error_ranking_admissible: bool,
    candidate_id: &'a str,
    problem_id: &'a str,
    rtol_bits: u64,
    atol_bits: u64,
    status: &'a IntegratorRunStatus,
    message: &'a str,
    errors: &'a Option<GlobalErrorMetrics>,
    work: &'a IntegratorWorkReport,
    diagnostics: &'a AdaptiveRunDiagnostics,
    transactional: &'a Option<TransactionalQ1Q2RunDiagnostics>,
    reference_checksum: &'a str,
    output_grid_id: &'a str,
}

fn scientific_checksum(
    profile: GlobalErrorParetoProfile,
    candidates: &[AdaptiveCandidateDescriptor],
    problems: &[AdaptiveProblemDescriptor],
    tolerances: &[f64],
    output_policy: &OutputPolicyMetadata,
    output_policy_pairs: &[AdaptiveOutputPolicyPairRecord],
    runs: &[AdaptiveRunRow],
) -> FairResult<String> {
    let scientific_runs = runs
        .iter()
        .map(|row| ScientificAdaptiveRow {
            record_id: &row.record_id,
            pair_id: &row.pair_id,
            output_mode: row.output_mode,
            same_error_ranking_admissible: row.same_error_ranking_admissible,
            candidate_id: &row.candidate_id,
            problem_id: &row.problem_id,
            rtol_bits: row.rtol.to_bits(),
            atol_bits: row.atol.to_bits(),
            status: &row.status,
            message: &row.message,
            errors: &row.errors,
            work: &row.work,
            diagnostics: &row.diagnostics,
            transactional: &row.transactional,
            reference_checksum: &row.reference_checksum,
            output_grid_id: &row.output_grid_id,
        })
        .collect::<Vec<_>>();
    Ok(sha256_hex(&serde_json::to_vec(&(
        profile,
        candidates,
        problems,
        output_policy,
        tolerances
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        output_policy_pairs,
        scientific_runs,
    ))?))
}

fn adaptive_output_policy() -> OutputPolicyMetadata {
    OutputPolicyMetadata {
        save_internal_steps: false,
        dense_output_used: true,
        landing: "paired-independent-step-clipping-and-dense-sampling".into(),
    }
}

pub fn run_adaptive_global_error_screen(
    profile: GlobalErrorParetoProfile,
    threads: usize,
) -> FairResult<AdaptiveGlobalErrorReport> {
    let execution = ParallelExecution::rayon(threads)?;
    let corpus = adaptive_corpus(profile)?;
    let tolerances = tolerance_ladder(profile);
    let candidates = AdaptiveCandidate::ALL
        .iter()
        .copied()
        .map(AdaptiveCandidate::descriptor)
        .collect::<Vec<_>>();
    let problems = corpus
        .iter()
        .map(|reference| AdaptiveProblemDescriptor {
            problem_id: reference.problem.name.clone(),
            dimension: reference.problem.dimension,
            t_span: reference.t_span,
            output_grid_id: reference.reference.output_grid.grid_id.clone(),
            reference_checksum: reference.reference.provenance.state_checksum.clone(),
        })
        .collect::<Vec<_>>();
    let mut specs = Vec::new();
    for reference in corpus {
        for &rtol in &tolerances {
            for candidate in AdaptiveCandidate::ALL {
                specs.push(AdaptiveRunSpec {
                    reference: reference.clone(),
                    candidate,
                    rtol,
                });
            }
        }
    }
    let started = Instant::now();
    let executions = execution.map_ordered(&specs, |spec| Ok(run_spec_pair(spec)))?;
    let scientific_suite_wall_seconds = started.elapsed().as_secs_f64();
    let mut runs = executions
        .iter()
        .flat_map(|execution| execution.rows.iter().cloned())
        .collect::<Vec<_>>();
    let mut output_policy_pairs = executions
        .into_iter()
        .map(|execution| execution.pair)
        .collect::<Vec<_>>();
    runs.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    output_policy_pairs.sort_by(|left, right| left.pair_id.cmp(&right.pair_id));
    let output_policy = adaptive_output_policy();
    let checksum = scientific_checksum(
        profile,
        &candidates,
        &problems,
        &tolerances,
        &output_policy,
        &output_policy_pairs,
        &runs,
    )?;
    Ok(AdaptiveGlobalErrorReport {
        schema: "rodas5p-adaptive-global-error-v2".into(),
        profile,
        execution: AdaptiveScreenExecution {
            threads: execution.threads(),
            backend: execution.backend().into(),
            scientific_suite_wall_seconds,
        },
        candidates,
        problems,
        tolerance_ladder: tolerances,
        output_policy,
        output_policy_pairs,
        runs,
        scientific_checksum: checksum,
    })
}

/// G1-specific complete-integrator screen.
///
/// This deliberately excludes unrelated mutable solver arms (LGMRES/GCRO-DR, direct RODAS,
/// SABR and q=7 homotopy) so a failure in a deferred comparator cannot mask the transactional
/// q1->q2 decision. BDF2 remains a legacy context comparator; Radau IIA3 is the authorized
/// Cell-G frozen-Jacobian/embedded-estimator context comparator.
pub fn run_g1_adaptive_global_error_screen(
    profile: GlobalErrorParetoProfile,
    threads: usize,
) -> FairResult<AdaptiveGlobalErrorReport> {
    let execution = ParallelExecution::rayon(threads)?;
    let corpus = g1_adaptive_corpus(profile)?;
    let tolerances = tolerance_ladder(profile);
    let selected = [
        AdaptiveCandidate::ProtectedSequentialJf,
        AdaptiveCandidate::Transactional1,
        AdaptiveCandidate::Transactional4,
        AdaptiveCandidate::Bdf2,
        AdaptiveCandidate::Radau3,
    ];
    let candidates = selected
        .iter()
        .copied()
        .map(AdaptiveCandidate::descriptor)
        .collect::<Vec<_>>();
    let problems = corpus
        .iter()
        .map(|reference| AdaptiveProblemDescriptor {
            problem_id: reference.problem.name.clone(),
            dimension: reference.problem.dimension,
            t_span: reference.t_span,
            output_grid_id: reference.reference.output_grid.grid_id.clone(),
            reference_checksum: reference.reference.provenance.state_checksum.clone(),
        })
        .collect::<Vec<_>>();
    let mut specs = Vec::new();
    for reference in corpus {
        for &rtol in &tolerances {
            for &candidate in &selected {
                specs.push(AdaptiveRunSpec {
                    reference: reference.clone(),
                    candidate,
                    rtol,
                });
            }
        }
    }
    let started = Instant::now();
    let executions = execution.map_ordered(&specs, |spec| Ok(run_spec_pair(spec)))?;
    let scientific_suite_wall_seconds = started.elapsed().as_secs_f64();
    let mut runs = executions
        .iter()
        .flat_map(|execution| execution.rows.iter().cloned())
        .collect::<Vec<_>>();
    let mut output_policy_pairs = executions
        .into_iter()
        .map(|execution| execution.pair)
        .collect::<Vec<_>>();
    runs.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    output_policy_pairs.sort_by(|left, right| left.pair_id.cmp(&right.pair_id));
    let output_policy = adaptive_output_policy();
    let checksum = scientific_checksum(
        profile,
        &candidates,
        &problems,
        &tolerances,
        &output_policy,
        &output_policy_pairs,
        &runs,
    )?;
    Ok(AdaptiveGlobalErrorReport {
        schema: "generic-q1-q2-adaptive-global-error-v2".into(),
        profile,
        execution: AdaptiveScreenExecution {
            threads: execution.threads(),
            backend: execution.backend().into(),
            scientific_suite_wall_seconds,
        },
        candidates,
        problems,
        tolerance_ladder: tolerances,
        output_policy,
        output_policy_pairs,
        runs,
        scientific_checksum: checksum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_max_step_is_independent_of_output_spacing() {
        let mut reference = adaptive_corpus(GlobalErrorParetoProfile::Smoke)
            .unwrap()
            .remove(0);
        let span = reference.t_span.1 - reference.t_span.0;
        let first = adaptive_config(&reference, 1.0e-6);
        reference.output_spacing *= 0.125;
        let second = adaptive_config(&reference, 1.0e-6);
        assert_eq!(first.max_step.to_bits(), span.to_bits());
        assert_eq!(second.max_step.to_bits(), span.to_bits());
    }

    #[test]
    fn scientific_checksum_covers_output_modes_and_pair_classification() {
        let report = run_adaptive_global_error_screen(GlobalErrorParetoProfile::Smoke, 1).unwrap();
        let recomputed = scientific_checksum(
            report.profile,
            &report.candidates,
            &report.problems,
            &report.tolerance_ladder,
            &report.output_policy,
            &report.output_policy_pairs,
            &report.runs,
        )
        .unwrap();
        assert_eq!(recomputed, report.scientific_checksum);

        let mut changed_rows = report.runs.clone();
        changed_rows[0].output_mode = match changed_rows[0].output_mode {
            AdaptiveOutputMode::Clipped => AdaptiveOutputMode::Dense,
            AdaptiveOutputMode::Dense => AdaptiveOutputMode::Clipped,
        };
        let changed_mode = scientific_checksum(
            report.profile,
            &report.candidates,
            &report.problems,
            &report.tolerance_ladder,
            &report.output_policy,
            &report.output_policy_pairs,
            &changed_rows,
        )
        .unwrap();
        assert_ne!(changed_mode, report.scientific_checksum);

        let mut changed_pairs = report.output_policy_pairs.clone();
        changed_pairs[0].status = match changed_pairs[0].status {
            AdaptiveOutputPolicyPairStatus::Admissible => {
                AdaptiveOutputPolicyPairStatus::OutputPolicyDominated
            }
            AdaptiveOutputPolicyPairStatus::OutputPolicyDominated
            | AdaptiveOutputPolicyPairStatus::Incomplete => {
                AdaptiveOutputPolicyPairStatus::Admissible
            }
        };
        let changed_pair = scientific_checksum(
            report.profile,
            &report.candidates,
            &report.problems,
            &report.tolerance_ladder,
            &report.output_policy,
            &changed_pairs,
            &report.runs,
        )
        .unwrap();
        assert_ne!(changed_pair, report.scientific_checksum);
    }
}
