//! Immutable Gray–Scott continuous-residual capability.
//!
//! The factory owns the point callbacks, periodic stencil, represented scalar
//! constants and tube provider. A public callback or supplied box cannot mint
//! this capability. The generic driver verifies a complete continuous tube
//! before committing each segment and inherited radius.
use rodas5p_core::directed::{Interval, add_up};
use rodas5p_core::{CoreError, CoreResult};

use crate::OdeProblem;
use crate::dense_output_v2::{
    ContinuousResidualProvider, ContinuousResidualVerificationResult, ContinuousResidualWorkspace,
    ContinuousSegmentErrorBound, DenseOutputResult, DenseSegmentWork, Rodas5pQuarticSegment,
};
use crate::generic_verified::{
    VerifiedContinuousProblem, VerifiedModelProvider, VerifiedResidualError, VerifiedTube,
};
use crate::gray_scott::{GrayScottLogNorm, gray_scott_2d_with_log_norm};

/// Generated immutable Gray–Scott model. There is no public constructor that
/// accepts independently supplied point callbacks or a logarithmic norm.
///
/// ```compile_fail
/// # use rodas5p_integrators::gray_scott_verified::BoundGrayScottProblem;
/// fn replace_callbacks(owner: &mut BoundGrayScottProblem) {
///     owner.problem.rhs = unreachable!();
/// }
/// ```
pub struct BoundGrayScottProblem {
    problem: OdeProblem,
    log_norm: GrayScottLogNorm,
    inv_dx2: f64,
}

impl BoundGrayScottProblem {
    /// Return the generated owner and its default nonuniform initial center.
    /// Constants are exactly the binary64 values used by `gray_scott_2d`.
    pub fn new(n_grid: usize) -> CoreResult<(Self, Vec<f64>)> {
        let (problem, initial, log_norm) = gray_scott_2d_with_log_norm(n_grid)?;
        let dx = 1.0 / n_grid as f64;
        let inv_dx2 = 1.0 / (dx * dx);
        Ok((
            Self {
                problem,
                log_norm,
                inv_dx2,
            },
            initial,
        ))
    }

    pub fn grid_size(&self) -> usize {
        self.log_norm.grid_size()
    }
    pub fn dimension(&self) -> usize {
        self.problem.dimension
    }

    pub fn into_verified(self) -> VerifiedContinuousProblem {
        VerifiedContinuousProblem::from_provider(self)
    }

    pub fn verify_continuous_residual_tube(
        &self,
        curve: &Rodas5pQuarticSegment,
        initial_radius: f64,
        tube_radius: f64,
        workspace: &mut ContinuousResidualWorkspace,
        work: &mut DenseSegmentWork,
    ) -> ContinuousResidualVerificationResult<ContinuousSegmentErrorBound> {
        curve.verify_continuous_residual_tube(self, initial_radius, tube_radius, workspace, work)
    }
}

/// Construct a Gray–Scott model admitted by `integrate_verified_residual`.
pub fn gray_scott_2d_verified(n_grid: usize) -> CoreResult<(VerifiedContinuousProblem, Vec<f64>)> {
    let (owner, initial) = BoundGrayScottProblem::new(n_grid)?;
    Ok((owner.into_verified(), initial))
}

macro_rules! op {
    ($work:expr, $value:expr) => {{
        $work.provider_operations = $work.provider_operations.saturating_add(1);
        $value
    }};
}

fn validate(
    owner: &BoundGrayScottProblem,
    time: Interval,
    state: &[Interval],
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<()> {
    work.validation_operations = work.validation_operations.saturating_add(1);
    if state.len() != owner.dimension() {
        return Err(CoreError::Dimension("Gray-Scott interval shape mismatch".into()).into());
    }
    op!(work, Interval::new(time.lo, time.hi))?;
    for &value in state {
        work.provider_component_items = work.provider_component_items.saturating_add(1);
        op!(work, Interval::new(value.lo, value.hi))?;
    }
    Ok(())
}

fn stencil(
    values: &[Interval],
    idx: usize,
    n_grid: usize,
    inv_dx2: f64,
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<Interval> {
    let i = idx % n_grid;
    let j = idx / n_grid;
    let left = if i > 0 { i - 1 } else { n_grid - 1 } + j * n_grid;
    let right = if i + 1 < n_grid { i + 1 } else { 0 } + j * n_grid;
    let up = i + if j + 1 < n_grid { j + 1 } else { 0 } * n_grid;
    let down = i + if j > 0 { j - 1 } else { n_grid - 1 } * n_grid;
    let sum = op!(work, values[left].add(values[right]))?;
    let sum = op!(work, sum.add(values[up]))?;
    let sum = op!(work, sum.add(values[down]))?;
    let center = op!(work, values[idx].scale(4.0))?;
    let lap = op!(work, sum.sub(center))?;
    Ok(op!(work, lap.scale(inv_dx2))?)
}

impl ContinuousResidualProvider for BoundGrayScottProblem {
    fn enclose_rhs(
        &self,
        time: Interval,
        state: &[Interval],
        rhs: &mut [Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<()> {
        validate(self, time, state, work)?;
        work.validation_operations = work.validation_operations.saturating_add(1);
        if rhs.len() != self.dimension() {
            return Err(
                CoreError::Dimension("Gray-Scott interval RHS shape mismatch".into()).into(),
            );
        }
        let cells = self.dimension() / 2;
        let (u, v) = state.split_at(cells);
        let (out_u, out_v) = rhs.split_at_mut(cells);
        let one = op!(work, Interval::point(1.0))?;
        for i in 0..cells {
            work.provider_component_items = work.provider_component_items.saturating_add(2);
            let lap_u = stencil(u, i, self.grid_size(), self.inv_dx2, work)?;
            let lap_v = stencil(v, i, self.grid_size(), self.inv_dx2, work)?;
            let uv = op!(work, u[i].mul(v[i]))?;
            let uv2 = op!(work, uv.mul(v[i]))?;
            let diffusion_u = op!(work, lap_u.scale(2.0e-3))?;
            let diffusion_v = op!(work, lap_v.scale(1.0e-3))?;
            let feed = op!(work, one.sub(u[i]))?;
            let feed = op!(work, feed.scale(0.04))?;
            let loss = op!(work, v[i].scale(0.04 + 0.06))?;
            let du = op!(work, diffusion_u.sub(uv2))?;
            out_u[i] = op!(work, du.add(feed))?;
            let dv = op!(work, diffusion_v.add(uv2))?;
            out_v[i] = op!(work, dv.sub(loss))?;
        }
        Ok(())
    }

    fn one_sided_log_norm_upper(
        &self,
        time: Interval,
        tube: &[Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<f64> {
        validate(self, time, tube, work)?;
        let (u, v) = tube.split_at(self.dimension() / 2);
        // Every attempted scalar primitive is charged inside the block loop,
        // including a prefix ending in an overflow refusal.
        Ok(self
            .log_norm
            .verified_log_norm_upper_counted(u, v, &mut work.provider_operations)?)
    }
}

impl VerifiedModelProvider for BoundGrayScottProblem {
    fn problem(&self) -> &OdeProblem {
        &self.problem
    }
    fn validate_span(&self, span: (f64, f64)) -> Result<(), VerifiedResidualError> {
        if !span.0.is_finite() || !span.1.is_finite() || span.0 > span.1 {
            return Err(VerifiedResidualError::Invalid(
                "Gray-Scott finite forward span required",
            ));
        }
        Ok(())
    }
    fn verify(
        &self,
        curve: &Rodas5pQuarticSegment,
        initial_radius: f64,
        tube_radius: f64,
        workspace: &mut ContinuousResidualWorkspace,
        work: &mut DenseSegmentWork,
    ) -> Result<VerifiedTube, VerifiedResidualError> {
        let bound = self.verify_continuous_residual_tube(
            curve,
            initial_radius,
            tube_radius,
            workspace,
            work,
        )?;
        work.gain_operations = work.gain_operations.saturating_add(1);
        Ok(VerifiedTube {
            endpoint_radius: add_up(
                bound.transported_initial_radius_upper,
                bound.residual_transport_upper,
            )?,
            uniform_radius: bound.error_radius_upper,
            residual_upper: bound.residual_l2_upper,
            logarithmic_norm_upper: bound.one_sided_log_norm_upper,
        })
    }
}
