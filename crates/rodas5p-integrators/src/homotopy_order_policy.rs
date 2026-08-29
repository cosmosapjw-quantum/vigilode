use std::collections::BTreeMap;

use rodas5p_core::{
    CoreError, CoreResult, LinearMethod, LinearSolverConfig, WorkCounters, safe_l2,
};
use serde::Serialize;

use crate::{
    HomotopyExperimentProfile, HomotopyExperimentSummary, HomotopyPathConfig, HomotopyPredictor,
    HomotopyStepConfig, OdeProblem, OutputBudgetPolicy, ParallelExecution, homotopy_step,
    manufactured_mass_nonlinear_problem, manufactured_vector_problem, prothero_robinson_problem,
    run_homotopy_experiment_screen, sequential_step,
};

const STEP_REFERENCE: f64 = 0.04;
const STEP_EXPONENT: u32 = 6;
const POLICY_ATOL: f64 = 1e-7;
const POLICY_RTOL: f64 = 1e-6;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PolicyExecutionMetadata {
    pub backend: String,
    pub threads: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PolicyReplayRow {
    pub case_id: String,
    pub case_family: String,
    pub split: String,
    pub policy_id: String,
    pub policy_family: String,
    pub theta: f64,
    pub q: usize,
    pub path_rounds: usize,
    pub predictor: HomotopyPredictor,
    pub corrections_per_point: usize,
    pub evaluable: bool,
    pub policy_budget: Option<f64>,
    pub output_wrms: Option<f64>,
    pub embedded_error: Option<f64>,
    pub oracle_output_wrms: Option<f64>,
    pub accepted: bool,
    pub false_accept: bool,
    pub low_depth: bool,
    pub w_solve_batches: Option<u64>,
    pub w_solve_vectors: Option<u64>,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PolicySplitSummary {
    pub policy_id: String,
    pub policy_family: String,
    pub split: String,
    pub rows: usize,
    pub evaluable_rows: usize,
    pub accepts: usize,
    pub low_depth_accepts: usize,
    pub false_accepts: usize,
    pub failures: usize,
    pub fallback_fraction: Option<f64>,
    pub median_w_solve_batches_accepted: Option<f64>,
    pub median_w_solve_vectors_accepted: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PolicyFamilyWinner {
    pub policy_family: String,
    pub policy_id: String,
    pub calibration: PolicySplitSummary,
    pub holdout: PolicySplitSummary,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PolicyTrajectoryRow {
    pub problem_id: String,
    pub method: String,
    pub policy_id: Option<String>,
    pub policy_family: Option<String>,
    pub theta: Option<f64>,
    pub q: Option<usize>,
    pub path_rounds: Option<usize>,
    pub predictor: Option<HomotopyPredictor>,
    pub corrections_per_point: Option<usize>,
    pub h: f64,
    pub final_time: f64,
    pub steps: usize,
    pub error_l2: f64,
    pub observed_order: Option<f64>,
    pub order_pair_all_fast: bool,
    pub fast_accepts: usize,
    pub fallbacks: usize,
    pub all_fast: bool,
    pub low_depth: bool,
    pub w_solve_batches: u64,
    pub w_solve_vectors: u64,
    pub counters: WorkCounters,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PolicyTrajectoryGate {
    pub problem_id: String,
    pub method: String,
    pub policy_id: Option<String>,
    pub fifth_order_applicable: bool,
    pub fifth_order_pass: bool,
    pub stiff_regression_applicable: bool,
    pub stiff_regression_pass: bool,
    pub low_depth_nonempty: bool,
    pub holdout_false_accepts: usize,
    pub promote: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyOrderPolicyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub execution: PolicyExecutionMetadata,
    pub source_summary: HomotopyExperimentSummary,
    pub policies: Vec<OutputBudgetPolicy>,
    pub replay_rows: Vec<PolicyReplayRow>,
    pub policy_summaries: Vec<PolicySplitSummary>,
    pub family_winners: Vec<PolicyFamilyWinner>,
    pub trajectory_rows: Vec<PolicyTrajectoryRow>,
    pub trajectory_gates: Vec<PolicyTrajectoryGate>,
}

pub fn output_policy_grid() -> CoreResult<Vec<OutputBudgetPolicy>> {
    let mut policies = Vec::new();
    for epsilon in [0.1, 0.03, 0.01] {
        policies.push(OutputBudgetPolicy::absolute(epsilon)?);
    }
    for eta in [0.5, 0.2, 0.1, 0.05] {
        policies.push(OutputBudgetPolicy::embedded_relative(eta)?);
    }
    for epsilon_ref in [0.1, 0.03, 0.01] {
        policies.push(OutputBudgetPolicy::step_power(
            epsilon_ref,
            STEP_REFERENCE,
            STEP_EXPONENT,
        )?);
    }
    for eta in [0.5, 0.2, 0.1] {
        for epsilon_ref in [0.1, 0.03, 0.01] {
            policies.push(OutputBudgetPolicy::mixed(
                eta,
                epsilon_ref,
                STEP_REFERENCE,
                STEP_EXPONENT,
            )?);
        }
    }
    policies.sort_by_key(OutputBudgetPolicy::id);
    Ok(policies)
}

fn stable_case_split(case_id: &str) -> &'static str {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in case_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    if hash & 1 == 0 {
        "calibration"
    } else {
        "holdout"
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

fn summarize_policy_split(
    policy: &OutputBudgetPolicy,
    split: &str,
    rows: &[PolicyReplayRow],
) -> PolicySplitSummary {
    let selected: Vec<&PolicyReplayRow> = rows
        .iter()
        .filter(|row| row.policy_id == policy.id() && row.split == split)
        .collect();
    let evaluable_rows = selected.iter().filter(|row| row.evaluable).count();
    let accepts = selected.iter().filter(|row| row.accepted).count();
    PolicySplitSummary {
        policy_id: policy.id(),
        policy_family: policy.family().into(),
        split: split.into(),
        rows: selected.len(),
        evaluable_rows,
        accepts,
        low_depth_accepts: selected
            .iter()
            .filter(|row| row.accepted && row.low_depth)
            .count(),
        false_accepts: selected.iter().filter(|row| row.false_accept).count(),
        failures: selected.iter().filter(|row| row.failure.is_some()).count(),
        fallback_fraction: (evaluable_rows > 0)
            .then_some(1.0 - accepts as f64 / evaluable_rows as f64),
        median_w_solve_batches_accepted: median(
            selected
                .iter()
                .filter(|row| row.accepted)
                .filter_map(|row| row.w_solve_batches.map(|value| value as f64))
                .collect(),
        ),
        median_w_solve_vectors_accepted: median(
            selected
                .iter()
                .filter(|row| row.accepted)
                .filter_map(|row| row.w_solve_vectors.map(|value| value as f64))
                .collect(),
        ),
    }
}

fn winner_key(summary: &PolicySplitSummary) -> (usize, usize, usize, u64, String) {
    let batches = summary
        .median_w_solve_batches_accepted
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(f64::to_bits)
        .unwrap_or(u64::MAX);
    (
        summary.false_accepts,
        usize::MAX - summary.low_depth_accepts,
        usize::MAX - summary.accepts,
        batches,
        summary.policy_id.clone(),
    )
}

#[derive(Clone)]
enum TrajectoryProblemKind {
    ManufacturedVector,
    ProtheroRobinson,
    ManufacturedMass,
}

#[derive(Clone)]
struct TrajectoryProblemSpec {
    problem_id: &'static str,
    kind: TrajectoryProblemKind,
    final_time: f64,
    step_sizes: Vec<f64>,
}

impl TrajectoryProblemSpec {
    fn build(&self) -> CoreResult<(OdeProblem, Vec<f64>)> {
        match self.kind {
            TrajectoryProblemKind::ManufacturedVector => {
                manufactured_vector_problem(6, 80.0, 10.0, 0.0, 0.0)
            }
            TrajectoryProblemKind::ProtheroRobinson => {
                Ok(prothero_robinson_problem(-10_000.0, 1_000.0, 0.0))
            }
            TrajectoryProblemKind::ManufacturedMass => {
                let (problem, y0, _, _) =
                    manufactured_mass_nonlinear_problem(1_000.0, 100.0, 0.9, 0.0)?;
                Ok((problem, y0))
            }
        }
    }
}

#[derive(Clone)]
enum TrajectoryMethod {
    Sequential,
    Homotopy {
        label: &'static str,
        path: HomotopyPathConfig,
    },
}

impl TrajectoryMethod {
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

#[derive(Clone)]
struct TrajectoryJob {
    problem: TrajectoryProblemSpec,
    method: TrajectoryMethod,
    policy: Option<OutputBudgetPolicy>,
    h: f64,
}

fn trajectory_problem_specs(profile: HomotopyExperimentProfile) -> Vec<TrajectoryProblemSpec> {
    let mut specs = vec![TrajectoryProblemSpec {
        problem_id: "manufactured-vector-order",
        kind: TrajectoryProblemKind::ManufacturedVector,
        final_time: 0.2,
        step_sizes: match profile {
            HomotopyExperimentProfile::Smoke => vec![0.04, 0.02],
            HomotopyExperimentProfile::Canonical => vec![0.04, 0.02, 0.01, 0.005],
        },
    }];
    if profile == HomotopyExperimentProfile::Canonical {
        specs.push(TrajectoryProblemSpec {
            problem_id: "prothero-robinson-stiff",
            kind: TrajectoryProblemKind::ProtheroRobinson,
            final_time: 0.04,
            step_sizes: vec![0.01, 0.005, 0.0025, 0.00125],
        });
        specs.push(TrajectoryProblemSpec {
            problem_id: "manufactured-mass-nonnormal",
            kind: TrajectoryProblemKind::ManufacturedMass,
            final_time: 0.04,
            step_sizes: vec![0.01, 0.005, 0.0025, 0.00125],
        });
    }
    specs
}

fn trajectory_methods() -> CoreResult<Vec<TrajectoryMethod>> {
    Ok(vec![
        TrajectoryMethod::Sequential,
        TrajectoryMethod::Homotopy {
            label: "homotopy-theta0-q0-r2-ab2-c0",
            path: HomotopyPathConfig::new(0.0, 0, 2, HomotopyPredictor::AdamsBashforth2, 0)?,
        },
        TrajectoryMethod::Homotopy {
            label: "homotopy-theta0-q1-r2-ab2-c0",
            path: HomotopyPathConfig::new(0.0, 1, 2, HomotopyPredictor::AdamsBashforth2, 0)?,
        },
        TrajectoryMethod::Homotopy {
            label: "homotopy-theta0-q2-r2-ab2-c0",
            path: HomotopyPathConfig::new(0.0, 2, 2, HomotopyPredictor::AdamsBashforth2, 0)?,
        },
        TrajectoryMethod::Homotopy {
            label: "homotopy-theta1-q2-r2-ab2-c1",
            path: HomotopyPathConfig::new(1.0, 2, 2, HomotopyPredictor::AdamsBashforth2, 1)?,
        },
        TrajectoryMethod::Homotopy {
            label: "homotopy-theta1-q7-r2-ab2-c1",
            path: HomotopyPathConfig::new(1.0, 7, 2, HomotopyPredictor::AdamsBashforth2, 1)?,
        },
    ])
}

fn run_trajectory_job(job: &TrajectoryJob) -> CoreResult<PolicyTrajectoryRow> {
    let (problem, y0) = job.problem.build()?;
    let exact = problem.exact(job.problem.final_time).ok_or_else(|| {
        CoreError::InvalidInput(format!(
            "trajectory problem {} lacks an exact solution",
            job.problem.problem_id
        ))
    })?;
    let direct = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let mut state = y0;
    let mut time = 0.0;
    let mut steps = 0_usize;
    let mut fast_accepts = 0_usize;
    let mut fallbacks = 0_usize;
    let mut w_solve_batches = 0_u64;
    let mut w_solve_vectors = 0_u64;
    let mut counters = WorkCounters::default();

    while time < job.problem.final_time - 10.0 * f64::EPSILON * job.problem.final_time.max(1.0) {
        let step_size = job.h.min(job.problem.final_time - time);
        match &job.method {
            TrajectoryMethod::Sequential => {
                let report = sequential_step(
                    &problem,
                    time,
                    &state,
                    step_size,
                    &direct,
                    None,
                    POLICY_ATOL,
                    POLICY_RTOL,
                    true,
                    &mut counters,
                )?;
                state = report.y_new;
                time = report.t_new;
            }
            TrajectoryMethod::Homotopy { path, .. } => {
                let policy = job.policy.clone().ok_or_else(|| {
                    CoreError::InvalidInput("homotopy trajectory job lacks a policy".into())
                })?;
                let config = HomotopyStepConfig::with_policy(path.clone(), policy)?;
                let report = homotopy_step(
                    &problem,
                    time,
                    &state,
                    step_size,
                    &config,
                    Some(&direct),
                    None,
                    POLICY_ATOL,
                    POLICY_RTOL,
                    true,
                    &mut counters,
                )?;
                fast_accepts += usize::from(report.fast_accepted);
                fallbacks += usize::from(report.step.used_fallback);
                if let Some(path_report) = &report.path {
                    w_solve_batches += path_report.work.w_solve_batches;
                    w_solve_vectors += path_report.work.w_solve_vectors;
                }
                state = report.step.y_new;
                time = report.step.t_new;
            }
        }
        steps += 1;
    }

    let error_l2 = safe_l2(
        &state
            .iter()
            .zip(exact)
            .map(|(computed, reference)| computed - reference)
            .collect::<Vec<_>>(),
    );
    let path = job.method.path();
    let all_fast = matches!(job.method, TrajectoryMethod::Sequential) || fast_accepts == steps;
    let low_depth =
        path.is_some() && all_fast && w_solve_batches < 8_u64.saturating_mul(steps as u64);
    Ok(PolicyTrajectoryRow {
        problem_id: job.problem.problem_id.into(),
        method: job.method.label().into(),
        policy_id: job.policy.as_ref().map(OutputBudgetPolicy::id),
        policy_family: job
            .policy
            .as_ref()
            .map(|policy| policy.family().to_string()),
        theta: path.map(HomotopyPathConfig::theta),
        q: path.map(HomotopyPathConfig::q),
        path_rounds: path.map(HomotopyPathConfig::path_rounds),
        predictor: path.map(HomotopyPathConfig::predictor),
        corrections_per_point: path.map(HomotopyPathConfig::corrections_per_point),
        h: job.h,
        final_time: job.problem.final_time,
        steps,
        error_l2,
        observed_order: None,
        order_pair_all_fast: false,
        fast_accepts,
        fallbacks,
        all_fast,
        low_depth,
        w_solve_batches,
        w_solve_vectors,
        counters,
        failure: None,
    })
}

fn add_observed_orders(rows: &mut [PolicyTrajectoryRow]) {
    let mut groups: BTreeMap<(String, String, Option<String>), Vec<usize>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        groups
            .entry((
                row.problem_id.clone(),
                row.method.clone(),
                row.policy_id.clone(),
            ))
            .or_default()
            .push(index);
    }
    for indices in groups.values_mut() {
        indices.sort_by(|left, right| rows[*right].h.total_cmp(&rows[*left].h));
        for pair in indices.windows(2) {
            let previous = pair[0];
            let current = pair[1];
            let previous_h = rows[previous].h;
            let current_h = rows[current].h;
            let previous_error = rows[previous].error_l2;
            let current_error = rows[current].error_l2;
            let order = if previous_h > current_h
                && previous_error > 0.0
                && current_error > 0.0
                && previous_error.is_finite()
                && current_error.is_finite()
            {
                Some((previous_error / current_error).ln() / (previous_h / current_h).ln())
                    .filter(|value| value.is_finite())
            } else {
                None
            };
            rows[current].observed_order = order;
            rows[current].order_pair_all_fast = rows[previous].all_fast && rows[current].all_fast;
        }
    }
}

fn build_trajectory_gates(
    rows: &[PolicyTrajectoryRow],
    replay_rows: &[PolicyReplayRow],
) -> Vec<PolicyTrajectoryGate> {
    let mut groups: BTreeMap<(String, String, Option<String>), Vec<&PolicyTrajectoryRow>> =
        BTreeMap::new();
    for row in rows {
        groups
            .entry((
                row.problem_id.clone(),
                row.method.clone(),
                row.policy_id.clone(),
            ))
            .or_default()
            .push(row);
    }
    let mut gates = Vec::new();
    for ((problem_id, method, policy_id), mut group) in groups {
        group.sort_by(|left, right| left.h.total_cmp(&right.h));
        let fifth_order_applicable = problem_id == "manufactured-vector-order";
        let relevant_orders: Vec<f64> = group
            .iter()
            .filter(|row| method == "sequential-direct" || row.order_pair_all_fast)
            .filter_map(|row| row.observed_order)
            .collect();
        let fifth_order_pass = !fifth_order_applicable
            || (!relevant_orders.is_empty()
                && relevant_orders
                    .iter()
                    .rev()
                    .take(2)
                    .all(|order| *order >= 4.8));

        let stiff_regression_applicable = problem_id != "manufactured-vector-order";
        let finest = group.first().copied();
        let sequential_finest = rows
            .iter()
            .filter(|row| row.problem_id == problem_id && row.method == "sequential-direct")
            .min_by(|left, right| left.h.total_cmp(&right.h));
        let stiff_regression_pass = if !stiff_regression_applicable || method == "sequential-direct"
        {
            true
        } else {
            match (finest, sequential_finest) {
                (Some(candidate), Some(control)) => {
                    candidate.all_fast
                        && candidate.error_l2.is_finite()
                        && control.error_l2.is_finite()
                        && candidate.error_l2 <= 5.0 * control.error_l2.max(f64::MIN_POSITIVE)
                }
                _ => false,
            }
        };
        let low_depth_nonempty = group.iter().any(|row| row.low_depth);
        let representative = group.first().copied();
        let holdout_false_accepts = match (policy_id.as_deref(), representative) {
            (Some(id), Some(path)) => replay_rows
                .iter()
                .filter(|row| row.split == "holdout" && row.false_accept)
                .filter(|row| row.policy_id == id)
                .filter(|row| {
                    path.theta
                        .is_some_and(|theta| row.theta.to_bits() == theta.to_bits())
                })
                .filter(|row| path.q == Some(row.q))
                .filter(|row| path.path_rounds == Some(row.path_rounds))
                .filter(|row| path.predictor == Some(row.predictor))
                .filter(|row| path.corrections_per_point == Some(row.corrections_per_point))
                .count(),
            _ => 0,
        };
        let promote = fifth_order_pass
            && stiff_regression_pass
            && holdout_false_accepts == 0
            && (method == "sequential-direct" || low_depth_nonempty);
        gates.push(PolicyTrajectoryGate {
            problem_id,
            method,
            policy_id,
            fifth_order_applicable,
            fifth_order_pass,
            stiff_regression_applicable,
            stiff_regression_pass,
            low_depth_nonempty,
            holdout_false_accepts,
            promote,
        });
    }
    gates.sort_by(|left, right| {
        left.problem_id
            .cmp(&right.problem_id)
            .then_with(|| left.method.cmp(&right.method))
            .then_with(|| left.policy_id.cmp(&right.policy_id))
    });
    gates
}

fn run_trajectory_screens(
    profile: HomotopyExperimentProfile,
    execution: &ParallelExecution,
    policies: &[OutputBudgetPolicy],
    winners: &[PolicyFamilyWinner],
    replay_rows: &[PolicyReplayRow],
) -> CoreResult<(Vec<PolicyTrajectoryRow>, Vec<PolicyTrajectoryGate>)> {
    let methods = trajectory_methods()?;
    let specs = trajectory_problem_specs(profile);
    let winner_policies: Vec<OutputBudgetPolicy> = winners
        .iter()
        .map(|winner| {
            policies
                .iter()
                .find(|policy| policy.id() == winner.policy_id)
                .cloned()
                .ok_or_else(|| {
                    CoreError::InvalidInput(format!(
                        "winner policy {} is absent from the policy grid",
                        winner.policy_id
                    ))
                })
        })
        .collect::<CoreResult<_>>()?;
    let mut jobs = Vec::new();
    for spec in specs {
        for &h in &spec.step_sizes {
            jobs.push(TrajectoryJob {
                problem: spec.clone(),
                method: TrajectoryMethod::Sequential,
                policy: None,
                h,
            });
            for policy in &winner_policies {
                for method in methods
                    .iter()
                    .filter(|method| matches!(method, TrajectoryMethod::Homotopy { .. }))
                {
                    jobs.push(TrajectoryJob {
                        problem: spec.clone(),
                        method: method.clone(),
                        policy: Some(policy.clone()),
                        h,
                    });
                }
            }
        }
    }
    let mut rows = execution.map_ordered(&jobs, run_trajectory_job)?;
    rows.sort_by(|left, right| {
        left.problem_id
            .cmp(&right.problem_id)
            .then_with(|| left.method.cmp(&right.method))
            .then_with(|| left.policy_id.cmp(&right.policy_id))
            .then_with(|| right.h.total_cmp(&left.h))
    });
    add_observed_orders(&mut rows);
    let gates = build_trajectory_gates(&rows, replay_rows);
    Ok((rows, gates))
}

pub fn run_homotopy_order_policy_screen(
    profile: HomotopyExperimentProfile,
    threads: usize,
) -> CoreResult<HomotopyOrderPolicyReport> {
    let execution = ParallelExecution::rayon(threads)?;
    let source = run_homotopy_experiment_screen(profile)?;
    let policies = output_policy_grid()?;
    let case_map: BTreeMap<&str, (&str, f64)> = source
        .cases
        .iter()
        .map(|case| (case.case_id.as_str(), (case.family.as_str(), case.h)))
        .collect();

    let policy_rows = execution.map_ordered(&policies, |policy| {
        let mut rows = Vec::with_capacity(source.candidates.len());
        for candidate in &source.candidates {
            let (case_family, h) = case_map.get(candidate.case_id.as_str()).ok_or_else(|| {
                CoreError::InvalidInput(format!(
                    "candidate references unknown case {}",
                    candidate.case_id
                ))
            })?;
            let policy_id = policy.id();
            let split = stable_case_split(&candidate.case_id).to_string();
            let work = candidate.work.as_ref();
            let result = match (candidate.output_wrms, candidate.embedded_error) {
                (Some(output_wrms), Some(embedded_error)) => {
                    match policy.decide(output_wrms, embedded_error, *h) {
                        Ok(decision) => {
                            let oracle = candidate.oracle_output_wrms;
                            PolicyReplayRow {
                                case_id: candidate.case_id.clone(),
                                case_family: (*case_family).into(),
                                split,
                                policy_id,
                                policy_family: policy.family().into(),
                                theta: candidate.theta,
                                q: candidate.q,
                                path_rounds: candidate.path_rounds,
                                predictor: candidate.predictor,
                                corrections_per_point: candidate.corrections_per_point,
                                evaluable: true,
                                policy_budget: Some(decision.budget),
                                output_wrms: Some(output_wrms),
                                embedded_error: Some(embedded_error),
                                oracle_output_wrms: oracle,
                                accepted: decision.accepted,
                                false_accept: decision.accepted
                                    && oracle.is_some_and(|value| value > decision.budget),
                                low_depth: work.is_some_and(|value| value.w_solve_batches < 8),
                                w_solve_batches: work.map(|value| value.w_solve_batches),
                                w_solve_vectors: work.map(|value| value.w_solve_vectors),
                                failure: None,
                            }
                        }
                        Err(error) => PolicyReplayRow {
                            case_id: candidate.case_id.clone(),
                            case_family: (*case_family).into(),
                            split,
                            policy_id,
                            policy_family: policy.family().into(),
                            theta: candidate.theta,
                            q: candidate.q,
                            path_rounds: candidate.path_rounds,
                            predictor: candidate.predictor,
                            corrections_per_point: candidate.corrections_per_point,
                            evaluable: false,
                            policy_budget: None,
                            output_wrms: Some(output_wrms),
                            embedded_error: Some(embedded_error),
                            oracle_output_wrms: candidate.oracle_output_wrms,
                            accepted: false,
                            false_accept: false,
                            low_depth: work.is_some_and(|value| value.w_solve_batches < 8),
                            w_solve_batches: work.map(|value| value.w_solve_batches),
                            w_solve_vectors: work.map(|value| value.w_solve_vectors),
                            failure: Some(error.to_string()),
                        },
                    }
                }
                _ => PolicyReplayRow {
                    case_id: candidate.case_id.clone(),
                    case_family: (*case_family).into(),
                    split,
                    policy_id,
                    policy_family: policy.family().into(),
                    theta: candidate.theta,
                    q: candidate.q,
                    path_rounds: candidate.path_rounds,
                    predictor: candidate.predictor,
                    corrections_per_point: candidate.corrections_per_point,
                    evaluable: false,
                    policy_budget: None,
                    output_wrms: candidate.output_wrms,
                    embedded_error: candidate.embedded_error,
                    oracle_output_wrms: candidate.oracle_output_wrms,
                    accepted: false,
                    false_accept: false,
                    low_depth: work.is_some_and(|value| value.w_solve_batches < 8),
                    w_solve_batches: work.map(|value| value.w_solve_batches),
                    w_solve_vectors: work.map(|value| value.w_solve_vectors),
                    failure: Some(
                        candidate
                            .failure
                            .clone()
                            .unwrap_or_else(|| "candidate lacks a complete certificate".into()),
                    ),
                },
            };
            rows.push(result);
        }
        Ok(rows)
    })?;
    let mut replay_rows: Vec<PolicyReplayRow> = policy_rows.into_iter().flatten().collect();
    replay_rows.sort_by(|a, b| {
        a.policy_id
            .cmp(&b.policy_id)
            .then_with(|| a.case_id.cmp(&b.case_id))
            .then_with(|| a.theta.total_cmp(&b.theta))
            .then_with(|| a.q.cmp(&b.q))
            .then_with(|| a.path_rounds.cmp(&b.path_rounds))
            .then_with(|| format!("{:?}", a.predictor).cmp(&format!("{:?}", b.predictor)))
            .then_with(|| a.corrections_per_point.cmp(&b.corrections_per_point))
    });

    let mut policy_summaries = Vec::new();
    for policy in &policies {
        for split in ["calibration", "holdout"] {
            policy_summaries.push(summarize_policy_split(policy, split, &replay_rows));
        }
    }
    policy_summaries.sort_by(|a, b| {
        a.policy_family
            .cmp(&b.policy_family)
            .then_with(|| a.policy_id.cmp(&b.policy_id))
            .then_with(|| a.split.cmp(&b.split))
    });

    let mut family_winners = Vec::new();
    for family in ["absolute", "embedded-relative", "step-power", "mixed"] {
        let mut calibration: Vec<PolicySplitSummary> = policy_summaries
            .iter()
            .filter(|summary| summary.policy_family == family && summary.split == "calibration")
            .cloned()
            .collect();
        calibration.sort_by_key(winner_key);
        let best = calibration.first().ok_or_else(|| {
            CoreError::InvalidInput(format!("no calibration policy found for family {family}"))
        })?;
        let holdout = policy_summaries
            .iter()
            .find(|summary| summary.policy_id == best.policy_id && summary.split == "holdout")
            .cloned()
            .ok_or_else(|| {
                CoreError::InvalidInput(format!(
                    "no holdout summary found for policy {}",
                    best.policy_id
                ))
            })?;
        family_winners.push(PolicyFamilyWinner {
            policy_family: family.into(),
            policy_id: best.policy_id.clone(),
            calibration: best.clone(),
            holdout,
        });
    }

    let (trajectory_rows, trajectory_gates) = run_trajectory_screens(
        profile,
        &execution,
        &policies,
        &family_winners,
        &replay_rows,
    )?;
    let false_accepts: usize = replay_rows.iter().filter(|row| row.false_accept).count();
    Ok(HomotopyOrderPolicyReport {
        schema: "rodas5p-homotopy-order-policy-screen-v1",
        status: if false_accepts == 0 {
            "policy-replay-complete"
        } else {
            "false-accept-detected"
        },
        profile: profile.as_str(),
        execution: PolicyExecutionMetadata {
            backend: execution.backend().into(),
            threads: execution.threads(),
        },
        source_summary: source.summary,
        policies,
        replay_rows,
        policy_summaries,
        family_winners,
        trajectory_rows,
        trajectory_gates,
    })
}
