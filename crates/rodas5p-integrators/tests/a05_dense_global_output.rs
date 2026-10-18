//! A05 contracts that can be exercised without an interval-residual witness:
//! failed callback work is charged, and zero-span entry still validates all
//! public adaptive/linear configuration.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{CoreError, DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, DenseErrorControl, IntegrationMethod, OdeProblem, OutputSamplingPlan,
    OutputSchedule, integrate_adaptive_dense_observed_with_dense_error_control,
    prothero_robinson_problem,
};

type TestBatchRhs =
    Arc<dyn Fn(&[f64], &[Vec<f64>]) -> Result<Vec<Vec<f64>>, CoreError> + Send + Sync>;

fn problem_with_rhs(
    rhs: impl Fn(f64, &[f64], &mut [f64]) -> Result<(), CoreError> + Send + Sync + 'static,
) -> OdeProblem {
    OdeProblem::new(
        "a05-rhs-counter",
        1,
        Arc::new(rhs),
        None,
        Some(Arc::new(|_, _| Ok(DenseMatrix::zeros(1, 1)))),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn problem_with_counted_rhs() -> (OdeProblem, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = Arc::clone(&calls);
    let problem = problem_with_rhs(move |_, _, out| {
        callback_calls.fetch_add(1, Ordering::SeqCst);
        out[0] = 1.0;
        Ok(())
    });
    (problem, calls)
}

#[test]
fn scalar_rhs_attempts_are_counted_on_err_and_nonfinite_output() {
    let err_problem = problem_with_rhs(|_, _, _| {
        Err(CoreError::InvalidInput("synthetic callback failure".into()))
    });
    let mut counters = WorkCounters::default();
    assert!(err_problem.eval_rhs(0.0, &[1.0], &mut counters).is_err());
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (1, 1));

    let nan_problem = problem_with_rhs(|_, _, out| {
        out[0] = f64::NAN;
        Ok(())
    });
    let mut counters = WorkCounters::default();
    assert!(nan_problem.eval_rhs(0.0, &[1.0], &mut counters).is_err());
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (1, 1));

    let mut out = [0.0];
    let mut counters = WorkCounters::default();
    assert!(
        nan_problem
            .eval_rhs_into(0.0, &[1.0], &mut out, &mut counters)
            .is_err()
    );
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (1, 1));
}

#[test]
fn invalid_scalar_rhs_shapes_do_not_charge_or_enter_callbacks() {
    let (problem, calls) = problem_with_counted_rhs();
    let mut counters = WorkCounters::default();
    assert!(problem.eval_rhs(0.0, &[], &mut counters).is_err());
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (0, 0));
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let mut out = [];
    assert!(
        problem
            .eval_rhs_into(0.0, &[1.0], &mut out, &mut counters)
            .is_err()
    );
    assert_eq!((counters.rhs_calls, counters.rhs_evaluations), (0, 0));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

fn batch_problem(batch: Option<TestBatchRhs>) -> OdeProblem {
    OdeProblem::new(
        "a05-batch-counter",
        1,
        Arc::new(|_, _, out| {
            out[0] = 2.0;
            Ok(())
        }),
        batch,
        Some(Arc::new(|_, _| Ok(DenseMatrix::zeros(1, 1)))),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn batch_callback_failure_success_and_scalar_fallback_are_accounted() {
    let failed_calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = Arc::clone(&failed_calls);
    let failed = batch_problem(Some(Arc::new(move |_, _| {
        callback_calls.fetch_add(1, Ordering::SeqCst);
        Err(CoreError::InvalidInput("synthetic batch failure".into()))
    })));
    let times = [0.0, 0.5];
    let states = vec![vec![1.0], vec![2.0]];
    let mut counters = WorkCounters::default();
    assert!(
        failed
            .eval_rhs_batch(&times, &[vec![1.0], vec![]], &mut counters)
            .is_err()
    );
    assert_eq!((counters.rhs_batch_calls, counters.rhs_evaluations), (0, 0));
    assert_eq!(failed_calls.load(Ordering::SeqCst), 0);
    assert!(
        failed
            .eval_rhs_batch(&times, &states, &mut counters)
            .is_err()
    );
    assert_eq!((counters.rhs_batch_calls, counters.rhs_evaluations), (1, 2));
    assert_eq!(failed_calls.load(Ordering::SeqCst), 1);

    let successful_calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = Arc::clone(&successful_calls);
    let successful = batch_problem(Some(Arc::new(move |_, states| {
        callback_calls.fetch_add(1, Ordering::SeqCst);
        Ok(states.iter().map(|_| vec![3.0]).collect())
    })));
    let mut counters = WorkCounters::default();
    let values = successful
        .eval_rhs_batch(&times, &states, &mut counters)
        .unwrap();
    assert_eq!(values, vec![vec![3.0], vec![3.0]]);
    assert_eq!((counters.rhs_batch_calls, counters.rhs_evaluations), (1, 2));
    assert_eq!(successful_calls.load(Ordering::SeqCst), 1);

    let fallback = batch_problem(None);
    let mut counters = WorkCounters::default();
    let values = fallback
        .eval_rhs_batch(&times, &states, &mut counters)
        .unwrap();
    assert_eq!(values, vec![vec![2.0], vec![2.0]]);
    assert_eq!(
        (
            counters.rhs_batch_calls,
            counters.rhs_calls,
            counters.rhs_evaluations
        ),
        (0, 2, 2)
    );

    let fallback_error = problem_with_rhs(|_, _, _| {
        Err(CoreError::InvalidInput(
            "synthetic scalar fallback failure".into(),
        ))
    });
    let mut counters = WorkCounters::default();
    assert!(
        fallback_error
            .eval_rhs_batch(&times, &states, &mut counters)
            .is_err()
    );
    assert_eq!(
        (
            counters.rhs_batch_calls,
            counters.rhs_calls,
            counters.rhs_evaluations
        ),
        (0, 1, 1)
    );
}

#[test]
fn zero_span_dense_entry_rejects_invalid_adaptive_and_linear_config() {
    let (problem, y0) = prothero_robinson_problem(-10.0, 0.0, 0.0);
    let sampling = OutputSamplingPlan::dense(OutputSchedule::new(vec![0.0]).unwrap());
    let invalid_adaptive = AdaptiveStepConfig {
        rtol: f64::NAN,
        ..AdaptiveStepConfig::default()
    };
    let linear = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    assert!(
        integrate_adaptive_dense_observed_with_dense_error_control(
            &problem,
            (0.0, 0.0),
            &y0,
            IntegrationMethod::Sequential,
            Some(&linear),
            None,
            &invalid_adaptive,
            &sampling,
            DenseErrorControl::Off,
        )
        .is_err()
    );

    let invalid_linear = LinearSolverConfig {
        rtol: f64::INFINITY,
        ..linear
    };
    assert!(
        integrate_adaptive_dense_observed_with_dense_error_control(
            &problem,
            (0.0, 0.0),
            &y0,
            IntegrationMethod::Sequential,
            Some(&invalid_linear),
            None,
            &AdaptiveStepConfig::default(),
            &sampling,
            DenseErrorControl::Off,
        )
        .is_err()
    );
}
