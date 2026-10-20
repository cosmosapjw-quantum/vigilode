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
pub(crate) fn small_solve(a: &DenseMatrix, b: &[f64], work: &mut Work) -> Result<Vec<f64>> {
    finite(a.as_slice())?;
    finite(b)?;
    work.small_factorizations += 1;
    let lu = LuFactorization::new(a)
        .map_err(|e| Failure::new(FailureKind::ProjectionSingular, e.to_string()))?;
    let x = lu
        .solve(b)
        .map_err(|e| Failure::new(FailureKind::ProjectionSingular, e.to_string()))?;
    work.small_solves += 1;
    finite(&x)?;
    let mut residual = vec![0.0; b.len()];
    for i in 0..b.len() {
        let mut row = vec![0.0; b.len()];
        for j in 0..b.len() {
            row[j] = a[(i, j)];
        }
        residual[i] = dot(&row, &x) - b[i];
    }
    let rel = norm(&residual) / norm(b).max(f64::MIN_POSITIVE);
    if !rel.is_finite() || rel > 1e-8 {
        return Err(Failure::new(
            FailureKind::ProjectionSingular,
            format!("small solve residual {rel:e}"),
        ));
    }
    Ok(x)
}
