//! Staged GMRES: restarted GMRES with an in-cycle projected exit (research
//! nodes `research/alg01_coupled_stage_target_20261008`, ALG01,
//! `research/alg03_stage_budget_guard_20261008`, ALG03, and their second
//! tests `research/alg04_coupled_target_v2_20261010`, ALG04, and
//! `research/alg06_guard_v2_20261010`, ALG06). Research only; no existing
//! solver calls it.
//!
//! Zero start, no preconditioner, two-pass modified Gram-Schmidt
//! ([`crate::kernels::two_pass_mgs_into`], counted as in production) and an
//! incremental Givens QR of the Hessenberg matrix, so the projected residual
//! `|g_(j+1)|` of every column is available without a least-squares solve.
//!
//! * **Exit and confirmation.** When the projected residual is at most the
//!   threshold `max(atol, rtol ||b||)`, the iterate `x + V_j y_j` is formed
//!   and one true residual `b - A x` is computed (a counted Krylov matvec).
//!   If it meets the threshold the solve ends there; no further diagnostic
//!   residual is computed. If it does not, the cycle continues and the next
//!   confirmation waits `gap` more columns, `gap` doubling after every failed
//!   confirmation of the solve (it starts at 1 and is not reset at a restart;
//!   each new cycle may confirm from its first column). A cycle that ends
//!   right after a failed confirmation restarts from that confirmed residual,
//!   so the same residual is never computed twice.
//! * **Restart boundary.** After a full cycle the iterate and its true
//!   residual are formed (one counted matvec) and tested, in this order: the
//!   threshold; the optional stall rule; the optional stagnation guard; the
//!   column budget.
//! * **Stall rule** (optional). After a cycle that cut the true residual by
//!   less than 4x, accept when
//!   `||r|| <= 1024 eps (||b|| + ||x|| + ||x - A x||)`.
//! * **Nonnormality guard** (optional). Whenever the projected test would
//!   pass, `nu = 1 / sigma_min(R_j)` of the Givens triangular factor is
//!   computed (a dense singular-value decomposition of the `j x j` factor
//!   through faer, charged in the report as `4 j^3` flops, not in the work
//!   counters). The running maximum `nu_hat` (starting at 1) divides the
//!   threshold; the projected test is repeated with the tightened threshold
//!   and the confirmation must meet it too. This follows the pilot
//!   (`pilot/phase3_code/critic/stack.py`): the first evaluation is at the
//!   first projected exit, later ones only where a projected exit would
//!   otherwise be taken.
//! * **Stagnation guard** (optional, ALG03). At a restart boundary with
//!   `q = ||r_k|| / ||r_(k-1)||`, abort when `q >= 0.98` or when the columns
//!   used plus `restart * ceil(log(thr / ||r_k||) / log q)` exceed the budget.
//! * **Fallback** (optional caller closure). On a guard abort or when the
//!   budget is exhausted, the closure receives the iterate and its true
//!   residual and may accept the iterate.
//! * **Shadow classification** (optional, ALG03 reporting). After a guard
//!   abort that the fallback does not accept, the solve continues without the
//!   guard on scratch counters until it would converge (a false abort) or
//!   exhausts the budget (a true abort). Nothing of the continuation is
//!   counted, returned or written; the solve still fails. With
//!   `classify_accepted_guard_aborts` (ALG06 reporting) the aborts the
//!   fallback accepted are classified the same way, after the accepted
//!   iterate was written to the output.
//! * **Small-system exhaustion** (optional, ALG04). When the dimension is at
//!   most `restart`, an in-cycle confirmation is taken only at a column that
//!   ends the Krylov space: a happy breakdown, the round-off floor
//!   `16 eps ||rhs||` of the projected residual, or the cycle's last column.
//!   Every other part of the column test is unchanged, the nonnormality
//!   guard included: `nu` is still evaluated at every column whose projected
//!   residual meets the (tightened) threshold outside the confirmation gap
//!   (`used >= next_check`), keeping the running maximum, exactly as in
//!   ALG01; only the confirmation waits for the end of the space.
//! * **Effective cycle length** (optional, ALG06 `G1`). The overrun
//!   prediction uses `m_eff = min(restart, n, budget - used)` in place of
//!   `restart`.
//! * **Attainable-accuracy floor at every confirmation** (optional, ALG06
//!   `G2`). Every true residual (an in-cycle confirmation or a restart
//!   boundary) that misses the threshold is accepted when
//!   `||r|| <= 1024 eps (||rhs|| + ||x|| + ||x - A x||)` (the stall rule's
//!   floor, without its slow-cycle condition). At a restart boundary the
//!   stall rule is tested first, so the floor labels only the acceptances
//!   the stall rule would not make.
//!
//! Counters are charged as in production GMRES: every operator application
//! through [`apply_counted`] (Krylov category, so the operator's own work,
//! for example one JVP, is charged), the orthogonalization through the
//! production kernels, one vector update for every iterate formed from the
//! basis, `linear_iterations` per Arnoldi column and `linear_solves` for an
//! accepted solve.

use crate::{
    common::{residual_threshold, validate_tolerances},
    gmres::arnoldi_happy_breakdown,
    kernels::{axpy, two_pass_mgs_into},
    workspace::{ensure_len, ensure_pool},
};
use faer::Mat;
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, LinearOperator, WorkCounters, apply_counted, safe_l2,
};

/// A restart cycle that leaves more than this fraction of the true residual
/// is a stall candidate ("cuts the residual by less than 4x").
pub const STAGED_STALL_CONTRACTION: f64 = 0.25;
/// The stall rule's backward-error factor, `1024 eps`.
pub const STAGED_STALL_FACTOR: f64 = 1024.0 * f64::EPSILON;
/// The stagnation guard's restart contraction limit.
pub const STAGED_GUARD_Q_ABORT: f64 = 0.98;
/// The small-system exhaustion's round-off floor of the projected residual,
/// relative to `||rhs||_2` (ALG04).
pub const STAGED_EXHAUSTION_FLOOR: f64 = 16.0 * f64::EPSILON;

/// Configuration of [`solve_staged_gmres`].
#[derive(Clone, Debug, PartialEq)]
pub struct StagedGmresConfig {
    /// Arnoldi columns per cycle (capped by the dimension and the budget).
    pub restart: usize,
    /// Column budget of the solve.
    pub max_columns: usize,
    /// Threshold `max(atol, rtol ||b||_2)` on the true residual.
    pub rtol: f64,
    pub atol: f64,
    pub stall_rule: bool,
    pub nu_guard: bool,
    pub stagnation_guard: bool,
    /// Classify guard aborts by an uncounted shadow continuation.
    pub classify_guard_aborts: bool,
    /// Small-system exhaustion (ALG04): with `n <= restart`, confirm only at
    /// the end of the Krylov space.
    pub small_system_exhaustion: bool,
    /// ALG06 `G1`: the overrun prediction uses the effective cycle length.
    pub effective_cycle_overrun: bool,
    /// ALG06 `G2`: accept the attainable-accuracy floor at every true
    /// residual.
    pub floor_at_confirmations: bool,
    /// ALG06 reporting: also classify the guard aborts the fallback
    /// accepted (needs `classify_guard_aborts`).
    pub classify_accepted_guard_aborts: bool,
}

impl StagedGmresConfig {
    /// The plain staged solver: no stall rule, no guards.
    pub fn new(restart: usize, max_columns: usize, rtol: f64, atol: f64) -> Self {
        Self {
            restart,
            max_columns,
            rtol,
            atol,
            stall_rule: false,
            nu_guard: false,
            stagnation_guard: false,
            classify_guard_aborts: false,
            small_system_exhaustion: false,
            effective_cycle_overrun: false,
            floor_at_confirmations: false,
            classify_accepted_guard_aborts: false,
        }
    }

    pub fn validate(&self) -> CoreResult<()> {
        if self.restart == 0 || self.max_columns == 0 {
            return Err(CoreError::InvalidInput(
                "staged GMRES iteration limits must be positive".into(),
            ));
        }
        validate_tolerances("staged GMRES", self.rtol, self.atol)
    }
}

/// How a staged solve ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StagedGmresOutcome {
    /// The true residual met the threshold.
    Converged,
    /// Accepted by the stall rule.
    StallAccepted,
    /// Accepted by the caller's fallback after a guard abort or with the
    /// budget exhausted.
    FallbackAccepted,
    /// Accepted by the attainable-accuracy floor (`G2`) where neither the
    /// threshold nor the stall rule accepted.
    FloorAccepted,
    /// Not accepted; see [`StagedGmresReport::failure`].
    Failed,
}

/// Why a staged solve failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StagedGmresFailure {
    BudgetExhausted,
    GuardContraction,
    GuardOverrun,
    /// A non-finite true residual.
    NonFinite,
    /// An exactly zero Arnoldi column before any progress.
    Breakdown,
}

/// Which guard test aborted the solve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StagedGuardAbort {
    /// `q >= 0.98`.
    Contraction,
    /// The geometric prediction exceeds the budget.
    Overrun,
}

/// The report of [`solve_staged_gmres`]. The solution is in the caller's
/// output, written only when the outcome is an acceptance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StagedGmresReport {
    pub outcome: StagedGmresOutcome,
    pub failure: Option<StagedGmresFailure>,
    /// Set when the stagnation guard fired (the solve then either was
    /// accepted by the fallback or failed).
    pub guard_abort: Option<StagedGuardAbort>,
    /// With classification on, after a guard abort the fallback did not
    /// accept (or, with `classify_accepted_guard_aborts`, any guard abort):
    /// whether the uncounted continuation converged within the budget (a
    /// false abort).
    pub shadow_converged: Option<bool>,
    /// Small-system exhaustion was active (`n <= restart`).
    pub exhaustion: bool,
    /// The column budget was reached at a restart boundary.
    pub budget_exhausted: bool,
    pub right_norm: f64,
    /// `max(atol, rtol ||b||)`.
    pub threshold: f64,
    /// The threshold after the nonnormality guard (equal to `threshold`
    /// when the guard is off or never tightened it).
    pub final_threshold: f64,
    /// The true residual norm of the returned (or last) iterate.
    pub residual_norm: f64,
    /// The projected residual at an in-cycle exit (`NaN` otherwise).
    pub projected_residual: f64,
    pub columns: u64,
    pub cycles: u64,
    /// True residuals computed (restart boundaries and confirmations).
    pub true_residuals: u64,
    pub confirmations: u64,
    pub failed_confirmations: u64,
    pub nu_evaluations: u64,
    /// Largest `nu` found (1 when the guard is off or never exceeded 1).
    pub nu_max: f64,
    /// `4 j^3` per singular-value evaluation of a `j x j` factor.
    pub nu_flops: u64,
    /// Krylov-category operator applications of this solve.
    pub matvecs: u64,
}

impl StagedGmresReport {
    pub fn accepted(&self) -> bool {
        self.outcome != StagedGmresOutcome::Failed
    }
}

/// A fallback test: `(iterate, true residual) -> accept`.
pub type StagedGmresFallback<'a> = &'a mut dyn FnMut(&[f64], &[f64]) -> bool;

/// Reused storage of [`solve_staged_gmres`].
#[derive(Clone, Debug, Default)]
pub struct StagedGmresWorkspace {
    basis: Vec<Vec<f64>>,
    w: Vec<f64>,
    h_column: Vec<f64>,
    /// Givens triangular factor, column-major with leading dimension
    /// `columns + 1`.
    r: Vec<f64>,
    cs: Vec<f64>,
    sn: Vec<f64>,
    g: Vec<f64>,
    y: Vec<f64>,
    x: Vec<f64>,
    residual: Vec<f64>,
    operator_output: Vec<f64>,
    candidate: Vec<f64>,
    candidate_residual: Vec<f64>,
    candidate_output: Vec<f64>,
    correction: Vec<f64>,
}

impl StagedGmresWorkspace {
    fn prepare(&mut self, n: usize, columns: usize) {
        ensure_pool(&mut self.basis, columns + 1, n);
        for v in [
            &mut self.w,
            &mut self.x,
            &mut self.residual,
            &mut self.operator_output,
            &mut self.candidate,
            &mut self.candidate_residual,
            &mut self.candidate_output,
            &mut self.correction,
        ] {
            ensure_len(v, n);
        }
        ensure_len(&mut self.h_column, columns + 1);
        ensure_len(&mut self.r, (columns + 1) * columns);
        ensure_len(&mut self.cs, columns);
        ensure_len(&mut self.sn, columns);
        ensure_len(&mut self.g, columns + 1);
        ensure_len(&mut self.y, columns);
    }
}

/// The state carried across restart boundaries (cloned for the shadow
/// continuation).
#[derive(Clone, Copy, Debug)]
struct Progress {
    threshold: f64,
    thr: f64,
    nu_hat: f64,
    rn: f64,
    rn_prev: Option<f64>,
    /// `operator_output` holds `A x` of the current iterate.
    has_image: bool,
    total: usize,
    gap: usize,
    cycles: u64,
    true_residuals: u64,
    confirmations: u64,
    failed_confirmations: u64,
    nu_evaluations: u64,
    nu_flops: u64,
    nu_max: f64,
    projected: f64,
}

enum End {
    Converged,
    Stall,
    Floor,
    Guard(StagedGuardAbort),
    Budget,
    NonFinite,
    Breakdown,
}

/// Column `j` of the triangular factor, row `i`.
fn r_index(columns: usize, i: usize, j: usize) -> usize {
    j * (columns + 1) + i
}

/// Smallest singular value of the leading `k x k` upper triangle of `r`.
fn sigma_min(r: &[f64], columns: usize, k: usize) -> CoreResult<f64> {
    let m = Mat::from_fn(k, k, |i, j| {
        if i <= j {
            r[r_index(columns, i, j)]
        } else {
            0.0
        }
    });
    let values = m
        .singular_values()
        .map_err(|e| CoreError::LinearSolve(format!("staged GMRES nu-guard SVD failed: {e:?}")))?;
    Ok(values.last().copied().unwrap_or(0.0))
}

/// `correction = V_k y_k` with `R_k y_k = g_k` (back substitution).
fn correction_into(ws: &mut StagedGmresWorkspace, columns: usize, k: usize) {
    for i in (0..k).rev() {
        let mut s = ws.g[i];
        for l in (i + 1)..k {
            s -= ws.r[r_index(columns, i, l)] * ws.y[l];
        }
        ws.y[i] = s / ws.r[r_index(columns, i, i)];
    }
    ws.correction.fill(0.0);
    for (v, &c) in ws.basis[..k].iter().zip(&ws.y[..k]) {
        for (o, vi) in ws.correction.iter_mut().zip(v) {
            *o += c * vi;
        }
    }
}

/// `1024 eps (||rhs|| + ||x|| + ||x - A x||)` (`scratch` receives
/// `x - A x`).
fn attainable_floor(right_norm: f64, x: &[f64], image: &[f64], scratch: &mut [f64]) -> f64 {
    let x_norm = safe_l2(x);
    for ((c, xi), a) in scratch.iter_mut().zip(x).zip(image) {
        *c = xi - a;
    }
    STAGED_STALL_FACTOR * (right_norm + x_norm + safe_l2(scratch))
}

/// The stagnation guard at a restart boundary: `q = rn / prev`, abort when
/// `q >= 0.98` or when `total + cycle * ceil(log(thr / rn) / log q)`
/// exceeds `max_columns` (`cycle` is `restart`, or `m_eff` with `G1`).
pub fn staged_guard_test(
    rn: f64,
    prev: f64,
    thr: f64,
    total: usize,
    cycle: usize,
    max_columns: usize,
) -> Option<StagedGuardAbort> {
    let q = rn / prev;
    if q >= STAGED_GUARD_Q_ABORT {
        return Some(StagedGuardAbort::Contraction);
    }
    let need = ((thr / rn).ln() / q.ln()).ceil();
    if total as f64 + cycle as f64 * need > max_columns as f64 {
        return Some(StagedGuardAbort::Overrun);
    }
    None
}

/// `residual = rhs - output`, with `output = A x` applied and counted.
fn true_residual(
    op: &dyn LinearOperator,
    rhs: &[f64],
    x: &[f64],
    output: &mut [f64],
    residual: &mut [f64],
    counters: &mut WorkCounters,
) -> CoreResult<f64> {
    apply_counted(op, x, output, counters, ApplyCategory::Krylov)?;
    for ((r, b), a) in residual.iter_mut().zip(rhs).zip(output.iter()) {
        *r = b - a;
    }
    Ok(safe_l2(residual))
}

/// Restart cycles from the state in `ws` / `p` until an end.
#[allow(clippy::too_many_arguments)]
fn iterate(
    op: &dyn LinearOperator,
    rhs: &[f64],
    config: &StagedGmresConfig,
    guard: bool,
    right_norm: f64,
    p: &mut Progress,
    ws: &mut StagedGmresWorkspace,
    columns: usize,
    counters: &mut WorkCounters,
) -> CoreResult<End> {
    let exhaust = exhaustion_active(config, rhs.len());
    let exhaustion_floor = STAGED_EXHAUSTION_FLOOR * right_norm;
    loop {
        let rn = p.rn;
        if !rn.is_finite() {
            return Ok(End::NonFinite);
        }
        if rn <= p.thr {
            return Ok(End::Converged);
        }
        if let (true, true, Some(prev)) = (config.stall_rule, p.has_image, p.rn_prev)
            && rn > STAGED_STALL_CONTRACTION * prev
        {
            let x_norm = safe_l2(&ws.x);
            for ((c, x), a) in ws.candidate.iter_mut().zip(&ws.x).zip(&ws.operator_output) {
                *c = x - a;
            }
            let defect = safe_l2(&ws.candidate);
            if rn <= STAGED_STALL_FACTOR * (right_norm + x_norm + defect) {
                return Ok(End::Stall);
            }
        }
        if config.floor_at_confirmations
            && p.has_image
            && rn <= attainable_floor(right_norm, &ws.x, &ws.operator_output, &mut ws.candidate)
        {
            return Ok(End::Floor);
        }
        if let (true, true, Some(prev)) = (guard, p.has_image, p.rn_prev) {
            let cycle = if config.effective_cycle_overrun {
                columns.min(config.max_columns - p.total)
            } else {
                config.restart
            };
            if let Some(abort) =
                staged_guard_test(rn, prev, p.thr, p.total, cycle, config.max_columns)
            {
                return Ok(End::Guard(abort));
            }
        }
        p.rn_prev = Some(rn);
        if p.total >= config.max_columns {
            return Ok(End::Budget);
        }
        let steps = columns.min(config.max_columns - p.total);
        for (b, r) in ws.basis[0].iter_mut().zip(&ws.residual) {
            *b = r / rn;
        }
        ws.g[..=steps].fill(0.0);
        ws.g[0] = rn;
        let mut used = 0usize;
        let mut next_check = 0usize;
        let mut candidate_current = false;
        let mut candidate_norm = f64::NAN;
        for j in 0..steps {
            apply_counted(op, &ws.basis[j], &mut ws.w, counters, ApplyCategory::Krylov)?;
            two_pass_mgs_into(&mut ws.w, &ws.basis[..=j], &mut ws.h_column, counters)?;
            let h_next = safe_l2(&ws.w);
            let breakdown = arnoldi_happy_breakdown(&ws.h_column[..=j], h_next)?;
            for i in 0..=j {
                ws.r[r_index(columns, i, j)] = ws.h_column[i];
            }
            for i in 0..j {
                let (a, b) = (
                    ws.r[r_index(columns, i, j)],
                    ws.r[r_index(columns, i + 1, j)],
                );
                ws.r[r_index(columns, i, j)] = ws.cs[i] * a + ws.sn[i] * b;
                ws.r[r_index(columns, i + 1, j)] = -ws.sn[i] * a + ws.cs[i] * b;
            }
            let diagonal = ws.r[r_index(columns, j, j)];
            let den = diagonal.hypot(h_next);
            if den == 0.0 || !den.is_finite() {
                // A zero (or non-finite) column adds nothing to the cycle.
                break;
            }
            ws.cs[j] = diagonal / den;
            ws.sn[j] = h_next / den;
            ws.r[r_index(columns, j, j)] = den;
            ws.g[j + 1] = -ws.sn[j] * ws.g[j];
            ws.g[j] *= ws.cs[j];
            used = j + 1;
            p.total += 1;
            candidate_current = false;
            let projected = ws.g[j + 1].abs();
            if projected <= p.thr && used >= next_check {
                if config.nu_guard {
                    let smin = sigma_min(&ws.r, columns, used)?;
                    p.nu_evaluations += 1;
                    p.nu_flops += 4 * (used as u64).pow(3);
                    if smin > 0.0 && 1.0 / smin > p.nu_hat {
                        p.nu_hat = 1.0 / smin;
                        p.thr = p.threshold / p.nu_hat;
                        p.nu_max = p.nu_max.max(p.nu_hat);
                    }
                }
                let end_of_space =
                    !exhaust || breakdown || projected <= exhaustion_floor || used == steps;
                if projected <= p.thr && end_of_space {
                    correction_into(ws, columns, used);
                    ws.candidate.copy_from_slice(&ws.x);
                    axpy(1.0, &ws.correction, &mut ws.candidate, counters)?;
                    let rc = true_residual(
                        op,
                        rhs,
                        &ws.candidate,
                        &mut ws.candidate_output,
                        &mut ws.candidate_residual,
                        counters,
                    )?;
                    p.true_residuals += 1;
                    p.confirmations += 1;
                    if rc.is_finite() && rc <= p.thr {
                        std::mem::swap(&mut ws.x, &mut ws.candidate);
                        std::mem::swap(&mut ws.residual, &mut ws.candidate_residual);
                        std::mem::swap(&mut ws.operator_output, &mut ws.candidate_output);
                        p.rn = rc;
                        p.has_image = true;
                        p.projected = projected;
                        p.cycles += 1;
                        counters.linear_iterations += used as u64;
                        return Ok(End::Converged);
                    }
                    if config.floor_at_confirmations
                        && rc.is_finite()
                        && rc
                            <= attainable_floor(
                                right_norm,
                                &ws.candidate,
                                &ws.candidate_output,
                                &mut ws.correction,
                            )
                    {
                        std::mem::swap(&mut ws.x, &mut ws.candidate);
                        std::mem::swap(&mut ws.residual, &mut ws.candidate_residual);
                        std::mem::swap(&mut ws.operator_output, &mut ws.candidate_output);
                        p.rn = rc;
                        p.has_image = true;
                        p.projected = projected;
                        p.cycles += 1;
                        counters.linear_iterations += used as u64;
                        return Ok(End::Floor);
                    }
                    p.failed_confirmations += 1;
                    p.gap = p.gap.saturating_mul(2);
                    next_check = used + p.gap;
                    candidate_current = true;
                    candidate_norm = rc;
                }
            }
            if breakdown {
                break;
            }
            if j + 1 < steps {
                for (b, w) in ws.basis[j + 1].iter_mut().zip(&ws.w) {
                    *b = w / h_next;
                }
            }
        }
        counters.linear_iterations += used as u64;
        p.cycles += 1;
        if used == 0 {
            return Ok(End::Breakdown);
        }
        if candidate_current {
            std::mem::swap(&mut ws.x, &mut ws.candidate);
            std::mem::swap(&mut ws.residual, &mut ws.candidate_residual);
            std::mem::swap(&mut ws.operator_output, &mut ws.candidate_output);
            p.rn = candidate_norm;
        } else {
            correction_into(ws, columns, used);
            axpy(1.0, &ws.correction, &mut ws.x, counters)?;
            p.rn = true_residual(
                op,
                rhs,
                &ws.x,
                &mut ws.operator_output,
                &mut ws.residual,
                counters,
            )?;
            p.true_residuals += 1;
        }
        p.has_image = true;
    }
}

/// Small-system exhaustion applies (`n <= restart`).
fn exhaustion_active(config: &StagedGmresConfig, n: usize) -> bool {
    config.small_system_exhaustion && n <= config.restart
}

fn fallback_accepts(
    fallback: &mut Option<StagedGmresFallback<'_>>,
    ws: &StagedGmresWorkspace,
) -> bool {
    match fallback {
        Some(test) => test(&ws.x, &ws.residual),
        None => false,
    }
}

/// Staged GMRES on `A x = rhs` from a zero start; see the module
/// documentation. `output` is written only when the solve is accepted.
pub fn solve_staged_gmres(
    op: &dyn LinearOperator,
    rhs: &[f64],
    config: &StagedGmresConfig,
    mut fallback: Option<StagedGmresFallback<'_>>,
    output: &mut [f64],
    workspace: &mut StagedGmresWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<StagedGmresReport> {
    config.validate()?;
    let n = op.dimension();
    if rhs.len() != n || output.len() != n {
        return Err(CoreError::Dimension(
            "staged GMRES system shape mismatch".into(),
        ));
    }
    if !rhs.iter().all(|v| v.is_finite()) {
        return Err(CoreError::NonFinite(
            "staged GMRES right-hand side contains NaN/Inf".into(),
        ));
    }
    let before = *counters;
    let right_norm = safe_l2(rhs);
    let threshold = residual_threshold("staged GMRES", config.rtol, config.atol, right_norm)?;
    let columns = config.restart.min(config.max_columns).min(n.max(1));
    let ws = workspace;
    ws.prepare(n, columns);
    ws.x.fill(0.0);
    ws.residual.copy_from_slice(rhs);
    let mut p = Progress {
        threshold,
        thr: threshold,
        nu_hat: 1.0,
        rn: right_norm,
        rn_prev: None,
        has_image: false,
        total: 0,
        gap: 1,
        cycles: 0,
        true_residuals: 0,
        confirmations: 0,
        failed_confirmations: 0,
        nu_evaluations: 0,
        nu_flops: 0,
        nu_max: 1.0,
        projected: f64::NAN,
    };
    let end = iterate(
        op,
        rhs,
        config,
        config.stagnation_guard,
        right_norm,
        &mut p,
        ws,
        columns,
        counters,
    )?;
    let mut guard_abort = None;
    let mut shadow_converged = None;
    let mut budget_exhausted = false;
    let mut output_written = false;
    let (outcome, failure) = match end {
        End::Converged => (StagedGmresOutcome::Converged, None),
        End::Stall => (StagedGmresOutcome::StallAccepted, None),
        End::Floor => (StagedGmresOutcome::FloorAccepted, None),
        End::NonFinite => (
            StagedGmresOutcome::Failed,
            Some(StagedGmresFailure::NonFinite),
        ),
        End::Breakdown => (
            StagedGmresOutcome::Failed,
            Some(StagedGmresFailure::Breakdown),
        ),
        End::Budget => {
            budget_exhausted = true;
            if fallback_accepts(&mut fallback, ws) {
                (StagedGmresOutcome::FallbackAccepted, None)
            } else {
                (
                    StagedGmresOutcome::Failed,
                    Some(StagedGmresFailure::BudgetExhausted),
                )
            }
        }
        End::Guard(kind) => {
            guard_abort = Some(kind);
            // The uncounted shadow continuation without the guard; it
            // overwrites the workspace, so it runs after the output was
            // written (accepted) or on a failed solve.
            let classify = |ws: &mut StagedGmresWorkspace| {
                let mut shadow = p;
                let mut scratch = WorkCounters::default();
                let continued = iterate(
                    op,
                    rhs,
                    config,
                    false,
                    right_norm,
                    &mut shadow,
                    ws,
                    columns,
                    &mut scratch,
                );
                Some(matches!(
                    continued,
                    Ok(End::Converged | End::Stall | End::Floor)
                ))
            };
            if fallback_accepts(&mut fallback, ws) {
                if config.classify_guard_aborts && config.classify_accepted_guard_aborts {
                    output.copy_from_slice(&ws.x);
                    output_written = true;
                    shadow_converged = classify(ws);
                }
                (StagedGmresOutcome::FallbackAccepted, None)
            } else {
                if config.classify_guard_aborts {
                    shadow_converged = classify(ws);
                }
                let failure = match kind {
                    StagedGuardAbort::Contraction => StagedGmresFailure::GuardContraction,
                    StagedGuardAbort::Overrun => StagedGmresFailure::GuardOverrun,
                };
                (StagedGmresOutcome::Failed, Some(failure))
            }
        }
    };
    if outcome != StagedGmresOutcome::Failed {
        counters.linear_solves += 1;
        if !output_written {
            output.copy_from_slice(&ws.x);
        }
    }
    let delta = counters.delta(before);
    Ok(StagedGmresReport {
        outcome,
        failure,
        guard_abort,
        shadow_converged,
        exhaustion: exhaustion_active(config, n),
        budget_exhausted,
        right_norm,
        threshold,
        final_threshold: p.thr,
        residual_norm: p.rn,
        projected_residual: p.projected,
        columns: p.total as u64,
        cycles: p.cycles,
        true_residuals: p.true_residuals,
        confirmations: p.confirmations,
        failed_confirmations: p.failed_confirmations,
        nu_evaluations: p.nu_evaluations,
        nu_max: p.nu_max,
        nu_flops: p.nu_flops,
        matvecs: delta.linear_matvecs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodas5p_core::{DenseMatrix, DenseOperator};

    fn tridiagonal(n: usize, sub: f64, diag: f64, sup: f64) -> DenseMatrix {
        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..n {
            a[(i, i)] = diag;
            if i > 0 {
                a[(i, i - 1)] = sub;
            }
            if i + 1 < n {
                a[(i, i + 1)] = sup;
            }
        }
        a
    }

    fn residual_norm(a: &DenseMatrix, b: &[f64], x: &[f64]) -> f64 {
        let ax = a.matvec(x).unwrap();
        safe_l2(&b.iter().zip(&ax).map(|(p, q)| p - q).collect::<Vec<_>>())
    }

    fn solve(
        a: &DenseMatrix,
        b: &[f64],
        config: &StagedGmresConfig,
        fallback: Option<StagedGmresFallback<'_>>,
    ) -> (StagedGmresReport, Vec<f64>, WorkCounters) {
        let op = DenseOperator::new(a.clone()).unwrap();
        let mut x = vec![f64::NAN; b.len()];
        let mut counters = WorkCounters::default();
        let mut ws = StagedGmresWorkspace::default();
        let report =
            solve_staged_gmres(&op, b, config, fallback, &mut x, &mut ws, &mut counters).unwrap();
        (report, x, counters)
    }

    #[test]
    fn converges_in_cycle_and_charges_like_production() {
        let n = 40;
        let a = tridiagonal(n, -1.0, 10.0, -2.0);
        let b: Vec<f64> = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect();
        let config = StagedGmresConfig::new(40, 200, 1.0e-10, 0.0);
        let (report, x, counters) = solve(&a, &b, &config, None);
        assert_eq!(report.outcome, StagedGmresOutcome::Converged);
        assert_eq!(report.cycles, 1);
        let c = report.columns;
        assert!(c < n as u64, "converged inside the first cycle");
        // The confirmed residual is the true residual of the returned x.
        let r = residual_norm(&a, &b, &x);
        assert!((r - report.residual_norm).abs() <= 1.0e-12 * report.right_norm);
        assert!(report.residual_norm <= report.threshold);
        // The projected residual equals the true one up to rounding.
        assert!(
            (report.projected_residual - report.residual_norm).abs()
                <= 1.0e-4 * report.residual_norm,
            "projected {} true {}",
            report.projected_residual,
            report.residual_norm
        );
        // One true residual (the confirmation), no duplicate.
        assert_eq!(report.confirmations, 1);
        assert_eq!(report.true_residuals, 1);
        assert_eq!(counters.linear_matvecs, c + 1);
        assert_eq!(report.matvecs, c + 1);
        assert_eq!(counters.diagnostic_matvecs, 0);
        assert_eq!(counters.linear_iterations, c);
        assert_eq!(counters.linear_solves, 1);
        assert_eq!(counters.orthogonalization_inner_products, c * (c + 1));
        assert_eq!(counters.orthogonalization_vector_updates, c * (c + 1) + 1);
    }

    #[test]
    fn restarted_solve_reaches_the_threshold() {
        let n = 60;
        let a = tridiagonal(n, -1.0, 2.5, -1.2);
        let b = vec![1.0; n];
        let config = StagedGmresConfig::new(5, 2000, 1.0e-11, 0.0);
        let (report, x, counters) = solve(&a, &b, &config, None);
        assert_eq!(report.outcome, StagedGmresOutcome::Converged);
        assert!(report.cycles > 1);
        assert!(residual_norm(&a, &b, &x) <= report.threshold);
        assert_eq!(
            counters.linear_matvecs,
            report.columns + report.true_residuals
        );
    }

    #[test]
    fn stall_rule_accepts_the_attainable_floor() {
        let n = 8;
        let a = tridiagonal(n, 0.3, 3.0, -0.7);
        let b: Vec<f64> = (0..n).map(|i| (i as f64).sin() + 1.0).collect();
        let mut config = StagedGmresConfig::new(8, 64, 0.0, 0.0);
        let (failed, x, _) = solve(&a, &b, &config, None);
        assert_eq!(failed.outcome, StagedGmresOutcome::Failed);
        assert_eq!(failed.failure, Some(StagedGmresFailure::BudgetExhausted));
        assert!(failed.budget_exhausted);
        assert!(x.iter().all(|v| v.is_nan()), "output untouched on failure");
        config.stall_rule = true;
        let (stalled, x, _) = solve(&a, &b, &config, None);
        assert_eq!(stalled.outcome, StagedGmresOutcome::StallAccepted);
        assert!(residual_norm(&a, &b, &x) <= 1.0e-13 * stalled.right_norm);
    }

    #[test]
    fn fallback_sees_the_iterate_and_its_true_residual() {
        let n = 8;
        let a = tridiagonal(n, 0.3, 3.0, -0.7);
        let b = vec![1.0; n];
        let config = StagedGmresConfig::new(4, 8, 0.0, 0.0);
        for accept in [true, false] {
            let mut seen = None;
            let mut test = |x: &[f64], r: &[f64]| {
                seen = Some((residual_norm(&a, &b, x), safe_l2(r)));
                accept
            };
            let (report, x, _) = solve(&a, &b, &config, Some(&mut test));
            let (direct, given) = seen.expect("fallback called on budget exhaustion");
            assert!((direct - given).abs() <= 1.0e-14 * report.right_norm);
            assert_eq!(given, report.residual_norm);
            if accept {
                assert_eq!(report.outcome, StagedGmresOutcome::FallbackAccepted);
                assert_eq!(residual_norm(&a, &b, &x), direct);
            } else {
                assert_eq!(report.outcome, StagedGmresOutcome::Failed);
            }
        }
    }

    /// The cyclic shift: restarted GMRES(10) on n = 50 never reduces the
    /// residual of `e_1`.
    fn cyclic_shift(n: usize) -> DenseMatrix {
        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..n {
            a[((i + 1) % n, i)] = 1.0;
        }
        a
    }

    #[test]
    fn guard_aborts_a_stagnating_solve_and_classifies_it() {
        let n = 50;
        let a = cyclic_shift(n);
        let mut b = vec![0.0; n];
        b[0] = 1.0;
        let mut config = StagedGmresConfig::new(10, 100, 1.0e-10, 0.0);
        config.stagnation_guard = true;
        let (plain, _, plain_counters) = solve(&a, &b, &config, None);
        assert_eq!(plain.failure, Some(StagedGmresFailure::GuardContraction));
        assert_eq!(plain.guard_abort, Some(StagedGuardAbort::Contraction));
        assert_eq!(plain.columns, 10, "aborted at the first restart boundary");
        assert_eq!(plain.shadow_converged, None);
        config.classify_guard_aborts = true;
        let (classified, _, counters) = solve(&a, &b, &config, None);
        assert_eq!(classified.shadow_converged, Some(false));
        assert_eq!(counters, plain_counters, "the shadow is not counted");
        // (Debug form: the NaN projected residual is not equal to itself.)
        assert_eq!(
            format!(
                "{:?}",
                StagedGmresReport {
                    shadow_converged: None,
                    ..classified
                }
            ),
            format!("{plain:?}")
        );
    }

    #[test]
    fn shadow_classification_is_the_guard_free_outcome() {
        // SPD-like diagonal systems whose restarted convergence is slow
        // enough for the geometric prediction to overrun small budgets.
        let n = 120;
        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..n {
            a[(i, i)] = 1.0 + 400.0 * (i as f64 / (n - 1) as f64).powi(2);
            if i + 1 < n {
                a[(i, i + 1)] = 0.4;
            }
        }
        let b: Vec<f64> = (0..n).map(|i| ((i * 7 % 11) as f64) - 5.0).collect();
        let mut aborts = 0;
        for budget in [60, 100, 160, 240, 400, 800] {
            let mut config = StagedGmresConfig::new(20, budget, 1.0e-12, 0.0);
            let (free, _, _) = solve(&a, &b, &config, None);
            config.stagnation_guard = true;
            config.classify_guard_aborts = true;
            let (guarded, _, _) = solve(&a, &b, &config, None);
            if let Some(converged) = guarded.shadow_converged {
                aborts += 1;
                assert_eq!(converged, free.accepted(), "budget {budget}");
            } else {
                assert_eq!(guarded.outcome, free.outcome, "budget {budget}");
            }
        }
        assert!(aborts > 0, "the guard fired at least once");
    }

    #[test]
    fn nu_guard_tightens_the_threshold_on_a_nonnormal_system() {
        // W = I - h J with VIG-type blocks lam [[-2, 2^k], [2^-k, -2]].
        let (k, h) = (12, 0.05);
        let n = 8;
        let mut a = DenseMatrix::identity(n);
        for j in 0..n / 2 {
            let lam = 10f64.powi(j as i32);
            let (p, q) = (2 * j, 2 * j + 1);
            a[(p, p)] += 2.0 * h * lam;
            a[(p, q)] -= h * lam * 2f64.powi(k);
            a[(q, p)] -= h * lam * 2f64.powi(-k);
            a[(q, q)] += 2.0 * h * lam;
        }
        let b: Vec<f64> = (0..n).map(|i| 1.0 / (1.0 + i as f64)).collect();
        let mut config = StagedGmresConfig::new(8, 64, 1.0e-6, 0.0);
        let (plain, _, _) = solve(&a, &b, &config, None);
        config.nu_guard = true;
        let (guarded, x, _) = solve(&a, &b, &config, None);
        assert_eq!(plain.nu_evaluations, 0);
        assert_eq!(plain.final_threshold, plain.threshold);
        assert!(guarded.nu_evaluations >= 1);
        assert!(guarded.nu_max > 1.0);
        assert_eq!(guarded.final_threshold, guarded.threshold / guarded.nu_max);
        assert_eq!(guarded.outcome, StagedGmresOutcome::Converged);
        assert!(residual_norm(&a, &b, &x) <= 1.0001 * guarded.final_threshold);
        assert!(guarded.residual_norm <= guarded.final_threshold);
    }

    /// `x -> A x + delta`: affine, so the projected residual keeps falling
    /// while the true residual stays near `|sum y - 1| ||delta||`.
    struct Affine {
        a: DenseMatrix,
        delta: f64,
    }

    impl LinearOperator for Affine {
        fn dimension(&self) -> usize {
            self.a.nrows()
        }
        fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
            self.a.matvec_into(x, y)?;
            for (i, v) in y.iter_mut().enumerate() {
                *v += if i % 3 == 0 {
                    self.delta
                } else {
                    -0.5 * self.delta
                };
            }
            Ok(())
        }
        fn token(&self) -> u64 {
            u64::MAX
        }
    }

    #[test]
    fn failed_confirmations_continue_the_cycle_with_doubling_gaps() {
        let n = 60;
        let op = Affine {
            a: tridiagonal(n, -1.0, 10.0, -2.0),
            delta: 1.0e-6,
        };
        let b = vec![1.0; n];
        let config = StagedGmresConfig::new(40, 200, 1.0e-12, 0.0);
        let mut x = vec![f64::NAN; n];
        let mut counters = WorkCounters::default();
        let mut ws = StagedGmresWorkspace::default();
        let report =
            solve_staged_gmres(&op, &b, &config, None, &mut x, &mut ws, &mut counters).unwrap();
        // The first cycle runs all 40 columns through its failed
        // confirmations (gaps 1, 2, 4, 8, ...), the restart residual is the
        // only other true residual, and the second cycle confirms at once.
        assert_eq!(report.outcome, StagedGmresOutcome::Converged, "{report:?}");
        assert_eq!(report.cycles, 2);
        assert!(report.columns > 40);
        assert!(report.failed_confirmations >= 2);
        assert_eq!(report.confirmations, report.failed_confirmations + 1);
        assert_eq!(report.true_residuals, report.confirmations + 1);
        assert_eq!(
            counters.linear_matvecs,
            report.columns + report.true_residuals
        );
        assert!(report.residual_norm <= report.threshold);
    }

    #[test]
    fn zero_right_hand_side_needs_no_work() {
        let a = tridiagonal(4, 1.0, 3.0, 1.0);
        let b = vec![0.0; 4];
        let config = StagedGmresConfig::new(4, 8, 1.0e-10, 0.0);
        let (report, x, counters) = solve(&a, &b, &config, None);
        assert_eq!(report.outcome, StagedGmresOutcome::Converged);
        assert_eq!(x, vec![0.0; 4]);
        assert_eq!(counters.linear_matvecs, 0);
    }
}
