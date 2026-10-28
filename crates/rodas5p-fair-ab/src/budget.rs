use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LinearOperator};

pub const BUDGET_EXHAUSTED_MARKER: &str = "fair operator budget exhausted";

#[derive(Debug)]
pub struct OperatorBudget {
    limit: u64,
    used: AtomicU64,
    refused: AtomicU64,
}

impl OperatorBudget {
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            used: AtomicU64::new(0),
            refused: AtomicU64::new(0),
        }
    }

    /// Admitted inner callbacks, including callbacks that return an error.
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    /// Applications refused before entering the inner callback.
    pub fn refused(&self) -> u64 {
        self.refused.load(Ordering::Relaxed)
    }

    fn reserve(&self) -> CoreResult<()> {
        let mut current = self.used.load(Ordering::Relaxed);
        loop {
            if current >= self.limit {
                self.refused.fetch_add(1, Ordering::Relaxed);
                return Err(CoreError::LinearSolve(BUDGET_EXHAUSTED_MARKER.into()));
            }
            match self.used.compare_exchange_weak(
                current,
                current + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(next) => current = next,
            }
        }
    }
}

pub struct StableDenseOperator {
    matrix: DenseMatrix,
    token: u64,
}

impl StableDenseOperator {
    pub fn new(matrix: DenseMatrix, operator_id: &str) -> CoreResult<Self> {
        if matrix.nrows() != matrix.ncols() {
            return Err(CoreError::Dimension(
                "stable operator must be square".into(),
            ));
        }
        // `get` is None for an id shorter than 16 bytes or one whose byte 16
        // is not a char boundary; slicing would panic (abort in release).
        let token = operator_id
            .get(..16)
            .and_then(|prefix| u64::from_str_radix(prefix, 16).ok())
            .ok_or_else(|| CoreError::InvalidInput("invalid operator digest".into()))?;
        Ok(Self { matrix, token })
    }
}

impl LinearOperator for StableDenseOperator {
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

pub struct BudgetedOperator {
    inner: Arc<dyn LinearOperator>,
    budget: Arc<OperatorBudget>,
}

impl BudgetedOperator {
    pub fn new(inner: Arc<dyn LinearOperator>, limit: u64) -> Self {
        Self {
            inner,
            budget: Arc::new(OperatorBudget::new(limit)),
        }
    }
    pub fn budget(&self) -> &Arc<OperatorBudget> {
        &self.budget
    }
}

impl LinearOperator for BudgetedOperator {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.budget.reserve()?;
        self.inner.apply(x, y)
    }
    fn explicit(&self) -> Option<&DenseMatrix> {
        self.inner.explicit()
    }
    fn token(&self) -> u64 {
        self.inner.token()
    }
}
