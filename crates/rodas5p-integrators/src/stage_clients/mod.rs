//! Opt-in stage linear-system clients for the protected fast RODAS5P driver.
//!
//! A client implements the `StageLinearSystem` trait of `client_stage.rs` and is passed to
//! `integrate_rodas5p_fast_client_observed_with_atol`. The driver keeps its stages, controller
//! and output; a client owns linearisation, factorisation and the stage solves. Nothing in this
//! module is selected by default.

pub mod assembly;
pub mod banded_tight;
pub mod dense_lu;
pub mod dense_lu_reuse;
pub mod krylov;
pub mod sparse_lu;
pub mod speculative;
pub mod timed;

pub use banded_tight::BandedTightStageLu;
pub use dense_lu::{DenseLuMode, DenseLuStageClient};
pub use timed::{PhaseTimes, PhaseTimesHandle, TimedStageClient};
