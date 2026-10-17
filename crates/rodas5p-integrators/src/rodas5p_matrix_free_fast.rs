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
    CoreError, CoreResult, IdentityPreconditioner, InitialGuess, KrylovSystemIdentity,
    LinearMethod, LinearOperator, LinearSolverConfig, OperatorApplicationWork, PreconditionerKind,
    ShiftedOperator, WorkCounters, exact_krylov_system_identity, rodas5p_coefficients, safe_l2,
    sha256_f64_slice_bytes,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrSolveOptions, GcrodrState, GcrodrWorkspace, GmresCapacity, GmresConfig,
    GmresIntoOptions, GmresWorkspace, HeuristicGain, LgmresConfig, LgmresIntoWorkspace,
    LgmresState, LgmresWorkspace, ResidualAccounting, StagedAcceptanceReason, StagedGmresConfig,
    StagedGmresFailure, StagedGmresOutcome, StagedGmresReport, StagedGmresWorkspace,
    StagedGuardAbort, StagedSolveTelemetry, TargetAttainment, VerifiedBound, copy_system_identity,
    solve_gcrodr_with_workspace, solve_gcrodr_with_workspace_and_options,
    solve_gmres_into_with_accounting, solve_gmres_with_workspace_and_accounting, solve_lgmres_into,
    solve_lgmres_with_workspace, solve_staged_gmres, solve_staged_gmres_observed,
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

/// How the U-form driver supplies the initial guess `x0` to each stage solve
/// (research node `research/sp04_zero_start_recycling_20261011`, node SP04).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum KrylovStartPolicy {
    /// Today's behaviour (the default): `x0` is supplied according to `config.x0_strategy`.
    /// For LGMRES and GCRO-DR, if `x0` is not given by previous stage, the solver's
    /// carried `previous_solution` is used.
    #[default]
    Configured,
    /// Hands every stage solve an explicit zero vector (`zero_x0`) owned by the workspace.
    /// `previous_solution` is never used as a start, for any method.
    ExplicitZero,
}

impl KrylovStartPolicy {
    /// Stable identifier for research records.
    pub fn id(self) -> &'static str {
        match self {
            Self::Configured => "krylov-start-configured-v1",
            Self::ExplicitZero => "krylov-start-explicit-zero-v1",
        }
    }
}

/// The lifetime of the carried recycle or augmentation subspace for GCRO-DR
/// and LGMRES (research node `research/sp04_zero_start_recycling_20261011`, node SP04).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum RecycleScope {
    /// Carried across steps and attempts (the pre-existing behaviour and default).
    #[default]
    Carried,
    /// Empties the carried subspace at the start of each attempt, so recycled
    /// vectors live only across the s stages of one W.
    CurrentW,
}

impl RecycleScope {
    /// Stable identifier for research records.
    pub fn id(self) -> &'static str {
        match self {
            Self::Carried => "recycle-scope-carried-v1",
            Self::CurrentW => "recycle-scope-current-w-v1",
        }
    }
}

/// The kind of start of one stage solve (research node SP04).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum StageStartKind {
    Zero,
    PreviousStage,
    PreviousStep,
    SolverCarried,
}

/// Opt-in start telemetry for Krylov stage solves (research node SP04).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct KrylovStartTelemetry {
    pub zero_starts: u64,
    pub previous_stage_starts: u64,
    pub previous_step_starts: u64,
    pub solver_carried_starts: u64,
    pub cross_system_basis_starts: u64,
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
/// (research nodes `research/alg01_coupled_stage_target_20261008`, ALG01,
/// and `research/alg04_coupled_target_v2_20261010`, ALG04). `Legacy` is the
/// default and the solve the driver always made; `DupFix` is that solve
/// without its final diagnostic residual; every other policy uses the staged
/// solver [`solve_staged_gmres`] (zero start, in-cycle projected exit
/// confirmed by one true residual, no duplicate final residual) and needs
/// GMRES without a preconditioner.
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
    /// ALG04: the `Legacy` stage solve (GMRES into, which the driver then
    /// requires) without the final diagnostic true residual. Same solution
    /// and trajectory bit for bit; one diagnostic operator application fewer
    /// per successful stage solve.
    DupFix,
    /// ALG04: `CoupledGuarded` plus small-system exhaustion (with
    /// `n <= restart` every stage solve confirms only at the end of the
    /// Krylov space: happy breakdown, the round-off floor `16 eps ||D b||`
    /// or the cycle's last column). The nonnormality guard is ALG01's rule.
    /// For `n > restart` it is `CoupledGuarded` bit for bit.
    CoupledGuarded2,
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
            Self::DupFix => "stage-target-dup-fix-v1",
            Self::CoupledGuarded2 => "stage-target-coupled-guarded2-v1",
        }
    }

    fn is_coupled(self) -> bool {
        matches!(
            self,
            Self::Coupled | Self::CoupledGuarded | Self::CoupledGuarded2
        )
    }

    /// The policy solves its stages with the staged solver (every policy
    /// but `Legacy` and `DupFix`).
    pub fn is_staged(self) -> bool {
        !matches!(self, Self::Legacy | Self::DupFix)
    }

    /// The nonnormality guard and, by default, the production fallback.
    fn is_guarded(self) -> bool {
        matches!(self, Self::CoupledGuarded | Self::CoupledGuarded2)
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
    /// `||b - W x||_2 <= max(|gamma| atol_lin, rtol_lin ||b||_2)`
    /// ([`ProductionFallbackRule`]: finite norms and thresholds only).
    pub production_fallback: bool,
    /// Classify guard aborts by an uncounted shadow continuation (reported
    /// only; it changes nothing in the run).
    pub classify_guard_aborts: bool,
    /// ALG06 reporting: also classify the guard aborts the production
    /// fallback accepted (with `classify_guard_aborts`; changes nothing).
    pub classify_accepted_guard_aborts: bool,
    /// ALG06 `G1`: the overrun prediction uses the effective cycle length
    /// `min(restart, n, budget - used)`.
    pub effective_cycle_overrun: bool,
    /// ALG06 `G2`: accept the attainable-accuracy floor
    /// `1024 eps (||D b|| + ||D x|| + ||D (x - W x)||)` at every true
    /// residual.
    pub floor_at_confirmations: bool,
    /// ALG06 `G3` (coupled policies): add `tau_e,i ||r_i||_WRMS` of every
    /// fallback-accepted stage `i` to the attempt's embedded error before
    /// the accept/reject decision.
    pub charge_fallback_residual: bool,
}

impl StageTargetOptions {
    /// `policy` as registered: the production fallback is part of
    /// `CoupledGuarded` and `CoupledGuarded2` only; no stagnation guard and
    /// none of the ALG06 switches.
    pub fn new(policy: StageTargetPolicy) -> Self {
        Self {
            policy,
            stagnation_guard: false,
            production_fallback: policy.is_guarded(),
            classify_guard_aborts: false,
            classify_accepted_guard_aborts: false,
            effective_cycle_overrun: false,
            floor_at_confirmations: false,
            charge_fallback_residual: false,
        }
    }
}

/// Production fallback branch taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FallbackBranch {
    #[serde(rename = "scaled-literal")]
    ScaledLiteral,
    #[serde(rename = "scaled-homogeneous")]
    ScaledHomogeneous,
    #[serde(rename = "unscaled-literal")]
    UnscaledLiteral,
    #[serde(rename = "unscaled-homogeneous")]
    UnscaledHomogeneous,
}

/// Evaluation record of a single fallback predicate invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FallbackEvaluation {
    pub branch: FallbackBranch,
    pub r_le_atol: bool,
}

impl FallbackEvaluation {
    pub const fn vector_copies(self) -> u64 {
        match (self.branch, self.r_le_atol) {
            (FallbackBranch::ScaledLiteral, _) => 1,
            (FallbackBranch::ScaledHomogeneous, true) => 1,
            (FallbackBranch::ScaledHomogeneous, false) => 2,
            (FallbackBranch::UnscaledLiteral, _) => 0,
            (FallbackBranch::UnscaledHomogeneous, _) => 1,
        }
    }
}

/// The overflow-safe production fallback rule (research node AS01,
/// `research/as01_fallback_seam_20261010`, finding F101).
///
/// Evaluates the production candidate against the production threshold:
/// * With `scale = None`: physical candidate `x` and physical true residual
///   `r = b - W x` against `max(atol_lin, rtol_lin ||b||_2)`.
/// * With `scale = Some(d)`: candidate `z = x / d` and scaled true residual
///   `r_scaled = r / d` componentwise; unscales them to physical `x` and `r`
///   before checking the physical threshold.
/// * If `||b||_2` overflows `f64`, computes the power of two `s = 2^exponent`
///   that scales `max |b_i|` into `[1, 2)` (away from underflow/overflow) and
///   checks `||r / s||_2 <= rtol_lin ||b / s||_2` (both
///   sides finite) or `||r||_2 <= atol_lin` (the absolute floor in its own,
///   physical, units). Scaling by a power of two is exact away from
///   underflow, so this is the same comparison wherever both forms are
///   representable, and a uniform rescaling by a power of two cannot turn
///   a rejection into an acceptance.
/// * A non-finite right-hand side, physical residual component or physical
///   candidate component, or a homogeneous threshold that is still not
///   representable, rejects: no claim is made that every large finite
///   system is solvable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProductionFallbackRule {
    atol: f64,
    threshold: ProductionFallbackThreshold,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ProductionFallbackThreshold {
    /// The literal threshold `max(atol_lin, rtol_lin ||b||_2)` (finite).
    Literal(f64),
    /// `1 / s` (a power of two) and the finite `rtol_lin ||b / s||_2`.
    Homogeneous { inverse_scale: f64, relative: f64 },
    /// No representable threshold: every candidate is rejected.
    Unrepresentable,
}

/// `2^k` for `-1074 <= k <= 1023`, exact (built from its bits).
fn power_of_two(k: i32) -> f64 {
    debug_assert!((-1074..=1023).contains(&k));
    if k >= -1022 {
        f64::from_bits(((k + 1023) as u64) << 52)
    } else {
        f64::from_bits(1_u64 << (k + 1074))
    }
}

impl ProductionFallbackRule {
    /// The rule for the physical right-hand side `rhs` with the linear
    /// tolerances `rtol` and `atol` (the driver passes `rtol_lin` and
    /// `atol_lin = |gamma| atol`). `||rhs||_2` and the threshold are checked
    /// here, before any fallback is built.
    pub fn new(rhs: &[f64], rtol: f64, atol: f64) -> Self {
        let rhs_norm = safe_l2(rhs);
        let literal = atol.max(rtol * rhs_norm);
        let threshold = if rhs_norm.is_finite() && literal.is_finite() {
            ProductionFallbackThreshold::Literal(literal)
        } else {
            Self::homogeneous(rhs, rtol)
        };
        Self { atol, threshold }
    }

    fn homogeneous(rhs: &[f64], rtol: f64) -> ProductionFallbackThreshold {
        let mut max = 0.0_f64;
        for value in rhs {
            let a = value.abs();
            if !a.is_finite() {
                return ProductionFallbackThreshold::Unrepresentable;
            }
            max = max.max(a);
        }
        if max == 0.0 {
            // (`||b||_2 = 0` takes the literal branch.)
            return ProductionFallbackThreshold::Unrepresentable;
        }
        // The unbiased exponent of `max` (subnormals as -1022), so that
        // `max / s` lies in [1, 2) for a normal `max`.
        let biased = ((max.to_bits() >> 52) & 0x7ff) as i32;
        let exponent = (biased - 1023).max(-1022);
        let inverse_scale = power_of_two(-exponent);
        let scaled: Vec<f64> = rhs.iter().map(|b| b * inverse_scale).collect();
        let relative = rtol * safe_l2(&scaled);
        if relative.is_finite() {
            ProductionFallbackThreshold::Homogeneous {
                inverse_scale,
                relative,
            }
        } else {
            ProductionFallbackThreshold::Unrepresentable
        }
    }

    /// The literal threshold `max(atol_lin, rtol_lin ||b||_2)` when it is
    /// representable (`None` on the homogeneous or rejecting branch).
    pub fn literal_threshold(&self) -> Option<f64> {
        match self.threshold {
            ProductionFallbackThreshold::Literal(threshold) => Some(threshold),
            _ => None,
        }
    }

    /// Whether some threshold (literal or homogeneous) is representable;
    /// without one every candidate is rejected.
    pub fn is_representable(&self) -> bool {
        self.threshold != ProductionFallbackThreshold::Unrepresentable
    }

    /// Accept the candidate `x` with true residual `r = b - W x`, returning
    /// the decision and the structured fallback evaluation record.
    pub fn accepts_observed(
        &self,
        candidate: &[f64],
        residual: &[f64],
        scale: Option<&[f64]>,
    ) -> (bool, FallbackEvaluation) {
        let branch = match (scale.is_some(), self.threshold) {
            (true, ProductionFallbackThreshold::Homogeneous { .. }) => {
                FallbackBranch::ScaledHomogeneous
            }
            (true, _) => FallbackBranch::ScaledLiteral,
            (false, ProductionFallbackThreshold::Homogeneous { .. }) => {
                FallbackBranch::UnscaledHomogeneous
            }
            (false, _) => FallbackBranch::UnscaledLiteral,
        };
        let unscaled: std::borrow::Cow<'_, [f64]> = match scale {
            Some(scale) => {
                if scale.len() != residual.len()
                    || scale.len() != candidate.len()
                    || !candidate
                        .iter()
                        .zip(scale)
                        .all(|(z, sc)| (z * sc).is_finite())
                {
                    return (
                        false,
                        FallbackEvaluation {
                            branch,
                            r_le_atol: false,
                        },
                    );
                }
                residual.iter().zip(scale).map(|(r, sc)| r * sc).collect()
            }
            None => {
                if !candidate.iter().all(|x| x.is_finite()) {
                    return (
                        false,
                        FallbackEvaluation {
                            branch,
                            r_le_atol: false,
                        },
                    );
                }
                residual.into()
            }
        };
        if !unscaled.iter().all(|r| r.is_finite()) {
            return (
                false,
                FallbackEvaluation {
                    branch,
                    r_le_atol: false,
                },
            );
        }
        let norm = safe_l2(&unscaled);
        let r_le_atol = norm.is_finite() && norm <= self.atol;
        let accepted = match self.threshold {
            ProductionFallbackThreshold::Literal(threshold) => norm <= threshold,
            ProductionFallbackThreshold::Homogeneous {
                inverse_scale,
                relative,
            } => {
                if norm <= self.atol {
                    true
                } else {
                    let scaled: Vec<f64> = unscaled.iter().map(|r| r * inverse_scale).collect();
                    let scaled_norm = safe_l2(&scaled);
                    scaled_norm.is_finite() && scaled_norm <= relative
                }
            }
            ProductionFallbackThreshold::Unrepresentable => false,
        };
        (accepted, FallbackEvaluation { branch, r_le_atol })
    }

    /// Accept the candidate `x` with true residual `r = b - W x`. With
    /// `scale = Some(d)` both are given in the scaled variables of the
    /// coupled target (physical `x = d z`, `r = d r_scaled` componentwise);
    /// with `None` they are physical.
    pub fn accepts(&self, candidate: &[f64], residual: &[f64], scale: Option<&[f64]>) -> bool {
        self.accepts_observed(candidate, residual, scale).0
    }
}

fn hex_digest_32<S>(digest: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut hex = [0u8; 64];
    for (i, &b) in digest.iter().enumerate() {
        hex[2 * i] = HEX_CHARS[(b >> 4) as usize];
        hex[2 * i + 1] = HEX_CHARS[(b & 0x0f) as usize];
    }
    let s = std::str::from_utf8(&hex).expect("valid hex is ascii");
    serializer.serialize_str(s)
}

/// The norm/scaling metric under which a stage residual was solved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ResidualMetric {
    ScaledWrms {
        atol_bits: u64,
        rtol_bits: u64,
        #[serde(serialize_with = "hex_digest_32")]
        state_digest: [u8; 32],
    },
    UnscaledL2,
}

/// Operator epoch tracking model epoch and linearization generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct OperatorEpoch {
    pub model_epoch: Option<u64>,
    pub generation: u64,
}

/// The outcome of an attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AttemptResidualOutcome {
    Committed,
    RejectedError,
    FailedLinear,
    FailedOther,
}

/// Scalar `Copy` telemetry record of a single stage solve.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct StageResidualRecord {
    pub attempt: usize,
    pub stage: usize,
    pub telemetry: StagedSolveTelemetry,
    pub metric: ResidualMetric,
    pub operator_epoch: OperatorEpoch,
    pub h_bits: u64,
    pub gamma_bits: u64,
    pub tau_e: f64,
    pub heuristic_debit: f64,
}

/// Scalar `Copy` telemetry record of a single step attempt.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct AttemptResidualRecord {
    pub t: f64,
    pub h: f64,
    pub outcome: AttemptResidualOutcome,
    pub converged: usize,
    pub stall_accepted: usize,
    pub floor_accepted: usize,
    pub fallback_accepted: usize,
    pub failed: usize,
    pub budget_exceeded_stages: usize,
    pub max_actual_over_requested: f64,
    pub debit_all: f64,
    pub debit_fallback_only: f64,
    pub jvp_calls: u64,
    pub arnoldi_jvps: u64,
    pub residual_jvps: u64,
    pub shadow_matvecs: u64,
    pub vector_copies: u64,
}

/// Cumulative work totals across all attempts of an integration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AttemptWorkTotals {
    pub jvp_calls: u64,
    pub arnoldi_jvps: u64,
    pub residual_jvps: u64,
    pub shadow_matvecs: u64,
    pub vector_copies: u64,
}

/// Complete telemetry structure for a matrix-free integration.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct StageResidualTelemetry {
    pub stage_records: Vec<StageResidualRecord>,
    pub attempt_records: Vec<AttemptResidualRecord>,
    pub fallback_evaluations: Vec<FallbackEvaluation>,
    pub committed_debit: f64,
    pub work_all_attempts: AttemptWorkTotals,
    pub jvp_outside_attempts: u64,
}

impl StageResidualTelemetry {
    pub fn clear(&mut self) {
        self.stage_records.clear();
        self.attempt_records.clear();
        self.fallback_evaluations.clear();
        self.committed_debit = 0.0;
        self.work_all_attempts = AttemptWorkTotals::default();
        self.jvp_outside_attempts = 0;
    }
}

const _: fn() = || {
    fn assert_copy<T: Copy>() {}
    assert_copy::<StageResidualRecord>();
    assert_copy::<AttemptResidualRecord>();
    assert_copy::<ResidualMetric>();
    assert_copy::<OperatorEpoch>();
    assert_copy::<AttemptResidualOutcome>();
    assert_copy::<FallbackEvaluation>();
    assert_copy::<AttemptWorkTotals>();
};

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
    /// ALG06 `G2`: accepted by the attainable-accuracy floor.
    pub floor_accepted: u64,
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
    /// Classified guard aborts the fallback accepted (ALG06 reporting):
    /// false (the shadow continuation converged) or true.
    pub guard_accepted_false: u64,
    pub guard_accepted_true: u64,
    /// Solves with small-system exhaustion active (ALG04).
    pub exhaustion_solves: u64,
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
    /// ALG06 `G3` bookkeeping (coupled policies; computed whether or not
    /// the charge is applied): attempts that completed with at least one
    /// fallback-accepted stage, the sum and maximum of their charges
    /// `sum_i tau_e,i ||r_i||_WRMS`, and the attempts whose embedded error
    /// was at most 1 and exceeds 1 once charged (with `G3` on, rejections
    /// caused by the charge).
    pub charged_attempts: u64,
    pub charge_sum: f64,
    pub charge_max: f64,
    pub charge_crosses_one: u64,
}

impl StageSolveStatistics {
    fn record(&mut self, report: &StagedGmresReport) {
        self.solves += 1;
        match report.outcome {
            StagedGmresOutcome::Converged => self.converged += 1,
            StagedGmresOutcome::StallAccepted => self.stall_accepted += 1,
            StagedGmresOutcome::FloorAccepted => self.floor_accepted += 1,
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
        let accepted_abort = report.outcome == StagedGmresOutcome::FallbackAccepted;
        match (report.shadow_converged, accepted_abort) {
            (Some(true), false) => self.guard_false += 1,
            (Some(false), false) => self.guard_true += 1,
            (Some(true), true) => self.guard_accepted_false += 1,
            (Some(false), true) => self.guard_accepted_true += 1,
            (None, _) => {}
        }
        if report.exhaustion {
            self.exhaustion_solves += 1;
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

/// One attempt that completed with at least one fallback-accepted stage
/// (ALG06 `G3` reporting): its time and step, the embedded error before the
/// charge, the charge `sum_i tau_e,i ||r_i||_WRMS` over those stages, their
/// number, and whether the charge was added to the returned error.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct AttemptCharge {
    pub t: f64,
    pub h: f64,
    pub error: f64,
    pub charge: f64,
    pub stages: u32,
    pub charged: bool,
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

/// Explicit driver option for Krylov workspace reuse and LGMRES-into
/// (research node `research/sp02_krylov_workspace_caller_20261011`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct KrylovWorkspaceOptions {
    pub ls_workspace: bool,
    pub lgmres_into: bool,
}

impl KrylovWorkspaceOptions {
    pub const fn none() -> Self {
        Self {
            ls_workspace: false,
            lgmres_into: false,
        }
    }

    pub const fn ls_workspace() -> Self {
        Self {
            ls_workspace: true,
            lgmres_into: false,
        }
    }

    pub const fn lgmres_into() -> Self {
        Self {
            ls_workspace: false,
            lgmres_into: true,
        }
    }

    pub const fn all() -> Self {
        Self {
            ls_workspace: true,
            lgmres_into: true,
        }
    }

    pub fn id(self) -> &'static str {
        match (self.ls_workspace, self.lgmres_into) {
            (false, false) => "krylov-ws-legacy-v1",
            (true, false) => "krylov-ws-ls-workspace-v1",
            (false, true) => "krylov-ws-lgmres-into-v1",
            (true, true) => "krylov-ws-ls-and-lgmres-into-v1",
        }
    }
}

/// Retained heap bytes by component of the Krylov workspaces and driver scratch
/// (research node `research/sp02_krylov_workspace_caller_20261011`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct KrylovWorkspaceBytes {
    pub gmres_bytes: usize,
    pub lgmres_bytes: usize,
    pub gcrodr_bytes: usize,
    pub staged_bytes: usize,
    pub lgmres_into_bytes: usize,
    pub attempt_snapshot_bytes: usize,
    pub driver_scratch_bytes: usize,
    pub cached_state_bytes: usize,
    pub least_squares_bytes: usize,
}

impl KrylovWorkspaceBytes {
    pub fn total(&self) -> usize {
        self.gmres_bytes
            + self.least_squares_bytes
            + self.lgmres_bytes
            + self.lgmres_into_bytes
            + self.attempt_snapshot_bytes
            + self.gcrodr_bytes
    }

    /// Bytes retained by the components the driver option adds
    /// (least-squares workspace, LGMRES-into snapshot and pool, attempt snapshot).
    pub fn option_added_bytes(&self) -> usize {
        if self.lgmres_into_bytes > 0 || self.attempt_snapshot_bytes > 0 {
            self.least_squares_bytes + self.lgmres_into_bytes + self.attempt_snapshot_bytes
        } else {
            self.least_squares_bytes
        }
    }
}

/// Growth events by Krylov workspace component across an integration trajectory
/// (research node `research/sp02_krylov_workspace_caller_20261011`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct KrylovWorkspaceGrowth {
    pub gmres_growth: u64,
    pub least_squares_growth: u64,
    pub lgmres_into_inner_growth: u64,
    pub lgmres_into_snapshot_growth: u64,
    pub lgmres_into_pool_growth: u64,
    pub attempt_snapshot_growth: u64,
}

impl KrylovWorkspaceGrowth {
    pub fn total(&self) -> u64 {
        self.gmres_growth
            + self.least_squares_growth
            + self.lgmres_into_inner_growth
            + self.lgmres_into_snapshot_growth
            + self.lgmres_into_pool_growth
            + self.attempt_snapshot_growth
    }
}

/// Rollback snapshot for LGMRES carried state across attempts.
/// Under `krylov_workspace.lgmres_into`, avoids per-attempt cloning of `LgmresState`
/// by copying directions, images, and previous solution into reusable buffers.
#[derive(Clone, Debug, Default)]
pub struct AttemptLgmresSnapshot {
    directions: Vec<Vec<f64>>,
    direction_count: usize,
    images: Vec<Vec<f64>>,
    image_present: Vec<bool>,
    image_count: usize,
    operator_token: Option<u64>,
    system_identity: Option<KrylovSystemIdentity>,
    previous_solution: Vec<f64>,
    previous_present: bool,
    generation: u64,
    pool: Vec<Vec<f64>>,
    growth_events: u64,
}

impl AttemptLgmresSnapshot {
    /// Save `state` into the reused snapshot buffers.
    pub fn save(&mut self, state: &LgmresState) {
        let before_bytes = self.retained_bytes();
        let count = state.directions.len();
        while self.directions.len() < count {
            self.directions.push(Vec::new());
        }
        for (i, dir) in state.directions.iter().enumerate() {
            self.directions[i].clear();
            self.directions[i].extend_from_slice(dir);
        }
        self.direction_count = count;

        let img_count = state.images.len();
        while self.images.len() < img_count {
            self.images.push(Vec::new());
        }
        self.image_present.clear();
        for (i, img) in state.images.iter().enumerate() {
            match img {
                Some(v) => {
                    self.images[i].clear();
                    self.images[i].extend_from_slice(v);
                    self.image_present.push(true);
                }
                None => {
                    self.image_present.push(false);
                }
            }
        }
        self.image_count = img_count;

        self.operator_token = state.operator_token;
        copy_system_identity(&mut self.system_identity, &state.system_identity);
        self.previous_present = state.previous_solution.is_some();
        if let Some(prev) = &state.previous_solution {
            self.previous_solution.clear();
            self.previous_solution.extend_from_slice(prev);
        }
        self.generation = state.generation;

        if self.retained_bytes() > before_bytes {
            self.growth_events += 1;
        }
    }

    /// Restore `state` from the snapshot buffers with zero heap allocations once warmed.
    pub fn restore(&mut self, state: &mut LgmresState) {
        let before_bytes = self.retained_bytes();
        while state.directions.len() > self.direction_count {
            if let Some(buf) = state.directions.pop() {
                self.pool.push(buf);
            }
        }
        while state.directions.len() < self.direction_count {
            let buf = self.pool.pop().unwrap_or_default();
            state.directions.push(buf);
        }
        for (i, dir) in self.directions[..self.direction_count].iter().enumerate() {
            state.directions[i].clear();
            state.directions[i].extend_from_slice(dir);
        }

        while state.images.len() > self.image_count {
            if let Some(Some(buf)) = state.images.pop() {
                self.pool.push(buf);
            }
        }
        while state.images.len() < self.image_count {
            state.images.push(None);
        }
        for i in 0..self.image_count {
            if self.image_present[i] {
                let src = &self.images[i];
                match &mut state.images[i] {
                    Some(dst) => {
                        dst.clear();
                        dst.extend_from_slice(src);
                    }
                    slot @ None => {
                        let mut buf = self.pool.pop().unwrap_or_default();
                        buf.clear();
                        buf.extend_from_slice(src);
                        *slot = Some(buf);
                    }
                }
            } else if let Some(buf) = state.images[i].take() {
                self.pool.push(buf);
            }
        }

        state.operator_token = self.operator_token;
        copy_system_identity(&mut state.system_identity, &self.system_identity);
        if self.previous_present {
            match &mut state.previous_solution {
                Some(dst) => {
                    dst.clear();
                    dst.extend_from_slice(&self.previous_solution);
                }
                slot @ None => {
                    let mut buf = self.pool.pop().unwrap_or_default();
                    buf.clear();
                    buf.extend_from_slice(&self.previous_solution);
                    *slot = Some(buf);
                }
            }
        } else if let Some(buf) = state.previous_solution.take() {
            self.pool.push(buf);
        }
        state.generation = self.generation;
        if self.retained_bytes() > before_bytes {
            self.growth_events += 1;
        }
    }

    /// Retained heap bytes of this snapshot.
    pub fn retained_bytes(&self) -> usize {
        self.directions.capacity() * std::mem::size_of::<Vec<f64>>()
            + self
                .directions
                .iter()
                .map(|v| v.capacity() * 8)
                .sum::<usize>()
            + self.images.capacity() * std::mem::size_of::<Vec<f64>>()
            + self.images.iter().map(|v| v.capacity() * 8).sum::<usize>()
            + self.image_present.capacity() * std::mem::size_of::<bool>()
            + self.previous_solution.capacity() * 8
            + self.pool.capacity() * std::mem::size_of::<Vec<f64>>()
            + self.pool.iter().map(|v| v.capacity() * 8).sum::<usize>()
    }

    pub fn growth_events(&self) -> u64 {
        self.growth_events
    }

    pub fn reset_growth_events(&mut self) {
        self.growth_events = 0;
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
    /// [`KrylovWorkspaceOptions::id`] of the run (research node SP02).
    pub krylov_workspace: &'static str,
    /// [`KrylovStartPolicy::id`] of the run (research node SP04).
    pub start_policy: &'static str,
    /// [`RecycleScope::id`] of the run (research node SP04).
    pub recycle_scope: &'static str,
    /// Opt-in start telemetry (research node SP04).
    pub start_telemetry: Option<KrylovStartTelemetry>,
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
    /// `tau_e,i` per stage (coupled policies; the ALG06 `G3` charge).
    stage_tau_e: Vec<f64>,
    /// `sum_i tau_e,i ||r_i||_WRMS` over the fallback-accepted stages of the
    /// current attempt, and their number.
    attempt_charge: f64,
    attempt_fallback_stages: u32,
    /// Every attempt with a fallback-accepted stage (ALG06 reporting).
    charge_log: Vec<AttemptCharge>,
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
    /// Research switch (SP01): the residual accounting of the GMRES stage
    /// solves (both the GMRES-into and the default path).
    residual_accounting: ResidualAccounting,
    /// Research record (SP01): one [`StageSolveLogEntry`] per successful
    /// non-staged Krylov stage solve, when switched on. Nothing reads it.
    solve_log: Option<Vec<StageSolveLogEntry>>,
    /// Research switch (SP02): Krylov workspace options.
    krylov_workspace: KrylovWorkspaceOptions,
    /// Reusable LGMRES-into workspace for stage solves under `krylov_workspace.lgmres_into`.
    lgmres_into: LgmresIntoWorkspace,
    /// Rollback snapshot for LGMRES carried state across attempts.
    attempt_snapshot: AttemptLgmresSnapshot,
    /// GMRES growth events count.
    gmres_growth_events: u64,
    /// Linearization generation count, incrementing whenever re-linearized.
    operator_generation: u64,
    /// Opt-in stage and attempt residual telemetry (research node AS04).
    stage_telemetry: Option<StageResidualTelemetry>,
    /// Index of current attempt.
    current_attempt_index: usize,
    /// Residual metric of current attempt.
    current_attempt_metric: ResidualMetric,
    /// Start policy for stage solves (research node SP04).
    start_policy: KrylovStartPolicy,
    /// Recycle scope for carried subspaces (research node SP04).
    recycle_scope: RecycleScope,
    /// Immutable zero vector for explicit zero starts (research node SP04).
    zero_x0: Vec<f64>,
    /// Whether start telemetry is enabled (research node SP04).
    start_telemetry_enabled: bool,
    /// Opt-in start telemetry accumulator (research node SP04).
    start_telemetry: KrylovStartTelemetry,
}

/// One successful Krylov stage solve's report, as recorded by
/// [`Rodas5pMfFastWorkspace::set_solve_log`] (research node
/// `research/sp01_dupfix_adoption_20261010`): the bits of the reported
/// residual norm and relative residual, and the iterations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct StageSolveLogEntry {
    pub residual_norm_bits: u64,
    pub relative_residual_bits: u64,
    pub iterations: u64,
}

impl StageSolveLogEntry {
    pub fn new(residual_norm: f64, relative_residual: f64, iterations: u64) -> Self {
        Self {
            residual_norm_bits: residual_norm.to_bits(),
            relative_residual_bits: relative_residual.to_bits(),
            iterations,
        }
    }
}

pub type MfFastInnerRun = (
    Rodas5pMfFastResult,
    StageSolveStatistics,
    Vec<AttemptCharge>,
    Option<Vec<StageSolveLogEntry>>,
);

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

/// A clone preserves config and options while starting with empty
/// least-squares and snapshot buffers and zeroed growth counters, which
/// regrow on first solve to produce bitwise-identical results.
impl Clone for Rodas5pMfFastWorkspace {
    fn clone(&self) -> Self {
        Self {
            n: self.n,
            s: self.s,
            gamma: self.gamma,
            c: self.c.clone(),
            gamma_rows: self.gamma_rows.clone(),
            a_nonzero: self.a_nonzero.clone(),
            c_nonzero: self.c_nonzero.clone(),
            b_nonzero: self.b_nonzero.clone(),
            config: self.config.clone(),
            preconditioner: IdentityPreconditioner::new(self.n),
            gmres: self.gmres.clone(),
            lgmres: self.lgmres.clone(),
            gcrodr: self.gcrodr.clone(),
            jvp: self.jvp.clone(),
            cached_callbacks: self.cached_callbacks.clone(),
            cached_t_bits: self.cached_t_bits,
            cached_y: self.cached_y.clone(),
            cached_epoch: self.cached_epoch,
            gmres_into: self.gmres_into,
            gcrodr_policy: self.gcrodr_policy,
            step_u: self.step_u.clone(),
            step_h: self.step_h,
            x0_scaled: self.x0_scaled.clone(),
            gmres_into_cycles: self.gmres_into_cycles,
            f0: self.f0.clone(),
            ft: self.ft.clone(),
            u: self.u.clone(),
            stage_state: self.stage_state.clone(),
            stage_rhs: self.stage_rhs.clone(),
            y_new: self.y_new.clone(),
            stage_target: self.stage_target,
            stage_tau: self.stage_tau.clone(),
            stage_tau_e: self.stage_tau_e.clone(),
            attempt_charge: self.attempt_charge,
            attempt_fallback_stages: self.attempt_fallback_stages,
            charge_log: self.charge_log.clone(),
            stage_span: self.stage_span,
            last_accepted_error: self.last_accepted_error,
            staged: self.staged.clone(),
            stage_eps: self.stage_eps.clone(),
            staged_scale: self.staged_scale.clone(),
            staged_weight: self.staged_weight.clone(),
            staged_rhs: self.staged_rhs.clone(),
            staged_z: self.staged_z.clone(),
            stage_statistics: self.stage_statistics,
            residual_accounting: self.residual_accounting,
            solve_log: self.solve_log.clone(),
            krylov_workspace: self.krylov_workspace,
            lgmres_into: self.lgmres_into.clone(),
            attempt_snapshot: AttemptLgmresSnapshot::default(),
            gmres_growth_events: 0,
            operator_generation: 0,
            stage_telemetry: self
                .stage_telemetry
                .as_ref()
                .map(|_| StageResidualTelemetry::default()),
            current_attempt_index: 0,
            current_attempt_metric: ResidualMetric::UnscaledL2,
            start_policy: self.start_policy,
            recycle_scope: self.recycle_scope,
            zero_x0: self.zero_x0.clone(),
            start_telemetry_enabled: self.start_telemetry_enabled,
            start_telemetry: self.start_telemetry.clone(),
        }
    }
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
            stage_tau_e: Vec::new(),
            attempt_charge: 0.0,
            attempt_fallback_stages: 0,
            charge_log: Vec::new(),
            stage_span: 0.0,
            last_accepted_error: None,
            staged: StagedGmresWorkspace::default(),
            stage_eps: Vec::new(),
            staged_scale: Vec::new(),
            staged_weight: Vec::new(),
            staged_rhs: Vec::new(),
            staged_z: Vec::new(),
            stage_statistics: StageSolveStatistics::default(),
            residual_accounting: ResidualAccounting::DEFAULT,
            solve_log: None,
            krylov_workspace: KrylovWorkspaceOptions::default(),
            lgmres_into: LgmresIntoWorkspace::default(),
            attempt_snapshot: AttemptLgmresSnapshot::default(),
            gmres_growth_events: 0,
            operator_generation: 0,
            stage_telemetry: None,
            current_attempt_index: 0,
            current_attempt_metric: ResidualMetric::UnscaledL2,
            start_policy: KrylovStartPolicy::Configured,
            recycle_scope: RecycleScope::Carried,
            zero_x0: vec![0.0; n],
            start_telemetry_enabled: false,
            start_telemetry: KrylovStartTelemetry::default(),
        })
    }

    /// Set opt-in stage and attempt residual telemetry (research node AS04).
    pub fn set_stage_telemetry(&mut self, on: bool) {
        self.stage_telemetry = on.then(StageResidualTelemetry::default);
    }

    /// Reserve capacity for opt-in stage and attempt residual telemetry.
    /// Enables `stage_telemetry` if not already enabled.
    pub fn reserve_stage_telemetry(&mut self, attempts: usize, stages: usize, fallbacks: usize) {
        let telemetry = self
            .stage_telemetry
            .get_or_insert_with(StageResidualTelemetry::default);
        telemetry.attempt_records.reserve(attempts);
        telemetry.stage_records.reserve(stages);
        telemetry.fallback_evaluations.reserve(fallbacks);
    }

    /// Read recorded stage and attempt residual telemetry, when enabled.
    pub fn stage_telemetry(&self) -> Option<&StageResidualTelemetry> {
        self.stage_telemetry.as_ref()
    }

    /// Set the residual accounting of the GMRES stage solves (research node
    /// `research/sp01_dupfix_adoption_20261010`), on the GMRES-into and the
    /// default path; the kernels' default otherwise. The `DupFix` stage
    /// target keeps ALG04's skip, which overrides it.
    pub fn set_residual_accounting(&mut self, accounting: ResidualAccounting) {
        self.residual_accounting = accounting;
    }

    pub fn residual_accounting(&self) -> ResidualAccounting {
        self.residual_accounting
    }

    /// Record every successful non-staged Krylov stage solve's report
    /// (research record; changes nothing). Switching it on clears the log.
    pub fn set_solve_log(&mut self, on: bool) {
        self.solve_log = on.then(Vec::new);
    }

    /// The recorded stage solves, if recording is on.
    pub fn solve_log(&self) -> Option<&[StageSolveLogEntry]> {
        self.solve_log.as_deref()
    }

    /// Set the stage-target policy (research nodes ALG01/ALG03). `Legacy`
    /// by default. The staged policies need GMRES without a preconditioner;
    /// the coupled ones compute the transfer constants here, from the
    /// coefficient snapshot, and need [`Self::set_stage_target_span`].
    pub fn set_stage_target(&mut self, options: StageTargetOptions) -> CoreResult<()> {
        if options.policy == StageTargetPolicy::DupFix && self.config.method != LinearMethod::Gmres
        {
            return Err(CoreError::InvalidInput(
                "the DupFix stage target needs GMRES (through GMRES into)".into(),
            ));
        }
        if options.charge_fallback_residual && !options.policy.is_coupled() {
            return Err(CoreError::InvalidInput(
                "charging the fallback residual needs a coupled stage target".into(),
            ));
        }
        if options.policy.is_staged() {
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
                self.stage_tau_e = tau.tau_e;
            }
        }
        self.stage_target = options;
        Ok(())
    }

    /// The attempts with a fallback-accepted stage so far (ALG06 `G3`
    /// reporting; coupled policies).
    pub fn charge_log(&self) -> &[AttemptCharge] {
        &self.charge_log
    }

    /// The charge `sum_i tau_e,i ||r_i||_WRMS` of the last attempt's
    /// fallback-accepted stages (zero without one).
    pub fn last_attempt_charge(&self) -> f64 {
        self.attempt_charge
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
        h: f64,
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
        // The physical norm and threshold are checked before the fallback
        // is built (AS01): an overflowing literal threshold is evaluated
        // homogeneously, never as `Inf <= Inf`.
        let production_rule =
            ProductionFallbackRule::new(&self.stage_rhs, production_rtol, linear_atol);
        let mut config = StagedGmresConfig {
            stagnation_guard: options.stagnation_guard,
            classify_guard_aborts: options.classify_guard_aborts,
            classify_accepted_guard_aborts: options.classify_accepted_guard_aborts,
            effective_cycle_overrun: options.effective_cycle_overrun,
            floor_at_confirmations: options.floor_at_confirmations,
            ..StagedGmresConfig::new(self.config.restart, budget, production_rtol, linear_atol)
        };
        let stage = &mut self.u[i * n..(i + 1) * n];
        let report = if options.policy.is_coupled() {
            let eps = self.stage_eps[i];
            config.atol = eps * (n as f64).sqrt();
            config.rtol = COUPLED_TARGET_ROUNDOFF;
            config.stall_rule = true;
            config.nu_guard = options.policy.is_guarded();
            config.small_system_exhaustion = options.policy == StageTargetPolicy::CoupledGuarded2;
            let requested_target = config.atol;
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
            let report = if let Some(telemetry) = &mut self.stage_telemetry {
                let fallback_start = telemetry.fallback_evaluations.len();
                let mut fallback = |z: &[f64], scaled_residual: &[f64]| {
                    let (acc, eval) =
                        production_rule.accepts_observed(z, scaled_residual, Some(scale));
                    telemetry.fallback_evaluations.push(eval);
                    acc
                };
                let mut tel = StagedSolveTelemetry {
                    reason: StagedAcceptanceReason::Failed,
                    requested_target,
                    raised_target: f64::NAN,
                    tightened_target: f64::NAN,
                    attainable_floor: f64::NAN,
                    actual_residual: f64::NAN,
                    actual_residual_inf: f64::NAN,
                    attainment: TargetAttainment::Unverified,
                    heuristic_gain: HeuristicGain::default(),
                    verified_bound: VerifiedBound::Unavailable,
                    arnoldi_jvps: 0,
                    residual_jvps: 0,
                    fallback_evaluations: 0,
                    acceptance_vector_copies: 0,
                    nu_flops: 0,
                    uncounted_shadow_matvecs: 0,
                };
                let report = solve_staged_gmres_observed(
                    &scaled,
                    &self.staged_rhs,
                    &config,
                    options
                        .production_fallback
                        .then_some(&mut fallback as &mut dyn FnMut(&[f64], &[f64]) -> bool),
                    &mut self.staged_z,
                    &mut self.staged,
                    counters,
                    requested_target,
                    &mut tel,
                )?;
                for eval in &telemetry.fallback_evaluations[fallback_start..] {
                    tel.acceptance_vector_copies += eval.vector_copies();
                }
                let heuristic_debit =
                    if report.accepted() && n > 0 && report.residual_norm.is_finite() {
                        self.stage_tau_e[i] * (report.residual_norm / (n as f64).sqrt())
                    } else {
                        0.0
                    };
                telemetry.stage_records.push(StageResidualRecord {
                    attempt: self.current_attempt_index,
                    stage: i,
                    telemetry: tel,
                    metric: self.current_attempt_metric,
                    operator_epoch: OperatorEpoch {
                        model_epoch: self.cached_epoch,
                        generation: self.operator_generation,
                    },
                    h_bits: h.to_bits(),
                    gamma_bits: self.gamma.to_bits(),
                    tau_e: self.stage_tau_e[i],
                    heuristic_debit,
                });
                report
            } else {
                let mut fallback = |z: &[f64], scaled_residual: &[f64]| {
                    production_rule.accepts(z, scaled_residual, Some(scale))
                };
                solve_staged_gmres(
                    &scaled,
                    &self.staged_rhs,
                    &config,
                    options
                        .production_fallback
                        .then_some(&mut fallback as &mut dyn FnMut(&[f64], &[f64]) -> bool),
                    &mut self.staged_z,
                    &mut self.staged,
                    counters,
                )?
            };
            if report.threshold > config.atol {
                self.stage_statistics.roundoff_floor_binds += 1;
            }
            if report.accepted() {
                for ((x, z), sc) in stage.iter_mut().zip(&self.staged_z).zip(scale) {
                    *x = z * sc;
                }
            }
            if report.outcome == StagedGmresOutcome::FallbackAccepted {
                // ||r||_WRMS = ||D r||_2 / sqrt(n) of the scaled residual.
                let wrms = if n > 0 && report.residual_norm.is_finite() {
                    report.residual_norm / (n as f64).sqrt()
                } else {
                    0.0
                };
                self.attempt_charge += self.stage_tau_e[i] * wrms;
                self.attempt_fallback_stages += 1;
            }
            report
        } else {
            if options.policy == StageTargetPolicy::L2Coupled {
                config.rtol = L2_COUPLED_RTOL_CAP.min(L2_COUPLED_FACTOR * rtol);
                config.atol = self.gamma.abs() * L2_COUPLED_FACTOR * atol;
            }
            let requested_target = config.atol.max(config.rtol * safe_l2(&self.stage_rhs));
            if let Some(telemetry) = &mut self.stage_telemetry {
                let fallback_start = telemetry.fallback_evaluations.len();
                let mut fallback = |x: &[f64], residual: &[f64]| {
                    let (acc, eval) = production_rule.accepts_observed(x, residual, None);
                    telemetry.fallback_evaluations.push(eval);
                    acc
                };
                let mut tel = StagedSolveTelemetry {
                    reason: StagedAcceptanceReason::Failed,
                    requested_target,
                    raised_target: f64::NAN,
                    tightened_target: f64::NAN,
                    attainable_floor: f64::NAN,
                    actual_residual: f64::NAN,
                    actual_residual_inf: f64::NAN,
                    attainment: TargetAttainment::Unverified,
                    heuristic_gain: HeuristicGain::default(),
                    verified_bound: VerifiedBound::Unavailable,
                    arnoldi_jvps: 0,
                    residual_jvps: 0,
                    fallback_evaluations: 0,
                    acceptance_vector_copies: 0,
                    nu_flops: 0,
                    uncounted_shadow_matvecs: 0,
                };
                let report = solve_staged_gmres_observed(
                    shifted,
                    &self.stage_rhs,
                    &config,
                    options
                        .production_fallback
                        .then_some(&mut fallback as &mut dyn FnMut(&[f64], &[f64]) -> bool),
                    stage,
                    &mut self.staged,
                    counters,
                    requested_target,
                    &mut tel,
                )?;
                for eval in &telemetry.fallback_evaluations[fallback_start..] {
                    tel.acceptance_vector_copies += eval.vector_copies();
                }
                telemetry.stage_records.push(StageResidualRecord {
                    attempt: self.current_attempt_index,
                    stage: i,
                    telemetry: tel,
                    metric: self.current_attempt_metric,
                    operator_epoch: OperatorEpoch {
                        model_epoch: self.cached_epoch,
                        generation: self.operator_generation,
                    },
                    h_bits: h.to_bits(),
                    gamma_bits: self.gamma.to_bits(),
                    tau_e: 0.0,
                    heuristic_debit: 0.0,
                });
                report
            } else {
                let mut fallback =
                    |x: &[f64], residual: &[f64]| production_rule.accepts(x, residual, None);
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
            }
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

    /// Explicit driver option for Krylov workspace reuse and LGMRES-into
    /// (research node `research/sp02_krylov_workspace_caller_20261011`).
    pub fn set_krylov_workspace(&mut self, options: KrylovWorkspaceOptions) -> CoreResult<()> {
        if (options.ls_workspace || options.lgmres_into)
            && !matches!(
                self.config.method,
                LinearMethod::Gmres | LinearMethod::Lgmres
            )
        {
            return Err(CoreError::InvalidInput(
                "Krylov workspace options only supported for GMRES and LGMRES methods".into(),
            ));
        }
        if options.lgmres_into && self.config.method != LinearMethod::Lgmres {
            return Err(CoreError::InvalidInput(
                "lgmres_into requires LinearMethod::Lgmres".into(),
            ));
        }
        if options.ls_workspace
            && self.config.method == LinearMethod::Lgmres
            && !options.lgmres_into
        {
            return Err(CoreError::InvalidInput(
                "ls_workspace on LGMRES requires lgmres_into".into(),
            ));
        }
        self.krylov_workspace = options;
        if self.config.method == LinearMethod::Gmres {
            if options.ls_workspace {
                self.gmres_into = true;
                self.gmres.set_ls_workspace(true);
            } else {
                self.gmres.set_ls_workspace(false);
            }
        } else if self.config.method == LinearMethod::Lgmres && options.lgmres_into {
            self.lgmres_into.set_least_squares_once(true);
            self.lgmres_into.set_ls_workspace(options.ls_workspace);
        }
        Ok(())
    }

    /// The active Krylov workspace options.
    pub fn krylov_workspace(&self) -> KrylovWorkspaceOptions {
        self.krylov_workspace
    }

    /// Mutable reference to the attempt snapshot.
    pub fn attempt_snapshot_mut(&mut self) -> &mut AttemptLgmresSnapshot {
        &mut self.attempt_snapshot
    }

    /// Reference to the attempt snapshot.
    pub fn attempt_snapshot(&self) -> &AttemptLgmresSnapshot {
        &self.attempt_snapshot
    }

    /// Retained heap bytes of all Krylov workspace components and driver scratch.
    pub fn krylov_workspace_bytes(&self) -> KrylovWorkspaceBytes {
        let driver_scratch_bytes = (self.u.capacity()
            + self.f0.capacity()
            + self.ft.capacity()
            + self.stage_state.capacity()
            + self.stage_rhs.capacity()
            + self.y_new.capacity()
            + self.x0_scaled.capacity()
            + self.step_u.capacity()
            + self.stage_eps.capacity()
            + self.staged_scale.capacity()
            + self.staged_weight.capacity()
            + self.staged_rhs.capacity()
            + self.staged_z.capacity())
            * std::mem::size_of::<f64>()
            + self.c.capacity() * std::mem::size_of::<f64>()
            + self.gamma_rows.capacity() * std::mem::size_of::<f64>();
        let cached_state_bytes = self.cached_y.capacity() * std::mem::size_of::<f64>();
        let least_squares_bytes =
            self.gmres.least_squares_bytes() + self.lgmres_into.least_squares_bytes();
        let gmres_bytes = self
            .gmres
            .retained_bytes()
            .saturating_sub(self.gmres.least_squares_bytes());
        let lgmres_into_bytes = self
            .lgmres_into
            .retained_bytes()
            .saturating_sub(self.lgmres_into.least_squares_bytes());
        KrylovWorkspaceBytes {
            gmres_bytes,
            lgmres_bytes: self.lgmres.retained_bytes(),
            gcrodr_bytes: self.gcrodr.retained_bytes(),
            staged_bytes: self.staged.retained_bytes(),
            lgmres_into_bytes,
            attempt_snapshot_bytes: self.attempt_snapshot.retained_bytes(),
            driver_scratch_bytes,
            cached_state_bytes,
            least_squares_bytes,
        }
    }

    /// Growth events of the Krylov workspaces across an integration trajectory.
    pub fn krylov_workspace_growth(&self) -> KrylovWorkspaceGrowth {
        let least_squares_growth = self.gmres.ls_workspace().map_or(0, |ls| ls.growth_events())
            + self.lgmres_into.ls_growth_events();
        KrylovWorkspaceGrowth {
            gmres_growth: self.gmres_growth_events,
            least_squares_growth,
            lgmres_into_inner_growth: self.lgmres_into.inner_growth_events(),
            lgmres_into_snapshot_growth: self.lgmres_into.snapshot_growth_events(),
            lgmres_into_pool_growth: self.lgmres_into.pool_growth_events(),
            attempt_snapshot_growth: self.attempt_snapshot.growth_events(),
        }
    }

    /// Reset growth event counters across all Krylov components.
    pub fn reset_krylov_workspace_growth(&mut self) {
        self.gmres_growth_events = 0;
        self.gmres.reset_growth_events();
        self.lgmres_into.reset_growth_events();
        self.attempt_snapshot.reset_growth_events();
    }

    /// Integrate an ODE problem using this workspace with default (no-op) observer.
    pub fn integrate_observed(
        &mut self,
        problem: &OdeProblem,
        t_span: (f64, f64),
        y0: &[f64],
        adaptive: &AdaptiveStepConfig,
        output: &OutputSchedule,
    ) -> CoreResult<Rodas5pMfFastResult> {
        self.integrate_observed_with_observer(
            problem,
            t_span,
            y0,
            adaptive,
            output,
            &mut |_, _, _, _, _| {},
        )
        .map(|(result, ..)| result)
    }

    /// Integrate an ODE problem with an attempt observer.
    pub fn integrate_observed_with_observer(
        &mut self,
        problem: &OdeProblem,
        t_span: (f64, f64),
        y0: &[f64],
        adaptive: &AdaptiveStepConfig,
        output: &OutputSchedule,
        observer: MfAttemptObserver<'_>,
    ) -> CoreResult<MfFastInnerRun> {
        adaptive.validate()?;
        let (mut t, tf) = t_span;
        if tf < t || y0.len() != problem.dimension {
            return Err(CoreError::InvalidInput(
                "invalid RODAS5P matrix-free U-form integration input".into(),
            ));
        }
        self.clear_accepted_step();
        self.jvp = None;
        self.cached_callbacks = None;
        self.cached_t_bits = 0;
        self.cached_epoch = None;
        self.operator_generation = 0;
        self.current_attempt_index = 0;
        if let Some(tel) = self.stage_telemetry.as_mut() {
            tel.clear();
        }
        self.start_telemetry = KrylovStartTelemetry::default();
        let staged = self.stage_target.policy != StageTargetPolicy::Legacy;
        if staged {
            self.set_stage_target_span(tf - t);
            self.clear_accepted_error();
        }
        let mut y = y0.to_vec();
        let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
        let mut controller = AdaptiveControllerState::default();
        let mut counters = WorkCounters::default();
        let mut recycle = KrylovState::for_method(self.config.method);
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
            if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf) || t + h == t
            {
                break;
            }
            let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
            attempts += 1;
            if fresh_state {
                reuses += 1;
            }
            let legacy_snapshot = if self.krylov_workspace.lgmres_into {
                None
            } else {
                Some(recycle.clone())
            };
            if self.krylov_workspace.lgmres_into
                && let Some(KrylovState::Lgmres(st)) = recycle.as_mut()
            {
                self.attempt_snapshot.save(st);
            }
            if self.recycle_scope == RecycleScope::CurrentW {
                match recycle.as_mut() {
                    Some(KrylovState::Gcrodr(st)) => {
                        st.basis.clear();
                        st.image.clear();
                        st.operator_token = None;
                        st.system_identity = None;
                        st.previous_solution = None;
                        st.generation = 0;
                    }
                    Some(KrylovState::Lgmres(st)) => {
                        st.directions.clear();
                        st.images.clear();
                        st.operator_token = None;
                        st.system_identity = None;
                        st.previous_solution = None;
                        st.generation = 0;
                    }
                    _ => {}
                }
            }
            let outcome = self.attempt(
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
            fresh_state = self.jvp.is_some();
            observer(t, &y, trial_h, outcome.as_ref().ok().copied(), self);
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
                    y.copy_from_slice(&self.y_new);
                    collector.accept(t, &y, clipped)?;
                    self.record_accepted_step(trial_h);
                    if staged {
                        self.record_accepted_error(error);
                    }
                    internal_steps += 1;
                    fresh_state = false;
                }
                Some(kind) => {
                    // A rejected attempt leaves no trace in the recycle state.
                    if self.krylov_workspace.lgmres_into {
                        if let Some(KrylovState::Lgmres(st)) = recycle.as_mut() {
                            self.attempt_snapshot.restore(st);
                        }
                    } else {
                        recycle = legacy_snapshot.expect("saved when lgmres_into is false");
                    }
                    counters.rejected_steps += 1;
                    rejected_steps += 1;
                    match kind {
                        AdaptiveFailureKind::LocalError => counters.local_error_failures += 1,
                        AdaptiveFailureKind::LinearSolve => counters.linear_solve_failures += 1,
                        AdaptiveFailureKind::NonlinearSolve => {
                            counters.nonlinear_solve_failures += 1
                        }
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
            gcrodr_policy: self.gcrodr_policy.id(),
            krylov_workspace: self.krylov_workspace.id(),
            start_policy: self.start_policy.id(),
            recycle_scope: self.recycle_scope.id(),
            start_telemetry: self
                .start_telemetry_enabled
                .then(|| self.start_telemetry.clone()),
        };
        if self.krylov_workspace.lgmres_into
            && let Some(KrylovState::Lgmres(st)) = recycle.as_mut()
        {
            self.lgmres_into.harvest_state(st);
        }
        if let Some(tel) = self.stage_telemetry.as_mut() {
            tel.jvp_outside_attempts = counters
                .jvp_calls
                .saturating_sub(tel.work_all_attempts.jvp_calls);
        }
        let charges = std::mem::take(&mut self.charge_log);
        let solve_log = self.solve_log.take();
        Ok((result, self.stage_statistics(), charges, solve_log))
    }

    /// Set the GCRO-DR recycle policy (research node
    /// `research/safe_recycle_policy_20261004`). `Legacy` by default.
    pub fn set_gcrodr_policy(&mut self, policy: GcrodrRecyclePolicy) {
        self.gcrodr_policy = policy;
    }

    pub fn gcrodr_policy(&self) -> GcrodrRecyclePolicy {
        self.gcrodr_policy
    }

    /// Set the Krylov start policy (research node SP04). `Configured` by default.
    pub fn set_start_policy(&mut self, policy: KrylovStartPolicy) {
        self.start_policy = policy;
    }

    pub fn start_policy(&self) -> KrylovStartPolicy {
        self.start_policy
    }

    /// Set the recycle scope for GCRO-DR and LGMRES (research node SP04). `Carried` by default.
    pub fn set_recycle_scope(&mut self, scope: RecycleScope) {
        self.recycle_scope = scope;
    }

    pub fn recycle_scope(&self) -> RecycleScope {
        self.recycle_scope
    }

    pub fn start_policy_configured() -> KrylovStartPolicy {
        KrylovStartPolicy::Configured
    }

    pub fn start_policy_explicit_zero() -> KrylovStartPolicy {
        KrylovStartPolicy::ExplicitZero
    }

    pub fn recycle_scope_carried() -> RecycleScope {
        RecycleScope::Carried
    }

    pub fn recycle_scope_current_w() -> RecycleScope {
        RecycleScope::CurrentW
    }

    pub fn set_explicit_zero(&mut self, on: bool) {
        self.start_policy = if on {
            KrylovStartPolicy::ExplicitZero
        } else {
            KrylovStartPolicy::Configured
        };
    }

    pub fn set_current_w(&mut self, on: bool) {
        self.recycle_scope = if on {
            RecycleScope::CurrentW
        } else {
            RecycleScope::Carried
        };
    }

    /// Enable or disable opt-in Krylov start telemetry (research node SP04).
    ///
    /// Start telemetry is strictly opt-in and diagnostic-only; it is disabled
    /// by default. Runs with start telemetry enabled are excluded from VG-1
    /// zero-allocation and wall-clock benchmarks because system identity
    /// hashing performs allocations.
    pub fn set_start_telemetry_enabled(&mut self, enabled: bool) {
        self.start_telemetry_enabled = enabled;
        if !enabled {
            self.start_telemetry = KrylovStartTelemetry::default();
        }
    }

    /// Alias for [`set_start_telemetry_enabled`].
    pub fn set_start_telemetry(&mut self, enabled: bool) {
        self.set_start_telemetry_enabled(enabled);
    }

    pub fn start_telemetry_enabled(&self) -> bool {
        self.start_telemetry_enabled
    }

    pub fn start_telemetry(&self) -> &KrylovStartTelemetry {
        &self.start_telemetry
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

    fn record_attempt_telemetry(
        &mut self,
        t: f64,
        h: f64,
        attempt_result: &CoreResult<f64>,
        stages_start: usize,
        jvp_delta: u64,
    ) {
        let Some(tel) = self.stage_telemetry.as_mut() else {
            return;
        };

        let outcome = match attempt_result {
            Ok(err_norm) => {
                if *err_norm <= 1.0 {
                    AttemptResidualOutcome::Committed
                } else {
                    AttemptResidualOutcome::RejectedError
                }
            }
            Err(CoreError::LinearSolve(_)) => AttemptResidualOutcome::FailedLinear,
            Err(_) => AttemptResidualOutcome::FailedOther,
        };

        let mut converged = 0;
        let mut stall_accepted = 0;
        let mut floor_accepted = 0;
        let mut fallback_accepted = 0;
        let mut failed = 0;
        let mut budget_exceeded_stages = 0;
        let mut max_actual_over_requested = 0.0_f64;
        let mut debit_all = 0.0_f64;
        let mut arnoldi_jvps = 0_u64;
        let mut residual_jvps = 0_u64;
        let mut shadow_matvecs = 0_u64;
        let mut vector_copies = 0_u64;

        for rec in &tel.stage_records[stages_start..] {
            match rec.telemetry.reason {
                StagedAcceptanceReason::Converged => converged += 1,
                StagedAcceptanceReason::StallAccepted => stall_accepted += 1,
                StagedAcceptanceReason::FloorAccepted => floor_accepted += 1,
                StagedAcceptanceReason::FallbackAccepted => fallback_accepted += 1,
                StagedAcceptanceReason::Failed => failed += 1,
            }
            if rec.telemetry.attainment == TargetAttainment::BudgetExceeded {
                budget_exceeded_stages += 1;
            }
            if rec.telemetry.requested_target > 0.0 && rec.telemetry.actual_residual.is_finite() {
                let ratio = rec.telemetry.actual_residual / rec.telemetry.requested_target;
                if ratio > max_actual_over_requested {
                    max_actual_over_requested = ratio;
                }
            }
            if rec.telemetry.reason != StagedAcceptanceReason::Failed
                && (rec.telemetry.attainment != TargetAttainment::Met
                    || rec.telemetry.reason == StagedAcceptanceReason::FallbackAccepted)
            {
                debit_all += rec.heuristic_debit;
            }
            arnoldi_jvps += rec.telemetry.arnoldi_jvps;
            residual_jvps += rec.telemetry.residual_jvps;
            shadow_matvecs += rec.telemetry.uncounted_shadow_matvecs;
            vector_copies += rec.telemetry.acceptance_vector_copies;
        }

        let debit_fallback_only = if matches!(
            outcome,
            AttemptResidualOutcome::Committed | AttemptResidualOutcome::RejectedError
        ) && self.attempt_fallback_stages > 0
        {
            self.attempt_charge
        } else {
            0.0
        };

        let record = AttemptResidualRecord {
            t,
            h,
            outcome,
            converged,
            stall_accepted,
            floor_accepted,
            fallback_accepted,
            failed,
            budget_exceeded_stages,
            max_actual_over_requested,
            debit_all,
            debit_fallback_only,
            jvp_calls: jvp_delta,
            arnoldi_jvps,
            residual_jvps,
            shadow_matvecs,
            vector_copies,
        };

        if outcome == AttemptResidualOutcome::Committed {
            tel.committed_debit += debit_all;
        }
        tel.work_all_attempts.jvp_calls += jvp_delta;
        tel.work_all_attempts.arnoldi_jvps += arnoldi_jvps;
        tel.work_all_attempts.residual_jvps += residual_jvps;
        tel.work_all_attempts.shadow_matvecs += shadow_matvecs;
        tel.work_all_attempts.vector_copies += vector_copies;

        tel.attempt_records.push(record);
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
        if self.stage_telemetry.is_none() {
            return self.attempt_inner(problem, t, y, h, fresh, recycle, atol, rtol, counters);
        }

        self.current_attempt_metric = if self.stage_target.policy.is_coupled() {
            ResidualMetric::ScaledWrms {
                atol_bits: atol.to_bits(),
                rtol_bits: rtol.to_bits(),
                state_digest: sha256_f64_slice_bytes(y),
            }
        } else {
            ResidualMetric::UnscaledL2
        };

        let stages_start = self
            .stage_telemetry
            .as_ref()
            .map_or(0, |tel| tel.stage_records.len());
        let jvp_before = counters.jvp_calls;

        let attempt_result =
            self.attempt_inner(problem, t, y, h, fresh, recycle, atol, rtol, counters);

        let jvp_delta = counters.jvp_calls.saturating_sub(jvp_before);
        self.record_attempt_telemetry(t, h, &attempt_result, stages_start, jvp_delta);
        self.current_attempt_index += 1;

        attempt_result
    }

    #[allow(clippy::too_many_arguments)]
    fn attempt_inner(
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
            self.operator_generation += 1;
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
        let staged = self.stage_target.policy.is_staged();
        let dup_fix = self.stage_target.policy == StageTargetPolicy::DupFix;
        if dup_fix && !(self.gmres_into && self.config.method == LinearMethod::Gmres) {
            return Err(CoreError::InvalidInput(
                "the DupFix stage target needs GMRES into".into(),
            ));
        }
        if staged {
            self.attempt_charge = 0.0;
            self.attempt_fallback_stages = 0;
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
        let telemetry_sys_id = if self.start_telemetry_enabled {
            exact_krylov_system_identity(&shifted, &self.preconditioner)
        } else {
            None
        };
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
                self.staged_stage_solve(i, h, &shifted, linear_atol, atol, rtol, counters)?;
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
            if self.start_telemetry_enabled {
                let cross_system = match state.as_deref() {
                    Some(KrylovState::Gcrodr(st)) => {
                        !st.basis.is_empty()
                            && (telemetry_sys_id.is_none()
                                || st.system_identity.as_ref() != telemetry_sys_id.as_ref())
                    }
                    Some(KrylovState::Lgmres(st)) => {
                        !st.directions.is_empty()
                            && (telemetry_sys_id.is_none()
                                || st.system_identity.as_ref() != telemetry_sys_id.as_ref())
                    }
                    _ => false,
                };
                if cross_system {
                    self.start_telemetry.cross_system_basis_starts += 1;
                }
                let kind = match self.start_policy {
                    KrylovStartPolicy::ExplicitZero => StageStartKind::Zero,
                    KrylovStartPolicy::Configured => {
                        if step_start {
                            StageStartKind::PreviousStep
                        } else if previous {
                            StageStartKind::PreviousStage
                        } else {
                            match state.as_deref() {
                                Some(KrylovState::Gcrodr(st))
                                    if st.previous_solution.is_some()
                                        && self.gcrodr_policy != GcrodrRecyclePolicy::Cold =>
                                {
                                    StageStartKind::SolverCarried
                                }
                                Some(KrylovState::Lgmres(st)) if st.previous_solution.is_some() => {
                                    StageStartKind::SolverCarried
                                }
                                _ => StageStartKind::Zero,
                            }
                        }
                    }
                };
                match kind {
                    StageStartKind::Zero => self.start_telemetry.zero_starts += 1,
                    StageStartKind::PreviousStage => {
                        self.start_telemetry.previous_stage_starts += 1
                    }
                    StageStartKind::PreviousStep => self.start_telemetry.previous_step_starts += 1,
                    StageStartKind::SolverCarried => {
                        self.start_telemetry.solver_carried_starts += 1
                    }
                }
            }
            if self.gmres_into && self.config.method == LinearMethod::Gmres {
                let (done, rest) = self.u.split_at_mut(i * n);
                let stage = &mut rest[..n];
                let x0 = match self.start_policy {
                    KrylovStartPolicy::ExplicitZero => Some(&self.zero_x0[..]),
                    KrylovStartPolicy::Configured => {
                        if !step_start {
                            previous.then(|| &done[(i - 1) * n..])
                        } else if self.config.x0_strategy == InitialGuess::PreviousStepScaled {
                            Some(&self.x0_scaled[..])
                        } else {
                            Some(&self.step_u[i * n..(i + 1) * n])
                        }
                    }
                };
                let report = solve_gmres_into_with_accounting(
                    &shifted,
                    &self.preconditioner,
                    &self.stage_rhs,
                    x0,
                    &gmres_config,
                    None,
                    stage,
                    &mut self.gmres,
                    GmresCapacity::unbounded(),
                    GmresIntoOptions {
                        skip_final_residual: dup_fix,
                    },
                    self.residual_accounting,
                    counters,
                )?;
                if report.workspace_grew {
                    self.gmres_growth_events += 1;
                }
                self.gmres_into_cycles += report.cycles;
                if !stage.iter().all(|value| value.is_finite()) {
                    return Err(CoreError::NonFinite(
                        "RODAS5P U-form stage solve produced NaN/Inf".into(),
                    ));
                }
                if let Some(log) = self.solve_log.as_mut() {
                    log.push(StageSolveLogEntry::new(
                        report.residual_norm,
                        report.relative_residual,
                        report.iterations,
                    ));
                }
                continue;
            }
            let x0 = match self.start_policy {
                KrylovStartPolicy::ExplicitZero => Some(&self.zero_x0[..]),
                KrylovStartPolicy::Configured => {
                    if !step_start {
                        previous.then(|| &self.u[(i - 1) * n..i * n])
                    } else if self.config.x0_strategy == InitialGuess::PreviousStepScaled {
                        Some(&self.x0_scaled[..])
                    } else {
                        Some(&self.step_u[i * n..(i + 1) * n])
                    }
                }
            };
            let report = match self.config.method {
                LinearMethod::Gmres => solve_gmres_with_workspace_and_accounting(
                    &shifted,
                    &self.preconditioner,
                    &self.stage_rhs,
                    x0,
                    &gmres_config,
                    None,
                    &mut self.gmres,
                    self.residual_accounting,
                    counters,
                )?,
                LinearMethod::Lgmres => {
                    let Some(KrylovState::Lgmres(st)) = state.as_deref_mut() else {
                        return Err(CoreError::InvalidInput(
                            "LGMRES needs an LGMRES recycle state".into(),
                        ));
                    };
                    let lgmres_config = LgmresConfig {
                        inner_m: self.config.inner_m,
                        max_outer: self.config.maxiter,
                        outer_k: self.config.outer_k,
                        rtol: self.config.rtol,
                        atol: linear_atol,
                    };
                    if self.krylov_workspace.lgmres_into {
                        let (done, rest) = self.u.split_at_mut(i * n);
                        let stage = &mut rest[..n];
                        let x0 = match self.start_policy {
                            KrylovStartPolicy::ExplicitZero => Some(&self.zero_x0[..]),
                            KrylovStartPolicy::Configured => {
                                if !step_start {
                                    previous.then(|| &done[(i - 1) * n..])
                                } else if self.config.x0_strategy
                                    == InitialGuess::PreviousStepScaled
                                {
                                    Some(&self.x0_scaled[..])
                                } else {
                                    Some(&self.step_u[i * n..(i + 1) * n])
                                }
                            }
                        };
                        let report = solve_lgmres_into(
                            &shifted,
                            &self.preconditioner,
                            &self.stage_rhs,
                            x0,
                            &lgmres_config,
                            st,
                            None,
                            stage,
                            &mut self.lgmres_into,
                            counters,
                        )?;
                        if !stage.iter().all(|value| value.is_finite()) {
                            return Err(CoreError::NonFinite(
                                "RODAS5P U-form stage solve produced NaN/Inf".into(),
                            ));
                        }
                        if let Some(log) = self.solve_log.as_mut() {
                            log.push(StageSolveLogEntry::new(
                                report.residual_norm,
                                report.relative_residual,
                                report.iterations,
                            ));
                        }
                        continue;
                    }
                    solve_lgmres_with_workspace(
                        &shifted,
                        &self.preconditioner,
                        &self.stage_rhs,
                        x0,
                        &lgmres_config,
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
            if let Some(log) = self.solve_log.as_mut() {
                log.push(StageSolveLogEntry::new(
                    report.residual_norm,
                    report.relative_residual,
                    report.iterations,
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
        let norm = if norm.is_finite() {
            norm
        } else {
            f64::INFINITY
        };
        if staged && self.attempt_fallback_stages > 0 {
            let charge = self.attempt_charge;
            let charged = self.stage_target.charge_fallback_residual;
            let statistics = &mut self.stage_statistics;
            statistics.charged_attempts += 1;
            statistics.charge_sum += charge;
            statistics.charge_max = statistics.charge_max.max(charge);
            if norm <= 1.0 && norm + charge > 1.0 {
                statistics.charge_crosses_one += 1;
            }
            self.charge_log.push(AttemptCharge {
                t,
                h,
                error: norm,
                charge,
                stages: self.attempt_fallback_stages,
                charged,
            });
            if charged {
                // ALG06 G3: the charged error decides accept/reject through
                // the ordinary local-error path.
                return Ok(norm + charge);
            }
        }
        Ok(norm)
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
        AccountingRun::DEFAULT,
    )
    .map(|(result, ..)| result)
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
        AccountingRun::DEFAULT,
    )
    .map(|(result, ..)| result)
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
        AccountingRun::DEFAULT,
    )
    .map(|(result, ..)| result)
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
    accounting: AccountingRun,
) -> CoreResult<MfFastInnerRun> {
    let mut work = Rodas5pMfFastWorkspace::new(problem, linear_config)?;
    work.set_gmres_into(gmres_into);
    work.set_gcrodr_policy(policy);
    work.set_ls_workspace(ls_workspace);
    work.set_residual_accounting(accounting.accounting);
    work.set_solve_log(accounting.solve_log);
    if ls_workspace && linear_config.method == LinearMethod::Gmres {
        work.set_krylov_workspace(KrylovWorkspaceOptions::ls_workspace())?;
    }
    let staged = stage_target.policy != StageTargetPolicy::Legacy;
    if staged {
        work.set_stage_target(stage_target)?;
        work.set_stage_target_span(t_span.1 - t_span.0);
        work.clear_accepted_error();
    }
    work.integrate_observed_with_observer(problem, t_span, y0, adaptive, output, observer)
}

/// The residual accounting of one [`integrate_mf_fast_inner`] run and
/// whether its stage solves are logged (research node SP01).
#[derive(Clone, Copy, Debug)]
struct AccountingRun {
    accounting: ResidualAccounting,
    solve_log: bool,
}

impl AccountingRun {
    /// The kernels' default accounting, no log: every pre-SP01 entry point.
    const DEFAULT: Self = Self {
        accounting: ResidualAccounting::DEFAULT,
        solve_log: false,
    };
}

/// A run of [`integrate_rodas5p_mf_fast_observed_with_stage_target`].
#[derive(Clone, Debug)]
pub struct Rodas5pMfStageTargetResult {
    pub result: Rodas5pMfFastResult,
    pub options: StageTargetOptions,
    /// All zero for `Legacy` and `DupFix`.
    pub statistics: StageSolveStatistics,
    /// Every attempt with a fallback-accepted stage (coupled policies; the
    /// ALG06 `G3` charge, applied or not).
    pub charges: Vec<AttemptCharge>,
}

/// The U-form driver with a stage-target policy (research nodes
/// `research/alg01_coupled_stage_target_20261008`,
/// `research/alg03_stage_budget_guard_20261008`,
/// `research/alg04_coupled_target_v2_20261010` and
/// `research/alg06_guard_v2_20261010`). With `StageTargetPolicy::Legacy`
/// this is [`integrate_rodas5p_mf_fast_observed_gmres_into`] bit for bit,
/// and `DupFix` has the same trajectory; the staged policies need GMRES
/// without a preconditioner, solve every stage from a zero start and get the
/// span `T = t_end - t_0` and the last accepted error (with `G3`, the
/// charged error) from the driver. The Krylov budget is
/// `linear_config.maxiter`.
pub fn integrate_rodas5p_mf_fast_observed_with_stage_target(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: StageTargetOptions,
) -> CoreResult<Rodas5pMfStageTargetResult> {
    let (result, statistics, charges, _) = integrate_mf_fast_inner(
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
        AccountingRun::DEFAULT,
    )?;
    Ok(Rodas5pMfStageTargetResult {
        result,
        options,
        statistics,
        charges,
    })
}

/// A run of [`integrate_rodas5p_mf_fast_observed_with_stage_telemetry`].
#[derive(Clone, Debug)]
pub struct Rodas5pMfStageTelemetryResult {
    pub result: Rodas5pMfFastResult,
    pub options: StageTargetOptions,
    pub statistics: StageSolveStatistics,
    pub charges: Vec<AttemptCharge>,
    pub telemetry: StageResidualTelemetry,
}

/// The U-form driver with opt-in stage and attempt residual telemetry (research node AS04).
pub fn integrate_rodas5p_mf_fast_observed_with_stage_telemetry(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: StageTargetOptions,
) -> CoreResult<Rodas5pMfStageTelemetryResult> {
    let mut work = Rodas5pMfFastWorkspace::new(problem, linear_config)?;
    work.set_gmres_into(true);
    work.set_stage_telemetry(true);
    let staged = options.policy != StageTargetPolicy::Legacy;
    if staged {
        work.set_stage_target(options)?;
        work.set_stage_target_span(t_span.1 - t_span.0);
        work.clear_accepted_error();
    }
    let (result, statistics, charges, _) = work.integrate_observed_with_observer(
        problem,
        t_span,
        y0,
        adaptive,
        output,
        &mut |_, _, _, _, _| {},
    )?;
    let telemetry = work.stage_telemetry().cloned().unwrap_or_default();
    Ok(Rodas5pMfStageTelemetryResult {
        result,
        options,
        statistics,
        charges,
        telemetry,
    })
}

/// A run of [`integrate_rodas5p_mf_fast_observed_with_residual_accounting`].
#[derive(Clone, Debug)]
pub struct Rodas5pMfAccountingResult {
    pub result: Rodas5pMfFastResult,
    pub accounting: ResidualAccounting,
    /// Every successful stage solve's report, in order.
    pub solve_log: Vec<StageSolveLogEntry>,
}

/// The U-form driver with an explicit GMRES residual accounting (research
/// node `research/sp01_dupfix_adoption_20261010`), on the GMRES-into path
/// (`gmres_into`) or the default path, with the stage solves' reports
/// logged. `stage_target` is `Legacy` or ALG04's `DupFix` (GMRES into
/// only, whose skip overrides the accounting). With the kernels' default
/// accounting and `Legacy` this is
/// [`integrate_rodas5p_mf_fast_observed_gmres_into`] (`gmres_into`) or
/// [`integrate_rodas5p_mf_fast_observed`] bit for bit.
#[allow(clippy::too_many_arguments)]
pub fn integrate_rodas5p_mf_fast_observed_with_residual_accounting(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    linear_config: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    gmres_into: bool,
    stage_target: StageTargetPolicy,
    accounting: ResidualAccounting,
) -> CoreResult<Rodas5pMfAccountingResult> {
    if stage_target.is_staged() {
        return Err(CoreError::InvalidInput(
            "the residual-accounting entry point takes the Legacy or DupFix stage target".into(),
        ));
    }
    let (result, _, _, solve_log) = integrate_mf_fast_inner(
        problem,
        t_span,
        y0,
        linear_config,
        adaptive,
        output,
        gmres_into,
        GcrodrRecyclePolicy::Legacy,
        &mut |_, _, _, _, _| {},
        false,
        StageTargetOptions::new(stage_target),
        AccountingRun {
            accounting,
            solve_log: true,
        },
    )?;
    Ok(Rodas5pMfAccountingResult {
        result,
        accounting,
        solve_log: solve_log.unwrap_or_default(),
    })
}
