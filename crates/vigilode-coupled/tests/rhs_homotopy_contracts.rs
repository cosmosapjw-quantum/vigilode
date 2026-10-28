use std::cell::Cell;
use vigilode_coupled::rhs_homotopy::{
    CubicDecayAccuracyPlan, RhsAcceleration, RhsHomotopyConfig, RhsHomotopyFailureKind, RhsOnlyMap,
    integrate_cubic_decay_accuracy, solve_rhs_homotopy,
};

struct Affine {
    a: f64,
    b: f64,
    calls: Cell<u64>,
}
impl Affine {
    fn new(a: f64, b: f64) -> Self {
        Self {
            a,
            b,
            calls: Cell::new(0),
        }
    }
}
impl RhsOnlyMap for Affine {
    fn dimension(&self) -> usize {
        1
    }
    fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
        self.calls.set(self.calls.get() + 1);
        out[0] = self.a * x[0] + self.b;
        Ok(())
    }
}

fn cfg(mode: RhsAcceleration) -> RhsHomotopyConfig {
    RhsHomotopyConfig {
        acceleration: mode,
        residual_l2_tolerance: 1e-12,
        ..RhsHomotopyConfig::default()
    }
}

#[test]
fn affine_target_uses_values_only_and_recovers_known_root() {
    let slow = Affine::new(0.99, 0.01);
    let plain = solve_rhs_homotopy(&slow, &[0.0], &cfg(RhsAcceleration::Picard));
    assert!(plain.failure.is_none(), "{:?}", plain.failure);
    let fast_map = Affine::new(0.99, 0.01);
    let accelerated = solve_rhs_homotopy(&fast_map, &[0.0], &cfg(RhsAcceleration::Screened));
    assert!(accelerated.failure.is_none(), "{:?}", accelerated.failure);
    assert!((accelerated.root.as_ref().unwrap()[0] - 1.0).abs() < 1e-11);
    assert!(accelerated.work.map_attempted < 8);
    assert!(plain.work.map_attempted > 1000);
    assert_eq!(fast_map.calls.get(), accelerated.work.map_attempted);
    assert_eq!(slow.calls.get(), plain.work.map_attempted);
}

#[test]
fn every_homotopy_station_solves_its_own_original_map() {
    let map = Affine::new(0.9, 0.1);
    let config = RhsHomotopyConfig {
        lambdas: vec![0.25, 0.5, 1.0],
        ..cfg(RhsAcceleration::Screened)
    };
    let result = solve_rhs_homotopy(&map, &[0.0], &config);
    assert!(result.failure.is_none(), "{:?}", result.failure);
    assert_eq!(result.points.len(), 3);
    for p in &result.points {
        let exact = p.lambda * 0.1 / (1.0 - p.lambda * 0.9);
        assert!((p.state[0] - exact).abs() < 1e-10);
        assert!(p.residual_l2 <= 1e-12);
    }
    assert_eq!(
        result.work.map_attempted,
        result.points.iter().map(|p| p.map_evaluations).sum::<u64>()
    );
}

#[test]
fn slow_only_avoids_extra_calls_on_a_fast_contraction() {
    let plain = solve_rhs_homotopy(
        &Affine::new(0.1, 0.9),
        &[0.0],
        &cfg(RhsAcceleration::Picard),
    );
    let adaptive = solve_rhs_homotopy(
        &Affine::new(0.1, 0.9),
        &[0.0],
        &cfg(RhsAcceleration::SlowOnly),
    );
    assert_eq!(plain.root, adaptive.root);
    assert_eq!(plain.work.map_attempted, adaptive.work.map_attempted);
    assert_eq!(adaptive.work.extrapolation_trials, 0);
    assert!(adaptive.work.policy_skips > 0);
}

#[test]
fn implicit_midpoint_nonlinear_rk_root_matches_monotone_bisection() {
    struct Midpoint;
    impl RhsOnlyMap for Midpoint {
        fn dimension(&self) -> usize {
            1
        }
        fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
            let midpoint = 1.0 + 0.5 * x[0];
            out[0] = -0.5 * (midpoint + 0.5 * midpoint.powi(3));
            Ok(())
        }
    }
    let result = solve_rhs_homotopy(&Midpoint, &[0.0], &cfg(RhsAcceleration::Screened));
    assert!(result.failure.is_none(), "{:?}", result.failure);
    let (mut lo, mut hi) = (-2.0_f64, 0.0_f64);
    for _ in 0..80 {
        let k = lo + 0.5 * (hi - lo);
        let ymid = 1.0 + 0.5 * k;
        if k + 0.5 * (ymid + 0.5 * ymid.powi(3)) > 0.0 {
            hi = k;
        } else {
            lo = k;
        }
    }
    assert!((result.root.unwrap()[0] - (lo + 0.5 * (hi - lo))).abs() < 2e-12);
}

#[test]
fn budget_is_global_and_charged_before_dispatch() {
    let map = Affine::new(0.99, 0.01);
    let config = RhsHomotopyConfig {
        max_map_evaluations: 2,
        ..cfg(RhsAcceleration::Screened)
    };
    let result = solve_rhs_homotopy(&map, &[0.0], &config);
    assert_eq!(result.failure.unwrap().kind, RhsHomotopyFailureKind::Budget);
    assert_eq!(map.calls.get(), 2);
    assert_eq!(result.work.map_attempted, 2);
    assert!(result.root.is_none());
}

#[test]
fn partial_write_is_rejected_and_attempted_work_survives() {
    struct Partial;
    impl RhsOnlyMap for Partial {
        fn dimension(&self) -> usize {
            2
        }
        fn evaluate(&self, _: &[f64], out: &mut [f64]) -> Result<(), String> {
            out[0] = 1.0;
            Ok(())
        }
    }
    let result = solve_rhs_homotopy(&Partial, &[0.0, 0.0], &cfg(RhsAcceleration::Picard));
    assert_eq!(
        result.failure.unwrap().kind,
        RhsHomotopyFailureKind::NonFinite
    );
    assert_eq!(result.work.map_attempted, 1);
    assert_eq!(result.work.map_completed, 0);
}

#[test]
fn callback_failure_at_speculative_trial_is_fatal() {
    struct FailsAtRoot {
        calls: Cell<u64>,
    }
    impl RhsOnlyMap for FailsAtRoot {
        fn dimension(&self) -> usize {
            1
        }
        fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
            self.calls.set(self.calls.get() + 1);
            if x[0] > 0.9 {
                return Err("fatal trial".into());
            }
            out[0] = 0.99 * x[0] + 0.01;
            Ok(())
        }
    }
    let map = FailsAtRoot {
        calls: Cell::new(0),
    };
    let result = solve_rhs_homotopy(&map, &[0.0], &cfg(RhsAcceleration::Screened));
    assert_eq!(
        result.failure.unwrap().kind,
        RhsHomotopyFailureKind::Callback
    );
    assert_eq!(result.work.map_attempted, map.calls.get());
    assert_eq!(result.work.map_attempted, result.work.map_completed + 1);
    assert_eq!(result.work.extrapolation_trials, 1);
    assert!(result.root.is_none());
}

#[test]
fn inadmissible_anchor_never_enters_callback() {
    struct Domain;
    impl RhsOnlyMap for Domain {
        fn dimension(&self) -> usize {
            1
        }
        fn admits(&self, x: &[f64]) -> bool {
            x[0] >= 0.0
        }
        fn evaluate(&self, _: &[f64], _: &mut [f64]) -> Result<(), String> {
            panic!("must not enter")
        }
    }
    let result = solve_rhs_homotopy(&Domain, &[-1.0], &cfg(RhsAcceleration::Picard));
    assert_eq!(result.failure.unwrap().kind, RhsHomotopyFailureKind::Domain);
    assert_eq!(result.work.map_attempted, 0);
}

#[test]
fn model_change_during_callback_prevents_success() {
    struct Changing {
        epoch: Cell<u64>,
    }
    impl RhsOnlyMap for Changing {
        fn dimension(&self) -> usize {
            1
        }
        fn model_epoch(&self) -> Option<u64> {
            Some(self.epoch.get())
        }
        fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
            out[0] = x[0];
            self.epoch.set(self.epoch.get() + 1);
            Ok(())
        }
    }
    let result = solve_rhs_homotopy(
        &Changing {
            epoch: Cell::new(0),
        },
        &[0.0],
        &cfg(RhsAcceleration::Picard),
    );
    assert_eq!(
        result.failure.unwrap().kind,
        RhsHomotopyFailureKind::ModelChanged
    );
    assert_eq!(result.work.map_attempted, 1);
    assert!(result.root.is_none());
}

#[test]
fn degenerate_residual_differences_do_not_dispatch_nan_candidates() {
    let config = RhsHomotopyConfig {
        max_iterations_per_lambda: 4,
        ..cfg(RhsAcceleration::Screened)
    };
    let result = solve_rhs_homotopy(&Affine::new(1.0, 1.0), &[0.0], &config);
    assert_eq!(
        result.failure.unwrap().kind,
        RhsHomotopyFailureKind::IterationLimit
    );
    assert_eq!(result.work.extrapolation_trials, 0);
    assert_eq!(result.work.coefficient_skips, 3);
    assert_eq!(result.work.map_attempted, 5);
}

#[test]
fn invalid_station_order_is_refused_before_callback() {
    let map = Affine::new(0.1, 1.0);
    let config = RhsHomotopyConfig {
        lambdas: vec![0.5, 0.5, 1.0],
        ..cfg(RhsAcceleration::Picard)
    };
    let result = solve_rhs_homotopy(&map, &[0.0], &config);
    assert_eq!(result.failure.unwrap().kind, RhsHomotopyFailureKind::Input);
    assert_eq!(map.calls.get(), 0);
}

#[test]
fn residual_dominance_has_no_hidden_heuristic_margin_or_coefficient_cap() {
    let config = RhsHomotopyConfig {
        acceleration: RhsAcceleration::ResidualDominance,
        min_residual_ratio: f64::NAN,
        improvement_factor: 0.0,
        max_coefficient: 0.0,
        max_rejected_trials_per_lambda: 0,
        ..cfg(RhsAcceleration::ResidualDominance)
    };
    let result = solve_rhs_homotopy(&Affine::new(0.99, 0.01), &[0.0], &config);
    assert!(result.failure.is_none(), "{:?}", result.failure);
    assert!((result.root.unwrap()[0] - 1.0).abs() < 1e-11);
    assert_eq!(result.work.map_attempted, 4);
    assert_eq!(result.work.extrapolations_accepted, 1);
}

#[test]
fn exact_model_plan_derives_the_smallest_dyadic_grid_and_positive_remainder() {
    // For k=Y=T=1,g=0, C=1/8 and Etrunc(N)=1/[4N(2N+1)].
    let plan = CubicDecayAccuracyPlan::new(1.0, 0.0, 1.0, 1.0, 1e-3, 1024, 100000).unwrap();
    assert_eq!(plan.steps(), 16);
    assert_eq!(plan.step_size(), 1.0 / 16.0);
    let coarser_steps = plan.steps() / 2;
    let coarser_bound = 1.0 / (4.0 * coarser_steps as f64 * (2.0 * coarser_steps as f64 + 1.0));
    assert!(coarser_bound > 1e-3);
    assert!(plan.truncation_bound() >= 1.0 / 2112.0);
    assert!(plan.truncation_bound() < 1e-3);
    assert!(plan.stage_residual_target() > 0.0);
    assert!(plan.contraction_bound() < 1.0);
}

#[test]
fn nonlinear_physical_budget_is_verified_on_every_returned_grid_state() {
    let plan = CubicDecayAccuracyPlan::new(1.0, 0.5, 1.0, 1.0, 1e-4, 4096, 100000).unwrap();
    for mode in [RhsAcceleration::Picard, RhsAcceleration::ResidualDominance] {
        let result = integrate_cubic_decay_accuracy(&plan, mode, true);
        assert!(result.completed, "{:?}", result.failure);
        assert_eq!(result.points.len(), plan.steps() + 1);
        for point in &result.points {
            let exact =
                (-point.time).exp() / (1.0 + 0.5 * (1.0 - (-2.0 * point.time).exp())).sqrt();
            assert!((point.state - exact).abs() <= point.physical_error_bound);
            assert!(point.physical_error_bound <= plan.physical_tolerance());
            assert_eq!(point.exact_time_interval, [point.time, point.time]);
        }
        assert_eq!(result.points.last().unwrap().time, 1.0);
        assert_eq!(
            result.interval_polynomial_evaluations,
            plan.steps() as u64 + result.polish_evaluations
        );
    }
}

#[test]
fn model_resource_limits_preserve_attempted_work_and_decline_incomplete_output() {
    let plan = CubicDecayAccuracyPlan::new(1.0, 0.5, 1.0, 1.0, 1e-4, 4096, 3).unwrap();
    let result = integrate_cubic_decay_accuracy(&plan, RhsAcceleration::ResidualDominance, false);
    assert!(!result.completed);
    assert!(result.failure.is_some());
    assert!(result.map_evaluations + result.interval_polynomial_evaluations <= 3);
    assert!(result.map_evaluations > 0);
    assert!(CubicDecayAccuracyPlan::new(1.0, 0.5, 1.0, 1.0, 1e-4, 4, 10000).is_err());
}

#[test]
fn exact_zero_solution_needs_no_unearned_error_allowance() {
    let plan = CubicDecayAccuracyPlan::new(1.0, 0.5, 0.0, 1.0, 1e-5, 1024, 10000).unwrap();
    let result = integrate_cubic_decay_accuracy(&plan, RhsAcceleration::ResidualDominance, false);
    assert!(result.completed, "{:?}", result.failure);
    for p in &result.points {
        assert_eq!(p.state, 0.0);
        assert_eq!(p.physical_error_bound, 0.0);
    }
}

#[test]
fn interval_polish_refuses_a_budget_below_the_represented_residual_floor() {
    let ordinary = CubicDecayAccuracyPlan::new(1.0, 0.0, 1.0, 1.0, 1e-3, 1024, 100000).unwrap();
    let almost_all_truncation = ordinary.truncation_bound().next_up();
    let plan = CubicDecayAccuracyPlan::new(1.0, 0.0, 1.0, 1.0, almost_all_truncation, 1024, 100000)
        .unwrap();
    assert_eq!(plan.steps(), ordinary.steps());
    let result = integrate_cubic_decay_accuracy(&plan, RhsAcceleration::ResidualDominance, false);
    assert!(!result.completed);
    assert!(result.polish_evaluations > 0, "{:?}", result.failure);
    assert!(
        result.failure.as_ref().unwrap().contains("budget")
            || result.failure.as_ref().unwrap().contains("floor")
    );
    assert_eq!(result.points.len(), 1);
}
