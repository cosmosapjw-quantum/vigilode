//! Low-degree resolvents for a **declared** full-space polynomial relation.
//!
//! The operator is already scaled: `B = s A`. Coefficients are ascending powers
//! of `B`, so no `1/s` is formed. A caller must supply mathematical bounds on
//! `||q(B)||_inf` and `||B||_inf`; a few successful probes do not establish either.
//! This optional structural kernel is not an automatic Jacobian approximation.
//! Bounds reported below are conditional floating-point estimates, not interval
//! certificates. The original-space residual is always evaluated before output
//! is committed. No allocation occurs during `apply_checked`.

use rodas5p_core::directed::Interval;
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
    #[error("polynomial witness does not match the live scaled provider")]
    WitnessMismatch,
    #[error("original-space residual refused the stage")]
    ResidualRejected,
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

/// The existing kernel measures the physical infinity norm only. Other norms
/// are explicit unsupported requests, rather than silently relabeled bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolynomialPhysicalNorm {
    Infinity,
    Other(u64),
}

/// Exact identity of an immutable, homogeneous two-state collision direct sum.
/// Scale is compared by exact binary64 bits, never by a numerical tolerance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolynomialWitnessIdentity {
    pub operator_rate_bits: [u64; 2],
    pub dimension: usize,
    pub provider_epoch: u64,
    pub model_epoch: u64,
    pub scale_bits: u64,
    pub physical_norm: PolynomialPhysicalNorm,
}

/// Full-space symbolic declaration, not a relation inferred from finite probes.
/// For this provider the exact-real block algebra is A²=-(a+b)A. Applying B
/// in binary64 still has callback/residual roundoff, supplied separately at use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactExchangeRelationWitness {
    pub identity: PolynomialWitnessIdentity,
    pub evidence_reference: u64,
}

/// All counters include refused setup/application/stage attempts. Operator
/// attempts are charged BEFORE an action, so an arithmetic failure remains paid.
/// Direct block arithmetic is separate: zero callback actions is not zero work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PolynomialProviderWork {
    pub setups_attempted: u64,
    pub setup_failures: u64,
    pub applications_attempted: u64,
    pub applications_refused: u64,
    pub operator_actions_attempted: u64,
    pub operator_actions_completed: u64,
    pub stages_attempted: u64,
    pub stages_accepted: u64,
    pub direct_solves_attempted: u64,
    pub direct_solves_failed: u64,
    pub direct_blocks_solved: u64,
    pub direct_block_formula_evaluations: u64,
}

/// Number-exchange collision: u'=-a*u+b*v, v'=a*u-b*v in each independent cell.
/// a,b>=0 are immutable and identical across cells; total number is conserved.
/// This opt-in linear collision client is not a transport/RODAS route or a
/// general nonlinear closure. No Jacobian assembly or inferred derivative occurs.
#[derive(Clone, Copy, Debug)]
pub struct ExchangeCollisionProvider {
    a: f64,
    b: f64,
    dimension: usize,
    model_epoch: u64,
    provider_epoch: u64,
}

impl ExchangeCollisionProvider {
    pub fn new(
        a: f64,
        b: f64,
        blocks: usize,
        model_epoch: u64,
        provider_epoch: u64,
    ) -> Result<Self, PolynomialResolventError> {
        if !a.is_finite()
            || !b.is_finite()
            || a < 0.0
            || b < 0.0
            || !(a + b).is_finite()
            || blocks == 0
        {
            return Err(PolynomialResolventError::InvalidInput);
        }
        let dimension = blocks
            .checked_mul(2)
            .ok_or(PolynomialResolventError::InvalidInput)?;
        Ok(Self {
            a: if a == 0.0 { 0.0 } else { a },
            b: if b == 0.0 { 0.0 } else { b },
            dimension,
            model_epoch,
            provider_epoch,
        })
    }
    pub fn dimension(&self) -> usize {
        self.dimension
    }
    fn identity(&self, scale: f64, norm: PolynomialPhysicalNorm) -> PolynomialWitnessIdentity {
        PolynomialWitnessIdentity {
            operator_rate_bits: [self.a.to_bits(), self.b.to_bits()],
            dimension: self.dimension,
            provider_epoch: self.provider_epoch,
            model_epoch: self.model_epoch,
            scale_bits: scale.to_bits(),
            physical_norm: norm,
        }
    }

    /// Prepare B=sA with q(z)=z²+s(a+b)z, valid on the entire direct sum.
    /// The evidence reference names the client's derivation/intake record; this
    /// module enforces binding and algebraic structure, not external provenance.
    pub fn prepare_implicit_euler(
        &self,
        step: f64,
        norm: PolynomialPhysicalNorm,
        evidence_reference: u64,
        minimum_denominator_ratio: f64,
        work: &mut PolynomialProviderWork,
    ) -> Result<BoundPolynomialResolvent, PolynomialResolventError> {
        work.setups_attempted = work.setups_attempted.saturating_add(1);
        let result = (|| {
            if !step.is_finite()
                || step < 0.0
                || norm != PolynomialPhysicalNorm::Infinity
                || evidence_reference == 0
            {
                return Err(PolynomialResolventError::InvalidInput);
            }
            let rate_sum = self.a + self.b;
            let scaled_sum = step * rate_sum;
            if !scaled_sum.is_finite() || (step > 0.0 && rate_sum > 0.0 && scaled_sum == 0.0) {
                return Err(PolynomialResolventError::NonFinite);
            }
            // For exact-real B=sA, q(B)=(c-s(a+b))*B where c is the
            // rounded stored coefficient. Enclose that coefficient defect over
            // the FULL direct sum; this is not an inference from sample vectors.
            let arithmetic = || -> Result<(f64, f64), rodas5p_core::CoreError> {
                let exact_sum = Interval::exact_sum(self.a, self.b)?;
                let scaled = exact_sum.mul(Interval::point(step)?)?;
                let defect = Interval::point(scaled_sum)?.sub(scaled)?.mag();
                let norm = scaled.hi;
                let relation = Interval::point(defect)?.mul(Interval::point(norm)?)?.hi;
                Ok((relation, norm))
            };
            let (relation_norm, operator_norm) =
                arithmetic().map_err(|_| PolynomialResolventError::NonFinite)?;
            let plan = DeclaredPolynomialResolvent::new(
                &[0.0, scaled_sum, 1.0],
                relation_norm,
                operator_norm,
                minimum_denominator_ratio,
            )?;
            Ok(BoundPolynomialResolvent {
                plan,
                witness: ExactExchangeRelationWitness {
                    identity: self.identity(step, norm),
                    evidence_reference,
                },
            })
        })();
        if result.is_err() {
            work.setup_failures = work.setup_failures.saturating_add(1);
        }
        result
    }

    fn apply_scaled(
        &self,
        step: f64,
        x: &[f64],
        out: &mut [f64],
    ) -> Result<(), PolynomialResolventError> {
        if x.len() != self.dimension || out.len() != self.dimension {
            return Err(PolynomialResolventError::InvalidInput);
        }
        for (input, output) in x.chunks_exact(2).zip(out.chunks_exact_mut(2)) {
            let transfer = (-self.a).mul_add(input[0], self.b * input[1]);
            let transfer = step * transfer;
            if !transfer.is_finite() {
                return Err(PolynomialResolventError::NonFinite);
            }
            output[0] = transfer;
            output[1] = -transfer;
        }
        Ok(())
    }

    /// Independent small-block formula, retained as the specialized alternative.
    /// It performs a validation pass before output commit, without allocation.
    pub fn direct_implicit_euler(
        &self,
        step: f64,
        rhs: &[f64],
        output: &mut [f64],
        work: &mut PolynomialProviderWork,
    ) -> Result<(), PolynomialResolventError> {
        work.direct_solves_attempted = work.direct_solves_attempted.saturating_add(1);
        let result = (|| {
            if !step.is_finite()
                || step < 0.0
                || rhs.len() != self.dimension
                || output.len() != self.dimension
                || rhs.iter().any(|x| !x.is_finite())
            {
                return Err(PolynomialResolventError::InvalidInput);
            }
            let ha = step * self.a;
            let hb = step * self.b;
            let denominator = 1.0 + ha + hb;
            if !ha.is_finite() || !hb.is_finite() || !denominator.is_finite() {
                return Err(PolynomialResolventError::NonFinite);
            }
            let solve = |pair: &[f64]| {
                [
                    ((1.0 + hb) * pair[0] + hb * pair[1]) / denominator,
                    (ha * pair[0] + (1.0 + ha) * pair[1]) / denominator,
                ]
            };
            for pair in rhs.chunks_exact(2) {
                work.direct_block_formula_evaluations =
                    work.direct_block_formula_evaluations.saturating_add(1);
                if solve(pair).iter().any(|x| !x.is_finite()) {
                    return Err(PolynomialResolventError::NonFinite);
                }
                work.direct_blocks_solved = work.direct_blocks_solved.saturating_add(1);
            }
            for (pair, out) in rhs.chunks_exact(2).zip(output.chunks_exact_mut(2)) {
                work.direct_block_formula_evaluations =
                    work.direct_block_formula_evaluations.saturating_add(1);
                out.copy_from_slice(&solve(pair));
            }
            Ok(())
        })();
        if result.is_err() {
            work.direct_solves_failed = work.direct_solves_failed.saturating_add(1);
        }
        result
    }
}

/// Typed binding around the legacy structural primitive. No arbitrary callback
/// can be paired with this exact-exchange witness: action and live identity both
/// come from the immutable provider object.
#[derive(Clone, Debug)]
pub struct BoundPolynomialResolvent {
    plan: DeclaredPolynomialResolvent,
    witness: ExactExchangeRelationWitness,
}
impl BoundPolynomialResolvent {
    pub fn degree(&self) -> usize {
        self.plan.degree()
    }
    pub fn witness(&self) -> ExactExchangeRelationWitness {
        self.witness
    }
    pub fn relation_defect_ratio(&self) -> f64 {
        self.plan.relation_defect_ratio()
    }
    pub fn apply_provider_checked(
        &self,
        provider: &ExchangeCollisionProvider,
        step: f64,
        norm: PolynomialPhysicalNorm,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut PolynomialResolventWorkspace,
        absolute_residual_tolerance: f64,
        residual_evaluation_error: f64,
        work: &mut PolynomialProviderWork,
    ) -> Result<PolynomialResolventReport, PolynomialResolventError> {
        work.applications_attempted = work.applications_attempted.saturating_add(1);
        let result = (|| {
            if provider.identity(step, norm) != self.witness.identity {
                return Err(PolynomialResolventError::WitnessMismatch);
            }
            self.plan.apply_checked(
                rhs,
                output,
                workspace,
                absolute_residual_tolerance,
                residual_evaluation_error,
                |x, y| {
                    work.operator_actions_attempted =
                        work.operator_actions_attempted.saturating_add(1);
                    provider.apply_scaled(step, x, y)?;
                    work.operator_actions_completed =
                        work.operator_actions_completed.saturating_add(1);
                    Ok(())
                },
            )
        })();
        if result.as_ref().map_or(true, |r| !r.residual_accepted) {
            work.applications_refused = work.applications_refused.saturating_add(1);
        }
        result
    }
}

#[derive(Debug)]
pub struct ExchangeImplicitEulerWorkspace {
    accepted: Vec<f64>,
    polynomial: PolynomialResolventWorkspace,
}
impl ExchangeImplicitEulerWorkspace {
    pub fn new(dimension: usize) -> Self {
        Self {
            accepted: vec![0.0; dimension],
            polynomial: PolynomialResolventWorkspace::new(dimension),
        }
    }
}
#[derive(Clone, Debug)]
pub struct ExchangeImplicitEulerReport {
    pub completed_steps: usize,
    pub last_accepted_time: f64,
    pub failure: Option<PolynomialResolventError>,
}

/// Actual opt-in implicit-Euler collision trajectory, not a RODAS adaptation.
/// Output is the last accepted state even after stage rejection/failure. Work
/// counters accumulate across calls; callback roundoff allowance is distinct
/// from the exact-real symbolic relation. No global accuracy certificate follows.
#[allow(clippy::too_many_arguments)]
pub fn integrate_exchange_implicit_euler(
    provider: &ExchangeCollisionProvider,
    plan: &BoundPolynomialResolvent,
    step: f64,
    steps: usize,
    initial: &[f64],
    output: &mut [f64],
    workspace: &mut ExchangeImplicitEulerWorkspace,
    absolute_residual_tolerance: f64,
    residual_evaluation_error: f64,
    work: &mut PolynomialProviderWork,
) -> Result<ExchangeImplicitEulerReport, PolynomialResolventError> {
    if !step.is_finite()
        || step <= 0.0
        || !(step * steps as f64).is_finite()
        || initial.len() != provider.dimension
        || output.len() != provider.dimension
        || workspace.accepted.len() != provider.dimension
        || initial.iter().any(|x| !x.is_finite())
        || provider.identity(step, PolynomialPhysicalNorm::Infinity) != plan.witness.identity
    {
        return Err(PolynomialResolventError::InvalidInput);
    }
    workspace.accepted.copy_from_slice(initial);
    output.copy_from_slice(initial);
    let mut report = ExchangeImplicitEulerReport {
        completed_steps: 0,
        last_accepted_time: 0.0,
        failure: None,
    };
    for _ in 0..steps {
        work.stages_attempted = work.stages_attempted.saturating_add(1);
        match plan.apply_provider_checked(
            provider,
            step,
            PolynomialPhysicalNorm::Infinity,
            &workspace.accepted,
            output,
            &mut workspace.polynomial,
            absolute_residual_tolerance,
            residual_evaluation_error,
            work,
        ) {
            Ok(result) if result.residual_accepted => {
                workspace.accepted.copy_from_slice(output);
                work.stages_accepted = work.stages_accepted.saturating_add(1);
                report.completed_steps += 1;
                report.last_accepted_time = step * report.completed_steps as f64;
            }
            Ok(_) => {
                report.failure = Some(PolynomialResolventError::ResidualRejected);
                break;
            }
            Err(error) => {
                report.failure = Some(error);
                break;
            }
        }
    }
    Ok(report)
}
