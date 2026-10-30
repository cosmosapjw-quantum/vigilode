use std::cell::Cell;
use vigilode_coupled::rhs_homotopy::{
    RhsAcceleration, RhsHomotopyConfig, RhsHomotopyFailureKind, RhsOnlyMap, solve_rhs_homotopy,
    solve_rhs_homotopy_contraction_fast_first,
};

struct Affine {
    a: f64,
    b: f64,
    calls: Cell<u64>,
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
fn affine() -> Affine {
    Affine {
        a: 0.99,
        b: 0.01,
        calls: Cell::new(0),
    }
}
fn cfg() -> RhsHomotopyConfig {
    RhsHomotopyConfig {
        acceleration: RhsAcceleration::ResidualDominance,
        residual_l2_tolerance: 1e-12,
        ..RhsHomotopyConfig::default()
    }
}

#[test]
fn scalar_affine_reuses_the_contraction_envelope_without_baseline_callback() {
    let old_map = affine();
    let old = solve_rhs_homotopy(&old_map, &[0.0], &cfg());
    let new_map = affine();
    let fast = solve_rhs_homotopy_contraction_fast_first(&new_map, &[0.0], &cfg(), 0.99);
    assert!(old.failure.is_none() && fast.failure.is_none());
    assert_eq!(old.work.map_attempted, 4);
    assert_eq!(fast.work.map_attempted, 3);
    assert_eq!(new_map.calls.get(), 3);
    assert_eq!(old_map.calls.get(), 4);
    assert_eq!(fast.work.extrapolations_accepted, 1);
    assert_eq!(fast.work.picard_updates, 1);
    assert_eq!(fast.points.len(), 1);
    assert!((fast.root.unwrap()[0] - 1.0).abs() < 1e-11);
}

#[test]
fn every_homotopy_station_uses_lambda_times_the_source_contraction() {
    struct Nonlinear {
        calls: Cell<u64>,
    }
    impl RhsOnlyMap for Nonlinear {
        fn dimension(&self) -> usize {
            1
        }
        fn admits(&self, x: &[f64]) -> bool {
            x.len() == 1 && x[0].is_finite() && (-1.0..=1.0).contains(&x[0])
        }
        fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
            self.calls.set(self.calls.get() + 1);
            out[0] = 0.35 + 0.4 * x[0] + 0.1 * x[0] * x[0];
            Ok(())
        }
    }
    // On [-1,1], G is invariant with |G'|<=0.6, for every fixed source.
    let map = Nonlinear {
        calls: Cell::new(0),
    };
    let config = RhsHomotopyConfig {
        lambdas: vec![0.25, 0.5, 1.0],
        ..cfg()
    };
    let result = solve_rhs_homotopy_contraction_fast_first(&map, &[0.0], &config, 0.6);
    assert!(result.failure.is_none(), "{:?}", result.failure);
    assert_eq!(result.points.len(), 3);
    assert_eq!(map.calls.get(), result.work.map_attempted);
    for point in &result.points {
        let mut lo = 0.0;
        let mut hi = 1.0;
        for _ in 0..100 {
            let mid = 0.5 * (lo + hi);
            if mid - point.lambda * (0.35 + 0.4 * mid + 0.1 * mid * mid) > 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let root = 0.5 * (lo + hi);
        assert!((point.state[0] - root).abs() < 3e-12);
        assert!(point.residual_l2 < config.residual_l2_tolerance);
        assert!(
            (point.state[0] - root).abs() < point.residual_l2 / (1.0 - point.lambda * 0.6) + 5e-15
        );
    }
}

#[test]
fn unproved_or_invalid_numeric_contraction_contract_is_rejected_before_rhs() {
    for q in [-0.1, 1.0, f64::INFINITY, f64::NAN] {
        let map = affine();
        let result = solve_rhs_homotopy_contraction_fast_first(&map, &[0.0], &cfg(), q);
        assert_eq!(result.failure.unwrap().kind, RhsHomotopyFailureKind::Input);
        assert_eq!(map.calls.get(), 0);
    }
    let mut wrong_mode = cfg();
    wrong_mode.acceleration = RhsAcceleration::Picard;
    let map = affine();
    let result = solve_rhs_homotopy_contraction_fast_first(&map, &[0.0], &wrong_mode, 0.99);
    assert_eq!(result.failure.unwrap().kind, RhsHomotopyFailureKind::Input);
    assert_eq!(map.calls.get(), 0);
}

#[test]
fn budget_keeps_one_fallback_callback_and_never_overdispatches() {
    let map = affine();
    let config = RhsHomotopyConfig {
        max_map_evaluations: 2,
        ..cfg()
    };
    let result = solve_rhs_homotopy_contraction_fast_first(&map, &[0.0], &config, 0.99);
    assert_eq!(result.failure.unwrap().kind, RhsHomotopyFailureKind::Budget);
    assert_eq!(map.calls.get(), 2);
    assert_eq!(result.work.map_attempted, 2);
    assert_eq!(result.work.extrapolation_trials, 0);
    assert!(result.root.is_none());
}

#[test]
fn speculative_callback_failure_is_fatal_and_attempt_is_charged() {
    struct Fails {
        calls: Cell<u64>,
    }
    impl RhsOnlyMap for Fails {
        fn dimension(&self) -> usize {
            1
        }
        fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
            self.calls.set(self.calls.get() + 1);
            if x[0] > 0.9 {
                return Err("speculative forbidden trial".into());
            }
            out[0] = 0.99 * x[0] + 0.01;
            Ok(())
        }
    }
    let map = Fails {
        calls: Cell::new(0),
    };
    let result = solve_rhs_homotopy_contraction_fast_first(&map, &[0.0], &cfg(), 0.99);
    assert_eq!(
        result.failure.unwrap().kind,
        RhsHomotopyFailureKind::Callback
    );
    assert_eq!(map.calls.get(), result.work.map_attempted);
    assert_eq!(result.work.map_attempted, result.work.map_completed + 1);
    assert!(result.root.is_none());
}

#[test]
fn mutation_during_callback_is_not_a_successful_source_certificate() {
    struct Changes {
        epoch: Cell<u64>,
    }
    impl RhsOnlyMap for Changes {
        fn dimension(&self) -> usize {
            1
        }
        fn model_epoch(&self) -> Option<u64> {
            Some(self.epoch.get())
        }
        fn evaluate(&self, _: &[f64], out: &mut [f64]) -> Result<(), String> {
            out[0] = 0.0;
            self.epoch.set(1);
            Ok(())
        }
    }
    let map = Changes {
        epoch: Cell::new(0),
    };
    let result = solve_rhs_homotopy_contraction_fast_first(&map, &[1.0], &cfg(), 0.5);
    assert_eq!(
        result.failure.unwrap().kind,
        RhsHomotopyFailureKind::ModelChanged
    );
    assert_eq!(result.work.map_attempted, 1);
    assert_eq!(result.work.map_completed, 0);
}
