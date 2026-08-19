use std::time::Instant;

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use serde::{Deserialize, Serialize};

use crate::{
    FairResult, FairSolveConfig, FairSolveResult, LinearSystemTrace, RecycleLifetime,
    RecycleSessionManager, SolveStatus, SolverKind, StateTransition, TimingLedger, WorkLedger,
    solve_case,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BenchmarkCell {
    pub solver: SolverKind,
    pub lifetime: RecycleLifetime,
}

impl BenchmarkCell {
    pub const fn new(solver: SolverKind, lifetime: RecycleLifetime) -> Self {
        Self { solver, lifetime }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkPlan {
    pub cells: Vec<BenchmarkCell>,
    pub repetitions: usize,
    pub warmups: usize,
    pub seed: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceRunResult {
    pub trace_id: String,
    pub solver: SolverKind,
    pub lifetime: RecycleLifetime,
    pub solves: Vec<FairSolveResult>,
    pub ledger: WorkLedger,
    pub timing: TimingLedger,
    pub failures: usize,
    pub system_ids: Vec<String>,
    pub policy_resets: u64,
    pub transition_log: Vec<StateTransition>,
    pub trace_wall_seconds: f64,
    pub repetition: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub trace_id: String,
    pub runs: Vec<TraceRunResult>,
    pub execution_order: Vec<Vec<BenchmarkCell>>,
    pub warmups: usize,
    pub seed: u64,
}

pub fn build_execution_order(
    cells: &[BenchmarkCell],
    repetitions: usize,
    seed: u64,
) -> FairResult<Vec<Vec<BenchmarkCell>>> {
    if cells.is_empty() || repetitions == 0 {
        return Err(crate::FairError::Invalid(
            "benchmark requires cells and repetitions".into(),
        ));
    }
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let mut orders = Vec::with_capacity(repetitions);
    for _ in 0..repetitions {
        let mut order = cells.to_vec();
        for i in (1..order.len()).rev() {
            let j = rng.random_range(0..=i);
            order.swap(i, j);
        }
        orders.push(order);
    }
    Ok(orders)
}

pub fn run_trace(
    trace: &LinearSystemTrace,
    config: &FairSolveConfig,
    lifetime: RecycleLifetime,
    repetition: usize,
) -> FairResult<TraceRunResult> {
    config.validate()?;
    let trace_start = Instant::now();
    let mut manager = RecycleSessionManager::new(config.solver, lifetime);
    let mut results = Vec::with_capacity(trace.cases.len());
    let mut previous_oracle: Option<Vec<f64>> = None;

    for case in &trace.cases {
        let x0 = if config.use_previous_oracle_guess {
            previous_oracle.as_deref()
        } else {
            None
        };
        let result = {
            let session = manager.acquire(case.step_index);
            solve_case(case, config, Some(session), x0)?
        };
        if result.status != SolveStatus::Converged {
            manager.reset(
                case.step_index,
                format!("linear_{:?}", result.status).to_lowercase(),
            );
        }
        previous_oracle = Some(case.oracle_solution.clone());
        results.push(result);
    }

    let mut ledger = WorkLedger::default();
    let mut timing = TimingLedger::default();
    for result in &results {
        ledger.add_assign(result.ledger);
        timing.add_assign(result.timing);
    }
    ledger.recycle_resets = manager.reset_count;
    let failures = results
        .iter()
        .filter(|result| result.status != SolveStatus::Converged)
        .count();
    Ok(TraceRunResult {
        trace_id: trace.trace_id.clone(),
        solver: config.solver,
        lifetime,
        system_ids: results
            .iter()
            .map(|result| result.system_id.clone())
            .collect(),
        solves: results,
        ledger,
        timing,
        failures,
        policy_resets: manager.reset_count,
        transition_log: manager.transition_log,
        trace_wall_seconds: trace_start.elapsed().as_secs_f64(),
        repetition,
    })
}

pub fn run_comparison<F>(
    trace: &LinearSystemTrace,
    plan: &BenchmarkPlan,
    config_for: F,
) -> FairResult<ComparisonResult>
where
    F: Fn(SolverKind) -> FairSolveConfig,
{
    if plan.cells.is_empty() || plan.repetitions == 0 {
        return Err(crate::FairError::Invalid("invalid benchmark plan".into()));
    }
    for _ in 0..plan.warmups {
        for cell in &plan.cells {
            let config = config_for(cell.solver);
            let _ = run_trace(trace, &config, cell.lifetime, usize::MAX)?;
        }
    }
    let execution_order = build_execution_order(&plan.cells, plan.repetitions, plan.seed)?;
    let mut runs = Vec::with_capacity(plan.cells.len() * plan.repetitions);
    for (repetition, order) in execution_order.iter().enumerate() {
        for cell in order {
            let config = config_for(cell.solver);
            runs.push(run_trace(trace, &config, cell.lifetime, repetition)?);
        }
    }
    Ok(ComparisonResult {
        trace_id: trace.trace_id.clone(),
        runs,
        execution_order,
        warmups: plan.warmups,
        seed: plan.seed,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SummaryRow {
    pub solver: SolverKind,
    pub lifetime: RecycleLifetime,
    pub repetitions: usize,
    pub failures: usize,
    pub wall_median_seconds: f64,
    pub wall_q25_seconds: f64,
    pub wall_q75_seconds: f64,
    pub operator_total_median: f64,
    pub maximum_relative_solution_error: f64,
}

fn quantile(mut values: Vec<f64>, probability: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    if values.is_empty() {
        return f64::NAN;
    }
    let position = probability * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        values[lower]
    } else {
        let weight = position - lower as f64;
        values[lower] * (1.0 - weight) + values[upper] * weight
    }
}

pub fn summarize_comparison(result: &ComparisonResult) -> Vec<SummaryRow> {
    let mut cells = result
        .runs
        .iter()
        .map(|run| BenchmarkCell::new(run.solver, run.lifetime))
        .collect::<Vec<_>>();
    cells.sort();
    cells.dedup();
    cells
        .into_iter()
        .map(|cell| {
            let group: Vec<&TraceRunResult> = result
                .runs
                .iter()
                .filter(|run| run.solver == cell.solver && run.lifetime == cell.lifetime)
                .collect();
            let wall: Vec<f64> = group.iter().map(|run| run.trace_wall_seconds).collect();
            let operator: Vec<f64> = group
                .iter()
                .map(|run| run.ledger.operator_total() as f64)
                .collect();
            let maximum_relative_solution_error = group
                .iter()
                .flat_map(|run| &run.solves)
                .map(|solve| solve.relative_solution_error)
                .fold(0.0, f64::max);
            SummaryRow {
                solver: cell.solver,
                lifetime: cell.lifetime,
                repetitions: group.len(),
                failures: group.iter().map(|run| run.failures).sum(),
                wall_median_seconds: quantile(wall.clone(), 0.5),
                wall_q25_seconds: quantile(wall.clone(), 0.25),
                wall_q75_seconds: quantile(wall, 0.75),
                operator_total_median: quantile(operator, 0.5),
                maximum_relative_solution_error,
            }
        })
        .collect()
}
