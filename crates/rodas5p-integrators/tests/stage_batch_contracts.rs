use std::{sync::Arc, time::Duration};

use rodas5p_core::{ClosureOperator, CoreResult, WorkCounters};
use rodas5p_integrators::{
    OdeProblem, ParallelExecution, StageBatchFeasibilityProfile, run_stage_batch_feasibility,
};

fn assert_rows_close(left: &[Vec<f64>], right: &[Vec<f64>], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() <= tolerance, "{x} != {y}");
        }
    }
}

#[test]
fn local_rayon_pool_executes_rhs_rows_concurrently_and_preserves_order() -> CoreResult<()> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let active_rhs = Arc::clone(&active);
    let maximum_rhs = Arc::clone(&maximum);
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let now = active_rhs.fetch_add(1, Ordering::SeqCst) + 1;
        maximum_rhs.fetch_max(now, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(2));
        for (dst, src) in out.iter_mut().zip(y) {
            *dst = *src + t;
        }
        active_rhs.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out.copy_from_slice(v);
        Ok(())
    });
    let problem = OdeProblem::new(
        "within-step-concurrency-contract",
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
    let times = (0..8).map(|i| i as f64 * 0.1).collect::<Vec<_>>();
    let states = (0..8)
        .map(|stage| vec![stage as f64; 4])
        .collect::<Vec<_>>();

    let mut serial_counters = WorkCounters::default();
    let serial = problem.eval_rhs_stage_rows(
        &times,
        &states,
        &ParallelExecution::sequential(),
        &mut serial_counters,
    )?;
    let mut parallel_counters = WorkCounters::default();
    let parallel = problem.eval_rhs_stage_rows(
        &times,
        &states,
        &ParallelExecution::rayon(4)?,
        &mut parallel_counters,
    )?;

    assert_rows_close(&serial, &parallel, 0.0);
    assert!(maximum.load(Ordering::SeqCst) >= 2);
    assert_eq!(parallel_counters.rhs_batch_calls, 1);
    assert_eq!(parallel_counters.rhs_evaluations, 8);
    Ok(())
}

#[test]
fn shared_operator_rows_are_identical_for_sequential_and_stage_parallel_execution() -> CoreResult<()>
{
    let operator = ClosureOperator::new(5, |x: &[f64], y: &mut [f64]| {
        for i in 0..5 {
            y[i] = 2.0 * x[i] - x[(i + 1) % 5];
        }
        Ok(())
    });
    let rows = (0..8)
        .map(|stage| {
            (0..5)
                .map(|component| (stage * 5 + component) as f64 * 0.125)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let serial = ParallelExecution::sequential().apply_operator_rows(&operator, &rows)?;
    let parallel = ParallelExecution::rayon(4)?.apply_operator_rows(&operator, &rows)?;
    assert_rows_close(&serial, &parallel, 0.0);
    Ok(())
}

#[test]
fn failed_parallel_rhs_batch_preserves_attempted_work_accounting() -> CoreResult<()> {
    let rhs = Arc::new(|t: f64, y: &[f64], out: &mut [f64]| {
        if (t - 0.3).abs() < 1.0e-12 {
            return Err(rodas5p_core::CoreError::InvalidInput(
                "intentional stage failure".into(),
            ));
        }
        out.copy_from_slice(y);
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out.copy_from_slice(v);
        Ok(())
    });
    let problem = OdeProblem::new(
        "failed-stage-accounting-contract",
        2,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )?;
    let times = (0..8).map(|i| i as f64 * 0.1).collect::<Vec<_>>();
    let states = vec![vec![1.0; 2]; 8];
    let mut counters = WorkCounters::default();
    let result = problem.eval_rhs_stage_rows(
        &times,
        &states,
        &ParallelExecution::rayon(4)?,
        &mut counters,
    );
    assert!(result.is_err());
    assert_eq!(counters.rhs_batch_calls, 1);
    assert_eq!(counters.rhs_evaluations, 8);
    Ok(())
}

#[test]
fn smoke_feasibility_report_is_deterministic_and_covers_all_required_kernels() -> CoreResult<()> {
    let first = run_stage_batch_feasibility(StageBatchFeasibilityProfile::Smoke)?;
    let second = run_stage_batch_feasibility(StageBatchFeasibilityProfile::Smoke)?;

    assert_eq!(first.scientific_checksum, second.scientific_checksum);
    assert_eq!(first.schema, "rodas5p-stage-batch-feasibility-v1");
    assert!(first.stage_parallelism_observed);
    assert!(first.observed_max_parallel_tasks >= 2);
    assert!(first.rhs_and_jvp_paths_matrix_free);
    assert!(first.common_w_dense_reference_setup_used);
    assert!(!first.strict_jacobian_free_common_w_demonstrated);
    assert!(!first.explicit_simd_demonstrated);
    for kernel in ["rhs", "jvp", "common-w", "combined-round"] {
        assert!(first.rows.iter().any(|row| row.kernel == kernel));
    }
    assert!(
        first
            .rows
            .iter()
            .all(|row| row.max_abs_difference <= 1.0e-10)
    );
    assert_eq!(first.cases.len(), 2);
    Ok(())
}
