use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, DenseOutputError, HomotopyPathConfig, HomotopyPredictor,
    HomotopyStepConfig, IntegrationMethod, OutputSamplingPlan, OutputSchedule, SabrConfig,
    StageHistory, homotopy_step, integrate_adaptive, integrate_adaptive_dense_observed_with_config,
    integrate_adaptive_observed_with_config, integrate_fixed, manufactured_vector_problem,
    sabr_step, sequential_step,
};

fn invalid_configs() -> [LinearSolverConfig; 3] {
    [
        LinearSolverConfig {
            method: LinearMethod::Direct,
            ..Default::default()
        },
        LinearSolverConfig {
            method: LinearMethod::Gmres,
            preconditioner: PreconditionerKind::Direct,
            ..Default::default()
        },
        LinearSolverConfig {
            method: LinearMethod::Gmres,
            preconditioner: PreconditionerKind::Jacobi,
            ..Default::default()
        },
    ]
}

#[test]
fn jvp_only_rejects_direct_and_explicit_preconditioners_upfront_with_zero_evals() {
    let (base_problem, y0) = manufactured_vector_problem(4, 100.0, 10.0, 0.2, 0.0).unwrap();
    let problem = base_problem.jvp_only_clone().unwrap();
    assert!(!problem.has_explicit_jacobian());
    assert!(problem.has_jvp());

    for cfg in invalid_configs() {
        // 1. sequential_step
        {
            let mut counters = WorkCounters::default();
            let res = sequential_step(
                &problem,
                0.0,
                &y0,
                0.01,
                &cfg,
                None,
                1e-4,
                1e-4,
                false,
                &mut counters,
            );
            assert!(
                matches!(res, Err(CoreError::InvalidInput(_))),
                "expected CoreError::InvalidInput for sequential_step with {:?}",
                cfg
            );
            assert_eq!(counters.rhs_calls, 0);
            assert_eq!(counters.rhs_evaluations, 0);
            assert_eq!(counters.jvp_calls, 0);
            assert_eq!(counters.rejected_steps, 0);
        }

        // 2. integrate_fixed
        {
            let res = integrate_fixed(
                &problem,
                (0.0, 0.1),
                &y0,
                0.01,
                IntegrationMethod::Sequential,
                Some(&cfg),
                None,
                1e-4,
                1e-4,
            );
            assert!(
                matches!(res, Err(CoreError::InvalidInput(_))),
                "expected CoreError::InvalidInput for integrate_fixed with {:?}",
                cfg
            );
        }

        // 3. integrate_adaptive
        {
            let res = integrate_adaptive(
                &problem,
                (0.0, 0.1),
                &y0,
                0.01,
                IntegrationMethod::Sequential,
                Some(&cfg),
                None,
                1e-4,
                1e-4,
                100,
                0.05,
            );
            assert!(
                matches!(res, Err(CoreError::InvalidInput(_))),
                "expected CoreError::InvalidInput for integrate_adaptive with {:?}",
                cfg
            );
        }

        // 4. integrate_adaptive_observed_with_config
        {
            let adaptive = AdaptiveStepConfig::legacy_rodas(1e-4, 1e-4, 0.01, 100, 0.05).unwrap();
            let schedule = OutputSchedule::uniform(0.0, 0.1, 0.05).unwrap();
            let res = integrate_adaptive_observed_with_config(
                &problem,
                (0.0, 0.1),
                &y0,
                IntegrationMethod::Sequential,
                Some(&cfg),
                None,
                &adaptive,
                &schedule,
            );
            assert!(
                matches!(res, Err(CoreError::InvalidInput(_))),
                "expected CoreError::InvalidInput for integrate_adaptive_observed_with_config with {:?}",
                cfg
            );
        }

        // 5. integrate_adaptive_dense_observed_with_config
        {
            let adaptive = AdaptiveStepConfig::legacy_rodas(1e-4, 1e-4, 0.01, 100, 0.05).unwrap();
            let schedule = OutputSchedule::uniform(0.0, 0.1, 0.05).unwrap();
            let sampling = OutputSamplingPlan::dense(schedule);
            let res = integrate_adaptive_dense_observed_with_config(
                &problem,
                (0.0, 0.1),
                &y0,
                IntegrationMethod::Sequential,
                Some(&cfg),
                None,
                &adaptive,
                &sampling,
            );
            assert!(
                matches!(res, Err(DenseOutputError::Core(CoreError::InvalidInput(_)))),
                "expected DenseOutputError::Core(CoreError::InvalidInput(_)) for integrate_adaptive_dense_observed_with_config with {:?}",
                cfg
            );
        }

        // 6. sabr_step (with Some(&direct_cfg))
        {
            let sabr_cfg = SabrConfig::default();
            let mut history = StageHistory::default();
            let mut counters = WorkCounters::default();
            let res = sabr_step(
                &problem,
                0.0,
                &y0,
                0.01,
                &sabr_cfg,
                Some(&cfg),
                &mut history,
                None,
                1e-4,
                1e-4,
                false,
                &mut counters,
            );
            assert!(
                matches!(res, Err(CoreError::InvalidInput(_))),
                "expected CoreError::InvalidInput for sabr_step with {:?}",
                cfg
            );
            assert_eq!(counters.rhs_calls, 0);
            assert_eq!(counters.rhs_evaluations, 0);
            assert_eq!(counters.jvp_calls, 0);
            assert_eq!(counters.rejected_steps, 0);
        }

        // 7. homotopy_step (with Some(&direct_cfg))
        {
            let path = HomotopyPathConfig::new(0.4, 4, 2, HomotopyPredictor::Euler, 1).unwrap();
            let homotopy_cfg = HomotopyStepConfig::new(path, 0.1).unwrap();
            let mut counters = WorkCounters::default();
            let res = homotopy_step(
                &problem,
                0.0,
                &y0,
                0.01,
                &homotopy_cfg,
                Some(&cfg),
                None,
                1e-4,
                1e-4,
                false,
                &mut counters,
            );
            assert!(
                matches!(res, Err(CoreError::InvalidInput(_))),
                "expected CoreError::InvalidInput for homotopy_step with {:?}",
                cfg
            );
            assert_eq!(counters.rhs_calls, 0);
            assert_eq!(counters.rhs_evaluations, 0);
            assert_eq!(counters.jvp_calls, 0);
            assert_eq!(counters.rejected_steps, 0);
        }
    }
}

#[test]
fn jvp_only_positive_control_with_gmres_none_succeeds() {
    let valid_cfg = LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        ..Default::default()
    };
    let (base_problem, y0) = manufactured_vector_problem(4, 100.0, 10.0, 0.2, 0.0).unwrap();
    let problem = base_problem.jvp_only_clone().unwrap();

    let mut counters = WorkCounters::default();
    let res = sequential_step(
        &problem,
        0.0,
        &y0,
        0.01,
        &valid_cfg,
        None,
        1e-4,
        1e-4,
        false,
        &mut counters,
    );
    assert!(
        res.is_ok(),
        "positive control sequential_step failed: {:?}",
        res
    );
    assert!(counters.rhs_calls > 0);
    assert!(counters.jvp_calls > 0);

    let res_fixed = integrate_fixed(
        &problem,
        (0.0, 0.05),
        &y0,
        0.01,
        IntegrationMethod::Sequential,
        Some(&valid_cfg),
        None,
        1e-4,
        1e-4,
    );
    assert!(
        res_fixed.is_ok(),
        "positive control integrate_fixed failed: {:?}",
        res_fixed
    );

    let res_adapt = integrate_adaptive(
        &problem,
        (0.0, 0.05),
        &y0,
        0.01,
        IntegrationMethod::Sequential,
        Some(&valid_cfg),
        None,
        1e-4,
        1e-4,
        100,
        0.05,
    );
    assert!(
        res_adapt.is_ok(),
        "positive control integrate_adaptive failed: {:?}",
        res_adapt
    );
}
