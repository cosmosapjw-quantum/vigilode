//! Structure-aware ideal-matrix witness and real-driver work/fallback contracts.
use rodas5p_core::{DenseMatrix, LuFactorization};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, Rodas5pLaggedOptions, StageSolver,
    StalenessRule, integrate_rodas5p_lagged_observed, rodas5p_diagonal_staleness_bound,
};
use std::sync::Arc;

fn explicit_t_norm(h: f64, hp: f64, g: f64, j: &DenseMatrix, jp: &DenseMatrix, s: &[f64]) -> f64 {
    let n = s.len();
    let mut p = DenseMatrix::zeros(n, n);
    let mut w = p.clone();
    for i in 0..n {
        for k in 0..n {
            p[(i, k)] = -jp[(i, k)];
            w[(i, k)] = -j[(i, k)];
        }
        p[(i, i)] += 1. / (hp * g);
        w[(i, i)] += 1. / (h * g);
    }
    let lu = LuFactorization::new(&p).unwrap();
    let mut t = DenseMatrix::identity(n);
    for k in 0..n {
        let x = lu
            .solve(&(0..n).map(|i| w[(i, k)]).collect::<Vec<_>>())
            .unwrap();
        for i in 0..n {
            t[(i, k)] -= x[i];
        }
    }
    (0..n)
        .map(|i| (0..n).map(|k| t[(i, k)].abs() * s[k] / s[i]).sum::<f64>())
        .fold(0., f64::max)
}

#[test]
fn diagonal_witness_encloses_row_bound_and_refuses_unproved_structure() {
    let jp = DenseMatrix::from_rows(&[&[-1., 0.], &[0., -4.]]).unwrap();
    let j = DenseMatrix::from_rows(&[&[-1.25, 0.5], &[0.25, -3.]]).unwrap();
    for h in [0.01, 0.5, 2.] {
        for ratio in [0.5, 0.8, 1., 1.25] {
            let hp = h * ratio;
            let q = rodas5p_diagonal_staleness_bound(h, hp, 0.25, &j, &jp, &[1., 2.]);
            let actual = explicit_t_norm(h, hp, 0.25, &j, &jp, &[1., 2.]);
            assert!(
                q.is_finite() && q >= actual - 2e-15,
                "actual={actual} q={q}"
            );
        }
    }
    // Entire fixture is exactly dyadic: hp=h=gamma=1 => T=diag(1/2,1/5)(J-Jp).
    let q = rodas5p_diagonal_staleness_bound(1., 1., 1., &j, &jp, &[1., 2.]);
    assert!((0.625..=0.625 + 2e-15).contains(&q));
    let bad = DenseMatrix::from_rows(&[&[-1., 1.], &[0., -4.]]).unwrap();
    assert_eq!(
        rodas5p_diagonal_staleness_bound(1., 1., 1., &j, &bad, &[1., 2.]),
        f64::INFINITY
    );
    let positive = DenseMatrix::from_rows(&[&[1., 0.], &[0., -4.]]).unwrap();
    assert_eq!(
        rodas5p_diagonal_staleness_bound(1., 1., 1., &j, &positive, &[1., 2.]),
        f64::INFINITY
    );
    assert_eq!(
        rodas5p_diagonal_staleness_bound(1., 1., 1., &j, &jp, &[0., 2.]),
        f64::INFINITY
    );
    assert_eq!(
        rodas5p_diagonal_staleness_bound(1., f64::MAX, f64::MAX, &j, &jp, &[1., 2.]),
        f64::INFINITY
    );
    // No NaN masking with extreme finite weights (the previous B1c bug).
    let q = rodas5p_diagonal_staleness_bound(1., 1., 1., &j, &jp, &[f64::MIN_POSITIVE, f64::MAX]);
    assert_eq!(q, f64::INFINITY);
}

fn problem(offdiag: f64) -> OdeProblem {
    OdeProblem::new(
        "n05-diagonal",
        2,
        Arc::new(move |_, y, out| {
            out[0] = -y[0] + offdiag * y[1];
            out[1] = -4. * y[1];
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, _| {
            DenseMatrix::from_rows(&[&[-1., offdiag], &[0., -4.]])
        })),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}
#[test]
fn diagonal_rule_has_no_inverse_construction_solves_and_accounts_structure_scans() {
    let p = problem(0.);
    for stage_solver in [StageSolver::Richardson, StageSolver::Gmres] {
        let r = integrate_rodas5p_lagged_observed(
            &p,
            (0., 1.),
            &[1., 1.],
            &AdaptiveStepConfig {
                initial_step: 0.01,
                max_step: 0.1,
                atol: 1e-8,
                rtol: 1e-6,
                ..Default::default()
            },
            &OutputSchedule::new(vec![0., 1.]).unwrap(),
            Rodas5pLaggedOptions {
                stage_solver,
                staleness: StalenessRule::Diagonal,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(r.fast.observed.success && r.staleness.diagonal_checks > 0);
        assert_eq!(r.staleness.diagonal_checks, r.staleness.checks);
        assert_eq!(
            r.staleness.diagonal_matrix_entries,
            4 * r.staleness.diagonal_checks
        );
        assert_eq!(r.staleness.diagonal_refusals, 0);
        assert_eq!(r.staleness.resolvent_solves, 0);
        assert_eq!(
            r.fast.observed.counters.direct_solve_calls,
            r.work.preconditioner_applications
        );
        let y = r.fast.observed.y.last().unwrap();
        assert!((y[0] - (-1_f64).exp()).abs() < 1e-6);
        assert!((y[1] - (-4_f64).exp()).abs() < 1e-6);
    }
}
#[test]
fn unsupported_stored_structure_falls_back_to_direct_refresh_and_counts_refusals() {
    let p = problem(0.5);
    let r = integrate_rodas5p_lagged_observed(
        &p,
        (0., 0.3),
        &[1., 1.],
        &AdaptiveStepConfig {
            initial_step: 0.01,
            max_step: 0.03,
            atol: 1e-8,
            rtol: 1e-6,
            ..Default::default()
        },
        &OutputSchedule::new(vec![0., 0.3]).unwrap(),
        Rodas5pLaggedOptions {
            stage_solver: StageSolver::Gmres,
            staleness: StalenessRule::Diagonal,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(r.fast.observed.success && r.staleness.diagonal_refusals > 0);
    assert_eq!(r.staleness.diagonal_checks, r.staleness.diagonal_refusals);
    assert_eq!(
        r.staleness.diagonal_matrix_entries,
        2 * r.staleness.diagonal_refusals
    );
    assert_eq!(
        r.staleness.staleness_refreshes,
        r.staleness.diagonal_refusals
    );
    assert_eq!(r.staleness.resolvent_solves, 0);
    assert_eq!(r.work.gmres_iterations, 0);
}
