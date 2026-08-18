use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rodas5p_core::{load_rodas5p_coefficients, sha256_hex};
use rodas5p_fair_ab::{
    BenchmarkCell, BenchmarkPlan, FairSolveConfig, GlobalErrorParetoProfile, PreconditionerKind,
    RecycleLifetime, SequenceConfig, SequenceKind, SolverKind, TraceDocument, generate_trace,
    run_adaptive_global_error_screen, run_comparison, run_g1_adaptive_global_error_screen,
    run_global_error_pareto_screen, summarize_comparison,
};
use rodas5p_integrators::{
    CandidateCatalog, CandidateFamily, CandidateStatus, G1TransactionalGateProfile,
    G2ExponentialGateProfile, G3FusedAdaptiveProfile, G4PrefixKernelProfile, G4S5B0Family,
    G4S5B0PrefixProbePolicy, G4S5B0Profile, G4S5B3Profile, HomotopyExperimentProfile,
    HomotopyRhsTelemetryProfile, MatrixFreeCommonWProfile, NativeIntegratorGateReport,
    PathControllerProfile, StageBatchFeasibilityProfile, UnifiedNonlinearScreen,
    UnifiedScientificGateReport, UnifiedScreenProfile, run_g1_transactional_gate,
    run_g2_exponential_gate, run_g3_fused_adaptive_gate, run_g4_prefix_kernel_gate,
    run_g4_s5b0_actual_level1_prefix_family, run_g4_s5b0_actual_level2_prefix_family,
    run_g4_s5b0_enforced_prefix_budget_family, run_g4_s5b0_regime_atlas,
    run_g4_s5b0_rjf_attempt_trace, run_g4_s5b0_rjf_attempt_trace_family, run_g4_s5b0_rjf_only,
    run_g4_s5b0_rjf_only_family, run_g4_s5b0_stage_growth_safety_audit_family,
    run_g4_s5b3_attempt_geometry, run_homotopy_design_check, run_homotopy_experiment_screen,
    run_homotopy_order_policy_screen, run_homotopy_rhs_telemetry_screen,
    run_matrix_free_common_w_gate, run_native_integrator_gates,
    run_p1_00_tolerance_scaled_early_defect, run_path_controller_screen,
    run_stage_batch_feasibility, run_unified_nonlinear_screen, run_unified_scientific_gates,
};
use serde::Serialize;
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
enum CliPolicyRedesignFamily {
    Robertson,
    Hires,
    VanDerPol,
    RotatingNonnormal,
    NonautonomousForcing,
    Semilinear,
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
    fn document_status_exposes_uncertified_reference_rows() {
        assert_eq!(
            unified_document_status(0, 0, 4),
            "complete-with-uncertified"
        );
        assert_eq!(unified_document_status(0, 0, 0), "complete");
        assert_eq!(unified_document_status(1, 0, 0), "complete-with-failures");
    }
}
