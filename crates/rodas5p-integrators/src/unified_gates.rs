use std::time::Instant;

use rodas5p_core::{
    CoreError, CoreResult, LinearMethod, LinearSolverConfig, WorkCounters, safe_l2,
};
use serde::Serialize;

use crate::{
    BlockMethod, BlockPreconditioner, CandidateCatalog, CandidateExecution, CandidateFamily,
    CandidateRecycleLifetime, CandidateSpec, CandidateStatus, HomotopyPathConfig,
    HomotopyPredictor, HomotopyStepConfig, KrylovState, OdeProblem, ParallelExecution,
    PredictorKind, SabrConfig, StageHistory, UnifiedCandidateOutcome, UnifiedNonlinearScreen,
    UnifiedScreenProfile, homotopy_step, manufactured_vector_problem, sabr_step,
    scalar_linear_problem, sequential_step,
};

const GATE_ATOL: f64 = 1.0e-7;
const GATE_RTOL: f64 = 1.0e-6;
const HOMOTOPY_OUTPUT_BUDGET: f64 = 0.1;
const ORDER_PASS_FLOOR: f64 = 4.5;
const STIFF_ABSOLUTE_FLOOR: f64 = 1.0e-8;
const STIFF_REFERENCE_FACTOR: f64 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateGateVerdict {
    Reference,
    Promote,
    Hold,
    Deferred,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateOrderGateRow {
    pub candidate_id: String,
    pub family: CandidateFamily,
    pub h: f64,
    pub steps: usize,
    pub error_l2: Option<f64>,
    pub observed_order: Option<f64>,
    pub roundoff_l2_floor: Option<f64>,
    pub above_roundoff_floor: Option<bool>,
    pub fast_steps: usize,
    pub fallback_steps: usize,
    pub all_fast: bool,
    pub counters: WorkCounters,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateStiffGateRow {
    pub candidate_id: String,
    pub family: CandidateFamily,
    pub z: f64,
    pub amplification: Option<f64>,
    pub protected_amplification: f64,
    pub allowed_amplification: f64,
    pub final_pass: bool,
    pub fast_pass: bool,
    pub used_fallback: bool,
    pub counters: WorkCounters,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateGateReport {
    pub candidate_id: String,
    pub family: CandidateFamily,
    pub status: CandidateStatus,
    pub verdict: CandidateGateVerdict,
    pub qualification_observed_order: Option<f64>,
    pub finest_observed_order: Option<f64>,
    pub order_pass: bool,
    pub order_all_fast: bool,
    pub stiff_decay_pass: bool,
    pub stiff_fast_pass: bool,
    pub one_step_rows: usize,
    pub one_step_rejections: usize,
    pub one_step_failures: usize,
    pub c3_false_accepts: usize,
    pub c3_reference_fallbacks: usize,
    pub nonnormal_pass: bool,
    pub maximum_oracle_output_wrms: Option<f64>,
    pub median_batch_depth: Option<f64>,
    pub median_batch_vectors: Option<f64>,
    pub measurable_advantage: bool,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnifiedScientificGateReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub threads: usize,
    pub order_pass_floor: f64,
    pub stiff_absolute_floor: f64,
    pub stiff_reference_factor: f64,
    pub order_rows: Vec<CandidateOrderGateRow>,
    pub stiff_rows: Vec<CandidateStiffGateRow>,
    pub candidates: Vec<CandidateGateReport>,
    pub compute_seconds: f64,
}

#[derive(Clone, Debug)]
struct CandidateTrajectoryResult {
    state: Vec<f64>,
    steps: usize,
    fast_steps: usize,
    fallback_steps: usize,
    counters: WorkCounters,
}

fn direct_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    }
}

fn sequential_config(method: LinearMethod) -> LinearSolverConfig {
    LinearSolverConfig {
        method,
        rtol: 1.0e-11,
        atol: 1.0e-13,
        restart: 40,
        maxiter: 200,
        inner_m: 40,
        outer_k: 6,
        recycle_dim: 6,
        ..LinearSolverConfig::default()
    }
}

fn map_sabr_config(candidate: &CandidateSpec) -> CoreResult<SabrConfig> {
    let CandidateExecution::Sabr {
        block_method,
        predictor,
    } = candidate.execution()
    else {
        return Err(CoreError::InvalidInput(
            "candidate is not a SABR configuration".into(),
        ));
    };
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
    Ok(SabrConfig {
        block_method,
        predictor,
        block_preconditioner: BlockPreconditioner::Direct,
        ..SabrConfig::default()
    })
}

fn map_homotopy_config(candidate: &CandidateSpec) -> CoreResult<HomotopyStepConfig> {
    let CandidateExecution::Homotopy {
        theta,
        q,
        path_rounds,
        predictor,
        corrections_per_point,
    } = candidate.execution()
    else {
        return Err(CoreError::InvalidInput(
            "candidate is not a homotopy configuration".into(),
        ));
    };
    let predictor = match predictor {
        crate::HomotopyPredictorVariant::Euler => HomotopyPredictor::Euler,
        crate::HomotopyPredictorVariant::AdamsBashforth2 => HomotopyPredictor::AdamsBashforth2,
    };
    HomotopyStepConfig::new(
        HomotopyPathConfig::new(*theta, *q, *path_rounds, predictor, *corrections_per_point)?,
        HOMOTOPY_OUTPUT_BUDGET,
    )
}

#[allow(clippy::too_many_arguments)]
fn integrate_candidate_fixed(
    candidate: &CandidateSpec,
    problem: &OdeProblem,
    y0: &[f64],
    final_time: f64,
    h: f64,
) -> CoreResult<CandidateTrajectoryResult> {
    if !matches!(candidate.status(), CandidateStatus::Executable) {
        return Err(CoreError::InvalidInput(
            "deferred candidate cannot be integrated".into(),
        ));
    }
    let fallback = direct_config();
    let mut counters = WorkCounters::default();
    let mut state = y0.to_vec();
    let mut time = 0.0;
    let mut steps = 0_usize;
    let mut fast_steps = 0_usize;
    let mut fallback_steps = 0_usize;
    let mut history = StageHistory::default();
    let mut persistent_recycle = match candidate.execution() {
        CandidateExecution::Sequential {
            linear_method,
            recycle_lifetime: CandidateRecycleLifetime::Persistent,
        } => KrylovState::for_method(*linear_method),
        _ => None,
    };

    while time < final_time - 10.0 * f64::EPSILON * final_time.abs().max(1.0) {
        let step_size = h.min(final_time - time);
        match candidate.execution() {
            CandidateExecution::Sequential {
                linear_method,
                recycle_lifetime,
            } => {
                let config = sequential_config(*linear_method);
                let mut stage_recycle = match recycle_lifetime {
                    CandidateRecycleLifetime::Stage => KrylovState::for_method(*linear_method),
                    _ => None,
                };
                let recycle = match recycle_lifetime {
                    CandidateRecycleLifetime::Persistent => persistent_recycle.as_mut(),
                    CandidateRecycleLifetime::Stage => stage_recycle.as_mut(),
                    CandidateRecycleLifetime::Off => None,
                };
                let report = sequential_step(
                    problem,
                    time,
                    &state,
                    step_size,
                    &config,
                    recycle,
                    GATE_ATOL,
                    GATE_RTOL,
                    true,
                    &mut counters,
                )?;
                state = report.y_new;
                time = report.t_new;
                fast_steps += 1;
            }
            CandidateExecution::Sabr { .. } => {
                let report = sabr_step(
                    problem,
                    time,
                    &state,
                    step_size,
                    &map_sabr_config(candidate)?,
                    Some(&fallback),
                    &mut history,
                    None,
                    GATE_ATOL,
                    GATE_RTOL,
                    true,
                    &mut counters,
                )?;
                if report.used_fallback {
                    fallback_steps += 1;
                } else {
                    fast_steps += 1;
                }
                state = report.y_new;
                time = report.t_new;
            }
            CandidateExecution::Homotopy { .. } => {
                let report = homotopy_step(
                    problem,
                    time,
                    &state,
                    step_size,
                    &map_homotopy_config(candidate)?,
                    Some(&fallback),
                    None,
                    GATE_ATOL,
                    GATE_RTOL,
                    true,
                    &mut counters,
                )?;
                if report.step.used_fallback {
                    fallback_steps += 1;
                } else {
                    fast_steps += 1;
                }
                state = report.step.y_new;
                time = report.step.t_new;
            }
            CandidateExecution::Bdf { .. } | CandidateExecution::RadauIrk { .. } => {
                return Err(CoreError::InvalidInput(
                    "complete-integrator candidate must use the native integrator gate".into(),
                ));
            }
            CandidateExecution::Deferred => {
                return Err(CoreError::InvalidInput(
                    "deferred candidate cannot be integrated".into(),
                ));
            }
        }
        steps += 1;
    }

    Ok(CandidateTrajectoryResult {
        state,
        steps,
        fast_steps,
        fallback_steps,
        counters,
    })
}

fn order_step_sizes(profile: UnifiedScreenProfile) -> &'static [f64] {
    match profile {
        UnifiedScreenProfile::Smoke => &[0.04, 0.02],
        UnifiedScreenProfile::Canonical => &[0.04, 0.02, 0.01, 0.005],
    }
}

fn finest_observed_order(rows: &[CandidateOrderGateRow]) -> Option<f64> {
    rows.iter()
        .rev()
        .filter_map(|row| row.observed_order)
        .find(|value| value.is_finite())
}

fn qualification_observed_order(rows: &[CandidateOrderGateRow]) -> Option<f64> {
    rows.iter().rev().find_map(|row| {
        (row.failure.is_none() && row.above_roundoff_floor == Some(true))
            .then_some(row.observed_order)
            .flatten()
            .filter(|value| value.is_finite())
    })
}

fn run_candidate_order_rows(
    candidate: &CandidateSpec,
    profile: UnifiedScreenProfile,
) -> Vec<CandidateOrderGateRow> {
    let (problem, y0) = match manufactured_vector_problem(6, 80.0, 10.0, 0.0, 0.0) {
        Ok(value) => value,
        Err(error) => {
            return vec![CandidateOrderGateRow {
                candidate_id: candidate.id().to_string(),
                family: candidate.family(),
                h: f64::NAN,
                steps: 0,
                error_l2: None,
                observed_order: None,
                roundoff_l2_floor: None,
                above_roundoff_floor: None,
                fast_steps: 0,
                fallback_steps: 0,
                all_fast: false,
                counters: WorkCounters::default(),
                failure: Some(error.to_string()),
            }];
        }
    };
    let final_time = 0.2;
    let exact = match problem.exact(final_time) {
        Some(value) => value,
        None => {
            return vec![CandidateOrderGateRow {
                candidate_id: candidate.id().to_string(),
                family: candidate.family(),
                h: f64::NAN,
                steps: 0,
                error_l2: None,
                observed_order: None,
                roundoff_l2_floor: None,
                above_roundoff_floor: None,
                fast_steps: 0,
                fallback_steps: 0,
                all_fast: false,
                counters: WorkCounters::default(),
                failure: Some("order problem has no exact solution".into()),
            }];
        }
    };
    let roundoff_l2_floor = 8.0 * f64::EPSILON * (1.0 + safe_l2(&exact));
    let mut rows = Vec::new();
    let mut previous: Option<(f64, f64)> = None;
    for &h in order_step_sizes(profile) {
        match integrate_candidate_fixed(candidate, &problem, &y0, final_time, h) {
            Ok(result) => {
                let error = safe_l2(
                    &result
                        .state
                        .iter()
                        .zip(&exact)
                        .map(|(computed, reference)| computed - reference)
                        .collect::<Vec<_>>(),
                );
                let observed_order = previous.and_then(|(previous_h, previous_error)| {
                    (error > 0.0 && previous_error > 0.0 && previous_h > h)
                        .then(|| (previous_error / error).ln() / (previous_h / h).ln())
                        .filter(|value| value.is_finite())
                });
                rows.push(CandidateOrderGateRow {
                    candidate_id: candidate.id().to_string(),
                    family: candidate.family(),
                    h,
                    steps: result.steps,
                    error_l2: Some(error),
                    observed_order,
                    roundoff_l2_floor: Some(roundoff_l2_floor),
                    above_roundoff_floor: Some(error > roundoff_l2_floor),
                    fast_steps: result.fast_steps,
                    fallback_steps: result.fallback_steps,
                    all_fast: result.fallback_steps == 0,
                    counters: result.counters,
                    failure: None,
                });
                previous = Some((h, error));
            }
            Err(error) => rows.push(CandidateOrderGateRow {
                candidate_id: candidate.id().to_string(),
                family: candidate.family(),
                h,
                steps: 0,
                error_l2: None,
                observed_order: None,
                roundoff_l2_floor: Some(roundoff_l2_floor),
                above_roundoff_floor: None,
                fast_steps: 0,
                fallback_steps: 0,
                all_fast: false,
                counters: WorkCounters::default(),
                failure: Some(error.to_string()),
            }),
        }
    }
    rows
}

fn run_candidate_stiff_row(
    candidate: &CandidateSpec,
    protected_amplification: f64,
) -> CandidateStiffGateRow {
    let lambda = -1_000.0;
    let h = 0.1;
    let z = lambda * h;
    let (problem, y0) = scalar_linear_problem(lambda, 1.0);
    let allowed = STIFF_ABSOLUTE_FLOOR.max(STIFF_REFERENCE_FACTOR * protected_amplification);
    match integrate_candidate_fixed(candidate, &problem, &y0, h, h) {
        Ok(result) => {
            let amplification = result.state[0].abs();
            let final_pass = amplification.is_finite() && amplification <= allowed;
            CandidateStiffGateRow {
                candidate_id: candidate.id().to_string(),
                family: candidate.family(),
                z,
                amplification: Some(amplification),
                protected_amplification,
                allowed_amplification: allowed,
                final_pass,
                fast_pass: final_pass && result.fallback_steps == 0,
                used_fallback: result.fallback_steps > 0,
                counters: result.counters,
                failure: None,
            }
        }
        Err(error) => CandidateStiffGateRow {
            candidate_id: candidate.id().to_string(),
            family: candidate.family(),
            z,
            amplification: None,
            protected_amplification,
            allowed_amplification: allowed,
            final_pass: false,
            fast_pass: false,
            used_fallback: false,
            counters: WorkCounters::default(),
            failure: Some(error.to_string()),
        },
    }
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

fn candidate_report(
    candidate: &CandidateSpec,
    order_rows: &[CandidateOrderGateRow],
    stiff: &CandidateStiffGateRow,
    nonlinear: &UnifiedNonlinearScreen,
) -> CandidateGateReport {
    if matches!(candidate.status(), CandidateStatus::Deferred { .. }) {
        return CandidateGateReport {
            candidate_id: candidate.id().to_string(),
            family: candidate.family(),
            status: candidate.status(),
            verdict: CandidateGateVerdict::Deferred,
            qualification_observed_order: None,
            finest_observed_order: None,
            order_pass: false,
            order_all_fast: false,
            stiff_decay_pass: false,
            stiff_fast_pass: false,
            one_step_rows: 0,
            one_step_rejections: 0,
            one_step_failures: 0,
            c3_false_accepts: 0,
            c3_reference_fallbacks: 0,
            nonnormal_pass: false,
            maximum_oracle_output_wrms: None,
            median_batch_depth: None,
            median_batch_vectors: None,
            measurable_advantage: false,
            blockers: vec!["Rust implementation is deferred".into()],
        };
    }

    let rows: Vec<_> = nonlinear
        .rows
        .iter()
        .filter(|row| row.candidate_id == candidate.id())
        .collect();
    let severe_rows: Vec<_> = rows
        .iter()
        .copied()
        .filter(|row| {
            if nonlinear.profile == "canonical" {
                row.case_id.contains("eta0.9") || row.case_id.contains("l1000000")
            } else {
                row.case_id.contains("manufactured-vector") || row.case_id.starts_with("mv-")
            }
        })
        .collect();
    let gate_rows = if severe_rows.is_empty() {
        rows.as_slice()
    } else {
        severe_rows.as_slice()
    };
    let finest_observed_order = finest_observed_order(order_rows);
    let qualification_observed_order = qualification_observed_order(order_rows);
    let order_pass = qualification_observed_order.is_some_and(|value| value >= ORDER_PASS_FLOOR)
        && order_rows.iter().all(|row| row.failure.is_none());
    let order_all_fast = order_rows.iter().all(|row| row.all_fast);
    let one_step_rejections = rows
        .iter()
        .filter(|row| row.outcome == UnifiedCandidateOutcome::Rejected)
        .count();
    let one_step_failures = rows
        .iter()
        .filter(|row| {
            matches!(
                row.outcome,
                UnifiedCandidateOutcome::NumericalFailure | UnifiedCandidateOutcome::Uncertified
            )
        })
        .count();
    let c3_false_accepts = rows.iter().filter(|row| row.c3_false_accept).count();
    let c3_reference_fallbacks = rows
        .iter()
        .filter(|row| row.reference_fallback_used)
        .count();
    let nonnormal_pass = !gate_rows.is_empty()
        && gate_rows.iter().all(|row| {
            row.outcome != UnifiedCandidateOutcome::NumericalFailure
                && row.outcome != UnifiedCandidateOutcome::Uncertified
                && row.oracle_output_budget_pass == Some(true)
                && (row.c3_output_budget_pass == Some(true)
                    || (row.reference_fallback_used && row.oracle_output_budget_pass == Some(true)))
                && !row.c3_false_accept
        });
    let maximum_oracle_output_wrms = rows
        .iter()
        .filter_map(|row| row.oracle_output_wrms)
        .max_by(f64::total_cmp);
    let median_batch_depth = median(rows.iter().map(|row| row.batch_depth as f64).collect());
    let median_batch_vectors = median(rows.iter().map(|row| row.batch_vectors as f64).collect());
    let measurable_advantage = median_batch_depth.is_some_and(|value| value < 8.0)
        || median_batch_vectors.is_some_and(|value| value < 8.0);

    let mut blockers = Vec::new();
    if !order_pass {
        blockers.push("global fifth-order gate failed".into());
    }
    if !order_all_fast && candidate.family() != CandidateFamily::Sequential {
        blockers.push("global-order trajectory used protected fallback".into());
    }
    if !stiff.final_pass {
        blockers.push("stiff-decay final-output gate failed".into());
    }
    if !stiff.fast_pass && candidate.family() != CandidateFamily::Sequential {
        blockers.push("stiff-decay gate required protected fallback".into());
    }
    if one_step_failures > 0 {
        blockers.push(format!(
            "{one_step_failures} one-step certification/execution failures"
        ));
    }
    if c3_false_accepts > 0 {
        blockers.push(format!("{c3_false_accepts} C3 false acceptances"));
    }
    if c3_reference_fallbacks > 0 {
        blockers.push(format!(
            "{c3_reference_fallbacks} C3 reference fallbacks required"
        ));
    }
    if !nonnormal_pass {
        blockers.push("nonnormal/noncommuting-mass output gate failed".into());
    }
    let verdict = if candidate.id() == "sequential-direct-off" {
        CandidateGateVerdict::Reference
    } else if blockers.is_empty() {
        CandidateGateVerdict::Promote
    } else {
        CandidateGateVerdict::Hold
    };
    CandidateGateReport {
        candidate_id: candidate.id().to_string(),
        family: candidate.family(),
        status: candidate.status(),
        verdict,
        qualification_observed_order,
        finest_observed_order,
        order_pass,
        order_all_fast,
        stiff_decay_pass: stiff.final_pass,
        stiff_fast_pass: stiff.fast_pass,
        one_step_rows: rows.len(),
        one_step_rejections,
        one_step_failures,
        c3_false_accepts,
        c3_reference_fallbacks,
        nonnormal_pass,
        maximum_oracle_output_wrms,
        median_batch_depth,
        median_batch_vectors,
        measurable_advantage,
        blockers,
    }
}

pub fn run_unified_scientific_gates(
    profile: UnifiedScreenProfile,
    threads: usize,
    nonlinear: &UnifiedNonlinearScreen,
) -> CoreResult<UnifiedScientificGateReport> {
    if nonlinear.profile != profile.as_str() {
        return Err(CoreError::InvalidInput(
            "unified nonlinear screen profile does not match gate profile".into(),
        ));
    }
    let catalog = CandidateCatalog::research_default()?;
    let executable: Vec<CandidateSpec> = catalog
        .executable()
        .filter(|candidate| candidate.is_rodas_stage_candidate())
        .cloned()
        .collect();
    let execution = ParallelExecution::rayon(threads)?;
    let start = Instant::now();
    let order_nested = execution.map_ordered(&executable, |candidate| {
        Ok(run_candidate_order_rows(candidate, profile))
    })?;
    let mut order_rows: Vec<CandidateOrderGateRow> = order_nested.into_iter().flatten().collect();
    order_rows.sort_by(|left, right| {
        left.candidate_id
            .cmp(&right.candidate_id)
            .then_with(|| right.h.total_cmp(&left.h))
    });

    let protected = executable
        .iter()
        .find(|candidate| candidate.id() == "sequential-direct-off")
        .ok_or_else(|| CoreError::InvalidInput("protected candidate missing".into()))?;
    let (stiff_problem, stiff_y0) = scalar_linear_problem(-1_000.0, 1.0);
    let protected_stiff =
        integrate_candidate_fixed(protected, &stiff_problem, &stiff_y0, 0.1, 0.1)?;
    let protected_amplification = protected_stiff.state[0].abs();
    let mut stiff_rows = execution.map_ordered(&executable, |candidate| {
        Ok(run_candidate_stiff_row(candidate, protected_amplification))
    })?;
    stiff_rows.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    let mut candidates = Vec::with_capacity(catalog.entries().len());
    for candidate in catalog.entries() {
        if matches!(candidate.status(), CandidateStatus::Deferred { .. }) {
            candidates.push(candidate_report(
                candidate,
                &[],
                &CandidateStiffGateRow {
                    candidate_id: candidate.id().to_string(),
                    family: candidate.family(),
                    z: -100.0,
                    amplification: None,
                    protected_amplification,
                    allowed_amplification: STIFF_ABSOLUTE_FLOOR
                        .max(STIFF_REFERENCE_FACTOR * protected_amplification),
                    final_pass: false,
                    fast_pass: false,
                    used_fallback: false,
                    counters: WorkCounters::default(),
                    failure: Some("deferred".into()),
                },
                nonlinear,
            ));
            continue;
        }
        if candidate.is_native_complete_integrator() {
            candidates.push(CandidateGateReport {
                candidate_id: candidate.id().to_string(),
                family: candidate.family(),
                status: candidate.status(),
                verdict: CandidateGateVerdict::Hold,
                qualification_observed_order: None,
                finest_observed_order: None,
                order_pass: false,
                order_all_fast: false,
                stiff_decay_pass: false,
                stiff_fast_pass: false,
                one_step_rows: 0,
                one_step_rejections: 0,
                one_step_failures: 0,
                c3_false_accepts: 0,
                c3_reference_fallbacks: 0,
                nonnormal_pass: false,
                maximum_oracle_output_wrms: None,
                median_batch_depth: None,
                median_batch_vectors: None,
                measurable_advantage: false,
                blockers: vec![
                    "complete-integrator candidate is evaluated by the native integrator gate, not the RODAS-stage gate".into(),
                ],
            });
            continue;
        }
        let candidate_orders: Vec<_> = order_rows
            .iter()
            .filter(|row| row.candidate_id == candidate.id())
            .cloned()
            .collect();
        let stiff = stiff_rows
            .iter()
            .find(|row| row.candidate_id == candidate.id())
            .ok_or_else(|| CoreError::InvalidInput("candidate stiff gate missing".into()))?;
        candidates.push(candidate_report(
            candidate,
            &candidate_orders,
            stiff,
            nonlinear,
        ));
    }
    candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    let failures = order_rows
        .iter()
        .filter(|row| row.failure.is_some())
        .count()
        + stiff_rows
            .iter()
            .filter(|row| row.failure.is_some())
            .count();
    Ok(UnifiedScientificGateReport {
        schema: "rodas5p-unified-scientific-gates-v3",
        status: if failures == 0 {
            "complete"
        } else {
            "complete-with-failures"
        },
        profile: profile.as_str(),
        threads,
        order_pass_floor: ORDER_PASS_FLOOR,
        stiff_absolute_floor: STIFF_ABSOLUTE_FLOOR,
        stiff_reference_factor: STIFF_REFERENCE_FACTOR,
        order_rows,
        stiff_rows,
        candidates,
        compute_seconds: start.elapsed().as_secs_f64(),
    })
}

#[cfg(test)]
mod qualification_tests {
    use super::*;

    fn row(order: Option<f64>, error: f64, floor: f64) -> CandidateOrderGateRow {
        CandidateOrderGateRow {
            candidate_id: "candidate".into(),
            family: CandidateFamily::Sequential,
            h: 0.01,
            steps: 1,
            error_l2: Some(error),
            observed_order: order,
            roundoff_l2_floor: Some(floor),
            above_roundoff_floor: Some(error > floor),
            fast_steps: 1,
            fallback_steps: 0,
            all_fast: true,
            counters: WorkCounters::default(),
            failure: None,
        }
    }

    #[test]
    fn qualification_order_ignores_a_roundoff_contaminated_finest_pair() {
        let rows = vec![
            row(None, 9.0e-12, 6.0e-15),
            row(Some(5.07), 2.7e-13, 6.0e-15),
            row(Some(4.94), 8.8e-15, 6.0e-15),
            row(Some(3.18), 9.7e-16, 6.0e-15),
        ];
        assert_eq!(qualification_observed_order(&rows), Some(4.94));
        assert_eq!(finest_observed_order(&rows), Some(3.18));
    }

    #[test]
    fn qualification_order_keeps_the_finest_pair_when_it_is_above_roundoff() {
        let rows = vec![
            row(None, 1.0e-3, 1.0e-14),
            row(Some(2.0), 2.5e-4, 1.0e-14),
            row(Some(1.5), 8.8e-5, 1.0e-14),
        ];
        assert_eq!(qualification_observed_order(&rows), Some(1.5));
    }

    #[test]
    fn iterative_sequential_order_rows_survive_roundoff_level_refinement() {
        let catalog = CandidateCatalog::research_default().unwrap();
        for candidate_id in [
            "sequential-gmres-off",
            "sequential-lgmres-off",
            "sequential-lgmres-stage",
            "sequential-lgmres-persistent",
            "sequential-gcrodr-off",
            "sequential-gcrodr-stage",
            "sequential-gcrodr-persistent",
        ] {
            let candidate = catalog
                .entries()
                .iter()
                .find(|candidate| candidate.id() == candidate_id)
                .unwrap();
            let rows = run_candidate_order_rows(candidate, UnifiedScreenProfile::Canonical);
            assert!(
                rows.iter().all(|row| row.failure.is_none()),
                "{candidate_id}: {:?}",
                rows.iter()
                    .filter_map(|row| row.failure.as_deref())
                    .collect::<Vec<_>>()
            );
        }
    }
}
