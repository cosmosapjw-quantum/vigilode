#![allow(dead_code)]
#[path = "bounds.rs"]
mod bounds;
#[path = "model.rs"]
mod model;
#[path = "trajectory.rs"]
mod trajectory;
pub use bounds::{Curve, Iv, Segment, quintic_controls};
pub use model::{DerivativeMode, Model, ShiftFactor, Work};
pub use trajectory::{Endpoint, Integration, IntegrationConfig, Trial, integrate};
pub type Result<T> = std::result::Result<T, String>;
pub(crate) fn finite(v: &[f64]) -> Result<()> {
    if v.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err("NONFINITE_VALUE".into())
    }
}
