#![forbid(unsafe_code)]

mod block;
mod candidates;
mod certification;
mod homotopy;
mod homotopy_experiments;
mod homotopy_order_policy;
mod homotopy_policy;
mod integrate;
mod nonlinear;
mod parallel;
mod problem;
mod problems;
mod sabr;
mod sequential;
mod unified_gates;
mod unified_screen;

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
pub use homotopy::{
    AffineOutputCertificate, AffinePartialCouplingOracle, HomotopyDesignCheckReport,
    HomotopyPathConfig, HomotopyPathPoint, HomotopyPathReport, HomotopyPredictor,
    HomotopyStepConfig, HomotopyStepReport, HomotopyWorkLedger, NonlinearOutputCertificate,
    NonnormalConditionRow, PartialCouplingParameters, PowerNormRow, TruncationScreenRow,
    certify_nonlinear_target, homotopy_step, run_fixed_homotopy_path, run_homotopy_design_check,
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
pub use integrate::{IntegrationMethod, IntegrationResult, integrate_adaptive, integrate_fixed};
pub use nonlinear::{NewtonConfig, NewtonReport, solve_dense_newton};
pub use parallel::ParallelExecution;
pub use problem::OdeProblem;
pub use problems::{
    constant_affine_mass_problem, manufactured_mass_nonlinear_problem, manufactured_vector_problem,
    prothero_robinson_problem, scalar_linear_problem,
};
pub use sabr::{PredictorKind, SabrConfig, StageHistory, sabr_step};
pub use sequential::{
    KrylovState, StageSolveData, StepCertificate, StepContext, StepResult, build_step_context,
    finish_step, sequential_stages, sequential_step,
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
