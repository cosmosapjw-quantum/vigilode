use vigilode::{Problem, SolverStrategy, VigilODE, VigilodeOptions};

fn main() {
    // These paths exercise the public crate, including the native coefficient fixture.
    let problem = Problem::new(1, |_t, y, dy| {
        dy[0] = -10.0 * y[0];
        Ok(())
    })
    .autonomous();
    let opts = VigilodeOptions {
        strategy: SolverStrategy::RobustStiff,
        ..Default::default()
    };
    let ordinary = VigilODE::solve(&problem, (0.0, 0.1), &[1.0], opts).unwrap();
    assert!(ordinary.success && ordinary.stats.accepted_steps > 0);
    let err = (ordinary.final_state[0] - (-1.0_f64).exp()).abs();
    assert!(err < 1.0e-6);
    println!(
        "protected endpoint={:.15} abs_error={:.3e}",
        ordinary.final_state[0], err
    );

    #[cfg(feature = "structured")]
    {
        use vigilode::structured::{CubicDecayAccuracyPlan, CubicStageSolver};

        let plan =
            CubicDecayAccuracyPlan::new(1.0, 0.5, 1.0, 0.125, 1e-4, 65536, 1_000_000).unwrap();
        let solution =
            VigilODE::solve_cubic_decay(&plan, CubicStageSolver::AlgebraicPredictor, true);
        assert!(solution.completed);
        let end = solution.points.last().unwrap();
        assert_eq!(end.time, 0.125);
        let exact = (-end.time).exp() / (1.0 - 0.5 * (-2.0 * end.time).exp_m1()).sqrt();
        assert!((end.state - exact).abs() <= end.physical_error_bound);
        assert!(end.physical_error_bound <= plan.physical_tolerance());
        println!(
            "structured endpoint={:.15} abs_error={:.3e}",
            end.state,
            (end.state - exact).abs()
        );
    }
}
