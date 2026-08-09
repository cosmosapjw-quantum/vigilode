use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{CoreError, CoreResult, DenseMatrix, WorkCounters, direct_solve};

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

pub trait LinearOperator: Send + Sync {
    fn dimension(&self) -> usize;
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()>;

    /// Apply one operator to several independent right-hand-side rows.
    ///
    /// The default implementation is deliberately serial and allocation-free with respect to the
    /// caller-provided output storage.  Backends that own a bounded thread pool or a genuine
    /// block kernel may override this method without changing Krylov solver semantics.
    fn apply_rows(&self, inputs: &[Vec<f64>], outputs: &mut [Vec<f64>]) -> CoreResult<()> {
        let n = self.dimension();
        if inputs.len() != outputs.len()
            || inputs.iter().any(|row| row.len() != n)
            || outputs.iter().any(|row| row.len() != n)
        {
            return Err(CoreError::Dimension(
                "linear-operator row batch shape mismatch".into(),
            ));
        }
        for (input, output) in inputs.iter().zip(outputs) {
            self.apply(input, output)?;
        }
        Ok(())
    }

    fn explicit(&self) -> Option<&DenseMatrix> {
        None
    }
    fn token(&self) -> u64;
}

#[derive(Clone)]
pub struct DenseOperator {
    matrix: DenseMatrix,
    token: u64,
}

impl DenseOperator {
    pub fn new(matrix: DenseMatrix) -> CoreResult<Self> {
        if matrix.nrows() != matrix.ncols() {
            return Err(CoreError::Dimension(
                "linear operator must be square".into(),
            ));
        }
        Ok(Self {
            matrix,
            token: NEXT_TOKEN.fetch_add(1, Ordering::Relaxed),
        })
    }
}

impl LinearOperator for DenseOperator {
    fn dimension(&self) -> usize {
        self.matrix.nrows()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.matrix.matvec_into(x, y)
    }
    fn explicit(&self) -> Option<&DenseMatrix> {
        Some(&self.matrix)
    }
    fn token(&self) -> u64 {
        self.token
    }
}

pub struct ClosureOperator<F>
where
    F: Fn(&[f64], &mut [f64]) -> CoreResult<()> + Send + Sync,
{
    n: usize,
    f: F,
    token: u64,
}

impl<F> ClosureOperator<F>
where
    F: Fn(&[f64], &mut [f64]) -> CoreResult<()> + Send + Sync,
{
    pub fn new(n: usize, f: F) -> Self {
        Self {
            n,
            f,
            token: NEXT_TOKEN.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl<F> LinearOperator for ClosureOperator<F>
where
    F: Fn(&[f64], &mut [f64]) -> CoreResult<()> + Send + Sync,
{
    fn dimension(&self) -> usize {
        self.n
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        (self.f)(x, y)
    }
    fn token(&self) -> u64 {
        self.token
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyCategory {
    Krylov,
    Refresh,
    Diagnostic,
    Block,
}

pub fn apply_counted(
    op: &dyn LinearOperator,
    x: &[f64],
    y: &mut [f64],
    counters: &mut WorkCounters,
    category: ApplyCategory,
) -> CoreResult<()> {
    op.apply(x, y)?;
    match category {
        ApplyCategory::Krylov => counters.linear_matvecs += 1,
        ApplyCategory::Refresh => counters.recycle_refresh_matvecs += 1,
        ApplyCategory::Diagnostic => counters.diagnostic_matvecs += 1,
        ApplyCategory::Block => counters.block_matvecs += 1,
    }
    Ok(())
}

pub trait Preconditioner: Send + Sync {
    fn dimension(&self) -> usize;
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()>;
    fn is_identity(&self) -> bool {
        false
    }
}

#[derive(Default)]
pub struct IdentityPreconditioner {
    n: usize,
}
impl IdentityPreconditioner {
    pub fn new(n: usize) -> Self {
        Self { n }
    }
}
impl Preconditioner for IdentityPreconditioner {
    fn dimension(&self) -> usize {
        self.n
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.n || y.len() != self.n {
            return Err(CoreError::Dimension(
                "identity preconditioner shape mismatch".into(),
            ));
        }
        y.copy_from_slice(x);
        Ok(())
    }
}

pub struct JacobiPreconditioner {
    inv_diag: Vec<f64>,
}
impl JacobiPreconditioner {
    pub fn from_matrix(a: &DenseMatrix) -> CoreResult<Self> {
        let diag = a.diagonal()?;
        let scale = diag
            .iter()
            .fold(0.0_f64, |acc, v| acc.max(v.abs()))
            .max(1.0);
        let tol = f64::EPSILON * scale;
        if diag.iter().any(|v| v.abs() <= tol) {
            return Err(CoreError::LinearSolve(
                "Jacobi preconditioner has a zero diagonal".into(),
            ));
        }
        Ok(Self {
            inv_diag: diag.into_iter().map(|v| 1.0 / v).collect(),
        })
    }
}
impl Preconditioner for JacobiPreconditioner {
    fn dimension(&self) -> usize {
        self.inv_diag.len()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.inv_diag.len() || y.len() != self.inv_diag.len() {
            return Err(CoreError::Dimension("Jacobi shape mismatch".into()));
        }
        for i in 0..x.len() {
            y[i] = self.inv_diag[i] * x[i];
        }
        Ok(())
    }
}

pub struct DirectPreconditioner {
    matrix: DenseMatrix,
}
impl DirectPreconditioner {
    pub fn new(matrix: DenseMatrix) -> CoreResult<Self> {
        if matrix.nrows() != matrix.ncols() {
            return Err(CoreError::Dimension("direct PC square".into()));
        }
        Ok(Self { matrix })
    }
}
impl Preconditioner for DirectPreconditioner {
    fn dimension(&self) -> usize {
        self.matrix.nrows()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let sol = direct_solve(&self.matrix, x)?;
        y.copy_from_slice(&sol);
        Ok(())
    }
}

pub struct ShiftedOperator {
    mass: Option<DenseMatrix>,
    jacobian: Arc<dyn LinearOperator>,
    h_gamma: f64,
    explicit: Option<DenseMatrix>,
    token: u64,
}

impl ShiftedOperator {
    pub fn new(
        mass: Option<DenseMatrix>,
        jacobian: Arc<dyn LinearOperator>,
        h: f64,
        gamma: f64,
    ) -> CoreResult<Self> {
        let n = jacobian.dimension();
        if let Some(m) = &mass
            && (m.nrows() != n || m.ncols() != n)
        {
            return Err(CoreError::Dimension("mass matrix shape mismatch".into()));
        }
        let h_gamma = h * gamma;
        let explicit = jacobian.explicit().map(|j| {
            let m = mass.clone().unwrap_or_else(|| DenseMatrix::identity(n));
            m.sub(&j.scale(h_gamma)).expect("validated dimensions")
        });
        Ok(Self {
            mass,
            jacobian,
            h_gamma,
            explicit,
            token: NEXT_TOKEN.fetch_add(1, Ordering::Relaxed),
        })
    }
    pub fn jacobian(&self) -> &Arc<dyn LinearOperator> {
        &self.jacobian
    }
    pub fn h_gamma(&self) -> f64 {
        self.h_gamma
    }
}
impl LinearOperator for ShiftedOperator {
    fn dimension(&self) -> usize {
        self.jacobian.dimension()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let n = self.dimension();
        if x.len() != n || y.len() != n {
            return Err(CoreError::Dimension(
                "shifted operator shape mismatch".into(),
            ));
        }
        self.jacobian.apply(x, y)?;
        if let Some(mass) = &self.mass {
            for (i, y_i) in y.iter_mut().enumerate() {
                let mass_x = mass.row(i).iter().zip(x).map(|(a, b)| a * b).sum::<f64>();
                *y_i = mass_x - self.h_gamma * *y_i;
            }
        } else {
            for (y_i, x_i) in y.iter_mut().zip(x) {
                *y_i = *x_i - self.h_gamma * *y_i;
            }
        }
        if y.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::NonFinite(
                "shifted operator produced NaN/Inf".into(),
            ))
        }
    }
    fn explicit(&self) -> Option<&DenseMatrix> {
        self.explicit.as_ref()
    }
    fn token(&self) -> u64 {
        self.token
    }
}

pub fn apply_preconditioner(
    p: &dyn Preconditioner,
    x: &[f64],
    y: &mut [f64],
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    p.apply(x, y)?;
    counters.preconditioner_apps += 1;
    Ok(())
}
