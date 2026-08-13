#![forbid(unsafe_code)]

mod adaptive;
mod adaptive_exponential;
mod bdf;
mod block;
mod candidates;
mod certification;
mod common_w_gate;
mod exponential;
mod g1_transactional_gate;
mod g2_exponential_gate;
mod homotopy;
mod homotopy_experiments;
mod homotopy_order_policy;
mod homotopy_policy;
mod integrate;
mod native_gates;
mod nonlinear;
mod output;
mod parallel;
mod path_controller;
mod problem;
mod problems;
mod radau;
mod rhs_telemetry;
mod sabr;
mod sequential;
mod stage_batch;
mod transactional_q1_q2;
mod unified_gates;
mod unified_screen;

pub use adaptive::{
    AdaptiveControllerState, AdaptiveObservedIntegrationResult, AdaptiveRunDiagnostics,
    AdaptiveStepConfig, ControllerKind, StepDoublingEstimate, step_doubling_wrms_error,
};
pub use adaptive_exponential::{
    AdaptiveFusedExponentialDiagnostics, AdaptiveFusedExponentialResult,
    integrate_pexprb54s4_fused_adaptive_observed,
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
pub use exponential::{
    ExponentialKrylovConfig, ExponentialStepReport, FusedExponentialStepReport,
    FusedOrthogonalization, FusedPhiActionReport, FusedPhiKrylovConfig,
    FusedPhiSubstepReport, FusedPhiTerm, Pexprb54s4Tableau, PhiActionReport,
    exprb2_fused_step, exprb2_step, exprb43_fused_step, exprb43_step, fused_phi_action,
    fused_phi_linear_combination, krylov_phi_action, pexprb54s4_fused_step,
    pexprb54s4_step, pexprb54s4_tableau,
};
pub use g1_transactional_gate::{
    G1TransactionalCase, G1TransactionalGateProfile, G1TransactionalGateReport,
    G1TransactionalGateSummary, G1TransactionalRow, run_g1_transactional_gate,
};
pub use g2_exponential_gate::{
    ExponentialCoefficientAuthority, ExponentialOrderConditionRow, ExponentialOrderRow,
    G2ExponentialGateProfile, G2ExponentialGateReport, G2ExponentialGateSummary,
    OscillatoryExponentialRow, PhiOracleRow, StiffLinearExponentialRow, run_g2_exponential_gate,
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
    IntegrationMethod, IntegrationResult, TransactionalQ1Q2AdaptiveResult, integrate_adaptive,
    integrate_adaptive_observed, integrate_adaptive_observed_with_config, integrate_fixed,
    integrate_fixed_observed, integrate_homotopy_adaptive_observed,
    integrate_sequential_matrix_free_adaptive_observed,
    integrate_transactional_q1_q2_adaptive_observed,
};
pub use native_gates::{
    NativeIntegratorGateReport, NativeIntegratorGateRow, run_native_integrator_gates,
};
pub use nonlinear::{NewtonConfig, NewtonReport, solve_dense_newton};
pub use output::{ObservedIntegrationResult, OutputSchedule};
pub use parallel::ParallelExecution;
pub use path_controller::{
    PathControllerCase, PathControllerControlRow, PathControllerProfile, PathControllerReport,
    PathControllerRow, PathControllerScheduleSummary, PathControllerSummary,
    run_path_controller_screen,
};
pub use problem::OdeProblem;
pub use problems::{
    complex_dahlquist_problem, constant_affine_mass_problem, manufactured_mass_nonlinear_problem,
    manufactured_vector_problem, oscillatory_prothero_robinson_problem, prothero_robinson_problem,
    robertson_problem, scalar_linear_problem, semilinear_advection_diffusion_problem,
    stiff_van_der_pol_problem,
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
    build_step_context_matrix_free, finish_step, sequential_matrix_free_step, sequential_stages,
    sequential_step,
};
pub use stage_batch::{
    StageBatchFeasibilityCase, StageBatchFeasibilityProfile, StageBatchFeasibilityReport,
    StageBatchFeasibilityRow, run_stage_batch_feasibility,
};
pub use transactional_q1_q2::{
    OperationalGateReport, TransactionalQ1Q2Config, TransactionalQ1Q2Lane,
    TransactionalQ1Q2RunDiagnostics, TransactionalQ1Q2StepReport, transactional_q1_q2_step,
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
