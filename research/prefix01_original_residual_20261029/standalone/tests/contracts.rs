use vigilode_prefix01::*;
fn diagonal(n: usize) -> BoundModel {
    let mut j = vec![0.0; n * n];
    for i in 0..n {
        j[i * n + i] = -2.0 - (i as f64) / 8.0;
    }
    BoundModel {
        n,
        jacobian: j,
        weights: vec![1.0; n],
        sigma: 0.25,
        model_id: "diag16".into(),
        source_id: "declared-rates".into(),
        clock_id: "x-ln-a".into(),
        norm_id: "weighted-infinity".into(),
        epoch: 0,
    }
}
fn rhs(n: usize) -> Vec<f64> {
    (0..n).map(|i| 1.0 + 0.25 * ((i % 3) as f64)).collect()
}
#[test]
fn scalar_resolvent_and_original_residual() {
    let m = diagonal(1);
    let x = [2.0 / 3.0];
    let c = m.certify(&[1.0], &x, 1e-12, 1e-12).unwrap();
    assert!(c.accepted);
    assert!(c.inverse_upper < 0.667);
    assert!(c.residual_upper < 1e-13);
}
#[test]
fn lognorm_negative_and_exact_diagonal() {
    let m = diagonal(16);
    let mu = m.mu_upper().unwrap();
    assert!((-2.0..-1.99999999999999).contains(&mu));
    assert!(m.inverse_bound().unwrap() < 0.667);
}
#[test]
fn prefix_accepts_with_fewer_columns_than_full() {
    let m = diagonal(16);
    let b = rhs(16);
    let z = vec![0.0; 16];
    let mut a = vec![0.0; 16];
    let mut c = vec![0.0; 16];
    let p = solve_into(
        &m,
        &b,
        &z,
        &mut a,
        SolveConfig {
            max_columns: 16,
            tau_inf: 1e-7,
            stage_budget: 1e-7,
            mode: TriggerMode::Prefix {
                projected_tau: 1e-7,
                ratio: 1.5,
            },
        },
    )
    .unwrap();
    let f = solve_into(
        &m,
        &b,
        &z,
        &mut c,
        SolveConfig {
            max_columns: 16,
            tau_inf: 1e-7,
            stage_budget: 1e-7,
            mode: TriggerMode::EndOnly,
        },
    )
    .unwrap();
    assert!(p.accepted && f.accepted);
    assert!(p.work.arnoldi_columns < f.work.arnoldi_columns);
    for i in 0..16 {
        let exact = b[i] / (1.0 - 0.25 * m.jacobian[i * 16 + i]);
        assert!((a[i] - exact).abs() < 1e-7);
        assert!((c[i] - exact).abs() < 1e-7);
    }
}
#[test]
fn always_trigger_cannot_force_false_accept() {
    let m = diagonal(8);
    let b = rhs(8);
    let z = vec![0.0; 8];
    let mut out = vec![17.0; 8];
    let r = solve_into(
        &m,
        &b,
        &z,
        &mut out,
        SolveConfig {
            max_columns: 2,
            tau_inf: 0.0,
            stage_budget: 0.0,
            mode: TriggerMode::Always,
        },
    )
    .unwrap();
    assert!(!r.accepted);
    assert!(r.work.refused_checks >= 1);
    assert!(out.iter().all(|x| *x == 17.0));
}
#[test]
fn warm_start_exact_enough_needs_no_arnoldi() {
    let m = diagonal(4);
    let b = rhs(4);
    let x = (0..4)
        .map(|i| b[i] / (1.0 - 0.25 * m.jacobian[i * 4 + i]))
        .collect::<Vec<_>>();
    let mut out = vec![0.0; 4];
    let r = solve_into(
        &m,
        &b,
        &x,
        &mut out,
        SolveConfig {
            max_columns: 4,
            tau_inf: 1e-12,
            stage_budget: 1e-12,
            mode: TriggerMode::EndOnly,
        },
    )
    .unwrap();
    assert!(r.accepted);
    assert_eq!(r.work.arnoldi_columns, 0);
    assert_eq!(out, x);
}
#[test]
fn invalid_model_and_margin_refuse() {
    let mut m = diagonal(2);
    m.weights[0] = 0.0;
    assert!(m.mu_upper().is_err());
    m.weights[0] = 1.0;
    m.sigma = -0.1;
    assert!(m.inverse_bound().is_err());
    m.sigma = 0.1;
    m.jacobian[1] = 50.0;
    assert!(m.inverse_bound().is_err());
}
#[test]
fn event_sign_uses_dual_radius() {
    let m = diagonal(2);
    assert_eq!(
        event_sign(&m, &[1.0, 1.0], &[1.0, -1.0], 0.0, 0.1).unwrap(),
        EventSign::Unresolved
    );
    assert_eq!(
        event_sign(&m, &[2.0, 1.0], &[1.0, -1.0], 0.0, 0.1).unwrap(),
        EventSign::Positive
    );
}
#[test]
fn no_mutation_on_dimension_or_nonfinite_failure() {
    let m = diagonal(2);
    let mut out = [55.0, 55.0];
    assert!(
        solve_into(
            &m,
            &[1.0, 1.0],
            &[0.0],
            &mut out,
            SolveConfig {
                max_columns: 2,
                tau_inf: 1e-6,
                stage_budget: 1e-6,
                mode: TriggerMode::EndOnly
            }
        )
        .is_err()
    );
    assert_eq!(out, [55.0, 55.0]);
}
#[test]
fn scalar_zero_shift() {
    let mut m = diagonal(1);
    m.sigma = 0.0;
    let mut out = [0.0];
    let r = solve_into(
        &m,
        &[1.0],
        &[0.0],
        &mut out,
        SolveConfig {
            max_columns: 1,
            tau_inf: 1e-12,
            stage_budget: 1e-12,
            mode: TriggerMode::Prefix {
                projected_tau: 1e-12,
                ratio: 1.0,
            },
        },
    )
    .unwrap();
    assert!(r.accepted);
    assert!((out[0] - 1.0).abs() < 1e-12);
}
