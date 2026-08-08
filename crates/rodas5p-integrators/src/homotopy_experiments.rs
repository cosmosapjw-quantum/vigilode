use rodas5p_core::{
    CoreResult, LinearMethod, LinearSolverConfig, WorkCounters, error_scale, safe_l2, wrms,
};
use serde::Serialize;

use crate::{
    BlockMethod, HomotopyPathConfig, HomotopyPredictor, HomotopyStepConfig, OdeProblem,
    PredictorKind, SabrConfig, StageHistory, constant_affine_mass_problem, flatten, homotopy_step,
    manufactured_mass_nonlinear_problem, manufactured_vector_problem, prothero_robinson_problem,
    sabr_step, scalar_linear_problem, sequential_step,
};

const SCREEN_ATOL: f64 = 1e-7;
const SCREEN_RTOL: f64 = 1e-6;
const DEFECT_BUDGET: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HomotopyExperimentProfile {
    Smoke,
    Canonical,
}

impl HomotopyExperimentProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Canonical => "canonical",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyExperimentCase {
    pub case_id: String,
    pub family: String,
    pub dimension: usize,
    pub t: f64,
    pub h: f64,
    pub stiffness: Option<f64>,
    pub nonlinearity: Option<f64>,
    pub nonnormality: Option<f64>,
    pub lambda: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyControlRow {
    pub case_id: String,
    pub method: String,
    pub accepted: bool,
    pub used_fallback: bool,
    pub embedded_error: Option<f64>,
    pub exact_error_l2: Option<f64>,
    pub failure: Option<String>,
    pub counters: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyCandidateRow {
    pub case_id: String,
    pub theta: f64,
    pub q: usize,
    pub path_rounds: usize,
    pub predictor: HomotopyPredictor,
    pub corrections_per_point: usize,
    pub fast_accepted: bool,
    pub used_fallback: bool,
    pub final_accepted: bool,
    pub output_wrms: Option<f64>,
    pub embedded_error: Option<f64>,
    pub combined_error: Option<f64>,
    pub oracle_output_wrms: Option<f64>,
    pub path_endpoint_target_residual_norm: Option<f64>,
    pub stage_difference_l2: Option<f64>,
    pub exact_error_l2: Option<f64>,
    pub false_accept: bool,
    pub fallback_reason: Option<String>,
    pub failure: Option<String>,
    pub work: Option<crate::HomotopyWorkLedger>,
    pub counters: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyQSummary {
    pub q: usize,
    pub rows: usize,
    pub fast_accepts: usize,
    pub false_accepts: usize,
    pub fallbacks: usize,
    pub failures: usize,
    pub median_oracle_output_wrms: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyOrderScreenRow {
    pub problem_id: String,
    pub method: String,
    pub theta: Option<f64>,
    pub q: Option<usize>,
    pub path_rounds: Option<usize>,
    pub predictor: Option<HomotopyPredictor>,
    pub corrections_per_point: Option<usize>,
    pub h: f64,
    pub steps: usize,
    pub error_l2: f64,
    pub observed_order: Option<f64>,
    pub fast_accepts: usize,
    pub fallbacks: usize,
    pub all_fast: bool,
    pub counters: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyExperimentSummary {
    pub cases: usize,
    pub controls: usize,
    pub candidates: usize,
    pub fast_accepts: usize,
    pub false_accepts: usize,
    pub fallbacks: usize,
    pub failures: usize,
    pub q_summary: Vec<HomotopyQSummary>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyExperimentReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub atol: f64,
    pub rtol: f64,
    pub output_wrms_budget: f64,
    pub cases: Vec<HomotopyExperimentCase>,
    pub controls: Vec<HomotopyControlRow>,
    pub candidates: Vec<HomotopyCandidateRow>,
    pub order_screens: Vec<HomotopyOrderScreenRow>,
    pub summary: HomotopyExperimentSummary,
}

struct RuntimeCase {
    descriptor: HomotopyExperimentCase,
    problem: OdeProblem,
    y0: Vec<f64>,
}

fn exact_error(problem: &OdeProblem, time: f64, computed: &[f64]) -> Option<f64> {
    problem
        .exact(time)
        .filter(|exact| exact.len() == computed.len())
        .map(|exact| {
            safe_l2(
                &computed
                    .iter()
                    .zip(exact)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|value| value.is_finite())
}

fn output_from_stages(
    problem_dimension: usize,
    y0: &[f64],
    weights: &[f64],
    stages: &[Vec<f64>],
) -> Vec<f64> {
    let mut out = y0.to_vec();
    for (weight, stage) in weights.iter().zip(stages) {
        for component in 0..problem_dimension {
            out[component] += weight * stage[component];
        }
    }
    out
}

fn oracle_output_wrms(y0: &[f64], oracle: &[f64], candidate: &[f64]) -> CoreResult<f64> {
    let scale = error_scale(y0, oracle, &[SCREEN_ATOL], SCREEN_RTOL)?;
    let difference: Vec<f64> = candidate.iter().zip(oracle).map(|(a, b)| a - b).collect();
    wrms(&difference, &scale)
}

fn build_cases(profile: HomotopyExperimentProfile) -> CoreResult<Vec<RuntimeCase>> {
    let mut cases = Vec::new();

    let (problem, y0, _mass, _jacobian) = constant_affine_mass_problem();
    cases.push(RuntimeCase {
        descriptor: HomotopyExperimentCase {
            case_id: "affine-mass-t0.2-h0.03".into(),
            family: "affine-mass".into(),
            dimension: 2,
            t: 0.2,
            h: 0.03,
            stiffness: None,
            nonlinearity: Some(0.0),
            nonnormality: None,
            lambda: None,
        },
        problem,
        y0,
    });

    let scalar_grid: &[(f64, f64)] = match profile {
        HomotopyExperimentProfile::Smoke => &[(-100.0, 0.005)],
        HomotopyExperimentProfile::Canonical => &[
            (-100.0, 0.001),
            (-100.0, 0.01),
            (-10_000.0, 0.001),
            (-10_000.0, 0.01),
        ],
    };
    for &(lambda, h) in scalar_grid {
        let (problem, y0) = scalar_linear_problem(lambda, 1.0);
        cases.push(RuntimeCase {
            descriptor: HomotopyExperimentCase {
                case_id: format!("scalar-l{lambda:.0}-h{h:.3e}"),
                family: "scalar-linear".into(),
                dimension: 1,
                t: 0.0,
                h,
                stiffness: Some(lambda.abs()),
                nonlinearity: Some(0.0),
                nonnormality: Some(0.0),
                lambda: Some(lambda),
            },
            problem,
            y0,
        });
    }

    let pr_grid: Vec<(f64, f64, f64)> = match profile {
        HomotopyExperimentProfile::Smoke => vec![(-1_000.0, 100.0, 0.005)],
        HomotopyExperimentProfile::Canonical => [-100.0, -10_000.0, -1_000_000.0]
            .into_iter()
            .flat_map(|lambda| {
                [0.0, 1_000.0]
                    .into_iter()
                    .flat_map(move |mu| [0.001, 0.01].into_iter().map(move |h| (lambda, mu, h)))
            })
            .collect(),
    };
    for (lambda, mu, h) in pr_grid {
        let (problem, y0) = prothero_robinson_problem(lambda, mu, 0.0);
        cases.push(RuntimeCase {
            descriptor: HomotopyExperimentCase {
                case_id: format!("pr-l{lambda:.0}-m{mu:.0}-h{h:.3e}"),
                family: "prothero-robinson".into(),
                dimension: 1,
                t: 0.0,
                h,
                stiffness: Some(lambda.abs()),
                nonlinearity: Some(mu),
                nonnormality: Some(0.0),
                lambda: Some(lambda),
            },
            problem,
            y0,
        });
    }

    let mass_grid: Vec<(f64, f64, f64, f64)> = match profile {
        HomotopyExperimentProfile::Smoke => vec![(100.0, 10.0, 0.4, 0.005)],
        HomotopyExperimentProfile::Canonical => [100.0, 10_000.0]
            .into_iter()
            .flat_map(|stiffness| {
                [1.0, 1_000.0].into_iter().flat_map(move |nonlinearity| {
                    [0.2, 0.9].into_iter().flat_map(move |nonnormality| {
                        [0.001, 0.01]
                            .into_iter()
                            .map(move |h| (stiffness, nonlinearity, nonnormality, h))
                    })
                })
            })
            .collect(),
    };
    for (stiffness, nonlinearity, nonnormality, h) in mass_grid {
        let (problem, y0, _mass, _linear) =
            manufactured_mass_nonlinear_problem(stiffness, nonlinearity, nonnormality, 0.0)?;
        cases.push(RuntimeCase {
            descriptor: HomotopyExperimentCase {
                case_id: format!(
                    "mm-s{stiffness:.0}-m{nonlinearity:.0}-eta{nonnormality:.1}-h{h:.3e}"
                ),
                family: "manufactured-mass".into(),
                dimension: 2,
                t: 0.0,
                h,
                stiffness: Some(stiffness),
                nonlinearity: Some(nonlinearity),
                nonnormality: Some(nonnormality),
                lambda: None,
            },
            problem,
            y0,
        });
    }

    let manufactured_grid: Vec<(f64, f64, f64, f64)> = match profile {
        HomotopyExperimentProfile::Smoke => vec![(100.0, 10.0, 0.2, 0.005)],
        HomotopyExperimentProfile::Canonical => [100.0, 10_000.0]
            .into_iter()
            .flat_map(|stiffness| {
                [1.0, 1_000.0].into_iter().flat_map(move |nonlinearity| {
                    [0.0, 0.2, 0.9].into_iter().flat_map(move |nonnormality| {
                        [0.001, 0.01]
                            .into_iter()
                            .map(move |h| (stiffness, nonlinearity, nonnormality, h))
                    })
                })
            })
            .collect(),
    };
    for (stiffness, nonlinearity, nonnormality, h) in manufactured_grid {
        let (problem, y0) =
            manufactured_vector_problem(4, stiffness, nonlinearity, nonnormality, 0.0)?;
        cases.push(RuntimeCase {
            descriptor: HomotopyExperimentCase {
                case_id: format!(
                    "mv-s{stiffness:.0}-m{nonlinearity:.0}-eta{nonnormality:.1}-h{h:.3e}"
                ),
                family: "manufactured-vector".into(),
                dimension: 4,
                t: 0.0,
                h,
                stiffness: Some(stiffness),
                nonlinearity: Some(nonlinearity),
                nonnormality: Some(nonnormality),
                lambda: None,
            },
            problem,
            y0,
        });
    }

    Ok(cases)
}

fn configurations(profile: HomotopyExperimentProfile) -> CoreResult<Vec<HomotopyPathConfig>> {
    let thetas: &[f64] = match profile {
        HomotopyExperimentProfile::Smoke => &[0.0, 1.0],
        HomotopyExperimentProfile::Canonical => &[0.0, 0.5, 1.0],
    };
    let qs: &[usize] = match profile {
        HomotopyExperimentProfile::Smoke => &[0, 2, 7],
        HomotopyExperimentProfile::Canonical => &[0, 1, 2, 7],
    };
    let rounds: &[usize] = match profile {
        HomotopyExperimentProfile::Smoke => &[2],
        HomotopyExperimentProfile::Canonical => &[2, 3, 4],
    };
    let predictors: &[HomotopyPredictor] = match profile {
        HomotopyExperimentProfile::Smoke => &[HomotopyPredictor::Euler],
        HomotopyExperimentProfile::Canonical => {
            &[HomotopyPredictor::Euler, HomotopyPredictor::AdamsBashforth2]
        }
    };
    let corrections: &[usize] = &[0, 1];

    let mut out = Vec::new();
    for &theta in thetas {
        for &q in qs {
            for &path_rounds in rounds {
                for &predictor in predictors {
                    for &corrections_per_point in corrections {
                        out.push(HomotopyPathConfig::new(
                            theta,
                            q,
                            path_rounds,
                            predictor,
                            corrections_per_point,
                        )?);
                    }
                }
            }
        }
    }
    Ok(out)
}

fn control_rows(case: &RuntimeCase) -> (Vec<HomotopyControlRow>, Option<crate::StepResult>) {
    let fallback = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let mut rows = Vec::new();

    let mut sequential_counters = WorkCounters::default();
    let sequential = sequential_step(
        &case.problem,
        case.descriptor.t,
        &case.y0,
        case.descriptor.h,
        &fallback,
        None,
        SCREEN_ATOL,
        SCREEN_RTOL,
        true,
        &mut sequential_counters,
    );
    let sequential_step_result = sequential.as_ref().ok().cloned();
    rows.push(match sequential {
        Ok(step) => HomotopyControlRow {
            case_id: case.descriptor.case_id.clone(),
            method: "sequential-direct".into(),
            accepted: step.accepted,
            used_fallback: step.used_fallback,
            embedded_error: Some(step.error_norm),
            exact_error_l2: exact_error(
                &case.problem,
                case.descriptor.t + case.descriptor.h,
                &step.y_new,
            ),
            failure: None,
            counters: step.counters,
        },
        Err(error) => HomotopyControlRow {
            case_id: case.descriptor.case_id.clone(),
            method: "sequential-direct".into(),
            accepted: false,
            used_fallback: false,
            embedded_error: None,
            exact_error_l2: None,
            failure: Some(error.to_string()),
            counters: sequential_counters,
        },
    });

    let sabr_config = SabrConfig {
        predictor: PredictorKind::Zero,
        max_iterations: 3,
        block_method: BlockMethod::Nilpotent,
        defect_budget_fraction: DEFECT_BUDGET,
        ..SabrConfig::default()
    };
    let mut history = StageHistory::default();
    let mut sabr_counters = WorkCounters::default();
    let sabr = sabr_step(
        &case.problem,
        case.descriptor.t,
        &case.y0,
        case.descriptor.h,
        &sabr_config,
        Some(&fallback),
        &mut history,
        None,
        SCREEN_ATOL,
        SCREEN_RTOL,
        true,
        &mut sabr_counters,
    );
    rows.push(match sabr {
        Ok(step) => HomotopyControlRow {
            case_id: case.descriptor.case_id.clone(),
            method: "sabr-nilpotent-3".into(),
            accepted: step.accepted,
            used_fallback: step.used_fallback,
            embedded_error: Some(step.error_norm),
            exact_error_l2: exact_error(
                &case.problem,
                case.descriptor.t + case.descriptor.h,
                &step.y_new,
            ),
            failure: None,
            counters: step.counters,
        },
        Err(error) => HomotopyControlRow {
            case_id: case.descriptor.case_id.clone(),
            method: "sabr-nilpotent-3".into(),
            accepted: false,
            used_fallback: false,
            embedded_error: None,
            exact_error_l2: None,
            failure: Some(error.to_string()),
            counters: sabr_counters,
        },
    });

    (rows, sequential_step_result)
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        0.5 * (values[middle - 1] + values[middle])
    } else {
        values[middle]
    })
}

#[derive(Clone)]
enum OrderScreenMethod {
    Sequential,
    Homotopy {
        label: &'static str,
        path: HomotopyPathConfig,
    },
}

impl OrderScreenMethod {
    fn label(&self) -> &'static str {
        match self {
            Self::Sequential => "sequential-direct",
            Self::Homotopy { label, .. } => label,
        }
    }

    fn path(&self) -> Option<&HomotopyPathConfig> {
        match self {
            Self::Sequential => None,
            Self::Homotopy { path, .. } => Some(path),
        }
    }
}

fn order_screen_methods() -> CoreResult<Vec<OrderScreenMethod>> {
    Ok(vec![
        OrderScreenMethod::Sequential,
        OrderScreenMethod::Homotopy {
            label: "homotopy-theta0-q0-r2-ab2-c0",
            path: HomotopyPathConfig::new(0.0, 0, 2, HomotopyPredictor::AdamsBashforth2, 0)?,
        },
        OrderScreenMethod::Homotopy {
            label: "homotopy-theta0-q1-r2-ab2-c0",
            path: HomotopyPathConfig::new(0.0, 1, 2, HomotopyPredictor::AdamsBashforth2, 0)?,
        },
        OrderScreenMethod::Homotopy {
            label: "homotopy-theta0-q2-r2-ab2-c0",
            path: HomotopyPathConfig::new(0.0, 2, 2, HomotopyPredictor::AdamsBashforth2, 0)?,
        },
        OrderScreenMethod::Homotopy {
            label: "homotopy-theta1-q2-r2-ab2-c1",
            path: HomotopyPathConfig::new(1.0, 2, 2, HomotopyPredictor::AdamsBashforth2, 1)?,
        },
        OrderScreenMethod::Homotopy {
            label: "homotopy-theta1-q7-r2-ab2-c1",
            path: HomotopyPathConfig::new(1.0, 7, 2, HomotopyPredictor::AdamsBashforth2, 1)?,
        },
    ])
}

fn integrate_order_screen_method(
    problem: &OdeProblem,
    y0: &[f64],
    final_time: f64,
    h: f64,
    method: &OrderScreenMethod,
) -> CoreResult<(Vec<f64>, usize, usize, usize, WorkCounters)> {
    let direct = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let mut counters = WorkCounters::default();
    let mut time = 0.0;
    let mut state = y0.to_vec();
    let mut steps = 0_usize;
    let mut fast_accepts = 0_usize;
    let mut fallbacks = 0_usize;

    while time < final_time - 10.0 * f64::EPSILON * final_time.max(1.0) {
        let step_size = h.min(final_time - time);
        match method {
            OrderScreenMethod::Sequential => {
                let report = sequential_step(
                    problem,
                    time,
                    &state,
                    step_size,
                    &direct,
                    None,
                    SCREEN_ATOL,
                    SCREEN_RTOL,
                    true,
                    &mut counters,
                )?;
                state = report.y_new;
                time = report.t_new;
            }
            OrderScreenMethod::Homotopy { path, .. } => {
                let step_config = HomotopyStepConfig::new(path.clone(), DEFECT_BUDGET)?;
                let report = homotopy_step(
                    problem,
                    time,
                    &state,
                    step_size,
                    &step_config,
                    Some(&direct),
                    None,
                    SCREEN_ATOL,
                    SCREEN_RTOL,
                    true,
                    &mut counters,
                )?;
                fast_accepts += usize::from(report.fast_accepted);
                fallbacks += usize::from(report.step.used_fallback);
                state = report.step.y_new;
                time = report.step.t_new;
            }
        }
        steps += 1;
    }

    Ok((state, steps, fast_accepts, fallbacks, counters))
}

fn run_order_screens(
    profile: HomotopyExperimentProfile,
) -> CoreResult<Vec<HomotopyOrderScreenRow>> {
    let problem_id = "manufactured-vector-n6-s80-m10-eta0";
    let (problem, y0) = manufactured_vector_problem(6, 80.0, 10.0, 0.0, 0.0)?;
    let final_time = 0.2;
    let step_sizes: &[f64] = match profile {
        HomotopyExperimentProfile::Smoke => &[0.04, 0.02],
        HomotopyExperimentProfile::Canonical => &[0.04, 0.02, 0.01, 0.005],
    };
    let methods = order_screen_methods()?;
    let exact = problem.exact(final_time).ok_or_else(|| {
        rodas5p_core::CoreError::InvalidInput(
            "order screen problem must provide an exact solution".into(),
        )
    })?;
    let mut rows = Vec::new();

    for method in methods {
        let mut previous: Option<(f64, f64)> = None;
        for &h in step_sizes {
            let (state, steps, fast_accepts, fallbacks, counters) =
                integrate_order_screen_method(&problem, &y0, final_time, h, &method)?;
            let error = safe_l2(
                &state
                    .iter()
                    .zip(&exact)
                    .map(|(computed, reference)| computed - reference)
                    .collect::<Vec<_>>(),
            );
            if !error.is_finite() {
                return Err(rodas5p_core::CoreError::NonFinite(
                    "order screen endpoint error contains NaN/Inf".into(),
                ));
            }
            let observed_order = previous.and_then(|(previous_h, previous_error)| {
                if error > 0.0 && previous_error > 0.0 && previous_h > h {
                    Some((previous_error / error).ln() / (previous_h / h).ln())
                        .filter(|value| value.is_finite())
                } else {
                    None
                }
            });
            let path = method.path();
            rows.push(HomotopyOrderScreenRow {
                problem_id: problem_id.into(),
                method: method.label().into(),
                theta: path.map(HomotopyPathConfig::theta),
                q: path.map(HomotopyPathConfig::q),
                path_rounds: path.map(HomotopyPathConfig::path_rounds),
                predictor: path.map(HomotopyPathConfig::predictor),
                corrections_per_point: path.map(HomotopyPathConfig::corrections_per_point),
                h,
                steps,
                error_l2: error,
                observed_order,
                fast_accepts,
                fallbacks,
                all_fast: path.is_some() && fast_accepts == steps,
                counters,
            });
            previous = Some((h, error));
        }
    }

    Ok(rows)
}

pub fn run_homotopy_experiment_screen(
    profile: HomotopyExperimentProfile,
) -> CoreResult<HomotopyExperimentReport> {
    let runtime_cases = build_cases(profile)?;
    let configs = configurations(profile)?;
    let descriptors: Vec<HomotopyExperimentCase> = runtime_cases
        .iter()
        .map(|case| case.descriptor.clone())
        .collect();
    let mut controls = Vec::new();
    let mut candidates = Vec::new();

    for case in &runtime_cases {
        let (case_controls, sequential) = control_rows(case);
        controls.extend(case_controls);
        let Some(sequential) = sequential else {
            for config in &configs {
                candidates.push(HomotopyCandidateRow {
                    case_id: case.descriptor.case_id.clone(),
                    theta: config.theta(),
                    q: config.q(),
                    path_rounds: config.path_rounds(),
                    predictor: config.predictor(),
                    corrections_per_point: config.corrections_per_point(),
                    fast_accepted: false,
                    used_fallback: false,
                    final_accepted: false,
                    output_wrms: None,
                    embedded_error: None,
                    combined_error: None,
                    oracle_output_wrms: None,
                    path_endpoint_target_residual_norm: None,
                    stage_difference_l2: None,
                    exact_error_l2: None,
                    false_accept: false,
                    fallback_reason: None,
                    failure: Some("sequential oracle unavailable".into()),
                    work: None,
                    counters: WorkCounters::default(),
                });
            }
            continue;
        };

        for path_config in &configs {
            let step_config = HomotopyStepConfig::new(path_config.clone(), DEFECT_BUDGET)?;
            let mut counters = WorkCounters::default();
            let result = homotopy_step(
                &case.problem,
                case.descriptor.t,
                &case.y0,
                case.descriptor.h,
                &step_config,
                None,
                None,
                SCREEN_ATOL,
                SCREEN_RTOL,
                true,
                &mut counters,
            );
            match result {
                Ok(report) => {
                    let path_stages = report.path.as_ref().map(|path| &path.stages);
                    // Use the protected coefficient snapshot directly for the oracle projection.
                    let weights = rodas5p_core::load_rodas5p_coefficients()?.b;
                    let candidate_output = path_stages.map(|stages| {
                        output_from_stages(case.problem.dimension, &case.y0, &weights, stages)
                    });
                    let oracle_wrms = candidate_output
                        .as_deref()
                        .map(|candidate| oracle_output_wrms(&case.y0, &sequential.y_new, candidate))
                        .transpose()?;
                    let stage_difference_l2 = path_stages.map(|stages| {
                        let difference: Vec<f64> = flatten(stages)
                            .iter()
                            .zip(flatten(&sequential.stages))
                            .map(|(a, b)| a - b)
                            .collect();
                        safe_l2(&difference)
                    });
                    let false_accept = report.fast_accepted
                        && oracle_wrms.is_some_and(|value| value > DEFECT_BUDGET);
                    let certificate = report.output_certificate.as_ref();
                    candidates.push(HomotopyCandidateRow {
                        case_id: case.descriptor.case_id.clone(),
                        theta: path_config.theta(),
                        q: path_config.q(),
                        path_rounds: path_config.path_rounds(),
                        predictor: path_config.predictor(),
                        corrections_per_point: path_config.corrections_per_point(),
                        fast_accepted: report.fast_accepted,
                        used_fallback: report.step.used_fallback,
                        final_accepted: report.step.accepted,
                        output_wrms: certificate.map(|value| value.output_wrms),
                        embedded_error: certificate.map(|value| value.embedded_error),
                        combined_error: certificate.map(|value| value.combined_error),
                        oracle_output_wrms: oracle_wrms,
                        path_endpoint_target_residual_norm: report
                            .path
                            .as_ref()
                            .and_then(|path| path.points.last())
                            .map(|point| point.target_residual_norm),
                        stage_difference_l2,
                        exact_error_l2: candidate_output.as_deref().and_then(|candidate| {
                            exact_error(
                                &case.problem,
                                case.descriptor.t + case.descriptor.h,
                                candidate,
                            )
                        }),
                        false_accept,
                        fallback_reason: report.fallback_reason,
                        failure: None,
                        work: report.path.map(|path| path.work),
                        counters: report.step.counters,
                    });
                }
                Err(error) => candidates.push(HomotopyCandidateRow {
                    case_id: case.descriptor.case_id.clone(),
                    theta: path_config.theta(),
                    q: path_config.q(),
                    path_rounds: path_config.path_rounds(),
                    predictor: path_config.predictor(),
                    corrections_per_point: path_config.corrections_per_point(),
                    fast_accepted: false,
                    used_fallback: false,
                    final_accepted: false,
                    output_wrms: None,
                    embedded_error: None,
                    combined_error: None,
                    oracle_output_wrms: None,
                    path_endpoint_target_residual_norm: None,
                    stage_difference_l2: None,
                    exact_error_l2: None,
                    false_accept: false,
                    fallback_reason: None,
                    failure: Some(error.to_string()),
                    work: None,
                    counters,
                }),
            }
        }
    }

    let q_summary = [0_usize, 1, 2, 7]
        .into_iter()
        .map(|q| {
            let rows: Vec<&HomotopyCandidateRow> =
                candidates.iter().filter(|row| row.q == q).collect();
            HomotopyQSummary {
                q,
                rows: rows.len(),
                fast_accepts: rows.iter().filter(|row| row.fast_accepted).count(),
                false_accepts: rows.iter().filter(|row| row.false_accept).count(),
                fallbacks: rows.iter().filter(|row| row.used_fallback).count(),
                failures: rows.iter().filter(|row| row.failure.is_some()).count(),
                median_oracle_output_wrms: median(
                    rows.iter()
                        .filter_map(|row| row.oracle_output_wrms)
                        .collect(),
                ),
            }
        })
        .collect();

    let summary = HomotopyExperimentSummary {
        cases: descriptors.len(),
        controls: controls.len(),
        candidates: candidates.len(),
        fast_accepts: candidates.iter().filter(|row| row.fast_accepted).count(),
        false_accepts: candidates.iter().filter(|row| row.false_accept).count(),
        fallbacks: candidates.iter().filter(|row| row.used_fallback).count(),
        failures: controls.iter().filter(|row| row.failure.is_some()).count()
            + candidates
                .iter()
                .filter(|row| row.failure.is_some())
                .count(),
        q_summary,
    };
    let status = if summary.false_accepts == 0 {
        "screen-complete"
    } else {
        "false-accept-detected"
    };
    let order_screens = run_order_screens(profile)?;

    Ok(HomotopyExperimentReport {
        schema: "rodas5p-homotopy-experiment-screen-v1",
        status,
        profile: profile.as_str(),
        atol: SCREEN_ATOL,
        rtol: SCREEN_RTOL,
        output_wrms_budget: DEFECT_BUDGET,
        cases: descriptors,
        controls,
        candidates,
        order_screens,
        summary,
    })
}
