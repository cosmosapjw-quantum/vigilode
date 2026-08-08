use rodas5p_core::{DenseMatrix, WorkCounters};
use rodas5p_integrators::{NewtonConfig, solve_dense_newton};

#[test]
fn dense_newton_solves_scalar_root_and_counts_every_operation() {
    let mut counters = WorkCounters::default();
    let report = solve_dense_newton(
        &[1.0],
        &[2.0_f64.sqrt()],
        &NewtonConfig {
            atol: 1.0e-13,
            rtol: 1.0e-11,
            max_iterations: 12,
            ..NewtonConfig::default()
        },
        &mut counters,
        |x, _| Ok(vec![x[0] * x[0] - 2.0]),
        |x, _| DenseMatrix::new(1, 1, vec![2.0 * x[0]]),
    )
    .unwrap();

    assert!(report.converged);
    assert!((report.x[0] - 2.0_f64.sqrt()).abs() < 1.0e-12);
    assert_eq!(counters.nonlinear_solves, 1);
    assert_eq!(counters.nonlinear_iterations, report.iterations as u64);
    assert!(counters.nonlinear_residual_evaluations >= report.iterations as u64 + 1);
    assert_eq!(
        counters.nonlinear_jacobian_evaluations,
        report.iterations as u64
    );
    assert_eq!(counters.direct_factorizations, report.iterations as u64);
    assert_eq!(counters.direct_solve_calls, report.iterations as u64);
}
