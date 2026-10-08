//! A JVP-only RODAS5P driver on the raw transformed stages (research node
//! `research/thread_transfer_mf_workspace_20261002`, thread-transfer DAG node
//! P1-MF-WORKSPACE). Research only.
//!
//! The strict matrix-free sequential path forms, in every stage after the
//! first, the `Gamma` mixture of the previous K stages and applies one JVP to
//! it for the right-hand side (seven per attempt), clones the tableau and
//! allocates stage, state, right-hand-side and report vectors per attempt.
//! This driver solves the raw U form of the same method
//! (`crate::raw_stage_target`, node P0-MF-TARGET) with `M = I`:
//!
//! ```text
//! (I - h gamma J) U_i = h gamma f(t + c_i h, y + sum_j A_ij U_j)
//!                       + gamma sum_j C_ij U_j + h^2 gamma gamma_i f_t,
//! y_new = y + sum_j b_code_j U_j,   embedded error U_(s-1),
//! ```
//!
//! so `J` is applied only inside the Krylov solver, through the same counted
//! JVP shifted operator. It keeps:
//!
//! * strict matrix-free semantics: a user JVP is required, and a direct
//!   solve, a direct or Jacobi preconditioner (both need an explicit matrix)
//!   or a mass matrix is refused;
//! * the configured GMRES, LGMRES or GCRO-DR with their true-residual check;
//!   the K path's relative tolerance applies to the U right-hand side's own
//!   norm, the absolute one is mapped to `|gamma| atol`
//!   ([`crate::raw_absolute_residual_budget`]);
//! * the Krylov recycle state, snapshotted before an attempt and restored on
//!   rejection, as in the sequential path;
//! * the sequential driver's step-size controller, represented-clock and
//!   output rules, unchanged.
//!
//! One workspace (stage vectors, Krylov workspaces, coefficient rows) serves
//! the whole integration; `f(t, y)`, `f_t` and the JVP operator are reused
//! after a rejected attempt from the same state. There is no inner forcing:
//! the protected driver's WRMS forcing and refinement passes are not ported.

use std::sync::{Arc, Mutex};

use faer::c64;
use rodas5p_core::LinearSolveReport;
use rodas5p_core::Preconditioner;
use rodas5p_core::{
    CoreError, CoreResult, IdentityPreconditioner, InitialGuess, LinearMethod, LinearOperator,
    LinearSolverConfig, OperatorApplicationWork, PreconditionerKind, ShiftedOperator, WorkCounters,
    rodas5p_coefficients, safe_l2,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrSolveOptions, GcrodrState, GcrodrWorkspace, GmresCapacity, GmresConfig,
    GmresWorkspace, LgmresConfig, LgmresWorkspace, StagedGmresConfig, StagedGmresFailure,
    StagedGmresOutcome, StagedGmresReport, StagedGmresWorkspace, StagedGuardAbort,
    solve_gcrodr_with_workspace, solve_gcrodr_with_workspace_and_options, solve_gmres_into,
    solve_gmres_with_workspace, solve_lgmres_with_workspace, solve_staged_gmres,
};
use serde::Serialize;

use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, KrylovState,
    ObservedIntegrationResult, OdeProblem, OutputSchedule, output::OutputCollector,
    problem::MatrixFreeCallbackIdentity, raw_absolute_residual_budget,
    rodas_next_step_after_attempt,
};

/// Identifier of this driver in research records.
pub const RODAS5P_MF_FAST_DRIVER_ID: &str = "rodas5p-mf-fast-transformed-v1";

/// How the U-form driver's GCRO-DR stage solves treat the carried recycle
/// pair (RVJ DAG node SAFE-RECYCLE, `research/safe_recycle_policy_20261004`).
/// `Legacy` is the default and the call the driver always made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GcrodrRecyclePolicy {
    /// Carried pair, no refresh after a recycle update (the pre-existing
    /// behaviour; L-0059 found it breaks `M^-1 A U = C`).
    #[default]
    Legacy,
    /// Carried pair, `C = M^-1 A U` recomputed (charged) and re-orthonormalized
    /// after every recycle update (REV-01c, L-0059).
    RefreshAfterUpdate,
    /// Every stage solve starts from an empty recycle state; the carried
    /// state is never changed.
    Cold,
}

impl GcrodrRecyclePolicy {
    /// Stable identifier for research records.
    pub fn id(self) -> &'static str {
        match self {
            Self::Legacy => "gcrodr-recycle-legacy-v1",
            Self::RefreshAfterUpdate => "gcrodr-recycle-refresh-after-update-v1",
            Self::Cold => "gcrodr-cold-v1",
        }
    }
}

/// One GCRO-DR stage solve under `policy`, exactly as the U-form driver
/// makes it.
#[allow(clippy::too_many_arguments)]
pub fn gcrodr_stage_solve(
    policy: GcrodrRecyclePolicy,
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GcrodrConfig,
    state: &mut GcrodrState,
    workspace: &mut GcrodrWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    match policy {
        GcrodrRecyclePolicy::Legacy => {
            solve_gcrodr_with_workspace(op, pc, rhs, x0, config, state, workspace, counters)
        }
        GcrodrRecyclePolicy::RefreshAfterUpdate => solve_gcrodr_with_workspace_and_options(
            op,
            pc,
            rhs,
            x0,
            config,
            state,
            None,
            workspace,
            GcrodrSolveOptions {
                refresh_after_update: true,
                ..GcrodrSolveOptions::default()
            },
            counters,
        ),
        GcrodrRecyclePolicy::Cold => {
            let mut cold = GcrodrState::default();
            solve_gcrodr_with_workspace(op, pc, rhs, x0, config, &mut cold, workspace, counters)
        }
    }
}

/// The stage-target policy of the U-form driver's GMRES stage solves
/// (research node `research/alg01_coupled_stage_target_20261008`, ALG01).
/// `Legacy` is the default and the solve the driver always made; every
/// other policy uses the staged solver [`solve_staged_gmres`] (zero start,
/// in-cycle projected exit confirmed by one true residual, no duplicate final
/// residual) and needs GMRES without a preconditioner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum StageTargetPolicy {
    /// Today's solver and arithmetic, bit for bit.
    #[default]
    Legacy,
    /// The production L2 target (linear rtol, `|gamma| atol_lin`) with
    /// in-cycle exit.
    ProjL2,
    /// `rtol_lin = min(1e-10, 1e-3 rtol)`, `atol_lin = |gamma| 1e-3 atol`,
    /// with in-cycle exit (the judge's L2 rival).
    L2Coupled,
    /// The coupled WRMS target on `D W D^-1 z = D b` with the stall rule.
    Coupled,
    /// `Coupled` plus the nonnormality guard and the production fallback.
    CoupledGuarded,
}

impl StageTargetPolicy {
    /// Stable identifier for research records.
    pub fn id(self) -> &'static str {
        match self {
            Self::Legacy => "stage-target-legacy-v1",
            Self::ProjL2 => "stage-target-proj-l2-v1",
            Self::L2Coupled => "stage-target-l2-coupled-v1",
            Self::Coupled => "stage-target-coupled-v1",
            Self::CoupledGuarded => "stage-target-coupled-guarded-v1",
        }
    }

    fn is_coupled(self) -> bool {
        matches!(self, Self::Coupled | Self::CoupledGuarded)
    }
}

/// A stage-target policy with the ALG03 switches (research node
/// `research/alg03_stage_budget_guard_20261008`). The Krylov budget is the
/// linear configuration's `maxiter`, as for `Legacy`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct StageTargetOptions {
    pub policy: StageTargetPolicy,
    /// The staged solver's stagnation guard (ALG03).
    pub stagnation_guard: bool,
    /// Accept, on a guard abort or budget exhaustion, an iterate whose
    /// unscaled true residual meets the production rule
    /// `||b - W x||_2 <= max(|gamma| atol_lin, rtol_lin ||b||_2)`.
    pub production_fallback: bool,
    /// Classify guard aborts by an uncounted shadow continuation (reported
    /// only; it changes nothing in the run).
    pub classify_guard_aborts: bool,
}

impl StageTargetOptions {
    /// `policy` as registered: the production fallback is part of
    /// `CoupledGuarded` only; no stagnation guard.
    pub fn new(policy: StageTargetPolicy) -> Self {
        Self {
            policy,
            stagnation_guard: false,
            production_fallback: policy == StageTargetPolicy::CoupledGuarded,
            classify_guard_aborts: false,
        }
    }
}

/// Global contamination budget `Theta` of the coupled target.
pub const COUPLED_TARGET_THETA: f64 = 0.2;
/// Reference error and exponent of the order factor `min(1, e/e_ref)^p`.
pub const COUPLED_TARGET_ERROR_REFERENCE: f64 = 0.5;
pub const COUPLED_TARGET_ORDER_EXPONENT: f64 = 6.0 / 5.0;
/// `e_hat` before the first accepted step.
pub const COUPLED_TARGET_INITIAL_ERROR: f64 = 1.0e-6;
/// The U8 cap `eps_8 <= 0.1 e_sat` with `e_sat = (0.9/5)^5`.
pub const COUPLED_TARGET_U8_FRACTION: f64 = 0.1;

/// `0.1 (0.9/5)^5`, the cap of the last stage's target.
pub fn coupled_target_u8_cap() -> f64 {
    COUPLED_TARGET_U8_FRACTION * (0.9_f64 / 5.0).powi(5)
}
/// Round-off guard of the scaled threshold, relative to `||D b||_2`.
pub const COUPLED_TARGET_ROUNDOFF: f64 = 16.0 * f64::EPSILON;
/// The judge's L2 coupling factors.
pub const L2_COUPLED_FACTOR: f64 = 1.0e-3;
pub const L2_COUPLED_RTOL_CAP: f64 = 1.0e-10;

/// The tableau's residual-to-output and residual-to-estimate transfer
/// constants `tau_y,i = sup |T_y,i(z)|`, `tau_e,i = sup |T_e,i(z)|` over
/// `Re z <= 0`, with `L(z) = (1 - gamma z) I - gamma (z A + C)`,
/// `T_y = b_code^T L^-1`, `T_e = e_s^T L^-1` (pilot
/// `phase2_code/tau_table.py`). The supremum is taken on the imaginary axis
/// (maximum modulus) over the pilot's grid `z = i y`,
/// `y in {0} u 10^[-3, 5]` (1500 log-spaced points; `|T(-iy)| = |T(iy)|`
/// for the real tableau).
#[derive(Clone, Debug, PartialEq)]
pub struct StageTransferConstants {
    pub tau_y: Vec<f64>,
    pub tau_e: Vec<f64>,
}

/// [`StageTransferConstants`] from the coefficient snapshot.
pub fn rodas5p_stage_transfer_constants() -> CoreResult<StageTransferConstants> {
    let coeffs = rodas5p_coefficients()?;
    let s = coeffs.stages();
    let gamma = coeffs.gamma;
    let mut tau_y = vec![0.0_f64; s];
    let mut tau_e = vec![0.0_f64; s];
    let mut w = vec![c64::new(0.0, 0.0); s];
    let solve = |z: c64, u: &dyn Fn(usize) -> f64, w: &mut [c64], tau: &mut [f64]| {
        // L^T w = u (L is lower triangular with diagonal 1 - gamma z).
        let diagonal = c64::new(1.0, 0.0) - z * gamma;
        for j in (0..s).rev() {
            let mut acc = c64::new(u(j), 0.0);
            for (i, wi) in w.iter().enumerate().skip(j + 1) {
                let l_ij = -(z * coeffs.a[(i, j)] + c64::new(coeffs.c_matrix[(i, j)], 0.0)) * gamma;
                acc -= l_ij * wi;
            }
            w[j] = acc / diagonal;
        }
        for (t, wj) in tau.iter_mut().zip(w.iter()) {
            *t = t.max(wj.norm());
        }
    };
    let b_code = coeffs.b_code.clone();
    let last = s - 1;
    let grid =
        std::iter::once(0.0).chain((0..1500).map(|k| 10f64.powf(-3.0 + 8.0 * k as f64 / 1499.0)));
    for y in grid {
        let z = c64::new(0.0, y);
        solve(z, &|j| b_code[j], &mut w, &mut tau_y);
        solve(
            z,
            &|j| if j == last { 1.0 } else { 0.0 },
            &mut w,
            &mut tau_e,
        );
    }
    Ok(StageTransferConstants { tau_y, tau_e })
}

/// The staged stage solves of one integration, by how they ended (all
/// zero for `Legacy`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct StageSolveStatistics {
    pub solves: u64,
    pub converged: u64,
    pub stall_accepted: u64,
    pub fallback_accepted: u64,
    /// Fallback acceptances after a guard abort (the rest followed budget
    /// exhaustion).
    pub fallback_after_guard: u64,
    pub failed: u64,
    pub failed_budget: u64,
    pub failed_guard: u64,
    pub failed_nonfinite: u64,
    pub failed_breakdown: u64,
    /// Solves that reached the column budget (accepted by the fallback or
    /// failed).
    pub budget_exhausted: u64,
    pub guard_contraction: u64,
    pub guard_overrun: u64,
    /// Classified guard aborts: the shadow continuation converged (false)
    /// or not (true).
    pub guard_false: u64,
    pub guard_true: u64,
    pub columns: u64,
    pub max_columns: u64,
    pub cycles: u64,
    pub true_residuals: u64,
    pub confirmations: u64,
    pub failed_confirmations: u64,
    pub nu_evaluations: u64,
    /// Solves whose threshold the nonnormality guard tightened.
    pub nu_tightened: u64,
    pub nu_max: f64,
    pub nu_flops: u64,
    /// Coupled solves whose threshold was the round-off guard
    /// `16 eps ||D b||` rather than `eps_i sqrt(n)`.
    pub roundoff_floor_binds: u64,
}

impl StageSolveStatistics {
    fn record(&mut self, report: &StagedGmresReport) {
        self.solves += 1;
        match report.outcome {
            StagedGmresOutcome::Converged => self.converged += 1,
            StagedGmresOutcome::StallAccepted => self.stall_accepted += 1,
            StagedGmresOutcome::FallbackAccepted => {
                self.fallback_accepted += 1;
                if report.guard_abort.is_some() {
                    self.fallback_after_guard += 1;
                }
            }
            StagedGmresOutcome::Failed => self.failed += 1,
        }
        match report.failure {
            Some(StagedGmresFailure::BudgetExhausted) => self.failed_budget += 1,
            Some(StagedGmresFailure::GuardContraction | StagedGmresFailure::GuardOverrun) => {
                self.failed_guard += 1
            }
            Some(StagedGmresFailure::NonFinite) => self.failed_nonfinite += 1,
            Some(StagedGmresFailure::Breakdown) => self.failed_breakdown += 1,
            None => {}
        }
        if report.budget_exhausted {
            self.budget_exhausted += 1;
        }
        match report.guard_abort {
            Some(StagedGuardAbort::Contraction) => self.guard_contraction += 1,
            Some(StagedGuardAbort::Overrun) => self.guard_overrun += 1,
            None => {}
        }
        match report.shadow_converged {
            Some(true) => self.guard_false += 1,
            Some(false) => self.guard_true += 1,
            None => {}
        }
        self.columns += report.columns;
        self.max_columns = self.max_columns.max(report.columns);
        self.cycles += report.cycles;
        self.true_residuals += report.true_residuals;
        self.confirmations += report.confirmations;
        self.failed_confirmations += report.failed_confirmations;
        self.nu_evaluations += report.nu_evaluations;
        if report.nu_max > 1.0 {
            self.nu_tightened += 1;
        }
        self.nu_max = self.nu_max.max(report.nu_max);
        self.nu_flops += report.nu_flops;
    }
}

/// `D W D^-1` with `D = diag(weight)` and `D^-1 = diag(scale)`
/// (`scale_i = atol + rtol |y_i|`, `weight_i = 1 / scale_i`). One
/// application is one application of `W` (charged by `W` itself, for
/// example one JVP); the two diagonal scalings are uncounted vector work.
struct DiagonalScaledOperator<'a> {
    inner: &'a dyn LinearOperator,
    weight: &'a [f64],
    scale: &'a [f64],
    scratch: Mutex<Vec<f64>>,
}

impl LinearOperator for DiagonalScaledOperator<'_> {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let mut scratch = self
            .scratch
            .lock()
            .map_err(|_| CoreError::InvalidInput("scaled operator scratch poisoned".into()))?;
        for ((t, xi), si) in scratch.iter_mut().zip(x).zip(self.scale) {
            *t = xi * si;
        }
        self.inner.apply(&scratch, y)?;
        for (yi, wi) in y.iter_mut().zip(self.weight) {
            *yi *= wi;
        }
        Ok(())
    }

    fn application_work(&self) -> OperatorApplicationWork {
        self.inner.application_work()
    }

    fn exact_identity(&self) -> Option<rodas5p_core::ExactOperatorIdentity> {
        None
    }

    fn token(&self) -> u64 {
        self.inner.token() ^ 0x5ca1_ed00_0000_0000
    }
}

#[derive(Clone, Debug)]
pub struct Rodas5pMfFastResult {
    pub observed: ObservedIntegrationResult,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    /// Attempts that reused `f(t, y)`, `f_t` and the JVP operator of a
    /// rejected attempt from the same state.
    pub state_reuses: usize,
    pub driver: &'static str,
    /// [`GcrodrRecyclePolicy::id`] of the run (meaningful for GCRO-DR).
    pub gcrodr_policy: &'static str,
}

/// The stage arithmetic and Krylov workspaces of one integration.
pub struct Rodas5pMfFastWorkspace {
    n: usize,
    s: usize,
    gamma: f64,
    c: Vec<f64>,
    gamma_rows: Vec<f64>,
    a_nonzero: Vec<Vec<(usize, f64)>>,
    c_nonzero: Vec<Vec<(usize, f64)>>,
    b_nonzero: Vec<(usize, f64)>,
    config: LinearSolverConfig,
    preconditioner: IdentityPreconditioner,
    gmres: GmresWorkspace,
    lgmres: LgmresWorkspace,
    gcrodr: GcrodrWorkspace,
    jvp: Option<Arc<dyn LinearOperator>>,
    cached_callbacks: Option<MatrixFreeCallbackIdentity>,
    cached_t_bits: u64,
    cached_y: Vec<f64>,
    cached_epoch: Option<u64>,
    /// Research switch (R-NEXT-02): GMRES stage solves through
    /// `solve_gmres_into`, writing into the stage storage.
    gmres_into: bool,
    /// Research switch (SAFE-RECYCLE): the GCRO-DR recycle policy.
    gcrodr_policy: GcrodrRecyclePolicy,
    /// Stages and step of the last accepted step, kept only for the
    /// step-indexed initial guesses (research node
    /// `research/spd07_mf_step_warm_start_20261007`); `step_h` is zero until
    /// a step was accepted.
    step_u: Vec<f64>,
    step_h: f64,
    x0_scaled: Vec<f64>,
    /// GMRES-into restart cycles of all stage solves so far (reported by
    /// research node SPD07; nothing reads it).
    gmres_into_cycles: u64,
    f0: Vec<f64>,
    ft: Vec<f64>,
    u: Vec<f64>,
    stage_state: Vec<f64>,
    stage_rhs: Vec<f64>,
    y_new: Vec<f64>,
    /// Research switch (ALG01/ALG03): the stage-target policy. Everything
    /// below is used only by the staged policies.
    stage_target: StageTargetOptions,
    /// `max(tau_y,i, tau_e,i)` per stage (coupled policies).
    stage_tau: Vec<f64>,
    /// The integration span `T` of the coupled target.
    stage_span: f64,
    /// `e_hat`: the embedded error of the last accepted step.
    last_accepted_error: Option<f64>,
    staged: StagedGmresWorkspace,
    stage_eps: Vec<f64>,
    staged_scale: Vec<f64>,
    staged_weight: Vec<f64>,
    staged_rhs: Vec<f64>,
    staged_z: Vec<f64>,
    stage_statistics: StageSolveStatistics,
}

fn validate_strict(problem: &OdeProblem, config: &LinearSolverConfig) -> CoreResult<()> {
    config.validate().map_err(CoreError::InvalidInput)?;
    if config.method == LinearMethod::Direct
        || matches!(
            config.preconditioner,
            PreconditionerKind::Direct | PreconditionerKind::Jacobi
        )
    {
        return Err(CoreError::InvalidInput(
            "strict matrix-free RODAS5P (U form) forbids direct solves and explicit-matrix preconditioners".into(),
        ));
    }
    if !problem.supports_matrix_free_jvp() {
        return Err(CoreError::InvalidInput(
            "strict matrix-free integration requires a user-supplied JVP".into(),
        ));
    }
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "the matrix-free U-form driver supports the identity mass matrix only".into(),
        ));
    }
    Ok(())
}

impl Rodas5pMfFastWorkspace {
    pub fn new(problem: &OdeProblem, config: &LinearSolverConfig) -> CoreResult<Self> {
        validate_strict(problem, config)?;
        let n = problem.dimension;
        let coeffs = rodas5p_coefficients()?;
        let s = coeffs.stages();
        for i in 0..s {
            for j in i..s {
                if coeffs.a[(i, j)] != 0.0 || coeffs.c_matrix[(i, j)] != 0.0 {
                    return Err(CoreError::Coefficients(
                        "RODAS5P transformed coefficients are not strictly lower triangular".into(),
                    ));
                }
            }
        }
        let nonzero_row = |m: &rodas5p_core::DenseMatrix, i: usize| {
            (0..i)
                .filter(|&j| m[(i, j)] != 0.0)
                .map(|j| (j, m[(i, j)]))
                .collect::<Vec<_>>()
        };
        Ok(Self {
            n,
            s,
            gamma: coeffs.gamma,
            c: coeffs.c.clone(),
            gamma_rows: coeffs.gamma_rows.clone(),
            a_nonzero: (0..s).map(|i| nonzero_row(&coeffs.a, i)).collect(),
            c_nonzero: (0..s).map(|i| nonzero_row(&coeffs.c_matrix, i)).collect(),
            b_nonzero: coeffs
                .b_code
                .iter()
                .enumerate()
                .filter(|(_, b)| **b != 0.0)
                .map(|(j, b)| (j, *b))
                .collect(),
            config: config.clone(),
            preconditioner: IdentityPreconditioner::new(n),
            gmres: GmresWorkspace::default(),
            lgmres: LgmresWorkspace::default(),
            gcrodr: GcrodrWorkspace::default(),
            jvp: None,
            cached_callbacks: None,
            cached_t_bits: 0,
            cached_y: vec![0.0; n],
            cached_epoch: None,
            gmres_into: false,
            gcrodr_policy: GcrodrRecyclePolicy::Legacy,
            step_u: Vec::new(),
            step_h: 0.0,
            x0_scaled: Vec::new(),
            gmres_into_cycles: 0,
            f0: vec![0.0; n],
            ft: vec![0.0; n],
            u: vec![0.0; s * n],
            stage_state: vec![0.0; n],
            stage_rhs: vec![0.0; n],
            y_new: vec![0.0; n],
            stage_target: StageTargetOptions::default(),
            stage_tau: Vec::new(),
            stage_span: 0.0,
            last_accepted_error: None,
            staged: StagedGmresWorkspace::default(),
            stage_eps: Vec::new(),
            staged_scale: Vec::new(),
            staged_weight: Vec::new(),
            staged_rhs: Vec::new(),
            staged_z: Vec::new(),
            stage_statistics: StageSolveStatistics::default(),
        })
    }

    /// Set the stage-target policy (research nodes ALG01/ALG03). `Legacy`
    /// by default. The staged policies need GMRES without a preconditioner;
    /// the coupled ones compute the transfer constants here, from the
    /// coefficient snapshot, and need [`Self::set_stage_target_span`].
    pub fn set_stage_target(&mut self, options: StageTargetOptions) -> CoreResult<()> {
        if options.policy != StageTargetPolicy::Legacy {
            if self.config.method != LinearMethod::Gmres
                || self.config.preconditioner != PreconditionerKind::None
            {
                return Err(CoreError::InvalidInput(
                    "the staged stage targets need GMRES without a preconditioner".into(),
                ));
            }
            if options.policy.is_coupled() && self.stage_tau.is_empty() {
                let tau = rodas5p_stage_transfer_constants()?;
                self.stage_tau = tau
                    .tau_y
                    .iter()
                    .zip(&tau.tau_e)
                    .map(|(a, b)| a.max(*b))
                    .collect();
            }
        }
        self.stage_target = options;
        Ok(())
    }

    pub fn stage_target(&self) -> StageTargetOptions {
        self.stage_target
    }

    /// The integration span `T` of the coupled target's per-unit-step
    /// budget (the driver passes `t_end - t_0`).
    pub fn set_stage_target_span(&mut self, span: f64) {
        self.stage_span = span;
    }

    /// Record the embedded error of an accepted step (`e_hat` of the
    /// coupled target's order factor).
    pub fn record_accepted_error(&mut self, error: f64) {
        self.last_accepted_error = Some(error);
    }

    /// Forget `e_hat` (the start of a new integration).
    pub fn clear_accepted_error(&mut self) {
        self.last_accepted_error = None;
    }

    /// The staged stage solves of this workspace so far.
    pub fn stage_statistics(&self) -> StageSolveStatistics {
        self.stage_statistics
    }

    /// The coupled targets `eps_i` (tolerance units) of the last attempt.
    pub fn stage_eps(&self) -> &[f64] {
        &self.stage_eps
    }

    /// The coupled targets of an attempt with step `h`:
    /// `eps_i = theta_n / (8 max(tau_y,i, tau_e,i))`,
    /// `theta_n = Theta (|h| / T) min(1, e_hat / 0.5)^(6/5)`,
    /// `eps_8 <= 0.1 (0.9/5)^5`.
    fn coupled_eps(&mut self, h: f64) -> CoreResult<()> {
        let span = self.stage_span;
        if !(span.is_finite() && span > 0.0) {
            return Err(CoreError::InvalidInput(
                "the coupled stage target needs a positive finite span".into(),
            ));
        }
        let e = self
            .last_accepted_error
            .unwrap_or(COUPLED_TARGET_INITIAL_ERROR);
        let order = (e.max(0.0) / COUPLED_TARGET_ERROR_REFERENCE)
            .min(1.0)
            .powf(COUPLED_TARGET_ORDER_EXPONENT);
        let theta = COUPLED_TARGET_THETA * (h.abs() / span) * order;
        let s = self.s;
        self.stage_eps.clear();
        self.stage_eps
            .extend(self.stage_tau.iter().map(|tau| theta / (s as f64 * tau)));
        let last = &mut self.stage_eps[s - 1];
        *last = last.min(coupled_target_u8_cap());
        Ok(())
    }

    /// Stage `i` with a staged policy: the solve of `W U_i = stage_rhs`
    /// (or of the scaled system) into `u[i]`.
    #[allow(clippy::too_many_arguments)]
    fn staged_stage_solve(
        &mut self,
        i: usize,
        shifted: &ShiftedOperator,
        linear_atol: f64,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        let n = self.n;
        let options = self.stage_target;
        let budget = self.config.maxiter.max(self.config.restart);
        let production_rtol = self.config.rtol;
        let rhs_norm = safe_l2(&self.stage_rhs);
        let production_threshold = linear_atol.max(production_rtol * rhs_norm);
        let mut config = StagedGmresConfig {
            stagnation_guard: options.stagnation_guard,
            classify_guard_aborts: options.classify_guard_aborts,
            ..StagedGmresConfig::new(self.config.restart, budget, production_rtol, linear_atol)
        };
        let stage = &mut self.u[i * n..(i + 1) * n];
        let report = if options.policy.is_coupled() {
            let eps = self.stage_eps[i];
            config.atol = eps * (n as f64).sqrt();
            config.rtol = COUPLED_TARGET_ROUNDOFF;
            config.stall_rule = true;
            config.nu_guard = options.policy == StageTargetPolicy::CoupledGuarded;
            for ((d, w), b) in self
                .staged_rhs
                .iter_mut()
                .zip(&self.staged_weight)
                .zip(&self.stage_rhs)
            {
                *d = w * b;
            }
            let scaled = DiagonalScaledOperator {
                inner: shifted,
                weight: &self.staged_weight,
                scale: &self.staged_scale,
                scratch: Mutex::new(vec![0.0; n]),
            };
            let scale = &self.staged_scale;
            let mut fallback = |_z: &[f64], scaled_residual: &[f64]| {
                let unscaled: Vec<f64> = scaled_residual
                    .iter()
                    .zip(scale)
                    .map(|(r, sc)| r * sc)
                    .collect();
                safe_l2(&unscaled) <= production_threshold
            };
            let report = solve_staged_gmres(
                &scaled,
                &self.staged_rhs,
                &config,
                options
                    .production_fallback
                    .then_some(&mut fallback as &mut dyn FnMut(&[f64], &[f64]) -> bool),
                &mut self.staged_z,
                &mut self.staged,
                counters,
            )?;
            if report.threshold > config.atol {
                self.stage_statistics.roundoff_floor_binds += 1;
            }
            if report.accepted() {
                for ((x, z), sc) in stage.iter_mut().zip(&self.staged_z).zip(scale) {
                    *x = z * sc;
                }
            }
            report
        } else {
            if options.policy == StageTargetPolicy::L2Coupled {
                config.rtol = L2_COUPLED_RTOL_CAP.min(L2_COUPLED_FACTOR * rtol);
                config.atol = self.gamma.abs() * L2_COUPLED_FACTOR * atol;
            }
            let mut fallback =
                |_x: &[f64], residual: &[f64]| safe_l2(residual) <= production_threshold;
            solve_staged_gmres(
                shifted,
                &self.stage_rhs,
                &config,
                options
                    .production_fallback
                    .then_some(&mut fallback as &mut dyn FnMut(&[f64], &[f64]) -> bool),
                stage,
                &mut self.staged,
                counters,
            )?
        };
        self.stage_statistics.record(&report);
        if !report.accepted() {
            return Err(CoreError::LinearSolve(format!(
                "staged GMRES stage solve failed ({:?}) after {} columns, true residual \
                 {:.3e} > {:.3e}",
                report.failure, report.columns, report.residual_norm, report.final_threshold
            )));
        }
        if !self.u[i * n..(i + 1) * n]
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(CoreError::NonFinite(
                "RODAS5P U-form stage solve produced NaN/Inf".into(),
            ));
        }
        Ok(())
    }

    /// The new state of the last attempt.
    /// Route GMRES stage solves through `solve_gmres_into` (research node
    /// `research/rnext02_gmres_into_20261003`); same results bit for bit,
    /// fewer allocations. Off by default.
    pub fn set_gmres_into(&mut self, on: bool) {
        self.gmres_into = on;
    }

    /// Solve the small least-squares problem of the `gmres_into` stage
    /// solves in a reused workspace (research node
    /// `research/spd04_ls_workspace_20261007`); same results bit for bit.
    /// Off by default; effective only together with
    /// [`Self::set_gmres_into`] (the other stage solvers never use it).
    pub fn set_ls_workspace(&mut self, on: bool) {
        self.gmres.set_ls_workspace(on);
    }

    /// Set the GCRO-DR recycle policy (research node
    /// `research/safe_recycle_policy_20261004`). `Legacy` by default.
    pub fn set_gcrodr_policy(&mut self, policy: GcrodrRecyclePolicy) {
        self.gcrodr_policy = policy;
    }

    pub fn gcrodr_policy(&self) -> GcrodrRecyclePolicy {
        self.gcrodr_policy
    }

    /// Record the stages of the attempt just accepted with step `h`, the
    /// start of the step-indexed initial guesses. A no-op for every other
    /// initial guess, so the default runs neither copy nor allocate.
    pub fn record_accepted_step(&mut self, h: f64) {
        if self.config.x0_strategy.is_step_indexed() {
            self.step_u.clone_from(&self.u);
            self.step_h = h;
        }
    }

    /// Forget the recorded step, so the next attempt starts as `Previous`
    /// (the start of a new integration with a reused workspace).
    pub fn clear_accepted_step(&mut self) {
        self.step_h = 0.0;
    }

    /// GMRES-into restart cycles (small least-squares solves) of all
    /// stage solves of this workspace so far.
    pub fn gmres_into_cycles(&self) -> u64 {
        self.gmres_into_cycles
    }

    pub fn y_new(&self) -> &[f64] {
        &self.y_new
    }

    /// Stage `i` of the last attempt.
    pub fn stage(&self, i: usize) -> &[f64] {
        &self.u[i * self.n..(i + 1) * self.n]
    }

    /// One attempt from `(t, y)` with step `h`: the new state in
    /// [`Self::y_new`] and the WRMS norm of the embedded error `U_(s-1)` in
    /// the sequential path's scale `atol + rtol max(|y|, |y_new|)`. `fresh`
    /// evaluates `f(t, y)`, `f_t` and the JVP operator at this state;
    /// otherwise reuse is allowed only for the exact same time/state bits and
    /// retained callback identities. Changing `h` alone does not invalidate
    /// the frozen state. Interior callback-data changes require `fresh = true`,
    /// unless the problem carries a model epoch
    /// ([`OdeProblem::with_model_epoch`]): the epoch is read once per attempt
    /// and any change rebuilds `f(t, y)`, `f_t` and the operator (whose new
    /// token makes a carried recycle state refresh its images).
    /// A failed refresh invalidates all frozen data before any retry.
    #[allow(clippy::too_many_arguments)]
    pub fn attempt(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        fresh: bool,
        recycle: Option<&mut KrylovState>,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        let (n, s, gamma) = (self.n, self.s, self.gamma);
        validate_strict(problem, &self.config)?;
        if problem.dimension != n {
            return Err(CoreError::Dimension(
                "matrix-free workspace/problem dimensions differ".into(),
            ));
        }
        if !(t.is_finite() && atol.is_finite() && atol >= 0.0 && rtol.is_finite() && rtol >= 0.0) {
            return Err(CoreError::InvalidInput(
                "time must be finite and output tolerances finite and nonnegative".into(),
            ));
        }
        if !(h.is_finite() && h != 0.0) {
            return Err(CoreError::InvalidInput(
                "step size must be finite and nonzero".into(),
            ));
        }
        if y.len() != n || !y.iter().all(|value| value.is_finite()) {
            return Err(CoreError::InvalidInput("invalid initial state".into()));
        }
        let epoch = problem.model_epoch();
        let same_state = self.cached_t_bits == t.to_bits()
            && self.cached_epoch == epoch
            && self
                .cached_y
                .iter()
                .zip(y)
                .all(|(a, b)| a.to_bits() == b.to_bits())
            && self
                .cached_callbacks
                .as_ref()
                .is_some_and(|id| id.matches(problem));
        if fresh || self.jvp.is_none() || !same_state {
            // Invalidate before fallible callbacks: a partially written RHS or
            // failed f_t must never be paired with the old state's operator.
            self.jvp = None;
            self.cached_callbacks = None;
            problem.eval_rhs_into(t, y, &mut self.f0, counters)?;
            if problem.autonomous {
                self.ft.fill(0.0);
            } else {
                let ft = problem.eval_partial_t(t, y, counters)?;
                self.ft.copy_from_slice(&ft);
            }
            let jvp = problem.linearize_matrix_free(t, y)?;
            self.cached_y.copy_from_slice(y);
            self.cached_t_bits = t.to_bits();
            self.cached_epoch = epoch;
            self.cached_callbacks = Some(problem.matrix_free_callback_identity());
            self.jvp = Some(jvp);
        }
        let jvp = self.jvp.clone().expect("linearized above");
        let shifted = ShiftedOperator::new_counted_jvp(None, jvp, h, gamma)?;
        debug_assert!(shifted.explicit().is_none());
        let linear_atol = if self.config.atol == 0.0 {
            0.0
        } else {
            raw_absolute_residual_budget(self.config.atol, gamma)?
        };
        let mut owned = KrylovState::for_method(self.config.method);
        let state = match recycle {
            Some(state) => Some(state),
            None => owned.as_mut(),
        };
        let mut state = state;
        let hg = h * gamma;
        let staged = self.stage_target.policy != StageTargetPolicy::Legacy;
        if staged {
            self.stage_eps.clear();
            if self.stage_target.policy.is_coupled() {
                self.coupled_eps(h)?;
                self.staged_scale.clear();
                self.staged_weight.clear();
                for v in y {
                    let scale = atol + rtol * v.abs();
                    if !(scale.is_finite() && scale > 0.0) {
                        return Err(CoreError::InvalidInput(
                            "the coupled stage target needs finite positive error weights".into(),
                        ));
                    }
                    self.staged_scale.push(scale);
                    self.staged_weight.push(1.0 / scale);
                }
                self.staged_rhs.resize(n, 0.0);
                self.staged_z.resize(n, 0.0);
            }
        }
        for i in 0..s {
            if i == 0 {
                self.stage_rhs.copy_from_slice(&self.f0);
            } else {
                self.stage_state.copy_from_slice(y);
                for &(j, aij) in &self.a_nonzero[i] {
                    let uj = &self.u[j * n..(j + 1) * n];
                    for (x, v) in self.stage_state.iter_mut().zip(uj) {
                        *x += aij * v;
                    }
                }
                problem.eval_rhs_into(
                    t + self.c[i] * h,
                    &self.stage_state,
                    &mut self.stage_rhs,
                    counters,
                )?;
            }
            // rhs = h gamma f_i + gamma sum_j C_ij U_j + h^2 gamma gamma_i f_t
            for x in self.stage_rhs.iter_mut() {
                *x *= hg;
            }
            for &(j, cij) in &self.c_nonzero[i] {
                let weight = gamma * cij;
                let uj = &self.u[j * n..(j + 1) * n];
                for (x, v) in self.stage_rhs.iter_mut().zip(uj) {
                    *x += weight * v;
                }
            }
            if !problem.autonomous {
                let g = hg * h * self.gamma_rows[i];
                for (x, v) in self.stage_rhs.iter_mut().zip(&self.ft) {
                    *x += g * v;
                }
            }
            if staged {
                self.staged_stage_solve(i, &shifted, linear_atol, atol, rtol, counters)?;
                continue;
            }
            // The step-indexed starts fall back to `Previous` until a step
            // was accepted (`step_h` is zero before).
            let step_start = self.config.x0_strategy.is_step_indexed() && self.step_h != 0.0;
            let previous = i > 0
                && !step_start
                && matches!(
                    self.config.x0_strategy,
                    InitialGuess::Previous
                        | InitialGuess::PreviousStep
                        | InitialGuess::PreviousStepScaled
                );
            if step_start && self.config.x0_strategy == InitialGuess::PreviousStepScaled {
                let ratio = h / self.step_h;
                self.x0_scaled.clear();
                self.x0_scaled
                    .extend(self.step_u[i * n..(i + 1) * n].iter().map(|v| v * ratio));
            }
            let gmres_config = GmresConfig {
                restart: self.config.restart,
                max_arnoldi: self.config.maxiter.max(self.config.restart),
                rtol: self.config.rtol,
                atol: linear_atol,
            };
            if self.gmres_into && self.config.method == LinearMethod::Gmres {
                let (done, rest) = self.u.split_at_mut(i * n);
                let stage = &mut rest[..n];
                let x0 = if !step_start {
                    previous.then(|| &done[(i - 1) * n..])
                } else if self.config.x0_strategy == InitialGuess::PreviousStepScaled {
                    Some(&self.x0_scaled[..])
                } else {
                    Some(&self.step_u[i * n..(i + 1) * n])
                };
                let report = solve_gmres_into(
                    &shifted,
                    &self.preconditioner,
                    &self.stage_rhs,
                    x0,
                    &gmres_config,
                    None,
                    stage,
                    &mut self.gmres,
                    GmresCapacity::unbounded(),
                    counters,
                )?;
                self.gmres_into_cycles += report.cycles;
                if !stage.iter().all(|value| value.is_finite()) {
                    return Err(CoreError::NonFinite(
                        "RODAS5P U-form stage solve produced NaN/Inf".into(),
                    ));
                }
                continue;
            }
            let x0 = if !step_start {
                previous.then(|| &self.u[(i - 1) * n..i * n])
            } else if self.config.x0_strategy == InitialGuess::PreviousStepScaled {
                Some(&self.x0_scaled[..])
            } else {
                Some(&self.step_u[i * n..(i + 1) * n])
            };
            let report = match self.config.method {
                LinearMethod::Gmres => solve_gmres_with_workspace(
                    &shifted,
                    &self.preconditioner,
                    &self.stage_rhs,
                    x0,
                    &gmres_config,
                    &mut self.gmres,
                    counters,
                )?,
                LinearMethod::Lgmres => {
                    let Some(KrylovState::Lgmres(st)) = state.as_deref_mut() else {
                        return Err(CoreError::InvalidInput(
                            "LGMRES needs an LGMRES recycle state".into(),
                        ));
                    };
                    solve_lgmres_with_workspace(
                        &shifted,
                        &self.preconditioner,
                        &self.stage_rhs,
                        x0,
                        &LgmresConfig {
                            inner_m: self.config.inner_m,
                            max_outer: self.config.maxiter,
                            outer_k: self.config.outer_k,
                            rtol: self.config.rtol,
                            atol: linear_atol,
                        },
                        st,
                        &mut self.lgmres,
                        counters,
                    )?
                }
                LinearMethod::Gcrodr => {
                    let Some(KrylovState::Gcrodr(st)) = state.as_deref_mut() else {
                        return Err(CoreError::InvalidInput(
                            "GCRO-DR needs a GCRO-DR recycle state".into(),
                        ));
                    };
                    gcrodr_stage_solve(
                        self.gcrodr_policy,
                        &shifted,
                        &self.preconditioner,
                        &self.stage_rhs,
                        x0,
                        &GcrodrConfig {
                            restart: self.config.restart,
                            max_arnoldi: self.config.maxiter.max(self.config.restart),
                            recycle_dim: self.config.recycle_dim,
                            rank_tol: self.config.recycle_rank_tol,
                            rtol: self.config.rtol,
                            atol: linear_atol,
                        },
                        st,
                        &mut self.gcrodr,
                        counters,
                    )?
                }
                LinearMethod::Direct => unreachable!("refused in validate_strict"),
            };
            if !report.x.iter().all(|value| value.is_finite()) {
                return Err(CoreError::NonFinite(
                    "RODAS5P U-form stage solve produced NaN/Inf".into(),
                ));
            }
            self.u[i * n..(i + 1) * n].copy_from_slice(&report.x);
        }
        self.y_new.copy_from_slice(y);
        for &(j, bj) in &self.b_nonzero {
            let uj = &self.u[j * n..(j + 1) * n];
            for (x, v) in self.y_new.iter_mut().zip(uj) {
                *x += bj * v;
            }
        }
        if !self.y_new.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "RODAS5P U-form step produced NaN/Inf".into(),
            ));
        }
        let error = &self.u[(s - 1) * n..s * n];
        let mut sum = 0.0;
        for ((e, a), b) in error.iter().zip(y).zip(&self.y_new) {
            let scale = atol + rtol * a.abs().max(b.abs());
            if !(scale.is_finite() && scale > 0.0) {
                return Err(CoreError::InvalidInput(
                    "every output error scale must be finite and positive".into(),
                ));
            }
            let z = e / scale;
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

fn failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        _ => None,
    }
}

/// Adaptive JVP-only RODAS5P on the raw U stages with the sequential
/// driver's controller and output rules. Requires a user JVP and the
/// identity mass matrix; any explicit-matrix request is refused.
pub fn integrate_rodas5p_mf_fast_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Rodas5pMfFastResult> {
    integrate_rodas5p_mf_fast_observed_traced(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        output,
        false,
        &mut |_, _, _, _, _| {},
    )
}

/// [`integrate_rodas5p_mf_fast_observed`] with
/// [`Rodas5pMfFastWorkspace::set_gmres_into`] on (research node
/// `research/rnext02_gmres_into_20261003`).
pub fn integrate_rodas5p_mf_fast_observed_gmres_into(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Rodas5pMfFastResult> {
    integrate_rodas5p_mf_fast_observed_traced(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        output,
        true,
        &mut |_, _, _, _, _| {},
    )
}

/// [`integrate_rodas5p_mf_fast_observed_gmres_into`] with
/// [`Rodas5pMfFastWorkspace::set_ls_workspace`] on (research node
/// `research/spd04_ls_workspace_20261007`).
pub fn integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Rodas5pMfFastResult> {
    integrate_mf_fast_inner(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        output,
        true,
        GcrodrRecyclePolicy::Legacy,
        &mut |_, _, _, _, _| {},
        true,
        StageTargetOptions::default(),
    )
    .map(|(result, _)| result)
}

/// Called after every attempt with the attempt's `t`, `y` and step, its
/// embedded error norm (`None` when the attempt failed) and the workspace
/// (stages and `y_new`), before the step is accepted or rejected.
pub type MfAttemptObserver<'a> =
    &'a mut dyn FnMut(f64, &[f64], f64, Option<f64>, &Rodas5pMfFastWorkspace);

/// The U-form driver with an attempt observer (research records, e.g. the
/// frozen systems of `research/rnext03_gcrodr_attribution_20261003`) and
/// the `gmres_into` switch. The observer cannot change the run.
#[allow(clippy::too_many_arguments)]
pub fn integrate_rodas5p_mf_fast_observed_traced(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    gmres_into: bool,
    observer: MfAttemptObserver<'_>,
) -> CoreResult<Rodas5pMfFastResult> {
    integrate_mf_fast_inner(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        output,
        gmres_into,
        GcrodrRecyclePolicy::Legacy,
        observer,
        false,
        StageTargetOptions::default(),
    )
    .map(|(result, _)| result)
}

/// [`integrate_rodas5p_mf_fast_observed`] with an explicit GCRO-DR recycle
/// policy (RVJ DAG node SAFE-RECYCLE,
/// `research/safe_recycle_policy_20261004`). `Legacy` is the default
/// driver; the policy only matters for `LinearMethod::Gcrodr`.
pub fn integrate_rodas5p_mf_fast_observed_with_gcrodr_policy(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    policy: GcrodrRecyclePolicy,
) -> CoreResult<Rodas5pMfFastResult> {
    integrate_mf_fast_inner(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        output,
        false,
        policy,
        &mut |_, _, _, _, _| {},
        false,
        StageTargetOptions::default(),
    )
    .map(|(result, _)| result)
}

#[allow(clippy::too_many_arguments)]
fn integrate_mf_fast_inner(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    gmres_into: bool,
    policy: GcrodrRecyclePolicy,
    observer: MfAttemptObserver<'_>,
    ls_workspace: bool,
    stage_target: StageTargetOptions,
) -> CoreResult<(Rodas5pMfFastResult, StageSolveStatistics)> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if tf < t || y0.len() != problem.dimension {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P matrix-free U-form integration input".into(),
        ));
    }
    let mut work = Rodas5pMfFastWorkspace::new(problem, linear_config)?;
    work.set_gmres_into(gmres_into);
    work.set_gcrodr_policy(policy);
    work.set_ls_workspace(ls_workspace);
    work.clear_accepted_step();
    let staged = stage_target.policy != StageTargetPolicy::Legacy;
    if staged {
        work.set_stage_target(stage_target)?;
        work.set_stage_target_span(tf - t);
        work.clear_accepted_error();
    }
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut recycle = KrylovState::for_method(linear_config.method);
    let mut collector =
        OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
    let (mut attempts, mut accepted_steps, mut rejected_steps, mut reuses) = (0, 0, 0, 0);
    let mut internal_steps = 0_usize;
    // Whether f(t, y), f_t and the JVP in the workspace belong to (t, y).
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
        let snapshot = recycle.clone();
        let outcome = work.attempt(
            problem,
            t,
            &y,
            trial_h,
            !fresh_state,
            recycle.as_mut(),
            adaptive.atol,
            adaptive.rtol,
            &mut counters,
        );
        fresh_state = work.jvp.is_some();
        observer(t, &y, trial_h, outcome.as_ref().ok().copied(), &work);
        let (error, failure) = match outcome {
            Ok(error) if error <= 1.0 => (error, None),
            Ok(error) => (error, Some(AdaptiveFailureKind::LocalError)),
            Err(error) => match failure_kind(&error) {
                Some(kind) => (f64::INFINITY, Some(kind)),
                None => return Err(error),
            },
        };
        match failure {
            None => {
                counters.accepted_steps += 1;
                accepted_steps += 1;
                t += trial_h;
                y.copy_from_slice(&work.y_new);
                collector.accept(t, &y, clipped)?;
                work.record_accepted_step(trial_h);
                if staged {
                    work.record_accepted_error(error);
                }
                internal_steps += 1;
                fresh_state = false;
            }
            Some(kind) => {
                // A rejected attempt leaves no trace in the recycle state.
                recycle = snapshot;
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
    let result = Rodas5pMfFastResult {
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
        state_reuses: reuses,
        driver: RODAS5P_MF_FAST_DRIVER_ID,
        gcrodr_policy: policy.id(),
    };
    Ok((result, work.stage_statistics()))
}

/// A run of [`integrate_rodas5p_mf_fast_observed_with_stage_target`].
#[derive(Clone, Debug)]
pub struct Rodas5pMfStageTargetResult {
    pub result: Rodas5pMfFastResult,
    pub options: StageTargetOptions,
    /// All zero for `Legacy`.
    pub statistics: StageSolveStatistics,
}

/// The U-form driver with a stage-target policy (research nodes
/// `research/alg01_coupled_stage_target_20261008` and
/// `research/alg03_stage_budget_guard_20261008`). With
/// `StageTargetPolicy::Legacy` this is
/// [`integrate_rodas5p_mf_fast_observed_gmres_into`] bit for bit; the staged
/// policies need GMRES without a preconditioner, solve every stage from a
/// zero start and get the span `T = t_end - t_0` and the last accepted
/// error from the driver. The Krylov budget is `linear_config.maxiter`.
pub fn integrate_rodas5p_mf_fast_observed_with_stage_target(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: StageTargetOptions,
) -> CoreResult<Rodas5pMfStageTargetResult> {
    let (result, statistics) = integrate_mf_fast_inner(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        output,
        true,
        GcrodrRecyclePolicy::Legacy,
        &mut |_, _, _, _, _| {},
        false,
        options,
    )?;
    Ok(Rodas5pMfStageTargetResult {
        result,
        options,
        statistics,
    })
}
