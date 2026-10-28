use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{CoreError, CoreResult, DenseMatrix, WorkCounters, direct_solve};

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactDenseMatrixIdentity {
    rows: usize,
    columns: usize,
    value_bits: Vec<u64>,
}

impl ExactDenseMatrixIdentity {
    fn from_matrix(matrix: &DenseMatrix) -> Self {
        Self {
            rows: matrix.nrows(),
            columns: matrix.ncols(),
            value_bits: matrix
                .as_slice()
                .iter()
                .map(|value| value.to_bits())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactOperatorIdentity {
    Instance(u64),
    Dense(ExactDenseMatrixIdentity),
    Shifted {
        jacobian: Box<ExactOperatorIdentity>,
        mass: Option<ExactDenseMatrixIdentity>,
        h_gamma_bits: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactPreconditionerIdentity {
    Identity { dimension: usize },
    Jacobi { inverse_diagonal_bits: Vec<u64> },
    Direct { matrix: ExactDenseMatrixIdentity },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KrylovSystemIdentity {
    pub operator: ExactOperatorIdentity,
    pub preconditioner: ExactPreconditionerIdentity,
}

pub fn exact_krylov_system_identity(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
) -> Option<KrylovSystemIdentity> {
    Some(KrylovSystemIdentity {
        operator: op.exact_identity()?,
        preconditioner: pc.exact_identity()?,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OperatorApplicationWork {
    pub jvp_calls: u64,
    pub jvp_vectors: u64,
    pub mass_matvecs: u64,
    pub block_matvecs: u64,
    /// Products with an explicit Jacobian matrix (not JVP callbacks).
    pub jacobian_matvecs: u64,
    /// State vectors one application acts on: 0 or 1 for a single-state
    /// operator, s for a block operator over s stages (audit F-051).
    pub state_vectors: u64,
}

impl OperatorApplicationWork {
    /// Add work actually performed, including a partial failed application.
    pub fn accumulate(&mut self, other: Self) {
        self.jvp_calls = self.jvp_calls.saturating_add(other.jvp_calls);
        self.jvp_vectors = self.jvp_vectors.saturating_add(other.jvp_vectors);
        self.mass_matvecs = self.mass_matvecs.saturating_add(other.mass_matvecs);
        self.block_matvecs = self.block_matvecs.saturating_add(other.block_matvecs);
        self.jacobian_matvecs = self.jacobian_matvecs.saturating_add(other.jacobian_matvecs);
        self.state_vectors = self.state_vectors.saturating_add(other.state_vectors);
    }

    fn repeated(self, count: usize) -> Self {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        Self {
            jvp_calls: self.jvp_calls.saturating_mul(count),
            jvp_vectors: self.jvp_vectors.saturating_mul(count),
            mass_matvecs: self.mass_matvecs.saturating_mul(count),
            block_matvecs: self.block_matvecs.saturating_mul(count),
            jacobian_matvecs: self.jacobian_matvecs.saturating_mul(count),
            state_vectors: self.state_vectors,
        }
    }

    /// State-vector units of one application; an undeclared operator acts on
    /// one state vector.
    pub fn state_vector_units(self) -> u64 {
        self.state_vectors.max(1)
    }

    fn charge(self, counters: &mut WorkCounters) {
        counters.jvp_calls = counters.jvp_calls.saturating_add(self.jvp_calls);
        counters.jvp_vectors = counters.jvp_vectors.saturating_add(self.jvp_vectors);
        counters.mass_matvecs = counters.mass_matvecs.saturating_add(self.mass_matvecs);
        counters.block_matvecs = counters.block_matvecs.saturating_add(self.block_matvecs);
        counters.jacobian_matvecs = counters
            .jacobian_matvecs
            .saturating_add(self.jacobian_matvecs);
    }
}

pub trait LinearOperator: Send + Sync {
    fn dimension(&self) -> usize;
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()>;

    /// Add actual partial work before returning an error when the provider
    /// can observe it. The default knows only successful application work;
    /// failed custom applications have unknown partial work, not a zero-work
    /// guarantee. Implementations add to, rather than replace, `work`.
    fn apply_with_work(
        &self,
        x: &[f64],
        y: &mut [f64],
        work: &mut OperatorApplicationWork,
    ) -> CoreResult<()> {
        self.apply(x, y)?;
        work.accumulate(self.application_work());
        Ok(())
    }

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
    /// Physical work performed by one successful application of this operator.
    ///
    /// Generic algebraic operators report no problem-specific work. Composite
    /// operators such as `M - h gamma J` override this hook so every caller,
    /// including recycle refresh and diagnostics, receives identical accounting.
    fn application_work(&self) -> OperatorApplicationWork {
        OperatorApplicationWork::default()
    }
    fn row_batch_work(&self, vector_count: usize) -> OperatorApplicationWork {
        self.application_work().repeated(vector_count)
    }
    /// Exact identity of the represented linear map, when available.
    ///
    /// The default instance identity permits reuse only while the same operator
    /// object is retained. Composite operators may provide a structural identity
    /// built from exact child identity and exact parameter bits.
    fn exact_identity(&self) -> Option<ExactOperatorIdentity> {
        Some(ExactOperatorIdentity::Instance(self.token()))
    }
    fn token(&self) -> u64;
}

#[derive(Clone)]
pub struct DenseOperator {
    matrix: DenseMatrix,
    application_work: OperatorApplicationWork,
    token: u64,
}

impl DenseOperator {
    pub fn new(matrix: DenseMatrix) -> CoreResult<Self> {
        Self::with_application_work(matrix, OperatorApplicationWork::default())
    }

    /// A dense operator that declares the physical work of one application,
    /// for example one explicit Jacobian product.
    pub fn with_application_work(
        matrix: DenseMatrix,
        application_work: OperatorApplicationWork,
    ) -> CoreResult<Self> {
        if matrix.nrows() != matrix.ncols() {
            return Err(CoreError::Dimension(
                "linear operator must be square".into(),
            ));
        }
        Ok(Self {
            matrix,
            application_work,
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
    fn application_work(&self) -> OperatorApplicationWork {
        self.application_work
    }
    fn exact_identity(&self) -> Option<ExactOperatorIdentity> {
        Some(ExactOperatorIdentity::Dense(
            ExactDenseMatrixIdentity::from_matrix(&self.matrix),
        ))
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
    application_work: OperatorApplicationWork,
    token: u64,
}

impl<F> ClosureOperator<F>
where
    F: Fn(&[f64], &mut [f64]) -> CoreResult<()> + Send + Sync,
{
    pub fn new(n: usize, f: F) -> Self {
        Self::with_application_work(n, f, OperatorApplicationWork::default())
    }

    /// A closure operator that declares the physical work of one application,
    /// for example one user JVP callback.
    pub fn with_application_work(
        n: usize,
        f: F,
        application_work: OperatorApplicationWork,
    ) -> Self {
        Self {
            n,
            f,
            application_work,
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
    fn apply_with_work(
        &self,
        x: &[f64],
        y: &mut [f64],
        work: &mut OperatorApplicationWork,
    ) -> CoreResult<()> {
        if x.len() != self.n || y.len() != self.n {
            return Err(CoreError::Dimension(
                "closure operator shape mismatch".into(),
            ));
        }
        // The native JVP provider declares one callback, entered immediately
        // below. Composite/custom closure declarations do not reveal partial
        // failure work and retain the conservative successful-only default.
        if self.application_work
            == (OperatorApplicationWork {
                jvp_calls: 1,
                jvp_vectors: 1,
                ..OperatorApplicationWork::default()
            })
        {
            work.accumulate(self.application_work);
            (self.f)(x, y)
        } else {
            (self.f)(x, y)?;
            work.accumulate(self.application_work);
            Ok(())
        }
    }
    fn application_work(&self) -> OperatorApplicationWork {
        self.application_work
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
    // Shape refusal precedes entry. Once entered, the solver-level category
    // counts an attempted application independently of provider completion.
    if x.len() != op.dimension() || y.len() != op.dimension() {
        return Err(CoreError::Dimension(
            "counted operator shape mismatch".into(),
        ));
    }
    let work = op.application_work();
    match category {
        ApplyCategory::Krylov => counters.linear_matvecs += 1,
        ApplyCategory::Refresh => counters.recycle_refresh_matvecs += 1,
        ApplyCategory::Diagnostic => counters.diagnostic_matvecs += 1,
        ApplyCategory::Block => counters.block_matvecs += 1,
    }
    if matches!(category, ApplyCategory::Krylov | ApplyCategory::Diagnostic) {
        counters.linear_matvec_vectors = counters
            .linear_matvec_vectors
            .saturating_add(work.state_vector_units());
    }
    let mut actual = OperatorApplicationWork::default();
    let outcome = op.apply_with_work(x, y, &mut actual);
    actual.charge(counters);
    outcome
}

pub fn apply_rows_counted(
    op: &dyn LinearOperator,
    inputs: &[Vec<f64>],
    outputs: &mut [Vec<f64>],
    counters: &mut WorkCounters,
    category: ApplyCategory,
) -> CoreResult<()> {
    op.apply_rows(inputs, outputs)?;
    let vectors = u64::try_from(inputs.len()).unwrap_or(u64::MAX);
    if matches!(category, ApplyCategory::Krylov | ApplyCategory::Diagnostic) {
        counters.linear_matvec_vectors = counters
            .linear_matvec_vectors
            .saturating_add(vectors.saturating_mul(op.application_work().state_vector_units()));
    }
    match category {
        ApplyCategory::Krylov => {
            counters.linear_matvecs = counters.linear_matvecs.saturating_add(vectors)
        }
        ApplyCategory::Refresh => {
            counters.recycle_refresh_matvecs =
                counters.recycle_refresh_matvecs.saturating_add(vectors);
        }
        ApplyCategory::Diagnostic => {
            counters.diagnostic_matvecs = counters.diagnostic_matvecs.saturating_add(vectors);
        }
        ApplyCategory::Block => {}
    }
    counters.block_matvecs = counters.block_matvecs.saturating_add(1);
    op.row_batch_work(inputs.len()).charge(counters);
    Ok(())
}

/// Apply an operator that is explicitly acting as a Jacobian-vector product.
///
/// This is for bare Jacobian operators. Composite operators should expose
/// [`LinearOperator::application_work`] and be called through [`apply_counted`].
pub fn apply_jvp_counted(
    jacobian: &dyn LinearOperator,
    x: &[f64],
    y: &mut [f64],
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    if x.len() != jacobian.dimension() || y.len() != jacobian.dimension() {
        return Err(CoreError::Dimension(
            "counted Jacobian shape mismatch".into(),
        ));
    }
    let mut actual = OperatorApplicationWork::default();
    let outcome = jacobian.apply_with_work(x, y, &mut actual);
    actual.charge(counters);
    outcome?;
    // An operator that declares its provenance (a JVP callback or an
    // explicit Jacobian product) is charged exactly that; an undeclared
    // operator keeps the historical role-based count of one JVP.
    let declared = jacobian.application_work();
    if declared == OperatorApplicationWork::default() {
        counters.jvp_calls = counters.jvp_calls.saturating_add(1);
        counters.jvp_vectors = counters.jvp_vectors.saturating_add(1);
    }
    Ok(())
}

/// Side on which a preconditioner acts in a Krylov solver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PreconditionerSide {
    /// Left preconditioning: solves M^-1 A x = M^-1 b.
    #[default]
    Left,
    /// Right preconditioning: solves A P^-1 u = b, then x = P^-1 u.
    Right,
}

/// A preconditioner (left or right). Implementations are not assumed nonsingular: every
/// Krylov kernel certifies convergence on the unpreconditioned true residual.
pub trait Preconditioner: Send + Sync {
    fn dimension(&self) -> usize;
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()>;
    fn is_identity(&self) -> bool {
        false
    }
    /// Exact identity of the represented preconditioner. Unknown custom
    /// implementations return `None`, which forces recycle-image refresh.
    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        None
    }
    /// State vectors one application acts on: 1, or s for a block
    /// preconditioner over s stages (audit F-051).
    fn application_vectors(&self) -> u64 {
        1
    }
    /// Whether this preconditioner is applied on the left or on the right.
    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Left
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
    fn is_identity(&self) -> bool {
        true
    }
    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        Some(ExactPreconditionerIdentity::Identity { dimension: self.n })
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
    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        Some(ExactPreconditionerIdentity::Jacobi {
            inverse_diagonal_bits: self.inv_diag.iter().map(|value| value.to_bits()).collect(),
        })
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
    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        Some(ExactPreconditionerIdentity::Direct {
            matrix: ExactDenseMatrixIdentity::from_matrix(&self.matrix),
        })
    }
}

pub struct ShiftedOperator {
    mass: Option<DenseMatrix>,
    jacobian: Arc<dyn LinearOperator>,
    h_gamma: f64,
    explicit: Option<DenseMatrix>,
    application_work: OperatorApplicationWork,
    token: u64,
}

impl ShiftedOperator {
    pub fn new(
        mass: Option<DenseMatrix>,
        jacobian: Arc<dyn LinearOperator>,
        h: f64,
        gamma: f64,
    ) -> CoreResult<Self> {
        // `M - h gamma J` performs one application of J, plus one mass
        // product when M is present. A Jacobian that declares its provenance
        // passes it on; an undeclared one keeps the historical zero.
        let mut application_work = jacobian.application_work();
        if application_work != OperatorApplicationWork::default() && mass.is_some() {
            application_work.mass_matvecs = application_work.mass_matvecs.saturating_add(1);
        }
        Self::new_with_application_work(mass, jacobian, h, gamma, application_work)
    }

    pub fn new_counted_jvp(
        mass: Option<DenseMatrix>,
        jacobian: Arc<dyn LinearOperator>,
        h: f64,
        gamma: f64,
    ) -> CoreResult<Self> {
        let application_work = OperatorApplicationWork {
            jvp_calls: 1,
            jvp_vectors: 1,
            mass_matvecs: u64::from(mass.is_some()),
            block_matvecs: 0,
            jacobian_matvecs: 0,
            state_vectors: 0,
        };
        Self::new_with_application_work(mass, jacobian, h, gamma, application_work)
    }

    fn new_with_application_work(
        mass: Option<DenseMatrix>,
        jacobian: Arc<dyn LinearOperator>,
        h: f64,
        gamma: f64,
        application_work: OperatorApplicationWork,
    ) -> CoreResult<Self> {
        let n = jacobian.dimension();
        if let Some(m) = &mass
            && (m.nrows() != n || m.ncols() != n)
        {
            return Err(CoreError::Dimension("mass matrix shape mismatch".into()));
        }
        let h_gamma = h * gamma;
        // Dimensions are validated above, but a non-finite Jacobian or shift
        // still fails `sub`; return it as a typed error, since a panic aborts
        // release builds (`panic = "abort"`) instead of failing the step.
        let explicit = jacobian
            .explicit()
            .map(|j| {
                let m = mass.clone().unwrap_or_else(|| DenseMatrix::identity(n));
                m.sub(&j.scale(h_gamma))
            })
            .transpose()?;
        Ok(Self {
            mass,
            jacobian,
            h_gamma,
            explicit,
            application_work,
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
    fn apply_with_work(
        &self,
        x: &[f64],
        y: &mut [f64],
        work: &mut OperatorApplicationWork,
    ) -> CoreResult<()> {
        let n = self.dimension();
        if x.len() != n || y.len() != n {
            return Err(CoreError::Dimension(
                "shifted operator shape mismatch".into(),
            ));
        }
        let mut partial = OperatorApplicationWork::default();
        if let Err(error) = self.jacobian.apply_with_work(x, y, &mut partial) {
            work.accumulate(partial);
            return Err(error);
        }
        // Preserve the complete successful declaration (including historical
        // role-based JVP counts), but only after the mass action is executed.
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
        work.accumulate(self.application_work);
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
    fn application_work(&self) -> OperatorApplicationWork {
        self.application_work
    }
    fn exact_identity(&self) -> Option<ExactOperatorIdentity> {
        Some(ExactOperatorIdentity::Shifted {
            jacobian: Box::new(self.jacobian.exact_identity()?),
            mass: self
                .mass
                .as_ref()
                .map(ExactDenseMatrixIdentity::from_matrix),
            h_gamma_bits: self.h_gamma.to_bits(),
        })
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
    counters.preconditioner_vectors = counters
        .preconditioner_vectors
        .saturating_add(p.application_vectors());
    Ok(())
}

#[cfg(test)]
mod terminal_application_work_tests {
    use super::*;

    fn jvp_work() -> OperatorApplicationWork {
        OperatorApplicationWork {
            jvp_calls: 1,
            jvp_vectors: 1,
            ..Default::default()
        }
    }

    #[test]
    fn failed_jvp_charges_callback_once_before_shifted_mass() {
        for fatal in [false, true] {
            let j: Arc<dyn LinearOperator> = Arc::new(ClosureOperator::with_application_work(
                1,
                move |_: &[f64], _: &mut [f64]| {
                    if fatal {
                        Err(CoreError::FatalCallback("stop".into()))
                    } else {
                        Err(CoreError::NonFinite("retry".into()))
                    }
                },
                jvp_work(),
            ));
            let op = ShiftedOperator::new(Some(DenseMatrix::identity(1)), j, 0.5, 1.0).unwrap();
            let mut counters = WorkCounters::default();
            let mut out = [0.0];
            assert!(
                apply_counted(&op, &[1.0], &mut out, &mut counters, ApplyCategory::Krylov).is_err()
            );
            assert_eq!(counters.jvp_calls, 1);
            assert_eq!(counters.jvp_vectors, 1);
            assert_eq!(counters.mass_matvecs, 0);
            assert_eq!(counters.linear_matvecs, 1);
            assert_eq!(counters.linear_matvec_vectors, 1);
            let mut refused = WorkCounters::default();
            assert!(
                apply_counted(&op, &[], &mut out, &mut refused, ApplyCategory::Krylov).is_err()
            );
            assert_eq!(refused.jvp_calls, 0);
            assert_eq!(refused.mass_matvecs, 0);
            assert_eq!(refused.linear_matvecs, 0);
            assert_eq!(refused.linear_matvec_vectors, 0);
            let mut counters = WorkCounters::default();
            assert!(apply_jvp_counted(&op, &[1.0], &mut out, &mut counters).is_err());
            assert_eq!(counters.jvp_calls, 1);
            assert_eq!(counters.mass_matvecs, 0);
            let mut partial = OperatorApplicationWork::default();
            assert!(op.apply_with_work(&[], &mut out, &mut partial).is_err());
            assert_eq!(partial, OperatorApplicationWork::default());
        }
    }

    #[test]
    fn successful_shifted_work_is_unchanged_and_hook_is_additive() {
        let j: Arc<dyn LinearOperator> = Arc::new(ClosureOperator::with_application_work(
            1,
            |v: &[f64], out: &mut [f64]| {
                out[0] = -v[0];
                Ok(())
            },
            jvp_work(),
        ));
        let op = ShiftedOperator::new(Some(DenseMatrix::identity(1)), j, 0.5, 1.0).unwrap();
        let mut reference = [0.0];
        op.apply(&[2.0], &mut reference).unwrap();
        let mut actual = [0.0];
        let mut work = jvp_work();
        op.apply_with_work(&[2.0], &mut actual, &mut work).unwrap();
        assert_eq!(reference[0].to_bits(), actual[0].to_bits());
        assert_eq!(work.jvp_calls, 2);
        assert_eq!(work.mass_matvecs, 1);
        let mut counters = WorkCounters::default();
        apply_counted(
            &op,
            &[2.0],
            &mut actual,
            &mut counters,
            ApplyCategory::Krylov,
        )
        .unwrap();
        assert_eq!(
            (
                counters.jvp_calls,
                counters.jvp_vectors,
                counters.mass_matvecs,
                counters.linear_matvecs
            ),
            (1, 1, 1, 1)
        );
        let mut refused = OperatorApplicationWork::default();
        assert!(
            op.jacobian
                .apply_with_work(&[], &mut actual, &mut refused)
                .is_err()
        );
        assert_eq!(refused, OperatorApplicationWork::default());
    }
}
