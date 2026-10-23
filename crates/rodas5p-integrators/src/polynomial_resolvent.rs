//! Low-degree resolvents for a **declared** full-space polynomial relation.
//!
//! The operator is already scaled: `B = s A`. Coefficients are ascending powers
//! of `B`, so no `1/s` is formed. A caller must supply mathematical bounds on
//! `||q(B)||_inf` and `||B||_inf`; a few successful probes do not establish either.
//! This optional structural kernel is not an automatic Jacobian approximation.
//! Bounds reported below are conditional floating-point estimates, not interval
//! certificates. The original-space residual is always evaluated before output
//! is committed. No allocation occurs during `apply_checked`.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum PolynomialResolventError {
    #[error("invalid polynomial, norm witness, dimension, or tolerance")]
    InvalidInput,
    #[error("q(1) is zero or too cancellation-sensitive")]
    DenominatorMargin,
    #[error("declared relation defect is not smaller than |q(1)|")]
    RelationDefect,
    #[error("polynomial or bound arithmetic is nonfinite")]
    NonFinite,
    #[error("operator callback failed")]
    OperatorFailure,
}

/// Prepared quotient polynomial `p(B)`, with `(I-B)p(B)=I-q(B)/q(1)`.
#[derive(Debug, Clone)]
pub struct DeclaredPolynomialResolvent {
    coefficients: Vec<f64>,
    relation_defect_ratio: f64,
    inverse_norm_estimate: f64,
    denominator_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct PolynomialResolventWorkspace {
    candidate: Vec<f64>,
    action: Vec<f64>,
}
impl PolynomialResolventWorkspace {
    pub fn new(dimension: usize) -> Self {
        Self {
            candidate: vec![0.0; dimension],
            action: vec![0.0; dimension],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PolynomialResolventReport {
    /// True only when the measured original residual plus supplied evaluation
    /// error allowance is within the requested absolute residual tolerance.
    pub residual_accepted: bool,
    pub original_residual_norm: f64,
    /// Conditional on truthful full-space witnesses and residual error allowance;
    /// floating-point bound arithmetic itself is not outward rounded.
    pub conditional_error_estimate: f64,
    pub candidate_operator_actions: usize,
    pub residual_operator_actions: usize,
}

impl DeclaredPolynomialResolvent {
    /// `q[k]` multiplies `B^k`. The relation need not be monic. Degree 1..=128.
    /// `relation_norm` bounds `||q(B)||_inf` before normalization.
    /// `operator_norm` bounds `||B||_inf` for the identical operator callback.
    /// `minimum_denominator_ratio` is a strict caller margin in `(0, 1)`; a roundoff
    /// floor is imposed as well. A small discriminant is not a rejection reason.
    pub fn new(
        q: &[f64],
        relation_norm: f64,
        operator_norm: f64,
        minimum_denominator_ratio: f64,
    ) -> Result<Self, PolynomialResolventError> {
        if !(2..=129).contains(&q.len())
            || q.iter().any(|x| !x.is_finite())
            || q.last() == Some(&0.0)
            || !relation_norm.is_finite()
            || relation_norm < 0.0
            || !operator_norm.is_finite()
            || operator_norm < 0.0
            || !minimum_denominator_ratio.is_finite()
            || minimum_denominator_ratio <= 0.0
            || minimum_denominator_ratio >= 1.0
        {
            return Err(PolynomialResolventError::InvalidInput);
        }
        let scale = q.iter().fold(0.0_f64, |a, b| a.max(b.abs()));
        let normalized: Vec<f64> = q.iter().map(|c| c / scale).collect();
        // Do not silently change polynomial degree through normalization underflow.
        if q.iter()
            .zip(&normalized)
            .any(|(a, b)| *a != 0.0 && *b == 0.0)
        {
            return Err(PolynomialResolventError::NonFinite);
        }
        let d = q.len() - 1;
        let mut quotient = vec![0.0; d];
        quotient[d - 1] = normalized[d];
        for k in (1..d).rev() {
            quotient[k - 1] = normalized[k] + quotient[k];
        }
        let denominator = normalized[0] + quotient[0];
        let sum_abs: f64 = normalized.iter().map(|c| c.abs()).sum();
        let denominator_ratio = denominator.abs() / sum_abs;
        let floor = minimum_denominator_ratio.max(64.0 * q.len() as f64 * f64::EPSILON);
        if !denominator.is_finite() || denominator_ratio <= floor {
            return Err(PolynomialResolventError::DenominatorMargin);
        }
        let normalized_defect = relation_norm / scale;
        if relation_norm > 0.0 && normalized_defect == 0.0 {
            return Err(PolynomialResolventError::NonFinite);
        }
        let relation_defect_ratio = normalized_defect / denominator.abs();
        if !relation_defect_ratio.is_finite() || relation_defect_ratio >= 1.0 {
            return Err(PolynomialResolventError::RelationDefect);
        }
        for c in &mut quotient {
            *c /= denominator;
        }
        let polynomial_norm = quotient
            .iter()
            .rev()
            .fold(0.0_f64, |v, c| v * operator_norm + c.abs());
        let inverse_norm_estimate = polynomial_norm / (1.0 - relation_defect_ratio);
        if quotient.iter().any(|c| !c.is_finite()) || !inverse_norm_estimate.is_finite() {
            return Err(PolynomialResolventError::NonFinite);
        }
        Ok(Self {
            coefficients: quotient,
            relation_defect_ratio,
            inverse_norm_estimate,
            denominator_ratio,
        })
    }
    /// Degree of the declared relation `q`; the candidate `p` has degree at most one less.
    pub fn degree(&self) -> usize {
        self.coefficients.len()
    }
    pub fn denominator_ratio(&self) -> f64 {
        self.denominator_ratio
    }
    pub fn relation_defect_ratio(&self) -> f64 {
        self.relation_defect_ratio
    }
    pub fn inverse_norm_estimate(&self) -> f64 {
        self.inverse_norm_estimate
    }

    /// Apply the prepared polynomial and test `b - x + Bx` in the original space.
    /// Output is unchanged on residual rejection or callback/arithmetic failure.
    /// `residual_evaluation_error` must cover callback and residual arithmetic
    /// uncertainty when interpreting the conditional error estimate as a bound.
    pub fn apply_checked<F>(
        &self,
        b: &[f64],
        output: &mut [f64],
        workspace: &mut PolynomialResolventWorkspace,
        absolute_residual_tolerance: f64,
        residual_evaluation_error: f64,
        mut apply_b: F,
    ) -> Result<PolynomialResolventReport, PolynomialResolventError>
    where
        F: FnMut(&[f64], &mut [f64]) -> Result<(), PolynomialResolventError>,
    {
        let n = b.len();
        if n == 0
            || output.len() != n
            || workspace.candidate.len() != n
            || b.iter().any(|x| !x.is_finite())
            || !absolute_residual_tolerance.is_finite()
            || absolute_residual_tolerance < 0.0
            || !residual_evaluation_error.is_finite()
            || residual_evaluation_error < 0.0
        {
            return Err(PolynomialResolventError::InvalidInput);
        }
        let last = self.coefficients.len() - 1;
        for (x, rhs) in workspace.candidate.iter_mut().zip(b) {
            *x = self.coefficients[last] * rhs;
        }
        if workspace.candidate.iter().any(|x| !x.is_finite()) {
            return Err(PolynomialResolventError::NonFinite);
        }
        for k in (0..last).rev() {
            workspace.action.fill(f64::NAN);
            apply_b(&workspace.candidate, &mut workspace.action)?;
            for ((x, bx), rhs) in workspace.candidate.iter_mut().zip(&workspace.action).zip(b) {
                *x = self.coefficients[k].mul_add(*rhs, *bx);
            }
            if workspace.candidate.iter().any(|x| !x.is_finite()) {
                return Err(PolynomialResolventError::NonFinite);
            }
        }
        workspace.action.fill(f64::NAN);
        apply_b(&workspace.candidate, &mut workspace.action)?;
        let mut residual_norm = 0.0_f64;
        for ((rhs, x), bx) in b.iter().zip(&workspace.candidate).zip(&workspace.action) {
            let residual = (*rhs - *x) + *bx;
            if !residual.is_finite() {
                return Err(PolynomialResolventError::NonFinite);
            }
            residual_norm = residual_norm.max(residual.abs());
        }
        let allowance = residual_norm + residual_evaluation_error;
        let error_estimate = self.inverse_norm_estimate * allowance;
        if !allowance.is_finite() || !error_estimate.is_finite() {
            return Err(PolynomialResolventError::NonFinite);
        }
        let accepted = allowance <= absolute_residual_tolerance;
        if accepted {
            output.copy_from_slice(&workspace.candidate);
        }
        Ok(PolynomialResolventReport {
            residual_accepted: accepted,
            original_residual_norm: residual_norm,
            conditional_error_estimate: error_estimate,
            candidate_operator_actions: last,
            residual_operator_actions: 1,
        })
    }
}
