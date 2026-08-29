use std::sync::Arc;

use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, ControllerKind,
    IntegrationMethod, OdeProblem, OutputSchedule, RODAS5P_ESTIMATOR_ORDER, integrate_adaptive,
    integrate_adaptive_observed_with_config, integrate_sequential_matrix_free_adaptive_observed,
    rodas_next_step_after_attempt, scalar_linear_problem, step_doubling_wrms_error,
};

#[test]
fn adaptive_config_rejects_inconsistent_bounds() {
    let config = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-6,
        initial_step: 0.1,
        min_step: 0.2,
        max_step: 0.1,
        max_attempts: 10,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 5.0,
        reject_max_factor: 0.9,
        controller: ControllerKind::Integral,
    };
    assert!(config.validate().is_err());
}

#[test]
fn integral_controller_uses_estimator_order_and_rejection_cap() {
    let config = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-6,
        initial_step: 0.1,
        min_step: 1.0e-12,
        max_step: 1.0,
        max_attempts: 100,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 5.0,
        reject_max_factor: 0.8,
        controller: ControllerKind::Integral,
    };
    let state = AdaptiveControllerState::default();
    let accepted = state.propose_factor(&config, 0.25, 2, true).unwrap();
    let rejected = state.propose_factor(&config, 0.25, 2, false).unwrap();
    assert!((accepted - 1.8).abs() < 1.0e-14);
    assert!((rejected - 0.8).abs() < 1.0e-14);
}

#[test]
fn pi_controller_updates_only_after_acceptance() {
    let config = AdaptiveStepConfig {
        controller: ControllerKind::Pi,
        ..AdaptiveStepConfig::default()
    };
    let mut state = AdaptiveControllerState::default();
    assert_eq!(state.previous_accepted_error(), None);
    state.record_rejection(4.0).unwrap();
    assert_eq!(state.previous_accepted_error(), None);
    state.record_acceptance(0.25).unwrap();
    assert_eq!(state.previous_accepted_error(), Some(0.25));
    let factor = state.propose_factor(&config, 0.5, 5, true).unwrap();
    let expected = config.safety * 0.5_f64.powf(-0.7 / 5.0) * 0.25_f64.powf(0.4 / 5.0);
    assert!((factor - expected).abs() < 1.0e-14);
}

#[test]
fn forced_output_clipped_acceptance_is_neutral_to_pi_history() {
    let config = AdaptiveStepConfig {
        controller: ControllerKind::Pi,
        safety: 0.5,
        min_factor: 0.1,
        max_factor: 10.0,
        ..AdaptiveStepConfig::default()
    };
    let mut state = AdaptiveControllerState::default();

    let next =
        rodas_next_step_after_attempt(&mut state, &config, 0.4, 0.01, 0.01, true, true).unwrap();

    assert_eq!(next.to_bits(), 0.4_f64.to_bits());
    assert_eq!(state.previous_accepted_error(), None);
    let neutral_factor = config.safety * 0.5_f64.powf(-1.0 / RODAS5P_ESTIMATOR_ORDER as f64);
    assert!(
        (state
            .propose_factor(&config, 0.5, RODAS5P_ESTIMATOR_ORDER, true)
            .unwrap()
            - neutral_factor)
            .abs()
            < 1.0e-14
    );
}

#[test]
fn forced_output_clipped_rejection_scales_the_actual_trial_step() {
    let config = AdaptiveStepConfig {
        controller: ControllerKind::Integral,
        safety: 0.5,
        min_factor: 0.1,
        reject_max_factor: 0.8,
        ..AdaptiveStepConfig::default()
    };
    let mut state = AdaptiveControllerState::default();
    let next =
        rodas_next_step_after_attempt(&mut state, &config, 0.4, 0.01, 4.0, false, true).unwrap();
    let expected = 0.01 * config.safety * 4.0_f64.powf(-1.0 / RODAS5P_ESTIMATOR_ORDER as f64);
    assert!((next - expected).abs() < 1.0e-15);
}

fn matrix_free_failure_problem(kind: AdaptiveFailureKind) -> OdeProblem {
    OdeProblem::new(
        "typed-adaptive-failure",
        1,
        Arc::new(|_t, _y, out| {
            out[0] = 0.0;
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_t, _y, _v, _out| {
            Err(match kind {
                AdaptiveFailureKind::LinearSolve => CoreError::LinearSolve("injected".into()),
                AdaptiveFailureKind::NonlinearSolve => CoreError::NonlinearSolve("injected".into()),
                AdaptiveFailureKind::NonFinite => CoreError::NonFinite("injected".into()),
                AdaptiveFailureKind::LocalError => {
                    CoreError::InvalidInput("not a solver failure".into())
                }
            })
        })),
        Some(Arc::new(|_t, _y, out| {
            out[0] = 0.0;
            Ok(())
        })),
        false,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn adaptive_failure_diagnostics_align_typed_rows_and_cause_counts() {
    // The matrix-free failure is a real integration attempt; substitute each
    // CoreError text only after the recovery path has been selected below.
    let adaptive = AdaptiveStepConfig {
        initial_step: 0.1,
        min_step: 1.0e-12,
        max_step: 0.1,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    for kind in [
        AdaptiveFailureKind::LinearSolve,
        AdaptiveFailureKind::NonlinearSolve,
        AdaptiveFailureKind::NonFinite,
    ] {
        let result = integrate_sequential_matrix_free_adaptive_observed(
            &matrix_free_failure_problem(kind),
            (0.0, 0.1),
            &[1.0],
            &LinearSolverConfig {
                method: LinearMethod::Gmres,
                preconditioner: PreconditionerKind::None,
                ..LinearSolverConfig::default()
            },
            &adaptive,
            &OutputSchedule::new(vec![0.0, 0.1]).unwrap(),
        )
        .unwrap();

        assert_eq!(result.diagnostics.attempts, 1);
        assert_eq!(result.diagnostics.failure_kinds, vec![Some(kind)]);
        assert_eq!(result.diagnostics.local_error_failures, 0);
        assert_eq!(
            result.diagnostics.linear_solve_failures,
            usize::from(kind == AdaptiveFailureKind::LinearSolve)
        );
        assert_eq!(
            result.diagnostics.nonlinear_solve_failures,
            usize::from(kind == AdaptiveFailureKind::NonlinearSolve)
        );
        assert_eq!(
            result.diagnostics.non_finite_failures,
            usize::from(kind == AdaptiveFailureKind::NonFinite)
        );
        assert_eq!(
            result.diagnostics.error_norms.len(),
            result.diagnostics.failure_kinds.len()
        );
        assert_eq!(
            result.diagnostics.estimator_orders.len(),
            result.diagnostics.failure_kinds.len()
        );
        assert_eq!(
            result.diagnostics.estimator_ids.len(),
            result.diagnostics.failure_kinds.len()
        );
        assert_eq!(
            result.observed.counters.linear_solve_failures,
            u64::from(kind == AdaptiveFailureKind::LinearSolve)
        );
        assert_eq!(
            result.observed.counters.nonlinear_solve_failures,
            u64::from(kind == AdaptiveFailureKind::NonlinearSolve)
        );
        assert_eq!(
            result.observed.counters.nonfinite_step_failures,
            u64::from(kind == AdaptiveFailureKind::NonFinite)
        );
        assert_eq!(result.observed.counters.local_error_failures, 0);
    }
}

#[test]
fn unobserved_adaptive_path_preserves_typed_solver_failure_work() {
    for kind in [
        AdaptiveFailureKind::LinearSolve,
        AdaptiveFailureKind::NonlinearSolve,
        AdaptiveFailureKind::NonFinite,
    ] {
        let result = integrate_adaptive(
            &matrix_free_failure_problem(kind),
            (0.0, 0.1),
            &[1.0],
            0.1,
            IntegrationMethod::Sequential,
            Some(&LinearSolverConfig {
                method: LinearMethod::Gmres,
                preconditioner: PreconditionerKind::None,
                ..LinearSolverConfig::default()
            }),
            None,
            1.0e-9,
            1.0e-6,
            1,
            0.1,
        )
        .unwrap();

        assert!(!result.success);
        assert_eq!(
            result.counters.linear_solve_failures,
            u64::from(kind == AdaptiveFailureKind::LinearSolve)
        );
        assert_eq!(
            result.counters.nonlinear_solve_failures,
            u64::from(kind == AdaptiveFailureKind::NonlinearSolve)
        );
        assert_eq!(
            result.counters.nonfinite_step_failures,
            u64::from(kind == AdaptiveFailureKind::NonFinite)
        );
        assert_eq!(result.counters.local_error_failures, 0);
    }
}

#[test]
fn ordinary_error_rejection_is_recorded_as_a_local_error() {
    let (problem, y0) = scalar_linear_problem(-100.0, 1.0);
    let config = AdaptiveStepConfig {
        atol: 1.0e-12,
        rtol: 1.0e-8,
        initial_step: 0.15,
        min_step: 1.0e-12,
        max_step: 0.15,
        max_attempts: 10_000,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_adaptive_observed_with_config(
        &problem,
        (0.0, 0.2),
        &y0,
        IntegrationMethod::Sequential,
        Some(&LinearSolverConfig::default()),
        None,
        &config,
        &OutputSchedule::uniform(0.0, 0.2, 0.04).unwrap(),
    )
    .unwrap();
    assert!(result.diagnostics.local_error_failures > 0);
    assert_eq!(
        result.observed.counters.local_error_failures,
        result.diagnostics.local_error_failures as u64
    );
    assert!(
        result
            .diagnostics
            .failure_kinds
            .contains(&Some(AdaptiveFailureKind::LocalError))
    );
}

#[test]
fn step_doubling_returns_fine_path_wrms_error() {
    let estimate = step_doubling_wrms_error(&[0.0], &[0.8], &[1.0], 1.0, 0.0, 1).unwrap();
    assert_eq!(estimate.method_order, 1);
    assert_eq!(estimate.estimator_order, 2);
    assert!((estimate.error_vector[0] - 0.2).abs() < 1.0e-14);
    assert!((estimate.error_norm - 0.2).abs() < 1.0e-14);
}
