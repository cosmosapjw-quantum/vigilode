//! Generated-model hybrid research integration. Model-specific continuous
//! certificates do not extend to arbitrary callbacks, meshes or PDE errors.
#![forbid(unsafe_code)]
mod kernel;
pub use kernel::*;
#[cfg(feature="native")]
mod native;
#[cfg(feature="native")]
pub use native::{NativeRun,solve_trajectory};
