#![forbid(unsafe_code)]

mod adapters;
mod adaptive_global_error;
mod budget;
mod contracts;
mod error;
mod external_comparators;
mod global_error;
mod numerical_reference;
mod runner;
mod scenarios;
mod scientific_validity_v2_campaign;
mod session;

pub use adapters::solve_case;
pub use adaptive_global_error::{
    AdaptiveCandidateDescriptor, AdaptiveCandidateFamily, AdaptiveGlobalErrorReport,
    AdaptiveOutputMode, AdaptiveOutputPolicyPairRecord, AdaptiveOutputPolicyPairStatus,
    AdaptiveProblemDescriptor, AdaptiveRunRow, AdaptiveScreenExecution,
    run_adaptive_global_error_screen, run_g1_adaptive_global_error_screen,
};
pub use budget::{BUDGET_EXHAUSTED_MARKER, BudgetedOperator, StableDenseOperator};
pub use contracts::{
    FairSolveConfig, FairSolveResult, LinearSystemCase, PreconditionerKind, RecycleLifetime,
    ResidualCertificate, SequenceKind, SolveStatus, SolverKind, TimingLedger, WorkLedger,
};
pub use error::{FairError, FairResult};
pub use external_comparators::{
    EXTERNAL_COMPARATOR_EVIDENCE_SCHEMA_VERSION, ExternalComparatorContract,
    ExternalComparatorEvidence, ExternalComparatorKind, ExternalDenseOutputPolicy,
    ExternalEvidenceChecksums, ExternalMassTreatment, ExternalNativeWork, ExternalProblemBinding,
    ExternalReferenceDependency, ExternalRunStatus, ExternalRunnerBinding,
    ExternalRunnerDependency, ExternalRuntimeIdentity, ExternalToleranceBinding,
    SundialsProbeFinding, external_runner_dependency_closure_checksum,
    external_runtime_identity_checksum, load_external_comparator_evidence,
    sundials_probe_evidence_checksum,
};
pub use runner::{
    BenchmarkCell, BenchmarkPlan, ComparisonResult, SummaryRow, TraceRunResult,
    build_execution_order, run_comparison, run_trace, summarize_comparison,
};
pub use scenarios::{
    CaseDocument, LinearSystemTrace, SequenceConfig, TraceDocument, generate_trace,
};
pub use scientific_validity_v2_campaign::{
    SCIENTIFIC_VALIDITY_V2_CAMPAIGN_SCHEMA, SCIENTIFIC_VALIDITY_V2_CANDIDATE_ID,
    SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM, SCIENTIFIC_VALIDITY_V2_RUNNER_SCHEMA,
    ScientificValidityV2CaseArtifact, V2CampaignArmEvidence, V2CampaignArmStatus,
    V2CampaignOutputMode, V2CampaignReferenceBinding, V2Rodas5pCampaignConfig,
    freeze_scientific_validity_v2_calibration_artifacts,
    replay_scientific_validity_v2_oregonator_artifacts, run_scientific_validity_v2_case,
    run_scientific_validity_v2_case_synthetic_smoke, scientific_validity_v2_campaign_specs,
    scientific_validity_v2_canonical_campaign_binding, scientific_validity_v2_compiled_revision,
    scientific_validity_v2_detected_revision, scientific_validity_v2_source_dirty_at_build,
    validate_scientific_validity_v2_case_artifact,
};
pub use session::{RecycleSessionManager, SolverSession, StateTransition};

pub(crate) use contracts::relative_solution_error;

pub use global_error::{
    CommonOutputGrid, DualOutputPolicyEvidence, ExternalErrorScale, FixedAnchorCandidate,
    GlobalErrorMetric, GlobalErrorMetrics, GlobalErrorParetoFront, GlobalErrorParetoProfile,
    GlobalErrorParetoReport, GlobalErrorReport, GlobalErrorRunRow, GlobalErrorTarget,
    GlobalRunStatus, IntegratorRunRecord, IntegratorRunStatus, IntegratorTimingReport,
    IntegratorWorkReport, OutputPolicyDominance, OutputPolicyMetadata, OutputPolicyPairRecord,
    OutputPolicyRunEvidence, ParetoCostMetric, ParetoFront, ParetoObservation,
    ParetoObservationStatus, ReferenceSolutionProvenance, ReferenceSourceKind, ReferenceTrajectory,
    ReferenceWrmsBasis, TargetAttainment, TimingProtocol, apply_output_policy_dominance,
    classify_output_policy_dominance, compute_global_error_metrics, nondominated_observation_ids,
    run_global_error_pareto_screen, select_cheapest_below_target,
};
pub use numerical_reference::{
    NUMERICAL_REFERENCE_ARTIFACT_SCHEMA_VERSION, NUMERICAL_REFERENCE_MANIFEST_SCHEMA_VERSION,
    NUMERICAL_REFERENCE_V2_ARTIFACT_SCHEMA_VERSION, NUMERICAL_REFERENCE_V2_MANIFEST_SCHEMA_VERSION,
    NUMERICAL_REFERENCE_V2_WRMS_FORMULA_ID, NumericalReferenceArtifact,
    NumericalReferenceArtifactV2, NumericalReferenceBundle, NumericalReferenceBundleV2,
    NumericalReferenceCaseBindingV2, NumericalReferenceChecksums, NumericalReferenceConvergence,
    NumericalReferenceConvergenceV2, NumericalReferenceGenerationStatusV2,
    NumericalReferenceGeneratorPins, NumericalReferenceManifest, NumericalReferenceManifestEntry,
    NumericalReferenceManifestEntryV2, NumericalReferenceManifestV2, NumericalReferenceMethod,
    NumericalReferenceProblem, NumericalReferenceProblemV2, NumericalReferenceProducerIdentityV2,
    NumericalReferenceProvenance, NumericalReferenceRunEvidenceV2, NumericalReferenceRunStatusV2,
    NumericalReferenceRuntimeIdentityV2, NumericalReferenceRuntimeLibraryV2,
    NumericalReferenceSourceDefinition, NumericalReferenceWrmsBasisV2,
    NumericalReferenceWrmsPolicyV2, NumericalReferenceWrmsScale, ReferenceDominance,
    classify_reference_dominance, load_numerical_reference, load_numerical_reference_v2,
    numerical_reference_artifact_set_checksum, numerical_reference_artifact_set_checksum_v2,
    numerical_reference_binding_set_checksum_v2, numerical_reference_case_binding_checksum_v2,
    numerical_reference_grid_checksum, numerical_reference_problem_definition_checksum_v2,
    numerical_reference_state_checksum, numerical_reference_v2_not_run_manifest,
    validate_numerical_reference_convergence, validate_numerical_reference_error_scale,
    validate_numerical_reference_manifest, validate_numerical_reference_manifest_v2,
};
