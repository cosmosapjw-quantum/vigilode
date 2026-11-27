use super::*;
use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use serde::Serialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ShiftedAdjointRoute {
    Identity,
    Longitude,
    Meridional,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ShiftedAdjointReport {
    pub residual_l1: f64,
    pub error_l1: f64,
    pub iterations: u64,
}
pub struct ShiftedAdjointWorkspace {
    grid: DiffusionGrid,
}
impl ShiftedAdjointWorkspace {
    pub fn new(grid: &DiffusionGrid, _route: ShiftedAdjointRoute) -> CoreResult<Self> {
        Ok(Self { grid: grid.clone() })
    }
    pub fn prepare(&mut self, _z: f64) -> CoreResult<()> {
        Err(CoreError::InvalidInput(
            "shifted adjoint not implemented".into(),
        ))
    }
    pub fn solve_into(
        &mut self,
        _b: &[f64],
        _out: &mut [f64],
        _c: &mut WorkCounters,
    ) -> CoreResult<ShiftedAdjointReport> {
        let _ = self.grid.dimension();
        Err(CoreError::InvalidInput(
            "shifted adjoint not implemented".into(),
        ))
    }
}
