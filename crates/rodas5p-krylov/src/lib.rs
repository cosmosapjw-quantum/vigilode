#![forbid(unsafe_code)]

mod block_gmres;
mod common;
mod gcrodr;
mod gmres;
mod gmres_givens;
mod kernels;
mod lgmres;
mod small;
mod workspace;

pub use block_gmres::{
    BlockGmresConfig, BlockLinearSolveReport, SeededGmresConfig, solve_block_gmres,
    solve_seeded_gmres,
};
pub use gcrodr::{
    GcrodrConfig, GcrodrState, solve_gcrodr, solve_gcrodr_observed, solve_gcrodr_with_workspace,
};
pub use gmres::{
    GmresConfig, GmresPrefixPrediction, GmresPrefixSession, solve_gmres, solve_gmres_incremental,
    solve_gmres_observed, solve_gmres_with_workspace,
};
pub use gmres_givens::{
    GmresGivensStatistics, GmresGivensWorkspace, solve_gmres_givens, solve_gmres_givens_observed,
    solve_gmres_givens_with_workspace,
};
pub use lgmres::{
    LgmresConfig, LgmresState, solve_lgmres, solve_lgmres_observed, solve_lgmres_with_workspace,
};
pub use workspace::{GcrodrWorkspace, GmresWorkspace, LgmresWorkspace};

/// Research-only metadata copied from residuals already computed by a linear solve.
///
/// This type is intentionally opt-in and does not participate in convergence decisions.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LinearSolveObservation {
    pub initial_guess_source: Option<ObservedInitialGuessSource>,
    pub initial_true_residual_norm: Option<f64>,
    pub final_true_residual_norm: Option<f64>,
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedInitialGuessSource {
    Zero,
    ExternalPrevious,
    InternalLgmresPrevious,
    InternalGcrodrPrevious,
    RecycleProjection,
    Direct,
}

impl LinearSolveObservation {
    pub(crate) fn reset(&mut self, source: ObservedInitialGuessSource) {
        self.initial_guess_source = Some(source);
        self.initial_true_residual_norm = None;
        self.final_true_residual_norm = None;
    }

    pub(crate) fn record_initial_once(&mut self, residual_norm: f64) {
        if self.initial_true_residual_norm.is_none() {
            self.initial_true_residual_norm = Some(residual_norm);
        }
    }

    pub(crate) fn record_recycle_projection(&mut self, residual_norm: f64) {
        if self.initial_guess_source != Some(ObservedInitialGuessSource::RecycleProjection) {
            self.initial_guess_source = Some(ObservedInitialGuessSource::RecycleProjection);
            self.initial_true_residual_norm = Some(residual_norm);
        }
    }

    pub(crate) fn record_final(&mut self, residual_norm: f64) {
        self.final_true_residual_norm = Some(residual_norm);
    }
}
