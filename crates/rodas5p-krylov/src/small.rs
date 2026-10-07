use faer::{Mat, c64, linalg::solvers::SolveLstsq};
use rodas5p_core::{CoreError, CoreResult, DenseMatrix};

pub fn least_squares(a: &DenseMatrix, b: &[f64]) -> CoreResult<Vec<f64>> {
    if a.nrows() != b.len() {
        return Err(CoreError::Dimension(
            "least-squares RHS shape mismatch".into(),
        ));
    }
    if a.ncols() == 0 {
        return Ok(Vec::new());
    }
    let fa = a.to_faer();
    let rhs = Mat::from_fn(b.len(), 1, |i, _| b[i]);
    let x = fa.col_piv_qr().solve_lstsq(&rhs);
    let out: Vec<f64> = (0..a.ncols()).map(|i| x[(i, 0)]).collect();
    if out.iter().all(|v| v.is_finite()) {
        Ok(out)
    } else {
        Err(CoreError::LinearSolve(format!(
            "least-squares solve produced NaN/Inf for {}x{} system",
            a.nrows(),
            a.ncols()
        )))
    }
}

/// Reused storage of [`LeastSquaresWorkspace::solve_into`] (research node
/// `research/spd04_ls_workspace_20261007`).
///
/// [`least_squares`] allocates about eleven times per call (the faer copy of
/// the matrix, the right-hand side, `ColPivQr::new`'s owned factor, both
/// permutation vectors, the Householder coefficients, the factor and solve
/// scratch, the split triangle, the solution and the output vector). This
/// workspace calls the same faer kernels with the parameters `ColPivQr::new`
/// and `solve_lstsq` use, on buffers it keeps, so the solution is bitwise
/// the same. Each faer matrix is a view with the column stride and 64-byte
/// alignment a freshly allocated `Mat` of that shape has (faer rounds the
/// row capacity of `f64` matrices up to a multiple of 8 and aligns the
/// allocation to 64 bytes), so no kernel can see a different layout. Every
/// reused buffer is zeroed or fully overwritten before use, as the fresh
/// `Mat::zeros` buffers of the allocating path are. Buffers grow only when a
/// system needs more than every earlier one; growth is counted.
pub struct LeastSquaresWorkspace {
    /// Backing storage of the factored matrix (faer's `QR`, later the unit
    /// lower Householder basis), one 64-byte aligned column.
    qr: Mat<f64>,
    /// Backing storage of the Householder coefficients `Q_coeff`.
    q_coeff: Mat<f64>,
    /// Backing storage of the upper triangle `R`.
    r: Mat<f64>,
    /// Backing storage of the right-hand side, solved in place.
    rhs: Mat<f64>,
    perm_forward: Vec<usize>,
    perm_inverse: Vec<usize>,
    scratch: Option<faer::dyn_stack::MemBuffer>,
    scratch_req: Option<faer::dyn_stack::StackReq>,
    last_grew: bool,
    growth_events: u64,
    solves: u64,
}

impl Default for LeastSquaresWorkspace {
    fn default() -> Self {
        Self {
            qr: Mat::new(),
            q_coeff: Mat::new(),
            r: Mat::new(),
            rhs: Mat::new(),
            perm_forward: Vec::new(),
            perm_inverse: Vec::new(),
            scratch: None,
            scratch_req: None,
            last_grew: false,
            growth_events: 0,
            solves: 0,
        }
    }
}

impl Clone for LeastSquaresWorkspace {
    /// A clone is an empty workspace: the buffers hold no state between
    /// solves, so a clone regrows them on its first solve.
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for LeastSquaresWorkspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LeastSquaresWorkspace")
            .field("qr_len", &self.qr.nrows())
            .field("q_coeff_len", &self.q_coeff.nrows())
            .field("r_len", &self.r.nrows())
            .field("rhs_len", &self.rhs.nrows())
            .field("perm_len", &self.perm_forward.len())
            .field(
                "scratch_bytes",
                &self.scratch_req.map_or(0, |req| req.size_bytes()),
            )
            .field("growth_events", &self.growth_events)
            .field("solves", &self.solves)
            .finish()
    }
}

/// Column stride of a freshly allocated faer `Mat<f64>` with `rows` rows.
fn faer_f64_stride(rows: usize) -> usize {
    rows.next_multiple_of(8)
}

/// Grow the single-column backing `storage` to at least `len` entries.
fn grow_backing(storage: &mut Mat<f64>, len: usize) -> bool {
    if storage.nrows() >= len {
        return false;
    }
    *storage = Mat::zeros(len, 1);
    true
}

fn backing_slice(storage: &mut Mat<f64>) -> &mut [f64] {
    storage
        .col_mut(0)
        .try_as_col_major_mut()
        .expect("a single faer column is contiguous")
        .as_slice_mut()
}

impl LeastSquaresWorkspace {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the last [`Self::solve_into`] grew any buffer.
    pub fn last_solve_grew(&self) -> bool {
        self.last_grew
    }

    /// Solves that grew at least one buffer, since creation.
    pub fn growth_events(&self) -> u64 {
        self.growth_events
    }

    /// Solves that reached the factorization, since creation.
    pub fn solves(&self) -> u64 {
        self.solves
    }

    /// [`least_squares`] into `out` (cleared, then holding `a.ncols()`
    /// values) with this workspace's buffers: the same solution bits, the
    /// same errors. On an error `out`'s contents are unspecified. A matrix
    /// with fewer rows than columns goes to [`least_squares`] unchanged (faer
    /// refuses it there).
    pub fn solve_into(&mut self, a: &DenseMatrix, b: &[f64], out: &mut Vec<f64>) -> CoreResult<()> {
        use faer::dyn_stack::{MemBuffer, MemStack, StackReq};
        use faer::linalg::qr::{col_pivoting, no_pivoting};
        use faer::{Conj, MatMut};

        self.last_grew = false;
        out.clear();
        if a.nrows() != b.len() {
            return Err(CoreError::Dimension(
                "least-squares RHS shape mismatch".into(),
            ));
        }
        if a.ncols() == 0 {
            return Ok(());
        }
        let (m, n) = (a.nrows(), a.ncols());
        if m < n {
            out.extend(least_squares(a, b)?);
            return Ok(());
        }
        self.solves += 1;
        // As `ColPivQr::new_imp` and `SolveLstsqCore for ColPivQr`.
        let par = faer::get_global_parallelism();
        let size = n;
        let block_size = no_pivoting::factor::recommended_block_size::<f64>(m, n);
        let factor_req = col_pivoting::factor::qr_in_place_scratch::<usize, f64>(
            m,
            n,
            block_size,
            par,
            Default::default(),
        );
        let solve_req = col_pivoting::solve::solve_lstsq_in_place_scratch::<usize, f64>(
            m, n, block_size, 1, par,
        );
        let req = StackReq::or(factor_req, solve_req);

        let (qr_stride, coeff_stride, r_stride, rhs_stride) = (
            faer_f64_stride(m),
            faer_f64_stride(block_size),
            faer_f64_stride(size),
            faer_f64_stride(m),
        );
        let mut grew = grow_backing(&mut self.qr, qr_stride * n);
        grew |= grow_backing(&mut self.q_coeff, coeff_stride * size);
        grew |= grow_backing(&mut self.r, r_stride * size);
        grew |= grow_backing(&mut self.rhs, rhs_stride);
        if self.perm_forward.len() < n {
            self.perm_forward.resize(n, 0);
            self.perm_inverse.resize(n, 0);
            grew = true;
        }
        let fits = self.scratch_req.is_some_and(|have| {
            have.size_bytes() >= req.size_bytes() && have.align_bytes() >= req.align_bytes()
        });
        if !fits {
            let grown = match self.scratch_req {
                Some(have) => StackReq::or(have, req),
                None => req,
            };
            self.scratch = Some(MemBuffer::new(grown));
            self.scratch_req = Some(grown);
            grew = true;
        }
        if grew {
            self.growth_events += 1;
            self.last_grew = true;
        }

        // `A.to_owned()`: every entry of the `m x n` view is written.
        let mut qr = MatMut::from_column_major_slice_with_stride_mut(
            backing_slice(&mut self.qr),
            m,
            n,
            qr_stride,
        );
        for j in 0..n {
            for i in 0..m {
                qr[(i, j)] = a[(i, j)];
            }
        }
        // `Mat::zeros(block_size, size)`.
        let mut q_coeff = MatMut::from_column_major_slice_with_stride_mut(
            backing_slice(&mut self.q_coeff),
            block_size,
            size,
            coeff_stride,
        );
        q_coeff.fill(0.0);
        // `vec![0usize; n]`, twice.
        let perm_forward = &mut self.perm_forward[..n];
        let perm_inverse = &mut self.perm_inverse[..n];
        perm_forward.fill(0);
        perm_inverse.fill(0);
        let scratch = self
            .scratch
            .as_mut()
            .expect("the scratch buffer was sized above");
        let (_, perm) = col_pivoting::factor::qr_in_place(
            qr.as_mut(),
            q_coeff.as_mut(),
            perm_forward,
            perm_inverse,
            par,
            MemStack::new(scratch),
            Default::default(),
        );
        // faer's private `split_LU` for `m >= n`: `R` is a zeroed `size x
        // size` matrix receiving the upper triangle, then the factored
        // matrix becomes the unit lower Householder basis.
        let mut r = MatMut::from_column_major_slice_with_stride_mut(
            backing_slice(&mut self.r),
            size,
            size,
            r_stride,
        );
        r.fill(0.0);
        r.copy_from_triangular_upper(qr.as_ref().get(..size, ..size));
        for j in 0..n {
            for i in 0..j.min(m) {
                qr[(i, j)] = 0.0;
            }
        }
        qr.as_mut().diagonal_mut().fill(1.0);
        // `solve_lstsq`: a zeroed `m x 1` copy of the right-hand side,
        // solved in place, then truncated to `n` rows.
        let mut rhs = MatMut::from_column_major_slice_with_stride_mut(
            backing_slice(&mut self.rhs),
            m,
            1,
            rhs_stride,
        );
        for (i, value) in b.iter().enumerate() {
            rhs[(i, 0)] = *value;
        }
        col_pivoting::solve::solve_lstsq_in_place_with_conj(
            qr.as_ref(),
            q_coeff.as_ref(),
            r.as_ref(),
            perm,
            Conj::No,
            rhs.as_mut(),
            par,
            MemStack::new(scratch),
        );
        out.extend((0..n).map(|i| rhs[(i, 0)]));
        if out.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::LinearSolve(format!(
                "least-squares solve produced NaN/Inf for {}x{} system",
                a.nrows(),
                a.ncols()
            )))
        }
    }
}

pub fn generalized_eigen(
    a: &DenseMatrix,
    b: &DenseMatrix,
) -> CoreResult<(Vec<c64>, Vec<Vec<c64>>)> {
    if a.nrows() != a.ncols() || b.nrows() != b.ncols() || a.nrows() != b.nrows() {
        return Err(CoreError::Dimension(
            "generalized eigenproblem shape mismatch".into(),
        ));
    }
    let n = a.nrows();
    if n == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    if n == 1 {
        let denominator = b[(0, 0)];
        let value = if denominator == 0.0 {
            c64::new(f64::INFINITY, 0.0)
        } else {
            c64::new(a[(0, 0)] / denominator, 0.0)
        };
        return Ok((vec![value], vec![vec![c64::new(1.0, 0.0)]]));
    }
    // faer's `Mat::generalized_eigen` sizes its scratch with `gevd_scratch`,
    // which omits the eigenvector back-substitution of `qz_to_gevd_real`
    // (two `n x 1` norms, an `n x 2` right-hand side and an `n x 2`
    // product). For small pairs with a complex eigenvalue pair the buffer is
    // too short and faer panics ("buffer is not large enough"; found by the
    // thread-transfer matrix-free driver, research node
    // `research/thread_transfer_mf_workspace_20261002`). Call the same
    // routine with that requirement added.
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::linalg::evd::ComputeEigenvectors;
    let mut fa = a.to_faer();
    let mut fb = b.to_faer();
    let par = faer::get_global_parallelism();
    let scratch = faer::linalg::gevd::gevd_scratch::<f64>(
        n,
        ComputeEigenvectors::No,
        ComputeEigenvectors::Yes,
        par,
        Default::default(),
    )
    .or(faer::linalg::temp_mat_scratch::<f64>(n, 8));
    let mut u_real = Mat::<f64>::zeros(n, n);
    let mut s_re = faer::diag::Diag::<f64>::zeros(n);
    let mut s_im = faer::diag::Diag::<f64>::zeros(n);
    let mut s_b = faer::diag::Diag::<f64>::zeros(n);
    faer::linalg::gevd::gevd_real(
        fa.as_mut(),
        fb.as_mut(),
        s_re.as_mut(),
        s_im.as_mut(),
        s_b.as_mut(),
        None,
        Some(u_real.as_mut()),
        par,
        MemStack::new(&mut MemBuffer::new(scratch)),
        Default::default(),
    )
    .map_err(|e| CoreError::LinearSolve(format!("generalized eigensolve failed: {e:?}")))?;
    // A complex pair (j, j + 1) is stored as the real and imaginary parts of
    // one eigenvector, as in faer's `real_to_cplx`.
    let sa = |j: usize| c64::new(s_re[j], s_im[j]);
    let mut values = Vec::with_capacity(n);
    let mut vectors = Vec::with_capacity(n);
    let mut j = 0;
    while j < n {
        // The same complex division by `beta + 0i` as faer's result.
        let den = c64::new(s_b[j], 0.0);
        let ratio = |value: c64| {
            if den.norm() == 0.0 {
                c64::new(f64::INFINITY, 0.0)
            } else {
                value / den
            }
        };
        if s_im[j] == 0.0 || j + 1 == n {
            values.push(ratio(sa(j)));
            vectors.push((0..n).map(|i| c64::new(u_real[(i, j)], 0.0)).collect());
            j += 1;
        } else {
            values.push(ratio(sa(j)));
            values.push(ratio(c64::new(s_re[j], -s_im[j])));
            vectors.push(
                (0..n)
                    .map(|i| c64::new(u_real[(i, j)], u_real[(i, j + 1)]))
                    .collect(),
            );
            vectors.push(
                (0..n)
                    .map(|i| c64::new(u_real[(i, j)], -u_real[(i, j + 1)]))
                    .collect(),
            );
            j += 2;
        }
    }
    Ok((values, vectors))
}

#[cfg(test)]
mod generalized_eigen_tests {
    use super::*;

    fn matrix(rows: &[&[f64]]) -> DenseMatrix {
        DenseMatrix::from_rows(rows).unwrap()
    }

    fn bits(hex: &str) -> f64 {
        f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
    }

    /// max_j ||A v_j - lambda_j B v_j|| / (||A|| + |lambda_j| ||B||) ||v_j||.
    fn worst_residual(a: &DenseMatrix, b: &DenseMatrix) -> f64 {
        let (values, vectors) = generalized_eigen(a, b).unwrap();
        let n = a.nrows();
        assert_eq!(values.len(), n);
        let norm = |m: &DenseMatrix| m.as_slice().iter().map(|x| x.abs()).fold(0.0, f64::max);
        let mut worst = 0.0_f64;
        for (lambda, v) in values.iter().zip(&vectors) {
            let mut residual = 0.0_f64;
            for i in 0..n {
                let mut r = c64::new(0.0, 0.0);
                for k in 0..n {
                    r += v[k] * a[(i, k)] - *lambda * v[k] * b[(i, k)];
                }
                residual = residual.max(r.norm());
            }
            let vnorm = v.iter().map(|z| z.norm()).fold(0.0, f64::max);
            worst = worst.max(residual / ((norm(a) + lambda.norm() * norm(b)) * vnorm));
        }
        worst
    }

    #[test]
    fn a_complex_pair_of_a_small_pencil_does_not_panic() {
        // The 2 x 2 pencil from the GCRO-DR harmonic Ritz step that made
        // faer 0.24.4's `Mat::generalized_eigen` panic (scratch too short).
        let a = matrix(&[
            &[bits("3feffffffffffd80"), bits("bd30084eafffff60")],
            &[bits("bd30084eafffff60"), bits("3ff00000000001da")],
        ]);
        let b = matrix(&[
            &[bits("3feffffffffffec4"), bits("3cb7ec23ffffff11")],
            &[bits("bd3ed1b14c0000dd"), bits("3ff00000000000f3")],
        ]);
        assert!(worst_residual(&a, &b) <= 1.0e-12);
        let (values, _) = generalized_eigen(&a, &b).unwrap();
        assert!(values.iter().all(|v| v.re.is_finite() && v.im.is_finite()));
    }

    #[test]
    fn real_and_complex_spectra_are_eigenpairs() {
        // Rotation-scaled block: eigenvalues 1 +- 2i, and a real 3.
        let a = matrix(&[&[1.0, -2.0, 0.0], &[2.0, 1.0, 0.0], &[0.0, 0.0, 3.0]]);
        let b = DenseMatrix::identity(3);
        assert!(worst_residual(&a, &b) <= 1.0e-13);
        let (mut values, _) = generalized_eigen(&a, &b).unwrap();
        values.sort_by(|x, y| x.re.total_cmp(&y.re).then(x.im.total_cmp(&y.im)));
        let expected = [c64::new(1.0, -2.0), c64::new(1.0, 2.0), c64::new(3.0, 0.0)];
        for (v, e) in values.iter().zip(expected) {
            assert!((v - e).norm() <= 1.0e-12, "{values:?}");
        }
        // A general pencil with a nontrivial B.
        let a = matrix(&[&[2.0, 1.0, 0.5], &[-1.0, 3.0, 0.2], &[0.3, -0.4, 1.0]]);
        let b = matrix(&[&[1.0, 0.1, 0.0], &[0.0, 2.0, 0.3], &[0.2, 0.0, 1.5]]);
        assert!(worst_residual(&a, &b) <= 1.0e-12);
    }
}
