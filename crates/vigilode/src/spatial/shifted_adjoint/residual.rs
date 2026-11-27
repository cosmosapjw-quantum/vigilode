use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ResidualPlan {
    Legacy,
    PackedInterval,
    FusedEdges,
    FusedDiagonal,
    Hybrid,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ResidualReceipt {
    pub residual_l1: f64,
    pub error_l1: f64,
    pub rhs_l1: f64,
    pub rounding_upper: f64,
    pub admitted: bool,
    pub fast_checks: u64,
    pub interval_checks: u64,
    pub fallbacks: u64,
    pub fmas: u64,
}
pub struct ResidualWorkspace {
    grid: DiffusionGrid,
    z: Option<f64>,
    scratch: Vec<Iv>,
}
impl ResidualWorkspace {
    pub fn new(g: &DiffusionGrid, _plan: ResidualPlan) -> CoreResult<Self> {
        Ok(Self {
            grid: g.clone(),
            z: None,
            scratch: vec![Z; g.dimension()],
        })
    }
    pub fn prepare(&mut self, z: f64) -> CoreResult<()> {
        self.z = None;
        if !z.is_finite() || z <= 0. {
            return Err(err("shift"));
        }
        self.z = Some(z);
        Ok(())
    }
    pub fn evaluate(&mut self, b: &[f64], x: &[f64]) -> CoreResult<ResidualReceipt> {
        let (r, e, s) = directed_residual(
            &self.grid,
            self.z.ok_or_else(|| err("not prepared"))?,
            b,
            x,
            &mut self.scratch,
        )?;
        Ok(ResidualReceipt {
            residual_l1: r,
            error_l1: e,
            rhs_l1: s,
            admitted: r <= add_up(mul_up(1e-10, s)?, 1e-13)?,
            interval_checks: 1,
            ..Default::default()
        })
    }
}
