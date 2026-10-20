#![allow(dead_code)]
#[path="model.rs"] mod model;
#[path="bounds.rs"] mod bounds;
#[path="trajectory.rs"] mod trajectory;
pub use model::{DerivativeMode,Model,ShiftFactor,Work};
pub use bounds::{Curve,Segment,Iv,quintic_controls};
pub use trajectory::{integrate,Integration,IntegrationConfig,Trial,Endpoint};
pub type Result<T>=std::result::Result<T,String>;
pub(crate) fn finite(v:&[f64])->Result<()>{if v.iter().all(|x|x.is_finite()){Ok(())}else{Err("NONFINITE_VALUE".into())}}
