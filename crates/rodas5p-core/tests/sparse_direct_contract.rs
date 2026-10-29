use rodas5p_core::sparse_direct::SparseDirectSolver;
use rodas5p_core::{DenseMatrix, LuFactorization, WorkCounters};

/// Deterministic LCG pseudo-random generator (zero dependency).
struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0
    }

    fn gen_range_f64(&mut self, min: f64, max: f64) -> f64 {
        let val = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        min + val * (max - min)
    }
}

/// Compute 1-norm (maximum absolute column sum) of sparse matrix.
fn sparse_one_norm(n: usize, col_ptr: &[usize], values: &[f64]) -> f64 {
    let mut max_col = 0.0f64;
    for j in 0..n {
        let mut col_sum = 0.0f64;
        for val in &values[col_ptr[j]..col_ptr[j + 1]] {
            col_sum += val.abs();
        }
        max_col = max_col.max(col_sum);
    }
    max_col
}

/// Compute 2-norm of slice.
fn norm2(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Multiply sparse matrix A (CSC) by vector x, storing result in y.
fn sparse_matvec(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    values: &[f64],
    x: &[f64],
    y: &mut [f64],
) {
    y.fill(0.0);
    for j in 0..n {
        let xj = x[j];
        for k in col_ptr[j]..col_ptr[j + 1] {
            let i = row_idx[k];
            y[i] += values[k] * xj;
        }
    }
}

#[test]
fn test_seeded_sparse_matrices_backward_error() {
    let mut rng = SimpleRng::new(0x0C01_5EED_2026);
    let mut counters = WorkCounters::default();

    // 200 seeded sparse matrices with n from 50 to 5000 (scaled across trials)
    for trial in 0..200 {
        let n: usize = 50 + (trial * (5000 - 50)) / 199;
        // Tridiagonal / pentadiagonal diagonally dominant matrix
        let mut col_ptr = Vec::with_capacity(n + 1);
        let mut row_idx = Vec::with_capacity(3 * n);
        let mut values = Vec::with_capacity(3 * n);

        col_ptr.push(0);
        for j in 0..n {
            // subdiagonal
            if j > 0 {
                row_idx.push(j - 1);
                values.push(rng.gen_range_f64(-1.0, 1.0));
            }
            // diagonal (dominant)
            row_idx.push(j);
            values.push(10.0 + rng.gen_range_f64(0.0, 5.0));
            // superdiagonal
            if j + 1 < n {
                row_idx.push(j + 1);
                values.push(rng.gen_range_f64(-1.0, 1.0));
            }
            col_ptr.push(row_idx.len());
        }

        let mut solver = SparseDirectSolver::try_new_symbolic(n, &col_ptr, &row_idx)
            .expect("Symbolic LU must succeed on banded pattern");

        solver
            .refactorize(&col_ptr, &row_idx, &values, &mut counters)
            .expect("Numeric refactorize must succeed");

        let mut b = vec![0.0f64; n];
        for val in &mut b {
            *val = rng.gen_range_f64(-2.0, 2.0);
        }

        let mut x = vec![0.0f64; n];
        solver
            .solve(&b, &mut x, &mut counters)
            .expect("Solve must succeed");

        // Compute backward error: ||b - A x|| / (||A||_1 ||x||_2 + ||b||_2)
        let mut ax = vec![0.0f64; n];
        sparse_matvec(n, &col_ptr, &row_idx, &values, &x, &mut ax);

        let mut res = vec![0.0f64; n];
        for i in 0..n {
            res[i] = b[i] - ax[i];
        }

        let norm_res = norm2(&res);
        let norm_x = norm2(&x);
        let norm_b = norm2(&b);
        let norm_a = sparse_one_norm(n, &col_ptr, &values);

        let backward_error = norm_res / (norm_a * norm_x + norm_b);
        assert!(
            backward_error <= 1.0e-13,
            "Trial {trial} (n={n}): backward error {backward_error:.3e} exceeds 1e-13"
        );
    }

    assert_eq!(counters.direct_factorizations, 200);
    assert_eq!(counters.direct_solve_calls, 200);
}

#[test]
fn test_brusselator_1d_agreement_with_dense_lu() {
    let cells = 25;
    let n = 2 * cells;
    let alpha = 0.02;
    let dx = 1.0 / (cells as f64);
    let diff = alpha / (dx * dx);

    // Build dense matrix for Brusselator Jacobian
    let mut dense_j = DenseMatrix::zeros(n, n);
    for i in 0..cells {
        let u_idx = 2 * i;
        let v_idx = 2 * i + 1;
        let left_u = if i > 0 { 2 * (i - 1) } else { 2 * (cells - 1) };
        let right_u = if i + 1 < cells { 2 * (i + 1) } else { 0 };

        dense_j[(u_idx, u_idx)] = -2.0 * diff + 1.0;
        dense_j[(u_idx, left_u)] = diff;
        dense_j[(u_idx, right_u)] = diff;
        dense_j[(u_idx, v_idx)] = 0.5;

        let left_v = if i > 0 {
            2 * (i - 1) + 1
        } else {
            2 * (cells - 1) + 1
        };
        let right_v = if i + 1 < cells { 2 * (i + 1) + 1 } else { 1 };
        dense_j[(v_idx, v_idx)] = -2.0 * diff - 0.5;
        dense_j[(v_idx, left_v)] = diff;
        dense_j[(v_idx, right_v)] = diff;
        dense_j[(v_idx, u_idx)] = -1.0;
    }

    // W = I - h*gamma*J
    let h_gamma = 0.05 * 0.25;
    let mut w_dense = DenseMatrix::identity(n);
    for i in 0..n {
        for j in 0..n {
            w_dense[(i, j)] -= h_gamma * dense_j[(i, j)];
        }
    }

    // Convert w_dense to CSC
    let mut col_ptr = vec![0usize];
    let mut row_idx = Vec::new();
    let mut values = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let val = w_dense[(i, j)];
            if val.abs() > 1e-15 {
                row_idx.push(i);
                values.push(val);
            }
        }
        col_ptr.push(row_idx.len());
    }

    let mut counters = WorkCounters::default();
    let mut sparse_solver = SparseDirectSolver::try_new_symbolic(n, &col_ptr, &row_idx).unwrap();
    sparse_solver
        .refactorize(&col_ptr, &row_idx, &values, &mut counters)
        .unwrap();

    let dense_lu = LuFactorization::new(&w_dense).unwrap();

    let mut b = vec![0.0f64; n];
    for (i, val) in b.iter_mut().enumerate() {
        *val = ((i + 1) as f64).sin();
    }

    let mut x_sparse = vec![0.0f64; n];
    sparse_solver
        .solve(&b, &mut x_sparse, &mut counters)
        .unwrap();

    let x_dense = dense_lu.solve(&b).unwrap();

    let mut diff_vec = vec![0.0f64; n];
    for i in 0..n {
        diff_vec[i] = x_sparse[i] - x_dense[i];
    }

    let rel_err = norm2(&diff_vec) / norm2(&x_dense);
    assert!(
        rel_err <= 1.0e-12,
        "Brusselator-1D agreement relative error {rel_err:.3e} exceeds 1e-12"
    );
}
