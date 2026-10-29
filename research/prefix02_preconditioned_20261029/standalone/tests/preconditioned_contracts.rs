use std::cell::Cell;
use vigilode_prefix02::preconditioned::{
    BindingSnapshot, PrefixConfig, Side, solve_preconditioned,
};
use vigilode_prefix02::{BoundModel, TriggerMode};
fn model(n: usize) -> BoundModel {
    let mut jacobian = vec![0.0; n * n];
    for i in 0..n {
        jacobian[i * n + i] = -2.0 - (i as f64) / 8.0;
    }
    BoundModel {
        n,
        jacobian,
        weights: vec![1.0; n],
        sigma: 0.25,
        model_id: "synthetic-rates".into(),
        source_id: "frozen-source".into(),
        clock_id: "x=-ln(a)".into(),
        norm_id: "weighted-infinity".into(),
        epoch: 9,
    }
}
fn b(n: usize) -> Vec<f64> {
    (0..n).map(|i| 1.0 + ((i % 3) as f64) * 0.25).collect()
}
fn inv(m: &BoundModel) -> Vec<f64> {
    (0..m.n)
        .map(|i| 1.0 / (1.0 - m.sigma * m.jacobian[i * m.n + i]))
        .collect()
}
fn config(mode: TriggerMode, restart: usize, max_columns: usize) -> PrefixConfig {
    PrefixConfig {
        max_columns,
        restart,
        tau_orig: 1e-10,
        stage_budget: 1e-10,
        mode,
    }
}
fn solve(
    m: &BoundModel,
    side: Side,
    diag: &[f64],
    cfg: PrefixConfig,
) -> (Vec<f64>, vigilode_prefix02::preconditioned::Outcome) {
    let key = BindingSnapshot::from_model(m);
    let mut live = || key.clone();
    let mut out = vec![123.0; m.n];
    let outcome = solve_preconditioned(
        m,
        &b(m.n),
        &vec![0.0; m.n],
        &mut out,
        diag,
        side,
        cfg,
        &key,
        &mut live,
    )
    .unwrap();
    (out, outcome)
}
#[test]
fn right_jacobi_one_column_original_residual() {
    let m = model(16);
    let (out, r) = solve(
        &m,
        Side::Right,
        &inv(&m),
        config(TriggerMode::Always, 16, 16),
    );
    assert!(r.accepted);
    assert_eq!(r.work.columns, 1);
    assert!(r.certificate.unwrap().forward_upper < 1e-10);
    for (i, x) in out.iter().enumerate() {
        let exact = b(16)[i] * inv(&m)[i];
        assert!((x - exact).abs() < 1e-13);
    }
}
#[test]
fn left_jacobi_one_column_original_residual() {
    let m = model(16);
    let (out, r) = solve(
        &m,
        Side::Left,
        &inv(&m),
        config(TriggerMode::Always, 16, 16),
    );
    assert!(r.accepted);
    assert_eq!(r.work.columns, 1);
    assert!(r.certificate.unwrap().forward_upper < 1e-10);
    assert!(out.iter().all(|v| v.is_finite()));
}
#[test]
fn identity_preconditioner_both_sides() {
    let m = model(16);
    for side in [Side::Right, Side::Left] {
        let (out, r) = solve(
            &m,
            side,
            &vec![1.0; m.n],
            config(
                TriggerMode::Prefix {
                    projected_tau: 1e-10,
                    ratio: 2.0,
                },
                16,
                16,
            ),
        );
        assert!(r.accepted);
        assert!(r.work.columns > 1);
        let rhs = b(m.n);
        let exact_inv = inv(&m);
        for (i, &xi) in out.iter().enumerate() {
            assert!((xi - rhs[i] * exact_inv[i]).abs() < 1e-9);
        }
    }
}
#[test]
fn left_projected_small_does_not_accept_original_large() {
    let mut m = model(2);
    m.sigma = 0.0;
    let key = BindingSnapshot::from_model(&m);
    let mut live = || key.clone();
    let mut out = [44.0, 44.0];
    let r = solve_preconditioned(
        &m,
        &[1.0, 0.0],
        &[0.0, 0.0],
        &mut out,
        &[1e-12, 1e-12],
        Side::Left,
        config(TriggerMode::Always, 2, 2),
        &key,
        &mut live,
    )
    .unwrap();
    assert!(r.work.columns >= 1);
    if r.accepted {
        assert!((out[0] - 1.0).abs() < 1e-10)
    } else {
        assert_eq!(out, [44.0, 44.0]);
    }
}
#[test]
fn wrong_projected_zero_budget_never_commits() {
    let m = model(8);
    let key = BindingSnapshot::from_model(&m);
    let mut live = || key.clone();
    let mut out = vec![55.0; 8];
    let r = solve_preconditioned(
        &m,
        &b(8),
        &[0.0; 8],
        &mut out,
        &[1.0; 8],
        Side::Right,
        PrefixConfig {
            max_columns: 1,
            restart: 1,
            tau_orig: 0.0,
            stage_budget: 0.0,
            mode: TriggerMode::Always,
        },
        &key,
        &mut live,
    )
    .unwrap();
    assert!(!r.accepted);
    assert!(r.work.refused >= 1);
    assert_eq!(out, vec![55.0; 8]);
}
#[test]
fn stale_epoch_refuses_transactionally() {
    let m = model(4);
    let key = BindingSnapshot::from_model(&m);
    let count = Cell::new(0);
    let mut live = || {
        count.set(count.get() + 1);
        let mut now = key.clone();
        if count.get() > 3 {
            now.epoch += 1
        };
        now
    };
    let mut out = vec![33.0; 4];
    let result = solve_preconditioned(
        &m,
        &b(4),
        &[0.0; 4],
        &mut out,
        &inv(&m),
        Side::Right,
        config(TriggerMode::Always, 4, 4),
        &key,
        &mut live,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("identity"));
    assert_eq!(out, vec![33.0; 4]);
}
#[test]
fn source_clock_and_norm_identity_refuse() {
    let m = model(4);
    let key = BindingSnapshot::from_model(&m);
    for field in 0..3 {
        let mut wrong = key.clone();
        match field {
            0 => wrong.source_id = "other".into(),
            1 => wrong.clock_id = "other".into(),
            _ => wrong.norm_id = "other".into(),
        };
        let mut live = || key.clone();
        let mut out = vec![11.0; 4];
        assert!(
            solve_preconditioned(
                &m,
                &b(4),
                &[0.0; 4],
                &mut out,
                &inv(&m),
                Side::Right,
                config(TriggerMode::Always, 4, 4),
                &wrong,
                &mut live
            )
            .is_err()
        );
        assert_eq!(out, vec![11.0; 4]);
    }
}
#[test]
fn invalid_preconditioner_refuses_without_output_mutation() {
    let m = model(4);
    let key = BindingSnapshot::from_model(&m);
    for d in [0.0, f64::NAN, f64::INFINITY] {
        let mut diag = inv(&m);
        diag[1] = d;
        let mut out = vec![9.0; 4];
        let mut live = || key.clone();
        assert!(
            solve_preconditioned(
                &m,
                &b(4),
                &[0.0; 4],
                &mut out,
                &diag,
                Side::Left,
                config(TriggerMode::Always, 4, 4),
                &key,
                &mut live
            )
            .is_err()
        );
        assert_eq!(out, vec![9.0; 4]);
    }
}
#[test]
fn invalid_logarithmic_margin_refuses() {
    let mut m = model(2);
    m.jacobian[0] = 10.0;
    m.sigma = 0.2;
    let key = BindingSnapshot::from_model(&m);
    let mut live = || key.clone();
    let mut out = [8.0, 8.0];
    assert!(
        solve_preconditioned(
            &m,
            &[1.0, 1.0],
            &[0.0, 0.0],
            &mut out,
            &[1.0, 1.0],
            Side::Right,
            config(TriggerMode::Always, 2, 2),
            &key,
            &mut live
        )
        .is_err()
    );
    assert_eq!(out, [8.0, 8.0]);
}
#[test]
fn warm_start_certified_without_arnoldi() {
    let m = model(4);
    let x: Vec<f64> = b(4).iter().zip(inv(&m)).map(|(b, d)| b * d).collect();
    let key = BindingSnapshot::from_model(&m);
    let mut live = || key.clone();
    let mut out = vec![0.0; 4];
    let r = solve_preconditioned(
        &m,
        &b(4),
        &x,
        &mut out,
        &inv(&m),
        Side::Right,
        config(TriggerMode::EndOnly, 4, 4),
        &key,
        &mut live,
    )
    .unwrap();
    assert!(r.accepted);
    assert_eq!(r.work.columns, 0);
    assert_eq!(out, x);
}
#[test]
fn nonnormal_upper_triangular_both_sides() {
    let mut m = model(3);
    m.jacobian[1] = 0.125;
    m.jacobian[2] = -0.0625;
    m.jacobian[5] = 0.25;
    let key = BindingSnapshot::from_model(&m);
    for side in [Side::Left, Side::Right] {
        let mut live = || key.clone();
        let mut out = vec![0.0; 3];
        let r = solve_preconditioned(
            &m,
            &b(3),
            &[0.0; 3],
            &mut out,
            &inv(&m),
            side,
            config(TriggerMode::Always, 3, 3),
            &key,
            &mut live,
        )
        .unwrap();
        assert!(r.accepted);
        assert!(r.certificate.unwrap().forward_upper < 1e-10);
    }
}
#[test]
fn restart_continues_without_false_accept() {
    let m = model(16);
    let key = BindingSnapshot::from_model(&m);
    let mut live = || key.clone();
    let mut out = vec![0.0; 16];
    let r = solve_preconditioned(
        &m,
        &b(16),
        &[0.0; 16],
        &mut out,
        &[1.0; 16],
        Side::Right,
        config(TriggerMode::EndOnly, 3, 24),
        &key,
        &mut live,
    )
    .unwrap();
    assert!(r.accepted);
    assert!(r.work.restarts >= 1);
    assert!(r.certificate.unwrap().forward_upper < 1e-10);
}
#[test]
fn zero_shift_both_sides() {
    let mut m = model(2);
    m.sigma = 0.0;
    for side in [Side::Left, Side::Right] {
        let (out, r) = solve(&m, side, &[0.5, 2.0], config(TriggerMode::Always, 2, 2));
        assert!(r.accepted);
        let rhs = b(2);
        for (i, &xi) in out.iter().enumerate() {
            assert!((xi - rhs[i]).abs() < 1e-10);
        }
    }
}
#[test]
fn underflowed_left_preconditioned_residual_cannot_be_success() {
    let mut m = model(2);
    m.sigma = 0.0;
    let key = BindingSnapshot::from_model(&m);
    let mut live = || key.clone();
    let mut out = [77.0, 77.0];
    let r = solve_preconditioned(
        &m,
        &[1.0, 0.0],
        &[0.0, 0.0],
        &mut out,
        &[1e-300, 1e-300],
        Side::Left,
        config(TriggerMode::Always, 2, 2),
        &key,
        &mut live,
    )
    .unwrap();
    assert!(!r.accepted);
    assert_eq!(out, [77.0, 77.0]);
}
#[test]
fn changed_epoch_at_final_commit_refuses() {
    let m = model(2);
    let key = BindingSnapshot::from_model(&m);
    let count = Cell::new(0);
    let mut live = || {
        count.set(count.get() + 1);
        let mut now = key.clone();
        if count.get() >= 5 {
            now.epoch += 1;
        }
        now
    };
    let mut out = [99.0, 99.0];
    let result = solve_preconditioned(
        &m,
        &b(2),
        &[0.0, 0.0],
        &mut out,
        &inv(&m),
        Side::Right,
        config(TriggerMode::Always, 2, 2),
        &key,
        &mut live,
    );
    assert!(result.is_err());
    assert_eq!(out, [99.0, 99.0]);
}
#[test]
fn refusal_at_restart_does_not_mutate_output() {
    let m = model(8);
    let key = BindingSnapshot::from_model(&m);
    let mut live = || key.clone();
    let mut out = [66.0; 8];
    let r = solve_preconditioned(
        &m,
        &b(8),
        &[0.0; 8],
        &mut out,
        &[1.0; 8],
        Side::Left,
        PrefixConfig {
            max_columns: 6,
            restart: 2,
            tau_orig: 0.0,
            stage_budget: 0.0,
            mode: TriggerMode::EndOnly,
        },
        &key,
        &mut live,
    )
    .unwrap();
    assert!(!r.accepted);
    assert!(r.work.restarts >= 1);
    assert_eq!(out, [66.0; 8]);
}
