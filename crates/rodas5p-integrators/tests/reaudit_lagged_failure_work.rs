//! End-to-end failed monitor-residual JVP keeps all prior Arnoldi work.
use rodas5p_core::{CoreError, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, Rodas5pLaggedOptions, StageSolver,
    integrate_rodas5p_lagged_observed,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[test]
fn recoverable_jvp_error_retains_completed_column_and_all_work_partitions() {
    let calls = Arc::new(AtomicU64::new(0));
    let jvp_calls = calls.clone();
    let problem = OdeProblem::new(
        "faulted-lagged-diagonal",
        3,
        Arc::new(|_, y, out| {
            for i in 0..3 {
                out[i] = -((i + 1) as f64) * y[i];
            }
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| {
            DenseMatrix::from_rows(&[&[-1., 0., 0.], &[0., -2., 0.], &[0., 0., -3.]])
        })),
        Some(Arc::new(move |_, _, v, out| {
            // initial explicit residual, cycle residual, first column,
            // then monitor residual: fail after the first column exists.
            if jvp_calls.fetch_add(1, Ordering::Relaxed) + 1 == 4 {
                return Err(CoreError::NonFinite("injected fourth JVP".into()));
            }
            for i in 0..3 {
                out[i] = -((i + 1) as f64) * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let config = AdaptiveStepConfig {
        initial_step: 0.01,
        max_step: 0.1,
        atol: 1e-8,
        rtol: 1e-6,
        ..Default::default()
    };
    let r = integrate_rodas5p_lagged_observed(
        &problem,
        (0., 1.),
        &[1.; 3],
        &config,
        &OutputSchedule::new(vec![0., 1.]).unwrap(),
        Rodas5pLaggedOptions {
            stage_solver: StageSolver::Gmres,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(r.fast.observed.success);
    let c = &r.fast.observed.counters;
    assert_eq!(c.nonfinite_step_failures, 1);
    assert!(r.work.abandoned_iterations >= 1);
    assert!(r.work.gmres_iterations > 0);
    assert!(r.work.gmres_column_attempts >= r.work.gmres_iterations);
    assert_eq!(c.linear_iterations, r.work.gmres_iterations);
    assert_eq!(
        r.work.operator_applications,
        r.work.gmres_column_applications + r.work.residual_applications
    );
    assert_eq!(c.linear_matvecs, r.work.operator_applications);
    assert_eq!(c.jvp_calls, r.work.jvp_applications);
    assert_eq!(c.jvp_calls, calls.load(Ordering::Relaxed));
    assert_eq!(c.direct_solve_calls, r.work.preconditioner_applications);
    for (i, y) in r.fast.observed.y.last().unwrap().iter().enumerate() {
        assert!((y - (-((i + 1) as f64)).exp()).abs() < 1e-6);
    }
}
