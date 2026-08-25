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
mod g3_fused_adaptive_gate;
mod g4_prefix_kernel_gate;
mod g4_s5b0_inner_tolerance;
mod g4_s5b0_regime_atlas;
mod g4_s5b0_trace_authority;
mod g4_s5b3_attempt_geometry;
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
mod policy_redesign_v25;
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
mod v38d_performance_tournament;

pub use adaptive::{
    AdaptiveControllerState, AdaptiveObservedIntegrationResult, AdaptiveRunDiagnostics,
    AdaptiveStepConfig, ControllerKind, StepDoublingEstimate, step_doubling_wrms_error,
};
pub use adaptive_exponential::{
    AdaptiveEarlyFlowDefectAttempt, AdaptiveEarlyFlowDefectOutcome,
    AdaptiveFusedExponentialDiagnostics, AdaptiveFusedExponentialResult,
    integrate_pexprb54s4_fused_adaptive_observed,
    integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode,
    integrate_pexprb54s4_fused_adaptive_observed_with_tolerance_scaled_telemetry,
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
    EarlyFlowDefectDiagnosticWork, EarlyFlowDefectTelemetry, EarlyFlowDefectTelemetryMode,
    ExponentialKrylovConfig, ExponentialStepReport, FusedExponentialStepReport,
    FusedOrthogonalization, FusedPhiActionReport, FusedPhiKrylovConfig, FusedPhiPrefixPrediction,
    FusedPhiPrefixSession, FusedPhiSubstepReport, FusedPhiTerm,
    Pexprb54s4AccountedBudgetedLevel2PrefixOutcome, Pexprb54s4BudgetExhaustedPrefixReport,
    Pexprb54s4BudgetedLevel2PrefixOutcome, Pexprb54s4FailedPrefixReport, Pexprb54s4Level1Prefix,
    Pexprb54s4Level1PrefixReport, Pexprb54s4Level2ContinuationLedger,
    Pexprb54s4Level2ContinuationOutcome, Pexprb54s4Level2Prefix, Pexprb54s4Level2PrefixReport,
    Pexprb54s4QuadraticRemainderDrift, Pexprb54s4RemainderVectorGeometry, Pexprb54s4Tableau,
    PhiActionReport, exprb2_fused_step, exprb2_step, exprb43_fused_step, exprb43_step,
    fused_phi_action, fused_phi_action_incremental, fused_phi_linear_combination,
    krylov_phi_action, pexprb54s4_fused_step, pexprb54s4_fused_step_resume_level1,
    pexprb54s4_fused_step_resume_level2, pexprb54s4_fused_step_resume_level2_accounted,
    pexprb54s4_fused_step_resume_level2_accounted_jvp_budget,
    pexprb54s4_fused_step_with_telemetry_mode,
    pexprb54s4_fused_step_with_tolerance_scaled_telemetry,
    pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry,
    pexprb54s4_level2_prefix_resume_level1,
    pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget,
    pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget_accounted,
    pexprb54s4_quadratic_remainder_drift, pexprb54s4_remainder_vector_geometry, pexprb54s4_step,
    pexprb54s4_tableau,
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
pub use g3_fused_adaptive_gate::{
    G3AdaptiveRow, G3FreshJvpRow, G3FusedAdaptiveProfile, G3FusedAdaptiveReport,
    G3FusedAdaptiveSummary, G3PhiFusionRow, run_g3_fused_adaptive_gate,
};
pub use g4_prefix_kernel_gate::{
    G4PrefixKernelProfile, G4PrefixKernelReport, G4PrefixKernelRow, G4PrefixKernelSummary,
    run_g4_prefix_kernel_gate,
};
pub use g4_s5b0_inner_tolerance::{
    G4_S5B0_COMMITTED_LINEAR_TOLERANCE_ARM, G4S5B0InnerToleranceLane, G4S5B0InnerTolerancePolicy,
    G4S5B0LinearToleranceArm, committed_g4_s5b0_linear_tolerance_arm,
};
pub use g4_s5b0_regime_atlas::{
    G4S5B0ActualLevel1PrefixReport, G4S5B0ActualLevel1PrefixRow, G4S5B0ActualLevel2PrefixReport,
    G4S5B0ActualLevel2PrefixRow, G4S5B0AttemptTraceReport, G4S5B0Family,
    G4S5B0FrozenFullEShadowEconomicsReport, G4S5B0FrozenFullEShadowHardGates,
    G4S5B0FrozenFullEShadowReport, G4S5B0FrozenFullEShadowRow, G4S5B0PrefixProbePolicy,
    G4S5B0Profile, G4S5B0Report, G4S5B0RjfAttemptRow, G4S5B0RjfParitySummary, G4S5B0ShadowWallArm,
    G4S5B0ShadowWallCalibrationRow, G4S5B0ShadowWallPair, G4S5B0ShadowWallReport,
    G4S5B0StageGrowthSafetyReport, G4S5B0StageGrowthSafetyRow, G4S5B0StepRow,
    G4S5B0TrajectorySummary, G4S5B0V37ContinuationTransactionHardGates,
    G4S5B0V37ContinuationTransactionReport, G4S5B0V37ContinuationTransactionRow,
    V36_FROZEN_ZETA34_TAU, V37_CONTINUATION_JVP_CAP, enforced_prefix_jvp_cap,
    frozen_full_e_shadow_recommended, run_g4_s5b0_actual_level1_prefix_family,
    run_g4_s5b0_actual_level2_prefix_family, run_g4_s5b0_enforced_prefix_budget_family,
    run_g4_s5b0_frozen_full_e_shadow, run_g4_s5b0_frozen_full_e_shadow_economics,
    run_g4_s5b0_frozen_full_e_shadow_family, run_g4_s5b0_regime_atlas,
    run_g4_s5b0_rjf_attempt_trace, run_g4_s5b0_rjf_attempt_trace_family, run_g4_s5b0_rjf_only,
    run_g4_s5b0_rjf_only_family, run_g4_s5b0_stage_growth_safety_audit_family,
    run_g4_s5b0_v37_continuation_transaction, run_g4_s5b0_v37_continuation_transaction_family,
};
pub use g4_s5b0_trace_authority::{
    g4_s5b0_rjf_trace_digest, run_g4_s5b0_rjf_attempt_trace_family_with_linear_tolerance_arm,
};
pub use g4_s5b3_attempt_geometry::{
    G4S5B3AttemptGeometryReport, G4S5B3AttemptRow, G4S5B3CalibrationRow, G4S5B3HardGateSummary,
    G4S5B3OverheadArm, G4S5B3OverheadPair, G4S5B3OverheadReport, G4S5B3Profile,
    G4S5B3TrajectorySummary, run_g4_s5b3_attempt_geometry, run_p1_00_tolerance_scaled_early_defect,
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

pub use v38d_performance_tournament::{
    V38D_EXPLORATORY_PROBE_SCHEMA, V38D_EXPLORATORY_PROBE_STATUS, V38D_MEASURED_REPETITIONS,
    V38D_WARMUP_REPETITIONS, V38dCandidateId, V38dProbeCaseId, V38dProbeReport, V38dProbeSample,
    run_v38d_probe,
};

pub use policy_redesign_v25::{
    CausalRjfStep, PersistenceLatch, PointFeature, PrefixBudget, ProbeAction, causal_feature_value,
};
