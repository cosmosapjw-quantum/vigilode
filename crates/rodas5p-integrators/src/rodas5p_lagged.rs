//! Lagged-factorization stage-root RODAS5P driver (production wave-1 node
//! `b1-lagged`; plan `docs/plan/PRODUCTION_PLAN_20261020.md` §5 T1, T2, T9
//! and §6 B1). Opt-in; no existing entry point changes.
//!
//! The method, coordinates, controller, represented clock and output rules
//! are those of [`crate::integrate_rodas5p_fast_observed`] (the dense fast
//! driver, U form with `W = I/(h gamma) - J_0` and `u_i = sum_j Gamma_ij k_j`);
//! its workspace, factorization and solve are reused unchanged. What differs
//! is how each stage system `W u_i = c_i` is solved:
//!
//! * The Jacobian `J_0 = J(t_n, y_n)` is evaluated at every new state, as in
//!   the fast driver: the stage equations are the exact RODAS5P equations
//!   (T1), not a W-method with an old Jacobian.
//! * Only the factorization is lagged. `P` is the stored LU of
//!   `I/(h_P gamma) - J_P` from an earlier attempt, and each stage runs the
//!   preconditioned Richardson iteration `x <- x + P^{-1} (c - W x)` from
//!   `x = P^{-1} c`. `W x = x/(h gamma) - J_0 x` uses the problem's JVP at
//!   `(t_n, y_n)` when there is one, else the current explicit Jacobian's
//!   matvec (no finite differences).
//! * Stage `i` stops when `WRMS(P^{-1} r) <= rho_i`,
//!   `rho_i = theta / (8 max(a_i, 1))` (T9), with the nonstiff-limit output
//!   sensitivities `a_i` of T1 ([`RODAS5P_NONSTIFF_STAGE_SENSITIVITIES`]).
//! * `P` is refreshed (refactored at the current `(h, J_0)`, and the stage
//!   restarted) before the first step, when the contraction estimate
//!   `q_k = WRMS(P^{-1} r_{k+1}) / WRMS(P^{-1} r_k)` exceeds
//!   [`RODAS5P_LAGGED_CONTRACTION_MAX`], or when a stage would need more than
//!   [`RODAS5P_LAGGED_MAX_ITERATIONS`] iterations (T2). A refresh is charged
//!   to `direct_factorizations` and to [`LaggedWork`]; the iterations it
//!   abandons stay counted.
//! * When `P` was factored at the attempt's own `(h, J_0)` it equals `W`,
//!   and the stage is the direct solve with no iteration. A refresh forced at
//!   every attempt ([`Rodas5pLaggedOptions::refresh_every_attempt`]) thus
//!   performs the fast driver's arithmetic in the fast driver's order.
//!
//! # Units of the stopping test
//!
//! The fast driver's stage system is the plan's `(I - h gamma J_0) U_i = ...`
//! divided by `h gamma`, so the raw residual `r = c - W x` is in right-hand
//! side units (state per time); the plan's residual of T1, in state units,
//! is `r_T1 = h gamma r`. The tested quantity is not `r` but `P^{-1} r`, and
//! `P = I/(h_P gamma) - J_P` carries the same `1/(time)` scaling as `W`, so
//! `P^{-1} r` is already a state increment of the U form: the conversion
//! factor is exactly 1, with no hidden `h gamma`. Equivalently, with the
//! plan's normalisation `P_T1 = h_P gamma P`,
//! `P^{-1} r = (h_P / h) P_T1^{-1} r_T1`; the factor `h_P / h` appears only if
//! one changes the preconditioner's normalisation, which the iteration does
//! not. The test quantity is the right one because the iteration's error
//! `e_k = x_k - W^{-1} c` satisfies `P^{-1} r_k = -(I - T) e_k` with
//! `T = I - P^{-1} W` (T2), so `WRMS(P^{-1} r_k)` estimates the stage error
//! in the state units of T1 within the factor `1 +- q`. The WRMS weights are
//! `atol + rtol |y_n|`: the step's error scale `atol + rtol max(|y_n|, |y_new|)`
//! is not known during the stages, and the `|y_n|` weights are componentwise
//! no larger, so the test is never looser than the step's scale.
//!
//! # Zero tolerance scale
//!
//! The end-of-step error norm follows the A1 zero-scale rule of every fast
//! driver (`rodas5p_core::scaled_error_component`), so a forced refresh
//! reproduces the fast driver also with `atol = 0`. The Richardson weights
//! are an exception, recorded next to the staged matrix-free one: a zero
//! weight `atol + rtol |y_n| = 0` on a stage that iterates returns a typed
//! `CoreError::InvalidInput` from the run (an input refusal; nothing is
//! counted as a rejection). `atol = 0` with a component that is zero at a
//! step start is therefore supported only on stages solved directly
//! (`P = W`, e.g. with [`Rodas5pLaggedOptions::refresh_every_attempt`]).
//!
//! # GMRES stage solver (wave-2 node `b1b-gmres`)
//!
//! [`StageSolver::Gmres`] (opt-in; preregistration
//! `docs/plan/nodes/b1b-gmres.md`) replaces Richardson on lagged stages by
//! GMRES on `W x = c`, right-preconditioned with the same stored LU (`W
//! P^{-1} v = c`, `x = P^{-1} v`), restart [`RODAS5P_LAGGED_GMRES_RESTART`],
//! through `rodas5p_krylov::solve_gmres_into_right_monitored` (the
//! `gmres_into` right-preconditioned Arnoldi cycle with the iterate handed
//! to a monitor after every column). Everything else above is unchanged.
//!
//! * Start `x_0 = P^{-1} c`, as Richardson.
//! * The tested residual is the left-preconditioned one of T2/T9,
//!   `nu_k = WRMS(P^{-1} r_k)`, `r_k = c - W x_k`, computed explicitly (one
//!   `W x` and one LU application) at `k = 0` and after every Arnoldi
//!   column `k`. GMRES itself minimizes the 2-norm of the true residual
//!   `r_k`; that norm is not tested.
//! * At each check: `nu_k <= rho_i` converges, and the computed correction is
//!   applied (`x <- x_k + P^{-1} r_k`, as Richardson applies its last one);
//!   else, for `k >= 1`, `(nu_k / nu_0)^{1/k} >` [`RODAS5P_LAGGED_CONTRACTION_MAX`]
//!   refreshes `P` (contraction); else `k =` [`RODAS5P_LAGGED_MAX_ITERATIONS`]
//!   refreshes `P` (budget). A refresh restarts the stage with the direct
//!   solve, as on the Richardson path.
//! * Counted: Arnoldi columns ([`LaggedWork::gmres_iterations`]), cycles,
//!   every `W x` ([`LaggedWork::operator_applications`]; those that are not
//!   columns, i.e. explicit stopping residuals and the cycle-start true
//!   residual, also in [`LaggedWork::residual_applications`]) and every LU
//!   application (`direct_solve_calls`).

use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, DenseMatrix, ExactOperatorIdentity, LinearOperator,
    OperatorApplicationWork, Preconditioner, PreconditionerSide, WorkCounters, apply_counted,
    apply_preconditioner, scaled_error_component,
};
use rodas5p_krylov::{
    GmresConfig, GmresMonitorDecision, GmresWorkspace, solve_gmres_into_right_monitored,
};
use serde::Serialize;

use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, ObservedIntegrationResult,
    OdeProblem, OutputSchedule,
    output::OutputCollector,
    problem::JvpFn,
    rodas_next_step_after_attempt,
    rodas5p_fast::{FastLuPolicy, Rodas5pFastResult, Workspace, failure_kind},
};

/// Identifier of this driver in benchmark and research records.
pub const RODAS5P_LAGGED_DRIVER_ID: &str = "rodas5p-lagged-richardson-v1";

/// Identifier of the [`StageSolver::Gmres`] arm (wave-2 node `b1b-gmres`).
pub const RODAS5P_LAGGED_GMRES_DRIVER_ID: &str = "rodas5p-lagged-gmres-v1";

/// The GMRES restart length of [`StageSolver::Gmres`] (preregistered; with
/// the budget of [`RODAS5P_LAGGED_MAX_ITERATIONS`] columns it never fires).
pub const RODAS5P_LAGGED_GMRES_RESTART: usize = 10;

/// The output-perturbation budget `theta` of T1/T9, in tolerance units.
pub const RODAS5P_LAGGED_THETA: f64 = 0.1;

/// The contraction estimate above which `P` is refreshed (T2, `q_max`).
pub const RODAS5P_LAGGED_CONTRACTION_MAX: f64 = 0.5;

/// The Richardson iterations a stage may take before `P` is refreshed (T2).
pub const RODAS5P_LAGGED_MAX_ITERATIONS: usize = 6;

/// The nonstiff-limit (`z = h lambda = 0`) sensitivities `|m_j|` of the
/// RODAS5P output to a residual on stage `j`, from plan §5 T1 ("Numbers for
/// RODAS5P", computed from the repository's coefficient snapshot); they
/// decrease with stiffness, so using the `z = 0` values is conservative for
/// normal `J_0` with spectrum in the left half plane. Each entry is the
/// t-verify value (`research/production_theory_20261021/RESULTS.json`,
/// `claim1_T1_numbers.a_values_at_z0.abs_m`) rounded up to two decimals, so
/// no entry is below the verified value (a_7 = 1.0110 gives 1.02; a test
/// reads the JSON).
pub const RODAS5P_NONSTIFF_STAGE_SENSITIVITIES: [f64; 8] =
    [0.58, 0.24, 6.25, 5.97, 3.73, 4.57, 1.02, 1.00];

/// The per-stage stopping targets `rho_i = theta / (8 max(a_i, 1))` (T9).
pub fn rodas5p_lagged_stage_targets() -> [f64; 8] {
    RODAS5P_NONSTIFF_STAGE_SENSITIVITIES.map(|a| {
        RODAS5P_LAGGED_THETA / (RODAS5P_NONSTIFF_STAGE_SENSITIVITIES.len() as f64 * a.max(1.0))
    })
}

/// How a lagged stage (`P != W`) is solved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum StageSolver {
    /// Preconditioned Richardson (wave-1 node `b1-lagged`, the default and
    /// bit-identical to it).
    #[default]
    Richardson,
    /// GMRES right-preconditioned with the stored LU (wave-2 node
    /// `b1b-gmres`; module docs).
    Gmres,
}

/// Opt-in variants of [`integrate_rodas5p_lagged_observed`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rodas5pLaggedOptions {
    /// Refactor `P` at the start of every attempt (so `P = W` and every
    /// stage is the direct solve): the equivalence arm of the fast driver.
    pub refresh_every_attempt: bool,
    /// The lagged-stage solver; Richardson by default.
    pub stage_solver: StageSolver,
}

/// Counted work of the lagged stage solves. Every refresh is also a
/// `direct_factorizations` count of the run's [`WorkCounters`]; every
/// application of the stored LU is a `direct_solve_calls` count there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct LaggedWork {
    /// Factorizations of `P`; the sum of the four reasons below.
    pub preconditioner_refreshes: u64,
    /// Before the first step, or after a failed attempt left `P` unusable.
    pub initial_refreshes: u64,
    /// [`Rodas5pLaggedOptions::refresh_every_attempt`].
    pub forced_refreshes: u64,
    /// `q_k > ` [`RODAS5P_LAGGED_CONTRACTION_MAX`].
    pub contraction_refreshes: u64,
    /// More than [`RODAS5P_LAGGED_MAX_ITERATIONS`] iterations needed.
    pub budget_refreshes: u64,
    /// Richardson iterations attempted, including abandoned ones.
    pub richardson_iterations: u64,
    /// GMRES Arnoldi columns built, including abandoned ones
    /// ([`StageSolver::Gmres`]).
    pub gmres_iterations: u64,
    /// GMRES cycles (one per stage solved by GMRES with at least one column
    /// while the restart does not fire).
    pub gmres_cycles: u64,
    /// Iterations (Richardson) or columns (GMRES) of stages that were then
    /// restarted after a refresh.
    pub abandoned_iterations: u64,
    /// Products `W x` (Richardson: one per iteration; GMRES: columns plus
    /// [`Self::residual_applications`]).
    pub operator_applications: u64,
    /// GMRES products `W x` that are not Arnoldi columns: the explicit
    /// stopping residuals and the cycle-start true residuals.
    pub residual_applications: u64,
    /// Of those, through the problem's JVP (the rest are explicit matvecs).
    pub jvp_applications: u64,
    /// Applications of the stored LU (initial solves and corrections).
    pub preconditioner_applications: u64,
    /// Stages solved directly with `P = W` (no refresh inside the stage).
    pub exact_stage_solves: u64,
    /// Stages that converged by Richardson iteration on a lagged `P`.
    pub lagged_stage_solves: u64,
    /// Stages restarted and solved directly after a refresh.
    pub refreshed_stage_solves: u64,
}

#[derive(Clone, Debug)]
pub struct Rodas5pLaggedResult {
    /// The run in the fast driver's record (driver
    /// [`RODAS5P_LAGGED_DRIVER_ID`]).
    pub fast: Rodas5pFastResult,
    pub work: LaggedWork,
}

#[derive(Clone, Copy)]
enum Refresh {
    Initial,
    Forced,
    Contraction,
    Budget,
}

/// The lagged preconditioner and the iteration buffers.
struct Lagged {
    options: Rodas5pLaggedOptions,
    targets: [f64; 8],
    /// Stage right-hand side `c`, iterate `x`, product `W x`, WRMS weights.
    rhs: Vec<f64>,
    x: Vec<f64>,
    wx: Vec<f64>,
    weight: Vec<f64>,
    /// GMRES: workspace, solution, and the last preconditioned residual.
    gmres: GmresWorkspace,
    solution: Vec<f64>,
    pres: Vec<f64>,
    /// Whether the workspace factors hold a usable `P`, and its `(h, J)`
    /// (the Jacobian by generation: one per Jacobian build).
    valid: bool,
    p_h: f64,
    p_generation: u64,
    generation: u64,
    work: LaggedWork,
}

impl Lagged {
    fn refresh(
        &mut self,
        ws: &mut Workspace,
        h: f64,
        reason: Refresh,
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        // Charged before the factorization so a failed one stays counted.
        self.valid = false;
        self.work.preconditioner_refreshes += 1;
        match reason {
            Refresh::Initial => self.work.initial_refreshes += 1,
            Refresh::Forced => self.work.forced_refreshes += 1,
            Refresh::Contraction => self.work.contraction_refreshes += 1,
            Refresh::Budget => self.work.budget_refreshes += 1,
        }
        ws.factor(h, counters)?;
        self.valid = true;
        self.p_h = h;
        self.p_generation = self.generation;
        Ok(())
    }

    /// `wx = W x = x/(h gamma) - J_0 x`.
    fn apply_w(
        &mut self,
        ws: &Workspace,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        inv: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.work.operator_applications += 1;
        counters.linear_matvecs += 1;
        if let Some(jvp) = problem.jvp_callback() {
            self.work.jvp_applications += 1;
            counters.jvp_calls += 1;
            counters.jvp_vectors += 1;
            self.wx.fill(0.0);
            jvp(t, y, &self.x, &mut self.wx)?;
        } else {
            ws.jacobian.matvec_into(&self.x, &mut self.wx)?;
        }
        for (w, x) in self.wx.iter_mut().zip(&self.x) {
            *w = inv * x - *w;
        }
        if self.wx.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::NonFinite(
                "RODAS5P lagged stage operator produced NaN/Inf".into(),
            ))
        }
    }

    fn wrms(&self, v: &[f64]) -> f64 {
        weighted_rms(v, &self.weight)
    }

    fn check_weights(&self) -> CoreResult<()> {
        // Zero-weight exception (controller ruling, final review item 2):
        // the weights `atol + rtol |y_n|` are start-state weights of the
        // stopping test, like the staged matrix-free Krylov weights, not an
        // end-of-step error scale. A zero weight (`atol = 0`, `y_n,i = 0`)
        // would make the test `x / 0`; the run is refused with a typed
        // `InvalidInput` (an input refusal, not a counted rejection).
        if self.weight.iter().all(|w| *w > 0.0 && w.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::InvalidInput(
                "the RODAS5P lagged Richardson stopping test needs finite positive Richardson \
                 weights atol + rtol |y_n| (atol = 0 with a zero component is unsupported on \
                 a lagged stage)"
                    .into(),
            ))
        }
    }

    /// GMRES on stage `stage` from `x = P^{-1} c` (in `self.x`, with `c` in
    /// `self.rhs`), right-preconditioned with the stored LU (module docs):
    /// `Ok(None)` when it converged (the solution in `self.x`),
    /// `Ok(Some(reason))` when `P` must be refreshed.
    #[allow(clippy::too_many_arguments)]
    fn iterate_gmres(
        &mut self,
        ws: &Workspace,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        stage: usize,
        counters: &mut WorkCounters,
    ) -> CoreResult<Option<Refresh>> {
        self.check_weights()?;
        let op = StageOperator {
            jacobian: &ws.jacobian,
            jvp: problem.jvp_callback(),
            t,
            y,
            inv: 1.0 / (h * ws.gamma),
            applications: AtomicU64::new(0),
        };
        let pc = StoredLu {
            ws,
            applications: AtomicU64::new(0),
        };
        let target = self.targets[stage];
        let outcome = self.gmres_stage(&op, &pc, target, counters);
        // Charged whether the stage converged, refreshes or failed.
        let products = op.applications.load(Ordering::Relaxed);
        let lu = pc.applications.load(Ordering::Relaxed);
        self.work.operator_applications += products;
        if op.jvp.is_some() {
            self.work.jvp_applications += products;
        }
        self.work.preconditioner_applications += lu;
        counters.direct_solve_calls += lu;
        let (columns, decision) = outcome?;
        self.work.residual_applications += products - columns;
        match decision {
            None => {
                for (x, z) in self.x.iter_mut().zip(&self.pres) {
                    *x += z;
                }
                Ok(None)
            }
            Some(reason) => {
                self.work.abandoned_iterations += columns;
                Ok(Some(reason))
            }
        }
    }

    /// The checks and the GMRES call of [`Self::iterate_gmres`]: the
    /// Arnoldi columns built and `None` (converged; `self.x = x_k`,
    /// `self.pres = P^{-1} r_k`) or the refresh reason.
    fn gmres_stage(
        &mut self,
        op: &StageOperator<'_>,
        pc: &StoredLu<'_>,
        target: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<(u64, Option<Refresh>)> {
        let nu0 = preconditioned_residual(
            op,
            pc,
            &self.rhs,
            &self.x,
            &mut self.wx,
            &mut self.pres,
            &self.weight,
            counters,
        )?;
        if nu0 <= target {
            return Ok((0, None));
        }
        let config = GmresConfig {
            restart: RODAS5P_LAGGED_GMRES_RESTART,
            max_arnoldi: RODAS5P_LAGGED_MAX_ITERATIONS,
            rtol: 0.0,
            atol: 0.0,
        };
        let mut reason = Refresh::Budget;
        let (rhs, weight, wx, pres) = (&self.rhs, &self.weight, &mut self.wx, &mut self.pres);
        let report = solve_gmres_into_right_monitored(
            op,
            pc,
            rhs,
            &self.x,
            &config,
            &mut self.solution,
            &mut self.gmres,
            counters,
            &mut |k, xk, counters| {
                let nu = preconditioned_residual(op, pc, rhs, xk, wx, pres, weight, counters)?;
                if nu <= target {
                    return Ok(GmresMonitorDecision::Accept);
                }
                if (nu / nu0).powf(1.0 / k as f64) > RODAS5P_LAGGED_CONTRACTION_MAX {
                    reason = Refresh::Contraction;
                    return Ok(GmresMonitorDecision::Abandon);
                }
                if k >= RODAS5P_LAGGED_MAX_ITERATIONS as u64 {
                    reason = Refresh::Budget;
                    return Ok(GmresMonitorDecision::Abandon);
                }
                Ok(GmresMonitorDecision::Continue)
            },
        )?;
        self.work.gmres_iterations += report.iterations;
        self.work.gmres_cycles += report.cycles;
        if report.accepted {
            self.x.copy_from_slice(&self.solution);
            Ok((report.iterations, None))
        } else {
            Ok((report.iterations, Some(reason)))
        }
    }

    /// Richardson on stage `stage` from `x = P^{-1} c` (in `self.x`, with
    /// `c` in `self.rhs`): `Ok(None)` when it converged (the solution in
    /// `self.x`), `Ok(Some(reason))` when `P` must be refreshed.
    #[allow(clippy::too_many_arguments)]
    fn iterate(
        &mut self,
        ws: &mut Workspace,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        stage: usize,
        counters: &mut WorkCounters,
    ) -> CoreResult<Option<Refresh>> {
        self.check_weights()?;
        let inv = 1.0 / (h * ws.gamma);
        let target = self.targets[stage];
        let mut previous: Option<f64> = None;
        for k in 1..=RODAS5P_LAGGED_MAX_ITERATIONS as u64 {
            self.apply_w(ws, problem, t, y, inv, counters)?;
            for ((r, c), w) in ws.stage_rhs.iter_mut().zip(&self.rhs).zip(&self.wx) {
                *r = c - w;
            }
            self.work.richardson_iterations += 1;
            self.work.preconditioner_applications += 1;
            ws.solve(counters)?;
            let norm = self.wrms(&ws.stage_rhs);
            for (x, z) in self.x.iter_mut().zip(&ws.stage_rhs) {
                *x += z;
            }
            if !norm.is_finite() {
                return Err(CoreError::NonFinite(
                    "RODAS5P lagged stage residual is not finite".into(),
                ));
            }
            if norm <= target {
                return Ok(None);
            }
            if let Some(p) = previous
                && norm / p > RODAS5P_LAGGED_CONTRACTION_MAX
            {
                self.work.abandoned_iterations += k;
                return Ok(Some(Refresh::Contraction));
            }
            previous = Some(norm);
        }
        self.work.abandoned_iterations += RODAS5P_LAGGED_MAX_ITERATIONS as u64;
        Ok(Some(Refresh::Budget))
    }

    /// One attempt from `(t, y)` with step `h`, as the fast driver's
    /// `Workspace::attempt` with the stage solves replaced (the Jacobian,
    /// `f(t, y)`, `f_t`, stage right-hand sides, update and error norm are
    /// its arithmetic in its order).
    #[allow(clippy::too_many_arguments)]
    fn attempt(
        &mut self,
        ws: &mut Workspace,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        fresh: bool,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        let (n, s) = (ws.n, ws.s);
        if fresh {
            // A new Jacobian generation before the build, so a partially
            // overwritten Jacobian never matches a stored P.
            self.generation += 1;
            problem.dense_jacobian_into(t, y, &mut ws.jacobian, counters)?;
            if !ws.lu_chosen {
                ws.choose_lu();
                ws.lu_chosen = true;
            }
            problem.eval_rhs_into(t, y, &mut ws.f0, counters)?;
            if !problem.autonomous {
                let ft = problem.eval_partial_t(t, y, counters)?;
                ws.ft.copy_from_slice(&ft);
            }
        }
        if !self.valid {
            self.refresh(ws, h, Refresh::Initial, counters)?;
        } else if self.options.refresh_every_attempt {
            self.refresh(ws, h, Refresh::Forced, counters)?;
        }
        let mut exact = self.p_generation == self.generation && self.p_h == h;
        for (w, v) in self.weight.iter_mut().zip(y) {
            *w = atol + rtol * v.abs();
        }
        for i in 0..s {
            if i == 0 {
                ws.stage_rhs.copy_from_slice(&ws.f0);
            } else {
                ws.stage_state.copy_from_slice(y);
                for &(j, aij) in &ws.a_nonzero[i] {
                    let uj = &ws.u[j * n..(j + 1) * n];
                    for (x, v) in ws.stage_state.iter_mut().zip(uj) {
                        *x += aij * v;
                    }
                }
                problem.eval_rhs_into(
                    t + ws.c[i] * h,
                    &ws.stage_state,
                    &mut ws.stage_rhs,
                    counters,
                )?;
            }
            for &(j, c) in &ws.c_nonzero[i] {
                let cij = c / h;
                if cij != 0.0 {
                    let uj = &ws.u[j * n..(j + 1) * n];
                    for (x, v) in ws.stage_rhs.iter_mut().zip(uj) {
                        *x += cij * v;
                    }
                }
            }
            if !problem.autonomous {
                let g = ws.gamma_rows[i] * h;
                for (x, v) in ws.stage_rhs.iter_mut().zip(&ws.ft) {
                    *x += g * v;
                }
            }
            if exact {
                self.work.preconditioner_applications += 1;
                ws.solve(counters)?;
                self.work.exact_stage_solves += 1;
            } else {
                self.rhs.copy_from_slice(&ws.stage_rhs);
                self.work.preconditioner_applications += 1;
                ws.solve(counters)?;
                self.x.copy_from_slice(&ws.stage_rhs);
                let iterated = match self.options.stage_solver {
                    StageSolver::Richardson => self.iterate(ws, problem, t, y, h, i, counters)?,
                    StageSolver::Gmres => self.iterate_gmres(ws, problem, t, y, h, i, counters)?,
                };
                match iterated {
                    None => {
                        ws.stage_rhs.copy_from_slice(&self.x);
                        self.work.lagged_stage_solves += 1;
                    }
                    Some(reason) => {
                        self.refresh(ws, h, reason, counters)?;
                        exact = true;
                        ws.stage_rhs.copy_from_slice(&self.rhs);
                        self.work.preconditioner_applications += 1;
                        ws.solve(counters)?;
                        self.work.refreshed_stage_solves += 1;
                    }
                }
            }
            ws.u[i * n..(i + 1) * n].copy_from_slice(&ws.stage_rhs);
        }
        ws.y_new.copy_from_slice(y);
        for &(j, bj) in &ws.b_nonzero {
            let uj = &ws.u[j * n..(j + 1) * n];
            for (x, v) in ws.y_new.iter_mut().zip(uj) {
                *x += bj * v;
            }
        }
        if !ws.y_new.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "RODAS5P lagged step produced NaN/Inf".into(),
            ));
        }
        // The fast driver's error norm: WRMS of u_s in the scale
        // atol + rtol max(|y|, |y_new|), with the A1 zero-scale rule
        // (`rodas5p_core::scaled_error_component`) as in the fast driver.
        let error = &ws.u[(s - 1) * n..s * n];
        let mut sum = 0.0;
        for ((e, a), b) in error.iter().zip(y).zip(&ws.y_new) {
            let z = scaled_error_component(*e, atol + rtol * a.abs().max(b.abs()));
            sum += z * z;
        }
        let norm = (sum / n as f64).sqrt();
        Ok(if norm.is_finite() {
            norm
        } else {
            f64::INFINITY
        })
    }
}

fn weighted_rms(v: &[f64], weight: &[f64]) -> f64 {
    let sum: f64 = v.iter().zip(weight).map(|(x, w)| (x / w) * (x / w)).sum();
    (sum / v.len() as f64).sqrt()
}

/// `nu = WRMS(P^{-1} (c - W x))`, explicitly: `wx = W x`, `pres = P^{-1} r`.
#[allow(clippy::too_many_arguments)]
fn preconditioned_residual(
    op: &StageOperator<'_>,
    pc: &StoredLu<'_>,
    rhs: &[f64],
    x: &[f64],
    wx: &mut [f64],
    pres: &mut [f64],
    weight: &[f64],
    counters: &mut WorkCounters,
) -> CoreResult<f64> {
    apply_counted(op, x, wx, counters, ApplyCategory::Krylov)?;
    for (w, c) in wx.iter_mut().zip(rhs) {
        *w = c - *w;
    }
    apply_preconditioner(pc, wx, pres, counters)?;
    let nu = weighted_rms(pres, weight);
    if nu.is_finite() {
        Ok(nu)
    } else {
        Err(CoreError::NonFinite(
            "RODAS5P lagged stage residual is not finite".into(),
        ))
    }
}

/// `W x = x/(h gamma) - J_0 x` as a Krylov operator: the arithmetic of the
/// Richardson path's `apply_w` (JVP at `(t_n, y_n)` when the problem has
/// one, else the current explicit Jacobian's matvec).
struct StageOperator<'a> {
    jacobian: &'a DenseMatrix,
    jvp: Option<&'a JvpFn>,
    t: f64,
    y: &'a [f64],
    inv: f64,
    applications: AtomicU64,
}

impl LinearOperator for StageOperator<'_> {
    fn dimension(&self) -> usize {
        self.y.len()
    }

    fn apply(&self, x: &[f64], out: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.y.len() || out.len() != self.y.len() {
            return Err(CoreError::Dimension(
                "RODAS5P lagged stage operator shape mismatch".into(),
            ));
        }
        self.applications.fetch_add(1, Ordering::Relaxed);
        if let Some(jvp) = self.jvp {
            out.fill(0.0);
            jvp(self.t, self.y, x, out)?;
        } else {
            self.jacobian.matvec_into(x, out)?;
        }
        for (w, v) in out.iter_mut().zip(x) {
            *w = self.inv * v - *w;
        }
        if out.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::NonFinite(
                "RODAS5P lagged stage operator produced NaN/Inf".into(),
            ))
        }
    }

    fn application_work(&self) -> OperatorApplicationWork {
        if self.jvp.is_some() {
            OperatorApplicationWork {
                jvp_calls: 1,
                jvp_vectors: 1,
                ..OperatorApplicationWork::default()
            }
        } else {
            OperatorApplicationWork {
                jacobian_matvecs: 1,
                ..OperatorApplicationWork::default()
            }
        }
    }

    /// A per-stage view; no identity to reuse across solves.
    fn exact_identity(&self) -> Option<ExactOperatorIdentity> {
        None
    }

    fn token(&self) -> u64 {
        0
    }
}

/// The stored lagged LU `P` as a right preconditioner (`P^{-1}` by the
/// fast workspace's dense factors).
struct StoredLu<'a> {
    ws: &'a Workspace,
    applications: AtomicU64,
}

impl Preconditioner for StoredLu<'_> {
    fn dimension(&self) -> usize {
        self.ws.n
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.ws.n || y.len() != self.ws.n {
            return Err(CoreError::Dimension(
                "RODAS5P lagged preconditioner shape mismatch".into(),
            ));
        }
        self.applications.fetch_add(1, Ordering::Relaxed);
        y.copy_from_slice(x);
        self.ws.solve_dense_into(y)
    }

    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
}

/// Adaptive RODAS5P with lagged factorizations and exact-Jacobian stage
/// equations solved by preconditioned Richardson iteration (module docs).
/// The controller, clock, output and rejection rules are those of
/// [`crate::integrate_rodas5p_fast_observed`] with default options; dense
/// storage only, identity mass matrix only.
pub fn integrate_rodas5p_lagged_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pLaggedOptions,
) -> CoreResult<Rodas5pLaggedResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if tf < t || y0.len() != problem.dimension {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P lagged integration input".into(),
        ));
    }
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "the RODAS5P lagged driver supports the identity mass matrix only".into(),
        ));
    }
    let n = problem.dimension;
    let mut ws = Workspace::new(n, FastLuPolicy::Legacy)?;
    if ws.s != RODAS5P_NONSTIFF_STAGE_SENSITIVITIES.len() {
        return Err(CoreError::Coefficients(
            "the RODAS5P lagged driver's stage sensitivities assume 8 stages".into(),
        ));
    }
    let mut lagged = Lagged {
        options,
        targets: rodas5p_lagged_stage_targets(),
        rhs: vec![0.0; n],
        x: vec![0.0; n],
        wx: vec![0.0; n],
        weight: vec![0.0; n],
        gmres: GmresWorkspace::default(),
        solution: vec![0.0; n],
        pres: vec![0.0; n],
        valid: false,
        p_h: 0.0,
        p_generation: 0,
        generation: 0,
        work: LaggedWork::default(),
    };
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut collector =
        OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
    let (mut attempts, mut accepted_steps, mut rejected_steps, mut reuses) = (0, 0, 0, 0);
    let mut internal_steps = 0_usize;
    // Whether the Jacobian, f(t, y) and f_t in the workspace belong to (t, y).
    let mut fresh_state = false;
    while t < tf && attempts < adaptive.max_attempts {
        let Some(next_h) =
            crate::output::adaptive_end_step(t, h, tf, adaptive.step_cap(), &controller)?
        else {
            break;
        };
        h = next_h;
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf) || t + h == t {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        attempts += 1;
        if fresh_state {
            reuses += 1;
        }
        let outcome = lagged.attempt(
            &mut ws,
            problem,
            t,
            &y,
            trial_h,
            !fresh_state,
            adaptive.atol,
            adaptive.rtol,
            &mut counters,
        );
        fresh_state = ws.lu_chosen;
        let (error, failure) = match outcome {
            Ok(error) if error <= 1.0 => (error, None),
            Ok(error) => (error, Some(AdaptiveFailureKind::LocalError)),
            Err(error) => match failure_kind(&error) {
                Some(kind) => {
                    fresh_state = false;
                    // A failed factorization or solve leaves no usable P.
                    lagged.valid = false;
                    (f64::INFINITY, Some(kind))
                }
                None => return Err(error),
            },
        };
        match failure {
            None => {
                counters.accepted_steps += 1;
                accepted_steps += 1;
                t += trial_h;
                y.copy_from_slice(&ws.y_new);
                collector.accept(t, &y, clipped)?;
                internal_steps += 1;
                fresh_state = false;
            }
            Some(kind) => {
                counters.rejected_steps += 1;
                rejected_steps += 1;
                match kind {
                    AdaptiveFailureKind::LocalError => counters.local_error_failures += 1,
                    AdaptiveFailureKind::LinearSolve => counters.linear_solve_failures += 1,
                    AdaptiveFailureKind::NonlinearSolve => counters.nonlinear_solve_failures += 1,
                    AdaptiveFailureKind::NonFinite => counters.nonfinite_step_failures += 1,
                }
            }
        }
        h = rodas_next_step_after_attempt(
            &mut controller,
            adaptive,
            h,
            trial_h,
            error,
            failure.is_none(),
            clipped,
        )?;
    }
    let success = t >= tf;
    let (times, states, output_clipped_steps) = if success {
        collector.finish()?
    } else {
        collector.finish_partial()
    };
    Ok(Rodas5pLaggedResult {
        fast: Rodas5pFastResult {
            observed: ObservedIntegrationResult {
                t: times,
                y: states,
                success,
                message: if success {
                    "success".into()
                } else {
                    "maximum step count or minimum step reached".into()
                },
                counters,
                internal_steps,
                output_clipped_steps,
            },
            attempts,
            accepted_steps,
            rejected_steps,
            jacobian_reuses: reuses,
            lu: ws.lu_kind,
            driver: match options.stage_solver {
                StageSolver::Richardson => RODAS5P_LAGGED_DRIVER_ID,
                StageSolver::Gmres => RODAS5P_LAGGED_GMRES_DRIVER_ID,
            },
        },
        work: lagged.work,
    })
}
