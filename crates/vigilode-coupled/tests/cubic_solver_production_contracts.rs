use vigilode_coupled::rhs_homotopy::{
    CubicDecayAccuracyPlan, CubicStageSolver, RhsAcceleration,
    integrate_cubic_decay_accuracy_with_solver as integrate,
};

fn plan(k: f64, g: f64, y: f64, t: f64, tol: f64, cap: u64) -> CubicDecayAccuracyPlan {
    CubicDecayAccuracyPlan::new(k, g, y, t, tol, 65536, cap).unwrap()
}

fn exact(k: f64, g: f64, y: f64, t: f64) -> f64 {
    y * (-k * t).exp() / (1.0 + (g / k) * y * y * (-(-2.0 * k * t).exp_m1())).sqrt()
}

#[test]
fn scalar_specialization_preserves_every_generic_state_bit_and_work_count() {
    for (k, g, y, t, tol) in [
        (1.0, 0.5, 1.0, 1.0, 1e-3),
        (1.0, 0.5, 1.0, 1.0, 1e-5),
        (1.0, 0.0, 1.0, 1.0, 1e-5),
        (1.0, 20.0, 1.0, 0.1, 1e-4),
        (2.0, 0.25, 0.0, 0.5, 1e-5),
    ] {
        let p = plan(k, g, y, t, tol, 1_000_000);
        for mode in [
            RhsAcceleration::Picard,
            RhsAcceleration::ResidualDominance,
            RhsAcceleration::Screened,
            RhsAcceleration::SlowOnly,
        ] {
            for warm in [false, true] {
                let baseline = integrate(&p, CubicStageSolver::GenericHomotopy(mode), warm);
                let scalar = integrate(&p, CubicStageSolver::ScalarHomotopy(mode), warm);
                assert!(
                    baseline.completed && scalar.completed,
                    "{:?} {:?}",
                    baseline.failure,
                    scalar.failure
                );
                assert_eq!(baseline.map_evaluations, scalar.map_evaluations);
                assert_eq!(
                    baseline.interval_polynomial_evaluations,
                    scalar.interval_polynomial_evaluations
                );
                assert_eq!(baseline.extrapolation_trials, scalar.extrapolation_trials);
                assert_eq!(
                    baseline.extrapolations_accepted,
                    scalar.extrapolations_accepted
                );
                assert_eq!(baseline.points.len(), scalar.points.len());
                for (a, b) in baseline.points.iter().zip(&scalar.points) {
                    assert_eq!(a.state.to_bits(), b.state.to_bits());
                    assert_eq!(
                        a.physical_error_bound.to_bits(),
                        b.physical_error_bound.to_bits()
                    );
                    assert_eq!(
                        a.algebraic_step_error_bound.to_bits(),
                        b.algebraic_step_error_bound.to_bits()
                    );
                    assert_eq!(a.exact_time_interval, b.exact_time_interval);
                }
            }
        }
    }
}

#[test]
fn algebraic_inverse_passes_original_outward_certificate_for_nonlinear_models() {
    for (k, g, y, t, tol) in [
        (1.0, 0.5, 1.0, 1.0, 1e-3),
        (1.0, 0.5, 1.0, 1.0, 1e-5),
        (1.0, 20.0, 1.0, 0.1, 1e-4),
        (50.0, 2.0, 0.5, 0.1, 1e-4),
    ] {
        let p = plan(k, g, y, t, tol, 1_000_000);
        let result = integrate(&p, CubicStageSolver::AlgebraicPredictor, true);
        assert!(result.completed, "{:?}", result.failure);
        assert_eq!(result.algebraic_predictors, p.steps() as u64);
        assert_eq!(result.algebraic_predictor_fallbacks, 0);
        assert_eq!(result.map_evaluations, p.steps() as u64);
        assert_eq!(result.interval_polynomial_evaluations, p.steps() as u64);
        for point in result.points {
            assert!((point.state - exact(k, g, y, point.time)).abs() <= point.physical_error_bound);
            assert!(point.physical_error_bound <= tol);
        }
    }
}

#[test]
fn algebraic_inverse_handles_linear_and_zero_limits_without_tuned_threshold() {
    for (g, y) in [(0.0, 1.0), (0.5, 0.0), (1e-200, 1.0)] {
        let p = plan(1.0, g, y, 1.0, 1e-5, 100000);
        let result = integrate(&p, CubicStageSolver::AlgebraicPredictor, false);
        assert!(result.completed, "{:?}", result.failure);
        for point in result.points {
            assert!(
                (point.state - exact(1.0, g, y, point.time)).abs() <= point.physical_error_bound
            );
        }
    }
}

#[test]
fn every_solver_preserves_partial_prefix_and_resource_charge_on_exhaustion() {
    for cap in [1, 2, 3, 4, 9] {
        let p = plan(1.0, 0.5, 1.0, 1.0, 1e-5, cap);
        for solver in [
            CubicStageSolver::GenericHomotopy(RhsAcceleration::Picard),
            CubicStageSolver::ScalarHomotopy(RhsAcceleration::Picard),
            CubicStageSolver::AlgebraicPredictor,
            CubicStageSolver::RationalPredictor,
        ] {
            let result = integrate(&p, solver, true);
            assert!(!result.completed);
            assert!(result.failure.is_some());
            assert_eq!(
                result.map_evaluations + result.interval_polynomial_evaluations,
                cap
            );
            assert!(!result.points.is_empty());
            for point in result.points {
                assert!(point.physical_error_bound <= p.physical_tolerance());
            }
        }
    }
}

#[test]
fn analytic_predictor_cannot_bypass_rounding_floor_or_fabricate_an_accepted_step() {
    let ordinary = plan(1.0, 0.0, 1.0, 1.0, 1e-3, 100000);
    let p = plan(
        1.0,
        0.0,
        1.0,
        1.0,
        ordinary.truncation_bound().next_up(),
        100000,
    );
    let result = integrate(&p, CubicStageSolver::AlgebraicPredictor, true);
    assert!(!result.completed);
    assert_eq!(result.points.len(), 1);
    assert_eq!(result.algebraic_predictors, 1);
    assert_eq!(result.algebraic_predictor_fallbacks, 1);
    assert!(result.polish_evaluations > 0);
    assert!(result.interval_polynomial_evaluations >= 2);
}

#[test]
fn rational_predictor_has_identical_certification_and_charged_repair_contract() {
    for (k, g, y, t, tol) in [
        (1.0, 0.5, 1.0, 1.0, 1e-3),
        (1.0, 0.5, 1.0, 1.0, 1e-5),
        (1.0, 20.0, 1.0, 0.1, 1e-4),
        (1.0, 0.0, 1.0, 1.0, 1e-5),
        (1.0, 0.5, 0.0, 1.0, 1e-5),
    ] {
        let p = plan(k, g, y, t, tol, 1_000_000);
        let result = integrate(&p, CubicStageSolver::RationalPredictor, true);
        assert!(result.completed, "{:?}", result.failure);
        assert_eq!(result.rational_predictors, p.steps() as u64);
        assert_eq!(result.algebraic_predictors, 0);
        assert_eq!(result.algebraic_predictor_fallbacks, 0);
        assert_eq!(result.map_evaluations, p.steps() as u64);
        assert_eq!(result.interval_polynomial_evaluations, p.steps() as u64);
        for point in result.points {
            assert!((point.state - exact(k, g, y, point.time)).abs() <= point.physical_error_bound);
            assert!(point.physical_error_bound <= tol);
        }
    }
    let ordinary = plan(1.0, 0.0, 1.0, 1.0, 1e-3, 100000);
    let p = plan(
        1.0,
        0.0,
        1.0,
        1.0,
        ordinary.truncation_bound().next_up(),
        100000,
    );
    let result = integrate(&p, CubicStageSolver::RationalPredictor, true);
    assert!(!result.completed);
    assert_eq!(result.points.len(), 1);
    assert_eq!(result.rational_predictors, 1);
    assert_eq!(result.algebraic_predictor_fallbacks, 1);
    assert!(result.polish_evaluations > 0);
}
