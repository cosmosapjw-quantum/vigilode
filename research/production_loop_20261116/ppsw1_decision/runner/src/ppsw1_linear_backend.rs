//! Opt-in linear-algebra backends for the PPSW1-6b stage solves (U04).
//!
//! The six stage systems share one matrix `W = I - gamma h T`, `T = J(t_m, y_m)` frozen at the
//! step start.  [`LinearBackend::Dense`] is the unchanged dense-LU path
//! ([`super::solve_stages`]).  [`LinearBackend::MatrixFree`] never forms `J` or `W`:
//!
//! * `T` is the user JVP linearised once at `(t_m, y_m)` ([`OdeProblem::linearize_matrix_free`]);
//!   it is never re-linearised at a stage value.
//! * Stage right-hand sides `b_i = explicit_i + h T transfer_i` cost one charged JVP each.
//! * `W x_i = b_i` is solved by six independent GMRES runs (stage-parallel under a multi-thread
//!   [`ParallelExecution`], every job joined and charged) or by one block GMRES over the six
//!   right-hand sides (existing rank deflation).  Identity preconditioner only.
//! * The backend then recomputes every original residual `r_i = b_i - W x_i` with one charged
//!   `W` application and refuses (typed `LinearSolve`) unless
//!   `||r_i||_2 <= true_residual_slack * max(atol, rtol ||b_i||_2)`.
//!
//! Permissions: the matrix-free backend is derivative-based (it requires a model-supplied JVP)
//! and is not derivative-free.  It is per-step matrix-free and Newton-free only.  The PPSW1
//! starter ([`super::start`]) still factors a dense `5n x 5n` Newton matrix and runs simplified
//! Newton, and [`super::jacobian_norm_bound`] builds a dense Jacobian; [`start_with_policy`]
//! refuses a request that forbids either before any callback.  No whole-run matrix-free claim.
//!
//! Residual-to-step-error budget (derivation in
//! `research/production_unification_20261115/matrix_free_w/BUDGET_KO.md`): an accepted stage
//! solution differs from the exact one by `W^{-1} r_i`; for a dissipative `T` in the norm used
//! (`mu(T) <= 0`), `||W^{-1}|| <= 1`, so the per-stage perturbation is bounded by the residual
//! threshold.  It enters the estimator and the next step like a local error, so the linear
//! threshold must sit well below the step tolerance; the budget is checked empirically against
//! the dense path, not assumed.

use std::sync::Arc;

use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, IdentityPreconditioner, LinearOperator,
    OperatorApplicationWork, ShiftedOperator, WorkCounters, apply_counted, safe_l2,
};
use rodas5p_krylov::{BlockGmresConfig, GmresConfig, solve_block_gmres, solve_gmres};

use super::{STAGES, StageInput, StageSolve, State};
use crate::{OdeProblem, ParallelExecution};

/// Krylov method of the matrix-free backend.
#[derive(Clone, Debug)]
pub enum KrylovMethod {
    /// Six independent restarted GMRES solves (stage-parallel).
    IndependentGmres(GmresConfig),
    /// One block GMRES over the six right-hand sides (rank-deflating).
    BlockGmres(BlockGmresConfig),
}

/// Configuration of the matrix-free backend.
#[derive(Clone, Debug)]
pub struct MatrixFreeConfig {
    pub method: KrylovMethod,
    /// Recompute every original residual with one extra `W` application per stage.
    pub verify_true_residual: bool,
    /// Accept `||r_i|| <= slack * max(atol, rtol ||b_i||)` in the backend check (>= 1).
    pub true_residual_slack: f64,
}

impl MatrixFreeConfig {
    pub fn independent(config: GmresConfig) -> Self {
        Self {
            method: KrylovMethod::IndependentGmres(config),
            verify_true_residual: true,
            true_residual_slack: 10.0,
        }
    }
    pub fn block(config: BlockGmresConfig) -> Self {
        Self {
            method: KrylovMethod::BlockGmres(config),
            verify_true_residual: true,
            true_residual_slack: 10.0,
        }
    }
    fn tolerances(&self) -> (f64, f64) {
        match &self.method {
            KrylovMethod::IndependentGmres(c) => (c.rtol, c.atol),
            KrylovMethod::BlockGmres(c) => (c.rtol, c.atol),
        }
    }
    fn validate(&self) -> CoreResult<()> {
        if !(self.true_residual_slack.is_finite() && self.true_residual_slack >= 1.0) {
            return Err(CoreError::InvalidInput(
                "true_residual_slack must be finite and >= 1".into(),
            ));
        }
        match &self.method {
            KrylovMethod::IndependentGmres(c) => c.validate(),
            KrylovMethod::BlockGmres(c) => {
                let finite = |x: f64| x.is_finite() && x >= 0.0;
                let rank_ok = c.rank_tolerance.is_finite() && c.rank_tolerance > 0.0;
                if c.max_basis == 0 || !finite(c.rtol) || !finite(c.atol) || !rank_ok {
                    return Err(CoreError::InvalidInput(
                        "block GMRES configuration is invalid".into(),
                    ));
                }
                Ok(())
            }
        }
    }
}

/// Linear-algebra backend of the stage solves.
#[derive(Clone, Debug, Default)]
pub enum LinearBackend {
    /// Dense Jacobian and one LU of `W` (the original PPSW1 path).
    #[default]
    Dense,
    /// JVP-based matrix-free Krylov path.
    MatrixFree(MatrixFreeConfig),
}

impl LinearBackend {
    /// Refuse, without calling any user callback, a backend the problem cannot support.
    pub fn admit(&self, problem: &OdeProblem) -> CoreResult<()> {
        match self {
            LinearBackend::Dense => Ok(()),
            LinearBackend::MatrixFree(cfg) => {
                cfg.validate()?;
                if problem.mass_matrix.is_some() {
                    return Err(CoreError::InvalidInput(
                        "PPSW1 matrix-free backend supports identity mass only".into(),
                    ));
                }
                if !problem.supports_matrix_free_jvp() {
                    return Err(CoreError::InvalidInput(
                        "PPSW1 matrix-free backend requires a user-supplied JVP".into(),
                    ));
                }
                Ok(())
            }
        }
    }

    /// Residual-to-step-error budget: refuse a matrix-free linear tolerance looser than
    /// [`LINEAR_BUDGET_RATIO`] times the step tolerance (rtol, or atol when the step rtol is
    /// zero). Pure check, no callback.
    pub fn check_budget(&self, step_rtol: f64, step_atol: f64) -> CoreResult<()> {
        let LinearBackend::MatrixFree(cfg) = self else {
            return Ok(());
        };
        let (rtol, atol) = cfg.tolerances();
        let ok = if step_rtol > 0.0 {
            rtol <= LINEAR_BUDGET_RATIO * step_rtol
        } else {
            atol <= LINEAR_BUDGET_RATIO * step_atol
        };
        if ok {
            Ok(())
        } else {
            Err(CoreError::InvalidInput(format!(
                "PPSW1 matrix-free linear tolerance (rtol {rtol:e}, atol {atol:e}) exceeds \
                 {LINEAR_BUDGET_RATIO:e} x step tolerance (rtol {step_rtol:e}, atol {step_atol:e})"
            )))
        }
    }
}

/// Largest admitted ratio of the matrix-free linear tolerance to the step tolerance in
/// [`super::Session::try_step`]. Evidence (U04 c4, Fisher MOL N=40): a ratio of 1 collapsed the
/// controller (1.13e6 steps, endpoint 58 tolerance units off Dense); every row with ratio
/// <= 1e-2 stayed within 1.1e-3 tolerance units of Dense. Scoped evidence, not a theorem.
pub const LINEAR_BUDGET_RATIO: f64 = 1e-2;

/// Per-step report of the linear backend (the counters carry the work).
#[derive(Clone, Debug, PartialEq)]
pub struct LinearReport {
    pub method: &'static str,
    /// Backend-recomputed `||b_i - W x_i||_2` (empty when verification is off or Dense).
    pub true_residual_norms: Vec<f64>,
    /// `max(atol, rtol ||b_i||_2)` per stage (empty for Dense).
    pub thresholds: Vec<f64>,
    /// Krylov iterations summed over the six stages (or block iterations).
    pub krylov_iterations: u64,
    /// Numerical rank of the initial right-hand-side block (block GMRES only).
    pub initial_block_rank: Option<usize>,
}

fn charge_jvp(work: OperatorApplicationWork, counters: &mut WorkCounters) {
    counters.jvp_calls = counters.jvp_calls.saturating_add(work.jvp_calls);
    counters.jvp_vectors = counters.jvp_vectors.saturating_add(work.jvp_vectors);
}

/// `b_i = explicit_i + h T transfer_i`; one charged JVP.
fn stage_rhs(
    jac: &dyn LinearOperator,
    input: &StageInput,
    h: f64,
    counters: &mut WorkCounters,
) -> CoreResult<Vec<f64>> {
    let mut tv = vec![0.0; input.transfer.len()];
    let mut work = OperatorApplicationWork::default();
    let outcome = jac.apply_with_work(&input.transfer, &mut tv, &mut work);
    charge_jvp(work, counters);
    outcome?;
    let b: Vec<f64> = input
        .explicit
        .iter()
        .zip(&tv)
        .map(|(e, v)| e + h * v)
        .collect();
    if b.iter().all(|x| x.is_finite()) {
        Ok(b)
    } else {
        Err(CoreError::NonFinite(
            "PPSW1 matrix-free stage RHS is non-finite".into(),
        ))
    }
}

fn verify(
    w: &dyn LinearOperator,
    b: &[f64],
    x: &[f64],
    threshold: f64,
    slack: f64,
    counters: &mut WorkCounters,
) -> CoreResult<f64> {
    let mut wx = vec![0.0; b.len()];
    apply_counted(w, x, &mut wx, counters, ApplyCategory::Diagnostic)?;
    let r: Vec<f64> = b.iter().zip(&wx).map(|(bi, wi)| bi - wi).collect();
    let norm = safe_l2(&r);
    if !(norm.is_finite() && norm <= slack * threshold) {
        return Err(CoreError::LinearSolve(format!(
            "PPSW1 matrix-free true residual {norm:.3e} exceeds {:.3e}",
            slack * threshold
        )));
    }
    Ok(norm)
}

type StageResult = (Vec<f64>, f64, f64, u64);

/// Solve the six stage systems with the selected backend.
pub fn solve_stages_with_backend(
    problem: &OdeProblem,
    state: &State,
    h: f64,
    inputs: &[StageInput; 6],
    backend: &LinearBackend,
    exec: &ParallelExecution,
    counters: &mut WorkCounters,
) -> CoreResult<(StageSolve, LinearReport)> {
    let cfg = match backend {
        LinearBackend::Dense => {
            let sol = super::solve_stages(problem, state, h, inputs, exec, counters)?;
            return Ok((
                sol,
                LinearReport {
                    method: "dense-lu",
                    true_residual_norms: Vec::new(),
                    thresholds: Vec::new(),
                    krylov_iterations: 0,
                    initial_block_rank: None,
                },
            ));
        }
        LinearBackend::MatrixFree(cfg) => cfg,
    };
    backend.admit(problem)?;
    super::check_positive("step size", h)?;
    let n = problem.dimension;
    super::validate_state(state, n)?;
    super::advancing_time(state.t, h)?;
    if inputs.iter().any(|s| {
        s.explicit.len() != n
            || s.transfer.len() != n
            || !s.t.is_finite()
            || !super::all_finite(&s.explicit)
            || !super::all_finite(&s.transfer)
    }) {
        return Err(CoreError::InvalidInput(
            "PPSW1 stage inputs must be finite and of the problem dimension".into(),
        ));
    }
    // Frozen linearisation at (t_m, y_m): no callback is made here.
    let jac = problem.linearize_matrix_free(state.t, &state.y[5])?;
    let w = ShiftedOperator::new_counted_jvp(None, Arc::clone(&jac), h, super::gamma())?;
    let pc = IdentityPreconditioner::new(n);
    let (rtol, atol) = cfg.tolerances();
    let slack = cfg.true_residual_slack;
    match &cfg.method {
        KrylovMethod::IndependentGmres(gcfg) => {
            let stage = |i: usize, work: &mut WorkCounters| -> CoreResult<StageResult> {
                let b = stage_rhs(jac.as_ref(), &inputs[i], h, work)?;
                let threshold = atol.max(rtol * safe_l2(&b));
                let before = work.linear_iterations;
                let rep = solve_gmres(&w, &pc, &b, None, gcfg, work)?;
                let iters = work.linear_iterations - before;
                let res = if cfg.verify_true_residual {
                    verify(&w, &b, &rep.x, threshold, slack, work)?
                } else {
                    f64::NAN
                };
                Ok((rep.x, res, threshold, iters))
            };
            let results: Vec<StageResult> = if exec.threads() == 1 {
                let mut out = Vec::with_capacity(STAGES);
                for i in 0..STAGES {
                    out.push(stage(i, counters)?);
                }
                out
            } else {
                let idx = [0usize, 1, 2, 3, 4, 5];
                // Join every stage job: no Rayon early cancellation, all work charged.
                let jobs = exec.map_ordered(&idx, |&i| {
                    let mut work = WorkCounters::default();
                    let r = stage(i, &mut work);
                    Ok((r, work))
                })?;
                for (_, work) in &jobs {
                    counters.accumulate(*work);
                }
                jobs.into_iter()
                    .map(|(r, _)| r)
                    .collect::<CoreResult<Vec<_>>>()?
            };
            let mut ys = Vec::with_capacity(STAGES);
            let mut report = LinearReport {
                method: "matrix-free-independent-gmres",
                true_residual_norms: Vec::new(),
                thresholds: Vec::new(),
                krylov_iterations: 0,
                initial_block_rank: None,
            };
            for (x, res, thr, it) in results {
                ys.push(x);
                if cfg.verify_true_residual {
                    report.true_residual_norms.push(res);
                }
                report.thresholds.push(thr);
                report.krylov_iterations += it;
            }
            Ok((
                StageSolve {
                    y: super::to_six(ys)?,
                },
                report,
            ))
        }
        KrylovMethod::BlockGmres(bcfg) => {
            let mut rhs = Vec::with_capacity(STAGES);
            for input in inputs.iter() {
                rhs.push(stage_rhs(jac.as_ref(), input, h, counters)?);
            }
            let thresholds: Vec<f64> = rhs.iter().map(|b| atol.max(rtol * safe_l2(b))).collect();
            let rep = solve_block_gmres(&w, &pc, &rhs, bcfg, counters)?;
            let mut norms = Vec::new();
            if cfg.verify_true_residual {
                for ((b, x), thr) in rhs.iter().zip(&rep.solutions).zip(&thresholds) {
                    norms.push(verify(&w, b, x, *thr, slack, counters)?);
                }
            }
            Ok((
                StageSolve {
                    y: super::to_six(rep.solutions)?,
                },
                LinearReport {
                    method: "matrix-free-block-gmres",
                    true_residual_norms: norms,
                    thresholds,
                    krylov_iterations: rep.block_iterations as u64,
                    initial_block_rank: Some(rep.initial_block_rank),
                },
            ))
        }
    }
}

/// Which linear-algebra capabilities a caller permits for the starter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StarterPolicy {
    pub allow_dense_matrix: bool,
    pub allow_newton: bool,
}

impl Default for StarterPolicy {
    fn default() -> Self {
        Self {
            allow_dense_matrix: true,
            allow_newton: true,
        }
    }
}

/// [`super::start`] under an explicit policy.  The only PPSW1 starter factors a dense
/// `5n x 5n` matrix and runs simplified Newton, so forbidding either is refused with
/// `InvalidInput` before any callback; no matrix-free starter is qualified.
pub fn start_with_policy(
    problem: &OdeProblem,
    t_init: f64,
    y0: &[f64],
    h0: f64,
    policy: StarterPolicy,
    counters: &mut WorkCounters,
) -> CoreResult<State> {
    if !policy.allow_dense_matrix || !policy.allow_newton {
        return Err(CoreError::InvalidInput(
            "PPSW1 starter requires a dense 5n LU and simplified Newton; no qualified \
             matrix-free/Newton-free starter exists"
                .into(),
        ));
    }
    super::start(problem, t_init, y0, h0, counters)
}
