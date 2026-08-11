#![forbid(unsafe_code)]

mod adapters;
mod adaptive_global_error;
mod budget;
mod contracts;
mod error;
mod global_error;
mod runner;
mod scenarios;
mod session;

pub use adapters::solve_case;
pub use adaptive_global_error::{
    AdaptiveCandidateDescriptor, AdaptiveCandidateFamily, AdaptiveGlobalErrorReport,
    AdaptiveProblemDescriptor, AdaptiveRunRow, AdaptiveScreenExecution,
    run_adaptive_global_error_screen, run_g1_adaptive_global_error_screen,
};
pub use budget::{BUDGET_EXHAUSTED_MARKER, BudgetedOperator, StableDenseOperator};
pub use contracts::{
    FairSolveConfig, FairSolveResult, LinearSystemCase, PreconditionerKind, RecycleLifetime,
    ResidualCertificate, SequenceKind, SolveStatus, SolverKind, TimingLedger, WorkLedger,
};
pub use error::{FairError, FairResult};
pub use runner::{
    BenchmarkCell, BenchmarkPlan, ComparisonResult, SummaryRow, TraceRunResult,
    build_execution_order, run_comparison, run_trace, summarize_comparison,
};
pub use scenarios::{
    CaseDocument, LinearSystemTrace, SequenceConfig, TraceDocument, generate_trace,
};
pub use session::{RecycleSessionManager, SolverSession, StateTransition};

pub(crate) use contracts::relative_solution_error;

pub use global_error::{
    CommonOutputGrid, ExternalErrorScale, FixedAnchorCandidate, GlobalErrorMetric,
    GlobalErrorMetrics, GlobalErrorParetoFront, GlobalErrorParetoProfile, GlobalErrorParetoReport,
    GlobalErrorReport, GlobalErrorRunRow, GlobalErrorTarget, GlobalRunStatus, IntegratorRunRecord,
    IntegratorRunStatus, IntegratorTimingReport, IntegratorWorkReport, OutputPolicyMetadata,
    ParetoCostMetric, ParetoFront, ParetoObservation, ParetoObservationStatus,
    ReferenceSolutionProvenance, ReferenceSourceKind, ReferenceTrajectory, TargetAttainment,
    TimingProtocol, compute_global_error_metrics, nondominated_observation_ids,
    run_global_error_pareto_screen, select_cheapest_below_target,
};
