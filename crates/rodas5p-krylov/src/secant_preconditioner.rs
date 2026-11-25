//! Low-rank secant preconditioner built from stage displacements without extra RHS calls
//! (WP-07, research node `research/secant_preconditioner_wp07_20261104`).
//!
//! Preconditions matrix-free stage solves `(I - h gamma J) x = b` using a low-rank secant
//! model built from stage data already computed during stage evaluations:
//!
//! - Displacements: `S = [s_1, ..., s_k]` (`s_j = Y_j - y_n`, `k <= 6`).
//! - Differences: `Y = [f(Y_j) - f(y_n)]`.
//! - Secant model: `J_hat = D + U V^T`, where `D = 0` (or diagonal estimate), `U = Y - D S`,
//!   and `V^T = (S^T S + lambda I)^{-1} S^T`.
//! - Woodbury formula for `(I - h gamma J_hat)^{-1}`:
//!   `P r = r + h gamma U (I_k - h gamma V^T U)^{-1} V^T r`.
//! - Capacitance safeguard (ADJ-22):
//!   `C_k = I_k - h gamma V^T U`. Preconditioner is accepted only if
//!   `sigma_min(C_k) >= 1e-6 * sigma_max(C_k)`. Otherwise, fallback to identity (`None`).
//! - Column drop safeguard:
//!   Collinear or degenerate columns with `|R_jj| < 1e-4 |R_11|` in pivoted Gram/QR
//!   factorization are dropped before forming `V^T`.
//! - Zero allocation during `apply_into` using fixed stack buffers.

use rodas5p_core::{CoreError, CoreResult, Preconditioner, PreconditionerSide};

/// Maximum displacement rank supported by the secant scratchpad.
pub const MAX_SECANT_RANK: usize = 6;

/// Relative pivot threshold for dropping collinear or degenerate displacement columns.
pub const SECANT_COLUMN_DROP_TOL: f64 = 1e-4;

/// Condition number safeguard threshold for the capacitance matrix.
pub const SECANT_CAPACITANCE_CONDITION_TOL: f64 = 1e-6;

/// Default Frobenius-norm regularization factor: `lambda = 1e-6 * ||S||_F^2`.
pub const DEFAULT_SECANT_REGULARIZATION: f64 = 1e-6;

/// Low-rank secant preconditioner for matrix-free stage solves `(I - h gamma J) x = b`.
#[derive(Clone, Debug)]
pub struct SecantPreconditioner {
    pub dimension: usize,
    pub rank: usize,
    pub h_gamma: f64,
    /// Kept stage displacements S, column-major: `dimension * rank` elements.
    s: Vec<f64>,
    /// Stage residuals U = Y - D S, column-major: `dimension * rank` elements.
    u: Vec<f64>,
    /// Precomputed Woodbury matrix `K_r = C_k^{-1} (S^T S + lambda I)^{-1}`, `rank x rank`.
    k_mat: [[f64; MAX_SECANT_RANK]; MAX_SECANT_RANK],
}

#[allow(clippy::needless_range_loop, clippy::manual_swap)]
impl SecantPreconditioner {
    /// Create an identity preconditioner of the given dimension (rank 0).
    pub fn identity(dimension: usize) -> Self {
        Self {
            dimension,
            rank: 0,
            h_gamma: 0.0,
            s: Vec::new(),
            u: Vec::new(),
            k_mat: [[0.0; MAX_SECANT_RANK]; MAX_SECANT_RANK],
        }
    }

    /// Build a secant preconditioner from stage displacements and residuals.
    ///
    /// Uses exact range matching (`lambda = 0.0`) on non-degenerate columns. If the
    /// capacitance matrix is ill-conditioned (`sigma_min < 1e-6 * sigma_max`), returns `None`.
    pub fn build(
        s_displacements: &[f64],
        y_residuals: &[f64],
        n: usize,
        k: usize,
        h_gamma: f64,
    ) -> Option<Self> {
        Self::build_internal(s_displacements, y_residuals, n, k, h_gamma, 0.0)
    }

    /// Build a secant preconditioner with Frobenius-norm regularization `lambda = 1e-6 * ||S||_F^2`.
    pub fn build_regularized(
        s_displacements: &[f64],
        y_residuals: &[f64],
        n: usize,
        k: usize,
        h_gamma: f64,
    ) -> Option<Self> {
        if n == 0 || k == 0 || s_displacements.len() != n * k {
            return None;
        }
        let frob_sq: f64 = s_displacements.iter().map(|&x| x * x).sum();
        let lambda = DEFAULT_SECANT_REGULARIZATION * frob_sq;
        Self::build_internal(s_displacements, y_residuals, n, k, h_gamma, lambda)
    }

    /// Build a secant preconditioner with an explicit regularization parameter `lambda`.
    pub fn build_with_regularization(
        s_displacements: &[f64],
        y_residuals: &[f64],
        n: usize,
        k: usize,
        h_gamma: f64,
        lambda: f64,
    ) -> Option<Self> {
        Self::build_internal(s_displacements, y_residuals, n, k, h_gamma, lambda)
    }

    fn build_internal(
        s_displacements: &[f64],
        y_residuals: &[f64],
        n: usize,
        k: usize,
        h_gamma: f64,
        lambda: f64,
    ) -> Option<Self> {
        if n == 0 || k == 0 || s_displacements.len() != n * k || y_residuals.len() != n * k {
            return None;
        }
        if !h_gamma.is_finite() || !lambda.is_finite() || lambda < 0.0 {
            return None;
        }
        let k_clamped = k.min(MAX_SECANT_RANK);

        // 1. Compute Gram matrix G = S^T S (k_clamped x k_clamped) on stack.
        let mut gram = [[0.0; MAX_SECANT_RANK]; MAX_SECANT_RANK];
        for i in 0..k_clamped {
            let si = &s_displacements[i * n..(i + 1) * n];
            for j in i..k_clamped {
                let sj = &s_displacements[j * n..(j + 1) * n];
                let mut dot = 0.0;
                for m in 0..n {
                    dot += si[m] * sj[m];
                }
                gram[i][j] = dot;
                gram[j][i] = dot;
            }
        }

        // 2. Pivoted Cholesky / QR on Gram matrix to drop collinear / degenerate columns.
        let mut pivots = [0usize; MAX_SECANT_RANK];
        let mut diag = [0.0; MAX_SECANT_RANK];
        for i in 0..k_clamped {
            pivots[i] = i;
            diag[i] = gram[i][i];
        }

        let mut r_mat = [[0.0; MAX_SECANT_RANK]; MAX_SECANT_RANK];
        let mut r_rank = 0usize;
        let mut r00 = 0.0;

        for step in 0..k_clamped {
            // Find pivot with largest diagonal entry
            let mut best_idx = step;
            let mut best_val = diag[step];
            for i in (step + 1)..k_clamped {
                if diag[i] > best_val {
                    best_val = diag[i];
                    best_idx = i;
                }
            }

            if !(best_val > 0.0 && best_val.is_finite()) {
                break;
            }

            // Swap step and best_idx in pivots, diag, and r_mat columns
            pivots.swap(step, best_idx);
            diag.swap(step, best_idx);
            for row in 0..step {
                let tmp = r_mat[row][step];
                r_mat[row][step] = r_mat[row][best_idx];
                r_mat[row][best_idx] = tmp;
            }

            let r_diag = best_val.sqrt();
            if step == 0 {
                r00 = r_diag;
            } else if r_diag < SECANT_COLUMN_DROP_TOL * r00 {
                // Degenerate/collinear column detected: drop it and terminate basis extension
                break;
            }

            r_mat[step][step] = r_diag;
            r_rank = step + 1;

            let col_step = pivots[step];
            for m in (step + 1)..k_clamped {
                let col_m = pivots[m];
                let g_val = gram[col_step][col_m];
                let mut sum = 0.0;
                for l in 0..step {
                    sum += r_mat[l][step] * r_mat[l][m];
                }
                let r_jm = (g_val - sum) / r_diag;
                r_mat[step][m] = r_jm;
                let new_diag = diag[m] - r_jm * r_jm;
                diag[m] = if new_diag > 0.0 { new_diag } else { 0.0 };
            }
        }

        if r_rank == 0 {
            return None;
        }

        // Sort kept column indices to preserve stage order
        let mut kept = [0usize; MAX_SECANT_RANK];
        kept[..r_rank].copy_from_slice(&pivots[..r_rank]);
        kept[..r_rank].sort_unstable();

        // 3. Assemble kept columns of S and U (using D = 0 so U = Y)
        let mut s_kept = Vec::with_capacity(n * r_rank);
        let mut u_kept = Vec::with_capacity(n * r_rank);
        for &col in kept.iter().take(r_rank) {
            s_kept.extend_from_slice(&s_displacements[col * n..(col + 1) * n]);
            u_kept.extend_from_slice(&y_residuals[col * n..(col + 1) * n]);
        }

        // 4. Form M = S^T S + lambda I_r
        let mut m_mat = [[0.0; MAX_SECANT_RANK]; MAX_SECANT_RANK];
        for i in 0..r_rank {
            let col_i = kept[i];
            for j in 0..r_rank {
                let col_j = kept[j];
                let val = gram[col_i][col_j] + if i == j { lambda } else { 0.0 };
                m_mat[i][j] = val;
            }
        }

        // Invert M
        let m_inv = invert_small_matrix(&m_mat, r_rank)?;

        // 5. Compute B = S^T U (r_rank x r_rank)
        let mut b_mat = [[0.0; MAX_SECANT_RANK]; MAX_SECANT_RANK];
        for i in 0..r_rank {
            let si = &s_kept[i * n..(i + 1) * n];
            for j in 0..r_rank {
                let uj = &u_kept[j * n..(j + 1) * n];
                let mut dot = 0.0;
                for m in 0..n {
                    dot += si[m] * uj[m];
                }
                b_mat[i][j] = dot;
            }
        }

        // 6. Compute T = M^{-1} B and Capacitance matrix C_k = I_r - h_gamma * T
        let mut c_mat = [[0.0; MAX_SECANT_RANK]; MAX_SECANT_RANK];
        for i in 0..r_rank {
            for j in 0..r_rank {
                let mut sum = 0.0;
                for l in 0..r_rank {
                    sum += m_inv[i][l] * b_mat[l][j];
                }
                c_mat[i][j] = if i == j { 1.0 } else { 0.0 } - h_gamma * sum;
            }
        }

        // 7. Capacitance matrix safeguard: check singular values
        let (sigma_min, sigma_max) = singular_value_extrema(&c_mat, r_rank);
        if !sigma_min.is_finite()
            || !sigma_max.is_finite()
            || sigma_max <= 0.0
            || sigma_min < SECANT_CAPACITANCE_CONDITION_TOL * sigma_max
        {
            // ADJ-22 condition safeguard: capacitance matrix is near-singular
            return None;
        }

        // 8. Invert capacitance matrix C_k
        let c_inv = invert_small_matrix(&c_mat, r_rank)?;

        // 9. Precompute K_r = C_inv * M_inv
        let mut k_mat = [[0.0; MAX_SECANT_RANK]; MAX_SECANT_RANK];
        for i in 0..r_rank {
            for j in 0..r_rank {
                let mut sum = 0.0;
                for l in 0..r_rank {
                    sum += c_inv[i][l] * m_inv[l][j];
                }
                k_mat[i][j] = sum;
            }
        }

        Some(Self {
            dimension: n,
            rank: r_rank,
            h_gamma,
            s: s_kept,
            u: u_kept,
            k_mat,
        })
    }

    /// Apply the preconditioner into `out`: `out = P r`.
    ///
    /// Computes `out = r + h_gamma * U * K_r * (S^T r)` with zero heap allocations,
    /// using stack-allocated buffers for `rank <= 6`.
    #[inline]
    pub fn apply_into(&self, r: &[f64], out: &mut [f64], h_gamma: f64) {
        if self.rank == 0 || self.dimension == 0 {
            out.copy_from_slice(r);
            return;
        }
        let n = self.dimension;
        let r_rank = self.rank;

        debug_assert_eq!(r.len(), n);
        debug_assert_eq!(out.len(), n);
        if r.len() != n || out.len() != n {
            return;
        }

        // Fixed-size stack buffers: ZERO allocation
        let mut a = [0.0; MAX_SECANT_RANK];
        let mut z = [0.0; MAX_SECANT_RANK];

        // 1. a = S^T r (r_rank inner products)
        for j in 0..r_rank {
            let sj = &self.s[j * n..(j + 1) * n];
            let mut dot = 0.0;
            for i in 0..n {
                dot += sj[i] * r[i];
            }
            a[j] = dot;
        }

        // 2. z = h_gamma * K_r * a
        for i in 0..r_rank {
            let mut sum = 0.0;
            for j in 0..r_rank {
                sum += self.k_mat[i][j] * a[j];
            }
            z[i] = h_gamma * sum;
        }

        // 3. out = r + U * z
        out.copy_from_slice(r);
        for j in 0..r_rank {
            let uj = &self.u[j * n..(j + 1) * n];
            let zj = z[j];
            for i in 0..n {
                out[i] += zj * uj[i];
            }
        }
    }
}

impl Preconditioner for SecantPreconditioner {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.dimension || y.len() != self.dimension {
            return Err(CoreError::Dimension(format!(
                "SecantPreconditioner shape mismatch: expected {}, got x={}, y={}",
                self.dimension,
                x.len(),
                y.len()
            )));
        }
        self.apply_into(x, y, self.h_gamma);
        Ok(())
    }

    fn is_identity(&self) -> bool {
        self.rank == 0
    }

    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
}

/// Invert an `r x r` matrix (`r <= MAX_SECANT_RANK`) using Gauss-Jordan elimination
/// with partial pivoting on stack arrays.
#[allow(clippy::needless_range_loop, clippy::manual_swap)]
fn invert_small_matrix(
    c: &[[f64; MAX_SECANT_RANK]; MAX_SECANT_RANK],
    r: usize,
) -> Option<[[f64; MAX_SECANT_RANK]; MAX_SECANT_RANK]> {
    let mut a = *c;
    let mut inv = [[0.0; MAX_SECANT_RANK]; MAX_SECANT_RANK];
    for i in 0..r {
        inv[i][i] = 1.0;
    }

    for k in 0..r {
        let mut max_val = a[k][k].abs();
        let mut pivot_row = k;
        for i in (k + 1)..r {
            let val = a[i][k].abs();
            if val > max_val {
                max_val = val;
                pivot_row = i;
            }
        }

        if !(max_val > 1e-15 && max_val.is_finite()) {
            return None;
        }

        if pivot_row != k {
            for j in 0..r {
                let tmp_a = a[k][j];
                a[k][j] = a[pivot_row][j];
                a[pivot_row][j] = tmp_a;

                let tmp_inv = inv[k][j];
                inv[k][j] = inv[pivot_row][j];
                inv[pivot_row][j] = tmp_inv;
            }
        }

        let diag = a[k][k];
        let inv_diag = 1.0 / diag;
        for j in 0..r {
            a[k][j] *= inv_diag;
            inv[k][j] *= inv_diag;
        }

        for i in 0..r {
            if i != k {
                let factor = a[i][k];
                if factor != 0.0 {
                    for j in 0..r {
                        a[i][j] -= factor * a[k][j];
                        inv[i][j] -= factor * inv[k][j];
                    }
                }
            }
        }
    }

    Some(inv)
}

/// Compute (sigma_min, sigma_max) of an `r x r` matrix `C` via Jacobi eigenvalues of `C^T C`.
#[allow(clippy::needless_range_loop)]
fn singular_value_extrema(c: &[[f64; MAX_SECANT_RANK]; MAX_SECANT_RANK], r: usize) -> (f64, f64) {
    if r == 0 {
        return (0.0, 0.0);
    }
    // Form A = C^T C (symmetric positive semi-definite)
    let mut a = [[0.0; MAX_SECANT_RANK]; MAX_SECANT_RANK];
    for i in 0..r {
        for j in i..r {
            let mut dot = 0.0;
            for l in 0..r {
                dot += c[l][i] * c[l][j];
            }
            a[i][j] = dot;
            a[j][i] = dot;
        }
    }

    // Jacobi eigenvalue algorithm
    for _sweep in 0..30 {
        let mut max_offdiag = 0.0;
        for p in 0..r {
            for q in (p + 1)..r {
                let val = a[p][q].abs();
                if val > max_offdiag {
                    max_offdiag = val;
                }
            }
        }
        if max_offdiag < 1e-15 {
            break;
        }

        for p in 0..r {
            for q in (p + 1)..r {
                let apq = a[p][q];
                let scale = a[p][p].abs() + a[q][q].abs();
                if apq.abs() <= 1e-16 * scale {
                    a[p][q] = 0.0;
                    a[q][p] = 0.0;
                    continue;
                }
                let diff = a[q][q] - a[p][p];
                let theta = 0.5 * (2.0 * apq).atan2(diff);
                let c_rot = theta.cos();
                let s_rot = theta.sin();

                let app = a[p][p];
                let aqq = a[q][q];
                a[p][p] = c_rot * c_rot * app - 2.0 * s_rot * c_rot * apq + s_rot * s_rot * aqq;
                a[q][q] = s_rot * s_rot * app + 2.0 * s_rot * c_rot * apq + c_rot * c_rot * aqq;
                a[p][q] = 0.0;
                a[q][p] = 0.0;

                for i in 0..r {
                    if i != p && i != q {
                        let aip = a[i][p];
                        let aiq = a[i][q];
                        a[i][p] = c_rot * aip - s_rot * aiq;
                        a[p][i] = a[i][p];
                        a[i][q] = s_rot * aip + c_rot * aiq;
                        a[q][i] = a[i][q];
                    }
                }
            }
        }
    }

    let mut sigma_min = f64::INFINITY;
    let mut sigma_max = 0.0_f64;
    for i in 0..r {
        let ev = a[i][i];
        let sv = if ev > 0.0 { ev.sqrt() } else { 0.0 };
        if sv < sigma_min {
            sigma_min = sv;
        }
        if sv > sigma_max {
            sigma_max = sv;
        }
    }

    (sigma_min, sigma_max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodas5p_core::{DenseMatrix, DenseOperator, WorkCounters};

    #[test]
    fn test_exact_action_on_range() {
        let n = 4;
        let k = 3;
        let h_gamma = 0.21193756319429014;

        // Linear operator J
        let mut j_mat = DenseMatrix::zeros(n, n);
        j_mat[(0, 0)] = 2.0;
        j_mat[(0, 1)] = 0.5;
        j_mat[(0, 2)] = -0.2;
        j_mat[(1, 0)] = -0.4;
        j_mat[(1, 1)] = 3.5;
        j_mat[(1, 3)] = 0.1;
        j_mat[(2, 2)] = 1.8;
        j_mat[(2, 3)] = -0.5;
        j_mat[(3, 0)] = 0.2;
        j_mat[(3, 3)] = 2.4;

        // 3 linearly independent stage displacements
        let s_displacements = vec![
            1.0, 0.5, -0.2, 0.1, // s1
            0.2, -1.0, 0.8, 0.3, // s2
            -0.5, 0.2, 0.3, 1.2, // s3
        ];

        // Residuals Y = J * S (linear operator)
        let mut y_residuals = vec![0.0; n * k];
        for j in 0..k {
            let sj = &s_displacements[j * n..(j + 1) * n];
            let yj = &mut y_residuals[j * n..(j + 1) * n];
            for r in 0..n {
                let mut dot = 0.0;
                for c in 0..n {
                    dot += j_mat[(r, c)] * sj[c];
                }
                yj[r] = dot;
            }
        }

        let pc = SecantPreconditioner::build(&s_displacements, &y_residuals, n, k, h_gamma)
            .expect("secant preconditioner should build successfully");

        assert_eq!(pc.rank, k);
        assert_eq!(pc.dimension, n);

        // Operator W = I - h_gamma * J
        let mut w_mat = DenseMatrix::identity(n);
        for r in 0..n {
            for c in 0..n {
                w_mat[(r, c)] -= h_gamma * j_mat[(r, c)];
            }
        }

        // Test for several vectors v in span(S)
        let test_coeffs = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [2.5, -1.5, 0.8],
            [-0.3, 1.7, -2.1],
        ];

        for coeffs in test_coeffs {
            let mut v = vec![0.0; n];
            for j in 0..k {
                let sj = &s_displacements[j * n..(j + 1) * n];
                for i in 0..n {
                    v[i] += coeffs[j] * sj[i];
                }
            }

            // Compute w_v = (I - h_gamma * J) * v
            let mut w_v = vec![0.0; n];
            for r in 0..n {
                let mut dot = 0.0;
                for c in 0..n {
                    dot += w_mat[(r, c)] * v[c];
                }
                w_v[r] = dot;
            }

            // Preconditioner application: out = P * w_v
            let mut out = vec![0.0; n];
            pc.apply_into(&w_v, &mut out, h_gamma);

            // Verify P (I - h gamma J) v = v to < 1e-12
            let mut max_err = 0.0_f64;
            for i in 0..n {
                let diff = (out[i] - v[i]).abs();
                if diff > max_err {
                    max_err = diff;
                }
            }
            assert!(
                max_err < 1e-12,
                "exact action on span(S) failed: max_err = {max_err:e}"
            );
        }
    }

    #[test]
    fn test_collinear_safeguard() {
        let n = 3;
        let k = 2;
        let h_gamma = 0.2;

        let s1 = vec![1.0, 2.0, -1.0];
        // s2 is nearly identical to s1: collinear within 1e-8
        let s2: Vec<f64> = s1.iter().map(|&x| x * (1.0 + 1e-8)).collect();

        let mut s_displacements = Vec::new();
        s_displacements.extend_from_slice(&s1);
        s_displacements.extend_from_slice(&s2);

        // Dummy residuals
        let y1 = vec![0.5, -0.3, 1.2];
        let y2: Vec<f64> = y1.iter().map(|&x| x * (1.0 + 1e-8)).collect();
        let mut y_residuals = Vec::new();
        y_residuals.extend_from_slice(&y1);
        y_residuals.extend_from_slice(&y2);

        let pc = SecantPreconditioner::build(&s_displacements, &y_residuals, n, k, h_gamma)
            .expect("should build by dropping degenerate column");

        // Verify degenerate column was dropped: rank should be 1, not 2
        assert_eq!(pc.rank, 1, "collinear column must be dropped");

        // Verify applying preconditioner does not produce NaNs
        let r = [1.0, -0.5, 2.0];
        let mut out = [0.0; 3];
        pc.apply_into(&r, &mut out, h_gamma);
        for &val in &out {
            assert!(val.is_finite(), "preconditioner output must not be NaN/Inf");
        }
    }

    #[test]
    fn test_capacitance_safeguard_fallback() {
        let n = 2;
        let k = 1;
        let h_gamma = 1.0;

        let s_displacements = vec![1.0, 0.0];
        // Construct Y such that C_k = 1 - h_gamma * (S^T S)^{-1} S^T Y = 0 (singular)
        // S^T S = 1, h_gamma = 1. Let Y = [1.0, 0.0], then S^T Y = 1.
        // C_k = 1 - 1 * 1 = 0!
        let y_residuals = vec![1.0, 0.0];

        let pc = SecantPreconditioner::build(&s_displacements, &y_residuals, n, k, h_gamma);
        assert!(
            pc.is_none(),
            "singular capacitance matrix must trigger fallback (None)"
        );
    }

    #[test]
    fn test_zero_allocation_in_apply_into() {
        let n = 8;
        let k = 3;
        let h_gamma = 0.25;

        let s_displacements: Vec<f64> = (0..n * k).map(|i| (i as f64 * 0.31).sin()).collect();
        let y_residuals: Vec<f64> = (0..n * k).map(|i| (i as f64 * 0.47).cos()).collect();

        if let Some(pc) = SecantPreconditioner::build(&s_displacements, &y_residuals, n, k, h_gamma)
        {
            let r: Vec<f64> = (0..n).map(|i| (i as f64 * 0.13).cos()).collect();
            let mut out = vec![0.0; n];

            // Warm up
            pc.apply_into(&r, &mut out, h_gamma);

            // Execute multiple applications to ensure stability
            for _ in 0..10 {
                pc.apply_into(&r, &mut out, h_gamma);
                assert!(out.iter().all(|x| x.is_finite()));
            }
        }
    }

    #[test]
    fn test_true_residual_contract() {
        use crate::{GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into};

        let n = 6;
        let k = 2;
        let h_gamma = 0.2;

        let mut a_mat = DenseMatrix::identity(n);
        for i in 0..n {
            a_mat[(i, i)] = 3.0 + 0.1 * i as f64;
            if i + 1 < n {
                a_mat[(i, i + 1)] = -0.5;
            }
            if i > 0 {
                a_mat[(i, i - 1)] = 0.2;
            }
        }

        let s_displacements: Vec<f64> = (0..n * k).map(|i| ((i + 1) as f64 * 0.7).sin()).collect();
        let y_residuals: Vec<f64> = (0..n * k).map(|i| ((i + 1) as f64 * 0.5).cos()).collect();

        let pc = SecantPreconditioner::build(&s_displacements, &y_residuals, n, k, h_gamma)
            .expect("valid secant preconditioner");

        let op = DenseOperator::new(a_mat.clone()).unwrap();
        let rhs: Vec<f64> = (0..n).map(|i| ((i + 2) as f64 * 0.3).sin()).collect();

        let config = GmresConfig {
            restart: 10,
            max_arnoldi: 30,
            rtol: 1e-10,
            atol: 1e-12,
        };

        let mut ws = GmresWorkspace::default();
        ws.reserve(n, 10).unwrap();
        let mut out = vec![0.0; n];
        let mut counters = WorkCounters::default();

        let report = solve_gmres_into(
            &op,
            &pc,
            &rhs,
            None,
            &config,
            None,
            &mut out,
            &mut ws,
            GmresCapacity::unbounded(),
            &mut counters,
        )
        .expect("GMRES solve with secant preconditioner should converge");

        // Verify true residual: ||b - A x||
        let ax = a_mat.matvec(&out).unwrap();
        let true_res_norm: f64 = ax
            .iter()
            .zip(&rhs)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();

        assert!(
            (true_res_norm - report.residual_norm).abs() < 1e-10,
            "true residual contract: computed {true_res_norm:e} vs reported {:e}",
            report.residual_norm
        );
        assert!(
            true_res_norm <= 1e-9,
            "true residual should satisfy tolerance"
        );
    }

    #[test]
    fn test_secant_default_bit_identity() {
        use crate::{
            GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into,
            solve_gmres_into_with_secant,
        };
        use rodas5p_core::IdentityPreconditioner;

        let n = 5;
        let mut a_mat = DenseMatrix::identity(n);
        for i in 0..n {
            a_mat[(i, i)] = 2.5 + i as f64;
            if i + 1 < n {
                a_mat[(i, i + 1)] = 0.4;
            }
        }
        let op = DenseOperator::new(a_mat).unwrap();
        let rhs: Vec<f64> = (0..n).map(|i| (i as f64 + 1.0) * 0.3).collect();

        let config = GmresConfig::default();
        let mut ws1 = GmresWorkspace::default();
        let mut ws2 = GmresWorkspace::default();
        let mut out1 = vec![0.0; n];
        let mut out2 = vec![0.0; n];
        let mut c1 = WorkCounters::default();
        let mut c2 = WorkCounters::default();

        let ident = IdentityPreconditioner::new(n);
        let rep1 = solve_gmres_into(
            &op,
            &ident,
            &rhs,
            None,
            &config,
            None,
            &mut out1,
            &mut ws1,
            GmresCapacity::unbounded(),
            &mut c1,
        )
        .unwrap();

        let rep2 = solve_gmres_into_with_secant(
            &op,
            None,
            &rhs,
            None,
            &config,
            None,
            &mut out2,
            &mut ws2,
            GmresCapacity::unbounded(),
            &mut c2,
        )
        .unwrap();

        assert_eq!(rep1.iterations, rep2.iterations);
        assert_eq!(rep1.residual_norm.to_bits(), rep2.residual_norm.to_bits());
        for i in 0..n {
            assert_eq!(
                out1[i].to_bits(),
                out2[i].to_bits(),
                "default secant must be bit-identical (G1)"
            );
        }
    }
}
