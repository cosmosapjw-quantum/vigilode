//! An opaque pairing of the generated collective ODE and its continuous
//! interval provider. The factory owns both sides of the binding; callers
//! cannot assemble a provider/problem pair from separate public values.

use std::sync::Arc;

use rodas5p_core::directed::{Interval, add_up, mul_up, sqrt_up};
use rodas5p_core::{CoreError, CoreResult};

use crate::collective_model::CollectiveModel;
use crate::dense_output_v2::{
    ContinuousResidualProvider, ContinuousResidualVerificationResult, ContinuousResidualWorkspace,
    ContinuousSegmentErrorBound, DenseOutputError, DenseOutputResult, DenseSegmentWork,
    Rodas5pQuarticSegment,
};
use crate::problem::OdeProblem;

/// Immutable collective ODE together with the only continuous residual
/// provider admitted for its generated callbacks.
///
/// Fields and construction are private. Obtain an instance from
/// [`CollectiveModel::to_bound_continuous_problem`].
///
/// The generated [`OdeProblem`] and provider are not exposed independently;
/// there is no constructor accepting an arbitrary problem/provider pair.
/// A caller also cannot obtain and mutate a clone of the owned problem:
///
/// ```compile_fail
/// # use rodas5p_integrators::BoundCollectiveProblem;
/// fn extract(owner: &BoundCollectiveProblem) {
///     let _problem = owner.problem().clone();
/// }
/// ```
pub struct BoundCollectiveProblem {
    problem: OdeProblem,
    model: Arc<CollectiveModel>,
}

impl BoundCollectiveProblem {
    /// Dimension of the private generated problem/provider pair.
    pub fn dimension(&self) -> usize {
        self.problem.dimension
    }

    /// Verify one dense segment using this owner's private problem/provider
    /// pair. This accepts no separately supplied problem or provider.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_continuous_residual_tube(
        &self,
        segment: &Rodas5pQuarticSegment,
        initial_error_radius: f64,
        tube_radius: f64,
        workspace: &mut ContinuousResidualWorkspace,
        work: &mut DenseSegmentWork,
    ) -> ContinuousResidualVerificationResult<ContinuousSegmentErrorBound> {
        if self.problem.dimension != self.model.dimension()
            || !self.problem.autonomous
            || self.problem.mass_matrix.is_some()
        {
            return Err(DenseOutputError::BoundModelMismatch.into());
        }
        segment.verify_continuous_residual_tube(
            self,
            initial_error_radius,
            tube_radius,
            workspace,
            work,
        )
    }
}

impl CollectiveModel {
    /// Create the supported immutable problem/provider pair from one
    /// validated collective model. The point callbacks and interval
    /// provider share the same `Arc<CollectiveModel>`.
    pub fn to_bound_continuous_problem(&self) -> CoreResult<BoundCollectiveProblem> {
        let model = Arc::new(self.clone());
        let name = format!("collective-cubic-{}", model.num_modes());
        let rhs_model = Arc::clone(&model);
        let jvp_model = Arc::clone(&model);
        let rhs = Arc::new(move |t, y: &[f64], dy: &mut [f64]| rhs_model.rhs(t, y, dy));
        let jvp =
            Arc::new(move |t, y: &[f64], v: &[f64], out: &mut [f64]| jvp_model.jvp(t, y, v, out));
        let problem = OdeProblem::new(
            name,
            model.dimension(),
            rhs,
            None,
            None,
            Some(jvp),
            None,
            true,
            None,
            None,
        )?;
        Ok(BoundCollectiveProblem { problem, model })
    }
}

fn charge(work: &mut DenseSegmentWork, operations: u64) {
    work.provider_operations = work.provider_operations.saturating_add(operations);
}

fn point(value: f64, work: &mut DenseSegmentWork) -> DenseOutputResult<Interval> {
    charge(work, 1);
    Ok(Interval::point(value)?)
}

fn add(a: Interval, b: Interval, work: &mut DenseSegmentWork) -> DenseOutputResult<Interval> {
    charge(work, 1);
    Ok(a.add(b)?)
}

fn mul(a: Interval, b: Interval, work: &mut DenseSegmentWork) -> DenseOutputResult<Interval> {
    charge(work, 1);
    Ok(a.mul(b)?)
}

fn validate_interval(value: Interval, work: &mut DenseSegmentWork) -> DenseOutputResult<()> {
    work.validation_operations = work.validation_operations.saturating_add(1);
    charge(work, 1);
    Interval::new(value.lo, value.hi)?;
    Ok(())
}

fn validate_boxes(
    time: Interval,
    state: &[Interval],
    expected_dimension: usize,
    output_dimension: usize,
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<()> {
    work.validation_operations = work.validation_operations.saturating_add(1);
    if state.len() != expected_dimension || output_dimension != expected_dimension {
        return Err(DenseOutputError::Core(CoreError::Dimension(
            "collective interval provider shape mismatch".into(),
        )));
    }
    validate_interval(time, work)?;
    for &entry in state {
        validate_interval(entry, work)?;
    }
    work.provider_component_items = work
        .provider_component_items
        .saturating_add(expected_dimension as u64);
    Ok(())
}

fn interval_mean(
    model: &CollectiveModel,
    state: &[Interval],
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<(Interval, Interval)> {
    let mut mean_re = point(0.0, work)?;
    let mut mean_im = point(0.0, work)?;
    for (mode, &weight) in model.weights().iter().enumerate() {
        let weight = point(weight, work)?;
        let re_term = mul(state[2 * mode], weight, work)?;
        mean_re = add(mean_re, re_term, work)?;
        let im_term = mul(state[2 * mode + 1], weight, work)?;
        mean_im = add(mean_im, im_term, work)?;
    }
    Ok((mean_re, mean_im))
}

impl ContinuousResidualProvider for BoundCollectiveProblem {
    fn enclose_rhs(
        &self,
        time: Interval,
        state: &[Interval],
        rhs: &mut [Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<()> {
        validate_boxes(time, state, self.model.dimension(), rhs.len(), work)?;
        let (mean_re, mean_im) = interval_mean(&self.model, state, work)?;
        let mean_re_sq = mul(mean_re, mean_re, work)?;
        let mean_im_sq = mul(mean_im, mean_im, work)?;
        let mean_sq = add(mean_re_sq, mean_im_sq, work)?;
        let negative_kappa = point(-self.model.kappa(), work)?;
        let chi = point(self.model.chi(), work)?;
        let negative_chi = point(-self.model.chi(), work)?;
        let cubic_re = mul(mul(negative_chi, mean_sq, work)?, mean_im, work)?;
        let cubic_im = mul(mul(chi, mean_sq, work)?, mean_re, work)?;

        for mode in 0..self.model.num_modes() {
            work.provider_component_items = work.provider_component_items.saturating_add(2);
            let xi = point(self.model.xi()[mode], work)?;
            let width = point(self.model.width(), work)?;
            let omega = mul(width, xi, work)?;
            let damping_re = mul(negative_kappa, state[2 * mode], work)?;
            let rotation_re = mul(-omega, state[2 * mode + 1], work)?;
            rhs[2 * mode] = add(add(damping_re, rotation_re, work)?, cubic_re, work)?;
            let damping_im = mul(negative_kappa, state[2 * mode + 1], work)?;
            let rotation_im = mul(omega, state[2 * mode], work)?;
            rhs[2 * mode + 1] = add(add(damping_im, rotation_im, work)?, cubic_im, work)?;
        }
        Ok(())
    }

    fn one_sided_log_norm_upper(
        &self,
        time: Interval,
        tube: &[Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<f64> {
        work.validation_operations = work.validation_operations.saturating_add(1);
        if tube.len() != self.model.dimension() {
            return Err(DenseOutputError::Core(CoreError::Dimension(
                "collective log-norm tube shape mismatch".into(),
            )));
        }
        validate_interval(time, work)?;
        for &entry in tube {
            validate_interval(entry, work)?;
        }
        work.provider_component_items = work
            .provider_component_items
            .saturating_add(self.model.dimension() as u64);
        if self.model.chi() == 0.0 {
            charge(work, 1);
            return Ok(-self.model.kappa());
        }
        let (mean_re, mean_im) = interval_mean(&self.model, tube, work)?;

        // For g(m)=chi*i*|m|^2*m, ||Dg||_2 <= 3|chi||m|^2.
        // With the repetition map U and weighted-mean map B,
        // ||U Dg B||_2 <= 3|chi||m|^2 sqrt(N) sqrt(sum_j w_j^2).
        // `tube` is the complete supplied error tube, so the mean includes
        // both the represented curve and its candidate-centered radius.
        charge(work, 1);
        let mean_re_mag = mean_re.mag();
        charge(work, 1);
        let mean_im_mag = mean_im.mag();
        charge(work, 1);
        let re_square = mul_up(mean_re_mag, mean_re_mag)?;
        charge(work, 1);
        let im_square = mul_up(mean_im_mag, mean_im_mag)?;
        charge(work, 1);
        let mean_sq_upper = add_up(re_square, im_square)?;

        let mut weight_sq_sum = 0.0;
        for &weight in self.model.weights() {
            charge(work, 1);
            let square = mul_up(weight, weight)?;
            charge(work, 1);
            weight_sq_sum = add_up(weight_sq_sum, square)?;
        }
        let n_upper = exact_usize_upper(self.model.num_modes())?;
        charge(work, 2);
        let sqrt_n = sqrt_up(n_upper)?;
        let sqrt_weight_sq_sum = sqrt_up(weight_sq_sum)?;
        charge(work, 4);
        let mut nonlinear = mul_up(3.0, self.model.chi().abs())?;
        nonlinear = mul_up(nonlinear, mean_sq_upper)?;
        nonlinear = mul_up(nonlinear, sqrt_n)?;
        nonlinear = mul_up(nonlinear, sqrt_weight_sq_sum)?;
        charge(work, 1);
        let mu = add_up(-self.model.kappa(), nonlinear)?;
        if !mu.is_finite() {
            return Err(DenseOutputError::Core(CoreError::NonFinite(
                "collective one-sided log norm is not finite".into(),
            )));
        }
        Ok(mu)
    }
}

fn exact_usize_upper(value: usize) -> DenseOutputResult<f64> {
    let rounded = value as f64;
    if !rounded.is_finite() {
        return Err(DenseOutputError::Core(CoreError::NonFinite(
            "mode count conversion is not finite".into(),
        )));
    }
    if (rounded as u128) < value as u128 {
        Ok(rounded.next_up())
    } else {
        Ok(rounded)
    }
}
