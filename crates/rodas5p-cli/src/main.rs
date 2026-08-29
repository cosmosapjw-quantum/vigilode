use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rodas5p_core::{load_rodas5p_coefficients, sha256_hex};
use rodas5p_fair_ab::{
    BenchmarkCell, BenchmarkPlan, FairSolveConfig, GlobalErrorParetoProfile, PreconditionerKind,
    RecycleLifetime, ScientificValidityV2CaseArtifact, SequenceConfig, SequenceKind, SolverKind,
    TraceDocument, freeze_scientific_validity_v2_calibration_artifacts, generate_trace,
    load_numerical_reference_v2, replay_scientific_validity_v2_oregonator_artifacts,
    run_adaptive_global_error_screen, run_comparison, run_g1_adaptive_global_error_screen,
    run_global_error_pareto_screen, run_scientific_validity_v2_case,
    scientific_validity_v2_canonical_campaign_binding, scientific_validity_v2_compiled_revision,
    summarize_comparison, validate_scientific_validity_v2_case_artifact,
};
use rodas5p_integrators::{
    A1ScientificExecutionIdentity, CandidateCatalog, CandidateFamily, CandidateStatus,
    G1TransactionalGateProfile, G2ExponentialGateProfile, G3FusedAdaptiveProfile,
    G4PrefixKernelProfile, G4S5B0Family, G4S5B0PrefixProbePolicy, G4S5B0Profile,
    G4S5B0V37ContinuationTransactionReport, G4S5B3Profile, HomotopyExperimentProfile,
    HomotopyRhsTelemetryProfile, MatrixFreeCommonWProfile, NativeIntegratorGateReport,
    PathControllerProfile, ScientificCaseSpec, ScientificCorpusV2, ScientificFamily,
    StageBatchFeasibilityProfile, UnifiedNonlinearScreen, UnifiedScientificGateReport,
    UnifiedScreenProfile, V2CalibrationFreezeEnvelope, V2GateProfile, V2GateRow,
    freeze_v2_calibration, replay_v2_oregonator_holdout, run_a1_two_arm_receipt_cell,
    run_g1_transactional_gate, run_g2_exponential_gate, run_g3_fused_adaptive_gate,
    run_g4_prefix_kernel_gate, run_g4_s5b0_actual_level1_prefix_family,
    run_g4_s5b0_actual_level2_prefix_family, run_g4_s5b0_enforced_prefix_budget_family,
    run_g4_s5b0_frozen_full_e_shadow_economics, run_g4_s5b0_frozen_full_e_shadow_family,
    run_g4_s5b0_regime_atlas, run_g4_s5b0_rjf_attempt_trace, run_g4_s5b0_rjf_attempt_trace_family,
    run_g4_s5b0_rjf_only, run_g4_s5b0_rjf_only_family,
    run_g4_s5b0_stage_growth_safety_audit_family, run_g4_s5b0_v37_continuation_transaction_family,
    run_g4_s5b3_attempt_geometry, run_homotopy_design_check, run_homotopy_experiment_screen,
    run_homotopy_order_policy_screen, run_homotopy_rhs_telemetry_screen,
    run_matrix_free_common_w_gate, run_native_integrator_gates,
    run_p1_00_tolerance_scaled_early_defect, run_path_controller_screen,
    run_stage_batch_feasibility, run_unified_nonlinear_screen, run_unified_scientific_gates,
    verify_v2_calibration_freeze,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Parser)]
#[command(
    name = "rodas5p",
    version,
    about = "Rust parity laboratory for RODAS5P"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate the frozen coefficient and Rust numerical contracts.
    Validate {
        #[arg(long)]
        output: PathBuf,
    },
    /// Freeze a complete scientific-validity-v2 calibration measurement set.
    #[command(name = "scientific-validity-v2-freeze")]
    ScientificValidityV2Freeze {
        #[arg(long, value_enum)]
        profile: CliV2GateProfile,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Replay only the predeclared Oregonator holdout against an immutable v2 freeze.
    #[command(name = "scientific-validity-v2-holdout-replay")]
    ScientificValidityV2HoldoutReplay {
        #[arg(long, value_enum)]
        profile: CliV2GateProfile,
        #[arg(long)]
        freeze: PathBuf,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Execute all 54 canonical v2.1 calibration cases against a complete v2 reference manifest.
    #[command(name = "scientific-validity-v2-run-calibration")]
    ScientificValidityV2RunCalibration {
        #[arg(long)]
        reference_manifest: PathBuf,
        #[arg(long)]
        output: PathBuf,
        /// Immutable calibration freeze emitted directly from the completed campaign.
        #[arg(long)]
        freeze_output: PathBuf,
    },
    /// After validating a frozen calibration, execute only the three Oregonator cases.
    #[command(name = "scientific-validity-v2-run-oregonator")]
    ScientificValidityV2RunOregonator {
        #[arg(long, value_enum)]
        profile: CliV2GateProfile,
        #[arg(long)]
        freeze: PathBuf,
        /// Complete 54-case campaign emitted beside `--freeze` by run-calibration.
        #[arg(long)]
        calibration_campaign: PathBuf,
        #[arg(long)]
        reference_manifest: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Emit deterministic affine and structural screens for the homotopy research branch.
    HomotopyDesignCheck {
        #[arg(long)]
        output: PathBuf,
    },
    /// Run the deterministic nonlinear partial-coupling homotopy screen.
    HomotopyExperimentScreen {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Replay order-aware output policies and run deterministic trajectory gates.
    HomotopyOrderPolicyScreen {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long, default_value_t = 1)]
        threads: usize,
        #[arg(long)]
        output: PathBuf,
    },
    /// Measure genuine within-step stage batching for RHS, JVP and common-W solves.
    StageBatchFeasibility {
        #[arg(long, value_enum, default_value_t = CliStageBatchProfile::Canonical)]
        profile: CliStageBatchProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Record read-only numerical-rank and subspace telemetry for actual SABR/homotopy RHS batches.
    HomotopyRhsTelemetry {
        #[arg(long, value_enum, default_value_t = CliStageBatchProfile::Smoke)]
        profile: CliStageBatchProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Compare strict matrix-free common-W multiple-RHS Krylov solvers.
    MatrixFreeCommonWGate {
        #[arg(long, value_enum, default_value_t = CliStageBatchProfile::Smoke)]
        profile: CliStageBatchProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Compare bounded nonstationary homotopy schedules and path-rejection telemetry.
    HomotopyPathController {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run the generic eight-family transactional q1-to-q2 calibration gate.
    #[command(name = "generic-q1-q2-gate")]
    GenericQ1Q2Gate {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Compare the G1 transactional candidate against protected JF RODAS and frozen comparators.
    GenericQ1Q2Adaptive {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long, default_value_t = 1)]
        threads: usize,
        #[arg(long)]
        output: PathBuf,
    },
    /// Lock pexprb54s4 coefficients and validate matrix-free phi-action/order foundations.
    GenericParallelExponentialGate {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run the fused-phi adaptive parallel exponential G3 gate.
    GenericParallelExponentialAdaptive {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run actual reusable Arnoldi/GMRES prefix kernels without active method switching.
    GenericPrefixKernelGate {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Build the expanded paired regime atlas without active method switching.
    GenericRegimeAtlas {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Smoke)]
        profile: CliHomotopyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run one split-dimension R-JF-only regime replay for policy-redesign calibration or regression.
    GenericPolicyRedesignAtlas {
        #[arg(long, value_enum)]
        profile: CliPolicyRedesignProfile,
        #[arg(long, value_enum)]
        family: Option<CliPolicyRedesignFamily>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Record every R-JF trial for causal event-to-next-attempt analysis.
    GenericPolicyRedesignAttemptTrace {
        #[arg(long, value_enum)]
        profile: CliPolicyRedesignProfile,
        #[arg(long, value_enum)]
        family: Option<CliPolicyRedesignFamily>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Measure the actual pexprb54s4 U2/D2 level-one prefix on causal first-next proposals.
    GenericPolicyRedesignActualPrefix {
        #[arg(long, value_enum)]
        profile: CliPolicyRedesignProfile,
        #[arg(long, value_enum)]
        family: CliPolicyRedesignFamily,
        #[arg(long, value_enum)]
        policy: CliPolicyRedesignPrefixPolicy,
        #[arg(long)]
        output: PathBuf,
    },
    /// Measure actual pexprb54s4 dependency levels one and two without endpoint completion.
    GenericPolicyRedesignLevel2Prefix {
        #[arg(long, value_enum)]
        profile: CliLevel2PrefixProfile,
        #[arg(long, value_enum)]
        family: CliPolicyRedesignFamily,
        #[arg(long, value_enum)]
        policy: CliPolicyRedesignPrefixPolicy,
        #[arg(long)]
        output: PathBuf,
    },
    /// Calibrate the v2.9 normalized stage-growth witness on explicit fresh profiles.
    GenericStageGrowthSafetyAudit {
        #[arg(long, value_enum)]
        profile: CliStageGrowthSafetyProfile,
        #[arg(long, value_enum)]
        family: CliPolicyRedesignFamily,
        #[arg(long)]
        output: PathBuf,
    },
    /// Enforce the frozen v3.5 speculative-prefix JVP budget transactionally.
    GenericEnforcedPrefixBudget {
        #[arg(long, value_enum)]
        profile: CliStageGrowthSafetyProfile,
        #[arg(long, value_enum)]
        family: CliPolicyRedesignFamily,
        #[arg(long)]
        output: PathBuf,
    },
    /// Resume retained level-2 prefixes into the frozen read-only v3.6 full-E shadow.
    GenericFrozenFullEShadow {
        #[arg(long, value_enum)]
        profile: CliStageGrowthSafetyProfile,
        #[arg(long, value_enum)]
        family: CliPolicyRedesignFamily,
        #[arg(long)]
        output: PathBuf,
    },
    /// Generate one read-only A1 two-arm authority-receipt cell at the fixed N=320 holdout.
    A1TwoArmReceiptCell {
        #[arg(long, value_enum)]
        family: CliA1ReceiptFamily,
        #[arg(long, value_enum)]
        arm: CliA1ReceiptArm,
        #[arg(long)]
        repository: String,
        #[arg(long)]
        pull_request: u64,
        #[arg(long)]
        scientific_execution_head_sha: String,
        #[arg(long)]
        scientific_execution_head_tree: String,
        #[arg(long)]
        base_sha: String,
        #[arg(long)]
        base_tree: String,
        #[arg(long)]
        tested_execution_merge_sha: String,
        #[arg(long)]
        tested_execution_merge_tree: String,
        #[arg(long)]
        execution_workflow_run_id: u64,
        #[arg(long)]
        execution_workflow_run_attempt: u64,
        #[arg(long)]
        rust_version: String,
        #[arg(long)]
        cargo_version: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Resume frozen recommendations under the event-local v3.7 continuation transaction.
    GenericV37ContinuationTransaction {
        #[arg(long, value_enum)]
        profile: CliStageGrowthSafetyProfile,
        #[arg(long, value_enum)]
        family: CliPolicyRedesignFamily,
        #[arg(long)]
        output: PathBuf,
    },
    /// Measure all-six-family optimized paired wall economics for the v3.6 shadow.
    GenericFrozenFullEShadowEconomics {
        #[arg(long, value_enum)]
        profile: CliStageGrowthSafetyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Measure threshold-free early-defect attempt geometry and read-only overhead.
    GenericEarlyDefectAttemptGeometry {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Measure tolerance-scaled early-defect geometry without selecting a threshold.
    GenericToleranceScaledEarlyDefect {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Canonical)]
        profile: CliHomotopyProfile,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run deterministic fixed-step BDF and Radau scientific anchor gates.
    NativeIntegratorGates {
        #[arg(long)]
        output: PathBuf,
    },
    /// Compare all current adaptive integrator families on common analytic output grids.
    AdaptiveGlobalError {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Smoke)]
        profile: CliHomotopyProfile,
        #[arg(long, default_value_t = 1)]
        threads: usize,
        #[arg(long)]
        output: PathBuf,
    },
    /// Compare fixed-step complete-integrator anchors at common external global error.
    GlobalErrorPareto {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Smoke)]
        profile: CliHomotopyProfile,
        #[arg(long, default_value_t = 1)]
        threads: usize,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run one integrated linear/nonlinear candidate screen under matched contracts.
    UnifiedCandidateScreen {
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Smoke)]
        profile: CliHomotopyProfile,
        #[arg(long, default_value_t = 1)]
        threads: usize,
        #[arg(long)]
        full: bool,
        #[arg(long)]
        output: PathBuf,
    },
    /// Generate a deterministic immutable linear-system trace.
    Trace {
        #[arg(long, value_enum)]
        kind: CliSequenceKind,
        #[arg(long, default_value_t = 48)]
        dimension: usize,
        #[arg(long, default_value_t = 4)]
        steps: usize,
        #[arg(long, default_value_t = 8)]
        stages: usize,
        #[arg(long, default_value_t = 20260806)]
        seed: u64,
        #[arg(long, default_value_t = 1e3)]
        stiffness: f64,
        #[arg(long, default_value_t = 0.2)]
        nonnormality: f64,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run the strict Rust-only GMRES/LGMRES/GCRO-DR A/B comparison.
    Benchmark {
        #[arg(long)]
        trace: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 5)]
        repetitions: usize,
        #[arg(long, default_value_t = 1)]
        warmups: usize,
        #[arg(long, default_value_t = 20260806)]
        seed: u64,
        #[arg(long, default_value_t = 20)]
        restart: usize,
        #[arg(long, default_value_t = 6)]
        recycle_dim: usize,
        #[arg(long, default_value_t = 2000)]
        operator_budget: u64,
        #[arg(long, default_value_t = 1e-9)]
        rtol: f64,
        #[arg(long, default_value_t = 1e-12)]
        atol: f64,
        #[arg(long, value_enum, default_value_t = CliPreconditioner::None)]
        preconditioner: CliPreconditioner,
        #[arg(long)]
        zero_guess: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliV2GateProfile {
    Smoke,
    Canonical,
}

impl From<CliV2GateProfile> for V2GateProfile {
    fn from(value: CliV2GateProfile) -> Self {
        match value {
            CliV2GateProfile::Smoke => Self::Smoke,
            CliV2GateProfile::Canonical => Self::Canonical,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliPolicyRedesignFamily {
    Robertson,
    Hires,
    VanDerPol,
    RotatingNonnormal,
    NonautonomousForcing,
    Semilinear,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliA1ReceiptFamily {
    #[value(name = "robertson-ramped")]
    RobertsonRamped,
    #[value(name = "hires-ramped")]
    HiresRamped,
    #[value(name = "van-der-pol-ramped")]
    VanDerPolRamped,
    #[value(name = "rotating-nonnormal")]
    RotatingNonnormal,
    #[value(name = "nonautonomous-stiff-forcing")]
    NonautonomousStiffForcing,
    #[value(name = "semilinear-advection-diffusion-ramped")]
    SemilinearAdvectionDiffusionRamped,
}

impl From<CliA1ReceiptFamily> for G4S5B0Family {
    fn from(value: CliA1ReceiptFamily) -> Self {
        match value {
            CliA1ReceiptFamily::RobertsonRamped => Self::RobertsonRamped,
            CliA1ReceiptFamily::HiresRamped => Self::HiresRamped,
            CliA1ReceiptFamily::VanDerPolRamped => Self::VanDerPolRamped,
            CliA1ReceiptFamily::RotatingNonnormal => Self::RotatingNonnormal,
            CliA1ReceiptFamily::NonautonomousStiffForcing => Self::NonautonomousStiffForcing,
            CliA1ReceiptFamily::SemilinearAdvectionDiffusionRamped => {
                Self::SemilinearAdvectionDiffusionRamped
            }
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliA1ReceiptArm {
    #[value(name = "legacy-fixed")]
    LegacyFixed,
    #[value(name = "outer-scaled-numeric-parity")]
    OuterScaledNumericParity,
}

impl From<CliA1ReceiptArm> for rodas5p_integrators::G4S5B0LinearToleranceArm {
    fn from(value: CliA1ReceiptArm) -> Self {
        match value {
            CliA1ReceiptArm::LegacyFixed => Self::LegacyFixed,
            CliA1ReceiptArm::OuterScaledNumericParity => Self::OuterScaledNumericParity,
        }
    }
}

impl From<CliPolicyRedesignFamily> for G4S5B0Family {
    fn from(value: CliPolicyRedesignFamily) -> Self {
        match value {
            CliPolicyRedesignFamily::Robertson => Self::RobertsonRamped,
            CliPolicyRedesignFamily::Hires => Self::HiresRamped,
            CliPolicyRedesignFamily::VanDerPol => Self::VanDerPolRamped,
            CliPolicyRedesignFamily::RotatingNonnormal => Self::RotatingNonnormal,
            CliPolicyRedesignFamily::NonautonomousForcing => Self::NonautonomousStiffForcing,
            CliPolicyRedesignFamily::Semilinear => Self::SemilinearAdvectionDiffusionRamped,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliPolicyRedesignPrefixPolicy {
    FrozenK1,
    K3,
}

impl From<CliPolicyRedesignPrefixPolicy> for G4S5B0PrefixProbePolicy {
    fn from(value: CliPolicyRedesignPrefixPolicy) -> Self {
        match value {
            CliPolicyRedesignPrefixPolicy::FrozenK1 => Self::FrozenK1Comparator,
            CliPolicyRedesignPrefixPolicy::K3 => Self::K3Development,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliPolicyRedesignProfile {
    Calibration,
    Holdout,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliLevel2PrefixProfile {
    Calibration,
    Holdout,
    Discovery96,
    Discovery256,
}

impl From<CliLevel2PrefixProfile> for G4S5B0Profile {
    fn from(value: CliLevel2PrefixProfile) -> Self {
        match value {
            CliLevel2PrefixProfile::Calibration => Self::Calibration128,
            CliLevel2PrefixProfile::Holdout => Self::Holdout512,
            CliLevel2PrefixProfile::Discovery96 => Self::StageGrowthCalibration96,
            CliLevel2PrefixProfile::Discovery256 => Self::StageGrowthCalibration256,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliStageGrowthSafetyProfile {
    Calibration96,
    Calibration192,
    Calibration256,
    Holdout320,
    Holdout384,
}

impl From<CliStageGrowthSafetyProfile> for G4S5B0Profile {
    fn from(value: CliStageGrowthSafetyProfile) -> Self {
        match value {
            CliStageGrowthSafetyProfile::Calibration96 => Self::StageGrowthCalibration96,
            CliStageGrowthSafetyProfile::Calibration192 => Self::StageGrowthCalibration192,
            CliStageGrowthSafetyProfile::Calibration256 => Self::StageGrowthCalibration256,
            CliStageGrowthSafetyProfile::Holdout320 => Self::EnforcedBudgetHoldout320,
            CliStageGrowthSafetyProfile::Holdout384 => Self::StageGrowthHoldout384,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliHomotopyProfile {
    Smoke,
    Canonical,
}

impl From<CliHomotopyProfile> for HomotopyExperimentProfile {
    fn from(value: CliHomotopyProfile) -> Self {
        match value {
            CliHomotopyProfile::Smoke => Self::Smoke,
            CliHomotopyProfile::Canonical => Self::Canonical,
        }
    }
}

impl From<CliHomotopyProfile> for PathControllerProfile {
    fn from(value: CliHomotopyProfile) -> Self {
        match value {
            CliHomotopyProfile::Smoke => Self::Smoke,
            CliHomotopyProfile::Canonical => Self::Canonical,
        }
    }
}

impl From<CliHomotopyProfile> for G1TransactionalGateProfile {
    fn from(value: CliHomotopyProfile) -> Self {
        match value {
            CliHomotopyProfile::Smoke => Self::Smoke,
            CliHomotopyProfile::Canonical => Self::Canonical,
        }
    }
}

impl From<CliHomotopyProfile> for G2ExponentialGateProfile {
    fn from(value: CliHomotopyProfile) -> Self {
        match value {
            CliHomotopyProfile::Smoke => Self::Smoke,
            CliHomotopyProfile::Canonical => Self::Canonical,
        }
    }
}

impl From<CliHomotopyProfile> for UnifiedScreenProfile {
    fn from(value: CliHomotopyProfile) -> Self {
        match value {
            CliHomotopyProfile::Smoke => Self::Smoke,
            CliHomotopyProfile::Canonical => Self::Canonical,
        }
    }
}

impl From<CliHomotopyProfile> for GlobalErrorParetoProfile {
    fn from(value: CliHomotopyProfile) -> Self {
        match value {
            CliHomotopyProfile::Smoke => Self::Smoke,
            CliHomotopyProfile::Canonical => Self::Canonical,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliStageBatchProfile {
    Smoke,
    Canonical,
}

impl From<CliStageBatchProfile> for StageBatchFeasibilityProfile {
    fn from(value: CliStageBatchProfile) -> Self {
        match value {
            CliStageBatchProfile::Smoke => Self::Smoke,
            CliStageBatchProfile::Canonical => Self::Canonical,
        }
    }
}

impl From<CliStageBatchProfile> for HomotopyRhsTelemetryProfile {
    fn from(value: CliStageBatchProfile) -> Self {
        match value {
            CliStageBatchProfile::Smoke => Self::Smoke,
            CliStageBatchProfile::Canonical => Self::Canonical,
        }
    }
}

impl From<CliStageBatchProfile> for MatrixFreeCommonWProfile {
    fn from(value: CliStageBatchProfile) -> Self {
        match value {
            CliStageBatchProfile::Smoke => Self::Smoke,
            CliStageBatchProfile::Canonical => Self::Canonical,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliSequenceKind {
    Fixed,
    SlowDrift,
    Abrupt,
    Rotating,
}

impl From<CliSequenceKind> for SequenceKind {
    fn from(value: CliSequenceKind) -> Self {
        match value {
            CliSequenceKind::Fixed => Self::Fixed,
            CliSequenceKind::SlowDrift => Self::SlowDrift,
            CliSequenceKind::Abrupt => Self::Abrupt,
            CliSequenceKind::Rotating => Self::Rotating,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliPreconditioner {
    None,
    Jacobi,
}

impl From<CliPreconditioner> for PreconditionerKind {
    fn from(value: CliPreconditioner) -> Self {
        match value {
            CliPreconditioner::None => Self::None,
            CliPreconditioner::Jacobi => Self::Jacobi,
        }
    }
}

#[derive(Serialize)]
struct BenchmarkDocument {
    schema: &'static str,
    trace_id: String,
    failures: usize,
    plan: BenchmarkPlan,
    summary: Vec<rodas5p_fair_ab::SummaryRow>,
    comparison: rodas5p_fair_ab::ComparisonResult,
}

#[derive(Serialize)]
struct UnifiedLinearSuite {
    kind: SequenceKind,
    trace_id: String,
    failures: usize,
    plan: BenchmarkPlan,
    summary: Vec<rodas5p_fair_ab::SummaryRow>,
    comparison: Option<rodas5p_fair_ab::ComparisonResult>,
}

#[derive(Serialize)]
struct UnifiedCandidateDocument {
    schema: &'static str,
    status: &'static str,
    profile: &'static str,
    threads: usize,
    scientific_checksum: String,
    catalog: CandidateCatalog,
    linear_suites: Vec<UnifiedLinearSuite>,
    linear_assessments: Vec<UnifiedLinearCandidateAssessment>,
    nonlinear: UnifiedNonlinearScreen,
    nonlinear_assessments: Vec<UnifiedNonlinearCandidateAssessment>,
    scientific_gates: UnifiedScientificGateReport,
    native_integrator_gates: NativeIntegratorGateReport,
    joint_assessments: Vec<UnifiedJointCandidateAssessment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum UnifiedJointVerdict {
    Reference,
    Promote,
    Hold,
    Deferred,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct UnifiedLinearCandidateAssessment {
    candidate_id: String,
    solver: SolverKind,
    lifetime: RecycleLifetime,
    suites: usize,
    failures: usize,
    maximum_relative_solution_error: f64,
    median_wall_ratio_to_gmres_off: Option<f64>,
    median_operator_ratio_to_gmres_off: Option<f64>,
    median_wall_speedup: Option<f64>,
    required_wall_speedup: f64,
    verdict: UnifiedJointVerdict,
    blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct UnifiedNonlinearCandidateAssessment {
    candidate_id: String,
    family: CandidateFamily,
    cases: usize,
    failures: usize,
    median_compute_ratio_to_direct: Option<f64>,
    median_rhs_evaluation_ratio_to_direct: Option<f64>,
    median_jvp_vector_ratio_to_direct: Option<f64>,
    median_batch_depth_ratio_to_direct: Option<f64>,
    median_batch_vector_ratio_to_direct: Option<f64>,
    median_wall_speedup: Option<f64>,
    required_wall_speedup: f64,
    verdict: UnifiedJointVerdict,
    blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct UnifiedJointCandidateAssessment {
    candidate_id: String,
    family: CandidateFamily,
    verdict: UnifiedJointVerdict,
    scientific_eligible: bool,
    tier_l_verdict: Option<UnifiedJointVerdict>,
    tier_n_verdict: Option<UnifiedJointVerdict>,
    blockers: Vec<String>,
}

const TIER_L_REQUIRED_WALL_SPEEDUP: f64 = 1.15;
const TIER_N_REQUIRED_WALL_SPEEDUP: f64 = 1.15;

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&fs::read(path).with_context(|| format!("reading {}", path.display()))?)
        .with_context(|| format!("parsing {}", path.display()))
}

fn write_json_create_new<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    if path.exists() {
        anyhow::bail!("immutable output already exists: {}", path.display());
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("immutable output has no UTF-8 file name")?;
    let temporary = path.with_file_name(format!(".{file_name}.tmp.{}", std::process::id()));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("creating atomic temporary output {}", temporary.display()))?;
    if let Err(error) = output.write_all(&bytes).and_then(|()| output.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(error).with_context(|| format!("writing {}", temporary.display()));
    }
    drop(output);
    if let Err(error) = fs::hard_link(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error).with_context(|| format!("atomically publishing {}", path.display()));
    }
    fs::remove_file(&temporary)
        .with_context(|| format!("removing atomic temporary output {}", temporary.display()))
}

fn preflight_create_new_outputs(paths: &[&Path]) -> Result<()> {
    for (index, path) in paths.iter().enumerate() {
        if path.exists() {
            anyhow::bail!("immutable output already exists: {}", path.display());
        }
        if paths[..index].contains(path) {
            anyhow::bail!(
                "immutable outputs must use distinct paths: {}",
                path.display()
            );
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
enum V2CampaignCaseRecord {
    Complete {
        artifact: Box<ScientificValidityV2CaseArtifact>,
    },
    Failed {
        spec: Box<ScientificCaseSpec>,
        phase: String,
        error: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct V2CalibrationCampaignDocument {
    schema: String,
    status: String,
    corpus_version: String,
    code_revision: String,
    expected_case_count: usize,
    attempted_case_count: usize,
    failure_count: usize,
    freeze_eligible: bool,
    freeze_checksum_sha256: Option<String>,
    freeze_admission_error: Option<String>,
    record_set_sha256: String,
    records: Vec<V2CampaignCaseRecord>,
    rows: Vec<V2GateRow>,
}

fn v2_record_set_sha256(records: &[V2CampaignCaseRecord]) -> String {
    let checksum_ledger = records
        .iter()
        .map(|record| match record {
            V2CampaignCaseRecord::Complete { artifact } => json!({
                "case_id": artifact.spec.id,
                "status": "complete",
                "artifact_checksum_sha256": artifact.artifact_checksum_sha256,
            }),
            V2CampaignCaseRecord::Failed { spec, error, .. } => json!({
                "case_id": spec.id,
                "status": "failed",
                "error": error,
            }),
        })
        .collect::<Vec<_>>();
    let mut checksum_bytes = b"vigilode-scientific-v2-campaign-record-set-v1\0".to_vec();
    checksum_bytes.extend_from_slice(
        &serde_json::to_vec(&checksum_ledger).expect("JSON value serialization cannot fail"),
    );
    sha256_hex(&checksum_bytes)
}

fn complete_v2_case_artifacts(
    records: &[V2CampaignCaseRecord],
) -> Vec<ScientificValidityV2CaseArtifact> {
    records
        .iter()
        .filter_map(|record| match record {
            V2CampaignCaseRecord::Complete { artifact } => Some((**artifact).clone()),
            V2CampaignCaseRecord::Failed { .. } => None,
        })
        .collect()
}

fn validate_v2_calibration_campaign_document(
    campaign: &V2CalibrationCampaignDocument,
) -> Result<V2CalibrationFreezeEnvelope> {
    let revision = scientific_validity_v2_compiled_revision()?;
    if campaign.schema != "scientific-validity-v2-calibration-campaign-v1"
        || campaign.status != "complete-pass"
        || campaign.corpus_version != ScientificCorpusV2::VERSION
        || campaign.code_revision != revision
        || campaign.expected_case_count != 54
        || campaign.attempted_case_count != 54
        || campaign.failure_count != 0
        || !campaign.freeze_eligible
        || campaign.freeze_admission_error.is_some()
        || campaign.records.len() != 54
        || campaign.rows.len() != 54
        || campaign.record_set_sha256 != v2_record_set_sha256(&campaign.records)
    {
        anyhow::bail!(
            "canonical calibration campaign aggregate failed identity/cardinality checks"
        );
    }

    let expected_ids = ScientificCorpusV2::calibration_specs()
        .into_iter()
        .map(|spec| spec.id)
        .collect::<std::collections::BTreeSet<_>>();
    let mut record_ids = std::collections::BTreeSet::new();
    let mut artifact_rows = Vec::with_capacity(54);
    for record in &campaign.records {
        let V2CampaignCaseRecord::Complete { artifact } = record else {
            anyhow::bail!("canonical calibration campaign contains a failed case record");
        };
        validate_scientific_validity_v2_case_artifact(artifact)?;
        if !record_ids.insert(artifact.spec.id.clone()) {
            anyhow::bail!("canonical calibration campaign contains a duplicate case artifact");
        }
        artifact_rows.push(artifact.row.clone());
    }
    if record_ids != expected_ids {
        anyhow::bail!("canonical calibration campaign does not contain the exact 54-case set");
    }
    artifact_rows.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    let mut declared_rows = campaign.rows.clone();
    declared_rows.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    if artifact_rows != declared_rows {
        anyhow::bail!("canonical calibration rows differ from their validated case artifacts");
    }
    let artifacts = complete_v2_case_artifacts(&campaign.records);
    let freeze = freeze_scientific_validity_v2_calibration_artifacts(&artifacts)?;
    if campaign.freeze_checksum_sha256.as_deref() != Some(freeze.checksum_sha256.as_str()) {
        anyhow::bail!("canonical calibration campaign freeze checksum mismatch");
    }
    Ok(freeze)
}

fn run_v2_case_records(
    reference_manifest: &Path,
    specs: Vec<ScientificCaseSpec>,
) -> (Vec<V2CampaignCaseRecord>, Vec<V2GateRow>, usize, String) {
    let mut records = Vec::with_capacity(specs.len());
    let mut rows = Vec::with_capacity(specs.len());
    let mut failures = 0_usize;
    for spec in specs {
        let result = load_numerical_reference_v2(reference_manifest, &spec)
            .and_then(|reference| run_scientific_validity_v2_case(&spec, &reference));
        match result {
            Ok(artifact) => {
                rows.push(artifact.row.clone());
                records.push(V2CampaignCaseRecord::Complete {
                    artifact: Box::new(artifact),
                });
            }
            Err(error) => {
                failures += 1;
                let message = error.to_string();
                records.push(V2CampaignCaseRecord::Failed {
                    spec: Box::new(spec),
                    phase: "reference-load-or-paired-integration".into(),
                    error: message,
                });
            }
        }
    }
    let record_set_sha256 = v2_record_set_sha256(&records);
    (records, rows, failures, record_set_sha256)
}

fn write_v37_continuation_transaction_report(
    path: &Path,
    report: &G4S5B0V37ContinuationTransactionReport,
) -> Result<()> {
    if !report.hard_gates.passed {
        anyhow::bail!(
            "v3.7 continuation transaction hard gates failed; refusing partial authority output"
        );
    }
    write_json(path, report)
}

fn strict_cells() -> Vec<BenchmarkCell> {
    vec![
        BenchmarkCell::new(SolverKind::Gmres, RecycleLifetime::Off),
        BenchmarkCell::new(SolverKind::Lgmres, RecycleLifetime::Off),
        BenchmarkCell::new(SolverKind::Lgmres, RecycleLifetime::Stage),
        BenchmarkCell::new(SolverKind::Lgmres, RecycleLifetime::Persistent),
        BenchmarkCell::new(SolverKind::Gcrodr, RecycleLifetime::Off),
        BenchmarkCell::new(SolverKind::Gcrodr, RecycleLifetime::Stage),
        BenchmarkCell::new(SolverKind::Gcrodr, RecycleLifetime::Persistent),
    ]
}

fn linear_candidate_id(solver: SolverKind, lifetime: RecycleLifetime) -> String {
    let solver = match solver {
        SolverKind::Gmres => "gmres",
        SolverKind::Lgmres => "lgmres",
        SolverKind::Gcrodr => "gcrodr",
    };
    let lifetime = match lifetime {
        RecycleLifetime::Off => "off",
        RecycleLifetime::Stage => "stage",
        RecycleLifetime::Persistent => "persistent",
    };
    format!("sequential-{solver}-{lifetime}")
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        0.5 * (values[middle - 1] + values[middle])
    } else {
        values[middle]
    })
}

fn assess_linear_candidates(
    suites: &[UnifiedLinearSuite],
) -> Vec<UnifiedLinearCandidateAssessment> {
    strict_cells()
        .into_iter()
        .map(|cell| {
            let mut wall_ratios = Vec::new();
            let mut operator_ratios = Vec::new();
            let mut failures = 0_usize;
            let mut maximum_relative_solution_error = 0.0_f64;
            let mut represented_suites = 0_usize;
            for suite in suites {
                let reference = suite.summary.iter().find(|row| {
                    row.solver == SolverKind::Gmres && row.lifetime == RecycleLifetime::Off
                });
                let candidate = suite
                    .summary
                    .iter()
                    .find(|row| row.solver == cell.solver && row.lifetime == cell.lifetime);
                let (Some(reference), Some(candidate)) = (reference, candidate) else {
                    continue;
                };
                represented_suites += 1;
                failures += candidate.failures;
                maximum_relative_solution_error =
                    maximum_relative_solution_error.max(candidate.maximum_relative_solution_error);
                if reference.wall_median_seconds > 0.0 {
                    wall_ratios.push(candidate.wall_median_seconds / reference.wall_median_seconds);
                }
                if reference.operator_total_median > 0.0 {
                    operator_ratios
                        .push(candidate.operator_total_median / reference.operator_total_median);
                }
            }
            let wall_ratio = median(wall_ratios);
            let operator_ratio = median(operator_ratios);
            let wall_speedup = wall_ratio
                .filter(|ratio| *ratio > 0.0)
                .map(|ratio| 1.0 / ratio);
            let is_reference =
                cell.solver == SolverKind::Gmres && cell.lifetime == RecycleLifetime::Off;
            let mut blockers = Vec::new();
            if represented_suites != suites.len() {
                blockers.push("missing one or more Tier-L trace summaries".into());
            }
            if failures > 0 {
                blockers.push(format!("{failures} Tier-L solve failures"));
            }
            if !maximum_relative_solution_error.is_finite() {
                blockers.push("nonfinite Tier-L solution error".into());
            }
            if !is_reference
                && !wall_speedup.is_some_and(|speedup| speedup >= TIER_L_REQUIRED_WALL_SPEEDUP)
            {
                blockers.push(format!(
                    "median Tier-L wall speedup below {:.2}x",
                    TIER_L_REQUIRED_WALL_SPEEDUP
                ));
            }
            if !is_reference && operator_ratio.is_some_and(|ratio| ratio > 1.0) {
                blockers.push("median Tier-L operator work exceeds GMRES/OFF".into());
            }
            let verdict = if is_reference {
                UnifiedJointVerdict::Reference
            } else if blockers.is_empty() {
                UnifiedJointVerdict::Promote
            } else {
                UnifiedJointVerdict::Hold
            };
            UnifiedLinearCandidateAssessment {
                candidate_id: linear_candidate_id(cell.solver, cell.lifetime),
                solver: cell.solver,
                lifetime: cell.lifetime,
                suites: represented_suites,
                failures,
                maximum_relative_solution_error,
                median_wall_ratio_to_gmres_off: wall_ratio,
                median_operator_ratio_to_gmres_off: operator_ratio,
                median_wall_speedup: wall_speedup,
                required_wall_speedup: TIER_L_REQUIRED_WALL_SPEEDUP,
                verdict,
                blockers,
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn nonlinear_performance_verdict(
    is_reference: bool,
    represented_cases: usize,
    expected_cases: usize,
    failures: usize,
    compute_ratio: Option<f64>,
    rhs_ratio: Option<f64>,
    jvp_ratio: Option<f64>,
    _batch_depth_ratio: Option<f64>,
) -> (UnifiedJointVerdict, Vec<String>) {
    if is_reference {
        return (UnifiedJointVerdict::Reference, Vec::new());
    }
    let mut blockers = Vec::new();
    if represented_cases != expected_cases {
        blockers.push("missing one or more Tier-N case results".into());
    }
    if failures > 0 {
        blockers.push(format!(
            "{failures} Tier-N execution/certification failures"
        ));
    }
    let wall_speedup = compute_ratio
        .filter(|ratio| *ratio > 0.0)
        .map(|ratio| 1.0 / ratio);
    if !wall_speedup.is_some_and(|speedup| speedup >= TIER_N_REQUIRED_WALL_SPEEDUP) {
        blockers.push(format!(
            "median nonlinear candidate wall speedup below {:.2}x",
            TIER_N_REQUIRED_WALL_SPEEDUP
        ));
    }
    if wall_speedup.is_none_or(|speedup| speedup < TIER_N_REQUIRED_WALL_SPEEDUP) {
        if rhs_ratio.is_some_and(|ratio| ratio > 1.0) {
            blockers.push("median nonlinear RHS work exceeds sequential/direct".into());
        }
        if jvp_ratio.is_some_and(|ratio| ratio > 1.0) {
            blockers.push("median nonlinear JVP work exceeds sequential/direct".into());
        }
    }
    let verdict = if blockers.is_empty() {
        UnifiedJointVerdict::Promote
    } else {
        UnifiedJointVerdict::Hold
    };
    (verdict, blockers)
}

fn ratio_if_positive(numerator: u64, denominator: u64) -> Option<f64> {
    (denominator > 0).then_some(numerator as f64 / denominator as f64)
}

fn assess_nonlinear_candidates(
    catalog: &CandidateCatalog,
    nonlinear: &UnifiedNonlinearScreen,
) -> Vec<UnifiedNonlinearCandidateAssessment> {
    let references: std::collections::BTreeMap<_, _> = nonlinear
        .rows
        .iter()
        .filter(|row| row.candidate_id == "sequential-direct-off")
        .map(|row| (row.case_id.as_str(), row))
        .collect();
    catalog
        .entries()
        .iter()
        .filter(|candidate| {
            !matches!(candidate.status(), CandidateStatus::Deferred { .. })
                && candidate.is_rodas_stage_candidate()
        })
        .map(|candidate| {
            let rows: Vec<_> = nonlinear
                .rows
                .iter()
                .filter(|row| row.candidate_id == candidate.id())
                .collect();
            let mut compute_ratios = Vec::new();
            let mut rhs_ratios = Vec::new();
            let mut jvp_ratios = Vec::new();
            let mut batch_depth_ratios = Vec::new();
            let mut batch_vector_ratios = Vec::new();
            let mut failures = 0_usize;
            for row in &rows {
                if matches!(
                    row.outcome,
                    rodas5p_integrators::UnifiedCandidateOutcome::NumericalFailure
                        | rodas5p_integrators::UnifiedCandidateOutcome::Uncertified
                ) {
                    failures += 1;
                }
                let Some(reference) = references.get(row.case_id.as_str()) else {
                    continue;
                };
                if reference.compute_seconds > 0.0 {
                    compute_ratios.push(row.compute_seconds / reference.compute_seconds);
                }
                if let Some(ratio) = ratio_if_positive(
                    row.candidate_counters.rhs_evaluations,
                    reference.candidate_counters.rhs_evaluations,
                ) {
                    rhs_ratios.push(ratio);
                }
                if let Some(ratio) = ratio_if_positive(
                    row.candidate_counters.jvp_vectors,
                    reference.candidate_counters.jvp_vectors,
                ) {
                    jvp_ratios.push(ratio);
                }
                if reference.batch_depth > 0 {
                    batch_depth_ratios.push(row.batch_depth as f64 / reference.batch_depth as f64);
                }
                if reference.batch_vectors > 0 {
                    batch_vector_ratios
                        .push(row.batch_vectors as f64 / reference.batch_vectors as f64);
                }
            }
            let compute_ratio = median(compute_ratios);
            let rhs_ratio = median(rhs_ratios);
            let jvp_ratio = median(jvp_ratios);
            let batch_depth_ratio = median(batch_depth_ratios);
            let batch_vector_ratio = median(batch_vector_ratios);
            let wall_speedup = compute_ratio
                .filter(|ratio| *ratio > 0.0)
                .map(|ratio| 1.0 / ratio);
            let (verdict, blockers) = nonlinear_performance_verdict(
                candidate.id() == "sequential-direct-off",
                rows.len(),
                nonlinear.cases.len(),
                failures,
                compute_ratio,
                rhs_ratio,
                jvp_ratio,
                batch_depth_ratio,
            );
            UnifiedNonlinearCandidateAssessment {
                candidate_id: candidate.id().to_string(),
                family: candidate.family(),
                cases: rows.len(),
                failures,
                median_compute_ratio_to_direct: compute_ratio,
                median_rhs_evaluation_ratio_to_direct: rhs_ratio,
                median_jvp_vector_ratio_to_direct: jvp_ratio,
                median_batch_depth_ratio_to_direct: batch_depth_ratio,
                median_batch_vector_ratio_to_direct: batch_vector_ratio,
                median_wall_speedup: wall_speedup,
                required_wall_speedup: TIER_N_REQUIRED_WALL_SPEEDUP,
                verdict,
                blockers,
            }
        })
        .collect()
}

fn build_joint_assessments(
    catalog: &CandidateCatalog,
    gates: &UnifiedScientificGateReport,
    native: &NativeIntegratorGateReport,
    linear: &[UnifiedLinearCandidateAssessment],
    nonlinear: &[UnifiedNonlinearCandidateAssessment],
) -> Vec<UnifiedJointCandidateAssessment> {
    catalog
        .entries()
        .iter()
        .map(|candidate| {
            if matches!(candidate.status(), CandidateStatus::Deferred { .. }) {
                return UnifiedJointCandidateAssessment {
                    candidate_id: candidate.id().to_string(),
                    family: candidate.family(),
                    verdict: UnifiedJointVerdict::Deferred,
                    scientific_eligible: false,
                    tier_l_verdict: None,
                    tier_n_verdict: None,
                    blockers: vec!["Rust implementation is deferred".into()],
                };
            }
            if candidate.is_native_complete_integrator() {
                let row = native
                    .rows
                    .iter()
                    .find(|row| row.candidate_id == candidate.id());
                let scientific_eligible = row.is_some_and(|row| {
                    row.order_pass && row.stiff_pass && row.mass_pass && row.failures == 0
                });
                let mut blockers = Vec::new();
                match row {
                    Some(row) => {
                        if !row.order_pass {
                            blockers.push("native complete-integrator order gate failed".into());
                        }
                        if !row.stiff_pass {
                            blockers.push("native complete-integrator stiff gate failed".into());
                        }
                        if !row.mass_pass {
                            blockers.push(
                                "native complete-integrator nonlinear mass-matrix gate failed"
                                    .into(),
                            );
                        }
                        if row.failures > 0 {
                            blockers.push(format!(
                                "{} native complete-integrator gate failures",
                                row.failures
                            ));
                        }
                    }
                    None => blockers.push("native complete-integrator gate result missing".into()),
                }
                blockers.push(
                    "complete-integrator global-error versus total-cost performance assessment required"
                        .into(),
                );
                return UnifiedJointCandidateAssessment {
                    candidate_id: candidate.id().to_string(),
                    family: candidate.family(),
                    verdict: UnifiedJointVerdict::Hold,
                    scientific_eligible,
                    tier_l_verdict: None,
                    tier_n_verdict: None,
                    blockers,
                };
            }
            let gate = gates
                .candidates
                .iter()
                .find(|row| row.candidate_id == candidate.id());
            let scientific_eligible = gate.is_some_and(|row| {
                row.order_pass
                    && row.stiff_decay_pass
                    && row.one_step_failures == 0
                    && row.c3_false_accepts == 0
                    && row.c3_reference_fallbacks == 0
                    && row.nonnormal_pass
            });
            let mut blockers = gate.map_or_else(
                || vec!["scientific gate result missing".into()],
                |row| row.blockers.clone(),
            );
            let tier_l = linear.iter().find(|row| row.candidate_id == candidate.id());
            let tier_l_verdict = tier_l.map(|row| row.verdict);
            let tier_n = nonlinear
                .iter()
                .find(|row| row.candidate_id == candidate.id());
            let tier_n_verdict = tier_n.map(|row| row.verdict);
            if candidate.family() == CandidateFamily::Sequential
                && candidate.id() != "sequential-direct-off"
            {
                blockers.retain(|blocker| blocker != "Tier-L performance assessment required");
                if let Some(linear) = tier_l {
                    blockers.extend(linear.blockers.clone());
                } else {
                    blockers.push("Tier-L assessment missing".into());
                }
            } else if candidate.id() != "sequential-direct-off" {
                if let Some(nonlinear) = tier_n {
                    blockers.extend(nonlinear.blockers.clone());
                } else {
                    blockers.push("Tier-N performance assessment missing".into());
                }
            }
            blockers.sort();
            blockers.dedup();
            let verdict = if candidate.id() == "sequential-direct-off"
                || tier_l_verdict == Some(UnifiedJointVerdict::Reference)
            {
                UnifiedJointVerdict::Reference
            } else if scientific_eligible
                && ((candidate.family() == CandidateFamily::Sequential
                    && tier_l_verdict == Some(UnifiedJointVerdict::Promote))
                    || (candidate.family() != CandidateFamily::Sequential
                        && tier_n_verdict == Some(UnifiedJointVerdict::Promote)))
                && blockers.is_empty()
            {
                UnifiedJointVerdict::Promote
            } else {
                UnifiedJointVerdict::Hold
            };
            UnifiedJointCandidateAssessment {
                candidate_id: candidate.id().to_string(),
                family: candidate.family(),
                verdict,
                scientific_eligible,
                tier_l_verdict,
                tier_n_verdict,
                blockers,
            }
        })
        .collect()
}

fn unified_document_status(
    linear_failures: usize,
    nonlinear_failures: usize,
    uncertified: usize,
) -> &'static str {
    if linear_failures > 0 || nonlinear_failures > 0 {
        "complete-with-failures"
    } else if uncertified > 0 {
        "complete-with-uncertified"
    } else {
        "complete"
    }
}

fn unified_linear_configs(profile: CliHomotopyProfile) -> Vec<SequenceConfig> {
    let kinds: &[SequenceKind] = match profile {
        CliHomotopyProfile::Smoke => &[SequenceKind::Fixed],
        CliHomotopyProfile::Canonical => &[
            SequenceKind::Fixed,
            SequenceKind::SlowDrift,
            SequenceKind::Abrupt,
            SequenceKind::Rotating,
        ],
    };
    kinds
        .iter()
        .enumerate()
        .map(|(index, &kind)| SequenceConfig {
            kind,
            dimension: if matches!(profile, CliHomotopyProfile::Smoke) {
                8
            } else {
                48
            },
            steps: if matches!(profile, CliHomotopyProfile::Smoke) {
                1
            } else {
                4
            },
            stages: 8,
            seed: 20260808 + index as u64,
            stiffness: if matches!(profile, CliHomotopyProfile::Smoke) {
                100.0
            } else {
                1_000.0
            },
            nonnormality: if matches!(profile, CliHomotopyProfile::Smoke) {
                0.05
            } else {
                0.2
            },
        })
        .collect()
}

fn run_unified_candidate_document(
    profile: CliHomotopyProfile,
    threads: usize,
    full: bool,
) -> Result<UnifiedCandidateDocument> {
    let repetitions = if matches!(profile, CliHomotopyProfile::Smoke) {
        1
    } else {
        3
    };
    let warmups = usize::from(matches!(profile, CliHomotopyProfile::Canonical));
    let plan = BenchmarkPlan {
        cells: strict_cells(),
        repetitions,
        warmups,
        seed: 20260808,
    };
    let mut linear_suites = Vec::new();
    for trace_config in unified_linear_configs(profile) {
        let trace = generate_trace(&trace_config)?;
        let comparison = run_comparison(&trace, &plan, |solver| FairSolveConfig {
            solver,
            rtol: 1e-9,
            atol: 1e-12,
            restart: 20,
            recycle_dim: 6,
            hard_operator_budget: 2_000,
            preconditioner: PreconditionerKind::None,
            use_previous_oracle_guess: true,
        })?;
        let summary = summarize_comparison(&comparison);
        let failures = summary.iter().map(|row| row.failures).sum();
        linear_suites.push(UnifiedLinearSuite {
            kind: trace_config.kind,
            trace_id: trace.trace_id,
            failures,
            plan: plan.clone(),
            summary,
            comparison: full.then_some(comparison),
        });
    }
    let mut nonlinear = run_unified_nonlinear_screen(profile.into(), threads)?;
    let scientific_gates = run_unified_scientific_gates(profile.into(), threads, &nonlinear)?;
    let native_integrator_gates = run_native_integrator_gates()?;
    let linear_assessments = assess_linear_candidates(&linear_suites);
    let catalog = CandidateCatalog::research_default()?;
    let nonlinear_assessments = assess_nonlinear_candidates(&catalog, &nonlinear);
    let joint_assessments = build_joint_assessments(
        &catalog,
        &scientific_gates,
        &native_integrator_gates,
        &linear_assessments,
        &nonlinear_assessments,
    );
    let mut scientific_rows = nonlinear.rows.clone();
    for row in &mut scientific_rows {
        row.compute_seconds = 0.0;
        row.certificate_seconds = 0.0;
    }
    let linear_scientific: Vec<_> = linear_suites
        .iter()
        .map(|suite| {
            json!({
                "kind": suite.kind,
                "trace_id": suite.trace_id,
                "failures": suite.failures,
                "summary": suite.summary.iter().map(|row| json!({
                    "solver": row.solver,
                    "lifetime": row.lifetime,
                    "failures": row.failures,
                    "operator_total_median": row.operator_total_median,
                    "maximum_relative_solution_error": row.maximum_relative_solution_error,
                })).collect::<Vec<_>>()
            })
        })
        .collect();
    let mut gate_scientific = scientific_gates.clone();
    gate_scientific.compute_seconds = 0.0;
    gate_scientific.threads = 0;
    let checksum_payload = serde_json::to_vec(&json!({
        "linear": linear_scientific,
        "nonlinear_cases": nonlinear.cases,
        "nonlinear_rows": scientific_rows,
        "nonlinear_summary": nonlinear.summary,
        "scientific_gates": gate_scientific,
        "native_integrator_gates": &native_integrator_gates,
    }))?;
    let scientific_checksum = sha256_hex(&checksum_payload);
    let linear_failures: usize = linear_suites.iter().map(|suite| suite.failures).sum();
    let status = unified_document_status(
        linear_failures,
        nonlinear.summary.failures,
        nonlinear.summary.uncertified,
    );
    // The report-level compute time is intentionally retained for performance analysis but is
    // excluded from the scientific checksum above.
    nonlinear
        .rows
        .sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
    Ok(UnifiedCandidateDocument {
        schema: "rodas5p-unified-candidate-screen-v4",
        status,
        profile: match profile {
            CliHomotopyProfile::Smoke => "smoke",
            CliHomotopyProfile::Canonical => "canonical",
        },
        threads,
        scientific_checksum,
        catalog,
        linear_suites,
        linear_assessments,
        nonlinear,
        nonlinear_assessments,
        scientific_gates,
        native_integrator_gates,
        joint_assessments,
    })
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { output } => {
            let coefficients = load_rodas5p_coefficients()?;
            let result = json!({
                "schema": "rodas5p-rust-validation-v1",
                "status": if coefficients.stages() == 8 && coefficients.gamma.is_finite() { "pass" } else { "fail" },
                "crate_version": env!("CARGO_PKG_VERSION"),
                "rust_toolchain_lock": "1.94.1",
                "linear_algebra_backend": "faer-0.24.4",
                "stages": coefficients.stages(),
                "gamma": coefficients.gamma,
            });
            write_json(&output, &result)?;
        }
        Command::ScientificValidityV2Freeze {
            profile,
            input,
            output,
        } => {
            let profile = V2GateProfile::from(profile);
            if profile == V2GateProfile::Canonical {
                anyhow::bail!(
                    "canonical raw-row freeze is disabled; use scientific-validity-v2-run-calibration so the freeze is emitted by the source-bound 54-case producer"
                );
            }
            let rows: Vec<V2GateRow> = read_json(&input)?;
            match freeze_v2_calibration(profile, rows.clone()) {
                Ok(freeze) => write_json_create_new(&output, &freeze)?,
                Err(error) => {
                    write_json_create_new(
                        &output,
                        &json!({
                            "schema": "scientific-validity-v2-calibration-freeze-failure-v1",
                            "status": "fail",
                            "profile": profile,
                            "campaign_label": profile.campaign_label(),
                            "error": error.to_string(),
                            "rows": rows,
                        }),
                    )?;
                    return Err(error.into());
                }
            }
        }
        Command::ScientificValidityV2HoldoutReplay {
            profile,
            freeze,
            input,
            output,
        } => {
            let profile = V2GateProfile::from(profile);
            if profile == V2GateProfile::Canonical {
                anyhow::bail!(
                    "canonical raw-row holdout replay is disabled; use scientific-validity-v2-run-oregonator so the three rows are emitted by the source-bound producer"
                );
            }
            let calibration_freeze: V2CalibrationFreezeEnvelope = read_json(&freeze)?;
            if let Err(error) = verify_v2_calibration_freeze(&calibration_freeze) {
                write_json_create_new(
                    &output,
                    &json!({
                        "schema": "scientific-validity-v2-oregonator-holdout-replay-failure-v1",
                        "status": "fail",
                        "profile": profile,
                        "campaign_label": profile.campaign_label(),
                        "error": error.to_string(),
                        "calibration_checksum_sha256": calibration_freeze.checksum_sha256,
                        "rows": [],
                        "holdout_input_accessed": false,
                    }),
                )?;
                return Err(error.into());
            }
            if calibration_freeze.payload.profile != profile {
                let error = "v2 replay CLI profile does not match calibration freeze profile";
                write_json_create_new(
                    &output,
                    &json!({
                        "schema": "scientific-validity-v2-oregonator-holdout-replay-failure-v1",
                        "status": "fail",
                        "profile": profile,
                        "campaign_label": profile.campaign_label(),
                        "error": error,
                        "calibration_checksum_sha256": calibration_freeze.checksum_sha256,
                        "rows": [],
                        "holdout_input_accessed": false,
                    }),
                )?;
                anyhow::bail!(error);
            }
            // The holdout path is deliberately opened only after the immutable
            // calibration authority and requested profile have both verified.
            let rows: Vec<V2GateRow> = read_json(&input)?;
            match replay_v2_oregonator_holdout(&calibration_freeze, rows.clone()) {
                Ok(replay) => {
                    let overall_pass = replay.payload.overall_pass;
                    write_json_create_new(&output, &replay)?;
                    if !overall_pass {
                        anyhow::bail!(
                            "v2 Oregonator holdout replay preserved a non-passing result"
                        );
                    }
                }
                Err(error) => {
                    write_json_create_new(
                        &output,
                        &json!({
                            "schema": "scientific-validity-v2-oregonator-holdout-replay-failure-v1",
                            "status": "fail",
                            "profile": profile,
                            "campaign_label": profile.campaign_label(),
                            "error": error.to_string(),
                            "calibration_checksum_sha256": calibration_freeze.checksum_sha256,
                            "rows": rows,
                        }),
                    )?;
                    return Err(error.into());
                }
            }
        }
        Command::ScientificValidityV2RunCalibration {
            reference_manifest,
            output,
            freeze_output,
        } => {
            preflight_create_new_outputs(&[&output, &freeze_output])?;
            let code_revision = match scientific_validity_v2_compiled_revision() {
                Ok(revision) => revision,
                Err(error) => {
                    write_json_create_new(
                        &output,
                        &json!({
                            "schema": "scientific-validity-v2-calibration-campaign-failure-v1",
                            "status": "failed-preflight",
                            "corpus_version": ScientificCorpusV2::VERSION,
                            "reference_manifest_accessed": false,
                            "records": [],
                            "error": error.to_string(),
                        }),
                    )?;
                    return Err(error.into());
                }
            };
            let specs = ScientificCorpusV2::calibration_specs();
            if specs.len() != 54 {
                anyhow::bail!("ScientificCorpusV2.1 calibration cardinality is not 54");
            }
            let (records, rows, failures, record_set_sha256) =
                run_v2_case_records(&reference_manifest, specs);
            let freeze_admission = if failures == 0 && records.len() == 54 && rows.len() == 54 {
                let artifacts = complete_v2_case_artifacts(&records);
                freeze_scientific_validity_v2_calibration_artifacts(&artifacts)
                    .map_err(|error| error.to_string())
            } else {
                Err("campaign lacks 54 complete bound rows".to_owned())
            };
            let freeze_eligible = freeze_admission.is_ok();
            let freeze_checksum_sha256 = freeze_admission
                .as_ref()
                .ok()
                .map(|freeze| freeze.checksum_sha256.clone());
            let freeze_admission_error = freeze_admission.as_ref().err().cloned();
            let campaign = V2CalibrationCampaignDocument {
                schema: "scientific-validity-v2-calibration-campaign-v1".into(),
                status: if freeze_eligible {
                    "complete-pass".into()
                } else {
                    "complete-nonpassing".into()
                },
                corpus_version: ScientificCorpusV2::VERSION.into(),
                code_revision: code_revision.into(),
                expected_case_count: 54,
                attempted_case_count: records.len(),
                failure_count: failures,
                freeze_eligible,
                freeze_checksum_sha256,
                freeze_admission_error,
                record_set_sha256,
                records,
                rows,
            };
            write_json_create_new(&output, &campaign)?;
            if !freeze_eligible {
                anyhow::bail!(
                    "v2 calibration preserved a non-freeze-eligible 54-case campaign ({failures} execution failures)"
                );
            }
            let freeze = freeze_admission.expect("freeze eligibility checked");
            // The full campaign is published first. A later filesystem race can
            // therefore never leave a freeze without its complete 54-case record.
            write_json_create_new(&freeze_output, &freeze)?;
        }
        Command::ScientificValidityV2RunOregonator {
            profile,
            freeze,
            calibration_campaign,
            reference_manifest,
            output,
        } => {
            // Neither holdout specifications nor the reference path are opened
            // before this immutable calibration authority passes completely.
            let profile = V2GateProfile::from(profile);
            if profile != V2GateProfile::Canonical {
                let error = "the Oregonator producer accepts only the canonical 3-row profile";
                write_json_create_new(
                    &output,
                    &json!({
                        "schema": "scientific-validity-v2-oregonator-campaign-failure-v1",
                        "status": "failed-profile-preflight",
                        "freeze_accessed": false,
                        "calibration_campaign_accessed": false,
                        "holdout_spec_accessed": false,
                        "reference_manifest_accessed": false,
                        "records": [],
                        "error": error,
                    }),
                )?;
                anyhow::bail!(error);
            }
            let calibration_freeze: V2CalibrationFreezeEnvelope = read_json(&freeze)?;
            if let Err(error) = verify_v2_calibration_freeze(&calibration_freeze) {
                write_json_create_new(
                    &output,
                    &json!({
                        "schema": "scientific-validity-v2-oregonator-campaign-failure-v1",
                        "status": "failed-freeze-preflight",
                        "calibration_campaign_accessed": false,
                        "holdout_spec_accessed": false,
                        "reference_manifest_accessed": false,
                        "records": [],
                        "error": error.to_string(),
                    }),
                )?;
                return Err(error.into());
            }
            if calibration_freeze.payload.profile != profile {
                let error = "Oregonator campaign profile differs from the verified freeze";
                write_json_create_new(
                    &output,
                    &json!({
                        "schema": "scientific-validity-v2-oregonator-campaign-failure-v1",
                        "status": "failed-profile-preflight",
                        "calibration_campaign_accessed": false,
                        "holdout_spec_accessed": false,
                        "reference_manifest_accessed": false,
                        "records": [],
                        "error": error,
                    }),
                )?;
                anyhow::bail!(error);
            }
            let code_revision = match scientific_validity_v2_compiled_revision() {
                Ok(revision) => revision,
                Err(error) => {
                    write_json_create_new(
                        &output,
                        &json!({
                        "schema": "scientific-validity-v2-oregonator-campaign-failure-v1",
                        "status": "failed-source-preflight",
                        "calibration_campaign_accessed": false,
                        "holdout_spec_accessed": false,
                            "reference_manifest_accessed": false,
                            "records": [],
                            "error": error.to_string(),
                        }),
                    )?;
                    return Err(error.into());
                }
            };
            let current_binding = scientific_validity_v2_canonical_campaign_binding()?;
            if calibration_freeze.payload.campaign_binding != current_binding {
                let error =
                    "verified freeze campaign binding differs from the current canonical runner";
                write_json_create_new(
                    &output,
                    &json!({
                        "schema": "scientific-validity-v2-oregonator-campaign-failure-v1",
                        "status": "failed-runner-binding-preflight",
                        "calibration_campaign_accessed": false,
                        "holdout_spec_accessed": false,
                        "reference_manifest_accessed": false,
                        "records": [],
                        "error": error,
                    }),
                )?;
                anyhow::bail!(error);
            }
            let campaign: V2CalibrationCampaignDocument = match read_json(&calibration_campaign) {
                Ok(campaign) => campaign,
                Err(error) => {
                    write_json_create_new(
                        &output,
                        &json!({
                            "schema": "scientific-validity-v2-oregonator-campaign-failure-v1",
                            "status": "failed-calibration-campaign-preflight",
                            "calibration_campaign_accessed": true,
                            "holdout_spec_accessed": false,
                            "reference_manifest_accessed": false,
                            "records": [],
                            "error": error.to_string(),
                        }),
                    )?;
                    return Err(error);
                }
            };
            let derived_freeze = match validate_v2_calibration_campaign_document(&campaign) {
                Ok(freeze) => freeze,
                Err(error) => {
                    write_json_create_new(
                        &output,
                        &json!({
                            "schema": "scientific-validity-v2-oregonator-campaign-failure-v1",
                            "status": "failed-calibration-campaign-preflight",
                            "calibration_campaign_accessed": true,
                            "holdout_spec_accessed": false,
                            "reference_manifest_accessed": false,
                            "records": [],
                            "error": error.to_string(),
                        }),
                    )?;
                    return Err(error);
                }
            };
            if derived_freeze != calibration_freeze {
                let error = "calibration freeze differs from the validated complete campaign";
                write_json_create_new(
                    &output,
                    &json!({
                        "schema": "scientific-validity-v2-oregonator-campaign-failure-v1",
                        "status": "failed-calibration-freeze-link-preflight",
                        "calibration_campaign_accessed": true,
                        "holdout_spec_accessed": false,
                        "reference_manifest_accessed": false,
                        "records": [],
                        "error": error,
                    }),
                )?;
                anyhow::bail!(error);
            }
            let specs = ScientificCorpusV2::holdout_specs()
                .into_iter()
                .filter(|spec| spec.family == ScientificFamily::Oregonator)
                .collect::<Vec<_>>();
            if specs.len() != 3 {
                anyhow::bail!("ScientificCorpusV2.1 Oregonator cardinality is not 3");
            }
            let (records, rows, failures, record_set_sha256) =
                run_v2_case_records(&reference_manifest, specs);
            let replay_result = if failures == 0 && rows.len() == 3 {
                let artifacts = complete_v2_case_artifacts(&records);
                replay_scientific_validity_v2_oregonator_artifacts(&calibration_freeze, &artifacts)
                    .map(Some)
                    .map_err(|error| error.to_string())
            } else {
                Ok(None)
            };
            let (replay, replay_error) = match replay_result {
                Ok(replay) => (replay, None),
                Err(error) => (None, Some(error)),
            };
            let replay_pass = replay
                .as_ref()
                .is_some_and(|value| value.payload.overall_pass);
            write_json_create_new(
                &output,
                &json!({
                    "schema": "scientific-validity-v2-oregonator-campaign-v1",
                    "status": if failures == 0 && replay_pass { "complete-pass" } else { "complete-nonpassing" },
                    "corpus_version": ScientificCorpusV2::VERSION,
                    "code_revision": code_revision,
                    "calibration_checksum_sha256": calibration_freeze.checksum_sha256,
                    "expected_case_count": 3,
                    "attempted_case_count": records.len(),
                    "failure_count": failures,
                    "replay_eligible": failures == 0 && rows.len() == 3,
                    "record_set_sha256": record_set_sha256,
                    "records": records,
                    "rows": rows,
                    "replay": replay,
                    "replay_error": replay_error,
                }),
            )?;
            if failures != 0 || !replay_pass {
                anyhow::bail!("v2 Oregonator campaign preserved a non-passing result");
            }
        }
        Command::HomotopyDesignCheck { output } => {
            write_json(&output, &run_homotopy_design_check()?)?;
        }
        Command::HomotopyExperimentScreen { profile, output } => {
            write_json(&output, &run_homotopy_experiment_screen(profile.into())?)?;
        }
        Command::HomotopyOrderPolicyScreen {
            profile,
            threads,
            output,
        } => {
            write_json(
                &output,
                &run_homotopy_order_policy_screen(profile.into(), threads)?,
            )?;
        }
        Command::StageBatchFeasibility { profile, output } => {
            let report = run_stage_batch_feasibility(profile.into())?;
            write_json(&output, &report)?;
        }
        Command::HomotopyRhsTelemetry { profile, output } => {
            write_json(&output, &run_homotopy_rhs_telemetry_screen(profile.into())?)?;
        }
        Command::MatrixFreeCommonWGate { profile, output } => {
            write_json(&output, &run_matrix_free_common_w_gate(profile.into())?)?;
        }
        Command::HomotopyPathController { profile, output } => {
            write_json(&output, &run_path_controller_screen(profile.into())?)?;
        }
        Command::GenericQ1Q2Gate { profile, output } => {
            write_json(&output, &run_g1_transactional_gate(profile.into())?)?;
        }
        Command::GenericQ1Q2Adaptive {
            profile,
            threads,
            output,
        } => {
            write_json(
                &output,
                &run_g1_adaptive_global_error_screen(profile.into(), threads)?,
            )?;
        }
        Command::GenericParallelExponentialGate { profile, output } => {
            write_json(&output, &run_g2_exponential_gate(profile.into())?)?;
        }
        Command::GenericParallelExponentialAdaptive { profile, output } => {
            let profile = match profile {
                CliHomotopyProfile::Smoke => G3FusedAdaptiveProfile::Smoke,
                CliHomotopyProfile::Canonical => G3FusedAdaptiveProfile::Canonical,
            };
            write_json(&output, &run_g3_fused_adaptive_gate(profile)?)?;
        }
        Command::GenericPrefixKernelGate { profile, output } => {
            let profile = match profile {
                CliHomotopyProfile::Smoke => G4PrefixKernelProfile::Smoke,
                CliHomotopyProfile::Canonical => G4PrefixKernelProfile::Canonical,
            };
            write_json(&output, &run_g4_prefix_kernel_gate(profile)?)?;
        }
        Command::GenericRegimeAtlas { profile, output } => {
            let profile = match profile {
                CliHomotopyProfile::Smoke => G4S5B0Profile::Smoke,
                CliHomotopyProfile::Canonical => G4S5B0Profile::Canonical,
            };
            write_json(&output, &run_g4_s5b0_regime_atlas(profile)?)?;
        }
        Command::GenericPolicyRedesignAtlas {
            profile,
            family,
            output,
        } => {
            let profile = match profile {
                CliPolicyRedesignProfile::Calibration => G4S5B0Profile::Calibration128,
                CliPolicyRedesignProfile::Holdout => G4S5B0Profile::Holdout512,
            };
            let report = match family {
                Some(family) => run_g4_s5b0_rjf_only_family(profile, family.into())?,
                None => run_g4_s5b0_rjf_only(profile)?,
            };
            write_json(&output, &report)?;
        }
        Command::GenericPolicyRedesignAttemptTrace {
            profile,
            family,
            output,
        } => {
            let profile = match profile {
                CliPolicyRedesignProfile::Calibration => G4S5B0Profile::Calibration128,
                CliPolicyRedesignProfile::Holdout => G4S5B0Profile::Holdout512,
            };
            let report = match family {
                Some(family) => run_g4_s5b0_rjf_attempt_trace_family(profile, family.into())?,
                None => run_g4_s5b0_rjf_attempt_trace(profile)?,
            };
            write_json(&output, &report)?;
        }
        Command::GenericPolicyRedesignActualPrefix {
            profile,
            family,
            policy,
            output,
        } => {
            let profile = match profile {
                CliPolicyRedesignProfile::Calibration => G4S5B0Profile::Calibration128,
                CliPolicyRedesignProfile::Holdout => G4S5B0Profile::Holdout512,
            };
            let report =
                run_g4_s5b0_actual_level1_prefix_family(profile, family.into(), policy.into())?;
            write_json(&output, &report)?;
        }
        Command::GenericPolicyRedesignLevel2Prefix {
            profile,
            family,
            policy,
            output,
        } => {
            let report = run_g4_s5b0_actual_level2_prefix_family(
                profile.into(),
                family.into(),
                policy.into(),
            )?;
            write_json(&output, &report)?;
        }
        Command::GenericStageGrowthSafetyAudit {
            profile,
            family,
            output,
        } => {
            let report =
                run_g4_s5b0_stage_growth_safety_audit_family(profile.into(), family.into())?;
            write_json(&output, &report)?;
        }
        Command::GenericEnforcedPrefixBudget {
            profile,
            family,
            output,
        } => {
            let report = run_g4_s5b0_enforced_prefix_budget_family(profile.into(), family.into())?;
            write_json(&output, &report)?;
        }
        Command::GenericFrozenFullEShadow {
            profile,
            family,
            output,
        } => {
            let report = run_g4_s5b0_frozen_full_e_shadow_family(profile.into(), family.into())?;
            write_json(&output, &report)?;
        }
        Command::A1TwoArmReceiptCell {
            family,
            arm,
            repository,
            pull_request,
            scientific_execution_head_sha,
            scientific_execution_head_tree,
            base_sha,
            base_tree,
            tested_execution_merge_sha,
            tested_execution_merge_tree,
            execution_workflow_run_id,
            execution_workflow_run_attempt,
            rust_version,
            cargo_version,
            output,
        } => {
            let identity = A1ScientificExecutionIdentity {
                repository,
                pull_request,
                scientific_execution_head_sha,
                scientific_execution_head_tree,
                base_sha,
                base_tree,
                tested_execution_merge_sha,
                tested_execution_merge_tree,
                execution_workflow_run_id,
                execution_workflow_run_attempt,
                rust_version,
                cargo_version,
            };
            let cell = run_a1_two_arm_receipt_cell(identity, family.into(), arm.into())?;
            write_json(&output, &cell)?;
        }
        Command::GenericV37ContinuationTransaction {
            profile,
            family,
            output,
        } => {
            let report =
                run_g4_s5b0_v37_continuation_transaction_family(profile.into(), family.into())?;
            write_v37_continuation_transaction_report(&output, &report)?;
        }
        Command::GenericFrozenFullEShadowEconomics { profile, output } => {
            let report = run_g4_s5b0_frozen_full_e_shadow_economics(profile.into())?;
            write_json(&output, &report)?;
        }
        Command::GenericEarlyDefectAttemptGeometry { profile, output } => {
            let profile = match profile {
                CliHomotopyProfile::Smoke => G4S5B3Profile::Smoke,
                CliHomotopyProfile::Canonical => G4S5B3Profile::Canonical,
            };
            write_json(&output, &run_g4_s5b3_attempt_geometry(profile)?)?;
        }
        Command::GenericToleranceScaledEarlyDefect { profile, output } => {
            let profile = match profile {
                CliHomotopyProfile::Smoke => G4S5B3Profile::Smoke,
                CliHomotopyProfile::Canonical => G4S5B3Profile::Canonical,
            };
            write_json(&output, &run_p1_00_tolerance_scaled_early_defect(profile)?)?;
        }
        Command::NativeIntegratorGates { output } => {
            let report: NativeIntegratorGateReport = run_native_integrator_gates()?;
            write_json(&output, &report)?;
        }
        Command::AdaptiveGlobalError {
            profile,
            threads,
            output,
        } => {
            write_json(
                &output,
                &run_adaptive_global_error_screen(profile.into(), threads)?,
            )?;
        }
        Command::GlobalErrorPareto {
            profile,
            threads,
            output,
        } => {
            write_json(
                &output,
                &run_global_error_pareto_screen(profile.into(), threads)?,
            )?;
        }
        Command::UnifiedCandidateScreen {
            profile,
            threads,
            full,
            output,
        } => {
            write_json(
                &output,
                &run_unified_candidate_document(profile, threads, full)?,
            )?;
        }
        Command::Trace {
            kind,
            dimension,
            steps,
            stages,
            seed,
            stiffness,
            nonnormality,
            output,
        } => {
            let trace = generate_trace(&SequenceConfig {
                kind: kind.into(),
                dimension,
                steps,
                stages,
                seed,
                stiffness,
                nonnormality,
            })?;
            write_json(&output, &TraceDocument::from_trace(&trace))?;
        }
        Command::Benchmark {
            trace,
            output,
            repetitions,
            warmups,
            seed,
            restart,
            recycle_dim,
            operator_budget,
            rtol,
            atol,
            preconditioner,
            zero_guess,
        } => {
            let document: TraceDocument = serde_json::from_slice(
                &fs::read(&trace).with_context(|| format!("reading {}", trace.display()))?,
            )?;
            let trace = document.into_trace()?;
            let plan = BenchmarkPlan {
                cells: strict_cells(),
                repetitions,
                warmups,
                seed,
            };
            let preconditioner = preconditioner.into();
            let comparison = run_comparison(&trace, &plan, |solver| FairSolveConfig {
                solver,
                rtol,
                atol,
                restart,
                recycle_dim,
                hard_operator_budget: operator_budget,
                preconditioner,
                use_previous_oracle_guess: !zero_guess,
            })?;
            let summary = summarize_comparison(&comparison);
            let failures = comparison.runs.iter().map(|run| run.failures).sum();
            write_json(
                &output,
                &BenchmarkDocument {
                    schema: "rodas5p-rust-fair-ab-v1",
                    trace_id: trace.trace_id,
                    failures,
                    plan,
                    summary,
                    comparison,
                },
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod scientific_validity_v2_producer_tests {
    use super::*;

    #[test]
    fn calibration_record_loop_attempts_all_fifty_four_cases_after_individual_failures() {
        let missing = std::env::temp_dir().join(format!(
            "vigilode-missing-v2-reference-manifest-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&missing);
        let (records, rows, failures, digest) =
            run_v2_case_records(&missing, ScientificCorpusV2::calibration_specs());
        assert_eq!(records.len(), 54);
        assert_eq!(failures, 54);
        assert!(rows.is_empty());
        assert_eq!(digest.len(), 64);
        assert!(
            records
                .iter()
                .all(|record| matches!(record, V2CampaignCaseRecord::Failed { .. }))
        );
        assert_eq!(
            records
                .iter()
                .filter_map(|record| match record {
                    V2CampaignCaseRecord::Failed { spec, .. } => Some(spec.id.as_str()),
                    V2CampaignCaseRecord::Complete { .. } => None,
                })
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            54
        );
    }
}

#[cfg(test)]
mod unified_assessment_tests {
    use super::*;

    fn summary(
        solver: SolverKind,
        lifetime: RecycleLifetime,
        wall: f64,
        operator: f64,
    ) -> rodas5p_fair_ab::SummaryRow {
        rodas5p_fair_ab::SummaryRow {
            solver,
            lifetime,
            repetitions: 3,
            failures: 0,
            wall_median_seconds: wall,
            wall_q25_seconds: wall,
            wall_q75_seconds: wall,
            operator_total_median: operator,
            maximum_relative_solution_error: 1.0e-10,
        }
    }

    fn suite(gcrodr_wall: f64) -> UnifiedLinearSuite {
        UnifiedLinearSuite {
            kind: SequenceKind::Fixed,
            trace_id: "trace".into(),
            failures: 0,
            plan: BenchmarkPlan {
                cells: strict_cells(),
                repetitions: 3,
                warmups: 1,
                seed: 1,
            },
            summary: vec![
                summary(SolverKind::Gmres, RecycleLifetime::Off, 1.0, 100.0),
                summary(
                    SolverKind::Gcrodr,
                    RecycleLifetime::Persistent,
                    gcrodr_wall,
                    70.0,
                ),
            ],
            comparison: None,
        }
    }

    #[test]
    fn linear_candidate_id_covers_every_strict_cell() {
        let ids: Vec<_> = strict_cells()
            .into_iter()
            .map(|cell| linear_candidate_id(cell.solver, cell.lifetime))
            .collect();
        assert!(ids.contains(&"sequential-lgmres-stage".to_string()));
        assert!(ids.contains(&"sequential-lgmres-persistent".to_string()));
        assert!(ids.contains(&"sequential-gcrodr-persistent".to_string()));
    }

    #[test]
    fn tier_l_promotion_requires_the_locked_fifteen_percent_wall_speedup() {
        let promoted = assess_linear_candidates(&[suite(0.8)]);
        let row = promoted
            .iter()
            .find(|row| row.candidate_id == "sequential-gcrodr-persistent")
            .unwrap();
        assert_eq!(row.verdict, UnifiedJointVerdict::Promote);

        let held = assess_linear_candidates(&[suite(0.9)]);
        let row = held
            .iter()
            .find(|row| row.candidate_id == "sequential-gcrodr-persistent")
            .unwrap();
        assert_eq!(row.verdict, UnifiedJointVerdict::Hold);
    }

    #[test]
    fn nonlinear_batch_depth_advantage_does_not_replace_wall_speedup() {
        let (verdict, blockers) = nonlinear_performance_verdict(
            false,
            6,
            6,
            0,
            Some(2.0),
            Some(3.0),
            Some(5.0),
            Some(0.25),
        );
        assert_eq!(verdict, UnifiedJointVerdict::Hold);
        assert!(
            blockers.iter().any(|blocker| {
                blocker.contains("median nonlinear candidate wall speedup below")
            })
        );
    }

    #[test]
    fn v37_failed_hard_gate_refuses_to_create_authority_output() {
        let mut report = run_g4_s5b0_v37_continuation_transaction_family(
            G4S5B0Profile::StageGrowthCalibration96,
            G4S5B0Family::RobertsonRamped,
        )
        .unwrap();
        report.hard_gates.passed = false;
        let mut output = std::env::temp_dir();
        output.push(format!(
            "rodas5p-v37-fail-closed-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&output);

        let error = write_v37_continuation_transaction_report(&output, &report).unwrap_err();
        assert!(error.to_string().contains("hard gates failed"));
        assert!(!output.exists());
    }

    #[test]
    fn document_status_exposes_uncertified_reference_rows() {
        assert_eq!(
            unified_document_status(0, 0, 4),
            "complete-with-uncertified"
        );
        assert_eq!(unified_document_status(0, 0, 0), "complete");
        assert_eq!(unified_document_status(1, 0, 0), "complete-with-failures");
    }
}
