use std::sync::Arc;

use rodas5p_core::{
    CoreResult, DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters, error_scale, safe_l2,
    wrms,
};
use serde::Serialize;

use crate::{
    HomotopyPredictor, HomotopyRoundSpec, HomotopyScheduleConfig, HomotopyWorkLedger, OdeProblem,
    ScheduledHomotopyRoundPoint, StructuredBlockSystem, build_step_context,
    certify_nonlinear_target, constant_affine_mass_problem, manufactured_mass_nonlinear_problem,
    manufactured_vector_problem, prothero_robinson_problem, run_scheduled_homotopy_path,
    sequential_step,
};

const SCREEN_ATOL: f64 = 1.0e-7;
const SCREEN_RTOL: f64 = 1.0e-6;
const ALGEBRAIC_OUTPUT_WRMS_BUDGET: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathControllerProfile {
    Smoke,
    Canonical,
}

impl PathControllerProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Canonical => "canonical",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PathControllerCase {
    pub case_id: String,
    pub family: String,
    pub dimension: usize,
    pub t: f64,
    pub h: f64,
    pub stiffness: Option<f64>,
    pub nonlinearity: Option<f64>,
    pub nonnormality: Option<f64>,
    pub oscillation_frequency: Option<f64>,
    pub hostile: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PathControllerControlRow {
    pub case_id: String,
    pub method: &'static str,
    pub completed: bool,
    pub output: Option<Vec<f64>>,
    pub embedded_error: Option<f64>,
    pub exact_error_l2: Option<f64>,
    pub failure: Option<String>,
    pub counters: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PathControllerRow {
    pub case_id: String,
    pub hostile: bool,
    pub schedule_id: String,
    pub schedule: HomotopyScheduleConfig,
    pub completed: bool,
    pub algebraic_accepted: bool,
    pub embedded_step_accepted: bool,
    pub full_step_accepted: bool,
    pub accepted_by_original_certificate: bool,
    pub false_accept: bool,
    pub failure: Option<String>,
    pub last_lambda: f64,
    pub rounds_completed: usize,
    pub initial_target_residual: Option<f64>,
    pub final_target_residual: Option<f64>,
    pub final_target_residual_ratio: Option<f64>,
    pub oracle_output_wrms: Option<f64>,
    pub certificate_output_wrms: Option<f64>,
    pub certificate_embedded_error: Option<f64>,
    pub certificate_combined_error: Option<f64>,
    pub exact_error_l2: Option<f64>,
    pub points: Vec<ScheduledHomotopyRoundPoint>,
    pub work: HomotopyWorkLedger,
    pub candidate_counters: WorkCounters,
    pub certificate_counters: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PathControllerScheduleSummary {
    pub schedule_id: String,
    pub rows: usize,
    pub nonhostile_rows: usize,
    pub completed: usize,
    pub accepted: usize,
    pub false_accepts: usize,
    pub failures: usize,
    pub nonhostile_failures: usize,
    pub median_final_target_residual_ratio: Option<f64>,
    pub median_w_solve_vectors: Option<f64>,
    pub median_rounds_completed: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PathControllerSummary {
    pub cases: usize,
    pub schedules: usize,
    pub controls: usize,
    pub rows: usize,
    pub completed: usize,
    pub accepted: usize,
    pub false_accepts: usize,
    pub failures: usize,
    pub nonhostile_failures: usize,
    pub schedule_summaries: Vec<PathControllerScheduleSummary>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PathControllerReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub atol: f64,
    pub rtol: f64,
    pub output_wrms_budget: f64,
    pub cases: Vec<PathControllerCase>,
    pub controls: Vec<PathControllerControlRow>,
    pub rows: Vec<PathControllerRow>,
    pub summary: PathControllerSummary,
}

struct RuntimeCase {
    descriptor: PathControllerCase,
    problem: OdeProblem,
    y0: Vec<f64>,
}

#[derive(Clone)]
struct NamedSchedule {
    id: &'static str,
    config: HomotopyScheduleConfig,
}

fn round(
    lambda_end: f64,
    theta: f64,
    q: usize,
    damping: f64,
    corrections: usize,
) -> CoreResult<HomotopyRoundSpec> {
    HomotopyRoundSpec::new(lambda_end, theta, q, damping, corrections)
}

fn one_round_schedule(
    id: &'static str,
    theta: f64,
    q: usize,
    damping: f64,
    corrections: usize,
) -> CoreResult<NamedSchedule> {
    Ok(NamedSchedule {
        id,
        config: HomotopyScheduleConfig::new(
            vec![round(1.0, theta, q, damping, corrections)?],
            HomotopyPredictor::Euler,
        )?,
    })
}

fn two_round_schedule(
    id: &'static str,
    theta: [f64; 2],
    q: [usize; 2],
    damping: [f64; 2],
    corrections: [usize; 2],
) -> CoreResult<NamedSchedule> {
    Ok(NamedSchedule {
        id,
        config: HomotopyScheduleConfig::new(
            vec![
                round(0.5, theta[0], q[0], damping[0], corrections[0])?,
                round(1.0, theta[1], q[1], damping[1], corrections[1])?,
            ],
            HomotopyPredictor::AdamsBashforth2,
        )?,
    })
}

fn three_round_schedule(
    id: &'static str,
    theta: [f64; 3],
    q: [usize; 3],
    damping: [f64; 3],
    corrections: [usize; 3],
) -> CoreResult<NamedSchedule> {
    Ok(NamedSchedule {
        id,
        config: HomotopyScheduleConfig::new(
            vec![
                round(1.0 / 3.0, theta[0], q[0], damping[0], corrections[0])?,
                round(2.0 / 3.0, theta[1], q[1], damping[1], corrections[1])?,
                round(1.0, theta[2], q[2], damping[2], corrections[2])?,
            ],
            HomotopyPredictor::AdamsBashforth2,
        )?,
    })
}

fn schedules(profile: PathControllerProfile) -> CoreResult<Vec<NamedSchedule>> {
    let mut out = vec![
        one_round_schedule("direct-decoupled-q1-c2", 0.0, 1, 1.0, 2)?,
        one_round_schedule("direct-decoupled-q2-c1", 0.0, 2, 1.0, 1)?,
        one_round_schedule("direct-decoupled-q2-c2", 0.0, 2, 1.0, 2)?,
        two_round_schedule(
            "two-round-decoupled-q1-c2",
            [0.0; 2],
            [1; 2],
            [1.0; 2],
            [0, 2],
        )?,
        two_round_schedule(
            "two-round-decoupled-q2-c1",
            [0.0; 2],
            [2; 2],
            [1.0; 2],
            [0, 1],
        )?,
        three_round_schedule("fixed-q0", [1.0; 3], [0; 3], [1.0; 3], [0; 3])?,
        three_round_schedule("fixed-q1", [1.0; 3], [1; 3], [1.0; 3], [0; 3])?,
        three_round_schedule("fixed-q2", [1.0; 3], [2; 3], [1.0; 3], [0; 3])?,
        three_round_schedule("fixed-q7", [1.0; 3], [7; 3], [1.0; 3], [0; 3])?,
        three_round_schedule("escalate-q012", [1.0; 3], [0, 1, 2], [1.0; 3], [0; 3])?,
        three_round_schedule("front-loaded-q211", [1.0; 3], [2, 1, 1], [1.0; 3], [0; 3])?,
        three_round_schedule("persistent-q222", [1.0; 3], [2, 2, 2], [1.0; 3], [0; 3])?,
        three_round_schedule("mixed-q122", [1.0; 3], [1, 2, 2], [1.0; 3], [0; 3])?,
        three_round_schedule(
            "theta-ramp-q211",
            [1.0, 0.5, 0.0],
            [2, 1, 1],
            [1.0; 3],
            [0; 3],
        )?,
        three_round_schedule(
            "damped-q2-final-correction",
            [1.0; 3],
            [2; 3],
            [0.75, 0.85, 1.0],
            [0, 0, 1],
        )?,
        three_round_schedule(
            "fixed-q1-final-correction",
            [1.0; 3],
            [1; 3],
            [1.0; 3],
            [0, 0, 1],
        )?,
        three_round_schedule(
            "front-loaded-q211-final-correction",
            [1.0; 3],
            [2, 1, 1],
            [1.0; 3],
            [0, 0, 1],
        )?,
        three_round_schedule(
            "decoupled-q0-final-correction",
            [0.0; 3],
            [0; 3],
            [1.0; 3],
            [0, 0, 1],
        )?,
        three_round_schedule(
            "decoupled-q1-final-correction",
            [0.0; 3],
            [1; 3],
            [1.0; 3],
            [0, 0, 1],
        )?,
        three_round_schedule(
            "decoupled-q2-final-correction",
            [0.0; 3],
            [2; 3],
            [1.0; 3],
            [0, 0, 1],
        )?,
        three_round_schedule(
            "decoupled-q1-final-two-corrections",
            [0.0; 3],
            [1; 3],
            [1.0; 3],
            [0, 0, 2],
        )?,
        three_round_schedule(
            "decoupled-q2-final-two-corrections",
            [0.0; 3],
            [2; 3],
            [1.0; 3],
            [0, 0, 2],
        )?,
    ];
    if profile == PathControllerProfile::Smoke {
        out.retain(|schedule| {
            matches!(
                schedule.id,
                "direct-decoupled-q2-c1"
                    | "fixed-q2"
                    | "fixed-q7"
                    | "fixed-q1-final-correction"
                    | "escalate-q012"
                    | "theta-ramp-q211"
            )
        });
    }
    Ok(out)
}

fn complex_dahlquist_problem(
    sigma: f64,
    omega: f64,
    t0: f64,
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let matrix = DenseMatrix::from_rows(&[&[-sigma, -omega], &[omega, -sigma]])?;
    let rhs_matrix = matrix.clone();
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| rhs_matrix.matvec_into(y, out));
    let batch_matrix = matrix.clone();
    let batch = Arc::new(move |_times: &[f64], states: &[Vec<f64>]| {
        states
            .iter()
            .map(|state| batch_matrix.matvec(state))
            .collect()
    });
    let jac_matrix = matrix.clone();
    let jac = Arc::new(move |_t: f64, _y: &[f64]| Ok(jac_matrix.clone()));
    let exact = Arc::new(move |t: f64| {
        let dt = t - t0;
        let amplitude = (-sigma * dt).exp();
        vec![
            amplitude * (omega * dt).cos(),
            amplitude * (omega * dt).sin(),
        ]
    });
    Ok((
        OdeProblem::new(
            format!("complex-dahlquist-s{sigma}-w{omega}"),
            2,
            rhs,
            Some(batch),
            Some(jac),
            None,
            None,
            true,
            None,
            Some(exact),
        )?,
        vec![1.0, 0.0],
    ))
}

fn build_cases(profile: PathControllerProfile) -> CoreResult<Vec<RuntimeCase>> {
    let mut cases = Vec::new();

    let (affine, y0, _, _) = constant_affine_mass_problem();
    cases.push(RuntimeCase {
        descriptor: PathControllerCase {
            case_id: "affine-noncommuting-mass".into(),
            family: "affine-mass".into(),
            dimension: 2,
            t: 0.2,
            h: 0.03,
            stiffness: None,
            nonlinearity: Some(0.0),
            nonnormality: None,
            oscillation_frequency: None,
            hostile: false,
        },
        problem: affine,
        y0,
    });

    let (complex, y0) = complex_dahlquist_problem(120.0, 180.0, 0.0)?;
    cases.push(RuntimeCase {
        descriptor: PathControllerCase {
            case_id: "complex-dahlquist-s120-w180".into(),
            family: "complex-dahlquist".into(),
            dimension: 2,
            t: 0.0,
            h: 0.002,
            stiffness: Some(120.0),
            nonlinearity: Some(0.0),
            nonnormality: Some(0.0),
            oscillation_frequency: Some(180.0),
            hostile: false,
        },
        problem: complex,
        y0,
    });

    let (moderate, y0) = manufactured_vector_problem(8, 100.0, 10.0, 0.2, 0.0)?;
    cases.push(RuntimeCase {
        descriptor: PathControllerCase {
            case_id: "mv-n8-s100-m10-eta0.2".into(),
            family: "manufactured-vector".into(),
            dimension: 8,
            t: 0.0,
            h: 0.005,
            stiffness: Some(100.0),
            nonlinearity: Some(10.0),
            nonnormality: Some(0.2),
            oscillation_frequency: None,
            hostile: false,
        },
        problem: moderate,
        y0,
    });

    if profile == PathControllerProfile::Canonical {
        let (pr, y0) = prothero_robinson_problem(-10_000.0, 1_000.0, 0.0);
        cases.push(RuntimeCase {
            descriptor: PathControllerCase {
                case_id: "pr-l1e4-m1e3".into(),
                family: "prothero-robinson".into(),
                dimension: 1,
                t: 0.0,
                h: 0.001,
                stiffness: Some(10_000.0),
                nonlinearity: Some(1_000.0),
                nonnormality: Some(0.0),
                oscillation_frequency: Some(1.0),
                hostile: false,
            },
            problem: pr,
            y0,
        });

        let (mass, y0, _, _) = manufactured_mass_nonlinear_problem(1_000.0, 100.0, 0.4, 0.0)?;
        cases.push(RuntimeCase {
            descriptor: PathControllerCase {
                case_id: "mass-noncommuting-s1e3-m100-eta0.4".into(),
                family: "manufactured-mass".into(),
                dimension: 2,
                t: 0.0,
                h: 0.001,
                stiffness: Some(1_000.0),
                nonlinearity: Some(100.0),
                nonnormality: Some(0.4),
                oscillation_frequency: None,
                hostile: false,
            },
            problem: mass,
            y0,
        });

        let (hostile, y0) = manufactured_vector_problem(8, 10_000.0, 1_000.0, 0.9, 0.0)?;
        cases.push(RuntimeCase {
            descriptor: PathControllerCase {
                case_id: "mv-n8-s1e4-m1e3-eta0.9-hostile".into(),
                family: "manufactured-vector".into(),
                dimension: 8,
                t: 0.0,
                h: 0.001,
                stiffness: Some(10_000.0),
                nonlinearity: Some(1_000.0),
                nonnormality: Some(0.9),
                oscillation_frequency: None,
                hostile: true,
            },
            problem: hostile,
            y0,
        });
    }

    Ok(cases)
}

fn output_from_stages(y0: &[f64], weights: &[f64], stages: &[Vec<f64>]) -> Vec<f64> {
    let mut output = y0.to_vec();
    for (weight, stage) in weights.iter().zip(stages) {
        for (value, increment) in output.iter_mut().zip(stage) {
            *value += weight * increment;
        }
    }
    output
}

fn exact_error(problem: &OdeProblem, time: f64, output: &[f64]) -> Option<f64> {
    problem.exact(time).map(|exact| {
        safe_l2(
            &output
                .iter()
                .zip(exact)
                .map(|(candidate, reference)| candidate - reference)
                .collect::<Vec<_>>(),
        )
    })
}

fn output_wrms(y0: &[f64], oracle: &[f64], candidate: &[f64]) -> CoreResult<f64> {
    let scale = error_scale(y0, oracle, &[SCREEN_ATOL], SCREEN_RTOL)?;
    let difference = candidate
        .iter()
        .zip(oracle)
        .map(|(a, b)| a - b)
        .collect::<Vec<_>>();
    wrms(&difference, &scale)
}

fn control(case: &RuntimeCase) -> PathControllerControlRow {
    let config = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let mut counters = WorkCounters::default();
    match sequential_step(
        &case.problem,
        case.descriptor.t,
        &case.y0,
        case.descriptor.h,
        &config,
        None,
        SCREEN_ATOL,
        SCREEN_RTOL,
        true,
        &mut counters,
    ) {
        Ok(step) => PathControllerControlRow {
            case_id: case.descriptor.case_id.clone(),
            method: "sequential-direct",
            completed: true,
            exact_error_l2: exact_error(&case.problem, step.t_new, &step.y_new),
            output: Some(step.y_new),
            embedded_error: Some(step.error_norm),
            failure: None,
            counters: step.counters,
        },
        Err(error) => PathControllerControlRow {
            case_id: case.descriptor.case_id.clone(),
            method: "sequential-direct",
            completed: false,
            output: None,
            embedded_error: None,
            exact_error_l2: None,
            failure: Some(error.to_string()),
            counters,
        },
    }
}

fn run_schedule(
    case: &RuntimeCase,
    schedule: &NamedSchedule,
    oracle: Option<&[f64]>,
) -> CoreResult<PathControllerRow> {
    let mut candidate_counters = WorkCounters::default();
    let context = build_step_context(
        &case.problem,
        case.descriptor.t,
        &case.y0,
        case.descriptor.h,
        &mut candidate_counters,
    )?;
    let block = StructuredBlockSystem::new(&context);
    let report = run_scheduled_homotopy_path(&block, &schedule.config, &mut candidate_counters)?;
    let initial_target = report
        .points
        .first()
        .map(|point| point.target_residual_after);
    let final_target = report
        .points
        .last()
        .map(|point| point.target_residual_after);
    let final_ratio = initial_target
        .zip(final_target)
        .map(|(initial, final_value)| final_value / initial.max(f64::MIN_POSITIVE));

    let mut certificate_counters = WorkCounters::default();
    let certificate = if report.completed {
        certify_nonlinear_target(
            &block,
            &report.stages,
            SCREEN_ATOL,
            SCREEN_RTOL,
            &mut certificate_counters,
        )
        .ok()
    } else {
        None
    };
    let candidate_output = report
        .completed
        .then(|| output_from_stages(&case.y0, &context.coeffs.b, &report.stages));
    let oracle_output_wrms = oracle
        .zip(candidate_output.as_deref())
        .map(|(reference, candidate)| output_wrms(&case.y0, reference, candidate))
        .transpose()?;
    let algebraic_accepted = certificate
        .as_ref()
        .is_some_and(|value| value.output_wrms <= ALGEBRAIC_OUTPUT_WRMS_BUDGET);
    let embedded_step_accepted = certificate
        .as_ref()
        .is_some_and(|value| value.embedded_error <= 1.0);
    let full_step_accepted = certificate
        .as_ref()
        .is_some_and(|value| value.combined_error <= 1.0);
    let false_accept = algebraic_accepted
        && oracle_output_wrms.is_some_and(|value| value > ALGEBRAIC_OUTPUT_WRMS_BUDGET);
    let failure = report.failure.clone().or_else(|| {
        if report.completed && certificate.is_none() {
            Some("original RODAS target certificate failed".into())
        } else {
            None
        }
    });

    Ok(PathControllerRow {
        case_id: case.descriptor.case_id.clone(),
        hostile: case.descriptor.hostile,
        schedule_id: schedule.id.into(),
        schedule: schedule.config.clone(),
        completed: report.completed,
        algebraic_accepted,
        embedded_step_accepted,
        full_step_accepted,
        accepted_by_original_certificate: algebraic_accepted,
        false_accept,
        failure,
        last_lambda: report.last_lambda,
        rounds_completed: report.points.len().saturating_sub(1),
        initial_target_residual: initial_target,
        final_target_residual: final_target,
        final_target_residual_ratio: final_ratio,
        oracle_output_wrms,
        certificate_output_wrms: certificate.as_ref().map(|value| value.output_wrms),
        certificate_embedded_error: certificate.as_ref().map(|value| value.embedded_error),
        certificate_combined_error: certificate.as_ref().map(|value| value.combined_error),
        exact_error_l2: candidate_output.as_deref().and_then(|output| {
            exact_error(&case.problem, case.descriptor.t + case.descriptor.h, output)
        }),
        points: report.points,
        work: report.work,
        candidate_counters,
        certificate_counters,
    })
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

fn summarize(schedule_id: &str, rows: &[PathControllerRow]) -> PathControllerScheduleSummary {
    let selected = rows
        .iter()
        .filter(|row| row.schedule_id == schedule_id)
        .collect::<Vec<_>>();
    PathControllerScheduleSummary {
        schedule_id: schedule_id.into(),
        rows: selected.len(),
        nonhostile_rows: selected.iter().filter(|row| !row.hostile).count(),
        completed: selected.iter().filter(|row| row.completed).count(),
        accepted: selected.iter().filter(|row| row.algebraic_accepted).count(),
        false_accepts: selected.iter().filter(|row| row.false_accept).count(),
        failures: selected.iter().filter(|row| row.failure.is_some()).count(),
        nonhostile_failures: selected
            .iter()
            .filter(|row| !row.hostile && row.failure.is_some())
            .count(),
        median_final_target_residual_ratio: median(
            selected
                .iter()
                .filter_map(|row| row.final_target_residual_ratio)
                .collect(),
        ),
        median_w_solve_vectors: median(
            selected
                .iter()
                .map(|row| row.work.w_solve_vectors as f64)
                .collect(),
        ),
        median_rounds_completed: median(
            selected
                .iter()
                .map(|row| row.rounds_completed as f64)
                .collect(),
        ),
    }
}

pub fn run_path_controller_screen(
    profile: PathControllerProfile,
) -> CoreResult<PathControllerReport> {
    let runtime_cases = build_cases(profile)?;
    let named_schedules = schedules(profile)?;
    let controls = runtime_cases.iter().map(control).collect::<Vec<_>>();
    let mut rows = Vec::new();
    for case in &runtime_cases {
        let oracle = controls
            .iter()
            .find(|row| row.case_id == case.descriptor.case_id)
            .and_then(|row| row.output.as_deref());
        for schedule in &named_schedules {
            rows.push(run_schedule(case, schedule, oracle)?);
        }
    }
    rows.sort_by(|left, right| {
        (&left.case_id, &left.schedule_id).cmp(&(&right.case_id, &right.schedule_id))
    });
    let schedule_summaries = named_schedules
        .iter()
        .map(|schedule| summarize(schedule.id, &rows))
        .collect::<Vec<_>>();
    let failures = rows.iter().filter(|row| row.failure.is_some()).count();
    let nonhostile_failures = rows
        .iter()
        .filter(|row| !row.hostile && row.failure.is_some())
        .count();
    let false_accepts = rows.iter().filter(|row| row.false_accept).count();
    let status = if controls.iter().any(|row| !row.completed) {
        "control-failure"
    } else if nonhostile_failures > 0 || false_accepts > 0 {
        "complete-with-scientific-failures"
    } else if failures > 0 {
        "complete-with-hostile-failures"
    } else {
        "complete"
    };
    Ok(PathControllerReport {
        schema: "rodas5p-path-controller-screen-v1",
        status,
        profile: profile.as_str(),
        atol: SCREEN_ATOL,
        rtol: SCREEN_RTOL,
        output_wrms_budget: ALGEBRAIC_OUTPUT_WRMS_BUDGET,
        cases: runtime_cases
            .iter()
            .map(|case| case.descriptor.clone())
            .collect(),
        controls,
        summary: PathControllerSummary {
            cases: runtime_cases.len(),
            schedules: named_schedules.len(),
            controls: runtime_cases.len(),
            rows: rows.len(),
            completed: rows.iter().filter(|row| row.completed).count(),
            accepted: rows.iter().filter(|row| row.algebraic_accepted).count(),
            false_accepts,
            failures,
            nonhostile_failures,
            schedule_summaries,
        },
        rows,
    })
}
