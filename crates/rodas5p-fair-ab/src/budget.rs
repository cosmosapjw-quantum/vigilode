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
}

impl OperatorBudget {
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            used: AtomicU64::new(0),
        }
    }

    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    fn reserve(&self) -> CoreResult<()> {
        let mut current = self.used.load(Ordering::Relaxed);
        loop {
            if current >= self.limit {
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
        let token = u64::from_str_radix(&operator_id[..16], 16)
            .map_err(|_| CoreError::InvalidInput("invalid operator digest".into()))?;
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
