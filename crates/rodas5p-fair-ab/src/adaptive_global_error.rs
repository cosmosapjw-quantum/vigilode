use std::time::Instant;

use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, WorkCounters, sha256_hex};
use rodas5p_integrators::{
    AdaptiveObservedIntegrationResult, AdaptiveRunDiagnostics, AdaptiveStepConfig, BdfConfig,
    BdfOrder, HomotopyPathConfig, HomotopyPredictor, HomotopyStepConfig, IntegrationMethod,
    OdeProblem, OutputSchedule, ParallelExecution, RadauConfig, RadauIiaStages, SabrConfig,
    integrate_adaptive_observed_with_config, integrate_bdf_adaptive_observed,
    integrate_homotopy_adaptive_observed, integrate_radau_adaptive_observed,
    manufactured_mass_nonlinear_problem, manufactured_vector_problem, prothero_robinson_problem,
    scalar_linear_problem,
};
use serde::Serialize;

use crate::{
    CommonOutputGrid, ExternalErrorScale, FairError, FairResult, GlobalErrorMetrics,
    GlobalErrorParetoProfile, IntegratorRunStatus, IntegratorWorkReport,
    ReferenceSolutionProvenance, ReferenceSourceKind, ReferenceTrajectory,
    compute_global_error_metrics,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptiveCandidateFamily {
    SequentialRodas5p,
    Sabr5p,
    HomotopyRodas5p,
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdaptiveRunRow {
    pub record_id: String,
    pub candidate_id: String,
    pub problem_id: String,
    pub rtol: f64,
    pub atol: f64,
    pub status: IntegratorRunStatus,
    pub message: String,
    pub errors: Option<GlobalErrorMetrics>,
    pub work: IntegratorWorkReport,
    pub diagnostics: AdaptiveRunDiagnostics,
    pub wall_seconds: f64,
    pub reference_checksum: String,
    pub output_grid_id: String,
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
    pub runs: Vec<AdaptiveRunRow>,
    pub scientific_checksum: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdaptiveCandidate {
    Sequential(LinearMethod),
    Sabr,
    Homotopy,
    Bdf1,
    Bdf2,
    Radau1,
    Radau3,
}

impl AdaptiveCandidate {
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
            Self::Bdf1 => AdaptiveCandidateDescriptor {
                candidate_id: "bdf1-adaptive-reference".into(),
                family: AdaptiveCandidateFamily::Bdf,
                linear_solver: Some("dense-newton".into()),
                estimator: "bdf1-step-doubling".into(),
            },
            Self::Bdf2 => AdaptiveCandidateDescriptor {
                candidate_id: "bdf2-adaptive-reference".into(),
                family: AdaptiveCandidateFamily::Bdf,
                linear_solver: Some("dense-newton".into()),
                estimator: "bdf2-step-doubling-with-startup".into(),
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
                estimator: "radau-iia3-step-doubling".into(),
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

fn tolerance_ladder(profile: GlobalErrorParetoProfile) -> Vec<f64> {
    match profile {
        GlobalErrorParetoProfile::Smoke => vec![1.0e-4, 1.0e-6],
        GlobalErrorParetoProfile::Canonical => vec![1.0e-4, 1.0e-6, 1.0e-8],
    }
}

fn adaptive_config(reference: &AdaptiveReferenceProblem, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 0.01 * rtol,
        rtol,
        initial_step: reference.output_spacing,
        min_step: 1.0e-12,
        max_step: reference.output_spacing,
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

fn execute_candidate(
    spec: &AdaptiveRunSpec,
) -> Result<AdaptiveObservedIntegrationResult, CoreError> {
    let reference = &spec.reference;
    let output = OutputSchedule::new(reference.reference.output_grid.times.clone())?;
    let adaptive = adaptive_config(reference, spec.rtol);
    match spec.candidate {
        AdaptiveCandidate::Sequential(method) => integrate_adaptive_observed_with_config(
            &reference.problem,
            reference.t_span,
            &reference.y0,
            IntegrationMethod::Sequential,
            Some(&linear_config(method)),
            None,
            &adaptive,
            &output,
        ),
        AdaptiveCandidate::Sabr => integrate_adaptive_observed_with_config(
            &reference.problem,
            reference.t_span,
            &reference.y0,
            IntegrationMethod::Sabr,
            Some(&linear_config(LinearMethod::Direct)),
            Some(SabrConfig::default()),
            &adaptive,
            &output,
        ),
        AdaptiveCandidate::Homotopy => {
            let path = HomotopyPathConfig::new(1.0, 7, 2, HomotopyPredictor::AdamsBashforth2, 1)?;
            let homotopy = HomotopyStepConfig::new(path, 0.1)?;
            integrate_homotopy_adaptive_observed(
                &reference.problem,
                reference.t_span,
                &reference.y0,
                &homotopy,
                Some(&linear_config(LinearMethod::Direct)),
                &adaptive,
                &output,
            )
        }
        AdaptiveCandidate::Bdf1 | AdaptiveCandidate::Bdf2 => {
            let order = if spec.candidate == AdaptiveCandidate::Bdf1 {
                BdfOrder::One
            } else {
                BdfOrder::Two
            };
            integrate_bdf_adaptive_observed(
                &reference.problem,
                reference.t_span,
                &reference.y0,
                &BdfConfig {
                    order,
                    ..BdfConfig::default()
                },
                &adaptive,
                &output,
            )
        }
        AdaptiveCandidate::Radau1 | AdaptiveCandidate::Radau3 => {
            let stages = if spec.candidate == AdaptiveCandidate::Radau1 {
                RadauIiaStages::One
            } else {
                RadauIiaStages::Three
            };
            integrate_radau_adaptive_observed(
                &reference.problem,
                reference.t_span,
                &reference.y0,
                &RadauConfig {
                    stages,
                    ..RadauConfig::default()
                },
                &adaptive,
                &output,
            )
        }
    }
}

fn retained_state_bytes(times: &[f64], states: &[Vec<f64>]) -> u64 {
    let scalars = times.len() + states.iter().map(Vec::len).sum::<usize>();
    (scalars * std::mem::size_of::<f64>()) as u64
}

fn record_id(spec: &AdaptiveRunSpec) -> String {
    format!(
        "{}|{}|rtol{:016x}",
        spec.reference.problem.name,
        spec.candidate.descriptor().candidate_id,
        spec.rtol.to_bits()
    )
}

fn failed_row(
    spec: &AdaptiveRunSpec,
    status: IntegratorRunStatus,
    message: String,
    wall_seconds: f64,
    diagnostics: AdaptiveRunDiagnostics,
    counters: WorkCounters,
) -> AdaptiveRunRow {
    AdaptiveRunRow {
        record_id: record_id(spec),
        candidate_id: spec.candidate.descriptor().candidate_id,
        problem_id: spec.reference.problem.name.clone(),
        rtol: spec.rtol,
        atol: 0.01 * spec.rtol,
        status,
        message,
        errors: None,
        work: IntegratorWorkReport {
            counters,
            internal_steps: 0,
            output_clipped_steps: 0,
            stored_state_bytes: 0,
        },
        diagnostics,
        wall_seconds,
        reference_checksum: spec.reference.reference.provenance.state_checksum.clone(),
        output_grid_id: spec.reference.reference.output_grid.grid_id.clone(),
    }
}

fn run_spec(spec: &AdaptiveRunSpec) -> AdaptiveRunRow {
    let started = Instant::now();
    let result = match execute_candidate(spec) {
        Ok(result) => result,
        Err(error) => {
            return failed_row(
                spec,
                IntegratorRunStatus::SolverFailure,
                error.to_string(),
                started.elapsed().as_secs_f64(),
                AdaptiveRunDiagnostics::default(),
                WorkCounters::default(),
            );
        }
    };
    let wall_seconds = started.elapsed().as_secs_f64();
    let observed = result.observed;
    if !observed.success {
        return failed_row(
            spec,
            IntegratorRunStatus::SolverFailure,
            observed.message,
            wall_seconds,
            result.diagnostics,
            observed.counters,
        );
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
            return failed_row(
                spec,
                IntegratorRunStatus::MissingOutput,
                error.to_string(),
                wall_seconds,
                result.diagnostics,
                observed.counters,
            );
        }
    };
    AdaptiveRunRow {
        record_id: record_id(spec),
        candidate_id: spec.candidate.descriptor().candidate_id,
        problem_id: spec.reference.problem.name.clone(),
        rtol: spec.rtol,
        atol: 0.01 * spec.rtol,
        status: IntegratorRunStatus::Success,
        message: "success".into(),
        errors: Some(errors),
        work: IntegratorWorkReport {
            counters: observed.counters,
            internal_steps: observed.internal_steps as u64,
            output_clipped_steps: observed.output_clipped_steps as u64,
            stored_state_bytes: retained_state_bytes(&observed.t, &observed.y),
        },
        diagnostics: result.diagnostics,
        wall_seconds,
        reference_checksum: spec.reference.reference.provenance.state_checksum.clone(),
        output_grid_id: spec.reference.reference.output_grid.grid_id.clone(),
    }
}

#[derive(Serialize)]
struct ScientificAdaptiveRow<'a> {
    record_id: &'a str,
    candidate_id: &'a str,
    problem_id: &'a str,
    rtol_bits: u64,
    atol_bits: u64,
    status: &'a IntegratorRunStatus,
    message: &'a str,
    errors: &'a Option<GlobalErrorMetrics>,
    work: &'a IntegratorWorkReport,
    diagnostics: &'a AdaptiveRunDiagnostics,
    reference_checksum: &'a str,
    output_grid_id: &'a str,
}

fn scientific_checksum(
    profile: GlobalErrorParetoProfile,
    candidates: &[AdaptiveCandidateDescriptor],
    problems: &[AdaptiveProblemDescriptor],
    tolerances: &[f64],
    runs: &[AdaptiveRunRow],
) -> FairResult<String> {
    let scientific_runs = runs
        .iter()
        .map(|row| ScientificAdaptiveRow {
            record_id: &row.record_id,
            candidate_id: &row.candidate_id,
            problem_id: &row.problem_id,
            rtol_bits: row.rtol.to_bits(),
            atol_bits: row.atol.to_bits(),
            status: &row.status,
            message: &row.message,
            errors: &row.errors,
            work: &row.work,
            diagnostics: &row.diagnostics,
            reference_checksum: &row.reference_checksum,
            output_grid_id: &row.output_grid_id,
        })
        .collect::<Vec<_>>();
    Ok(sha256_hex(&serde_json::to_vec(&(
        profile,
        candidates,
        problems,
        tolerances
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        scientific_runs,
    ))?))
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
    let mut runs = execution.map_ordered(&specs, |spec| Ok(run_spec(spec)))?;
    let scientific_suite_wall_seconds = started.elapsed().as_secs_f64();
    runs.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    let checksum = scientific_checksum(profile, &candidates, &problems, &tolerances, &runs)?;
    Ok(AdaptiveGlobalErrorReport {
        schema: "rodas5p-adaptive-global-error-v1".into(),
        profile,
        execution: AdaptiveScreenExecution {
            threads: execution.threads(),
            backend: execution.backend().into(),
            scientific_suite_wall_seconds,
        },
        candidates,
        problems,
        tolerance_ladder: tolerances,
        runs,
        scientific_checksum: checksum,
    })
}
