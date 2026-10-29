use faer::dyn_stack::{MemBuffer, MemStack};
use faer::sparse::linalg::lu::{
    LuRef, LuSymbolicParams, NumericLu, SymbolicLu, factorize_symbolic_lu,
};
use faer::sparse::{SparseColMatRef, SymbolicSparseColMatRef};
use faer::{Conj, MatMut, Par};

use crate::{CoreError, CoreResult, WorkCounters};

/// Sparse direct LU solver wrapping `faer::sparse::linalg::lu` with reuse of
/// the symbolic factorization and numeric refactorization.
pub struct SparseDirectSolver {
    dimension: usize,
    nnz_a: usize,
    col_ptr: Vec<usize>,
    row_idx: Vec<usize>,
    numeric_ready: bool,
    symbolic: SymbolicLu<usize>,
    numeric: NumericLu<usize, f64>,
    numeric_scratch: MemBuffer,
    solve_scratch: MemBuffer,
    factorizations: u64,
    solves: u64,
    solve_operations: u64,
}

impl SparseDirectSolver {
    /// Create a new sparse direct solver from the symbolic pattern of A (CSC format).
    pub fn try_new_symbolic(
        dimension: usize,
        col_ptr: &[usize],
        row_idx: &[usize],
    ) -> CoreResult<Self> {
        if dimension == 0 || col_ptr.len() != dimension.saturating_add(1) {
            return Err(CoreError::Dimension(format!(
                "col_ptr length {} does not match dimension + 1 ({})",
                col_ptr.len(),
                dimension.saturating_add(1)
            )));
        }
        if col_ptr[0] != 0 || col_ptr[dimension] != row_idx.len() {
            return Err(CoreError::InvalidInput(
                "col_ptr must start at 0 and end at row_idx.len()".into(),
            ));
        }
        if col_ptr.windows(2).any(|p| p[0] > p[1]) {
            return Err(CoreError::InvalidInput(
                "CSC column pointers must be monotone".into(),
            ));
        }
        for col in 0..dimension {
            let rows = &row_idx[col_ptr[col]..col_ptr[col + 1]];
            if rows.iter().any(|&row| row >= dimension) || rows.windows(2).any(|p| p[0] >= p[1]) {
                return Err(CoreError::InvalidInput(
                    "CSC rows must be in range and strictly increasing in each column".into(),
                ));
            }
        }

        let sym_mat =
            SymbolicSparseColMatRef::new_checked(dimension, dimension, col_ptr, None, row_idx);

        let symbolic = factorize_symbolic_lu(sym_mat, LuSymbolicParams::default())
            .map_err(|e| CoreError::LinearSolve(format!("Symbolic LU failed: {e:?}")))?;

        let par = Par::Seq;
        let num_scratch_req = symbolic.factorize_numeric_lu_scratch::<f64>(par, Default::default());
        let numeric_scratch = MemBuffer::new(num_scratch_req);

        let solve_scratch_req = symbolic.solve_in_place_scratch::<f64>(1, par);
        let solve_scratch = MemBuffer::new(solve_scratch_req);

        Ok(Self {
            dimension,
            nnz_a: row_idx.len(),
            col_ptr: col_ptr.to_vec(),
            row_idx: row_idx.to_vec(),
            numeric_ready: false,
            symbolic,
            numeric: NumericLu::new(),
            numeric_scratch,
            solve_scratch,
            factorizations: 0,
            solves: 0,
            solve_operations: 0,
        })
    }

    /// Dimension of the square system.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Number of nonzero entries in the original input matrix.
    pub fn nnz_a(&self) -> usize {
        self.nnz_a
    }

    /// Refactorize numerically in-place using the existing symbolic factorization.
    pub fn refactorize(
        &mut self,
        col_ptr: &[usize],
        row_idx: &[usize],
        values: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        // A failed attempt must never leave a previously factored matrix usable
        // as though it represented this new request.
        self.numeric_ready = false;
        if col_ptr.len() != self.dimension + 1
            || row_idx.len() != self.nnz_a
            || values.len() != self.nnz_a
        {
            return Err(CoreError::Dimension(
                "Sparse matrix structure mismatch with symbolic factorization".into(),
            ));
        }
        if col_ptr != self.col_ptr || row_idx != self.row_idx {
            return Err(CoreError::InvalidInput(
                "CSC pattern differs from the retained symbolic factorization".into(),
            ));
        }
        if !values.iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite(
                "Sparse matrix contains NaN or Inf".into(),
            ));
        }

        let sym_mat = SymbolicSparseColMatRef::new_checked(
            self.dimension,
            self.dimension,
            col_ptr,
            None,
            row_idx,
        );
        let mat = SparseColMatRef::<'_, usize, f64>::new(sym_mat, values);

        let par = Par::Seq;
        let stack = MemStack::new(&mut self.numeric_scratch);
        self.symbolic
            .factorize_numeric_lu(&mut self.numeric, mat, par, stack, Default::default())
            .map_err(|e| CoreError::LinearSolve(format!("Numeric LU refactorize failed: {e:?}")))?;

        self.factorizations += 1;
        self.numeric_ready = true;
        counters.direct_factorizations += 1;
        Ok(())
    }

    /// Solve $A x = b$ in place, replacing `rhs` with the solution.
    pub fn solve_in_place(
        &mut self,
        rhs: &mut [f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if !self.numeric_ready {
            return Err(CoreError::LinearSolve(
                "sparse solve requires a successful current numeric factorization".into(),
            ));
        }
        if rhs.len() != self.dimension {
            return Err(CoreError::Dimension(format!(
                "RHS dimension {} does not match system dimension {}",
                rhs.len(),
                self.dimension
            )));
        }
        if !rhs.iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite(
                "Sparse RHS contains NaN or Inf".into(),
            ));
        }

        let par = Par::Seq;
        let lu_ref = LuRef::new_unchecked(&self.symbolic, &self.numeric);
        let mut rhs_mat = MatMut::from_column_major_slice_mut(rhs, self.dimension, 1);
        let stack = MemStack::new(&mut self.solve_scratch);

        lu_ref.solve_in_place_with_conj(Conj::No, rhs_mat.as_mut(), par, stack);

        self.solves += 1;
        self.solve_operations += (2 * self.nnz_a) as u64;
        counters.direct_solve_calls += 1;
        if !rhs.iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite(
                "Sparse solve produced NaN or Inf".into(),
            ));
        }
        Ok(())
    }

    /// Solve $A x = b$, writing the result to `out`.
    pub fn solve(
        &mut self,
        rhs: &[f64],
        out: &mut [f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if rhs.len() != self.dimension || out.len() != self.dimension {
            return Err(CoreError::Dimension(
                "RHS or out length does not match system dimension".into(),
            ));
        }
        out.copy_from_slice(rhs);
        self.solve_in_place(out, counters)
    }

    /// Total number of numeric factorizations performed.
    pub fn factorizations(&self) -> u64 {
        self.factorizations
    }

    /// Total number of solves performed.
    pub fn solves(&self) -> u64 {
        self.solves
    }

    /// Cumulative operation count estimate.
    pub fn solve_operations(&self) -> u64 {
        self.solve_operations
    }
}
