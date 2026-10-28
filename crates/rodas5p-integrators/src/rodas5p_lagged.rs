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
//!
//! # Staleness bound (wave-3 node `b1c-staleness`)
//!
//! [`StalenessRule::Bounded`] (opt-in; preregistration
//! `research/b1c_staleness_20261021/PREREGISTRATION.md`) adds a refresh rule
//! and corrects the stopping test, on both stage solvers. With the
//! start-state weights `s_i = atol + rtol |y_n,i|`, `||x||_S = max_i |x_i| /
//! s_i`, the induced norm `||A||_S` and the logarithmic norm `mu_S`, Lemma
//! B1c bounds `T = I - P^{-1} W` by
//! `q_pred = (|1 - h_P/h| + h_P gamma ||J - J_P||_S) / (1 - h_P gamma mu_S(J_P))`
//! when `h_P gamma mu_S(J_P) < 1`, and `q_pred = +inf` otherwise
//! ([`rodas5p_staleness_bound`]; `+inf` also for a zero weight).
//!
//! * Before an attempt's stage solves, when `P` is valid and neither an
//!   initial nor a forced refresh is due, `q_pred` is computed from the
//!   current Jacobian and a dense copy of `J_P` kept at the last refresh
//!   (`O(n^2)`); `q_pred >` [`RODAS5P_LAGGED_STALENESS_Q_MAX`] refreshes `P`.
//! * Otherwise stage `i` stops when `||P^{-1} r_k||_S <= (1 - q_pred) rho_i`,
//!   which by the Neumann bound implies `WRMS(x_k - W^{-1} c) <= rho_i`. The
//!   contraction and progress refreshes keep their WRMS tests.
//! * Counted in [`StalenessWork`]. [`StalenessRule::Off`] (the default) is
//!   the wave-2 driver bit for bit.

#[path = "stage_target_policy.rs"]
mod stage_target_policy;
pub use stage_target_policy::{
    BoundLinearStageModel, LinearStageAssessment, LinearStageAssessmentWork, LinearStageTargetPlan,
    StageBoundCalculus,
};

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

/// Identifier of the Richardson arm with [`StalenessRule::Bounded`]
/// (wave-3 node `b1c-staleness`).
pub const RODAS5P_LAGGED_RICHARDSON_BOUNDED_DRIVER_ID: &str =
    "rodas5p-lagged-richardson-bounded-v1";

/// Identifier of the GMRES arm with [`StalenessRule::Bounded`] (wave-3 node
/// `b1c-staleness`).
pub const RODAS5P_LAGGED_GMRES_BOUNDED_DRIVER_ID: &str = "rodas5p-lagged-gmres-bounded-v1";

/// Opt-in resolvent diagnostic, with all extra inverse solves charged.
pub const RODAS5P_LAGGED_GMRES_RESOLVENT_DRIVER_ID: &str = "rodas5p-lagged-gmres-resolvent-v1";
/// Richardson variant of the opt-in resolvent diagnostic.
pub const RODAS5P_LAGGED_RICHARDSON_RESOLVENT_DRIVER_ID: &str =
    "rodas5p-lagged-richardson-resolvent-v1";

/// GMRES with the opt-in, outward diagonal resolvent check.
pub const RODAS5P_LAGGED_GMRES_DIAGONAL_DRIVER_ID: &str = "rodas5p-lagged-gmres-diagonal-v1";
/// Richardson with the opt-in, outward diagonal resolvent check.
pub const RODAS5P_LAGGED_RICHARDSON_DIAGONAL_DRIVER_ID: &str =
    "rodas5p-lagged-richardson-diagonal-v1";

/// The staleness bound above which `P` is refreshed before an attempt's
/// stage solves (T2's production `q_max`; preregistered, never tuned).
pub const RODAS5P_LAGGED_STALENESS_Q_MAX: f64 = 0.5;

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
/// RODAS5P", computed from the repository's coefficient snapshot).
/// These are diagnostic targets, not bounds on the whole left half-plane:
/// even the normal rotation at `z = i` has `|m_1| = 0.8519... > 0.58`.
/// The published negative-real-axis samples do not certify oscillatory or
/// nonlinear stage sensitivity. See the exact Fraction oracle in
/// `research/production_reaudit_20261021/evidence/integrator/`.
/// Each entry is the t-verify value (`research/production_theory_20261021/RESULTS.json`,
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

/// Whether the lagged driver checks the staleness of `P` (module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum StalenessRule {
    /// No staleness check; the wave-2 driver bit for bit.
    #[default]
    Off,
    /// Refresh when `q_pred >` [`RODAS5P_LAGGED_STALENESS_Q_MAX`], and stop
    /// stages on `||P^{-1} r||_S <= (1 - q_pred) rho_i` (Lemma B1c).
    Bounded,
    /// Use the stored absolute resolvent instead of the log-norm bound.
    /// This removes the log-norm hypothesis in exact arithmetic. Its LU
    /// evaluation uses ordinary floating point and is a diagnostic, not
    /// an outward-rounded stage or global-error certificate.
    Resolvent,
    /// Outward bound for an exactly diagonal, nonpositive stored Jacobian.
    /// Scans O(n^2) entries, applies no inverse RHS solves, and refreshes if
    /// structure or arithmetic is unproved. Only the ideal-matrix staleness
    /// bound is outward: LU/monitor arithmetic and generic stage targets
    /// remain diagnostic, so this does not promote a production certificate.
    Diagonal,
}

/// Opt-in variants of [`integrate_rodas5p_lagged_observed`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rodas5pLaggedOptions {
    /// Refactor `P` at the start of every attempt (so `P = W` and every
    /// stage is the direct solve): the equivalence arm of the fast driver.
    pub refresh_every_attempt: bool,
    /// The lagged-stage solver; Richardson by default.
    pub stage_solver: StageSolver,
    /// The staleness rule; off by default.
    pub staleness: StalenessRule,
}

/// Counted work of the lagged stage solves. Every refresh is also a
/// `direct_factorizations` count of the run's [`WorkCounters`]; every
/// application of the stored LU is a `direct_solve_calls` count there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct LaggedWork {
    /// Factorizations of `P`: the sum of the four reasons below plus
    /// [`StalenessWork::staleness_refreshes`] (zero unless
    /// [`StalenessRule::Bounded`]).
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
    /// GMRES column routines entered, including columns interrupted by an
    /// operator/preconditioner error before construction completed.
    pub gmres_column_attempts: u64,
    /// `W` applications begun inside Arnoldi columns, including failed
    /// ones. Together with residual_applications this partitions all
    /// GMRES operator applications even on errors.
    pub gmres_column_applications: u64,
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

/// Counted work of the staleness rule ([`StalenessRule::Bounded`]); all zero
/// with [`StalenessRule::Off`]. Separate from [`LaggedWork`], which derives
/// `Eq`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct StalenessWork {
    /// Bounds `q_pred` computed (at most one per attempt; none when an
    /// initial or forced refresh is due).
    pub checks: u64,
    /// Refreshes because `q_pred >` [`RODAS5P_LAGGED_STALENESS_Q_MAX`].
    pub staleness_refreshes: u64,
    /// Checks with `q_pred = +inf` (`h_P gamma mu_S(J_P) >= 1`, or a zero
    /// weight).
    pub unbounded_checks: u64,
    /// The largest finite `q_pred` of the run (0 when there was none).
    pub q_pred_max: f64,
    /// Extra LU applications to construct the resolvent (one per column
    /// on each successful refresh; failed solves stay counted).
    pub resolvent_solves: u64,
    /// Checks with an unbounded B1c estimate but resolvent estimate <= 0.5.
    pub rescued_checks: u64,
    /// Diagonal-rule checks, including refusals.
    pub diagonal_checks: u64,
    /// Matrix entry pairs inspected by diagonal-rule checks. This counts
    /// interrupted/refused scans and exposes the O(n^2) structure-check cost.
    pub diagonal_matrix_entries: u64,
    /// Diagonal-rule checks refused for structure/domain/arithmetic reasons.
    pub diagonal_refusals: u64,
}

#[derive(Clone, Debug)]
pub struct Rodas5pLaggedResult {
    /// The run in the fast driver's record (driver
    /// [`RODAS5P_LAGGED_DRIVER_ID`]).
    pub fast: Rodas5pFastResult,
    pub work: LaggedWork,
    pub staleness: StalenessWork,
}

/// Outward ideal-matrix bound on ||I-P^-1 W||_S when J_P is exactly
/// diagonal with nonpositive diagonal. This uses the diagonal inverse
/// directly (zero extra linear solves), retaining each row's cancellation:
/// max_i |1-a J_P,ii|^-1 sum_j |(1-h_P/h) delta_ij + a(J-J_P)_ij| s_j/s_i.
/// `a=h_P gamma`. Unsupported structure, zero scales and overflow give +inf.
/// This proves no floating LU backward error or nonlinear output sensitivity.
pub fn rodas5p_diagonal_staleness_bound(
    h: f64,
    h_p: f64,
    gamma: f64,
    j: &DenseMatrix,
    j_p: &DenseMatrix,
    s: &[f64],
) -> f64 {
    diagonal_staleness_counted(h, h_p, gamma, j, j_p, s).0
}

fn diagonal_staleness_counted(
    h: f64,
    h_p: f64,
    gamma: f64,
    j: &DenseMatrix,
    j_p: &DenseMatrix,
    s: &[f64],
) -> (f64, u64) {
    use rodas5p_core::directed::{Interval, add_up};
    let mut inspected = 0_u64;
    let mut compute = || -> CoreResult<f64> {
        let n = s.len();
        if n == 0
            || [j.nrows(), j.ncols(), j_p.nrows(), j_p.ncols()]
                .iter()
                .any(|v| *v != n)
            || [h, h_p, gamma].iter().any(|v| !v.is_finite() || *v <= 0.0)
            || s.iter().any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(CoreError::InvalidInput("diagonal staleness domain".into()));
        }
        let one = Interval::point(1.0)?;
        let a = Interval::point(h_p)?.mul(Interval::point(gamma)?)?;
        if a.lo <= 0.0 {
            return Err(CoreError::InvalidInput(
                "diagonal staleness step underflow".into(),
            ));
        }
        let shift = one.sub(Interval::point(h_p)?.div(Interval::point(h)?)?)?;
        let mut bound = 0.0_f64;
        for i in 0..n {
            inspected += 1;
            let diagonal = Interval::point(j_p[(i, i)])?;
            let current_diagonal = Interval::point(j[(i, i)])?;
            if diagonal.hi > 0.0 {
                return Err(CoreError::InvalidInput("positive diagonal".into()));
            }
            let inverse = one.div(one.sub(a.mul(diagonal)?)?)?;
            let mut row = 0.0;
            for k in 0..n {
                if i != k {
                    inspected += 1;
                }
                let old = if i == k {
                    diagonal
                } else {
                    Interval::point(j_p[(i, k)])?
                };
                let now = if i == k {
                    current_diagonal
                } else {
                    Interval::point(j[(i, k)])?
                };
                if i != k && j_p[(i, k)] != 0.0 {
                    return Err(CoreError::InvalidInput(
                        "stored Jacobian is not diagonal".into(),
                    ));
                }
                let mut entry = a.mul(now.sub(old)?)?;
                if i == k {
                    entry = entry.add(shift)?;
                }
                let weighted = entry
                    .mul(inverse)?
                    .mul(Interval::point(s[k])?)?
                    .div(Interval::point(s[i])?)?;
                row = add_up(row, weighted.mag())?;
            }
            bound = bound.max(row);
        }
        Ok(bound)
    };
    let result = compute().unwrap_or(f64::INFINITY);
    (result, inspected)
}

#[derive(Clone, Copy)]
enum Refresh {
    Initial,
    Forced,
    Contraction,
    Budget,
    Staleness,
}

/// Lemma B1c's bound `q_pred >= ||I - P^{-1} W||_S` for
/// `W = I/(h gamma) - J`, `P = I/(h_P gamma) - J_P` and weights `s`:
/// `(|1 - h_P/h| + h_P gamma ||J - J_P||_S) / (1 - h_P gamma mu_S(J_P))`,
/// with `||A||_S = max_i sum_j |A_ij| s_j / s_i` and
/// `mu_S(A) = max_i (A_ii + sum_{j != i} |A_ij| s_j / s_i)`. Returns `+inf`
/// when the lemma gives no bound (`h_P gamma mu_S(J_P) >= 1`), when a weight
/// is not finite and positive (ruling B2d: `s_i = 0`), and for non-positive
/// steps, mismatched shapes or non-finite entries. Never NaN. `O(n^2)`.
pub fn rodas5p_staleness_bound(
    h: f64,
    h_p: f64,
    gamma: f64,
    j: &DenseMatrix,
    j_p: &DenseMatrix,
    s: &[f64],
) -> f64 {
    let n = s.len();
    let shapes = [j.nrows(), j.ncols(), j_p.nrows(), j_p.ncols()];
    if n == 0
        || !(h > 0.0
            && h_p > 0.0
            && gamma > 0.0
            && h.is_finite()
            && h_p.is_finite()
            && gamma.is_finite())
        || shapes.iter().any(|d| *d != n)
        || !s.iter().all(|v| *v > 0.0 && v.is_finite())
        || !j
            .as_slice()
            .iter()
            .chain(j_p.as_slice())
            .all(|v| v.is_finite())
    {
        return f64::INFINITY;
    }
    let a = h_p * gamma;
    if !(a > 0.0 && a.is_finite()) {
        return f64::INFINITY;
    }
    let mut mu = f64::NEG_INFINITY;
    let mut drift = 0.0_f64;
    for i in 0..n {
        let (row, row_p) = (j.row(i), j_p.row(i));
        let mut row_mu = row_p[i];
        let mut row_drift = 0.0;
        for k in 0..n {
            let ratio = s[k] / s[i];
            // A finite input may still overflow/underflow this ratio. In
            // particular 0 * infinity makes NaN, which f64::max would
            // silently discard when reducing the rows. Refuse that bound.
            if !(ratio > 0.0 && ratio.is_finite()) {
                return f64::INFINITY;
            }
            row_drift += (row[k] - row_p[k]).abs() * ratio;
            if k != i {
                row_mu += row_p[k].abs() * ratio;
            }
        }
        if !(row_mu.is_finite() && row_drift.is_finite()) {
            return f64::INFINITY;
        }
        mu = mu.max(row_mu);
        drift = drift.max(row_drift);
    }
    let denominator = 1.0 - a * mu;
    if !denominator.is_finite() || denominator <= 0.0 {
        return f64::INFINITY;
    }
    let q = ((1.0 - h_p / h).abs() + a * drift) / denominator;
    if q.is_finite() { q } else { f64::INFINITY }
}

/// Resolvent version of the B1c estimate:
/// `||R||_S (|1-h_P/h| + h_P gamma ||J-J_P||_S)`, where
/// `R=(I-h_P gamma J_P)^(-1)` and `abs_r` supplies its absolute entries.
///
/// In exact arithmetic and for exact `abs_r`, this bounds
/// `||I-P^(-1)W||_S` without a log-norm hypothesis. This function uses
/// ordinary floating arithmetic; neither caller-supplied `abs_r` nor
/// the reductions are validated enclosures. The lagged driver therefore
/// remains opt-in, with no verified-output or production promotion.
/// Invalid shapes/weights, negative entries, or nonfinite intermediates
/// conservatively return infinity. Work and storage are O(n^2).
pub fn rodas5p_resolvent_staleness_bound(
    h: f64,
    h_p: f64,
    gamma: f64,
    j: &DenseMatrix,
    j_p: &DenseMatrix,
    abs_r: &DenseMatrix,
    s: &[f64],
) -> f64 {
    let n = s.len();
    if n == 0
        || ![h, h_p, gamma].iter().all(|x| *x > 0.0 && x.is_finite())
        || [j, j_p, abs_r]
            .iter()
            .any(|m| m.nrows() != n || m.ncols() != n)
        || !s.iter().all(|x| *x > 0.0 && x.is_finite())
        || !j
            .as_slice()
            .iter()
            .chain(j_p.as_slice())
            .all(|x| x.is_finite())
        || !abs_r.as_slice().iter().all(|x| *x >= 0.0 && x.is_finite())
    {
        return f64::INFINITY;
    }
    let a = h_p * gamma;
    if !(a > 0.0 && a.is_finite()) {
        return f64::INFINITY;
    }
    let (mut r_norm, mut drift) = (0.0_f64, 0.0_f64);
    for i in 0..n {
        let (mut r_sum, mut d_sum) = (0.0, 0.0);
        for k in 0..n {
            let ratio = s[k] / s[i];
            if !(ratio > 0.0 && ratio.is_finite()) {
                return f64::INFINITY;
            }
            r_sum += abs_r[(i, k)] * ratio;
            d_sum += (j[(i, k)] - j_p[(i, k)]).abs() * ratio;
        }
        if !(r_sum.is_finite() && d_sum.is_finite()) {
            return f64::INFINITY;
        }
        r_norm = r_norm.max(r_sum);
        drift = drift.max(d_sum);
    }
    let q = r_norm * ((1.0 - h_p / h).abs() + a * drift);
    if q.is_finite() { q } else { f64::INFINITY }
}

/// `||v||_S = max_i |v_i| / s_i`.
fn weighted_max(v: &[f64], weight: &[f64]) -> f64 {
    v.iter()
        .zip(weight)
        .map(|(x, w)| (x / w).abs())
        .fold(0.0, f64::max)
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
    /// [`StalenessRule::Bounded`]: the Jacobian `J_P` of the stored `P`
    /// (allocated once at setup; empty with `Off`), the attempt's stop
    /// factor `1 - q_pred` and the counts.
    j_p: DenseMatrix,
    abs_r: DenseMatrix,
    stop_scale: f64,
    staleness: StalenessWork,
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
            Refresh::Staleness => self.staleness.staleness_refreshes += 1,
        }
        ws.factor(h, counters)?;
        if self.bounded() {
            self.j_p
                .as_mut_slice()
                .copy_from_slice(ws.jacobian.as_slice());
        }
        if self.options.staleness == StalenessRule::Resolvent {
            let inv = 1.0 / (h * ws.gamma);
            for col in 0..ws.n {
                self.wx.fill(0.0);
                self.wx[col] = 1.0;
                self.staleness.resolvent_solves += 1;
                counters.direct_solve_calls += 1;
                ws.solve_dense_into(&mut self.wx)?;
                for row in 0..ws.n {
                    // R = P^{-1}/(h_P gamma), with P in U coordinates.
                    self.abs_r[(row, col)] = (self.wx[row] * inv).abs();
                }
            }
        }
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

    fn bounded(&self) -> bool {
        self.options.staleness != StalenessRule::Off
    }

    /// The stage stop: `Off` tests `WRMS(P^{-1} r) <= rho_i` (wave 2);
    /// `Bounded` tests `||P^{-1} r||_S <= (1 - q_pred) rho_i` (Lemma B1c).
    fn stops(&self, pres: &[f64], wrms: f64, target: f64) -> bool {
        if self.bounded() {
            weighted_max(pres, &self.weight) <= self.stop_scale * target
        } else {
            wrms <= target
        }
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
        let columns_before = self.work.gmres_iterations;
        let column_products_before = self.work.gmres_column_applications;
        let counted_products_before = counters.linear_matvecs;
        let physical_products_before = if op.jvp.is_some() {
            counters.jvp_calls
        } else {
            counters.jacobian_matvecs
        };
        let successful_pc_before = counters.preconditioner_apps;
        let outcome = self.gmres_stage(&op, &pc, target, counters);
        // Charged whether the stage converged, refreshes or failed.
        let products = op.applications.load(Ordering::Relaxed);
        let lu = pc.applications.load(Ordering::Relaxed);
        // apply_counted charges solver-level attempts before provider entry,
        // while the default provider work hook charges only successful work.
        // Reconcile the two ledgers independently before propagating an error.
        let uncharged_products = products - (counters.linear_matvecs - counted_products_before);
        counters.linear_matvecs += uncharged_products;
        counters.linear_matvec_vectors += uncharged_products;
        let physical_products_after = if op.jvp.is_some() {
            counters.jvp_calls
        } else {
            counters.jacobian_matvecs
        };
        let uncharged_physical_products =
            products - (physical_products_after - physical_products_before);
        if op.jvp.is_some() {
            counters.jvp_calls += uncharged_physical_products;
            counters.jvp_vectors += uncharged_physical_products;
        } else {
            counters.jacobian_matvecs += uncharged_physical_products;
        }
        // apply_preconditioner still charges successful applications only.
        let failed_pc = lu - (counters.preconditioner_apps - successful_pc_before);
        counters.preconditioner_apps += failed_pc;
        counters.preconditioner_vectors += failed_pc;
        self.work.operator_applications += products;
        if op.jvp.is_some() {
            self.work.jvp_applications += products;
        }
        self.work.preconditioner_applications += lu;
        counters.direct_solve_calls += lu;
        let column_products = self.work.gmres_column_applications - column_products_before;
        self.work.residual_applications += products - column_products;
        let (columns, decision) = match outcome {
            Ok(value) => value,
            Err(error) => {
                self.work.abandoned_iterations += self.work.gmres_iterations - columns_before;
                return Err(error);
            }
        };
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
        if self.stops(&self.pres, nu0, target) {
            return Ok((0, None));
        }
        let config = GmresConfig {
            restart: RODAS5P_LAGGED_GMRES_RESTART,
            max_arnoldi: RODAS5P_LAGGED_MAX_ITERATIONS,
            rtol: 0.0,
            atol: 0.0,
        };
        let mut reason = Refresh::Budget;
        let (bounded, stop_target) = (self.bounded(), self.stop_scale * target);
        let (rhs, weight, wx, pres) = (&self.rhs, &self.weight, &mut self.wx, &mut self.pres);
        let outcome = solve_gmres_into_right_monitored(
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
                let converged = if bounded {
                    weighted_max(pres, weight) <= stop_target
                } else {
                    nu <= target
                };
                if converged {
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
        );
        let progress = self.gmres.monitored_progress();
        self.work.gmres_iterations += progress.completed_columns;
        self.work.gmres_column_attempts += progress.attempted_columns;
        self.work.gmres_column_applications += progress.column_operator_applications;
        self.work.gmres_cycles += progress.cycles;
        let report = outcome?;
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
            if self.stops(&ws.stage_rhs, norm, target) {
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
        // The start-state weights (before the refresh decision, which the
        // staleness bound needs; the refreshes do not read them).
        for (w, v) in self.weight.iter_mut().zip(y) {
            *w = atol + rtol * v.abs();
        }
        self.stop_scale = 1.0;
        if !self.valid {
            self.refresh(ws, h, Refresh::Initial, counters)?;
        } else if self.options.refresh_every_attempt {
            self.refresh(ws, h, Refresh::Forced, counters)?;
        } else if self.bounded() {
            // Ruling B2c: only with a valid P and no initial or forced
            // refresh due on this attempt.
            let q_log = if self.options.staleness == StalenessRule::Diagonal {
                0.0
            } else {
                rodas5p_staleness_bound(
                    h,
                    self.p_h,
                    ws.gamma,
                    &ws.jacobian,
                    &self.j_p,
                    &self.weight,
                )
            };
            let q = if self.options.staleness == StalenessRule::Diagonal {
                let (q, entries) = diagonal_staleness_counted(
                    h,
                    self.p_h,
                    ws.gamma,
                    &ws.jacobian,
                    &self.j_p,
                    &self.weight,
                );
                self.staleness.diagonal_checks += 1;
                self.staleness.diagonal_matrix_entries += entries;
                if !q.is_finite() {
                    self.staleness.diagonal_refusals += 1;
                }
                q
            } else if self.options.staleness == StalenessRule::Resolvent {
                let q = rodas5p_resolvent_staleness_bound(
                    h,
                    self.p_h,
                    ws.gamma,
                    &ws.jacobian,
                    &self.j_p,
                    &self.abs_r,
                    &self.weight,
                );
                if !q_log.is_finite() && q <= RODAS5P_LAGGED_STALENESS_Q_MAX {
                    self.staleness.rescued_checks += 1;
                }
                q
            } else {
                q_log
            };
            self.staleness.checks += 1;
            if q.is_finite() {
                self.staleness.q_pred_max = self.staleness.q_pred_max.max(q);
            } else {
                self.staleness.unbounded_checks += 1;
            }
            if q > RODAS5P_LAGGED_STALENESS_Q_MAX {
                self.refresh(ws, h, Refresh::Staleness, counters)?;
            } else {
                self.stop_scale = if self.options.staleness == StalenessRule::Diagonal {
                    rodas5p_core::directed::sub_down(1.0, q)?
                } else {
                    1.0 - q
                };
            }
        }
        let mut exact = self.p_generation == self.generation && self.p_h == h;
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
        j_p: if options.staleness != StalenessRule::Off {
            DenseMatrix::zeros(n, n)
        } else {
            DenseMatrix::zeros(0, 0)
        },
        abs_r: if options.staleness == StalenessRule::Resolvent {
            DenseMatrix::zeros(n, n)
        } else {
            DenseMatrix::zeros(0, 0)
        },
        stop_scale: 1.0,
        staleness: StalenessWork::default(),
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
            driver: match (options.stage_solver, options.staleness) {
                (StageSolver::Richardson, StalenessRule::Off) => RODAS5P_LAGGED_DRIVER_ID,
                (StageSolver::Gmres, StalenessRule::Off) => RODAS5P_LAGGED_GMRES_DRIVER_ID,
                (StageSolver::Richardson, StalenessRule::Bounded) => {
                    RODAS5P_LAGGED_RICHARDSON_BOUNDED_DRIVER_ID
                }
                (StageSolver::Gmres, StalenessRule::Bounded) => {
                    RODAS5P_LAGGED_GMRES_BOUNDED_DRIVER_ID
                }
                (StageSolver::Richardson, StalenessRule::Resolvent) => {
                    RODAS5P_LAGGED_RICHARDSON_RESOLVENT_DRIVER_ID
                }
                (StageSolver::Gmres, StalenessRule::Resolvent) => {
                    RODAS5P_LAGGED_GMRES_RESOLVENT_DRIVER_ID
                }
                (StageSolver::Richardson, StalenessRule::Diagonal) => {
                    RODAS5P_LAGGED_RICHARDSON_DIAGONAL_DRIVER_ID
                }
                (StageSolver::Gmres, StalenessRule::Diagonal) => {
                    RODAS5P_LAGGED_GMRES_DIAGONAL_DRIVER_ID
                }
            },
        },
        work: lagged.work,
        staleness: lagged.staleness,
    })
}
