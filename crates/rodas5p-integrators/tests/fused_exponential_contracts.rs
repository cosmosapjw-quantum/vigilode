use rodas5p_core::safe_l2;
use rodas5p_integrators::OdeProblem;
use rodas5p_integrators::{
    FusedOrthogonalization, FusedPhiKrylovConfig, ParallelExecution, exprb2_fused_step,
    exprb43_fused_step, pexprb54s4_fused_step,
};
use std::sync::Arc;

fn square_problem() -> OdeProblem {
    OdeProblem::new(
        "square",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = 2.0 * y[0] * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        Some(Arc::new(|t| vec![1.0 / (1.0 - t)])),
    )
    .unwrap()
}

fn cfg() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 8,
        dimension_increment: 1,
        relative_tolerance: 1e-12,
        absolute_tolerance: 1e-14,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 8,
    }
}

fn integrate(method: &str, h: f64) -> f64 {
    let problem = square_problem();
    let mut y = vec![1.0];
    let mut t = 0.0;
    let execution = ParallelExecution::sequential();
    for _ in 0..(0.25 / h).round() as usize {
        let report = match method {
            "e2" => exprb2_fused_step(&problem, t, &y, h, cfg()).unwrap(),
            "e43" => exprb43_fused_step(&problem, t, &y, h, cfg(), &execution).unwrap(),
            "p54" => pexprb54s4_fused_step(&problem, t, &y, h, cfg(), &execution).unwrap(),
            _ => unreachable!(),
        };
        y = report.y_new;
        t += h;
    }
    (y[0] - 1.0 / (1.0 - 0.25)).abs()
}

#[test]
fn fused_methods_retain_declared_orders() {
    for (method, minimum) in [("e2", 1.8), ("e43", 3.7), ("p54", 4.7)] {
        let coarse = integrate(method, 0.025);
        let fine = integrate(method, 0.0125);
        let order = (coarse / fine).log2();
        assert!(
            order > minimum,
            "method={method} coarse={coarse:e} fine={fine:e} order={order}"
        );
    }
}

#[test]
fn fused_parallel_method_uses_five_logical_phi_actions() {
    let problem = square_problem();
    let report = pexprb54s4_fused_step(
        &problem,
        0.0,
        &[1.0],
        0.01,
        cfg(),
        &ParallelExecution::sequential(),
    )
    .unwrap();
    assert_eq!(report.logical_critical_depth, 3);
    assert_eq!(report.fused_phi_reports.len(), 5);
    assert_eq!(report.work.phi_actions, 5);
    assert_eq!(report.work.jacobian_builds, 0);
    assert_eq!(report.work.direct_factorizations, 0);
    assert_eq!(report.work.nonlinear_iterations, 0);
    assert!(safe_l2(report.error_estimate.as_ref().unwrap()).is_finite());
}
