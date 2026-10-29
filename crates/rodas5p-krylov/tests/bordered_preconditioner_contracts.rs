//! Integration through the existing SP02-compatible GMRES-into API.
use rodas5p_core::bordered::BorderedBandedPreconditioner;
use rodas5p_core::{DenseMatrix, DenseOperator, WorkCounters};
use rodas5p_krylov::{
    CapacityGrowth, GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into,
};

#[test]
fn bordered_right_preconditioner_reaches_true_residual_on_exact_and_perturbed_operator() {
    let n = 16;
    let m = 2;
    let mut band = vec![0.0; 3 * n];
    let b = vec![0.125; n * m];
    let c = vec![-0.0625; n * m];
    let d = [2.0, 0.25, 0.125, 3.0];
    let mut matrix = DenseMatrix::zeros(n + m, n + m);
    for i in 0..n {
        band[3 * i + 1] = 4.0;
        matrix[(i, i)] = 4.0;
        if i > 0 {
            band[3 * i] = -0.5;
            matrix[(i, i - 1)] = -0.5;
        }
        if i + 1 < n {
            band[3 * i + 2] = 0.25;
            matrix[(i, i + 1)] = 0.25;
        }
        for j in 0..m {
            matrix[(i, n + j)] = b[i * m + j];
            matrix[(n + j, i)] = c[j * n + i];
        }
    }
    for i in 0..m {
        for j in 0..m {
            matrix[(n + i, n + j)] = d[i * m + j];
        }
    }
    let mut pc = BorderedBandedPreconditioner::new(n, m, 1, 1).unwrap();
    pc.refactor(n, m, &band, &b, &c, &d).unwrap();
    let config = GmresConfig {
        restart: 8,
        max_arnoldi: 32,
        rtol: 1e-11,
        atol: 1e-13,
    };
    let mut ws = GmresWorkspace::default();
    ws.reserve(n + m, 8).unwrap();
    for perturbation in [0.0, 0.01] {
        let mut actual = matrix.clone();
        for i in 0..n {
            actual[(i, (i + 5) % n)] += perturbation;
        }
        let exact: Vec<f64> = (0..n + m).map(|i| (i % 7) as f64 * 0.125 - 0.25).collect();
        let rhs = actual.matvec(&exact).unwrap();
        let op = DenseOperator::new(actual.clone()).unwrap();
        let mut out = vec![f64::NAN; n + m];
        let report = solve_gmres_into(
            &op,
            &pc,
            &rhs,
            None,
            &config,
            None,
            &mut out,
            &mut ws,
            GmresCapacity {
                max_dimension: n + m,
                max_columns: 8,
                growth: CapacityGrowth::Refuse,
            },
            &mut WorkCounters::default(),
        )
        .unwrap();
        let got_rhs = actual.matvec(&out).unwrap();
        let residual2 = got_rhs
            .iter()
            .zip(&rhs)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(residual2 <= 1e-10, "true residual {residual2:e}");
        assert!(!report.workspace_grew);
        assert!(out.iter().zip(&exact).all(|(a, b)| (a - b).abs() < 1e-10));
    }
}
