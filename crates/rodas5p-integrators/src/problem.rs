use std::sync::Arc;

use crate::parallel::ParallelExecution;
use crate::rodas5p_fast::{BandedJacobian, BandedJacobianFn};

use rodas5p_core::{
    ClosureOperator, CoreError, CoreResult, DenseMatrix, DenseOperator, LinearOperator,
    OperatorApplicationWork, WorkCounters,
};

/// One user JVP callback evaluates one Jacobian-vector product.
fn jvp_callback_work() -> OperatorApplicationWork {
    OperatorApplicationWork {
        jvp_calls: 1,
        jvp_vectors: 1,
        ..OperatorApplicationWork::default()
    }
}

pub type RhsFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
pub type BatchRhsFn = Arc<dyn Fn(&[f64], &[Vec<f64>]) -> CoreResult<Vec<Vec<f64>>> + Send + Sync>;
pub type JacobianFn = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
/// Writes the Jacobian into a caller-owned `n x n` matrix; see
/// [`OdeProblem::with_jacobian_into`] for the contract.
pub type JacobianIntoFn =
    Arc<dyn Fn(f64, &[f64], &mut DenseMatrix) -> CoreResult<()> + Send + Sync>;
pub type JvpFn = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
pub type PartialTFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
pub type ExactFn = Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync>;
/// A client-owned model generation counter; see [`OdeProblem::with_model_epoch`].
pub type ModelEpochFn = Arc<dyn Fn() -> u64 + Send + Sync>;

/// The linear-algebra structure a caller declares for its problem (research
/// node SP03, `research/sp03_declared_structure_routing_20261010`). Only the
/// opt-in router [`crate::integrate_rodas5p_routed_observed`] reads it; no
/// other entry point changes with it. Attach it with
/// [`OdeProblem::with_declared_structure`], which validates it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum ProblemStructure {
    /// Nothing is declared (the default of every problem).
    #[default]
    Unstructured,
    /// A small problem with an explicit Jacobian, for the dense direct driver.
    Dense,
    /// The Jacobian is zero outside `lower` diagonals below and `upper`
    /// diagonals above the main diagonal, and a band Jacobian callback in the
    /// layout of [`BandedJacobianFn`] writes it.
    Banded { lower: usize, upper: usize },
}

/// A structure declaration: the dimension it was written for, the
/// structure, and the band Jacobian callback (required for
/// [`ProblemStructure::Banded`], refused otherwise).
#[derive(Clone)]
pub struct StructureDeclaration {
    pub dimension: usize,
    pub structure: ProblemStructure,
    pub band_jacobian: Option<BandedJacobianFn>,
}

impl StructureDeclaration {
    /// No declaration (what every problem carries by default).
    pub fn unstructured(dimension: usize) -> Self {
        Self {
            dimension,
            structure: ProblemStructure::Unstructured,
            band_jacobian: None,
        }
    }

    /// A dense declaration; the problem must have an explicit Jacobian.
    pub fn dense(dimension: usize) -> Self {
        Self {
            dimension,
            structure: ProblemStructure::Dense,
            band_jacobian: None,
        }
    }

    /// A band declaration with its band Jacobian callback.
    pub fn banded(dimension: usize, lower: usize, upper: usize, fill: BandedJacobianFn) -> Self {
        Self {
            dimension,
            structure: ProblemStructure::Banded { lower, upper },
            band_jacobian: Some(fill),
        }
    }
}

/// Why a structure declaration was refused. Every variant is raised before
/// any integration step: the declaration-time variants by
/// [`OdeProblem::with_declared_structure`], the band-verification variants
/// by the router at `(t0, y0)`.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum StructureError {
    #[error("structure declared for dimension {declared}, problem has dimension {problem}")]
    DimensionMismatch { declared: usize, problem: usize },
    #[error(
        "band (lower {lower}, upper {upper}) lies outside the {dimension} x {dimension} matrix"
    )]
    BandOutsideMatrix {
        lower: usize,
        upper: usize,
        dimension: usize,
    },
    #[error("a banded declaration needs a band Jacobian callback")]
    MissingBandCallback,
    #[error("a band Jacobian callback was given for a non-banded declaration")]
    UnexpectedBandCallback,
    #[error("a dense declaration needs an explicit Jacobian")]
    DenseWithoutExplicitJacobian,
    #[error(
        "band verification rejected the declaration: vector {vector}, mismatch {mismatch:e} > {tolerance:e} x reference norm {reference_norm:e}"
    )]
    BandVerificationMismatch {
        vector: usize,
        mismatch: f64,
        reference_norm: f64,
        tolerance: f64,
    },
    #[error("band verification could not evaluate the declaration: {0}")]
    BandVerificationFailed(String),
}

#[derive(Clone)]
pub struct OdeProblem {
    pub name: String,
    pub dimension: usize,
    rhs: RhsFn,
    rhs_batch: Option<BatchRhsFn>,
    jacobian: Option<JacobianFn>,
    jacobian_into: Option<JacobianIntoFn>,
    jvp: Option<JvpFn>,
    partial_t: Option<PartialTFn>,
    pub autonomous: bool,
    pub mass_matrix: Option<DenseMatrix>,
    exact_solution: Option<ExactFn>,
    model_epoch: Option<ModelEpochFn>,
    /// Declared structure (SP03); `Unstructured` unless validated by
    /// [`Self::with_declared_structure`].
    structure: ProblemStructure,
    band_jacobian: Option<BandedJacobianFn>,
}

/// Retained callback identities for a frozen matrix-free state. This is an
/// allocation identity, not a proof that interior mutable callback data stayed
/// unchanged. Callers changing such data must request a fresh linearization.
#[derive(Clone)]
pub(crate) struct MatrixFreeCallbackIdentity {
    rhs: RhsFn,
    jvp: Option<JvpFn>,
    partial_t: Option<PartialTFn>,
    model_epoch: Option<ModelEpochFn>,
    autonomous: bool,
}

impl MatrixFreeCallbackIdentity {
    pub(crate) fn matches(&self, problem: &OdeProblem) -> bool {
        fn same<T: ?Sized>(a: &Option<Arc<T>>, b: &Option<Arc<T>>) -> bool {
            match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
        }
        Arc::ptr_eq(&self.rhs, &problem.rhs)
            && same(&self.jvp, &problem.jvp)
            && same(&self.partial_t, &problem.partial_t)
            && same(&self.model_epoch, &problem.model_epoch)
            && self.autonomous == problem.autonomous
    }
}

impl OdeProblem {
    pub(crate) fn matrix_free_callback_identity(&self) -> MatrixFreeCallbackIdentity {
        MatrixFreeCallbackIdentity {
            rhs: self.rhs.clone(),
            jvp: self.jvp.clone(),
            partial_t: self.partial_t.clone(),
            model_epoch: self.model_epoch.clone(),
            autonomous: self.autonomous,
        }
    }

    /// Attach a client-owned model epoch (research node
    /// `research/rnext07_model_epoch_20261003`). The client must change the
    /// returned value whenever data read by `rhs`, the Jacobian, `jvp` or
    /// `partial_t` change, for example parameters behind an `Arc<AtomicU64>`
    /// or a `Mutex`. Drivers that keep frozen linearization data across calls
    /// (the U-form matrix-free workspace) then rebuild it when the epoch
    /// differs from the one it was built under. The epoch is the client's
    /// promise: the library cannot check it, and neither it nor any callback
    /// pointer is a proof that the model is unchanged. Without an epoch, such
    /// changes need `fresh = true` (the manual contract).
    pub fn with_model_epoch(mut self, epoch: ModelEpochFn) -> Self {
        self.model_epoch = Some(epoch);
        self
    }

    /// The current model epoch, if the client supplied one.
    pub fn model_epoch(&self) -> Option<u64> {
        self.model_epoch.as_ref().map(|epoch| epoch())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        dimension: usize,
        rhs: RhsFn,
        rhs_batch: Option<BatchRhsFn>,
        jacobian: Option<JacobianFn>,
        jvp: Option<JvpFn>,
        partial_t: Option<PartialTFn>,
        autonomous: bool,
        mass_matrix: Option<DenseMatrix>,
        exact_solution: Option<ExactFn>,
    ) -> CoreResult<Self> {
        if dimension == 0 {
            return Err(CoreError::InvalidInput(
                "ODE dimension must be positive".into(),
            ));
        }
        if jacobian.is_none() && jvp.is_none() {
            return Err(CoreError::InvalidInput(
                "provide an explicit Jacobian or JVP".into(),
            ));
        }
        if mass_matrix
            .as_ref()
            .is_some_and(|m| m.nrows() != dimension || m.ncols() != dimension)
        {
            return Err(CoreError::Dimension("mass matrix shape mismatch".into()));
        }
        Ok(Self {
            name: name.into(),
            dimension,
            rhs,
            rhs_batch,
            jacobian,
            jacobian_into: None,
            jvp,
            partial_t,
            autonomous,
            mass_matrix,
            exact_solution,
            model_epoch: None,
            structure: ProblemStructure::Unstructured,
            band_jacobian: None,
        })
    }

    /// Attach a validated structure declaration (research node SP03). The
    /// declaration must name this problem's dimension; a band must lie
    /// inside the matrix (`lower, upper < n`) and come with its band
    /// Jacobian callback; a dense declaration needs an explicit Jacobian; a
    /// callback without a band is refused. Whether the band callback agrees
    /// with the problem's Jacobian is checked by the router at `(t0, y0)`.
    /// Only [`crate::integrate_rodas5p_routed_observed`] reads the
    /// declaration.
    pub fn with_declared_structure(
        mut self,
        declaration: StructureDeclaration,
    ) -> Result<Self, StructureError> {
        let n = self.dimension;
        if declaration.dimension != n {
            return Err(StructureError::DimensionMismatch {
                declared: declaration.dimension,
                problem: n,
            });
        }
        match declaration.structure {
            ProblemStructure::Banded { lower, upper } => {
                if lower >= n || upper >= n {
                    return Err(StructureError::BandOutsideMatrix {
                        lower,
                        upper,
                        dimension: n,
                    });
                }
                if declaration.band_jacobian.is_none() {
                    return Err(StructureError::MissingBandCallback);
                }
            }
            ProblemStructure::Dense => {
                if declaration.band_jacobian.is_some() {
                    return Err(StructureError::UnexpectedBandCallback);
                }
                if !self.has_explicit_jacobian_callback() {
                    return Err(StructureError::DenseWithoutExplicitJacobian);
                }
            }
            ProblemStructure::Unstructured => {
                if declaration.band_jacobian.is_some() {
                    return Err(StructureError::UnexpectedBandCallback);
                }
            }
        }
        self.structure = declaration.structure;
        self.band_jacobian = declaration.band_jacobian;
        Ok(self)
    }

    /// The declared structure (`Unstructured` unless one was attached).
    pub fn declared_structure(&self) -> ProblemStructure {
        self.structure
    }

    /// The declared band and its callback, if a band was declared.
    pub fn declared_band(&self) -> Option<BandedJacobian> {
        match (self.structure, &self.band_jacobian) {
            (ProblemStructure::Banded { lower, upper }, Some(fill)) => Some(BandedJacobian {
                lower,
                upper,
                fill: fill.clone(),
            }),
            _ => None,
        }
    }

    /// An explicit Jacobian callback (allocating or in place) is present.
    pub(crate) fn has_explicit_jacobian_callback(&self) -> bool {
        self.jacobian.is_some() || self.jacobian_into.is_some()
    }

    /// The JVP callback, if any (band verification of SP03).
    pub(crate) fn jvp_callback(&self) -> Option<&JvpFn> {
        self.jvp.as_ref()
    }

    fn eval_rhs_uncounted(&self, t: f64, y: &[f64]) -> CoreResult<Vec<f64>> {
        if y.len() != self.dimension {
            return Err(CoreError::Dimension("RHS state shape mismatch".into()));
        }
        let mut out = vec![0.0; self.dimension];
        (self.rhs)(t, y, &mut out)?;
        if out.iter().all(|v| v.is_finite()) {
            Ok(out)
        } else {
            Err(CoreError::NonFinite("RHS produced NaN/Inf".into()))
        }
    }

    /// [`Self::eval_rhs`] into a caller-owned buffer, with the same checks
    /// and counting (the RODAS5P fast driver's allocation-free stages).
    pub fn eval_rhs_into(
        &self,
        t: f64,
        y: &[f64],
        out: &mut [f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if y.len() != self.dimension || out.len() != self.dimension {
            return Err(CoreError::Dimension("RHS state shape mismatch".into()));
        }
        (self.rhs)(t, y, out)?;
        if !out.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite("RHS produced NaN/Inf".into()));
        }
        counters.rhs_calls += 1;
        counters.rhs_evaluations += 1;
        Ok(())
    }

    pub fn eval_rhs(&self, t: f64, y: &[f64], counters: &mut WorkCounters) -> CoreResult<Vec<f64>> {
        let out = self.eval_rhs_uncounted(t, y)?;
        counters.rhs_calls += 1;
        counters.rhs_evaluations += 1;
        Ok(out)
    }

    /// Evaluate independent method-stage RHS rows through a bounded execution context.
    ///
    /// This path calls the scalar physics kernel once per stage.  With a local Rayon pool those
    /// calls run concurrently, so the implementation measures real within-step stage parallelism
    /// rather than case-level throughput or a serial function with a batched signature.
    pub fn eval_rhs_stage_rows(
        &self,
        times: &[f64],
        states: &[Vec<f64>],
        execution: &ParallelExecution,
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<Vec<f64>>> {
        if times.len() != states.len() || states.iter().any(|s| s.len() != self.dimension) {
            return Err(CoreError::Dimension(
                "stage RHS batch shape mismatch".into(),
            ));
        }
        let indices: Vec<usize> = (0..states.len()).collect();
        // Count the entire scheduled batch before execution so an early error cannot erase work
        // requested from the numerical method. This is the same vector-work convention used by
        // the provided batch callback.
        counters.rhs_batch_calls += 1;
        counters.rhs_evaluations += states.len() as u64;
        execution.map_ordered(&indices, |&index| {
            self.eval_rhs_uncounted(times[index], &states[index])
        })
    }

    pub fn eval_rhs_batch(
        &self,
        times: &[f64],
        states: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<Vec<f64>>> {
        if times.len() != states.len() || states.iter().any(|s| s.len() != self.dimension) {
            return Err(CoreError::Dimension("batched RHS shape mismatch".into()));
        }
        if let Some(batch) = &self.rhs_batch {
            let out = batch(times, states)?;
            if out.len() != states.len() || out.iter().any(|r| r.len() != self.dimension) {
                return Err(CoreError::Dimension(
                    "batched RHS output shape mismatch".into(),
                ));
            }
            if !out.iter().flatten().all(|v| v.is_finite()) {
                return Err(CoreError::NonFinite("batched RHS produced NaN/Inf".into()));
            }
            counters.rhs_batch_calls += 1;
            counters.rhs_evaluations += times.len() as u64;
            Ok(out)
        } else {
            times
                .iter()
                .zip(states)
                .map(|(&t, y)| self.eval_rhs(t, y, counters))
                .collect()
        }
    }

    pub fn eval_partial_t(
        &self,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<f64>> {
        counters.ft_calls += 1;
        if self.autonomous {
            return Ok(vec![0.0; self.dimension]);
        }
        if let Some(f) = &self.partial_t {
            let mut out = vec![0.0; self.dimension];
            f(t, y, &mut out)?;
            if out.iter().all(|v| v.is_finite()) {
                return Ok(out);
            }
            return Err(CoreError::NonFinite("partial_t produced NaN/Inf".into()));
        }
        let eps = f64::EPSILON.sqrt() * t.abs().max(1.0);
        let fp = self.eval_rhs(t + eps, y, counters)?;
        let fm = self.eval_rhs(t - eps, y, counters)?;
        Ok(fp
            .iter()
            .zip(fm)
            .map(|(a, b)| (a - b) / (2.0 * eps))
            .collect())
    }

    pub fn mass_or_identity(&self) -> DenseMatrix {
        self.mass_matrix
            .clone()
            .unwrap_or_else(|| DenseMatrix::identity(self.dimension))
    }

    /// Add an in-place Jacobian for drivers that keep one matrix across
    /// steps (the RODAS5P fast driver). The callback receives the matrix it
    /// last wrote for this problem, all zero on the first call, and must
    /// write every entry that is nonzero in any Jacobian of the problem;
    /// entries it never writes must be zero in every Jacobian (a fixed
    /// sparsity pattern). It must produce the same values as the explicit
    /// Jacobian, which every other path keeps using.
    pub fn with_jacobian_into(mut self, jacobian_into: JacobianIntoFn) -> Self {
        self.jacobian_into = Some(jacobian_into);
        self
    }

    /// The Jacobian into `out`, which must be `n x n` and hold the previous
    /// Jacobian of this problem (or zeros): through the in-place callback
    /// when there is one, otherwise copied from [`Self::dense_jacobian`].
    pub fn dense_jacobian_into(
        &self,
        t: f64,
        y: &[f64],
        out: &mut DenseMatrix,
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if y.len() != self.dimension
            || out.nrows() != self.dimension
            || out.ncols() != self.dimension
        {
            return Err(CoreError::Dimension("Jacobian state shape mismatch".into()));
        }
        match &self.jacobian_into {
            Some(fill) => {
                counters.jacobian_builds += 1;
                fill(t, y, out)
            }
            None => {
                let matrix = self.dense_jacobian(t, y, counters)?;
                out.as_mut_slice().copy_from_slice(matrix.as_slice());
                Ok(())
            }
        }
    }

    pub fn dense_jacobian(
        &self,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<DenseMatrix> {
        if y.len() != self.dimension {
            return Err(CoreError::Dimension("Jacobian state shape mismatch".into()));
        }
        if let Some(jacobian) = &self.jacobian {
            counters.jacobian_builds += 1;
            let matrix = jacobian(t, y)?;
            if matrix.nrows() != self.dimension || matrix.ncols() != self.dimension {
                return Err(CoreError::Dimension(
                    "Jacobian output shape mismatch".into(),
                ));
            }
            return Ok(matrix);
        }
        let jvp = self.jvp.as_ref().expect("validated JVP");
        let mut matrix = DenseMatrix::zeros(self.dimension, self.dimension);
        let mut basis = vec![0.0; self.dimension];
        let mut column = vec![0.0; self.dimension];
        for j in 0..self.dimension {
            basis.fill(0.0);
            basis[j] = 1.0;
            column.fill(0.0);
            jvp(t, y, &basis, &mut column)?;
            counters.jvp_calls += 1;
            counters.jvp_vectors += 1;
            if !column.iter().all(|value| value.is_finite()) {
                return Err(CoreError::NonFinite("JVP produced NaN/Inf".into()));
            }
            for i in 0..self.dimension {
                matrix[(i, j)] = column[i];
            }
        }
        counters.jacobian_builds += 1;
        Ok(matrix)
    }

    pub fn linearize(
        &self,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<Arc<dyn LinearOperator>> {
        // Each linearization declares what one application costs, so every
        // lane counts user JVP callbacks in `jvp_calls` and explicit Jacobian
        // products in `jacobian_matvecs` (audit F-048, F-022).
        if let Some(j) = &self.jacobian {
            counters.jacobian_builds += 1;
            return Ok(Arc::new(DenseOperator::with_application_work(
                j(t, y)?,
                OperatorApplicationWork {
                    jacobian_matvecs: 1,
                    ..OperatorApplicationWork::default()
                },
            )?));
        }
        let jvp = self.jvp.clone().expect("validated JVP");
        let state = y.to_vec();
        let n = self.dimension;
        Ok(Arc::new(ClosureOperator::with_application_work(
            n,
            move |v, out| jvp(t, &state, v, out),
            jvp_callback_work(),
        )))
    }

    /// Build a strictly matrix-free linearization operator.
    ///
    /// Unlike [`OdeProblem::linearize`], this entry point never falls back to an explicit
    /// Jacobian callback.  It is the load-bearing contract for the generic vectorized/JF fast
    /// path: callers either supplied a genuine JVP implementation or the request fails closed
    /// before a speculative timestep starts.
    pub fn linearize_matrix_free(&self, t: f64, y: &[f64]) -> CoreResult<Arc<dyn LinearOperator>> {
        if y.len() != self.dimension {
            return Err(CoreError::Dimension(
                "matrix-free linearization state shape mismatch".into(),
            ));
        }
        let jvp = self.jvp.clone().ok_or_else(|| {
            CoreError::InvalidInput(
                "strict matrix-free integration requires a user-supplied JVP".into(),
            )
        })?;
        let state = y.to_vec();
        let n = self.dimension;
        Ok(Arc::new(ClosureOperator::with_application_work(
            n,
            move |v, out| jvp(t, &state, v, out),
            jvp_callback_work(),
        )))
    }

    pub fn supports_matrix_free_jvp(&self) -> bool {
        self.jvp.is_some()
    }

    /// Return a clone whose linearization is exposed only through the configured JVP.
    ///
    /// This is the strict matrix-free research lane: even when an explicit Jacobian callback is
    /// available for offline certification, the returned problem cannot materialize it through
    /// [`OdeProblem::linearize`].
    pub fn jvp_only_clone(&self) -> CoreResult<Self> {
        if self.jvp.is_none() {
            return Err(CoreError::InvalidInput(
                "strict matrix-free clone requires a JVP callback".into(),
            ));
        }
        let mut cloned = self.clone();
        cloned.jacobian = None;
        cloned.jacobian_into = None;
        // A declared structure is an explicit-matrix provider: the strict
        // matrix-free clone carries none (SP03).
        cloned.structure = ProblemStructure::Unstructured;
        cloned.band_jacobian = None;
        Ok(cloned)
    }

    pub fn has_explicit_jacobian(&self) -> bool {
        self.jacobian.is_some()
    }

    pub fn has_jvp(&self) -> bool {
        self.jvp.is_some()
    }

    /// Convert a nonautonomous identity-mass problem into the autonomous augmented system
    /// `(y, tau)' = (F(tau, y), 1)` without forming an explicit Jacobian.
    ///
    /// The augmented JVP is `(J_y v_y + F_t v_tau, 0)`. A declared `partial_t` callback is
    /// mandatory: hidden finite differences would violate the strict work and JVP-quality contract.
    pub fn time_augmented_clone(&self) -> CoreResult<Self> {
        if self.mass_matrix.is_some() {
            return Err(CoreError::InvalidInput(
                "time augmentation currently supports identity mass only".into(),
            ));
        }
        let jvp = self.jvp.clone().ok_or_else(|| {
            CoreError::InvalidInput("time augmentation requires a JVP callback".into())
        })?;
        let partial_t = self.partial_t.clone().ok_or_else(|| {
            CoreError::InvalidInput(
                "time augmentation requires an explicit partial_t callback".into(),
            )
        })?;
        let rhs = self.rhs.clone();
        let n = self.dimension;
        let augmented_rhs: RhsFn = Arc::new(move |_, state, out| {
            if state.len() != n + 1 || out.len() != n + 1 {
                return Err(CoreError::Dimension(
                    "time-augmented RHS shape mismatch".into(),
                ));
            }
            rhs(state[n], &state[..n], &mut out[..n])?;
            out[n] = 1.0;
            Ok(())
        });
        let augmented_jvp: JvpFn = Arc::new(move |_, state, direction, out| {
            if state.len() != n + 1 || direction.len() != n + 1 || out.len() != n + 1 {
                return Err(CoreError::Dimension(
                    "time-augmented JVP shape mismatch".into(),
                ));
            }
            jvp(state[n], &state[..n], &direction[..n], &mut out[..n])?;
            if direction[n] != 0.0 {
                let mut ft = vec![0.0; n];
                partial_t(state[n], &state[..n], &mut ft)?;
                for (value, source) in out[..n].iter_mut().zip(ft) {
                    *value += direction[n] * source;
                }
            }
            out[n] = 0.0;
            Ok(())
        });
        let exact = self.exact_solution.clone().map(|solution| {
            Arc::new(move |time| {
                let mut state = solution(time);
                state.push(time);
                state
            }) as ExactFn
        });
        Self::new(
            format!("{}-time-augmented", self.name),
            n + 1,
            augmented_rhs,
            None,
            None,
            Some(augmented_jvp),
            None,
            true,
            None,
            exact,
        )
    }

    pub fn exact(&self, t: f64) -> Option<Vec<f64>> {
        self.exact_solution.as_ref().map(|f| f(t))
    }
}
