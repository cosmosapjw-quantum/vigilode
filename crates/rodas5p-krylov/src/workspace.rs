use crate::small::LeastSquaresWorkspace;
use rodas5p_core::{CoreResult, DenseMatrix};

pub(crate) fn ensure_len(buffer: &mut Vec<f64>, len: usize) {
    if buffer.len() != len {
        buffer.resize(len, 0.0);
    }
}

pub(crate) fn ensure_pool(pool: &mut Vec<Vec<f64>>, count: usize, dimension: usize) {
    while pool.len() < count {
        pool.push(vec![0.0; dimension]);
    }
    for vector in &mut pool[..count] {
        ensure_len(vector, dimension);
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CommonWorkspace {
    pub x: Vec<f64>,
    pub residual: Vec<f64>,
    pub operator_output: Vec<f64>,
    pub preconditioned: Vec<f64>,
    pub scratch_a: Vec<f64>,
    pub scratch_b: Vec<f64>,
    pub coefficients: Vec<f64>,
}

impl CommonWorkspace {
    pub fn prepare(&mut self, dimension: usize) {
        ensure_len(&mut self.x, dimension);
        self.x.fill(0.0);
        ensure_len(&mut self.residual, dimension);
        ensure_len(&mut self.operator_output, dimension);
        ensure_len(&mut self.preconditioned, dimension);
        ensure_len(&mut self.scratch_a, dimension);
        ensure_len(&mut self.scratch_b, dimension);
    }

    pub fn capacity_f64(&self) -> usize {
        self.x.capacity()
            + self.residual.capacity()
            + self.operator_output.capacity()
            + self.preconditioned.capacity()
            + self.scratch_a.capacity()
            + self.scratch_b.capacity()
            + self.coefficients.capacity()
    }

    pub(crate) fn retained_bytes(&self) -> usize {
        self.capacity_f64() * std::mem::size_of::<f64>()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ArnoldiWorkspace {
    pub basis: Vec<Vec<f64>>,
    pub directions: Vec<Vec<f64>>,
    pub hessenberg: DenseMatrix,
    pub hessenberg_prefix: DenseMatrix,
    pub h_column: Vec<f64>,
    pub rhs_small: Vec<f64>,
    pub correction: Vec<f64>,
    pub raw_operator_output: Vec<f64>,
}

impl ArnoldiWorkspace {
    pub fn prepare(&mut self, dimension: usize, total_columns: usize) -> CoreResult<()> {
        ensure_pool(&mut self.basis, total_columns + 1, dimension);
        ensure_pool(&mut self.directions, total_columns, dimension);
        self.hessenberg
            .resize_zeros(total_columns + 1, total_columns)?;
        self.hessenberg_prefix.resize_zeros(0, 0)?;
        ensure_len(&mut self.h_column, total_columns + 1);
        ensure_len(&mut self.rhs_small, total_columns + 1);
        ensure_len(&mut self.correction, dimension);
        ensure_len(&mut self.raw_operator_output, dimension);
        Ok(())
    }

    pub fn capacity_f64(&self) -> usize {
        self.basis.iter().map(Vec::capacity).sum::<usize>()
            + self.directions.iter().map(Vec::capacity).sum::<usize>()
            + self.hessenberg.capacity()
            + self.hessenberg_prefix.capacity()
            + self.h_column.capacity()
            + self.rhs_small.capacity()
            + self.correction.capacity()
            + self.raw_operator_output.capacity()
    }

    pub(crate) fn retained_bytes(&self) -> usize {
        self.capacity_f64() * std::mem::size_of::<f64>()
            + (self.basis.capacity() + self.directions.capacity()) * std::mem::size_of::<Vec<f64>>()
    }
}

#[derive(Clone, Debug, Default)]
pub struct GmresWorkspace {
    pub(crate) common: CommonWorkspace,
    pub(crate) arnoldi: ArnoldiWorkspace,
    /// Opt-in allocation-free small least squares of `solve_gmres_into`
    /// (research node `research/spd04_ls_workspace_20261007`); `None` keeps
    /// the allocating [`crate::small::least_squares`]. The other GMRES entry
    /// points never use it.
    pub(crate) least_squares: Option<Box<LeastSquaresWorkspace>>,
    /// The least-squares solution when `least_squares` is used.
    pub(crate) least_squares_solution: Vec<f64>,
}

impl GmresWorkspace {
    pub fn capacity_f64(&self) -> usize {
        self.common.capacity_f64()
            + self.arnoldi.capacity_f64()
            + self.least_squares_solution.capacity()
    }

    /// Retained heap bytes of the least-squares workspace and solution buffer.
    pub fn least_squares_bytes(&self) -> usize {
        self.least_squares.as_ref().map_or(0, |ls| {
            let retained = ls.retained_bytes();
            if retained == 0 {
                0
            } else {
                std::mem::size_of::<LeastSquaresWorkspace>() + retained
            }
        }) + self.least_squares_solution.capacity() * std::mem::size_of::<f64>()
    }

    /// Retained heap bytes of this workspace.
    pub fn retained_bytes(&self) -> usize {
        self.common.retained_bytes() + self.arnoldi.retained_bytes() + self.least_squares_bytes()
    }

    /// A workspace whose `solve_gmres_into` solves the small least-squares
    /// problem in a reused [`LeastSquaresWorkspace`] (research node
    /// `research/spd04_ls_workspace_20261007`); bitwise the same results.
    pub fn with_ls_workspace() -> Self {
        let mut workspace = Self::default();
        workspace.set_ls_workspace(true);
        workspace
    }

    /// Turn the reused least-squares workspace of `solve_gmres_into` on or
    /// off (off by default). Turning it off drops its buffers.
    pub fn set_ls_workspace(&mut self, on: bool) {
        match (on, self.least_squares.is_some()) {
            (true, false) => self.least_squares = Some(Box::default()),
            (false, true) => {
                self.least_squares = None;
                self.least_squares_solution = Vec::new();
            }
            _ => {}
        }
    }

    /// The reused least-squares workspace, if on (its growth flag and
    /// counters).
    pub fn ls_workspace(&self) -> Option<&LeastSquaresWorkspace> {
        self.least_squares.as_deref()
    }

    /// Mutable reference to the reused least-squares workspace, if on.
    pub fn ls_workspace_mut(&mut self) -> Option<&mut LeastSquaresWorkspace> {
        self.least_squares.as_deref_mut()
    }

    /// Reset growth counters on the reused least-squares workspace, if on.
    pub fn reset_growth_events(&mut self) {
        if let Some(ls) = self.least_squares.as_mut() {
            ls.reset_growth_events();
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LgmresWorkspace {
    pub(crate) common: CommonWorkspace,
    pub(crate) arnoldi: ArnoldiWorkspace,
}

impl LgmresWorkspace {
    pub fn capacity_f64(&self) -> usize {
        self.common.capacity_f64() + self.arnoldi.capacity_f64()
    }

    /// Retained heap bytes of this workspace.
    pub fn retained_bytes(&self) -> usize {
        self.common.retained_bytes() + self.arnoldi.retained_bytes()
    }
}

#[derive(Clone, Debug, Default)]
pub struct GcrodrWorkspace {
    pub(crate) common: CommonWorkspace,
}

impl GcrodrWorkspace {
    pub fn capacity_f64(&self) -> usize {
        self.common.capacity_f64()
    }

    /// Retained heap bytes of this workspace.
    pub fn retained_bytes(&self) -> usize {
        self.common.retained_bytes()
    }
}
