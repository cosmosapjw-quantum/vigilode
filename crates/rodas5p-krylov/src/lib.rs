#![forbid(unsafe_code)]

mod block_gmres;
mod common;
mod gcrodr;
mod gmres;
mod gmres_givens;
mod gmres_into;
mod kernels;
mod lgmres;
mod lgmres_into;
mod small;
mod workspace;

pub use block_gmres::{
    BlockGmresConfig, BlockLinearSolveReport, SeededGmresConfig, solve_block_gmres,
    solve_seeded_gmres,
};
pub use gcrodr::{
    GcrodrConfig, GcrodrCycleTrace, GcrodrReuseCheck, GcrodrReusePolicy, GcrodrSolveOptions,
    GcrodrState, GcrodrTrace, solve_gcrodr, solve_gcrodr_traced, solve_gcrodr_with_options,
    solve_gcrodr_with_policy, solve_gcrodr_with_residual_scale, solve_gcrodr_with_workspace,
    solve_gcrodr_with_workspace_and_residual_scale,
};
pub use gmres::{
    GmresConfig, GmresPrefixPrediction, GmresPrefixSession, solve_gmres, solve_gmres_incremental,
    solve_gmres_with_residual_scale, solve_gmres_with_workspace,
    solve_gmres_with_workspace_and_residual_scale,
};
pub use gmres_givens::{
    GmresGivensStatistics, GmresGivensWorkspace, solve_gmres_givens,
    solve_gmres_givens_with_residual_scale, solve_gmres_givens_with_workspace,
    solve_gmres_givens_with_workspace_and_residual_scale,
};
pub use gmres_into::{CapacityGrowth, GmresCapacity, GmresIntoReport, solve_gmres_into};
pub use lgmres::{
    LgmresConfig, LgmresState, solve_lgmres, solve_lgmres_with_residual_scale,
    solve_lgmres_with_workspace, solve_lgmres_with_workspace_and_residual_scale,
};
pub use lgmres_into::{LgmresIntoReport, LgmresIntoWorkspace, solve_lgmres_into};
pub use workspace::{GcrodrWorkspace, GmresWorkspace, LgmresWorkspace};
