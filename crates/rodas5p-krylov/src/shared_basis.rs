//! Shared per-step Krylov basis module for matrix-free stage solves.
//!
//! # Theory (T1: Shift-invariance of polynomial Krylov spaces)
//!
//! In standard Rosenbrock methods such as RODAS5P, all eight stage linear systems
//! have the shifted form `(I / (h gamma) - J) U_i = b_i`. Because each stage
//! right-hand side `b_i` (except for small nonlinear increments) lies within
//! `range(Q) + range(Z)` where `Z = J Q`, a single orthogonal Krylov basis `Q`
//! and its Jacobian image `Z = J Q` can be accumulated across stages within a step.
//!
//! Reusing `Q` and `Z` reduces the total number of Jacobian-vector products (JVPs)
//! per step significantly compared to restarting Krylov iteration from scratch
//! for each stage.

#![forbid(unsafe_code)]

/// Callback type for Jacobian-vector products in shared basis solves.
pub type JvpFn<'a, E> = dyn FnMut(&[f64], &mut [f64]) -> Result<(), E> + 'a;

/// Configuration of a shared per-step Krylov basis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SharedBasisConfig {
    /// Maximum basis dimension (prototype: 60).
    pub max_dim: usize,
    /// Residual budget: stop when h_gamma * ||r||_w <= theta (prototype: 1e-3).
    pub theta: f64,
}

impl Default for SharedBasisConfig {
    fn default() -> Self {
        Self {
            max_dim: 60,
            theta: 1.0e-3,
        }
    }
}

/// Outcome of one shifted solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SharedSolveStats {
    /// Number of new Jacobian-vector products evaluated during this solve call.
    pub new_jvps: usize,
    /// Final dimension of the basis upon return.
    pub basis_dim: usize,
    /// Weighted residual norm ||r||_w upon return.
    pub weighted_residual: f64,
    /// Whether the residual budget criterion `h_gamma * ||r||_w <= theta` was met.
    pub converged: bool,
}

/// A reusable Krylov subspace basis shared across stages within one integration step.
///
/// All Arnoldi basis vectors `Q`, transformed matrix `Z = J Q`, Householder QR scratch
/// buffers, residual vectors, and least-squares work buffers are pre-allocated in
/// [`SharedBasis::new`] and reused across calls to ensure zero allocations in hot loops.
#[derive(Clone, Debug)]
pub struct SharedBasis {
    n: usize,
    cfg: SharedBasisConfig,
    k: usize,
    /// Column-major matrix Q of size n x max_dim.
    q: Vec<f64>,
    /// Column-major matrix Z = J Q of size n x max_dim.
    z: Vec<f64>,
    /// Column-major scratch matrix A = Q / h_gamma - Z of size n x max_dim.
    a: Vec<f64>,
    /// Residual vector r of size n.
    r: Vec<f64>,
    /// Direction vector v of size n for Gram-Schmidt expansion.
    v: Vec<f64>,
    /// Solution vector y of size max_dim for least-squares problem min_y ||b - A y||_2.
    y: Vec<f64>,
    /// RHS vector d of size n transformed in place during Householder QR.
    d: Vec<f64>,
}

impl SharedBasis {
    /// Creates a new pre-allocated [`SharedBasis`].
    ///
    /// # Errors
    ///
    /// Returns `Err` if `n == 0`, `cfg.max_dim == 0`, or `cfg.theta` is not finite and positive.
    pub fn new(n: usize, cfg: SharedBasisConfig) -> Result<Self, String> {
        if n == 0 {
            return Err("dimension n must be positive".to_string());
        }
        if cfg.max_dim == 0 {
            return Err("max_dim must be positive".to_string());
        }
        if !cfg.theta.is_finite() || cfg.theta <= 0.0 {
            return Err("theta must be finite and positive".to_string());
        }
        let total_basis_entries = match n.checked_mul(cfg.max_dim) {
            Some(val) => val,
            None => return Err("n * max_dim overflowed usize".to_string()),
        };
        Ok(Self {
            n,
            cfg,
            k: 0,
            q: vec![0.0; total_basis_entries],
            z: vec![0.0; total_basis_entries],
            a: vec![0.0; total_basis_entries],
            r: vec![0.0; n],
            v: vec![0.0; n],
            y: vec![0.0; cfg.max_dim],
            d: vec![0.0; n],
        })
    }

    /// Forget the accumulated basis.
    ///
    /// Must be called once per ODE step since the Jacobian `J` changes between steps.
    /// This operation is zero-allocation and only resets the basis dimension counter.
    pub fn reset(&mut self) {
        self.k = 0;
    }

    /// Current dimension `k` of the accumulated basis.
    pub fn dim(&self) -> usize {
        self.k
    }

    /// Configuration parameters for this shared basis.
    pub fn config(&self) -> SharedBasisConfig {
        self.cfg
    }

    /// Approximately solve `(I / h_gamma - J) x = b` with `x in range(Q)`.
    ///
    /// # Parameters
    ///
    /// - `h_gamma`: The shifted diagonal parameter `h * gamma`.
    /// - `b`: Right-hand side vector of length `n`.
    /// - `weights`: Error weights vector of length `n`, typically `1 / (atol_i + rtol * |y_i|)`.
    /// - `jvp`: Callback computing `out = J * v`.
    /// - `x`: Output slice of length `n` receiving the solution.
    ///
    /// # Convergence Criterion
    ///
    /// The solve converges when `h_gamma * ||r||_w <= theta`, where
    /// `||v||_w = sqrt(mean((v_i * weights_i)^2))`.
    pub fn solve_shifted<E>(
        &mut self,
        h_gamma: f64,
        b: &[f64],
        weights: &[f64],
        jvp: &mut JvpFn<'_, E>,
        x: &mut [f64],
    ) -> Result<SharedSolveStats, E> {
        // 1. Check slice lengths and h_gamma positivity/finiteness.
        if b.len() != self.n
            || weights.len() != self.n
            || x.len() != self.n
            || !h_gamma.is_finite()
            || h_gamma <= 0.0
        {
            x.fill(0.0);
            return Ok(SharedSolveStats {
                new_jvps: 0,
                basis_dim: self.k,
                weighted_residual: f64::NAN,
                converged: false,
            });
        }

        let mut new_jvps = 0;

        // Check for exact zero RHS: if b == 0, x = 0 is exact.
        let mut b_norm_sq = 0.0;
        for &val in b {
            b_norm_sq += val * val;
        }
        let b_norm = b_norm_sq.sqrt();
        if b_norm == 0.0 || !b_norm.is_finite() {
            x.fill(0.0);
            return Ok(SharedSolveStats {
                new_jvps: 0,
                basis_dim: self.k,
                weighted_residual: 0.0,
                converged: true,
            });
        }

        // 2. If k == 0: seed basis with q_1 = b / ||b||_2 and z_1 = J q_1 (one JVP).
        if self.k == 0 {
            let inv_b_norm = 1.0 / b_norm;
            for (qi, &bi) in self.q[..self.n].iter_mut().zip(&b[..self.n]) {
                *qi = bi * inv_b_norm;
            }
            jvp(&self.q[0..self.n], &mut self.z[0..self.n])?;
            new_jvps += 1;
            self.k = 1;
        }

        let inv_hg = 1.0 / h_gamma;

        // 3. Main basis extension loop.
        loop {
            // 3a. Form A = Q / h_gamma - Z (size n x k, column-major).
            for j in 0..self.k {
                let q_col = &self.q[j * self.n..(j + 1) * self.n];
                let z_col = &self.z[j * self.n..(j + 1) * self.n];
                let a_col = &mut self.a[j * self.n..(j + 1) * self.n];
                for i in 0..self.n {
                    a_col[i] = q_col[i] * inv_hg - z_col[i];
                }
            }

            // Solve min_y ||b - A y||_2 via Householder QR of A.
            self.d[..self.n].copy_from_slice(b);
            let mut qr_failed = false;

            for j in 0..self.k {
                let alpha = self.a[j * self.n + j];
                let mut sigma = 0.0;
                for i in (j + 1)..self.n {
                    let val = self.a[j * self.n + i];
                    sigma += val * val;
                }
                if sigma == 0.0 {
                    if alpha == 0.0 {
                        qr_failed = true;
                        break;
                    }
                    continue;
                }
                // Avoid subtractive cancellation: sign matches alpha.
                let s = if alpha >= 0.0 { 1.0 } else { -1.0 };
                let mu = s * (alpha * alpha + sigma).sqrt();
                let v0 = alpha + mu;
                let u_norm_sq = v0 * v0 + sigma;
                if u_norm_sq == 0.0 || !u_norm_sq.is_finite() {
                    qr_failed = true;
                    break;
                }
                let beta = 2.0 / u_norm_sq;

                // Apply H_j to trailing columns c in (j + 1)..k
                for c in (j + 1)..self.k {
                    let mut dot = v0 * self.a[c * self.n + j];
                    for i in (j + 1)..self.n {
                        dot += self.a[j * self.n + i] * self.a[c * self.n + i];
                    }
                    let tau_dot = beta * dot;
                    self.a[c * self.n + j] -= tau_dot * v0;
                    for i in (j + 1)..self.n {
                        self.a[c * self.n + i] -= tau_dot * self.a[j * self.n + i];
                    }
                }

                // Apply H_j to transformed RHS vector d
                let mut dot = v0 * self.d[j];
                for i in (j + 1)..self.n {
                    dot += self.a[j * self.n + i] * self.d[i];
                }
                let tau_dot = beta * dot;
                self.d[j] -= tau_dot * v0;
                for i in (j + 1)..self.n {
                    self.d[i] -= tau_dot * self.a[j * self.n + i];
                }

                self.a[j * self.n + j] = -mu;
                for i in (j + 1)..self.n {
                    self.a[j * self.n + i] = 0.0;
                }
            }

            if qr_failed {
                return Ok(SharedSolveStats {
                    new_jvps,
                    basis_dim: self.k,
                    weighted_residual: f64::NAN,
                    converged: false,
                });
            }

            // Back-substitution to solve R y = d[0..k]
            for i in (0..self.k).rev() {
                let mut sum = self.d[i];
                for m in (i + 1)..self.k {
                    sum -= self.a[m * self.n + i] * self.y[m];
                }
                let diag = self.a[i * self.n + i];
                if diag.abs() <= 1.0e-15 || !diag.is_finite() {
                    return Ok(SharedSolveStats {
                        new_jvps,
                        basis_dim: self.k,
                        weighted_residual: f64::NAN,
                        converged: false,
                    });
                }
                self.y[i] = sum / diag;
            }

            // 3b. x = Q y; r = b - A y = b - (x / h_gamma - Z y)
            for i in 0..self.n {
                let mut xi = 0.0;
                let mut zyi = 0.0;
                for j in 0..self.k {
                    let yj = self.y[j];
                    xi += self.q[j * self.n + i] * yj;
                    zyi += self.z[j * self.n + i] * yj;
                }
                x[i] = xi;
                self.r[i] = b[i] - (xi * inv_hg - zyi);
            }

            // Compute ||r||_w = sqrt(mean((r_i * weights_i)^2))
            let mut sum_sq = 0.0;
            for (ri, &wi) in self.r[..self.n].iter().zip(&weights[..self.n]) {
                let rw = ri * wi;
                sum_sq += rw * rw;
            }
            let r_w = (sum_sq / self.n as f64).sqrt();

            // 3c. If h_gamma * ||r||_w <= theta: return converged
            if h_gamma * r_w <= self.cfg.theta {
                return Ok(SharedSolveStats {
                    new_jvps,
                    basis_dim: self.k,
                    weighted_residual: r_w,
                    converged: true,
                });
            }

            // 3d. If k == max_dim or k >= n: return converged: false
            if self.k >= self.cfg.max_dim || self.k >= self.n {
                return Ok(SharedSolveStats {
                    new_jvps,
                    basis_dim: self.k,
                    weighted_residual: r_w,
                    converged: false,
                });
            }

            // 3e. Two-pass Gram-Schmidt: v = r - Q (Q^T r) twice
            self.v.copy_from_slice(&self.r);
            for _pass in 0..2 {
                for j in 0..self.k {
                    let q_col = &self.q[j * self.n..(j + 1) * self.n];
                    let mut dot = 0.0;
                    for (qi, vi) in q_col.iter().zip(&self.v) {
                        dot += qi * vi;
                    }
                    for (vi, &qi) in self.v.iter_mut().zip(q_col) {
                        *vi -= dot * qi;
                    }
                }
            }

            let mut v_norm_sq = 0.0;
            let mut r_norm_sq = 0.0;
            for i in 0..self.n {
                v_norm_sq += self.v[i] * self.v[i];
                r_norm_sq += self.r[i] * self.r[i];
            }
            let v_norm = v_norm_sq.sqrt();
            let r_norm = r_norm_sq.sqrt();

            // Breakdown check: ||v||_2 <= 1e-14 ||r||_2
            if v_norm <= 1.0e-14 * r_norm || !v_norm.is_finite() {
                return Ok(SharedSolveStats {
                    new_jvps,
                    basis_dim: self.k,
                    weighted_residual: r_w,
                    converged: false,
                });
            }

            // 3f. q_{k+1} = v / ||v||_2; z_{k+1} = J q_{k+1} (one JVP); k += 1
            let inv_v_norm = 1.0 / v_norm;
            let next_k = self.k;
            for i in 0..self.n {
                self.q[next_k * self.n + i] = self.v[i] * inv_v_norm;
            }
            jvp(
                &self.q[next_k * self.n..(next_k + 1) * self.n],
                &mut self.z[next_k * self.n..(next_k + 1) * self.n],
            )?;
            new_jvps += 1;
            self.k += 1;
        }
    }
}
