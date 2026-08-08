use rodas5p_core::{CoreError, CoreResult, WorkCounters, safe_l2};
use serde::Serialize;

use crate::{
    BdfConfig, BdfOrder, CandidateCatalog, CandidateExecution, CandidateFamily, CandidateStatus,
    OdeProblem, RadauConfig, RadauIiaStages, integrate_bdf_fixed, integrate_radau_fixed,
    manufactured_mass_nonlinear_problem, scalar_linear_problem,
};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NativeIntegratorGateRow {
    pub candidate_id: String,
    pub family: CandidateFamily,
    pub target_order: usize,
    pub observed_order: Option<f64>,
    pub order_pass: bool,
    pub stiff_amplification: f64,
    pub stiff_pass: bool,
    pub mass_error_l2: f64,
    pub mass_pass: bool,
    pub order_counters: WorkCounters,
    pub stiff_counters: WorkCounters,
    pub mass_counters: WorkCounters,
    pub failures: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NativeIntegratorGateReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub rows: Vec<NativeIntegratorGateRow>,
}

#[derive(Clone, Debug)]
struct NativeTrajectory {
    state: Vec<f64>,
    counters: WorkCounters,
}

fn integrate_candidate(
    execution: &CandidateExecution,
    problem: &OdeProblem,
    y0: &[f64],
    final_time: f64,
    h: f64,
) -> CoreResult<NativeTrajectory> {
    match execution {
        CandidateExecution::Bdf { order } => {
            let result = integrate_bdf_fixed(
                problem,
                (0.0, final_time),
                y0,
                h,
                &BdfConfig {
                    order: *order,
                    ..BdfConfig::default()
                },
            )?;
            Ok(NativeTrajectory {
                state: result
                    .y
                    .last()
                    .cloned()
                    .ok_or_else(|| CoreError::InvalidInput("BDF trajectory is empty".into()))?,
                counters: result.counters,
            })
        }
        CandidateExecution::RadauIrk { stages } => {
            let result = integrate_radau_fixed(
                problem,
                (0.0, final_time),
                y0,
                h,
                &RadauConfig {
                    stages: *stages,
                    ..RadauConfig::default()
                },
            )?;
            Ok(NativeTrajectory {
                state: result
                    .y
                    .last()
                    .cloned()
                    .ok_or_else(|| CoreError::InvalidInput("Radau trajectory is empty".into()))?,
                counters: result.counters,
            })
        }
        _ => Err(CoreError::InvalidInput(
            "candidate is not a native BDF/Radau anchor".into(),
        )),
    }
}

fn target_order(execution: &CandidateExecution) -> Option<usize> {
    match execution {
        CandidateExecution::Bdf {
            order: BdfOrder::One,
        } => Some(1),
        CandidateExecution::Bdf {
            order: BdfOrder::Two,
        } => Some(2),
        CandidateExecution::RadauIrk {
            stages: RadauIiaStages::One,
        } => Some(1),
        CandidateExecution::RadauIrk {
            stages: RadauIiaStages::Three,
        } => Some(5),
        _ => None,
    }
}

fn l2_error(computed: &[f64], exact: &[f64]) -> f64 {
    safe_l2(
        &computed
            .iter()
            .zip(exact)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    )
}

fn order_gate(execution: &CandidateExecution, target: usize) -> CoreResult<(f64, WorkCounters)> {
    let (problem, y0) = scalar_linear_problem(-4.0, 1.0);
    let exact = problem
        .exact(0.2)
        .ok_or_else(|| CoreError::InvalidInput("order gate lacks exact solution".into()))?;
    let mut errors = Vec::new();
    let mut final_counters = WorkCounters::default();
    for h in [0.04, 0.02, 0.01, 0.005] {
        let result = integrate_candidate(execution, &problem, &y0, 0.2, h)?;
        errors.push(l2_error(&result.state, &exact));
        final_counters = result.counters;
    }
    let mut orders: Vec<f64> = errors
        .windows(2)
        .filter_map(|pair| {
            (pair[0] > 0.0 && pair[1] > 0.0)
                .then(|| (pair[0] / pair[1]).ln() / 2.0_f64.ln())
                .filter(|value| value.is_finite())
        })
        .collect();
    if orders.is_empty() {
        return Err(CoreError::NonFinite(
            "native order gate produced no finite order".into(),
        ));
    }
    let observed = if target >= 4 && orders.len() >= 2 {
        let tail = &orders[orders.len() - 2..];
        tail.iter().sum::<f64>() / tail.len() as f64
    } else {
        orders.pop().expect("orders is nonempty")
    };
    Ok((observed, final_counters))
}

fn stiff_gate(execution: &CandidateExecution) -> CoreResult<(f64, WorkCounters)> {
    let (problem, y0) = scalar_linear_problem(-1000.0, 1.0);
    let result = integrate_candidate(execution, &problem, &y0, 0.04, 0.01)?;
    Ok((result.state[0].abs(), result.counters))
}

fn mass_gate(execution: &CandidateExecution) -> CoreResult<(f64, WorkCounters)> {
    let (problem, y0, _, _) = manufactured_mass_nonlinear_problem(20.0, 1.0, 0.2, 0.0)?;
    let result = integrate_candidate(execution, &problem, &y0, 0.05, 0.0025)?;
    let exact = problem
        .exact(0.05)
        .ok_or_else(|| CoreError::InvalidInput("mass gate lacks exact solution".into()))?;
    Ok((l2_error(&result.state, &exact), result.counters))
}

pub fn run_native_integrator_gates() -> CoreResult<NativeIntegratorGateReport> {
    let catalog = CandidateCatalog::research_default()?;
    let mut rows = Vec::new();
    for candidate in catalog.entries().iter().filter(|candidate| {
        matches!(candidate.status(), CandidateStatus::Executable)
            && matches!(
                candidate.family(),
                CandidateFamily::Bdf | CandidateFamily::RadauIrk
            )
    }) {
        let target = target_order(candidate.execution()).ok_or_else(|| {
            CoreError::InvalidInput("native executable has no target order".into())
        })?;
        let (observed_order, order_counters) = order_gate(candidate.execution(), target)?;
        let (stiff_amplification, stiff_counters) = stiff_gate(candidate.execution())?;
        let (mass_error_l2, mass_counters) = mass_gate(candidate.execution())?;
        let order_floor = target as f64 - if target >= 4 { 0.5 } else { 0.2 };
        let mass_floor = match target {
            1 => 1.0e-2,
            2 => 5.0e-4,
            _ => 1.0e-7,
        };
        rows.push(NativeIntegratorGateRow {
            candidate_id: candidate.id().to_string(),
            family: candidate.family(),
            target_order: target,
            observed_order: Some(observed_order),
            order_pass: observed_order >= order_floor,
            stiff_amplification,
            stiff_pass: stiff_amplification <= 0.1,
            mass_error_l2,
            mass_pass: mass_error_l2 <= mass_floor,
            order_counters,
            stiff_counters,
            mass_counters,
            failures: 0,
        });
    }
    rows.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    Ok(NativeIntegratorGateReport {
        schema: "rodas5p-native-integrator-gates-v1",
        status: "research-anchor",
        rows,
    })
}
