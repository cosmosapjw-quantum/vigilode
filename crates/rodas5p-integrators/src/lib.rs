#![forbid(unsafe_code)]

#[cfg(feature = "audit2-bateman-authority")]
mod audit2_bateman_real_client_research;
#[cfg(feature = "audit2-research")]
mod audit2_matrix_free_research;
#[cfg(feature = "audit2-research")]
pub mod audit2_research;
#[cfg(feature = "audit2-research")]
mod audit2_reusable_transaction_research;

mod a1_two_arm_receipt;
mod adaptive;
mod adaptive_exponential;
mod bdf;
mod block;
mod candidates;
mod certification;
mod common_w_gate;
mod dense_output_v2;
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
mod scientific_corpus_v2;
mod scientific_validity_v2_gate;
mod sequential;
mod stage_batch;
mod transactional_q1_q2;
mod unified_gates;
mod unified_screen;
mod v38d_performance_tournament;

#[cfg(feature = "audit2-bateman-authority")]
pub use audit2_bateman_real_client_research::{
    AUDIT2_BATEMAN_AUTHORITY_MANIFEST_SHA256, AUDIT2_BATEMAN_AUTHORITY_PROOF_SHA256,
    AUDIT2_BATEMAN_AUTHORITY_VERIFIER_SHA256, AUDIT2_BATEMAN_CHANGED_W_CASE_ID,
    AUDIT2_BATEMAN_CLIENT_ID, AUDIT2_BATEMAN_FROZEN_W_SCHEMA, AUDIT2_BATEMAN_NOMINAL_CASE_ID,
    AUDIT2_BATEMAN_SCENARIO_IDS, Audit2BatemanOperatorAuthority, Audit2BatemanPartialFailure,
    Audit2BatemanRealClientAuthority, Audit2BatemanRealClientManifest,
    Audit2BatemanRuntimeBindingReceipt, Audit2BatemanScenarioDisposition,
    Audit2BatemanScenarioKind, Audit2BatemanScenarioPlan, Audit2BatemanScenarioReceipt,
    Audit2BatemanSixCaseReport, Audit2BatemanStepReceipt,
    admit_audit2_bateman_real_client_authority, audit2_bateman_real_client_manifest,
    audit2_bateman_six_case_plan, audit2_bateman_verify_runtime_operator_bindings_candidate_free,
    run_audit2_bateman_local_six_case_suite,
};
#[cfg(feature = "audit2-research")]
pub use audit2_matrix_free_research::{
    Audit2MatrixFreeBatchFailure, Audit2MatrixFreeBatchOutcome, Audit2MatrixFreeBatchSuccess,
    Audit2MatrixFreeCommonWConfig, Audit2MatrixFreeCommonWSession,
    Audit2MatrixFreeCorrectionFailure, Audit2MatrixFreeCorrectionFailurePhase,
    Audit2MatrixFreeCorrectionOutcome, Audit2MatrixFreeCorrectionSuccess,
    Audit2MatrixFreeCorrectionWork, Audit2MatrixFreeFailurePhase,
    Audit2MatrixFreeSessionSetupFailure, Audit2MatrixFreeSessionSnapshot,
    run_audit2_matrix_free_common_w_correction,
};
#[cfg(feature = "audit2-research")]
pub use audit2_reusable_transaction_research::{
    Audit2ExternalOutputReference, Audit2FrozenWSemanticIdentity, Audit2IndependentBudgetReceipt,
    Audit2IndependentStepBudget, Audit2ReferenceAwareOutputAssessment,
    Audit2ReferenceUncertaintyTreatment, Audit2ReusablePreconditionerBinding,
    Audit2ReusablePreconditionerCache, Audit2ReusablePreconditionerCacheSnapshot,
    Audit2ReusablePreconditionerIdentity, Audit2TransactionalAttemptConfig,
    Audit2TransactionalAttemptFailure, Audit2TransactionalAttemptOutcome,
    Audit2TransactionalAttemptSuccess, Audit2TransactionalCandidateReceipt,
    Audit2TransactionalFailurePhase, Audit2TransactionalSelection,
    assess_audit2_reference_aware_output, audit2_conservative_l2_difference_upper,
    audit2_conservative_output_budget_lower,
    run_audit2_reusable_preconditioner_transactional_attempt,
};

pub use adaptive::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveObservedIntegrationResult,
    AdaptiveRunDiagnostics, AdaptiveStepConfig, ControllerKind, RODAS5P_ESTIMATOR_ORDER,
    StepDoublingEstimate, adaptive_next_step_after_attempt, rodas_next_step_after_attempt,
    step_doubling_wrms_error,
};
pub use adaptive_exponential::{
    AdaptiveEarlyFlowDefectAttempt, AdaptiveEarlyFlowDefectOutcome,
    AdaptiveFusedExponentialDiagnostics, AdaptiveFusedExponentialResult,
    integrate_pexprb54s4_fused_adaptive_observed,
    integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode,
    integrate_pexprb54s4_fused_adaptive_observed_with_tolerance_scaled_telemetry,
};
pub use bdf::{
    BDF2_ZERO_STABILITY_RATIO_MAX, BdfConfig, BdfHistory, BdfIntegrationResult, BdfOrder,
    BdfStepReport, VariableBdf2Coefficients, bdf_step, bdf_step_variable,
    bdf1_predictor_correction_lte_factor, bdf2_predictor_correction_lte_factor,
    integrate_bdf_adaptive_observed, integrate_bdf_fixed, integrate_bdf_fixed_observed,
    variable_bdf2_coefficients, variable_bdf2_predictor,
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
pub use dense_output_v2::{
    DenseOutputError, DenseOutputResult, bdf_dense_output,
    integrate_adaptive_dense_observed_with_config, integrate_bdf_adaptive_dense_observed,
    integrate_bdf_fixed_dense_observed, integrate_fixed_dense_observed,
    integrate_homotopy_adaptive_dense_observed, integrate_radau_adaptive_dense_observed,
    integrate_radau_fixed_dense_observed, integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_transactional_q1_q2_adaptive_dense_observed, radau_dense_output,
    rodas5p_dense_output,
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
    G4S5B0LinearToleranceArm, RODAS5P_INNER_FORCING_CLAIM_SCOPE, RODAS5P_INNER_FORCING_ETA_MAX,
    RODAS5P_INNER_FORCING_FLOOR, RODAS5P_INNER_RESIDUAL_HEURISTIC_FRACTION,
    Rodas5pInnerForcingClaimScope, Rodas5pInnerForcingTarget,
    committed_g4_s5b0_linear_tolerance_arm, rodas5p_inner_forcing_target,
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
pub use output::{ObservedIntegrationResult, OutputSamplingPlan, OutputSchedule};
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
    RadauConfig, RadauIia3TransformOracle, RadauIiaStages, RadauIntegrationResult,
    RadauStageSolveArchitecture, RadauStepReport, RadauTransformLimitation,
    integrate_radau_adaptive_observed, integrate_radau_fixed, integrate_radau_fixed_observed,
    radau_iia3_tableau, radau_iia3_transform_oracle, radau_step,
};
pub use rhs_telemetry::{
    BackendRecommendationSummary, CommonWBackendChoice, HomotopyRhsTelemetryCase,
    HomotopyRhsTelemetryProfile, HomotopyRhsTelemetryReport, RhsBatchAnalysis,
    RhsBatchTelemetryRow, RhsSubspaceComparison, RhsTelemetryFailure, RhsTelemetryRisk,
    analyze_rhs_batch, analyze_rhs_directions, compare_rhs_subspaces, recommend_common_w_backend,
    run_homotopy_rhs_telemetry_screen,
};
pub use sabr::{PredictorKind, SabrConfig, StageHistory, sabr_step};
pub use scientific_corpus_v2::{
    CorpusPartition, ScientificCaseSpec, ScientificCorpusV2, ScientificFamily,
    ScientificProblemCase, ScientificProblemSegment, ScientificSourceProvenance,
    v2_diversity_multiplier,
};
pub use scientific_validity_v2_gate::{
    V2_THRESHOLD_DERIVATION_ID, V2CalibrationFreezeEnvelope, V2CalibrationFreezePayload,
    V2CampaignBinding, V2EvidenceAuthority, V2GateProfile, V2GateRow, V2GateRowStatus,
    V2OregonatorReplayEnvelope, V2OregonatorReplayPayload, V2OregonatorReplayRow,
    V2RowEvidenceBinding, freeze_v2_calibration, replay_v2_oregonator_holdout,
    v2_calibration_payload_checksum, v2_oregonator_replay_payload_checksum,
    verify_v2_calibration_freeze, verify_v2_oregonator_replay,
};
pub use sequential::{
    InnerForcedStageSolveData, InnerForcedStepResult, KrylovState, StageInnerForcingReport,
    StageSolveData, StepCertificate, StepContext, StepResult, build_step_context,
    build_step_context_matrix_free, finish_step, sequential_matrix_free_step,
    sequential_matrix_free_step_with_inner_forcing, sequential_stages,
    sequential_stages_with_inner_forcing, sequential_step,
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

pub use a1_two_arm_receipt::{
    A1_TWO_ARM_RECEIPT_PROFILE, A1_TWO_ARM_RECEIPT_SCHEMA, A1ScientificExecutionIdentity,
    A1ToleranceReceiptCell, A1ToleranceReceiptEventRow, A1ToleranceReceiptRecommendationRow,
    run_a1_two_arm_receipt_cell,
};
pub use policy_redesign_v25::{
    CausalRjfStep, PersistenceLatch, PointFeature, PrefixBudget, ProbeAction, causal_feature_value,
};
