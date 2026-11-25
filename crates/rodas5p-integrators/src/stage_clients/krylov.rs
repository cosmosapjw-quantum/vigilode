//! Per-stage GMRES and shared-basis Krylov stage clients (loop node V06).
//!
//! [`KrylovStageClient`] solves the stage systems `W x = b`,
//! `W = I/(h gamma) - J`, of
//! [`crate::integrate_rodas5p_fast_client_observed_with_atol`] with the
//! problem's JVP only. It has two modes with one stopping test:
//!
//! * per stage ([`KrylovStageClient::per_stage`]): every solve builds its
//!   own Krylov space with restarted GMRES;
//! * shared basis ([`KrylovStageClient::shared_basis`]): the eight solves of
//!   one attempt extend one basis through
//!   [`rodas5p_krylov::shared_basis::SharedBasis`], as the opt-in hook of
//!   the protected matrix-free driver does (`rodas5p_matrix_free_fast.rs`,
//!   `attempt_inner`); a solve the kernel does not finish falls back to the
//!   per-stage GMRES from a zero start.
//!
//! # Stopping test
//!
//! [`StageStoppingRule`] is the test of the protected matrix-free driver,
//! which solves the scaled system `(I - h gamma J) x = h gamma b` and
//! accepts an iterate when the 2-norm of its true residual is at most
//! `max(gamma atol, rtol ||h gamma b||)` (`rodas5p_matrix_free_fast.rs`,
//! `linear_atol` and `GmresConfig`; `rodas5p-krylov` `gmres_into.rs`, the
//! true residual at the top of every restart pass; `common.rs`,
//! `residual_threshold`). Divided by `h gamma` it reads
//! `||b - W x|| <= max(atol gamma / (h gamma), rtol ||b||)`, the form used
//! here. Both modes accept an iterate only on a true residual formed with
//! one fresh JVP (or on the right-hand side itself for the zero iterate);
//! that JVP is charged like every other.
//!
//! The protected kernel tests the residual once per restart pass. The
//! per-stage mode can do the same ([`PerStageGmresOptions::shipped_cadence`])
//! or leave a pass as soon as the projected residual of the Arnoldi relation
//! meets the threshold ([`PerStageGmresOptions::baseline`]); the shared
//! kernel always tests after every new basis vector.
//!
//! # Counters
//!
//! The driver charges `jacobian_builds` and `linear_solves`. The client
//! charges every JVP through [`rodas5p_core::apply_counted`] (Krylov
//! vectors and stopping-test residuals in the `Krylov` category, the final
//! residual of the shipped cadence in `Diagnostic`), `linear_iterations`
//! per Krylov vector, `preconditioner_apps` per Jacobi application and the
//! orthogonalization counters of its own GMRES. [`KrylovStats`] splits the
//! JVPs by purpose and by stage.
//!
//! # Allocation
//!
//! Every buffer is allocated at construction. `linearize`, `factor` and
//! `solve_in_place` do not allocate, except for the message of an error.
//!
//! # Independent residual check
//!
//! [`ResidualAuditClient`] wraps any client, saves `b` before each solve and
//! tests the original residual `||b - W x||` afterwards with its own JVP
//! operator and its own counters.
//!
//! # Step-tied tolerance (loop node V17)
//!
//! [`KrylovStageClient::with_step_tied_tolerance`] gives a client the
//! policy of [`StepTiedTolerance`]: every solve is then accepted at the
//! larger of the protected threshold and
//! `kappa sqrt(n) min_i (atol + rtol |y_i|) / (h gamma)`, with `y` the state
//! of the last linearization. Without the call nothing changes.
//! [`ResidualAuditClient::with_step_tied_tolerance`] makes the audit test
//! against the same threshold, computed from the state it is linearized at.

mod step_tied;

pub use step_tied::StepTiedTolerance;

use std::sync::{Arc, Mutex, PoisonError};

use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, LinearOperator, WorkCounters, apply_counted,
    rodas5p_coefficients, safe_l2,
};
use rodas5p_krylov::shared_basis::{SharedBasis, SharedBasisConfig};

use crate::problem::JvpOperator;
use crate::{OdeProblem, StageLinearSystem};

/// Slots of the per-stage tallies of [`KrylovStats`]: the stages of RODAS5P.
/// A solve beyond the eighth of an attempt is booked in the last slot.
pub const KRYLOV_STAGE_SLOTS: usize = 8;

/// The linear stopping test of the protected matrix-free driver in the
/// client's unscaled form; see the module documentation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageStoppingRule {
    rtol: f64,
    atol: f64,
    gamma: f64,
}

impl StageStoppingRule {
    /// Relative tolerance of the routed production matrix-free path
    /// (`routing.rs`, `RoutingOptions::default`).
    pub const PRODUCTION_RTOL: f64 = 1.0e-10;
    /// Absolute tolerance of the routed production matrix-free path.
    pub const PRODUCTION_ATOL: f64 = 1.0e-14;

    /// The test with linear tolerances `rtol` and `atol` (finite and
    /// nonnegative) and the method's `gamma`.
    pub fn new(rtol: f64, atol: f64) -> CoreResult<Self> {
        if !(rtol.is_finite() && rtol >= 0.0 && atol.is_finite() && atol >= 0.0) {
            return Err(CoreError::InvalidInput(
                "stage stopping tolerances must be finite and nonnegative".into(),
            ));
        }
        Ok(Self {
            rtol,
            atol,
            gamma: rodas5p_coefficients()?.gamma,
        })
    }

    /// The test of the routed production matrix-free path.
    pub fn protected_production() -> CoreResult<Self> {
        Self::new(Self::PRODUCTION_RTOL, Self::PRODUCTION_ATOL)
    }

    /// The relative tolerance.
    pub fn rtol(&self) -> f64 {
        self.rtol
    }

    /// The absolute tolerance (of the scaled system, before `gamma`).
    pub fn atol(&self) -> f64 {
        self.atol
    }

    /// The residual bound `max(atol gamma / (h gamma), rtol ||b||)` for a
    /// right-hand side of 2-norm `rhs_norm` and `inv_h_gamma = 1/(h gamma)`.
    pub fn threshold(&self, inv_h_gamma: f64, rhs_norm: f64) -> CoreResult<f64> {
        if !rhs_norm.is_finite() {
            return Err(CoreError::NonFinite(
                "stage right-hand side norm is NaN/Inf".into(),
            ));
        }
        let threshold =
            (self.atol * self.gamma.abs() * inv_h_gamma.abs()).max(self.rtol * rhs_norm);
        if threshold.is_finite() {
            Ok(threshold)
        } else {
            Err(CoreError::InvalidInput(
                "stage residual threshold is not finite".into(),
            ))
        }
    }
}

/// Writes the diagonal of the Jacobian at `(t, y)` into its last argument.
pub type JacobianDiagonalFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// The preconditioner of a [`KrylovStageClient`], the same in both modes.
#[derive(Clone, Default)]
pub enum KrylovPreconditioner {
    /// No preconditioner (the only option of the protected matrix-free
    /// driver).
    #[default]
    None,
    /// Right preconditioning by `D = diag(W) = 1/(h gamma) - diag(J)`: the
    /// solver works on `W D^-1 u = b`, `x = D^-1 u`, so its residual is the
    /// original one. The callback supplies `diag(J)`.
    RightJacobi(JacobianDiagonalFn),
}

/// Options of the per-stage restarted GMRES.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PerStageGmresOptions {
    /// Arnoldi columns per restart pass.
    pub restart: usize,
    /// Arnoldi columns per solve; a solve that needs more is a
    /// [`CoreError::LinearSolve`].
    pub max_arnoldi: usize,
    /// Leave a pass when the projected residual meets the threshold.
    pub in_cycle_exit: bool,
    /// Form the true residual once more after the loop (`Diagnostic`), as
    /// the protected kernel's default accounting does.
    pub recompute_final: bool,
}

impl PerStageGmresOptions {
    /// Restart 40 and 200 vectors as the routed production path, leaving a
    /// pass on the projected residual.
    pub const fn baseline() -> Self {
        Self {
            restart: 40,
            max_arnoldi: 200,
            in_cycle_exit: true,
            recompute_final: false,
        }
    }

    /// The cadence of the protected kernel: full passes and a final
    /// diagnostic residual.
    pub const fn shipped_cadence() -> Self {
        Self {
            restart: 40,
            max_arnoldi: 200,
            in_cycle_exit: false,
            recompute_final: true,
        }
    }

    fn validate(&self) -> CoreResult<()> {
        if self.restart == 0 || self.max_arnoldi == 0 {
            return Err(CoreError::InvalidInput(
                "per-stage GMRES needs positive restart and max_arnoldi".into(),
            ));
        }
        Ok(())
    }
}

/// Options of the shared-basis mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SharedBasisOptions {
    /// Largest dimension of the shared basis.
    pub max_dim: usize,
    /// Keep the basis for the retry after an attempt rejected by the error
    /// estimate (same Jacobian, new `h`). Only without a preconditioner.
    pub keep_across_rejection: bool,
    /// How often the kernel is called again with a halved internal target
    /// when the confirming true residual misses the threshold.
    pub confirmation_retries: u32,
}

impl SharedBasisOptions {
    /// A basis of 200 vectors (the Arnoldi budget of one protected stage
    /// solve), reset at every attempt.
    pub const fn baseline() -> Self {
        Self {
            max_dim: 200,
            keep_across_rejection: false,
            confirmation_retries: 4,
        }
    }
}

/// Work of a [`KrylovStageClient`], split by purpose.
///
/// `krylov_vectors + residual_jvps` is the number of JVPs the client
/// charged to the driver's counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KrylovStats {
    pub linearizations: u64,
    /// Calls of `factor`, one per attempt.
    pub attempts: u64,
    pub solves: u64,
    /// Solves that returned an error.
    pub failed_solves: u64,
    /// JVPs that built a Krylov vector (Arnoldi columns of every GMRES
    /// solve, the fallback included, and vectors of the shared basis).
    pub krylov_vectors: u64,
    /// JVPs of stopping-test residuals.
    pub residual_jvps: u64,
    /// Restart passes of the per-stage GMRES (fallback solves included).
    pub restart_passes: u64,
    /// Most Krylov vectors one solve built.
    pub max_solve_vectors: u64,
    /// Solves accepted at the zero iterate without a JVP.
    pub zero_iterate_accepts: u64,
    /// Shared mode: true residuals above the threshold after the kernel
    /// reported convergence.
    pub confirmation_failures: u64,
    /// Krylov vectors by position of the solve in its attempt.
    pub stage_krylov_vectors: [u64; KRYLOV_STAGE_SLOTS],
    /// Solves by position in their attempt.
    pub stage_solves: [u64; KRYLOV_STAGE_SLOTS],
    /// Vectors appended to the shared basis.
    pub shared_vectors: u64,
    /// Solves accepted from the shared basis.
    pub shared_accepted_solves: u64,
    /// Solves that fell back to the per-stage GMRES.
    pub fallback_solves: u64,
    pub fallback_krylov_vectors: u64,
    pub fallback_residual_jvps: u64,
    /// Largest dimension of the shared basis.
    pub basis_dim_max: u64,
    /// Sum over attempts of the basis dimension after the attempt's last
    /// solve, and the number of attempts summed.
    pub basis_dim_sum: u64,
    pub basis_dim_attempts: u64,
    /// Attempts that started from the basis of the rejected one.
    pub basis_kept: u64,
    /// Bytes of the GMRES basis and Hessenberg storage.
    pub gmres_storage_bytes: u64,
    /// Bytes of the shared kernel's `Q`, `Z`, scratch matrix and vectors.
    pub shared_storage_bytes: u64,
    /// Solves whose threshold was the bound of a [`StepTiedTolerance`]
    /// (larger than the protected one); zero without the policy.
    pub policy_bound_solves: u64,
}

/// A reader of a client's [`KrylovStats`] that outlives the client's move
/// into the driver. The client publishes after every call.
#[derive(Clone, Debug)]
pub struct KrylovStatsHandle {
    shared: Arc<Mutex<KrylovStats>>,
}

impl KrylovStatsHandle {
    /// The statistics published so far.
    pub fn snapshot(&self) -> KrylovStats {
        *self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What one solve did; folded into [`KrylovStats`] whether it succeeded or
/// not.
#[derive(Clone, Copy, Debug, Default)]
struct SolveTally {
    krylov_vectors: u64,
    residual_jvps: u64,
    passes: u64,
    shared_vectors: u64,
    shared_accepted: bool,
    zero_accept: bool,
    confirmation_failures: u64,
    fallback: bool,
    fallback_krylov_vectors: u64,
    fallback_residual_jvps: u64,
    policy_bound: bool,
}

/// `residual = b - W x`, `W = inv_h_gamma I - J`, with one JVP charged to
/// `counters` in `category`; returns the 2-norm. The clients' confirming
/// residual and the audit's residual are this function.
fn stage_true_residual(
    jacobian: &dyn LinearOperator,
    inv_h_gamma: f64,
    b: &[f64],
    x: &[f64],
    residual: &mut [f64],
    counters: &mut WorkCounters,
    category: ApplyCategory,
) -> CoreResult<f64> {
    apply_counted(jacobian, x, residual, counters, category)?;
    for ((r, bi), xi) in residual.iter_mut().zip(b).zip(x) {
        *r = bi - (inv_h_gamma * xi - *r);
    }
    Ok(safe_l2(residual))
}

/// The operator of one attempt: `W = inv I - J` and, under Jacobi, the
/// inverse diagonal of `W`.
struct StageOperator<'a> {
    jacobian: &'a JvpOperator,
    inv: f64,
    inv_diag: Option<&'a [f64]>,
}

impl StageOperator<'_> {
    /// `out = W v` with one JVP in the `Krylov` category.
    fn apply_w(&self, v: &[f64], out: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        apply_counted(self.jacobian, v, out, counters, ApplyCategory::Krylov)?;
        for (o, vi) in out.iter_mut().zip(v) {
            *o = self.inv * vi - *o;
        }
        Ok(())
    }
}

/// Storage of the per-stage GMRES.
struct GmresWork {
    n: usize,
    /// Columns per pass the storage holds.
    m: usize,
    /// `(m + 1) n`, column-major Arnoldi basis.
    basis: Vec<f64>,
    /// One Hessenberg column (`m + 1`).
    hess: Vec<f64>,
    /// `m m`, column-major upper triangle after the Givens rotations.
    r: Vec<f64>,
    cs: Vec<f64>,
    sn: Vec<f64>,
    /// Rotated right-hand side (`m + 1`).
    g: Vec<f64>,
    y: Vec<f64>,
    x: Vec<f64>,
    res: Vec<f64>,
    work: Vec<f64>,
    pv: Vec<f64>,
}

impl GmresWork {
    fn new(n: usize, options: &PerStageGmresOptions) -> CoreResult<Self> {
        options.validate()?;
        let m = options.restart.min(options.max_arnoldi).min(n);
        let basis_len = (m + 1).checked_mul(n).ok_or_else(|| {
            CoreError::InvalidInput("per-stage GMRES basis size overflows".into())
        })?;
        Ok(Self {
            n,
            m,
            basis: vec![0.0; basis_len],
            hess: vec![0.0; m + 1],
            r: vec![0.0; m * m],
            cs: vec![0.0; m],
            sn: vec![0.0; m],
            g: vec![0.0; m + 1],
            y: vec![0.0; m],
            x: vec![0.0; n],
            res: vec![0.0; n],
            work: vec![0.0; n],
            pv: vec![0.0; n],
        })
    }

    fn storage_bytes(&self) -> u64 {
        let doubles = self.basis.len()
            + self.hess.len()
            + self.r.len()
            + self.cs.len()
            + self.sn.len()
            + self.g.len()
            + self.y.len()
            + self.x.len()
            + self.res.len()
            + self.work.len()
            + self.pv.len();
        (doubles * std::mem::size_of::<f64>()) as u64
    }
}

/// The rotation `[c s; -s c]` that maps `(a, b)` to `(r, 0)`.
fn givens(a: f64, b: f64) -> (f64, f64, f64) {
    if b == 0.0 {
        (1.0, 0.0, a)
    } else if a == 0.0 {
        (0.0, b.signum(), b.abs())
    } else {
        let r = a.hypot(b);
        (a / r, b / r, r)
    }
}

/// Restarted GMRES for `W x = b` from the zero start; on success `b` holds
/// `x`. The loop is the protected kernel's (`gmres_into.rs`): a true
/// residual at the top of every pass decides, the pass starts from that
/// residual, the budget is `max_arnoldi` columns. With
/// `options.in_cycle_exit` a pass also ends when the projected residual
/// meets `tau`.
#[allow(clippy::too_many_arguments)]
fn gmres_solve(
    work: &mut GmresWork,
    op: &StageOperator<'_>,
    tau: f64,
    b_norm: f64,
    b: &mut [f64],
    options: PerStageGmresOptions,
    counters: &mut WorkCounters,
    tally: &mut SolveTally,
) -> CoreResult<()> {
    let (n, m) = (work.n, work.m);
    work.x.fill(0.0);
    let mut total = 0usize;
    loop {
        let residual_norm = if work.x.iter().all(|value| *value == 0.0) {
            work.res.copy_from_slice(b);
            b_norm
        } else {
            tally.residual_jvps += 1;
            stage_true_residual(
                op.jacobian,
                op.inv,
                b,
                &work.x,
                &mut work.res,
                counters,
                ApplyCategory::Krylov,
            )?
        };
        if !residual_norm.is_finite() {
            return Err(CoreError::NonFinite(
                "per-stage GMRES residual is NaN/Inf".into(),
            ));
        }
        if residual_norm <= tau {
            break;
        }
        if total >= options.max_arnoldi {
            return Err(CoreError::LinearSolve(format!(
                "per-stage GMRES exhausted {} Arnoldi vectors",
                options.max_arnoldi
            )));
        }
        let beta = residual_norm;
        if beta <= f64::MIN_POSITIVE {
            return Err(CoreError::LinearSolve(
                "per-stage GMRES residual breakdown".into(),
            ));
        }
        let steps = options.restart.min(options.max_arnoldi - total).min(m);
        for (v, r) in work.basis[..n].iter_mut().zip(&work.res) {
            *v = r / beta;
        }
        work.g[0] = beta;
        let mut columns = 0usize;
        for j in 0..steps {
            let (previous, rest) = work.basis.split_at_mut((j + 1) * n);
            let vj = &previous[j * n..];
            let w = &mut rest[..n];
            match op.inv_diag {
                Some(inv_diag) => {
                    for ((p, v), d) in work.pv.iter_mut().zip(vj).zip(inv_diag) {
                        *p = v * d;
                    }
                    counters.preconditioner_apps += 1;
                    counters.preconditioner_vectors += 1;
                    op.apply_w(&work.pv, w, counters)?;
                }
                None => op.apply_w(vj, w, counters)?,
            }
            tally.krylov_vectors += 1;
            // Two-pass modified Gram-Schmidt, as the protected kernel.
            work.hess[..=j].fill(0.0);
            for _pass in 0..2 {
                for (vi, h) in previous.chunks_exact(n).zip(work.hess.iter_mut()) {
                    let mut dot = 0.0;
                    for (a, c) in vi.iter().zip(w.iter()) {
                        dot += a * c;
                    }
                    *h += dot;
                    for (c, a) in w.iter_mut().zip(vi) {
                        *c -= dot * a;
                    }
                }
            }
            let projections = 2 * (j as u64 + 1);
            counters.orthogonalization_inner_products += projections;
            counters.orthogonalization_vector_updates += projections;
            let h_next = safe_l2(w);
            if !h_next.is_finite() {
                return Err(CoreError::NonFinite(
                    "per-stage GMRES Arnoldi vector is NaN/Inf".into(),
                ));
            }
            // The protected kernel's breakdown rule (`gmres.rs`,
            // `arnoldi_happy_breakdown_from_norm`).
            let column_scale = safe_l2(&work.hess[..=j]).hypot(h_next);
            let happy = column_scale == 0.0 || h_next <= 100.0 * f64::EPSILON * column_scale;
            columns = j + 1;
            for i in 0..j {
                let (c, s) = (work.cs[i], work.sn[i]);
                let (a, d) = (work.hess[i], work.hess[i + 1]);
                work.hess[i] = c * a + s * d;
                work.hess[i + 1] = -s * a + c * d;
            }
            let (c, s, diagonal) = givens(work.hess[j], h_next);
            work.cs[j] = c;
            work.sn[j] = s;
            work.hess[j] = diagonal;
            work.r[j * m..j * m + j + 1].copy_from_slice(&work.hess[..=j]);
            let gj = work.g[j];
            work.g[j] = c * gj;
            work.g[j + 1] = -s * gj;
            let projected = work.g[j + 1].abs();
            if !happy {
                for value in w.iter_mut() {
                    *value /= h_next;
                }
            }
            if happy || (options.in_cycle_exit && projected <= tau) {
                break;
            }
        }
        for i in (0..columns).rev() {
            let mut sum = work.g[i];
            for k in i + 1..columns {
                sum -= work.r[k * m + i] * work.y[k];
            }
            let diagonal = work.r[i * m + i];
            if diagonal == 0.0 || !diagonal.is_finite() {
                return Err(CoreError::LinearSolve(
                    "per-stage GMRES triangular factor is singular".into(),
                ));
            }
            work.y[i] = sum / diagonal;
        }
        work.work.fill(0.0);
        for (vk, yk) in work.basis.chunks_exact(n).zip(&work.y[..columns]) {
            for (c, v) in work.work.iter_mut().zip(vk) {
                *c += yk * v;
            }
        }
        if let Some(inv_diag) = op.inv_diag {
            for (c, d) in work.work.iter_mut().zip(inv_diag) {
                *c *= d;
            }
            counters.preconditioner_apps += 1;
            counters.preconditioner_vectors += 1;
        }
        for (x, c) in work.x.iter_mut().zip(&work.work) {
            *x += c;
        }
        total += columns;
        tally.passes += 1;
        counters.linear_iterations += columns as u64;
    }
    if options.recompute_final {
        tally.residual_jvps += 1;
        let residual_norm = stage_true_residual(
            op.jacobian,
            op.inv,
            b,
            &work.x,
            &mut work.res,
            counters,
            ApplyCategory::Diagnostic,
        )?;
        if !residual_norm.is_finite() || residual_norm > tau {
            return Err(CoreError::LinearSolve(format!(
                "per-stage GMRES true residual {residual_norm:.3e} exceeds {tau:.3e}"
            )));
        }
    }
    b.copy_from_slice(&work.x);
    Ok(())
}

/// The shared kernel and its buffers.
struct SharedState {
    basis: SharedBasis,
    options: SharedBasisOptions,
    /// The kernel's solution (the preconditioned variable under Jacobi).
    x: Vec<f64>,
    weights: Vec<f64>,
    /// Scratch of the Jacobi-wrapped JVP.
    pv: Vec<f64>,
    /// A linearization happened since the basis was last reset.
    stale: bool,
    /// Dimension after the last solve of the current attempt.
    last_dim: u64,
}

/// One attempt's stage solve through the shared basis: `Ok(true)` when `b`
/// holds an accepted solution, `Ok(false)` when the caller must fall back.
fn shared_solve(
    shared: &mut SharedState,
    residual: &mut [f64],
    op: &StageOperator<'_>,
    tau: f64,
    b: &mut [f64],
    counters: &mut WorkCounters,
    tally: &mut SolveTally,
) -> CoreResult<bool> {
    let SharedState {
        basis,
        options,
        x,
        weights,
        pv,
        ..
    } = shared;
    let n = b.len();
    // Without a preconditioner the kernel is called as the merged hook
    // calls it: shift `h gamma`, operator `J`. Under Jacobi it gets the
    // operator `W D^-1` as `I - J_k` with `J_k v = v - W D^-1 v` and the
    // unit shift, which avoids the cancellation of two terms of size
    // `1/(h gamma)`.
    let kernel_h_gamma = if op.inv_diag.is_some() {
        1.0
    } else {
        1.0 / op.inv
    };
    let mut target = tau;
    for _ in 0..=options.confirmation_retries {
        // `h_gamma ||r||_w <= 1` with these uniform weights is
        // `||r||_2 <= target`.
        let weight = (n as f64).sqrt() / (kernel_h_gamma * target);
        if !(weight.is_finite() && weight > 0.0) {
            return Ok(false);
        }
        weights.fill(weight);
        let stats = {
            let mut jvp = |v: &[f64], out: &mut [f64]| -> CoreResult<()> {
                match op.inv_diag {
                    None => apply_counted(op.jacobian, v, out, counters, ApplyCategory::Krylov),
                    Some(inv_diag) => {
                        for ((p, vi), d) in pv.iter_mut().zip(v).zip(inv_diag) {
                            *p = vi * d;
                        }
                        counters.preconditioner_apps += 1;
                        counters.preconditioner_vectors += 1;
                        apply_counted(op.jacobian, pv, out, counters, ApplyCategory::Krylov)?;
                        for ((o, vi), p) in out.iter_mut().zip(v).zip(pv.iter()) {
                            *o = vi - (op.inv * p - *o);
                        }
                        Ok(())
                    }
                }
            };
            basis.solve_shifted(kernel_h_gamma, b, weights, &mut jvp, x)?
        };
        let new_vectors = stats.new_jvps as u64;
        tally.krylov_vectors += new_vectors;
        tally.shared_vectors += new_vectors;
        counters.linear_iterations += new_vectors;
        if !stats.converged {
            return Ok(false);
        }
        if let Some(inv_diag) = op.inv_diag {
            for (xi, d) in x.iter_mut().zip(inv_diag) {
                *xi *= d;
            }
            counters.preconditioner_apps += 1;
            counters.preconditioner_vectors += 1;
        }
        tally.residual_jvps += 1;
        let residual_norm = stage_true_residual(
            op.jacobian,
            op.inv,
            b,
            x,
            residual,
            counters,
            ApplyCategory::Krylov,
        )?;
        if residual_norm <= tau {
            b.copy_from_slice(x);
            tally.shared_accepted = true;
            return Ok(true);
        }
        tally.confirmation_failures += 1;
        target *= 0.5;
    }
    Ok(false)
}

/// A JVP-only stage client: per-stage GMRES or one shared basis per
/// attempt. See the module documentation.
pub struct KrylovStageClient {
    n: usize,
    rule: StageStoppingRule,
    gmres_options: PerStageGmresOptions,
    gmres: GmresWork,
    shared: Option<SharedState>,
    diagonal: Option<JacobianDiagonalFn>,
    /// `diag(J)` at the linearized state (Jacobi only).
    diag_j: Vec<f64>,
    /// `1 / diag(W)` of the current attempt (Jacobi only).
    inv_diag: Vec<f64>,
    jacobian: Option<Arc<JvpOperator>>,
    linearized: bool,
    factored: bool,
    inv: f64,
    /// Solves since the last `factor`.
    stage: usize,
    stats: KrylovStats,
    published: Arc<Mutex<KrylovStats>>,
    /// The step-tied tolerance, if the client was given one.
    policy: Option<StepTiedTolerance>,
    /// `min_i (atol + rtol |y_i|)` at the last linearization (policy only).
    weight_floor: f64,
}

impl KrylovStageClient {
    /// The per-stage GMRES client of order `n`.
    pub fn per_stage(
        n: usize,
        rule: StageStoppingRule,
        options: PerStageGmresOptions,
        preconditioner: KrylovPreconditioner,
    ) -> CoreResult<Self> {
        Self::build(n, rule, options, None, preconditioner)
    }

    /// The shared-basis client of order `n`; `fallback` configures the
    /// per-stage GMRES of the solves the kernel does not finish.
    pub fn shared_basis(
        n: usize,
        rule: StageStoppingRule,
        options: SharedBasisOptions,
        fallback: PerStageGmresOptions,
        preconditioner: KrylovPreconditioner,
    ) -> CoreResult<Self> {
        Self::build(n, rule, fallback, Some(options), preconditioner)
    }

    fn build(
        n: usize,
        rule: StageStoppingRule,
        gmres_options: PerStageGmresOptions,
        shared_options: Option<SharedBasisOptions>,
        preconditioner: KrylovPreconditioner,
    ) -> CoreResult<Self> {
        if n == 0 {
            return Err(CoreError::InvalidInput(
                "the Krylov stage client needs a positive order".into(),
            ));
        }
        let gmres = GmresWork::new(n, &gmres_options)?;
        let diagonal = match preconditioner {
            KrylovPreconditioner::None => None,
            KrylovPreconditioner::RightJacobi(diagonal) => Some(diagonal),
        };
        let jacobi_len = if diagonal.is_some() { n } else { 0 };
        let mut stats = KrylovStats {
            gmres_storage_bytes: gmres.storage_bytes(),
            ..KrylovStats::default()
        };
        let shared = match shared_options {
            None => None,
            Some(options) => {
                let basis = SharedBasis::new(
                    n,
                    SharedBasisConfig {
                        max_dim: options.max_dim,
                        theta: 1.0,
                    },
                )
                .map_err(CoreError::InvalidInput)?;
                // `SharedBasis::new` (shared_basis.rs): Q, Z and the scratch
                // matrix (n max_dim each), three vectors of n, one of
                // max_dim.
                let kernel_doubles = 3 * n * options.max_dim + 3 * n + options.max_dim;
                stats.shared_storage_bytes =
                    ((kernel_doubles + 2 * n + jacobi_len) * std::mem::size_of::<f64>()) as u64;
                Some(SharedState {
                    basis,
                    options,
                    x: vec![0.0; n],
                    weights: vec![0.0; n],
                    pv: vec![0.0; jacobi_len],
                    stale: true,
                    last_dim: 0,
                })
            }
        };
        Ok(Self {
            n,
            rule,
            gmres_options,
            gmres,
            shared,
            diagonal,
            diag_j: vec![0.0; jacobi_len],
            inv_diag: vec![0.0; jacobi_len],
            jacobian: None,
            linearized: false,
            factored: false,
            inv: 0.0,
            stage: 0,
            stats,
            published: Arc::new(Mutex::new(stats)),
            policy: None,
            weight_floor: 0.0,
        })
    }

    /// The same client accepting every solve at the larger of its stopping
    /// test's threshold and the bound of `policy` (loop node V17). The
    /// bound follows the state of the last linearization.
    pub fn with_step_tied_tolerance(mut self, policy: StepTiedTolerance) -> Self {
        self.policy = Some(policy);
        self
    }

    /// The step-tied tolerance, if the client has one.
    pub fn step_tied_tolerance(&self) -> Option<StepTiedTolerance> {
        self.policy
    }

    /// A reader of the statistics for after the client has been boxed and
    /// moved into the driver.
    pub fn stats_handle(&self) -> KrylovStatsHandle {
        KrylovStatsHandle {
            shared: Arc::clone(&self.published),
        }
    }

    /// The statistics so far.
    pub fn stats(&self) -> KrylovStats {
        self.snapshot()
    }

    /// The stopping test.
    pub fn rule(&self) -> StageStoppingRule {
        self.rule
    }

    /// The statistics with the basis dimension of the attempt in flight
    /// folded in.
    fn snapshot(&self) -> KrylovStats {
        let mut stats = self.stats;
        if let Some(shared) = &self.shared
            && self.stage > 0
        {
            stats.basis_dim_sum += shared.last_dim;
            stats.basis_dim_attempts += 1;
        }
        stats
    }

    fn publish(&self) {
        *self
            .published
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = self.snapshot();
    }

    fn fold(&mut self, tally: &SolveTally, failed: bool) {
        let slot = self.stage.min(KRYLOV_STAGE_SLOTS - 1);
        let stats = &mut self.stats;
        stats.solves += 1;
        stats.failed_solves += u64::from(failed);
        stats.krylov_vectors += tally.krylov_vectors;
        stats.residual_jvps += tally.residual_jvps;
        stats.restart_passes += tally.passes;
        stats.max_solve_vectors = stats.max_solve_vectors.max(tally.krylov_vectors);
        stats.zero_iterate_accepts += u64::from(tally.zero_accept);
        stats.confirmation_failures += tally.confirmation_failures;
        stats.stage_krylov_vectors[slot] += tally.krylov_vectors;
        stats.stage_solves[slot] += 1;
        stats.shared_vectors += tally.shared_vectors;
        stats.shared_accepted_solves += u64::from(tally.shared_accepted);
        stats.fallback_solves += u64::from(tally.fallback);
        stats.fallback_krylov_vectors += tally.fallback_krylov_vectors;
        stats.fallback_residual_jvps += tally.fallback_residual_jvps;
        stats.policy_bound_solves += u64::from(tally.policy_bound);
        if let Some(shared) = &mut self.shared {
            shared.last_dim = shared.basis.dim() as u64;
            stats.basis_dim_max = stats.basis_dim_max.max(shared.last_dim);
        }
        self.stage += 1;
    }

    fn solve(
        &mut self,
        b: &mut [f64],
        counters: &mut WorkCounters,
        tally: &mut SolveTally,
    ) -> CoreResult<()> {
        if b.len() != self.n {
            return Err(CoreError::Dimension(
                "Krylov stage solve shape mismatch".into(),
            ));
        }
        let Some(jacobian) = self.jacobian.as_deref().filter(|_| self.factored) else {
            return Err(CoreError::LinearSolve(
                "the Krylov stage client has no linearization or shift".into(),
            ));
        };
        let op = StageOperator {
            jacobian,
            inv: self.inv,
            inv_diag: self.diagonal.as_ref().map(|_| self.inv_diag.as_slice()),
        };
        let b_norm = safe_l2(b);
        let mut tau = self.rule.threshold(self.inv, b_norm)?;
        if let Some(policy) = &self.policy {
            let bound = policy.bound(self.n, self.weight_floor, self.inv)?;
            if bound > tau {
                tau = bound;
                tally.policy_bound = true;
            }
        }
        if let Some(shared) = self.shared.as_mut() {
            // The zero iterate is accepted on the right-hand side itself,
            // as the per-stage loop does before its first pass.
            if b_norm <= tau {
                b.fill(0.0);
                tally.zero_accept = true;
                return Ok(());
            }
            if shared_solve(shared, &mut self.gmres.res, &op, tau, b, counters, tally)? {
                return Ok(());
            }
            tally.fallback = true;
            let before = *tally;
            let outcome = gmres_solve(
                &mut self.gmres,
                &op,
                tau,
                b_norm,
                b,
                self.gmres_options,
                counters,
                tally,
            );
            tally.fallback_krylov_vectors = tally.krylov_vectors - before.krylov_vectors;
            tally.fallback_residual_jvps = tally.residual_jvps - before.residual_jvps;
            return outcome;
        }
        let before_passes = tally.passes;
        gmres_solve(
            &mut self.gmres,
            &op,
            tau,
            b_norm,
            b,
            self.gmres_options,
            counters,
            tally,
        )?;
        tally.zero_accept = tally.passes == before_passes;
        Ok(())
    }
}

impl StageLinearSystem for KrylovStageClient {
    fn dimension(&self) -> usize {
        self.n
    }

    /// The JVP operator at `(t, y)` (the problem must supply a JVP) and,
    /// under Jacobi, the diagonal of `J`. No JVP is evaluated here.
    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.linearized = false;
        self.factored = false;
        if problem.dimension != self.n {
            return Err(CoreError::Dimension(
                "Krylov stage client and problem dimensions differ".into(),
            ));
        }
        // The returned handle is dropped at once: the stored operator stays
        // uniquely owned, so the next linearization updates it in place.
        drop(problem.linearize_matrix_free_into(t, y, &mut self.jacobian)?);
        if let Some(diagonal) = &self.diagonal {
            diagonal(t, y, &mut self.diag_j)?;
            if !self.diag_j.iter().all(|value| value.is_finite()) {
                return Err(CoreError::NonFinite(
                    "Jacobian diagonal contains NaN/Inf".into(),
                ));
            }
        }
        if let Some(shared) = self.shared.as_mut() {
            shared.stale = true;
        }
        if let Some(policy) = &self.policy {
            self.weight_floor = policy.weight_floor(y)?;
        }
        self.linearized = true;
        self.stats.linearizations += 1;
        self.publish();
        Ok(())
    }

    /// Stores the shift, forms `1/diag(W)` under Jacobi and resets the
    /// shared basis (unless it is kept across a rejection). Charges
    /// nothing: there is no factorization.
    fn factor(&mut self, inv_h_gamma: f64, _counters: &mut WorkCounters) -> CoreResult<()> {
        self.factored = false;
        if !(inv_h_gamma.is_finite() && inv_h_gamma > 0.0) {
            return Err(CoreError::InvalidInput(
                "the Krylov stage client needs a finite positive 1/(h gamma)".into(),
            ));
        }
        if !self.linearized {
            return Err(CoreError::LinearSolve(
                "the Krylov stage client has no linearization".into(),
            ));
        }
        if self.diagonal.is_some() {
            for (inverse, dj) in self.inv_diag.iter_mut().zip(&self.diag_j) {
                let diagonal = inv_h_gamma - dj;
                if diagonal == 0.0 || !diagonal.is_finite() {
                    return Err(CoreError::LinearSolve(
                        "Jacobi preconditioner has a zero diagonal".into(),
                    ));
                }
                *inverse = 1.0 / diagonal;
            }
        }
        let had_solves = self.stage > 0;
        if let Some(shared) = self.shared.as_mut() {
            if had_solves {
                self.stats.basis_dim_sum += shared.last_dim;
                self.stats.basis_dim_attempts += 1;
            }
            let keep = shared.options.keep_across_rejection
                && self.diagonal.is_none()
                && !shared.stale
                && shared.basis.dim() > 0;
            if keep {
                self.stats.basis_kept += 1;
            } else {
                shared.basis.reset();
            }
            shared.stale = false;
            shared.last_dim = shared.basis.dim() as u64;
        }
        self.inv = inv_h_gamma;
        self.stage = 0;
        self.factored = true;
        self.stats.attempts += 1;
        self.publish();
        Ok(())
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        let mut tally = SolveTally::default();
        let outcome = self.solve(b, counters, &mut tally);
        self.fold(&tally, outcome.is_err());
        self.publish();
        outcome
    }
}

/// What a [`ResidualAuditClient`] found.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResidualAuditReport {
    /// Solves checked (every solve the inner client returned `Ok` for).
    pub checks: u64,
    /// Checks whose residual exceeded the threshold, was not finite or
    /// could not be formed.
    pub failures: u64,
    /// Largest `||b - W x|| / threshold` over the checks with a positive
    /// threshold (0 before the first).
    pub max_ratio: f64,
    /// Checks with a zero threshold and a nonzero residual.
    pub zero_threshold_failures: u64,
    /// 1-based index of the first failed check (0: none).
    pub first_failure: u64,
    /// JVPs of the audit itself; never part of the run's counters.
    pub jvp_calls: u64,
}

/// A reader of a [`ResidualAuditClient`]'s report that outlives its move
/// into the driver.
#[derive(Clone, Debug)]
pub struct ResidualAuditHandle {
    shared: Arc<Mutex<ResidualAuditReport>>,
}

impl ResidualAuditHandle {
    /// The report published so far.
    pub fn snapshot(&self) -> ResidualAuditReport {
        *self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Checks the original residual of every stage solve of an inner client.
///
/// Before a solve it saves `b`; after a successful solve it forms
/// `b - W x` with its own JVP operator (linearized by itself at the same
/// state) and its own [`WorkCounters`], and tests the 2-norm against
/// [`StageStoppingRule::threshold`] of the saved `b`. The inner client's
/// arguments, results and counters are untouched, so a run with the audit
/// is the run without it, bit for bit. A failed check is recorded, not
/// raised. The problem must supply a JVP.
pub struct ResidualAuditClient<C> {
    inner: C,
    rule: StageStoppingRule,
    jacobian: Option<Arc<JvpOperator>>,
    inv: f64,
    saved: Vec<f64>,
    residual: Vec<f64>,
    counters: WorkCounters,
    report: ResidualAuditReport,
    published: Arc<Mutex<ResidualAuditReport>>,
    /// The step-tied tolerance of the audited arm, if it has one.
    policy: Option<StepTiedTolerance>,
    /// The audit's own `min_i (atol + rtol |y_i|)` at the last
    /// linearization (policy only); NaN when it could not be formed, which
    /// fails every check until the next linearization.
    weight_floor: f64,
}

impl<C: StageLinearSystem> ResidualAuditClient<C> {
    /// Wrap `inner`, checking against `rule`.
    pub fn new(inner: C, rule: StageStoppingRule) -> Self {
        let n = inner.dimension();
        Self {
            inner,
            rule,
            jacobian: None,
            inv: 0.0,
            saved: vec![0.0; n],
            residual: vec![0.0; n],
            counters: WorkCounters::default(),
            report: ResidualAuditReport::default(),
            published: Arc::new(Mutex::new(ResidualAuditReport::default())),
            policy: None,
            weight_floor: 0.0,
        }
    }

    /// The same audit checking against the larger of `rule`'s threshold and
    /// the bound of `policy` (loop node V17), with the weight floor the
    /// audit computes itself from the state it is linearized at.
    pub fn with_step_tied_tolerance(mut self, policy: StepTiedTolerance) -> Self {
        self.policy = Some(policy);
        self
    }

    /// A reader of the report for after the wrapper has been boxed.
    pub fn handle(&self) -> ResidualAuditHandle {
        ResidualAuditHandle {
            shared: Arc::clone(&self.published),
        }
    }

    /// The report so far.
    pub fn report(&self) -> ResidualAuditReport {
        self.report
    }

    /// The wrapped client.
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// The residual norm of `x` against the saved right-hand side and the
    /// threshold of that right-hand side.
    fn residual_and_threshold(&mut self, x: &[f64]) -> CoreResult<(f64, f64)> {
        let jacobian = self.jacobian.as_deref().ok_or_else(|| {
            CoreError::LinearSolve("the residual audit has no linearization".into())
        })?;
        let residual_norm = stage_true_residual(
            jacobian,
            self.inv,
            &self.saved,
            x,
            &mut self.residual,
            &mut self.counters,
            ApplyCategory::Diagnostic,
        )?;
        let mut tau = self.rule.threshold(self.inv, safe_l2(&self.saved))?;
        if let Some(policy) = &self.policy {
            let bound = policy.bound(self.saved.len(), self.weight_floor, self.inv)?;
            if bound > tau {
                tau = bound;
            }
        }
        Ok((residual_norm, tau))
    }

    fn check(&mut self, x: &[f64]) {
        self.report.checks += 1;
        let verdict = self.residual_and_threshold(x);
        self.report.jvp_calls = self.counters.jvp_calls;
        let passed = match verdict {
            Ok((residual_norm, tau)) => {
                if tau > 0.0 {
                    let ratio = residual_norm / tau;
                    if ratio.is_finite() {
                        self.report.max_ratio = self.report.max_ratio.max(ratio);
                    }
                } else if residual_norm != 0.0 {
                    self.report.zero_threshold_failures += 1;
                }
                residual_norm <= tau
            }
            Err(_) => false,
        };
        if !passed {
            self.report.failures += 1;
            if self.report.first_failure == 0 {
                self.report.first_failure = self.report.checks;
            }
        }
        *self
            .published
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = self.report;
    }
}

impl<C: StageLinearSystem> StageLinearSystem for ResidualAuditClient<C> {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.inner.linearize(problem, t, y, counters)?;
        drop(problem.linearize_matrix_free_into(t, y, &mut self.jacobian)?);
        if let Some(policy) = &self.policy {
            self.weight_floor = policy.weight_floor(y).unwrap_or(f64::NAN);
        }
        Ok(())
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        self.inner.factor(inv_h_gamma, counters)?;
        self.inv = inv_h_gamma;
        Ok(())
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if b.len() == self.saved.len() {
            self.saved.copy_from_slice(b);
        }
        self.inner.solve_in_place(b, counters)?;
        if b.len() == self.saved.len() {
            self.check(b);
        }
        Ok(())
    }
}
