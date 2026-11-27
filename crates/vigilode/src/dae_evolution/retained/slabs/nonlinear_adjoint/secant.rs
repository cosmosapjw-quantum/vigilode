use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SecantGoalRoute {
    WholeHorner,
    Sub4Horner,
    Sub4Bernstein,
    Diagonal4,
    Scaled4,
    Scaled16,
    Shifted4,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SecantGoalWork {
    pub cells: usize,
    pub signed_error: [f64; 2],
    pub width_upper: f64,
}
pub struct SecantGoalWorkspace;
impl SecantGoalWorkspace {
    pub fn new() -> Self {
        Self
    }
}
impl SlabReplayRun {
    pub fn secant_goal_into(
        &self,
        _weights: &[f64],
        _route: SecantGoalRoute,
        _w: &mut SecantGoalWorkspace,
        _out: &mut Iv,
    ) -> CoreResult<SecantGoalWork> {
        Err(bad("secant goal residual not implemented"))
    }
}
