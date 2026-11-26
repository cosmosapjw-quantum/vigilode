//! Sparse direct stage client (loop node V11).
//!
//! [`SparseLuStageClient`] solves the stage systems `W x = b`, `W = I/(h gamma) - J`, of
//! [`crate::integrate_rodas5p_fast_client_observed_with_atol`] with
//! [`rodas5p_core::SparseDirectSolver`] (faer's sparse LU with partial pivoting):
//!
//! * the sparsity pattern of `J` (compressed columns, the diagonal included) and the symbolic
//!   analysis (fill-reducing column ordering) are fixed at construction;
//! * [`StageLinearSystem::linearize`] writes the Jacobian values on that pattern through a
//!   fill callback (for a [`MolOperator`], [`MolOperator::jacobian_values_into`]);
//! * [`StageLinearSystem::factor`] forms `W` on the pattern and runs one numeric
//!   factorisation per call, that is once per attempt;
//! * [`StageLinearSystem::solve_in_place`] runs the two triangular solves.
//!
//! # Counters
//!
//! The client charges `direct_factorizations` before every numeric factorisation and
//! `direct_solve_calls` before every solve, as the dense reference client does, so a failed
//! call stays charged. The solver's own counters are kept separately
//! ([`SparseLuStageClient::factorizations`]).
//!
//! # Errors
//!
//! A pattern without a diagonal entry is refused at construction. A non-finite Jacobian
//! value is a [`CoreError::NonFinite`] of `linearize`, a failed numeric factorisation a
//! [`CoreError::LinearSolve`] of `factor`: the driver rejects the attempt and retries with a
//! smaller step.
//!
//! # Reproducibility and allocation
//!
//! The factorisation runs sequentially (`Par::Seq`), so two runs on the same host give the
//! same bits. Every buffer of the client is allocated at construction. Whether the numeric
//! factorisation allocates is decided inside faer: its simplicial LU reuses its storage once
//! the fill of a pivot sequence has been seen, its supernodal LU (chosen by faer for patterns
//! with large fill per entry) allocates per factorisation. The allocation contract and its
//! measurement are in `tests/stage_clients_sparse_lu_alloc.rs` and in
//! `research/production_loop_20261116/battery_v4/`.

use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, SparseDirectSolver, WorkCounters};

use crate::mol::MolOperator;
use crate::{OdeProblem, StageLinearSystem};

/// Writes the Jacobian at `(t, y)` into the values of a fixed compressed-column pattern.
pub type SparseJacobianFill = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// A [`StageLinearSystem`] over [`SparseDirectSolver`]: symbolic analysis once, one numeric
/// factorisation per attempt. See the module documentation.
pub struct SparseLuStageClient {
    n: usize,
    col_ptr: Vec<usize>,
    row_idx: Vec<usize>,
    diagonal: Vec<usize>,
    fill: SparseJacobianFill,
    jacobian: Vec<f64>,
    w: Vec<f64>,
    solver: SparseDirectSolver,
    factored: bool,
}

impl SparseLuStageClient {
    /// A client for the compressed-column pattern `(col_ptr, row_idx)` of order `n` (rows
    /// strictly increasing per column, every diagonal entry present) and the callback that
    /// fills its values.
    pub fn new(
        n: usize,
        col_ptr: Vec<usize>,
        row_idx: Vec<usize>,
        fill: SparseJacobianFill,
    ) -> CoreResult<Self> {
        let solver = SparseDirectSolver::try_new_symbolic(n, &col_ptr, &row_idx)?;
        let diagonal = (0..n)
            .map(|c| {
                let start = col_ptr[c];
                row_idx[start..col_ptr[c + 1]]
                    .binary_search(&c)
                    .ok()
                    .map(|k| start + k)
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                CoreError::InvalidInput(
                    "the sparse stage client needs every diagonal entry in the pattern".into(),
                )
            })?;
        let nnz = row_idx.len();
        Ok(Self {
            n,
            col_ptr,
            row_idx,
            diagonal,
            fill,
            jacobian: vec![0.0; nnz],
            w: vec![0.0; nnz],
            solver,
            factored: false,
        })
    }

    /// A client for an operator's declared pattern, filled by
    /// [`MolOperator::jacobian_values_into`].
    pub fn for_operator(op: Arc<dyn MolOperator>) -> CoreResult<Self> {
        let pattern = op.structure().pattern();
        let (n, col_ptr, row_idx) = (
            pattern.dimension(),
            pattern.col_ptr().to_vec(),
            pattern.row_idx().to_vec(),
        );
        Self::new(
            n,
            col_ptr,
            row_idx,
            Arc::new(move |t, y, values| op.jacobian_values_into(t, y, values)),
        )
    }

    /// Stored entries of the pattern.
    pub fn nnz(&self) -> usize {
        self.row_idx.len()
    }

    /// Successful numeric factorisations of the underlying solver.
    pub fn factorizations(&self) -> u64 {
        self.solver.factorizations()
    }

    /// Successful solves of the underlying solver.
    pub fn solves(&self) -> u64 {
        self.solver.solves()
    }

    /// The Jacobian values of the last linearization (zero before the first).
    pub fn jacobian_values(&self) -> &[f64] {
        &self.jacobian
    }
}

impl StageLinearSystem for SparseLuStageClient {
    fn dimension(&self) -> usize {
        self.n
    }

    fn linearize(
        &mut self,
        _problem: &OdeProblem,
        t: f64,
        y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.factored = false;
        if y.len() != self.n {
            return Err(CoreError::Dimension(
                "sparse stage client state shape mismatch".into(),
            ));
        }
        (self.fill)(t, y, &mut self.jacobian)?;
        if !self.jacobian.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "sparse stage client Jacobian contains NaN or Inf".into(),
            ));
        }
        Ok(())
    }

    /// `W = I/(h gamma) - J` on the pattern and one numeric factorisation, charged to
    /// `direct_factorizations` before it runs.
    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        self.factored = false;
        for (w, j) in self.w.iter_mut().zip(&self.jacobian) {
            *w = -j;
        }
        for &d in &self.diagonal {
            self.w[d] += inv_h_gamma;
        }
        counters.direct_factorizations += 1;
        let mut own = WorkCounters::default();
        self.solver
            .refactorize(&self.col_ptr, &self.row_idx, &self.w, &mut own)?;
        self.factored = true;
        Ok(())
    }

    /// The two triangular solves, charged to `direct_solve_calls` before they run.
    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if b.len() != self.n {
            return Err(CoreError::Dimension(
                "sparse stage solve shape mismatch".into(),
            ));
        }
        counters.direct_solve_calls += 1;
        if !self.factored {
            return Err(CoreError::LinearSolve(
                "the sparse stage client has no factors of W".into(),
            ));
        }
        let mut own = WorkCounters::default();
        self.solver.solve_in_place(b, &mut own)
    }
}
