//! Contracts of `solve_lgmres_into` (research node
//! `research/rev04_lgmres_into_20261003`): bit for bit the existing LGMRES on
//! success, the same rollback on failure, output written only on success.

use rodas5p_core::{DenseMatrix, DenseOperator, IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{
    LgmresConfig, LgmresIntoWorkspace, LgmresState, LgmresWorkspace, solve_lgmres_into,
    solve_lgmres_with_workspace_and_residual_scale,
};

fn operator(n: usize, shift: f64) -> DenseOperator {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 2.0 + shift + 0.05 * i as f64;
        if i > 0 {
            a[(i, i - 1)] = -1.2;
        }
        if i + 1 < n {
            a[(i, i + 1)] = -0.6;
        }
    }
    DenseOperator::new(a).unwrap()
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

fn compare(config: &LgmresConfig, corrupt: bool) {
    let n = 50;
    let pc = IdentityPreconditioner::new(n);
    let (mut so, mut sn) = (LgmresState::default(), LgmresState::default());
    let (mut wo, mut wn) = (LgmresWorkspace::default(), LgmresIntoWorkspace::default());
    for k in 0..8 {
        if corrupt && k == 3 {
            so.directions.push(vec![1.0; n + 1]);
            sn.directions.push(vec![1.0; n + 1]);
        }
        let op = operator(n, 0.02 * (k / 3) as f64);
        let b: Vec<f64> = (0..n).map(|i| ((i * (k + 2)) % 9) as f64 - 4.0).collect();
        let before = so.clone();
        let (mut co, mut cn) = (WorkCounters::default(), WorkCounters::default());
        let old = solve_lgmres_with_workspace_and_residual_scale(
            &op, &pc, &b, None, config, &mut so, None, &mut wo, &mut co,
        );
        let mut output = vec![-7.0; n];
        let new = solve_lgmres_into(
            &op,
            &pc,
            &b,
            None,
            config,
            &mut sn,
            None,
            &mut output,
            &mut wn,
            &mut cn,
        );
        match (&old, &new) {
            (Ok(o), Ok(r)) => {
                assert_eq!(bits(&o.x), bits(&output));
                assert_eq!(o.residual_norm.to_bits(), r.residual_norm.to_bits());
                assert_eq!(o.iterations, r.iterations);
            }
            (Err(_), Err(_)) => {
                assert!(output.iter().all(|v| *v == -7.0));
                assert_eq!(so, before);
            }
            _ => panic!("outcomes differ at solve {k}"),
        }
        assert_eq!(co, cn);
        assert_eq!(so, sn);
    }
}

#[test]
fn successful_solves_match_bit_for_bit() {
    compare(
        &LgmresConfig {
            inner_m: 10,
            outer_k: 3,
            ..LgmresConfig::default()
        },
        false,
    );
}

#[test]
fn failing_solves_roll_back_alike_and_leave_the_output() {
    compare(
        &LgmresConfig {
            inner_m: 4,
            max_outer: 1,
            rtol: 1e-14,
            outer_k: 3,
            ..LgmresConfig::default()
        },
        false,
    );
}

#[test]
fn a_direction_of_the_wrong_length_is_dropped_alike() {
    compare(
        &LgmresConfig {
            inner_m: 10,
            outer_k: 3,
            ..LgmresConfig::default()
        },
        true,
    );
}

#[test]
fn a_wrong_output_length_is_rejected() {
    let n = 10;
    let op = operator(n, 0.0);
    let result = solve_lgmres_into(
        &op,
        &IdentityPreconditioner::new(n),
        &vec![1.0; n],
        None,
        &LgmresConfig::default(),
        &mut LgmresState::default(),
        None,
        &mut vec![0.0; n - 1],
        &mut LgmresIntoWorkspace::default(),
        &mut WorkCounters::default(),
    );
    assert!(result.is_err());
}
