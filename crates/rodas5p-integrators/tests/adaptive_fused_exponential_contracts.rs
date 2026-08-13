use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerKind, FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem,
    OutputSchedule, ParallelExecution, integrate_pexprb54s4_fused_adaptive_observed,
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

fn phi_config() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 12,
        dimension_increment: 1,
        relative_tolerance: 1e-10,
        absolute_tolerance: 1e-13,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 8,
    }
}

#[test]
fn adaptive_fused_parallel_exponential_responds_to_tolerance() {
    let problem = square_problem();
    let output = OutputSchedule::new(vec![0.0, 0.25]).unwrap();
    let run = |rtol| {
        let cfg = AdaptiveStepConfig {
            atol: rtol * 0.01,
            rtol,
            initial_step: 0.1,
            min_step: 1e-12,
            max_step: 0.25,
            max_attempts: 10000,
            safety: 0.9,
            min_factor: 0.2,
            max_factor: 4.0,
            reject_max_factor: 0.8,
            controller: ControllerKind::Pi,
        };
        integrate_pexprb54s4_fused_adaptive_observed(
            &problem,
            (0.0, 0.25),
            &[1.0],
            &cfg,
            &output,
            phi_config(),
            &ParallelExecution::sequential(),
        )
        .unwrap()
    };
    let loose = run(1e-4);
    let tight = run(1e-8);
    assert!(loose.observed.success && tight.observed.success);
    let exact = 1.0 / (1.0 - 0.25);
    let loose_error = (loose.observed.y.last().unwrap()[0] - exact).abs();
    let tight_error = (tight.observed.y.last().unwrap()[0] - exact).abs();
    assert!(
        tight_error <= loose_error * 1.1,
        "loose={loose_error:e} tight={tight_error:e}"
    );
    assert_eq!(tight.observed.counters.jacobian_builds, 0);
    assert_eq!(tight.observed.counters.direct_factorizations, 0);
    assert_eq!(tight.observed.counters.nonlinear_iterations, 0);
}

#[test]
fn nonautonomous_time_augmentation_has_exact_rhs_and_jvp() {
    let problem = OdeProblem::new(
        "nonautonomous",
        1,
        Arc::new(|t, y: &[f64], out: &mut [f64]| {
            out[0] = t * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|t, _y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = t * v[0];
            Ok(())
        })),
        Some(Arc::new(|_t, y: &[f64], out: &mut [f64]| {
            out[0] = y[0];
            Ok(())
        })),
        false,
        None,
        None,
    )
    .unwrap();
    let augmented = problem.time_augmented_clone().unwrap();
    assert!(augmented.autonomous);
    assert_eq!(augmented.dimension, 2);
    let mut counters = Default::default();
    let rhs = augmented
        .eval_rhs(99.0, &[2.0, 0.5], &mut counters)
        .unwrap();
    assert_eq!(rhs, vec![1.0, 1.0]);
    let op = augmented.linearize_matrix_free(0.0, &[2.0, 0.5]).unwrap();
    let mut out = vec![0.0; 2];
    op.apply(&[3.0, 4.0], &mut out).unwrap();
    assert!((out[0] - 9.5).abs() < 1e-14 && out[1] == 0.0);
}

#[test]
fn incomplete_adaptive_run_preserves_attempted_work_and_partial_output() {
    let problem = square_problem();
    let output = OutputSchedule::new(vec![0.0, 0.25]).unwrap();
    let cfg = AdaptiveStepConfig {
        atol: 1e-14,
        rtol: 1e-12,
        initial_step: 0.25,
        min_step: 1e-16,
        max_step: 0.25,
        max_attempts: 1,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 4.0,
        reject_max_factor: 0.8,
        controller: ControllerKind::Pi,
    };
    let run = integrate_pexprb54s4_fused_adaptive_observed(
        &problem,
        (0.0, 0.25),
        &[1.0],
        &cfg,
        &output,
        phi_config(),
        &ParallelExecution::sequential(),
    )
    .unwrap();
    assert!(!run.observed.success);
    assert_eq!(run.observed.t, vec![0.0]);
    assert_eq!(run.observed.y, vec![vec![1.0]]);
    assert!(run.observed.counters.rhs_evaluations > 0);
    assert!(run.observed.counters.jvp_vectors > 0);
    assert_eq!(run.diagnostics.attempts, 1);
}
