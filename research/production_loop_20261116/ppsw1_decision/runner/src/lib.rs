//! Loop L1 node V10_PPSW1_DECISION: a research copy of PPSW1 (`ppsw1`), the PeerOde step loop
//! over it (`driver`), the declared problems (`problems`) and helpers of the measurement program.
//!
//! The research copy and the byte copy of its linear backend name `crate::OdeProblem` and
//! `crate::ParallelExecution`; the re-exports below make those paths resolve to the crate types,
//! so neither file needs an import change.

pub use rodas5p_integrators::{OdeProblem, ParallelExecution};

pub mod driver;
pub mod ppsw1;
pub mod problems;
pub mod util;
