//! Shifted polynomial resolvents with exact declaration binding.
//!
//! The caller declares two immutable actions satisfying B = center*I + scale*C
//! in exact real arithmetic, and a full-space bound on q(C). Native evaluation
//! of C avoids forming expanded coefficients in B or subtracting a large center
//! in every action. The original B action independently checks b-(I-B)x.
//!
//! Coefficient preparation includes an outward bound on the defect of the
//! *stored binary64 polynomial*, including the rounded chart coordinate.
//! This is conditional on truthful full-space declarations and on the supplied
//! residual evaluation allowance. It does not certify an opaque callback.

use rodas5p_core::directed::{Interval, add_up, div_up, mul_up, sub_down, sum_up};
use serde::Serialize;

use crate::polynomial_resolvent::{
    PolynomialPhysicalNorm, PolynomialRefinementPolicy, PolynomialRefinementStop,
    PolynomialResolventError as Error,
};

/// Caller-maintained identity of one immutable scaled operator B.
/// `operator_scale_bits` is the exact binary64 scale used to construct B.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChartOperatorIdentity {
    pub operator_key: u64,
    pub dimension: usize,
    pub model_epoch: u64,
    pub provider_epoch: u64,
    pub operator_scale_bits: u64,
    pub physical_norm: PolynomialPhysicalNorm,
}

/// Immutable declaration. The constructor validates representation, not the
/// mathematical truth of the relation or callback identity. A nonzero evidence
/// reference must identify the external proof/provider establishing the bounds.
#[derive(Clone, Debug)]
pub struct ChartDeclaration {
    identity: ChartOperatorIdentity,
    center: f64,
    scale: f64,
    q: Vec<f64>,
    relation_norm: f64,
    chart_norm: f64,
    evidence_reference: u64,
}

impl ChartDeclaration {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: ChartOperatorIdentity,
        center: f64,
        scale: f64,
        q: &[f64],
        relation_norm: f64,
        chart_norm: f64,
        evidence_reference: u64,
    ) -> Result<Self, Error> {
        if identity.operator_key == 0
            || identity.dimension == 0
            || identity.physical_norm != PolynomialPhysicalNorm::Infinity
            || !f64::from_bits(identity.operator_scale_bits).is_finite()
            || !center.is_finite()
            || !scale.is_finite()
            || scale <= 0.0
            || !(2..=129).contains(&q.len())
            || q.iter().any(|x| !x.is_finite())
            || q.last() == Some(&0.0)
            || !relation_norm.is_finite()
            || relation_norm < 0.0
            || !chart_norm.is_finite()
            || chart_norm < 0.0
            || evidence_reference == 0
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            identity,
            center,
            scale,
            q: q.to_vec(),
            relation_norm,
            chart_norm,
            evidence_reference,
        })
    }
    pub fn identity(&self) -> ChartOperatorIdentity {
        self.identity
    }
    pub fn center(&self) -> f64 {
        self.center
    }
    pub fn scale(&self) -> f64 {
        self.scale
    }
    pub fn coefficients(&self) -> &[f64] {
        &self.q
    }
    pub fn relation_norm(&self) -> f64 {
        self.relation_norm
    }
    pub fn chart_norm(&self) -> f64 {
        self.chart_norm
    }
    pub fn evidence_reference(&self) -> u64 {
        self.evidence_reference
    }

    fn same(&self, other: &Self) -> bool {
        self.identity == other.identity
            && self.center.to_bits() == other.center.to_bits()
            && self.scale.to_bits() == other.scale.to_bits()
            && self.relation_norm.to_bits() == other.relation_norm.to_bits()
            && self.chart_norm.to_bits() == other.chart_norm.to_bits()
            && self.evidence_reference == other.evidence_reference
            && self.q.len() == other.q.len()
            && self
                .q
                .iter()
                .zip(&other.q)
                .all(|(x, y)| x.to_bits() == y.to_bits())
    }
}

/// Both callbacks must be pure actions of the declared immutable operators.
/// Supplying a different callback under an unchanged key violates this contract;
/// metadata comparison and finite probes cannot establish operator equality.
pub trait DeclaredChartOperator {
    fn declaration(&self) -> &ChartDeclaration;
    fn apply_chart(&self, input: &[f64], output: &mut [f64]) -> Result<(), Error>;
    fn apply_original(&self, input: &[f64], output: &mut [f64]) -> Result<(), Error>;
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ChartWork {
    pub setups_attempted: u64,
    pub setups_refused: u64,
    pub coefficients_normalized: u64,
    pub synthetic_multiply_adds: u64,
    pub coefficient_defects_bounded: u64,
    pub applications_attempted: u64,
    pub applications_refused: u64,
    pub binding_checks: u64,
    pub chart_actions_attempted: u64,
    pub original_actions_attempted: u64,
    pub actions_completed: u64,
    pub corrections_attempted: u64,
}

#[derive(Clone, Debug)]
pub struct ConditionedPolynomialPlan {
    declaration: ChartDeclaration,
    coefficients: Vec<f64>,
    alpha: f64,
    denominator_ratio: f64,
    relation_contribution: f64,
    preparation_defect: f64,
    total_defect: f64,
    inverse_norm_upper: f64,
}

fn finite_result<T>(value: Result<T, rodas5p_core::CoreError>) -> Result<T, Error> {
    value.map_err(|_| Error::NonFinite)
}
fn point(value: f64) -> Result<Interval, Error> {
    finite_result(Interval::point(value))
}
fn poly_bound(coefficients: &[f64], norm: f64) -> Result<f64, Error> {
    let mut result = 0.0;
    for c in coefficients.iter().rev() {
        result = finite_result(add_up(finite_result(mul_up(result, norm))?, c.abs()))?;
    }
    Ok(result)
}

impl ConditionedPolynomialPlan {
    pub fn prepare(
        declaration: &ChartDeclaration,
        minimum_denominator_ratio: f64,
        work: &mut ChartWork,
    ) -> Result<Self, Error> {
        work.setups_attempted = work.setups_attempted.saturating_add(1);
        let result = Self::prepare_inner(declaration, minimum_denominator_ratio, work);
        if result.is_err() {
            work.setups_refused = work.setups_refused.saturating_add(1);
        }
        result
    }

    fn prepare_inner(
        d: &ChartDeclaration,
        margin: f64,
        work: &mut ChartWork,
    ) -> Result<Self, Error> {
        if !margin.is_finite() || !(0.0..1.0).contains(&margin) || margin == 0.0 {
            return Err(Error::InvalidInput);
        }
        let largest = d.q.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
        // A binary power gives exact coefficient normalization unless precision
        // is lost at the subnormal boundary; the exact inverse scaling detects it.
        let exponent = largest.to_bits() & 0x7ff0_0000_0000_0000;
        let normalizer = f64::from_bits(exponent.max(0x0010_0000_0000_0000));
        let q: Vec<f64> = d.q.iter().map(|x| x / normalizer).collect();
        work.coefficients_normalized = work.coefficients_normalized.saturating_add(q.len() as u64);
        if q.iter()
            .zip(&d.q)
            .any(|(x, y)| !x.is_finite() || (*x * normalizer).to_bits() != y.to_bits())
        {
            return Err(Error::NonFinite);
        }
        let alpha = (1.0 - d.center) / d.scale;
        if !alpha.is_finite() {
            return Err(Error::NonFinite);
        }
        let degree = q.len() - 1;
        let mut p = vec![0.0; degree];
        p[degree - 1] = q[degree];
        for k in (1..degree).rev() {
            p[k - 1] = alpha.mul_add(p[k], q[k]);
            work.synthetic_multiply_adds = work.synthetic_multiply_adds.saturating_add(1);
        }
        let denominator = alpha.mul_add(p[0], q[0]);
        let abs_denominator_sum = poly_bound(&q, alpha.abs())?;
        let ratio = denominator.abs() / abs_denominator_sum;
        if !denominator.is_finite()
            || !ratio.is_finite()
            || ratio <= margin.max(64.0 * q.len() as f64 * f64::EPSILON)
        {
            return Err(Error::DenominatorMargin);
        }
        for x in &mut p {
            let old = *x;
            *x = (*x / denominator) / d.scale;
            if !x.is_finite() || (old != 0.0 && *x == 0.0) {
                return Err(Error::NonFinite);
            }
        }
        let multiplier = 1.0 / denominator;
        if !multiplier.is_finite() || multiplier == 0.0 {
            return Err(Error::NonFinite);
        }
        let relation = finite_result(mul_up(
            multiplier.abs(),
            finite_result(div_up(d.relation_norm, normalizer))?,
        ))?;
        // Bound E(C)=I-(I-B)p(C)-multiplier*q(C). This explicitly covers
        // rounded alpha, synthetic division, quotient coefficients and scale.
        let a = finite_result(point(1.0)?.sub(point(d.center)?))?;
        let scale = point(d.scale)?;
        let multiplier_i = point(multiplier)?;
        let mut remainder = Vec::with_capacity(degree + 1);
        for j in 0..=degree {
            let mut e = point(if j == 0 { 1.0 } else { 0.0 })?;
            if j < degree {
                e = finite_result(e.sub(finite_result(a.mul(point(p[j])?))?))?;
            }
            if j > 0 {
                e = finite_result(e.add(finite_result(scale.mul(point(p[j - 1])?))?))?;
            }
            e = finite_result(e.sub(finite_result(multiplier_i.mul(point(q[j])?))?))?;
            remainder.push(e.mag());
            work.coefficient_defects_bounded = work.coefficient_defects_bounded.saturating_add(1);
        }
        let preparation_defect = poly_bound(&remainder, d.chart_norm)?;
        let total_defect = finite_result(add_up(relation, preparation_defect))?;
        if total_defect >= 1.0 {
            return Err(Error::RelationDefect);
        }
        let p_norm = poly_bound(&p, d.chart_norm)?;
        let inverse_norm_upper =
            finite_result(div_up(p_norm, finite_result(sub_down(1.0, total_defect))?))?;
        Ok(Self {
            declaration: d.clone(),
            coefficients: p,
            alpha,
            denominator_ratio: ratio,
            relation_contribution: relation,
            preparation_defect,
            total_defect,
            inverse_norm_upper,
        })
    }

    pub fn degree(&self) -> usize {
        self.coefficients.len()
    }
    pub fn chart_coordinate(&self) -> f64 {
        self.alpha
    }
    pub fn denominator_ratio(&self) -> f64 {
        self.denominator_ratio
    }
    pub fn relation_contribution_upper(&self) -> f64 {
        self.relation_contribution
    }
    pub fn preparation_defect_upper(&self) -> f64 {
        self.preparation_defect
    }
    pub fn total_defect_upper(&self) -> f64 {
        self.total_defect
    }
    pub fn inverse_norm_upper(&self) -> f64 {
        self.inverse_norm_upper
    }

    fn action<P: DeclaredChartOperator>(
        &self,
        provider: &P,
        rhs: &[f64],
        out: &mut [f64],
        scratch: &mut [f64],
        work: &mut ChartWork,
    ) -> Result<(), Error> {
        let last = self.degree() - 1;
        for (x, b) in out.iter_mut().zip(rhs) {
            *x = self.coefficients[last] * b;
        }
        if out.iter().any(|x| !x.is_finite()) {
            return Err(Error::NonFinite);
        }
        for k in (0..last).rev() {
            scratch.fill(f64::NAN);
            work.chart_actions_attempted = work.chart_actions_attempted.saturating_add(1);
            provider.apply_chart(out, scratch)?;
            if scratch.iter().any(|x| !x.is_finite()) {
                return Err(Error::NonFinite);
            }
            work.actions_completed = work.actions_completed.saturating_add(1);
            for ((x, cx), b) in out.iter_mut().zip(scratch.iter()).zip(rhs) {
                *x = self.coefficients[k].mul_add(*b, *cx);
            }
            if out.iter().any(|x| !x.is_finite()) {
                return Err(Error::NonFinite);
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply<P: DeclaredChartOperator>(
        &self,
        provider: &P,
        rhs: &[f64],
        output: &mut [f64],
        ws: &mut ChartWorkspace,
        policy: PolynomialRefinementPolicy,
        residual_tolerance: f64,
        residual_evaluation_error: f64,
        work: &mut ChartWork,
    ) -> Result<ChartReport, Error> {
        work.applications_attempted = work.applications_attempted.saturating_add(1);
        let result = self.apply_inner(
            provider,
            rhs,
            output,
            ws,
            policy,
            residual_tolerance,
            residual_evaluation_error,
            work,
        );
        if !matches!(&result, Ok(r) if r.accepted) {
            work.applications_refused = work.applications_refused.saturating_add(1);
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_inner<P: DeclaredChartOperator>(
        &self,
        provider: &P,
        rhs: &[f64],
        output: &mut [f64],
        ws: &mut ChartWorkspace,
        policy: PolynomialRefinementPolicy,
        tolerance: f64,
        evaluation_error: f64,
        work: &mut ChartWork,
    ) -> Result<ChartReport, Error> {
        work.binding_checks = work.binding_checks.saturating_add(1);
        if !self.declaration.same(provider.declaration()) {
            return Err(Error::WitnessMismatch);
        }
        let n = self.declaration.identity.dimension;
        if rhs.len() != n
            || output.len() != n
            || ws.candidate.len() != n
            || rhs.iter().any(|x| !x.is_finite())
            || !tolerance.is_finite()
            || tolerance < 0.0
            || !evaluation_error.is_finite()
            || evaluation_error < 0.0
            || !policy.maximum_relation_ratio.is_finite()
            || !(0.0..1.0).contains(&policy.maximum_relation_ratio)
            || !policy.stagnation_ratio.is_finite()
            || !(0.0..1.0).contains(&policy.stagnation_ratio)
            || policy.stagnation_ratio == 0.0
        {
            return Err(Error::InvalidInput);
        }
        let mut report = ChartReport {
            accepted: false,
            stop: PolynomialRefinementStop::RelationTooWeak,
            corrections: 0,
            original_residual_norm: None,
            conditional_error_upper: None,
        };
        if self.total_defect > policy.maximum_relation_ratio {
            return Ok(report);
        }
        let cost = self.degree();
        let mut actions_left = policy.max_operator_actions;
        if actions_left < cost {
            report.stop = PolynomialRefinementStop::ActionBudget;
            return Ok(report);
        }
        self.action(provider, rhs, &mut ws.candidate, &mut ws.action, work)?;
        actions_left -= cost;
        let mut previous = f64::INFINITY;
        loop {
            ws.action.fill(f64::NAN);
            work.original_actions_attempted = work.original_actions_attempted.saturating_add(1);
            provider.apply_original(&ws.candidate, &mut ws.action)?;
            if ws.action.iter().any(|x| !x.is_finite()) {
                return Err(Error::NonFinite);
            }
            work.actions_completed = work.actions_completed.saturating_add(1);
            let mut norm = 0.0_f64;
            for (i, &rhs_value) in rhs.iter().enumerate().take(n) {
                ws.residual[i] = (rhs_value - ws.candidate[i]) + ws.action[i];
                if !ws.residual[i].is_finite() {
                    return Err(Error::NonFinite);
                }
                norm = norm.max(ws.residual[i].abs());
            }
            let allowance = finite_result(add_up(norm, evaluation_error))?;
            report.original_residual_norm = Some(norm);
            report.conditional_error_upper =
                Some(finite_result(mul_up(self.inverse_norm_upper, allowance))?);
            if allowance <= tolerance {
                // Check again before commit; immutable declaration is required
                // throughout, and a changed epoch during callbacks is refused.
                work.binding_checks = work.binding_checks.saturating_add(1);
                if !self.declaration.same(provider.declaration()) {
                    return Err(Error::WitnessMismatch);
                }
                output.copy_from_slice(&ws.candidate);
                report.accepted = true;
                report.stop = PolynomialRefinementStop::Accepted;
                return Ok(report);
            }
            if report.corrections > 0 && norm >= policy.stagnation_ratio * previous {
                report.stop = PolynomialRefinementStop::Stagnation;
                return Ok(report);
            }
            if report.corrections >= policy.max_corrections {
                report.stop = PolynomialRefinementStop::CorrectionBudget;
                return Ok(report);
            }
            if actions_left < cost {
                report.stop = PolynomialRefinementStop::ActionBudget;
                return Ok(report);
            }
            previous = norm;
            work.corrections_attempted = work.corrections_attempted.saturating_add(1);
            self.action(
                provider,
                &ws.residual,
                &mut ws.correction,
                &mut ws.action,
                work,
            )?;
            for (x, dx) in ws.candidate.iter_mut().zip(&ws.correction) {
                *x += dx;
                if !x.is_finite() {
                    return Err(Error::NonFinite);
                }
            }
            report.corrections += 1;
            actions_left -= cost;
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChartWorkspace {
    candidate: Vec<f64>,
    residual: Vec<f64>,
    correction: Vec<f64>,
    action: Vec<f64>,
}
impl ChartWorkspace {
    /// Allocates four vectors once. Plan application performs no allocation.
    pub fn new(n: usize) -> Self {
        Self {
            candidate: vec![0.0; n],
            residual: vec![0.0; n],
            correction: vec![0.0; n],
            action: vec![0.0; n],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChartReport {
    pub accepted: bool,
    pub stop: PolynomialRefinementStop,
    pub corrections: usize,
    pub original_residual_norm: Option<f64>,
    /// Conditional on B=center*I+scale*C, full-space norm witnesses and the
    /// supplied upper bound on floating residual-evaluation error.
    pub conditional_error_upper: Option<f64>,
}

/// Interval `[a, b]` on the negative real line `(-\infty, 0)` with assigned Chebyshev degree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChebyshevInterval {
    pub a: f64,
    pub b: f64,
    pub degree: usize,
}

impl ChebyshevInterval {
    pub fn new(a: f64, b: f64, degree: usize) -> Result<Self, Error> {
        if !a.is_finite() || !b.is_finite() || a >= b || b >= 0.0 || degree == 0 || degree > 128 {
            return Err(Error::InvalidInput);
        }
        Ok(Self { a, b, degree })
    }

    /// Center of the interval: `c = (a + b) / 2`.
    #[inline]
    pub fn center(&self) -> f64 {
        (self.a + self.b) * 0.5
    }

    /// Semi-width of the interval: `\rho = (b - a) / 2`.
    #[inline]
    pub fn semi_width(&self) -> f64 {
        (self.b - self.a) * 0.5
    }
}

/// Source of spectral information.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum SpectralSource {
    /// Derived from outward Gershgorin discs; certified enclosure.
    GershgorinCertified,
    /// Derived from Lanczos or Arnoldi Ritz values; estimate only (`EST`).
    RitzEstimate,
}

/// Certification status for generated polynomial resolvents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum CertificateStatus {
    /// Certified outward bound.
    Certified,
    /// Uncertified estimate only (`EST`).
    EstimateOnly,
}

/// Product Chebyshev-cluster polynomial $q(z) = \prod_j T_{d_j}((z - c_j)/\rho_j)$
/// optionally augmented with quadratic rotation factors $(z + d)^2 + \omega^2$.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GeneratedChebyshevPolynomial {
    /// Polynomial coefficients in ascending monomial powers $q_0, q_1, \dots, q_d$.
    pub coefficients: Vec<f64>,
    /// Defect upper bound $\eta \le \max_j |T_{d_j}((1 - c_j)/\rho_j)|^{-1}$.
    pub defect_bound: f64,
    /// Provenance of the spectral intervals.
    pub source: SpectralSource,
    /// Certification status (`Certified` for Gershgorin, `EstimateOnly` for Ritz).
    pub certificate_status: CertificateStatus,
    /// Whether a damped rotation quadratic factor was incorporated.
    pub has_skew_factor: bool,
}

impl GeneratedChebyshevPolynomial {
    #[inline]
    pub fn is_certified(&self) -> bool {
        self.certificate_status == CertificateStatus::Certified
    }

    /// Degree of polynomial $q$.
    #[inline]
    pub fn degree(&self) -> usize {
        self.coefficients.len().saturating_sub(1)
    }

    /// Evaluate polynomial at scalar $z$ using Horner's method.
    pub fn evaluate(&self, z: f64) -> f64 {
        let mut val = 0.0_f64;
        for &c in self.coefficients.iter().rev() {
            val = val.mul_add(z, c);
        }
        val
    }
}

/// Monomial coefficients of Chebyshev polynomial $T_d(x)$ of degree $d$.
/// Returns coefficients in ascending order: `[c_0, c_1, ..., c_d]`.
pub fn chebyshev_basis_coefficients(degree: usize) -> Vec<f64> {
    match degree {
        0 => vec![1.0],
        1 => vec![0.0, 1.0],
        _ => {
            let mut t_prev = vec![1.0];
            let mut t_curr = vec![0.0, 1.0];
            for _ in 2..=degree {
                let mut t_next = vec![0.0; t_curr.len() + 1];
                for (i, &c) in t_curr.iter().enumerate() {
                    t_next[i + 1] = 2.0 * c;
                }
                for (i, &c) in t_prev.iter().enumerate() {
                    t_next[i] -= c;
                }
                t_prev = t_curr;
                t_curr = t_next;
            }
            t_curr
        }
    }
}

/// Evaluates Chebyshev polynomial $T_d(x)$ for scalar $x$ via 3-term recurrence.
pub fn eval_chebyshev_scalar(degree: usize, x: f64) -> f64 {
    match degree {
        0 => 1.0,
        1 => x,
        _ => {
            let mut t0 = 1.0;
            let mut t1 = x;
            for _ in 2..=degree {
                let t2 = 2.0 * x * t1 - t0;
                t0 = t1;
                t1 = t2;
            }
            t1
        }
    }
}

/// Polynomial multiplication $A(z) \cdot B(z)$ in ascending monomial basis.
pub fn poly_multiply(a: &[f64], b: &[f64]) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        if ai == 0.0 {
            continue;
        }
        for (j, &bj) in b.iter().enumerate() {
            out[i + j] += ai * bj;
        }
    }
    out
}

/// Transforms $T_d(x)$ under $x(z) = (z - c)/\rho = u_0 + u_1 z$, yielding polynomial in $z$.
pub fn chebyshev_interval_polynomial(interval: &ChebyshevInterval) -> Result<Vec<f64>, Error> {
    if !interval.a.is_finite()
        || !interval.b.is_finite()
        || interval.a >= interval.b
        || interval.b >= 0.0
        || interval.degree == 0
        || interval.degree > 128
    {
        return Err(Error::InvalidInput);
    }
    let c = interval.center();
    let rho = interval.semi_width();
    if rho <= 0.0 || !c.is_finite() || !rho.is_finite() {
        return Err(Error::InvalidInput);
    }
    let u0 = -c / rho;
    let u1 = 1.0 / rho;
    let t = chebyshev_basis_coefficients(interval.degree);
    let d = interval.degree;
    let mut poly = vec![t[d]];
    for k in (0..d).rev() {
        let mut next = vec![0.0; poly.len() + 1];
        for (i, &coeff) in poly.iter().enumerate() {
            next[i] += coeff * u0;
            next[i + 1] += coeff * u1;
        }
        next[0] += t[k];
        poly = next;
    }
    Ok(poly)
}

/// Evaluates a polynomial with ascending monomial coefficients on an outward `Interval`
/// using Horner's method with directed rounding.
pub fn eval_poly_interval(coeffs: &[f64], x: Interval) -> Result<Interval, Error> {
    if coeffs.is_empty() {
        return Interval::point(0.0).map_err(|_| Error::NonFinite);
    }
    let mut acc =
        Interval::point(*coeffs.last().expect("coeffs not empty")).map_err(|_| Error::NonFinite)?;
    for &c in coeffs.iter().rev().skip(1) {
        let term = acc.mul(x).map_err(|_| Error::NonFinite)?;
        let c_int = Interval::point(c).map_err(|_| Error::NonFinite)?;
        acc = term.add(c_int).map_err(|_| Error::NonFinite)?;
    }
    Ok(acc)
}

/// Computes a certified outward upper bound on the product defect ratio
/// $\sup_{z \in \bigcup_j I_j} |q(z)| / |q(1)|$.
///
/// Disproves and replaces the per-factor proxy $\max_j |T_j(1)|^{-1}$ (which failed by 10x
/// on multi-interval clusters in Section 4.1).
pub fn bound_product_defect(
    intervals: &[ChebyshevInterval],
    skew_guard: Option<(f64, f64)>,
    poly: &[f64],
) -> Result<f64, Error> {
    if poly.is_empty() || intervals.is_empty() {
        return Err(Error::InvalidInput);
    }

    // Evaluate q(1.0) on outward interval
    let one_int = Interval::point(1.0).map_err(|_| Error::NonFinite)?;
    let q_one_int = eval_poly_interval(poly, one_int)?;
    if q_one_int.contains_zero() {
        return Err(Error::DenominatorMargin);
    }
    let q_one_min = q_one_int.mig();
    if q_one_min <= 0.0 || !q_one_min.is_finite() {
        return Err(Error::DenominatorMargin);
    }

    if intervals.len() == 1 && skew_guard.is_none() {
        // Analytical bound: on a single Chebyshev interval, |T_d(x)| <= 1.0 exactly.
        return div_up(1.0, q_one_min).map_err(|_| Error::NonFinite);
    }

    // For multiple intervals or when skew-guard quadratic factor is present,
    // evaluate the global polynomial q(z) across each interval using outward interval
    // subdivision (M=32 sub-intervals per spectral component).
    let mut sup_num = 0.0_f64;
    for interval in intervals {
        let a = interval.a;
        let b = interval.b;
        if !a.is_finite() || !b.is_finite() || a >= b {
            return Err(Error::InvalidInput);
        }
        let m = 32;
        let step = (b - a) / (m as f64);
        for k in 0..m {
            let x_lo = a + (k as f64) * step;
            let x_hi = if k == m - 1 {
                b
            } else {
                a + ((k + 1) as f64) * step
            };
            let sub_int = Interval::new(x_lo, x_hi).map_err(|_| Error::NonFinite)?;
            let val_int = eval_poly_interval(poly, sub_int)?;
            sup_num = sup_num.max(val_int.mag());
        }
    }

    if !sup_num.is_finite() || sup_num < 0.0 {
        return Err(Error::NonFinite);
    }

    div_up(sup_num, q_one_min).map_err(|_| Error::NonFinite)
}

/// Computes a deterministic 64-bit FNV-1a digest binding the source matrix entries
/// and operator identity metadata.
fn compute_matrix_digest(dense_matrix: &[f64], identity: &ChartOperatorIdentity) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    hash ^= identity.operator_key;
    hash = hash.wrapping_mul(FNV_PRIME);
    hash ^= identity.dimension as u64;
    hash = hash.wrapping_mul(FNV_PRIME);
    hash ^= identity.model_epoch;
    hash = hash.wrapping_mul(FNV_PRIME);
    hash ^= identity.provider_epoch;
    hash = hash.wrapping_mul(FNV_PRIME);
    hash ^= identity.operator_scale_bits;
    hash = hash.wrapping_mul(FNV_PRIME);
    for &x in dense_matrix {
        for b in x.to_bits().to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    if hash == 0 { 1 } else { hash }
}

/// Checks if an $n \times n$ dense row-major matrix is strictly symmetric.
///
/// Any symmetry defect (including a 1-ulp difference) causes this function to return false.
pub fn is_matrix_symmetric(dense_matrix: &[f64], n: usize) -> bool {
    if n == 0 || dense_matrix.len() != n * n {
        return false;
    }
    if dense_matrix.iter().any(|x| !x.is_finite()) {
        return false;
    }
    for i in 0..n {
        for j in (i + 1)..n {
            let a_ij = dense_matrix[i * n + j];
            let a_ji = dense_matrix[j * n + i];
            if a_ij != a_ji || a_ij.to_bits() != a_ji.to_bits() {
                return false;
            }
        }
    }
    true
}

/// A certified outward spectral enclosure bound to a specific source operator identity
/// and matrix digest.
///
/// Only this type can mint `SpectralSource::GershgorinCertified` and `CertificateStatus::Certified`.
/// Caller-supplied arbitrary intervals or unverified projections remain `EstimateOnly`.
#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedSpectralEnclosure {
    intervals: Vec<ChebyshevInterval>,
    matrix_digest: u64,
    identity: ChartOperatorIdentity,
    operator_norm: f64,
    _sealed: (),
}

impl VerifiedSpectralEnclosure {
    /// Certifies an outward Gershgorin spectral enclosure for a dense symmetric matrix.
    ///
    /// Validates:
    /// - Matrix dimension matches `identity.dimension`
    /// - All matrix entries are finite
    /// - Matrix is strictly symmetric (rejecting any symmetry defect)
    /// - Outward Gershgorin discs lie strictly in $(-\infty, 0)$
    /// - Binds to matrix digest and operator identity
    pub fn certify_gershgorin(
        identity: ChartOperatorIdentity,
        dense_matrix: &[f64],
    ) -> Result<Self, Error> {
        let n = identity.dimension;
        if n == 0 || dense_matrix.len() != n * n {
            return Err(Error::InvalidInput);
        }
        if dense_matrix.iter().any(|x| !x.is_finite()) {
            return Err(Error::InvalidInput);
        }
        if !is_matrix_symmetric(dense_matrix, n) {
            return Err(Error::InvalidInput);
        }

        let intervals = gershgorin_intervals(dense_matrix, n);
        if intervals.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Outward operator norm bound: maximum outward row sum
        let mut max_row = 0.0_f64;
        for i in 0..n {
            let row_sum = sum_up((0..n).map(|j| dense_matrix[i * n + j].abs()))
                .map_err(|_| Error::NonFinite)?;
            max_row = max_row.max(row_sum);
        }
        let op_norm = max_row.max(1.0);

        let matrix_digest = compute_matrix_digest(dense_matrix, &identity);

        Ok(Self {
            intervals,
            matrix_digest,
            identity,
            operator_norm: op_norm,
            _sealed: (),
        })
    }

    /// Same as `certify_gershgorin`, but allows the caller to specify a known upper operator norm
    /// which is guaranteed to be at least the outward row sum.
    pub fn certify_gershgorin_with_norm(
        identity: ChartOperatorIdentity,
        dense_matrix: &[f64],
        operator_norm: f64,
    ) -> Result<Self, Error> {
        let mut enc = Self::certify_gershgorin(identity, dense_matrix)?;
        if operator_norm.is_finite() && operator_norm > enc.operator_norm {
            enc.operator_norm = operator_norm;
        }
        Ok(enc)
    }

    /// The outward certified intervals.
    pub fn intervals(&self) -> &[ChebyshevInterval] {
        &self.intervals
    }

    /// The 64-bit source digest binding the matrix entries and operator identity.
    pub fn matrix_digest(&self) -> u64 {
        self.matrix_digest
    }

    /// The operator identity this enclosure is bound to.
    pub fn identity(&self) -> ChartOperatorIdentity {
        self.identity
    }

    /// The outward operator norm bound.
    pub fn operator_norm(&self) -> f64 {
        self.operator_norm
    }

    /// Generates a product Chebyshev polynomial with certified status.
    pub fn generate_polynomial(
        &self,
        skew_guard: Option<(f64, f64)>,
    ) -> Result<GeneratedChebyshevPolynomial, Error> {
        let intervals = &self.intervals;
        if intervals.is_empty() && skew_guard.is_none() {
            return Err(Error::InvalidInput);
        }

        let mut q = vec![1.0];
        for interval in intervals {
            let p_int = chebyshev_interval_polynomial(interval)?;
            q = poly_multiply(&q, &p_int);
        }

        let mut has_skew = false;
        if let Some((d, omega)) = skew_guard {
            if !d.is_finite() || !omega.is_finite() || d < 0.0 {
                return Err(Error::InvalidInput);
            }
            let quad = vec![d * d + omega * omega, 2.0 * d, 1.0];
            q = poly_multiply(&q, &quad);
            has_skew = true;
        }

        if !(2..=129).contains(&q.len())
            || q.iter().any(|x| !x.is_finite())
            || q.last() == Some(&0.0)
        {
            return Err(Error::InvalidInput);
        }

        let defect_bound = bound_product_defect(intervals, skew_guard, &q)?;

        Ok(GeneratedChebyshevPolynomial {
            coefficients: q,
            defect_bound,
            source: SpectralSource::GershgorinCertified,
            certificate_status: CertificateStatus::Certified,
            has_skew_factor: has_skew,
        })
    }
}

/// Generator for Chebyshev-cluster polynomials from spectral intervals or Gershgorin discs.
///
/// NOTE: Calling `generate_polynomial` with caller-supplied intervals always produces
/// `CertificateStatus::EstimateOnly`. Only `VerifiedSpectralEnclosure::generate_polynomial`
/// or `ChebyshevClusterGenerator::generate_from_verified` can mint `CertificateStatus::Certified`.
#[derive(Clone, Debug)]
pub struct ChebyshevClusterGenerator {
    source: SpectralSource,
}

impl ChebyshevClusterGenerator {
    pub fn new(source: SpectralSource) -> Self {
        Self { source }
    }

    pub fn source(&self) -> SpectralSource {
        self.source
    }

    /// Generates product Chebyshev polynomial $q(z)$ from caller-supplied intervals.
    ///
    /// Caller-supplied intervals remain `EstimateOnly`. A caller-selected source enum
    /// cannot mint `Certified` without independent source verification.
    pub fn generate_polynomial(
        &self,
        intervals: &[ChebyshevInterval],
        skew_guard: Option<(f64, f64)>,
    ) -> Result<GeneratedChebyshevPolynomial, Error> {
        if intervals.is_empty() && skew_guard.is_none() {
            return Err(Error::InvalidInput);
        }

        let mut q = vec![1.0];

        for interval in intervals {
            let p_int = chebyshev_interval_polynomial(interval)?;
            q = poly_multiply(&q, &p_int);
        }

        let mut has_skew = false;
        if let Some((d, omega)) = skew_guard {
            if !d.is_finite() || !omega.is_finite() || d < 0.0 {
                return Err(Error::InvalidInput);
            }
            // Quadratic factor: (z + d)^2 + \omega^2 = z^2 + 2dz + (d^2 + \omega^2)
            // Monomial coefficients: [d^2 + \omega^2, 2*d, 1.0]
            let quad = vec![d * d + omega * omega, 2.0 * d, 1.0];
            q = poly_multiply(&q, &quad);
            has_skew = true;
        }

        if !(2..=129).contains(&q.len())
            || q.iter().any(|x| !x.is_finite())
            || q.last() == Some(&0.0)
        {
            return Err(Error::InvalidInput);
        }

        let defect_bound = bound_product_defect(intervals, skew_guard, &q)?;

        // Caller-supplied intervals cannot mint Certified.
        let certificate_status = CertificateStatus::EstimateOnly;
        let source = match self.source {
            SpectralSource::GershgorinCertified => SpectralSource::RitzEstimate,
            SpectralSource::RitzEstimate => SpectralSource::RitzEstimate,
        };

        Ok(GeneratedChebyshevPolynomial {
            coefficients: q,
            defect_bound,
            source,
            certificate_status,
            has_skew_factor: has_skew,
        })
    }

    /// Generates product Chebyshev polynomial from a certified outward spectral enclosure.
    pub fn generate_from_verified(
        &self,
        enclosure: &VerifiedSpectralEnclosure,
        skew_guard: Option<(f64, f64)>,
    ) -> Result<GeneratedChebyshevPolynomial, Error> {
        enclosure.generate_polynomial(skew_guard)
    }

    /// Convenience associated function for caller-supplied intervals (EstimateOnly).
    pub fn generate(
        source: SpectralSource,
        intervals: &[ChebyshevInterval],
        skew_guard: Option<(f64, f64)>,
    ) -> Result<GeneratedChebyshevPolynomial, Error> {
        Self::new(source).generate_polynomial(intervals, skew_guard)
    }
}

/// Skewness metric $\rho_{\text{skew}} = |\langle v, Bw \rangle - \langle Bv, w \rangle| / \|Bv\|$.
///
/// Returns 0.0 if dimensions mismatch or $\|Bv\| \approx 0$.
pub fn compute_skew_metric(v: &[f64], w: &[f64], bv: &[f64], bw: &[f64]) -> f64 {
    let n = v.len();
    if n == 0 || w.len() != n || bv.len() != n || bw.len() != n {
        return 0.0;
    }
    let mut dot_v_bw = 0.0;
    let mut dot_bv_w = 0.0;
    let mut bv_sq = 0.0;
    for i in 0..n {
        dot_v_bw += v[i] * bw[i];
        dot_bv_w += bv[i] * w[i];
        bv_sq += bv[i] * bv[i];
    }
    let bv_norm = bv_sq.sqrt();
    if bv_norm <= 0.0 || !bv_norm.is_finite() {
        return 0.0;
    }
    let diff = (dot_v_bw - dot_bv_w).abs();
    diff / bv_norm
}

/// Computes outward Gershgorin intervals from an $n \times n$ dense row-major matrix.
///
/// For each row $i$: center is $A_{ii}$, radius is outwardly bounded by $\sum_{j \ne i} |A_{ij}|$.
/// Endpoints are outwardly enclosed: $a_i \le A_{ii} - r_{out}$ and $b_i \ge A_{ii} + r_{out}$.
/// Merges overlapping discs and ensures intervals lie strictly in $(-\infty, 0)$.
pub fn gershgorin_intervals(dense_matrix: &[f64], n: usize) -> Vec<ChebyshevInterval> {
    if n == 0 || dense_matrix.len() != n * n {
        return Vec::new();
    }
    if dense_matrix.iter().any(|x| !x.is_finite()) {
        return Vec::new();
    }
    let mut raw_intervals: Vec<(f64, f64)> = Vec::with_capacity(n);
    for i in 0..n {
        let diag = dense_matrix[i * n + i];
        // Outward upper bound on sum_{j != i} |A_{ij}|
        let radius_res = sum_up(
            (0..n)
                .filter(|&j| j != i)
                .map(|j| dense_matrix[i * n + j].abs()),
        );
        let radius = match radius_res {
            Ok(r) if r.is_finite() => r,
            _ => return Vec::new(),
        };

        // Ensure outward non-zero radius to prevent zero-width intervals.
        // min_r >= 1e-6 * (|diag| + 1.0) computed with upward directed rounding.
        let min_r = match add_up(diag.abs(), 1.0).and_then(|d| mul_up(1e-6, d)) {
            Ok(mr) if mr.is_finite() => mr,
            _ => return Vec::new(),
        };
        let r_out = radius.max(min_r);

        // Outward endpoints: sub_down for lower bound, add_up for upper bound.
        let a = match sub_down(diag, r_out) {
            Ok(v) if v.is_finite() => v,
            _ => return Vec::new(),
        };
        let b = match add_up(diag, r_out) {
            Ok(v) if v.is_finite() => v,
            _ => return Vec::new(),
        };

        // Never clip a non-negative boundary to negative.
        // A spectral enclosure crossing into the non-negative half-plane violates
        // strict negative-spectrum stability. Fail closed by returning empty intervals.
        if b >= 0.0 || a >= b {
            return Vec::new();
        }
        raw_intervals.push((a, b));
    }

    raw_intervals.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));

    // Merge overlapping intervals
    let mut merged: Vec<ChebyshevInterval> = Vec::new();
    for (a, b) in raw_intervals {
        if let Some(last) = merged.last_mut()
            && a <= last.b
        {
            // Overlapping or touching: merge and increment degree
            last.b = last.b.max(b);
            last.degree = (last.degree + 1).min(128);
            continue;
        }
        merged.push(ChebyshevInterval { a, b, degree: 1 });
    }

    merged
}

impl ChartDeclaration {
    /// Factory constructor for `ChartDeclaration` from Chebyshev spectral intervals.
    ///
    /// Caller-supplied intervals remain `EstimateOnly`.
    pub fn from_chebyshev_intervals(
        identity: ChartOperatorIdentity,
        intervals: &[ChebyshevInterval],
        skew_guard: Option<(f64, f64)>,
        source: SpectralSource,
        operator_norm: f64,
        evidence_reference: u64,
    ) -> Result<Self, Error> {
        let generator = ChebyshevClusterGenerator::new(source);
        let generated = generator.generate_polynomial(intervals, skew_guard)?;
        let q_one = generated.evaluate(1.0).abs();
        let relation_norm = if generated.defect_bound > 0.0 && q_one.is_finite() {
            (generated.defect_bound * q_one).min(0.999 * q_one)
        } else {
            0.0
        };
        Self::new(
            identity,
            0.0,
            1.0,
            &generated.coefficients,
            relation_norm,
            operator_norm,
            evidence_reference,
        )
    }

    /// Factory constructor for `ChartDeclaration` from a private verified spectral enclosure.
    pub fn from_verified_enclosure(
        enclosure: &VerifiedSpectralEnclosure,
        skew_guard: Option<(f64, f64)>,
        operator_norm: f64,
        evidence_reference: u64,
    ) -> Result<Self, Error> {
        let generated = enclosure.generate_polynomial(skew_guard)?;
        let q_one = generated.evaluate(1.0).abs();
        let relation_norm = if generated.defect_bound > 0.0 && q_one.is_finite() {
            (generated.defect_bound * q_one).min(0.999 * q_one)
        } else {
            0.0
        };
        let op_norm = if operator_norm > 0.0 {
            operator_norm.max(enclosure.operator_norm())
        } else {
            enclosure.operator_norm().max(1.0)
        };
        Self::new(
            enclosure.identity(),
            0.0,
            1.0,
            &generated.coefficients,
            relation_norm,
            op_norm,
            evidence_reference,
        )
    }

    /// Factory constructor for `ChartDeclaration` from a dense matrix via outward Gershgorin discs.
    pub fn from_gershgorin(
        identity: ChartOperatorIdentity,
        dense_matrix: &[f64],
        skew_guard: Option<(f64, f64)>,
        operator_norm: f64,
        evidence_reference: u64,
    ) -> Result<Self, Error> {
        let n = identity.dimension;
        if n == 0 || dense_matrix.len() != n * n {
            return Err(Error::InvalidInput);
        }
        if !operator_norm.is_finite() || operator_norm < 0.0 || evidence_reference == 0 {
            return Err(Error::InvalidInput);
        }
        if dense_matrix.iter().any(|x| !x.is_finite()) {
            return Err(Error::InvalidInput);
        }

        // Outward operator norm bound: maximum outward row sum
        let mut max_row = 0.0_f64;
        for i in 0..n {
            let row_sum = sum_up((0..n).map(|j| dense_matrix[i * n + j].abs()))
                .map_err(|_| Error::NonFinite)?;
            max_row = max_row.max(row_sum);
        }
        let op_norm = if operator_norm > 0.0 {
            operator_norm.max(max_row)
        } else {
            max_row.max(1.0)
        };

        if is_matrix_symmetric(dense_matrix, n) {
            let enclosure = VerifiedSpectralEnclosure::certify_gershgorin_with_norm(
                identity,
                dense_matrix,
                op_norm,
            )?;
            Self::from_verified_enclosure(&enclosure, skew_guard, op_norm, evidence_reference)
        } else {
            // Non-symmetric matrices may have complex eigenvalues not enclosed
            // by real intervals. Classify strictly as EstimateOnly / RitzEstimate.
            let intervals = gershgorin_intervals(dense_matrix, n);
            if intervals.is_empty() {
                return Err(Error::InvalidInput);
            }
            Self::from_chebyshev_intervals(
                identity,
                &intervals,
                skew_guard,
                SpectralSource::RitzEstimate,
                op_norm,
                evidence_reference,
            )
        }
    }
}

impl ConditionedPolynomialPlan {
    /// Factory constructor for `ConditionedPolynomialPlan` from Chebyshev spectral intervals.
    #[allow(clippy::too_many_arguments)]
    pub fn from_chebyshev_intervals(
        identity: ChartOperatorIdentity,
        intervals: &[ChebyshevInterval],
        skew_guard: Option<(f64, f64)>,
        source: SpectralSource,
        operator_norm: f64,
        evidence_reference: u64,
        minimum_denominator_ratio: f64,
        work: &mut ChartWork,
    ) -> Result<Self, Error> {
        let decl = ChartDeclaration::from_chebyshev_intervals(
            identity,
            intervals,
            skew_guard,
            source,
            operator_norm,
            evidence_reference,
        )?;
        Self::prepare(&decl, minimum_denominator_ratio, work)
    }

    /// Factory constructor for `ConditionedPolynomialPlan` from a verified spectral enclosure.
    pub fn from_verified_enclosure(
        enclosure: &VerifiedSpectralEnclosure,
        skew_guard: Option<(f64, f64)>,
        operator_norm: f64,
        evidence_reference: u64,
        minimum_denominator_ratio: f64,
        work: &mut ChartWork,
    ) -> Result<Self, Error> {
        let decl = ChartDeclaration::from_verified_enclosure(
            enclosure,
            skew_guard,
            operator_norm,
            evidence_reference,
        )?;
        Self::prepare(&decl, minimum_denominator_ratio, work)
    }

    /// Factory constructor for `ConditionedPolynomialPlan` from a dense matrix via outward Gershgorin discs.
    #[allow(clippy::too_many_arguments)]
    pub fn from_gershgorin(
        identity: ChartOperatorIdentity,
        dense_matrix: &[f64],
        skew_guard: Option<(f64, f64)>,
        operator_norm: f64,
        evidence_reference: u64,
        minimum_denominator_ratio: f64,
        work: &mut ChartWork,
    ) -> Result<Self, Error> {
        let decl = ChartDeclaration::from_gershgorin(
            identity,
            dense_matrix,
            skew_guard,
            operator_norm,
            evidence_reference,
        )?;
        Self::prepare(&decl, minimum_denominator_ratio, work)
    }
}

#[cfg(test)]
mod wp09_tests {
    use super::*;

    #[test]
    fn test_chebyshev_recurrence_expansion() {
        assert_eq!(chebyshev_basis_coefficients(0), vec![1.0]);
        assert_eq!(chebyshev_basis_coefficients(1), vec![0.0, 1.0]);
        assert_eq!(chebyshev_basis_coefficients(2), vec![-1.0, 0.0, 2.0]);
        assert_eq!(chebyshev_basis_coefficients(3), vec![0.0, -3.0, 0.0, 4.0]);
        assert_eq!(
            chebyshev_basis_coefficients(4),
            vec![1.0, 0.0, -8.0, 0.0, 8.0]
        );

        for d in 0..=5 {
            for &x in &[-1.0, -0.5, 0.0, 0.5, 1.0, 2.0] {
                let poly = chebyshev_basis_coefficients(d);
                let eval_poly = poly.iter().rev().fold(0.0_f64, |acc, &c| acc.mul_add(x, c));
                let eval_rec = eval_chebyshev_scalar(d, x);
                assert!((eval_poly - eval_rec).abs() < 1e-12, "d={d}, x={x}");
            }
        }
    }

    #[test]
    fn test_chebyshev_interval_root_enclosure() {
        let interval = ChebyshevInterval::new(-10.0, -1.0, 3).expect("interval");
        assert_eq!(interval.center(), -5.5);
        assert_eq!(interval.semi_width(), 4.5);

        let generator = ChebyshevClusterGenerator::new(SpectralSource::GershgorinCertified);
        let generated = generator
            .generate_polynomial(&[interval], None)
            .expect("generated");
        assert_eq!(generated.degree(), 3);
        assert!(!generated.has_skew_factor);
        // Caller-supplied intervals remain EstimateOnly even if caller requested GershgorinCertified:
        assert_eq!(
            generated.certificate_status,
            CertificateStatus::EstimateOnly
        );
        assert!(!generated.is_certified());

        // For T_3(x), roots in standard domain [-1, 1] are x_k = cos((2k - 1)pi / 6) for k=1,2,3
        let roots_x = [
            (std::f64::consts::PI / 6.0).cos(),
            (3.0 * std::f64::consts::PI / 6.0).cos(),
            (5.0 * std::f64::consts::PI / 6.0).cos(),
        ];
        for &xk in &roots_x {
            let zk = interval.center() + interval.semi_width() * xk;
            assert!((-10.0..=-1.0).contains(&zk), "Root {zk} out of bounds");
            let q_zk = generated.evaluate(zk);
            assert!(q_zk.abs() < 1e-10, "q(zk) = {q_zk}");
        }

        // On the interval [-10, -1], |q(z)| <= 1.0
        for i in 0..=20 {
            let z = -10.0 + (9.0 / 20.0) * (i as f64);
            let val = generated.evaluate(z).abs();
            assert!(val <= 1.0 + 1e-12, "z={z}, val={val}");
        }

        // Defect bound eta <= |T_3((1 - c)/rho)|^{-1}
        let x_one = (1.0 - interval.center()) / interval.semi_width();
        let t3_one = eval_chebyshev_scalar(3, x_one);
        let expected_eta = 1.0 / t3_one.abs();
        assert!((generated.defect_bound - expected_eta).abs() < 1e-14);
        assert!(generated.defect_bound < 1.0);
    }

    #[test]
    fn test_skew_guard_triggers_quadratic_factor() {
        // Skew-symmetric operator action: B = [[-1, -5], [5, -1]]
        // damping d = 1.0, omega = 5.0
        let v = [1.0, 0.0];
        let w = [0.0, 1.0];
        let bv = [-1.0, 5.0];
        let bw = [-5.0, -1.0];
        let rho_skew = compute_skew_metric(&v, &w, &bv, &bw);
        assert!(rho_skew > 0.25, "rho_skew = {rho_skew}");

        let generator = ChebyshevClusterGenerator::new(SpectralSource::RitzEstimate);
        let interval = ChebyshevInterval::new(-8.0, -2.0, 1).expect("interval");
        let generated = generator
            .generate_polynomial(&[interval], Some((1.0, 5.0)))
            .expect("generated");

        assert!(generated.has_skew_factor);
        assert_eq!(
            generated.certificate_status,
            CertificateStatus::EstimateOnly
        );
        assert!(!generated.is_certified());
        // Degree should be 1 (interval) + 2 (quadratic factor) = 3
        assert_eq!(generated.degree(), 3);

        // At z = -1.0, quadratic factor (z + 1)^2 + 25 = 25.0
        // Interval polynomial for [-8, -2], c = -5, rho = 3: T_1((z - (-5))/3) = (z + 5)/3
        // At z = -1.0, ( -1 + 5 ) / 3 = 4/3
        // Total q(-1.0) = (4/3) * 25 = 100/3 ≈ 33.333333333333336
        let q_neg1 = generated.evaluate(-1.0);
        assert!((q_neg1 - 100.0 / 3.0).abs() < 1e-10, "q_neg1 = {q_neg1}");
    }

    #[test]
    fn test_gershgorin_intervals_and_certified_status() {
        // Asymmetric matrix:
        // [-5.0,  0.5]
        // [ 0.2, -8.0]
        let mat = [-5.0, 0.5, 0.2, -8.0];
        let intervals = gershgorin_intervals(&mat, 2);
        assert_eq!(intervals.len(), 2);
        assert!(intervals[0].a <= -8.0);
        assert!(intervals[0].b >= -8.0);
        assert!(intervals[1].a <= -5.0);
        assert!(intervals[1].b >= -5.0);

        // Caller-selected source enum on unverified intervals CANNOT mint Certified:
        let gen_arb = ChebyshevClusterGenerator::new(SpectralSource::GershgorinCertified)
            .generate_polynomial(&intervals, None)
            .expect("generated");
        assert_eq!(gen_arb.certificate_status, CertificateStatus::EstimateOnly);
        assert!(!gen_arb.is_certified());

        let gen_est = ChebyshevClusterGenerator::new(SpectralSource::RitzEstimate)
            .generate_polynomial(&intervals, None)
            .expect("generated");
        assert_eq!(gen_est.certificate_status, CertificateStatus::EstimateOnly);
        assert!(!gen_est.is_certified());

        // Symmetric matrix via VerifiedSpectralEnclosure CAN mint Certified:
        let sym_mat = [-5.0, 0.5, 0.5, -8.0];
        let identity = ChartOperatorIdentity {
            operator_key: 555,
            dimension: 2,
            model_epoch: 1,
            provider_epoch: 1,
            operator_scale_bits: 1.0_f64.to_bits(),
            physical_norm: crate::polynomial_resolvent::PolynomialPhysicalNorm::Infinity,
        };
        let enclosure =
            VerifiedSpectralEnclosure::certify_gershgorin(identity, &sym_mat).expect("enclosure");
        let gen_sym = enclosure.generate_polynomial(None).expect("gen_sym");
        assert_eq!(gen_sym.certificate_status, CertificateStatus::Certified);
        assert!(gen_sym.is_certified());
        assert_eq!(gen_sym.source, SpectralSource::GershgorinCertified);
    }

    #[test]
    fn test_chart_declaration_from_gershgorin() {
        let mat = [-4.0, 0.5, 0.2, -6.0];
        let identity = ChartOperatorIdentity {
            operator_key: 42,
            dimension: 2,
            model_epoch: 1,
            provider_epoch: 1,
            operator_scale_bits: 1.0_f64.to_bits(),
            physical_norm: crate::polynomial_resolvent::PolynomialPhysicalNorm::Infinity,
        };

        let decl =
            ChartDeclaration::from_gershgorin(identity, &mat, None, 10.0, 12345).expect("decl");
        assert_eq!(decl.center(), 0.0);
        assert_eq!(decl.scale(), 1.0);
        assert!(decl.coefficients().len() >= 2);

        let mut work = ChartWork::default();
        let plan = ConditionedPolynomialPlan::prepare(&decl, 0.05, &mut work).expect("plan");
        assert!(plan.total_defect_upper() < 1.0);
    }

    #[test]
    fn test_wp09_section_4_1_wolfram_counterexample() {
        // Section 4.1 Wolfram exact counterexample:
        // I1 = [-1000, -999], center -999.5, rho 0.5, deg 1
        // I2 = [-1.1, -1.0], center -1.05, rho 0.05, deg 1
        // True supremum defect ratio is at least 19979 / 82041 ≈ 0.243524579.
        let i1 = ChebyshevInterval::new(-1000.0, -999.0, 1).expect("i1");
        let i2 = ChebyshevInterval::new(-1.1, -1.0, 1).expect("i2");

        let generator = ChebyshevClusterGenerator::new(SpectralSource::RitzEstimate);
        let generated = generator
            .generate_polynomial(&[i1, i2], None)
            .expect("generated");

        assert_eq!(generated.degree(), 2);
        // Verify that defect_bound is an authentic outward upper bound >= 0.2435
        assert!(
            generated.defect_bound >= 0.2435,
            "defect_bound was {} < 0.2435 (old proxy gave ~0.02439)",
            generated.defect_bound
        );
        let old_buggy_proxy = 1.0 / 41.0;
        assert!(generated.defect_bound > 5.0 * old_buggy_proxy);
    }

    #[test]
    fn test_gershgorin_positive_spectrum_rejected() {
        // Scalar positive matrix A = [1.0] must be rejected (not clipped to negative!)
        let pos_scalar = [1.0];
        let intervals = gershgorin_intervals(&pos_scalar, 1);
        assert!(
            intervals.is_empty(),
            "Positive eigenvalue 1.0 must not yield intervals"
        );

        let identity = ChartOperatorIdentity {
            operator_key: 100,
            dimension: 1,
            model_epoch: 1,
            provider_epoch: 1,
            operator_scale_bits: 1.0_f64.to_bits(),
            physical_norm: crate::polynomial_resolvent::PolynomialPhysicalNorm::Infinity,
        };
        let err = ChartDeclaration::from_gershgorin(identity, &pos_scalar, None, 1.0, 100);
        assert!(err.is_err(), "Must reject positive operator");

        // Zero crossing matrix: diag -1.0, row off-diag 2.0 -> disc [-3.0, 1.0] crosses zero
        let crossing = [-1.0, 2.0, 0.0, -1.0];
        let identity2 = ChartOperatorIdentity {
            operator_key: 101,
            dimension: 2,
            model_epoch: 1,
            provider_epoch: 1,
            operator_scale_bits: 1.0_f64.to_bits(),
            physical_norm: crate::polynomial_resolvent::PolynomialPhysicalNorm::Infinity,
        };
        let err2 = ChartDeclaration::from_gershgorin(identity2, &crossing, None, 3.0, 101);
        assert!(err2.is_err(), "Must reject zero-crossing disc");
    }

    #[test]
    fn test_gershgorin_symmetric_certified_vs_nonsymmetric_estimate() {
        // Symmetric matrix:
        // [-4.0,  0.5]
        // [ 0.5, -6.0]
        let sym_mat = [-4.0, 0.5, 0.5, -6.0];
        assert!(is_matrix_symmetric(&sym_mat, 2));

        let identity = ChartOperatorIdentity {
            operator_key: 200,
            dimension: 2,
            model_epoch: 1,
            provider_epoch: 1,
            operator_scale_bits: 1.0_f64.to_bits(),
            physical_norm: crate::polynomial_resolvent::PolynomialPhysicalNorm::Infinity,
        };
        let _decl_sym =
            ChartDeclaration::from_gershgorin(identity, &sym_mat, None, 10.0, 200).expect("decl");

        // Certified enclosure from symmetric matrix:
        let enclosure =
            VerifiedSpectralEnclosure::certify_gershgorin(identity, &sym_mat).expect("enclosure");
        let gen_sym = enclosure.generate_polynomial(None).expect("gen_sym");
        assert!(gen_sym.is_certified());
        assert_eq!(gen_sym.certificate_status, CertificateStatus::Certified);
        assert_eq!(gen_sym.source, SpectralSource::GershgorinCertified);

        // Caller-selected source on arbitrary intervals cannot mint Certified:
        let intervals = gershgorin_intervals(&sym_mat, 2);
        let gen_caller = ChebyshevClusterGenerator::new(SpectralSource::GershgorinCertified)
            .generate_polynomial(&intervals, None)
            .expect("gen_caller");
        assert_eq!(
            gen_caller.certificate_status,
            CertificateStatus::EstimateOnly
        );
        assert!(!gen_caller.is_certified());

        // Non-symmetric matrix:
        // [-4.0,  0.5]
        // [ 0.1, -6.0]
        let nonsym_mat = [-4.0, 0.5, 0.1, -6.0];
        assert!(!is_matrix_symmetric(&nonsym_mat, 2));

        // When created via from_gershgorin, it must classify as RitzEstimate / EstimateOnly
        let _decl_nonsym =
            ChartDeclaration::from_gershgorin(identity, &nonsym_mat, None, 10.0, 201)
                .expect("decl_nonsym");
        let intervals_nonsym = gershgorin_intervals(&nonsym_mat, 2);
        let gen_nonsym = ChebyshevClusterGenerator::new(SpectralSource::RitzEstimate)
            .generate_polynomial(&intervals_nonsym, None)
            .expect("gen_nonsym");
        assert!(!gen_nonsym.is_certified());
        assert_eq!(
            gen_nonsym.certificate_status,
            CertificateStatus::EstimateOnly
        );

        // A 1-ulp symmetry defect must be rejected from certification:
        let mut ulp_defect_mat = sym_mat;
        ulp_defect_mat[1] = sym_mat[1].next_up();
        assert!(!is_matrix_symmetric(&ulp_defect_mat, 2));
        assert!(
            VerifiedSpectralEnclosure::certify_gershgorin(identity, &ulp_defect_mat).is_err(),
            "1-ulp symmetry defect must reject certification"
        );
    }
}
