//! Regular, derivative-free chart of a scalar quadratic homotopy.
//!
//! The declared equation is `lambda*c*u*u + a*u - b = 0`, with `a > 0`
//! and `0 <= lambda <= 1`. This is an exact algebraic model supplied by the
//! caller, not a quadratic fit to an arbitrary callback. The returned branch
//! continues from `u=b/a` and has positive derivative with respect to `u`.
//! Floating-point residual diagnostics are operational, not interval bounds.

use serde::Serialize;

/// A refusal of the regular quadratic chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum QuadraticBranchError {
    /// Nonfinite coefficients, nonpositive `a`, or lambda outside `[0,1]`.
    #[error("invalid quadratic homotopy input")]
    Input,
    /// Normalization lost a nonzero coefficient to underflow.
    #[error("quadratic coefficient scale is not representable")]
    ScaleUnderflow,
    /// A real regular branch is absent or too close to the discriminant wall.
    #[error("quadratic chart requires a resolved positive discriminant")]
    DiscriminantWall,
    /// The root or its residual diagnostic cannot be represented.
    #[error("quadratic root or residual is not finite")]
    NonFinite,
    /// The original scaled equation did not pass the residual screen.
    #[error("quadratic original residual exceeds roundoff screen")]
    Residual,
}

/// Root and diagnostics for the original equation after common scaling.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct QuadraticBranchRoot {
    /// The continuation branch connected to `b/a` at lambda zero.
    pub value: f64,
    /// Common positive coefficient scale; `F_original = scale * F_scaled`.
    pub coefficient_scale: f64,
    /// Positive square root of the scaled discriminant.
    pub scaled_discriminant_root: f64,
    /// Signed residual of the original equation after common scaling.
    pub scaled_residual: f64,
    /// Absolute scaled residual divided by the sum of absolute equation terms.
    pub relative_residual: f64,
    /// Diagnostic `2*abs(residual)/(a+2*C*u+sqrt(discriminant))` in scaled
    /// coordinates. In exact arithmetic this is the branch error identity;
    /// floating-point evaluation does not include rounding uncertainty.
    pub residual_error_indicator: f64,
}

/// Solve the regular branch with a stable rationalized root formula.
///
/// Common coefficient scaling prevents discriminant overflow. A relative
/// discriminant margin of `64*EPSILON` separates this chart from an unresolved
/// fold. Near that wall callers can subdivide their continuation path or select
/// a separate validated chart. No derivative callbacks or iterations are used.
pub fn quadratic_homotopy_root(
    a: f64,
    b: f64,
    c: f64,
    lambda: f64,
) -> Result<QuadraticBranchRoot, QuadraticBranchError> {
    use QuadraticBranchError as E;
    if !a.is_finite()
        || a <= 0.0
        || !b.is_finite()
        || !c.is_finite()
        || !lambda.is_finite()
        || !(0.0..=1.0).contains(&lambda)
    {
        return Err(E::Input);
    }
    // Exact exceptional charts precede normalization: a quadratic scale may
    // erase a perfectly representable zero root or linear degree-drop root.
    if b == 0.0 {
        return Ok(QuadraticBranchRoot {
            value: 0.0,
            coefficient_scale: a,
            scaled_discriminant_root: 1.0,
            scaled_residual: 0.0,
            relative_residual: 0.0,
            residual_error_indicator: 0.0,
        });
    }
    if c == 0.0 || lambda == 0.0 {
        let value = b / a;
        if !value.is_finite() {
            return Err(E::NonFinite);
        }
        if value == 0.0 {
            return Err(E::ScaleUnderflow);
        }
        let scale = a.max(b.abs());
        let derivative = a / scale;
        let residual = a.mul_add(value, -b) / scale;
        let norm = (derivative * value).abs() + (b / scale).abs();
        let relative = residual.abs() / norm;
        let indicator = residual.abs() / derivative;
        if !indicator.is_finite() || !relative.is_finite() {
            return Err(E::NonFinite);
        }
        if relative > 128.0 * f64::EPSILON {
            return Err(E::Residual);
        }
        return Ok(QuadraticBranchRoot {
            value,
            coefficient_scale: scale,
            scaled_discriminant_root: derivative,
            scaled_residual: residual,
            relative_residual: relative,
            residual_error_indicator: indicator,
        });
    }
    let cc = lambda * c;
    if lambda != 0.0 && c != 0.0 && cc == 0.0 {
        return Err(E::ScaleUnderflow);
    }
    let scale = a.max(b.abs()).max(cc.abs());
    let aa = a / scale;
    let bb = b / scale;
    let dd = cc / scale;
    if aa == 0.0 || (b != 0.0 && bb == 0.0) || (cc != 0.0 && dd == 0.0) {
        return Err(E::ScaleUnderflow);
    }
    let square = aa * aa;
    let cross = 4.0 * dd * bb;
    let discriminant = aa.mul_add(aa, cross);
    let disc_scale = square + cross.abs();
    if !discriminant.is_finite()
        || discriminant <= 0.0
        || discriminant <= 64.0 * f64::EPSILON * disc_scale
    {
        return Err(E::DiscriminantWall);
    }
    let root = discriminant.sqrt();
    let value = if cc == 0.0 {
        b / a
    } else {
        2.0 * bb / (aa + root)
    };
    if !value.is_finite() {
        return Err(E::NonFinite);
    }
    // Evaluate D*u^2 as (D*u)*u: u^2 alone may overflow for a finite residual.
    let du = dd * value;
    let quadratic_term = du * value;
    let linear_term = aa * value;
    let residual = du.mul_add(value, linear_term - bb);
    let norm = quadratic_term.abs() + linear_term.abs() + bb.abs();
    let derivative = (2.0 * dd).mul_add(value, aa);
    if !residual.is_finite() || !norm.is_finite() || !derivative.is_finite() {
        return Err(E::NonFinite);
    }
    if derivative < 0.0 {
        return Err(E::DiscriminantWall);
    }
    let relative_residual = if norm == 0.0 {
        0.0
    } else {
        residual.abs() / norm
    };
    if relative_residual > 128.0 * f64::EPSILON {
        return Err(E::Residual);
    }
    let error_indicator = (residual.abs() / (derivative + root)) * 2.0;
    if !error_indicator.is_finite() {
        return Err(E::NonFinite);
    }
    Ok(QuadraticBranchRoot {
        value,
        coefficient_scale: scale,
        scaled_discriminant_root: root,
        scaled_residual: residual,
        relative_residual,
        residual_error_indicator: error_indicator,
    })
}
