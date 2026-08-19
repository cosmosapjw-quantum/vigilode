use std::{
    hint::black_box,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use rodas5p_core::{
    CoreError, CoreResult, LinearOperator, Preconditioner, WorkCounters, load_rodas5p_coefficients,
    safe_l2, sha256_hex,
};
use rodas5p_krylov::{
    BlockGmresConfig, GmresConfig, SeededGmresConfig, solve_block_gmres, solve_gmres,
    solve_seeded_gmres,
};
use serde::Serialize;

use crate::ParallelExecution;

const RHS_COUNT: usize = 8;
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0xC0_0000);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatrixFreeCommonWProfile {
    Smoke,
    Canonical,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MatrixFreeCommonWCase {
    pub case_id: String,
    pub dimension: usize,
    pub rhs_count: usize,
    pub rhs_rank: usize,
    pub nonnormality: f64,
    pub stiffness: f64,
    pub threads: usize,
    pub h: f64,
    pub gamma: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MatrixFreeCommonWRow {
    pub case_id: String,
    pub solver: String,
    pub threads: usize,
    pub success: bool,
    pub failure: Option<String>,
    pub median_seconds: Option<f64>,
    pub timing_repetitions: usize,
    pub timing_batch_iterations: usize,
    pub speedup_vs_serial: Option<f64>,
    pub maximum_residual_norm: Option<f64>,
    pub maximum_relative_residual: Option<f64>,
    pub maximum_solution_difference_vs_serial: f64,
    pub operator_vectors: u64,
    pub krylov_operator_vectors: u64,
    pub diagnostic_operator_vectors: u64,
    pub jvp_vectors: u64,
    pub mass_vectors: u64,
    pub block_operator_calls: u64,
    pub preconditioner_vectors: u64,
    pub block_preconditioner_calls: u64,
    pub linear_iterations: u64,
    pub orthogonalization_inner_products: u64,
    pub orthogonalization_vector_updates: u64,
    pub initial_block_rank: Option<usize>,
    pub final_basis_dimension: Option<usize>,
    pub estimated_krylov_bytes: u64,
    pub explicit_jacobian_builds: u64,
    pub factorization_builds: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MatrixFreeCommonWReport {
    pub schema: &'static str,
    pub profile: MatrixFreeCommonWProfile,
    pub cases: Vec<MatrixFreeCommonWCase>,
    pub rows: Vec<MatrixFreeCommonWRow>,
    pub successful_rows: usize,
    pub failed_rows: usize,
    pub strict_jacobian_free: bool,
    pub explicit_jacobian_builds: u64,
    pub factorization_builds: u64,
    pub block_gmres_successes: usize,
    pub seeded_shared_successes: usize,
    pub block_rows_above_1_15x: usize,
    pub seeded_rows_above_1_15x: usize,
    pub scientific_checksum: String,
    pub verdict: String,
}

#[derive(Clone)]
struct CommonWSpec {
    descriptor: MatrixFreeCommonWCase,
    rhs_rows: Vec<Vec<f64>>,
}

struct MatrixFreeCommonWOperator {
    dimension: usize,
    stiffness: f64,
    nonnormality: f64,
    omega: f64,
    h_gamma: f64,
    mass_diagonal: Vec<f64>,
    execution: Arc<ParallelExecution>,
    vector_applications: AtomicU64,
    block_calls: AtomicU64,
    token: u64,
}

impl MatrixFreeCommonWOperator {
    fn new(spec: &MatrixFreeCommonWCase, execution: Arc<ParallelExecution>) -> Self {
        let mass_diagonal = (0..spec.dimension)
            .map(|index| 1.0 + 0.08 * (0.017 * (index + 1) as f64).sin())
            .collect();
        Self {
            dimension: spec.dimension,
            stiffness: spec.stiffness,
            nonnormality: spec.nonnormality,
            omega: 0.35 * spec.stiffness,
            h_gamma: spec.h * spec.gamma,
            mass_diagonal,
            execution,
            vector_applications: AtomicU64::new(0),
            block_calls: AtomicU64::new(0),
            token: NEXT_TOKEN.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn apply_formula(&self, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        if input.len() != self.dimension || output.len() != self.dimension {
            return Err(CoreError::Dimension(
                "matrix-free common-W vector shape mismatch".into(),
            ));
        }
        for pair in 0..self.dimension / 2 {
            let i = 2 * pair;
            let scale = 1.0 + 0.03 * ((pair % 11) as f64);
            let next_real = if i + 2 < self.dimension {
                input[i + 2]
            } else {
                0.0
            };
            let next_imag = if i + 3 < self.dimension {
                input[i + 3]
            } else {
                0.0
            };
            let j_real = -self.stiffness * scale * input[i] - self.omega * input[i + 1]
                + self.nonnormality * self.stiffness * next_real;
            let j_imag = self.omega * input[i] - self.stiffness * scale * input[i + 1]
                + self.nonnormality * self.stiffness * next_imag;
            output[i] = self.mass_diagonal[i] * input[i] - self.h_gamma * j_real;
            output[i + 1] = self.mass_diagonal[i + 1] * input[i + 1] - self.h_gamma * j_imag;
        }
        if output.iter().all(|value| value.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::NonFinite(
                "matrix-free common-W action produced NaN/Inf".into(),
            ))
        }
    }

    fn vector_applications(&self) -> u64 {
        self.vector_applications.load(Ordering::Relaxed)
    }

    fn block_calls(&self) -> u64 {
        self.block_calls.load(Ordering::Relaxed)
    }
}

impl LinearOperator for MatrixFreeCommonWOperator {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        self.vector_applications.fetch_add(1, Ordering::Relaxed);
        self.apply_formula(input, output)
    }

    fn apply_rows(&self, inputs: &[Vec<f64>], outputs: &mut [Vec<f64>]) -> CoreResult<()> {
        if inputs.len() != outputs.len()
            || inputs.iter().any(|row| row.len() != self.dimension)
            || outputs.iter().any(|row| row.len() != self.dimension)
        {
            return Err(CoreError::Dimension(
                "matrix-free common-W row batch shape mismatch".into(),
            ));
        }
        self.vector_applications
            .fetch_add(inputs.len() as u64, Ordering::Relaxed);
        self.block_calls.fetch_add(1, Ordering::Relaxed);
        self.execution
            .try_for_each_ordered_mut(inputs, outputs, |input, output| {
                self.apply_formula(input, output)
            })
    }

    fn token(&self) -> u64 {
        self.token
    }
}

struct AnalyticDiagonalPreconditioner {
    inverse: Vec<f64>,
}

impl AnalyticDiagonalPreconditioner {
    fn new(operator: &MatrixFreeCommonWOperator) -> CoreResult<Self> {
        let inverse = operator
            .mass_diagonal
            .iter()
            .enumerate()
            .map(|(index, mass)| {
                let scale = 1.0 + 0.03 * (((index / 2) % 11) as f64);
                let diagonal = mass + operator.h_gamma * operator.stiffness * scale;
                if diagonal.abs() <= f64::MIN_POSITIVE || !diagonal.is_finite() {
                    Err(CoreError::LinearSolve(
                        "matrix-free analytic diagonal preconditioner is singular".into(),
                    ))
                } else {
                    Ok(1.0 / diagonal)
                }
            })
            .collect::<CoreResult<Vec<_>>>()?;
        Ok(Self { inverse })
    }
}

impl Preconditioner for AnalyticDiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.inverse.len()
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        if input.len() != self.inverse.len() || output.len() != self.inverse.len() {
            return Err(CoreError::Dimension(
                "matrix-free preconditioner shape mismatch".into(),
            ));
        }
        for ((target, &value), &inverse) in output.iter_mut().zip(input).zip(&self.inverse) {
            *target = inverse * value;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct SolverOutcome {
    solutions: Option<Vec<Vec<f64>>>,
    failure: Option<String>,
    counters: WorkCounters,
    maximum_residual_norm: Option<f64>,
    maximum_relative_residual: Option<f64>,
    initial_block_rank: Option<usize>,
    final_basis_dimension: Option<usize>,
    operator_vectors: u64,
    block_operator_calls: u64,
}

impl SolverOutcome {
    fn success(&self) -> bool {
        self.failure.is_none() && self.solutions.is_some()
    }
}

fn add_counters(target: &mut WorkCounters, source: WorkCounters) {
    macro_rules! add_fields {
        ($($field:ident),* $(,)?) => {
            $(
                target.$field = target.$field.saturating_add(source.$field);
            )*
        };
    }
    add_fields!(
        rhs_calls,
        rhs_batch_calls,
        rhs_evaluations,
        ft_calls,
        jacobian_builds,
        jvp_calls,
        jvp_vectors,
        mass_matvecs,
        nonlinear_solves,
        nonlinear_iterations,
        nonlinear_residual_evaluations,
        nonlinear_jacobian_evaluations,
        nonlinear_failures,
        linear_solves,
        linear_iterations,
        linear_matvecs,
        preconditioner_apps,
        direct_factorizations,
        direct_solve_calls,
        recycle_projection_calls,
        recycle_same_operator_uses,
        recycle_cross_operator_refreshes,
        recycle_refresh_matvecs,
        recycle_updates,
        recycle_vectors_selected,
        recycle_dropped_vectors,
        harmonic_ritz_solves,
        orthogonalization_inner_products,
        orthogonalization_vector_updates,
        diagnostic_matvecs,
        block_linear_solves,
        block_linear_iterations,
        block_matvecs,
        block_preconditioner_apps,
        fast_attempts,
        fast_accepts,
        fallback_steps,
        accepted_steps,
        rejected_steps,
    );
}

fn independent_solve(
    spec: &CommonWSpec,
    execution: Arc<ParallelExecution>,
    rayon_across_rhs: bool,
) -> CoreResult<SolverOutcome> {
    let operator_execution = Arc::new(ParallelExecution::sequential());
    let operator = Arc::new(MatrixFreeCommonWOperator::new(
        &spec.descriptor,
        operator_execution,
    ));
    let preconditioner = Arc::new(AnalyticDiagonalPreconditioner::new(&operator)?);
    let config = GmresConfig {
        restart: 32.min(spec.descriptor.dimension.max(1)),
        max_arnoldi: 128.min(spec.descriptor.dimension.max(1)),
        rtol: 1.0e-9,
        atol: 1.0e-12,
    };
    let serial_execution = ParallelExecution::sequential();
    let active_execution = if rayon_across_rhs {
        execution.as_ref()
    } else {
        &serial_execution
    };
    let indices = (0..spec.rhs_rows.len()).collect::<Vec<_>>();
    let solved = active_execution.map_ordered(&indices, |&index| {
        let mut counters = WorkCounters::default();
        let result = solve_gmres(
            operator.as_ref(),
            preconditioner.as_ref(),
            &spec.rhs_rows[index],
            None,
            &config,
            &mut counters,
        );
        Ok((result, counters))
    })?;
    let mut counters = WorkCounters::default();
    let mut solutions = Vec::with_capacity(solved.len());
    let mut maximum_residual_norm = 0.0_f64;
    let mut maximum_relative_residual = 0.0_f64;
    let mut failure = None;
    for (result, local) in solved {
        add_counters(&mut counters, local);
        match result {
            Ok(report) => {
                maximum_residual_norm = maximum_residual_norm.max(report.residual_norm);
                maximum_relative_residual = maximum_relative_residual.max(report.relative_residual);
                solutions.push(report.x);
            }
            Err(error) => {
                if failure.is_none() {
                    failure = Some(error.to_string());
                }
            }
        }
    }
    let success = failure.is_none() && solutions.len() == spec.rhs_rows.len();
    Ok(SolverOutcome {
        solutions: success.then_some(solutions),
        failure,
        counters,
        maximum_residual_norm: success.then_some(maximum_residual_norm),
        maximum_relative_residual: success.then_some(maximum_relative_residual),
        initial_block_rank: None,
        final_basis_dimension: None,
        operator_vectors: operator.vector_applications(),
        block_operator_calls: operator.block_calls(),
    })
}

fn block_solve(
    spec: &CommonWSpec,
    execution: Arc<ParallelExecution>,
    seeded: bool,
) -> CoreResult<SolverOutcome> {
    let operator = Arc::new(MatrixFreeCommonWOperator::new(&spec.descriptor, execution));
    let preconditioner = AnalyticDiagonalPreconditioner::new(&operator)?;
    let mut counters = WorkCounters::default();
    let result = if seeded {
        solve_seeded_gmres(
            operator.as_ref(),
            &preconditioner,
            &spec.rhs_rows,
            &SeededGmresConfig {
                shared_basis: 12.min(spec.descriptor.dimension),
                restart: 32.min(spec.descriptor.dimension.max(1)),
                max_arnoldi: 128.min(spec.descriptor.dimension.max(1)),
                rtol: 1.0e-9,
                atol: 1.0e-12,
                rank_tolerance: 1.0e-12,
            },
            &mut counters,
        )
    } else {
        solve_block_gmres(
            operator.as_ref(),
            &preconditioner,
            &spec.rhs_rows,
            &BlockGmresConfig {
                max_basis: 128.min(spec.descriptor.dimension.max(1)),
                rtol: 1.0e-9,
                atol: 1.0e-12,
                rank_tolerance: 1.0e-12,
            },
            &mut counters,
        )
    };
    match result {
        Ok(report) => Ok(SolverOutcome {
            solutions: Some(report.solutions),
            failure: None,
            counters,
            maximum_residual_norm: Some(report.maximum_residual_norm),
            maximum_relative_residual: Some(report.maximum_relative_residual),
            initial_block_rank: Some(report.initial_block_rank),
            final_basis_dimension: Some(report.final_basis_dimension),
            operator_vectors: operator.vector_applications(),
            block_operator_calls: operator.block_calls(),
        }),
        Err(error) => Ok(SolverOutcome {
            solutions: None,
            failure: Some(error.to_string()),
            counters,
            maximum_residual_norm: None,
            maximum_relative_residual: None,
            initial_block_rank: None,
            final_basis_dimension: None,
            operator_vectors: operator.vector_applications(),
            block_operator_calls: operator.block_calls(),
        }),
    }
}

fn execute_solver(
    spec: &CommonWSpec,
    solver: &str,
    execution: Arc<ParallelExecution>,
) -> CoreResult<SolverOutcome> {
    match solver {
        "independent-gmres-serial" => independent_solve(spec, execution, false),
        "independent-gmres-rayon" => independent_solve(spec, execution, true),
        "block-gmres" => block_solve(spec, execution, false),
        "seeded-shared-gmres" => block_solve(spec, execution, true),
        _ => Err(CoreError::InvalidInput(format!(
            "unknown common-W solver {solver}"
        ))),
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    if values.len() % 2 == 1 {
        values[values.len() / 2]
    } else {
        let middle = values.len() / 2;
        0.5 * (values[middle - 1] + values[middle])
    }
}

fn timed_solver(
    spec: &CommonWSpec,
    solver: &str,
    execution: Arc<ParallelExecution>,
    repetitions: usize,
) -> CoreResult<(f64, usize)> {
    const TARGET_SAMPLE_SECONDS: f64 = 5.0e-3;
    const MAX_BATCH_ITERATIONS: usize = 4096;

    black_box(execute_solver(spec, solver, execution.clone())?.maximum_relative_residual);
    let mut batch_iterations = 1_usize;
    loop {
        let start = Instant::now();
        for _ in 0..batch_iterations {
            black_box(execute_solver(spec, solver, execution.clone())?.maximum_relative_residual);
        }
        let elapsed = start.elapsed().as_secs_f64();
        if elapsed >= TARGET_SAMPLE_SECONDS || batch_iterations >= MAX_BATCH_ITERATIONS {
            break;
        }
        let multiplier = (TARGET_SAMPLE_SECONDS / elapsed.max(1.0e-9))
            .ceil()
            .clamp(2.0, 16.0) as usize;
        batch_iterations = batch_iterations
            .saturating_mul(multiplier)
            .min(MAX_BATCH_ITERATIONS);
    }

    let mut samples = Vec::with_capacity(repetitions);
    for _ in 0..repetitions {
        let start = Instant::now();
        for _ in 0..batch_iterations {
            black_box(execute_solver(spec, solver, execution.clone())?.maximum_relative_residual);
        }
        samples.push(start.elapsed().as_secs_f64() / batch_iterations as f64);
    }
    Ok((median(&mut samples), batch_iterations))
}

fn max_solution_difference(reference: &[Vec<f64>], candidate: &[Vec<f64>]) -> CoreResult<f64> {
    if reference.len() != candidate.len()
        || reference
            .iter()
            .zip(candidate)
            .any(|(left, right)| left.len() != right.len())
    {
        return Err(CoreError::Dimension(
            "common-W solution comparison shape mismatch".into(),
        ));
    }
    Ok(reference
        .iter()
        .zip(candidate)
        .flat_map(|(left, right)| left.iter().zip(right))
        .map(|(left, right)| (left - right).abs())
        .fold(0.0_f64, f64::max))
}

fn orthonormal_rank_basis(dimension: usize, rank: usize) -> CoreResult<Vec<Vec<f64>>> {
    let mut basis = Vec::<Vec<f64>>::new();
    for mode in 0..rank {
        let mut vector = (0..dimension)
            .map(|index| {
                let x = (index + 1) as f64;
                ((mode + 1) as f64 * 0.017 * x).sin() + (0.011 * (mode + 2) as f64 * x).cos()
            })
            .collect::<Vec<_>>();
        for direction in &basis {
            let coefficient = direction
                .iter()
                .zip(&vector)
                .map(|(a, b)| a * b)
                .sum::<f64>();
            for (value, &component) in vector.iter_mut().zip(direction) {
                *value -= coefficient * component;
            }
        }
        let norm = safe_l2(&vector);
        if norm <= 1.0e-12 {
            return Err(CoreError::LinearSolve(
                "deterministic RHS rank basis collapsed".into(),
            ));
        }
        for value in &mut vector {
            *value /= norm;
        }
        basis.push(vector);
    }
    Ok(basis)
}

fn rhs_rows(dimension: usize, rank: usize) -> CoreResult<Vec<Vec<f64>>> {
    let basis = orthonormal_rank_basis(dimension, rank)?;
    let mut rows = vec![vec![0.0; dimension]; RHS_COUNT];
    for (stage, row) in rows.iter_mut().enumerate() {
        for (mode, direction) in basis.iter().enumerate() {
            let coefficient = if stage < rank && stage == mode {
                1.0
            } else {
                0.35 * ((stage + 1) as f64 * (mode + 2) as f64).cos()
                    + 0.15 * ((stage + mode + 1) as f64).sin()
            };
            for (value, &component) in row.iter_mut().zip(direction) {
                *value += coefficient * component;
            }
        }
    }
    Ok(rows)
}

fn build_specs(profile: MatrixFreeCommonWProfile) -> CoreResult<Vec<CommonWSpec>> {
    let gamma = load_rodas5p_coefficients()?.gamma;
    let (dimensions, ranks, nonnormalities, stiffnesses, threads, h) = match profile {
        MatrixFreeCommonWProfile::Smoke => (
            vec![24],
            vec![1, 4],
            vec![0.0, 0.9],
            vec![1.0e2],
            vec![1, 2],
            1.0e-3,
        ),
        MatrixFreeCommonWProfile::Canonical => (
            vec![32, 128, 512, 2048],
            vec![1, 2, 4, 8],
            vec![0.0, 0.2, 0.9],
            vec![1.0e2, 1.0e4, 1.0e6],
            vec![1, 2, 4],
            1.0e-3,
        ),
    };
    let mut specs = Vec::new();
    for dimension in dimensions {
        for &rhs_rank in &ranks {
            for &nonnormality in &nonnormalities {
                for &stiffness in &stiffnesses {
                    for &thread_count in &threads {
                        let descriptor = MatrixFreeCommonWCase {
                            case_id: format!(
                                "n{dimension}-rank{rhs_rank}-eta{nonnormality:.1}-s{stiffness:.0e}-t{thread_count}"
                            ),
                            dimension,
                            rhs_count: RHS_COUNT,
                            rhs_rank,
                            nonnormality,
                            stiffness,
                            threads: thread_count,
                            h,
                            gamma,
                        };
                        specs.push(CommonWSpec {
                            rhs_rows: rhs_rows(dimension, rhs_rank)?,
                            descriptor,
                        });
                    }
                }
            }
        }
    }
    Ok(specs)
}

fn scientific_checksum(cases: &[MatrixFreeCommonWCase], rows: &[MatrixFreeCommonWRow]) -> String {
    let mut signature = String::new();
    for case in cases {
        signature.push_str(&format!(
            "case|{}|{}|{}|{:016x}|{:016x}|{}|{:016x}|{:016x}\n",
            case.case_id,
            case.dimension,
            case.rhs_rank,
            case.nonnormality.to_bits(),
            case.stiffness.to_bits(),
            case.threads,
            case.h.to_bits(),
            case.gamma.to_bits(),
        ));
    }
    for row in rows {
        signature.push_str(&format!(
            "row|{}|{}|{}|{}|{:016x}|{:016x}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
            row.case_id,
            row.solver,
            row.threads,
            row.success,
            row.maximum_relative_residual
                .unwrap_or(f64::INFINITY)
                .to_bits(),
            row.maximum_solution_difference_vs_serial.to_bits(),
            row.operator_vectors,
            row.krylov_operator_vectors,
            row.diagnostic_operator_vectors,
            row.block_operator_calls,
            row.preconditioner_vectors,
            row.block_preconditioner_calls,
            row.linear_iterations,
            row.explicit_jacobian_builds,
            row.factorization_builds,
        ));
    }
    sha256_hex(signature.as_bytes())
}

pub fn run_matrix_free_common_w_gate(
    profile: MatrixFreeCommonWProfile,
) -> CoreResult<MatrixFreeCommonWReport> {
    let specs = build_specs(profile)?;
    let repetitions = match profile {
        MatrixFreeCommonWProfile::Smoke => 3,
        MatrixFreeCommonWProfile::Canonical => 5,
    };
    let solvers = [
        "independent-gmres-serial",
        "independent-gmres-rayon",
        "block-gmres",
        "seeded-shared-gmres",
    ];
    let mut rows = Vec::new();
    for spec in &specs {
        let execution = Arc::new(ParallelExecution::rayon(spec.descriptor.threads)?);
        let baseline = execute_solver(spec, "independent-gmres-serial", execution.clone())?;
        let (baseline_time, baseline_batch_iterations) = timed_solver(
            spec,
            "independent-gmres-serial",
            execution.clone(),
            repetitions,
        )?;
        for solver in solvers {
            let outcome = execute_solver(spec, solver, execution.clone())?;
            if outcome.success() {
                let (measured, timing_batch_iterations) = if solver == "independent-gmres-serial" {
                    (baseline_time, baseline_batch_iterations)
                } else {
                    timed_solver(spec, solver, execution.clone(), repetitions)?
                };
                let difference = max_solution_difference(
                    baseline
                        .solutions
                        .as_deref()
                        .expect("checked baseline success"),
                    outcome
                        .solutions
                        .as_deref()
                        .expect("checked candidate success"),
                )?;
                let basis_vectors = outcome
                    .final_basis_dimension
                    .unwrap_or_else(|| 32.min(spec.descriptor.dimension));
                rows.push(MatrixFreeCommonWRow {
                    case_id: spec.descriptor.case_id.clone(),
                    solver: solver.into(),
                    threads: if solver == "independent-gmres-serial" {
                        1
                    } else {
                        spec.descriptor.threads
                    },
                    success: true,
                    failure: None,
                    median_seconds: Some(measured),
                    timing_repetitions: repetitions,
                    timing_batch_iterations,
                    speedup_vs_serial: Some(baseline_time / measured),
                    maximum_residual_norm: outcome.maximum_residual_norm,
                    maximum_relative_residual: outcome.maximum_relative_residual,
                    maximum_solution_difference_vs_serial: difference,
                    operator_vectors: outcome.operator_vectors,
                    krylov_operator_vectors: outcome.counters.linear_matvecs,
                    diagnostic_operator_vectors: outcome.counters.diagnostic_matvecs,
                    jvp_vectors: outcome.operator_vectors,
                    mass_vectors: outcome.operator_vectors,
                    block_operator_calls: outcome.block_operator_calls,
                    preconditioner_vectors: outcome.counters.preconditioner_apps,
                    block_preconditioner_calls: outcome.counters.block_preconditioner_apps,
                    linear_iterations: outcome.counters.linear_iterations,
                    orthogonalization_inner_products: outcome
                        .counters
                        .orthogonalization_inner_products,
                    orthogonalization_vector_updates: outcome
                        .counters
                        .orthogonalization_vector_updates,
                    initial_block_rank: outcome.initial_block_rank,
                    final_basis_dimension: outcome.final_basis_dimension,
                    estimated_krylov_bytes: (basis_vectors
                        * spec.descriptor.dimension
                        * std::mem::size_of::<f64>())
                        as u64,
                    explicit_jacobian_builds: outcome.counters.jacobian_builds,
                    factorization_builds: outcome.counters.direct_factorizations,
                });
            } else {
                rows.push(MatrixFreeCommonWRow {
                    case_id: spec.descriptor.case_id.clone(),
                    solver: solver.into(),
                    threads: if solver == "independent-gmres-serial" {
                        1
                    } else {
                        spec.descriptor.threads
                    },
                    success: false,
                    failure: outcome.failure,
                    median_seconds: None,
                    timing_repetitions: repetitions,
                    timing_batch_iterations: 0,
                    speedup_vs_serial: None,
                    maximum_residual_norm: outcome.maximum_residual_norm,
                    maximum_relative_residual: outcome.maximum_relative_residual,
                    maximum_solution_difference_vs_serial: 0.0,
                    operator_vectors: outcome.operator_vectors,
                    krylov_operator_vectors: outcome.counters.linear_matvecs,
                    diagnostic_operator_vectors: outcome.counters.diagnostic_matvecs,
                    jvp_vectors: outcome.operator_vectors,
                    mass_vectors: outcome.operator_vectors,
                    block_operator_calls: outcome.block_operator_calls,
                    preconditioner_vectors: outcome.counters.preconditioner_apps,
                    block_preconditioner_calls: outcome.counters.block_preconditioner_apps,
                    linear_iterations: outcome.counters.linear_iterations,
                    orthogonalization_inner_products: outcome
                        .counters
                        .orthogonalization_inner_products,
                    orthogonalization_vector_updates: outcome
                        .counters
                        .orthogonalization_vector_updates,
                    initial_block_rank: outcome.initial_block_rank,
                    final_basis_dimension: outcome.final_basis_dimension,
                    estimated_krylov_bytes: 0,
                    explicit_jacobian_builds: outcome.counters.jacobian_builds,
                    factorization_builds: outcome.counters.direct_factorizations,
                });
            }
        }
    }
    rows.sort_by(|left, right| {
        left.case_id
            .cmp(&right.case_id)
            .then_with(|| left.solver.cmp(&right.solver))
    });
    let cases = specs
        .iter()
        .map(|spec| spec.descriptor.clone())
        .collect::<Vec<_>>();
    let explicit_jacobian_builds = rows.iter().map(|row| row.explicit_jacobian_builds).sum();
    let factorization_builds = rows.iter().map(|row| row.factorization_builds).sum();
    let successful_rows = rows.iter().filter(|row| row.success).count();
    let failed_rows = rows.len() - successful_rows;
    let block_gmres_successes = rows
        .iter()
        .filter(|row| row.solver == "block-gmres" && row.success)
        .count();
    let seeded_shared_successes = rows
        .iter()
        .filter(|row| row.solver == "seeded-shared-gmres" && row.success)
        .count();
    let block_rows_above_1_15x = rows
        .iter()
        .filter(|row| {
            row.solver == "block-gmres"
                && row.success
                && row.speedup_vs_serial.is_some_and(|speedup| speedup >= 1.15)
        })
        .count();
    let seeded_rows_above_1_15x = rows
        .iter()
        .filter(|row| {
            row.solver == "seeded-shared-gmres"
                && row.success
                && row.speedup_vs_serial.is_some_and(|speedup| speedup >= 1.15)
        })
        .count();
    let strict_jacobian_free = explicit_jacobian_builds == 0 && factorization_builds == 0;
    let scientific_checksum = scientific_checksum(&cases, &rows);
    Ok(MatrixFreeCommonWReport {
        schema: "rodas5p-matrix-free-common-w-gate-v1",
        profile,
        cases,
        rows,
        successful_rows,
        failed_rows,
        strict_jacobian_free,
        explicit_jacobian_builds,
        factorization_builds,
        block_gmres_successes,
        seeded_shared_successes,
        block_rows_above_1_15x,
        seeded_rows_above_1_15x,
        scientific_checksum,
        verdict: if strict_jacobian_free && failed_rows == 0 {
            "matrix-free common-W correctness gate passed; performance remains empirical".into()
        } else {
            "matrix-free common-W gate has unresolved failures".into()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodas5p_core::{DenseMatrix, LuFactorization};

    #[test]
    fn smoke_matrix_free_solutions_match_offline_materialized_direct_oracle() -> CoreResult<()> {
        let spec = build_specs(MatrixFreeCommonWProfile::Smoke)?
            .into_iter()
            .next()
            .expect("smoke profile is nonempty");
        let execution = Arc::new(ParallelExecution::sequential());
        let operator = MatrixFreeCommonWOperator::new(&spec.descriptor, execution.clone());
        let n = operator.dimension();
        let mut matrix = DenseMatrix::zeros(n, n);
        for column in 0..n {
            let mut basis = vec![0.0; n];
            basis[column] = 1.0;
            let mut image = vec![0.0; n];
            operator.apply_formula(&basis, &mut image)?;
            for row in 0..n {
                matrix[(row, column)] = image[row];
            }
        }
        let direct = LuFactorization::new(&matrix)?.solve_rows(&spec.rhs_rows)?;
        for solver in [
            "independent-gmres-serial",
            "block-gmres",
            "seeded-shared-gmres",
        ] {
            let outcome = execute_solver(&spec, solver, execution.clone())?;
            assert!(outcome.success(), "{solver}: {:?}", outcome.failure);
            let difference = max_solution_difference(
                &direct,
                outcome.solutions.as_deref().expect("checked success"),
            )?;
            assert!(difference <= 2.0e-9, "{solver}: difference={difference:e}");
        }
        Ok(())
    }
}
