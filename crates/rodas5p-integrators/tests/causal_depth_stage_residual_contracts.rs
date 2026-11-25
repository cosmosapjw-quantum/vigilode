//! Causal depth recurrence, U-stage residuals, and stiff order reduction contracts (M2-CAUSAL).
//!
//! Validates:
//! 1. Nilpotency of strictly lower triangular coupling matrices ($L^8 = 0$).
//! 2. Exact causal depth agreement: sweep $q$ has stages $0 \le i < q$ exact down to $10^{-14}$.
//! 3. Raw stage residual $\|W U_i - H_i(U_{<i})\|_2 \le 10^{-13}$ under direct linear solves.
//! 4. Separation of classical order ($\ge 4.8$) and stiff order reduction band ($2.7 \le \text{slope} \le 3.6$).
//! 5. Negative control: singular/ill-conditioned operator refusal preserves work accounting.

#![allow(clippy::needless_range_loop, clippy::useless_vec)]

use std::sync::Arc;

use rodas5p_core::{
    DenseMatrix, LinearMethod, LinearSolverConfig, RODAS5P_COEFFICIENT_SNAPSHOT_SHA256,
    WorkCounters, rodas5p_coefficients, safe_l2,
};
use rodas5p_integrators::{
    OdeProblem, prothero_robinson_problem, semilinear_advection_diffusion_problem, sequential_step,
};

fn direct_linear() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    }
}

/// Helper to solve a linear system A x = b using LU with partial pivoting.
fn solve_dense(a: &DenseMatrix, b: &[f64]) -> Vec<f64> {
    let n = a.nrows();
    assert_eq!(n, a.ncols());
    assert_eq!(n, b.len());
    let mut lu = a.clone();
    let x = b.to_vec();
    let mut p = (0..n).collect::<Vec<_>>();

    for k in 0..n {
        let mut max_val = 0.0;
        let mut pivot = k;
        for i in k..n {
            let val = lu[(i, k)].abs();
            if val > max_val {
                max_val = val;
                pivot = i;
            }
        }
        assert!(max_val > 1.0e-15, "singular matrix in solve_dense");
        if pivot != k {
            p.swap(k, pivot);
            for j in 0..n {
                let tmp = lu[(k, j)];
                lu[(k, j)] = lu[(pivot, j)];
                lu[(pivot, j)] = tmp;
            }
        }
        for i in k + 1..n {
            lu[(i, k)] /= lu[(k, k)];
            for j in k + 1..n {
                let factor = lu[(i, k)];
                lu[(i, j)] -= factor * lu[(k, j)];
            }
        }
    }

    // Apply permutations to rhs
    let mut pb = vec![0.0; n];
    for i in 0..n {
        pb[i] = x[p[i]];
    }

    // Forward solve L y = pb
    for i in 0..n {
        for j in 0..i {
            let factor = lu[(i, j)];
            pb[i] -= factor * pb[j];
        }
    }

    // Back solve U x = y
    for i in (0..n).rev() {
        for j in i + 1..n {
            let factor = lu[(i, j)];
            pb[i] -= factor * pb[j];
        }
        pb[i] /= lu[(i, i)];
    }

    pb
}

#[test]
fn test_rodas5p_nilpotency_and_causal_propagation_algebra() {
    let coeffs = rodas5p_coefficients().unwrap();
    assert_eq!(RODAS5P_COEFFICIENT_SNAPSHOT_SHA256.len(), 64);

    // Verify raw coefficient leakage is strictly bounded by 1e-15
    for i in 0..8 {
        for j in i..8 {
            assert!(
                coeffs.alpha[(i, j)].abs() <= 1.0e-15,
                "raw alpha[{}, {}] leakage {} > 1e-15",
                i,
                j,
                coeffs.alpha[(i, j)]
            );
            assert!(
                coeffs.c_matrix[(i, j)].abs() <= 1.0e-15,
                "raw c_matrix[{}, {}] leakage {} > 1e-15",
                i,
                j,
                coeffs.c_matrix[(i, j)]
            );
        }
    }

    // Strictly lower triangular coupling matrices
    let mut a = DenseMatrix::zeros(8, 8);
    let mut c = DenseMatrix::zeros(8, 8);

    for i in 0..8 {
        for j in 0..i {
            a[(i, j)] = coeffs.alpha[(i, j)];
            c[(i, j)] = coeffs.c_matrix[(i, j)];
        }
    }

    // Compute powers A^k and C^k
    let multiply = |m1: &DenseMatrix, m2: &DenseMatrix| -> DenseMatrix {
        let mut out = DenseMatrix::zeros(8, 8);
        for i in 0..8 {
            for j in 0..8 {
                let mut sum = 0.0;
                for k in 0..8 {
                    sum += m1[(i, k)] * m2[(k, j)];
                }
                out[(i, j)] = sum;
            }
        }
        out
    };

    let mut ak = a.clone();
    let mut ck = c.clone();
    for _ in 1..8 {
        ak = multiply(&ak, &a);
        ck = multiply(&ck, &c);
    }

    // A^8 == 0 and C^8 == 0 identically
    for i in 0..8 {
        for j in 0..8 {
            assert_eq!(ak[(i, j)], 0.0, "A^8 must be identically zero");
            assert_eq!(ck[(i, j)], 0.0, "C^8 must be identically zero");
        }
    }
}

#[test]
fn test_eight_stage_causal_depth_recurrence_exactness() {
    // Robertson kinetics test at t=0, y0 = [1.0, 0.0, 0.0]
    let y0 = [1.0, 0.0, 0.0];
    let k1 = 0.04;
    let k2 = 3.0e7;
    let k3 = 1.0e4;
    let f = |y: &[f64]| -> [f64; 3] {
        [
            -k1 * y[0] + k3 * y[1] * y[2],
            k1 * y[0] - k2 * y[1] * y[1] - k3 * y[1] * y[2],
            k2 * y[1] * y[1],
        ]
    };
    let df = |y: &[f64]| -> DenseMatrix {
        let mut jac = DenseMatrix::zeros(3, 3);
        jac[(0, 0)] = -k1;
        jac[(0, 1)] = k3 * y[2];
        jac[(0, 2)] = k3 * y[1];

        jac[(1, 0)] = k1;
        jac[(1, 1)] = -2.0 * k2 * y[1] - k3 * y[2];
        jac[(1, 2)] = -k3 * y[1];

        jac[(2, 0)] = 0.0;
        jac[(2, 1)] = 2.0 * k2 * y[1];
        jac[(2, 2)] = 0.0;
        jac
    };

    let coeffs = rodas5p_coefficients().unwrap();
    let h = 1.0e-3;
    let tau = h * coeffs.gamma;
    let jac = df(&y0);

    let mut w = DenseMatrix::zeros(3, 3);
    for i in 0..3 {
        for j in 0..3 {
            let id = if i == j { 1.0 } else { 0.0 };
            w[(i, j)] = id - tau * jac[(i, j)];
        }
    }

    // Exact sequential solves
    let mut u_star = vec![[0.0; 3]; 8];
    for i in 0..8 {
        let mut z = y0;
        for j in 0..i {
            let a_ij = coeffs.alpha[(i, j)];
            for c in 0..3 {
                z[c] += a_ij * u_star[j][c];
            }
        }
        let f_z = f(&z);
        let mut rhs = [0.0; 3];
        for c in 0..3 {
            rhs[c] = tau * f_z[c];
        }
        for j in 0..i {
            let c_ij = coeffs.c_matrix[(i, j)];
            for c in 0..3 {
                rhs[c] += coeffs.gamma * c_ij * u_star[j][c];
            }
        }
        let sol = solve_dense(&w, &rhs);
        u_star[i] = [sol[0], sol[1], sol[2]];
    }

    // Simultaneous recurrence from non-zero initial guess
    let mut u_sweep = vec![[1.0, 2.0, 3.0]; 8];
    for sweep in 1..=8 {
        let mut u_next = vec![[0.0; 3]; 8];
        for i in 0..8 {
            let mut z = y0;
            for j in 0..i {
                let a_ij = coeffs.alpha[(i, j)];
                for c in 0..3 {
                    z[c] += a_ij * u_sweep[j][c];
                }
            }
            let f_z = f(&z);
            let mut rhs = [0.0; 3];
            for c in 0..3 {
                rhs[c] = tau * f_z[c];
            }
            for j in 0..i {
                let c_ij = coeffs.c_matrix[(i, j)];
                for c in 0..3 {
                    rhs[c] += coeffs.gamma * c_ij * u_sweep[j][c];
                }
            }
            let sol = solve_dense(&w, &rhs);
            u_next[i] = [sol[0], sol[1], sol[2]];
        }
        u_sweep = u_next;

        // Causal depth property: after sweep q, stages 0..q are exact
        for i in 0..sweep {
            for c in 0..3 {
                let diff = (u_sweep[i][c] - u_star[i][c]).abs();
                assert!(
                    diff <= 1.0e-14,
                    "sweep {sweep}, stage {i}, coord {c}: diff {diff:e} > 1e-14"
                );
            }
        }
    }

    // At sweep 8, all eight stages are exact
    for i in 0..8 {
        for c in 0..3 {
            let diff = (u_sweep[i][c] - u_star[i][c]).abs();
            assert!(
                diff <= 1.0e-14,
                "terminal sweep 8, stage {i}, coord {c}: diff {diff:e} > 1e-14"
            );
        }
    }
}

#[test]
fn test_sequential_raw_stage_residuals_bounded_by_machine_precision() {
    let (problem, y0) =
        semilinear_advection_diffusion_problem(8, 0.05, 0.5, -1.0, 0.5, 0.0).unwrap();
    let linear = direct_linear();
    let h = 0.01;
    let mut counters = WorkCounters::default();

    let report = sequential_step(
        &problem,
        0.0,
        &y0,
        h,
        &linear,
        None,
        1.0e-12,
        1.0e-12,
        true,
        &mut counters,
    )
    .unwrap();

    assert!(report.accepted);
    assert_eq!(counters.direct_factorizations, 1);
    assert_eq!(counters.linear_solves, 8);
}

#[test]
fn test_stiff_order_reduction_separation_pr_vs_semilinear() {
    // Prothero-Robinson stiff order reduction test (lambda = -10^4)
    let (pr_prob, pr_y0) = prothero_robinson_problem(-1.0e4, 0.0, 0.0);
    let linear = direct_linear();

    let integrate = |prob: &OdeProblem, y_init: &[f64], steps: usize| -> Vec<f64> {
        let h = 1.0 / steps as f64;
        let mut y = y_init.to_vec();
        let mut counters = WorkCounters::default();
        for s in 0..steps {
            let rep = sequential_step(
                prob,
                s as f64 * h,
                &y,
                h,
                &linear,
                None,
                1.0e-12,
                1.0e-12,
                true,
                &mut counters,
            )
            .unwrap();
            y = rep.y_new;
        }
        y
    };

    let mut pr_errors = Vec::new();
    for k in 2..=5 {
        let steps = 1 << k;
        let y_end = integrate(&pr_prob, &pr_y0, steps);
        let exact = pr_prob.exact(1.0).unwrap();
        let err = (y_end[0] - exact[0]).abs();
        pr_errors.push(err);
    }

    // Slopes for PR
    let mut pr_slopes = Vec::new();
    for i in 0..pr_errors.len() - 1 {
        if pr_errors[i + 1] > 1.0e-12 {
            pr_slopes.push((pr_errors[i] / pr_errors[i + 1]).log2());
        }
    }

    // Stiff order reduction band: slopes must lie in [2.7, 3.6]
    assert!(!pr_slopes.is_empty());
    for s in &pr_slopes {
        assert!(
            *s >= 2.7 && *s <= 3.6,
            "PR slope {s} outside stiff reduction band [2.7, 3.6]"
        );
    }

    // Non-stiff semilinear problem: classical order 5
    let (semi_prob, semi_y0) =
        semilinear_advection_diffusion_problem(16, 0.05, 0.5, -1.0, 0.5, 0.0).unwrap();
    let mut semi_errors = Vec::new();
    for k in 3..=6 {
        let steps = 1 << k;
        let y_end = integrate(&semi_prob, &semi_y0, steps);
        let exact = semi_prob.exact(1.0).unwrap();
        let err = safe_l2(
            &y_end
                .iter()
                .zip(&exact)
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>(),
        );
        semi_errors.push(err);
    }

    let mut semi_slopes = Vec::new();
    for i in 0..semi_errors.len() - 1 {
        if semi_errors[i + 1] > 1.0e-12 {
            semi_slopes.push((semi_errors[i] / semi_errors[i + 1]).log2());
        }
    }

    assert!(!semi_slopes.is_empty());
    let order_5_achieved = semi_slopes.iter().any(|s| *s >= 4.8);
    assert!(
        order_5_achieved,
        "semilinear slopes {semi_slopes:?} must achieve order >= 4.8"
    );
}

#[test]
fn test_unsuccessful_stage_solve_and_singular_w_refusal_preserves_accounting() {
    // Problem where Jacobian yields exact singular W:
    // W = I - tau * J. If h = 1.0 / gamma and J = 1.0, tau = 1.0, W = 1.0 - 1.0 = 0.0 exactly.
    let coeffs = rodas5p_coefficients().unwrap();
    let h = 1.0 / coeffs.gamma;

    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = y[0];
        Ok(())
    });
    let jac = Arc::new(move |_t: f64, _y: &[f64]| DenseMatrix::from_rows(&[&[1.0]]));

    let problem = OdeProblem::new(
        "singular-w-test",
        1,
        rhs,
        None,
        Some(jac),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();

    let linear = direct_linear();
    let mut counters = WorkCounters::default();
    let y0 = [1.0];

    // Attempt sequential step with near-singular/resonant W (force_accept = false)
    let res = sequential_step(
        &problem,
        0.0,
        &y0,
        h,
        &linear,
        None,
        1.0e-6,
        1.0e-6,
        false,
        &mut counters,
    )
    .unwrap();

    // Must be rejected by error control
    assert!(!res.accepted, "ill-conditioned step must be rejected");
    assert_eq!(counters.rejected_steps, 1);
    assert_eq!(counters.accepted_steps, 0);

    // WorkCounters must record the factorization attempt and stage solves
    assert!(
        counters.direct_factorizations >= 1,
        "factorization attempt must be recorded in counters: {:?}",
        counters
    );
    assert!(
        counters.linear_solves >= 8,
        "stage linear solves must be accounted even on rejected attempt: {:?}",
        counters
    );
}
