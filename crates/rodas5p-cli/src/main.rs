use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

mod r3_campaigns;
mod r4_studies;
mod stiff_benchmark;
use clap::{Parser, Subcommand, ValueEnum};
use rodas5p_core::{load_rodas5p_coefficients, sha256_hex};
use rodas5p_fair_ab::{
    ArmIdentity, BenchmarkCell, BenchmarkPlan, FairSolveConfig, GlobalErrorParetoProfile,
    PairedTimingDecision, PairedTimingEvidence, PairedTimingProtocol, PairedTimingReceipt,
    PairedWorkload, PreconditionerKind, RecycleLifetime, ScientificValidityV2CaseArtifact,
    SequenceConfig, SequenceKind, SessionRecord, SolverKind, TraceDocument,
    freeze_scientific_validity_v2_calibration_artifacts, generate_trace,
    load_numerical_reference_v2, measure_paired_session,
    replay_scientific_validity_v2_oregonator_artifacts, run_adaptive_global_error_screen,
    run_comparison, run_g1_adaptive_global_error_screen, run_global_error_pareto_screen,
    run_scientific_validity_v2_case, run_trace, scientific_validity_v2_canonical_campaign_binding,
    scientific_validity_v2_compiled_revision, summarize_comparison,
    validate_scientific_validity_v2_case_artifact,
};
use rodas5p_integrators::{
    A1ScientificExecutionIdentity, CandidateCatalog, CandidateFamily, CandidateStatus,
    ComparatorFidelity, G1TransactionalGateProfile, G2ExponentialGateProfile,
    G3FusedAdaptiveProfile, G4PrefixKernelProfile, G4S5B0Family, G4S5B0PrefixProbePolicy,
    G4S5B0Profile, G4S5B0V37ContinuationTransactionReport, G4S5B3Profile,
    HomotopyExperimentProfile, HomotopyRhsTelemetryProfile, MatrixFreeCommonWProfile,
    NativeIntegratorGateReport, PathControllerProfile, ScientificCaseSpec, ScientificCorpusV2,
    ScientificFamily, StageBatchFeasibilityProfile, UnifiedNonlinearScreen,
    UnifiedScientificGateReport, UnifiedScreenProfile, V2CalibrationFreezeEnvelope, V2GateProfile,
    V2GateRow, freeze_v2_calibration, replay_v2_oregonator_holdout, run_a1_two_arm_receipt_cell,
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
        /// JSON map from Tier-L candidate id to its paired timing evidence
        /// (`paired-timing-campaign` output). Without it every wall
        /// criterion is not evaluated.
        #[arg(long)]
        paired_timing_evidence: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Measure one paired-timing session of a Tier-L candidate against
    /// GMRES/OFF in this process (run by `paired-timing-campaign`).
    #[command(name = "paired-timing-session")]
    PairedTimingSession {
        #[arg(long)]
        campaign_id: String,
        #[arg(long)]
        session: u32,
        #[arg(long)]
        candidate: String,
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Smoke)]
        profile: CliHomotopyProfile,
        #[arg(long, default_value_t = 20_261_001)]
        seed: u64,
        /// Batch per case fixed by the first session (JSON map).
        #[arg(long)]
        batches: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run a paired-timing campaign: each session in its own process, then
    /// merge the raw session records into a receipt and assess it.
    #[command(name = "paired-timing-campaign")]
    PairedTimingCampaign {
        #[arg(long)]
        candidate: String,
        #[arg(long, value_enum, default_value_t = CliHomotopyProfile::Smoke)]
        profile: CliHomotopyProfile,
        #[arg(long, default_value_t = 6)]
        sessions: u32,
        #[arg(long, default_value_t = 20_261_001)]
        seed: u64,
        #[arg(long)]
        output: PathBuf,
    },
    /// Coverage study of the paired timing interval and decision under the
    /// preregistered dependence and missingness models (re-audit R3,
    /// STAT-DEV-04). Refuses to overwrite its output.
    #[command(name = "paired-timing-coverage-study")]
    PairedTimingCoverageStudy {
        #[arg(long, default_value_t = rodas5p_fair_ab::COVERAGE_STUDY_REPLICATIONS)]
        replications: usize,
        #[arg(long, default_value_t = 20_261_001)]
        seed: u64,
        #[arg(long, default_value_t = 4)]
        threads: usize,
        /// Run only these scenario ids (default: the whole grid).
        #[arg(long)]
        scenario: Vec<String>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Coverage and power of the exact session-median interval on its
    /// preregistered grid (re-audit R4, R4-STAT-DEV-04). Refuses to
    /// overwrite its output.
    #[command(name = "session-median-coverage-study")]
    SessionMedianCoverageStudy {
        #[arg(long, default_value_t = 10_000)]
        replications: usize,
        #[arg(long, default_value_t = 20_261_013)]
        seed: u64,
        #[arg(long, default_value_t = 4)]
        threads: usize,
        /// Run only these scenario ids (default: the whole grid).
        #[arg(long)]
        scenario: Vec<String>,
        #[arg(long)]
        output: PathBuf,
    },
    /// A deterministic R4 research study (`laguerre`, `polynomial-regimes`,
    /// `homotopy-cost`); work counters, enclosures and accuracy decide, wall
    /// seconds are diagnostics. Refuses to overwrite its output.
    #[command(name = "r4-study")]
    /// Work-precision benchmark of RODAS5P against the in-repository BDF
    /// and Radau IIA integrators on four standard stiff problems. Refuses
    /// to overwrite its output.
    #[command(name = "stiff-benchmark")]
    StiffBenchmark {
        #[arg(long, default_value_t = 7)]
        repetitions: usize,
        #[arg(long, default_value_t = 1)]
        warmups: usize,
        /// Comma-separated problem ids (default: the four of the first run).
        #[arg(long, value_delimiter = ',')]
        problems: Option<Vec<String>>,
        /// Comma-separated arms (default: all five).
        #[arg(long, value_delimiter = ',')]
        arms: Option<Vec<String>>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run one stiff-benchmark arm on one problem, `repetitions` times, and
    /// print its work: the workload of the RODAS5P profiling node.
    #[command(name = "stiff-profile-run")]
    StiffProfileRun {
        #[arg(long)]
        problem: String,
        #[arg(long, default_value = "rodas5p")]
        arm: String,
        #[arg(long)]
        rtol: f64,
        #[arg(long, default_value_t = 1)]
        repetitions: usize,
    },
    R4Study {
        #[arg(long)]
        study: String,
        #[arg(long, default_value_t = 20)]
        actions: usize,
        #[arg(long)]
        output: PathBuf,
    },
    /// One paired-timing session of an R3 matched-accuracy arm (run by
    /// `r3-campaign`).
    #[command(name = "r3-campaign-session")]
    R3CampaignSession {
        #[arg(long, value_enum)]
        study: r3_campaigns::Study,
        #[arg(long)]
        arm: String,
        #[arg(long)]
        campaign_id: String,
        #[arg(long)]
        session: u32,
        #[arg(long, default_value_t = 20_261_001)]
        seed: u64,
        #[arg(long)]
        batches: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    /// R3 matched-accuracy paired timing (HOM-06, POLY-03): every candidate
    /// arm against the study's reference, one process per session.
    /// Refuses to overwrite its output.
    #[command(name = "r3-campaign")]
    R3Campaign {
        #[arg(long, value_enum)]
        study: r3_campaigns::Study,
        #[arg(long, default_value_t = 6)]
        sessions: u32,
        #[arg(long, default_value_t = 20_261_001)]
        seed: u64,
        #[arg(long)]
        output: PathBuf,
    },
    /// Untimed accuracy, work and memory of one R3 arm (run by
    /// `r3-campaign-verify`).
    #[command(name = "r3-verify-arm")]
    R3VerifyArm {
        #[arg(long, value_enum)]
        study: r3_campaigns::Study,
        #[arg(long)]
        arm: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Untimed verification of every arm of an R3 study, one process per
    /// arm. Refuses to overwrite its output.
    #[command(name = "r3-campaign-verify")]
    R3CampaignVerify {
        #[arg(long, value_enum)]
        study: r3_campaigns::Study,
        #[arg(long)]
        output: PathBuf,
    },
    /// The statistical authority of every arm of a published paired-timing
    /// campaign (re-audit R4, R4-STAT-DEV-01): the verified diagnostic
    /// decision, unchanged, beside the authority the compiled study registry
    /// assigns to its design and the decision a consumer may act on.
    /// Refuses to overwrite its output.
    #[command(name = "timing-authority")]
    TimingAuthority {
        #[arg(long)]
        campaign: PathBuf,
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
    /// Replayed N=128 calibration decisions; refused on holdout profiles.
    #[value(alias = "frozen-k1")]
    ReplayedK1,
    /// Causal k=1 comparator on the observed error-drop feature.
    CausalK1,
    K3,
}

impl From<CliPolicyRedesignPrefixPolicy> for G4S5B0PrefixProbePolicy {
    fn from(value: CliPolicyRedesignPrefixPolicy) -> Self {
        match value {
            CliPolicyRedesignPrefixPolicy::ReplayedK1 => Self::ReplayedK1Table,
            CliPolicyRedesignPrefixPolicy::CausalK1 => Self::CausalK1,
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
    /// No relative-performance verdict is admissible: the reference is a
    /// reference-implementation-only comparator (audit F-052/F-056).
    NotEvaluated,
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
    reference_fidelity: ComparatorFidelity,
    verdict: UnifiedJointVerdict,
    blockers: Vec<String>,
    /// Criteria the gate refused to evaluate; never Promote/Hold evidence.
    not_evaluated: Vec<String>,
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
    reference_fidelity: ComparatorFidelity,
    verdict: UnifiedJointVerdict,
    blockers: Vec<String>,
    not_evaluated: Vec<String>,
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
    not_evaluated: Vec<String>,
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

/// Tier-L reference arm (GMRES/OFF) is a production RODAS5P linear solver.
const TIER_L_REFERENCE_FIDELITY: ComparatorFidelity = ComparatorFidelity::Production;

const REFERENCE_ONLY_NOT_EVALUATED: &str =
    "relative performance not evaluated: reference is a reference-implementation-only comparator";

const WALL_NOT_EVALUATED: &str = "wall-time criterion not evaluated: no authoritative paired timing assessment (A/A control, at least six independent sessions) was supplied; repeated medians are recorded only";

const WALL_REJECTED: &str = "wall-time criterion not evaluated: the paired timing record is not a confirmatory, self-consistent assessment";
const WALL_INCONCLUSIVE: &str = "wall-time criterion not evaluated: the paired timing interval is inconclusive or its A/A control is not authoritative";
const WALL_AUTHORITY_HOLD: &str = "wall-time criterion not evaluated: the paired timing design has no admissible statistical authority (STATISTICAL_AUTHORITY_HOLD or NOT_EVALUATED); the diagnostic decision is recorded only";

/// Wall time decides Promote/Hold only through a paired timing assessment
/// (`rodas5p_fair_ab::assess_paired_timing`): its gate decision is Promote
/// or Block only when the A/A control is authoritative and at least six
/// independent sessions were measured. A median of a few repetitions, with
/// or without warmups, carries no dispersion estimate and never decides
/// (audit F-053; audit 2026-09-30, B-03).
enum WallCriterion {
    Passed,
    Failed,
    NotEvaluated(&'static str),
}

const WALL_WRONG_ARMS: &str = "wall-time criterion not evaluated: the paired timing evidence does not time this candidate against GMRES/OFF";

/// The Tier-L reference arm of every paired timing.
const TIER_L_TIMING_REFERENCE: &str = "sequential-gmres-off";

const WALL_WRONG_IDENTITY: &str = "wall-time criterion not evaluated: the paired timing evidence was measured on another workload or executable";

/// What paired timing evidence must have measured to count for this
/// document: the workload profile and, when known, this executable.
#[derive(Clone, Debug)]
struct TimingExpectation {
    workload_prefix: String,
    executable_sha256: Option<String>,
    /// A counterfactual study registry for unit tests of the post-authority
    /// paths; absent from every non-test build, where the compiled registry
    /// alone decides (re-audit R4, R4-STAT-DEV-01).
    #[cfg(test)]
    authorities: Option<Vec<rodas5p_fair_ab::TimingAuthority>>,
}

impl TimingExpectation {
    fn for_profile(profile: CliHomotopyProfile) -> Result<Self> {
        Ok(Self {
            workload_prefix: format!("tier-l-{}-", cli_profile_name(profile)),
            executable_sha256: Some(sha256_hex(&fs::read(std::env::current_exe()?)?)),
            #[cfg(test)]
            authorities: None,
        })
    }
}

fn cli_profile_name(profile: CliHomotopyProfile) -> &'static str {
    match profile {
        CliHomotopyProfile::Smoke => "smoke",
        CliHomotopyProfile::Canonical => "canonical",
    }
}

fn wall_criterion(
    candidate_id: &str,
    paired: Option<&PairedTimingEvidence>,
    expected: &TimingExpectation,
) -> WallCriterion {
    let Some(evidence) = paired else {
        return WallCriterion::NotEvaluated(WALL_NOT_EVALUATED);
    };
    let receipt = &evidence.receipt;
    if receipt.candidate.arm_id != candidate_id
        || receipt.reference.arm_id != TIER_L_TIMING_REFERENCE
    {
        return WallCriterion::NotEvaluated(WALL_WRONG_ARMS);
    }
    // Smoke-profile or other-binary evidence never gates this document
    // (re-audit R3 review).
    let identity_matches = [&receipt.candidate, &receipt.reference].iter().all(|arm| {
        arm.workload_id.starts_with(&expected.workload_prefix)
            && expected
                .executable_sha256
                .as_ref()
                .is_none_or(|hash| &arm.executable_sha256 == hash)
    });
    if !identity_matches {
        return WallCriterion::NotEvaluated(WALL_WRONG_IDENTITY);
    }
    // The gate is recomputed from the raw receipt: the assessment must
    // equal its recomputation from the receipt's session records, the
    // sessions must be distinct processes of one campaign, and a receipt
    // with any failed case never gates (re-audit R3, STAT-DEV-02; R2,
    // R2-STAT-03).
    // Integrity first, then the statistical authority of the design: a
    // verified Promote of a design whose coverage study failed is recorded,
    // never acted on (re-audit R4, R4-STAT-DEV-01).
    #[cfg(test)]
    let decision = match &expected.authorities {
        Some(registry) => evidence
            .hypothetical_decision_in(registry)
            .map(|decision| (decision.diagnostic, decision.would_admit)),
        None => evidence
            .admissible_decision()
            .map(|decision| (decision.diagnostic(), decision.admissible())),
    };
    #[cfg(not(test))]
    let decision = evidence
        .admissible_decision()
        .map(|decision| (decision.diagnostic(), decision.admissible()));
    match decision {
        Err(_) => WallCriterion::NotEvaluated(WALL_REJECTED),
        Ok((diagnostic, admissible)) => match admissible {
            None if diagnostic == PairedTimingDecision::Inconclusive => {
                WallCriterion::NotEvaluated(WALL_INCONCLUSIVE)
            }
            None => WallCriterion::NotEvaluated(WALL_AUTHORITY_HOLD),
            Some(PairedTimingDecision::Promote) => WallCriterion::Passed,
            Some(PairedTimingDecision::Block) => WallCriterion::Failed,
            Some(PairedTimingDecision::Inconclusive) => {
                WallCriterion::NotEvaluated(WALL_INCONCLUSIVE)
            }
        },
    }
}

/// Per arm of a campaign JSON (`{"evidence": {arm: PairedTimingEvidence}}`):
/// the published numeric summary is read, never rewritten.
fn timing_authority_report(campaign: &serde_json::Value) -> Result<serde_json::Value> {
    let evidence = campaign["evidence"]
        .as_object()
        .context("campaign has no evidence object")?;
    let mut arms = BTreeMap::new();
    for (arm, value) in evidence {
        let row = match serde_json::from_value::<PairedTimingEvidence>(value.clone()) {
            Err(error) => json!({ "error": format!("not paired timing evidence: {error}") }),
            Ok(evidence) => match evidence.admissible_decision() {
                Err(error) => json!({ "error": format!("not verified: {error}") }),
                Ok(decision) => json!({
                    "diagnostic_decision": decision.diagnostic(),
                    "design": decision.design(),
                    "authority": decision.authority(),
                    "studies": decision.studies(),
                    "admissible_decision": decision.admissible(),
                    "reason": decision.reason(),
                    "speedup_point": evidence.assessment.corpus.point,
                    "speedup_lower": evidence.assessment.corpus.lower,
                    "speedup_upper": evidence.assessment.corpus.upper,
                }),
            },
        };
        arms.insert(arm.clone(), row);
    }
    Ok(json!({
        "schema": "vigilode-timing-authority-report-v1",
        "study": campaign["study"],
        "arms": arms,
    }))
}

/// Parse a Tier-L candidate id back to its strict cell.
fn tier_l_cell(candidate_id: &str) -> Result<BenchmarkCell> {
    strict_cells()
        .into_iter()
        .find(|cell| linear_candidate_id(cell.solver, cell.lifetime) == candidate_id)
        .with_context(|| format!("unknown Tier-L candidate {candidate_id}"))
}

fn tier_l_solve_config(solver: SolverKind) -> FairSolveConfig {
    FairSolveConfig {
        solver,
        rtol: 1e-9,
        atol: 1e-12,
        restart: 20,
        recycle_dim: 6,
        hard_operator_budget: 2_000,
        preconditioner: PreconditionerKind::None,
        use_previous_oracle_guess: true,
    }
}

/// One session of the Tier-L paired timing: each unified linear trace is a
/// case, the candidate cell against GMRES/OFF, with its A/A control. A
/// trace with a failed solve is a session failure, not a fast sample.
fn run_tier_l_paired_session(
    campaign_id: &str,
    session: u32,
    candidate_id: &str,
    profile: CliHomotopyProfile,
    seed: u64,
    batches: &BTreeMap<String, usize>,
) -> Result<SessionRecord> {
    let cell = tier_l_cell(candidate_id)?;
    let reference = tier_l_cell(TIER_L_TIMING_REFERENCE)?;
    let traces = unified_linear_configs(profile)
        .iter()
        .map(generate_trace)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let arm = |cell: BenchmarkCell, trace: &rodas5p_fair_ab::LinearSystemTrace| {
        let config = tier_l_solve_config(cell.solver);
        let trace = trace.clone();
        move || -> rodas5p_fair_ab::FairResult<()> {
            let run = run_trace(&trace, &config, cell.lifetime, 0)?;
            if run.failures > 0 {
                return Err(rodas5p_fair_ab::FairError::Invalid(format!(
                    "{} failed solve(s) on {}",
                    run.failures, trace.trace_id
                )));
            }
            Ok(())
        }
    };
    let mut workloads = traces
        .iter()
        .map(|trace| PairedWorkload {
            case_id: trace.trace_id.clone(),
            candidate: Box::new(arm(cell, trace)),
            reference: Box::new(arm(reference, trace)),
        })
        .collect::<Vec<_>>();
    Ok(measure_paired_session(
        campaign_id,
        session,
        &PairedTimingProtocol::authoritative(seed),
        &mut workloads,
        batches,
    )?)
}

/// Run each session in a fresh process of this executable, the first to
/// calibrate the batch per case, merge the records into a receipt, and
/// assess it.
fn run_tier_l_paired_campaign(
    candidate_id: &str,
    profile: CliHomotopyProfile,
    sessions: u32,
    seed: u64,
    output: &Path,
) -> Result<PairedTimingEvidence> {
    tier_l_cell(candidate_id)?;
    let executable = std::env::current_exe()?;
    let executable_sha256 = sha256_hex(&fs::read(&executable)?);
    let campaign_id = format!(
        "tier-l-{candidate_id}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    let directory = output.with_extension("sessions");
    fs::create_dir_all(&directory)?;
    let profile_arg = cli_profile_name(profile);
    let mut records = Vec::new();
    let mut failed_sessions = Vec::new();
    let batches_path = directory.join("batches.json");
    for session in 0..sessions {
        let record_path = directory.join(format!("session-{session}.json"));
        let mut command = std::process::Command::new(&executable);
        command
            .arg("paired-timing-session")
            .args(["--campaign-id", &campaign_id])
            .args(["--session", &session.to_string()])
            .args(["--candidate", candidate_id])
            .args(["--profile", profile_arg])
            .args(["--seed", &seed.to_string()])
            .arg("--output")
            .arg(&record_path);
        if session > 0 && batches_path.exists() {
            command.arg("--batches").arg(&batches_path);
        }
        let status = command.status()?;
        let record = if status.success() {
            fs::read(&record_path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<SessionRecord>(&bytes).ok())
        } else {
            None
        };
        // A session that produced no record is retained as a failure; the
        // receipt then never gates (re-audit R3 review).
        let Some(record) = record else {
            failed_sessions.push(rodas5p_fair_ab::SessionFailure {
                session,
                case_id: "*".into(),
                message: format!("session process exited with {status} and no readable record"),
            });
            continue;
        };
        if session == 0 {
            let batches = rodas5p_fair_ab::session_batches(&record);
            write_json(&batches_path, &batches)?;
        }
        records.push(record);
    }
    let workload_id = format!("tier-l-{profile_arg}-seed-{seed}");
    let receipt = PairedTimingReceipt::from_sessions_with_failures(
        &campaign_id,
        ArmIdentity {
            arm_id: candidate_id.to_owned(),
            executable_sha256: executable_sha256.clone(),
            workload_id: workload_id.clone(),
        },
        ArmIdentity {
            arm_id: TIER_L_TIMING_REFERENCE.to_owned(),
            executable_sha256,
            workload_id,
        },
        PairedTimingProtocol::authoritative(seed),
        records,
        failed_sessions,
    )?;
    Ok(PairedTimingEvidence::from_receipt(receipt)?)
}

/// The Tier-N nonlinear screen times each case with a single `Instant`
/// sample, so its wall ratios are never admissible gate evidence.
const TIER_N_WALL_TIMING_ADMISSIBLE: bool = false;

/// Tier-L candidates without paired timing evidence: every wall criterion
/// is NotEvaluated.
#[cfg(test)]
fn assess_linear_candidates(
    suites: &[UnifiedLinearSuite],
) -> Vec<UnifiedLinearCandidateAssessment> {
    assess_linear_candidates_against(
        suites,
        TIER_L_REFERENCE_FIDELITY,
        &BTreeMap::new(),
        &TimingExpectation {
            workload_prefix: "test".into(),
            executable_sha256: None,
            authorities: None,
        },
    )
}

/// `paired_timing` maps a candidate id to its paired assessment against
/// GMRES/OFF.
fn assess_linear_candidates_against(
    suites: &[UnifiedLinearSuite],
    reference_fidelity: ComparatorFidelity,
    paired_timing: &BTreeMap<String, PairedTimingEvidence>,
    expected: &TimingExpectation,
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
            let relative_admissible = reference_fidelity.admits_relative_performance_reading();
            let candidate_id = linear_candidate_id(cell.solver, cell.lifetime);
            let mut not_evaluated = Vec::new();
            if !is_reference && !relative_admissible {
                not_evaluated.push(REFERENCE_ONLY_NOT_EVALUATED.to_string());
            }
            if !is_reference && relative_admissible {
                match wall_criterion(&candidate_id, paired_timing.get(&candidate_id), expected) {
                    WallCriterion::Passed => {}
                    WallCriterion::Failed => blockers.push(format!(
                        "paired Tier-L wall-speedup interval lies below {:.2}x",
                        TIER_L_REQUIRED_WALL_SPEEDUP
                    )),
                    WallCriterion::NotEvaluated(reason) => not_evaluated.push(reason.to_string()),
                }
            }
            if !is_reference
                && relative_admissible
                && operator_ratio.is_some_and(|ratio| ratio > 1.0)
            {
                blockers.push("median Tier-L operator work exceeds GMRES/OFF".into());
            }
            let verdict = tier_verdict(is_reference, &blockers, &not_evaluated);
            UnifiedLinearCandidateAssessment {
                candidate_id,
                solver: cell.solver,
                lifetime: cell.lifetime,
                suites: represented_suites,
                failures,
                maximum_relative_solution_error,
                median_wall_ratio_to_gmres_off: wall_ratio,
                median_operator_ratio_to_gmres_off: operator_ratio,
                median_wall_speedup: wall_speedup,
                required_wall_speedup: TIER_L_REQUIRED_WALL_SPEEDUP,
                reference_fidelity,
                verdict,
                blockers,
                not_evaluated,
            }
        })
        .collect()
}

/// Tier verdict: intrinsic blockers hold, refused criteria are NotEvaluated,
/// and only a fully evaluated, blocker-free candidate is promoted.
fn tier_verdict(
    is_reference: bool,
    blockers: &[String],
    not_evaluated: &[String],
) -> UnifiedJointVerdict {
    if is_reference {
        UnifiedJointVerdict::Reference
    } else if !blockers.is_empty() {
        UnifiedJointVerdict::Hold
    } else if !not_evaluated.is_empty() {
        UnifiedJointVerdict::NotEvaluated
    } else {
        UnifiedJointVerdict::Promote
    }
}

#[allow(clippy::too_many_arguments)]
fn nonlinear_performance_verdict(
    is_reference: bool,
    reference_fidelity: ComparatorFidelity,
    wall_timing_admissible: bool,
    represented_cases: usize,
    expected_cases: usize,
    failures: usize,
    compute_ratio: Option<f64>,
    rhs_ratio: Option<f64>,
    jvp_ratio: Option<f64>,
    _batch_depth_ratio: Option<f64>,
) -> (UnifiedJointVerdict, Vec<String>, Vec<String>) {
    if is_reference {
        return (UnifiedJointVerdict::Reference, Vec::new(), Vec::new());
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
    if !reference_fidelity.admits_relative_performance_reading() {
        let not_evaluated = vec![REFERENCE_ONLY_NOT_EVALUATED.to_string()];
        let verdict = tier_verdict(false, &blockers, &not_evaluated);
        return (verdict, blockers, not_evaluated);
    }
    if !wall_timing_admissible {
        // The RHS/JVP work blockers apply only when wall speedup fails, so
        // without admissible wall evidence none of the relative criteria
        // can decide.
        let not_evaluated = vec![WALL_NOT_EVALUATED.to_string()];
        let verdict = tier_verdict(false, &blockers, &not_evaluated);
        return (verdict, blockers, not_evaluated);
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
    let verdict = tier_verdict(false, &blockers, &[]);
    (verdict, blockers, Vec::new())
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
    let reference_fidelity = catalog
        .entries()
        .iter()
        .find(|candidate| candidate.id() == "sequential-direct-off")
        .map_or(
            ComparatorFidelity::ReferenceImplementationOnly,
            |candidate| candidate.comparator_fidelity(),
        );
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
            let (verdict, blockers, not_evaluated) = nonlinear_performance_verdict(
                candidate.id() == "sequential-direct-off",
                reference_fidelity,
                TIER_N_WALL_TIMING_ADMISSIBLE,
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
                reference_fidelity,
                verdict,
                blockers,
                not_evaluated,
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
                    not_evaluated: Vec::new(),
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
                    not_evaluated: Vec::new(),
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
            let mut not_evaluated = Vec::new();
            if candidate.family() == CandidateFamily::Sequential
                && candidate.id() != "sequential-direct-off"
            {
                blockers.retain(|blocker| blocker != "Tier-L performance assessment required");
                if let Some(linear) = tier_l {
                    blockers.extend(linear.blockers.clone());
                    not_evaluated.extend(linear.not_evaluated.clone());
                } else {
                    blockers.push("Tier-L assessment missing".into());
                }
            } else if candidate.id() != "sequential-direct-off" {
                if let Some(nonlinear) = tier_n {
                    blockers.extend(nonlinear.blockers.clone());
                    not_evaluated.extend(nonlinear.not_evaluated.clone());
                } else {
                    blockers.push("Tier-N performance assessment missing".into());
                }
            }
            blockers.sort();
            blockers.dedup();
            not_evaluated.sort();
            not_evaluated.dedup();
            let tier_verdict_for_family = if candidate.family() == CandidateFamily::Sequential {
                tier_l_verdict
            } else {
                tier_n_verdict
            };
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
            } else if scientific_eligible
                && blockers.is_empty()
                && tier_verdict_for_family == Some(UnifiedJointVerdict::NotEvaluated)
            {
                UnifiedJointVerdict::NotEvaluated
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
                not_evaluated,
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
    paired_timing: &BTreeMap<String, PairedTimingEvidence>,
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
    let linear_assessments = assess_linear_candidates_against(
        &linear_suites,
        TIER_L_REFERENCE_FIDELITY,
        paired_timing,
        &TimingExpectation::for_profile(profile)?,
    );
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
            paired_timing_evidence,
            output,
        } => {
            let evidence = match paired_timing_evidence {
                Some(path) => serde_json::from_slice::<BTreeMap<String, PairedTimingEvidence>>(
                    &fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
                )
                .with_context(|| format!("parsing {}", path.display()))?,
                None => BTreeMap::new(),
            };
            write_json(
                &output,
                &run_unified_candidate_document(profile, threads, full, &evidence)?,
            )?;
        }
        Command::PairedTimingSession {
            campaign_id,
            session,
            candidate,
            profile,
            seed,
            batches,
            output,
        } => {
            let batches = match batches {
                Some(path) => serde_json::from_slice(&fs::read(&path)?)?,
                None => BTreeMap::new(),
            };
            let record = run_tier_l_paired_session(
                &campaign_id,
                session,
                &candidate,
                profile,
                seed,
                &batches,
            )?;
            write_json(&output, &record)?;
        }
        Command::PairedTimingCampaign {
            candidate,
            profile,
            sessions,
            seed,
            output,
        } => {
            let evidence =
                run_tier_l_paired_campaign(&candidate, profile, sessions, seed, &output)?;
            let mut map = BTreeMap::new();
            map.insert(candidate, evidence);
            write_json(&output, &map)?;
        }
        Command::R3CampaignSession {
            study,
            arm,
            campaign_id,
            session,
            seed,
            batches,
            output,
        } => {
            let batches = match batches {
                Some(path) => serde_json::from_slice(&fs::read(&path)?)?,
                None => BTreeMap::new(),
            };
            let record =
                r3_campaigns::run_session(study, &arm, &campaign_id, session, seed, &batches)?;
            write_json(&output, &record)?;
        }
        Command::R3Campaign {
            study,
            sessions,
            seed,
            output,
        } => {
            if output.exists() {
                anyhow::bail!("immutable output already exists: {}", output.display());
            }
            let mut evidence = BTreeMap::new();
            let mut summary = BTreeMap::new();
            for arm in study.arms() {
                // An arm whose campaign cannot be assessed is recorded, not
                // allowed to discard the other arms (re-audit R3 review).
                let arm_evidence =
                    match r3_campaigns::run_campaign(study, arm, sessions, seed, &output) {
                        Ok(arm_evidence) => arm_evidence,
                        Err(error) => {
                            println!("{} {arm}: campaign failed: {error}", study.name());
                            summary.insert(arm.to_string(), json!({ "error": error.to_string() }));
                            continue;
                        }
                    };
                let verified = arm_evidence.verified_decision().map_or_else(
                    |error| format!("not verified: {error}"),
                    |d| format!("{d:?}"),
                );
                let authority = arm_evidence
                    .admissible_decision()
                    .map(|decision| json!({ "authority": decision.authority(), "admissible_decision": decision.admissible(), "reason": decision.reason() }))
                    .unwrap_or_else(|error| json!({ "error": error.to_string() }));
                summary.insert(
                    arm.to_string(),
                    json!({
                        "verified_decision": verified,
                        "statistical_authority": authority,
                        "assessment_decision": arm_evidence.assessment.decision,
                        "gate_decision": arm_evidence.assessment.gate_decision,
                        "speedup_point": arm_evidence.assessment.corpus.point,
                        "speedup_lower": arm_evidence.assessment.corpus.lower,
                        "speedup_upper": arm_evidence.assessment.corpus.upper,
                        "independent_sessions": arm_evidence.assessment.corpus.independent_blocks,
                        "failed_sessions": arm_evidence.receipt.failed_sessions.len(),
                        "receipt_failures": arm_evidence.receipt.failures.len(),
                    }),
                );
                println!("{} {arm}: verified decision {verified}", study.name());
                evidence.insert(arm.to_string(), arm_evidence);
            }
            write_json_create_new(
                &output,
                &json!({
                    "study": study.name(),
                    "reference": study.reference(),
                    "seed": seed,
                    "sessions": sessions,
                    "summary": summary,
                    "evidence": evidence,
                }),
            )?;
        }
        Command::R3VerifyArm { study, arm, output } => {
            r3_campaigns::verify_arm(study, &arm, &output)?;
        }
        Command::TimingAuthority { campaign, output } => {
            if output.exists() {
                anyhow::bail!("immutable output already exists: {}", output.display());
            }
            let report = timing_authority_report(&serde_json::from_slice(&fs::read(&campaign)?)?)?;
            write_json_create_new(&output, &report)?;
        }
        Command::R3CampaignVerify { study, output } => {
            if output.exists() {
                anyhow::bail!("immutable output already exists: {}", output.display());
            }
            let executable = std::env::current_exe()?;
            let directory = output.with_extension("arms");
            fs::create_dir_all(&directory)?;
            let mut arms = BTreeMap::new();
            for arm in std::iter::once(study.reference()).chain(study.arms().iter().copied()) {
                let path = directory.join(format!("{arm}.json"));
                let status = std::process::Command::new(&executable)
                    .arg("r3-verify-arm")
                    .args(["--study", study.name()])
                    .args(["--arm", arm])
                    .arg("--output")
                    .arg(&path)
                    .status()?;
                let value = if status.success() {
                    serde_json::from_slice::<serde_json::Value>(&fs::read(&path)?)?
                } else {
                    json!({ "arm": arm, "failed": format!("verification process exited with {status}") })
                };
                arms.insert(arm.to_string(), value);
            }
            write_json_create_new(&output, &json!({ "study": study.name(), "arms": arms }))?;
        }
        Command::StiffBenchmark {
            repetitions,
            warmups,
            problems,
            arms,
            output,
        } => {
            let problems = problems.unwrap_or_else(|| {
                stiff_benchmark::DEFAULT_PROBLEMS
                    .iter()
                    .map(|id| id.to_string())
                    .collect()
            });
            let arms = arms.unwrap_or_else(|| {
                stiff_benchmark::ARMS
                    .iter()
                    .map(|id| id.to_string())
                    .collect()
            });
            if output.exists() {
                anyhow::bail!("immutable output already exists: {}", output.display());
            }
            write_json(
                &output,
                &stiff_benchmark::stiff_benchmark(repetitions, warmups, &problems, &arms)?,
            )?;
        }
        Command::StiffProfileRun {
            problem,
            arm,
            rtol,
            repetitions,
        } => {
            println!(
                "{}",
                stiff_benchmark::profile_run(&problem, &arm, rtol, repetitions)?
            );
        }
        Command::R4Study {
            study,
            actions,
            output,
        } => {
            if output.exists() {
                anyhow::bail!("immutable output already exists: {}", output.display());
            }
            let report = match study.as_str() {
                "laguerre" => r4_studies::laguerre_study()?,
                "polynomial-regimes" => r4_studies::polynomial_regimes_study(actions)?,
                "homotopy-cost" => r4_studies::homotopy_cost_study()?,
                other => anyhow::bail!("unknown R4 study {other:?}"),
            };
            write_json_create_new(&output, &report)?;
            println!("r4 study {study}: written to {}", output.display());
        }
        Command::SessionMedianCoverageStudy {
            replications,
            seed,
            threads,
            scenario,
            output,
        } => {
            if output.exists() {
                anyhow::bail!("immutable output already exists: {}", output.display());
            }
            let grid = rodas5p_fair_ab::session_median_study_grid()
                .into_iter()
                .filter(|entry| scenario.is_empty() || scenario.contains(&entry.id))
                .collect::<Vec<_>>();
            if grid.is_empty() {
                anyhow::bail!("no session-median scenario matches {scenario:?}");
            }
            let report = rodas5p_fair_ab::session_median_study(&grid, replications, seed, threads)?;
            write_json_create_new(&output, &report)?;
            println!(
                "session-median study: {} scenarios, verdict {} (in-domain failures {}, out-of-domain failures {})",
                report.scenarios.len(),
                report.verdict,
                report.in_domain_failures,
                report.out_of_domain_failures
            );
        }
        Command::PairedTimingCoverageStudy {
            replications,
            seed,
            threads,
            scenario,
            output,
        } => {
            let protocol = rodas5p_fair_ab::PairedTimingProtocol::authoritative(seed);
            let grid = rodas5p_fair_ab::preregistered_coverage_grid(&protocol)
                .into_iter()
                .filter(|entry| scenario.is_empty() || scenario.contains(&entry.id))
                .collect::<Vec<_>>();
            if grid.is_empty() {
                anyhow::bail!("no coverage scenario matches {scenario:?}");
            }
            let report =
                rodas5p_fair_ab::coverage_study(&grid, replications, seed, threads, &protocol)?;
            write_json_create_new(&output, &report)?;
            println!(
                "coverage study: {} scenarios, verdict {} (sensitivity pass: {})",
                report.scenarios.len(),
                report.verdict,
                report.sensitivity_pass
            );
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
        suite_with_plan(gcrodr_wall, 3, 1)
    }

    fn suite_with_plan(gcrodr_wall: f64, repetitions: usize, warmups: usize) -> UnifiedLinearSuite {
        UnifiedLinearSuite {
            kind: SequenceKind::Fixed,
            trace_id: "trace".into(),
            failures: 0,
            plan: BenchmarkPlan {
                cells: strict_cells(),
                repetitions,
                warmups,
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

    /// Paired timing evidence of GCRO-DR/persistent against GMRES/OFF from
    /// deterministic samples: six session records (distinct processes) of
    /// four cases with 2% noise, merged into a receipt.
    fn paired_evidence(speedup: f64, with_aa: bool) -> PairedTimingEvidence {
        use rodas5p_fair_ab::{
            PAIRED_TIMING_MONOTONIC_CLOCK, PairedTimingCase, SessionProvenance,
            detect_timing_host_metadata,
        };
        let protocol = PairedTimingProtocol::authoritative(7);
        let mut state = 1_u64;
        let mut noise = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            1.0 + 0.02 * (2.0 * ((state >> 11) as f64 / (1_u64 << 53) as f64) - 1.0)
        };
        let mut cases = |ratio: f64, suffix: &str, session: u32| {
            (0..4)
                .map(|case| {
                    let base = 1.0e-3 * (1.0 + case as f64);
                    let candidate = (0..protocol.pairs)
                        .map(|_| base * noise())
                        .collect::<Vec<_>>();
                    let reference = (0..protocol.pairs)
                        .map(|_| base * ratio * noise())
                        .collect::<Vec<_>>();
                    PairedTimingCase::from_samples(
                        format!("case-{case}{suffix}"),
                        &protocol,
                        vec![5.0e-3, 1.0e-3],
                        candidate,
                        reference,
                    )
                    .unwrap()
                    .with_process_blocks(vec![session; protocol.pairs])
                })
                .collect::<Vec<_>>()
        };
        let records = (0..6)
            .map(|session| SessionRecord {
                campaign_id: "test-campaign".into(),
                provenance: SessionProvenance {
                    session,
                    process_id: 1_000 + session,
                    started_unix_seconds: 0.0,
                    finished_unix_seconds: 1.0,
                    clock: PAIRED_TIMING_MONOTONIC_CLOCK.into(),
                    host: detect_timing_host_metadata(1),
                },
                cases: cases(speedup, "", session),
                aa_cases: if with_aa {
                    cases(1.0, "#aa", session)
                } else {
                    Vec::new()
                },
                failures: Vec::new(),
            })
            .collect();
        let arm = |id: &str| ArmIdentity {
            arm_id: id.into(),
            executable_sha256: "0".repeat(64),
            workload_id: "test".into(),
        };
        let receipt = PairedTimingReceipt::from_sessions(
            "test-campaign",
            arm("sequential-gcrodr-persistent"),
            arm(TIER_L_TIMING_REFERENCE),
            protocol,
            records,
        )
        .unwrap();
        PairedTimingEvidence::from_receipt(receipt).unwrap()
    }

    #[test]
    fn the_published_poly03_campaign_keeps_its_numbers_under_an_authority_hold() {
        // Re-audit R4, R4-STAT-DEV-01: the R3 POLY-03 PASS (ledger L-0009)
        // stays a verified diagnostic Promote; its design's authority is the
        // failed coverage studies' hold, so no consumer may act on it.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../research/r3_matched_accuracy_poly03_20261001/CAMPAIGN.json"
        );
        let campaign: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let report = timing_authority_report(&campaign).unwrap();
        let arms = report["arms"].as_object().unwrap();
        assert_eq!(arms.len(), 4);
        for (arm, row) in arms {
            assert_eq!(row["authority"], "hold", "{arm}: {row}");
            assert!(row["admissible_decision"].is_null(), "{arm}");
            assert!(row["reason"].as_str().unwrap().contains("L-0010"), "{arm}");
            let published = &campaign["summary"][arm];
            assert_eq!(row["speedup_point"], published["speedup_point"], "{arm}");
            assert_eq!(
                row["diagnostic_decision"].as_str().unwrap(),
                published["verified_decision"]
                    .as_str()
                    .unwrap()
                    .to_lowercase(),
                "{arm}"
            );
        }
    }

    #[test]
    fn tier_l_wall_decisions_come_only_from_a_paired_assessment() {
        // Audit 2026-09-30, B-03: three repetitions after one warmup with a
        // median wall ratio of 0.8 used to promote. Without paired timing
        // evidence the wall criterion is not evaluated.
        let gcrodr = |rows: Vec<UnifiedLinearCandidateAssessment>| {
            rows.into_iter()
                .find(|row| row.candidate_id == "sequential-gcrodr-persistent")
                .unwrap()
        };
        let row = gcrodr(assess_linear_candidates(&[suite(0.8)]));
        assert_eq!(row.verdict, UnifiedJointVerdict::NotEvaluated);
        assert_eq!(row.not_evaluated, vec![WALL_NOT_EVALUATED]);

        // A registry in which the pooled design passed its study: the
        // integrity path alone decides (as before R4).
        let admissible = rodas5p_fair_ab::timing_authority_registry()
            .into_iter()
            .map(|mut study| {
                study.status = rodas5p_fair_ab::TimingAuthorityStatus::Admissible;
                study
            })
            .collect::<Vec<_>>();
        let with_registry =
            |evidence: PairedTimingEvidence,
             authorities: Option<Vec<rodas5p_fair_ab::TimingAuthority>>| {
                let map = BTreeMap::from([("sequential-gcrodr-persistent".to_string(), evidence)]);
                gcrodr(assess_linear_candidates_against(
                    &[suite(0.8)],
                    TIER_L_REFERENCE_FIDELITY,
                    &map,
                    &TimingExpectation {
                        workload_prefix: "test".into(),
                        executable_sha256: Some("0".repeat(64)),
                        authorities,
                    },
                ))
            };
        let with =
            |evidence: PairedTimingEvidence| with_registry(evidence, Some(admissible.clone()));
        // Receipt-backed paired evidence above 1.15x promotes.
        let row = with(paired_evidence(1.30, true));
        assert_eq!(row.verdict, UnifiedJointVerdict::Promote, "{row:?}");
        // One below 1.15x holds.
        let row = with(paired_evidence(0.90, true));
        assert_eq!(row.verdict, UnifiedJointVerdict::Hold, "{row:?}");
        assert!(row.blockers[0].contains("paired Tier-L wall-speedup interval"));
        // Re-audit R4, R4-STAT-DEV-01: under the compiled registry (both
        // coverage studies failed) neither decision is acted on; the
        // diagnostic Promote is still what the evidence verifies to.
        for speedup in [1.30, 0.90] {
            let evidence = paired_evidence(speedup, true);
            let decision = evidence.admissible_decision().unwrap();
            assert_eq!(
                decision.authority(),
                rodas5p_fair_ab::TimingAuthorityStatus::Hold
            );
            assert!(decision.admissible().is_none());
            let row = with_registry(evidence, None);
            assert_eq!(row.verdict, UnifiedJointVerdict::NotEvaluated, "{row:?}");
            assert_eq!(row.not_evaluated, vec![WALL_AUTHORITY_HOLD]);
        }
        assert_eq!(
            paired_evidence(1.30, true).verified_decision().unwrap(),
            PairedTimingDecision::Promote
        );
        // Without an A/A control the timing is not authoritative.
        let row = with(paired_evidence(1.30, false));
        assert_eq!(row.verdict, UnifiedJointVerdict::NotEvaluated);
        assert_eq!(row.not_evaluated, vec![WALL_INCONCLUSIVE]);
        // Re-audit R3, STAT-DEV-02: an assessment whose raw receipt was
        // corrupted, a receipt with a failed case, or evidence for another
        // arm never gates.
        let mut corrupted = paired_evidence(1.30, true);
        corrupted.receipt.session_records[2].cases[0].candidate_seconds[0] *= 0.5;
        assert_eq!(with(corrupted).not_evaluated, vec![WALL_REJECTED]);
        let mut failed = paired_evidence(1.30, true);
        let failure = rodas5p_fair_ab::SessionFailure {
            session: 3,
            case_id: "case-9".into(),
            message: "failed solve".into(),
        };
        failed.receipt.session_records[3]
            .failures
            .push(failure.clone());
        failed.receipt.failures.push(failure);
        assert_eq!(with(failed).not_evaluated, vec![WALL_INCONCLUSIVE]);
        let mut other = paired_evidence(1.30, true);
        other.receipt.candidate.arm_id = "sequential-lgmres-off".into();
        assert_eq!(with(other).not_evaluated, vec![WALL_WRONG_ARMS]);
        // Evidence from another executable or workload never gates.
        let mut binary = paired_evidence(1.30, true);
        binary.receipt.reference.executable_sha256 = "1".repeat(64);
        assert_eq!(with(binary).not_evaluated, vec![WALL_WRONG_IDENTITY]);
        let mut smoke = paired_evidence(1.30, true);
        smoke.receipt.candidate.workload_id = "tier-l-smoke-seed-1".into();
        assert_eq!(with(smoke).not_evaluated, vec![WALL_WRONG_IDENTITY]);
        // Failures dropped from the top-level list are detected.
        let mut hidden = paired_evidence(1.30, true);
        hidden.receipt.session_records[1]
            .failures
            .push(rodas5p_fair_ab::SessionFailure {
                session: 1,
                case_id: "case-9".into(),
                message: "failed solve".into(),
            });
        assert_eq!(with(hidden).not_evaluated, vec![WALL_REJECTED]);
    }

    #[test]
    fn nonlinear_batch_depth_advantage_does_not_replace_wall_speedup() {
        let (verdict, blockers, not_evaluated) = nonlinear_performance_verdict(
            false,
            ComparatorFidelity::Production,
            true,
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
        assert!(not_evaluated.is_empty());
    }

    #[test]
    fn smoke_and_single_sample_wall_blockers_are_not_evaluated() {
        // Audit F-053: the Smoke profile (repetitions=1, warmups=0) and
        // single-sample timings never decide Promote/Hold on wall time, in
        // either direction.
        for gcrodr_wall in [0.5, 0.9, 2.0] {
            for (repetitions, warmups) in [(1, 0), (1, 1), (3, 0)] {
                let rows =
                    assess_linear_candidates(&[suite_with_plan(gcrodr_wall, repetitions, warmups)]);
                let row = rows
                    .iter()
                    .find(|row| row.candidate_id == "sequential-gcrodr-persistent")
                    .unwrap();
                assert_eq!(row.verdict, UnifiedJointVerdict::NotEvaluated);
                assert!(row.blockers.is_empty(), "{:?}", row.blockers);
                assert_eq!(row.not_evaluated, vec![WALL_NOT_EVALUATED]);
            }
        }
        for compute_ratio in [0.5, 2.0] {
            let (verdict, blockers, not_evaluated) = nonlinear_performance_verdict(
                false,
                ComparatorFidelity::Production,
                TIER_N_WALL_TIMING_ADMISSIBLE,
                6,
                6,
                0,
                Some(compute_ratio),
                Some(3.0),
                Some(5.0),
                None,
            );
            assert_eq!(verdict, UnifiedJointVerdict::NotEvaluated);
            assert!(blockers.is_empty());
            assert_eq!(not_evaluated, vec![WALL_NOT_EVALUATED]);
        }
        // Intrinsic failures still hold without any wall evidence.
        let (verdict, blockers, _) = nonlinear_performance_verdict(
            false,
            ComparatorFidelity::Production,
            false,
            5,
            6,
            1,
            Some(0.5),
            None,
            None,
            None,
        );
        assert_eq!(verdict, UnifiedJointVerdict::Hold);
        assert_eq!(blockers.len(), 2);
    }

    #[test]
    fn relative_gates_against_a_reference_only_comparator_are_not_evaluated() {
        // Audit F-052/F-056 Tier B: neither a clear win nor a clear loss may
        // be emitted when the reference arm is reference-implementation-only.
        for gcrodr_wall in [0.5, 2.0] {
            let rows = assess_linear_candidates_against(
                &[suite(gcrodr_wall)],
                ComparatorFidelity::ReferenceImplementationOnly,
                &BTreeMap::new(),
                &TimingExpectation {
                    workload_prefix: "test".into(),
                    executable_sha256: None,
                    authorities: None,
                },
            );
            let row = rows
                .iter()
                .find(|row| row.candidate_id == "sequential-gcrodr-persistent")
                .unwrap();
            assert_eq!(row.verdict, UnifiedJointVerdict::NotEvaluated);
            assert!(row.blockers.is_empty());
            assert_eq!(row.not_evaluated, vec![REFERENCE_ONLY_NOT_EVALUATED]);
        }
        for compute_ratio in [0.5, 2.0] {
            let (verdict, blockers, not_evaluated) = nonlinear_performance_verdict(
                false,
                ComparatorFidelity::ReferenceImplementationOnly,
                true,
                6,
                6,
                0,
                Some(compute_ratio),
                Some(3.0),
                Some(5.0),
                None,
            );
            assert_eq!(verdict, UnifiedJointVerdict::NotEvaluated);
            assert!(blockers.is_empty());
            assert_eq!(not_evaluated, vec![REFERENCE_ONLY_NOT_EVALUATED]);
        }
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
