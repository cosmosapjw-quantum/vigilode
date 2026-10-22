//! Bounded shared Krylov construction and small-system operations.
use crate::{Config, Failure, FailureKind, Result, Work};
use rodas5p_core::{DenseMatrix, LinearOperator, LuFactorization, safe_l2};

pub(crate) fn finite(v: &[f64]) -> Result<()> {
    if v.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(Failure::new(
            FailureKind::NonFinite,
            "nonfinite vector or callback output",
        ))
    }
}
pub(crate) fn dot(a: &[f64], b: &[f64]) -> f64 {
    // Fixed-order compensated sum, independent of worker scheduling.
    let (mut sum, mut correction) = (0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        let term = x * y - correction;
        let next = sum + term;
        correction = (next - sum) - term;
        sum = next;
    }
    sum
}
pub(crate) fn norm(v: &[f64]) -> f64 {
    safe_l2(v)
}
pub(crate) fn apply(
    op: &dyn LinearOperator,
    v: &[f64],
    c: &Config,
    w: &mut Work,
    basis: bool,
) -> Result<Vec<f64>> {
    w.reserve(c, 0, 1)?;
    w.jvp_attempted += 1;
    if basis {
        w.basis_jvp += 1;
    }
    let mut out = vec![f64::NAN; v.len()];
    op.apply(v, &mut out)
        .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
    finite(&out)?;
    w.jvp_completed += 1;
    Ok(out)
}
#[derive(Debug)]
pub(crate) struct Basis {
    pub q: Vec<Vec<f64>>,
    pub jq: Vec<Vec<f64>>,
    processed: usize,
    n: usize,
}
impl Basis {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            q: Vec::new(),
            jq: Vec::new(),
            processed: 0,
            n,
        }
    }
    pub(crate) fn len(&self) -> usize {
        self.q.len()
    }
    pub(crate) fn project(&self, x: &[f64]) -> Vec<f64> {
        self.q.iter().map(|q| dot(q, x)).collect()
    }
    pub(crate) fn lift(&self, z: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n];
        for (a, q) in z.iter().zip(&self.q) {
            for (x, v) in out.iter_mut().zip(q) {
                *x += a * v;
            }
        }
        out
    }
    pub(crate) fn image_lift(&self, z: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n];
        for (a, q) in z.iter().zip(&self.jq) {
            for (x, v) in out.iter_mut().zip(q) {
                *x += a * v;
            }
        }
        out
    }
    fn orthogonalize(&self, input: &[f64], w: &mut Work) -> Vec<f64> {
        let mut v = input.to_vec();
        for _ in 0..2 {
            for q in &self.q {
                let a = dot(q, &v);
                w.orthogonalization_dots += 1;
                for (x, y) in v.iter_mut().zip(q) {
                    *x -= a * y;
                }
            }
        }
        v
    }
    pub(crate) fn add(
        &mut self,
        seed: &[f64],
        op: &dyn LinearOperator,
        c: &Config,
        w: &mut Work,
    ) -> Result<bool> {
        if self.len() >= c.max_basis.min(self.n) {
            return Ok(false);
        }
        finite(seed)?;
        let original = norm(seed);
        if original == 0.0 {
            return Ok(false);
        }
        let mut q = self.orthogonalize(seed, w);
        let size = norm(&q);
        if !size.is_finite() {
            return Err(Failure::new(FailureKind::NonFinite, "basis norm overflow"));
        }
        if size <= c.rank_relative_tol * original {
            return Ok(false);
        }
        for x in &mut q {
            *x /= size;
        }
        let image = apply(op, &q, c, w, true)?;
        self.q.push(q);
        self.jq.push(image);
        w.maximum_basis = w.maximum_basis.max(self.len());
        Ok(true)
    }
    pub(crate) fn grow(
        &mut self,
        goal: usize,
        op: &dyn LinearOperator,
        c: &Config,
        w: &mut Work,
    ) -> Result<()> {
        while self.len() < goal.min(c.max_basis).min(self.n) && self.processed < self.len() {
            let v = self.jq[self.processed].clone();
            self.processed += 1;
            self.add(&v, op, c, w)?;
        }
        Ok(())
    }
    pub(crate) fn expand(
        &mut self,
        residuals: &[Vec<f64>],
        op: &dyn LinearOperator,
        c: &Config,
        w: &mut Work,
    ) -> Result<bool> {
        if self.len() >= c.max_basis.min(self.n) {
            return Ok(false);
        }
        let before = self.len();
        let mut best: Option<Vec<f64>> = None;
        let mut size = 0.0;
        for r in residuals {
            let q = self.orthogonalize(r, w);
            let nr = norm(&q);
            let original = norm(r);
            if nr > size && nr > c.rank_relative_tol * original {
                size = nr;
                best = Some(q);
            }
        }
        if let Some(v) = best {
            self.add(&v, op, c, w)?;
        }
        self.grow(before.saturating_add(4), op, c, w)?;
        if self.len() > before {
            w.basis_expansions += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub(crate) fn projected_jacobian(&self) -> DenseMatrix {
        let m = self.len();
        let mut j = DenseMatrix::zeros(m, m);
        for i in 0..m {
            for k in 0..m {
                j[(i, k)] = dot(&self.q[i], &self.jq[k]);
            }
        }
        j
    }
}
/// Solve the eight-stage projected affine/Newton system using its repeated
/// diagonal block. Both homotopy terms and current-stage nonlinear derivatives
/// change only strictly lower blocks, so every diagonal is the same W_m.
///
/// One m-by-m factorization plus eight forward solves costs O(m^3 + s^2 m^2)
/// (s = 8), rather than factoring the entire sm-by-sm matrix. The dense block
/// matrix is still stored and the original full-system residual is still checked.
/// `small_solves` counts a complete projected solve, not its eight block solves.
/// A changed matrix structure is refused rather than silently approximated.
pub(crate) fn small_solve(a: &DenseMatrix, b: &[f64], work: &mut Work) -> Result<Vec<f64>> {
    const STAGES: usize = 8;
    finite(a.as_slice())?;
    finite(b)?;
    let n = b.len();
    if n == 0 || n % STAGES != 0 || a.nrows() != n || a.ncols() != n {
        return Err(Failure::new(
            FailureKind::Input,
            "projected solve requires a square eight-block system",
        ));
    }
    let m = n / STAGES;
    let mut diagonal = DenseMatrix::zeros(m, m);
    for u in 0..m {
        for v in 0..m {
            diagonal[(u, v)] = a[(u, v)];
        }
    }
    for i in 0..STAGES {
        for u in 0..m {
            for v in 0..m {
                if a[(i * m + u, i * m + v)] != diagonal[(u, v)] {
                    return Err(Failure::new(
                        FailureKind::Input,
                        "projected diagonal blocks are not identical",
                    ));
                }
            }
            for j in (i + 1) * m..n {
                if a[(i * m + u, j)] != 0.0 {
                    return Err(Failure::new(
                        FailureKind::Input,
                        "projected system is not block lower triangular",
                    ));
                }
            }
        }
    }
    work.small_factorizations += 1;
    let lu = LuFactorization::new(&diagonal)
        .map_err(|e| Failure::new(FailureKind::ProjectionSingular, e.to_string()))?;
    let mut x = vec![0.0; n];
    let mut block_rhs = vec![0.0; m];
    for i in 0..STAGES {
        let offset = i * m;
        for u in 0..m {
            block_rhs[u] = b[offset + u] - dot(&a.row(offset + u)[..offset], &x[..offset]);
        }
        finite(&block_rhs)?;
        let block = lu
            .solve(&block_rhs)
            .map_err(|e| Failure::new(FailureKind::ProjectionSingular, e.to_string()))?;
        x[offset..offset + m].copy_from_slice(&block);
    }
    work.small_solves += 1;
    finite(&x)?;
    let residual: Vec<_> = (0..n).map(|i| dot(a.row(i), &x) - b[i]).collect();
    let rel = norm(&residual) / norm(b).max(f64::MIN_POSITIVE);
    if !rel.is_finite() || rel > 1e-8 {
        return Err(Failure::new(
            FailureKind::ProjectionSingular,
            format!("small solve residual {rel:e}"),
        ));
    }
    Ok(x)
}

#[cfg(test)]
mod common_block_tests {
    use super::*;

    // Independently assembled generic block matrix, not RODAS coefficients.
    // The diagonal is deliberately nonnormal; m >= 2 requires pivoting.
    fn system(m: usize) -> (DenseMatrix, Vec<f64>) {
        let n = 8 * m;
        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..8 {
            for u in 0..m {
                for v in 0..m {
                    a[(i * m + u, i * m + v)] = if u == v {
                        2.0 + u as f64
                    } else if v > u {
                        0.9
                    } else {
                        -0.25
                    };
                    if m > 1 && u == 0 && v == 0 {
                        a[(i * m + u, i * m + v)] = 0.01;
                    }
                }
                for j in 0..i {
                    for v in 0..m {
                        a[(i * m + u, j * m + v)] = ((i + 2 * j + u + v) as f64).sin() * 0.015;
                    }
                }
            }
        }
        let b = (0..n).map(|k| (k as f64 + 0.3).cos()).collect();
        (a, b)
    }

    #[test]
    fn common_block_matches_independent_dense_lu() {
        for m in [1, 2, 4, 7] {
            let (a, b) = system(m);
            let reference = LuFactorization::new(&a).unwrap().solve(&b).unwrap();
            let mut work = Work::default();
            let got = small_solve(&a, &b, &mut work).unwrap();
            let error: Vec<_> = got.iter().zip(&reference).map(|(x, y)| x - y).collect();
            assert!(norm(&error) <= 2e-12 * norm(&reference).max(1.0));
            assert_eq!(work.small_factorizations, 1);
            assert_eq!(work.small_solves, 1);
        }
    }

    #[test]
    fn common_block_rejects_structure_changes_before_factoring() {
        let (a, b) = system(3);
        let mut changed_diagonal = a.clone();
        changed_diagonal[(3, 3)] += 0.125;
        let mut upper_coupling = a.clone();
        upper_coupling[(0, 3)] = 1e-30;
        for invalid in [changed_diagonal, upper_coupling] {
            let mut work = Work::default();
            let failure = small_solve(&invalid, &b, &mut work).unwrap_err();
            assert_eq!(failure.kind, FailureKind::Input);
            assert_eq!(work.small_factorizations, 0);
            assert_eq!(work.small_solves, 0);
        }
        let mut work = Work::default();
        assert_eq!(
            small_solve(&DenseMatrix::identity(7), &[1.0; 7], &mut work)
                .unwrap_err()
                .kind,
            FailureKind::Input
        );
    }

    #[test]
    fn common_block_singular_diagonal_fails_with_work_preserved() {
        let mut a = DenseMatrix::zeros(16, 16);
        for i in 0..8 {
            a[(2 * i, 2 * i)] = 1.0;
        }
        let mut work = Work::default();
        let failure = small_solve(&a, &[1.0; 16], &mut work).unwrap_err();
        assert_eq!(failure.kind, FailureKind::ProjectionSingular);
        assert_eq!(work.small_factorizations, 1);
        assert_eq!(work.small_solves, 0);
    }

    #[test]
    fn common_block_zero_rhs_and_nonfinite_input() {
        let (a, _) = system(3);
        let mut work = Work::default();
        assert_eq!(
            small_solve(&a, &[0.0; 24], &mut work).unwrap(),
            vec![0.0; 24]
        );
        let mut b = vec![0.0; 24];
        b[2] = f64::NAN;
        let before = work.small_factorizations;
        assert_eq!(
            small_solve(&a, &b, &mut work).unwrap_err().kind,
            FailureKind::NonFinite
        );
        assert_eq!(work.small_factorizations, before);
    }
}
