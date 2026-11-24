//! The workspace of the router: the checkpoint between legs and the work counters.
//!
//! No driver takes storage from here. `Dopri5::solve`, `Rodas5p::solve` and the switching
//! adapter allocate their own workspaces, a dense matrix included when the route they run
//! needs one; the router allocates none. The workspace holds four vectors of the state
//! dimension.

use rodas5p_core::WorkCounters;

/// Checkpoint, counters and three scratch vectors of the state dimension.
///
/// The first unified router also allocated eight stage vectors, a Krylov basis and an
/// `n * n` matrix here for every problem; no driver ever received them, and they are gone.
pub struct VigilodeWorkspace {
    /// State dimension of the last solve.
    pub dimension: usize,
    /// The value passed to [`VigilodeWorkspace::new`]. Stored only: it sizes nothing.
    pub max_krylov_dim: usize,
    /// The last accepted state: where a fallback leg starts.
    pub y_checkpoint: Vec<f64>,
    /// Scratch of [`VigilodeWorkspace::step_into`]: the Euler predictor.
    pub y_scratch: Vec<f64>,
    /// Scratch of [`VigilodeWorkspace::step_into`]: the right-hand side at the start.
    pub dy_scratch: Vec<f64>,
    /// Scratch of [`VigilodeWorkspace::step_into`]: the right-hand side at the predictor.
    pub stage_scratch: Vec<f64>,
    /// Time of the checkpoint.
    pub checkpoint_time: f64,
    /// The work of the solve so far: every leg and every pilot.
    pub work_counters: WorkCounters,
}

impl VigilodeWorkspace {
    /// A workspace for dimension `dimension`. `max_krylov_dim` is stored and sizes
    /// nothing; it is a parameter so that existing callers compile.
    pub fn new(dimension: usize, max_krylov_dim: usize) -> Self {
        let n = dimension.max(1);
        Self {
            dimension,
            max_krylov_dim: max_krylov_dim.max(1),
            y_checkpoint: vec![0.0; n],
            y_scratch: vec![0.0; n],
            dy_scratch: vec![0.0; n],
            stage_scratch: vec![0.0; n],
            checkpoint_time: 0.0,
            work_counters: WorkCounters::default(),
        }
    }

    /// Prepare for a new solve from `y0`: grow the vectors if the dimension grew, store
    /// `y0` as the checkpoint and zero the counters.
    pub fn reset(&mut self, y0: &[f64]) {
        self.dimension = y0.len();
        let n = self.dimension.max(1);
        for buffer in [
            &mut self.y_checkpoint,
            &mut self.y_scratch,
            &mut self.dy_scratch,
            &mut self.stage_scratch,
        ] {
            if buffer.len() < n {
                buffer.resize(n, 0.0);
            }
        }
        self.checkpoint_time = 0.0;
        self.y_checkpoint[..y0.len()].copy_from_slice(y0);
        self.work_counters = WorkCounters::default();
    }

    /// Store `(t, y)` as the last accepted state.
    pub fn checkpoint(&mut self, t: f64, y: &[f64]) {
        self.checkpoint_time = t;
        let len = y.len().min(self.y_checkpoint.len());
        self.y_checkpoint[..len].copy_from_slice(&y[..len]);
    }

    /// Copy the checkpoint state into `y_out` and return its time.
    pub fn rollback(&self, y_out: &mut [f64]) -> f64 {
        let len = y_out.len().min(self.y_checkpoint.len());
        y_out[..len].copy_from_slice(&self.y_checkpoint[..len]);
        self.checkpoint_time
    }

    /// Heap bytes the workspace holds.
    pub fn retained_bytes(&self) -> usize {
        (self.y_checkpoint.capacity()
            + self.y_scratch.capacity()
            + self.dy_scratch.capacity()
            + self.stage_scratch.capacity())
            * std::mem::size_of::<f64>()
    }

    /// One fixed-step Heun (explicit trapezoidal) step of size `dt` from `(t, y)` into
    /// `y_out`, on the scratch vectors of the workspace: two right-hand-side calls and no
    /// heap allocation. It has no error control, and no solver of the facade calls it:
    /// [`super::VigilODE::solve`] runs the drivers, which own their step loops and their
    /// storage. The two calls and the step are added to
    /// [`VigilodeWorkspace::work_counters`].
    #[allow(clippy::needless_range_loop)]
    pub fn step_into(
        &mut self,
        problem: &crate::problem::Problem,
        t: f64,
        y: &[f64],
        dt: f64,
        y_out: &mut [f64],
    ) -> Result<bool, crate::error::CallbackError> {
        let n = self.dimension;
        (problem.rhs_fn())(t, y, &mut self.dy_scratch[..n])?;
        for i in 0..n {
            self.y_scratch[i] = y[i] + dt * self.dy_scratch[i];
        }
        (problem.rhs_fn())(t + dt, &self.y_scratch[..n], &mut self.stage_scratch[..n])?;
        for i in 0..n {
            y_out[i] = y[i] + 0.5 * dt * (self.dy_scratch[i] + self.stage_scratch[i]);
        }
        self.work_counters.rhs_calls += 2;
        self.work_counters.accepted_steps += 1;
        Ok(true)
    }
}
