use rodas5p_core::{LinearMethod, LinearOperator, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::collective_direct::{CollectiveDirectWork, CollectiveStageFactors};
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::{build_step_context_matrix_free, sequential_stages};

fn model(n: usize, chi: f64) -> CollectiveModel {
    let weights = vec![1.0 / n as f64; n];
    let xi = (0..n)
        .map(|j| {
            if n == 1 {
                0.0
            } else {
                -1.0 + 2.0 * j as f64 / (n - 1) as f64
            }
        })
        .collect();
    let z = (0..n)
        .map(|j| (0.15 + 0.01 * j as f64, -0.08 + 0.005 * j as f64))
        .collect();
    CollectiveModel::new(xi, weights, z, 0.7, chi, 0.0, 0.12).unwrap()
}

#[test]
fn woodbury_uses_actual_sequential_first_stage_rhs_without_shift_scaling() {
    // For this autonomous scalar damping member, f(y0)=(-1/2,0), J=-I,
    // r0=h*f(y0)=(-1/8,0), and W=I-h*gamma*J=(1+s)I.
    let model =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.5, 0.0)], 0.0, 0.0, 0.0, 1.0).unwrap();
    let problem = model.to_ode_problem().unwrap();
    let y0 = [0.5, 0.0];
    let h = 0.25;
    let mut stage_work = WorkCounters::default();
    let context = build_step_context_matrix_free(&problem, 0.0, &y0, h, &mut stage_work).unwrap();
    assert_eq!(context.f0, vec![-0.5, 0.0]);
    assert_eq!(context.ft0, vec![0.0, 0.0]);
    let config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        ..LinearSolverConfig::default()
    };
    let sequential = sequential_stages(&context, &config, None, &mut stage_work).unwrap();
    let first_rhs = [h * context.f0[0], h * context.f0[1]];
    assert_eq!(first_rhs, [-0.125, 0.0]);
    let s = context.shifted.h_gamma();
    assert!(s > 0.0 && s < 1.0);
    let mut direct_work = CollectiveDirectWork::default();
    let factors = CollectiveStageFactors::factorize(&model, &y0, s, &mut direct_work).unwrap();
    let mut direct = [0.0; 2];
    factors
        .solve_into(&first_rhs, &mut direct, &mut direct_work)
        .unwrap();
    for (a, b) in direct.iter().zip(&sequential.stages[0]) {
        assert!(
            (a - b).abs() < 1e-13,
            "stage/direct mismatch {a:e} vs {b:e}"
        );
    }
    let mut image = [0.0; 2];
    context.shifted.apply(&direct, &mut image).unwrap();
    assert!((image[0] - first_rhs[0]).abs() < 1e-14);
    assert!((image[1] - first_rhs[1]).abs() < 1e-14);

    // The mistaken W=I/(h gamma)-J convention would send s*r0. That solve
    // must fail the residual gate for the actual shifted operator.
    let wrong_rhs = [s * first_rhs[0], s * first_rhs[1]];
    let mut wrong = [0.0; 2];
    factors
        .solve_into(&wrong_rhs, &mut wrong, &mut direct_work)
        .unwrap();
    context.shifted.apply(&wrong, &mut image).unwrap();
    let wrong_residual = first_rhs[0] - image[0];
    assert!((wrong_residual - (1.0 - s) * first_rhs[0]).abs() < 1e-14);
    assert!(wrong_residual.abs() > 0.1);
    let witness = factors
        .residual_witness(&model, &y0, &first_rhs, &wrong, &mut direct_work)
        .unwrap();
    assert!(witness.weighted_residual_upper >= wrong_residual.abs());
    assert!(direct_work.setup_ops > 0 && direct_work.rhs_ops > 0);
}

fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for k in 0..n {
        let p = (k..n)
            .max_by(|&i, &j| a[i][k].abs().total_cmp(&a[j][k].abs()))
            .unwrap();
        a.swap(k, p);
        b.swap(k, p);
        let d = a[k][k];
        for value in a[k].iter_mut().skip(k) {
            *value /= d;
        }
        b[k] /= d;
        let pivot_row = a[k].clone();
        for i in 0..n {
            if i != k {
                let q = a[i][k];
                for (j, value) in a[i].iter_mut().enumerate().skip(k) {
                    *value -= q * pivot_row[j];
                }
                b[i] -= q * b[k];
            }
        }
    }
    b
}

#[test]
fn woodbury_solve_matches_dense_frozen_stage_and_residual_witness() {
    for n in [1, 2, 4, 8, 16, 32, 64] {
        let m = model(n, 0.18);
        let dim = 2 * n;
        let mut y = vec![0.0; dim];
        m.initial_state(&mut y).unwrap();
        let s = 0.37;
        let mut matrix = vec![vec![0.0; dim]; dim];
        let mut basis = vec![0.0; dim];
        let mut jv = vec![0.0; dim];
        for col in 0..dim {
            basis.fill(0.0);
            basis[col] = 1.0;
            m.jvp(0.0, &y, &basis, &mut jv).unwrap();
            for row in 0..dim {
                matrix[row][col] = (if row == col { 1.0 } else { 0.0 }) - s * jv[row];
            }
        }
        let rhs = (0..dim)
            .map(|i| ((i * 17 + 5) as f64).sin() * 0.2)
            .collect::<Vec<_>>();
        let dense = solve_dense(matrix.clone(), rhs.clone());
        let mut out = vec![0.0; dim];
        let mut work = CollectiveDirectWork::default();
        let factors = CollectiveStageFactors::factorize(&m, &y, s, &mut work).unwrap();
        let det = factors.schur_determinant_enclosure();
        assert!(
            det[1] < 0.0 || det[0] > 0.0,
            "N={n} Schur determinant enclosure contains zero: {det:?}"
        );
        factors.solve_into(&rhs, &mut out, &mut work).unwrap();
        let err = out
            .iter()
            .zip(&dense)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        assert!(err < 2e-11, "N={n} dense discrepancy {err:e}");
        let cert = factors
            .residual_witness(&m, &y, &rhs, &out, &mut work)
            .unwrap();
        let weighted = out
            .iter()
            .zip(&dense)
            .enumerate()
            .map(|(i, (a, b))| m.weights()[i / 2] * (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        assert!(
            weighted <= cert.solution_error_upper + 1e-14,
            "N={n} error {weighted:e} exceeds {}",
            cert.solution_error_upper
        );
        assert_eq!(work.rhs_ops, 16 * n as u64 + 18);
        assert!(
            work.setup_ops > 0 && work.schur_certificate_ops > 0 && work.reconstruction_ops > 0
        );
    }
}

#[test]
fn refuses_when_wb2_inverse_witness_is_unavailable() {
    let m = model(2, 1e6);
    let mut y = vec![0.0; 4];
    m.initial_state(&mut y).unwrap();
    let mut work = CollectiveDirectWork::default();
    let result = CollectiveStageFactors::factorize(&m, &y, 1.0, &mut work);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("WB2 inverse bound unavailable")
    );
}

#[test]
fn refuses_nonzero_carrier_metadata_and_nonexact_weight_normalization() {
    let carrier = CollectiveModel::new(
        vec![-0.5, 0.5],
        vec![0.5, 0.5],
        vec![(0.1, 0.0), (0.0, 0.1)],
        0.7,
        0.1,
        0.25,
        0.0,
    )
    .unwrap();
    let y = [0.1, 0.0, 0.0, 0.1];
    let mut work = CollectiveDirectWork::default();
    assert!(
        CollectiveStageFactors::factorize(&carrier, &y, 0.1, &mut work)
            .unwrap_err()
            .to_string()
            .contains("carrier=0")
    );

    let inexact_sum = CollectiveModel::new(
        vec![-0.5, 0.5],
        vec![0.5, 0.5000000000001],
        vec![(0.1, 0.0), (0.0, 0.1)],
        0.7,
        0.1,
        0.0,
        0.0,
    )
    .unwrap();
    assert!(
        CollectiveStageFactors::factorize(&inexact_sum, &y, 0.1, &mut work)
            .unwrap_err()
            .to_string()
            .contains("exact represented weight sum 1")
    );
}

#[test]
fn reviewer_regression_residual_uses_candidate_mean() {
    let model =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.5, 0.0)], 0.0, 1.0, 0.0, 0.0).unwrap();
    let y = [0.5, 0.0];
    let rhs = [0.0, -0.0375];
    let x = [0.0, 0.0];
    let mut work = CollectiveDirectWork::default();
    let factors = CollectiveStageFactors::factorize(&model, &y, 0.1, &mut work).unwrap();
    let witness = factors
        .residual_witness(&model, &y, &rhs, &x, &mut work)
        .unwrap();
    assert!(witness.weighted_residual_upper >= 0.0375);
}

#[test]
fn residual_witness_refuses_mismatched_source_identity_without_indexing() {
    let source = model(2, 0.1);
    let mut y = vec![0.0; 4];
    source.initial_state(&mut y).unwrap();
    let mut work = CollectiveDirectWork::default();
    let factors = CollectiveStageFactors::factorize(&source, &y, 0.2, &mut work).unwrap();
    let rhs = [0.1, 0.2, -0.2, 0.3];
    let x = [0.01, 0.02, -0.01, 0.03];
    let other = model(2, 0.2);
    let err = factors
        .residual_witness(&other, &y, &rhs, &x, &mut work)
        .unwrap_err();
    assert!(err.to_string().contains("identity differs"));

    let shorter =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.1, 0.0)], 0.7, 0.1, 0.0, 0.12).unwrap();
    let err = factors
        .residual_witness(&shorter, &y, &rhs, &x, &mut work)
        .unwrap_err();
    assert!(err.to_string().contains("model dimension differs"));
}
