//! Sealed local continuous-residual provider for the frozen scalar PR101
//! witness. This module certifies one RODAS5P quartic segment only; it does
//! not integrate, connect segments, or grant trajectory-wide authority.
//!
//! `provider_operations` counts attempted interval API and directed arithmetic
//! calls one at a time, immediately before the call. The non-failing interval
//! magnitude projection `mag` is counted; scalar `abs`/`max` projections are
//! not. Validation and identity work stays in their dedicated counters.
//! Chained fallible calls are split so a refusal cannot charge later calls
//! that were never reached.

use std::sync::Arc;

use rodas5p_core::directed::{
    Interval, add_down, add_up, div_up, exp_interval, mul_down, mul_up, sub_down, sub_up,
};
use rodas5p_core::{CoreError, CoreResult, DenseMatrix};
use thiserror::Error;

use crate::dense_output_v2::{
    ContinuousResidualProvider, DenseOutputError, DenseOutputResult, DenseSegmentWork,
    Rodas5pQuarticSegment,
};
use crate::problem::OdeProblem;

const LINEAR_RATE: f64 = 1.0e6;
const QUADRATIC_RATE: f64 = 10.0;
const DOMAIN_END: f64 = 1.0;
const TRIG_DEGREE: usize = 32;
const LOCAL_TRIG_DEGREE: usize = 24;
const LOCAL_POLY_DEGREE: usize = 48;
const ATOL: f64 = 1.0e-11;
const RTOL: f64 = 1.0e-8;

/// A point-problem and interval-provider pair generated from the same private
/// immutable model. There is no constructor accepting caller callbacks or an
/// OdeProblem, and the point problem cannot be extracted or rebound.
///
/// ```compile_fail
/// # use rodas5p_integrators::BoundPr101Problem;
/// let owner = BoundPr101Problem::new().unwrap();
/// let problem = owner.problem().clone();
/// ```
pub struct BoundPr101Problem {
    problem: OdeProblem,
    _model: Arc<Pr101Model>,
}

struct Pr101Model;

impl BoundPr101Problem {
    /// Construct the one frozen PR101 model: `L=1e6`, `beta=10`, dimension 1,
    /// nonautonomous, identity mass matrix.
    pub fn new() -> CoreResult<Self> {
        let model = Arc::new(Pr101Model);
        let rhs_model = Arc::clone(&model);
        let jac_model = Arc::clone(&model);
        let jvp_model = Arc::clone(&model);
        let partial_model = Arc::clone(&model);
        let rhs = Arc::new(move |t, y: &[f64], out: &mut [f64]| rhs_model.point_rhs(t, y, out));
        let jacobian = Arc::new(move |t, y: &[f64]| jac_model.point_jacobian(t, y));
        let jvp = Arc::new(move |t, y: &[f64], v: &[f64], out: &mut [f64]| {
            jvp_model.point_jvp(t, y, v, out)
        });
        let partial_t =
            Arc::new(move |t, y: &[f64], out: &mut [f64]| partial_model.point_partial_t(t, y, out));
        let problem = OdeProblem::new(
            "pr101-sealed",
            1,
            rhs,
            None,
            Some(jacobian),
            Some(jvp),
            Some(partial_t),
            false,
            None,
            None,
        )?;
        Ok(Self {
            problem,
            _model: model,
        })
    }

    /// Dimension of the private generated pair.
    pub fn dimension(&self) -> usize {
        self.problem.dimension
    }

    /// Verify one local quartic segment using an outward Taylor residual and
    /// the full candidate-centered error tube. This result is segment-local.
    /// `initial_error_radius` is a caller premise: it must already enclose the
    /// true solution's distance from the represented curve at `t_old`.
    pub fn verify_segment(
        &self,
        segment: &Rodas5pQuarticSegment,
        initial_error_radius: f64,
        tube_radius: f64,
        work: &mut DenseSegmentWork,
    ) -> Result<Pr101SegmentCertificate, Pr101Error> {
        work.validation_operations = work.validation_operations.saturating_add(1);
        validate_owner(self)?;
        if !(initial_error_radius.is_finite()
            && initial_error_radius >= 0.0
            && tube_radius.is_finite()
            && tube_radius >= 0.0)
        {
            return Err(Pr101Error::InvalidRadius);
        }
        let (q, delta, t_old, t_new) = segment.scalar_quartic_coefficients(work)?;
        charge(work, 1);
        let time = Interval::new(t_old, t_new)?;
        if time.lo < 0.0 || time.hi > DOMAIN_END {
            return Err(Pr101Error::OutsideDomain);
        }
        validate_time(time, work)?;

        let (sin_coefficients, cos_coefficients, trig_remainder) =
            local_trig_coefficients(t_old, delta, work)?;
        let mut error_poly = [zero(); LOCAL_TRIG_DEGREE + 1];
        let mut cosine_poly = [zero(); LOCAL_TRIG_DEGREE + 1];
        for degree in 0..=LOCAL_TRIG_DEGREE {
            let q_coefficient = if degree < q.len() { q[degree] } else { zero() };
            charge(work, 1);
            error_poly[degree] = q_coefficient.sub(sin_coefficients[degree])?;
            cosine_poly[degree] = cos_coefficients[degree];
        }

        // If E0=q-Psin and δs=sin-Psin, then q−sin=E0−δs. Thus
        // r = Rpoly − Lδs − δc + 2βE0δs − βδs², where
        // Rpoly=q′/δ + L E0 − Pcos − βE0². The remainder terms below enclose
        // each correction with |δs|,|δc| <= δ^25/25!.
        let residual_poly = residual_polynomial(&q, &error_poly, &cosine_poly, delta, work)?;
        let residual_poly_bound = coefficient_abs_sum(&residual_poly, work)?;
        let error_poly_bound = coefficient_abs_sum(&error_poly, work)?;
        let linear_remainder = mul_positive(LINEAR_RATE, trig_remainder, work)?;
        let cross_remainder = mul_positive(
            mul_positive(2.0 * QUADRATIC_RATE, error_poly_bound, work)?,
            trig_remainder,
            work,
        )?;
        let square_remainder = mul_positive(
            QUADRATIC_RATE,
            mul_positive(trig_remainder, trig_remainder, work)?,
            work,
        )?;
        let residual_upper = add_positive(
            add_positive(
                add_positive(residual_poly_bound, linear_remainder, work)?,
                trig_remainder,
                work,
            )?,
            add_positive(cross_remainder, square_remainder, work)?,
            work,
        )?;

        let candidate_minus_sine_upper = polynomial_positive_part_sum(&error_poly, work)?;
        let candidate_minus_sine_upper =
            add_positive(candidate_minus_sine_upper, trig_remainder, work)?;
        let tube_mean_upper = add_positive(candidate_minus_sine_upper, tube_radius, work)?;
        let nonlinear_lognorm = mul_positive(2.0 * QUADRATIC_RATE, tube_mean_upper, work)?;
        charge(work, 1);
        let log_norm_upper = add_up(-LINEAR_RATE, nonlinear_lognorm)?;
        if !log_norm_upper.is_finite() {
            return Err(Pr101Error::NonFiniteBound);
        }

        let endpoint_radius = comparison_bound(
            log_norm_upper,
            residual_upper,
            initial_error_radius,
            delta.hi,
            work,
        )?;
        let required_radius = initial_error_radius.max(endpoint_radius);
        if required_radius > tube_radius {
            return Err(Pr101Error::TubeNotVerified {
                required_radius,
                tube_radius,
            });
        }

        work.identity_comparison_items = work.identity_comparison_items.saturating_add(14);
        Ok(Pr101SegmentCertificate {
            segment_key: segment_key(q, delta, t_old, t_new),
            t_start: t_old,
            t_end: t_new,
            quartic_coefficients: q,
            delta,
            log_norm_upper,
            residual_upper,
            residual_coefficients: residual_poly,
            initial_error_radius,
            tube_radius,
            endpoint_error_radius_upper: endpoint_radius,
            required_tube_radius: required_radius,
        })
    }

    /// Return the ordinary represented center and an outward radius around
    /// that exact returned binary64 center at one time in the certified
    /// segment. The radius includes both the flow comparison and evaluation
    /// displacement between interval and center evaluators.
    pub fn output_enclosure(
        &self,
        segment: &Rodas5pQuarticSegment,
        certificate: &Pr101SegmentCertificate,
        time: f64,
        work: &mut DenseSegmentWork,
    ) -> Result<Pr101OutputEnclosure, Pr101Error> {
        work.validation_operations = work.validation_operations.saturating_add(1);
        validate_owner(self)?;
        let (q, delta, t_old, t_new) = segment.scalar_quartic_coefficients(work)?;
        work.identity_comparison_items = work.identity_comparison_items.saturating_add(14);
        if segment_key(q, delta, t_old, t_new) != certificate.segment_key
            || t_old.to_bits() != certificate.t_start.to_bits()
            || t_new.to_bits() != certificate.t_end.to_bits()
            || !(time.is_finite() && time >= t_old && time <= t_new)
        {
            return Err(Pr101Error::SegmentCertificateMismatch);
        }
        let mut center = [0.0];
        segment.evaluate_center_at_into(time, &mut center, work)?;
        let mut curve = [zero()];
        segment.evaluate_value_at_into(time, &mut curve, work)?;
        let displacement = interval_point_distance(curve[0], center[0], work)?;
        charge(work, 1);
        let elapsed = Interval::exact_sum(time, -t_old)?;
        let flow_radius = comparison_bound_interval(
            certificate.log_norm_upper,
            certificate.residual_upper,
            certificate.initial_error_radius,
            elapsed,
            work,
        )?;
        let radius = add_positive(flow_radius, displacement, work)?;
        // `abs` and `max` are non-failing scalar projections; directed
        // arithmetic operations are charged individually immediately before
        // their call.
        charge(work, 1);
        let magnitude_lower = sub_down(center[0].abs(), radius)?.max(0.0);
        charge(work, 1);
        let scaled_tolerance = mul_down(RTOL, magnitude_lower)?;
        charge(work, 1);
        let tolerance_lower = add_down(ATOL, scaled_tolerance)?;
        Ok(Pr101OutputEnclosure {
            time,
            center: center[0],
            radius,
            tolerance_lower,
            within_normalized_tolerance: radius <= tolerance_lower,
        })
    }
}

impl ContinuousResidualProvider for BoundPr101Problem {
    fn enclose_rhs(
        &self,
        time: Interval,
        state: &[Interval],
        rhs: &mut [Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<()> {
        work.validation_operations = work.validation_operations.saturating_add(1);
        validate_owner(self).map_err(DenseOutputError::from)?;
        validate_time(time, work)?;
        if state.len() != 1 || rhs.len() != 1 {
            return Err(DenseOutputError::Core(CoreError::Dimension(
                "PR101 interval RHS shape mismatch".into(),
            )));
        }
        validate_interval(state[0], work)?;
        let (sine, cosine) = trig_range(time, work)?;
        charge(work, 1);
        let displacement = state[0].sub(sine)?;
        charge(work, 1);
        let linear = displacement.scale(-LINEAR_RATE)?;
        charge(work, 1);
        let squared = displacement.mul(displacement)?;
        charge(work, 1);
        let quadratic = squared.scale(QUADRATIC_RATE)?;
        charge(work, 1);
        let sum = linear.add(cosine)?;
        charge(work, 1);
        rhs[0] = sum.add(quadratic)?;
        Ok(())
    }

    fn one_sided_log_norm_upper(
        &self,
        time: Interval,
        tube: &[Interval],
        work: &mut DenseSegmentWork,
    ) -> DenseOutputResult<f64> {
        work.validation_operations = work.validation_operations.saturating_add(1);
        validate_owner(self).map_err(DenseOutputError::from)?;
        validate_time(time, work)?;
        if tube.len() != 1 {
            return Err(DenseOutputError::Core(CoreError::Dimension(
                "PR101 log-norm tube shape mismatch".into(),
            )));
        }
        validate_interval(tube[0], work)?;
        let (sine, _) = trig_range(time, work)?;
        charge(work, 1);
        let displacement = tube[0].sub(sine)?;
        charge(work, 1);
        let nonlinear_upper = mul_up(2.0 * QUADRATIC_RATE, displacement.hi)?;
        add_positive(-LINEAR_RATE, nonlinear_upper, work)
    }
}

/// Local, segment-only residual and flow certificate. Its fields cannot be
/// supplied by callers; it is valid for its frozen segment start and tube.
/// The key fingerprints the represented time/coefficient interval box only;
/// it does not assert provenance from a particular source `StepResult`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pr101SegmentCertificate {
    segment_key: [u64; 14],
    t_start: f64,
    t_end: f64,
    quartic_coefficients: [Interval; 5],
    delta: Interval,
    log_norm_upper: f64,
    residual_upper: f64,
    residual_coefficients: [Interval; LOCAL_POLY_DEGREE + 1],
    initial_error_radius: f64,
    tube_radius: f64,
    endpoint_error_radius_upper: f64,
    required_tube_radius: f64,
}

impl Pr101SegmentCertificate {
    pub fn quartic_coefficient_intervals(&self) -> &[Interval; 5] {
        &self.quartic_coefficients
    }

    pub fn delta_interval(&self) -> Interval {
        self.delta
    }

    pub fn residual_upper(&self) -> f64 {
        self.residual_upper
    }

    /// Outward intervals for coefficients of the residual polynomial before
    /// adding the separately bounded analytic Taylor remainder.
    pub fn residual_coefficient_intervals(&self) -> &[Interval; LOCAL_POLY_DEGREE + 1] {
        &self.residual_coefficients
    }

    pub fn log_norm_upper(&self) -> f64 {
        self.log_norm_upper
    }

    pub fn endpoint_error_radius_upper(&self) -> f64 {
        self.endpoint_error_radius_upper
    }

    pub fn required_tube_radius(&self) -> f64 {
        self.required_tube_radius
    }

    pub fn tube_radius(&self) -> f64 {
        self.tube_radius
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Immutable output enclosure. Read values through getters so callers cannot
/// mutate or forge a result after verification.
///
/// ```compile_fail
/// # use rodas5p_integrators::Pr101OutputEnclosure;
/// let mut output: Pr101OutputEnclosure = todo!();
/// output.center = 0.0;
/// ```
pub struct Pr101OutputEnclosure {
    time: f64,
    center: f64,
    radius: f64,
    tolerance_lower: f64,
    within_normalized_tolerance: bool,
}

impl Pr101OutputEnclosure {
    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn center(&self) -> f64 {
        self.center
    }

    pub fn radius(&self) -> f64 {
        self.radius
    }

    pub fn tolerance_lower(&self) -> f64 {
        self.tolerance_lower
    }

    pub fn within_normalized_tolerance(&self) -> bool {
        self.within_normalized_tolerance
    }
}

#[derive(Debug, Error)]
pub enum Pr101Error {
    #[error(transparent)]
    DenseOutput(#[from] DenseOutputError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("PR101 initial and tube radii must be finite and nonnegative")]
    InvalidRadius,
    #[error("PR101 segment interval is outside the certified [0, 1] domain")]
    OutsideDomain,
    #[error("PR101 segment and local certificate do not match")]
    SegmentCertificateMismatch,
    #[error("PR101 verified local error radius {required_radius:e} exceeds tube {tube_radius:e}")]
    TubeNotVerified {
        required_radius: f64,
        tube_radius: f64,
    },
    #[error("PR101 outward residual or flow bound is non-finite")]
    NonFiniteBound,
}

fn validate_owner(owner: &BoundPr101Problem) -> CoreResult<()> {
    if owner.problem.dimension != 1
        || owner.problem.autonomous
        || owner.problem.mass_matrix.is_some()
    {
        return Err(CoreError::InvalidInput(
            "PR101 immutable owner invariant failed".into(),
        ));
    }
    Ok(())
}

impl Pr101Model {
    fn point_rhs(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        if y.len() != 1 || out.len() != 1 || !t.is_finite() || !(0.0..=DOMAIN_END).contains(&t) {
            return Err(CoreError::InvalidInput(
                "invalid PR101 point RHS input".into(),
            ));
        }
        let error = y[0] - t.sin();
        out[0] = -LINEAR_RATE * error + t.cos() + QUADRATIC_RATE * error * error;
        finite_slice(out, "PR101 RHS")
    }

    fn point_jacobian(&self, t: f64, y: &[f64]) -> CoreResult<DenseMatrix> {
        if y.len() != 1 || !t.is_finite() || !(0.0..=DOMAIN_END).contains(&t) {
            return Err(CoreError::InvalidInput(
                "invalid PR101 Jacobian input".into(),
            ));
        }
        let error = y[0] - t.sin();
        DenseMatrix::new(1, 1, vec![-LINEAR_RATE + 2.0 * QUADRATIC_RATE * error])
    }

    fn point_jvp(&self, t: f64, y: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
        if y.len() != 1 || v.len() != 1 || out.len() != 1 {
            return Err(CoreError::Dimension("PR101 JVP shape mismatch".into()));
        }
        if !t.is_finite() || !(0.0..=DOMAIN_END).contains(&t) {
            return Err(CoreError::InvalidInput("invalid PR101 JVP time".into()));
        }
        let error = y[0] - t.sin();
        let jacobian = -LINEAR_RATE + 2.0 * QUADRATIC_RATE * error;
        out[0] = jacobian * v[0];
        finite_slice(out, "PR101 JVP")
    }

    fn point_partial_t(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        if y.len() != 1 || out.len() != 1 || !t.is_finite() || !(0.0..=DOMAIN_END).contains(&t) {
            return Err(CoreError::InvalidInput(
                "invalid PR101 partial_t input".into(),
            ));
        }
        let error = y[0] - t.sin();
        out[0] = LINEAR_RATE * t.cos() - t.sin() - 2.0 * QUADRATIC_RATE * error * t.cos();
        finite_slice(out, "PR101 partial_t")
    }
}

fn finite_slice(values: &[f64], name: &'static str) -> CoreResult<()> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(CoreError::NonFinite(format!("{name} produced NaN/Inf")))
    }
}

fn zero() -> Interval {
    Interval { lo: 0.0, hi: 0.0 }
}

fn segment_key(q: [Interval; 5], delta: Interval, t_old: f64, t_new: f64) -> [u64; 14] {
    [
        t_old.to_bits(),
        t_new.to_bits(),
        delta.lo.to_bits(),
        delta.hi.to_bits(),
        q[0].lo.to_bits(),
        q[0].hi.to_bits(),
        q[1].lo.to_bits(),
        q[1].hi.to_bits(),
        q[2].lo.to_bits(),
        q[2].hi.to_bits(),
        q[3].lo.to_bits(),
        q[3].hi.to_bits(),
        q[4].lo.to_bits(),
        q[4].hi.to_bits(),
    ]
}

fn charge(work: &mut DenseSegmentWork, operations: u64) {
    work.provider_operations = work.provider_operations.saturating_add(operations);
}

fn validate_interval(value: Interval, work: &mut DenseSegmentWork) -> DenseOutputResult<()> {
    work.validation_operations = work.validation_operations.saturating_add(1);
    charge(work, 1);
    Interval::new(value.lo, value.hi)?;
    Ok(())
}

fn validate_time(time: Interval, work: &mut DenseSegmentWork) -> DenseOutputResult<()> {
    validate_interval(time, work)?;
    if time.lo < 0.0 || time.hi > DOMAIN_END {
        return Err(DenseOutputError::InvalidSegment(
            "PR101 time outside [0, 1]",
        ));
    }
    Ok(())
}

fn trig_point(x: f64, work: &mut DenseSegmentWork) -> DenseOutputResult<(Interval, Interval)> {
    charge(work, 1);
    if !(x.is_finite() && (0.0..=DOMAIN_END).contains(&x)) {
        return Err(DenseOutputError::InvalidSegment(
            "PR101 trigonometric point outside [0, 1]",
        ));
    }
    charge(work, 1);
    let x_i = Interval::point(x)?;
    charge(work, 1);
    let mut power = Interval::point(1.0)?;
    charge(work, 1);
    let mut reciprocal_factorial = Interval::point(1.0)?;
    let mut sine = zero();
    let mut cosine = zero();
    let mut remainder = zero();
    for degree in 0..=TRIG_DEGREE + 1 {
        if degree > 0 {
            charge(work, 1);
            power = power.mul(x_i)?;
            charge(work, 1);
            let divisor = Interval::point(degree as f64)?;
            charge(work, 1);
            reciprocal_factorial = reciprocal_factorial.div(divisor)?;
        }
        charge(work, 1);
        let term = power.mul(reciprocal_factorial)?;
        if degree <= TRIG_DEGREE {
            charge(work, 1);
            if degree % 2 == 0 {
                cosine = if (degree / 2) % 2 == 0 {
                    cosine.add(term)?
                } else {
                    cosine.sub(term)?
                };
            } else {
                sine = if ((degree - 1) / 2) % 2 == 0 {
                    sine.add(term)?
                } else {
                    sine.sub(term)?
                };
            }
        } else {
            charge(work, 1);
            remainder = Interval::new(0.0, term.hi)?;
        }
    }
    charge(work, 1);
    let symmetric = Interval::new(-remainder.hi, remainder.hi)?;
    charge(work, 1);
    let sine = sine.add(symmetric)?;
    charge(work, 1);
    Ok((sine, cosine.add(symmetric)?))
}

/// On [0,1], sin is increasing because cos(x)>=1-x²/2>=1/2, and cos is
/// decreasing because sin(x)>=x-x³/6>=0. Endpoint Taylor enclosures therefore
/// enclose the full interval range without an unverified extrema search.
fn trig_range(
    time: Interval,
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<(Interval, Interval)> {
    validate_time(time, work)?;
    let (sin_lo, _) = trig_point(time.lo, work)?;
    let (sin_hi, _) = trig_point(time.hi, work)?;
    let (_, cos_lo) = trig_point(time.hi, work)?;
    let (_, cos_hi) = trig_point(time.lo, work)?;
    Ok((
        {
            charge(work, 1);
            Interval::new(sin_lo.lo, sin_hi.hi)?
        },
        {
            charge(work, 1);
            Interval::new(cos_lo.lo, cos_hi.hi)?
        },
    ))
}

fn local_trig_coefficients(
    t_old: f64,
    delta: Interval,
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<([Interval; 25], [Interval; 25], f64)> {
    let (sine0, cosine0) = trig_point(t_old, work)?;
    let mut sine = [zero(); LOCAL_TRIG_DEGREE + 1];
    let mut cosine = [zero(); LOCAL_TRIG_DEGREE + 1];
    sine[0] = sine0;
    cosine[0] = cosine0;
    for degree in 0..LOCAL_TRIG_DEGREE {
        charge(work, 1);
        let divisor = Interval::point((degree + 1) as f64)?;
        charge(work, 1);
        let sine_product = delta.mul(cosine[degree])?;
        charge(work, 1);
        sine[degree + 1] = sine_product.div(divisor)?;
        charge(work, 1);
        let cosine_product = delta.mul(sine[degree])?;
        charge(work, 1);
        cosine[degree + 1] = -cosine_product.div(divisor)?;
    }
    let remainder = positive_power_over_factorial(delta.hi, LOCAL_TRIG_DEGREE + 1, work)?;
    Ok((sine, cosine, remainder))
}

fn positive_power_over_factorial(
    value: f64,
    degree: usize,
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<f64> {
    charge(work, 1);
    let mut term = Interval::point(1.0)?;
    for n in 1..=degree {
        charge(work, 1);
        let value_interval = Interval::point(value)?;
        charge(work, 1);
        let product = term.mul(value_interval)?;
        charge(work, 1);
        let divisor = Interval::point(n as f64)?;
        charge(work, 1);
        term = product.div(divisor)?;
    }
    Ok(term.hi)
}

fn residual_polynomial(
    q: &[Interval; 5],
    error_poly: &[Interval; 25],
    cosine_poly: &[Interval; 25],
    delta: Interval,
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<[Interval; 49]> {
    let mut result = [zero(); LOCAL_POLY_DEGREE + 1];
    for degree in 0..=LOCAL_TRIG_DEGREE {
        charge(work, 1);
        result[degree] = error_poly[degree].scale(LINEAR_RATE)?;
        charge(work, 1);
        result[degree] = result[degree].sub(cosine_poly[degree])?;
    }
    for degree in 0_usize..5 {
        let derivative_degree = degree.saturating_sub(1);
        if degree > 0 {
            charge(work, 1);
            let scaled = q[degree].scale(degree as f64)?;
            charge(work, 1);
            let derivative = scaled.div(delta)?;
            charge(work, 1);
            result[derivative_degree] = result[derivative_degree].add(derivative)?;
        }
    }
    for left in 0..=LOCAL_TRIG_DEGREE {
        for right in 0..=LOCAL_TRIG_DEGREE {
            charge(work, 1);
            let product = error_poly[left].mul(error_poly[right])?;
            charge(work, 1);
            let product = product.scale(QUADRATIC_RATE)?;
            charge(work, 1);
            result[left + right] = result[left + right].sub(product)?;
        }
    }
    Ok(result)
}

fn coefficient_abs_sum(
    coefficients: &[Interval],
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<f64> {
    let mut sum = 0.0;
    for coefficient in coefficients {
        // `mag` is a non-failing interval projection and is counted as one
        // provider operation, just like the following outward sum.
        charge(work, 1);
        let magnitude = coefficient.mag();
        charge(work, 1);
        sum = add_up(sum, magnitude)?;
    }
    Ok(sum)
}

fn polynomial_positive_part_sum(
    coefficients: &[Interval],
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<f64> {
    let mut sum = 0.0;
    for coefficient in coefficients {
        charge(work, 1);
        sum = add_up(sum, coefficient.hi.max(0.0))?;
    }
    Ok(sum)
}

fn add_positive(a: f64, b: f64, work: &mut DenseSegmentWork) -> DenseOutputResult<f64> {
    charge(work, 1);
    add_up(a, b).map_err(DenseOutputError::from)
}

fn mul_positive(a: f64, b: f64, work: &mut DenseSegmentWork) -> DenseOutputResult<f64> {
    charge(work, 1);
    mul_up(a, b).map_err(DenseOutputError::from)
}

fn comparison_bound(
    mu: f64,
    rho: f64,
    initial_radius: f64,
    elapsed: f64,
    work: &mut DenseSegmentWork,
) -> Result<f64, Pr101Error> {
    charge(work, 1);
    let interval = Interval::point(elapsed)?;
    comparison_bound_interval(mu, rho, initial_radius, interval, work).map_err(Into::into)
}

fn comparison_bound_interval(
    mu: f64,
    rho: f64,
    initial_radius: f64,
    elapsed: Interval,
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<f64> {
    work.validation_operations = work.validation_operations.saturating_add(1);
    charge(work, 1);
    if elapsed.lo < 0.0 || elapsed.hi < elapsed.lo {
        return Err(DenseOutputError::InvalidSegment(
            "negative PR101 elapsed time",
        ));
    }
    let exponent = if mu >= 0.0 {
        charge(work, 1);
        mul_up(mu, elapsed.hi)?
    } else {
        charge(work, 1);
        mul_up(mu, elapsed.lo)?
    };
    charge(work, 1);
    let gain = exp_interval(exponent)?.hi;
    let integral = residual_integral_upper(mu, elapsed.hi, gain, work)?;
    charge(work, 1);
    let transported = mul_up(gain, initial_radius)?;
    charge(work, 1);
    let forcing = mul_up(rho, integral)?;
    charge(work, 1);
    add_up(transported, forcing).map_err(DenseOutputError::from)
}

fn residual_integral_upper(
    mu: f64,
    elapsed: f64,
    gain: f64,
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<f64> {
    if mu == 0.0 {
        return Ok(elapsed);
    }
    if mu < 0.0 {
        charge(work, 1);
        let exponent_lower = mul_down(mu, elapsed)?;
        charge(work, 1);
        let exp_lower = exp_interval(exponent_lower)?.lo;
        charge(work, 1);
        let numerator = sub_up(1.0, exp_lower)?;
        charge(work, 1);
        let signed = div_up(numerator, -mu)?;
        return Ok(signed.min(elapsed));
    }
    // Ψ=(exp(μs)-1)/μ, with the stable coarse upper s exp(μs) available if
    // the signed numerator loses its small positive value.
    charge(work, 1);
    let coarse = mul_up(elapsed, gain)?;
    charge(work, 1);
    let exponent_upper = mul_up(mu, elapsed)?;
    charge(work, 1);
    let exp_upper = exp_interval(exponent_upper)?.hi;
    charge(work, 1);
    let numerator = sub_up(exp_upper, 1.0)?;
    charge(work, 1);
    match div_up(numerator, mu) {
        Ok(signed) if signed >= 0.0 => Ok(signed.min(coarse)),
        _ => Ok(coarse),
    }
}

fn interval_point_distance(
    interval: Interval,
    center: f64,
    work: &mut DenseSegmentWork,
) -> DenseOutputResult<f64> {
    charge(work, 1);
    let point = Interval::point(center)?;
    charge(work, 1);
    let difference = point.sub(interval)?;
    charge(work, 1);
    let distance = difference.mag();
    if distance.is_finite() {
        Ok(distance)
    } else {
        Err(DenseOutputError::Core(CoreError::NonFinite(
            "PR101 output displacement is non-finite".into(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_jvp_matches_scalar_jacobian_without_materializing_matrix() {
        let owner = BoundPr101Problem::new().unwrap();
        let time: f64 = 0.75;
        let state = [time.sin() + 1.0e-4];
        let direction = [2.5];
        let error = state[0] - time.sin();
        let jacobian = -LINEAR_RATE + 2.0 * QUADRATIC_RATE * error;
        let operator = owner.problem.linearize_matrix_free(time, &state).unwrap();
        let mut actual = [0.0];
        operator.apply(&direction, &mut actual).unwrap();
        assert_eq!(actual[0].to_bits(), (jacobian * direction[0]).to_bits());
    }

    #[test]
    fn fallible_final_outward_add_is_charged_before_refusal() {
        let mut work = DenseSegmentWork::default();
        let before = work.provider_operations;
        assert!(add_positive(f64::MAX, f64::MAX, &mut work).is_err());
        assert_eq!(work.provider_operations - before, 1);
    }
}
