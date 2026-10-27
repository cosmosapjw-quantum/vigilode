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

use rodas5p_core::directed::{Interval, add_up, div_up, mul_up, sub_down};
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
            for i in 0..n {
                ws.residual[i] = (rhs[i] - ws.candidate[i]) + ws.action[i];
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
