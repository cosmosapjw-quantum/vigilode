use super::*;
use rodas5p_core::{CoreError, CoreResult, directed::Interval as Iv};
use serde::Serialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SpatialAdjointRoute {
    Taylor18,
    Poisson18,
    PoissonAdaptive,
    ModeResidual,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SpatialAdjointWork {
    pub cells: usize,
    pub actions: usize,
    pub max_order: usize,
    pub l1_tail_upper: f64,
    pub mode_rate: f64,
    pub width_upper: f64,
    pub exact_goal: [f64; 2],
    pub signed_goal_error: [f64; 2],
}
pub struct SpatialAdjointWorkspace {
    grid: DiffusionGrid,
}
impl SpatialAdjointWorkspace {
    pub fn new(grid: &DiffusionGrid) -> CoreResult<Self> {
        Ok(Self { grid: grid.clone() })
    }
    pub fn certify_into(
        &mut self,
        _goal: &[f64],
        _initial: &[f64],
        _endpoint: &[f64],
        _duration: f64,
        _route: SpatialAdjointRoute,
        _out: &mut [Iv],
    ) -> CoreResult<SpatialAdjointWork> {
        let _ = self.grid.dimension();
        Err(CoreError::InvalidInput(
            "spatial adjoint certificate not implemented".into(),
        ))
    }
}
