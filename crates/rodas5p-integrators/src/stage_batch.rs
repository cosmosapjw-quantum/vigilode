use std::{
    hint::black_box,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearOperator, LuFactorization, WorkCounters,
    load_rodas5p_coefficients, sha256_hex,
};
use serde::Serialize;

use crate::{OdeProblem, ParallelExecution};

const STAGES: usize = 8;
const PROMOTION_SPEEDUP: f64 = 1.15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StageBatchFeasibilityProfile {
    Smoke,
    Canonical,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StageBatchFeasibilityCase {
    pub case_id: String,
    pub family: String,
    pub dimension: usize,
    pub work_repeats: usize,
    pub stages: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StageBatchFeasibilityRow {
    pub case_id: String,
    pub kernel: String,
    pub backend: String,
    pub threads: usize,
    pub stages: usize,
    pub dimension: usize,
    pub work_repeats: usize,
    pub timing_iterations: usize,
    pub timing_repetitions: usize,
    pub median_seconds: f64,
    pub speedup_vs_sequential: f64,
    pub max_abs_difference: f64,
    pub actual_stage_parallel: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StageBatchFeasibilityReport {
    pub schema: &'static str,
    pub profile: StageBatchFeasibilityProfile,
    pub cases: Vec<StageBatchFeasibilityCase>,
    pub rows: Vec<StageBatchFeasibilityRow>,
    pub observed_max_parallel_tasks: usize,
    pub maximum_rhs_speedup: f64,
    pub maximum_jvp_speedup: f64,
    pub maximum_common_w_speedup: f64,
    pub maximum_combined_speedup: f64,
    pub combined_rows_above_1_15x: usize,
    pub stage_parallelism_observed: bool,
    /// The benchmark physics definitions expose JVP closures without an explicit Jacobian.
    pub rhs_and_jvp_paths_matrix_free: bool,
    /// The common-W microbenchmark materializes a dense W during untimed setup and is therefore
    /// a multiple-RHS hardware reference, not a strict Jacobian-free solver demonstration.
    pub common_w_dense_reference_setup_used: bool,
    /// No matrix-free block-Krylov common-W solve is implemented in this gate.
    pub strict_jacobian_free_common_w_demonstrated: bool,
    /// The component-major batch is vectorization-ready, but no explicit portable-SIMD kernel is
    /// claimed by this report.
    pub explicit_simd_demonstrated: bool,
    pub scientific_checksum: String,
    pub verdict: String,
}

struct BenchmarkRuntimeCase {
    descriptor: StageBatchFeasibilityCase,
    problem: OdeProblem,
    times: Vec<f64>,
    states: Vec<Vec<f64>>,
    vectors: Vec<Vec<f64>>,
    jvp_operator: Arc<dyn LinearOperator>,
    w_factor: LuFactorization,
}

#[derive(Clone, Copy)]
struct TimingConfig {
    repetitions: usize,
    target: Duration,
    maximum_iterations: usize,
}

#[derive(Clone, Copy)]
struct TimingResult {
    median_seconds: f64,
    iterations: usize,
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    if values.len() % 2 == 1 {
        values[values.len() / 2]
    } else {
        let upper = values.len() / 2;
        0.5 * (values[upper - 1] + values[upper])
    }
}

fn time_kernel<F>(config: TimingConfig, mut operation: F) -> CoreResult<TimingResult>
where
    F: FnMut() -> CoreResult<()>,
{
    operation()?;
    let probe_start = Instant::now();
    operation()?;
    let probe = probe_start.elapsed().as_secs_f64().max(1.0e-9);
    let iterations = ((config.target.as_secs_f64() / probe).ceil() as usize)
        .clamp(1, config.maximum_iterations.max(1));
    let mut samples = Vec::with_capacity(config.repetitions);
    for _ in 0..config.repetitions {
        let start = Instant::now();
        for _ in 0..iterations {
            operation()?;
        }
        samples.push(start.elapsed().as_secs_f64() / iterations as f64);
    }
    Ok(TimingResult {
        median_seconds: median(&mut samples),
        iterations,
    })
}

fn max_abs_difference(left: &[Vec<f64>], right: &[Vec<f64>]) -> CoreResult<f64> {
    if left.len() != right.len() || left.iter().zip(right).any(|(a, b)| a.len() != b.len()) {
        return Err(CoreError::Dimension(
            "stage batch comparison shape mismatch".into(),
        ));
    }
    Ok(left
        .iter()
        .zip(right)
        .flat_map(|(a, b)| a.iter().zip(b))
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max))
}

fn checksum_rows(rows: &[Vec<f64>]) -> f64 {
    rows.iter()
        .enumerate()
        .flat_map(|(stage, row)| {
            row.iter().enumerate().map(move |(component, value)| {
                (stage as f64 + 1.0) * (component as f64 + 1.0) * value
            })
        })
        .sum()
}

fn benchmark_problem(
    family: &str,
    dimension: usize,
    work_repeats: usize,
) -> CoreResult<BenchmarkRuntimeCase> {
    if dimension == 0 || (family == "complex-dahlquist" && !dimension.is_multiple_of(2)) {
        return Err(CoreError::InvalidInput(
            "invalid stage-batch benchmark dimension".into(),
        ));
    }
    let coeffs = load_rodas5p_coefficients()?;
    let (problem, base_time, h, states) = if family == "complex-dahlquist" {
        let damping = 120.0;
        let frequency = 180.0;
        let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
            for pair in 0..dimension / 2 {
                let i = 2 * pair;
                out[i] = -damping * y[i] - frequency * y[i + 1];
                out[i + 1] = frequency * y[i] - damping * y[i + 1];
            }
            Ok(())
        });
        let batch = Arc::new(move |_times: &[f64], states: &[Vec<f64>]| {
            let mut out = vec![vec![0.0; dimension]; states.len()];
            for pair in 0..dimension / 2 {
                let i = 2 * pair;
                for stage in 0..states.len() {
                    out[stage][i] = -damping * states[stage][i] - frequency * states[stage][i + 1];
                    out[stage][i + 1] =
                        frequency * states[stage][i] - damping * states[stage][i + 1];
                }
            }
            Ok(out)
        });
        let jvp = Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
            for pair in 0..dimension / 2 {
                let i = 2 * pair;
                out[i] = -damping * v[i] - frequency * v[i + 1];
                out[i + 1] = frequency * v[i] - damping * v[i + 1];
            }
            Ok(())
        });
        let problem = OdeProblem::new(
            format!("complex-dahlquist-n{dimension}"),
            dimension,
            rhs,
            Some(batch),
            None,
            Some(jvp),
            None,
            true,
            None,
            None,
        )?;
        let base_time: f64 = 0.125;
        let h: f64 = 1.0e-3;
        let times = coeffs
            .c
            .iter()
            .map(|c| base_time + c * h)
            .collect::<Vec<_>>();
        let states = times
            .iter()
            .enumerate()
            .map(|(stage, &t)| {
                let envelope = (-damping * t).exp();
                (0..dimension / 2)
                    .flat_map(|pair| {
                        let phase = frequency * t + 0.013 * pair as f64;
                        let epsilon = 1.0e-5 * (stage + 1) as f64;
                        [
                            envelope * phase.cos() + epsilon,
                            envelope * phase.sin() - epsilon,
                        ]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        (problem, base_time, h, states)
    } else {
        let stiffness = -2.0e4;
        let frequency = 140.0;
        let nonlinearity = 2.5e3;
        let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            for component in 0..dimension {
                let phase = 0.007 * component as f64;
                let angle = frequency * t + phase;
                let phi = angle.sin();
                let defect = y[component] - phi;
                out[component] =
                    stiffness * defect + frequency * angle.cos() + nonlinearity * defect.powi(3);
            }
            Ok(())
        });
        let batch = Arc::new(move |times: &[f64], states: &[Vec<f64>]| {
            let mut out = vec![vec![0.0; dimension]; states.len()];
            for component in 0..dimension {
                let phase = 0.007 * component as f64;
                for stage in 0..states.len() {
                    let angle = frequency * times[stage] + phase;
                    let phi = angle.sin();
                    let defect = states[stage][component] - phi;
                    out[stage][component] = stiffness * defect
                        + frequency * angle.cos()
                        + nonlinearity * defect.powi(3);
                }
            }
            Ok(out)
        });
        let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
            for component in 0..dimension {
                let phi = (frequency * t + 0.007 * component as f64).sin();
                let defect = y[component] - phi;
                out[component] = (stiffness + 3.0 * nonlinearity * defect * defect) * v[component];
            }
            Ok(())
        });
        let problem = OdeProblem::new(
            format!("oscillatory-pr-n{dimension}"),
            dimension,
            rhs,
            Some(batch),
            None,
            Some(jvp),
            None,
            false,
            None,
            None,
        )?;
        let base_time: f64 = 0.03125;
        let h: f64 = 5.0e-5;
        let times = coeffs
            .c
            .iter()
            .map(|c| base_time + c * h)
            .collect::<Vec<_>>();
        let states = times
            .iter()
            .enumerate()
            .map(|(stage, &t)| {
                (0..dimension)
                    .map(|component| {
                        (frequency * t + 0.007 * component as f64).sin()
                            + 5.0e-6 * (stage + 1) as f64
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        (problem, base_time, h, states)
    };
    let times = coeffs
        .c
        .iter()
        .map(|c| base_time + c * h)
        .collect::<Vec<_>>();
    let vectors = (0..STAGES)
        .map(|stage| {
            (0..dimension)
                .map(|component| ((stage + 2) as f64 * (component + 1) as f64 * 0.011).cos())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut work = WorkCounters::default();
    let jvp_operator = problem.linearize(times[0], &states[0], &mut work)?;
    let mut jacobian = DenseMatrix::zeros(dimension, dimension);
    for column in 0..dimension {
        let mut basis = vec![0.0; dimension];
        basis[column] = 1.0;
        let mut out = vec![0.0; dimension];
        jvp_operator.apply(&basis, &mut out)?;
        for row in 0..dimension {
            jacobian[(row, column)] = out[row];
        }
    }
    let w = DenseMatrix::identity(dimension).sub(&jacobian.scale(h * coeffs.gamma))?;
    let w_factor = LuFactorization::new(&w)?;
    Ok(BenchmarkRuntimeCase {
        descriptor: StageBatchFeasibilityCase {
            case_id: format!("{family}-n{dimension}-r{work_repeats}"),
            family: family.into(),
            dimension,
            work_repeats,
            stages: STAGES,
        },
        problem,
        times,
        states,
        vectors,
        jvp_operator,
        w_factor,
    })
}

fn build_cases(profile: StageBatchFeasibilityProfile) -> CoreResult<Vec<BenchmarkRuntimeCase>> {
    let (dimensions, repeats) = match profile {
        StageBatchFeasibilityProfile::Smoke => (&[16_usize][..], &[1_usize][..]),
        StageBatchFeasibilityProfile::Canonical => {
            (&[32_usize, 128, 512][..], &[1_usize, 8, 32][..])
        }
    };
    let mut cases = Vec::new();
    for &dimension in dimensions {
        for &work_repeats in repeats {
            cases.push(benchmark_problem(
                "complex-dahlquist",
                dimension,
                work_repeats,
            )?);
            cases.push(benchmark_problem(
                "oscillatory-prothero-robinson",
                dimension,
                work_repeats,
            )?);
        }
    }
    Ok(cases)
}

fn rhs_rows(
    case: &BenchmarkRuntimeCase,
    execution: &ParallelExecution,
) -> CoreResult<Vec<Vec<f64>>> {
    let mut counters = WorkCounters::default();
    let mut out = Vec::new();
    for _ in 0..case.descriptor.work_repeats {
        out = case.problem.eval_rhs_stage_rows(
            &case.times,
            &case.states,
            execution,
            &mut counters,
        )?;
        black_box(&out);
    }
    Ok(out)
}

fn rhs_provided(case: &BenchmarkRuntimeCase) -> CoreResult<Vec<Vec<f64>>> {
    let mut counters = WorkCounters::default();
    let mut out = Vec::new();
    for _ in 0..case.descriptor.work_repeats {
        out = case
            .problem
            .eval_rhs_batch(&case.times, &case.states, &mut counters)?;
        black_box(&out);
    }
    Ok(out)
}

fn jvp_rows(
    case: &BenchmarkRuntimeCase,
    execution: &ParallelExecution,
) -> CoreResult<Vec<Vec<f64>>> {
    let mut out = Vec::new();
    for _ in 0..case.descriptor.work_repeats {
        out = execution.apply_operator_rows(case.jvp_operator.as_ref(), &case.vectors)?;
        black_box(&out);
    }
    Ok(out)
}

fn solve_serial(case: &BenchmarkRuntimeCase, rows: &[Vec<f64>]) -> CoreResult<Vec<Vec<f64>>> {
    let mut out = Vec::new();
    for _ in 0..case.descriptor.work_repeats {
        out = rows
            .iter()
            .map(|row| case.w_factor.solve(row))
            .collect::<CoreResult<Vec<_>>>()?;
        black_box(&out);
    }
    Ok(out)
}

fn solve_batched(case: &BenchmarkRuntimeCase, rows: &[Vec<f64>]) -> CoreResult<Vec<Vec<f64>>> {
    let mut out = Vec::new();
    for _ in 0..case.descriptor.work_repeats {
        out = case.w_factor.solve_rows(rows)?;
        black_box(&out);
    }
    Ok(out)
}

fn combined_round(
    case: &BenchmarkRuntimeCase,
    rhs_backend: &str,
    execution: &ParallelExecution,
    batched_solve: bool,
) -> CoreResult<Vec<Vec<f64>>> {
    let rhs = if rhs_backend == "provided-component-major" {
        rhs_provided(case)?
    } else {
        rhs_rows(case, execution)?
    };
    let jvp = jvp_rows(case, execution)?;
    let mixed = rhs
        .iter()
        .zip(jvp)
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x + 1.0e-3 * y).collect())
        .collect::<Vec<Vec<f64>>>();
    if batched_solve {
        solve_batched(case, &mixed)
    } else {
        solve_serial(case, &mixed)
    }
}

#[allow(clippy::too_many_arguments)]
fn push_row(
    rows: &mut Vec<StageBatchFeasibilityRow>,
    case: &BenchmarkRuntimeCase,
    kernel: &str,
    backend: &str,
    threads: usize,
    timing: TimingResult,
    baseline_seconds: f64,
    max_abs_difference: f64,
    actual_stage_parallel: bool,
    repetitions: usize,
) {
    rows.push(StageBatchFeasibilityRow {
        case_id: case.descriptor.case_id.clone(),
        kernel: kernel.into(),
        backend: backend.into(),
        threads,
        stages: STAGES,
        dimension: case.descriptor.dimension,
        work_repeats: case.descriptor.work_repeats,
        timing_iterations: timing.iterations,
        timing_repetitions: repetitions,
        median_seconds: timing.median_seconds,
        speedup_vs_sequential: baseline_seconds / timing.median_seconds,
        max_abs_difference,
        actual_stage_parallel,
    });
}

fn run_concurrency_probe() -> CoreResult<usize> {
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let active_rhs = Arc::clone(&active);
    let maximum_rhs = Arc::clone(&maximum);
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let now = active_rhs.fetch_add(1, Ordering::SeqCst) + 1;
        let mut observed = maximum_rhs.load(Ordering::Relaxed);
        while now > observed {
            match maximum_rhs.compare_exchange_weak(
                observed,
                now,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(value) => observed = value,
            }
        }
        std::thread::sleep(Duration::from_millis(1));
        out.copy_from_slice(y);
        active_rhs.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out.copy_from_slice(v);
        Ok(())
    });
    let problem = OdeProblem::new(
        "stage-concurrency-probe",
        4,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )?;
    let mut counters = WorkCounters::default();
    problem.eval_rhs_stage_rows(
        &[0.0; STAGES],
        &vec![vec![1.0; 4]; STAGES],
        &ParallelExecution::rayon(4)?,
        &mut counters,
    )?;
    Ok(maximum.load(Ordering::Relaxed))
}

fn scientific_checksum(
    cases: &[StageBatchFeasibilityCase],
    rows: &[StageBatchFeasibilityRow],
    stage_parallelism_observed: bool,
) -> String {
    // The exact maximum number of simultaneously observed tasks is scheduler-dependent.  It is a
    // diagnostic, not a scientific field.  Hash only the stable contract that genuine within-step
    // concurrency was observed, while retaining the exact maximum in the report itself.
    let mut signature = format!("parallel-observed={stage_parallelism_observed}\n");
    for case in cases {
        signature.push_str(&format!(
            "case|{}|{}|{}|{}|{}\n",
            case.case_id, case.family, case.dimension, case.work_repeats, case.stages
        ));
    }
    for row in rows {
        signature.push_str(&format!(
            "row|{}|{}|{}|{}|{}|{}|{}|{:016x}|{}\n",
            row.case_id,
            row.kernel,
            row.backend,
            row.threads,
            row.stages,
            row.dimension,
            row.work_repeats,
            row.max_abs_difference.to_bits(),
            row.actual_stage_parallel
        ));
    }
    sha256_hex(signature.as_bytes())
}

pub fn run_stage_batch_feasibility(
    profile: StageBatchFeasibilityProfile,
) -> CoreResult<StageBatchFeasibilityReport> {
    let cases = build_cases(profile)?;
    let timing = match profile {
        StageBatchFeasibilityProfile::Smoke => TimingConfig {
            repetitions: 2,
            target: Duration::from_micros(100),
            maximum_iterations: 8,
        },
        StageBatchFeasibilityProfile::Canonical => TimingConfig {
            repetitions: 7,
            target: Duration::from_millis(5),
            maximum_iterations: 2048,
        },
    };
    let sequential = ParallelExecution::sequential();
    let rayon_two = ParallelExecution::rayon(2)?;
    let rayon_four = ParallelExecution::rayon(4)?;
    let mut rows = Vec::new();

    for case in &cases {
        let rhs_reference = rhs_rows(case, &sequential)?;
        let rhs_serial = time_kernel(timing, || {
            black_box(checksum_rows(&rhs_rows(case, &sequential)?));
            Ok(())
        })?;
        push_row(
            &mut rows,
            case,
            "rhs",
            "sequential-scalar",
            1,
            rhs_serial,
            rhs_serial.median_seconds,
            0.0,
            false,
            timing.repetitions,
        );
        for (name, execution) in [
            ("rayon-stage-2", &rayon_two),
            ("rayon-stage-4", &rayon_four),
        ] {
            let result = rhs_rows(case, execution)?;
            let difference = max_abs_difference(&rhs_reference, &result)?;
            let measured = time_kernel(timing, || {
                black_box(checksum_rows(&rhs_rows(case, execution)?));
                Ok(())
            })?;
            push_row(
                &mut rows,
                case,
                "rhs",
                name,
                execution.threads(),
                measured,
                rhs_serial.median_seconds,
                difference,
                true,
                timing.repetitions,
            );
        }
        let provided = rhs_provided(case)?;
        let difference = max_abs_difference(&rhs_reference, &provided)?;
        let measured = time_kernel(timing, || {
            black_box(checksum_rows(&rhs_provided(case)?));
            Ok(())
        })?;
        push_row(
            &mut rows,
            case,
            "rhs",
            "provided-component-major",
            1,
            measured,
            rhs_serial.median_seconds,
            difference,
            false,
            timing.repetitions,
        );

        let jvp_reference = jvp_rows(case, &sequential)?;
        let jvp_serial = time_kernel(timing, || {
            black_box(checksum_rows(&jvp_rows(case, &sequential)?));
            Ok(())
        })?;
        push_row(
            &mut rows,
            case,
            "jvp",
            "sequential-scalar",
            1,
            jvp_serial,
            jvp_serial.median_seconds,
            0.0,
            false,
            timing.repetitions,
        );
        for (name, execution) in [
            ("rayon-stage-2", &rayon_two),
            ("rayon-stage-4", &rayon_four),
        ] {
            let result = jvp_rows(case, execution)?;
            let difference = max_abs_difference(&jvp_reference, &result)?;
            let measured = time_kernel(timing, || {
                black_box(checksum_rows(&jvp_rows(case, execution)?));
                Ok(())
            })?;
            push_row(
                &mut rows,
                case,
                "jvp",
                name,
                execution.threads(),
                measured,
                jvp_serial.median_seconds,
                difference,
                true,
                timing.repetitions,
            );
        }

        let solve_reference = solve_serial(case, &case.vectors)?;
        let solve_serial_timing = time_kernel(timing, || {
            black_box(checksum_rows(&solve_serial(case, &case.vectors)?));
            Ok(())
        })?;
        push_row(
            &mut rows,
            case,
            "common-w",
            "sequential-single-rhs",
            1,
            solve_serial_timing,
            solve_serial_timing.median_seconds,
            0.0,
            false,
            timing.repetitions,
        );
        let solve_batch = solve_batched(case, &case.vectors)?;
        let difference = max_abs_difference(&solve_reference, &solve_batch)?;
        let measured = time_kernel(timing, || {
            black_box(checksum_rows(&solve_batched(case, &case.vectors)?));
            Ok(())
        })?;
        push_row(
            &mut rows,
            case,
            "common-w",
            "faer-multi-rhs",
            1,
            measured,
            solve_serial_timing.median_seconds,
            difference,
            false,
            timing.repetitions,
        );

        let combined_reference = combined_round(case, "sequential", &sequential, false)?;
        let combined_serial = time_kernel(timing, || {
            black_box(checksum_rows(&combined_round(
                case,
                "sequential",
                &sequential,
                false,
            )?));
            Ok(())
        })?;
        push_row(
            &mut rows,
            case,
            "combined-round",
            "sequential-scalar",
            1,
            combined_serial,
            combined_serial.median_seconds,
            0.0,
            false,
            timing.repetitions,
        );
        for (name, rhs_backend, execution) in [
            (
                "provided-batch+multi-rhs",
                "provided-component-major",
                &sequential,
            ),
            ("rayon-stage-2+multi-rhs", "rayon", &rayon_two),
            ("rayon-stage-4+multi-rhs", "rayon", &rayon_four),
        ] {
            let result = combined_round(case, rhs_backend, execution, true)?;
            let difference = max_abs_difference(&combined_reference, &result)?;
            let measured = time_kernel(timing, || {
                black_box(checksum_rows(&combined_round(
                    case,
                    rhs_backend,
                    execution,
                    true,
                )?));
                Ok(())
            })?;
            push_row(
                &mut rows,
                case,
                "combined-round",
                name,
                execution.threads(),
                measured,
                combined_serial.median_seconds,
                difference,
                execution.threads() > 1,
                timing.repetitions,
            );
        }
    }

    rows.sort_by(|a, b| {
        (&a.case_id, &a.kernel, &a.backend, a.threads)
            .cmp(&(&b.case_id, &b.kernel, &b.backend, b.threads))
    });
    let descriptors = cases
        .iter()
        .map(|case| case.descriptor.clone())
        .collect::<Vec<_>>();
    let observed_max_parallel_tasks = run_concurrency_probe()?;
    let maximum_for = |kernel: &str| {
        rows.iter()
            .filter(|row| row.kernel == kernel)
            .map(|row| row.speedup_vs_sequential)
            .fold(0.0_f64, f64::max)
    };
    let maximum_rhs_speedup = maximum_for("rhs");
    let maximum_jvp_speedup = maximum_for("jvp");
    let maximum_common_w_speedup = maximum_for("common-w");
    let maximum_combined_speedup = maximum_for("combined-round");
    let combined_rows_above_1_15x = rows
        .iter()
        .filter(|row| {
            row.kernel == "combined-round"
                && row.backend != "sequential-scalar"
                && row.speedup_vs_sequential >= PROMOTION_SPEEDUP
        })
        .count();
    let stage_parallelism_observed = observed_max_parallel_tasks >= 2;
    let verdict = if stage_parallelism_observed && combined_rows_above_1_15x > 0 {
        "conditional-stage-batch-feasibility-observed"
    } else if stage_parallelism_observed {
        "stage-concurrency-observed-without-combined-speedup"
    } else {
        "stage-concurrency-not-observed"
    };
    let scientific_checksum = scientific_checksum(&descriptors, &rows, stage_parallelism_observed);
    Ok(StageBatchFeasibilityReport {
        schema: "rodas5p-stage-batch-feasibility-v1",
        profile,
        cases: descriptors,
        rows,
        observed_max_parallel_tasks,
        maximum_rhs_speedup,
        maximum_jvp_speedup,
        maximum_common_w_speedup,
        maximum_combined_speedup,
        combined_rows_above_1_15x,
        stage_parallelism_observed,
        rhs_and_jvp_paths_matrix_free: true,
        common_w_dense_reference_setup_used: true,
        strict_jacobian_free_common_w_demonstrated: false,
        explicit_simd_demonstrated: false,
        scientific_checksum,
        verdict: verdict.into(),
    })
}
