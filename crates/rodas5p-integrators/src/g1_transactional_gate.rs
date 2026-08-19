use rodas5p_core::{
    CoreResult, LinearMethod, LinearSolverConfig, WorkCounters, error_scale, safe_l2, wrms,
};
use serde::Serialize;

use crate::{
    OdeProblem, TransactionalQ1Q2Config, TransactionalQ1Q2Lane, complex_dahlquist_problem,
    constant_affine_mass_problem, oscillatory_prothero_robinson_problem, robertson_problem,
    semilinear_advection_diffusion_problem, sequential_step, stiff_van_der_pol_problem,
    transactional_q1_q2_step,
};

const SCREEN_ATOL: f64 = 1.0e-9;
const SCREEN_RTOL: f64 = 1.0e-7;
const FALSE_ACCEPT_WRMS: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum G1TransactionalGateProfile {
    Smoke,
    Canonical,
}

impl G1TransactionalGateProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Canonical => "canonical",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct G1TransactionalCase {
    pub case_id: String,
    pub family: String,
    pub dimension: usize,
    pub t: f64,
    pub h: f64,
    pub reference_kind: &'static str,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct G1TransactionalRow {
    pub case_id: String,
    pub completed: bool,
    pub lane: Option<TransactionalQ1Q2Lane>,
    pub fast_accepted: bool,
    pub escalated: bool,
    pub used_fallback: bool,
    pub false_accept: bool,
    pub failure: Option<String>,
    pub output_wrms_vs_protected: Option<f64>,
    pub candidate_exact_error_l2: Option<f64>,
    pub protected_exact_error_l2: Option<f64>,
    pub q1_gate_accepted: Option<bool>,
    pub q2_gate_accepted: Option<bool>,
    pub critical_path_depth: Option<u64>,
    pub w_solve_batches: Option<u64>,
    pub w_solve_vectors: Option<u64>,
    pub candidate_counters: WorkCounters,
    pub protected_counters: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct G1TransactionalGateSummary {
    pub cases: usize,
    pub completed: usize,
    pub fast_accepted: usize,
    pub q1_fast: usize,
    pub q2_escalated: usize,
    pub sequential_fallback: usize,
    pub false_accepts: usize,
    pub explicit_jacobian_builds: u64,
    pub direct_factorizations: u64,
    pub fast_path_newton_iterations: u64,
    pub median_critical_path_depth: Option<f64>,
    pub p95_critical_path_depth: Option<f64>,
    pub fast_fraction: f64,
    pub fallback_fraction: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct G1TransactionalGateReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub atol: f64,
    pub rtol: f64,
    pub false_accept_wrms: f64,
    pub cases: Vec<G1TransactionalCase>,
    pub rows: Vec<G1TransactionalRow>,
    pub summary: G1TransactionalGateSummary,
}

struct RuntimeCase {
    descriptor: G1TransactionalCase,
    problem: OdeProblem,
    y0: Vec<f64>,
}

fn runtime_case(family: &str, problem: OdeProblem, y0: Vec<f64>, t: f64, h: f64) -> RuntimeCase {
    let reference_kind = if problem.exact(t + h).is_some() {
        "analytic-exact-plus-protected-jf"
    } else {
        "protected-sequential-jf-local-oracle"
    };
    RuntimeCase {
        descriptor: G1TransactionalCase {
            case_id: problem.name.clone(),
            family: family.into(),
            dimension: problem.dimension,
            t,
            h,
            reference_kind,
        },
        problem,
        y0,
    }
}

fn build_cases(profile: G1TransactionalGateProfile) -> CoreResult<Vec<RuntimeCase>> {
    let mut cases = Vec::new();

    let (complex, y0) = complex_dahlquist_problem(2, 120.0, 180.0, 0.0)?;
    cases.push(runtime_case("complex-dahlquist", complex, y0, 0.0, 0.002));

    let (oscillatory_pr, y0) =
        oscillatory_prothero_robinson_problem(-10_000.0, 1_000.0, 140.0, 0.0)?;
    cases.push(runtime_case(
        "oscillatory-prothero-robinson",
        oscillatory_pr,
        y0,
        0.0,
        1.0e-4,
    ));

    let (mass, y0, _, _) = constant_affine_mass_problem();
    cases.push(runtime_case(
        "constant-noncommuting-mass",
        mass,
        y0,
        0.2,
        0.03,
    ));

    if profile == G1TransactionalGateProfile::Canonical {
        let (vdp, y0) = stiff_van_der_pol_problem(1_000.0)?;
        cases.push(runtime_case("stiff-van-der-pol", vdp, y0, 0.0, 1.0e-4));

        let (robertson, y0) = robertson_problem()?;
        cases.push(runtime_case("robertson", robertson, y0, 0.0, 1.0e-6));

        let (nonnormal, y0) = crate::manufactured_vector_problem(8, 1_000.0, 100.0, 0.5, 0.0)?;
        cases.push(runtime_case(
            "nonlinear-nonnormal-block",
            nonnormal,
            y0,
            0.0,
            1.0e-3,
        ));

        let (diffusion, y0) =
            semilinear_advection_diffusion_problem(32, 0.01, 0.0, -1.0, 10.0, 0.0)?;
        cases.push(runtime_case(
            "diffusion-reaction",
            diffusion,
            y0,
            0.0,
            1.0e-3,
        ));

        let (advection, y0) =
            semilinear_advection_diffusion_problem(32, 0.01, 5.0, -1.0, 10.0, 0.0)?;
        cases.push(runtime_case(
            "advection-diffusion-reaction",
            advection,
            y0,
            0.0,
            1.0e-4,
        ));
    }

    Ok(cases)
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

fn output_wrms(y0: &[f64], reference: &[f64], candidate: &[f64]) -> CoreResult<f64> {
    let scale = error_scale(y0, reference, &[SCREEN_ATOL], SCREEN_RTOL)?;
    let difference = candidate
        .iter()
        .zip(reference)
        .map(|(candidate, reference)| candidate - reference)
        .collect::<Vec<_>>();
    wrms(&difference, &scale)
}

fn percentile(values: &mut [u64], probability: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let index = probability * (values.len().saturating_sub(1) as f64);
    let low = index.floor() as usize;
    let high = index.ceil() as usize;
    let fraction = index - low as f64;
    Some(values[low] as f64 * (1.0 - fraction) + values[high] as f64 * fraction)
}

fn evaluate_case(case: &RuntimeCase) -> G1TransactionalRow {
    let matrix_free = match case.problem.jvp_only_clone() {
        Ok(problem) => problem,
        Err(error) => {
            return G1TransactionalRow {
                case_id: case.descriptor.case_id.clone(),
                completed: false,
                lane: None,
                fast_accepted: false,
                escalated: false,
                used_fallback: false,
                false_accept: false,
                failure: Some(error.to_string()),
                output_wrms_vs_protected: None,
                candidate_exact_error_l2: None,
                protected_exact_error_l2: None,
                q1_gate_accepted: None,
                q2_gate_accepted: None,
                critical_path_depth: None,
                w_solve_batches: None,
                w_solve_vectors: None,
                candidate_counters: WorkCounters::default(),
                protected_counters: WorkCounters::default(),
            };
        }
    };
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        restart: 32,
        maxiter: 256,
        ..LinearSolverConfig::default()
    };
    let mut protected_counters = WorkCounters::default();
    let protected = sequential_step(
        &matrix_free,
        case.descriptor.t,
        &case.y0,
        case.descriptor.h,
        &linear,
        None,
        SCREEN_ATOL,
        SCREEN_RTOL,
        true,
        &mut protected_counters,
    );
    let protected = match protected {
        Ok(step) => step,
        Err(error) => {
            return G1TransactionalRow {
                case_id: case.descriptor.case_id.clone(),
                completed: false,
                lane: None,
                fast_accepted: false,
                escalated: false,
                used_fallback: false,
                false_accept: false,
                failure: Some(format!("protected sequential JF failed: {error}")),
                output_wrms_vs_protected: None,
                candidate_exact_error_l2: None,
                protected_exact_error_l2: None,
                q1_gate_accepted: None,
                q2_gate_accepted: None,
                critical_path_depth: None,
                w_solve_batches: None,
                w_solve_vectors: None,
                candidate_counters: WorkCounters::default(),
                protected_counters,
            };
        }
    };

    let mut candidate_counters = WorkCounters::default();
    let report = transactional_q1_q2_step(
        &matrix_free,
        case.descriptor.t,
        &case.y0,
        case.descriptor.h,
        &TransactionalQ1Q2Config::default(),
        SCREEN_ATOL,
        SCREEN_RTOL,
        true,
        &mut candidate_counters,
    );
    let report = match report {
        Ok(report) => report,
        Err(error) => {
            return G1TransactionalRow {
                case_id: case.descriptor.case_id.clone(),
                completed: false,
                lane: None,
                fast_accepted: false,
                escalated: false,
                used_fallback: false,
                false_accept: false,
                failure: Some(format!("transactional candidate failed: {error}")),
                output_wrms_vs_protected: None,
                candidate_exact_error_l2: None,
                protected_exact_error_l2: exact_error(
                    &case.problem,
                    case.descriptor.t + case.descriptor.h,
                    &protected.y_new,
                ),
                q1_gate_accepted: None,
                q2_gate_accepted: None,
                critical_path_depth: None,
                w_solve_batches: None,
                w_solve_vectors: None,
                candidate_counters,
                protected_counters,
            };
        }
    };
    let defect = output_wrms(&case.y0, &protected.y_new, &report.step.y_new).ok();
    let false_accept = report.fast_accepted && defect.is_none_or(|value| value > FALSE_ACCEPT_WRMS);
    G1TransactionalRow {
        case_id: case.descriptor.case_id.clone(),
        completed: true,
        lane: Some(report.lane),
        fast_accepted: report.fast_accepted,
        escalated: report.escalated,
        used_fallback: report.step.used_fallback,
        false_accept,
        failure: None,
        output_wrms_vs_protected: defect,
        candidate_exact_error_l2: exact_error(
            &case.problem,
            case.descriptor.t + case.descriptor.h,
            &report.step.y_new,
        ),
        protected_exact_error_l2: exact_error(
            &case.problem,
            case.descriptor.t + case.descriptor.h,
            &protected.y_new,
        ),
        q1_gate_accepted: Some(report.q1_gate.accepted),
        q2_gate_accepted: report.q2_gate.as_ref().map(|gate| gate.accepted),
        critical_path_depth: Some(report.critical_path_depth),
        w_solve_batches: Some(report.work.w_solve_batches),
        w_solve_vectors: Some(report.work.w_solve_vectors),
        candidate_counters,
        protected_counters,
    }
}

pub fn run_g1_transactional_gate(
    profile: G1TransactionalGateProfile,
) -> CoreResult<G1TransactionalGateReport> {
    let runtime = build_cases(profile)?;
    let cases = runtime
        .iter()
        .map(|case| case.descriptor.clone())
        .collect::<Vec<_>>();
    let rows = runtime.iter().map(evaluate_case).collect::<Vec<_>>();
    let completed_rows = rows.iter().filter(|row| row.completed).collect::<Vec<_>>();
    let completed = completed_rows.len();
    let q1_fast = completed_rows
        .iter()
        .filter(|row| row.lane == Some(TransactionalQ1Q2Lane::Q1Fast))
        .count();
    let q2_escalated = completed_rows
        .iter()
        .filter(|row| row.lane == Some(TransactionalQ1Q2Lane::Q2Escalated))
        .count();
    let sequential_fallback = completed_rows
        .iter()
        .filter(|row| row.lane == Some(TransactionalQ1Q2Lane::SequentialFallback))
        .count();
    let fast_accepted = q1_fast + q2_escalated;
    let mut depths = completed_rows
        .iter()
        .filter_map(|row| row.critical_path_depth)
        .collect::<Vec<_>>();
    let mut depths_p95 = depths.clone();
    let median_critical_path_depth = percentile(&mut depths, 0.5);
    let p95_critical_path_depth = percentile(&mut depths_p95, 0.95);
    let explicit_jacobian_builds = completed_rows
        .iter()
        .map(|row| row.candidate_counters.jacobian_builds)
        .sum();
    let direct_factorizations = completed_rows
        .iter()
        .map(|row| row.candidate_counters.direct_factorizations)
        .sum();
    let fast_path_newton_iterations = completed_rows
        .iter()
        .filter(|row| row.fast_accepted)
        .map(|row| row.candidate_counters.nonlinear_iterations)
        .sum();
    let denominator = completed.max(1) as f64;
    let summary = G1TransactionalGateSummary {
        cases: cases.len(),
        completed,
        fast_accepted,
        q1_fast,
        q2_escalated,
        sequential_fallback,
        false_accepts: completed_rows.iter().filter(|row| row.false_accept).count(),
        explicit_jacobian_builds,
        direct_factorizations,
        fast_path_newton_iterations,
        median_critical_path_depth,
        p95_critical_path_depth,
        fast_fraction: fast_accepted as f64 / denominator,
        fallback_fraction: sequential_fallback as f64 / denominator,
    };
    let status = if completed == cases.len()
        && summary.false_accepts == 0
        && summary.explicit_jacobian_builds == 0
        && summary.direct_factorizations == 0
        && summary.fast_path_newton_iterations == 0
    {
        "pass-with-scientific-hold"
    } else {
        "fail"
    };
    Ok(G1TransactionalGateReport {
        schema: "generic-q1-q2-transactional-gate-v1",
        status,
        profile: profile.as_str(),
        atol: SCREEN_ATOL,
        rtol: SCREEN_RTOL,
        false_accept_wrms: FALSE_ACCEPT_WRMS,
        cases,
        rows,
        summary,
    })
}
