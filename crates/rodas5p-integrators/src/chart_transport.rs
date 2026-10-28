//! Model-specific, research-only error transport for the dimensionless chart
//! `y = x^2 (w + 1/kappa)`. No integrator calls this module.
//!
//! The caller must already have valid coordinate bounds `|dx| <= rx` and
//! `|dw| <= rw` for the same state, time, branch and positive fixed `kappa`.
//! With `a = w + 1/kappa`, the exact reconstruction difference is
//! `(2 x dx + dx^2) a + (x + dx)^2 dw`. Consequently its magnitude is at
//! most `(2 |x| rx + rx^2) |a| + (|x| + rx)^2 rw`.
//!
//! Every arithmetic operation below is enclosed or rounded upward. The
//! difference between the exact central reconstruction and the *stored*
//! `y` is also included. The whole x-box must remain in a regular chart.
//! This proves a conditional physical-coordinate bound, not a local/global
//! ODE error bound, an embedded-estimator guarantee, or a new stiff solver.

use rodas5p_core::{
    CoreError, CoreResult,
    directed::{Interval, add_up, mul_down, mul_up, sub_up},
};

/// Stored coordinates (treated as exact binary64 reals), certified absolute
/// coordinate-error bounds, actual stored physical value, and required lower
/// bound on the chart denominator `x^2` throughout the box.
#[derive(Clone, Copy, Debug)]
pub struct ReconstructionInput {
    pub x: f64,
    pub w: f64,
    pub kappa: f64,
    pub x_error: f64,
    pub w_error: f64,
    pub stored_y: f64,
    pub denominator_min: f64,
}

/// A conditional certificate for the exact input supplied to the constructor.
/// It is neither deserializable nor a cache admission token for another state.
#[derive(Clone, Copy, Debug)]
pub struct ReconstructionCertificate {
    physical_y_error_upper: f64,
    central_reconstruction_error_upper: f64,
    denominator_lower: f64,
}

impl ReconstructionCertificate {
    pub fn physical_y_error_upper(&self) -> f64 {
        self.physical_y_error_upper
    }
    pub fn central_reconstruction_error_upper(&self) -> f64 {
        self.central_reconstruction_error_upper
    }
    pub fn denominator_lower(&self) -> f64 {
        self.denominator_lower
    }
}

/// Transport already-certified coordinate errors to the physical y error.
/// Overflow, a box touching x=0, and an insufficient denominator margin are
/// rejected rather than producing a finite-looking or falsely zero bound.
pub fn certify_reconstruction(
    input: &ReconstructionInput,
) -> CoreResult<ReconstructionCertificate> {
    let ReconstructionInput {
        x,
        w,
        kappa,
        x_error: rx,
        w_error: rw,
        stored_y,
        denominator_min,
    } = *input;
    if ![x, w, kappa, rx, rw, stored_y, denominator_min]
        .iter()
        .all(|v| v.is_finite())
        || kappa <= 0.0
        || rx < 0.0
        || rw < 0.0
        || denominator_min <= 0.0
    {
        return Err(CoreError::InvalidInput(
            "CHART_DOMAIN_UNSUPPORTED: invalid coordinate/error inputs".into(),
        ));
    }
    let x_interval = Interval::point(x)?.add(Interval::new(-rx, rx)?)?;
    let denominator_lower = mul_down(x_interval.mig(), x_interval.mig())?;
    if x_interval.contains_zero() || denominator_lower < denominator_min {
        return Err(CoreError::InvalidInput(
            "CHART_DOMAIN_UNSUPPORTED: the entire x-error box must be regular".into(),
        ));
    }
    let a = Interval::point(w)?.add(Interval::point(1.0)?.div(Interval::point(kappa)?)?)?;
    let x_abs_upper = add_up(x.abs(), rx)?;
    let quadratic_increment = add_up(mul_up(mul_up(2.0, x.abs())?, rx)?, mul_up(rx, rx)?)?;
    let coordinate_error = add_up(
        mul_up(quadratic_increment, a.mag())?,
        mul_up(mul_up(x_abs_upper, x_abs_upper)?, rw)?,
    )?;
    let central = Interval::point(x)?.mul(Interval::point(x)?)?.mul(a)?;
    let central_reconstruction_error_upper = central.sub(Interval::point(stored_y)?)?.mag();
    let physical_y_error_upper = add_up(coordinate_error, central_reconstruction_error_upper)?;
    Ok(ReconstructionCertificate {
        physical_y_error_upper,
        central_reconstruction_error_upper,
        denominator_lower,
    })
}

// ---------------------------------------------------------------------------
// Certified coordinate-error provider and controller (research node
// `research/rnext05_chart_provider_20261003`). Model-specific:
// `x' = x^2`, `y' = (-kappa + 2x) y + x^2 + eps x^3`, chart
// `w = y / x^2 - 1/kappa`, `w' = -kappa w + eps x`.
// ---------------------------------------------------------------------------

/// The parameters a chart run is bound to; compared bit for bit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartIdentity {
    pub kappa: f64,
    pub eps: f64,
    /// Sign of `x` on the run's branch: +1 or -1.
    pub branch: i8,
}

impl ChartIdentity {
    fn same(&self, other: &ChartIdentity) -> bool {
        self.kappa.to_bits() == other.kappa.to_bits()
            && self.eps.to_bits() == other.eps.to_bits()
            && self.branch == other.branch
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChartControllerConfig {
    pub atol: f64,
    pub rtol: f64,
    pub initial_step: f64,
    pub min_step: f64,
    pub max_step: f64,
    /// Lower bound on `x^2` over every whole-step tube.
    pub denominator_min: f64,
}

/// One accepted step (or a dense point inside one): the approximation, the
/// boxes that contain the exact chart state, the certified coordinate and
/// physical bounds, and the unused embedded proxy.
#[derive(Clone, Copy, Debug)]
pub struct ChartPoint {
    pub t: f64,
    pub x: f64,
    pub w: f64,
    pub y: f64,
    pub x_box: Interval,
    pub w_box: Interval,
    pub x_error: f64,
    pub w_error: f64,
    /// Global bound: distance of `y` to the exact solution from the exact
    /// initial data (the propagated enclosure), transported to `y`.
    pub physical_y_error_upper: f64,
    /// Local bound: the method's error over this step from the computed
    /// start point, transported to `y`; the controller uses this one.
    pub local_physical_bound: f64,
    /// Reported only; never used for acceptance.
    pub embedded_proxy: f64,
}

/// The start of an accepted step, from which its dense points are formed.
#[derive(Clone, Copy, Debug)]
pub struct ChartStepStart {
    pub t: f64,
    pub h: f64,
    pub x: f64,
    pub w: f64,
    pub x_box: Interval,
    pub w_box: Interval,
    /// Whole-step `x` tube.
    pub tube: Interval,
}

#[derive(Clone, Debug)]
pub struct ChartRun {
    pub identity: ChartIdentity,
    pub steps: Vec<(ChartStepStart, ChartPoint)>,
    pub outputs: Vec<ChartPoint>,
    pub rejected_attempts: usize,
    /// `None` when the run reached its end; otherwise the time and reason it
    /// refused to continue (the caller must use the protected solver there).
    pub refused: Option<(f64, String)>,
}

fn refuse(reason: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("CHART_DOMAIN_UNSUPPORTED: {reason}"))
}

/// `R(z) = (1 + z/3) / (1 - 2z/3 + z^2/6)`, the L-stable (1,2) Pade
/// approximant of `e^z` (order 3).
fn rational(z: f64) -> f64 {
    (1.0 + z / 3.0) / (1.0 - 2.0 * z / 3.0 + z * z / 6.0)
}

/// The exact chart flow over `tau` from the boxes `x_box`, `w_box`, and the
/// whole tube of `x` over `[0, tau]`. Refuses where the flow or the chart
/// is not regular.
fn enclose_flow(
    identity: &ChartIdentity,
    x_box: Interval,
    w_box: Interval,
    tau: f64,
    denominator_min: f64,
) -> CoreResult<(Interval, Interval, Interval)> {
    let one = Interval::point(1.0)?;
    let t = Interval::point(tau)?;
    // phi(x) = x / (1 - x tau), increasing in x; defined with margin while
    // x tau <= 1/2 on the box.
    if mul_up(x_box.hi.max(0.0), tau)? > 0.5 {
        return Err(refuse(
            "the step comes within a factor 2 of the blow-up of x",
        ));
    }
    let phi = |x: f64| -> CoreResult<Interval> {
        let p = Interval::point(x)?;
        p.div(one.sub(p.mul(t)?)?)
    };
    let x_end = Interval::new(phi(x_box.lo)?.lo, phi(x_box.hi)?.hi)?;
    // x' = x^2 >= 0: x increases in time, so the tube is [x_lo, phi(x_hi)].
    let tube = Interval::new(x_box.lo, x_end.hi)?;
    let sign_ok = if identity.branch > 0 {
        tube.lo > 0.0
    } else {
        tube.hi < 0.0
    };
    if !sign_ok || tube.contains_zero() {
        return Err(refuse(
            "the whole-step x tube leaves the branch or touches 0",
        ));
    }
    if mul_down(tube.mig(), tube.mig())? < denominator_min {
        return Err(refuse("x^2 falls below the denominator margin on the tube"));
    }
    // w(tau) = e^{-kappa tau} w0 + eps * int_0^tau e^{-kappa (tau - s)} x(s) ds,
    // and the integral lies in tube * (1 - e^{-kappa tau}) / kappa.
    // SAFE-ENCLOSURE: e^{-x} decreases, so enclose it over the whole product
    // interval of kappa tau, not only at the rounded-up product.
    let decay = rodas5p_core::polynomial_action::exp_neg_interval_enclosure(Interval::new(
        // kappa, tau > 0: the product is nonnegative even when mul_down
        // steps below zero on underflow.
        mul_down(identity.kappa, tau)?.max(0.0),
        mul_up(identity.kappa, tau)?,
    )?)?;
    let kappa = Interval::point(identity.kappa)?;
    let gain = one.sub(decay)?.div(kappa)?;
    let w_end = decay
        .mul(w_box)?
        .add(Interval::point(identity.eps)?.mul(tube)?.mul(gain)?)?;
    Ok((x_end, w_end, tube))
}

fn distance_to_box(value: f64, b: Interval) -> CoreResult<f64> {
    Ok(sub_up(value, b.lo)?.abs().max(sub_up(b.hi, value)?.abs()))
}

/// The approximation, enclosure and bounds `tau` after `start`.
fn advance(
    identity: &ChartIdentity,
    start: &ChartStepStart,
    tau: f64,
    denominator_min: f64,
) -> CoreResult<(ChartPoint, Interval)> {
    // Global: the propagated enclosure of the exact solution.
    let (x_box, w_box, tube) =
        enclose_flow(identity, start.x_box, start.w_box, tau, denominator_min)?;
    // Method: the x flow formula in binary64; for w the exponential-integrator
    // form with x linear over the step and e^z replaced by R.
    let kappa = identity.kappa;
    let z = -kappa * tau;
    let r = rational(z);
    let x = start.x / (1.0 - start.x * tau);
    let w = r * start.w
        + identity.eps
            * (start.x * (1.0 - r) / kappa
                + start.x * start.x * (tau / kappa - (1.0 - r) / (kappa * kappa)));
    let y = x * x * (w + 1.0 / kappa);
    if ![x, w, y].iter().all(|v| v.is_finite()) {
        return Err(refuse("the approximation is not finite"));
    }
    let x_error = distance_to_box(x, x_box)?;
    let w_error = distance_to_box(w, w_box)?;
    let global = certify_reconstruction(&ReconstructionInput {
        x,
        w,
        kappa,
        x_error,
        w_error,
        stored_y: y,
        denominator_min,
    })?;
    // Local: the exact flow from the computed start point.
    let (x_local, w_local, _) = enclose_flow(
        identity,
        Interval::point(start.x)?,
        Interval::point(start.w)?,
        tau,
        denominator_min,
    )?;
    let local = certify_reconstruction(&ReconstructionInput {
        x,
        w,
        kappa,
        x_error: distance_to_box(x, x_local)?,
        w_error: distance_to_box(w, w_local)?,
        stored_y: y,
        denominator_min,
    })?;
    let pade = (1.0 + 0.5 * z) / (1.0 - 0.5 * z);
    let embedded_proxy = (r - pade).abs() * start.w.abs() * x * x;
    Ok((
        ChartPoint {
            t: start.t + tau,
            x,
            w,
            y,
            x_box,
            w_box,
            x_error,
            w_error,
            physical_y_error_upper: global.physical_y_error_upper(),
            local_physical_bound: local.physical_y_error_upper(),
            embedded_proxy,
        },
        tube,
    ))
}

/// A dense point at `theta` in `(0, 1]` of an accepted step of a run bound
/// to `run_identity`. A query with another identity is refused.
pub fn chart_dense_point(
    run_identity: &ChartIdentity,
    query_identity: &ChartIdentity,
    start: &ChartStepStart,
    theta: f64,
    denominator_min: f64,
) -> CoreResult<ChartPoint> {
    if !run_identity.same(query_identity) {
        return Err(refuse(
            "the dense query's kappa, eps or branch differs from the run's",
        ));
    }
    if !(theta > 0.0 && theta <= 1.0) {
        return Err(CoreError::InvalidInput(
            "dense theta must lie in (0, 1]".into(),
        ));
    }
    Ok(advance(run_identity, start, theta * start.h, denominator_min)?.0)
}

/// Integrate the chart from the binary64 initial values `(x0, y0)` at `t0`
/// to `t_end`, landing on every `output` time. Each step is accepted iff its
/// certified *local* physical bound is at most `atol + rtol |y|`; otherwise
/// `h` is halved. The global bound is reported with every point. Below `min_step`, or when the chart is not regular, the run
/// stops with `refused` set.
pub fn chart_integrate(
    identity: &ChartIdentity,
    x0: f64,
    y0: f64,
    t0: f64,
    t_end: f64,
    outputs: &[f64],
    config: &ChartControllerConfig,
) -> CoreResult<ChartRun> {
    if !(identity.kappa.is_finite()
        && identity.kappa > 0.0
        && identity.eps.is_finite()
        && (identity.branch == 1 || identity.branch == -1)
        && x0.is_finite()
        && y0.is_finite()
        && (x0 > 0.0) == (identity.branch > 0)
        && x0 != 0.0
        && t0.is_finite()
        && t_end.is_finite()
        && t_end > t0
        && [
            config.atol,
            config.rtol,
            config.initial_step,
            config.min_step,
            config.max_step,
            config.denominator_min,
        ]
        .iter()
        .all(|v| v.is_finite())
        && outputs.iter().all(|t| t.is_finite())
        && config.atol >= 0.0
        && config.rtol >= 0.0
        && config.initial_step > 0.0
        && config.min_step > 0.0
        && config.max_step >= config.min_step
        && config.denominator_min > 0.0)
    {
        return Err(CoreError::InvalidInput("invalid chart run inputs".into()));
    }
    let x_box = Interval::point(x0)?;
    let inv_kappa = Interval::point(1.0)?.div(Interval::point(identity.kappa)?)?;
    let w_box = Interval::point(y0)?
        .div(x_box.mul(x_box)?)?
        .sub(inv_kappa)?;
    let mut state = ChartStepStart {
        t: t0,
        h: 0.0,
        x: x0,
        w: y0 / (x0 * x0) - 1.0 / identity.kappa,
        x_box,
        w_box,
        tube: x_box,
    };
    let mut run = ChartRun {
        identity: *identity,
        steps: Vec::new(),
        outputs: Vec::new(),
        rejected_attempts: 0,
        refused: None,
    };
    let mut targets: Vec<f64> = outputs
        .iter()
        .copied()
        .filter(|&t| t > t0 && t <= t_end)
        .collect();
    targets.push(t_end);
    targets.sort_by(f64::total_cmp);
    targets.dedup();
    let mut next_target = 0;
    let mut h = config.initial_step.min(config.max_step);
    while next_target < targets.len() {
        let target = targets[next_target];
        let remaining = target - state.t;
        let landing = h >= remaining;
        let tau = if landing { remaining } else { h };
        // A physical advance at a timestamp which cannot advance is not a
        // step of this run. Refuse before changing state or its enclosure.
        if !(tau.is_finite() && (state.t + tau).is_finite() && state.t + tau > state.t) {
            run.refused = Some((state.t, "the step cannot advance representable time".into()));
            return Ok(run);
        }
        let trial = advance(identity, &state, tau, config.denominator_min);
        let accepted = match &trial {
            Ok((point, _)) => {
                point.local_physical_bound <= config.atol + config.rtol * point.y.abs()
            }
            Err(_) => false,
        };
        if !accepted {
            run.rejected_attempts += 1;
            h = 0.5 * tau;
            if h < config.min_step {
                let reason = match trial {
                    Err(error) => error.to_string(),
                    Ok(_) => {
                        "the certified bound does not meet the tolerance above the minimum step"
                            .into()
                    }
                };
                run.refused = Some((state.t, reason));
                return Ok(run);
            }
            continue;
        }
        let (mut point, tube) = trial.expect("accepted");
        if landing {
            point.t = target;
        }
        let start = ChartStepStart {
            h: tau,
            tube,
            ..state
        };
        run.steps.push((start, point));
        if landing {
            run.outputs.push(point);
            next_target += 1;
        }
        state = ChartStepStart {
            t: point.t,
            h: 0.0,
            x: point.x,
            w: point.w,
            x_box: point.x_box,
            w_box: point.w_box,
            tube: point.x_box,
        };
        h = (1.5 * tau).min(config.max_step);
    }
    Ok(run)
}
