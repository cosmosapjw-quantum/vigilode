#![forbid(unsafe_code)]

mod adapters;
mod budget;
mod contracts;
mod error;
mod runner;
mod scenarios;
mod session;

pub use adapters::solve_case;
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
