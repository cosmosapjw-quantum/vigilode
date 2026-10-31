use vigilode_coupled::certified_fastfirst::{
    CertifiedConfig, CertifiedFailureKind, FrozenDiagonalAffine,
};

fn cfg(fast: bool) -> CertifiedConfig {
    CertifiedConfig {
        fast_first: fast,
        root_error_target: 1e-10,
        ..CertifiedConfig::default()
    }
}

fn source(n: usize) -> FrozenDiagonalAffine {
    FrozenDiagonalAffine::new(vec![0.99; n], vec![0.01; n]).unwrap()
}

#[test]
fn source_owns_the_exact_represented_coefficients_not_a_caller_q() {
    let mut diagonal = vec![0.99, -0.8];
    let mut shift = vec![0.01, 0.2];
    let source = FrozenDiagonalAffine::new(diagonal.clone(), shift.clone()).unwrap();
    let bits = source.represented_source_bits();
    diagonal[0] = -0.1;
    shift[0] = 0.0;
    assert_eq!(bits[0], (0.99_f64.to_bits(), 0.01_f64.to_bits()));
    assert_eq!(bits[1], ((-0.8_f64).to_bits(), 0.2_f64.to_bits()));
    assert_eq!(
        source.represented_contraction().to_bits(),
        0.99_f64.to_bits()
    );
    assert_eq!(source.dimension(), 2);
}

#[test]
fn fast_first_scalar_has_fewer_charged_map_and_interval_evaluations() {
    let owned = source(1);
    let old = owned.solve(&[0.0], &cfg(false));
    let fast = owned.solve(&[0.0], &cfg(true));
    assert!(old.failure.is_none(), "{:?}", old.failure);
    assert!(fast.failure.is_none(), "{:?}", fast.failure);
    assert_eq!(old.work.map_attempted, 4);
    assert_eq!(fast.work.map_attempted, 3);
    assert_eq!(old.work.exact_residual_interval_evaluations, 4);
    assert_eq!(fast.work.exact_residual_interval_evaluations, 3);
    assert_eq!(fast.work.trials_admitted, 1);
    assert_eq!(fast.work.rounded_picard_admissions, 1);
    let target = 0.01_f64 / (1.0 - 0.99_f64);
    let value = fast.root.unwrap()[0];
    assert!((value - target).abs() <= fast.points[0].exact_root_error_upper);
    assert!(fast.points[0].exact_root_error_upper <= cfg(true).root_error_target);
}

#[test]
fn many_independent_modes_keep_the_original_error_target_and_full_state() {
    for n in [3, 8, 12, 24, 65, 257] {
        let owned = source(n);
        let old = owned.solve(&vec![0.0; n], &cfg(false));
        let fast = owned.solve(&vec![0.0; n], &cfg(true));
        assert!(
            old.failure.is_none() && fast.failure.is_none(),
            "{n}: {:?} {:?}",
            old.failure,
            fast.failure
        );
        assert_eq!(old.work.map_attempted, 4, "n={n}");
        assert_eq!(fast.work.map_attempted, 3, "n={n}");
        assert_eq!(
            old.work.map_completed,
            old.work.exact_residual_interval_evaluations
        );
        assert_eq!(
            fast.work.map_completed,
            fast.work.exact_residual_interval_evaluations
        );
        assert_eq!(old.points[0].state.len(), n);
        assert_eq!(fast.points[0].state.len(), n);
        assert_eq!(old.points[0].state, fast.points[0].state);
        assert!(fast.points[0].exact_root_error_upper <= 1e-10);
        println!(
            "CERTIFIED_FASTFIRST n={n} picard={} fast={} exact_error_upper={:.17e}",
            old.work.map_attempted, fast.work.map_attempted, fast.points[0].exact_root_error_upper
        );
    }
}

#[test]
fn source_contraction_with_mixed_signs_and_nonuniform_rates_is_still_admissible() {
    let a = vec![0.3, -0.8, 0.99, 0.5, -0.45];
    let b = vec![0.7, 0.3, 0.01, -0.5, 0.125];
    let target: Vec<_> = a.iter().zip(&b).map(|(a, b)| *b / (1.0 - *a)).collect();
    let owned = FrozenDiagonalAffine::new(a, b).unwrap();
    let result = owned.solve(&[0.0; 5], &cfg(true));
    assert!(result.failure.is_none(), "{:?}", result.failure);
    let root = result.root.unwrap();
    let error = root
        .iter()
        .zip(&target)
        .map(|(x, t)| (x - t) * (x - t))
        .sum::<f64>()
        .sqrt();
    assert!(error < result.points[0].exact_root_error_upper + 1e-14);
    assert!(result.points[0].exact_root_error_upper < 1e-10);
}

#[test]
fn continuation_station_stays_source_locked_and_uses_each_lambda() {
    let owned = FrozenDiagonalAffine::new(vec![0.6], vec![0.2]).unwrap();
    let conf = CertifiedConfig {
        lambdas: vec![0.25, 0.5, 1.0],
        root_error_target: 1e-11,
        ..cfg(true)
    };
    let attempt = owned.solve(&[0.1], &conf);
    assert!(attempt.failure.is_none(), "{:?}", attempt.failure);
    assert_eq!(attempt.points.len(), 3);
    assert_eq!(attempt.work.continuation_stations, 3);
    for point in &attempt.points {
        let lambda = point.lambda;
        let exact = ((1.0 - lambda) * 0.1 + lambda * 0.2) / (1.0 - lambda * 0.6);
        assert!((point.state[0] - exact).abs() <= point.exact_root_error_upper + 2e-16);
        assert!(point.exact_root_error_upper <= conf.root_error_target);
    }
}

#[test]
fn invalid_or_noncontracting_sources_refuse_before_any_candidate() {
    for a in [1.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            FrozenDiagonalAffine::new(vec![a], vec![0.5])
                .unwrap_err()
                .kind,
            CertifiedFailureKind::Input
        );
    }
    assert_eq!(
        FrozenDiagonalAffine::new(vec![0.1], vec![f64::INFINITY])
            .unwrap_err()
            .kind,
        CertifiedFailureKind::Input
    );
    assert_eq!(
        FrozenDiagonalAffine::new(vec![0.1], vec![])
            .unwrap_err()
            .kind,
        CertifiedFailureKind::Input
    );
    assert_eq!(
        FrozenDiagonalAffine::new(vec![], vec![]).unwrap_err().kind,
        CertifiedFailureKind::Input
    );
}

#[test]
fn invalid_station_and_anchor_refuse_without_source_dispatch() {
    let owned = source(2);
    for conf in [
        CertifiedConfig {
            lambdas: vec![0.5, 0.5, 1.0],
            ..cfg(true)
        },
        CertifiedConfig {
            lambdas: vec![0.25, 0.5],
            ..cfg(true)
        },
        CertifiedConfig {
            root_error_target: f64::NAN,
            ..cfg(true)
        },
        CertifiedConfig {
            max_map_evaluations: 0,
            ..cfg(true)
        },
    ] {
        let o = owned.solve(&[0.0, 0.0], &conf);
        assert_eq!(o.failure.unwrap().kind, CertifiedFailureKind::Input);
        assert_eq!(o.work.map_attempted, 0);
    }
    let o = owned.solve(&[0.0, f64::INFINITY], &cfg(true));
    assert_eq!(o.failure.unwrap().kind, CertifiedFailureKind::Input);
    assert_eq!(o.work.map_attempted, 0);
}

#[test]
fn attempted_work_and_accepted_prefix_survive_budget_refusal() {
    let owned = source(1);
    for cap in [1, 2] {
        let conf = CertifiedConfig {
            max_map_evaluations: cap,
            ..cfg(true)
        };
        let attempt = owned.solve(&[0.0], &conf);
        assert_eq!(attempt.failure.unwrap().kind, CertifiedFailureKind::Budget);
        assert_eq!(attempt.work.map_attempted, cap);
        assert_eq!(attempt.work.map_completed, cap);
        assert_eq!(attempt.work.exact_residual_interval_evaluations, cap);
        assert!(attempt.root.is_none());
        assert!(attempt.points.is_empty());
    }
}

#[test]
fn exact_real_residual_cannot_be_replaced_by_nonfinite_float_state() {
    let owned = FrozenDiagonalAffine::new(vec![0.5], vec![1.0e308]).unwrap();
    let out = owned.solve(&[1.0e308], &cfg(true));
    assert_eq!(out.failure.unwrap().kind, CertifiedFailureKind::Arithmetic);
    assert_eq!(out.work.map_attempted, 1);
    assert_eq!(out.work.map_completed, 0);
    assert!(out.root.is_none());
}

#[test]
fn impossible_below_roundoff_root_target_fails_closed() {
    let owned = source(1);
    let conf = CertifiedConfig {
        root_error_target: 1e-30,
        max_iterations_per_station: 20,
        max_map_evaluations: 40,
        ..cfg(true)
    };
    let result = owned.solve(&[0.0], &conf);
    assert!(result.failure.is_some());
    assert!(result.root.is_none());
    assert!(result.work.map_attempted > 0);
}

#[test]
fn exact_zero_coefficient_map_and_unique_root_are_admitted() {
    let owned = FrozenDiagonalAffine::new(vec![0.0], vec![0.5]).unwrap();
    let result = owned.solve(&[0.0], &cfg(true));
    assert!(result.failure.is_none(), "{:?}", result.failure);
    assert_eq!(result.root.unwrap(), vec![0.5]);
    assert_eq!(result.points[0].exact_residual_l2_upper, 0.0);
    assert_eq!(result.work.map_attempted, 2);
}
