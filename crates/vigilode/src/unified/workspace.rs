//! Unified zero-allocation pre-allocated workspace for `VigilODE` (`VG-1`).

use rodas5p_core::WorkCounters;

/// Unified pre-allocated workspace enforcing zero heap allocations in integration hot loops (`VG-1`).
///
/// Allocates all scratch vectors, stage arrays, checkpoint buffers, and Krylov pools
/// once upon construction or resize. Hot loops operate entirely on slices of these
/// pre-allocated buffers.
pub struct VigilodeWorkspace {
    /// Dimension of the state space $N$.
    pub dimension: usize,
    /// Maximum Krylov subspace dimension $m$.
    pub max_krylov_dim: usize,
    /// Pre-allocated buffer for up to 8 stage vectors ($8 \times N$).
    pub stage_vectors: Vec<f64>,
    /// Pre-allocated Krylov basis buffer ($(m + 1) \times N$).
    pub krylov_basis: Vec<f64>,
    /// Pre-allocated matrix scratch buffer ($N \times N$).
    pub linear_scratch: Vec<f64>,
    /// Checkpoint state buffer for fail-closed transactional rollback.
    pub y_checkpoint: Vec<f64>,
    /// Intermediate state scratch buffer ($N$).
    pub y_scratch: Vec<f64>,
    /// Intermediate right-hand-side scratch buffer ($N$).
    pub dy_scratch: Vec<f64>,
    /// Embedded error estimate scratch buffer ($N$).
    pub error_scratch: Vec<f64>,
    /// Error weighting scale scratch buffer ($N$).
    pub scale_scratch: Vec<f64>,
    /// Checkpoint time $t_{\text{checkpoint}}$ for rollback.
    pub checkpoint_time: f64,
    /// Accumulated operational work counters.
    pub work_counters: WorkCounters,
}

impl VigilodeWorkspace {
    /// Construct a new pre-allocated workspace for dimension `n` and Krylov basis `max_krylov_dim`.
    pub fn new(dimension: usize, max_krylov_dim: usize) -> Self {
        let n = dimension.max(1);
        let m = max_krylov_dim.max(1);
        Self {
            dimension,
            max_krylov_dim: m,
            stage_vectors: vec![0.0; 8 * n],
            krylov_basis: vec![0.0; (m + 1) * n],
            linear_scratch: vec![0.0; n * n],
            y_checkpoint: vec![0.0; n],
            y_scratch: vec![0.0; n],
            dy_scratch: vec![0.0; n],
            error_scratch: vec![0.0; n],
            scale_scratch: vec![0.0; n],
            checkpoint_time: 0.0,
            work_counters: WorkCounters::default(),
        }
    }

    /// Reset the workspace state and work counters for a new initial value problem.
    pub fn reset(&mut self, y0: &[f64]) {
        self.dimension = y0.len();
        let n = self.dimension.max(1);
        let m = self.max_krylov_dim.max(1);
        if self.stage_vectors.len() < 8 * n {
            self.stage_vectors.resize(8 * n, 0.0);
        }
        if self.krylov_basis.len() < (m + 1) * n {
            self.krylov_basis.resize((m + 1) * n, 0.0);
        }
        if self.linear_scratch.len() < n * n {
            self.linear_scratch.resize(n * n, 0.0);
        }
        if self.y_checkpoint.len() < n {
            self.y_checkpoint.resize(n, 0.0);
        }
        if self.y_scratch.len() < n {
            self.y_scratch.resize(n, 0.0);
        }
        if self.dy_scratch.len() < n {
            self.dy_scratch.resize(n, 0.0);
        }
        if self.error_scratch.len() < n {
            self.error_scratch.resize(n, 0.0);
        }
        if self.scale_scratch.len() < n {
            self.scale_scratch.resize(n, 0.0);
        }

        self.checkpoint_time = 0.0;
        self.y_checkpoint[..y0.len()].copy_from_slice(y0);
        self.work_counters = WorkCounters::default();
    }

    /// Record a transactional checkpoint $(t, y)$ for fail-closed recovery.
    pub fn checkpoint(&mut self, t: f64, y: &[f64]) {
        self.checkpoint_time = t;
        let len = y.len().min(self.y_checkpoint.len());
        self.y_checkpoint[..len].copy_from_slice(&y[..len]);
    }

    /// Roll back state to the last accepted checkpoint.
    pub fn rollback(&self, y_out: &mut [f64]) -> f64 {
        let len = y_out.len().min(self.y_checkpoint.len());
        y_out[..len].copy_from_slice(&self.y_checkpoint[..len]);
        self.checkpoint_time
    }

    /// Retained heap bytes of this workspace.
    pub fn retained_bytes(&self) -> usize {
        self.stage_vectors.capacity() * 8
            + self.krylov_basis.capacity() * 8
            + self.linear_scratch.capacity() * 8
            + self.y_checkpoint.capacity() * 8
            + self.y_scratch.capacity() * 8
            + self.dy_scratch.capacity() * 8
            + self.error_scratch.capacity() * 8
            + self.scale_scratch.capacity() * 8
    }

    /// Perform a single integration step into caller storage with zero heap allocations (`VG-1`).
    ///
    /// Operates purely on the pre-allocated scratch slices without growing or reallocating memory.
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
        // In-place Heun 2(1) step using pre-allocated workspace buffers
        (problem.rhs_fn())(t, y, &mut self.dy_scratch[..n])?;
        for i in 0..n {
            self.y_scratch[i] = y[i] + dt * self.dy_scratch[i];
        }
        (problem.rhs_fn())(t + dt, &self.y_scratch[..n], &mut self.stage_vectors[..n])?;
        for i in 0..n {
            y_out[i] = y[i] + 0.5 * dt * (self.dy_scratch[i] + self.stage_vectors[i]);
        }
        self.work_counters.rhs_calls += 2;
        self.work_counters.accepted_steps += 1;
        Ok(true)
    }
}
