//! New-domain and all-stage witnesses for A06. Exact dyadic fixtures below
//! are independent of the native factorization and use no empirical slack.
use rodas5p_core::WorkCounters;
use rodas5p_integrators::collective_direct::{CollectiveDirectWork, CollectiveStageFactors};
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::collective_rodas5p::CollectiveRodas5pWorkspace;

fn identical(n: usize, omega: f64) -> CollectiveModel {
    CollectiveModel::new(
        vec![omega / omega.abs().max(1.0); n],
        vec![1.0 / n as f64; n],
        vec![(1.0, 0.0); n],
        omega.abs().max(1.0),
        1.0,
        0.0,
        0.0,
    )
    .unwrap()
}

#[test]
fn non_neumann_inverse_covers_exact_dyadic_solutions_at_native_sizes() {
    for n in [1, 2, 4, 8, 64, 256] {
        let model = identical(n, 0.0);
        let mut y = vec![0.0; 2 * n];
        model.initial_state(&mut y).unwrap();
        let rhs: Vec<f64> = (0..2 * n).map(|i| ((i % 7) as f64 - 3.0) * 0.125).collect();
        let mut work = CollectiveDirectWork::default();
        // ||s C|| = 3 > 1, so the old WB2-only path refused this matrix.
        let factors = CollectiveStageFactors::factorize(&model, &y, 1.0, &mut work).unwrap();
        let mut out = vec![0.0; 2 * n];
        factors.solve_into(&rhs, &mut out, &mut work).unwrap();
        let mean = (0..n).fold([0.0; 2], |mut m, j| {
            m[0] += rhs[2 * j] / n as f64;
            m[1] += rhs[2 * j + 1] / n as f64;
            m
        });
        // Exact inverse of S=[[1,1],[-3,1]]; every operation here is dyadic.
        let q = [(mean[0] - mean[1]) * 0.25, (3.0 * mean[0] + mean[1]) * 0.25];
        let exact: Vec<f64> = (0..2 * n)
            .map(|i| rhs[i] + if i % 2 == 0 { -q[1] } else { 3.0 * q[0] })
            .collect();
        out[0] = out[0].next_up();
        let witness = factors
            .residual_witness(&model, &y, &rhs, &out, &mut work)
            .unwrap();
        let actual_error = out
            .iter()
            .zip(&exact)
            .map(|(x, z)| (x - z).powi(2) / n as f64)
            .sum::<f64>()
            .sqrt();
        assert!(
            actual_error <= witness.solution_error_upper,
            "n={n}: {actual_error:e} vs {witness:?}"
        );
        assert!(witness.inverse_norm_upper >= 0.75);
        assert_eq!(work.residual_witness_calls, 1);
        assert_eq!(work.inverse_certificate_ops, 48);
    }
}

#[test]
fn exactly_singular_schur_refuses_and_invalidates_reusable_factors() {
    let good = identical(4, 0.0);
    let singular = identical(4, -2.0);
    let y = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
    let mut work = CollectiveDirectWork::default();
    let mut factors = CollectiveStageFactors::factorize(&good, &y, 1.0, &mut work).unwrap();
    // On constant modes J=[[0,1],[1,0]], so I-J has an exact nullvector.
    let err = factors.refactor(&singular, &y, 1.0, &mut work).unwrap_err();
    assert!(
        err.to_string()
            .contains("Schur determinant interval contains zero")
    );
    assert!(!factors.is_valid());
    let mut out = [7.0; 8];
    assert!(factors.solve_into(&[1.0; 8], &mut out, &mut work).is_err());
    assert_eq!(out, [7.0; 8]);
    assert!(work.schur_certificate_ops > 0);
}

#[test]
fn opt_in_witnesses_preserve_every_stage_and_charge_eight_interval_passes() {
    for n in [1, 2, 4, 8, 64] {
        let model = identical(n, 0.25);
        let mut y = vec![0.0; 2 * n];
        model.initial_state(&mut y).unwrap();
        let mut plain = CollectiveRodas5pWorkspace::new(model.clone()).unwrap();
        let mut checked = CollectiveRodas5pWorkspace::new(model).unwrap();
        let mut a = vec![0.0; 2 * n];
        let mut b = vec![0.0; 2 * n];
        let mut ca = WorkCounters::default();
        let mut cb = WorkCounters::default();
        let ea = plain
            .step_into(0.0, &y, 0.02, 1e-10, 1e-7, &mut a, &mut ca)
            .unwrap();
        let eb = checked
            .step_into_with_residual_witnesses(0.0, &y, 0.02, 1e-10, 1e-7, &mut b, &mut cb)
            .unwrap();
        assert_eq!(ea.to_bits(), eb.to_bits());
        assert_eq!(a, b);
        for stage in 0..8 {
            assert_eq!(plain.stage(stage), checked.stage(stage));
            assert!(plain.stage_residual_witness(stage).is_none());
            let witness = checked.stage_residual_witness(stage).unwrap();
            assert!(
                witness.solution_error_upper.is_finite() && witness.solution_error_upper < 1e-10
            );
        }
        assert_eq!(checked.linear_work().residual_witness_calls, 8);
        assert_eq!(plain.linear_work().residual_witness_calls, 0);
        assert!(checked.linear_work().reconstruction_ops > plain.linear_work().reconstruction_ops);
        assert_eq!(cb.direct_factorizations, 1);
        assert_eq!(cb.direct_solve_calls, 8);
        let before = b.clone();
        assert!(
            checked
                .step_into_with_residual_witnesses(0.0, &y, -1.0, 1e-10, 1e-7, &mut b, &mut cb)
                .is_err()
        );
        assert_eq!(before, b);
        assert!(checked.stage_residual_witness(0).is_none());
        checked
            .step_into(0.0, &y, 0.02, 1e-10, 1e-7, &mut b, &mut cb)
            .unwrap();
        assert!(checked.stage_residual_witness(0).is_none());
    }
}
