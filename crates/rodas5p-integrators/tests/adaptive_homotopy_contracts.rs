use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, HomotopyPathConfig, HomotopyPredictor, HomotopyStepConfig, OutputSchedule,
    integrate_homotopy_adaptive_observed, scalar_linear_problem,
};

fn adaptive_config(atol: f64, rtol: f64, initial_step: f64, max_step: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol,
        rtol,
        initial_step,
        min_step: 1.0e-12,
        max_step,
        max_attempts: 20_000,
        ..AdaptiveStepConfig::default()
    }
}

fn direct_fallback() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    }
}

#[test]
fn adaptive_homotopy_accepts_an_affine_fast_path_on_the_requested_grid() {
    let (problem, y0) = scalar_linear_problem(-5.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.2, 0.02).unwrap();
    let path = HomotopyPathConfig::new(0.4, 7, 2, HomotopyPredictor::Euler, 1).unwrap();
    let homotopy = HomotopyStepConfig::new(path, 0.1).unwrap();
    let result = integrate_homotopy_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        &homotopy,
        Some(&direct_fallback()),
        &adaptive_config(1.0e-10, 1.0e-7, 0.05, 0.05),
        &schedule,
    )
    .unwrap();
    assert!(result.observed.success);
    assert_eq!(result.observed.t, schedule.times());
    assert_eq!(result.diagnostics.fallback_steps, 0);
    assert!(result.observed.counters.fast_accepts > 0);
    assert!(
        result
            .diagnostics
            .estimator_ids
            .iter()
            .all(|id| id == "homotopy-native-rodas-endpoint")
    );
}

#[test]
fn adaptive_homotopy_fallback_is_transactional_across_rejections() {
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.2, 0.02).unwrap();
    let path = HomotopyPathConfig::new(0.0, 0, 1, HomotopyPredictor::Euler, 0).unwrap();
    let homotopy = HomotopyStepConfig::new(path, 0.0).unwrap();
    let fallback = LinearSolverConfig {
        method: LinearMethod::Lgmres,
        ..LinearSolverConfig::default()
    };
    let result = integrate_homotopy_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        &homotopy,
        Some(&fallback),
        &adaptive_config(1.0e-11, 1.0e-7, 0.15, 0.15),
        &schedule,
    )
    .unwrap();
    assert!(result.observed.success);
    assert_eq!(result.observed.t, schedule.times());
    assert!(result.diagnostics.fallback_steps > 0);
    assert!(result.diagnostics.rejected_macro_steps > 0);
    assert_eq!(
        result.observed.counters.accepted_steps as usize,
        result.diagnostics.accepted_macro_steps
    );
    let exact = problem.exact(0.2).unwrap()[0];
    let error = (result.observed.y.last().unwrap()[0] - exact).abs();
    assert!(error < 2.0e-7, "endpoint error={error:e}");
}
