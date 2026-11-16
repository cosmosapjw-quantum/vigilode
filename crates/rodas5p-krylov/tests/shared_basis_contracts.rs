use rodas5p_krylov::shared_basis::{SharedBasis, SharedBasisConfig, SharedSolveStats};

#[test]
fn shared_basis_meets_budget_on_shifted_diagonal() {
    let n = 32;
    let h_gamma = 0.01;
    let weights = vec![2.0; n];
    let theta = 1.0e-8;
    let cfg = SharedBasisConfig { max_dim: 60, theta };
    let mut basis = SharedBasis::new(n, cfg).expect("valid configuration");
    assert_eq!(basis.dim(), 0);

    let b: Vec<f64> = (0..n).map(|i| 1.0 + (i as f64) / 32.0).collect();

    let mut jvp_calls = 0;
    let mut jvp = |v: &[f64], out: &mut [f64]| -> Result<(), String> {
        jvp_calls += 1;
        for (i, (&vi, outi)) in v.iter().zip(out.iter_mut()).enumerate() {
            *outi = -10.0 * (i as f64 + 1.0) * vi;
        }
        Ok(())
    };

    let mut x = vec![0.0; n];
    let stats = basis
        .solve_shifted(h_gamma, &b, &weights, &mut jvp, &mut x)
        .expect("solve shifted");

    assert!(stats.converged);
    assert_eq!(stats.new_jvps, jvp_calls);
    assert!(stats.new_jvps > 0);
    assert!(stats.basis_dim > 0);
    assert_eq!(basis.dim(), stats.basis_dim);

    // Recompute the true residual b - (x / h_gamma - J x)
    let mut true_r = vec![0.0; n];
    for i in 0..n {
        let j_xi = -10.0 * (i as f64 + 1.0) * x[i];
        let shifted_xi = x[i] / h_gamma - j_xi;
        true_r[i] = b[i] - shifted_xi;
    }
    let sum_sq: f64 = true_r
        .iter()
        .zip(&weights)
        .map(|(&ri, &wi)| (ri * wi).powi(2))
        .sum();
    let true_r_w = (sum_sq / n as f64).sqrt();

    assert!(h_gamma * true_r_w <= theta);
    assert!((stats.weighted_residual - true_r_w).abs() < 1.0e-12);
}

#[test]
fn shared_basis_reuses_directions_across_stages() {
    let n = 32;
    let h_gamma = 0.01;
    let weights = vec![2.0; n];
    let theta = 1.0e-8;
    let cfg = SharedBasisConfig { max_dim: 60, theta };
    let mut basis = SharedBasis::new(n, cfg).expect("valid configuration");

    let b1: Vec<f64> = (0..n).map(|i| 1.0 + (i as f64) / 32.0).collect();

    let mut jvp = |v: &[f64], out: &mut [f64]| -> Result<(), String> {
        for (i, (&vi, outi)) in v.iter().zip(out.iter_mut()).enumerate() {
            *outi = -10.0 * (i as f64 + 1.0) * vi;
        }
        Ok(())
    };

    let mut x1 = vec![0.0; n];
    let stats1 = basis
        .solve_shifted(h_gamma, &b1, &weights, &mut jvp, &mut x1)
        .expect("solve b1");
    assert!(stats1.converged);
    assert!(stats1.new_jvps > 1);

    // Form b2 = 0.5 * b1 + 0.1 * (x1 / h_gamma) which lies in range(Q) + range(Z)
    let mut b2 = vec![0.0; n];
    for i in 0..n {
        b2[i] = 0.5 * b1[i] + 0.1 * (x1[i] / h_gamma);
    }

    let mut x2 = vec![0.0; n];
    let stats2 = basis
        .solve_shifted(h_gamma, &b2, &weights, &mut jvp, &mut x2)
        .expect("solve b2 with warm basis");
    assert!(stats2.converged);
    assert!(stats2.new_jvps <= 1);

    // Solve b2 alone with a fresh basis
    let mut fresh_basis = SharedBasis::new(n, cfg).expect("valid configuration");
    let mut x_fresh = vec![0.0; n];
    let stats_fresh = fresh_basis
        .solve_shifted(h_gamma, &b2, &weights, &mut jvp, &mut x_fresh)
        .expect("solve b2 with fresh basis");
    assert!(stats_fresh.converged);
    assert!(stats_fresh.new_jvps > stats2.new_jvps);
}

#[test]
fn shared_basis_refuses_invalid_input() {
    let valid_cfg = SharedBasisConfig {
        max_dim: 10,
        theta: 1.0e-3,
    };

    // new with n = 0
    assert!(SharedBasis::new(0, valid_cfg).is_err());

    // new with max_dim = 0
    assert!(
        SharedBasis::new(
            10,
            SharedBasisConfig {
                max_dim: 0,
                theta: 1.0e-3
            }
        )
        .is_err()
    );

    // new with theta = 0.0
    assert!(
        SharedBasis::new(
            10,
            SharedBasisConfig {
                max_dim: 10,
                theta: 0.0
            }
        )
        .is_err()
    );

    // new with theta negative
    assert!(
        SharedBasis::new(
            10,
            SharedBasisConfig {
                max_dim: 10,
                theta: -1.0
            }
        )
        .is_err()
    );

    // new with theta NaN
    assert!(
        SharedBasis::new(
            10,
            SharedBasisConfig {
                max_dim: 10,
                theta: f64::NAN
            }
        )
        .is_err()
    );

    // new with theta Inf
    assert!(
        SharedBasis::new(
            10,
            SharedBasisConfig {
                max_dim: 10,
                theta: f64::INFINITY
            }
        )
        .is_err()
    );

    // solve_shifted with wrong slice lengths or invalid h_gamma
    let mut basis = SharedBasis::new(10, valid_cfg).expect("valid configuration");
    let jvp_calls = std::cell::Cell::new(0usize);
    let mut jvp = |_: &[f64], _: &mut [f64]| -> Result<(), String> {
        jvp_calls.set(jvp_calls.get() + 1);
        Ok(())
    };

    let b_good = [1.0; 10];
    let w_good = [1.0; 10];
    let mut x_good = [0.0; 10];

    // Wrong b length
    let stats = basis
        .solve_shifted(0.01, &[1.0; 9], &w_good, &mut jvp, &mut x_good)
        .expect("call succeeds");
    assert!(!stats.converged);
    assert_eq!(jvp_calls.get(), 0);

    // Wrong weights length
    let stats = basis
        .solve_shifted(0.01, &b_good, &[1.0; 11], &mut jvp, &mut x_good)
        .expect("call succeeds");
    assert!(!stats.converged);
    assert_eq!(jvp_calls.get(), 0);

    // Wrong x length
    let mut x_short = [0.0; 8];
    let stats = basis
        .solve_shifted(0.01, &b_good, &w_good, &mut jvp, &mut x_short)
        .expect("call succeeds");
    assert!(!stats.converged);
    assert_eq!(jvp_calls.get(), 0);

    // Non-positive or non-finite h_gamma
    let stats = basis
        .solve_shifted(0.0, &b_good, &w_good, &mut jvp, &mut x_good)
        .expect("call succeeds");
    assert!(!stats.converged);
    assert_eq!(jvp_calls.get(), 0);

    let stats = basis
        .solve_shifted(-0.5, &b_good, &w_good, &mut jvp, &mut x_good)
        .expect("call succeeds");
    assert!(!stats.converged);
    assert_eq!(jvp_calls.get(), 0);

    let stats = basis
        .solve_shifted(f64::NAN, &b_good, &w_good, &mut jvp, &mut x_good)
        .expect("call succeeds");
    assert!(!stats.converged);
    assert_eq!(jvp_calls.get(), 0);
}

#[test]
fn shared_basis_zero_rhs_converges_immediately() {
    let n = 8;
    let cfg = SharedBasisConfig {
        max_dim: 10,
        theta: 1.0e-3,
    };
    let mut basis = SharedBasis::new(n, cfg).expect("valid");
    let b = vec![0.0; n];
    let weights = vec![1.0; n];
    let mut x = vec![1.0; n]; // should be zeroed
    let mut jvp_called = 0;
    let mut jvp = |_: &[f64], _: &mut [f64]| -> Result<(), String> {
        jvp_called += 1;
        Ok(())
    };

    let stats = basis
        .solve_shifted(0.1, &b, &weights, &mut jvp, &mut x)
        .expect("zero rhs solve");
    assert!(stats.converged);
    assert_eq!(stats.new_jvps, 0);
    assert_eq!(jvp_called, 0);
    assert_eq!(stats.weighted_residual, 0.0);
    assert!(x.iter().all(|&xi| xi == 0.0));
}

#[test]
fn shared_basis_dim_and_reset_invariants() {
    let n = 16;
    let cfg = SharedBasisConfig {
        max_dim: 17,
        theta: 2.5e-4,
    };
    let mut basis = SharedBasis::new(n, cfg).expect("valid");
    assert_eq!(basis.dim(), 0);
    assert_eq!(basis.config(), cfg);

    let b = vec![1.0; n];
    let weights = vec![1.0; n];
    let mut x = vec![0.0; n];
    let mut jvp = |v: &[f64], out: &mut [f64]| -> Result<(), String> {
        for (out_i, &v_i) in out.iter_mut().zip(v) {
            *out_i = -2.0 * v_i;
        }
        Ok(())
    };

    let stats = basis
        .solve_shifted(0.05, &b, &weights, &mut jvp, &mut x)
        .expect("solve");
    assert!(stats.converged);
    assert!(basis.dim() > 0);
    assert_eq!(basis.dim(), stats.basis_dim);

    basis.reset();
    assert_eq!(basis.dim(), 0);
}

#[test]
fn shared_basis_jvp_error_propagates() {
    let n = 8;
    let cfg = SharedBasisConfig::default();
    let mut basis = SharedBasis::new(n, cfg).expect("valid");
    let b = vec![1.0; n];
    let weights = vec![1.0; n];
    let mut x = vec![0.0; n];
    let mut jvp =
        |_: &[f64], _: &mut [f64]| -> Result<(), &'static str> { Err("synthetic JVP failure") };

    let result = basis.solve_shifted(0.1, &b, &weights, &mut jvp, &mut x);
    assert_eq!(result, Err("synthetic JVP failure"));
}

#[test]
fn shared_basis_max_dim_reached_reports_failure() {
    let n = 16;
    // Set max_dim very low and theta extremely small so it cannot converge
    let cfg = SharedBasisConfig {
        max_dim: 2,
        theta: 1.0e-30,
    };
    let mut basis = SharedBasis::new(n, cfg).expect("valid");
    let mut b = vec![0.0; n];
    for (i, bi) in b.iter_mut().enumerate() {
        *bi = (i as f64 + 1.0) * 0.5;
    }
    let weights = vec![1.0; n];
    let mut x = vec![0.0; n];
    let mut jvp = |v: &[f64], out: &mut [f64]| -> Result<(), String> {
        for (i, (&vi, outi)) in v.iter().zip(out.iter_mut()).enumerate() {
            *outi = -5.0 * (i as f64 + 1.0) * vi;
        }
        Ok(())
    };

    let stats = basis
        .solve_shifted(0.1, &b, &weights, &mut jvp, &mut x)
        .expect("solve");
    assert!(!stats.converged);
    assert_eq!(stats.basis_dim, 2);
}

#[test]
fn shared_basis_config_and_stats_traits() {
    let cfg1 = SharedBasisConfig {
        max_dim: 50,
        theta: 1.0e-4,
    };
    let cfg2 = cfg1;
    assert_eq!(cfg1, cfg2);
    let debug_cfg = format!("{cfg1:?}");
    assert!(debug_cfg.contains("50"));

    let stats1 = SharedSolveStats {
        new_jvps: 3,
        basis_dim: 4,
        weighted_residual: 1.0e-5,
        converged: true,
    };
    let stats2 = stats1;
    assert_eq!(stats1, stats2);
    let debug_stats = format!("{stats1:?}");
    assert!(debug_stats.contains("new_jvps: 3"));
}
