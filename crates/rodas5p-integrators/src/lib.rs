#![forbid(unsafe_code)]

mod adaptive;
mod bdf;
mod block;
mod candidates;
mod certification;
mod common_w_gate;
mod homotopy;
mod homotopy_experiments;
mod homotopy_order_policy;
mod homotopy_policy;
mod integrate;
mod native_gates;
mod nonlinear;
mod output;
mod parallel;
mod problem;
mod problems;
mod radau;
mod rhs_telemetry;
mod sabr;
mod sequential;
mod stage_batch;
mod unified_gates;
mod unified_screen;

pub use adaptive::{
    AdaptiveControllerState, AdaptiveObservedIntegrationResult, AdaptiveRunDiagnostics,
    AdaptiveStepConfig, ControllerKind, StepDoublingEstimate, step_doubling_wrms_error,
};
pub use bdf::{
    BdfConfig, BdfHistory, BdfIntegrationResult, BdfOrder, BdfStepReport, VariableBdf2Coefficients,
    bdf_step, bdf_step_variable, integrate_bdf_adaptive_observed, integrate_bdf_fixed,
    integrate_bdf_fixed_observed, variable_bdf2_coefficients, variable_bdf2_predictor,
};
pub use block::{
    BlockMethod, BlockPreconditioner, BlockSolveReport, NonlinearRemainderSnapshot,
    StructuredBlockSystem, flatten, unflatten,
};
pub use candidates::{
    CandidateCatalog, CandidateExecution, CandidateFamily, CandidateRecycleLifetime, CandidateSpec,
    CandidateStatus, HomotopyPredictorVariant, SabrBlockVariant, SabrPredictorVariant,
};
pub use certification::{
    CorrectionDiagnostic, RefinedRootCertificate, RefinedRootConfig, certify_second_correction,
    refine_target_root,
};
pub use common_w_gate::{
    MatrixFreeCommonWCase, MatrixFreeCommonWProfile, MatrixFreeCommonWReport, MatrixFreeCommonWRow,
    run_matrix_free_common_w_gate,
};
pub use homotopy::{
    AffineOutputCertificate, AffinePartialCouplingOracle, HomotopyDesignCheckReport,
    HomotopyPathConfig, HomotopyPathPoint, HomotopyPathReport, HomotopyPredictor,
    HomotopyRoundSpec, HomotopyScheduleConfig, HomotopyStepConfig, HomotopyStepReport,
    HomotopyWorkLedger, NonlinearOutputCertificate, NonnormalConditionRow,
    PartialCouplingParameters, PowerNormRow, ScheduledHomotopyPathReport,
    ScheduledHomotopyRoundPoint, TruncationScreenRow, certify_nonlinear_target, homotopy_step,
    run_fixed_homotopy_path, run_homotopy_design_check, run_scheduled_homotopy_path,
};
pub use homotopy_experiments::{
    HomotopyCandidateRow, HomotopyControlRow, HomotopyExperimentCase, HomotopyExperimentProfile,
    HomotopyExperimentReport, HomotopyExperimentSummary, HomotopyOrderScreenRow, HomotopyQSummary,
    run_homotopy_experiment_screen,
};
pub use homotopy_order_policy::{
    HomotopyOrderPolicyReport, PolicyExecutionMetadata, PolicyFamilyWinner, PolicyReplayRow,
    PolicySplitSummary, PolicyTrajectoryGate, PolicyTrajectoryRow, output_policy_grid,
    run_homotopy_order_policy_screen,
};
pub use homotopy_policy::{OutputBudgetDecision, OutputBudgetPolicy};
pub use integrate::{
    IntegrationMethod, IntegrationResult, integrate_adaptive, integrate_adaptive_observed,
    integrate_adaptive_observed_with_config, integrate_fixed, integrate_fixed_observed,
    integrate_homotopy_adaptive_observed,
};
pub use native_gates::{
    NativeIntegratorGateReport, NativeIntegratorGateRow, run_native_integrator_gates,
};
pub use nonlinear::{NewtonConfig, NewtonReport, solve_dense_newton};
pub use output::{ObservedIntegrationResult, OutputSchedule};
pub use parallel::ParallelExecution;
pub use problem::OdeProblem;
pub use problems::{
    constant_affine_mass_problem, manufactured_mass_nonlinear_problem, manufactured_vector_problem,
    prothero_robinson_problem, scalar_linear_problem,
};
pub use radau::{
    RadauConfig, RadauIiaStages, RadauIntegrationResult, RadauStepReport,
    integrate_radau_adaptive_observed, integrate_radau_fixed, integrate_radau_fixed_observed,
    radau_iia3_tableau, radau_step,
};
pub use rhs_telemetry::{
    BackendRecommendationSummary, CommonWBackendChoice, HomotopyRhsTelemetryCase,
    HomotopyRhsTelemetryProfile, HomotopyRhsTelemetryReport, RhsBatchAnalysis,
    RhsBatchTelemetryRow, RhsSubspaceComparison, RhsTelemetryFailure, RhsTelemetryRisk,
    analyze_rhs_batch, analyze_rhs_directions, compare_rhs_subspaces, recommend_common_w_backend,
    run_homotopy_rhs_telemetry_screen,
};
pub use sabr::{PredictorKind, SabrConfig, StageHistory, sabr_step};
pub use sequential::{
    KrylovState, StageSolveData, StepCertificate, StepContext, StepResult, build_step_context,
    finish_step, sequential_stages, sequential_step,
};
pub use stage_batch::{
    StageBatchFeasibilityCase, StageBatchFeasibilityProfile, StageBatchFeasibilityReport,
    StageBatchFeasibilityRow, run_stage_batch_feasibility,
};
pub use unified_gates::{
    CandidateGateReport, CandidateGateVerdict, CandidateOrderGateRow, CandidateStiffGateRow,
    UnifiedScientificGateReport, run_unified_scientific_gates,
};
pub use unified_screen::{
    ReferenceCertificateSource, UnifiedCandidateOutcome, UnifiedCandidateRow,
    UnifiedCaseDescriptor, UnifiedNonlinearScreen, UnifiedNonlinearSummary, UnifiedScreenProfile,
    run_unified_nonlinear_screen,
};
