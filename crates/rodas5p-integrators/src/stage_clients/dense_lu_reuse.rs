//! Dense LU stage client with reused factor storage and opt-in threads
//! (loop node V18).
//!
//! [`DenseLuReuseStageClient`] is the dense client of the client kit
//! ([`DenseLuStageClient`]) with a faer path that allocates nothing per
//! factorization. Under
//! [`crate::integrate_rodas5p_fast_client_observed_with_atol`] it reproduces
//! the protected dense driver
//! ([`crate::integrate_rodas5p_fast_observed_with_atol`]) bit for bit when
//! its mode is the protected policy's ([`DenseLuMode::protected_choice`])
//! and no worker threads are requested.
//!
//! # What the protected faer path does, and what is kept
//!
//! `rodas5p_core::LuFactorization::new(&w)` (`rodas5p-core/src/matrix.rs`)
//! is `w.to_faer().partial_piv_lu()`. In faer 0.24.4 that is:
//!
//! 1. `Mat::from_fn(n, n, |i, j| w[(i, j)])`: a new column-major matrix;
//! 2. `PartialPivLu::new`: a copy of it (`to_owned`), two `vec![0usize; n]`
//!    for the permutation, one `MemBuffer` for the transpositions;
//! 3. `linalg::lu::partial_pivoting::factor::lu_in_place(lu, perm, perm_inv,
//!    get_global_parallelism(), stack, Default::default())`: recursive
//!    blocked LU with partial (row) pivoting, parameters
//!    `recursion_threshold = 16`, `block_size = 64`,
//!    `par_threshold = 128 * 128`. The workspace builds faer without its
//!    `rayon` feature, so the global parallelism is `Par::Seq` and `Par` has
//!    no other variant;
//! 4. `split_LU`: a third `n x n` matrix for `U`, the upper triangle copied
//!    into it, the upper triangle of `L` zeroed, its diagonal set to one.
//!
//! The solve is `partial_pivoting::solve::solve_in_place_with_conj(L, U, P,
//! Conj::No, rhs, Par::Seq, stack)` on a retained `n x 1` matrix: a row
//! permutation, a unit lower triangular solve with `L`, an upper triangular
//! solve with `U`.
//!
//! This client calls the same `lu_in_place` with the same arguments on a
//! faer `Mat` it owns (the layout of the protected matrices: 64-byte aligned
//! columns, column stride `n` rounded up to 8) and the same
//! `solve_in_place_with_conj`. Two steps are left out because they change no
//! value a kernel reads:
//!
//! * `W = I/(h gamma) - J` is written straight into the column-major matrix
//!   (`-j`, then `+ inv_h_gamma` on the diagonal: the operations of the
//!   protected assembly) instead of into a row-major matrix that is then
//!   copied twice;
//! * the factors are not split. faer's triangular solves recurse on the
//!   top-left, bottom-left and bottom-right blocks and their base cases read
//!   the strict lower triangle (unit solve) or the lower triangle with the
//!   diagonal (the upper solve is the lower solve of the reversed matrix),
//!   so the combined matrix is passed as `L` and as `U`.
//!
//! The identity with the protected driver is a tested property
//! (`tests/stage_clients_dense_lu_reuse_identity.rs`), not a consequence of
//! this description alone.
//!
//! # Threads
//!
//! [`DenseLuReuseStageClient::with_factor_workers`] moves the factorization
//! to a thread pool owned by the client (2 or more workers; nothing global
//! is set and no thread exists without the call). faer's own threaded
//! factorization is not available in this workspace: it needs the cargo
//! feature `rayon` of faer, which the workspace manifest leaves off. The
//! threaded path therefore runs faer's recursion in this module: panels of
//! at most [`DENSE_LU_REUSE_SEQUENTIAL_PANEL_COLUMNS`] columns go to the
//! same sequential `lu_in_place`, and above them the triangular solve, the
//! matrix product and the row swaps of each level are split over the
//! workers, each part with faer's sequential kernel. The factors are a
//! partial-pivoting LU of the same matrix; they are not promised to equal
//! the sequential ones bit for bit. The solves stay sequential.
//!
//! # Allocation and errors
//!
//! Every buffer is allocated in the constructor (and the pool in
//! [`DenseLuReuseStageClient::with_factor_workers`]); `factor` and
//! `solve_in_place` allocate nothing. A problem without an in-place Jacobian
//! callback allocates one matrix per linearization inside
//! [`OdeProblem::dense_jacobian_into`], as in the protected driver.
//!
//! A singular `W` is reported as the protected faer path reports it: the
//! factorization itself does not fail, and the first solve whose result has
//! a NaN or an infinite entry returns [`CoreError::LinearSolve`]. In the
//! in-place mode the factorization returns that error, again as the
//! protected driver does.

mod threaded;

use faer::dyn_stack::{MemBuffer, MemStack};
use faer::linalg::lu::partial_pivoting::{factor as faer_lu_factor, solve as faer_lu_solve};
use faer::perm::PermRef;
use faer::{Conj, Mat, Par};
use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};

use super::dense_lu::{DenseLuMode, DenseLuStageClient};
use crate::{OdeProblem, StageLinearSystem};

pub use threaded::{
    DENSE_LU_REUSE_CHUNK_FLOPS, DENSE_LU_REUSE_CHUNK_SWAPS, DENSE_LU_REUSE_SEQUENTIAL_PANEL_COLUMNS,
};

/// The largest worker count [`DenseLuReuseStageClient::with_factor_workers`]
/// accepts.
pub const DENSE_LU_REUSE_MAX_FACTOR_WORKERS: usize = 64;

/// Rows of the Jacobian copied per pass of the transposing assembly of `W`.
const ASSEMBLY_ROWS: usize = 32;

/// The dense stage client with client-owned factor storage. See the module
/// documentation.
pub struct DenseLuReuseStageClient {
    kernel: Kernel,
}

enum Kernel {
    /// The protected policy's in-place LU: the client-kit client, which
    /// already allocates nothing.
    InPlace(Box<DenseLuStageClient>),
    Faer(Box<FaerReuse>),
}

/// The faer path with reused storage.
struct FaerReuse {
    n: usize,
    /// The Jacobian at the last linearized state, row-major, kept across
    /// steps (an in-place Jacobian callback rewrites its own pattern).
    jacobian: DenseMatrix,
    /// Column-major `W`, then its combined factors: `L` strictly below the
    /// diagonal (unit diagonal implied), `U` on and above it.
    lu: Mat<f64>,
    /// `perm_forward[i]` is the row of `W` at position `i` of `P W`;
    /// `perm_inverse` is its inverse.
    perm_forward: Vec<usize>,
    perm_inverse: Vec<usize>,
    /// The transpositions of faer's `lu_in_place`.
    factor_scratch: MemBuffer,
    /// The right-hand side of a solve, in faer's layout.
    rhs: Mat<f64>,
    /// The scratch of the row permutation of a solve.
    solve_scratch: MemBuffer,
    /// Whether `lu` holds factors of the last `W`.
    factored: bool,
    /// The opt-in threaded factorization; `None` is faer's sequential call.
    pool: Option<threaded::FactorPool>,
}

impl DenseLuReuseStageClient {
    /// A client of order `n` with the LU `mode` and no worker threads.
    /// Refuses `n == 0` and an `n x n` matrix that cannot be addressed.
    pub fn new(n: usize, mode: DenseLuMode) -> CoreResult<Self> {
        let kernel = match mode {
            DenseLuMode::InPlace => Kernel::InPlace(Box::new(DenseLuStageClient::new(n, mode)?)),
            DenseLuMode::Faer => Kernel::Faer(Box::new(FaerReuse::new(n)?)),
        };
        Ok(Self { kernel })
    }

    /// A client whose mode is the protected driver's choice for the run of
    /// `problem` that starts at `(t0, y0)`
    /// ([`DenseLuMode::protected_choice_at`]).
    pub fn for_protected_run(problem: &OdeProblem, t0: f64, y0: &[f64]) -> CoreResult<Self> {
        let mode = DenseLuMode::protected_choice_at(problem, t0, y0)?;
        Self::new(problem.dimension, mode)
    }

    /// The same client with the factorization on `workers` threads of a
    /// pool the client owns. One worker is faer's sequential call (no pool,
    /// the bits of the protected driver). Two or more build the pool here;
    /// they require [`DenseLuMode::Faer`] and at most
    /// [`DENSE_LU_REUSE_MAX_FACTOR_WORKERS`] workers. The factors of the
    /// last factorization are discarded.
    pub fn with_factor_workers(mut self, workers: usize) -> CoreResult<Self> {
        if workers == 0 || workers > DENSE_LU_REUSE_MAX_FACTOR_WORKERS {
            return Err(CoreError::InvalidInput(format!(
                "the dense stage client takes 1 to {DENSE_LU_REUSE_MAX_FACTOR_WORKERS} \
                 factorization workers, got {workers}"
            )));
        }
        match &mut self.kernel {
            Kernel::InPlace(_) if workers > 1 => {
                return Err(CoreError::InvalidInput(
                    "factorization workers need the faer mode of the dense stage client".into(),
                ));
            }
            Kernel::InPlace(_) => {}
            Kernel::Faer(faer) => {
                faer.factored = false;
                faer.pool = if workers > 1 {
                    Some(threaded::FactorPool::new(faer.n, workers)?)
                } else {
                    None
                };
            }
        }
        Ok(self)
    }

    /// The LU of this client.
    pub fn mode(&self) -> DenseLuMode {
        match &self.kernel {
            Kernel::InPlace(_) => DenseLuMode::InPlace,
            Kernel::Faer(_) => DenseLuMode::Faer,
        }
    }

    /// The threads of the factorization (1 without a pool).
    pub fn factor_workers(&self) -> usize {
        match &self.kernel {
            Kernel::InPlace(_) => 1,
            Kernel::Faer(faer) => faer.pool.as_ref().map_or(1, threaded::FactorPool::workers),
        }
    }

    /// The Jacobian at the last linearized state (zero before the first
    /// linearization).
    pub fn jacobian(&self) -> &DenseMatrix {
        match &self.kernel {
            Kernel::InPlace(client) => client.jacobian(),
            Kernel::Faer(faer) => &faer.jacobian,
        }
    }

    /// Entry `(i, j)` of the combined factors of the last `W` in the faer
    /// mode: the multiplier of `L` for `i > j`, the entry of `U` otherwise.
    /// `None` in the in-place mode, without factors, or out of range.
    pub fn combined_factor(&self, i: usize, j: usize) -> Option<f64> {
        match &self.kernel {
            Kernel::Faer(faer) if faer.factored && i < faer.n && j < faer.n => {
                Some(faer.lu.col_as_slice(j)[i])
            }
            _ => None,
        }
    }

    /// The row permutation of the last factorization in the faer mode:
    /// entry `i` is the row of `W` that the factors hold at position `i`.
    /// `None` in the in-place mode or without factors.
    pub fn row_permutation(&self) -> Option<&[usize]> {
        match &self.kernel {
            Kernel::Faer(faer) if faer.factored => Some(&faer.perm_forward),
            _ => None,
        }
    }
}

impl FaerReuse {
    fn new(n: usize) -> CoreResult<Self> {
        let addressable = n
            .checked_mul(n)
            .filter(|len| *len <= isize::MAX as usize / std::mem::size_of::<f64>());
        if n == 0 || addressable.is_none() {
            return Err(CoreError::InvalidInput(format!(
                "the dense stage client needs a positive addressable order, got {n}"
            )));
        }
        let factor_scratch =
            faer_lu_factor::lu_in_place_scratch::<usize, f64>(n, n, Par::Seq, Default::default());
        let solve_scratch = faer_lu_solve::solve_in_place_scratch::<usize, f64>(n, 1, Par::Seq);
        Ok(Self {
            n,
            jacobian: DenseMatrix::zeros(n, n),
            lu: Mat::zeros(n, n),
            perm_forward: vec![0; n],
            perm_inverse: vec![0; n],
            factor_scratch: MemBuffer::new(factor_scratch),
            rhs: Mat::zeros(n, 1),
            solve_scratch: MemBuffer::new(solve_scratch),
            factored: false,
            pool: None,
        })
    }

    /// `W = I/(h gamma) - J` in the column-major matrix: every entry is
    /// `-j`, the diagonal then gets `+ inv_h_gamma`, as in the protected
    /// assembly. The Jacobian is read a block of rows at a time so that
    /// both the reads and the writes stay within a few cache lines per
    /// column.
    fn assemble(&mut self, inv_h_gamma: f64) {
        let n = self.n;
        let jacobian = self.jacobian.as_slice();
        let mut first = 0;
        while first < n {
            let last = (first + ASSEMBLY_ROWS).min(n);
            let rows = &jacobian[first * n..last * n];
            for j in 0..n {
                let column = &mut self.lu.col_as_slice_mut(j)[first..last];
                for (w, row) in column.iter_mut().zip(rows.chunks_exact(n)) {
                    *w = -row[j];
                }
            }
            first = last;
        }
        for j in 0..n {
            self.lu.col_as_slice_mut(j)[j] += inv_h_gamma;
        }
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) {
        self.factored = false;
        self.assemble(inv_h_gamma);
        counters.direct_factorizations += 1;
        match self.pool.as_mut() {
            None => {
                faer_lu_factor::lu_in_place(
                    self.lu.as_mut(),
                    &mut self.perm_forward,
                    &mut self.perm_inverse,
                    Par::Seq,
                    MemStack::new(&mut self.factor_scratch),
                    Default::default(),
                );
            }
            Some(pool) => pool.factor(
                self.lu.as_mut(),
                &mut self.perm_forward,
                &mut self.perm_inverse,
            ),
        }
        self.factored = true;
    }

    /// The checks, the kernel call and the error kinds of
    /// `LuFactorization::solve_in_place_with_workspace`.
    fn solve(&mut self, b: &mut [f64]) -> CoreResult<()> {
        if !self.factored {
            return Err(not_factored());
        }
        if !b.iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite("LU RHS contains NaN/Inf".into()));
        }
        self.rhs.col_as_slice_mut(0).copy_from_slice(b);
        let permutation = PermRef::new_checked(&self.perm_forward, &self.perm_inverse, self.n);
        faer_lu_solve::solve_in_place_with_conj(
            self.lu.as_ref(),
            self.lu.as_ref(),
            permutation,
            Conj::No,
            self.rhs.as_mut(),
            Par::Seq,
            MemStack::new(&mut self.solve_scratch),
        );
        b.copy_from_slice(self.rhs.col_as_slice(0));
        if b.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::LinearSolve("LU produced NaN/Inf".into()))
        }
    }
}

impl StageLinearSystem for DenseLuReuseStageClient {
    fn dimension(&self) -> usize {
        match &self.kernel {
            Kernel::InPlace(client) => client.dimension(),
            Kernel::Faer(faer) => faer.n,
        }
    }

    /// The dense Jacobian at `(t, y)` through
    /// [`OdeProblem::dense_jacobian_into`], with the count of
    /// `jacobian_builds` put back to the driver's charge, as in
    /// [`DenseLuStageClient`].
    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        match &mut self.kernel {
            Kernel::InPlace(client) => client.linearize(problem, t, y, counters),
            Kernel::Faer(faer) => {
                let charged_by_driver = counters.jacobian_builds;
                let built = problem.dense_jacobian_into(t, y, &mut faer.jacobian, counters);
                counters.jacobian_builds = charged_by_driver;
                built
            }
        }
    }

    /// `W = I/(h gamma) - J` and its LU, charging `direct_factorizations`
    /// before the factorization as the protected driver does.
    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        match &mut self.kernel {
            Kernel::InPlace(client) => client.factor(inv_h_gamma, counters),
            Kernel::Faer(faer) => {
                faer.factor(inv_h_gamma, counters);
                Ok(())
            }
        }
    }

    /// The stage solve with the stored factors, charging
    /// `direct_solve_calls`. A solve without factors is a
    /// [`CoreError::LinearSolve`].
    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        match &mut self.kernel {
            Kernel::InPlace(client) => client.solve_in_place(b, counters),
            Kernel::Faer(faer) => {
                if b.len() != faer.n {
                    return Err(CoreError::Dimension(
                        "dense stage solve shape mismatch".into(),
                    ));
                }
                counters.direct_solve_calls += 1;
                faer.solve(b)
            }
        }
    }
}

fn not_factored() -> CoreError {
    CoreError::LinearSolve("the dense stage client has no factors of W".into())
}
