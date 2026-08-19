use std::{sync::Arc, time::Instant};

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, DenseOperator, LinearMethod, LinearSolverConfig,
    WorkCounters, dense_fused_phi_action, safe_l2,
};
use serde::{Deserialize, Serialize};

use crate::{
    AdaptiveStepConfig, BdfConfig, ControllerKind, FusedOrthogonalization, FusedPhiKrylovConfig,
    G1TransactionalGateProfile, OdeProblem, OutputSchedule, ParallelExecution, RadauConfig,
    TransactionalQ1Q2Config, complex_dahlquist_problem, fused_phi_action,
    integrate_bdf_adaptive_observed, integrate_pexprb54s4_fused_adaptive_observed,
    integrate_radau_adaptive_observed, integrate_sequential_matrix_free_adaptive_observed,
    integrate_transactional_q1_q2_adaptive_observed, krylov_phi_action,
    oscillatory_prothero_robinson_problem, semilinear_advection_diffusion_problem,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum G3FusedAdaptiveProfile {
    Smoke,
    Canonical,
}

impl G3FusedAdaptiveProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Canonical => "canonical",
        }
    }
}

impl From<G1TransactionalGateProfile> for G3FusedAdaptiveProfile {
    fn from(value: G1TransactionalGateProfile) -> Self {
        match value {
            G1TransactionalGateProfile::Smoke => Self::Smoke,
            G1TransactionalGateProfile::Canonical => Self::Canonical,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G3PhiFusionRow {
    pub case_id: String,
    pub dimension: usize,
    pub nonnormality: f64,
    pub orthogonalization: String,
    pub completed: bool,
    pub relative_error_vs_dense: Option<f64>,
    pub separate_wall_seconds: f64,
    pub fused_wall_seconds: f64,
    pub wall_speedup: Option<f64>,
    pub separate_jvp_vectors: u64,
    pub fused_jvp_vectors: u64,
    pub separate_orthogonalizations: u64,
    pub fused_orthogonalizations: u64,
    pub fused_substeps: Option<usize>,
    pub fused_residual_error_estimate: Option<f64>,
    pub fused_nested_difference_estimate: Option<f64>,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G3AdaptiveRow {
    pub problem_id: String,
    pub candidate_id: String,
    pub rtol: f64,
    pub atol: f64,
    pub success: bool,
    pub failure: Option<String>,
    pub wall_seconds: f64,
    pub endpoint_l2_error: Option<f64>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub maximum_time_error: Option<f64>,
    pub maximum_phi_error: Option<f64>,
    pub maximum_total_error: Option<f64>,
    pub work: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G3FreshJvpRow {
    pub problem_id: String,
    pub dimension: usize,
    pub epsilon: f64,
    pub half_step_disagreement: f64,
    pub relative_error_vs_supplied_jvp: f64,
    pub rhs_equivalent_calls: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G3FusedAdaptiveSummary {
    pub phi_rows: usize,
    pub phi_completed: usize,
    pub adaptive_rows: usize,
    pub adaptive_successes: usize,
    pub explicit_jacobian_builds_in_primary: u64,
    pub direct_factorizations_in_primary: u64,
    pub newton_iterations_in_primary: u64,
    pub legacy_to_fused_phi_action_ratio: f64,
    pub median_fused_phi_wall_speedup: Option<f64>,
    pub maximum_fresh_jvp_half_disagreement: f64,
    pub maximum_fresh_jvp_error_vs_supplied: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G3FusedAdaptiveReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub phi_rows: Vec<G3PhiFusionRow>,
    pub adaptive_rows: Vec<G3AdaptiveRow>,
    pub fresh_jvp_rows: Vec<G3FreshJvpRow>,
    pub summary: G3FusedAdaptiveSummary,
}

pub(crate) struct RuntimeProblem {
    pub(crate) id: String,
    pub(crate) problem: OdeProblem,
    pub(crate) y0: Vec<f64>,
    pub(crate) t_span: (f64, f64),
}

fn quadratic_problem() -> CoreResult<RuntimeProblem> {
    let problem = OdeProblem::new(
        "quadratic",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = 2.0 * y[0] * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        Some(Arc::new(|t| vec![1.0 / (1.0 - t)])),
    )?;
    Ok(RuntimeProblem {
        id: "quadratic".into(),
        problem,
        y0: vec![1.0],
        t_span: (0.0, 0.25),
    })
}

pub(crate) fn build_problems(profile: G3FusedAdaptiveProfile) -> CoreResult<Vec<RuntimeProblem>> {
    let mut out = vec![quadratic_problem()?];
    let (complex, y0) = complex_dahlquist_problem(4, 5.0, 20.0, 0.0)?;
    out.push(RuntimeProblem {
        id: "complex-dahlquist".into(),
        problem: complex,
        y0,
        t_span: (0.0, 0.2),
    });
    let (pr, y0) = oscillatory_prothero_robinson_problem(-1_000.0, 100.0, 20.0, 0.0)?;
    out.push(RuntimeProblem {
        id: "oscillatory-pr".into(),
        problem: pr,
        y0,
        t_span: (0.0, 0.2),
    });
    if profile == G3FusedAdaptiveProfile::Canonical {
        let (ad, y0) = semilinear_advection_diffusion_problem(16, 0.02, 3.0, -1.0, 10.0, 0.0)?;
        out.push(RuntimeProblem {
            id: "advection-diffusion".into(),
            problem: ad,
            y0,
            t_span: (0.0, 0.1),
        });
    }
    Ok(out)
}

pub(crate) fn adaptive_config(rtol: f64, span: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 0.01 * rtol,
        rtol,
        initial_step: (span / 4.0).max(1e-6),
        min_step: 1e-13,
        max_step: span,
        max_attempts: 10_000,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 4.0,
        reject_max_factor: 0.8,
        controller: ControllerKind::Pi,
    }
}

pub(crate) fn phi_config(
    rtol: f64,
    orth: FusedOrthogonalization,
    dimension: usize,
) -> FusedPhiKrylovConfig {
    let incomplete = matches!(orth, FusedOrthogonalization::Incomplete { .. });
    FusedPhiKrylovConfig {
        minimum_dimension: 2.min(dimension.max(1)),
        maximum_dimension: dimension.clamp(2, if incomplete { 24 } else { 32 }),
        dimension_increment: 2,
        relative_tolerance: (0.03 * rtol).max(1e-12),
        absolute_tolerance: (3e-4 * rtol).max(1e-14),
        orthogonalization: orth,
        maximum_substeps: if incomplete { 4 } else { 16 },
    }
}

fn endpoint_error(problem: &OdeProblem, tf: f64, state: &[f64]) -> Option<f64> {
    problem.exact(tf).map(|exact| {
        let n = exact.len();
        safe_l2(
            &state[..n]
                .iter()
                .zip(exact)
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>(),
        )
    })
}

fn run_exponential(
    runtime: &RuntimeProblem,
    rtol: f64,
    orth: FusedOrthogonalization,
    threads: usize,
) -> G3AdaptiveRow {
    let span = runtime.t_span.1 - runtime.t_span.0;
    let adaptive = adaptive_config(rtol, span);
    let output =
        OutputSchedule::new(vec![runtime.t_span.0, runtime.t_span.1]).expect("valid output");
    let (problem, y0) = if runtime.problem.autonomous {
        (
            runtime.problem.jvp_only_clone().expect("JVP clone"),
            runtime.y0.clone(),
        )
    } else {
        let p = runtime
            .problem
            .jvp_only_clone()
            .and_then(|p| p.time_augmented_clone());
        match p {
            Ok(p) => {
                let mut y = runtime.y0.clone();
                y.push(runtime.t_span.0);
                (p, y)
            }
            Err(error) => {
                return failed_row(
                    &runtime.id,
                    &format!("pexprb54s4-fused-{orth:?}-t{threads}"),
                    rtol,
                    0.01 * rtol,
                    error.to_string(),
                );
            }
        }
    };
    let phi = phi_config(rtol, orth, problem.dimension + 4);
    let execution = match if threads == 1 {
        Ok(ParallelExecution::sequential())
    } else {
        ParallelExecution::rayon(threads)
    } {
        Ok(x) => x,
        Err(e) => {
            return failed_row(
                &runtime.id,
                "pexprb54s4-fused",
                rtol,
                0.01 * rtol,
                e.to_string(),
            );
        }
    };
    let start = Instant::now();
    let result = integrate_pexprb54s4_fused_adaptive_observed(
        &problem,
        runtime.t_span,
        &y0,
        &adaptive,
        &output,
        phi,
        &execution,
    );
    let wall = start.elapsed().as_secs_f64();
    let id = format!(
        "pexprb54s4-fused-{}-t{threads}",
        match orth {
            FusedOrthogonalization::FullMgs => "full-mgs",
            FusedOrthogonalization::Incomplete { .. } => "iop2",
        }
    );
    match result {
        Ok(run) => G3AdaptiveRow {
            problem_id: runtime.id.clone(),
            candidate_id: id,
            rtol,
            atol: 0.01 * rtol,
            success: run.observed.success,
            failure: (!run.observed.success).then_some(run.observed.message.clone()),
            wall_seconds: wall,
            endpoint_l2_error: endpoint_error(
                &runtime.problem,
                runtime.t_span.1,
                run.observed.y.last().unwrap(),
            ),
            accepted_steps: run.diagnostics.accepted_steps,
            rejected_steps: run.diagnostics.rejected_steps,
            maximum_time_error: run
                .diagnostics
                .time_error_norms
                .iter()
                .copied()
                .filter(|x| x.is_finite())
                .reduce(f64::max),
            maximum_phi_error: run
                .diagnostics
                .phi_error_norms
                .iter()
                .copied()
                .filter(|x| x.is_finite())
                .reduce(f64::max),
            maximum_total_error: run
                .diagnostics
                .total_error_norms
                .iter()
                .copied()
                .filter(|x| x.is_finite())
                .reduce(f64::max),
            work: run.observed.counters,
        },
        Err(error) => failed_row(
            &runtime.id,
            &id,
            rtol,
            0.01 * rtol,
            format!("{error}; wall={wall}"),
        ),
    }
}

fn failed_row(
    problem: &str,
    candidate: &str,
    rtol: f64,
    atol: f64,
    failure: String,
) -> G3AdaptiveRow {
    G3AdaptiveRow {
        problem_id: problem.into(),
        candidate_id: candidate.into(),
        rtol,
        atol,
        success: false,
        failure: Some(failure),
        wall_seconds: 0.0,
        endpoint_l2_error: None,
        accepted_steps: 0,
        rejected_steps: 0,
        maximum_time_error: None,
        maximum_phi_error: None,
        maximum_total_error: None,
        work: WorkCounters::default(),
    }
}

fn run_comparator(runtime: &RuntimeProblem, rtol: f64, candidate: &str) -> G3AdaptiveRow {
    let span = runtime.t_span.1 - runtime.t_span.0;
    let adaptive = adaptive_config(rtol, span);
    let output =
        OutputSchedule::new(vec![runtime.t_span.0, runtime.t_span.1]).expect("valid output");
    let start = Instant::now();
    let result: CoreResult<(crate::ObservedIntegrationResult, usize, usize)> = (|| match candidate {
        "protected-jf-rodas5p" => {
            let problem = runtime.problem.jvp_only_clone()?;
            let linear = LinearSolverConfig {
                method: LinearMethod::Gmres,
                rtol: 1e-10,
                atol: 1e-12,
                restart: 32,
                maxiter: 256,
                ..LinearSolverConfig::default()
            };
            integrate_sequential_matrix_free_adaptive_observed(
                &problem,
                runtime.t_span,
                &runtime.y0,
                &linear,
                &adaptive,
                &output,
            )
            .map(|run| {
                (
                    run.observed,
                    run.diagnostics.accepted_macro_steps,
                    run.diagnostics.rejected_macro_steps,
                )
            })
        }
        "held-g1-q1-q2" => {
            let problem = runtime.problem.jvp_only_clone()?;
            integrate_transactional_q1_q2_adaptive_observed(
                &problem,
                runtime.t_span,
                &runtime.y0,
                &TransactionalQ1Q2Config::default(),
                &adaptive,
                &output,
            )
            .map(|run| {
                (
                    run.observed,
                    run.diagnostics.accepted_macro_steps,
                    run.diagnostics.rejected_macro_steps,
                )
            })
        }
        "frozen-bdf2" => integrate_bdf_adaptive_observed(
            &runtime.problem,
            runtime.t_span,
            &runtime.y0,
            &BdfConfig::default(),
            &adaptive,
            &output,
        )
        .map(|run| {
            (
                run.observed,
                run.diagnostics.accepted_macro_steps,
                run.diagnostics.rejected_macro_steps,
            )
        }),
        "frozen-radau-iia3" => integrate_radau_adaptive_observed(
            &runtime.problem,
            runtime.t_span,
            &runtime.y0,
            &RadauConfig::default(),
            &adaptive,
            &output,
        )
        .map(|run| {
            (
                run.observed,
                run.diagnostics.accepted_macro_steps,
                run.diagnostics.rejected_macro_steps,
            )
        }),
        _ => Err(CoreError::InvalidInput("unknown G3 comparator".into())),
    })();
    let wall = start.elapsed().as_secs_f64();
    match result {
        Ok((observed, accepted, rejected)) => G3AdaptiveRow {
            problem_id: runtime.id.clone(),
            candidate_id: candidate.into(),
            rtol,
            atol: 0.01 * rtol,
            success: observed.success,
            failure: (!observed.success).then_some(observed.message.clone()),
            wall_seconds: wall,
            endpoint_l2_error: endpoint_error(
                &runtime.problem,
                runtime.t_span.1,
                observed.y.last().unwrap(),
            ),
            accepted_steps: accepted,
            rejected_steps: rejected,
            maximum_time_error: None,
            maximum_phi_error: None,
            maximum_total_error: None,
            work: observed.counters,
        },
        Err(error) => failed_row(
            &runtime.id,
            candidate,
            rtol,
            0.01 * rtol,
            format!("{error}; wall={wall}"),
        ),
    }
}

fn nonnormal_matrix(n: usize, eta: f64) -> CoreResult<DenseMatrix> {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -2.0 - (i as f64) / (n as f64);
        if i + 1 < n {
            a[(i, i + 1)] = eta * 8.0;
        }
    }
    Ok(a)
}

fn phi_fusion_rows(profile: G3FusedAdaptiveProfile) -> CoreResult<Vec<G3PhiFusionRow>> {
    let dimensions: &[usize] = match profile {
        G3FusedAdaptiveProfile::Smoke => &[16],
        G3FusedAdaptiveProfile::Canonical => &[16, 64, 128],
    };
    let etas: &[f64] = match profile {
        G3FusedAdaptiveProfile::Smoke => &[0.5],
        G3FusedAdaptiveProfile::Canonical => &[0.0, 0.5, 0.9],
    };
    let mut rows = Vec::new();
    for &n in dimensions {
        for &eta in etas {
            let a = nonnormal_matrix(n, eta)?;
            let operator = Arc::new(DenseOperator::new(a.clone())?);
            let vectors = (0..5)
                .map(|k| {
                    (0..n)
                        .map(|i| (((i + 1) * (k + 2)) as f64 * 0.17).sin() / (k + 1) as f64)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let scale = 0.08;
            let dense = dense_fused_phi_action(&a, scale, &vectors)?;
            for orth in [
                FusedOrthogonalization::FullMgs,
                FusedOrthogonalization::Incomplete { length: 2 },
            ] {
                let mut separate_work = WorkCounters::default();
                let separate_start = Instant::now();
                let mut separate = vec![0.0; n];
                let mut separate_ok = true;
                for (k, vector) in vectors.iter().enumerate() {
                    match krylov_phi_action(
                        operator.clone(),
                        scale,
                        k,
                        vector,
                        crate::ExponentialKrylovConfig {
                            minimum_dimension: 2,
                            maximum_dimension: (n + 4).min(32),
                            dimension_increment: 2,
                            relative_tolerance: 1e-10,
                            absolute_tolerance: 1e-13,
                            reorthogonalize: true,
                        },
                        &mut separate_work,
                    ) {
                        Ok(report) if report.converged => {
                            let factor = scale.powi(k as i32);
                            for (out, value) in separate.iter_mut().zip(report.value) {
                                *out += factor * value;
                            }
                        }
                        _ => {
                            separate_ok = false;
                            break;
                        }
                    }
                }
                let separate_wall = separate_start.elapsed().as_secs_f64();
                let mut fused_work = WorkCounters::default();
                let fused_start = Instant::now();
                let fused = fused_phi_action(
                    operator.clone(),
                    scale,
                    &vectors,
                    FusedPhiKrylovConfig {
                        minimum_dimension: 2,
                        maximum_dimension: (n + 4).min(32),
                        dimension_increment: 2,
                        relative_tolerance: 1e-10,
                        absolute_tolerance: 1e-13,
                        orthogonalization: orth,
                        maximum_substeps: 16,
                    },
                    &mut fused_work,
                );
                let fused_wall = fused_start.elapsed().as_secs_f64();
                let (
                    completed,
                    error,
                    substeps,
                    residual_estimate,
                    nested_difference_estimate,
                    failure,
                ) = match fused {
                    Ok(report) if report.converged && separate_ok => {
                        let defect = report
                            .value
                            .iter()
                            .zip(&dense)
                            .map(|(x, y)| x - y)
                            .collect::<Vec<_>>();
                        (
                            true,
                            Some(safe_l2(&defect) / safe_l2(&dense).max(1e-300)),
                            Some(report.substeps),
                            Some(report.error_estimate),
                            Some(report.nested_difference_estimate),
                            None,
                        )
                    }
                    Ok(report) => (
                        false,
                        None,
                        Some(report.substeps),
                        Some(report.error_estimate),
                        Some(report.nested_difference_estimate),
                        Some("fused or separate action did not converge".into()),
                    ),
                    Err(error) => (false, None, None, None, None, Some(error.to_string())),
                };
                rows.push(G3PhiFusionRow {
                    case_id: format!("n{n}-eta{eta}"),
                    dimension: n,
                    nonnormality: eta,
                    orthogonalization: format!("{orth:?}"),
                    completed,
                    relative_error_vs_dense: error,
                    separate_wall_seconds: separate_wall,
                    fused_wall_seconds: fused_wall,
                    wall_speedup: (completed && fused_wall > 0.0)
                        .then_some(separate_wall / fused_wall),
                    separate_jvp_vectors: separate_work.jvp_vectors,
                    fused_jvp_vectors: fused_work.jvp_vectors,
                    separate_orthogonalizations: separate_work.orthogonalization_inner_products,
                    fused_orthogonalizations: fused_work.orthogonalization_inner_products,
                    fused_substeps: substeps,
                    fused_residual_error_estimate: residual_estimate,
                    fused_nested_difference_estimate: nested_difference_estimate,
                    failure,
                });
            }
        }
    }
    Ok(rows)
}

fn fresh_jvp_rows(problems: &[RuntimeProblem]) -> CoreResult<Vec<G3FreshJvpRow>> {
    let mut rows = Vec::new();
    for runtime in problems {
        let n = runtime.problem.dimension;
        let y = &runtime.y0;
        let t = runtime.t_span.0;
        let direction = (0..n)
            .map(|i| (((i + 1) as f64) * 0.37).sin())
            .collect::<Vec<_>>();
        let epsilon =
            f64::EPSILON.sqrt() * (1.0 + safe_l2(y)) / safe_l2(&direction).max(f64::MIN_POSITIVE);
        let mut counters = WorkCounters::default();
        let f0 = runtime.problem.eval_rhs(t, y, &mut counters)?;
        let evaluate = |eps: f64, counters: &mut WorkCounters| -> CoreResult<Vec<f64>> {
            let pert = y
                .iter()
                .zip(&direction)
                .map(|(a, v)| a + eps * v)
                .collect::<Vec<_>>();
            let f = runtime.problem.eval_rhs(t, &pert, counters)?;
            Ok(f.iter().zip(&f0).map(|(a, b)| (a - b) / eps).collect())
        };
        let j1 = evaluate(epsilon, &mut counters)?;
        let j2 = evaluate(0.5 * epsilon, &mut counters)?;
        let op = runtime.problem.linearize_matrix_free(t, y)?;
        let mut exact = vec![0.0; n];
        op.apply(&direction, &mut exact)?;
        let half = j1.iter().zip(&j2).map(|(a, b)| a - b).collect::<Vec<_>>();
        let err = j2
            .iter()
            .zip(&exact)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>();
        rows.push(G3FreshJvpRow {
            problem_id: runtime.id.clone(),
            dimension: n,
            epsilon,
            half_step_disagreement: safe_l2(&half) / safe_l2(&j2).max(1e-300),
            relative_error_vs_supplied_jvp: safe_l2(&err) / safe_l2(&exact).max(1e-300),
            rhs_equivalent_calls: counters.rhs_evaluations,
        });
    }
    Ok(rows)
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    Some(if n % 2 == 1 {
        values[n / 2]
    } else {
        0.5 * (values[n / 2 - 1] + values[n / 2])
    })
}

pub fn run_g3_fused_adaptive_gate(
    profile: G3FusedAdaptiveProfile,
) -> CoreResult<G3FusedAdaptiveReport> {
    let problems = build_problems(profile)?;
    let rtols: &[f64] = match profile {
        G3FusedAdaptiveProfile::Smoke => &[1e-5],
        G3FusedAdaptiveProfile::Canonical => &[1e-4, 1e-6, 1e-8],
    };
    let phi_rows = phi_fusion_rows(profile)?;
    let fresh_jvp_rows = fresh_jvp_rows(&problems)?;
    let mut adaptive_rows = Vec::new();
    for problem in &problems {
        for &rtol in rtols {
            adaptive_rows.push(run_exponential(
                problem,
                rtol,
                FusedOrthogonalization::FullMgs,
                1,
            ));
            adaptive_rows.push(run_exponential(
                problem,
                rtol,
                FusedOrthogonalization::FullMgs,
                4,
            ));
            adaptive_rows.push(run_exponential(
                problem,
                rtol,
                FusedOrthogonalization::Incomplete { length: 2 },
                1,
            ));
            adaptive_rows.push(run_exponential(
                problem,
                rtol,
                FusedOrthogonalization::Incomplete { length: 2 },
                4,
            ));
            for candidate in [
                "protected-jf-rodas5p",
                "held-g1-q1-q2",
                "frozen-bdf2",
                "frozen-radau-iia3",
            ] {
                adaptive_rows.push(run_comparator(problem, rtol, candidate));
            }
        }
    }
    let primary = adaptive_rows
        .iter()
        .filter(|row| row.candidate_id == "pexprb54s4-fused-full-mgs-t1");
    let explicit_jacobian_builds_in_primary = primary.clone().map(|r| r.work.jacobian_builds).sum();
    let direct_factorizations_in_primary =
        primary.clone().map(|r| r.work.direct_factorizations).sum();
    let newton_iterations_in_primary = primary.map(|r| r.work.nonlinear_iterations).sum();
    let summary = G3FusedAdaptiveSummary {
        phi_rows: phi_rows.len(),
        phi_completed: phi_rows.iter().filter(|r| r.completed).count(),
        adaptive_rows: adaptive_rows.len(),
        adaptive_successes: adaptive_rows.iter().filter(|r| r.success).count(),
        explicit_jacobian_builds_in_primary,
        direct_factorizations_in_primary,
        newton_iterations_in_primary,
        legacy_to_fused_phi_action_ratio: 15.0 / 5.0,
        median_fused_phi_wall_speedup: median(
            phi_rows.iter().filter_map(|r| r.wall_speedup).collect(),
        ),
        maximum_fresh_jvp_half_disagreement: fresh_jvp_rows
            .iter()
            .map(|r| r.half_step_disagreement)
            .fold(0.0, f64::max),
        maximum_fresh_jvp_error_vs_supplied: fresh_jvp_rows
            .iter()
            .map(|r| r.relative_error_vs_supplied_jvp)
            .fold(0.0, f64::max),
    };
    let status = if summary.adaptive_successes == summary.adaptive_rows
        && explicit_jacobian_builds_in_primary == 0
        && direct_factorizations_in_primary == 0
        && newton_iterations_in_primary == 0
    {
        "pass"
    } else {
        "hold"
    };
    Ok(G3FusedAdaptiveReport {
        schema: "generic-parallel-exponential-g3-v1",
        status,
        profile: profile.as_str(),
        phi_rows,
        adaptive_rows,
        fresh_jvp_rows,
        summary,
    })
}
