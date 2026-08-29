use std::sync::Arc;

use rodas5p_core::{CoreError, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveFailureKind, AdaptiveStepConfig, OdeProblem, OutputSchedule, RadauConfig,
    RadauIiaStages, integrate_radau_adaptive_observed, radau_iia3_tableau, radau_step,
    scalar_linear_problem,
};

const RADAU3_EMBEDDED_ID: &str = "radau-iia3-scipy-1.17.0-embedded-order3";

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

#[test]
fn adaptive_radau1_is_transactional_and_responds_to_tolerance() {
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.2, 0.02).unwrap();
    let radau = RadauConfig {
        stages: RadauIiaStages::One,
        ..Default::default()
    };
    let loose = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        &radau,
        &adaptive_config(1.0e-8, 1.0e-4, 0.15, 0.15),
        &schedule,
    )
    .unwrap();
    let tight = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        &radau,
        &adaptive_config(1.0e-11, 1.0e-7, 0.15, 0.15),
        &schedule,
    )
    .unwrap();
    assert!(loose.observed.success && tight.observed.success);
    assert_eq!(loose.observed.t, schedule.times());
    assert_eq!(tight.observed.t, schedule.times());
    assert!(loose.diagnostics.rejected_macro_steps > 0);
    assert!(tight.diagnostics.accepted_macro_steps >= loose.diagnostics.accepted_macro_steps);
    assert_eq!(
        loose.observed.counters.accepted_steps as usize,
        2 * loose.diagnostics.accepted_macro_steps
    );
    assert!(
        loose
            .diagnostics
            .estimator_orders
            .iter()
            .all(|order| *order == 2)
    );
    let exact = problem.exact(0.2).unwrap()[0];
    let loose_error = (loose.observed.y.last().unwrap()[0] - exact).abs();
    let tight_error = (tight.observed.y.last().unwrap()[0] - exact).abs();
    assert!(
        tight_error < loose_error,
        "loose={loose_error:e}, tight={tight_error:e}"
    );
}

#[test]
fn adaptive_radau3_uses_fourth_power_embedded_estimator_and_one_step_per_acceptance() {
    let (problem, y0) = scalar_linear_problem(-5.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.4, 0.04).unwrap();
    let radau = RadauConfig {
        stages: RadauIiaStages::Three,
        ..Default::default()
    };
    let result = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 0.4),
        &y0,
        &radau,
        &adaptive_config(1.0e-11, 1.0e-8, 0.12, 0.12),
        &schedule,
    )
    .unwrap();
    assert!(result.observed.success);
    assert_eq!(result.observed.t, schedule.times());
    assert!(
        result
            .diagnostics
            .estimator_orders
            .iter()
            .all(|order| *order == 4)
    );
    assert!(
        result
            .diagnostics
            .estimator_ids
            .iter()
            .all(|id| id == RADAU3_EMBEDDED_ID)
    );
    assert_eq!(
        result.observed.counters.accepted_steps as usize,
        result.diagnostics.accepted_macro_steps
    );
    assert_eq!(
        result.observed.internal_steps,
        result.diagnostics.accepted_macro_steps
    );
}

#[test]
fn accepted_output_clipping_restores_the_preclip_radau_request() {
    let (problem, y0) = scalar_linear_problem(0.0, 1.0);
    let schedule = OutputSchedule::new(vec![0.0, 0.03, 1.0]).unwrap();
    let result = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 1.0),
        &y0,
        &RadauConfig::default(),
        &adaptive_config(1.0, 0.0, 0.1, 1.0),
        &schedule,
    )
    .unwrap();
    assert!(result.observed.success);
    assert_eq!(
        result.diagnostics.accepted_step_sizes[0].to_bits(),
        0.03_f64.to_bits()
    );
    assert_eq!(
        result.diagnostics.accepted_step_sizes[1].to_bits(),
        0.1_f64.to_bits()
    );
}

fn scalar_constant_mass_problem(mass: f64, lambda: f64) -> (OdeProblem, Vec<f64>) {
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = mass * lambda * y[0];
        Ok(())
    });
    let jacobian = Arc::new(move |_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![mass * lambda]));
    let exact = Arc::new(move |t: f64| vec![(lambda * t).exp()]);
    (
        OdeProblem::new(
            "scalar-constant-mass",
            1,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            Some(DenseMatrix::new(1, 1, vec![mass]).unwrap()),
            Some(exact),
        )
        .unwrap(),
        vec![1.0],
    )
}

fn expected_scalar_embedded_error(
    problem: &OdeProblem,
    y0: &[f64],
    h: f64,
    mass: f64,
    lambda: f64,
) -> f64 {
    let mut counters = WorkCounters::default();
    let report = radau_step(
        problem,
        0.0,
        y0,
        h,
        &RadauConfig {
            stages: RadauIiaStages::Three,
            ..Default::default()
        },
        &mut counters,
    )
    .unwrap();
    let (a, _, _) = radau_iia3_tableau();
    let z = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| a[(i, j)] * report.stage_increments[j][0])
                .sum::<f64>()
        })
        .collect::<Vec<_>>();
    let sqrt6 = 6.0_f64.sqrt();
    let e = [
        (-13.0 - 7.0 * sqrt6) / 3.0,
        (-13.0 + 7.0 * sqrt6) / 3.0,
        -1.0 / 3.0,
    ];
    let v = e.iter().zip(z).map(|(weight, zi)| weight * zi).sum::<f64>() / h;
    let mu = 3.0 + 3.0_f64.powf(2.0 / 3.0) - 3.0_f64.powf(1.0 / 3.0);
    let rhs = mass * lambda * y0[0] + mass * v;
    rhs / (mu / h * mass - mass * lambda)
}

#[test]
fn radau3_embedded_error_uses_stage_displacements_and_constant_mass_equivalently() {
    let h = 0.1;
    let lambda = -5.0;
    let (identity_problem, identity_y0) = scalar_linear_problem(lambda, 1.0);
    let (mass_problem, mass_y0) = scalar_constant_mass_problem(2.5, lambda);
    let schedule = OutputSchedule::new(vec![0.0, h]).unwrap();
    let adaptive = adaptive_config(1.0, 0.0, h, h);
    let radau = RadauConfig {
        stages: RadauIiaStages::Three,
        ..Default::default()
    };

    let identity = integrate_radau_adaptive_observed(
        &identity_problem,
        (0.0, h),
        &identity_y0,
        &radau,
        &adaptive,
        &schedule,
    )
    .unwrap();
    let mass = integrate_radau_adaptive_observed(
        &mass_problem,
        (0.0, h),
        &mass_y0,
        &radau,
        &adaptive,
        &schedule,
    )
    .unwrap();
    let expected_identity =
        expected_scalar_embedded_error(&identity_problem, &identity_y0, h, 1.0, lambda).abs();
    let expected_mass =
        expected_scalar_embedded_error(&mass_problem, &mass_y0, h, 2.5, lambda).abs();

    assert_eq!(identity.diagnostics.attempts, 1);
    assert_eq!(mass.diagnostics.attempts, 1);
    assert!((identity.diagnostics.error_norms[0] - expected_identity).abs() < 2.0e-14);
    assert!((mass.diagnostics.error_norms[0] - expected_mass).abs() < 2.0e-14);
    assert!(
        (identity.diagnostics.error_norms[0] - mass.diagnostics.error_norms[0]).abs() < 2.0e-14
    );
    assert_eq!(identity.observed.counters.direct_factorizations, 2);
    assert_eq!(mass.observed.counters.direct_factorizations, 2);
}

#[test]
fn radau3_previous_local_rejection_reuses_the_estimator_factor_for_correction() {
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let schedule = OutputSchedule::new(vec![0.0, 0.2]).unwrap();
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-12,
        rtol: 1.0e-10,
        initial_step: 0.2,
        min_step: 1.0e-12,
        max_step: 0.2,
        max_attempts: 2_000,
        min_factor: 0.9,
        reject_max_factor: 0.9,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        &RadauConfig {
            stages: RadauIiaStages::Three,
            ..Default::default()
        },
        &adaptive,
        &schedule,
    )
    .unwrap();

    assert!(result.observed.success);
    assert!(result.diagnostics.rejected_macro_steps > 1);
    assert_eq!(
        result.observed.counters.local_error_failures as usize,
        result.diagnostics.local_error_failures
    );
    assert_eq!(
        result.observed.counters.direct_factorizations,
        2 * result.diagnostics.attempts as u64
    );
    let estimator_solves = result
        .observed
        .counters
        .direct_solve_calls
        .checked_sub(result.observed.counters.nonlinear_iterations)
        .unwrap();
    assert!(estimator_solves > result.diagnostics.attempts as u64);
}

#[test]
fn radau3_nonfinite_residual_failure_is_typed_and_counted() {
    let rhs = Arc::new(|_t: f64, _y: &[f64], _out: &mut [f64]| {
        Err(CoreError::NonFinite(
            "intentional Radau residual failure".into(),
        ))
    });
    let jacobian = Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![-1.0]));
    let problem = OdeProblem::new(
        "radau-nonfinite-residual",
        1,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let adaptive = AdaptiveStepConfig {
        initial_step: 0.1,
        max_step: 0.1,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 0.1),
        &[1.0],
        &RadauConfig {
            stages: RadauIiaStages::Three,
            ..Default::default()
        },
        &adaptive,
        &OutputSchedule::new(vec![0.0, 0.1]).unwrap(),
    )
    .unwrap();

    assert!(!result.observed.success);
    assert_eq!(result.diagnostics.attempts, 1);
    assert_eq!(
        result.diagnostics.failure_kinds,
        vec![Some(AdaptiveFailureKind::NonFinite)]
    );
    assert_eq!(result.diagnostics.non_finite_failures, 1);
    assert_eq!(result.diagnostics.local_error_failures, 0);
    assert_eq!(result.observed.counters.nonlinear_solves, 1);
    assert_eq!(result.observed.counters.nonlinear_failures, 1);
    assert_eq!(result.observed.counters.rejected_steps, 1);
    assert_eq!(result.observed.counters.nonfinite_step_failures, 1);
    assert_eq!(result.observed.t, vec![0.0]);
    assert_eq!(result.observed.y, vec![vec![1.0]]);
}
