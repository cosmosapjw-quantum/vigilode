use std::{sync::Arc, time::Instant};

use rodas5p_core::{
    ClosureOperator, CoreError, CoreResult, IdentityPreconditioner, LinearOperator, WorkCounters,
    safe_l2,
};
use rodas5p_krylov::{
    GmresConfig, GmresPrefixPrediction, GmresPrefixSession, solve_gmres_incremental,
};
use serde::{Deserialize, Serialize};

use crate::{
    FusedOrthogonalization, FusedPhiActionReport, FusedPhiKrylovConfig, FusedPhiPrefixPrediction,
    FusedPhiPrefixSession, fused_phi_action_incremental,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum G4PrefixKernelProfile {
    Smoke,
    Canonical,
}

impl G4PrefixKernelProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Canonical => "canonical",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4PrefixKernelRow {
    pub case_id: String,
    pub dimension: usize,
    pub stiffness: f64,
    pub nonnormality: f64,
    pub oscillation: f64,
    pub scale: f64,
    pub prefix_dimension: usize,
    pub completed: bool,
    pub failure: Option<String>,

    pub exponential_cold_wall_seconds: Option<f64>,
    pub exponential_prefix_wall_seconds: Option<f64>,
    pub exponential_resume_wall_seconds: Option<f64>,
    pub exponential_total_reused_wall_seconds: Option<f64>,
    pub exponential_actual_dimension: Option<usize>,
    pub exponential_predicted_dimension: Option<usize>,
    pub exponential_prefix_cost_inflation: Option<f64>,
    pub exponential_value_relative_defect: Option<f64>,
    pub exponential_jvp_vectors: Option<u64>,

    pub gmres_cold_wall_seconds: Option<f64>,
    pub gmres_prefix_wall_seconds: Option<f64>,
    pub gmres_resume_wall_seconds: Option<f64>,
    pub gmres_total_reused_wall_seconds: Option<f64>,
    pub gmres_actual_iterations: Option<usize>,
    pub gmres_predicted_iterations: Option<usize>,
    pub gmres_prefix_cost_inflation: Option<f64>,
    pub gmres_solution_relative_defect: Option<f64>,
    pub gmres_operator_vectors: Option<u64>,

    pub actual_exponential_to_gmres_cost_ratio: Option<f64>,
    pub predicted_exponential_to_gmres_cost_ratio: Option<f64>,
    pub cost_ratio_relative_prediction_error: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4PrefixKernelSummary {
    pub rows: usize,
    pub completed: usize,
    pub maximum_exponential_value_relative_defect: Option<f64>,
    pub maximum_gmres_solution_relative_defect: Option<f64>,
    pub median_cost_ratio_prediction_error: Option<f64>,
    pub p95_cost_ratio_prediction_error: Option<f64>,
    pub median_exponential_prefix_inflation: Option<f64>,
    pub p95_exponential_prefix_inflation: Option<f64>,
    pub median_gmres_prefix_inflation: Option<f64>,
    pub p95_gmres_prefix_inflation: Option<f64>,
    pub all_prefix_prediction_gate_pass: bool,
    pub all_prefix_cost_gate_pass: bool,
    pub selected_prefix_dimension: usize,
    pub selected_rows: usize,
    pub selected_p95_cost_ratio_prediction_error: Option<f64>,
    pub selected_p95_exponential_prefix_inflation: Option<f64>,
    pub selected_p95_gmres_prefix_inflation: Option<f64>,
    pub selected_prediction_gate_pass: bool,
    pub selected_prefix_cost_gate_pass: bool,
    pub reuse_parity_gate_pass: bool,
    pub active_switching_authorized: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4PrefixKernelReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub prefix_kernel: &'static str,
    pub rows: Vec<G4PrefixKernelRow>,
    pub summary: G4PrefixKernelSummary,
    pub limitations: Vec<String>,
}

#[derive(Clone, Copy)]
struct PrefixCase {
    dimension: usize,
    stiffness: f64,
    nonnormality: f64,
    oscillation: f64,
    scale: f64,
}

fn cases(profile: G4PrefixKernelProfile) -> Vec<PrefixCase> {
    let regimes = [
        (20.0, 0.10, 5.0, 0.02),
        (200.0, 0.70, 20.0, 0.01),
        (1_000.0, 0.90, 50.0, 0.005),
    ];
    let dimensions: &[usize] = match profile {
        G4PrefixKernelProfile::Smoke => &[32],
        G4PrefixKernelProfile::Canonical => &[32, 128, 512],
    };
    dimensions
        .iter()
        .flat_map(|&dimension| {
            regimes.map(
                move |(stiffness, nonnormality, oscillation, scale)| PrefixCase {
                    dimension,
                    stiffness,
                    nonnormality,
                    oscillation,
                    scale,
                },
            )
        })
        .collect()
}

fn apply_generator(case: PrefixCase, x: &[f64], y: &mut [f64]) {
    let n = case.dimension;
    for i in 0..n {
        let fraction = (i + 1) as f64 / n as f64;
        let diagonal = -case.stiffness * (0.25 + 0.75 * fraction);
        let upper = if i + 1 < n {
            0.35 * case.nonnormality * case.stiffness * x[i + 1]
        } else {
            0.0
        };
        let oscillatory = if i % 2 == 0 && i + 1 < n {
            -case.oscillation * x[i + 1]
        } else if i % 2 == 1 {
            case.oscillation * x[i - 1]
        } else {
            0.0
        };
        y[i] = diagonal * x[i] + upper + oscillatory;
    }
}

fn generator(case: PrefixCase) -> Arc<dyn LinearOperator> {
    Arc::new(ClosureOperator::new(case.dimension, move |x, y| {
        apply_generator(case, x, y);
        Ok(())
    }))
}

fn shifted(case: PrefixCase) -> Arc<dyn LinearOperator> {
    let h_gamma = 0.19 * case.scale;
    Arc::new(ClosureOperator::new(case.dimension, move |x, y| {
        apply_generator(case, x, y);
        for i in 0..case.dimension {
            y[i] = x[i] - h_gamma * y[i];
        }
        Ok(())
    }))
}

fn fused_vectors(case: PrefixCase) -> Vec<Vec<f64>> {
    let n = case.dimension;
    vec![
        vec![0.0; n],
        (0..n)
            .map(|i| (0.13 * (i + 1) as f64).sin() + 0.25)
            .collect(),
        (0..n)
            .map(|i| 0.1 * (0.07 * (i + 1) as f64).cos())
            .collect(),
        (0..n)
            .map(|i| 0.02 * (0.11 * (i + 1) as f64).sin())
            .collect(),
        (0..n)
            .map(|i| 0.01 * (0.09 * (i + 1) as f64).cos())
            .collect(),
    ]
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(values[values.len() / 2])
}

fn quantile(mut values: Vec<f64>, probability: f64) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let index = ((values.len() - 1) as f64 * probability).ceil() as usize;
    values.get(index).copied()
}

fn vector_relative_defect(a: &[f64], b: &[f64]) -> f64 {
    let difference = a
        .iter()
        .zip(b)
        .map(|(left, right)| left - right)
        .collect::<Vec<_>>();
    safe_l2(&difference) / safe_l2(a).max(1.0e-300)
}

fn benchmark_operation<F>(
    repeats: usize,
    minimum_sample_seconds: f64,
    mut operation: F,
) -> CoreResult<f64>
where
    F: FnMut() -> CoreResult<()>,
{
    // Prefix kernels are microsecond-scale on the calibration dimensions.  A fixed
    // number of one-shot timings is dominated by clock and scheduler noise, so each
    // sample is automatically batched to a minimum wall duration.  The reported time
    // is always per operation, and the warm-up/calibration executions are excluded.
    operation()?;
    let mut batch = 1usize;
    loop {
        let start = Instant::now();
        for _ in 0..batch {
            operation()?;
        }
        if start.elapsed().as_secs_f64() >= minimum_sample_seconds || batch >= (1 << 20) {
            break;
        }
        batch = batch.saturating_mul(2);
    }
    let mut samples = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        let start = Instant::now();
        for _ in 0..batch {
            operation()?;
        }
        samples.push(start.elapsed().as_secs_f64() / batch as f64);
    }
    median(samples).ok_or_else(|| CoreError::InvalidInput("empty timing sample".into()))
}

struct ExponentialMeasurement {
    cold_wall: f64,
    prefix_wall: f64,
    resume_wall: f64,
    cold: FusedPhiActionReport,
    resumed: FusedPhiActionReport,
    prediction: FusedPhiPrefixPrediction,
    work: WorkCounters,
}

fn measure_exponential(
    case: PrefixCase,
    prefix_dimension: usize,
    repeats: usize,
) -> CoreResult<ExponentialMeasurement> {
    let vectors = fused_vectors(case);
    let config = FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: (case.dimension + 4).min(32),
        dimension_increment: 1,
        relative_tolerance: 1.0e-10,
        absolute_tolerance: 1.0e-13,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 1,
    };
    let minimum_sample_seconds = if repeats >= 7 { 0.01 } else { 0.002 };
    let cold_wall = benchmark_operation(repeats, minimum_sample_seconds, || {
        let mut counters = WorkCounters::default();
        let report = fused_phi_action_incremental(
            generator(case),
            case.scale,
            &vectors,
            config,
            &mut counters,
        )?;
        if !report.converged {
            return Err(CoreError::LinearSolve(
                "cold fused-phi incremental path did not converge".into(),
            ));
        }
        Ok(())
    })?;
    let prefix_wall = benchmark_operation(repeats, minimum_sample_seconds, || {
        let mut counters = WorkCounters::default();
        let _session = FusedPhiPrefixSession::begin(
            generator(case),
            case.scale,
            &vectors,
            config,
            prefix_dimension,
            &mut counters,
        )?;
        Ok(())
    })?;
    let total_reused_wall = benchmark_operation(repeats, minimum_sample_seconds, || {
        let mut counters = WorkCounters::default();
        let session = FusedPhiPrefixSession::begin(
            generator(case),
            case.scale,
            &vectors,
            config,
            prefix_dimension,
            &mut counters,
        )?;
        let report = session.finish(&mut counters)?;
        if !report.converged {
            return Err(CoreError::LinearSolve(
                "resumed fused-phi prefix path did not converge".into(),
            ));
        }
        Ok(())
    })?;

    let mut cold_work = WorkCounters::default();
    let cold = fused_phi_action_incremental(
        generator(case),
        case.scale,
        &vectors,
        config,
        &mut cold_work,
    )?;
    let mut reused_work = WorkCounters::default();
    let session = FusedPhiPrefixSession::begin(
        generator(case),
        case.scale,
        &vectors,
        config,
        prefix_dimension,
        &mut reused_work,
    )?;
    let prediction = session.prediction();
    let resumed = session.finish(&mut reused_work)?;
    if !cold.converged || !resumed.converged {
        return Err(CoreError::LinearSolve(
            "fused-phi timing authority did not converge".into(),
        ));
    }
    Ok(ExponentialMeasurement {
        cold_wall,
        prefix_wall,
        resume_wall: (total_reused_wall - prefix_wall).max(0.0),
        cold,
        resumed,
        prediction,
        work: reused_work,
    })
}

struct GmresMeasurement {
    cold_wall: f64,
    prefix_wall: f64,
    resume_wall: f64,
    cold_x: Vec<f64>,
    resumed_x: Vec<f64>,
    resumed_iterations: usize,
    prediction: GmresPrefixPrediction,
    work: WorkCounters,
}

fn measure_gmres(
    case: PrefixCase,
    prefix_iterations: usize,
    repeats: usize,
) -> CoreResult<GmresMeasurement> {
    let rhs = fused_vectors(case)[1].clone();
    let config = GmresConfig {
        restart: case.dimension.min(32),
        max_arnoldi: case.dimension.min(32),
        rtol: 1.0e-10,
        atol: 1.0e-13,
    };
    let pc = IdentityPreconditioner::new(case.dimension);
    let minimum_sample_seconds = if repeats >= 7 { 0.01 } else { 0.002 };
    let cold_wall = benchmark_operation(repeats, minimum_sample_seconds, || {
        let operator = shifted(case);
        let mut counters = WorkCounters::default();
        let _ =
            solve_gmres_incremental(operator.as_ref(), &pc, &rhs, None, &config, &mut counters)?;
        Ok(())
    })?;
    let prefix_wall = benchmark_operation(repeats, minimum_sample_seconds, || {
        let operator = shifted(case);
        let mut counters = WorkCounters::default();
        let _session = GmresPrefixSession::begin(
            operator.as_ref(),
            &pc,
            &rhs,
            None,
            &config,
            prefix_iterations,
            &mut counters,
        )?;
        Ok(())
    })?;
    let total_reused_wall = benchmark_operation(repeats, minimum_sample_seconds, || {
        let operator = shifted(case);
        let mut counters = WorkCounters::default();
        let session = GmresPrefixSession::begin(
            operator.as_ref(),
            &pc,
            &rhs,
            None,
            &config,
            prefix_iterations,
            &mut counters,
        )?;
        let _ = session.finish(operator.as_ref(), &pc, &mut counters)?;
        Ok(())
    })?;

    let operator = shifted(case);
    let mut cold_work = WorkCounters::default();
    let cold =
        solve_gmres_incremental(operator.as_ref(), &pc, &rhs, None, &config, &mut cold_work)?;
    let operator = shifted(case);
    let mut reused_work = WorkCounters::default();
    let session = GmresPrefixSession::begin(
        operator.as_ref(),
        &pc,
        &rhs,
        None,
        &config,
        prefix_iterations,
        &mut reused_work,
    )?;
    let prediction = session.prediction();
    let resumed = session.finish(operator.as_ref(), &pc, &mut reused_work)?;
    Ok(GmresMeasurement {
        cold_wall,
        prefix_wall,
        resume_wall: (total_reused_wall - prefix_wall).max(0.0),
        cold_x: cold.x,
        resumed_x: resumed.x,
        resumed_iterations: resumed.iterations as usize,
        prediction,
        work: reused_work,
    })
}

fn failure_row(case: PrefixCase, prefix_dimension: usize, failure: String) -> G4PrefixKernelRow {
    G4PrefixKernelRow {
        case_id: format!(
            "n{}-s{}-eta{:.2}-omega{}-h{}",
            case.dimension, case.stiffness, case.nonnormality, case.oscillation, case.scale
        ),
        dimension: case.dimension,
        stiffness: case.stiffness,
        nonnormality: case.nonnormality,
        oscillation: case.oscillation,
        scale: case.scale,
        prefix_dimension,
        completed: false,
        failure: Some(failure),
        exponential_cold_wall_seconds: None,
        exponential_prefix_wall_seconds: None,
        exponential_resume_wall_seconds: None,
        exponential_total_reused_wall_seconds: None,
        exponential_actual_dimension: None,
        exponential_predicted_dimension: None,
        exponential_prefix_cost_inflation: None,
        exponential_value_relative_defect: None,
        exponential_jvp_vectors: None,
        gmres_cold_wall_seconds: None,
        gmres_prefix_wall_seconds: None,
        gmres_resume_wall_seconds: None,
        gmres_total_reused_wall_seconds: None,
        gmres_actual_iterations: None,
        gmres_predicted_iterations: None,
        gmres_prefix_cost_inflation: None,
        gmres_solution_relative_defect: None,
        gmres_operator_vectors: None,
        actual_exponential_to_gmres_cost_ratio: None,
        predicted_exponential_to_gmres_cost_ratio: None,
        cost_ratio_relative_prediction_error: None,
    }
}

fn run_row(case: PrefixCase, prefix_dimension: usize, repeats: usize) -> G4PrefixKernelRow {
    let result = (|| -> CoreResult<G4PrefixKernelRow> {
        let exponential = measure_exponential(case, prefix_dimension, repeats)?;
        let gmres = measure_gmres(case, prefix_dimension, repeats)?;
        let exponential_total = exponential.prefix_wall + exponential.resume_wall;
        let gmres_total = gmres.prefix_wall + gmres.resume_wall;
        let exponential_dimension = exponential.resumed.maximum_krylov_dimension;
        let gmres_iterations = gmres.resumed_iterations;
        let exponential_prediction = exponential.prediction.predicted_total_dimension;
        let gmres_prediction = gmres.prediction.predicted_total_iterations;
        let exponential_predicted_wall = exponential.prefix_wall
            * exponential_prediction.max(1) as f64
            / prefix_dimension.max(1) as f64;
        let gmres_predicted_wall =
            gmres.prefix_wall * gmres_prediction.max(1) as f64 / prefix_dimension.max(1) as f64;
        let actual_ratio = exponential.cold_wall / gmres.cold_wall.max(f64::MIN_POSITIVE);
        let predicted_ratio =
            exponential_predicted_wall / gmres_predicted_wall.max(f64::MIN_POSITIVE);
        let prediction_error = (predicted_ratio / actual_ratio.max(f64::MIN_POSITIVE) - 1.0).abs();
        let exponential_inflation = (exponential.prefix_wall
            / exponential.cold_wall.max(f64::MIN_POSITIVE))
            / (prefix_dimension as f64 / exponential_dimension.max(1) as f64);
        let gmres_inflation = (gmres.prefix_wall / gmres.cold_wall.max(f64::MIN_POSITIVE))
            / (prefix_dimension as f64 / gmres_iterations.max(1) as f64);
        Ok(G4PrefixKernelRow {
            case_id: format!(
                "n{}-s{}-eta{:.2}-omega{}-h{}",
                case.dimension, case.stiffness, case.nonnormality, case.oscillation, case.scale
            ),
            dimension: case.dimension,
            stiffness: case.stiffness,
            nonnormality: case.nonnormality,
            oscillation: case.oscillation,
            scale: case.scale,
            prefix_dimension,
            completed: true,
            failure: None,
            exponential_cold_wall_seconds: Some(exponential.cold_wall),
            exponential_prefix_wall_seconds: Some(exponential.prefix_wall),
            exponential_resume_wall_seconds: Some(exponential.resume_wall),
            exponential_total_reused_wall_seconds: Some(exponential_total),
            exponential_actual_dimension: Some(exponential_dimension),
            exponential_predicted_dimension: Some(exponential_prediction),
            exponential_prefix_cost_inflation: Some(exponential_inflation),
            exponential_value_relative_defect: Some(vector_relative_defect(
                &exponential.cold.value,
                &exponential.resumed.value,
            )),
            exponential_jvp_vectors: Some(exponential.work.jvp_vectors),
            gmres_cold_wall_seconds: Some(gmres.cold_wall),
            gmres_prefix_wall_seconds: Some(gmres.prefix_wall),
            gmres_resume_wall_seconds: Some(gmres.resume_wall),
            gmres_total_reused_wall_seconds: Some(gmres_total),
            gmres_actual_iterations: Some(gmres_iterations),
            gmres_predicted_iterations: Some(gmres_prediction),
            gmres_prefix_cost_inflation: Some(gmres_inflation),
            gmres_solution_relative_defect: Some(vector_relative_defect(
                &gmres.cold_x,
                &gmres.resumed_x,
            )),
            gmres_operator_vectors: Some(gmres.work.linear_matvecs),
            actual_exponential_to_gmres_cost_ratio: Some(actual_ratio),
            predicted_exponential_to_gmres_cost_ratio: Some(predicted_ratio),
            cost_ratio_relative_prediction_error: Some(prediction_error),
        })
    })();
    result.unwrap_or_else(|error| failure_row(case, prefix_dimension, error.to_string()))
}

pub fn run_g4_prefix_kernel_gate(
    profile: G4PrefixKernelProfile,
) -> CoreResult<G4PrefixKernelReport> {
    let repeats = match profile {
        G4PrefixKernelProfile::Smoke => 3,
        G4PrefixKernelProfile::Canonical => 7,
    };
    let mut rows = Vec::new();
    for case in cases(profile) {
        for prefix_dimension in [1_usize, 2_usize] {
            rows.push(run_row(case, prefix_dimension, repeats));
        }
    }
    let completed_rows = rows.iter().filter(|row| row.completed).collect::<Vec<_>>();
    let prediction_errors = completed_rows
        .iter()
        .filter_map(|row| row.cost_ratio_relative_prediction_error)
        .collect::<Vec<_>>();
    let exponential_inflations = completed_rows
        .iter()
        .filter_map(|row| row.exponential_prefix_cost_inflation)
        .collect::<Vec<_>>();
    let gmres_inflations = completed_rows
        .iter()
        .filter_map(|row| row.gmres_prefix_cost_inflation)
        .collect::<Vec<_>>();
    let maximum_exponential_defect = completed_rows
        .iter()
        .filter_map(|row| row.exponential_value_relative_defect)
        .reduce(f64::max);
    let maximum_gmres_defect = completed_rows
        .iter()
        .filter_map(|row| row.gmres_solution_relative_defect)
        .reduce(f64::max);
    let p95_prediction = quantile(prediction_errors.clone(), 0.95);
    let p95_exponential_inflation = quantile(exponential_inflations.clone(), 0.95);
    let p95_gmres_inflation = quantile(gmres_inflations.clone(), 0.95);
    // G4-S4 calibration selected a two-vector prefix before the larger-dimension holdout.
    // The one-vector rows remain an adversarial negative control rather than being hidden.
    let selected_prefix_dimension = 2usize;
    let selected_rows = completed_rows
        .iter()
        .copied()
        .filter(|row| row.prefix_dimension == selected_prefix_dimension)
        .collect::<Vec<_>>();
    let selected_prediction_errors = selected_rows
        .iter()
        .filter_map(|row| row.cost_ratio_relative_prediction_error)
        .collect::<Vec<_>>();
    let selected_exponential_inflations = selected_rows
        .iter()
        .filter_map(|row| row.exponential_prefix_cost_inflation)
        .collect::<Vec<_>>();
    let selected_gmres_inflations = selected_rows
        .iter()
        .filter_map(|row| row.gmres_prefix_cost_inflation)
        .collect::<Vec<_>>();
    let selected_p95_prediction = quantile(selected_prediction_errors, 0.95);
    let selected_p95_exponential_inflation = quantile(selected_exponential_inflations, 0.95);
    let selected_p95_gmres_inflation = quantile(selected_gmres_inflations, 0.95);
    let reuse_parity = maximum_exponential_defect.is_some_and(|value| value <= 1.0e-12)
        && maximum_gmres_defect.is_some_and(|value| value <= 1.0e-12);
    let summary = G4PrefixKernelSummary {
        rows: rows.len(),
        completed: completed_rows.len(),
        maximum_exponential_value_relative_defect: maximum_exponential_defect,
        maximum_gmres_solution_relative_defect: maximum_gmres_defect,
        median_cost_ratio_prediction_error: median(prediction_errors),
        p95_cost_ratio_prediction_error: p95_prediction,
        median_exponential_prefix_inflation: median(exponential_inflations),
        p95_exponential_prefix_inflation: p95_exponential_inflation,
        median_gmres_prefix_inflation: median(gmres_inflations),
        p95_gmres_prefix_inflation: p95_gmres_inflation,
        all_prefix_prediction_gate_pass: p95_prediction.is_some_and(|value| value <= 0.19),
        all_prefix_cost_gate_pass: p95_exponential_inflation.is_some_and(|value| value <= 1.9)
            && p95_gmres_inflation.is_some_and(|value| value <= 1.9),
        selected_prefix_dimension,
        selected_rows: selected_rows.len(),
        selected_p95_cost_ratio_prediction_error: selected_p95_prediction,
        selected_p95_exponential_prefix_inflation: selected_p95_exponential_inflation,
        selected_p95_gmres_prefix_inflation: selected_p95_gmres_inflation,
        selected_prediction_gate_pass: selected_p95_prediction.is_some_and(|value| value <= 0.19),
        selected_prefix_cost_gate_pass: selected_p95_exponential_inflation
            .is_some_and(|value| value <= 1.9)
            && selected_p95_gmres_inflation.is_some_and(|value| value <= 1.9),
        reuse_parity_gate_pass: reuse_parity,
        active_switching_authorized: false,
    };
    Ok(G4PrefixKernelReport {
        schema: "generic-prefix-kernel-g4-s4-v1",
        status: "read-only-prefix-kernel-gate",
        profile: profile.as_str(),
        prefix_kernel: "actual-rust-arnoldi-and-gmres-prefix-reuse",
        rows,
        summary,
        limitations: vec![
            "The gate uses structured matrix-free local operators, not full adaptive trajectories."
                .into(),
            "The wall predictor extrapolates short-prefix cost and residual decay; delayed nonnormal convergence remains possible."
                .into(),
            "The fused-phi prefix kernel is full-MGS, one-substep only; IOP/restart policy is unchanged."
                .into(),
            "Active method switching remains forbidden regardless of this gate result.".into(),
        ],
    })
}
