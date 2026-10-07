use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinearMethod {
    Direct,
    Gmres,
    Lgmres,
    Gcrodr,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreconditionerKind {
    None,
    Jacobi,
    Direct,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitialGuess {
    Zero,
    Previous,
    /// Stage `i` starts from stage `i` of the last accepted step (research
    /// node `research/spd07_mf_step_warm_start_20261007`). Honoured only by
    /// the matrix-free U-form fast driver, which records each accepted step
    /// (`Rodas5pMfFastWorkspace::record_accepted_step`); a caller driving the
    /// workspace's `attempt` itself must record accepted steps too, or the
    /// start stays `Previous`. The sequential stage solver refuses it. Before
    /// the first accepted step of an integration it acts as `Previous`.
    PreviousStep,
    /// [`InitialGuess::PreviousStep`] scaled by `h / h_prev` (same node,
    /// reported only).
    PreviousStepScaled,
}

impl InitialGuess {
    /// Whether the start comes from the last accepted step's stages.
    pub fn is_step_indexed(self) -> bool {
        matches!(self, Self::PreviousStep | Self::PreviousStepScaled)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinearSolverConfig {
    pub method: LinearMethod,
    pub rtol: f64,
    pub atol: f64,
    pub restart: usize,
    pub maxiter: usize,
    pub inner_m: usize,
    pub outer_k: usize,
    pub recycle_dim: usize,
    pub recycle_rank_tol: f64,
    pub preconditioner: PreconditionerKind,
    pub x0_strategy: InitialGuess,
}
impl Default for LinearSolverConfig {
    fn default() -> Self {
        Self {
            method: LinearMethod::Direct,
            rtol: 1e-11,
            atol: 1e-13,
            restart: 40,
            maxiter: 200,
            inner_m: 30,
            outer_k: 8,
            recycle_dim: 8,
            recycle_rank_tol: 1e-12,
            preconditioner: PreconditionerKind::None,
            x0_strategy: InitialGuess::Previous,
        }
    }
}
impl LinearSolverConfig {
    pub fn validate(&self) -> Result<(), String> {
        // `< 0.0` alone lets NaN and +Inf through to every kernel (audit F-034).
        let finite_nonnegative = |value: f64| value.is_finite() && value >= 0.0;
        if !finite_nonnegative(self.rtol) || !finite_nonnegative(self.atol) {
            return Err("linear tolerances must be finite and nonnegative".into());
        }
        if self.restart == 0 || self.maxiter == 0 || self.inner_m == 0 || self.outer_k == 0 {
            return Err("iteration limits must be positive".into());
        }
        if self.recycle_dim == 0 {
            return Err("recycle_dim must be positive".into());
        }
        if !(0.0 < self.recycle_rank_tol && self.recycle_rank_tol < 1.0) {
            return Err("recycle_rank_tol must lie in (0,1)".into());
        }
        if self.method == LinearMethod::Gcrodr && self.recycle_dim >= self.restart {
            return Err("GCRO-DR requires recycle_dim < restart".into());
        }
        if self.method == LinearMethod::Direct && self.preconditioner != PreconditionerKind::None {
            return Err("direct solve cannot use a Krylov preconditioner".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinearSolveReport {
    pub x: Vec<f64>,
    pub converged: bool,
    pub info: i32,
    pub residual_norm: f64,
    pub relative_residual: f64,
    pub iterations: u64,
    pub matvecs: u64,
    pub preconditioner_apps: u64,
    pub method: String,
}
