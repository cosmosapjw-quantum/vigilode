//! Contracts of `LgmresIntoWorkspace::set_least_squares_once` (research node
//! `research/spd05_lgmres_ls_once_20261007`): bit for bit the default
//! LGMRES-into (and so the legacy LGMRES) on success and on failure, with and
//! without SPD04's least-squares workspace; one least-squares solve per
//! cycle instead of one per column.

use rodas5p_core::{DenseMatrix, DenseOperator, IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{LgmresConfig, LgmresIntoWorkspace, LgmresState, solve_lgmres_into};

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

fn compare(config: &LgmresConfig, ls_workspace: bool) -> (u64, u64) {
    let n = 50;
    let pc = IdentityPreconditioner::new(n);
    let (mut s_default, mut s_once) = (LgmresState::default(), LgmresState::default());
    let mut w_default = LgmresIntoWorkspace::default();
    let mut w_once = LgmresIntoWorkspace::default();
    w_once.set_least_squares_once(true);
    w_once.set_ls_workspace(ls_workspace);
    assert!(!w_default.least_squares_once());
    let (mut solves_default, mut solves_once) = (0, 0);
    let mut failures = 0;
    for k in 0..8 {
        let op = operator(n, 0.02 * (k / 3) as f64);
        let b: Vec<f64> = (0..n).map(|i| ((i * (k + 2)) % 9) as f64 - 4.0).collect();
        let before = s_default.clone();
        let (mut c_default, mut c_once) = (WorkCounters::default(), WorkCounters::default());
        let (mut o_default, mut o_once) = (vec![-7.0; n], vec![-7.0; n]);
        let r_default = solve_lgmres_into(
            &op,
            &pc,
            &b,
            None,
            config,
            &mut s_default,
            None,
            &mut o_default,
            &mut w_default,
            &mut c_default,
        );
        let r_once = solve_lgmres_into(
            &op,
            &pc,
            &b,
            None,
            config,
            &mut s_once,
            None,
            &mut o_once,
            &mut w_once,
            &mut c_once,
        );
        match (&r_default, &r_once) {
            (Ok(d), Ok(o)) => {
                assert_eq!(bits(&o_default), bits(&o_once));
                assert_eq!(d.residual_norm.to_bits(), o.residual_norm.to_bits());
                assert_eq!(d.iterations, o.iterations);
                assert_eq!(d.matvecs, o.matvecs);
                assert_eq!(d.inner_iterations, o.inner_iterations);
                assert_eq!(d.inner_iterations, d.iterations);
                // One solve per column against one per cycle.
                assert_eq!(d.least_squares_solves, d.inner_iterations);
                assert!(o.least_squares_solves < d.least_squares_solves);
                solves_default += d.least_squares_solves;
                solves_once += o.least_squares_solves;
            }
            (Err(d), Err(o)) => {
                failures += 1;
                assert_eq!(d.to_string(), o.to_string());
                assert!(o_once.iter().all(|v| *v == -7.0));
                assert_eq!(s_once, before);
            }
            _ => panic!("outcomes differ at solve {k}"),
        }
        assert_eq!(c_default, c_once);
        assert_eq!(s_default, s_once);
    }
    if ls_workspace {
        let ls = w_once.ls_workspace().expect("workspace on");
        // Failed solves also solve once per cycle but report nothing.
        if failures == 0 {
            assert_eq!(ls.solves(), solves_once);
        } else {
            assert!(ls.solves() >= failures);
        }
    }
    (solves_default, solves_once)
}

#[test]
fn successful_solves_match_bit_for_bit() {
    for ls_workspace in [false, true] {
        let (d, o) = compare(
            &LgmresConfig {
                inner_m: 10,
                outer_k: 3,
                ..LgmresConfig::default()
            },
            ls_workspace,
        );
        assert!(o > 0 && o < d);
    }
}

#[test]
fn failing_solves_roll_back_alike_and_leave_the_output() {
    for ls_workspace in [false, true] {
        compare(
            &LgmresConfig {
                inner_m: 4,
                max_outer: 1,
                rtol: 1e-14,
                outer_k: 3,
                ..LgmresConfig::default()
            },
            ls_workspace,
        );
    }
}
