//! Dense LU reference client (loop node V15).
//!
//! [`DenseLuStageClient`] is the dense-storage linear algebra of the
//! protected fast driver (`rodas5p_fast.rs`, `Workspace::factor` and
//! `Workspace::solve`) written as a [`StageLinearSystem`] client. Under
//! [`crate::integrate_rodas5p_fast_client_observed_with_atol`] it reproduces
//! [`crate::integrate_rodas5p_fast_observed_with_atol`] bit for bit when its
//! mode is the LU the protected driver picks for the problem
//! ([`DenseLuMode::protected_choice`]): the same Jacobian callback, the same
//! assembly of `W = I/(h gamma) - J`, the same kernels
//! ([`crate::lu_research`] for the in-place LU, [`LuFactorization`] for
//! faer) and the same counters.
//!
//! The mode is explicit. The protected driver chooses its LU from the first
//! Jacobian of a run; a client that wants the same run asks
//! [`DenseLuMode::protected_choice`] for that choice. The other mode is a
//! valid solver of the same systems whose rounding differs.
//!
//! Allocation: every buffer is allocated in [`DenseLuStageClient::new`]. In
//! [`DenseLuMode::InPlace`] no call allocates. In [`DenseLuMode::Faer`] each
//! factorization allocates inside [`LuFactorization::new`], exactly as the
//! protected driver's faer path does; the solves do not allocate. A problem
//! without an in-place Jacobian callback allocates one matrix per
//! linearization inside [`OdeProblem::dense_jacobian_into`], again as the
//! protected driver does.

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LuFactorization, LuSolveWorkspace, WorkCounters,
};

use crate::{
    OdeProblem, RODAS5P_FAST_SMALL_LU_MAX, RODAS5P_FAST_SPARSE_DENSITY_MAX, Rodas5pFastLu,
    StageLinearSystem, lu_research,
};

/// The LU of a [`DenseLuStageClient`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DenseLuMode {
    /// Row-major partial-pivoting LU in the client's own storage with row
    /// extents and zero-multiplier skipping: the protected driver's
    /// [`Rodas5pFastLu::InPlaceZeroSkipping`] (its default LU policy).
    InPlace,
    /// faer's blocked partial-pivoting LU through [`LuFactorization`]: the
    /// protected driver's [`Rodas5pFastLu::Faer`].
    Faer,
}

impl DenseLuMode {
    /// The LU the protected dense driver picks under its default options
    /// for a run whose first Jacobian is `first_jacobian` (`choose_lu` in
    /// `rodas5p_fast.rs`): in place when the order is at most
    /// [`RODAS5P_FAST_SMALL_LU_MAX`] or at most
    /// [`RODAS5P_FAST_SPARSE_DENSITY_MAX`] of the entries are nonzero,
    /// faer otherwise.
    pub fn protected_choice(first_jacobian: &DenseMatrix) -> Self {
        let n = first_jacobian.nrows();
        let nonzero = first_jacobian
            .as_slice()
            .iter()
            .filter(|v| **v != 0.0)
            .count();
        let density = nonzero as f64 / (n * n) as f64;
        if n <= RODAS5P_FAST_SMALL_LU_MAX || density <= RODAS5P_FAST_SPARSE_DENSITY_MAX {
            Self::InPlace
        } else {
            Self::Faer
        }
    }

    /// [`Self::protected_choice`] for the Jacobian of `problem` at
    /// `(t, y)`. The evaluation is charged to scratch counters, not to a
    /// run.
    pub fn protected_choice_at(problem: &OdeProblem, t: f64, y: &[f64]) -> CoreResult<Self> {
        let n = problem.dimension;
        let mut jacobian = DenseMatrix::zeros(n, n);
        problem.dense_jacobian_into(t, y, &mut jacobian, &mut WorkCounters::default())?;
        Ok(Self::protected_choice(&jacobian))
    }

    /// The protected driver's name for this LU, as its result reports it.
    pub fn protected_lu(self) -> Rodas5pFastLu {
        match self {
            Self::InPlace => Rodas5pFastLu::InPlaceZeroSkipping,
            Self::Faer => Rodas5pFastLu::Faer,
        }
    }
}

/// The dense Jacobian, `W` and its LU factors of the fast driver, owned by a
/// client. See the module documentation for the identity it keeps with the
/// protected dense driver.
pub struct DenseLuStageClient {
    n: usize,
    mode: DenseLuMode,
    /// The Jacobian at the last linearized state, kept across steps (an
    /// in-place Jacobian callback rewrites its own pattern).
    jacobian: DenseMatrix,
    /// Row-major `W`; in [`DenseLuMode::InPlace`] its factors after
    /// [`StageLinearSystem::factor`].
    w: DenseMatrix,
    /// In-place LU: the row swapped with row `k`, and per row the last
    /// column that can be nonzero and the first stored multiplier.
    pivots: Vec<usize>,
    row_end: Vec<usize>,
    l_start: Vec<usize>,
    /// Whether `w` holds in-place factors of the last `W`.
    in_place_factored: bool,
    faer_lu: Option<LuFactorization>,
    faer_solve_workspace: Option<LuSolveWorkspace>,
}

impl DenseLuStageClient {
    /// A client of order `n` with the LU `mode`. Refuses `n == 0` and an
    /// `n x n` matrix that cannot be addressed.
    pub fn new(n: usize, mode: DenseLuMode) -> CoreResult<Self> {
        let addressable = n
            .checked_mul(n)
            .filter(|len| *len <= isize::MAX as usize / std::mem::size_of::<f64>());
        if n == 0 || addressable.is_none() {
            return Err(CoreError::InvalidInput(format!(
                "the dense stage client needs a positive addressable order, got {n}"
            )));
        }
        let in_place = mode == DenseLuMode::InPlace;
        let extents = if in_place { n } else { 0 };
        Ok(Self {
            n,
            mode,
            jacobian: DenseMatrix::zeros(n, n),
            w: DenseMatrix::zeros(n, n),
            pivots: vec![0; extents],
            row_end: vec![0; extents],
            l_start: vec![n; extents],
            in_place_factored: false,
            faer_lu: None,
            faer_solve_workspace: (!in_place).then(|| LuSolveWorkspace::new(n)),
        })
    }

    /// A client whose mode is the protected driver's choice for the run of
    /// `problem` that starts at `(t0, y0)`
    /// ([`DenseLuMode::protected_choice_at`]).
    pub fn for_protected_run(problem: &OdeProblem, t0: f64, y0: &[f64]) -> CoreResult<Self> {
        let mode = DenseLuMode::protected_choice_at(problem, t0, y0)?;
        Self::new(problem.dimension, mode)
    }

    /// The LU of this client.
    pub fn mode(&self) -> DenseLuMode {
        self.mode
    }

    /// The Jacobian at the last linearized state (zero before the first
    /// linearization).
    pub fn jacobian(&self) -> &DenseMatrix {
        &self.jacobian
    }
}

impl StageLinearSystem for DenseLuStageClient {
    fn dimension(&self) -> usize {
        self.n
    }

    /// The dense Jacobian at `(t, y)` through
    /// [`OdeProblem::dense_jacobian_into`], as the protected driver builds
    /// it. That call charges `jacobian_builds` itself and the client
    /// driver has already charged this build, so the count is put back to
    /// the driver's: one per linearization, the protected driver's total.
    /// Every other counter the problem charges (a JVP-built Jacobian) is
    /// kept.
    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        let charged_by_driver = counters.jacobian_builds;
        let built = problem.dense_jacobian_into(t, y, &mut self.jacobian, counters);
        counters.jacobian_builds = charged_by_driver;
        built
    }

    /// `W = I/(h gamma) - J` and its LU, in the protected driver's order of
    /// operations: negate `J`, add `inv_h_gamma` to the diagonal, charge
    /// `direct_factorizations`, factor.
    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let n = self.n;
        self.in_place_factored = false;
        let target = self.w.as_mut_slice();
        for (w, j) in target.iter_mut().zip(self.jacobian.as_slice()) {
            *w = -j;
        }
        for i in 0..n {
            target[i * n + i] += inv_h_gamma;
        }
        counters.direct_factorizations += 1;
        match self.mode {
            DenseLuMode::InPlace => {
                for (i, row) in target.chunks_exact(n).enumerate() {
                    let last = row.iter().rposition(|v| *v != 0.0).unwrap_or(0);
                    self.row_end[i] = last.max(i);
                    self.l_start[i] = n;
                }
                lu_research::zero_skipping(
                    target,
                    n,
                    &mut self.pivots,
                    &mut self.row_end,
                    &mut self.l_start,
                )?;
                self.in_place_factored = true;
                Ok(())
            }
            DenseLuMode::Faer => {
                // The new factors are built before the old ones are dropped,
                // as in the protected driver: the order of the allocator
                // calls is part of what the client reproduces.
                self.faer_lu = Some(LuFactorization::new(&self.w)?);
                Ok(())
            }
        }
    }

    /// The stage solve with the stored factors, charging
    /// `direct_solve_calls` as the protected driver does. A solve without
    /// factors (no factorization yet, or the last one failed) is a
    /// [`CoreError::LinearSolve`].
    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if b.len() != self.n {
            return Err(CoreError::Dimension(
                "dense stage solve shape mismatch".into(),
            ));
        }
        counters.direct_solve_calls += 1;
        match self.mode {
            DenseLuMode::InPlace => {
                if !self.in_place_factored {
                    return Err(not_factored());
                }
                lu_research::solve(
                    self.w.as_slice(),
                    self.n,
                    &self.pivots,
                    &self.row_end,
                    &self.l_start,
                    b,
                );
                Ok(())
            }
            DenseLuMode::Faer => {
                let (Some(lu), Some(workspace)) =
                    (self.faer_lu.as_ref(), self.faer_solve_workspace.as_mut())
                else {
                    return Err(not_factored());
                };
                lu.solve_in_place_with_workspace(b, workspace)
            }
        }
    }
}

fn not_factored() -> CoreError {
    CoreError::LinearSolve("the dense stage client has no factors of W".into())
}
