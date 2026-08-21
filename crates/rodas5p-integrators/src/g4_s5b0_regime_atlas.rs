use std::{collections::BTreeSet, sync::Arc, time::Instant};

use rodas5p_core::{
    CoreError, CoreResult, LinearMethod, LinearSolverConfig, WorkCounters, error_scale, safe_l2,
    wrms,
};
use serde::{Deserialize, Serialize};

use crate::{
    AdaptiveControllerState, AdaptiveStepConfig, ControllerKind, FusedOrthogonalization,
    FusedPhiKrylovConfig, FusedPhiPrefixSession, OdeProblem, ParallelExecution, PersistenceLatch,
    Pexprb54s4AccountedBudgetedLevel2PrefixOutcome, Pexprb54s4BudgetedLevel2PrefixOutcome,
    Pexprb54s4Level1PrefixReport, Pexprb54s4Level2ContinuationOutcome, Pexprb54s4Level2Prefix,
    Pexprb54s4Level2PrefixReport, Pexprb54s4QuadraticRemainderDrift,
    Pexprb54s4RemainderVectorGeometry, pexprb54s4_fused_step, pexprb54s4_fused_step_resume_level2,
    pexprb54s4_fused_step_resume_level2_accounted,
    pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry,
    pexprb54s4_level2_prefix_resume_level1,
    pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget,
    pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget_accounted,
    pexprb54s4_tableau, sequential_matrix_free_step,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum G4S5B0Profile {
    Smoke,
    Canonical,
    Calibration128,
    Holdout512,
    StageGrowthCalibration96,
    StageGrowthCalibration192,
    StageGrowthCalibration256,
    EnforcedBudgetHoldout320,
    StageGrowthHoldout384,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum G4S5B0Family {
    RobertsonRamped,
    HiresRamped,
    VanDerPolRamped,
    RotatingNonnormal,
    NonautonomousStiffForcing,
    SemilinearAdvectionDiffusionRamped,
}

impl G4S5B0Family {
    pub const ALL: [Self; 6] = [
        Self::RobertsonRamped,
        Self::HiresRamped,
        Self::VanDerPolRamped,
        Self::RotatingNonnormal,
        Self::NonautonomousStiffForcing,
        Self::SemilinearAdvectionDiffusionRamped,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RobertsonRamped => "robertson-ramped",
            Self::HiresRamped => "hires-ramped",
            Self::VanDerPolRamped => "van-der-pol-ramped",
            Self::RotatingNonnormal => "rotating-nonnormal",
            Self::NonautonomousStiffForcing => "nonautonomous-stiff-forcing",
            Self::SemilinearAdvectionDiffusionRamped => "semilinear-advection-diffusion-ramped",
        }
    }
}

impl G4S5B0Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Canonical => "canonical",
            Self::Calibration128 => "calibration-128",
            Self::Holdout512 => "holdout-512",
            Self::StageGrowthCalibration96 => "stage-growth-calibration-96",
            Self::StageGrowthCalibration192 => "stage-growth-calibration-192",
            Self::StageGrowthCalibration256 => "stage-growth-calibration-256",
            Self::EnforcedBudgetHoldout320 => "enforced-budget-holdout-320",
            Self::StageGrowthHoldout384 => "stage-growth-holdout-384",
        }
    }

    pub fn dimensions(self) -> &'static [usize] {
        match self {
            Self::Smoke => &[32],
            Self::Canonical => &[128, 512],
            Self::Calibration128 => &[128],
            Self::Holdout512 => &[512],
            Self::StageGrowthCalibration96 => &[96],
            Self::StageGrowthCalibration192 => &[192],
            Self::StageGrowthCalibration256 => &[256],
            Self::EnforcedBudgetHoldout320 => &[320],
            Self::StageGrowthHoldout384 => &[384],
        }
    }

    pub fn uses_canonical_tolerances(self) -> bool {
        matches!(
            self,
            Self::Canonical | Self::Calibration128 | Self::Holdout512
        )
    }

    pub fn tolerances(self) -> (f64, f64) {
        match self {
            Self::Smoke => (1.0e-6, 1.0e-4),
            Self::Canonical | Self::Calibration128 | Self::Holdout512 => (1.0e-7, 1.0e-5),
            Self::StageGrowthCalibration96 => (3.0e-8, 3.0e-6),
            Self::StageGrowthCalibration192 => (1.5e-7, 1.5e-5),
            Self::StageGrowthCalibration256 => (3.0e-7, 3.0e-5),
            Self::EnforcedBudgetHoldout320 => (1.0e-7, 1.0e-5),
            Self::StageGrowthHoldout384 => (7.0e-8, 7.0e-6),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0StepRow {
    pub trajectory_id: String,
    pub family: String,
    pub dimension: usize,
    pub rtol: f64,
    pub step_index: usize,
    pub t_start: f64,
    pub h: f64,
    pub transition_level: f64,

    pub rodas_embedded_error: f64,
    pub rodas_wall_seconds: f64,
    pub rodas_rhs_evaluations: u64,
    pub rodas_jvp_vectors: u64,
    pub rodas_linear_matvecs: u64,

    pub exponential_completed: bool,
    pub exponential_total_error: Option<f64>,
    pub exponential_locally_admissible: bool,
    pub exponential_wall_seconds: Option<f64>,
    pub exponential_prefix_wall_seconds: Option<f64>,
    pub exponential_rhs_evaluations: Option<u64>,
    pub exponential_jvp_vectors: Option<u64>,
    pub exponential_maximum_krylov_dimension: Option<usize>,
    pub exponential_phi_substeps: Option<usize>,
    pub exponential_failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0TrajectorySummary {
    pub trajectory_id: String,
    pub family: String,
    pub dimension: usize,
    pub rtol: f64,
    pub success: bool,
    pub failure: Option<String>,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub endpoint_time: f64,
    pub explicit_jacobian_builds: u64,
    pub direct_factorizations: u64,
    pub newton_iterations: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0RjfAttemptRow {
    pub trajectory_id: String,
    pub family: String,
    pub dimension: usize,
    pub rtol: f64,
    pub attempt_index: usize,
    pub accepted_steps_before: usize,
    pub t_start: f64,
    pub h: f64,
    pub error_norm: Option<f64>,
    pub accepted: bool,
    pub recoverable_failure: bool,
    pub failure: Option<String>,
    pub wall_seconds: f64,
    pub rhs_evaluations: u64,
    pub jvp_vectors: u64,
    pub linear_matvecs: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0AttemptTraceReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub switching_active: bool,
    pub committed_method: &'static str,
    pub attempt_rows: Vec<G4S5B0RjfAttemptRow>,
    pub accepted_rows: Vec<G4S5B0StepRow>,
    pub trajectories: Vec<G4S5B0TrajectorySummary>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum G4S5B0PrefixProbePolicy {
    FrozenK1Comparator,
    K3Development,
}

impl G4S5B0PrefixProbePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FrozenK1Comparator => "frozen-k1-comparator",
            Self::K3Development => "k3-development",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0ActualLevel1PrefixRow {
    pub trajectory_id: String,
    pub family: String,
    pub dimension: usize,
    pub rtol: f64,
    pub policy: String,
    pub decision_accepted_step: usize,
    pub feature_value: Option<f64>,
    pub target_attempt_index: usize,
    pub target_accepted_steps_before: usize,
    pub t_start: f64,
    pub h: f64,
    pub target_r_attempt_accepted: bool,
    pub target_r_error_norm: Option<f64>,
    pub target_r_recoverable_failure: bool,
    pub prefix_wall_seconds: f64,
    pub prefix_succeeded: bool,
    pub prefix_failure: Option<String>,
    pub prefix_report: Option<Pexprb54s4Level1PrefixReport>,
    pub full_e_continued: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0ActualLevel1PrefixReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub policy: &'static str,
    pub switching_active: bool,
    pub committed_method: &'static str,
    pub full_e_continuations: usize,
    pub attempt_rows: Vec<G4S5B0RjfAttemptRow>,
    pub accepted_rows: Vec<G4S5B0StepRow>,
    pub prefix_rows: Vec<G4S5B0ActualLevel1PrefixRow>,
    pub trajectories: Vec<G4S5B0TrajectorySummary>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0ActualLevel2PrefixRow {
    pub trajectory_id: String,
    pub family: String,
    pub dimension: usize,
    pub rtol: f64,
    pub policy: String,
    pub decision_accepted_step: usize,
    pub feature_value: Option<f64>,
    pub target_attempt_index: usize,
    pub target_accepted_steps_before: usize,
    pub t_start: f64,
    pub h: f64,
    pub target_r_attempt_accepted: bool,
    pub target_r_error_norm: Option<f64>,
    pub target_r_recoverable_failure: bool,
    pub prefix_wall_seconds: f64,
    pub prefix_succeeded: bool,
    pub prefix_failure: Option<String>,
    pub prefix_report: Option<Pexprb54s4Level2PrefixReport>,
    pub full_e_continued: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0ActualLevel2PrefixReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub policy: &'static str,
    pub switching_active: bool,
    pub committed_method: &'static str,
    pub full_e_continuations: usize,
    pub attempt_rows: Vec<G4S5B0RjfAttemptRow>,
    pub accepted_rows: Vec<G4S5B0StepRow>,
    pub prefix_rows: Vec<G4S5B0ActualLevel2PrefixRow>,
    pub trajectories: Vec<G4S5B0TrajectorySummary>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0StageGrowthSafetyRow {
    pub trajectory_id: String,
    pub family: String,
    pub dimension: usize,
    pub rtol: f64,
    pub decision_accepted_step: usize,
    pub feature_value: Option<f64>,
    pub target_attempt_index: usize,
    pub target_accepted_steps_before: usize,
    pub t_start: f64,
    pub h: f64,
    pub target_r_attempt_accepted: bool,
    pub target_r_error_norm: Option<f64>,
    pub committed_rjf_jvp_before_target: u64,
    pub speculative_jvp_before_target: u64,
    pub budget_reserve_jvp: u64,
    pub budget_cap_jvp: u64,
    pub budget_fraction: f64,
    pub budget_admitted: bool,
    pub budget_exhausted: bool,
    pub prefix_succeeded: bool,
    pub prefix_failure: Option<String>,
    pub actual_prefix_jvp_vectors: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix_work: Option<WorkCounters>,
    pub normalized_stage_growth_a34: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rho2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rho3: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rho4: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_log_slope_s23: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_log_slope_s34: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_log_curvature_kappa234: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remainder_chi23: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remainder_chi34: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remainder_chi24: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remainder_q34_perp: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remainder_delta_chi: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quadratic_drift_zeta23: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quadratic_drift_zeta34: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quadratic_drift_relative: Option<f64>,
    pub budget_breached: bool,
    pub audit_full_e_completed: bool,
    pub audit_full_e_total_error: Option<f64>,
    pub audit_full_e_locally_admissible: bool,
    pub audit_full_e_failure: Option<String>,
    pub audit_full_e_work: Option<WorkCounters>,
    pub runtime_full_e_continued: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0StageGrowthSafetyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub switching_active: bool,
    pub committed_method: &'static str,
    pub runtime_full_e_continuations: usize,
    pub audit_full_e_continuations: usize,
    pub budget_breaches: usize,
    pub budget_exhaustions: usize,
    pub attempt_rows: Vec<G4S5B0RjfAttemptRow>,
    pub accepted_rows: Vec<G4S5B0StepRow>,
    pub rows: Vec<G4S5B0StageGrowthSafetyRow>,
    pub trajectories: Vec<G4S5B0TrajectorySummary>,
    pub limitations: Vec<String>,
}

/// Sealed v3.6 quadratic-drift threshold. This value is consumed as authority,
/// never calibrated or retuned by the full-E shadow runner.
pub const V36_FROZEN_ZETA34_TAU: f64 = 13.39706618860016;

pub fn frozen_full_e_shadow_recommended(
    prefix_succeeded: bool,
    budget_exhausted: bool,
    budget_breached: bool,
    zeta34: Option<f64>,
) -> bool {
    prefix_succeeded
        && !budget_exhausted
        && !budget_breached
        && zeta34.is_some_and(|value| value.is_finite() && value <= V36_FROZEN_ZETA34_TAU)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct G4S5B0RjfParitySummary {
    pub attempt_rows_exact_excluding_wall: bool,
    pub accepted_rows_exact_excluding_wall: bool,
    pub trajectories_exact: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct G4S5B0FrozenFullEShadowHardGates {
    pub all_rjf_trajectories_successful: bool,
    pub rjf_trace_exact_excluding_wall: bool,
    pub zero_budget_breaches: bool,
    pub prefix_transactions_resolved: bool,
    pub zero_continuation_failures: bool,
    pub zero_unsafe_recommendations: bool,
    pub work_ledgers_exact: bool,
    pub realized_work_ratios_finite: bool,
    pub resume_cardinality_exact: bool,
    pub shadow_implicit_expensive_work_zero: bool,
    pub active_switching_false: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0FrozenFullEShadowRow {
    pub trajectory_id: String,
    pub family: String,
    pub dimension: usize,
    pub rtol: f64,
    pub decision_accepted_step: usize,
    pub feature_value: Option<f64>,
    pub target_attempt_index: usize,
    pub target_accepted_steps_before: usize,
    pub t_start: f64,
    pub h: f64,
    pub target_r_attempt_accepted: bool,
    pub target_r_error_norm: Option<f64>,
    pub target_r_recoverable_failure: bool,
    pub committed_rjf_jvp_before_target: u64,
    pub prefix_speculative_jvp_before_target: u64,
    pub prefix_speculative_jvp_after_target: u64,
    pub total_speculative_jvp_before_target: u64,
    pub total_speculative_jvp_after_target: u64,
    pub budget_reserve_jvp: u64,
    pub budget_cap_jvp: u64,
    pub budget_fraction: f64,
    pub budget_admitted: bool,
    pub budget_exhausted: bool,
    pub budget_breached: bool,
    pub prefix_succeeded: bool,
    pub prefix_failure: Option<String>,
    pub actual_prefix_jvp_vectors: Option<u64>,
    pub prefix_work: Option<WorkCounters>,
    pub normalized_stage_growth_a34: Option<f64>,
    pub rho2: Option<f64>,
    pub rho3: Option<f64>,
    pub rho4: Option<f64>,
    pub stage_log_slope_s23: Option<f64>,
    pub stage_log_slope_s34: Option<f64>,
    pub stage_log_curvature_kappa234: Option<f64>,
    pub remainder_chi23: Option<f64>,
    pub remainder_chi34: Option<f64>,
    pub remainder_chi24: Option<f64>,
    pub remainder_q34_perp: Option<f64>,
    pub remainder_delta_chi: Option<f64>,
    pub quadratic_drift_zeta23: Option<f64>,
    pub quadratic_drift_zeta34: Option<f64>,
    pub quadratic_drift_relative: Option<f64>,
    pub frozen_zeta34_tau: f64,
    pub recommended: bool,
    pub retained_level2_resumed: bool,
    pub shadow_prefix_wall_seconds: f64,
    pub shadow_continuation_wall_seconds: Option<f64>,
    pub shadow_total_wall_seconds: f64,
    pub shadow_full_e_completed: bool,
    pub shadow_full_e_total_error: Option<f64>,
    pub shadow_full_e_locally_admissible: bool,
    pub shadow_full_e_failure: Option<String>,
    pub continuation_work: Option<WorkCounters>,
    pub shadow_full_e_work: Option<WorkCounters>,
    pub work_roundtrip_exact: bool,
    pub target_rjf_wall_seconds: Option<f64>,
    pub target_rjf_jvp_vectors: Option<u64>,
    pub prefix_over_target_rjf_jvp: Option<f64>,
    pub continuation_over_target_rjf_jvp: Option<f64>,
    pub full_e_over_target_rjf_jvp: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0FrozenFullEShadowReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub switching_active: bool,
    pub committed_method: &'static str,
    pub shadow_method: &'static str,
    pub persistence_k: usize,
    pub absolute_prefix_jvp_cap: u64,
    pub frozen_cumulative_prefix_budget_fraction: f64,
    pub frozen_zeta34_tau: f64,
    pub recommendations: usize,
    pub retained_level2_resumptions: usize,
    pub shadow_full_e_completions: usize,
    pub shadow_full_e_failures: usize,
    pub unsafe_recommendations: usize,
    pub budget_breaches: usize,
    pub budget_exhaustions: usize,
    pub prefix_speculative_work: WorkCounters,
    pub continuation_work: WorkCounters,
    pub total_speculative_work: WorkCounters,
    pub committed_rjf_jvp_vectors: u64,
    pub realized_prefix_over_committed_rjf_jvp: f64,
    pub realized_continuation_over_committed_rjf_jvp: f64,
    pub realized_total_speculative_over_committed_rjf_jvp: f64,
    pub rjf_parity: G4S5B0RjfParitySummary,
    pub hard_gates: G4S5B0FrozenFullEShadowHardGates,
    pub attempt_rows: Vec<G4S5B0RjfAttemptRow>,
    pub accepted_rows: Vec<G4S5B0StepRow>,
    pub rows: Vec<G4S5B0FrozenFullEShadowRow>,
    pub trajectories: Vec<G4S5B0TrajectorySummary>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0ShadowWallCalibrationRow {
    pub repetitions: usize,
    pub wall_seconds: f64,
    pub proposed_interval: f64,
    pub gamma_seconds_per_interval: f64,
    pub all_suite_identities_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0ShadowWallArm {
    pub mode: String,
    pub repetitions: usize,
    pub wall_seconds: f64,
    pub proposed_interval: f64,
    pub gamma_seconds_per_interval: f64,
    pub family_count: usize,
    pub all_suite_identities_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0ShadowWallPair {
    pub pair_index: usize,
    pub order: String,
    pub rjf_only: G4S5B0ShadowWallArm,
    pub frozen_full_e_shadow: G4S5B0ShadowWallArm,
    pub wall_ratio_shadow_over_rjf: f64,
    pub gamma_ratio_shadow_over_rjf: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0ShadowWallReport {
    pub required_build_profile: &'static str,
    pub measurement_build_verified: bool,
    pub compiled_cargo_profile: &'static str,
    pub compiled_profile_directory: &'static str,
    pub suite_scope: &'static str,
    pub calibration_arm: &'static str,
    pub gamma_denominator: &'static str,
    pub warmup_pairs: usize,
    pub measured_pairs: usize,
    pub frozen_repetitions: usize,
    pub minimum_calibration_wall_seconds: f64,
    pub maximum_calibration_repetitions: usize,
    pub calibration_rows: Vec<G4S5B0ShadowWallCalibrationRow>,
    pub warmup_rows: Vec<G4S5B0ShadowWallPair>,
    pub measured_rows: Vec<G4S5B0ShadowWallPair>,
    pub all_suite_identities_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0FrozenFullEShadowEconomicsReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub switching_active: bool,
    pub committed_method: &'static str,
    pub shadow_method: &'static str,
    pub frozen_zeta34_tau: f64,
    pub all_six_families_present: bool,
    pub reference_recommendations: usize,
    pub reference_shadow_completions: usize,
    pub reference_unsafe_recommendations: usize,
    pub reference_hard_gates: G4S5B0FrozenFullEShadowHardGates,
    pub paired_wall: G4S5B0ShadowWallReport,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0Report {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub switching_active: bool,
    pub committed_method: &'static str,
    pub rows: Vec<G4S5B0StepRow>,
    pub trajectories: Vec<G4S5B0TrajectorySummary>,
    pub limitations: Vec<String>,
}

type TransitionMetric = Arc<dyn Fn(f64, &[f64]) -> f64 + Send + Sync>;

struct AtlasProblem {
    family: &'static str,
    id: String,
    problem: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
    transition: TransitionMetric,
}

fn smooth_ramp(t: f64, center: f64, width: f64) -> (f64, f64) {
    let z = (t - center) / width;
    let th = z.tanh();
    (0.5 * (1.0 + th), 0.5 * (1.0 - th * th) / width)
}

fn block_padding_rhs(y: &[f64], out: &mut [f64], first_padding: usize, rate: f64) {
    for i in first_padding..y.len() {
        out[i] = -rate * y[i];
    }
}

fn robertson_transition_problem(n: usize) -> CoreResult<AtlasProblem> {
    if n < 3 {
        return Err(CoreError::InvalidInput(
            "Robertson atlas requires n>=3".into(),
        ));
    }
    let blocks = n / 3;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.045, 0.010);
        let activity = 0.05 + 0.95 * ramp;
        for block in 0..blocks {
            let i = 3 * block;
            let scale = 1.0 + 0.02 * (block % 5) as f64;
            let k1 = 0.04 * scale;
            let k2 = 1.0e4 * activity * scale;
            let k3 = 3.0e7 * activity * scale;
            let y1 = y[i];
            let y2 = y[i + 1];
            let y3 = y[i + 2];
            out[i] = -k1 * y1 + k2 * y2 * y3;
            out[i + 1] = k1 * y1 - k2 * y2 * y3 - k3 * y2 * y2;
            out[i + 2] = k3 * y2 * y2;
        }
        block_padding_rhs(y, out, 3 * blocks, 20.0 * activity);
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.045, 0.010);
        let activity = 0.05 + 0.95 * ramp;
        for block in 0..blocks {
            let i = 3 * block;
            let scale = 1.0 + 0.02 * (block % 5) as f64;
            let k1 = 0.04 * scale;
            let k2 = 1.0e4 * activity * scale;
            let k3 = 3.0e7 * activity * scale;
            out[i] = -k1 * v[i] + k2 * y[i + 2] * v[i + 1] + k2 * y[i + 1] * v[i + 2];
            out[i + 1] = k1 * v[i] + (-k2 * y[i + 2] - 2.0 * k3 * y[i + 1]) * v[i + 1]
                - k2 * y[i + 1] * v[i + 2];
            out[i + 2] = 2.0 * k3 * y[i + 1] * v[i + 1];
        }
        for i in 3 * blocks..n {
            out[i] = -20.0 * activity * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (_, dramp) = smooth_ramp(t, 0.045, 0.010);
        let dactivity = 0.95 * dramp;
        for block in 0..blocks {
            let i = 3 * block;
            let scale = 1.0 + 0.02 * (block % 5) as f64;
            let dk2 = 1.0e4 * dactivity * scale;
            let dk3 = 3.0e7 * dactivity * scale;
            let y2 = y[i + 1];
            let y3 = y[i + 2];
            out[i] = dk2 * y2 * y3;
            out[i + 1] = -dk2 * y2 * y3 - dk3 * y2 * y2;
            out[i + 2] = dk3 * y2 * y2;
        }
        for i in 3 * blocks..n {
            out[i] = -20.0 * dactivity * y[i];
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for block in 0..blocks {
        y0[3 * block] = 1.0;
    }
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.045, 0.010).0);
    Ok(AtlasProblem {
        family: "robertson-ramped",
        id: format!("robertson-ramped-n{n}"),
        problem: OdeProblem::new(
            format!("robertson-ramped-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            None,
        )?,
        y0,
        t_span: (0.0, 0.10),
        transition,
    })
}

fn hires_transition_problem(n: usize) -> CoreResult<AtlasProblem> {
    if n < 8 {
        return Err(CoreError::InvalidInput("HIRES atlas requires n>=8".into()));
    }
    let blocks = n / 8;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.08);
        let activity = 0.1 + 0.9 * ramp;
        for block in 0..blocks {
            let i = 8 * block;
            let y1 = y[i];
            let y2 = y[i + 1];
            let y3 = y[i + 2];
            let y4 = y[i + 3];
            let y5 = y[i + 4];
            let y6 = y[i + 5];
            let y7 = y[i + 6];
            let y8 = y[i + 7];
            let q = 280.0 * activity * y6 * y8;
            out[i] = -1.71 * y1 + 0.43 * y2 + 8.32 * y3 + 0.0007;
            out[i + 1] = 1.71 * y1 - 8.75 * y2;
            out[i + 2] = -10.03 * y3 + 0.43 * y4 + 0.035 * y5;
            out[i + 3] = 8.32 * y2 + 1.71 * y3 - 1.12 * y4;
            out[i + 4] = -1.745 * y5 + 0.43 * y6 + 0.43 * y7;
            out[i + 5] = -q + 0.69 * y4 + 1.71 * y5 - 0.43 * y6 + 0.69 * y7;
            out[i + 6] = q - 1.81 * y7;
            out[i + 7] = -q + 1.81 * y7;
        }
        block_padding_rhs(y, out, 8 * blocks, 2.0 + 20.0 * activity);
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.08);
        let activity = 0.1 + 0.9 * ramp;
        for block in 0..blocks {
            let i = 8 * block;
            let qv = 280.0 * activity * (y[i + 7] * v[i + 5] + y[i + 5] * v[i + 7]);
            out[i] = -1.71 * v[i] + 0.43 * v[i + 1] + 8.32 * v[i + 2];
            out[i + 1] = 1.71 * v[i] - 8.75 * v[i + 1];
            out[i + 2] = -10.03 * v[i + 2] + 0.43 * v[i + 3] + 0.035 * v[i + 4];
            out[i + 3] = 8.32 * v[i + 1] + 1.71 * v[i + 2] - 1.12 * v[i + 3];
            out[i + 4] = -1.745 * v[i + 4] + 0.43 * v[i + 5] + 0.43 * v[i + 6];
            out[i + 5] =
                -qv + 0.69 * v[i + 3] + 1.71 * v[i + 4] - 0.43 * v[i + 5] + 0.69 * v[i + 6];
            out[i + 6] = qv - 1.81 * v[i + 6];
            out[i + 7] = -qv + 1.81 * v[i + 6];
        }
        for i in 8 * blocks..n {
            out[i] = -(2.0 + 20.0 * activity) * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (_, dramp) = smooth_ramp(t, 0.45, 0.08);
        let dactivity = 0.9 * dramp;
        out.fill(0.0);
        for block in 0..blocks {
            let i = 8 * block;
            let dq = 280.0 * dactivity * y[i + 5] * y[i + 7];
            out[i + 5] = -dq;
            out[i + 6] = dq;
            out[i + 7] = -dq;
        }
        for i in 8 * blocks..n {
            out[i] = -20.0 * dactivity * y[i];
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for block in 0..blocks {
        y0[8 * block] = 1.0;
        y0[8 * block + 7] = 0.0057;
    }
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.45, 0.08).0);
    Ok(AtlasProblem {
        family: "hires-ramped",
        id: format!("hires-ramped-n{n}"),
        problem: OdeProblem::new(
            format!("hires-ramped-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            None,
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn vdp_transition_problem(n: usize) -> CoreResult<AtlasProblem> {
    if n < 2 {
        return Err(CoreError::InvalidInput("VDP atlas requires n>=2".into()));
    }
    let blocks = n / 2;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let mu = 10.0 + 490.0 * ramp;
        for block in 0..blocks {
            let i = 2 * block;
            let local_mu = mu * (1.0 + 0.01 * (block % 7) as f64);
            out[i] = y[i + 1];
            out[i + 1] = local_mu * (1.0 - y[i] * y[i]) * y[i + 1] - y[i];
        }
        block_padding_rhs(y, out, 2 * blocks, 5.0 + mu);
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let mu = 10.0 + 490.0 * ramp;
        for block in 0..blocks {
            let i = 2 * block;
            let local_mu = mu * (1.0 + 0.01 * (block % 7) as f64);
            out[i] = v[i + 1];
            out[i + 1] = (-2.0 * local_mu * y[i] * y[i + 1] - 1.0) * v[i]
                + local_mu * (1.0 - y[i] * y[i]) * v[i + 1];
        }
        for i in 2 * blocks..n {
            out[i] = -(5.0 + mu) * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (_, dramp) = smooth_ramp(t, 0.50, 0.08);
        let dmu = 490.0 * dramp;
        out.fill(0.0);
        for block in 0..blocks {
            let i = 2 * block;
            let local_dmu = dmu * (1.0 + 0.01 * (block % 7) as f64);
            out[i + 1] = local_dmu * (1.0 - y[i] * y[i]) * y[i + 1];
        }
        for i in 2 * blocks..n {
            out[i] = -dmu * y[i];
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for block in 0..blocks {
        y0[2 * block] = 2.0;
    }
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.50, 0.08).0);
    Ok(AtlasProblem {
        family: "van-der-pol-ramped",
        id: format!("van-der-pol-ramped-n{n}"),
        problem: OdeProblem::new(
            format!("van-der-pol-ramped-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            None,
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn exact_shape(n: usize, t: f64) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let k = 1.0 + (i % 7) as f64;
            0.4 * (k * t).sin() + 0.2 * (0.5 * k * t).cos()
        })
        .collect()
}

fn exact_shape_first(n: usize, t: f64) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let k = 1.0 + (i % 7) as f64;
            0.4 * k * (k * t).cos() - 0.1 * k * (0.5 * k * t).sin()
        })
        .collect()
}

fn apply_rotating_operator(n: usize, t: f64, x: &[f64], out: &mut [f64]) {
    let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
    let stiffness = 20.0 + 480.0 * ramp;
    let eta = 0.1 + 0.8 * ramp;
    let theta = 8.0 * t + 0.4 * (4.0 * t).sin();
    let c = theta.cos();
    let s = theta.sin();
    let blocks = n / 2;
    for block in 0..blocks {
        let i = 2 * block;
        let xr0 = c * x[i] + s * x[i + 1];
        let xr1 = -s * x[i] + c * x[i + 1];
        let ar0 = -stiffness * xr0 + eta * stiffness * xr1;
        let ar1 = -0.35 * stiffness * xr1;
        out[i] = c * ar0 - s * ar1;
        out[i + 1] = s * ar0 + c * ar1;
    }
    for i in 2 * blocks..n {
        out[i] = -stiffness * x[i];
    }
}

fn rotating_nonnormal_problem(n: usize) -> CoreResult<AtlasProblem> {
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = exact_shape(n, t);
        let dphi = exact_shape_first(n, t);
        let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_rotating_operator(n, t, &defect, out);
        let (ramp, _) = smooth_ramp(t, 0.60, 0.06);
        let nonlinear = 40.0 * ramp;
        for i in 0..n {
            out[i] += dphi[i] + nonlinear * (y[i] * y[i] - phi[i] * phi[i]);
        }
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        apply_rotating_operator(n, t, v, out);
        let (ramp, _) = smooth_ramp(t, 0.60, 0.06);
        let nonlinear = 40.0 * ramp;
        for i in 0..n {
            out[i] += 2.0 * nonlinear * y[i] * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let eps = 1.0e-6;
        let mut plus = vec![0.0; n];
        let mut minus = vec![0.0; n];
        let phi_p = exact_shape(n, t + eps);
        let dphi_p = exact_shape_first(n, t + eps);
        let defect_p = y.iter().zip(&phi_p).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_rotating_operator(n, t + eps, &defect_p, &mut plus);
        let (rp, _) = smooth_ramp(t + eps, 0.60, 0.06);
        for i in 0..n {
            plus[i] += dphi_p[i] + 40.0 * rp * (y[i] * y[i] - phi_p[i] * phi_p[i]);
        }
        let phi_m = exact_shape(n, t - eps);
        let dphi_m = exact_shape_first(n, t - eps);
        let defect_m = y.iter().zip(&phi_m).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_rotating_operator(n, t - eps, &defect_m, &mut minus);
        let (rm, _) = smooth_ramp(t - eps, 0.60, 0.06);
        for i in 0..n {
            minus[i] += dphi_m[i] + 40.0 * rm * (y[i] * y[i] - phi_m[i] * phi_m[i]);
            out[i] = (plus[i] - minus[i]) / (2.0 * eps);
        }
        Ok(())
    });
    let y0 = exact_shape(n, 0.0);
    let exact = Arc::new(move |t: f64| exact_shape(n, t));
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.55, 0.08).0);
    Ok(AtlasProblem {
        family: "rotating-nonnormal",
        id: format!("rotating-nonnormal-n{n}"),
        problem: OdeProblem::new(
            format!("rotating-nonnormal-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            Some(exact),
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn nonautonomous_forcing_problem(n: usize) -> CoreResult<AtlasProblem> {
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.07);
        let stiffness = 30.0 + 470.0 * ramp;
        let frequency = 2.0 + 28.0 * ramp;
        for i in 0..n {
            let phase = (i % 11) as f64 * 0.17;
            let phi = (frequency * t + phase).sin();
            let dphi = frequency * (frequency * t + phase).cos();
            let defect = y[i] - phi;
            out[i] = -stiffness * defect + dphi + 20.0 * ramp * defect * defect;
        }
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.07);
        let stiffness = 30.0 + 470.0 * ramp;
        let frequency = 2.0 + 28.0 * ramp;
        for i in 0..n {
            let phase = (i % 11) as f64 * 0.17;
            let phi = (frequency * t + phase).sin();
            let defect = y[i] - phi;
            out[i] = (-stiffness + 40.0 * ramp * defect) * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let eps = 1.0e-6;
        let evaluate = |time: f64, state: &[f64]| {
            let (ramp, _) = smooth_ramp(time, 0.45, 0.07);
            let stiffness = 30.0 + 470.0 * ramp;
            let frequency = 2.0 + 28.0 * ramp;
            (0..n)
                .map(|i| {
                    let phase = (i % 11) as f64 * 0.17;
                    let phi = (frequency * time + phase).sin();
                    let dphi = frequency * (frequency * time + phase).cos();
                    let defect = state[i] - phi;
                    -stiffness * defect + dphi + 20.0 * ramp * defect * defect
                })
                .collect::<Vec<_>>()
        };
        let plus = evaluate(t + eps, y);
        let minus = evaluate(t - eps, y);
        for i in 0..n {
            out[i] = (plus[i] - minus[i]) / (2.0 * eps);
        }
        Ok(())
    });
    let y0 = (0..n)
        .map(|i| ((i % 11) as f64 * 0.17).sin())
        .collect::<Vec<_>>();
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.45, 0.07).0);
    Ok(AtlasProblem {
        family: "nonautonomous-stiff-forcing",
        id: format!("nonautonomous-stiff-forcing-n{n}"),
        problem: OdeProblem::new(
            format!("nonautonomous-stiff-forcing-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            None,
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn apply_advection_diffusion(n: usize, t: f64, x: &[f64], out: &mut [f64]) {
    let dx = 1.0 / (n + 1) as f64;
    let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
    let diffusion = 0.002;
    let advection = 0.5 + 3.5 * ramp;
    let reaction = -1.0;
    for i in 0..n {
        let left = if i == 0 { 0.0 } else { x[i - 1] };
        let right = if i + 1 == n { 0.0 } else { x[i + 1] };
        out[i] = diffusion * (left - 2.0 * x[i] + right) / (dx * dx)
            - advection * (x[i] - left) / dx
            + reaction * x[i];
    }
}

fn semilinear_transition_problem(n: usize) -> CoreResult<AtlasProblem> {
    let dx = 1.0 / (n + 1) as f64;
    let exact = Arc::new(move |t: f64| {
        (1..=n)
            .map(|i| (-t).exp() * (std::f64::consts::PI * i as f64 * dx).sin())
            .collect::<Vec<_>>()
    });
    let rhs_exact = exact.clone();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = rhs_exact(t);
        let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_advection_diffusion(n, t, &defect, out);
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let nonlinear = 2.0 + 48.0 * ramp;
        for i in 0..n {
            out[i] += -phi[i] + nonlinear * (y[i] * y[i] - phi[i] * phi[i]);
        }
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        apply_advection_diffusion(n, t, v, out);
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let nonlinear = 2.0 + 48.0 * ramp;
        for i in 0..n {
            out[i] += 2.0 * nonlinear * y[i] * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let eps = 1.0e-6;
        let evaluate = |time: f64, state: &[f64]| {
            let phi = (1..=n)
                .map(|i| (-time).exp() * (std::f64::consts::PI * i as f64 * dx).sin())
                .collect::<Vec<_>>();
            let defect = state
                .iter()
                .zip(&phi)
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>();
            let mut value = vec![0.0; n];
            apply_advection_diffusion(n, time, &defect, &mut value);
            let (ramp, _) = smooth_ramp(time, 0.50, 0.08);
            let nonlinear = 2.0 + 48.0 * ramp;
            for i in 0..n {
                value[i] += -phi[i] + nonlinear * (state[i] * state[i] - phi[i] * phi[i]);
            }
            value
        };
        let plus = evaluate(t + eps, y);
        let minus = evaluate(t - eps, y);
        for i in 0..n {
            out[i] = (plus[i] - minus[i]) / (2.0 * eps);
        }
        Ok(())
    });
    let y0 = exact(0.0);
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.50, 0.08).0);
    Ok(AtlasProblem {
        family: "semilinear-advection-diffusion-ramped",
        id: format!("semilinear-advection-diffusion-ramped-n{n}"),
        problem: OdeProblem::new(
            format!("semilinear-advection-diffusion-ramped-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            Some(exact),
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn build_problems(profile: G4S5B0Profile) -> CoreResult<Vec<AtlasProblem>> {
    let mut problems = Vec::new();
    for &n in profile.dimensions() {
        problems.push(robertson_transition_problem(n)?);
        problems.push(hires_transition_problem(n)?);
        problems.push(vdp_transition_problem(n)?);
        problems.push(rotating_nonnormal_problem(n)?);
        problems.push(nonautonomous_forcing_problem(n)?);
        problems.push(semilinear_transition_problem(n)?);
    }
    Ok(problems)
}

fn adaptive_config(profile: G4S5B0Profile, span: f64) -> AdaptiveStepConfig {
    let (atol, rtol) = profile.tolerances();
    AdaptiveStepConfig {
        atol,
        rtol,
        initial_step: (span / 20.0).max(1.0e-8),
        min_step: 1.0e-12,
        max_step: span / 5.0,
        max_attempts: 20_000,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 4.0,
        reject_max_factor: 0.8,
        controller: ControllerKind::Pi,
    }
}

fn phi_config(rtol: f64, dimension: usize) -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 2,
        maximum_dimension: (dimension + 4).min(32),
        dimension_increment: 2,
        relative_tolerance: (0.03 * rtol).max(1.0e-12),
        absolute_tolerance: (3.0e-4 * rtol).max(1.0e-14),
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 16,
    }
}

fn linear_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        restart: 32,
        maxiter: 256,
        ..LinearSolverConfig::default()
    }
}

struct ExponentialShadow {
    completed: bool,
    total_error: Option<f64>,
    admissible: bool,
    wall: Option<f64>,
    prefix_wall: Option<f64>,
    work: Option<WorkCounters>,
    max_dimension: Option<usize>,
    substeps: Option<usize>,
    failure: Option<String>,
}

fn benchmark_operation<F>(minimum_sample_seconds: f64, mut operation: F) -> CoreResult<f64>
where
    F: FnMut() -> CoreResult<()>,
{
    operation()?;
    let mut batch = 1usize;
    loop {
        let start = Instant::now();
        for _ in 0..batch {
            operation()?;
        }
        if start.elapsed().as_secs_f64() >= minimum_sample_seconds || batch >= (1 << 16) {
            break;
        }
        batch = batch.saturating_mul(2);
    }
    let mut samples = Vec::with_capacity(3);
    for _ in 0..3 {
        let start = Instant::now();
        for _ in 0..batch {
            operation()?;
        }
        samples.push(start.elapsed().as_secs_f64() / batch as f64);
    }
    samples.sort_by(f64::total_cmp);
    Ok(samples[1])
}

fn shadow_problem_state(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let base = problem.jvp_only_clone()?;
    if problem.autonomous {
        Ok((base, y.to_vec()))
    } else {
        let augmented = base.time_augmented_clone()?;
        let mut state = y.to_vec();
        state.push(t);
        Ok((augmented, state))
    }
}

fn measure_e_prefix(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    minimum_sample_seconds: f64,
) -> CoreResult<f64> {
    let (shadow_problem, state) = shadow_problem_state(problem, t, y)?;
    let mut prep = WorkCounters::default();
    let f0 = shadow_problem.eval_rhs(t, &state, &mut prep)?;
    let tableau = pexprb54s4_tableau();
    let scale = tableau.c2 * h;
    let mut vectors = vec![vec![0.0; shadow_problem.dimension]; 2];
    for i in 0..shadow_problem.dimension {
        vectors[1][i] = tableau.c2 * f0[i] / scale;
    }
    benchmark_operation(minimum_sample_seconds, || {
        let operator = shadow_problem.linearize_matrix_free(t, &state)?;
        let mut counters = WorkCounters::default();
        let _ = FusedPhiPrefixSession::begin(
            operator,
            scale,
            &vectors,
            FusedPhiKrylovConfig {
                minimum_dimension: 1,
                maximum_dimension: config.maximum_dimension,
                dimension_increment: 1,
                relative_tolerance: config.relative_tolerance,
                absolute_tolerance: config.absolute_tolerance,
                orthogonalization: FusedOrthogonalization::FullMgs,
                maximum_substeps: 1,
            },
            2,
            &mut counters,
        )?;
        Ok(())
    })
}

fn run_exponential_shadow(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    atol: f64,
    rtol: f64,
    profile: G4S5B0Profile,
) -> ExponentialShadow {
    let result = (|| -> CoreResult<ExponentialShadow> {
        let (shadow_problem, state) = shadow_problem_state(problem, t, y)?;
        let config = phi_config(rtol, shadow_problem.dimension + 4);
        let execution = ParallelExecution::sequential();
        let start = Instant::now();
        let report = pexprb54s4_fused_step(&shadow_problem, t, &state, h, config, &execution)?;
        let wall = start.elapsed().as_secs_f64();
        let physical_n = problem.dimension;
        let y_new = &report.y_new[..physical_n];
        let error_vector = report
            .error_estimate
            .as_ref()
            .ok_or_else(|| CoreError::InvalidInput("pexprb54s4 omitted embedded error".into()))?;
        let scale = error_scale(y, y_new, &[atol], rtol)?;
        let time_error = wrms(&error_vector[..physical_n], &scale)?;
        let phi_error = h.abs()
            * report
                .fused_phi_reports
                .iter()
                .map(|entry| entry.error_estimate)
                .filter(|value| value.is_finite())
                .sum::<f64>()
            / safe_l2(&scale).max(f64::MIN_POSITIVE);
        let total_error = time_error.max(phi_error);
        let max_dimension = report
            .fused_phi_reports
            .iter()
            .map(|entry| entry.maximum_krylov_dimension)
            .max();
        let substeps = report
            .fused_phi_reports
            .iter()
            .map(|entry| entry.substeps)
            .sum();
        let prefix_wall = measure_e_prefix(
            problem,
            t,
            y,
            h,
            config,
            if profile.uses_canonical_tolerances() {
                0.001
            } else {
                0.0002
            },
        )?;
        Ok(ExponentialShadow {
            completed: true,
            total_error: Some(total_error),
            admissible: total_error.is_finite() && total_error <= 1.0,
            wall: Some(wall),
            prefix_wall: Some(prefix_wall),
            work: Some(report.work),
            max_dimension,
            substeps: Some(substeps),
            failure: None,
        })
    })();
    result.unwrap_or_else(|error| ExponentialShadow {
        completed: false,
        total_error: None,
        admissible: false,
        wall: None,
        prefix_wall: None,
        work: None,
        max_dimension: None,
        substeps: None,
        failure: Some(error.to_string()),
    })
}

fn run_trajectory(
    problem: AtlasProblem,
    profile: G4S5B0Profile,
    include_exponential_shadow: bool,
) -> (Vec<G4S5B0StepRow>, G4S5B0TrajectorySummary) {
    let adaptive = adaptive_config(profile, problem.t_span.1 - problem.t_span.0);
    let linear = linear_config();
    let mut controller = AdaptiveControllerState::default();
    let mut t = problem.t_span.0;
    let tf = problem.t_span.1;
    let mut y = problem.y0.clone();
    let mut h = adaptive.initial_step.min(tf - t);
    let tolerance = 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let mut attempts = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut rows = Vec::new();
    let mut total_jacobian_builds = 0u64;
    let mut total_factorizations = 0u64;
    let mut failure = None;

    while t < tf - tolerance && attempts < adaptive.max_attempts {
        attempts += 1;
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            failure = Some("minimum step reached".into());
            break;
        }
        let mut step_counters = WorkCounters::default();
        let start = Instant::now();
        let trial = sequential_matrix_free_step(
            &problem.problem,
            t,
            &y,
            h,
            &linear,
            None,
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut step_counters,
        );
        let rodas_wall = start.elapsed().as_secs_f64();
        total_jacobian_builds += step_counters.jacobian_builds;
        total_factorizations += step_counters.direct_factorizations;
        let report = match trial {
            Ok(report) => report,
            Err(CoreError::NonFinite(_) | CoreError::LinearSolve(_)) => {
                rejected += 1;
                h *= adaptive.min_factor;
                continue;
            }
            Err(error) => {
                failure = Some(error.to_string());
                break;
            }
        };
        let error = report.error_norm;
        let step_accepted = report.accepted
            && error.is_finite()
            && error <= 1.0
            && report.y_new.iter().all(|value| value.is_finite());
        if !step_accepted {
            rejected += 1;
            let _ = controller.record_rejection(error.max(1.0e-16));
            h *= controller
                .propose_factor(&adaptive, error.max(1.0e-16), 5, false)
                .unwrap_or(adaptive.min_factor);
            continue;
        }

        let shadow = if include_exponential_shadow {
            run_exponential_shadow(
                &problem.problem,
                t,
                &y,
                h,
                adaptive.atol,
                adaptive.rtol,
                profile,
            )
        } else {
            ExponentialShadow {
                completed: false,
                total_error: None,
                admissible: false,
                wall: None,
                prefix_wall: None,
                work: None,
                max_dimension: None,
                substeps: None,
                failure: None,
            }
        };
        let transition_level = (problem.transition)(t, &y);
        let exp_work = shadow.work.unwrap_or_default();
        rows.push(G4S5B0StepRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            step_index: accepted,
            t_start: t,
            h,
            transition_level,
            rodas_embedded_error: error,
            rodas_wall_seconds: rodas_wall,
            rodas_rhs_evaluations: step_counters.rhs_evaluations,
            rodas_jvp_vectors: step_counters.jvp_vectors,
            rodas_linear_matvecs: step_counters.linear_matvecs,
            exponential_completed: shadow.completed,
            exponential_total_error: shadow.total_error,
            exponential_locally_admissible: shadow.admissible,
            exponential_wall_seconds: shadow.wall,
            exponential_prefix_wall_seconds: shadow.prefix_wall,
            exponential_rhs_evaluations: shadow.completed.then_some(exp_work.rhs_evaluations),
            exponential_jvp_vectors: shadow.completed.then_some(exp_work.jvp_vectors),
            exponential_maximum_krylov_dimension: shadow.max_dimension,
            exponential_phi_substeps: shadow.substeps,
            exponential_failure: shadow.failure,
        });

        t = report.t_new;
        y = report.y_new;
        accepted += 1;
        let _ = controller.record_acceptance(error);
        h *= controller
            .propose_factor(&adaptive, error.max(1.0e-16), 5, true)
            .unwrap_or(1.0);
    }

    let success = t >= tf - tolerance;
    if !success && failure.is_none() {
        failure = Some("maximum attempts reached".into());
    }
    let summary = G4S5B0TrajectorySummary {
        trajectory_id: problem.id,
        family: problem.family.into(),
        dimension: problem.problem.dimension,
        rtol: adaptive.rtol,
        success,
        failure,
        attempts,
        accepted_steps: accepted,
        rejected_steps: rejected,
        endpoint_time: t,
        explicit_jacobian_builds: total_jacobian_builds,
        direct_factorizations: total_factorizations,
        newton_iterations: 0,
    };
    (rows, summary)
}

pub fn run_g4_s5b0_regime_atlas(profile: G4S5B0Profile) -> CoreResult<G4S5B0Report> {
    let mut rows = Vec::new();
    let mut trajectories = Vec::new();
    for problem in build_problems(profile)? {
        let (mut local_rows, summary) = run_trajectory(problem, profile, true);
        rows.append(&mut local_rows);
        trajectories.push(summary);
    }
    let status = if trajectories.iter().all(|row| row.success) {
        "complete"
    } else {
        "complete-with-failures"
    };
    Ok(G4S5B0Report {
        schema: "g4-s5b0-expanded-paired-regime-atlas-v1",
        status,
        profile: profile.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        rows,
        trajectories,
        limitations: vec![
            "The six families are transition-bearing scaled/replicated atlas variants, not canonical work-precision reproductions of the original benchmarks.".into(),
            "The committed trajectory is protected matrix-free RODAS5P; E-K is read-only shadow telemetry.".into(),
            "One-shot full-step wall times require repeated campaigns and median aggregation before residence labels are authoritative.".into(),
            "N=2048 remains sealed for the later selector holdout.".into(),
            "No active method switching or physical-client execution occurs in this node.".into(),
        ],
    })
}

fn run_g4_s5b0_rjf_only_filtered(
    profile: G4S5B0Profile,
    family: Option<G4S5B0Family>,
) -> CoreResult<G4S5B0Report> {
    let mut rows = Vec::new();
    let mut trajectories = Vec::new();
    for problem in build_problems(profile)? {
        if family.is_some_and(|selected| problem.family != selected.as_str()) {
            continue;
        }
        let (mut local_rows, summary) = run_trajectory(problem, profile, false);
        rows.append(&mut local_rows);
        trajectories.push(summary);
    }
    let status = if trajectories.iter().all(|row| row.success) {
        "complete"
    } else {
        "complete-with-failures"
    };
    Ok(G4S5B0Report {
        schema: "g4-s5b0-rjf-only-regime-replay-v1",
        status,
        profile: profile.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        rows,
        trajectories,
        limitations: vec![
            "This runner executes only the committed protected R-JF trajectory; all E-K fields are intentionally empty.".into(),
            "E-K labels and optimized timing may be joined only from separately provenance-checked durable audit data after exact R-JF numerical parity checks.".into(),
            "No active method switching or E-K prefix/full-step work occurs in this runner.".into(),
        ],
    })
}

pub fn run_g4_s5b0_rjf_only(profile: G4S5B0Profile) -> CoreResult<G4S5B0Report> {
    run_g4_s5b0_rjf_only_filtered(profile, None)
}

pub fn run_g4_s5b0_rjf_only_family(
    profile: G4S5B0Profile,
    family: G4S5B0Family,
) -> CoreResult<G4S5B0Report> {
    run_g4_s5b0_rjf_only_filtered(profile, Some(family))
}

const V25_ERROR_DROP_THRESHOLD: f64 = 0.012790399606947056;

fn frozen_k1_decision(family: &str, dimension: usize, step: usize) -> bool {
    let steps: &[usize] = match (dimension, family) {
        (128, "hires-ramped") => &[8, 13],
        (128, "nonautonomous-stiff-forcing") => &[
            7, 10, 12, 14, 17, 32, 46, 59, 77, 88, 94, 98, 104, 110, 116, 121, 139, 159, 170,
        ],
        (128, "robertson-ramped") => &[5, 14, 17, 21],
        (128, "rotating-nonnormal") => &[6, 20, 29, 34, 43, 68, 95],
        (128, "semilinear-advection-diffusion-ramped") => &[9, 15],
        (128, "van-der-pol-ramped") => &[11, 14, 24, 62],
        (512, "hires-ramped") => &[8, 13],
        (512, "nonautonomous-stiff-forcing") => &[
            7, 10, 12, 14, 17, 33, 46, 59, 77, 88, 93, 98, 104, 110, 115, 121, 139, 159, 170,
        ],
        (512, "robertson-ramped") => &[5, 14, 17, 21],
        (512, "rotating-nonnormal") => &[6, 20, 29, 34, 43, 69, 95],
        (512, "semilinear-advection-diffusion-ramped") => &[8, 20, 27, 34],
        (512, "van-der-pol-ramped") => &[11, 14, 24, 62],
        _ => &[],
    };
    steps.contains(&step)
}

struct PrefixPolicyState {
    policy: G4S5B0PrefixProbePolicy,
    log_errors: Vec<f64>,
    k3_latch: PersistenceLatch,
}

impl PrefixPolicyState {
    fn new(policy: G4S5B0PrefixProbePolicy) -> CoreResult<Self> {
        Ok(Self {
            policy,
            log_errors: Vec::new(),
            k3_latch: PersistenceLatch::new(3)?,
        })
    }

    fn observe_accepted(
        &mut self,
        family: &str,
        dimension: usize,
        step_index: usize,
        error: f64,
    ) -> Option<Option<f64>> {
        let log_error = error.max(1.0e-300).log10();
        let feature = if self.log_errors.len() >= 2 {
            Some(-(log_error - self.log_errors[self.log_errors.len() - 2]))
        } else {
            None
        };
        let fire = match self.policy {
            G4S5B0PrefixProbePolicy::FrozenK1Comparator => {
                frozen_k1_decision(family, dimension, step_index)
            }
            G4S5B0PrefixProbePolicy::K3Development => self.k3_latch.update(
                feature.is_some_and(|value| value.is_finite() && value >= V25_ERROR_DROP_THRESHOLD),
            ),
        };
        self.log_errors.push(log_error);
        fire.then_some(feature)
    }
}

fn run_rjf_attempt_trace_trajectory(
    problem: AtlasProblem,
    profile: G4S5B0Profile,
) -> (
    Vec<G4S5B0RjfAttemptRow>,
    Vec<G4S5B0StepRow>,
    G4S5B0TrajectorySummary,
) {
    let adaptive = adaptive_config(profile, problem.t_span.1 - problem.t_span.0);
    let linear = linear_config();
    let mut controller = AdaptiveControllerState::default();
    let mut t = problem.t_span.0;
    let tf = problem.t_span.1;
    let mut y = problem.y0.clone();
    let mut h = adaptive.initial_step.min(tf - t);
    let tolerance = 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let mut attempts = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut attempt_rows = Vec::new();
    let mut accepted_rows = Vec::new();
    let mut total_jacobian_builds = 0u64;
    let mut total_factorizations = 0u64;
    let mut failure = None;

    while t < tf - tolerance && attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            failure = Some("minimum step reached".into());
            break;
        }
        let attempt_index = attempts;
        attempts += 1;
        let accepted_steps_before = accepted;
        let t_start = t;
        let h_trial = h;
        let mut step_counters = WorkCounters::default();
        let start = Instant::now();
        let trial = sequential_matrix_free_step(
            &problem.problem,
            t,
            &y,
            h,
            &linear,
            None,
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut step_counters,
        );
        let wall = start.elapsed().as_secs_f64();
        total_jacobian_builds += step_counters.jacobian_builds;
        total_factorizations += step_counters.direct_factorizations;

        let report = match trial {
            Ok(report) => report,
            Err(error @ (CoreError::NonFinite(_) | CoreError::LinearSolve(_))) => {
                attempt_rows.push(G4S5B0RjfAttemptRow {
                    trajectory_id: problem.id.clone(),
                    family: problem.family.into(),
                    dimension: problem.problem.dimension,
                    rtol: adaptive.rtol,
                    attempt_index,
                    accepted_steps_before,
                    t_start,
                    h: h_trial,
                    error_norm: None,
                    accepted: false,
                    recoverable_failure: true,
                    failure: Some(error.to_string()),
                    wall_seconds: wall,
                    rhs_evaluations: step_counters.rhs_evaluations,
                    jvp_vectors: step_counters.jvp_vectors,
                    linear_matvecs: step_counters.linear_matvecs,
                });
                rejected += 1;
                h *= adaptive.min_factor;
                continue;
            }
            Err(error) => {
                attempt_rows.push(G4S5B0RjfAttemptRow {
                    trajectory_id: problem.id.clone(),
                    family: problem.family.into(),
                    dimension: problem.problem.dimension,
                    rtol: adaptive.rtol,
                    attempt_index,
                    accepted_steps_before,
                    t_start,
                    h: h_trial,
                    error_norm: None,
                    accepted: false,
                    recoverable_failure: false,
                    failure: Some(error.to_string()),
                    wall_seconds: wall,
                    rhs_evaluations: step_counters.rhs_evaluations,
                    jvp_vectors: step_counters.jvp_vectors,
                    linear_matvecs: step_counters.linear_matvecs,
                });
                failure = Some(error.to_string());
                break;
            }
        };
        let error = report.error_norm;
        let step_accepted = report.accepted
            && error.is_finite()
            && error <= 1.0
            && report.y_new.iter().all(|value| value.is_finite());
        attempt_rows.push(G4S5B0RjfAttemptRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            attempt_index,
            accepted_steps_before,
            t_start,
            h: h_trial,
            error_norm: Some(error),
            accepted: step_accepted,
            recoverable_failure: false,
            failure: None,
            wall_seconds: wall,
            rhs_evaluations: step_counters.rhs_evaluations,
            jvp_vectors: step_counters.jvp_vectors,
            linear_matvecs: step_counters.linear_matvecs,
        });
        if !step_accepted {
            rejected += 1;
            let _ = controller.record_rejection(error.max(1.0e-16));
            h *= controller
                .propose_factor(&adaptive, error.max(1.0e-16), 5, false)
                .unwrap_or(adaptive.min_factor);
            continue;
        }

        accepted_rows.push(G4S5B0StepRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            step_index: accepted,
            t_start: t,
            h,
            transition_level: (problem.transition)(t, &y),
            rodas_embedded_error: error,
            rodas_wall_seconds: wall,
            rodas_rhs_evaluations: step_counters.rhs_evaluations,
            rodas_jvp_vectors: step_counters.jvp_vectors,
            rodas_linear_matvecs: step_counters.linear_matvecs,
            exponential_completed: false,
            exponential_total_error: None,
            exponential_locally_admissible: false,
            exponential_wall_seconds: None,
            exponential_prefix_wall_seconds: None,
            exponential_rhs_evaluations: None,
            exponential_jvp_vectors: None,
            exponential_maximum_krylov_dimension: None,
            exponential_phi_substeps: None,
            exponential_failure: None,
        });
        t = report.t_new;
        y = report.y_new;
        accepted += 1;
        let _ = controller.record_acceptance(error);
        h *= controller
            .propose_factor(&adaptive, error.max(1.0e-16), 5, true)
            .unwrap_or(1.0);
    }

    let success = t >= tf - tolerance;
    if !success && failure.is_none() {
        failure = Some("maximum attempts reached".into());
    }
    let summary = G4S5B0TrajectorySummary {
        trajectory_id: problem.id,
        family: problem.family.into(),
        dimension: problem.problem.dimension,
        rtol: adaptive.rtol,
        success,
        failure,
        attempts,
        accepted_steps: accepted,
        rejected_steps: rejected,
        endpoint_time: t,
        explicit_jacobian_builds: total_jacobian_builds,
        direct_factorizations: total_factorizations,
        newton_iterations: 0,
    };
    (attempt_rows, accepted_rows, summary)
}

struct RjfAttemptTraceExecution {
    attempt_rows: Vec<G4S5B0RjfAttemptRow>,
    accepted_rows: Vec<G4S5B0StepRow>,
    trajectories: Vec<G4S5B0TrajectorySummary>,
}

fn execute_rjf_attempt_trace_filtered(
    profile: G4S5B0Profile,
    family: Option<G4S5B0Family>,
) -> CoreResult<RjfAttemptTraceExecution> {
    let mut execution = RjfAttemptTraceExecution {
        attempt_rows: Vec::new(),
        accepted_rows: Vec::new(),
        trajectories: Vec::new(),
    };
    for problem in build_problems(profile)? {
        if family.is_some_and(|selected| problem.family != selected.as_str()) {
            continue;
        }
        let (mut attempts, mut accepted, summary) =
            run_rjf_attempt_trace_trajectory(problem, profile);
        execution.attempt_rows.append(&mut attempts);
        execution.accepted_rows.append(&mut accepted);
        execution.trajectories.push(summary);
    }
    Ok(execution)
}

fn run_g4_s5b0_rjf_attempt_trace_filtered(
    profile: G4S5B0Profile,
    family: Option<G4S5B0Family>,
) -> CoreResult<G4S5B0AttemptTraceReport> {
    let RjfAttemptTraceExecution {
        attempt_rows,
        accepted_rows,
        trajectories,
    } = execute_rjf_attempt_trace_filtered(profile, family)?;
    Ok(G4S5B0AttemptTraceReport {
        schema: "g4-s5b0-rjf-attempt-trace-v1",
        status: "read-only-rjf-attempt-trace",
        profile: profile.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        attempt_rows,
        accepted_rows,
        trajectories,
        limitations: vec![
            "No E-K or prefix work is performed by this attempt-level trace.".into(),
            "Wall timings are diagnostic only; policy cost authority remains separately controlled.".into(),
        ],
    })
}

pub fn run_g4_s5b0_rjf_attempt_trace(
    profile: G4S5B0Profile,
) -> CoreResult<G4S5B0AttemptTraceReport> {
    run_g4_s5b0_rjf_attempt_trace_filtered(profile, None)
}

pub fn run_g4_s5b0_rjf_attempt_trace_family(
    profile: G4S5B0Profile,
    family: G4S5B0Family,
) -> CoreResult<G4S5B0AttemptTraceReport> {
    run_g4_s5b0_rjf_attempt_trace_filtered(profile, Some(family))
}

fn finalize_actual_prefix_row(
    rows: &mut Vec<G4S5B0ActualLevel1PrefixRow>,
    row: Option<G4S5B0ActualLevel1PrefixRow>,
    accepted: bool,
    error_norm: Option<f64>,
    recoverable_failure: bool,
) {
    if let Some(mut row) = row {
        row.target_r_attempt_accepted = accepted;
        row.target_r_error_norm = error_norm;
        row.target_r_recoverable_failure = recoverable_failure;
        rows.push(row);
    }
}

fn finalize_actual_level2_prefix_row(
    rows: &mut Vec<G4S5B0ActualLevel2PrefixRow>,
    row: Option<G4S5B0ActualLevel2PrefixRow>,
    accepted: bool,
    error_norm: Option<f64>,
    recoverable_failure: bool,
) {
    if let Some(mut row) = row {
        row.target_r_attempt_accepted = accepted;
        row.target_r_error_norm = error_norm;
        row.target_r_recoverable_failure = recoverable_failure;
        rows.push(row);
    }
}

fn run_rjf_actual_level1_prefix_trajectory(
    problem: AtlasProblem,
    profile: G4S5B0Profile,
    policy: G4S5B0PrefixProbePolicy,
) -> (
    Vec<G4S5B0RjfAttemptRow>,
    Vec<G4S5B0StepRow>,
    Vec<G4S5B0ActualLevel1PrefixRow>,
    G4S5B0TrajectorySummary,
) {
    let adaptive = adaptive_config(profile, problem.t_span.1 - problem.t_span.0);
    let linear = linear_config();
    let mut controller = AdaptiveControllerState::default();
    let mut policy_state =
        PrefixPolicyState::new(policy).expect("sealed persistence policy is valid");
    let mut pending_probe: Option<(usize, Option<f64>)> = None;
    let mut t = problem.t_span.0;
    let tf = problem.t_span.1;
    let mut y = problem.y0.clone();
    let mut h = adaptive.initial_step.min(tf - t);
    let tolerance = 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let mut attempts = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut attempt_rows = Vec::new();
    let mut accepted_rows = Vec::new();
    let mut prefix_rows = Vec::new();
    let mut total_jacobian_builds = 0u64;
    let mut total_factorizations = 0u64;
    let mut failure = None;

    while t < tf - tolerance && attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            failure = Some("minimum step reached".into());
            break;
        }
        let attempt_index = attempts;
        attempts += 1;
        let accepted_steps_before = accepted;
        let t_start = t;
        let h_trial = h;

        // An event observed after the preceding accepted R-JF step acts on this
        // first controller proposal, before we know whether this R trial accepts.
        let mut actual_prefix_row = pending_probe.take().map(|(decision_step, feature_value)| {
            let start = Instant::now();
            let prefix_result = (|| -> CoreResult<Pexprb54s4Level1PrefixReport> {
                let (shadow_problem, state) = shadow_problem_state(&problem.problem, t, &y)?;
                let config = phi_config(adaptive.rtol, shadow_problem.dimension + 4);
                let physical_n = problem.problem.dimension;
                let prefix = pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
                    &shadow_problem,
                    t,
                    &state,
                    h_trial,
                    config,
                    physical_n,
                    adaptive.atol,
                    adaptive.rtol,
                )?;
                Ok(prefix.report().clone())
            })();
            let prefix_wall_seconds = start.elapsed().as_secs_f64();
            let (prefix_succeeded, prefix_failure, prefix_report) = match prefix_result {
                Ok(report) => (true, None, Some(report)),
                Err(error) => (false, Some(error.to_string()), None),
            };
            G4S5B0ActualLevel1PrefixRow {
                trajectory_id: problem.id.clone(),
                family: problem.family.into(),
                dimension: problem.problem.dimension,
                rtol: adaptive.rtol,
                policy: policy.as_str().into(),
                decision_accepted_step: decision_step,
                feature_value,
                target_attempt_index: attempt_index,
                target_accepted_steps_before: accepted_steps_before,
                t_start,
                h: h_trial,
                target_r_attempt_accepted: false,
                target_r_error_norm: None,
                target_r_recoverable_failure: false,
                prefix_wall_seconds,
                prefix_succeeded,
                prefix_failure,
                prefix_report,
                full_e_continued: false,
            }
        });

        let mut step_counters = WorkCounters::default();
        let start = Instant::now();
        let trial = sequential_matrix_free_step(
            &problem.problem,
            t,
            &y,
            h,
            &linear,
            None,
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut step_counters,
        );
        let wall = start.elapsed().as_secs_f64();
        total_jacobian_builds += step_counters.jacobian_builds;
        total_factorizations += step_counters.direct_factorizations;

        let report = match trial {
            Ok(report) => report,
            Err(error @ (CoreError::NonFinite(_) | CoreError::LinearSolve(_))) => {
                attempt_rows.push(G4S5B0RjfAttemptRow {
                    trajectory_id: problem.id.clone(),
                    family: problem.family.into(),
                    dimension: problem.problem.dimension,
                    rtol: adaptive.rtol,
                    attempt_index,
                    accepted_steps_before,
                    t_start,
                    h: h_trial,
                    error_norm: None,
                    accepted: false,
                    recoverable_failure: true,
                    failure: Some(error.to_string()),
                    wall_seconds: wall,
                    rhs_evaluations: step_counters.rhs_evaluations,
                    jvp_vectors: step_counters.jvp_vectors,
                    linear_matvecs: step_counters.linear_matvecs,
                });
                finalize_actual_prefix_row(
                    &mut prefix_rows,
                    actual_prefix_row.take(),
                    false,
                    None,
                    true,
                );
                rejected += 1;
                h *= adaptive.min_factor;
                continue;
            }
            Err(error) => {
                attempt_rows.push(G4S5B0RjfAttemptRow {
                    trajectory_id: problem.id.clone(),
                    family: problem.family.into(),
                    dimension: problem.problem.dimension,
                    rtol: adaptive.rtol,
                    attempt_index,
                    accepted_steps_before,
                    t_start,
                    h: h_trial,
                    error_norm: None,
                    accepted: false,
                    recoverable_failure: false,
                    failure: Some(error.to_string()),
                    wall_seconds: wall,
                    rhs_evaluations: step_counters.rhs_evaluations,
                    jvp_vectors: step_counters.jvp_vectors,
                    linear_matvecs: step_counters.linear_matvecs,
                });
                finalize_actual_prefix_row(
                    &mut prefix_rows,
                    actual_prefix_row.take(),
                    false,
                    None,
                    false,
                );
                failure = Some(error.to_string());
                break;
            }
        };
        let error = report.error_norm;
        let step_accepted = report.accepted
            && error.is_finite()
            && error <= 1.0
            && report.y_new.iter().all(|value| value.is_finite());
        attempt_rows.push(G4S5B0RjfAttemptRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            attempt_index,
            accepted_steps_before,
            t_start,
            h: h_trial,
            error_norm: Some(error),
            accepted: step_accepted,
            recoverable_failure: false,
            failure: None,
            wall_seconds: wall,
            rhs_evaluations: step_counters.rhs_evaluations,
            jvp_vectors: step_counters.jvp_vectors,
            linear_matvecs: step_counters.linear_matvecs,
        });
        finalize_actual_prefix_row(
            &mut prefix_rows,
            actual_prefix_row.take(),
            step_accepted,
            Some(error),
            false,
        );
        if !step_accepted {
            rejected += 1;
            let _ = controller.record_rejection(error.max(1.0e-16));
            h *= controller
                .propose_factor(&adaptive, error.max(1.0e-16), 5, false)
                .unwrap_or(adaptive.min_factor);
            continue;
        }

        accepted_rows.push(G4S5B0StepRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            step_index: accepted,
            t_start: t,
            h,
            transition_level: (problem.transition)(t, &y),
            rodas_embedded_error: error,
            rodas_wall_seconds: wall,
            rodas_rhs_evaluations: step_counters.rhs_evaluations,
            rodas_jvp_vectors: step_counters.jvp_vectors,
            rodas_linear_matvecs: step_counters.linear_matvecs,
            exponential_completed: false,
            exponential_total_error: None,
            exponential_locally_admissible: false,
            exponential_wall_seconds: None,
            exponential_prefix_wall_seconds: None,
            exponential_rhs_evaluations: None,
            exponential_jvp_vectors: None,
            exponential_maximum_krylov_dimension: None,
            exponential_phi_substeps: None,
            exponential_failure: None,
        });

        let decision_step = accepted;
        let event_feature = policy_state.observe_accepted(
            problem.family,
            problem.problem.dimension,
            decision_step,
            error,
        );
        t = report.t_new;
        y = report.y_new;
        accepted += 1;
        let _ = controller.record_acceptance(error);
        h *= controller
            .propose_factor(&adaptive, error.max(1.0e-16), 5, true)
            .unwrap_or(1.0);
        if let Some(feature_value) = event_feature {
            pending_probe = Some((decision_step, feature_value));
        }
    }

    let success = t >= tf - tolerance;
    if !success && failure.is_none() {
        failure = Some("maximum attempts reached".into());
    }
    let summary = G4S5B0TrajectorySummary {
        trajectory_id: problem.id,
        family: problem.family.into(),
        dimension: problem.problem.dimension,
        rtol: adaptive.rtol,
        success,
        failure,
        attempts,
        accepted_steps: accepted,
        rejected_steps: rejected,
        endpoint_time: t,
        explicit_jacobian_builds: total_jacobian_builds,
        direct_factorizations: total_factorizations,
        newton_iterations: 0,
    };
    (attempt_rows, accepted_rows, prefix_rows, summary)
}

fn run_rjf_actual_level2_prefix_trajectory(
    problem: AtlasProblem,
    profile: G4S5B0Profile,
    policy: G4S5B0PrefixProbePolicy,
) -> (
    Vec<G4S5B0RjfAttemptRow>,
    Vec<G4S5B0StepRow>,
    Vec<G4S5B0ActualLevel2PrefixRow>,
    G4S5B0TrajectorySummary,
) {
    let adaptive = adaptive_config(profile, problem.t_span.1 - problem.t_span.0);
    let linear = linear_config();
    let mut controller = AdaptiveControllerState::default();
    let mut policy_state =
        PrefixPolicyState::new(policy).expect("sealed persistence policy is valid");
    let mut pending_probe: Option<(usize, Option<f64>)> = None;
    let mut t = problem.t_span.0;
    let tf = problem.t_span.1;
    let mut y = problem.y0.clone();
    let mut h = adaptive.initial_step.min(tf - t);
    let tolerance = 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let mut attempts = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut attempt_rows = Vec::new();
    let mut accepted_rows = Vec::new();
    let mut prefix_rows = Vec::new();
    let mut total_jacobian_builds = 0u64;
    let mut total_factorizations = 0u64;
    let mut failure = None;

    while t < tf - tolerance && attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            failure = Some("minimum step reached".into());
            break;
        }
        let attempt_index = attempts;
        attempts += 1;
        let accepted_steps_before = accepted;
        let t_start = t;
        let h_trial = h;

        // An event observed after the preceding accepted R-JF step acts on this
        // first controller proposal, before we know whether this R trial accepts.
        let mut actual_prefix_row = pending_probe.take().map(|(decision_step, feature_value)| {
            let start = Instant::now();
            let prefix_result = (|| -> CoreResult<Pexprb54s4Level2PrefixReport> {
                let (shadow_problem, state) = shadow_problem_state(&problem.problem, t, &y)?;
                let config = phi_config(adaptive.rtol, shadow_problem.dimension + 4);
                let physical_n = problem.problem.dimension;
                let level1 = pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
                    &shadow_problem,
                    t,
                    &state,
                    h_trial,
                    config,
                    physical_n,
                    adaptive.atol,
                    adaptive.rtol,
                )?;
                let level2 = pexprb54s4_level2_prefix_resume_level1(
                    level1,
                    &ParallelExecution::sequential(),
                )?;
                Ok(level2.report().clone())
            })();
            let prefix_wall_seconds = start.elapsed().as_secs_f64();
            let (prefix_succeeded, prefix_failure, prefix_report) = match prefix_result {
                Ok(report) => (true, None, Some(report)),
                Err(error) => (false, Some(error.to_string()), None),
            };
            G4S5B0ActualLevel2PrefixRow {
                trajectory_id: problem.id.clone(),
                family: problem.family.into(),
                dimension: problem.problem.dimension,
                rtol: adaptive.rtol,
                policy: policy.as_str().into(),
                decision_accepted_step: decision_step,
                feature_value,
                target_attempt_index: attempt_index,
                target_accepted_steps_before: accepted_steps_before,
                t_start,
                h: h_trial,
                target_r_attempt_accepted: false,
                target_r_error_norm: None,
                target_r_recoverable_failure: false,
                prefix_wall_seconds,
                prefix_succeeded,
                prefix_failure,
                prefix_report,
                full_e_continued: false,
            }
        });

        let mut step_counters = WorkCounters::default();
        let start = Instant::now();
        let trial = sequential_matrix_free_step(
            &problem.problem,
            t,
            &y,
            h,
            &linear,
            None,
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut step_counters,
        );
        let wall = start.elapsed().as_secs_f64();
        total_jacobian_builds += step_counters.jacobian_builds;
        total_factorizations += step_counters.direct_factorizations;

        let report = match trial {
            Ok(report) => report,
            Err(error @ (CoreError::NonFinite(_) | CoreError::LinearSolve(_))) => {
                attempt_rows.push(G4S5B0RjfAttemptRow {
                    trajectory_id: problem.id.clone(),
                    family: problem.family.into(),
                    dimension: problem.problem.dimension,
                    rtol: adaptive.rtol,
                    attempt_index,
                    accepted_steps_before,
                    t_start,
                    h: h_trial,
                    error_norm: None,
                    accepted: false,
                    recoverable_failure: true,
                    failure: Some(error.to_string()),
                    wall_seconds: wall,
                    rhs_evaluations: step_counters.rhs_evaluations,
                    jvp_vectors: step_counters.jvp_vectors,
                    linear_matvecs: step_counters.linear_matvecs,
                });
                finalize_actual_level2_prefix_row(
                    &mut prefix_rows,
                    actual_prefix_row.take(),
                    false,
                    None,
                    true,
                );
                rejected += 1;
                h *= adaptive.min_factor;
                continue;
            }
            Err(error) => {
                attempt_rows.push(G4S5B0RjfAttemptRow {
                    trajectory_id: problem.id.clone(),
                    family: problem.family.into(),
                    dimension: problem.problem.dimension,
                    rtol: adaptive.rtol,
                    attempt_index,
                    accepted_steps_before,
                    t_start,
                    h: h_trial,
                    error_norm: None,
                    accepted: false,
                    recoverable_failure: false,
                    failure: Some(error.to_string()),
                    wall_seconds: wall,
                    rhs_evaluations: step_counters.rhs_evaluations,
                    jvp_vectors: step_counters.jvp_vectors,
                    linear_matvecs: step_counters.linear_matvecs,
                });
                finalize_actual_level2_prefix_row(
                    &mut prefix_rows,
                    actual_prefix_row.take(),
                    false,
                    None,
                    false,
                );
                failure = Some(error.to_string());
                break;
            }
        };
        let error = report.error_norm;
        let step_accepted = report.accepted
            && error.is_finite()
            && error <= 1.0
            && report.y_new.iter().all(|value| value.is_finite());
        attempt_rows.push(G4S5B0RjfAttemptRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            attempt_index,
            accepted_steps_before,
            t_start,
            h: h_trial,
            error_norm: Some(error),
            accepted: step_accepted,
            recoverable_failure: false,
            failure: None,
            wall_seconds: wall,
            rhs_evaluations: step_counters.rhs_evaluations,
            jvp_vectors: step_counters.jvp_vectors,
            linear_matvecs: step_counters.linear_matvecs,
        });
        finalize_actual_level2_prefix_row(
            &mut prefix_rows,
            actual_prefix_row.take(),
            step_accepted,
            Some(error),
            false,
        );
        if !step_accepted {
            rejected += 1;
            let _ = controller.record_rejection(error.max(1.0e-16));
            h *= controller
                .propose_factor(&adaptive, error.max(1.0e-16), 5, false)
                .unwrap_or(adaptive.min_factor);
            continue;
        }

        accepted_rows.push(G4S5B0StepRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            step_index: accepted,
            t_start: t,
            h,
            transition_level: (problem.transition)(t, &y),
            rodas_embedded_error: error,
            rodas_wall_seconds: wall,
            rodas_rhs_evaluations: step_counters.rhs_evaluations,
            rodas_jvp_vectors: step_counters.jvp_vectors,
            rodas_linear_matvecs: step_counters.linear_matvecs,
            exponential_completed: false,
            exponential_total_error: None,
            exponential_locally_admissible: false,
            exponential_wall_seconds: None,
            exponential_prefix_wall_seconds: None,
            exponential_rhs_evaluations: None,
            exponential_jvp_vectors: None,
            exponential_maximum_krylov_dimension: None,
            exponential_phi_substeps: None,
            exponential_failure: None,
        });

        let decision_step = accepted;
        let event_feature = policy_state.observe_accepted(
            problem.family,
            problem.problem.dimension,
            decision_step,
            error,
        );
        t = report.t_new;
        y = report.y_new;
        accepted += 1;
        let _ = controller.record_acceptance(error);
        h *= controller
            .propose_factor(&adaptive, error.max(1.0e-16), 5, true)
            .unwrap_or(1.0);
        if let Some(feature_value) = event_feature {
            pending_probe = Some((decision_step, feature_value));
        }
    }

    let success = t >= tf - tolerance;
    if !success && failure.is_none() {
        failure = Some("maximum attempts reached".into());
    }
    let summary = G4S5B0TrajectorySummary {
        trajectory_id: problem.id,
        family: problem.family.into(),
        dimension: problem.problem.dimension,
        rtol: adaptive.rtol,
        success,
        failure,
        attempts,
        accepted_steps: accepted,
        rejected_steps: rejected,
        endpoint_time: t,
        explicit_jacobian_builds: total_jacobian_builds,
        direct_factorizations: total_factorizations,
        newton_iterations: 0,
    };
    (attempt_rows, accepted_rows, prefix_rows, summary)
}

fn run_g4_s5b0_actual_level1_prefix_filtered(
    profile: G4S5B0Profile,
    family: Option<G4S5B0Family>,
    policy: G4S5B0PrefixProbePolicy,
) -> CoreResult<G4S5B0ActualLevel1PrefixReport> {
    if matches!(profile, G4S5B0Profile::Canonical) {
        return Err(CoreError::InvalidInput(
            "v2.7 prefix research requires an explicit single-dimension profile".into(),
        ));
    }
    let mut attempt_rows = Vec::new();
    let mut accepted_rows = Vec::new();
    let mut prefix_rows = Vec::new();
    let mut trajectories = Vec::new();
    for problem in build_problems(profile)? {
        if family.is_some_and(|selected| problem.family != selected.as_str()) {
            continue;
        }
        let (mut local_attempts, mut local_accepted, mut local_prefix, summary) =
            run_rjf_actual_level1_prefix_trajectory(problem, profile, policy);
        attempt_rows.append(&mut local_attempts);
        accepted_rows.append(&mut local_accepted);
        prefix_rows.append(&mut local_prefix);
        trajectories.push(summary);
    }
    let status = if trajectories.iter().all(|row| row.success)
        && prefix_rows.iter().all(|row| row.prefix_succeeded)
    {
        "complete"
    } else {
        "complete-with-failures"
    };
    Ok(G4S5B0ActualLevel1PrefixReport {
        schema: "g4-s5b0-actual-pexprb-level1-prefix-v1",
        status,
        profile: profile.as_str(),
        policy: policy.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        full_e_continuations: 0,
        attempt_rows,
        accepted_rows,
        prefix_rows,
        trajectories,
        limitations: vec![
            "Only the actual pexprb54s4 U2/D2 dependency-level-1 prefix is evaluated at event targets; U3/U4 and endpoints are never computed.".into(),
            "The committed trajectory remains protected R-JF and prefix failures cannot modify its controller or state.".into(),
            "Debug wall timings are diagnostic only and are not promotion authority.".into(),
            "N=2048 remains sealed in this node.".into(),
        ],
    })
}

pub fn run_g4_s5b0_actual_level1_prefix_family(
    profile: G4S5B0Profile,
    family: G4S5B0Family,
    policy: G4S5B0PrefixProbePolicy,
) -> CoreResult<G4S5B0ActualLevel1PrefixReport> {
    run_g4_s5b0_actual_level1_prefix_filtered(profile, Some(family), policy)
}

fn run_g4_s5b0_actual_level2_prefix_filtered(
    profile: G4S5B0Profile,
    family: Option<G4S5B0Family>,
    policy: G4S5B0PrefixProbePolicy,
) -> CoreResult<G4S5B0ActualLevel2PrefixReport> {
    if matches!(profile, G4S5B0Profile::Canonical) {
        return Err(CoreError::InvalidInput(
            "v2.8 staged prefix research requires an explicit single-dimension profile".into(),
        ));
    }
    let mut attempt_rows = Vec::new();
    let mut accepted_rows = Vec::new();
    let mut prefix_rows = Vec::new();
    let mut trajectories = Vec::new();
    for problem in build_problems(profile)? {
        if family.is_some_and(|selected| problem.family != selected.as_str()) {
            continue;
        }
        let (mut local_attempts, mut local_accepted, mut local_prefix, summary) =
            run_rjf_actual_level2_prefix_trajectory(problem, profile, policy);
        attempt_rows.append(&mut local_attempts);
        accepted_rows.append(&mut local_accepted);
        prefix_rows.append(&mut local_prefix);
        trajectories.push(summary);
    }
    let status = if trajectories.iter().all(|row| row.success)
        && prefix_rows.iter().all(|row| row.prefix_succeeded)
    {
        "complete"
    } else {
        "complete-with-failures"
    };
    Ok(G4S5B0ActualLevel2PrefixReport {
        schema: "g4-s5b0-actual-pexprb-level2-prefix-v1",
        status,
        profile: profile.as_str(),
        policy: policy.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        full_e_continuations: 0,
        attempt_rows,
        accepted_rows,
        prefix_rows,
        trajectories,
        limitations: vec![
            "Only pexprb54s4 dependency levels 1 and 2 (U2/D2 then U3/D3 and U4/D4) are evaluated at event targets; main and embedded endpoints are never computed.".into(),
            "The committed trajectory remains protected R-JF and staged-prefix failures cannot modify its controller or state.".into(),
            "Debug wall timings are diagnostic only and are not promotion authority.".into(),
            "N=2048 remains sealed and no final safety threshold is selected in this node.".into(),
        ],
    })
}

pub fn run_g4_s5b0_actual_level2_prefix_family(
    profile: G4S5B0Profile,
    family: G4S5B0Family,
    policy: G4S5B0PrefixProbePolicy,
) -> CoreResult<G4S5B0ActualLevel2PrefixReport> {
    run_g4_s5b0_actual_level2_prefix_filtered(profile, Some(family), policy)
}

const V29_PREFIX_RESERVE_JVP: u64 = 80;
const V29_PREFIX_BUDGET_FRACTION: f64 = 0.25;

pub fn enforced_prefix_jvp_cap(committed_rjf_jvp: u64, speculative_jvp: u64) -> u64 {
    let cumulative_limit = committed_rjf_jvp / 4;
    V29_PREFIX_RESERVE_JVP.min(cumulative_limit.saturating_sub(speculative_jvp))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StageGrowthBudgetMode {
    Predictive,
    Enforced,
}
const V29_STAGE_GROWTH_BASELINE: f64 = 3.24;

fn stage_trajectory_shape_features(rho2: f64, rho3: f64, rho4: f64) -> Option<(f64, f64, f64)> {
    if !rho2.is_finite()
        || !rho3.is_finite()
        || !rho4.is_finite()
        || rho2 <= 0.0
        || rho3 <= 0.0
        || rho4 <= 0.0
    {
        return None;
    }
    let s23 = (rho3 / rho2).ln() / (0.5_f64 / 0.25_f64).ln();
    let s34 = (rho4 / rho3).ln() / (0.9_f64 / 0.5_f64).ln();
    let kappa234 = s34 - s23;
    if s23.is_finite() && s34.is_finite() && kappa234.is_finite() {
        Some((s23, s34, kappa234))
    } else {
        None
    }
}

#[derive(Clone, Debug)]
struct StageGrowthAuditSample {
    prefix_jvp: u64,
    prefix_work: WorkCounters,
    a34: Option<f64>,
    rho2: Option<f64>,
    rho3: Option<f64>,
    rho4: Option<f64>,
    shape: Option<(f64, f64, f64)>,
    vector_geometry: Option<Pexprb54s4RemainderVectorGeometry>,
    quadratic_drift: Option<Pexprb54s4QuadraticRemainderDrift>,
    total_error: f64,
    audit_work: WorkCounters,
}

enum StageGrowthPrefixResolution {
    Complete(Box<StageGrowthAuditSample>),
    BudgetExhausted {
        used_jvp: u64,
        work: Box<WorkCounters>,
    },
}

fn finish_stage_growth_audit(
    level2: Pexprb54s4Level2Prefix,
    y: &[f64],
    physical_n: usize,
    adaptive: &AdaptiveStepConfig,
    h_trial: f64,
) -> CoreResult<StageGrowthAuditSample> {
    let prefix_report = level2.report().clone();
    let prefix_jvp = prefix_report.cumulative_work.jvp_vectors;
    let rho2 = prefix_report
        .level1_report
        .early_flow_defect
        .as_ref()
        .and_then(|entry| entry.tolerance_scaled_defect_wrms);
    let rho3 = prefix_report
        .stage3_flow_defect
        .as_ref()
        .and_then(|entry| entry.tolerance_scaled_defect_wrms);
    let rho4 = prefix_report
        .stage4_flow_defect
        .as_ref()
        .and_then(|entry| entry.tolerance_scaled_defect_wrms);
    let shape = match (rho2, rho3, rho4) {
        (Some(r2), Some(r3), Some(r4)) => stage_trajectory_shape_features(r2, r3, r4),
        _ => None,
    };
    let a34 = match (rho3, rho4) {
        (Some(r3), Some(r4)) if r3.is_finite() && r3 > 0.0 && r4.is_finite() && r4 >= 0.0 => {
            Some((r4 / r3) / V29_STAGE_GROWTH_BASELINE)
        }
        _ => None,
    };
    let full = pexprb54s4_fused_step_resume_level2(level2, &ParallelExecution::sequential())?;
    let physical_y_new = &full.y_new[..physical_n];
    let error_vector = full
        .error_estimate
        .as_ref()
        .ok_or_else(|| CoreError::InvalidInput("pexprb54s4 omitted embedded error".into()))?;
    let scale = error_scale(y, physical_y_new, &[adaptive.atol], adaptive.rtol)?;
    let time_error = wrms(&error_vector[..physical_n], &scale)?;
    let phi_error = h_trial.abs()
        * full
            .fused_phi_reports
            .iter()
            .map(|entry| entry.error_estimate)
            .filter(|value| value.is_finite())
            .sum::<f64>()
        / safe_l2(&scale).max(f64::MIN_POSITIVE);
    Ok(StageGrowthAuditSample {
        prefix_jvp,
        prefix_work: prefix_report.cumulative_work,
        a34,
        rho2,
        rho3,
        rho4,
        shape,
        vector_geometry: prefix_report.remainder_vector_geometry,
        quadratic_drift: prefix_report.quadratic_remainder_drift,
        total_error: time_error.max(phi_error),
        audit_work: full.work,
    })
}

fn run_rjf_stage_growth_safety_trajectory(
    problem: AtlasProblem,
    profile: G4S5B0Profile,
    budget_mode: StageGrowthBudgetMode,
) -> (
    Vec<G4S5B0RjfAttemptRow>,
    Vec<G4S5B0StepRow>,
    Vec<G4S5B0StageGrowthSafetyRow>,
    G4S5B0TrajectorySummary,
) {
    let adaptive = adaptive_config(profile, problem.t_span.1 - problem.t_span.0);
    let linear = linear_config();
    let mut controller = AdaptiveControllerState::default();
    let mut policy_state = PrefixPolicyState::new(G4S5B0PrefixProbePolicy::K3Development)
        .expect("sealed k=3 policy is valid");
    let mut pending_probe: Option<(usize, Option<f64>)> = None;
    let mut t = problem.t_span.0;
    let tf = problem.t_span.1;
    let mut y = problem.y0.clone();
    let mut h = adaptive.initial_step.min(tf - t);
    let tolerance = 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let mut attempts = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut attempt_rows = Vec::new();
    let mut accepted_rows = Vec::new();
    let mut safety_rows = Vec::new();
    let mut total_jacobian_builds = 0u64;
    let mut total_factorizations = 0u64;
    let mut committed_rjf_jvp = 0u64;
    let mut speculative_jvp = 0u64;
    let mut failure = None;

    while t < tf - tolerance && attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            failure = Some("minimum step reached".into());
            break;
        }
        let attempt_index = attempts;
        attempts += 1;
        let accepted_steps_before = accepted;
        let t_start = t;
        let h_trial = h;

        let mut safety_row = pending_probe.take().map(|(decision_step, feature_value)| {
            let budget_cap_jvp = match budget_mode {
                StageGrowthBudgetMode::Predictive => {
                    let reserve_ok = (speculative_jvp + V29_PREFIX_RESERVE_JVP) as f64
                        <= V29_PREFIX_BUDGET_FRACTION * committed_rjf_jvp as f64;
                    if reserve_ok { V29_PREFIX_RESERVE_JVP } else { 0 }
                }
                StageGrowthBudgetMode::Enforced => {
                    enforced_prefix_jvp_cap(committed_rjf_jvp, speculative_jvp)
                }
            };
            let reserve_ok = budget_cap_jvp > 0;
            let mut row = G4S5B0StageGrowthSafetyRow {
                trajectory_id: problem.id.clone(),
                family: problem.family.into(),
                dimension: problem.problem.dimension,
                rtol: adaptive.rtol,
                decision_accepted_step: decision_step,
                feature_value,
                target_attempt_index: attempt_index,
                target_accepted_steps_before: accepted_steps_before,
                t_start,
                h: h_trial,
                target_r_attempt_accepted: false,
                target_r_error_norm: None,
                committed_rjf_jvp_before_target: committed_rjf_jvp,
                speculative_jvp_before_target: speculative_jvp,
                budget_reserve_jvp: V29_PREFIX_RESERVE_JVP,
                budget_cap_jvp,
                budget_fraction: V29_PREFIX_BUDGET_FRACTION,
                budget_admitted: reserve_ok,
                budget_exhausted: false,
                prefix_succeeded: false,
                prefix_failure: None,
                actual_prefix_jvp_vectors: None,
                prefix_work: None,
                normalized_stage_growth_a34: None,
                rho2: None,
                rho3: None,
                rho4: None,
                stage_log_slope_s23: None,
                stage_log_slope_s34: None,
                stage_log_curvature_kappa234: None,
                remainder_chi23: None,
                remainder_chi34: None,
                remainder_chi24: None,
                remainder_q34_perp: None,
                remainder_delta_chi: None,
                quadratic_drift_zeta23: None,
                quadratic_drift_zeta34: None,
                quadratic_drift_relative: None,
                budget_breached: false,
                audit_full_e_completed: false,
                audit_full_e_total_error: None,
                audit_full_e_locally_admissible: false,
                audit_full_e_failure: None,
                audit_full_e_work: None,
                runtime_full_e_continued: false,
            };

            if reserve_ok {
                let prefix_resolution = (|| -> CoreResult<StageGrowthPrefixResolution> {
                    let (shadow_problem, state) = shadow_problem_state(&problem.problem, t, &y)?;
                    let config = phi_config(adaptive.rtol, shadow_problem.dimension + 4);
                    let physical_n = problem.problem.dimension;
                    match budget_mode {
                        StageGrowthBudgetMode::Predictive => {
                            let level1 = pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
                                &shadow_problem,
                                t,
                                &state,
                                h_trial,
                                config,
                                physical_n,
                                adaptive.atol,
                                adaptive.rtol,
                            )?;
                            let level2 = pexprb54s4_level2_prefix_resume_level1(
                                level1,
                                &ParallelExecution::sequential(),
                            )?;
                            Ok(StageGrowthPrefixResolution::Complete(Box::new(
                                finish_stage_growth_audit(
                                    level2,
                                    &y,
                                    physical_n,
                                    &adaptive,
                                    h_trial,
                                )?,
                            )))
                        }
                        StageGrowthBudgetMode::Enforced => {
                            match pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget(
                                &shadow_problem,
                                t,
                                &state,
                                h_trial,
                                config,
                                physical_n,
                                adaptive.atol,
                                adaptive.rtol,
                                budget_cap_jvp,
                            )? {
                                Pexprb54s4BudgetedLevel2PrefixOutcome::Complete(level2) => {
                                    Ok(StageGrowthPrefixResolution::Complete(Box::new(
                                        finish_stage_growth_audit(
                                            *level2,
                                            &y,
                                            physical_n,
                                            &adaptive,
                                            h_trial,
                                        )?,
                                    )))
                                }
                                Pexprb54s4BudgetedLevel2PrefixOutcome::BudgetExhausted(report) => {
                                    Ok(StageGrowthPrefixResolution::BudgetExhausted {
                                        used_jvp: report.used_jvp_vectors,
                                        work: Box::new(report.work),
                                    })
                                }
                            }
                        }
                    }
                })();
                match prefix_resolution {
                    Ok(StageGrowthPrefixResolution::Complete(sample)) => {
                        row.prefix_succeeded = true;
                        row.actual_prefix_jvp_vectors = Some(sample.prefix_jvp);
                        row.prefix_work = Some(sample.prefix_work);
                        row.normalized_stage_growth_a34 = sample.a34;
                        row.rho2 = sample.rho2;
                        row.rho3 = sample.rho3;
                        row.rho4 = sample.rho4;
                        if let Some((s23, s34, kappa234)) = sample.shape {
                            row.stage_log_slope_s23 = Some(s23);
                            row.stage_log_slope_s34 = Some(s34);
                            row.stage_log_curvature_kappa234 = Some(kappa234);
                        }
                        if let Some(geometry) = sample.vector_geometry {
                            row.remainder_chi23 = geometry.chi23;
                            row.remainder_chi34 = geometry.chi34;
                            row.remainder_chi24 = geometry.chi24;
                            row.remainder_q34_perp = geometry.q34_perp;
                            row.remainder_delta_chi = geometry.delta_chi;
                        }
                        if let Some(drift) = sample.quadratic_drift {
                            row.quadratic_drift_zeta23 = drift.zeta23;
                            row.quadratic_drift_zeta34 = drift.zeta34;
                            row.quadratic_drift_relative = drift.relative_drift;
                        }
                        row.budget_breached = match budget_mode {
                            StageGrowthBudgetMode::Predictive => {
                                sample.prefix_jvp > V29_PREFIX_RESERVE_JVP
                                    || (speculative_jvp + sample.prefix_jvp) as f64
                                        > V29_PREFIX_BUDGET_FRACTION * committed_rjf_jvp as f64
                            }
                            StageGrowthBudgetMode::Enforced => {
                                sample.prefix_jvp > budget_cap_jvp
                            }
                        };
                        speculative_jvp += sample.prefix_jvp;
                        row.audit_full_e_completed = true;
                        row.audit_full_e_total_error = Some(sample.total_error);
                        row.audit_full_e_locally_admissible =
                            sample.total_error.is_finite() && sample.total_error <= 1.0;
                        row.audit_full_e_work = Some(sample.audit_work);
                    }
                    Ok(StageGrowthPrefixResolution::BudgetExhausted { used_jvp, work }) => {
                        row.budget_exhausted = true;
                        row.prefix_failure = Some("budget-exhausted".into());
                        row.actual_prefix_jvp_vectors = Some(used_jvp);
                        row.prefix_work = Some(*work);
                        row.budget_breached = used_jvp > budget_cap_jvp;
                        speculative_jvp += used_jvp;
                    }
                    Err(error) => {
                        row.prefix_failure = Some(error.to_string());
                        row.audit_full_e_failure = Some(error.to_string());
                    }
                }
            }
            row
        });

        let mut step_counters = WorkCounters::default();
        let start = Instant::now();
        let trial = sequential_matrix_free_step(
            &problem.problem,
            t,
            &y,
            h,
            &linear,
            None,
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut step_counters,
        );
        let wall = start.elapsed().as_secs_f64();
        committed_rjf_jvp += step_counters.jvp_vectors;
        total_jacobian_builds += step_counters.jacobian_builds;
        total_factorizations += step_counters.direct_factorizations;

        let report = match trial {
            Ok(report) => report,
            Err(error @ (CoreError::NonFinite(_) | CoreError::LinearSolve(_))) => {
                attempt_rows.push(G4S5B0RjfAttemptRow {
                    trajectory_id: problem.id.clone(),
                    family: problem.family.into(),
                    dimension: problem.problem.dimension,
                    rtol: adaptive.rtol,
                    attempt_index,
                    accepted_steps_before,
                    t_start,
                    h: h_trial,
                    error_norm: None,
                    accepted: false,
                    recoverable_failure: true,
                    failure: Some(error.to_string()),
                    wall_seconds: wall,
                    rhs_evaluations: step_counters.rhs_evaluations,
                    jvp_vectors: step_counters.jvp_vectors,
                    linear_matvecs: step_counters.linear_matvecs,
                });
                if let Some(mut row) = safety_row.take() {
                    row.target_r_attempt_accepted = false;
                    safety_rows.push(row);
                }
                rejected += 1;
                h *= adaptive.min_factor;
                continue;
            }
            Err(error) => {
                failure = Some(error.to_string());
                break;
            }
        };
        let error = report.error_norm;
        let step_accepted = report.accepted
            && error.is_finite()
            && error <= 1.0
            && report.y_new.iter().all(|value| value.is_finite());
        attempt_rows.push(G4S5B0RjfAttemptRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            attempt_index,
            accepted_steps_before,
            t_start,
            h: h_trial,
            error_norm: Some(error),
            accepted: step_accepted,
            recoverable_failure: false,
            failure: None,
            wall_seconds: wall,
            rhs_evaluations: step_counters.rhs_evaluations,
            jvp_vectors: step_counters.jvp_vectors,
            linear_matvecs: step_counters.linear_matvecs,
        });
        if let Some(mut row) = safety_row.take() {
            row.target_r_attempt_accepted = step_accepted;
            row.target_r_error_norm = Some(error);
            safety_rows.push(row);
        }
        if !step_accepted {
            rejected += 1;
            let _ = controller.record_rejection(error.max(1.0e-16));
            h *= controller
                .propose_factor(&adaptive, error.max(1.0e-16), 5, false)
                .unwrap_or(adaptive.min_factor);
            continue;
        }

        accepted_rows.push(G4S5B0StepRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            step_index: accepted,
            t_start: t,
            h,
            transition_level: (problem.transition)(t, &y),
            rodas_embedded_error: error,
            rodas_wall_seconds: wall,
            rodas_rhs_evaluations: step_counters.rhs_evaluations,
            rodas_jvp_vectors: step_counters.jvp_vectors,
            rodas_linear_matvecs: step_counters.linear_matvecs,
            exponential_completed: false,
            exponential_total_error: None,
            exponential_locally_admissible: false,
            exponential_wall_seconds: None,
            exponential_prefix_wall_seconds: None,
            exponential_rhs_evaluations: None,
            exponential_jvp_vectors: None,
            exponential_maximum_krylov_dimension: None,
            exponential_phi_substeps: None,
            exponential_failure: None,
        });
        let decision_step = accepted;
        let event_feature = policy_state.observe_accepted(
            problem.family,
            problem.problem.dimension,
            decision_step,
            error,
        );
        t = report.t_new;
        y = report.y_new;
        accepted += 1;
        let _ = controller.record_acceptance(error);
        h *= controller
            .propose_factor(&adaptive, error.max(1.0e-16), 5, true)
            .unwrap_or(1.0);
        if let Some(feature_value) = event_feature {
            pending_probe = Some((decision_step, feature_value));
        }
    }

    let success = t >= tf - tolerance;
    if !success && failure.is_none() {
        failure = Some("maximum attempts reached".into());
    }
    let summary = G4S5B0TrajectorySummary {
        trajectory_id: problem.id,
        family: problem.family.into(),
        dimension: problem.problem.dimension,
        rtol: adaptive.rtol,
        success,
        failure,
        attempts,
        accepted_steps: accepted,
        rejected_steps: rejected,
        endpoint_time: t,
        explicit_jacobian_builds: total_jacobian_builds,
        direct_factorizations: total_factorizations,
        newton_iterations: 0,
    };
    (attempt_rows, accepted_rows, safety_rows, summary)
}

pub fn run_g4_s5b0_stage_growth_safety_audit_family(
    profile: G4S5B0Profile,
    family: G4S5B0Family,
) -> CoreResult<G4S5B0StageGrowthSafetyReport> {
    if matches!(profile, G4S5B0Profile::Canonical) {
        return Err(CoreError::InvalidInput(
            "v2.9 safety calibration requires an explicit single-dimension profile".into(),
        ));
    }
    let mut attempt_rows = Vec::new();
    let mut accepted_rows = Vec::new();
    let mut rows = Vec::new();
    let mut trajectories = Vec::new();
    for problem in build_problems(profile)? {
        if problem.family != family.as_str() {
            continue;
        }
        let (mut attempts, mut accepted, mut local_rows, summary) =
            run_rjf_stage_growth_safety_trajectory(
                problem,
                profile,
                StageGrowthBudgetMode::Predictive,
            );
        attempt_rows.append(&mut attempts);
        accepted_rows.append(&mut accepted);
        rows.append(&mut local_rows);
        trajectories.push(summary);
    }
    let budget_breaches = rows.iter().filter(|row| row.budget_breached).count();
    let audit_full_e_continuations = rows.iter().filter(|row| row.audit_full_e_completed).count();
    let status = if trajectories.iter().all(|row| row.success)
        && rows.iter().all(|row| !row.budget_breached)
        && rows
            .iter()
            .filter(|row| row.budget_admitted)
            .all(|row| row.prefix_succeeded)
    {
        "complete"
    } else {
        "complete-with-failures"
    };
    Ok(G4S5B0StageGrowthSafetyReport {
        schema: "g4-s5b0-stage-growth-safety-audit-v1",
        status,
        profile: profile.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        runtime_full_e_continuations: 0,
        audit_full_e_continuations,
        budget_breaches,
        budget_exhaustions: rows.iter().filter(|row| row.budget_exhausted).count(),
        attempt_rows,
        accepted_rows,
        rows,
        trajectories,
        limitations: vec![
            "The k=3 event gate is frozen upstream and uses only committed R-JF accepted-step history.".into(),
            "Level1+2 prefix work is purchased only under the predeclared JVP token reserve; budget-denied events abstain to R-JF.".into(),
            "Full E-K endpoint work is explicit offline audit labeling only and never mutates the R-JF state or controller.".into(),
            "The runtime policy performs zero full-E continuations and no active switching.".into(),
            "N=2048 remains sealed.".into(),
        ],
    })
}

pub fn run_g4_s5b0_enforced_prefix_budget_family(
    profile: G4S5B0Profile,
    family: G4S5B0Family,
) -> CoreResult<G4S5B0StageGrowthSafetyReport> {
    if matches!(profile, G4S5B0Profile::Canonical) {
        return Err(CoreError::InvalidInput(
            "v3.5 enforced prefix budget requires an explicit single-dimension profile".into(),
        ));
    }
    let mut attempt_rows = Vec::new();
    let mut accepted_rows = Vec::new();
    let mut rows = Vec::new();
    let mut trajectories = Vec::new();
    for problem in build_problems(profile)? {
        if problem.family != family.as_str() {
            continue;
        }
        let (mut attempts, mut accepted, mut local_rows, summary) =
            run_rjf_stage_growth_safety_trajectory(
                problem,
                profile,
                StageGrowthBudgetMode::Enforced,
            );
        attempt_rows.append(&mut attempts);
        accepted_rows.append(&mut accepted);
        rows.append(&mut local_rows);
        trajectories.push(summary);
    }
    let budget_breaches = rows.iter().filter(|row| row.budget_breached).count();
    let budget_exhaustions = rows.iter().filter(|row| row.budget_exhausted).count();
    let audit_full_e_continuations = rows.iter().filter(|row| row.audit_full_e_completed).count();
    let status = if trajectories.iter().all(|row| row.success)
        && budget_breaches == 0
        && rows
            .iter()
            .filter(|row| row.budget_admitted)
            .all(|row| row.prefix_succeeded || row.budget_exhausted)
    {
        "complete"
    } else {
        "complete-with-failures"
    };
    Ok(G4S5B0StageGrowthSafetyReport {
        schema: "g4-s5b0-enforced-prefix-budget-v1",
        status,
        profile: profile.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        runtime_full_e_continuations: 0,
        audit_full_e_continuations,
        budget_breaches,
        budget_exhaustions,
        attempt_rows,
        accepted_rows,
        rows,
        trajectories,
        limitations: vec![
            "The k=3 event gate and zeta34 threshold are frozen upstream; v3.5 changes only speculative-prefix budget semantics.".into(),
            "Each event uses B_k=min(80,floor(0.25*committed_R_JVP-speculative_JVP)); the prefix guard refuses JVP B_k+1 before operator application.".into(),
            "Budget exhaustion is a charged read-only abstention: no completed zeta34 witness and no full-E endpoint audit are emitted for that event.".into(),
            "The committed R-JF trajectory/controller remains authoritative; runtime full-E continuation and active switching are zero.".into(),
            "N=2048 remains sealed.".into(),
        ],
    })
}

fn v36_profile_is_consumed(profile: G4S5B0Profile) -> bool {
    matches!(
        profile,
        G4S5B0Profile::StageGrowthCalibration96
            | G4S5B0Profile::StageGrowthCalibration192
            | G4S5B0Profile::StageGrowthCalibration256
            | G4S5B0Profile::EnforcedBudgetHoldout320
            | G4S5B0Profile::StageGrowthHoldout384
    )
}

fn fill_frozen_shadow_prefix_evidence(
    row: &mut G4S5B0FrozenFullEShadowRow,
    report: &Pexprb54s4Level2PrefixReport,
) {
    let rho2 = report
        .level1_report
        .early_flow_defect
        .as_ref()
        .and_then(|entry| entry.tolerance_scaled_defect_wrms);
    let rho3 = report
        .stage3_flow_defect
        .as_ref()
        .and_then(|entry| entry.tolerance_scaled_defect_wrms);
    let rho4 = report
        .stage4_flow_defect
        .as_ref()
        .and_then(|entry| entry.tolerance_scaled_defect_wrms);
    row.rho2 = rho2;
    row.rho3 = rho3;
    row.rho4 = rho4;
    if let (Some(r2), Some(r3), Some(r4)) = (rho2, rho3, rho4)
        && let Some((s23, s34, kappa234)) = stage_trajectory_shape_features(r2, r3, r4)
    {
        row.stage_log_slope_s23 = Some(s23);
        row.stage_log_slope_s34 = Some(s34);
        row.stage_log_curvature_kappa234 = Some(kappa234);
    }
    row.normalized_stage_growth_a34 = match (rho3, rho4) {
        (Some(r3), Some(r4)) if r3.is_finite() && r3 > 0.0 && r4.is_finite() && r4 >= 0.0 => {
            Some((r4 / r3) / V29_STAGE_GROWTH_BASELINE)
        }
        _ => None,
    };
    if let Some(geometry) = &report.remainder_vector_geometry {
        row.remainder_chi23 = geometry.chi23;
        row.remainder_chi34 = geometry.chi34;
        row.remainder_chi24 = geometry.chi24;
        row.remainder_q34_perp = geometry.q34_perp;
        row.remainder_delta_chi = geometry.delta_chi;
    }
    if let Some(drift) = &report.quadratic_remainder_drift {
        row.quadratic_drift_zeta23 = drift.zeta23;
        row.quadratic_drift_zeta34 = drift.zeta34;
        row.quadratic_drift_relative = drift.relative_drift;
    }
}

fn frozen_shadow_total_error(
    full: &crate::FusedExponentialStepReport,
    y: &[f64],
    physical_n: usize,
    adaptive: &AdaptiveStepConfig,
    h_trial: f64,
) -> CoreResult<f64> {
    let physical_y_new = &full.y_new[..physical_n];
    let error_vector = full
        .error_estimate
        .as_ref()
        .ok_or_else(|| CoreError::InvalidInput("pexprb54s4 omitted embedded error".into()))?;
    let scale = error_scale(y, physical_y_new, &[adaptive.atol], adaptive.rtol)?;
    let time_error = wrms(&error_vector[..physical_n], &scale)?;
    let phi_error = h_trial.abs()
        * full
            .fused_phi_reports
            .iter()
            .map(|entry| entry.error_estimate)
            .filter(|value| value.is_finite())
            .sum::<f64>()
        / safe_l2(&scale).max(f64::MIN_POSITIVE);
    Ok(time_error.max(phi_error))
}

fn exact_continuation_roundtrip(
    prefix: WorkCounters,
    continuation: WorkCounters,
    cumulative: WorkCounters,
) -> bool {
    if cumulative.checked_delta(prefix) != Some(continuation) {
        return false;
    }
    let mut reconstructed = prefix;
    reconstructed.accumulate(continuation);
    reconstructed == cumulative
}

fn optional_f64_bits_equal(left: Option<f64>, right: Option<f64>) -> bool {
    left.map(f64::to_bits) == right.map(f64::to_bits)
}

fn attempt_row_exact_excluding_wall(
    left: &G4S5B0RjfAttemptRow,
    right: &G4S5B0RjfAttemptRow,
) -> bool {
    if left.rtol.to_bits() != right.rtol.to_bits()
        || left.t_start.to_bits() != right.t_start.to_bits()
        || left.h.to_bits() != right.h.to_bits()
        || !optional_f64_bits_equal(left.error_norm, right.error_norm)
    {
        return false;
    }
    let mut left = left.clone();
    let mut right = right.clone();
    left.wall_seconds = 0.0;
    right.wall_seconds = 0.0;
    left == right
}

fn accepted_row_exact_excluding_wall(left: &G4S5B0StepRow, right: &G4S5B0StepRow) -> bool {
    if left.rtol.to_bits() != right.rtol.to_bits()
        || left.t_start.to_bits() != right.t_start.to_bits()
        || left.h.to_bits() != right.h.to_bits()
        || left.transition_level.to_bits() != right.transition_level.to_bits()
        || left.rodas_embedded_error.to_bits() != right.rodas_embedded_error.to_bits()
        || !optional_f64_bits_equal(left.exponential_total_error, right.exponential_total_error)
        || !optional_f64_bits_equal(
            left.exponential_wall_seconds,
            right.exponential_wall_seconds,
        )
        || !optional_f64_bits_equal(
            left.exponential_prefix_wall_seconds,
            right.exponential_prefix_wall_seconds,
        )
    {
        return false;
    }
    let mut left = left.clone();
    let mut right = right.clone();
    left.rodas_wall_seconds = 0.0;
    right.rodas_wall_seconds = 0.0;
    left == right
}

fn trajectory_exact(left: &G4S5B0TrajectorySummary, right: &G4S5B0TrajectorySummary) -> bool {
    left.rtol.to_bits() == right.rtol.to_bits()
        && left.endpoint_time.to_bits() == right.endpoint_time.to_bits()
        && left == right
}

fn rjf_parity(
    attempts: &[G4S5B0RjfAttemptRow],
    accepted: &[G4S5B0StepRow],
    trajectories: &[G4S5B0TrajectorySummary],
    reference: &G4S5B0AttemptTraceReport,
) -> G4S5B0RjfParitySummary {
    let attempt_rows_exact_excluding_wall = attempts.len() == reference.attempt_rows.len()
        && attempts
            .iter()
            .zip(&reference.attempt_rows)
            .all(|(left, right)| attempt_row_exact_excluding_wall(left, right));
    let accepted_rows_exact_excluding_wall = accepted.len() == reference.accepted_rows.len()
        && accepted
            .iter()
            .zip(&reference.accepted_rows)
            .all(|(left, right)| accepted_row_exact_excluding_wall(left, right));
    let trajectories_exact = trajectories.len() == reference.trajectories.len()
        && trajectories
            .iter()
            .zip(&reference.trajectories)
            .all(|(left, right)| trajectory_exact(left, right));
    G4S5B0RjfParitySummary {
        attempt_rows_exact_excluding_wall,
        accepted_rows_exact_excluding_wall,
        trajectories_exact,
        passed: attempt_rows_exact_excluding_wall
            && accepted_rows_exact_excluding_wall
            && trajectories_exact,
    }
}

fn finalize_frozen_shadow_target(
    row: &mut G4S5B0FrozenFullEShadowRow,
    accepted: bool,
    error_norm: Option<f64>,
    recoverable_failure: bool,
    wall_seconds: f64,
    target_jvp_vectors: u64,
) {
    row.target_r_attempt_accepted = accepted;
    row.target_r_error_norm = error_norm;
    row.target_r_recoverable_failure = recoverable_failure;
    row.target_rjf_wall_seconds = Some(wall_seconds);
    row.target_rjf_jvp_vectors = Some(target_jvp_vectors);
    if target_jvp_vectors > 0 {
        let denominator = target_jvp_vectors as f64;
        row.prefix_over_target_rjf_jvp = row
            .prefix_work
            .map(|work| work.jvp_vectors as f64 / denominator);
        row.continuation_over_target_rjf_jvp = row
            .continuation_work
            .map(|work| work.jvp_vectors as f64 / denominator);
        row.full_e_over_target_rjf_jvp = row
            .shadow_full_e_work
            .map(|work| work.jvp_vectors as f64 / denominator);
    }
}

fn frozen_shadow_expensive_work_zero(work: WorkCounters) -> bool {
    work.jacobian_builds == 0
        && work.direct_factorizations == 0
        && work.nonlinear_solves == 0
        && work.nonlinear_iterations == 0
        && work.nonlinear_residual_evaluations == 0
        && work.nonlinear_jacobian_evaluations == 0
}

#[allow(clippy::too_many_arguments)]
fn new_frozen_shadow_row(
    problem: &AtlasProblem,
    adaptive: &AdaptiveStepConfig,
    decision_step: usize,
    feature_value: Option<f64>,
    attempt_index: usize,
    accepted_steps_before: usize,
    t_start: f64,
    h_trial: f64,
    committed_rjf_jvp: u64,
    prefix_speculative_jvp: u64,
    total_speculative_jvp: u64,
    budget_cap_jvp: u64,
) -> G4S5B0FrozenFullEShadowRow {
    G4S5B0FrozenFullEShadowRow {
        trajectory_id: problem.id.clone(),
        family: problem.family.into(),
        dimension: problem.problem.dimension,
        rtol: adaptive.rtol,
        decision_accepted_step: decision_step,
        feature_value,
        target_attempt_index: attempt_index,
        target_accepted_steps_before: accepted_steps_before,
        t_start,
        h: h_trial,
        target_r_attempt_accepted: false,
        target_r_error_norm: None,
        target_r_recoverable_failure: false,
        committed_rjf_jvp_before_target: committed_rjf_jvp,
        prefix_speculative_jvp_before_target: prefix_speculative_jvp,
        prefix_speculative_jvp_after_target: prefix_speculative_jvp,
        total_speculative_jvp_before_target: total_speculative_jvp,
        total_speculative_jvp_after_target: total_speculative_jvp,
        budget_reserve_jvp: V29_PREFIX_RESERVE_JVP,
        budget_cap_jvp,
        budget_fraction: V29_PREFIX_BUDGET_FRACTION,
        budget_admitted: budget_cap_jvp > 0,
        budget_exhausted: false,
        budget_breached: false,
        prefix_succeeded: false,
        prefix_failure: None,
        actual_prefix_jvp_vectors: None,
        prefix_work: None,
        normalized_stage_growth_a34: None,
        rho2: None,
        rho3: None,
        rho4: None,
        stage_log_slope_s23: None,
        stage_log_slope_s34: None,
        stage_log_curvature_kappa234: None,
        remainder_chi23: None,
        remainder_chi34: None,
        remainder_chi24: None,
        remainder_q34_perp: None,
        remainder_delta_chi: None,
        quadratic_drift_zeta23: None,
        quadratic_drift_zeta34: None,
        quadratic_drift_relative: None,
        frozen_zeta34_tau: V36_FROZEN_ZETA34_TAU,
        recommended: false,
        retained_level2_resumed: false,
        shadow_prefix_wall_seconds: 0.0,
        shadow_continuation_wall_seconds: None,
        shadow_total_wall_seconds: 0.0,
        shadow_full_e_completed: false,
        shadow_full_e_total_error: None,
        shadow_full_e_locally_admissible: false,
        shadow_full_e_failure: None,
        continuation_work: None,
        shadow_full_e_work: None,
        work_roundtrip_exact: false,
        target_rjf_wall_seconds: None,
        target_rjf_jvp_vectors: None,
        prefix_over_target_rjf_jvp: None,
        continuation_over_target_rjf_jvp: None,
        full_e_over_target_rjf_jvp: None,
    }
}

fn run_rjf_frozen_full_e_shadow_trajectory(
    problem: AtlasProblem,
    profile: G4S5B0Profile,
) -> (
    Vec<G4S5B0RjfAttemptRow>,
    Vec<G4S5B0StepRow>,
    Vec<G4S5B0FrozenFullEShadowRow>,
    G4S5B0TrajectorySummary,
) {
    let adaptive = adaptive_config(profile, problem.t_span.1 - problem.t_span.0);
    let linear = linear_config();
    let mut controller = AdaptiveControllerState::default();
    let mut policy_state = PrefixPolicyState::new(G4S5B0PrefixProbePolicy::K3Development)
        .expect("sealed k=3 policy is valid");
    let mut pending_probe: Option<(usize, Option<f64>)> = None;
    let mut t = problem.t_span.0;
    let tf = problem.t_span.1;
    let mut y = problem.y0.clone();
    let mut h = adaptive.initial_step.min(tf - t);
    let tolerance = 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let mut attempts = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut attempt_rows = Vec::new();
    let mut accepted_rows = Vec::new();
    let mut shadow_rows = Vec::new();
    let mut total_jacobian_builds = 0u64;
    let mut total_factorizations = 0u64;
    let mut committed_rjf_jvp = 0u64;
    let mut prefix_speculative_jvp = 0u64;
    let mut total_speculative_jvp = 0u64;
    let mut failure = None;

    while t < tf - tolerance && attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            failure = Some("minimum step reached".into());
            break;
        }
        let attempt_index = attempts;
        attempts += 1;
        let accepted_steps_before = accepted;
        let t_start = t;
        let h_trial = h;

        let mut shadow_row = pending_probe.take().map(|(decision_step, feature_value)| {
            let budget_cap_jvp = enforced_prefix_jvp_cap(committed_rjf_jvp, prefix_speculative_jvp);
            let mut row = new_frozen_shadow_row(
                &problem,
                &adaptive,
                decision_step,
                feature_value,
                attempt_index,
                accepted_steps_before,
                t_start,
                h_trial,
                committed_rjf_jvp,
                prefix_speculative_jvp,
                total_speculative_jvp,
                budget_cap_jvp,
            );

            if budget_cap_jvp > 0 {
                let prefix_start = Instant::now();
                let prefix_outcome =
                    (|| -> CoreResult<Pexprb54s4AccountedBudgetedLevel2PrefixOutcome> {
                    let (shadow_problem, state) = shadow_problem_state(&problem.problem, t, &y)?;
                    let config = phi_config(adaptive.rtol, shadow_problem.dimension + 4);
                    pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget_accounted(
                        &shadow_problem,
                        t,
                        &state,
                        h_trial,
                        config,
                        problem.problem.dimension,
                        adaptive.atol,
                        adaptive.rtol,
                        budget_cap_jvp,
                    )
                })();
                row.shadow_prefix_wall_seconds = prefix_start.elapsed().as_secs_f64();

                match prefix_outcome {
                    Ok(Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::Complete(level2)) => {
                        let prefix_work = level2.report().cumulative_work;
                        row.prefix_succeeded = true;
                        row.actual_prefix_jvp_vectors = Some(prefix_work.jvp_vectors);
                        row.prefix_work = Some(prefix_work);
                        fill_frozen_shadow_prefix_evidence(&mut row, level2.report());
                        row.budget_breached = prefix_work.jvp_vectors > budget_cap_jvp;
                        prefix_speculative_jvp =
                            prefix_speculative_jvp.saturating_add(prefix_work.jvp_vectors);
                        total_speculative_jvp =
                            total_speculative_jvp.saturating_add(prefix_work.jvp_vectors);
                        row.recommended = frozen_full_e_shadow_recommended(
                            true,
                            false,
                            row.budget_breached,
                            row.quadratic_drift_zeta34,
                        );

                        if row.recommended {
                            row.retained_level2_resumed = true;
                            let continuation_start = Instant::now();
                            let continuation_outcome =
                                pexprb54s4_fused_step_resume_level2_accounted(
                                    *level2,
                                    &ParallelExecution::sequential(),
                                );
                            let continuation_wall = continuation_start.elapsed().as_secs_f64();
                            row.shadow_continuation_wall_seconds = Some(continuation_wall);
                            match continuation_outcome {
                                Ok(Pexprb54s4Level2ContinuationOutcome::Complete {
                                    report,
                                    ledger,
                                }) => {
                                    row.continuation_work = Some(ledger.continuation_work);
                                    row.shadow_full_e_work = Some(ledger.cumulative_work);
                                    row.work_roundtrip_exact = ledger.prefix_work == prefix_work
                                        && report.work == ledger.cumulative_work
                                        && exact_continuation_roundtrip(
                                            ledger.prefix_work,
                                            ledger.continuation_work,
                                            ledger.cumulative_work,
                                        );
                                    total_speculative_jvp = total_speculative_jvp
                                        .saturating_add(ledger.continuation_work.jvp_vectors);
                                    if row.work_roundtrip_exact {
                                        match frozen_shadow_total_error(
                                            &report,
                                            &y,
                                            problem.problem.dimension,
                                            &adaptive,
                                            h_trial,
                                        ) {
                                            Ok(total_error) => {
                                                row.shadow_full_e_completed = true;
                                                row.shadow_full_e_total_error = Some(total_error);
                                                row.shadow_full_e_locally_admissible =
                                                    total_error.is_finite() && total_error <= 1.0;
                                            }
                                            Err(error) => {
                                                row.shadow_full_e_failure = Some(error.to_string());
                                            }
                                        }
                                    } else {
                                        row.shadow_full_e_failure = Some(
                                            "retained level-2 continuation work ledger mismatch"
                                                .into(),
                                        );
                                    }
                                }
                                Ok(Pexprb54s4Level2ContinuationOutcome::Failed {
                                    error,
                                    ledger,
                                }) => {
                                    row.continuation_work = Some(ledger.continuation_work);
                                    row.shadow_full_e_work = Some(ledger.cumulative_work);
                                    row.work_roundtrip_exact = ledger.prefix_work == prefix_work
                                        && exact_continuation_roundtrip(
                                            ledger.prefix_work,
                                            ledger.continuation_work,
                                            ledger.cumulative_work,
                                        );
                                    total_speculative_jvp = total_speculative_jvp
                                        .saturating_add(ledger.continuation_work.jvp_vectors);
                                    row.shadow_full_e_failure = Some(error.to_string());
                                }
                                Err(error) => {
                                    row.shadow_full_e_failure = Some(error.to_string());
                                }
                            }
                        }
                    }
                    Ok(Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::BudgetExhausted(report)) => {
                        row.budget_exhausted = true;
                        row.prefix_failure = Some("budget-exhausted".into());
                        row.actual_prefix_jvp_vectors = Some(report.used_jvp_vectors);
                        row.prefix_work = Some(report.work);
                        row.budget_breached = report.used_jvp_vectors > budget_cap_jvp;
                        prefix_speculative_jvp =
                            prefix_speculative_jvp.saturating_add(report.used_jvp_vectors);
                        total_speculative_jvp =
                            total_speculative_jvp.saturating_add(report.used_jvp_vectors);
                    }
                    Ok(Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::Failed(report)) => {
                        row.prefix_failure = Some(report.error.to_string());
                        row.actual_prefix_jvp_vectors = Some(report.work.jvp_vectors);
                        row.prefix_work = Some(report.work);
                        row.budget_breached = report.work.jvp_vectors > budget_cap_jvp;
                        prefix_speculative_jvp =
                            prefix_speculative_jvp.saturating_add(report.work.jvp_vectors);
                        total_speculative_jvp =
                            total_speculative_jvp.saturating_add(report.work.jvp_vectors);
                    }
                    Err(error) => {
                        row.prefix_failure = Some(error.to_string());
                    }
                }
            }
            row.prefix_speculative_jvp_after_target = prefix_speculative_jvp;
            row.total_speculative_jvp_after_target = total_speculative_jvp;
            row.shadow_total_wall_seconds = row.shadow_prefix_wall_seconds
                + row.shadow_continuation_wall_seconds.unwrap_or(0.0);
            row
        });

        let mut step_counters = WorkCounters::default();
        let start = Instant::now();
        let trial = sequential_matrix_free_step(
            &problem.problem,
            t,
            &y,
            h,
            &linear,
            None,
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut step_counters,
        );
        let wall = start.elapsed().as_secs_f64();
        committed_rjf_jvp = committed_rjf_jvp.saturating_add(step_counters.jvp_vectors);
        total_jacobian_builds += step_counters.jacobian_builds;
        total_factorizations += step_counters.direct_factorizations;

        let report = match trial {
            Ok(report) => report,
            Err(error @ (CoreError::NonFinite(_) | CoreError::LinearSolve(_))) => {
                attempt_rows.push(G4S5B0RjfAttemptRow {
                    trajectory_id: problem.id.clone(),
                    family: problem.family.into(),
                    dimension: problem.problem.dimension,
                    rtol: adaptive.rtol,
                    attempt_index,
                    accepted_steps_before,
                    t_start,
                    h: h_trial,
                    error_norm: None,
                    accepted: false,
                    recoverable_failure: true,
                    failure: Some(error.to_string()),
                    wall_seconds: wall,
                    rhs_evaluations: step_counters.rhs_evaluations,
                    jvp_vectors: step_counters.jvp_vectors,
                    linear_matvecs: step_counters.linear_matvecs,
                });
                if let Some(mut row) = shadow_row.take() {
                    finalize_frozen_shadow_target(
                        &mut row,
                        false,
                        None,
                        true,
                        wall,
                        step_counters.jvp_vectors,
                    );
                    shadow_rows.push(row);
                }
                rejected += 1;
                h *= adaptive.min_factor;
                continue;
            }
            Err(error) => {
                attempt_rows.push(G4S5B0RjfAttemptRow {
                    trajectory_id: problem.id.clone(),
                    family: problem.family.into(),
                    dimension: problem.problem.dimension,
                    rtol: adaptive.rtol,
                    attempt_index,
                    accepted_steps_before,
                    t_start,
                    h: h_trial,
                    error_norm: None,
                    accepted: false,
                    recoverable_failure: false,
                    failure: Some(error.to_string()),
                    wall_seconds: wall,
                    rhs_evaluations: step_counters.rhs_evaluations,
                    jvp_vectors: step_counters.jvp_vectors,
                    linear_matvecs: step_counters.linear_matvecs,
                });
                if let Some(mut row) = shadow_row.take() {
                    finalize_frozen_shadow_target(
                        &mut row,
                        false,
                        None,
                        false,
                        wall,
                        step_counters.jvp_vectors,
                    );
                    shadow_rows.push(row);
                }
                failure = Some(error.to_string());
                break;
            }
        };
        let error = report.error_norm;
        let step_accepted = report.accepted
            && error.is_finite()
            && error <= 1.0
            && report.y_new.iter().all(|value| value.is_finite());
        attempt_rows.push(G4S5B0RjfAttemptRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            attempt_index,
            accepted_steps_before,
            t_start,
            h: h_trial,
            error_norm: Some(error),
            accepted: step_accepted,
            recoverable_failure: false,
            failure: None,
            wall_seconds: wall,
            rhs_evaluations: step_counters.rhs_evaluations,
            jvp_vectors: step_counters.jvp_vectors,
            linear_matvecs: step_counters.linear_matvecs,
        });
        if let Some(mut row) = shadow_row.take() {
            finalize_frozen_shadow_target(
                &mut row,
                step_accepted,
                Some(error),
                false,
                wall,
                step_counters.jvp_vectors,
            );
            shadow_rows.push(row);
        }
        if !step_accepted {
            rejected += 1;
            let _ = controller.record_rejection(error.max(1.0e-16));
            h *= controller
                .propose_factor(&adaptive, error.max(1.0e-16), 5, false)
                .unwrap_or(adaptive.min_factor);
            continue;
        }

        accepted_rows.push(G4S5B0StepRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            step_index: accepted,
            t_start: t,
            h,
            transition_level: (problem.transition)(t, &y),
            rodas_embedded_error: error,
            rodas_wall_seconds: wall,
            rodas_rhs_evaluations: step_counters.rhs_evaluations,
            rodas_jvp_vectors: step_counters.jvp_vectors,
            rodas_linear_matvecs: step_counters.linear_matvecs,
            exponential_completed: false,
            exponential_total_error: None,
            exponential_locally_admissible: false,
            exponential_wall_seconds: None,
            exponential_prefix_wall_seconds: None,
            exponential_rhs_evaluations: None,
            exponential_jvp_vectors: None,
            exponential_maximum_krylov_dimension: None,
            exponential_phi_substeps: None,
            exponential_failure: None,
        });
        let decision_step = accepted;
        let event_feature = policy_state.observe_accepted(
            problem.family,
            problem.problem.dimension,
            decision_step,
            error,
        );
        t = report.t_new;
        y = report.y_new;
        accepted += 1;
        let _ = controller.record_acceptance(error);
        h *= controller
            .propose_factor(&adaptive, error.max(1.0e-16), 5, true)
            .unwrap_or(1.0);
        if let Some(feature_value) = event_feature {
            pending_probe = Some((decision_step, feature_value));
        }
    }

    let success = t >= tf - tolerance;
    if !success && failure.is_none() {
        failure = Some("maximum attempts reached".into());
    }
    let summary = G4S5B0TrajectorySummary {
        trajectory_id: problem.id,
        family: problem.family.into(),
        dimension: problem.problem.dimension,
        rtol: adaptive.rtol,
        success,
        failure,
        attempts,
        accepted_steps: accepted,
        rejected_steps: rejected,
        endpoint_time: t,
        explicit_jacobian_builds: total_jacobian_builds,
        direct_factorizations: total_factorizations,
        newton_iterations: 0,
    };
    (attempt_rows, accepted_rows, shadow_rows, summary)
}

struct FrozenFullEShadowExecution {
    attempt_rows: Vec<G4S5B0RjfAttemptRow>,
    accepted_rows: Vec<G4S5B0StepRow>,
    rows: Vec<G4S5B0FrozenFullEShadowRow>,
    trajectories: Vec<G4S5B0TrajectorySummary>,
}

fn execute_frozen_full_e_shadow_filtered(
    profile: G4S5B0Profile,
    family: Option<G4S5B0Family>,
) -> CoreResult<FrozenFullEShadowExecution> {
    if !v36_profile_is_consumed(profile) {
        return Err(CoreError::InvalidInput(
            "v3.6 full-E shadow is restricted to the consumed N=96/192/256/320/384 profiles".into(),
        ));
    }
    let mut execution = FrozenFullEShadowExecution {
        attempt_rows: Vec::new(),
        accepted_rows: Vec::new(),
        rows: Vec::new(),
        trajectories: Vec::new(),
    };
    for problem in build_problems(profile)? {
        if family.is_some_and(|selected| problem.family != selected.as_str()) {
            continue;
        }
        let (mut attempts, mut accepted, mut rows, summary) =
            run_rjf_frozen_full_e_shadow_trajectory(problem, profile);
        execution.attempt_rows.append(&mut attempts);
        execution.accepted_rows.append(&mut accepted);
        execution.rows.append(&mut rows);
        execution.trajectories.push(summary);
    }
    Ok(execution)
}

fn run_g4_s5b0_frozen_full_e_shadow_filtered(
    profile: G4S5B0Profile,
    family: Option<G4S5B0Family>,
) -> CoreResult<G4S5B0FrozenFullEShadowReport> {
    let FrozenFullEShadowExecution {
        attempt_rows,
        accepted_rows,
        rows,
        trajectories,
    } = execute_frozen_full_e_shadow_filtered(profile, family)?;
    let reference = run_g4_s5b0_rjf_attempt_trace_filtered(profile, family)?;
    let rjf_parity = rjf_parity(&attempt_rows, &accepted_rows, &trajectories, &reference);

    let recommendations = rows.iter().filter(|row| row.recommended).count();
    let retained_level2_resumptions = rows
        .iter()
        .filter(|row| row.retained_level2_resumed)
        .count();
    let shadow_full_e_completions = rows
        .iter()
        .filter(|row| row.shadow_full_e_completed)
        .count();
    let shadow_full_e_failures = rows
        .iter()
        .filter(|row| row.recommended && !row.shadow_full_e_completed)
        .count();
    let unsafe_recommendations = rows
        .iter()
        .filter(|row| {
            row.recommended && row.shadow_full_e_completed && !row.shadow_full_e_locally_admissible
        })
        .count();
    let budget_breaches = rows.iter().filter(|row| row.budget_breached).count();
    let budget_exhaustions = rows.iter().filter(|row| row.budget_exhausted).count();

    let mut prefix_speculative_work = WorkCounters::default();
    let mut continuation_work = WorkCounters::default();
    for row in &rows {
        if let Some(work) = row.prefix_work {
            prefix_speculative_work.accumulate(work);
        }
        if let Some(work) = row.continuation_work {
            continuation_work.accumulate(work);
        }
    }
    let mut total_speculative_work = prefix_speculative_work;
    total_speculative_work.accumulate(continuation_work);
    let committed_rjf_jvp_vectors = attempt_rows.iter().map(|row| row.jvp_vectors).sum::<u64>();
    let committed_rjf_denominator = committed_rjf_jvp_vectors as f64;
    let realized_prefix_over_committed_rjf_jvp =
        prefix_speculative_work.jvp_vectors as f64 / committed_rjf_denominator;
    let realized_continuation_over_committed_rjf_jvp =
        continuation_work.jvp_vectors as f64 / committed_rjf_denominator;
    let realized_total_speculative_over_committed_rjf_jvp =
        total_speculative_work.jvp_vectors as f64 / committed_rjf_denominator;
    let realized_work_ratios_finite = committed_rjf_jvp_vectors > 0
        && realized_prefix_over_committed_rjf_jvp.is_finite()
        && realized_continuation_over_committed_rjf_jvp.is_finite()
        && realized_total_speculative_over_committed_rjf_jvp.is_finite();

    let prefix_transactions_resolved = rows
        .iter()
        .all(|row| !row.budget_admitted || row.prefix_succeeded || row.budget_exhausted);
    let row_ledgers_exact = rows.iter().all(|row| {
        let prefix_jvp = row.prefix_work.map_or(0, |work| work.jvp_vectors);
        let continuation_jvp = row.continuation_work.map_or(0, |work| work.jvp_vectors);
        let prefix_state_exact = row.prefix_speculative_jvp_after_target
            == row
                .prefix_speculative_jvp_before_target
                .saturating_add(prefix_jvp);
        let total_state_exact = row.total_speculative_jvp_after_target
            == row
                .total_speculative_jvp_before_target
                .saturating_add(prefix_jvp)
                .saturating_add(continuation_jvp);
        let cap_exact = row.budget_cap_jvp
            == enforced_prefix_jvp_cap(
                row.committed_rjf_jvp_before_target,
                row.prefix_speculative_jvp_before_target,
            );
        let endpoint_ledger_exact = if row.recommended {
            row.work_roundtrip_exact
                && row.continuation_work.is_some()
                && row.shadow_full_e_work.is_some()
        } else {
            !row.retained_level2_resumed
                && row.continuation_work.is_none()
                && row.shadow_full_e_work.is_none()
        };
        prefix_state_exact && total_state_exact && cap_exact && endpoint_ledger_exact
    });
    let resume_cardinality_exact = recommendations == retained_level2_resumptions
        && rows
            .iter()
            .all(|row| row.recommended == row.retained_level2_resumed);
    let shadow_implicit_expensive_work_zero = rows.iter().all(|row| {
        row.prefix_work
            .is_none_or(frozen_shadow_expensive_work_zero)
            && row
                .continuation_work
                .is_none_or(frozen_shadow_expensive_work_zero)
            && row
                .shadow_full_e_work
                .is_none_or(frozen_shadow_expensive_work_zero)
    });
    let all_rjf_trajectories_successful = trajectories.iter().all(|row| row.success);
    let mut hard_gates = G4S5B0FrozenFullEShadowHardGates {
        all_rjf_trajectories_successful,
        rjf_trace_exact_excluding_wall: rjf_parity.passed,
        zero_budget_breaches: budget_breaches == 0,
        prefix_transactions_resolved,
        zero_continuation_failures: shadow_full_e_failures == 0,
        zero_unsafe_recommendations: unsafe_recommendations == 0,
        work_ledgers_exact: row_ledgers_exact,
        realized_work_ratios_finite,
        resume_cardinality_exact,
        shadow_implicit_expensive_work_zero,
        active_switching_false: true,
        passed: false,
    };
    hard_gates.passed = hard_gates.all_rjf_trajectories_successful
        && hard_gates.rjf_trace_exact_excluding_wall
        && hard_gates.zero_budget_breaches
        && hard_gates.prefix_transactions_resolved
        && hard_gates.zero_continuation_failures
        && hard_gates.zero_unsafe_recommendations
        && hard_gates.work_ledgers_exact
        && hard_gates.realized_work_ratios_finite
        && hard_gates.resume_cardinality_exact
        && hard_gates.shadow_implicit_expensive_work_zero
        && hard_gates.active_switching_false;
    let status = if hard_gates.passed {
        "complete"
    } else {
        "complete-with-failures"
    };

    Ok(G4S5B0FrozenFullEShadowReport {
        schema: "g4-s5b0-frozen-full-e-shadow-v1",
        status,
        profile: profile.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        shadow_method: "pexprb54s4-fused-resume-retained-level2",
        persistence_k: 3,
        absolute_prefix_jvp_cap: V29_PREFIX_RESERVE_JVP,
        frozen_cumulative_prefix_budget_fraction: V29_PREFIX_BUDGET_FRACTION,
        frozen_zeta34_tau: V36_FROZEN_ZETA34_TAU,
        recommendations,
        retained_level2_resumptions,
        shadow_full_e_completions,
        shadow_full_e_failures,
        unsafe_recommendations,
        budget_breaches,
        budget_exhaustions,
        prefix_speculative_work,
        continuation_work,
        total_speculative_work,
        committed_rjf_jvp_vectors,
        realized_prefix_over_committed_rjf_jvp,
        realized_continuation_over_committed_rjf_jvp,
        realized_total_speculative_over_committed_rjf_jvp,
        rjf_parity,
        hard_gates,
        attempt_rows,
        accepted_rows,
        rows,
        trajectories,
        limitations: vec![
            "The committed protected R-JF trajectory and controller remain authoritative; the retained full-E result is read-only shadow evidence.".into(),
            "The frozen k=3, B_abs=80, delta=0.25, and zeta34 threshold are consumed without retuning.".into(),
            "Continuation work is charged to the total speculative ledger but never feeds the prefix-only budget ledger or later caps.".into(),
            "R-JF parity excludes only attempt and accepted-step wall-clock fields; no stronger state/output parity claim is made without explicit digests.".into(),
            "These five profiles are consumed economics evidence, not a fresh safety holdout; active switching and N=2048 remain sealed.".into(),
        ],
    })
}

pub fn run_g4_s5b0_frozen_full_e_shadow_family(
    profile: G4S5B0Profile,
    family: G4S5B0Family,
) -> CoreResult<G4S5B0FrozenFullEShadowReport> {
    run_g4_s5b0_frozen_full_e_shadow_filtered(profile, Some(family))
}

pub fn run_g4_s5b0_frozen_full_e_shadow(
    profile: G4S5B0Profile,
) -> CoreResult<G4S5B0FrozenFullEShadowReport> {
    run_g4_s5b0_frozen_full_e_shadow_filtered(profile, None)
}

#[derive(Clone, Copy)]
struct ShadowWallProtocol {
    warmup_pairs: usize,
    measured_pairs: usize,
    minimum_wall_seconds: f64,
    maximum_repetitions: usize,
}

fn production_shadow_wall_protocol() -> ShadowWallProtocol {
    ShadowWallProtocol {
        warmup_pairs: 1,
        measured_pairs: 7,
        minimum_wall_seconds: 0.25,
        maximum_repetitions: 1024,
    }
}

fn measurement_build_verified() -> bool {
    env!("VIGILODE_CARGO_PROFILE_DIR") == "measurement"
}

fn next_calibration_repetitions(current: usize, maximum: usize) -> usize {
    current.saturating_mul(2).min(maximum)
}

fn pair_runs_rjf_first(pair_index: usize) -> bool {
    pair_index.is_multiple_of(2)
}

fn sorted_attempt_interval_sum(attempts: &[G4S5B0RjfAttemptRow]) -> f64 {
    let mut intervals: Vec<f64> = attempts.iter().map(|row| row.h.abs()).collect();
    intervals.sort_by(f64::total_cmp);
    intervals.into_iter().sum()
}

fn family_count(trajectories: &[G4S5B0TrajectorySummary]) -> usize {
    trajectories
        .iter()
        .map(|row| row.family.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn frozen_family_set_is_exact(trajectories: &[G4S5B0TrajectorySummary]) -> bool {
    let observed = trajectories
        .iter()
        .map(|row| row.family.as_str())
        .collect::<BTreeSet<_>>();
    let expected = G4S5B0Family::ALL
        .iter()
        .map(|family| family.as_str())
        .collect::<BTreeSet<_>>();
    observed == expected
}

fn zero_optional_wall(value: &mut Option<f64>) {
    if value.is_some() {
        *value = Some(0.0);
    }
}

fn frozen_shadow_row_exact_excluding_wall(
    left: &G4S5B0FrozenFullEShadowRow,
    right: &G4S5B0FrozenFullEShadowRow,
) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.shadow_prefix_wall_seconds = 0.0;
    right.shadow_prefix_wall_seconds = 0.0;
    zero_optional_wall(&mut left.shadow_continuation_wall_seconds);
    zero_optional_wall(&mut right.shadow_continuation_wall_seconds);
    left.shadow_total_wall_seconds = 0.0;
    right.shadow_total_wall_seconds = 0.0;
    zero_optional_wall(&mut left.target_rjf_wall_seconds);
    zero_optional_wall(&mut right.target_rjf_wall_seconds);
    left == right
}

fn frozen_shadow_execution_exact(
    execution: &FrozenFullEShadowExecution,
    shadow_reference: &G4S5B0FrozenFullEShadowReport,
    rjf_reference: &G4S5B0AttemptTraceReport,
) -> bool {
    rjf_parity(
        &execution.attempt_rows,
        &execution.accepted_rows,
        &execution.trajectories,
        rjf_reference,
    )
    .passed
        && execution.rows.len() == shadow_reference.rows.len()
        && execution
            .rows
            .iter()
            .zip(&shadow_reference.rows)
            .all(|(left, right)| frozen_shadow_row_exact_excluding_wall(left, right))
}

#[derive(Clone, Copy)]
enum ShadowWallMode {
    RjfOnly,
    FrozenFullE,
}

impl ShadowWallMode {
    fn label(self) -> &'static str {
        match self {
            Self::RjfOnly => "rjf-only",
            Self::FrozenFullE => "frozen-full-e-shadow",
        }
    }
}

fn timed_shadow_wall_arm(
    profile: G4S5B0Profile,
    mode: ShadowWallMode,
    repetitions: usize,
    rjf_reference: &G4S5B0AttemptTraceReport,
    shadow_reference: &G4S5B0FrozenFullEShadowReport,
) -> CoreResult<G4S5B0ShadowWallArm> {
    let mut wall_seconds = 0.0;
    let mut proposed_interval = 0.0;
    let mut all_suite_identities_passed = true;
    let mut observed_family_count = 0usize;
    for _ in 0..repetitions {
        match mode {
            ShadowWallMode::RjfOnly => {
                let start = Instant::now();
                let suite = execute_rjf_attempt_trace_filtered(profile, None)?;
                wall_seconds += start.elapsed().as_secs_f64();
                proposed_interval += sorted_attempt_interval_sum(&suite.attempt_rows);
                observed_family_count = family_count(&suite.trajectories);
                all_suite_identities_passed &= rjf_parity(
                    &suite.attempt_rows,
                    &suite.accepted_rows,
                    &suite.trajectories,
                    rjf_reference,
                )
                .passed;
                all_suite_identities_passed &= frozen_family_set_is_exact(&suite.trajectories);
            }
            ShadowWallMode::FrozenFullE => {
                let start = Instant::now();
                let suite = execute_frozen_full_e_shadow_filtered(profile, None)?;
                wall_seconds += start.elapsed().as_secs_f64();
                proposed_interval += sorted_attempt_interval_sum(&suite.attempt_rows);
                observed_family_count = family_count(&suite.trajectories);
                all_suite_identities_passed &=
                    frozen_shadow_execution_exact(&suite, shadow_reference, rjf_reference);
                all_suite_identities_passed &= frozen_family_set_is_exact(&suite.trajectories);
            }
        }
        all_suite_identities_passed &= observed_family_count == G4S5B0Family::ALL.len();
    }
    let gamma_seconds_per_interval = if proposed_interval > 0.0 {
        wall_seconds / proposed_interval
    } else {
        f64::INFINITY
    };
    Ok(G4S5B0ShadowWallArm {
        mode: mode.label().into(),
        repetitions,
        wall_seconds,
        proposed_interval,
        gamma_seconds_per_interval,
        family_count: observed_family_count,
        all_suite_identities_passed,
    })
}

fn shadow_wall_pair(
    profile: G4S5B0Profile,
    repetitions: usize,
    pair_index: usize,
    rjf_reference: &G4S5B0AttemptTraceReport,
    shadow_reference: &G4S5B0FrozenFullEShadowReport,
) -> CoreResult<G4S5B0ShadowWallPair> {
    let rjf_first = pair_runs_rjf_first(pair_index);
    let (rjf_only, frozen_full_e_shadow, order) = if rjf_first {
        let rjf_only = timed_shadow_wall_arm(
            profile,
            ShadowWallMode::RjfOnly,
            repetitions,
            rjf_reference,
            shadow_reference,
        )?;
        let frozen_full_e_shadow = timed_shadow_wall_arm(
            profile,
            ShadowWallMode::FrozenFullE,
            repetitions,
            rjf_reference,
            shadow_reference,
        )?;
        (rjf_only, frozen_full_e_shadow, "rjf-first")
    } else {
        let frozen_full_e_shadow = timed_shadow_wall_arm(
            profile,
            ShadowWallMode::FrozenFullE,
            repetitions,
            rjf_reference,
            shadow_reference,
        )?;
        let rjf_only = timed_shadow_wall_arm(
            profile,
            ShadowWallMode::RjfOnly,
            repetitions,
            rjf_reference,
            shadow_reference,
        )?;
        (rjf_only, frozen_full_e_shadow, "shadow-first")
    };
    if rjf_only.proposed_interval.to_bits() != frozen_full_e_shadow.proposed_interval.to_bits() {
        return Err(CoreError::InvalidInput(
            "paired v3.6 proposed intervals differ despite exact R-JF trace parity".into(),
        ));
    }
    Ok(G4S5B0ShadowWallPair {
        pair_index,
        order: order.into(),
        wall_ratio_shadow_over_rjf: frozen_full_e_shadow.wall_seconds / rjf_only.wall_seconds,
        gamma_ratio_shadow_over_rjf: frozen_full_e_shadow.gamma_seconds_per_interval
            / rjf_only.gamma_seconds_per_interval,
        rjf_only,
        frozen_full_e_shadow,
    })
}

fn run_shadow_wall_protocol(
    profile: G4S5B0Profile,
    protocol: ShadowWallProtocol,
    rjf_reference: &G4S5B0AttemptTraceReport,
    shadow_reference: &G4S5B0FrozenFullEShadowReport,
) -> CoreResult<G4S5B0ShadowWallReport> {
    let mut repetitions = 1usize;
    let mut calibration_rows = Vec::new();
    loop {
        let arm = timed_shadow_wall_arm(
            profile,
            ShadowWallMode::RjfOnly,
            repetitions,
            rjf_reference,
            shadow_reference,
        )?;
        calibration_rows.push(G4S5B0ShadowWallCalibrationRow {
            repetitions,
            wall_seconds: arm.wall_seconds,
            proposed_interval: arm.proposed_interval,
            gamma_seconds_per_interval: arm.gamma_seconds_per_interval,
            all_suite_identities_passed: arm.all_suite_identities_passed,
        });
        if arm.wall_seconds >= protocol.minimum_wall_seconds
            || repetitions >= protocol.maximum_repetitions
        {
            break;
        }
        repetitions = next_calibration_repetitions(repetitions, protocol.maximum_repetitions);
    }
    let mut warmup_rows = Vec::new();
    for pair_index in 0..protocol.warmup_pairs {
        warmup_rows.push(shadow_wall_pair(
            profile,
            repetitions,
            pair_index,
            rjf_reference,
            shadow_reference,
        )?);
    }
    let mut measured_rows = Vec::new();
    for pair_index in 0..protocol.measured_pairs {
        measured_rows.push(shadow_wall_pair(
            profile,
            repetitions,
            pair_index,
            rjf_reference,
            shadow_reference,
        )?);
    }
    let all_suite_identities_passed = calibration_rows
        .iter()
        .all(|row| row.all_suite_identities_passed)
        && warmup_rows.iter().all(|row| {
            row.rjf_only.all_suite_identities_passed
                && row.frozen_full_e_shadow.all_suite_identities_passed
        })
        && measured_rows.iter().all(|row| {
            row.rjf_only.all_suite_identities_passed
                && row.frozen_full_e_shadow.all_suite_identities_passed
        });
    Ok(G4S5B0ShadowWallReport {
        required_build_profile: "measurement",
        measurement_build_verified: measurement_build_verified(),
        compiled_cargo_profile: env!("VIGILODE_CARGO_PROFILE"),
        compiled_profile_directory: env!("VIGILODE_CARGO_PROFILE_DIR"),
        suite_scope: "all-six-families",
        calibration_arm: "rjf-only",
        gamma_denominator: "sum-absolute-proposed-attempt-h",
        warmup_pairs: protocol.warmup_pairs,
        measured_pairs: protocol.measured_pairs,
        frozen_repetitions: repetitions,
        minimum_calibration_wall_seconds: protocol.minimum_wall_seconds,
        maximum_calibration_repetitions: protocol.maximum_repetitions,
        calibration_rows,
        warmup_rows,
        measured_rows,
        all_suite_identities_passed,
    })
}

pub fn run_g4_s5b0_frozen_full_e_shadow_economics(
    profile: G4S5B0Profile,
) -> CoreResult<G4S5B0FrozenFullEShadowEconomicsReport> {
    if !measurement_build_verified() {
        return Err(CoreError::InvalidInput(
            "v3.6 paired wall authority must be compiled with the Cargo measurement profile; invoke cargo run --profile measurement"
                .into(),
        ));
    }
    let shadow_reference = run_g4_s5b0_frozen_full_e_shadow(profile)?;
    let rjf_reference = run_g4_s5b0_rjf_attempt_trace(profile)?;
    let all_six_families_present = frozen_family_set_is_exact(&shadow_reference.trajectories)
        && frozen_family_set_is_exact(&rjf_reference.trajectories);
    let paired_wall = run_shadow_wall_protocol(
        profile,
        production_shadow_wall_protocol(),
        &rjf_reference,
        &shadow_reference,
    )?;
    let status = if shadow_reference.hard_gates.passed
        && all_six_families_present
        && paired_wall.all_suite_identities_passed
        && paired_wall.measurement_build_verified
    {
        "complete"
    } else {
        "complete-with-failures"
    };
    Ok(G4S5B0FrozenFullEShadowEconomicsReport {
        schema: "g4-s5b0-frozen-full-e-shadow-economics-v1",
        status,
        profile: profile.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        shadow_method: "pexprb54s4-fused-resume-retained-level2",
        frozen_zeta34_tau: V36_FROZEN_ZETA34_TAU,
        all_six_families_present,
        reference_recommendations: shadow_reference.recommendations,
        reference_shadow_completions: shadow_reference.shadow_full_e_completions,
        reference_unsafe_recommendations: shadow_reference.unsafe_recommendations,
        reference_hard_gates: shadow_reference.hard_gates,
        paired_wall,
        limitations: vec![
            "Wall economics are descriptive: v3.6 freezes no speedup threshold and authorizes no active switching.".into(),
            "Authority results require cargo --profile measurement; runtime profile detection is intentionally not inferred from debug assertions.".into(),
            "All paired repetitions are retained in alternating order; no favorable pair is selected.".into(),
            "The N=96/192/256/320/384 profiles are consumed evidence and N=2048 remains sealed.".into(),
        ],
    })
}

#[cfg(test)]
mod stage_trajectory_geometry_tests {
    use super::{
        G4S5B0Profile, ShadowWallProtocol, frozen_shadow_row_exact_excluding_wall,
        next_calibration_repetitions, pair_runs_rjf_first, production_shadow_wall_protocol,
        run_g4_s5b0_frozen_full_e_shadow, run_g4_s5b0_rjf_attempt_trace, run_shadow_wall_protocol,
        stage_trajectory_shape_features,
    };

    #[test]
    fn stage_trajectory_shape_matches_power_law_and_rejects_nonpositive_inputs() {
        let (s23, s34, kappa) = stage_trajectory_shape_features(1.0, 4.0, 12.96)
            .expect("positive finite power-law samples should be scoreable");
        assert!((s23 - 2.0).abs() < 1.0e-12);
        assert!((s34 - 2.0).abs() < 1.0e-12);
        assert!(kappa.abs() < 1.0e-12);

        assert!(stage_trajectory_shape_features(0.0, 1.0, 2.0).is_none());
        assert!(stage_trajectory_shape_features(1.0, f64::NAN, 2.0).is_none());
    }

    #[test]
    fn production_shadow_wall_protocol_is_frozen_and_alternating() {
        let protocol = production_shadow_wall_protocol();
        assert_eq!(protocol.warmup_pairs, 1);
        assert_eq!(protocol.measured_pairs, 7);
        assert_eq!(protocol.minimum_wall_seconds.to_bits(), 0.25_f64.to_bits());
        assert_eq!(protocol.maximum_repetitions, 1024);
        assert_eq!(
            (0..protocol.measured_pairs)
                .map(pair_runs_rjf_first)
                .collect::<Vec<_>>(),
            vec![true, false, true, false, true, false, true],
        );
        assert_eq!(next_calibration_repetitions(1, 1024), 2);
        assert_eq!(next_calibration_repetitions(512, 1024), 1024);
        assert_eq!(next_calibration_repetitions(1024, 1024), 1024);
    }

    #[test]
    #[ignore = "optimized all-six wall-contract check; run explicitly with --profile measurement"]
    fn one_pair_wall_protocol_is_self_describing_and_identity_checked() {
        let profile = G4S5B0Profile::StageGrowthCalibration96;
        let shadow_reference = run_g4_s5b0_frozen_full_e_shadow(profile).unwrap();
        let rjf_reference = run_g4_s5b0_rjf_attempt_trace(profile).unwrap();
        let report = run_shadow_wall_protocol(
            profile,
            ShadowWallProtocol {
                warmup_pairs: 0,
                measured_pairs: 1,
                minimum_wall_seconds: 0.0,
                maximum_repetitions: 1,
            },
            &rjf_reference,
            &shadow_reference,
        )
        .unwrap();

        assert_eq!(report.calibration_arm, "rjf-only");
        assert_eq!(report.gamma_denominator, "sum-absolute-proposed-attempt-h");
        assert_eq!(report.frozen_repetitions, 1);
        assert_eq!(report.measured_rows.len(), 1);
        assert!(report.all_suite_identities_passed);
        let pair = &report.measured_rows[0];
        assert_eq!(pair.order, "rjf-first");
        assert_eq!(pair.rjf_only.mode, "rjf-only");
        assert_eq!(pair.frozen_full_e_shadow.mode, "frozen-full-e-shadow");
        assert_eq!(pair.rjf_only.family_count, 6);
        assert_eq!(pair.frozen_full_e_shadow.family_count, 6);
        assert_eq!(
            pair.rjf_only.proposed_interval.to_bits(),
            pair.frozen_full_e_shadow.proposed_interval.to_bits()
        );
        assert_eq!(
            pair.rjf_only.gamma_seconds_per_interval.to_bits(),
            (pair.rjf_only.wall_seconds / pair.rjf_only.proposed_interval).to_bits()
        );

        let mut wall_only_mutation = shadow_reference.rows[0].clone();
        wall_only_mutation.shadow_prefix_wall_seconds += 1.0;
        wall_only_mutation.shadow_total_wall_seconds += 1.0;
        if let Some(value) = &mut wall_only_mutation.shadow_continuation_wall_seconds {
            *value += 1.0;
        }
        assert!(frozen_shadow_row_exact_excluding_wall(
            &shadow_reference.rows[0],
            &wall_only_mutation
        ));
        wall_only_mutation.h = f64::from_bits(wall_only_mutation.h.to_bits() + 1);
        assert!(!frozen_shadow_row_exact_excluding_wall(
            &shadow_reference.rows[0],
            &wall_only_mutation
        ));
    }
}
