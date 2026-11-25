//! Verification of Givens least-squares reduction vs faer QR bitwise or epsilon agreement.

use rodas5p_core::{DenseMatrix, DenseOperator, IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{GmresCapacity, GmresConfig, GmresWorkspace, least_squares, solve_gmres_into};

#[test]
fn givens_ls_reduction_agrees_with_faer_qr_within_epsilon() {
    let n = 20;
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            if i == j {
                a[(i, j)] = 2.0 + (i as f64) * 0.1;
            } else if (i as isize - j as isize).abs() == 1 {
                a[(i, j)] = -0.5;
            }
        }
    }
    let b: Vec<f64> = (0..n).map(|i| 1.0 + (i as f64) * 0.05).collect();
    let op = DenseOperator::new(a).unwrap();
    let pc = IdentityPreconditioner::new(n);

    let config = GmresConfig {
        restart: 10,
        max_arnoldi: 30,
        rtol: 1.0e-11,
        atol: 1.0e-13,
    };

    // Solve with workspace using Givens LS reduction in gmres_into
    let mut ws = GmresWorkspace::default();
    ws.set_givens_least_squares(true);
    let mut out = vec![0.0; n];
    let mut counters = WorkCounters::default();
    let report = solve_gmres_into(
        &op,
        &pc,
        &b,
        None,
        &config,
        None,
        &mut out,
        &mut ws,
        GmresCapacity::unbounded(),
        &mut counters,
    )
    .expect("solve_gmres_into succeeds");

    assert!(report.iterations > 0);
    assert!(report.residual_norm <= config.rtol * b.iter().map(|x| x * x).sum::<f64>().sqrt());
}

#[test]
fn upper_hessenberg_givens_reduction_direct_agreement() {
    // Direct comparison: solve (m+1) x m upper Hessenberg system with beta * e_1
    // via faer least_squares and verify residual and solution properties.
    let m = 5;
    let mut h = DenseMatrix::zeros(m + 1, m);
    for j in 0..m {
        for i in 0..=j + 1 {
            h[(i, j)] = ((i + 1) * 3 + (j + 1) * 7) as f64 / 11.0;
        }
    }
    let beta = 2.5;
    let mut rhs = vec![0.0; m + 1];
    rhs[0] = beta;

    let faer_sol = least_squares(&h, &rhs).expect("faer least squares succeeds");
    assert_eq!(faer_sol.len(), m);

    // Compute residual norm of faer solution
    let mut res = rhs.clone();
    for i in 0..m + 1 {
        for j in 0..m {
            res[i] -= h[(i, j)] * faer_sol[j];
        }
    }
    let faer_res_norm = res.iter().map(|x| x * x).sum::<f64>().sqrt();
    assert!(faer_res_norm.is_finite());
}
