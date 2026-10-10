#![forbid(unsafe_code)]

mod accounting;
mod block_gmres;
mod common;
mod gcrodr;
mod gmres;
mod gmres_givens;
mod gmres_into;
mod gmres_staged;
mod kernels;
mod lgmres;
mod lgmres_into;
mod small;
mod workspace;

pub use accounting::ResidualAccounting;
pub use block_gmres::{
    BlockGmresConfig, BlockLinearSolveReport, SeededGmresConfig, solve_block_gmres,
    solve_seeded_gmres,
};
pub use gcrodr::{
    GcrodrConfig, GcrodrCycleTrace, GcrodrReuseCheck, GcrodrReusePolicy, GcrodrSolveOptions,
    GcrodrState, GcrodrTrace, solve_gcrodr, solve_gcrodr_traced, solve_gcrodr_with_options,
    solve_gcrodr_with_policy, solve_gcrodr_with_residual_scale, solve_gcrodr_with_workspace,
    solve_gcrodr_with_workspace_and_options, solve_gcrodr_with_workspace_and_residual_scale,
};
pub use gmres::{
    GmresConfig, GmresPrefixPrediction, GmresPrefixSession, solve_gmres, solve_gmres_incremental,
    solve_gmres_with_accounting, solve_gmres_with_residual_scale, solve_gmres_with_workspace,
    solve_gmres_with_workspace_and_accounting, solve_gmres_with_workspace_and_residual_scale,
};
pub use gmres_givens::{
    GmresGivensStatistics, GmresGivensWorkspace, solve_gmres_givens,
    solve_gmres_givens_with_residual_scale, solve_gmres_givens_with_workspace,
    solve_gmres_givens_with_workspace_and_residual_scale,
};
pub use gmres_into::{
    CapacityGrowth, GmresCapacity, GmresIntoOptions, GmresIntoReport, solve_gmres_into,
    solve_gmres_into_with_accounting, solve_gmres_into_with_options,
};
pub use gmres_staged::{
    STAGED_EXHAUSTION_FLOOR, STAGED_GUARD_Q_ABORT, STAGED_STALL_CONTRACTION, STAGED_STALL_FACTOR,
    StagedGmresConfig, StagedGmresFailure, StagedGmresFallback, StagedGmresOutcome,
    StagedGmresReport, StagedGmresWorkspace, StagedGuardAbort, solve_staged_gmres,
    staged_guard_test,
};
pub use lgmres::{
    LgmresConfig, LgmresState, solve_lgmres, solve_lgmres_with_residual_scale,
    solve_lgmres_with_workspace, solve_lgmres_with_workspace_and_residual_scale,
};
pub use lgmres_into::{LgmresIntoReport, LgmresIntoWorkspace, solve_lgmres_into};
/// `least_squares` is exported as the bitwise reference of
/// `LeastSquaresWorkspace` (research node `research/spd04_ls_workspace_20261007`).
pub use small::{LeastSquaresWorkspace, least_squares};
pub use workspace::{GcrodrWorkspace, GmresWorkspace, LgmresWorkspace};
