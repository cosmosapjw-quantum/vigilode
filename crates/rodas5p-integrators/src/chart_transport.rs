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
    directed::{Interval, add_up, mul_down, mul_up},
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
