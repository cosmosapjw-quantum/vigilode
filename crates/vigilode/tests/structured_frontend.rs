#![cfg(feature = "structured")]
use vigilode::{
    VigilODE,
    structured::{
        CoupledCubicConfig, CoupledCubicSource, CubicDecayAccuracyPlan, CubicStageSolver,
        RhsAcceleration,
    },
};

// Independent closed solution, including the linear g=0 and equilibrium y0=0 limits.
fn exact(t: f64, k: f64, g: f64, y0: f64) -> f64 {
    y0 * (-k * t).exp() / (1.0 - (g / k) * y0 * y0 * (-2.0 * k * t).exp_m1()).sqrt()
}

#[test]
fn scalar_homotopy_and_inverse_meet_the_same_physical_budget() {
    for (k, g, y0) in [(1.0, 0.5, 1.0), (50.0, 2.0, 0.5), (2.0, 0.0, 0.0)] {
        let plan = CubicDecayAccuracyPlan::new(k, g, y0, 0.125, 1e-4, 65536, 1_000_000).unwrap();
        for solver in [
            CubicStageSolver::ScalarHomotopy(RhsAcceleration::Picard),
            CubicStageSolver::AlgebraicPredictor,
        ] {
            let sol = VigilODE::solve_cubic_decay(&plan, solver, true);
            assert!(sol.completed, "{:?}", sol.failure);
            for p in &sol.points {
                assert!((p.state - exact(p.time, k, g, y0)).abs() <= p.physical_error_bound);
                assert!(p.physical_error_bound <= plan.physical_tolerance());
            }
        }
    }
    let capped = CubicDecayAccuracyPlan::new(1., 0.5, 1., 0.125, 1e-4, 65536, 1).unwrap();
    let stopped = VigilODE::solve_cubic_decay(&capped, CubicStageSolver::AlgebraicPredictor, true);
    assert!(!stopped.completed && stopped.failure.is_some());
    assert_eq!(
        stopped.map_evaluations + stopped.interval_polynomial_evaluations,
        1
    );
}

#[test]
fn coupled_be_preserves_the_synchronous_solution_with_unequal_coupling() {
    let source = CoupledCubicSource::new(
        vec![1.; 2],
        vec![0.5; 2],
        vec![vec![0., 2.], vec![3., 0.]],
        vec![0.1; 2],
    )
    .unwrap();
    let cfg = CoupledCubicConfig {
        horizon: 0.125,
        steps: 128,
        physical_tolerance: 0.01,
        ..Default::default()
    };
    let sol = VigilODE::solve_coupled_cubic(&source, cfg);
    assert!(sol.completed, "{:?}", sol.failure);
    assert_eq!(sol.points.len(), cfg.steps + 1);
    for p in sol.points {
        assert!(
            p.state
                .iter()
                .all(|y| (y - exact(p.time, 1., 0.5, 0.1)).abs() <= p.physical_error_upper)
        );
        assert!(p.physical_error_upper <= cfg.physical_tolerance);
    }
}
